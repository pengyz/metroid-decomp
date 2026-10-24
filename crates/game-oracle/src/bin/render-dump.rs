//! render-dump —— L4 game-core 渲染腿帧导出（S6-10：analysis/render-shots/<金面名>.png）。
//! 用法: render-dump --scenario <name> [--manifest oracle/traces/l3_manifest.json]
//!
//! 同构 l3-dump（manifest case+输入回放+spin-exit 缝采样）——渲染时机=缝采样后帧完整态
//! （spec s6-9 §1 相位契约）。产出=金面同名 PNG（<场景>_f<帧:05d>_<tag>.png）——
//! diff_visual 第二腿对拍面。

use game_core::frame::FrameEngine;
use game_oracle::png_encode::encode_rgb;

fn main() {
    let mut scenario = String::new();
    let mut manifest_path = "oracle/traces/l3_manifest.json".to_string();
    let args: Vec<String> = std::env::args().collect();
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--scenario" => { scenario = args.get(i + 1).cloned().unwrap_or_default(); i += 2; }
            "--manifest" => { manifest_path = args.get(i + 1).cloned().unwrap_or(manifest_path); i += 2; }
            _ => { i += 1; }
        }
    }
    if scenario.is_empty() {
        eprintln!("用法: render-dump --scenario <name>");
        std::process::exit(2);
    }
    let text = std::fs::read_to_string(&manifest_path).expect("manifest 读取");
    let manifest: serde_json::Value = serde_json::from_str(&text).expect("manifest 解析");
    let frames = manifest["cases"].as_array().and_then(|cs| {
        cs.iter().find(|c| c["scenario"] == scenario.as_str())?.get("frames")?.as_u64()
    }).expect("manifest case.frames 缺失");
    // 金面帧点（oracle/screens/<场景>_f<帧:05d>_<tag>.png 的帧号集）
    // 金面同名契约：(帧号, 金面全名) 对——产出名=金面名（第二腿同名对拍）
    let mut gold_points: Vec<(u64, String)> = Vec::new();
    if let Ok(rd) = std::fs::read_dir("oracle/screens") {
        for e in rd.flatten() {
            let n = e.file_name().to_string_lossy().to_string();
            if let Some(rest) = n.strip_prefix(&format!("{scenario}_f")) {
                if let Some(fv) = rest.split('_').next() {
                    if let Ok(v) = fv.parse::<u64>() { gold_points.push((v, n)); }
                }
            }
        }
    }
    gold_points.sort_by_key(|g| g.0);
    // 输入回放（同 l3-dump DSL）
    fn btn_bit(name: &str) -> u8 {
        match name.to_ascii_lowercase().as_str() {
            "a" => 0x80, "b" => 0x40, "select" => 0x20, "start" => 0x10,
            "up" => 0x08, "down" => 0x04, "left" => 0x02, "right" => 0x01, _ => 0,
        }
    }
    let mut press_len: u32 = 2;
    let mut inputs: std::collections::HashMap<u64, u8> = std::collections::HashMap::new();
    let scen_path = std::path::Path::new("oracle/scenarios").join(format!("{scenario}.txt"));
    if let Ok(text) = std::fs::read_to_string(&scen_path) {
        for line in text.lines() {
            let line = line.trim();
            if let Some(rest) = line.strip_prefix("press_len ") {
                if let Ok(v) = rest.trim().parse::<u32>() { press_len = v; }
            } else if let Some(rest) = line.strip_prefix("press ") {
                let parts: Vec<&str> = rest.split_whitespace().collect();
                if parts.len() >= 2 {
                    if let (Ok(f), Some(bit)) = (parts[0].parse::<u64>(), Some(btn_bit(parts[1]))) {
                        for k in 0..press_len { *inputs.entry(f + k as u64).or_insert(0) |= bit; }
                    }
                }
            } else if let Some(rest) = line.strip_prefix("hold ") {
                let parts: Vec<&str> = rest.split_whitespace().collect();
                if parts.len() >= 2 {
                    if let Some((a, b)) = parts[0].split_once('-') {
                        if let (Ok(a), Ok(b), Some(bit)) = (a.parse::<u64>(), b.parse::<u64>(), Some(btn_bit(parts[1]))) {
                            for f in a..=b { *inputs.entry(f).or_insert(0) |= bit; }
                        }
                    }
                }
            }
        }
    }
    // 帧循环（spin-exit 缝采样——同 l3-dump）
    let mut engine = FrameEngine::new();
    std::fs::create_dir_all("analysis/render-shots").ok();
    let mut written = 0usize;
    for f in 1..=frames {
        let fi = f as u64;
        engine.mem.joy1 = inputs.get(&fi).copied().unwrap_or(0);
        engine.tick_frame();
        for (gf, gname) in &gold_points {
            // 采样相位契约（**r22 更正**：tick≡帧——r21 裁定补 2 spin 等待帧后，
            // 原 F-2 系缺帧镜像（eae3212b 复核）；金面帧 F 捕于 tick F）。
            if *gf == fi {
                let pal = &engine.mem.render.palette;
                let fb = game_core::render::render_frame(
                    &engine.mem.render, &bg16(pal), &spr16(pal), &engine.mem.ram[0x200..0x300]);
                let png = encode_rgb(256, 240, &fb);
                std::fs::write(std::path::Path::new("analysis/render-shots").join(gname), &png).expect("写 PNG");
                written += 1;
            }
        }
    }
    println!("render-dump: {scenario} {frames} 帧——金面点捕 {written} 张 → analysis/render-shots/");
}

/// 32B 调色板→16B BG 面（组 0-3 各 4 色——$3F00 布局镜像 $3F10 等价）。
fn bg16(pal: &[u8; 32]) -> [u8; 16] {
    let mut out = [0u8; 16];
    out.copy_from_slice(&pal[0..16]);
    out
}

/// 32B 调色板→16B 精灵面（$3F10-$3F1F——组 0 色 0 透明位）。
fn spr16(pal: &[u8; 32]) -> [u8; 16] {
    let mut out = [0u8; 16];
    out.copy_from_slice(&pal[16..32]);
    out
}
