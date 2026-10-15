//! 标题状态面 ↔ zp 字节域（地址只活在本层——two-layer §1）。
//!
//! 地址证据（A 级）：bank00:$806E `e6 1f` INC zp $1F（TitleRoutine）；bank00:$8283
//! `e6 1f a9 10 85 2c`（Timer3←$10）；bank00:$8068 `a9 00 85 cb 85 c9`（SpareMemCB/C9）。
//! 常量内容锚：constants_ram.asm:479 `TitleRoutine db ;$1F`、:496 `Timer3 db ;$2C
//! ;Decremented every 10 frames after set`、:744-745 `SpareMemC9 dw ;$C9`（**写面只低字节**
//! ——`85 c9` zp 单址）/`SpareMemCB db ;$CB`。
//! TitleRoutine 地址唯一源=crate::ram_map::TITLE_ROUTINE（引擎级 zp 图既有件，不另立）。
//! 命名台账 N-0066（第三方名采纳自证）；批1 助手簇（title::HelperOutcome 语义面）映射。

use crate::ram_map::TITLE_ROUTINE;

/// Timer3=$2C（「Decremented every 10 frames after set」——0F DelayIntroReplay 写 $10
/// =160 帧延迟的单位面）。
pub const TIMER3: u16 = 0x002C;

/// SpareMemC9 低字节=$C9（「Written to in title routine, but never accessed」）。
pub const SPARE_C9: u16 = 0x00C9;

/// SpareMemCB=$CB（同上注）。
pub const SPARE_CB: u16 = 0x00CB;

/// TitleRoutine 读（相位面）。
pub fn read_phase(ram: &[u8]) -> u8 {
    ram[TITLE_ROUTINE as usize & 0x7FF]
}

/// 写事件：TitleRoutine（INC zp 后值）。
pub fn write_event_phase(v: u8) -> (u16, u8) {
    (TITLE_ROUTINE, v)
}

/// 写事件：Timer3。
pub fn write_event_timer3(v: u8) -> (u16, u8) {
    (TIMER3, v)
}

/// 写事件：SpareMemC9 低字节。
pub fn write_event_spare_c9(v: u8) -> (u16, u8) {
    (SPARE_C9, v)
}

/// 写事件：SpareMemCB。
pub fn write_event_spare_cb(v: u8) -> (u16, u8) {
    (SPARE_CB, v)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 地址唯一源+写事件面（$1F 复用 ram_map::TITLE_ROUTINE 不另立）。
    #[test]
    fn title_face_addresses_and_events() {
        let mut ram = vec![0u8; 0x800];
        ram[TITLE_ROUTINE as usize] = 0x0A;
        assert_eq!(read_phase(&ram), 0x0A);
        assert_eq!(write_event_phase(0x0B), (0x001F, 0x0B));
        assert_eq!(write_event_timer3(0x10), (TIMER3, 0x10));
        assert_eq!(write_event_spare_c9(0), (SPARE_C9, 0));
        assert_eq!(write_event_spare_cb(0), (SPARE_CB, 0));
    }
}
