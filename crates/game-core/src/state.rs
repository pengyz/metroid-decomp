//! `game-core::state` —— 周期态类型层（G11 类型下沉——r19）。
//!
//! 【消环声明】`FrameMemory`/`RenderState`/`Registers` 的**定义**归本层（frame/ 之下），
//! 域件（samus/title/…）与 render/ 经本层取型 ⇒ **frame↔域件环消除**（原域件反向借用
//! frame 类型）。方法实现（impl 块）留原位（frame/memory.rs、frame/render.rs——同 crate
//! 跨模块 impl 合法）；frame/ 保留 re-export（兼容径）。依赖方向正典：域件←frame 会师
//! 调用；类型依赖一律向下（state/memmap）。

/// 寄存器面（6502 程序员态 + PC）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Registers {
    pub a: u8,
    pub x: u8,
    pub y: u8,
    pub p: u8,
    pub sp: u8,
    pub pc: u16,
}

/// 全内存模型（定义=态层；方法实现见 frame/memory.rs）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FrameMemory {
    /// RAM 2KB 归一镜像面（$0000-$07FF）。
    pub ram: [u8; 0x800],
    /// WRAM 8KB（$6000-$7FFF）。
    pub wram: [u8; 0x2000],
    /// 寄存器面。
    pub regs: Registers,
    /// NMI 使能态（PPUCTRL bit7——write_io($2000) 权威面；60Hz VBlank 调度开关）。
    pub nmi_enabled: bool,
    /// title 子步游标（跨帧件段索引——port2 契约面：next_cursor>0=spin 续点/0=完成复位）。
    pub title_cursor: u8,
    /// 续段归属相位（**同相位续段判定**（r14 复核 F4）：cursor 由该相位的段序列发出——
    /// 相位推进后残留的 cursor=孤儿段，下 tick=新相位首个真迭代（禁误门为续段））。
    pub title_cursor_phase: u8,
    /// 手柄 1 输入面（场景驱动；boot-title=无输入 0）。
    pub joy1: u8,
    /// 手柄 2 输入面。
    pub joy2: u8,
    /// 渲染态（S6-6/7 建模+S6-10 喂入面——L4 视觉腿底座）。
    pub render: RenderState,
}

impl Default for FrameMemory {
    fn default() -> Self {
        Self { ram: [0; 0x800], wram: [0; 0x2000], regs: Registers::default(), nmi_enabled: false,
               title_cursor: 0, title_cursor_phase: 0, joy1: 0, joy2: 0,
               render: RenderState::default() }
    }
}

/// 渲染态（PPU 地址空间视图；方法实现见 frame/render.rs）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RenderState {
    /// CHR-RAM 8KB（图案表）。
    pub chr: Vec<u8>,
    /// 名表 2KB（$2000-$2FFF）。
    pub nametable: Vec<u8>,
    /// PPUADDR 当前地址（$2006 双写累积）。
    pub ppuaddr: u16,
    /// $2006 flip-flop（true=下一写为 hi）。
    pub latch_hi: bool,
    /// PPU 调色板 RAM（$3F00-$3F1F 32B——S6-10 渲染腿供面）。
    pub palette: [u8; 32],
    /// PPUCTRL（$2000）寄存器面——bit4=BG 图案表选、bit3=精灵图案表选、bit5=精灵尺寸。
    pub ppuctrl: u8,
}

impl Default for RenderState {
    fn default() -> Self {
        Self { chr: vec![0; 0x2000], nametable: vec![0; 0x800], ppuaddr: 0, latch_hi: true,
               palette: [0x0F; 32], ppuctrl: 0 }
    }
}
