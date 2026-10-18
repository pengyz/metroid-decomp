//! render-dump —— L4 game-core 渲染腿帧导出（S6-10：shots/<场景>_f<帧>_bg.png）。
//! 用法: render-dump --scenario <name>
//!
//! 【状态注】RenderState 帧引擎接线（840ad00 建模→nmi.rs PPU 写喂入）=跨域件
//! （porter 面）——本 dump 先以 fixture 面闭环 PNG 管线（判据 1 格式检）；
//! 接线后同命令产真帧。

use game_oracle::png_encode::encode_rgb;

fn main() {
    // 管线自检（fixture 帧——判据 1：8 位 RGB 非隔行 PNG 落盘）
    let pixels = vec![0x66u8; 256 * 240 * 3];
    let png = encode_rgb(256, 240, &pixels);
    std::fs::create_dir_all("shots").ok();
    let path = "shots/selftest_f0_bg.png";
    std::fs::write(path, &png).expect("写 PNG");
    println!("render-dump: {path} 落盘（{}B——fixture 面）", png.len());
}
