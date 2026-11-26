//! game-core —— 纯逻辑核（AGENTS.md D1/D7；无 I/O、确定性）。
//!
//! 分层：产品层（域模块，状态机语义）+ 映射层（memmap/ram_map，地址只此一层）。
//! 验收=L4 渲染帧对拍（tools/l4_diff_run.py）；分歧诊断=emu trace（Mesen2 真值）+m1disasm 人读对拍。

/// 资产包（ROM 派生数据的唯一载体）——上游（shell/bin）统一从这里初始化，
/// 不必各自直连 `game-data`。见 `docs/spec/data-code-separation.md`。
pub use game_data::pack;

pub mod gap;
pub mod state;
pub mod enemy;
pub mod frame;
pub mod memmap;
pub mod ram_map;
pub mod samus;
pub mod rng;
pub mod arith;
pub mod world;
pub mod pickup;
pub mod input;
pub mod weapon;
pub mod ui;
pub mod render;
pub mod sound;
pub mod ending;
pub mod title;
