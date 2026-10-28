//! `game-core::engine::samus_hurt_f311` —— asm 例程族（首例程 SamusHurt_F311（07:F311）（P2 批 1 二次拆（一族一文件（文件名=m1disasm 标签（设计稿 §2））））。

use super::*;

// ---- SC-S2-22（S2-52）员转写（3 员——SamusHurt 族；L2 真 0 留白）----

/// $F311 SamusHurt（root——簇共根形四六）：cs 分段（$010F←$E0）+$F338 调+$0F 判
/// （$73←1）+$6E/$6F←0（指针清）。
/// 返回 (写事件, cs 与否)。
pub fn s2_samus_hurt(carry: bool, v0f: u8) -> (Vec<(u16, u8)>, bool) {
    if carry {
        return (vec![], true); // bcs Exit22 段
    }
    let mut ev = vec![(0x010F, 0xE0)];
    if v0f != 0 {
        ev.push((0x0073, 1));
    }
    ev.push((0x006E, 0));
    ev.push((0x006F, 0));
    (ev, false)
}

/// $F133 员体：SamusHurt+Xminus16+cmp $C0 段+尾跳 $CE92（S1-26 接面）。
pub fn s2_f133_body(carry: bool, v0f: u8, x: u8) -> (Vec<(u16, u8)>, u8) {
    let (mut ev, _) = s2_samus_hurt(carry, v0f);
    ev.push((0x0002, x.wrapping_sub(0x10))); // Xminus16 面
    (ev, x.wrapping_sub(0x10))
}

/// $FEC2 员体：SamusHurt+$6E←$50+尾跳 $CE92（S2-27 同构面——表载段）。
pub fn s2_fec2_body(carry: bool, v0f: u8) -> (Vec<(u16, u8)>, u16) {
    let (mut ev, _) = s2_samus_hurt(carry, v0f);
    ev.push((0x006E, 0x50));
    (ev, 0xCE92)
}

#[cfg(test)]
mod s2_052_tests {
    use super::*;

    /// 三员转写形（SamusHurt root+二壳）。
    #[test]
    fn s2_052_units_three() {
        assert_eq!(s2_samus_hurt(true, 0).1, true); // bcs 段
        let (ev, _) = s2_samus_hurt(false, 5);
        assert!(ev.contains(&(0x010F, 0xE0)));
        assert!(ev.contains(&(0x0073, 1)));
        assert!(ev.contains(&(0x006E, 0)));
        let (ev, nx) = s2_f133_body(false, 5, 0x70);
        assert_eq!(nx, 0x60);
        assert_eq!(s2_fec2_body(false, 5).1, 0xCE92);
    }
}

