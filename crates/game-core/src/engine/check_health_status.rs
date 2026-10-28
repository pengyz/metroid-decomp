//! `game-core::engine::check_health_status` —— asm 例程族（首例程 CheckHealthStatus（07:CDFA）（P2 批 1 二次拆（一族一文件（文件名=m1disasm 标签（设计稿 §2））））。

use super::*;

// ---- SC-S2-07（S2-37）员转写（3 员——CheckHealthStatus 族；免重判定=真转写+重合注记）----

/// $CDFA CheckHealthStatus（root——簇共根形三一）：$030A 位5 判——**beq +47=写径**
/// （$030A 清+$0683 声旗）；位5 置=hit 径（$70/$72/$77 写族）。分支向正案。
pub fn s2_check_health(v030a: u8) -> (Vec<(u16, u8)>, bool) {
    if v030a & 0x20 == 0 {
        (vec![(0x030A, 0), (0x0683, 8)], false) // 写径（beq +47）
    } else {
        (vec![(0x0070, 0x32), (0x0072, 0xFF)], true) // hit 径（$70/$72 族）
    }
}

/// $D406 员体：jmp $D411（尾形——**S1-11 流区双视注记**（跳面/员面））。
pub fn s2_d406_body() -> u16 { 0xD411 }

/// $D411 员体：$CDFA/$CD92 子调链+jmp $DE4A（**S1-11 流区双视注记**——跳面已折
/// （s1_d400 尾跳）/员面本卡真现）。
pub fn s2_d411_body(v030a: u8) -> (Vec<(u16, u8)>, u16) {
    let (ev, _) = s2_check_health(v030a);
    (ev, 0xDE4A)
}

#[cfg(test)]
mod s2_037_tests {
    use super::*;

    /// 三员转写形（CheckHealthStatus 分支向正+双视注记）。
    #[test]
    fn s2_037_units_three() {
        let (ev, hit) = s2_check_health(0);
        assert!(!hit);
        assert!(ev.contains(&(0x030A, 0)));
        assert!(ev.contains(&(0x0683, 8)));
        let (ev2, hit2) = s2_check_health(0x20);
        assert!(hit2);
        assert!(ev2.contains(&(0x0070, 0x32)));
        assert_eq!(s2_d406_body(), 0xD411);
        assert_eq!(s2_d411_body(0).1, 0xDE4A);
    }
}

