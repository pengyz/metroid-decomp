//! game-platform —— 平台壳契约层（AGENTS §2：game-platform（trait））。
//!
//! game-core 零平台依赖（确定性纯逻辑）；本 crate 定义**壳层要实现的 trait**与共享
//! 类型（D2 A 阶段预留扩展缝的落点：分辨率/输入/音频/存档/UI），外加写给 trait 的
//! **泛型壳运行环**（`runtime`——零平台依赖，全平台共用编排）。
//!
//! 结构裁定（r24 辩证分析）：平台实现=**独立 crate**（`game-shell/` 目录制——
//! game-shell-pc/game-shell-android/…各持平台依赖与产物形态）；本 crate 只持契约+环。
//!
//! 契约要点：
//! - 帧节拍：`FrameClock::wait_next` 一次/帧（60Hz NTSC 定步）；
//! - 视频：`VideoSink::present` 收 256×240 RGB888（game_core::render::render_frame 出口形）；
//! - 输入：`InputSource::poll` 出 NES 手柄位形（$4016 串行位序）；
//! - 音频：`AudioSink` 收 APU 寄存器写流（$4000-$4017）+帧界（壳侧合成）。

pub mod runtime;
pub use runtime::{run_frame, run_game, GameDriver};

/// NES 手柄位形（**hardware.asm:89-96 内容锚**：bit0=Right … bit7=A——与引擎 joy1/
/// Joy1Change 字节同形；r25 勘误：旧位序反（Enter 被读成 UP））。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PadState {
    pub bits: u8,
}

impl PadState {
    pub const RIGHT: u8 = 0x01;
    pub const LEFT: u8 = 0x02;
    pub const DOWN: u8 = 0x04;
    pub const UP: u8 = 0x08;
    pub const START: u8 = 0x10;
    pub const SELECT: u8 = 0x20;
    pub const B: u8 = 0x40;
    pub const A: u8 = 0x80;

    /// 按键置位。
    pub fn with(mut self, button: u8, pressed: bool) -> Self {
        if pressed {
            self.bits |= button;
        } else {
            self.bits &= !button;
        }
        self
    }
}

/// 帧缓冲面（256×240 RGB888——`game_core::render::render_frame` 出口形）。
pub const FRAME_W: usize = 256;
pub const FRAME_H: usize = 240;
pub const FRAME_RGB_LEN: usize = FRAME_W * FRAME_H * 3;

/// 视频面：帧呈现 + 事件泵（false=用户请求退出）。
pub trait VideoSink {
    fn present(&mut self, frame: &[u8]);
    fn pump_events(&mut self) -> bool {
        true
    }
}

/// 音频面：APU 寄存器写流（$4000-$4017）+ 帧界（壳侧合成/缓冲）。
pub trait AudioSink {
    fn write_reg(&mut self, addr: u16, value: u8);
    /// 帧界标（帧步进一次——合成器的节拍/缓冲面对齐点）。
    fn frame_boundary(&mut self);
}

/// 输入面：手柄位形轮询（每帧一次）。
pub trait InputSource {
    fn poll(&mut self) -> (PadState, PadState);
}

/// 帧钟：60Hz 定步（NTSC——与引擎虚拟时钟对齐）。
pub trait FrameClock {
    fn wait_next(&mut self);
}

/// 平台宿主聚合（壳实现此 trait——每平台一件）。
pub trait PlatformShell {
    type Video: VideoSink;
    type Audio: AudioSink;
    type Input: InputSource;
    type Clock: FrameClock;

    fn video(&mut self) -> &mut Self::Video;
    fn audio(&mut self) -> &mut Self::Audio;
    fn input(&mut self) -> &mut Self::Input;
    fn clock(&mut self) -> &mut Self::Clock;
}

#[cfg(test)]
mod tests {
    use super::*;

    /// PadState 位形语义（hardware.asm:89-96 位序——A=bit7/Start=bit4/Right=bit0）。
    #[test]
    fn pad_state_bit_order() {
        let p = PadState::default().with(PadState::A, true).with(PadState::RIGHT, true);
        assert_eq!(p.bits, 0x81, "A(0x80)|Right(0x01)");
        let start = PadState::default().with(PadState::START, true);
        assert_eq!(start.bits, 0x10, "Start=bit4（head 检 `and #$10` 面）");
        let p2 = p.with(PadState::A, false);
        assert_eq!(p2.bits, 0x01);
    }

    /// 帧缓冲常量锚（256×240 RGB888——render_frame 出口形自证）。
    #[test]
    fn frame_geometry() {
        assert_eq!(FRAME_RGB_LEN, 256 * 240 * 3);
        assert_eq!(FRAME_W, 256);
        assert_eq!(FRAME_H, 240);
    }
}
