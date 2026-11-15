//! Skree AI（`refs/m1disasm/SRC/enemies/skree.asm`；5 label）。

use crate::enemy::framework::*;
use crate::enemy::motion::*;
use crate::enemy::prims::*;
use crate::frame::tick::*;
use crate::ram_map;

/// SkreeAIRoutine（**asm 内容锚** `SkreeAIRoutine_{AREA}`（refs/m1disasm/SRC/enemies/
/// skree.asm:2-71——A 级逐行））：悬顶静息（PreAI=Resting⇒anim 8）→近距激活（阈距
/// data05 bit3——F75B 件）→坠落（movementInstrIndex($+7)<$0F——EnemyGetDeltaY delta 径）
/// →落地（≥$0F）⇒爆延时←$3A→≥$11⇒爆体径（jumpDsplcmnt($+13) 倒数⇒清槽+**4 弹**）。
pub(crate) fn skree_ai(mem: &mut crate::frame::FrameMemory, x: u8) {
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
    // 坠落/落地径（SkreeExit_Active（:56-60）：anim 2+移动）——
    // 【对抗复核收敛⑤（2026-11-01）】EnemyMove 双门（areas_common:115-123 逐字）：
    // ①data05 bit6=30fps 门（`lda data05/asl/bmi`——置⇒**跳过移动**）②status==Active(2)
    // 门（`lda status/cmp #2/bne RTS`）——两门不过⇒只 anim+draw（无位移）。
    let d05 = mem.read_ram(0x0400 + x as u16 + 5);
    let st_now = mem.read_wram(0x6AF4 + x as u16);
    let gates_ok = (d05 & 0x40) == 0 && st_now == 2;
    if gates_ok {
    let type_ = mem.read_wram(0x6AF4 + x as u16 + 14) & 0x3F;
    let d05 = mem.read_ram(0x0400 + x as u16 + 5);
    let mov_idx = mem.read_ram(0x0400 + x as u16 + EN_MOV_IDX); // ROM `BD 08 04`
    let instr2 = mem.read_ram(0x0400 + x as u16 + EN_INSTR); // ROM `BC 06 04`
    let delay = mem.read_ram(0x0400 + x as u16 + EN_DELAY); // ROM `FE 09 04`
    let sub_y = mem.read_ram(0x0400 + x as u16 + EN_SUBPIXEL_Y); // union 同字节
    let speed_y = mem.read_ram(0x0400 + x as u16 + 2);
    let accel_y = mem.read_wram(0x6AF4 + x as u16 + 10);
    let d1f = mem.read_wram(0x6AF4 + x as u16 + 15);
    let can_up = enemy_check_move_up(mem, x);
    let can_down = enemy_check_move_down(mem, x);
    let (dy, dx, ndelay, ninstr, nd1f, trig) = crate::enemy::common::enemy_get_delta_xy(
        crate::frame::cur_area(mem), type_, d05, mov_idx, instr2, delay, sub_y, speed_y, accel_y, d1f, can_up, can_down);
    mem.write_wram(0x6AF4 + x as u16 + 15, nd1f);
    if trig {
        // `$82FB @fail`：`L968B[type]` bit5 置 ⇒ **翻面朝 + 清 bits6-7**
        // （asm `lda data05 / eor #$05 / ora L968B,y / and #$1F / sta data05`）。
        // 对 Skree（`L968B[4]=$86`，bit5=0）不可观测；对 `$18` 一系（`L968B[0x18]=$20`）必需。
        let l968b = game_data::assets::enemy_force_attribs(crate::frame::cur_area(mem))
            .get(type_ as usize)
            .copied()
            .unwrap_or(0);
        if l968b & 0x20 != 0 {
            let d5f = mem.read_ram(0x0400 + x as u16 + 5);
            mem.write_ram(0x0400 + x as u16 + 5, ((d5f ^ 0x05) | l968b) & 0x1F);
        }
        // `EnemyTriggerResting_AndClearEnAccelY`（`$81B1`）：bits5 置 + accelY（EnsExtra+10）清 0。
        let d5t = mem.read_ram(0x0400 + x as u16 + 5);
        mem.write_ram(0x0400 + x as u16 + 5, d5t | 0x20);
        mem.write_wram(0x6AF4 + x as u16 + 10, 0);
    }
    mem.write_ram(0x0400 + x as u16 + EN_DELAY, ndelay);
    mem.write_ram(0x0400 + x as u16 + EN_INSTR, ninstr);
    if dy != 0 {
        // Δ 施加（EnemyMove 径（:10029-10046 同构）：y±|Δ|——名表换补偿）
        let mut y = mem.read_ram(0x0400 + x as u16 + 0);
        let hi = mem.read_wram(0x6AF4 + x as u16 + 7);
        for _ in 0..dy.unsigned_abs() {
            if dy < 0 {
                if y == 0 {
                    y = 0xEF;
                    mem.write_wram(0x6AF4 + x as u16 + 7, hi ^ 1);
                } else {
                    y -= 1;
                }
            } else if y == 0xEF {
                y = 0;
                mem.write_wram(0x6AF4 + x as u16 + 7, hi ^ 1);
            } else {
                y += 1;
            }
        }
        mem.write_ram(0x0400 + x as u16 + 0, y);
    }
    // ⑦ deltaX 整轴（skree 串 packed nibble 低半字节——斜线下坠（复核⑦））
    if dx != 0 {
        let mut xx = mem.read_ram(0x0400 + x as u16 + 1);
        let hi = mem.read_wram(0x6AF4 + x as u16 + 7);
        for _ in 0..dx.unsigned_abs() {
            if dx < 0 {
                if xx == 0 {
                    xx = 0xFF;
                    mem.write_wram(0x6AF4 + x as u16 + 7, hi ^ 1);
                } else {
                    xx -= 1;
                }
            } else if xx == 0xFF {
                xx = 0;
                mem.write_wram(0x6AF4 + x as u16 + 7, hi ^ 1);
            } else {
                xx += 1;
            }
        }
        mem.write_ram(0x0400 + x as u16 + 1, xx);
    }
    } // 门径尾（gates_ok）
    update_enemy_anim(mem, x, 2);
    update_enemy_common_no_move_no_anim(mem, x);
}
