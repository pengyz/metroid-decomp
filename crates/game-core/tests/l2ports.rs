//! game-core l2ports 语义测试（RED 先行）+ gap 登记簿测试（基础设施）。

use game_core::{gap, l2ports, L2In};

fn input(a: u8, x: u8, y: u8) -> L2In {
    L2In { a, x, y, ..Default::default() }
}

#[test]
fn sub_c16d_writes_pointer_and_returns_ff() {
    let out = l2ports::sub_c16d(&input(3, 5, 7));
    assert_eq!((out.a, out.x, out.y), (0xFF, 5, 7));
    let mut w = out.writes.clone();
    w.sort();
    assert_eq!(w, vec![(0x0000, 0xFF), (0x0001, 0x01)]);
}

#[test]
fn sub_c376_indexed_zero_store() {
    let out = l2ports::sub_c376(&input(9, 2, 0));
    assert_eq!((out.a, out.x, out.y), (0x00, 2, 0));
    assert_eq!(out.writes, vec![(0x07A3, 0x00)]);
}

#[test]
fn lookup_returns_registered_ports() {
    assert!(l2ports::lookup("sub_C16D").is_some());
    assert!(l2ports::lookup("sub_C376").is_some());
    assert!(l2ports::lookup("sub_TODO").is_none());
}

#[test]
fn unported_stubs_fail_loud_and_record() {
    gap::reset();
    let r = std::panic::catch_unwind(|| game_core::unported!("bank07:DEAD"));
    assert!(r.is_err(), "unported! 必须 panic");
    assert_eq!(gap::recorded(), vec!["bank07:DEAD".to_string()]);
    gap::reset();
}

#[test]
fn sub_c376_write_addr_mirrors_at_2k() {
    // F2: 写地址须按契约归一 &0x7FF；X=0x5F ⇒ 真实 $0800 镜像 $0000
    let out = l2ports::sub_c376(&input(0, 0x5F, 0));
    assert_eq!(out.writes, vec![(0x0000, 0x00)]);
}

#[test]
fn micro_slices_semantics() {
    let inp = input(0x30, 0x40, 0x50);
    let r = l2ports::sub_f1f4(&inp);
    assert_eq!((r.a, r.x, r.y, r.writes), (0x30, 0x30, 0x50, vec![])); // X-=0x10；A=结果
    let r = l2ports::sub_f1ee(&inp);
    assert_eq!((r.a, r.x, r.y, r.writes), (0x60, 0x40, 0x60, vec![])); // Y+=0x10；A=结果
    let mut inp2 = input(0x10, 0x00, 0x4F); // Y=0x4F ⇒ 读 $0350
    inp2.ram[0x350] = 0x22;
    let r = l2ports::sub_f1e0(&inp2);
    assert_eq!((r.a, r.writes), (0x32, vec![(0x0004, 0x32)])); // A+RAM[$301+Y] → $04
}

#[test]
fn f152_wram_and_eor_zp_semantics() {
    let mut inp = input(0, 50, 0);
    inp.wram = vec![0; 0x2000];
    inp.wram[0x6AFB + 50 - 0x6000] = 0b101; // wram[$6B2D] bit0=1
    inp.ram[0x00FF] = 0b000;                 // EOR 零页 $FF = 0
    let r = l2ports::sub_f152(&inp);
    assert_eq!(r.a, 1); // (5 ^ 0) & 1 = 1
    inp.ram[0x00FF] = 0b001; // 零页 $FF = 1 ⇒ 翻转 bit0
    let r = l2ports::sub_f152(&inp);
    assert_eq!(r.a, 0); // (5 ^ 1) & 1 = 0 —— EOR zp 语义锁定
    assert_eq!(r.writes[2], (0x000B, r.a));
}

#[test]
fn f186_f1bf_semantics() {
    let mut inp = input(0x11, 0x02, 0x03); // Y=3
    inp.ram[0x030D + 3] = 0xAA;
    inp.ram[0x030E + 3] = 0xBB;
    inp.ram[0x030C + 3] = 0b101;
    inp.ram[0x00FF] = 0b001; // EOR 零页 $FF ⇒ bit0 翻转
    let r = l2ports::sub_f186(&inp);
    assert_eq!((r.a, r.writes), (0, vec![(0x0006, 0xAA), (0x0008, 0xBB), (0x000A, 0)]));
    let mut inp2 = input(0, 0x02, 0x03);
    inp2.wram = vec![0; 0x2000];
    inp2.wram[0x6AF5 + 2 - 0x6000] = 0xF0;
    inp2.wram[0x6AF6 + 2 - 0x6000] = 0x20;
    inp2.ram[0x0301 + 3] = 0x20;
    inp2.ram[0x0302 + 3] = 0x90;
    let r = l2ports::sub_f1bf(&inp2);
    assert_eq!((r.a, r.writes), (0xB0, vec![(0x0004, 0x10), (0x0005, 0xB0)])); // 8 位回绕加法
}
