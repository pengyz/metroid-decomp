//! kernel::main_loop —— 主循环（帧调度+状态管理）。
//! refs 考证（bank:addr（A 级原文））：MainLoop（bank07:C0BC）。

/// 游戏态（帧计数+模式）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameState {
    pub frame_count: u8,
    pub game_mode: u8,
}

/// 主循环（帧调度+状态管理）——frame_count+1（wrapping）+模式分派面
/// （MainRoutinePtrTable 07:C141 三态+默认（模式字段不干预（判径保留）））。
pub fn main_loop(state: &mut GameState) {
    state.frame_count = state.frame_count.wrapping_add(1);
    match state.game_mode {
        0 => { /* 标题画面 */ }
        1 => { /* 游戏中 */ }
        2 => { /* 暂停 */ }
        _ => { /* 未知状态 */ }
    }
}
