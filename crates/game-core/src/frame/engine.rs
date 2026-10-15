//! 帧引擎（S5 骨架）：60Hz tick 驱动 boot 执行序；每帧输出 watch 状态向量。
//!
//! 执行序端口口径（task-10④）：boot 序按**观测执行序**逐步域化——已端口步=域函数，
//! 未端口步=`unported!(bank:addr)` 确定性引爆（每引爆一个端一个）。
//! 首拍（A 级 ROM 自证 bank07:$FFB0-$FFD5 `78 D8 A2 00 … 4C 1A C0`，reset 向量 $FFB0）：
//! SEI/CLD/关 PPU/双 vblank 等待/MMC1 复位写/JMP $C01A——**零 RAM 写**（寄存器/PPU/MMC1
//! 面），帧推进见 boot_prologue；$C01A 主初始化=首个引爆点。
//! NMI 向量 $C0D9（bank7 同证）。

use super::clock::VirtualClock;
use super::memory::FrameMemory;
use super::rng::GameRng;

/// 帧引擎（内存模型 + 虚拟时钟 + PRNG 句柄）。
pub struct FrameEngine {
    /// 全内存模型。
    pub mem: FrameMemory,
    /// 虚拟时钟（60Hz tick）。
    pub clock: VirtualClock,
    /// 游戏 PRNG（算法缺口面）。
    pub rng: GameRng,
}

impl Default for FrameEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl FrameEngine {
    /// 新建（RAM/WRAM/寄存器零态）。
    pub fn new() -> Self {
        Self { mem: FrameMemory::default(), clock: VirtualClock::new(), rng: GameRng::new() }
    }

    /// 推进一帧（60Hz tick）：boot 执行序步进。
    pub fn tick_frame(&mut self) {
        self.clock.tick();
        self.boot_step();
    }

    /// boot 执行序一步（骨架：reset 序首拍 → bank07:C01A 单元 → 引爆点 bank07:C158）。
    fn boot_step(&mut self) {
        self.boot_prologue();
        // C01A 主初始化单元（已端口：MMC1 写/栈指针/双清零环/zp 初始化序前段；
        // 内嵌桩阵逐个引爆——当前引爆点 bank07:C158）。
        super::boot::boot_init(&mut self.mem);
    }

    /// reset 序首拍（bank07:$FFB0-$FFD5 逐字节自证；零 RAM 写）。
    ///
    /// SEI/CLD（P 面）；LDX #$00/STX $2000/STX $2001（PPU 关）；双 vblank 等待
    /// （LDA $2002/BPL——PPU 时序面，S5 骨架按虚拟帧语义瞬时完成，时序保真待 task-13
    /// 帧界对齐设计）；ORA #$FF/STA $8000/$A000/$C000/$E000（MMC1 复位写）；JMP $C01A。
    fn boot_prologue(&mut self) {
        self.mem.regs.p |= 0x04; // SEI（I=1）
        self.mem.regs.p &= !0x08; // CLD（D=0）
        self.mem.regs.x = 0x00; // LDX #$00
        // STX $2000/$2001（PPU 寄存器面——不入 RAM watch）；LDA $2002 vblank 等待同理。
        // ORA #$FF → A=$FF；STA $8000/$A000/$C000/$E000（MMC1 复位写，不入 RAM watch）。
        self.mem.regs.a = 0xFF;
    }

    /// watch 状态向量（每帧导出面）。
    pub fn watch_snapshot(&self) -> Vec<(&'static str, String)> {
        self.mem.watch_snapshot()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// panic payload 双型兜底（String / &str——工具链形态差异）。
    fn panic_message(err: &Box<dyn std::any::Any + Send>) -> String {
        err.downcast_ref::<String>()
            .cloned()
            .or_else(|| err.downcast_ref::<&'static str>().map(|s| (*s).to_string()))
            .unwrap_or_default()
    }

    /// 缺口确定性爆在 bank:addr（D7 机制②）——S5 骨架首引爆面自证。
    #[test]
    fn unported_detonates_at_bank_addr() {
        let mut e = FrameEngine::new();
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| e.tick_frame()));
        let err = r.unwrap_err();
        let msg = panic_message(&err);
        assert!(msg.contains("unported @ bank07:C4B2"), "引爆点失准: {msg}");
    }

    /// tick_frame 执行序：tick 先行 + C01A 单元清零环生效 + 引爆前态可检。
    #[test]
    fn tick_frame_runs_boot_unit_before_detonation() {
        let mut e = FrameEngine::new();
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| e.tick_frame()));
        assert_eq!(e.clock.ticks(), 1); // tick 先行
        assert_eq!(e.mem.read_ram(0x0355), 0x55); // 清零环已跑（Y 型填充）
        assert_eq!(e.mem.read_ram(0x0025), 0x0E); // zp 初始化序已跑
        assert_eq!(e.mem.regs.sp, 0xFF); // TXS
        assert_eq!(e.mem.read_ram(0x0000), 0x00); // C1A3 ptr（$00=0，覆写 C158 的 FF——执行序后写胜）
        assert_eq!(e.mem.read_ram(0x0001), 2); // C1A3 ptr 高位
        assert_eq!(e.mem.read_ram(0x00FF) & 0x04, 0x00); // C158 $FF 清 bit2
        assert_eq!(e.mem.read_ram(0x0200), 0xF0); // C1A3 OAM 影子 F0 填充（帧 1 实测面）
    }

    /// GameRng 算法缺口面 fail-loud（未证不发明）。
    #[test]
    fn game_rng_detonates_until_verified() {
        let mut rng = GameRng::new();
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| rng.next_u8()));
        let msg = panic_message(&r.unwrap_err());
        assert!(msg.contains("rng-algorithm-unverified"), "引爆点失准: {msg}");
    }
}
