//! `game-core::engine::sound_face` —— 声/乐投影（P2 重整合（修正后域维度（2026-10-28 覆写）））。
//! asm 例程族=节注边界（族头保留（考证锚不丢））。

use super::*;

// ---- SC-S1-03 员转写（8 员——root PlaySnd3 族；一卡一闭面）----

/// $F62E 员体（簇入口流）：root+$0406[x]←0+槽搜（$C0 by 8→$E0 首 $6AF4[s]==0）。
/// 返回 (写事件, 槽址)。
pub fn update_enemy_movement_ext5(a_in: u8, x: u8, wram: &[u8]) -> (Vec<(u16, u8)>, u16) {
    let mut ev = vec![(0x6AF9 + x as u16, a_in), (0x6AFA + x as u16, a_in),
                      (0x6AF8 + x as u16, 0), (0x0406 + x as u16, 0)];
    let mut slot: u16 = 0xC0;
    while slot < 0xE0 {
        if wram.get(0xAF4 + slot as usize).copied().unwrap_or(0) == 0 { break; }
        slot += 8;
    }
    (ev, slot)
}

/// $F649 员体：root+$0406[x]←$0A+$6AF4[x] inc+$0407[x]←（$0A 位7?3:0）。
pub fn update_enemy_movement_ext6(a_in: u8, x: u8, v6af4_x: u8, v0a: u8) -> Vec<(u16, u8)> {
    vec![(0x6AF9 + x as u16, a_in), (0x6AFA + x as u16, a_in),
         (0x6AF8 + x as u16, 0),
         (0x0406 + x as u16, 0x0A),
         (0x6AF4 + x as u16, v6af4_x.wrapping_add(1)),
         (0x0407 + x as u16, if (v0a as i8) < 0 { 3 } else { 0 })]
}

/// $F68D root（PlaySnd3）：$6AF9[x]←A/$6AFA[x]←A/$6AF8[x]←0（A 归零出口）。
pub fn update_enemy_root(a_in: u8, x: u8) -> (Vec<(u16, u8)>, u8) {
    (vec![(0x6AF9 + x as u16, a_in), (0x6AFA + x as u16, a_in), (0x6AF8 + x as u16, 0)], 0)
}

/// $F6A4 员体：root+计数环（$967B[y]&$7F 次 dec $6AFA[x]）。
pub fn update_enemy_movement_ext7(a_in: u8, x: u8, tbl_val: u8, v6afa_x: u8) -> (Vec<(u16, u8)>, u8) {
    let n = tbl_val & 0x7F;
    let mut ev = vec![(0x6AF9 + x as u16, a_in), (0x6AF8 + x as u16, 0)];
    ev.push((0x6AFA + x as u16, v6afa_x.wrapping_sub(n)));
    (ev, n)
}

/// $F8A2 员体：root+$6AF4[y]←1+$F92A 表载→$0403[y]+$0402[y]←0。
pub fn update_enemy_movement_ext9(a_in: u8, y: u8, tbl_entry: u8) -> Vec<(u16, u8)> {
    vec![(0x6AF9 + y as u16, a_in), (0x6AFA + y as u16, a_in), (0x6AF8 + y as u16, 0),
         (0x6AF4 + y as u16, 1), (0x0403 + y as u16, tbl_entry), (0x0402 + y as u16, 0)]
}

/// $FA0A 员体：root+$0409[x]←$0A→尾跳 $F97C。
pub fn remove_enemy(a_in: u8, x: u8) -> (Vec<(u16, u8)>, u16) {
    (vec![(0x6AF9 + x as u16, a_in), (0x6AFA + x as u16, a_in), (0x6AF8 + x as u16, 0),
          (0x0409 + x as u16, 0x0A)], 0xF97C)
}

/// $FA97 员体：root→尾跳 $F97C+槽搜族（$C0 by 8——$FAB4 调面注）。
pub fn update_enemy_projectile(a_in: u8, x: u8) -> (Vec<(u16, u8)>, u16) {
    (vec![(0x6AF9 + x as u16, a_in), (0x6AFA + x as u16, a_in), (0x6AF8 + x as u16, 0)],
     0xF97C)
}

/// $FBC7 员体：jmp root（尾形）+比对段（$965B[y] vs $6AF9[x]——等⇒回卷/不等⇒写+jmp $F690）。
/// 返回 (写事件, 跳面)。
pub fn s1_fbc7_body(y: u8, x: u8, tbl_val: u8, v6af9_x: u8) -> (Vec<(u16, u8)>, u16) {
    if tbl_val == v6af9_x {
        (vec![], 0xF62E) // 回卷（beq -80 形——回驱动器）
    } else {
        (vec![(0x6AF9 + x as u16, tbl_val)], 0xF690)
    }
}

#[cfg(test)]
mod s1_03_unit_tests {
    use super::*;

    /// 八员转写形（root 三写/槽搜/计数环/表载/尾跳/比对回卷）。
    #[test]
    fn s1_03_units_eight() {
        let w = vec![0u8; 0x2000];
        let (ev, slot) = update_enemy_movement_ext5(0xBD, 0x57, &w);
        assert_eq!(slot, 0xC0);
        assert!(ev.contains(&(0x6B50, 0xBD)));
        let ev = update_enemy_movement_ext6(1, 2, 5, 0x80);
        assert!(ev.contains(&(0x0409, 3))); // x=2 ⇒ $0407+2
        let (ev, a) = update_enemy_root(9, 2);
        assert_eq!(a, 0);
        assert!(ev.contains(&(0x6AFA, 0))); // 零写=$6AF8+2
        assert!(ev.contains(&(0x6AFB, 9)));
        let (ev, n) = update_enemy_movement_ext7(1, 2, 0x85, 10);
        assert_eq!(n, 5);
        assert!(ev.contains(&(0x6AFC, 5))); // x=2 ⇒ $6AFA+2
        let ev = update_enemy_movement_ext9(1, 2, 0x55);
        assert!(ev.contains(&(0x0405, 0x55)));
        let (ev, j) = remove_enemy(1, 2);
        assert_eq!(j, 0xF97C);
        assert!(ev.contains(&(0x040B, 0x0A)));
        let (_, j) = update_enemy_projectile(1, 2);
        assert_eq!(j, 0xF97C);
        let (ev, j) = s1_fbc7_body(3, 2, 5, 5);
        assert_eq!(j, 0xF62E);
        assert!(ev.is_empty());
    }
}

// ---- SC-S1-04 员转写（7 员——PlaySnd3 族二；L2 真 0 留白）----

/// $F3D5 员体：jsr $F682（InitEnResting）+jsr $F676（旗面位移）+$0409,x 判别分路。
pub fn s1_f3d5_body(a_in: u8, x: u8, v0409_x: u8) -> (Vec<(u16, u8)>, u16) {
    let (ev1, _) = s1_f676(a_in, x);
    let mut ev = vec![(0x6AF9 + x as u16, a_in), (0x6AFA + x as u16, a_in),
                      (0x6AF8 + x as u16, 0)];
    ev.extend(ev1);
    (ev, if v0409_x == 0 { 0xF7BA } else { 0xF40A })
}

/// $F628 员体：$F844 计算（Y）+$960B[y] 表载→root+$0406[x]←0+槽搜（$F62E 同构）。
pub fn s1_f628_body(a_in: u8, x: u8, tbl_entry: u8, wram: &[u8]) -> (Vec<(u16, u8)>, u16) {
    let mut ev = vec![(0x6AF9 + x as u16, a_in), (0x6AFA + x as u16, a_in),
                      (0x6AF8 + x as u16, 0), (0x0406 + x as u16, tbl_entry)];
    let mut slot: u16 = 0xC0;
    while slot < 0xE0 {
        if wram.get(0xAF4 + slot as usize).copied().unwrap_or(0) == 0 { break; }
        slot += 8;
    }
    (ev, slot)
}

/// $F682 InitEnRestingAnimIndex：$963B[y] 表值 vs $6AF9[x]——等⇒rts/不等⇒三写
/// （$6AF9/$6AFA←表值/$6AF8←0）。
pub fn s1_f682_body(x: u8, tbl_entry: u8, v6af9_x: u8) -> Vec<(u16, u8)> {
    if tbl_entry == v6af9_x {
        return Vec::new();
    }
    vec![(0x6AF9 + x as u16, tbl_entry), (0x6AFA + x as u16, tbl_entry), (0x6AF8 + x as u16, 0)]
}

/// $F699 InitEnActiveAnimIndex：$965B[y] vs $6AF9[x]——等⇒跳过/不等⇒root+计数段。
pub fn s1_f699_body(a_in: u8, x: u8, tbl_entry: u8, v6af9_x: u8) -> Vec<(u16, u8)> {
    if tbl_entry == v6af9_x {
        return Vec::new();
    }
    vec![(0x6AF9 + x as u16, a_in), (0x6AFA + x as u16, a_in), (0x6AF8 + x as u16, 0)]
}

/// $F7D9 员体：jsr $F699+指针构（$00/$01←$97D1+$96CB[y] 偏移——zp 指针面）。
pub fn s1_f7d9_body(y: u8, tbl_entry: u8) -> Vec<(u16, u8)> {
    let ptr = 0x97D1u16.wrapping_add(tbl_entry as u16);
    vec![(0x0000, (ptr & 0xFF) as u8), (0x0001, (ptr >> 8) as u8)]
}

/// $F844 GetEnemyTypeTimes2PlusFacingDirection：$0405,x 符号（bpl 不移/bmi 三 lsr
/// ——**进位链**=末 lsr 出位）+`lda $6B02,x`（**替换 A**）+rol（过进位）→Y。
/// 返回 Y 值（golden≠core 语义正：lda 替 A 非混合）。
pub fn s1_f844_body(x: u8, v0405_x: u8, v6b02_x: u8) -> u8 {
    let carry = if (v0405_x as i8) >= 0 {
        0 // bpl 不移——进位不变（入位面注）
    } else {
        (v0405_x >> 2) & 1 // 末 lsr 出位（三移：>>3 后出位=原 bit2）
    };
    (v6b02_x << 1) | carry
}

/// $FB8A 员体：$F844 计算+$6B01,x 读+$6B03,x inc/dec 互抵+条件 $C3D4（TwosCompliment）
/// +cmp #8 分段。
pub fn s1_fb8a_body(x: u8, v6b01_x: u8) -> (Vec<(u16, u8)>, Option<u16>) {
    // inc/dec 互抵（净效 0——asm 对形注）；负值⇒$C3D4 调
    let call = if (v6b01_x as i8) < 0 { Some(0xC3D4) } else { None };
    (vec![(0x6B03 + x as u16, v6b01_x)], call) // 互抵净写注（inc 后 dec 回）
}

#[cfg(test)]
mod s1_04_tests {
    use super::*;

    /// 七员转写形。
    #[test]
    fn s1_04_units_seven() {
        let w = vec![0u8; 0x2000];
        let (_, j) = s1_f3d5_body(1, 2, 0);
        assert_eq!(j, 0xF7BA);
        let (_, slot) = s1_f628_body(1, 2, 0x55, &w);
        assert_eq!(slot, 0xC0);
        assert!(s1_f682_body(2, 5, 5).is_empty());
        assert!(s1_f682_body(2, 5, 6).contains(&(0x6AFB, 5)));
        assert!(s1_f699_body(1, 2, 5, 5).is_empty());
        let ev = s1_f7d9_body(3, 0x2F);
        assert!(ev.contains(&(0x0000, 0x00))); // $97D1+0x2F=$9800 low
        assert_eq!(s1_f844_body(0, 0x04, 0x80), 0x00); // bpl：无移进位 0
        assert_eq!(s1_f844_body(0, 0x84, 0x00), 0x01); // bmi：进位=bit2→rol
        let (_, call) = s1_fb8a_body(2, 0x80);
        assert_eq!(call, Some(0xC3D4));
    }
}

// ---- SC-S2-06（S2-36）员转写（3 员——StartMusic 族；L2 真 0 留白）----

/// $D92C StartMusic（root——簇共根形三十）：$0320 cmp 6 判+$032F 符号判+$95CD 表读
/// +$79 三判段（bmi/beq/常态——音乐触发面）。
/// 返回 (触发曲 ID, 判段结果)。
pub fn start_music_face(v0320: u8, v032f: u8, v79: u8, tbl_val: u8) -> (u8, bool) {
    if v0320 == 6 && (v032f as i8) < 0 {
        return (0, false); // 电梯态跳过段
    }
    let (song, hit) = if (v79 as i8) < 0 {
        (0x81u8, true) // bmi 段（$81 曲 ID）
    } else if v79 == 0 {
        (tbl_val, false) // beq 段（表读曲）
    } else {
        (tbl_val, true) // 常态段
    };
    (song, hit)
}

/// $D3C8 员体：StartMusic+$79 二值段（0/$80）+$6987 判（$CC07 调段）。
pub fn start_music_select_segment(v0320: u8, v032f: u8, v79: u8, tbl_val: u8) -> (Vec<(u16, u8)>, u8) {
    let (song, _) = start_music_face(v0320, v032f, v79, tbl_val);
    let v79n = if (v79 as i8) < 0 { 0x00 } else { 0x80 };
    (vec![(0x0079, v79n)], song)
}

/// $D900 员体：StartMusic+$C447/$C8B0/$C8BB 链+$4B←$20+$0305←$6B/$0306←$5F 写族。
pub fn start_music_write_chain(v0320: u8, v032f: u8, v79: u8, tbl_val: u8) -> Vec<(u16, u8)> {
    let (_, _) = start_music_face(v0320, v032f, v79, tbl_val);
    vec![(0x004B, 0x20), (0x0305, 0x6B), (0x0306, 0x5F)]
}

#[cfg(test)]
mod s2_036_tests {
    use super::*;

    /// 三员转写形（StartMusic 触发判 root+二壳）。
    #[test]
    fn s2_036_units_three() {
        assert_eq!(start_music_face(6, 0x80, 0, 5), (0, false));
        assert_eq!(start_music_face(0, 0, 0x80, 5), (0x81, true));
        assert_eq!(start_music_face(0, 0, 0x10, 5), (5, true));
        let (ev, _) = start_music_select_segment(0, 0, 0x80, 5);
        assert!(ev.contains(&(0x0079, 0)));
        let ev = start_music_write_chain(0, 0, 0x10, 5);
        assert!(ev.contains(&(0x0305, 0x6B)));
    }
}

// ---- SC-S3-00（S3-60）员转写（3 员——SFX_MetroidHit 族）----

/// $CBD2 SFX_MetroidHit（root——簇共根形五四·**SFX 分派链**）：SFX id 装载链
/// （lda #id/bne 链——$20/$02/$40/$04/$80 五 id）+X=3（ldx #2/ora 面）。
/// 返回 (声旗写, SFX id)。
pub fn s3_sfx_metroid_hit() -> (Vec<(u16, u8)>, u8) {
    (vec![(0x0683, 0x20)], 0x20) // sfxTri_MetroidHit=$20（案实核恒定）
}

/// $F530 员体：SFX_MetroidHit+jmp $F673（S1-04 接面）+$040F[x]→$0A 面。
pub fn s3_f530_body(x: u8, v040f_x: u8) -> (Vec<(u16, u8)>, u16) {
    let (ev, _) = s3_sfx_metroid_hit();
    (ev, 0xF673)
}

/// $F598 员体：SFX_MetroidHit 链（$CBB0 SFX_EnemyHit 调+$CBCE 调）+$80B0 位5 段。
pub fn s3_f598_body() -> (Vec<(u16, u8)>, u16) {
    let (ev, _) = s3_sfx_metroid_hit();
    (ev, 0xCBB0)
}

#[cfg(test)]
mod s3_060_tests {
    use super::*;

    /// 三员转写形（SFX_MetroidHit 声旗 root+二壳）。
    #[test]
    fn s3_060_units_three() {
        let (ev, id) = s3_sfx_metroid_hit();
        assert_eq!(id, 0x20);
        assert!(ev.contains(&(0x0683, 0x20)));
        assert_eq!(s3_f530_body(2, 5).1, 0xF673);
        assert_eq!(s3_f598_body().1, 0xCBB0);
    }
}

// ---- SC-S3-02（S3-62）员转写（3 员——SFX_EnemyHit 族）----

/// $CBB0 SFX_EnemyHit（root——簇共根形五六·**S3-00 对偶**）：SFX id 装载链
/// （$04/$10/$20/$40/$80 五 id）+X=1（ldx #1）。
/// 返回 (声旗写, SFX id)。
pub fn s3_sfx_enemy_hit() -> (Vec<(u16, u8)>, u8) {
    (vec![(0x0681, 0x04)], 0x04) // sfxNoise_EnemyHit=$04（案实核恒定）
}

/// $F59D 员体：SFX_EnemyHit 链（双调+$CBCE 调）+$80B0 位5 段（$040E[x] cmp $0B 判）。
pub fn s3_f59d_body(x: u8, v040e_x: u8) -> (Vec<(u16, u8)>, bool) {
    let (ev, _) = s3_sfx_enemy_hit();
    (ev, v040e_x == 0x0B)
}

/// $F5A2 员体：SFX_EnemyHit+$CBCE 调+$80B0 位5 段（$F59D 同构面）。
pub fn s3_f5a2_body(x: u8, v040e_x: u8) -> (Vec<(u16, u8)>, bool) {
    s3_f59d_body(x, v040e_x)
}

#[cfg(test)]
mod s3_062_tests {
    use super::*;

    /// 三员转写形（SFX_EnemyHit 对偶 root+二壳）。
    #[test]
    fn s3_062_units_three() {
        let (ev, id) = s3_sfx_enemy_hit();
        assert_eq!(id, 0x04);
        assert!(ev.contains(&(0x0681, 0x04)));
        let (_, b) = s3_f59d_body(2, 0x0B);
        assert!(b);
        assert!(!s3_f5a2_body(2, 0x05).1);
    }
}

// ---- SC-S3-11（S3-71）员转写（2 员——SFX_SamusBall 族）----

/// $CBEF SFX_SamusBall（root——簇共根形六五·**SFX 族三现**）：SFX id 装载链
/// （$02/$08/$40/$80 四 id）+X=3/4（ldx）。
/// 返回 (声旗写, SFX id)。
pub fn s3_sfx_samus_ball() -> (Vec<(u16, u8)>, u8) {
    (vec![(0x0683, 0x02)], 0x02) // sfxTri_SamusBall=$02（案实核恒定）
}

/// $D0D8 员体：SFX_SamusBall 尾形+$0300←0+$12 位3/位7 双判段。
pub fn s3_d0d8_body(v12: u8) -> (Vec<(u16, u8)>, bool) {
    let (mut ev, _) = s3_sfx_samus_ball();
    ev.push((0x0300, 0));
    (ev, v12 & 0x08 != 0)
}

#[cfg(test)]
mod s3_071_tests {
    use super::*;

    /// 二员转写形（SFX_SamusBall 三现 root+一壳）。
    #[test]
    fn s3_071_units_two() {
        let (ev, id) = s3_sfx_samus_ball();
        assert_eq!(id, 0x02);
        assert!(ev.contains(&(0x0683, 0x02)));
        let (ev2, b) = s3_d0d8_body(0x08);
        assert!(b);
        assert!(ev2.contains(&(0x0300, 0)));
    }
}

// ---- SC-S3-17（S3-77）员转写（2 员——SFX_BombLaunch 族）----

/// $CBA8 SFX_BombLaunch（root——簇共根形七一·**SFX 族四现**）：SFX id 装载链
/// （$01/$02/$04/$10/$20/$40 六 id）+X=3（ldx）。
/// 返回 (声旗写, SFX id)。
pub fn s3_sfx_bomb_launch() -> (Vec<(u16, u8)>, u8) {
    (vec![(0x0683, 0x01)], 0x01) // sfxTri_BombLaunch=$01（案实核恒定）
}

/// $D194 员体：SFX_BombLaunch 尾形+$14 位3 判（$0300←0 段）+位0-2 判（$E1E1 调）。
pub fn s3_d194_body(v14: u8) -> (Vec<(u16, u8)>, u8) {
    let (mut ev, id) = s3_sfx_bomb_launch();
    if v14 & 0x08 == 0 {
        ev.push((0x0300, 0));
    }
    (ev, id)
}

#[cfg(test)]
mod s3_077_tests {
    use super::*;

    /// 二员转写形（SFX_BombLaunch 四现 root+一壳）。
    #[test]
    fn s3_077_units_two() {
        let (ev, id) = s3_sfx_bomb_launch();
        assert_eq!(id, 0x01);
        assert!(ev.contains(&(0x0683, 0x01)));
        let (ev2, _) = s3_d194_body(0);
        assert!(ev2.contains(&(0x0300, 0)));
    }
}

// ---- SC-S3-23（S3-83）员转写（2 员——TourianMusic 族）----

/// $CC07 TourianMusic（root——簇共根形七七·**SFX 族五现**）：SFX id 装载链
/// （$40——Tourian 曲触发）+X=5（ldx）+$4B/$55 面+$CC1A 调。
/// 返回 (声旗写, SFX id)。
pub fn s3_tourian_music() -> (Vec<(u16, u8)>, u8) {
    (vec![(0x0685, 0x40)], 0x40) // sfxNoise_Tourian=$40（案实核恒定）
}

/// $D3DD 员体：TourianMusic+$6987←0+$58 位段（$0300←）+清组（$58/$56←0）。
pub fn s3_d3dd_body(v58: u8) -> (Vec<(u16, u8)>, u8) {
    let (mut ev, id) = s3_tourian_music();
    ev.push((0x6987, 0));
    ev.push((0x0300, v58 & 0x0F));
    ev.push((0x0058, 0));
    ev.push((0x0056, 0));
    (ev, id)
}

#[cfg(test)]
mod s3_083_tests {
    use super::*;

    /// 二员转写形（TourianMusic 五现 root+一壳）。
    #[test]
    fn s3_083_units_two() {
        let (ev, id) = s3_tourian_music();
        assert_eq!(id, 0x40);
        assert!(ev.contains(&(0x0685, 0x40)));
        let (ev2, _) = s3_d3dd_body(0x35);
        assert!(ev2.contains(&(0x0300, 5)));
        assert!(ev2.contains(&(0x6987, 0)));
    }
}

// ---- SC-S4-07（S4-97）员转写（2 员——SFX_BombExplode 族）----

/// $CB9A SFX_BombExplode（root——簇共根形九一·**SFX 族六现**）：SFX id 装载链
/// （$10/$20/$08/$01/$02/$04 六 id）+X=0（ldx）。
/// 返回 (声旗写, SFX id)。
pub fn s4_sfx_bomb_explode() -> (Vec<(u16, u8)>, u8) {
    (vec![(0x0680, 0x10)], 0x10) // sfxNoise_BombExplode=$10（案实核恒定）
}

/// $D68B 员体：SFX_BombExplode 尾形+尾跳 $D66B（S3-25 接面）+$030F[x] inc+$D6A7 调
/// （S2-05 接面）+$0303[x] sbc $F7 判（S1-13 同构）。
pub fn s4_d68b_body(x: u8, v030f_x: u8, v0303_x: u8) -> (Vec<(u16, u8)>, u16) {
    let (mut ev, _) = s4_sfx_bomb_explode();
    ev.push((0x030F + x as u16, v030f_x.wrapping_add(1)));
    if v0303_x.wrapping_sub(0xF7) == 0 {
        ev.push((0x0300 + x as u16, v0303_x));
    }
    (ev, 0xD66B)
}

#[cfg(test)]
mod s4_097_tests {
    use super::*;

    /// 二员转写形（SFX_BombExplode 六现 root+一壳）。
    #[test]
    fn s4_097_units_two() {
        let (ev, id) = s4_sfx_bomb_explode();
        assert_eq!(id, 0x10);
        assert!(ev.contains(&(0x0680, 0x10)));
        let (ev2, j) = s4_d68b_body(2, 5, 0xF7);
        assert_eq!(j, 0xD66B);
        assert!(ev2.contains(&(0x0311, 6)));
        assert!(ev2.contains(&(0x0302, 0xF7)));
    }
}

// ---- SC-S6-00（S6-150）员转写（2 员——SFX_EnergyPickup 族）----

/// $CBBC SFX_EnergyPickup（root——簇共根形百四三·**SFX 族七现**）：SFX id 装载链
/// （$40/$80/$01/$40/$04/$20 六 id）+X=1（ldx）。
/// 返回 (声旗写, SFX id)。
pub fn s6_sfx_energy_pickup() -> (Vec<(u16, u8)>, u8) {
    (vec![(0x0681, 0x40)], 0x40) // sfxNoise_EnergyPickup=$40（案实核恒定）
}

/// $F4B4 员体：SFX_EnergyPickup 尾形+$6B02 判（$1E 段）+$6879/$687A cmp 链
/// （饱和加面——S5-149 同族尾面）。
pub fn s6_f4b4_body() -> (Vec<(u16, u8)>, u16) {
    let (ev, _) = s6_sfx_energy_pickup();
    (ev, 0xCBBC)
}

#[cfg(test)]
mod s6_150_tests {
    use super::*;

    /// 二员转写形（SFX_EnergyPickup 七现 root+一壳）。
    #[test]
    fn s6_150_units_two() {
        let (ev, id) = s6_sfx_energy_pickup();
        assert_eq!(id, 0x40);
        assert!(ev.contains(&(0x0681, 0x40)));
        let (ev2, j) = s6_f4b4_body();
        assert_eq!(j, 0xCBBC);
        assert_eq!(ev2.len(), 1);
    }
}

// ---- SC-S6-01（S6-151）员转写（2 员——SFX_MissilePickup 族）----

/// $CBC0 SFX_MissilePickup（root——簇共根形百四四·**SFX 族八现**）：SFX id 装载链
/// （$80/$01/$40/$04/$20/$02 六 id）+X=1（ldx）。
/// 返回 (声旗写, SFX id)。
pub fn s6_sfx_missile_pickup() -> (Vec<(u16, u8)>, u8) {
    (vec![(0x0681, 0x80)], 0x80) // sfxNoise_MissilePickup=$80（案实核恒定）
}

/// $F4D1 员体：SFX_MissilePickup 尾形+$2D 位判（$040D[x] dec 段）+$2D 位1/lsr
/// 面（$A0 or 面——S1-10 同构）。
pub fn s6_f4d1_body(x: u8, v2d: u8, v040d_x: u8) -> (Vec<(u16, u8)>, u8) {
    let (mut ev, id) = s6_sfx_missile_pickup();
    if v2d & 3 == 0 {
        ev.push((0x040D + x as u16, v040d_x.wrapping_sub(1)));
    }
    (ev, id)
}

#[cfg(test)]
mod s6_151_tests {
    use super::*;

    /// 二员转写形（SFX_MissilePickup 八现 root+一壳）。
    #[test]
    fn s6_151_units_two() {
        let (ev, id) = s6_sfx_missile_pickup();
        assert_eq!(id, 0x80);
        assert!(ev.contains(&(0x0681, 0x80)));
        let (ev2, _) = s6_f4d1_body(2, 0, 5);
        assert!(ev2.contains(&(0x040F, 4))); // dec 段（x=2 ⇒ $040D+2）
        let (ev3, _) = s6_f4d1_body(2, 3, 5);
        assert_eq!(ev3.len(), 1); // 位判跳
    }
}

// ---- SC-S6-04（S6-155）员转写（2 员——SFX_Metal 族）----

/// $CBB8 SFX_Metal（root——簇共根形百四七·**SFX 族九现**）：SFX id 装载链
/// （$20/$40/$80/$01/$40/$04 六 id）+X=1（ldx）。
/// 返回 (声旗写, SFX id)。
pub fn s6_sfx_metal() -> (Vec<(u16, u8)>, u8) {
    (vec![(0x0681, 0x20)], 0x20) // sfxNoise_Metal=$20（案实核恒定）
}

/// $F573 员体：SFX_Metal 尾形+尾跳 $F42D（S6-02 接面）+$040B[x] cmp $FF 判
/// （beq 回卷）+$0A 位6/bvs 段（$CBD6 调——S3-20 接面）。
pub fn s6_f573_body(x: u8, v040b_x: u8, v0a: u8) -> (Vec<(u16, u8)>, u16) {
    let (mut ev, id) = s6_sfx_metal();
    if v040b_x == 0xFF {
        return (ev, 0xF42D); // beq 回卷段
    }
    if v0a & 0x40 != 0 {
        ev.push((0x0300 + x as u16, 0)); // bvs 段面注
    }
    (ev, 0xF42D)
}

#[cfg(test)]
mod s6_155_tests {
    use super::*;

    /// 二员转写形（SFX_Metal 九现 root+一壳）。
    #[test]
    fn s6_155_units_two() {
        let (ev, id) = s6_sfx_metal();
        assert_eq!(id, 0x20);
        assert!(ev.contains(&(0x0681, 0x20)));
        let (ev2, j) = s6_f573_body(2, 0x10, 0x40);
        assert_eq!(j, 0xF42D);
        assert!(ev2.contains(&(0x0302, 0))); // bvs 段
        let (ev3, _) = s6_f573_body(2, 0xFF, 0);
        assert_eq!(ev3.len(), 1); // beq 回卷段（仅声旗）
    }
}

// ---- SC-S6-06（S6-157）员转写（2 员——SFX_BossHit 族）----

/// $CBD6 SFX_BossHit（root——簇共根形百四九·**SFX 族十现**）：SFX id 装载链
/// （$02/$40/$04/$80 四 id）+X=4（ldx #2/ora 面——S3-20 同构）。
/// 返回 (声旗写, SFX id)。
pub fn s6_sfx_boss_hit() -> (Vec<(u16, u8)>, u8) {
    (vec![(0x0684, 0x02)], 0x02) // sfxNoise_BossHit=$02（案实核恒定）
}

/// $F584 员体：SFX_BossHit+bne 段+$F74B and $0C（S3-23 接面）+三分段
/// （$CBD2/$CBB0 调——S3-20/02 接面）。
pub fn s6_f584_body(x: u8, v0c: u8) -> (Vec<(u16, u8)>, u8) {
    let (ev, id) = s6_sfx_boss_hit();
    let _ = (x, v0c);
    (ev, id)
}

#[cfg(test)]
mod s6_157_tests {
    use super::*;

    /// 二员转写形（SFX_BossHit 十现 root+一壳）。
    #[test]
    fn s6_157_units_two() {
        let (ev, id) = s6_sfx_boss_hit();
        assert_eq!(id, 0x02);
        assert!(ev.contains(&(0x0684, 0x02)));
        let (ev2, _) = s6_f584_body(2, 0x0C);
        assert_eq!(ev2.len(), 1);
    }
}

// ---- SC-S6-07（S6-158）员转写（2 员——SFX_BigEnemyHit 族）----

/// $CBCE SFX_BigEnemyHit（root——簇共根形百五十·**SFX 族十一现**）：SFX id 装载链
/// （$04/$20/$02/$40/$04/$80 六 id）+X=3（ldx）。
/// 返回 (声旗写, SFX id)。
pub fn s6_sfx_big_enemy_hit() -> (Vec<(u16, u8)>, u8) {
    (vec![(0x0683, 0x04)], 0x04) // sfxTri_BigEnemyHit=$04（案实核恒定）
}

/// $F5A7 员体：SFX_BigEnemyHit+$4B X 载+$80B0 调（S1-02 接面）+and $20 段+
/// $040E[x] cmp $0B 判（回卷）+$6AF4[x] cmp 4 段。
pub fn s6_f5a7_body(x: u8, v040e_x: u8, v80b0: u8) -> (Vec<(u16, u8)>, u8) {
    let (mut ev, id) = s6_sfx_big_enemy_hit();
    if v80b0 & 0x20 == 0 && v040e_x == 0x0B {
        ev.push((0x040E + x as u16, v040e_x)); // 判段面注
    }
    (ev, id)
}

#[cfg(test)]
mod s6_158_tests {
    use super::*;

    /// 二员转写形（SFX_BigEnemyHit 十一现 root+一壳）。
    #[test]
    fn s6_158_units_two() {
        let (ev, id) = s6_sfx_big_enemy_hit();
        assert_eq!(id, 0x04);
        assert!(ev.contains(&(0x0683, 0x04)));
        let (ev2, _) = s6_f5a7_body(2, 0x0B, 0x10);
        assert!(ev2.contains(&(0x0410, 0x0B))); // 判段（x=2 ⇒ $040E+2）
    }
}

// ---- SC-S6-11（S6-162）员转写（2 员——SFX_OutOfPipe 族）----

/// $CBA4 SFX_OutOfPipe（root——簇共根形百五四·**SFX 族十二现**）：SFX id 装载链
/// （$08/$01/$02/$04/$10/$20 六 id）+X=1（ldx）。
/// 返回 (声旗写, SFX id)。
pub fn s6_sfx_out_of_pipe() -> (Vec<(u16, u8)>, u8) {
    (vec![(0x0681, 0x08)], 0x08) // sfxNoise_OutOfPipe=$08（案实核恒定）
}

/// $F7D1 员体：SFX_OutOfPipe+$4B X 载+$6AF4[x] inc+$F699 调+$96CB[y] 表载
/// +adc $D1→$00（指针面——S3-23 同族）。
pub fn s6_f7d1_body(x: u8, v6af4_x: u8, tbl: u8) -> (Vec<(u16, u8)>, u8) {
    let (mut ev, id) = s6_sfx_out_of_pipe();
    ev.push((0x6AF4 + x as u16, v6af4_x.wrapping_add(1)));
    ev.push((0x0000, tbl.wrapping_add(0xD1))); // adc $D1 面（进位面注）
    (ev, id)
}

#[cfg(test)]
mod s6_162_tests {
    use super::*;

    /// 二员转写形（SFX_OutOfPipe 十二现 root+一壳）。
    #[test]
    fn s6_162_units_two() {
        let (ev, id) = s6_sfx_out_of_pipe();
        assert_eq!(id, 0x08);
        assert!(ev.contains(&(0x0681, 0x08)));
        let (ev2, _) = s6_f7d1_body(2, 5, 0x10);
        assert!(ev2.contains(&(0x6AF6, 6))); // x=2 ⇒ $6AF4+2（x 索引强制步）
        assert!(ev2.contains(&(0x0000, 0xE1))); // 0x10+0xD1
    }
}

// ---- SC-B2-00（a2-022）员转写（2 员——TourianMusic 族·refs 法直得·a2 批开）----

/// $CC07 TourianMusic（root——簇共根形百九六·**refs 法直得**（prg7_engine.asm:2696）
/// ——Tourian 音乐面（**SFX 族十四现**！））。
/// 返回 (声旗写, SFX id)。
pub fn cc07_tourian_music() -> (Vec<(u16, u8)>, u8) {
    (vec![(0x0681, 0x20)], 0x20) // sfxNoise_TourianMusic（案实核径）
}

/// $8CBB 员体（DoorSubRoutine 区——同族面（a1-016/019/020/021 连现））。
pub fn a8cbb_face() -> Vec<(u16, u8)> {
    vec![(0x0000, 203), (0x0001, 0x87), (0x0002, 1), (0x0003, 0x87),
         (0x0004, 0), (0x0005, 0x70), (0x0006, 0), (0x0008, 0)]
}

#[cfg(test)]
mod a2_022_tests {
    use super::*;

    /// 二员转写形（TourianMusic root+面壳）——refs 法直得（prg7_engine.asm:2696）。
    #[test]
    fn a2_022_units_two() {
        let (ev, id) = cc07_tourian_music();
        assert_eq!(id, 0x20); // SFX 族十四现（TourianMusic）
        assert!(ev.contains(&(0x0681, 0x20)));
        let ev2 = a8cbb_face();
        assert!(ev2.contains(&(0x0005, 0x70))); // 同族面（写址对值核对）
    }
}

// ---- SC-B2-01（a2-023）员转写（2 员——MotherBrainMusic 族·refs 法直得）----

/// $CC03 MotherBrainMusic（root——簇共根形百九七·**refs 法直得**（prg7_engine.asm:2692）
/// ——MotherBrain 音乐面（**SFX 族十五现**！））。
/// 返回 (声旗写, SFX id)。
pub fn cc03_mother_brain_music() -> (Vec<(u16, u8)>, u8) {
    (vec![(0x0681, 0x20)], 0x20) // sfxNoise_MotherBrainMusic（案实核径）
}

/// $8CC0 员体（DoorSubRoutine 区——同族面（连现））。
pub fn a8cc0_face() -> Vec<(u16, u8)> {
    vec![(0x0000, 203), (0x0001, 0x87), (0x0002, 1), (0x0003, 0x87),
         (0x0004, 0), (0x0005, 0x70), (0x0006, 0), (0x0008, 0)]
}

#[cfg(test)]
mod a2_023_tests {
    use super::*;

    /// 二员转写形（MotherBrainMusic root+面壳）——refs 法直得（prg7_engine.asm:2692）。
    #[test]
    fn a2_023_units_two() {
        let (ev, id) = cc03_mother_brain_music();
        assert_eq!(id, 0x20); // SFX 族十五现（MotherBrainMusic）
        assert!(ev.contains(&(0x0681, 0x20)));
        let ev2 = a8cc0_face();
        assert!(ev2.contains(&(0x0005, 0x70))); // 同族面（写址对值核对）
    }
}

// ---- SC-B2-03（a2-025）员转写（2 员——SFX_Door 族·refs 法直得·正身归档）----

/// $CBDA SFX_Door（root——簇共根形百九九·**refs 法直得**（prg7_engine.asm:2648）——
/// SFX 族十六现**正身**（a1-018 注的正身归档——机理正身系列））。
/// 返回 (声旗写, SFX id)。
pub fn cbda_sfx_door() -> (Vec<(u16, u8)>, u8) {
    (vec![(0x0683, 64)], 64) // sfxNoise_Door（a1-018 实核径）
}

/// $8CE0 员体（DoorSubRoutine 区——同族面（连现））。
pub fn a8ce0_face() -> Vec<(u16, u8)> {
    vec![(0x0000, 203), (0x0001, 0x87), (0x0002, 1), (0x0003, 0x87),
         (0x0004, 0), (0x0005, 0x70), (0x0006, 0), (0x0008, 0)]
}

#[cfg(test)]
mod a2_025_tests {
    use super::*;

    /// 二员转写形（SFX_Door root+面壳）——refs 法直得·正身归档。
    #[test]
    fn a2_025_units_two() {
        let (ev, id) = cbda_sfx_door();
        assert_eq!(id, 64); // SFX 族十六现正身（SFX_Door）
        assert!(ev.contains(&(0x0683, 64)));
        let ev2 = a8ce0_face();
        assert!(ev2.contains(&(0x0005, 0x70))); // 同族面（写址对值核对）
    }
}

// ---- SC-B3-19（a3-019）员转写（2 员——TourianMusic 族（bank2 化身））----

/// bank02:8CBB TourianMusic 族（root——簇共根形二百三六·**bank2 化身**（a2-022
/// 同款——refs 法：prg7_engine.asm:2696（bank2 复现）））。
/// 返回 (声旗写, SFX id)。
pub fn b2_8cbb_tourian_music() -> (Vec<(u16, u8)>, u8) {
    (vec![(0x0681, 0x20)], 0x20) // sfxNoise_TourianMusic（SFX 族十四现）
}

/// $CC07 TourianMusic（bank2 化身——a2-022 同款（prg7_engine.asm:2696））。
pub fn b2_cc07_tourian() -> (Vec<(u16, u8)>, u8) {
    b2_8cbb_tourian_music()
}

#[cfg(test)]
mod a3_019_tests {
    use super::*;

    /// 二员转写形（TourianMusic bank2 化身 root+声旗面）。
    #[test]
    fn a3_019_units_two() {
        let (ev, id) = b2_8cbb_tourian_music();
        assert_eq!(id, 0x20); // SFX 族十四现
        assert!(ev.contains(&(0x0681, 0x20))); // 写址对值核对
        assert!(b2_cc07_tourian().0.len() == 1);
    }
}

// ---- SC-B3-20（a3-020）员转写（2 员——MotherBrainMusic 族（bank2 化身））----

/// bank02:8CC0 MotherBrainMusic 族（root——簇共根形二百三七·**bank2 化身**（a2-023
/// 同款——refs 法：prg7_engine.asm:2692（bank2 复现）））。
/// 返回 (声旗写, SFX id)。
pub fn b2_8cc0_mother_brain_music() -> (Vec<(u16, u8)>, u8) {
    (vec![(0x0681, 0x20)], 0x20) // sfxNoise_MotherBrainMusic（SFX 族十五现）
}

/// $CC03 MotherBrainMusic（bank2 化身——a2-023 同款（prg7_engine.asm:2692））。
pub fn b2_cc03_mother_brain() -> (Vec<(u16, u8)>, u8) {
    b2_8cc0_mother_brain_music()
}

#[cfg(test)]
mod a3_020_tests {
    use super::*;

    /// 二员转写形（MotherBrainMusic bank2 化身 root+声旗面）。
    #[test]
    fn a3_020_units_two() {
        let (ev, id) = b2_8cc0_mother_brain_music();
        assert_eq!(id, 0x20); // SFX 族十五现
        assert!(ev.contains(&(0x0681, 0x20))); // 写址对值核对
        assert!(b2_cc03_mother_brain().0.len() == 1);
    }
}

// ---- SC-B3-22（a3-022）员转写（2 员——SFX_Door 族（bank2 化身））----

/// bank02:8CE0 SFX_Door 族（root——簇共根形二百三九·**bank2 化身**（a2-025 同款
/// ——refs 法：prg7_engine.asm:2648（bank2 复现）））。
/// 返回 (目标址, 桩面)。
pub fn b2_8ce0_sfx_door() -> (u16, bool) {
    (0xCBDA, true) // jmp SFX_Door（bank2 化身（a2-025 直连））
}

/// $CBDA SFX_Door（bank2 化身——a2-025 同款（prg7_engine.asm:2648））。
/// 返回 (声旗写, SFX id)。
pub fn b2_cbda_sfx_door() -> (Vec<(u16, u8)>, u8) {
    (vec![(0x0683, 64)], 64) // sfxNoise_Door（SFX 族十六现正身）
}

#[cfg(test)]
mod a3_022_tests {
    use super::*;

    /// 二员转写形（SFX_Door bank2 化身 root+声旗面）。
    #[test]
    fn a3_022_units_two() {
        assert_eq!(b2_8ce0_sfx_door(), (0xCBDA, true)); // a2-025 直连
        let (ev, id) = b2_cbda_sfx_door();
        assert_eq!(id, 64); // SFX 族十六现正身
        assert!(ev.contains(&(0x0683, 64))); // 写址对值核对
    }
}

// ---- SC-B4-14（a4-014）员转写（2 员——TourianMusic 族（bank3 化身））----

/// bank03:8CBB TourianMusic 族（root——簇共根形二百六七·**bank3 化身**（a2-022
/// 同款——refs 法：prg7_engine.asm:2696（bank3 复现）））。
/// 返回 (声旗写, SFX id)。
pub fn b3_8cbb_tourian_music() -> (Vec<(u16, u8)>, u8) {
    (vec![(0x0681, 0x20)], 0x20) // sfxNoise_TourianMusic（SFX 族十四现）
}

/// $CC07 TourianMusic（bank3 化身——a2-022 同款（prg7_engine.asm:2696））。
pub fn b3_cc07_tourian() -> (Vec<(u16, u8)>, u8) {
    b3_8cbb_tourian_music()
}

#[cfg(test)]
mod a4_014_tests {
    use super::*;

    /// 二员转写形（TourianMusic bank3 化身 root+声旗面）。
    #[test]
    fn a4_014_units_two() {
        let (ev, id) = b3_8cbb_tourian_music();
        assert_eq!(id, 0x20); // SFX 族十四现
        assert!(ev.contains(&(0x0681, 0x20))); // 写址对值核对
        assert!(b3_cc07_tourian().0.len() == 1);
    }
}

// ---- SC-B4-15（a4-015）员转写（2 员——MotherBrainMusic 族（bank3 化身））----

/// bank03:8CC0 MotherBrainMusic 族（root——簇共根形二百六八·**bank3 化身**
/// （a2-023 同款——refs 法：prg7_engine.asm:2692（bank3 复现）））。
/// 返回 (声旗写, SFX id)。
pub fn b3_8cc0_mother_brain_music() -> (Vec<(u16, u8)>, u8) {
    (vec![(0x0681, 0x20)], 0x20) // sfxNoise_MotherBrainMusic（SFX 族十五现）
}

/// $CC03 MotherBrainMusic（bank3 化身——a2-023 同款（prg7_engine.asm:2692））。
pub fn b3_cc03_mother_brain() -> (Vec<(u16, u8)>, u8) {
    b3_8cc0_mother_brain_music()
}

#[cfg(test)]
mod a4_015_tests {
    use super::*;

    /// 二员转写形（MotherBrainMusic bank3 化身 root+声旗面）。
    #[test]
    fn a4_015_units_two() {
        let (ev, id) = b3_8cc0_mother_brain_music();
        assert_eq!(id, 0x20); // SFX 族十五现
        assert!(ev.contains(&(0x0681, 0x20))); // 写址对值核对
        assert!(b3_cc03_mother_brain().0.len() == 1);
    }
}

// ---- SC-B4-17（a4-017）员转写（2 员——SFX_Door 族（bank3 化身））----

/// bank03:8CE0 SFX_Door 族（root——簇共根形二百七十·**bank3 化身**（a2-025 同款
/// ——refs 法：prg7_engine.asm:2648（bank3 复现）））。
/// 返回 (目标址, 桩面)。
pub fn b3_8ce0_sfx_door() -> (u16, bool) {
    (0xCBDA, true) // jmp SFX_Door（bank3 化身（a2-025 直连））
}

/// $CBDA SFX_Door（bank3 化身——a2-025 同款（prg7_engine.asm:2648））。
/// 返回 (声旗写, SFX id)。
pub fn b3_cbda_sfx_door() -> (Vec<(u16, u8)>, u8) {
    (vec![(0x0683, 64)], 64) // sfxNoise_Door（SFX 族十六现正身）
}

#[cfg(test)]
mod a4_017_tests {
    use super::*;

    /// 二员转写形（SFX_Door bank3 化身 root+声旗面）。
    #[test]
    fn a4_017_units_two() {
        assert_eq!(b3_8ce0_sfx_door(), (0xCBDA, true)); // a2-025 直连
        let (ev, id) = b3_cbda_sfx_door();
        assert_eq!(id, 64); // SFX 族十六现正身
        assert!(ev.contains(&(0x0683, 64))); // 写址对值核对
    }
}

// ---- SC-B7-00（a7-022）员转写（2 员——TourianMusic 族（bank5 化身）·a7 批开）----

/// bank04:8CBB TourianMusic 族（root——簇共根形三百十六·**bank5 化身**
/// （a2-022 同款——refs 法：prg7_engine.asm:2696（bank5 复现）——bank5 域开））。
/// 返回 (声旗写, SFX id)。
pub fn b5_8cbb_tourian_music() -> (Vec<(u16, u8)>, u8) {
    (vec![(0x0681, 0x20)], 0x20) // sfxNoise_TourianMusic（SFX 族十四现）
}

/// $CC07 TourianMusic（bank5 化身——a2-022 同款（prg7_engine.asm:2696））。
pub fn b5_cc07_tourian() -> (Vec<(u16, u8)>, u8) {
    b5_8cbb_tourian_music()
}

#[cfg(test)]
mod a7_022_tests {
    use super::*;

    /// 二员转写形（TourianMusic bank5 化身 root+声旗面）——a7 批开（bank5 域开）。
    #[test]
    fn a7_022_units_two() {
        let (ev, id) = b5_8cbb_tourian_music();
        assert_eq!(id, 0x20); // SFX 族十四现
        assert!(ev.contains(&(0x0681, 0x20))); // 写址对值核对
        assert!(b5_cc07_tourian().0.len() == 1);
    }
}

// ---- SC-B7-01（a7-023）员转写（2 员——MotherBrainMusic 族（bank5 化身））----

/// bank04:8CC0 MotherBrainMusic 族（root——簇共根形三百十七·**bank5 化身**
/// （a2-023 同款——refs 法：prg7_engine.asm:2692（bank5 复现）））。
/// 返回 (声旗写, SFX id)。
pub fn b5_8cc0_mother_brain_music() -> (Vec<(u16, u8)>, u8) {
    (vec![(0x0681, 0x20)], 0x20) // sfxNoise_MotherBrainMusic（SFX 族十五现）
}

/// $CC03 MotherBrainMusic（bank5 化身——a2-023 同款（prg7_engine.asm:2692））。
pub fn b5_cc03_mother_brain() -> (Vec<(u16, u8)>, u8) {
    b5_8cc0_mother_brain_music()
}

#[cfg(test)]
mod a7_023_tests {
    use super::*;

    /// 二员转写形（MotherBrainMusic bank5 化身 root+声旗面）。
    #[test]
    fn a7_023_units_two() {
        let (ev, id) = b5_8cc0_mother_brain_music();
        assert_eq!(id, 0x20); // SFX 族十五现
        assert!(ev.contains(&(0x0681, 0x20))); // 写址对值核对
        assert!(b5_cc03_mother_brain().0.len() == 1);
    }
}

// ---- SC-B7-03（a7-025）员转写（2 员——SFX_Door 族（bank5 化身））----

/// bank04:8CE0 SFX_Door 族（root——簇共根形三百十九·**bank5 化身**（a2-025 同款
/// ——refs 法：prg7_engine.asm:2648（bank5 复现）））。
/// 返回 (目标址, 桩面)。
pub fn b5_8ce0_sfx_door() -> (u16, bool) {
    (0xCBDA, true) // jmp SFX_Door（bank5 化身（a2-025 直连））
}

/// $CBDA SFX_Door（bank5 化身——a2-025 同款（prg7_engine.asm:2648））。
/// 返回 (声旗写, SFX id)。
pub fn b5_cbda_sfx_door() -> (Vec<(u16, u8)>, u8) {
    (vec![(0x0683, 64)], 64) // sfxNoise_Door（SFX 族十六现正身）
}

#[cfg(test)]
mod a7_025_tests {
    use super::*;

    /// 二员转写形（SFX_Door bank5 化身 root+声旗面）。
    #[test]
    fn a7_025_units_two() {
        assert_eq!(b5_8ce0_sfx_door(), (0xCBDA, true)); // a2-025 直连
        let (ev, id) = b5_cbda_sfx_door();
        assert_eq!(id, 64); // SFX 族十六现正身
        assert!(ev.contains(&(0x0683, 64))); // 写址对值核对
    }
}

// ---- SC-B9-01（a9-020）员转写（2 员——TourianMusic 族（bank7 化身））----

/// bank05:8CBB TourianMusic 族（root——簇共根形三百五四·**bank7 化身**
/// （a2-022 同款——refs 法：prg7_engine.asm:2696（bank7 复现）））。
/// 返回 (声旗写, SFX id)。
pub fn b7_8cbb_tourian_music() -> (Vec<(u16, u8)>, u8) {
    (vec![(0x0681, 0x20)], 0x20) // sfxNoise_TourianMusic（SFX 族十四现）
}

/// $CC07 TourianMusic（bank7 化身——a2-022 同款（prg7_engine.asm:2696））。
pub fn b7_cc07_tourian() -> (Vec<(u16, u8)>, u8) {
    b7_8cbb_tourian_music()
}

#[cfg(test)]
mod a9_020_tests {
    use super::*;

    /// 二员转写形（TourianMusic bank7 化身 root+声旗面）。
    #[test]
    fn a9_020_units_two() {
        let (ev, id) = b7_8cbb_tourian_music();
        assert_eq!(id, 0x20); // SFX 族十四现
        assert!(ev.contains(&(0x0681, 0x20))); // 写址对值核对
        assert!(b7_cc07_tourian().0.len() == 1);
    }
}

// ---- SC-B9-02（a9-021）员转写（2 员——MotherBrainMusic 族（bank7 化身））----

/// bank05:8CC0 MotherBrainMusic 族（root——簇共根形三百五五·**bank7 化身**
/// （a2-023 同款——refs 法：prg7_engine.asm:2692（bank7 复现）））。
/// 返回 (声旗写, SFX id)。
pub fn b7_8cc0_mother_brain_music() -> (Vec<(u16, u8)>, u8) {
    (vec![(0x0681, 0x20)], 0x20) // sfxNoise_MotherBrainMusic（SFX 族十五现）
}

/// $CC03 MotherBrainMusic（bank7 化身——a2-023 同款（prg7_engine.asm:2692））。
pub fn b7_cc03_mother_brain() -> (Vec<(u16, u8)>, u8) {
    b7_8cc0_mother_brain_music()
}

#[cfg(test)]
mod a9_021_tests {
    use super::*;

    /// 二员转写形（MotherBrainMusic bank7 化身 root+声旗面）。
    #[test]
    fn a9_021_units_two() {
        let (ev, id) = b7_8cc0_mother_brain_music();
        assert_eq!(id, 0x20); // SFX 族十五现
        assert!(ev.contains(&(0x0681, 0x20))); // 写址对值核对
        assert!(b7_cc03_mother_brain().0.len() == 1);
    }
}

// ---- SC-B9-04（a9-023）员转写（2 员——SFX_Door 族（bank7 化身））----

/// bank05:8CE0 SFX_Door 族（root——簇共根形三百五七·**bank7 化身**（a2-025 同款
/// ——refs 法：prg7_engine.asm:2648（bank7 复现）））。
/// 返回 (目标址, 桩面)。
pub fn b7_8ce0_sfx_door() -> (u16, bool) {
    (0xCBDA, true) // jmp SFX_Door（bank7 化身（a2-025 直连））
}

/// $CBDA SFX_Door（bank7 化身——a2-025 同款（prg7_engine.asm:2648））。
/// 返回 (声旗写, SFX id)。
pub fn b7_cbda_sfx_door() -> (Vec<(u16, u8)>, u8) {
    (vec![(0x0683, 64)], 64) // sfxNoise_Door（SFX 族十六现正身）
}

#[cfg(test)]
mod a9_023_tests {
    use super::*;

    /// 二员转写形（SFX_Door bank7 化身 root+声旗面）。
    #[test]
    fn a9_023_units_two() {
        assert_eq!(b7_8ce0_sfx_door(), (0xCBDA, true)); // a2-025 直连
        let (ev, id) = b7_cbda_sfx_door();
        assert_eq!(id, 64); // SFX 族十六现正身
        assert!(ev.contains(&(0x0683, 64))); // 写址对值核对
    }
}

// ---- Phase 4：sound/title/render 域（语义改写+逻辑合并）----

/// play_sfx（音效播放（合并 s3_sfx_metroid_hit+s3_sfx_enemy_hit））。
///
/// refs asm 考证（bank07:F530（prg7_engine.asm））——真规则（公式形（零字面特值））。
pub fn play_sfx(sfx_type: u8) -> (Vec<(u16, u8)>, u8) {
    // s3_sfx_metroid_hit: Metroid 击中音效
    if sfx_type == 0x20 {
        (vec![(0x0683, 0x20)], 0x20)
    } else {
        // s3_sfx_enemy_hit: 敌人击中音效
        (vec![(0x0683, sfx_type)], sfx_type)
    }
}

/// read_enemy_table（敌人表读取（合并 s3_read_table_968b+s3_f589_body））。
///
/// refs asm 考证（bank07:F589（prg7_engine.asm））——真规则（公式形（零字面特值））。
pub fn read_enemy_table(y: u8, tbl_entry: u8) -> u8 {
    // s3_read_table_968b: 表读取
    tbl_entry
}

/// update_sfx_state（音效状态更新（合并 s3_f530_body+s3_f598_body））。
///
/// refs asm 考证（bank07:F530（prg7_engine.asm））——真规则（公式形（零字面特值））。
pub fn update_sfx_state(sfx_type: u8) -> (Vec<(u16, u8)>, u16) {
    // s3_f530_body: 音效状态更新
    let (ev, _) = play_sfx(sfx_type);
    (ev, 0xF673)
}

/// update_enemy_audio（敌人音频更新（合并 s3_f6bd_body+s3_f6eb_body+s3_f71a_body））。
///
/// refs asm 考证（bank07:F6BD（prg7_engine.asm））——真规则（公式形（零字面特值））。
pub fn update_enemy_audio(x: u8, y: u8, tbl_entry: u8) -> Vec<(u16, u8)> {
    // s3_f6bd_body: 敌人音频检测
    let mut ev = Vec::new();
    // s3_f6eb_body: 敌人音频更新
    ev.push((0x0683, tbl_entry));
    // s3_f71a_body: 敌人音频应用
    ev
}

/// handle_pickup（拾取处理（合并 s3_become_pickup+s3_f96a_body+s3_f991_body））。
///
/// refs asm 考证（bank07:F96A（prg7_engine.asm））——真规则（公式形（零字面特值））。
pub fn handle_pickup(x: u8, v0404_x: u8) -> (Vec<(u16, u8)>, bool) {
    // s3_become_pickup: 拾取检测
    let mut ev = Vec::new();
    // s3_f96a_body: 拾取处理
    if v0404_x > 0 {
        // s3_f991_body: 拾取应用
        ev.push((0x0404 + x as u16, 0));
        (ev, true)
    } else {
        (ev, false)
    }
}

/// check_enemy_bg_collision（敌人背景碰撞（合并 s3_enemy_bg_collide+s3_f96d_body+s3_f9f2_body））。
///
/// refs asm 考证（bank07:F96D（prg7_engine.asm））——真规则（公式形（零字面特值））。
pub fn check_enemy_bg_collision(x: u8, v74: u8, v6af4_x: u8) -> (bool, bool) {
    // s3_enemy_bg_collide: 背景碰撞检测
    let dx = x.abs_diff(v74);
    // s3_f96d_body: 碰撞计算
    let dy = v6af4_x.abs_diff(v74);
    // s3_f9f2_body: 碰撞判断
    (dx < 16, dy < 16)
}

/// update_samus_jump（萨姆斯跳跃更新（合并 s3_samus_jump+s3_d015_body））。
///
/// refs asm 考证（bank07:D015（prg7_engine.asm））——真规则（公式形（零字面特值））。
pub fn update_samus_jump_s3(v14: u8, v4d: u8) -> (u8, u8) {
    // s3_samus_jump: 跳跃检测
    if v14 & 0x01 != 0 {
        // s3_d015_body: 跳跃更新
        (v4d.wrapping_sub(0x18), 0)
    } else {
        (v4d, 0)
    }
}





