//! game-shell-pc —— PC 平台壳（win/linux/mac）。
//!
//! minifb 窗体（256×240 放大呈现+键盘）；帧钟=60Hz 粗步；音频=v1 桩（cpal 合成随件）。
//! 结构裁定（r24）：平台实现=独立 crate（本件）；契约+泛型环=game-platform。
//!
//! 键位（NES 手柄形）：X/Z=A/B，Enter=Start，RShift=Select，方向键=方向，Esc=退出。

pub mod script;

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

/// PC 音频面：接 `game_platform::apu::Apu` 真合成（设备输出=cpl 随件；当前落 PCM 缓冲+WAV）。
///
/// 帧界协定：`frame_boundary` 走满一帧的 CPU 周期（`CPU_HZ/60`），
/// 与引擎"每帧一次 NMI"的节拍对齐——这是 APU 时间轴的唯一驱动点。
pub struct PcAudio {
    pub regs_seen: u64,
    apu: game_platform::apu::Apu,
    pcm: Vec<i16>,
    frames: u64,
}

impl Default for PcAudio {
    fn default() -> Self {
        Self { regs_seen: 0, apu: game_platform::apu::Apu::new(), pcm: Vec::new(), frames: 0 }
    }
}

impl PcAudio {
    /// 取走已合成 PCM（44.1 kHz 单声道 i16）。
    pub fn take_pcm(&mut self) -> Vec<i16> {
        core::mem::take(&mut self.pcm)
    }

    /// 已推进的帧数。
    pub fn frames(&self) -> u64 {
        self.frames
    }

    /// 把 PCM 落成 WAV（16-bit mono 44.1 kHz；零依赖，供人耳/工具核对）。
    pub fn write_wav(path: &std::path::Path, pcm: &[i16]) -> std::io::Result<()> {
        use std::io::Write;
        let mut f = std::io::BufWriter::new(std::fs::File::create(path)?);
        let data_len = (pcm.len() * 2) as u32;
        let rate = game_platform::apu::SAMPLE_RATE;
        f.write_all(b"RIFF")?;
        f.write_all(&(36 + data_len).to_le_bytes())?;
        f.write_all(b"WAVEfmt ")?;
        f.write_all(&16u32.to_le_bytes())?;
        f.write_all(&1u16.to_le_bytes())?; // PCM
        f.write_all(&1u16.to_le_bytes())?; // mono
        f.write_all(&rate.to_le_bytes())?;
        f.write_all(&(rate * 2).to_le_bytes())?;
        f.write_all(&2u16.to_le_bytes())?;
        f.write_all(&16u16.to_le_bytes())?;
        f.write_all(b"data")?;
        f.write_all(&data_len.to_le_bytes())?;
        for s in pcm {
            f.write_all(&s.to_le_bytes())?;
        }
        f.flush()
    }
}

impl AudioSink for PcAudio {
    fn write_reg(&mut self, addr: u16, value: u8) {
        self.regs_seen += 1;
        self.apu.write_reg(addr, value);
    }
    fn frame_boundary(&mut self) {
        // 一帧 = CPU_HZ/60 ≈ 29829 CPU 周期（NTSC 60.0988 Hz 的整数近似）。
        const CYCLES_PER_FRAME: u32 = game_platform::apu::CPU_HZ / 60;
        for _ in 0..CYCLES_PER_FRAME {
            self.apu.clock_cpu();
        }
        self.pcm.extend_from_slice(&self.apu.take_samples());
        self.frames += 1;
    }
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
            self.engine.mem.oam_shadow(),
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

#[cfg(test)]
mod audio_tests {
    use super::*;

    /// 帧界推进一帧：PCM 长度≈SAMPLE_RATE/60（44.1k ⇒ 735 采样/帧）。
    #[test]
    fn frame_boundary_yields_one_frame_of_pcm() {
        let mut a = PcAudio::default();
        a.frame_boundary();
        let pcm = a.take_pcm();
        let expect = game_platform::apu::SAMPLE_RATE as usize / 60;
        assert!(
            (pcm.len() as i64 - expect as i64).abs() <= 2,
            "每帧采样应 ≈{expect}，实得 {}",
            pcm.len()
        );
        assert_eq!(a.frames(), 1);
    }

    /// 寄存器写入经壳进入合成器：使能 pulse + 设音量后，PCM 必须非全零。
    #[test]
    fn registers_reach_synthesizer_and_produce_audio() {
        let mut a = PcAudio::default();
        a.write_reg(0x4015, 0x01);
        a.write_reg(0x4000, 0xBF); // duty 50%、恒定音量 15
        a.write_reg(0x4002, 0x80);
        a.write_reg(0x4003, 0x00);
        for _ in 0..3 {
            a.frame_boundary();
        }
        let pcm = a.take_pcm();
        assert!(pcm.iter().any(|&v| v > 0), "pulse 使能后应有非零采样");
    }

    /// WAV 头部字段正确且长度 == 44 + 2×采样数。
    #[test]
    fn wav_roundtrip_header() {
        let dir = std::env::temp_dir();
        let path = dir.join("apu_test.wav");
        let pcm: Vec<i16> = vec![0, 1000, -1000, 32767];
        PcAudio::write_wav(&path, &pcm).unwrap();
        let bytes = std::fs::read(&path).unwrap();
        assert_eq!(&bytes[0..4], b"RIFF");
        assert_eq!(&bytes[8..12], b"WAVE");
        assert_eq!(u32::from_le_bytes(bytes[4..8].try_into().unwrap()) as usize, 36 + pcm.len() * 2);
        assert_eq!(u32::from_le_bytes(bytes[40..44].try_into().unwrap()) as usize, pcm.len() * 2);
        assert_eq!(bytes.len(), 44 + pcm.len() * 2);
        let _ = std::fs::remove_file(&path);
    }
}
