//! `game-core::title::interaction` —— 标题画面交互（开始游戏（行为等价））。
//!
//! refs 考证（bank:addr（A 级原文））：MainRoutinePtrTable（bank07:C141（主调度表
//! （状态→函数映射）））+GameEngine（bank07:C92B（游戏引擎））+SamusInit（bank07:C8D1
//! （萨姆斯初始化））。

#[cfg(test)]
mod tests_title_interaction {
    //! TDD step1（穷举等价证明（全域穷举（边界条件（异常路径））））。
    //! 判径（任务规约面）：input 位0 置⇒1（开始游戏（游戏模式））；否则保持 game_mode。

    use super::*;

    /// 独立参考径（证伪立场）：分路公式直序。
    fn reference_interact(input: u8, game_mode: u8) -> u8 {
        if input & 0x01 != 0 { 1 } else { game_mode }
    }

    /// 【穷举·全域】input×game_mode 全域 256² 逐位对拍。
    #[test]
    fn exhaustive_title_interaction() {
        for input in 0u8..=255 {
            for mode in 0u8..=255 {
                assert_eq!(title_screen_interaction(input, mode), reference_interact(input, mode),
                           "input={} mode={}", input, mode);
            }
        }
    }

    /// 【边界】位0 置/清定钉（0x01⇒1/0x00⇒mode 保持）+mode 极值（0xFF 保持面）。
    #[test]
    fn boundary_bits() {
        assert_eq!(title_screen_interaction(0x01, 0xFF), 1, "位0 置⇒开始（覆盖 mode）");
        assert_eq!(title_screen_interaction(0x00, 0xFF), 0xFF, "位0 清⇒保持（mode=0xFF）");
        assert_eq!(title_screen_interaction(0x00, 0x00), 0x00, "位0 清⇒保持（mode=0）");
    }

    /// 【异常路径】非位0 输入位不干预（0xFE（位0 清）⇒保持；0x03（位0 置）⇒1）。
    #[test]
    fn exception_other_bits() {
        assert_eq!(title_screen_interaction(0xFE, 7), 7, "非位0 全置但位0 清⇒保持");
        assert_eq!(title_screen_interaction(0x03, 7), 1, "位0 置（杂位）⇒开始");
    }
}

// ===== 标题画面交互（实现（真规则（公式形（零字面特值））））=====
// refs 考证（bank:addr（A 级原文））：MainRoutinePtrTable（bank07:C141）+GameEngine
// （bank07:C92B）+SamusInit（bank07:C8D1）。

/// 标题画面交互（开始游戏（行为等价））——input 位0 置⇒1（开始游戏（游戏模式）
/// （MainRoutinePtrTable 07:C141 状态映射（标题→游戏中）））；否则保持 game_mode。
pub fn title_screen_interaction(input: u8, game_mode: u8) -> u8 {
    if input & 0x01 != 0 {
        1
    } else {
        game_mode
    }
}
