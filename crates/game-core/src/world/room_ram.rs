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

/// 瓦片"是否阻挡移动"判据（**asm 内容锚** `CheckMoveVertical ; 07:E7BD` 的瓦判链 →
/// `CheckBlastTile ; 07:D651` + `IsBlastTile ; 07:E9BE`）。
///
/// asm 三件合起来的语义（逐行读 `refs/m1disasm/SRC/prg7_engine.asm`）：
/// - `cmp #$4E / beq SamusProjectileHitDoorOrStatue`：门/像瓦——非弹更新径一律碰撞；
/// - `CheckBlastTile`：**固实阈值按区分**——Brinstar（`InArea==$10`）`cmp #$80`、
///   其余区 `cmp #$70`；低于阈值 ⇒ CF=0 ⇒ 固实；
/// - `cmp #$A0 / bcs IsWalkableTile`：`>=$A0` 可走（含 `$A0/$A1` 门瓦，Samus 另有判）；
/// - 余下 `[$80,$9F]`（Brinstar）/ `[$70,$9F]`（其余区）⇒ `IsBlastTile`：
///   `ldy UpdatingSamusProjectile / beq @RTS` ⇒ **非弹更新径直接 rts，CF 仍是
///   上一条 `cmp #$A0` 的 0 ⇒ 碰撞**；只有弹更新径才继续找空槽破坏瓦（CF=1 ⇒ 可走）。
///
/// 旧实现把 `>=$80` 一律当可走（注释「$80-$9F 可炸⇒简化视为可走」）——**错**：
/// 可炸瓦对 Samus/敌等非弹对象是固实块（要炸开才能过）。room 15 的 Waver 即因此
/// 提前 2 像素（emu 撞在 `$82/$83`，rust 放行）。
pub fn tile_blocks_movement(tile: u8, in_area: u8, updating_projectile: bool) -> bool {
    if tile == 0x4E {
        return true;
    }
    let solid_below = if in_area == 0x10 { 0x80 } else { 0x70 };
    if tile < solid_below {
        return true;
    }
    if tile >= 0xA0 {
        return false;
    }
    !updating_projectile
}

/// 非弹对象（Samus/敌）的可走性判据——[`tile_blocks_movement`] 在
/// `updating_projectile == false` 时的**等价化简**：`tile < $A0` 即固实。
///
/// 理由（asm 直接推得，非近似）：可炸瓦 `$80-$9F`（Brinstar）/`$70-$9F`（其余区）
/// 在非弹更新径一律走 `IsBlastTile` 的 `beq @RTS` 支 ⇒ 碰撞；而 `>=$A0` 一律
/// `bcs IsWalkableTile` ⇒ 可走。于是区阈值（`$80` vs `$70`）只影响弹更新径，
/// 对非弹对象整个判据塌成一条：`tile < $A0`。
/// 这样 Samus/敌的移动径不必读 `InArea`/`UpdatingSamusProjectile` 两个 RAM 字节
/// （也避免在移动热路径上增加裸口读数）。
pub fn tile_blocks_non_projectile(tile: u8) -> bool {
    tile < 0xA0
}

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

    /// `WritePPUAttribTbl ; 07:E5E2` 的写目标页对：`(NT 高字节, RoomRAM 源页高字节)`。
    ///
    /// asm：`jsr GetNameAddrs ; 07:E564` → `GetNameTableAtScrollDir ; 07:EB85` =
    /// `lda PPUCTRL_ZP / eor ScrollDir / and #$01` → `tay` →
    /// `lda PPUAddrs,y`（`$20,$2C`，写尾 `ora #$03` ⇒ `$23/$2F`）+
    /// `ldx RoomRAMAddrs,y`（`>RoomRAMA,>RoomRAMB` = `$60,$64`，写尾 `ora #$03` ⇒ `$63/$67`）。
    /// ⇒ **NT 与 RoomRAM 页由同一个 y 现算**，不能用载房时的 `RoomRAMPtr` 快照反推
    /// （`SelectRoomRAM` 之后 `PPUCTRL`/`ScrollDir` 可能已变）。
    pub fn attrib_write_pages(&mut self) -> (u16, u16) {
        let y = (self.mem.read_ram(crate::ram_map::PPUCTRL_ZP)
            ^ self.mem.read_ram(crate::ram_map::SCROLL_DIR))
            & 0x01;
        let nt_hi = if y == 0 { 0x23 } else { 0x2F };
        let src_hi = if y == 0 { 0x63 } else { 0x67 };
        (nt_hi, src_hi)
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
const ROOM_CURSOR_UNMODELED: &str = "房间游标未 typed：小端指针与房间号真相在字节面（crates/game-core/src/world/room_ram.rs:162）";

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

#[cfg(test)]
mod tests_tile_blocks {
    use super::tile_blocks_movement;

    /// asm `CheckBlastTile ; 07:D651`：固实阈值按区分（Brinstar `$80` / 其余 `$70`）。
    #[test]
    fn solid_threshold_depends_on_area() {
        // Brinstar（InArea=$10）：$7F 固实、$80 进可炸判
        assert!(tile_blocks_movement(0x7F, 0x10, false));
        assert!(tile_blocks_movement(0x80, 0x10, false));
        // 其余区：$6F 固实、$70 进可炸判
        assert!(tile_blocks_movement(0x6F, 0x11, false));
        assert!(tile_blocks_movement(0x70, 0x11, false));
        // 交叉：$70 在 Brinstar 是"可炸"（非固实阈值以上），非弹下仍碰撞；
        // 但 $7F 在非 Brinstar 已是可炸带 ⇒ 弹更新径可走
        assert!(!tile_blocks_movement(0x7F, 0x11, true));
    }

    /// asm `IsBlastTile ; 07:E9BE` 首件：**非弹更新径 ⇒ 可炸瓦碰撞**（CF 来自 `cmp #$A0`）。
    #[test]
    fn blastable_tiles_block_non_projectiles() {
        for t in [0x80u8, 0x82, 0x83, 0x9F] {
            assert!(tile_blocks_movement(t, 0x10, false), "Brinstar ${t:02X} 非弹应固实");
            assert!(!tile_blocks_movement(t, 0x10, true), "Brinstar ${t:02X} 弹更新径可走");
        }
        for t in [0x70u8, 0x76, 0x9F] {
            assert!(tile_blocks_movement(t, 0x11, false), "Norfair ${t:02X} 非弹应固实");
            assert!(!tile_blocks_movement(t, 0x11, true), "Norfair ${t:02X} 弹更新径可走");
        }
    }

    /// `>=$A0` 恒可走（`cmp #$A0 / bcs IsWalkableTile`）；`$4E` 门/像恒固实。
    #[test]
    fn walkable_and_door_tiles() {
        for t in [0xA0u8, 0xA1, 0xFF] {
            assert!(!tile_blocks_movement(t, 0x10, false));
            assert!(!tile_blocks_movement(t, 0x11, true));
        }
        assert!(tile_blocks_movement(0x4E, 0x10, false));
        assert!(tile_blocks_movement(0x4E, 0x11, true));
    }

    /// 化简等式：非弹径下 `tile_blocks_movement(t, area, false) == tile < $A0`
    /// 对全部 256 个 tile 与两个区阈值都成立（不是近似，是可证的等价）。
    #[test]
    fn non_projectile_specialization_is_exact() {
        for t in 0u16..=0xFF {
            let t = t as u8;
            for area in [0x10u8, 0x11] {
                assert_eq!(
                    tile_blocks_movement(t, area, false),
                    super::tile_blocks_non_projectile(t),
                    "tile=${t:02X} area=${area:02X}"
                );
            }
        }
    }
}
