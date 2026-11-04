//! TDD 锚·卡 E animI 总根（docs/findings/animi-divergence-collide-f800.md §10-§11）。
//!
//! 【锚】collide f800 emu 逐帧双腿 0 差面：
//!   ① 淡入轨迹（Timer3 $2C/status $300）逐帧金值=f350-800（emu frames.jsonl 实录，
//!      scenario watch 002C/0300-030F 可复读——cases/case-collide-probe.txt 已钉）；
//!   ② f800 走表收敛态=animFrame=0x10/animDelay=0x02/animResetIndex=0x0C/
//!      animIndex=0x0D（JUMP 稳态——卡 A 靶）；f798-802 切换窗恒同。
//! 总根（卡 E 定谳）：D1=CF88 pressed 尾 SetSamusAnim 全形缺失（旧形 reset-only）+
//! D2=DE4A 站幻影走表（spawn.rs:172/mainloop.rs skree 弹）——修后四层链（卡 A-D）自动消。
//!
//! 运行：`cargo test -p game-core --test card_e_animi_anchor`（真跑 rust_screenshot bin）。
use std::fs;
use std::process::Command;

/// 淡入轨迹金值（f350..=800 逐帧 (t3,status) 十六进制对）——emu frames.jsonl 实录压缩。
/// 结构自证：t3=44 起（f360 SamusInit）→每 10 帧减 1→步进 st 0x14→15（t3=1E）→16（14）
/// →17（0B）→18（04）→FF（t3=0 收尾）→01。与 SamusFadeInTimeTbl CA13=$1E,$14,$0B,$04 逐点合。
const FADE_GOLDEN: &str = concat!(
"12001200120012001200120012001200120012002C142C142C142C142C142C142B142B142B142B142B142B142B142B14",
"2B142B142A142A142A142A142A142A142A142A142A142A14291429142914291429142914291429142914291428142814",
"281428142814281428142814281428142714271427142714271427142714271427142714261426142614261426142614",
"261426142614261425142514251425142514251425142514251425142414241424142414241424142414241424142414",
"231423142314231423142314231423142314231422142214221422142214221422142214221422142114211421142114",
"21142114211421142114211420142014201420142014201420142014201420141F141F141F141F141F141F141F141F14",
"1F141F141E151E151E151E151E151E151E151E151E151E151D151D151D151D151D151D151D151D151D151D151C151C15",
"1C151C151C151C151C151C151C151C151B151B151B151B151B151B151B151B151B151B151A151A151A151A151A151A15",
"1A151A151A151A1519151915191519151915191519151915191519151815181518151815181518151815181518151815",
"171517151715171517151715171517151715171516151615161516151615161516151615161516151515151515151515",
"151515151515151515151515141614161416141614161416141614161416141613161316131613161316131613161316",
"131613161216121612161216121612161216121612161216111611161116111611161116111611161116111610161016",
"101610161016101610161016101610160F160F160F160F160F160F160F160F160F160F160E160E160E160E160E160E16",
"0E160E160E160E160D160D160D160D160D160D160D160D160D160D160C160C160C160C160C160C160C160C160C160C16",
"0B170B170B170B170B170B170B170B170B170B170A170A170A170A170A170A170A170A170A170A170917091709170917",
"091709170917091709170917081708170817081708170817081708170817081707170717071707170717071707170717",
"071707170617061706170617061706170617061706170617051705170517051705170517051705170517051704180418",
"041804180418041804180418041804180318031803180318031803180318031803180318021802180218021802180218",
"0218021802180218011801180118011801180118011801180118011800FF0001000100010001",
);

fn unhex(s: &str) -> Vec<u8> {
    (0..s.len() / 2)
        .map(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16).unwrap())
        .collect()
}

/// case-collide-probe 同形调度（cases/case-collide-probe.txt 展开同源）：
/// 310/311+350/351 start（press_len=2）、440-540 持右、560-561 a+左、562-800 持左。
fn collide_sched() -> String {
    let mut s = String::from("name case-cardEanchor\nframes 800\n");
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

fn field<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    let pat = format!("\"{key}\":\"");
    let i = line.find(&pat)? + pat.len();
    let j = line[i..].find('"')? + i;
    Some(&line[i..j])
}

/// 【卡 E TDD 锚·红→绿】①淡入轨迹 f350-800 逐帧双腿 0 差 ②f800 animIndex=0x0D
/// 收敛（四层链总根修后自动消——卡 A 靶）+f798-802 切换窗恒 JUMP 稳态。
#[test]
fn card_e_fade_trajectory_and_f800_animindex_anchor() {
    let tmp = std::env::temp_dir().join(format!("dsh_card_e_{}", std::process::id()));
    let _ = fs::remove_dir_all(&tmp);
    fs::create_dir_all(&tmp).expect("mkd");
    let sched_path = tmp.join("sched.txt");
    fs::write(&sched_path, collide_sched()).expect("写调度表");
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
    let golden = unhex(FADE_GOLDEN);
    let mut seen = 0usize;
    let mut present: Vec<usize> = Vec::new();
    let mut f800: Option<(u8, u8, u8, u8)> = None;
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
        // ① 淡入轨迹逐帧（f350..=800）：(t3=$2C, status=$300) 对齐金值
        if (350..=800).contains(&f) {
            let k = (f - 350) * 2;
            assert_eq!(zp[0x2C], golden[k], "f{f} Timer3 双腿 0 差");
            assert_eq!(iram[0x200], golden[k + 1], "f{f} status 双腿 0 差");
            seen += 1;
            present.push(f);
        }
        // ② f800 走表收敛态 + 切换窗（f798-802）恒 JUMP 稳态
        if (798..=802).contains(&f) {
            assert_eq!(iram[0x203], 0x10, "f{f} animFrame=JUMP 帧");
            assert_eq!(iram[0x205], 0x0C, "f{f} animResetIndex=JUMP");
            assert_eq!(iram[0x206], 0x0D, "f{f} animIndex=JUMP 稳态（卡 A 靶）");
        }
        if f == 800 {
            f800 = Some((iram[0x203], iram[0x204], iram[0x205], iram[0x206]));
        }
    }
    // 缝缺行=引擎 spin 出口采样缝不覆盖的标题迁移帧（352-358 七帧）——集合钉死
    // （有行者逐帧 0 差 + 缺行集恒定 ⇒ 逐帧双腿 0 差成立）。
    let missing: Vec<usize> = (350..=800).filter(|f| !present.contains(f)).collect();
    assert_eq!(missing, (352..=358).collect::<Vec<_>>(),
               "缝缺行集=标题迁移帧 352-358（恒定）");
    assert_eq!(seen, 451 - 7, "淡入轨迹金值帧覆盖（f350-800）");
    let (af, ad, ar, ai) = f800.expect("f800 行存在");
    assert_eq!((af, ad, ar, ai), (0x10, 0x02, 0x0C, 0x0D),
               "f800 anim 四件=emu 锚（10/02/0C/0D——卡 A animIndex 0 差）");
}
