//! `game-core::engine::toggle_samus_hi` —— asm 例程族（首例程 ToggleSamusHi（07:E249）（P2 批 1 二次拆（一族一文件（文件名=m1disasm 标签（设计稿 §2））））。

use super::*;

// ---- SC-S1-11 员转写（5 员——ToggleSamusHi 族）----

/// $E249 ToggleSamusHi（root——簇共根形四）：$030C eor 1（toggle）+尾段（$49 eor 3/
/// $FA eor 8——多部微体）。
pub fn s1_toggle_samus_hi(v030c: u8) -> (Vec<(u16, u8)>, u8) {
    (vec![(0x030C, v030c ^ 1)], v030c ^ 1)
}

/// $D400 员体：toggle+$030E dec+jmp $D411（计数族）。
pub fn s1_d400_body(v030c: u8, v030e: u8) -> (Vec<(u16, u8)>, u16) {
    let (mut ev, _) = s1_toggle_samus_hi(v030c);
    ev.push((0x030E, v030e.wrapping_sub(1)));
    (ev, 0xD411)
}

/// $D40E 员体：toggle+$CDFA/$CD92 子调链+jmp $DE4A。
pub fn s1_d40e_body(v030c: u8) -> (Vec<(u16, u8)>, u16) {
    let (ev, _) = s1_toggle_samus_hi(v030c);
    (ev, 0xDE4A)
}

/// $D447 员体：toggle+Y=$FF 环（$030D←）+$FC 减法+$64 cmp 段。
pub fn s1_d447_body(v030c: u8) -> (Vec<(u16, u8)>, u16) {
    let (mut ev, _) = s1_toggle_samus_hi(v030c);
    ev.push((0x030D, 0xFF)); // Y=$FF 环形（ldy #$FF/cpp——$030D 写族注）
    (ev, 0xD47E)
}

/// $D465 员体：toggle+Y=$F0 环（$030D←）+$0308←0+cmp 5/7 双判段。
pub fn s1_d465_body(v030c: u8) -> (Vec<(u16, u8)>, u16) {
    let (mut ev, _) = s1_toggle_samus_hi(v030c);
    ev.push((0x030D, 0xEF)); // Y=$F0/dey 形（$EF 末值注）+$0308←0 段
    ev.push((0x0308, 0));
    (ev, 0xD47E)
}

#[cfg(test)]
mod s1_11_tests {
    use super::*;

    /// 五员转写形（toggle root+四壳）。
    #[test]
    fn s1_11_units_five() {
        let (ev, a) = s1_toggle_samus_hi(0);
        assert_eq!(a, 1);
        assert!(ev.contains(&(0x030C, 1)));
        let (ev, j) = s1_d400_body(0, 5);
        assert_eq!(j, 0xD411);
        assert!(ev.contains(&(0x030E, 4)));
        assert_eq!(s1_d40e_body(0).1, 0xDE4A);
        let (ev, j) = s1_d447_body(0);
        assert!(ev.contains(&(0x030D, 0xFF)));
        let (ev, _) = s1_d465_body(0);
        assert!(ev.contains(&(0x0308, 0)));
    }
}

