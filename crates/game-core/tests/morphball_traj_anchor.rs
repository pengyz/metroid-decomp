//! TDD 锚·morphball 轨迹域+敌槽生成/位形（docs/findings/morphball-trajectory-root.md）。
//!
//! 【锚面】case-morphball-probe 双腿逐帧 0 差：
//!   ① 敌 B（$0440）f820-860 (y,x,d05) 逐帧——钻进节律+f846 停机位形（63,200,0x18）
//!      双腿一致（屏上门 F351 修后 emu 离屏双返=y/闪位零写同形）；
//!   ② 轨迹 f1255-1290 七字段（sX.I/sX.F/sY.I/sY.F/Y/X/ScrollX）——顶头反应三写
//!      （E2C6-E2CB）+f1271 落地+敌面击退消除（f1278 IsHit=0）；
//!   ③ f1524 换房窗 MapPosX=1/RoomNumber=255 + 拾取面 gear=0x10/pw0=FF。
//!
//! 运行：`cargo test -p game-core --test morphball_traj_anchor`（真跑 rust_screenshot）。
use std::fs;
use std::process::Command;

/// f820..=860 逐帧 (敌B y,x,data05) 十六进制——emu 实录（.archive/…-20261105-115335）。
const ENEMY_GOLDEN: &str = concat!(
"2BC81A2CC85A2DC81A2EC85A2FC81A30C85A30C81A30C85A31C81A32C85A33C81A34C85A35C81A36C85A36C81A36C85A37C81A38C85A39C8",
"1A3AC85A3BC81A3CC85A3CC81A3CC85A3DC81A3EC85A3FC8183FC8183FC8183FC8183FC8183FC8183FC8183FC8183FC8183FC8183FC8183F",
"C8183FC8183FC8183FC818",
);

/// f1255..=1290 逐帧 (sX.I,sX.F,sY.I,sY.F,Y,X,ScrollX)——emu 实录压缩。
const TRAJ_GOLDEN: &str = concat!(
"FEE0FD6840EC7CFEE0FD803EEB7BFEE0FD983CEA7AFEE0FDB039E979FEE0FDC837E878FEE0FDE035E676FEE0FDF833E575FEE0FE1031E474",
"FEE0FE282FE373FEE0FF202FE272FEE0FF382FE171FEE0FFA82FE070FEE0FFC02FDF6FFEE0FFEC2FDD6DFEE000042FDC6CFEE0001C30DB6B",
"FEE0003430DA6AFEE0004C30D969FEE0006430D868FEE0007C31D767FEE0000031D666FEB0000031D464FE80000031D363FE80000031D161",
"FE80000031D060FE80000031CE5EFE80000031CD5DFE80000031CB5BFE80000031CA5AFE80000031C858FE80000031C757FE80000031C555",
"FE80000031C454FE80000031C252FE80000031C151FE80000031BF4F",
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

/// 【morphball TDD 锚·红→绿】①敌B f820-860 位形 0 差 ②轨迹 f1255-1290 七字段 0 差
/// +f1278 无击（IsHit=0/InvDly=0）③f1524 换房窗+拾取面。
#[test]
fn morphball_traj_window_and_enemy_anchor() {
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
    let enemy_golden = unhex(ENEMY_GOLDEN);
    let traj_golden = unhex(TRAJ_GOLDEN);
    let mut seen_e = 0usize;
    let mut seen_t = 0usize;
    let mut at1524: Option<(u8, u8, u8, u8)> = None;
    let mut at1560: Option<(u8, u8, u8)> = None;
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
        let (zp, iram, wram) = match (field(line, "zp"), field(line, "iram"), field(line, "wram")) {
            (Some(z), Some(i), Some(w)) => (unhex(z), unhex(i), unhex(w)),
            _ => continue,
        };
        // ① 敌B 位形窗（f820-860：y,x,data05）
        // FC 对齐（缝缺行标号伪差免疫——morphball-trajectory-root.md §10）：
        // emu/rust 各自的 f 标号在采样缝后可偏 1，FC 为共同真值。
        if (820..=860).contains(&f) {
            let k = (f - 820) * 3;
            let got = [iram[0x340], iram[0x341], iram[0x345]];
            assert_eq!(&got[..], &enemy_golden[k..k + 3], "f{f} 敌B 位形双腿 0 差");
            seen_e += 1;
        }
        // ② 轨迹窗（f1255-1290 七字段）
        if (1255..=1290).contains(&f) {
            let k = (f - 1255) * 7;
            let got = [iram[0x209], iram[0x213], iram[0x208], iram[0x212],
                       iram[0x20D], iram[0x20E], zp[0xFD]];
            assert_eq!(&got[..], &traj_golden[k..k + 7], "f{f} 轨迹七字段双腿 0 差");
            seen_t += 1;
            // f1278 敌撞击退消除锚
            if (1278..=1290).contains(&f) {
                assert_eq!(iram[0x20A], 0, "f{f} IsHit=0（敌撞击退双腿一致=无击）");
                assert_eq!(zp[0x70], 0, "f{f} InvDly=0（无击退 i 帧）");
            }
        }
        // ③ 换房窗+拾取面+冻结面
        if f == 1524 {
            at1524 = Some((zp[0x50], zp[0x5A], wram[0x878], iram[0x648]));
        }
        if f == 1560 {
            at1560 = Some((iram[0x20E], iram[0x20D], zp[0x1E]));
        }
    }
    assert_eq!(seen_e, 41, "敌B 窗金值帧全覆盖（f820-860）");
    assert_eq!(seen_t, 36, "轨迹窗金值帧全覆盖（f1255-1290）");
    let (map_x, room, gear, pw0) = at1524.expect("f1524 行存在");
    assert_eq!((map_x, room), (1, 255), "f1524 换房窗双腿一致（MapPosX/RoomNumber）");
    assert_eq!(gear, 0x10, "拾取面 gear=gr_MARUMARI（1<<type(4)）");
    assert_eq!(pw0, 0xFF, "拾取面 pw0=FF（物品消耗）");
    // 【缝会计卡·冻结面锚（2026-11-05）】f1521 拾取⇒GameEngine 延时旗（$0109）⇒
    // SetTimer 三写（Timer3←$18/NextRoutine←3/MainRoutine←WaitTimer(9)）⇒萨姆斯
    // 冻结 (98,135)（能力动画 240 帧窗）——emu f1521+ MainRoutine=9 恒驻实录。
    let (fx, fy, fmr) = at1560.expect("f1560 行存在");
    assert_eq!((fx, fy), (98, 135), "f1560 萨姆斯冻结位形 (98,135)");
    assert_eq!(fmr, 9, "f1560 MainRoutine=_id_WaitTimer（冻结窗内）");
}
