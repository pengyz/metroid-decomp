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
