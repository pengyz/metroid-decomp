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
pub mod verify_shim;
pub mod l2ports;

/// L2 入参：寄存器 + 2KB RAM 快照（$0000-$07FF，镜像归一后）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct L2In {
    /// $6000-$7FFF WRAM（8KB，敌槽/暂存区——见 findings/ram-region-6af4-contradiction.md；缺省空=全 0）
    pub wram: Vec<u8>,
    pub a: u8,
    pub x: u8,
    pub y: u8,
    pub p: u8,
    pub ram: Vec<u8>,
}

impl Default for L2In {
    fn default() -> Self {
        Self { wram: vec![0; 0x2000], a: 0, x: 0, y: 0, p: 0x24, ram: vec![0; 0x800] }
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
