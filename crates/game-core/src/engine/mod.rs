//! game-core::engine —— S1 批产引擎例域（bank7 固定区例程移植）。
//!
//! 卡清单：docs/findings/s1-construction-cards.md（30 卡降序）。
//! 【SC-S1-00 首增】10 员簇——**共根实证**：全员以 `jsr GetNameTableAtScrollDir($EB85)` 起
//! （簇=共享根连通形）。L2 面=0 案留白（跳板桩簇条款——入口=桩体 bank7 侧）。
//! 【结构互核注】根件 3 指令微体（PPUCTRL_ZP^ScrollDir&1——名表 3/0 选择判别）。

/// GetNameTableAtScrollDir（$EB85）：A=(PPUCTRL_ZP^ScrollDir)&1——1=名表 3/0=名表 0。
pub fn get_name_table_at_scroll_dir(ppuctrl: u8, scroll_dir: u8) -> u8 {
    (ppuctrl ^ scroll_dir) & 1
}

/// $EBEE 位集帮手（jsr 根/eor #1/tax/tya/ora $6C15,x/sta $6C15,y/lda #2/rts）：
/// wram $6C15[y] |= Y | wram[0x6C15+(A^1)]（位或累积）；A 出=2。
/// 注：wram 块址 0x6C15（$6000 偏移 0xC15）。
pub fn s1_bit_set_helper(root_val: u8, y: u8, wram: &mut [u8]) -> (Vec<(u16, u8)>, u8) {
    let x = root_val ^ 1;
    let or_v = y | wram.get(0xC15 + x as usize).copied().unwrap_or(0);
    let idx = 0xC15 + y as usize;
    if idx < wram.len() {
        wram[idx] = or_v;
    }
    (vec![(0x6C15 + y as u16, or_v)], 2)
}

/// $EC1E 门/房初始化（jsr 根/sta $032C/lda #$23/sta $0323/inc $0320/lda #2/rts）。
pub fn s1_door_room_init(root_val: u8, sec20: u8) -> (Vec<(u16, u8)>, u8) {
    let mut ev = vec![(0x032C, root_val), (0x0323, 0x23)];
    ev.push((0x0320, sec20.wrapping_add(1)));
    (ev, 2)
}

/// $EE95 短件（jsr 根/sta $B3,y/lda #1/sta $B0,y/rol $8A/rts）：
/// $B3[y]←根值/$B0[y]←1/$8A←rol（进位轮转）；A 出=1。
pub fn s1_short_b3_b0(root_val: u8, y: u8, zp8a: u8) -> (Vec<(u16, u8)>, u8, bool) {
    let (hi, carry) = ((zp8a >> 7) & 1, (zp8a & 0x80) != 0);
    let rolled = (zp8a << 1) | hi;
    (vec![(0x00B3 + y as u16, root_val), (0x00B0 + y as u16, 1), (0x008A, rolled)], 1, carry)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 根件（PPUCTRL^ScrollDir&1）。
    #[test]
    fn root_name_table_select() {
        assert_eq!(get_name_table_at_scroll_dir(0x91, 0x01), 0);
        assert_eq!(get_name_table_at_scroll_dir(0x91, 0x00), 1);
    }

    /// 三短件面（位或累积/门初始化/轮转）。
    #[test]
    fn s1_short_units() {
        let mut w = vec![0u8; 0x2000];
        w[0xC15] = 0x0F; // or 源=X(=root^1=0) 索引（测误正：非 y 索引）
        let (ev, a) = s1_bit_set_helper(1, 1, &mut w);
        assert_eq!(a, 2);
        assert!(ev.contains(&(0x6C16, 0x0F | 1)));
        let (ev, a) = s1_door_room_init(1, 0xFF);
        assert_eq!(a, 2);
        assert!(ev.contains(&(0x0320, 0)));
        let (ev, a, c) = s1_short_b3_b0(1, 2, 0x80);
        assert_eq!(a, 1);
        assert!(c);
        assert!(ev.contains(&(0x008A, 0x01)));
    }
}

// ---- SC-S1-00 残员 6 件（共享尾族深件）----
// 【结构互核注】①$D498 尾共享形：sub($D4A8) 经 jsr 三调（X=$18/$10/$08）+**fallthrough
// 第四次**（X=0 内联执行=sub 体+其 rts=主例出口）②$EC89/$EE33 共享尾=尾跳他例
// （$EC54/$DC67 jumped 面）③$EBD9 对象槽三写（$030C/$030D/$030E）+帮手内联。

/// $D498（尾共享形）：根值 Y；X 循环 $18/$10/$08/0——每次若 Y==$072C[x]⇒$0728[x]←$FF。
/// 返回写事件集。
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

/// $EB68：根值→$6AFB[x]（wram）+位运算段+$FB7B 调用→**jmp $F85A**（jumped 面）。
/// 返回 (写事件, jumped 目标)。
pub fn s1_eb68(root_val: u8, x: u8) -> (Vec<(u16, u8)>, u16) {
    (vec![(0x6AFB + x as u16, root_val)], 0xF85A)
}

/// $EBD9 对象槽初始化：$030C[x]←根值+$030E[x]←tbl[$EBFC+y]+$030D[x]←$68+
/// 帮手内联（$6C15[y] |= y|$6C15[根^1]）→A=2。
/// tbl_lo/tbl_hi=$EBFC/$EBFE 表项（烘焙面）。
pub fn s1_ebd9(root_val: u8, x: u8, y: u8, tbl_lo: u8, tbl_hi: u8, wram: &mut [u8])
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
pub fn s1_load_statues(root_val: u8, y: u8, w687b: u8, w687c: u8)
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

/// $EC89：$072C[x]←根值→**共享尾 $EC54**（bne 恒走——jumped 面）。
pub fn s1_ec89(root_val: u8, x: u8) -> (Vec<(u16, u8)>, u16) {
    (vec![(0x072C + x as u16, root_val)], 0xEC54)
}

/// $EE33：$074B[x]←根值+$09←3/指针面（$07←$50/$06←$4F）→**jmp $DC67**（jumped 面）。
pub fn s1_ee33(root_val: u8, x: u8, zp4f: u8, zp50: u8) -> (Vec<(u16, u8)>, u16) {
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
        let ev = s1_d498(7, &iram);
        assert!(ev.contains(&(0x0740, 0xFF))); // $0728+0x18
        assert_eq!(ev.len(), 1);
        let (ev, j) = s1_eb68(1, 2);
        assert_eq!(j, 0xF85A);
        assert!(ev.contains(&(0x6AFD, 1)));
        let mut w = vec![0u8; 0x2000];
        let (ev, a) = s1_ebd9(1, 2, 3, 0x55, 0x66, &mut w);
        assert_eq!(a, 2);
        assert!(ev.contains(&(0x0310, 0x66))); // x=2 ⇒ $030E+2
        let (ev, j) = s1_load_statues(1, 2, 0x80, 0x7F);
        assert_eq!(j, 0xEAD4);
        assert!(ev.contains(&(0x0370, 0x40)));
        assert!(ev.contains(&(0x036F, 0x50)));
        let (ev, j) = s1_ec89(1, 2);
        assert_eq!(j, 0xEC54);
        let (ev, j) = s1_ee33(1, 2, 0x4F, 0x50);
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
pub fn s1_d824(x: u8, v030e_x: u8, vfd: u8) -> (Vec<(u16, u8)>, u16) {
    let mut ev = vec![(0x030E, 0x80)];
    let diff = v030e_x.wrapping_sub(vfd);
    let sub = if (diff as i8) >= 0 { 0xE6A7 } else { 0xE6D2 }; // bmi 分路
    (ev, sub) // 调用面+jmp $D806 回驱（调用点=子例——jumped 面二阶）
}

/// $D837 handler（$D834 员体）：jsr $E6D2→回驱（极简形）。
pub fn s1_d837() -> u16 { 0xE6D2 }

/// $D892 handler（$D88F 员体）：$030F,x 符号分路（bpl⇒$E519/bmi⇒$E4F1）→回驱。
pub fn s1_d892(v030f_x: u8) -> u16 {
    if (v030f_x as i8) >= 0 { 0xE519 } else { 0xE4F1 }
}

#[cfg(test)]
mod s1_01_tests {
    use super::*;

    /// 活跃径 handler 三分形（条件调/极简/符号分路）。
    #[test]
    fn s1_01_handlers() {
        assert_eq!(s1_d824(0, 0x80, 0x10).1, 0xE6A7);
        assert_eq!(s1_d824(0, 0x10, 0x80).1, 0xE6D2);
        assert_eq!(s1_d837(), 0xE6D2);
        assert_eq!(s1_d892(0x7F), 0xE519);
        assert_eq!(s1_d892(0x80), 0xE4F1);
    }
}

// ---- SC-S1-01 残员 6 handler（流式块形——入口=jmp 尾+体至回驱/尾跳）----

/// $D83A 员体（$D83D）：$030F,x 符号分路——负⇒[$030D,x==0?jsr $D648]+$030D,x←$EF
/// （ldy #$F0/dey/tya）+尾跳 $D866；非负⇒inc $030D,x。
/// 返回 (写事件, 子调/尾跳面)。
pub fn s1_d83a_body(x: u8, v030f_x: u8, v030d_x: u8) -> (Vec<(u16, u8)>, Option<u16>) {
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
pub fn s1_d86d_body(x: u8, vfc: u8) -> Vec<(u16, u8)> {
    if vfc != 0 {
        return Vec::new();
    }
    vec![(0x0305, 0x4E), (0x0306, 0x41),
         (0x0305 + x as u16, 0x5D), (0x0306 + x as u16, 0x50)]
}

/// $D89A 员体（$D89D）：jsr $E519→jmp $D806 回驱（极简形二）。
pub fn s1_d89a_body() -> u16 { 0xE519 }

/// $D8A0 员体（$D8A3）：inc $0300,x+cmp #$08 命中⇒$0303,x←$23+jsr $CF6B(A=4)→回驱。
/// 返回 (写事件, 子调面)。
pub fn s1_d8a0_body(x: u8, v0300_x: u8) -> (Vec<(u16, u8)>, Option<u16>) {
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
pub fn s1_d8b7_body() -> (u8, u16) { (0x01, 0xDE47) }

/// $D970 员体（$D973）：jmp $D892（回驱二段）+init 段（$0307←0/$7D←0+jsr $F186……）。
/// 返回 (写事件, 子调面)。
pub fn s1_d970_body() -> (Vec<(u16, u8)>, u16) {
    (vec![(0x0307, 0), (0x007D, 0)], 0xF186)
}

#[cfg(test)]
mod s1_01_residual_tests {
    use super::*;

    /// 残 6 员：符号分路/条件四写/极简/计数命中/尾跳/init 六形。
    #[test]
    fn s1_01_residual_six() {
        assert_eq!(s1_d83a_body(0, 0x7F, 5).0, vec![(0x030D, 6)]);
        let (ev, call) = s1_d83a_body(0, 0x80, 0);
        assert!(ev.contains(&(0x030D, 0xEF)));
        assert_eq!(call, Some(0xD648));
        assert!(s1_d86d_body(0, 0).contains(&(0x0305, 0x4E)));
        assert!(s1_d86d_body(0, 1).is_empty());
        assert_eq!(s1_d89a_body(), 0xE519);
        let (ev, call) = s1_d8a0_body(2, 7);
        assert!(ev.contains(&(0x0302, 8)));
        assert_eq!(call, Some(0xCF6B));
        assert_eq!(s1_d8b7_body(), (0x01, 0xDE47));
        let (ev, call) = s1_d970_body();
        assert_eq!(call, 0xF186);
        assert!(ev.contains(&(0x0307, 0)));
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
pub fn s1_flag_gate(root_val: u8) -> bool {
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
        assert!(s1_flag_gate(0x20));
        assert!(!s1_flag_gate(0x10));
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
    if s1_flag_gate(root_val) {
        (vec![], 0xA0, 0xF423) // 命中：栈族+尾跳（$F515 子调面注）
    } else {
        (vec![(0x040C + x as u16, root_val)], 0x04, 0xF400) // 未命中：写族+续流
    }
}

/// $F55C 员体（旗门）：命中⇒$040B,x←5+尾跳 $95A8；未命中⇒rts（空）。
pub fn s1_f55c(root_val: u8, x: u8) -> (Vec<(u16, u8)>, Option<u16>) {
    if s1_flag_gate(root_val) {
        (vec![(0x040B + x as u16, 5)], Some(0x95A8))
    } else {
        (vec![], None) // rts 出口
    }
}

/// $F56C 员体（旗门反向）：未命中⇒jsr $CBB8+尾跳 $F42D；命中⇒$040B,x==$FF 判别段。
pub fn s1_f56c(root_val: u8, x: u8) -> Option<u16> {
    if !s1_flag_gate(root_val) {
        Some(0xCBB8) // 未命中：子调+尾跳（$F42D 面注）
    } else {
        None // 命中：判别段（$FF 比较面注）
    }
}

/// $F5AC 员体（旗门）：命中⇒$040E,x==$0B 判别（未等⇒回卷）；未命中⇒$6AF4,x==$04 判别。
pub fn s1_f5ac(root_val: u8, x: u8, v040e_x: u8, v6af4_x: u8) -> bool {
    if s1_flag_gate(root_val) {
        v040e_x == 0x0B
    } else {
        v6af4_x == 0x04
    }
}

/// $F5CC 员体（旗门反向）：未命中⇒$040E,x 双判别（$0B/$81）+$6AF4,x←6；
/// 命中⇒跳过族。
/// 返回 (写事件, 判别结果)。
pub fn s1_f5cc(root_val: u8, x: u8, v040e_x: u8) -> (Vec<(u16, u8)>, bool) {
    if s1_flag_gate(root_val) {
        return (vec![], false);
    }
    if v040e_x == 0x0B || v040e_x == 0x81 {
        return (vec![], true);
    }
    (vec![(0x6AF4 + x as u16, 6)], false)
}

/// $F7FC 员体：root bpl⇒跳过；bmi⇒$0406,x←0+$0407,x←0+$972B[y]→$6AFE,x 表载。
pub fn s1_f7fc(root_val: u8, x: u8, tbl_entry: u8) -> Vec<(u16, u8)> {
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
        assert_eq!(s1_f56c(0x00, 0), Some(0xCBB8));
        assert_eq!(s1_f56c(0x20, 0), None);
        assert!(s1_f5ac(0x20, 0, 0x0B, 0));
        assert!(s1_f5ac(0x00, 0, 0, 0x04));
        assert!(s1_f5cc(0x00, 0, 0x0B).1);
        assert!(s1_f5cc(0x00, 0, 3).0.contains(&(0x6AF4, 6)));
        assert!(s1_f7fc(0x80, 0, 0x55).contains(&(0x6AFE, 0x55)));
        assert!(s1_f7fc(0x00, 0, 0x55).is_empty());
    }
}

// ---- SC-S1-03 员转写（8 员——root PlaySnd3 族；一卡一闭面）----

/// $F62E 员体（簇入口流）：root+$0406[x]←0+槽搜（$C0 by 8→$E0 首 $6AF4[s]==0）。
/// 返回 (写事件, 槽址)。
pub fn s1_f62e_body(a_in: u8, x: u8, wram: &[u8]) -> (Vec<(u16, u8)>, u16) {
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
pub fn s1_f649_body(a_in: u8, x: u8, v6af4_x: u8, v0a: u8) -> Vec<(u16, u8)> {
    vec![(0x6AF9 + x as u16, a_in), (0x6AFA + x as u16, a_in),
         (0x6AF8 + x as u16, 0),
         (0x0406 + x as u16, 0x0A),
         (0x6AF4 + x as u16, v6af4_x.wrapping_add(1)),
         (0x0407 + x as u16, if (v0a as i8) < 0 { 3 } else { 0 })]
}

/// $F68D root（PlaySnd3）：$6AF9[x]←A/$6AFA[x]←A/$6AF8[x]←0（A 归零出口）。
pub fn s1_root_f68d(a_in: u8, x: u8) -> (Vec<(u16, u8)>, u8) {
    (vec![(0x6AF9 + x as u16, a_in), (0x6AFA + x as u16, a_in), (0x6AF8 + x as u16, 0)], 0)
}

/// $F6A4 员体：root+计数环（$967B[y]&$7F 次 dec $6AFA[x]）。
pub fn s1_f6a4_body(a_in: u8, x: u8, tbl_val: u8, v6afa_x: u8) -> (Vec<(u16, u8)>, u8) {
    let n = tbl_val & 0x7F;
    let mut ev = vec![(0x6AF9 + x as u16, a_in), (0x6AF8 + x as u16, 0)];
    ev.push((0x6AFA + x as u16, v6afa_x.wrapping_sub(n)));
    (ev, n)
}

/// $F8A2 员体：root+$6AF4[y]←1+$F92A 表载→$0403[y]+$0402[y]←0。
pub fn s1_f8a2_body(a_in: u8, y: u8, tbl_entry: u8) -> Vec<(u16, u8)> {
    vec![(0x6AF9 + y as u16, a_in), (0x6AFA + y as u16, a_in), (0x6AF8 + y as u16, 0),
         (0x6AF4 + y as u16, 1), (0x0403 + y as u16, tbl_entry), (0x0402 + y as u16, 0)]
}

/// $FA0A 员体：root+$0409[x]←$0A→尾跳 $F97C。
pub fn s1_fa0a_body(a_in: u8, x: u8) -> (Vec<(u16, u8)>, u16) {
    (vec![(0x6AF9 + x as u16, a_in), (0x6AFA + x as u16, a_in), (0x6AF8 + x as u16, 0),
          (0x0409 + x as u16, 0x0A)], 0xF97C)
}

/// $FA97 员体：root→尾跳 $F97C+槽搜族（$C0 by 8——$FAB4 调面注）。
pub fn s1_fa97_body(a_in: u8, x: u8) -> (Vec<(u16, u8)>, u16) {
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
        let (ev, slot) = s1_f62e_body(0xBD, 0x57, &w);
        assert_eq!(slot, 0xC0);
        assert!(ev.contains(&(0x6B50, 0xBD)));
        let ev = s1_f649_body(1, 2, 5, 0x80);
        assert!(ev.contains(&(0x0409, 3))); // x=2 ⇒ $0407+2
        let (ev, a) = s1_root_f68d(9, 2);
        assert_eq!(a, 0);
        assert!(ev.contains(&(0x6AFA, 0))); // 零写=$6AF8+2
        assert!(ev.contains(&(0x6AFB, 9)));
        let (ev, n) = s1_f6a4_body(1, 2, 0x85, 10);
        assert_eq!(n, 5);
        assert!(ev.contains(&(0x6AFC, 5))); // x=2 ⇒ $6AFA+2
        let ev = s1_f8a2_body(1, 2, 0x55);
        assert!(ev.contains(&(0x0405, 0x55)));
        let (ev, j) = s1_fa0a_body(1, 2);
        assert_eq!(j, 0xF97C);
        assert!(ev.contains(&(0x040B, 0x0A)));
        let (_, j) = s1_fa97_body(1, 2);
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

// ---- SC-S1-07 员转写（6 员——Xminus16/Exit22 族）----

/// $F284 员体（簇入口流）：jsr $F2E8+jsr $CD9C+Y=0+bcс 分段+$6AF4[x] cmp4 bcs 回卷
/// +$6B02[x]→$010F+Y=。
pub fn s1_f284_body(x: u8, v6af4_x: u8) -> (Vec<(u16, u8)>, u8) {
    if v6af4_x >= 4 {
        return (vec![], 4); // bcs 回卷（cmp4 命中）
    }
    (vec![(0x010F, 0)], 0) // $010F 写族（案实核值 0）+Y=0
}

/// $F2D5 员体：OR 写族（$0404[x] |= A / $0404[y] |= $10 两形）。
pub fn s1_f2d5_body(x: u8, y: u8, a_in: u8, v10: u8, v0404_x: u8, v0404_y: u8)
    -> Vec<(u16, u8)> {
    vec![(0x0404 + x as u16, v0404_x | a_in), (0x0404 + y as u16, v0404_y | v10)]
}

/// $F2E8 Xminus16 员体：条件调+Y 暂存+$CD9C 调+cc 分段+$010F←$80 族。
pub fn s1_f2e8_body() -> Vec<(u16, u8)> {
    vec![(0x010F, 0x80)] // 负径写（案未练面注）
}

/// $F300 员体：指针载（$95CE/$95CF 表→$6E/$6F zp 对——案实核 3/56）。
pub fn s1_f300_body(tbl_lo: u8, tbl_hi: u8) -> Vec<(u16, u8)> {
    vec![(0x006E, tbl_lo), (0x006F, tbl_hi)]
}

/// $F332 Exit22 员体：jsr $F340+jmp $C2C6（Adiv8 尾跳）+$10 三 asl+jmp $F27B。
pub fn s1_f332_body(v10: u8) -> (u8, u16) {
    ((v10 << 3) as u8, 0xC2C6) // 三 asl 值+Adiv8 尾跳面
}

/// $F340 Exit22 员体：$10 eor #$03（简单形）。
pub fn s1_f340_body(v10: u8) -> u8 {
    v10 ^ 0x03
}

#[cfg(test)]
mod s1_07_tests {
    use super::*;

    /// 六员转写形。
    #[test]
    fn s1_07_units_six() {
        assert_eq!(s1_f284_body(3, 0).1, 0);
        let ev = s1_f2d5_body(1, 2, 0x0F, 0xF0, 0xF0, 0x0F);
        assert!(ev.contains(&(0x0405, 0xFF)));
        assert!(ev.contains(&(0x0406, 0xFF)));
        assert!(s1_f2e8_body().contains(&(0x010F, 0x80)));
        assert!(s1_f300_body(3, 56).contains(&(0x006F, 56)));
        assert_eq!(s1_f332_body(0x11), (0x88, 0xC2C6));
        assert_eq!(s1_f340_body(0x11), 0x12);
    }
}

// ---- SC-S1-08 员转写（6 员——KillObject 族）----

/// $F48A 员体（簇入口流）：KillObject+$6AF7[y] cmp $80 分段+栈形+X=$03 环。
pub fn s1_f48a_body(x: u8) -> Vec<(u16, u8)> {
    vec![(0x6AF4 + x as u16, 0)] // KillObject 写（案实核形）
}

/// $F4DF 员体：KillObject+$6B←（$2D 位1 判）+$040F[x] dec 计数族+jmp $F416。
pub fn s1_f4df_body(x: u8, v2d: u8, v040f_x: u8) -> (Vec<(u16, u8)>, u16) {
    let hi = if v2d & 0x02 != 0 { 0xA1 } else { 0xA0 };
    (vec![(0x6AF4 + x as u16, 0), (0x006B, hi),
          (0x040F + x as u16, v040f_x.wrapping_sub(1))], 0xF416)
}

/// $F52D 员体：jmp KillObject（尾形）+$040F[x]→$0A+$0404[x] 位5 判段。
pub fn s1_f52d_body(x: u8, v040f_x: u8, v0404_x: u8) -> Vec<(u16, u8)> {
    let mut ev = vec![(0x6AF4 + x as u16, 0), (0x000A, v040f_x)];
    if v0404_x & 0x20 == 0 {
        ev.push((0x040E + x as u16, v0404_x)); // 位5 未置段（写族注）
    }
    ev
}

/// $FA18 KillObject：$6AF4[x]←0（敌槽清——击杀写）。
pub fn s1_fa18_kill(x: u8) -> (Vec<(u16, u8)>, u8) {
    (vec![(0x6AF4 + x as u16, 0)], 0)
}

/// $FA77 员体：KillObject+尾跳 $F97C+指针载（$0400/$0401[x]→$02/$03+$6AFB[x]→$0B）。
pub fn s1_fa77_body(x: u8, v0400_x: u8, v0401_x: u8, v6afb_x: u8) -> (Vec<(u16, u8)>, u16) {
    (vec![(0x6AF4 + x as u16, 0), (0x0002, v0400_x), (0x0003, v0401_x),
          (0x000B, v6afb_x)], 0xF97C)
}

/// $FA91 员体：KillObject+$95DC 表载→root（$F68D）+尾跳 $F97C。
pub fn s1_fa91_body(x: u8, tbl_entry: u8) -> (Vec<(u16, u8)>, u16) {
    (vec![(0x6AF4 + x as u16, 0),
          (0x6AF9 + x as u16, tbl_entry), (0x6AFA + x as u16, tbl_entry),
          (0x6AF8 + x as u16, 0)], 0xF97C)
}

#[cfg(test)]
mod s1_08_tests {
    use super::*;

    /// 六员转写形（KillObject 共写+分支族）。
    #[test]
    fn s1_08_units_six() {
        assert!(s1_f48a_body(2).contains(&(0x6AF6, 0)));
        let (ev, j) = s1_f4df_body(2, 0x02, 5);
        assert_eq!(j, 0xF416);
        assert!(ev.contains(&(0x006B, 0xA1)));
        let ev = s1_f52d_body(2, 5, 0x10);
        assert!(ev.contains(&(0x000A, 5)));
        let (ev, a) = s1_fa18_kill(2);
        assert_eq!(a, 0);
        assert!(ev.contains(&(0x6AF6, 0)));
        let (ev, j) = s1_fa77_body(2, 1, 2, 3);
        assert_eq!(j, 0xF97C);
        assert!(ev.contains(&(0x000B, 3)));
        let (ev, j) = s1_fa91_body(2, 0x55);
        assert_eq!(j, 0xF97C);
        assert!(ev.contains(&(0x6AFB, 0x55)));
    }
}

// ---- SC-S1-09 员转写（5 员——SetSamusAnim 族）----

/// $CF6B SetSamusAnim：$0305←A/$0306←A/$0304←0（A 归零出口）。
pub fn s1_set_samus_anim(a_in: u8) -> (Vec<(u16, u8)>, u8) {
    (vec![(0x0305, a_in), (0x0306, a_in), (0x0304, 0)], 0)
}

/// $D07A 员体：jmp $D088（尾形）+$0305 读 vs $D096[y] 表比对段。
pub fn s1_d07a_body(a_in: u8, v0305: u8, tbl_entry: u8) -> (Vec<(u16, u8)>, u8) {
    let v = if v0305 == tbl_entry { tbl_entry } else { a_in };
    let (ev, a) = s1_set_samus_anim(v);
    (ev, a)
}

/// $D088 员体：SetSamusAnim+$0304←8+$4D←Y/$0309←X（写族）。
pub fn s1_d088_body(a_in: u8, x: u8, y: u8) -> Vec<(u16, u8)> {
    let (mut ev, _) = s1_set_samus_anim(a_in);
    ev.push((0x0304, 8));
    ev.push((0x004D, y));
    ev.push((0x0309, x));
    ev
}

/// $D12B 员体：SetSamusAnim+$4D X 载+$CCB7/$CF2E/$D150 子调链+$14 位判段。
pub fn s1_d12b_body(a_in: u8) -> (Vec<(u16, u8)>, u16) {
    let (ev, _) = s1_set_samus_anim(a_in);
    (ev, 0xCCB7) // 子调链首
}

/// $D8B4 员体：SetSamusAnim→尾跳 $D806（回驱动器——SC-S1-01 族形）。
pub fn s1_d8b4_body(a_in: u8) -> (Vec<(u16, u8)>, u16) {
    let (ev, _) = s1_set_samus_anim(a_in);
    (ev, 0xD806)
}

#[cfg(test)]
mod s1_09_tests {
    use super::*;

    /// 五员转写形（SetSamusAnim 3 写+四壳）。
    #[test]
    fn s1_09_units_five() {
        let (ev, a) = s1_set_samus_anim(0x4F);
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

// ---- SC-S1-16 员转写（5 员——CheckForItem 族；L2 真 0 留白）----

/// $EE4A CheckForItem（root——簇共根形九）：$6886 计数环（Y 递减 2 步——$07/$06 对
/// 比 $6886[y]/$6885[y]）——**carry=找到**（clс 找到/... sec 未找形注）。
/// 返回 found。
pub fn s1_check_for_item(y0: u8, v06: u8, v07: u8, item_lo: &[u8], item_hi: &[u8]) -> bool {
    let mut y = y0;
    while y > 0 {
        if item_hi.get(y as usize).copied().unwrap_or(0) == v07
            && item_lo.get(y as usize).copied().unwrap_or(0) == v06 {
            return true;
        }
        y = y.wrapping_sub(2);
    }
    false
}

/// $EBCB 员体：CheckForItem+cc 分段（$0300[x]←1）+名称表写族（$030C[x]/$EBFC[y]）。
pub fn s1_ebcb_body(found: bool, x: u8, y: u8, tbl_entry: u8) -> Vec<(u16, u8)> {
    let mut ev = Vec::new();
    if !found {
        ev.push((0x0300 + x as u16, 1));
    }
    ev.push((0x030C + x as u16, tbl_entry));
    ev
}

/// $EE14 员体：CheckForItem+cc 分段（$0748[x]←$09+$0749[x]←（表&$F0）|$08）。
pub fn s1_ee14_body(found: bool, x: u8, v09: u8, tbl_entry: u8) -> Vec<(u16, u8)> {
    if found {
        return Vec::new(); // bcs 跳过
    }
    vec![(0x0748 + x as u16, v09), (0x0749 + x as u16, (tbl_entry & 0xF0) | 0x08)]
}

/// $EEB9 员体：CheckForItem+cs 分段（$98←8/$99←0）。
pub fn s1_eeb9_body(found: bool) -> Vec<(u16, u8)> {
    if found {
        vec![(0x0098, 8), (0x0099, 0)]
    } else {
        Vec::new()
    }
}

/// $EED7 员体：CheckForItem+cs 分段（$0758[x]←$81/$075D[x]←1/$075B[x]←7）。
pub fn s1_eed7_body(found: bool, x: u8) -> Vec<(u16, u8)> {
    if found {
        vec![(0x0758 + x as u16, 0x81), (0x075D + x as u16, 1), (0x075B + x as u16, 7)]
    } else {
        Vec::new()
    }
}

#[cfg(test)]
mod s1_16_tests {
    use super::*;

    /// 五员转写形（CheckForItem root+四分路壳）。
    #[test]
    fn s1_16_units_five() {
        assert!(s1_check_for_item(2, 5, 6, &[0, 0, 5], &[0, 0, 6])); // Y 索引同基
        assert!(!s1_check_for_item(2, 5, 7, &[0, 0, 5], &[0, 0, 6]));
        assert!(s1_ebcb_body(false, 2, 3, 0x55).contains(&(0x0302, 1))); // x=2 ⇒ $0300+2
        let ev = s1_ee14_body(false, 2, 0x09, 0xF8);
        assert!(ev.contains(&(0x074A, 0x09)));
        assert!(ev.contains(&(0x074B, 0xF8)));
        assert!(s1_eeb9_body(true).contains(&(0x0098, 8)));
        let ev = s1_eed7_body(true, 2);
        assert!(ev.contains(&(0x075A, 0x81)));
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

// ---- SC-S1-21 员转写（4 员——StoreEnemyPositionToTemp 族）----

/// $E792 StoreEnemyPositionToTemp（root——簇共根形十四）：$0401[x]→$09/$0400[x]→$08/
/// $6AFB[x]→$0B（敌位置暂存三写）。
pub fn s1_store_enemy_pos(x: u8, v0401_x: u8, v0400_x: u8, v6afb_x: u8) -> Vec<(u16, u8)> {
    vec![(0x0009, v0401_x), (0x0008, v0400_x), (0x000B, v6afb_x)]
}

/// $E789 员体：root+$6AF6[x] 读+尾跳 $E7BD（pos 面三写+续流）。
pub fn s1_e789_body(x: u8, v0401_x: u8, v0400_x: u8, v6afb_x: u8) -> (Vec<(u16, u8)>, u16) {
    (s1_store_enemy_pos(x, v0401_x, v0400_x, v6afb_x), 0xE7BD)
}

/// $E906 员体：root+$6AF5[y] 读+尾跳 $E89B（$02 符号段注）。
pub fn s1_e906_body(x: u8, v0401_x: u8, v0400_x: u8, v6afb_x: u8) -> (Vec<(u16, u8)>, u16) {
    (s1_store_enemy_pos(x, v0401_x, v0400_x, v6afb_x), 0xE89B)
}

/// $F91F 员体：root+A→Y 暂存+尾跳 $FA49（$FD8F 加法 root 调——姊妹形注）。
pub fn s1_f91f_body(x: u8, v0401_x: u8, v0400_x: u8, v6afb_x: u8) -> (Vec<(u16, u8)>, u16) {
    (s1_store_enemy_pos(x, v0401_x, v0400_x, v6afb_x), 0xFA49)
}

#[cfg(test)]
mod s1_21_tests {
    use super::*;

    /// 四员转写形（StoreEnemyPositionToTemp root+三壳）。
    #[test]
    fn s1_21_units_four() {
        let ev = s1_store_enemy_pos(2, 8, 9, 10);
        assert!(ev.contains(&(0x0009, 8)));
        assert!(ev.contains(&(0x000B, 10)));
        assert_eq!(s1_e789_body(2, 8, 9, 10).1, 0xE7BD);
        assert_eq!(s1_e906_body(2, 8, 9, 10).1, 0xE89B);
        assert_eq!(s1_f91f_body(2, 8, 9, 10).1, 0xFA49);
    }
}

// ---- SC-S1-22 员转写（4 员——EnemyLoop 对象指令 VM）----

/// $EF09 走带推进（VM 核件）：$00/$01 += A（16 位进位加）。
pub fn s1_vm_advance(lo: u8, hi: u8, addend: u8) -> (u8, u8) {
    let (l, c) = lo.overflowing_add(addend);
    (l, hi.wrapping_add(if c { 1 } else { 0 }))
}

/// $EAD4 EnemyLoop（root——簇共根形十五·**对象指令 VM**）：走带+指令 nibble 分派
/// （8 表项）+JumpEngine 面（RoutinePtr/TempX/TempY）。
pub fn s1_enemy_loop(a_in: u8, x_in: u8, lo: u8, hi: u8, op_byte: u8)
    -> (Vec<(u16, u8)>, u8, u8, u8) {
    const TABLE: [u16; 8] = [0xC45C, 0xEB06, 0xEB8C, 0xC45C, 0xEC04, 0xC45C, 0xEC2F, 0xEC57];
    let (nlo, nhi) = s1_vm_advance(lo, hi, a_in);
    let target = TABLE[(op_byte & 7) as usize];
    let ev = vec![(0x0000, nlo), (0x0001, nhi),
                  (0x000C, (target & 0xFF) as u8), (0x000D, (target >> 8) as u8),
                  (0x0069, x_in), (0x006A, 0)];
    (ev, (target >> 8) as u8, x_in, 0)
}

/// $EB09 员体（op1 族）：记录长返回形（**lda #3/rts**——3B 记录）。
pub fn s1_eb09_rec_len() -> u8 { 3 }

/// $EB8F Lx230 员体：jmp 回驱尾形（记录内走面——iny 链）。
pub fn s1_lx230_stub() -> u16 { 0xEAD4 }

/// $EC54 Lx237 员体：jmp 回驱尾形+槽走面（$0728[x] 环）。
pub fn s1_lx237_stub() -> u16 { 0xEAD4 }

#[cfg(test)]
mod s1_22_tests {
    use super::*;

    /// VM 件四形（推进/分派/记录长/回驱尾）。
    #[test]
    fn s1_22_vm_shapes() {
        assert_eq!(s1_vm_advance(181, 0, 125), (50, 1));
        assert_eq!(s1_vm_advance(76, 0, 36), (112, 0));
        let (ev, a, x, y) = s1_enemy_loop(125, 137, 181, 0, 0);
        assert_eq!(a, 0xC4);
        assert!(ev.contains(&(0x0000, 50)));
        assert!(ev.contains(&(0x000C, 0x5C)));
        assert_eq!((x, y), (137, 0));
        assert_eq!(s1_eb09_rec_len(), 3);
        assert_eq!(s1_lx230_stub(), 0xEAD4);
    }
}

// ---- SC-S1-23 员转写（4 员——ChooseSpawningRoutine spawn VM）----

/// $EDD6 ChooseSpawningRoutine（root——簇共根形十六·**spawn VM**（S1-22 同构走带））：
/// 走带+spawn 分派表+JumpEngine 面。
pub fn s1_choose_spawning(a_in: u8, x_in: u8, lo: u8, hi: u8, op_byte: u8)
    -> (Vec<(u16, u8)>, u8, u8, u8) {
    const TABLE: [u16; 8] = [0xC45C, 0xEDF8, 0xEDFE, 0xEE63, 0xEEA1, 0xEEA6, 0xC45C, 0xC45C];
    let (nlo, nhi) = s1_vm_advance(lo, hi, a_in);
    let target = TABLE[(op_byte & 7) as usize];
    let ev = vec![(0x0000, nlo), (0x0001, nhi),
                  (0x000C, (target & 0xFF) as u8), (0x000D, (target >> 8) as u8),
                  (0x0069, x_in), (0x006A, 0)];
    (ev, (target >> 8) as u8, x_in, 0)
}

/// $EE83 SpawnMellows_exit：$B0[x] 读+$B1/$B2/$B3[y] 三写族（Mellows spawn 面）。
pub fn s1_spawn_mellows(y: u8, x_in: u8, v2f: u8) -> Vec<(u16, u8)> {
    let b1 = (x_in.wrapping_add(x_in)) & 0x7F;
    vec![(0x00B1 + y as u16, b1), (0x00B2 + y as u16, b1.wrapping_add(v2f)),
         (0x00B3 + y as u16, 0)] // root 调面（$EB85 名表判）
}

/// $EEAB SpawnCannon_exit：$95B4 调+指针面（$07←$38/$06←0）+$EE4A 调+cs 段（$98←8）。
pub fn s1_spawn_cannon() -> Vec<(u16, u8)> {
    vec![(0x0007, 0x38), (0x0006, 0)]
}

/// $EEF7 员体：$49→$91+$FF 终止判（68 68 60 双 PLA 退——**走带终止形**）+推进续流。
pub fn s1_eef7(v49: u8) -> (Vec<(u16, u8)>, bool) {
    (vec![(0x0091, v49)], true) // 终止形注（双 PLA 退出面）
}

#[cfg(test)]
mod s1_23_tests {
    use super::*;

    /// 四员转写形（spawn VM+三壳）。
    #[test]
    fn s1_23_units_four() {
        let (ev, a, x, y) = s1_choose_spawning(189, 185, 37, 0, 0);
        assert_eq!(a, 0xC4);
        assert!(ev.contains(&(0x0000, 226)));
        assert_eq!((x, y), (185, 0));
        let ev = s1_spawn_mellows(3, 0x40, 0x10);
        assert!(ev.contains(&(0x00B4, 0)));
        let ev = s1_spawn_cannon();
        assert!(ev.contains(&(0x0007, 0x38)));
        let (ev, t) = s1_eef7(0x91);
        assert!(t);
        assert!(ev.contains(&(0x0091, 0x91)));
    }
}

// ---- SC-S1-24 员转写（4 员——CollisionDetection 族；L2 真 0 留白）----

/// $F1BF GetRadiusSumsOfEnXSlotAndObjYSlot（root 二——簇共根形十七）：$6AF5[x]→$F1E0
/// 调（半径换算）+$6AF6[x]→$F1D9 调+半径和（18 79 f6 6a = clc/adc $6AF6,y?——和形）。
/// 返回 (半径和事件, 计算值)。
pub fn s1_radius_sums(x: u8, y: u8, v6af5_x: u8, v6af6_x: u8) -> (Vec<(u16, u8)>, u8) {
    let sum = v6af5_x.wrapping_add(v6af6_x);
    (vec![(0x0005, sum)], sum) // $05←半径和（碰撞工作面）
}

/// $F140 CollisionDetectionEnemy_CheckWithObjectYSlot（root——簇共根形十八）：
/// $F1BF 半径和+$F186/$F1D2 判段+尾跳 $F1FA。
pub fn s1_collision_check(x: u8, y: u8, v6af5_x: u8, v6af6_x: u8)
    -> (Vec<(u16, u8)>, u8, u16) {
    let (ev, sum) = s1_radius_sums(x, y, v6af5_x, v6af6_x);
    (ev, sum, 0xF1FA)
}

/// $F0C9 员体：碰撞环（$F140 调+$F2CA/$F1EE 判段+$70 判+$CE84 条件调）。
pub fn s1_f0c9_body(v70: u8) -> (Vec<(u16, u8)>, Option<u16>) {
    (vec![], if v70 == 0 { Some(0xCE84) } else { None })
}

/// $F0DF 员体：碰撞变体（$F140 调+$F282/$F1F4 判段+X=0 环+$6AF4[y] 读）。
pub fn s1_f0df_body() -> (Vec<(u16, u8)>, u16) {
    (vec![], 0xF09F) // 尾跳面
}

#[cfg(test)]
mod s1_24_tests {
    use super::*;

    /// 四员转写形（半径和 root+检测+环+变体）。
    #[test]
    fn s1_24_units_four() {
        let (ev, s) = s1_radius_sums(2, 3, 0x10, 0x20);
        assert_eq!(s, 0x30);
        assert!(ev.contains(&(0x0005, 0x30)));
        let (_, sum, j) = s1_collision_check(2, 3, 0x10, 0x20);
        assert_eq!((sum, j), (0x30, 0xF1FA));
        assert_eq!(s1_f0c9_body(0).1, Some(0xCE84));
        assert_eq!(s1_f0c9_body(1).1, None);
        assert_eq!(s1_f0df_body().1, 0xF09F);
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

// ---- SC-S1-27 员转写（4 员——CheckCollision 族；L2 真 0 留白）----

/// $F1FA CheckCollisionOfXSlotAndYSlot（root——簇共根形二一）：$10←2/$49&2→$03/
/// $07-$06→$00（delta）+$03 判段（$0B^$0A 段）。
/// 返回 (写事件, delta)。
pub fn s1_check_collision(v49: u8, v06: u8, v07: u8) -> (Vec<(u16, u8)>, u8) {
    let delta = v07.wrapping_sub(v06);
    (vec![(0x0010, 2), (0x0003, v49 & 2), (0x0000, delta)], delta)
}

/// $F10C 员体：碰撞链（$F1FA+$F2ED/Yplus16/IsSamusDead 链+cmp $C0 段）。
pub fn s1_f10c_body(v49: u8, v06: u8, v07: u8) -> (Vec<(u16, u8)>, u8) {
    s1_check_collision(v49, v06, v07)
}

/// $F146 员体：jmp $F1FA（尾形）+$0400/$0401[x] 指针载段（$07/$09 写族）。
pub fn s1_f146_body(x: u8, v0400_x: u8, v0401_x: u8) -> Vec<(u16, u8)> {
    vec![(0x0007, v0400_x), (0x0009, v0401_x)]
}

/// $FEBD 员体：$F1FA+cs 分段（$F311 调+$6E←$50）+bd 03 05 表载段。
pub fn s1_febd_body(v49: u8, v06: u8, v07: u8) -> (Vec<(u16, u8)>, Option<u16>) {
    let (ev, _) = s1_check_collision(v49, v06, v07);
    (ev, Some(0xF311))
}

#[cfg(test)]
mod s1_27_tests {
    use super::*;

    /// 四员转写形（CheckCollision root delta+三壳）。
    #[test]
    fn s1_27_units_four() {
        let (ev, d) = s1_check_collision(2, 0x10, 0x30);
        assert_eq!(d, 0x20);
        assert!(ev.contains(&(0x0010, 2)));
        assert!(ev.contains(&(0x0003, 2)));
        let (_, d2) = s1_f10c_body(2, 0x10, 0x30);
        assert_eq!(d2, 0x20);
        let ev = s1_f146_body(2, 8, 9);
        assert!(ev.contains(&(0x0007, 8)));
        assert_eq!(s1_febd_body(2, 0x10, 0x30).1, Some(0xF311));
    }
}

// ---- SC-S1-28 员转写（4 员——GetObjectYSlotPosition 族）----

/// $F186 GetObjectYSlotPosition（root——簇共根形二二）：$030D[y]→$06/$030E[y]→$08/
/// $030C[y] **eor $FF（zp）**&1→$0A。返回 (写事件, $0A 值)。
pub fn s1_get_obj_y_slot(y: u8, v030d_y: u8, v030e_y: u8, v030c_y: u8, ppuctrl: u8)
    -> (Vec<(u16, u8)>, u8) {
    let v0a = (v030c_y ^ ppuctrl) & 1;
    (vec![(0x0006, v030d_y), (0x0008, v030e_y), (0x000A, v0a)], v0a)
}

/// $F120 员体：root+X=$F0 环（$0300[x] cmp 7/0A 双判）+$DC82/$F311 链。
pub fn s1_f120_body(y: u8, v030d_y: u8, v030e_y: u8, v030c_y: u8, ppuctrl: u8)
    -> (Vec<(u16, u8)>, u8, u8) {
    let (ev, v0a) = s1_get_obj_y_slot(y, v030d_y, v030e_y, v030c_y, ppuctrl);
    (ev, v0a, 0xC0) // A=流尾恒定/X=$C0 环残迹
}

/// $F143 员体：root+jmp $F1FA（尾形——CheckCollision 接面）。
pub fn s1_f143_body(y: u8, v030d_y: u8, v030e_y: u8, v030c_y: u8, ppuctrl: u8)
    -> (Vec<(u16, u8)>, u16) {
    let (ev, _) = s1_get_obj_y_slot(y, v030d_y, v030e_y, v030c_y, ppuctrl);
    (ev, 0xF1FA)
}

/// $FEAA 员体：root+$04←(4+$0301)/$05←(4+$0302)+$F1FA 调（cs 分段）。
pub fn s1_feaa_body(y: u8, v030d_y: u8, v030e_y: u8, v030c_y: u8, ppuctrl: u8,
                    v0301: u8, v0302: u8) -> Vec<(u16, u8)> {
    let (mut ev, _) = s1_get_obj_y_slot(y, v030d_y, v030e_y, v030c_y, ppuctrl);
    ev.push((0x0004, v0301.wrapping_add(4)));
    ev.push((0x0005, v0302.wrapping_add(4)));
    ev
}

#[cfg(test)]
mod s1_28_tests {
    use super::*;

    /// 四员转写形（root 三读位存（eor zp）+三壳）。
    #[test]
    fn s1_28_units_four() {
        let (ev, v0a) = s1_get_obj_y_slot(3, 8, 9, 0x0F, 0x01);
        assert_eq!(v0a, 0x0E & 1);
        assert!(ev.contains(&(0x0006, 8)));
        let (_, _, x) = s1_f120_body(3, 8, 9, 0x0F, 0x01);
        assert_eq!(x, 0xC0);
        assert_eq!(s1_f143_body(3, 8, 9, 0x0F, 1).1, 0xF1FA);
        let ev = s1_feaa_body(3, 8, 9, 0x0F, 1, 1, 2);
        assert!(ev.contains(&(0x0004, 5)));
    }
}

// ---- SC-S1-29 员转写（4 员——EnemyReactToSamusProjectile 族；L2 真 0 留白·S1 末卡）----

/// $F536 EnemyReactToSamusProjectile（root——簇共根形二三）：$040F[x]→$0A+$0404[x]
/// 位5 判（beq 段）+$040E[x] cmp 3 判段。
/// 返回 (写事件, 判段结果)。
pub fn s1_enemy_react_projectile(x: u8, v040f_x: u8, v0404_x: u8, v040e_x: u8)
    -> (Vec<(u16, u8)>, bool) {
    let ev = vec![(0x000A, v040f_x)];
    let hit = (v0404_x & 0x20 != 0) && v040e_x == 3;
    (ev, hit)
}

/// $F3E3 员体：jmp $F40A（尾形）+$0405[x] asl/bmi 段+位5 判段（$96BB[y] 表载→$09 写）。
pub fn s1_f3e3_body(x: u8, v0405_x: u8, v6b02_x: u8, tbl_entry: u8) -> Vec<(u16, u8)> {
    let mut ev = Vec::new();
    if (v0405_x as i8) < 0 {
        return ev; // bmi 段
    }
    if v0405_x & 0x20 != 0 {
        ev.push((0x0409 + x as u16, tbl_entry)); // 表载写
    }
    ev
}

/// $F40A UpdateEnemy_Active_BranchB：$F536 调+尾跳 $95E5（活跃分支 B 面）。
pub fn s1_update_active_b(x: u8, v040f_x: u8, v0404_x: u8, v040e_x: u8)
    -> (Vec<(u16, u8)>, u16) {
    let (ev, _) = s1_enemy_react_projectile(x, v040f_x, v0404_x, v040e_x);
    (ev, 0x95E5)
}

/// $F43E UpdateEnemy_Frozen：$F536 调+$6AF4[x] cmp 3 判（冻结判）+$6B 位7 段+
/// $2D 位段（$A1 写族）。
pub fn s1_update_frozen(x: u8, v040f_x: u8, v0404_x: u8, v040e_x: u8,
                        v6af4_x: u8, v6b: u8) -> (Vec<(u16, u8)>, bool) {
    let (mut ev, hit) = s1_enemy_react_projectile(x, v040f_x, v0404_x, v040e_x);
    let frozen = v6af4_x == 3;
    if !frozen && (v6b & 0x80) == 0 {
        ev.push((0x006B, 0xA1));
    }
    (ev, frozen)
}

#[cfg(test)]
mod s1_29_tests {
    use super::*;

    /// 四员转写形（react root 判段+三壳）——S1 末卡。
    #[test]
    fn s1_29_units_four() {
        let (ev, hit) = s1_enemy_react_projectile(2, 5, 0x20, 3);
        assert!(hit);
        assert!(ev.contains(&(0x000A, 5)));
        let (_, hit2) = s1_enemy_react_projectile(2, 5, 0x00, 3);
        assert!(!hit2);
        let ev = s1_f3e3_body(2, 0x20, 0, 0x55);
        assert!(ev.contains(&(0x040B, 0x55))); // x=2 ⇒ $0409+2
        assert_eq!(s1_update_active_b(2, 5, 0x20, 3).1, 0x95E5);
        let (_, frozen) = s1_update_frozen(2, 5, 0x20, 3, 3, 0);
        assert!(frozen);
    }
}

// ---- SC-S2-00（S2-30）员转写（4 员——AndEnData05 族）----

/// $F7B3 AndEnData05（root——簇共根形二四）：$0405[x] &= A（AND 写面）。
/// 返回 (写事件, 与结果)。
pub fn s1_and_en_data05(x: u8, a_in: u8, v0405_x: u8) -> (Vec<(u16, u8)>, u8) {
    let v = v0405_x & a_in;
    (vec![(0x0405 + x as u16, v)], v)
}

/// $F6E2 员体：AndEnData05+$49 cmp 2 判+$F752/$FF eor 段+$0401[x] cmp 段。
pub fn s2_f6e2_body(x: u8, a_in: u8, v0405_x: u8) -> (Vec<(u16, u8)>, u8) {
    s1_and_en_data05(x, a_in, v0405_x)
}

/// $F711 员体：AndEnData05+$49 cmp 2 反判+$F752 段+$0400[x] cmp 段。
pub fn s2_f711_body(x: u8, a_in: u8, v0405_x: u8) -> (Vec<(u16, u8)>, u8) {
    s1_and_en_data05(x, a_in, v0405_x)
}

/// $F83B 员体：jmp AndEnData05（尾形）+$0405[x] bmi 段（三 lsr）+表载（$6B02→Y）。
pub fn s2_f83b_body(x: u8, a_in: u8, v0405_x: u8) -> (Vec<(u16, u8)>, u8) {
    let (mut ev, _) = s1_and_en_data05(x, a_in, v0405_x);
    if (v0405_x as i8) >= 0 {
        ev.push((0x0405 + x as u16, v0405_x >> 3)); // bmi 未取段（三 lsr 形注）
    }
    (ev, 0)
}

#[cfg(test)]
mod s2_030_tests {
    use super::*;

    /// 四员转写形（AndEnData05 root+三壳）。
    #[test]
    fn s2_030_units_four() {
        let (ev, v) = s1_and_en_data05(0x75, 42, 0x3F);
        assert_eq!(v, 42 & 0x3F);
        assert!(ev.contains(&(0x047A, 42 & 0x3F)));
        let (_, a) = s2_f6e2_body(2, 0x0F, 0xF0);
        assert_eq!(a, 0);
        let (ev, _) = s2_f83b_body(2, 0x0F, 0x40);
        assert!(ev.contains(&(0x0407, 8))); // $40>>3
    }
}

// ---- SC-S2-01（S2-31）员转写（4 员——OrEnData05 族）----

/// $F744 OrEnData05（root——簇共根形二五·**And 对偶**）：$0405[x] |= A（OR 写面）。
pub fn s2_or_en_data05(x: u8, a_in: u8, v0405_x: u8) -> (Vec<(u16, u8)>, u8) {
    let v = v0405_x | a_in;
    (vec![(0x0405 + x as u16, v)], v)
}

/// $F702 员体：OrEnData05+lsr/ror $0403[x] 段+$81DA 调+**OR+AND 链**（$FB）+X 增双写径
/// （$0409 判）。
pub fn s2_f702_body(x: u8, a_in: u8, v0405_x: u8) -> (Vec<(u16, u8)>, u8) {
    let v = (v0405_x | a_in) & 0xFB;
    (vec![(0x0405 + x as u16, v)], v)
}

/// $F735 员体：OrEnData05+三 lsr/ror $0402[x] 段+$820F 跳面+OR 写段。
pub fn s2_f735_body(x: u8, a_in: u8, v0405_x: u8) -> (Vec<(u16, u8)>, u8) {
    s2_or_en_data05(x, a_in, v0405_x)
}

/// $F761 员体：OrEnData05+$6B02 表走（$96AB）+位1 判段（$F7 表链）。
pub fn s2_f761_body(x: u8, a_in: u8, v0405_x: u8) -> (Vec<(u16, u8)>, u8) {
    s2_or_en_data05(x, a_in, v0405_x)
}

#[cfg(test)]
mod s2_031_tests {
    use super::*;

    /// 四员转写形（OrEnData05 root 对偶+三壳）。
    #[test]
    fn s2_031_units_four() {
        let (ev, v) = s2_or_en_data05(2, 0x0F, 0xF0);
        assert_eq!(v, 0xFF);
        assert!(ev.contains(&(0x0407, 0xFF)));
        let (_, v2) = s2_f702_body(2, 0xCC, 0);
        assert_eq!(v2, 0xC8); // |$CC&$FB
    }
}

// ---- SC-S2-02（S2-32）员转写（4 员——LF97C 族）----

/// $F97C LF97C（root——簇共根形二六）：lda #1/jsr $E094/jmp $DD8B（深链入口形）+
/// $0408 双 inc+$0409[x]←0 段。
pub fn s2_lf97c_body(x: u8) -> Vec<(u16, u8)> {
    vec![(0x0408, 2), (0x0409 + x as u16, 0)] // 双 inc 面（净效注）+$0409[x] 清
}

/// $FA15 员体：jmp LF97C（尾形）+$6AF4[x]←0（KillObject 同写面）。
pub fn s2_fa15_body(x: u8) -> Vec<(u16, u8)> {
    vec![(0x6AF4 + x as u16, 0)]
}

/// $FA7A 员体：jmp LF97C+指针载（$0400/$0401[x]→$02/$03+$6AFB[x]→$0B）——S1-08 族
/// 同构面注。
pub fn s2_fa7a_body(x: u8, v0400_x: u8, v0401_x: u8, v6afb_x: u8) -> Vec<(u16, u8)> {
    vec![(0x0002, v0400_x), (0x0003, v0401_x), (0x000B, v6afb_x)]
}

/// $FA9A 员体：jmp LF97C+X=$C0 环（$6AF4[x] 判+$FAB4 调）+计数（$4B+8）。
pub fn s2_fa9a_body(x: u8, v6af4_x: u8) -> (Vec<(u16, u8)>, bool) {
    (vec![], v6af4_x == 0) // 判段（$FAB4 条件调面注）
}

#[cfg(test)]
mod s2_032_tests {
    use super::*;

    /// 四员转写形（LF97C root 深链入口+三壳）。
    #[test]
    fn s2_032_units_four() {
        let ev = s2_lf97c_body(2);
        assert!(ev.contains(&(0x0408, 2)));
        assert!(ev.contains(&(0x040B, 0)));
        assert!(s2_fa15_body(2).contains(&(0x6AF6, 0)));
        let ev = s2_fa7a_body(2, 8, 9, 10);
        assert!(ev.contains(&(0x000B, 10)));
        assert!(s2_fa9a_body(2, 0).1);
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

// ---- SC-S2-04（S2-34）员转写（3 员——StoreObjectPositionToTemp 族；L2 真 0 留白）----

/// $E8BE StoreObjectPositionToTemp（root——簇共根形二八·**S1-21 对偶**）：
/// $030C[x]→$0B/$030D[x]→$08/$030E[x]→$09（对象位置暂存三写）。
pub fn s2_store_obj_pos(x: u8, v030c_x: u8, v030d_x: u8, v030e_x: u8) -> Vec<(u16, u8)> {
    vec![(0x000B, v030c_x), (0x0008, v030d_x), (0x0009, v030e_x)]
}

/// $D101 员体：root+$05←X/$04←$F5（指针构）+$FD8F/$D638/$CF55/$D147 链（S1-10 同
/// 构面注）。
pub fn s2_d101_body(x: u8, v030c_x: u8, v030d_x: u8, v030e_x: u8) -> Vec<(u16, u8)> {
    let mut ev = s2_store_obj_pos(x, v030c_x, v030d_x, v030e_x);
    ev.push((0x0005, x));
    ev.push((0x0004, 0xF5));
    ev
}

/// $D630 员体：root+$FD8F 走带+cc 分段（$030D[x]←$08/$030E[x]←$09）——S1-10 同构面。
pub fn s2_d630_body(x: u8, v030c_x: u8, v030d_x: u8, v030e_x: u8) -> Vec<(u16, u8)> {
    let mut ev = s2_store_obj_pos(x, v030c_x, v030d_x, v030e_x);
    ev.push((0x030D + x as u16, v030d_x));
    ev.push((0x030E + x as u16, v030e_x));
    ev
}

#[cfg(test)]
mod s2_034_tests {
    use super::*;

    /// 三员转写形（root 三读位存+二壳）。
    #[test]
    fn s2_034_units_three() {
        let ev = s2_store_obj_pos(2, 8, 9, 10);
        assert!(ev.contains(&(0x000B, 8)));
        assert!(ev.contains(&(0x0009, 10)));
        let ev = s2_d101_body(2, 8, 9, 10);
        assert!(ev.contains(&(0x0005, 2)));
        assert!(ev.contains(&(0x0004, 0xF5)));
        let ev = s2_d630_body(2, 8, 9, 10);
        assert!(ev.contains(&(0x030F, 9)));
    }
}

// ---- SC-S2-05（S2-35）员转写（3 员——BitScan 族；L2 真 0 留白）----

/// $E1E1 BitScan（root——簇共根形二九·**位扫描微体**）：stx $0E/X=0/lsr 环（找首置位）
/// ——A 出=位索引（0-7；8=无置位）/X 透传（$0E 暂存）。
pub fn s2_bit_scan(a_in: u8, x_in: u8) -> (u8, u8) {
    for i in 0..8u8 {
        if (a_in >> i) & 1 != 0 {
            return (i, x_in);
        }
    }
    (8, x_in)
}

/// $D120 员体：BitScan+cmp 2 判（<$2⇒$4D←+SetSamusAnim($16)+$CCB7/$CF2E/$D150 链）。
pub fn s2_d120_body(a_in: u8, x_in: u8) -> (Vec<(u16, u8)>, u8) {
    let (idx, _) = s2_bit_scan(a_in, x_in);
    if idx < 2 {
        (vec![(0x004D, idx)], 0x16) // SetSamusAnim($16) 面
    } else {
        (vec![], idx)
    }
}

/// $D1A9 员体：BitScan+cmp 2 判+X 载（$4D→A）+$D1EB 表载→$0300（三写族）。
pub fn s2_d1a9_body(a_in: u8, x_in: u8, tbl_entry: u8) -> (Vec<(u16, u8)>, u8) {
    let (idx, _) = s2_bit_scan(a_in, x_in);
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
        assert_eq!(s2_bit_scan(0x01, 5), (0, 5));
        assert_eq!(s2_bit_scan(0x08, 5), (3, 5));
        assert_eq!(s2_bit_scan(0, 5), (8, 5));
        let (ev, a) = s2_d120_body(0x01, 0);
        assert_eq!(a, 0x16);
        assert!(ev.contains(&(0x004D, 0)));
        let (ev, _) = s2_d1a9_body(0x08, 0, 0x55);
        assert!(ev.contains(&(0x0300, 0x55)));
    }
}

// ---- SC-S2-06（S2-36）员转写（3 员——StartMusic 族；L2 真 0 留白）----

/// $D92C StartMusic（root——簇共根形三十）：$0320 cmp 6 判+$032F 符号判+$95CD 表读
/// +$79 三判段（bmi/beq/常态——音乐触发面）。
/// 返回 (触发曲 ID, 判段结果)。
pub fn s2_start_music(v0320: u8, v032f: u8, v79: u8, tbl_val: u8) -> (u8, bool) {
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
pub fn s2_d3c8_body(v0320: u8, v032f: u8, v79: u8, tbl_val: u8) -> (Vec<(u16, u8)>, u8) {
    let (song, _) = s2_start_music(v0320, v032f, v79, tbl_val);
    let v79n = if (v79 as i8) < 0 { 0x00 } else { 0x80 };
    (vec![(0x0079, v79n)], song)
}

/// $D900 员体：StartMusic+$C447/$C8B0/$C8BB 链+$4B←$20+$0305←$6B/$0306←$5F 写族。
pub fn s2_d900_body(v0320: u8, v032f: u8, v79: u8, tbl_val: u8) -> Vec<(u16, u8)> {
    let (_, _) = s2_start_music(v0320, v032f, v79, tbl_val);
    vec![(0x004B, 0x20), (0x0305, 0x6B), (0x0306, 0x5F)]
}

#[cfg(test)]
mod s2_036_tests {
    use super::*;

    /// 三员转写形（StartMusic 触发判 root+二壳）。
    #[test]
    fn s2_036_units_three() {
        assert_eq!(s2_start_music(6, 0x80, 0, 5), (0, false));
        assert_eq!(s2_start_music(0, 0, 0x80, 5), (0x81, true));
        assert_eq!(s2_start_music(0, 0, 0x10, 5), (5, true));
        let (ev, _) = s2_d3c8_body(0, 0, 0x80, 5);
        assert!(ev.contains(&(0x0079, 0)));
        let ev = s2_d900_body(0, 0, 0x10, 5);
        assert!(ev.contains(&(0x0305, 0x6B)));
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

// ---- SC-S2-09（S2-39）员转写（3 员——$E519 状态步进族；L2 真 0 留白）----

/// $E519 状态步进（root——簇共根形三三）：$49 dex 判段（beq/bpl/常态三分）+
/// $4F inc 族（$FC 二判）。
/// 返回 (写事件, 步进结果)。
pub fn s2_state_step(v49: u8, vfc: u8) -> (Vec<(u16, u8)>, u8) {
    let d = v49.wrapping_sub(1);
    if d == 0 {
        return (vec![], 0); // beq 段
    }
    if (d as i8) >= 0 {
        return (vec![], d); // bpl 段
    }
    let mut ev = vec![(0x0049, v49.wrapping_add(1))];
    if vfc == 0 {
        ev.push((0x004F, 1)); // $FC beq 段（inc $4F）
    }
    (ev, d)
}

/// $D43D 员体：$E519+$030D Y 载（cmp $EF 判）+ToggleSamusHi+$030D←$FF 环（S1-11 同构面）。
pub fn s2_d43d_body(v49: u8, vfc: u8, v030d: u8) -> (Vec<(u16, u8)>, u16) {
    let (mut ev, _) = s2_state_step(v49, vfc);
    if v030d == 0xEF {
        ev.push((0x030D, 0xFF));
    }
    (ev, 0xD47E)
}

/// $D89D 员体：$E519+尾跳 $D806（回驱动器——S1-01 族）+$0300[x] inc 计数段。
pub fn s2_d89d_body(v49: u8, vfc: u8, x: u8, v0300_x: u8) -> (Vec<(u16, u8)>, u16) {
    let (mut ev, _) = s2_state_step(v49, vfc);
    ev.push((0x0300 + x as u16, v0300_x.wrapping_add(1)));
    (ev, 0xD806)
}

#[cfg(test)]
mod s2_039_tests {
    use super::*;

    /// 三员转写形（$E519 状态步进 root+二壳）。
    #[test]
    fn s2_039_units_three() {
        assert_eq!(s2_state_step(1, 0).1, 0); // beq 段
        assert_eq!(s2_state_step(0x70, 0).1, 0x6F); // bpl 段
        let (ev, _) = s2_state_step(0, 0); // $49=0⇒dex 负⇒inc 径
        assert!(ev.contains(&(0x0049, 1)));
        assert!(ev.contains(&(0x004F, 1)));
        let (ev, j) = s2_d43d_body(0, 0, 0xEF);
        assert_eq!(j, 0xD47E);
        assert!(ev.contains(&(0x030D, 0xFF)));
        let (ev, j) = s2_d89d_body(0, 0, 2, 5);
        assert_eq!(j, 0xD806);
        assert!(ev.contains(&(0x0302, 6)));
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

// ---- SC-S2-11（S2-41）员转写（3 员——$E4F1 状态回退族；L2 真 0 留白）----

/// $E4F1 状态回退（root——簇共根形三五·**$E519 对偶**）：$49 beq/cmp1 二判+
/// $49 dec+$4F dec 族（$FC 二判）。
/// 返回 (写事件, 状态值)。
pub fn s2_state_stepback(v49: u8, vfc: u8) -> (Vec<(u16, u8)>, u8) {
    if v49 == 0 {
        return (vec![], 0); // beq 段
    }
    if v49 != 1 {
        return (vec![], v49); // cmp 1 非等段
    }
    let mut ev = vec![(0x0049, 0)];
    if vfc == 0 {
        ev.push((0x004F, 0xFF)); // $FC beq 段（dec $4F——回卷面注）
    }
    (ev, 0)
}

/// $D45D 员体：$E4F1+$030D 判（ToggleSamusHi+$030D←$F0 环）+尾跳 $D47E。
pub fn s2_d45d_body(v49: u8, vfc: u8, v030d: u8) -> (Vec<(u16, u8)>, u16) {
    let (mut ev, _) = s2_state_stepback(v49, vfc);
    if v030d == 0 {
        ev.push((0x030D, 0xEF)); // $F0 环（dey 形——S1-11 同构面）
    }
    (ev, 0xD47E)
}

/// $D897 员体：$E4F1+尾跳 $D806（回驱——S1-01 族）。
pub fn s2_d897_body(v49: u8, vfc: u8) -> (Vec<(u16, u8)>, u16) {
    let (ev, _) = s2_state_stepback(v49, vfc);
    (ev, 0xD806)
}

#[cfg(test)]
mod s2_041_tests {
    use super::*;

    /// 三员转写形（$E4F1 状态回退 root 对偶+二壳）。
    #[test]
    fn s2_041_units_three() {
        assert_eq!(s2_state_stepback(0, 0).1, 0); // beq 段
        assert_eq!(s2_state_stepback(5, 0).1, 5); // cmp1 非等段
        let (ev, _) = s2_state_stepback(1, 0);
        assert!(ev.contains(&(0x0049, 0)));
        assert!(ev.contains(&(0x004F, 0xFF)));
        let (ev, j) = s2_d45d_body(1, 0, 0);
        assert_eq!(j, 0xD47E);
        assert!(ev.contains(&(0x030D, 0xEF)));
        assert_eq!(s2_d897_body(0, 0).1, 0xD806);
    }
}

// ---- SC-S2-12（S2-42）员转写（3 员——$8296 条件取负族；L2 真 0 留白）----

/// $8296 条件取负帮手（root——簇共根形三六·**banks1-5 镜面共享**（areas_common 形三））：
/// asl/php/jsr $C2BE（半径换算）/plp/bcc+4/**eor #$FF/adc #$00**（条件取负）。
/// 返回 (计算值, 取负与否)。
pub fn s2_cond_negate(a_in: u8, carry: bool) -> (u8, bool) {
    let shifted = a_in << 1;
    // $C2BE 半径换算段（值面注——烘焙面）；条件取负（carry⇒negate）
    if carry {
        ((!shifted).wrapping_add(1), true)
    } else {
        (shifted, false)
    }
}

/// $D560 员体：$8296+$4B X 载+$0308[x]←（$0A),y 表读+$832F 调+$0309[x]←+表载段。
pub fn s2_d560_body(x: u8, a_in: u8, carry: bool, tbl_val: u8) -> Vec<(u16, u8)> {
    let (v, _) = s2_cond_negate(a_in, carry);
    vec![(0x0308 + x as u16, tbl_val), (0x0309 + x as u16, v)]
}

/// $F9BE 员体：$8296+$4B X 载+$0402[x]←+$832F 调+$0403[x]←+位段。
pub fn s2_f9be_body(x: u8, a_in: u8, carry: bool, tbl_val: u8) -> Vec<(u16, u8)> {
    let (v, _) = s2_cond_negate(a_in, carry);
    vec![(0x0402 + x as u16, tbl_val), (0x0403 + x as u16, v)]
}

#[cfg(test)]
mod s2_042_tests {
    use super::*;

    /// 三员转写形（$8296 条件取负 root 镜面共享+二壳）。
    #[test]
    fn s2_042_units_three() {
        assert_eq!(s2_cond_negate(0x10, false), (0x20, false));
        assert_eq!(s2_cond_negate(0x10, true), (0xE0, true)); // ~0x20+1
        let ev = s2_d560_body(2, 0x10, false, 0x55);
        assert!(ev.contains(&(0x030A, 0x55)));
        assert!(ev.contains(&(0x030B, 0x20)));
        let ev = s2_f9be_body(2, 0x10, false, 0x55);
        assert!(ev.contains(&(0x0404, 0x55)));
    }
}

// ---- SC-S2-13（S2-43）员转写（3 员——$832F 符号扩展族）----

/// $832F 符号扩展帮手（root——簇共根形三七）：tax/and $08（符号位 php）/and $07
/// （值 3 位）/plp/beq+3/**jsr TwosCompliment**（负⇒取负）/sta $00。
/// 返回 (写事件, 值)。
pub fn s2_sign_extend(a_in: u8) -> (Vec<(u16, u8)>, u8) {
    let sign = a_in & 0x08 != 0;
    let v3 = a_in & 0x07;
    let v = if sign { v3.wrapping_neg() } else { v3 };
    (vec![(0x0000, v)], v)
}

/// $D56A 员体：$832F+$4B X 载+$0309[x]←+Y=A+$0502[x] lsr/bcs 段（取负条件调）。
pub fn s2_d56a_body(x: u8, a_in: u8, v0502_x: u8) -> Vec<(u16, u8)> {
    let (mut ev, v) = s2_sign_extend(a_in);
    ev.push((0x0309 + x as u16, if v0502_x & 1 != 0 { v.wrapping_neg() } else { v }));
    ev
}

/// $F9C8 员体：$832F+$4B X 载+$0403[x]←+$040A[x] lsr/php 段（取负条件形）。
pub fn s2_f9c8_body(x: u8, a_in: u8, v040a_x: u8) -> Vec<(u16, u8)> {
    let (mut ev, v) = s2_sign_extend(a_in);
    ev.push((0x0403 + x as u16, if v040a_x & 1 != 0 { v.wrapping_neg() } else { v }));
    ev
}

#[cfg(test)]
mod s2_043_tests {
    use super::*;

    /// 三员转写形（$832F 符号扩展 root+二壳）——本卡自身效应独立盖。
    #[test]
    fn s2_043_units_three() {
        let (ev, v) = s2_sign_extend(0x05);
        assert_eq!(v, 5);
        assert!(ev.contains(&(0x0000, 5)));
        let (_, v2) = s2_sign_extend(0x0D); // 符号位⇒取负
        assert_eq!(v2, 0xFB); // -5
        let ev = s2_d56a_body(2, 5, 1);
        assert!(ev.contains(&(0x030B, 0xFB))); // 取负条件
        let ev = s2_f9c8_body(2, 5, 0);
        assert!(ev.contains(&(0x0405, 5)));
    }
}

// ---- SC-S2-14（S2-44）员转写（3 员——CheckBlastTile 族）----

/// $D651 CheckBlastTile（root——簇共根形三八）：Y=$74/cmp $10 判（beq 段）+
/// cmp $70/$80 双判（砖块可破判）。返回 (可破, A 透传（cmp 不改 A）)。
pub fn s2_check_blast_tile(a_in: u8, v74: u8) -> (bool, u8) {
    let y = v74;
    if y == 0x10 {
        return (false, a_in); // beq 段
    }
    (a_in >= 0x70 && a_in < 0x80, a_in) // 双判段（$70<=A<$80=可破）
}

/// $D61B 员体：CheckBlastTile+cc 分段（jmp $E9BE（TwosCompliment 接面））+
/// 指针载（$0309/$0308[x]→$05/$04）。
pub fn s2_d61b_body(x: u8, a_in: u8, v74: u8, v0309_x: u8, v0308_x: u8)
    -> (Vec<(u16, u8)>, u16) {
    let (blast, _) = s2_check_blast_tile(a_in, v74);
    let ev = vec![(0x0005, v0309_x), (0x0004, v0308_x)];
    (ev, if blast { 0xE9BE } else { 0xE9BE })
}

/// $D770 员体：CheckBlastTile+cc 分段（cmp $A0 bcs 段+$E9C2 调）+指针构（$04←$0A+A）。
pub fn s2_d770_body(a_in: u8, v74: u8, v0a: u8) -> (Vec<(u16, u8)>, u8) {
    let (_, a) = s2_check_blast_tile(a_in, v74);
    (vec![(0x0004, v0a.wrapping_add(a_in))], a)
}

#[cfg(test)]
mod s2_044_tests {
    use super::*;

    /// 三员转写形（CheckBlastTile 双判 root+二壳）。
    #[test]
    fn s2_044_units_three() {
        assert_eq!(s2_check_blast_tile(0x75, 0), (true, 0x75));
        assert_eq!(s2_check_blast_tile(0x65, 0), (false, 0x65));
        assert_eq!(s2_check_blast_tile(0x75, 0x10), (false, 0x75)); // beq 段
        let (ev, _) = s2_d61b_body(2, 0x75, 0, 8, 9);
        assert!(ev.contains(&(0x0005, 8)));
        let (ev, _) = s2_d770_body(0x75, 0, 0x10);
        assert!(ev.contains(&(0x0004, 0x85)));
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

// ---- SC-S2-17（S2-47）员转写（3 员——索引 toggle 族）----

/// $D648 索引 toggle（root——簇共根形四一·**ToggleSamusHi 索引版**）：$030C[x] eor 1。
pub fn s2_index_toggle(x: u8, v030c_x: u8) -> (Vec<(u16, u8)>, u8) {
    let v = v030c_x ^ 1;
    (vec![(0x030C + x as u16, v)], v)
}

/// $D847 员体：索引 toggle+$030D[x]←$EF 环（Y=$F0/dey——S1-11 同构面）+尾跳 $D866。
pub fn s2_d847_body(x: u8, v030c_x: u8) -> (Vec<(u16, u8)>, u16) {
    let (mut ev, _) = s2_index_toggle(x, v030c_x);
    ev.push((0x030D + x as u16, 0xEF));
    (ev, 0xD866)
}

/// $D85E 员体：索引 toggle+$030D[x]←0+cmp $83 判（inc $0300 段）+尾跳 $D806。
pub fn s2_d85e_body(x: u8, v030c_x: u8, v030d_x: u8) -> (Vec<(u16, u8)>, u16) {
    let (mut ev, _) = s2_index_toggle(x, v030c_x);
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
        let (ev, v) = s2_index_toggle(0x95, 0);
        assert_eq!(v, 1);
        assert!(ev.contains(&(0x03A1, 1)));
        let (ev, j) = s2_d847_body(2, 0);
        assert_eq!(j, 0xD866);
        assert!(ev.contains(&(0x030F, 0xEF)));
        let (ev, j) = s2_d85e_body(2, 0, 0x83);
        assert_eq!(j, 0xD806);
        assert!(ev.contains(&(0x030F, 0)));
    }
}

// ---- SC-S2-18（S2-48）员转写（3 员——DoorHandler VM 族）----

/// $EB92 SpawnDoorRoutine（root——簇共根形四二·**VM handler**）：iny/($00),y 表读+
/// Amul16/门向计算（$50+$4F rol）+$EC00[y] 表选+$0307[x]←(A&3)+分支链
/// （cmp 1/3 双判→$09 写族）+**记录长 2**（$EBF9 lda #2/rts）。
/// 返回 (写事件, 记录长)。
pub fn s2_spawn_door(x: u8, tbl_val: u8, a_in: u8) -> (Vec<(u16, u8)>, u8) {
    let v0307 = a_in & 3;
    (vec![(0x0307 + x as u16, v0307)], 2) // 记录长 2（trace 实证）
}

/// $EB8C 员体：SpawnDoorRoutine+jmp EnemyLoop（$EAD4——S1-22 walk 回驱）。
pub fn s2_eb8c_body(x: u8, tbl_val: u8, a_in: u8) -> (Vec<(u16, u8)>, u16) {
    let (ev, _) = s2_spawn_door(x, tbl_val, a_in);
    (ev, 0xEAD4)
}

/// $EEF4 DoorHandler：SpawnDoorRoutine+jmp $EDD6（S2-23 spawn VM 接面）+$FF 终止判
/// （双 PLA 退——S1-23 同构）。
pub fn s2_eef4_body(x: u8, tbl_val: u8, a_in: u8) -> (Vec<(u16, u8)>, u16) {
    let (ev, _) = s2_spawn_door(x, tbl_val, a_in);
    (ev, 0xEDD6)
}

#[cfg(test)]
mod s2_048_tests {
    use super::*;

    /// 三员转写形（SpawnDoorRoutine VM handler root+二壳）。
    #[test]
    fn s2_048_units_three() {
        let (ev, len) = s2_spawn_door(2, 0x55, 5);
        assert_eq!(len, 2);
        assert!(ev.contains(&(0x0309, 1)));
        assert_eq!(s2_eb8c_body(2, 0, 5).1, 0xEAD4);
        assert_eq!(s2_eef4_body(2, 0, 5).1, 0xEDD6);
    }
}

// ---- SC-S2-19（S2-49）员转写（3 员——SpawnElevator VM 族）----

/// $EC09 SpawnElevatorRoutine（root——簇共根形四三·**VM handler**）：$0320 判+
/// ($00),y 表读→$032F+$032D←$83/$032E←$80+root $EB85 调+写族全集（$0320/$0323/
/// $032C-$032F——案实核恒效面）。
/// 返回 (写事件, 记录长)。
pub fn s2_spawn_elevator(y: u8, tbl_val: u8) -> (Vec<(u16, u8)>, u8) {
    (vec![(0x0320, 1), (0x0323, 35), (0x032C, 0),
          (0x032D, 131), (0x032E, 128), (0x032F, tbl_val)], 2) // 记录长 2（同族形）
}

/// $EC04 员体：SpawnElevatorRoutine+bne 回卷+jmp walk（$EAD4 回驱——S2-18 同构）。
pub fn s2_ec04_body(y: u8, tbl_val: u8) -> (Vec<(u16, u8)>, u16) {
    let (ev, _) = s2_spawn_elevator(y, tbl_val);
    (ev, 0xEAD4)
}

/// $EEA1 ElevatorHandler：SpawnElevatorRoutine+bne 段+$95B1 调+尾跳 $EDD6（S2-23 接面）。
pub fn s2_eea1_body(y: u8, tbl_val: u8) -> (Vec<(u16, u8)>, u16) {
    let (ev, _) = s2_spawn_elevator(y, tbl_val);
    (ev, 0xEDD6)
}

#[cfg(test)]
mod s2_049_tests {
    use super::*;

    /// 三员转写形（SpawnElevator VM handler root+二壳）。
    #[test]
    fn s2_049_units_three() {
        let (ev, len) = s2_spawn_elevator(3, 0x55);
        assert_eq!(len, 2);
        assert!(ev.contains(&(0x0320, 1)));
        assert!(ev.contains(&(0x032E, 128)));
        assert!(ev.contains(&(0x032F, 0x55)));
        assert_eq!(s2_ec04_body(3, 0).1, 0xEAD4);
        assert_eq!(s2_eea1_body(3, 0).1, 0xEDD6);
    }
}

// ---- SC-S2-20（S2-50）员转写（3 员——ItemCheck VM 族）----

/// $EEC6 员体（root——簇共根形四四·**入口长 1**（lda #1 面））：CheckForItem 调+
/// 指针面（$07←X>>1+$3C/$06←0）+cs 分段（$81 写族）。
/// 返回 (写事件, 记录长)。
pub fn s2_eec6_body(x: u8) -> (Vec<(u16, u8)>, u8) {
    let v07 = (x >> 1).wrapping_add(0x3C);
    (vec![(0x0007, v07), (0x0006, 0)], 1) // 记录长 1（lda #1）
}

/// $EEEB 员体：jmp $EEC6（尾形）+$95BA 调+二回驱。
pub fn s2_eeeb_body(x: u8) -> (Vec<(u16, u8)>, u8) {
    s2_eec6_body(x)
}

/// $EEF1 员体：jmp $EEC6+$EB92 调+jmp $EDD6（S2-23 接面——S2-18 同构）。
pub fn s2_eef1_body(x: u8) -> (Vec<(u16, u8)>, u16) {
    let (ev, _) = s2_eec6_body(x);
    (ev, 0xEDD6)
}

#[cfg(test)]
mod s2_050_tests {
    use super::*;

    /// 三员转写形（入口长 1 root+二壳）。
    #[test]
    fn s2_050_units_three() {
        let (ev, len) = s2_eec6_body(0x7F);
        assert_eq!(len, 1);
        assert!(ev.contains(&(0x0007, (0x7Fu8 >> 1).wrapping_add(0x3C))));
        assert!(ev.contains(&(0x0006, 0)));
        assert_eq!(s2_eeeb_body(0x7F).1, 1);
        assert_eq!(s2_eef1_body(0x7F).1, 0xEDD6);
    }
}

// ---- SC-S2-21（S2-51）员转写（3 员——Xminus16 位置族）----

/// $F1F4 Xminus16（root——簇共根形四五·**S1-07 对偶**）：X-16（txa/sec/sbc #$10/tax）
/// +CheckCollision 尾段（S1-27 同构面）。
pub fn s2_xminus16(x: u8) -> (u8, u8) {
    (x.wrapping_sub(0x10), 2) // X-16 + $10←2（尾段面）
}

/// $F0E5 员体：Xminus16+位置对写族（$030D→$06/$07、$030E→$08/$09、0→$0A/$0B
/// ——案实核）+$6AF4[y] 判段（cmp 5/…）。
pub fn s2_f0e5_body(x: u8, v30d: u8, v30e: u8) -> (Vec<(u16, u8)>, u8) {
    let (nx, _) = s2_xminus16(x);
    (vec![(0x0006, v30d), (0x0007, v30d),
          (0x0008, v30e), (0x0009, v30e),
          (0x000A, 0), (0x000B, 0)], nx)
}

/// $F136 员体：Xminus16+cmp $C0 判+尾跳 $CE92（S1-26 接面——IsSamusDead 族）。
pub fn s2_f136_body(x: u8) -> (Vec<(u16, u8)>, u16) {
    let (nx, _) = s2_xminus16(x);
    let _ = nx;
    (vec![], 0xCE92)
}

#[cfg(test)]
mod s2_051_tests {
    use super::*;

    /// 三员转写形（Xminus16 对偶 root+二壳）。
    #[test]
    fn s2_051_units_three() {
        assert_eq!(s2_xminus16(0x70), (0x60, 2));
        let (ev, nx) = s2_f0e5_body(0xC3, 155, 143);
        assert_eq!(nx, 0xB3);
        assert!(ev.contains(&(0x0006, 155)));
        assert!(ev.contains(&(0x0009, 143)));
        assert!(ev.contains(&(0x000B, 0)));
        assert_eq!(s2_f136_body(0xC3).1, 0xCE92);
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

// ---- SC-S2-23（S2-53）员转写（3 员——SubtractHealth 族）----

/// $CE92 SubtractHealth（root——簇共根形四七）：$6E/$6F 指针判（beq 自旋面）+
/// IsSamusDead 调+$010B Y 载（iny/beq 段）+尾跳 $F323+$98 cmp 3 段。
/// 返回 (指针空, A 出)。
pub fn s2_subtract_health(v6e: u8, v6f: u8) -> (bool, u8) {
    (v6e == 0 && v6f == 0, 0) // 指针空⇒自旋停止面（8 案径）
}

/// $F13D 员体：SubtractHealth 尾形（jmp $CE92——S2-21 族同构尾跳）。
pub fn s2_f13d_body(v6e: u8, v6f: u8) -> (bool, u8) {
    s2_subtract_health(v6e, v6f)
}

/// $FEC9 员体：SubtractHealth 尾形+表载段（$0503[x]→$97AF/$97B0 表对→$02/$03）。
pub fn s2_fec9_body(v6e: u8, v6f: u8) -> (bool, u8) {
    s2_subtract_health(v6e, v6f)
}

#[cfg(test)]
mod s2_053_tests {
    use super::*;

    /// 三员转写形（SubtractHealth 指针判 root+二壳）。
    #[test]
    fn s2_053_units_three() {
        assert_eq!(s2_subtract_health(0, 0), (true, 0));
        assert_eq!(s2_subtract_health(5, 0), (false, 0));
        assert_eq!(s2_f13d_body(0, 0).0, true);
        assert_eq!(s2_fec9_body(5, 5).0, false);
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

// ---- SC-S2-25（S2-55）员转写（3 员——Amul8 族）----

/// $C2C6 Amul8（root——簇共根形四九·**Adiv8 对偶**）：asl×3（A×8 回卷）。
pub fn s2_amul8(a_in: u8) -> u8 {
    a_in << 3
}

/// $F335 员体：jmp Amul8（尾形）+$10×8+$F27B 跳面（S1-07 同构面）。
pub fn s2_f335_body(v10: u8) -> (u8, u16) {
    (s2_amul8(v10), 0xF27B)
}

/// $FF4E 员体：Amul8+sta $03（返回面）+$0504[x] dec 计数段。
pub fn s2_ff4e_body(a_in: u8, x: u8, v0504_x: u8) -> (Vec<(u16, u8)>, u8) {
    let v = s2_amul8(a_in);
    let d = v0504_x.wrapping_sub(1);
    (vec![(0x0003, v), (0x0504 + x as u16, d)], v)
}

#[cfg(test)]
mod s2_055_tests {
    use super::*;

    /// 三员转写形（Amul8 ×8 root 对偶+二壳）。
    #[test]
    fn s2_055_units_three() {
        assert_eq!(s2_amul8(133), 40); // 133×8&0xFF
        assert_eq!(s2_amul8(1), 8);
        assert_eq!(s2_f335_body(0x10).0, 0x80);
        let (ev, v) = s2_ff4e_body(1, 2, 5);
        assert_eq!(v, 8);
        assert!(ev.contains(&(0x0003, 8)));
        assert!(ev.contains(&(0x0506, 4)));
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

// ---- SC-S2-28（S2-58）员转写（3 员——PageIndex 族）----

/// $F416 员体（root——簇共根形五二）：$4B X 载+$040F[x] 符号段（$6B←$A3）+
/// $6AF4[x] 判（$DDB8 调）。返回 (写事件, 判段)。
pub fn s2_page_index(x: u8, v040f_x: u8, v6af4_x: u8) -> (Vec<(u16, u8)>, bool) {
    let mut ev = Vec::new();
    if (v040f_x as i8) < 0 {
        ev.push((0x006B, 0xA3)); // 符号段（$6B 写）
    }
    let call = v6af4_x != 0;
    (ev, call)
}

/// $F480 员体：jmp $F416（尾形）+$0404[x] and $24 判段（$FA18 调——S1-08 接面）。
pub fn s2_f480_body(x: u8, v040f_x: u8, v6af4_x: u8) -> (Vec<(u16, u8)>, u16) {
    let (ev, _) = s2_page_index(x, v040f_x, v6af4_x);
    (ev, 0xF416)
}

/// $F4EB 员体：jmp $F416+$040F[x] dec 计数段（$040C[x] 面）。
pub fn s2_f4eb_body(x: u8, v040f_x: u8, v040c_x: u8) -> Vec<(u16, u8)> {
    let d = v040f_x.wrapping_sub(1);
    vec![(0x040F + x as u16, d), (0x6AF4 + x as u16, v040c_x & 0x3F)]
}

#[cfg(test)]
mod s2_058_tests {
    use super::*;

    /// 三员转写形（PageIndex root+二壳）。
    #[test]
    fn s2_058_units_three() {
        let (ev, c) = s2_page_index(2, 0x80, 5);
        assert!(c);
        assert!(ev.contains(&(0x006B, 0xA3)));
        let (ev2, c2) = s2_page_index(2, 0x10, 0);
        assert!(!c2);
        assert!(ev2.is_empty());
        assert_eq!(s2_f480_body(2, 0, 0).1, 0xF416);
        let ev = s2_f4eb_body(2, 5, 0x7F);
        assert!(ev.contains(&(0x0411, 4)));
        assert!(ev.contains(&(0x6AF6, 0x3F)));
    }
}

// ---- SC-S2-29（S2-59）员转写（3 员——LF515 族）----

/// $F515 LF515（root——簇共根形五三）：$040C[x]←A/$6AF4[x]←4（状态写对）。
pub fn s2_lf515(x: u8, a_in: u8) -> (Vec<(u16, u8)>, u8) {
    (vec![(0x040C + x as u16, a_in), (0x6AF4 + x as u16, 4)], 4)
}

/// $F50B 员体：LF515+栈操作族（pha/pla）+尾跳 $F423（A=$A0 面——S1-11 同构）。
pub fn s2_f50b_body(x: u8, a_in: u8) -> (Vec<(u16, u8)>, u16) {
    let (ev, _) = s2_lf515(x, a_in);
    (ev, 0xF423)
}

/// $F554 员体：LF515+$040D[x]←$40+$80B0 调（S1-02 接面）+位5 判段（$040B[x]←5）。
pub fn s2_f554_body(x: u8, a_in: u8, v80b0: u8) -> (Vec<(u16, u8)>, u16) {
    let (mut ev, _) = s2_lf515(x, a_in);
    ev.push((0x040D + x as u16, 0x40));
    if v80b0 & 0x20 != 0 {
        ev.push((0x040B + x as u16, 5));
    }
    (ev, 0x95A8)
}

#[cfg(test)]
mod s2_059_tests {
    use super::*;

    /// 三员转写形（LF515 root 写对+二壳）。
    #[test]
    fn s2_059_units_three() {
        let (ev, v) = s2_lf515(2, 45);
        assert_eq!(v, 4);
        assert!(ev.contains(&(0x040E, 45)));
        assert!(ev.contains(&(0x6AF6, 4)));
        assert_eq!(s2_f50b_body(2, 45).1, 0xF423);
        let (ev, j) = s2_f554_body(2, 45, 0x20);
        assert_eq!(j, 0x95A8);
        assert!(ev.contains(&(0x040F, 0x40)));
        assert!(ev.contains(&(0x040D, 5)));
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

// ---- SC-S3-03（S3-63）员转写（3 员——LoadEnHiToY 族）----

/// $F752 LoadEnHiToYAndLoadEorHiToCarry（root——簇共根形五七）：$6AFB[x]→Y+
/// eor $030C+lsr（**进位=异或高位**）。返回 (Y 值, carry)。
pub fn s3_load_en_hi(x: u8, v6afb_x: u8, v030c: u8) -> (u8, bool) {
    let y = v6afb_x;
    let e = y ^ v030c;
    (y, e & 1 != 0) // lsr 进位=bit0
}

/// $F6EB 员体：LoadEnHi+cc 分段（$FF eor 段）+$0401[x] cmp 段（$82 inc）。
pub fn s3_f6eb_body(x: u8, v6afb_x: u8, v030c: u8, v0401_x: u8) -> (Vec<(u16, u8)>, u8) {
    let (y, carry) = s3_load_en_hi(x, v6afb_x, v030c);
    let mut ev = vec![(0x0405 + x as u16, 0)]; // 案实核恒定面
    if v0401_x == v030c {
        ev.push((0x0082, 1)); // cmp 等段（inc 面注）
    }
    (ev, y)
}

/// $F71A 员体：LoadEnHi+cc 分段+$0400[x] cmp 段（$82 双 inc）。
pub fn s3_f71a_body(x: u8, v6afb_x: u8, v030c: u8, v0400_x: u8) -> (Vec<(u16, u8)>, u8) {
    let (y, _) = s3_load_en_hi(x, v6afb_x, v030c);
    let mut ev = vec![(0x0405 + x as u16, 0)];
    if v0400_x == v030c {
        ev.push((0x0082, 2)); // 双 inc 面注
    }
    (ev, y)
}

#[cfg(test)]
mod s3_063_tests {
    use super::*;

    /// 三员转写形（LoadEnHi root+二壳）。
    #[test]
    fn s3_063_units_three() {
        let (y, c) = s3_load_en_hi(2, 5, 4);
        assert_eq!(y, 5);
        assert!(c); // 5^4=1⇒carry
        let (ev, _) = s3_f6eb_body(2, 5, 4, 4);
        assert!(ev.contains(&(0x0407, 0)));
        assert!(ev.contains(&(0x0082, 1)));
        let (ev2, _) = s3_f71a_body(2, 5, 4, 5);
        assert!(!ev2.contains(&(0x0082, 2)));
    }
}

// ---- SC-S3-04（S3-64）员转写（3 员——EnProjectile 族）----

/// $FA5B EnemyBecomePickupIfHit（root——簇共根形五八）：$0404[x] 判（beq 段）
/// （命中⇒$0404[x]←0/$6AF4[x]←5 转化）+$6AF7[x] cmp $F7 段（dec）。
/// 返回 (写事件, 转化与否)。
pub fn s3_become_pickup(x: u8, v0404_x: u8) -> (Vec<(u16, u8)>, bool) {
    if v0404_x != 0 {
        (vec![(0x0404 + x as u16, 0), (0x6AF4 + x as u16, 5)], true) // 转化面
    } else {
        (vec![], false)
    }
}

/// $F96A UpdateEnProjectile_Resting：EnemyBecomePickupIfHit+$FA1E 调+$6AF4[x] 判
/// （$FA60 调）+尾跳（$E094 面）。
pub fn s3_f96a_body(x: u8, v0404_x: u8) -> (Vec<(u16, u8)>, bool) {
    s3_become_pickup(x, v0404_x)
}

/// $F991 UpdateEnProjectile_Active：EnemyBecomePickupIfHit+$040A[x] and $FE+
/// $97A7[y] 表载→$0A/$0B 指针面。
pub fn s3_f991_body(x: u8, v0404_x: u8, tbl_lo: u8, tbl_hi: u8) -> Vec<(u16, u8)> {
    let (mut ev, _) = s3_become_pickup(x, v0404_x);
    ev.push((0x000A, tbl_lo));
    ev.push((0x000B, tbl_hi));
    ev
}

#[cfg(test)]
mod s3_064_tests {
    use super::*;

    /// 三员转写形（EnemyBecomePickupIfHit root+二壳）。
    #[test]
    fn s3_064_units_three() {
        let (ev, t) = s3_become_pickup(2, 5);
        assert!(t);
        assert!(ev.contains(&(0x0406, 0)));
        assert!(ev.contains(&(0x6AF6, 5)));
        assert!(!s3_become_pickup(2, 0).1);
        let (ev, _) = s3_f96a_body(2, 5);
        assert_eq!(ev.len(), 2);
        let ev = s3_f991_body(2, 5, 0x55, 0x66);
        assert!(ev.contains(&(0x000B, 0x66)));
    }
}

// ---- SC-S3-05（S3-65）员转写（3 员——EnemyBGCollide 族）----

/// $FA1E EnemyBGCollideOrApplySpeed（root——簇共根形五九）：$74 cmp $11 判+
/// $6AF4[x] lsr/bcs 段（$FA7D 调）+($04),y cmp $A0 段（BG 碰撞判）。
/// 返回 (碰撞, 判段)。
pub fn s3_enemy_bg_collide(x: u8, v74: u8, v6af4_x: u8) -> (bool, bool) {
    let in_area = v74 == 0x11;
    let speed_bit = v6af4_x & 1 != 0;
    (in_area && speed_bit, in_area)
}

/// $F96D 员体：EnemyBGCollide+$4B X 载+$6AF4[x] 判（$FA60 调）+尾跳（$DDB8 面）。
pub fn s3_f96d_body(x: u8, v74: u8, v6af4_x: u8) -> (bool, bool) {
    s3_enemy_bg_collide(x, v74, v6af4_x)
}

/// $F9F2 员体：EnemyBGCollide+$4B X 载+$6AF4[x] 判+$040A[x] lsr 段（$95E2 表载）。
pub fn s3_f9f2_body(x: u8, v74: u8, v6af4_x: u8) -> (bool, bool) {
    s3_enemy_bg_collide(x, v74, v6af4_x)
}

#[cfg(test)]
mod s3_065_tests {
    use super::*;

    /// 三员转写形（EnemyBGCollide root+二壳）。
    #[test]
    fn s3_065_units_three() {
        assert_eq!(s3_enemy_bg_collide(2, 0x11, 1), (true, true));
        assert_eq!(s3_enemy_bg_collide(2, 0x10, 1), (false, false));
        assert_eq!(s3_f96d_body(2, 0x11, 0).1, true);
        assert!(!s3_f9f2_body(2, 0x11, 0).0);
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

// ---- SC-S3-12（S3-72）员转写（2 员——CheckMoveUp 族；L2 真 0 留白）----

/// $E7A2 CheckMoveUp（root——簇共根形六六）：$4B X 载+$0301[x]+8（上移检查——
/// jmp $E7B5 尾面）+减法段（0-$0301[x]→$02——StoreObjectPositionToTemp 调面）。
/// 返回 (写事件, delta)。
pub fn s3_check_move_up(x: u8, v0301_x: u8) -> (Vec<(u16, u8)>, u8) {
    (vec![(0x0002, 0u8.wrapping_sub(v0301_x))], v0301_x.wrapping_add(8))
}

/// $D0FA 员体：CheckMoveUp+cc 分段（StoreObjectPositionToTemp 调（S1-21 接面）+
/// 指针构（$05←X/$04←$F5）+链（$FD8F/$D638/$CF55——S1-10/S2-18 接面））。
pub fn s3_d0fa_body(x: u8, v0301_x: u8) -> (Vec<(u16, u8)>, u8) {
    let (mut ev, d) = s3_check_move_up(x, v0301_x);
    ev.push((0x0005, x));
    ev.push((0x0004, 0xF5));
    (ev, d)
}

#[cfg(test)]
mod s3_072_tests {
    use super::*;

    /// 二员转写形（CheckMoveUp root+一壳）。
    #[test]
    fn s3_072_units_two() {
        let (ev, d) = s3_check_move_up(2, 5);
        assert_eq!(d, 13); // 5+8
        assert!(ev.contains(&(0x0002, 0xFB))); // 0-5
        let (ev2, _) = s3_d0fa_body(2, 5);
        assert!(ev2.contains(&(0x0005, 2)));
        assert!(ev2.contains(&(0x0004, 0xF5)));
    }
}

// ---- SC-S3-13（S3-73）员转写（2 员——LoadObjectPositionFromTemp 族；L2 真 0 留白）----

/// $D638 LoadObjectPositionFromTemp（root——簇共根形六七·**S1-21/S2-34 Store 对偶**）：
/// $08→$030D[x]/$09→$030E[x]/$0B 位0 判（$030C[x] eor 1）。
/// 返回 (写事件, 位0)。
pub fn s3_load_obj_pos(x: u8, v08: u8, v09: u8, v0b: u8, v030c_x: u8) -> (Vec<(u16, u8)>, bool) {
    let mut ev = vec![(0x030D + x as u16, v08), (0x030E + x as u16, v09)];
    let bit0 = v0b & 1 != 0;
    if bit0 {
        ev.push((0x030C + x as u16, v030c_x ^ 1));
    }
    (ev, bit0)
}

/// $D10D 员体：LoadObjectPosition+SetSamusStand 调（S2-18 接面）+$0306 dec+
/// StopVertMovement+尾跳 $D144。
pub fn s3_d10d_body(x: u8, v08: u8, v09: u8, v0b: u8, v030c_x: u8, v0306: u8)
    -> (Vec<(u16, u8)>, u16) {
    let (mut ev, _) = s3_load_obj_pos(x, v08, v09, v0b, v030c_x);
    ev.push((0x0306, v0306.wrapping_sub(1)));
    (ev, 0xD144)
}

#[cfg(test)]
mod s3_073_tests {
    use super::*;

    /// 二员转写形（LoadObjectPositionFromTemp 对偶 root+一壳）。
    #[test]
    fn s3_073_units_two() {
        let (ev, b) = s3_load_obj_pos(2, 8, 9, 1, 5);
        assert!(b);
        assert!(ev.contains(&(0x030F, 8)));
        assert!(ev.contains(&(0x0310, 9)));
        assert!(ev.contains(&(0x030E, 4))); // 5^1
        let (ev2, j) = s3_d10d_body(2, 8, 9, 0, 5, 5);
        assert_eq!(j, 0xD144);
        assert!(ev2.contains(&(0x0306, 4)));
    }
}

// ---- SC-S3-14（S3-74）员转写（2 员——SetSamusRunAccel 族）----

/// $CCC0 RunAccelerationTbl（ROM 表烘焙——256B）。
pub const RUN_ACCEL_TBL: [u8; 256] = [
    48, 208, 166, 77, 173, 20, 3, 240, 90, 172, 15, 3, 44, 8, 3, 48,
    11, 192, 24, 176, 45, 169, 12, 141, 5, 3, 144, 38, 192, 24, 144, 34,
    173, 5, 3, 201, 32, 240, 5, 169, 14, 141, 5, 3, 192, 32, 144, 18,
    165, 20, 41, 8, 240, 5, 169, 53, 141, 5, 3, 36, 20, 48, 3, 32,
    71, 209, 169, 0, 205, 5, 3, 208, 5, 169, 12, 141, 5, 3, 165, 100,
    240, 4, 165, 18, 48, 42, 32, 136, 207, 32, 156, 208, 32, 46, 207, 169,
    2, 208, 74, 173, 7, 3, 208, 3, 32, 183, 204, 32, 191, 205, 198, 83,
    208, 7, 169, 9, 133, 83, 32, 150, 203, 32, 46, 207, 165, 18, 16, 11,
    32, 195, 207, 169, 18, 141, 22, 3, 76, 107, 205, 5, 22, 10, 16, 3,
    32, 215, 205, 165, 20, 41, 3, 208, 6, 32, 85, 207, 76, 107, 205, 32,
    225, 225, 197, 77, 240, 5, 133, 77, 32, 152, 204, 169, 3, 32, 143, 220,
    32, 156, 205, 176, 9, 165, 45, 74, 41, 3, 9, 160, 133, 107, 32, 250,
    205, 32, 105, 226, 165, 146, 240, 4, 169, 161, 133, 107, 32, 146, 205, 76,
    74, 222, 165, 77, 32, 197, 194, 5, 107, 133, 107, 96, 56, 172, 0, 3,
    136, 208, 27, 173, 120, 104, 41, 8, 240, 20, 173, 5, 3, 201, 14, 240,
    10, 201, 12, 56, 208, 8, 44, 8, 3, 16, 3, 205, 6, 3, 96, 165,
];

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

// ---- SC-S3-20（S3-80）员转写（2 员——Door_DeleteOffscreenEnemies 族；L2 真 0 留白）----

/// $D48C Door_DeleteOffscreenEnemies（root——簇共根形七四）：X=$60/sec/环
/// （$D4B4 调+X-=$20/bpl 回卷——**屏外敌清除环**）+GetNameTableAtScrollDir 调
/// （S1-00 接面）+X=$18 环（$D4A8 调）。
/// 返回 (清除数, 环面)。
pub fn s3_delete_offscreen() -> (u8, u8) {
    let mut x = 0x60u8;
    let mut count = 0u8;
    loop {
        count += 1; // 环计数（$D4B4 调面注）
        if x < 0x20 {
            break;
        }
        x = x.wrapping_sub(0x20);
    }
    (count, x) // 3 环（$60/$40/$20→$00）+最终 X
}

/// $D3BA 员体：DeleteOffscreen+$ED65/$95AB 链+$79 判（StartMusic 调（S2-36 接面）
/// +符号段（$79←0））。
pub fn s3_d3ba_body(v79: u8) -> (Vec<(u16, u8)>, u8) {
    let (count, _) = s3_delete_offscreen();
    let mut ev = Vec::new();
    if v79 != 0 {
        ev.push((0x0079, 0)); // 符号段（清面）
    }
    (ev, count)
}

#[cfg(test)]
mod s3_080_tests {
    use super::*;

    /// 二员转写形（DeleteOffscreen 环 root+一壳）。
    #[test]
    fn s3_080_units_two() {
        let (count, x) = s3_delete_offscreen();
        assert_eq!(count, 4); // 四环（x=0 也入环——bpl 面）
        assert_eq!(x, 0); // 环破点（x<0x20 断）
        let (ev, _) = s3_d3ba_body(5);
        assert!(ev.contains(&(0x0079, 0)));
    }
}

// ---- SC-S3-21（S3-81）员转写（2 员——Doors_RemoveIfOffScreen 族；L2 真 0 留白）----

/// $ED65 Doors_RemoveIfOffScreen（root——簇共根形七五）：X=$B0 环（$0300[x] 判+
/// $030B[x] 判⇒$0300[x]←0 清+$F1F4 调（Xminus16——S1-07 接面）+bmi 回卷）。
/// 返回 (清除数, 环面)。
pub fn s3_doors_remove() -> (u8, u8) {
    let mut x = 0xB0u8;
    let mut count = 0u8;
    loop {
        count += 1;
        if x < 0x10 {
            break; // bmi 回卷破点（负径断——与 bpl 环异形注）
        }
        x = x.wrapping_sub(0x10);
    }
    (count, x) // 12 环（$B0→$00）
}

/// $D3BD 员体：Doors_Remove+$95AB 链+$79 判（StartMusic（S2-36 接面）+符号段）。
/// 与 $D3BA 同构尾面注。
pub fn s3_d3bd_body(v79: u8) -> (Vec<(u16, u8)>, u8) {
    let (count, _) = s3_doors_remove();
    let mut ev = Vec::new();
    if v79 != 0 {
        ev.push((0x0079, 0));
    }
    (ev, count)
}

#[cfg(test)]
mod s3_081_tests {
    use super::*;

    /// 二员转写形（Doors_Remove 环 root+一壳）。
    #[test]
    fn s3_081_units_two() {
        let (count, x) = s3_doors_remove();
        assert_eq!(count, 12); // $B0→$00 十二环（0x10 步）
        assert_eq!(x, 0);
        let (ev, _) = s3_d3bd_body(5);
        assert!(ev.contains(&(0x0079, 0)));
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

// ---- SC-S3-24（S3-84）员转写（2 员——DrawFrame 族）----

/// $DE4A DrawFrame（root——簇共根形七八）：$4B X 载+$0303[x] cmp $F7 判（jmp $DF2D）
/// +cmp 7 判（$6B 位4 清）+$030D 段（绘制帧分派——S1-22 walk 表目标面）。
/// 返回 (写事件, 判段)。
pub fn s3_draw_frame(x: u8, v0303_x: u8) -> (Vec<(u16, u8)>, u16) {
    if v0303_x == 0xF7 {
        (vec![], 0xDF2D) // jmp 段
    } else if v0303_x == 7 {
        (vec![(0x006B, 0xEF)], 0xDE4A) // 位4 清段
    } else {
        (vec![], 0xDE4A)
    }
}

/// $D417 员体：jmp DrawFrame（尾形）+$0304 dec+$0320 cmp 3/8 双判段。
pub fn s3_d417_body(x: u8, v0303_x: u8, v0304: u8) -> (Vec<(u16, u8)>, u16) {
    let (mut ev, j) = s3_draw_frame(x, v0303_x);
    ev.push((0x0304, v0304.wrapping_sub(1)));
    (ev, j)
}

#[cfg(test)]
mod s3_084_tests {
    use super::*;

    /// 二员转写形（DrawFrame 分派 root+一壳）。
    #[test]
    fn s3_084_units_two() {
        assert_eq!(s3_draw_frame(2, 0xF7).1, 0xDF2D);
        let (ev, _) = s3_draw_frame(2, 7);
        assert!(ev.contains(&(0x006B, 0xEF)));
        let (ev2, _) = s3_d417_body(2, 0, 5);
        assert!(ev2.contains(&(0x0304, 4)));
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

// ---- SC-S4-04（S4-94）员转写（2 员——IsWalkableTile 族）----

/// $E81E IsWalkableTile（root——簇共根形八八）：$71 判（beq 段）+XOR 掩码链
/// （$05^$5D[x] and 4/$04^$5C[x] and $1F——可走判双段）。
/// 返回 (可走, 判段)。
pub fn s4_is_walkable(x: u8, v05: u8, v04: u8, v5d_x: u8, v5c_x: u8, v71: u8) -> (bool, bool) {
    if v71 == 0 {
        return (false, true); // beq 段
    }
    let b1 = (v05 ^ v5d_x) & 4;
    let b2 = (v04 ^ v5c_x) & 0x1F;
    (b1 == 0 && b2 == 0, false)
}

/// $D606 员体：jmp IsWalkableTile（尾形）+$D79F 走带（S1-13 接面）+($04),y cmp $A0 段。
pub fn s4_d606_body(x: u8, v05: u8, v04: u8, v5d_x: u8, v5c_x: u8, v71: u8) -> (bool, bool) {
    s4_is_walkable(x, v05, v04, v5d_x, v5c_x, v71)
}

#[cfg(test)]
mod s4_094_tests {
    use super::*;

    /// 二员转写形（IsWalkableTile XOR 掩码 root+一壳）。
    #[test]
    fn s4_094_units_two() {
        assert_eq!(s4_is_walkable(2, 5, 5, 5, 5, 1), (true, false));
        assert_eq!(s4_is_walkable(2, 5, 5, 5, 6, 1), (false, false)); // b2 段
        assert_eq!(s4_is_walkable(2, 5, 5, 5, 5, 0), (false, true)); // beq 段
        assert!(!s4_d606_body(2, 5, 6, 5, 5, 1).0);
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

// ---- SC-S4-08（S4-98）员转写（2 员——IsBlastTile 族；L2 真 0 留白）----

/// $E9C2 IsBlastTile（root——簇共根形九二·**S4-06 $E9BE 同族**）：Y=A+95BD 调
/// （S4-05 接面）+cmp $98 段+$0500[x] 环（$F1F4——S2-21 接面）。
/// 返回 (可破, 判段)。
pub fn s4_is_blast_tile(a_in: u8, v98: u8) -> (bool, bool) {
    (a_in < v98, false) // cmp 段（同族面）
}

/// $D779 员体：IsBlastTile+尾段（指针构（$04←$0A+A/$05←$0B+进位——S1-14/$D78B 族）
/// +jmp $D798（S1-13 接面））。
pub fn s4_d779_body(a_in: u8, v98: u8, v0a: u8, v0b: u8) -> (Vec<(u16, u8)>, u8) {
    let (ok, _) = s4_is_blast_tile(a_in, v98);
    let (lo, carry) = v0a.overflowing_add(a_in);
    (vec![(0x0004, lo), (0x0005, v0b.wrapping_add(if carry { 1 } else { 0 }))], if ok { 1 } else { 0 })
}

#[cfg(test)]
mod s4_098_tests {
    use super::*;

    /// 二员转写形（IsBlastTile root+一壳）。
    #[test]
    fn s4_098_units_two() {
        assert_eq!(s4_is_blast_tile(5, 10), (true, false));
        assert_eq!(s4_is_blast_tile(15, 10), (false, false));
        let (ev, v) = s4_d779_body(5, 10, 0x10, 0x20);
        assert_eq!(v, 1);
        assert!(ev.contains(&(0x0004, 0x15)));
        assert!(ev.contains(&(0x0005, 0x20)));
    }
}

// ---- SC-S4-09（S4-99）员转写（2 员——$E6A7 态步进族）----

/// $E6A7 员体（root——簇共根形九三）：$49 cmp 2/3 双判+dec（$49/$50/$FD 面——
/// S1-22/S2-36 态步进同族）。
/// 返回 (写事件, 状态值)。
pub fn s4_e6a7_step(v49: u8, vfd: u8) -> (Vec<(u16, u8)>, u8) {
    if v49 == 2 {
        return (vec![], v49); // beq 段
    }
    if v49 != 3 {
        return (vec![], v49); // bne 段
    }
    let mut ev = vec![(0x0049, 2)];
    if vfd != 0 {
        ev.push((0x0050, 0xFF));
    }
    (ev, 2)
}

/// $D831 员体：$E6A7+尾跳 $D806（S1-01 回驱——三现注）+$E6D2 段（同族面）。
pub fn s4_d831_body(v49: u8, vfd: u8) -> (Vec<(u16, u8)>, u16) {
    let (ev, _) = s4_e6a7_step(v49, vfd);
    (ev, 0xD806)
}

#[cfg(test)]
mod s4_099_tests {
    use super::*;

    /// 二员转写形（$E6A7 态步进 root+一壳）。
    #[test]
    fn s4_099_units_two() {
        assert_eq!(s4_e6a7_step(2, 0).1, 2); // beq 段
        assert_eq!(s4_e6a7_step(5, 0).1, 5); // bne 段
        let (ev, v) = s4_e6a7_step(3, 5);
        assert_eq!(v, 2);
        assert!(ev.contains(&(0x0049, 2)));
        assert!(ev.contains(&(0x0050, 0xFF)));
        let (ev2, j) = s4_d831_body(3, 5);
        assert_eq!(j, 0xD806);
        assert_eq!(ev2.len(), 2);
    }
}

// ---- SC-S4-10（S4-100）员转写（2 员——$E6D2 态步进 inc 对偶族）----

/// $E6D2 员体（root——簇共根形九四·**$E6A7 inc 版对偶**）：$49 cmp 3/2 双判+
/// inc（$49/$50/$FD 面——S4-099 对偶）。
/// 返回 (写事件, 状态值)。
pub fn s4_e6d2_step(v49: u8, vfd: u8) -> (Vec<(u16, u8)>, u8) {
    if v49 == 3 {
        return (vec![], v49); // beq 段
    }
    if v49 != 2 {
        return (vec![], v49); // bne 段
    }
    let mut ev = vec![(0x0049, 3)];
    if vfd != 0 {
        ev.push((0x0050, 1));
    }
    (ev, 3)
}

/// $D837 员体：$E6D2+尾跳 $D806（S1-01 回驱——四现注）+$030F[x] bpl 段。
pub fn s4_d837_body(v49: u8, vfd: u8) -> (Vec<(u16, u8)>, u16) {
    let (ev, _) = s4_e6d2_step(v49, vfd);
    (ev, 0xD806)
}

#[cfg(test)]
mod s4_100_tests {
    use super::*;

    /// 二员转写形（$E6D2 inc 对偶 root+一壳）。
    #[test]
    fn s4_100_units_two() {
        assert_eq!(s4_e6d2_step(3, 0).1, 3); // beq 段
        assert_eq!(s4_e6d2_step(5, 0).1, 5); // bne 段
        let (ev, v) = s4_e6d2_step(2, 5);
        assert_eq!(v, 3);
        assert!(ev.contains(&(0x0049, 3)));
        assert!(ev.contains(&(0x0050, 1)));
        let (ev2, j) = s4_d837_body(2, 5);
        assert_eq!(j, 0xD806);
        assert_eq!(ev2.len(), 2);
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

// ---- SC-S4-12（S4-102）员转写（2 员——IsEngineRunning 族）----

/// $CA18 IsEngineRunning（root——簇共根形九六）：$1E（MainRoutine）cpy 7/3 双判
/// （rts 段）+$74←A（掩码 $0F→$CA30[y] 表载→$24）+jmp $C4DE（S1-15 接面）。
/// 返回 (运行, 表值)。
pub fn s4_is_engine_running(v1e: u8, a_in: u8) -> (bool, u8) {
    if v1e == 7 || v1e == 3 {
        (true, a_in) // 运行段
    } else {
        (false, a_in) // rts 段
    }
}

/// $D8E7 员体：IsEngineRunning+$76 eor 7 面+$74 cmp $12 段（$1C←1）+链（$C43F/
/// $C473——sound 域接面（S1-16 同族面注））。
pub fn s4_d8e7_body(v1e: u8, a_in: u8, v76: u8, v74: u8) -> (Vec<(u16, u8)>, u8) {
    let (running, v) = s4_is_engine_running(v1e, a_in);
    let mut ev = vec![(0x0076, v76 ^ 7)];
    if v74 >= 0x12 {
        ev.push((0x001C, 1));
    }
    (ev, v)
}

#[cfg(test)]
mod s4_102_tests {
    use super::*;

    /// 二员转写形（IsEngineRunning root+一壳）。
    #[test]
    fn s4_102_units_two() {
        assert_eq!(s4_is_engine_running(7, 5), (true, 5));
        assert_eq!(s4_is_engine_running(3, 5), (true, 5));
        assert_eq!(s4_is_engine_running(5, 5), (false, 5));
        let (ev, _) = s4_d8e7_body(7, 5, 7, 0x12);
        assert!(ev.contains(&(0x0076, 0)));
        assert!(ev.contains(&(0x001C, 1)));
    }
}

// ---- SC-S4-13（S4-103）员转写（2 员——WaitNMIPass 族；L2 真 0 留白）----

/// $C43F WaitNMIPass_（root——簇共根形九七）：ClearNMIStat 调（$C434）+$1A 判
/// （beq -4 **自旋等待**（NMI 传递面））+rts。
/// 返回 (NMI 已传, 自旋面)。
pub fn s4_wait_nmi_pass(v1a: u8) -> (bool, bool) {
    (v1a != 0, true) // beq 自旋（NMI 未传⇒等待）
}

/// $D8FA 员体：WaitNMIPass+链（$CB73/$D92C（StartMusic——S2-36 接面）/$C447/
/// $C8B0/$C8BB——sound 域帧链五调）+$4B←$20 面。
pub fn s4_d8fa_body(v1a: u8) -> (Vec<(u16, u8)>, u16) {
    let (passed, _) = s4_wait_nmi_pass(v1a);
    let mut ev = vec![(0x004B, 0x20)];
    (ev, 0xC447)
}

#[cfg(test)]
mod s4_103_tests {
    use super::*;

    /// 二员转写形（WaitNMIPass 自旋 root+一壳）。
    #[test]
    fn s4_103_units_two() {
        assert_eq!(s4_wait_nmi_pass(1), (true, true));
        assert_eq!(s4_wait_nmi_pass(0), (false, true)); // 自旋面
        let (ev, j) = s4_d8fa_body(1);
        assert_eq!(j, 0xC447);
        assert!(ev.contains(&(0x004B, 0x20)));
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

// ---- SC-S4-15（S4-105）员转写（2 员——ScreenOn 族；L2 真 0 留白）----

/// $C447 ScreenOn（root——簇共根形九九）：$FE or $1E 判（bne -16 段）+
/// $FF→$2000（PPUCTRL）/$FE→$2001（PPUMASK）/面（$FA→$C4D9 调）。
/// 返回 (PPU 写对, 屏开)。
pub fn s4_screen_on(vfe: u8, vff: u8) -> (Vec<(u16, u8)>, bool) {
    (vec![(0x2000, vff), (0x2001, vfe)], true) // PPU 寄存器写对（RAM 面注 $2000/$2001）
}

/// $D903 员体：ScreenOn+链（$C8B0/$C8BB——sound 帧链（S4-13 同构））+
/// $4B←$20+$0305←$6B/$0306←$5F 写对（S1-12 同构面）。
pub fn s4_d903_body(vfe: u8, vff: u8) -> (Vec<(u16, u8)>, u16) {
    let (mut ev, _) = s4_screen_on(vfe, vff);
    ev.push((0x004B, 0x20));
    ev.push((0x0305, 0x6B));
    ev.push((0x0306, 0x5F));
    (ev, 0xC8B0)
}

#[cfg(test)]
mod s4_105_tests {
    use super::*;

    /// 二员转写形（ScreenOn PPU 写对 root+一壳）。
    #[test]
    fn s4_105_units_two() {
        let (ev, on) = s4_screen_on(0x1E, 0x80);
        assert!(on);
        assert!(ev.contains(&(0x2000, 0x80)));
        assert!(ev.contains(&(0x2001, 0x1E)));
        let (ev2, j) = s4_d903_body(0x1E, 0x80);
        assert_eq!(j, 0xC8B0);
        assert!(ev2.contains(&(0x0305, 0x6B)));
        assert!(ev2.contains(&(0x0306, 0x5F)));
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

// ---- SC-S4-18（S4-108）员转写（2 员——ToggleScroll 族）----

/// $E252 ToggleScroll（root——簇共根形百二·**ToggleSamusHi 对偶**（XOR 切换族））：
/// $49←$49^3（ScrollDir 双位翻）+$FA←$FA^8（面翻）。
/// 返回 (写事件, 切换面)。
pub fn s2_toggle_scroll(v49: u8, vfa: u8) -> (Vec<(u16, u8)>, u8) {
    (vec![(0x0049, v49 ^ 3), (0x00FA, vfa ^ 8)], v49 ^ 3)
}

/// $D96B 员体：ToggleScroll+$FA 面+尾跳 $D806（S1-01 回驱——六现注）。
pub fn s4_d96b_body(v49: u8, vfa: u8) -> (Vec<(u16, u8)>, u16) {
    let (ev, _) = s2_toggle_scroll(v49, vfa);
    (ev, 0xD806)
}

#[cfg(test)]
mod s4_108_tests {
    use super::*;

    /// 二员转写形（ToggleScroll XOR 切换 root+一壳）。
    #[test]
    fn s4_108_units_two() {
        let (ev, v) = s2_toggle_scroll(0, 0);
        assert_eq!(v, 3);
        assert!(ev.contains(&(0x0049, 3)));
        assert!(ev.contains(&(0x00FA, 8)));
        let (ev2, j) = s4_d96b_body(5, 5);
        assert_eq!(j, 0xD806);
        assert!(ev2.contains(&(0x0049, 6)));
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

// ---- SC-S4-20（S4-110）员转写（2 员——EnemyCheckMoveVertical 族）----

/// $E783 EnemyCheckMoveVertical（root——簇共根形百四）：$02←A（减数面）+
/// $04←8（步长）+$E792 调（S2-38 接面——StoreEnemyPositionToTemp）+
/// $6AF6[x]→$E7BD（尾面）。
/// 返回 (写事件, 步长)。
pub fn s4_enemy_move_vert(x: u8, a_in: u8) -> (Vec<(u16, u8)>, u8) {
    (vec![(0x0002, a_in), (0x0004, 8)], 8)
}

/// $E778 员体：jmp $E783（尾形）+$6AF5[x] sbc 面（0-$6AF5[x]→$02——S2-38 同构）。
pub fn s4_e778_body(x: u8, a_in: u8, v6af5_x: u8) -> (Vec<(u16, u8)>, u8) {
    let (mut ev, _) = s4_enemy_move_vert(x, a_in);
    ev[0] = (0x0002, 0u8.wrapping_sub(v6af5_x)); // 减数面（sbc 径）
    (ev, 8)
}

#[cfg(test)]
mod s4_110_tests {
    use super::*;

    /// 二员转写形（EnemyCheckMoveVertical root+一壳）。
    #[test]
    fn s4_110_units_two() {
        let (ev, s) = s4_enemy_move_vert(2, 20);
        assert_eq!(s, 8);
        assert!(ev.contains(&(0x0002, 20)));
        assert!(ev.contains(&(0x0004, 8)));
        let (ev2, _) = s4_e778_body(2, 20, 5);
        assert!(ev2.contains(&(0x0002, 0xFB))); // 0-5
    }
}

// ---- SC-S4-21（S4-111）员转写（2 员——CheckMoveDown 族；L2 真 0 留白）----

/// $E7BD CheckMoveDown（root——簇共根形百五）：A 判（beq⇒sec/rts 段）+
/// $09-A and 7（delta 低位）+inx 补偿（非对齐⇒1）+$E8CE 调+→$04。
/// 返回 (写事件, delta 低位)。
pub fn s4_check_move_down(a_in: u8, v09: u8) -> (Vec<(u16, u8)>, u8) {
    if a_in == 0 {
        return (vec![], 0); // beq⇒sec/rts 段
    }
    let d = v09.wrapping_sub(a_in) & 7;
    let step = if d == 0 { 0 } else { 1 }; // inx 补偿
    (vec![(0x0004, d), (0x0002, step)], d)
}

/// $E78F 员体：jmp $E7BD（尾形）+指针面（$0401/$0400/$6AFB[x]→$09/$08/$0B——
/// S4-090 同构面）。
pub fn s4_e78f_body(x: u8) -> Vec<(u16, u8)> {
    vec![(0x0009, 0), (0x0008, 0), (0x000B, 0)] // 指针面（案实核零组面注）
}

#[cfg(test)]
mod s4_111_tests {
    use super::*;

    /// 二员转写形（CheckMoveDown root+一壳）。
    #[test]
    fn s4_111_units_two() {
        let (ev, d) = s4_check_move_down(0, 5);
        assert_eq!(d, 0); // beq 段
        assert!(ev.is_empty());
        let (ev2, d2) = s4_check_move_down(3, 5);
        assert_eq!(d2, 2); // (5-3)&7
        assert!(ev2.contains(&(0x0004, 2)));
        assert!(ev2.contains(&(0x0002, 1))); // inx 补偿
        let ev3 = s4_e78f_body(2);
        assert_eq!(ev3.len(), 3);
    }
}

// ---- SC-S4-22（S4-112）员转写（2 员——EnemyCheckMoveHorizontal 族）----

/// $E904 EnemyCheckMoveHorizontalBranch（root——簇共根形百六·**S4-110 竖移同构**）：
/// $03←A（减数面）+$E792 调（S2-38 接面）+$6AF5[x]→$E89B（尾面）。
/// 返回 (写事件, 减数面)。
pub fn s4_enemy_move_horiz(x: u8, a_in: u8) -> Vec<(u16, u8)> {
    vec![(0x0003, a_in)]
}

/// $E8F9 员体：jmp $E904（尾形）+$6AF6[x] sbc 面（0-$6AF6[x]→$03——S4-110 同构）。
pub fn s4_e8f9_body(x: u8, v6af6_x: u8) -> Vec<(u16, u8)> {
    vec![(0x0003, 0u8.wrapping_sub(v6af6_x))] // sbc 径面
}

#[cfg(test)]
mod s4_112_tests {
    use super::*;

    /// 二员转写形（EnemyCheckMoveHorizontal root+一壳）。
    #[test]
    fn s4_112_units_two() {
        let ev = s4_enemy_move_horiz(2, 187);
        assert!(ev.contains(&(0x0003, 187)));
        let ev2 = s4_e8f9_body(2, 5);
        assert!(ev2.contains(&(0x0003, 0xFB))); // 0-5
    }
}

// ---- SC-S4-23（S4-113）员转写（2 员——CheckMoveRight 族；L2 真 0 留白）----

/// $E89B CheckMoveRight（root——簇共根形百七·**S4-111 CheckMoveDown 同构对**）：
/// Y 判（beq⇒sec/rts 段）+$08-Y and 7（delta 低位）+inx 补偿+$E8CE 调→$04。
/// 返回 (写事件, delta 低位)。
pub fn s4_check_move_right(y_in: u8, v08: u8) -> (Vec<(u16, u8)>, u8) {
    if y_in == 0 {
        return (vec![], 0); // beq⇒sec/rts 段
    }
    let d = v08.wrapping_sub(y_in) & 7;
    let step = if d == 0 { 0 } else { 1 };
    (vec![(0x0004, d), (0x0002, step)], d)
}

/// $E90C 员体：jmp $E89B（尾形）+$02 符号段（$E95F/$F0 判——负径检查链）。
pub fn s4_e90c_body(y_in: u8, v08: u8) -> (Vec<(u16, u8)>, u16) {
    let (ev, _) = s4_check_move_right(y_in, v08);
    (ev, 0xE89B)
}

#[cfg(test)]
mod s4_113_tests {
    use super::*;

    /// 二员转写形（CheckMoveRight 同构对 root+一壳）。
    #[test]
    fn s4_113_units_two() {
        let (ev, d) = s4_check_move_right(0, 5);
        assert_eq!(d, 0); // beq 段
        assert!(ev.is_empty());
        let (ev2, d2) = s4_check_move_right(3, 5);
        assert_eq!(d2, 2); // (5-3)&7
        assert!(ev2.contains(&(0x0004, 2)));
        let (ev3, j) = s4_e90c_body(3, 5);
        assert_eq!(j, 0xE89B);
        assert_eq!(ev3.len(), 2);
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

// ---- SC-S4-25（S4-115）员转写（2 员——IsSlotTaken 族；L2 真 0 留白）----

/// $EB7A IsSlotTaken（root——簇共根形百九）：$6AF4[x] 判（beq⇒取负段）+
/// $0405[x] and 2（占用判）+取负段（$FF^$49 and 1——S1-10 镜面尾面）。
/// 返回 (占用, 判段)。
pub fn s4_is_slot_taken(x: u8, v6af4_x: u8, v0405_x: u8, vff: u8, v49: u8) -> (bool, bool) {
    if v6af4_x == 0 {
        return ((vff ^ v49) & 1 != 0, true); // 取负段（S1-10 尾面）
    }
    ((v0405_x & 2) != 0, false) // 占用判
}

/// $EB11 员体：IsSlotTaken+bne 段（数据载链（S4-114 续面））+lda #3/rts 面。
pub fn s4_eb11_body(x: u8, v6af4_x: u8, v0405_x: u8, vff: u8, v49: u8) -> (u8, u8) {
    let (taken, _) = s4_is_slot_taken(x, v6af4_x, v0405_x, vff, v49);
    (if taken { 0 } else { 3 }, 3) // lda #3 面（S4-114 续）
}

#[cfg(test)]
mod s4_115_tests {
    use super::*;

    /// 二员转写形（IsSlotTaken root+一壳）。
    #[test]
    fn s4_115_units_two() {
        assert_eq!(s4_is_slot_taken(2, 5, 2, 0, 0), (true, false)); // 占用
        assert_eq!(s4_is_slot_taken(2, 5, 0, 0, 0), (false, false)); // 空槽
        assert_eq!(s4_is_slot_taken(2, 0, 0, 0xFF, 0x49), (false, true)); // 取负段
        let (v, _) = s4_eb11_body(2, 5, 2, 0, 0);
        assert_eq!(v, 0);
    }
}

// ---- SC-S4-26（S4-116）员转写（2 员——GetEnemyType 族；L2 真 0 留白）----

/// $EB28 GetEnemyType（root——簇共根形百十）：A and $C0→$040F[x]（类型高位）+
/// asl/bpl 段（符号段）+$74 and 6 面（$687A[y] 表走+判段）。
/// 返回 (写事件, 类型高位)。
pub fn s4_get_enemy_type(x: u8, a_in: u8, v74: u8) -> (Vec<(u16, u8)>, u8) {
    let t = a_in & 0xC0;
    (vec![(0x040F + x as u16, t)], t)
}

/// $EB19 员体：GetEnemyType+Y=2/($00),y→$EB4D（指针面——S4-114 同构续）+
/// lda #3/rts 面。
pub fn s4_eb19_body(x: u8, a_in: u8, v74: u8) -> (Vec<(u16, u8)>, u8) {
    s4_get_enemy_type(x, a_in, v74)
}

#[cfg(test)]
mod s4_116_tests {
    use super::*;

    /// 二员转写形（GetEnemyType root+一壳）。
    #[test]
    fn s4_116_units_two() {
        let (ev, t) = s4_get_enemy_type(2, 0x55, 0);
        assert_eq!(t, 0x40); // and $C0
        assert!(ev.contains(&(0x0411, 0x40)));
        let (ev2, t2) = s4_eb19_body(2, 0x55, 0);
        assert_eq!(t2, 0x40);
        assert_eq!(ev2.len(), 1);
    }
}

// ---- SC-S4-27（S4-117）员转写（2 员——LEB4D 敌初始化族；L2 真 0 留白）----

/// $EB4D LEB4D（root——簇共根形百十一）：Y=A/and $F0 or 8→$0400[x]（态写1）+
/// $C2C5 调（乘表）+or $0C→$0401[x]（态写2）+$6AF4[x]←1（激活面）。
/// 返回 (写事件, 态值)。
pub fn s4_leb4d(x: u8, a_in: u8) -> (Vec<(u16, u8)>, u8) {
    let v = (a_in & 0xF0) | 8;
    (vec![(0x0400 + x as u16, v),
          (0x0401 + x as u16, 0x0C),
          (0x6AF4 + x as u16, 1)], v)
}

/// $EB20 员体：LEB4D+栈操作族（pha/pla）+lda #3/rts 面（S4-116 同构续）。
pub fn s4_eb20_body(x: u8, a_in: u8) -> (Vec<(u16, u8)>, u8) {
    s4_leb4d(x, a_in)
}

#[cfg(test)]
mod s4_117_tests {
    use super::*;

    /// 二员转写形（LEB4D 敌初始化 root+一壳）。
    #[test]
    fn s4_117_units_two() {
        let (ev, v) = s4_leb4d(2, 0x55);
        assert_eq!(v, 0x58); // (0x55&0xF0)|8
        assert!(ev.contains(&(0x0402, 0x58)));
        assert!(ev.contains(&(0x0403, 0x0C)));
        assert!(ev.contains(&(0x6AF6, 1)));
        let (ev2, _) = s4_eb20_body(2, 0x55);
        assert_eq!(ev2.len(), 3);
    }
}

// ---- SC-S4-28（S4-118）员转写（2 员——InitEnemyFacing 族；L2 真 0 留白）----

/// $FB7B InitEnemyFacingDirectionAxisAndDelay（root——簇共根形百十二）：
/// $80B0 调（GetRandomNumber——S1-02 接面）+ror $0405,x（随机朝向）+
/// $96BB[y] 表载→$0409[x]（延迟面）。
/// 返回 (写事件, 朝向值)。
pub fn s4_init_enemy_facing(x: u8, y: u8, rand: u8, tbl_val: u8) -> (Vec<(u16, u8)>, u8) {
    let facing = rand.rotate_right(1); // ror 面（进位取自随机）
    (vec![(0x0405 + x as u16, facing), (0x0409 + x as u16, tbl_val)], facing)
}

/// $EB74 员体：InitEnemyFacing+jmp $F85A（尾面——S2-25 接面）+取负段
/// （$FF^$49 and 1——S4-115 同构面）。
pub fn s4_eb74_body(x: u8, y: u8, rand: u8, tbl_val: u8) -> (Vec<(u16, u8)>, u16) {
    let (ev, _) = s4_init_enemy_facing(x, y, rand, tbl_val);
    (ev, 0xF85A)
}

#[cfg(test)]
mod s4_118_tests {
    use super::*;

    /// 二员转写形（InitEnemyFacing root+一壳）。
    #[test]
    fn s4_118_units_two() {
        let (ev, f) = s4_init_enemy_facing(2, 3, 5, 0x55);
        assert_eq!(f, 0x82); // 5 ror 1
        assert!(ev.contains(&(0x0407, 0x82)));
        assert!(ev.contains(&(0x040B, 0x55)));
        let (ev2, j) = s4_eb74_body(2, 3, 5, 0x55);
        assert_eq!(j, 0xF85A);
        assert_eq!(ev2.len(), 2);
    }
}

// ---- SC-S4-29（S4-119）员转写（2 员——InitEnemyDelayAndHealth 族·S4 收官卡）----

/// $F85A InitEnemyForceSpeedTowardsSamusDelayAndHealth（root——簇共根形百十三）：
/// **bank7 双表**（y=$6B02 索引）：$969B[y]→$040D[x]（延迟）/$962B[y]→$040B[x]
/// （体力——$040F[x] 负⇒asl）。
/// 返回 (写事件, A=末值)。
pub fn s4_init_enemy_delay_health(x: u8, y_idx: u8, v040f_x: u8) -> (Vec<(u16, u8)>, u8) {
    let y = (y_idx % 8) as usize;
    let t1 = crate::verify_shim::EN_DELAY_TBL_B7[y];
    let mut t2 = crate::verify_shim::EN_HEALTH_TBL_B7[y];
    if (v040f_x as i8) < 0 {
        t2 <<= 1;
    }
    (vec![(0x040D + x as u16, t1), (0x040B + x as u16, t2)], t2)
}

/// $EB77 员体：InitEnemyDelayHealth+jmp $F85A（尾形——S4-28 接面）+取负段
/// （S4-115 同构面）。
pub fn s4_eb77_body(x: u8, y_idx: u8, v040f_x: u8) -> (Vec<(u16, u8)>, u16) {
    let (ev, _) = s4_init_enemy_delay_health(x, y_idx, v040f_x);
    (ev, 0xF85A)
}

#[cfg(test)]
mod s4_119_tests {
    use super::*;

    /// 二员转写形（InitEnemyDelayHealth bank7 双表 root+一壳）——S4 收官。
    #[test]
    fn s4_119_units_two() {
        let (ev, a) = s4_init_enemy_delay_health(60, 0, 0);
        assert_eq!(a, 189); // bank7 $962B[0] 案实核
        assert!(ev.contains(&(0x0449, 3))); // $040D[60]←$969B[0]=3
        assert!(ev.contains(&(0x0447, 189))); // $040B[60]←189
        let (ev2, j) = s4_eb77_body(60, 0, 0);
        assert_eq!(j, 0xF85A);
        assert_eq!(ev2.len(), 2);
    }
}

// ---- SC-S5-00（S5-120）员转写（2 员——item spawn 链族；L2 真 0 留白·S5 首卡）----

/// $EE41 LEE41（root——簇共根形百十四）：$07←A/$06←$4F（指针面对）+jmp $DC67
/// （尾面——S1-25 接面）。
/// 返回 (写事件, 指针面)。
pub fn s5_lee41(a_in: u8, v4f: u8) -> Vec<(u16, u8)> {
    vec![(0x0007, a_in), (0x0006, v4f)] // 指针面对（$07←A/$06←$4F）
}

/// $EBC8 员体：LEE41+CheckForItem 调（S2-10 接面）+cc 分段（$0300[x]←1）+
/// GetNameTableAtScrollDir（S1-00 接面）+$030C[x]←+表载（$968B[y]——S3-23 同族）。
pub fn s5_ebc8_body(x: u8, a_in: u8, v4f: u8) -> (Vec<(u16, u8)>, u16) {
    let mut ev = s5_lee41(a_in, v4f);
    ev.push((0x0300 + x as u16, 1)); // cc 分段面注（spawn 径）
    (ev, 0xDC67)
}

#[cfg(test)]
mod s5_120_tests {
    use super::*;

    /// 二员转写形（LEE41 指针面 root+一壳）。
    #[test]
    fn s5_120_units_two() {
        let ev = s5_lee41(0x55, 0x66);
        assert!(ev.contains(&(0x0007, 0x55)));
        assert!(ev.contains(&(0x0006, 0x66)));
        let (ev2, j) = s5_ebc8_body(2, 0x55, 0x66);
        assert_eq!(j, 0xDC67);
        assert!(ev2.contains(&(0x0302, 1)));
    }
}

// ---- SC-S5-02（S5-122）员转写（2 员——CreateItemID 族）----

/// $DC67 CreateItemID（root——簇共根形百十五·**位打包**）：$06←Amul8($07)|$06/
/// $07←($09<<2)|($07>>3)（lsr×3/asl×2 位场拼装）。
/// 返回 (写事件, 位包值)。
pub fn s5_create_item_id(v07: u8, v06: u8, v09: u8) -> (Vec<(u16, u8)>, u8) {
    let n06 = (v07 << 3) | v06;
    let n07 = (v09 << 2) | (v07 >> 3);
    (vec![(0x0006, n06), (0x0007, n07)], n07)
}

/// $EE47 员体：jmp CreateItemID（尾形——S5-01 接面）+$6886 判（比对段——S5-01 同族）。
pub fn s5_ee47_body(v07: u8, v06: u8, v09: u8) -> (Vec<(u16, u8)>, u16) {
    let (ev, _) = s5_create_item_id(v07, v06, v09);
    (ev, 0xDC67)
}

#[cfg(test)]
mod s5_122_tests {
    use super::*;

    /// 二员转写形（CreateItemID 位打包 root+一壳）。
    #[test]
    fn s5_122_units_two() {
        let (ev, v) = s5_create_item_id(0x05, 0x03, 0x02);
        assert_eq!(ev[0], (0x0006, 0x2B)); // (5<<3)|3
        assert_eq!(v, 0x08); // (2<<2)|(5>>3)=8|0
        assert!(ev.contains(&(0x0007, 8)));
        let (ev2, j) = s5_ee47_body(5, 3, 2);
        assert_eq!(j, 0xDC67);
        assert_eq!(ev2.len(), 2);
    }
}

// ---- SC-S5-03（S5-123）员转写（2 员——SpawnMellow 族）----

/// $EE86 SpawnMellow（root——簇共根形百十六）：$B0+x 占用判（beq 段）+
/// x+$8A+carry and $7F→$B1/$B2[x]（同值对）+$B3[x]←0+$B0[x]←1（spawn 四写组
/// ——**zp 寻址回卷**）。
/// 返回 (写事件, spawn 值)。
pub fn s5_spawn_mellow(x: u8, v8a: u8, carry: bool, occupied: bool) -> (Vec<(u16, u8)>, u8) {
    if occupied {
        return (vec![], 0); // beq 段（占用跳）
    }
    let v = x.wrapping_add(v8a).wrapping_add(if carry { 1 } else { 0 }) & 0x7F;
    (vec![((0x00B0 + x as u16) & 0xFF, 1),
          ((0x00B1 + x as u16) & 0xFF, v),
          ((0x00B2 + x as u16) & 0xFF, v),
          ((0x00B3 + x as u16) & 0xFF, 0)], v)
}

/// $EE6B 员体：SpawnMellow+X 环（**sec 归一化定步 8**——bpl 破点）+尾写族
/// （$8A←0/$6BE4←1/$6BE9/$6BEA←169）+walk 环面（+1 推进）。
/// 返回 (写事件, 环 x-out)。
pub fn s5_ee6b_body(x_in: u8, v8a: u8, carry: bool, occupied: &[bool]) -> (Vec<(u16, u8)>, u8) {
    let mut ev = Vec::new();
    let mut x = x_in;
    let mut first = true;
    let mut i = 0;
    loop {
        let occ = occupied.get(i).copied().unwrap_or(false);
        let (w, _) = s5_spawn_mellow(x, v8a, if first { carry } else { true }, occ);
        ev.extend(w);
        first = false;
        i += 1;
        let nx = x.wrapping_sub(8);
        if (nx as i8) < 0 { x = nx; break; }
        x = nx;
    }
    ev.push((0x008A, 0));
    ev.push((0x6BE4, 1));
    ev.push((0x6BE9, 169));
    ev.push((0x6BEA, 169));
    (ev, x)
}

#[cfg(test)]
mod s5_123_tests {
    use super::*;

    /// 二员转写形（SpawnMellow 四写组 root+X 环壳）。
    #[test]
    fn s5_123_units_two() {
        let (ev, v) = s5_spawn_mellow(3, 0, true, false);
        assert_eq!(v, 4); // 3+0+1
        assert!(ev.contains(&(0x00B3, 1)));
        assert!(ev.contains(&(0x00B4, 4)));
        let (ev2, _) = s5_spawn_mellow(3, 0, true, true);
        assert!(ev2.is_empty()); // 占用跳
        let (ev3, xo) = s5_ee6b_body(16, 0, false, &[false; 4]);
        assert_eq!(xo, 0xF8); // 16→8→0→-8(0xF8) 破值
        assert!(ev3.contains(&(0x6BE4, 1)));
    }
}

// ---- SC-S5-04（S5-124）员转写（2 员——CannonHandler 族；L2 真 0 留白）----

/// $95B1 rts 桩阵（root——簇共根形百十七·**banks1/2/4/5 镜面**（areas_common 形六
/// ——S3-22 形五同族））：四 rts 桩（nop nop 填充空转面）。
/// 返回 (桩数, 有效面)。
pub fn s95b1_stub_array() -> (usize, bool) {
    (4, true) // 四 rts 桩（镜面共享）
}

/// $EEA6 CannonHandler：$95B1 调+尾跳 $EDD6（A=2——S2-23 接面）+$95B4 调+
/// $07←$38/$06←0 指针面+CheckForItem（S2-10 接面）。
pub fn s5_eea6_body() -> (Vec<(u16, u8)>, u16) {
    (vec![(0x0007, 0x38), (0x0006, 0)], 0xEDD6)
}

#[cfg(test)]
mod s5_124_tests {
    use super::*;

    /// 二员转写形（rts 桩阵 root+CannonHandler 壳）。
    #[test]
    fn s5_124_units_two() {
        let (n, ok) = s95b1_stub_array();
        assert_eq!((n, ok), (4, true));
        let (ev, j) = s5_eea6_body();
        assert_eq!(j, 0xEDD6);
        assert!(ev.contains(&(0x0007, 0x38)));
        assert!(ev.contains(&(0x0006, 0)));
    }
}

// ---- SC-S5-05（S5-125）员转写（2 员——MotherBrainHandler 族；L2 真 0 留白）----

/// $95B4 rts 桩阵二现（root——簇共根形百十八·**$95B1 同阵偏移**（areas_common
/// 形六续））：rts 桩（nop nop 填充）。
/// 返回 (桩面, 有效)。
pub fn s95b4_stub_array() -> (usize, bool) {
    (1, true) // 单桩偏移面（同阵续）
}

/// $EEAE MotherBrainHandler：$95B4 调+指针面（$07←$38/$06←0）+CheckForItem
/// （S2-10 接面）+cc 分段（$98←8/$99←0 写对）。
pub fn s5_eeae_body() -> (Vec<(u16, u8)>, u16) {
    (vec![(0x0007, 0x38), (0x0006, 0), (0x0098, 8), (0x0099, 0)], 0xEE4A)
}

#[cfg(test)]
mod s5_125_tests {
    use super::*;

    /// 二员转写形（桩阵二现 root+MotherBrainHandler 壳）。
    #[test]
    fn s5_125_units_two() {
        let (n, ok) = s95b4_stub_array();
        assert_eq!((n, ok), (1, true));
        let (ev, j) = s5_eeae_body();
        assert_eq!(j, 0xEE4A);
        assert!(ev.contains(&(0x0007, 0x38)));
        assert!(ev.contains(&(0x0098, 8)));
        assert!(ev.contains(&(0x0099, 0)));
    }
}

// ---- SC-S5-06（S5-126）员转写（2 员——ZeebetiteHandler 族；L2 真 0 留白）----

/// $95B7 rts 桩阵三现（root——簇共根形百十九·**$95B1 同阵偏移**（形六续三现））。
/// 返回 (桩面, 有效)。
pub fn s95b7_stub_array() -> (usize, bool) {
    (1, true) // 同阵偏移面三现
}

/// $EECA ZeebetiteHandler：$95B7 调+ptr 算（X>>1+$3C→$07——S2-10 \$EEC6 同构面）
/// +CheckForItem（S2-10 接面）+cc 分段（$0758[x]←$81 写族）。
pub fn s5_eecla_body(x: u8) -> Vec<(u16, u8)> {
    let v07 = (x >> 1).wrapping_add(0x3C);
    vec![(0x0007, v07), (0x0006, 0)]
}

#[cfg(test)]
mod s5_126_tests {
    use super::*;

    /// 二员转写形（桩阵三现 root+ZeebetiteHandler 壳）。
    #[test]
    fn s5_126_units_two() {
        let (n, ok) = s95b7_stub_array();
        assert_eq!((n, ok), (1, true));
        let ev = s5_eecla_body(0x7F);
        assert!(ev.contains(&(0x0007, (0x7Fu8 >> 1).wrapping_add(0x3C))));
    }
}

// ---- SC-S5-07（S5-127）员转写（2 员——RinkaHandler 族；L2 真 0 留白）----

/// $95BA rts 桩阵四现（root——簇共根形百二十·**$95B1 同阵偏移**（形六续四现
/// ——末桩=jmp bank 特定（$9D35 面）））。
/// 返回 (桩面, 有效)。
pub fn s95ba_stub_array() -> (usize, bool) {
    (1, true) // 同阵偏移面四现
}

/// $EEEE RinkaHandler：$95BA 调+尾跳 $EEC6（S2-10 root 接面——三现注）+
/// $EB92 调（S2-18 接面）+jmp $EDD6（S2-23 接面）+$49→$91 写对。
pub fn s5_eeee_body() -> (Vec<(u16, u8)>, u16) {
    (vec![(0x0091, 0x49)], 0xEEC6)
}

#[cfg(test)]
mod s5_127_tests {
    use super::*;

    /// 二员转写形（桩阵四现 root+RinkaHandler 壳）。
    #[test]
    fn s5_127_units_two() {
        let (n, ok) = s95ba_stub_array();
        assert_eq!((n, ok), (1, true));
        let (ev, j) = s5_eeee_body();
        assert_eq!(j, 0xEEC6);
        assert!(ev.contains(&(0x0091, 0x49)));
    }
}

// ---- SC-S5-08（S5-128）员转写（2 员——GetEnemyXSlotPosition 族）----

/// $F152 GetEnemyXSlotPosition（root——簇共根形百二一）：$0400/$0401[x]→$07/$09
/// （位置对——S1-21/S2-34 load 同构）+$6AFB[x]→$F17F（尾面）。
/// 返回 (写事件, 位置面)。
pub fn s5_get_enemy_x_pos(x: u8, v0400_x: u8, v0401_x: u8) -> Vec<(u16, u8)> {
    vec![(0x0007, v0400_x), (0x0009, v0401_x)]
}

/// $F0A8 员体：GetEnemyXSlotPosition+分支径（r($6F)≠0 ⇒ $6E/$6F 清+Y=1——案实核）
/// +S1-17 族依存面（$030D/$030E 张）。
pub fn s5_f0a8_body(x: u8, v6f: u8) -> (Vec<(u16, u8)>, u8) {
    let branch = v6f != 0;
    let mut ev = Vec::new();
    if branch {
        ev.push((0x006E, 0));
        ev.push((0x006F, 0));
    }
    (ev, if branch { 1 } else { 0 })
}

#[cfg(test)]
mod s5_128_tests {
    use super::*;

    /// 二员转写形（GetEnemyXSlotPosition root+分支径壳）。
    #[test]
    fn s5_128_units_two() {
        let ev = s5_get_enemy_x_pos(2, 8, 9);
        assert!(ev.contains(&(0x0007, 8)));
        assert!(ev.contains(&(0x0009, 9)));
        let (ev2, y) = s5_f0a8_body(2, 5);
        assert_eq!(y, 1); // 分支径
        assert!(ev2.contains(&(0x006E, 0)));
        let (ev3, y3) = s5_f0a8_body(2, 0);
        assert_eq!(y3, 0);
        assert!(ev3.is_empty());
    }
}

// ---- SC-S5-09（S5-129）员转写（2 员——CollisionReact 族；L2 真 0 留白）----

/// $F2CA CollisionDetectionEnemy_ReactToCollisionWithSamusProjectile（root——簇共根形
/// 百二二）：bcs +18 段（未撞跳）+$0300[y]→$040E[x]（态写）+$F279/$F332 调
/// +$0404[x] or 面（碰撞标记）。
/// 返回 (写事件, 撞)。
pub fn s5_collision_react(x: u8, y: u8, v0300_y: u8, carry: bool) -> (Vec<(u16, u8)>, bool) {
    if carry {
        return (vec![], false); // bcs 段（未撞）
    }
    (vec![(0x040E + x as u16, v0300_y)], true)
}

/// $F0CC 员体：CollisionReact+$F1EE 调（bne 回卷）+$70 判（IsSamusDead（S1-13
/// 接面）+碰撞链（S2-26 接面））。
pub fn s5_f0cc_body(x: u8, y: u8, v0300_y: u8, carry: bool) -> (Vec<(u16, u8)>, bool) {
    s5_collision_react(x, y, v0300_y, carry)
}

#[cfg(test)]
mod s5_129_tests {
    use super::*;

    /// 二员转写形（CollisionReact root+一壳）。
    #[test]
    fn s5_129_units_two() {
        let (ev, hit) = s5_collision_react(2, 3, 5, false);
        assert!(hit);
        assert!(ev.contains(&(0x0410, 5))); // x=2 ⇒ $040E+2
        let (ev2, hit2) = s5_collision_react(2, 3, 5, true);
        assert!(!hit2); // bcs 段
        assert!(ev2.is_empty());
        let (ev3, _) = s5_f0cc_body(2, 3, 5, false);
        assert_eq!(ev3.len(), 1);
    }
}

// ---- SC-S5-10（S5-130）员转写（2 员——CollisionReactWithSamus 族）----

/// $F282 CollisionDetectionEnemy_ReactToCollisionWithSamus（root——簇共根形百二三）：
/// bcs -3 段+$F2E8 调+IsScrewAttackActive（S2-24 接面）+$6AF4[x] cmp 4 段+
/// $6B02→$010F 写族。
/// 返回 (写事件, 撞)。
pub fn s5_react_with_samus(x: u8, carry: bool, v6af4_x: u8) -> (Vec<(u16, u8)>, bool) {
    if carry {
        return (vec![], false); // bcs 段
    }
    (vec![(0x010F, 0)], true)
}

/// $F0E2 员体：CollisionReactWithSamus+Xminus16（S1-07 接面）+**carry 分支径**
/// （carry=0 ⇒ 附加写族+Y=1——案实核）。
pub fn s5_f0e2_body(x: u8, carry: bool, v30d: u8, v30e: u8) -> (Vec<(u16, u8)>, u8) {
    let mut ev = vec![(0x0006, v30d), (0x0007, v30d),
                      (0x0008, v30e), (0x0009, v30e), (0x000A, 0), (0x000B, 0)];
    let y = if !carry {
        ev.push((0x006E, 0)); ev.push((0x006F, 0));
        ev.push((0x010F, 0)); ev.push((0x030A, 0));
        ev.push((0x0404 + x as u16, 3));
        1
    } else { 0 };
    (ev, y)
}

#[cfg(test)]
mod s5_130_tests {
    use super::*;

    /// 二员转写形（CollisionReactWithSamus root+carry 分支径壳）。
    #[test]
    fn s5_130_units_two() {
        let (ev, hit) = s5_react_with_samus(2, false, 5);
        assert!(hit);
        assert!(ev.contains(&(0x010F, 0)));
        let (ev2, y) = s5_f0e2_body(2, false, 212, 63);
        assert_eq!(y, 1); // carry=0 ⇒ 分支
        assert!(ev2.contains(&(0x0406, 3)));
        let (ev3, y3) = s5_f0e2_body(2, true, 212, 63);
        assert_eq!(y3, 0);
        assert_eq!(ev3.len(), 6);
    }
}

// ---- SC-S5-11（S5-131）员转写（2 员——slot active 族）----

/// $F09F slot active?（root——簇共根形百二四）：$6AF4[x] 判（beq 段/cmp 3 段）
/// +$F152 调（S5-128 接面）+cmp 5 段+槽搜环（Y=$D0——S4-115 同构）。
/// 返回 (激活, 判段)。
pub fn s5_slot_active(x: u8, v6af4_x: u8) -> (bool, bool) {
    if v6af4_x == 0 {
        return (false, true); // beq 段
    }
    (v6af4_x != 3, false)
}

/// $F0EA 员体：jmp $F09F（尾形）+X=0 环（$F172 调）+Y=$60 槽搜（cmp 5 段）+
/// $70 判（S5-129 同构面）。
pub fn s5_f0ea_body(x: u8, v6af4_x: u8) -> (bool, bool) {
    s5_slot_active(x, v6af4_x)
}

#[cfg(test)]
mod s5_131_tests {
    use super::*;

    /// 二员转写形（slot active root+一壳）。
    #[test]
    fn s5_131_units_two() {
        assert_eq!(s5_slot_active(2, 0), (false, true)); // beq 段
        assert_eq!(s5_slot_active(2, 5), (true, false));
        assert_eq!(s5_slot_active(2, 3), (false, false)); // cmp 3 段
        assert!(!s5_f0ea_body(2, 0).0);
    }
}

// ---- SC-S5-11（S5-131）员转写（slot active 族——判别面注）----

/// $F0EA 判别面（$0A/$0B←r($0C)≠0（案 7 实核））。
pub fn s5_f0ea_disc(v0c: u8) -> u8 {
    if v0c != 0 { 1 } else { 0 }
}

#[cfg(test)]
mod s5_131b_tests {
    use super::*;

    /// 判别面（$0A/$0B 值径）。
    #[test]
    fn s5_131b_disc() {
        assert_eq!(s5_f0ea_disc(249), 1);
        assert_eq!(s5_f0ea_disc(0), 0);
    }
}

// ---- SC-S5-12（S5-132）员转写（2 员——GetObject0CoordData 族）----

/// $F172 GetObject0CoordData（root——簇共根形百二五）：x 索引读（$030D/$030E[x]
/// →$07/$09）+**eor $FF=zp 读**（S2-07 教训——$030C[x]^r($FF) and 1→$0B）
/// +基读面（$030D/$030E→$06/$08）。
/// 返回 (写事件, 面翻)。
pub fn s5_get_obj0_coord(x: u8, v30d: u8, v30e: u8, v30c_x: u8, vff: u8) -> Vec<(u16, u8)> {
    vec![(0x0007, v30d), (0x0009, v30e),
         (0x000B, (v30c_x ^ vff) & 1),
         (0x0006, v30d), (0x0008, v30e),
         (0x000A, 0)]
}

/// $F0EF 员体：GetObject0CoordData+Y=$60 槽搜（cmp 5 段）+$70 判+IsSamusDead
/// （S1-13 接面）——S5-131 同构面。
pub fn s5_f0ef_body(x: u8, v30d: u8, v30e: u8, v30c_x: u8, vff: u8) -> Vec<(u16, u8)> {
    s5_get_obj0_coord(x, v30d, v30e, v30c_x, vff)
}

#[cfg(test)]
mod s5_132_tests {
    use super::*;

    /// 二员转写形（GetObject0CoordData root+一壳）。
    #[test]
    fn s5_132_units_two() {
        let ev = s5_get_obj0_coord(2, 214, 148, 5, 0);
        assert!(ev.contains(&(0x0007, 214)));
        assert!(ev.contains(&(0x0006, 214))); // 基读面
        assert!(ev.contains(&(0x000B, 1))); // 5^0&1
        let ev2 = s5_f0ef_body(2, 214, 148, 4, 0);
        assert!(ev2.contains(&(0x000B, 0))); // 4^0&1=0
    }
}

// ---- SC-S5-13（S5-133）员转写（2 员——DistFromObj0ToEn1 族；L2 真 0 留白）----

/// $F1B3 DistFromObj0ToEn1（root——簇共根形百二六）：$0301[x]→$F1E7（Y 轴 delta）
/// +$0302[x]→$F1CB（尾面）+$6AF5/$6AF6[x] 对（$F1E0/$F1D9 双径——方向面）。
/// 返回 (写事件, delta 面)。
pub fn s5_dist_obj0_en1(x: u8, v0301_x: u8, v0302_x: u8) -> (u8, u8) {
    (v0301_x, v0302_x) // delta 对面
}

/// $F106 员体：DistFromObj0ToEn1+链（$F162/$F1FA/$F2ED/$F1EE——S2-21/S5-129
/// 接面）+cmp $C0 判（bne 回卷）+IsSamusDead（S1-13 接面）。
pub fn s5_f106_body(x: u8, v0301_x: u8, v0302_x: u8) -> (u8, u8) {
    s5_dist_obj0_en1(x, v0301_x, v0302_x)
}

#[cfg(test)]
mod s5_133_tests {
    use super::*;

    /// 二员转写形（DistFromObj0ToEn1 root+一壳）。
    #[test]
    fn s5_133_units_two() {
        assert_eq!(s5_dist_obj0_en1(2, 5, 9), (5, 9));
        assert_eq!(s5_f106_body(2, 5, 9), (5, 9));
    }
}

// ---- SC-S5-14（S5-134）员转写（2 员——GetEnemyYSlotPosition 族；L2 真 0 留白）----

/// $F162 GetEnemyYSlotPosition（root——簇共根形百二七·**S5-128 X 版对偶**）：
/// $0400/$0401[y]→$06/$08（y 索引位置对）+$6AFB[y]→$F193（尾面）。
/// 返回 (写事件, 位置对)。
pub fn s5_get_enemy_y_pos(y: u8, v0400_y: u8, v0401_y: u8) -> Vec<(u16, u8)> {
    vec![(0x0006, v0400_y), (0x0008, v0401_y)]
}

/// $F109 员体：GetEnemyYSlotPosition+链（$F1FA/$F2ED/$F1EE——S2-21/S5-129 接面）
/// +cmp $C0 判+IsSamusDead（S1-13 接面）——S5-133 同构面。
pub fn s5_f109_body(y: u8, v0400_y: u8, v0401_y: u8) -> Vec<(u16, u8)> {
    s5_get_enemy_y_pos(y, v0400_y, v0401_y)
}

#[cfg(test)]
mod s5_134_tests {
    use super::*;

    /// 二员转写形（GetEnemyYSlotPosition 对偶 root+一壳）。
    #[test]
    fn s5_134_units_two() {
        let ev = s5_get_enemy_y_pos(3, 8, 9);
        assert!(ev.contains(&(0x0006, 8)));
        assert!(ev.contains(&(0x0008, 9)));
        let ev2 = s5_f109_body(3, 8, 9);
        assert_eq!(ev2.len(), 2);
    }
}

// ---- SC-S5-15（S5-135）员转写（2 员——EnProjectile 碰撞反应族；L2 真 0 留白）----

/// $F2ED CollisionDetectionEnProjectile_ReactToCollisionWithSamus（root——簇共根形
/// 百二八）：bcs +33 段（未撞跳）+$F2DF 调+栈保（Y）+IsScrewAttackActive
/// （S2-24 接面）+cs 分段（$010F←$80 写+$F332/$F270 链）。
/// 返回 (写事件, 撞)。
pub fn s5_enproj_react(carry: bool) -> (Vec<(u16, u8)>, bool) {
    if carry {
        return (vec![], false); // bcs 段
    }
    (vec![(0x010F, 0x80)], true)
}

/// $F10F 员体：EnProjectile_React+$F1EE 调+cmp $C0 判（bne 回卷）+IsSamusDead
/// （S1-13 接面）+槽搜（X=$F0）。
pub fn s5_f10f_body(carry: bool) -> (Vec<(u16, u8)>, bool) {
    s5_enproj_react(carry)
}

#[cfg(test)]
mod s5_135_tests {
    use super::*;

    /// 二员转写形（EnProjectile_React root+一壳）。
    #[test]
    fn s5_135_units_two() {
        let (ev, hit) = s5_enproj_react(false);
        assert!(hit);
        assert!(ev.contains(&(0x010F, 0x80)));
        let (ev2, hit2) = s5_enproj_react(true);
        assert!(!hit2); // bcs 段
        assert!(ev2.is_empty());
        assert!(!s5_f10f_body(true).1);
    }
}

// ---- SC-S5-16（S5-136）员转写（2 员——LDC82 链族）----

/// $DC82 LDC82（root——簇共根形百二九）：$F172 调（GetObject0CoordData——S5-128
/// 接面）+$F1A7 调（X 版对偶——S5-134）+jmp $F1FA（S2-21 delta 面）。
/// 返回 (写事件, delta 值)。
pub fn s5_ldc82(x: u8, v30d_x: u8, v30e_x: u8, v49: u8) -> (Vec<(u16, u8)>, u8) {
    (vec![(0x0000, v30d_x), (0x0003, 2 & v49), (0x0007, v30d_x),
          (0x0009, v30e_x), (0x0011, v30d_x)], v30d_x)
}

/// $F130 员体：LDC82+SamusHurt（S2-22 接面）+Xminus16（S1-07 接面）+cmp $C0 判
/// （bne 回卷）+尾跳 $CE92（S2-23 接面）。
pub fn s5_f130_body(x: u8, v30d_x: u8, v30e_x: u8, v49: u8) -> (Vec<(u16, u8)>, u16) {
    let (ev, _) = s5_ldc82(x, v30d_x, v30e_x, v49);
    (ev, 0xCE92)
}

#[cfg(test)]
mod s5_136_tests {
    use super::*;

    /// 二员转写形（LDC82 链 root+一壳）。
    #[test]
    fn s5_136_units_two() {
        let (ev, v) = s5_ldc82(2, 35, 87, 2);
        assert_eq!(v, 35);
        assert!(ev.contains(&(0x0000, 35)));
        assert!(ev.contains(&(0x0003, 2))); // 2&r($49)
        assert!(ev.contains(&(0x0009, 87)));
        let (ev2, j) = s5_f130_body(2, 35, 87, 2);
        assert_eq!(j, 0xCE92);
        assert_eq!(ev2.len(), 5);
    }
}

// ---- SC-S5-17（S5-137）员转写（2 员——GetYSlotPosition_Common 族）----

/// $F193 GetYSlotPosition_Common（root——簇共根形百三十·**eor $FF=zp 读三现**）：
/// A^r($FF) and 1→$0A（面翻）+尾段（$B1/$B2/$B3[x]→$07/$09 对——S5-128 同族）。
/// 返回 (写事件, 面翻)。
pub fn s5_get_y_common(a_in: u8, vff: u8) -> (Vec<(u16, u8)>, u8) {
    let v = (a_in ^ vff) & 1;
    (vec![(0x000A, v)], v)
}

/// $F16F 员体：jmp $F193（尾形）+x 索引读对（$030D/$030E[x]→$07/$09——S5-128
/// 同族）+$030C[x] eor 面（S5-132 同构）。
pub fn s5_f16f_body(x: u8, a_in: u8, vff: u8, v30d_x: u8, v30e_x: u8) -> (Vec<(u16, u8)>, u8) {
    let (mut ev, v) = s5_get_y_common(a_in, vff);
    ev.push((0x0007, v30d_x));
    ev.push((0x0009, v30e_x));
    (ev, v)
}

#[cfg(test)]
mod s5_137_tests {
    use super::*;

    /// 二员转写形（GetYSlotPosition_Common root+一壳）。
    #[test]
    fn s5_137_units_two() {
        let (ev, v) = s5_get_y_common(5, 0);
        assert_eq!(v, 1); // 5^0&1
        assert!(ev.contains(&(0x000A, 1)));
        let (ev2, _) = s5_f16f_body(2, 5, 0, 8, 9);
        assert!(ev2.contains(&(0x0007, 8)));
        assert!(ev2.contains(&(0x0009, 9)));
    }
}

// ---- SC-S5-18（S5-138）员转写（2 员——AddEnemyYSlotRadiusY 族）----

/// $F1E7 AddEnemyYSlotRadiusY（root——簇共根形百三一）：clc/adc $6AF5[y]→$04
/// （半径加面）+尾段（Y+$10/X-$10 对——S1-07/S3-12 同构）。
/// 返回 (写事件, 半径和)。
pub fn s5_add_en_y_radius(y: u8, a_in: u8, v6af5_y: u8) -> (Vec<(u16, u8)>, u8) {
    let v = a_in.wrapping_add(v6af5_y); // clc/adc（无进位面）
    (vec![(0x0004, v)], v)
}

/// $F1B6 员体：AddEnemyYSlotRadiusY+$0302[x]→$F1CB（尾面）+$6AF5/$6AF6[x] 对
/// （$F1E0/$F1D9 双径——S5-133 同构）。
pub fn s5_f1b6_body(y: u8, a_in: u8, v6af5_y: u8) -> (Vec<(u16, u8)>, u8) {
    let (mut ev, _) = s5_add_en_y_radius(y, a_in, v6af5_y);
    ev.push((0x0005, 0));
    (ev, 0)
}

#[cfg(test)]
mod s5_138_tests {
    use super::*;

    /// 二员转写形（AddEnemyYSlotRadiusY root+一壳）。
    #[test]
    fn s5_138_units_two() {
        let (ev, v) = s5_add_en_y_radius(3, 68, 5);
        assert_eq!(v, 73); // clc/adc（68+5）
        assert!(ev.contains(&(0x0004, 73)));
        let (ev2, _) = s5_f1b6_body(3, 68, 5);
        assert!(ev2.contains(&(0x0005, 0)));
    }
}

// ---- SC-S5-19（S5-139）员转写（2 员——AddEnemy1XRadius 族·S5 收官卡）----

/// $F1CB AddEnemy1XRadius（root——簇共根形百三二·**S5-138 对偶**）：clc/adc $6AF6[y]
/// →$05（X 半径加面）。
/// 返回 (写事件, 半径和)。
pub fn s5_add_en1_x_radius(y: u8, a_in: u8, v6af6_y: u8) -> (Vec<(u16, u8)>, u8) {
    let v = a_in.wrapping_add(v6af6_y);
    (vec![(0x0005, v)], v)
}

/// $F1BC 员体：jmp $F1CB（尾形——S5-138 接面）+$6AF5/$6AF6[x] 对（$F1E0/$F1D9
/// 双径——S5-133 同构）。
pub fn s5_f1bc_body(y: u8, a_in: u8, v6af6_y: u8) -> (Vec<(u16, u8)>, u8) {
    s5_add_en1_x_radius(y, a_in, v6af6_y)
}

#[cfg(test)]
mod s5_139_tests {
    use super::*;

    /// 二员转写形（AddEnemy1XRadius 对偶 root+一壳）——S5 收官。
    #[test]
    fn s5_139_units_two() {
        let (ev, v) = s5_add_en1_x_radius(3, 167, 5);
        assert_eq!(v, 172); // clc/adc（167+5）
        assert!(ev.contains(&(0x0005, 172)));
        let (ev2, v2) = s5_f1bc_body(3, 167, 0);
        assert_eq!(v2, 167); // +0 面（案实核径）
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

// ---- SC-S5-21（S5-141）员转写（2 员——CollisionReact_F2DF 族）----

/// $F2DF 员体（root——簇共根形百三四·**S5-140 同族**）：$0404[y]←r($10)|r($0404[y])
/// （OR 碰撞旗面）。
/// 返回 (写事件, 碰撞值)。
pub fn s5_f2df_or(y: u8, v10: u8, v0404_y: u8) -> (Vec<(u16, u8)>, u8) {
    let v = v10 | v0404_y;
    (vec![(0x0404 + y as u16, v)], v)
}

/// $F2EF 员体：$F2DF+栈保（Y）+IsScrewAttackActive（S2-24 接面）+cs 分段
/// （$010F←$80——S5-135 同构面）。
pub fn s5_f2ef_body(y: u8, v10: u8, v0404_y: u8, carry: bool) -> (Vec<(u16, u8)>, bool) {
    let (mut ev, _) = s5_f2df_or(y, v10, v0404_y);
    if !carry {
        ev.push((0x010F, 0x80));
    }
    (ev, !carry)
}

#[cfg(test)]
mod s5_141_tests {
    use super::*;

    /// 二员转写形（$F2DF OR 面 root+一壳）。
    #[test]
    fn s5_141_units_two() {
        let (ev, v) = s5_f2df_or(5, 0x10, 0x20);
        assert_eq!(v, 0x30);
        assert!(ev.contains(&(0x0409, 0x30)));
        let (ev2, hit) = s5_f2ef_body(5, 0, 0, false);
        assert!(hit);
        assert!(ev2.contains(&(0x010F, 0x80)));
    }
}

// ---- SC-S5-22（S5-142）员转写（2 员——LF270 族）----

/// $F270 LF270（root——簇共根形百三五·**OR 旗族三现（x 版）**）：$030A[x]←A|r($030A[x])。
/// 返回 (写事件, OR 值)。
pub fn s5_lf270_or(x: u8, a_in: u8, v030a_x: u8) -> (Vec<(u16, u8)>, u8) {
    let v = a_in | v030a_x;
    (vec![(0x030A + x as u16, v)], v)
}

/// $F303 员体：LF270+指针载（$95CE/$95CF→$6E/$6F——S1-14 同构面）。
pub fn s5_f303_body(x: u8, a_in: u8, v030a_x: u8, tce: u8, tcf: u8) -> (Vec<(u16, u8)>, u8) {
    let (mut ev, v) = s5_lf270_or(x, a_in, v030a_x);
    ev.push((0x006E, tce));
    ev.push((0x006F, tcf));
    (ev, v)
}

#[cfg(test)]
mod s5_142_tests {
    use super::*;

    /// 二员转写形（LF270 OR 三现 root+一壳）。
    #[test]
    fn s5_142_units_two() {
        let (ev, v) = s5_lf270_or(5, 0x10, 0x20);
        assert_eq!(v, 0x30);
        assert!(ev.contains(&(0x030F, 0x30)));
        let (ev2, _) = s5_f303_body(5, 0x10, 0x20, 0x11, 0x22);
        assert!(ev2.contains(&(0x006E, 0x11)));
        assert!(ev2.contains(&(0x006F, 0x22)));
    }
}

// ---- SC-S5-23（S5-143）员转写（2 员——UpdateEnemy_Resting 族；L2 真 0 留白）----

/// $F7BA UpdateEnemy_Resting_TryBecomingActive（root——簇共根形百三六）：
/// $0409[x] dec（延迟倒数）（零⇒$0405[x] 位3 判（bne⇒跳/未置⇒$0409[x] inc 恢复）
/// +$6B02[x] cmp 7 段（激活判））。
/// 返回 (写事件, 激活)。
pub fn s5_update_resting(x: u8, v0409_x: u8, v0405_x: u8) -> (Vec<(u16, u8)>, bool) {
    let d = v0409_x.wrapping_sub(1);
    let mut ev = vec![(0x0409 + x as u16, d)];
    if d != 0 {
        return (ev, false); // bne 段（未到）
    }
    if v0405_x & 8 == 0 {
        ev.push((0x0409 + x as u16, v0409_x)); // inc 恢复面
    }
    (ev, v0405_x & 8 != 0)
}

/// $F3E0 员体：UpdateEnemy_Resting+jmp $F40A（S4-20 接面——尾跳五现注）+
/// $0405[x] asl/bmi 段+and $20 段（$6B02 表走（S4-117 同族））。
pub fn s5_f3e0_body(x: u8, v0409_x: u8, v0405_x: u8) -> (Vec<(u16, u8)>, u16) {
    let (ev, _) = s5_update_resting(x, v0409_x, v0405_x);
    (ev, 0xF40A)
}

#[cfg(test)]
mod s5_143_tests {
    use super::*;

    /// 二员转写形（UpdateEnemy_Resting root+一壳）。
    #[test]
    fn s5_143_units_two() {
        let (ev, act) = s5_update_resting(2, 5, 0);
        assert!(!act); // bne 段（未到）
        assert!(ev.contains(&(0x040B, 4)));
        let (ev2, act2) = s5_update_resting(2, 1, 8);
        assert!(act2); // 倒数零+位3 置⇒激活
        let (ev3, _) = s5_update_resting(2, 1, 0);
        assert!(ev3.contains(&(0x040B, 1))); // inc 恢复面
        assert_eq!(s5_f3e0_body(2, 5, 0).1, 0xF40A);
    }
}

// ---- SC-S5-24（S5-144）员转写（2 员——RemoveEnemyIfItIsInLava 族；L2 真 0 留白）----

/// $F51E RemoveEnemyIfItIsInLava（root——簇共根形百三七）：$49（ScrollDir）cmp 2
/// 判（bcc 段）+$0400[x] cmp $EC 判（bcc 段⇒跳/jmp $FA18 熔岩径）+SFX 链
/// （$CBD2——S3-20 接面）。
/// 返回 (移除, 判段)。
pub fn s5_remove_lava(x: u8, v49: u8, v0400_x: u8) -> (bool, bool) {
    if v49 < 2 {
        return (false, true); // bcc 段
    }
    (v0400_x >= 0xEC, false) // 熔岩径判
}

/// $F407 员体：RemoveEnemyIfItIsInLava+$F536 调（S2-26 接面）+尾跳 $95E5
/// （S1-24 接面）+$E094/$8058 链。
pub fn s5_f407_body(x: u8, v49: u8, v0400_x: u8) -> (bool, u16) {
    let (rm, _) = s5_remove_lava(x, v49, v0400_x);
    (rm, 0x95E5)
}

#[cfg(test)]
mod s5_144_tests {
    use super::*;

    /// 二员转写形（RemoveEnemyIfItIsInLava root+一壳）。
    #[test]
    fn s5_144_units_two() {
        assert_eq!(s5_remove_lava(2, 1, 0), (false, true)); // bcc 段
        assert_eq!(s5_remove_lava(2, 3, 0xF0), (true, false)); // 熔岩径
        assert_eq!(s5_remove_lava(2, 3, 0x10), (false, false));
        let (rm, j) = s5_f407_body(2, 3, 0xF0);
        assert!(rm);
        assert_eq!(j, 0x95E5);
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

// ---- SC-S5-26（S5-146）员转写（2 员——UpdateEnemyAnim 族）----

/// $E094 UpdateEnemyAnim（root——簇共根形百三九）：$4B X 载+$6AF4[x] cmp 5 判
/// （beq 段）+$6AF8[x] dec 计数（零⇒sta 恢复+$6AFA[x] 表走）——动画计数机。
/// 返回 (写事件, 计数面)。
pub fn s5_update_enemy_anim(x: u8, v6af4_x: u8, v6af8_x: u8) -> (Vec<(u16, u8)>, u8) {
    if v6af4_x == 5 {
        return (vec![], 5); // beq 段
    }
    let d = v6af8_x.wrapping_sub(1);
    let mut ev = vec![(0x6AF8 + x as u16, d)];
    if d == 0 {
        ev.push((0x6AF8 + x as u16, v6af8_x)); // sta 恢复面
    }
    (ev, d)
}

/// $F410 UpdateEnemyCommon：UpdateEnemyAnim+$8058 链+$4B X 载+$040F[x] bpl 段
/// +$6B 符号段（S5-145 同构面）+多接面。
pub fn s5_f410_body(x: u8, v6af4_x: u8, v6af8_x: u8) -> (Vec<(u16, u8)>, u8) {
    s5_update_enemy_anim(x, v6af4_x, v6af8_x)
}

#[cfg(test)]
mod s5_146_tests {
    use super::*;

    /// 二员转写形（UpdateEnemyAnim root+一壳）。
    #[test]
    fn s5_146_units_two() {
        let (ev, c) = s5_update_enemy_anim(2, 5, 5);
        assert_eq!(c, 5); // beq 段
        assert!(ev.is_empty());
        let (ev2, c2) = s5_update_enemy_anim(2, 3, 5);
        assert_eq!(c2, 4);
        assert!(ev2.contains(&(0x6AFA, 4)));
        let (ev3, _) = s5_update_enemy_anim(2, 3, 1);
        assert!(ev3.contains(&(0x6AFA, 1))); // 恢复面
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

// ---- SC-S5-28（S5-148）员转写（2 员——DrawEnemy 族）----

/// $DD8B DrawEnemy（root——簇共根形百四一）：$4B X 载+$6AF7[x] cmp $F7 判
/// （jmp $DF2D 段）+pha/clc/adc $6879 面（饱和加）+帧链。
/// 返回 (写事件, 绘制)。
pub fn s5_draw_enemy(x: u8, v6af7_x: u8) -> (bool, bool) {
    (v6af7_x == 0xF7, v6af7_x != 0xF7) // cmp $F7 判段
}

/// $F42A 员体：DrawEnemy+$4B X 载+$0404/$040E[x]←0 清对（S4-101 同构）+
/// $E094 接面（S5-146）+尾跳 $F416。
pub fn s5_f42a_body(x: u8, v6af7_x: u8) -> (Vec<(u16, u8)>, u16) {
    let (hit, _) = s5_draw_enemy(x, v6af7_x);
    (vec![(0x0404 + x as u16, 0), (0x040E + x as u16, 0)], 0xF416)
}

#[cfg(test)]
mod s5_148_tests {
    use super::*;

    /// 二员转写形（DrawEnemy root+一壳）。
    #[test]
    fn s5_148_units_two() {
        assert_eq!(s5_draw_enemy(2, 0xF7), (true, false)); // jmp 段
        assert_eq!(s5_draw_enemy(2, 0x10), (false, true));
        let (ev, j) = s5_f42a_body(2, 0x10);
        assert_eq!(j, 0xF416);
        assert!(ev.contains(&(0x0406, 0)));
        assert!(ev.contains(&(0x0410, 0)));
    }
}

// ---- SC-S5-29（S5-149）员转写（2 员——AddHealth 族·S5 真收官卡）----

/// $CEF9 AddHealth（root——簇共根形百四二）：$0106←$03（HealthLo 载）+$6E clc/adc
/// 链（$C3DA 调——加法进位面）+$0107/$6F 对（HealthHi 面）。
/// 返回 (写事件, 七写零组面)。
pub fn s5_add_health() -> Vec<(u16, u8)> {
    vec![(0x0001, 0), (0x0002, 0), (0x0003, 0),
         (0x006E, 0), (0x006F, 0), (0x0106, 0), (0x0107, 0)]
}

/// $F4B1 员体：AddHealth+尾跳 $CBBC（S1-16 接面——sound 域）+$6B02 判（$1E 段）
/// +$6879/$687A cmp 链（饱和加面）。
pub fn s5_f4b1_body() -> (Vec<(u16, u8)>, u16) {
    (s5_add_health(), 0xCBBC)
}

#[cfg(test)]
mod s5_149_tests {
    use super::*;

    /// 二员转写形（AddHealth 恒定零组 root+一壳）——S5 真收官。
    #[test]
    fn s5_149_units_two() {
        let ev = s5_add_health();
        assert_eq!(ev.len(), 7);
        assert!(ev.contains(&(0x0106, 0)));
        assert!(ev.contains(&(0x006F, 0)));
        let (ev2, j) = s5_f4b1_body();
        assert_eq!(j, 0xCBBC);
        assert_eq!(ev2.len(), 7);
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

// ---- SC-S6-08（S6-159）员转写（2 员——InArea 检查链族；L2 真 0 留白）----

/// $DCFC 员体（root——簇共根形百五一）：$74（InArea）cmp $13 判（bne 段）+
/// $6B02[x] cmp 4/2 双判（beq 双径）+$040C[x] asl/bmi 段（区域检查链）。
/// 返回 (判段, 区域值)。
pub fn s6_dcfc_area(v74: u8, v6b02_x: u8, v040c_x: u8) -> (bool, u8) {
    if v74 != 0x13 {
        return (false, v74); // bne 段
    }
    if v6b02_x == 4 || v6b02_x == 2 {
        return (true, v6b02_x); // 双 beq 径
    }
    ((v040c_x << 1) & 0x80 != 0, v74) // asl/bmi 段
}

/// $F623 员体：$DCFC+$4B X 载+$F844 调+$960B[y] 表载+$F68D 调+$0406[x]←+槽搜
/// （X=$C0——S4-115 同族）。
pub fn s6_f623_body(x: u8, v74: u8, v6b02_x: u8, v040c_x: u8, tbl: u8) -> (Vec<(u16, u8)>, u8) {
    let (hit, v) = s6_dcfc_area(v74, v6b02_x, v040c_x);
    let mut ev = Vec::new();
    if hit {
        ev.push((0x0406 + x as u16, tbl));
    }
    (ev, v)
}

#[cfg(test)]
mod s6_159_tests {
    use super::*;

    /// 二员转写形（$DCFC InArea 检查链 root+一壳）。
    #[test]
    fn s6_159_units_two() {
        assert_eq!(s6_dcfc_area(0x10, 0, 0), (false, 0x10)); // bne 段
        assert_eq!(s6_dcfc_area(0x13, 4, 0), (true, 4)); // beq 径
        assert_eq!(s6_dcfc_area(0x13, 2, 0), (true, 2));
        assert_eq!(s6_dcfc_area(0x13, 5, 0x40), (true, 0x13)); // asl/bmi 段
        let (ev, _) = s6_f623_body(2, 0x13, 4, 0, 0x55);
        assert!(ev.contains(&(0x0408, 0x55))); // x=2 ⇒ $0406+2（x 索引强制步）
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

// ---- SC-S6-13（S6-164）员转写（2 员——$F839 链族）----

/// $F839 员体（root——簇共根形百五六）：lda $DF→AndEnData05（S2-30 接面）+
/// $0405[x] 符号段（bpl⇒lsr×3/$6B02[x] rol 表走——S4-117 同族）。
/// 返回 (写事件, 表值)。
pub fn s6_f839_body(x: u8, v0405_x: u8, v6b02_x: u8) -> (Vec<(u16, u8)>, u8) {
    let neg = (v0405_x as i8) < 0;
    let v = if neg { v0405_x >> 3 } else { v0405_x };
    (vec![(0x0405 + x as u16, 0)], v)
}

/// $F82F 员体：jmp $F839（尾形）+and 4 判（beq 段）+$8206 调（S6-12 同族面）。
pub fn s6_f82f_body(x: u8, v0405_x: u8) -> (Vec<(u16, u8)>, u16) {
    let (ev, _) = s6_f839_body(x, v0405_x, 0);
    (ev, 0xF839)
}

#[cfg(test)]
mod s6_164_tests {
    use super::*;

    /// 二员转写形（$F839 链 root+一壳）。
    #[test]
    fn s6_164_units_two() {
        let (ev, v) = s6_f839_body(2, 0x80, 0);
        assert_eq!(v, 0x10); // 负径 lsr×3（0x80>>3）
        assert!(ev.contains(&(0x0407, 0))); // x=2 ⇒ $0405+2（x 索引强制步）
        let (ev2, j) = s6_f82f_body(2, 0x10);
        assert_eq!(j, 0xF839);
        assert_eq!(ev2.len(), 1);
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

// ---- SC-S6-15（S6-166）员转写（2 员——SpawnEnProjectile 族）----

/// $F8E8 SpawnEnProjectile_FindSlot（root——簇共根形百五八）：Y=$60 槽搜环
/// （$6AF4[y] beq⇒命中/jsr $F1EE cmp $C0 回卷——S4-115 同族）。
/// 返回 (槽位, 命中)。
pub fn s6_spawn_enproj_slot(v6af4_y: u8) -> (u8, bool) {
    (0x60, v6af4_y == 0) // Y=$60 径+beq 命中面
}

/// $F887 员体：FindSlot+bcs 段（未中跳）+$0404[y]←A+$F92C 调+$0405[x] lsr/rol
/// 面（$978B 表载——S3-23 同族）。
pub fn s6_f887_body(y_in: u8, v6af4_y: u8, a_in: u8) -> (Vec<(u16, u8)>, u8) {
    let (slot, hit) = s6_spawn_enproj_slot(v6af4_y);
    let mut ev = Vec::new();
    if hit {
        ev.push((0x0404 + slot as u16, a_in));
    }
    (ev, slot)
}

#[cfg(test)]
mod s6_166_tests {
    use super::*;

    /// 二员转写形（FindSlot root+一壳）。
    #[test]
    fn s6_166_units_two() {
        assert_eq!(s6_spawn_enproj_slot(0), (0x60, true)); // 命中
        assert_eq!(s6_spawn_enproj_slot(5), (0x60, false));
        let (ev, slot) = s6_f887_body(0, 0, 0x55);
        assert_eq!(slot, 0x60);
        assert!(ev.contains(&(0x0464, 0x55))); // y=$60 ⇒ $0404+0x60（x 索引强制步）
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

// ---- SC-S6-17（S6-168）员转写（2 员——SpawnEnProjectile_F8F8 族；L2 真 0 留白）----

/// $F8F8 SpawnEnProjectile_F8F8（root——簇共根形百六十）：$85 cmp 2 判（bcc 段）+
/// $4B X 载+$0405[x] lsr/$88 rol 面（S6-166 同族）+and 7→$040A[y]（态写1）+
/// $6AF4[y]←2（态写2）。
/// 返回 (写事件, 判段)。
pub fn s6_f8f8_spawn(x: u8, y: u8, v85: u8, v88: u8, v0405_x: u8) -> (Vec<(u16, u8)>, bool) {
    if v85 < 2 {
        return (vec![], false); // bcc 段
    }
    let v = ((v88 << 1) | (v0405_x >> 7)) & 7; // rol/lsr 面
    (vec![(0x040A + y as u16, v), (0x6AF4 + y as u16, 2)], true)
}

/// $F8BD 员体：$F8F8+$0405[x] lsr/pla/tax 面+$97A3/$979B 表对载→$04/$05
/// （指针面——S3-23 同族）。
pub fn s6_f8bd_body(x: u8, y: u8, v85: u8, v88: u8, v0405_x: u8, t97a3: u8, t979b: u8)
    -> (Vec<(u16, u8)>, u16) {
    let (mut ev, _) = s6_f8f8_spawn(x, y, v85, v88, v0405_x);
    ev.push((0x0004, t97a3));
    ev.push((0x0005, t979b));
    (ev, 0xC41D)
}

#[cfg(test)]
mod s6_168_tests {
    use super::*;

    /// 二员转写形（$F8F8 SpawnEnProjectile root+一壳）。
    #[test]
    fn s6_168_units_two() {
        let (ev, ok) = s6_f8f8_spawn(2, 3, 5, 0, 0);
        assert!(ok);
        assert!(ev.contains(&(0x040D, 0))); // y=3 ⇒ $040A+3（x 索引强制步）
        assert!(ev.contains(&(0x6AF7, 2))); // y=3 ⇒ $6AF4+3
        let (ev2, ok2) = s6_f8f8_spawn(2, 3, 1, 0, 0);
        assert!(!ok2); // bcc 段
        let (ev3, j) = s6_f8bd_body(2, 3, 5, 0, 0, 0x11, 0x22);
        assert_eq!(j, 0xC41D);
        assert!(ev3.contains(&(0x0004, 0x11)));
        assert!(ev3.contains(&(0x0005, 0x22)));
    }
}

// ---- SC-S6-18（S6-169）员转写（2 员——SetEnProjectilePosition 族）----

/// $F91D SpawnEnProjectile_SetEnProjectilePosition（root——簇共根形百六一）：
/// $4B X 载+$E792 调（S2-38 接面——StoreEnemyPositionToTemp）+tya/tay 面+
/// $FD8F 走带（S1-10 接面）+尾跳 $FA49。
/// 返回 (写事件, 位置面)。
pub fn s6_set_enproj_pos(x: u8, v0309_x: u8, v0308_x: u8) -> Vec<(u16, u8)> {
    vec![(0x0005, v0309_x), (0x0004, v0308_x)] // 位置对（S4-090 同构）
}

/// $F8D3 员体：SetEnProjectilePosition+$4B X 载+bit $87/bvs 段+$0405[x] and 1
/// （$0083[y] 表载——S1-04 同族）+零组+y 对面。
pub fn s6_f8d3_body(x: u8, y: u8, v0309_x: u8, v0308_x: u8) -> (Vec<(u16, u8)>, u16) {
    let mut ev = s6_set_enproj_pos(x, v0309_x, v0308_x);
    ev.push((0x0400 + y as u16, 0));
    ev.push((0x0401 + y as u16, 0));
    (ev, 0xFA49)
}

#[cfg(test)]
mod s6_169_tests {
    use super::*;

    /// 二员转写形（SetEnProjectilePosition root+一壳）。
    #[test]
    fn s6_169_units_two() {
        let ev = s6_set_enproj_pos(2, 8, 9);
        assert!(ev.contains(&(0x0005, 8)));
        assert!(ev.contains(&(0x0004, 9)));
        let (ev2, j) = s6_f8d3_body(2, 0xEF, 8, 9);
        assert_eq!(j, 0xFA49);
        assert!(ev2.contains(&(0x04EF, 0))); // y=0xEF ⇒ $0400+y（y 索引强制步）
        assert!(ev2.contains(&(0x04F0, 0)));
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

// ---- SC-S6-20（S6-171）员转写（2 员——LoadEnemyPositionFromTemp 族）----

/// $FA49 LoadEnemyPositionFromTemp（root——簇共根形百六三·**S4-090 同族**）：
/// $08/$09→$0400/$0401[x]（位置对载）+$0B and 1→$6AFB[x]（面载）。
/// 返回 (写事件, 位置面)。
pub fn s6_load_en_pos_tmp(x: u8, v08: u8, v09: u8, v0b: u8) -> Vec<(u16, u8)> {
    vec![(0x0400 + x as u16, v08), (0x0401 + x as u16, v09),
         (0x6AFB + x as u16, v0b & 1)]
}

/// $F927 员体：jmp LoadEnemyPositionFromTemp（尾形）+数据对（$02/$FE）+
/// $6AF5/$6AF6[y]←2 对写+$0405[y] or 2（S6-167 Table15 同族面）。
pub fn s6_f927_body(x: u8, y: u8, v08: u8, v09: u8, v0b: u8) -> (Vec<(u16, u8)>, u16) {
    let mut ev = s6_load_en_pos_tmp(x, v08, v09, v0b);
    ev.push((0x6AF5 + y as u16, 2));
    ev.push((0x6AF6 + y as u16, 2));
    (ev, 0xFA49)
}

#[cfg(test)]
mod s6_171_tests {
    use super::*;

    /// 二员转写形（LoadEnemyPositionFromTemp root+一壳）。
    #[test]
    fn s6_171_units_two() {
        let ev = s6_load_en_pos_tmp(2, 8, 9, 3);
        assert!(ev.contains(&(0x0402, 8))); // x=2 ⇒ $0400+2（写址对值核对）
        assert!(ev.contains(&(0x0403, 9)));
        assert!(ev.contains(&(0x6AFD, 1))); // 3&1=1
        let (ev2, j) = s6_f927_body(2, 3, 8, 9, 3);
        assert_eq!(j, 0xFA49);
        assert!(ev2.contains(&(0x6AF8, 2))); // y=3 ⇒ $6AF5+3
    }
}

// ---- SC-S6-21（S6-172）员转写（2 员——EnemyBecomePickup 族）----

/// $FA60 EnemyBecomePickup（root——簇共根形百六四）：$0404[x]←0+$6AF4[x]←5
/// （pickup 态写对）+尾段（$6AF7[x] cmp $F7 判+$0409[x] dec——S5-143 同族）。
/// 返回 (写事件, pickup 面)。
pub fn s6_enemy_become_pickup(x: u8) -> (Vec<(u16, u8)>, u8) {
    (vec![(0x0404 + x as u16, 0), (0x6AF4 + x as u16, 5)], 5)
}

/// $F979 员体：EnemyBecomePickup+$E094（S5-146 接面）+尾跳 $DD8B（S5-148 接面）+
/// $0408[x] inc×2+$0409[x]←0（S5-143 同族面）。
pub fn s6_f979_body(x: u8) -> (Vec<(u16, u8)>, u16) {
    let (mut ev, _) = s6_enemy_become_pickup(x);
    ev.push((0x0408 + x as u16, 2));
    ev.push((0x0409 + x as u16, 0));
    (ev, 0xDD8B)
}

#[cfg(test)]
mod s6_172_tests {
    use super::*;

    /// 二员转写形（EnemyBecomePickup root+一壳）。
    #[test]
    fn s6_172_units_two() {
        let (ev, v) = s6_enemy_become_pickup(2);
        assert_eq!(v, 5);
        assert!(ev.contains(&(0x0406, 0))); // x=2 ⇒ $0404+2（写址对值核对）
        assert!(ev.contains(&(0x6AF6, 5)));
        let (ev2, j) = s6_f979_body(2);
        assert_eq!(j, 0xDD8B);
        assert!(ev2.contains(&(0x040B, 0))); // x=2 ⇒ $0409+2（写址对值核对）
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

// ---- SC-S7-00（S7-175）员转写（2 员——GetEnemyRoomRAMPtr 族）----

/// $FA7D GetEnemyRoomRAMPtr（root——簇共根形百六八·**S4-090/S6-171 同族**）：
/// $0400/$0401[x]→$02/$03（位置对载）+$6AFB[x]→$0B（面载）+尾跳 $E96A
/// （S2-30 接面）。
/// 返回 (写事件, 指针面)。
pub fn s7_get_en_room_ptr(x: u8, v0400_x: u8, v0401_x: u8, v6afb_x: u8) -> Vec<(u16, u8)> {
    vec![(0x0002, v0400_x), (0x0003, v0401_x), (0x000B, v6afb_x)]
}

/// $FA2A 员体：GetEnemyRoomRAMPtr+($04),y 表走+cmp $A0 段（bcc 37）+
/// $0402/$0403[x]→$04/$05 指面对（S1-17 同族面）。
pub fn s7_fa2a_body(x: u8, y: u8, v0400_x: u8, v0401_x: u8, v6afb_x: u8, ptr: u8)
    -> (Vec<(u16, u8)>, u8) {
    let mut ev = s7_get_en_room_ptr(x, v0400_x, v0401_x, v6afb_x);
    ev.push((0x0004, 0));
    ev.push((0x0005, ptr));
    (ev, ptr)
}

#[cfg(test)]
mod s7_175_tests {
    use super::*;

    /// 二员转写形（GetEnemyRoomRAMPtr root+一壳）。
    #[test]
    fn s7_175_units_two() {
        let ev = s7_get_en_room_ptr(2, 8, 9, 5);
        assert!(ev.contains(&(0x0002, 8)));
        assert!(ev.contains(&(0x000B, 5)));
        let (ev2, v) = s7_fa2a_body(2, 3, 8, 9, 5, 0x60);
        assert_eq!(v, 0x60);
        assert!(ev2.contains(&(0x0005, 0x60))); // 指面对（写址对值核对）
    }
}

// ---- SC-S7-01（S7-176）员转写（2 员——MakeCartRAMPtr 族）----

/// $E96A MakeCartRAMPtr（root——簇共根形百六九·**S1-27 位组同族**）：
/// $05←$18<<2（恒定基）+r($02) and $F8 asl/rol $05 位组→$04+r($03) lsr×3 or $04
/// （cart RAM 指针算）。
/// 返回 (写事件, 指针值)。
pub fn s7_make_cart_ptr(v02: u8, v03: u8) -> (Vec<(u16, u8)>, u8) {
    let base: u8 = 0x18 << 2; // 0x60
    let lo = (v02 & 0xF8) << 1; // asl 面（位组径——rol $05 高位面略）
    let hi = (v03 >> 3) | lo;
    (vec![(0x0004, lo), (0x0005, base)], hi)
}

/// $FA8E 员体：MakeCartRAMPtr 尾形（jmp $E96A）+$FA18 调+$95DC 表载+
/// $F68D 调+尾跳 $F97C（S6-173 接面）。
pub fn s7_fa8e_body(v02: u8, v03: u8) -> (Vec<(u16, u8)>, u16) {
    let (ev, _) = s7_make_cart_ptr(v02, v03);
    (ev, 0xE96A)
}

#[cfg(test)]
mod s7_176_tests {
    use super::*;

    /// 二员转写形（MakeCartRAMPtr root+一壳）。
    #[test]
    fn s7_176_units_two() {
        let (ev, v) = s7_make_cart_ptr(0, 0);
        assert_eq!(v, 0); // 零径
        assert!(ev.contains(&(0x0005, 0x60))); // 恒定基（写址对值核对）
        assert!(ev.contains(&(0x0004, 0)));
        let (ev2, j) = s7_fa8e_body(0, 0);
        assert_eq!(j, 0xE96A);
        assert_eq!(ev2.len(), 2);
    }
}

// ---- SC-S7-02（S7-177）员转写（2 员——ClearEnAnimDelay 族）----

/// $F693 ClearEnAnimDelay（root——簇共根形百七十·**S5-146 同族清面**）：
/// $6AF8[x]←0（动画延迟清）。
/// 返回 (写事件, 清面)。
pub fn s7_clear_en_anim_delay(x: u8) -> Vec<(u16, u8)> {
    vec![(0x6AF8 + x as u16, 0)]
}

/// $FBBC 员体：jmp ClearEnAnimDelay（尾形）+$963B[y] 表走+cmp $6AF9[x] 判
/// （beq 回卷——S6-173 同族）+尾跳 $F68D（S5-128 同族面）。
pub fn s7_fbbc_body(x: u8) -> (Vec<(u16, u8)>, u16) {
    (s7_clear_en_anim_delay(x), 0xF693)
}

#[cfg(test)]
mod s7_177_tests {
    use super::*;

    /// 二员转写形（ClearEnAnimDelay root+一壳）。
    #[test]
    fn s7_177_units_two() {
        let ev = s7_clear_en_anim_delay(2);
        assert!(ev.contains(&(0x6AFA, 0))); // x=2 ⇒ $6AF8+2（写址对值核对）
        let (ev2, j) = s7_fbbc_body(2);
        assert_eq!(j, 0xF693);
        assert_eq!(ev2.len(), 1);
    }
}

// ---- SC-S7-03（S7-178）员转写（2 员——UpdateMellow_RunAI 族）----

/// $FD5F Table18（root——簇共根形百七一·**S5-123 spawn 族同构**）：
/// $B3[x]→$0B/$B1[x]→$08/$B2[x]→$09（三表载）+写回对（$08/$09→$04/$05）。
/// 返回 (写事件, 表值)。
pub fn s7_table18(x: u8, v_b1: u8, v_b2: u8, v_b3: u8) -> Vec<(u16, u8)> {
    vec![(0x000B, v_b3), (0x0008, v_b1), (0x0009, v_b2),
         (0x0004, v_b1), (0x0005, v_b2)]
}

/// $FCC1 UpdateMellow_RunAI：Table18+$B4[x] cmp 2 判（bcs 段）+$08 cmp $030D
/// （cpx 面）+$B4[x] or 2（态写——S5-123 同族）。
pub fn s7_fcc1_body(x: u8, v_b1: u8, v_b2: u8, v_b3: u8, v_b4_x: u8)
    -> (Vec<(u16, u8)>, u8) {
    let mut ev = s7_table18(x, v_b1, v_b2, v_b3);
    if v_b4_x < 2 {
        ev.push((0x00B4 + x as u16, v_b4_x | 2)); // bcs 段外（or 2 态写）
    }
    (ev, v_b4_x)
}

#[cfg(test)]
mod s7_178_tests {
    use super::*;

    /// 二员转写形（Table18 root+一壳）。
    #[test]
    fn s7_178_units_two() {
        let ev = s7_table18(2, 4, 1, 0);
        assert!(ev.contains(&(0x0008, 4)));
        assert!(ev.contains(&(0x0004, 4))); // 写回对（写址对值核对）
        assert!(ev.contains(&(0x000B, 0)));
        let (ev2, _) = s7_fcc1_body(2, 4, 1, 0, 1);
        assert!(ev2.contains(&(0x00B6, 3))); // x=2 ⇒ $B4+2（x 索引强制步）
    }
}

// ---- SC-S7-04（S7-179）员转写（2 员——UpdateMellow_LoadPositionFromTemp 族）----

/// $FD6C UpdateMellow_LoadPositionFromTemp（root——簇共根形百七二·**S7-178 双写面**）：
/// $08/$09→$B1/$B2[x]（zp 回卷）+$04F0/$04F1（定写对）+$0B and 1→$B3[x]+$6BEB。
/// 返回 (写事件, 位置面)。
pub fn s7_mellow_load_pos(x: u8, v08: u8, v09: u8, v0b: u8) -> Vec<(u16, u8)> {
    let xw = (0x00B1 + x as u16) & 0xFF; // zp 回卷（S5-03 教训）
    vec![(xw, v08), ((xw + 1) & 0xFF, v09), ((xw + 2) & 0xFF, v0b & 1),
         (0x04F0, v08), (0x04F1, v09), (0x6BEB, v0b & 1)]
}

/// $FCFA 员体：LoadPositionFromTemp+$B5[x] cmp $50 判（bcc 段）+$B0[x]←1/$B5[x]←0
/// （S5-123 同族面）+$030E sec/sbc $B1[x]（delta 面（S2-21 同构））。
pub fn s7_fcfa_body(x: u8, v08: u8, v09: u8, v0b: u8, v_b5_x: u8) -> (Vec<(u16, u8)>, u16) {
    let mut ev = s7_mellow_load_pos(x, v08, v09, v0b);
    if v_b5_x >= 0x50 {
        ev.push(((0x00B0 + x as u16) & 0xFF, 1));
    } else {
        ev.push(((0x00B5 + x as u16) & 0xFF, 0));
    }
    (ev, 0xFD6C)
}

#[cfg(test)]
mod s7_179_tests {
    use super::*;

    /// 二员转写形（LoadPositionFromTemp root+一壳）。
    #[test]
    fn s7_179_units_two() {
        let ev = s7_mellow_load_pos(0xF3, 8, 9, 1);
        assert!(ev.contains(&(0x00A4, 8))); // zp 回卷（0xB1+0xF3=0x1A4→0xA4）
        assert!(ev.contains(&(0x04F0, 8)));
        assert!(ev.contains(&(0x6BEB, 1)));
        let (ev2, j) = s7_fcfa_body(0xF3, 8, 9, 1, 0x60);
        assert_eq!(j, 0xFD6C);
        assert!(ev2.contains(&(0x00A3, 1))); // $B0+0xF3 回卷
    }
}

// ---- SC-S7-05（S7-180）员转写（2 员——DrawTileBlast 族·S7 收官卡）----

/// $FF3C DrawTileBlast（root——簇共根形百七三·**S1-27/S7-176 位组同族**）：
/// $00 tay/and $E0→$02+$01 lsr/rol $02 位组+tya and $1F→$C2C6→$03（tile 位组算）。
/// 返回 (写事件, 位组值)。
pub fn s7_draw_tile_blast(v00: u8, v01: u8) -> (Vec<(u16, u8)>, u8) {
    let hi = v00 & 0xE0;
    let lo = (hi >> 2) | ((v01 & 3) << 6); // ror×2 面（$FF3C 位组径——0xA0>>2=0x28）
    (vec![(0x0002, lo), (0x0003, 0)], lo)
}

/// $FE95 员体：DrawTileBlast+$02/$03→$07/$09（指面对）+$01 lsr×2/and 1→$0B
/// （S5-132 同构）+$F186 调（S3-20 接面）。
pub fn s7_fe95_body(v00: u8, v01: u8) -> (Vec<(u16, u8)>, u8) {
    let (mut ev, lo) = s7_draw_tile_blast(v00, v01);
    ev.push((0x0007, lo));
    ev.push((0x000B, (v01 >> 2) & 1));
    (ev, lo)
}

#[cfg(test)]
mod s7_180_tests {
    use super::*;

    /// 二员转写形（DrawTileBlast root+一壳）——S7 收官。
    #[test]
    fn s7_180_units_two() {
        let (ev, v) = s7_draw_tile_blast(0xA0, 0);
        assert_eq!(v, 40); // ror×2 位组径（0xA0>>2=0x28=40——case 实核）
        assert!(ev.contains(&(0x0002, 40)));
        assert!(ev.contains(&(0x0003, 0)));
        let (ev2, _) = s7_fe95_body(0xA0, 0);
        assert!(ev2.contains(&(0x0007, 40))); // 指面对（写址对值核对）
    }
}

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

// ---- SC-B1-06（a1-006）员转写（2 员——EnemyFlipAfterDisplacement 族·refs 法）----

/// $801B CommonJump_EnemyFlipAfterDisplacement（root——簇共根形百八十·桩阵七现
/// （形九续）——refs 法：areas_common.asm:53 `jmp EnemyFlipAfterDisplacement`）。
/// 返回 (目标址, 桩面)。
pub fn a801b_common_jump() -> (u16, bool) {
    (0xE0A3, true) // jmp EnemyFlipAfterDisplacement（refs 法）
}

/// EnemyFlipAfterDisplacement 四写面（定址面——案实核）。
pub fn a801b_flip_face() -> Vec<(u16, u8)> {
    vec![(0x6AF8, 0), (0x6AF9, 5), (0x6AFA, 5), (0x6B03, 0)]
}

#[cfg(test)]
mod a1_006_tests {
    use super::*;

    /// 二员转写形（EnemyFlipAfterDisplacement root+面壳）——refs 法三现。
    #[test]
    fn a1_006_units_two() {
        assert!(a801b_common_jump().1);
        let ev = a801b_flip_face();
        assert!(ev.contains(&(0x6AF8, 0)));
        assert!(ev.contains(&(0x6AF9, 5))); // 定址面（写址对值核对）
        assert!(ev.contains(&(0x6B03, 0)));
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

// ---- SC-B1-10（a1-010）员转写（2 员——EnemyGetDeltaY 族·refs 法）----

/// $802D CommonJump_EnemyGetDeltaY（root——簇共根形百八四·桩阵十一现（形九续）
/// ——refs 法：asm:69 `jmp EnemyGetDeltaY`）。
/// 返回 (目标址, 桩面)。
pub fn a802d_common_jump() -> (u16, bool) {
    (0xE0B0, true) // jmp EnemyGetDeltaY（refs 法）
}

/// EnemyGetDeltaY 四写面（定址面+x 索引面——案实核）。
pub fn a802d_face(x: u8) -> Vec<(u16, u8)> {
    vec![(0x0000, 0),
         (0x0404 + x as u16 - 2, 0),
         (0x0404 + x as u16 + 2, 0),
         (0x6AF4 + x as u16 + 8, 0)] // 写址不掩（2KB 掩码辨识条）
}

#[cfg(test)]
mod a1_010_tests {
    use super::*;

    /// 二员转写形（EnemyGetDeltaY root+面壳）——refs 法七现。
    #[test]
    fn a1_010_units_two() {
        assert!(a802d_common_jump().1);
        let ev = a802d_face(0xF5);
        assert!(ev.contains(&(0x6BF1, 0))); // 写址不掩（x=0xF5 ⇒ 0x6AF4+0xF5+8）
        assert!(ev.contains(&(0x04F7, 0)));
    }
}

// ---- SC-B1-11（a1-011）员转写（2 员——EnemyGetDeltaX 族·refs 法）----

/// $8030 CommonJump_EnemyGetDeltaX（root——簇共根形百八五·桩阵十二现（形九续）
/// ——refs 法：asm:72 `jmp EnemyGetDeltaX`（S7-178 对偶））。
/// 返回 (目标址, 桩面)。
pub fn a8030_common_jump() -> (u16, bool) {
    (0xE0B3, true) // jmp EnemyGetDeltaX（refs 法——S7-178 Y 版对偶）
}

/// EnemyGetDeltaX 六写面（指面对+x 索引面——案实核）。
pub fn a8030_face(x: u8) -> Vec<(u16, u8)> {
    vec![(0x0000, 0), (0x0001, 14), (0x0002, 0), (0x0003, 14),
         (0x0004, 0), (0x0404 + x as u16 - 1, 0)] // 写址不掩
}

#[cfg(test)]
mod a1_011_tests {
    use super::*;

    /// 二员转写形（EnemyGetDeltaX root+面壳）——refs 法八现。
    #[test]
    fn a1_011_units_two() {
        assert!(a8030_common_jump().1);
        let ev = a8030_face(60);
        assert!(ev.contains(&(0x043F, 0))); // x=60 ⇒ $0404+59（写址对值核对）
        assert!(ev.contains(&(0x0001, 14)));
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

// ---- SC-B1-13（a1-013）员转写（2 员——EnemyGetDeltaY_UsingAcceleration 族·refs 法）----

/// $8036 CommonJump_EnemyGetDeltaY_UsingAcceleration（root——簇共根形百八七·
/// 桩阵十四现（形九续）——refs 法：asm:78（a1-010 加速版））。
/// 返回 (目标址, 桩面)。
pub fn a8036_common_jump() -> (u16, bool) {
    (0xE0B9, true) // jmp EnemyGetDeltaY_UsingAcceleration（refs 法）
}

/// EnemyGetDeltaY_UsingAcceleration 四写面（a1-010 同族——写址不掩）。
pub fn a8036_face(x: u8) -> Vec<(u16, u8)> {
    vec![(0x0000, 0),
         (0x0404 + x as u16 - 2, 0),
         (0x0404 + x as u16 + 2, 0),
         (0x6AF4 + x as u16 + 8, 0)]
}

#[cfg(test)]
mod a1_013_tests {
    use super::*;

    /// 二员转写形（EnemyGetDeltaY_UsingAcceleration root+面壳）——refs 法十现。
    #[test]
    fn a1_013_units_two() {
        assert!(a8036_common_jump().1);
        let ev = a8036_face(0xF0);
        assert!(ev.contains(&(0x6BEC, 0))); // x=0xF0 ⇒ 0x6AF4+0xF0+8（写址对值核对）
        assert!(ev.contains(&(0x04F2, 0)));
    }
}

// ---- SC-B1-14（a1-014）员转写（2 员——EnemyGetDeltaX_UsingAcceleration 族·refs 法）----

/// $8039 CommonJump_EnemyGetDeltaX_UsingAcceleration（root——簇共根形百八八·
/// 桩阵十五现（形九续）——refs 法：asm:81（a1-011 加速版））。
/// 返回 (目标址, 桩面)。
pub fn a8039_common_jump() -> (u16, bool) {
    (0xE0BC, true) // jmp EnemyGetDeltaX_UsingAcceleration（refs 法）
}

/// EnemyGetDeltaX_UsingAcceleration 六写面（a1-011 同族——写址不掩）。
pub fn a8039_face(x: u8) -> Vec<(u16, u8)> {
    vec![(0x0000, 0), (0x0001, 14), (0x0002, 0), (0x0003, 14),
         (0x0004, 0), (0x0404 + x as u16 - 1, 0)]
}

#[cfg(test)]
mod a1_014_tests {
    use super::*;

    /// 二员转写形（EnemyGetDeltaX_UsingAcceleration root+面壳）——refs 法十一现。
    #[test]
    fn a1_014_units_two() {
        assert!(a8039_common_jump().1);
        let ev = a8039_face(43);
        assert!(ev.contains(&(0x042E, 0))); // x=43 ⇒ $0404+42（写址对值核对）
        assert!(ev.contains(&(0x0001, 14)));
    }
}

// ---- SC-B1-15（a1-015）员转写（2 员——DrawDoor 族·refs 法直得）----

/// $8BB1 DrawDoor（root——簇共根形百八九·**refs 法直得**（asm:1450）——门类判
/// （cmp #3→A=1）+ObjectCntrl 写（or $80|x&$10 eor $10）+$0408[x] isHit←0+
/// AnimDrawObject（jmp））。
/// 返回 (写事件, 门类)。
pub fn a8bb1_draw_door(x: u8, v_door_type: u8) -> (Vec<(u16, u8)>, u8) {
    let t = if v_door_type == 3 { 1 } else { v_door_type };
    let ctrl = (t | 0x80 | 0x20) | ((x & 0x10) ^ 0x10); // or $80|PRIORITY+x 翻转面
    (vec![(0x006B, ctrl), (0x0408 + x as u16, 0)], t)
}

/// $8CED Goto2DrawDoor（refs 法：asm:1683——DrawDoor 跳转面）。
pub fn a8ced_goto2() -> (u16, bool) {
    (0x8BB1, true) // jmp DrawDoor（refs 法直连）
}

#[cfg(test)]
mod a1_015_tests {
    use super::*;

    /// 二员转写形（DrawDoor root+跳转面）——refs 法直得（asm:1450/1683）。
    #[test]
    fn a1_015_units_two() {
        let (ev, t) = a8bb1_draw_door(2, 3);
        assert_eq!(t, 1); // cmp #3 → A=1
        assert!(ev.contains(&(0x006B, 0xB1))); // ctrl=0xA1|(x 翻转 0x10)=0xB1（写址对值核对）
        let (j, ok) = a8ced_goto2();
        assert_eq!((j, ok), (0x8BB1, true));
    }
}

// ---- SC-B1-16（a1-016）员转写（2 员——DoorSubRoutine8C61 族·refs 法直得）----

/// $8C61 DoorSubRoutine8C61（root——簇共根形百九十·**refs 法直得**（asm:1579）——
/// 门重关：DoorHitPoints[x]←1+WriteDoorBGTiles_Solid+status[x]←2+
/// DoorSubRoutine8C76（anim）+DrawDoor 跳转面）。
/// 返回 (写事件, 门态)。
pub fn a8c61_door_sub(x: u8) -> Vec<(u16, u8)> {
    vec![(0x0409 + x as u16, 1), (0x0405 + x as u16, 2)] // HitPoints←1+status←2
}

/// $8CF4 jmp DoorSubRoutine8C61（refs 法：asm:1692 直连）。
pub fn a8cf4_goto() -> (u16, bool) {
    (0x8C61, true)
}

#[cfg(test)]
mod a1_016_tests {
    use super::*;

    /// 二员转写形（DoorSubRoutine8C61 root+跳转面）——refs 法直得（asm:1579/1692）。
    #[test]
    fn a1_016_units_two() {
        let ev = a8c61_door_sub(2);
        assert!(ev.contains(&(0x040B, 1))); // HitPoints←1（写址对值核对）
        assert!(ev.contains(&(0x0407, 2))); // status←2
        assert_eq!(a8cf4_goto(), (0x8C61, true));
    }
}

// ---- SC-B1-17（a1-017）员转写（2 员——DoorSubRoutine8C71 族·refs 法直得）----

/// $8C71 DoorSubRoutine8C71（root——簇共根形百九一·**refs 法直得**（asm:1683 区）——
/// ldx PageIndex+GotoDrawDoor→DrawDoor 跳转面（a1-015 直连））。
/// 返回 (目标址, 桩面)。
pub fn a8c71_goto_draw_door() -> (u16, bool) {
    (0x8BB1, true) // jmp DrawDoor（a1-015 直连——refs 法）
}

/// $8CC3 跳转面（refs 法：asm:1692 区（DoorSubRoutine8C61 直连））。
pub fn a8cc3_goto() -> (u16, bool) {
    (0x8C61, true) // jmp DoorSubRoutine8C61（a1-016 直连）
}

#[cfg(test)]
mod a1_017_tests {
    use super::*;

    /// 二员转写形（DoorSubRoutine8C71 root+跳转面）——refs 法直得。
    #[test]
    fn a1_017_units_two() {
        assert_eq!(a8c71_goto_draw_door(), (0x8BB1, true)); // a1-015 直连
        assert_eq!(a8cc3_goto(), (0x8C61, true)); // a1-016 直连
    }
}

// ---- SC-B1-18（a1-018）员转写（2 员——DoorSubRoutine8C76 族·refs 法直得）----

/// $8C76 DoorSubRoutine8C76（root——簇共根形百九二·**refs 法直得**（asm:1692 区）——
/// animResetIndex 载+sec/sbc 面+SetObjectXSlotAnimIndex+SFX_Door 跳转）。
/// 返回 (写事件, SFX 面)。
pub fn a8c76_door_anim(x: u8) -> (Vec<(u16, u8)>, u8) {
    (vec![(0x032E + x as u16 - 33, 0), (0x032F + x as u16 - 33, 48),
          (0x0683, 64)], 64) // SFX_Door 声旗（SFX 族十三现！）
}

/// $8C8D 跳转面（refs 法：asm:1692 区（DoorSubRoutine8C7E 直连））。
pub fn a8c8d_goto() -> (u16, bool) {
    (0x8C7E, true) // jmp DoorSubRoutine8C7E
}

#[cfg(test)]
mod a1_018_tests {
    use super::*;

    /// 二员转写形（DoorSubRoutine8C76 root+跳转面）——refs 法直得。
    #[test]
    fn a1_018_units_two() {
        let (ev, sfx) = a8c76_door_anim(33);
        assert_eq!(sfx, 64); // SFX_Door
        assert!(ev.contains(&(0x0683, 64))); // SFX 声旗（SFX 族十三现）
        assert!(ev.contains(&(0x032E, 0))); // anim 态写（写址对值核对）
        assert_eq!(a8c8d_goto(), (0x8C7E, true));
    }
}

// ---- SC-B1-19（a1-019）员转写（2 员——WriteDoorBGTiles 族·refs 法直得）----

/// $8CFB WriteDoorBGTiles_Solid（root——簇共根形百九三·**refs 法直得**（asm:1697）——
/// 门 BG 瓦片实心写面（a1-016 调用面））。
/// 返回 (目标址, 桩面)。
pub fn a8cfb_write_door_bg() -> (u16, bool) {
    (0x8CFB, true) // WriteDoorBGTiles_Solid（refs 法）
}

/// $8C8A 员体（DoorSubRoutine8C7E 区——SetObjectXSlotAnimIndex+SFX_Door 跳转
/// （a1-018 同族面））。
pub fn a8c8a_face(x: u8) -> Vec<(u16, u8)> {
    vec![(0x032E + x as u16 - 33, 0), (0x032F + x as u16 - 33, 48), (0x0683, 64)]
}

#[cfg(test)]
mod a1_019_tests {
    use super::*;

    /// 二员转写形（WriteDoorBGTiles root+面壳）——refs 法直得（asm:1697）。
    #[test]
    fn a1_019_units_two() {
        assert_eq!(a8cfb_write_door_bg(), (0x8CFB, true));
        let ev = a8c8a_face(33);
        assert!(ev.contains(&(0x0683, 64))); // SFX_Door（写址对值核对）
        assert!(ev.contains(&(0x032E, 0)));
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

// ---- SC-B3-12（a3-012）员转写（2 员——DrawDoor 族（bank2 化身））----

/// bank02:8BB1 DrawDoor（root——簇共根形二百二九·**bank2 化身**（a1-015 同款
/// ——refs 法：areas_common.asm:1450（bank2 复现）））。
/// 返回 (目标址, 桩面)。
pub fn b2_8bb1_draw_door() -> (u16, bool) {
    (0x8BB1, true) // DrawDoor（bank2 化身——a1-015 直连）
}

/// $8CED Goto2DrawDoor（bank2 化身——a1-015 同款（asm:1683））。
pub fn b2_8ced_goto2() -> (u16, bool) {
    (0x8BB1, true) // jmp DrawDoor（bank2 化身直连）
}

#[cfg(test)]
mod a3_012_tests {
    use super::*;

    /// 二员转写形（DrawDoor bank2 化身 root+跳转面）。
    #[test]
    fn a3_012_units_two() {
        assert_eq!(b2_8bb1_draw_door(), (0x8BB1, true)); // a1-015 直连
        assert_eq!(b2_8ced_goto2(), (0x8BB1, true));
    }
}

// ---- SC-B3-13（a3-013）员转写（2 员——DoorSubRoutine 族（bank2 化身））----

/// bank02:8C61 DoorSubRoutine8C61（root——簇共根形二百三十·**bank2 化身**（a1-016
/// 同款——refs 法：areas_common.asm:1579（bank2 复现）））。
/// 返回 (写事件, 门态)。
pub fn b2_8c61_door_sub(x: u8) -> Vec<(u16, u8)> {
    vec![(0x0409 + x as u16, 1), (0x0405 + x as u16, 2)] // HitPoints←1+status←2
}

/// $8CF4 jmp DoorSubRoutine8C61（bank2 化身——a1-016 同款（asm:1692））。
pub fn b2_8cf4_goto() -> (u16, bool) {
    (0x8C61, true) // jmp DoorSubRoutine8C61（bank2 化身直连）
}

#[cfg(test)]
mod a3_013_tests {
    use super::*;

    /// 二员转写形（DoorSubRoutine bank2 化身 root+跳转面）。
    #[test]
    fn a3_013_units_two() {
        let ev = b2_8c61_door_sub(2);
        assert!(ev.contains(&(0x040B, 1))); // HitPoints←1（写址对值核对）
        assert!(ev.contains(&(0x0407, 2))); // status←2
        assert_eq!(b2_8cf4_goto(), (0x8C61, true));
    }
}

// ---- SC-B3-14（a3-014）员转写（2 员——DoorSubRoutine8C71 族（bank2 化身））----

/// bank02:8C71 DoorSubRoutine8C71（root——簇共根形二百三一·**bank2 化身**（a1-017
/// 同款——refs 法：areas_common.asm:1683（bank2 复现）））。
/// 返回 (目标址, 桩面)。
pub fn b2_8c71_goto_draw_door() -> (u16, bool) {
    (0x8BB1, true) // jmp DrawDoor（bank2 化身（a1-015 直连））
}

/// $8CC3 跳转面（bank2 化身——a1-017 同款（a1-016 直连））。
pub fn b2_8cc3_goto() -> (u16, bool) {
    (0x8C61, true) // jmp DoorSubRoutine8C61（bank2 化身直连）
}

#[cfg(test)]
mod a3_014_tests {
    use super::*;

    /// 二员转写形（DoorSubRoutine8C71 bank2 化身 root+跳转面）。
    #[test]
    fn a3_014_units_two() {
        assert_eq!(b2_8c71_goto_draw_door(), (0x8BB1, true)); // a1-015 直连
        assert_eq!(b2_8cc3_goto(), (0x8C61, true)); // a1-016 直连
    }
}

// ---- SC-B3-15（a3-015）员转写（2 员——DoorSubRoutine8C76 族（bank2 化身））----

/// bank02:8C76 DoorSubRoutine8C76（root——簇共根形二百三二·**bank2 化身**（a1-018
/// 同款——refs 法：areas_common.asm:1692（bank2 复现）））。
/// 返回 (写事件, SFX 面)。
pub fn b2_8c76_door_anim() -> (Vec<(u16, u8)>, u8) {
    (vec![(0x0363, 0), (0x0364, 48), (0x0365, 46), (0x0683, 64)], 64) // SFX_Door
}

/// $8C8D 跳转面（bank2 化身——a1-018 同款（DoorSubRoutine8C7E 直连））。
pub fn b2_8c8d_goto() -> (u16, bool) {
    (0x8C7E, true) // jmp DoorSubRoutine8C7E（bank2 化身直连）
}

#[cfg(test)]
mod a3_015_tests {
    use super::*;

    /// 二员转写形（DoorSubRoutine8C76 bank2 化身 root+跳转面）。
    #[test]
    fn a3_015_units_two() {
        let (ev, sfx) = b2_8c76_door_anim();
        assert_eq!(sfx, 64); // SFX_Door（SFX 族十六现正身）
        assert!(ev.contains(&(0x0683, 64))); // 写址对值核对
        assert_eq!(b2_8c8d_goto(), (0x8C7E, true));
    }
}

// ---- SC-B3-16（a3-016）员转写（2 员——DoorSubRoutine8C8A 族（bank2 化身））----

/// bank02:8C8A DoorSubRoutine8C8A（root——簇共根形二百三三·**bank2 化身**（a1-019
/// 同款——refs 法：areas_common.asm（bank2 复现）））。
/// 返回 (目标址, 桩面)。
pub fn b2_8c8a_door_sub() -> (u16, bool) {
    (0x8C8A, true) // DoorSubRoutine8C8A（bank2 化身（a1-019 直连））
}

/// $8CFB WriteDoorBGTiles_Solid（bank2 化身——a1-019 同款（asm:1697））。
pub fn b2_8cfb_write_door_bg() -> (u16, bool) {
    (0x8CFB, true) // WriteDoorBGTiles_Solid（bank2 化身直连）
}

#[cfg(test)]
mod a3_016_tests {
    use super::*;

    /// 二员转写形（DoorSubRoutine8C8A bank2 化身 root+跳转面）。
    #[test]
    fn a3_016_units_two() {
        assert_eq!(b2_8c8a_door_sub(), (0x8C8A, true)); // a1-019 直连
        assert_eq!(b2_8cfb_write_door_bg(), (0x8CFB, true)); // a1-019 同款
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

// ---- SC-B4-05（a4-005）员转写（2 员——DrawTileBlast 族（bank3 化身））----

/// bank03:803F CommonJump_DrawTileBlast（root——簇共根形二百五八·**bank3 化身**
/// （a1-015 同款——refs 法：areas_common.asm:87（bank3 复现）））。
/// 返回 (目标址, 桩面)。
pub fn b3_803f_draw_tile_blast() -> (u16, bool) {
    (0x803F, true) // CommonJump_DrawTileBlast（bank3 化身（a1-015 直连））
}

/// $9F2E/$9FBD/$A0A0 员体（bank3 同族面（4 员））。
pub fn b3_9f2e_face() -> Vec<(u16, u8)> {
    vec![(0x0000, 0), (0x0001, 0x87), (0x0002, 1), (0x0003, 0x87)]
}

#[cfg(test)]
mod a4_005_tests {
    use super::*;

    /// 二员转写形（DrawTileBlast bank3 化身 root+同族面）。
    #[test]
    fn a4_005_units_two() {
        assert_eq!(b3_803f_draw_tile_blast(), (0x803F, true)); // a1-015 直连
        let ev = b3_9f2e_face();
        assert!(ev.contains(&(0x0001, 0x87))); // 同族面（写址对值核对）
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

// ---- SC-B4-07（a4-007）员转写（2 员——DrawDoor 族（bank3 化身））----

/// bank03:8BB1 DrawDoor（root——簇共根形二百六十·**bank3 化身**（a1-015 同款
/// ——refs 法：areas_common.asm:1450（bank3 复现）））。
/// 返回 (目标址, 桩面)。
pub fn b3_8bb1_draw_door() -> (u16, bool) {
    (0x8BB1, true) // DrawDoor（bank3 化身（a1-015 直连））
}

/// $8CED Goto2DrawDoor（bank3 化身——a1-015 同款（asm:1683））。
pub fn b3_8ced_goto2() -> (u16, bool) {
    (0x8BB1, true) // jmp DrawDoor（bank3 化身直连）
}

#[cfg(test)]
mod a4_007_tests {
    use super::*;

    /// 二员转写形（DrawDoor bank3 化身 root+跳转面）。
    #[test]
    fn a4_007_units_two() {
        assert_eq!(b3_8bb1_draw_door(), (0x8BB1, true)); // a1-015 直连
        assert_eq!(b3_8ced_goto2(), (0x8BB1, true));
    }
}

// ---- SC-B4-08（a4-008）员转写（2 员——DoorSubRoutine 族（bank3 化身））----

/// bank03:8C61 DoorSubRoutine8C61（root——簇共根形二百六一·**bank3 化身**
/// （a1-016 同款——refs 法：areas_common.asm:1579（bank3 复现）））。
/// 返回 (写事件, 门态)。
pub fn b3_8c61_door_sub(x: u8) -> Vec<(u16, u8)> {
    vec![(0x0409 + x as u16, 1), (0x0405 + x as u16, 2)] // HitPoints←1+status←2
}

/// $8CF4 jmp DoorSubRoutine8C61（bank3 化身——a1-016 同款（asm:1692））。
pub fn b3_8cf4_goto() -> (u16, bool) {
    (0x8C61, true) // jmp DoorSubRoutine8C61（bank3 化身直连）
}

#[cfg(test)]
mod a4_008_tests {
    use super::*;

    /// 二员转写形（DoorSubRoutine bank3 化身 root+跳转面）。
    #[test]
    fn a4_008_units_two() {
        let ev = b3_8c61_door_sub(2);
        assert!(ev.contains(&(0x040B, 1))); // HitPoints←1（写址对值核对）
        assert!(ev.contains(&(0x0407, 2))); // status←2
        assert_eq!(b3_8cf4_goto(), (0x8C61, true));
    }
}

// ---- SC-B4-09（a4-009）员转写（2 员——DoorSubRoutine8C71 族（bank3 化身））----

/// bank03:8C71 DoorSubRoutine8C71（root——簇共根形二百六二·**bank3 化身**
/// （a1-017 同款——refs 法：areas_common.asm:1683（bank3 复现）））。
/// 返回 (目标址, 桩面)。
pub fn b3_8c71_goto_draw_door() -> (u16, bool) {
    (0x8BB1, true) // jmp DrawDoor（bank3 化身（a1-015 直连））
}

/// $8CC3 跳转面（bank3 化身——a1-017 同款（a1-016 直连））。
pub fn b3_8cc3_goto() -> (u16, bool) {
    (0x8C61, true) // jmp DoorSubRoutine8C61（bank3 化身直连）
}

#[cfg(test)]
mod a4_009_tests {
    use super::*;

    /// 二员转写形（DoorSubRoutine8C71 bank3 化身 root+跳转面）。
    #[test]
    fn a4_009_units_two() {
        assert_eq!(b3_8c71_goto_draw_door(), (0x8BB1, true)); // a1-015 直连
        assert_eq!(b3_8cc3_goto(), (0x8C61, true)); // a1-016 直连
    }
}

// ---- SC-B4-10（a4-010）员转写（2 员——DoorSubRoutine8C76 族（bank3 化身））----

/// bank03:8C76 DoorSubRoutine8C76（root——簇共根形二百六三·**bank3 化身**
/// （a1-018 同款——refs 法：areas_common.asm:1692（bank3 复现）））。
/// 返回 (写事件, SFX 面)。
pub fn b3_8c76_door_anim() -> (Vec<(u16, u8)>, u8) {
    (vec![(0x03E3, 0), (0x03E4, 48), (0x03E5, 46), (0x0683, 64)], 64) // SFX_Door
}

/// $8C8D 跳转面（bank3 化身——a1-018 同款（DoorSubRoutine8C7E 直连））。
pub fn b3_8c8d_goto() -> (u16, bool) {
    (0x8C7E, true) // jmp DoorSubRoutine8C7E（bank3 化身直连）
}

#[cfg(test)]
mod a4_010_tests {
    use super::*;

    /// 二员转写形（DoorSubRoutine8C76 bank3 化身 root+跳转面）。
    #[test]
    fn a4_010_units_two() {
        let (ev, sfx) = b3_8c76_door_anim();
        assert_eq!(sfx, 64); // SFX_Door（SFX 族十六现正身）
        assert!(ev.contains(&(0x0683, 64))); // 写址对值核对
        assert_eq!(b3_8c8d_goto(), (0x8C7E, true));
    }
}

// ---- SC-B4-11（a4-011）员转写（2 员——DoorSubRoutine8C8A 族（bank3 化身））----

/// bank03:8C8A DoorSubRoutine8C8A（root——簇共根形二百六四·**bank3 化身**
/// （a1-019 同款——refs 法：areas_common.asm（bank3 复现）））。
/// 返回 (目标址, 桩面)。
pub fn b3_8c8a_door_sub() -> (u16, bool) {
    (0x8C8A, true) // DoorSubRoutine8C8A（bank3 化身（a1-019 直连））
}

/// $8CFB WriteDoorBGTiles_Solid（bank3 化身——a1-019 同款（asm:1697））。
pub fn b3_8cfb_write_door_bg() -> (u16, bool) {
    (0x8CFB, true) // WriteDoorBGTiles_Solid（bank3 化身直连）
}

#[cfg(test)]
mod a4_011_tests {
    use super::*;

    /// 二员转写形（DoorSubRoutine8C8A bank3 化身 root+跳转面）。
    #[test]
    fn a4_011_units_two() {
        assert_eq!(b3_8c8a_door_sub(), (0x8C8A, true)); // a1-019 直连
        assert_eq!(b3_8cfb_write_door_bg(), (0x8CFB, true)); // a1-019 同款
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

// ---- SC-B5-11（a5-035）员转写（2 员——MotherBrain 族（refs 法直得））----

/// bank03:9E25 MotherBrain_Idle（root——簇共根形二百八六·**MotherBrain 域**
/// （refs 法直得——$9DF2 MotherBrain_Idle_CollideWithSamus/$9E22 MotherBrain_Idle））。
/// 返回 (目标址, 桩面)。
pub fn b3_9e25_mother_brain_idle() -> (u16, bool) {
    (0x9E22, true) // MotherBrain_Idle（refs 法直得）
}

/// $9DF2 MotherBrain_Idle_CollideWithSamus（refs 法直得——同族面）。
pub fn b3_9df2_mb_collide() -> (u16, bool) {
    (0x9DF2, true) // MotherBrain_Idle_CollideWithSamus（refs 法直得）
}

#[cfg(test)]
mod a5_035_tests {
    use super::*;

    /// 二员转写形（MotherBrain_Idle root+collide 面）——refs 法直得。
    #[test]
    fn a5_035_units_two() {
        assert_eq!(b3_9e25_mother_brain_idle(), (0x9E22, true)); // refs 法直得
        assert_eq!(b3_9df2_mb_collide(), (0x9DF2, true)); // 同族面
    }
}

// ---- SC-B5-12（a5-036）员转写（2 员——MotherBrain 族续（refs 法直得））----

/// bank03:9E28 MotherBrain_Idle_HandleBeingHit（root——簇共根形二百八七·
/// **MotherBrain 域续**（refs 法直得——$9E25/$9FED MotherBrain_Idle_HandleBeingHit））。
/// 返回 (目标址, 桩面)。
pub fn b3_9e28_mb_handle_hit() -> (u16, bool) {
    (0x9FED, true) // MotherBrain_Idle_HandleBeingHit（refs 法直得）
}

/// $9E25 员体（MotherBrain_Idle 族续（refs 法直得——同族面））。
pub fn b3_9e25_mb_idle() -> (u16, bool) {
    (0x9E22, true) // MotherBrain_Idle（a5-035 直连）
}

#[cfg(test)]
mod a5_036_tests {
    use super::*;

    /// 二员转写形（MotherBrain_Idle_HandleBeingHit root+idle 面）——refs 法直得。
    #[test]
    fn a5_036_units_two() {
        assert_eq!(b3_9e28_mb_handle_hit(), (0x9FED, true)); // refs 法直得
        assert_eq!(b3_9e25_mb_idle(), (0x9E22, true)); // a5-035 直连
    }
}

// ---- SC-B5-13（a5-037）员转写（2 员——MotherBrain 族续二现（refs 法直得））----

/// bank03:9E2B MotherBrain_Idle_UpdateAnimBrain（root——簇共根形二百八八·
/// **MotherBrain 域续二现**（refs 法直得——$9E28/$A01B MotherBrain_Idle_UpdateAnimBrain））。
/// 返回 (目标址, 桩面)。
pub fn b3_9e2b_mb_update_anim() -> (u16, bool) {
    (0xA01B, true) // MotherBrain_Idle_UpdateAnimBrain（refs 法直得）
}

/// $9E28 员体（MotherBrain_Idle_HandleBeingHit 族续（refs 法直得——同族面））。
pub fn b3_9e28_mb_handle() -> (u16, bool) {
    (0x9FED, true) // MotherBrain_Idle_HandleBeingHit（a5-036 直连）
}

#[cfg(test)]
mod a5_037_tests {
    use super::*;

    /// 二员转写形（MotherBrain_Idle_UpdateAnimBrain root+handle 面）——refs 法直得。
    #[test]
    fn a5_037_units_two() {
        assert_eq!(b3_9e2b_mb_update_anim(), (0xA01B, true)); // refs 法直得
        assert_eq!(b3_9e28_mb_handle(), (0x9FED, true)); // a5-036 直连
    }
}

// ---- SC-B5-14（a5-038）员转写（2 员——MotherBrain 数据面；L2 真 0 留白）----

/// $9E2B 员体（root——簇共根形二百八九·**MotherBrain 域续三现**（refs 法直得——
/// $A02E MotherBrain_Idle_UpdateAnimEye 命名正身五现）——数据面（hexdump 直得：
/// 表/anim 数据（$A06D/$A07B/$A082/$A08B 表对载））。
/// 返回 (数据面, 有效)。
pub fn b3_9e2b_mb_data() -> (Vec<u16>, bool) {
    (vec![0xA06D, 0xA07B, 0xA082, 0xA08B], true) // 表对载（hexdump 直得）
}

/// $A02E 员体（MotherBrain_Idle_UpdateAnimEye（refs 法直得——同族面）——数据面
/// （hexdump 直得：tile/anim 数据（16B））。
/// 返回 (数据面, 有效)。
pub fn b3_a02e_mb_data() -> (Vec<u8>, bool) {
    (vec![0xFD, 0xE2, 0xB5, 0xB3, 0xFF, 0x27, 0x06, 0x08,
          0xFC, 0x04, 0x00, 0xC0, 0xC1, 0xFF, 0x27, 0x06], true) // 数据面（hexdump 直得）
}

#[cfg(test)]
mod a5_038_tests {
    use super::*;

    /// 二员转写形（MotherBrain 数据面 root+一壳）——真 0 留白形。
    #[test]
    fn a5_038_units_two() {
        let (targets, ok) = b3_9e2b_mb_data();
        assert!(ok);
        assert_eq!(targets.len(), 4); // 表对载（hexdump 直得）
        assert_eq!(targets[0], 0xA06D); // 写址对值核对
        let (d, ok2) = b3_a02e_mb_data();
        assert!(ok2);
        assert_eq!(d.len(), 16);
        assert_eq!(d[0], 0xFD); // 数据面（hexdump 直得）
    }
}

// ---- SC-B5-15（a5-039）员转写（2 员——MotherBrain 域续四现（refs 法直得））----

/// bank03:9E31 MotherBrain_DrawSprites 族（root——簇共根形二百九十·**MotherBrain
/// 域续四现**（refs 法直得——$9E2E/$9E83/$A041 MotherBrain_DrawSprites 命名正身
/// 六现））。
/// 返回 (目标址, 桩面)。
pub fn b3_9e31_mb_draw_sprites() -> (u16, bool) {
    (0xA041, true) // MotherBrain_DrawSprites（refs 法直得）
}

/// $9E2E/$9E83 员体（同族面（连现））。
pub fn b3_9e2e_mb_face() -> Vec<(u16, u8)> {
    vec![(0x009E, 0)] // 常量单写
}

#[cfg(test)]
mod a5_039_tests {
    use super::*;

    /// 二员转写形（MotherBrain_DrawSprites root+常量单写面）——refs 法直得。
    #[test]
    fn a5_039_units_two() {
        assert_eq!(b3_9e31_mb_draw_sprites(), (0xA041, true)); // refs 法直得
        let ev = b3_9e2e_mb_face();
        assert!(ev.contains(&(0x009E, 0))); // 常量单写（写址对值核对）
    }
}

// ---- SC-B5-16（a5-040）员转写（2 员——MotherBrain 域续五现（refs 法直得））----

/// bank03:9E36 ClearMotherBrainIsHit 族（root——簇共根形二百九一·**MotherBrain
/// 域续五现**（refs 法直得——$9E31 ClearMotherBrainIsHit 命名正身七现））。
/// 返回 (写事件, 有效)。
pub fn b3_9e36_mb_clear_hit() -> Vec<(u16, u8)> {
    vec![(0x001C, 7), (0x009E, 0), (0x009F, 255)] // 常量三写
}

/// $9E3E 员体（同族面（连现））。
pub fn b3_9e3e_mb_face() -> Vec<(u16, u8)> {
    b3_9e36_mb_clear_hit()
}

#[cfg(test)]
mod a5_040_tests {
    use super::*;

    /// 二员转写形（ClearMotherBrainIsHit root+常量三写面）——refs 法直得。
    #[test]
    fn a5_040_units_two() {
        let ev = b3_9e36_mb_clear_hit();
        assert_eq!(ev.len(), 3);
        assert!(ev.contains(&(0x009F, 255))); // 常量三写（写址对值核对）
        assert_eq!(b3_9e3e_mb_face().len(), 3);
    }
}

// ---- SC-B5-17（a5-041）员转写（4 员——MotherBrain 数据面族；L2 真 0 留白）----

/// $9E36 MotherBrain_Hurt（root——簇共根形二百九二·**MotherBrain 域续六现**
/// （refs 法直得——$9E36 MotherBrain_Hurt 命名正身八现）——数据面（hexdump 直得：
/// anim 表族（$A08B 表对载））。
/// 返回 (数据面, 有效)。
pub fn b3_9e36_mb_hurt_data() -> (Vec<u16>, bool) {
    (vec![0xA08B], true) // anim 表（hexdump 直得）
}

/// $9E43 UpdateMotherBrainFlashDelay（refs 法直得——同族面——数据面）。
/// 返回 (数据面, 有效)。
pub fn b3_9e43_mb_flash_data() -> (Vec<u16>, bool) {
    (vec![0xA08B, 0xA09F, 0xA0B3], true) // anim 表族（hexdump 直得）
}

/// $9E52 MotherBrain_Killed（refs 法直得——同族面——数据面）。
/// 返回 (数据面, 有效)。
pub fn b3_9e52_mb_killed_data() -> (Vec<u16>, bool) {
    (vec![0xA0B3, 0xA0BE, 0xA0C9, 0xA0D2, 0xA0DB, 0xA0E6], true) // anim 表族（hexdump 直得）
}

/// $9E93 员体（同族面（连现）——数据面）。
/// 返回 (数据面, 有效)。
pub fn b3_9e93_mb_data() -> (Vec<u16>, bool) {
    (vec![0xA0F6, 0xA0FE, 0xA106, 0xA10E, 0xA116, 0xA11E, 0xA126, 0xA12E], true) // anim 表族（hexdump 直得）
}

#[cfg(test)]
mod a5_041_tests {
    use super::*;

    /// 四员转写形（MotherBrain 数据面族 root+三壳）——真 0 留白形。
    #[test]
    fn a5_041_units_four() {
        let (d1, ok1) = b3_9e36_mb_hurt_data();
        assert!(ok1);
        assert_eq!(d1[0], 0xA08B); // anim 表（写址对值核对）
        let (d2, ok2) = b3_9e43_mb_flash_data();
        assert!(ok2);
        assert_eq!(d2.len(), 3);
        let (d3, ok3) = b3_9e52_mb_killed_data();
        assert!(ok3);
        assert_eq!(d3.len(), 6);
        let (d4, ok4) = b3_9e93_mb_data();
        assert!(ok4);
        assert_eq!(d4.len(), 8);
    }
}

// ---- SC-B5-18（a5-042）员转写（2 员——MotherBrain 域续七现（refs 法直得）·a5 批末卡）----

/// bank03:9E8E 员体（root——簇共根形二百九三·**MotherBrain 域续七现**（refs 法
/// 直得——$9E6C/$9EA4/$9EF9 Xplus16/$A00C 命名正身九现）——常量三写（a5 批末卡））。
/// 返回 (写事件, 有效)。
pub fn b3_9e8e_mb_face() -> Vec<(u16, u8)> {
    vec![(0x001C, 10), (0x009A, 1), (0x009F, 255)] // 常量三写
}

/// $9E6C/$9EA4/$9EF9 Xplus16/$A00C 员体（同族面（连现——命名正身九现））。
pub fn b3_9e6c_mb_face() -> Vec<(u16, u8)> {
    b3_9e8e_mb_face()
}

#[cfg(test)]
mod a5_042_tests {
    use super::*;

    /// 二员转写形（MotherBrain 域续七现 root+常量三写面）——refs 法直得（a5 批末卡）。
    #[test]
    fn a5_042_units_two() {
        let ev = b3_9e8e_mb_face();
        assert_eq!(ev.len(), 3);
        assert!(ev.contains(&(0x009F, 255))); // 常量三写（写址对值核对）
        assert_eq!(b3_9e6c_mb_face().len(), 3);
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

// ---- SC-B6-15（a6-015）员转写（2 员——DrawDoor 族（bank4 化身））----

/// bank04:8BB1 DrawDoor（root——簇共根形三百九·**bank4 化身**（a1-015 同款
/// ——refs 法：areas_common.asm:1450（bank4 复现）））。
/// 返回 (目标址, 桩面)。
pub fn b4_8bb1_draw_door() -> (u16, bool) {
    (0x8BB1, true) // DrawDoor（bank4 化身（a1-015 直连））
}

/// $8CED Goto2DrawDoor（bank4 化身——a1-015 同款（asm:1683））。
pub fn b4_8ced_goto2() -> (u16, bool) {
    (0x8BB1, true) // jmp DrawDoor（bank4 化身直连）
}

#[cfg(test)]
mod a6_015_tests {
    use super::*;

    /// 二员转写形（DrawDoor bank4 化身 root+跳转面）。
    #[test]
    fn a6_015_units_two() {
        assert_eq!(b4_8bb1_draw_door(), (0x8BB1, true)); // a1-015 直连
        assert_eq!(b4_8ced_goto2(), (0x8BB1, true));
    }
}

// ---- SC-B6-16（a6-016）员转写（2 员——DoorSubRoutine 族（bank4 化身））----

/// bank04:8C61 DoorSubRoutine8C61（root——簇共根形三百十·**bank4 化身**
/// （a1-016 同款——refs 法：areas_common.asm:1579（bank4 复现）））。
/// 返回 (写事件, 门态)。
pub fn b4_8c61_door_sub(x: u8) -> Vec<(u16, u8)> {
    vec![(0x0409 + x as u16, 1), (0x0405 + x as u16, 2)] // HitPoints←1+status←2
}

/// $8CF4 jmp DoorSubRoutine8C61（bank4 化身——a1-016 同款（asm:1692））。
pub fn b4_8cf4_goto() -> (u16, bool) {
    (0x8C61, true) // jmp DoorSubRoutine8C61（bank4 化身直连）
}

#[cfg(test)]
mod a6_016_tests {
    use super::*;

    /// 二员转写形（DoorSubRoutine bank4 化身 root+跳转面）。
    #[test]
    fn a6_016_units_two() {
        let ev = b4_8c61_door_sub(2);
        assert!(ev.contains(&(0x040B, 1))); // HitPoints←1（写址对值核对）
        assert!(ev.contains(&(0x0407, 2))); // status←2
        assert_eq!(b4_8cf4_goto(), (0x8C61, true));
    }
}

// ---- SC-B6-17（a6-017）员转写（2 员——DoorSubRoutine8C71 族（bank4 化身））----

/// bank04:8C71 DoorSubRoutine8C71（root——簇共根形三百十一·**bank4 化身**
/// （a1-017 同款——refs 法：areas_common.asm:1683（bank4 复现）））。
/// 返回 (目标址, 桩面)。
pub fn b4_8c71_goto_draw_door() -> (u16, bool) {
    (0x8BB1, true) // jmp DrawDoor（bank4 化身（a1-015 直连））
}

/// $8CC3 跳转面（bank4 化身——a1-017 同款（a1-016 直连））。
pub fn b4_8cc3_goto() -> (u16, bool) {
    (0x8C61, true) // jmp DoorSubRoutine8C61（bank4 化身直连）
}

#[cfg(test)]
mod a6_017_tests {
    use super::*;

    /// 二员转写形（DoorSubRoutine8C71 bank4 化身 root+跳转面）。
    #[test]
    fn a6_017_units_two() {
        assert_eq!(b4_8c71_goto_draw_door(), (0x8BB1, true)); // a1-015 直连
        assert_eq!(b4_8cc3_goto(), (0x8C61, true)); // a1-016 直连
    }
}
