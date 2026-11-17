//! 房间 RAM（`$6000-$67FF`）与它的三个游标——**唯一**允许做房间地址算术的地方。
//!
//! asm 依据：
//! - 缓冲本体 `RoomRAM`：`$6000-$67FF`（2 KB = 两个名表页，`$6000`/`$6400`，
//!   由 `hi` 的 bit2 选页——`nt_hi & $04`）；
//! - 写游标 `RoomRAMWorkPtr`：`$37/$38`（`FillRoomRAM`/`SetupRoom` 用它顺序写）；
//! - 缓冲基址 `RoomRAMPtr`：`$39/$3A`（`$00`/`$60`/`$64` 之一，`DrawRoom` 用它读）；
//! - 当前房号 `RoomPtr`：`$33`。
//!
//! 判据（与 `world/object.rs`、`enemy/state.rs` 同）：**基址、页选、行/列换算只在这里**；
//! 业务层只经 [`Room`] 的语义方法读写，不再自己拼 `hi<<8|lo`。

use crate::frame::Manager;
use crate::memmap::nesmap::{NesSpan, SpanKey};

/// 房间缓冲基址（`RoomRAM` 的低页）。
pub const ROOM_RAM_BASE: u16 = 0x6000;
/// 房间缓冲长度（2 KB：`$6000-$67FF`）。
pub const ROOM_RAM_LEN: u16 = 0x800;

/// `RoomRAMWorkPtr`（顺序写游标）。
pub const WORK_PTR: u16 = 0x0037;
/// `RoomRAMPtr`（读基址，取值 `$00`/`$60`/`$64`）。
pub const ROOM_PTR: u16 = 0x0039;
/// 当前房号。
pub const ROOM_NUMBER: u16 = 0x0033;

/// 房间缓冲句柄：短生命周期借用，不持状态。
pub struct Room<'a> {
    mem: &'a mut Manager,
}

impl Manager {
    /// 派生房间缓冲句柄。
    pub fn room(&mut self) -> Room<'_> {
        Room { mem: self }
    }
}

impl<'a> Room<'a> {
    /// 行/列 → 房间缓冲内偏移（asm `StoreRoomTile`/`MakeRoomRAMPtr` 的换算）。
    ///
    /// `hi` 取 `$60 | Ybit7<<1 | Ybit6 | (nt_hi & $04)`：低位两比特是行号的高两位
    /// （每页 8 行 × 32 列 = 256 B，两页共 16 行），bit2 是名表页选。
    pub fn offset(y: u8, x: u8, nt_hi: u8) -> u16 {
        let mut hi = 0x60u8;
        hi |= ((y >> 7) & 1) << 1;
        hi |= (y >> 6) & 1;
        hi |= nt_hi & 0x04;
        let lo = ((y & 0x38) << 2) | (x >> 3);
        (((hi & 0x07) as u16) << 8) | lo as u16
    }

    /// 行/列 → **绝对**地址（`$6000 + offset`）。
    pub fn addr(y: u8, x: u8, nt_hi: u8) -> u16 {
        ROOM_RAM_BASE + Self::offset(y, x, nt_hi)
    }

    /// 读瓦片 id（越界行/列由调用方保证，asm 同）。
    pub fn tile(&self, y: u8, x: u8, nt_hi: u8) -> u8 {
        let a = Self::addr(y, x, nt_hi);
        self.mem.read_wram(a)
    }

    /// 写瓦片 id。
    pub fn set_tile(&mut self, y: u8, x: u8, nt_hi: u8, v: u8) {
        let a = Self::addr(y, x, nt_hi);
        self.mem.write_wram(a, v);
    }

    /// 顺序写游标 `RoomRAMWorkPtr`。
    pub fn work_ptr(&self) -> u16 {
        u16::from_le_bytes([self.mem.read_ram(WORK_PTR), self.mem.read_ram(WORK_PTR + 1)])
    }

    /// 设顺序写游标。
    pub fn set_work_ptr(&mut self, v: u16) {
        let [lo, hi] = v.to_le_bytes();
        self.mem.write_ram(WORK_PTR, lo);
        self.mem.write_ram(WORK_PTR + 1, hi);
    }

    /// 读基址 `RoomRAMPtr`（`$00`/`$60`/`$64`）。
    pub fn room_ptr(&self) -> u16 {
        u16::from_le_bytes([self.mem.read_ram(ROOM_PTR), self.mem.read_ram(ROOM_PTR + 1)])
    }

    /// 设读基址 `RoomRAMPtr`（asm `sta RoomRAMPtr/sta RoomRAMPtr+1`）。
    pub fn set_room_ptr(&mut self, v: u16) {
        let [lo, hi] = v.to_le_bytes();
        self.mem.write_ram(ROOM_PTR, lo);
        self.mem.write_ram(ROOM_PTR + 1, hi);
    }

    /// 当前房号（`RoomPtr`）。
    pub fn number(&self) -> u8 {
        self.mem.read_ram(ROOM_NUMBER)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXPECTED: &[(u8, u8, u8, u16)] = &[
        (0x50, 0xE8, 0, 0x615D), // hi=$61 lo=$5D
        (0x50, 0x10, 0, 0x6142), // hi=$61 lo=$42
        (0x00, 0x00, 4, 0x6400), // hi=$64（页选）lo=$00
        (0xFF, 0xF8, 4, 0x67FF), // hi=$67 lo=$FF
    ];

    #[test]
    fn offset_matches_make_room_ram_ptr() {
        for &(y, x, nt, want) in EXPECTED {
            // 断言**绝对地址**（缓冲内偏移 + $6000），即 asm `MakeRoomRAMPtr` 的 hi:lo 合成值。
            assert_eq!(Room::addr(y, x, nt), want, "y={y:#04X} x={x:#04X} nt={nt}");
        }
    }

    #[test]
    fn addr_and_tile_roundtrip_inside_buffer() {
        let mut m = Manager::default();
        m.room().set_tile(0x50, 0xE8, 0, 0x5A);
        assert_eq!(m.room().tile(0x50, 0xE8, 0), 0x5A);
        let a = Room::addr(0x50, 0xE8, 0);
        assert!(a >= ROOM_RAM_BASE && a < ROOM_RAM_BASE + ROOM_RAM_LEN);
    }

    #[test]
    fn cursors_and_room_number_are_byte_little_endian() {
        let mut m = Manager::default();
        m.room().set_work_ptr(0x6412);
        assert_eq!(m.room().work_ptr(), 0x6412);
        assert_eq!(m.read_ram(0x0037), 0x12);
        assert_eq!(m.read_ram(0x0038), 0x64);
        assert_eq!(m.read_ram(0x0039), 0x00); // 基址独立于写游标
    }
}

// ─────────────────────────── P2：`NesMapped` 声明 ───────────────────────────

/// 房间缓冲（WRAM 瓦片矩阵）未 typed 的缺口说明。
const ROOM_BUF_UNMODELED: &str = "房间缓冲未 typed：瓦片真相在 WRAM 字节面（$6000 起 2 KB；crates/game-core/src/world/room_ram.rs:17）";
/// 房间游标（`RoomPtr`/`RoomRAMWorkPtr`/`RoomNumber`）未 typed 的缺口说明。
const ROOM_CURSOR_UNMODELED: &str = "房间游标未 typed：小端指针与房间号真相在字节面（crates/game-core/src/world/room_ram.rs:156）";

/// 房间域 `NesMapped` 声明：WRAM 房间缓冲整块 + 三个游标，全部未 typed（0 个 owner）。
///
/// 缓冲 `$6000-$67FF` 不在 P2 对拍面（`$0000-$07FF ∪ $6AF4-$6BD3`）内，但它仍是实体
/// 字节，按「每字节有主」纪律登记为 `Unmodeled`（typed 债，§2 的迁移清单里）。
impl<'a> crate::memmap::nesmap::NesMapped for Room<'a> {
    const DISCRIMINATOR: Option<u16> = None;

    const SPANS: &'static [(NesSpan, SpanKey)] = &[
        (
            NesSpan::value(ROOM_RAM_BASE, ROOM_RAM_LEN),
            SpanKey::Unmodeled(ROOM_BUF_UNMODELED),
        ),
        (
            NesSpan::value(ROOM_NUMBER, 1),
            SpanKey::Unmodeled(ROOM_CURSOR_UNMODELED),
        ),
        (
            NesSpan::value(WORK_PTR, 2),
            SpanKey::Unmodeled(ROOM_CURSOR_UNMODELED),
        ),
        (
            NesSpan::value(ROOM_PTR, 2),
            SpanKey::Unmodeled(ROOM_CURSOR_UNMODELED),
        ),
    ];

    /// 0 个 `Owner` 段 ⇒ 无可投影的真相（缓冲由 `storage` 字节面持有）。
    fn project(&self, _out: &mut crate::memmap::storage::NesImage) {}
}
