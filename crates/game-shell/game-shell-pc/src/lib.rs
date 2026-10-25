//! game-shell-pc —— PC 平台壳（win/linux/mac）。
//!
//! minifb 窗体（256×240 放大呈现+键盘）；帧钟=60Hz 粗步；音频=v1 桩（cpal 合成随件）。
//! 结构裁定（r24）：平台实现=独立 crate（本件）；契约+泛型环=game-platform。
//!
//! 键位（NES 手柄形）：X/Z=A/B，Enter=Start，RShift=Select，方向键=方向，Esc=退出。

use std::cell::RefCell;
use std::rc::Rc;

use game_core::frame::FrameEngine;
use game_core::render::render_frame;
use game_platform::runtime::GameDriver;
use game_platform::{
    AudioSink, FrameClock, InputSource, PadState, PlatformShell, VideoSink, FRAME_H, FRAME_W,
    FRAME_RGB_LEN,
};

/// 共享窗体（视频/输入两面共用 minifb::Window——Rc<RefCell> 借用安全面）。
type SharedWindow = Rc<RefCell<minifb::Window>>;

/// PC 视频面（RGB888→ARGB 呈现）。
pub struct PcVideo {
    window: SharedWindow,
    argb: Vec<u32>,
}

impl VideoSink for PcVideo {
    fn present(&mut self, frame: &[u8]) {
        for (i, px) in self.argb.iter_mut().enumerate() {
            let r = frame[i * 3] as u32;
            let g = frame[i * 3 + 1] as u32;
            let b = frame[i * 3 + 2] as u32;
            *px = (r << 16) | (g << 8) | b;
        }
        let mut w = self.window.borrow_mut();
        let _ = w.update_with_buffer(&self.argb, FRAME_W, FRAME_H);
    }

    fn pump_events(&mut self) -> bool {
        let mut w = self.window.borrow_mut();
        w.update();
        w.is_open() && !w.is_key_down(minifb::Key::Escape)
    }
}

/// PC 输入面（minifb 键盘→NES 位形）。
pub struct PcInput {
    window: SharedWindow,
}

impl InputSource for PcInput {
    fn poll(&mut self) -> (PadState, PadState) {
        use minifb::Key;
        let w = self.window.borrow();
        let mut p = PadState::default();
        p = p.with(PadState::A, w.is_key_down(Key::X));
        p = p.with(PadState::B, w.is_key_down(Key::Z));
        p = p.with(PadState::SELECT, w.is_key_down(Key::RightShift));
        p = p.with(PadState::START, w.is_key_down(Key::Enter));
        p = p.with(PadState::UP, w.is_key_down(Key::Up));
        p = p.with(PadState::DOWN, w.is_key_down(Key::Down));
        p = p.with(PadState::LEFT, w.is_key_down(Key::Left));
        p = p.with(PadState::RIGHT, w.is_key_down(Key::Right));
        (p, PadState::default())
    }
}

/// PC 音频面（v1 桩：写流记数不合成——cpal 合成随件登记）。
#[derive(Default)]
pub struct PcAudio {
    pub regs_seen: u64,
}

impl AudioSink for PcAudio {
    fn write_reg(&mut self, _addr: u16, _value: u8) {
        self.regs_seen += 1;
    }
    fn frame_boundary(&mut self) {}
}

/// PC 帧钟（60Hz 定步——粗步睡眠；精细定步随件）。
pub struct PcClock {
    next: std::time::Instant,
    period: std::time::Duration,
}

impl PcClock {
    pub fn new(fps: u32) -> Self {
        Self {
            next: std::time::Instant::now(),
            period: std::time::Duration::from_nanos(1_000_000_000 / fps.max(1) as u64),
        }
    }
}

impl FrameClock for PcClock {
    fn wait_next(&mut self) {
        let now = std::time::Instant::now();
        if now < self.next {
            std::thread::sleep(self.next - now);
        }
        self.next += self.period;
    }
}

/// PC 壳（PlatformShell 实现——窗体共用面装配）。
pub struct PcShell {
    pub video: PcVideo,
    pub audio: PcAudio,
    pub input: PcInput,
    pub clock: PcClock,
}

impl PcShell {
    /// 开壳（title=窗口标题；scale=放大倍数；fps=帧率（NTSC 60））。
    pub fn new(title: &str, scale: usize, fps: u32) -> std::io::Result<Self> {
        let window = minifb::Window::new(
            title,
            FRAME_W * scale,
            FRAME_H * scale,
            minifb::WindowOptions { resize: true, ..Default::default() },
        )
        .map_err(|e| std::io::Error::other(e.to_string()))?;
        let shared: SharedWindow = Rc::new(RefCell::new(window));
        Ok(Self {
            video: PcVideo { window: shared.clone(), argb: vec![0u32; FRAME_W * FRAME_H] },
            audio: PcAudio::default(),
            input: PcInput { window: shared },
            clock: PcClock::new(fps),
        })
    }
}

impl PlatformShell for PcShell {
    type Video = PcVideo;
    type Audio = PcAudio;
    type Input = PcInput;
    type Clock = PcClock;
    fn video(&mut self) -> &mut Self::Video {
        &mut self.video
    }
    fn audio(&mut self) -> &mut Self::Audio {
        &mut self.audio
    }
    fn input(&mut self) -> &mut Self::Input {
        &mut self.input
    }
    fn clock(&mut self) -> &mut Self::Clock {
        &mut self.clock
    }
}

/// CoreDriver —— game-core 引擎的 GameDriver 适配（机械三委托；渲染径=render-dump 金面同源）。
pub struct CoreDriver {
    pub engine: FrameEngine,
    rgb: Vec<u8>,
}

impl CoreDriver {
    pub fn new() -> Self {
        Self { engine: FrameEngine::new(), rgb: vec![0u8; FRAME_RGB_LEN] }
    }
}

impl Default for CoreDriver {
    fn default() -> Self {
        Self::new()
    }
}

impl GameDriver for CoreDriver {
    fn step(&mut self, joy1: u8, joy2: u8) {
        self.engine.mem.joy1 = joy1;
        self.engine.mem.joy2 = joy2;
        self.engine.tick_frame();
        let pal = &self.engine.mem.render.palette;
        let fb = render_frame(
            &self.engine.mem.render,
            &slice16(&pal[0..16]),
            &slice16(&pal[16..32]),
            &self.engine.mem.ram[0x200..0x300],
        );
        self.rgb.copy_from_slice(&fb);
    }

    fn video_frame(&self) -> &[u8] {
        &self.rgb
    }

    fn drain_apu(&mut self, out: &mut Vec<(u16, u8)>) {
        out.extend_from_slice(&self.engine.mem.apu_log);
        self.engine.mem.apu_log.clear();
    }
}

fn slice16(s: &[u8]) -> [u8; 16] {
    let mut out = [0u8; 16];
    out.copy_from_slice(s);
    out
}
