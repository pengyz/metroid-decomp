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

use crate::state::Manager;

impl Manager {
    /// `$4B PageIndex`——当前对象槽号（0..=15；门 8-11、子弹 12-15）。
    pub(crate) fn page_index(&self) -> u8 {
        self.read_ram(crate::ram_map::PAGE_INDEX)
    }

    pub(crate) fn set_page_index(&mut self, v: u8) {
        self.write_ram(crate::ram_map::PAGE_INDEX, v);
    }

    /// `$5A RoomNumber`——当前房间号（`$FF`=未定义房间；`$F0+` 是绘制游标的行号区间）。
    pub(crate) fn room_number(&self) -> u8 {
        self.read_ram(crate::ram_map::ROOM_NUMBER)
    }

    pub(crate) fn set_room_number(&mut self, v: u8) {
        self.write_ram(crate::ram_map::ROOM_NUMBER, v);
    }

    /// `$5B SpritePagePos`——下一个可用 OAM 槽（`$00`=OAM 槽 0）。
    pub(crate) fn sprite_page_pos(&self) -> u8 {
        self.read_ram(crate::ram_map::SPRITE_PAGE_POS)
    }

    pub(crate) fn set_sprite_page_pos(&mut self, v: u8) {
        self.write_ram(crate::ram_map::SPRITE_PAGE_POS, v);
    }

    /// `$6B ObjectCntrl`——精灵属性覆盖态；MSB=1 时 `SpriteAttrsOverride` 生效。
    pub(crate) fn obj_cntrl(&self) -> u8 {
        self.read_ram(crate::ram_map::OBJECT_CNTRL)
    }

    pub(crate) fn set_obj_cntrl(&mut self, v: u8) {
        self.write_ram(crate::ram_map::OBJECT_CNTRL, v);
    }

    /// `ClearObjectCntrl ; 07:DF2D`——帧终清覆盖态（`DrawSpriteObject` 的 `$FF` 终字节
    /// fallthrough 到这里）。单独给个名字是因为它必须在**每个绘制帧末**发生，漏一次就会
    /// 把覆盖态泄漏给同帧后续对象。
    pub(crate) fn clear_obj_cntrl(&mut self) {
        self.set_obj_cntrl(0);
    }
}
