//! 对象池：`$0300-$03FF` 的 16 个 16 字节槽（门、子弹、道具等）。
//!
//! 与敌池（`enemy/state.rs`）同构：**池才是聚合根**，槽句柄只是借用视图。
//! 0 号槽是 Samus（见 `docs/spec/entity-scene-model.md` D-01），但本模块**不**假设
//! 谁是 Samus——识别 Samus 的唯一入口仍是 `Manager::samus()`。

use crate::frame::Manager;

/// 对象池基址（`Objects`）。
pub(crate) const OBJECTS_BASE: u16 = 0x0300;
/// 槽步长（字节）。
pub(crate) const SLOT_STRIDE: u16 = 16;
/// 槽数（`$0300-$03FF`）。
pub(crate) const SLOT_COUNT: u8 = 16;

/// `Object` 结构体字段偏移（`refs/m1disasm/out/M1_NES_NTSC.sym`，`Object` = `$0300`）。
pub(crate) mod obj {
    pub(crate) const STATUS: u16 = 0;
    pub(crate) const RADIUS_Y: u16 = 1;
    pub(crate) const RADIUS_X: u16 = 2;
    pub(crate) const ANIM_FRAME: u16 = 3;
    pub(crate) const ANIM_DELAY: u16 = 4;
    pub(crate) const ANIM_RESET: u16 = 5;
    pub(crate) const ANIM_INDEX: u16 = 6;
    /// 门专用（`DoorType`），其余对象为普通字段——同址多义，不美化（见 `samus/state.rs` 细则①）。
    pub(crate) const DOOR_TYPE: u16 = 7;
    pub(crate) const Y: u16 = 8;
    pub(crate) const X: u16 = 9;
    /// 已上屏标记（`onScreen`）：`Doors_RemoveIfOffScreen ; 07:ED65` 读 `$030B,x`。
    pub(crate) const ON_SCREEN: u16 = 0x0B;
}

/// 一个对象槽的借用视图。
pub struct Object<'a> {
    pub(crate) mem: &'a mut Manager,
    slot: u8,
}

impl<'a> Manager {
    /// 取对象槽句柄。槽号 0..=15，越界是编程错误（fail-loud）。
    pub fn object(&mut self, slot: u8) -> Object<'_> {
        assert!(slot < SLOT_COUNT, "对象槽号越界：{slot}（应为 0..=15）");
        Object { mem: self, slot }
    }
}

impl Object<'_> {
    #[inline]
    fn addr(&self, off: u16) -> u16 {
        OBJECTS_BASE + self.slot as u16 * SLOT_STRIDE + off
    }

    pub fn status(&self) -> u8 {
        self.mem.read_ram(self.addr(obj::STATUS))
    }

    pub fn set_status(&mut self, v: u8) {
        self.mem.write_ram(self.addr(obj::STATUS), v);
    }

    pub fn on_screen(&self) -> u8 {
        self.mem.read_ram(self.addr(obj::ON_SCREEN))
    }

    pub fn set_on_screen(&mut self, v: u8) {
        self.mem.write_ram(self.addr(obj::ON_SCREEN), v);
    }

    pub fn y(&self) -> u8 {
        self.mem.read_ram(self.addr(obj::Y))
    }

    pub fn x(&self) -> u8 {
        self.mem.read_ram(self.addr(obj::X))
    }

    pub fn door_type(&self) -> u8 {
        self.mem.read_ram(self.addr(obj::DOOR_TYPE))
    }
}

#[cfg(test)]
mod tests_object_pool {
    use crate::frame::Manager;

    /// 槽基址换算：槽 8 的 status 落在 `$0380`（门窗口起点），与 asm `Doors-Objects` 一致。
    #[test]
    fn slot_eight_maps_to_door_window() {
        let mut m = Manager::default();
        m.object(8).set_status(0x02);
        assert_eq!(m.read_ram(0x0380), 0x02);
        assert_eq!(m.object(8).status(), 0x02);
    }

    /// `onScreen` 是 `+$0B`：槽 10 ⇒ `$03AB`（`Doors_RemoveIfOffScreen ; 07:ED65` 的读点）。
    #[test]
    fn on_screen_offset_is_zero_b() {
        let mut m = Manager::default();
        m.object(10).set_on_screen(1);
        assert_eq!(m.read_ram(0x03AB), 1);
        assert_eq!(m.object(10).on_screen(), 1);
    }
}
