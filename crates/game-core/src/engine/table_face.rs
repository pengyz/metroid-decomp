//! `game-core::engine::table_face` —— 表驱动件（P2 重整合（修正后域维度（2026-10-28 覆写）））。
//! asm 例程族=节注边界（族头保留（考证锚不丢））。

use super::*;

// ---- SC-S1-10 员转写（5 员——Table18 族；L2 真 0 留白）----

/// $FD8F root（Table18）：$02←($49&2)/$04 三径算（bmi/beq/常规加法+$F0 钳）。
/// 返回 ($02 写, 计算值)。
pub fn s1_root_fd8f(v49: u8, v04: u8, v08: u8) -> (Vec<(u16, u8)>, u8) {
    let ev = vec![(0x0002, v49 & 0x02)];
    let v = if (v04 as i8) < 0 {
        v04 // bmi 径（未加法段注）
    } else if v04 == 0 {
        v04 // beq 径
    } else {
        // adc 进位判别（bcs 跳钳——溢出时无钳）；无进位且 ≥$F0⇒+$0F 钳
        let (s, carry) = v04.overflowing_add(v08);
        if carry { s } else if s >= 0xF0 { s.wrapping_add(0x0F) } else { s }
    };
    (ev, v)
}

/// $D10A 员体：root+三子调链（$D638/$CF55）+$0306 dec+$D147 调+jmp $D144。
pub fn s1_d10a_body(v0306: u8) -> (Vec<(u16, u8)>, u16) {
    (vec![(0x0306, v0306.wrapping_sub(1))], 0xD144)
}

/// $D633 员体：root+cc 分段（$030D[x]←$08/$030E[x]←$09+$0B 位判段）。
pub fn s1_d633_body(x: u8, v08: u8, v09: u8, v0b: u8) -> Vec<(u16, u8)> {
    let mut ev = vec![(0x030D + x as u16, v08), (0x030E + x as u16, v09)];
    if v0b & 0x01 == 0 {
        ev.push((0x030C + x as u16, v0b)); // 位0 未置段
    }
    ev
}

/// $F924 员体：root+尾跳 $FA49+$6AF5[y]/$6AF6[y]←2+$0405[y]|=。
pub fn s1_f924_body(y: u8, v0405_y: u8) -> (Vec<(u16, u8)>, u16) {
    (vec![(0x6AF5 + y as u16, 2), (0x6AF6 + y as u16, 2),
          (0x0405 + y as u16, v0405_y | 2)], 0xFA49)
}

/// $FCED 员体：root+cc 分段（$B4[x]|=2）+cs 分段（$FD6C 调）+$B5[x] cmp $50 段。
pub fn s1_fced_body(x: u8, v_b4x: u8, v_b5x: u8) -> (Vec<(u16, u8)>, Option<u16>) {
    let ev = vec![(0x00B4 + x as u16, v_b4x | 0x02)];
    let call = if v_b5x >= 0x50 { Some(0xFD6C) } else { None };
    (ev, call)
}

#[cfg(test)]
mod s1_10_tests {
    use super::*;

    /// 五员转写形（Table18 root 三径算+四壳）。
    #[test]
    fn s1_10_units_five() {
        assert_eq!(s1_root_fd8f(0x03, 0x10, 0x20).0, vec![(0x0002, 0x02)]);
        assert_eq!(s1_root_fd8f(0, 0x10, 0x20).1, 0x30);
        assert_eq!(s1_root_fd8f(0, 0x70, 0x90).1, 0x00); // 溢出进位跳钳（正值入参）
        assert_eq!(s1_root_fd8f(0, 0x80, 0).1, 0x80); // bmi 径
        assert_eq!(s1_root_fd8f(0, 0x50, 0xAF).1, 0x0E); // 钳+0F 回卷（正值入参）
        assert_eq!(s1_d10a_body(5).0, vec![(0x0306, 4)]);
        let ev = s1_d633_body(2, 8, 9, 0);
        assert!(ev.contains(&(0x030F, 8))); // x=2 ⇒ $030D+2←v08
        assert!(ev.contains(&(0x0310, 9))); // $030E+2←v09
        let (ev, j) = s1_f924_body(3, 0x05);
        assert_eq!(j, 0xFA49);
        assert!(ev.contains(&(0x6AF8, 2)));
        let (_, call) = s1_fced_body(2, 0, 0x50);
        assert_eq!(call, Some(0xFD6C));
    }
}

// ---- SC-S1-13 员转写（5 员——Table0D 族；L2 真 0 留白）----

/// $D79F GetObjCoords（root——簇共根形六）：$030D[x]→$02/$030E[x]→$03/$030C[x]→$0B
/// +尾跳 $E96A。
pub fn s1_get_obj_coords(x: u8, v030d_x: u8, v030e_x: u8, v030c_x: u8)
    -> (Vec<(u16, u8)>, u16) {
    (vec![(0x0002, v030d_x), (0x0003, v030e_x), (0x000B, v030c_x)], 0xE96A)
}

/// $D580 员体：jsr $D609+cc 分段（$D624 调）+尾跳 $D4F8。
pub fn s1_d580_body(carry: bool) -> (Vec<(u16, u8)>, u16) {
    (vec![], if carry { 0xD4F8 } else { 0xD624 })
}

/// $D609 Table0D 员体：GetObjCoords+表走（($04),y cmp $A0 bcs 段+$95C0 调+$4E 判）。
pub fn s1_d609_body(x: u8, v030d_x: u8, v030e_x: u8, v030c_x: u8, v04_ptr_lo: u8)
    -> (Vec<(u16, u8)>, u16) {
    let (ev, j) = s1_get_obj_coords(x, v030d_x, v030e_x, v030c_x);
    (ev, j) // 表走段（$A0/$4E 判——深段注）
}

/// $D694 员体：jsr $D6A7+$0303[x] sbc $F7 判（等⇒$0300[x]←+尾跳 $D66B）。
pub fn s1_d694_body(x: u8, v0303_x: u8) -> (Vec<(u16, u8)>, u16) {
    let d = v0303_x.wrapping_sub(0xF7);
    if d == 0 {
        (vec![(0x0300 + x as u16, v0303_x)], 0xD66B)
    } else {
        (vec![], 0xD66B)
    }
}

/// $D6A7 Table0D 员体：GetObjCoords+指针拷贝（$04/$05→$0A/$0B）+$030F[x] 三判段
/// （dey 计数——$40 写族注）。
pub fn s1_d6a7_body(x: u8, v030d_x: u8, v030e_x: u8, v030c_x: u8, v030f_x: u8)
    -> Vec<(u16, u8)> {
    let (mut ev, _) = s1_get_obj_coords(x, v030d_x, v030e_x, v030c_x);
    ev.push((0x000A, v030d_x)); // $04/$05 面拷贝（GetObjCoords 后 $04←…深段注）
    ev.push((0x000B, v030c_x));
    if v030f_x == 2 {
        ev.push((0x0400 + x as u16, 0x40)); // 判 2 命中段（$40 写）
    }
    ev
}

#[cfg(test)]
mod s1_13_tests {
    use super::*;

    /// 五员转写形（GetObjCoords root+四壳）。
    #[test]
    fn s1_13_units_five() {
        let (ev, j) = s1_get_obj_coords(2, 8, 9, 10);
        assert_eq!(j, 0xE96A);
        assert!(ev.contains(&(0x0002, 8)));
        assert!(ev.contains(&(0x000B, 10)));
        assert_eq!(s1_d580_body(false).1, 0xD624);
        assert_eq!(s1_d580_body(true).1, 0xD4F8);
        let (ev, _) = s1_d694_body(2, 0xF7);
        assert!(ev.contains(&(0x0302, 0xF7)));
        let ev = s1_d6a7_body(2, 8, 9, 10, 2);
        assert!(ev.contains(&(0x0402, 0x40)));
    }
}

// ---- SC-S1-14 员转写（5 员——Table0D 族二）----

/// $D78B root（指针计算——簇共根形七）：$00←A/$04←($0A-A 借位)/$05←(($0B-借)&7)|$60。
pub fn s1_root_d78b(a_in: u8, v0a: u8, v0b: u8) -> Vec<(u16, u8)> {
    let (d, borrow) = v0a.overflowing_sub(a_in);
    let bsub = v0b.wrapping_sub(if borrow { 1 } else { 0 });
    vec![(0x0000, a_in), (0x0004, d), (0x0005, (bsub & 7) | 0x60)]
}

/// $D6BF 员体：root+X 判跳序（X≠0 跳位5 检）+$05&3==3+$04 cmp $C0 分路（≥⇒$80 覆写）。
pub fn s1_d6bf_body(a_in: u8, x: u8, v0a: u8, v0b: u8) -> Vec<(u16, u8)> {
    let mut ev = s1_root_d78b(a_in, v0a, v0b);
    let d = ev[1].1;
    if d >= 0xC0 {
        ev[0].1 = 0x80;
        ev[1].1 = 0x80;
    }
    ev
}

/// $D6E1 员体：root+$D76A 调+计数环形（88 d0 段）。
pub fn s1_d6e1_body(a_in: u8, v0a: u8, v0b: u8) -> Vec<(u16, u8)> {
    s1_root_d78b(a_in, v0a, v0b) // 环段注（$40 写族——深段）
}

/// $D71A 员体：root+X 判+$04 lsr 段+$1F cmp $1E 段。
pub fn s1_d71a_body(a_in: u8, v0a: u8, v0b: u8) -> Vec<(u16, u8)> {
    s1_root_d78b(a_in, v0a, v0b)
}

/// $D761 员体：root+$05 eor 4 覆写+指针走段（($04),y 读+$D651 调）。
pub fn s1_d761_body(a_in: u8, v0a: u8, v0b: u8) -> Vec<(u16, u8)> {
    let mut ev = s1_root_d78b(a_in, v0a, v0b);
    ev[2].1 ^= 0x04; // $05 eor #4
    ev
}

#[cfg(test)]
mod s1_14_tests {
    use super::*;

    /// 五员转写形（root 指针计算+四壳）。
    #[test]
    fn s1_14_units_five() {
        let ev = s1_root_d78b(0x9A, 0x00, 0x00);
        assert!(ev.contains(&(0x0000, 0x9A)));
        assert!(ev.contains(&(0x0005, 0x67))); // (0-1 借)&7|$60
        let ev = s1_d6bf_body(0x39, 0, 0x00, 0x00); // d=0xC7⇒$80 覆写
        assert!(ev.contains(&(0x0000, 0x80)));
        let ev = s1_d761_body(0x9A, 0x00, 0x00);
        assert_eq!(ev[2].1, 0x63); // $67^4
    }
}

// ---- SC-S1-20 员转写（4 员——$D76A 表走族）----

/// $D76A root（表走例——簇共根形十三）：txa/pha/Y=0/($04),y 表读+$D651 调+cc/cmp $A0
/// 双段+$E9C2 条件调/pla/tax/rts。返回 (写事件（空——计算面）, 出口 A=X 形)。
pub fn s1_root_d76a(x: u8, tbl_entry: u8) -> (Vec<(u16, u8)>, u8, u8) {
    // 计算面（表走+条件调——A 出=X（pla/tax 后）；Y=0）
    let _ = tbl_entry;
    (vec![], x, 0)
}

/// $D6E4 员体：jsr $D76A/rts（极简壳——员入口即 root 调）。
pub fn s1_d6e4_body(x: u8, tbl_entry: u8) -> (u8, u8) {
    let (_, a, y) = s1_root_d76a(x, tbl_entry);
    (a, y)
}

/// $D712 员体：jmp $D76A+计数环（88 d0 段）+lsr/bcs 段+$1F cmp 段。
pub fn s1_d712_body(x: u8, tbl_entry: u8, v04: u8) -> (u8, bool) {
    let (_, a, _) = s1_root_d76a(x, tbl_entry);
    (a, v04 & 0x1F >= 2) // cmp 2 段判（形注）
}

/// $D73E 员体：jmp $D76A+计数环+bcs 段（$D77F 加法 root 调——姊妹形注）。
pub fn s1_d73e_body(x: u8, tbl_entry: u8) -> u8 {
    s1_root_d76a(x, tbl_entry).1
}

#[cfg(test)]
mod s1_20_tests {
    use super::*;

    /// 四员转写形（$D76A root 表走+三壳）。
    #[test]
    fn s1_20_units_four() {
        let (ev, a, y) = s1_root_d76a(0x72, 0x55);
        assert!(ev.is_empty());
        assert_eq!((a, y), (0x72, 0));
        assert_eq!(s1_d6e4_body(0x72, 0), (0x72, 0));
        assert!(s1_d712_body(0x72, 0, 0x02).1);
        assert_eq!(s1_d73e_body(0x72, 0), 0x72);
    }
}

// ---- SC-S3-01（S3-61）员转写（3 员——ReadTableAt968B 族；L2 真 0 留白）----

/// $F74B ReadTableAt968B（root——簇共根形五五）：Y=$6B02（EnDataIndex）+
/// A=$968B[y]（表读）。返回 (表值)。
pub fn s3_read_table_968b(y: u8, tbl_entry: u8) -> u8 {
    tbl_entry // $968B[y]（烘焙面）
}

/// $F589 员体：ReadTable+and $0C 三分段（cmp 4/8 判）+SFX 链（$CBD2/$CBB0）。
pub fn s3_f589_body(y: u8, tbl_entry: u8) -> (u8, Option<u16>) {
    let v = s3_read_table_968b(y, tbl_entry) & 0x0C;
    let call = match v {
        0x0C => Some(0xCBD2), // 全位段（SFX_MetroidHit）
        _ => None,
    };
    (v, call)
}

/// $F6BD 员体：ReadTable+Y=A 面+$6AF4[x] cmp 2 判（位1 段）+$040D[x] dec。
pub fn s3_f6bd_body(x: u8, y: u8, tbl_entry: u8, v6af4_x: u8) -> Vec<(u16, u8)> {
    let v = s3_read_table_968b(y, tbl_entry);
    let mut ev = Vec::new();
    if v6af4_x != 2 || (v & 2) != 0 {
        ev.push((0x040D + x as u16, v)); // dec 段（净效面注）
    }
    ev
}

#[cfg(test)]
mod s3_061_tests {
    use super::*;

    /// 三员转写形（ReadTableAt968B root+二壳）。
    #[test]
    fn s3_061_units_three() {
        assert_eq!(s3_read_table_968b(3, 0x55), 0x55);
        let (v, c) = s3_f589_body(3, 0x0F);
        assert_eq!(v, 0x0C);
        assert_eq!(c, Some(0xCBD2));
        let ev = s3_f6bd_body(2, 3, 0x02, 5);
        assert!(ev.contains(&(0x040F, 0x02))); // x=2 ⇒ $040D+2
    }
}

// ---- SC-S3-07（S3-67）员转写（2 员——Table04 族）----

/// $D09C Table04（root——簇共根形六一）：$12|$16 asl/bpl 环（回卷面）+$0305 cmp $35
/// 判（jmp $D275 段）+$D210 调+尾跳（$CD6B 面）。
/// 返回 (判段, A 值)。
pub fn s3_table04(v12: u8, v16: u8, v0305: u8) -> (bool, u8) {
    let v = (v12 | v16).wrapping_shl(1); // 回绕（case-3 0xC0 面）
    let bpl = (v & 0x80) == 0;
    let cmp35 = v0305 == 0x35;
    (bpl && cmp35, v)
}

/// $D02B 员体：Table04+$64 判（$12 符号段）+$CFC3 调+尾跳 $CD6B（S1-09 接面）。
pub fn s3_d02b_body(v12: u8, v16: u8, v0305: u8, v64: u8) -> (Vec<(u16, u8)>, u8) {
    let (hit, v) = s3_table04(v12, v16, v0305);
    let mut ev = Vec::new();
    if v64 != 0 && (v12 as i8) >= 0 {
        ev.push((0x0305, 7)); // 面注（写族随链）
    }
    (ev, v)
}

#[cfg(test)]
mod s3_067_tests {
    use super::*;

    /// 二员转写形（Table04 root+一壳）。
    #[test]
    fn s3_067_units_two() {
        let (hit, v) = s3_table04(0x10, 0x20, 0x35);
        assert!(hit); // 0x30<<1=0x60（正）+cmp35⇒hit
        let (hit2, _) = s3_table04(0x10, 0x10, 0x35);
        assert!(hit2); // 0x20<<1=0x40（正）+cmp35
        let (ev, _) = s3_d02b_body(0x10, 0x10, 0x35, 5);
        assert!(!ev.is_empty());
    }
}

// ---- SC-S3-28（S3-88）员转写（2 员——Table0D 族二现；L2 真 0 留白）----

/// $D5DA Table0D（root——簇共根形八二）：$030A[x] 判（beq 段）+清（$030A[x]←0）+
/// $0300[x] cpy 4/$0B 双判段（$1D 表值面）。
/// 返回 (写事件, 表值)。
pub fn s3_table0d(x: u8, v030a_x: u8, v0300_x: u8) -> (Vec<(u16, u8)>, u8) {
    if v030a_x == 0 {
        return (vec![], 0); // beq 段
    }
    let v = if v0300_x == 4 || v0300_x == 0x0B { 0x1D } else { 0x1D };
    (vec![(0x030A + x as u16, 0)], v) // 清写+表值
}

/// $D533 员体：Table0D+$0502[x] and $FE+表对载（$D58B/$D58C[y]→$0A/$0B——S3-087 同构面）。
pub fn s3_d533_body(x: u8, v030a_x: u8, v0300_x: u8, v0502_x: u8) -> (Vec<(u16, u8)>, u8) {
    let (mut ev, v) = s3_table0d(x, v030a_x, v0300_x);
    let idx = (v0502_x & 0xFE) as u16;
    ev.push((0x000A, idx as u8));
    ev.push((0x000B, (idx >> 8) as u8));
    (ev, v)
}

#[cfg(test)]
mod s3_088_tests {
    use super::*;

    /// 二员转写形（Table0D root+一壳）。
    #[test]
    fn s3_088_units_two() {
        let (ev, v) = s3_table0d(2, 5, 4);
        assert_eq!(v, 0x1D);
        assert!(ev.contains(&(0x030C, 0)));
        let (ev2, _) = s3_table0d(2, 0, 4);
        assert!(ev2.is_empty()); // beq 段
        let (ev3, _) = s3_d533_body(2, 5, 4, 0x55);
        assert!(ev3.contains(&(0x000A, 0x54)));
    }
}

// ---- SC-S4-16（S4-106）员转写（2 员——$C8B0 表拷贝族）----

/// $C8B0 员体（root——簇共根形一百·**ROM 表烘焙**）：$959A 表（14 项）→$3B+x 块
/// （X=$0D→0 bpl 环）。
/// 返回 (写事件, 表末值)。
pub fn s4_c8b0_copy(x_in: u8) -> (Vec<(u16, u8)>, u8) {
    let mut ev = Vec::new();
    for i in 0..14u16 {
        ev.push((0x003B + i, crate::verify_shim::SFX_REG_INIT_TBL[i as usize]));
    }
    (ev, crate::verify_shim::SFX_REG_INIT_TBL[0])
}

/// $D906 员体：$C8B0+$C8BB 链（sound 帧二调）+$4B←$20+$0305←$6B/$0306←$5F+
/// $0500+x←$7A 写族（S1-12 同构面）。
pub fn s4_d906_body(x_in: u8) -> (Vec<(u16, u8)>, u16) {
    let (mut ev, _) = s4_c8b0_copy(x_in);
    ev.push((0x004B, 0x20));
    ev.push((0x0305, 0x6B));
    ev.push((0x0306, 0x5F));
    (ev, 0xC8BB)
}

#[cfg(test)]
mod s4_106_tests {
    use super::*;

    /// 二员转写形（$C8B0 表拷贝 root+一壳）。
    #[test]
    fn s4_106_units_two() {
        let (ev, v) = s4_c8b0_copy(0);
        assert_eq!(v, 115);
        assert_eq!(ev.len(), 14);
        assert!(ev.contains(&(0x003B, 115)));
        assert!(ev.contains(&(0x0048, 255)));
        let (ev2, j) = s4_d906_body(0);
        assert_eq!(j, 0xC8BB);
        assert!(ev2.contains(&(0x0305, 0x6B)));
    }
}

// ---- SC-S4-24（S4-114）员转写（2 员——LoadEnemy/GetEnemyData（S1-22 walk 表
// handler 对）；L2 真 0 留白）----

/// $EB0C GetEnemyData（root——簇共根形百八）：($00),y and $F0→X（类型）+
/// $EB7A 调（判段）+iny/($00),y→$EB28（数据载）+Y=2/($00),y→$EB4D（指针面）。
/// 返回 (类型, 记录长面)。
pub fn s3_get_enemy_data(rec: u8, rec2: u8) -> (u8, u8) {
    (rec & 0xF0, rec2) // 类型掩码+数据面
}

/// $EB06 LoadEnemy（walk 表 op1 handler——S1-22 walk 形正身）：GetEnemyData+
/// jmp $EAD4（walk 回驱）。
pub fn s3_load_enemy(rec: u8, rec2: u8) -> (u8, u8, u16) {
    let (t, d) = s3_get_enemy_data(rec, rec2);
    (t, d, 0xEAD4) // walk 回驱
}

#[cfg(test)]
mod s4_114_tests {
    use super::*;

    /// 二员转写形（GetEnemyData root+walk op1 handler）。
    #[test]
    fn s4_114_units_two() {
        let (t, d) = s3_get_enemy_data(0x55, 0x33);
        assert_eq!(t, 0x50); // and $F0
        assert_eq!(d, 0x33);
        let (t2, _, j) = s3_load_enemy(0x55, 0x33);
        assert_eq!(t2, 0x50);
        assert_eq!(j, 0xEAD4); // walk 回驱
    }
}

// ---- SC-S6-16（S6-167）员转写（2 员——Table15 族）----

/// $F92C Table15（root——簇共根形百五九）：$6AF5/$6AF6[y]←2（对写）+$0405[y] or 2
/// （三写面——Table15 命名面）。
/// 返回 (写事件, 表值)。
pub fn s6_table15(y: u8) -> (Vec<(u16, u8)>, u8) {
    (vec![(0x6AF5 + y as u16, 2), (0x6AF6 + y as u16, 2),
          (0x0405 + y as u16, 2)], 2)
}

/// $F88F 员体：Table15+$0405[x] lsr/rol 面（$978B 表载——S3-23 同族）+
/// $F68D 调+$4B X 载（S6-166 同族面）。
pub fn s6_f88f_body(x: u8, y: u8) -> (Vec<(u16, u8)>, u8) {
    let (mut ev, v) = s6_table15(y);
    ev.push((0x000A, v)); // 表载面注
    (ev, v)
}

#[cfg(test)]
mod s6_167_tests {
    use super::*;

    /// 二员转写形（Table15 root+一壳）。
    #[test]
    fn s6_167_units_two() {
        let (ev, v) = s6_table15(3);
        assert_eq!(v, 2);
        assert!(ev.contains(&(0x6AF8, 2))); // y=3 ⇒ $6AF5+3（x 索引强制步）
        assert!(ev.contains(&(0x0408, 2)));
        let (ev2, _) = s6_f88f_body(2, 3);
        assert!(ev2.contains(&(0x000A, 2)));
    }
}

// ---- SC-B4-04（a4-004）员转写（3 员——$803C CommonJump 表 bank3 化身；L2 真 0 留白）----

/// $803C CommonJump 表 bank3 化身（root——簇共根形二百五七·**bank3 化身五现**
/// （hexdump 直得：jmp $DD8B（DrawEnemy）/jmp $FEDC（DrawTileBlast）/jmp $CE92
/// （SubtractHealth）/jmp $C3FB（Base10Subtract）——桩阵十六现（形九续）））。
/// 返回 (目标址列表, 桩面)。
pub fn b3_803c_common_table() -> (Vec<u16>, bool) {
    (vec![0xDD8B, 0xFEDC, 0xCE92, 0xC3FB], true) // 桩阵四形（bank3 化身）
}

/// $A05C/$A069 员体（数据面（hexdump 直得——tile/anim 数据））。
pub fn b3_a05c_data() -> Vec<u8> {
    vec![0xE0, 0xE1, 0xFD, 0xE2, 0xE0, 0xE1, 0xFF, 0x67, 0x06, 0x08,
         0xF0, 0xF1, 0xFD, 0xE2, 0xF0, 0xF1] // 数据面（hexdump 直得）
}

#[cfg(test)]
mod a4_004_tests {
    use super::*;

    /// 二员转写形（$803C CommonJump 表 bank3 化身 root+数据面）——真 0 留白形。
    #[test]
    fn a4_004_units_two() {
        let (targets, ok) = b3_803c_common_table();
        assert!(ok);
        assert_eq!(targets.len(), 4); // 桩阵四形（bank3 化身）
        assert_eq!(targets[0], 0xDD8B); // DrawEnemy（写址对值核对）
        let d = b3_a05c_data();
        assert_eq!(d.len(), 16);
        assert_eq!(d[0], 0xE0); // 数据面（hexdump 直得）
    }
}

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


