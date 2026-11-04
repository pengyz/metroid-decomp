//! TDD 锚·H3 敌槽残差（findings/h3-enemy-slot-residual.md §0/§5）。
//!
//! 【锚】case-collide-probe f800 emu 实测值（analysis/traces/…_f00800_l240.ram.hex
//! 窄域取证档双腿直读）：
//!   Ens.4($0440) x=$BD(189) data05=$1A；Ens.5($0450) x=$5D(93) data05=$5A。
//! 修后 rust 腿（本测）须 **0 差** 复现（±2px X 差=运动相位瞬态、data05 位 6 双槽
//! 互换=+1 相位补偿失配——两修（bit6 去 +1/gate 去 −1）后双腿 0 差）。
//!
//! 运行：`cargo test -p game-core --test h3_enemy_slot_anchor`（真跑 rust_screenshot
//! bin——case-collide-probe 同形输入（310/350 start+440-540 右+560-800 左（含 560 a）））。
use std::fs;
use std::process::Command;

fn unhex(s: &str) -> Vec<u8> {
    (0..s.len() / 2)
        .map(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16).unwrap())
        .collect()
}

/// case-collide-probe 同形调度（cases/case-collide-probe.txt+shots_rust/…sched 同源展开）：
/// 310/311+350/351 start（press_len=2）、440-540 持右、560-561 a+左、562-800 持左。
fn collide_sched() -> String {
    let mut s = String::from("name case-h3anchor\nframes 800\n");
    s.push_str("joy 310 10\njoy 311 10\njoy 350 10\njoy 351 10\n");
    for f in 440..=540usize {
        s.push_str(&format!("joy {f} 01\n"));
    }
    s.push_str("joy 560 82\njoy 561 82\n");
    for f in 562..=800usize {
        s.push_str(&format!("joy {f} 02\n"));
    }
    s.push_str("capture 800 l240\n");
    s
}

/// 【H3 TDD 锚·红→绿】f800 敌槽双腿 0 差：Ens4.x=$BD/Ens5.x=$5D+
/// data05=$1A/$5A（emu 实测相位——bit6=（slot^FC）&1（FC 逻辑读 $10））。
#[test]
fn h3_collide_f800_enemy_slot_anchor() {
    let tmp = std::env::temp_dir().join(format!("dsh_h3_anchor_{}", std::process::id()));
    let _ = fs::remove_dir_all(&tmp);
    fs::create_dir_all(&tmp).expect("mkd");
    let sched_path = tmp.join("sched.txt");
    fs::write(&sched_path, collide_sched()).expect("写调度表");
    let out_dir = tmp.join("out");

    let st = Command::new(env!("CARGO_BIN_EXE_rust_screenshot"))
        .args([
            "--schedule",
            sched_path.to_str().unwrap(),
            "--out",
            out_dir.to_str().unwrap(),
        ])
        .status()
        .expect("跑 rust_screenshot");
    assert!(st.success(), "rust_screenshot 须成功退出");

    let ram_hex = fs::read_to_string(out_dir.join("case-h3anchor_f00800_l240.ram.hex"))
        .expect("读 f800 ram.hex");
    let ram = unhex(ram_hex.trim());
    // emu 实测锚（h3-enemy-slot-residual.md §0）——双腿 0 差
    assert_eq!(ram[0x441], 0xBD, "Ens4.x=$BD(189)——emu 锚（±2px 残差根修后 0 差）");
    assert_eq!(ram[0x451], 0x5D, "Ens5.x=$5D(93)——emu 锚");
    assert_eq!(ram[0x445], 0x1A, "Ens4.data05=$1A——位 6 相位锚（slot4→0）");
    assert_eq!(ram[0x455], 0x5A, "Ens5.data05=$5A——位 6 相位锚（slot5→1）");
    // 空槽位 6 公式面（f800 emu=00 40 00 40——全槽 (slot^$10)&1）
    for (i, want) in [0u8, 1, 0, 1, 0, 1].iter().enumerate() {
        let got = (ram[0x400 + i * 0x10 + 5] >> 6) & 1;
        assert_eq!(got, *want, "槽{i} 位 6=(slot^FC)&1（FC=16——f800 全槽锚）");
    }
}
