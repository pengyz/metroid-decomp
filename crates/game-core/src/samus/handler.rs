//! Samus 状态机脊柱（bank07:CC0D 族；task-25 首件——先立脊柱后填肉范式③）。
//!
//! **asm 内容锚+结构互核注**（施工令②精化版）——
//! refs/m1disasm/SRC/prg7_engine.asm `UpdateSamus: ; 07:CC0D`（`ldx #$00/stx PageIndex`/
//! `inx/stx IsSamus`/`jsr GoSamusHandler`/`dec IsSamus`/`rts`）+ `GoSamusHandler: ;
//! 07:CC1A`（`lda Samus.status/bmi SamusStand`（bit7=站立）/`jsr JumpEngine`（$C27C
//! 表分派）/10 项 `.word` 表）。
//!
//! **结构互核注**：①10 态分发表=GoMainRoutine/title 31 项同构法第三次用（脊柱范式）
//! ②`bmi` 位 7=站立旗（Samus.status 位语义——Object 结构 $0300 面 status 注
//! 「0=object slot not in use」+位 7 站立扩展）③JumpEngine 内联表返回址读栈（`pla`×2）
//! ——栈面一层，域形=match 分派复刻。
//!
//! R-mv 挂点（spec 源=docs/spec/samus-movement.md）：状态转移=规则触发面。
//!
//! 地址面：PageIndex=$4B/IsSamus=$55（constants_ram 内容锚）；Samus.status=$0300
//! （Object 结构首字段——constants_ram `.struct Object` 内容锚）。

use crate::state::FrameMemory; // G11 破环：类型层直取（原经 frame/）
use crate::ram_map::{IS_SAMUS, PAGE_INDEX, SAMUS_STATUS};

/// Samus 处理态（GoSamusHandler 表序自证——asm `.word` 10 项）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SamusHandler {
    /// $CC36 站立（含位 7 站立旗直入）。
    Stand,
    /// $CCC2 奔跑。
    Run,
    /// $D002 跳跃。
    Jump,
    /// $D0E1 滚动。
    Roll,
    /// $D198 上指。
    PntUp,
    /// $D3A8 门内滚动。
    Door,
    /// $D002 上指跳跃（表第 7 项——与 Jump 同标的 asm 侧）。
    JumpPntUp,
    /// $D41A 死亡。
    Dead,
    /// $D41F 二次死亡。
    Dead2,
    /// $D423 电梯。
    Elevator,
}

impl SamusHandler {
    /// 表序→态（GoSamusHandler `.word` 序自证）。
    pub fn from_index(i: u8) -> Option<Self> {
        Some(match i {
            0 => Self::Stand,
            1 => Self::Run,
            2 => Self::Jump,
            3 => Self::Roll,
            4 => Self::PntUp,
            5 => Self::Door,
            6 => Self::JumpPntUp,
            7 => Self::Dead,
            8 => Self::Dead2,
            9 => Self::Elevator,
            _ => return None,
        })
    }

    /// 态→处理件址（fail-loud 引爆面——各状态处理族随 move 相队列填）。
    pub fn handler_addr(self) -> &'static str {
        match self {
            Self::Stand => "bank07:CC36",
            Self::Run => "bank07:CCC2",
            Self::Jump | Self::JumpPntUp => "bank07:D002",
            Self::Roll => "bank07:D0E1",
            Self::PntUp => "bank07:D198",
            Self::Door => "bank07:D3A8",
            Self::Dead => "bank07:D41A",
            Self::Dead2 => "bank07:D41F",
            Self::Elevator => "bank07:D423",
        }
    }
}

/// UpdateSamus（asm :2705 内容锚）：PageIndex=0/IsSamus=1→处理→IsSamus=0。
pub fn update_samus(mem: &mut FrameMemory) {
    mem.write_ram(PAGE_INDEX, 0); // `ldx #$00/stx PageIndex`
    mem.write_ram(IS_SAMUS, 1); // `inx/stx IsSamus`
    go_samus_handler(mem);
    mem.write_ram(IS_SAMUS, 0); // `dec IsSamus`
}

/// GoSamusHandler 分发壳（asm :2716 族内容锚）：status 位 7=站立直入；否则表序分派。
pub fn go_samus_handler(mem: &mut FrameMemory) {
    let status = mem.read_ram(SAMUS_STATUS);
    let handler = if status & 0x80 != 0 {
        SamusHandler::Stand // `bmi SamusStand`
    } else {
        match SamusHandler::from_index(status & 0x0F) {
            Some(h) => h,
            None => crate::unported!("bank07:CC1A-index"), // 表外索引（0x0A-0x0F）
        }
    };
    // 【r26 接线】dispatch→处理器族（9 件已在——旧恒爆桩清）
    match handler {
        SamusHandler::Stand => samus_stand(mem),
        SamusHandler::Run => samus_run(mem),
        SamusHandler::Jump => samus_jump(mem),
        SamusHandler::JumpPntUp => samus_jump(mem), // asm 表 6=SamusJump 同件（上指跳！）
        SamusHandler::Roll => samus_roll(mem),
        SamusHandler::PntUp => samus_pnt_up(mem),
        SamusHandler::Door => samus_door(mem),
        SamusHandler::Dead => samus_dead(mem),
        SamusHandler::Dead2 => samus_dead2(mem),
        SamusHandler::Elevator => samus_elevator(mem),
    }
}

/// SamusRun 处理件（**字节自证**——10 态肉批2；asm :CCC2）：
/// 跳跃动画状态机（`ldy SamusJumpDsplcmnt/bit Samus.speedY($308)`）：
/// 下落（bit7=0）：位移<$18→unspin（anim=$0C）；上升（bit7=1）：位移≥$18→salto（$0E，
/// 若现 anim≠$20 JumpFire）→≥$20 时 UP 按→JumpPntUp（$35）+A 未按→StopVertMovement。
/// 字节自证（CCC2 区逐字）：anim 常量 $0C=Jump/$20=JumpFire/$0E=Salto/$35=JumpPntUp。
/// 尾段（CD02+run 动画/movement）+StopVertMovement（D147）=桩（随队列填）。
pub fn samus_run(mem: &mut FrameMemory) {
    use button::*;
    let disp = mem.read_ram(crate::ram_map::SAMUS_JUMP_DISP);
    let speed_y_i = mem.read_ram(super::physics::SAMUS_SPEED_Y_I);
    if mem.read_ram(crate::ram_map::SAMUS_ACCEL_Y) != 0 {
        if speed_y_i & 0x80 == 0 {
            // 下落：位移<$18→unspin
            if disp < 0x18 {
                mem.write_ram(ANIM_RESET_INDEX, 0x0C);
            }
        } else {
            // 上升：≥$18→salto（若现≠JumpFire）
            if disp >= 0x18 {
                if mem.read_ram(ANIM_RESET_INDEX) != 0x20 {
                    mem.write_ram(ANIM_RESET_INDEX, 0x0E);
                }
                if disp >= 0x20 {
                    if mem.read_ram(crate::ram_map::JOY1_STATUS) & UP != 0 {
                        mem.write_ram(ANIM_RESET_INDEX, 0x35);
                    }
                    if mem.read_ram(crate::ram_map::JOY1_STATUS) & A == 0 {
                        super::physics::stop_vert_movement(mem); // D147（已全折）
                    }
                }
            }
        }
    }
    crate::unported!("bank07:CD02"); // 尾段（run 动画/movement 面）
}

/// 动画面址（字节自证：$0305=animResetIndex（CF5D/CCC2 同址一致））。
pub const ANIM_RESET_INDEX: u16 = 0x0305;

#[cfg(test)]
mod tests {
    use super::*;

    /// 10 态表序自证（from_index 全覆盖）+分发引爆面。
    #[test]
    fn samus_handler_dispatch_ten_states() {
        for (i, addr) in [
            (0u8, "bank07:CC36"), (1, "bank07:CCC2"), (2, "bank07:D002"), (3, "bank07:D0E1"),
            (4, "bank07:D198"), (5, "bank07:D3A8"), (6, "bank07:D002"), (7, "bank07:D41A"),
            (8, "bank07:D41F"), (9, "bank07:D423"),
        ] {
            assert_eq!(SamusHandler::from_index(i).unwrap().handler_addr(), addr);
        }
        assert!(SamusHandler::from_index(10).is_none());
    }

    /// BitScan 首位扫描（bit 号语义）。
    #[test]
    fn bit_scan_first_set_bit() {
        assert_eq!(bit_scan(0x01), 0);
        assert_eq!(bit_scan(0x02), 1);
        assert_eq!(bit_scan(0x04), 2);
        assert_eq!(bit_scan(0x08), 3);
        assert_eq!(bit_scan(0x00), 8);
    }

    /// SamusStand 转移核（ActionTable：右/左→Run(1)、下→Roll(3)、上→PntUp(4)）。
    #[test]
    fn samus_stand_action_table_transitions() {
        use crate::ram_map::{JOY1_STATUS, SAMUS_STATUS};
        for (joy, want) in [(0x01u8, 1u8), (0x02, 1), (0x04, 3), (0x08, 4)] {
            let mut m = FrameMemory::default();
            m.write_ram(JOY1_STATUS, joy);
            let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| samus_stand(&mut m)));
            assert!(r.is_ok(), "stand 转移链应全通（r26）: {:?}", r.err()); // SetSamusData 已端口
            assert_eq!(m.read_ram(SAMUS_STATUS), want, "ActionTable[{joy:#x}]");
        }
    }

    /// SamusRun 跳跃动画状态机（字节自证阈值面：$18/$20+anim 常量）。
    #[test]
    fn samus_run_jump_anim_state_machine() {
        use crate::ram_map::{JOY1_STATUS, SAMUS_JUMP_DISP};
        // 下落（speedY_i=0）+位移 0x10 → anim=$0C
        let mut m = FrameMemory::default();
        m.write_ram(crate::ram_map::SAMUS_ACCEL_Y, 1);
        m.write_ram(SAMUS_JUMP_DISP, 0x10);
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| samus_run(&mut m)));
        assert!(r.is_err());
        assert_eq!(m.read_ram(ANIM_RESET_INDEX), 0x0C);
        // 上升（speedY_i=0x80）+位移 0x28+UP → anim=$35
        let mut m = FrameMemory::default();
        m.write_ram(crate::ram_map::SAMUS_ACCEL_Y, 1);
        m.write_ram(crate::samus::physics::SAMUS_SPEED_Y_I, 0x80);
        m.write_ram(SAMUS_JUMP_DISP, 0x28);
        m.write_ram(JOY1_STATUS, 0x88); // UP+A（A 防 StopVertMovement 爆）
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| samus_run(&mut m)));
        assert!(r.is_err()); // 尾段 CD02 桩
        assert_eq!(m.read_ram(ANIM_RESET_INDEX), 0x35);
    }

    /// SamusJump 位移门（≥$20+JUMP 松开→StopVertMovement 爆点）。
    #[test]
    fn samus_jump_release_gate() {
        use crate::ram_map::{JOY1_STATUS, SAMUS_JUMP_DISP};
        let mut m = FrameMemory::default();
        m.write_ram(crate::samus::physics::SAMUS_SPEED_Y_I, 0x80); // 上升
        m.write_ram(SAMUS_JUMP_DISP, 0x20); // ≥$20
        m.write_ram(JOY1_STATUS, 0); // JUMP 松开
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| samus_jump(&mut m)));
        let err = r.unwrap_err();
        let msg = err.downcast_ref::<String>().cloned()
            .or_else(|| err.downcast_ref::<&'static str>().map(|s| (*s).to_string()))
            .unwrap_or_default();
        assert!(msg.contains("unported @ bank07:D015"), "松开后链爆点失准: {msg}"); // D147 已折→D015
    }

    /// SamusRoll 球形解除（radiusY+=8 可观测写面）。
    #[test]
    fn samus_roll_breakout_writes_radius() {
        use crate::ram_map::{JOY1_CHANGE, JOY1_STATUS, SAMUS_RADIUS_Y};
        let mut m = FrameMemory::default();
        m.write_ram(JOY1_CHANGE, 0x08); // UP 开门
        m.write_ram(JOY1_STATUS, 0); // DOWN 未按
        m.write_ram(SAMUS_RADIUS_Y, 0x10);
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| samus_roll(&mut m)));
        let err = r.unwrap_err();
        let msg = err.downcast_ref::<String>().cloned()
            .or_else(|| err.downcast_ref::<&'static str>().map(|s| (*s).to_string()))
            .unwrap_or_default();
        assert!(msg.contains("unported @ bank07:E796"), "站立检爆点失准: {msg}");
        assert_eq!(m.read_ram(SAMUS_RADIUS_Y), 0x18); // +8
    }

    /// PntUp 松开回 Stand+方向转移（表 3B 界互核）。
    #[test]
    fn samus_pnt_up_release_and_direction() {
        use crate::ram_map::{JOY1_STATUS, SAMUS_STATUS};
        let mut m = FrameMemory::default();
        m.write_ram(JOY1_STATUS, 0x04); // DOWN（UP 松开）
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| samus_pnt_up(&mut m)));
        assert!(r.is_ok(), "pnt_up 链应全通（r26）: {:?}", r.err()); // SetSamusData 已端口
        assert_eq!(m.read_ram(SAMUS_STATUS), 3); // 表[2]=Roll
    }

    /// Dead2 微件（dec animDelay 全可端）。
    #[test]
    fn samus_dead2_decrements_anim_delay() {
        let mut m = FrameMemory::default();
        m.write_ram(0x0304, 5);
        samus_dead2(&mut m);
        assert_eq!(m.read_ram(0x0304), 4);
        m.write_ram(0x0304, 0);
        samus_dead2(&mut m);
        assert_eq!(m.read_ram(0x0304), 0xFF); // dec wrap
    }

    /// Door 状态机（入门相短路+延迟倒数面）。
    #[test]
    fn samus_door_state_machine_face() {
        use crate::ram_map::{DOOR_DELAY, DOOR_ENTRY_STATUS};
        let mut m = FrameMemory::default();
        m.write_ram(DOOR_ENTRY_STATUS, 2); // 入门相（<$05）→短路
        samus_door(&mut m);
        m.write_ram(DOOR_ENTRY_STATUS, 5); // 移动相
        m.write_ram(DOOR_DELAY, 3);
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| samus_door(&mut m)));
        let err = r.unwrap_err();
        let msg = err.downcast_ref::<String>().cloned()
            .or_else(|| err.downcast_ref::<&'static str>().map(|s| (*s).to_string()))
            .unwrap_or_default();
        assert!(msg.contains("unported @ bank07:D3AE"), "MoveOutDoor 爆点失准: {msg}");
        assert_eq!(m.read_ram(DOOR_DELAY), 2); // 倒数先行
    }

    /// UpdateSamus 面：PageIndex/IsSamus 落态+**站立径零爆全通**（r26 链贯通！）。
    #[test]
    fn update_samus_spine_face() {
        let mut m = FrameMemory::default();
        m.write_ram(SAMUS_STATUS, 0x80); // 站立旗
        update_samus(&mut m); // 全链（stand→physics→显示会师→蜂鸣）——零爆完成！
        assert_eq!(m.read_ram(crate::ram_map::PAGE_INDEX), 0, "PageIndex 归零");
        assert_eq!(m.read_ram(crate::ram_map::IS_SAMUS), 0, "IsSamus 复位");
    }
}

// ==== 10 态肉·批1（轻仪式——链规 20）====

/// BUTTON 位面（hardware.asm 内容锚：%00000001 RIGHT…%10000000 A）。
pub mod button {
    pub const RIGHT: u8 = 0x01;
    pub const LEFT: u8 = 0x02;
    pub const DOWN: u8 = 0x04;
    pub const UP: u8 = 0x08;
    pub const START: u8 = 0x10;
    pub const SELECT: u8 = 0x20;
    pub const B: u8 = 0x40;
    pub const A: u8 = 0x80;
}

/// ActionTable（asm :2804 内容锚 `ActionTable: ; 07:CC87`——ROM 自证 `01 01 03 04`）：
/// 索引=BitScan 位号（0=右/1=左/2=下/3=上）→ 转移目标态（sa_Run/sa_Run/sa_Roll/
/// sa_PntUp）。
pub use game_data::samus::samus_data::ACTION_TABLE;

/// SetSamusStand_NoFootstep 折叠（**字节自证**——asm :CF5D/`NoHorzMoveNoDelay: ;
/// 07:CF81`；名址映射以 ROM 字节为准：`20 81 cf 8c 00 03 a5 14 29 08 d0 0e a9 07 8d 05 03
/// 8d 06 03 a9 00 8d 04 03 60`——jsr CF81/sty $0300/lda $14/and #$08/bne/lda #$07/
/// sta $0305/sta $0306/lda #$00/sta $0304/rts）。
///
/// **结构互核注**：asm 标签（animResetIndex/animIndex/animDelay）↔字节址（$0305/$0306/
/// $0304）映射=**本码字节自证**（名址一致性随 task-18 残余考证）；CF81=jsr CFB7+sty $0304
/// （Y 写面）；**BUG 注保真**（fire 动画单帧——D4 保留）。
pub fn set_samus_stand_no_footstep(mem: &mut FrameMemory) {
    let y = 0u8; // 链内 Y 终值=0（CF4C `ldy #$00` 先行——字节自证）
    // CF4C：speedX_i/speedX_f←0（`8c 09 03 8c 13 03`）
    mem.write_ram(0x0309, 0);
    mem.write_ram(0x0313, 0);
    // CFB7：accelX←Y（`8c 15 03`）
    mem.write_ram(0x0315, y);
    // CF81：$0304←Y（`8c 04 03`）
    mem.write_ram(0x0304, y);
    // CF5D：status←Y（`8c 00 03`——stand=0）
    mem.write_ram(crate::ram_map::SAMUS_STATUS, y);
    if mem.read_ram(crate::ram_map::JOY1_STATUS) & button::UP != 0 {
        // SetSamusPntUp（CF77：`a9 04 8d 00 03 a9 27 20 6b cf`）
        mem.write_ram(crate::ram_map::SAMUS_STATUS, 4);
        mem.write_ram(0x0305, 0x27);
        mem.write_ram(0x0306, 0x27);
    } else {
        // 站立动画（`a9 07 8d 05 03 8d 06 03`）
        mem.write_ram(0x0305, 0x07);
        mem.write_ram(0x0306, 0x07);
    }
    mem.write_ram(0x0304, 0); // SetSamusNextAnim 尾（`a9 00 8d 04 03`）
}

/// BitScan（asm 内容锚 `BitScan: ; 07:E1E1`）：A 首个置位 bit 号（8=无置位）。
pub fn bit_scan(mut a: u8) -> u8 {
    for i in 0..8u8 {
        if a & 1 != 0 {
            return i;
        }
        a >>= 1;
    }
    8
}

/// SamusStand 处理件（**asm 内容锚+结构互核注**——10 态肉批1）：
/// `SamusStand: ; 07:CC36`——输入面（Joy1Status mask SELECT|START）→[BUG 注保真：
/// fire 动画单帧 bug（D4 保留）]→方向面（DOWN|LEFT|RIGHT 或 Joy1Change UP）→
/// BitScan→**ActionTable 转移核**（R-mv-1 状态转移面）→FIRE（Joy1Change|Retrig）
/// →JUMP（sa_Jump）→SetSamusData（CD6D）。
/// 嵌套件桩：SetSamusStand_NoFootstep（CF5D）/FireSamusProjectile（D1EE）/
/// SetSamusData（CD6D）——随队列填。
pub fn samus_stand(mem: &mut FrameMemory) {
    use button::*;
    let joy = mem.read_ram(crate::ram_map::JOY1_STATUS);
    if joy & !(SELECT | START) != 0 {
        set_samus_stand_no_footstep(mem); // CF5D（BUG 注保真面——已端口）
    }
    let mut input = joy & (DOWN | LEFT | RIGHT);
    if input == 0 {
        if mem.read_ram(crate::ram_map::JOY1_CHANGE) & UP != 0 {
            input = UP; // 上帧 UP 按下（Joy1Change 面）
        }
    }
    if input != 0 {
        let bit = bit_scan(input);
        if bit < 4 {
            if bit < 2 {
                mem.write_ram(crate::ram_map::SAMUS_DIR, bit); // 1=left, 0=right
            }
            // 转移核：ActionTable[bit] → status（R-mv-1）
            mem.write_ram(crate::ram_map::SAMUS_STATUS, ACTION_TABLE[bit as usize]);
        }
    }
    // FIRE（Joy1Change|Joy1Retrig 高位）→ FireSamusProjectile
    let fire = mem.read_ram(crate::ram_map::JOY1_CHANGE) | mem.read_ram(crate::memmap::title_state::JOY1_RETRIG);
    if fire & A != 0 {
        crate::unported!("bank07:D1EE"); // FireSamusProjectile
    }
    // JUMP（Joy1Change bit7）→ sa_Jump
    if mem.read_ram(crate::ram_map::JOY1_CHANGE) & A != 0 {
        mem.write_ram(crate::ram_map::SAMUS_STATUS, 2); // sa_Jump
    }
    super::physics::set_samus_data(mem); // CD6D（壳已端口）
}

// ==== 10 态肉·批3（SamusJump/SamusRoll——同形轻仪式）====

/// SamusJump 处理件（asm 内容锚 `SamusJump: ; 07:D002`——跳跃物理+输入面）：
/// 位移/速向门（JumpDsplcmnt≥$20+JUMP 松开→StopVertMovement）→水平检（嵌套件）→
/// UP→JumpPntUp 动画+sa_PntJump(6) 状态→Fire 检→岩浆跳（SetSamusJump+3 帧延迟）→
/// 触地检（AccelY）。嵌套件桩：SamusJump_CheckHorzMovement/LCF2E/
/// SamusJump_CheckFire/SetSamusJump/SetSamusData_3FrameAnimDelay。
pub fn samus_jump(mem: &mut FrameMemory) {
    // **高跳语义**（asm Lx017-Lx018：`ldy #$18/lda SamusGear/and #gr_HIGHJUMP/beq/ldy #$12`
    // →`sty SamusAccelY`——**重力常数**：$18=普通/$12=高跳（低重力=高跳！decomp 疑②证实）。
    let gear = mem.read_ram(crate::ram_map::SAMUS_GEAR);
    let gravity = if gear & crate::ram_map::gear::HIGHJUMP != 0 { 0x12 } else { 0x18 };
    mem.write_ram(crate::ram_map::SAMUS_ACCEL_Y, gravity);

    use button::*;
    let disp = mem.read_ram(crate::ram_map::SAMUS_JUMP_DISP);
    let speed_y_i = mem.read_ram(crate::samus::physics::SAMUS_SPEED_Y_I);
    if speed_y_i & 0x80 != 0 && disp >= 0x20 {
        if mem.read_ram(crate::ram_map::JOY1_STATUS) & A == 0 {
            super::physics::stop_vert_movement(mem); // D147（已全折）（松开起跳）
        }
    }
    crate::unported!("bank07:D015"); // SamusJump_CheckHorzMovement 族（水平检链）
}

/// SamusRoll 处理件（asm 内容锚 `SamusRoll: ; 07:D0E1`——球形解除面）：
/// UP/JUMP 开门+DOWN 闭门→**radiusY+=8**（$0301 写面——可观测）→ObjectCheckMoveUp
/// 站立可行检→位移 11px→SetSamusStand。嵌套件桩：ObjectCheckMoveUp（E7AD 族）/
/// ApplySpeedToPosition/SetSamusStand。
pub fn samus_roll(mem: &mut FrameMemory) {
    use button::*;
    let change = mem.read_ram(crate::ram_map::JOY1_CHANGE);
    let open_gate = (change & UP != 0) || (change & A != 0); // UP 或 JUMP
    if open_gate && mem.read_ram(crate::ram_map::JOY1_STATUS) & DOWN == 0 {
        // 球形解除：radiusY+=8（字节自证 `clc/adc #$08/sta Samus.radiusY`）
        let ry = mem.read_ram(crate::ram_map::SAMUS_RADIUS_Y).wrapping_add(8);
        mem.write_ram(crate::ram_map::SAMUS_RADIUS_Y, ry);
        crate::unported!("bank07:E796"); // ObjectCheckMoveUp（站立可行检）
    }
    crate::unported!("bank07:D0E1-tail"); // 滚动保持面（尾段）
}

// ==== 10 态肉·批4（PntUp/Dead/Dead2/Door/Elevator——轻仪式收尾）====

/// PntUp 状态表（字节自证 D1EB 3B `01 01 03`——x=BitScan(0..2)=右/左/下→Run/Run/Roll；
/// 第 4 字节 `a5`=FireSamusProjectile 首码（表界互核：表长 3=D1EE 处 jsr 目标））。
pub use game_data::samus::samus_data::PNT_UP_STATUS_TABLE;

/// SamusPntUp 处理件（字节自证 D198 区）：UP 保持/松开→sa_Stand(0)；DOWN|LEFT|RIGHT
/// →BitScan→SamusDir（x<2）+表转移；FIRE→FireSamusProjectile 桩；JUMP→sa_PntJump(6)
/// →SetSamusData 桩+JumpEngine 尾。
pub fn samus_pnt_up(mem: &mut FrameMemory) {
    use button::*;
    let joy = mem.read_ram(crate::ram_map::JOY1_STATUS);
    if joy & UP == 0 {
        mem.write_ram(crate::ram_map::SAMUS_STATUS, 0); // 松开→Stand
    }
    let dirs = joy & (DOWN | LEFT | RIGHT);
    if dirs != 0 {
        let bit = bit_scan(dirs) as usize;
        if bit < 2 {
            mem.write_ram(crate::ram_map::SAMUS_DIR, bit as u8);
        }
        if bit < 3 {
            mem.write_ram(crate::ram_map::SAMUS_STATUS, PNT_UP_STATUS_TABLE[bit]);
        }
    }
    let fire = mem.read_ram(crate::ram_map::JOY1_CHANGE) | mem.read_ram(crate::memmap::title_state::JOY1_RETRIG);
    if fire & A != 0 {
        crate::unported!("bank07:D1EE"); // FireSamusProjectile（上射）
    }
    if mem.read_ram(crate::ram_map::JOY1_CHANGE) & A != 0 {
        mem.write_ram(crate::ram_map::SAMUS_STATUS, 6); // sa_PntJump
    }
    super::physics::set_samus_data(mem); // CD6D（壳已端口）
}

/// SamusDead 处理件（字节自证 D41A `a9 01 4c 6d cd`——lda #$01/jmp SetSamusData）：
/// 可观测=SetSamusData 效果（动画面）——桩。
pub fn samus_dead(mem: &mut FrameMemory) {
    mem.regs.a = 0x01;
    mem.regs.a = 0x01; super::physics::set_samus_data(mem); // CD6D（A=1 参数面）
}

/// SamusDead2 处理件（字节自证 D41F `ce 05 03 60`——dec Samus.animDelay/rts）：
/// **全可端微件**（单写面）。
pub fn samus_dead2(mem: &mut FrameMemory) {
    let d = mem.read_ram(ANIM_RESET_INDEX.wrapping_sub(1)).wrapping_sub(1); // animDelay=$0304
    mem.write_ram(0x0304, d);
}

/// SamusDoor 处理件（字节自证 D3A8 区——门滚动态状态机）：
/// `lda DoorEntryStatus($56)/cmp #$05/bcc`（<$55 入门相）→`dec DoorDelay($59)/bne
/// MoveOutDoor`（延迟倒数）→完成相（`asl/bcc`→`lsr/sta DoorEntryStatus`）→敌清链
/// （Door_DeleteOffscreenEnemies/Doors_RemoveIfOffScreen/GotoClearAllMetroidLatches）
/// →音乐面（ItemRoomMusicStatus/StartMusic）。深链件=桩（随件填）。
pub fn samus_door(mem: &mut FrameMemory) {
    let status = mem.read_ram(crate::ram_map::DOOR_ENTRY_STATUS);
    if status < 0x05 {
        return; // 入门相（Lx055 路——ScrollDoor 族另面）
    }
    let delay = mem.read_ram(crate::ram_map::DOOR_DELAY).wrapping_sub(1);
    mem.write_ram(crate::ram_map::DOOR_DELAY, delay);
    if delay != 0 {
        crate::unported!("bank07:D3AE"); // MoveOutDoor（移动面——随件）
    }
    crate::unported!("bank07:D3BA"); // 完成相敌清链（随件）
}

/// SamusElevator 处理件（字节自证 D423 区——电梯态）：
/// `lda Elevator.status/cmp #$03/cmp #$08`（态门）→ElevatorType 方向（bit7=up）→
/// 屏幕 Y 检（Samus.y-ScrollY≥$84→ScrollDown）→wrap 检（SCRN_VY-1）。深链件=桩。
pub fn samus_elevator(mem: &mut FrameMemory) {
    let _ = mem;
    crate::unported!("bank07:D42E"); // 方向/滚动链（随件）
}

// ===== 1:1 Rust 改写·批产·块 6（handler 面去地址名（TDD 穷举等价证明））=====

#[cfg(test)]
mod tests_rewrite_b6_handler {
    //! TDD step1（测试先行（强制完整））——穷举等价。refs 锚（engine/mod.rs 注内 bank:addr）：
    //! $F284（:603（$6AF4 cmp4 回卷+$010F 写族））/$F2E8（:622（$010F←$80 负径））/
    //! $F300（:627（指针载 $6E/$6F））。

    use super::*;
    use crate::engine::{check_samus_collision, set_enemy_touching_samus_flags, check_enemy_hit_flags};

    /// 【穷举·门写】x×v6af4 全域——gate_motion_state_write==check_samus_collision（cmp4 回卷边界）。
    #[test]
    fn exhaustive_gate_write_equivalence() {
        for x in 0u8..=255 {
            for v in 0u8..=255 {
                assert_eq!(check_samus_collision(x, v), gate_motion_state_write(x, v));
            }
        }
    }

    /// 【穷举·负径写】set_negative_motion_flag==set_enemy_touching_samus_flags（退化域）。
    #[test]
    fn negative_flag_equivalence() {
        assert_eq!(set_enemy_touching_samus_flags(), set_negative_motion_flag());
    }

    /// 【穷举·指针载】tbl_lo×tbl_hi 全域——load_motion_pointer==check_enemy_hit_flags。
    #[test]
    fn exhaustive_pointer_load_equivalence() {
        for lo in 0u8..=255 {
            for hi in 0u8..=255 {
                assert_eq!(check_enemy_hit_flags(lo, hi), load_motion_pointer(lo, hi));
            }
        }
    }
}

// ===== 1:1 Rust 改写·批产·块 6：handler 面语义命名（去地址名（公式形（零字面特值）））=====
// refs 锚（engine/mod.rs 注内 bank:addr）：$F284（:603）/$F2E8（:622）/$F300（:627）。

/// 门写（旧 check_samus_collision（$F284）：$6AF4[x]>=4⇒回卷（空,4）；否则 $010F←0（Y=0））。
pub fn gate_motion_state_write(x: u8, v6af4_x: u8) -> (Vec<(u16, u8)>, u8) {
    let _ = x;
    if v6af4_x >= 4 {
        return (vec![], 4);
    }
    (vec![(0x010F, 0)], 0)
}

/// 负径写（旧 set_enemy_touching_samus_flags（$F2E8）：$010F←$80（负径（案未练面注）））。
pub fn set_negative_motion_flag() -> Vec<(u16, u8)> {
    vec![(0x010F, 0x80)]
}

/// 指针载（旧 check_enemy_hit_flags（$F300）：$6E←tbl_lo/$6F←tbl_hi（zp 对））。
pub fn load_motion_pointer(tbl_lo: u8, tbl_hi: u8) -> Vec<(u16, u8)> {
    vec![(0x006E, tbl_lo), (0x006F, tbl_hi)]
}
