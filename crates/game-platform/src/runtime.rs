//! 泛型壳运行环（写给 trait 的可移植编排——零平台依赖）。
//!
//! 环序契约（TDD 钉死）：事件泵→输入→驱动 step→APU 流→音频面→视频呈现→帧钟。

use crate::{AudioSink, FrameClock, InputSource, PlatformShell, VideoSink};

/// 游戏驱动界面（壳对引擎的消费面——引擎适配器实现此 trait）。
///
/// 契约：`step` 一帧（输入位形进→引擎 tick+渲染）；`video_frame` 出 256×240 RGB888；
/// `drain_apu` 出本帧 APU 寄存器写流（$4000-$4017 面）。
pub trait GameDriver {
    fn step(&mut self, joy1: u8, joy2: u8);
    fn video_frame(&self) -> &[u8];
    fn drain_apu(&mut self, out: &mut Vec<(u16, u8)>);
}

/// 一帧的壳侧编排（可测核心）。
pub fn run_frame<D: GameDriver, S: PlatformShell>(
    driver: &mut D,
    shell: &mut S,
    apu_scratch: &mut Vec<(u16, u8)>,
) -> bool {
    if !shell.video().pump_events() {
        return false; // 用户请求退出
    }
    let (joy1, joy2) = shell.input().poll();
    driver.step(joy1.bits, joy2.bits);
    apu_scratch.clear();
    driver.drain_apu(apu_scratch);
    {
        let audio = shell.audio();
        for &(a, v) in apu_scratch.iter() {
            audio.write_reg(a, v);
        }
        audio.frame_boundary();
    }
    let frame = driver.video_frame();
    shell.video().present(frame);
    shell.clock().wait_next();
    true
}

/// 壳主环（退出=事件泵 false 或帧数上限）。
pub fn run_game<D: GameDriver, S: PlatformShell>(
    driver: &mut D,
    shell: &mut S,
    max_frames: Option<u64>,
) -> u64 {
    let mut apu_scratch = Vec::with_capacity(64);
    let mut frames = 0u64;
    while max_frames.map_or(true, |m| frames < m) {
        if !run_frame(driver, shell, &mut apu_scratch) {
            break;
        }
        frames += 1;
    }
    frames
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{PadState, FRAME_RGB_LEN};

    #[derive(Default)]
    struct MockVideo {
        presented: Vec<usize>,
        alive: bool,
    }
    impl VideoSink for MockVideo {
        fn present(&mut self, frame: &[u8]) {
            self.presented.push(frame.len());
        }
        fn pump_events(&mut self) -> bool {
            let a = self.alive;
            self.alive = !self.alive;
            a
        }
    }
    #[derive(Default)]
    struct MockAudio {
        regs: Vec<(u16, u8)>,
        boundaries: u32,
    }
    impl AudioSink for MockAudio {
        fn write_reg(&mut self, addr: u16, value: u8) {
            self.regs.push((addr, value));
        }
        fn frame_boundary(&mut self) {
            self.boundaries += 1;
        }
    }
    #[derive(Default)]
    struct MockInput;
    impl InputSource for MockInput {
        fn poll(&mut self) -> (PadState, PadState) {
            (PadState::default().with(PadState::START, true), PadState::default())
        }
    }
    #[derive(Default)]
    struct MockClock {
        ticks: u32,
    }
    impl FrameClock for MockClock {
        fn wait_next(&mut self) {
            self.ticks += 1;
        }
    }
    struct MockShell {
        v: MockVideo,
        a: MockAudio,
        i: MockInput,
        c: MockClock,
    }
    impl PlatformShell for MockShell {
        type Video = MockVideo;
        type Audio = MockAudio;
        type Input = MockInput;
        type Clock = MockClock;
        fn video(&mut self) -> &mut Self::Video {
            &mut self.v
        }
        fn audio(&mut self) -> &mut Self::Audio {
            &mut self.a
        }
        fn input(&mut self) -> &mut Self::Input {
            &mut self.i
        }
        fn clock(&mut self) -> &mut Self::Clock {
            &mut self.c
        }
    }
    struct MockDriver {
        steps: Vec<u8>,
        frame: Vec<u8>,
    }
    impl GameDriver for MockDriver {
        fn step(&mut self, joy1: u8, _joy2: u8) {
            self.steps.push(joy1);
        }
        fn video_frame(&self) -> &[u8] {
            &self.frame
        }
        fn drain_apu(&mut self, out: &mut Vec<(u16, u8)>) {
            out.push((0x4000, self.steps.len() as u8));
        }
    }

    /// 环序契约：事件泵→输入→驱动→APU→音频→视频→帧钟。
    #[test]
    fn run_frame_orders_host_calls() {
        let mut d = MockDriver { steps: vec![], frame: vec![0u8; FRAME_RGB_LEN] };
        let mut s = MockShell {
            v: MockVideo { presented: vec![], alive: true },
            a: MockAudio::default(),
            i: MockInput,
            c: MockClock::default(),
        };
        let mut scratch = Vec::new();
        assert!(run_frame(&mut d, &mut s, &mut scratch));
        assert_eq!(d.steps, vec![PadState::START], "输入进驱动");
        assert_eq!(s.a.regs, vec![(0x4000, 1)], "APU 流进音频面");
        assert_eq!(s.a.boundaries, 1, "帧界一次");
        assert_eq!(s.v.presented, vec![FRAME_RGB_LEN], "视频呈现一次");
        assert_eq!(s.c.ticks, 1, "帧钟一次");
    }

    /// 退出路径：事件泵 false⇒环止；帧数上限亦止。
    #[test]
    fn run_game_stops_on_exit_and_cap() {
        let mut d = MockDriver { steps: vec![], frame: vec![0u8; FRAME_RGB_LEN] };
        let mut s = MockShell {
            v: MockVideo { presented: vec![], alive: true },
            a: MockAudio::default(),
            i: MockInput,
            c: MockClock::default(),
        };
        assert_eq!(run_game(&mut d, &mut s, None), 1);
        let mut d2 = MockDriver { steps: vec![], frame: vec![0u8; FRAME_RGB_LEN] };
        let mut s2 = MockShell {
            v: MockVideo { presented: vec![], alive: false },
            a: MockAudio::default(),
            i: MockInput,
            c: MockClock::default(),
        };
        assert_eq!(run_game(&mut d2, &mut s2, Some(5)), 0, "事件泵即退");
    }
}
