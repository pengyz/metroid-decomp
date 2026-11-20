//! 离线 APU 产物导出（Tier-1 声音验证仪器，不是游戏入口）。
//!
//! 用法：`apu_dump --schedule <file.sched> [--start N] [--frames N] --out out.wav`
//!
//! 语义：按调度表驱动 `FrameEngine`，每帧把**本帧新增**的 APU 寄存器写喂给
//! `game_platform::apu::Apu`（与壳内 `PcAudio` 同一套合成路径），帧界推进一帧
//! CPU 周期，最后落 16-bit mono WAV。因此这份 WAV 就是游戏运行时该有的声音。

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use game_core::frame::FrameEngine;
use game_platform::apu::{Apu, CPU_HZ, SAMPLE_RATE};

struct Plan {
    frames: u64,
    joy: BTreeMap<u64, u8>,
}

fn parse(text: &str) -> Plan {
    let mut frames = 0u64;
    let mut joy = BTreeMap::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut it = line.split_whitespace();
        match it.next() {
            Some("frames") => frames = it.next().and_then(|s| s.parse().ok()).unwrap_or(0),
            Some("joy") => {
                let f = it.next().and_then(|s| s.parse::<u64>().ok());
                let v = it.next().and_then(|s| u8::from_str_radix(s, 16).ok());
                if let (Some(f), Some(v)) = (f, v) {
                    joy.insert(f, v);
                }
            }
            _ => {}
        }
    }
    Plan { frames, joy }
}

fn write_wav(path: &PathBuf, pcm: &[i16]) -> std::io::Result<()> {
    use std::io::Write;
    let mut f = std::io::BufWriter::new(fs::File::create(path)?);
    let data_len = (pcm.len() * 2) as u32;
    f.write_all(b"RIFF")?;
    f.write_all(&(36 + data_len).to_le_bytes())?;
    f.write_all(b"WAVEfmt ")?;
    f.write_all(&16u32.to_le_bytes())?;
    f.write_all(&1u16.to_le_bytes())?;
    f.write_all(&1u16.to_le_bytes())?;
    f.write_all(&SAMPLE_RATE.to_le_bytes())?;
    f.write_all(&(SAMPLE_RATE * 2).to_le_bytes())?;
    f.write_all(&2u16.to_le_bytes())?;
    f.write_all(&16u16.to_le_bytes())?;
    f.write_all(b"data")?;
    f.write_all(&data_len.to_le_bytes())?;
    for s in pcm {
        f.write_all(&s.to_le_bytes())?;
    }
    f.flush()
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut schedule: Option<String> = None;
    let mut out = PathBuf::from("apu.wav");
    let mut start = 0u64;
    let mut limit: Option<u64> = None;
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--schedule" => {
                schedule = args.get(i + 1).cloned();
                i += 2;
            }
            "--out" => {
                out = PathBuf::from(args.get(i + 1).cloned().unwrap_or_default());
                i += 2;
            }
            "--start" => {
                start = args.get(i + 1).and_then(|s| s.parse().ok()).unwrap_or(0);
                i += 2;
            }
            "--frames" => {
                limit = args.get(i + 1).and_then(|s| s.parse().ok());
                i += 2;
            }
            other => {
                eprintln!("未知参数 {other}");
                std::process::exit(2);
            }
        }
    }
    let Some(sched_path) = schedule else {
        eprintln!("用法：apu_dump --schedule <file.sched> [--start N] [--frames N] --out out.wav");
        std::process::exit(2);
    };
    let text = fs::read_to_string(&sched_path).unwrap_or_else(|e| {
        eprintln!("读调度表失败 {sched_path}: {e}");
        std::process::exit(1);
    });
    let plan = parse(&text);
    let end = limit.map_or(plan.frames, |n| start.saturating_add(n));

    let mut engine = FrameEngine::new();
    let mut apu = Apu::new();
    let mut pcm: Vec<i16> = Vec::new();
    let mut cur_joy = 0u8;
    let mut cursor = 0usize;
    let mut writes_total = 0u64;
    let mut pending: Vec<(u16, u8)> = Vec::new();

    for f in 0..end {
        if let Some(v) = plan.joy.get(&f) {
            cur_joy = *v;
        }
        engine.mem.joy1 = cur_joy; // 与 rust_screenshot.rs:564 同口径（逐帧稀疏注入）
        pending.clear();
        let c = cursor;
        engine.tick_frame_with(|m| {
            if m.apu_log.len() > c {
                pending.extend_from_slice(&m.apu_log[c..]);
            }
        });
        cursor += pending.len();
        for &(addr, v) in &pending {
            apu.write_reg(addr, v);
        }
        writes_total += pending.len() as u64;

        const CYCLES_PER_FRAME: u32 = CPU_HZ / 60;
        for _ in 0..CYCLES_PER_FRAME {
            apu.clock_cpu();
        }
        pcm.extend_from_slice(&apu.take_samples());
    }

    if let Err(e) = write_wav(&out, &pcm) {
        eprintln!("写 WAV 失败: {e}");
        std::process::exit(1);
    }
    let nonzero = pcm.iter().filter(|&&v| v != 0).count();
    eprintln!(
        "[apu_dump] frames={end} pcm={} ({:.2}s) 非零={nonzero} 寄存器写={writes_total} → {}",
        pcm.len(),
        pcm.len() as f32 / SAMPLE_RATE as f32,
        out.display()
    );
}
