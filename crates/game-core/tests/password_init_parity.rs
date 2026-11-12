//! 卡 B G3 出生态对拍（TDD 硬验收）：rust 密码提交→校验→InitializeGame 全径 vs
//! ROM 字节执行铁证（102 组——analysis/password_verify/emu.py 迷你 6502 实跑
//! $8C5E：100 随机 canonical（pwlib encode 面生成，seed=20261110）+JB+NAR）。
//!
//! ROM 侧细节：ClearRAM_33_DF/ClearSamusStats/LoadPasswordData/LoadUniqueItems/
//! LoadTanksAndMissiles 全部真执行；NMIOn $2002 自旋停机容忍（状态面已全载）。

mod common;
use common::password_init::VECTORS;

use game_core::frame::title_bridge::main_title_routine;
use game_core::frame::FrameMemory;
use game_core::ram_map::{
    BANK_SWITCH_PENDING, IN_AREA, KRAID_STATUE_STATUS, MAIN_ROUTINE, MAX_MISSILES,
    MISSILE_COUNT, NAR_PASSWORD, NUMBER_OF_UNIQUE_ITEMS, PASSWORD_BYTE, PASSWORD_CHAR,
    RIDLEY_STATUE_STATUS, SAMUS_AGE, SAMUS_X, SAMUS_Y, TITLE_ROUTINE, UNIQUE_ITEM_HISTORY,
};
use game_core::ram_map::{ENERGY_TANK_COUNT, JUST_IN_BAILEY, SAMUS_GEAR, JOY1_CHANGE};

/// 单组对拍：chars 写入 PasswordChar 面 → 0x1A+START 提交 → 出生态逐字段全等。
fn run_one(chars: &[u8; 24]) -> FrameMemory {
    let mut m = FrameMemory::default();
    for (i, &c) in chars.iter().enumerate() {
        m.write_wram(PASSWORD_CHAR + i as u16, c);
    }
    m.write_ram(TITLE_ROUTINE, 0x1A); // WaitForSTART
    m.write_ram(JOY1_CHANGE, 0x10); // START 按下
    main_title_routine(&mut m); // 首拍=START 消费拍（CheckPassword 链置 pending）
    main_title_routine(&mut m); // 第二拍执行链（emu UnscramblePassword 忙跨延拍）
    m
}

#[test]
fn init_parity_102_vectors() {
    let mut diverged = 0usize;
    for (n, v) in VECTORS.iter().enumerate() {
        let e = &v.expect;
        let m = run_one(&v.chars);
        let mut bad: Vec<String> = Vec::new();
        let mut chk = |name: String, got: u8, want: u8| {
            if got != want {
                bad.push(format!("{name}: got {got:02X} want {want:02X}"));
            }
        };
        chk("gear".into(), m.read_wram(SAMUS_GEAR), e.gear);
        chk("missile".into(), m.read_wram(MISSILE_COUNT), e.missile);
        chk("maxm".into(), m.read_wram(MAX_MISSILES), e.maxm);
        chk("etank".into(), m.read_wram(ENERGY_TANK_COUNT), e.etank);
        chk("kraid".into(), m.read_wram(KRAID_STATUE_STATUS), e.kraid);
        chk("ridley".into(), m.read_wram(RIDLEY_STATUE_STATUS), e.ridley);
        chk("jib".into(), m.read_wram(JUST_IN_BAILEY), e.jib);
        chk("inarea".into(), m.read_ram(IN_AREA), e.inarea);
        chk("nuniq".into(), m.read_wram(NUMBER_OF_UNIQUE_ITEMS), e.nuniq);
        // bank 不对拍：ROM 探针在 NMIOn $2002 自旋停机，BankTable 装载（$931B-9321）
        // 未执行——真值=BankTable ROM 字节直接查表（07:CA30 已逐字节校验，见 JB 锚例）。
        let _ = e.bank;
        chk("y".into(), m.read_ram(SAMUS_Y), e.y);
        chk("x".into(), m.read_ram(SAMUS_X), e.x);
        chk("mainr".into(), m.read_ram(MAIN_ROUTINE), e.mainr);
        chk("nar".into(), m.read_wram(NAR_PASSWORD), e.nar);
        for (i, &h) in e.hist.iter().enumerate() {
            let got = m.read_wram(UNIQUE_ITEM_HISTORY + i as u16);
            if got != h {
                bad.push(format!("hist[{i}]: got {got:02X} want {h:02X}"));
            }
        }
        for (i, &a) in e.age.iter().enumerate() {
            let got = m.read_wram(SAMUS_AGE + i as u16);
            if got != a {
                bad.push(format!("age[{i}]: got {got:02X} want {a:02X}"));
            }
        }
        for (i, &b) in e.pb.iter().enumerate() {
            let got = m.read_wram(PASSWORD_BYTE + i as u16);
            if got != b {
                bad.push(format!("pb[{i}]: got {got:02X} want {b:02X}"));
            }
        }
        if !bad.is_empty() {
            diverged += 1;
            if diverged <= 5 {
                eprintln!("vec[{n}] chars={:02X?}: {}", &v.chars[..6], bad.join("; "));
            }
        }
    }
    assert_eq!(diverged, 0, "{diverged}/{}/组分歧", VECTORS.len());
}
