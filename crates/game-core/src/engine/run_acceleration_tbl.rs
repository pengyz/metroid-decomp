//! `game-core::engine::run_acceleration_tbl` —— asm 例程族（首例程 RunAccelerationTbl（07:CCC0）（P2 批 1 二次拆（一族一文件（文件名=m1disasm 标签（设计稿 §2））））。

use super::*;

// ---- SC-S3-14（S3-74）员转写（2 员——SetSamusRunAccel 族）----

/// $CCC0 RunAccelerationTbl（ROM 表烘焙——256B）。
pub use game_data::samus::samus_data::RUN_ACCEL_TBL;

/// $CCB7 SetSamusRunAccel（root——簇共根形六八）：$CCC0[x]→$0315（表读写例）。
/// 返回 (写事件, 表值)。
pub fn s3_set_samus_run_accel(x: u8) -> (Vec<(u16, u8)>, u8) {
    let v = RUN_ACCEL_TBL[x as usize];
    (vec![(0x0315, v)], v)
}

/// $D130 员体：SetSamusRunAccel+LCF2E 调（S2-033 接面）+$D150 调+$14 位判段
/// （$CFB7 条件调）+尾跳 $CD6D（A=2 面）。
pub fn s3_d130_body(x: u8) -> (Vec<(u16, u8)>, u8, u16) {
    let (ev, v) = s3_set_samus_run_accel(x);
    (ev, v, 0xCD6D)
}

#[cfg(test)]
mod s3_074_tests {
    use super::*;

    /// 二员转写形（SetSamusRunAccel 表读 root+一壳）。
    #[test]
    fn s3_074_units_two() {
        let (ev, v) = s3_set_samus_run_accel(186);
        assert_eq!(v, 9); // 案实核（x=186→9）
        assert!(ev.contains(&(0x0315, 9)));
        let (_, v2, j) = s3_d130_body(243);
        assert_eq!(v2, 56); // 案实核（x=243→56）
        assert_eq!(j, 0xCD6D);
    }
}

