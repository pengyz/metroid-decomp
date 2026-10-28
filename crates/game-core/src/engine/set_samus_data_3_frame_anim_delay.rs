//! `game-core::engine::set_samus_data_3_frame_anim_delay` —— asm 例程族（首例程 SetSamusData_3FrameAnimDelay（07:CD6B）（P2 批 1 二次拆（一族一文件（文件名=m1disasm 标签（设计稿 §2））））。

use super::*;

// ---- SC-S3-09（S3-69）员转写（2 员——$CD6B 态链族；L2 真 0 留白）----

/// $CD6B 员体（root——簇共根形六三）：lda #3→$DC8F 调+IsScrewAttackActive 调
/// （S2-24 接面）+cs 分段（$2D lsr/and 3/ora $A0→$6B 写）+$CDFA 调+尾段。
/// 返回 (写事件, cs)。
pub fn s3_cd6b_body(v2d: u8, carry: bool) -> (Vec<(u16, u8)>, bool) {
    if carry {
        return (vec![], true); // bcs +9 段
    }
    let v6b = ((v2d >> 1) & 3) | 0xA0;
    (vec![(0x006B, v6b)], false)
}

/// $D039 员体：jmp $CD6B（尾形）+$0314 判（bne 段）+$0300 cmp 6 判（$CF77/$CF55 调）。
pub fn s3_d039_body(v2d: u8, carry: bool) -> (Vec<(u16, u8)>, u16) {
    let (ev, _) = s3_cd6b_body(v2d, carry);
    (ev, 0xCD6B)
}

#[cfg(test)]
mod s3_069_tests {
    use super::*;

    /// 二员转写形（$CD6B 态链 root+一壳）。
    #[test]
    fn s3_069_units_two() {
        let (ev, c) = s3_cd6b_body(0x02, false);
        assert!(!c);
        assert!(ev.contains(&(0x006B, 0xA1)));
        let (ev2, c2) = s3_cd6b_body(0, true);
        assert!(c2);
        assert!(ev2.is_empty());
        assert_eq!(s3_d039_body(0, false).1, 0xCD6B);
    }
}

