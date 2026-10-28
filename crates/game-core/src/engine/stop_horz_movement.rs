//! `game-core::engine::stop_horz_movement` —— asm 例程族（首例程 StopHorzMovement（07:CFB7）（P2 批 1 二次拆（一族一文件（文件名=m1disasm 标签（设计稿 §2））））。

use super::*;

// ---- SC-S3-16（S3-76）员转写（2 员——ClearHorzData 族）----

/// $CFB7 ClearHorzData（root——簇共根形七十）：$CF4C 调（ClearHorzMvmntData）
/// +$0315←Y（清族三写面：$0309/$0313/$0315←0——案实核恒定）。
/// 返回 (写事件, 恒定 3 写)。
pub fn s3_clear_horz_data() -> Vec<(u16, u8)> {
    vec![(0x0309, 0), (0x0313, 0), (0x0315, 0)]
}

/// $D13F 员体：ClearHorzData+尾跳 $CD6D（A=2 面——S1-09 接面）+$0308/$0312 清。
pub fn s3_d13f_body() -> (Vec<(u16, u8)>, u8, u16) {
    let mut ev = s3_clear_horz_data();
    ev.push((0x0308, 0));
    ev.push((0x0312, 0));
    (ev, 2, 0xCD6D)
}

#[cfg(test)]
mod s3_076_tests {
    use super::*;

    /// 二员转写形（ClearHorzData 恒定 root+一壳）。
    #[test]
    fn s3_076_units_two() {
        let ev = s3_clear_horz_data();
        assert_eq!(ev.len(), 3);
        assert!(ev.contains(&(0x0315, 0)));
        let (ev2, a, j) = s3_d13f_body();
        assert_eq!((a, j), (2, 0xCD6D));
        assert!(ev2.contains(&(0x0308, 0)));
    }
}


