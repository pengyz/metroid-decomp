//! tools/rust_screenshot.rs —— Rust 实现截图生成（用于对拍（Rust vs 模拟器））。
//!
//! 职责：
//! 1. 运行 Rust 实现（game-core（渲染帧））
//! 2. 生成截图（shots/（用于对拍））
//! 3. 消费统一输入格式（scenario DSL（两边消费））
//!
//! 用法：
//!     cargo run --bin rust_screenshot -- <scenario> [--timeout 秒]
//!     cargo run --bin rust_screenshot -- case-001-game-start-samus-interaction --timeout 60
//!
//! 输出：
//!     shots/<scenario>_f<N>_<tag>.png  # 截图（用于对拍（Rust vs 模拟器））

use std::fs;
use std::path::Path;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        println!("用法：cargo run --bin rust_screenshot -- <scenario> [--timeout 秒]");
        return;
    }
    
    let scenario = &args[1];
    let timeout: u64 = if args.len() > 2 { args[2].parse().unwrap_or(30) } else { 30 };
    
    println!("[*] 运行 Rust 实现：{}（timeout={}）", scenario, timeout);
    
    // TODO: 实现 Rust 实现执行（game-core（渲染帧））
    // 当前：模拟 Rust 实现执行（生成截图）
    
    let output_dir = Path::new("shots");
    if !output_dir.exists() {
        fs::create_dir_all(output_dir).unwrap();
    }
    
    // 模拟 Rust 实现执行（生成截图）
    println!("[*] Rust 实现执行完成（TODO（待实现））");
    println!("[*] 截图生成：shots/（TODO（待实现））");
}
