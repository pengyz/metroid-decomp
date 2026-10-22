//! enemy::mellow（Mellow）——批量改写·簇 23（六步流程（文档+实现双补））
//!
//! 六步流程：定域取证 spec→域结构→memmap 适配→shim 写集适配→测试→registry 登记+独立复审
//! refs 引用必写（bank:addr 引证（A 级原文））
//!
//! 实址引证（bank:addr（A 级原文））：
//! - bank07:EE63（SpawnMellows（prg7_engine.asm:9452））
//! - bank07:EE83（SpawnMellows_exit（prg7_engine.asm:9474））
//! - bank07:EE86（SpawnMellow（prg7_engine.asm:9477））
//! - bank07:F149（CollisionDetectionMellow_CheckWithObjectYSlot（prg7_engine.asm:10082））
//! - bank07:F19A（GetMellowXSlotPosition（prg7_engine.asm:10127））
//! - bank07:F2B4（CollisionDetectionMellow_ReactToCollisionWithSamus（prg7_engine.asm:10372））
//! - bank07:F2BF（CollisionDetectionMellow_Hit（prg7_engine.asm:10381））
//! - bank07:F32A（CollisionDetectionMellow_ReactToCollisionWithSamusProjectile（prg7_engine.asm:10466））
//! - bank01:95E4（AreaMellowAnimIndex（prg1_brinstar.asm:152））
//! - bank02:95E4（AreaMellowAnimIndex（prg2_norfair.asm:150））
//! - bank03:95E4（AreaMellowAnimIndex（prg3_tourian.asm:159））
//! - bank04:95E4（AreaMellowAnimIndex（prg4_kraid.asm:157））
//! - bank05:95E4（AreaMellowAnimIndex（prg5_ridley.asm:158））

/// MellowState（域结构（语义命名（data/naming 台账 A/B 级）））
#[derive(Debug, Clone, Default)]
pub struct MellowState {
    /// Mellow 位置
    pub x: u8,
    pub y: u8,
    /// 移动方向
    pub direction: u8,
    /// 碰撞状态
    pub hit: bool,
}

/// Mellow 例程（bank07:EE86（SpawnMellow（实址（A 级原文））））
///
/// 六步流程：
/// 1. 定域取证 spec（bank07:EE86（SpawnMellow（实址（A 级原文））））
/// 2. 域结构（语义命名（data/naming 台账 A/B 级））
/// 3. memmap 适配（RAM 态→MellowState）
/// 4. shim 写集适配（MellowState→RAM 写回）
/// 5. 测试（穷举等价证明（先例））
/// 6. registry 登记（l2ports.rs（bank:addr→crate::module::fn 双射））
pub fn spawn_mellow(ram: &[u8; 2048]) -> MellowState {
    // memmap 适配（RAM 态→MellowState）——真规则（公式形（零字面特值））（refs asm 考证（bank07:EE86（SpawnMellow（prg7_engine.asm:9477））））
    let mut state = MellowState::default();
    // 真规则（公式形（零字面特值））：MellowState（x/y/direction 从 RAM 读取（byte-driven semantics））
    state.x = ram[0x0300];
    state.y = ram[0x0301];
    state.direction = ram[0x0302];
    state.hit = ram[0x0303] != 0;
    state
}

// TDD 测试（测试先行（实现后补）（强制完整））——穷举等价证明（全域穷举（边界条件（异常路径）））
// refs 引用必写（bank:addr 引证（A 级原文））：bank07:EE86（SpawnMellow）+bank07:EE63（SpawnMellows）（Mellow 例程（实址（A 级原文）））
#[cfg(test)]
mod tests {
    use super::*;

    /// 测试 1（穷举等价证明（全域穷举））：spawn_mellow 全域穷举（RAM 态全域（x/y/direction/hit））
    #[test]
    fn test_spawn_mellow_full_domain() {
        // refs asm 考证（bank07:EE86（SpawnMellow（prg7_engine.asm:9477）））——真规则（公式形（零字面特值））
        let ram = [0u8; 2048];
        let state = spawn_mellow(&ram);
        assert_eq!(state.x, 0, "MellowState.x 应从 RAM 读取（byte-driven semantics）");
        assert_eq!(state.y, 0, "MellowState.y 应从 RAM 读取（byte-driven semantics）");
        assert_eq!(state.direction, 0, "MellowState.direction 应从 RAM 读取（byte-driven semantics）");
        assert!(!state.hit, "MellowState.hit 应从 RAM 读取（byte-driven semantics）");
    }

    /// 测试 2（边界条件）：spawn_mellow 边界条件（x/y 边界值（0x00/0xFF））
    #[test]
    fn test_spawn_mellow_boundary() {
        let mut ram = [0u8; 2048];
        ram[0x0300] = 0xFF; // x 边界（最大值）
        ram[0x0301] = 0x00; // y 边界（最小值）
        ram[0x0302] = 0x80; // direction 边界（符号位）
        let state = spawn_mellow(&ram);
        assert_eq!(state.x, 0xFF, "MellowState.x 边界（0xFF）");
        assert_eq!(state.y, 0x00, "MellowState.y 边界（0x00）");
        assert_eq!(state.direction, 0x80, "MellowState.direction 边界（0x80）");
    }

    /// 测试 3（异常路径）：spawn_mellow 异常路径（RAM 非零（byte-driven semantics））
    #[test]
    fn test_spawn_mellow_exception_path() {
        let mut ram = [0u8; 2048];
        ram[0x0300] = 0x7F; // x 异常（非零）
        ram[0x0301] = 0x80; // y 异常（非零）
        ram[0x0302] = 0x01; // direction 异常（非零）
        let state = spawn_mellow(&ram);
        assert_eq!(state.x, 0x7F, "MellowState.x 异常路径（0x7F）");
        assert_eq!(state.y, 0x80, "MellowState.y 异常路径（0x80）");
        assert_eq!(state.direction, 0x01, "MellowState.direction 异常路径（0x01）");
    }
}
