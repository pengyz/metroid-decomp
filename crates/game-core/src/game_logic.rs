//! `game-core::game_logic` —— 游戏逻辑（战斗系统+道具系统+存档系统（TDD 件））。
//!
//! refs 考证（bank:addr（A 级原文））：UpdateEnemy（bank07:F10C（敌人更新））+
//! EnemyMovementChoice（bank07:F1FA（敌人移动选择））+EnemyIfMoveFailed（bank07:F284
//! （敌人移动失败处理））。

#[cfg(test)]
mod tests_game_logic {
    //! TDD step1（穷举等价证明（全域穷举（边界条件（异常路径））））。
    //! 判径（任务规约面）：combat（|dx|<16 且 |dy|<16（u8 abs_diff））；item（timer>0
    //! ? state+1 : 0（wrapping））；save（[(0x0000,frame_count),(0x0001,game_mode)]）。

    use super::*;
    use crate::kernel::main_loop::GameState;

    /// 独立参考径（证伪立场）：三分量公式直序。
    fn ref_combat(sx: u8, sy: u8, ex: u8, ey: u8) -> bool {
        sx.abs_diff(ex) < 16 && sy.abs_diff(ey) < 16
    }
    fn ref_item(s: u8, t: u8) -> u8 {
        if t > 0 { s.wrapping_add(1) } else { 0 }
    }

    /// 【穷举·战斗】x 对+y 对全域 256²×2 逐位对拍。
    #[test]
    fn exhaustive_combat() {
        for sx in 0u8..=255 {
            for ex in 0u8..=255 {
                assert_eq!(combat_system(sx, 100, ex, 100), ref_combat(sx, 100, ex, 100));
            }
        }
        for sy in 0u8..=255 {
            for ey in 0u8..=255 {
                assert_eq!(combat_system(100, sy, 100, ey), ref_combat(100, sy, 100, ey));
            }
        }
    }

    /// 【边界·战斗】dx/dy=15（命中）/16（不命中）四象限+回绕面（0↔255）。
    #[test]
    fn combat_boundary() {
        assert!(combat_system(0, 0, 15, 15), "双 15 命中");
        assert!(!combat_system(0, 0, 16, 15), "x=16 不命中");
        assert!(!combat_system(0, 0, 15, 16), "y=16 不命中");
        assert!(!combat_system(0, 0, 255, 0), "回绕面（|0-255|=255）");
    }

    /// 【穷举·道具】state×timer 全域 256² 逐位对拍。
    #[test]
    fn exhaustive_item() {
        for s in 0u8..=255 {
            for t in 0u8..=255 {
                assert_eq!(item_system(s, t), ref_item(s, t), "s={} t={}", s, t);
            }
        }
    }

    /// 【穷举·存档】frame_count×game_mode 全域 256²——写面序/写值逐钉。
    #[test]
    fn exhaustive_save() {
        for fc in 0u8..=255 {
            for gm in 0u8..=255 {
                let gs = GameState { frame_count: fc, game_mode: gm };
                let w = save_system(&gs);
                assert_eq!(w, vec![(0x0000, fc), (0x0001, gm)], "fc={} gm={}", fc, gm);
            }
        }
    }

    /// 【异常路径】回绕面（item 0xFF+1=0/timer==0 清零）+存档面长恒 2。
    #[test]
    fn exception_paths() {
        assert_eq!(item_system(0xFF, 1), 0, "回绕（0xFF+1=0）");
        assert_eq!(item_system(0xFF, 0), 0, "timer==0 清零");
        let gs = GameState { frame_count: 0xFF, game_mode: 0xFF };
        assert_eq!(save_system(&gs).len(), 2, "存档面长恒 2");
    }
}

// ===== 游戏逻辑（实现（真规则（公式形（零字面特值））））=====
// refs 考证（bank:addr（A 级原文））：UpdateEnemy（bank07:F10C）+EnemyMovementChoice
// （bank07:F1FA）+EnemyIfMoveFailed（bank07:F284）。

use crate::kernel::main_loop::GameState;

/// 战斗系统（碰撞判定）——|dx|<16 且 |dy|<16（u8 abs_diff 邻域判
/// （UpdateEnemy 07:F10C 状态面+EnemyMovementChoice 07:F1FA 邻域选面））。
pub fn combat_system(samus_x: u8, samus_y: u8, enemy_x: u8, enemy_y: u8) -> bool {
    let dx = samus_x.abs_diff(enemy_x);
    let dy = samus_y.abs_diff(enemy_y);
    dx < 16 && dy < 16
}

/// 道具系统（状态推进）——timer>0 ? item_state+1 : 0（wrapping）
/// （EnemyIfMoveFailed 07:F284 计时面对偶）。
pub fn item_system(item_state: u8, timer: u8) -> u8 {
    if timer > 0 { item_state.wrapping_add(1) } else { 0 }
}

/// 存档系统（状态写面）——[(0x0000, frame_count), (0x0001, game_mode)]（写序恒定）。
pub fn save_system(game_state: &GameState) -> Vec<(u16, u8)> {
    let mut writes = Vec::with_capacity(2);
    writes.push((0x0000, game_state.frame_count));
    writes.push((0x0001, game_state.game_mode));
    writes
}
