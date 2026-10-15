//! l3-dump —— L3 game-core 腿帧导出（S5；analysis/dump/<scenario>.frames.jsonl）。
//!
//! 用法: l3-dump --scenario <name> [--manifest oracle/traces/l3_manifest.json]
//! 语义：FrameEngine 逐帧推进 → watch 状态向量（manifest watch 基址区间）逐帧写盘，
//! 与 oracle 帧 trace 同形（{"f":N,"w":{"zp":…,"iram":…}}）；tools/diff_traces.py 裁定双侧。
//! fail-loud：任一 unported! 引爆 ⇒ 非零退出且 **dump 不落盘**（禁半成品绿面）；
//! 帧数须与 manifest.frames 一致（防缩水）。

use game_core::frame::FrameEngine;
use serde::Deserialize;
use std::process::ExitCode;

#[derive(Deserialize)]
struct Manifest {
    cases: Vec<Case>,
}

#[derive(Deserialize)]
struct Case {
    scenario: String,
    #[serde(default)]
    frames: Option<u64>,
    #[serde(default)]
    watch: Vec<Watch>,
}

#[derive(Deserialize)]
struct Watch {
    label: String,
    #[allow(dead_code)]
    base: String,
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    let mut scenario = String::new();
    let mut manifest_path = String::from("oracle/traces/l3_manifest.json");
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--scenario" => { scenario = args.get(i + 1).cloned().unwrap_or_default(); i += 2; }
            "--manifest" => { manifest_path = args.get(i + 1).cloned().unwrap_or_default(); i += 2; }
            other => { eprintln!("未知参数: {other}"); return ExitCode::from(2); }
        }
    }
    if scenario.is_empty() {
        eprintln!("用法: l3-dump --scenario <name> [--manifest <path>]");
        return ExitCode::from(2);
    }
    let text = match std::fs::read_to_string(&manifest_path) {
        Ok(t) => t,
        Err(e) => { eprintln!("读 manifest 失败: {e}"); return ExitCode::from(2); }
    };
    let manifest: Manifest = match serde_json::from_str(&text) {
        Ok(m) => m,
        Err(e) => { eprintln!("manifest 解析失败: {e}"); return ExitCode::from(2); }
    };
    let case = match manifest.cases.iter().find(|c| c.scenario == scenario) {
        Some(c) => c,
        None => { eprintln!("manifest 无场景 {scenario}"); return ExitCode::from(2); }
    };
    let frames = match case.frames {
        Some(f) => f,
        None => { eprintln!("manifest case.frames 缺失（防缩水——给了就必严）"); return ExitCode::from(2); }
    };
    if case.watch.is_empty() {
        eprintln!("manifest watch 为空（空跑不得绿）");
        return ExitCode::from(2);
    }

    // stale fail-open 防御（verifier v4 路由①/r11）：run 起点删旧 dump——旧件不删会被
    // diff_traces 当有效腿（fail-open）；本 bin 引爆即不落盘+落盘走临时名 rename。
    let dump_dir = std::path::Path::new("analysis/dump");
    let path = dump_dir.join(format!("{scenario}.frames.jsonl"));
    if path.exists() {
        if let Err(e) = std::fs::remove_file(&path) {
            eprintln!("清理旧 dump 失败（fail-loud，拒带旧腿起跑）: {e}");
            return ExitCode::from(1);
        }
        println!("[stale-clean] 旧 dump 已清: {}", path.display());
    }

    let mut engine = FrameEngine::new();
    let mut out = String::new();
    for f in 1..=frames {
        engine.tick_frame(); // unported! 引爆 ⇒ panic（fail-loud，dump 不落盘）
        let snap = engine.watch_snapshot();
        let w: Vec<String> = snap
            .iter()
            .map(|(label, hex)| format!("\"{label}\":\"{hex}\""))
            .collect();
        out.push_str(&format!("{{\"f\":{f},\"w\":{{{}}}}}\n", w.join(",")));
    }
    if let Err(e) = std::fs::create_dir_all(dump_dir) {
        eprintln!("建 dump 目录失败: {e}");
        return ExitCode::from(1);
    }
    let tmp = dump_dir.join(format!("{scenario}.frames.jsonl.tmp"));
    if let Err(e) = std::fs::write(&tmp, &out) {
        eprintln!("写临时 dump 失败: {e}");
        return ExitCode::from(1);
    }
    if let Err(e) = std::fs::rename(&tmp, &path) {
        eprintln!("dump 落盘失败: {e}");
        return ExitCode::from(1);
    }
    println!("L3 game-core 腿: {frames} 帧 watch 向量 → {}", path.display());
    ExitCode::from(0)
}
