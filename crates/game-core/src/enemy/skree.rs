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
        update_enemy_anim(mem, x, 2);
        update_enemy_common_no_move_no_anim(mem, x);
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
