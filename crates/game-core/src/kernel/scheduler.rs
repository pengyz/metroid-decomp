//! kernel::scheduler —— 主调度（游戏状态分派）。
//! refs 考证（bank:addr（A 级原文））：GoMainRoutine（bank07:C114）+MainRoutinePtrTable
//! （bank07:C141（状态→函数映射））。

/// 主调度（游戏状态分派）——0/1/2 恒等（MainRoutinePtrTable 07:C141 三态）；未知⇒0（默认标题）。
pub fn go_main_routine(game_mode: u8) -> u8 {
    match game_mode {
        0 => 0,
        1 => 1,
        2 => 2,
        _ => 0,
    }
}
