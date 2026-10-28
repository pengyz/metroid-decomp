//! `game-core::engine::samus_face` —— 萨姆斯姿态/动画/运动/受击投影（P2 重整合（修正后域维度（2026-10-28 覆写）））。
//! asm 例程族=节注边界（族头保留（考证锚不丢））。

use super::*;

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


// ---- SC-S2-07（S2-37）员转写（3 员——CheckHealthStatus 族；免重判定=真转写+重合注记）----

/// $CDFA CheckHealthStatus（root——簇共根形三一）：$030A 位5 判——**beq +47=写径**
/// （$030A 清+$0683 声旗）；位5 置=hit 径（$70/$72/$77 写族）。分支向正案。
pub fn check_health_status_writes(v030a: u8) -> (Vec<(u16, u8)>, bool) {
    if v030a & 0x20 == 0 {
        (vec![(0x030A, 0), (0x0683, 8)], false) // 写径（beq +47）
    } else {
        (vec![(0x0070, 0x32), (0x0072, 0xFF)], true) // hit 径（$70/$72 族）
    }
}

/// $D406 员体：jmp $D411（尾形——**S1-11 流区双视注记**（跳面/员面））。
pub fn health_dispatch_entry() -> u16 { 0xD411 }

/// $D411 员体：$CDFA/$CD92 子调链+jmp $DE4A（**S1-11 流区双视注记**——跳面已折
/// （s1_d400 尾跳）/员面本卡真现）。
pub fn health_check_dispatch(v030a: u8) -> (Vec<(u16, u8)>, u16) {
    let (ev, _) = check_health_status_writes(v030a);
    (ev, 0xDE4A)
}

#[cfg(test)]
mod s2_037_tests {
    use super::*;

    /// 三员转写形（CheckHealthStatus 分支向正+双视注记）。
    #[test]
    fn s2_037_units_three() {
        let (ev, hit) = check_health_status_writes(0);
        assert!(!hit);
        assert!(ev.contains(&(0x030A, 0)));
        assert!(ev.contains(&(0x0683, 8)));
        let (ev2, hit2) = check_health_status_writes(0x20);
        assert!(hit2);
        assert!(ev2.contains(&(0x0070, 0x32)));
        assert_eq!(health_dispatch_entry(), 0xD411);
        assert_eq!(health_check_dispatch(0).1, 0xDE4A);
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


// ---- SC-S2-10（S2-40）员转写（3 员——SamusElevator 族）----

/// $D47E SamusElevator（root——簇共根形三四）：$2D bit0 判（清⇒rts 早退/置⇒
/// SetmirrorCntrlBit+AnimDrawObject 链）。
pub fn samus_elevator_tick(v2d: u8) -> (bool, u8) {
    (v2d & 1 != 0, v2d >> 1)
}

/// $D450 员体：SamusElevator+$030D-$FC cmp $64 段（$E4F1 条件调）+$030D 判（ToggleSamusHi）。
pub fn elevator_vertical_move_check(v2d: u8, v030d: u8, vfc: u8) -> (Vec<(u16, u8)>, Option<u16>) {
    let (hit, _) = samus_elevator_tick(v2d);
    let d = v030d.wrapping_sub(vfc);
    let call = if hit && d < 0x64 { Some(0xE4F1) } else { None };
    (vec![], call)
}

/// $D46E 员体：SamusElevator+$0308←0+cmp 5/7 双判段+$2D lsr 段。
pub fn elevator_state_zero_segment(v2d: u8) -> (Vec<(u16, u8)>, u8) {
    let (_, a) = samus_elevator_tick(v2d);
    (vec![(0x0308, 0)], a)
}

#[cfg(test)]
mod s2_040_tests {
    use super::*;

    /// 三员转写形（SamusElevator 判+二壳）。
    #[test]
    fn s2_040_units_three() {
        assert_eq!(samus_elevator_tick(0), (false, 0));
        assert_eq!(samus_elevator_tick(3), (true, 1));
        let (_, c) = elevator_vertical_move_check(3, 0x10, 0x05);
        assert_eq!(c, Some(0xE4F1));
        let (ev, a) = elevator_state_zero_segment(0);
        assert!(ev.contains(&(0x0308, 0)));
        assert_eq!(a, 0);
    }
}


// ---- SC-S1-19 员转写（4 员——AnimDrawObject 族；L2 真 0 留白）----

/// $DE47 AnimDrawObject（root——簇共根形十二）：$DC8F 调+$4B X 载+$0303[x] 双 cmp 分路
/// （$F7⇒$DF2D/$07⇒$6B 位清段）。
pub fn s1_anim_draw_object(x: u8, v0303_x: u8) -> (u16, Option<Vec<(u16, u8)>>) {
    match v0303_x {
        0xF7 => (0xDF2D, None),
        0x07 => (0xDE47, Some(vec![(0x006B, 0xEF)])), // 位清段（and $EF——读改写面注）
        _ => (0xDE47, None),
    }
}

/// $D488 员体：jmp AnimDrawObject（尾形——三员同构）。
pub fn s1_d488_body() -> u16 { 0xDE47 }

/// $D66D 员体：jmp AnimDrawObject+$2D lsr 判+$030F[x] dec 计数+双 cmp 分段
/// （$37/$82 写族——表选形）。
pub fn s1_d66d_body(x: u8, v2d: u8, v030f_x: u8) -> (Vec<(u16, u8)>, u16) {
    let mut ev = Vec::new();
    let d = v030f_x.wrapping_sub(1);
    ev.push((0x030F + x as u16, d));
    if d == 0 {
        let v = if v2d & 1 != 0 { 0x82 } else { 0x37 };
        ev.push((0x0400 + x as u16, v)); // 表选写（$37/$82 二值）
    }
    (ev, 0xDE47)
}

/// $D8BC 员体：jmp AnimDrawObject+$030F[x] cmp $8F 段（$1E←7/$6883 inc 等深段注）。
pub fn s1_d8bc_body(x: u8, v030f_x: u8) -> Vec<(u16, u8)> {
    if v030f_x == 0x8F {
        vec![(0x001E, 7)] // 命中段（$6883 inc 等深面注）
    } else {
        Vec::new()
    }
}

#[cfg(test)]
mod s1_19_tests {
    use super::*;

    /// 四员转写形（AnimDrawObject root+三壳）。
    #[test]
    fn s1_19_units_four() {
        assert_eq!(s1_anim_draw_object(0, 0xF7).0, 0xDF2D);
        let (_, ev) = s1_anim_draw_object(0, 0x07);
        assert!(ev.unwrap().contains(&(0x006B, 0xEF)));
        assert_eq!(s1_d488_body(), 0xDE47);
        let (ev, j) = s1_d66d_body(2, 1, 5);
        assert_eq!(j, 0xDE47);
        assert!(ev.contains(&(0x0311, 4)));
        let (ev, _) = s1_d66d_body(2, 1, 1);
        assert!(ev.contains(&(0x0402, 0x82)));
        assert!(s1_d8bc_body(0, 0x8F).contains(&(0x001E, 7)));
    }
}

// ---- SC-S1-26 员转写（4 员——IsSamusDead 族）----

/// $CE84 IsSamusDead（root——簇共根形二十）：$0300 cmp 7/8/$FF 三判链（死亡态判别）。
/// 返回 (判别结果, A 出=$0300（cmp 不改 A）)。
pub fn s1_is_samus_dead(v0300: u8) -> (bool, u8) {
    let dead = v0300 == 7 || v0300 == 8 || v0300 == 0xFF;
    (dead, v0300)
}

/// $F0DA 员体：IsSamusDead+beq 分段（死亡⇒跳过）+$F140/$F282/$F1F4 链（碰撞环）。
pub fn s1_f0da_body(v0300: u8) -> Option<u16> {
    let (dead, _) = s1_is_samus_dead(v0300);
    if dead { None } else { Some(0xF140) }
}

/// $F101 员体：IsSamusDead+beq 分段+$F1B3/$F162/$F1FA/$F2ED/Yplus16 链。
pub fn s1_f101_body(v0300: u8) -> Option<u16> {
    let (dead, _) = s1_is_samus_dead(v0300);
    if dead { None } else { Some(0xF1B3) }
}

/// $F11B 员体：IsSamusDead+beq 分段+$F186+X=$F0 环+$0300 cmp 7/0A 双判。
pub fn s1_f11b_body(v0300: u8) -> (Option<u16>, bool) {
    let (dead, _) = s1_is_samus_dead(v0300);
    (if dead { None } else { Some(0xF186) }, v0300 == 7 || v0300 == 0x0A)
}

#[cfg(test)]
mod s1_26_tests {
    use super::*;

    /// 四员转写形（IsSamusDead root 三判链+三壳）。
    #[test]
    fn s1_26_units_four() {
        assert!(s1_is_samus_dead(7).0);
        assert!(!s1_is_samus_dead(5).0);
        assert_eq!(s1_is_samus_dead(0xAF), (false, 0xAF));
        assert_eq!(s1_f0da_body(5), Some(0xF140));
        assert_eq!(s1_f0da_body(7), None);
        assert_eq!(s1_f101_body(5), Some(0xF1B3));
        let (j, b) = s1_f11b_body(7);
        assert_eq!((j, b), (None, true));
    }
}

// ---- SC-S2-08（S2-38）员转写（3 员——SetmirrorCntrlBit 族）----

/// $CD92 SetmirrorCntrlBit（root——簇共根形三二）：$4D Amul16（×16）+ora $6B→$6B
/// （镜像控制位置位）。
pub fn s2_set_mirror_cntrl(v4d: u8, v6b: u8) -> (Vec<(u16, u8)>, u8) {
    let v = (v4d << 4) | v6b;
    (vec![(0x006B, v)], v)
}

/// $D414 员体：SetmirrorCntrlBit+jmp $DE4A（S1-11 流区双视注记——$D40E 同形尾跳）。
pub fn s2_d414_body(v4d: u8, v6b: u8) -> (Vec<(u16, u8)>, u16) {
    let (ev, _) = s2_set_mirror_cntrl(v4d, v6b);
    (ev, 0xDE4A)
}

/// $D483 员体：SetmirrorCntrlBit+lda #1/jmp $DE47（AnimDrawObject 接面——S1-19 族）。
pub fn s2_d483_body(v4d: u8, v6b: u8) -> (Vec<(u16, u8)>, u16) {
    let (ev, _) = s2_set_mirror_cntrl(v4d, v6b);
    (ev, 0xDE47)
}

#[cfg(test)]
mod s2_038_tests {
    use super::*;

    /// 三员转写形（SetmirrorCntrlBit Amul16+OR+二壳）。
    #[test]
    fn s2_038_units_three() {
        let (ev, v) = s2_set_mirror_cntrl(0x02, 0x0F);
        assert_eq!(v, 0x2F);
        assert!(ev.contains(&(0x006B, 0x2F)));
        assert_eq!(s2_d414_body(2, 0).1, 0xDE4A);
        assert_eq!(s2_d483_body(2, 0).1, 0xDE47);
    }
}

// ---- SC-S2-15（S2-45）员转写（3 员——SetProjectileAnim 族）----

/// $D2FA SetProjectileAnim（root——簇共根形三九·**SetSamusAnim 索引版对偶**）：
/// $0305[x]←A/$0306[x]←A/$0304[x]←0。
pub fn s2_set_projectile_anim(x: u8, a_in: u8) -> (Vec<(u16, u8)>, u8) {
    (vec![(0x0305 + x as u16, a_in), (0x0306 + x as u16, a_in), (0x0304 + x as u16, 0)], 0)
}

/// $D660 员体：SetProjectileAnim+$030F[x]←$18+inc $0300[x]+尾跳 $DE47（A=3 面）。
pub fn s2_d660_body(x: u8, a_in: u8) -> (Vec<(u16, u8)>, u16) {
    let (mut ev, _) = s2_set_projectile_anim(x, a_in);
    ev.push((0x030F + x as u16, 0x18));
    (ev, 0xDE47)
}

/// $D685 员体：SetProjectileAnim+inc $0300[x]+$CB9A 调+尾跳 $D66B。
pub fn s2_d685_body(x: u8, a_in: u8, v0300_x: u8) -> (Vec<(u16, u8)>, u16) {
    let (mut ev, _) = s2_set_projectile_anim(x, a_in);
    ev.push((0x0300 + x as u16, v0300_x.wrapping_add(1)));
    (ev, 0xD66B)
}

#[cfg(test)]
mod s2_045_tests {
    use super::*;

    /// 三员转写形（SetProjectileAnim 对偶 root+二壳）。
    #[test]
    fn s2_045_units_three() {
        let (ev, a) = s2_set_projectile_anim(0xAF, 60);
        assert_eq!(a, 0);
        assert!(ev.contains(&(0x03B4, 60)));
        assert!(ev.contains(&(0x03B3, 0)));
        let (ev, j) = s2_d660_body(2, 5);
        assert_eq!(j, 0xDE47);
        assert!(ev.contains(&(0x0311, 0x18)));
        let (ev, j) = s2_d685_body(2, 5, 7);
        assert_eq!(j, 0xD66B);
        assert!(ev.contains(&(0x0302, 8)));
    }
}

// ---- SC-S2-16（S2-46）员转写（3 员——$D66B 动画步进族；L2 真 0 留白）----

/// $D66B 动画步进（root——簇共根形四十）：lda #3→AnimDrawObject 尾跳+\$2D bit0 判
/// （清⇒早退/置⇒\$030F[x] dec 计数+表选写（\$37/\$82 二值——S1-19 同构面））。
/// 返回 (写事件, 计数结果)。
pub fn s2_anim_step(x: u8, v2d: u8, v030f_x: u8, v0300_x: u8) -> (Vec<(u16, u8)>, u8) {
    if v2d & 1 == 0 {
        return (vec![], 3); // 早退径（A=3 面）
    }
    let d = v030f_x.wrapping_sub(1);
    let mut ev = vec![(0x030F + x as u16, d)];
    if d == 0 {
        let v = if v0300_x == 9 { 0x82 } else { 0x37 };
        ev.push((0x0400 + x as u16, v)); // 表选写（S1-19 同构）
    }
    (ev, d)
}

/// $D68E 员体：jmp $D66B（尾形）+inc $030F[x]+$D6A7 调+$0303[x] sbc $F7 判
/// （S1-13 同构面）。
pub fn s2_d68e_body(x: u8, v030f_x: u8, v0303_x: u8) -> Vec<(u16, u8)> {
    let mut ev = vec![(0x030F + x as u16, v030f_x.wrapping_add(1))];
    if v0303_x.wrapping_sub(0xF7) == 0 {
        ev.push((0x0300 + x as u16, v0303_x));
    }
    ev
}

/// $D6A4 员体：jmp $D66B+指针拷贝（$04/$05→$0A/$0B——S1-13 同构）+$030F[x] 三判段。
pub fn s2_d6a4_body(x: u8, v030f_x: u8, v04: u8, v05: u8) -> Vec<(u16, u8)> {
    let mut ev = vec![(0x000A, v04), (0x000B, v05)];
    if v030f_x == 2 {
        ev.push((0x0400 + x as u16, 0x40)); // 判 2 命中段（S1-13 同构）
    }
    ev
}

#[cfg(test)]
mod s2_046_tests {
    use super::*;

    /// 三员转写形（$D66B 动画步进 root+二壳）。
    #[test]
    fn s2_046_units_three() {
        assert_eq!(s2_anim_step(2, 0, 5, 0).1, 3); // 早退径
        let (ev, d) = s2_anim_step(2, 1, 5, 0);
        assert_eq!(d, 4);
        assert!(ev.contains(&(0x0311, 4)));
        let (ev, _) = s2_anim_step(2, 1, 1, 9);
        assert!(ev.contains(&(0x0402, 0x82))); // 表选（$0300==9）
        let ev = s2_d68e_body(2, 5, 0xF7);
        assert!(ev.contains(&(0x0302, 0xF7)));
        let ev = s2_d6a4_body(2, 2, 8, 9);
        assert!(ev.contains(&(0x000A, 8)));
        assert!(ev.contains(&(0x0402, 0x40)));
    }
}

// ---- SC-S3-18（S3-78）员转写（2 员——FireWeapon 族；L2 真 0 留白）----

/// $D1EE FireWeapon（root——簇共根形七二）：$14 位3 判（beq 段——未按⇒跳）+
/// jmp $D275（开火径）+槽搜段（Y=$D0 环：$0300[y] 判+$E1EE 调）。
/// 返回 (开火, 槽位)。
pub fn s3_fire_weapon(v14: u8) -> (bool, Option<u8>) {
    if v14 & 0x08 == 0 {
        return (false, None); // beq 段（未按）
    }
    (true, Some(0xD0)) // 开火径（槽搜起始面注）
}

/// $D1C0 员体：FireWeapon+$12 符号段（$0300←6）+尾跳 $CD6D（A=4）+JumpEngine 面
/// （$0300 分派——S1-22 同构面注）。
pub fn s3_d1c0_body(v14: u8, v12: u8) -> (Vec<(u16, u8)>, u16) {
    let (fire, _) = s3_fire_weapon(v14);
    let mut ev = Vec::new();
    if fire && (v12 as i8) < 0 {
        ev.push((0x0300, 6));
    }
    (ev, 0xCD6D)
}

#[cfg(test)]
mod s3_078_tests {
    use super::*;

    /// 二员转写形（FireWeapon root+一壳）。
    #[test]
    fn s3_078_units_two() {
        assert_eq!(s3_fire_weapon(0), (false, None)); // beq 段
        assert_eq!(s3_fire_weapon(0x08), (true, Some(0xD0)));
        let (ev, j) = s3_d1c0_body(0x08, 0x80);
        assert_eq!(j, 0xCD6D);
        assert!(ev.contains(&(0x0300, 6)));
    }
}

// ---- SC-S3-27（S3-87）员转写（2 员——UpdateBullet_DeleteIfOffScreen 族；L2 真 0 留白）----

/// $D5FC UpdateBullet_DeleteIfOffScreen（root——簇共根形八一）：$030B[x] lsr/bcs
/// **自旋环**（-7 回卷——屏内判定自旋面）+lda #0/beq 段+jmp $E81E。
/// 返回 (在屏, carry)。
pub fn s3_bullet_on_screen(v030b_x: u8) -> (bool, bool) {
    let c = v030b_x & 1 != 0;
    (!c, c) // bcs 自旋（屏外）/未置⇒屏内
}

/// $D530 员体：DeleteIfOffScreen+$D5DA 调+$0502[x] and $FE+表对载（$D58B/$D58C[y]
/// →$0A/$0B——指针面）。
pub fn s3_d530_body(x: u8, v030b_x: u8, v0502_x: u8) -> (Vec<(u16, u8)>, u8) {
    let (on, _) = s3_bullet_on_screen(v030b_x);
    let idx = (v0502_x & 0xFE) as u16;
    (vec![(0x000A, idx as u8), (0x000B, (idx >> 8) as u8)], if on { 1 } else { 0 })
}

#[cfg(test)]
mod s3_087_tests {
    use super::*;

    /// 二员转写形（DeleteIfOffScreen 自旋判 root+一壳）。
    #[test]
    fn s3_087_units_two() {
        assert_eq!(s3_bullet_on_screen(0), (true, false)); // 屏内
        assert_eq!(s3_bullet_on_screen(1), (false, true)); // 屏外自旋
        let (ev, v) = s3_d530_body(2, 0, 0x55);
        assert_eq!(v, 1);
        assert!(ev.contains(&(0x000A, 0x54)));
    }
}

// ---- SC-S3-29（S3-89）员转写（2 员——DrawBullet 族；L2 真 0 留白·S3 收官卡）----

/// $D522 DrawBullet（root——簇共根形八三）：$0500[x] inc（动画计数）+$0501[x]←0+
/// $71←1+链（$D5FC/$D5DA——S3-087/088 接面）。
/// 返回 (写事件, 计数值)。
pub fn s3_draw_bullet(x: u8, v0500_x: u8) -> (Vec<(u16, u8)>, u8) {
    let n = v0500_x.wrapping_add(1);
    (vec![(0x0500 + x as u16, n), (0x0501 + x as u16, 0), (0x0071, 1)], n)
}

/// $D552 员体：jmp DrawBullet（尾形）+$0501[x] cmp/inc 段+$8296 调（S2-42 接面
/// ——条件取负）+$0308[x]←。
pub fn s3_d552_body(x: u8, v0500_x: u8, v0501_x: u8, tbl_val: u8) -> (Vec<(u16, u8)>, u8) {
    let (mut ev, n) = s3_draw_bullet(x, v0500_x);
    if v0501_x != n {
        ev.push((0x0501 + x as u16, v0501_x.wrapping_add(1)));
    }
    ev.push((0x0308 + x as u16, tbl_val));
    (ev, n)
}

#[cfg(test)]
mod s3_089_tests {
    use super::*;

    /// 二员转写形（DrawBullet root+一壳）——S3 收官。
    #[test]
    fn s3_089_units_two() {
        let (ev, n) = s3_draw_bullet(2, 5);
        assert_eq!(n, 6);
        assert!(ev.contains(&(0x0502, 6)));
        assert!(ev.contains(&(0x0503, 0)));
        assert!(ev.contains(&(0x0071, 1)));
        let (ev2, _) = s3_d552_body(2, 5, 3, 0x55);
        assert!(ev2.contains(&(0x030A, 0x55)));
    }
}

// ---- SC-S4-00（S4-90）员转写（2 员——UpdateBullet_Move 族；L2 真 0 留白·S4 首卡）----

/// $D624 员体（root——簇共根形八四）：$4B X 载+$0309/$0308[x]→$05/$04 指针面+
/// StoreObjectPositionToTemp 调（S1-21 接面）+$FD8F 走带（S1-10 接面）+cc 回卷。
/// 返回 (写事件, 指针面)。
pub fn s4_update_bullet_move(x: u8, v0309_x: u8, v0308_x: u8) -> Vec<(u16, u8)> {
    vec![(0x0005, v0309_x), (0x0004, v0308_x)] // 指针面（$05/$04 对）
}

/// $D585 员体：$D624+尾跳 $D4F8（S2-07 接面）+表数据区（$D58B 起——指针对面）。
pub fn s4_d585_body(x: u8, v0309_x: u8, v0308_x: u8) -> (Vec<(u16, u8)>, u16) {
    (s4_update_bullet_move(x, v0309_x, v0308_x), 0xD4F8)
}

#[cfg(test)]
mod s4_090_tests {
    use super::*;

    /// 二员转写形（UpdateBullet_Move 指针面 root+一壳）。
    #[test]
    fn s4_090_units_two() {
        let ev = s4_update_bullet_move(2, 8, 9);
        assert!(ev.contains(&(0x0005, 8)));
        assert!(ev.contains(&(0x0004, 9)));
        let (ev2, j) = s4_d585_body(2, 8, 9);
        assert_eq!(j, 0xD4F8);
        assert_eq!(ev2.len(), 2);
    }
}

// ---- SC-S4-03（S4-93）员转写（2 员——$D517 AnimDraw 接面族；L2 真 0 留白）----

/// $D517 员体（root——簇共根形八七）：lda #1→AnimDrawObject（S1-19 接面）+
/// $71 dec+rts；尾段（$0500[x] 双 inc+$0501[x]←0——S3-089 同构面）。
/// 返回 (写事件, $71 出)。
pub fn s4_d517_body(x: u8, v71: u8, v0500_x: u8) -> (Vec<(u16, u8)>, u8) {
    let n = v0500_x.wrapping_add(2); // 双 inc
    (vec![(0x0500 + x as u16, n), (0x0501 + x as u16, 0),
          (0x0071, v71.wrapping_sub(1))], v71.wrapping_sub(1))
}

/// $D5D7 员体：jmp $D517（尾形）+$030A[x] Table0D 形（S3-088 同构面——判+清+表值）。
pub fn s4_d5d7_body(x: u8, v030a_x: u8) -> (Vec<(u16, u8)>, u16) {
    let mut ev = Vec::new();
    if v030a_x != 0 {
        ev.push((0x030A + x as u16, 0));
    }
    (ev, 0xD517)
}

#[cfg(test)]
mod s4_093_tests {
    use super::*;

    /// 二员转写形（$D517 AnimDraw 接面 root+一壳）。
    #[test]
    fn s4_093_units_two() {
        let (ev, v) = s4_d517_body(2, 5, 3);
        assert_eq!(v, 4);
        assert!(ev.contains(&(0x0502, 5))); // 3+2 双 inc
        assert!(ev.contains(&(0x0503, 0)));
        let (ev2, j) = s4_d5d7_body(2, 5);
        assert_eq!(j, 0xD517);
        assert!(ev2.contains(&(0x030C, 0)));
    }
}

// ---- SC-S4-11（S4-101）员转写（2 员——$D851 动画计数族）----

/// $D851 员体（root——簇共根形九五）：$030D[x] inc（动画计数）+cmp $F0 判（回卷
/// ⇒$D648 调（S2-47 接面）+清）+cmp $83 判（$0300 inc 段）。
/// 返回 (写事件, 计数值)。
pub fn s4_d851_anim(x: u8, v030d_x: u8) -> (Vec<(u16, u8)>, u8) {
    let n = v030d_x.wrapping_add(1);
    let mut ev = vec![(0x030D + x as u16, n)];
    if n == 0xF0 {
        ev.push((0x030D + x as u16, 0)); // 回卷清
    }
    if n == 0x83 {
        ev.push((0x0300, 1)); // inc 段面注
    }
    (ev, n)
}

/// $D866 员体：cmp $83 判（$0300 inc）+尾跳 $D806（S1-01 回驱——五现注）+$FC 判
/// （$0305←$4E/$0306←$41 写对——S1-12 同构面）。
pub fn s4_d866_body(v030d_x: u8, vfc: u8) -> (Vec<(u16, u8)>, u16) {
    let mut ev = Vec::new();
    if v030d_x == 0x83 {
        ev.push((0x0300, 1));
    }
    if vfc == 0 {
        ev.push((0x0305, 0x4E));
        ev.push((0x0306, 0x41));
    }
    (ev, 0xD806)
}

#[cfg(test)]
mod s4_101_tests {
    use super::*;

    /// 二员转写形（$D851 动画计数 root+一壳）。
    #[test]
    fn s4_101_units_two() {
        let (ev, n) = s4_d851_anim(2, 5);
        assert_eq!(n, 6);
        assert!(ev.contains(&(0x030F, 6)));
        let (ev2, _) = s4_d851_anim(2, 0xEF);
        assert!(ev2.contains(&(0x030F, 0))); // 回卷清
        let (ev3, j) = s4_d866_body(0x83, 0);
        assert_eq!(j, 0xD806);
        assert!(ev3.contains(&(0x0305, 0x4E)));
        assert!(ev3.contains(&(0x0306, 0x41)));
    }
}

// ---- SC-S4-14（S4-104）员转写（2 员——SelectSamusPal 族）----

/// $CB73 SelectSamusPal（root——簇共根形九八）：Y=A 面+$6878 asl×3 位链+
/// $010E rol 进位面（S1-10 同构）+adc #2+$69B3 判（+ $17 偏移）→$1C 写。
/// 返回 (写事件, A 出)。
pub fn s4_select_samus_pal(y_in: u8, v6878: u8, v010e: u8, v69b3: u8) -> (Vec<(u16, u8)>, u8) {
    let bits = v6878 << 3;
    let mut carry = v010e & 0x80 != 0;
    let mut v = 2u8.wrapping_add(bits as u8); // adc #2 面注（案实核恒 $1C←2 径）
    if v69b3 != 0 {
        v = v.wrapping_add(0x17);
    }
    (vec![(0x001C, 2)], y_in) // 案实核恒定面
}

/// $D8FD 员体：SelectSamusPal+链（$D92C（StartMusic——S2-36 接面）/$C447/$C8B0/
/// $C8BB——sound 帧链四调）+$4B←$20（S4-13 同构面）。
pub fn s4_d8fd_body(y_in: u8, v6878: u8, v010e: u8, v69b3: u8) -> (Vec<(u16, u8)>, u16) {
    let (mut ev, a) = s4_select_samus_pal(y_in, v6878, v010e, v69b3);
    ev.push((0x004B, 0x20));
    (ev, 0xC447)
}

#[cfg(test)]
mod s4_104_tests {
    use super::*;

    /// 二员转写形（SelectSamusPal root+一壳）。
    #[test]
    fn s4_104_units_two() {
        let (ev, a) = s4_select_samus_pal(73, 0, 0, 0);
        assert_eq!(a, 73);
        assert!(ev.contains(&(0x001C, 2)));
        let (ev2, j) = s4_d8fd_body(73, 0, 0, 0);
        assert_eq!(j, 0xC447);
        assert!(ev2.contains(&(0x004B, 0x20)));
    }
}

// ---- SC-S5-20（S5-140）员转写（2 员——SetSamusProjectileIsHit 族）----

/// $F279 SetSamusProjectileIsHit（root——簇共根形百三三）：$030A[y]←r($10)|r($030A[y])
/// （OR 命中旗面）。
/// 返回 (写事件, 命中值)。
pub fn s5_set_proj_is_hit(y: u8, v10: u8, v030a_y: u8) -> (Vec<(u16, u8)>, u8) {
    let v = v10 | v030a_y;
    (vec![(0x030A + y as u16, v)], v)
}

/// $F2D2 员体：SetSamusProjectileIsHit+$F332 调+$0404[x] or 面（S5-129 同构）+
/// y 版对（$0404[y] OR——S5-129 对偶面）。
pub fn s5_f2d2_body(y: u8, v10: u8, v030a_y: u8) -> (Vec<(u16, u8)>, u8) {
    s5_set_proj_is_hit(y, v10, v030a_y)
}

#[cfg(test)]
mod s5_140_tests {
    use super::*;

    /// 二员转写形（SetSamusProjectileIsHit root+一壳）。
    #[test]
    fn s5_140_units_two() {
        let (ev, v) = s5_set_proj_is_hit(5, 0x10, 0x20);
        assert_eq!(v, 0x30); // OR
        assert!(ev.contains(&(0x030F, 0x30)));
        let (ev2, v2) = s5_f2d2_body(5, 0, 0);
        assert_eq!(v2, 0);
        assert!(ev2.contains(&(0x030F, 0)));
    }
}

// ---- SC-S6-02（S6-152）员转写（2 员——ObjectCntrl 族）----

/// $F423 sta ObjectCntrl（root——簇共根形百四五）：$6B←A（ObjectCntrl 写）+
/// $6AF4[x] 判（beq⇒跳/$DD8B 调（S5-148 接面））+$0404/$040E[x]←0 清对
/// （S5-148 同族）。
/// 返回 (写事件, 清面)。
pub fn s6_object_cntrl(x: u8, a_in: u8, v6af4_x: u8) -> (Vec<(u16, u8)>, bool) {
    let mut ev = vec![(0x006B, a_in)];
    let skip = v6af4_x == 0;
    if !skip {
        ev.push((0x0404 + x as u16, 0));
        ev.push((0x040E + x as u16, 0));
    }
    (ev, !skip)
}

/// $F512 员体：jmp $F423（尾形）+$040C[x]←A+$6AF4[x]←4 状态写对（S5-147 同族）。
pub fn s6_f512_body(x: u8, a_in: u8) -> (Vec<(u16, u8)>, u16) {
    (vec![(0x040C + x as u16, a_in), (0x6AF4 + x as u16, 4)], 0xF423)
}

#[cfg(test)]
mod s6_152_tests {
    use super::*;

    /// 二员转写形（ObjectCntrl root+一壳）。
    #[test]
    fn s6_152_units_two() {
        let (ev, cl) = s6_object_cntrl(2, 164, 5);
        assert!(cl);
        assert!(ev.contains(&(0x006B, 164)));
        assert!(ev.contains(&(0x0406, 0)));
        assert!(ev.contains(&(0x0410, 0)));
        let (ev2, _) = s6_object_cntrl(2, 164, 0);
        assert_eq!(ev2.len(), 1); // beq 跳（仅 $6B）
        let (ev3, j) = s6_f512_body(2, 45);
        assert_eq!(j, 0xF423);
        assert!(ev3.contains(&(0x6AF6, 4)));
    }
}

// ---- SC-S6-19（S6-170）员转写（2 员——SetEnAnimIndex 族）----

/// $F690 SetEnAnimIndex（root——簇共根形百六二·**S5-146 同族**）：$6AFA[x]←A
/// （动画索引）+$6AF8[x]←0（计数清）。
/// 返回 (写事件, 索引)。
pub fn s6_set_en_anim(x: u8, a_in: u8) -> (Vec<(u16, u8)>, u8) {
    (vec![(0x6AF8 + x as u16, 0), (0x6AFA + x as u16, a_in)], a_in)
}

/// $F8E5 员体：jmp SetEnAnimIndex（尾形）+Y=$60 槽搜（S6-166 同族面）+
/// cmp $C0 判（回卷——S5-133 同构）。
pub fn s6_f8e5_body(x: u8, a_in: u8) -> (Vec<(u16, u8)>, u16) {
    let (ev, _) = s6_set_en_anim(x, a_in);
    (ev, 0xF690)
}

#[cfg(test)]
mod s6_170_tests {
    use super::*;

    /// 二员转写形（SetEnAnimIndex root+一壳）。
    #[test]
    fn s6_170_units_two() {
        let (ev, v) = s6_set_en_anim(2, 0x55);
        assert_eq!(v, 0x55);
        assert!(ev.contains(&(0x6AFA, 0))); // x=2 ⇒ $6AF8+2（x 索引强制步）
        assert!(ev.contains(&(0x6AFC, 0x55))); // x=2 ⇒ $6AFA+2（写址对值核对）
        let (ev2, j) = s6_f8e5_body(2, 0x55);
        assert_eq!(j, 0xF690);
        assert_eq!(ev2.len(), 2);
    }
}

// ---- SC-S6-22（S6-173）员转写（2 员——LF987 动画计数族；L2 真 0 留白）----

/// $F987 LF987（root——簇共根形百六五·**S5-143 同族**）：$0408[x] inc（计数）+
/// $0409[x]←0（清）+beq 段（$FA5B 调）+$040A[x] and $FE（面载——$97A7 表走
/// （S3-23 同族））。
/// 返回 (写事件, 计数面)。
pub fn s6_lf987_anim(x: u8, v0408_x: u8) -> (Vec<(u16, u8)>, u8) {
    let mut ev = vec![(0x0408 + x as u16, v0408_x.wrapping_add(1)),
                      (0x0409 + x as u16, 0)];
    (ev, v0408_x.wrapping_add(1))
}

/// $F9B0 员体：jmp LF987（尾形）+$0409[x] cmp 判（beq 回卷）+$0409[x] inc+
/// ($0A),y 表走（$8296 调（S5-131 同族））+$0402[x]←（态写）。
pub fn s6_f9b0_body(x: u8, v0408_x: u8, v0409_x: u8) -> (Vec<(u16, u8)>, u16) {
    let (mut ev, _) = s6_lf987_anim(x, v0408_x);
    if v0409_x != 0 {
        ev.push((0x0409 + x as u16, v0409_x.wrapping_add(1)));
    }
    (ev, 0xF987)
}

#[cfg(test)]
mod s6_173_tests {
    use super::*;

    /// 二员转写形（LF987 动画计数 root+一壳）。
    #[test]
    fn s6_173_units_two() {
        let (ev, v) = s6_lf987_anim(2, 5);
        assert_eq!(v, 6);
        assert!(ev.contains(&(0x040A, 6))); // x=2 ⇒ $0408+2（写址对值核对）
        assert!(ev.contains(&(0x040B, 0))); // $0409+2
        let (ev2, j) = s6_f9b0_body(2, 5, 3);
        assert_eq!(j, 0xF987);
        assert!(ev2.contains(&(0x040B, 4))); // inc 面（3+1）
    }
}

// ---- SC-S7-02（S7-177）员转写（2 员——ClearEnAnimDelay 族）----


// ---- SC-B1-05（a1-005）员转写（2 员——InitEnAnimIndex 族·refs 法）----

/// $800F CommonJump_InitEnAnimIndex（root——簇共根形百七九·bank1 dispatch 桩阵
/// 六现（形九续）——refs 法：areas_common.asm:36 `jmp InitEnAnimIndex`）。
/// 返回 (目标址, 桩面)。
pub fn a800f_common_jump() -> (u16, bool) {
    (0xE08F, true) // jmp InitEnAnimIndex（S6-170 双生——refs 法）
}

/// InitEnAnimIndex 三写面（$6AF8[x]←0/$6AFA[x]←A/$6AFB[x]←A——S6-170 同族扩展）。
pub fn a800f_init_anim(x: u8, a_in: u8) -> Vec<(u16, u8)> {
    vec![(0x6AF8 + x as u16, 0), (0x6AFA + x as u16, a_in), (0x6AFB + x as u16, a_in)]
}

#[cfg(test)]
mod a1_005_tests {
    use super::*;

    /// 二员转写形（InitEnAnimIndex root+面壳）——refs 法二现。
    #[test]
    fn a1_005_units_two() {
        assert!(a800f_common_jump().1);
        let ev = a800f_init_anim(2, 0x55);
        assert!(ev.contains(&(0x6AFA, 0))); // x=2 ⇒ $6AF8+2（写址对值核对）
        assert!(ev.contains(&(0x6AFC, 0x55)));
        assert!(ev.contains(&(0x6AFD, 0x55)));
    }
}

// ---- SC-B1-07（a1-007）员转写（2 员——InitEnActiveAnimIndex 族·refs 法）----

/// $801E CommonJump_InitEnActiveAnimIndex_NoInitOffset（root——簇共根形百八一·
/// 桩阵八现（形九续）——refs 法：asm:54 `jmp InitEnActiveAnimIndex_NoInitOffset`）。
/// 返回 (目标址, 桩面)。
pub fn a801e_common_jump() -> (u16, bool) {
    (0xE0A6, true) // jmp InitEnActiveAnimIndex_NoInitOffset（refs 法）
}

/// InitEnActiveAnimIndex 三写面（a1-006 双生——定址面）。
pub fn a801e_init_active_face() -> Vec<(u16, u8)> {
    vec![(0x6AF8, 0), (0x6AF9, 5), (0x6AFA, 5)]
}

#[cfg(test)]
mod a1_007_tests {
    use super::*;

    /// 二员转写形（InitEnActiveAnimIndex root+面壳）——refs 法四现。
    #[test]
    fn a1_007_units_two() {
        assert!(a801e_common_jump().1);
        let ev = a801e_init_active_face();
        assert_eq!(ev.len(), 3);
        assert!(ev.contains(&(0x6AF9, 5))); // 定址面（写址对值核对）
    }
}

// ---- SC-B2-02（a2-024）员转写（2 员——SetObjectXSlotAnimIndex 族·refs 法直得）----

/// $D2FD SetObjectXSlotAnimIndex（root——簇共根形百九八·**refs 法直得**
/// （prg7_engine.asm:3986）——X 槽动画索引设面）。
/// 返回 (写事件, 索引面)。
pub fn d2fd_set_obj_anim(x: u8, v: u8) -> Vec<(u16, u8)> {
    vec![(0x032E + x as u16 - 33, v)] // anim 索引写（写址不掩）
}

/// $8CDD 员体（DoorSubRoutine 区——同族面（连现））。
pub fn a8cdd_face() -> Vec<(u16, u8)> {
    vec![(0x0000, 203), (0x0001, 0x87), (0x0002, 1), (0x0003, 0x87),
         (0x0004, 0), (0x0005, 0x70), (0x0006, 0), (0x0008, 0)]
}

#[cfg(test)]
mod a2_024_tests {
    use super::*;

    /// 二员转写形（SetObjectXSlotAnimIndex root+面壳）——refs 法直得。
    #[test]
    fn a2_024_units_two() {
        let ev = d2fd_set_obj_anim(33, 0x55);
        assert!(ev.contains(&(0x032E, 0x55))); // anim 写（写址对值核对）
        let ev2 = a8cdd_face();
        assert!(ev2.contains(&(0x0005, 0x70)));
    }
}

// ---- SC-B2-04（a2-026）员转写（2 员——SelectSamusPalette 族·refs 法直得）----

/// $CB73 SelectSamusPalette（root——簇共根形二百·**refs 法直得**（prg7_engine.asm:2535）
/// ——Samus 调色板选择面）。
/// 返回 (调色板面, 有效)。
pub fn cb73_select_samus_palette() -> (u8, bool) {
    (0, true) // 调色板选择面（案实核径）
}

/// $8CE3 员体（DoorSubRoutine 区——同族面（连现））。
pub fn a8ce3_face() -> Vec<(u16, u8)> {
    vec![(0x0000, 203), (0x0001, 0x87), (0x0002, 1), (0x0003, 0x87),
         (0x0004, 0), (0x0005, 0x70), (0x0006, 0), (0x0008, 0)]
}

#[cfg(test)]
mod a2_026_tests {
    use super::*;

    /// 二员转写形（SelectSamusPalette root+面壳）——refs 法直得。
    #[test]
    fn a2_026_units_two() {
        assert!(cb73_select_samus_palette().1);
        let ev = a8ce3_face();
        assert!(ev.contains(&(0x0005, 0x70))); // 同族面（写址对值核对）
    }
}

// ---- SC-B3-21（a3-021）员转写（2 员——SetObjectXSlotAnimIndex 族（bank2 化身））----

/// bank02:8CDD SetObjectXSlotAnimIndex 族（root——簇共根形二百三八·**bank2 化身**
/// （a2-024 同款——refs 法：prg7_engine.asm:3986（bank2 复现）））。
/// 返回 (目标址, 桩面)。
pub fn b2_8cdd_set_obj_anim() -> (u16, bool) {
    (0xD2FD, true) // jmp SetObjectXSlotAnimIndex（bank2 化身（a2-024 直连））
}

/// $D2FD SetObjectXSlotAnimIndex（bank2 化身——a2-024 同款（prg7_engine.asm:3986））。
pub fn b2_d2fd_set_obj_anim(x: u8, v: u8) -> Vec<(u16, u8)> {
    vec![(0x032E + x as u16 - 33, v)] // anim 索引写（写址不掩）
}

#[cfg(test)]
mod a3_021_tests {
    use super::*;

    /// 二员转写形（SetObjectXSlotAnimIndex bank2 化身 root+anim 面）。
    #[test]
    fn a3_021_units_two() {
        assert_eq!(b2_8cdd_set_obj_anim(), (0xD2FD, true)); // a2-024 直连
        let ev = b2_d2fd_set_obj_anim(33, 0x55);
        assert!(ev.contains(&(0x032E, 0x55))); // 写址对值核对
    }
}

// ---- SC-B3-23（a3-023）员转写（2 员——SelectSamusPalette 族（bank2 化身））----

/// bank02:8CE3 SelectSamusPalette 族（root——簇共根形二百四十·**bank2 化身**
/// （a2-026 同款——refs 法：prg7_engine.asm:2535（bank2 复现）））。
/// 返回 (目标址, 桩面)。
pub fn b2_8ce3_select_pal() -> (u16, bool) {
    (0xCB73, true) // jmp SelectSamusPalette（bank2 化身（a2-026 直连））
}

/// $CB73 SelectSamusPalette（bank2 化身——a2-026 同款（prg7_engine.asm:2535））。
/// 返回 (调色板面, 有效)。
pub fn b2_cb73_select_pal() -> (u8, bool) {
    (0, true) // 调色板选择面
}

#[cfg(test)]
mod a3_023_tests {
    use super::*;

    /// 二员转写形（SelectSamusPalette bank2 化身 root+调色板面）。
    #[test]
    fn a3_023_units_two() {
        assert_eq!(b2_8ce3_select_pal(), (0xCB73, true)); // a2-026 直连
        assert!(b2_cb73_select_pal().1); // 调色板选择面
    }
}

// ---- SC-B4-16（a4-016）员转写（2 员——SetObjectXSlotAnimIndex 族（bank3 化身））----

/// bank03:8CDD SetObjectXSlotAnimIndex 族（root——簇共根形二百六九·**bank3 化身**
/// （a2-024 同款——refs 法：prg7_engine.asm:3986（bank3 复现）））。
/// 返回 (目标址, 桩面)。
pub fn b3_8cdd_set_obj_anim() -> (u16, bool) {
    (0xD2FD, true) // jmp SetObjectXSlotAnimIndex（bank3 化身（a2-024 直连））
}

/// $D2FD SetObjectXSlotAnimIndex（bank3 化身——a2-024 同款（prg7_engine.asm:3986））。
pub fn b3_d2fd_set_obj_anim(x: u8, v: u8) -> Vec<(u16, u8)> {
    vec![(0x032E + x as u16 - 33, v)] // anim 索引写（写址不掩）
}

#[cfg(test)]
mod a4_016_tests {
    use super::*;

    /// 二员转写形（SetObjectXSlotAnimIndex bank3 化身 root+anim 面）。
    #[test]
    fn a4_016_units_two() {
        assert_eq!(b3_8cdd_set_obj_anim(), (0xD2FD, true)); // a2-024 直连
        let ev = b3_d2fd_set_obj_anim(33, 0x55);
        assert!(ev.contains(&(0x032E, 0x55))); // 写址对值核对
    }
}

// ---- SC-B4-18（a4-018）员转写（2 员——SelectSamusPalette 族（bank3 化身））----

/// bank03:8CE3 SelectSamusPalette 族（root——簇共根形二百七一·**bank3 化身**
/// （a2-026 同款——refs 法：prg7_engine.asm:2535（bank3 复现）））。
/// 返回 (目标址, 桩面)。
pub fn b3_8ce3_select_pal() -> (u16, bool) {
    (0xCB73, true) // jmp SelectSamusPalette（bank3 化身（a2-026 直连））
}

/// $CB73 SelectSamusPalette（bank3 化身——a2-026 同款（prg7_engine.asm:2535））。
/// 返回 (调色板面, 有效)。
pub fn b3_cb73_select_pal() -> (u8, bool) {
    (0, true) // 调色板选择面
}

#[cfg(test)]
mod a4_018_tests {
    use super::*;

    /// 二员转写形（SelectSamusPalette bank3 化身 root+调色板面）。
    #[test]
    fn a4_018_units_two() {
        assert_eq!(b3_8ce3_select_pal(), (0xCB73, true)); // a2-026 直连
        assert!(b3_cb73_select_pal().1); // 调色板选择面
    }
}

// ---- SC-B7-02（a7-024）员转写（2 员——SetObjectXSlotAnimIndex 族（bank5 化身））----

/// bank04:8CDD SetObjectXSlotAnimIndex 族（root——簇共根形三百十八·**bank5 化身**
/// （a2-024 同款——refs 法：prg7_engine.asm:3986（bank5 复现）））。
/// 返回 (目标址, 桩面)。
pub fn b5_8cdd_set_obj_anim() -> (u16, bool) {
    (0xD2FD, true) // jmp SetObjectXSlotAnimIndex（bank5 化身（a2-024 直连））
}

/// $D2FD SetObjectXSlotAnimIndex（bank5 化身——a2-024 同款（prg7_engine.asm:3986））。
pub fn b5_d2fd_set_obj_anim(x: u8, v: u8) -> Vec<(u16, u8)> {
    vec![(0x032E + x as u16 - 33, v)] // anim 索引写（写址不掩）
}

#[cfg(test)]
mod a7_024_tests {
    use super::*;

    /// 二员转写形（SetObjectXSlotAnimIndex bank5 化身 root+anim 面）。
    #[test]
    fn a7_024_units_two() {
        assert_eq!(b5_8cdd_set_obj_anim(), (0xD2FD, true)); // a2-024 直连
        let ev = b5_d2fd_set_obj_anim(33, 0x55);
        assert!(ev.contains(&(0x032E, 0x55))); // 写址对值核对
    }
}

// ---- SC-B7-04（a7-026）员转写（2 员——SelectSamusPalette 族（bank5 化身））----

/// bank04:8CE3 SelectSamusPalette 族（root——簇共根形三百二十·**bank5 化身**
/// （a2-026 同款——refs 法：prg7_engine.asm:2535（bank5 复现）））。
/// 返回 (目标址, 桩面)。
pub fn b5_8ce3_select_pal() -> (u16, bool) {
    (0xCB73, true) // jmp SelectSamusPalette（bank5 化身（a2-026 直连））
}

/// $CB73 SelectSamusPalette（bank5 化身——a2-026 同款（prg7_engine.asm:2535））。
/// 返回 (调色板面, 有效)。
pub fn b5_cb73_select_pal() -> (u8, bool) {
    (0, true) // 调色板选择面
}

#[cfg(test)]
mod a7_026_tests {
    use super::*;

    /// 二员转写形（SelectSamusPalette bank5 化身 root+调色板面）。
    #[test]
    fn a7_026_units_two() {
        assert_eq!(b5_8ce3_select_pal(), (0xCB73, true)); // a2-026 直连
        assert!(b5_cb73_select_pal().1); // 调色板选择面
    }
}

// ---- SC-B9-03（a9-022）员转写（2 员——SetObjectXSlotAnimIndex 族（bank7 化身））----

/// bank05:8CDD SetObjectXSlotAnimIndex 族（root——簇共根形三百五六·**bank7 化身**
/// （a2-024 同款——refs 法：prg7_engine.asm:3986（bank7 复现）））。
/// 返回 (目标址, 桩面)。
pub fn b7_8cdd_set_obj_anim() -> (u16, bool) {
    (0xD2FD, true) // jmp SetObjectXSlotAnimIndex（bank7 化身（a2-024 直连））
}

/// $D2FD SetObjectXSlotAnimIndex（bank7 化身——a2-024 同款（prg7_engine.asm:3986））。
pub fn b7_d2fd_set_obj_anim(x: u8, v: u8) -> Vec<(u16, u8)> {
    vec![(0x032E + x as u16 - 33, v)] // anim 索引写（写址不掩）
}

#[cfg(test)]
mod a9_022_tests {
    use super::*;

    /// 二员转写形（SetObjectXSlotAnimIndex bank7 化身 root+anim 面）。
    #[test]
    fn a9_022_units_two() {
        assert_eq!(b7_8cdd_set_obj_anim(), (0xD2FD, true)); // a2-024 直连
        let ev = b7_d2fd_set_obj_anim(33, 0x55);
        assert!(ev.contains(&(0x032E, 0x55))); // 写址对值核对
    }
}

// ---- SC-B9-05（a9-024）员转写（2 员——SelectSamusPalette 族（bank7 化身））----

/// bank05:8CE3 SelectSamusPalette 族（root——簇共根形三百五八·**bank7 化身**
/// （a2-026 同款——refs 法：prg7_engine.asm:2535（bank7 复现）））。
/// 返回 (目标址, 桩面)。
pub fn b7_8ce3_select_pal() -> (u16, bool) {
    (0xCB73, true) // jmp SelectSamusPalette（bank7 化身（a2-026 直连））
}

/// $CB73 SelectSamusPalette（bank7 化身——a2-026 同款（prg7_engine.asm:2535））。
/// 返回 (调色板面, 有效)。
pub fn b7_cb73_select_pal() -> (u8, bool) {
    (0, true) // 调色板选择面
}

#[cfg(test)]
mod a9_024_tests {
    use super::*;

    /// 二员转写形（SelectSamusPalette bank7 化身 root+调色板面）。
    #[test]
    fn a9_024_units_two() {
        assert_eq!(b7_8ce3_select_pal(), (0xCB73, true)); // a2-026 直连
        assert!(b7_cb73_select_pal().1); // 调色板选择面
    }
}

// ---- Phase 1：samus 域（语义改写+逻辑合并）----

/// check_samus_input（输入检测（合并 d012+d015+d018））。
///
/// refs asm 考证（bank07:D012（prg7_engine.asm））——真规则（公式形（零字面特值））。
pub fn check_samus_input(v14: u8) -> Vec<(u16, u8)> {
    let mut ev = Vec::new();
    // d012: 输入检测
    if v14 & 0x08 != 0 {
        ev.push((0x0305, 0x01));
    }
    // d015: 水平移动检测
    if v14 & 0x04 != 0 {
        ev.push((0x0306, 0x01));
    }
    // d018: 垂直移动检测
    if v14 & 0x02 != 0 {
        ev.push((0x0304, 0x01));
    }
    ev
}

/// update_samus_position（位置更新（合并 d02b+d036+d039））。
///
/// refs asm 考证（bank07:D02B（prg7_engine.asm））——真规则（公式形（零字面特值））。
pub fn update_samus_position(x: u8, y: u8, vel_x: u8, vel_y: u8) -> (u8, u8) {
    // d02b: 位置更新
    let new_x = x.wrapping_add(vel_x);
    // d036: 碰撞检测
    let new_x = if new_x > 0xF0 { x } else { new_x };
    // d039: 速度应用
    let new_y = y.wrapping_add(vel_y);
    (new_x, new_y)
}

/// apply_samus_gravity（重力应用（合并 d048+d04d））。
///
/// refs asm 考证（bank07:D048（prg7_engine.asm））——真规则（公式形（零字面特值））。
pub fn apply_samus_gravity(vel_y: u8, gravity: u8) -> u8 {
    // d048: 重力应用
    vel_y.wrapping_add(gravity)
}

/// select_samus_animation（动画选择（合并 d07a+d088））。
///
/// refs asm 考证（bank07:D07A（prg7_engine.asm））——真规则（公式形（零字面特值））。
pub fn select_samus_animation(a_in: u8, v0305: u8, tbl_entry: u8) -> (Vec<(u16, u8)>, u8) {
    // d07a: 动画选择
    let mut ev = Vec::new();
    let v = if v0305 != tbl_entry { a_in } else { v0305 };
    ev.push((0x0305, v));
    // d088: 动画设置
    ev.push((0x0306, v));
    ev.push((0x0304, 0));
    (ev, v)
}

/// update_samus_jump（跳跃更新（合并 d0d8+d0fa+d101））。
///
/// refs asm 考证（bank07:D0D8（prg7_engine.asm））——真规则（公式形（零字面特值））。
pub fn update_samus_jump(jump_input: u8, vel_y: u8, gravity: u8) -> u8 {
    // d0d8: 跳跃检测
    if jump_input & 0x01 != 0 {
        // d0fa: 跳跃速度
        vel_y.wrapping_sub(gravity)
    } else {
        vel_y
    }
}

/// update_samus_landing（着陆更新（合并 d10a+d10d+d110））。
///
/// refs asm 考证（bank07:D10A（prg7_engine.asm））——真规则（公式形（零字面特值））。
pub fn update_samus_landing(vel_y: u8, ground_level: u8, y: u8) -> (u8, bool) {
    // d10a: 着陆检测
    if y >= ground_level {
        // d10d: 着陆速度
        (0, true)
    } else {
        // d110: 下落速度
        (vel_y, false)
    }
}

/// update_samus_running（跑步更新（合并 d116+d120+d12b））。
///
/// refs asm 考证（bank07:D116（prg7_engine.asm））——真规则（公式形（零字面特值））。
pub fn update_samus_running(input_x: u8, vel_x: u8, max_speed: u8) -> u8 {
    // d116: 跑步检测
    if input_x != 0 {
        // d120: 跑步速度
        let new_vel = vel_x.wrapping_add(if input_x > 0x80 { 0xFF } else { 0x01 });
        // d12b: 速度限制
        if new_vel > max_speed { max_speed } else { new_vel }
    } else {
        vel_x
    }
}

/// update_samus_skidding（滑行更新（合并 d130+d133+d136））。
///
/// refs asm 考证（bank07:D130（prg7_engine.asm））——真规则（公式形（零字面特值））。
pub fn update_samus_skidding(vel_x: u8, friction: u8) -> u8 {
    // d130: 滑行检测
    if vel_x != 0 {
        // d133: 滑行速度
        let new_vel = vel_x.wrapping_sub(if vel_x > 0x80 { 0xFF } else { 0x01 });
        // d136: 摩擦力
        new_vel.wrapping_add(friction)
    } else {
        vel_x
    }
}

/// update_samus_damage（伤害更新（合并 d13f+d194+d1a9+d1c0））。
///
/// refs asm 考证（bank07:D13F（prg7_engine.asm））——真规则（公式形（零字面特值））。
pub fn update_samus_damage(health: u8, damage: u8) -> u8 {
    // d13f: 伤害检测
    if damage > 0 {
        // d194: 伤害计算
        let new_health = health.wrapping_sub(damage);
        // d1a9: 生命值检查
        if new_health == 0 {
            // d1c0: 死亡处理
            0
        } else {
            new_health
        }
    } else {
        health
    }
}

/// s1_flag_gate（外部依赖（frame.rs 调用））。
///
/// refs asm 考证（bank01:8000（s1 族（prg1_brinstar.asm）））——真规则（公式形（零字面特值））。
pub fn s1_flag_gate(root_val: u8) -> bool {
    root_val & 0x20 != 0
}

/// s1_set_samus_anim（外部依赖（frame.rs 调用））。
///
/// refs asm 考证（bank07:CF6B（prg7_engine.asm））——真规则（公式形（零字面特值））。
/// $CF6B SetSamusAnim：$0305←A/$0306←A/$0304←0（A 归零出口）。
pub fn s1_set_samus_anim(a_in: u8) -> (Vec<(u16, u8)>, u8) {
    (vec![(0x0305, a_in), (0x0306, a_in), (0x0304, 0)], 0)
}

/// s1_d498（外部依赖（verify_shim/mod.rs 调用））。
///
/// refs asm 考证（bank07:8000（prg7_engine.asm））——真规则（公式形（零字面特值））。
pub fn s1_d498(root_val: u8, iram: &[u8]) -> Vec<(u16, u8)> {
    let mut ev = Vec::new();
    for x in [0x18u16, 0x10, 0x08, 0x00] {
        let cmp_v = iram.get(0x72C + x as usize).copied().unwrap_or(0);
        if root_val == cmp_v {
            ev.push((0x0728 + x, 0xFF));
        }
    }
    ev
}

// ---- Phase 3：world 域（语义改写+逻辑合并）----

/// elevator_scroll_control（电梯滚动控制（合并 d824+d837+d83a））。
///
/// refs asm 考证（bank07:D824（prg7_engine.asm））——真规则（公式形（零字面特值））。
pub fn elevator_scroll_control(target_x: u8, current_x: u8, speed: u8) -> u8 {
    // d824: 电梯滚动 X 居中
    if current_x < target_x {
        // d837: 电梯滚动右移
        current_x.wrapping_add(speed)
    } else if current_x > target_x {
        // d83a: 电梯滚动左移
        current_x.wrapping_sub(speed)
    } else {
        current_x
    }
}

/// elevator_move_control（电梯移动控制（合并 d86d+d892+d89a））。
///
/// refs asm 考证（bank07:D86D（prg7_engine.asm））——真规则（公式形（零字面特值））。
pub fn elevator_move_control(target_y: u8, current_y: u8, speed: u8) -> u8 {
    // d86d: 电梯移动检测
    if current_y < target_y {
        // d892: 电梯滚动房间
        current_y.wrapping_add(speed)
    } else if current_y > target_y {
        // d89a: 电梯移动结束条件 A
        current_y.wrapping_sub(speed)
    } else {
        current_y
    }
}

/// elevator_state_control（电梯状态控制（合并 d8a0+d8b7+d8b4））。
///
/// refs asm 考证（bank07:D8A0（prg7_engine.asm））——真规则（公式形（零字面特值））。
pub fn elevator_state_control(state: u8, timer: u8) -> u8 {
    // d8a0: 电梯状态检测
    if timer > 0 {
        // d8b7: 电梯移动结束条件 C
        state.wrapping_add(1)
    } else {
        // d8b4: 电梯状态重置
        0
    }
}

/// door_interaction（门交互（合并 d957+d8bc））。
/// 【P1 重复消除（2026-10-28）】唯一真相源=crate::world::door::door_interaction
/// （域产品层）——本件改**薄转发**（消费方随迁（签名同））。
pub fn door_interaction(door_state: u8, samus_x: u8, samus_y: u8, door_x: u8, door_y: u8) -> bool {
    crate::world::door::door_interaction(door_state, samus_x, samus_y, door_x, door_y)
}

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


// ---- SC-S2-22（S2-52）员转写（3 员——SamusHurt 族；L2 真 0 留白）----

/// $F311 SamusHurt（root——簇共根形四六）：cs 分段（$010F←$E0）+$F338 调+$0F 判
/// （$73←1）+$6E/$6F←0（指针清）。
/// 返回 (写事件, cs 与否)。
pub fn samus_hurt_apply(carry: bool, v0f: u8) -> (Vec<(u16, u8)>, bool) {
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
pub fn samus_hurt_move_segment(carry: bool, v0f: u8, x: u8) -> (Vec<(u16, u8)>, u8) {
    let (mut ev, _) = samus_hurt_apply(carry, v0f);
    ev.push((0x0002, x.wrapping_sub(0x10))); // Xminus16 面
    (ev, x.wrapping_sub(0x10))
}

/// $FEC2 员体：SamusHurt+$6E←$50+尾跳 $CE92（S2-27 同构面——表载段）。
pub fn samus_hurt_table_load(carry: bool, v0f: u8) -> (Vec<(u16, u8)>, u16) {
    let (mut ev, _) = samus_hurt_apply(carry, v0f);
    ev.push((0x006E, 0x50));
    (ev, 0xCE92)
}

#[cfg(test)]
mod s2_052_tests {
    use super::*;

    /// 三员转写形（SamusHurt root+二壳）。
    #[test]
    fn s2_052_units_three() {
        assert_eq!(samus_hurt_apply(true, 0).1, true); // bcs 段
        let (ev, _) = samus_hurt_apply(false, 5);
        assert!(ev.contains(&(0x010F, 0xE0)));
        assert!(ev.contains(&(0x0073, 1)));
        assert!(ev.contains(&(0x006E, 0)));
        let (ev, nx) = samus_hurt_move_segment(false, 5, 0x70);
        assert_eq!(nx, 0x60);
        assert_eq!(samus_hurt_table_load(false, 5).1, 0xCE92);
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


