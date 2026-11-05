//! TDD 锚·morphball 轨迹域根修（docs/findings/morphball-trajectory-root.md）。
//!
//! 【锚】case-morphball-probe 轨迹窗 f1255-1278 七字段（sX.I/sX.F/sY.I/sY.F/
//! SamusY/SamusX/ScrollX）逐帧 0 差——含顶头反应三写（f1264 (FE,40)→(FF,20)、
//! f1266 (FF,50)→(FF,A8)、f1268 (FF,D8)→(FF,EC)——E2C6-E2CB sec/ror/ror 逐位）
//! 与 f1271 落地时点/f1272+ max 翻横漂相位。
//!
//! 【域界（findings §4/§6）】f1279+ 再分叉=敌槽 B 自生成即异轨（emu 恒 (63,200) vs
//! rust 巡逻——goal-轮134 敌面已登记缺口）⇒f1278 敌撞击退单腿触发——非 Samus 轨迹域，
//! f1524 换房锚在敌面收口前不可达（如实登记不伪锚）。
//!
//! 运行：`cargo test -p game-core --test morphball_traj_anchor`（真跑 rust_screenshot）。
use std::fs;
use std::process::Command;

/// f1255..=1278 逐帧 (sX.I,sX.F,sY.I,sY.F,Y,X,ScrollX) 十六进制——emu
/// case-morphball-probe.frames.jsonl 实录（.archive/…-20261105-115335 全量档自锚）。
const TRAJ_GOLDEN: &str = concat!(
"FEE0FD6840EC7CFEE0FD803EEB7BFEE0FD983CEA7AFEE0FDB039E979FEE0FDC837E878FEE0FDE035E676FEE0FDF833E575FEE0FE1031E474",
"FEE0FE282FE373FEE0FF202FE272FEE0FF382FE171FEE0FFA82FE070FEE0FFC02FDF6FFEE0FFEC2FDD6DFEE000042FDC6CFEE0001C30DB6B",
"FEE0003430DA6AFEE0004C30D969FEE0006430D868FEE0007C31D767FEE0000031D666FEB0000031D464FE80000031D363FE80000031D161",
);

fn unhex(s: &str) -> Vec<u8> {
    (0..s.len() / 2)
        .map(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16).unwrap())
        .collect()
}

/// case-morphball-probe 同形调度（shots_rust/case-morphball-probe.sched 权威展开逐段
/// 复刻——边界点 1420=01/1502-1558=81/1570=01 实核）。
fn morphball_sched() -> String {
    let mut s = String::from("name case-mbanchor\nframes 1700\n");
    for f in [310usize, 311, 350, 351] {
        s.push_str(&format!("joy {f} 10\n"));
    }
    let seg: &[(usize, usize, &str)] = &[
        (450, 1129, "02"), (1130, 1185, "82"), (1186, 1239, "02"),
        (1240, 1295, "82"), (1296, 1419, "02"), (1420, 1501, "01"),
        (1502, 1558, "81"), (1559, 1570, "01"),
    ];
    for &(a, b, bits) in seg {
        for f in a..=b {
            s.push_str(&format!("joy {f} {bits}\n"));
        }
    }
    s
}

fn field<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    let pat = format!("\"{key}\":\"");
    let i = line.find(&pat)? + pat.len();
    let j = line[i..].find('"')? + i;
    Some(&line[i..j])
}

/// 【morphball TDD 锚·红→绿】f1255-1278 七字段逐帧 0 差（顶头反应三写逐位+连锁面）。
/// f1279+/f1524=敌面击退连锁残差（findings §4 域界——不伪锚）。
#[test]
fn morphball_traj_window_anchor() {
    let tmp = std::env::temp_dir().join(format!("dsh_mb_anchor_{}", std::process::id()));
    let _ = fs::remove_dir_all(&tmp);
    fs::create_dir_all(&tmp).expect("mkd");
    let sched_path = tmp.join("sched.txt");
    fs::write(&sched_path, morphball_sched()).expect("写调度表");
    let out_dir = tmp.join("out");
    let state_path = tmp.join("state.jsonl");

    let st = Command::new(env!("CARGO_BIN_EXE_rust_screenshot"))
        .args([
            "--schedule",
            sched_path.to_str().unwrap(),
            "--out",
            out_dir.to_str().unwrap(),
            "--state-out",
            state_path.to_str().unwrap(),
        ])
        .status()
        .expect("跑 rust_screenshot");
    assert!(st.success(), "rust_screenshot 须成功退出");

    let rows = fs::read_to_string(&state_path).expect("读 state 行");
    let golden = unhex(TRAJ_GOLDEN);
    let mut seen = 0usize;
    for line in rows.lines() {
        let f: usize = match line
            .split("\"f\":")
            .nth(1)
            .and_then(|s| s.split(',').next())
            .and_then(|s| s.parse().ok())
        {
            Some(v) => v,
            None => continue,
        };
        let (zp, iram) = match (field(line, "zp"), field(line, "iram")) {
            (Some(z), Some(i)) => (unhex(z), unhex(i)),
            _ => continue,
        };
        if (1255..=1278).contains(&f) {
            let k = (f - 1255) * 7;
            let got = [iram[0x209], iram[0x213], iram[0x208], iram[0x212],
                       iram[0x20D], iram[0x20E], zp[0xFD]];
            let want = &golden[k..k + 7];
            assert_eq!(&got[..], want, "f{f} 轨迹七字段双腿 0 差");
            seen += 1;
        }
    }
    assert_eq!(seen, 24, "轨迹窗金值帧全覆盖（f1255-1278）");
}
