//! WorldMap 烘焙面（CopyMap 源；window_bank=7 约定 A9 族——00:A53E → prg[0x253E]）。
//!
//! asm 引证：refs/m1disasm/SRC/prg0_title.asm:5118 `WorldMap: ; 00:A53E`（世界地图 ROM 表，
//! 1KB）+ :5153 `CopyMap: ; 00:A93E`（拷至 WorldMapRAM）+ constants_ram.asm:1298
//! `WorldMapRAM ds $400 ;$7000-$73FF`。
//! 数据自证：prg[0x253E..0x293E]（511/1024 非 FF——实测地图面）。

/// WorldMap ROM 数据（1KB；CopyMap 源面）。
pub use game_data::world::world_map::WORLD_MAP;

/// WorldMapRAM 基址（constants_ram.asm:1298「$7000-$73FF. The map is 1Kb in size」）。
pub const WORLD_MAP_RAM_BASE: u16 = 0x7000;

/// WorldMap 长度（4 页=1024B；`ldx #$04/dex/bne` 4 迭代自证）。
pub const WORLD_MAP_LEN: usize = 0x400;
