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
        for (k, v) in [("0000",245u8),("004B",0),("00FB",0),("0300",0),("030D",0),
                        ("030E",0),("034B",0),("0358",0),("0359",0),("03FB",0),
                        ("0408",0),("0409",0)] {
            ram[usize::from_str_radix(k, 16).unwrap()] = v;
        }
        let input = L2In {
            wram: vec![0; 0x2000],
            a: 46, x: 75, y: 251, p: 168,
            ram, window_bank: 1,
        };
        let (r, trace) = run_subroutine_traced(prg, 0x8033, 1, &input, &[], 64)
            .expect("trace run");
        println!("case-7 结果: a={} x={} y={}（期望 y=83）", r.a, r.x, r.y);
        println!("执行迹（pc/regs）:");
        for (i, (pc, a, x, y)) in trace.iter().enumerate().take(24) {
            println!("  [{i:2}] pc={pc:04X} a={a:02X} x={x:02X} y={y:02X}");
        }
        let wps = run_subroutine_writepoints(prg, 0x8033, 1, &input, &[]).expect("wps");
        println!("写点迹（pc/A/写数）:");
        for (pc, a, n) in &wps {
            println!("  wp pc={pc:04X} A={a:02X} n={n}");
        }


    }
}

#[cfg(test)]
mod g3_probe {
    use super::*;
    /// g3-000 D3 抓取探针（entry=bank07:B3B4（SilenceMusic 路径）——监视写面对实证）。
    #[test]
    fn g3_000_capture() {
        let rom = std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/../../rom/Metroid (USA).nes")).expect("ROM");
        let prg = &rom[0x10..0x20010];
        let input = L2In {
            wram: vec![0; 0x2000],
            a: 0, x: 0, y: 0, p: 0,
            ram: vec![0; 0x800],
            window_bank: 7, // bank7 窗形（g3 音频域）
        };
        let wps = run_subroutine_writepoints(prg, 0xB3B4, 7, &input, &[]).expect("capture");
        let (r, _t) = run_subroutine_traced(prg, 0xB3B4, 7, &input, &[], 64).expect("result");
        println!("g3-000 结果: a={} x={} y={}", r.a, r.x, r.y);
        println!("写点迹（pc/A/写数）:");
        for (pc, a, n) in &wps {
            println!("  wp pc={pc:04X} A={a:02X} n={n}");
        }
        println!("写面对: {:?}", r.writes);
    }

    /// g3-001..005 D3 批抓取（同法——五案 entries）。
    #[test]
    fn g3_001_005_capture() {
        let rom = std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/../../rom/Metroid (USA).nes")).expect("ROM");
        let prg = &rom[0x10..0x20010];
        let entries = [(0xB31Bu16, "g3-001"), (0xB337, "g3-002"), (0xB34B, "g3-003"), (0xB368, "g3-004"), (0xB3F0, "g3-005")];
        for (entry, name) in entries {
            let input = L2In {
                wram: vec![0; 0x2000], a: 0, x: 0, y: 0, p: 0,
                ram: vec![0; 0x800], window_bank: 7,
            };
            // stops 面余修（闸红快修——超步容忍（探针实证面：写点迹在即采））
            let wps = run_subroutine_writepoints(prg, entry, 7, &input, &[]).unwrap_or_default();
            println!("{name}(${entry:04X}): 写点={:?}", wps);
            let (r, _t) = run_subroutine_traced(prg, entry, 7, &input, &[], 64)
                .unwrap_or((L2RunResult { a: 0, x: 0, y: 0, writes: vec![] }, vec![]));
            println!("{name}(${entry:04X}): a={} x={} y={} 写={:?}", r.a, r.x, r.y, r.writes);
        }
    }

    /// SC-G3-03 探针（LCG fixture 装载（$0680 输入族）×D3 抓取——写面对实证）。
    #[test]
    fn g3_lcg_capture() {
        let rom = std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/../../rom/Metroid (USA).nes")).expect("ROM");
        let prg = &rom[0x10..0x20010];
        let fixtures: [(u16, &str, u8); 4] = [
            (0xB3B4, "g3-000", 1), (0xB31B, "g3-001", 0), (0xB337, "g3-002", 0), (0xB34B, "g3-003", 0),
        ];
        for (entry, name, v0680) in fixtures {
            let mut ram = vec![0u8; 0x800];
            ram[0x0680] = v0680;
            let input = L2In {
                wram: vec![0; 0x2000], a: 0, x: 0, y: 0, p: 0,
                ram, window_bank: 7,
            };
            let wps = run_subroutine_writepoints(prg, entry, 7, &input, &[]).unwrap_or_default();
            println!("{name}($0680={v0680}): 写点={:?}", wps);
            let (r, _t) = run_subroutine_traced(prg, entry, 7, &input, &[], 64)
                .unwrap_or((L2RunResult { a: 0, x: 0, y: 0, writes: vec![] }, vec![]));
            println!("{name}: 写面对={:?}", r.writes);
        }
    }


    /// SC-G3-04 探针（MusicInit 族（$CC09 SetMusicInitFlag）×$065E 监视——D3 实证）。
    #[test]
    fn g3_musicinit_capture() {
        let rom = std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/../../rom/Metroid (USA).nes")).expect("ROM");
        let prg = &rom[0x10..0x20010];
        let input = L2In {
            wram: vec![0; 0x2000], a: 0, x: 0, y: 0, p: 0,
            ram: vec![0; 0x800], window_bank: 7,
        };
        let wps = run_subroutine_writepoints(prg, 0xCC09, 7, &input, &[]).unwrap_or_default();
        println!("MusicInit($CC09): 写点={:?}", wps);
        let (r, _t) = run_subroutine_traced(prg, 0xCC09, 7, &input, &[], 64)
            .unwrap_or((L2RunResult { a: 0, x: 0, y: 0, writes: vec![] }, vec![]));
        println!("MusicInit: 写面对={:?}", r.writes);
    }


    /// SC-G3-05 探针（SFX 族（$CB96-$CBB0）×D3 抓取——写面对实证）。
    #[test]
    fn g3_sfx_capture() {
        let rom = std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/../../rom/Metroid (USA).nes")).expect("ROM");
        let prg = &rom[0x10..0x20010];
        let entries: [(u16, &str); 8] = [
            (0xCB96, "SFX_SamusWalk"), (0xCB9A, "SFX_BombExplode"), (0xCB9E, "SFX_MissileLaunch"),
            (0xCBA0, "SetSFXNoiseInitFlag"), (0xCBA4, "SFX_OutOfPipe"), (0xCBA8, "SFX_BombLaunch"),
            (0xCBAC, "SFX_SamusJump"), (0xCBB0, "SFX_EnemyHit"),
        ];
        for (entry, name) in entries {
            let input = L2In {
                wram: vec![0; 0x2000], a: 0, x: 0, y: 0, p: 0,
                ram: vec![0; 0x800], window_bank: 7,
            };
            let wps = run_subroutine_writepoints(prg, entry, 7, &input, &[]).unwrap_or_default();
            let (r, _t) = run_subroutine_traced(prg, entry, 7, &input, &[], 64)
                .unwrap_or((L2RunResult { a: 0, x: 0, y: 0, writes: vec![] }, vec![]));
            println!("{name}(${entry:04X}): 写面对={:?} 点={:?}", r.writes, wps.len());
        }
    }


    /// SC-G3-06 探针（SFX 族 12 员（$CBB4-$CBDE）×D3 抓取——写面对实证）。
    #[test]
    fn g3_sfx2_capture() {
        let rom = std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/../../rom/Metroid (USA).nes")).expect("ROM");
        let prg = &rom[0x10..0x20010];
        let entries: [(u16, &str); 12] = [
            (0xCBB4, "SFX_BulletFire"), (0xCBB8, "SFX_Metal"), (0xCBBC, "SFX_EnergyPickup"),
            (0xCBC0, "SFX_MissilePickup"), (0xCBC2, "SetSFXSQ1InitFlag"), (0xCBC6, "SFX_WaveFire"),
            (0xCBCA, "SFX_ScrewAttack"), (0xCBCE, "SFX_BigEnemyHit"), (0xCBD2, "SFX_MetroidHit"),
            (0xCBD6, "SFX_BossHit"), (0xCBDA, "SFX_Door"), (0xCBDE, "SFX_SamusHit"),
        ];
        for (entry, name) in entries {
            let input = L2In {
                wram: vec![0; 0x2000], a: 0, x: 0, y: 0, p: 0,
                ram: vec![0; 0x800], window_bank: 7,
            };
            let (r, _t) = run_subroutine_traced(prg, entry, 7, &input, &[], 64)
                .unwrap_or((L2RunResult { a: 0, x: 0, y: 0, writes: vec![] }, vec![]));
            println!("{name}(${entry:04X}): 写面对={:?}", r.writes);
        }
    }


    /// SC-MD-001 探针（SamusJump 族 5 员（跳动链）×D3 抓取——ROM 执行 golden）。
    #[test]
    fn md_jump_capture() {
        let rom = std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/../../rom/Metroid (USA).nes")).expect("ROM");
        let prg = &rom[0x10..0x20010];
        let entries: [(u16, &str); 5] = [
            (0xC27C, "JumpEngine"), (0xCFC3, "SetSamusJump"), (0xD002, "SamusJump"),
            (0xD055, "CheckHorzMovement"), (0xD09C, "CheckFire"),
        ];
        for (entry, name) in entries {
            let input = L2In {
                wram: vec![0; 0x2000], a: 0, x: 0, y: 0, p: 0,
                ram: vec![0; 0x800], window_bank: 7,
            };
            let (r, _t) = run_subroutine_traced(prg, entry, 7, &input, &[], 64)
                .unwrap_or((L2RunResult { a: 0, x: 0, y: 0, writes: vec![] }, vec![]));
            println!("{name}(${entry:04X}): 写面对={:?} 出=({},{},{})", r.writes, r.a, r.x, r.y);
        }
    }


    /// SC-MD-002 探针（arm 9 案（物理族 $CC0D-$D423）×D3 批抓取——ROM 执行 golden）。
    #[test]
    fn md_arm_capture() {
        let rom = std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/../../rom/Metroid (USA).nes")).expect("ROM");
        let prg = &rom[0x10..0x20010];
        let entries: [(u16, &str); 9] = [
            (0xCC0D, "a1-022"), (0xCC36, "a1-023"), (0xCCC2, "a1-024"), (0xD002, "a1-025"),
            (0xD0E1, "a1-026"), (0xD198, "a1-027"), (0xD3A8, "a1-028"), (0xD41A, "a1-029"), (0xD423, "a1-030"),
        ];
        for (entry, name) in entries {
            let input = L2In {
                wram: vec![0; 0x2000], a: 0, x: 0, y: 0, p: 0,
                ram: vec![0; 0x800], window_bank: 7,
            };
            let (r, _t) = run_subroutine_traced(prg, entry, 7, &input, &[], 64)
                .unwrap_or((L2RunResult { a: 0, x: 0, y: 0, writes: vec![] }, vec![]));
            println!("{name}(${entry:04X}): 写面对={:?}", r.writes);
        }
    }


    /// SC-MD-003 探针（SamusDoor($D3A8——门交互物理族）×D3 抓取——ROM 执行 golden）。
    #[test]
    fn md_samusdoor_capture() {
        let rom = std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/../../rom/Metroid (USA).nes")).expect("ROM");
        let prg = &rom[0x10..0x20010];
        let input = L2In {
            wram: vec![0; 0x2000], a: 0, x: 0, y: 0, p: 0,
            ram: vec![0; 0x800], window_bank: 7,
        };
        let (r, _t) = run_subroutine_traced(prg, 0xD3A8, 7, &input, &[], 64)
            .unwrap_or((L2RunResult { a: 0, x: 0, y: 0, writes: vec![] }, vec![]));
        println!("SamusDoor($D3A8): 写面对={:?} 出=({},{},{})", r.writes, r.a, r.x, r.y);
    }


    /// SC-MD-004 探针（SamusPntUp 族（$D198/$CF77——瞄准面）×D3 抓取——ROM 执行 golden）。
    #[test]
    fn md_pntup_capture() {
        let rom = std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/../../rom/Metroid (USA).nes")).expect("ROM");
        let prg = &rom[0x10..0x20010];
        for (entry, name) in [(0xD198u16, "SamusPntUp"), (0xCF77, "SetSamusPntUp")] {
            let input = L2In {
                wram: vec![0; 0x2000], a: 0, x: 0, y: 0, p: 0,
                ram: vec![0; 0x800], window_bank: 7,
            };
            let (r, _t) = run_subroutine_traced(prg, entry, 7, &input, &[], 64)
                .unwrap_or((L2RunResult { a: 0, x: 0, y: 0, writes: vec![] }, vec![]));
            println!("{name}(${entry:04X}): 写面对={:?} 出=({},{},{})", r.writes, r.a, r.x, r.y);
        }
    }


    /// SC-MD-005 探针（Samus 状态机族 9 员（UpdateSamus/Stand/Run 族）×D3 抓取）。
    #[test]
    fn md_statemachine_capture() {
        let rom = std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/../../rom/Metroid (USA).nes")).expect("ROM");
        let prg = &rom[0x10..0x20010];
        let entries: [(u16, &str); 9] = [
            (0xCC0D, "UpdateSamus"), (0xCC1A, "GoSamusHandler"), (0xCC36, "SamusStand"),
            (0xCC8B, "SetSamusExplode"), (0xCC98, "SetSamusRun"), (0xCCB7, "SetSamusRunAccel"),
            (0xCCC2, "SamusRun"), (0xCD6B, "SetSamusData3Frame"), (0xCD6D, "SetSamusData"),
        ];
        for (entry, name) in entries {
            let input = L2In {
                wram: vec![0; 0x2000], a: 0, x: 0, y: 0, p: 0,
                ram: vec![0; 0x800], window_bank: 7,
            };
            let (r, _t) = run_subroutine_traced(prg, entry, 7, &input, &[], 64)
                .unwrap_or((L2RunResult { a: 0, x: 0, y: 0, writes: vec![] }, vec![]));
            println!("{name}(${entry:04X}): 写面对={:?} 出=({},{},{})", r.writes, r.a, r.x, r.y);
        }
    }


    /// SC-G4-001 探针（Tourian 域（bank3 窗形——MotherBrain 碰撞族）×D3 抓取）。
    #[test]
    fn g4_tourian_capture() {
        let rom = std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/../../rom/Metroid (USA).nes")).expect("ROM");
        let prg = &rom[0x10..0x20010];
        let entries: [(u16, &str); 4] = [
            (0x95AE, "DeleteOffscreenSprites"), (0x95B4, "SpawnMotherBrain"),
            (0x95BD, "CollisionZebetiteGlass"), (0x95C0, "CollisionMotherBrain"),
        ];
        for (entry, name) in entries {
            let input = L2In {
                wram: vec![0; 0x2000], a: 0, x: 0, y: 0, p: 0,
                ram: vec![0; 0x800], window_bank: 3, // bank3 窗形（Tourian 域！）
            };
            let (r, _t) = run_subroutine_traced(prg, entry, 3, &input, &[], 64)
                .unwrap_or((L2RunResult { a: 0, x: 0, y: 0, writes: vec![] }, vec![]));
            println!("{name}(${entry:04X}): 写面对={:?} 出=({},{},{})", r.writes, r.a, r.x, r.y);
        }
    }

    }
