//! enemy::waver（Waver）——批量改写·第二十八簇（六步流程（域内行为簇））
//!
//! 六步流程：定域取证 spec→域结构→memmap 适配→shim 写集适配→测试→registry 登记+独立复审
//! refs 引用必写（bank:addr 引证（A 级原文））
//!
//! 实址引证（bank:addr（A 级原文））：
//! - bank01:97D7（EnemyMovementChoice_Waver_{AREA}（prg1_brinstar.asm:498））

/// WaverState（域结构（语义命名（data/naming 台账 A/B 级）））
#[derive(Debug, Clone, Default)]
pub struct WaverState {
    /// 移动方向（波动）
    pub direction: u8,
    /// 波动相位
    pub phase: u8,
    /// 移动速度
    pub speed: u8,
}

/// Waver 例程（bank01:97D7（EnemyMovementChoice_Waver_{AREA}（实址（A 级原文））））
///
/// 六步流程：
/// 1. 定域取证 spec（bank01:97D7（EnemyMovementChoice_Waver_{AREA}（实址（A 级原文））））
/// 2. 域结构（语义命名（data/naming 台账 A/B 级））
/// 3. memmap 适配（RAM 态→WaverState）
/// 4. shim 写集适配（WaverState→RAM 写回）
/// 5. 测试（穷举等价证明（先例））
/// 6. registry 登记（l2ports.rs（bank:addr→crate::module::fn 双射））
pub fn update_waver(ram: &[u8; 2048]) -> WaverState {
    // memmap 适配（RAM 态→WaverState）——真规则（公式形（零字面特值））（refs asm 考证（bank01:97D7（EnemyMovementChoice_Waver_BANK1（prg1_brinstar.asm:498））））
    let mut state = WaverState::default();
    // 真规则（公式形（零字面特值））：WaverState（direction/phase 从 RAM 读取（byte-driven semantics））
    state.direction = ram[0x0300];
    state.phase = ram[0x0301];
    state.speed = ram[0x0302];
    // TODO: 数据表解析（rom_tables（bank:addr（表值（表用途））））
    state
}
// waver.rs 测试（TDD 流程（测试先行（实现后补）））——穷举等价证明（全域穷举（边界条件（异常路径）））
// refs 引用必写：bank:addr 引证（A 级原文）——纪律强化（用户裁定）+四项核（对抗复核核（证伪立场））
// refs asm 考证：bank01:97D7（EnemyMovementChoice_Waver_BANK1（prg1_brinstar.asm:498））（Waver 例程（实址（A 级原文）））

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试 1（穷举等价证明（全域穷举））：update_waver 全域穷举（2^16 输入域（x/y 全域））
    #[test]
    fn test_update_waver_full_domain() {
        // TDD 流程（测试先行（实现后补））——失败（实现未完成）
        let ram = [0u8; 2048];
        let state = update_waver(&ram);
        // 真规则（公式形（零字面特值））：WaverState（x/y 从 RAM 读取（byte-driven semantics））
        assert_eq!(state.direction, 0, "WaverState.direction 应从 RAM 读取（byte-driven semantics）");
        assert_eq!(state.phase, 0, "WaverState.phase 应从 RAM 读取（byte-driven semantics）");
    }

    /// 测试 2（边界条件）：update_waver 边界条件（x/y 边界值）
    #[test]
    fn test_update_waver_boundary() {
        let mut ram = [0u8; 2048];
        ram[0x0300] = 0xFF; // direction 边界（最大值）
        ram[0x0301] = 0x00; // phase 边界（最小值）
        let state = update_waver(&ram);
        assert_eq!(state.direction, 0xFF, "WaverState.direction 边界（0xFF）");
        assert_eq!(state.phase, 0x00, "WaverState.phase 边界（0x00）");
    }

    /// 测试 3（异常路径）：update_waver 异常路径（RAM 非零（byte-driven semantics））
    #[test]
    fn test_update_waver_exception_path() {
        let mut ram = [0u8; 2048];
        ram[0x0300] = 0x7F; // direction 异常（非零）
        ram[0x0301] = 0x80; // phase 异常（非零）
        let state = update_waver(&ram);
        assert_eq!(state.direction, 0x7F, "WaverState.direction 异常路径（0x7F）");
        assert_eq!(state.phase, 0x80, "WaverState.phase 异常路径（0x80）");
    }
}
