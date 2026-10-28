//! `game-core::engine::set_samus_pnt_up` —— asm 例程族（首例程 SetSamusPntUp（07:CF77）（P2 批 1 二次拆（一族一文件（文件名=m1disasm 标签（设计稿 §2））））。

use super::*;

// ---- SC-S3-10（S3-70）员转写（2 员——SetSamusPntUp 族）----

/// $CF77 SetSamusPntUp（root——簇共根形六四）：$0300←4/SetSamusAnim($27)
/// （S1-09 接面）+$CFB7 调+$0304←Y+清组（案实核恒定 7 写面）。
/// 返回 (写事件, 恒定 7 写)。
pub fn s3_set_samus_pnt_up() -> Vec<(u16, u8)> {
    vec![(0x0300, 4), (0x0304, 0), (0x0305, 39), (0x0306, 39),
         (0x0309, 0), (0x0313, 0), (0x0315, 0)]
}

/// $D048 员体：SetSamusPntUp+bne 段（$CF55 调——S2-18 接面）+尾跳 $CD6D（A=3 面）。
pub fn s3_d048_body() -> (Vec<(u16, u8)>, u8, u16) {
    (s3_set_samus_pnt_up(), 3, 0xCD6D)
}

#[cfg(test)]
mod s3_070_tests {
    use super::*;

    /// 二员转写形（SetSamusPntUp 恒定 root+一壳）。
    #[test]
    fn s3_070_units_two() {
        let ev = s3_set_samus_pnt_up();
        assert_eq!(ev.len(), 7);
        assert!(ev.contains(&(0x0305, 39)));
        assert!(ev.contains(&(0x0300, 4)));
        let (_, a, j) = s3_d048_body();
        assert_eq!((a, j), (3, 0xCD6D));
    }
}

