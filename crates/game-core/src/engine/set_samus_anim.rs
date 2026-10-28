//! `game-core::engine::set_samus_anim` —— asm 例程族（首例程 SetSamusAnim（07:CF6B）（P2 批 1 二次拆（一族一文件（文件名=m1disasm 标签（设计稿 §2））））。

use super::*;

// ---- SC-S1-09 员转写（5 员——SetSamusAnim 族）----

/// $CF6B SetSamusAnim：$0305←A/$0306←A/$0304←0（A 归零出口）。
/// 【P1 重复消除（2026-10-28）】唯一真相源=crate::samus::animation::set_samus_animation
/// （域产品层）——本件改**薄转发**（s1_* 消费方随迁（签名同（Vec 写面形）））。
pub fn set_samus_animation(a_in: u8) -> (Vec<(u16, u8)>, u8) {
    crate::samus::animation::set_samus_animation(a_in)
}

/// $D07A 员体：jmp $D088（尾形）+$0305 读 vs $D096[y] 表比对段。
pub fn s1_d07a_body(a_in: u8, v0305: u8, tbl_entry: u8) -> (Vec<(u16, u8)>, u8) {
    let v = if v0305 == tbl_entry { tbl_entry } else { a_in };
    let (ev, a) = set_samus_animation(v);
    (ev, a)
}

/// $D088 员体：SetSamusAnim+$0304←8+$4D←Y/$0309←X（写族）。
pub fn s1_d088_body(a_in: u8, x: u8, y: u8) -> Vec<(u16, u8)> {
    let (mut ev, _) = set_samus_animation(a_in);
    ev.push((0x0304, 8));
    ev.push((0x004D, y));
    ev.push((0x0309, x));
    ev
}

/// $D12B 员体：SetSamusAnim+$4D X 载+$CCB7/$CF2E/$D150 子调链+$14 位判段。
pub fn s1_d12b_body(a_in: u8) -> (Vec<(u16, u8)>, u16) {
    let (ev, _) = set_samus_animation(a_in);
    (ev, 0xCCB7) // 子调链首
}

/// $D8B4 员体：SetSamusAnim→尾跳 $D806（回驱动器——SC-S1-01 族形）。
pub fn s1_d8b4_body(a_in: u8) -> (Vec<(u16, u8)>, u16) {
    let (ev, _) = set_samus_animation(a_in);
    (ev, 0xD806)
}

#[cfg(test)]
mod s1_09_tests {
    use super::*;

    /// 五员转写形（SetSamusAnim 3 写+四壳）。
    #[test]
    fn s1_09_units_five() {
        let (ev, a) = set_samus_animation(0x4F);
        assert_eq!(a, 0);
        assert!(ev.contains(&(0x0305, 0x4F)));
        assert!(ev.contains(&(0x0304, 0)));
        let (ev, _) = s1_d07a_body(0x4F, 5, 5);
        assert!(ev.contains(&(0x0305, 5)));
        let ev = s1_d088_body(0x4F, 2, 3);
        assert!(ev.contains(&(0x0304, 8)));
        assert!(ev.contains(&(0x004D, 3)));
        assert_eq!(s1_d12b_body(0x4F).1, 0xCCB7);
        assert_eq!(s1_d8b4_body(0x4F).1, 0xD806);
    }
}

