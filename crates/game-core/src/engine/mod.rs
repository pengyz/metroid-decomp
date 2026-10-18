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
    let v = (v12 | v16) << 1;
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
