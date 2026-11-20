//! Swooper/Geruta（Norfair 变体）—— `refs/m1disasm/SRC/enemies/swooper.asm`
//! （90 行 / 76 条指令，逐行对读）。asm 原文（bank2 `02:9694` 起；本件被
//! prg2_norfair.asm:618 **与** prg5_ridley.asm:623 include——`BANK` 条件编译出
//! Geruta（bank2）与 Holtz（bank5）两套动画字面量。**本卡只接 Norfair**（ai_table
//! area=1 的 00/01 两项）；Ridley 的接线留 Unported 记账（另一卡）。
//!
//! asm 原文（bank2 实例化，`02:9694` 起）：
//! ```text
//! SwooperAIRoutine00:
//!     lda #$03 / sta $00        ; 活动帧 anim 节拍 3
//!     lda #$08 / sta $01        ; 休息帧 anim 节拍 8
//!     lda status / cmp #Resting / bne @draw
//!     lda data05 / and #$10 / beq @draw
//!         lda #$01 / jsr SwooperChangeEnemyType   ; 未见萨→扑萨形态
//! @draw: jsr UpdateSwooperAnim / jmp UpdateEnemyCommon_Decide
//!
//! UpdateSwooperAnim:
//!     lda status / cmp #Active / bne @RTS
//!     lda #EnAnim_GerutaSwooping - EnAnimTable   ; 默认扑下 0x25
//!     ldy speedY / bpl @endIf_A
//!         lda #EnAnim_GerutaIdle - EnAnimTable   ; 上升⇒悬停 0x22
//! @endIf_A: sta resetAnimIndex / @RTS: rts
//!
//! SwooperAIRoutine01:
//!     lda PreAI / cmp #Resting / beq Exit_Resting
//!     cmp #Explode / beq Exit_Explode
//!     lda status / cmp #Resting / bne Swoop
//!         lda #$00 / jsr SwooperChangeEnemyType     ; 换回坠落形态（直落）
//! Exit_Resting: lda #$08 / jmp UpdateEnemyCommon_noMove   ; ←换型后**落入**此出口
//! Swoop:
//!     lda #$80 / sta accelY                          ; 朝天花板重力（−128）
//!     lda speedY / bmi @endIf_A                      ; 上升中⇒不判
//!     lda data05 / and #$10 / beq @endIf_A
//!     lda y / sec / sbc Samus.y / bpl @endIf_B
//!         jsr TwosComplement_
//! @endIf_B: cmp #$10 / bcs @endIf_A                  ; |Δy|≥$10⇒不贴
//!         lda #$00 / sta accelY                      ; 垂直对齐⇒停重力
//! @endIf_A: jsr UpdateSwooperAnim / lda #$03 / jmp UpdateEnemyCommon
//! Exit_Explode: jmp UpdateEnemyCommon_noMoveNoAnim
//!
//! SwooperChangeEnemyType:
//!     sta EnsExtra.0.type / lda health / pha
//!     jsr CommonJump_0E                    ; 07:EB6E → FB7B + F85A
//!     pla / sta health / rts
//! ```
//!
//! `CommonJump_0E = LEB6E`（`prg7_engine.asm:8952`）逐指令净效果：
//! - `asl data05`（b7→C）→ `jsr LoadTableAt977B`（不触旗标）→ `ror data05`
//!   （C→b7，b0→C 出）。**净效果 = data05 bit0 清 0、其余位不变、carry=0**。
//! - `InitEnemyFacingDirectionAxisAndDelay ; 07:FB7B`：`delay ← EnemyInitDelayTbl[type]`
//!   （type=**新**型，`ldy` 在 LEB6E 头部已取）。
//! - `InitEnemyForceSpeedTowardsSamusDelayAndHealth ; 07:F85A`：
//!   `data0D ← EnemyForceSpeedTowardsSamusDelayTbl[type]`；
//!   `health ← EnemyHealthTbl[type]`（specialAttribs bit7 ⇒ `asl` 翻倍，截 8 位）。
//! - 调用方 `pha/pla` 恢复原 health ⇒ **health 净不变**。

use crate::enemy::framework;
use crate::enemy::tables::load_table_at_977b;
use crate::frame::{cur_area, Manager};
use crate::ram_map;

/// `EnemyStatusPreAI`（`$0081`）/ status 取值（asm `cmp #enemyStatus_*`）。
const PRE_RESTING: u8 = 1;
const PRE_EXPLODE: u8 = 3;
const ENEMY_RESTING: u8 = 1;
const ENEMY_ACTIVE: u8 = 2;

/// `UpdateSwooperAnim` 的 anim 字面量（bank2：`#EnAnim_GerutaSwooping/Idle -
/// EnAnimTable_NORFAIR`，`02:9BFC`=Idle 0x22、`02:9BFF`=Swooping 0x25，表基
/// `02:9BDA`。代码即时量，非数据段）。
const ANIM_OFF_GERUTA_IDLE: u8 = 0x22;
const ANIM_OFF_GERUTA_SWOOPING: u8 = 0x25;

/// 零页 anim 节拍预设槽（`$00`=活动 / `$01`=休息，`UpdateEnemyCommon_Decide` 消费）。

/// `SwooperAIRoutine00`（悬停待机形态：未见萨时挂在天花板）。
pub(crate) fn swooper_ai_00(mem: &mut Manager, x: u8) {
    // `lda #$03 / sta $00 ; lda #$08 / sta $01`
    // `lda #$03 / sta $00 ; lda #$08 / sta $01`（即时量直传 Decide 变体）

    let slot = x / 16;
    // 现役 Resting 且 data05 bit4 置 ⇒ 换扑萨形态（type←1）。
    if mem.enemy(slot).status_raw() == ENEMY_RESTING && mem.enemy(slot).data05() & 0x10 != 0 {
        swooper_change_enemy_type(mem, x, 0x01);
    }
    // @draw：UpdateSwooperAnim + Decide（$0081 分流）。
    update_swooper_anim(mem, x);
    framework::update_enemy_common_decide_with_beats(mem, x, 0x03, 0x08);
}

/// `SwooperAIRoutine01`（扑萨形态：重力朝天花板 + 贴萨悬停）。
pub(crate) fn swooper_ai_01(mem: &mut Manager, x: u8) {
    let slot = x / 16;
    let pre = mem.enemy_status_pre_ai();
    // `cmp #Resting / beq Exit_Resting`
    if pre == PRE_RESTING {
        framework::update_enemy_common_no_move_anim(mem, x, 0x08);
        return;
    }
    // `cmp #Explode / beq Exit_Explode`
    if pre == PRE_EXPLODE {
        framework::update_enemy_common_no_move_no_anim(mem, x);
        return;
    }
    // **asm 读现役 status**（与 Rio 的 pre 快照径不同）：Active→Resting 帧换回直落型，
    // 然后**落入** Exit_Resting（noMove anim8）。
    if mem.enemy(slot).status_raw() == ENEMY_RESTING {
        swooper_change_enemy_type(mem, x, 0x00);
        framework::update_enemy_common_no_move_anim(mem, x, 0x08);
        return;
    }
    // Swoop 径：重力 −128。
    mem.enemy(slot).set_accel_y(-128);
    // `lda speedY / bmi @endIf_A`：上升中⇒不判贴萨。
    if mem.enemy(slot).speed_y() >= 0 {
        // `and #$10`
        if mem.enemy(slot).data05() & 0x10 != 0 {
            // `y − Samus.y`（负则取补）`cmp #$10 / bcs` ⇒ |Δy|<$10 才停重力。
            let dy = mem.enemy(slot).y().wrapping_sub(mem.samus().y());
            let ady = if dy & 0x80 != 0 { dy.wrapping_neg() } else { dy };
            if ady < 0x10 {
                mem.enemy(slot).set_accel_y(0);
            }
        }
    }
    update_swooper_anim(mem, x);
    // `lda #$03 / jmp CommonJump_UpdateEnemyCommon`
    framework::update_enemy_common_anim_move(mem, x, 0x03);
}

/// `UpdateSwooperAnim`（bank2 分支）：现役 Active 时按速度选 Geruta 动画。
fn update_swooper_anim(mem: &mut Manager, x: u8) {
    let slot = x / 16;
    if mem.enemy(slot).status_raw() != ENEMY_ACTIVE {
        return;
    }
    // 默认 Swooping；speedY<0（上升）⇒ Idle。
    let off = if mem.enemy(slot).speed_y() < 0 {
        ANIM_OFF_GERUTA_IDLE
    } else {
        ANIM_OFF_GERUTA_SWOOPING
    };
    mem.enemy(slot).set_reset_anim_index(off);
}

/// `SwooperChangeEnemyType`：type←t、暂存 health、`CommonJump_0E`（LEB6E）、还原 health。
fn swooper_change_enemy_type(mem: &mut Manager, x: u8, t: u8) {
    let slot = x / 16;
    mem.enemy(slot).set_enemy_type(t);
    // `pha`：暂存现 health（LEB6E 会重写，调用方 `pla` 还原 ⇒ 净不变）。
    let saved_health = mem.enemy(slot).health();

    // CommonJump_0E（07:EB6E）：
    // `asl data05` + `ror data05` 净效果＝bit0 清 0、其余不变（见模块注释逐指令推导）。
    let d5 = mem.enemy(slot).data05();
    mem.enemy(slot).set_data05(d5 & 0xFE);
    // `jsr LoadTableAt977B`（FB7B 头；A 值在本径不被消费，保留调用以对齐旗标无关的表读）。
    let _ = load_table_at_977b(mem, x);
    // `lda EnemyInitDelayTbl,y / sta delay`（y=新型）。
    let area = cur_area(mem);
    let delay = game_data::assets::enemy_init_delay(area)[t as usize];
    mem.enemy(slot).set_delay(delay);
    // InitEnemyForceSpeedTowardsSamusDelayAndHealth（07:F85A）：
    let fsd = game_data::assets::enemy_force_speed_delay(area)[t as usize];
    mem.enemy(slot).set_data0d(fsd);
    let mut hp = game_data::assets::enemy_health(area)[t as usize];
    if mem.enemy(slot).special_attribs() & 0x80 != 0 {
        hp = hp.wrapping_shl(1); // asm `asl`（截 8 位）
    }
    mem.enemy(slot).set_health(hp);
    // `pla / sta health`：还原。
    mem.enemy(slot).set_health(saved_health);
}

#[cfg(test)]
mod tests_swooper {
    use super::*;
    use crate::enemy::prep::{EN_F_DATA05, ENS_F_STATUS, ENS_F_TYPE};
    use crate::enemy::tables::{en_addr, ens_addr};
    use crate::ram_map::{ENEMY_STATUS_PRE_AI, IN_AREA};

    const X: u8 = 0x20;

    fn stage(m: &mut Manager, pre: u8, status: u8, data05: u8, speed_y: i8) {
        game_data::pack::init_from_env().ok();
        m.write_ram(IN_AREA, 1); // Norfair
        m.write_ram(ENEMY_STATUS_PRE_AI, pre);
        m.write_wram(ens_addr(X, ENS_F_STATUS), status);
        m.write_wram(ens_addr(X, ENS_F_TYPE), 0x00); // Swooper
        m.write_ram(en_addr(X, EN_F_DATA05), data05);
        m.enemy(X / 16).set_speed_y(speed_y);
        m.enemy(X / 16).set_health(0x10);
        m.enemy(X / 16).set_data0d(0x33);
        m.enemy(X / 16).set_delay(0x44);
        m.samus().set_y(0x80);
        m.enemy(X / 16).set_y(0x90);
    }

    /// 00 号 + Resting + bit4 置 ⇒ 换型 1：type 改、delay/data0D 按新型查表、
    /// health 被 pha/pla 还原（0x10）、data05 bit0 清 0 其余不变。
    #[test]
    fn routine00_resting_bit4_changes_type_to_1() {
        let mut m = Manager::default();
        stage(&mut m, ENEMY_ACTIVE, ENEMY_RESTING, 0x10 | 0x01, -2);
        swooper_ai_00(&mut m, X);
        assert_eq!(m.enemy(X / 16).enemy_type(), 1, "换扑萨型");
        assert_eq!(m.enemy(X / 16).health(), 0x10, "pha/pla 还原");
        assert_eq!(m.enemy(X / 16).data05() & 0x01, 0, "bit0 清 0");
        assert_eq!(m.enemy(X / 16).data05() & 0x10, 0x10, "其余位不变");
        assert_eq!(
            m.enemy(X / 16).delay(),
            game_data::assets::enemy_init_delay(1)[1],
            "delay ← EnemyInitDelayTbl[新型]"
        );
        assert_eq!(
            m.enemy(X / 16).data0d(),
            game_data::assets::enemy_force_speed_delay(1)[1],
            "data0D ← ForceSpeedDelayTbl[新型]"
        );
    }

    /// 00 号 + Resting 但 bit4 清 ⇒ 不换型。
    #[test]
    fn routine00_resting_without_bit4_keeps_type() {
        let mut m = Manager::default();
        stage(&mut m, ENEMY_ACTIVE, ENEMY_RESTING, 0x00, -2);
        swooper_ai_00(&mut m, X);
        assert_eq!(m.enemy(X / 16).enemy_type(), 0);
        assert_eq!(m.enemy(X / 16).delay(), 0x44, "不换型不改 delay");
    }

    /// UpdateSwooperAnim：现役 Active + speedY<0 ⇒ GerutaIdle 0x22。
    #[test]
    fn anim_ascending_picks_idle() {
        let mut m = Manager::default();
        stage(&mut m, ENEMY_ACTIVE, ENEMY_ACTIVE, 0x00, -1);
        swooper_ai_00(&mut m, X);
        assert_eq!(m.enemy(X / 16).reset_anim_index(), 0x22);
    }

    /// UpdateSwooperAnim：Active + speedY≥0 ⇒ GerutaSwooping 0x25；Resting ⇒ 不写。
    #[test]
    fn anim_descending_picks_swooping_resting_skips() {
        let mut m = Manager::default();
        stage(&mut m, ENEMY_ACTIVE, ENEMY_ACTIVE, 0x00, 1);
        swooper_ai_00(&mut m, X);
        assert_eq!(m.enemy(X / 16).reset_anim_index(), 0x25);

        let mut m2 = Manager::default();
        stage(&mut m2, ENEMY_ACTIVE, ENEMY_RESTING, 0x00, 1);
        swooper_ai_00(&mut m2, X);
        assert_eq!(m2.enemy(X / 16).reset_anim_index(), 0, "非 Active 不写 anim");
    }

    /// 01 号：pre==Resting ⇒ Exit_Resting（anim8 noMove），不换型不写 accelY。
    #[test]
    fn routine01_pre_resting_exit() {
        let mut m = Manager::default();
        stage(&mut m, PRE_RESTING, ENEMY_ACTIVE, 0x10, 3);
        m.enemy(X / 16).set_accel_y(5);
        swooper_ai_01(&mut m, X);
        assert_eq!(m.enemy(X / 16).accel_y(), 5, "提前出口不写重力");
        assert_eq!(m.enemy(X / 16).enemy_type(), 0);
    }

    /// 01 号：pre==Explode ⇒ Explode 出口（不写 accelY/resetAnimIndex）。
    #[test]
    fn routine01_pre_explode_exit() {
        let mut m = Manager::default();
        stage(&mut m, PRE_EXPLODE, ENEMY_ACTIVE, 0x10, 3);
        m.enemy(X / 16).set_accel_y(5);
        swooper_ai_01(&mut m, X);
        assert_eq!(m.enemy(X / 16).accel_y(), 5);
        assert_eq!(m.enemy(X / 16).reset_anim_index(), 0);
    }

    /// 01 号：现役 Resting（Active→Resting 帧）⇒ 换直落型 0 且**落入** Resting 出口。
    #[test]
    fn routine01_active_to_resting_changes_type() {
        let mut m = Manager::default();
        stage(&mut m, ENEMY_ACTIVE, ENEMY_RESTING, 0x10, 3);
        swooper_ai_01(&mut m, X);
        assert_eq!(m.enemy(X / 16).enemy_type(), 0, "换回直落型");
        assert_eq!(m.enemy(X / 16).accel_y(), 0, "换型径不写 −128");
    }

    /// 01 号 Swoop 径：下降 + bit4 置 + |Δy|<$10（Δ=8）⇒ 停重力。
    #[test]
    fn routine01_aligned_clears_gravity() {
        let mut m = Manager::default();
        stage(&mut m, ENEMY_ACTIVE, ENEMY_ACTIVE, 0x10, 1);
        m.enemy(X / 16).set_y(0x88); // Δ = 8 < $10
        swooper_ai_01(&mut m, X);
        assert_eq!(m.enemy(X / 16).accel_y(), 0);
        assert_eq!(m.enemy(X / 16).reset_anim_index(), 0x25);
    }

    /// 01 号：上升（speedY<0）时即使垂直对齐也保持 −128（`bmi` 在 data05 判前）。
    #[test]
    fn routine01_ascending_keeps_gravity() {
        let mut m = Manager::default();
        stage(&mut m, ENEMY_ACTIVE, ENEMY_ACTIVE, 0x10, -1);
        m.enemy(X / 16).set_y(0x80); // Δ=0
        swooper_ai_01(&mut m, X);
        assert_eq!(m.enemy(X / 16).accel_y(), -128);
        assert_eq!(m.enemy(X / 16).reset_anim_index(), 0x22);
    }

    /// 01 号：|Δy|≥$10 ⇒ 保持重力（`bcs` 含等）。
    #[test]
    fn routine01_far_keeps_gravity() {
        let mut m = Manager::default();
        stage(&mut m, ENEMY_ACTIVE, ENEMY_ACTIVE, 0x10, 1);
        m.enemy(X / 16).set_y(0x90); // Δ=0x10 ⇒ 不小于
        swooper_ai_01(&mut m, X);
        assert_eq!(m.enemy(X / 16).accel_y(), -128);
    }

    /// dispatch 级接线：type 0x00（Norfair Swooper00）走真实链。
    #[test]
    fn dispatch_wiring_reaches_swooper00_for_type_0() {
        let mut m = Manager::default();
        game_data::pack::init_from_env().ok();
        m.write_ram(IN_AREA, 1);
        m.write_wram(ens_addr(X, ENS_F_STATUS), 0x01);
        m.write_wram(ens_addr(X, ENS_F_TYPE), 0x00);
        m.write_ram(en_addr(X, 0), 0x40);
        m.write_ram(en_addr(X, 1), 0x50);
        m.write_ram(en_addr(X, EN_F_DATA05), 0x10);
        
        
        crate::enemy::slot_cycle::update_all_enemies(&mut m);
        let r = m.enemy(X / 16).reset_anim_index();
        assert!(r == 0x22 || r == 0x25, "经 choose_enemy_ai 到 00 号径（Geruta anim 写入，got {r:#04X}）");
    }
}
