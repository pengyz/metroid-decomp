//! input::joy（输入）——批量改写·第十二簇（六步流程（域内行为簇））
//!
//! 六步流程：定域取证 spec→域结构→memmap 适配→shim 写集适配→测试→registry 登记+独立复审
//! refs 引用必写（bank:addr 引证（A 级原文））

/// JoyState（域结构（语义命名（data/naming 台账 A/B 级）））
#[derive(Debug, Clone, Default)]
pub struct JoyState {
    /// 按键状态（A/B/Select/Start/Up/Down/Left/Right）
    pub buttons: u8,
    /// 按键序列（跳跃（A）/变球（下））
    pub sequence: [u8; 16],
}

/// 输入例程（bank:addr（实址（A 级原文）））
///
/// 六步流程：
/// 1. 定域取证 spec（bank:addr（输入例程（实址（A 级原文））））
/// 2. 域结构（语义命名（data/naming 台账 A/B 级））
/// 3. memmap 适配（RAM 态→JoyState）
/// 4. shim 写集适配（JoyState→RAM 写回）
/// 5. 测试（穷举等价证明（先例））
/// 6. registry 登记（l2ports.rs（bank:addr→crate::module::fn 双射））
pub fn read_joypads(ram: &[u8; 2048]) -> JoyState {
    // memmap 适配（RAM 态→JoyState）
    let mut state = JoyState::default();
    state.buttons = ram[0x0000];
    // TODO: 数据表解析（rom_tables（bank:addr（表值（表用途））））
    state
}
