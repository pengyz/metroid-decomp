//! `game-core::enemy::spawn_control` —— 敌生成控制面（1:1 Rust 改写·批产·块 7 批 1）。
//!
//! 【worktree 隔离】独立模块文件（避免共享文件并发写入）；【timer 纪律】穷举≤256³/依赖分解。
//! refs 锚（engine/mod.rs 注内 bank:addr）：$F68D root（:436）/$F62E（:415）/$F649（:427）/
//! $F6A4（:441）/$F8A2（:449）/$FA0A（:455）/$FA97（:461）。
//!
//! 共写根（$F68D PlaySnd3 root）：$6AF9[x]←A/$6AFA[x]←A/$6AF8[x]←0（A 归零出口）。

#[cfg(test)]
mod tests_b7_batch1 {
    //! TDD step1（测试先行（强制完整））——穷举等价（新==旧（全域穷举（边界（异常路径））
    //! （≤256³ 闸时限界（块 6 教训））））。

    use super::*;
    use crate::engine::{s1_root_f68d, s1_f62e_body, s1_f649_body, s1_f6a4_body,
                        s1_f8a2_body, s1_fa0a_body, s1_fa97_body};

    /// 【穷举·root 面】a×x 全域——set_enemy_spawn_face==s1_root_f68d。
    #[test]
    fn exhaustive_root_face_equivalence() {
        for a in 0u8..=255 {
            for x in 0u8..=255 {
                assert_eq!(s1_root_f68d(a, x), set_enemy_spawn_face(a, x));
            }
        }
    }

    /// 【穷举·槽搜生成】a×x 全域+槽位状态 16 组合（$C0/$C8/$D0/$D8 空满）+首空位全 4 位
    /// ——spawn_with_slot_search==s1_f62e_body（搜索径逐钉）。
    #[test]
    fn exhaustive_slot_search_equivalence() {
        for a in 0u8..=255 {
            for x in [0u8, 1, 0x80, 0xFF] {
                for mask in 0u8..16 {
                    let mut wram = vec![0u8; 0x2000];
                    for (i, slot) in [0xC0u16, 0xC8, 0xD0, 0xD8].iter().enumerate() {
                        if mask & (1 << i) != 0 {
                            wram[0xAF4 + *slot as usize] = 1;
                        }
                    }
                    assert_eq!(s1_f62e_body(a, x, &wram), spawn_with_slot_search(a, x, &wram));
                }
            }
        }
    }

    /// 【穷举·活跃生成】a×x×v6af4×v0a 依赖分解（写面分量独立（a/x｜v6af4｜v0a 符号））
    /// ——spawn_active_with_timer==s1_f649_body。
    #[test]
    fn exhaustive_active_spawn_equivalence() {
        for a in 0u8..=255 {
            for x in 0u8..=255 {
                assert_eq!(s1_f649_body(a, x, 0x33, 0x7F), spawn_active_with_timer(a, x, 0x33, 0x7F));
            }
        }
        for v6af4 in 0u8..=255 {
            assert_eq!(s1_f649_body(5, 6, v6af4, 0x7F), spawn_active_with_timer(5, 6, v6af4, 0x7F));
        }
        for v0a in 0u8..=255 {
            assert_eq!(s1_f649_body(5, 6, 0x33, v0a), spawn_active_with_timer(5, 6, 0x33, v0a));
        }
        for &(a, x, b, c) in &[(0u8, 0, 0, 0), (0xFF, 0xFF, 0xFF, 0x80), (0x7F, 0x80, 0x01, 0xFF)] {
            assert_eq!(s1_f649_body(a, x, b, c), spawn_active_with_timer(a, x, b, c));
        }
    }

    /// 【穷举·倒数生成/弹生成/延迟生成/派发生成】a×x/tbl 全域（≤256²）——四员等价。
    #[test]
    fn exhaustive_spawn_variants_equivalence() {
        for a in 0u8..=255 {
            for x in 0u8..=255 {
                assert_eq!(s1_fa0a_body(a, x), spawn_with_delay_flag(a, x));
                assert_eq!(s1_fa97_body(a, x), spawn_and_dispatch(a, x));
                for t in [0u8, 1, 0x7F, 0x80, 0xFF] {
                    assert_eq!(s1_f6a4_body(a, x, t, 0x55), spawn_with_countdown(a, x, t, 0x55));
                    assert_eq!(s1_f8a2_body(a, x, t), spawn_projectile_with_anim(a, x, t));
                }
            }
        }
        for t in 0u8..=255 {
            for v in 0u8..=255 {
                assert_eq!(s1_f6a4_body(3, 4, t, v), spawn_with_countdown(3, 4, t, v));
            }
        }
    }
}

// ===== 1:1 Rust 改写·批产·块 7 批 1：敌生成控制语义命名（去地址名（公式形（零字面特值）））=====

/// 共写根（旧 s1_root_f68d（$F68D PlaySnd3 root）：$6AF9[x]←A/$6AFA[x]←A/$6AF8[x]←0）。
pub fn set_enemy_spawn_face(a_in: u8, x: u8) -> (Vec<(u16, u8)>, u8) {
    (vec![(0x6AF9 + x as u16, a_in), (0x6AFA + x as u16, a_in), (0x6AF8 + x as u16, 0)], 0)
}

/// 槽搜生成（旧 s1_f62e_body（$F62E）：root+$0406[x]←0+槽搜（$C0 by 8→$E0
/// 首 $6AF4[s]==0））。
pub fn spawn_with_slot_search(a_in: u8, x: u8, wram: &[u8]) -> (Vec<(u16, u8)>, u16) {
    let mut ev = vec![(0x6AF9 + x as u16, a_in), (0x6AFA + x as u16, a_in),
                      (0x6AF8 + x as u16, 0), (0x0406 + x as u16, 0)];
    let mut slot: u16 = 0xC0;
    while slot < 0xE0 {
        if wram.get(0xAF4 + slot as usize).copied().unwrap_or(0) == 0 { break; }
        slot += 8;
    }
    (ev, slot)
}

/// 活跃生成（旧 s1_f649_body（$F649）：root+$0406[x]←$0A+$6AF4[x] inc
/// +$0407[x]←（$0A 位7?3:0））。
pub fn spawn_active_with_timer(a_in: u8, x: u8, v6af4_x: u8, v0a: u8) -> Vec<(u16, u8)> {
    vec![(0x6AF9 + x as u16, a_in), (0x6AFA + x as u16, a_in),
         (0x6AF8 + x as u16, 0),
         (0x0406 + x as u16, 0x0A),
         (0x6AF4 + x as u16, v6af4_x.wrapping_add(1)),
         (0x0407 + x as u16, if (v0a as i8) < 0 { 3 } else { 0 })]
}

/// 倒数生成（旧 s1_f6a4_body（$F6A4）：root+$967B[y]&$7F 次 dec $6AFA[x]（一次减 n））。
pub fn spawn_with_countdown(a_in: u8, x: u8, tbl_val: u8, v6afa_x: u8) -> (Vec<(u16, u8)>, u8) {
    let n = tbl_val & 0x7F;
    let mut ev = vec![(0x6AF9 + x as u16, a_in), (0x6AF8 + x as u16, 0)];
    ev.push((0x6AFA + x as u16, v6afa_x.wrapping_sub(n)));
    (ev, n)
}

/// 弹生成（旧 s1_f8a2_body（$F8A2）：root+$6AF4[y]←1+$0403[y]←tbl+$0402[y]←0）。
pub fn spawn_projectile_with_anim(a_in: u8, y: u8, tbl_entry: u8) -> Vec<(u16, u8)> {
    vec![(0x6AF9 + y as u16, a_in), (0x6AFA + y as u16, a_in), (0x6AF8 + y as u16, 0),
         (0x6AF4 + y as u16, 1), (0x0403 + y as u16, tbl_entry), (0x0402 + y as u16, 0)]
}

/// 延迟生成（旧 s1_fa0a_body（$FA0A）：root+$0409[x]←$0A→尾跳 $F97C）。
pub fn spawn_with_delay_flag(a_in: u8, x: u8) -> (Vec<(u16, u8)>, u16) {
    (vec![(0x6AF9 + x as u16, a_in), (0x6AFA + x as u16, a_in), (0x6AF8 + x as u16, 0),
          (0x0409 + x as u16, 0x0A)], 0xF97C)
}

/// 派发生成（旧 s1_fa97_body（$FA97）：root→尾跳 $F97C（槽搜族（$FAB4 调面注）））。
pub fn spawn_and_dispatch(a_in: u8, x: u8) -> (Vec<(u16, u8)>, u16) {
    (vec![(0x6AF9 + x as u16, a_in), (0x6AFA + x as u16, a_in), (0x6AF8 + x as u16, 0)],
     0xF97C)
}
