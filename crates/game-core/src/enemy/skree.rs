//! Skree AI（`refs/m1disasm/SRC/enemies/skree.asm`；5 label）。

use crate::enemy::framework::*;
use crate::enemy::motion::*;
use crate::enemy::prep::*;
use crate::frame::tick::*;
use crate::ram_map;

/// SkreeAIRoutine（**asm 内容锚** `SkreeAIRoutine_{AREA}`（refs/m1disasm/SRC/enemies/
/// skree.asm:2-71——A 级逐行））：悬顶静息（PreAI=Resting⇒anim 8）→近距激活（阈距
/// data05 bit3——F75B 件）→坠落（movementInstrIndex($+7)<$0F——EnemyGetDeltaY delta 径）
/// →落地（≥$0F）⇒爆延时←$3A→≥$11⇒爆体径（jumpDsplcmnt($+13) 倒数⇒清槽+**4 弹**）。
pub(crate) fn skree_ai(mem: &mut crate::frame::Manager, x: u8) {
    let pre = mem.read_ram(0x0081); // EnemyStatusPreAI
    if pre == 1 {
        // SkreeExit_Resting（:62-66）：anim 8+noMove
        update_enemy_anim(mem, x, 8);
        update_enemy_common_no_move_no_anim(mem, x);
        return;
    }
    if pre == 3 {
        // SkreeExit_Explode（:68-70）：noMoveNoAnim
        update_enemy_common_no_move_no_anim(mem, x);
        return;
    }
    let instr = mem.read_ram(0x0400 + x as u16 + 6); // movementInstrIndex=+6（ROM BC 06 04）
    if instr >= 0x11 {
        // SkreeBlowUpIntoProjectiles（:27-54）：jumpDsplcmnt 倒数⇒0⇒清槽+4 弹
        let d = mem.read_wram(0x6AF4 + x as u16 + 13); // EnsExtra.jumpDsplcmnt=+13
        let nd = d.wrapping_sub(1);
        mem.write_wram(0x6AF4 + x as u16 + 13, nd);
        if nd == 0 {
            mem.write_wram(0x6AF4 + x as u16, 0); // status←NoEnemy（:33-35）
            // 4 弹生成（skree.asm:36-54 逐字——SkreeProjectiles=$A0-$AF（4 槽×4B））：
            // 每槽 dieDelay←$0A+y/x/hi←Skree 位（Y=12/8/4/0 递减环）
            let ey = mem.read_ram(0x0400 + x as u16 + 0);
            let ex = mem.read_ram(0x0400 + x as u16 + 1);
            let ehi = mem.read_wram(0x6AF4 + x as u16 + 7);
            for yy in [12u16, 8, 4, 0] {
                let base = 0x00A0u16 + yy;
                mem.write_ram(base, 0x0A); // dieDelay←10 帧
                mem.write_ram(base + 1, ey);
                mem.write_ram(base + 2, ex);
                mem.write_ram(base + 3, ehi);
            }
        }
        // 【2026-11-25 修（#49 f1787）】ROM 的 Explode 分支**没有 rts**：spawn 循环
        // `bpl @loop` 后直接落到 `SkreeExit_Active`（skree.asm:41-60），仍然
        // `lda #$02 / jmp UpdateEnemyCommon` ⇒ **EnemyMove 照跑**。旧形在这里换成
        // `no_move_no_anim`，于是落地后 `Ens.delay(+9)` 不再被 EnemyMove 推进：
        // emu f1787 起 delay=02/03/04…，rust 恒 01 —— TAS 100% 首分歧就在这。
        update_enemy_common_anim_move(mem, x, 2);
        return;
    }
    if instr >= 0x0F {
        // 刚落地（:21-25）：爆延时←$3A（约 1 秒）
        mem.write_wram(0x6AF4 + x as u16 + 13, 0x3A);
    }
    // SkreeExit_Active（skree.asm:56-60 逐字）：`lda #$02 / jmp CommonJump_UpdateEnemyCommon`
    // ⇒ **标准件** UpdateEnemyCommon（07:F410）= UpdateEnemyAnim(2) + EnemyMove + 尾件。
    // 【2026-11-16 修：case-002 f1122】旧形在此**内联复制**了一份 Δ 引擎 + 手写「y±|Δ| 一次写回」，
    // 绕过 EnemyMove 的逐像素 `EnemyMoveOnePixelDown`⇒`EnemyCheckMoveDown`（BG 碰撞）——
    // 后果=Skree 穿地 8px（一整块）：f1122 锈 y=198/金 196，f1124 锈 204 恒/金 196 恒。
    // 同族病=「同一 asm 例程两份不等价实现」（`docs/findings/per-bank-assets.md` §0 纪律）。
    // 触发休止（`EnemyGetDeltaY@fail`→`EnemyTriggerResting_AndClearEnAccelY $81B1`）已归位到
    // 唯一的 Δ 引擎 `motion.rs::enemy_delta_for_move`（asm 本体就在 EnemyGetDeltaY 内）。
    update_enemy_common_anim_move(mem, x, 2);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::enemy::state::EnemyStatus;

    fn pack() {
        if game_data::pack::init_from_env().is_err() {
            eprintln!("SKIP（显式）：未加载资产包——先跑 pack-tool extract");
            std::process::exit(0);
        }
    }

    /// Skree 落地后（`movementInstrIndex >= $11`）ROM **没有 rts**：spawn 循环落空后
    /// 直接落到 `SkreeExit_Active`，仍然 `lda #$02 / jmp UpdateEnemyCommon` ⇒ EnemyMove
    /// 照跑、`Ens.delay(+9)` 继续被推进。旧形在这里换成 no_move_no_anim ⇒ delay 恒 1，
    /// TAS 100% 首分歧 f1787 `$0429` emu=0x02 / rust=0x01 就是这条。
    #[test]
    fn landing_branch_still_runs_enemy_move_and_advances_delay() {
        pack();
        let mut m = crate::frame::Manager::default();
        crate::frame::set_enemy_status_pre_ai(&mut m, 0); // 非 Resting/Explode 出口
        m.enemy(0).set_status(EnemyStatus::Active);
        m.enemy(0).set_data05(0x00);
        m.enemy(0).set_data06(0x11); // movementInstrIndex >= $11 ⇒ 爆体径
        m.enemy(0).set_jump_dsplcmnt(10); // jumpDsplcmnt 非零 ⇒ 不清槽
        m.enemy(0).set_enemy_type(4); // Skree
        m.enemy(0).set_delay(0);

        skree_ai(&mut m, 0);

        assert_eq!(
            m.enemy(0).delay(),
            1,
            "落地径仍走 UpdateEnemyCommon：EnemyMove 每帧推进 Ens.delay（旧形恒 0）"
        );
        assert_eq!(m.enemy(0).status_raw(), 2, "status 仍是 Active");
    }
}
