//! pickup::items（道具拾取）——批量改写·第五簇（六步流程（域内行为簇））
//!
//! 六步流程：定域取证 spec→域结构→memmap 适配→shim 写集适配→测试→registry 登记+独立复审
//! refs 引用必写（bank:addr 引证（A 级原文））

/// ItemState（域结构（语义命名（data/naming 台账 A/B 级）））
#[derive(Debug, Clone, Default)]
pub struct ItemState {
    /// 道具 ID
    pub item_id: u8,
    /// 道具位置
    pub x: u8,
    pub y: u8,
    /// 道具状态（已拾取/未拾取）
    pub collected: bool,
}

/// 道具拾取例程（bank:addr（实址（A 级原文）））
///
/// 六步流程：
/// 1. 定域取证 spec（bank:addr（道具拾取例程（实址（A 级原文））））
/// 2. 域结构（语义命名（data/naming 台账 A/B 级））
/// 3. memmap 适配（RAM 态→ItemState）
/// 4. shim 写集适配（ItemState→RAM 写回）
/// 5. 测试（穷举等价证明（先例））
/// 6. registry 登记（l2ports.rs（bank:addr→crate::module::fn 双射））
pub fn check_item_pickup(ram: &[u8; 2048], samus_x: u8, samus_y: u8) -> Option<ItemState> {
    // memmap 适配（RAM 态→ItemState）
    // TODO: 数据表解析（rom_tables（bank:addr（表值（表用途））））
    None
}
