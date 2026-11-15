//! 敌人引擎层（frame 侧）：每帧调度、绘制、移动壳。
//!
//! 与 `crate::enemy`（域层）的分工：`crate::enemy` 只放**纯逻辑**（表、Δ 计算、
//! 朝向位运算），不碰帧内存的绘制/动画/碰撞申请；本模块负责"把域层算出的东西
//! 施加到 FrameMemory 并驱动渲染"，所以它必须留在 frame 侧（G11：域件禁 `use
//! crate::frame::`）。

pub(crate) mod move_shell;
pub(crate) mod update;
