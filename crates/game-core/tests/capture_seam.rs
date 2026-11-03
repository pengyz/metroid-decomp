//! TDD 双锚·capture 相位（docs/findings/walk-fc-capture-seam.md §2c/§6a）。
//!
//! 【锚】capture(N) 的 ram.hex 面 ≡ state-row(N)（FC($2D)/ScrollX($FD)/SamusX($30E)）：
//! emu endFrame(N) 读活体=work(N-1) 收口（$1A=00 判别位）——≡ spin 出口缝 N 的
//! 三面（NMI 对三面透明）。旧径（capture 在 tick(N) 收口）读到 state-row(N+1) 面
//! （walkfcprobe 实证：rust cap f820=FC24/F4/64=行 f821）。
//!
//! 【绿态（2026-11-03，修径 a 落地）】capture 产物移 tick(N-1) 双相位
//! （png+ppo.hex=seam(N-1) 快照、ram.hex=tick(N-1) 收口活体）后转绿。
//! 红态实录留档：capture(450)=(FC180,0,128) vs 行 450=(FC179,0,128)（+1 面）。
//!
//! 运行：`cargo test -p game-core --test capture_seam`（真跑 rust_screenshot bin——
//! 短 walk 表（入戏+持左 20 帧）；capture(N)≠state-row(N+1) 判别力由持左走动保证）。
use std::collections::HashMap;
use std::fs;
use std::process::Command;

fn unhex(s: &str) -> Vec<u8> {
    (0..s.len() / 2)
        .map(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16).unwrap())
        .collect()
}

/// state 行（rust_screenshot::state_row 同形）→ {frame: (FC, ScrollX, SamusX)}。
/// zp 面=ram[0x00-0xFF]；iram 面=ram[0x100-0x7FF]（SamusX=$30E ⇒ iram[0x20E]）。
fn parse_rows(text: &str) -> HashMap<usize, (u8, u8, u8)> {
    let mut m = HashMap::new();
    for line in text.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let fpos = line.find("\"f\":").expect("行须含 f") + 4;
        let rest = &line[fpos..];
        let f: usize = rest[..rest.find(',').expect("f 后须有 ,")].parse().expect("f 数值");
        let grab = |key: &str| -> String {
            let pat = format!("\"{key}\":\"");
            let s = line.split(&pat).nth(1).expect("行须含面");
            s.split('"').next().unwrap().to_string()
        };
        let zp = unhex(&grab("zp"));
        let iram = unhex(&grab("iram"));
        m.insert(f, (zp[0x2D], zp[0xFD], iram[0x30E - 0x100]));
    }
    m
}

/// 【TDD 锚·修径 a 绿】capture(N) 的 ram.hex 面（FC/ScrollX/SamusX）须≡
/// state-row(N)。红态实录（2026-11-03）：capture(450)=(FC180,0,128) vs 行 450
/// =(FC179,0,128)——旧径读 tick(N) 收口=state-row(N+1) 面（+1）。
/// 修径 a（产物移 tick(N-1) 双相位：png+ppo.hex=seam(N-1) 快照、
/// ram.hex=tick(N-1) 收口活体）落地后转绿；若回退旧径本测即红。
#[test]
fn capture_n_ram_face_matches_state_row_n() {
    // 短 walk 表（leftwalk 简形）：310/350 start 入戏 + 440-459 持左——逐帧滚动走动
    // ⇒ state-row(N)≠state-row(N+1)（红判别力锚）。
    let tmp = std::env::temp_dir().join(format!("dsh_cap_seam_{}", std::process::id()));
    let _ = fs::remove_dir_all(&tmp);
    fs::create_dir_all(&tmp).expect("mkd");
    let mut sched = String::from("name case-capseam\nframes 460\n");
    sched.push_str("joy 310 0x10\njoy 350 0x10\n");
    for f in 440..=459usize {
        sched.push_str(&format!("joy {f} 0x02\n"));
    }
    sched.push_str("capture 450 t1\ncapture 455 t2\n");
    let sched_path = tmp.join("sched.txt");
    fs::write(&sched_path, &sched).expect("写调度表");
    let rows_path = tmp.join("rows.jsonl");
    let out_dir = tmp.join("out");

    let st = Command::new(env!("CARGO_BIN_EXE_rust_screenshot"))
        .args([
            "--schedule",
            sched_path.to_str().unwrap(),
            "--out",
            out_dir.to_str().unwrap(),
            "--state-out",
            rows_path.to_str().unwrap(),
        ])
        .status()
        .expect("跑 rust_screenshot");
    assert!(st.success(), "rust_screenshot 须成功退出");

    let rows = parse_rows(&fs::read_to_string(&rows_path).expect("读状态行"));
    // 判别力前置：走动窗相邻行三面必异（否则红测不红）。
    let (a, b) = (rows.get(&450).expect("行 450"), rows.get(&451).expect("行 451"));
    assert_ne!(a, b, "走动窗 state-row(450)≠state-row(451)（红判别力）");

    for (frame, tag) in [(450usize, "t1"), (455usize, "t2")] {
        let ram_hex = fs::read_to_string(out_dir.join(format!(
            "case-capseam_f{frame:05}_{tag}.ram.hex"
        )))
        .expect("读 ram.hex");
        let ram = unhex(ram_hex.trim());
        let got = (ram[0x2D], ram[0xFD], ram[0x30E]);
        let want = rows[&frame];
        assert_eq!(
            got, want,
            "capture({frame}) ram 面≡state-row({frame})（FC/ScrollX/SamusX——§6a 锚）"
        );
    }
}
