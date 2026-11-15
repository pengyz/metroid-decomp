//! Crawler/Zoomer 攀爬 AI（引擎侧实体化）。
//!
//! asm 内容锚（`refs/m1disasm/SRC/enemies/crawler.asm`，10 label）：
//! - `CrawlerAIRoutine_{AREA}`（:1）——①移动门 `ShouldCrawlerMove ; 07:F852`
//!   （`txa / lsr×3（carry←bit2）/ adc FrameCount / lsr / and #$03`）②`EnemyStatusPreAI`
//!   三连门（Resting/Explode 两出口 + 现役 Explode）③朝向 `Ens.data0A & $03`
//!   （0 地面正行 / 1 墙面向下 / 2 天花板倒行 / 3 对墙向上）④岩浆折返
//!   （朝向==1 ∧ y==边界 bank1/4=`$E4`、bank2/5=`$EB` ⇒ `CrawlerFlipDirection` + 朝向←3；
//!   **BUG 保真 D4**：asm 自注「不检水平房」照留）⑤`@move` = `JumpByRTSToMovementRoutine`
//!   + 内角检 ⑥`@exit` 每帧 `UpdateEnemyAnim #3`。
//! - `CrawlerReorientSprite_{AREA}`（:82）——`idx = 朝向*2 + 方向位`，表 `$9AA0` 8 项
//!   `[0x35,0x35,0x3E,0x38,0x3B,0x3B,0x38,0x3E]`（Zoomer 序），写
//!   `resetAnimIndex`/`animIndex`、`animDelay←0`、清 `data05` bit2/1。
//!
//! 本文件原先还有一份「纯逻辑壳」（`CrawlerOrientation` / `should_crawler_move` /
//! `lava_turnaround` / `reorient_sprite_index` / `ZOOMER_ANIM_TABLE` / `crawler_writes`），
//! 与这里的实现是**同一 asm 的第二份投影且不等价**（`should_crawler_move` 只吃帧计数、
//! 漏了 `x>>3` 项；`crawler_writes` 写 `$0000/$0001` 且无 ROM 依据；`ZOOMER_ANIM_TABLE`
//! 只有名字没有值），且全仓零引用——已删除，改由下方 ROM 锚测试守住真实现。

use crate::enemy::motion::*;
use crate::enemy::prep::*;
use crate::frame::tick::*;
use crate::ram_map;

/// ShouldCrawlerMove（**asm 内容锚** `07:F852`——`txa/lsr×3（carry←bit2）/adc FrameCount/
/// lsr`）：gate=((X>>3)+bit2(X)+FC)>>1&3（移动门 6/8 帧形）。
/// 【H3 根修（h3-enemy-slot-residual.md §2）】asm 合形直读 FC（无 −1）——goal-轮 43 的
/// FC−1 补偿=把 dump FC（逻辑读+1）代入公式的错位代偿（金方跳格 dump 17/18/25/26=
/// 逻辑读 16/17/24/25 ⇒ raw 公式四点全中）；r14 FC 对齐后失配（与 bit6 +1 同源）。
pub(crate) fn crawler_move_gate(x: u8, fc: u8) -> u8 {
    ((x >> 3).wrapping_add((x >> 2) & 1).wrapping_add(fc) >> 1) & 0x03
}

/// CrawlerAIRoutine（**asm 内容锚** `enemies/crawler.asm`——r26+2026-10-31 PreAI 门补装）：
/// ShouldCrawlerMove（F852——((x>>3+FC+lsr 进位))>>1&3=6/8 帧步门）；不动帧⇒@exit
/// （anim3+显示）；动帧⇒**EnemyStatusPreAI 三连门**（Resting⇒SkreeExit_Resting=anim8+
/// 显示（bank1 共享 skree 出口）/PreAI Explode⇒SkreeExit_Explode（仅显示）/
/// 现役 Explode⇒@exit）→@move。缺门症（已修）：Resting 帧即跑 AI——激活帧半径未写
/// （draw 同 tick 后行）⇒rx=0 全检快径⇒外角下探伪成功转角⇒菱形回路（crawlprobe f797）。
pub(crate) fn crawler_ai(mem: &mut crate::frame::FrameMemory, x: u8) {
    // 【goal-轮30 全径打印】入口+全出口（n=1664 漏步帧出口面直证）。
    // F852：txa/lsr×3（carry←原 bit2）/adc FrameCount（含进位）/lsr——门含 lsr 移出位。
    // 【goal-轮2 采样缝更正】toggle 公式正确——"bit6 反相"=采样缝伪象（我 spin-exit=帧首
    // /emu=帧末，同帧差一次更新）⇒真根=走位首动帧差（f798 早 1px）。
    // 【H3 测试缝（findings/h3-enemy-slot-residual.md）】gate 公式抽函数
    // crawler_move_gate（行为随形——H3 TDD 红靶：FC−1 相位补偿）。
    let gate = crawler_move_gate(x, mem.read_ram(crate::ram_map::FRAME_COUNT));
    // 【goal-轮15 gate 值直证】F852 门值逐帧（金方 2 帧跳 vs 我方移动模式）。
    if gate == 0 {
        update_enemy_anim(mem, x, 3); // @exit `lda #3/jsr UpdateEnemyAnim`
        update_enemy_common_no_move_no_anim(mem, x);
        return;
    }
    // EnemyStatusPreAI 三连门（crawler.asm:8-22 逐条——激活帧拦 Resting=本修核心）。
    let pre_ai = mem.read_ram(0x0081);
    if pre_ai == 1 {
        // SkreeExit_Resting（bank1/4 共享——skree.asm:62 `lda #8/noMove`）
        update_enemy_anim(mem, x, 8);
        update_enemy_common_no_move_no_anim(mem, x);
        return;
    }
    if pre_ai == 3 {
        // SkreeExit_Explode（skree.asm:68——`jmp noMoveNoAnim` 无 anim）
        update_enemy_common_no_move_no_anim(mem, x);
        return;
    }
    if mem.read_wram(0x6AF4 + x as u16) == 3 {
        // 现役 Explode⇒@exit（crawler.asm:20-22——anim3 落穿 noMoveNoAnim）
        update_enemy_anim(mem, x, 3);
        update_enemy_common_no_move_no_anim(mem, x);
        return;
    }
    // @move 径：JumpByRTSToMovementRoutine（areas_common \$8048 表 RTS 跳）
    // Y=(orientation*2+direction)*2→CrawlerMovementRoutinesTable→四例行。
    let d5 = mem.read_ram(0x0400 + x as u16 + 5);
    let orientation = mem.read_ram(0x0400 + x as u16 + 10) & 0x03; // data0A=+10
    // 【goal-轮7 转向路径直证】orientation 变化+原因。
    let direction = d5 & 1;
    // 【岩浆折返径】（crawler.asm:26-64——**asm BUG 注保真**（D4：竖房误触发不修））：
    // orientation==1（下墙径）∧ y==$E4（BANK1/4 阈——BANK2/5=$EB）⇒FlipDirection+
    // orientation←3（上墙）+跳过本帧移动（@afterLavaTurnAround 直入外角检）。
    let ey = mem.read_ram(0x0400 + x as u16 + 0);
    if orientation == 1 && ey == 0xE4 {
        let d = mem.read_ram(0x0400 + x as u16 + 5);
        mem.write_ram(0x0400 + x as u16 + 5, d ^ 0x01); // CrawlerFlipDirection
        mem.write_ram(0x0400 + x as u16 + 10, 0x03); // orientation←3（上墙）
        // @afterLavaTurnAround（跳过 @move 移动+内角检）
        crawler_outside_corner_check(mem, x);
        update_enemy_anim(mem, x, 3);
        update_enemy_common_no_move_no_anim(mem, x);
        return;
    }
    let idx = ((orientation as usize) * 2 + direction as usize) & 0x07;
    let ok = match idx {
        0 | 5 => enemy_move_one_pixel_right(mem, x),
        1 | 4 => enemy_move_one_pixel_left(mem, x),
        2 | 3 => enemy_move_one_pixel_down(mem, x),
        _ => enemy_move_one_pixel_up(mem, x),
    };
    if !ok {
        // CrawlerInsideCornerCheck（bank1:$9AA8——**goal-r16 G3'**）：$00 契约判据。
        // $00=1(离屏/wrap)⇒FlipDirection(data05 eor #1)；$00=0(块界碰撞)⇒data0A=(data0A−1)&3。
        // asm：move($9AE2)后 **InsideCorner($9AA8 失败径)+OutsideCorner($9AC6 探径)都跑**（非互斥）。
        let zero = mem.read_ram(0x00);
        if zero != 0 {
            let d = mem.read_ram(0x0400 + x as u16 + 5);
            mem.write_ram(0x0400 + x as u16 + 5, d ^ 0x01); // FlipDirection（离屏径）
        } else {
            let o = mem.read_ram(0x0400 + x as u16 + 10); // data0A
            mem.write_ram(0x0400 + x as u16 + 10, o.wrapping_sub(1) & 0x03); // 朝向--
            crawler_reorient_sprite(mem, x); // $9A8F（ InsideCorner 碰撞径 `4C 8F 9A`）
        }
    }
    // CrawlerOutsideCornerCheck（bank1:$9AC6——**goal-r16 G3'**）：恒跑
    crawler_outside_corner_check(mem, x);
    update_enemy_anim(mem, x, 3); // @exit `A9 03/20 0C 80`（cadence=3）
    update_enemy_common_no_move_no_anim(mem, x); // @exit 径 `4C 06 80`（anim+显示）
}

/// CrawlerOutsideCornerCheck（bank1:$9AC6——`jsr GetNextOrientation/jsr MoveRoutine`
/// `bcc @RTS`（探移失败⇒RTS）/成功⇒data0A←next+ReorientSprite）——恒跑（@move 与
/// @afterLavaTurnAround 共径）。
pub(crate) fn crawler_outside_corner_check(mem: &mut crate::frame::FrameMemory, x: u8) {
    let o = mem.read_ram(0x0400 + x as u16 + 10) & 0x03;
    let next_o = (o + 1) & 0x03;
    let dir = mem.read_ram(0x0400 + x as u16 + 5) & 1;
    let nidx = ((next_o as usize) * 2 + dir as usize) & 0x07;
    let probe_ok = match nidx {
        0 | 5 => enemy_move_one_pixel_right(mem, x),
        1 | 4 => enemy_move_one_pixel_left(mem, x),
        2 | 3 => enemy_move_one_pixel_down(mem, x),
        _ => enemy_move_one_pixel_up(mem, x),
    };
    // 【goal-轮91 探径碰撞直证】外角探径成功/失败（转向差源）。
    if probe_ok {
        mem.write_ram(0x0400 + x as u16 + 10, next_o); // data0A+1（asm `9D 0A 04`）
        crawler_reorient_sprite(mem, x); // $9A8F（OutsideCorner 径 `20 8F 9A`）
    }
}

/// CrawlerReorientSprite（**ROM 字节锚** bank1:$9A8F——m1disasm 未标注区（D5 四方交叉：
/// ROM 直读 + 三层闭环：$9AA0→EnAnimTable→EnFramePtrTable1→emu OAM 实测一致））：
/// `lda data05/lsr`（flip=bit0 进 carry）`lda data0A/and #3/rol A`（idx=orient*2+flip）
/// `lda $9AA0,y / jmp InitEnAnimIndex`（F68D：reset/index←A、animDelay←0）。
/// 旧形「随件占位」⇒ 敌 animFrame 恒 0 ⇒ 画错帧（emu tile CA/CB/DA/DB vs rust 14）。
pub(crate) fn crawler_reorient_sprite(mem: &mut crate::frame::FrameMemory, x: u8) {
    const CRAWLER_SPRITE_ANIM_TBL: [u8; 8] =
        [0x35, 0x35, 0x3E, 0x38, 0x3B, 0x3B, 0x38, 0x3E]; // ROM $9AA0（orient*2+flip）
    let d05 = mem.read_ram(0x0400 + x as u16 + 5);
    let flip = d05 & 0x01; // lsr→carry=bit0
    let orient = (mem.read_ram(0x0400 + x as u16 + 10) & 0x03) as usize;
    let idx = ((orient << 1) | flip as usize) & 0x07;
    let v = CRAWLER_SPRITE_ANIM_TBL[idx];
    // 【goal-轮86 reorient idx 直证】reset 源（帧面差源）。
    // InitEnAnimIndex(F68D)
    mem.write_wram(0x6AF4 + x as u16 + 5, v); // resetAnimIndex
    mem.write_wram(0x6AF4 + x as u16 + 6, v); // animIndex
    mem.write_wram(0x6AF4 + x as u16 + 4, 0); // animDelay←0
    // 【goal-轮42 bit2/1 清源件】金腿实证（f1838 转角径 d05 0x1A→0x58——bit2/1(0x06)
    // 清+bit6 置）——转角径写面：d05 bit2/1 清（朝向/轴位重组——constants 无注余项位）。
    let d5c = mem.read_ram(0x0400 + x as u16 + 5);
    mem.write_ram(0x0400 + x as u16 + 5, d5c & 0xF9); // bit2/1 清（0x06）
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame::FrameMemory;

    /// `ShouldCrawlerMove ; 07:F852` 的 6/8 帧性：256 帧里恰好 64 帧 gate==0
    /// （每 8 帧动 6 帧）。对多个槽位抽样，覆盖 `x>>3` 与 `bit2` 两项。
    #[test]
    fn move_gate_is_six_of_eight_frames() {
        for x in [0x00u8, 0x20, 0x40, 0x98, 0xE0] {
            let gated = (0u16..=255).filter(|fc| crawler_move_gate(x, *fc as u8) == 0).count();
            assert_eq!(gated, 64, "槽 {x:#04x} 的 gate==0 帧数应为 64");
        }
    }

    /// gate 上界：恒在 0..=3（asm `and #$03`）。
    #[test]
    fn move_gate_is_two_bit() {
        for x in 0u8..=0xFF {
            for fc in [0u8, 1, 0x7F, 0xFF] {
                assert!(crawler_move_gate(x, fc) <= 3);
            }
        }
    }

    /// `CrawlerReorientSprite`：`idx=朝向*2+方向位` 取 ROM `$9AA0` 表，
    /// 写入 `resetAnimIndex` 与 `animIndex`、`animDelay←0`。
    #[test]
    fn reorient_writes_rom_table_entry() {
        const TBL: [u8; 8] = [0x35, 0x35, 0x3E, 0x38, 0x3B, 0x3B, 0x38, 0x3E];
        for orient in 0u8..4 {
            for flip in 0u8..2 {
                let mut m = FrameMemory::default();
                let x = 0x10u8;
                let en = 0x0400 + x as u16;
                m.write_ram(en + 5, flip); // data05 方向位
                m.write_ram(en + 10, orient); // data0A
                crawler_reorient_sprite(&mut m, x);
                let idx = ((orient << 1) | flip) as usize;
                let want = TBL[idx];
                assert_eq!(m.read_wram(0x6AF4 + x as u16 + 5), want, "resetAnimIndex");
                assert_eq!(m.read_wram(0x6AF4 + x as u16 + 6), want, "animIndex");
                assert_eq!(m.read_wram(0x6AF4 + x as u16 + 4), 0, "animDelay");
            }
        }
    }

    /// 转角径写面：`data05` bit2/1 清、bit0（方向）与 bit6 保留。
    #[test]
    fn reorient_clears_bit2_and_bit1_keeps_others() {
        for d5 in [0x1Au8, 0x5A, 0x7F] {
            let mut m = FrameMemory::default();
            let x = 0x10u8;
            let en = 0x0400 + x as u16;
            m.write_ram(en + 5, d5);
            m.write_ram(en + 10, 2);
            crawler_reorient_sprite(&mut m, x);
            let got = m.read_ram(en + 5);
            assert_eq!(got, d5 & 0xF9, "data05 {d5:#04x} ⇒ 只清 bit2/1");
        }
    }
}
