//! 帧引擎（S5 骨架）：60Hz tick 驱动 boot 执行序；每帧输出 watch 状态向量。
//!
//! 执行序端口口径（task-10④）：boot 序按**观测执行序**逐步域化——已端口步=域函数，
//! 未端口步=`unported!(bank:addr)` 确定性引爆（每引爆一个端一个）。
//! 首拍（A 级 ROM 自证 bank07:$FFB0-$FFD5 `78 D8 A2 00 … 4C 1A C0`，reset 向量 $FFB0）：
//! SEI/CLD/关 PPU/双 vblank 等待/MMC1 复位写/JMP $C01A——**零 RAM 写**（寄存器/PPU/MMC1
//! 面），帧推进见 boot_prologue；$C01A 主初始化=首个引爆点。
//! NMI 向量 $C0D9（bank7 同证）。

use super::clock::VirtualClock;
use super::memory::{FrameMemory, Registers};
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
    /// 【Task 4 snapshot 续跑面】restore 装载点=spin 出口（post-NMI 相位 P，$1A=1）——
    /// 该帧 NMI 已发生，**首 tick 跳过 NMI** 续跑从帧工作起（emu bundle-ff f=1 行语义锚）。
    skip_next_nmi: bool,
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
            skip_next_nmi: false,
        }
    }

    /// 【Task 4 snapshot 消费面】bundle 恢复=**从快照态起跑**（非重跑 boot——
    /// boot_pre_spin 清 RAM 低页/清 WRAM 会把装载态抹掉，"加载成功"≠从快照态起跑）：
    /// · `mem`←`FrameMemory::restore_mss`（三直映灌入，spec §4 接缝表）；
    /// · `phase`←MainLoop（快照采样点=spin 出口=游戏主循环中段）；
    /// · 首 tick 跳过 NMI（装载点=post-NMI 相位 P——emu bundle-ff f=1 行==bundle 三直映
    ///   实证同锚）；首缝=装载态逐字节（对齐验收首帧锚）。
    pub fn restore_mss(&mut self, ram: &[u8], wram: &[u8], regs: Registers) {
        self.mem = FrameMemory::restore_mss(ram, wram, regs);
        self.phase = BootPhase::MainLoop;
        self.skip_next_nmi = true;
    }

    /// 推进一帧（60Hz tick）——**tick 序**（契约 §8.1①）：NMI（尾写 $1A=1）→ spin 唤醒
    /// → 帧工作 → 清 0 → spin yield。采样=无操作（快照用 tick_frame_with）。
    pub fn tick_frame(&mut self) {
        self.tick_frame_with(|_| {});
    }

    /// 推进一帧+**spin 出口采样缝**（task-24 对齐：oracle $C0D3 同点——NMI 尾写 1 后/
    /// 帧工作前；$1A=1 恒量面+FrameCount 未增=缝位语义锚）。
    ///
    /// **迭代跨帧 span**（move-drift 定谳 R2/R3——findings/move-drift-iteration-scheduling.md）：
    /// 工作可声明帧门队列（WaitNMIPass 等待帧/bogged 重活帧）；span 内 tick 不跑 MainLoop
    /// 四件/臂/采样缝（金腿窗内无行同形），tick 尾按门置 NMIStatus（等待帧=0→下一 NMI 内块
    /// 全跑；bogged=1→内块跳过（asm prg7:251 掉帧门）），队列排空 tick 尾=迭代完成点
    /// （`inc FrameCount`（asm prg7:207）延至此+唤醒门）。
    pub fn tick_frame_with<F: FnMut(&FrameMemory)>(&mut self, mut on_spin_exit: F) {
        self.clock.tick();
        if self.skip_next_nmi {
            self.skip_next_nmi = false; // restore 续跑：装载点=post-NMI——首 tick 不再 NMI
        } else if self.mem.nmi_enabled {
            super::nmi::nmi_handler(&mut self.mem);
        }
        // === spin 出口采样缝（$C0D3 对齐）——**仅主循环 spin 出口**（boot 链无主循环
        // spin——oracle 295 行=主循环 spin 出口数自证；boot tick 不采样）。
        // 缝位=**首个主循环界唤醒**（PostSpin+MainLoop——oracle f=1 语义（工作前）；
        // PreSpin/AwaitNmi=boot 内部等待不采）。span tick 不采（迭代未完成无 spin 出口）。
        let in_span = self.mem.iter_span_left > 0;
        if std::env::var("DSH_SPAN_TRACE").is_ok() {
            eprintln!("[span] tick={} in_span={} left={} gates={} fc={}", self.clock.ticks(),
                      in_span, self.mem.iter_span_left, self.mem.iter_gates.len(),
                      self.mem.read_ram(crate::ram_map::FRAME_COUNT));
        }
        if !in_span && (self.phase == BootPhase::PostSpin || self.phase == BootPhase::MainLoop) {
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
                    self.end_of_tick_gate();
                    self.phase = BootPhase::MainLoop;
                }
            }
            BootPhase::PostSpin | BootPhase::MainLoop => {
                if in_span {
                    // span tick：迭代工作中（四件/臂冻结——等待帧/bogged 帧见门队列语义）
                    self.mem.iter_span_left -= 1;
                } else {
                    super::mainloop::main_loop_iter(&mut self.mem); // 主循环一迭代（C0BC 体）
                    if !self.mem.iter_gates.is_empty() {
                        // 迭代声明跨帧：span=门队列长（门描述 T+1..T+n 的 NMI 内块跑/跳）
                        self.mem.iter_span_left = self.mem.iter_gates.len() as u8;
                    }
                }
                self.end_of_tick_gate();
                self.phase = BootPhase::MainLoop;
            }
        }
    }

    /// tick 尾 NMI 门（描述**下一 tick** NMI 内块是否跑——asm prg7:251 `lda NMIStatus/
    /// bne @endIf_A` 门的预置）：门队列非空=按门项（1 等待帧→0 全跑 / 0 bogged→1 跳过）；
    /// 排空=迭代完成点（唤醒门 NMIStatus←0（asm :210-211）+`inc FrameCount` 延迟收尾）。
    fn end_of_tick_gate(&mut self) {
        if self.mem.iter_gates.is_empty() {
            self.mem.write_ram(crate::ram_map::NMI_STATUS, 0);
            if self.mem.iter_span_fc {
                self.mem.iter_span_fc = false;
                let fc = self.mem.read_ram(crate::ram_map::FRAME_COUNT).wrapping_add(1);
                self.mem.write_ram(crate::ram_map::FRAME_COUNT, fc);
            }
        } else {
            let g = self.mem.iter_gates.remove(0); // 队列 ≤8 项（窗长界）——O(n) 无碍
            self.mem
                .write_ram(crate::ram_map::NMI_STATUS, if g != 0 { 0 } else { 1 });
        }
    }

    /// span 进行中（迭代跨帧——采样/行导出抑制面；l3-dump 行集同金腿缺行窗）。
    pub fn in_span(&self) -> bool {
        self.mem.iter_span_left > 0 || !self.mem.iter_gates.is_empty()
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

    /// 【move-drift R2/R3】span 机制：迭代声明帧门后——span tick 无采样缝（金腿窗内无行
    /// 同形）/FC 延至 span 尾（`inc FrameCount` 在迭代真正完成点）/tick 尾按门置 NMIStatus。
    #[test]
    fn span_gates_suppress_seam_and_defer_fc() {
        let mut e = FrameEngine::new();
        for _ in 0..4 {
            e.tick_frame(); // boot→MainLoop（tick4=首主循环采样）
        }
        let fc0 = e.mem.read_ram(crate::ram_map::FRAME_COUNT);
        // 手工注入声明径结果（门件生产=more_init/c4de_unit iter_wait/iter_bogged——另测）；
        // 形态=工作 tick 尾已弹首门后（不变量 span_left=gates.len()+1）。
        e.mem.iter_gates = vec![1, 0];
        e.mem.iter_span_left = 3;
        e.mem.iter_span_fc = true;
        let mut samples = 0;
        for _ in 0..3 {
            e.tick_frame_with(|_| samples += 1);
        }
        assert_eq!(samples, 0, "span tick 无采样缝（迭代跨帧无 spin 出口）");
        assert_eq!(e.mem.iter_span_left, 0);
        assert_eq!(e.mem.read_ram(crate::ram_map::FRAME_COUNT), fc0.wrapping_add(1),
                   "FC 延至 span 尾收尾（一迭代恰一次 inc）");
        assert_eq!(e.mem.read_ram(crate::ram_map::NMI_STATUS), 0, "排空 tick 尾=唤醒门");
        e.tick_frame_with(|_| samples += 1);
        assert_eq!(samples, 1, "span 后采样缝恢复（新迭代界）");
    }

    /// 【Task 4 snapshot 消费面】restore_mss=**从快照态起跑**（非重跑 boot）：
    /// ① phase 直入 MainLoop——boot_pre_spin（清 RAM 低页+清 WRAM $6000-$7FFF）不得跑，
    ///   否则装载态被抹掉（"加载成功"≠从快照态起跑）；
    /// ② 首 tick **skip NMI**——bundle 采集点=spin 出口（post-NMI 相位 P，$1A=1 实证
    ///   analysis/bundles/mid-move.json），NMI 已发生，续跑从帧工作起；
    /// ③ 首缝（tick1 采样缝）=装载态逐字节（emu bundle-ff f=1 行语义同锚：
    ///   emu f=1 行 == bundle 三直映（2026-10-26 实测））。
    #[test]
    fn restore_mss_starts_from_snapshot_state_not_boot() {
        let mut ram = vec![0u8; 0x800];
        ram[0x0300] = 0xAB; // boot 清页区（$0200-$07FF）标志值——boot 重跑会抹
        ram[0x00FF] = 0x90; // NMI 开（bit7）——skip 首 NMI 语义的测试面
        ram[0x001A] = 0x01; // spin 出口相位 P 常量 1（$1A=NMIStatus）
        let wram = vec![0x5Au8; 0x2000]; // boot 清 WRAM 区标志值
        let regs = Registers { a: 1, x: 2, y: 3, sp: 4, pc: 0xC0D3, p: 0x24 };
        let expect = FrameMemory::restore_mss(&ram, &wram, regs).watch_snapshot();

        let mut e = FrameEngine::new();
        e.restore_mss(&ram, &wram, regs);
        assert!(matches!(e.phase, BootPhase::MainLoop),
                "restore 须直入 MainLoop（快照=游戏主循环中段——boot 重跑会抹装载态）");

        // 首缝==装载态（skip 首 NMI：若 nmi 先跑，缝位=后 NMI 态≠装载态）
        let mut first: Option<Vec<(String, String)>> = None;
        let tick = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            e.tick_frame_with(|m| {
                first = Some(m.watch_snapshot().into_iter()
                    .map(|(k, v)| (k.to_string(), v)).collect());
            });
        }));
        let first = first.expect("首 tick 必出采样缝（MainLoop 相位——boot 相位不采样即缺陷）");
        let expect_named: Vec<(String, String)> = expect.into_iter()
            .map(|(k, v)| (k.to_string(), v)).collect();
        assert_eq!(first, expect_named,
                   "首缝须=装载态逐字节（三面 zp/iram/wram）——加载成功≠对齐成功的首帧锚");
        // boot 重跑判别：boot_pre_spin 后 phase=AwaitNmi；帧工作 panic 时无法看尾相位
        if tick.is_ok() {
            assert!(matches!(e.phase, BootPhase::MainLoop),
                    "tick 尾 phase 仍须 MainLoop（boot 臂会把 phase 推去 AwaitNmi）");
        }
    }
}

