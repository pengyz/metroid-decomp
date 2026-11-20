//! NES 2A03 APU 合成核心（纯逻辑、零平台依赖、零第三方库）。
//!
//! 存在理由：引擎侧（`game-core::sound`）已把正确的寄存器值写进 `$4000-$4017`
//! （音乐四磁道 + SFX 全族），但壳侧 `AudioSink` 此前只是**计数**——整条链最后一环
//! （寄存器 → 波形）缺失，游戏绝对静音。本模块补这一环。
//!
//! 覆盖范围（增量落地，当前=第 1 增量）：
//! - frame counter（240 Hz 四步/五步序列、`$4017` mode 位）；
//! - length counter（`$4015` 使能 + 32 项查找表）；
//! - 硬件包络（`$4000` bit4 选择恒定音量 / 包络衰减）；
//! - sweep（`$4001`，含「目标周期 > $7FF 或当前周期 < 8 ⇒ 静音」的硬门）；
//! - **pulse ×2**（duty 四档、周期定时器、长度/包络/sweep 联合静音）；
//! - mixer（pulse 支路非线性公式）+ 44.1 kHz 重采样。
//!
//! 尚未做（后续增量）：triangle（含 `$400B` bit7 与长度计数器的耦合坑）、
//! noise（15 位 LFSR / 短模式）、DMC（本游戏不用）。
//!
//! 证据锚：`refs/m1disasm/SRC/sound_engine.asm` 只负责**写寄存器**，波形语义以
//! 2A03 硬件规格为准（本项目唯一非 ROM 证据源，属硬件文档级 A 证据）。

/// NTSC CPU 主频（2A03 = 1.789773 MHz）。
pub const CPU_HZ: u32 = 1_789_773;
/// 输出采样率。
pub const SAMPLE_RATE: u32 = 44_100;

/// pulse 声道长度计数器查找表（`$4003` 高 5 位索引；2A03 硬件表，A 级）。
pub const LENGTH_TABLE: [u8; 32] = [
    10, 254, 20, 2, 40, 4, 80, 6, 160, 8, 60, 10, 14, 12, 26, 14, 12, 16, 24, 18, 48, 20, 96,
    22, 192, 24, 72, 26, 16, 28, 32, 30,
];

/// duty 序列（四档 × 8 步）。
const DUTY_TABLE: [[u8; 8]; 4] = [
    [0, 1, 0, 0, 0, 0, 0, 0],
    [0, 1, 1, 0, 0, 0, 0, 0],
    [0, 1, 1, 1, 1, 0, 0, 0],
    [1, 0, 0, 1, 1, 1, 1, 1],
];

/// 硬件包络 / 常量音量。
#[derive(Clone, Copy, Debug, Default)]
pub struct Envelope {
    start: bool,
    loop_flag: bool,
    constant_volume: bool,
    volume: u8,
    divider: u8,
    decay: u8,
}

impl Envelope {
    /// `$4000` / `$4004` 写注册。
    pub fn write(&mut self, value: u8) {
        self.loop_flag = value & 0x20 != 0;
        self.constant_volume = value & 0x10 != 0;
        self.volume = value & 0x0F;
    }

    /// 每次 quarter-frame 递减；返回当前输出音量（0-15）。
    pub fn output(&self) -> u8 {
        if self.constant_volume {
            self.volume
        } else {
            self.decay
        }
    }

    /// quarter-frame 时钟。
    pub fn clock(&mut self) {
        if self.start {
            self.start = false;
            self.decay = 15;
            self.divider = self.volume;
        } else if self.divider == 0 {
            self.divider = self.volume;
            if self.decay > 0 {
                self.decay -= 1;
            } else if self.loop_flag {
                self.decay = 15;
            }
        } else {
            self.divider -= 1;
        }
    }

    /// length counter 归零或 `$4003` 写：重置起振。
    pub fn restart(&mut self) {
        self.start = true;
    }
}

/// sweep 单元（`$4001` / `$4005`）。
#[derive(Clone, Copy, Debug, Default)]
pub struct Sweep {
    enabled: bool,
    period: u8,
    negate: bool,
    shift: u8,
    reload: bool,
    divider: u8,
    /// pulse1 的负向取补码（一字反），pulse2 取二补（2A03 的不对称 quirk）。
    ones_complement: bool,
}

impl Sweep {
    pub fn new(ones_complement: bool) -> Self {
        Self { ones_complement, ..Self::default() }
    }

    pub fn write(&mut self, value: u8) {
        self.enabled = value & 0x80 != 0;
        self.period = (value >> 4) & 0x07;
        self.negate = value & 0x08 != 0;
        self.shift = value & 0x07;
        self.reload = true;
    }

    /// 目标周期**未掩码**值（溢出判定用；硬件在 > `$7FF` 时置 overflow 并静音）。
    pub fn target_raw(&self, period: u16) -> u16 {
        let delta = period >> self.shift;
        if self.negate {
            if self.ones_complement {
                period.wrapping_sub(delta).wrapping_sub(1)
            } else {
                period.wrapping_sub(delta)
            }
        } else {
            period.wrapping_add(delta)
        }
    }

    /// 目标周期（掩码到 11 位——写回 `$4002/$4003` 用）。
    pub fn target_period(&self, period: u16) -> u16 {
        self.target_raw(period) & 0x7FF
    }

    /// half-frame（120 Hz）时钟：周期到 0 时把 `$4002/$4003` 换成目标周期。
    pub fn clock(&mut self, period: &mut u16) {
        if self.divider == 0 && self.enabled && self.shift > 0 && !self.muting(*period) {
            *period = self.target_period(*period);
        }
        if self.divider == 0 || self.reload {
            self.divider = self.period;
            self.reload = false;
        } else {
            self.divider -= 1;
        }
    }

    /// 静音硬门：目标周期 > `$7FF` 或当前周期 < 8（硬件规格）。
    pub fn muting(&self, period: u16) -> bool {
        period < 8 || self.target_raw(period) > 0x7FF
    }
}

/// 单个 pulse 声道。
#[derive(Clone, Copy, Debug, Default)]
pub struct Pulse {
    pub envelope: Envelope,
    pub sweep: Sweep,
    duty: u8,
    timer_period: u16,
    timer: u16,
    sequencer: u8,
    length: u8,
    enabled: bool,
}

impl Pulse {
    pub fn new(ones_complement: bool) -> Self {
        Self { sweep: Sweep::new(ones_complement), ..Self::default() }
    }

    /// `$4000-$4003`（`base` = 0x4000 或 0x4004）。
    pub fn write_reg(&mut self, base: u16, addr: u16, value: u8) {
        match addr - base {
            0 => {
                self.duty = (value >> 6) & 0x03;
                self.envelope.write(value);
            }
            1 => self.sweep.write(value),
            2 => {
                self.timer_period = (self.timer_period & 0x700) | value as u16;
            }
            3 => {
                self.timer_period = (self.timer_period & 0x0FF) | (((value as u16) & 0x07) << 8);
                if self.enabled {
                    self.length = LENGTH_TABLE[(value >> 3) as usize];
                }
                self.sequencer = 0;
                self.envelope.restart();
            }
            _ => {}
        }
    }

    /// `$4015` 使能位。
    pub fn set_enabled(&mut self, on: bool) {
        self.enabled = on;
        if !on {
            self.length = 0;
        }
    }

    /// half-frame（120 Hz）：长度计数 + sweep。
    pub fn clock_half(&mut self) {
        if self.enabled && self.length > 0 && !self.envelope.loop_flag {
            self.length -= 1;
        }
        let mut p = self.timer_period;
        self.sweep.clock(&mut p);
        self.timer_period = p;
    }

    /// CPU 周期步进（pulse 定时器每 2 个 CPU 周期走一步）。
    pub fn clock_cpu(&mut self, even_cycle: bool) {
        if !even_cycle {
            return;
        }
        if self.timer == 0 {
            self.timer = self.timer_period;
            self.sequencer = (self.sequencer + 1) & 0x07;
        } else {
            self.timer -= 1;
        }
    }

    /// 当前输出（0-15）。静音条件：长度 0 / sweep 静音 / 当前 duty 步为低。
    pub fn output(&self) -> u8 {
        if self.length == 0 || self.timer_period < 8 || self.sweep.muting(self.timer_period) {
            return 0;
        }
        if DUTY_TABLE[self.duty as usize][self.sequencer as usize] == 0 {
            0
        } else {
            self.envelope.output()
        }
    }
}


/// 三角波 32 步序列（15→0→15，硬件表）。
const TRI_SEQUENCE: [u8; 32] = [
    15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1, 0, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11,
    12, 13, 14, 15,
];

/// NTSC 噪声周期表（`$400E` 低 4 位索引）。
pub const NOISE_PERIODS: [u16; 16] = [
    4, 8, 16, 32, 64, 96, 128, 160, 202, 254, 380, 508, 762, 1016, 2034, 4068,
];

/// 三角波声道。
///
/// 硬件坑（本模块显式建模）：`$400B` bit7 是**控制/长度 halt**，它同时决定
/// linear counter 的 reload 标志是否在半帧被清除——两者耦合，不是独立两个开关。
#[derive(Clone, Copy, Debug, Default)]
pub struct Triangle {
    control: u8,
    timer_period: u16,
    timer: u16,
    sequencer: u8,
    length: u8,
    linear: u8,
    reload: bool,
    enabled: bool,
}

impl Triangle {
    /// `$4008-$400B`。
    pub fn write_reg(&mut self, addr: u16, value: u8) {
        match addr {
            0x4008 => {
                self.control = value;
                self.reload = true;
            }
            0x400A => self.timer_period = (self.timer_period & 0x700) | value as u16,
            0x400B => {
                self.timer_period = (self.timer_period & 0x0FF) | (((value as u16) & 0x07) << 8);
                if self.enabled {
                    self.length = LENGTH_TABLE[(value >> 3) as usize];
                }
                self.reload = true;
            }
            _ => {}
        }
    }

    pub fn set_enabled(&mut self, on: bool) {
        self.enabled = on;
        if !on {
            self.length = 0;
        }
    }

    /// quarter-frame：linear counter 装载/递减。
    pub fn clock_quarter(&mut self) {
        if self.reload {
            self.linear = self.control & 0x7F;
        } else if self.linear > 0 {
            self.linear -= 1;
        }
    }

    /// half-frame：长度计数 + （控制位 bit7 为 0 时）清 reload 标志。
    pub fn clock_half(&mut self) {
        if self.enabled && self.length > 0 && (self.control & 0x80) == 0 {
            self.length -= 1;
        }
        if self.control & 0x80 == 0 {
            self.reload = false;
        }
    }

    /// 三角波定时器每 CPU 周期走一步（不是每 2 周期——与 pulse 不同）。
    pub fn clock_cpu(&mut self) {
        if self.timer == 0 {
            self.timer = self.timer_period;
            self.sequencer = (self.sequencer + 1) & 0x1F;
        } else {
            self.timer -= 1;
        }
    }

    pub fn output(&self) -> u8 {
        if self.length == 0 || self.linear == 0 {
            return 0;
        }
        TRI_SEQUENCE[self.sequencer as usize]
    }
}

/// 噪声声道（15 位 LFSR）。
#[derive(Clone, Copy, Debug)]
pub struct Noise {
    pub envelope: Envelope,
    timer_period: u16,
    timer: u16,
    shift: u16,
    mode_short: bool,
    length: u8,
    enabled: bool,
}

impl Default for Noise {
    fn default() -> Self {
        Self {
            envelope: Envelope::default(),
            timer_period: NOISE_PERIODS[0],
            timer: NOISE_PERIODS[0],
            shift: 1,
            mode_short: false,
            length: 0,
            enabled: false,
        }
    }
}

impl Noise {
    /// `$400C-$400F`。
    pub fn write_reg(&mut self, addr: u16, value: u8) {
        match addr {
            0x400C => self.envelope.write(value),
            0x400E => {
                self.mode_short = value & 0x80 != 0;
                self.timer_period = NOISE_PERIODS[(value & 0x0F) as usize];
            }
            0x400F => {
                if self.enabled {
                    self.length = LENGTH_TABLE[(value >> 3) as usize];
                }
                self.envelope.restart();
            }
            _ => {}
        }
    }

    pub fn set_enabled(&mut self, on: bool) {
        self.enabled = on;
        if !on {
            self.length = 0;
        }
    }

    pub fn clock_half(&mut self) {
        if self.enabled && self.length > 0 && !self.envelope.loop_flag {
            self.length -= 1;
        }
    }

    /// 定时器每 CPU 周期一步；到点时钟 LFSR。
    pub fn clock_cpu(&mut self) {
        if self.timer == 0 {
            self.timer = self.timer_period;
            let tap = if self.mode_short { 6 } else { 1 };
            let feedback = (self.shift & 1) ^ ((self.shift >> tap) & 1);
            self.shift = (self.shift >> 1) | (feedback << 14);
        } else {
            self.timer -= 1;
        }
    }

    /// 当前 LFSR 位（bit0 为输出）。
    pub fn lfsr(&self) -> u16 {
        self.shift
    }

    pub fn output(&self) -> u8 {
        if self.length == 0 || self.shift & 1 != 0 {
            return 0;
        }
        self.envelope.output()
    }
}

/// APU 顶层（pulse×2 + triangle + noise；DMC 本游戏不用）。
#[derive(Clone, Debug)]
pub struct Apu {
    pub pulse1: Pulse,
    pub pulse2: Pulse,
    pub triangle: Triangle,
    pub noise: Noise,
    /// `$4015` 原始值（读回/使能位）。
    pub status: u8,
    /// `$4017` bit7 置 = 五步序列（不做 IRQ）。
    five_step: bool,
    /// CPU 周期计数（frame counter 用）。
    cycles: u64,
    /// 重采样累加器（CPU 周期 / 采样）。
    sample_acc: u32,
    samples: Vec<i16>,
}

impl Default for Apu {
    fn default() -> Self {
        Self::new()
    }
}

impl Apu {
    pub fn new() -> Self {
        Self {
            pulse1: Pulse::new(true),
            pulse2: Pulse::new(false),
            triangle: Triangle::default(),
            noise: Noise::default(),
            status: 0,
            five_step: false,
            cycles: 0,
            sample_acc: 0,
            samples: Vec::new(),
        }
    }

    /// 寄存器写（`$4000-$4017`；未覆盖范围忽略）。
    pub fn write_reg(&mut self, addr: u16, value: u8) {
        match addr {
            0x4000..=0x4003 => self.pulse1.write_reg(0x4000, addr, value),
            0x4004..=0x4007 => self.pulse2.write_reg(0x4004, addr, value),
            0x4008..=0x400B => self.triangle.write_reg(addr, value),
            0x400C..=0x400F => self.noise.write_reg(addr, value),
            0x4015 => {
                self.status = value;
                self.pulse1.set_enabled(value & 0x01 != 0);
                self.pulse2.set_enabled(value & 0x02 != 0);
                self.triangle.set_enabled(value & 0x04 != 0);
                self.noise.set_enabled(value & 0x08 != 0);
            }
            0x4017 => self.five_step = value & 0x80 != 0,
            _ => {}
        }
    }

    /// 走一个 CPU 周期并可能产出一个采样点。
    pub fn clock_cpu(&mut self) {
        let even = self.cycles % 2 == 0;
        self.pulse1.clock_cpu(even);
        self.pulse2.clock_cpu(even);
        self.triangle.clock_cpu();
        self.noise.clock_cpu();

        // frame counter：240 Hz 四步序列（quarter=7457，half=14913 周期处）。
        let step = self.cycles % 7457;
        if step == 0 {
            let q = (self.cycles / 7457) % 4;
            self.pulse1.envelope.clock();
            self.pulse2.envelope.clock();
            self.noise.envelope.clock();
            self.triangle.clock_quarter();
            if q % 2 == 1 {
                self.pulse1.clock_half();
                self.pulse2.clock_half();
                self.triangle.clock_half();
                self.noise.clock_half();
            }
        }

        self.cycles += 1;

        // 重采样：CPU_HZ / SAMPLE_RATE ≈ 40.585 周期/样本。
        self.sample_acc += SAMPLE_RATE;
        if self.sample_acc >= CPU_HZ {
            self.sample_acc -= CPU_HZ;
            let s = self.mix();
            self.samples.push(s);
        }
    }

    /// 取走已合成采样（壳侧调用）。
    pub fn take_samples(&mut self) -> Vec<i16> {
        core::mem::take(&mut self.samples)
    }

    /// pulse 支路混音（2A03 非线性公式；triangle/noise 尚未接入）。
    fn mix(&self) -> i16 {
        let p = self.pulse1.output() as f32 + self.pulse2.output() as f32;
        let pulse_out = if p == 0.0 { 0.0 } else { 95.88 / (8128.0 / p + 100.0) };
        let t = self.triangle.output() as f32;
        let n = self.noise.output() as f32;
        let tnd = if t == 0.0 && n == 0.0 {
            0.0
        } else {
            159.79 / (1.0 / (t / 8227.0 + n / 12241.0) + 100.0)
        };
        let v = pulse_out + tnd;
        (v.clamp(0.0, 1.0) * i16::MAX as f32) as i16
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn enable(a: &mut Apu) {
        a.write_reg(0x4015, 0x03);
    }

    /// `$4003` 高 5 位装载长度计数（硬件表）。
    #[test]
    fn length_counter_loads_from_table() {
        let mut a = Apu::new();
        enable(&mut a);
        a.write_reg(0x4003, 0x08); // 索引 1 ⇒ 254
        assert_eq!(a.pulse1.length, 254);
        a.write_reg(0x4003, 0x00); // 索引 0 ⇒ 10
        assert_eq!(a.pulse1.length, 10);
    }

    /// `$4015` 关断立即清长度计数（⇒ 静音）。
    #[test]
    fn disabling_channel_clears_length() {
        let mut a = Apu::new();
        enable(&mut a);
        a.write_reg(0x4003, 0x08);
        a.write_reg(0x4015, 0x00);
        assert_eq!(a.pulse1.length, 0);
        assert_eq!(a.pulse1.output(), 0);
    }

    /// 恒定音量下 duty=50% 的采样里高低占比接近 1:1。
    #[test]
    fn duty_50_has_balanced_high_low() {
        let mut a = Apu::new();
        enable(&mut a);
        a.write_reg(0x4000, 0xBF); // duty=2(50%) + 常量音量 15
        a.write_reg(0x4002, 0x40); // 周期低位
        a.write_reg(0x4003, 0x00); // 长度 10、周期高位 0
        for _ in 0..CPU_HZ / 20 {
            a.clock_cpu();
        }
        let s = a.take_samples();
        assert!(s.len() > 100, "应有采样输出，实得 {}", s.len());
        let high = s.iter().filter(|&&v| v > 0).count();
        let ratio = high as f32 / s.len() as f32;
        assert!((0.30..=0.70).contains(&ratio), "duty50 高电平占比应≈50%，实得 {ratio}");
    }

    /// 全静音时输出恒 0（防空对空：先证明"有声"用例真的不是恒 0）。
    #[test]
    fn silence_yields_zero_samples() {
        let mut a = Apu::new();
        for _ in 0..CPU_HZ / 100 {
            a.clock_cpu();
        }
        let s = a.take_samples();
        assert!(!s.is_empty());
        assert!(s.iter().all(|&v| v == 0));
    }

    /// 包络起振后按 quarter-frame 阶梯衰减：15→14→…→0，loop 置位则回 15。
    #[test]
    fn envelope_decays_then_loops() {
        let mut a = Apu::new();
        enable(&mut a);
        a.write_reg(0x4000, 0xA0); // duty=2、非恒定音量、loop=1、period=0（每 quarter 都递减）
        a.write_reg(0x4002, 0x40);
        a.write_reg(0x4003, 0x00);
        // 写 `$4003` 只置 start 位；15 在下一个 quarter-frame 才装载。
        assert_eq!(a.pulse1.envelope.output(), 0);
        a.clock_cpu();
        assert_eq!(a.pulse1.envelope.output(), 15);

        let mut seq = Vec::new();
        for _ in 0..17 {
            for _ in 0..7457 {
                a.clock_cpu();
            }
            seq.push(a.pulse1.envelope.output());
        }
        assert_eq!(&seq[..4], &[14, 13, 12, 11], "逐 quarter 递减，实得 {seq:?}");
        assert_eq!(seq[14], 0, "第 15 个 quarter 到 0，实得 {seq:?}");
        assert_eq!(seq[15], 15, "loop 置位 ⇒ 到底后回到 15，实得 {seq:?}");
    }


    /// 三角波 linear counter：halt 位（`$4008` bit7）置位时计数器不递减。
    #[test]
    fn triangle_linear_counter_halt_holds_value() {
        let mut t = Triangle::default();
        t.set_enabled(true);
        t.write_reg(0x4008, 0x8A); // halt=1、reload=10
        t.write_reg(0x400A, 0x40);
        t.write_reg(0x400B, 0x00);
        t.clock_quarter();
        assert_eq!(t.linear, 10);
        for _ in 0..5 {
            t.clock_quarter();
            t.clock_half();
        }
        assert_eq!(t.linear, 10, "halt 置位 ⇒ 装载后不再递减");

        let mut t2 = Triangle::default();
        t2.set_enabled(true);
        t2.write_reg(0x4008, 0x0A); // halt=0
        t2.write_reg(0x400B, 0x00);
        t2.clock_quarter();
        assert_eq!(t2.linear, 10);
        for _ in 0..3 {
            t2.clock_quarter();
            t2.clock_half();
        }
        assert!(t2.linear < 10, "halt 清 ⇒ 逐 quarter 递减，实得 {}", t2.linear);
    }

    /// 三角波出声：linear/length 都非零时输出非零，且序列在 15→0→15 之间摆动。
    #[test]
    fn triangle_produces_waveform() {
        let mut a = Apu::new();
        a.write_reg(0x4015, 0x07);
        a.write_reg(0x4008, 0xFF); // halt + reload 满
        a.write_reg(0x400A, 0x40); // 周期 $40 ⇒ 序列步进快
        a.write_reg(0x400B, 0x08); // 长度 254
        let mut seen_low = false;
        let mut seen_high = false;
        for _ in 0..2000 {
            a.clock_cpu();
            let v = a.triangle.output();
            if v == 0 {
                seen_low = true;
            }
            if v >= 14 {
                seen_high = true;
            }
        }
        assert!(seen_high, "应出现 14/15 档");
        assert!(seen_low, "应出现 0 档（32 步序列走完一轮）");
    }

    /// 噪声 LFSR：短模式（tap=6）周期远短于长模式（tap=1）。
    #[test]
    fn noise_lfsr_short_mode_cycles_faster() {
        fn period_steps(short: bool) -> usize {
            let mut n = Noise::default();
            n.set_enabled(true);
            n.write_reg(0x400C, 0x3F); // 无 loop、恒定音量
            n.write_reg(0x400E, if short { 0x80 } else { 0x00 }); // 最快周期、mode
            n.write_reg(0x400F, 0x08); // 长度 254
            let start = n.lfsr();
            // 返回「LFSR 时钟次数」，换算成 CPU 周期（timer 周期 4 ⇒ 5 周期/步）。
            let mut steps = 0usize;
            for _ in 1..200_000 {
                n.clock_cpu();
                if n.timer == 4 {
                    // timer 刚被重装 ⇒ 说明发生了一次 LFSR 时钟
                    steps += 1;
                    if n.lfsr() == start {
                        return steps;
                    }
                }
            }
            usize::MAX
        }
        let s = period_steps(true);
        let l = period_steps(false);
        assert!(s < 200, "短模式周期应 ≈93 步，实得 {s}");
        assert!(l > 20_000, "长模式周期应 ≈32767 步（远大于短模式），实得 {l}");
    }

    /// 三角波单独也能出声（混音公式 tnd 支路接上了）。
    #[test]
    fn triangle_alone_mixes_to_nonzero() {
        let mut a = Apu::new();
        a.write_reg(0x4015, 0x07);
        a.write_reg(0x4008, 0xFF);
        a.write_reg(0x400A, 0x40);
        a.write_reg(0x400B, 0x08);
        for _ in 0..CPU_HZ / 20 {
            a.clock_cpu();
        }
        let s = a.take_samples();
        assert!(s.iter().any(|&v| v > 0), "三角波应产生非零采样");
    }

    /// sweep 静音硬门：目标周期越界 ⇒ 声道静音。
    #[test]
    fn sweep_muting_gate() {
        let mut a = Apu::new();
        enable(&mut a);
        a.write_reg(0x4001, 0x87); // sweep 使能、shift=7 ⇒ target 变大
        a.write_reg(0x4002, 0xFF);
        a.write_reg(0x4003, 0x07); // 周期 = $7FF ⇒ target 溢出 11 位
        assert!(a.pulse1.sweep.muting(a.pulse1.timer_period));
        assert_eq!(a.pulse1.output(), 0);
    }
}
