//! `game-core::enemy::ai` —— 敌人 AI（状态更新+移动决策+行为执行（TDD 件））。
//!
//! refs 考证（bank:addr（A 级原文））：UpdateEnemy（bank07:F10C（敌人更新））+
//! EnemyMovementChoice（bank07:F1FA（敌人移动选择））+EnemyIfMoveFailed（bank07:F284
//! （敌人移动失败处理））。

#[cfg(test)]
mod tests_enemy_ai {
    //! TDD step1（穷举等价证明（全域穷举（边界条件（异常路径））））。
    //! 判径（任务规约面）：new_state=(timer>0 ? state+1 : 0)；new_vel=(new_state 位0/1
    //! ⇒vel+1)；new_pos=pos+new_vel（均 wrapping）。

    use super::*;

    /// 独立参考径（证伪立场）：分量公式直序。
    fn reference_ai(s: u8, timer: u8, x: u8, y: u8, vx: u8, vy: u8) -> (u8, u8, u8, u8, u8) {
        let ns = if timer > 0 { s.wrapping_add(1) } else { 0 };
        let nvx = if ns & 0x01 != 0 { vx.wrapping_add(1) } else { vx };
        let nvy = if ns & 0x02 != 0 { vy.wrapping_add(1) } else { vy };
        (ns, x.wrapping_add(nvx), y.wrapping_add(nvy), nvx, nvy)
    }

    /// 【穷举·状态分量】state×timer 全域 256²——new_state 逐位对拍。
    #[test]
    fn exhaustive_state_component() {
        for s in 0u8..=255 {
            for timer in 0u8..=255 {
                let (ns, ..) = update_enemy_ai(s, timer, 0, 0, 0, 0);
                let (rns, ..) = reference_ai(s, timer, 0, 0, 0, 0);
                assert_eq!(ns, rns, "state={} timer={}", s, timer);
            }
        }
    }

    /// 【穷举·速度分量】ns 位面×vel 全域——new_vel 位0/1 判逐位对拍。
    #[test]
    fn exhaustive_velocity_component() {
        for s in 0u8..=255 {
            for timer in [0u8, 1] {
                for vx in 0u8..=255 {
                    let (ns, _, _, nvx, _) = update_enemy_ai(s, timer, 0, 0, vx, 0);
                    let (rns, _, _, rnvx, _) = reference_ai(s, timer, 0, 0, vx, 0);
                    assert_eq!((ns, nvx), (rns, rnvx));
                }
                for vy in 0u8..=255 {
                    let (ns, _, _, _, nvy) = update_enemy_ai(s, timer, 0, 0, 0, vy);
                    let (rns, _, _, _, rnvy) = reference_ai(s, timer, 0, 0, 0, vy);
                    assert_eq!((ns, nvy), (rns, rnvy));
                }
            }
        }
    }

    /// 【穷举·位置分量】pos×new_vel 全域 256²——wrapping 加逐位对拍。
    #[test]
    fn exhaustive_position_component() {
        for x in 0u8..=255 {
            for vx in 0u8..=255 {
                let (_, nx, _, _, _) = update_enemy_ai(0x01, 1, x, 0, vx, 0);
                let (_, rnx, _, _, _) = reference_ai(0x01, 1, x, 0, vx, 0);
                assert_eq!(nx, rnx);
            }
        }
        for y in 0u8..=255 {
            for vy in 0u8..=255 {
                let (_, _, ny, _, _) = update_enemy_ai(0x02, 1, 0, y, 0, vy);
                let (_, _, rny, _, _) = reference_ai(0x02, 1, 0, y, 0, vy);
                assert_eq!(ny, rny);
            }
        }
    }

    /// 【边界+异常路径】回绕面（0xFF+1=0）+timer==0⇒state 清零+跨分量全组合抽样。
    #[test]
    fn boundary_and_exception() {
        // 回绕面
        assert_eq!(update_enemy_ai(0xFF, 1, 0xFF, 0xFF, 0xFF, 0xFF),
                   reference_ai(0xFF, 1, 0xFF, 0xFF, 0xFF, 0xFF));
        // 异常路径：timer==0⇒state=0（速度不进位）
        let (ns, nx, ny, nvx, nvy) = update_enemy_ai(0xFF, 0, 10, 20, 30, 40);
        // 【证伪自纠】原期望漏 pos 加（timer=0 仍走 pos+vel（vel 不进位但参与积分））——按正源。
        assert_eq!((ns, nx, ny, nvx, nvy), (0, 40, 60, 30, 40));
        // 跨分量抽样
        for &(s, t, x, y, vx, vy) in &[(0u8, 1u8, 0u8, 0u8, 0u8, 0u8),
                                       (1, 2, 0x80, 0x80, 1, 2), (3, 3, 255, 255, 255, 255)] {
            assert_eq!(update_enemy_ai(s, t, x, y, vx, vy), reference_ai(s, t, x, y, vx, vy));
        }
    }
}

// ===== 敌人 AI（实现（真规则（公式形（零字面特值））））=====
// refs 考证（bank:addr（A 级原文））：UpdateEnemy（bank07:F10C）+EnemyMovementChoice
// （bank07:F1FA）+EnemyIfMoveFailed（bank07:F284）。

/// 敌人 AI（状态更新+移动决策+行为执行）——
/// new_state=(timer>0 ? state+1 : 0)（UpdateEnemy 07:F10C 面）；
/// new_vel=(new_state 位0/位1 ⇒ vel+1)（EnemyMovementChoice 07:F1FA 面）；
/// new_pos=pos+new_vel（均 wrapping）（EnemyIfMoveFailed 07:F284 面）。
pub fn update_enemy_ai(enemy_state: u8, timer: u8, x: u8, y: u8, vel_x: u8, vel_y: u8)
    -> (u8, u8, u8, u8, u8) {
    let new_state = if timer > 0 { enemy_state.wrapping_add(1) } else { 0 };
    let new_vel_x = if new_state & 0x01 != 0 { vel_x.wrapping_add(1) } else { vel_x };
    let new_vel_y = if new_state & 0x02 != 0 { vel_y.wrapping_add(1) } else { vel_y };
    let new_x = x.wrapping_add(new_vel_x);
    let new_y = y.wrapping_add(new_vel_y);
    (new_state, new_x, new_y, new_vel_x, new_vel_y)
}
