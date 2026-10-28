//! `game-core::engine::set_samus_jump` —— asm 例程族（首例程 SetSamusJump（07:CFC3）（P2 批 1 二次拆（一族一文件（文件名=m1disasm 标签（设计稿 §2））））。

use super::*;

// ---- SC-S3-08（S3-68）员转写（2 员——SetSamusJump 族）----

/// $CFC3 SetSamusJump（root——簇共根形六二）：Y=$0C→$0305/dey→$0306/$0304←4/
/// $030F←0/$0308←$FC+尾段（$0314←24/$0681←2 声旗——案实核恒定面）。
/// 返回 (写事件, 恒定 7 写)。
pub fn s3_set_samus_jump() -> Vec<(u16, u8)> {
    vec![(0x0304, 4), (0x0305, 12), (0x0306, 11), (0x0308, 0xFC),
         (0x030F, 0), (0x0314, 24), (0x0681, 2)]
}

/// $D036 员体：SetSamusJump+尾跳 $CD6B（S1-09 接面）+$0314 判（$0300 cmp 6 段）。
pub fn s3_d036_body() -> (Vec<(u16, u8)>, u16) {
    (s3_set_samus_jump(), 0xCD6B)
}

#[cfg(test)]
mod s3_068_tests {
    use super::*;

    /// 二员转写形（SetSamusJump 恒定 root+一壳）。
    #[test]
    fn s3_068_units_two() {
        let ev = s3_set_samus_jump();
        assert_eq!(ev.len(), 7);
        assert!(ev.contains(&(0x0305, 12)));
        assert!(ev.contains(&(0x0681, 2)));
        assert_eq!(s3_d036_body().1, 0xCD6B);
    }
}

