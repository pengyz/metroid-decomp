//! `game-core::engine::is_screw_attack_active` —— asm 例程族（首例程 IsScrewAttackActive（07:CD9C）（P2 批 1 二次拆（一族一文件（文件名=m1disasm 标签（设计稿 §2））））。

use super::*;

// ---- SC-S2-24（S2-54）员转写（3 员——IsScrewAttackActive 族）----

/// $CD9C IsScrewAttackActive（root——簇共根形四八）：sec/Y=$0300-1（dey 链）+
/// $6878 位3 判+$0305 cmp $0E/$0C 双判（螺旋攻击态判）。
/// 返回 (激活, Y 值)。
pub fn s2_is_screw_attack(v300: u8, v6878: u8, v0305: u8) -> (bool, u8) {
    let y = v300.wrapping_sub(1);
    if y != 0 {
        return (false, y); // bne 段
    }
    let active = (v6878 & 0x08 != 0) && (v0305 == 0x0E || v0305 == 0x0C);
    (active, y)
}

/// $F287 员体：IsScrewAttackActive+Y=0 面+$6AF4[x] cmp 4 判+$6B02→$010F 表载。
pub fn s2_f287_body(v300: u8, v6878: u8, v0305: u8) -> (bool, u8) {
    s2_is_screw_attack(v300, v6878, v0305)
}

/// $F2F4 员体：IsScrewAttackActive+cs 分段（$010F←$80）+$F332/$F270 链+表载（$95CE→$6E）。
pub fn s2_f2f4_body(v300: u8, v6878: u8, v0305: u8) -> (Vec<(u16, u8)>, bool) {
    let (active, y) = s2_is_screw_attack(v300, v6878, v0305);
    let ev = if active { vec![(0x010F, 0x80)] } else { vec![] };
    (ev, active)
}

#[cfg(test)]
mod s2_054_tests {
    use super::*;

    /// 三员转写形（IsScrewAttackActive 双判 root+二壳）。
    #[test]
    fn s2_054_units_three() {
        assert_eq!(s2_is_screw_attack(1, 0x08, 0x0E), (true, 0));
        assert_eq!(s2_is_screw_attack(1, 0, 0x0E), (false, 0));
        assert_eq!(s2_is_screw_attack(5, 0x08, 0x0E), (false, 4)); // bne 段
        let (_, y) = s2_f287_body(1, 0x08, 0x0C);
        assert_eq!(y, 0);
        let (ev, _) = s2_f2f4_body(1, 0x08, 0x0E);
        assert!(ev.contains(&(0x010F, 0x80)));
    }
}

