//! game-core —— 纯逻辑核（AGENTS.md D1/D7；无 I/O、确定性）。
//!
//! 分层：产品层（域模块，状态机语义）+ 映射层（memmap/ram_map，地址只此一层）。
//! 验收=L4 渲染帧对拍（tools/l4_diff_run.py）；分歧诊断=emu trace（Mesen2 真值）+m1disasm 人读对拍。

pub mod gap;
pub mod state;
pub mod enemy;
pub mod frame;
pub mod memmap;
pub mod ram_map;
pub mod samus;
pub mod rng;
pub mod world;
pub mod pickup;
pub mod input;
pub mod weapon;
pub mod ui;
pub mod render;
pub mod sound;
pub mod title;

/// ending 模块（1:1 Rust 改写·批产·块 12（s1_/s2_ 族（继续去地址名）））——refs asm 考证：bank07:CAF5-CB14（ChooseEnding）（结局例程（实址（A 级原文）））
pub mod ending;
