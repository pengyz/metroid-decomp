//! `game-core::engine::ld47e` —— asm 例程族（首例程 LD47E（07:D47E）（P2 批 1 二次拆（一族一文件（文件名=m1disasm 标签（设计稿 §2））））。

use super::*;

// ---- SC-S2-10（S2-40）员转写（3 员——SamusElevator 族）----

/// $D47E SamusElevator（root——簇共根形三四）：$2D bit0 判（清⇒rts 早退/置⇒
/// SetmirrorCntrlBit+AnimDrawObject 链）。
pub fn s2_samus_elevator(v2d: u8) -> (bool, u8) {
    (v2d & 1 != 0, v2d >> 1)
}

/// $D450 员体：SamusElevator+$030D-$FC cmp $64 段（$E4F1 条件调）+$030D 判（ToggleSamusHi）。
pub fn s2_d450_body(v2d: u8, v030d: u8, vfc: u8) -> (Vec<(u16, u8)>, Option<u16>) {
    let (hit, _) = s2_samus_elevator(v2d);
    let d = v030d.wrapping_sub(vfc);
    let call = if hit && d < 0x64 { Some(0xE4F1) } else { None };
    (vec![], call)
}

/// $D46E 员体：SamusElevator+$0308←0+cmp 5/7 双判段+$2D lsr 段。
pub fn s2_d46e_body(v2d: u8) -> (Vec<(u16, u8)>, u8) {
    let (_, a) = s2_samus_elevator(v2d);
    (vec![(0x0308, 0)], a)
}

#[cfg(test)]
mod s2_040_tests {
    use super::*;

    /// 三员转写形（SamusElevator 判+二壳）。
    #[test]
    fn s2_040_units_three() {
        assert_eq!(s2_samus_elevator(0), (false, 0));
        assert_eq!(s2_samus_elevator(3), (true, 1));
        let (_, c) = s2_d450_body(3, 0x10, 0x05);
        assert_eq!(c, Some(0xE4F1));
        let (ev, a) = s2_d46e_body(0);
        assert!(ev.contains(&(0x0308, 0)));
        assert_eq!(a, 0);
    }
}

