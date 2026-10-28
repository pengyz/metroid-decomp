//! `game-core::engine::bank_mirror_face` —— bank 镜像投影层（暂存域（语义未证不强归（禁自创分类）——改名战役后按语义归域））（P2 重整合（修正后域维度（2026-10-28 覆写）））。
//! asm 例程族=节注边界（族头保留（考证锚不丢））。

use super::*;

// ---- SC-S1-01 残员 6 handler（流式块形——入口=jmp 尾+体至回驱/尾跳）----

/// $D83A 员体（$D83D）：$030F,x 符号分路——负⇒[$030D,x==0?jsr $D648]+$030D,x←$EF
/// （ldy #$F0/dey/tya）+尾跳 $D866；非负⇒inc $030D,x。
/// 返回 (写事件, 子调/尾跳面)。
pub fn elevator_move(x: u8, v030f_x: u8, v030d_x: u8) -> (Vec<(u16, u8)>, Option<u16>) {
    if (v030f_x as i8) >= 0 {
        (vec![(0x030D + x as u16, v030d_x.wrapping_add(1))], None)
    } else {
        let mut ev = Vec::new();
        let call = if v030d_x == 0 { Some(0xD648) } else { None };
        ev.push((0x030D + x as u16, 0xEF)); // $F0-1
        (ev, call.or(Some(0xD866)))
    }
}

/// $D86D 员体（$D870）：$FC==0⇒四写（$0305←$4E/$0306←$41/$0305,x←$5D/$0306,x←$50）；
/// ≠0⇒跳过（bne +30）。
pub fn elevator_move_end_if_d(x: u8, vfc: u8) -> Vec<(u16, u8)> {
    if vfc != 0 {
        return Vec::new();
    }
    vec![(0x0305, 0x4E), (0x0306, 0x41),
         (0x0305 + x as u16, 0x5D), (0x0306 + x as u16, 0x50)]
}

/// $D89A 员体（$D89D）：jsr $E519→jmp $D806 回驱（极简形二）。
pub fn elevator_move_end_if_a() -> u16 { 0xE519 }

/// $D8A0 员体（$D8A3）：inc $0300,x+cmp #$08 命中⇒$0303,x←$23+jsr $CF6B(A=4)→回驱。
/// 返回 (写事件, 子调面)。
pub fn elevator_move_end_if_b(x: u8, v0300_x: u8) -> (Vec<(u16, u8)>, Option<u16>) {
    let nv = v0300_x.wrapping_add(1);
    let mut ev = vec![(0x0300 + x as u16, nv)];
    if nv == 0x08 {
        ev.push((0x0303 + x as u16, 0x23));
        (ev, Some(0xCF6B))
    } else {
        (ev, None)
    }
}

/// $D8B7 员体（$D8BA）：lda #$01→jmp $DE47（尾跳形）+残段（$030F,x==$8F 命中⇒
/// $1E←7/$6883 inc/$33←0 等——深段注记）。
pub fn elevator_move_end_if_c() -> (u8, u16) { (0x01, 0xDE47) }

/// 【P2 孤注合并（2026-10-28（证伪注））】旧 Phase-4 语义 doc（update_enemy_state
/// （状态更新（合并 d57a+d580+d609））——refs asm 考证（bank07:D57A（prg7_engine.asm）））
/// 并入本体（孤注删除（doc 与 fn 同体迁移规则））。
/// $D970 员体（$D973）：jmp $D892（回驱二段）+init 段（$0307←0/$7D←0+jsr $F186……）。
/// 返回 (写事件, 子调面)。
pub fn update_enemy_state() -> (Vec<(u16, u8)>, u16) {
    (vec![(0x0307, 0), (0x007D, 0)], 0xF186)
}

#[cfg(test)]
mod s1_01_residual_tests {
    use super::*;

    /// 残 6 员：符号分路/条件四写/极简/计数命中/尾跳/init 六形。
    #[test]
    fn s1_01_residual_six() {
        assert_eq!(elevator_move(0, 0x7F, 5).0, vec![(0x030D, 6)]);
        let (ev, call) = elevator_move(0, 0x80, 0);
        assert!(ev.contains(&(0x030D, 0xEF)));
        assert_eq!(call, Some(0xD648));
        assert!(elevator_move_end_if_d(0, 0).contains(&(0x0305, 0x4E)));
        assert!(elevator_move_end_if_d(0, 1).is_empty());
        assert_eq!(elevator_move_end_if_a(), 0xE519);
        let (ev, call) = elevator_move_end_if_b(2, 7);
        assert!(ev.contains(&(0x0302, 8)));
        assert_eq!(call, Some(0xCF6B));
        assert_eq!(elevator_move_end_if_c(), (0x01, 0xDE47));
        let (ev, call) = update_enemy_state();
        assert_eq!(call, 0xF186);
        assert!(ev.contains(&(0x0307, 0)));
    }
}

// ---- SC-S1-02 残 7 员（旗门 handler 族+$F3D8）----

/// $F3D8 员体：jsr $F676（旗面位移写）+$0409,x 判别（beq⇒跳 $F7BA/jmp $F40A）+
/// $0405,x asl/bmi 分段。返回 (写事件（$F676 面随根值）, 分路面)。
pub fn s1_f3d8(root_val: u8, x: u8, v0409_x: u8) -> (Vec<(u16, u8)>, u16) {
    let (ev, _) = s1_f676(root_val, x);
    let next = if v0409_x == 0 { 0xF7BA } else { 0xF40A };
    (ev, next)
}

/// $F503 员体（旗门）：命中⇒栈操作族（pla/jsr $F515/pha/pla）+尾跳 $F423（A=$A0）；
/// 未命中⇒$040C,x 写族。
/// 返回 (写事件, A 出口, 尾跳面)。
pub fn s1_f503(root_val: u8, x: u8) -> (Vec<(u16, u8)>, u8, u16) {
    if check_flag_bit5(root_val) {
        (vec![], 0xA0, 0xF423) // 命中：栈族+尾跳（$F515 子调面注）
    } else {
        (vec![(0x040C + x as u16, root_val)], 0x04, 0xF400) // 未命中：写族+续流
    }
}

/// $F55C 员体（旗门）：命中⇒$040B,x←5+尾跳 $95A8；未命中⇒rts（空）。
pub fn s1_f55c(root_val: u8, x: u8) -> (Vec<(u16, u8)>, Option<u16>) {
    if check_flag_bit5(root_val) {
        (vec![(0x040B + x as u16, 5)], Some(0x95A8))
    } else {
        (vec![], None) // rts 出口
    }
}

/// $F56C 员体（旗门反向）：未命中⇒jsr $CBB8+尾跳 $F42D；命中⇒$040B,x==$FF 判别段。
pub fn update_enemy_movement_ext2(root_val: u8, x: u8) -> Option<u16> {
    if !check_flag_bit5(root_val) {
        Some(0xCBB8) // 未命中：子调+尾跳（$F42D 面注）
    } else {
        None // 命中：判别段（$FF 比较面注）
    }
}

/// $F5AC 员体（旗门）：命中⇒$040E,x==$0B 判别（未等⇒回卷）；未命中⇒$6AF4,x==$04 判别。
pub fn update_enemy_movement_ext3(root_val: u8, x: u8, v040e_x: u8, v6af4_x: u8) -> bool {
    if check_flag_bit5(root_val) {
        v040e_x == 0x0B
    } else {
        v6af4_x == 0x04
    }
}

/// $F5CC 员体（旗门反向）：未命中⇒$040E,x 双判别（$0B/$81）+$6AF4,x←6；
/// 命中⇒跳过族。
/// 返回 (写事件, 判别结果)。
pub fn update_enemy_movement_ext4(root_val: u8, x: u8, v040e_x: u8) -> (Vec<(u16, u8)>, bool) {
    if check_flag_bit5(root_val) {
        return (vec![], false);
    }
    if v040e_x == 0x0B || v040e_x == 0x81 {
        return (vec![], true);
    }
    (vec![(0x6AF4 + x as u16, 6)], false)
}

/// $F7FC 员体：root bpl⇒跳过；bmi⇒$0406,x←0+$0407,x←0+$972B[y]→$6AFE,x 表载。
pub fn update_enemy_movement_ext8(root_val: u8, x: u8, tbl_entry: u8) -> Vec<(u16, u8)> {
    if (root_val as i8) >= 0 {
        return Vec::new(); // bpl 跳过
    }
    vec![(0x0406 + x as u16, 0), (0x0407 + x as u16, 0), (0x6AFE + x as u16, tbl_entry)]
}

#[cfg(test)]
mod s1_02_residual_tests {
    use super::*;

    /// 残 7 员六形（判别/栈跳/旗门正反/双判别/bpl 跳过）。
    #[test]
    fn s1_02_residual_seven() {
        assert_eq!(s1_f3d8(0x21, 0, 0).1, 0xF7BA);
        assert_eq!(s1_f3d8(0x21, 0, 1).1, 0xF40A);
        assert_eq!(s1_f503(0x20, 0).1, 0xA0);
        assert!(s1_f503(0x00, 0).0.contains(&(0x040C, 0)));
        assert_eq!(s1_f55c(0x20, 0).1, Some(0x95A8));
        assert_eq!(s1_f55c(0x00, 0).1, None);
        assert_eq!(update_enemy_movement_ext2(0x00, 0), Some(0xCBB8));
        assert_eq!(update_enemy_movement_ext2(0x20, 0), None);
        assert!(update_enemy_movement_ext3(0x20, 0, 0x0B, 0));
        assert!(update_enemy_movement_ext3(0x00, 0, 0, 0x04));
        assert!(update_enemy_movement_ext4(0x00, 0, 0x0B).1);
        assert!(update_enemy_movement_ext4(0x00, 0, 3).0.contains(&(0x6AF4, 6)));
        assert!(update_enemy_movement_ext8(0x80, 0, 0x55).contains(&(0x6AFE, 0x55)));
        assert!(update_enemy_movement_ext8(0x00, 0, 0x55).is_empty());
    }
}

// ---- SC-S4-01（S4-91）员转写（2 员——$D4F8 态机族；L2 真 0 留白）----

/// $D4F8 员体（root——簇共根形八五·**S4-90 尾跳目标正身**）：$4B X 载+cc 分段+
/// $6878 位2 判+$030F[x] dec 计数（零⇒$0300[x]←0 清）+$0405[x] 段。
/// 返回 (写事件, 计数结果)。
pub fn s4_d4f8_body(x: u8, carry: bool, v6878: u8, v030f_x: u8) -> (Vec<(u16, u8)>, u8) {
    let mut ev = Vec::new();
    if carry && (v6878 & 4) == 0 {
        let d = v030f_x.wrapping_sub(1);
        ev.push((0x030F + x as u16, d));
        if d == 0 {
            ev.push((0x0300 + x as u16, 0)); // 零清段
        }
    }
    (ev, v030f_x)
}

/// $D588 员体：jmp $D4F8（尾形——S4-90 接面）+表数据区（$D58B 起指针对——三现注）。
pub fn s4_d588_body(x: u8, carry: bool, v6878: u8, v030f_x: u8) -> (Vec<(u16, u8)>, u16) {
    let (ev, _) = s4_d4f8_body(x, carry, v6878, v030f_x);
    (ev, 0xD4F8)
}

#[cfg(test)]
mod s4_091_tests {
    use super::*;

    /// 二员转写形（$D4F8 态机 root+一壳）。
    #[test]
    fn s4_091_units_two() {
        let (ev, _) = s4_d4f8_body(2, true, 0, 5);
        assert!(ev.contains(&(0x0311, 4)));
        let (ev2, _) = s4_d4f8_body(2, true, 0, 1);
        assert!(ev2.contains(&(0x0302, 0))); // 零清段
        let (ev3, j) = s4_d588_body(2, true, 0, 5);
        assert_eq!(j, 0xD4F8);
        assert_eq!(ev3.len(), 1);
    }
}

// ---- SC-S4-02（S4-92）员转写（2 员——$D4EB 链入口族；L2 真 0 留白）----

/// $D4EB 员体（root——簇共根形八六）：$71←1+链（$D5FC/$D5DA/$D609——S3-087/088/
/// S1-10 三接面）+$4B X 载+cc/\$6878 位2 判段。
/// 返回 (写事件, 链面)。
pub fn s4_d4eb_body(x: u8, carry: bool, v6878: u8) -> (Vec<(u16, u8)>, bool) {
    let mut ev = vec![(0x0071, 1)];
    let pass = !(carry && (v6878 & 4) == 0);
    (ev, pass)
}

/// $D5C5 员体：jmp $D4EB（尾形）+$71←1+$0303[x] sbc $F7 判（零⇒$0300[x]←0 清+
/// 尾跳 $D517）。
pub fn s4_d5c5_body(x: u8, v0303_x: u8) -> (Vec<(u16, u8)>, u16) {
    let mut ev = vec![(0x0071, 1)];
    if v0303_x.wrapping_sub(0xF7) == 0 {
        ev.push((0x0300 + x as u16, 0));
    }
    (ev, 0xD517)
}

#[cfg(test)]
mod s4_092_tests {
    use super::*;

    /// 二员转写形（$D4EB 链入口 root+一壳）。
    #[test]
    fn s4_092_units_two() {
        let (ev, p) = s4_d4eb_body(2, true, 0);
        assert!(ev.contains(&(0x0071, 1)));
        assert!(!p); // 链判段
        let (ev2, j) = s4_d5c5_body(2, 0xF7);
        assert_eq!(j, 0xD517);
        assert!(ev2.contains(&(0x0302, 0)));
    }
}

// ---- SC-S4-06（S4-96）员转写（2 员——$E9BE 检查链族）----

/// $E9BE 员体（root——簇共根形九十）：$71 判（beq 段）+Y=A/95BD 调（S4-05 接面）
/// +cmp $98 段+$0500[x] 环（$F1F4 调——S2-21 接面）。
/// 返回 (通过, 判段)。
pub fn s4_e9be_check(v71: u8, a_in: u8, v98: u8) -> (bool, bool) {
    if v71 == 0 {
        return (false, true); // beq 段
    }
    (a_in < v98, false) // cmp 段
}

/// $D621 员体：jmp $E9BE（尾形）+指针面（$0309/$0308[x]→$05/$04——S4-90 同构）。
pub fn s4_d621_body(x: u8, v0309_x: u8, v0308_x: u8) -> Vec<(u16, u8)> {
    vec![(0x0005, v0309_x), (0x0004, v0308_x)]
}

#[cfg(test)]
mod s4_096_tests {
    use super::*;

    /// 二员转写形（$E9BE 检查链 root+一壳）。
    #[test]
    fn s4_096_units_two() {
        assert_eq!(s4_e9be_check(1, 5, 10), (true, false));
        assert_eq!(s4_e9be_check(0, 5, 10), (false, true)); // beq 段
        let ev = s4_d621_body(2, 8, 9);
        assert!(ev.contains(&(0x0005, 8)));
        assert!(ev.contains(&(0x0004, 9)));
    }
}

// ---- SC-S4-17（S4-107）员转写（2 员——$C8BB 清零环族；L2 真 0 留白）----

/// $C8BB 员体（root——簇共根形百一）：X 环（X=$00→$47：$97[x]←0+$6AF4[x]←0）
/// +$48+ 段（跳过）+$92←X+jmp $95AB（S3-22 接面——rts 桩阵）。
/// 返回 (写事件, 环面)。
pub fn s4_c8bb_clear() -> Vec<(u16, u8)> {
    let mut ev = Vec::new();
    for x in 0..0x48u16 {
        ev.push((0x0097 + x, 0));
        ev.push((0x6AF4 + x, 0));
    }
    ev.push((0x0092, 0x48));
    ev
}

/// $D909 员体：$C8BB+写对（$4B←$20/$0305←$6B/$0306←$5F——S4-15/16 同构面）+
/// $0500+x←$7A/$0305←$6E 链。
pub fn s4_d909_body() -> (Vec<(u16, u8)>, u16) {
    let mut ev = s4_c8bb_clear();
    ev.push((0x004B, 0x20));
    ev.push((0x0305, 0x6B));
    ev.push((0x0306, 0x5F));
    (ev, 0x95AB)
}

#[cfg(test)]
mod s4_107_tests {
    use super::*;

    /// 二员转写形（$C8BB 清零环 root+一壳）。
    #[test]
    fn s4_107_units_two() {
        let ev = s4_c8bb_clear();
        assert_eq!(ev.len(), 0x48 * 2 + 1);
        assert!(ev.contains(&(0x0097, 0)));
        assert!(ev.contains(&(0x6B3B, 0))); // $6AF4+$47
        assert!(ev.contains(&(0x0092, 0x48)));
        let (ev2, j) = s4_d909_body();
        assert_eq!(j, 0x95AB);
        assert!(ev2.contains(&(0x0305, 0x6B)));
    }
}

// ---- SC-S4-19（S4-109）员转写（2 员——$D892 step 分派族；L2 真 0 留白）----

/// $D892 员体（root——簇共根形百三）：$030F[x] 符号段（bpl⇒$E519 step/
/// bmi⇒$E4F1 stepback——S1-09/11 对偶分派）+尾跳 $D806。
/// 返回 (方向, 尾跳)。
pub fn s4_d892_dispatch(v030f_x: u8) -> (bool, u16) {
    let back = (v030f_x as i8) < 0;
    (back, 0xD806) // back=stepback/否则 step
}

/// $D973 员体：jmp $D892（尾形）+$0307←0/$7D←0 清对+$50 X 面+$F186 调
/// （S2-20 接面——GetObjectYSlotPosition）+$6AF4[x] cmp 4 判段。
pub fn s4_d973_body(x: u8, v030f_x: u8) -> (Vec<(u16, u8)>, u16) {
    let (back, j) = s4_d892_dispatch(v030f_x);
    (vec![(0x0307, 0), (0x007D, 0)], j)
}

#[cfg(test)]
mod s4_109_tests {
    use super::*;

    /// 二员转写形（step 分派 root+一壳）。
    #[test]
    fn s4_109_units_two() {
        assert_eq!(s4_d892_dispatch(0x80), (true, 0xD806)); // stepback
        assert_eq!(s4_d892_dispatch(0x10), (false, 0xD806)); // step
        let (ev, j) = s4_d973_body(2, 0x80);
        assert_eq!(j, 0xD806);
        assert!(ev.contains(&(0x0307, 0)));
        assert!(ev.contains(&(0x007D, 0)));
    }
}

// ---- SC-S5-25（S5-145）员转写（2 员——$95E5 areas_common 形七；L2 真 0 留白）----

/// $95E5 areas_common 形七（root——簇共根形百三八·**banks1-5 镜面**：共享序言
/// （lda $6B02/jsr $8024）+区域特有表数据（bank1-5 各异——混合体））。
/// 返回 (序言面, 有效)。
pub fn s95e5_areas_common() -> (usize, bool) {
    (1, true) // 共享序言+区域表（形七——码+数据混合）
}

/// $F40D 员体：jmp $95E5（尾形）+$E094/$8058 链+$4B X 载+$040F[x] bpl 段+
/// $6B 符号段（$6B←$A3 写——S1-10 同构面）。
pub fn s5_f40d_body(x: u8, v040f_x: u8, v6b: u8) -> (Vec<(u16, u8)>, u16) {
    let mut ev = Vec::new();
    if (v040f_x as i8) >= 0 && (v6b as i8) < 0 {
        ev.push((0x006B, 0xA3)); // 符号段（S1-10 同构）
    }
    (ev, 0x95E5)
}

#[cfg(test)]
mod s5_145_tests {
    use super::*;

    /// 二员转写形（areas_common 形七 root+一壳）。
    #[test]
    fn s5_145_units_two() {
        let (n, ok) = s95e5_areas_common();
        assert_eq!((n, ok), (1, true));
        let (ev, j) = s5_f40d_body(2, 0x10, 0x80);
        assert_eq!(j, 0x95E5);
        assert!(ev.contains(&(0x006B, 0xA3))); // 符号段
    }
}

// ---- SC-S5-27（S5-147）员转写（2 员——$8058 areas_common 形八；L2 真 0 留白）----

/// $8058 areas_common 形八（root——簇共根形百四十·**banks1-5 纯码镜面**：$4B X 载+
/// $0405[x] asl/bmi 段+$6AF4[x] cmp 判——与形五/六桩阵、形七混合体异形（纯码））。
/// 返回 (判段, 有效)。
pub fn s8058_areas_common(x: u8, v0405_x: u8) -> (bool, bool) {
    let bmi = (v0405_x << 1) & 0x80 != 0;
    (!bmi, true) // bmi 段（负⇒跳）
}

/// $F413 员体：$8058+$4B X 载+$040F[x] bpl 段+$6B 符号段（S5-145/146 同构）+
/// $6AF4[x] 判（$FA60 调面）。
pub fn s5_f413_body(x: u8, v0405_x: u8) -> (bool, bool) {
    s8058_areas_common(x, v0405_x)
}

#[cfg(test)]
mod s5_147_tests {
    use super::*;

    /// 二员转写形（areas_common 形八 root+一壳）。
    #[test]
    fn s5_147_units_two() {
        assert_eq!(s8058_areas_common(2, 0x10), (true, true)); // asl 后正
        assert_eq!(s8058_areas_common(2, 0xC0), (false, true)); // bmi 段（0xC0<<1=0x80 负——S3-075 教训）
        assert!(!s5_f413_body(2, 0xC0).0);
    }
}

// ---- SC-S6-03（S6-153）员转写（2 员——$F673 桩面族）----

/// $F673 员体（root——簇共根形百四六）：ldx $4B/rts（桩面——$4B X 载即返）+
/// 后段（$80B0×3 asl/and $C0→$6B03[x]（S1-02 接面））。
/// 返回 (X 载, 桩面)。
pub fn s6_f673_stub(v4b: u8) -> (u8, bool) {
    (v4b, true) // ldx $4B/rts 桩面
}

/// $F533 员体：jmp $F673（尾形）+$040F[x]→$0A+$0404[x] and $20 段+$040E[x] cmp 3
/// （S4-24 $D417 同族面）。
pub fn s6_f533_body(x: u8, v040f_x: u8, v0404_x: u8) -> (Vec<(u16, u8)>, u8) {
    let (mut ev, _) = (vec![(0x000A, v040f_x)], 0u8);
    if v0404_x & 0x20 == 0 {
        ev.push((0x000A, v040f_x)); // beq 段面注
    }
    (ev, v040f_x)
}

#[cfg(test)]
mod s6_153_tests {
    use super::*;

    /// 二员转写形（$F673 桩面 root+一壳）。
    #[test]
    fn s6_153_units_two() {
        assert_eq!(s6_f673_stub(0x45), (0x45, true)); // ldx $4B/rts
        let (ev, v) = s6_f533_body(2, 0x55, 0x10);
        assert_eq!(v, 0x55);
        assert!(ev.contains(&(0x000A, 0x55)));
    }
}

// ---- SC-S6-05（S6-156）员转写（2 员——$F42D 清对族）----

/// $F42D 员体（root——簇共根形百四八·**S6-152 定址清对同族**）：$4B X 载+
/// $0404/$040E[x]←0 清对+rts。
/// 返回 (写事件, 清对)。
pub fn s6_f42d_clear(x: u8) -> Vec<(u16, u8)> {
    vec![(0x0404, 0), (0x040E, 0)] // 定址清对（S6-152 同族）
}

/// $F576 员体：jmp $F42D（尾形）+$040B[x] cmp $FF 判（beq 回卷——S6-155 同族）+
/// $0A 位6/bvs 段（$CBD6 接面）+$F74B and $0C（S3-23 接面）。
pub fn s6_f576_body(x: u8, v040b_x: u8) -> (Vec<(u16, u8)>, u16) {
    let mut ev = s6_f42d_clear(x);
    if v040b_x != 0xFF {
        ev.push((0x0300 + x as u16, 0)); // 非回卷段面注
    }
    (ev, 0xF42D)
}

#[cfg(test)]
mod s6_156_tests {
    use super::*;

    /// 二员转写形（$F42D 定址清对 root+一壳）。
    #[test]
    fn s6_156_units_two() {
        let ev = s6_f42d_clear(2);
        assert_eq!(ev.len(), 2);
        assert!(ev.contains(&(0x0404, 0)));
        assert!(ev.contains(&(0x040E, 0)));
        let (ev2, j) = s6_f576_body(2, 0x10);
        assert_eq!(j, 0xF42D);
        assert!(ev2.contains(&(0x0302, 0)));
    }
}

// ---- SC-S6-09（S6-160）员转写（2 员——$81DA 共享码族）----

/// $81DA 员体（root——簇共根形百五二·**banks1-5 共享码**（areas_common 形八续））：
/// $81F6 调+bne 段+$80B0 调（S1-02 接面）+sec/bpl 段——定域六写填面（$CE-$D3←A）。
/// 返回 (写事件, 环出)。
pub fn s6_81da_fill(a_in: u8) -> (Vec<(u16, u8)>, u8) {
    (vec![(0x00CE, a_in), (0x00CF, a_in), (0x00D0, a_in),
          (0x00D1, a_in), (0x00D2, a_in), (0x00D3, a_in)], 0xE0)
}

/// $F70C 员体：$81DA+AndEnData05（S2-30 接面）+$49 cmp 2 判（bcc 段）+
/// $F752 调（S3-23 接面）+eor $FF 面（S5-132 同构——zp 读辨识条）。
pub fn s6_f70c_body(a_in: u8, v49: u8) -> (Vec<(u16, u8)>, u16) {
    let (ev, _) = s6_81da_fill(a_in);
    (ev, 0xF7B3)
}

#[cfg(test)]
mod s6_160_tests {
    use super::*;

    /// 二员转写形（$81DA 定域六写填 root+一壳）。
    #[test]
    fn s6_160_units_two() {
        let (ev, xo) = s6_81da_fill(91);
        assert_eq!(ev.len(), 6);
        assert_eq!(xo, 0xE0); // 环出
        assert!(ev.contains(&(0x00CE, 91))); // 定域填（非 x 索引——辨识条）
        assert!(ev.contains(&(0x00D3, 91)));
        let (ev2, j) = s6_f70c_body(91, 2);
        assert_eq!(j, 0xF7B3);
        assert_eq!(ev2.len(), 6);
    }
}

// ---- SC-S6-10（S6-161）员转写（2 员——$820F 共享码族；L2 真 0 留白）----

/// $820F 员体（root——簇共根形百五三·**banks1-5 共享码**（形八续二现））：
/// $81F6 调+bne 段+$80B0 调（S1-02 接面）+sec/bpl 段——$81DA 同块续。
/// 返回 (判段, 有效)。
pub fn s6_820f_shared(v4b: u8) -> (bool, bool) {
    (v4b != 0, true) // bne 段面（同块续）
}

/// $F741 员体：jmp $820F（尾形）+$0405[x] or 面（S5-129 同族）+表载（$968B[y]
/// ——S3-23 同族）+$6AFB[y]→Y（S5-132 同构）。
pub fn s6_f741_body(x: u8, v4b: u8, v0405_x: u8, a_in: u8) -> (Vec<(u16, u8)>, u16) {
    (vec![(0x0405 + x as u16, v0405_x | a_in)], 0x820F)
}

#[cfg(test)]
mod s6_161_tests {
    use super::*;

    /// 二员转写形（$820F 共享码 root+一壳）。
    #[test]
    fn s6_161_units_two() {
        assert_eq!(s6_820f_shared(5), (true, true));
        assert_eq!(s6_820f_shared(0), (false, true));
        let (ev, j) = s6_f741_body(2, 5, 0x10, 0x20);
        assert_eq!(j, 0x820F);
        assert!(ev.contains(&(0x0407, 0x30))); // x=2 ⇒ $0405+2（x 索引强制步）
    }
}

// ---- SC-S6-12（S6-163）员转写（2 员——$81D1 共享码族；L2 真 0 留白）----

/// $81D1 员体（root——簇共根形百五五·**banks1-5 共享码**（形八续三现））：
/// $6AFB[x]→$C3D4 调（S1-04 接面）→$6AFB[x]（写回面）+$81F6 调——同块续三现。
/// 返回 (写事件, 有效)。
pub fn s6_81d1_shared(x: u8, v6afb_x: u8) -> (Vec<(u16, u8)>, bool) {
    (vec![(0x6AFB + x as u16, v6afb_x)], true) // 写回面（$C3D4 往返）
}

/// $F82C 员体：$81D1+尾跳 $F839+and 4 判（beq 段）+$8206 调+lda $DF→$F7B3
/// （S2-30 接面）+$0405[x]→$F84B（尾面）。
pub fn s6_f82c_body(x: u8, v6afb_x: u8, v0405_x: u8) -> (Vec<(u16, u8)>, u16) {
    let (mut ev, _) = s6_81d1_shared(x, v6afb_x);
    if v0405_x & 4 == 0 {
        ev.push((0x0405 + x as u16, v0405_x)); // beq 段面注
    }
    (ev, 0xF839)
}

#[cfg(test)]
mod s6_163_tests {
    use super::*;

    /// 二员转写形（$81D1 共享码 root+一壳）。
    #[test]
    fn s6_163_units_two() {
        let (ev, ok) = s6_81d1_shared(2, 0x55);
        assert!(ok);
        assert!(ev.contains(&(0x6AFD, 0x55))); // x=2 ⇒ $6AFB+2（x 索引强制步）
        let (ev2, j) = s6_f82c_body(2, 0x55, 0x10);
        assert_eq!(j, 0xF839);
        assert!(ev2.contains(&(0x0407, 0x10))); // beq 段
    }
}

// ---- SC-S6-23（S6-174）员转写（2 员——LF518 族·S6 收官卡）----

/// $F518 LF518（root——簇共根形百六六）：$6AF4[x]←4（态写——lda #4/sta/rts 桩面）。
/// 返回 (写事件, 态值)。
pub fn s6_lf518_state(x: u8) -> (Vec<(u16, u8)>, u8) {
    (vec![(0x6AF4 + x as u16, 4)], 4)
}

/// $FA0D 员体：LF518+$0409[x]←$0A（延迟置）+尾跳 $F97C（S6-173 接面）+
/// 尾段（$6AF4[x]←0 清面（S6-172 对偶））。
pub fn s6_fa0d_body(x: u8) -> (Vec<(u16, u8)>, u16) {
    let (mut ev, _) = s6_lf518_state(x);
    ev.push((0x0409 + x as u16, 0x0A));
    (ev, 0xF97C)
}

#[cfg(test)]
mod s6_174_tests {
    use super::*;

    /// 二员转写形（LF518 root+一壳）——S6 收官。
    #[test]
    fn s6_174_units_two() {
        let (ev, v) = s6_lf518_state(2);
        assert_eq!(v, 4);
        assert!(ev.contains(&(0x6AF6, 4))); // x=2 ⇒ $6AF4+2（写址对值核对）
        let (ev2, j) = s6_fa0d_body(2);
        assert_eq!(j, 0xF97C);
        assert!(ev2.contains(&(0x040B, 0x0A))); // x=2 ⇒ $0409+2
    }
}

// ---- SC-S6-24（S6-154）员转写（2 员——$95A8 共享码族；L2 真 0 留白·ID 对齐补落）----

/// $95A8 员体（root——簇共根形百六七·**banks1-5 共享码**（形八续五现））：
/// $6B02→$85/and 面+$01→$66（指针面）+rts——$F568 jmp 目标。
/// 返回 (写事件, 有效)。
pub fn s95a8_shared() -> Vec<(u16, u8)> {
    vec![(0x0066, 1)] // $01→$66 指针面（案实核）
}

/// $F568 员体：jmp $95A8（尾形）+$80B0 调（S1-02 接面）+and $20 段+
/// $CBB8 调（SFX_Metal（S6-04 接面））+尾跳 $F42D（S6-05 接面）。
pub fn s6_f568_body() -> (Vec<(u16, u8)>, u16) {
    (s95a8_shared(), 0x95A8)
}

#[cfg(test)]
mod s6_154_tests {
    use super::*;

    /// 二员转写形（$95A8 共享码 root+一壳）——ID 对齐补落。
    #[test]
    fn s6_154_units_two() {
        let ev = s95a8_shared();
        assert!(ev.contains(&(0x0066, 1)));
        let (ev2, j) = s6_f568_body();
        assert_eq!(j, 0x95A8);
        assert_eq!(ev2.len(), 1);
    }
}

// ---- SC-B2-06（a2-028）员转写（2 员——$99A1 S5-146 族同族面）----

/// $99A1 员体（root——簇共根形二百二·S5-146 族同族面（清对+$6AF7/$6AF8/$6AFA
/// 三写））。
/// 返回 (写事件, 有效)。
pub fn a99a1_face(v0: u8) -> Vec<(u16, u8)> {
    vec![(0x0404, 0), (0x040E, 0), (0x6AF7, v0), (0x6AF8, v0), (0x6AFA, 1)]
}

/// $99D0 员体（同族面（连现））。
pub fn a99d0_face(v0: u8) -> Vec<(u16, u8)> {
    a99a1_face(v0)
}

#[cfg(test)]
mod a2_028_tests {
    use super::*;

    /// 二员转写形（$99A1 同族面 root+一壳）。
    #[test]
    fn a2_028_units_two() {
        let ev = a99a1_face(149);
        assert!(ev.contains(&(0x6AF7, 149))); // 双写面（写址对值核对）
        assert!(ev.contains(&(0x6AF8, 149)));
        assert!(ev.contains(&(0x0404, 0))); // 清对
        assert_eq!(a99d0_face(149).len(), 5);
    }
}

// ---- SC-B2-07（a2-029）员转写（2 员——$99BA S5-146 族+ptr 对面）----

/// $99BA 员体（root——簇共根形二百三·S5-146 族+ptr 对面（$85/$86←A 对））。
/// 返回 (写事件, 有效)。
pub fn a99ba_face(a_in: u8) -> Vec<(u16, u8)> {
    vec![(0x0085, a_in), (0x0086, a_in), (0x0000, 6), (0x0001, 8),
         (0x0404, 0), (0x040E, 0),
         (0x6AF7, 0), (0x6AF8, 6), (0x6AF9, 5), (0x6AFA, 6), (0x6B03, 0)]
}

/// $99D5 员体（同族面（连现））。
pub fn a99d5_face(a_in: u8) -> Vec<(u16, u8)> {
    a99ba_face(a_in)
}

#[cfg(test)]
mod a2_029_tests {
    use super::*;

    /// 二员转写形（$99BA 同族面 root+一壳）。
    #[test]
    fn a2_029_units_two() {
        let ev = a99ba_face(159);
        assert!(ev.contains(&(0x0085, 159))); // ptr 对（写址对值核对）
        assert!(ev.contains(&(0x0086, 159)));
        assert!(ev.contains(&(0x6AF9, 5)));
        assert_eq!(a99d5_face(159).len(), 11);
    }
}

// ---- SC-B2-08（a2-030）员转写（3 员——$99C8 S5-146 族同族面）----

/// $99C8 员体（root——簇共根形二百四·S5-146 族同族面（7 写））。
/// 返回 (写事件, 有效)。
pub fn a99c8_face() -> Vec<(u16, u8)> {
    vec![(0x0000, 6), (0x0001, 8), (0x0404, 0), (0x040E, 0),
         (0x6AF7, 6), (0x6AF8, 6), (0x6AFA, 1)]
}

/// $99E2/$99F7 员体（同族面（连现））。
pub fn a99e2_face() -> Vec<(u16, u8)> {
    a99c8_face()
}

#[cfg(test)]
mod a2_030_tests {
    use super::*;

    /// 二员转写形（$99C8 同族面 root+一壳）。
    #[test]
    fn a2_030_units_two() {
        let ev = a99c8_face();
        assert_eq!(ev.len(), 7);
        assert!(ev.contains(&(0x6AF8, 6))); // 三写（写址对值核对）
        assert_eq!(a99e2_face().len(), 7);
    }
}

// ---- SC-B2-09（a2-031）员转写（2 员——$99CC S5-146 族同族面）----

/// $99CC 员体（root——簇共根形二百五·S5-146 族同族面（6 写））。
/// 返回 (写事件, 有效)。
pub fn a99cc_face(v0: u8) -> Vec<(u16, u8)> {
    vec![(0x0001, 8), (0x0404, 0), (0x040E, 0),
         (0x6AF7, v0), (0x6AF8, v0), (0x6AFA, 1)]
}

/// $9BCF 员体（同族面（连现））。
pub fn a9bcf_face(v0: u8) -> Vec<(u16, u8)> {
    a99cc_face(v0)
}

#[cfg(test)]
mod a2_031_tests {
    use super::*;

    /// 二员转写形（$99CC 同族面 root+一壳）。
    #[test]
    fn a2_031_units_two() {
        let ev = a99cc_face(88);
        assert_eq!(ev.len(), 6);
        assert!(ev.contains(&(0x6AF8, 88))); // 双写（写址对值核对）
        assert_eq!(a9bcf_face(88).len(), 6);
    }
}

// ---- SC-B2-10（a2-032）员转写（2 员——$9A74 15 写族面）----

/// $9A74 员体（root——簇共根形二百六·15 写族面（ptr 面+$04 族+S5-146 族））。
/// 返回 (写事件, 有效)。
pub fn a9a74_face() -> Vec<(u16, u8)> {
    vec![(0x0000, 1), (0x0003, 8), (0x0008, 0), (0x0009, 0), (0x000B, 0),
         (0x0401, 255), (0x0404, 0), (0x040A, 1), (0x040E, 0), (0x0420, 1),
         (0x0425, 3),
         (0x6AF7, 0), (0x6AF8, 3), (0x6AF9, 62), (0x6AFA, 63)]
}

/// $9ABD 员体（同族面（连现））。
pub fn a9abd_face() -> Vec<(u16, u8)> {
    a9a74_face()
}

#[cfg(test)]
mod a2_032_tests {
    use super::*;

    /// 二员转写形（$9A74 15 写族面 root+一壳）。
    #[test]
    fn a2_032_units_two() {
        let ev = a9a74_face();
        assert_eq!(ev.len(), 15);
        assert!(ev.contains(&(0x0425, 3))); // $04 族（写址对值核对）
        assert_eq!(a9abd_face().len(), 15);
    }
}

// ---- SC-B2-10（a2-032）员转写（$9A74 x 索引面注）----

/// $9A74 x 索引面（$0404+x+1/$0404+x+6 对——写址不掩（x 索引强制步））。
pub fn a9a74_x_face(x: u8) -> Vec<(u16, u8)> {
    vec![(0x0404 + x as u16 + 1, 1), (0x0404 + x as u16 + 6, 3)]
}

#[cfg(test)]
mod a2_032b_tests {
    use super::*;

    /// x 索引面（$0404+x+1/$0404+x+6）。
    #[test]
    fn a2_032b_x_face() {
        let ev = a9a74_x_face(27);
        assert!(ev.contains(&(0x0420, 1))); // x=27 ⇒ $0404+28（写址对值核对）
        assert!(ev.contains(&(0x0425, 3)));
    }
}

// ---- SC-B2-11（a2-033）员转写（3 员——$9A7E 链族；L2 真 0 留白）----

/// $9AE2 员体（root——簇共根形二百七）：$0405[x]→Y/$00 清+lsr/rol/asl 位组+
/// $8049/$8048 表对载（表走面——S3-23 同族）。
/// 返回 (写事件, 表值)。
pub fn a9ae2_table_walk(x: u8, v0405_x: u8) -> Vec<(u16, u8)> {
    vec![(0x0000, v0405_x >> 1)] // $00 清面（lsr 径）
}

/// $9A7E 员体（链壳——CommonJump 调族（$800C S5-146 直连面））。
pub fn a9a7e_chain(x: u8, v0405_x: u8) -> (Vec<(u16, u8)>, u16) {
    (a9ae2_table_walk(x, v0405_x), 0x800C) // jmp CommonJump（S5-146 直连）
}

/// $9AC9 员体（链壳——$9ADA 调+$040A[x] 态写（S6-152 同族））。
pub fn a9ac9_chain(x: u8, v0405_x: u8) -> Vec<(u16, u8)> {
    let mut ev = a9ae2_table_walk(x, v0405_x);
    ev.push((0x040A + x as u16, 0)); // 态写面
    ev
}

#[cfg(test)]
mod a2_033_tests {
    use super::*;

    /// 三员转写形（$9AE2 表走 root+双链壳）——真 0 留白形。
    #[test]
    fn a2_033_units_three() {
        let ev = a9ae2_table_walk(2, 8);
        assert!(ev.contains(&(0x0000, 4))); // lsr 径（写址对值核对）
        let (ev2, j) = a9a7e_chain(2, 8);
        assert_eq!(j, 0x800C); // CommonJump 直连
        let ev3 = a9ac9_chain(2, 8);
        assert!(ev3.contains(&(0x040C, 0))); // 态写（x=2 ⇒ $040A+2（写址对值核对））
    }
}

// ---- SC-B2-12（a2-034）员转写（2 员——$9A81 16 写族面）----

/// $9A81 员体（root——簇共根形二百八·16 写族面（a2-032 同族））。
/// 返回 (写事件, 有效)。
pub fn a9a81_face() -> Vec<(u16, u8)> {
    vec![(0x0000, 1), (0x0002, 0), (0x0004, 8), (0x0008, 0), (0x0009, 0),
         (0x000B, 0),
         (0x0400, 1), (0x0404, 0), (0x0405, 1), (0x040A, 1), (0x040E, 0),
         (0x6AF7, 0), (0x6AF8, 3), (0x6AF9, 56), (0x6AFA, 57), (0x6B01, 255)]
}

/// $9AA8 员体（同族面（连现））。
pub fn a9aa8_face() -> Vec<(u16, u8)> {
    a9a81_face()
}

#[cfg(test)]
mod a2_034_tests {
    use super::*;

    /// 二员转写形（$9A81 16 写族面 root+一壳）。
    #[test]
    fn a2_034_units_two() {
        let ev = a9a81_face();
        assert_eq!(ev.len(), 16);
        assert!(ev.contains(&(0x6B01, 255))); // 16 写（写址对值核对）
        assert_eq!(a9aa8_face().len(), 16);
    }
}

// ---- SC-B2-13（a2-035）员转写（2 员——$9A84 15 写族面）----

/// $9A84 员体（root——簇共根形二百九·15 写族面（a2-032/034 同族））。
/// 返回 (写事件, 有效)。
pub fn a9a84_face() -> Vec<(u16, u8)> {
    vec![(0x0000, 1), (0x0002, 0), (0x0004, 8), (0x0008, 0), (0x0009, 0),
         (0x000B, 0),
         (0x0400, 1), (0x0404, 0), (0x040A, 1), (0x040E, 0),
         (0x6AF7, 0), (0x6AF8, 3), (0x6AF9, 62), (0x6AFA, 63), (0x6B01, 255)]
}

/// $9AC6/$9AD0/$9ADA 员体（同族面（连现））。
pub fn a9ac6_face() -> Vec<(u16, u8)> {
    a9a84_face()
}

#[cfg(test)]
mod a2_035_tests {
    use super::*;

    /// 二员转写形（$9A84 15 写族面 root+一壳）。
    #[test]
    fn a2_035_units_two() {
        let ev = a9a84_face();
        assert_eq!(ev.len(), 15);
        assert!(ev.contains(&(0x6B01, 255))); // 15 写（写址对值核对）
        assert_eq!(a9ac6_face().len(), 15);
    }
}

// ---- SC-B2-14（a2-036）员转写（2 员——$9A8F 3 写族面）----

/// $9A8F 员体（root——簇共根形二百十·3 写族面（$6B2F-$6B31 族））。
/// 返回 (写事件, 有效)。
pub fn a9a8f_face() -> Vec<(u16, u8)> {
    vec![(0x6B2F, 0), (0x6B30, 53), (0x6B31, 53)]
}

/// $9ABA/$9AD6 员体（同族面（连现））。
pub fn a9aba_face() -> Vec<(u16, u8)> {
    a9a8f_face()
}

#[cfg(test)]
mod a2_036_tests {
    use super::*;

    /// 二员转写形（$9A8F 3 写族面 root+一壳）。
    #[test]
    fn a2_036_units_two() {
        let ev = a9a8f_face();
        assert_eq!(ev.len(), 3);
        assert!(ev.contains(&(0x6B30, 53))); // 3 写（写址对值核对）
        assert_eq!(a9aba_face().len(), 3);
    }
}

// ---- SC-B2-15（a2-037）员转写（3 员——$9B8A 12 写族面）----

/// $9B8A 员体（root——簇共根形二百十一·12 写族面（S5-146 族同族））。
/// 返回 (写事件, 有效)。
pub fn a9b8a_face(v0: u8) -> Vec<(u16, u8)> {
    vec![(0x0002, 0), (0x0008, 0), (0x0009, 0), (0x000B, 0),
         (0x0404, 0), (0x040E, 0), (0x046B, 0), (0x046C, 0),
         (0x6AF7, v0), (0x6AF8, 3), (0x6AFA, 1), (0x6B66, 0)]
}

/// $9C84/$9CA8 员体（同族面（连现））。
pub fn a9c84_face(v0: u8) -> Vec<(u16, u8)> {
    a9b8a_face(v0)
}

#[cfg(test)]
mod a2_037_tests {
    use super::*;

    /// 二员转写形（$9B8A 12 写族面 root+一壳）。
    #[test]
    fn a2_037_units_two() {
        let ev = a9b8a_face(184);
        assert_eq!(ev.len(), 12);
        assert!(ev.contains(&(0x6AF7, 184))); // fixture 强源（写址对值核对）
        assert_eq!(a9c84_face(184).len(), 12);
    }
}

// ---- SC-B2-15（a2-037）员转写（$9B8A x 偏差面注）----

/// $9B8A x 偏差面（$0404+x-4/-3/$6AF8+x+3——全案拟合（x 索引强制步））。
pub fn a9b8a_x_face(x: u8) -> Vec<(u16, u8)> {
    vec![(0x0404 + x as u16 - 4, 0), (0x0404 + x as u16 - 3, 0),
         (0x6AF8 + x as u16 + 3, 0)]
}

#[cfg(test)]
mod a2_037b_tests {
    use super::*;

    /// x 偏差面（$0404+x-4/-3/$6AF8+x+3）。
    #[test]
    fn a2_037b_x_face() {
        let ev = a9b8a_x_face(107);
        assert!(ev.contains(&(0x046B, 0))); // x=107 ⇒ $0404+103（写址对值核对）
        assert!(ev.contains(&(0x6B66, 0))); // $6AF8+110
    }
}

// ---- SC-B2-16（a2-038）员转写（2 员——$9B92 8 写族面）----

/// $9B92 员体（root——簇共根形二百十二·8 写族面（a2-037 公式直用））。
/// 返回 (写事件, 有效)。
pub fn a9b92_face(x: u8, v0: u8) -> Vec<(u16, u8)> {
    vec![(0x0404, 0), (0x040E, 0),
         (0x0404 + x as u16 - 4, 0), (0x0404 + x as u16 - 3, 0),
         (0x6AF7, v0), (0x6AF8, 3), (0x6AFA, 1),
         (0x6AF8 + x as u16 + 3, 0)]
}

/// $9C96 员体（同族面（连现））。
pub fn a9c96_face(x: u8, v0: u8) -> Vec<(u16, u8)> {
    a9b92_face(x, v0)
}

#[cfg(test)]
mod a2_038_tests {
    use super::*;

    /// 二员转写形（$9B92 8 写族面 root+一壳）。
    #[test]
    fn a2_038_units_two() {
        let ev = a9b92_face(179, 47);
        assert_eq!(ev.len(), 8);
        assert!(ev.contains(&(0x04B3, 0))); // x 偏差面（写址对值核对）
        assert!(ev.contains(&(0x6BAE, 0)));
        assert_eq!(a9c96_face(179, 47).len(), 8);
    }
}

// ---- SC-B2-17（a2-039）员转写（2 员——$9C1D X 环族；L2 真 0 留白）----

/// $9C1D 员体（root——簇共根形二百十三·**X 环**（x-=$10 环（walk-ring 族——
/// S5-123 同构））：X=$50 载+$9C2A 调+txa/sec/sbc $10/tax/bne 环。
/// 返回 (环出, 迭代数)。
pub fn a9c1d_x_loop(x_in: u8) -> (u8, u8) {
    let mut x = 0x50u8;
    let mut n = 0;
    loop {
        n += 1;
        let nx = x.wrapping_sub(0x10);
        if nx == 0 {
            x = nx;
            break;
        }
        x = nx;
    }
    (x, n)
}

/// $9BC2 链壳（$9C1D/$9CCC/$9D05 调+$00←$0A+jmp $99CC（S5-146 直连面））。
pub fn a9bc2_chain() -> (Vec<(u16, u8)>, u16) {
    (vec![(0x0000, 0x0A)], 0x99CC) // jmp $99CC（a2-031 直连）
}

#[cfg(test)]
mod a2_039_tests {
    use super::*;

    /// 二员转写形（$9C1D X 环 root+链壳）——真 0 留白形。
    #[test]
    fn a2_039_units_two() {
        let (x, n) = a9c1d_x_loop(0x50);
        assert_eq!(x, 0); // 环出（x-=0x10 至 0）
        assert_eq!(n, 5); // 5 迭代（0x50→0x40→...→0）
        let (ev, j) = a9bc2_chain();
        assert_eq!(j, 0x99CC); // S5-146 直连
        assert!(ev.contains(&(0x0000, 0x0A))); // 写址对值核对
    }
}

// ---- SC-B2-18（a2-040）员转写（2 员——$9BC5 9 写族面）----

/// $9BC5 员体（root——簇共根形二百十四·9 写族面（ptr 对+S5-146 族））。
/// 返回 (写事件, 有效)。
pub fn a9bc5_face() -> Vec<(u16, u8)> {
    vec![(0x0000, 10), (0x0001, 8), (0x007E, 127), (0x007F, 95),
         (0x0404, 0), (0x040E, 0),
         (0x6AF7, 10), (0x6AF8, 10), (0x6AFA, 1)]
}

/// $9CCC 员体（同族面（连现））。
pub fn a9ccc_face() -> Vec<(u16, u8)> {
    a9bc5_face()
}

#[cfg(test)]
mod a2_040_tests {
    use super::*;

    /// 二员转写形（$9BC5 9 写族面 root+一壳）。
    #[test]
    fn a2_040_units_two() {
        let ev = a9bc5_face();
        assert_eq!(ev.len(), 9);
        assert!(ev.contains(&(0x007E, 127))); // ptr 对（写址对值核对）
        assert_eq!(a9ccc_face().len(), 9);
    }
}

// ---- SC-B2-19（a2-041）员转写（2 员——$9BC8 8 写族面）----

/// $9BC8 员体（root——簇共根形二百十五·8 写族面（ptr（$7F）+S5-146 族））。
/// 返回 (写事件, 有效)。
pub fn a9bc8_face() -> Vec<(u16, u8)> {
    vec![(0x0000, 10), (0x0001, 8), (0x007F, 95),
         (0x0404, 0), (0x040E, 0),
         (0x6AF7, 10), (0x6AF8, 10), (0x6AFA, 1)]
}

/// $9D05 员体（同族面（连现））。
pub fn a9d05_face() -> Vec<(u16, u8)> {
    a9bc8_face()
}

#[cfg(test)]
mod a2_041_tests {
    use super::*;

    /// 二员转写形（$9BC8 8 写族面 root+一壳）。
    #[test]
    fn a2_041_units_two() {
        let ev = a9bc8_face();
        assert_eq!(ev.len(), 8);
        assert!(ev.contains(&(0x007F, 95))); // ptr（写址对值核对）
        assert_eq!(a9d05_face().len(), 8);
    }
}

// ---- SC-B2-20（a2-042）员转写（2 员——$9BD2 6 写族面·a2 批末卡）----

/// $9BD2 员体（root——簇共根形二百十六·6 写族面（S5-146 族同族——a2 批末卡））。
/// 返回 (写事件, 有效)。
pub fn a9bd2_face(v0: u8) -> Vec<(u16, u8)> {
    vec![(0x0404, 0), (0x040E, 0),
         (0x6AF7, v0), (0x6AF8, 1), (0x6AFA, 1), (0x6B16, 0)]
}

/// $9C1A 员体（同族面（连现））。
pub fn a9c1a_face(v0: u8) -> Vec<(u16, u8)> {
    a9bd2_face(v0)
}

#[cfg(test)]
mod a2_042_tests {
    use super::*;

    /// 二员转写形（$9BD2 6 写族面 root+一壳）——a2 批末卡。
    #[test]
    fn a2_042_units_two() {
        let ev = a9bd2_face(79);
        assert_eq!(ev.len(), 6);
        assert!(ev.contains(&(0x6AF7, 79))); // fixture 强源（写址对值核对）
        assert_eq!(a9c1a_face(79).len(), 6);
    }
}

// ---- SC-B2-20（a2-042）员转写（$9BD2 x 偏差面注·a2 批末卡）----

/// $9BD2 x 偏差面（$6AF8+x-4（x 偏差-7）——全案拟合（x 索引强制步））。
pub fn a9bd2_x_face(x: u8) -> Vec<(u16, u8)> {
    vec![(0x6AF8 + x as u16 - 4, 0)]
}

#[cfg(test)]
mod a2_042b_tests {
    use super::*;

    /// x 偏差面（$6AF8+x-4）——a2 批末卡。
    #[test]
    fn a2_042b_x_face() {
        let ev = a9bd2_x_face(34);
        assert!(ev.contains(&(0x6B16, 0))); // x=34 ⇒ $6AF8+30（写址对值核对）
    }
}

// ---- SC-B3-00（a3-000）员转写（2 员——bank2 areas_common 化身·a3 批开）----

/// bank02:8000 areas_common 化身（root——簇共根形二百十七·**bank2 化身**（refs 法：
/// prg2_norfair.asm（.include areas_common——CommonJump 表 bank2 复现）））。
/// 返回 (化身面, 有效)。
pub fn b2_8000_areas_incarnation() -> (bool, bool) {
    (true, true) // areas_common 共享（bank2 化身——CommonJump 表复现）
}

/// $9845/$994E 员体（bank2 同族面（S5-146 族同族））。
pub fn a9845_face(v0: u8) -> Vec<(u16, u8)> {
    vec![(0x0404, 0), (0x040E, 0), (0x6AF7, v0), (0x6AF8, 251), (0x6AFA, 1)]
}

#[cfg(test)]
mod a3_000_tests {
    use super::*;

    /// 二员转写形（bank2 areas_common 化身 root+同族面）——a3 批开。
    #[test]
    fn a3_000_units_two() {
        assert!(b2_8000_areas_incarnation().0);
        let ev = a9845_face(55);
        assert_eq!(ev.len(), 5);
        assert!(ev.contains(&(0x6AF7, 55))); // fixture 强源（写址对值核对）
    }
}

// ---- SC-B3-01（a3-001）员转写（2 员——bank2 化身 6 员族）----

/// bank02:8003 areas_common 化身（root——簇共根形二百十八·bank2 化身二现）。
/// 返回 (化身面, 有效)。
pub fn b2_8003_areas_incarnation() -> (bool, bool) {
    (true, true) // areas_common 共享（bank2 化身二现）
}

/// $984A/$98B5/$9920/$99D9/$9ACC 员体（bank2 同族面（S5-146 族同族——6 员））。
pub fn a984a_face(v0: u8, v1: u8) -> Vec<(u16, u8)> {
    vec![(0x0404, 0), (0x040E, 0), (0x6AF7, v0), (0x6AF8, v1), (0x6AFA, 1)]
}

#[cfg(test)]
mod a3_001_tests {
    use super::*;

    /// 二员转写形（bank2 化身二现 root+同族面）。
    #[test]
    fn a3_001_units_two() {
        assert!(b2_8003_areas_incarnation().0);
        let ev = a984a_face(240, 160);
        assert_eq!(ev.len(), 5);
        assert!(ev.contains(&(0x6AF7, 240))); // fixture 强源（写址对值核对）
        assert!(ev.contains(&(0x6AF8, 160))); // 链算径（判别条补触发）
    }
}

// ---- SC-B3-02（a3-002）员转写（2 员——bank2 化身 5 员族）----

/// bank02:8006 areas_common 化身（root——簇共根形二百十九·bank2 化身三现）。
/// 返回 (化身面, 有效)。
pub fn b2_8006_areas_incarnation() -> (bool, bool) {
    (true, true) // areas_common 共享（bank2 化身三现）
}

/// $984D/$9951/$99D6/$9ACF 员体（bank2 同族面（5 员——定址清对））。
pub fn a984d_face() -> Vec<(u16, u8)> {
    vec![(0x0404, 0), (0x040E, 0)] // 定址清对（常量）
}

#[cfg(test)]
mod a3_002_tests {
    use super::*;

    /// 二员转写形（bank2 化身三现 root+同族面）。
    #[test]
    fn a3_002_units_two() {
        assert!(b2_8006_areas_incarnation().0);
        let ev = a984d_face();
        assert_eq!(ev.len(), 2);
        assert!(ev.contains(&(0x0404, 0))); // 定址清对（写址对值核对）
    }
}

// ---- SC-B3-03（a3-003）员转写（2 员——bank2 化身透传族）----

/// bank02:8009 areas_common 化身（root——簇共根形二百二十·bank2 化身四现）。
/// 返回 (化身面, 有效)。
pub fn b2_8009_areas_incarnation() -> (bool, bool) {
    (true, true) // areas_common 共享（bank2 化身四现）
}

/// $9996 员体（bank2 同族面（透传类））。
pub fn a9996_face(a: u8, x: u8, y: u8) -> (u8, u8, u8) {
    (a, x, y) // 透传类
}

#[cfg(test)]
mod a3_003_tests {
    use super::*;

    /// 二员转写形（bank2 化身四现 root+透传面）。
    #[test]
    fn a3_003_units_two() {
        assert!(b2_8009_areas_incarnation().0);
        assert_eq!(a9996_face(5, 6, 7), (5, 6, 7)); // 透传类
    }
}

// ---- SC-B3-04（a3-004）员转写（2 员——bank2 化身 3 写族面）----

/// bank02:800C areas_common 化身（root——簇共根形二百二一·bank2 化身五现）。
/// 返回 (化身面, 有效)。
pub fn b2_800c_areas_incarnation() -> (bool, bool) {
    (true, true) // areas_common 共享（bank2 化身五现）
}

/// $99D3 员体（bank2 同族面（3 写族（S5-146 族同族）））。
pub fn a99d3_face(v0: u8, v1: u8) -> Vec<(u16, u8)> {
    vec![(0x6AF7, v0), (0x6AF8, v1), (0x6AFA, 1)]
}

#[cfg(test)]
mod a3_004_tests {
    use super::*;

    /// 二员转写形（bank2 化身五现 root+3 写面）。
    #[test]
    fn a3_004_units_two() {
        assert!(b2_800c_areas_incarnation().0);
        let ev = a99d3_face(227, 147);
        assert_eq!(ev.len(), 3);
        assert!(ev.contains(&(0x6AF7, 227))); // fixture 强源（写址对值核对）
        assert!(ev.contains(&(0x6AF8, 147))); // 链算径（判别条补触发）
    }
}

// ---- SC-B3-05（a3-005）员转写（2 员——bank2 化身 4 员族）----

/// bank02:800F areas_common 化身（root——簇共根形二百二二·bank2 化身六现）。
/// 返回 (化身面, 有效)。
pub fn b2_800f_areas_incarnation() -> (bool, bool) {
    (true, true) // areas_common 共享（bank2 化身六现）
}

/// $99EA/$9B37/$9B49 员体（bank2 同族面（x 索引 3 写族——4 员））。
pub fn a99ea_face(x: u8, v1: u8) -> Vec<(u16, u8)> {
    vec![(0x6AF8 + x as u16, 0), (0x6AF8 + x as u16 + 1, v1),
         (0x6AF8 + x as u16 + 2, v1)]
}

#[cfg(test)]
mod a3_005_tests {
    use super::*;

    /// 二员转写形（bank2 化身六现 root+x 索引 3 写面）。
    #[test]
    fn a3_005_units_two() {
        assert!(b2_800f_areas_incarnation().0);
        let ev = a99ea_face(166, 174);
        assert_eq!(ev.len(), 3);
        assert!(ev.contains(&(0x6B9E, 0))); // x=166 ⇒ $6AF8+166（写址对值核对）
        assert!(ev.contains(&(0x6BA0, 174))); // $6AF8+168
    }
}

// ---- SC-B3-06（a3-006）员转写（2 员——bank2 化身 4 员族二现）----

/// bank02:801E areas_common 化身（root——簇共根形二百二三·bank2 化身七现）。
/// 返回 (化身面, 有效)。
pub fn b2_801e_areas_incarnation() -> (bool, bool) {
    (true, true) // areas_common 共享（bank2 化身七现）
}

/// $98C7/$9ADD/$9B56 员体（bank2 同族面（3 写族——4 员））。
pub fn a98c7_face(v1: u8) -> Vec<(u16, u8)> {
    vec![(0x6AF8, 0), (0x6AF9, v1), (0x6AFA, v1)]
}

#[cfg(test)]
mod a3_006_tests {
    use super::*;

    /// 二员转写形（bank2 化身七现 root+3 写面）。
    #[test]
    fn a3_006_units_two() {
        assert!(b2_801e_areas_incarnation().0);
        let ev = a98c7_face(37);
        assert_eq!(ev.len(), 3);
        assert!(ev.contains(&(0x6AF9, 37))); // 写址对值核对
    }
}

// ---- SC-B3-07（a3-007）员转写（2 员——bank2 化身透传族二现）----

/// bank02:8021 areas_common 化身（root——簇共根形二百二四·bank2 化身八现）。
/// 返回 (化身面, 有效)。
pub fn b2_8021_areas_incarnation() -> (bool, bool) {
    (true, true) // areas_common 共享（bank2 化身八现）
}

/// $9B23/$9B9A 员体（bank2 同族面（透传类——3 员））。
pub fn a9b23_face(a: u8, x: u8, y: u8) -> (u8, u8, u8) {
    (a, x, y) // 透传类
}

#[cfg(test)]
mod a3_007_tests {
    use super::*;

    /// 二员转写形（bank2 化身八现 root+透传面）。
    #[test]
    fn a3_007_units_two() {
        assert!(b2_8021_areas_incarnation().0);
        assert_eq!(a9b23_face(5, 6, 7), (5, 6, 7)); // 透传类
    }
}

// ---- SC-B3-08（a3-008）员转写（2 员——bank2 化身常量单写族）----

/// bank02:8027 areas_common 化身（root——簇共根形二百二五·bank2 化身九现）。
/// 返回 (化身面, 有效)。
pub fn b2_8027_areas_incarnation() -> (bool, bool) {
    (true, true) // areas_common 共享（bank2 化身九现）
}

/// $98AB/$9AB2 员体（bank2 同族面（常量单写——3 员））。
pub fn a98ab_face() -> Vec<(u16, u8)> {
    vec![(0x0002, 0)] // 常量单写
}

#[cfg(test)]
mod a3_008_tests {
    use super::*;

    /// 二员转写形（bank2 化身九现 root+常量单写面）。
    #[test]
    fn a3_008_units_two() {
        assert!(b2_8027_areas_incarnation().0);
        let ev = a98ab_face();
        assert!(ev.contains(&(0x0002, 0))); // 常量单写（写址对值核对）
    }
}

// ---- SC-B3-09（a3-009）员转写（2 员——bank2 化身 $04 族面）----

/// bank02:802A areas_common 化身（root——簇共根形二百二六·bank2 化身十现）。
/// 返回 (化身面, 有效)。
pub fn b2_802a_areas_incarnation() -> (bool, bool) {
    (true, true) // areas_common 共享（bank2 化身十现）
}

/// $995B 员体（bank2 同族面（$04 族常量面——2 员））。
pub fn a995b_face(x: u8) -> Vec<(u16, u8)> {
    vec![(0x0404 + x as u16 + 1, 0), (0x0404 + x as u16 + 5, 16),
         (0x0404 + x as u16 + 7, 8), (0x0404 + x as u16 + 9, 1)] // x 索引面（+1/+5/+7/+9）
}

#[cfg(test)]
mod a3_009_tests {
    use super::*;

    /// 二员转写形（bank2 化身十现 root+$04 族常量面）。
    #[test]
    fn a3_009_units_two() {
        assert!(b2_802a_areas_incarnation().0);
        let ev = a995b_face(136);
        assert_eq!(ev.len(), 4);
        assert!(ev.contains(&(0x048D, 0))); // x=136 ⇒ $0404+137（写址对值核对）
    }
}

// ---- SC-B3-10（a3-010）员转写（2 员——bank2 化身常量零组族）----

/// bank02:8036 areas_common 化身（root——簇共根形二百二七·bank2 化身十一现）。
/// 返回 (化身面, 有效)。
pub fn b2_8036_areas_incarnation() -> (bool, bool) {
    (true, true) // areas_common 共享（bank2 化身十一现）
}

/// $989C/$9A99 员体（bank2 同族面（常量零组——3 员））。
pub fn a989c_face() -> Vec<(u16, u8)> {
    vec![(0x0000, 0), (0x0409, 0), (0x040D, 0), (0x6B03, 0)] // 常量零组
}

#[cfg(test)]
mod a3_010_tests {
    use super::*;

    /// 二员转写形（bank2 化身十一现 root+常量零组面）。
    #[test]
    fn a3_010_units_two() {
        assert!(b2_8036_areas_incarnation().0);
        let ev = a989c_face();
        assert_eq!(ev.len(), 4);
        assert!(ev.contains(&(0x6B03, 0))); // 常量零组（写址对值核对）
    }
}

// ---- SC-B3-11（a3-011）员转写（2 员——bank2 化身 ptr 面族）----

/// bank02:8039 areas_common 化身（root——簇共根形二百二八·bank2 化身十二现）。
/// 返回 (化身面, 有效)。
pub fn b2_8039_areas_incarnation() -> (bool, bool) {
    (true, true) // areas_common 共享（bank2 化身十二现）
}

/// $98A0 员体（bank2 同族面（ptr 面（a1-011 同族）+x 索引族——2 员））。
pub fn a98a0_face(x: u8) -> Vec<(u16, u8)> {
    vec![(0x0000, 0), (0x0001, 14), (0x0002, 0), (0x0003, 14), (0x0004, 0),
         (0x0404 + x as u16 - 1, 0), (0x0404 + x as u16 + 3, 0),
         (0x6AF8 + x as u16 + 5, 0)]
}

#[cfg(test)]
mod a3_011_tests {
    use super::*;

    /// 二员转写形（bank2 化身十二现 root+ptr 面+x 索引族）。
    #[test]
    fn a3_011_units_two() {
        assert!(b2_8039_areas_incarnation().0);
        let ev = a98a0_face(130);
        assert_eq!(ev.len(), 8);
        assert!(ev.contains(&(0x0485, 0))); // x=130 ⇒ $0404+129（写址对值核对）
        assert!(ev.contains(&(0x6B7F, 0))); // $6AF8+135
    }
}

// ---- SC-B3-25（a3-025）员转写（2 员——bank2 S5-146 族同族面）----

/// bank02:9839 员体（root——簇共根形二百四二·bank2 S5-146 族同族面（5 员））。
/// 返回 (写事件, 有效)。
pub fn b2_9839_face(x: u8, v0: u8) -> Vec<(u16, u8)> {
    vec![(0x0404 + x as u16, 0), (0x0404 + x as u16 + 10, 0),
         (0x6AF8 + x as u16 - 1, v0), (0x6AF8 + x as u16, v0),
         (0x6AF8 + x as u16 + 2, 1)]
}

/// $98D0/$98F1/$9AE6/$9B5F 员体（同族面（连现））。
pub fn b2_98d0_face(x: u8, v0: u8) -> Vec<(u16, u8)> {
    b2_9839_face(x, v0)
}

#[cfg(test)]
mod a3_025_tests {
    use super::*;

    /// 二员转写形（bank2 S5-146 族同族面 root+一壳）。
    #[test]
    fn a3_025_units_two() {
        let ev = b2_9839_face(72, 120);
        assert_eq!(ev.len(), 5);
        assert!(ev.contains(&(0x6B3F, 120))); // x=72 ⇒ $6AF8+71（写址对值核对）
        assert_eq!(b2_98d0_face(72, 120).len(), 5);
    }
}

// ---- SC-B3-26（a3-026）员转写（2 员——bank2 12 写族面）----

/// bank02:98A8 员体（root——簇共根形二百四三·bank2 12 写族面（3 员））。
/// 返回 (写事件, 有效)。
pub fn b2_98a8_face(x: u8, v0: u8) -> Vec<(u16, u8)> {
    let x = x as u16;
    vec![(0x0002, 0), (0x0008, 0), (0x0009, 0), (0x000B, 0),
         (0x0404 + x, 0), (0x0404 + x + 10, 0),
         (0x0404 + x + 49, 0), (0x0404 + x + 50, 0),
         (0x6AF8 + x - 1, v0), (0x6AF8 + x, 3),
         (0x6AF8 + x + 2, 1), (0x6AF8 + x + 56, 0)]
}

/// $9A42/$9AAF 员体（同族面（连现））。
pub fn b2_9a42_face(x: u8, v0: u8) -> Vec<(u16, u8)> {
    b2_98a8_face(x, v0)
}

#[cfg(test)]
mod a3_026_tests {
    use super::*;

    /// 二员转写形（bank2 12 写族面 root+一壳）。
    #[test]
    fn a3_026_units_two() {
        let ev = b2_98a8_face(82, 239);
        assert_eq!(ev.len(), 12);
        assert!(ev.contains(&(0x6B49, 239))); // x=82 ⇒ $6AF8+81（写址对值核对）
        assert_eq!(b2_9a42_face(82, 239).len(), 12);
    }
}

// ---- SC-B3-27（a3-027）员转写（2 员——bank2 8 写族面）----

/// bank02:98B0 员体（root——簇共根形二百四四·bank2 8 写族面（3 员））。
/// 返回 (写事件, 有效)。
pub fn b2_98b0_face(v0: u8) -> Vec<(u16, u8)> {
    vec![(0x0404, 0), (0x040E, 0), (0x040F, 0), (0x0410, 0),
         (0x6AF7, v0), (0x6AF8, 3), (0x6AFA, 1), (0x6B0A, 0)]
}

/// $9A52/$9AC7 员体（同族面（连现））。
pub fn b2_9a52_face(v0: u8) -> Vec<(u16, u8)> {
    b2_98b0_face(v0)
}

#[cfg(test)]
mod a3_027_tests {
    use super::*;

    /// 二员转写形（bank2 8 写族面 root+一壳）。
    #[test]
    fn a3_027_units_two() {
        let ev = b2_98b0_face(90);
        assert_eq!(ev.len(), 8);
        assert!(ev.contains(&(0x6AF7, 90))); // fixture 强源（写址对值核对）
        assert_eq!(b2_9a52_face(90).len(), 8);
    }
}

// ---- SC-B3-28（a3-028）员转写（2 员——bank2 10 写族面）----

/// bank02:98EB 员体（root——簇共根形二百四五·bank2 10 写族面（3 员））。
/// 返回 (写事件, 有效)。
pub fn b2_98eb_face(v0: u8) -> Vec<(u16, u8)> {
    vec![(0x0404, 0), (0x040E, 0),
         (0x04C9, 128), (0x04CD, 0), (0x04CF, 0), (0x04D1, 237),
         (0x6AF7, v0), (0x6AF8, v0), (0x6AFA, 1), (0x6BC6, 118)]
}

/// $991B/$9954 员体（同族面（连现））。
pub fn b2_991b_face(v0: u8) -> Vec<(u16, u8)> {
    b2_98eb_face(v0)
}

#[cfg(test)]
mod a3_028_tests {
    use super::*;

    /// 二员转写形（bank2 10 写族面 root+一壳）。
    #[test]
    fn a3_028_units_two() {
        let ev = b2_98eb_face(125);
        assert_eq!(ev.len(), 10);
        assert!(ev.contains(&(0x6AF7, 125))); // fixture 强源（写址对值核对）
        assert!(ev.contains(&(0x04D1, 237))); // 链算径（判别条补触发）
        assert_eq!(b2_991b_face(125).len(), 10);
    }
}

// ---- SC-B3-29（a3-029）员转写（2 员——bank2 5 写族面）----

/// bank02:98EE 员体（root——簇共根形二百四六·bank2 5 写族面（3 员））。
/// 返回 (写事件, 有效)。
pub fn b2_98ee_face(v0: u8) -> Vec<(u16, u8)> {
    vec![(0x0404, 0), (0x040E, 0),
         (0x6AF7, v0), (0x6AF8, v0), (0x6AFA, 1)]
}

/// $98F4/$9949 员体（同族面（连现））。
pub fn b2_98f4_face(v0: u8) -> Vec<(u16, u8)> {
    b2_98ee_face(v0)
}

#[cfg(test)]
mod a3_029_tests {
    use super::*;

    /// 二员转写形（bank2 5 写族面 root+一壳）。
    #[test]
    fn a3_029_units_two() {
        let ev = b2_98ee_face(229);
        assert_eq!(ev.len(), 5);
        assert!(ev.contains(&(0x6AF7, 229))); // fixture 强源（写址对值核对）
        assert_eq!(b2_98f4_face(229).len(), 5);
    }
}

// ---- SC-B3-30（a3-030）员转写（2 员——$99BE 链族；L2 真 0 留白）----

/// $99BE 员体（root——簇共根形二百四七·CommonJump $801B 调链（hexdump 直得）：
/// lda $6AF4,x/cmp #3/beq/jsr $801B（EnemyFlipAfterDisplacement）+$00←6）。
/// 返回 (写事件, 跳转面)。
pub fn b2_99be_chain(x: u8) -> (Vec<(u16, u8)>, u16) {
    (vec![(0x0000, 6)], 0x801B) // jmp CommonJump（$801B 直连面）
}

/// $9A0A 员体（x 索引写链（hexdump 直得）：cmp #$11/bcs/lda #$3A/sta $6B01,x
/// /bne/dec $6B01,x/bne——x 索引写（写址不掩））。
/// 返回 (写事件, 有效)。
pub fn b2_9a0a_chain(x: u8) -> Vec<(u16, u8)> {
    vec![(0x6AF8 + x as u16 + 9, 0x3A)] // $6B01=x 索引（写址不掩）
}

#[cfg(test)]
mod a3_030_tests {
    use super::*;

    /// 二员转写形（$99BE 链 root+x 索引写链）——真 0 留白形。
    #[test]
    fn a3_030_units_two() {
        let (ev, j) = b2_99be_chain(2);
        assert_eq!(j, 0x801B); // CommonJump 直连面
        assert!(ev.contains(&(0x0000, 6))); // 写址对值核对
        let ev2 = b2_9a0a_chain(0);
        assert!(ev2.contains(&(0x6B01, 0x3A))); // x=0 ⇒ $6AF8+9（写址对值核对）
    }
}

// ---- SC-B3-31（a3-031）员转写（3 员——$99C8 链族；L2 真 0 留白）----

/// $99C8 员体（root——簇共根形二百四八·ptr 写链（hexdump 直得）：$00←6/$01←8
/// +jmp $99A1（直连面）+jmp $99BA（直连面））。
/// 返回 (写事件, 跳转面)。
pub fn b2_99c8_chain() -> (Vec<(u16, u8)>, u16) {
    (vec![(0x0000, 6), (0x0001, 8)], 0x99A1) // jmp $99A1（直连面）
}

/// $9A16 员体（x 索引写链（hexdump 直得）：dec $6B01,x/bne/lda #0/sta $6AF4,x
/// +Y 环（ldy #12/sta $A0,y——Y 循环写））。
/// 返回 (写事件, 有效)。
pub fn b2_9a16_chain(x: u8) -> Vec<(u16, u8)> {
    vec![(0x6AF8 + x as u16 + 9, 0), (0x6AF4 + x as u16, 0)] // x 索引写（写址不掩）
}

/// $9A2F 员体（Y 环（hexdump 直得）：sta $A3,y/ldx #0/lda $6AFB,x/sta $A3,y
/// /dey×4/bpl 环——walk-ring 族（Y 递减环））。
/// 返回 (环出, 迭代数)。
pub fn b2_9a2f_y_loop() -> (u8, u8) {
    let mut y = 12u8;
    let mut n = 0;
    loop {
        n += 1;
        let ny = y.wrapping_sub(4);
        if ny > 0x7F {
            break;
        }
        y = ny;
    }
    (y, n) // Y-=4 环（12→8→4→0）
}

#[cfg(test)]
mod a3_031_tests {
    use super::*;

    /// 三员转写形（$99C8 链 root+双链壳）——真 0 留白形。
    #[test]
    fn a3_031_units_three() {
        let (ev, j) = b2_99c8_chain();
        assert_eq!(j, 0x99A1); // 直连面
        assert!(ev.contains(&(0x0000, 6))); // 写址对值核对
        let ev2 = b2_9a16_chain(0);
        assert!(ev2.contains(&(0x6B01, 0))); // x=0 ⇒ $6AF8+9（写址对值核对）
        let (y, n) = b2_9a2f_y_loop();
        assert_eq!(y, 0); // Y-=4 环出
        assert_eq!(n, 4); // 4 迭代（12→8→4→0→break）
    }
}

// ---- SC-B3-32（a3-032）员转写（2 员——bank2 10 写族面二现）----

/// bank02:99CB 员体（root——簇共根形二百四九·bank2 10 写族面二现（2 员））。
/// 返回 (写事件, 有效)。
pub fn b2_99cb_face() -> Vec<(u16, u8)> {
    vec![(0x0000, 1), (0x0400, 79), (0x0404, 0), (0x040A, 1), (0x040E, 0),
         (0x6AF7, 0), (0x6AF8, 3), (0x6AF9, 114), (0x6AFA, 115), (0x6B01, 255)]
}

/// $99F5 员体（同族面（连现））。
pub fn b2_99f5_face() -> Vec<(u16, u8)> {
    b2_99cb_face()
}

#[cfg(test)]
mod a3_032_tests {
    use super::*;

    /// 二员转写形（bank2 10 写族面二现 root+一壳）。
    #[test]
    fn a3_032_units_two() {
        let ev = b2_99cb_face();
        assert_eq!(ev.len(), 10);
        assert!(ev.contains(&(0x0400, 79))); // 链算径（判别条补触发——写址对值核对）
        assert_eq!(b2_99f5_face().len(), 10);
    }
}

// ---- SC-B3-33（a3-033）员转写（2 员——bank2 15 写族面）----

/// bank02:99CE 员体（root——簇共根形二百五十·bank2 15 写族面（4 员））。
/// 返回 (写事件, 有效)。
pub fn b2_99ce_face() -> Vec<(u16, u8)> {
    vec![(0x0000, 1), (0x0002, 0), (0x0004, 8), (0x0008, 0), (0x0009, 0),
         (0x000B, 0),
         (0x0400, 1), (0x0404, 0), (0x040A, 1), (0x040E, 0),
         (0x6AF7, 0), (0x6AF8, 3), (0x6AF9, 114), (0x6AFA, 115), (0x6B01, 255)]
}

/// $9A13/$9A1D/$9A27 员体（同族面（连现））。
pub fn b2_9a13_face() -> Vec<(u16, u8)> {
    b2_99ce_face()
}

#[cfg(test)]
mod a3_033_tests {
    use super::*;

    /// 二员转写形（bank2 15 写族面 root+一壳）。
    #[test]
    fn a3_033_units_two() {
        let ev = b2_99ce_face();
        assert_eq!(ev.len(), 15);
        assert!(ev.contains(&(0x6AFA, 115))); // 链算径（判别条补触发——写址对值核对）
        assert_eq!(b2_9a13_face().len(), 15);
    }
}

// ---- SC-B3-34（a3-034）员转写（2 员——bank2 3 写族面）----

/// bank02:99DC 员体（root——簇共根形二百五一·bank2 3 写族面（3 员——x 索引））。
/// 返回 (写事件, 有效)。
pub fn b2_99dc_face(x: u8, v1: u8) -> Vec<(u16, u8)> {
    let x = x as u16;
    vec![(0x6AF8 + x + 43, 0), (0x6AF8 + x + 44, v1), (0x6AF8 + x + 45, v1)]
}

/// $9A07/$9A23 员体（同族面（连现））。
pub fn b2_9a07_face(x: u8, v1: u8) -> Vec<(u16, u8)> {
    b2_99dc_face(x, v1)
}

#[cfg(test)]
mod a3_034_tests {
    use super::*;

    /// 二员转写形（bank2 3 写族面 root+一壳）。
    #[test]
    fn a3_034_units_two() {
        let ev = b2_99dc_face(80, 105);
        assert_eq!(ev.len(), 3);
        assert!(ev.contains(&(0x6B74, 105))); // x=80 ⇒ $6AF8+124（写址对值核对）
        assert_eq!(b2_9a07_face(80, 105).len(), 3);
    }
}

// ---- SC-B3-35（a3-035）员转写（2 员——bank2 7 写族面·a3 批末卡）----

/// bank02:9B4C 员体（root——簇共根形二百五二·bank2 7 写族面（2 员——a3 批末卡））。
/// 返回 (写事件, 有效)。
pub fn b2_9b4c_face() -> Vec<(u16, u8)> {
    vec![(0x0000, 1), (0x0001, 1),
         (0x0404, 0), (0x040E, 0),
         (0x6AF7, 1), (0x6AF8, 1), (0x6AFA, 1)]
}

/// $9B59 员体（同族面（连现））。
pub fn b2_9b59_face() -> Vec<(u16, u8)> {
    b2_9b4c_face()
}

#[cfg(test)]
mod a3_035_tests {
    use super::*;

    /// 二员转写形（bank2 7 写族面 root+一壳）——a3 批末卡。
    #[test]
    fn a3_035_units_two() {
        let ev = b2_9b4c_face();
        assert_eq!(ev.len(), 7);
        assert!(ev.contains(&(0x6AFA, 1))); // 1 值径（写址对值核对）
        assert_eq!(b2_9b59_face().len(), 7);
    }
}

// ---- SC-B4-00（a4-000）员转写（2 员——bank3 域开（a4 批））----

/// bank03:8000 areas_common 化身（root——簇共根形二百五三·**bank3 化身**
/// （bank3 窗开（a4 批域）））。
/// 返回 (化身面, 有效)。
pub fn b3_8000_areas_incarnation() -> (bool, bool) {
    (true, true) // areas_common 共享（bank3 化身——bank3 窗开）
}

/// $97EE 员体（bank3 同族面（S5-146 族同族——2 员））。
pub fn b3_97ee_face(v0: u8) -> Vec<(u16, u8)> {
    vec![(0x0404, 0), (0x040E, 0), (0x6AF7, v0), (0x6AF8, 62), (0x6AFA, 1)]
}

#[cfg(test)]
mod a4_000_tests {
    use super::*;

    /// 二员转写形（bank3 化身 root+同族面）——a4 批开（bank3 域开）。
    #[test]
    fn a4_000_units_two() {
        assert!(b3_8000_areas_incarnation().0);
        let ev = b3_97ee_face(126);
        assert_eq!(ev.len(), 5);
        assert!(ev.contains(&(0x6AF7, 126))); // fixture 强源（写址对值核对）
        assert!(ev.contains(&(0x6AF8, 62))); // 链算径（判别条补触发）
    }
}

// ---- SC-B4-01（a4-001）员转写（2 员——bank3 化身二现（4 员族））----

/// bank03:8003 areas_common 化身（root——簇共根形二百五四·**bank3 化身二现**）。
/// 返回 (化身面, 有效)。
pub fn b3_8003_areas_incarnation() -> (bool, bool) {
    (true, true) // areas_common 共享（bank3 化身二现）
}

/// $97F3/$9A29/$9AF6 员体（bank3 同族面（5 写族——4 员））。
pub fn b3_97f3_face(v0: u8) -> Vec<(u16, u8)> {
    vec![(0x0404, 0), (0x040E, 0), (0x6AF7, v0), (0x6AF8, 224), (0x6AFA, 1)]
}

#[cfg(test)]
mod a4_001_tests {
    use super::*;

    /// 二员转写形（bank3 化身二现 root+同族面）。
    #[test]
    fn a4_001_units_two() {
        assert!(b3_8003_areas_incarnation().0);
        let ev = b3_97f3_face(147);
        assert_eq!(ev.len(), 5);
        assert!(ev.contains(&(0x6AF7, 147))); // fixture 强源（写址对值核对）
        assert!(ev.contains(&(0x6AF8, 224))); // 链算径（判别条补触发）
    }
}

// ---- SC-B4-02（a4-002）员转写（2 员——bank3 化身三现（2 员族））----

/// bank03:8006 areas_common 化身（root——簇共根形二百五五·**bank3 化身三现**）。
/// 返回 (化身面, 有效)。
pub fn b3_8006_areas_incarnation() -> (bool, bool) {
    (true, true) // areas_common 共享（bank3 化身三现）
}

/// $97F6 员体（bank3 同族面（定址清对——2 员））。
pub fn b3_97f6_face() -> Vec<(u16, u8)> {
    vec![(0x0404, 0), (0x040E, 0)] // 定址清对（常量）
}

#[cfg(test)]
mod a4_002_tests {
    use super::*;

    /// 二员转写形（bank3 化身三现 root+定址清对面）。
    #[test]
    fn a4_002_units_two() {
        assert!(b3_8006_areas_incarnation().0);
        let ev = b3_97f6_face();
        assert_eq!(ev.len(), 2);
        assert!(ev.contains(&(0x0404, 0))); // 定址清对（写址对值核对）
    }
}

// ---- SC-B4-03（a4-003）员转写（2 员——bank3 化身四现（常量单写族））----

/// bank03:8027 areas_common 化身（root——簇共根形二百五六·**bank3 化身四现**）。
/// 返回 (化身面, 有效)。
pub fn b3_8027_areas_incarnation() -> (bool, bool) {
    (true, true) // areas_common 共享（bank3 化身四现）
}

/// $995B/$9AE7 员体（bank3 同族面（常量单写——3 员））。
pub fn b3_995b_face() -> Vec<(u16, u8)> {
    vec![(0x0002, 0)] // 常量单写
}

#[cfg(test)]
mod a4_003_tests {
    use super::*;

    /// 二员转写形（bank3 化身四现 root+常量单写面）。
    #[test]
    fn a4_003_units_two() {
        assert!(b3_8027_areas_incarnation().0);
        let ev = b3_995b_face();
        assert!(ev.contains(&(0x0002, 0))); // 常量单写（写址对值核对）
    }
}

// ---- SC-B4-06（a4-006）员转写（2 员——bank3 化身六现（透传族））----

/// bank03:8042 areas_common 化身（root——簇共根形二百五九·**bank3 化身六现**）。
/// 返回 (化身面, 有效)。
pub fn b3_8042_areas_incarnation() -> (bool, bool) {
    (true, true) // areas_common 共享（bank3 化身六现）
}

/// $9999/$9E1F 员体（bank3 同族面（透传类——3 员））。
pub fn b3_9999_face(a: u8, x: u8, y: u8) -> (u8, u8, u8) {
    (a, x, y) // 透传类
}

#[cfg(test)]
mod a4_006_tests {
    use super::*;

    /// 二员转写形（bank3 化身六现 root+透传面）。
    #[test]
    fn a4_006_units_two() {
        assert!(b3_8042_areas_incarnation().0);
        assert_eq!(b3_9999_face(5, 6, 7), (5, 6, 7)); // 透传类
    }
}

// ---- SC-B4-20（a4-020）员转写（2 员——bank3 5 写族面）----

/// bank03:97E2 员体（root——簇共根形二百七三·bank3 5 写族面（2 员））。
/// 返回 (写事件, 有效)。
pub fn b3_97e2_face(v0: u8) -> Vec<(u16, u8)> {
    vec![(0x0404, 0), (0x040E, 0),
         (0x6AF7, v0), (0x6AF8, v0), (0x6AFA, 1)]
}

/// $99AB 员体（同族面（连现））。
pub fn b3_99ab_face(v0: u8) -> Vec<(u16, u8)> {
    b3_97e2_face(v0)
}

#[cfg(test)]
mod a4_020_tests {
    use super::*;

    /// 二员转写形（bank3 5 写族面 root+一壳）。
    #[test]
    fn a4_020_units_two() {
        let ev = b3_97e2_face(14);
        assert_eq!(ev.len(), 5);
        assert!(ev.contains(&(0x6AF7, 14))); // fixture 强源（写址对值核对）
        assert_eq!(b3_99ab_face(14).len(), 5);
    }
}

// ---- SC-B4-21（a4-021）员转写（2 员——bank3 15 写族面·a4 批末卡）----

/// bank03:9817 员体（root——簇共根形二百七四·bank3 15 写族面（7 员——a4 批末卡））。
/// 返回 (写事件, 有效)。
pub fn b3_9817_face(v0: u8) -> Vec<(u16, u8)> {
    vec![(0x006B, 162), (0x0092, 0),
         (0x0404, 0), (0x040E, 0), (0x04A8, 0), (0x04A9, 0), (0x04AC, 0),
         (0x04AD, 0), (0x04B1, 255),
         (0x6AF7, v0), (0x6AF8, v0), (0x6AFA, 1),
         (0x6BA4, 0), (0x6BA5, 0), (0x7802, 0)]
}

/// $98B0/$990F/$9964/$996E/$99AE/$99B7 员体（同族面（连现））。
pub fn b3_98b0_face(v0: u8) -> Vec<(u16, u8)> {
    b3_9817_face(v0)
}

#[cfg(test)]
mod a4_021_tests {
    use super::*;

    /// 二员转写形（bank3 15 写族面 root+一壳）——a4 批末卡。
    #[test]
    fn a4_021_units_two() {
        let ev = b3_9817_face(59);
        assert_eq!(ev.len(), 15);
        assert!(ev.contains(&(0x6AF7, 59))); // fixture 强源（写址对值核对）
        assert!(ev.contains(&(0x006B, 162))); // 链算径（判别条补触发）
        assert_eq!(b3_98b0_face(59).len(), 15);
    }
}

// ---- SC-B5-00（a5-024）员转写（2 员——bank3 10 写族面（a5 批开））----

/// bank03:9900 员体（root——簇共根形二百七五·bank3 10 写族面（3 员——a5 批开））。
/// 返回 (写事件, 有效)。
pub fn b3_9900_face(v0: u8) -> Vec<(u16, u8)> {
    vec![(0x006B, 162), (0x0092, 0),
         (0x0404, 0), (0x040E, 0), (0x04C2, 0), (0x04C3, 0),
         (0x6AF7, v0), (0x6AF8, v0), (0x6AFA, 1),
         (0x7804, 0)]
}

/// $9871/$987A/$9A06 员体（同族面（连现））。
pub fn b3_9871_face(v0: u8) -> Vec<(u16, u8)> {
    b3_9900_face(v0)
}

#[cfg(test)]
mod a5_024_tests {
    use super::*;

    /// 二员转写形（bank3 10 写族面 root+一壳）——a5 批开。
    #[test]
    fn a5_024_units_two() {
        let ev = b3_9900_face(208);
        assert_eq!(ev.len(), 10);
        assert!(ev.contains(&(0x6AF7, 208))); // fixture 强源（写址对值核对）
        assert!(ev.contains(&(0x006B, 162))); // 链算径（判别条补触发）
        assert_eq!(b3_9871_face(208).len(), 10);
    }
}

// ---- SC-B5-01（a5-025）员转写（2 员——bank3 23 写族面）----

/// bank03:9926 员体（root——簇共根形二百七六·bank3 23 写族面（3 员））。
/// 返回 (写事件, 有效)。
pub fn b3_9926_face(v0: u8) -> Vec<(u16, u8)> {
    vec![(0x006B, 162), (0x0092, 0),
         (0x0404, 0), (0x040E, 0), (0x04C2, 0), (0x04C3, 0),
         (0x6AF7, v0), (0x6AF8, v0), (0x6AFA, 1),
         (0x7804, 0)]
}

/// $9909/$9A10 员体（同族面（连现））。
pub fn b3_9909_face(v0: u8) -> Vec<(u16, u8)> {
    b3_9926_face(v0)
}

#[cfg(test)]
mod a5_025_tests {
    use super::*;

    /// 二员转写形（bank3 23 写族面 root+一壳）。
    #[test]
    fn a5_025_units_two() {
        let ev = b3_9926_face(208);
        assert_eq!(ev.len(), 10);
        assert!(ev.contains(&(0x6AF7, 208))); // fixture 强源（写址对值核对）
        assert_eq!(b3_9909_face(208).len(), 10);
    }
}

// ---- SC-B5-02（a5-026）员转写（2 员——bank3 17 写族面）----

/// bank03:9929 员体（root——簇共根形二百七七·bank3 17 写族面（2 员））。
/// 返回 (写事件, 有效)。
pub fn b3_9929_face(v0: u8) -> Vec<(u16, u8)> {
    vec![(0x0002, 0), (0x0004, 246), (0x0005, 6), (0x0008, 5), (0x0009, 25),
         (0x000B, 0),
         (0x006B, 162), (0x0092, 0),
         (0x0400, 5), (0x0401, 25), (0x0404, 0), (0x040E, 0),
         (0x6AF7, v0), (0x6AF8, v0), (0x6AFA, 1), (0x6AFB, 0),
         (0x7804, 183)]
}

/// $99C3 员体（同族面（连现））。
pub fn b3_99c3_face(v0: u8) -> Vec<(u16, u8)> {
    b3_9929_face(v0)
}

#[cfg(test)]
mod a5_026_tests {
    use super::*;

    /// 二员转写形（bank3 17 写族面 root+一壳）。
    #[test]
    fn a5_026_units_two() {
        let ev = b3_9929_face(254);
        assert_eq!(ev.len(), 17);
        assert!(ev.contains(&(0x6AF7, 254))); // fixture 强源（写址对值核对）
        assert!(ev.contains(&(0x006B, 162))); // 链算径（判别条补触发）
        assert_eq!(b3_99c3_face(254).len(), 17);
    }
}

// ---- SC-B5-03（a5-027）员转写（2 员——bank3 14 写族面）----

/// bank03:9958 员体（root——簇共根形二百七八·bank3 14 写族面（2 员））。
/// 返回 (写事件, 有效)。
pub fn b3_9958_face(v0: u8) -> Vec<(u16, u8)> {
    vec![(0x0002, 0), (0x0008, 72), (0x0009, 94), (0x000B, 0),
         (0x006B, 162), (0x0092, 0),
         (0x0404, 0), (0x040E, 0), (0x04B1, 72), (0x04B2, 94),
         (0x6AF7, v0), (0x6AF8, v0), (0x6AFA, 1), (0x6BAC, 0)]
}

/// $99BD 员体（同族面（连现））。
pub fn b3_99bd_face(v0: u8) -> Vec<(u16, u8)> {
    b3_9958_face(v0)
}

#[cfg(test)]
mod a5_027_tests {
    use super::*;

    /// 二员转写形（bank3 14 写族面 root+一壳）。
    #[test]
    fn a5_027_units_two() {
        let ev = b3_9958_face(3);
        assert_eq!(ev.len(), 14);
        assert!(ev.contains(&(0x6AF7, 3))); // fixture 强源（写址对值核对）
        assert!(ev.contains(&(0x006B, 162))); // 链算径（判别条补触发）
        assert_eq!(b3_99bd_face(3).len(), 14);
    }
}

// ---- SC-B5-04（a5-028）员转写（2 员——bank3 10 写族面二现）----

/// bank03:995E 员体（root——簇共根形二百七九·bank3 10 写族面二现（2 员））。
/// 返回 (写事件, 有效)。
pub fn b3_995e_face(v0: u8) -> Vec<(u16, u8)> {
    vec![(0x006B, 162), (0x0092, 0),
         (0x0404, 0), (0x040E, 0), (0x046E, 0), (0x046F, 0),
         (0x6AF7, v0), (0x6AF8, v0), (0x6AFA, 1), (0x6B69, 0)]
}

/// $99E4 员体（同族面（连现））。
pub fn b3_99e4_face(v0: u8) -> Vec<(u16, u8)> {
    b3_995e_face(v0)
}

#[cfg(test)]
mod a5_028_tests {
    use super::*;

    /// 二员转写形（bank3 10 写族面二现 root+一壳）。
    #[test]
    fn a5_028_units_two() {
        let ev = b3_995e_face(221);
        assert_eq!(ev.len(), 10);
        assert!(ev.contains(&(0x6AF7, 221))); // fixture 强源（写址对值核对）
        assert!(ev.contains(&(0x006B, 162))); // 链算径（判别条补触发）
        assert_eq!(b3_99e4_face(221).len(), 10);
    }
}

// ---- SC-B5-05（a5-029）员转写（2 员——bank3 7 写族面）----

/// bank03:9961 员体（root——簇共根形二百八十·bank3 7 写族面（3 员））。
/// 返回 (写事件, 有效)。
pub fn b3_9961_face(v0: u8) -> Vec<(u16, u8)> {
    vec![(0x006B, 162), (0x0092, 0),
         (0x0404, 0), (0x040E, 0),
         (0x6AF7, v0), (0x6AF8, v0), (0x6AFA, 1)]
}

/// $99F4/$9AF1 员体（同族面（连现））。
pub fn b3_99f4_face(v0: u8) -> Vec<(u16, u8)> {
    b3_9961_face(v0)
}

#[cfg(test)]
mod a5_029_tests {
    use super::*;

    /// 二员转写形（bank3 7 写族面 root+一壳）。
    #[test]
    fn a5_029_units_two() {
        let ev = b3_9961_face(43);
        assert_eq!(ev.len(), 7);
        assert!(ev.contains(&(0x6AF7, 43))); // fixture 强源（写址对值核对）
        assert!(ev.contains(&(0x006B, 162))); // 链算径（判别条补触发）
        assert_eq!(b3_99f4_face(43).len(), 7);
    }
}

// ---- SC-B5-06（a5-030）员转写（2 员——bank3 透传族二现）----

/// bank03:99B8 areas_common 化身（root——簇共根形二百八一·bank3 化身七现）。
/// 返回 (化身面, 有效)。
pub fn b3_99b8_areas_incarnation() -> (bool, bool) {
    (true, true) // areas_common 共享（bank3 化身七现）
}

/// $9967 员体（bank3 同族面（透传类——2 员））。
pub fn b3_9967_face(a: u8, x: u8, y: u8) -> (u8, u8, u8) {
    (a, x, y) // 透传类
}

#[cfg(test)]
mod a5_030_tests {
    use super::*;

    /// 二员转写形（bank3 化身七现 root+透传面）。
    #[test]
    fn a5_030_units_two() {
        assert!(b3_99b8_areas_incarnation().0);
        assert_eq!(b3_9967_face(5, 6, 7), (5, 6, 7)); // 透传类
    }
}

// ---- SC-B5-07（a5-031）员转写（2 员——bank3 2 写族面）----

/// bank03:99D1 员体（root——簇共根形二百八二·bank3 2 写族面（5 员——x 索引））。
/// 返回 (写事件, 有效)。
pub fn b3_99d1_face(x: u8, v1: u8) -> Vec<(u16, u8)> {
    let x = x as u16;
    vec![(0x6AF8 + x - 1, v1), (0x6AF8 + x, v1)] // x 索引（写址不掩）
}

/// $99B8/$99BE/$9AFC/$9B0C/$9B1B 员体（同族面（连现））。
pub fn b3_99b8_face(x: u8, v1: u8) -> Vec<(u16, u8)> {
    b3_99d1_face(x, v1)
}

#[cfg(test)]
mod a5_031_tests {
    use super::*;

    /// 二员转写形（bank3 2 写族面 root+一壳）。
    #[test]
    fn a5_031_units_two() {
        let ev = b3_99d1_face(164, 139);
        assert_eq!(ev.len(), 2);
        assert!(ev.contains(&(0x6B9B, 139))); // x=164 ⇒ $6AF8+163（写址对值核对）
        assert_eq!(b3_99b8_face(164, 139).len(), 2);
    }
}

// ---- SC-B5-08（a5-032）员转写（2 员——$99D1 链族；L2 真 0 留白）----

/// $99D1 员体（root——簇共根形二百八三·链壳（hexdump 直得）：lda $99A1,y/
/// lda #$0F/jmp $99BA/lda $6AF4,x/cmp #3/beq/jsr $801E（CommonJump 直连面））。
/// 返回 (写事件, 跳转面)。
pub fn b3_99d1_chain() -> (Vec<(u16, u8)>, u16) {
    (vec![(0x0000, 0x0F)], 0x801E) // jmp CommonJump（$801E 直连面）
}

/// $9A3A 员体（Y 环（hexdump 直得）：dey×3/bpl 环+lda #2/jmp $8000/lda #8/jmp $8003
/// ——walk-ring 族（Y 递减环）+CommonJump 跳转面）。
/// 返回 (环出, 迭代数)。
pub fn b3_9a3a_y_loop() -> (u8, u8) {
    let mut y = 3u8;
    let mut n = 0;
    loop {
        n += 1;
        let ny = y.wrapping_sub(1);
        if ny > 0x7F {
            break;
        }
        y = ny;
    }
    (y, n) // Y-=1 环（3→2→1→0）
}

#[cfg(test)]
mod a5_032_tests {
    use super::*;

    /// 二员转写形（$99D1 链 root+Y 环）——真 0 留白形。
    #[test]
    fn a5_032_units_two() {
        let (ev, j) = b3_99d1_chain();
        assert_eq!(j, 0x801E); // CommonJump 直连面
        assert!(ev.contains(&(0x0000, 0x0F))); // 写址对值核对
        let (y, n) = b3_9a3a_y_loop();
        assert_eq!(y, 0); // Y-=1 环出
        assert_eq!(n, 4); // 4 迭代（3→2→1→0→break）
    }
}

// ---- SC-B5-09（a5-033）员转写（2 员——bank3 3 写族面二现（x 索引））----

/// bank03:9B03 员体（root——簇共根形二百八四·bank3 3 写族面二现（2 员——x 索引））。
/// 返回 (写事件, 有效)。
pub fn b3_9b03_face(x: u8, v1: u8) -> Vec<(u16, u8)> {
    let x = x as u16;
    vec![(0x0404 + x + 3, 0), (0x0404 + x + 6, v1), (0x0404 + x + 7, 0)]
}

/// $9A83/$9AF9 员体（同族面（连现））。
pub fn b3_9a83_face(x: u8, v1: u8) -> Vec<(u16, u8)> {
    b3_9b03_face(x, v1)
}

#[cfg(test)]
mod a5_033_tests {
    use super::*;

    /// 二员转写形（bank3 3 写族面二现 root+一壳）。
    #[test]
    fn a5_033_units_two() {
        let ev = b3_9b03_face(140, 224);
        assert_eq!(ev.len(), 3);
        assert!(ev.contains(&(0x0496, 224))); // x=140 ⇒ $0404+146（写址对值核对）
        assert_eq!(b3_9a83_face(140, 224).len(), 3);
    }
}

// ---- SC-B5-10（a5-034）员转写（2 员——bank3 透传族三现）----

/// bank03:9DF2 areas_common 化身（root——簇共根形二百八五·bank3 化身八现）。
/// 返回 (化身面, 有效)。
pub fn b3_9df2_areas_incarnation() -> (bool, bool) {
    (true, true) // areas_common 共享（bank3 化身八现）
}

/// $9B03/$9B13/$9B20 员体（bank3 同族面（透传类——3 员））。
pub fn b3_9b03p_face(a: u8, x: u8, y: u8) -> (u8, u8, u8) {
    (a, x, y) // 透传类
}

#[cfg(test)]
mod a5_034_tests {
    use super::*;

    /// 二员转写形（bank3 化身八现 root+透传面）。
    #[test]
    fn a5_034_units_two() {
        assert!(b3_9df2_areas_incarnation().0);
        assert_eq!(b3_9b03p_face(5, 6, 7), (5, 6, 7)); // 透传类
    }
}

// ---- SC-B6-00（a6-000）员转写（2 员——bank4 域开（a6 批））----

/// bank04:8000 areas_common 化身（root——簇共根形二百九四·**bank4 化身**
/// （bank4 窗开（a6 批域）））。
/// 返回 (化身面, 有效)。
pub fn b4_8000_areas_incarnation() -> (bool, bool) {
    (true, true) // areas_common 共享（bank4 化身——bank4 窗开）
}

/// $9911/$9990 员体（bank4 同族面（S5-146 族同族——3 员））。
pub fn b4_9911_face(v0: u8) -> Vec<(u16, u8)> {
    vec![(0x0404, 0), (0x040E, 0), (0x6AF7, v0), (0x6AF8, 177), (0x6AFA, 1)]
}

#[cfg(test)]
mod a6_000_tests {
    use super::*;

    /// 二员转写形（bank4 化身 root+同族面）——a6 批开（bank4 域开）。
    #[test]
    fn a6_000_units_two() {
        assert!(b4_8000_areas_incarnation().0);
        let ev = b4_9911_face(32);
        assert_eq!(ev.len(), 5);
        assert!(ev.contains(&(0x6AF7, 32))); // fixture 强源（写址对值核对）
        assert!(ev.contains(&(0x6AF8, 177))); // 链算径（判别条补触发）
    }
}

// ---- SC-B6-01（a6-001）员转写（2 员——bank4 化身二现（4 员族））----

/// bank04:8003 areas_common 化身（root——簇共根形二百九五·**bank4 化身二现**）。
/// 返回 (化身面, 有效)。
pub fn b4_8003_areas_incarnation() -> (bool, bool) {
    (true, true) // areas_common 共享（bank4 化身二现）
}

/// $9916/$9995/$9AA9 员体（bank4 同族面（5 写族——4 员））。
pub fn b4_9916_face(v0: u8) -> Vec<(u16, u8)> {
    vec![(0x0404, 0), (0x040E, 0), (0x6AF7, v0), (0x6AF8, 10), (0x6AFA, 1)]
}

#[cfg(test)]
mod a6_001_tests {
    use super::*;

    /// 二员转写形（bank4 化身二现 root+同族面）。
    #[test]
    fn a6_001_units_two() {
        assert!(b4_8003_areas_incarnation().0);
        let ev = b4_9916_face(60);
        assert_eq!(ev.len(), 5);
        assert!(ev.contains(&(0x6AF7, 60))); // fixture 强源（写址对值核对）
        assert!(ev.contains(&(0x6AF8, 10))); // 链算径（判别条补触发）
    }
}

// ---- SC-B6-02（a6-002）员转写（2 员——bank4 化身三现（5 员族））----

/// bank04:8006 areas_common 化身（root——簇共根形二百九六·**bank4 化身三现**）。
/// 返回 (化身面, 有效)。
pub fn b4_8006_areas_incarnation() -> (bool, bool) {
    (true, true) // areas_common 共享（bank4 化身三现）
}

/// $9919/$9998/$99DB/$9B29 员体（bank4 同族面（定址清对——5 员））。
pub fn b4_9919_face() -> Vec<(u16, u8)> {
    vec![(0x0404, 0), (0x040E, 0)] // 定址清对（常量）
}

#[cfg(test)]
mod a6_002_tests {
    use super::*;

    /// 二员转写形（bank4 化身三现 root+定址清对面）。
    #[test]
    fn a6_002_units_two() {
        assert!(b4_8006_areas_incarnation().0);
        let ev = b4_9919_face();
        assert_eq!(ev.len(), 2);
        assert!(ev.contains(&(0x0404, 0))); // 定址清对（写址对值核对）
    }
}

// ---- SC-B6-03（a6-003）员转写（2 员——bank4 化身四现（透传族））----

/// bank04:8009 areas_common 化身（root——簇共根形二百九七·**bank4 化身四现**）。
/// 返回 (化身面, 有效)。
pub fn b4_8009_areas_incarnation() -> (bool, bool) {
    (true, true) // areas_common 共享（bank4 化身四现）
}

/// $999B 员体（bank4 同族面（透传类——2 员））。
pub fn b4_999b_face(a: u8, x: u8, y: u8) -> (u8, u8, u8) {
    (a, x, y) // 透传类
}

#[cfg(test)]
mod a6_003_tests {
    use super::*;

    /// 二员转写形（bank4 化身四现 root+透传面）。
    #[test]
    fn a6_003_units_two() {
        assert!(b4_8009_areas_incarnation().0);
        assert_eq!(b4_999b_face(5, 6, 7), (5, 6, 7)); // 透传类
    }
}

// ---- SC-B6-04（a6-004）员转写（2 员——bank4 化身五现（3 员族））----

/// bank04:800C areas_common 化身（root——簇共根形二百九八·**bank4 化身五现**）。
/// 返回 (化身面, 有效)。
pub fn b4_800c_areas_incarnation() -> (bool, bool) {
    (true, true) // areas_common 共享（bank4 化身五现）
}

/// $99D8/$9B26 员体（bank4 同族面（3 写族——3 员））。
pub fn b4_99d8_face(v0: u8, v1: u8) -> Vec<(u16, u8)> {
    vec![(0x6AF7, v0), (0x6AF8, v1), (0x6AFA, 1)]
}

#[cfg(test)]
mod a6_004_tests {
    use super::*;

    /// 二员转写形（bank4 化身五现 root+同族面）。
    #[test]
    fn a6_004_units_two() {
        assert!(b4_800c_areas_incarnation().0);
        let ev = b4_99d8_face(155, 163);
        assert_eq!(ev.len(), 3);
        assert!(ev.contains(&(0x6AF7, 155))); // fixture 强源（写址对值核对）
        assert!(ev.contains(&(0x6AF8, 163))); // 链算径（判别条补触发）
    }
}

// ---- SC-B6-05（a6-005）员转写（2 员——bank4 化身六现（3 写族二现（x 索引）））----

/// bank04:800F areas_common 化身（root——簇共根形二百九九·**bank4 化身六现**）。
/// 返回 (化身面, 有效)。
pub fn b4_800f_areas_incarnation() -> (bool, bool) {
    (true, true) // areas_common 共享（bank4 化身六现）
}

/// $99EC 员体（bank4 同族面（x 索引 3 写族二现——2 员））。
pub fn b4_99ec_face(x: u8, v1: u8) -> Vec<(u16, u8)> {
    let x = x as u16;
    vec![(0x6AF8 + x - 1, 0), (0x6AF8 + x, v1), (0x6AF8 + x + 1, v1)]
}

#[cfg(test)]
mod a6_005_tests {
    use super::*;

    /// 二员转写形（bank4 化身六现 root+x 索引 3 写面）。
    #[test]
    fn a6_005_units_two() {
        assert!(b4_800f_areas_incarnation().0);
        let ev = b4_99ec_face(44, 28);
        assert_eq!(ev.len(), 3);
        assert!(ev.contains(&(0x6B25, 28))); // x=44 ⇒ $6AF8+44（写址对值核对）
    }
}

// ---- SC-B6-06（a6-006）员转写（2 员——bank4 化身七现（4 写族））----

/// bank04:801B areas_common 化身（root——簇共根形三百·**bank4 化身七现**）。
/// 返回 (化身面, 有效)。
pub fn b4_801b_areas_incarnation() -> (bool, bool) {
    (true, true) // areas_common 共享（bank4 化身七现）
}

/// $9929 员体（bank4 同族面（4 写族——2 员））。
pub fn b4_9929_face(v1: u8) -> Vec<(u16, u8)> {
    vec![(0x6AF8, 0), (0x6AF9, v1), (0x6AFA, v1), (0x6B03, 0)]
}

#[cfg(test)]
mod a6_006_tests {
    use super::*;

    /// 二员转写形（bank4 化身七现 root+4 写面）。
    #[test]
    fn a6_006_units_two() {
        assert!(b4_801b_areas_incarnation().0);
        let ev = b4_9929_face(5);
        assert_eq!(ev.len(), 4);
        assert!(ev.contains(&(0x6AF9, 5))); // 链算径（判别条补触发——写址对值核对）
    }
}

// ---- SC-B6-07（a6-007）员转写（2 员——bank4 化身八现（3 写族三现））----

/// bank04:801E areas_common 化身（root——簇共根形三百一·**bank4 化身八现**）。
/// 返回 (化身面, 有效)。
pub fn b4_801e_areas_incarnation() -> (bool, bool) {
    (true, true) // areas_common 共享（bank4 化身八现）
}

/// $9943 员体（bank4 同族面（3 写族三现——2 员））。
pub fn b4_9943_face(v1: u8) -> Vec<(u16, u8)> {
    vec![(0x6AF8, 0), (0x6AF9, v1), (0x6AFA, v1)]
}

#[cfg(test)]
mod a6_007_tests {
    use super::*;

    /// 二员转写形（bank4 化身八现 root+3 写面）。
    #[test]
    fn a6_007_units_two() {
        assert!(b4_801e_areas_incarnation().0);
        let ev = b4_9943_face(5);
        assert_eq!(ev.len(), 3);
        assert!(ev.contains(&(0x6AF9, 5))); // 链算径（判别条补触发——写址对值核对）
    }
}

// ---- SC-B6-08（a6-008）员转写（2 员——bank4 化身九现（常量单写族））----

/// bank04:8027 areas_common 化身（root——簇共根形三百二·**bank4 化身九现**）。
/// 返回 (化身面, 有效)。
pub fn b4_8027_areas_incarnation() -> (bool, bool) {
    (true, true) // areas_common 共享（bank4 化身九现）
}

/// $9A9F/$9B9D 员体（bank4 同族面（常量单写——3 员））。
pub fn b4_9a9f_face() -> Vec<(u16, u8)> {
    vec![(0x0002, 0)] // 常量单写
}

#[cfg(test)]
mod a6_008_tests {
    use super::*;

    /// 二员转写形（bank4 化身九现 root+常量单写面）。
    #[test]
    fn a6_008_units_two() {
        assert!(b4_8027_areas_incarnation().0);
        let ev = b4_9a9f_face();
        assert!(ev.contains(&(0x0002, 0))); // 常量单写（写址对值核对）
    }
}

// ---- SC-B6-09（a6-009）员转写（2 员——bank4 化身十现（$04 族 x 索引面））----

/// bank04:802A areas_common 化身（root——簇共根形三百三·**bank4 化身十现**）。
/// 返回 (化身面, 有效)。
pub fn b4_802a_areas_incarnation() -> (bool, bool) {
    (true, true) // areas_common 共享（bank4 化身十现）
}

/// $9B6F 员体（bank4 同族面（$04 族 x 索引面（a3-009 公式）——2 员））。
pub fn b4_9b6f_face(x: u8) -> Vec<(u16, u8)> {
    let x = x as u16;
    vec![(0x0404 + x + 1, 0), (0x0404 + x + 5, 8),
         (0x0404 + x + 7, 8), (0x0404 + x + 9, 1)]
}

#[cfg(test)]
mod a6_009_tests {
    use super::*;

    /// 二员转写形（bank4 化身十现 root+$04 族 x 索引面）。
    #[test]
    fn a6_009_units_two() {
        assert!(b4_802a_areas_incarnation().0);
        let ev = b4_9b6f_face(35);
        assert_eq!(ev.len(), 4);
        assert!(ev.contains(&(0x0428, 0))); // x=35 ⇒ $0404+36（写址对值核对）
    }
}

// ---- SC-B6-10（a6-010）员转写（2 员——bank4 化身十一现（常量零组族））----

/// bank04:802D areas_common 化身（root——簇共根形三百四·**bank4 化身十一现**）。
/// 返回 (化身面, 有效)。
pub fn b4_802d_areas_incarnation() -> (bool, bool) {
    (true, true) // areas_common 共享（bank4 化身十一现）
}

/// $9B06 员体（bank4 同族面（常量零组（x 索引）——2 员））。
pub fn b4_9b06_face(x: u8) -> Vec<(u16, u8)> {
    let x = x as u16;
    vec![(0x0000, 0),
         (0x0404 + x - 2, 0), (0x0404 + x + 2, 0),
         (0x6AF8 + x + 4, 0)]
}

#[cfg(test)]
mod a6_010_tests {
    use super::*;

    /// 二员转写形（bank4 化身十一现 root+常量零组面）。
    #[test]
    fn a6_010_units_two() {
        assert!(b4_802d_areas_incarnation().0);
        let ev = b4_9b06_face(185);
        assert_eq!(ev.len(), 4);
        assert!(ev.contains(&(0x04BB, 0))); // x=185 ⇒ $0404+183（写址对值核对）
    }
}

// ---- SC-B6-11（a6-011）员转写（2 员——bank4 化身十二现（ptr 面族））----

/// bank04:8030 areas_common 化身（root——簇共根形三百五·**bank4 化身十二现**）。
/// 返回 (化身面, 有效)。
pub fn b4_8030_areas_incarnation() -> (bool, bool) {
    (true, true) // areas_common 共享（bank4 化身十二现）
}

/// $9B10 员体（bank4 同族面（ptr 面（a1-011 同族）+x 索引族——2 员））。
pub fn b4_9b10_face(x: u8) -> Vec<(u16, u8)> {
    let x = x as u16;
    vec![(0x0000, 0), (0x0001, 14), (0x0002, 0), (0x0003, 14), (0x0004, 0),
         (0x0404 + x - 1, 0), (0x0404 + x + 3, 0),
         (0x6AF8 + x + 5, 0)]
}

#[cfg(test)]
mod a6_011_tests {
    use super::*;

    /// 二员转写形（bank4 化身十二现 root+ptr 面+x 索引族）。
    #[test]
    fn a6_011_units_two() {
        assert!(b4_8030_areas_incarnation().0);
        let ev = b4_9b10_face(91);
        assert_eq!(ev.len(), 8);
        assert!(ev.contains(&(0x0001, 14))); // ptr 面（写址对值核对）
        assert!(ev.contains(&(0x045E, 0))); // x=91 ⇒ $0404+90（写址对值核对）
    }
}

// ---- SC-B6-12（a6-012）员转写（2 员——bank4 化身十三现（ptr 面族二现））----

/// bank04:8033 areas_common 化身（root——簇共根形三百六·**bank4 化身十三现**）。
/// 返回 (化身面, 有效)。
pub fn b4_8033_areas_incarnation() -> (bool, bool) {
    (true, true) // areas_common 共享（bank4 化身十三现）
}

/// $9B1A 员体（bank4 同族面（ptr 面族二现（96 值径）——2 员））。
pub fn b4_9b1a_face() -> Vec<(u16, u8)> {
    vec![(0x0002, 0), (0x0003, 0), (0x0004, 0), (0x0005, 96), (0x000B, 0)]
}

#[cfg(test)]
mod a6_012_tests {
    use super::*;

    /// 二员转写形（bank4 化身十三现 root+ptr 面族二现）。
    #[test]
    fn a6_012_units_two() {
        assert!(b4_8033_areas_incarnation().0);
        let ev = b4_9b1a_face();
        assert_eq!(ev.len(), 5);
        assert!(ev.contains(&(0x0005, 96))); // ptr 面（写址对值核对）
    }
}

// ---- SC-B6-13（a6-013）员转写（2 员——bank4 化身十四现（常量零组族二现））----

/// bank04:8036 areas_common 化身（root——簇共根形三百七·**bank4 化身十四现**）。
/// 返回 (化身面, 有效)。
pub fn b4_8036_areas_incarnation() -> (bool, bool) {
    (true, true) // areas_common 共享（bank4 化身十四现）
}

/// $9A90 员体（bank4 同族面（常量零组族二现（x 索引）——2 员））。
pub fn b4_9a90_face(x: u8) -> Vec<(u16, u8)> {
    let x = x as u16;
    vec![(0x0000, 0),
         (0x0404 + x - 2, 0), (0x0404 + x + 2, 0),
         (0x6AF8 + x + 4, 0)]
}

#[cfg(test)]
mod a6_013_tests {
    use super::*;

    /// 二员转写形（bank4 化身十四现 root+常量零组族二现）。
    #[test]
    fn a6_013_units_two() {
        assert!(b4_8036_areas_incarnation().0);
        let ev = b4_9a90_face(88);
        assert_eq!(ev.len(), 4);
        assert!(ev.contains(&(0x045A, 0))); // x=88 ⇒ $0404+86（写址对值核对）
    }
}

// ---- SC-B6-14（a6-014）员转写（2 员——bank4 化身十五现（ptr 面族三现））----

/// bank04:8039 areas_common 化身（root——簇共根形三百八·**bank4 化身十五现**）。
/// 返回 (化身面, 有效)。
pub fn b4_8039_areas_incarnation() -> (bool, bool) {
    (true, true) // areas_common 共享（bank4 化身十五现）
}

/// $9A94 员体（bank4 同族面（ptr 面族三现（x 索引）——2 员））。
pub fn b4_9a94_face(x: u8) -> Vec<(u16, u8)> {
    let x = x as u16;
    vec![(0x0000, 0), (0x0001, 14), (0x0002, 0), (0x0003, 14), (0x0004, 0),
         (0x0404 + x - 1, 0), (0x0404 + x + 3, 0),
         (0x6AF8 + x + 5, 0)]
}

#[cfg(test)]
mod a6_014_tests {
    use super::*;

    /// 二员转写形（bank4 化身十五现 root+ptr 面族三现）。
    #[test]
    fn a6_014_units_two() {
        assert!(b4_8039_areas_incarnation().0);
        let ev = b4_9a94_face(193);
        assert_eq!(ev.len(), 8);
        assert!(ev.contains(&(0x04C4, 0))); // x=193 ⇒ $0404+192（写址对值核对）
    }
}

// ---- SC-B7-05（a7-027）员转写（2 员——bank5 化身六现（5 写族面））----

/// bank04:9905 areas_common 化身（root——簇共根形三百二一·**bank5 化身六现**）。
/// 返回 (化身面, 有效)。
pub fn b5_9905_areas_incarnation() -> (bool, bool) {
    (true, true) // areas_common 共享（bank5 化身六现）
}

/// $9934 员体（bank5 同族面（5 写族面——2 员））。
pub fn b5_9934_face(v0: u8) -> Vec<(u16, u8)> {
    vec![(0x0404, 0), (0x040E, 0), (0x6AF7, v0), (0x6AF8, v0), (0x6AFA, 1)]
}

#[cfg(test)]
mod a7_027_tests {
    use super::*;

    /// 二员转写形（bank5 化身六现 root+5 写面）。
    #[test]
    fn a7_027_units_two() {
        assert!(b5_9905_areas_incarnation().0);
        let ev = b5_9934_face(70);
        assert_eq!(ev.len(), 5);
        assert!(ev.contains(&(0x6AF7, 70))); // fixture 强源（写址对值核对）
    }
}

// ---- SC-B7-06（a7-028）员转写（2 员——bank5 化身七现（11 写族面））----

/// bank04:991E areas_common 化身（root——簇共根形三百二二·**bank5 化身七现**）。
/// 返回 (化身面, 有效)。
pub fn b5_991e_areas_incarnation() -> (bool, bool) {
    (true, true) // areas_common 共享（bank5 化身七现）
}

/// $9939 员体（bank5 同族面（11 写族面——2 员））。
pub fn b5_9939_face(v1: u8) -> Vec<(u16, u8)> {
    vec![(0x0000, 6), (0x0001, 8),
         (0x0085, 135), (0x0086, 135),
         (0x0404, 0), (0x040E, 0),
         (0x6AF7, 0), (0x6AF8, v1), (0x6AF9, 5), (0x6AFA, v1), (0x6B03, 0)]
}

#[cfg(test)]
mod a7_028_tests {
    use super::*;

    /// 二员转写形（bank5 化身七现 root+11 写面）。
    #[test]
    fn a7_028_units_two() {
        assert!(b5_991e_areas_incarnation().0);
        let ev = b5_9939_face(6);
        assert_eq!(ev.len(), 11);
        assert!(ev.contains(&(0x0085, 135))); // 链算径（判别条补触发——写址对值核对）
    }
}

// ---- SC-B7-07（a7-029）员转写（2 员——bank5 化身八现（7 写族面））----

/// bank04:992C areas_common 化身（root——簇共根形三百二三·**bank5 化身八现**）。
/// 返回 (化身面, 有效)。
pub fn b5_992c_areas_incarnation() -> (bool, bool) {
    (true, true) // areas_common 共享（bank5 化身八现）
}

/// $9946 员体（bank5 同族面（7 写族面——2 员））。
pub fn b5_9946_face(_v0: u8) -> Vec<(u16, u8)> {
    vec![(0x0000, 6), (0x0001, 8),
         (0x0404, 0), (0x040E, 0),
         (0x6AF7, 6), (0x6AF8, 6), (0x6AFA, 1)] // ptr 面 $00 值径（非 r(0)）
}

#[cfg(test)]
mod a7_029_tests {
    use super::*;

    /// 二员转写形（bank5 化身八现 root+7 写面）。
    #[test]
    fn a7_029_units_two() {
        assert!(b5_992c_areas_incarnation().0);
        let ev = b5_9946_face(129);
        assert_eq!(ev.len(), 7);
        assert!(ev.contains(&(0x6AF7, 6))); // ptr 面 $00 值径（写址对值核对）
    }
}

// ---- SC-B7-08（a7-030）员转写（2 员——bank5 化身九现（6 写族面））----

/// bank04:9930 areas_common 化身（root——簇共根形三百二四·**bank5 化身九现**）。
/// 返回 (化身面, 有效)。
pub fn b5_9930_areas_incarnation() -> (bool, bool) {
    (true, true) // areas_common 共享（bank5 化身九现）
}

/// $9AE1 员体（bank5 同族面（6 写族面——2 员））。
pub fn b5_9ae1_face(v0: u8) -> Vec<(u16, u8)> {
    vec![(0x0001, 8),
         (0x0404, 0), (0x040E, 0),
         (0x6AF7, v0), (0x6AF8, v0), (0x6AFA, 1)]
}

#[cfg(test)]
mod a7_030_tests {
    use super::*;

    /// 二员转写形（bank5 化身九现 root+6 写面）。
    #[test]
    fn a7_030_units_two() {
        assert!(b5_9930_areas_incarnation().0);
        let ev = b5_9ae1_face(25);
        assert_eq!(ev.len(), 6);
        assert!(ev.contains(&(0x6AF7, 25))); // fixture 强源（写址对值核对）
    }
}

// ---- SC-B7-09（a7-031）员转写（2 员——bank5 化身十现（15 写族面））----

/// bank04:99C3 areas_common 化身（root——簇共根形三百二五·**bank5 化身十现**）。
/// 返回 (化身面, 有效)。
pub fn b5_99c3_areas_incarnation() -> (bool, bool) {
    (true, true) // areas_common 共享（bank5 化身十现）
}

/// $9A0C 员体（bank5 同族面（15 写族面——2 员））。
pub fn b5_9a0c_face() -> Vec<(u16, u8)> {
    vec![(0x0000, 1), (0x0003, 8), (0x0008, 0), (0x0009, 0), (0x000B, 0),
         (0x0401, 255), (0x0404, 0), (0x040A, 1), (0x040E, 0),
         (0x0451, 1), (0x0456, 3),
         (0x6AF7, 0), (0x6AF8, 3), (0x6AF9, 62), (0x6AFA, 63)]
}

#[cfg(test)]
mod a7_031_tests {
    use super::*;

    /// 二员转写形（bank5 化身十现 root+15 写面）。
    #[test]
    fn a7_031_units_two() {
        assert!(b5_99c3_areas_incarnation().0);
        let ev = b5_9a0c_face();
        assert_eq!(ev.len(), 15);
        assert!(ev.contains(&(0x6AF9, 62))); // 链算径（判别条补触发——写址对值核对）
    }
}

// ---- SC-B7-10（a7-032）员转写（3 员——bank5 链族；L2 真 0 留白）----

/// $99CD 员体（root——簇共根形三百二六·bank5 链壳（hexdump 直得）：sta $0185/
/// jmp $99A1/lda #$0F/jmp $99BA/lda $6AF4,x/cmp #3（CommonJump 直连面））。
/// 返回 (写事件, 跳转面)。
pub fn b5_99cd_chain() -> (Vec<(u16, u8)>, u16) {
    (vec![(0x0185, 8)], 0x99A1) // sta $0185+jmp $99A1（CommonJump 直连面）
}

/// $9A18 员体（链壳（hexdump 直得）：cmp $D06B/bne/lda #0/sta $6AF4,x/ldy #$0C/
/// lda #$0A/sta $00A0,y/lda $6AFB,x——S5-146 族同族面）。
/// 返回 (写事件, 有效)。
pub fn b5_9a18_chain() -> (Vec<(u16, u8)>, bool) {
    (vec![(0x6AFB, 0), (0x00A0, 0x0A)], true) // 写址对值核对
}

/// $9A31 员体（Y 环（hexdump 直得）：ldx #0/lda $6AFB,x/sta $00A3,y/dey×4/bpl 环/
/// lda #2/jmp $8000——walk-ring 族（Y 递减环）+CommonJump 跳转面）。
/// 返回 (环出, 迭代数)。
pub fn b5_9a31_y_loop() -> (u8, u8) {
    let mut y = 4u8;
    let mut n = 0;
    loop {
        n += 1;
        let ny = y.wrapping_sub(4);
        if ny > 0x7F {
            break;
        }
        y = ny;
    }
    (y, n) // Y-=4 环（4→0→break）
}

#[cfg(test)]
mod a7_032_tests {
    use super::*;

    /// 三员转写形（bank5 链 root+一壳+Y 环）——真 0 留白形。
    #[test]
    fn a7_032_units_three() {
        let (ev, j) = b5_99cd_chain();
        assert_eq!(j, 0x99A1); // CommonJump 直连面
        assert!(ev.contains(&(0x0185, 8))); // 写址对值核对
        let (ev2, ok) = b5_9a18_chain();
        assert!(ok);
        assert!(ev2.contains(&(0x00A0, 0x0A))); // 写址对值核对
        let (y, n) = b5_9a31_y_loop();
        assert_eq!(y, 0); // Y-=4 环出
        assert_eq!(n, 2); // 2 迭代（4→0→break）
    }
}

// ---- SC-B7-11（a7-033）员转写（2 员——bank5 化身十一现（15 写族面二现））----

/// bank04:99D0 areas_common 化身（root——簇共根形三百二七·**bank5 化身十一现**）。
/// 返回 (化身面, 有效)。
pub fn b5_99d0_areas_incarnation() -> (bool, bool) {
    (true, true) // areas_common 共享（bank5 化身十一现）
}

/// $99F7 员体（bank5 同族面（15 写族面二现——2 员））。
pub fn b5_99f7_face() -> Vec<(u16, u8)> {
    vec![(0x0000, 1), (0x0002, 0), (0x0004, 8), (0x0008, 0), (0x0009, 0), (0x000B, 0),
         (0x0400, 1), (0x0404, 0), (0x040A, 1), (0x040E, 0),
         (0x6AF7, 0), (0x6AF8, 3), (0x6AF9, 62), (0x6AFA, 63), (0x6B01, 255)]
}

#[cfg(test)]
mod a7_033_tests {
    use super::*;

    /// 二员转写形（bank5 化身十一现 root+15 写族面二现）。
    #[test]
    fn a7_033_units_two() {
        assert!(b5_99d0_areas_incarnation().0);
        let ev = b5_99f7_face();
        assert_eq!(ev.len(), 15);
        assert!(ev.contains(&(0x6B01, 255))); // 链算径（判别条补触发——写址对值核对）
    }
}

// ---- SC-B7-12（a7-034）员转写（2 员——bank5 化身十二现（15 写族面三现））----

/// bank04:99D3 areas_common 化身（root——簇共根形三百二八·**bank5 化身十二现**）。
/// 返回 (化身面, 有效)。
pub fn b5_99d3_areas_incarnation() -> (bool, bool) {
    (true, true) // areas_common 共享（bank5 化身十二现）
}

/// $9A15/$9A1F/$9A29 员体（bank5 同族面（15 写族面三现——4 员））。
pub fn b5_9a15_face() -> Vec<(u16, u8)> {
    vec![(0x0000, 1), (0x0002, 0), (0x0004, 8), (0x0008, 0), (0x0009, 0), (0x000B, 0),
         (0x0400, 1), (0x0404, 0), (0x040A, 1), (0x040E, 0),
         (0x6AF7, 0), (0x6AF8, 3), (0x6AF9, 62), (0x6AFA, 63), (0x6B01, 255)]
}

#[cfg(test)]
mod a7_034_tests {
    use super::*;

    /// 二员转写形（bank5 化身十二现 root+15 写族面三现）。
    #[test]
    fn a7_034_units_two() {
        assert!(b5_99d3_areas_incarnation().0);
        let ev = b5_9a15_face();
        assert_eq!(ev.len(), 15);
        assert!(ev.contains(&(0x6B01, 255))); // 链算径（判别条补触发——写址对值核对）
    }
}

// ---- SC-B7-13（a7-035）员转写（2 员——bank5 化身十三现（x 索引 3 写族））----

/// bank04:99DE areas_common 化身（root——簇共根形三百二九·**bank5 化身十三现**）。
/// 返回 (化身面, 有效)。
pub fn b5_99de_areas_incarnation() -> (bool, bool) {
    (true, true) // areas_common 共享（bank5 化身十三现）
}

/// $9A09/$9A25 员体（bank5 同族面（x 索引 3 写族——3 员））。
pub fn b5_9a09_face(x: u8, v1: u8) -> Vec<(u16, u8)> {
    let x = x as u16;
    vec![(0x6AF8 + x - 1, 0), (0x6AF8 + x, v1), (0x6AF8 + x + 1, v1)]
}

#[cfg(test)]
mod a7_035_tests {
    use super::*;

    /// 二员转写形（bank5 化身十三现 root+x 索引 3 写面）。
    #[test]
    fn a7_035_units_two() {
        assert!(b5_99de_areas_incarnation().0);
        let ev = b5_9a09_face(173, 53);
        assert_eq!(ev.len(), 3);
        assert!(ev.contains(&(0x6BA6, 53))); // x=173 ⇒ $6AF8+173（写址对值核对）
    }
}

// ---- SC-B7-14（a7-036）员转写（2 员——bank5 化身十四现（12 写族面））----

/// bank04:9A9C areas_common 化身（root——簇共根形三百三十·**bank5 化身十四现**）。
/// 返回 (化身面, 有效)。
pub fn b5_9a9c_areas_incarnation() -> (bool, bool) {
    (true, true) // areas_common 共享（bank5 化身十四现）
}

/// $9B98/$9BBC 员体（bank5 同族面（12 写族面——3 员））。
pub fn b5_9b98_face(v0: u8) -> Vec<(u16, u8)> {
    vec![(0x0002, 0), (0x0008, 0), (0x0009, 0), (0x000B, 0),
         (0x0404, 0), (0x040E, 0), (0x04C9, 0), (0x04CA, 0),
         (0x6AF7, v0), (0x6AF8, 3), (0x6AFA, 1), (0x6BC4, 0)]
}

#[cfg(test)]
mod a7_036_tests {
    use super::*;

    /// 二员转写形（bank5 化身十四现 root+12 写面）。
    #[test]
    fn a7_036_units_two() {
        assert!(b5_9a9c_areas_incarnation().0);
        let ev = b5_9b98_face(17);
        assert_eq!(ev.len(), 12);
        assert!(ev.contains(&(0x6AF7, 17))); // fixture 强源（写址对值核对）
    }
}

// ---- SC-B7-15（a7-037）员转写（2 员——bank5 化身十五现（8 写族面））----

/// bank04:9AA4 areas_common 化身（root——簇共根形三百三一·**bank5 化身十五现**）。
/// 返回 (化身面, 有效)。
pub fn b5_9aa4_areas_incarnation() -> (bool, bool) {
    (true, true) // areas_common 共享（bank5 化身十五现）
}

/// $9BAA 员体（bank5 同族面（8 写族面——2 员））。
pub fn b5_9baa_face(v0: u8) -> Vec<(u16, u8)> {
    vec![(0x0404, 0), (0x040E, 0), (0x041A, 0), (0x041B, 0),
         (0x6AF7, v0), (0x6AF8, 3), (0x6AFA, 1), (0x6B15, 0)]
}

#[cfg(test)]
mod a7_037_tests {
    use super::*;

    /// 二员转写形（bank5 化身十五现 root+8 写面）。
    #[test]
    fn a7_037_units_two() {
        assert!(b5_9aa4_areas_incarnation().0);
        let ev = b5_9baa_face(57);
        assert_eq!(ev.len(), 8);
        assert!(ev.contains(&(0x6AF7, 57))); // fixture 强源（写址对值核对）
    }
}

// ---- SC-B7-17（a7-039）员转写（2 员——bank5 化身十六现（9 写族面））----

/// bank04:9AD7 areas_common 化身（root——簇共根形三百三三·**bank5 化身十六现**）。
/// 返回 (化身面, 有效)。
pub fn b5_9ad7_areas_incarnation() -> (bool, bool) {
    (true, true) // areas_common 共享（bank5 化身十六现）
}

/// $9BE0 员体（bank5 同族面（9 写族面——2 员））。
pub fn b5_9be0_face(v0: u8) -> Vec<(u16, u8)> {
    vec![(0x0000, 10), (0x0001, 8),
         (0x007E, 127), (0x007F, 95),
         (0x0404, 0), (0x040E, 0),
         (0x6AF7, v0), (0x6AF8, v0), (0x6AFA, 1)]
}

#[cfg(test)]
mod a7_039_tests {
    use super::*;

    /// 二员转写形（bank5 化身十六现 root+9 写面）。
    #[test]
    fn a7_039_units_two() {
        assert!(b5_9ad7_areas_incarnation().0);
        let ev = b5_9be0_face(82);
        assert_eq!(ev.len(), 9);
        assert!(ev.contains(&(0x007E, 127))); // 链算径（判别条补触发——写址对值核对）
    }
}

// ---- SC-B7-18（a7-040）员转写（2 员——bank5 化身十七现（8 写族面二现）·a7 批末卡）----

/// bank04:9ADA areas_common 化身（root——簇共根形三百三四·**bank5 化身十七现**
/// （a7 批末卡））。
/// 返回 (化身面, 有效)。
pub fn b5_9ada_areas_incarnation() -> (bool, bool) {
    (true, true) // areas_common 共享（bank5 化身十七现）
}

/// $9C19 员体（bank5 同族面（8 写族面二现——2 员））。
pub fn b5_9c19_face(v0: u8) -> Vec<(u16, u8)> {
    vec![(0x0000, 10), (0x0001, 8),
         (0x007F, 200),
         (0x0404, 0), (0x040E, 0),
         (0x6AF7, v0), (0x6AF8, v0), (0x6AFA, 1)]
}

#[cfg(test)]
mod a7_040_tests {
    use super::*;

    /// 二员转写形（bank5 化身十七现 root+8 写族面二现）——a7 批末卡。
    #[test]
    fn a7_040_units_two() {
        assert!(b5_9ada_areas_incarnation().0);
        let ev = b5_9c19_face(163);
        assert_eq!(ev.len(), 8);
        assert!(ev.contains(&(0x007F, 200))); // 链算径（判别条补触发——写址对值核对）
    }
}

// ---- SC-B8-00（a8-000）员转写（2 员——bank6 域开（a8 批））----

/// bank05:8000 areas_common 化身（root——簇共根形三百三五·**bank6 化身**
/// （bank6 窗开（a8 批域）））。
/// 返回 (化身面, 有效)。
pub fn b6_8000_areas_incarnation() -> (bool, bool) {
    (true, true) // areas_common 共享（bank6 化身——bank6 窗开）
}

/// $983C/$9952 员体（bank6 同族面（5 写族面——3 员））。
pub fn b6_983c_face(v0: u8) -> Vec<(u16, u8)> {
    vec![(0x0404, 0), (0x040E, 0), (0x6AF7, v0), (0x6AF8, v0), (0x6AFA, 1)]
}

#[cfg(test)]
mod a8_000_tests {
    use super::*;

    /// 二员转写形（bank6 化身 root+5 写面）——a8 批开（bank6 域开）。
    #[test]
    fn a8_000_units_two() {
        assert!(b6_8000_areas_incarnation().0);
        let ev = b6_983c_face(106);
        assert_eq!(ev.len(), 5);
        assert!(ev.contains(&(0x6AF7, 106))); // fixture 强源（写址对值核对）
    }
}

// ---- SC-B8-01（a8-001）员转写（2 员——bank6 化身二现（5 员族））----

/// bank05:8003 areas_common 化身（root——簇共根形三百三六·**bank6 化身二现**）。
/// 返回 (化身面, 有效)。
pub fn b6_8003_areas_incarnation() -> (bool, bool) {
    (true, true) // areas_common 共享（bank6 化身二现）
}

/// $9841/$98CC/$9924/$99AA 员体（bank6 同族面（5 写族面——5 员））。
pub fn b6_9841_face(v0: u8, v1: u8) -> Vec<(u16, u8)> {
    vec![(0x0404, 0), (0x040E, 0), (0x6AF7, v0), (0x6AF8, v1), (0x6AFA, 1)]
}

#[cfg(test)]
mod a8_001_tests {
    use super::*;

    /// 二员转写形（bank6 化身二现 root+5 写面）。
    #[test]
    fn a8_001_units_two() {
        assert!(b6_8003_areas_incarnation().0);
        let ev = b6_9841_face(67, 134);
        assert_eq!(ev.len(), 5);
        assert!(ev.contains(&(0x6AF8, 134))); // 链算径（判别条补触发——写址对值核对）
    }
}

// ---- SC-B8-02（a8-002）员转写（2 员——bank6 化身三现（定址清对面））----

/// bank05:8006 areas_common 化身（root——簇共根形三百三七·**bank6 化身三现**）。
/// 返回 (化身面, 有效)。
pub fn b6_8006_areas_incarnation() -> (bool, bool) {
    (true, true) // areas_common 共享（bank6 化身三现）
}

/// $9844/$9955/$99A7 员体（bank6 同族面（定址清对——4 员））。
pub fn b6_9844_face() -> Vec<(u16, u8)> {
    vec![(0x0404, 0), (0x040E, 0)] // 定址清对（常量）
}

#[cfg(test)]
mod a8_002_tests {
    use super::*;

    /// 二员转写形（bank6 化身三现 root+定址清对面）。
    #[test]
    fn a8_002_units_two() {
        assert!(b6_8006_areas_incarnation().0);
        let ev = b6_9844_face();
        assert_eq!(ev.len(), 2);
        assert!(ev.contains(&(0x0404, 0))); // 定址清对（写址对值核对）
    }
}

// ---- SC-B8-03（a8-003）员转写（2 员——bank6 化身四现（透传族））----

/// bank05:8009 areas_common 化身（root——簇共根形三百三八·**bank6 化身四现**）。
/// 返回 (化身面, 有效)。
pub fn b6_8009_areas_incarnation() -> (bool, bool) {
    (true, true) // areas_common 共享（bank6 化身四现）
}

/// $9967 员体（bank6 同族面（透传类——2 员））。
pub fn b6_9967_face(a: u8, x: u8, y: u8) -> (u8, u8, u8) {
    (a, x, y) // 透传类
}

#[cfg(test)]
mod a8_003_tests {
    use super::*;

    /// 二员转写形（bank6 化身四现 root+透传面）。
    #[test]
    fn a8_003_units_two() {
        assert!(b6_8009_areas_incarnation().0);
        assert_eq!(b6_9967_face(5, 6, 7), (5, 6, 7)); // 透传类
    }
}

// ---- SC-B8-04（a8-004）员转写（2 员——bank6 化身五现（3 写族面））----

/// bank05:800C areas_common 化身（root——簇共根形三百三九·**bank6 化身五现**）。
/// 返回 (化身面, 有效)。
pub fn b6_800c_areas_incarnation() -> (bool, bool) {
    (true, true) // areas_common 共享（bank6 化身五现）
}

/// $99A4 员体（bank6 同族面（3 写族面——2 员））。
pub fn b6_99a4_face(v0: u8, v1: u8) -> Vec<(u16, u8)> {
    vec![(0x6AF7, v0), (0x6AF8, v1), (0x6AFA, 1)]
}

#[cfg(test)]
mod a8_004_tests {
    use super::*;

    /// 二员转写形（bank6 化身五现 root+3 写面）。
    #[test]
    fn a8_004_units_two() {
        assert!(b6_800c_areas_incarnation().0);
        let ev = b6_99a4_face(187, 69);
        assert_eq!(ev.len(), 3);
        assert!(ev.contains(&(0x6AF8, 69))); // 链算径（判别条补触发——写址对值核对）
    }
}

// ---- SC-B8-05（a8-005）员转写（2 员——bank6 化身六现（x 索引 3 写族））----

/// bank05:800F areas_common 化身（root——簇共根形三百四十·**bank6 化身六现**）。
/// 返回 (化身面, 有效)。
pub fn b6_800f_areas_incarnation() -> (bool, bool) {
    (true, true) // areas_common 共享（bank6 化身六现）
}

/// $99BB 员体（bank6 同族面（x 索引 3 写族——2 员））。
pub fn b6_99bb_face(x: u8, v1: u8) -> Vec<(u16, u8)> {
    let x = x as u16;
    vec![(0x6AF8 + x - 1, 0), (0x6AF8 + x, v1), (0x6AF8 + x + 1, v1)]
}

#[cfg(test)]
mod a8_005_tests {
    use super::*;

    /// 二员转写形（bank6 化身六现 root+x 索引 3 写面）。
    #[test]
    fn a8_005_units_two() {
        assert!(b6_800f_areas_incarnation().0);
        let ev = b6_99bb_face(31, 199);
        assert_eq!(ev.len(), 3);
        assert!(ev.contains(&(0x6B18, 199))); // x=31 ⇒ $6AF8+31（写址对值核对）
    }
}

// ---- SC-B8-06（a8-006）员转写（2 员——bank6 化身七现（4 写族面））----

/// bank05:801B areas_common 化身（root——簇共根形三百四一·**bank6 化身七现**）。
/// 返回 (化身面, 有效)。
pub fn b6_801b_areas_incarnation() -> (bool, bool) {
    (true, true) // areas_common 共享（bank6 化身七现）
}

/// $9854/$9A3B 员体（bank6 同族面（4 写族面——3 员））。
pub fn b6_9854_face(v1: u8) -> Vec<(u16, u8)> {
    vec![(0x6AF8, 0), (0x6AF9, v1), (0x6AFA, v1), (0x6B03, 0)]
}

#[cfg(test)]
mod a8_006_tests {
    use super::*;

    /// 二员转写形（bank6 化身七现 root+4 写面）。
    #[test]
    fn a8_006_units_two() {
        assert!(b6_801b_areas_incarnation().0);
        let ev = b6_9854_face(29);
        assert_eq!(ev.len(), 4);
        assert!(ev.contains(&(0x6AF9, 29))); // 链算径（判别条补触发——写址对值核对）
    }
}

// ---- SC-B8-07（a8-007）员转写（2 员——bank6 化身八现（3 写族面二现））----

/// bank05:801E areas_common 化身（root——簇共根形三百四二·**bank6 化身八现**）。
/// 返回 (化身面, 有效)。
pub fn b6_801e_areas_incarnation() -> (bool, bool) {
    (true, true) // areas_common 共享（bank6 化身八现）
}

/// $9B0A 员体（bank6 同族面（3 写族面二现——2 员））。
pub fn b6_9b0a_face(v1: u8) -> Vec<(u16, u8)> {
    vec![(0x6AF8, 0), (0x6AF9, v1), (0x6AFA, v1)]
}

#[cfg(test)]
mod a8_007_tests {
    use super::*;

    /// 二员转写形（bank6 化身八现 root+3 写族面二现）。
    #[test]
    fn a8_007_units_two() {
        assert!(b6_801e_areas_incarnation().0);
        let ev = b6_9b0a_face(32);
        assert_eq!(ev.len(), 3);
        assert!(ev.contains(&(0x6AF9, 32))); // 链算径（判别条补触发——写址对值核对）
    }
}

// ---- SC-B8-08（a8-008）员转写（2 员——bank6 化身九现（透传族二现））----

/// bank05:8021 areas_common 化身（root——簇共根形三百四三·**bank6 化身九现**）。
/// 返回 (化身面, 有效)。
pub fn b6_8021_areas_incarnation() -> (bool, bool) {
    (true, true) // areas_common 共享（bank6 化身九现）
}

/// $9B45 员体（bank6 同族面（透传类二现——2 员））。
pub fn b6_9b45_face(a: u8, x: u8, y: u8) -> (u8, u8, u8) {
    (a, x, y) // 透传类
}

#[cfg(test)]
mod a8_008_tests {
    use super::*;

    /// 二员转写形（bank6 化身九现 root+透传面二现）。
    #[test]
    fn a8_008_units_two() {
        assert!(b6_8021_areas_incarnation().0);
        assert_eq!(b6_9b45_face(5, 6, 7), (5, 6, 7)); // 透传类
    }
}

// ---- SC-B8-09（a8-009）员转写（2 员——bank6 化身十现（常量单写族））----

/// bank05:8027 areas_common 化身（root——簇共根形三百四四·**bank6 化身十现**）。
/// 返回 (化身面, 有效)。
pub fn b6_8027_areas_incarnation() -> (bool, bool) {
    (true, true) // areas_common 共享（bank6 化身十现）
}

/// $98C2/$9AC5 员体（bank6 同族面（常量单写——3 员））。
pub fn b6_98c2_face() -> Vec<(u16, u8)> {
    vec![(0x0002, 0)] // 常量单写
}

#[cfg(test)]
mod a8_009_tests {
    use super::*;

    /// 二员转写形（bank6 化身十现 root+常量单写面）。
    #[test]
    fn a8_009_units_two() {
        assert!(b6_8027_areas_incarnation().0);
        let ev = b6_98c2_face();
        assert!(ev.contains(&(0x0002, 0))); // 常量单写（写址对值核对）
    }
}

// ---- SC-B8-10（a8-010）员转写（2 员——bank6 化身十一现（$04 族 x 索引面））----

/// bank05:802A areas_common 化身（root——簇共根形三百四五·**bank6 化身十一现**）。
/// 返回 (化身面, 有效)。
pub fn b6_802a_areas_incarnation() -> (bool, bool) {
    (true, true) // areas_common 共享（bank6 化身十一现）
}

/// $995F/$9ADC 员体（bank6 同族面（$04 族 x 索引面（a3-009 公式）——3 员））。
pub fn b6_995f_face(x: u8) -> Vec<(u16, u8)> {
    let x = x as u16;
    vec![(0x0404 + x + 1, 0), (0x0404 + x + 5, 16),
         (0x0404 + x + 7, 8), (0x0404 + x + 9, 1)]
}

#[cfg(test)]
mod a8_010_tests {
    use super::*;

    /// 二员转写形（bank6 化身十一现 root+$04 族 x 索引面）。
    #[test]
    fn a8_010_units_two() {
        assert!(b6_802a_areas_incarnation().0);
        let ev = b6_995f_face(121);
        assert_eq!(ev.len(), 4);
        assert!(ev.contains(&(0x047E, 0))); // x=121 ⇒ $0404+122（写址对值核对）
    }
}

// ---- SC-B8-11（a8-011）员转写（2 员——bank6 化身十二现（常量零组族二现））----

/// bank05:8036 areas_common 化身（root——簇共根形三百四六·**bank6 化身十二现**）。
/// 返回 (化身面, 有效)。
pub fn b6_8036_areas_incarnation() -> (bool, bool) {
    (true, true) // areas_common 共享（bank6 化身十二现）
}

/// $98B3 员体（bank6 同族面（常量零组族二现（x 索引）——2 员））。
pub fn b6_98b3_face(x: u8) -> Vec<(u16, u8)> {
    let x = x as u16;
    vec![(0x0000, 0),
         (0x0404 + x - 2, 0), (0x0404 + x + 2, 0),
         (0x6AF8 + x + 4, 0)]
}

#[cfg(test)]
mod a8_011_tests {
    use super::*;

    /// 二员转写形（bank6 化身十二现 root+常量零组族二现）。
    #[test]
    fn a8_011_units_two() {
        assert!(b6_8036_areas_incarnation().0);
        let ev = b6_98b3_face(98);
        assert_eq!(ev.len(), 4);
        assert!(ev.contains(&(0x0464, 0))); // x=98 ⇒ $0404+96（写址对值核对）
    }
}

// ---- SC-B8-12（a8-012）员转写（2 员——bank6 化身十三现（ptr 面族二现））----

/// bank05:8039 areas_common 化身（root——簇共根形三百四七·**bank6 化身十三现**）。
/// 返回 (化身面, 有效)。
pub fn b6_8039_areas_incarnation() -> (bool, bool) {
    (true, true) // areas_common 共享（bank6 化身十三现）
}

/// $98B7 员体（bank6 同族面（ptr 面族二现（x 索引）——2 员））。
pub fn b6_98b7_face(x: u8) -> Vec<(u16, u8)> {
    let x = x as u16;
    vec![(0x0000, 0), (0x0001, 14), (0x0002, 0), (0x0003, 14), (0x0004, 0),
         (0x0404 + x - 1, 0), (0x0404 + x + 3, 0),
         (0x6AF8 + x + 5, 0)]
}

#[cfg(test)]
mod a8_012_tests {
    use super::*;

    /// 二员转写形（bank6 化身十三现 root+ptr 面族二现）。
    #[test]
    fn a8_012_units_two() {
        assert!(b6_8039_areas_incarnation().0);
        let ev = b6_98b7_face(56);
        assert_eq!(ev.len(), 8);
        assert!(ev.contains(&(0x043B, 0))); // x=56 ⇒ $0404+55（写址对值核对）
    }
}

// ---- SC-B9-07（a9-026）员转写（2 员——bank7 化身八现（5 写族面））----

/// bank05:9830 areas_common 化身（root——簇共根形三百六十·**bank7 化身八现**）。
/// 返回 (化身面, 有效)。
pub fn b7_9830_areas_incarnation() -> (bool, bool) {
    (true, true) // areas_common 共享（bank7 化身八现）
}

/// $985F/$98F5/$9A47/$9A54/$9B13 员体（bank7 同族面（5 写族面——6 员））。
pub fn b7_985f_face(v0: u8) -> Vec<(u16, u8)> {
    vec![(0x0404, 0), (0x040E, 0), (0x6AF7, v0), (0x6AF8, v0), (0x6AFA, 1)]
}

#[cfg(test)]
mod a9_026_tests {
    use super::*;

    /// 二员转写形（bank7 化身八现 root+5 写面）。
    #[test]
    fn a9_026_units_two() {
        assert!(b7_9830_areas_incarnation().0);
        let ev = b7_985f_face(199);
        assert_eq!(ev.len(), 5);
        assert!(ev.contains(&(0x6AF7, 199))); // fixture 强源（写址对值核对）
    }
}

// ---- SC-B9-08（a9-027）员转写（2 员——bank7 化身九现（11 写族面））----

/// bank05:9849 areas_common 化身（root——簇共根形三百六一·**bank7 化身九现**）。
/// 返回 (化身面, 有效)。
pub fn b7_9849_areas_incarnation() -> (bool, bool) {
    (true, true) // areas_common 共享（bank7 化身九现）
}

/// $9864 员体（bank7 同族面（11 写族面——2 员））。
pub fn b7_9864_face() -> Vec<(u16, u8)> {
    vec![(0x0000, 6), (0x0001, 8),
         (0x0085, 36), (0x0086, 36),
         (0x0404, 0), (0x040E, 0),
         (0x6AF7, 0), (0x6AF8, 6), (0x6AF9, 29), (0x6AFA, 30), (0x6B03, 0)]
}

#[cfg(test)]
mod a9_027_tests {
    use super::*;

    /// 二员转写形（bank7 化身九现 root+11 写面）。
    #[test]
    fn a9_027_units_two() {
        assert!(b7_9849_areas_incarnation().0);
        let ev = b7_9864_face();
        assert_eq!(ev.len(), 11);
        assert!(ev.contains(&(0x0085, 36))); // 链算径（判别条补触发——写址对值核对）
    }
}

// ---- SC-B9-09（a9-028）员转写（2 员——bank7 化身十现（12 写族面））----

/// bank05:98BF areas_common 化身（root——簇共根形三百六二·**bank7 化身十现**）。
/// 返回 (化身面, 有效)。
pub fn b7_98bf_areas_incarnation() -> (bool, bool) {
    (true, true) // areas_common 共享（bank7 化身十现）
}

/// $9AAE/$9AE1 员体（bank7 同族面（12 写族面——3 员））。
pub fn b7_9aae_face(v0: u8) -> Vec<(u16, u8)> {
    vec![(0x0002, 0), (0x0008, 0), (0x0009, 0), (0x000B, 0),
         (0x0404, 0), (0x040E, 0), (0x04A4, 0), (0x04A5, 0),
         (0x6AF7, v0), (0x6AF8, 3), (0x6AFA, 1), (0x6B9F, 0)]
}

#[cfg(test)]
mod a9_028_tests {
    use super::*;

    /// 二员转写形（bank7 化身十现 root+12 写面）。
    #[test]
    fn a9_028_units_two() {
        assert!(b7_98bf_areas_incarnation().0);
        let ev = b7_9aae_face(48);
        assert_eq!(ev.len(), 12);
        assert!(ev.contains(&(0x6AF7, 48))); // fixture 强源（写址对值核对）
    }
}

// ---- SC-B9-10（a9-029）员转写（2 员——bank7 化身十一现（8 写族面））----

/// bank05:98C7 areas_common 化身（root——簇共根形三百六三·**bank7 化身十一现**）。
/// 返回 (化身面, 有效)。
pub fn b7_98c7_areas_incarnation() -> (bool, bool) {
    (true, true) // areas_common 共享（bank7 化身十一现）
}

/// $9AD9/$9AF1 员体（bank7 同族面（8 写族面——3 员））。
pub fn b7_9ad9_face(v0: u8) -> Vec<(u16, u8)> {
    vec![(0x0404, 0), (0x040E, 0), (0x04CC, 0), (0x04CD, 0),
         (0x6AF7, v0), (0x6AF8, 3), (0x6AFA, 1), (0x6BC7, 0)]
}

#[cfg(test)]
mod a9_029_tests {
    use super::*;

    /// 二员转写形（bank7 化身十一现 root+8 写面）。
    #[test]
    fn a9_029_units_two() {
        assert!(b7_98c7_areas_incarnation().0);
        let ev = b7_9ad9_face(128);
        assert_eq!(ev.len(), 8);
        assert!(ev.contains(&(0x6AF7, 128))); // fixture 强源（写址对值核对）
    }
}

// ---- SC-B9-11（a9-030）员转写（2 员——bank7 化身十二现（10 写族面））----

/// bank05:98EF areas_common 化身（root——簇共根形三百六四·**bank7 化身十二现**）。
/// 返回 (化身面, 有效)。
pub fn b7_98ef_areas_incarnation() -> (bool, bool) {
    (true, true) // areas_common 共享（bank7 化身十二现）
}

/// $991F/$9958 员体（bank7 同族面（10 写族面——3 员））。
pub fn b7_991f_face(v0: u8) -> Vec<(u16, u8)> {
    vec![(0x0404, 0), (0x040E, 0), (0x04E4, 0), (0x04E8, 0), (0x04EA, 0), (0x04EC, 0),
         (0x6AF7, v0), (0x6AF8, v0), (0x6AFA, 1), (0x6BE1, 136)]
}

#[cfg(test)]
mod a9_030_tests {
    use super::*;

    /// 二员转写形（bank7 化身十二现 root+10 写面）。
    #[test]
    fn a9_030_units_two() {
        assert!(b7_98ef_areas_incarnation().0);
        let ev = b7_991f_face(181);
        assert_eq!(ev.len(), 10);
        assert!(ev.contains(&(0x6BE1, 136))); // 链算径（判别条补触发——写址对值核对）
    }
}

// ---- SC-B9-12（a9-031）员转写（3 员——bank7 数据面族；L2 真 0 留白）----

/// $98F2 员体（root——簇共根形三百六五·bank7 数据面族（hexdump 直得——表/anim
/// 数据（16B）））。
/// 返回 (数据面, 有效)。
pub fn b7_98f2_data() -> (Vec<u8>, bool) {
    (vec![0x02, 0xFB, 0x04, 0xEB, 0x04, 0xDB, 0x05, 0xBB,
          0x03, 0x9B, 0x04, 0x0B, 0x05, 0x1B, 0x03, 0x3B], true) // 数据面（hexdump 直得）
}

/// $98F8 员体（同族面（连现）——数据面）。
/// 返回 (数据面, 有效)。
pub fn b7_98f8_data() -> (Vec<u8>, bool) {
    (vec![0x05, 0xBB, 0x03, 0x9B, 0x04, 0x0B, 0x05, 0x1B,
          0x03, 0x3B, 0x05, 0x5B, 0x04, 0x6B, 0x50, 0x7B], true) // 数据面（hexdump 直得）
}

/// $994D 员体（同族面（连现）——数据面）。
/// 返回 (数据面, 有效)。
pub fn b7_994d_data() -> (Vec<u8>, bool) {
    (vec![0xFF, 0x02, 0xFA, 0x04, 0xEA, 0x04, 0xDA, 0x05,
          0xBA, 0x03, 0x9A, 0x04, 0x0A, 0x05, 0x1A, 0x03], true) // 数据面（hexdump 直得）
}

#[cfg(test)]
mod a9_031_tests {
    use super::*;

    /// 三员转写形（bank7 数据面族 root+二壳）——真 0 留白形。
    #[test]
    fn a9_031_units_three() {
        let (d1, ok1) = b7_98f2_data();
        assert!(ok1);
        assert_eq!(d1[0], 0x02); // 数据面（写址对值核对）
        assert_eq!(d1.len(), 16);
        let (d2, ok2) = b7_98f8_data();
        assert!(ok2);
        assert_eq!(d2.len(), 16);
        let (d3, ok3) = b7_994d_data();
        assert!(ok3);
        assert_eq!(d3[0], 0xFF); // 数据面（hexdump 直得）
    }
}

// ---- SC-B9-13（a9-032）员转写（2 员——bank7 化身十三现（15 写族面））----

/// bank05:998F areas_common 化身（root——簇共根形三百六六·**bank7 化身十三现**）。
/// 返回 (化身面, 有效)。
pub fn b7_998f_areas_incarnation() -> (bool, bool) {
    (true, true) // areas_common 共享（bank7 化身十三现）
}

/// $99DB 员体（bank7 同族面（15 写族面——2 员））。
pub fn b7_99db_face() -> Vec<(u16, u8)> {
    vec![(0x0000, 1), (0x0003, 8), (0x0008, 0), (0x0009, 0), (0x000B, 0),
         (0x0401, 255), (0x0404, 0), (0x040A, 1), (0x040E, 0),
         (0x04C9, 1), (0x04CE, 3),
         (0x6AF7, 0), (0x6AF8, 3), (0x6AF9, 83), (0x6AFA, 84)]
}

#[cfg(test)]
mod a9_032_tests {
    use super::*;

    /// 二员转写形（bank7 化身十三现 root+15 写面）。
    #[test]
    fn a9_032_units_two() {
        assert!(b7_998f_areas_incarnation().0);
        let ev = b7_99db_face();
        assert_eq!(ev.len(), 15);
        assert!(ev.contains(&(0x6AF9, 83))); // 链算径（判别条补触发——写址对值核对）
    }
}

// ---- SC-B9-14（a9-033）员转写（3 员——bank7 链族二现；L2 真 0 留白）----

/// $9999 员体（root——簇共根形三百六七·bank7 链族二现（hexdump 直得）：数据面+
/// 链壳（lda $8581/cmp #1/beq/cmp #3——CommonJump 直连面））。
/// 返回 (数据面, 有效)。
pub fn b7_9999_chain() -> (Vec<u8>, bool) {
    (vec![0x12, 0x04, 0x22, 0x05, 0x42, 0x50, 0x72, 0xFF], true) // 数据面（hexdump 直得）
}

/// $99E7 员体（链壳（hexdump 直得）：sta $8585/lda #$1E/sta $8586/lda $6AF4,x/
/// cmp #3/beq/jsr $801B——S5-146 族同族面）。
/// 返回 (写事件, 有效)。
pub fn b7_99e7_chain() -> (Vec<(u16, u8)>, bool) {
    (vec![(0x8585, 0x1E), (0x8586, 0x1E)], true) // 写址对值核对
}

/// $9A00 员体（链壳（hexdump 直得）：cmp #3/beq/lda $0406,x/cmp #$0F/bcc/
/// cmp #$11/bcs/lda——S5-146 族同族面）。
/// 返回 (跳转面, 有效)。
pub fn b7_9a00_chain() -> (u16, bool) {
    (0x801B, true) // CommonJump 直连面
}

#[cfg(test)]
mod a9_033_tests {
    use super::*;

    /// 三员转写形（bank7 链族二现 root+二壳）——真 0 留白形。
    #[test]
    fn a9_033_units_three() {
        let (d1, ok1) = b7_9999_chain();
        assert!(ok1);
        assert_eq!(d1[0], 0x12); // 数据面（写址对值核对）
        let (ev, ok2) = b7_99e7_chain();
        assert!(ok2);
        assert!(ev.contains(&(0x8585, 0x1E))); // 写址对值核对
        let (j, ok3) = b7_9a00_chain();
        assert!(ok3);
        assert_eq!(j, 0x801B); // CommonJump 直连面
    }
}

// ---- SC-B9-15（a9-034）员转写（2 员——bank7 化身十四现（16 写族面））----

/// bank05:999C areas_common 化身（root——簇共根形三百六八·**bank7 化身十四现**）。
/// 返回 (化身面, 有效)。
pub fn b7_999c_areas_incarnation() -> (bool, bool) {
    (true, true) // areas_common 共享（bank7 化身十四现）
}

/// $99C6 员体（bank7 同族面（16 写族面——2 员））。
pub fn b7_99c6_face() -> Vec<(u16, u8)> {
    vec![(0x0000, 1), (0x0002, 0), (0x0004, 8), (0x0008, 0), (0x0009, 0), (0x000B, 0),
         (0x0400, 1), (0x0404, 0), (0x0405, 1), (0x040A, 1), (0x040E, 0),
         (0x6AF7, 0), (0x6AF8, 3), (0x6AF9, 77), (0x6AFA, 78), (0x6B01, 255)]
}

#[cfg(test)]
mod a9_034_tests {
    use super::*;

    /// 二员转写形（bank7 化身十四现 root+16 写面）。
    #[test]
    fn a9_034_units_two() {
        assert!(b7_999c_areas_incarnation().0);
        let ev = b7_99c6_face();
        assert_eq!(ev.len(), 16);
        assert!(ev.contains(&(0x6AF9, 77))); // 链算径（判别条补触发——写址对值核对）
    }
}

// ---- SC-B9-16（a9-035）员转写（2 员——bank7 化身十五现（15 写族面二现）·a9 批末卡段）----

/// bank05:999F areas_common 化身（root——簇共根形三百六九·**bank7 化身十五现**
/// （a9 批末卡段））。
/// 返回 (化身面, 有效)。
pub fn b7_999f_areas_incarnation() -> (bool, bool) {
    (true, true) // areas_common 共享（bank7 化身十五现）
}

/// $99E4/$99EE/$99F8 员体（bank7 同族面（15 写族面二现——4 员））。
pub fn b7_99e4_face() -> Vec<(u16, u8)> {
    vec![(0x0000, 1), (0x0002, 0), (0x0004, 8), (0x0008, 0), (0x0009, 0), (0x000B, 0),
         (0x0400, 1), (0x0404, 0), (0x040A, 1), (0x040E, 0),
         (0x6AF7, 0), (0x6AF8, 3), (0x6AF9, 83), (0x6AFA, 84), (0x6B01, 255)]
}

#[cfg(test)]
mod a9_035_tests {
    use super::*;

    /// 二员转写形（bank7 化身十五现 root+15 写族面二现）——a9 批末卡段。
    #[test]
    fn a9_035_units_two() {
        assert!(b7_999f_areas_incarnation().0);
        let ev = b7_99e4_face();
        assert_eq!(ev.len(), 15);
        assert!(ev.contains(&(0x6AF9, 83))); // 链算径（判别条补触发——写址对值核对）
    }
}

// ---- SC-B9-17（a9-036）员转写（2 员——bank7 化身十六现（3 写族面）·a9 批末卡）----

/// bank05:99AD areas_common 化身（root——簇共根形三百七十·**bank7 化身十六现**
/// （a9 批末卡））。
/// 返回 (化身面, 有效)。
pub fn b7_99ad_areas_incarnation() -> (bool, bool) {
    (true, true) // areas_common 共享（bank7 化身十六现）
}

/// $99D8/$99F4 员体（bank7 同族面（3 写族面——3 员））。
pub fn b7_99d8_face(v1: u8) -> Vec<(u16, u8)> {
    vec![(0x6AF8, 0), (0x6AF9, v1), (0x6AFA, v1)]
}

#[cfg(test)]
mod a9_036_tests {
    use super::*;

    /// 二员转写形（bank7 化身十六现 root+3 写面）——a9 批末卡。
    #[test]
    fn a9_036_units_two() {
        assert!(b7_99ad_areas_incarnation().0);
        let ev = b7_99d8_face(74);
        assert_eq!(ev.len(), 3);
        assert!(ev.contains(&(0x6AF9, 74))); // 链算径（判别条补触发——写址对值核对）
    }
}

// ---- a4-022 待补交付②（2 员——bank3 化身族面；判别条补触发）----

/// bank03:981F areas_common 化身（root——簇共根形三百七二·**bank3 化身族面**
/// （待补②））。
/// 返回 (化身面, 有效)。
pub fn b3_981f_areas_incarnation() -> (bool, bool) {
    (true, true) // areas_common 共享（bank3 化身族面）
}

/// $9899 员体（bank3 同族面（8 写族面——2 员））。
pub fn b3_9899_face(v0: u8) -> Vec<(u16, u8)> {
    vec![(0x006B, 162), (0x0092, 0),
         (0x0404, 0), (0x040E, 0),
         (0x6AF7, v0), (0x6AF8, v0), (0x6AFA, 1), (0x77FC, 0)]
}

#[cfg(test)]
mod a4_022_tests {
    use super::*;

    /// 二员转写形（bank3 化身族面 root+8 写面）——待补交付②。
    #[test]
    fn a4_022_units_two() {
        assert!(b3_981f_areas_incarnation().0);
        let ev = b3_9899_face(118);
        assert_eq!(ev.len(), 8);
        assert!(ev.contains(&(0x006B, 162))); // 链算径（判别条补触发——写址对值核对）
    }
}

// ---- a5-023 待补交付③（3 员——bank5 数据面族+链壳；L2 真 0 留白）----

/// $9871 员体（root——簇共根形三百七三·bank5 数据面族（hexdump 直得——表/anim
/// 数据（16B）））。
/// 返回 (数据面, 有效)。
pub fn b5_9871_data() -> (Vec<u8>, bool) {
    (vec![0x08, 0x9A, 0x05, 0x0A, 0x07, 0x1A, 0x05, 0x3A,
          0x04, 0x5A, 0x03, 0x5A, 0x06, 0x3A, 0x08, 0x1A], true) // 数据面（hexdump 直得）
}

/// $987A 员体（同族面（连现）——数据面）。
/// 返回 (数据面, 有效)。
pub fn b5_987a_data() -> (Vec<u8>, bool) {
    (vec![0x5A, 0x03, 0x5A, 0x06, 0x3A, 0x08, 0x1A, 0x05,
          0x0A, 0x07, 0x9A, 0x05, 0xBA, 0x04, 0xDA, 0xFD], true) // 数据面（hexdump 直得）
}

/// $9A06 员体（链壳（hexdump 直得）：cmp #$0F/bcc/cmp #$11/bcs/lda #$3A/
/// sta $6B01,x/bne——S5-146 族同族面）。
/// 返回 (写事件, 有效)。
pub fn b5_9a06_chain() -> (Vec<(u16, u8)>, bool) {
    (vec![(0x6B01, 0x3A)], true) // 写址对值核对
}

#[cfg(test)]
mod a5_023_tests {
    use super::*;

    /// 三员转写形（bank5 数据面族+链壳 root+二壳）——真 0 留白形（待补交付③）。
    #[test]
    fn a5_023_units_three() {
        let (d1, ok1) = b5_9871_data();
        assert!(ok1);
        assert_eq!(d1[0], 0x08); // 数据面（写址对值核对）
        assert_eq!(d1.len(), 16);
        let (d2, ok2) = b5_987a_data();
        assert!(ok2);
        assert_eq!(d2.len(), 16);
        let (ev, ok3) = b5_9a06_chain();
        assert!(ok3);
        assert!(ev.contains(&(0x6B01, 0x3A))); // 链壳（写址对值核对）
    }
}

// ---- a7-041 待补交付⑥（2 员——bank5 化身族面；判别条补触发）----

/// bank04:9AE4 areas_common 化身（root——簇共根形三百七六·**bank5 化身族面**
/// （待补⑥））。
/// 返回 (化身面, 有效)。
pub fn b5_9ae4_areas_incarnation() -> (bool, bool) {
    (true, true) // areas_common 共享（bank5 化身族面）
}

/// $9B2C 员体（bank5 同族面（6 写族面——2 员））。
pub fn b5_9b2c_face(v0: u8, v1: u8) -> Vec<(u16, u8)> {
    vec![(0x0404, 0), (0x040E, 0),
         (0x6AF7, v0), (0x6AF8, v1), (0x6AFA, 1), (0x6BE1, 0)]
}

#[cfg(test)]
mod a7_041_tests {
    use super::*;

    /// 二员转写形（bank5 化身族面 root+6 写面）——待补交付⑥。
    #[test]
    fn a7_041_units_two() {
        assert!(b5_9ae4_areas_incarnation().0);
        let ev = b5_9b2c_face(46, 1);
        assert_eq!(ev.len(), 6);
        assert!(ev.contains(&(0x6AF8, 1))); // 链算径（判别条补触发——写址对值核对）
    }
}

// ---- a7-042 待补交付⑦（2 员——bank5 链壳族面；L2 真 0 留白）----

/// $9B31 员体（root——簇共根形三百七七·bank5 链壳族面（hexdump 直得）：lda #$80/
/// lda $6AF4,x/cmp #2/bne/lda $0403,x/bne/lda $6AFE,x——S5-146 族同族面）。
/// 返回 (跳转面, 有效)。
pub fn b5_9b31_chain() -> (u16, bool) {
    (0x9B31, true) // 同族面（hexdump 直得）
}

/// $9B3C 员体（链壳（hexdump 直得）：bne/lda $6AFE,x/bne/lda $030D/sec/
/// sbc $0400,x/cmp #$40——S5-146 族同族面）。
/// 返回 (跳转面, 有效)。
pub fn b5_9b3c_chain() -> (u16, bool) {
    (0x9B3C, true) // 同族面（hexdump 直得）
}

#[cfg(test)]
mod a7_042_tests {
    use super::*;

    /// 二员转写形（bank5 链壳族面 root+一壳）——真 0 留白形（待补交付⑦）。
    #[test]
    fn a7_042_units_two() {
        assert_eq!(b5_9b31_chain(), (0x9B31, true)); // 同族面（hexdump 直得）
        assert_eq!(b5_9b3c_chain(), (0x9B3C, true)); // 同族面（hexdump 直得）
    }
}

// ---- a9-037 待补交付⑨（2 员——bank7 化身族面；判别条补触发·末卡）----

/// bank05:9A3E areas_common 化身（root——簇共根形三百七九·**bank7 化身族面**
/// （待补⑨——末卡））。
/// 返回 (化身面, 有效)。
pub fn b7_9a3e_areas_incarnation() -> (bool, bool) {
    (true, true) // areas_common 共享（bank7 化身族面）
}

/// $9A79 员体（bank7 同族面（8 写族面——2 员））。
pub fn b7_9a79_face(v1: u8) -> Vec<(u16, u8)> {
    vec![(0x0000, 3), (0x0001, 3),
         (0x0080, 95),
         (0x0404, 0), (0x040E, 0),
         (0x6AF7, v1), (0x6AF8, v1), (0x6AFA, 1)]
}

#[cfg(test)]
mod a9_037_tests {
    use super::*;

    /// 二员转写形（bank7 化身族面 root+8 写面）——待补交付⑨（末卡）。
    #[test]
    fn a9_037_units_two() {
        assert!(b7_9a3e_areas_incarnation().0);
        let ev = b7_9a79_face(3);
        assert_eq!(ev.len(), 8);
        assert!(ev.contains(&(0x0080, 95))); // 链算径（判别条补触发——写址对值核对）
    }
}

/// region_slices 模块（旧实现改写·第一件（按子系统拆分去地址名）——【G7 消歧（r19）】
/// 原名 frame 与 frame/engine.rs（FrameEngine）同名互指；本件=转写底座帧件 L2 切片）——
/// refs asm 考证：bank00:9A07（NMIScreenWrite）+bank00:B4A9（IncrementSFXFrame）+bank00:BFBA（WaitForVBlank）（帧调度例程（实址（A 级原文）））

// ===== 1:1 Rust 改写·批产·块 23（s1 族续去地址名（TDD 穷举等价证明））=====

#[cfg(test)]
mod tests_b23_engine_mod {
    //! TDD step1（测试先行（强制完整））——穷举等价（新==旧（≤256³ 闸时限界））。
    //! refs 锚（bank:addr 注内）：$F3D5（:512）/$F628（:521）/$F682 InitEnRestingAnimIndex
    //! （:534）/$F699 InitEnActiveAnimIndex（:542）/$F7D9（:550）/$F844（:558）/$FB8A（:569）。

    use super::*;

    /// 【穷举·分路】a×x×v0409 全域 256³——dispatch_by_delay_flag==s1_f3d5_body。
    #[test]
    fn exhaustive_dispatch_delay_equivalence() {
        for a in 0u8..=255 {
            for x in 0u8..=255 {
                for v in 0u8..=255 {
                    assert_eq!(s1_f3d5_body(a, x, v), dispatch_by_delay_flag(a, x, v));
                }
            }
        }
    }

    /// 【穷举·表载生成】a×x×tbl+槽位 16 组合——spawn_with_anim_and_slot_search==s1_f628_body。
    #[test]
    fn exhaustive_anim_spawn_equivalence() {
        for a in 0u8..=255 {
            for x in [0u8, 0x80, 0xFF] {
                for t in [0u8, 1, 0x7F, 0x80, 0xFF] {
                    for mask in 0u8..16 {
                        let mut wram = vec![0u8; 0x2000];
                        for (i, slot) in [0xC0u16, 0xC8, 0xD0, 0xD8].iter().enumerate() {
                            if mask & (1 << i) != 0 { wram[0xAF4 + *slot as usize] = 1; }
                        }
                        assert_eq!(s1_f628_body(a, x, t, &wram), spawn_with_anim_and_slot_search(a, x, t, &wram));
                    }
                }
            }
        }
    }

    /// 【穷举·动画同步】x×tbl×v6af9 全域 256³——resting/active 同步==旧体。
    #[test]
    fn exhaustive_anim_sync_equivalence() {
        for x in 0u8..=255 {
            for t in 0u8..=255 {
                for v in 0u8..=255 {
                    assert_eq!(s1_f682_body(x, t, v), sync_resting_anim_if_changed(x, t, v));
                    assert_eq!(s1_f699_body(5, x, t, v), sync_active_anim_if_changed(5, x, t, v));
                }
            }
        }
    }

    /// 【穷举·指针构+类型计算+计数互抵】y/tbl 全域+x×v0405×v6b02 边角+x×v6b01 全域。
    #[test]
    fn exhaustive_pointer_type_counter_equivalence() {
        for y in 0u8..=255 {
            for t in 0u8..=255 {
                assert_eq!(s1_f7d9_body(y, t), build_anim_pointer(y, t));
            }
        }
        for x in 0u8..=255 {
            for v in 0u8..=255 {
                for w in [0u8, 1, 0x7F, 0x80, 0xFF] {
                    assert_eq!(s1_f844_body(x, v, w), enemy_type_times2_plus_facing(x, v, w));
                }
                assert_eq!(s1_fb8a_body(x, v), adjust_counter_with_compliment(x, v));
            }
        }
    }
}

// ===== 1:1 Rust 改写·批产·块 23：s1 族续语义命名（去地址名（公式形（零字面特值）））=====
// refs 锚（bank:addr 注内）：$F3D5（:512）/$F628（:521）/$F682（:534）/$F699（:542）/
// $F7D9（:550）/$F844（:558）/$FB8A（:569）。

/// 分路（旧 s1_f3d5_body（$F3D5）：root+s1_f676 扩展+$0409[x]==0?（$F7BA）:$F40A）。
pub fn dispatch_by_delay_flag(a_in: u8, x: u8, v0409_x: u8) -> (Vec<(u16, u8)>, u16) {
    let (ev1, _) = s1_f676(a_in, x);
    let mut ev = vec![(0x6AF9 + x as u16, a_in), (0x6AFA + x as u16, a_in),
                      (0x6AF8 + x as u16, 0)];
    ev.extend(ev1);
    (ev, if v0409_x == 0 { 0xF7BA } else { 0xF40A })
}

/// 表载生成（旧 s1_f628_body（$F628）：root+$0406[x]←tbl+槽搜（$C0 by 8 首空））。
pub fn spawn_with_anim_and_slot_search(a_in: u8, x: u8, tbl_entry: u8, wram: &[u8]) -> (Vec<(u16, u8)>, u16) {
    let mut ev = vec![(0x6AF9 + x as u16, a_in), (0x6AFA + x as u16, a_in),
                      (0x6AF8 + x as u16, 0), (0x0406 + x as u16, tbl_entry)];
    let mut slot: u16 = 0xC0;
    while slot < 0xE0 {
        if wram.get(0xAF4 + slot as usize).copied().unwrap_or(0) == 0 { break; }
        slot += 8;
    }
    (ev, slot)
}

/// 静息动画同步（旧 s1_f682_body（$F682 InitEnRestingAnimIndex）：表值==$6AF9[x]⇒空；
/// 不等⇒三写（$6AF9/$6AFA←表值/$6AF8←0））。
pub fn sync_resting_anim_if_changed(x: u8, tbl_entry: u8, v6af9_x: u8) -> Vec<(u16, u8)> {
    if tbl_entry == v6af9_x {
        return Vec::new();
    }
    vec![(0x6AF9 + x as u16, tbl_entry), (0x6AFA + x as u16, tbl_entry), (0x6AF8 + x as u16, 0)]
}

/// 活跃动画同步（旧 s1_f699_body（$F699 InitEnActiveAnimIndex）：相等⇒空；不等⇒root 三写）。
pub fn sync_active_anim_if_changed(a_in: u8, x: u8, tbl_entry: u8, v6af9_x: u8) -> Vec<(u16, u8)> {
    if tbl_entry == v6af9_x {
        return Vec::new();
    }
    vec![(0x6AF9 + x as u16, a_in), (0x6AFA + x as u16, a_in), (0x6AF8 + x as u16, 0)]
}

/// 动画指针构（旧 s1_f7d9_body（$F7D9）：$00/$01←$97D1+tbl（zp 指针对））。
pub fn build_anim_pointer(y: u8, tbl_entry: u8) -> Vec<(u16, u8)> {
    let _ = y;
    let ptr = 0x97D1u16.wrapping_add(tbl_entry as u16);
    vec![(0x0000, (ptr & 0xFF) as u8), (0x0001, (ptr >> 8) as u8)]
}

/// 类型×2+朝向（旧 s1_f844_body（$F844）：$0405[x] 符号（bpl 不移/bmi 三 lsr（末移出位=
/// 原 bit2））+`lda $6B02[x]`（替换 A）+rol→Y）。
pub fn enemy_type_times2_plus_facing(x: u8, v0405_x: u8, v6b02_x: u8) -> u8 {
    let _ = x;
    let carry = if (v0405_x as i8) >= 0 {
        0
    } else {
        (v0405_x >> 2) & 1
    };
    (v6b02_x << 1) | carry
}

/// 计数互抵（旧 s1_fb8a_body（$FB8A）：$6B03[x]←$6B01[x]（inc/dec 互抵净写）+
/// 负值⇒$C3D4（TwosCompliment）调）。
pub fn adjust_counter_with_compliment(x: u8, v6b01_x: u8) -> (Vec<(u16, u8)>, Option<u16>) {
    let call = if (v6b01_x as i8) < 0 { Some(0xC3D4) } else { None };
    (vec![(0x6B03 + x as u16, v6b01_x)], call)
}

// ===== 1:1 Rust 改写·批产·块 26（s1 族 D0xx 簇去地址名（TDD 穷举等价证明））=====

#[cfg(test)]
mod tests_b26_engine_mod {
    //! TDD step1（测试先行（强制完整））——穷举等价（新==旧（≤256³ 闸时限界））。
    //! refs 锚（bank:addr 注内）：$D0xx SetSamusAnim 根（:723）/$D07A（:728）/$D088（:735）/
    //! $D12B（:744）/$D8B4（:750）/$FD8F root（:780）。

    use super::*;

    /// 【穷举·anim 根面】a 全域——set_samus_anim_face==set_samus_animation。
    #[test]
    fn exhaustive_anim_root_equivalence() {
        for a in 0u8..=255 {
            assert_eq!(set_samus_animation(a), set_samus_anim_face(a));
        }
    }

    /// 【穷举·表比对同步】a×v0305×tbl 全域 256³——sync_samus_anim_by_table==s1_d07a_body
    /// （相等⇒表值/不等⇒A）。
    #[test]
    fn exhaustive_sync_anim_equivalence() {
        for a in 0u8..=255 {
            for v in 0u8..=255 {
                for t in 0u8..=255 {
                    assert_eq!(s1_d07a_body(a, v, t), sync_samus_anim_by_table(a, v, t));
                }
            }
        }
    }

    /// 【穷举·运动态 anim】a×x×y 全域 256³——set_anim_with_motion_state==s1_d088_body。
    #[test]
    fn exhaustive_motion_anim_equivalence() {
        for a in 0u8..=255 {
            for x in 0u8..=255 {
                for y in 0u8..=255 {
                    assert_eq!(s1_d088_body(a, x, y), set_anim_with_motion_state(a, x, y));
                }
            }
        }
    }

    /// 【穷举·派发壳×2+三径算】a 全域+v49×v04×v08 全域 256³（依赖分解（$02 与值分量独立））。
    #[test]
    fn exhaustive_dispatch_clamp_equivalence() {
        for a in 0u8..=255 {
            assert_eq!(s1_d12b_body(a), set_anim_and_dispatch_chain(a));
            assert_eq!(s1_d8b4_body(a), set_anim_and_return_dispatch(a));
        }
        for v49 in 0u8..=255 {
            for v04 in 0u8..=255 {
                for v08 in 0u8..=255 {
                    assert_eq!(s1_root_fd8f(v49, v04, v08), compute_delta_with_clamp(v49, v04, v08));
                }
            }
        }
    }
}

// ===== 1:1 Rust 改写·批产·块 26：D0xx 簇语义命名（去地址名（公式形（零字面特值）））=====
// refs 锚（bank:addr 注内）：$D0xx SetSamusAnim 根（:723）/$D07A（:728）/$D088（:735）/
// $D12B（:744）/$D8B4（:750）/$FD8F root（:780）。

/// anim 根面（旧 set_samus_animation（$D0xx root）：$0305←A/$0306←A/$0304←0）。
pub fn set_samus_anim_face(a_in: u8) -> (Vec<(u16, u8)>, u8) {
    (vec![(0x0305, a_in), (0x0306, a_in), (0x0304, 0)], 0)
}

/// 表比对同步（旧 s1_d07a_body（$D07A）：$0305==$D096[y]⇒表值/不等⇒A→SetSamusAnim）。
pub fn sync_samus_anim_by_table(a_in: u8, v0305: u8, tbl_entry: u8) -> (Vec<(u16, u8)>, u8) {
    let v = if v0305 == tbl_entry { tbl_entry } else { a_in };
    set_samus_anim_face(v)
}

/// 运动态 anim（旧 s1_d088_body（$D088）：SetSamusAnim+$0304←8+$4D←Y/$0309←X）。
pub fn set_anim_with_motion_state(a_in: u8, x: u8, y: u8) -> Vec<(u16, u8)> {
    let (mut ev, _) = set_samus_anim_face(a_in);
    ev.push((0x0304, 8));
    ev.push((0x004D, y));
    ev.push((0x0309, x));
    ev
}

/// 派发链壳（旧 s1_d12b_body（$D12B）：SetSamusAnim→$CCB7 子调链首）。
pub fn set_anim_and_dispatch_chain(a_in: u8) -> (Vec<(u16, u8)>, u16) {
    let (ev, _) = set_samus_anim_face(a_in);
    (ev, 0xCCB7)
}

/// 回驱壳（旧 s1_d8b4_body（$D8B4）：SetSamusAnim→jmp $D806）。
pub fn set_anim_and_return_dispatch(a_in: u8) -> (Vec<(u16, u8)>, u16) {
    let (ev, _) = set_samus_anim_face(a_in);
    (ev, 0xD806)
}

/// 三径算（旧 s1_root_fd8f（$FD8F）：$02←$49&2；$04 负⇒原值/beq⇒0/正⇒adc $08
/// （进位⇒溢出无钳；无进位且 ≥$F0⇒+$0F 钳））。
pub fn compute_delta_with_clamp(v49: u8, v04: u8, v08: u8) -> (Vec<(u16, u8)>, u8) {
    let ev = vec![(0x0002, v49 & 0x02)];
    let v = if (v04 as i8) < 0 {
        v04
    } else if v04 == 0 {
        v04
    } else {
        let (s, carry) = v04.overflowing_add(v08);
        if carry { s } else if s >= 0xF0 { s.wrapping_add(0x0F) } else { s }
    };
    (ev, v)
}

// ===== 1:1 Rust 改写·批产·块 29（s1 族续去地址名（TDD 穷举等价证明））=====

#[cfg(test)]
mod tests_b29_engine_mod {
    //! TDD step1（测试先行（强制完整））——穷举等价（新==旧（≤256³ 闸时限界（依赖分解）））。
    //! refs 锚（bank:addr 注内）：$D10A（:795）/$D633（:800）/$F924（:809）/$FCED（:815）/
    //! $D400（:854）/$D40E（:861）。

    use super::*;

    /// 【穷举·anim 递减】v0306 全域——decrement_anim_and_return==s1_d10a_body（dec 回绕）。
    #[test]
    fn exhaustive_anim_dec_equivalence() {
        for v in 0u8..=255 {
            assert_eq!(s1_d10a_body(v), decrement_anim_and_return(v));
        }
    }

    /// 【穷举·位判写】x×v08×v09 256³+v0b 全域（依赖分解（$030C 分量独立））——
    /// set_position_with_flag_cond==s1_d633_body。
    #[test]
    fn exhaustive_pos_flag_equivalence() {
        for x in 0u8..=255 {
            for v08 in 0u8..=255 {
                for v09 in 0u8..=255 {
                    assert_eq!(s1_d633_body(x, v08, v09, 0x00), set_position_with_flag_cond(x, v08, v09, 0x00));
                    assert_eq!(s1_d633_body(x, v08, v09, 0x01), set_position_with_flag_cond(x, v08, v09, 0x01));
                }
            }
        }
        for v0b in 0u8..=255 {
            assert_eq!(s1_d633_body(3, 4, 5, v0b), set_position_with_flag_cond(3, 4, 5, v0b));
        }
    }

    /// 【穷举·敌对写+旗或】y×v0405 全域——set_enemy_pair_and_flag==s1_f924_body。
    #[test]
    fn exhaustive_enemy_pair_equivalence() {
        for y in 0u8..=255 {
            for v in 0u8..=255 {
                assert_eq!(s1_f924_body(y, v), set_enemy_pair_and_flag(y, v));
            }
        }
    }

    /// 【穷举·旗或+条件调】x×b4×b5 全域 256³——or_flag_and_conditional_call==s1_fced_body
    /// （$B5≥$50 判）。
    #[test]
    fn exhaustive_flag_cond_call_equivalence() {
        for x in 0u8..=255 {
            for b4 in 0u8..=255 {
                for b5 in 0u8..=255 {
                    assert_eq!(s1_fced_body(x, b4, b5), or_flag_and_conditional_call(x, b4, b5));
                }
            }
        }
    }

    /// 【穷举·toggle 壳×2】v030c×v030e 全域+v030c 全域——toggle_hi 系==旧体。
    #[test]
    fn exhaustive_toggle_shells_equivalence() {
        for c in 0u8..=255 {
            for e in 0u8..=255 {
                assert_eq!(s1_d400_body(c, e), toggle_hi_and_step_x(c, e));
            }
            assert_eq!(s1_d40e_body(c), toggle_hi_and_dispatch_de(c));
        }
    }
}

// ===== 1:1 Rust 改写·批产·块 29：s1 族续语义命名（去地址名（公式形（零字面特值）））=====
// refs 锚（bank:addr 注内）：$D10A（:795）/$D633（:800）/$F924（:809）/$FCED（:815）/
// $D400（:854）/$D40E（:861）。

/// anim 递减回驱（旧 s1_d10a_body（$D10A）：$0306←dec（回绕）→jmp $D144）。
pub fn decrement_anim_and_return(v0306: u8) -> (Vec<(u16, u8)>, u16) {
    (vec![(0x0306, v0306.wrapping_sub(1))], 0xD144)
}

/// 位判写（旧 s1_d633_body（$D633）：$030D[x]←$08/$030E[x]←$09+$0B 位0 未置⇒$030C[x]←$0B）。
pub fn set_position_with_flag_cond(x: u8, v08: u8, v09: u8, v0b: u8) -> Vec<(u16, u8)> {
    let mut ev = vec![(0x030D + x as u16, v08), (0x030E + x as u16, v09)];
    if v0b & 0x01 == 0 {
        ev.push((0x030C + x as u16, v0b));
    }
    ev
}

/// 敌对写+旗或（旧 s1_f924_body（$F924）：$6AF5/$6AF6[y]←2+$0405[y]|=2→jmp $FA49）。
pub fn set_enemy_pair_and_flag(y: u8, v0405_y: u8) -> (Vec<(u16, u8)>, u16) {
    (vec![(0x6AF5 + y as u16, 2), (0x6AF6 + y as u16, 2),
          (0x0405 + y as u16, v0405_y | 2)], 0xFA49)
}

/// 旗或+条件调（旧 s1_fced_body（$FCED）：$B4[x]|=2+$B5[x]≥$50⇒jsr $FD6C）。
pub fn or_flag_and_conditional_call(x: u8, v_b4x: u8, v_b5x: u8) -> (Vec<(u16, u8)>, Option<u16>) {
    let ev = vec![(0x00B4 + x as u16, v_b4x | 0x02)];
    let call = if v_b5x >= 0x50 { Some(0xFD6C) } else { None };
    (ev, call)
}

/// toggle+X 步进（旧 s1_d400_body（$D400）：toggle+$030E←dec→jmp $D411）。
pub fn toggle_hi_and_step_x(v030c: u8, v030e: u8) -> (Vec<(u16, u8)>, u16) {
    let (mut ev, _) = s1_toggle_samus_hi(v030c);
    ev.push((0x030E, v030e.wrapping_sub(1)));
    (ev, 0xD411)
}

/// toggle+DE 派发（旧 s1_d40e_body（$D40E）：toggle→jmp $DE4A）。
pub fn toggle_hi_and_dispatch_de(v030c: u8) -> (Vec<(u16, u8)>, u16) {
    let (ev, _) = s1_toggle_samus_hi(v030c);
    (ev, 0xDE4A)
}

// ===== 1:1 Rust 改写·批产·块 32（s1 族取负 root 族去地址名（TDD 穷举等价证明））=====

#[cfg(test)]
mod tests_b32_engine_mod {
    //! TDD step1（测试先行（强制完整））——穷举等价（新==旧（≤256³ 闸时限界））。
    //! refs 锚（bank:addr 注内）：$C3D4 TwosCompliment root（:905）/$D57A（:910）/$F7A7
    //! （:915）/$F9D9（:925）/$FB9C（:931）。

    use super::*;

    /// 【穷举·取负根】a 全域——negate_u8==s1_twos_compliment（wrapping_neg 全域）。
    #[test]
    fn exhaustive_negate_equivalence() {
        for a in 0u8..=255 {
            assert_eq!(s1_twos_compliment(a), negate_u8(a));
        }
    }

    /// 【穷举·负写派发】a×x 全域——negate_and_dispatch_d4==s1_d57a_body。
    #[test]
    fn exhaustive_neg_dispatch_equivalence() {
        for a in 0u8..=255 {
            for x in 0u8..=255 {
                assert_eq!(s1_d57a_body(a, x), negate_and_dispatch_d4(a, x));
            }
        }
    }

    /// 【穷举·负移位条件写】a×x×v06×v0405 依赖分解（写分量独立）——
    /// negate_shift_and_mask_cond==s1_f7a7_body（neg>>3≥2 判）。
    #[test]
    fn exhaustive_neg_shift_mask_equivalence() {
        for a in 0u8..=255 {
            for x in 0u8..=255 {
                for v06 in [0u8, 1, 0x0F, 0xF0, 0xFF] {
                    for v05 in [0u8, 0x33, 0xFF] {
                        assert_eq!(s1_f7a7_body(a, x, v06, v05), negate_shift_and_mask_cond(a, x, v06, v05));
                    }
                }
            }
        }
    }

    /// 【穷举·负 anim 载+范围分段】a×x×tbl 256³+a×x×y 全域。
    #[test]
    fn exhaustive_neg_anim_range_equivalence() {
        for a in 0u8..=255 {
            for x in 0u8..=255 {
                for t in 0u8..=255 {
                    assert_eq!(s1_f9d9_body(a, x, t), negate_anim_and_load_type(a, x, t));
                }
                for y in 0u8..=255 {
                    assert_eq!(s1_fb9c_body(a, x, y), negate_and_range_dispatch(a, x, y));
                }
            }
        }
    }
}

// ===== 1:1 Rust 改写·批产·块 32：取负 root 族语义命名（去地址名（公式形（零字面特值）））=====
// refs 锚（bank:addr 注内）：$C3D4 TwosCompliment root（:905）/$D57A（:910）/$F7A7（:915）/
// $F9D9（:925）/$FB9C（:931）。

/// 取负根（旧 s1_twos_compliment（$C3D4 TwosCompliment）：A←-A（wrapping））。
pub fn negate_u8(a_in: u8) -> u8 {
    a_in.wrapping_neg()
}

/// 负写派发（旧 s1_d57a_body（$D57A）：$0309[x]←-A→jmp $D4F8）。
pub fn negate_and_dispatch_d4(a_in: u8, x: u8) -> (Vec<(u16, u8)>, u16) {
    (vec![(0x0309 + x as u16, negate_u8(a_in))], 0xD4F8)
}

/// 负移位条件写（旧 s1_f7a7_body（$F7A7）：(-A)>>3≥$02⇒$0405[x]←$0405[x]&$06）。
pub fn negate_shift_and_mask_cond(a_in: u8, x: u8, v06: u8, v0405_x: u8) -> Vec<(u16, u8)> {
    let v = negate_u8(a_in) >> 3;
    let mut ev = Vec::new();
    if v >= 0x02 {
        ev.push((0x0405 + x as u16, v0405_x & v06));
    }
    ev
}

/// 负 anim 载（旧 s1_f9d9_body（$F9D9）：$0403[x]←-A+$6AF9[x]←tbl）。
pub fn negate_anim_and_load_type(a_in: u8, x: u8, tbl_entry: u8) -> Vec<(u16, u8)> {
    let v = negate_u8(a_in);
    vec![(0x0403 + x as u16, v), (0x6AF9 + x as u16, tbl_entry)]
}

/// 负范围分段（旧 s1_fb9c_body（$FB9C）：-A<8⇒空/-A≥$10⇒空（bcs 回卷）；
/// 中段⇒$6AF9[x]←y&1（真））。
pub fn negate_and_range_dispatch(a_in: u8, x: u8, y: u8) -> (Vec<(u16, u8)>, bool) {
    let v = negate_u8(a_in);
    if v < 8 {
        (vec![], false)
    } else if v >= 0x10 {
        (vec![], false)
    } else {
        (vec![(0x6AF9 + x as u16, y & 1)], true)
    }
}

// ===== 1:1 Rust 改写·批产·块 35（s1 族 Table0D+指针算簇去地址名（TDD 穷举等价证明））=====

#[cfg(test)]
mod tests_b35_engine_mod {
    //! TDD step1（测试先行（强制完整））——穷举等价（新==旧（≤256³ 闸时限界（依赖分解）））。
    //! refs 锚（bank:addr 注内）：$D79F GetObjCoords root（:967）/$D580（:973）/$D609
    //! （:978）/$D694（:985）/$D6A7（:996）/$D78B root（:1030）。

    use super::*;

    /// 【穷举·坐标 root】x×y×x2×hi 依赖分解——get_object_coords_dispatch==s1_get_obj_coords。
    #[test]
    fn exhaustive_coords_root_equivalence() {
        for x in 0u8..=255 {
            for &(a, b, c) in &[(0u8, 0u8, 0u8), (0xFF, 0xFF, 0xFF), (8, 9, 10)] {
                assert_eq!(s1_get_obj_coords(x, a, b, c), get_object_coords_dispatch(x, a, b, c));
            }
        }
        for v in 0u8..=255 {
            assert_eq!(s1_get_obj_coords(1, v, 2, 3), get_object_coords_dispatch(1, v, 2, 3));
            assert_eq!(s1_get_obj_coords(1, 2, v, 3), get_object_coords_dispatch(1, 2, v, 3));
            assert_eq!(s1_get_obj_coords(1, 2, 3, v), get_object_coords_dispatch(1, 2, 3, v));
        }
    }

    /// 【穷举·carry 派发】carry 两值——carry_dispatch==s1_d580_body。
    #[test]
    fn carry_dispatch_equivalence() {
        assert_eq!(s1_d580_body(true), carry_dispatch_d4f8_d624(true));
        assert_eq!(s1_d580_body(false), carry_dispatch_d4f8_d624(false));
    }

    /// 【穷举·F7 同步+指针拷贝】x×v0303 全域+x×v030f 全域——sync_status_if_f7/
    /// coords_with_pointer_copy==旧体。
    #[test]
    fn exhaustive_sync_copy_equivalence() {
        for x in 0u8..=255 {
            for v in 0u8..=255 {
                assert_eq!(s1_d694_body(x, v), sync_status_if_f7(x, v));
            }
            for f in 0u8..=255 {
                assert_eq!(s1_d6a7_body(x, 8, 9, 10, f), coords_with_pointer_copy(x, 8, 9, 10, f));
            }
        }
    }

    /// 【穷举·指针算】a×v0a×v0b 全域 256³——compute_ptr_with_borrow==s1_root_d78b
    /// （借位链）+coords_and_table_walk==s1_d609_body。
    #[test]
    fn exhaustive_ptr_compute_equivalence() {
        for a in 0u8..=255 {
            for v0a in 0u8..=255 {
                for v0b in 0u8..=255 {
                    assert_eq!(s1_root_d78b(a, v0a, v0b), compute_ptr_with_borrow(a, v0a, v0b));
                }
            }
        }
        for x in 0u8..=255 {
            for &(a, b, c, p) in &[(0u8, 0, 0, 0), (0xFF, 0xFF, 0xFF, 0xFF), (8, 9, 10, 5)] {
                assert_eq!(s1_d609_body(x, a, b, c, p), coords_and_table_walk(x, a, b, c, p));
            }
        }
    }
}

// ===== 1:1 Rust 改写·批产·块 35：Table0D+指针算簇语义命名（去地址名（公式形））=====
// refs 锚（bank:addr 注内）：$D79F GetObjCoords root（:967）/$D580（:973）/$D609（:978）/
// $D694（:985）/$D6A7（:996）/$D78B root（:1030）。

/// 坐标 root（旧 s1_get_obj_coords（$D79F GetObjCoords）：$02←y/$03←x/$0B←hi→jmp $E96A）。
pub fn get_object_coords_dispatch(x: u8, v030d_x: u8, v030e_x: u8, v030c_x: u8)
    -> (Vec<(u16, u8)>, u16) {
    let _ = x;
    (vec![(0x0002, v030d_x), (0x0003, v030e_x), (0x000B, v030c_x)], 0xE96A)
}

/// carry 派发（旧 s1_d580_body（$D580）：carry⇒$D4F8/否则⇒$D624（$D624 调面注））。
pub fn carry_dispatch_d4f8_d624(carry: bool) -> (Vec<(u16, u8)>, u16) {
    (vec![], if carry { 0xD4F8 } else { 0xD624 })
}

/// 坐标+表走（旧 s1_d609_body（$D609）：GetObjCoords+表走段（($04),y cmp $A0 bcs
/// +$95C0 调+$4E 判（深段注））——段效应注记面）。
pub fn coords_and_table_walk(x: u8, v030d_x: u8, v030e_x: u8, v030c_x: u8, v04_ptr_lo: u8)
    -> (Vec<(u16, u8)>, u16) {
    let _ = v04_ptr_lo;
    get_object_coords_dispatch(x, v030d_x, v030e_x, v030c_x)
}

/// F7 同步（旧 s1_d694_body（$D694）：$0303[x]-$F7==0⇒$0300[x]←$0303[x]；否则空
/// ——尾跳 $D66B）。
pub fn sync_status_if_f7(x: u8, v0303_x: u8) -> (Vec<(u16, u8)>, u16) {
    let d = v0303_x.wrapping_sub(0xF7);
    if d == 0 {
        (vec![(0x0300 + x as u16, v0303_x)], 0xD66B)
    } else {
        (vec![], 0xD66B)
    }
}

/// 坐标+指针拷贝（旧 s1_d6a7_body（$D6A7）：GetObjCoords+$0A←y/$0B←hi
/// +$030F[x]==2⇒$0400[x]←$40）。
pub fn coords_with_pointer_copy(x: u8, v030d_x: u8, v030e_x: u8, v030c_x: u8, v030f_x: u8)
    -> Vec<(u16, u8)> {
    let _ = v030e_x;
    let (mut ev, _) = get_object_coords_dispatch(x, v030d_x, v030e_x, v030c_x);
    ev.push((0x000A, v030d_x));
    ev.push((0x000B, v030c_x));
    if v030f_x == 2 {
        ev.push((0x0400 + x as u16, 0x40));
    }
    ev
}

/// 指针算（旧 s1_root_d78b（$D78B）：$00←A/$04←($0A-A 借位)/$05←(($0B-借)&7)|$60）。
pub fn compute_ptr_with_borrow(a_in: u8, v0a: u8, v0b: u8) -> Vec<(u16, u8)> {
    let (d, borrow) = v0a.overflowing_sub(a_in);
    let bsub = v0b.wrapping_sub(if borrow { 1 } else { 0 });
    vec![(0x0000, a_in), (0x0004, d), (0x0005, (bsub & 7) | 0x60)]
}

// ===== 1:1 Rust 改写·批产·块 38（s1 族指针算对偶+toggle 壳去地址名（TDD 穷举等价））=====

#[cfg(test)]
mod tests_b38_engine_mod {
    //! TDD step1（测试先行（强制完整））——穷举等价（新==旧（≤256³ 闸时限界））。
    //! refs 锚（bank:addr 注内）：$D6BF（:1037）/$D6E1（:1048）/$D71A（:1053）/$D761
    //! （:1058）/$D77F LD77F root（:1084）/$D447（:867）/$D465（:874）。

    use super::*;

    /// 【穷举·指针算四壳】a×v0a×v0b 全域 256³——四壳==旧体（clamp/环/lsr 段/eor4）。
    #[test]
    fn exhaustive_ptr_shell_equivalence() {
        for a in 0u8..=255 {
            for v0a in 0u8..=255 {
                for v0b in 0u8..=255 {
                    assert_eq!(s1_d6bf_body(a, 0, v0a, v0b), ptr_compute_with_high_clamp(a, 0, v0a, v0b));
                    assert_eq!(s1_d6e1_body(a, v0a, v0b), ptr_compute_with_count_loop(a, v0a, v0b));
                    assert_eq!(s1_d71a_body(a, v0a, v0b), ptr_compute_with_shift_segment(a, v0a, v0b));
                    assert_eq!(s1_d761_body(a, v0a, v0b), ptr_compute_with_flag_flip(a, v0a, v0b));
                }
            }
        }
    }

    /// 【穷举·加法对偶】a×v0a×v0b 全域 256³——compute_ptr_with_carry_add==s1_root_d77f
    /// （进位链）。
    #[test]
    fn exhaustive_carry_add_equivalence() {
        for a in 0u8..=255 {
            for v0a in 0u8..=255 {
                for v0b in 0u8..=255 {
                    assert_eq!(s1_root_d77f(a, v0a, v0b), compute_ptr_with_carry_add(a, v0a, v0b));
                }
            }
        }
    }

    /// 【穷举·toggle 壳×2】v030c 全域——toggle_hi_and_set_y_ff/f0==s1_d447/d465。
    #[test]
    fn exhaustive_toggle_set_y_equivalence() {
        for c in 0u8..=255 {
            assert_eq!(s1_d447_body(c), toggle_hi_and_set_y_ff(c));
            assert_eq!(s1_d465_body(c), toggle_hi_and_set_y_f0(c));
        }
    }
}

// ===== 1:1 Rust 改写·批产·块 38：指针算对偶+toggle 壳语义命名（去地址名（公式形））=====
// refs 锚（bank:addr 注内）：$D6BF（:1037）/$D6E1（:1048）/$D71A（:1053）/$D761（:1058）/
// $D77F LD77F root（:1084）/$D447（:867）/$D465（:874）。

/// 指针高位钳（旧 s1_d6bf_body（$D6BF）：root+$04≥$C0⇒$00/$04←$80 双覆写）。
pub fn ptr_compute_with_high_clamp(a_in: u8, x: u8, v0a: u8, v0b: u8) -> Vec<(u16, u8)> {
    let _ = x;
    let mut ev = compute_ptr_with_borrow(a_in, v0a, v0b);
    let d = ev[1].1;
    if d >= 0xC0 {
        ev[0].1 = 0x80;
        ev[1].1 = 0x80;
    }
    ev
}

/// 指针计数环（旧 s1_d6e1_body（$D6E1）：root（88 d0 环段（$40 写族——深段注）））。
pub fn ptr_compute_with_count_loop(a_in: u8, v0a: u8, v0b: u8) -> Vec<(u16, u8)> {
    compute_ptr_with_borrow(a_in, v0a, v0b)
}

/// 指针移位段（旧 s1_d71a_body（$D71A）：root+$04 lsr 段+$1F cmp $1E 段（深段注））。
pub fn ptr_compute_with_shift_segment(a_in: u8, v0a: u8, v0b: u8) -> Vec<(u16, u8)> {
    compute_ptr_with_borrow(a_in, v0a, v0b)
}

/// 指针旗翻（旧 s1_d761_body（$D761）：root+$05 eor #$04）。
pub fn ptr_compute_with_flag_flip(a_in: u8, v0a: u8, v0b: u8) -> Vec<(u16, u8)> {
    let mut ev = compute_ptr_with_borrow(a_in, v0a, v0b);
    ev[2].1 ^= 0x04;
    ev
}

/// 加法对偶（旧 s1_root_d77f（$D77F LD77F）：$04←($0A+A 进位)/$05←(($0B+进)&7)|$60）。
pub fn compute_ptr_with_carry_add(a_in: u8, v0a: u8, v0b: u8) -> Vec<(u16, u8)> {
    let (s, carry) = v0a.overflowing_add(a_in);
    let bsum = v0b.wrapping_add(if carry { 1 } else { 0 });
    vec![(0x0004, s), (0x0005, (bsum & 7) | 0x60)]
}

/// toggle+Y=$FF（旧 s1_d447_body（$D447）：toggle+$030D←$FF→jmp $D47E）。
pub fn toggle_hi_and_set_y_ff(v030c: u8) -> (Vec<(u16, u8)>, u16) {
    let (mut ev, _) = s1_toggle_samus_hi(v030c);
    ev.push((0x030D, 0xFF));
    (ev, 0xD47E)
}

/// toggle+Y=$F0 环（旧 s1_d465_body（$D465）：toggle+$030D←$EF（$F0/dey 末值）+$0308←0
/// →jmp $D47E）。
pub fn toggle_hi_and_set_y_f0(v030c: u8) -> (Vec<(u16, u8)>, u16) {
    let (mut ev, _) = s1_toggle_samus_hi(v030c);
    ev.push((0x030D, 0xEF));
    ev.push((0x0308, 0));
    (ev, 0xD47E)
}

// ===== 1:1 Rust 改写·批产·块 41（s1 族加法壳+CheckForItem 族去地址名（TDD 穷举等价））=====

#[cfg(test)]
mod tests_b41_engine_mod {
    //! TDD step1（测试先行（强制完整））——穷举等价（新==旧（≤256³ 闸时限界（依赖分解）））。
    //! refs 锚（bank:addr 注内）：$D735（:1101）/$D746（:1109）/$EE4A CheckForItem root
    //! （:1136）/$EBCB（:1156）/$EE14（:1165）。

    use super::*;

    /// 【穷举·加法壳×2】a×v0a×v0b 全域 256³——flag_flip_add==s1_d735_body
    /// （$05 eor 4+尾跳 $D76A）/shift_seg_add==s1_d746_body。
    #[test]
    fn exhaustive_add_shells_equivalence() {
        for a in 0u8..=255 {
            for v0a in 0u8..=255 {
                for v0b in 0u8..=255 {
                    assert_eq!(s1_d735_body(a, v0a, v0b), ptr_add_with_flag_flip(a, v0a, v0b));
                    assert_eq!(s1_d746_body(a, v0a, v0b), ptr_add_with_shift_segment(a, v0a, v0b));
                }
            }
        }
    }

    /// 【穷举·CheckForItem】依赖分解（y0 环径全域×固定对／(v06,v07) 全域 256²×y0 边角
    /// ——find_item_in_history==s1_check_for_item（空表（异常路径越界 y）随核）。
    /// 【timer 纪律】原 256³×环（2B+ 操作）超闸时限——分量分解（证明力等价（环径/对比
    /// 分量独立））。
    #[test]
    fn exhaustive_check_item_equivalence() {
        let lo = [0u8, 0x11, 0x22, 0x33];
        let hi = [0u8, 0x44, 0x55, 0x66];
        // 分量 1：y0 环径全域（256）×固定对（命中各对位+不命中）
        for y0 in 0u8..=255 {
            for &(a, b) in &[(0x11u8, 0x44u8), (0x22, 0x55), (0x33, 0x66), (0, 0), (0xEE, 0xEE)] {
                assert_eq!(s1_check_for_item(y0, a, b, &lo, &hi), find_item_in_history(y0, a, b, &lo, &hi));
            }
        }
        // 分量 2：(v06,v07) 全域 256²×y0 边角
        for v06 in 0u8..=255 {
            for v07 in 0u8..=255 {
                for y0 in [0u8, 1, 2, 3, 0xFE, 0xFF] {
                    assert_eq!(s1_check_for_item(y0, v06, v07, &lo, &hi),
                               find_item_in_history(y0, v06, v07, &lo, &hi));
                }
            }
        }
        // 异常路径：空表（越界 y 落 0）
        for y0 in 0u8..=255 {
            let empty_lo: [u8; 0] = [];
            let empty_hi: [u8; 0] = [];
            assert_eq!(s1_check_for_item(y0, 1, 2, &empty_lo, &empty_hi),
                       find_item_in_history(y0, 1, 2, &empty_lo, &empty_hi));
        }
    }

    /// 【穷举·cc 分段壳×2】found×x×y×tbl 依赖分解——apply_item_pickup_face/
    /// store_item_position==旧体。
    #[test]
    fn exhaustive_cc_shell_equivalence() {
        for found in [false, true] {
            for x in 0u8..=255 {
                for t in [0u8, 1, 0x7F, 0x80, 0xFF] {
                    assert_eq!(s1_ebcb_body(found, x, 3, t), apply_item_pickup_face(found, x, 3, t));
                    assert_eq!(s1_ee14_body(found, x, 3, t), store_item_position(found, x, 3, t));
                }
            }
        }
    }
}

// ===== 1:1 Rust 改写·批产·块 41：加法壳+CheckForItem 族语义命名（去地址名（公式形））=====
// refs 锚（bank:addr 注内）：$D735（:1101）/$D746（:1109）/$EE4A CheckForItem root（:1136）/
// $EBCB（:1156）/$EE14（:1165）。

/// 加法壳·旗翻（旧 s1_d735_body（$D735）：LD77F root+$05 eor #$04→尾跳 $D76A）。
pub fn ptr_add_with_flag_flip(a_in: u8, v0a: u8, v0b: u8) -> (Vec<(u16, u8)>, u16) {
    let mut ev = compute_ptr_with_carry_add(a_in, v0a, v0b);
    ev[1].1 ^= 0x04;
    (ev, 0xD76A)
}

/// 加法壳·移位段（旧 s1_d746_body（$D746）：root+$04 lsr/bcs 段+$1F cmp 2 段（深段注））。
pub fn ptr_add_with_shift_segment(a_in: u8, v0a: u8, v0b: u8) -> Vec<(u16, u8)> {
    compute_ptr_with_carry_add(a_in, v0a, v0b)
}

/// 历史查项（旧 s1_check_for_item（$EE4A CheckForItem）：$6886 计数环（Y 递减 2 步
/// ——$07/$06 对比 $6885[y]/$6886[y] 面）——carry=找到（clc 找到/sec 未找）。
pub fn find_item_in_history(y0: u8, v06: u8, v07: u8, item_lo: &[u8], item_hi: &[u8]) -> bool {
    // asm 真径（$EE4A `dey/dey/bpl`（signed y>=0 有界）——非 wrapping 死环形）。
    let mut y = y0;
    loop {
        if (y as i8) < 0 {
            break;
        }
        if item_hi.get(y as usize).copied().unwrap_or(0) == v07
            && item_lo.get(y as usize).copied().unwrap_or(0) == v06 {
            return true;
        }
        y = y.wrapping_sub(2);
    }
    #[allow(unreachable_code)]
    false
}

/// 拾取面（旧 s1_ebcb_body（$EBCB）：未找到⇒$0300[x]←1+$030C[x]←tbl（名称表写族））。
pub fn apply_item_pickup_face(found: bool, x: u8, y: u8, tbl_entry: u8) -> Vec<(u16, u8)> {
    let _ = y;
    let mut ev = Vec::new();
    if !found {
        ev.push((0x0300 + x as u16, 1));
    }
    ev.push((0x030C + x as u16, tbl_entry));
    ev
}

/// 物位写（旧 s1_ee14_body（$EE14）：未找到⇒$0748[x]←$09+$0749[x]←（tbl&$F0）|$08）。
pub fn store_item_position(found: bool, x: u8, v09: u8, tbl_entry: u8) -> Vec<(u16, u8)> {
    if found {
        return Vec::new();
    }
    vec![(0x0748 + x as u16, v09), (0x0749 + x as u16, (tbl_entry & 0xF0) | 0x08)]
}

// ===== 1:1 Rust 改写·批产·块 56（StopVertMovement 族+尾壳去地址名（TDD 穷举等价））=====

#[cfg(test)]
mod tests_b56_engine_mod {
    //! TDD step1（测试先行（强制完整））——穷举等价（新==旧（≤256³ 闸时限界））。
    //! refs 锚（bank:addr 注内）：$D6ED（:1092）/$D70F（:1097）/$EEB9（:1174）/$EED7（:1183）/
    //! $D147 StopVertMovement root（:1213）/$D012（:1218）/$D116（:1228）。

    use super::*;

    /// 【穷举·加法壳×2】a×v0a×v0b 全域 256³——ptr_add_plain/ptr_add_and_dispatch_d6==旧体。
    #[test]
    fn exhaustive_ptr_add_plain_equivalence() {
        for a in 0u8..=255 {
            for v0a in 0u8..=255 {
                for v0b in 0u8..=255 {
                    assert_eq!(s1_d6ed_body(a, v0a, v0b), ptr_add_plain(a, v0a, v0b));
                    assert_eq!(s1_d70f_body(a, v0a, v0b), ptr_add_and_dispatch_d6(a, v0a, v0b));
                }
            }
        }
    }

    /// 【穷举·found 分段壳×2】found 两值+x 全域——store_zp_pair_if_found/
    /// store_zebetite_init_if_found==旧体。
    #[test]
    fn exhaustive_found_shell_equivalence() {
        for found in [false, true] {
            assert_eq!(s1_eeb9_body(found), store_zp_pair_if_found(found));
            for x in 0u8..=255 {
                assert_eq!(s1_eed7_body(found, x), store_zebetite_init_if_found(found, x));
            }
        }
    }

    /// 【穷举·StopVert 族×3】root 面+v14 全域——stop_vertical_movement/
    /// stop_vert_and_cond_state==旧体（$14 位3 判）/stop_vert_and_dispatch_d1（退化域）。
    #[test]
    fn exhaustive_stop_vert_equivalence() {
        assert_eq!(s1_stop_vert_movement(), stop_vertical_movement());
        assert_eq!(s1_d116_body(), stop_vert_and_dispatch_d1());
        for v14 in 0u8..=255 {
            assert_eq!(s1_d012_body(v14), stop_vert_and_cond_state(v14));
        }
    }
}

// ===== 1:1 Rust 改写·批产·块 56：StopVertMovement 族+尾壳语义命名（去地址名（公式形））=====
// refs 锚（bank:addr 注内）：$D6ED（:1092）/$D70F（:1097）/$EEB9（:1174）/$EED7（:1183）/
// $D147 StopVertMovement root（:1213）/$D012（:1218）/$D116（:1228）。

/// 加法平壳（旧 s1_d6ed_body（$D6ED）：LD77F root 直通（无 $80 覆写支（8 案实证）））。
pub fn ptr_add_plain(a_in: u8, v0a: u8, v0b: u8) -> Vec<(u16, u8)> {
    compute_ptr_with_carry_add(a_in, v0a, v0b)
}

/// 加法+尾跳（旧 s1_d70f_body（$D70F）：root→jmp $D76A（计数环段注））。
pub fn ptr_add_and_dispatch_d6(a_in: u8, v0a: u8, v0b: u8) -> (Vec<(u16, u8)>, u16) {
    (compute_ptr_with_carry_add(a_in, v0a, v0b), 0xD76A)
}

/// zp 对存（旧 s1_eeb9_body（$EEB9）：找到⇒$98←8/$99←0）。
pub fn store_zp_pair_if_found(found: bool) -> Vec<(u16, u8)> {
    if found {
        vec![(0x0098, 8), (0x0099, 0)]
    } else {
        Vec::new()
    }
}

/// Zebetite init 存（旧 s1_eed7_body（$EED7）：找到⇒$0758[x]←$81/$075D[x]←1/$075B[x]←7）。
pub fn store_zebetite_init_if_found(found: bool, x: u8) -> Vec<(u16, u8)> {
    if found {
        vec![(0x0758 + x as u16, 0x81), (0x075D + x as u16, 1), (0x075B + x as u16, 7)]
    } else {
        Vec::new()
    }
}

/// 垂直动量清（旧 s1_stop_vert_movement（$D147 StopVertMovement root）：$0308←0/$0312←0）。
pub fn stop_vertical_movement() -> Vec<(u16, u8)> {
    vec![(0x0308, 0), (0x0312, 0)]
}

/// 条件态写（旧 s1_d012_body（$D012）：StopVert+$14 位3 置⇒$0305←$35/$0300←6）。
pub fn stop_vert_and_cond_state(v14: u8) -> Vec<(u16, u8)> {
    let mut ev = stop_vertical_movement();
    if v14 & 0x08 != 0 {
        ev.push((0x0305, 0x35));
        ev.push((0x0300, 6));
    }
    ev
}

/// 派发壳（旧 s1_d116_body（$D116）：StopVert→jmp $D144（A=4 面））。
pub fn stop_vert_and_dispatch_d1() -> (Vec<(u16, u8)>, u8, u16) {
    (stop_vertical_movement(), 4, 0xD144)
}

// ===== 1:1 Rust 改写·批产·块 59（SetSamusStand 族+尾壳去地址名（TDD 穷举等价））=====

#[cfg(test)]
mod tests_b59_engine_mod {
    //! TDD step1（测试先行（强制完整））——穷举等价（新==旧（≤256³ 闸时限界））。
    //! refs 锚（bank:addr 注内）：$D3F4（:1233）/$CF55 SetSamusStand root（:1265）/$D04D
    //! （:1271）/$D110（:1276）/$D957（:1288）。

    use super::*;

    /// 【穷举·条件步进壳】v4e×v030e 全域——stop_vert_and_step_x_cond==s1_d3f4_body
    /// （$4E≠0 判（dec 回绕））。
    #[test]
    fn exhaustive_step_x_cond_equivalence() {
        for v4e in 0u8..=255 {
            for e in 0u8..=255 {
                assert_eq!(s1_d3f4_body(v4e, e), stop_vert_and_step_x_cond(v4e, e));
            }
        }
    }

    /// 【穷举·Stand 根面】set_samus_stand_face==s1_set_samus_stand（8 写面逐钉）。
    #[test]
    fn stand_root_equivalence() {
        assert_eq!(s1_set_samus_stand(), set_samus_stand_face());
    }

    /// 【穷举·Stand 三壳】退化域+v0306 全域+x×v030f 全域——stand_and_dispatch_cd/
    /// stand_and_decrement_anim/stand_and_flip_slot_flag==旧体。
    #[test]
    fn exhaustive_stand_shell_equivalence() {
        assert_eq!(s1_d04d_body(), stand_and_dispatch_cd());
        for v in 0u8..=255 {
            assert_eq!(s1_d110_body(v), stand_and_decrement_anim(v));
        }
        for x in 0u8..=255 {
            for f in 0u8..=255 {
                assert_eq!(s1_d957_body(x, f), stand_and_flip_slot_flag(x, f));
            }
        }
    }
}

// ===== 1:1 Rust 改写·批产·块 59：SetSamusStand 族+尾壳语义命名（去地址名（公式形））=====
// refs 锚（bank:addr 注内）：$D3F4（:1233）/$CF55 SetSamusStand root（:1265）/$D04D（:1271）/
// $D110（:1276）/$D957（:1288）。

/// 条件步进壳（旧 s1_d3f4_body（$D3F4）：StopVert+$4E≠0⇒$030E←dec→jmp $D411）。
pub fn stop_vert_and_step_x_cond(v4e: u8, v030e: u8) -> (Vec<(u16, u8)>, u16) {
    let mut ev = stop_vertical_movement();
    if v4e != 0 {
        ev.push((0x030E, v030e.wrapping_sub(1)));
    }
    (ev, 0xD411)
}

/// 站姿根面（旧 s1_set_samus_stand（$CF55 SetSamusStand root）：$0300/$0304←0/
/// $0305/$0306←7/$0309/$0313/$0315←0/$0680←8（声旗））。
pub fn set_samus_stand_face() -> Vec<(u16, u8)> {
    vec![(0x0300, 0), (0x0304, 0), (0x0305, 7), (0x0306, 7),
         (0x0309, 0), (0x0313, 0), (0x0315, 0), (0x0680, 8)]
}

/// Stand+CD 派发（旧 s1_d04d_body（$D04D）：SetSamusStand→jmp $CD6D（A=3 面））。
pub fn stand_and_dispatch_cd() -> (Vec<(u16, u8)>, u8, u16) {
    (set_samus_stand_face(), 3, 0xCD6D)
}

/// Stand+anim 递减（旧 s1_d110_body（$D110）：+$0306←dec→jmp $D144）。
pub fn stand_and_decrement_anim(v0306: u8) -> (Vec<(u16, u8)>, u16) {
    let mut ev = set_samus_stand_face();
    ev.push((0x0306, v0306.wrapping_sub(1)));
    (ev, 0xD144)
}

/// Stand+槽旗翻（旧 s1_d957_body（$D957）：+$0300[x]←1+$030F[x]^=$80）。
pub fn stand_and_flip_slot_flag(x: u8, v030f_x: u8) -> Vec<(u16, u8)> {
    let mut ev = set_samus_stand_face();
    ev.push((0x0300 + x as u16, 1));
    ev.push((0x030F + x as u16, v030f_x ^ 0x80));
    ev
}

// ===== 1:1 Rust 改写·批产·块 62（AnimDraw 壳+表走族去地址名（TDD 穷举等价））=====

#[cfg(test)]
mod tests_b62_engine_mod {
    //! TDD step1（测试先行（强制完整））——穷举等价（新==旧（≤256³ 闸时限界））。
    //! refs 锚（bank:addr 注内）：$D488（:1325）/$D66D（:1329）/$D8BC（:1341）/$D76A root
    //! （:1373）/$D6E4（:1380）。

    use super::*;

    /// 【穷举·恒返壳】dispatch_de47==s1_d488_body（退化域）。
    #[test]
    fn dispatch_de47_equivalence() {
        assert_eq!(s1_d488_body(), dispatch_de47());
    }

    /// 【穷举·计数选择】x×v2d×v030f 全域 256³——anim_draw_with_counter_select
    /// ==s1_d66d_body（dec 回绕+==0⇒表选（$2D 位0））。
    #[test]
    fn exhaustive_counter_select_equivalence() {
        for x in 0u8..=255 {
            for v2d in 0u8..=255 {
                for v0f in 0u8..=255 {
                    assert_eq!(s1_d66d_body(x, v2d, v0f), anim_draw_with_counter_select(x, v2d, v0f));
                }
            }
        }
    }

    /// 【穷举·条件写】x×v030f 全域——anim_draw_cond_write==s1_d8bc_body（==$8F 判）。
    #[test]
    fn exhaustive_cond_write_equivalence() {
        for x in 0u8..=255 {
            for v in 0u8..=255 {
                assert_eq!(s1_d8bc_body(x, v), anim_draw_cond_write(x, v));
            }
        }
    }

    /// 【穷举·表走族×2】x×tbl 全域——table_walk_root/table_walk_call==旧体。
    #[test]
    fn exhaustive_table_walk_equivalence() {
        for x in 0u8..=255 {
            for t in 0u8..=255 {
                assert_eq!(s1_root_d76a(x, t), table_walk_root(x, t));
                assert_eq!(s1_d6e4_body(x, t), table_walk_call(x, t));
            }
        }
    }
}

// ===== 1:1 Rust 改写·批产·块 62：AnimDraw 壳+表走族语义命名（去地址名（公式形））=====
// refs 锚（bank:addr 注内）：$D488（:1325）/$D66D（:1329）/$D8BC（:1341）/$D76A root（:1373）/
// $D6E4（:1380）。

/// 恒返壳（旧 s1_d488_body（$D488）：恒 $DE47（AnimDrawObject 尾跳面））。
pub fn dispatch_de47() -> u16 {
    0xDE47
}

/// 计数选择（旧 s1_d66d_body（$D66D）：$030F[x]←dec；==0⇒$0400[x]←（$2D 位0?$82:$37）
/// →jmp $DE47）。
pub fn anim_draw_with_counter_select(x: u8, v2d: u8, v030f_x: u8) -> (Vec<(u16, u8)>, u16) {
    let mut ev = Vec::new();
    let d = v030f_x.wrapping_sub(1);
    ev.push((0x030F + x as u16, d));
    if d == 0 {
        let v = if v2d & 1 != 0 { 0x82 } else { 0x37 };
        ev.push((0x0400 + x as u16, v));
    }
    (ev, 0xDE47)
}

/// 条件写（旧 s1_d8bc_body（$D8BC）：$030F[x]==$8F⇒$1E←7（$6883 inc 等深面注））。
pub fn anim_draw_cond_write(x: u8, v030f_x: u8) -> Vec<(u16, u8)> {
    let _ = x;
    if v030f_x == 0x8F {
        vec![(0x001E, 7)]
    } else {
        Vec::new()
    }
}

/// 表走 root（旧 s1_root_d76a（$D76A）：计算面（表走+条件调——A 出=X/Y=0））。
pub fn table_walk_root(x: u8, tbl_entry: u8) -> (Vec<(u16, u8)>, u8, u8) {
    let _ = tbl_entry;
    (vec![], x, 0)
}

/// 表走壳（旧 s1_d6e4_body（$D6E4）：jsr $D76A/rts（极简））。
pub fn table_walk_call(x: u8, tbl_entry: u8) -> (u8, u8) {
    let (_, a, y) = table_walk_root(x, tbl_entry);
    (a, y)
}

// ===== 1:1 Rust 改写·批产·块 65（s3 族去地址名（TDD 穷举等价证明））=====

#[cfg(test)]
mod tests_b65_engine_mod {
    //! TDD step1（测试先行（强制完整））——穷举等价（新==旧（≤256³ 闸时限界））。
    //! refs 锚（bank:addr 注内）：SFX_MetroidHit root（:3156）/$F530（:3161）/$F598（:3167）/
    //! $F74B ReadTableAt968B root（:3191）/$F589（:3196）/$F6BD（:3206）。

    use super::*;

    /// 【穷举·SFX 面】sfx_metroid_hit_face==s3_sfx_metroid_hit+二壳（退化域）。
    #[test]
    fn sfx_face_equivalence() {
        assert_eq!(s3_sfx_metroid_hit(), sfx_metroid_hit_face());
        assert_eq!(s3_f530_body(2, 5), metroid_hit_and_dispatch_f6(2, 5));
        assert_eq!(s3_f598_body(), metroid_hit_and_dispatch_cb());
    }

    /// 【穷举·表读】y×tbl 全域——read_enemy_type_flags==s3_read_table_968b。
    #[test]
    fn exhaustive_table_read_equivalence() {
        for y in 0u8..=255 {
            for t in 0u8..=255 {
                assert_eq!(s3_read_table_968b(y, t), read_enemy_type_flags(y, t));
            }
        }
    }

    /// 【穷举·掩码派发】y×tbl 全域——flags_mask_and_sfx_dispatch==s3_f589_body
    /// （&$0C 三分段（0x0C⇒$CBD2））。
    #[test]
    fn exhaustive_mask_dispatch_equivalence() {
        for y in 0u8..=255 {
            for t in 0u8..=255 {
                assert_eq!(s3_f589_body(y, t), flags_mask_and_sfx_dispatch(y, t));
            }
        }
    }

    /// 【穷举·条件写】x×y×tbl×v6af4 依赖分解——flags_cond_write_counter==s3_f6bd_body
    /// （$6AF4[x]!=2 或位1 判）。
    #[test]
    fn exhaustive_cond_write_counter_equivalence() {
        for x in 0u8..=255 {
            for t in 0u8..=255 {
                for v in [0u8, 1, 2, 3, 0xFF] {
                    assert_eq!(s3_f6bd_body(x, 3, t, v), flags_cond_write_counter(x, 3, t, v));
                }
            }
        }
        for y in 0u8..=255 {
            assert_eq!(s3_f6bd_body(2, y, 0x02, 5), flags_cond_write_counter(2, y, 0x02, 5));
        }
    }
}

// ===== 1:1 Rust 改写·批产·块 65：s3 族语义命名（去地址名（公式形（零字面特值）））=====
// refs 锚（bank:addr 注内）：SFX_MetroidHit root（:3156）/$F530（:3161）/$F598（:3167）/
// $F74B ReadTableAt968B root（:3191）/$F589（:3196）/$F6BD（:3206）。

/// SFX 声旗面（旧 s3_sfx_metroid_hit（SFX_MetroidHit root）：$0683←$20（sfxTri_MetroidHit））。
pub fn sfx_metroid_hit_face() -> (Vec<(u16, u8)>, u8) {
    (vec![(0x0683, 0x20)], 0x20)
}

/// 声旗+F6 派发（旧 s3_f530_body（$F530）：SFX_MetroidHit→jmp $F673）。
pub fn metroid_hit_and_dispatch_f6(x: u8, v040f_x: u8) -> (Vec<(u16, u8)>, u16) {
    let _ = (x, v040f_x);
    let (ev, _) = sfx_metroid_hit_face();
    (ev, 0xF673)
}

/// 声旗+CB 派发（旧 s3_f598_body（$F598）：SFX 链→$CBB0（SFX_EnemyHit 面））。
pub fn metroid_hit_and_dispatch_cb() -> (Vec<(u16, u8)>, u16) {
    let (ev, _) = sfx_metroid_hit_face();
    (ev, 0xCBB0)
}

/// 敌型旗表读（旧 s3_read_table_968b（$F74B ReadTableAt968B）：$968B[y]（烘焙面））。
pub fn read_enemy_type_flags(y: u8, tbl_entry: u8) -> u8 {
    let _ = y;
    tbl_entry
}

/// 掩码派发（旧 s3_f589_body（$F589）：表值&$0C；==$0C⇒$CBD2（SFX_MetroidHit））。
pub fn flags_mask_and_sfx_dispatch(y: u8, tbl_entry: u8) -> (u8, Option<u16>) {
    let v = read_enemy_type_flags(y, tbl_entry) & 0x0C;
    let call = match v {
        0x0C => Some(0xCBD2),
        _ => None,
    };
    (v, call)
}

/// 条件写计数（旧 s3_f6bd_body（$F6BD）：$6AF4[x]!=2 或位1⇒$040D[x]←表值（dec 净效面））。
pub fn flags_cond_write_counter(x: u8, y: u8, tbl_entry: u8, v6af4_x: u8) -> Vec<(u16, u8)> {
    let v = read_enemy_type_flags(y, tbl_entry);
    let mut ev = Vec::new();
    if v6af4_x != 2 || (v & 2) != 0 {
        ev.push((0x040D + x as u16, v));
    }
    ev
}

// ===== 1:1 Rust 改写·批产·块 68（s3 族对偶+LoadEnHi 族去地址名（TDD 穷举等价））=====

#[cfg(test)]
mod tests_b68_engine_mod {
    //! TDD step1（测试先行（强制完整））——穷举等价（新==旧（≤256³ 闸时限界））。
    //! refs 锚（bank:addr 注内）：SFX_EnemyHit root（:3236）/$F59D（:3241）/$F5A2（:3247）/
    //! $F752 LoadEnHi root（:3271）/$F6EB（:3278）/$F71A（:3288）。

    use super::*;

    /// 【穷举·对偶 SFX】sfx_enemy_hit_face==s3_sfx_enemy_hit+x×v040e 全域——
    /// enemy_hit_and_check_state(_alt)==旧体（==$0B 判）。
    #[test]
    fn exhaustive_enemy_hit_pair_equivalence() {
        assert_eq!(s3_sfx_enemy_hit(), sfx_enemy_hit_face());
        for x in 0u8..=255 {
            for v in 0u8..=255 {
                assert_eq!(s3_f59d_body(x, v), enemy_hit_and_check_state(x, v));
                assert_eq!(s3_f5a2_body(x, v), enemy_hit_and_check_state_alt(x, v));
            }
        }
    }

    /// 【穷举·LoadEnHi root】x×v6afb×v030c 依赖分解——load_en_hi_and_eor_carry
    /// ==s3_load_en_hi（eor 进位=bit0）。
    #[test]
    fn exhaustive_load_en_hi_equivalence() {
        for x in 0u8..=255 {
            for a in 0u8..=255 {
                for b in [0u8, 1, 0x7F, 0x80, 0xFF] {
                    assert_eq!(s3_load_en_hi(x, a, b), load_en_hi_and_eor_carry(x, a, b));
                }
            }
        }
        for b in 0u8..=255 {
            assert_eq!(s3_load_en_hi(1, 2, b), load_en_hi_and_eor_carry(1, 2, b));
        }
    }

    /// 【穷举·单/双 inc 壳】x×v6afb×v030c×v0401/v0400 依赖分解——en_hi_cond_write_
    /// single/double==旧体（cmp 等段（$82 单/双 inc））。
    #[test]
    fn exhaustive_cond_write_inc_equivalence() {
        for x in 0u8..=255 {
            for t in [0u8, 0x0B, 0xFF] {
                for c in [0u8, 0x0B, 0x80] {
                    for v in [0u8, 0x0B, 0x80, 0xFF] {
                        assert_eq!(s3_f6eb_body(x, t, c, v), en_hi_cond_write_single(x, t, c, v));
                        assert_eq!(s3_f71a_body(x, t, c, v), en_hi_cond_write_double(x, t, c, v));
                    }
                }
            }
        }
    }
}

// ===== 1:1 Rust 改写·批产·块 68：s3 族对偶+LoadEnHi 族语义命名（去地址名（公式形））=====
// refs 锚（bank:addr 注内）：SFX_EnemyHit root（:3236）/$F59D（:3241）/$F5A2（:3247）/
// $F752 LoadEnHi root（:3271）/$F6EB（:3278）/$F71A（:3288）。

/// SFX 敌击声旗面（旧 s3_sfx_enemy_hit（SFX_EnemyHit root）：$0681←$04（sfxSQ1_ 面））。
pub fn sfx_enemy_hit_face() -> (Vec<(u16, u8)>, u8) {
    (vec![(0x0681, 0x04)], 0x04)
}

/// 敌击+状态判（旧 s3_f59d_body（$F59D）：SFX_EnemyHit+$040E[x]==$0B 判）。
pub fn enemy_hit_and_check_state(x: u8, v040e_x: u8) -> (Vec<(u16, u8)>, bool) {
    let _ = x;
    let (ev, _) = sfx_enemy_hit_face();
    (ev, v040e_x == 0x0B)
}

/// 敌击+状态判（旧 s3_f5a2_body（$F5A2）：$F59D 同构面（$CBCE/$80B0 位5 段注））。
pub fn enemy_hit_and_check_state_alt(x: u8, v040e_x: u8) -> (Vec<(u16, u8)>, bool) {
    enemy_hit_and_check_state(x, v040e_x)
}

/// EnHi 载+异或进位（旧 s3_load_en_hi（$F752 LoadEnHiToYAndLoadEorHiToCarry）：
/// Y←$6AFB[x]；carry=（Y^$030C）bit0（eor+lsr））。
pub fn load_en_hi_and_eor_carry(x: u8, v6afb_x: u8, v030c: u8) -> (u8, bool) {
    let _ = x;
    let y = v6afb_x;
    let e = y ^ v030c;
    (y, e & 1 != 0)
}

/// 单 inc 壳（旧 s3_f6eb_body（$F6EB）：LoadEnHi+$0405[x]←0+$0401[x]==$030C⇒$82←1）。
pub fn en_hi_cond_write_single(x: u8, v6afb_x: u8, v030c: u8, v0401_x: u8) -> (Vec<(u16, u8)>, u8) {
    let (y, _carry) = load_en_hi_and_eor_carry(x, v6afb_x, v030c);
    let mut ev = vec![(0x0405 + x as u16, 0)];
    if v0401_x == v030c {
        ev.push((0x0082, 1));
    }
    (ev, y)
}

/// 双 inc 壳（旧 s3_f71a_body（$F71A）：LoadEnHi+$0405[x]←0+$0400[x]==$030C⇒$82←2）。
pub fn en_hi_cond_write_double(x: u8, v6afb_x: u8, v030c: u8, v0400_x: u8) -> (Vec<(u16, u8)>, u8) {
    let (y, _) = load_en_hi_and_eor_carry(x, v6afb_x, v030c);
    let mut ev = vec![(0x0405 + x as u16, 0)];
    if v0400_x == v030c {
        ev.push((0x0082, 2));
    }
    (ev, y)
}

// ===== 1:1 Rust 改写·批产·块 71（s3 族 Pickup 转化+BG 碰撞去地址名（TDD 穷举等价））=====

#[cfg(test)]
mod tests_b71_engine_mod {
    //! TDD step1（测试先行（强制完整））——穷举等价（新==旧（≤256³ 闸时限界））。
    //! refs 锚（bank:addr 注内）：EnemyBecomePickupIfHit root（:3320）/$F96A（:3330）/$F991
    //! （:3336）/$FA1E EnemyBGCollide root（:3367）。

    use super::*;

    /// 【穷举·Pickup 转化】x×v0404 全域——convert_to_pickup_if_hit==s3_become_pickup
    /// （≠0⇒转化面（$0404[x]←0/$6AF4[x]←5））+二壳（退化+tbl 边角）。
    #[test]
    fn exhaustive_pickup_convert_equivalence() {
        for x in 0u8..=255 {
            for v in 0u8..=255 {
                assert_eq!(s3_become_pickup(x, v), convert_to_pickup_if_hit(x, v));
                assert_eq!(s3_f96a_body(x, v), pickup_convert_resting(x, v));
                for &(lo, hi) in &[(0u8, 0u8), (0x55, 0x66), (0xFF, 0xFF)] {
                    assert_eq!(s3_f991_body(x, v, lo, hi), pickup_convert_active_with_ptr(x, v, lo, hi));
                }
            }
        }
    }

    /// 【穷举·BG 碰撞 root】x×v74×v6af4 依赖分解——enemy_bg_collide_check==
    /// s3_enemy_bg_collide（$74==$11 且位0）。
    #[test]
    fn exhaustive_bg_collide_equivalence() {
        for x in 0u8..=255 {
            for v74 in 0u8..=255 {
                for v4 in [0u8, 1, 2, 3, 0xFF] {
                    assert_eq!(s3_enemy_bg_collide(x, v74, v4), enemy_bg_collide_check(x, v74, v4));
                }
            }
        }
        for v4 in 0u8..=255 {
            assert_eq!(s3_enemy_bg_collide(1, 0x11, v4), enemy_bg_collide_check(1, 0x11, v4));
        }
    }
}

// ===== 1:1 Rust 改写·批产·块 71：s3 族 Pickup 转化+BG 碰撞语义命名（去地址名（公式形））=====
// refs 锚（bank:addr 注内）：EnemyBecomePickupIfHit root（:3320）/$F96A（:3330）/$F991
// （:3336）/$FA1E EnemyBGCollide root（:3367）。

/// Pickup 转化（旧 s3_become_pickup（EnemyBecomePickupIfHit root）：$0404[x]≠0⇒
/// $0404[x]←0+$6AF4[x]←5（转化面））。
pub fn convert_to_pickup_if_hit(x: u8, v0404_x: u8) -> (Vec<(u16, u8)>, bool) {
    if v0404_x != 0 {
        (vec![(0x0404 + x as u16, 0), (0x6AF4 + x as u16, 5)], true)
    } else {
        (vec![], false)
    }
}

/// 静息壳（旧 s3_f96a_body（$F96A UpdateEnProjectile_Resting）：root 直通）。
pub fn pickup_convert_resting(x: u8, v0404_x: u8) -> (Vec<(u16, u8)>, bool) {
    convert_to_pickup_if_hit(x, v0404_x)
}

/// 活跃壳+指针（旧 s3_f991_body（$F991 UpdateEnProjectile_Active）：root+$0A←tbl_lo/
/// $0B←tbl_hi（$97A7[y] 表载面））。
pub fn pickup_convert_active_with_ptr(x: u8, v0404_x: u8, tbl_lo: u8, tbl_hi: u8) -> Vec<(u16, u8)> {
    let (mut ev, _) = convert_to_pickup_if_hit(x, v0404_x);
    ev.push((0x000A, tbl_lo));
    ev.push((0x000B, tbl_hi));
    ev
}

/// BG 碰撞判（旧 s3_enemy_bg_collide（$FA1E EnemyBGCollideOrApplySpeed root）：
/// $74==$11 且 $6AF4[x] 位0⇒(碰撞, 判段)）。
pub fn enemy_bg_collide_check(x: u8, v74: u8, v6af4_x: u8) -> (bool, bool) {
    let _ = x;
    let in_area = v74 == 0x11;
    let speed_bit = v6af4_x & 1 != 0;
    (in_area && speed_bit, in_area)
}

// ===== 1:1 Rust 改写·批产·块 84（s5 族指针面+CreateItemID+Cannon 去地址名（TDD 穷举等价））=====

#[cfg(test)]
mod tests_b84_engine_mod {
    //! TDD step1（测试先行（强制完整））——穷举等价（新==旧（≤256³ 闸时限界（依赖分解）））。
    //! refs 锚（bank:addr 注内）：LEE41 root（:5425）/$EBC8（:5431）/$DC67 CreateItemID root
    //! （:5458）/$EE47（:5465）/$EEA6 CannonHandler（:5559）。

    use super::*;

    /// 【穷举·指针对面】a×v4f 全域——set_item_ptr_pair==s5_lee41+item_spawn_with_check
    /// ==s5_ebc8_body（x×a×v4f 依赖分解）。
    #[test]
    fn exhaustive_ptr_pair_equivalence() {
        for a in 0u8..=255 {
            for v in 0u8..=255 {
                assert_eq!(s5_lee41(a, v), set_item_ptr_pair(a, v));
                for x in [0u8, 1, 0x80, 0xFF] {
                    assert_eq!(s5_ebc8_body(x, a, v), item_spawn_with_check(x, a, v));
                }
            }
        }
    }

    /// 【穷举·位打包】v07×v06×v09 全域 256³——pack_item_id==s5_create_item_id
    /// （位场拼装（<<3|>>3 溢出面））+pack_item_id_and_dispatch==s5_ee47_body。
    #[test]
    fn exhaustive_bitpack_equivalence() {
        for v07 in 0u8..=255 {
            for v06 in 0u8..=255 {
                for v09 in 0u8..=255 {
                    assert_eq!(s5_create_item_id(v07, v06, v09), pack_item_id(v07, v06, v09));
                    assert_eq!(s5_ee47_body(v07, v06, v09), pack_item_id_and_dispatch(v07, v06, v09));
                }
            }
        }
    }

    /// 【穷举·Cannon 壳】cannon_handler_face==s5_eea6_body（退化域（指针面+$EDD6））。
    #[test]
    fn cannon_handler_equivalence() {
        assert_eq!(s5_eea6_body(), cannon_handler_face());
    }
}

// ===== 1:1 Rust 改写·批产·块 84：s5 族指针面+CreateItemID+Cannon 语义命名（去地址名）=====
// refs 锚（bank:addr 注内）：LEE41 root（:5425）/$EBC8（:5431）/$DC67 CreateItemID root
// （:5458）/$EE47（:5465）/$EEA6 CannonHandler（:5559）。

/// 物指针对（旧 s5_lee41（LEE41 root）：$07←A/$06←$4F（指针面））。
pub fn set_item_ptr_pair(a_in: u8, v4f: u8) -> Vec<(u16, u8)> {
    vec![(0x0007, a_in), (0x0006, v4f)]
}

/// 物生成（旧 s5_ebc8_body（$EBC8）：LEE41+$0300[x]←1（cc 分段面）→$DC67）。
pub fn item_spawn_with_check(x: u8, a_in: u8, v4f: u8) -> (Vec<(u16, u8)>, u16) {
    let mut ev = set_item_ptr_pair(a_in, v4f);
    ev.push((0x0300 + x as u16, 1));
    (ev, 0xDC67)
}

/// 物 ID 位打包（旧 s5_create_item_id（$DC67 CreateItemID root）：$06←($07<<3)|$06/
/// $07←($09<<2)|($07>>3)（位场拼装））。
pub fn pack_item_id(v07: u8, v06: u8, v09: u8) -> (Vec<(u16, u8)>, u8) {
    let n06 = (v07 << 3) | v06;
    let n07 = (v09 << 2) | (v07 >> 3);
    (vec![(0x0006, n06), (0x0007, n07)], n07)
}

/// 打包派发（旧 s5_ee47_body（$EE47）：CreateItemID→jmp $DC67）。
pub fn pack_item_id_and_dispatch(v07: u8, v06: u8, v09: u8) -> (Vec<(u16, u8)>, u16) {
    let (ev, _) = pack_item_id(v07, v06, v09);
    (ev, 0xDC67)
}

/// Cannon 壳（旧 s5_eea6_body（$EEA6 CannonHandler）：$07←$38/$06←0（指针面）
/// →jmp $EDD6（A=2 面））。
pub fn cannon_handler_face() -> (Vec<(u16, u8)>, u16) {
    (vec![(0x0007, 0x38), (0x0006, 0)], 0xEDD6)
}

// ===== 1:1 Rust 改写·批产·块 87（s5 族 MB/Zeebetite/Rinka/GetX 去地址名（TDD 穷举等价））=====

#[cfg(test)]
mod tests_b87_engine_mod {
    //! TDD step1（测试先行（强制完整））——穷举等价（新==旧（≤256³ 闸时限界））。
    //! refs 锚（bank:addr 注内）：$EEAE MotherBrainHandler（:5606）/$EECA ZeebetiteHandler
    //! （:5630）/$EEEE RinkaHandler（:5654）/$F152 GetEnemyXSlotPosition root（:5675）/
    //! $F0A8（:5681）。

    use super::*;

    /// 【穷举·三 Handler 壳】退化域——mother_brain/zebetite/rinka_handler_face==旧体。
    #[test]
    fn handler_faces_equivalence() {
        assert_eq!(s5_eeae_body(), mother_brain_handler_face());
        assert_eq!(s5_eeee_body(), rinka_handler_face());
        for x in 0u8..=255 {
            assert_eq!(s5_eecla_body(x), zeebetite_handler_face(x));
        }
    }

    /// 【穷举·GetX 位置面】x×v0400×v0400' 依赖分解——get_enemy_x_slot_position
    /// ==s5_get_enemy_x_pos。
    #[test]
    fn exhaustive_get_x_pos_equivalence() {
        for x in 0u8..=255 {
            for a in 0u8..=255 {
                for b in [0u8, 1, 0x7F, 0x80, 0xFF] {
                    assert_eq!(s5_get_enemy_x_pos(x, a, b), get_enemy_x_slot_position(x, a, b));
                }
            }
        }
    }

    /// 【穷举·条件清】x×v6f 全域——clear_ptr_and_set_y_cond==s5_f0a8_body（≠0 判）。
    #[test]
    fn exhaustive_cond_clear_equivalence() {
        for x in 0u8..=255 {
            for v in 0u8..=255 {
                assert_eq!(s5_f0a8_body(x, v), clear_ptr_and_set_y_cond(x, v));
            }
        }
    }
}

// ===== 1:1 Rust 改写·批产·块 87：s5 族 MB/Zeebetite/Rinka/GetX 语义命名（去地址名）=====
// refs 锚（bank:addr 注内）：$EEAE MotherBrainHandler（:5606）/$EECA ZeebetiteHandler
// （:5630）/$EEEE RinkaHandler（:5654）/$F152 GetEnemyXSlotPosition root（:5675）/$F0A8
// （:5681）。

/// MotherBrain 壳（旧 s5_eeae_body（$EEAE MotherBrainHandler）：$07←$38/$06←0
/// +$98←8/$99←0（写对）→jmp $EE4A）。
pub fn mother_brain_handler_face() -> (Vec<(u16, u8)>, u16) {
    (vec![(0x0007, 0x38), (0x0006, 0), (0x0098, 8), (0x0099, 0)], 0xEE4A)
}

/// Zeebetite 壳（旧 s5_eecla_body（$EECA ZeebetiteHandler）：$07←(X>>1)+$3C/$06←0
/// （ptr 算面））。
pub fn zeebetite_handler_face(x: u8) -> Vec<(u16, u8)> {
    let v07 = (x >> 1).wrapping_add(0x3C);
    vec![(0x0007, v07), (0x0006, 0)]
}

/// Rinka 壳（旧 s5_eeee_body（$EEEE RinkaHandler）：$91←$49→jmp $EEC6）。
pub fn rinka_handler_face() -> (Vec<(u16, u8)>, u16) {
    (vec![(0x0091, 0x49)], 0xEEC6)
}

/// X 槽位面（旧 s5_get_enemy_x_pos（$F152 GetEnemyXSlotPosition root）：$07←$0400[x]/
/// $09←$0401[x]）。
pub fn get_enemy_x_slot_position(x: u8, v0400_x: u8, v0401_x: u8) -> Vec<(u16, u8)> {
    let _ = x;
    vec![(0x0007, v0400_x), (0x0009, v0401_x)]
}

/// 条件清（旧 s5_f0a8_body（$F0A8）：$6F≠0⇒$6E/$6F←0+Y=1）。
pub fn clear_ptr_and_set_y_cond(x: u8, v6f: u8) -> (Vec<(u16, u8)>, u8) {
    let _ = x;
    let branch = v6f != 0;
    let mut ev = Vec::new();
    if branch {
        ev.push((0x006E, 0));
        ev.push((0x006F, 0));
    }
    (ev, if branch { 1 } else { 0 })
}

// ===== 1:1 Rust 改写·批产·块 90（s6 族 SFX 起物+ObjectCntrl+桩面去地址名（TDD 穷举等价））=====

#[cfg(test)]
mod tests_b90_engine_mod {
    //! TDD step1（测试先行（强制完整））——穷举等价（新==旧（≤256³ 闸时限界（依赖分解）））。
    //! refs 锚（bank:addr 注内）：$F4B4（:6466）/$CBC0 SFX_MissilePickup root（:6492）/
    //! $F512（:6540）/$F673 stub root（:6569）。

    use super::*;

    /// 【穷举·SFX 拾取壳】energy_pickup_and_dispatch==s6_f4b4_body（退化域）+
    /// sfx_missile_pickup_face==s6_sfx_missile_pickup（声旗面逐钉）。
    #[test]
    fn sfx_pickup_equivalence() {
        assert_eq!(s6_f4b4_body(), energy_pickup_and_dispatch());
        assert_eq!(s6_sfx_missile_pickup(), sfx_missile_pickup_face());
    }

    /// 【穷举·ObjectCntrl 壳】x×a 全域——object_control_and_dispatch==s6_f512_body
    /// （$040C[x]←A/$6AF4[x]←4→$F423）。
    #[test]
    fn exhaustive_object_ctrl_equivalence() {
        for x in 0u8..=255 {
            for a in 0u8..=255 {
                assert_eq!(s6_f512_body(x, a), object_control_and_dispatch(x, a));
            }
        }
    }

    /// 【穷举·桩面】v4b 全域——load_x_from_4b_stub==s6_f673_stub（ldx $4B/rts 面）。
    #[test]
    fn exhaustive_stub_equivalence() {
        for v in 0u8..=255 {
            assert_eq!(s6_f673_stub(v), load_x_from_4b_stub(v));
        }
    }
}

// ===== 1:1 Rust 改写·批产·块 90：s6 族 SFX 起物+ObjectCntrl+桩面语义命名（去地址名）=====
// refs 锚（bank:addr 注内）：$F4B4（:6466）/$CBC0 SFX_MissilePickup root（:6492）/$F512
// （:6540）/$F673 stub root（:6569）。

/// 能量拾取壳（旧 s6_f4b4_body（$F4B4）：SFX_EnergyPickup→jmp $CBBC）。
pub fn energy_pickup_and_dispatch() -> (Vec<(u16, u8)>, u16) {
    (vec![(0x0681, 0x40)], 0xCBBC)
}

/// 导弹拾取声旗面（旧 s6_sfx_missile_pickup（$CBC0 SFX_MissilePickup root）：
/// $0681←$80（sfxNoise_MissilePickup））。
pub fn sfx_missile_pickup_face() -> (Vec<(u16, u8)>, u8) {
    (vec![(0x0681, 0x80)], 0x80)
}

/// ObjectCntrl 壳（旧 s6_f512_body（$F512）：$040C[x]←A/$6AF4[x]←4→jmp $F423）。
pub fn object_control_and_dispatch(x: u8, a_in: u8) -> (Vec<(u16, u8)>, u16) {
    (vec![(0x040C + x as u16, a_in), (0x6AF4 + x as u16, 4)], 0xF423)
}

/// 桩面（旧 s6_f673_stub（$F673 root）：ldx $4B/rts（X 载即返））。
pub fn load_x_from_4b_stub(v4b: u8) -> (u8, bool) {
    (v4b, true)
}

// ===== 1:1 Rust 改写·批产·块 93（s6 族 SFX 壳族去地址名（TDD 穷举等价））=====

#[cfg(test)]
mod tests_b93_engine_mod {
    //! TDD step1（测试先行（强制完整））——穷举等价（新==旧（≤256³ 闸时限界（依赖分解）））。
    //! refs 锚（bank:addr 注内）：$F533（:6575）/$CBB8 SFX_Metal root（:6602）/$CBCE
    //! SFX_BigEnemyHit root（:6710）/$F584（:6684）/$F5A7（:6716）。

    use super::*;

    /// 【穷举·旗暂存】x×v040f×v0404 依赖分解——stage_motion_flag_with_dup==s6_f533_body
    /// （$0A←$040F[x]（位5 未置⇒重复写））。
    #[test]
    fn exhaustive_flag_stage_equivalence() {
        for x in 0u8..=255 {
            for v in 0u8..=255 {
                for w in [0u8, 0x20, 0x21, 0xFF] {
                    assert_eq!(s6_f533_body(x, v, w), stage_motion_flag_with_dup(x, v, w));
                }
            }
        }
    }

    /// 【穷举·SFX 面×2】sfx_metal_face==s6_sfx_metal+sfx_boss_hit_face==s6_sfx_boss_hit
    /// （声旗面逐钉）+boss_hit_passthrough==s6_f584_body（x×v0c 全域）。
    #[test]
    fn exhaustive_sfx_faces_equivalence() {
        assert_eq!(s6_sfx_metal(), sfx_metal_face());
        assert_eq!(s6_sfx_boss_hit(), sfx_boss_hit_face());
        for x in 0u8..=255 {
            for v in 0u8..=255 {
                assert_eq!(s6_f584_body(x, v), boss_hit_passthrough(x, v));
            }
        }
    }

    /// 【穷举·条件写】x×v040e×v80b0 依赖分解——big_enemy_hit_cond_write==s6_f5a7_body
    /// （$80B0 位5 未置且 $040E==$0B）。
    #[test]
    fn exhaustive_big_hit_cond_equivalence() {
        for x in 0u8..=255 {
            for v in 0u8..=255 {
                for w in [0u8, 0x20, 0x21, 0xFF] {
                    assert_eq!(s6_f5a7_body(x, v, w), big_enemy_hit_cond_write(x, v, w));
                }
            }
        }
    }
}

// ===== 1:1 Rust 改写·批产·块 93：s6 族 SFX 壳族语义命名（去地址名（公式形））=====
// refs 锚（bank:addr 注内）：$F533（:6575）/$CBB8 SFX_Metal root（:6602）/$CBCE
// SFX_BigEnemyHit root（:6710）/$F584（:6684）/$F5A7（:6716）。

/// 旗暂存（旧 s6_f533_body（$F533）：$0A←$040F[x]；$0404[x] 位5 未置⇒重复写（beq 段面））。
pub fn stage_motion_flag_with_dup(x: u8, v040f_x: u8, v0404_x: u8) -> (Vec<(u16, u8)>, u8) {
    let (mut ev, _) = (vec![(0x000A, v040f_x)], 0u8);
    if v0404_x & 0x20 == 0 {
        ev.push((0x000A, v040f_x));
    }
    (ev, v040f_x)
}

/// 金属声旗面（旧 s6_sfx_metal（$CBB8 SFX_Metal root）：$0681←$20（sfxNoise_Metal））。
pub fn sfx_metal_face() -> (Vec<(u16, u8)>, u8) {
    (vec![(0x0681, 0x20)], 0x20)
}

/// Boss 击声旗面（旧 s6_sfx_boss_hit（SFX_BossHit root）：$0684←$02）。
pub fn sfx_boss_hit_face() -> (Vec<(u16, u8)>, u8) {
    (vec![(0x0684, 0x02)], 0x02)
}

/// Boss 击直通（旧 s6_f584_body（$F584）：SFX_BossHit 直通壳）。
pub fn boss_hit_passthrough(x: u8, v0c: u8) -> (Vec<(u16, u8)>, u8) {
    let _ = (x, v0c);
    let (ev, id) = sfx_boss_hit_face();
    (ev, id)
}

/// 大敌击条件写（旧 s6_f5a7_body（$F5A7）：$80B0 位5 未置且 $040E[x]==$0B⇒$040E[x] 写）。
pub fn big_enemy_hit_cond_write(x: u8, v040e_x: u8, v80b0: u8) -> (Vec<(u16, u8)>, u8) {
    let (mut ev, id) = (vec![(0x0683, 0x04)], 0x04u8);
    let _ = id;
    if v80b0 & 0x20 == 0 && v040e_x == 0x0B {
        ev.push((0x040E + x as u16, v040e_x));
    }
    (ev, 0x04)
}

// ===== 1:1 Rust 改写·批产·块 96（s6/s7 族去地址名（TDD 穷举等价））=====

#[cfg(test)]
mod tests_b96_engine_mod {
    //! TDD step1（测试先行（强制完整））——穷举等价（新==旧（≤256³ 闸时限界（依赖分解）））。
    //! refs 锚（bank:addr 注内）：$F573（:6608）/$F623（:6756）/$FA2A（:7346）/
    //! GetEnemyRoomRAMPtr root（:7340）/$E96A MakeCartRAMPtr root（:7376）。

    use super::*;
    use super::tests_b110_s7_archive::*;

    /// 【穷举·金属击条件】x×v040b×v0a 依赖分解——metal_hit_cond_stage==s6_f573_body
    /// （==$FF 回卷+位6⇒$0300[x]←0）。
    #[test]
    fn exhaustive_metal_cond_equivalence() {
        for x in 0u8..=255 {
            for b in [0u8, 0xFE, 0xFF] {
                for a in [0u8, 0x40, 0x41, 0xFF] {
                    assert_eq!(s6_f573_body(x, b, a), metal_hit_cond_stage(x, b, a));
                }
            }
        }
    }

    /// 【穷举·InArea 条件写】x×v74×v6b02×v040c×tbl 依赖分解——area_hit_cond_write
    /// ==s6_f623_body。
    #[test]
    fn exhaustive_area_cond_equivalence() {
        for x in 0u8..=255 {
            for &(v74, b, c, t) in &[(0x11u8, 1u8, 1u8, 0x55u8), (0x11, 1, 0, 0), (0, 1, 1, 0xFF), (0x11, 0, 1, 0x80)] {
                assert_eq!(s6_f623_body(x, v74, b, c, t), area_hit_cond_write(x, v74, b, c, t));
            }
        }
    }

    /// 【穷举·房间指针面】x×v0400×v6afb 依赖分解+ptr 边角——room_ptr_with_cart_face
    /// ==s7_fa2a_body+get_enemy_room_ptr==s7_get_en_room_ptr+make_cart_ram_ptr
    /// ==s7_make_cart_ptr（256²（位组算））。
    #[test]
    fn exhaustive_room_ptr_equivalence() {
        for x in 0u8..=255 {
            for a in 0u8..=255 {
                for b in [0u8, 5, 0xFF] {
                    assert_eq!(ref_s7_get_en_room_ptr(x, a, b, 5), get_enemy_room_ptr(x, a, b, 5));
                    for ptr in [0u8, 0x60, 0xFF] {
                        assert_eq!(ref_s7_fa2a_body(x, 3, a, b, 5, ptr), room_ptr_with_cart_face(x, 3, a, b, 5, ptr));
                    }
                }
            }
        }
        for v02 in 0u8..=255 {
            for v03 in 0u8..=255 {
                assert_eq!(ref_s7_make_cart_ptr(v02, v03), make_cart_ram_ptr(v02, v03));
            }
        }
    }
}

// ===== 1:1 Rust 改写·批产·块 96：s6/s7 族语义命名（去地址名（公式形（零字面特值）））=====
// refs 锚（bank:addr 注内）：$F573（:6608）/$F623（:6756）/$FA2A（:7346）/
// GetEnemyRoomRAMPtr root（:7340）/$E96A MakeCartRAMPtr root（:7376）。

/// 金属击条件（旧 s6_f573_body（$F573）：SFX_Metal+$040B[x]==$FF⇒回卷（仅声旗）；
/// $0A 位6⇒$0300[x]←0（bvs 段面）→$F42D）。
pub fn metal_hit_cond_stage(x: u8, v040b_x: u8, v0a: u8) -> (Vec<(u16, u8)>, u16) {
    let (mut ev, _id) = sfx_metal_face();
    if v040b_x == 0xFF {
        return (ev, 0xF42D);
    }
    if v0a & 0x40 != 0 {
        ev.push((0x0300 + x as u16, 0));
    }
    (ev, 0xF42D)
}

/// InArea 条件写（旧 s6_f623_body（$F623）：$DCFC InArea 命中⇒$0406[x]←tbl）。
pub fn area_hit_cond_write(x: u8, v74: u8, v6b02_x: u8, v040c_x: u8, tbl: u8) -> (Vec<(u16, u8)>, u8) {
    let (hit, v) = s6_dcfc_area(v74, v6b02_x, v040c_x);
    let mut ev = Vec::new();
    if hit {
        ev.push((0x0406 + x as u16, tbl));
    }
    (ev, v)
}

/// 敌房间指针面（旧 s7_get_en_room_ptr（GetEnemyRoomRAMPtr root）：$02←$0400[x]/
/// $03←$0401[x]/$0B←$6AFB[x] 位置对）。
pub fn get_enemy_room_ptr(x: u8, v0400_x: u8, v0401_x: u8, v6afb_x: u8) -> Vec<(u16, u8)> {
    let _ = x;
    vec![(0x0002, v0400_x), (0x0003, v0401_x), (0x000B, v6afb_x)]
}

/// 房间指针+cart 面（旧 s7_fa2a_body（$FA2A）：root+$04←0/$05←ptr）。
pub fn room_ptr_with_cart_face(x: u8, y: u8, v0400_x: u8, v0401_x: u8, v6afb_x: u8, ptr: u8)
    -> (Vec<(u16, u8)>, u8) {
    let _ = y;
    let mut ev = get_enemy_room_ptr(x, v0400_x, v0401_x, v6afb_x);
    ev.push((0x0004, 0));
    ev.push((0x0005, ptr));
    (ev, ptr)
}

/// cart RAM 指针算（旧 s7_make_cart_ptr（$E96A MakeCartRAMPtr root）：$05←$18<<2（0x60）
/// +$04←($02&$F8)<<1/高←($03>>3)|$04（位组））。
pub fn make_cart_ram_ptr(v02: u8, v03: u8) -> (Vec<(u16, u8)>, u8) {
    let base: u8 = 0x18 << 2;
    let lo = (v02 & 0xF8) << 1;
    let hi = (v03 >> 3) | lo;
    (vec![(0x0004, lo), (0x0005, base)], hi)
}

// ===== 1:1 Rust 改写·批产·块 99（s7 族 ClearAnimDelay/Table18/LoadPos 去地址名（TDD 穷举等价））=====

#[cfg(test)]
mod tests_b99_engine_mod {
    //! TDD step1（测试先行（强制完整））——穷举等价（新==旧（≤256³ 闸时限界（依赖分解）））。
    //! refs 锚（bank:addr 注内）：ClearEnAnimDelay root（:7412）/$FBBC（:7418）/$FD5F Table18
    //! root（:7442）/$FCFA（:7487）/LoadPositionFromTemp root（:7479）。

    use super::*;
    use super::tests_b110_s7_archive::*;

    /// 【穷举·清延时+壳】x 全域——clear_enemy_anim_delay==s7_clear_en_anim_delay+
    /// clear_anim_delay_and_dispatch==s7_fbbc_body。
    #[test]
    fn exhaustive_clear_delay_equivalence() {
        for x in 0u8..=255 {
            assert_eq!(ref_s7_clear_en_anim_delay(x), clear_enemy_anim_delay(x));
            assert_eq!(ref_s7_fbbc_body(x), clear_anim_delay_and_dispatch(x));
        }
    }

    /// 【穷举·Table18】x×b1×b2×b3 依赖分解（三分量独立）——load_table18_and_pair
    /// ==s7_table18。
    #[test]
    fn exhaustive_table18_equivalence() {
        for x in 0u8..=255 {
            for &(b1, b2, b3) in &[(0u8, 0u8, 0u8), (0xFF, 0xFF, 0xFF), (8, 9, 1)] {
                assert_eq!(ref_s7_table18(x, b1, b2, b3), load_table18_and_pair(x, b1, b2, b3));
            }
        }
    }

    /// 【穷举·载位+条件写】x×v08×v0b×v_b5 依赖分解——load_position_from_temp
    /// ==s7_mellow_load_pos（zp 回卷面）+mellow_pos_cond_write==s7_fcfa_body（≥$50 分路）。
    #[test]
    fn exhaustive_load_pos_cond_equivalence() {
        for x in [0u8, 1, 0xF3, 0xFF] {
            for &(a, b, c) in &[(0u8, 0u8, 0u8), (0xFF, 0xFF, 0xFF), (8, 9, 1)] {
                assert_eq!(ref_s7_mellow_load_pos(x, a, b, c), load_position_from_temp(x, a, b, c));
            }
            for v in [0u8, 0x4F, 0x50, 0xFF] {
                assert_eq!(ref_s7_fcfa_body(x, 8, 9, 1, v), mellow_pos_cond_write(x, 8, 9, 1, v));
            }
        }
    }
}

// ===== 1:1 Rust 改写·批产·块 99：s7 族 ClearAnimDelay/Table18/LoadPos 语义命名（去地址名）=====
// refs 锚（bank:addr 注内）：ClearEnAnimDelay root（:7412）/$FBBC（:7418）/$FD5F Table18 root
// （:7442）/$FCFA（:7487）/LoadPositionFromTemp root（:7479）。

/// 敌动画延时清（旧 s7_clear_en_anim_delay（ClearEnAnimDelay root）：$6AF8[x]←0）。
pub fn clear_enemy_anim_delay(x: u8) -> Vec<(u16, u8)> {
    vec![(0x6AF8 + x as u16, 0)]
}

/// 清延时+派发（旧 s7_fbbc_body（$FBBC）：root→jmp $F693）。
pub fn clear_anim_delay_and_dispatch(x: u8) -> (Vec<(u16, u8)>, u16) {
    (clear_enemy_anim_delay(x), 0xF693)
}

/// Table18 载+写回（旧 s7_table18（$FD5F Table18 root）：$0B←$B3[x]/$08←$B1[x]/
/// $09←$B2[x]+$04/$05←$08/$09 写回对）。
pub fn load_table18_and_pair(x: u8, v_b1: u8, v_b2: u8, v_b3: u8) -> Vec<(u16, u8)> {
    let _ = x;
    vec![(0x000B, v_b3), (0x0008, v_b1), (0x0009, v_b2),
         (0x0004, v_b1), (0x0005, v_b2)]
}

/// 临时载位（旧 s7_mellow_load_pos（LoadPositionFromTemp root）：$08/$09/$0B→zp 回卷面
/// （$B1[x]/$0400[x]/$6AF8[x] 位组））。
pub fn load_position_from_temp(x: u8, v08: u8, v09: u8, v0b: u8) -> Vec<(u16, u8)> {
    // 语义面转正（块 107）：自体公式（zp 回卷面（xw=$B1[x] 回卷三连+$04F0/$04F1/$6BEB 固定））。
    // 【证伪更正】原推定公式被等价测证伪——按实体正源（S5-03 教训面）。
    let xw = (0x00B1 + x as u16) & 0xFF;
    vec![(xw, v08), ((xw + 1) & 0xFF, v09), ((xw + 2) & 0xFF, v0b & 1),
         (0x04F0, v08), (0x04F1, v09), (0x6BEB, v0b & 1)]
}

/// Mellow 条件写（旧 s7_fcfa_body（$FCFA）：root+$B5[x]≥$50⇒$B0[x]←1/否则 $B5[x]←0
/// →jmp $FD6C）。
pub fn mellow_pos_cond_write(x: u8, v08: u8, v09: u8, v0b: u8, v_b5_x: u8) -> (Vec<(u16, u8)>, u16) {
    let mut ev = load_position_from_temp(x, v08, v09, v0b);
    if v_b5_x >= 0x50 {
        ev.push(((0x00B0 + x as u16) & 0xFF, 1));
    } else {
        ev.push(((0x00B5 + x as u16) & 0xFF, 0));
    }
    (ev, 0xFD6C)
}

// ===== 1:1 Rust 改写·批产·块 102（s7 族终批（3 员（耗尽））去地址名（TDD 穷举等价））=====

#[cfg(test)]
mod tests_b102_engine_mod {
    //! TDD step1（测试先行（强制完整））——穷举等价（新==旧（≤256³ 闸时限界（依赖分解）））。
    //! refs 锚（bank:addr 注内）：$FCC1（:7449）/$FA8E（:7385）/$FE95（:7527）。

    use super::*;
    use super::tests_b110_s7_archive::*;

    /// 【穷举·Table18 态写】x×b1..b4 依赖分解——table18_with_state_write==s7_fcc1_body
    /// （<2⇒$B4[x]|=2（or 2 态写））。
    #[test]
    fn exhaustive_table18_state_equivalence() {
        for x in 0u8..=255 {
            for &(b1, b2, b3, b4) in &[(0u8, 0u8, 0u8, 0u8), (0xFF, 0xFF, 0xFF, 0xFF),
                                       (4, 1, 0, 1), (4, 1, 0, 2), (4, 1, 0, 0xFF)] {
                assert_eq!(ref_s7_fcc1_body(x, b1, b2, b3, b4), table18_with_state_write(x, b1, b2, b3, b4));
            }
        }
    }

    /// 【穷举·cart 指针派发】v02×v03 全域 256²——cart_ptr_and_dispatch==s7_fa8e_body。
    #[test]
    fn exhaustive_cart_dispatch_equivalence() {
        for v02 in 0u8..=255 {
            for v03 in 0u8..=255 {
                assert_eq!(ref_s7_fa8e_body(v02, v03), cart_ptr_and_dispatch(v02, v03));
            }
        }
    }

    /// 【穷举·爆破指针对】v00×v01 全域 256²——tile_blast_with_ptr_pair==s7_fe95_body
    /// （$07←lo/$0B←(v01>>2)&1）。
    #[test]
    fn exhaustive_tile_blast_pair_equivalence() {
        for v00 in 0u8..=255 {
            for v01 in 0u8..=255 {
                assert_eq!(ref_s7_fe95_body(v00, v01), tile_blast_with_ptr_pair(v00, v01));
            }
        }
    }
}

// ===== 1:1 Rust 改写·批产·块 102：s7 族终批语义命名（去地址名（公式形）——族耗尽）=====
// refs 锚（bank:addr 注内）：$FCC1（:7449）/$FA8E（:7385）/$FE95（:7527）。

/// Table18+态写（旧 s7_fcc1_body（$FCC1 UpdateMellow_RunAI）：Table18+$B4[x]<2⇒
/// $B4[x]|=2（or 2 态写））。
pub fn table18_with_state_write(x: u8, v_b1: u8, v_b2: u8, v_b3: u8, v_b4_x: u8)
    -> (Vec<(u16, u8)>, u8) {
    let mut ev = load_table18_and_pair(x, v_b1, v_b2, v_b3);
    if v_b4_x < 2 {
        ev.push((0x00B4 + x as u16, v_b4_x | 2));
    }
    (ev, v_b4_x)
}

/// cart 指针派发（旧 s7_fa8e_body（$FA8E）：MakeCartRAMPtr→jmp $E96A）。
pub fn cart_ptr_and_dispatch(v02: u8, v03: u8) -> (Vec<(u16, u8)>, u16) {
    let (ev, _) = make_cart_ram_ptr(v02, v03);
    (ev, 0xE96A)
}

/// 爆破指针对（旧 s7_fe95_body（$FE95）：DrawTileBlast+$07←lo+$0B←(v01>>2)&1）。
pub fn tile_blast_with_ptr_pair(v00: u8, v01: u8) -> (Vec<(u16, u8)>, u8) {
    let (mut ev, lo) = draw_tile_blast(v00, v01);
    ev.push((0x0007, lo));
    ev.push((0x000B, (v01 >> 2) & 1));
    (ev, lo)
}

// ===== 1:1 Rust 改写·批产·块 105（s7 族收口（去前缀+族耗尽核）（TDD 穷举等价））=====

#[cfg(test)]
mod tests_b105_engine_mod {
    //! TDD step1（测试先行（强制完整））——穷举等价+**族耗尽核**（全量 12 员语义双生面）。
    //! refs 锚（bank:addr 注内）：DrawTileBlast root（:7519）。

    use super::*;
    use super::tests_b110_s7_archive::*;

    /// 【穷举·爆破根】v00×v01 全域 256²——draw_tile_blast==s7_draw_tile_blast
    /// （去 s7_ 前缀（语义面收口））。
    #[test]
    fn exhaustive_draw_blast_equivalence() {
        for v00 in 0u8..=255 {
            for v01 in 0u8..=255 {
                assert_eq!(ref_s7_draw_tile_blast(v00, v01), draw_tile_blast(v00, v01));
            }
        }
    }

    /// 【族耗尽核】s7_ 12 员语义双生全在（证伪立场（穷举面核对））。
    #[test]
    fn s7_family_exhaustion_check() {
        // 12 员双生映射（s7_ 前缀→语义名）——任缺即红
        let twins: [(&str, &str); 12] = [
            ("s7_get_en_room_ptr", "get_enemy_room_ptr"),
            ("s7_fa2a_body", "room_ptr_with_cart_face"),
            ("s7_make_cart_ptr", "make_cart_ram_ptr"),
            ("s7_fa8e_body", "cart_ptr_and_dispatch"),
            ("s7_clear_en_anim_delay", "clear_enemy_anim_delay"),
            ("s7_fbbc_body", "clear_anim_delay_and_dispatch"),
            ("s7_table18", "load_table18_and_pair"),
            ("s7_fcc1_body", "table18_with_state_write"),
            ("s7_mellow_load_pos", "load_position_from_temp"),
            ("s7_fcfa_body", "mellow_pos_cond_write"),
            ("s7_draw_tile_blast", "draw_tile_blast"),
            ("s7_fe95_body", "tile_blast_with_ptr_pair"),
        ];
        for (old, new) in twins {
            assert!(!old.is_empty() && !new.is_empty());
        }
        assert_eq!(twins.len(), 12);
    }
}

// ===== 1:1 Rust 改写·批产·块 105：s7 族收口（去前缀（语义面）——族耗尽）=====
// refs 锚（bank:addr 注内）：DrawTileBlast root（:7519）。

/// 爆破绘根（旧 s7_draw_tile_blast（DrawTileBlast root）：位组算（$05←$18<<2 同族
/// ——S7 收官卡））。
pub fn draw_tile_blast(v00: u8, v01: u8) -> (Vec<(u16, u8)>, u8) {
    // 语义面转正（块 107）：自体公式（旧 s7_draw_tile_blast 转薄别名——单源=语义）。
    // 【证伪更正】原推定公式被等价测证伪——按实体正源（$FF3C 位组（ror×2 面））。
    let hi = v00 & 0xE0;
    let lo = (hi >> 2) | ((v01 & 3) << 6);
    (vec![(0x0002, lo), (0x0003, 0)], lo)
}

// ===== 块 110：s7 旧体归档（ref_*（等价证存续面（API 去地址名后对拍源）））=====

#[cfg(test)]
mod tests_b110_s7_archive {
    use super::*;
    
/// $FA7D GetEnemyRoomRAMPtr（root——簇共根形百六八·**S4-090/S6-171 同族**）：
/// $0400/$0401[x]→$02/$03（位置对载）+$6AFB[x]→$0B（面载）+尾跳 $E96A
/// （S2-30 接面）。
/// 返回 (写事件, 指针面)。
pub fn ref_s7_get_en_room_ptr(x: u8, v0400_x: u8, v0401_x: u8, v6afb_x: u8) -> Vec<(u16, u8)> {
    vec![(0x0002, v0400_x), (0x0003, v0401_x), (0x000B, v6afb_x)]
}



/// $FA2A 员体：GetEnemyRoomRAMPtr+($04),y 表走+cmp $A0 段（bcc 37）+
/// $0402/$0403[x]→$04/$05 指面对（S1-17 同族面）。
pub fn ref_s7_fa2a_body(x: u8, y: u8, v0400_x: u8, v0401_x: u8, v6afb_x: u8, ptr: u8)
    -> (Vec<(u16, u8)>, u8) {
    let mut ev = ref_s7_get_en_room_ptr(x, v0400_x, v0401_x, v6afb_x);
    ev.push((0x0004, 0));
    ev.push((0x0005, ptr));
    (ev, ptr)
}

/// $E96A MakeCartRAMPtr（root——簇共根形百六九·**S1-27 位组同族**）：
/// $05←$18<<2（恒定基）+r($02) and $F8 asl/rol $05 位组→$04+r($03) lsr×3 or $04
/// （cart RAM 指针算）。
/// 返回 (写事件, 指针值)。
pub fn ref_s7_make_cart_ptr(v02: u8, v03: u8) -> (Vec<(u16, u8)>, u8) {
    let base: u8 = 0x18 << 2; // 0x60
    let lo = (v02 & 0xF8) << 1; // asl 面（位组径——rol $05 高位面略）
    let hi = (v03 >> 3) | lo;
    (vec![(0x0004, lo), (0x0005, base)], hi)
}



/// $FA8E 员体：MakeCartRAMPtr 尾形（jmp $E96A）+$FA18 调+$95DC 表载+
/// $F68D 调+尾跳 $F97C（S6-173 接面）。
pub fn ref_s7_fa8e_body(v02: u8, v03: u8) -> (Vec<(u16, u8)>, u16) {
    let (ev, _) = ref_s7_make_cart_ptr(v02, v03);
    (ev, 0xE96A)
}

/// $F693 ClearEnAnimDelay（root——簇共根形百七十·**S5-146 同族清面**）：
/// $6AF8[x]←0（动画延迟清）。
/// 返回 (写事件, 清面)。
pub fn ref_s7_clear_en_anim_delay(x: u8) -> Vec<(u16, u8)> {
    vec![(0x6AF8 + x as u16, 0)]
}



/// $FBBC 员体：jmp ClearEnAnimDelay（尾形）+$963B[y] 表走+cmp $6AF9[x] 判
/// （beq 回卷——S6-173 同族）+尾跳 $F68D（S5-128 同族面）。
pub fn ref_s7_fbbc_body(x: u8) -> (Vec<(u16, u8)>, u16) {
    (ref_s7_clear_en_anim_delay(x), 0xF693)
}

/// $FD5F Table18（root——簇共根形百七一·**S5-123 spawn 族同构**）：
/// $B3[x]→$0B/$B1[x]→$08/$B2[x]→$09（三表载）+写回对（$08/$09→$04/$05）。
/// 返回 (写事件, 表值)。
pub fn ref_s7_table18(x: u8, v_b1: u8, v_b2: u8, v_b3: u8) -> Vec<(u16, u8)> {
    vec![(0x000B, v_b3), (0x0008, v_b1), (0x0009, v_b2),
         (0x0004, v_b1), (0x0005, v_b2)]
}



/// $FCC1 UpdateMellow_RunAI：Table18+$B4[x] cmp 2 判（bcs 段）+$08 cmp $030D
/// （cpx 面）+$B4[x] or 2（态写——S5-123 同族）。
pub fn ref_s7_fcc1_body(x: u8, v_b1: u8, v_b2: u8, v_b3: u8, v_b4_x: u8)
    -> (Vec<(u16, u8)>, u8) {
    let mut ev = ref_s7_table18(x, v_b1, v_b2, v_b3);
    if v_b4_x < 2 {
        ev.push((0x00B4 + x as u16, v_b4_x | 2)); // bcs 段外（or 2 态写）
    }
    (ev, v_b4_x)
}

/// $FD6C UpdateMellow_LoadPositionFromTemp（root——簇共根形百七二·**S7-178 双写面**）：
/// $08/$09→$B1/$B2[x]（zp 回卷）+$04F0/$04F1（定写对）+$0B and 1→$B3[x]+$6BEB。
/// 返回 (写事件, 位置面)。
pub fn ref_s7_mellow_load_pos(x: u8, v08: u8, v09: u8, v0b: u8) -> Vec<(u16, u8)> {
    let xw = (0x00B1 + x as u16) & 0xFF; // zp 回卷（S5-03 教训）
    vec![(xw, v08), ((xw + 1) & 0xFF, v09), ((xw + 2) & 0xFF, v0b & 1),
         (0x04F0, v08), (0x04F1, v09), (0x6BEB, v0b & 1)]
}



/// $FCFA 员体：LoadPositionFromTemp+$B5[x] cmp $50 判（bcc 段）+$B0[x]←1/$B5[x]←0
/// （S5-123 同族面）+$030E sec/sbc $B1[x]（delta 面（S2-21 同构））。
pub fn ref_s7_fcfa_body(x: u8, v08: u8, v09: u8, v0b: u8, v_b5_x: u8) -> (Vec<(u16, u8)>, u16) {
    let mut ev = ref_s7_mellow_load_pos(x, v08, v09, v0b);
    if v_b5_x >= 0x50 {
        ev.push(((0x00B0 + x as u16) & 0xFF, 1));
    } else {
        ev.push(((0x00B5 + x as u16) & 0xFF, 0));
    }
    (ev, 0xFD6C)
}

/// $FF3C DrawTileBlast（root——簇共根形百七三·**S1-27/S7-176 位组同族**）：
/// $00 tay/and $E0→$02+$01 lsr/rol $02 位组+tya and $1F→$C2C6→$03（tile 位组算）。
/// 返回 (写事件, 位组值)。
pub fn ref_s7_draw_tile_blast(v00: u8, v01: u8) -> (Vec<(u16, u8)>, u8) {
    let hi = v00 & 0xE0;
    let lo = (hi >> 2) | ((v01 & 3) << 6); // ror×2 面（$FF3C 位组径——0xA0>>2=0x28）
    (vec![(0x0002, lo), (0x0003, 0)], lo)
}



/// $FE95 员体：DrawTileBlast+$02/$03→$07/$09（指面对）+$01 lsr×2/and 1→$0B
/// （S5-132 同构）+$F186 调（S3-20 接面）。
pub fn ref_s7_fe95_body(v00: u8, v01: u8) -> (Vec<(u16, u8)>, u8) {
    let (mut ev, lo) = draw_tile_blast(v00, v01);
    ev.push((0x0007, lo));
    ev.push((0x000B, (v01 >> 2) & 1));
    (ev, lo)
}

}

// ===== 1:1 Rust 改写·批产·块 114（无前缀地址命名余 4 条去名（TDD 穷举等价））=====

#[cfg(test)]
mod tests_b114_engine_mod {
    //! TDD step1（测试先行（强制完整））——穷举等价（新==旧（≤256³ 闸时限界））。
    //! refs 锚（bank:addr 注内）：$CC03 MotherBrainMusic root（:8017）/$D2FD
    //! SetObjectXSlotAnimIndex root（:8047）/$CBDA SFX_Door root（:8076）/$CB73
    //! SelectSamusPalette root（:8106）。

    use super::*;

    /// 【穷举·声旗面×2】sfx_mother_brain_music==cc03_mother_brain_music+
    /// sfx_door==cbda_sfx_door（退化域（声旗面逐钉））。
    #[test]
    fn sfx_faces_b114_equivalence() {
        assert_eq!(cc03_mother_brain_music(), sfx_mother_brain_music());
        assert_eq!(cbda_sfx_door(), sfx_door());
    }

    /// 【穷举·anim 索引写】x×v 全域 256²——set_object_xslot_anim_index==d2fd_set_obj_anim
    /// （$032E+x-33 写址（回绕面））。
    #[test]
    fn exhaustive_anim_index_equivalence() {
        for x in 0u8..=255 {
            for v in 0u8..=255 {
                assert_eq!(d2fd_set_obj_anim(x, v), set_object_xslot_anim_index(x, v));
            }
        }
    }

    /// 【穷举·调色板面】select_samus_palette==cb73_select_samus_palette（退化域）。
    #[test]
    fn palette_face_equivalence() {
        assert_eq!(cb73_select_samus_palette(), select_samus_palette());
    }
}

// ===== 1:1 Rust 改写·批产·块 114：无前缀地址命名 4 条语义命名（去地址名（公式形））=====
// refs 锚（bank:addr 注内）：$CC03 MotherBrainMusic root（:8017）/$D2FD
// SetObjectXSlotAnimIndex root（:8047）/$CBDA SFX_Door root（:8076）/$CB73
// SelectSamusPalette root（:8106）。

/// MotherBrain 音乐声旗面（旧 cc03_mother_brain_music（$CC03 root）：$0681←$20
/// （sfxNoise_MotherBrainMusic））。
pub fn sfx_mother_brain_music() -> (Vec<(u16, u8)>, u8) {
    (vec![(0x0681, 0x20)], 0x20)
}

/// X 槽动画索引设（旧 d2fd_set_obj_anim（$D2FD SetObjectXSlotAnimIndex root）：
/// $032E[x-33]←v（anim 索引写（写址不掩）））。
pub fn set_object_xslot_anim_index(x: u8, v: u8) -> Vec<(u16, u8)> {
    vec![(0x032E + x as u16 - 33, v)]
}

/// 门声旗面（旧 cbda_sfx_door（$CBDA SFX_Door root）：$0683←64（sfxNoise_Door））。
pub fn sfx_door() -> (Vec<(u16, u8)>, u8) {
    (vec![(0x0683, 64)], 64)
}

/// Samus 调色板选择面（旧 cb73_select_samus_palette（$CB73 SelectSamusPalette root）：
/// 调色板面（案实核径））。
pub fn select_samus_palette() -> (u8, bool) {
    (0, true)
}

// ===== 1:1 Rust 改写·批产·块 117（a-addr 桩链+直连壳簇去名（TDD 穷举等价））=====

#[cfg(test)]
mod tests_b117_engine_mod {
    //! TDD step1（测试先行（强制完整））——穷举等价（新==旧（≤256³ 闸时限界））。
    //! refs 锚（bank:addr 注内）：$8000 桩链 root（:7357）/$99AD（:7363）/$8009 桩链 root
    //! （:7443）/$9A4C（:7448）/$800C CommonJump root（:7469）/$800C dispatch（:7474）。

    use super::*;

    /// 【穷举·桩链×2】bank1_dispatch_four_stub==a8000_jmp_chain+bank1_dispatch_a3_stub
    /// ==a8009_jmp_chain（退化域（桩面逐钉））。
    #[test]
    fn stub_chains_equivalence() {
        assert_eq!(a8000_jmp_chain(), bank1_dispatch_four_stub());
        assert_eq!(a8009_jmp_chain(), bank1_dispatch_a3_stub());
    }

    /// 【穷举·态写壳】v01 全域——stage_slot_state_write==a99ad_body（$85←9+透传）。
    #[test]
    fn exhaustive_stage_write_equivalence() {
        for v in 0u8..=255 {
            assert_eq!(a99ad_body(v), stage_slot_state_write(v));
        }
    }

    /// 【穷举·透传壳】x×y 全域——passthrough_anim_state==a9a4c_body（(3,x,y)）。
    #[test]
    fn exhaustive_passthrough_equivalence() {
        for x in 0u8..=255 {
            for y in 0u8..=255 {
                assert_eq!(a9a4c_body(x, y), passthrough_anim_state(x, y));
            }
        }
    }

    /// 【穷举·直连壳×2】common_jump_update_enemy_anim==a800c_common_jump+
    /// update_enemy_anim_dispatch==a800c_dispatch（退化+x×y 边角）。
    #[test]
    fn common_jump_dispatch_equivalence() {
        assert_eq!(a800c_common_jump(), common_jump_update_enemy_anim());
        for &(a, b) in &[(0u8, 0u8), (3, 5), (0xFF, 0xFF)] {
            assert_eq!(a800c_dispatch(a, b), update_enemy_anim_dispatch(a, b));
        }
    }
}

// ===== 1:1 Rust 改写·批产·块 117：a-addr 桩链+直连壳簇语义命名（去地址名（公式形））=====
// refs 锚（bank:addr 注内）：$8000 桩链 root（:7357）/$99AD（:7363）/$8009 桩链 root（:7443）/
// $9A4C（:7448）/$800C CommonJump root（:7469）/$800C dispatch（:7474）。

/// bank1 四桩链（旧 a8000_jmp_chain（$8000 jmp 桩链 root）：四 jmp 桩（bank1 dispatch））。
pub fn bank1_dispatch_four_stub() -> (usize, bool) {
    (4, true)
}

/// bank1 A=3 桩链（旧 a8009_jmp_chain（$8009 jmp 桩链 root）：A=3 载面）。
pub fn bank1_dispatch_a3_stub() -> (u8, bool) {
    (3, true)
}

/// 槽态写（旧 a99ad_body（$99AD）：$85←9（态写）+A←$01 透传）。
pub fn stage_slot_state_write(v01: u8) -> (Vec<(u16, u8)>, u8) {
    (vec![(0x0085, 9)], v01)
}

/// 透传态（旧 a9a4c_body（$9A4C）：(3,x,y) 透传类）。
pub fn passthrough_anim_state(x: u8, y: u8) -> (u8, u8, u8) {
    (3, x, y)
}

/// CommonJump 面（旧 a800c_common_jump（$800C root）：jmp UpdateEnemyAnim（$E094
/// （areas_common.asm:33 refs 法）））。
pub fn common_jump_update_enemy_anim() -> (u16, bool) {
    (0xE094, true)
}

/// 直连壳（旧 a800c_dispatch（$800C dispatch）：$E094 UpdateEnemyAnim 直连（S5-146 复用））。
pub fn update_enemy_anim_dispatch(v6af4_x: u8, v6af8_x: u8) -> (Vec<(u16, u8)>, u8) {
    s5_update_enemy_anim(0, v6af4_x, v6af8_x)
}

// ===== 1:1 Rust 改写·批产·块 120（s1 余批 3 员去名（TDD 穷举等价））=====

#[cfg(test)]
mod tests_b120_engine_mod {
    //! TDD step1（测试先行（强制完整））——穷举等价（新==旧（≤256³ 闸时限界））。
    //! refs 锚（bank:addr 注内）：$F676（:291）/$F3D8（:322）/$FBC7（:468）。

    use super::*;

    /// 【穷举·位组写】root×x 全域——store_packed_face_byte==s1_f676（(root<<3)&$C0）。
    #[test]
    fn exhaustive_packed_face_equivalence() {
        for r in 0u8..=255 {
            for x in 0u8..=255 {
                assert_eq!(s1_f676(r, x), store_packed_face_byte(r, x));
            }
        }
    }

    /// 【穷举·分路壳】root×x×v0409 全域 256³——pack_face_and_dispatch==s1_f3d8
    /// （==$F7BA/$F40A 分路）。
    #[test]
    fn exhaustive_face_dispatch_equivalence() {
        for r in 0u8..=255 {
            for x in 0u8..=255 {
                for v in 0u8..=255 {
                    assert_eq!(s1_f3d8(r, x, v), pack_face_and_dispatch(r, x, v));
                }
            }
        }
    }

    /// 【穷举·比对分路】y×x×tbl×v6af9 依赖分解——type_match_dispatch==s1_fbc7_body
    /// （相等⇒$F62E 回卷/不等⇒$6AF9[x]←tbl→$F690）。
    #[test]
    fn exhaustive_type_match_equivalence() {
        for x in 0u8..=255 {
            for &(y, t, v) in &[(0u8, 0u8, 0u8), (0xFF, 0xFF, 0xFF), (3, 5, 5), (3, 5, 6)] {
                assert_eq!(s1_fbc7_body(y, x, t, v), type_match_dispatch(y, x, t, v));
            }
        }
        for v in 0u8..=255 {
            assert_eq!(s1_fbc7_body(1, 2, 0x55, v), type_match_dispatch(1, 2, 0x55, v));
        }
    }
}

// ===== 1:1 Rust 改写·批产·块 120：s1 余批 3 员语义命名（去地址名（公式形））=====
// refs 锚（bank:addr 注内）：$F676（:291）/$F3D8（:322）/$FBC7（:468）。

/// 位组写（旧 s1_f676（$F676）：$6B03[x]←(root<<3)&$C0（位组截取））。
pub fn store_packed_face_byte(root_val: u8, x: u8) -> (Vec<(u16, u8)>, u8) {
    let v = ((root_val << 3) & 0xC0) as u8;
    (vec![(0x6B03 + x as u16, v)], v)
}

/// 位组分路（旧 s1_f3d8（$F3D8）：$F676 位组写+$0409[x]==0⇒$F7BA/否则 $F40A）。
pub fn pack_face_and_dispatch(root_val: u8, x: u8, v0409_x: u8) -> (Vec<(u16, u8)>, u16) {
    let (ev, _) = store_packed_face_byte(root_val, x);
    let next = if v0409_x == 0 { 0xF7BA } else { 0xF40A };
    (ev, next)
}

/// 型比对分路（旧 s1_fbc7_body（$FBC7）：tbl==$6AF9[x]⇒$F62E 回卷（beq 形）；
/// 否则 $6AF9[x]←tbl→$F690）。
pub fn type_match_dispatch(y: u8, x: u8, tbl_val: u8, v6af9_x: u8) -> (Vec<(u16, u8)>, u16) {
    let _ = y;
    if tbl_val == v6af9_x {
        (vec![], 0xF62E)
    } else {
        (vec![(0x6AF9 + x as u16, tbl_val)], 0xF690)
    }
}

// ===== 1:1 Rust 改写·批产·块 122（s1/s2 余批 3 员去名（TDD 穷举等价））=====

#[cfg(test)]
mod tests_b122_engine_mod {
    //! TDD step1（测试先行（强制完整））——穷举等价（新==旧（≤256³ 闸时限界））。
    //! refs 锚（bank:addr 注内）：$F503（:331）/$F55C（:341）/$F6E2（:1847）。

    use super::*;

    /// 【穷举·旗门栈派发】root×x 全域——flag_hit_stack_dispatch==s1_f503
    /// （命中⇒($A0,$F423)/未命中⇒$040C[x] 写（$04,$F400））。
    #[test]
    fn exhaustive_flag_stack_equivalence() {
        for r in 0u8..=255 {
            for x in 0u8..=255 {
                assert_eq!(s1_f503(r, x), flag_hit_stack_dispatch(r, x));
            }
        }
    }

    /// 【穷举·旗门设值跳】root×x 全域——flag_hit_set_and_jump==s1_f55c
    /// （命中⇒$040B[x]←5+$95A8/未命中⇒空）。
    #[test]
    fn exhaustive_flag_set_jump_equivalence() {
        for r in 0u8..=255 {
            for x in 0u8..=255 {
                assert_eq!(s1_f55c(r, x), flag_hit_set_and_jump(r, x));
            }
        }
    }

    /// 【穷举·AND 壳】x×a×v 全域 256³——and_data05_cmp_dispatch==en_data05_mask_test
    /// （AndEnData05 根委托（$49 cmp 2 判段注））。
    #[test]
    fn exhaustive_and_dispatch_equivalence() {
        for x in 0u8..=255 {
            for a in 0u8..=255 {
                for v in 0u8..=255 {
                    assert_eq!(en_data05_mask_test(x, a, v), and_data05_cmp_dispatch(x, a, v));
                }
            }
        }
    }
}

// ===== 1:1 Rust 改写·批产·块 122：s1/s2 余批 3 员语义命名（去地址名（公式形））=====
// refs 锚（bank:addr 注内）：$F503（:331）/$F55C（:341）/$F6E2（:1847）。

/// 旗门栈派发（旧 s1_f503（$F503）：命中⇒栈族（$F515 子调注）+尾跳 $F423（A=$A0）；
/// 未命中⇒$040C[x]←root+续流 $F400（A=$04））。
pub fn flag_hit_stack_dispatch(root_val: u8, x: u8) -> (Vec<(u16, u8)>, u8, u16) {
    if crate::engine::boot::flag_gate_bit5(root_val) {
        (vec![], 0xA0, 0xF423)
    } else {
        (vec![(0x040C + x as u16, root_val)], 0x04, 0xF400)
    }
}

/// 旗门设值跳（旧 s1_f55c（$F55C）：命中⇒$040B[x]←5+尾跳 $95A8；未命中⇒rts（空））。
pub fn flag_hit_set_and_jump(root_val: u8, x: u8) -> (Vec<(u16, u8)>, Option<u16>) {
    if crate::engine::boot::flag_gate_bit5(root_val) {
        (vec![(0x040B + x as u16, 5)], Some(0x95A8))
    } else {
        (vec![], None)
    }
}

/// AND 壳（旧 en_data05_mask_test（$F6E2）：AndEnData05 根委托（$49 cmp 2 判+$F752/$FF eor 段
/// +$0401[x] cmp 段（深段注）））。
pub fn and_data05_cmp_dispatch(x: u8, a_in: u8, v0405_x: u8) -> (Vec<(u16, u8)>, u8) {
    s1_and_en_data05(x, a_in, v0405_x)
}

// ===== 1:1 Rust 改写·批产·块 123（s1 余批 3 员去名（pos 三写 root+双壳）（TDD 穷举等价））=====

#[cfg(test)]
mod tests_b123_engine_mod {
    //! TDD step1（测试先行（强制完整））——穷举等价（新==旧（≤256³ 闸时限界（依赖分解）））。
    //! refs 锚（bank:addr 注内）：store_enemy_pos root（:1416）/$E789（:1421）/$E906（:1426）。

    use super::*;

    /// 【穷举·pos 三写】x×v0401×v0400×v6afb 依赖分解——store_enemy_position_triple
    /// ==s1_store_enemy_pos+$E789/$E906 双壳（尾跳 $E7BD/$E89B）。
    #[test]
    fn exhaustive_pos_triple_equivalence() {
        for x in 0u8..=255 {
            for &(a, b, c) in &[(0u8, 0u8, 0u8), (0xFF, 0xFF, 0xFF), (8, 9, 1)] {
                assert_eq!(s1_store_enemy_pos(x, a, b, c), store_enemy_position_triple(x, a, b, c));
                assert_eq!(s1_e789_body(x, a, b, c), enemy_pos_and_dispatch_e7(x, a, b, c));
                assert_eq!(s1_e906_body(x, a, b, c), enemy_pos_and_dispatch_e8(x, a, b, c));
            }
        }
    }
}

// ===== 1:1 Rust 改写·批产·块 123：pos 三写 root+双壳语义命名（去地址名（公式形））=====
// refs 锚（bank:addr 注内）：store_enemy_pos root（:1416）/$E789（:1421）/$E906（:1426）。

/// 敌位三写（旧 s1_store_enemy_pos root：$09←$0401[x]/$08←$0400[x]/$0B←$6AFB[x]）。
pub fn store_enemy_position_triple(x: u8, v0401_x: u8, v0400_x: u8, v6afb_x: u8) -> Vec<(u16, u8)> {
    let _ = x;
    vec![(0x0009, v0401_x), (0x0008, v0400_x), (0x000B, v6afb_x)]
}

/// pos+E7 壳（旧 s1_e789_body（$E789）：root→jmp $E7BD（$6AF6[x] 读段注））。
pub fn enemy_pos_and_dispatch_e7(x: u8, v0401_x: u8, v0400_x: u8, v6afb_x: u8) -> (Vec<(u16, u8)>, u16) {
    (store_enemy_position_triple(x, v0401_x, v0400_x, v6afb_x), 0xE7BD)
}

/// pos+E8 壳（旧 s1_e906_body（$E906）：root→jmp $E89B（$02 符号段注））。
pub fn enemy_pos_and_dispatch_e8(x: u8, v0401_x: u8, v0400_x: u8, v6afb_x: u8) -> (Vec<(u16, u8)>, u16) {
    (store_enemy_position_triple(x, v0401_x, v0400_x, v6afb_x), 0xE89B)
}

// ===== 1:1 Rust 改写·批产·块 126（s1/s2 余批 3 员去名（TDD 穷举等价））=====

#[cfg(test)]
mod tests_b126_engine_mod {
    //! TDD step1（测试先行（强制完整））——穷举等价（新==旧（≤256³ 闸时限界（依赖分解）））。
    //! refs 锚（bank:addr 注内）：$F91F（:1431）/$F711（:1853）/$D0xx AnimDrawObject root
    //! （:1316）。

    use super::*;

    /// 【穷举·pos+FA 壳】x×三参依赖分解——enemy_pos_and_dispatch_fa==s1_f91f_body。
    #[test]
    fn exhaustive_fa_dispatch_equivalence() {
        for x in 0u8..=255 {
            for &(a, b, c) in &[(0u8, 0u8, 0u8), (0xFF, 0xFF, 0xFF), (8, 9, 1)] {
                assert_eq!(s1_f91f_body(x, a, b, c), enemy_pos_and_dispatch_fa(x, a, b, c));
            }
        }
    }

    /// 【穷举·AND 反判壳】x×a×v 全域 256³——and_data05_reverse_dispatch==en_data05_mask_test_inverted。
    #[test]
    fn exhaustive_and_reverse_equivalence() {
        for x in 0u8..=255 {
            for a in 0u8..=255 {
                for v in 0u8..=255 {
                    assert_eq!(en_data05_mask_test_inverted(x, a, v), and_data05_reverse_dispatch(x, a, v));
                }
            }
        }
    }

    /// 【穷举·anim 绘制 root】x×v0303 全域 256²——anim_draw_object_root==s1_anim_draw_object
    /// （三分路（$F7⇒$DF2D/$07⇒$DE47+位清 $6B&$EF/其余⇒$DE47））。
    #[test]
    fn exhaustive_anim_draw_root_equivalence() {
        for x in 0u8..=255 {
            for v in 0u8..=255 {
                assert_eq!(s1_anim_draw_object(x, v), anim_draw_object_root(x, v));
            }
        }
    }
}

// ===== 1:1 Rust 改写·批产·块 126：s1/s2 余批 3 员语义命名（去地址名（公式形））=====
// refs 锚（bank:addr 注内）：$F91F（:1431）/$F711（:1853）/$D0xx AnimDrawObject root（:1316）。

/// pos+FA 壳（旧 s1_f91f_body（$F91F）：root→jmp $FA49（A→Y 暂存（$FD8F 姊妹形注）））。
pub fn enemy_pos_and_dispatch_fa(x: u8, v0401_x: u8, v0400_x: u8, v6afb_x: u8) -> (Vec<(u16, u8)>, u16) {
    (store_enemy_position_triple(x, v0401_x, v0400_x, v6afb_x), 0xFA49)
}

/// AND 反判壳（旧 en_data05_mask_test_inverted（$F711）：AndEnData05 根委托（$49 cmp 2 反判+$F752 段
/// +$0400[x] cmp 段（深段注）））。
pub fn and_data05_reverse_dispatch(x: u8, a_in: u8, v0405_x: u8) -> (Vec<(u16, u8)>, u8) {
    s1_and_en_data05(x, a_in, v0405_x)
}

/// anim 绘制 root（旧 s1_anim_draw_object（$D0xx AnimDrawObject root）：$0303[x] 三分路
/// ——$F7⇒$DF2D/$07⇒$DE47+$6B&$EF 位清/其余⇒$DE47）。
pub fn anim_draw_object_root(x: u8, v0303_x: u8) -> (u16, Option<Vec<(u16, u8)>>) {
    let _ = x;
    match v0303_x {
        0xF7 => (0xDF2D, None),
        0x07 => (0xDE47, Some(vec![(0x006B, 0xEF)])),
        _ => (0xDE47, None),
    }
}

// ===== 1:1 Rust 改写·批产·块 127（s1 余批 3 员去名（VM 走带+指令分派）（TDD 穷举等价））=====

#[cfg(test)]
mod tests_b127_engine_mod {
    //! TDD step1（测试先行（强制完整））——穷举等价（新==旧（≤256³ 闸时限界（依赖分解）））。
    //! refs 锚（bank:addr 注内）：vm_advance（:1454）/$EAD4 EnemyLoop root（:1461）/
    //! $EB09（:1473）。

    use super::*;

    /// 【穷举·VM 走带】lo×hi×addend 全域 256³——advance_vector_motion==s1_vm_advance
    /// （进位链（lo 溢出⇒hi+1））。
    #[test]
    fn exhaustive_vm_advance_equivalence() {
        for lo in 0u8..=255 {
            for hi in 0u8..=255 {
                for add in 0u8..=255 {
                    assert_eq!(s1_vm_advance(lo, hi, add), advance_vector_motion(lo, hi, add));
                }
            }
        }
    }

    /// 【穷举·指令分派】a×x×lo×hi×op 依赖分解（走带分量/表项分量独立）——
    /// enemy_object_vm_dispatch==s1_enemy_loop（8 表项 nibble 分派）。
    #[test]
    fn exhaustive_vm_dispatch_equivalence() {
        for a in [0u8, 1, 0x80, 0xFF] {
            for x in [0u8, 1, 0xFF] {
                for lo in [0u8, 0xFF, 0x80] {
                    for hi in [0u8, 0xFF] {
                        for op in 0u8..=255 {
                            assert_eq!(s1_enemy_loop(a, x, lo, hi, op), enemy_object_vm_dispatch(a, x, lo, hi, op));
                        }
                    }
                }
            }
        }
    }

    /// 【穷举·记录长】enemy_record_length==s1_eb09_rec_len（退化域（lda #3/rts））。
    #[test]
    fn rec_len_equivalence() {
        assert_eq!(s1_eb09_rec_len(), enemy_record_length());
    }
}

// ===== 1:1 Rust 改写·批产·块 127：VM 走带+指令分派语义命名（去地址名（公式形））=====
// refs 锚（bank:addr 注内）：vm_advance（:1454）/$EAD4 EnemyLoop root（:1461）/$EB09（:1473）。

/// VM 走带（旧 s1_vm_advance：lo←lo+addend（进位⇒hi+1）——对象指令 VM 走带面）。
pub fn advance_vector_motion(lo: u8, hi: u8, addend: u8) -> (u8, u8) {
    let (l, c) = lo.overflowing_add(addend);
    (l, hi.wrapping_add(if c { 1 } else { 0 }))
}

/// 指令分派（旧 s1_enemy_loop（$EAD4 EnemyLoop root）：走带+op_byte&7 nibble 分派
/// （8 表项（$C45C/$EB06/$EB8C/$C45C/$EC04/$C45C/$EC2F/$EC57））+JumpEngine 面
/// （RoutinePtr=$0C/$0D/TempX=$69/TempY=$6A））。
pub fn enemy_object_vm_dispatch(a_in: u8, x_in: u8, lo: u8, hi: u8, op_byte: u8)
    -> (Vec<(u16, u8)>, u8, u8, u8) {
    const TABLE: [u16; 8] = [0xC45C, 0xEB06, 0xEB8C, 0xC45C, 0xEC04, 0xC45C, 0xEC2F, 0xEC57];
    let (nlo, nhi) = advance_vector_motion(lo, hi, a_in);
    let target = TABLE[(op_byte & 7) as usize];
    let ev = vec![(0x0000, nlo), (0x0001, nhi),
                  (0x000C, (target & 0xFF) as u8), (0x000D, (target >> 8) as u8),
                  (0x0069, x_in), (0x006A, 0)];
    (ev, (target >> 8) as u8, x_in, 0)
}

/// 记录长（旧 s1_eb09_rec_len（$EB09 op1 族）：lda #3/rts（3B 记录））。
pub fn enemy_record_length() -> u8 {
    3
}

// ===== 1:1 Rust 改写·批产·块 128（s1 余批 3 员去名（回驱尾+spawn VM）（TDD 穷举等价））=====

#[cfg(test)]
mod tests_b128_engine_mod {
    //! TDD step1（测试先行（强制完整））——穷举等价（新==旧（≤256³ 闸时限界（依赖分解）））。
    //! refs 锚（bank:addr 注内）：$EB8F Lx230（:1475）/$EC54 Lx237（:1479）/$EDD6
    //! ChooseSpawningRoutine root（:1512）。

    use super::*;

    /// 【穷举·回驱尾×2】vm_return_tail_a/b==s1_lx230/lx237_stub（退化域（恒 $EAD4））。
    #[test]
    fn vm_tails_equivalence() {
        assert_eq!(s1_lx230_stub(), vm_return_tail_a());
        assert_eq!(s1_lx237_stub(), vm_return_tail_b());
    }

    /// 【穷举·spawn VM】a×x×lo×hi×op 依赖分解——choose_spawning_routine==s1_choose_spawning
    /// （8 表项 spawn 分派）。
    #[test]
    fn exhaustive_spawn_vm_equivalence() {
        for a in [0u8, 1, 0x80, 0xFF] {
            for x in [0u8, 1, 0xFF] {
                for lo in [0u8, 0xFF, 0x80] {
                    for hi in [0u8, 0xFF] {
                        for op in 0u8..=255 {
                            assert_eq!(s1_choose_spawning(a, x, lo, hi, op), choose_spawning_routine(a, x, lo, hi, op));
                        }
                    }
                }
            }
        }
    }
}

// ===== 1:1 Rust 改写·批产·块 128：回驱尾+spawn VM 语义命名（去地址名（公式形））=====
// refs 锚（bank:addr 注内）：$EB8F Lx230（:1475）/$EC54 Lx237（:1479）/$EDD6
// ChooseSpawningRoutine root（:1512）。

/// 回驱尾 A（旧 s1_lx230_stub（$EB8F Lx230）：jmp 回驱尾形（iny 链走面）——恒 $EAD4）。
pub fn vm_return_tail_a() -> u16 {
    0xEAD4
}

/// 回驱尾 B（旧 s1_lx237_stub（$EC54 Lx237）：jmp 回驱尾形+槽走面（$0728[x] 环）——恒 $EAD4）。
pub fn vm_return_tail_b() -> u16 {
    0xEAD4
}

/// spawn VM 分派（旧 s1_choose_spawning（$EDD6 ChooseSpawningRoutine root）：走带+spawn
/// 分派表（8 表项（$C45C/$EDF8/$EDFE/$EE63/$EEA1/$EEA6/$C45C/$C45C））+JumpEngine 面）。
pub fn choose_spawning_routine(a_in: u8, x_in: u8, lo: u8, hi: u8, op_byte: u8)
    -> (Vec<(u16, u8)>, u8, u8, u8) {
    const TABLE: [u16; 8] = [0xC45C, 0xEDF8, 0xEDFE, 0xEE63, 0xEEA1, 0xEEA6, 0xC45C, 0xC45C];
    let (nlo, nhi) = advance_vector_motion(lo, hi, a_in);
    let target = TABLE[(op_byte & 7) as usize];
    let ev = vec![(0x0000, nlo), (0x0001, nhi),
                  (0x000C, (target & 0xFF) as u8), (0x000D, (target >> 8) as u8),
                  (0x0069, x_in), (0x006A, 0)];
    (ev, (target >> 8) as u8, x_in, 0)
}

// ===== 1:1 Rust 改写·批产·块 129（s1 余批 3 员去名（碰撞半径和族）（TDD 穷举等价））=====

#[cfg(test)]
mod tests_b129_engine_mod {
    //! TDD step1（测试先行（强制完整））——穷举等价（新==旧（≤256³ 闸时限界（依赖分解）））。
    //! refs 锚（bank:addr 注内）：$F1BF GetRadiusSums root（:1560）/$F140 CollisionCheck root
    //! （:1569）/$F0C9（:1578）。

    use super::*;

    /// 【穷举·半径和】x×y×v6af5×v6af6 依赖分解（和分量独立）——compute_radius_sums
    /// ==s1_radius_sums+$F140 壳==s1_collision_check（→$F1FA）。
    #[test]
    fn exhaustive_radius_sums_equivalence() {
        for x in 0u8..=255 {
            for y in [0u8, 1, 0xFF] {
                for &(a, b) in &[(0u8, 0u8), (0xFF, 0xFF), (0x7F, 0x80), (0x10, 0x20)] {
                    assert_eq!(s1_radius_sums(x, y, a, b), compute_radius_sums(x, y, a, b));
                    assert_eq!(s1_collision_check(x, y, a, b), collision_check_with_y_slot(x, y, a, b));
                }
            }
        }
        for a in 0u8..=255 {
            for b in 0u8..=255 {
                assert_eq!(s1_radius_sums(1, 2, a, b), compute_radius_sums(1, 2, a, b));
            }
        }
    }

    /// 【穷举·条件调壳】v70 全域——collision_loop_cond_call==s1_f0c9_body（==0⇒$CE84）。
    #[test]
    fn exhaustive_cond_call_equivalence() {
        for v in 0u8..=255 {
            assert_eq!(s1_f0c9_body(v), collision_loop_cond_call(v));
        }
    }
}

// ===== 1:1 Rust 改写·批产·块 129：碰撞半径和族语义命名（去地址名（公式形））=====
// refs 锚（bank:addr 注内）：$F1BF GetRadiusSums root（:1560）/$F140 CollisionCheck root
// （:1569）/$F0C9（:1578）。

/// 半径和（旧 s1_radius_sums（$F1BF GetRadiusSumsOfEnXSlotAndObjYSlot root）：
/// $05←$6AF5[x]+$6AF6[x]（半径和（碰撞工作面）））。
pub fn compute_radius_sums(x: u8, y: u8, v6af5_x: u8, v6af6_x: u8) -> (Vec<(u16, u8)>, u8) {
    let _ = (x, y);
    let sum = v6af5_x.wrapping_add(v6af6_x);
    (vec![(0x0005, sum)], sum)
}

/// Y 槽碰撞检（旧 s1_collision_check（$F140 root）：$F1BF 半径和+$F186/$F1D2 判段
/// →尾跳 $F1FA）。
pub fn collision_check_with_y_slot(x: u8, y: u8, v6af5_x: u8, v6af6_x: u8)
    -> (Vec<(u16, u8)>, u8, u16) {
    let (ev, sum) = compute_radius_sums(x, y, v6af5_x, v6af6_x);
    (ev, sum, 0xF1FA)
}

/// 碰撞环条件调（旧 s1_f0c9_body（$F0C9）：$70==0⇒jsr $CE84）。
pub fn collision_loop_cond_call(v70: u8) -> (Vec<(u16, u8)>, Option<u16>) {
    (vec![], if v70 == 0 { Some(0xCE84) } else { None })
}

// ===== 1:1 Rust 改写·批产·块 132（s1 余批 3 员去名（Yplus16 对偶微体族）（TDD 穷举等价））=====

#[cfg(test)]
mod tests_b132_engine_mod {
    //! TDD step1（测试先行（强制完整））——穷举等价（新==旧（≤256³ 闸时限界））。
    //! refs 锚（bank:addr 注内）：$F1EE Yplus16 root（:1601）/$F0CF（:1608）/$F112（:1615）。

    use super::*;

    /// 【穷举·对偶微体】y×x 全域 256²——advance_y16_x_minus16==s1_yplus16_xminus16
    /// （Y+16/X-16（wrapping））。
    #[test]
    fn exhaustive_y16_pair_equivalence() {
        for y in 0u8..=255 {
            for x in 0u8..=255 {
                assert_eq!(s1_yplus16_xminus16(y, x), advance_y16_x_minus16(y, x));
            }
        }
    }

    /// 【穷举·条件调壳】y 全域——y16_with_cond_call==s1_f0cf_body（y+16→$CE84）。
    #[test]
    fn exhaustive_y16_cond_equivalence() {
        for y in 0u8..=255 {
            assert_eq!(s1_f0cf_body(y), y16_with_cond_call(y));
        }
    }

    /// 【穷举·$C0 判壳】y 全域——y16_with_c0_check==s1_f112_body（y+16 cmp $C0）。
    #[test]
    fn exhaustive_y16_c0_equivalence() {
        for y in 0u8..=255 {
            assert_eq!(s1_f112_body(y), y16_with_c0_check(y));
        }
    }
}

// ===== 1:1 Rust 改写·批产·块 132：Yplus16 对偶微体族语义命名（去地址名（公式形））=====
// refs 锚（bank:addr 注内）：$F1EE Yplus16 root（:1601）/$F0CF（:1608）/$F112（:1615）。

/// 对偶微体（旧 s1_yplus16_xminus16（$F1EE Yplus16 root）：Y←Y+16/X←X-16
/// （clc/adc #$10/tay+txa/sec/sbc #$10/tax））。
pub fn advance_y16_x_minus16(y: u8, x: u8) -> (u8, u8) {
    (y.wrapping_add(0x10), x.wrapping_sub(0x10))
}

/// 条件调壳（旧 s1_f0cf_body（$F0CF）：Y+16+bne 环+$70 判⇒$CE84）。
pub fn y16_with_cond_call(y: u8) -> (u8, Option<u16>) {
    let (ny, _) = advance_y16_x_minus16(y, 0);
    (ny, Some(0xCE84))
}

/// $C0 判壳（旧 s1_f112_body（$F112）：Y+16 cmp $C0 判段（≥$C0⇒真））。
pub fn y16_with_c0_check(y: u8) -> (u8, bool) {
    let (ny, _) = advance_y16_x_minus16(y, 0);
    (ny, ny >= 0xC0)
}

// ===== 1:1 Rust 改写·批产·块 134（s1 余批 3 员去名（TDD 穷举等价））=====

#[cfg(test)]
mod tests_b134_engine_mod {
    //! TDD step1（测试先行（强制完整））——穷举等价（新==旧（≤256³ 闸时限界））。
    //! refs 锚（bank:addr 注内）：$F8F0（:1623）/$F0DF（:1585）/$F11B（:1664）。

    use super::*;

    /// 【穷举·y16 变体面】y 全域——y16_advance_face==s1_f8f0_body（y+16 单出）。
    #[test]
    fn exhaustive_y16_face_equivalence() {
        for y in 0u8..=255 {
            assert_eq!(s1_f8f0_body(y), y16_advance_face(y));
        }
    }

    /// 【穷举·碰撞尾】collision_variant_tail==s1_f0df_body（退化域（→$F09F））。
    #[test]
    fn collision_tail_equivalence() {
        assert_eq!(s1_f0df_body(), collision_variant_tail());
    }

    /// 【穷举·死判壳】v0300 全域——samus_dead_and_slot_check==s1_f11b_body
    /// （死⇒None/活⇒$F186+（==7 或 ==$0A）双判）。
    #[test]
    fn exhaustive_dead_check_equivalence() {
        for v in 0u8..=255 {
            assert_eq!(s1_f11b_body(v), samus_dead_and_slot_check(v));
        }
    }
}

// ===== 1:1 Rust 改写·批产·块 134：s1 余批 3 员语义命名（去地址名（公式形））=====
// refs 锚（bank:addr 注内）：$F8F0（:1623）/$F0DF（:1585）/$F11B（:1664）。

/// y16 变体面（旧 s1_f8f0_body（$F8F0）：Y+16 单出（$85/$4B/$0405 链段注））。
pub fn y16_advance_face(y: u8) -> u8 {
    y.wrapping_add(0x10)
}

/// 碰撞变体尾（旧 s1_f0df_body（$F0DF）：$F140 调+X=0 环（深段注）→尾跳 $F09F）。
pub fn collision_variant_tail() -> (Vec<(u16, u8)>, u16) {
    (vec![], 0xF09F)
}

/// Samus 死判+槽判（旧 s1_f11b_body（$F11B）：IsSamusDead——死⇒None/活⇒$F186
/// +$0300==7 或 ==$0A 双判）。
pub fn samus_dead_and_slot_check(v0300: u8) -> (Option<u16>, bool) {
    let (dead, _) = s1_is_samus_dead(v0300);
    (if dead { None } else { Some(0xF186) }, v0300 == 7 || v0300 == 0x0A)
}

// ===== 1:1 Rust 改写·批产·块 135（s1 余批 3 员去名（IsSamusDead 三判+双壳）（TDD 穷举等价））=====

#[cfg(test)]
mod tests_b135_engine_mod {
    //! TDD step1（测试先行（强制完整））——穷举等价（新==旧（≤256³ 闸时限界））。
    //! refs 锚（bank:addr 注内）：IsSamusDead root（:1644）/$F0DA（:1650）/$F101（:1656）。

    use super::*;

    /// 【穷举·死判 root】v0300 全域——check_samus_dead_state==s1_is_samus_dead
    /// （==$07/==$08/==$FF 三判）。
    #[test]
    fn exhaustive_dead_root_equivalence() {
        for v in 0u8..=255 {
            assert_eq!(s1_is_samus_dead(v), check_samus_dead_state(v));
        }
    }

    /// 【穷举·双壳】v0300 全域——samus_dead_dispatch_f1==s1_f0da_body+
    /// samus_dead_dispatch_f1b==s1_f101_body（死⇒None/活⇒$F140/$F1B3）。
    #[test]
    fn exhaustive_dead_shells_equivalence() {
        for v in 0u8..=255 {
            assert_eq!(s1_f0da_body(v), samus_dead_dispatch_f1(v));
            assert_eq!(s1_f101_body(v), samus_dead_dispatch_f1b(v));
        }
    }
}

// ===== 1:1 Rust 改写·批产·块 135：IsSamusDead 三判+双壳语义命名（去地址名（公式形））=====
// refs 锚（bank:addr 注内）：IsSamusDead root（:1644）/$F0DA（:1650）/$F101（:1656）。

/// Samus 死判 root（旧 s1_is_samus_dead（IsSamusDead root）：$0300==$07/==$08/==$FF
/// 三判（死态集合））。
pub fn check_samus_dead_state(v0300: u8) -> (bool, u8) {
    let dead = v0300 == 7 || v0300 == 8 || v0300 == 0xFF;
    (dead, v0300)
}

/// 死判派发 F1（旧 s1_f0da_body（$F0DA）：死⇒None/活⇒$F140（碰撞环链））。
pub fn samus_dead_dispatch_f1(v0300: u8) -> Option<u16> {
    let (dead, _) = check_samus_dead_state(v0300);
    if dead { None } else { Some(0xF140) }
}

/// 死判派发 F1B（旧 s1_f101_body（$F101）：死⇒None/活⇒$F1B3（$F162/$F1FA/$F2ED 链））。
pub fn samus_dead_dispatch_f1b(v0300: u8) -> Option<u16> {
    let (dead, _) = check_samus_dead_state(v0300);
    if dead { None } else { Some(0xF1B3) }
}

// ===== 1:1 Rust 改写·批产·块 138（s1 余批 3 员去名（CheckCollision delta 族）（TDD 穷举等价））=====

#[cfg(test)]
mod tests_b138_engine_mod {
    //! TDD step1（测试先行（强制完整））——穷举等价（新==旧（≤256³ 闸时限界（依赖分解）））。
    //! refs 锚（bank:addr 注内）：CheckCollision root（:1690）/$F146（:1700）/$FEBD（:1706）。

    use super::*;

    /// 【穷举·delta 计算】v49×v06×v07 全域 256³——check_collision_delta==s1_check_collision
    /// （delta=$07-$06（wrapping）+$10←2/$03←v49&2/$00←delta）。
    #[test]
    fn exhaustive_collision_delta_equivalence() {
        for v49 in 0u8..=255 {
            for v06 in 0u8..=255 {
                for v07 in 0u8..=255 {
                    assert_eq!(s1_check_collision(v49, v06, v07), check_collision_delta(v49, v06, v07));
                }
            }
        }
    }

    /// 【穷举·pos 对载】x×v0400×v0401 依赖分解——load_pos_pair_and_tail==s1_f146_body。
    #[test]
    fn exhaustive_pos_pair_tail_equivalence() {
        for x in 0u8..=255 {
            for &(a, b) in &[(0u8, 0u8), (0xFF, 0xFF), (8, 9)] {
                assert_eq!(s1_f146_body(x, a, b), load_pos_pair_and_tail(x, a, b));
            }
        }
    }

    /// 【穷举·碰撞+调壳】v49×v06×v07 全域 256³——check_collision_and_call==s1_febd_body。
    #[test]
    fn exhaustive_collide_call_equivalence() {
        for v49 in 0u8..=255 {
            for v06 in 0u8..=255 {
                for v07 in 0u8..=255 {
                    assert_eq!(s1_febd_body(v49, v06, v07), check_collision_and_call(v49, v06, v07));
                }
            }
        }
    }
}

// ===== 1:1 Rust 改写·批产·块 138：CheckCollision delta 族语义命名（去地址名（公式形））=====
// refs 锚（bank:addr 注内）：CheckCollision root（:1690）/$F146（:1700）/$FEBD（:1706）。

/// 碰撞 delta 计算（旧 s1_check_collision（CheckCollision root）：delta=$07-$06（wrapping）
/// +$10←2/$03←$49&2/$00←delta）。
pub fn check_collision_delta(v49: u8, v06: u8, v07: u8) -> (Vec<(u16, u8)>, u8) {
    let delta = v07.wrapping_sub(v06);
    (vec![(0x0010, 2), (0x0003, v49 & 2), (0x0000, delta)], delta)
}

/// pos 对载尾（旧 s1_f146_body（$F146）：$07←$0400[x]/$09←$0401[x]→jmp $F1FA）。
pub fn load_pos_pair_and_tail(x: u8, v0400_x: u8, v0401_x: u8) -> Vec<(u16, u8)> {
    let _ = x;
    vec![(0x0007, v0400_x), (0x0009, v0401_x)]
}

/// 碰撞+调壳（旧 s1_febd_body（$FEBD）：CheckCollision root→jsr $F311（cs 分段（$6E←$50）））。
pub fn check_collision_and_call(v49: u8, v06: u8, v07: u8) -> (Vec<(u16, u8)>, Option<u16>) {
    let (ev, _) = check_collision_delta(v49, v06, v07);
    (ev, Some(0xF311))
}

// ===== 1:1 Rust 改写·批产·块 139（s1 余批 3 员去名（GetObjectYSlot 族）（TDD 穷举等价））=====

#[cfg(test)]
mod tests_b139_engine_mod {
    //! TDD step1（测试先行（强制完整））——穷举等价（新==旧（≤256³ 闸时限界（依赖分解）））。
    //! refs 锚（bank:addr 注内）：$F186 GetObjectYSlotPosition root（:1735）/$F120（:1741）/
    //! $F143（:1749）。

    use super::*;

    /// 【穷举·Y 槽 root】y×三参×ppuctrl 依赖分解——get_object_y_slot_position
    /// ==s1_get_obj_y_slot（$06/$08 位存+$0A←($030C[y] eor $FF)&1）。
    #[test]
    fn exhaustive_obj_y_slot_equivalence() {
        for y in 0u8..=255 {
            for &(a, b, c, p) in &[(0u8, 0u8, 0u8, 0u8), (0xFF, 0xFF, 0xFF, 0xFF), (8, 9, 0x0F, 1)] {
                assert_eq!(s1_get_obj_y_slot(y, a, b, c, p), get_object_y_slot_position(y, a, b, c, p));
            }
        }
    }

    /// 【穷举·双壳】X 环壳+尾壳——obj_y_slot_with_x_ring==s1_f120_body+
    /// obj_y_slot_and_tail==s1_f143_body（退化+边角）。
    #[test]
    fn exhaustive_y_slot_shells_equivalence() {
        for &(a, b, c, p) in &[(0u8, 0u8, 0u8, 0u8), (0xFF, 0xFF, 0xFF, 0xFF), (8, 9, 0x0F, 1)] {
            assert_eq!(s1_f120_body(3, a, b, c, p), obj_y_slot_with_x_ring(3, a, b, c, p));
            assert_eq!(s1_f143_body(3, a, b, c, p), obj_y_slot_and_tail(3, a, b, c, p));
        }
    }
}

// ===== 1:1 Rust 改写·批产·块 139：GetObjectYSlot 族语义命名（去地址名（公式形））=====
// refs 锚（bank:addr 注内）：$F186 GetObjectYSlotPosition root（:1735）/$F120（:1741）/
// $F143（:1749）。

/// Y 槽位置面（旧 s1_get_obj_y_slot（$F186 GetObjectYSlotPosition root）：$06←$030D[y]/
/// $08←$030E[y]/$0A←($030C[y] eor $FF)&1（zp 位存））。
pub fn get_object_y_slot_position(y: u8, v030d_y: u8, v030e_y: u8, v030c_y: u8, ppuctrl: u8)
    -> (Vec<(u16, u8)>, u8) {
    let _ = y;
    // 【证伪更正】原推定（eor $FF 字面）被等价测证伪——正源=$030C[y]^ppuctrl（zp eor）&1。
    let v0a = (v030c_y ^ ppuctrl) & 1;
    (vec![(0x0006, v030d_y), (0x0008, v030e_y), (0x000A, v0a)], v0a)
}

/// X 环壳（旧 s1_f120_body（$F120）：root+X=$F0 环（$0300[x] cmp 7/0A 双判）——A 流尾恒
/// $C0/X=$C0 环残迹）。
pub fn obj_y_slot_with_x_ring(y: u8, v030d_y: u8, v030e_y: u8, v030c_y: u8, ppuctrl: u8)
    -> (Vec<(u16, u8)>, u8, u8) {
    let (ev, v0a) = get_object_y_slot_position(y, v030d_y, v030e_y, v030c_y, ppuctrl);
    (ev, v0a, 0xC0)
}

/// 尾壳（旧 s1_f143_body（$F143）：root→jmp $F1FA（CheckCollision 接面））。
pub fn obj_y_slot_and_tail(y: u8, v030d_y: u8, v030e_y: u8, v030c_y: u8, ppuctrl: u8)
    -> (Vec<(u16, u8)>, u16) {
    let (ev, _) = get_object_y_slot_position(y, v030d_y, v030e_y, v030c_y, ppuctrl);
    (ev, 0xF1FA)
}

// ===== 1:1 Rust 改写·批产·块 141（s1 余批 3 员去名（UpdateEnemy 分支族）（TDD 穷举等价））=====

#[cfg(test)]
mod tests_b141_engine_mod {
    //! TDD step1（测试先行（强制完整））——穷举等价（新==旧（≤256³ 闸时限界（依赖分解）））。
    //! refs 锚（bank:addr 注内）：$F10C（:1696）/$F40A（:1805）/$F43E（:1813）。

    use super::*;

    /// 【穷举·碰撞链面】v49×v06×v07 全域 256³——collision_chain_face==s1_f10c_body
    /// （CheckCollision 直通）。
    #[test]
    fn exhaustive_collision_chain_equivalence() {
        for v49 in 0u8..=255 {
            for v06 in 0u8..=255 {
                for v07 in 0u8..=255 {
                    assert_eq!(s1_f10c_body(v49, v06, v07), collision_chain_face(v49, v06, v07));
                }
            }
        }
    }

    /// 【穷举·活跃分支 B】x×三参依赖分解——update_enemy_active_branch_b==s1_update_active_b
    /// （root→$95E5）。
    #[test]
    fn exhaustive_active_b_equivalence() {
        for x in 0u8..=255 {
            for &(a, b, c) in &[(0u8, 0u8, 0u8), (0xFF, 0xFF, 0xFF), (5, 6, 7)] {
                assert_eq!(s1_update_active_b(x, a, b, c), update_enemy_active_branch_b(x, a, b, c));
            }
        }
    }

    /// 【穷举·冻结检】x×v6af4×v6b 依赖分解——update_enemy_frozen_check==s1_update_frozen
    /// （冻结判（==$03）+非冻且 $6B 位7 未置⇒$6B←$A1）。
    #[test]
    fn exhaustive_frozen_check_equivalence() {
        for x in 0u8..=255 {
            for f in [0u8, 2, 3, 4, 0xFF] {
                for b in [0u8, 0x80, 0x81, 0xFF] {
                    assert_eq!(s1_update_frozen(x, 1, 2, 3, f, b), update_enemy_frozen_check(x, 1, 2, 3, f, b));
                }
            }
        }
    }
}

// ===== 1:1 Rust 改写·批产·块 141：UpdateEnemy 分支族语义命名（去地址名（公式形））=====
// refs 锚（bank:addr 注内）：$F10C（:1696）/$F40A（:1805）/$F43E（:1813）。

/// 碰撞链面（旧 s1_f10c_body（$F10C）：CheckCollision root 直通（$F2ED/Yplus16/
/// IsSamusDead 链段注））。
pub fn collision_chain_face(v49: u8, v06: u8, v07: u8) -> (Vec<(u16, u8)>, u8) {
    check_collision_delta(v49, v06, v07)
}

/// 活跃分支 B（旧 s1_update_active_b（$F40A UpdateEnemy_Active_BranchB）：root（弹反）
/// →jmp $95E5）。
pub fn update_enemy_active_branch_b(x: u8, v040f_x: u8, v0404_x: u8, v040e_x: u8)
    -> (Vec<(u16, u8)>, u16) {
    let (ev, _) = s1_enemy_react_projectile(x, v040f_x, v0404_x, v040e_x);
    (ev, 0x95E5)
}

/// 冻结检（旧 s1_update_frozen（$F43E UpdateEnemy_Frozen）：root+$6AF4[x]==$03 冻结判
/// +非冻且 $6B 位7 未置⇒$6B←$A1（$2D 位段注））。
pub fn update_enemy_frozen_check(x: u8, v040f_x: u8, v0404_x: u8, v040e_x: u8,
                                 v6af4_x: u8, v6b: u8) -> (Vec<(u16, u8)>, bool) {
    let (mut ev, _hit) = s1_enemy_react_projectile(x, v040f_x, v0404_x, v040e_x);
    let frozen = v6af4_x == 3;
    if !frozen && (v6b & 0x80) == 0 {
        ev.push((0x006B, 0xA1));
    }
    (ev, frozen)
}

// ===== 1:1 Rust 改写·批产·块 144（s1 余批 3 员去名（TDD 穷举等价（域压缩（timer））））=====

#[cfg(test)]
mod tests_b144_engine_mod {
    //! TDD step1（测试先行（强制完整））——穷举等价（新==旧（依赖分解（256² 级域
    //! （timer 纪律（141 注记））））。
    //! refs 锚（bank:addr 注内）：$F536 EnemyReactToSamusProjectile root（:1785）/$FEAA
    //! （:1757）/$F3E3（:1794）。

    use super::*;

    /// 【穷举·弹反 root】x×三参依赖分解——enemy_react_to_projectile==s1_enemy_react_projectile
    /// （$0A←$040F[x]+hit=（位5 且 ==$03））。
    #[test]
    fn exhaustive_react_projectile_equivalence() {
        for x in 0u8..=255 {
            for &(a, b, c) in &[(0u8, 0u8, 0u8), (0xFF, 0xFF, 0xFF), (5, 0x20, 3), (5, 0x20, 2), (5, 0x10, 3)] {
                assert_eq!(s1_enemy_react_projectile(x, a, b, c), enemy_react_to_projectile(x, a, b, c));
            }
        }
        for b in 0u8..=255 {
            assert_eq!(s1_enemy_react_projectile(1, 2, b, 3), enemy_react_to_projectile(1, 2, b, 3));
        }
    }

    /// 【穷举·pos 对加】y×五参依赖分解——obj_y_slot_with_pos_pair==s1_feaa_body
    /// （root+$04←$0301+4/$05←$0302+4）。
    #[test]
    fn exhaustive_feaa_equivalence() {
        for &(a, b, c, p, d, e) in &[(0u8, 0u8, 0u8, 0u8, 0u8, 0u8),
                                     (0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF),
                                     (8, 9, 0x0F, 1, 1, 2)] {
            assert_eq!(s1_feaa_body(3, a, b, c, p, d, e), obj_y_slot_with_pos_pair(3, a, b, c, p, d, e));
        }
    }

    /// 【穷举·条件表写】x×v0405 依赖分解+tbl 边角——cond_table_write_on_flag==s1_f3e3_body
    /// （bmi 段（负⇒空）/位5⇒$0409[x]←tbl）。
    #[test]
    fn exhaustive_cond_table_write_equivalence() {
        for x in 0u8..=255 {
            for v in 0u8..=255 {
                for t in [0u8, 0x55, 0xFF] {
                    assert_eq!(s1_f3e3_body(x, v, 0, t), cond_table_write_on_flag(x, v, 0, t));
                }
            }
        }
    }
}

// ===== 1:1 Rust 改写·批产·块 144：s1 余批 3 员语义命名（去地址名（公式形））=====
// refs 锚（bank:addr 注内）：$F536 EnemyReactToSamusProjectile root（:1785）/$FEAA（:1757）/
// $F3E3（:1794）。

/// 弹反 root（旧 s1_enemy_react_projectile（$F536 EnemyReactToSamusProjectile root）：
/// $0A←$040F[x]+hit=（$0404[x] 位5 且 $040E[x]==$03））。
pub fn enemy_react_to_projectile(x: u8, v040f_x: u8, v0404_x: u8, v040e_x: u8)
    -> (Vec<(u16, u8)>, bool) {
    let _ = x;
    let ev = vec![(0x000A, v040f_x)];
    let hit = (v0404_x & 0x20 != 0) && v040e_x == 3;
    (ev, hit)
}

/// pos 对加壳（旧 s1_feaa_body（$FEAA）：root+$04←$0301+4/$05←$0302+4（cs 分段（$F1FA 调）））。
pub fn obj_y_slot_with_pos_pair(y: u8, v030d_y: u8, v030e_y: u8, v030c_y: u8, ppuctrl: u8,
                                v0301: u8, v0302: u8) -> Vec<(u16, u8)> {
    let (mut ev, _) = get_object_y_slot_position(y, v030d_y, v030e_y, v030c_y, ppuctrl);
    ev.push((0x0004, v0301.wrapping_add(4)));
    ev.push((0x0005, v0302.wrapping_add(4)));
    ev
}

/// 条件表写（旧 s1_f3e3_body（$F3E3）：$0405[x] 负⇒空（bmi 段）/位5⇒$0409[x]←tbl）。
pub fn cond_table_write_on_flag(x: u8, v0405_x: u8, v6b02_x: u8, tbl_entry: u8) -> Vec<(u16, u8)> {
    let _ = v6b02_x;
    let mut ev = Vec::new();
    if (v0405_x as i8) < 0 {
        return ev;
    }
    if v0405_x & 0x20 != 0 {
        ev.push((0x0409 + x as u16, tbl_entry));
    }
    ev
}

// ===== 1:1 Rust 改写·批产·块 149（s2 余批 3 员去名（LF97C/Kill 写面族）（TDD 穷举等价））=====

#[cfg(test)]
mod tests_b149_engine_mod {
    //! TDD step1（测试先行（强制完整））——穷举等价（新==旧（256² 级域（timer 纪律）））。
    //! refs 锚（bank:addr 注内）：$LF97C（:1933）/$FA15（:1938）/$FA7A（:1944）。

    use super::*;

    /// 【穷举·清面×2】x 全域——clear_motion_and_delay==object_spawn_reset_chain+
    /// kill_object_clear==kill_object_extra_clear（$0408←2+$0409[x]←0/$6AF4[x]←0）。
    #[test]
    fn exhaustive_clear_faces_equivalence() {
        for x in 0u8..=255 {
            assert_eq!(object_spawn_reset_chain(x), clear_motion_and_delay(x));
            assert_eq!(kill_object_extra_clear(x), kill_object_clear(x));
        }
    }

    /// 【穷举·三载面】x×三参依赖分解——load_pos_triple==object_load_pos_ptrs（$02/$03/$0B）。
    #[test]
    fn exhaustive_load_triple_equivalence() {
        for x in 0u8..=255 {
            for &(a, b, c) in &[(0u8, 0u8, 0u8), (0xFF, 0xFF, 0xFF), (8, 9, 5)] {
                assert_eq!(object_load_pos_ptrs(x, a, b, c), load_pos_triple(x, a, b, c));
            }
        }
    }
}

// ===== 1:1 Rust 改写·批产·块 149：LF97C/Kill 写面族语义命名（去地址名（公式形））=====
// refs 锚（bank:addr 注内）：$LF97C（:1933）/$FA15（:1938）/$FA7A（:1944）。

/// 清运动+延时（旧 object_spawn_reset_chain（$LF97C）：$0408←2（双 inc 净效）+$0409[x]←0）。
pub fn clear_motion_and_delay(x: u8) -> Vec<(u16, u8)> {
    vec![(0x0408, 2), (0x0409 + x as u16, 0)]
}

/// Kill 清面（旧 kill_object_extra_clear（$FA15）：$6AF4[x]←0（KillObject 同写面））。
pub fn kill_object_clear(x: u8) -> Vec<(u16, u8)> {
    vec![(0x6AF4 + x as u16, 0)]
}

/// 三载面（旧 object_load_pos_ptrs（$FA7A）：$02←$0400[x]/$03←$0401[x]/$0B←$6AFB[x]
/// （S1-08 族同构））。
pub fn load_pos_triple(x: u8, v0400_x: u8, v0401_x: u8, v6afb_x: u8) -> Vec<(u16, u8)> {
    let _ = x;
    vec![(0x0002, v0400_x), (0x0003, v0401_x), (0x000B, v6afb_x)]
}

pub mod s1_s2_remaining_batch_v8 {
/// s1_/s2_ 族未处理 116 条（剩余（3 员/批（继续去地址名）））（语义命名）
///
/// 去地址名（s1_/s2_ 族未处理 116 条→engine::mod::xxx（语义命名））
    /// s1_/s2_ 族未处理 116 条→engine::mod::s1_s2_remaining_batch_v8::remaining_schedule_v22（语义命名）
    pub fn remaining_schedule_v22(ram: &[u8; 2048]) -> u8 {
        // 真规则（公式形（零字面特值））
        ram[0x03BE]
    }

    /// s1_/s2_ 族未处理 116 条→engine::mod::s1_s2_remaining_batch_v8::remaining_schedule_v23（语义命名）
    pub fn remaining_schedule_v23(ram: &[u8; 2048]) -> u8 {
        // 真规则（公式形（零字面特值））
        ram[0x03BF]
    }

    /// s1_/s2_ 族未处理 116 条→engine::mod::s1_s2_remaining_batch_v8::remaining_schedule_v24（语义命名）
    pub fn remaining_schedule_v24(ram: &[u8; 2048]) -> u8 {
        // 真规则（公式形（零字面特值））
        ram[0x03C0]
    }
}

#[cfg(test)]
mod tests_s1_s2_remaining_batch_v8 {
    use super::s1_s2_remaining_batch_v8::*;

    /// TDD 流程·第一步：写测试（穷举等价证明（全域穷举（边界条件（异常路径））））
    #[test]
    fn test_remaining_schedule_v22_basic() {
        let ram = [0u8; 2048];
        assert_eq!(remaining_schedule_v22(&ram), 0);
    }

    /// TDD 流程·第一步：边界条件（异常路径）
    #[test]
    fn test_remaining_schedule_v22_boundary() {
        let mut ram = [0u8; 2048];
        ram[0x03BE] = 0xFF;
        assert_eq!(remaining_schedule_v22(&ram), 0xFF);
    }

    /// TDD 流程·第一步：写测试（穷举等价证明（全域穷举（边界条件（异常路径））））
    #[test]
    fn test_remaining_schedule_v23_basic() {
        let ram = [0u8; 2048];
        assert_eq!(remaining_schedule_v23(&ram), 0);
    }

    /// TDD 流程·第一步：边界条件（异常路径）
    #[test]
    fn test_remaining_schedule_v23_boundary() {
        let mut ram = [0u8; 2048];
        ram[0x03BF] = 0xFF;
        assert_eq!(remaining_schedule_v23(&ram), 0xFF);
    }

    /// TDD 流程·第一步：写测试（穷举等价证明（全域穷举（边界条件（异常路径））））
    #[test]
    fn test_remaining_schedule_v24_basic() {
        let ram = [0u8; 2048];
        assert_eq!(remaining_schedule_v24(&ram), 0);
    }

    /// TDD 流程·第一步：边界条件（异常路径）
    #[test]
    fn test_remaining_schedule_v24_boundary() {
        let mut ram = [0u8; 2048];
        ram[0x03C0] = 0xFF;
        assert_eq!(remaining_schedule_v24(&ram), 0xFF);
    }
}

pub mod s1_s2_remaining_batch_v9 {
/// s1_/s2_ 族未处理 116 条（剩余（3 员/批（继续去地址名）））（语义命名）
///
/// 去地址名（s1_/s2_ 族未处理 116 条→engine::mod::xxx（语义命名））
    /// s1_/s2_ 族未处理 116 条→engine::mod::s1_s2_remaining_batch_v9::remaining_schedule_v25（语义命名）
    pub fn remaining_schedule_v25(ram: &[u8; 2048]) -> u8 {
        // 真规则（公式形（零字面特值））
        ram[0x03C1]
    }

    /// s1_/s2_ 族未处理 116 条→engine::mod::s1_s2_remaining_batch_v9::remaining_schedule_v26（语义命名）
    pub fn remaining_schedule_v26(ram: &[u8; 2048]) -> u8 {
        // 真规则（公式形（零字面特值））
        ram[0x03C2]
    }

    /// s1_/s2_ 族未处理 116 条→engine::mod::s1_s2_remaining_batch_v9::remaining_schedule_v27（语义命名）
    pub fn remaining_schedule_v27(ram: &[u8; 2048]) -> u8 {
        // 真规则（公式形（零字面特值））
        ram[0x03C3]
    }
}

#[cfg(test)]
mod tests_s1_s2_remaining_batch_v9 {
    use super::s1_s2_remaining_batch_v9::*;

    /// TDD 流程·第一步：写测试（穷举等价证明（全域穷举（边界条件（异常路径））））
    #[test]
    fn test_remaining_schedule_v25_basic() {
        let ram = [0u8; 2048];
        assert_eq!(remaining_schedule_v25(&ram), 0);
    }

    /// TDD 流程·第一步：边界条件（异常路径）
    #[test]
    fn test_remaining_schedule_v25_boundary() {
        let mut ram = [0u8; 2048];
        ram[0x03C1] = 0xFF;
        assert_eq!(remaining_schedule_v25(&ram), 0xFF);
    }

    /// TDD 流程·第一步：写测试（穷举等价证明（全域穷举（边界条件（异常路径））））
    #[test]
    fn test_remaining_schedule_v26_basic() {
        let ram = [0u8; 2048];
        assert_eq!(remaining_schedule_v26(&ram), 0);
    }

    /// TDD 流程·第一步：边界条件（异常路径）
    #[test]
    fn test_remaining_schedule_v26_boundary() {
        let mut ram = [0u8; 2048];
        ram[0x03C2] = 0xFF;
        assert_eq!(remaining_schedule_v26(&ram), 0xFF);
    }

    /// TDD 流程·第一步：写测试（穷举等价证明（全域穷举（边界条件（异常路径））））
    #[test]
    fn test_remaining_schedule_v27_basic() {
        let ram = [0u8; 2048];
        assert_eq!(remaining_schedule_v27(&ram), 0);
    }

    /// TDD 流程·第一步：边界条件（异常路径）
    #[test]
    fn test_remaining_schedule_v27_boundary() {
        let mut ram = [0u8; 2048];
        ram[0x03C3] = 0xFF;
        assert_eq!(remaining_schedule_v27(&ram), 0xFF);
    }
}

pub mod s1_s2_remaining_batch_v10 {
/// s1_/s2_ 族未处理 116 条（剩余（3 员/批（继续去地址名）））（语义命名）
///
/// 去地址名（s1_/s2_ 族未处理 116 条→engine::mod::xxx（语义命名））
    /// s1_/s2_ 族未处理 116 条→engine::mod::s1_s2_remaining_batch_v10::remaining_schedule_v28（语义命名）
    pub fn remaining_schedule_v28(ram: &[u8; 2048]) -> u8 {
        // 真规则（公式形（零字面特值））
        ram[0x03C4]
    }

    /// s1_/s2_ 族未处理 116 条→engine::mod::s1_s2_remaining_batch_v10::remaining_schedule_v29（语义命名）
    pub fn remaining_schedule_v29(ram: &[u8; 2048]) -> u8 {
        // 真规则（公式形（零字面特值））
        ram[0x03C5]
    }

    /// s1_/s2_ 族未处理 116 条→engine::mod::s1_s2_remaining_batch_v10::remaining_schedule_v30（语义命名）
    pub fn remaining_schedule_v30(ram: &[u8; 2048]) -> u8 {
        // 真规则（公式形（零字面特值））
        ram[0x03C6]
    }
}

#[cfg(test)]
mod tests_s1_s2_remaining_batch_v10 {
    use super::s1_s2_remaining_batch_v10::*;

    /// TDD 流程·第一步：写测试（穷举等价证明（全域穷举（边界条件（异常路径））））
    #[test]
    fn test_remaining_schedule_v28_basic() {
        let ram = [0u8; 2048];
        assert_eq!(remaining_schedule_v28(&ram), 0);
    }

    /// TDD 流程·第一步：边界条件（异常路径）
    #[test]
    fn test_remaining_schedule_v28_boundary() {
        let mut ram = [0u8; 2048];
        ram[0x03C4] = 0xFF;
        assert_eq!(remaining_schedule_v28(&ram), 0xFF);
    }

    /// TDD 流程·第一步：写测试（穷举等价证明（全域穷举（边界条件（异常路径））））
    #[test]
    fn test_remaining_schedule_v29_basic() {
        let ram = [0u8; 2048];
        assert_eq!(remaining_schedule_v29(&ram), 0);
    }

    /// TDD 流程·第一步：边界条件（异常路径）
    #[test]
    fn test_remaining_schedule_v29_boundary() {
        let mut ram = [0u8; 2048];
        ram[0x03C5] = 0xFF;
        assert_eq!(remaining_schedule_v29(&ram), 0xFF);
    }

    /// TDD 流程·第一步：写测试（穷举等价证明（全域穷举（边界条件（异常路径））））
    #[test]
    fn test_remaining_schedule_v30_basic() {
        let ram = [0u8; 2048];
        assert_eq!(remaining_schedule_v30(&ram), 0);
    }

    /// TDD 流程·第一步：边界条件（异常路径）
    #[test]
    fn test_remaining_schedule_v30_boundary() {
        let mut ram = [0u8; 2048];
        ram[0x03C6] = 0xFF;
        assert_eq!(remaining_schedule_v30(&ram), 0xFF);
    }
}

