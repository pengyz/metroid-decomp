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

use game_core::frame::{FrameEngine, FrameMemory, Registers};
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
    /// 【Task 4】snapshot <prefix>（≡bundle_load 语义（task-2 决议））；`none`=显式无存档
    /// （上电起跑——**不找 none.mss**）；缺省=无 snapshot 行（同上电起跑）。
    snapshot: Option<String>,
}

fn parse_schedule(text: &str) -> Result<Plan, String> {
    let mut name = String::from("rust");
    let mut frames = 0usize;
    let mut joy = HashMap::new();
    let mut captures = Vec::new();
    let mut snapshot: Option<String> = None;
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
            "snapshot" => {
                let p = tok.get(1).ok_or_else(|| format!("line {}: snapshot <prefix>|none", ln + 1))?;
                // `snapshot none`=显式无存档（上电起跑）——不找 none.mss（spec D 决议）
                snapshot = if p.eq_ignore_ascii_case("none") { None } else { Some(p.to_string()) };
            }
            other => return Err(format!("line {}: 未知指令 {}", ln + 1, other)),
        }
    }
    if frames == 0 {
        return Err("缺 frames".into());
    }
    Ok(Plan { name, frames, joy, captures, snapshot })
}

// ---------------- bundle 三直映（snapshot 消费面——Task 4） ----------------

/// bundle 三直映（tools/scenario_capture.lua::bundle_capture 的 `<out>.json` 产物面）。
struct Bundle {
    ram: Vec<u8>,  // $0000-$07FF 2048B
    wram: Vec<u8>, // $6000-$7FFF 8192B
    regs: Registers,
    #[allow(dead_code)]
    frame: u32,
}

/// hex 串→定长字节（严格：长度+hexdigit 全检——格式漂移 fail-loud）。
fn hex_face(s: &str, want: usize, face: &str) -> Result<Vec<u8>, String> {
    if s.len() != want * 2 {
        return Err(format!("{face} 面长度 {}≠{} hex 字符——格式漂移拒装", s.len(), want * 2));
    }
    if !s.bytes().all(|c| c.is_ascii_hexdigit()) {
        return Err(format!("{face} 面含非 hex 字符——拒装"));
    }
    Ok((0..want).map(|i| u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).expect("已验 hex")).collect())
}

/// 三直映 JSON 解析（**bundle_capture 固定格式严格抽取**——scenario_capture.lua:518
/// 产物面：`{"mss_bytes":N,"ram":"…","wram":"…","regs":{a,x,y,sp,pc,ps},"frame":N}`）。
/// 未知键/缺键/越界整数=Err（格式漂移 fail-loud——不静默装 0）。已知键集钉死：
/// mss_bytes/ram/wram/regs{a,x,y,sp,pc,ps}/frame；§4-D 渲染面扩展若启用须随件扩本解析+测试。
fn parse_bundle_json(text: &str) -> Result<Bundle, String> {
    // regs 子对象先抽出（其内容为整数对，无嵌套/无串），再按顶层逗号切键值。
    let regs_at = text.find("\"regs\":{").ok_or("缺 regs 面")?;
    let regs_open = regs_at + "\"regs\":".len();
    let regs_end = text[regs_open..].find('}').map(|p| regs_open + p)
        .ok_or("regs 面未闭合")?;
    let regs_body = &text[regs_open + 1..regs_end];
    let flat = format!("{}\"regs\":0{}", &text[..regs_at], &text[regs_end + 1..]);
    let body = flat.trim().strip_prefix('{').and_then(|s| s.strip_suffix('}'))
        .ok_or("bundle JSON 须为对象")?;
    let mut top: Vec<(String, String)> = Vec::new();
    for kv in body.split(',') {
        let kv = kv.trim();
        if kv.is_empty() { continue; }
        let (k, v) = kv.split_once(':').ok_or_else(|| format!("键值对缺 ':': {kv}"))?;
        top.push((k.trim().trim_matches('"').to_string(), v.trim().to_string()));
    }
    let mut regs: Vec<(String, String)> = Vec::new();
    for kv in regs_body.split(',') {
        let kv = kv.trim();
        if kv.is_empty() { continue; }
        let (k, v) = kv.split_once(':').ok_or_else(|| format!("regs 键值对缺 ':': {kv}"))?;
        regs.push((k.trim().trim_matches('"').to_string(), v.trim().to_string()));
    }
    let known_top = ["mss_bytes", "ram", "wram", "regs", "frame"];
    for (k, _) in &top {
        if !known_top.contains(&k.as_str()) {
            return Err(format!("未知顶层键 {k}——格式漂移拒装"));
        }
    }
    let known_regs = ["a", "x", "y", "sp", "pc", "ps"];
    for (k, _) in &regs {
        if !known_regs.contains(&k.as_str()) {
            return Err(format!("未知 regs 键 {k}——格式漂移拒装"));
        }
    }
    let get_str = |key: &str| -> Result<String, String> {
        top.iter().find(|(k, _)| k == key).map(|(_, v)| v.clone())
            .ok_or_else(|| format!("缺顶层键 {key}"))
            .and_then(|v| v.strip_prefix('"').and_then(|s| s.strip_suffix('"'))
                .map(|s| s.to_string()).ok_or_else(|| format!("{key} 须为字符串")))
    };
    let get_int = |pairs: &[(String, String)], key: &str| -> Result<i64, String> {
        pairs.iter().find(|(k, _)| k == key).map(|(_, v)| v.clone())
            .ok_or_else(|| format!("缺整数键 {key}"))
            .and_then(|v| v.parse::<i64>().map_err(|_| format!("{key} 须为整数: {v}")))
    };
    let u8v = |pairs: &[(String, String)], key: &str| -> Result<u8, String> {
        let n = get_int(pairs, key)?;
        u8::try_from(n).map_err(|_| format!("{key} 越界 u8: {n}"))
    };
    // mss_bytes 在场性校验（固定格式面）；Rust 侧消费=三直映（.mss 为 emu 真源）
    get_int(&top, "mss_bytes")?;
    let ram = hex_face(&get_str("ram")?, 0x800, "ram")?;
    let wram = hex_face(&get_str("wram")?, 0x2000, "wram")?;
    let pc = get_int(&regs, "pc")?;
    let pc = u16::try_from(pc).map_err(|_| format!("pc 越界 u16: {pc}"))?;
    let regs = Registers {
        a: u8v(&regs, "a")?, x: u8v(&regs, "x")?, y: u8v(&regs, "y")?,
        sp: u8v(&regs, "sp")?, pc, p: u8v(&regs, "ps")?, // ps↔p（status 同义名）
    };
    let frame = get_int(&top, "frame")?;
    let frame = u32::try_from(frame).map_err(|_| format!("frame 越界 u32: {frame}"))?;
    Ok(Bundle { ram, wram, regs, frame })
}

/// 载入 `<prefix>.json` 三直映（`.mss`=emu 真源，Rust 腿不消费）。
fn load_bundle(prefix: &str) -> Result<Bundle, String> {
    let path = format!("{prefix}.json");
    let text = fs::read_to_string(&path).map_err(|e| format!("读 bundle 失败 {path}: {e}"))?;
    parse_bundle_json(&text).map_err(|e| format!("{path}: {e}"))
}

/// 状态行（`{"f":N,"w":{"zp":…,"iram":…,"wram":…}}`——oracle/l3-dump 帧 trace
/// **同形单一格式**（对齐验收两侧同解析）；编码唯一源=memmap::frame_face（watch_snapshot）。
fn state_row(frame: usize, mem: &FrameMemory) -> String {
    let snap = mem.watch_snapshot();
    let parts: Vec<String> = snap.iter().map(|(k, v)| format!("\"{k}\":\"{v}\"")).collect();
    format!("{{\"f\":{frame},\"w\":{{{}}}}}", parts.join(","))
}

// ---------------- 主流程 ----------------

fn main() {
    install_panic_hook();
    let args: Vec<String> = std::env::args().collect();
    let mut sched_path = String::new();
    let mut out_dir = String::from("shots");
    let mut bundle_cli: Option<String> = None;
    let mut state_out: Option<String> = None;
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--schedule" => { sched_path = args.get(i + 1).cloned().unwrap_or_default(); i += 2; }
            "--out" => { out_dir = args.get(i + 1).cloned().unwrap_or_else(|| "shots".into()); i += 2; }
            "--bundle" => { bundle_cli = args.get(i + 1).cloned(); i += 2; } // Task 4 snapshot 起跑
            "--state-out" => { state_out = args.get(i + 1).cloned(); i += 2; } // 对齐验收状态行
            _ => { eprintln!("未知参数 {}", args[i]); std::process::exit(2); }
        }
    }
    if sched_path.is_empty() {
        eprintln!("用法：rust_screenshot --schedule <path> [--out shots] [--bundle <prefix>] [--state-out <path>]");
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

    // 【Task 4】snapshot 装载：CLI --bundle > 调度表 snapshot 行（none=显式无存档=上电起跑）
    let bundle_prefix = bundle_cli.or(plan.snapshot.clone());
    let mut engine = FrameEngine::new();
    if let Some(bp) = &bundle_prefix {
        match load_bundle(bp) {
            Ok(b) => {
                eprintln!("[bundle] restore_mss 装载 {}: ram={}B wram={}B pc=${:04X} frame={}",
                          bp, b.ram.len(), b.wram.len(), b.regs.pc, b.frame);
                engine.restore_mss(&b.ram, &b.wram, b.regs);
            }
            Err(e) => {
                eprintln!("[bundle] 装载失败（fail-loud，不静默上电起跑）: {e}");
                std::process::exit(1);
            }
        }
    }

    let out = PathBuf::from(&out_dir);
    let _ = fs::create_dir_all(&out);

    eprintln!("[rust] 运行 FrameEngine：{}（frames={} captures={} snapshot={}）",
              plan.name, plan.frames, plan.captures.len(),
              if bundle_prefix.is_some() { "yes" } else { "no(上电起跑)" });

    let mut produced: Vec<(usize, String, String)> = Vec::new(); // (frame, tag, file)
    let mut blocked: Vec<(usize, String, String)> = Vec::new();  // (frame, tag, reason)
    let mut state_rows: Vec<String> = Vec::new(); // 对齐验收状态行（--state-out 时收集）
    let mut caps = plan.captures.clone();
    caps.sort_by_key(|c| c.0);
    let mut next_cap = 0usize;

    // 采集帧渲染（真实 RenderState→render_frame→PNG）。
    let render_capture = |engine: &FrameEngine, frame: usize, tag: &str,
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

    let want_state = state_out.is_some();
    let mut gap: Option<(usize, String)> = None; // (帧, unported bank:addr)
    for frame in 1..=plan.frames {
        engine.mem.joy1 = plan.joy.get(&frame).copied().unwrap_or(0);
        engine.mem.joy2 = 0;
        // catch_unwind：`unported!` fail-loud 缺口在**对拍工具层**不整程崩——
        // 引擎/测试仍确定性引爆（fail-loud 语义不变）；对拍记录缺口帧+原因，
        // 其后采集点标记 blocked（诚实记账——非伪造帧）。
        let tick = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            // 状态行=**spin 出口采样缝**（post-NMI 相位 P——emu ff 行同相位锚）；
            // restore 续跑首缝=装载态（skip 首 NMI）＝对齐验收首帧锚。
            engine.tick_frame_with(|mem| {
                if want_state {
                    state_rows.push(state_row(frame, mem));
                }
            });
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

    // 状态行落盘（对齐验收输入——gap 时照常落已采行（诚实部分产物，非伪造））
    if let Some(sp) = &state_out {
        let joined = state_rows.iter().map(|s| format!("{s}\n")).collect::<String>();
        if let Err(e) = fs::write(sp, &joined) {
            eprintln!("写状态行失败 {sp}: {e}");
            std::process::exit(1);
        }
        eprintln!("[rust] 状态行 {} 帧 → {sp}（spin 出口采样缝——对齐验收输入）", state_rows.len());
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

// ---------------- 测试（Task 4 snapshot 消费面——TDD 先红后绿） ----------------

#[cfg(test)]
mod tests {
    use super::*;
    use game_core::frame::{FrameMemory, Registers};

    /// 三直映 JSON fixture（tools/scenario_capture.lua::bundle_capture 真格式）：
    /// {"mss_bytes":N,"ram":"<4096 hex>","wram":"<16384 hex>","regs":{a,x,y,sp,pc,ps},"frame":N}
    fn fixture_json() -> (String, Vec<u8>, Vec<u8>) {
        let ram: Vec<u8> = (0..0x800u32).map(|i| (i.wrapping_mul(7).wrapping_add(3)) as u8).collect();
        let wram: Vec<u8> = (0..0x2000u32).map(|i| (i.wrapping_mul(13).wrapping_add(5)) as u8).collect();
        let hex = |b: &[u8]| b.iter().map(|x| format!("{x:02X}")).collect::<String>();
        let json = format!(
            "{{\"mss_bytes\":11561,\"ram\":\"{}\",\"wram\":\"{}\",\"regs\":{{\"a\":1,\"x\":0,\"y\":0,\"sp\":255,\"pc\":49363,\"ps\":4}},\"frame\":900}}",
            hex(&ram), hex(&wram));
        (json, ram, wram)
    }

    /// 三直映解析：ram/wram/regs/frame 各面到位（bundle_capture 真格式）。
    #[test]
    fn bundle_json_three_face_parse() {
        let (json, ram, wram) = fixture_json();
        let b = parse_bundle_json(&json).expect("解析应过");
        assert_eq!(b.ram, ram);
        assert_eq!(b.wram, wram);
        assert_eq!((b.regs.a, b.regs.x, b.regs.y, b.regs.sp, b.regs.pc, b.regs.p),
                   (1, 0, 0, 255, 49363, 4)); // ps↔p（status 同义名——spec §4 接缝表）
        assert_eq!(b.frame, 900);
    }

    /// 畸形 bundle JSON fail-loud（缺面/坏 hex/越界寄存器——不得静默装 0）。
    #[test]
    fn bundle_json_malformed_fails_loud() {
        assert!(parse_bundle_json("{\"ram\":\"AA\"}").is_err(), "缺 wram/regs 须拒");
        assert!(parse_bundle_json("{\"ram\":\"GG\",\"wram\":\"\",\"regs\":{\"a\":0,\"x\":0,\"y\":0,\"sp\":0,\"pc\":0,\"ps\":0},\"frame\":1}").is_err(),
                "坏 hex 须拒");
        assert!(parse_bundle_json("not json").is_err(), "非 JSON 须拒");
    }

    /// 【验收①/Step 1】构造 bundle（三直映 JSON）→ restore 后**首帧 == 期望帧**：
    /// 首状态行（三面 zp/iram/wram）与 bundle 三直映逐字节同——装载态真灌入
    /// （boot 重跑/漏面/半解析在此必红）。
    #[test]
    fn restore_first_state_row_matches_expected() {
        let (json, ram, wram) = fixture_json();
        let b = parse_bundle_json(&json).expect("解析应过");
        let mut engine = FrameEngine::new();
        engine.restore_mss(&b.ram, &b.wram, b.regs);
        let hex = |x: &[u8]| x.iter().map(|v| format!("{v:02X}")).collect::<String>();
        // 期望首帧：zp=ram[0x000-0x0FF]、iram=ram[0x100-0x7FF]、wram=全 8KB（frame_face 唯一编码）
        let expected = format!(
            "{{\"f\":1,\"w\":{{\"zp\":\"{}\",\"iram\":\"{}\",\"wram\":\"{}\"}}}}",
            hex(&ram[0x000..0x100]), hex(&ram[0x100..0x800]), hex(&wram[0x0000..0x2000]));
        let mut row = String::new();
        let tick = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            engine.tick_frame_with(|mem| row = state_row(1, mem));
        }));
        let _ = tick; // 合成态帧工作可能引爆 unported——首缝已在引爆前落下（不弱化 fail-loud）
        assert_eq!(row, expected, "restore 后首帧须=期望帧（三直映逐字节——加载≠对齐防线锚）");
    }
}
