//! Samus 状态语义视图——`$0300-$0315` 的字段面（地址只活在本文件）。
//!
//! 地址证据（A 级）：`refs/m1disasm/SRC/constants_ram.asm:218-238` + `:860-878`
//! （`Samus` 结构体 22 字节：status/radiusY/radiusX/animFrame/animDelay/animResetIndex/
//! animIndex/SamusOnElevator/speedY/speedX/isHit/onScreen/hi/y/x/SamusJumpDsplcmnt/
//! SamusSubPixelY/SamusSubPixelX/SamusSpeedSubPixelY/SamusSpeedSubPixelX/SamusAccelY/
//! SamusAccelX）；`Samus.status` 取值=`constants.asm:112` 的 `sa_*` 十态。
//!
//! 形态纪律（typed 域三条细则；迁移期唯一权威位置=本文件头，其余域沿用）：
//! - `status` 出 [`SamusStatus`] 枚举而不是裸 `u8`——调用方写 `SamusStatus::Roll` 而不是 `3`；
//! - 速度/加速出 `i8`（asm 侧本就是二补数，`lda/lsr/ror` 的算术在视图里不外泄）；
//! - `health` 跨 `$0106/$0107` 两个字节，视图出 `u16` 并封装「低字节在 $0106」的规则。

use crate::memmap::nesmap::{NesSpan, SpanKey};
use crate::ram_map::{
    SAMUS_ACCEL_X, SAMUS_ACCEL_Y, SAMUS_ANIM_FRAME, SAMUS_HI, SAMUS_IS_HIT, SAMUS_JUMP_DISP,
    SAMUS_ON_ELEVATOR, SAMUS_RADIUS_X, SAMUS_RADIUS_Y, SAMUS_SCR_X, SAMUS_SPEED_SUBPIXEL_X,
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
/// `SamusHorizontalSpeedMax`（`$0316`）——asm 里紧贴 `Samus` 结构体之后的**独立一字节**，
/// 但语义上是 Samus 域状态（跑速上限，`SamusSetHorizontalSpeed ; 07:E4A9` 读它钳制）。
/// typed 面按**域**收（不是按 asm 结构体边界），故它以第 23 个 typed 字节存在。
const HORZ_SPEED_MAX: u16 = 0x0316;
/// `Health` 低字节（`constants_ram.asm:806`）。
pub(crate) const HEALTH_LO: u16 = 0x0106;
/// `Health` 高字节。
pub(crate) const HEALTH_HI: u16 = 0x0107;

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
/// `Samus` 的位置三元组（`$030C hi` / `$030D y` / `$030E x`）。
///
/// 三字节必须**一起**看才是坐标：`hi` 是 256px 页内的高位（滚动/换屏时翻转），
/// 把 `(hi,y,x)` 拆成三个独立字节正是"回绕逻辑散落各处"的根源。typed 化后由
/// [`Position`] 承载，后续把 `y±dy` 的回绕封成方法时只改这一处。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Position {
    pub hi: u8,
    pub y: u8,
    pub x: u8,
}

impl Position {
    /// 16 位屏内 y（`hi:y` 拼接，asm 侧 `hi*256+y` 的语义面）。
    pub fn y16(self) -> u16 {
        (self.hi as u16) << 8 | self.y as u16
    }
}

/// `Samus` 的动画四元组（`$0303 frame` / `$0304 delay` / `$0305 resetIndex` / `$0306 index`）。
///
/// 四者是一个整体：`index` 指向 anim 表项、`frame` 是该表项内的帧计数、`delay` 是**帧内**
/// 倒计时、`resetIndex` 是动画结束后的回卷目标。拆开存取就会出现「改了 index 忘了 reset」
/// 这类错位（`InitEnAnimIndex` 一族维护的正是这个不变量）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Anim {
    pub frame: u8,
    pub delay: u8,
    pub reset: u8,
    pub index: u8,
}

/// 碰撞半径（`$0301 radiusY` / `$0302 radiusX`）。
///
/// 两者成对参与「物体边界是否落在瓦片边界上」的判定（`EnemyMoveOnePixelDown` 的
/// `(y+radiusY)&7==0` 一族），拆开存最容易出现的错是「只改 y 忘了 x」。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Radius {
    pub y: u8,
    pub x: u8,
}

/// 子像素累加器（`$0310 subPixelY` / `$0311 subPixelX` / `$0312 speedSubPixelY` / `$0313 speedSubPixelX`）。
///
/// 这是 6502 里「8.8 定点数」的小数面：整数部在 `$0308/$0309`，本结构是小数部。
/// `lsr/ror` 的半速反弹、`asl/rol` 的加速都在这一对之间搬运位——所以它们必须一起看，
/// 单独改一个 e 半速运算就错在末位。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SubPixel {
    pub y: u8,
    pub x: u8,
    pub speed_y: u8,
    pub speed_x: u8,
}

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
    /// 位置三元组 `$030C/$030D/$030E`。
    pos: Position,
    /// 动画四元组 `$0303-$0306`。
    anim: Anim,
    /// 碰撞半径 `$0301/$0302`。
    radius: Radius,
    /// `SamusOnElevator`（`$0307`）。
    on_elevator: u8,
    /// `Samus.isHit`（`$030A`）——**位域**（`& $41 == $40` 一类判据），保持 `u8`。
    is_hit: u8,
    /// `Samus.onScreen`（`$030B`）。
    on_screen: u8,
    /// `SamusJumpDsplcmnt`（`$030F`）。
    jump_disp: u8,
    /// 子像素四件 `$0310-$0313`。
    sub: SubPixel,
    /// `SamusHorizontalSpeedMax`（`$0316`）——跑速上限（0x18=1.5px/帧）。
    horz_speed_max: u8,
    /// 本拍是否被载房窗吞掉（`CheckUpdateNameTable ; 07:E54A` 首件 `jsr SetupRoom`
    /// 跑过帧边界——asm 的像素写点 `07:E65E`/`07:E692` 在 `jsr ScrollLeft` **之后**，
    /// 必然落在装载窗结束那一帧）。**非 RAM 字段**：纯行为相位记账。
    pub load_consumed_frame: bool,
    /// 被载房窗挂起的待兑像素（`false`=左移 1 px / `true`=右移 1 px）。
    /// 兑现点 = 迭代门队列排空那一拍（`Engine::end_of_tick_gate`）。
    pub deferred_horz_px: Option<bool>,
}

impl Default for SamusState {
    fn default() -> Self {
        Self {
            status: SamusStatus::Stand,
            speed_y: 0,
            speed_x: 0,
            accel_y: 0,
            accel_x: 0,
            pos: Position::default(),
            anim: Anim::default(),
            radius: Radius::default(),
            on_elevator: 0,
            is_hit: 0,
            on_screen: 0,
            jump_disp: 0,
            sub: SubPixel::default(),
            horz_speed_max: 0,
            load_consumed_frame: false,
            deferred_horz_px: None,
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
            SAMUS_HI => self.pos.hi,
            SAMUS_Y => self.pos.y,
            SAMUS_X => self.pos.x,
            SAMUS_ANIM_FRAME => self.anim.frame,
            ANIM_DELAY => self.anim.delay,
            ANIM_RESET_INDEX => self.anim.reset,
            ANIM_INDEX => self.anim.index,
            SAMUS_RADIUS_Y => self.radius.y,
            SAMUS_RADIUS_X => self.radius.x,
            SAMUS_ON_ELEVATOR => self.on_elevator,
            SAMUS_IS_HIT => self.is_hit,
            ON_SCREEN => self.on_screen,
            SAMUS_JUMP_DISP => self.jump_disp,
            SAMUS_SUBPIXEL_Y => self.sub.y,
            SAMUS_SUBPIXEL_X => self.sub.x,
            SAMUS_SPEED_SUBPIXEL_Y => self.sub.speed_y,
            SAMUS_SPEED_SUBPIXEL_X => self.sub.speed_x,
            HORZ_SPEED_MAX => self.horz_speed_max,
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
            SAMUS_HI => self.pos.hi = v,
            SAMUS_Y => self.pos.y = v,
            SAMUS_X => self.pos.x = v,
            SAMUS_ANIM_FRAME => self.anim.frame = v,
            ANIM_DELAY => self.anim.delay = v,
            ANIM_RESET_INDEX => self.anim.reset = v,
            ANIM_INDEX => self.anim.index = v,
            SAMUS_RADIUS_Y => self.radius.y = v,
            SAMUS_RADIUS_X => self.radius.x = v,
            SAMUS_ON_ELEVATOR => self.on_elevator = v,
            SAMUS_IS_HIT => self.is_hit = v,
            ON_SCREEN => self.on_screen = v,
            SAMUS_JUMP_DISP => self.jump_disp = v,
            SAMUS_SUBPIXEL_Y => self.sub.y = v,
            SAMUS_SUBPIXEL_X => self.sub.x = v,
            SAMUS_SPEED_SUBPIXEL_Y => self.sub.speed_y = v,
            SAMUS_SPEED_SUBPIXEL_X => self.sub.speed_x = v,
            HORZ_SPEED_MAX => self.horz_speed_max = v,
            _ => return false,
        }
        true
    }

    /// 迁入 typed 的地址清单（`snapshot_nes()` 逐项投影用——**唯一投影源**）。
    /// `$0300-$0315` 全 22 字节（asm `Samus` 结构体一字节对一字段）——迁完即整段退出
    /// `NesStorage.ram`，本表成为「域状态 ↔ NES 字节」的唯一映射。
    pub const TYPED_ADDRS: [u16; 23] = [
        SAMUS_STATUS,
        SAMUS_SPEED_Y_INT,
        SAMUS_SPEED_X_INT,
        SAMUS_ACCEL_Y,
        SAMUS_ACCEL_X,
        SAMUS_HI,
        SAMUS_Y,
        SAMUS_X,
        SAMUS_ANIM_FRAME,
        ANIM_DELAY,
        ANIM_RESET_INDEX,
        ANIM_INDEX,
        SAMUS_RADIUS_Y,
        SAMUS_RADIUS_X,
        SAMUS_ON_ELEVATOR,
        SAMUS_IS_HIT,
        ON_SCREEN,
        SAMUS_JUMP_DISP,
        SAMUS_SUBPIXEL_Y,
        SAMUS_SUBPIXEL_X,
        SAMUS_SPEED_SUBPIXEL_Y,
        SAMUS_SPEED_SUBPIXEL_X,
        HORZ_SPEED_MAX,
    ];

    /// typed 面在 `$0300` 起的**连续**字节数（= [`Self::TYPED_ADDRS`] 长度）——对象池声明
    /// 「slot0 是 Samus 的 typed overlay」时用它定 `View` 段长度（不另立数字）。
    pub const TYPED_LEN: u16 = Self::TYPED_ADDRS.len() as u16;

    pub fn pos(&self) -> Position {
        self.pos
    }

    pub fn set_pos(&mut self, p: Position) {
        self.pos = p;
    }

    pub fn anim(&self) -> Anim {
        self.anim
    }

    pub fn set_anim(&mut self, a: Anim) {
        self.anim = a;
    }

    pub fn radius(&self) -> Radius {
        self.radius
    }

    pub fn set_radius(&mut self, r: Radius) {
        self.radius = r;
    }

    pub fn sub(&self) -> SubPixel {
        self.sub
    }

    pub fn set_sub(&mut self, s: SubPixel) {
        self.sub = s;
    }
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
        #[cfg(test)]
        crate::memmap::verify_shim::note_read(addr);
        self.mem.ram()[(addr as usize) & 0x7FF]
    }

    #[inline]
    fn set(&mut self, addr: u16, v: u8) {
        if self.mem.samus.set_byte(addr, v) {
            return;
        }
        #[cfg(test)]
        crate::memmap::verify_shim::note_write(addr);
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
    /// `$0305 animResetIndex`——动画播完回卷到的表项索引。
    pub fn set_anim_reset_index(&mut self, v: u8) {
        self.set(ANIM_RESET_INDEX, v);
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
    /// 位置三元组（typed 面：一次拿全 `hi/y/x`，避免三字节各自漂移）。
    pub fn pos(&self) -> Position {
        self.mem.samus.pos()
    }

    pub fn set_pos(&mut self, p: Position) {
        self.mem.samus.set_pos(p);
    }

    /// 动画四元组（typed 面）。
    pub fn anim(&self) -> Anim {
        self.mem.samus.anim()
    }

    pub fn set_anim(&mut self, a: Anim) {
        self.mem.samus.set_anim(a);
    }

    pub fn radius(&self) -> Radius {
        self.mem.samus.radius()
    }

    pub fn set_radius(&mut self, r: Radius) {
        self.mem.samus.set_radius(r);
    }

    pub fn sub(&self) -> SubPixel {
        self.mem.samus.sub()
    }

    pub fn set_sub(&mut self, s: SubPixel) {
        self.mem.samus.set_sub(s);
    }

    pub fn set_x(&mut self, v: u8) {
        self.set(SAMUS_X, v);
    }

    /// 本拍是否被载房窗吞掉（**非 RAM 字段**——纯行为相位记账；asm 依据见
    /// `crates/game-core/src/samus/physics.rs::apply_deferred_horz_px` 的文档注释）。
    pub fn load_consumed_frame(&self) -> bool {
        self.mem.samus.load_consumed_frame
    }
    pub fn set_load_consumed_frame(&mut self, v: bool) {
        self.mem.samus.load_consumed_frame = v;
    }

    /// `SamusScrX`（`$0051`）——萨姆斯在屏幕内的 x（`x − ScrollX`）。
    pub fn scr_x(&self) -> u8 {
        self.get(SAMUS_SCR_X)
    }
    pub fn set_scr_x(&mut self, v: u8) {
        self.set(SAMUS_SCR_X, v);
    }

    /// 被载房窗挂起的待兑水平像素（`false`=左移 1 px / `true`=右移 1 px）。
    pub fn deferred_horz_px(&self) -> Option<bool> {
        self.mem.samus.deferred_horz_px
    }
    pub fn take_deferred_horz_px(&mut self) -> Option<bool> {
        self.mem.samus.deferred_horz_px.take()
    }
    pub fn set_deferred_horz_px(&mut self, v: Option<bool>) {
        self.mem.samus.deferred_horz_px = v;
    }

    pub fn jump_displacement(&self) -> u8 {
        self.get(SAMUS_JUMP_DISP)
    }
    pub fn set_jump_displacement(&mut self, v: u8) {
        self.set(SAMUS_JUMP_DISP, v);
    }

    /// `SamusHorizontalSpeedMax`（`$0316`）——跑速上限（0x18=1.5px/帧）。
    pub fn horz_speed_max(&self) -> u8 {
        self.get(HORZ_SPEED_MAX)
    }
    pub fn set_horz_speed_max(&mut self, v: u8) {
        self.set(HORZ_SPEED_MAX, v);
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

/// Samus 血量两字节未 typed 的缺口说明（`SPANS` 里唯一一条非 `Owner`）。
const TODO: &str = "血量两字节未 typed：health()/set_health() 仍走字节面（$0106 低 / $0107 高；crates/game-core/src/samus/state.rs:656）";

impl crate::memmap::nesmap::NesMapped for SamusState {
    /// 判别字段：`status`（`$0300`，`sa_*` 十态）——同址按状态重解释时的判别源
    /// （P3 的判别感知腿用；P2 只登记，不参与 `coverage` 计位）。
    const DISCRIMINATOR: Option<u16> = Some(SAMUS_STATUS);

    /// **23 条逐字节 `Owner` 声明**（`$0300-$0316`，按地址升序）：typed 面每字节恰一个 owner；
    /// 末尾 2 条是 `Unmodeled` 的血量两字节（`$0106-$0107`，仍在字节面）——共 25 条。
    ///
    /// 手写而非循环生成——循环会让声明退化成 [`Self::TYPED_ADDRS`] 的回声，覆盖度核算
    /// 就变成自证（`docs/spec/entity-ownership-and-nesmap.md` §6 防自证）。地址取既有常量：
    /// 本文件豁免 `no-raw-addr`，但声明仍不许再写一遍字面量（否则地址有了两个家）。
    const SPANS: &'static [(NesSpan, SpanKey)] = &[
        (NesSpan::value(SAMUS_STATUS, 1), SpanKey::Owner),
        (NesSpan::value(SAMUS_RADIUS_Y, 1), SpanKey::Owner),
        (NesSpan::value(SAMUS_RADIUS_X, 1), SpanKey::Owner),
        (NesSpan::value(SAMUS_ANIM_FRAME, 1), SpanKey::Owner),
        (NesSpan::value(ANIM_DELAY, 1), SpanKey::Owner),
        (NesSpan::value(ANIM_RESET_INDEX, 1), SpanKey::Owner),
        (NesSpan::value(ANIM_INDEX, 1), SpanKey::Owner),
        (NesSpan::value(SAMUS_ON_ELEVATOR, 1), SpanKey::Owner),
        (NesSpan::value(SAMUS_SPEED_Y_INT, 1), SpanKey::Owner),
        (NesSpan::value(SAMUS_SPEED_X_INT, 1), SpanKey::Owner),
        (NesSpan::value(SAMUS_IS_HIT, 1), SpanKey::Owner),
        (NesSpan::value(ON_SCREEN, 1), SpanKey::Owner),
        (NesSpan::value(SAMUS_HI, 1), SpanKey::Owner),
        (NesSpan::value(SAMUS_Y, 1), SpanKey::Owner),
        (NesSpan::value(SAMUS_X, 1), SpanKey::Owner),
        (NesSpan::value(SAMUS_JUMP_DISP, 1), SpanKey::Owner),
        (NesSpan::value(SAMUS_SUBPIXEL_Y, 1), SpanKey::Owner),
        (NesSpan::value(SAMUS_SUBPIXEL_X, 1), SpanKey::Owner),
        (NesSpan::value(SAMUS_SPEED_SUBPIXEL_Y, 1), SpanKey::Owner),
        (NesSpan::value(SAMUS_SPEED_SUBPIXEL_X, 1), SpanKey::Owner),
        (NesSpan::value(SAMUS_ACCEL_Y, 1), SpanKey::Owner),
        (NesSpan::value(SAMUS_ACCEL_X, 1), SpanKey::Owner),
        (NesSpan::value(HORZ_SPEED_MAX, 1), SpanKey::Owner),
        // 血量两字节（`$0106` 低 / `$0107` 高）：**未 typed**——`health()` 走 `get()` 的字节面回退。
        (NesSpan::value(HEALTH_LO, 2), SpanKey::Unmodeled(TODO)),
    ];

    /// 逐字节取 `get_byte`（typed 真相）写进 RAM 面——与 `snapshot_nes()` 同源同值。
    fn project(&self, out: &mut crate::memmap::storage::NesImage) {
        for (span, key) in Self::SPANS {
            if !matches!(key, crate::memmap::nesmap::SpanKey::Owner) {
                continue;
            }
            for a in span.addrs() {
                if let Some(v) = self.get_byte(a) {
                    out.ram[usize::from(a) & (crate::memmap::nesmap::RAM_LEN - 1)] = v;
                }
            }
        }
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

    /// 位置三元组 typed 化：`(hi,y,x)` 一起存取，投影三字节逐项落位。
    #[test]
    fn position_triple_is_typed_and_projects() {
        let mut mem = m();
        mem.samus().set_pos(Position { hi: 0x01, y: 0x9D, x: 0xD8 });
        let img = mem.snapshot_nes();
        assert_eq!((img.ram[0x030C], img.ram[0x030D], img.ram[0x030E]), (0x01, 0x9D, 0xD8));
        assert_eq!(mem.samus().pos().y16(), 0x019D);
        // 地址面同源
        assert_eq!(mem.read_ram(0x030D), 0x9D);
        mem.write_ram(0x030C, 0x00);
        assert_eq!(mem.samus().pos().hi, 0x00, "字节写落 typed");
    }

    /// 动画四元组 typed 化：四字节一起存取，投影逐项落位（`index/reset` 不再各自漂移）。
    #[test]
    fn anim_quad_is_typed_and_projects() {
        let mut mem = m();
        mem.samus().set_anim(Anim { frame: 0x12, delay: 0x01, reset: 0x20, index: 0x21 });
        let img = mem.snapshot_nes();
        assert_eq!(
            (img.ram[0x0303], img.ram[0x0304], img.ram[0x0305], img.ram[0x0306]),
            (0x12, 0x01, 0x20, 0x21)
        );
        assert_eq!(mem.samus().anim().index, 0x21);
        // 地址面同源（asm 侧仍按字节改写，落 typed）
        mem.write_ram(0x0306, 0x30);
        assert_eq!(mem.samus().anim().index, 0x30);
        assert_eq!(mem.snapshot_nes().ram[0x0306], 0x30);
    }

    /// 余下 10 字节（半径/电梯/受伤/屏内/跳跃位移/子像素）typed 化：投影逐项落位。
    #[test]
    fn remaining_fields_are_typed_and_project() {
        let mut mem = m();
        mem.samus().set_radius(Radius { y: 0x0F, x: 0x04 });
        mem.samus().set_sub(SubPixel { y: 0x12, x: 0x34, speed_y: 0x56, speed_x: 0x78 });
        mem.write_ram(0x0307, 1);
        mem.write_ram(0x030A, 0x40);
        mem.write_ram(0x030B, 1);
        mem.write_ram(0x030F, 0x3A);
        let img = mem.snapshot_nes();
        assert_eq!((img.ram[0x0301], img.ram[0x0302]), (0x0F, 0x04));
        assert_eq!((img.ram[0x0310], img.ram[0x0311], img.ram[0x0312], img.ram[0x0313]), (0x12, 0x34, 0x56, 0x78));
        assert_eq!((img.ram[0x0307], img.ram[0x030A], img.ram[0x030B], img.ram[0x030F]), (1, 0x40, 1, 0x3A));
    }

    /// 全 22 字节覆盖：`$0300-$0315` 每一格都必须可 typed 读写（无遗漏、无重复）。
    #[test]
    /// **投影表与解码表必须同源**：`get_byte` 的 match 与 `TYPED_ADDRS` 是两份手写清单，
    /// `len()==23` 只能抓"数错"，抓不住"只往一边加一字节"（症状：`read_ram` 读到新值、
    /// ram.hex dump 仍是旧字节——全绿）。这里逐地址断言两者等价。
    #[test]
    fn typed_addrs_and_decoder_agree_on_every_address() {
        let mut mem = m();
        for a in 0x0300u16..=0x03FF {
            assert_eq!(
                mem.samus.get_byte(a).is_some(),
                SamusState::TYPED_ADDRS.contains(&a),
                "{a:#06X}：解码面与 TYPED_ADDRS 不一致（加字段忘登记投影表会命中这里）"
            );
        }
        // 反向也查一遍：TYPED_ADDRS 里的每一项都必须真能解码。
        for &a in SamusState::TYPED_ADDRS.iter() {
            assert!(mem.samus.get_byte(a).is_some(), "{a:#06X} 登记了但解不出");
        }
    }

    #[test]
    fn all_22_samus_bytes_are_typed() {
        let mut mem = m();
        let mut seen = std::collections::BTreeSet::new();
        for a in 0x0300u16..=0x0315 {
            let v = (a as u8) ^ 0x5A;
            assert!(mem.samus.set_byte(a, v), "{a:#06X} 必须已 typed 化");
            assert_eq!(mem.samus.get_byte(a), Some(v), "{a:#06X} 往返不一致");
            seen.insert(a);
        }
        assert_eq!(seen.len(), 22, "asm Samus 结构体 $0300-$0315 共 22 字节");
        // 第 23 项是 `SamusHorizontalSpeedMax`（$0316）——asm 里在结构体之外，但属 Samus 域。
        assert!(mem.samus.set_byte(0x0316, 0x18), "$0316 必须已 typed 化");
        assert_eq!(mem.samus.get_byte(0x0316), Some(0x18));
        assert_eq!(SamusState::TYPED_ADDRS.len(), 23);
        let img = mem.snapshot_nes();
        for a in 0x0300u16..=0x0315 {
            assert_eq!(img.ram[a as usize], (a as u8) ^ 0x5A, "投影 {a:#06X}");
        }
        assert_eq!(img.ram[0x0316], 0x18, "水平速上限也是 typed 面");
    }

    /// `$0316` 走 typed：地址面、视图面、投影面三者同源。
    #[test]
    fn horz_speed_max_is_typed_on_all_three_faces() {
        let mut mem = m();
        mem.write_ram(0x0316, 0x18);
        assert_eq!(mem.samus().horz_speed_max(), 0x18, "视图面");
        assert_eq!(mem.read_ram(0x0316), 0x18, "地址面");
        assert_eq!(mem.snapshot_nes().ram[0x0316], 0x18, "投影面");
        mem.samus().set_horz_speed_max(0x12);
        assert_eq!(mem.read_ram(0x0316), 0x12, "视图写回地址面");
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

/// # 行为面：聚合根开始承载自己的逻辑
///
/// 第 1 步（`5817a049`）只做了「typed 类搬到实体目录」——那是搬家。这一步把**纯自域**
/// 的过程搬进来，`Samus` 才真的是聚合根：判断与写入都建立在同一份不变量上（status 用
/// 枚举、竖速用 `i8` 语义），跨域写入仍按「取句柄 → 短句柄用完即还」的顺序穿过
/// `self.mem`（与 asm 的 `lda/sta` 顺序同构）。
impl<'a> Samus<'a> {
    /// 球形态落地反弹（asm 内容锚 `SamusMoveVertically ; 07:E2A9` 的 `@loop_down`
    /// `; 07:E2E3` 受阻分支 `$E2E6-$E309`）：`status != sa_Roll` ⇒ `@landingNoBall`；
    /// 否则 `lsr Samus.speedY`（进位=原 bit0）——减半后为 0 ⇒ `@landingNoBounce`，
    /// 非 0 ⇒ `ror SamusSpeedSubPixelY` 后**双取负**（第二次带第一次借位）
    /// ⇒ `jmp SamusMoveHorizontally`。返回 true=已走反弹径（调用方不得再走落地清零）。
    pub(crate) fn ball_bounce_after_landing(&mut self) -> bool {
        if self.status() != Some(SamusStatus::Roll) {
            return false; // sa_Roll（constants.asm:106）
        }
        let si = self.get(SAMUS_SPEED_Y_INT);
        let half = si >> 1;
        let carry = si & 1;
        self.set(SAMUS_SPEED_Y_INT, half); // `lsr Samus.speedY` 原址写
        if half == 0 {
            return false; // `beq @landingNoBounce`
        }
        let sf = self.get(SAMUS_SPEED_SUBPIXEL_Y);
        let ror = (sf >> 1) | (carry << 7); // `ror SamusSpeedSubPixelY`
        let ns = 0u8.wrapping_sub(ror);
        let borrow = u8::from(ror != 0);
        let nsi = 0u8.wrapping_sub(half).wrapping_sub(borrow);
        self.set(SAMUS_SPEED_SUBPIXEL_Y, ns);
        self.set(SAMUS_SPEED_Y_INT, nsi);
        true
    }

    /// 岩浆态副作用（`LavaAndMoveCheck ; 07:E269`）：击退方向 `$FF`、清 `HealthChange`、
    /// 无敌延时 `$32`、每 4 帧播一次跳跃 SFX、伤害节拍 `(FrameCount>>1)&3==0`
    /// （无 Varia 该拍必伤；有 Varia 仅 `FrameCount` 奇数的拍半伤）。
    pub(crate) fn apply_lava_state(&mut self, in_lava: bool) {
        if !in_lava {
            self.mem.write_ram(crate::ram_map::SAMUS_IN_LAVA, 0);
            return;
        }
        self.mem.write_ram(crate::ram_map::SAMUS_KNOCKBACK_DIR, 0xFF);
        self.mem.write_ram(crate::ram_map::HEALTH_CHANGE, 0);
        self.mem.write_ram(crate::ram_map::HEALTH_CHANGE + 1, 0);
        self.mem
            .write_ram(crate::ram_map::SAMUS_INVINCIBLE_DELAY, 0x32);
        let fc = self.mem.read_ram(crate::ram_map::FRAME_COUNT);
        if fc & 0x03 == 0 {
            // SFX_SamusJump（constants.asm:160 `sfxSQ1_SamusJump = %00000010`）
            let f = self.mem.read_ram(0x0681) | 0x02;
            self.mem.write_ram(0x0681, f);
        }
        if (fc >> 1) & 0x03 == 0 {
            // gr_VARIA=%00100000（constants.asm:75）×SamusGear($6878)
            let has_varia = self.mem.read_ram(crate::ram_map::SAMUS_GEAR) & 0x20 != 0;
            let carry = fc & 0x01 != 0; // asm `lda FrameCount / lsr` 的进位
            if !has_varia || carry {
                self.mem.write_ram(crate::ram_map::HEALTH_CHANGE, 0x07);
                crate::weapon::projectile::subtract_health(&mut *self.mem);
            }
        }
        self.mem.write_ram(crate::ram_map::SAMUS_IN_LAVA, 1);
    }

    /// 水平移动受阻判定（`CheckStopHorzMvmt ; 07:E365`）：返回 true=环终止。
    /// `moved`=本帧是否真的移动成功（asm 进位）。
    pub(crate) fn check_stop_horz_mvmt(&mut self, moved: bool) -> bool {
        if moved {
            return false; // bcs Exit10
        }
        self.mem.write_ram(crate::ram_map::OBJECT_COUNTER, 1); // 断环
        if self.get(SAMUS_ACCEL_Y) != 0 {
            return true; // 空中出
        }
        // asm `cmp #sa_Roll`（$E370）= **3**；旧形误写 4（=sa_PntUp）——
        // 后果是球形 Samus 撞墙被强行 `SetSamusStand`（球被「站起来」）。
        if self.status() == Some(SamusStatus::Roll) {
            return true; // sa_Roll 显式放行
        }
        crate::samus::handler::set_samus_stand(&mut *self.mem); // jmp SetSamusStand（CF55）
        true
    }
}
