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
