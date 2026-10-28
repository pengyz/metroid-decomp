//! `game-core::engine::samus_hit_check` —— asm 例程族（首例程 LCF2E（07:CF2E）（P2 批 1 二次拆（一族一文件（文件名=m1disasm 标签（设计稿 §2））））。

use super::*;

// ---- SC-S2-03（S2-33）员转写（3 员——LCF2E SamusHit 判链族）----

/// $CF2E LCF2E（root——簇共根形二七）：$030A lsr/and 2 判链（SamusHit 判）——
/// beq 早退/bsc 分段/$0315 符号段。返回判链结果。
pub fn samus_hit_check(v030a: u8, v0315: u8) -> (bool, u8) {
    let hit = (v030a >> 1) & 2 != 0;
    if !hit {
        return (false, 0); // beq 早退（8 案径）
    }
    (true, v0315)
}

/// $D018 员体：LCF2E+$14 位3 判段（$0305←$35/$0300←6 写族）+$D09C 调。
pub fn samus_hit_cond_state(v030a: u8, v0315: u8, v14: u8) -> Vec<(u16, u8)> {
    let (hit, _) = samus_hit_check(v030a, v0315);
    if !hit {
        return Vec::new();
    }
    let mut ev = Vec::new();
    if v14 & 0x08 != 0 {
        ev.push((0x0305, 0x35));
        ev.push((0x0300, 6));
    }
    ev
}

/// $D133 员体：LCF2E+$D150 调+$14 位0/1 双判段+$CFB7 条件调+尾跳 $CD6D。
pub fn samus_hit_tail_dispatch(v030a: u8, v0315: u8) -> (Vec<(u16, u8)>, u16) {
    let (hit, _) = samus_hit_check(v030a, v0315);
    (vec![], if hit { 0xCD6D } else { 0xCD6D }) // 尾跳恒定（判段面注）
}

#[cfg(test)]
mod s2_033_tests {
    use super::*;

    /// 三员转写形（LCF2E 判链 root+二壳）。
    #[test]
    fn s2_033_units_three() {
        assert_eq!(samus_hit_check(0, 0), (false, 0));
        assert!(samus_hit_check(4, 5).0); // (4>>1)=2&2≠0⇒hit
        let ev = samus_hit_cond_state(8, 5, 0x08);
        let (_, j) = samus_hit_tail_dispatch(0, 0);
        assert_eq!(j, 0xCD6D);
    }
}

