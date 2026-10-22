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
    // memmap 适配（RAM 态→MellowState）
    let mut state = MellowState::default();
    // TODO: 数据表解析（rom_tables（bank:addr（表值（表用途））））
    state
}
