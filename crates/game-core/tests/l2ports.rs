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
