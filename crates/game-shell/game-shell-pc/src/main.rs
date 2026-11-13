//! game-shell-pc 可执行入口（PC 壳主环——win/linux/mac）。
//!
//! 用法：`game-shell-pc [--scale N] [--fps N] [--frames N]`（frames=帧数上限（调试））。
//! 注：重制=Rust 逻辑移植（非模拟器）——**运行不需 ROM**（rom/ 仅验证轨用（BYO-ROM））。

use game_platform::runtime::run_game;

fn main() {
    let mut scale = 2usize;
    let mut fps = 60u32;
    let mut frames: Option<u64> = None;
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--scale" => scale = args.next().and_then(|v| v.parse().ok()).unwrap_or(2),
            "--fps" => fps = args.next().and_then(|v| v.parse().ok()).unwrap_or(60),
            "--frames" => frames = args.next().and_then(|v| v.parse().ok()),
            "--help" | "-h" => {
                println!("game-shell-pc [--scale N] [--fps N] [--frames N]");
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
