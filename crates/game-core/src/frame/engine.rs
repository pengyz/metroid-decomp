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

/// boot 相位（spin=跨帧 yield 构造物——契约 §8.1①；tick 序=NMI(尾写 1)→唤醒→帧工作→清 0→spin）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BootPhase {
    /// 链前段（reset 序…C531 head+ScreenNmiOff pre——首 tick 一次跑）。
    PreSpin,
    /// spin 等待（$1A=0 已写；等 NMI 尾写 1 唤醒）。
    AwaitNmi,
    /// spin 出口后段（ScreenNmiOff post/CopyMap/…——被唤醒 tick 续）。
    PostSpin,
    /// 主循环相（C0BC 体——成环后每 tick 一迭代）。
    MainLoop,
}

/// 帧引擎（内存模型 + 虚拟时钟 + PRNG 句柄 + boot 相位机）。
pub struct FrameEngine {
    /// 全内存模型。
    pub mem: FrameMemory,
    /// 虚拟时钟（60Hz tick）。
    pub clock: VirtualClock,
    /// 游戏 PRNG（算法缺口面）。
    pub rng: GameRng,
    /// boot 相位（spin 跨帧状态机）。
    pub phase: BootPhase,
}

impl Default for FrameEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl FrameEngine {
    /// 新建（RAM/WRAM/寄存器零态）。
    pub fn new() -> Self {
        Self {
            mem: FrameMemory::default(),
            clock: VirtualClock::new(),
            rng: GameRng::new(),
            phase: BootPhase::PreSpin,
        }
    }

    /// 推进一帧（60Hz tick）——**tick 序**（契约 §8.1①）：NMI（尾写 $1A=1）→ spin 唤醒
    /// → 帧工作 → 清 0 → spin yield。
    pub fn tick_frame(&mut self) {
        self.clock.tick();
        if self.mem.nmi_enabled {
            super::nmi::nmi_handler(&mut self.mem);
        }
        match self.phase {
            BootPhase::PreSpin => {
                super::boot::boot_pre_spin(&mut self.mem);
                self.phase = BootPhase::AwaitNmi; // spin yield（$1A=0 已写）
            }
            BootPhase::AwaitNmi => {
                if self.mem.read_ram(crate::ram_map::NMI_STATUS) != 0 {
                    super::boot::boot_post_spin(&mut self.mem);
                    self.phase = BootPhase::PostSpin;
                }
            }
            BootPhase::PostSpin | BootPhase::MainLoop => {
                crate::unported!("bank07:C0BC"); // 主循环相（成环件——未端口）
            }
        }
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

    /// 缺口确定性爆在 bank:addr（D7 机制②）——tick1=PreSpin 静默 yield；tick2=NMI 先行
    /// 引爆（bank07:C1E0 CheckPaletteWrite——NMI 工作体次件）；9A07 已消化（短路面）。
    #[test]
    fn unported_detonates_at_bank_addr() {
        let mut e = FrameEngine::new();
        e.tick_frame(); // PreSpin → AwaitNmi（无炸）
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| e.tick_frame()));
        let err = r.unwrap_err();
        let msg = panic_message(&err);
        assert!(msg.contains("unported @ bank07:C1E0"), "引爆点失准: {msg}");
    }

    /// tick_frame 执行序（契约断言②：**f=1 帧界锚**——首 tick 后 clock=1）+ pre 链态。
    #[test]
    fn tick_frame_runs_boot_unit_before_detonation() {
        let mut e = FrameEngine::new();
        e.tick_frame();
        assert_eq!(e.clock.ticks(), 1); // 契约②：f=1 帧界锚
        assert_eq!(e.phase, BootPhase::AwaitNmi); // spin yield 态
        assert_eq!(e.mem.read_ram(0x0355), 0x55); // 清零环已跑（Y 型填充）
        assert_eq!(e.mem.read_ram(0x0200), 0xF0); // C1A3 OAM 影子 F0 填充（帧 1 实测面）
        assert_eq!(e.mem.read_ram(0x0001), 2); // C1A3 ptr 高位
        assert!(e.mem.nmi_enabled); // C08A $2000=$90
        assert_eq!(e.mem.read_ram(crate::ram_map::NMI_STATUS), 0); // spin 入口（ClearNMIStat）
    }

    /// GameRng 算法缺口面 fail-loud（未证不发明）。
    #[test]
    fn game_rng_detonates_until_verified() {
        let mut rng = GameRng::new();
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| rng.next_u8()));
        let msg = panic_message(&r.unwrap_err());
        assert!(msg.contains("unported @ bank07:C000"), "引爆点失准: {msg}");
    }
}
