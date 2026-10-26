//! rust_screenshot（bin）—— Rust 实现截图生成（**真实渲染帧**：FrameEngine→render_frame）。
//!
//! 职责（L4 对拍 Rust 侧）：
//! 1. 消费**预解析调度表**（schedule 文本——由 tools/l4_diff_run.py 用 scenario_run.parse_scenario
//!    唯一 DSL 解析器生成；本 bin 不做 DSL 解析——守住“DSL 语义只在 scenario_run.py 解析”纪律）；
//! 2. 逐帧驱动 game_core::frame::FrameEngine（boot→title→gameplay 全序），按调度注入 joy1；
//! 3. 在 capture 帧调用 render_frame（**真实 RenderState + 调色板 + OAM 影子**）→ 256×240 RGB；
//! 4. 自包含 PNG 编码（stored-deflate + zlib 头/adler32 + CRC32——零第三方依赖）→
//!    `shots/<name>_f<NNNNN>_<tag>.png`（与模拟器命名**完全一致**——直接可比）。
//!
//! 调度表文本格式（每行空白分隔，# 注释）：
//!     name <scenario>
//!     frames <N>
//!     joy <frame> <joy1_hex>      # 1-based 帧 → NES 手柄字节（可稀疏；缺省 0）
//!     capture <frame> <tag>       # 采集点（帧渲染完成后截图）
//!
//! 用法：
//!     cargo run -p game-core --bin rust_screenshot -- --schedule <path> [--out shots]
//!
//! 手柄位形（game-platform PadState / hardware.asm:89-96——A=bit7 … Right=bit0）：
//!     A=0x80 B=0x40 Select=0x20 Start=0x10 Up=0x08 Down=0x04 Left=0x02 Right=0x01

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

use game_core::frame::FrameEngine;
use game_core::render::render_frame;

// panic 消息捕获（unported 缺口 bank:addr 取证——对拍工具层记录用，不弱化引擎 fail-loud）。
static PANIC_MSG: Mutex<Option<String>> = Mutex::new(None);

fn take_panic_message() -> Option<String> {
    PANIC_MSG.lock().ok().and_then(|mut g| g.take())
}

fn install_panic_hook() {
    std::panic::set_hook(Box::new(|info| {
        let msg = info
            .payload()
            .downcast_ref::<String>()
            .cloned()
            .or_else(|| info.payload().downcast_ref::<&str>().map(|s| s.to_string()))
            .unwrap_or_else(|| "panic".into());
        if let Ok(mut g) = PANIC_MSG.lock() {
            *g = Some(msg);
        }
    }));
}

// ---------------- 自包含 PNG 编码（stored-deflate） ----------------

fn crc32(data: &[u8]) -> u32 {
    let mut table = [0u32; 256];
    for (i, t) in table.iter_mut().enumerate() {
        let mut c = i as u32;
        for _ in 0..8 {
            c = if c & 1 != 0 { 0xEDB88320 ^ (c >> 1) } else { c >> 1 };
        }
        *t = c;
    }
    let mut c = 0xFFFF_FFFFu32;
    for &b in data {
        c = table[((c ^ b as u32) & 0xFF) as usize] ^ (c >> 8);
    }
    c ^ 0xFFFF_FFFF
}

fn adler32(data: &[u8]) -> u32 {
    let (mut a, mut b) = (1u32, 0u32);
    for &x in data {
        a = (a + x as u32) % 65521;
        b = (b + a) % 65521;
    }
    (b << 16) | a
}

fn png_chunk(kind: &[u8; 4], data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len() + 12);
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    out.extend_from_slice(kind);
    out.extend_from_slice(data);
    let mut crc_in = Vec::with_capacity(data.len() + 4);
    crc_in.extend_from_slice(kind);
    crc_in.extend_from_slice(data);
    out.extend_from_slice(&crc32(&crc_in).to_be_bytes());
    out
}

/// RGB（256×240×3）→ PNG（8 位 RGB 非隔行）。IDAT=zlib（CMF/FLG + stored-deflate 块 + adler32）。
fn encode_png(rgb: &[u8], w: usize, h: usize) -> Vec<u8> {
    let stride = w * 3;
    let mut raw = Vec::with_capacity(h * (stride + 1));
    for y in 0..h {
        raw.push(0u8); // filter=None
        raw.extend_from_slice(&rgb[y * stride..(y + 1) * stride]);
    }
    // zlib：CMF=0x78（CM=8/CINFO=7），FLG=0x01（(0x78*256+0x01)%31==0）
    let mut z = vec![0x78u8, 0x01];
    let mut off = 0usize;
    while off < raw.len() {
        let n = std::cmp::min(65535, raw.len() - off);
        let last = if off + n >= raw.len() { 1u8 } else { 0u8 };
        z.push(last); // BFINAL + BTYPE=00 + pad
        z.extend_from_slice(&(n as u16).to_le_bytes());
        z.extend_from_slice(&(!(n as u16)).to_le_bytes());
        z.extend_from_slice(&raw[off..off + n]);
        off += n;
    }
    z.extend_from_slice(&adler32(&raw).to_be_bytes());

    let mut out = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    let mut ihdr = Vec::new();
    ihdr.extend_from_slice(&(w as u32).to_be_bytes());
    ihdr.extend_from_slice(&(h as u32).to_be_bytes());
    ihdr.extend_from_slice(&[8, 2, 0, 0, 0]); // depth=8, color=RGB
    out.extend_from_slice(&png_chunk(b"IHDR", &ihdr));
    out.extend_from_slice(&png_chunk(b"IDAT", &z));
    out.extend_from_slice(&png_chunk(b"IEND", &[]));
    out
}

// ---------------- 调度表解析（行式——非 DSL） ----------------

struct Plan {
    name: String,
    frames: usize,
    joy: HashMap<usize, u8>,
    captures: Vec<(usize, String)>,
}

fn parse_schedule(text: &str) -> Result<Plan, String> {
    let mut name = String::from("rust");
    let mut frames = 0usize;
    let mut joy = HashMap::new();
    let mut captures = Vec::new();
    for (ln, line) in text.lines().enumerate() {
        let line = line.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        let tok: Vec<&str> = line.split_whitespace().collect();
        match tok[0] {
            "name" => name = tok.get(1).unwrap_or(&"rust").to_string(),
            "frames" => {
                frames = tok.get(1).and_then(|s| s.parse().ok())
                    .ok_or_else(|| format!("line {}: frames 需正整数", ln + 1))?;
            }
            "joy" => {
                let f: usize = tok.get(1).and_then(|s| s.parse().ok())
                    .ok_or_else(|| format!("line {}: joy 帧号", ln + 1))?;
                let v: u8 = tok.get(2)
                    .and_then(|s| u8::from_str_radix(s.trim_start_matches("0x"), 16).ok())
                    .ok_or_else(|| format!("line {}: joy 值 hex", ln + 1))?;
                joy.insert(f, v);
            }
            "capture" => {
                let f: usize = tok.get(1).and_then(|s| s.parse().ok())
                    .ok_or_else(|| format!("line {}: capture 帧号", ln + 1))?;
                let tag = tok.get(2).unwrap_or(&"cap").to_string();
                captures.push((f, tag));
            }
            other => return Err(format!("line {}: 未知指令 {}", ln + 1, other)),
        }
    }
    if frames == 0 {
        return Err("缺 frames".into());
    }
    Ok(Plan { name, frames, joy, captures })
}

// ---------------- 主流程 ----------------

fn main() {
    install_panic_hook();
    let args: Vec<String> = std::env::args().collect();
    let mut sched_path = String::new();
    let mut out_dir = String::from("shots");
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--schedule" => { sched_path = args.get(i + 1).cloned().unwrap_or_default(); i += 2; }
            "--out" => { out_dir = args.get(i + 1).cloned().unwrap_or_else(|| "shots".into()); i += 2; }
            _ => { eprintln!("未知参数 {}", args[i]); std::process::exit(2); }
        }
    }
    if sched_path.is_empty() {
        eprintln!("用法：rust_screenshot --schedule <path> [--out shots]");
        std::process::exit(2);
    }
    let text = fs::read_to_string(&sched_path).unwrap_or_else(|e| {
        eprintln!("读调度表失败 {}: {}", sched_path, e);
        std::process::exit(1);
    });
    let plan = parse_schedule(&text).unwrap_or_else(|e| {
        eprintln!("解析调度表失败: {}", e);
        std::process::exit(1);
    });

    let out = PathBuf::from(&out_dir);
    let _ = fs::create_dir_all(&out);

    eprintln!("[rust] 运行 FrameEngine：{}（frames={} captures={}）",
              plan.name, plan.frames, plan.captures.len());

    let mut engine = FrameEngine::new();
    let mut produced: Vec<(usize, String, String)> = Vec::new(); // (frame, tag, file)
    let mut blocked: Vec<(usize, String, String)> = Vec::new();  // (frame, tag, reason)
    let mut caps = plan.captures.clone();
    caps.sort_by_key(|c| c.0);
    let mut next_cap = 0usize;

    // 采集帧渲染（真实 RenderState→render_frame→PNG）。
    let mut render_capture = |engine: &FrameEngine, frame: usize, tag: &str,
                              out: &PathBuf, produced: &mut Vec<(usize, String, String)>| {
        let pal = &engine.mem.render.palette;
        let bg: [u8; 16] = pal[0..16].try_into().unwrap();
        let spr: [u8; 16] = pal[16..32].try_into().unwrap();
        let fb = render_frame(&engine.mem.render, &bg, &spr, &engine.mem.ram[0x200..0x300]);
        let png = encode_png(&fb, 256, 240);
        let fname = format!("{}_f{:05}_{}.png", plan.name, frame, tag);
        fs::write(out.join(&fname), &png).expect("写 PNG");
        eprintln!("[rust]   capture f{:05} {} → {}", frame, tag, fname);
        produced.push((frame, tag.to_string(), fname));
    };

    let mut gap: Option<(usize, String)> = None; // (帧, unported bank:addr)
    for frame in 1..=plan.frames {
        engine.mem.joy1 = plan.joy.get(&frame).copied().unwrap_or(0);
        engine.mem.joy2 = 0;
        // catch_unwind：`unported!` fail-loud 缺口在**对拍工具层**不整程崩——
        // 引擎/测试仍确定性引爆（fail-loud 语义不变）；对拍记录缺口帧+原因，
        // 其后采集点标记 blocked（诚实记账——非伪造帧）。
        let tick = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            engine.tick_frame();
        }));
        if tick.is_err() {
            let reason = take_panic_message().unwrap_or_else(|| "unknown gap".into());
            eprintln!("[rust] ⚠ 帧 {} 引爆缺口：{}", frame, reason);
            gap = Some((frame, reason.clone()));
            // 其后采集点全部 blocked
            while next_cap < caps.len() {
                let (f, tag) = &caps[next_cap];
                blocked.push((*f, tag.clone(), format!("unported gap @ f{}: {}", frame, reason)));
                next_cap += 1;
            }
            break;
        }
        // 触发该帧采集点（帧渲染完成后）
        while next_cap < caps.len() && caps[next_cap].0 == frame {
            let (_f, tag) = &caps[next_cap];
            render_capture(&engine, frame, tag, &out, &mut produced);
            next_cap += 1;
        }
    }
    // 未到 gap 但超出 frames 的残留采集（防御）
    while next_cap < caps.len() {
        let (f, tag) = &caps[next_cap];
        blocked.push((*f, tag.clone(), "超出 frames".into()));
        next_cap += 1;
    }

    // manifest（供 Python 对拍读取）
    let mut man = String::from("{\"name\":\"");
    man.push_str(&plan.name);
    man.push_str("\",\"frames\":");
    man.push_str(&plan.frames.to_string());
    man.push_str(&format!(",\"gap\":{}", if gap.is_some() { "true" } else { "false" }));
    man.push_str(",\"captures\":[");
    for (k, (f, tag, fname)) in produced.iter().enumerate() {
        if k > 0 { man.push(','); }
        man.push_str(&format!("{{\"frame\":{},\"tag\":\"{}\",\"file\":\"{}\",\"status\":\"produced\"}}", f, tag, fname));
    }
    man.push_str("],\"blocked\":[");
    for (k, (f, tag, reason)) in blocked.iter().enumerate() {
        if k > 0 { man.push(','); }
        let r = reason.replace('\\', "\\\\").replace('"', "\\\"");
        man.push_str(&format!("{{\"frame\":{},\"tag\":\"{}\",\"reason\":\"{}\"}}", f, tag, r));
    }
    man.push_str("]}");
    fs::write(out.join(format!("{}.rust.captures.json", plan.name)), &man).expect("写 manifest");

    eprintln!("[rust] 完成：产出 {} 采集点，缺口阻塞 {} 采集点 → {}",
              produced.len(), blocked.len(), out_dir);
}
