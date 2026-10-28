//! `game-core::enemy::spawn_control` —— 敌生成控制面（1:1 Rust 改写·批产·块 7 批 1）。
//!
//! 【worktree 隔离】独立模块文件（避免共享文件并发写入）；【timer 纪律】穷举≤256³/依赖分解。
//! refs 锚（engine/mod.rs 注内 bank:addr）：$F68D root（:436）/$F62E（:415）/$F649（:427）/
//! $F6A4（:441）/$F8A2（:449）/$FA0A（:455）/$FA97（:461）。
//!
//! 共写根（$F68D PlaySnd3 root）：$6AF9[x]←A/$6AFA[x]←A/$6AF8[x]←0（A 归零出口）。

// ===== 1:1 Rust 改写·批产·块 7 批 1：敌生成控制语义命名（去地址名（公式形（零字面特值）））=====

/// 共写根（旧 update_enemy_root（$F68D PlaySnd3 root）：$6AF9[x]←A/$6AFA[x]←A/$6AF8[x]←0）。
pub fn set_enemy_spawn_face(a_in: u8, x: u8) -> (Vec<(u16, u8)>, u8) {
    (vec![(0x6AF9 + x as u16, a_in), (0x6AFA + x as u16, a_in), (0x6AF8 + x as u16, 0)], 0)
}

/// 槽搜生成（旧 update_enemy_movement_ext5（$F62E）：root+$0406[x]←0+槽搜（$C0 by 8→$E0
/// 首 $6AF4[s]==0））。
pub fn spawn_with_slot_search(a_in: u8, x: u8, wram: &[u8]) -> (Vec<(u16, u8)>, u16) {
    let ev = vec![(0x6AF9 + x as u16, a_in), (0x6AFA + x as u16, a_in),
                      (0x6AF8 + x as u16, 0), (0x0406 + x as u16, 0)];
    let mut slot: u16 = 0xC0;
    while slot < 0xE0 {
        if wram.get(0xAF4 + slot as usize).copied().unwrap_or(0) == 0 { break; }
        slot += 8;
    }
    (ev, slot)
}

/// 活跃生成（旧 update_enemy_movement_ext6（$F649）：root+$0406[x]←$0A+$6AF4[x] inc
/// +$0407[x]←（$0A 位7?3:0））。
pub fn spawn_active_with_timer(a_in: u8, x: u8, v6af4_x: u8, v0a: u8) -> Vec<(u16, u8)> {
    vec![(0x6AF9 + x as u16, a_in), (0x6AFA + x as u16, a_in),
         (0x6AF8 + x as u16, 0),
         (0x0406 + x as u16, 0x0A),
         (0x6AF4 + x as u16, v6af4_x.wrapping_add(1)),
         (0x0407 + x as u16, if (v0a as i8) < 0 { 3 } else { 0 })]
}

/// 倒数生成（旧 update_enemy_movement_ext7（$F6A4）：root+$967B[y]&$7F 次 dec $6AFA[x]（一次减 n））。
pub fn spawn_with_countdown(a_in: u8, x: u8, tbl_val: u8, v6afa_x: u8) -> (Vec<(u16, u8)>, u8) {
    let n = tbl_val & 0x7F;
    let mut ev = vec![(0x6AF9 + x as u16, a_in), (0x6AF8 + x as u16, 0)];
    ev.push((0x6AFA + x as u16, v6afa_x.wrapping_sub(n)));
    (ev, n)
}

/// 弹生成（旧 update_enemy_movement_ext9（$F8A2）：root+$6AF4[y]←1+$0403[y]←tbl+$0402[y]←0）。
pub fn spawn_projectile_with_anim(a_in: u8, y: u8, tbl_entry: u8) -> Vec<(u16, u8)> {
    vec![(0x6AF9 + y as u16, a_in), (0x6AFA + y as u16, a_in), (0x6AF8 + y as u16, 0),
         (0x6AF4 + y as u16, 1), (0x0403 + y as u16, tbl_entry), (0x0402 + y as u16, 0)]
}

/// 延迟生成（旧 remove_enemy（$FA0A）：root+$0409[x]←$0A→尾跳 $F97C）。
pub fn spawn_with_delay_flag(a_in: u8, x: u8) -> (Vec<(u16, u8)>, u16) {
    (vec![(0x6AF9 + x as u16, a_in), (0x6AFA + x as u16, a_in), (0x6AF8 + x as u16, 0),
          (0x0409 + x as u16, 0x0A)], 0xF97C)
}

/// 派发生成（旧 update_enemy_projectile（$FA97）：root→尾跳 $F97C（槽搜族（$FAB4 调面注）））。
pub fn spawn_and_dispatch(a_in: u8, x: u8) -> (Vec<(u16, u8)>, u16) {
    (vec![(0x6AF9 + x as u16, a_in), (0x6AFA + x as u16, a_in), (0x6AF8 + x as u16, 0)],
     0xF97C)
}
