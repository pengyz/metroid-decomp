//! L2 差分裁定器单元测试（RED 先行：todo!() 桩期应全部失败）。
//!
//! 合成 PRG 注入真实指令字节（来源 reference/seed，A 级），不依赖 rom/ 文件。

use game_core::L2In;
use game_oracle::cpu6502::Cpu;
use game_oracle::run_subroutine;

fn prg_with(bank7_off: usize, bytes: &[u8]) -> Vec<u8> {
    let mut prg = vec![0u8; 0x20000];
    prg[0x1C000 + bank7_off..0x1C000 + bank7_off + bytes.len()].copy_from_slice(bytes);
    prg
}

fn input(a: u8, x: u8, y: u8) -> L2In {
    L2In { a, x, y, ..Default::default() }
}

#[test]
fn cpu_lda_imm_sets_a_and_flags() {
    // bank7:$C000: A9 01  (LDA #$01)
    let prg = prg_with(0x0000, &[0xA9, 0x01]);
    let mut cpu = Cpu::new(&prg, 7, &input(0xFF, 0, 0));
    cpu.set_pc(0xC000);
    cpu.step().expect("step");
    assert_eq!(cpu.a, 0x01);
}

#[test]
fn cpu_sta_zp_records_write() {
    // $C000: A9 7F 85 10  (LDA #$7F; STA $10)
    let prg = prg_with(0x0000, &[0xA9, 0x7F, 0x85, 0x10]);
    let mut cpu = Cpu::new(&prg, 7, &input(0, 0, 0));
    cpu.set_pc(0xC000);
    cpu.step().unwrap();
    cpu.step().unwrap();
    assert_eq!(cpu.take_writes(), vec![(0x0010, 0x7F)]);
}

#[test]
fn cpu_jsr_rts_roundtrip() {
    // $C000: 20 10 C0 (JSR $C010)；$C010: 60 (RTS)
    let mut prg = vec![0u8; 0x20000];
    prg[0x1C000..0x1C003].copy_from_slice(&[0x20, 0x10, 0xC0]);
    prg[0x1C010] = 0x60;
    let mut cpu = Cpu::new(&prg, 7, &input(0, 0, 0));
    cpu.set_pc(0xC000);
    cpu.step().unwrap(); // JSR
    assert_eq!(cpu.pc, 0xC010);
    cpu.step().unwrap(); // RTS
    assert_eq!(cpu.pc, 0xC003);
}

#[test]
fn l2_sub_c16d_golden_from_rom_bytes() {
    // bank07:$C16D: A9 01 85 01 A9 FF 85 00（reference/seed，止于 $C175）
    let prg = prg_with(0x016D, &[0xA9, 0x01, 0x85, 0x01, 0xA9, 0xFF, 0x85, 0x00]);
    let out = run_subroutine(&prg, 0xC16D, 7, &input(3, 5, 7), &[0xC175]).expect("run");
    assert_eq!((out.a, out.x, out.y), (0xFF, 5, 7));
    let mut w = out.writes.clone();
    w.sort();
    assert_eq!(w, vec![(0x0000, 0xFF), (0x0001, 0x01)]);
}

#[test]
fn l2_sub_c376_absx_indexed_write() {
    // bank07:$C376: A9 00 9D A1 07（STA $07A1,X 止于 $C37B，PLA 尾部不入面）
    let prg = prg_with(0x0376, &[0xA9, 0x00, 0x9D, 0xA1, 0x07]);
    let out = run_subroutine(&prg, 0xC376, 7, &input(9, 2, 0), &[0xC37B]).expect("run");
    assert_eq!((out.a, out.x, out.y), (0x00, 2, 0));
    assert_eq!(out.writes, vec![(0x07A3, 0x00)]);
}

#[test]
fn l2_unsupported_opcode_fails_loud() {
    let prg = prg_with(0x0000, &[0x02]); // 非法操作码（6502 undoc，L2 面不支持）
    let mut cpu = Cpu::new(&prg, 7, &input(0, 0, 0));
    cpu.set_pc(0xC000);
    assert!(cpu.step().is_err());
}
