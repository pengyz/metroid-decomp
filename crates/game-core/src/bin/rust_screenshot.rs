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
use game_core::state::RenderState;

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
    // 资产包（ROM 派生数据的唯一载体；见 docs/spec/data-code-separation.md）。
    // 缺包 ⇒ 直接退出并打印生成命令——绝不静默回落到内置副本（那样会把数据漏进仓库）。
    if let Err(e) = game_data::pack::init_from_env() {
        eprintln!("[pack] 资产包加载失败：{e}");
        eprintln!("[pack] 生成：cargo run -p pack-tool -- extract --rom \"rom/Metroid (USA).nes\" --out assets/metroid.pack");
        std::process::exit(3);
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
    // 【修径 a（findings/walk-fc-capture-seam.md §2c/§6a——walk FC 根修）】capture 帧 N
    // 双相位截取（emu endFrame N 单回调天然两相位）：
    //  · 图像/PPU 面 = snap（tick(N-1) 的 on_spin_exit 缝快照（NMI(N-1) 后/工作前——
    //    RenderState）——滚动=state-row(N-1)、名表≤work(N-2) 冲刷；
    //    **OAM $200-2FF = tick(N-1) 缝克隆（display set）**（emu spin-exit-seam 截取
    //    同相位——修径 C 协议；asm：NMI 只 DMA 不写 SpriteRAM，SpriteRAM=GoMainRoutine
    //    帧工作产物 ⇒ OAMDMA 源像=work(N-1) 收口=显示集）。
    //    【2026-11-09 更正】2026-11-03 探针"oam@收口 0 差"系 walk-fc end 采集金面，
    //    该金面=frames+1-fallback 兜底相（晚一拍）=scenario_capture.lua 末缝
    //    sampleTrace closed-file 抛错中断缝回调之工具 bug 污染产物（SEAM-FIRE/
    //    SAMPLE-ERR 实录定谳，commit e1609e28 修）——已按修径 C 全量 seam 相回归。
    //    缺缝（span/title 续段不采）⇒活体回退（span 冻结窗内等价）。
    //  · ram.hex 面 = 活体（tick(N-1) 收口（$1A=00 判别位）——work(N-1) 后/NMI(N) 前）。
    // 旧径两面整取 tick(N) 收口 ⇒ 名表/滚动 latch 晚一拍（f820 dx=+2 根因）。
    let render_capture = |engine: &FrameEngine,
                          snap: Option<(&RenderState, &[u8])>,
                          frame: usize, tag: &str,
                              out: &PathBuf, produced: &mut Vec<(usize, String, String)>| {
        let (rs, oam): (&RenderState, &[u8]) =
            capture_inputs(snap, &engine.mem.render, &engine.mem.ram[0x200..0x300]);
        let pal = &rs.palette;
        let bg: [u8; 16] = pal[0..16].try_into().unwrap();
        let spr: [u8; 16] = pal[16..32].try_into().unwrap();
        let fb = render_frame(rs, &bg, &spr, oam);
        let png = encode_png(&fb, 256, 240);
        let fname = format!("{}_f{:05}_{}.png", plan.name, frame, tag);
        fs::write(out.join(&fname), &png).expect("写 PNG");
        // PPU 真值 dump（终结判卡（2026-10-28））：与 scenario_capture.lua::dumpPpu 同构
        // 12KB（$0000-1FFF=chr 8KB（直）+$2000-27FF=nametable 2KB（A/B 半）+
        // $2800-2FFF=竖镜（A/B 镜））——hex 串（mt9 PPU 总线对拍面）。
        {
            let mut hex = String::new();
            for i in 0..0x2000usize {
                hex.push_str(&format!("{:02X}", rs.chr.get(i).copied().unwrap_or(0)));
            }
            for i in 0..0x800usize {
                hex.push_str(&format!("{:02X}", rs.nametable.get(i).copied().unwrap_or(0)));
            }
            for i in 0..0x400usize {
                hex.push_str(&format!("{:02X}", rs.nametable.get(i).copied().unwrap_or(0)));
            }
            for i in 0x400..0x800usize {
                hex.push_str(&format!("{:02X}", rs.nametable.get(i).copied().unwrap_or(0)));
            }
            // palette RAM（$3F00-$3F1F（32B（与 lua dumpPpu 同构）））
            for i in 0..32usize {
                hex.push_str(&format!("{:02X}", rs.palette.get(i).copied().unwrap_or(0)));
            }
            // 滚动/控制寄存器（scroll_x/scroll_y/ppuctrl（3B（细滚相位面）））
            hex.push_str(&format!("{:02X}", rs.scroll_x));
            hex.push_str(&format!("{:02X}", rs.scroll_y));
            hex.push_str(&format!("{:02X}", rs.ppuctrl));
            hex.push('\n');
            let fname = format!("{}_f{:05}_{}.ppu.hex", plan.name, frame, tag);
            fs::write(out.join(&fname), hex).expect("写 PPU dump");
            // WRAM 2KB dump（与 scenario_capture.lua::dumpRam 同构（0000-07FF hex）
            // +敌 extra 面 $6AF4-$6B73（②终钉（EnsExtra（双侧同构）））
            {
                let mut ram_hex = String::new();
                for i in 0..0x800usize {
                    ram_hex.push_str(&format!("{:02X}", engine.mem.ram.get(i).copied().unwrap_or(0)));
                }
                for i in 0x6AF4u16..=0x6B73 {
                    ram_hex.push_str(&format!("{:02X}", engine.mem.read_wram(i)));
                }
                ram_hex.push('\n');
                let fname = format!("{}_f{:05}_{}.ram.hex", plan.name, frame, tag);
                fs::write(out.join(&fname), ram_hex).expect("写 RAM dump");
            }
        }
        eprintln!("[rust]   capture f{:05} {} → {}", frame, tag, fname);
        produced.push((frame, tag.to_string(), fname));
    };

    let want_state = state_out.is_some();
    let mut gap: Option<(usize, String)> = None; // (帧, unported bank:addr)
    // 【止血件1·记账可观测】占位臂缺口（DIVERGENCES）逐帧增量——L4 腿与 l3-dump 同形
    // 落 sidecar `<name>.divergences.json`（gap_map+events+count+unique，l3-dump.rs:244 同构）。
    // 消费端：handler 触达清单固化（白名单增长即红的数据源，diff_traces.py:616 同契约）。
    let mut div_seen = 0usize;
    let mut div_events: Vec<(usize, u16)> = Vec::new(); // (frame, addr)
    // 【修径 a（findings/walk-fc-capture-seam.md §2c——walk FC 根修）】capture 帧 N 的产物
    // = **tick(N-1) 双相位**（emu endFrame N 单回调天然两相位）：
    //  · png+ppu.hex = tick(N-1) 的 on_spin_exit 缝快照（NMI(N-1) 后/工作前——滚动=
    //    state-row(N-1)、名表≤work(N-2) 冲刷）+ **OAM 收口活体**（tick(N-1) 收口——
    //    与 ram.hex 同相位；探针定谳 f1128 逐像素 0 差）；
    //  · ram.hex = tick(N-1) 收口活体（work(N-1) 后/NMI(N) 前——$1A=00 判别位）。
    // 发射点=迭代 frame 头（=tick(frame-1) 收口，先于 joy 注入）。旧径整取 tick(frame)
    // 收口 ⇒ 图像/名表/滚动 latch 晚一拍（walkfcprobe f820：+1 面+dx=+2/12085px 根因）。
    let mut seam_snap: Option<(usize, RenderState, Vec<u8>)> = None; // (tick, render, oam)
    for frame in 1..=plan.frames {
        // ——发射采集帧 frame（tick(frame-1) 相位）——
        while next_cap < caps.len() && caps[next_cap].0 == frame {
            let (_f, tag) = &caps[next_cap];
            let snap = match &seam_snap {
                Some((k, rs, oam)) if *k == frame.wrapping_sub(1) => Some((rs, oam.as_slice())),
                _ => None,
            };
            render_capture(&engine, snap, frame, tag, &out, &mut produced);
            next_cap += 1;
        }
        engine.mem.joy1 = plan.joy.get(&frame).copied().unwrap_or(0);
        if std::env::var("DSH_SPRITE_TRACE").is_ok() && frame >= 918 && frame <= 924 {
            eprintln!("[joy] f{} nmi_en={} ppuctrl={:02X}", frame, engine.mem.nmi_enabled, engine.mem.ram[0x00FF]);
            eprintln!("[joy] f{} joy1_field={:02X} sched={:02X}", frame, engine.mem.joy1, plan.joy.get(&frame).copied().unwrap_or(0));
        }
        engine.mem.joy2 = 0;
        // catch_unwind：`unported!` fail-loud 缺口在**对拍工具层**不整程崩——
        // 引擎/测试仍确定性引爆（fail-loud 语义不变）；对拍记录缺口帧+原因，
        // 其后采集点标记 blocked（诚实记账——非伪造帧）。
        let tick = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            // 状态行=**spin 出口采样缝**（post-NMI 相位 P——emu ff 行同相位锚）；
            // restore 续跑首缝=装载态（skip 首 NMI）＝对齐验收首帧锚。
            engine.tick_frame_with(|mem| {
                if std::env::var("DSH_FC_TRACE").is_ok() {
                    eprintln!("[fc] {} {} {}", frame, mem.read_ram(0x2D), mem.read_ram(0x1F));
                }
                if want_state {
                    state_rows.push(state_row(frame, mem));
                }
                // 缝快照（修径 a 图像/PPU 面——=state 行同缝，findings §2c 之②）：
                // post-NMI(frame)/pre-work(frame) 的 RenderState ——供采集帧 frame+1
                // 的 png+ppu.hex（滚动=state-row(frame)、名表≤work(frame-1) 冲刷）。
                // OAM 不入快照（收口活体——探针定谳 f1128 0 差）。
                if next_cap < caps.len() {
                    seam_snap = Some((
                        frame,
                        mem.render.clone(),
                        mem.ram[0x200..0x300].to_vec(),
                    ));
                }
            });
        }));
        // 【止血件1】本帧缺口增量（sound_hook `_ => None` 记账臂等）→ 逐帧事件面。
        // 先于 gap 分支记账：引爆帧自身的记账（panic 前 push）不丢。
        let now_div = game_core::frame::FrameEngine::divergences();
        if now_div.len() > div_seen {
            for a in &now_div[div_seen..] {
                div_events.push((frame, *a));
            }
            div_seen = now_div.len();
        }
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
        // 采集发射已移至迭代头（tick(frame-1) 收口相位——修径 a，findings §2c）。
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

    // 【止血件1·记账可观测】缺口 sidecar（与 l3-dump.rs:244 同构：gap_map+events+count+unique）
    // ——L4 腿首次落盘：矩阵跑完即得 handler 触达清单数据源（diff_traces.py:616 同契约 JSON）。
    {
        let gap_raw = game_core::frame::FrameEngine::divergences();
        let mut gap_counts: std::collections::BTreeMap<u16, usize> = std::collections::BTreeMap::new();
        for a in &gap_raw {
            *gap_counts.entry(*a).or_default() += 1;
        }
        let gap_map = gap_counts.iter()
            .map(|(a, n)| format!("{{\"addr\":{a},\"n\":{n}}}"))
            .collect::<Vec<_>>().join(", ");
        let djson = format!(
            "{{\"gap_map\": [{}], \"events\": [{}], \"count\": {}, \"unique\": {}}}",
            gap_map,
            div_events.iter().map(|(f, a)| format!("{{\"tick\":{f},\"addr\":{a}}}"))
                .collect::<Vec<_>>().join(", "),
            gap_raw.len(), gap_counts.len());
        let dgap = out.join(format!("{}.divergences.json", plan.name));
        fs::write(&dgap, &djson).expect("写缺口记账 sidecar");
        eprintln!("[rust]   缺口记账 {}/{} → {}", gap_raw.len(), gap_counts.len(), dgap.display());
    }

    eprintln!("[rust] 完成：产出 {} 采集点，缺口阻塞 {} 采集点 → {}",
              produced.len(), blocked.len(), out_dir);
}

/// 采集帧输入选择（修径 C——findings/walk-fc-capture-seam.md §2c）：**OAM 恒取缝克隆**。
/// asm 取证（2026-11-09，refs/m1disasm/SRC/prg7_engine.asm）：
///  · NMI（`NMI: ; 07:C0D9` :233）开头只做 `sta OAMADDR`+`sta OAMDMA`（:241-249）
///    ——**NMI 全程不写 SpriteRAM**（后随 CheckPaletteWrite/WriteScroll/ReadJoyPads/
///    SoundEngine/UpdateAge 皆非 $200-2FF 写者）；
///  · SpriteRAM($0200-2FF) = MainLoop（`MainLoop: ; 07:C0BC` :198）GoMainRoutine
///    帧工作产物（jsr DisplayBar :2513 / UpdateObjAnim :2972,:6226 /
///    ObjDrawFrame :5561,:12212 皆在工作链）；
///  · 帧 N 的 OAMDMA 源像 = work(N-1) 收口 SpriteRAM = 显示集（display set）；
///    缝克隆（post-NMI/pre-work）= 该显示集（emu spin-exit-seam 截取同相位——修径 C）。
/// rs = 缝快照（名表/滚动 latch 相位）；缺缝（span/title 续段不采）⇒活体回退。
///
/// 【2026-11-09 更正链】（证伪更正——历史主张原地保留于 git 史）：
///  ① "OAM 恒取收口"（2026-11-03 探针 f1128 "rs@缝+oam@收口 0 差"）——该金面系
///     walk-fc end 采集 = **frames+1-fallback 兜底相（晚一拍）**，工具 bug 污染产物
///     （③ 根因）；f650/f1400 dump 对实：mid 采集 emu png=缝克隆显示集，收口=下一
///     相位（f650 全 0xF0 藏集）。
///  ② is_end 判据（mid→缝/end→收口）→ f1200 回归 172px 证伪；span 判据
///     （span_left>0→收口）→ f1400 回归 58px 证伪。
///  ③ 两判据之根因同被证伪：frames+1-fallback 兜底相真因 = scenario_capture.lua
///     末缝 sampleTrace "attempt to use a closed file" 抛错中断 onSpinExitSeam
///     ⇒ fulfillShot 跳过（SEAM-FIRE f=1400/SAMPLE-ERR 实录）——已修（commit
///     e1609e28：文件关闭移 writeDoneAndStop 收口+pcall）⇒ 金面全量 seam 相
///     （修径 C 协议）⇒ OAM 恒取缝克隆与全部金面同相。
fn capture_inputs<'a>(
    snap: Option<(&'a RenderState, &'a [u8])>,
    live_rs: &'a RenderState,
    live_oam: &'a [u8],
) -> (&'a RenderState, &'a [u8]) {
    match snap {
        // OAM 恒取缝克隆（display set——修径 C 同相位；见 doc 更正链）。
        Some((rs, seam_oam)) => (rs, seam_oam),
        None => (live_rs, live_oam),
    }
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
    /// 首状态行（四面 zp/iram/wram/pw）与 bundle 三直映逐字节同——装载态真灌入
    /// （boot 重跑/漏面/半解析在此必红）。
    #[test]
    fn restore_first_state_row_matches_expected() {
        let (json, ram, wram) = fixture_json();
        let b = parse_bundle_json(&json).expect("解析应过");
        let mut engine = FrameEngine::new();
        engine.restore_mss(&b.ram, &b.wram, b.regs);
        let hex = |x: &[u8]| x.iter().map(|v| format!("{v:02X}")).collect::<String>();
        // 期望首帧：zp/iram/wram 全面+pw（$6988-$698F 密码面）——frame_face 唯一编码
        let expected = format!(
            "{{\"f\":1,\"w\":{{\"zp\":\"{}\",\"iram\":\"{}\",\"wram\":\"{}\",\"pw\":\"{}\"}}}}",
            hex(&ram[0x000..0x100]), hex(&ram[0x100..0x800]), hex(&wram[0x0000..0x2000]),
            hex(&wram[0x0988..0x0990]));
        let mut row = String::new();
        let tick = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            engine.tick_frame_with(|mem| row = state_row(1, mem));
        }));
        let _ = tick; // 合成态帧工作可能引爆 unported——首缝已在引爆前落下（不弱化 fail-loud）
        assert_eq!(row, expected, "restore 后首帧须=期望帧（三直映逐字节——加载≠对齐防线锚）");
    }

    /// 缝快照在场：OAM 恒取缝克隆（display set——与 emu spin-exit-seam 截取同相位）。
    /// 【2026-11-09 更正链见证 capture_inputs doc】恒取收口→is_end→span 三主张
    /// 皆被证伪（fallback 金面=工具 bug 污染，e1609e28 修后金面全 seam 相）。
    #[test]
    fn capture_inputs_uses_seam_oam_when_snap() {
        let snap_rs = RenderState::default();
        let live_rs = RenderState::default();
        let snap_oam = vec![0xAAu8; 0x100];
        let live_oam = vec![0x55u8; 0x100];
        let (rs, oam) = capture_inputs(Some((&snap_rs, &snap_oam)), &live_rs, &live_oam);
        assert!(std::ptr::eq(rs, &snap_rs), "rs 必须取缝快照（名表/滚动 latch 相位）");
        assert!(std::ptr::eq(oam, snap_oam.as_slice()), "OAM 须恒取缝克隆（display set——修径 C 同相位）");
    }

    /// 缺缝（span/title 续段不采）⇒ 活体回退（span 冻结窗内等价）。
    #[test]
    fn capture_inputs_fallback_live_when_no_snap() {
        let live_rs = RenderState::default();
        let live_oam = vec![0x55u8; 0x100];
        let (rs, oam) = capture_inputs(None, &live_rs, &live_oam);
        assert!(std::ptr::eq(rs, &live_rs), "缺缝 rs=活体回退");
        assert!(std::ptr::eq(oam, live_oam.as_slice()), "缺缝 OAM=活体回退");
    }
}
