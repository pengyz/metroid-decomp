//! l2 —— 子程序级执行引擎：ROM 原字节 + 入参快照 → 出参/写集。
//!
//! 内存模型（L2 裁剪面，AGENTS D7/R3）：
//!   $0000-$1FFF RAM（2KB 镜像归一 `addr & 0x7FF`；写集只记 RAM）
//!   $2000-$3FFF PPU 寄存器 stub（读 0，写忽略不入写集）
//!   $4000-$401F APU/IO stub（同上）
//!   $8000-$BFFF 可切窗口（case 的 window_bank，MMC1 bank0-6）
//!   $C000-$FFFF bank7 固定
//! 执行约定：`stops` 非空 ⇒ 跑到命中其一；为空 ⇒ 预压哨兵返回地址跑到 RTS。

use crate::cpu6502::Cpu;
use game_core::L2In;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct L2RunResult {
    pub a: u8,
    pub x: u8,
    pub y: u8,
    pub writes: Vec<(u16, u8)>,
}

pub const SENTINEL: u16 = 0xFFFF;

/// 写事件序列 → 最终值映射（同址 last-wins，按地址排序）。契约：L2 写集=最终值映射。
pub fn final_writes(events: Vec<(u16, u8)>) -> Vec<(u16, u8)> {
    let mut map = std::collections::BTreeMap::new();
    for (a, v) in events {
        map.insert(a, v);
    }
    map.into_iter().collect()
}
const MAX_STEPS: usize = 1_000_000;

/// 在 prg（128KB PRG，8×16KB）上执行单个子程序。
/// `entry` 为 CPU 地址；`window_bank` 为 $8000 窗口 bank 号（全程生效；bank07 用例令 window_bank=7，
/// 窗口态假设见 rounds/r3 遗留风险）。
pub fn run_subroutine(
    prg: &[u8],
    entry: u16,
    window_bank: usize,
    input: &L2In,
    stops: &[u16],
) -> Result<L2RunResult, String> {
    let mut cpu = Cpu::new(prg, window_bank, input);
    cpu.set_pc(entry);
    if stops.is_empty() {
        // 调用约定：预压哨兵返回地址（RTS 弹出后 pc=SENTINEL）
        cpu.sp = 0xF8;
        let ret = SENTINEL.wrapping_sub(1);
        cpu.push8((ret >> 8) as u8);
        cpu.push8((ret & 0xFF) as u8);
    }
    for _ in 0..MAX_STEPS {
        if stops.contains(&cpu.pc) {
            break;
        }
        if cpu.pc == SENTINEL {
            break;
        }
        cpu.step()?;
    }
    // r3 F1：任一约定下跑满步数均 fail-loud（此前哨兵约定静默截断为 Ok）
    if cpu.pc != SENTINEL && !stops.contains(&cpu.pc) {
        return Err(format!("执行超步数上限（疑似缺 stops 或不终止）entry=${entry:04X}"));
    }
    Ok(L2RunResult {
        a: cpu.a,
        x: cpu.x,
        y: cpu.y,
        // r3 F3：写集=最终值映射（同址多次写 last-wins；契约见 oracle/subroutines/README.md）
        writes: final_writes(cpu.take_writes()),
    })
}

impl Cpu {
    /// L2 引擎专用：压栈一字节（与 cpu6502::push 语义一致）。
    pub(crate) fn push8(&mut self, v: u8) {
        let addr = 0x0100 | self.sp as u16;
        self.store_ram(addr, v);
        self.sp = self.sp.wrapping_sub(1);
    }
}

// === task-38 ②：引擎单测 watchdog——spin 构造测试（坏例先喂：JMP-to-self 永不达
// stops → MAX_STEPS 限时打回=挂死防实证；测试限时=步数上限即 watchdog）。===
#[cfg(test)]
mod task38_watchdog {
    use super::*;

    /// spin 坏例：入口处 JMP $8000（自跳死循环）——无 stops。
    /// 判据：run_subroutine 在 MAX_STEPS 上限 fail-loud（超步数错误），不挂死。
    #[test]
    fn spin_program_bounded_fail_not_hang() {
        let mut prg = vec![0u8; 0x20000]; // 128KB PRG（Cpu::new 断言面）
        prg[0x0000] = 0x4C; // JMP abs（bank0 偏移 0=$8000 窗口映射）
        prg[0x0001] = 0x00;
        prg[0x0002] = 0x80; // $8000 = 自身（spin 死循环）
        let input = game_core::L2In {
            wram: vec![0; 0x2000],
            ram: vec![0; 0x800],
            a: 0,
            x: 0,
            y: 0,
            p: 0,
            window_bank: 0,
        };
        let t0 = std::time::Instant::now();
        let res = run_subroutine(&prg, 0x8000, 0, &input, &[]);
        let dt = t0.elapsed();
        assert!(res.is_err(), "spin 坏例必须 fail-loud（超步数错误）——实得 {res:?}");
        let e = res.unwrap_err();
        assert!(e.contains("超步数"), "错误须显式=超步数上限：{e}");
        // watchdog 语义：限时打回（1M 步 Rust 引擎应秒级）——防挂死回归锚
        assert!(dt.as_secs() < 30, "spin 打回耗时 {}s（>30s=护栏失效）", dt.as_secs());
    }
}

/// run_subroutine 插桩变体（**trace 面**——PC/opcode/regs 逐字节实证（D3 最终真相））。
pub fn run_subroutine_traced(
    prg: &[u8], entry: u16, window_bank: usize,
    input: &L2In, stops: &[u16], max_trace: usize,
) -> Result<(L2RunResult, Vec<(u16, u8, u8, u8)>), String> {
    let mut cpu = Cpu::new(prg, window_bank, input);
    cpu.set_pc(entry);
    if stops.is_empty() {
        cpu.sp = 0xF8;
        let ret = SENTINEL.wrapping_sub(1);
        cpu.push8((ret >> 8) as u8);
        cpu.push8((ret & 0xFF) as u8);
    }
    let mut trace = Vec::new();
    for _ in 0..MAX_STEPS {
        if stops.contains(&cpu.pc) || cpu.pc == SENTINEL {
            break;
        }
        let pc = cpu.pc;
        if trace.len() < max_trace {
            trace.push((pc, cpu.a, cpu.x, cpu.y)); // pc+regs（opcode 随件）
        }
        cpu.step()?;
    }
    let r = L2RunResult {
        a: cpu.a, x: cpu.x, y: cpu.y,
        writes: final_writes(cpu.take_writes()),
    };
    Ok((r, trace))
}

/// anim 写点迹（**D3 定案法**——$6AF7 写时 A/pc 实证）。
pub fn run_subroutine_writepoints(
    prg: &[u8], entry: u16, window_bank: usize,
    input: &L2In, stops: &[u16],
) -> Result<Vec<(u16, u8, u8)>, String> {
    let mut cpu = Cpu::new(prg, window_bank, input);
    cpu.set_pc(entry);
    if stops.is_empty() {
        cpu.sp = 0xF8;
        let ret = SENTINEL.wrapping_sub(1);
        cpu.push8((ret >> 8) as u8);
        cpu.push8((ret & 0xFF) as u8);
    }
    let mut wps = Vec::new();
    for _ in 0..MAX_STEPS {
        if stops.contains(&cpu.pc) || cpu.pc == SENTINEL {
            break;
        }
        let pc = cpu.pc;
        let a = cpu.a;
        let wlen = cpu.take_writes().len(); // 写集长度（前后差=写点）
        cpu.step()?;
        let wlen2 = cpu.take_writes().len();
        if wlen2 > wlen {
            wps.push((pc, a, wlen2 as u8)); // 写点（pc/A/写数）
        }
    }
    Ok(wps)
}

#[cfg(test)]
mod trace_probe {
    use super::*;
    /// case-7 执行迹探针（Δ 形定案法——pc/regs 逐字节）。
    #[test]
    fn case7_trace_probe() {
        // case-7：entry=bank01:8003（CommonJump_noMove）——ROM 提取
        let rom = std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/../../rom/Metroid (USA).nes")).expect("ROM");
        let prg = &rom[0x10..0x20010];
        let mut ram = vec![0u8; 0x800];
        // case 全量装载（保真面——D3 迹实证输入）
        for (k, v) in [("0000",249u8),("0053",0),("00D0",0),("0300",249),("030D",5),
                        ("030E",6),("0353",0),("0360",0),("0361",0),("03D0",249),
                        ("03DD",0),("03DE",0)] {
            ram[usize::from_str_radix(k, 16).unwrap()] = v;
        }
        let input = L2In {
            wram: vec![0; 0x2000],
            a: 104, x: 80, y: 83, p: 106,
            ram, window_bank: 1,
        };
        let (r, trace) = run_subroutine_traced(prg, 0x8006, 1, &input, &[], 64)
            .expect("trace run");
        println!("case-7 结果: a={} x={} y={}（期望 y=83）", r.a, r.x, r.y);
        println!("执行迹（pc/regs）:");
        for (i, (pc, a, x, y)) in trace.iter().enumerate().take(24) {
            println!("  [{i:2}] pc={pc:04X} a={a:02X} x={x:02X} y={y:02X}");
        }
        let wps = run_subroutine_writepoints(prg, 0x8006, 1, &input, &[]).expect("wps");
        println!("写点迹（pc/A/写数）:");
        for (pc, a, n) in &wps {
            println!("  wp pc={pc:04X} A={a:02X} n={n}");
        }
    }
}
