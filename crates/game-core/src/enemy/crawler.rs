//! `game-core::enemy::crawler` —— Crawler/Zoomer 攀爬 AI（批量改写·第二十六簇（六步流程））。
//!
//! 【step1 定域取证 spec】asm 内容锚（refs/m1disasm/SRC/enemies/crawler.asm——A 级引证）：
//! - `CrawlerAIRoutine_{AREA}`（:1）——①6/8 帧门（`jsr ShouldCrawlerMove/and #$03/beq @exit`
//!   （:3-6）⇒counter&3!=0 移动（3/4=6/8 帧））②状态门（EnemyStatusPreAI Resting/Explode
//!   出口+EnsExtra.status==Explode 出口（:8-22））③朝向（`Ens.data0A&$03`（:24-25）：0=地面
//!   正行/1=墙面下行/2=天花板倒行/3=对墙上行）④lava 转向（朝向==1 且 y==边界（bank1/4=$E4
//!   （:40-42）/bank2/5=$EB）⇒CrawlerFlipDirection+朝向←3（:60-66）——**BUG 保真（D4）**：
//!   不检水平房（:36-39 注释））⑤@move（JumpByRTSToMovementRoutine+CrawlerInsideCornerCheck
//!   （:68-71））⑥@exit（每 3 帧 anim（`lda #$03/jsr UpdateEnemyAnim`（:74-75）））
//! - `CrawlerReorientSprite_{AREA}`（:82）——Y=朝向*2+方向（`data05 lsr`方向位→carry/
//!   `data0A&$03 rol`合成）⇒CrawlerAnimIndexTable 8 项（:91-98（Zoomer 序：Floor/Floor/
//!   LeftWall/RightWall/Ceiling/Ceiling/RightWall/LeftWall））
//!
//! 【step2 域结构】CrawlerState（朝向/方向/y/动画帧计数——语义命名（data/naming A 级锚注））；
//! 【step3 memmap 适配】Ens.data0A（朝向）/Ens.data05（方向位）/Ens.y（constants_ram En 面）；
//! 【step4 shim 写集】crawler_tick⇒朝向写回+动画帧号+方向翻转标记；
//! 【step5 测试】朝向×方向全域+帧门全域+转向全域（下 `mod tests`）；
//! 【step6 registry 登记】l2ports.rs `s4_cluster_crawler`。

/// Crawler 朝向（Ens.data0A&$03（crawler.asm:24-25 注释））。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CrawlerOrientation {
    /// #$00 地面正行（forwards on floor）
    FloorForward = 0,
    /// #$01 墙面下行（moving down a wall）
    WallDown = 1,
    /// #$02 天花板倒行（backwards on ceiling）
    CeilingBackward = 2,
    /// #$03 对墙上行（moving up an opposite wall）
    WallUp = 3,
}

impl CrawlerOrientation {
    pub fn from_bits(b: u8) -> Self {
        match b & 3 {
            0 => Self::FloorForward,
            1 => Self::WallDown,
            2 => Self::CeilingBackward,
            _ => Self::WallUp,
        }
    }
    pub fn bits(self) -> u8 {
        self as u8
    }
}

/// 6/8 帧移动门（:3-6 `and #$03/beq`——counter&3!=0 ⇒ 移动（3/4=6/8 帧））。
pub fn should_crawler_move(frame_counter: u8) -> bool {
    frame_counter & 0x03 != 0
}

/// lava 转向裁定（:36-66）：朝向==WallDown 且 y==lava 边界 ⇒ 翻转（CrawlerFlipDirection）
/// +朝向←WallUp（BUG 保真：不检水平房（D4））。返回（新朝向, 是否翻转）。
pub fn lava_turnaround(orientation: CrawlerOrientation, y: u8, lava_boundary: u8) -> (CrawlerOrientation, bool) {
    if orientation == CrawlerOrientation::WallDown && y == lava_boundary {
        (CrawlerOrientation::WallUp, true) // 横向翻转随记（flip 面）
    } else {
        (orientation, false)
    }
}

/// CrawlerReorientSprite（:82-89）：anim 表索引=朝向*2+方向（data05 方向位（lsr→carry））。
/// 表序（Zoomer bank1（:91-98））：Floor/Floor/LeftWall/RightWall/Ceiling/Ceiling/RightWall/LeftWall。
pub fn reorient_sprite_index(direction_bit: u8, orientation: CrawlerOrientation) -> u8 {
    ((orientation.bits() << 1) | (direction_bit & 1)) & 7
}

/// Zoomer CrawlerAnimIndexTable 语义序（:91-98——8 项）。
pub const ZOOMER_ANIM_TABLE: [&str; 8] = [
    "EnAnim_ZoomerOnFloor", "EnAnim_ZoomerOnFloor",
    "EnAnim_ZoomerOnLeftWall", "EnAnim_ZoomerOnRightWall",
    "EnAnim_ZoomerOnCeiling", "EnAnim_ZoomerOnCeiling",
    "EnAnim_ZoomerOnRightWall", "EnAnim_ZoomerOnLeftWall",
];

/// shim 写集适配（step4）：朝向写回（data0A 低 2 位）+动画帧号（reorient 索引）。
pub fn crawler_writes(orientation: CrawlerOrientation, anim_index: u8) -> Vec<(u16, u8)> {
    vec![(0x0000, orientation.bits()), (0x0001, anim_index)]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 【step5 穷举等价证明】帧门全域（counter 0..=255——6/8 形）。
    #[test]
    fn exhaustive_move_gate_6_of_8() {
        let mut moves = 0u32;
        for c in 0u8..=255 {
            if should_crawler_move(c) {
                moves += 1;
            }
        }
        // 256 帧中 3/4 移动=192（6/8 帧）
        assert_eq!(moves, 192);
    }

    /// 朝向×方向全域（4×2=8 项——表索引逐钉）。
    #[test]
    fn exhaustive_reorient_index() {
        for o in 0u8..4 {
            let ori = CrawlerOrientation::from_bits(o);
            for d in 0u8..2 {
                let idx = reorient_sprite_index(d, ori);
                assert_eq!(idx, (o << 1) | d);
                assert!(idx < 8);
            }
        }
        // 表语义逐钉（Zoomer 序）
        assert_eq!(ZOOMER_ANIM_TABLE[0], "EnAnim_ZoomerOnFloor");
        assert_eq!(ZOOMER_ANIM_TABLE[2], "EnAnim_ZoomerOnLeftWall");
        assert_eq!(ZOOMER_ANIM_TABLE[4], "EnAnim_ZoomerOnCeiling");
        assert_eq!(ZOOMER_ANIM_TABLE[7], "EnAnim_ZoomerOnLeftWall");
    }

    /// lava 转向全域（朝向 4×y 全域（0/边界/255）——翻转条件逐钉）。
    #[test]
    fn exhaustive_lava_turnaround() {
        for o in 0u8..4 {
            let ori = CrawlerOrientation::from_bits(o);
            for y in 0u8..=255 {
                for &b in &[0xE4u8, 0xEB] {
                    let (new_ori, flipped) = lava_turnaround(ori, y, b);
                    if ori == CrawlerOrientation::WallDown && y == b {
                        assert_eq!(new_ori, CrawlerOrientation::WallUp);
                        assert!(flipped);
                    } else {
                        assert_eq!(new_ori, ori);
                        assert!(!flipped);
                    }
                }
            }
        }
    }
}

// ---- 引擎侧 AI 例程（原 `behaviour.rs`，纯搬移；与上方纯逻辑件待后续去重，见 docs TODO）----

use crate::enemy::framework::*;
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
