//! sound::sfx（音效）——批量改写·第七簇（六步流程（域内行为簇））
//!
//! 六步流程：定域取证 spec→域结构→memmap 适配→shim 写集适配→测试→registry 登记+独立复审
//! refs 引用必写（bank:addr 引证（A 级原文））

/// SfxState（域结构（语义命名（data/naming 台账 A/B 级）））
#[derive(Debug, Clone, Default)]
pub struct SfxState {
    /// 音效 ID
    pub sfx_id: u8,
    /// 音效状态（播放中/停止）
    pub playing: bool,
    /// 音效帧计数
    pub frame_count: u8,
}

/// 音效例程（bank:addr（实址（A 级原文）））
///
/// 六步流程：
/// 1. 定域取证 spec（bank:addr（音效例程（实址（A 级原文））））
/// 2. 域结构（语义命名（data/naming 台账 A/B 级））
/// 3. memmap 适配（RAM 态→SfxState）
/// 4. shim 写集适配（SfxState→RAM 写回）
/// 5. 测试（穷举等价证明（先例））
/// 6. registry 登记（l2ports.rs（bank:addr→crate::module::fn 双射））
pub fn play_sfx(ram: &[u8; 2048], sfx_id: u8) -> SfxState {
    // memmap 适配（RAM 态→SfxState）
    let mut state = SfxState::default();
    state.sfx_id = sfx_id;
    state.playing = true;
    // TODO: 数据表解析（rom_tables（bank:addr（表值（表用途））））
    state
}
