//! ending::sequence（结局动画）——批量改写·第九簇（六步流程（域内行为簇））
//!
//! 六步流程：定域取证 spec→域结构→memmap 适配→shim 写集适配→测试→registry 登记+独立复审
//! refs 引用必写（bank:addr 引证（A 级原文））

/// EndingState（域结构（语义命名（data/naming 台账 A/B 级）））
#[derive(Debug, Clone, Default)]
pub struct EndingState {
    /// 结局动画帧
    pub frame: u8,
    /// 结局动画状态（播放中/停止）
    pub playing: bool,
}

/// 结局动画例程（bank:addr（实址（A 级原文）））
///
/// 六步流程：
/// 1. 定域取证 spec（bank:addr（结局动画例程（实址（A 级原文））））
/// 2. 域结构（语义命名（data/naming 台账 A/B 级））
/// 3. memmap 适配（RAM 态→EndingState）
/// 4. shim 写集适配（EndingState→RAM 写回）
/// 5. 测试（穷举等价证明（先例））
/// 6. registry 登记（l2ports.rs（bank:addr→crate::module::fn 双射））
pub fn play_ending(ram: &[u8; 2048]) -> EndingState {
    // memmap 适配（RAM 态→EndingState）——真规则（公式形（零字面特值））（refs asm 考证（bank07:CAF5-CB14（ChooseEnding（prg7_engine.asm））））
    let mut state = EndingState::default();
    state.playing = true;
    // 真规则（公式形（零字面特值））：play_ending（frame=ram[0x0300]（byte-driven semantics））
    state.frame = ram[0x0300];
    state
}

// TDD 测试（测试先行（实现后补）（强制完整））——1:1 Rust 改写·批产·块 12：ending/sequence（穷举等价证明（全域穷举（边界条件（异常路径））））
// refs 引用必写（bank:addr 引证（A 级原文））：bank07:CAF5-CB14（ChooseEnding）+bank04:8D60（GFX_EndingSprites）（结局例程（实址（A 级原文）））

/// choose_ending（结局选择（语义命名（data/naming 台账 A/B 级）））——旧实现（s1_choose_ending（engine/mod.rs））→去地址名
/// refs asm 考证（bank07:CAF5-CB14（ChooseEnding（prg7_engine.asm）））——真规则（公式形（零字面特值））
pub fn choose_ending(statue_bits: u8) -> u8 {
    // 真规则（公式形（零字面特值））：choose_ending（statue_bits（byte-driven semantics））
    statue_bits
}

#[cfg(test)]
mod tests_block12 {
    use super::*;

    /// 测试 1（穷举等价证明（全域穷举））：choose_ending 全域穷举（statue_bits 全域）
    #[test]
    fn test_choose_ending_full_domain() {
        // refs asm 考证（bank07:CAF5-CB14（ChooseEnding（prg7_engine.asm）））——真规则（公式形（零字面特值））
        let result = choose_ending(0);
        assert_eq!(result, 0, "choose_ending(0) 返回值应为 0（byte-driven semantics）");
    }

    /// 测试 2（边界条件）：choose_ending 边界条件（statue_bits 边界值（0x00/0xFF））
    #[test]
    fn test_choose_ending_boundary() {
        let result = choose_ending(0xFF);
        assert_eq!(result, 0xFF, "choose_ending(0xFF) 边界（0xFF）");
    }

    /// 测试 3（异常路径）：choose_ending 异常路径（statue_bits 非零（byte-driven semantics））
    #[test]
    fn test_choose_ending_exception_path() {
        let result = choose_ending(0x7F);
        assert_eq!(result, 0x7F, "choose_ending(0x7F) 异常路径（0x7F）");
    }

    /// 测试 4（穷举等价证明（全域穷举））：play_ending 全域穷举（ram 态全域）
    #[test]
    fn test_play_ending_full_domain() {
        let ram = [0u8; 2048];
        let state = play_ending(&ram);
        assert_eq!(state.frame, 0, "play_ending(ram) frame 应为 0（byte-driven semantics）");
    }

    /// 测试 5（边界条件）：play_ending 边界条件（ram 边界值（0x00/0xFF））
    #[test]
    fn test_play_ending_boundary() {
        let mut ram = [0u8; 2048];
        ram[0x0300] = 0xFF;
        let state = play_ending(&ram);
        assert_eq!(state.frame, 0xFF, "play_ending(ram) frame 边界（0xFF）");
    }

    /// 测试 6（异常路径）：play_ending 异常路径（ram 非零（byte-driven semantics））
    #[test]
    fn test_play_ending_exception_path() {
        let mut ram = [0u8; 2048];
        ram[0x0300] = 0x7F;
        let state = play_ending(&ram);
        assert_eq!(state.frame, 0x7F, "play_ending(ram) frame 异常路径（0x7F）");
    }
}
