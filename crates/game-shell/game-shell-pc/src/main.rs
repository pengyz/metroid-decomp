//! game-shell-pc 可执行入口（PC 壳主环——win/linux/mac）。
//!
//! 用法：`game-shell-pc [--scale N] [--fps N] [--frames N]`
//! 无窗口验收：`--headless [--script FILE] [--state-out FILE] [--shot-frames 1,100] [--shot-dir DIR]`。
//! 注：重制=Rust 逻辑移植（非模拟器）——**运行不需 ROM**（rom/ 仅验证轨用（BYO-ROM））。

use game_platform::runtime::run_game;

fn main() {
    let mut scale = 2usize;
    let mut fps = 60u32;
    let mut frames: Option<u64> = None;
    let mut headless = false;
    let mut script_path: Option<String> = None;
    let mut state_out: Option<String> = None;
    let mut shot_dir: Option<String> = None;
    let mut shot_frames: Vec<u64> = Vec::new();
    let mut wram_out: Option<String> = None;
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--scale" => scale = args.next().and_then(|v| v.parse().ok()).unwrap_or(2),
            "--fps" => fps = args.next().and_then(|v| v.parse().ok()).unwrap_or(60),
            "--frames" => frames = args.next().and_then(|v| v.parse().ok()),
            "--headless" => headless = true,
            "--script" => script_path = args.next(),
            "--state-out" => state_out = args.next(),
            "--shot-dir" => shot_dir = args.next(),
            "--wram-out" => wram_out = args.next(),
            "--shot-frames" => {
                let spec = args.next().unwrap_or_default();
                for part in spec.split(',').filter(|s| !s.trim().is_empty()) {
                    match part.trim().parse::<u64>() {
                        Ok(v) => shot_frames.push(v),
                        Err(_) => {
                            eprintln!("--shot-frames 里有非整数帧号: {part:?}");
                            std::process::exit(2);
                        }
                    }
                }
            }
            "--help" | "-h" => {
                println!("game-shell-pc [--scale N] [--fps N] [--frames N]");
                println!("  --headless --script FILE [--state-out FILE] [--shot-frames a,b] [--shot-dir DIR]");
                return;
            }
            other => {
                eprintln!("未知参数: {other}（--help 看用法）");
                std::process::exit(2);
            }
        }
    }

    // 资产包（ROM 派生数据的唯一载体；见 docs/spec/data-code-separation.md）。
    // 缺包 ⇒ 打印生成命令后退出——绝不静默回落到内置副本。
    if let Err(e) = game_core::pack::init_from_env() {
        eprintln!("[pack] 资产包加载失败：{e}");
        eprintln!("[pack] 生成：cargo run -p pack-tool -- extract --rom \"rom/Metroid (USA).nes\" --out assets/metroid.pack");
        std::process::exit(3);
    }

    let mut driver = game_shell_pc::CoreDriver::new();

    // 无窗口验收径：脚本化按键 + 逐帧状态 CSV + 指定帧 PPM（#102 可玩构建回归命令）。
    if headless {
        let script = match script_path {
            Some(p) => match std::fs::read_to_string(&p) {
                Ok(text) => match game_shell_pc::script::Script::parse(&text) {
                    Ok(s) => s,
                    Err(e) => {
                        eprintln!("[script] {p} 解析失败：{e}");
                        std::process::exit(2);
                    }
                },
                Err(e) => {
                    eprintln!("[script] 读不到 {p}：{e}");
                    std::process::exit(2);
                }
            },
            None => game_shell_pc::script::Script::default(),
        };
        let n = frames.unwrap_or(600);
        let state_out = state_out.as_deref().map(std::path::Path::new);
        let shot_dir = shot_dir.as_deref().map(std::path::Path::new);
        match game_shell_pc::script::run_scripted(
            &mut driver,
            &script,
            n,
            state_out,
            &shot_frames,
            shot_dir,
        ) {
            Ok(r) => {
                if let Some(p) = wram_out.as_deref() {
                    let mut text = String::new();
                    for a in 0x6000u16..0x6800 {
                        text.push_str(&format!("{:02X}", driver.engine.mem.read_wram(a)));
                        if a % 0x20 == 0x1F {
                            text.push('\n');
                        }
                    }
                    if let Err(e) = std::fs::write(p, text) {
                        eprintln!("[headless] 写 {p} 失败：{e}");
                        std::process::exit(1);
                    }
                }
                println!(
                    "game-shell-pc headless: {} 帧；房间/地图格变化 {} 次：{:?}",
                    r.frames,
                    r.states.len(),
                    r.states
                );
            }
            Err(e) => {
                eprintln!("[headless] 运行/落盘失败：{e}");
                std::process::exit(1);
            }
        }
        return;
    }
    let mut shell = match game_shell_pc::PcShell::new("Metroid (Rust remake)", scale, fps) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("开窗失败: {e}");
            std::process::exit(1);
        }
    };
    let n = run_game(&mut driver, &mut shell, frames);
    println!("game-shell-pc: {n} 帧后退出");
}
