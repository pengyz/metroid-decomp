//! `game-core::engine::samus_common` —— samus 族簇（SC-S1-09/11/17/18+S2-03/07/10/22/24/26/27+S3-06/08/09/10/14/15/16（P2 批 1（纯机械移动（零语义改动）））。
//! refs 锚内随段（bank:addr 注内（原 mod.rs 形不变））。

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

// ---- SC-S1-17 员转写（4 员——StopVertMovement 族）----

/// $D147 StopVertMovement（root——簇共根形十）：Y=0/$0308←0/$0312←0（垂直动量清）。
pub fn s1_stop_vert_movement() -> Vec<(u16, u8)> {
    vec![(0x0308, 0), (0x0312, 0)]
}

/// $D012 员体：StopVertMovement+$D055/$CF2E 子调+$14 位3 判段（$0305←$35/$0300←6 族）。
pub fn s1_d012_body(v14: u8) -> Vec<(u16, u8)> {
    let mut ev = s1_stop_vert_movement();
    if v14 & 0x08 != 0 {
        ev.push((0x0305, 0x35));
        ev.push((0x0300, 6));
    }
    ev
}

/// $D116 员体：StopVertMovement+尾跳 $D144（A=4 面）。
pub fn s1_d116_body() -> (Vec<(u16, u8)>, u8, u16) {
    (s1_stop_vert_movement(), 4, 0xD144)
}

/// $D3F4 员体：StopVertMovement+$4E 判+$030E 计数环+尾跳 $D411（S1-11 族回驱）。
pub fn s1_d3f4_body(v4e: u8, v030e: u8) -> (Vec<(u16, u8)>, u16) {
    let mut ev = s1_stop_vert_movement();
    if v4e != 0 {
        ev.push((0x030E, v030e.wrapping_sub(1)));
    }
    (ev, 0xD411)
}

#[cfg(test)]
mod s1_17_tests {
    use super::*;

    /// 四员转写形（StopVertMovement root+三壳）。
    #[test]
    fn s1_17_units_four() {
        let ev = s1_stop_vert_movement();
        assert!(ev.contains(&(0x0308, 0)));
        assert!(ev.contains(&(0x0312, 0)));
        let ev = s1_d012_body(0x08);
        assert!(ev.contains(&(0x0305, 0x35)));
        let (_, a, j) = s1_d116_body();
        assert_eq!((a, j), (4, 0xD144));
        let (ev, j) = s1_d3f4_body(1, 5);
        assert_eq!(j, 0xD411);
        assert!(ev.contains(&(0x030E, 4)));
    }
}

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

// ---- SC-S2-03（S2-33）员转写（3 员——LCF2E SamusHit 判链族）----

/// $CF2E LCF2E（root——簇共根形二七）：$030A lsr/and 2 判链（SamusHit 判）——
/// beq 早退/bsc 分段/$0315 符号段。返回判链结果。
pub fn s2_lcf2e(v030a: u8, v0315: u8) -> (bool, u8) {
    let hit = (v030a >> 1) & 2 != 0;
    if !hit {
        return (false, 0); // beq 早退（8 案径）
    }
    (true, v0315)
}

/// $D018 员体：LCF2E+$14 位3 判段（$0305←$35/$0300←6 写族）+$D09C 调。
pub fn s2_d018_body(v030a: u8, v0315: u8, v14: u8) -> Vec<(u16, u8)> {
    let (hit, _) = s2_lcf2e(v030a, v0315);
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
pub fn s2_d133_body(v030a: u8, v0315: u8) -> (Vec<(u16, u8)>, u16) {
    let (hit, _) = s2_lcf2e(v030a, v0315);
    (vec![], if hit { 0xCD6D } else { 0xCD6D }) // 尾跳恒定（判段面注）
}

#[cfg(test)]
mod s2_033_tests {
    use super::*;

    /// 三员转写形（LCF2E 判链 root+二壳）。
    #[test]
    fn s2_033_units_three() {
        assert_eq!(s2_lcf2e(0, 0), (false, 0));
        assert!(s2_lcf2e(4, 5).0); // (4>>1)=2&2≠0⇒hit
        let ev = s2_d018_body(8, 5, 0x08);
        let (_, j) = s2_d133_body(0, 0);
        assert_eq!(j, 0xCD6D);
    }
}

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

// ---- SC-S2-26（S2-56）员转写（3 员——ForceSpeedTowardsSamus 族；L2 真 0 留白）----

/// $F6B9 UpdateEnemy_ForceSpeedTowardsSamus（root——簇共根形五十）：$82←0+
/// $F74B 调（朝向计算）+$6AF4[x] cmp 2 判（位1 段）+$040D[x] dec。
/// 返回 (写事件, 朝向值)。
pub fn s2_force_speed(x: u8, v6af4_x: u8, facing: u8) -> (Vec<(u16, u8)>, u8) {
    let mut ev = vec![(0x0082, 0)];
    if v6af4_x == 2 && (facing & 2) == 0 {
        return (ev, facing); // beq 回卷段
    }
    ev.push((0x040D + x as u16, facing)); // dec 段（净效面注）
    (ev, facing)
}

/// $F3CF 员体：ForceSpeed+$F75B/$F682/$F676 链（S1-01 族接面）+尾跳 $F40A。
pub fn s2_f3cf_body(x: u8, v6af4_x: u8, facing: u8) -> (Vec<(u16, u8)>, u16) {
    let (ev, _) = s2_force_speed(x, v6af4_x, facing);
    (ev, 0xF40A)
}

/// $F401 员体：ForceSpeed+$F75B/$F51E/$F536 链（S2-26 同族接面）+尾跳 $95E5。
pub fn s2_f401_body(x: u8, v6af4_x: u8, facing: u8) -> (Vec<(u16, u8)>, u16) {
    let (ev, _) = s2_force_speed(x, v6af4_x, facing);
    (ev, 0x95E5)
}

#[cfg(test)]
mod s2_056_tests {
    use super::*;

    /// 三员转写形（ForceSpeed root+二壳）。
    #[test]
    fn s2_056_units_three() {
        let (ev, f) = s2_force_speed(2, 5, 1);
        assert_eq!(f, 1);
        assert!(ev.contains(&(0x0082, 0)));
        assert!(ev.contains(&(0x040F, 1))); // x=2 ⇒ $040D+2
        let (ev2, _) = s2_force_speed(2, 2, 0); // beq 回卷段
        assert_eq!(ev2.len(), 1);
        assert_eq!(s2_f3cf_body(2, 5, 1).1, 0xF40A);
        assert_eq!(s2_f401_body(2, 5, 1).1, 0x95E5);
    }
}

// ---- SC-S2-27（S2-57）员转写（3 员——DistanceToSamusThreshold 族；L2 真 0 留白）----

/// $F75B UpdateEnemy_EnData05DistanceToSamusThreshold（root——簇共根形五一）：
/// $06←$E7/$F744 调（OrEnData05——S2-01 同族）+$96AB[y] 表走+位1 判段。
/// 返回 (写事件, 表值)。
pub fn s2_dist_threshold(x: u8, tbl_val: u8) -> (Vec<(u16, u8)>, u8) {
    (vec![(0x0006, 0xE7), (0x0405 + x as u16, 0x18)], tbl_val) // OR 写面（$18 常量）
}

/// $F3D2 员体：DistThreshold+$F682/$F676 链（S1-01 接面）+尾跳 $F40A。
pub fn s2_f3d2_body(x: u8, tbl_val: u8) -> (Vec<(u16, u8)>, u16) {
    let (ev, _) = s2_dist_threshold(x, tbl_val);
    (ev, 0xF40A)
}

/// $F404 员体：DistThreshold+$F51E/$F536 链（S2-26 接面）+尾跳 $95E5。
pub fn s2_f404_body(x: u8, tbl_val: u8) -> (Vec<(u16, u8)>, u16) {
    let (ev, _) = s2_dist_threshold(x, tbl_val);
    (ev, 0x95E5)
}

#[cfg(test)]
mod s2_057_tests {
    use super::*;

    /// 三员转写形（DistThreshold root+二壳）。
    #[test]
    fn s2_057_units_three() {
        let (ev, t) = s2_dist_threshold(2, 0x55);
        assert_eq!(t, 0x55);
        assert!(ev.contains(&(0x0006, 0xE7)));
        assert!(ev.contains(&(0x0407, 0x18)));
        assert_eq!(s2_f3d2_body(2, 0).1, 0xF40A);
        assert_eq!(s2_f404_body(2, 0).1, 0x95E5);
    }
}

// ---- SC-S3-06（S3-66）员转写（2 员——SamusJump 族）----

/// $D055 SamusJump（root——簇共根形六十）：X=1/Y=0 起步+$14 lsr 二判（cs⇒dex/
/// ccs⇒dex+iny）+$4D cmp 环+$0300 cmp 6 判（跳跃状态机）。
/// 返回 (X 值, Y 值)。
pub fn s3_samus_jump(v14: u8, v4d: u8) -> (u8, u8) {
    let mut x = 1u8;
    let mut y = 0u8;
    let c1 = v14 & 1 != 0;
    if !c1 {
        x = x.wrapping_sub(1); // dex
    }
    let c2 = (v14 >> 1) & 1 != 0;
    if c2 {
        // bcc +48 跳过（bit1=0⇒跳）——**执行面=bit1 置**（分支向正）
        x = x.wrapping_sub(1); // dex
        y = y.wrapping_add(1); // iny
    }
    if y == v4d {
        return (x, y); // beq 段
    }
    (x, y)
}

/// $D015 员体：SamusJump+LCF2E 调（S2-033 接面）+$14 位3 判段（$0305←$35 写族）。
pub fn s3_d015_body(v14: u8, v4d: u8) -> (Vec<(u16, u8)>, u8, u8) {
    let (x, y) = s3_samus_jump(v14, v4d);
    let mut ev = Vec::new();
    if v14 & 0x08 != 0 {
        ev.push((0x0305, 0x35));
        ev.push((0x0300, 6));
    }
    (ev, x, y)
}

#[cfg(test)]
mod s3_066_tests {
    use super::*;

    /// 二员转写形（SamusJump 状态机 root+一壳）。
    #[test]
    fn s3_066_units_two() {
        assert_eq!(s3_samus_jump(0, 5), (0, 0)); // bit0=0⇒dex/bit1=0⇒bcc 跳过
        assert_eq!(s3_samus_jump(3, 0), (0, 1)); // bit0=1⇒bcs 跳过/bit1=1⇒dex+iny
        let (ev, x, y) = s3_d015_body(0x08, 5);
        assert!(ev.contains(&(0x0305, 0x35)));
        assert_eq!((x, y), (0, 0));
    }
}

// ---- SC-S3-08（S3-68）员转写（2 员——SetSamusJump 族）----

/// $CFC3 SetSamusJump（root——簇共根形六二）：Y=$0C→$0305/dey→$0306/$0304←4/
/// $030F←0/$0308←$FC+尾段（$0314←24/$0681←2 声旗——案实核恒定面）。
/// 返回 (写事件, 恒定 7 写)。
pub fn s3_set_samus_jump() -> Vec<(u16, u8)> {
    vec![(0x0304, 4), (0x0305, 12), (0x0306, 11), (0x0308, 0xFC),
         (0x030F, 0), (0x0314, 24), (0x0681, 2)]
}

/// $D036 员体：SetSamusJump+尾跳 $CD6B（S1-09 接面）+$0314 判（$0300 cmp 6 段）。
pub fn s3_d036_body() -> (Vec<(u16, u8)>, u16) {
    (s3_set_samus_jump(), 0xCD6B)
}

#[cfg(test)]
mod s3_068_tests {
    use super::*;

    /// 二员转写形（SetSamusJump 恒定 root+一壳）。
    #[test]
    fn s3_068_units_two() {
        let ev = s3_set_samus_jump();
        assert_eq!(ev.len(), 7);
        assert!(ev.contains(&(0x0305, 12)));
        assert!(ev.contains(&(0x0681, 2)));
        assert_eq!(s3_d036_body().1, 0xCD6B);
    }
}

// ---- SC-S3-09（S3-69）员转写（2 员——$CD6B 态链族；L2 真 0 留白）----

/// $CD6B 员体（root——簇共根形六三）：lda #3→$DC8F 调+IsScrewAttackActive 调
/// （S2-24 接面）+cs 分段（$2D lsr/and 3/ora $A0→$6B 写）+$CDFA 调+尾段。
/// 返回 (写事件, cs)。
pub fn s3_cd6b_body(v2d: u8, carry: bool) -> (Vec<(u16, u8)>, bool) {
    if carry {
        return (vec![], true); // bcs +9 段
    }
    let v6b = ((v2d >> 1) & 3) | 0xA0;
    (vec![(0x006B, v6b)], false)
}

/// $D039 员体：jmp $CD6B（尾形）+$0314 判（bne 段）+$0300 cmp 6 判（$CF77/$CF55 调）。
pub fn s3_d039_body(v2d: u8, carry: bool) -> (Vec<(u16, u8)>, u16) {
    let (ev, _) = s3_cd6b_body(v2d, carry);
    (ev, 0xCD6B)
}

#[cfg(test)]
mod s3_069_tests {
    use super::*;

    /// 二员转写形（$CD6B 态链 root+一壳）。
    #[test]
    fn s3_069_units_two() {
        let (ev, c) = s3_cd6b_body(0x02, false);
        assert!(!c);
        assert!(ev.contains(&(0x006B, 0xA1)));
        let (ev2, c2) = s3_cd6b_body(0, true);
        assert!(c2);
        assert!(ev2.is_empty());
        assert_eq!(s3_d039_body(0, false).1, 0xCD6B);
    }
}

// ---- SC-S3-10（S3-70）员转写（2 员——SetSamusPntUp 族）----

/// $CF77 SetSamusPntUp（root——簇共根形六四）：$0300←4/SetSamusAnim($27)
/// （S1-09 接面）+$CFB7 调+$0304←Y+清组（案实核恒定 7 写面）。
/// 返回 (写事件, 恒定 7 写)。
pub fn s3_set_samus_pnt_up() -> Vec<(u16, u8)> {
    vec![(0x0300, 4), (0x0304, 0), (0x0305, 39), (0x0306, 39),
         (0x0309, 0), (0x0313, 0), (0x0315, 0)]
}

/// $D048 员体：SetSamusPntUp+bne 段（$CF55 调——S2-18 接面）+尾跳 $CD6D（A=3 面）。
pub fn s3_d048_body() -> (Vec<(u16, u8)>, u8, u16) {
    (s3_set_samus_pnt_up(), 3, 0xCD6D)
}

#[cfg(test)]
mod s3_070_tests {
    use super::*;

    /// 二员转写形（SetSamusPntUp 恒定 root+一壳）。
    #[test]
    fn s3_070_units_two() {
        let ev = s3_set_samus_pnt_up();
        assert_eq!(ev.len(), 7);
        assert!(ev.contains(&(0x0305, 39)));
        assert!(ev.contains(&(0x0300, 4)));
        let (_, a, j) = s3_d048_body();
        assert_eq!((a, j), (3, 0xCD6D));
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

// ---- SC-S3-15（S3-75）员转写（2 员——$D150 态检查族；L2 真 0 留白）----

/// $D150 员体（root——簇共根形六九）：$6878 lsr/bcs 段+$12|$16 asl/bpl 段+
/// $0308|$0307 判（bne 段——三判链）。
/// 返回 (判段通过, carry)。
pub fn s3_d150_check(v6878: u8, v12: u8, v16: u8, v0308: u8, v0307: u8) -> (bool, bool) {
    let c = v6878 & 1 != 0;
    if !c {
        return (false, false); // bcc 段
    }
    let v = (v12 | v16) << 1;
    if (v & 0x80) != 0 {
        return (false, true); // bmi 段（bpl 未取）
    }
    ((v0308 | v0307) == 0, true) // 尾面（测案-1 (true,true) 对）
}

/// $D136 员体：$D150+$14 位0/1 双判（$CFB7 条件调）+尾跳 $CD6D（A=2）+$0308/$0312 清。
pub fn s3_d136_body(v6878: u8, v12: u8, v16: u8, v0308: u8, v0307: u8, v14: u8)
    -> (Vec<(u16, u8)>, u16) {
    let (ok, _) = s3_d150_check(v6878, v12, v16, v0308, v0307);
    let mut ev = vec![(0x0308, 0), (0x0312, 0)];
    if ok && (v14 & 3) == 0 {
        ev.push((0x0305, 7)); // 条件调面注（写族随链）
    }
    (ev, 0xCD6D)
}

#[cfg(test)]
mod s3_075_tests {
    use super::*;

    /// 二员转写形（$D150 三判链 root+一壳）。
    #[test]
    fn s3_075_units_two() {
        assert_eq!(s3_d150_check(1, 0x10, 0x10, 0, 0), (true, true));
        assert_eq!(s3_d150_check(0, 0x10, 0x10, 0, 0), (false, false)); // bcc 段
        assert_eq!(s3_d150_check(1, 0xC0, 0, 0, 0), (false, true)); // bmi 段（0xC0<<1=0x80 负）
        let (ev, j) = s3_d136_body(1, 0x10, 0x10, 5, 0, 0);
        assert_eq!(j, 0xCD6D);
        assert!(ev.contains(&(0x0308, 0)));
    }
}

// ---- SC-S3-16（S3-76）员转写（2 员——ClearHorzData 族）----

/// $CFB7 ClearHorzData（root——簇共根形七十）：$CF4C 调（ClearHorzMvmntData）
/// +$0315←Y（清族三写面：$0309/$0313/$0315←0——案实核恒定）。
/// 返回 (写事件, 恒定 3 写)。
pub fn s3_clear_horz_data() -> Vec<(u16, u8)> {
    vec![(0x0309, 0), (0x0313, 0), (0x0315, 0)]
}

/// $D13F 员体：ClearHorzData+尾跳 $CD6D（A=2 面——S1-09 接面）+$0308/$0312 清。
pub fn s3_d13f_body() -> (Vec<(u16, u8)>, u8, u16) {
    let mut ev = s3_clear_horz_data();
    ev.push((0x0308, 0));
    ev.push((0x0312, 0));
    (ev, 2, 0xCD6D)
}

#[cfg(test)]
mod s3_076_tests {
    use super::*;

    /// 二员转写形（ClearHorzData 恒定 root+一壳）。
    #[test]
    fn s3_076_units_two() {
        let ev = s3_clear_horz_data();
        assert_eq!(ev.len(), 3);
        assert!(ev.contains(&(0x0315, 0)));
        let (ev2, a, j) = s3_d13f_body();
        assert_eq!((a, j), (2, 0xCD6D));
        assert!(ev2.contains(&(0x0308, 0)));
    }
}

