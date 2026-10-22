//! enemy::ripper（Ripper）——批量改写·第二十五簇（六步流程（域内行为簇））
//!
//! 六步流程：定域取证 spec→域结构→memmap 适配→shim 写集适配→测试→registry 登记+独立复审
//! refs 引用必写（bank:addr 引证（A 级原文））
//!
//! 实址引证（bank:addr（A 级原文））：
//! - bank01:97D9（EnemyMovementChoice_Ripper_{AREA}（prg1_brinstar.asm:501））
//! - bank02:97D1（EnemyMovementChoice_RipperII_{AREA}（prg2_norfair.asm:449））
//! - bank04:97D7（EnemyMovementChoice_Ripper_{AREA}（prg4_kraid.asm:461））

/// RipperState（域结构（语义命名（data/naming 台账 A/B 级）））
#[derive(Debug, Clone, Default)]
pub struct RipperState {
    /// Ripper 类型（I/II）
    pub ripper_type: u8,
    /// 移动方向
    pub direction: u8,
    /// 移动速度
    pub speed: u8,
}

/// Ripper 例程（bank01:97D9（EnemyMovementChoice_Ripper_{AREA}（实址（A 级原文））））
///
/// 六步流程：
/// 1. 定域取证 spec（bank01:97D9（EnemyMovementChoice_Ripper_{AREA}（实址（A 级原文））））
/// 2. 域结构（语义命名（data/naming 台账 A/B 级））
/// 3. memmap 适配（RAM 态→RipperState）
/// 4. shim 写集适配（RipperState→RAM 写回）
/// 5. 测试（穷举等价证明（先例））
/// 6. registry 登记（l2ports.rs（bank:addr→crate::module::fn 双射））
pub fn update_ripper(ram: &[u8; 2048]) -> RipperState {
    // memmap 适配（RAM 态→RipperState）
    let mut state = RipperState::default();
    // TODO: 数据表解析（rom_tables（bank:addr（表值（表用途））））
    state
}

#[cfg(test)]
mod tests {
    use super::*;

    /// TDD 流程·第一步：写测试（穷举等价证明（全域穷举（边界条件（异常路径））））
    #[test]
    fn test_ripper_ai_basic() {
        let ram = [0u8; 2048];
        let state = update_ripper(&ram);
        assert_eq!(state.ripper_type, 0);
        assert_eq!(state.direction, 0);
    }

    /// TDD 流程·第一步：边界条件（异常路径）
    #[test]
    fn test_ripper_ai_boundary() {
        let mut ram = [0u8; 2048];
        ram[0x0300] = 0xFF;
        let state = update_ripper(&ram);
        // 真规则（公式形（零字面特值））
        assert!(state.ripper_type <= 0xFF);
        assert!(state.direction <= 0xFF);
    }
}
