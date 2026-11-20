//! Rio —— `refs/m1disasm/SRC/enemies/rio.asm`（54 行 / 26 条指令，逐行对读）。
//!
//! asm 原文（bank1 `01:9AF5`；**只有 Brinstar include 本件**——
//! `refs/m1disasm/SRC/prg1_brinstar.asm:905 .include "enemies/rio.asm"`）：
//! ```text
//! RioAIRoutine_{AREA}:
//!     lda EnemyStatusPreAI / cmp #enemyStatus_Resting / beq RioExit_Resting
//!     cmp #enemyStatus_Explode / beq RioExit_Explode
//!     lda #$80 / sta EnsExtra.0.accelY,x        ; 重力朝天花板
//!     lda Ens.0.speedY,x / bmi RioExit_Active    ; 上升中⇒不再判 Samus
//!     lda Ens.0.data05,x / and #$10 / beq RioExit_Active
//!     lda Ens.0.y,x / sec / sbc Samus.y / bpl @endIf_A
//!         jsr TwosComplement_
//!  @endIf_A:
//!     cmp #$10 / bcs RioExit_Active              ; |Δy|≥$10 ⇒ 继续掉
//!     lda #$00 / sta EnsExtra.0.accelY,x          ; 垂直对齐⇒停止重力
//! RioExit_Active:  lda #$03 / jmp CommonJump_UpdateEnemyCommon
//! RioExit_Explode: jmp CommonJump_UpdateEnemyCommon_noMoveNoAnim
//! RioExit_Resting: lda #$08 / jmp CommonJump_UpdateEnemyCommon_noMove
//! ```
//!
//! 三个出口对应本仓 `framework` 的三件：
//! `update_enemy_common_anim_move(anim=3)` / `update_enemy_common_no_move_no_anim` /
//! `update_enemy_common_no_move_anim(anim=8)`。
//!
//! 语义要点（易错处）：
//! - `accelY` 是**有符号**（`i8`）：`#$80` = **−128**（重力把 Rio 拉向天花板）。
//! - `bmi` 判的是 `speedY` 的 bit7（上升），**不是** delta——上升时跳过 data05/Samus 距离判。
//! - `TwosComplement_` 只在 `y - Samus.y` 为负时取补 ⇒ 比较的是 **|Δy|**；
//!   相等（Δ=0）也落在 `< $10` 支。

use crate::enemy::framework;
use crate::enemy::prep::ENS_F_STATUS;
use crate::enemy::tables::{en_addr, ens_addr};
use crate::ram_map;

/// `EnemyStatusPreAI`（`$0081`）的两个取值（asm `cmp #enemyStatus_Resting|Explode`）。
///
/// 读径说明（**复核 B 项更正**）：asm 读 `$0081`，而 `$0081` 是 `UpdateEnemy` 在
/// `JumpEngine` 分派前写下的**快照**（asm `prg7:10511 lda EnsExtra.0.status /
/// sta EnemyStatusPreAI`）。分派之后、AI 之前仍会改现役 status：
/// `try_becoming_active`（1→2）、`dec_status`（2→1）、`react_to_projectile`（→3/4/6），
/// 因此读现役 `status_raw()` 会在**激活帧 / Active→Resting 帧 / 中弹冻结帧**三类可达帧上
/// 与 asm 分叉（旧形即此错，`docs/rounds/r9-task-rio-review.md` B 项有直证）。
/// 现统一走 `mem.enemy_status_pre_ai()`（裸读收口在 `frame/memory.rs`，`raw-surface` 豁免辖）。
const PRE_RESTING: u8 = 1;
const PRE_EXPLODE: u8 = 3;

/// `RioAIRoutine`（`enemies/rio.asm`，仅 Brinstar 生成）。
pub(crate) fn rio_ai(mem: &mut crate::frame::Manager, x: u8) {
    let slot = x / 16;
    let pre = mem.enemy_status_pre_ai();
    if pre == PRE_RESTING {
        // RioExit_Resting：`lda #$08 / jmp CommonJump_UpdateEnemyCommon_noMove`
        framework::update_enemy_common_no_move_anim(mem, x, 0x08);
        return;
    }
    if pre == PRE_EXPLODE {
        // RioExit_Explode：`jmp CommonJump_UpdateEnemyCommon_noMoveNoAnim`
        framework::update_enemy_common_no_move_no_anim(mem, x);
        return;
    }
    // `lda #$80 / sta EnsExtra.0.accelY,x`——重力朝天花板（有符号 −128）。
    mem.enemy(slot).set_accel_y(-128);
    // `lda Ens.0.speedY,x / bmi RioExit_Active`：上升中 ⇒ 跳过后续判。
    if mem.enemy(slot).speed_y() >= 0 {
        // `lda Ens.0.data05,x / and #$10 / beq RioExit_Active`
        if mem.enemy(slot).data05() & 0x10 != 0 {
            // `lda Ens.0.y,x / sec / sbc Samus.y / bpl / TwosComplement_`
            let dy = mem.enemy(slot).y().wrapping_sub(mem.samus().y());
            let ady = if dy & 0x80 != 0 { dy.wrapping_neg() } else { dy };
            // `cmp #$10 / bcs RioExit_Active`
            if ady < 0x10 {
                // `lda #$00 / sta EnsExtra.0.accelY,x`——垂直对齐，停止重力。
                mem.enemy(slot).set_accel_y(0);
            }
        }
    }
    // RioExit_Active：`lda #$03 / jmp CommonJump_UpdateEnemyCommon`
    framework::update_enemy_common_anim_move(mem, x, 0x03);
}

#[cfg(test)]
mod tests_rio {
    use super::*;
    use crate::enemy::prep::{EN_F_DATA05, ENS_F_TYPE};
    use crate::frame::Manager;

    const X: u8 = 0x20;

    fn stage(m: &mut Manager, pre: u8, speed_y: i8, data05: u8, ey: u8, sy: u8) {
        game_data::pack::init_from_env().ok();
        m.write_ram(ram_map::IN_AREA, 0); // Brinstar
        m.write_wram(ens_addr(X, ENS_F_STATUS), pre);
        // AI 读的是分派前快照 `$0081`（asm `lda EnemyStatusPreAI`）——同步写入，
        // 其余测试与 asm 语义一致。
        m.write_ram(ram_map::ENEMY_STATUS_PRE_AI, pre);
        m.write_wram(ens_addr(X, ENS_F_TYPE), 6); // Rio
        m.write_ram(en_addr(X, 0), ey);
        m.write_ram(en_addr(X, 1), 0x50);
        m.write_ram(en_addr(X, EN_F_DATA05), data05);
        m.enemy(X / 16).set_speed_y(speed_y);
        m.samus().set_y(sy);
    }

    /// `lda #$80 / sta accelY` 恒执行（非 Resting/Explode 径）——且是**有符号 −128**。
    #[test]
    fn active_path_sets_ceiling_gravity() {
        let mut m = Manager::default();
        // speedY 上升（负）⇒ 直接走 Active 出口，accelY 保持 $80
        stage(&mut m, 0x02, -4, 0x00, 0x80, 0x80);
        rio_ai(&mut m, X);
        assert_eq!(m.enemy(X / 16).accel_y(), -128, "accelY 应为 $80(−128)");
    }

    /// `bmi` 判上升：speedY<0 时即使 data05 bit4 置、Samus 垂直对齐，
    /// 也不把 accelY 清 0（asm `bmi RioExit_Active` 在 data05 判之前）。
    #[test]
    fn rising_skips_samus_alignment_check() {
        let mut m = Manager::default();
        stage(&mut m, 0x02, -1, 0x10, 0x80, 0x80); // 垂直对齐（Δ=0）
        rio_ai(&mut m, X);
        assert_eq!(m.enemy(X / 16).accel_y(), -128, "上升中不得清重力");
    }

    /// 下落 + data05 bit4 置 + |Δy|<$10 ⇒ `accelY ← 0`（垂直对齐，停止重力）。
    #[test]
    fn falling_aligned_clears_gravity() {
        let mut m = Manager::default();
        stage(&mut m, 0x02, 1, 0x10, 0x80, 0x88); // Δ = −8（取补后 8 < $10）
        rio_ai(&mut m, X);
        assert_eq!(m.enemy(X / 16).accel_y(), 0, "|Δy|<$10 应清重力");
    }

    /// 下落 + bit4 置但 |Δy|≥$10 ⇒ 不清重力（`cmp #$10 / bcs`）。
    #[test]
    fn falling_far_keeps_gravity() {
        let mut m = Manager::default();
        stage(&mut m, 0x02, 1, 0x10, 0x80, 0x90); // Δ = −$10 ⇒ 恰等于阈值 ⇒ 保留
        rio_ai(&mut m, X);
        assert_eq!(m.enemy(X / 16).accel_y(), -128);
    }

    /// data05 bit4 未置 ⇒ 不判 Samus 距离（重力保持）。
    #[test]
    fn falling_without_bit4_keeps_gravity() {
        let mut m = Manager::default();
        stage(&mut m, 0x02, 1, 0x00, 0x80, 0x80);
        rio_ai(&mut m, X);
        assert_eq!(m.enemy(X / 16).accel_y(), -128);
    }

    /// Resting/Explode 两出口：都不写 accelY（asm 提前 return），且状态不变。
    #[test]
    fn resting_and_explode_exit_without_gravity_write() {
        for pre in [0x01u8, 0x03] {
            let mut m = Manager::default();
            stage(&mut m, pre, 1, 0x10, 0x80, 0x80);
            m.enemy(X / 16).set_accel_y(7);
            rio_ai(&mut m, X);
            assert_eq!(m.enemy(X / 16).accel_y(), 7, "pre={pre} 不应改 accelY");
        }
    }

    /// **复核 B 项回归测**：AI 必须读 `$0081` 快照而非现役 status。
    ///
    /// 激活帧形态（`try_becoming_active` 把现役 status 1→2 之后才调 AI，而 asm 的
    /// `$0081` 仍是 1）⇒ asm 走 Resting 出口（anim8 + noMove，**不写 accelY**）。
    /// 旧形读现役 status 会走 Active 径写 −128（复核直证 `left: -128, right: 7`）。
    #[test]
    fn ai_reads_pre_ai_snapshot_not_live_status() {
        let mut m = Manager::default();
        stage(&mut m, 0x01, 1, 0x10, 0x80, 0x80);
        // 现役 status 已被框架改成 Active(2)，但分派前快照仍是 Resting(1)。
        m.write_wram(ens_addr(X, ENS_F_STATUS), 0x02);
        m.enemy(X / 16).set_accel_y(7);
        rio_ai(&mut m, X);
        assert_eq!(
            m.enemy(X / 16).accel_y(),
            7,
            "pre=$0081 为 Resting 时不得写重力（读快照，不读现役 status）"
        );
        // 反向对照：快照为 Active ⇒ 写 −128。
        let mut m2 = Manager::default();
        // data05 bit4=0 ⇒ 不判 Samus 距离，重力保持（对照只验证"快照 Active 会写重力"）。
        stage(&mut m2, 0x02, 1, 0x00, 0x80, 0x80);
        m2.enemy(X / 16).set_accel_y(7);
        rio_ai(&mut m2, X);
        assert_eq!(m2.enemy(X / 16).accel_y(), -128, "快照 Active ⇒ 写重力");
    }

    /// dispatch 级接线：type 6（Brinstar Rio）走真实链 `update_all_enemies`。
    #[test]
    fn dispatch_wiring_reaches_rio_for_type_6() {
        let mut m = Manager::default();
        game_data::pack::init_from_env().ok();
        m.write_ram(ram_map::IN_AREA, 0);
        m.write_wram(ens_addr(X, ENS_F_STATUS), 0x02);
        m.write_wram(ens_addr(X, ENS_F_TYPE), 6);
        m.write_ram(en_addr(X, 0), 0x40);
        m.write_ram(en_addr(X, 1), 0x50);
        m.write_ram(en_addr(X, EN_F_DATA05), 0x02);
        m.enemy(X / 16).set_accel_y(0);
        m.enemy(X / 16).set_speed_y(1);
        crate::enemy::slot_cycle::update_all_enemies(&mut m);
        assert_eq!(m.enemy(X / 16).accel_y(), -128, "type=6 未进 Rio");
    }
}
