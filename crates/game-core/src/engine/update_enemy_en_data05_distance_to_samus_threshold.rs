//! `game-core::engine::update_enemy_en_data05_distance_to_samus_threshold` —— asm 例程族（首例程 UpdateEnemy_EnData05DistanceToSamusThreshold（07:F75B）（P2 批 1 二次拆（一族一文件（文件名=m1disasm 标签（设计稿 §2））））。

use super::*;

// ---- SC-S2-27（S2-57）员转写（3 员——DistanceToSamusThreshold 族；L2 真 0 留白）----

/// $F75B UpdateEnemy_EnData05DistanceToSamusThreshold（root——簇共根形五一）：
/// $06←$E7/$F744 调（OrEnData05——S2-01 同族）+$96AB[y] 表走+位1 判段。
/// 返回 (写事件, 表值)。
pub fn s2_dist_threshold(x: u8, tbl_val: u8) -> (Vec<(u16, u8)>, u8) {
    (vec![(0x0006, 0xE7), (0x0405 + x as u16, 0x18)], tbl_val) // OR 写面（$18 常量）
}

/// $F3D2 员体：DistThreshold+$F682/$F676 链（S1-01 接面）+尾跳 $F40A。
pub fn s2_f3d2_body(x: u8, tbl_val: u8) -> (Vec<(u16, u8)>, u16) {
    let (ev, _) = s2_dist_threshold(x, tbl_val);
    (ev, 0xF40A)
}

/// $F404 员体：DistThreshold+$F51E/$F536 链（S2-26 接面）+尾跳 $95E5。
pub fn s2_f404_body(x: u8, tbl_val: u8) -> (Vec<(u16, u8)>, u16) {
    let (ev, _) = s2_dist_threshold(x, tbl_val);
    (ev, 0x95E5)
}

#[cfg(test)]
mod s2_057_tests {
    use super::*;

    /// 三员转写形（DistThreshold root+二壳）。
    #[test]
    fn s2_057_units_three() {
        let (ev, t) = s2_dist_threshold(2, 0x55);
        assert_eq!(t, 0x55);
        assert!(ev.contains(&(0x0006, 0xE7)));
        assert!(ev.contains(&(0x0407, 0x18)));
        assert_eq!(s2_f3d2_body(2, 0).1, 0xF40A);
        assert_eq!(s2_f404_body(2, 0).1, 0x95E5);
    }
}

