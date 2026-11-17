//! 对象系统的**全局伪寄存器**（asm 全局变量，不属于任何单个实体）。
//!
//! 这四个字节在 asm 里是「对象/绘制系统正在用到的游标与覆盖位」，都是全局单例，
//! 谁都能写——正是这种「谁都能写」让 `$6B` 的覆盖态两次泄漏成调色板 bug
//! （`bulle-ish`：受伤敌写覆盖 → 同帧后续对象被染色）。收在这里的目的不是封装状态
//! （它们本来就是全局），而是：
//!
//! 1. **给出唯一写入点**：想写这四个字节，只能走本模块的方法 ⇒ 想知道「谁改了
//!    `ObjectCntrl`」时不必全仓 grep 字面量；
//! 2. **给每个字节一个语义名**：`0x005A` 在不同上下文被叫过 `RoomNumber` /
//!    `ScrollBlockOnNameTable3`（后者是**误读**：名表块在 `$6C/$6D`）——本模块的
//!    名字以 `constants_ram.asm` 为准；
//! 3. 让 `raw-surface` 账本可以逐文件往下压。
//!
//! 地址与 asm 锚（`refs/m1disasm/SRC/constants_ram.asm`）：
//! - `$4B PageIndex`：当前正在处理的对象槽号（`UpdateObjAnim ; 07:DC8F` 的 `ldx PageIndex`）
//! - `$5A RoomNumber`：当前房间号（`GetRoomNum ; 07:E758` / `DrawRoom` 自进）
//! - `$5B SpritePagePos`：下一个可用 OAM 槽（`DrawSpriteObject` 写完即推进）
//! - `$6B ObjectCntrl`：精灵属性覆盖态（MSB 置位 ⇒ `SpriteAttrsOverride ; 07:E038` 生效）

use crate::memmap::nesmap::{NesMapped, NesSpan, SpanKey};
use crate::state::Manager;

/// 迁入 typed 的三个全局游标地址（`$4B`/`$5A`/`$5B`）——三者不连续，故是三个 1 B span。
pub const OBJECT_GLOBALS_ADDRS: [u16; 5] = [
    crate::ram_map::PAGE_INDEX,
    crate::ram_map::ROOM_NUMBER,
    crate::ram_map::SPRITE_PAGE_POS,
    crate::ram_map::OBJECT_CNTRL,
    crate::ram_map::OBJECT_COUNTER,
];

/// 对象系统三个全局游标的 typed 真源。
///
/// 证据（B 级：代码语境 + asm 锚）：
/// - `$4B PageIndex`：`UpdateObjAnim ; 07:DC8F` 的 `ldx PageIndex`；本仓唯一读写在
///   [`Manager::page_index`]/[`Manager::set_page_index`]。
/// - `$5A RoomNumber`：`GetRoomNum ; 07:E758` 与 `DrawRoom` 自进；唯一读写在
///   [`Manager::room_number`]/[`Manager::set_room_number`]。
/// - `$5B SpritePagePos`：`DrawSpriteObject` 写完即推进的下一个 OAM 槽；唯一读写在
///   [`Manager::sprite_page_pos`]/[`Manager::set_sprite_page_pos`]。
/// - `$6B ObjectCntrl`：精灵属性覆盖态（MSB 置位 ⇒ `SpriteAttrsOverride ; 07:E038`），
///   读写在 [`Manager::obj_cntrl`]/[`Manager::set_obj_cntrl`]/[`Manager::clear_obj_cntrl`]，
///   帧终必须清（漏清 ⇒ 覆盖态泄漏成调色板 bug）。
/// - `$65 ObjectCounter`：外包对象循环的断环计数器（`ObjectCounter ; $65`，本仓唯一
///   读写点是 [`Manager::object_counter`]/[`Manager::set_object_counter`]）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ObjectGlobals {
    page_index: u8,
    room_number: u8,
    sprite_page_pos: u8,
    obj_cntrl: u8,
    object_counter: u8,
}

impl ObjectGlobals {
    /// 迁入 typed 的地址清单（`restore_mss` 回灌/清零与 `snapshot_nes` 投影共用）。
    pub const ADDRS: [u16; 5] = OBJECT_GLOBALS_ADDRS;

    pub fn page_index(&self) -> u8 {
        self.page_index
    }

    pub fn set_page_index(&mut self, v: u8) {
        self.page_index = v;
    }

    pub fn room_number(&self) -> u8 {
        self.room_number
    }

    pub fn set_room_number(&mut self, v: u8) {
        self.room_number = v;
    }

    pub fn sprite_page_pos(&self) -> u8 {
        self.sprite_page_pos
    }

    pub fn set_sprite_page_pos(&mut self, v: u8) {
        self.sprite_page_pos = v;
    }

    pub fn obj_cntrl(&self) -> u8 {
        self.obj_cntrl
    }

    pub fn set_obj_cntrl(&mut self, v: u8) {
        self.obj_cntrl = v;
    }

    pub fn object_counter(&self) -> u8 {
        self.object_counter
    }

    pub fn set_object_counter(&mut self, v: u8) {
        self.object_counter = v;
    }

    /// 地址 → 字节（不在本域返回 `None`）。
    pub fn get_byte(&self, addr: u16) -> Option<u8> {
        match addr {
            a if a == Self::ADDRS[0] => Some(self.page_index),
            a if a == Self::ADDRS[1] => Some(self.room_number),
            a if a == Self::ADDRS[2] => Some(self.sprite_page_pos),
            a if a == Self::ADDRS[3] => Some(self.obj_cntrl),
            a if a == Self::ADDRS[4] => Some(self.object_counter),
            _ => None,
        }
    }

    /// 字节 → 地址（不在本域返回 `false`，调用方继续往下一域分派）。
    pub fn set_byte(&mut self, addr: u16, v: u8) -> bool {
        match addr {
            a if a == Self::ADDRS[0] => self.page_index = v,
            a if a == Self::ADDRS[1] => self.room_number = v,
            a if a == Self::ADDRS[2] => self.sprite_page_pos = v,
            a if a == Self::ADDRS[3] => self.obj_cntrl = v,
            a if a == Self::ADDRS[4] => self.object_counter = v,
            _ => return false,
        }
        true
    }
}

impl NesMapped for ObjectGlobals {
    const DISCRIMINATOR: Option<u16> = None;

    /// 五个**不连续**的 1 B `Owner`（`$4B`/`$5A`/`$5B`/`$6B`/`$65`）——asm 侧是五个独立
    /// 全局变量，不是一段数组，故不给整段 span（会把中间字节一起吞掉）。
    const SPANS: &'static [(NesSpan, SpanKey)] = &[
        (NesSpan::value(Self::ADDRS[0], 1), SpanKey::Owner),
        (NesSpan::value(Self::ADDRS[1], 1), SpanKey::Owner),
        (NesSpan::value(Self::ADDRS[2], 1), SpanKey::Owner),
        (NesSpan::value(Self::ADDRS[3], 1), SpanKey::Owner),
        (NesSpan::value(Self::ADDRS[4], 1), SpanKey::Owner),
    ];

    fn project(&self, out: &mut crate::memmap::storage::NesImage) {
        out.ram[(Self::ADDRS[0] & 0x7FF) as usize] = self.page_index;
        out.ram[(Self::ADDRS[1] & 0x7FF) as usize] = self.room_number;
        out.ram[(Self::ADDRS[2] & 0x7FF) as usize] = self.sprite_page_pos;
        out.ram[(Self::ADDRS[3] & 0x7FF) as usize] = self.obj_cntrl;
        out.ram[(Self::ADDRS[4] & 0x7FF) as usize] = self.object_counter;
    }
}

impl Manager {
    /// `$4B PageIndex`——当前对象槽号（0..=15；门 8-11、子弹 12-15）。
    pub(crate) fn page_index(&self) -> u8 {
        self.object_globals.page_index()
    }

    pub(crate) fn set_page_index(&mut self, v: u8) {
        self.object_globals.set_page_index(v);
    }

    /// `$5A RoomNumber`——当前房间号（`$FF`=未定义房间；`$F0+` 是绘制游标的行号区间）。
    pub(crate) fn room_number(&self) -> u8 {
        self.object_globals.room_number()
    }

    pub(crate) fn set_room_number(&mut self, v: u8) {
        self.object_globals.set_room_number(v);
    }

    /// `$5B SpritePagePos`——下一个可用 OAM 槽（`$00`=OAM 槽 0）。
    pub(crate) fn sprite_page_pos(&self) -> u8 {
        self.object_globals.sprite_page_pos()
    }

    pub(crate) fn set_sprite_page_pos(&mut self, v: u8) {
        self.object_globals.set_sprite_page_pos(v);
    }

    /// `$6B ObjectCntrl`——精灵属性覆盖态；MSB=1 时 `SpriteAttrsOverride` 生效。
    pub(crate) fn obj_cntrl(&self) -> u8 {
        self.object_globals.obj_cntrl()
    }

    pub(crate) fn set_obj_cntrl(&mut self, v: u8) {
        self.object_globals.set_obj_cntrl(v);
    }

    /// `ClearObjectCntrl ; 07:DF2D`——帧终清覆盖态（`DrawSpriteObject` 的 `$FF` 终字节
    /// fallthrough 到这里）。单独给个名字是因为它必须在**每个绘制帧末**发生，漏一次就会
    /// 把覆盖态泄漏给同帧后续对象。
    pub(crate) fn clear_obj_cntrl(&mut self) {
        self.set_obj_cntrl(0);
    }
}

/// `ObjectCounter ; $65`：全屏对象循环的**断环计数器**（`SamusSetExplode` 清零、
/// `SamusMoveHorizontally` 置 1 断环、`samus/state.rs` 递减）。它不是实体状态，
/// 而是「本帧对象循环提前结束」的全局信号 ⇒ 归 object_globals。
impl Manager {
    pub(crate) fn object_counter(&self) -> u8 {
        self.object_globals.object_counter()
    }

    pub(crate) fn set_object_counter(&mut self, v: u8) {
        self.object_globals.set_object_counter(v);
    }
}

#[cfg(test)]
mod typed_tests {
    use super::*;
    use crate::frame::Manager;

    /// 三个游标是 typed 真源：typed 写 ⇒ 字节面读回同值、storage 不持有副本、
    /// `snapshot_nes()` 投影同源；字节面写 ⇒ typed 立刻可见（读/写径都走 typed）。
    #[test]
    fn object_globals_are_typed() {
        let mut m = Manager::default();
        m.set_page_index(3);
        m.set_room_number(0x12);
        m.set_sprite_page_pos(7);
        m.set_obj_cntrl(0x81);
        m.set_object_counter(4);
        assert_eq!(m.read_ram(crate::ram_map::PAGE_INDEX), 3);
        assert_eq!(m.read_ram(crate::ram_map::ROOM_NUMBER), 0x12);
        assert_eq!(m.read_ram(crate::ram_map::SPRITE_PAGE_POS), 7);
        assert_eq!(m.read_ram(crate::ram_map::OBJECT_CNTRL), 0x81);
        assert_eq!(m.read_ram(crate::ram_map::OBJECT_COUNTER), 4);
        for a in ObjectGlobals::ADDRS {
            assert_eq!(m.ram()[a as usize & 0x7FF], 0, "字节面不持有 typed 字节");
        }
        assert_eq!(m.snapshot_nes().ram[crate::ram_map::PAGE_INDEX as usize], 3);
        assert_eq!(m.snapshot_nes().ram[crate::ram_map::ROOM_NUMBER as usize], 0x12);
        assert_eq!(m.snapshot_nes().ram[crate::ram_map::SPRITE_PAGE_POS as usize], 7);
        assert_eq!(m.snapshot_nes().ram[crate::ram_map::OBJECT_CNTRL as usize], 0x81);
        m.clear_obj_cntrl();
        assert_eq!(m.obj_cntrl(), 0, "帧终清覆盖态");
        // 镜像别名同格：`$084B` 与 `$004B` 是同一格（2KB 镜像归一先于 typed 分派）。
        assert_eq!(m.read_ram(0x084B), 3);
        m.write_ram(0x085A, 0x34);
        assert_eq!(m.room_number(), 0x34);
    }

    /// `restore_mss` 回灌三个游标、storage 清零（清单覆盖本域）。
    #[test]
    fn object_globals_are_restored_and_cleared_from_mss() {
        let mut ram = vec![0u8; 0x800];
        ram[crate::ram_map::PAGE_INDEX as usize] = 0x0B;
        ram[crate::ram_map::ROOM_NUMBER as usize] = 0x23;
        ram[crate::ram_map::SPRITE_PAGE_POS as usize] = 0x08;
        ram[crate::ram_map::OBJECT_CNTRL as usize] = 0x82;
        ram[crate::ram_map::OBJECT_COUNTER as usize] = 5;
        let wram = vec![0u8; 0x2000];
        let m = Manager::restore_mss(&ram, &wram, crate::state::Registers::default());
        assert_eq!(m.page_index(), 0x0B);
        assert_eq!(m.room_number(), 0x23);
        assert_eq!(m.sprite_page_pos(), 0x08);
        assert_eq!(m.obj_cntrl(), 0x82);
        assert_eq!(m.object_counter(), 5);
        for a in ObjectGlobals::ADDRS {
            assert_eq!(m.ram()[a as usize & 0x7FF], 0, "typed 格在 storage 清零");
        }
    }
}
