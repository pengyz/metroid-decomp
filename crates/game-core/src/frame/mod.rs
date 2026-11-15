//! game-core::frame —— S5 帧引擎骨架（task-10；two-layer-architecture.md 形态）。
//!
//! ① 60Hz tick / 虚拟时钟 / GameRng（确定性；AGENTS D1）；
//! ② 全内存模型（RAM 2KB 含栈页 + WRAM 8KB + 寄存器面）；
//! ③ unported! 桩阵——缺口确定性爆在 bank:addr（AGENTS D7 机制②）；
//! ④ boot 序列**执行序**端口（每引爆一个端一个；L2 管线按需补给）；
//! ⑤ L3 双腿（game-core 腿 dump ⇄ oracle 腿 trace，tools/diff_traces.py 裁定）。
//!
//! 执行序保真口径（栈页实证）：L3 watch 含栈页 $0100-$01FF，oracle 帧界存稳定残迹
//! （$01EB-$01FF 19 非零字节跨 292 帧——偏离帧=开机段 0-7；$01FC-$01FE 三字节值漂移）——调用影子（驱动层）须复刻原调用结构的
//! 栈写面；产品层域逻辑零栈字面量（机械址只在驱动/memmap）。

/// B 跑首分歧记账面（占位臂表址收集——分类报告输入；thread-local 每引擎实例）。
thread_local! {
    pub(crate) static DIVERGENCES: std::cell::RefCell<Vec<u16>> =
        std::cell::RefCell::new(Vec::new());
}

pub mod boot;
pub mod clock;
pub mod cycles; // 6502 逐条周期计数底座（路线丙 ②）
pub mod engine;
pub mod tick;
#[cfg(test)]
mod tests;
pub mod memory;
pub mod nmi;
pub mod rng;

pub use clock::{VirtualClock, TARGET_FPS};
pub use engine::FrameEngine;
pub use crate::render::gfx::init_title_gfx;
pub use tick::{main_loop_iter, samus_init};
pub use crate::title::intro::star_palette_switch;
pub use boot::io;
pub use memory::{FrameMemory, Registers};
pub use nmi::nmi_handler;
pub use rng::GameRng;

/// 当前区序号（`InArea & $0F`：0=Br/1=No/2=Kr/3=To/4=Ri）。
///
/// 敌九表按**区 bank** 存放（五区同址 `$962B…$977B`，内容不同）⇒ 所有敌表读取都必须
/// 先取区号。`InArea` 高半字节是「进区方向」等附加位，故统一掩 `$0F`。
pub fn cur_area(mem: &FrameMemory) -> u8 {
    mem.read_ram(crate::ram_map::IN_AREA) & 0x0F
}

/// 区序号 → MMC1 bank 号（`BankTable ; 07:CA30` = `02 03 05 04 06` 减一）。
///
/// 0=Br→1 / 1=No→2 / 2=Kr→4 / 3=To→3 / 4=Ri→5。**注意 Kr/To 的 bank 序与区序相反**，
/// 任何「area+1」式推算都会把这两区弄反（`680507b9` 修过一次同类）。
pub fn area_bank(area: u8) -> u8 {
    match area & 0x0F {
        0 => 1,
        1 => 2,
        2 => 4,
        3 => 3,
        4 => 5,
        _ => crate::unported!("bank07:CA30-area_bank"),
    }
}
