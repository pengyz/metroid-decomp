//! 对象池：`$0300-$03FF` 的 16 个 16 字节槽（门、子弹、道具等）。
//!
//! 与敌池（`enemy/state.rs`）同构：**池才是聚合根**，槽句柄只是借用视图。
//! 0 号槽是 Samus（见 `docs/spec/entity-scene-model.md` D-01），但本模块**不**假设
//! 谁是 Samus——识别 Samus 的唯一入口仍是 `Manager::samus()`。

use crate::frame::Manager;
use crate::memmap::nesmap::{NesSpan, SpanKey};

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
    /// `speedY`（`$0308`，i8）。
    pub(crate) const SPEED_Y: u16 = 8;
    /// `speedX`（`$0309`，i8）。
    pub(crate) const SPEED_X: u16 = 9;
    /// `y`（`$030D`）——房间坐标，非屏幕坐标。
    pub(crate) const Y: u16 = 0x0D;
    /// `x`（`$030E`）。
    pub(crate) const X: u16 = 0x0E;
    /// 已上屏标记（`onScreen`）：`Doors_RemoveIfOffScreen ; 07:ED65` 读 `$030B,x`。
    pub(crate) const ON_SCREEN: u16 = 0x0B;
    /// `SamusProjectileDieDelay`（`$030F` 族）——炸弹引信/爆炸相位计数。
    pub(crate) const DIE_DELAY: u16 = 0x0F;
    /// `isHit`（`$030A`）。
    pub(crate) const IS_HIT: u16 = 0x0A;
    /// `hi`（`$030C`，名表号高位 / 目标名表）。
    pub(crate) const HI: u16 = 0x0C;
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

    /// `radiusY`（`$0301`）——门槽同址多义为 `DoorHitPoints`（`SpawnDoor` 写 `$01`）。
    pub fn radius_y(&self) -> u8 {
        self.mem.read_ram(self.addr(obj::RADIUS_Y))
    }
    pub fn set_radius_y(&mut self, v: u8) {
        self.mem.write_ram(self.addr(obj::RADIUS_Y), v);
    }

    pub fn y(&self) -> u8 {
        self.mem.read_ram(self.addr(obj::Y))
    }

    pub fn x(&self) -> u8 {
        self.mem.read_ram(self.addr(obj::X))
    }

    /// `$0308 speedY`（i8）。
    pub fn speed_y(&self) -> i8 {
        self.mem.read_ram(self.addr(obj::SPEED_Y)) as i8
    }

    pub fn set_speed_y(&mut self, v: i8) {
        self.mem.write_ram(self.addr(obj::SPEED_Y), v as u8);
    }

    /// `$0309 speedX`（i8）。
    pub fn speed_x(&self) -> i8 {
        self.mem.read_ram(self.addr(obj::SPEED_X)) as i8
    }

    pub fn set_speed_x(&mut self, v: i8) {
        self.mem.write_ram(self.addr(obj::SPEED_X), v as u8);
    }

    pub fn door_type(&self) -> u8 {
        self.mem.read_ram(self.addr(obj::DOOR_TYPE))
    }

    pub fn set_y(&mut self, v: u8) {
        self.mem.write_ram(self.addr(obj::Y), v);
    }

    pub fn set_x(&mut self, v: u8) {
        self.mem.write_ram(self.addr(obj::X), v);
    }

    pub fn set_door_type(&mut self, v: u8) {
        self.mem.write_ram(self.addr(obj::DOOR_TYPE), v);
    }

    /// `$030A isHit`（门绘制前清零；子弹命中标记）。
    /// `SamusProjectileDieDelay`（`$030F` 族）——炸弹引信剩余帧数 / 爆炸相位计数。
    pub fn die_delay(&self) -> u8 {
        self.mem.read_ram(self.addr(obj::DIE_DELAY))
    }

    /// 设 `SamusProjectileDieDelay`。
    pub fn set_die_delay(&mut self, v: u8) {
        self.mem.write_ram(self.addr(obj::DIE_DELAY), v);
    }

    /// `animFrame`（`$0303` 族）——当前显示帧。
    pub fn anim_frame(&self) -> u8 {
        self.mem.read_ram(self.addr(obj::ANIM_FRAME))
    }

    /// 设 `animFrame`。
    pub fn set_anim_frame(&mut self, v: u8) {
        self.mem.write_ram(self.addr(obj::ANIM_FRAME), v);
    }

    /// `animDelay`（`$0304` 族）——帧间隔计数。
    pub fn anim_delay(&self) -> u8 {
        self.mem.read_ram(self.addr(obj::ANIM_DELAY))
    }

    /// 设 `animDelay`。
    pub fn set_anim_delay(&mut self, v: u8) {
        self.mem.write_ram(self.addr(obj::ANIM_DELAY), v);
    }

    /// `animResetIndex`（`$0305` 族）——动画回绕点。
    pub fn anim_reset_index(&self) -> u8 {
        self.mem.read_ram(self.addr(obj::ANIM_RESET))
    }

    /// 设 `animResetIndex`。
    pub fn set_anim_reset_index(&mut self, v: u8) {
        self.mem.write_ram(self.addr(obj::ANIM_RESET), v);
    }

    /// `animIndex`（`$0306` 族）——动画序列表游标。
    pub fn anim_index(&self) -> u8 {
        self.mem.read_ram(self.addr(obj::ANIM_INDEX))
    }

    /// 设 `animIndex`。
    pub fn set_anim_index(&mut self, v: u8) {
        self.mem.write_ram(self.addr(obj::ANIM_INDEX), v);
    }

    pub fn is_hit(&self) -> u8 {
        self.mem.read_ram(self.addr(obj::IS_HIT))
    }

    pub fn set_is_hit(&mut self, v: u8) {
        self.mem.write_ram(self.addr(obj::IS_HIT), v);
    }

    /// `$030C hi`（对象所在名表号；门用它决定写哪页）。
    pub fn hi(&self) -> u8 {
        self.mem.read_ram(self.addr(obj::HI))
    }

    pub fn set_hi(&mut self, v: u8) {
        self.mem.write_ram(self.addr(obj::HI), v);
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

// ─────────────────────────── P2：`NesMapped` 声明 ───────────────────────────

/// 对象池未 typed 的缺口说明。
const OBJ_UNMODELED: &str = "对象池未 typed：门/子弹/道具字段的真相仍在字节面（本池只是读写视图；crates/game-core/src/world/object.rs:11）";

/// 对象池 `$0300-$03FF`（16 槽 × 16 B）的 `NesMapped` 声明。
///
/// **slot0 起前 23 B 是 Samus 的 typed overlay**：`$0300-$0316` 的真相在 `SamusState`
/// （`R2` 已证 `$0307` 同时是 `SamusOnElevator` 与对象槽 0 的 `DoorType`）⇒ 本池对这段
/// 只有**只读视图**（`View`），不抢 owner 位。其余 233 B 未 typed ⇒ `Unmodeled`。
///
/// `View` 长度取 `SamusState::TYPED_LEN`（不另立数字：Samus 的 typed 面一变，这里自动跟）。
impl<'a> crate::memmap::nesmap::NesMapped for Object<'a> {
    const DISCRIMINATOR: Option<u16> = None;

    const SPANS: &'static [(NesSpan, SpanKey)] = &[
        (
            NesSpan::value(OBJECTS_BASE, crate::samus::state::SamusState::TYPED_LEN),
            SpanKey::View(&["SamusState"]),
        ),
        (
            NesSpan::value(
                OBJECTS_BASE + crate::samus::state::SamusState::TYPED_LEN,
                SLOT_COUNT as u16 * SLOT_STRIDE - crate::samus::state::SamusState::TYPED_LEN,
            ),
            SpanKey::Unmodeled(OBJ_UNMODELED),
        ),
    ];

    /// 0 个 `Owner` 段 ⇒ 无可投影的真相（本池的字节由各持有域投影）。
    fn project(&self, _out: &mut crate::memmap::storage::NesImage) {}
}
