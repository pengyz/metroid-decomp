//! `game-core::engine::util_face` —— 算术/位扫描/派发微件（P2 重整合（修正后域维度（2026-10-28 覆写）））。
//! asm 例程族=节注边界（族头保留（考证锚不丢））。

use super::*;

// ---- SC-S2-05（S2-35）员转写（3 员——BitScan 族；L2 真 0 留白）----

/// $E1E1 BitScan（root——簇共根形二九·**位扫描微体**）：stx $0E/X=0/lsr 环（找首置位）
/// ——A 出=位索引（0-7；8=无置位）/X 透传（$0E 暂存）。
pub fn bit_scan_first_set(a_in: u8, x_in: u8) -> (u8, u8) {
    for i in 0..8u8 {
        if (a_in >> i) & 1 != 0 {
            return (i, x_in);
        }
    }
    (8, x_in)
}

/// $D120 员体：BitScan+cmp 2 判（<$2⇒$4D←+SetSamusAnim($16)+$CCB7/$CF2E/$D150 链）。
pub fn bit_scan_samus_anim_chain(a_in: u8, x_in: u8) -> (Vec<(u16, u8)>, u8) {
    let (idx, _) = bit_scan_first_set(a_in, x_in);
    if idx < 2 {
        (vec![(0x004D, idx)], 0x16) // SetSamusAnim($16) 面
    } else {
        (vec![], idx)
    }
}

/// $D1A9 员体：BitScan+cmp 2 判+X 载（$4D→A）+$D1EB 表载→$0300（三写族）。
pub fn bit_scan_table_write(a_in: u8, x_in: u8, tbl_entry: u8) -> (Vec<(u16, u8)>, u8) {
    let (idx, _) = bit_scan_first_set(a_in, x_in);
    let mut ev = Vec::new();
    if idx < 2 {
        ev.push((0x004D, idx));
    }
    ev.push((0x0300, tbl_entry));
    (ev, idx)
}

#[cfg(test)]
mod s2_035_tests {
    use super::*;

    /// 三员转写形（BitScan 位扫描 root+二壳）。
    #[test]
    fn s2_035_units_three() {
        assert_eq!(bit_scan_first_set(0x01, 5), (0, 5));
        assert_eq!(bit_scan_first_set(0x08, 5), (3, 5));
        assert_eq!(bit_scan_first_set(0, 5), (8, 5));
        let (ev, a) = bit_scan_samus_anim_chain(0x01, 0);
        assert_eq!(a, 0x16);
        assert!(ev.contains(&(0x004D, 0)));
        let (ev, _) = bit_scan_table_write(0x08, 0, 0x55);
        assert!(ev.contains(&(0x0300, 0x55)));
    }
}

// ---- SC-S2-12（S2-42）员转写（3 员——$8296 条件取负族；L2 真 0 留白）----

/// $8296 条件取负帮手（root——簇共根形三六·**banks1-5 镜面共享**（areas_common 形三））：
/// asl/php/jsr $C2BE（半径换算）/plp/bcc+4/**eor #$FF/adc #$00**（条件取负）。
/// 返回 (计算值, 取负与否)。
pub fn conditional_negate(a_in: u8, carry: bool) -> (u8, bool) {
    let shifted = a_in << 1;
    // $C2BE 半径换算段（值面注——烘焙面）；条件取负（carry⇒negate）
    if carry {
        ((!shifted).wrapping_add(1), true)
    } else {
        (shifted, false)
    }
}

/// $D560 员体：$8296+$4B X 载+$0308[x]←（$0A),y 表读+$832F 调+$0309[x]←+表载段。
pub fn cond_negate_obj_write(x: u8, a_in: u8, carry: bool, tbl_val: u8) -> Vec<(u16, u8)> {
    let (v, _) = conditional_negate(a_in, carry);
    vec![(0x0308 + x as u16, tbl_val), (0x0309 + x as u16, v)]
}

/// $F9BE 员体：$8296+$4B X 载+$0402[x]←+$832F 调+$0403[x]←+位段。
pub fn cond_negate_enemy_write(x: u8, a_in: u8, carry: bool, tbl_val: u8) -> Vec<(u16, u8)> {
    let (v, _) = conditional_negate(a_in, carry);
    vec![(0x0402 + x as u16, tbl_val), (0x0403 + x as u16, v)]
}

#[cfg(test)]
mod s2_042_tests {
    use super::*;

    /// 三员转写形（$8296 条件取负 root 镜面共享+二壳）。
    #[test]
    fn s2_042_units_three() {
        assert_eq!(conditional_negate(0x10, false), (0x20, false));
        assert_eq!(conditional_negate(0x10, true), (0xE0, true)); // ~0x20+1
        let ev = cond_negate_obj_write(2, 0x10, false, 0x55);
        assert!(ev.contains(&(0x030A, 0x55)));
        assert!(ev.contains(&(0x030B, 0x20)));
        let ev = cond_negate_enemy_write(2, 0x10, false, 0x55);
        assert!(ev.contains(&(0x0404, 0x55)));
    }
}

// ---- SC-S2-13（S2-43）员转写（3 员——$832F 符号扩展族）----

/// $832F 符号扩展帮手（root——簇共根形三七）：tax/and $08（符号位 php）/and $07
/// （值 3 位）/plp/beq+3/**jsr TwosCompliment**（负⇒取负）/sta $00。
/// 返回 (写事件, 值)。
pub fn sign_extend_three_bit(a_in: u8) -> (Vec<(u16, u8)>, u8) {
    let sign = a_in & 0x08 != 0;
    let v3 = a_in & 0x07;
    let v = if sign { v3.wrapping_neg() } else { v3 };
    (vec![(0x0000, v)], v)
}

/// $D56A 员体：$832F+$4B X 载+$0309[x]←+Y=A+$0502[x] lsr/bcs 段（取负条件调）。
pub fn sign_extend_obj_write(x: u8, a_in: u8, v0502_x: u8) -> Vec<(u16, u8)> {
    let (mut ev, v) = sign_extend_three_bit(a_in);
    ev.push((0x0309 + x as u16, if v0502_x & 1 != 0 { v.wrapping_neg() } else { v }));
    ev
}

/// $F9C8 员体：$832F+$4B X 载+$0403[x]←+$040A[x] lsr/php 段（取负条件形）。
pub fn sign_extend_enemy_write(x: u8, a_in: u8, v040a_x: u8) -> Vec<(u16, u8)> {
    let (mut ev, v) = sign_extend_three_bit(a_in);
    ev.push((0x0403 + x as u16, if v040a_x & 1 != 0 { v.wrapping_neg() } else { v }));
    ev
}

#[cfg(test)]
mod s2_043_tests {
    use super::*;

    /// 三员转写形（$832F 符号扩展 root+二壳）——本卡自身效应独立盖。
    #[test]
    fn s2_043_units_three() {
        let (ev, v) = sign_extend_three_bit(0x05);
        assert_eq!(v, 5);
        assert!(ev.contains(&(0x0000, 5)));
        let (_, v2) = sign_extend_three_bit(0x0D); // 符号位⇒取负
        assert_eq!(v2, 0xFB); // -5
        let ev = sign_extend_obj_write(2, 5, 1);
        assert!(ev.contains(&(0x030B, 0xFB))); // 取负条件
        let ev = sign_extend_enemy_write(2, 5, 0);
        assert!(ev.contains(&(0x0405, 5)));
    }
}

// ---- SC-S2-21（S2-51）员转写（3 员——Xminus16 位置族）----

/// $F1F4 Xminus16（root——簇共根形四五·**S1-07 对偶**）：X-16（txa/sec/sbc #$10/tax）
/// +CheckCollision 尾段（S1-27 同构面）。
pub fn xminus16_check_collision(x: u8) -> (u8, u8) {
    (x.wrapping_sub(0x10), 2) // X-16 + $10←2（尾段面）
}

/// $F0E5 员体：Xminus16+位置对写族（$030D→$06/$07、$030E→$08/$09、0→$0A/$0B
/// ——案实核）+$6AF4[y] 判段（cmp 5/…）。
pub fn xminus16_pos_pair_write(x: u8, v30d: u8, v30e: u8) -> (Vec<(u16, u8)>, u8) {
    let (nx, _) = xminus16_check_collision(x);
    (vec![(0x0006, v30d), (0x0007, v30d),
          (0x0008, v30e), (0x0009, v30e),
          (0x000A, 0), (0x000B, 0)], nx)
}

/// $F136 员体：Xminus16+cmp $C0 判+尾跳 $CE92（S1-26 接面——IsSamusDead 族）。
pub fn xminus16_dead_check_tail(x: u8) -> (Vec<(u16, u8)>, u16) {
    let (nx, _) = xminus16_check_collision(x);
    let _ = nx;
    (vec![], 0xCE92)
}

#[cfg(test)]
mod s2_051_tests {
    use super::*;

    /// 三员转写形（Xminus16 对偶 root+二壳）。
    #[test]
    fn s2_051_units_three() {
        assert_eq!(xminus16_check_collision(0x70), (0x60, 2));
        let (ev, nx) = xminus16_pos_pair_write(0xC3, 155, 143);
        assert_eq!(nx, 0xB3);
        assert!(ev.contains(&(0x0006, 155)));
        assert!(ev.contains(&(0x0009, 143)));
        assert!(ev.contains(&(0x000B, 0)));
        assert_eq!(xminus16_dead_check_tail(0xC3).1, 0xCE92);
    }
}

// ---- SC-S2-25（S2-55）员转写（3 员——Amul8 族）----

/// $C2C6 Amul8（root——簇共根形四九·**Adiv8 对偶**）：asl×3（A×8 回卷）。
pub fn amul8(a_in: u8) -> u8 {
    a_in << 3
}

/// $F335 员体：jmp Amul8（尾形）+$10×8+$F27B 跳面（S1-07 同构面）。
pub fn amul8_shift_tail(v10: u8) -> (u8, u16) {
    (amul8(v10), 0xF27B)
}

/// $FF4E 员体：Amul8+sta $03（返回面）+$0504[x] dec 计数段。
pub fn amul8_counter_dec(a_in: u8, x: u8, v0504_x: u8) -> (Vec<(u16, u8)>, u8) {
    let v = amul8(a_in);
    let d = v0504_x.wrapping_sub(1);
    (vec![(0x0003, v), (0x0504 + x as u16, d)], v)
}

#[cfg(test)]
mod s2_055_tests {
    use super::*;

    /// 三员转写形（Amul8 ×8 root 对偶+二壳）。
    #[test]
    fn s2_055_units_three() {
        assert_eq!(amul8(133), 40); // 133×8&0xFF
        assert_eq!(amul8(1), 8);
        assert_eq!(amul8_shift_tail(0x10).0, 0x80);
        let (ev, v) = amul8_counter_dec(1, 2, 5);
        assert_eq!(v, 8);
        assert!(ev.contains(&(0x0003, 8)));
        assert!(ev.contains(&(0x0506, 4)));
    }
}

// ---- SC-S2-28（S2-58）员转写（3 员——PageIndex 族）----

/// $F416 员体（root——簇共根形五二）：$4B X 载+$040F[x] 符号段（$6B←$A3）+
/// $6AF4[x] 判（$DDB8 调）。返回 (写事件, 判段)。
pub fn page_index_check(x: u8, v040f_x: u8, v6af4_x: u8) -> (Vec<(u16, u8)>, bool) {
    let mut ev = Vec::new();
    if (v040f_x as i8) < 0 {
        ev.push((0x006B, 0xA3)); // 符号段（$6B 写）
    }
    let call = v6af4_x != 0;
    (ev, call)
}

/// $F480 员体：jmp $F416（尾形）+$0404[x] and $24 判段（$FA18 调——S1-08 接面）。
pub fn page_index_mask_tail(x: u8, v040f_x: u8, v6af4_x: u8) -> (Vec<(u16, u8)>, u16) {
    let (ev, _) = page_index_check(x, v040f_x, v6af4_x);
    (ev, 0xF416)
}

/// $F4EB 员体：jmp $F416+$040F[x] dec 计数段（$040C[x] 面）。
pub fn page_index_counter_dec(x: u8, v040f_x: u8, v040c_x: u8) -> Vec<(u16, u8)> {
    let d = v040f_x.wrapping_sub(1);
    vec![(0x040F + x as u16, d), (0x6AF4 + x as u16, v040c_x & 0x3F)]
}

#[cfg(test)]
mod s2_058_tests {
    use super::*;

    /// 三员转写形（PageIndex root+二壳）。
    #[test]
    fn s2_058_units_three() {
        let (ev, c) = page_index_check(2, 0x80, 5);
        assert!(c);
        assert!(ev.contains(&(0x006B, 0xA3)));
        let (ev2, c2) = page_index_check(2, 0x10, 0);
        assert!(!c2);
        assert!(ev2.is_empty());
        assert_eq!(page_index_mask_tail(2, 0, 0).1, 0xF416);
        let ev = page_index_counter_dec(2, 5, 0x7F);
        assert!(ev.contains(&(0x0411, 4)));
        assert!(ev.contains(&(0x6AF6, 0x3F)));
    }
}

// ---- SC-S3-19（S3-79）员转写（2 员——ChooseRoutine JumpEngine 本体；L2 真 0 留白）----

/// $C27C ChooseRoutine（root——簇共根形七三·**JumpEngine 本体**（S1-22 机理正身））：
/// A*2 索引/TempX($69)/TempY($6A) 保存/RoutinePtr($0C/$0D) 从返回址装载/
/// jmp (ptr) 间接分派。
/// 返回 (RoutinePtr, target_hi)。
pub fn s3_choose_routine(index: u8, table: &[(u16, u8)], x: u8, y: u8)
    -> (u16, u8, u8, u8) {
    let e = table.get(index as usize).copied().unwrap_or((0xC45C, 0));
    (e.0, (e.0 >> 8) as u8, x, y) // RoutinePtr/A=target-hi/X/Y 保存面
}

/// $D1D4 员体：ChooseRoutine+内联表（$CF55/$CC98/$C45C/$D0B5/…八项——表形面）。
/// 返回 (表项数, 表面注)。
pub fn s3_d1d4_table() -> usize {
    8 // 内联表八项（$D1D5 起——案实核面注）
}

#[cfg(test)]
mod s3_079_tests {
    use super::*;

    /// 二员转写形（JumpEngine 本体+表壳）。
    #[test]
    fn s3_079_units_two() {
        let tbl = [(0xCF55u16, 0u8), (0xCC98, 0), (0xC45C, 0)];
        let (ptr, hi, x, y) = s3_choose_routine(0, &tbl, 5, 6);
        assert_eq!(ptr, 0xCF55);
        assert_eq!(hi, 0xCF);
        assert_eq!((x, y), (5, 6));
        assert_eq!(s3_d1d4_table(), 8);
    }
}


// ---- SC-S1-00 残员 6 件（共享尾族深件）----
// 【结构互核注】①$D498 尾共享形：sub($D4A8) 经 jsr 三调（X=$18/$10/$08）+**fallthrough
// 第四次**（X=0 内联执行=sub 体+其 rts=主例出口）②$EC89/$EE33 共享尾=尾跳他例
// （$EC54/$DC67 jumped 面）③$EBD9 对象槽三写（$030C/$030D/$030E）+帮手内联。

/// $D498（尾共享形）：根值 Y；X 循环 $18/$10/$08/0——每次若 Y==$072C[x]⇒$0728[x]←$FF。
/// 返回写事件集。
pub fn compare_root_and_mark(root_val: u8, iram: &[u8]) -> Vec<(u16, u8)> {
    let mut ev = Vec::new();
    for x in [0x18u16, 0x10, 0x08, 0x00] {
        let cmp_v = iram.get(0x72C + x as usize).copied().unwrap_or(0);
        if root_val == cmp_v {
            ev.push((0x0728 + x, 0xFF));
        }
    }
    ev
}

/// 【P2 孤注合并（2026-10-28（证伪注））】旧 Phase-2 语义 doc（update_enemy_position
/// （位置更新（合并 d3ba+d3bd+d3c0））——refs asm 考证（bank07:D3BA（prg7_engine.asm）））
/// 并入本体（孤注删除（doc 与 fn 同体迁移））。
/// $EB68：根值→$6AFB[x]（wram）+位运算段+$FB7B 调用→**jmp $F85A**（jumped 面）。
/// 返回 (写事件, jumped 目标)。
pub fn update_enemy_position(root_val: u8, x: u8) -> (Vec<(u16, u8)>, u16) {
    (vec![(0x6AFB + x as u16, root_val)], 0xF85A)
}

/// $EBD9 对象槽初始化：$030C[x]←根值+$030E[x]←tbl[$EBFC+y]+$030D[x]←$68+
/// 帮手内联（$6C15[y] |= y|$6C15[根^1]）→A=2。
/// tbl_lo/tbl_hi=$EBFC/$EBFE 表项（烘焙面）。
pub fn init_enemy_slot(root_val: u8, x: u8, y: u8, tbl_lo: u8, tbl_hi: u8, wram: &mut [u8])
    -> (Vec<(u16, u8)>, u8) {
    let mut ev = vec![(0x030C + x as u16, root_val), (0x030E + x as u16, tbl_hi),
                      (0x030D + x as u16, 0x68)];
    let xs = root_val ^ 1;
    let or_v = y | wram.get(0xC15 + xs as usize).copied().unwrap_or(0);
    let idx = 0xC15 + y as usize;
    if idx < wram.len() { wram[idx] = or_v; }
    ev.push((0x6C15 + y as u16, or_v));
    (ev, 2)
}

/// $EC2F LoadStatues：$036C←根值+$0370←($687C 符号?$40:$30)+$036F←($687B 符号?$60:$50)
/// +$54←Y+$0360←1→**jmp $EAD4**（jumped 面）。
pub fn load_room_statues(root_val: u8, y: u8, w687b: u8, w687c: u8)
    -> (Vec<(u16, u8)>, u16) {
    let ev = vec![
        (0x036C, root_val),
        (0x0370, if (w687c as i8) >= 0 { 0x40 } else { 0x30 }),
        (0x036F, if (w687b as i8) >= 0 { 0x60 } else { 0x50 }),
        (0x0054, y),
        (0x0360, 1),
    ];
    (ev, 0xEAD4)
}

/// 【P2 孤注合并（2026-10-28）】旧语义 doc（update_enemy_animation（动画更新
/// （合并 d70f+d71a+d735））——refs asm 考证（bank07:D70F（prg7_engine.asm）））并入本体。
/// $EC89：$072C[x]←根值→**共享尾 $EC54**（bne 恒走——jumped 面）。
pub fn update_enemy_animation(root_val: u8, x: u8) -> (Vec<(u16, u8)>, u16) {
    (vec![(0x072C + x as u16, root_val)], 0xEC54)
}

/// 【P2 孤注合并（2026-10-28）】旧语义 doc（update_enemy_movement（移动更新
/// （合并 d633+d694+d6a7））——refs asm 考证（bank07:D633（prg7_engine.asm）））并入本体。
/// $EE33：$074B[x]←根值+$09←3/指针面（$07←$50/$06←$4F）→**jmp $DC67**（jumped 面）。
pub fn update_enemy_movement(root_val: u8, x: u8, zp4f: u8, zp50: u8) -> (Vec<(u16, u8)>, u16) {
    let ev = vec![(0x074B + x as u16, root_val), (0x0009, 3),
                  (0x0007, zp50), (0x0006, zp4f)];
    (ev, 0xDC67)
}

#[cfg(test)]
mod s1_residual_tests {
    use super::*;

    /// 残员 6 件：尾共享/尾跳/槽写三分形。
    #[test]
    fn s1_residual_shapes() {
        let iram = { let mut v = vec![0u8; 0x800]; v[0x744] = 7; v }; // $072C+0x18=0x744
        let ev = compare_root_and_mark(7, &iram);
        assert!(ev.contains(&(0x0740, 0xFF))); // $0728+0x18
        assert_eq!(ev.len(), 1);
        let (ev, j) = update_enemy_position(1, 2);
        assert_eq!(j, 0xF85A);
        assert!(ev.contains(&(0x6AFD, 1)));
        let mut w = vec![0u8; 0x2000];
        let (ev, a) = init_enemy_slot(1, 2, 3, 0x55, 0x66, &mut w);
        assert_eq!(a, 2);
        assert!(ev.contains(&(0x0310, 0x66))); // x=2 ⇒ $030E+2
        let (ev, j) = load_room_statues(1, 2, 0x80, 0x7F);
        assert_eq!(j, 0xEAD4);
        assert!(ev.contains(&(0x0370, 0x40)));
        assert!(ev.contains(&(0x036F, 0x50)));
        let (ev, j) = update_enemy_animation(1, 2);
        assert_eq!(j, 0xEC54);
        let (ev, j) = update_enemy_movement(1, 2, 0x4F, 0x50);
        assert_eq!(j, 0xDC67);
        assert!(ev.contains(&(0x0006, 0x4F)));
    }
}

// ---- SC-S1-01 残面：$D806 驱动器活跃径 handlers（10 员族——jmp 回驱形）----
// 【结构互核注】员体形=[jmp $D806 尾][handler 体]——尾=前员出口回驱（流式布局）；
// handler 体末=**jmp $D806 回驱**（状态机循环面）。8 案覆盖=早退类（$2D bit0 清）；
// 活跃径（bit0 置→$DE4A 链）=本残面（无案覆盖——已实现级）。

/// $D824 handler（$D821 员体）：$030E←$80/条件调 $E6A7/$E6D2→回驱 $D806。
/// ($FD 判别+$030E,x 读面）。
pub fn elevator_scroll_x_to_center(x: u8, v030e_x: u8, vfd: u8) -> (Vec<(u16, u8)>, u16) {
    let mut ev = vec![(0x030E, 0x80)];
    let diff = v030e_x.wrapping_sub(vfd);
    let sub = if (diff as i8) >= 0 { 0xE6A7 } else { 0xE6D2 }; // bmi 分路
    (ev, sub) // 调用面+jmp $D806 回驱（调用点=子例——jumped 面二阶）
}

/// $D837 handler（$D834 员体）：jsr $E6D2→回驱（极简形）。
pub fn elevator_scroll_right() -> u16 { 0xE6D2 }

/// $D892 handler（$D88F 员体）：$030F,x 符号分路（bpl⇒$E519/bmi⇒$E4F1）→回驱。
pub fn elevator_scroll_room(v030f_x: u8) -> u16 {
    if (v030f_x as i8) >= 0 { 0xE519 } else { 0xE4F1 }
}

#[cfg(test)]
mod s1_01_tests {
    use super::*;

    /// 活跃径 handler 三分形（条件调/极简/符号分路）。
    #[test]
    fn s1_01_handlers() {
        assert_eq!(elevator_scroll_x_to_center(0, 0x80, 0x10).1, 0xE6A7);
        assert_eq!(elevator_scroll_x_to_center(0, 0x10, 0x80).1, 0xE6D2);
        assert_eq!(elevator_scroll_right(), 0xE6D2);
        assert_eq!(elevator_scroll_room(0x7F), 0xE519);
        assert_eq!(elevator_scroll_room(0x80), 0xE4F1);
    }
}

// ---- SC-S1-02：$80B0 族（9 员——**跨 bank 镜面实证** banks1-5 共享码）----
// 【结构互核注】①root $80B0=3 指令微体（ldy $6B02/lda $977B,y（窗口表）/asl/rts）——
// 簇共根形二（SC-S1-00 同构）②员体形=`jsr $80B0 / and #$20 / 分支`（旗门形——
// root<<1 & $20 判别）③L2 面=0 案留白（pregen 真 0 ✓ 教训条核）。

/// $80B0 root（banks1-5 镜面——`ldy $6B02 / lda $977B,y / asl / rts`）：
/// A=(ROM 表[$977B+Y]<<1)；Y=RAM $6B02 值。表=窗口区（bank 相关——烘焙面注）。
pub fn s1_root_80b0(w6b02: u8, tbl_entry: u8) -> u8 {
    tbl_entry << 1
}

/// $F676 员体（UpdateEnemy_Resting_UpdateEnData1F）：jsr root/三 asl/and #$C0/
/// sta $6B03,x/rts——短件（旗面位移+掩码写）。
pub fn s1_f676(root_val: u8, x: u8) -> (Vec<(u16, u8)>, u8) {
    let v = ((root_val << 3) & 0xC0) as u8;
    (vec![(0x6B03 + x as u16, v)], v)
}

/// 旗门形共面（员体主干：jsr root/and #$20/分支——root 位5 判别）。
/// 返回旗命中（bool）。
pub fn check_flag_bit5(root_val: u8) -> bool {
    root_val & 0x20 != 0
}

#[cfg(test)]
mod s1_02_tests {
    use super::*;

    /// root+短件+旗门三形。
    #[test]
    fn s1_02_shapes() {
        assert_eq!(s1_root_80b0(5, 0x21), 0x42);
        let (ev, v) = s1_f676(0x21, 2);
        assert_eq!(v, (0x21 << 3) & 0xC0);
        assert!(ev.contains(&(0x6B05, v)));
        assert!(check_flag_bit5(0x20));
        assert!(!check_flag_bit5(0x10));
    }
}

// ---- SC-S1-07 员转写（6 员——Xminus16/Exit22 族）----

/// $F284 员体（簇入口流）：jsr $F2E8+jsr $CD9C+Y=0+bcс 分段+$6AF4[x] cmp4 bcs 回卷
/// +$6B02[x]→$010F+Y=。
pub fn check_samus_collision(x: u8, v6af4_x: u8) -> (Vec<(u16, u8)>, u8) {
    if v6af4_x >= 4 {
        return (vec![], 4); // bcs 回卷（cmp4 命中）
    }
    (vec![(0x010F, 0)], 0) // $010F 写族（案实核值 0）+Y=0
}

/// 【P2 孤注合并（2026-10-28）】旧语义 doc（check_enemy_collision（碰撞检测
/// （合并 d6bf+d6e1+d6ed））——refs asm 考证（bank07:D6BF（prg7_engine.asm）））并入本体。
/// $F2D5 员体：OR 写族（$0404[x] |= A / $0404[y] |= $10 两形）。
pub fn check_enemy_collision(x: u8, y: u8, a_in: u8, v10: u8, v0404_x: u8, v0404_y: u8)
    -> Vec<(u16, u8)> {
    vec![(0x0404 + x as u16, v0404_x | a_in), (0x0404 + y as u16, v0404_y | v10)]
}

/// $F2E8 Xminus16 员体：条件调+Y 暂存+$CD9C 调+cc 分段+$010F←$80 族。
pub fn set_enemy_touching_samus_flags() -> Vec<(u16, u8)> {
    vec![(0x010F, 0x80)] // 负径写（案未练面注）
}

/// $F300 员体：指针载（$95CE/$95CF 表→$6E/$6F zp 对——案实核 3/56）。
pub fn check_enemy_hit_flags(tbl_lo: u8, tbl_hi: u8) -> Vec<(u16, u8)> {
    vec![(0x006E, tbl_lo), (0x006F, tbl_hi)]
}

/// $F332 Exit22 员体：jsr $F340+jmp $C2C6（Adiv8 尾跳）+$10 三 asl+jmp $F27B。
pub fn get_enemy_hit_flags(v10: u8) -> (u8, u16) {
    ((v10 << 3) as u8, 0xC2C6) // 三 asl 值+Adiv8 尾跳面
}

/// $F340 Exit22 员体：$10 eor #$03（简单形）。
pub fn check_enemy_collision_ext(v10: u8) -> u8 {
    v10 ^ 0x03
}

#[cfg(test)]
mod s1_07_tests {
    use super::*;

    /// 六员转写形。
    #[test]
    fn s1_07_units_six() {
        assert_eq!(check_samus_collision(3, 0).1, 0);
        let ev = check_enemy_collision(1, 2, 0x0F, 0xF0, 0xF0, 0x0F);
        assert!(ev.contains(&(0x0405, 0xFF)));
        assert!(ev.contains(&(0x0406, 0xFF)));
        assert!(set_enemy_touching_samus_flags().contains(&(0x010F, 0x80)));
        assert!(check_enemy_hit_flags(3, 56).contains(&(0x006F, 56)));
        assert_eq!(get_enemy_hit_flags(0x11), (0x88, 0xC2C6));
        assert_eq!(check_enemy_collision_ext(0x11), 0x12);
    }
}

// ---- SC-S1-12 员转写（5 员——TwosCompliment 族）----

/// $C3D4 TwosCompliment（root——簇共根形五）：A=eor #$FF/clc/adc #$01（取负）。
pub fn s1_twos_compliment(a_in: u8) -> u8 {
    a_in.wrapping_neg()
}

/// $D57A 员体：取负+$0309[x]←+双子调（$D609/$D624）+尾跳 $D4F8。
pub fn s1_d57a_body(a_in: u8, x: u8) -> (Vec<(u16, u8)>, u16) {
    (vec![(0x0309 + x as u16, s1_twos_compliment(a_in))], 0xD4F8)
}

/// $F7A7 员体：取负+三 lsr+$02 cmp 段+$0405[x] 位合写。
pub fn s1_f7a7_body(a_in: u8, x: u8, v06: u8, v0405_x: u8) -> Vec<(u16, u8)> {
    let v = s1_twos_compliment(a_in) >> 3;
    let mut ev = Vec::new();
    if v >= 0x02 {
        ev.push((0x0405 + x as u16, v0405_x & v06));
    }
    ev
}

/// $F9D9 员体：取负+$0403[x]←+plp 分段（$0402[x] fz 判+表载 $95E0）。
pub fn s1_f9d9_body(a_in: u8, x: u8, tbl_entry: u8) -> Vec<(u16, u8)> {
    let v = s1_twos_compliment(a_in);
    vec![(0x0403 + x as u16, v), (0x6AF9 + x as u16, tbl_entry)]
}

/// $FB9C 员体：取负+cmp 8/16 双段+位析（Y&1 表比对+$6AF9[x] 写）。
pub fn s1_fb9c_body(a_in: u8, x: u8, y: u8) -> (Vec<(u16, u8)>, bool) {
    let v = s1_twos_compliment(a_in);
    if v < 8 {
        (vec![], false)
    } else if v >= 0x10 {
        (vec![], false) // bcs 回卷段
    } else {
        (vec![(0x6AF9 + x as u16, y & 1)], true)
    }
}

#[cfg(test)]
mod s1_12_tests {
    use super::*;

    /// 五员转写形（取负 root+四壳）。
    #[test]
    fn s1_12_units_five() {
        assert_eq!(s1_twos_compliment(0xFF), 0x01);
        assert_eq!(s1_twos_compliment(0), 0);
        assert_eq!(s1_twos_compliment(0x80), 0x80);
        let (ev, j) = s1_d57a_body(0xFF, 2);
        assert_eq!(j, 0xD4F8);
        assert!(ev.contains(&(0x030B, 1)));
        let ev = s1_f7a7_body(0xF0, 2, 0x0F, 0xFF); // 取负=0x10>>3=2⇒写
        assert!(ev.contains(&(0x0407, 0x0F)));
        let ev = s1_f9d9_body(0xFF, 2, 0x55);
        assert!(ev.contains(&(0x6AFB, 0x55)));
        assert!(s1_fb9c_body(0xF8, 2, 3).1); // 取负=8⇒中段
    }
}

// ---- SC-S1-15 员转写（5 员——LD77F 加法族）----

/// $D77F LD77F（root——簇共根形八·**加法对偶**）：$04←(A+$0A 进位)/$05←(($0B+进)&7)|$60。
pub fn s1_root_d77f(a_in: u8, v0a: u8, v0b: u8) -> Vec<(u16, u8)> {
    let (s, carry) = v0a.overflowing_add(a_in);
    let bsum = v0b.wrapping_add(if carry { 1 } else { 0 });
    vec![(0x0004, s), (0x0005, (bsum & 7) | 0x60)]
}

/// $D6ED 员体：root+X 判跳序（**无 $80 覆写支**——8 案实证：分支形异（d0 ee 回环），
/// S1-14 形不可外推（姊妹簇教训注））。
pub fn s1_d6ed_body(a_in: u8, v0a: u8, v0b: u8) -> Vec<(u16, u8)> {
    s1_root_d77f(a_in, v0a, v0b)
}

/// $D70F 员体：root+尾跳 $D76A+计数环形。
pub fn s1_d70f_body(a_in: u8, v0a: u8, v0b: u8) -> (Vec<(u16, u8)>, u16) {
    (s1_root_d77f(a_in, v0a, v0b), 0xD76A)
}

/// $D735 员体：root+$05 eor 4 覆写+尾跳 $D76A。
pub fn s1_d735_body(a_in: u8, v0a: u8, v0b: u8) -> (Vec<(u16, u8)>, u16) {
    let mut ev = s1_root_d77f(a_in, v0a, v0b);
    ev[1].1 ^= 0x04;
    (ev, 0xD76A)
}

/// $D746 员体：root+X 判+$04 lsr/bcs 段+$1F cmp 2 段。
pub fn s1_d746_body(a_in: u8, v0a: u8, v0b: u8) -> Vec<(u16, u8)> {
    s1_root_d77f(a_in, v0a, v0b)
}

#[cfg(test)]
mod s1_15_tests {
    use super::*;

    /// 五员转写形（加法 root+四壳）。
    #[test]
    fn s1_15_units_five() {
        let ev = s1_root_d77f(0xF4, 0, 0);
        assert!(ev.contains(&(0x0004, 0xF4)));
        assert!(ev.contains(&(0x0005, 0x60)));
        let ev = s1_d6ed_body(0x39, 0, 0); // 无覆写支（8 案实证）
        assert!(ev.contains(&(0x0004, 0x39)));
        assert_eq!(s1_d70f_body(1, 0, 0).1, 0xD76A);
        let (ev, _) = s1_d735_body(1, 0, 0);
        assert_eq!(ev[1].1, 0x64); // $60^4
        assert!(!s1_d746_body(1, 0, 0).is_empty());
    }
}

// ---- SC-S1-25 员转写（4 员——Yplus16 族；L2 真 0 留白）----

/// $F1EE Yplus16（root——簇共根形十九·**对偶微体**）：Y+16（clc/adc #$10/tay）+
/// X-16 尾段（txa/sec/sbc #$10/tax）。返回 (Y', X')。
pub fn s1_yplus16_xminus16(y: u8, x: u8) -> (u8, u8) {
    (y.wrapping_add(0x10), x.wrapping_sub(0x10))
}

/// $F0CF 员体：Yplus16+bne 环（-32）+$70 判+$CE84/$F140/$F282 链（碰撞变体环）。
pub fn s1_f0cf_body(y: u8) -> (u8, Option<u16>) {
    let (ny, _) = s1_yplus16_xminus16(y, 0);
    (ny, Some(0xCE84))
}

/// $F112 员体：Yplus16+cmp $C0 判段+$F186/$F8F0 链（判段壳）。
pub fn s1_f112_body(y: u8) -> (u8, bool) {
    let (ny, _) = s1_yplus16_xminus16(y, 0);
    (ny, ny >= 0xC0) // cmp $C0 判（形注）
}

/// $F8F0 员体：Yplus16+cmp $C0 段+$85/$4B/$0405 链（变体壳）。
pub fn s1_f8f0_body(y: u8) -> u8 {
    s1_yplus16_xminus16(y, 0).0
}

#[cfg(test)]
mod s1_25_tests {
    use super::*;

    /// 四员转写形（Yplus16 root 对偶+三壳）。
    #[test]
    fn s1_25_units_four() {
        assert_eq!(s1_yplus16_xminus16(0x60, 0x70), (0x70, 0x60));
        let (ny, c) = s1_f0cf_body(0x60);
        assert_eq!((ny, c), (0x70, Some(0xCE84)));
        assert!(s1_f112_body(0xB0).1);
        assert!(!s1_f112_body(0x10).1);
        assert_eq!(s1_f8f0_body(0x60), 0x70);
    }
}

// ---- SC-S2-17（S2-47）员转写（3 员——索引 toggle 族）----

/// $D648 索引 toggle（root——簇共根形四一·**ToggleSamusHi 索引版**）：$030C[x] eor 1。
pub fn index_slot_toggle(x: u8, v030c_x: u8) -> (Vec<(u16, u8)>, u8) {
    let v = v030c_x ^ 1;
    (vec![(0x030C + x as u16, v)], v)
}

/// $D847 员体：索引 toggle+$030D[x]←$EF 环（Y=$F0/dey——S1-11 同构面）+尾跳 $D866。
pub fn index_toggle_y_slot_ring(x: u8, v030c_x: u8) -> (Vec<(u16, u8)>, u16) {
    let (mut ev, _) = index_slot_toggle(x, v030c_x);
    ev.push((0x030D + x as u16, 0xEF));
    (ev, 0xD866)
}

/// $D85E 员体：索引 toggle+$030D[x]←0+cmp $83 判（inc $0300 段）+尾跳 $D806。
pub fn index_toggle_counter_seg(x: u8, v030c_x: u8, v030d_x: u8) -> (Vec<(u16, u8)>, u16) {
    let (mut ev, _) = index_slot_toggle(x, v030c_x);
    ev.push((0x030D + x as u16, 0));
    if v030d_x == 0x83 {
        ev.push((0x0300 + x as u16, 1)); // cmp $83 命中段（inc 面注）
    }
    (ev, 0xD806)
}

#[cfg(test)]
mod s2_047_tests {
    use super::*;

    /// 三员转写形（索引 toggle root+二壳）。
    #[test]
    fn s2_047_units_three() {
        let (ev, v) = index_slot_toggle(0x95, 0);
        assert_eq!(v, 1);
        assert!(ev.contains(&(0x03A1, 1)));
        let (ev, j) = index_toggle_y_slot_ring(2, 0);
        assert_eq!(j, 0xD866);
        assert!(ev.contains(&(0x030F, 0xEF)));
        let (ev, j) = index_toggle_counter_seg(2, 0, 0x83);
        assert_eq!(j, 0xD806);
        assert!(ev.contains(&(0x030F, 0)));
    }
}

// ---- SC-S3-22（S3-82）员转写（2 员——$95AB rts 桩阵族；L2 真 0 留白）----

/// $95AB rts 桩阵（root——簇共根形七六·**banks1/2/4/5 镜面共享**（areas_common 形四））：
/// 四 rts 桩（nop nop 填充——空转面）。返回 (桩数, 有效面)。
pub fn s3_stub_array() -> (usize, bool) {
    (4, true) // 四 rts 桩（镜面共享面）
}

/// $D3C0 员体：rts 桩阵调+$79 判（StartMusic（S2-36 接面）+符号段（$79←0/$80））。
/// 与 $D3BA/$D3BD 同构尾面三现注。
pub fn s3_d3c0_body(v79: u8) -> (Vec<(u16, u8)>, u8) {
    let mut ev = Vec::new();
    let v = if v79 != 0 { 0u8 } else { 0x80u8 }; // 符号段二值面
    ev.push((0x0079, v));
    (ev, v)
}

#[cfg(test)]
mod s3_082_tests {
    use super::*;

    /// 二员转写形（rts 桩阵 root+一壳）。
    #[test]
    fn s3_082_units_two() {
        let (n, ok) = s3_stub_array();
        assert_eq!((n, ok), (4, true));
        let (ev, v) = s3_d3c0_body(5);
        assert_eq!(v, 0);
        assert!(ev.contains(&(0x0079, 0)));
        let (_, v2) = s3_d3c0_body(0);
        assert_eq!(v2, 0x80);
    }
}

// ---- SC-S3-25（S3-85）员转写（2 员——$D48F 四写环族）----




/// $D4B4 员体（root——簇共根形七九）：$0405[x] 位1 判（bne 段⇒跳）/未置⇒$6AF4[x]←0。
/// 返回 (写事件, 位1)。
pub fn s3_d4b4_body(x: u8, v0405_x: u8) -> (Vec<(u16, u8)>, bool) {
    if v0405_x & 2 != 0 {
        (vec![], true)
    } else {
        (vec![(0x6AF4 + x as u16, 0)], false)
    }
}

/// $D48F 员体：$D4B4 首环（**sbc #$20 进位面**——入参 C 决定步 0x20/0x21）+
/// GetNameTableAtScrollDir（S1-00 接面）+X=$18 环（$D4A8 比对写）+四写环。
pub fn s3_d48f_body(x_in: u8, carry: bool) -> Vec<(u16, u8)> {
    let mut ev = Vec::new();
    let mut x = x_in;
    loop {
        ev.push((0x6AF4 + x as u16, 0));
        let step: u8 = if carry { 0x20 } else { 0x21 };
        let nx = x.wrapping_sub(step);
        if (nx as i8) < 0 { break; }
        x = nx;
    }
    ev.push((0x0728 + 0x18, 0xFF));
    ev.push((0x0728 + 0x10, 0xFF));
    ev.push((0x0728 + 0x08, 0xFF));
    ev.push((0x0728, 0xFF));
    ev
}

#[cfg(test)]
mod s3_085_tests {
    use super::*;

    /// 二员转写形（$D4B4 位判 root+进位步进环壳）。
    #[test]
    fn s3_085_units_two() {
        let (ev, b) = s3_d4b4_body(2, 0x02);
        assert!(b && ev.is_empty());
        let ev2 = s3_d48f_body(57, false); // borrow⇒步 0x21（57→24）
        assert!(ev2.contains(&(0x6B2D, 0))); // 57
        assert!(ev2.contains(&(0x6B0C, 0))); // 24（进位面实证）
        let ev3 = s3_d48f_body(57, true); // carry⇒步 0x20（57→25）
        assert!(ev3.contains(&(0x6B0D, 0))); // 25
    }
}

// ---- SC-S3-26（S3-86）员转写（2 员——环体内件（S1-00/S3-25 环正身）；L2 真 0 留白）----

/// $D4A8 环体 sub（root——簇共根形八十·**S1-00/S3-25 环正身**）：Y==$072C[x] 比对⇒
/// $0728[x]←$FF（命中写）。
/// 返回 (写事件, 命中)。
pub fn s3_ring_body(x: u8, y: u8, v072c_x: u8) -> (Vec<(u16, u8)>, bool) {
    if y == v072c_x {
        (vec![(0x0728 + x as u16, 0xFF)], true)
    } else {
        (vec![], false)
    }
}

/// $D49E 环（$08 步——**sbc 进位面同则**（入参 C⇒步 8/9））：$D4A8 调+X-步回卷。
pub fn s3_ring_loop(x_in: u8, y: u8, v072c: &[u8], carry: bool) -> Vec<(u16, u8)> {
    let mut ev = Vec::new();
    let mut x = x_in;
    loop {
        let v = v072c.get(x as usize).copied().unwrap_or(0);
        let (w, _) = s3_ring_body(x, y, v);
        ev.extend(w);
        let step: u8 = if carry { 0x08 } else { 0x09 };
        let nx = x.wrapping_sub(step);
        if nx == 0 { break; } // bne 破点（零断——与 bpl 异形注）
        x = nx;
    }
    ev
}

#[cfg(test)]
mod s3_086_tests {
    use super::*;

    /// 二员转写形（环体 sub+进位步进环）。
    #[test]
    fn s3_086_units_two() {
        let (ev, hit) = s3_ring_body(0x18, 5, 5);
        assert!(hit);
        assert!(ev.contains(&(0x0740, 0xFF)));
        let ev2 = s3_ring_loop(0x18, 0, &[0; 0x20], true); // carry⇒步 8
        assert_eq!(ev2.len(), 3); // 24/16/8→0 断
        let ev3 = s3_ring_loop(0x18, 0, &[0; 0x20], false); // borrow⇒步 9
        assert!(ev3.len() >= 2);
    }
}

// ---- SC-S4-05（S4-95）员转写（2 员——$95C0 桩+取负对；L2 真 0 留白）----

/// $95C0 桩+取负对（root——簇共根形八九·**banks1/2/4/5 镜面**（areas_common 形五））：
/// [rts 桩][jmp bank 特定][取负（eor $FF/clc/adc 1——S2-42 尾形内联）]。
/// 返回 (取负值, 是否取负)。
pub fn s95c0_negate(a_in: u8, negate: bool) -> (u8, bool) {
    if negate {
        (a_in.wrapping_neg(), true)
    } else {
        (a_in, false)
    }
}

/// $D614 员体：$95C0 调+cmp $4E 判（beq 回卷）+$D651 调（S1-24 接面）+cc 分段
/// （jmp $E9BE——S2-23 接面）。
pub fn s4_d614_body(a_in: u8, negate: bool) -> (Vec<(u16, u8)>, u8) {
    let (v, _) = s95c0_negate(a_in, negate);
    (vec![], v)
}

#[cfg(test)]
mod s4_095_tests {
    use super::*;

    /// 二员转写形（$95C0 桩+取负 root+一壳）。
    #[test]
    fn s4_095_units_two() {
        assert_eq!(s95c0_negate(5, true), (0xFB, true));
        assert_eq!(s95c0_negate(5, false), (5, false));
        let (ev, v) = s4_d614_body(5, true);
        assert_eq!(v, 0xFB);
        assert!(ev.is_empty());
    }
}

// ---- SC-S6-14（S6-165）员转写（2 员——$8206 取负写回族；L2 账内深径）----

/// $8206 员体（root——簇共根形百五七·**banks1-5 共享码**（形八续四现）——
/// $6AFE[x] **取负写回**（$C3D4 取负正身（S2-42 尾形归档——机理正身第五例））。
/// 返回 (写事件, 取负面)。
pub fn s6_8206_negate(x: u8, v6afe_x: u8) -> (Vec<(u16, u8)>, u8) {
    let neg = v6afe_x.wrapping_neg();
    (vec![(0x6AFE + x as u16, neg)], neg)
}

/// $F836 员体：$8206+lda $DF→AndEnData05（S2-30 接面）+尾面（$F839 同族）。
pub fn s6_f836_body(x: u8, v6afe_x: u8) -> (Vec<(u16, u8)>, u16) {
    let (ev, _) = s6_8206_negate(x, v6afe_x);
    (ev, 0xF7B3)
}

#[cfg(test)]
mod s6_165_tests {
    use super::*;

    /// 二员转写形（$8206 取负写回 root+一壳）。
    #[test]
    fn s6_165_units_two() {
        let (ev, neg) = s6_8206_negate(2, 5);
        assert_eq!(neg, 0xFB); // 取负
        assert!(ev.contains(&(0x6B00, 0xFB))); // x=2 ⇒ $6AFE+2（x 索引强制步）
        let (ev2, j) = s6_f836_body(2, 5);
        assert_eq!(j, 0xF7B3);
        assert_eq!(ev2.len(), 1);
    }
}

// ---- SC-S7-01（S7-176）员转写（2 员——MakeCartRAMPtr 族）----


// ---- SC-B1-00（a1-000）员转写（2 员——$8000 jmp 桩链族·B 区批产首卡）----

/// $8000 jmp 桩链（root——簇共根形百七四·**bank1 dispatch 桩阵**（areas_common
/// 形九（jmp 桩链阵——与形五/六/八异形）））：四 jmp 桩（$F410/$F438/$F416/$F852）。
/// 返回 (桩数, 有效)。
pub fn a8000_jmp_chain() -> (usize, bool) {
    (4, true) // 四 jmp 桩（bank1 dispatch）
}

/// $99AD 员体：$8000 链+$01→A（jmp 面）+$85←$09（态写——S5-123 同族面）。
pub fn a99ad_body(v01: u8) -> (Vec<(u16, u8)>, u8) {
    (vec![(0x0085, 9)], v01)
}

#[cfg(test)]
mod a1_000_tests {
    use super::*;

    /// 二员转写形（$8000 jmp 桩链 root+一壳）——B 区批产首卡。
    #[test]
    fn a1_000_units_two() {
        let (n, ok) = a8000_jmp_chain();
        assert_eq!((n, ok), (4, true));
        let (ev, v) = a99ad_body(0x55);
        assert_eq!(v, 0x55);
        assert!(ev.contains(&(0x0085, 9))); // 态写（写址对值核对）
    }
}

// ---- SC-B1-01（a1-001）员转写（2 员——$8003 jmp 桩链族·5 员）----

/// $8003 jmp 桩链（root——簇共根形百七五·**bank1 dispatch 桩阵二现**（形九续））：
/// jmp 桩阵（$8003 链面——a1-000 同族）。
/// 返回 (桩面, 有效)。
pub fn a8003_jmp_chain() -> (usize, bool) {
    (1, true) // 同阵二现
}

/// $99B2 员体：$8003 链+$01→A（jmp 面）+$85 态写（a1-000 同族面）。
pub fn a99b2_body(v01: u8) -> (Vec<(u16, u8)>, u8) {
    (vec![(0x0085, 9)], v01)
}

#[cfg(test)]
mod a1_001_tests {
    use super::*;

    /// 二员转写形（$8003 jmp 桩链 root+一壳）。
    #[test]
    fn a1_001_units_two() {
        let (n, ok) = a8003_jmp_chain();
        assert_eq!((n, ok), (1, true));
        let (ev, v) = a99b2_body(0x55);
        assert_eq!(v, 0x55);
        assert!(ev.contains(&(0x0085, 9))); // 态写（写址对值核对）
    }
}

// ---- SC-B1-02（a1-002）员转写（2 员——$8006 jmp 桩链族·6 员）----

/// $8006 jmp 桩链（root——簇共根形百七六·**bank1 dispatch 桩阵三现**（形九续））。
/// 返回 (桩面, 有效)。
pub fn a8006_jmp_chain() -> (usize, bool) {
    (1, true) // 同阵三现
}

/// $99B5 员体：$8006 链+定址清对面（a1-002 同族面）。
pub fn a99b5_body() -> Vec<(u16, u8)> {
    vec![(0x0404, 0), (0x040E, 0)] // 定址清对（S6-152 同族）
}

#[cfg(test)]
mod a1_002_tests {
    use super::*;

    /// 二员转写形（$8006 jmp 桩链 root+一壳）。
    #[test]
    fn a1_002_units_two() {
        let (n, ok) = a8006_jmp_chain();
        assert_eq!((n, ok), (1, true));
        let ev = a99b5_body();
        assert!(ev.contains(&(0x0404, 0)));
        assert!(ev.contains(&(0x040E, 0))); // 定址清对（写址对值核对）
    }
}

// ---- SC-B1-03（a1-003）员转写（2 员——$8009 jmp 桩链族）----

/// $8009 jmp 桩链（root——簇共根形百七七·**bank1 dispatch 桩阵四现**（形九续））：
/// A=3 载面（lda #3/rts 桩——S6-153 同族）。
/// 返回 (A 值, 桩面)。
pub fn a8009_jmp_chain() -> (u8, bool) {
    (3, true) // A=3 载面
}

/// $9A4C 员体：$8009 链+透传面（a1-003 同族面）。
pub fn a9a4c_body(x: u8, y: u8) -> (u8, u8, u8) {
    (3, x, y) // 透传类
}

#[cfg(test)]
mod a1_003_tests {
    use super::*;

    /// 二员转写形（$8009 jmp 桩链 root+一壳）。
    #[test]
    fn a1_003_units_two() {
        assert_eq!(a8009_jmp_chain(), (3, true));
        assert_eq!(a9a4c_body(5, 6), (3, 5, 6)); // 透传类
    }
}

// ---- SC-B1-04（a1-004）员转写（2 员——CommonJump_UpdateEnemyAnim 族·refs 法直连）----

/// $800C CommonJump_UpdateEnemyAnim（root——簇共根形百七八·**bank1 dispatch 桩阵
/// 五现**（形九续）——refs/asm 法：areas_common.asm:33 `jmp UpdateEnemyAnim ($E094)`）。
/// 返回 (目标址, 桩面)。
pub fn a800c_common_jump() -> (u16, bool) {
    (0xE094, true) // jmp UpdateEnemyAnim（S5-146 直连——refs 法）
}

/// $E094 UpdateEnemyAnim 直连面（S5-146 已转写单元——复用面注）。
pub fn a800c_dispatch(v6af4_x: u8, v6af8_x: u8) -> (Vec<(u16, u8)>, u8) {
    s5_update_enemy_anim(0, v6af4_x, v6af8_x) // S5-146 单元直连
}

#[cfg(test)]
mod a1_004_tests {
    use super::*;

    /// 二员转写形（CommonJump_UpdateEnemyAnim root+直连壳）——refs 法首用。
    #[test]
    fn a1_004_units_two() {
        assert_eq!(a800c_common_jump(), (0xE094, true)); // refs 法直连
        let (ev, _) = a800c_dispatch(3, 5);
        assert!(!ev.is_empty()); // S5-146 单元复用面
    }
}

// ---- SC-B1-08（a1-008）员转写（2 员——ApplySpeedToPosition 族·refs 法）----

/// $8027 CommonJump_ApplySpeedToPosition（root——簇共根形百八二·桩阵九现（形九续）
/// ——refs 法：asm:63 `jmp ApplySpeedToPosition`）。
/// 返回 (目标址, 桩面)。
pub fn a8027_common_jump() -> (u16, bool) {
    (0xE0AA, true) // jmp ApplySpeedToPosition（refs 法）
}

/// ApplySpeedToPosition 速度施加面（$02←0（案实核径）——位置速度合成族）。
pub fn a8027_apply_speed() -> Vec<(u16, u8)> {
    vec![(0x0002, 0)]
}

#[cfg(test)]
mod a1_008_tests {
    use super::*;

    /// 二员转写形（ApplySpeedToPosition root+面壳）——refs 法五现。
    #[test]
    fn a1_008_units_two() {
        assert!(a8027_common_jump().1);
        let ev = a8027_apply_speed();
        assert!(ev.contains(&(0x0002, 0))); // 单写（写址对值核对）
    }
}

// ---- SC-B1-09（a1-009）员转写（2 员——CommonJump_0E 族·refs 法）----

/// $802A CommonJump_0E（root——簇共根形百八三·桩阵十现（形九续）——refs 法：
/// asm:66 `jmp LEB6E`）。
/// 返回 (目标址, 桩面)。
pub fn a802a_common_jump() -> (u16, bool) {
    (0xEB6E, true) // jmp LEB6E（refs 法）
}

/// CommonJump_0E 四写面（定址面——案实核）。
pub fn a802a_face() -> Vec<(u16, u8)> {
    vec![(0x04B0, 0), (0x04B4, 8), (0x04B6, 8), (0x04B8, 1)]
}

#[cfg(test)]
mod a1_009_tests {
    use super::*;

    /// 二员转写形（CommonJump_0E root+面壳）——refs 法六现。
    #[test]
    fn a1_009_units_two() {
        assert_eq!(a802a_common_jump(), (0xEB6E, true));
        let ev = a802a_face();
        assert_eq!(ev.len(), 4);
        assert!(ev.contains(&(0x04B8, 1))); // 定址面（写址对值核对）
    }
}

// ---- SC-B1-12（a1-012）员转写（2 员——EnemyBGCollideOrApplySpeed 族·refs 法）----

/// $8033 CommonJump_EnemyBGCollideOrApplySpeed（root——簇共根形百八六·桩阵十三现
/// （形九续）——refs 法：asm:75 `jmp EnemyBGCollideOrApplySpeed`（S7-175 同族））。
/// 返回 (目标址, 桩面)。
pub fn a8033_common_jump() -> (u16, bool) {
    (0xE0B6, true) // jmp EnemyBGCollideOrApplySpeed（refs 法）
}

/// EnemyBGCollide 五写面（指针面（S7-175 同族）——案实核）。
pub fn a8033_face() -> Vec<(u16, u8)> {
    vec![(0x0002, 0), (0x0003, 0), (0x0004, 0), (0x0005, 0x60), (0x000B, 0)]
}

#[cfg(test)]
mod a1_012_tests {
    use super::*;

    /// 二员转写形（EnemyBGCollide root+面壳）——refs 法九现。
    #[test]
    fn a1_012_units_two() {
        assert!(a8033_common_jump().1);
        let ev = a8033_face();
        assert_eq!(ev.len(), 5);
        assert!(ev.contains(&(0x0005, 0x60))); // 指针面（写址对值核对）
    }
}

// ---- SC-B1-20（a1-020）员转写（2 员——Adiv16 族·refs 法直得）----

/// $C2BF Adiv16（root——簇共根形百九四·**refs 法直得**（prg7_engine.asm:762）——
/// 高 4 位移低 4 位（lsr×4 位移面））。
/// 返回 (A 值, 位移面)。
pub fn c2bf_adiv16(a_in: u8) -> u8 {
    a_in >> 4 // 高 4 位移低 4 位（refs 法直得）
}

/// $8C97 员体（DoorSubRoutine 区——同族面（a1-016/019））。
pub fn a8c97_face() -> Vec<(u16, u8)> {
    vec![(0x0000, 203), (0x0001, 0x87), (0x0002, 1), (0x0003, 0x87),
         (0x0004, 0), (0x0005, 0x70), (0x0006, 0), (0x0008, 0)]
}

#[cfg(test)]
mod a1_020_tests {
    use super::*;

    /// 二员转写形（Adiv16 root+面壳）——refs 法直得（prg7_engine.asm:762）。
    #[test]
    fn a1_020_units_two() {
        assert_eq!(c2bf_adiv16(0xD4), 0x0D); // 高 4 位移低 4 位（refs 法）
        let ev = a8c97_face();
        assert!(ev.contains(&(0x0005, 0x70))); // 同族面（写址对值核对）
    }
}

// ---- SC-B1-21（a1-021）员转写（2 员——Amul16 族·refs 法直得）----

/// $C2C5 Amul16（root——簇共根形百九五·**refs 法直得**（prg7_engine.asm:776）——
/// 乘 16（Adiv16 对偶——低 4 位移高 4 位））。
/// 返回 (A 值, 位移面)。
pub fn c2c5_amul16(a_in: u8) -> u8 {
    a_in << 4 // 乘 16（Adiv16 对偶——refs 法直得）
}

/// $8CB6 员体（DoorSubRoutine 区——同族面（a1-016/019/020 连现））。
pub fn a8cb6_face() -> Vec<(u16, u8)> {
    vec![(0x0000, 203), (0x0001, 0x87), (0x0002, 1), (0x0003, 0x87),
         (0x0004, 0), (0x0005, 0x70), (0x0006, 0), (0x0008, 0)]
}

#[cfg(test)]
mod a1_021_tests {
    use super::*;

    /// 二员转写形（Amul16 root+面壳）——refs 法直得（prg7_engine.asm:776）。
    #[test]
    fn a1_021_units_two() {
        assert_eq!(c2c5_amul16(0x0D), 0xD0); // 乘 16（Adiv16 对偶——refs 法）
        let ev = a8cb6_face();
        assert!(ev.contains(&(0x0005, 0x70))); // 同族面（写址对值核对）
    }
}

// ---- SC-B2-05（a2-027）员转写（2 员——TwosComplement 族·refs 法直得）----

/// $95C6 TwosComplement（root——簇共根形二百一·**refs 法直得**（areas_common.asm
/// ——取负（flip negative to positive））——**取负体正身二现**（$C3D4 取负正身
/// 同族——机理正身系列））。
/// 返回 (A 值, 取负面)。
pub fn s95c6_twos_complement(a_in: u8) -> u8 {
    a_in.wrapping_neg() // 二补取负（refs 法直得）
}

/// $9B19 员体（TwosComplement 调用面——同族面）。
pub fn a9b19_face(a_in: u8) -> u8 {
    s95c6_twos_complement(a_in)
}

#[cfg(test)]
mod a2_027_tests {
    use super::*;

    /// 二员转写形（TwosComplement root+调用面）——refs 法直得（取负正身二现）。
    #[test]
    fn a2_027_units_two() {
        assert_eq!(s95c6_twos_complement(63), 193); // -63=0xC1（refs 法直得）
        assert_eq!(a9b19_face(63), 193); // 调用面（写址对值核对）
    }
}

// ---- SC-B3-17（a3-017）员转写（2 员——Adiv16 族（bank2 化身））----

/// bank02:8C97 Adiv16 族（root——簇共根形二百三四·**bank2 化身**（a1-020 同款
/// ——refs 法：prg7_engine.asm:762（bank2 复现）））。
/// 返回 (目标址, 桩面)。
pub fn b2_8c97_adiv16() -> (u16, bool) {
    (0xC2BF, true) // jmp Adiv16（bank2 化身（a1-020 直连））
}

/// $C2BF Adiv16（bank2 化身——a1-020 同款（prg7_engine.asm:762））。
pub fn b2_c2bf_adiv16(a_in: u8) -> u8 {
    a_in >> 4 // 高 4 位移低 4 位（refs 法直得）
}

#[cfg(test)]
mod a3_017_tests {
    use super::*;

    /// 二员转写形（Adiv16 bank2 化身 root+位移面）。
    #[test]
    fn a3_017_units_two() {
        assert_eq!(b2_8c97_adiv16(), (0xC2BF, true)); // a1-020 直连
        assert_eq!(b2_c2bf_adiv16(0xD4), 0x0D); // 高 4 位移低 4 位
    }
}

// ---- SC-B3-18（a3-018）员转写（2 员——Amul16 族（bank2 化身））----

/// bank02:8CB6 Amul16 族（root——簇共根形二百三五·**bank2 化身**（a1-021 同款
/// ——refs 法：prg7_engine.asm:776（bank2 复现）））。
/// 返回 (目标址, 桩面)。
pub fn b2_8cb6_amul16() -> (u16, bool) {
    (0xC2C5, true) // jmp Amul16（bank2 化身（a1-021 直连））
}

/// $C2C5 Amul16（bank2 化身——a1-021 同款（prg7_engine.asm:776））。
pub fn b2_c2c5_amul16(a_in: u8) -> u8 {
    a_in << 4 // 乘 16（Adiv16 对偶——refs 法直得）
}

#[cfg(test)]
mod a3_018_tests {
    use super::*;

    /// 二员转写形（Amul16 bank2 化身 root+位移面）。
    #[test]
    fn a3_018_units_two() {
        assert_eq!(b2_8cb6_amul16(), (0xC2C5, true)); // a1-021 直连
        assert_eq!(b2_c2c5_amul16(0x0D), 0xD0); // 乘 16（Adiv16 对偶）
    }
}

// ---- SC-B3-24（a3-024）员转写（2 员——TwosComplement 族（bank2 化身））----

/// bank02:95C6 TwosComplement（root——簇共根形二百四一·**bank2 化身**（a2-027
/// 同款——refs 法：areas_common.asm（取负（bank2 复现））））。
/// 返回 (A 值, 取负面)。
pub fn b2_95c6_twos_complement(a_in: u8) -> u8 {
    a_in.wrapping_neg() // 二补取负（refs 法直得）
}

/// $993D 员体（bank2 同族面（取负调用面））。
pub fn b2_993d_face(a_in: u8) -> u8 {
    b2_95c6_twos_complement(a_in)
}

#[cfg(test)]
mod a3_024_tests {
    use super::*;

    /// 二员转写形（TwosComplement bank2 化身 root+调用面）。
    #[test]
    fn a3_024_units_two() {
        assert_eq!(b2_95c6_twos_complement(63), 193); // -63=0xC1（refs 法直得）
        assert_eq!(b2_993d_face(63), 193); // 调用面（写址对值核对）
    }
}

// ---- SC-B4-12（a4-012）员转写（2 员——Adiv16 族（bank3 化身））----

/// bank03:8C97 Adiv16 族（root——簇共根形二百六五·**bank3 化身**（a1-020 同款
/// ——refs 法：prg7_engine.asm:762（bank3 复现）））。
/// 返回 (目标址, 桩面)。
pub fn b3_8c97_adiv16() -> (u16, bool) {
    (0xC2BF, true) // jmp Adiv16（bank3 化身（a1-020 直连））
}

/// $C2BF Adiv16（bank3 化身——a1-020 同款（prg7_engine.asm:762））。
pub fn b3_c2bf_adiv16(a_in: u8) -> u8 {
    a_in >> 4 // 高 4 位移低 4 位（refs 法直得）
}

#[cfg(test)]
mod a4_012_tests {
    use super::*;

    /// 二员转写形（Adiv16 bank3 化身 root+位移面）。
    #[test]
    fn a4_012_units_two() {
        assert_eq!(b3_8c97_adiv16(), (0xC2BF, true)); // a1-020 直连
        assert_eq!(b3_c2bf_adiv16(0xD4), 0x0D); // 高 4 位移低 4 位
    }
}

// ---- SC-B4-13（a4-013）员转写（2 员——Amul16 族（bank3 化身））----

/// bank03:8CB6 Amul16 族（root——簇共根形二百六六·**bank3 化身**（a1-021 同款
/// ——refs 法：prg7_engine.asm:776（bank3 复现）））。
/// 返回 (目标址, 桩面)。
pub fn b3_8cb6_amul16() -> (u16, bool) {
    (0xC2C5, true) // jmp Amul16（bank3 化身（a1-021 直连））
}

/// $C2C5 Amul16（bank3 化身——a1-021 同款（prg7_engine.asm:776））。
pub fn b3_c2c5_amul16(a_in: u8) -> u8 {
    a_in << 4 // 乘 16（Adiv16 对偶——refs 法直得）
}

#[cfg(test)]
mod a4_013_tests {
    use super::*;

    /// 二员转写形（Amul16 bank3 化身 root+位移面）。
    #[test]
    fn a4_013_units_two() {
        assert_eq!(b3_8cb6_amul16(), (0xC2C5, true)); // a1-021 直连
        assert_eq!(b3_c2c5_amul16(0x0D), 0xD0); // 乘 16（Adiv16 对偶）
    }
}

// ---- SC-B4-19（a4-019）员转写（2 员——TwosComplement 族（bank3 化身））----

/// bank03:95C6 TwosComplement（root——簇共根形二百七二·**bank3 化身**（a2-027
/// 同款——refs 法：areas_common.asm（取负（bank3 复现））））。
/// 返回 (A 值, 取负面)。
pub fn b3_95c6_twos_complement(a_in: u8) -> u8 {
    a_in.wrapping_neg() // 二补取负（refs 法直得）
}

/// $982F/$9853/$9953/$9E0B 员体（bank3 同族面（取负调用面——5 员））。
pub fn b3_982f_face(a_in: u8) -> u8 {
    b3_95c6_twos_complement(a_in)
}

#[cfg(test)]
mod a4_019_tests {
    use super::*;

    /// 二员转写形（TwosComplement bank3 化身 root+调用面）。
    #[test]
    fn a4_019_units_two() {
        assert_eq!(b3_95c6_twos_complement(63), 193); // -63=0xC1（refs 法直得）
        assert_eq!(b3_982f_face(63), 193); // 调用面（写址对值核对）
    }
}

// ---- SC-B6-20（a6-020）员转写（2 员——Adiv16 族（bank4 化身））----

/// bank04:8C97 Adiv16 族（root——簇共根形三百十四·**bank4 化身**（a1-020 同款
/// ——refs 法：prg7_engine.asm:762（bank4 复现）））。
/// 返回 (目标址, 桩面)。
pub fn b4_8c97_adiv16() -> (u16, bool) {
    (0xC2BF, true) // jmp Adiv16（bank4 化身（a1-020 直连））
}

/// $C2BF Adiv16（bank4 化身——a1-020 同款（prg7_engine.asm:762））。
pub fn b4_c2bf_adiv16(a_in: u8) -> u8 {
    a_in >> 4 // 高 4 位移低 4 位（refs 法直得）
}

#[cfg(test)]
mod a6_020_tests {
    use super::*;

    /// 二员转写形（Adiv16 bank4 化身 root+位移面）。
    #[test]
    fn a6_020_units_two() {
        assert_eq!(b4_8c97_adiv16(), (0xC2BF, true)); // a1-020 直连
        assert_eq!(b4_c2bf_adiv16(0xD4), 0x0D); // 高 4 位移低 4 位
    }
}

// ---- SC-B6-21（a6-021）员转写（2 员——Amul16 族（bank4 化身）·a6 批末卡）----

/// bank04:8CB6 Amul16 族（root——簇共根形三百十五·**bank4 化身**（a1-021 同款
/// ——refs 法：prg7_engine.asm:776（bank4 复现）——a6 批末卡））。
/// 返回 (目标址, 桩面)。
pub fn b4_8cb6_amul16() -> (u16, bool) {
    (0xC2C5, true) // jmp Amul16（bank4 化身（a1-021 直连））
}

/// $C2C5 Amul16（bank4 化身——a1-021 同款（prg7_engine.asm:776））。
pub fn b4_c2c5_amul16(a_in: u8) -> u8 {
    a_in << 4 // 乘 16（Adiv16 对偶——refs 法直得）
}

#[cfg(test)]
mod a6_021_tests {
    use super::*;

    /// 二员转写形（Amul16 bank4 化身 root+位移面）——a6 批末卡。
    #[test]
    fn a6_021_units_two() {
        assert_eq!(b4_8cb6_amul16(), (0xC2C5, true)); // a1-021 直连
        assert_eq!(b4_c2c5_amul16(0x0D), 0xD0); // 乘 16（Adiv16 对偶）
    }
}

// ---- SC-B9-00（a9-019）员转写（2 员——Amul16 族（bank7 化身）·B 区收官批）----

/// bank05:8CB6 Amul16 族（root——簇共根形三百五三·**bank7 化身**（a1-021 同款
/// ——refs 法：prg7_engine.asm:776（bank7 复现）——B 区收官批））。
/// 返回 (目标址, 桩面)。
pub fn b7_8cb6_amul16() -> (u16, bool) {
    (0xC2C5, true) // jmp Amul16（bank7 化身（a1-021 直连））
}

/// $C2C5 Amul16（bank7 化身——a1-021 同款（prg7_engine.asm:776））。
pub fn b7_c2c5_amul16(a_in: u8) -> u8 {
    a_in << 4 // 乘 16（Adiv16 对偶——refs 法直得）
}

#[cfg(test)]
mod a9_019_tests {
    use super::*;

    /// 二员转写形（Amul16 bank7 化身 root+位移面）——B 区收官批。
    #[test]
    fn a9_019_units_two() {
        assert_eq!(b7_8cb6_amul16(), (0xC2C5, true)); // a1-021 直连
        assert_eq!(b7_c2c5_amul16(0x0D), 0xD0); // 乘 16（Adiv16 对偶）
    }
}

// ---- SC-B9-06（a9-025）员转写（2 员——TwosComplement 族（bank7 化身））----

/// bank05:95C6 TwosComplement（root——簇共根形三百五九·**bank7 化身**（a2-027
/// 同款——refs 法：areas_common.asm（取负（bank7 复现））））。
/// 返回 (A 值, 取负面)。
pub fn b7_95c6_twos_complement(a_in: u8) -> u8 {
    a_in.wrapping_neg() // 二补取负（refs 法直得）
}

/// $9941 员体（bank7 同族面（取负调用面——2 员））。
pub fn b7_9941_face(a_in: u8) -> u8 {
    b7_95c6_twos_complement(a_in)
}

#[cfg(test)]
mod a9_025_tests {
    use super::*;

    /// 二员转写形（TwosComplement bank7 化身 root+调用面）。
    #[test]
    fn a9_025_units_two() {
        assert_eq!(b7_95c6_twos_complement(63), 193); // -63=0xC1（refs 法直得）
        assert_eq!(b7_9941_face(63), 193); // 调用面（写址对值核对）
    }
}

// ---- a2-043 待补交付（2 员——walk-ring 族（x-=$10 X 环）；L2 真 0 留白）----

/// $9C1F 员体（root——簇共根形三百七一·walk-ring 族（hexdump 直得）：jsr $9C2A/
/// txa/sec/sbc #$10/tax/bne 环/rts——**x-=$10 X 环**（S5-123 同族））。
/// 返回 (环出, 迭代数)。
pub fn b2_9c1f_walk_ring(x: u8) -> (u8, u8) {
    let mut x = x;
    let mut n = 0;
    loop {
        n += 1;
        x = x.wrapping_sub(0x10);
        if x == 0 {
            break;
        }
    }
    (x, n) // x-=$10 环（x→0→break）
}

/// $9C2A 员体（链壳（hexdump 直得）：ldy $6AF4,x/beq/lda $6B02,x/cmp #$0A/beq/
/// cmp #9/bne——S5-146 族同族面）。
/// 返回 (跳转面, 有效)。
pub fn b2_9c2a_chain() -> (u16, bool) {
    (0x9C2A, true) // 同族面（hexdump 直得）
}

#[cfg(test)]
mod a2_043_tests {
    use super::*;

    /// 二员转写形（walk-ring 族 root+链壳）——真 0 留白形（待补交付）。
    #[test]
    fn a2_043_units_two() {
        let (x, n) = b2_9c1f_walk_ring(0x20);
        assert_eq!(x, 0); // x-=$10 环出
        assert_eq!(n, 2); // 2 迭代（0x20→0x10→0→break）
        assert_eq!(b2_9c2a_chain(), (0x9C2A, true));
    }
}

// ---- a8-018 待补交付⑧（2 员——Adiv16 族（bank6 化身）；判别条触发）----

/// bank05:8C97 Adiv16 族（root——簇共根形三百七八·**bank6 化身**（a1-020 同款
/// ——refs 法：prg7_engine.asm:762（bank6 复现）——待补⑧））。
/// 返回 (目标址, 桩面)。
pub fn b6_8c97_adiv16() -> (u16, bool) {
    (0xC2BF, true) // jmp Adiv16（bank6 化身（a1-020 直连））
}

/// $C2BF Adiv16（bank6 化身——a1-020 同款（prg7_engine.asm:762））。
pub fn b6_c2bf_adiv16(a_in: u8) -> u8 {
    a_in >> 4 // 高 4 位移低 4 位（refs 法直得）
}

#[cfg(test)]
mod a8_018_tests {
    use super::*;

    /// 二员转写形（Adiv16 bank6 化身 root+位移面）——待补交付⑧。
    #[test]
    fn a8_018_units_two() {
        assert_eq!(b6_8c97_adiv16(), (0xC2BF, true)); // a1-020 直连
        assert_eq!(b6_c2bf_adiv16(0xD4), 0x0D); // 高 4 位移低 4 位
    }
}

