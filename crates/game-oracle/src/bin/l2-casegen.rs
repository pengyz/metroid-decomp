//! l2-casegen —— L2 用例自动生成器（batch-0 吞吐引擎）。
//!
//! 用法: l2-casegen --rom <rom.nes> --funcs <funcs.jsonl> --out <cases.jsonl>
//! funcs.jsonl 行格式:
//!   {"id","subroutine","entry":"bankNN:AAAA","stops":["bankNN:BBBB"],
//!    "samples":N,"ram_addrs":["0308","0312",...]}
//! 生成语义（completeness.md §3）：确定性合成入参（LCG，种子=func id + 序号）→ ROM 腿执行
//! → golden=out（=ROM 执行结果）。**golden 自 ROM 产出，非手推**——ROM 腿独立裁决移植；
//! 考题抽样人工复核防 same-source（r3 遗留项）。
//! 输出 case 与 oracle/subroutines/l2_cases.jsonl 同 schema，可直接喂 l2-diff。

use game_core::L2In;
use game_oracle::run_subroutine;
use serde::Deserialize;
use std::process::ExitCode;

#[derive(Deserialize)]
struct FuncSpec {
    id: String,
    subroutine: String,
    entry: String,
    stops: Vec<String>,
    #[serde(default = "default_samples")]
    samples: usize,
    #[serde(default)]
    ram_addrs: Vec<String>,
}

fn default_samples() -> usize {
    4
}

/// 确定性 LCG（种子混合 func id 字节 + 样本序号）——跨平台同序列。
struct Lcg(u64);
impl Lcg {
    fn new(seed: u64) -> Self {
        Lcg(seed ^ 0x9E3779B97F4A7C15)
    }
    fn next_u8(&mut self) -> u8 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.0 >> 56) as u8
    }
}

fn parse_cpu_addr(s: &str) -> Result<u16, String> {
    let hex = s.rsplit(':').next().ok_or_else(|| format!("bad addr {s}"))?;
    u16::from_str_radix(hex, 16).map_err(|e| format!("bad addr {s}: {e}"))
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    let mut rom_path = String::new();
    let mut funcs_path = String::new();
    let mut out_path = String::new();
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--rom" => { rom_path = args.get(i + 1).cloned().unwrap_or_default(); i += 2; }
            "--funcs" => { funcs_path = args.get(i + 1).cloned().unwrap_or_default(); i += 2; }
            "--out" => { out_path = args.get(i + 1).cloned().unwrap_or_default(); i += 2; }
            other => { eprintln!("未知参数: {other}"); return ExitCode::from(2); }
        }
    }
    if rom_path.is_empty() || funcs_path.is_empty() || out_path.is_empty() {
        eprintln!("用法: l2-casegen --rom <rom.nes> --funcs <funcs.jsonl> --out <cases.jsonl>");
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
        eprintln!("ROM 尺寸/魔数不符");
        return ExitCode::from(2);
    };

    let funcs_text = match std::fs::read_to_string(&funcs_path) {
        Ok(t) => t,
        Err(e) => { eprintln!("读 funcs 失败: {e}"); return ExitCode::from(2); }
    };
    let mut out_lines: Vec<String> = Vec::new();
    let mut total = 0usize;
    for (lineno, line) in funcs_text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let spec: FuncSpec = match serde_json::from_str(line) {
            Ok(s) => s,
            Err(e) => { eprintln!("funcs 第 {} 行解析失败: {e}", lineno + 1); return ExitCode::from(2); }
        };
        let entry = match parse_cpu_addr(&spec.entry) {
            Ok(a) => a,
            Err(e) => { eprintln!("{}", e); return ExitCode::from(2); }
        };
        let window_bank = spec
            .entry
            .strip_prefix("bank")
            .and_then(|r| r.split(':').next())
            .and_then(|n| n.parse::<usize>().ok())
            .unwrap_or(7);
        let mut stops = Vec::new();
        for s in &spec.stops {
            match parse_cpu_addr(s) {
                Ok(a) => stops.push(a),
                Err(e) => { eprintln!("{}", e); return ExitCode::from(2); }
            }
        }
        for n in 0..spec.samples {
            let mut seed: u64 = 0;
            for b in spec.id.bytes() {
                seed = seed.wrapping_mul(31).wrapping_add(b as u64);
            }
            let mut rng = Lcg::new(seed.wrapping_add(n as u64));
            let mut ram = vec![0u8; 0x800];
            let mut wram = vec![0u8; 0x2000];
            for addr_s in &spec.ram_addrs {
                let addr = u16::from_str_radix(addr_s, 16).map_err(|e| format!("{e}"));
                match addr {
                    Ok(a) => {
                        let v = rng.next_u8();
                        if (0x6000..0x8000).contains(&a) {
                            wram[a as usize - 0x6000] = v; // $6AFx 敌槽/暂存域
                        } else {
                            ram[(a as usize) & 0x7FF] = v;
                        }
                    }
                    Err(e) => { eprintln!("ram_addrs 错误: {}", e); return ExitCode::from(2); }
                }
            }
            let input = L2In {
                wram,
                a: rng.next_u8(),
                x: rng.next_u8(),
                y: rng.next_u8(),
                p: rng.next_u8() | 0x20,
                ram,
            };
            let run = match run_subroutine(&prg, entry, window_bank, &input, &stops) {
                Ok(r) => r,
                Err(e) => {
                    eprintln!("[gen-FAIL] {}#{}: ROM 腿执行失败: {e}", spec.id, n);
                    return ExitCode::from(1);
                }
            };
            // golden=ROM 执行结果；写集=最终值映射（与 l2-diff 契约一致）
            let mut writes: Vec<(String, u8)> = run
                .writes
                .iter()
                .map(|(a, v)| (format!("{a:04X}"), *v))
                .collect();
            writes.sort();
            let writes_json: Vec<String> = writes
                .iter()
                .map(|(a, v)| format!("\"{a}\":{v}"))
                .collect();
            let ram_in: Vec<String> = spec
                .ram_addrs
                .iter()
                .filter_map(|a| {
                    let addr = u16::from_str_radix(a, 16).ok()?;
                    let v = if (0x6000..0x8000).contains(&addr) {
                        input.wram.get(addr as usize - 0x6000).copied().unwrap_or(0)
                    } else {
                        input.ram[(addr as usize) & 0x7FF]
                    };
                    Some(format!("\"{a}\":{}", v))
                })
                .collect();
            out_lines.push(format!(
                "{{\"id\":\"{id}-{n}\",\"subroutine\":\"{sub}\",\"entry\":\"{entry}\",\"stops\":[{stops}],\"in\":{{\"regs\":{{\"a\":{a},\"x\":{x},\"y\":{y},\"p\":{p}}},\"ram\":{{{ram_in}}}}},\"out\":{{\"regs\":{{\"a\":{ra},\"x\":{rx},\"y\":{ry}}},\"writes\":{{{writes}}}}}}}",
                id = spec.id,
                sub = spec.subroutine,
                entry = spec.entry,
                stops = spec.stops.iter().map(|s| format!("\"{s}\"")).collect::<Vec<_>>().join(","),
                a = input.a, x = input.x, y = input.y, p = input.p,
                ram_in = ram_in.join(","),
                ra = run.a, rx = run.x, ry = run.y,
                writes = writes_json.join(","),
            ));
            total += 1;
        }
    }
    if let Err(e) = std::fs::write(&out_path, out_lines.join("\n") + "\n") {
        eprintln!("写输出失败: {e}");
        return ExitCode::from(2);
    }
    println!("L2 case factory: 生成 {total} 条用例 → {out_path}");
    ExitCode::from(0)
}
