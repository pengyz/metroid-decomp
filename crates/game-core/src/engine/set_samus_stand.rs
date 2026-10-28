//! `game-core::engine::set_samus_stand` —— asm 例程族（首例程 SetSamusStand（07:CF55）（P2 批 1 二次拆（一族一文件（文件名=m1disasm 标签（设计稿 §2））））。

use super::*;

// ---- SC-S1-18 员转写（4 员——SetSamusStand 族）----

/// $CF55 SetSamusStand（root——簇共根形十一·**自入口**）：$0315 判+$0300←Y/站姿常量
/// $0305/$0306←7+$14 位3 判段+$0313/$0315 清+$0680←8 声旗。
pub fn s1_set_samus_stand() -> Vec<(u16, u8)> {
    vec![(0x0300, 0), (0x0304, 0), (0x0305, 7), (0x0306, 7),
         (0x0309, 0), (0x0313, 0), (0x0315, 0), (0x0680, 8)]
}

/// $D04D 员体：SetSamusStand+尾跳 $CD6D（A=3 面——SetSamusData 跨卡面注）。
pub fn s1_d04d_body() -> (Vec<(u16, u8)>, u8, u16) {
    (s1_set_samus_stand(), 3, 0xCD6D)
}

/// $D110 员体：SetSamusStand+$0306 dec+StopVertMovement+尾跳 $D144。
pub fn s1_d110_body(v0306: u8) -> (Vec<(u16, u8)>, u16) {
    let mut ev = s1_set_samus_stand();
    ev.push((0x0306, v0306.wrapping_sub(1)));
    (ev, 0xD144)
}

/// $D957 员体：SetSamusStand+$4B X 载+$0300[x]←1+$030F[x] eor $80 段。
pub fn s1_d957_body(x: u8, v030f_x: u8) -> Vec<(u16, u8)> {
    let mut ev = s1_set_samus_stand();
    ev.push((0x0300 + x as u16, 1));
    ev.push((0x030F + x as u16, v030f_x ^ 0x80));
    ev
}

#[cfg(test)]
mod s1_18_tests {
    use super::*;

    /// 四员转写形（SetSamusStand root 自入口+三壳）。
    #[test]
    fn s1_18_units_four() {
        let ev = s1_set_samus_stand();
        assert_eq!(ev.len(), 8);
        assert!(ev.contains(&(0x0305, 7)));
        assert!(ev.contains(&(0x0680, 8)));
        let (_, a, j) = s1_d04d_body();
        assert_eq!((a, j), (3, 0xCD6D));
        let (ev, j) = s1_d110_body(5);
        assert_eq!(j, 0xD144);
        assert!(ev.contains(&(0x0306, 4)));
        let ev = s1_d957_body(2, 0x0F);
        assert!(ev.contains(&(0x0302, 1)));
        assert!(ev.contains(&(0x0311, 0x8F)));
    }
}

