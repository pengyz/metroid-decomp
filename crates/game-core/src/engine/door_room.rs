//! `game-core::engine::door_room` —— door_room 族簇（P2 批 3（纯机械移动））。
//! refs 锚内随段。

use super::*;

// ---- SC-S1-16 员转写（5 员——CheckForItem 族；L2 真 0 留白）----

/// $EE4A CheckForItem（root——簇共根形九）：$6886 计数环（Y 递减 2 步——$07/$06 对
/// 比 $6886[y]/$6885[y]）——**carry=找到**（clс 找到/... sec 未找形注）。
/// 返回 found。
pub fn s1_check_for_item(y0: u8, v06: u8, v07: u8, item_lo: &[u8], item_hi: &[u8]) -> bool {
    // 【BUG 更正（2026-10-23）】原 `while y>0 + wrapping_sub(2)` 对 odd y0 无命中=死环
    // （移植错（vs asm `dey/dey/bpl` 有界（signed y>=0））——D3 ROM 真相径修正）。
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

