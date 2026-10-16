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
    // 键集声明（严重-7，task-22 slot③）：`w` 对象标签集=manifest watch 声明集（无隐藏标签）；
    // 编码唯一源=memmap::frame_face（严重-6）。声明标签无对应编码面 ⇒ fail-loud（拒半成品）。
    let watch_labels: Vec<&str> = case.watch.iter().map(|w| w.label.as_str()).collect();
    let all_snap = engine.watch_snapshot();
    for label in &watch_labels {
        if !all_snap.iter().any(|(l, _)| l == label) {
            eprintln!("manifest watch 标签 {label} 无对应编码面（memmap::frame_face::WATCH_FACES 未定义）——fail-loud");
            return ExitCode::from(2);
        }
    }
    let mut out = String::new();
    let mut div_seen = 0usize;
    let mut div_events: Vec<(u64, u16)> = Vec::new();
    for f in 1..=frames {
        // task-24 ②：spin 出口采样（tick_frame_with 缝=NMI 尾写 1 后/帧工作前——
        // oracle $C0D3 同点）；unported! 引爆 ⇒ panic（fail-loud，dump 不落盘）。
        let mut row = String::new();
        engine.tick_frame_with(|mem| {
            let snap = mem.watch_snapshot();
            let w: Vec<String> = watch_labels
                .iter()
                .map(|label| {
                    let (_, hex) = snap.iter().find(|(l, _)| l == label).expect("标签已预检");
                    format!("\"{label}\":\"{hex}\"")
                })
                .collect();
            row = format!("{{\"f\":{f},\"w\":{{{}}}}}\n", w.join(","));
        });
        out.push_str(&row);
        // task-29 (ii)：占位臂缺口记账逐 tick 增量捕获（路线图事件面）
        let now_div = game_core::frame::FrameEngine::divergences();
        if now_div.len() > div_seen {
            for a in &now_div[div_seen..] {
                div_events.push((f, *a));
            }
            div_seen = now_div.len();
        }
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
    // task-29 (ii)：缺口记账 sidecar（gap 地图+事件——diff_traces 记账排除面源）
    let gap_raw = game_core::frame::FrameEngine::divergences();
    // 唯一缺口地图（roadmap）：址→次数；事件=逐 tick 时间线
    let mut gap_counts: std::collections::BTreeMap<u16, usize> = std::collections::BTreeMap::new();
    for a in &gap_raw {
        *gap_counts.entry(*a).or_default() += 1;
    }
    let gap_map = gap_counts.iter()
        .map(|(a, n)| format!("{{\"addr\":{a},\"n\":{n}}}"))
        .collect::<Vec<_>>().join(", ");
    let dgap = dump_dir.join(format!("{scenario}.divergences.json"));
    let djson = format!(
        "{{\"gap_map\": [{}], \"events\": [{}], \"count\": {}, \"unique\": {}}}",
        gap_map,
        div_events.iter().map(|(f, a)| format!("{{\"tick\":{f},\"addr\":{a}}}"))
            .collect::<Vec<_>>().join(", "),
        gap_raw.len(), gap_counts.len());
    if let Err(e) = std::fs::write(&dgap, djson) {
        eprintln!("写缺口记账 sidecar 失败: {e}");
        return ExitCode::from(1);
    }
    println!("L3 game-core 腿: {frames} 帧 watch 向量 → {}（缺口记账 {} 事件/{} 唯一址 → {}）",
        path.display(), gap_raw.len(), gap_counts.len(), dgap.display());
    ExitCode::from(0)
}
