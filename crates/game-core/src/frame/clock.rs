//! 虚拟时钟（确定性时间源；AGENTS D1 禁真实时间）。

/// 目标帧率（NES NTSC 60Hz——D6 保真期 NES_NTSC）。
pub const TARGET_FPS: u32 = 60;

/// 虚拟时钟：60Hz tick 计数（帧引擎唯一时间源）。
#[derive(Clone, Debug, Default)]
pub struct VirtualClock {
    ticks: u64,
}

impl VirtualClock {
    /// 新建零起点时钟。
    pub fn new() -> Self {
        Self { ticks: 0 }
    }

    /// 推进一 tick（=1 帧）。
    pub fn tick(&mut self) {
        self.ticks = self.ticks.wrapping_add(1);
    }

    /// 已推进 tick 数。
    pub fn ticks(&self) -> u64 {
        self.ticks
    }
}
