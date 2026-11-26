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
use crate::memmap::nesmap::{NesSpan, SpanKey};
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
    /// `Ens.0.weaponAction`（`$040E`）——击杀它的武器类型，`ExplodeEnemy` 用它判「普通束」。
    pub const WEAPON_ACTION: u16 = 14;
    /// `Ens.0.specialAttribs`（`$040F`）——`$40`=miniboss（不播爆散/三连爆）。
    pub const SPECIAL_ATTRIBS: u16 = 15;

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
    /// 竖速小数（`$6AFC`）——`EnemyGetDeltaY_UsingAcceleration` 的累加面。
    pub const SUB_PIXEL_Y: u16 = 8;
    /// 横速小数（`$6AFD`）。
    pub const SUB_PIXEL_X: u16 = 9;
    /// 竖加速（`$6AFE`）。
    pub const ACCEL_Y: u16 = 10;
    /// 横加速（`$6AFF`）。
    pub const ACCEL_X: u16 = 11;
    /// 跳跃位移（`$6B01`）——Skree 的 `jumpDsplcmnt` 倒数。
    pub const JUMP_DSPLCMNT: u16 = 13;
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

/// `EnsExtra` 里被 typed 化的字节偏移（`$6AF4 + slot*16 + off`）。
/// 前 7 字节成三个**不变量组**：status（独占）、碰撞半径（成对定义盒）、动画四元组
/// （`index` 指表项 / `frame` 表项内帧序号 / `delay` 帧内倒计时 / `reset` 回卷目标——
/// 四者互相约束，拆开就会"改 index 忘 reset"）。
pub mod ensx {
    /// `EnsExtra.status`。
    pub const STATUS: u16 = 0;
    /// `EnsExtra.radiusY`。
    pub const RADIUS_Y: u16 = 1;
    /// `EnsExtra.radiusX`。
    pub const RADIUS_X: u16 = 2;
    /// `EnsExtra.animFrame`。
    pub const ANIM_FRAME: u16 = 3;
    /// `EnsExtra.animDelay`。
    pub const ANIM_DELAY: u16 = 4;
    /// `EnsExtra.resetAnimIndex`。
    pub const RESET_ANIM_INDEX: u16 = 5;
    /// `EnsExtra.animIndex`。
    pub const ANIM_INDEX: u16 = 6;
    /// `EnsExtra.hi`（+7，`$6AFB`）——母脑/生成器的名表号。
    pub const HI: u16 = 7;
    /// typed 面的字节上界（0..=6 归 typed，7 起仍是字节面）。
    pub const TYPED_MAX: u16 = ANIM_INDEX;
}

/// 碰撞半径（`radiusY/radiusX`）——成对解释"这个判定盒有多大"，分开存就会有一半是旧值。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Radius {
    pub y: u8,
    pub x: u8,
}

/// 动画四元组（`animFrame/animDelay/resetAnimIndex/animIndex`）——见 [`ensx`] 注释。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Anim {
    pub frame: u8,
    pub delay: u8,
    pub reset: u8,
    pub index: u8,
}

/// 走表推进的一步结果（`Anim::step` 的返回值）——纯值，不含 I/O。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AnimStep {
    /// `animDelay` 递减后仍未归零：只更新 delay，帧不动（asm: `dec / bne @exit`）
    Tick(u8),
    /// delay 归零 ⇒ 重灌 `a` 并走表：`animFrame` 取表值、`animIndex` 自增
    Advance { delay: u8, frame: u8, index: u8 },
    /// 走表不变量破裂（`$FF` 连撞 16 次）——调用方 fail-loud，
    /// 带 `idx`(当前动画索引) / `reset`(回绕索引) / `len`(表长) 便于定位
    Broken { idx: u8, reset: u8, len: u8 },
}

impl Anim {
    /// `UpdateEnemyAnim ; 07:E094` 的**表段**（纯函数，无 I/O、无资产全局）。
    ///
    /// asm 形（`refs/m1disasm/SRC/areas_common.asm` 邻域）：
    /// `dec animDelay / bne exit` → `sta animDelay←A` → 取 `EnAnimTable[animIndex]`；
    /// 表值是 `$FF` ⇒ `animIndex ← resetAnimIndex` 再取一次（最多 16 次本模型护栏）。
    ///
    /// 表由调用方传入（按区 `assets::enemy_anim_table(area)`）：值对象不该知道"区"，
    /// 也不该在单测里拉起 gitignored 的资产包。
    pub(crate) fn step(self, table: &[u8], a: u8) -> AnimStep {
        if self.delay != 0 {
            let d = self.delay - 1;
            if d != 0 {
                return AnimStep::Tick(d);
            }
        }
        let mut idx = self.index;
        for _ in 0..16u32 {
            match table.get(idx as usize) {
                Some(&v) if v != 0xFF => {
                    return AnimStep::Advance { delay: a, frame: v, index: idx.wrapping_add(1) }
                }
                _ => idx = self.reset,
            }
        }
        // asm `@resetAnimIndex ; 07:E0BC` 是 `ldy resetAnimIndex,x / bcs @getAnimFrame` 的
        // **无条件回跳**：reset 处的字节若又是 FrameEnd，ROM 里就是死循环 —— 也就是说
        // 走到这里必然是我们的 `resetAnimIndex`/表基址与 ROM 不一致（不是"该回绕"）。
        // 带上 idx/reset/表长，便于在 trace 里直接指认是哪个敌槽哪张表。
        AnimStep::Broken { idx: self.index, reset: self.reset, len: table.len() as u8 }
    }
}

/// 一个敌槽的 typed 状态（`EnsExtra` 前 7 字节）。其余字节仍走 `NesStorage`。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct SlotState {
    pub status: EnemyStatus,
    pub radius: Radius,
    pub anim: Anim,
}

impl Default for EnemyStatus {
    fn default() -> Self {
        Self::NoEnemy
    }
}

/// 地址 → (槽号, 槽内偏移)：`$6AF4 + slot*16 + off`（`off` 必须 < 16）。
pub fn split_ens_extra(addr: u16) -> Option<(usize, u16)> {
    const BASE: u16 = 0x6AF4;
    let end = BASE + SLOT_COUNT as u16 * 16;
    if !(BASE..end).contains(&addr) {
        return None;
    }
    let rel = addr - BASE;
    Some(((rel / 16) as usize, rel % 16))
}

impl SlotState {
    /// typed 字节读（`off` 是 `$6AF4 + slot*16` 之后的偏移）；`None` ⇒ 该字节未 typed 化。
    pub fn get_byte(&self, off: u16) -> Option<u8> {
        Some(match off {
            ensx::STATUS => self.status.to_byte(),
            ensx::RADIUS_Y => self.radius.y,
            ensx::RADIUS_X => self.radius.x,
            ensx::ANIM_FRAME => self.anim.frame,
            ensx::ANIM_DELAY => self.anim.delay,
            ensx::RESET_ANIM_INDEX => self.anim.reset,
            ensx::ANIM_INDEX => self.anim.index,
            _ => return None,
        })
    }

    pub fn set_byte(&mut self, off: u16, v: u8) -> bool {
        match off {
            ensx::STATUS => self.status = EnemyStatus::from_byte(v),
            ensx::RADIUS_Y => self.radius.y = v,
            ensx::RADIUS_X => self.radius.x = v,
            ensx::ANIM_FRAME => self.anim.frame = v,
            ensx::ANIM_DELAY => self.anim.delay = v,
            ensx::RESET_ANIM_INDEX => self.anim.reset = v,
            ensx::ANIM_INDEX => self.anim.index = v,
            _ => return false,
        }
        true
    }
}

/// `Ens` 主面基址（`$0400`，16 字节/槽）——句柄内部的唯一定义处。
pub(crate) const ENS_BASE: u16 = 0x0400;

/// `EnsExtra` 基址（`$6AF4`，16 字节/槽）——句柄内部的唯一定义处。
pub(crate) const ENS_EXTRA_BASE: u16 = 0x6AF4;

/// 敌槽步长（16 字节/槽；爆炸池是 8，两者不可混用）。
pub(crate) const ENEMY_STRIDE: u16 = 16;

/// 敌槽字节偏移 → 槽号。asm 例程普遍以 `x = slot*16` 作索引，句柄化时在此换算一次。
/// 【2026-11-17 上移】原在 `enemy/framework.rs`，因 `enemy/anim.rs` 也要用而收进本模块。
#[inline]
pub(crate) fn slot_of(x: u8) -> u8 {
    debug_assert_eq!(x % 16, 0, "敌槽偏移必须是 16 的倍数，收到 {x:#04X}");
    x / 16
}

/// **「`Ens` 基址 + asm 偏移」面**：共用例程（`UpdateEnemyAnim ; 07:E094` /
/// `DrawEnemy ; 07:DD8B`）的入参 `x` 不是槽号，而是「`Ens` 基址 + 偏移」索引。
///
/// 敌池 `x` 步长 16（`0..=0xAF`），爆炸池步长 8（`0xC0..=0xD8`），两池靠
/// `$6AF4 + $C0 == $6BB4` 恰好接上——所以**不能用 `slot_of(x)` 换算**（槽 0 之外会整条错位，
/// 2026-11-17 实测被池测抓出）。本句柄的职责就是把这条偏移算术关在一处，
/// 让共用例程不必各自写 `$6AF4 + x + 3` 这类字面量。
pub(crate) struct EnsFace<'a> {
    mem: &'a mut Manager,
    x: u8,
}

impl Manager {
    /// 派生共用例程的字段面（`x` = asm 的「`Ens` 基址 + 偏移」，两池通用）。
    pub(crate) fn ens_face(&mut self, x: u8) -> EnsFace<'_> {
        EnsFace { mem: self, x }
    }
}

impl<'a> EnsFace<'a> {
    #[inline]
    fn extra(&self, off: u16) -> u16 {
        ENS_EXTRA_BASE + self.x as u16 + off
    }

    #[inline]
    fn main(&self, off: u16) -> u16 {
        ENS_BASE + self.x as u16 + off
    }

    /// `EnsExtra.animFrame`（+3）。
    pub(crate) fn anim_frame(&self) -> u8 {
        self.mem.read_wram(self.extra(ensx::ANIM_FRAME as u16))
    }

    /// 字节面写口（`Ens` 主面）——本文件唯一写入口，避免每加一个字段多一处裸面。
    #[inline]
    fn putm(&mut self, off: u16, v: u8) {
        self.mem.write_ram(self.main(off), v);
    }
    /// 字节面写口（`EnsExtra` 面）。
    #[inline]
    fn putx(&mut self, off: u16, v: u8) {
        self.mem.write_wram(self.extra(off), v);
    }
    /// 字节面读口（`EnsExtra` 面）。
    #[inline]
    fn getx_(&self, off: u16) -> u8 {
        self.mem.read_wram(self.extra(off))
    }

    /// 帧记录携带的半径（`asm :6156-6168` 帧记字 1/2 → `EnsExtra.radiusY/X`）。
    pub(crate) fn set_radius_y(&mut self, v: u8) {
        self.putx(ensx::RADIUS_Y as u16, v);
    }
    pub(crate) fn set_radius_x(&mut self, v: u8) {
        self.putx(ensx::RADIUS_X as u16, v);
    }

    /// `Ens.data05`（+5）——`bit1` 是 `IsObjectVisible` 的逐帧闩。
    pub(crate) fn data05(&self) -> u8 {
        self.mem.read_ram(self.main(5))
    }
    pub(crate) fn set_data05(&mut self, v: u8) {
        self.putm(5, v);
    }

    /// `Ens.y`（+0）/ `Ens.x`（+1）——房内坐标（非屏位）。
    pub(crate) fn y(&self) -> u8 {
        self.mem.read_ram(self.main(0))
    }
    pub(crate) fn x_pos(&self) -> u8 {
        self.mem.read_ram(self.main(1))
    }
    pub(crate) fn set_y(&mut self, v: u8) {
        self.putm(0, v);
    }
    pub(crate) fn set_pos_x(&mut self, v: u8) {
        self.putm(1, v);
    }
    /// `EnsExtra.hi`（+7）——母脑/生成器把名表号写这里。
    pub(crate) fn set_hi(&mut self, v: u8) {
        self.putx(ensx::HI as u16, v);
    }
    /// `EnsExtra.animFrame`（+3）写侧。
    pub(crate) fn set_anim_frame(&mut self, v: u8) {
        self.putx(ensx::ANIM_FRAME as u16, v);
    }
    /// `EnsExtra.status`（+0）读侧（字节面，含 `Undefined` 保真）。
    pub(crate) fn status(&self) -> EnemyStatus {
        EnemyStatus::from_byte(self.getx_(ensx::STATUS as u16))
    }
    /// `EnsExtra.status`（+0）写侧（字节自增以外的整体赋值，如母脑清场）。
    pub(crate) fn set_status(&mut self, v: EnemyStatus) {
        self.putx(ensx::STATUS as u16, v.to_byte());
    }
}

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
    /// `$6AFE EnsExtra.accelY`（i8）。
    pub fn accel_y(&self) -> i8 {
        self.getx(ens_extra::ACCEL_Y) as i8
    }

    pub fn set_accel_y(&mut self, v: i8) {
        self.setx(ens_extra::ACCEL_Y, v as u8);
    }

    /// `$6AFF EnsExtra.accelX`（i8）。
    pub fn accel_x(&self) -> i8 {
        self.getx(ens_extra::ACCEL_X) as i8
    }

    pub fn set_accel_x(&mut self, v: i8) {
        self.setx(ens_extra::ACCEL_X, v as u8);
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
    pub fn set_is_hit(&mut self, v: u8) {
        self.set(ens::IS_HIT, v);
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
    /// 写 `Ens.prevStatus`（`$040C`）——`KraidUpdateProjectile ; 04:9B3C` 的「冻前状态」
    /// 判据读它；测试需要构造冻前态，故与读侧配对给出写侧。
    pub fn set_prev_status(&mut self, v: u8) {
        self.set(ens::PREV_STATUS, v);
    }
    /// `Ens.speedSubPixelX`（`$0407`）——横速小数位。
    pub fn speed_subpixel_x(&self) -> u8 {
        self.get(ens::SPEED_SUBPIXEL_X)
    }
    pub fn set_speed_subpixel_x(&mut self, v: u8) {
        self.set(ens::SPEED_SUBPIXEL_X, v);
    }
    /// `Ens.speedSubPixelY`（`$0406`）——竖速小数位。
    ///
    /// **同址三名之一**（见 [`ens::DATA06_ALIASES`]）：`chunkyExplosionTimer` /
    /// `movementInstrIndex` / `speedSubPixelY`。`RinkaAIRoutine ; 03:9A2C` 与
    /// `ClearRinkaAcceleration` 走的是这个语义（8.4 定点的竖速小数）。
    pub fn speed_subpixel_y(&self) -> u8 {
        self.get(ens::DATA06)
    }
    pub fn set_speed_subpixel_y(&mut self, v: u8) {
        self.set(ens::DATA06, v);
    }
    /// `Ens.data0A`（`$040A`）——无独立语义名（`_todo_`），按 asm 用途保留裸字段。
    pub fn data0a(&self) -> u8 {
        self.get(ens::DATA0A)
    }
    pub fn set_data0a(&mut self, v: u8) {
        self.set(ens::DATA0A, v);
    }
    /// 拾取物剩余寿命字节（`$040D`）——`UpdateEnemy_Pickup ; 07:F483` 每 4 帧递减。
    pub fn data0d(&self) -> u8 {
        self.get(ens::DATA0D)
    }

    pub fn set_data0d(&mut self, v: u8) {
        self.set(ens::DATA0D, v);
    }
    /// 击杀武器（`$040E`）——`<= $01` 即普通束（见 `ExplodeEnemy ; 07:F611`）。
    /// 写 `Ens.weaponAction`（`$040E`）——测试与 `ExplodeEnemy` 前置都要构造击杀武器类型。
    pub fn set_weapon_action(&mut self, v: u8) {
        self.set(ens::WEAPON_ACTION, v);
    }
    pub fn weapon_action(&self) -> u8 {
        self.get(ens::WEAPON_ACTION)
    }
    /// 特殊属性（`$040F`）——bit6 = miniboss。
    pub fn special_attribs(&self) -> u8 {
        self.get(ens::SPECIAL_ATTRIBS)
    }
    /// 写 `Ens.specialAttribs`（`$040F`）——`KraidUpdateProjectile@remove` 清它；
    /// health 计算按 bit7 决定是否翻倍（见 `InitEnemyForceSpeedTowardsSamusDelayAndHealth`）。
    pub fn set_special_attribs(&mut self, v: u8) {
        self.set(ens::SPECIAL_ATTRIBS, v);
    }
    /// 跳跃位移（`$6B01`）。
    pub fn jump_dsplcmnt(&self) -> u8 {
        self.getx(ens_extra::JUMP_DSPLCMNT)
    }
    pub fn set_jump_dsplcmnt(&mut self, v: u8) {
        self.setx(ens_extra::JUMP_DSPLCMNT, v);
    }

    /// `EnemyFlipAfterDisplacement` 的符号判定（`07:FB8F-07:FB99`）：
    /// `data1F != 0` 时 N 旗来自 `data1F` bit7（`inc`/`dec` 只为置 Z/N，值不变），
    /// 只有 `data1F == 0` 时才回落到 `jumpDsplcmnt` 自身 bit7。
    ///
    /// 业务层只调本方法，字节掩码关在 typed 面内（`raw-surface` 只减不增）。
    pub fn flip_displacement_is_negative(&self, jump_dsplcmnt: u8) -> bool {
        let d1f = self.getx(ens_extra::DATA1F);
        if d1f != 0 {
            d1f & 0x80 != 0
        } else {
            jump_dsplcmnt & 0x80 != 0
        }
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
    /// 动画状态整组写（`Anim` 四元组是一个不变量：`index` 指表项、`frame` 表项内序号、
    /// `delay` 帧内倒计时、`reset` 回卷目标）。**逐字节写会允许"改一个忘另一个"。**
    pub fn set_anim(&mut self, a: Anim) {
        self.setx(ensx::ANIM_FRAME, a.frame);
        self.setx(ensx::ANIM_DELAY, a.delay);
        self.setx(ensx::RESET_ANIM_INDEX, a.reset);
        self.setx(ensx::ANIM_INDEX, a.index);
    }

    pub fn set_anim_frame(&mut self, v: u8) {
        self.setx(ensx::ANIM_FRAME, v);
    }
    pub fn anim_delay(&self) -> u8 {
        self.getx(ensx::ANIM_DELAY)
    }
    pub fn set_anim_delay(&mut self, v: u8) {
        self.setx(ensx::ANIM_DELAY, v);
    }
    pub fn reset_anim_index(&self) -> u8 {
        self.getx(ens_extra::RESET_ANIM_INDEX)
    }
    pub fn set_reset_anim_index(&mut self, v: u8) {
        self.setx(ensx::RESET_ANIM_INDEX, v);
    }
    pub fn set_anim_index(&mut self, v: u8) {
        self.setx(ensx::ANIM_INDEX, v);
    }
    pub fn set_radius_y(&mut self, v: u8) {
        self.setx(ensx::RADIUS_Y, v);
    }
    pub fn set_radius_x(&mut self, v: u8) {
        self.setx(ensx::RADIUS_X, v);
    }

    pub fn anim_index(&self) -> u8 {
        self.getx(ens_extra::ANIM_INDEX)
    }
    /// 动画四元组整体读出（与 [`Self::set_anim`] 配对，保证读写同源）。
    pub(crate) fn anim(&self) -> Anim {
        Anim {
            frame: self.anim_frame(),
            delay: self.anim_delay(),
            reset: self.reset_anim_index(),
            index: self.anim_index(),
        }
    }

    /// `UpdateEnemyAnim ; 07:E094` 的**实体侧**：Pickup 态不更（asm 首件 `cmp #$05 / beq`）
    /// + 调 [`Anim::step`] + 写回本槽。返回是否推进了帧（供调用方判断要不要重绘）。
    pub(crate) fn advance_anim(&mut self, table: &[u8], a: u8) -> bool {
        if self.status_raw() == 5 {
            return false; // Pickup 态不更
        }
        match self.anim().step(table, a) {
            AnimStep::Tick(d) => {
                self.set_anim_delay(d);
                false
            }
            AnimStep::Advance { delay, frame, index } => {
                self.set_anim(Anim { frame, delay, reset: self.reset_anim_index(), index });
                true
            }
            AnimStep::Broken { idx, reset, len } => {
                {
                    // 带槽标识：同一 `idx/reset` 在多个槽/池里都可能出现，没有 x/type 就无法指认。
                    crate::gap::record("bank07:E0BC-wrap");
                    panic!(
                        "unported @ bank07:E0BC-wrap idx={idx:02X} reset={reset:02X} len={len} \
                         x={:02X} type={:02X} status={:02X}",
                        self.x(),
                        self.getx(ens_extra::TYPE),
                        self.status_raw()
                    );
                }
            }
        }
    }

    /// `$6AFC EnsExtra.subPixelY`（竖速小数）。
    pub fn subpixel_y(&self) -> u8 {
        self.getx(ens_extra::SUB_PIXEL_Y)
    }
    pub fn set_subpixel_y(&mut self, v: u8) {
        self.setx(ens_extra::SUB_PIXEL_Y, v);
    }
    /// `$6AFD EnsExtra.subPixelX`（横速小数）。
    pub fn subpixel_x(&self) -> u8 {
        self.getx(ens_extra::SUB_PIXEL_X)
    }
    pub fn set_subpixel_x(&mut self, v: u8) {
        self.setx(ens_extra::SUB_PIXEL_X, v);
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
mod tests_typed_slot {
    use super::{ensx, Anim, EnemyStatus};
    use crate::frame::Manager;

    fn m() -> Manager {
        Manager::default()
    }

    /// `$6AF4 + slot*16 + off`（off 0..=6）是 typed 辖：地址面、视图面、投影面三面同源。
    #[test]
    fn ens_extra_typed_bytes_are_single_source() {
        let mut mem = m();
        let base = 0x6AF4u16 + 3 * 16;
        mem.write_wram(base + ensx::STATUS, 2); // Active
        mem.write_wram(base + ensx::RADIUS_Y, 12);
        mem.write_wram(base + ensx::ANIM_INDEX, 0x42);
        assert_eq!(mem.enemy(3).status(), EnemyStatus::Active, "视图面");
        assert_eq!(mem.read_wram(base + ensx::RADIUS_Y), 12, "地址面");
        let img = mem.snapshot_nes();
        assert_eq!(img.wram[(base + ensx::ANIM_INDEX - 0x6000) as usize], 0x42, "投影面");
        // 视图写回地址面
        mem.enemy(3).set_status(EnemyStatus::Pickup);
        assert_eq!(mem.read_wram(base + ensx::STATUS), 5);
    }

    /// `set_anim` 是四元组整体写：任何一处都读得回，且不会只改一半。
    #[test]
    fn set_anim_writes_all_four_fields() {
        let mut mem = m();
        mem.enemy(7).set_anim(Anim { frame: 0x21, delay: 0x0C, reset: 0x30, index: 0x31 });
        let e = mem.enemy(7);
        assert_eq!((e.anim_frame(), e.anim_delay(), e.anim_index()), (0x21, 0x0C, 0x31));
        let base = 0x6AF4u16 + 7 * 16;
        assert_eq!(mem.read_wram(base + ensx::RESET_ANIM_INDEX), 0x30, "reset 也落盘");
        assert_eq!(mem.read_wram(base + ensx::ANIM_INDEX), 0x31, "index 落盘");
    }

    /// 未 typed 的字节（如 `hi` = +7）仍直落字节面，不被 typed 层吞掉。
    #[test]
    fn untyped_bytes_fall_through() {
        let mut mem = m();
        let a = 0x6AF4u16 + 2 * 16 + 7;
        mem.write_wram(a, 0x9D);
        assert_eq!(mem.read_wram(a), 0x9D);
    }

    /// MSS 恢复必须把敌 typed 面一并回灌（同族事故：整面 dump 曾漏 typed）。
    #[test]
    fn restore_mss_backfills_enemy_typed() {
        let ram = vec![0u8; 0x800];
        let mut wram = vec![0u8; 0x2000];
        let base = 0x6AF4u16 + 5 * 16;
        wram[(base - 0x6000) as usize + ensx::STATUS as usize] = 4; // Frozen
        wram[(base - 0x6000) as usize + ensx::ANIM_FRAME as usize] = 0x11;
        let mut mem = Manager::restore_mss(&ram, &wram, Default::default());
        assert_eq!(mem.enemy(5).status(), EnemyStatus::Frozen);
        assert_eq!(mem.enemy(5).anim_frame(), 0x11);
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


#[cfg(test)]
mod tests_anim_step {
    use super::*;

    /// 表：`0x10` 是正常项、`0x11` 是 `$FF`（走回卷）。
    const TBL: [u8; 18] = [
        0x10, 0xFF, 0x10, 0x10, 0xFF, 0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x10,
        0x10, 0x10, 0x99,
    ];

    /// delay 未归零 ⇒ 只递减，帧不动（asm `dec / bne`）。
    #[test]
    fn tick_only_decrements_delay() {
        let a = Anim { frame: 0x33, delay: 3, reset: 0, index: 1 };
        assert_eq!(a.step(&TBL, 5), AnimStep::Tick(2));
    }

    /// delay 归零 ⇒ 重灌 `a` 并取 `table[index]`，index 自增。
    #[test]
    fn advance_reads_table_and_bumps_index() {
        let a = Anim { frame: 0x33, delay: 1, reset: 0, index: 2 };
        assert_eq!(a.step(&TBL, 5), AnimStep::Advance { delay: 5, frame: 0x10, index: 3 });
    }

    /// `$FF` ⇒ 回卷到 `reset` 再取（asm `animIndex ← resetAnimIndex`）。
    #[test]
    fn ff_entry_wraps_to_reset() {
        let a = Anim { frame: 0x00, delay: 1, reset: 2, index: 1 };
        assert_eq!(a.step(&TBL, 5), AnimStep::Advance { delay: 5, frame: 0x10, index: 3 });
    }

    /// reset 本身也指向 `$FF` ⇒ 16 次后判不变量破裂（而不是无限循环或静默取 $FF）。
    #[test]
    fn broken_when_reset_also_points_at_ff() {
        let a = Anim { frame: 0x00, delay: 1, reset: 1, index: 1 };
        assert!(matches!(a.step(&TBL, 5), AnimStep::Broken { .. }));
    }

    /// 越界索引等同 `$FF`（不 panic、按回卷处理）——与 `AnimIdx::resolve` 的 None 同义。
    #[test]
    fn out_of_range_index_wraps_like_ff() {
        let a = Anim { frame: 0x00, delay: 1, reset: 0, index: 200 };
        assert_eq!(a.step(&TBL, 5), AnimStep::Advance { delay: 5, frame: 0x10, index: 1 });
    }
}

// ─────────────────────────── P2：`NesMapped` 声明（域级） ───────────────────────────

// 额外面尾 9 B 已由 `crate::memmap::enemy_extra_face::EnemyExtraFace` 以 typed `Owner`
// 声明（卡 #113），本域不再重复登记那 108 B；见 `docs/spec/b2b-extra-6afb.md`。

/// `EnsExtra` 第 `slot` 槽的槽首地址（`$6AF4 + slot*16`）——只从既有常量推导。
const fn ensx_slot(slot: u16) -> u16 {
    ENS_EXTRA_BASE + slot * ENEMY_STRIDE
}

/// 敌域两面的 `NesMapped` 声明（**域级**：12 槽全脸；句柄级 `project` 只写本槽 7 B）。
///
/// - 主面 `Ens`（`$0400-$04BF`，12 槽 × 16 B）：**未 typed**——字段真相仍在字节面
///   （`EnsFace` 只是读视图），整脸登记为 `Unmodeled`（typed 债 192 B）。
/// - 额外面 `EnsExtra`（`$6AF4-$6BB3`）：每槽前 7 B（`status..animIndex`）已是 typed
///   （`SlotState` = 真相面）⇒ `Owner`；尾 9 B 未 typed ⇒ `Unmodeled`（债 108 B）。
///
/// 判别字段 `status` 的地址随槽变（`$6AF4 + slot*16`），故 `DISCRIMINATOR = None`；
/// 判别感知腿（P3）按槽算地址，不在这里伪装成单值。
impl<'a> crate::memmap::nesmap::NesMapped for Enemy<'a> {
    const DISCRIMINATOR: Option<u16> = None;

    const SPANS: &'static [(NesSpan, SpanKey)] = &[
        // 主面整脸（12 槽 × 16 B，`$0400-$04BF`）：B1-c 起改由
        // `crate::memmap::object_slot_face::ObjectSlotFace` 以 typed `Owner` 声明，
        // 本域不再重复登记（同一批字节双声明会破坏四态恒等式）。
        // 额外面：逐槽 typed 7 B（槽头）；尾 9 B 由
        // `crate::memmap::enemy_extra_face::EnemyExtraFace` 认领，本域不再登记。
     (NesSpan::value(ensx_slot(0), ensx::TYPED_MAX + 1), SpanKey::Owner),
     (NesSpan::value(ensx_slot(1), ensx::TYPED_MAX + 1), SpanKey::Owner),
     (NesSpan::value(ensx_slot(2), ensx::TYPED_MAX + 1), SpanKey::Owner),
     (NesSpan::value(ensx_slot(3), ensx::TYPED_MAX + 1), SpanKey::Owner),
     (NesSpan::value(ensx_slot(4), ensx::TYPED_MAX + 1), SpanKey::Owner),
     (NesSpan::value(ensx_slot(5), ensx::TYPED_MAX + 1), SpanKey::Owner),
     (NesSpan::value(ensx_slot(6), ensx::TYPED_MAX + 1), SpanKey::Owner),
     (NesSpan::value(ensx_slot(7), ensx::TYPED_MAX + 1), SpanKey::Owner),
     (NesSpan::value(ensx_slot(8), ensx::TYPED_MAX + 1), SpanKey::Owner),
     (NesSpan::value(ensx_slot(9), ensx::TYPED_MAX + 1), SpanKey::Owner),
     (NesSpan::value(ensx_slot(10), ensx::TYPED_MAX + 1), SpanKey::Owner),
     (NesSpan::value(ensx_slot(11), ensx::TYPED_MAX + 1), SpanKey::Owner),
    ];

    /// 只写**本槽** `EnsExtra` 的 7 个 typed 字节（`getx` = 该域自己的读径）。
    fn project(&self, out: &mut crate::memmap::storage::NesImage) {
        let base = ensx_slot(u16::from(self.slot));
        for off in 0..=ensx::TYPED_MAX {
            let a = base + off;
            out.wram[usize::from(a - crate::memmap::nesmap::WRAM_BASE)] = self.getx(off);
        }
    }
}
