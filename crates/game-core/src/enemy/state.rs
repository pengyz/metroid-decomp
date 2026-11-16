//! 敌槽语义视图——`Ens`（`$0400+i*16`）与 `EnsExtra`（`$6AF4+i*16`）的字段面。
//!
//! 地址证据（A 级）：`refs/m1disasm/out/M1_NES_NTSC.sym`——
//! `Ens.0 = $0400`、`EnsExtra.0 = $6AF4`，槽步长 16 B，共 12 槽（对象槽 0-11 之后）。
//!
//! 形态纪律（同 [`super::samus`]）：
//! - 槽号是**语义**，字节偏移是**实现**：`mem.enemy(slot)` 收 `slot: u8`（0..=11），
//!   内部换算 `x = slot * 16`；调用点不再写 `en_addr(x, f)` 这种双参数拼接。
//! - `$0406` 一字节三名（`chunkyExplosionTimer` / `movementInstrIndex` /
//!   `speedSubPixelY`）——`samus::state` 细则①「同址多义不美化」：本视图**分别**暴露
//!   三个访问器，各自写明 asm 语义，**不合并成一个"猜含义"的名字**；具体谁在用，
//!   由读代码的人按上下文判断（这正是原版 ROM 的设计：三处代码各按自己的语义用同一格）。

use crate::frame::Manager;
use crate::ram_map;

/// `Ens` 结构体的字段偏移（`$0400+i*16+f`）。
pub mod ens {
    pub const Y: u16 = 0;
    pub const X: u16 = 1;
    pub const SPEED_Y: u16 = 2;
    pub const SPEED_X: u16 = 3;
    pub const IS_HIT: u16 = 4;
    pub const DATA05: u16 = 5;
    /// **同址三名之二**：`chunkyExplosionTimer`（爆散计时）/ `movementInstrIndex`
    /// （移动指令指针）/ `speedSubPixelY`（竖速小数）——三处代码各按自己的语义用同一格。
    pub const DATA06: u16 = 6;
    pub const SPEED_SUBPIXEL_X: u16 = 7;
    pub const MOVEMENT_INDEX: u16 = 8;
    pub const DELAY: u16 = 9;
    pub const DATA0A: u16 = 10;
    pub const HEALTH: u16 = 11;
    pub const PREV_STATUS: u16 = 12;
    pub const DATA0D: u16 = 13;

    /// `$0406` 的三个语义名（见 [`DATA06`]）——**只登记不合并**。
    pub const DATA06_ALIASES: [&str; 3] =
        ["chunkyExplosionTimer", "movementInstrIndex", "speedSubPixelY"];
}

/// `EnsExtra` 结构体的字段偏移（`$6AF4+i*16+f`）。
pub mod ens_extra {
    pub const STATUS: u16 = 0;
    pub const RADIUS_Y: u16 = 1;
    pub const RADIUS_X: u16 = 2;
    pub const ANIM_FRAME: u16 = 3;
    pub const ANIM_DELAY: u16 = 4;
    pub const RESET_ANIM_INDEX: u16 = 5;
    pub const ANIM_INDEX: u16 = 6;
    pub const HI: u16 = 7;
    /// 竖加速（`$6AFE`）。
    pub const ACCEL_Y: u16 = 10;
    /// 横加速（`$6AFF`）。
    pub const ACCEL_X: u16 = 11;
    /// 敌类型（`$6B02`）。
    pub const TYPE: u16 = 14;
    pub const DATA1F: u16 = 15;
}

/// `EnsExtra.status`（`refs/m1disasm/SRC/constants.asm:135-142` 的 `enemyStatus_*`）。
///
/// asm 原文七态 + 一句"更都等于没有敌人"：`0 NoEnemy / 1 Resting / 2 Active /
/// 3 Explode / 4 Frozen / 5 Pickup / 6 Hurt`，`>6` 与原版一样按空槽处理。
/// 未定义值保真（`Undefined(u8)`）——不静默映射成 `NoEnemy`，否则"原版把 7 当空槽"
/// 这条 quirk 会被悄悄改成"从未见过 7"。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EnemyStatus {
    NoEnemy,
    Resting,
    Active,
    Explode,
    Frozen,
    Pickup,
    Hurt,
    Undefined(u8),
}

impl EnemyStatus {
    pub const ENUM_MAX: u8 = 6;

    pub fn from_byte(b: u8) -> Self {
        match b {
            0 => Self::NoEnemy,
            1 => Self::Resting,
            2 => Self::Active,
            3 => Self::Explode,
            4 => Self::Frozen,
            5 => Self::Pickup,
            6 => Self::Hurt,
            other => Self::Undefined(other),
        }
    }

    pub fn to_byte(self) -> u8 {
        match self {
            Self::NoEnemy => 0,
            Self::Resting => 1,
            Self::Active => 2,
            Self::Explode => 3,
            Self::Frozen => 4,
            Self::Pickup => 5,
            Self::Hurt => 6,
            Self::Undefined(b) => b,
        }
    }

    /// 空槽判据：`NoEnemy(0)` 本身是空槽，asm 另注"大于 6 同等于没有敌人"
    /// （`; greater than 6 is the same as no enemy`）⇒ 两段都算空。
    pub fn is_empty(self) -> bool {
        let b = self.to_byte();
        b == 0 || b > Self::ENUM_MAX
    }
}

/// 敌槽数（对象槽 0-11 之后，`Ens` 面 12×16 B）。
pub const SLOT_COUNT: u8 = 12;

/// **一个敌槽的语义句柄**（短生命周期，由 `Manager::enemy(slot)` 派生）。
pub struct Enemy<'a> {
    mem: &'a mut Manager,
    slot: u8,
}

impl<'a> Enemy<'a> {
    pub(crate) fn new(mem: &'a mut Manager, slot: u8) -> Self {
        assert!(slot < SLOT_COUNT, "敌槽号越界：{slot}（应为 0..={}）", SLOT_COUNT - 1);
        Self { mem, slot }
    }

    #[inline]
    pub fn slot(&self) -> u8 {
        self.slot
    }

    /// 槽在 `Ens` 面的**字节偏移**（= `slot*16`）——asm 例程普遍以它为 `x` 索引，
    /// 需要与既有 asm 形代码互操作时用它，日常请用语义访问器。
    #[inline]
    pub fn x(&self) -> u8 {
        self.slot * 16
    }

    #[inline]
    fn get(&self, f: u16) -> u8 {
        self.mem.read_ram(0x0400 + self.x() as u16 + f)
    }

    #[inline]
    fn set(&mut self, f: u16, v: u8) {
        self.mem.write_ram(0x0400 + self.x() as u16 + f, v);
    }

    #[inline]
    fn getx(&self, f: u16) -> u8 {
        self.mem.read_wram(0x6AF4 + self.x() as u16 + f)
    }

    #[inline]
    fn setx(&mut self, f: u16, v: u8) {
        self.mem.write_wram(0x6AF4 + self.x() as u16 + f, v);
    }

    // ---- Ens 面 ----
    pub fn y(&self) -> u8 {
        self.get(ens::Y)
    }
    pub fn set_y(&mut self, v: u8) {
        self.set(ens::Y, v);
    }
    pub fn pos_x(&self) -> u8 {
        self.get(ens::X)
    }
    pub fn set_pos_x(&mut self, v: u8) {
        self.set(ens::X, v);
    }
    pub fn speed_y(&self) -> i8 {
        self.get(ens::SPEED_Y) as i8
    }
    pub fn set_speed_y(&mut self, v: i8) {
        self.set(ens::SPEED_Y, v as u8);
    }
    pub fn speed_x(&self) -> i8 {
        self.get(ens::SPEED_X) as i8
    }
    pub fn set_speed_x(&mut self, v: i8) {
        self.set(ens::SPEED_X, v as u8);
    }
    /// `Ens.isHit`——位域（`SetEnemyTouchingSamusFlags` 的 `ora` 累加面），保持 `u8`。
    pub fn is_hit(&self) -> u8 {
        self.get(ens::IS_HIT)
    }
    pub fn data05(&self) -> u8 {
        self.get(ens::DATA05)
    }
    pub fn set_data05(&mut self, v: u8) {
        self.set(ens::DATA05, v);
    }
    /// `$0406` 的三语义读数——调用方**必须**说明自己按哪一种在用（见 [`ens::DATA06`]）。
    pub fn data06(&self) -> u8 {
        self.get(ens::DATA06)
    }
    pub fn set_data06(&mut self, v: u8) {
        self.set(ens::DATA06, v);
    }
    pub fn movement_index(&self) -> u8 {
        self.get(ens::MOVEMENT_INDEX)
    }
    pub fn set_movement_index(&mut self, v: u8) {
        self.set(ens::MOVEMENT_INDEX, v);
    }
    pub fn delay(&self) -> u8 {
        self.get(ens::DELAY)
    }
    pub fn set_delay(&mut self, v: u8) {
        self.set(ens::DELAY, v);
    }
    pub fn health(&self) -> u8 {
        self.get(ens::HEALTH)
    }
    pub fn set_health(&mut self, v: u8) {
        self.set(ens::HEALTH, v);
    }
    pub fn prev_status(&self) -> u8 {
        self.get(ens::PREV_STATUS)
    }

    // ---- EnsExtra 面 ----
    /// `EnsExtra.status` 原始字节（判 `>6` 这类数值序时用；语义判据请用 [`Self::status`]）。
    pub fn status_raw(&self) -> u8 {
        self.getx(ens_extra::STATUS)
    }
    /// `EnsExtra.status` 语义态（`enemyStatus_*`，含 `Undefined` 保真）。
    pub fn status(&self) -> EnemyStatus {
        EnemyStatus::from_byte(self.getx(ens_extra::STATUS))
    }
    pub fn set_status(&mut self, v: EnemyStatus) {
        self.setx(ens_extra::STATUS, v.to_byte());
    }

    /// asm `@becomeActive: inc EnsExtra.0.status,x`——**字节自增，不映射枚举**。
    ///
    /// 这条径在 ROM 里就是数值 `inc`：从 Resting(1) 走到 Active(2) 只是巧合地等于
    /// 枚举 +1；若哪天从 6 走 inc 会得到 7（原版按"没有敌人"处理）。枚举化会把这种
    /// 可达的 `Undefined` 变成不可表达，反而失真。
    pub fn inc_status(&mut self) {
        let v = self.status_raw().wrapping_add(1);
        self.setx(ens_extra::STATUS, v);
    }

    /// asm 静息径的 `dec EnsExtra.0.status,x`（Active→Resting）。保留字节语义，理由同 [`Self::inc_status`]。
    pub fn dec_status(&mut self) {
        let v = self.status_raw().wrapping_sub(1);
        self.setx(ens_extra::STATUS, v);
    }
    pub fn radius_y(&self) -> u8 {
        self.getx(ens_extra::RADIUS_Y)
    }
    pub fn radius_x(&self) -> u8 {
        self.getx(ens_extra::RADIUS_X)
    }
    pub fn anim_frame(&self) -> u8 {
        self.getx(ens_extra::ANIM_FRAME)
    }
    pub fn anim_index(&self) -> u8 {
        self.getx(ens_extra::ANIM_INDEX)
    }
    pub fn hi(&self) -> u8 {
        self.getx(ens_extra::HI)
    }
    pub fn set_hi(&mut self, v: u8) {
        self.setx(ens_extra::HI, v);
    }
    pub fn enemy_type(&self) -> u8 {
        self.getx(ens_extra::TYPE)
    }
    pub fn set_enemy_type(&mut self, v: u8) {
        self.setx(ens_extra::TYPE, v);
    }
    pub fn data1f(&self) -> u8 {
        self.getx(ens_extra::DATA1F)
    }
    pub fn set_data1f(&mut self, v: u8) {
        self.setx(ens_extra::DATA1F, v);
    }

    /// 用于跨域系统函数（拿 `&mut Manager` 的既有件）——本句柄不长期持有。
    pub fn mem(&mut self) -> &mut Manager {
        self.mem
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn m() -> Manager {
        Manager::default()
    }

    /// 槽号 ↔ 字节偏移：`slot*16`，且两面对同一槽必须落在 `$0400`/`$6AF4` 基址上。
    #[test]
    fn slot_maps_to_byte_offset() {
        let mut mem = m();
        assert_eq!(mem.enemy(0).x(), 0);
        assert_eq!(mem.enemy(2).x(), 32);
        mem.enemy(2).set_y(0x9D);
        assert_eq!(mem.read_ram(0x0420), 0x9D);
        mem.enemy(2).set_status(EnemyStatus::Active);
        assert_eq!(mem.read_wram(0x6B14), 0x02);
        assert_eq!(mem.enemy(2).status(), EnemyStatus::Active);
    }

    /// 越界槽号 fail-loud（不静默回绕到别的槽）。
    #[test]
    #[should_panic(expected = "敌槽号越界")]
    fn out_of_range_slot_fails_loud() {
        let mut mem = m();
        let _ = mem.enemy(SLOT_COUNT).status();
    }

    /// `$0406` 三名同址：一个写、三个读法都看同一格（这就是原版的设计，不能"美化"掉）。
    #[test]
    fn data06_is_one_byte_with_three_names() {
        let mut mem = m();
        mem.enemy(1).set_data06(0x3C);
        let e = mem.enemy(1);
        assert_eq!(e.data06(), 0x3C);
        assert_eq!(e.movement_index(), mem.enemy(1).movement_index()); // 不同格，仅确认访问器不串
        assert_eq!(ens::DATA06_ALIASES.len(), 3);
        assert_eq!(mem.read_ram(0x0410 + ens::DATA06), 0x3C);
    }

    /// 速度是二补数（`i8`）。
    #[test]
    fn speeds_are_signed() {
        let mut mem = m();
        mem.enemy(3).set_speed_y(-2);
        assert_eq!(mem.enemy(3).speed_y(), -2);
        assert_eq!(mem.read_ram(0x0430 + ens::SPEED_Y), 0xFE);
    }

    /// 七态逐字节往返 + 未定义值保真（`>6` 是原版 quirk：按空槽处理，但不能洗成 0）。
    #[test]
    fn status_enum_roundtrips_and_preserves_undefined() {
        let mut mem = m();
        for (b, want) in [
            (0u8, EnemyStatus::NoEnemy),
            (1, EnemyStatus::Resting),
            (2, EnemyStatus::Active),
            (3, EnemyStatus::Explode),
            (4, EnemyStatus::Frozen),
            (5, EnemyStatus::Pickup),
            (6, EnemyStatus::Hurt),
        ] {
            mem.enemy(3).set_status(want);
            assert_eq!(mem.enemy(3).status_raw(), b, "枚举→字节");
            assert_eq!(mem.enemy(3).status(), want, "字节→枚举");
            assert_eq!(want.is_empty(), b == 0, "只有 NoEnemy 是空槽");
        }
        let odd = EnemyStatus::Undefined(0x09);
        mem.enemy(3).set_status(odd);
        assert_eq!(mem.enemy(3).status(), odd, "未定义值原样保留");
        assert_eq!(mem.enemy(3).status_raw(), 0x09);
        assert!(odd.is_empty(), "asm：greater than 6 = 没有敌人");
        assert!(EnemyStatus::NoEnemy.is_empty());
    }
}
