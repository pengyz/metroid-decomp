//! l2-diff —— L2 三方裁定 CLI（subroutine-diff 门禁入口的唯一裁定逻辑）。
//!
//! 用法: l2-diff --rom <rom.nes> --cases <l2_cases.jsonl>
//! 裁定: case.out(golden) == 6502 核执行 ROM 原字节 == game-core 同名函数。
//! 用例格式见 oracle/subroutines/README.md。

use game_core::{l2ports, L2In, L2Out};
use game_oracle::run_subroutine;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::process::ExitCode;

#[derive(Deserialize)]
struct Case {
    id: String,
    subroutine: String,
    entry: String,
    #[serde(default)]
    stops: Vec<String>,
    #[serde(rename = "in")]
    input: CaseIn,
    #[serde(rename = "out")]
    output: CaseOut,
}

#[derive(Deserialize)]
struct CaseIn {
    #[serde(default)]
    regs: BTreeMap<String, u8>,
    #[serde(default)]
    ram: BTreeMap<String, u8>,
}

#[derive(Deserialize)]
struct CaseOut {
    #[serde(default)]
    regs: BTreeMap<String, u8>,
    #[serde(default)]
    writes: BTreeMap<String, u8>,
}

/// "bankNN:AAAA" 或 "AAAA" → CPU 地址。
fn parse_cpu_addr(s: &str) -> Result<u16, String> {
    let hex = s.rsplit(':').next().ok_or_else(|| format!("bad addr {s}"))?;
    u16::from_str_radix(hex, 16).map_err(|e| format!("bad addr {s}: {e}"))
}

fn bank_of(s: &str) -> Result<usize, String> {
    if let Some(rest) = s.strip_prefix("bank") {
        let n: usize = rest.split(':').next().unwrap_or("7").parse().map_err(|e| format!("{e}"))?;
        Ok(n)
    } else {
        Ok(7)
    }
}

fn golden_writes(c: &CaseOut) -> Result<Vec<(u16, u8)>, String> {
    let mut v = Vec::new();
    for (k, b) in &c.writes {
        v.push((u16::from_str_radix(k, 16).map_err(|e| format!("{e}"))?, *b));
    }
    v.sort();
    Ok(v)
}

fn norm_writes(mut v: Vec<(u16, u8)>) -> Vec<(u16, u8)> {
    v.sort();
    v
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    let mut rom_path = String::new();
    let mut cases_path = String::new();
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--rom" => { rom_path = args.get(i + 1).cloned().unwrap_or_default(); i += 2; }
            "--cases" => { cases_path = args.get(i + 1).cloned().unwrap_or_default(); i += 2; }
            other => { eprintln!("未知参数: {other}"); return ExitCode::from(2); }
        }
    }
    if rom_path.is_empty() || cases_path.is_empty() {
        eprintln!("用法: l2-diff --rom <rom.nes> --cases <l2_cases.jsonl>");
        return ExitCode::from(2);
    }

    let rom = match std::fs::read(&rom_path) {
        Ok(v) => v,
        Err(e) => { eprintln!("读 ROM 失败: {e}"); return ExitCode::from(2); }
    };
    let prg: Vec<u8> = if rom.len() >= 0x20010 && &rom[0..4] == b"NES\x1A" {
        rom[0x10..0x20010].to_vec()
    } else if rom.len() == 0x20000 {
        rom
    } else {
        eprintln!("ROM 尺寸/魔数不符（期望 iNES 131088B 或裸 PRG 131072B）");
        return ExitCode::from(2);
    };

    let cases_text = match std::fs::read_to_string(&cases_path) {
        Ok(t) => t,
        Err(e) => { eprintln!("读用例失败: {e}"); return ExitCode::from(2); }
    };

    let mut total = 0usize;
    let mut failed = 0usize;
    for (lineno, line) in cases_text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let case: Case = match serde_json::from_str(line) {
            Ok(c) => c,
            Err(e) => { eprintln!("用例第 {} 行解析失败: {e}", lineno + 1); failed += 1; total += 1; continue; }
        };
        total += 1;
        match run_case(&prg, &case) {
            Ok(()) => println!("[PASS] {}", case.id),
            Err(msg) => { println!("[FAIL] {}: {}", case.id, msg); failed += 1; }
        }
    }
    if total == 0 {
        eprintln!("L2 三方裁定: 用例集为空——fail-loud（r3 F7：空跑不得绿）");
        return ExitCode::from(2);
    }
    println!("L2 三方裁定: {} 条用例, {} 失败（golden == ROM 执行 == game-core）", total, failed);
    if failed > 0 { ExitCode::from(1) } else { ExitCode::from(0) }
}

fn run_case(prg: &[u8], case: &Case) -> Result<(), String> {
    let entry = parse_cpu_addr(&case.entry)?;
    let window_bank = bank_of(&case.entry)?;
    let mut stops = Vec::new();
    for s in &case.stops {
        stops.push(parse_cpu_addr(s)?);
    }
    let mut ram = vec![0u8; 0x800];
    for (k, v) in &case.input.ram {
        let addr = u16::from_str_radix(k, 16).map_err(|e| format!("{e}"))? as usize;
        ram[addr & 0x7FF] = *v;
    }
    let input = L2In {
        a: *case.input.regs.get("a").unwrap_or(&0),
        x: *case.input.regs.get("x").unwrap_or(&0),
        y: *case.input.regs.get("y").unwrap_or(&0),
        p: *case.input.regs.get("p").unwrap_or(&0x24),
        ram,
    };

    // golden 侧
    let g = (
        *case.output.regs.get("a").unwrap_or(&0),
        *case.output.regs.get("x").unwrap_or(&0),
        *case.output.regs.get("y").unwrap_or(&0),
        golden_writes(&case.output)?,
    );

    // ROM 执行侧（6502 核直跑原字节）
    let rom_run = run_subroutine(prg, entry, window_bank, &input, &stops)?;
    let r = (rom_run.a, rom_run.x, rom_run.y, norm_writes(rom_run.writes.clone()));

    // game-core 侧
    let port = l2ports::lookup(&case.subroutine)
        .ok_or_else(|| format!("game-core 缺移植函数 {}（实现缺口）", case.subroutine))?;
    let core: L2Out = port(&input);
    let c = (core.a, core.x, core.y, norm_writes(core.writes.clone()));

    if g != r {
        return Err(format!("golden≠ROM执行: golden={g:?} rom={r:?}"));
    }
    if g != c {
        return Err(format!("golden≠game-core: golden={g:?} core={c:?}"));
    }
    Ok(())
}
