//! `game-core::samus::control` —— 萨姆斯左右移动/跳跃/攻击（行为等价）。
//!
//! refs 考证（bank:addr（A 级原文））：SamusMoveHorizontally（bank07:E0C2（萨姆斯水平移动））
//! +SamusJump（bank07:E12D（萨姆斯跳跃））+SamusAttack（bank07:E26C（萨姆斯攻击））。

#[cfg(test)]
mod tests_samus_control {
    //! TDD step1（穷举等价证明（全域穷举（边界条件（异常路径））））。
    //! 判径（任务规约面）：move（input 位1⇒左（x-1,vel-1）（**左优先**）/位0⇒右（x+1,vel+1）
    //! /否则不变）；jump（位2⇒（y-2,vel-2）/否则不变）；attack（位3⇒true）。

    use super::*;

    /// 独立参考径（证伪立场）：三分量公式直序。
    fn ref_move(input: u8, x: u8, vel_x: u8) -> (u8, u8) {
        if input & 0x02 != 0 {
            (x.wrapping_sub(1), vel_x.wrapping_sub(1))
        } else if input & 0x01 != 0 {
            (x.wrapping_add(1), vel_x.wrapping_add(1))
        } else {
            (x, vel_x)
        }
    }
    fn ref_jump(input: u8, y: u8, vel_y: u8) -> (u8, u8) {
        if input & 0x04 != 0 {
            (y.wrapping_sub(2), vel_y.wrapping_sub(2))
        } else {
            (y, vel_y)
        }
    }

    /// 【穷举·移动】input×x 全域+input×vel 全域+依赖分解——逐位对拍。
    #[test]
    fn exhaustive_move() {
        for input in 0u8..=255 {
            for x in 0u8..=255 {
                assert_eq!(samus_move_horizontal(input, x, 50), ref_move(input, x, 50));
            }
            for vel in 0u8..=255 {
                assert_eq!(samus_move_horizontal(input, 50, vel), ref_move(input, 50, vel));
            }
        }
    }

    /// 【穷举·跳跃】input×y 全域+input×vel 全域——逐位对拍。
    #[test]
    fn exhaustive_jump() {
        for input in 0u8..=255 {
            for y in 0u8..=255 {
                assert_eq!(samus_jump(input, y, 50), ref_jump(input, y, 50));
            }
            for vel in 0u8..=255 {
                assert_eq!(samus_jump(input, 50, vel), ref_jump(input, 50, vel));
            }
        }
    }

    /// 【穷举·攻击】input 全域 256——位3 判逐钉。
    #[test]
    fn exhaustive_attack() {
        for input in 0u8..=255 {
            assert_eq!(samus_attack(input), input & 0x08 != 0, "input={}", input);
        }
    }

    /// 【边界+异常路径】回绕面（x=0 左⇒255/x=255 右⇒0/y 同理（±2））+左优先
    /// （双位同置⇒左径）+杂位不干预（0x00/0xF0/0x80）。
    #[test]
    fn boundary_and_exception() {
        assert_eq!(samus_move_horizontal(0x02, 0, 0), (255, 255), "左回绕（0-1=255）");
        assert_eq!(samus_move_horizontal(0x01, 255, 255), (0, 0), "右回绕（255+1=0）");
        assert_eq!(samus_jump(0x04, 0, 1), (254, 255), "跳回绕（0-2=254）");
        assert_eq!(samus_jump(0x04, 1, 0), (255, 254), "跳回绕（1-2=255）");
        assert_eq!(samus_move_horizontal(0x03, 10, 10), (9, 9), "左优先（双位同置⇒左径）");
        assert_eq!(samus_move_horizontal(0x00, 10, 10), (10, 10), "无位⇒不变");
        assert_eq!(samus_move_horizontal(0xF0, 10, 10), (10, 10), "杂位不干预");
        assert_eq!(samus_jump(0x00, 10, 10), (10, 10), "跳无位⇒不变");
        assert!(!samus_attack(0x00), "攻击无位⇒false");
    }
}

// ===== 萨姆斯控制（实现（真规则（公式形（零字面特值））））=====
// refs 考证（bank:addr（A 级原文））：SamusMoveHorizontally（bank07:E0C2）+SamusJump
// （bank07:E12D）+SamusAttack（bank07:E26C）。

/// 萨姆斯水平移动（行为等价）——input 位1⇒左（x-1,vel-1）（**左优先**（双位同置走左径））
/// /位0⇒右（x+1,vel+1）/否则不变（均 wrapping）。
pub fn samus_move_horizontal(input: u8, x: u8, vel_x: u8) -> (u8, u8) {
    if input & 0x02 != 0 {
        (x.wrapping_sub(1), vel_x.wrapping_sub(1))
    } else if input & 0x01 != 0 {
        (x.wrapping_add(1), vel_x.wrapping_add(1))
    } else {
        (x, vel_x)
    }
}

/// 萨姆斯跳跃（行为等价）——input 位2⇒（y-2,vel-2）/否则不变（均 wrapping）。
pub fn samus_jump(input: u8, y: u8, vel_y: u8) -> (u8, u8) {
    if input & 0x04 != 0 {
        (y.wrapping_sub(2), vel_y.wrapping_sub(2))
    } else {
        (y, vel_y)
    }
}

/// 萨姆斯攻击（行为等价）——input 位3 置⇒true。
pub fn samus_attack(input: u8) -> bool {
    input & 0x08 != 0
}
