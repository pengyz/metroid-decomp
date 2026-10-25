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
pub mod engine;
pub mod gfx;
pub mod mainloop;
pub mod intro;
pub mod memory;
pub mod nmi;
pub mod rng;
pub mod sound_hook;
pub mod render;
pub mod statue;
pub mod title_bridge;
pub mod vram;
pub mod room; // r25 房间绘制引擎（DrawRoom \ 族）

pub use clock::{VirtualClock, TARGET_FPS};
pub use engine::FrameEngine;
pub use gfx::init_title_gfx;
pub use mainloop::main_loop_iter;
pub use intro::star_palette_switch;
pub use boot::io;
pub use memory::{FrameMemory, Registers};
pub use nmi::nmi_handler;
pub use rng::GameRng;
