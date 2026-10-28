//! `game-core::engine::door_room_face` —— 门/房/物品/爆破瓦投影（P2 重整合（修正后域维度（2026-10-28 覆写）））。
//! asm 例程族=节注边界（族头保留（考证锚不丢））。

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

// ---- SC-S7-00（S7-175）员转写（2 员——GetEnemyRoomRAMPtr 族）----


// ---- SC-S7-05（S7-180）员转写（2 员——DrawTileBlast 族·S7 收官卡）----


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

// ---- SC-B6-18（a6-018）员转写（2 员——DoorSubRoutine8C76 族（bank4 化身））----

/// bank04:8C76 DoorSubRoutine8C76（root——簇共根形三百十二·**bank4 化身**
/// （a1-018 同款——refs 法：areas_common.asm:1692（bank4 复现）））。
/// 返回 (写事件, SFX 面)。
pub fn b4_8c76_door_anim() -> (Vec<(u16, u8)>, u8) {
    (vec![(0x0392, 0), (0x0393, 48), (0x0394, 46), (0x0683, 64)], 64) // SFX_Door
}

/// $8C8D 跳转面（bank4 化身——a1-018 同款（DoorSubRoutine8C7E 直连））。
pub fn b4_8c8d_goto() -> (u16, bool) {
    (0x8C7E, true) // jmp DoorSubRoutine8C7E（bank4 化身直连）
}

#[cfg(test)]
mod a6_018_tests {
    use super::*;

    /// 二员转写形（DoorSubRoutine8C76 bank4 化身 root+跳转面）。
    #[test]
    fn a6_018_units_two() {
        let (ev, sfx) = b4_8c76_door_anim();
        assert_eq!(sfx, 64); // SFX_Door（SFX 族十六现正身）
        assert!(ev.contains(&(0x0683, 64))); // 写址对值核对
        assert_eq!(b4_8c8d_goto(), (0x8C7E, true));
    }
}

// ---- SC-B6-19（a6-019）员转写（2 员——DoorSubRoutine8C8A 族（bank4 化身））----

/// bank04:8C8A DoorSubRoutine8C8A（root——簇共根形三百十三·**bank4 化身**
/// （a1-019 同款——refs 法：areas_common.asm（bank4 复现）））。
/// 返回 (目标址, 桩面)。
pub fn b4_8c8a_door_sub() -> (u16, bool) {
    (0x8C8A, true) // DoorSubRoutine8C8A（bank4 化身（a1-019 直连））
}

/// $8CFB WriteDoorBGTiles_Solid（bank4 化身——a1-019 同款（asm:1697））。
pub fn b4_8cfb_write_door_bg() -> (u16, bool) {
    (0x8CFB, true) // WriteDoorBGTiles_Solid（bank4 化身直连）
}

#[cfg(test)]
mod a6_019_tests {
    use super::*;

    /// 二员转写形（DoorSubRoutine8C8A bank4 化身 root+跳转面）。
    #[test]
    fn a6_019_units_two() {
        assert_eq!(b4_8c8a_door_sub(), (0x8C8A, true)); // a1-019 直连
        assert_eq!(b4_8cfb_write_door_bg(), (0x8CFB, true)); // a1-019 同款
    }
}

// ---- SC-B8-13（a8-013）员转写（2 员——DrawDoor 族（bank6 化身））----

/// bank05:8BB1 DrawDoor（root——簇共根形三百四八·**bank6 化身**（a1-015 同款
/// ——refs 法：areas_common.asm:1450（bank6 复现）））。
/// 返回 (目标址, 桩面)。
pub fn b6_8bb1_draw_door() -> (u16, bool) {
    (0x8BB1, true) // DrawDoor（bank6 化身（a1-015 直连））
}

/// $8CED Goto2DrawDoor（bank6 化身——a1-015 同款（asm:1683））。
pub fn b6_8ced_goto2() -> (u16, bool) {
    (0x8BB1, true) // jmp DrawDoor（bank6 化身直连）
}

#[cfg(test)]
mod a8_013_tests {
    use super::*;

    /// 二员转写形（DrawDoor bank6 化身 root+跳转面）。
    #[test]
    fn a8_013_units_two() {
        assert_eq!(b6_8bb1_draw_door(), (0x8BB1, true)); // a1-015 直连
        assert_eq!(b6_8ced_goto2(), (0x8BB1, true));
    }
}

// ---- SC-B8-14（a8-014）员转写（2 员——DoorSubRoutine 族（bank6 化身））----

/// bank05:8C61 DoorSubRoutine8C61（root——簇共根形三百四九·**bank6 化身**
/// （a1-016 同款——refs 法：areas_common.asm:1579（bank6 复现）））。
/// 返回 (写事件, 门态)。
pub fn b6_8c61_door_sub(x: u8) -> Vec<(u16, u8)> {
    vec![(0x0409 + x as u16, 1), (0x0405 + x as u16, 2)] // HitPoints←1+status←2
}

/// $8CF4 jmp DoorSubRoutine8C61（bank6 化身——a1-016 同款（asm:1692））。
pub fn b6_8cf4_goto() -> (u16, bool) {
    (0x8C61, true) // jmp DoorSubRoutine8C61（bank6 化身直连）
}

#[cfg(test)]
mod a8_014_tests {
    use super::*;

    /// 二员转写形（DoorSubRoutine bank6 化身 root+跳转面）。
    #[test]
    fn a8_014_units_two() {
        let ev = b6_8c61_door_sub(2);
        assert!(ev.contains(&(0x040B, 1))); // HitPoints←1（写址对值核对）
        assert!(ev.contains(&(0x0407, 2))); // status←2
        assert_eq!(b6_8cf4_goto(), (0x8C61, true));
    }
}

// ---- SC-B8-15（a8-015）员转写（2 员——DoorSubRoutine8C71 族（bank6 化身））----

/// bank05:8C71 DoorSubRoutine8C71（root——簇共根形三百五十·**bank6 化身**
/// （a1-017 同款——refs 法：areas_common.asm:1683（bank6 复现）））。
/// 返回 (目标址, 桩面)。
pub fn b6_8c71_goto_draw_door() -> (u16, bool) {
    (0x8BB1, true) // jmp DrawDoor（bank6 化身（a1-015 直连））
}

/// $8CC3 跳转面（bank6 化身——a1-017 同款（a1-016 直连））。
pub fn b6_8cc3_goto() -> (u16, bool) {
    (0x8C61, true) // jmp DoorSubRoutine8C61（bank6 化身直连）
}

#[cfg(test)]
mod a8_015_tests {
    use super::*;

    /// 二员转写形（DoorSubRoutine8C71 bank6 化身 root+跳转面）。
    #[test]
    fn a8_015_units_two() {
        assert_eq!(b6_8c71_goto_draw_door(), (0x8BB1, true)); // a1-015 直连
        assert_eq!(b6_8cc3_goto(), (0x8C61, true)); // a1-016 直连
    }
}

// ---- SC-B8-16（a8-016）员转写（2 员——DoorSubRoutine8C76 族（bank6 化身））----

/// bank05:8C76 DoorSubRoutine8C76（root——簇共根形三百五一·**bank6 化身**
/// （a1-018 同款——refs 法：areas_common.asm:1692（bank6 复现）））。
/// 返回 (写事件, SFX 面)。
pub fn b6_8c76_door_anim() -> (Vec<(u16, u8)>, u8) {
    (vec![(0x0324, 0), (0x0325, 48), (0x0326, 46), (0x0683, 64)], 64) // SFX_Door
}

/// $8C8D 跳转面（bank6 化身——a1-018 同款（DoorSubRoutine8C7E 直连））。
pub fn b6_8c8d_goto() -> (u16, bool) {
    (0x8C7E, true) // jmp DoorSubRoutine8C7E（bank6 化身直连）
}

#[cfg(test)]
mod a8_016_tests {
    use super::*;

    /// 二员转写形（DoorSubRoutine8C76 bank6 化身 root+跳转面）。
    #[test]
    fn a8_016_units_two() {
        let (ev, sfx) = b6_8c76_door_anim();
        assert_eq!(sfx, 64); // SFX_Door（SFX 族十六现正身）
        assert!(ev.contains(&(0x0683, 64))); // 写址对值核对
        assert_eq!(b6_8c8d_goto(), (0x8C7E, true));
    }
}

// ---- SC-B8-17（a8-017）员转写（2 员——DoorSubRoutine8C8A 族（bank6 化身）·a8 批末卡段）----

/// bank05:8C8A DoorSubRoutine8C8A（root——簇共根形三百五二·**bank6 化身**
/// （a1-019 同款——refs 法：areas_common.asm（bank6 复现）——a8 批末卡段））。
/// 返回 (目标址, 桩面)。
pub fn b6_8c8a_door_sub() -> (u16, bool) {
    (0x8C8A, true) // DoorSubRoutine8C8A（bank6 化身（a1-019 直连））
}

/// $8CFB WriteDoorBGTiles_Solid（bank6 化身——a1-019 同款（asm:1697））。
pub fn b6_8cfb_write_door_bg() -> (u16, bool) {
    (0x8CFB, true) // WriteDoorBGTiles_Solid（bank6 化身直连）
}

#[cfg(test)]
mod a8_017_tests {
    use super::*;

    /// 二员转写形（DoorSubRoutine8C8A bank6 化身 root+跳转面）——a8 批末卡段。
    #[test]
    fn a8_017_units_two() {
        assert_eq!(b6_8c8a_door_sub(), (0x8C8A, true)); // a1-019 直连
        assert_eq!(b6_8cfb_write_door_bg(), (0x8CFB, true)); // a1-019 同款
    }
}

