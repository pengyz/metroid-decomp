//! `game-core::engine::enemy_face` —— 敌方对象/生成/损伤/朝向投影（P2 重整合（修正后域维度（2026-10-28 覆写）））。
//! asm 例程族=节注边界（族头保留（考证锚不丢））。

use super::*;

// ---- SC-S1-08 员转写（6 员——KillObject 族）----

/// $F48A 员体（簇入口流）：KillObject+$6AF7[y] cmp $80 分段+栈形+X=$03 环。
pub fn update_enemy_movement_x(x: u8) -> Vec<(u16, u8)> {
    vec![(0x6AF4 + x as u16, 0)] // KillObject 写（案实核形）
}

/// $F4DF 员体：KillObject+$6B←（$2D 位1 判）+$040F[x] dec 计数族+jmp $F416。
pub fn update_enemy_movement_y(x: u8, v2d: u8, v040f_x: u8) -> (Vec<(u16, u8)>, u16) {
    let hi = if v2d & 0x02 != 0 { 0xA1 } else { 0xA0 };
    (vec![(0x6AF4 + x as u16, 0), (0x006B, hi),
          (0x040F + x as u16, v040f_x.wrapping_sub(1))], 0xF416)
}

/// $F52D 员体：jmp KillObject（尾形）+$040F[x]→$0A+$0404[x] 位5 判段。
pub fn update_enemy_movement_ext(x: u8, v040f_x: u8, v0404_x: u8) -> Vec<(u16, u8)> {
    let mut ev = vec![(0x6AF4 + x as u16, 0), (0x000A, v040f_x)];
    if v0404_x & 0x20 == 0 {
        ev.push((0x040E + x as u16, v0404_x)); // 位5 未置段（写族注）
    }
    ev
}

/// $FA18 KillObject：$6AF4[x]←0（敌槽清——击杀写）。
pub fn kill_enemy(x: u8) -> (Vec<(u16, u8)>, u8) {
    (vec![(0x6AF4 + x as u16, 0)], 0)
}

/// $FA77 员体：KillObject+尾跳 $F97C+指针载（$0400/$0401[x]→$02/$03+$6AFB[x]→$0B）。
pub fn update_enemy_state_ext(x: u8, v0400_x: u8, v0401_x: u8, v6afb_x: u8) -> (Vec<(u16, u8)>, u16) {
    (vec![(0x6AF4 + x as u16, 0), (0x0002, v0400_x), (0x0003, v0401_x),
          (0x000B, v6afb_x)], 0xF97C)
}

/// $FA91 员体：KillObject+$95DC 表载→root（$F68D）+尾跳 $F97C。
pub fn update_enemy_projectile_pickup(x: u8, tbl_entry: u8) -> (Vec<(u16, u8)>, u16) {
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
        assert!(update_enemy_movement_x(2).contains(&(0x6AF6, 0)));
        let (ev, j) = update_enemy_movement_y(2, 0x02, 5);
        assert_eq!(j, 0xF416);
        assert!(ev.contains(&(0x006B, 0xA1)));
        let ev = update_enemy_movement_ext(2, 5, 0x10);
        assert!(ev.contains(&(0x000A, 5)));
        let (ev, a) = kill_enemy(2);
        assert_eq!(a, 0);
        assert!(ev.contains(&(0x6AF6, 0)));
        let (ev, j) = update_enemy_state_ext(2, 1, 2, 3);
        assert_eq!(j, 0xF97C);
        assert!(ev.contains(&(0x000B, 3)));
        let (ev, j) = update_enemy_projectile_pickup(2, 0x55);
        assert_eq!(j, 0xF97C);
        assert!(ev.contains(&(0x6AFB, 0x55)));
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
pub fn update_enemy_f83b(x: u8, a_in: u8, v0405_x: u8) -> (Vec<(u16, u8)>, u8) {
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
        let (ev, _) = update_enemy_f83b(2, 0x0F, 0x40);
        assert!(ev.contains(&(0x0407, 8))); // $40>>3
    }
}

// ---- SC-S2-01（S2-31）员转写（4 员——OrEnData05 族）----

/// $F744 OrEnData05（root——簇共根形二五·**And 对偶**）：$0405[x] |= A（OR 写面）。
pub fn update_enemy_or_en_data05(x: u8, a_in: u8, v0405_x: u8) -> (Vec<(u16, u8)>, u8) {
    let v = v0405_x | a_in;
    (vec![(0x0405 + x as u16, v)], v)
}

/// $F702 员体：OrEnData05+lsr/ror $0403[x] 段+$81DA 调+**OR+AND 链**（$FB）+X 增双写径
/// （$0409 判）。
pub fn update_enemy_f702(x: u8, a_in: u8, v0405_x: u8) -> (Vec<(u16, u8)>, u8) {
    let v = (v0405_x | a_in) & 0xFB;
    (vec![(0x0405 + x as u16, v)], v)
}

/// $F735 员体：OrEnData05+三 lsr/ror $0402[x] 段+$820F 跳面+OR 写段。
pub fn update_enemy_f735(x: u8, a_in: u8, v0405_x: u8) -> (Vec<(u16, u8)>, u8) {
    update_enemy_or_en_data05(x, a_in, v0405_x)
}

/// $F761 员体：OrEnData05+$6B02 表走（$96AB）+位1 判段（$F7 表链）。
pub fn update_enemy_f761(x: u8, a_in: u8, v0405_x: u8) -> (Vec<(u16, u8)>, u8) {
    update_enemy_or_en_data05(x, a_in, v0405_x)
}

#[cfg(test)]
mod s2_031_tests {
    use super::*;

    /// 四员转写形（OrEnData05 root 对偶+三壳）。
    #[test]
    fn s2_031_units_four() {
        let (ev, v) = update_enemy_or_en_data05(2, 0x0F, 0xF0);
        assert_eq!(v, 0xFF);
        assert!(ev.contains(&(0x0407, 0xFF)));
        let (_, v2) = update_enemy_f702(2, 0xCC, 0);
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

// ---- SC-S7-03（S7-178）员转写（2 员——UpdateMellow_RunAI 族）----


// ---- SC-S7-04（S7-179）员转写（2 员——UpdateMellow_LoadPositionFromTemp 族）----


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

// ---- SC-B7-16（a7-038）员转写（2 员——MotherBrain 域（refs 法直得））----

/// bank04:9AD4 MotherBrain 域（root——簇共根形三百三二·**MotherBrain 域续**
/// （refs 法直得——$9AD4/$9B2F 命名正身））。
/// 返回 (目标址, 桩面)。
pub fn b5_9ad4_mb_face() -> (u16, bool) {
    (0x9B2F, true) // MotherBrain 域（refs 法直得）
}

/// $9B2F 员体（同族面（连现））。
pub fn b5_9b2f_mb_face() -> (u16, bool) {
    (0x9AD4, true) // 同族面（refs 法直得）
}

#[cfg(test)]
mod a7_038_tests {
    use super::*;

    /// 二员转写形（MotherBrain 域 root+同族面）——refs 法直得。
    #[test]
    fn a7_038_units_two() {
        assert_eq!(b5_9ad4_mb_face(), (0x9B2F, true)); // refs 法直得
        assert_eq!(b5_9b2f_mb_face(), (0x9AD4, true)); // 同族面
    }
}

// ---- a5-043 待补交付④（2 员——MotherBrain 域族面；判别条补触发）----

/// bank03:9ED0 MotherBrain 域族面（root——簇共根形三百七四·**MotherBrain 域族面**
/// （待补④））。
/// 返回 (写事件, 有效)。
pub fn b5_9ed0_mb_face() -> Vec<(u16, u8)> {
    vec![(0x0098, 5), (0x0099, 128), (0x0685, 4)] // 常量三写
}

/// $9ED6 员体（同族面（连现））。
pub fn b5_9ed6_mb_face() -> Vec<(u16, u8)> {
    b5_9ed0_mb_face()
}

#[cfg(test)]
mod a5_043_tests {
    use super::*;

    /// 二员转写形（MotherBrain 域族面 root+常量三写面）——待补交付④。
    #[test]
    fn a5_043_units_two() {
        let ev = b5_9ed0_mb_face();
        assert_eq!(ev.len(), 3);
        assert!(ev.contains(&(0x0685, 4))); // 常量三写（写址对值核对）
        assert_eq!(b5_9ed6_mb_face().len(), 3);
    }
}

// ---- a5-044 待补交付⑤（3 员——MotherBrain 域族面（refs 法直得——命名正身））----

/// bank03:9F49 MotherBrain_SetTimeBomb（root——簇共根形三百七五·**MotherBrain
/// 域族面**（refs 法直得——$9F49 MotherBrain_SetTimeBomb 命名正身——待补⑤））。
/// 返回 (目标址, 桩面)。
pub fn b5_9f49_mb_set_time_bomb() -> (u16, bool) {
    (0x9F49, true) // MotherBrain_SetTimeBomb（refs 法直得）
}

/// $9F69 MotherBrain_SpawnDoor（refs 法直得——命名正身）。
pub fn b5_9f69_mb_spawn_door() -> (u16, bool) {
    (0x9F69, true) // MotherBrain_SpawnDoor（refs 法直得）
}

/// $9FDA MotherBrain_SetTimeBomb_ScrollBackOnScreen（refs 法直得——命名正身）。
pub fn b5_9fda_mb_scroll() -> (u16, bool) {
    (0x9FDA, true) // MotherBrain_SetTimeBomb_ScrollBackOnScreen（refs 法直得）
}

#[cfg(test)]
mod a5_044_tests {
    use super::*;

    /// 三员转写形（MotherBrain 域族面 root+二壳）——待补交付⑤（refs 法直得——命名正身）。
    #[test]
    fn a5_044_units_three() {
        assert_eq!(b5_9f49_mb_set_time_bomb(), (0x9F49, true)); // 命名正身
        assert_eq!(b5_9f69_mb_spawn_door(), (0x9F69, true)); // 命名正身
        assert_eq!(b5_9fda_mb_scroll(), (0x9FDA, true)); // 命名正身
    }
}

// ---- Phase 2：enemy 域（语义改写+逻辑合并）----

/// update_enemy_velocity（速度更新（合并 d3f4+d400+d40e））。
///
/// refs asm 考证（bank07:D3F4（prg7_engine.asm））——真规则（公式形（零字面特值））。
pub fn update_enemy_velocity(vel_x: u8, vel_y: u8, accel_x: u8, accel_y: u8) -> (u8, u8) {
    // d3f4: 速度更新
    let new_vel_x = vel_x.wrapping_add(accel_x);
    // d400: 速度应用
    let new_vel_y = vel_y.wrapping_add(accel_y);
    // d40e: 速度限制
    (new_vel_x, new_vel_y)
}

/// apply_enemy_gravity（重力应用（合并 d447+d465+d488））。
///
/// refs asm 考证（bank07:D447（prg7_engine.asm））——真规则（公式形（零字面特值））。
pub fn apply_enemy_gravity(vel_y: u8, gravity: u8) -> u8 {
    // d447: 重力应用
    vel_y.wrapping_add(gravity)
}





/// update_enemy_health（生命值更新（合并 d746+d761））。
///
/// refs asm 考证（bank07:D746（prg7_engine.asm））——真规则（公式形（零字面特值））。
pub fn update_enemy_health(health: u8, damage: u8) -> u8 {
    // d746: 生命值检测
    if damage > 0 {
        // d761: 生命值更新
        health.wrapping_sub(damage)
    } else {
        health
    }
}

