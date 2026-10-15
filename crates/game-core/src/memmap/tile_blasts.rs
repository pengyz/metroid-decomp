//! Tile blast 槽视图 ↔ RAM $0500 字节域（地址只活在本层——two-layer §1）。
//!
//! 地址证据（A 级）：bank07:FE83 `9D 00 05` STA $0500,X（routine 删零）；
//! bank07:FE86 `BD 08 05`/`BD 09 05` LDA $0508,X/$0509,X（roomRAMPtr）。
//! 结构证据：refs/m1disasm/SRC/constants_ram.asm:36 `.struct TileBlast`（16B 槽：
//! routine@0、roomRAMPtr@8 dw）+ :964 `TileBlasts instanceof TileBlast $D startfrom 0`
//! （13 槽 $0500-$05CF）。命名台账 N-0064（第三方名采纳自证）。
//! 语义：tile blast 复活/爆炸动画状态块（B 级——FE83/UpdateTileBlastAnim 行为触达）。

/// 槽基址（RAM）。
pub const TILE_BLASTS_BASE: u16 = 0x0500;

/// 槽字节宽（.struct TileBlast=16B）。
pub const TILE_BLAST_SLOT_SIZE: u16 = 0x10;

/// routine 字段偏移（槽内 @0；FE83 删槽=写 0）。
pub const OFF_ROUTINE: u16 = 0x00;

/// roomRAMPtr 字段偏移（槽内 @8 dw，低在前；FE83 读双字节）。
pub const OFF_ROOM_RAM_PTR: u16 = 0x08;

/// 槽 routine 字段读。
pub fn read_routine(ram: &[u8], x: u8) -> u8 {
    ram[(TILE_BLASTS_BASE + OFF_ROUTINE + x as u16) as usize & 0x7FF]
}

/// 槽 roomRAMPtr 读（16 位，低在前）。
pub fn read_room_ram_ptr(ram: &[u8], x: u8) -> u16 {
    let base = (TILE_BLASTS_BASE + OFF_ROOM_RAM_PTR + x as u16) as usize & 0x7FF;
    ram[base] as u16 | ((ram[(base + 1) & 0x7FF] as u16) << 8)
}

/// 语义变更点 → 写事件（routine 字段；同值写也是事件）。
pub fn write_event_routine(x: u8, v: u8) -> (u16, u8) {
    (TILE_BLASTS_BASE + OFF_ROUTINE + x as u16, v)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 读写往返（偏移契约面；X 任意槽偏移 0..0xC0）。
    #[test]
    fn slot_read_write_roundtrip() {
        let mut ram = vec![0u8; 0x800];
        ram[0x0500 + 0x30] = 0x03;
        ram[0x0508 + 0x30] = 0x43;
        ram[0x0509 + 0x30] = 0x05;
        assert_eq!(read_routine(&ram, 0x30), 0x03);
        assert_eq!(read_room_ram_ptr(&ram, 0x30), 0x0543);
        assert_eq!(write_event_routine(0x30, 0), (0x0530, 0));
    }
}
