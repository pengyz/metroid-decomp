//! Samus 状态语义视图——`$0300-$0315` 的字段面（地址只活在本文件）。
//!
//! 地址证据（A 级）：`refs/m1disasm/SRC/constants_ram.asm:218-238` + `:860-878`
//! （`Samus` 结构体 22 字节：status/radiusY/radiusX/animFrame/animDelay/animResetIndex/
//! animIndex/SamusOnElevator/speedY/speedX/isHit/onScreen/hi/y/x/SamusJumpDsplcmnt/
//! SamusSubPixelY/SamusSubPixelX/SamusSpeedSubPixelY/SamusSpeedSubPixelX/SamusAccelY/
//! SamusAccelX）；`Samus.status` 取值=`constants.asm:112` 的 `sa_*` 十态。
//!
//! 形态纪律（`views/mod.rs` 三条细则）：
//! - `status` 出 [`SamusStatus`] 枚举而不是裸 `u8`——调用方写 `SamusStatus::Roll` 而不是 `3`；
//! - 速度/加速出 `i8`（asm 侧本就是二补数，`lda/lsr/ror` 的算术在视图里不外泄）；
//! - `health` 跨 `$0106/$0107` 两个字节，视图出 `u16` 并封装「低字节在 $0106」的规则。

use crate::ram_map::{
    SAMUS_ACCEL_X, SAMUS_ACCEL_Y, SAMUS_ANIM_FRAME, SAMUS_HI, SAMUS_IS_HIT, SAMUS_JUMP_DISP,
    SAMUS_ON_ELEVATOR, SAMUS_RADIUS_X, SAMUS_RADIUS_Y, SAMUS_SPEED_SUBPIXEL_X,
    SAMUS_SPEED_SUBPIXEL_Y, SAMUS_SPEED_X_INT, SAMUS_SPEED_Y_INT, SAMUS_STATUS, SAMUS_SUBPIXEL_X,
    SAMUS_SUBPIXEL_Y, SAMUS_X, SAMUS_Y,
};

// 下列字段 `ram_map` 未立名（历史只给被单独引用的字节起名）——地址归属本层，
// 在此就地定义；`ram_map` 侧的重复名（`SAMUS_JUMP_DISP` / `SAMUS_JUMP_DISP`
// 同指 `$030F`）留待 `consts.rs` 统一时合并。
/// `Samus.animDelay`（`constants_ram.asm:222`）。
const ANIM_DELAY: u16 = 0x0304;
/// `Samus.animResetIndex`。
const ANIM_RESET_INDEX: u16 = 0x0305;
/// `Samus.animIndex`。
const ANIM_INDEX: u16 = 0x0306;
/// `Samus.onScreen`。
const ON_SCREEN: u16 = 0x030B;
/// `Health` 低字节（`constants_ram.asm:806`）。
const HEALTH_LO: u16 = 0x0106;
/// `Health` 高字节。
const HEALTH_HI: u16 = 0x0107;

/// `Samus.status`（`constants.asm:112` 的 `sa_*` 十态）。
///
/// 【旧注纠正】`sa_Elevator` 不是表序 4 而是 **9**（表序 4 是 `sa_PntUp`）；旧实现按 4
/// 判定会让电梯态落进 `status >= sa_Dead(7)` 的死人出径。
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SamusStatus {
    Stand = 0,
    Run = 1,
    Jump = 2,
    Roll = 3,
    PointUp = 4,
    Door = 5,
    JumpUp = 6,
    Dead = 7,
    Dead2 = 8,
    Elevator = 9,
    /// 未定义字节（ROM 行为空间外）。**保真保留原值**：`from_byte/to_byte` 往返不丢失，
    /// 存储层不可能产生它，但脚本注入/未初始化态可能。
    Undefined(u8),
}

impl SamusStatus {
    /// 字节 → 枚举。未定义值不在 ROM 行为空间内（十态遍历完整），返回 `None` 供调用方
    /// fail-loud，不静默映射成 `Stand`。
    pub fn from_u8(v: u8) -> Option<Self> {
        Some(match v {
            0 => Self::Stand,
            1 => Self::Run,
            2 => Self::Jump,
            3 => Self::Roll,
            4 => Self::PointUp,
            5 => Self::Door,
            6 => Self::JumpUp,
            7 => Self::Dead,
            8 => Self::Dead2,
            9 => Self::Elevator,
            _ => return None,
        })
    }

    /// 字节 → 枚举（不失败版：未定义值进 [`Self::Undefined`]，往返保真——存储层用）。
    pub fn from_byte(v: u8) -> Self {
        Self::from_u8(v).unwrap_or(Self::Undefined(v))
    }

    /// 枚举 → 字节（存储层投影用）。
    pub fn to_byte(self) -> u8 {
        match self {
            Self::Stand => 0,
            Self::Run => 1,
            Self::Jump => 2,
            Self::Roll => 3,
            Self::PointUp => 4,
            Self::Door => 5,
            Self::JumpUp => 6,
            Self::Dead => 7,
            Self::Dead2 => 8,
            Self::Elevator => 9,
            Self::Undefined(v) => v,
        }
    }

    pub fn as_u8(self) -> u8 {
        self.to_byte()
    }

    /// 球形态（`cmp #sa_Roll` 的语义面——落地反弹/变形判定用）。
    pub fn is_roll(self) -> bool {
        matches!(self, Self::Roll)
    }

    /// 死亡族（`status >= sa_Dead`——`SamusMoveVertically` 等件的短路判据）。
    pub fn is_dead(self) -> bool {
        matches!(self, Self::Dead | Self::Dead2)
    }
}

/// **Samus 域的 typed 状态**（存储层：`Manager.samus`）。
///
/// 【第一批】只收 `status`（`$0300`）：它是本域语义密度最高的字段（十态枚举），且是
/// 「typed 存储 → NES 字节投影」这套机制的试点。其余 21 字节仍在 `NesStorage.ram` 里，
/// 后续按字段逐个迁入本结构——每迁一个，`Manager::snapshot_nes()` 的投影就多一行，
/// 而 `read_ram/write_ram` 的地址面保持不变（兼容辖）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SamusState {
    status: SamusStatus,
    /// `Samus.speedY`（`$0308`）——asm 侧为二补数，typed 面出 `i8`（`lsr/ror` 半速反弹等
    /// 算术不再以裸字节外泄）。
    speed_y: i8,
    /// `Samus.speedX`（`$0309`）。
    speed_x: i8,
    /// `SamusAccelY`（`$0314`）。
    accel_y: i8,
    /// `SamusAccelX`（`$0315`）。
    accel_x: i8,
}

impl Default for SamusState {
    fn default() -> Self {
        Self {
            status: SamusStatus::Stand,
            speed_y: 0,
            speed_x: 0,
            accel_y: 0,
            accel_x: 0,
        }
    }
}

impl SamusState {
    /// 枚举面（未定义值 → `None`，调用方 fail-loud）。
    pub fn status(&self) -> Option<SamusStatus> {
        match self.status {
            SamusStatus::Undefined(_) => None,
            s => Some(s),
        }
    }

    /// 字节面（保真：含未定义值）。
    pub fn status_byte(&self) -> u8 {
        self.status.to_byte()
    }

    pub fn set_status(&mut self, s: SamusStatus) {
        self.status = s;
    }

    pub fn set_status_byte(&mut self, v: u8) {
        self.status = SamusStatus::from_byte(v);
    }

    /// typed 域的字节读写面（地址 → 值）。`None` = 该地址尚未 typed 化，走存储层。
    ///
    /// 这是 `read_ram/write_ram` 与 `Samus` 视图**共用的唯一映射表**——加新字段只改这里
    /// 与 `snapshot_nes()` 各一行，避免"两份地址映射"漂移。
    pub fn get_byte(&self, addr: u16) -> Option<u8> {
        Some(match addr {
            SAMUS_STATUS => self.status.to_byte(),
            SAMUS_SPEED_Y_INT => self.speed_y as u8,
            SAMUS_SPEED_X_INT => self.speed_x as u8,
            SAMUS_ACCEL_Y => self.accel_y as u8,
            SAMUS_ACCEL_X => self.accel_x as u8,
            _ => return None,
        })
    }

    pub fn set_byte(&mut self, addr: u16, v: u8) -> bool {
        match addr {
            SAMUS_STATUS => self.status = SamusStatus::from_byte(v),
            SAMUS_SPEED_Y_INT => self.speed_y = v as i8,
            SAMUS_SPEED_X_INT => self.speed_x = v as i8,
            SAMUS_ACCEL_Y => self.accel_y = v as i8,
            SAMUS_ACCEL_X => self.accel_x = v as i8,
            _ => return false,
        }
        true
    }

    /// 迁入 typed 的地址清单（`snapshot_nes()` 逐项投影用——**唯一投影源**）。
    pub const TYPED_ADDRS: [u16; 5] = [
        SAMUS_STATUS,
        SAMUS_SPEED_Y_INT,
        SAMUS_SPEED_X_INT,
        SAMUS_ACCEL_Y,
        SAMUS_ACCEL_X,
    ];
}

/// **Samus 主体**：既是状态句柄，也是行为载体（领域层只认这个类型）。
///
/// 包装 `&mut Manager` 而不是只包装 `&mut ram`：Samus 的行为会跨域取值（碰撞要查房间
/// RAM、射击要查敌槽、能力位在 WRAM `$6878`），只借 ram 会逼调用方拆借。
///
/// 与「全局单例 + `get::<Samus>()`」的区别（设计裁定）：本类型是**短生命周期句柄**，
/// 由唯一所有者 `Manager` 现场派生——`mem.samus()`。这样：
/// - 调用点写法与类实例一致（`mem.samus().move_vertically()`）；
/// - 无 `unsafe`、无 `RefCell`、无运行期类型分发（`get::<T>()` 需要 `Any`+downcast，
///   在静态语言里等于把类型检查推迟到运行期）；
/// - 借用检查器保证「同一时刻只有一个域持有可变访问」，与 asm 的时序天然同构；
/// - 测试可并行（每个测试各持一份 `Manager`；全局单例会毁掉这一点）。
pub struct Samus<'a> {
    mem: &'a mut crate::frame::Manager,
}

impl<'a> Samus<'a> {
    pub(crate) fn new(mem: &'a mut crate::frame::Manager) -> Self {
        Self { mem }
    }

    #[inline]
    fn get(&self, addr: u16) -> u8 {
        // typed 域辖字段走 `Manager.samus`；其余仍在字节面。
        if let Some(v) = self.mem.samus.get_byte(addr) {
            return v;
        }
        self.mem.ram()[(addr as usize) & 0x7FF]
    }

    #[inline]
    fn set(&mut self, addr: u16, v: u8) {
        if self.mem.samus.set_byte(addr, v) {
            return;
        }
        self.mem.ram_mut()[(addr as usize) & 0x7FF] = v;
    }

    pub fn status(&self) -> Option<SamusStatus> {
        self.mem.samus.status()
    }

    /// 原始字节（比较 `>= sa_Dead` 这类**数值序**判据时用；语义判据请用 [`Self::status`]）。
    pub fn status_raw(&self) -> u8 {
        self.get(SAMUS_STATUS)
    }

    pub fn set_status(&mut self, v: SamusStatus) {
        self.mem.samus.set_status(v);
    }

    pub fn radius_y(&self) -> u8 {
        self.get(SAMUS_RADIUS_Y)
    }
    pub fn set_radius_y(&mut self, v: u8) {
        self.set(SAMUS_RADIUS_Y, v);
    }
    pub fn radius_x(&self) -> u8 {
        self.get(SAMUS_RADIUS_X)
    }
    pub fn set_radius_x(&mut self, v: u8) {
        self.set(SAMUS_RADIUS_X, v);
    }

    pub fn anim_frame(&self) -> u8 {
        self.get(SAMUS_ANIM_FRAME)
    }
    pub fn set_anim_frame(&mut self, v: u8) {
        self.set(SAMUS_ANIM_FRAME, v);
    }
    pub fn anim_delay(&self) -> u8 {
        self.get(ANIM_DELAY)
    }
    pub fn set_anim_delay(&mut self, v: u8) {
        self.set(ANIM_DELAY, v);
    }
    pub fn anim_reset_index(&self) -> u8 {
        self.get(ANIM_RESET_INDEX)
    }
    pub fn anim_index(&self) -> u8 {
        self.get(ANIM_INDEX)
    }
    pub fn set_anim_index(&mut self, v: u8) {
        self.set(ANIM_INDEX, v);
    }

    pub fn on_elevator(&self) -> bool {
        self.get(SAMUS_ON_ELEVATOR) != 0
    }
    pub fn set_on_elevator(&mut self, v: bool) {
        self.set(SAMUS_ON_ELEVATOR, v as u8);
    }

    /// `speedY`（`$0308`）——asm 侧二补数，视图出 `i8`。
    pub fn speed_y(&self) -> i8 {
        self.get(SAMUS_SPEED_Y_INT) as i8
    }
    pub fn set_speed_y(&mut self, v: i8) {
        self.set(SAMUS_SPEED_Y_INT, v as u8);
    }
    pub fn speed_x(&self) -> i8 {
        self.get(crate::ram_map::SAMUS_SPEED_X_INT) as i8
    }
    pub fn set_speed_x(&mut self, v: i8) {
        self.set(crate::ram_map::SAMUS_SPEED_X_INT, v as u8);
    }

    pub fn is_hit(&self) -> u8 {
        self.get(SAMUS_IS_HIT)
    }
    pub fn set_is_hit(&mut self, v: u8) {
        self.set(SAMUS_IS_HIT, v);
    }
    pub fn on_screen(&self) -> u8 {
        self.get(ON_SCREEN)
    }

    pub fn hi(&self) -> u8 {
        self.get(SAMUS_HI)
    }
    pub fn set_hi(&mut self, v: u8) {
        self.set(SAMUS_HI, v);
    }
    pub fn y(&self) -> u8 {
        self.get(SAMUS_Y)
    }
    pub fn set_y(&mut self, v: u8) {
        self.set(SAMUS_Y, v);
    }
    pub fn x(&self) -> u8 {
        self.get(SAMUS_X)
    }
    pub fn set_x(&mut self, v: u8) {
        self.set(SAMUS_X, v);
    }

    pub fn jump_displacement(&self) -> u8 {
        self.get(SAMUS_JUMP_DISP)
    }
    pub fn set_jump_displacement(&mut self, v: u8) {
        self.set(SAMUS_JUMP_DISP, v);
    }

    pub fn sub_pixel_y(&self) -> u8 {
        self.get(SAMUS_SUBPIXEL_Y)
    }
    pub fn set_sub_pixel_y(&mut self, v: u8) {
        self.set(SAMUS_SUBPIXEL_Y, v);
    }
    pub fn sub_pixel_x(&self) -> u8 {
        self.get(SAMUS_SUBPIXEL_X)
    }
    pub fn set_sub_pixel_x(&mut self, v: u8) {
        self.set(SAMUS_SUBPIXEL_X, v);
    }
    pub fn speed_sub_pixel_y(&self) -> u8 {
        self.get(SAMUS_SPEED_SUBPIXEL_Y)
    }
    pub fn set_speed_sub_pixel_y(&mut self, v: u8) {
        self.set(SAMUS_SPEED_SUBPIXEL_Y, v);
    }
    pub fn speed_sub_pixel_x(&self) -> u8 {
        self.get(SAMUS_SPEED_SUBPIXEL_X)
    }
    pub fn set_speed_sub_pixel_x(&mut self, v: u8) {
        self.set(SAMUS_SPEED_SUBPIXEL_X, v);
    }

    pub fn accel_y(&self) -> i8 {
        self.get(SAMUS_ACCEL_Y) as i8
    }
    pub fn set_accel_y(&mut self, v: i8) {
        self.set(SAMUS_ACCEL_Y, v as u8);
    }
    pub fn accel_x(&self) -> i8 {
        self.get(SAMUS_ACCEL_X) as i8
    }
    pub fn set_accel_x(&mut self, v: i8) {
        self.set(SAMUS_ACCEL_X, v as u8);
    }

    /// 血量（`$0106` 低 + `$0107` 高）——`SamusHurt ; 07:F306` 的 `lda/sta` 两字节接力
    /// 就封在这里。
    pub fn health(&self) -> u16 {
        u16::from(self.get(HEALTH_LO)) | (u16::from(self.get(HEALTH_HI)) << 8)
    }

    pub fn set_health(&mut self, v: u16) {
        self.set(HEALTH_LO, (v & 0x00FF) as u8);
        self.set(HEALTH_HI, (v >> 8) as u8);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame::Manager;

    /// 各测独立持一份 `Manager`（短生命周期句柄的前提：无全局单例）。
    fn m() -> Manager {
        Manager::default()
    }

    /// 枚举 ↔ 字节往返：`status` 已 typed 化（真相在 `Manager.samus`），
    /// NES 字节面经 `snapshot_nes()` 投影得到。
    #[test]
    fn status_roundtrip_through_view() {
        let mut mem = m();
        mem.samus().set_status(SamusStatus::Elevator);
        assert_eq!(mem.snapshot_nes().ram[0x0300], 9, "sa_Elevator=9（旧注误记 4）");
        assert_eq!(mem.samus().status(), Some(SamusStatus::Elevator));
        // 地址面（兼容辖）读的是同一份 typed 状态。
        assert_eq!(mem.read_ram(0x0300), 9);
    }

    /// 未定义值不静默映射（fail-loud 由调用方决定），且**字节往返保真**。
    #[test]
    fn undefined_status_is_none() {
        let mut mem = m();
        mem.write_ram(0x0300, 0x0A);
        assert_eq!(mem.samus().status(), None);
        assert_eq!(mem.samus().status_raw(), 0x0A);
        assert_eq!(mem.snapshot_nes().ram[0x0300], 0x0A, "未定义值必须原样投影");
    }

    /// 单一真相：`$0300` 的真相在 typed 状态，直接改字节数组不改变它（这是 typed 化的定义）。
    #[test]
    fn raw_byte_write_does_not_override_typed_truth() {
        let mut mem = m();
        mem.samus().set_status(SamusStatus::Roll);
        mem.ram_mut()[0x0300] = 0x07; // 直接写存储层
        assert_eq!(mem.samus().status(), Some(SamusStatus::Roll), "typed 面不被裸字节改写");
        assert_eq!(mem.snapshot_nes().ram[0x0300], 3, "投影仍取 typed 值");
    }


    /// 速度/加速 typed 化：`i8` 与字节二补数互转，投影逐项落位。
    #[test]
    fn speeds_and_accel_are_typed_and_project() {
        let mut mem = m();
        mem.samus().set_speed_y(-2);
        mem.samus().set_speed_x(3);
        mem.samus().set_accel_y(-1);
        mem.samus().set_accel_x(7);
        let img = mem.snapshot_nes();
        assert_eq!(img.ram[0x0308], 0xFE, "speedY=-2 → $0308=$FE");
        assert_eq!(img.ram[0x0309], 0x03);
        assert_eq!(img.ram[0x0314], 0xFF, "accelY=-1 → $0314=$FF");
        assert_eq!(img.ram[0x0315], 0x07);
        // 地址面读的是同一份 typed 状态。
        assert_eq!(mem.read_ram(0x0308), 0xFE);
        assert_eq!(mem.samus().speed_y(), -2);
    }

    /// 字节面写入经 typed 解码后存取：`write_ram($0308,$FE)` ⇒ 速度就是 -2（不是 254）。
    #[test]
    fn byte_write_decodes_into_signed_typed_field() {
        let mut mem = m();
        mem.write_ram(0x0308, 0xFE);
        assert_eq!(mem.samus().speed_y(), -2);
        assert_eq!(mem.snapshot_nes().ram[0x0308], 0xFE, "往返保真");
    }

    /// typed 地址表与实际投影一致：每个 TYPED_ADDRS 项在投影上都能读回 typed 值。
    #[test]
    fn typed_addr_table_is_the_single_projection_source() {
        let mut mem = m();
        mem.write_ram(0x0300, 9);
        mem.write_ram(0x0308, 0x02);
        mem.write_ram(0x0309, 0x01);
        mem.write_ram(0x0314, 0x18);
        mem.write_ram(0x0315, 0x00);
        let img = mem.snapshot_nes();
        for a in SamusState::TYPED_ADDRS {
            let typed = mem.samus.get_byte(a).expect("TYPED_ADDRS 项必须可 typed 读");
            assert_eq!(img.ram[a as usize & 0x7FF], typed, "投影与 typed 面不一致 @ {a:#06X}");
        }
    }

    /// 速度是二补数：`-2` 写进 `$0308` 得 `0xFE`（旧形态断言，保留作回归）。
    #[test]
    fn speed_y_is_signed() {
        let mut mem = m();
        mem.samus().set_speed_y(-2);
        assert_eq!(mem.snapshot_nes().ram[0x0308], 0xFE);
        assert_eq!(mem.samus().speed_y(), -2);
    }

    /// 血量跨两字节：`0x0120` ⇒ `$0106=0x20` / `$0107=0x01`。
    #[test]
    fn health_spans_two_bytes() {
        let mut mem = m();
        mem.samus().set_health(0x0120);
        assert_eq!(mem.ram()[0x0106], 0x20);
        assert_eq!(mem.ram()[0x0107], 0x01);
        assert_eq!(mem.samus().health(), 0x0120);
    }

    /// 球形态/死亡族判据（asm `cmp #sa_Roll`、`status >= sa_Dead`）。
    #[test]
    fn roll_and_dead_predicates() {
        assert!(SamusStatus::Roll.is_roll());
        assert!(!SamusStatus::Stand.is_roll());
        assert!(SamusStatus::Dead.is_dead());
        assert!(SamusStatus::Dead2.is_dead());
        assert!(!SamusStatus::Elevator.is_dead());
    }
}
