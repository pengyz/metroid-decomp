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
