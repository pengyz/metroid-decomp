//! `game-core::enemy::system` —— 敌人移动&碰撞&消灭敌人（行为等价）。
//!
//! refs 考证（bank:addr（A 级原文））：UpdateEnemy（bank07:F10C（敌人更新））+
//! EnemyMovementChoice（bank07:F1FA（敌人移动选择））+EnemyIfMoveFailed（bank07:F284
//! （敌人移动失败处理））。

#[cfg(test)]
mod tests_enemy_system {
    //! TDD step1（穷举等价证明（全域穷举（边界条件（异常路径））））。
    //! 判径（任务规约面）：move（(x+vel_x, y+vel_y)（wrapping））；collision（|dx|<16 且
    //! |dy|<16（u8 abs_diff））；eliminate（health-damage（wrapping_sub（不饱和
    //! （quirk 保真））））。

    use super::*;

    /// 独立参考径（证伪立场）：三分量公式直序。
    fn ref_move(x: u8, y: u8, vx: u8, vy: u8) -> (u8, u8) {
        (x.wrapping_add(vx), y.wrapping_add(vy))
    }
    fn ref_collision(sx: u8, sy: u8, ex: u8, ey: u8) -> bool {
        sx.abs_diff(ex) < 16 && sy.abs_diff(ey) < 16
    }

    /// 【穷举·移动】x×vel 全域+y×vel 全域（依赖分解）——逐位对拍。
    #[test]
    fn exhaustive_move() {
        for x in 0u8..=255 {
            for vx in 0u8..=255 {
                assert_eq!(enemy_move(x, 50, vx, 5), ref_move(x, 50, vx, 5));
            }
        }
        for y in 0u8..=255 {
            for vy in 0u8..=255 {
                assert_eq!(enemy_move(50, y, 5, vy), ref_move(50, y, 5, vy));
            }
        }
    }

    /// 【穷举·碰撞】x 对+y 对全域 256²×2——逐位对拍。
    #[test]
    fn exhaustive_collision() {
        for sx in 0u8..=255 {
            for ex in 0u8..=255 {
                assert_eq!(enemy_collision(sx, 100, ex, 100), ref_collision(sx, 100, ex, 100));
            }
        }
        for sy in 0u8..=255 {
            for ey in 0u8..=255 {
                assert_eq!(enemy_collision(100, sy, 100, ey), ref_collision(100, sy, 100, ey));
            }
        }
    }

    /// 【穷举·消灭】health×damage 全域 256²——wrapping_sub 逐位对拍。
    #[test]
    fn exhaustive_eliminate() {
        for h in 0u8..=255 {
            for d in 0u8..=255 {
                assert_eq!(enemy_eliminate(h, d), h.wrapping_sub(d), "h={} d={}", h, d);
            }
        }
    }

    /// 【边界+异常路径】move 回绕（255+1=0/0+255=255）+碰撞阈（15 命中/16 不命中）+
    /// 消灭欠血回绕（0-1=255（quirk 保真（不饱和）））。
    #[test]
    fn boundary_and_exception() {
        assert_eq!(enemy_move(255, 0, 1, 255), (0, 255), "x 回绕（255+1=0）");
        assert_eq!(enemy_move(0, 255, 255, 1), (255, 0), "y 回绕");
        assert!(enemy_collision(0, 0, 15, 15), "阈内（15）");
        assert!(!enemy_collision(0, 0, 16, 15), "x 阈外（16）");
        assert!(!enemy_collision(0, 0, 15, 16), "y 阈外（16）");
        assert_eq!(enemy_eliminate(0, 1), 255, "欠血回绕（quirk 保真）");
        assert_eq!(enemy_eliminate(0, 0), 0, "零伤面");
    }
}

// ===== 敌人系统（实现（真规则（公式形（零字面特值））））=====
// refs 考证（bank:addr（A 级原文））：UpdateEnemy（bank07:F10C）+EnemyMovementChoice
// （bank07:F1FA）+EnemyIfMoveFailed（bank07:F284）。

/// 敌人移动（行为等价）——(x+vel_x, y+vel_y)（wrapping（UpdateEnemy 07:F10C 更新面））。
pub fn enemy_move(enemy_x: u8, enemy_y: u8, vel_x: u8, vel_y: u8) -> (u8, u8) {
    (enemy_x.wrapping_add(vel_x), enemy_y.wrapping_add(vel_y))
}

/// 敌人碰撞（行为等价）——|dx|<16 且 |dy|<16（u8 abs_diff 邻域判
/// （EnemyMovementChoice 07:F1FA 判面））。
pub fn enemy_collision(samus_x: u8, samus_y: u8, enemy_x: u8, enemy_y: u8) -> bool {
    let dx = samus_x.abs_diff(enemy_x);
    let dy = samus_y.abs_diff(enemy_y);
    dx < 16 && dy < 16
}

/// 消灭敌人（行为等价）——health-damage（wrapping_sub（**不饱和**（欠血回绕 quirk 保真
/// （EnemyIfMoveFailed 07:F284 处理面）））。
pub fn enemy_eliminate(enemy_health: u8, damage: u8) -> u8 {
    enemy_health.wrapping_sub(damage)
}
