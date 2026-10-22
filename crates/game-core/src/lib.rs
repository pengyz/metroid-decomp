//! game-core —— 纯逻辑核（AGENTS.md D1/D7；无 I/O、确定性）。
//!
//! 两层架构（docs/spec/two-layer-architecture.md）：产品层（enemy/ 等域模块，零地址字面量）
//! + 映射层（memmap/，地址只此一层）+ 验证适配层（verify_shim/，域方法→L2In/L2Out 写集契约）。
//! L2 三方全等裁定：case.out(golden) == 6502 核执行 ROM 原字节 == 本 crate 同名函数
//! （l2ports 注册表机械名只在注册注释，产品层零机械名）。

pub mod gap;
pub mod integrators;
pub mod enemy;
pub mod frame;
pub mod memmap;
pub mod ram_map;
pub mod samus;
pub mod rng;
pub mod tile_blast;
pub mod engine;
pub mod world;
pub mod pickup;
pub mod ui;
pub mod render;
pub mod sound;
pub mod title;
pub mod verify_shim;
pub mod l2ports;

/// L2 入参：寄存器 + 2KB RAM 快照（$0000-$07FF，镜像归一后）+ 窗口态。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct L2In {
    /// $6000-$7FFF WRAM（8KB，敌槽/暂存区——见 findings/ram-region-6af4-contradiction.md；缺省空=全 0）
    pub wram: Vec<u8>,
    pub a: u8,
    pub x: u8,
    pub y: u8,
    pub p: u8,
    pub ram: Vec<u8>,
    /// $8000 窗口 bank（MMC1 16KB 窗口，契约 ∈0..8）——窗口读表取数面（task-14 窗口参数化，
    /// A9 证伪处置；decode-sweep-r3-dynamic-close.md §1.2 A 级：game 期窗口=banks1-5 区域
    /// bank）。L2 契约 case `window_bank` 字段即本面；缺省=7=entry bank 标签缺省约定
    /// （bank07 用例行为钉死），runner 侧 l2-diff 按 case 字段/entry 标签填充。
    pub window_bank: usize,
}

impl Default for L2In {
    fn default() -> Self {
        Self {
            wram: vec![0; 0x2000],
            a: 0,
            x: 0,
            y: 0,
            p: 0x24,
            ram: vec![0; 0x800],
            window_bank: 7,
        }
    }
}

/// L2 出参：寄存器（a/x/y；p 为执行副产物不入比对面）+ 写集（addr 归一至 $0000-$07FF）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct L2Out {
    pub a: u8,
    pub x: u8,
    pub y: u8,
    pub writes: Vec<(u16, u8)>,
}
