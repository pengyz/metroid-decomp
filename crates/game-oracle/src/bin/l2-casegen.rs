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
    /// $8000 窗口 bank（task-14 窗口参数化契约面，∈0..8）。缺省=entry bank 标签；
    /// 多窗口 funcs 显式携带（生成 case 时透传该字段）。
    #[serde(default)]
    window_bank: Option<usize>,
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
        // task-14：窗口 bank 显式字段优先，缺省=entry bank 标签（既有生成行为钉死）
        let window_bank = match spec.window_bank {
            Some(w) if w < 8 => w,
            Some(w) => {
                eprintln!("funcs 第 {} 行 window_bank={w} 超出 0..8 契约域——fail-loud", lineno + 1);
                return ExitCode::from(2);
            }
            None => spec
                .entry
                .strip_prefix("bank")
                .and_then(|r| r.split(':').next())
                .and_then(|n| n.parse::<usize>().ok())
                .unwrap_or(7),
        };
        let mut stops = Vec::new();
        for s in &spec.stops {
            match parse_cpu_addr(s) {
                Ok(a) => stops.push(a),
                Err(e) => { eprintln!("{}", e); return ExitCode::from(2); }
            }
        }
        // 边界相位（b-int-y-boundary-5 教训族——判别值域补全）：samples 后追加 4 案，
        // 钳位边界池 [5,6,0xFA,0xF9]（正 5/6+负幅 6/7）——**LCG 流保持**（每值仍抽一
        // 次 rng 只覆值）——既有案 golden 不动（additive 保证）。
        const BOUNDARY_POOL: [u8; 4] = [5, 6, 0xFA, 0xF9];
        for n in 0..spec.samples + BOUNDARY_POOL.len() {
            let mut seed: u64 = 0;
            for b in spec.id.bytes() {
                seed = seed.wrapping_mul(31).wrapping_add(b as u64);
            }
            let mut rng = Lcg::new(seed.wrapping_add(n as u64));
            // 寄存器先抽样——有效址 fixture 必须用**最终** x/y 值（v2 修正：独立 hint≠寄存器）
            let reg_a = rng.next_u8();
            let reg_x = rng.next_u8();
            let reg_y = rng.next_u8();
            let reg_p = rng.next_u8() | 0x20;
            let input_x_hint = reg_x as u16;
            let input_y_hint = reg_y as u16;
            let mut ram = vec![0u8; 0x800];
            let mut wram = vec![0u8; 0x2000];
            let mut written: Vec<u16> = Vec::new(); // 回显须覆盖全部落值址（含有效址）
            for addr_s in &spec.ram_addrs {
                let addr = u16::from_str_radix(addr_s, 16).map_err(|e| format!("{e}"));
                match addr {
                    Ok(a) => {
                        // 字面地址必须在可写域（fail-loud）；辅助有效址 base+X/base+Y 可跳
                        if !((0x6000..0x8000).contains(&a) || a < 0x2000) {
                            eprintln!("ram_addrs 地址 ${a:04X} 不在可写域（<$2000 / $6000-$7FFF）——fail-loud");
                            return ExitCode::from(2);
                        }
                        // 判别力修复（F1xx 复审 2）：基址与变址有效址都赋值
                        for eff in [a, a.wrapping_add(input_x_hint), a.wrapping_add(input_y_hint)] {
                            let v_raw = rng.next_u8(); // LCG 流保持（值可覆）
                            let v = if n < spec.samples {
                                v_raw
                            } else {
                                BOUNDARY_POOL[(n - spec.samples + eff as usize) % BOUNDARY_POOL.len()]
                            };
                            if (0x6000..0x8000).contains(&eff) {
                                wram[eff as usize - 0x6000] = v;
                                written.push(eff);
                            } else if eff < 0x2000 {
                                ram[(eff as usize) & 0x7FF] = v;
                                written.push(eff);
                            }
                        }
                    }
                    Err(e) => { eprintln!("ram_addrs 错误: {}", e); return ExitCode::from(2); }
                }
            }
            let input = L2In {
                wram,
                a: reg_a,
                x: reg_x,
                y: reg_y,
                p: reg_p,
                ram,
                window_bank,
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
            written.sort_unstable();
            written.dedup();
            let ram_in: Vec<String> = written
                .iter()
                .map(|addr| {
                    let v = if (0x6000..0x8000).contains(addr) {
                        input.wram[*addr as usize - 0x6000]
                    } else {
                        input.ram[(*addr as usize) & 0x7FF]
                    };
                    format!("\"{:04X}\":{}", addr, v)
                })
                .collect();
            // task-14：显式 window_bank 透传入 case（缺省不写——既有生成文本钉死）
            let window_field = match spec.window_bank {
                Some(w) => format!(",\"window_bank\":{w}"),
                None => String::new(),
            };
            out_lines.push(format!(
                "{{\"id\":\"{id}-{n}\",\"subroutine\":\"{sub}\",\"entry\":\"{entry}\"{window_field},\"stops\":[{stops}],\"in\":{{\"regs\":{{\"a\":{a},\"x\":{x},\"y\":{y},\"p\":{p}}},\"ram\":{{{ram_in}}}}},\"out\":{{\"regs\":{{\"a\":{ra},\"x\":{rx},\"y\":{ry}}},\"writes\":{{{writes}}}}}}}",
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
