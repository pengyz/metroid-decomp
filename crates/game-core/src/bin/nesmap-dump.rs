//! nesmap-dump（bin）—— 静态腿 `nes-map` 的 **Rust 侧唯一出口**。
//!
//! 只做一件事：把 `memmap::nesmap::dump_json()` 打到 stdout，喂
//! `tools/nes_map_check.py`。判定权在 `gates/`，本 bin 不做判定、不读 NES 字节。
//!
//! 用法：
//!     cargo run -q -p game-core --bin nesmap-dump

fn main() {
    match game_core::memmap::nesmap::dump_json() {
        Ok(s) => println!("{s}"),
        Err(errs) => {
            for e in errs {
                eprintln!("[nesmap-dump] {e}");
            }
            std::process::exit(1);
        }
    }
}
