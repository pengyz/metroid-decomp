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
    /// → 帧工作 → 清 0 → spin yield。采样=无操作（快照用 tick_frame_with）。
    pub fn tick_frame(&mut self) {
        self.tick_frame_with(|_| {});
    }

    /// 推进一帧+**spin 出口采样缝**（task-24 对齐：oracle $C0D3 同点——NMI 尾写 1 后/
    /// 帧工作前；$1A=1 恒量面+FrameCount 未增=缝位语义锚）。
    pub fn tick_frame_with<F: FnMut(&FrameMemory)>(&mut self, mut on_spin_exit: F) {
        self.clock.tick();
        if self.mem.nmi_enabled {
            super::nmi::nmi_handler(&mut self.mem);
        }
        // === spin 出口采样缝（$C0D3 对齐）——**仅主循环 spin 出口**（boot 链无主循环
        // spin——oracle 295 行=主循环 spin 出口数自证；boot tick 不采样）。
        // 缝位=**首个主循环界唤醒**（PostSpin+MainLoop——oracle f=1 语义（工作前）；
        // PreSpin/AwaitNmi=boot 内部等待不采）。
        if self.phase == BootPhase::PostSpin || self.phase == BootPhase::MainLoop {
            on_spin_exit(&self.mem);
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
                    // 【唤醒 tick 合并首迭代】（r14 $C3 根因件）：真机 f1=NMI#1+boot_post+
                    // 首工作**同一帧**（NMI-1 即 boot 唤醒 NMI）——分帧会多出一个 NMI 周期
                    // （星调色板 switch 在 boot NMI 多自增一次且首个迭代的队列门错相）。
                    // 采样缝=boot_post 后/首工作前（oracle f=1 语义锚）。
                    on_spin_exit(&self.mem);
                    super::mainloop::main_loop_iter(&mut self.mem);
                    self.phase = BootPhase::MainLoop;
                }
            }
            BootPhase::PostSpin | BootPhase::MainLoop => {
                super::mainloop::main_loop_iter(&mut self.mem); // 主循环一迭代（C0BC 体）
                self.phase = BootPhase::MainLoop;
            }
        }
    }

    /// watch 状态向量（每帧导出面）。
    pub fn watch_snapshot(&self) -> Vec<(&'static str, String)> {
        self.mem.watch_snapshot()
    }

    /// 占位臂缺口记账面（task-29 (ii)）：自进程启动以来占位臂记账的缺口址序列
    /// （unported 例行入口址——DIVERGENCES 面，title_bridge 记账源）。
    pub fn divergences() -> Vec<u16> {
        super::DIVERGENCES.with(|d| d.borrow().clone())
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

    /// spin 出口缝位语义（task-24 对齐锚）：**仅主循环 spin 出口采样**（boot tick 不采
    /// ——oracle 295 行=主循环 spin 出口数自证）；采样点恒 1 断言（契约断言①面）。
    #[test]
    fn spin_exit_seam_mainloop_only_constant_one() {
        let mut e = FrameEngine::new();
        let mut samples: Vec<u8> = vec![];
        for _ in 0..4 { // tick4=首主循环采样（避 01 段 SFX 写触发声缺口 B31B——另案）
            e.tick_frame_with(|m| samples.push(m.read_ram(crate::ram_map::NMI_STATUS)));
        }
        assert!(!samples.is_empty(), "主循环 spin 出口应采样");
        assert!(samples.iter().all(|a| *a == 1), "恒 1 断言失守: {samples:?}");
    }

    /// 占位臂记账**终形（账清断言）**（批6b 清偿毕：0x18 专件已臂化——01/12/15/1B/
    /// 1C/17/18 全清，title 面账清=strict 绿前置达成；直测桥面）。
    #[test]
    fn placeholder_arm_records_divergence() {
        let mut m = crate::frame::FrameMemory::default();
        m.write_ram(crate::ram_map::TITLE_ROUTINE, 0x18); // EnterPassword（已臂化）
        crate::frame::title_bridge::main_title_routine(&mut m);
        let recorded = crate::frame::DIVERGENCES.with(|d| d.borrow().clone());
        assert!(!recorded.contains(&0x9147), "0x18 专件未清: {recorded:?}");
    }

    /// 【诊断】boot 早窗逐 tick 状态（r14 $C3 根因定位）。
    #[test]
    fn diag_boot_window_state() {
        let mut e = FrameEngine::new();
        for t in 1..=8 {
            e.tick_frame();
            println!(
                "t{t}: $C3={:02X} GameMode={:02X} FC={:02X} phase={:?}",
                e.mem.read_ram(0x00C3), e.mem.read_ram(crate::ram_map::GAME_MODE),
                e.mem.read_ram(crate::ram_map::FRAME_COUNT), e.phase
            );
        }
    }

    /// tick_frame 执行序（契约断言②：**f=1 帧界锚**——首 tick 后 clock=1）+ pre 链态。
    #[test]
    fn tick_frame_runs_boot_unit_before_detonation() {
        let mut e = FrameEngine::new();
        e.tick_frame();
        assert_eq!(e.clock.ticks(), 1); // 契约②：f=1 帧界锚
        assert_eq!(e.phase, BootPhase::AwaitNmi); // spin yield 态
        assert_eq!(e.mem.read_ram(0x0355), 0x00); // 清零环已跑（**零填充**——更正案）
        assert_eq!(e.mem.read_ram(0x0200), 0xF0); // C1A3 OAM 影子 F0 填充（帧 1 实测面）
        assert_eq!(e.mem.read_ram(0x0001), 2); // C1A3 ptr 高位
        assert!(e.mem.nmi_enabled); // C08A $2000=$90
        assert_eq!(e.mem.read_ram(crate::ram_map::NMI_STATUS), 0); // spin 入口（ClearNMIStat）
    }

}

