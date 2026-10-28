//! `game-core::engine::enemy_common` —— enemy 公共族簇（P2 批 2（纯机械移动））。
//! refs 锚内随段。

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

