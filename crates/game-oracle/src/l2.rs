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
const MAX_STEPS: usize = 1_000_000;

/// 在 prg（128KB PRG，8×16KB）上执行单个子程序。
/// `entry` 为 CPU 地址；`window_bank` 为 $8000 窗口 bank 号（entry 在 $C000+ 时忽略）。
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
    if !stops.is_empty() && !stops.contains(&cpu.pc) && cpu.pc != SENTINEL {
        return Err(format!("执行超步数上限（疑似缺 stops 或不终止）entry=${entry:04X}"));
    }
    Ok(L2RunResult {
        a: cpu.a,
        x: cpu.x,
        y: cpu.y,
        writes: cpu.take_writes(),
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
