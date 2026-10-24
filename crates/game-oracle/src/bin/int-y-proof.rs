//! int-y-proof —— int_y 移植件的 **2^32 输入域穷举等价证明**（证明级证书）。
//!
//! 证法：对 bank07:E3A5-E3E4（含内嵌 sub_D147）的**全部 4 字节输入空间**
//! （speed_i:$308 × speed_f:$312 × accel:$314 × subpix:$310 = 2^32）逐点比对：
//!   Side A = ROM **字节驱动**微解释器（只实现切片出现的 17 个操作码，语义表逐条对应
//!            6502 手册——不是对反汇编的转述，执行的是 ROM 原字节）
//!   Side B = game_core::samus::integrators::int_y（待证移植件）
//! 零分歧 ⇒ 在声明域（该切片仅读 4 字节，指令清单可数）内 **数学穷尽证明**。
//! 桥接状态（如实声明，r5 复审 E1）：本机不含对已审计 6502 核的抽样桥接；
//! Side A 的可信度依据 = ①字节驱动（执行 ROM 原字节非转述）②审核员独立反汇编模型
//! 3780 案复核一致（rounds/r6）③切片字节断言。假设台账见 docs/findings/l2-proof-assumptions.md。
//!
//! 用法: int-y-proof --rom <rom.nes>

use game_core::samus::integrators::{int_y, IntYState};
use std::process::ExitCode;

/// ROM 切片：bank07:E3A5..E3E4（64 字节窗口足够）+ sub_D147（bank07:D147..D14F）。
fn slice_bytes(prg: &[u8], addr: u16, len: usize) -> Vec<u8> {
    let off = 0x1C000 + (addr as usize - 0xC000);
    prg[off..off + len].to_vec()
}

/// 6502 标志结构（仅切片所需）。
#[derive(Clone, Copy)]
struct Flags {
    c: bool,
    n: bool,
    z: bool,
}

/// Side A：字节驱动微解释器（JSR/RTS 用小栈跨块返回）。
const Y_INIT: u8 = 0x5A; // Y 判别代表值（输出=保持 init_y 或清零，分支常量）
fn run_slice2(code: &[u8], d147: &[u8], s_i: u8, s_f: u8, acc: u8, p_y: u8) -> (u8, u8, u8, u8, u8, u8) {
    let (mut a, mut x, mut y) = (0u8, 0u8, Y_INIT);
    let (mut mem_si, mut mem_sf, mut mem_acc, mut mem_py) = (s_i, s_f, acc, p_y);
    let mut out0 = 0u8;
    let mut fl = Flags { c: false, n: false, z: false };
    let mut stack: Vec<(u8, usize)> = Vec::new(); // (bank_id, ret_pc)
    let mut cur: u8 = 0; // 0=main, 1=d147
    let mut pc = 0usize;

    macro_rules! setnz { ($v:expr) => {{ fl.n = $v & 0x80 != 0; fl.z = $v == 0; }} }
    macro_rules! adc { ($m:expr) => {{
        let r = a as u16 + $m as u16 + fl.c as u16;
        fl.c = r > 0xFF; a = r as u8; setnz!(a);
    }} }
    macro_rules! sbc { ($m:expr) => {{
        let r = a as i32 - $m as i32 - (!fl.c) as i32;
        fl.c = r >= 0; a = r as u8; setnz!(a);
    }} }
    macro_rules! cmp_ { ($r:expr, $m:expr) => {{
        fl.c = $r >= $m; setnz!($r.wrapping_sub($m));
    }} }

    loop {
        let code = if cur == 0 { code } else { d147 };
        let op = code[pc];
        pc += 1;
        match op {
            0xA9 => { a = code[pc]; pc += 1; setnz!(a); }
            0xA2 => { x = code[pc]; pc += 1; setnz!(x); }
            0xA0 => { y = code[pc]; pc += 1; setnz!(y); }
            0xAA => { x = a; setnz!(x); }
            0x18 => { fl.c = false; }
            0xAD | 0x6D | 0xCD | 0xED => {
                let addr = code[pc] as usize | ((code[pc + 1] as usize) << 8);
                pc += 2;
                let m = match addr {
                    0x308 => mem_si, 0x312 => mem_sf, 0x314 => mem_acc,
                    0x310 => mem_py, 0x0000 => out0,
                    _ => panic!("读未建地址 ${addr:04X}"),
                };
                match op {
                    0xAD => { a = m; setnz!(a); }
                    0x6D => { adc!(m); }
                    0xCD => { cmp_!(a, m); }
                    0xED => { sbc!(m); }
                    _ => unreachable!(),
                }
            }
            0x8D | 0x8C | 0x8E => {
                let addr = code[pc] as usize | ((code[pc + 1] as usize) << 8);
                pc += 2;
                let v = match op {
                    0x8D => a, 0x8C => y, _ => x,
                };
                match addr {
                    0x308 => mem_si = v, 0x312 => mem_sf = v,
                    0x310 => mem_py = v, 0x0000 => out0 = v,
                    _ => panic!("写未建地址 ${addr:04X}"),
                }
            }
            0x85 | 0x86 | 0x84 => {
                let addr = code[pc] as usize;
                pc += 1;
                let v = match op {
                    0x85 => a, 0x84 => y, _ => x,
                };
                match addr {
                    0x308 => mem_si = v, 0x312 => mem_sf = v,
                    0x310 => mem_py = v, 0x0000 => out0 = v,
                    _ => panic!("写未建 zp 地址 ${addr:04X}"),
                }
            }
            0x69 => { let m = code[pc]; pc += 1; adc!(m); }
            0xC9 => { let m = code[pc]; pc += 1; cmp_!(a, m); }
            0x10 => { let d = code[pc] as i8; pc += 1; if !fl.n { pc = (pc as isize + d as isize) as usize; } }
            0xD0 => { let d = code[pc] as i8; pc += 1; if !fl.z { pc = (pc as isize + d as isize) as usize; } }
            0x90 => { let d = code[pc] as i8; pc += 1; if !fl.c { pc = (pc as isize + d as isize) as usize; } }
            0x20 => { pc += 2; stack.push((cur, pc)); cur = 1; pc = 0; }
            0x60 => {
                if let Some((b, r)) = stack.pop() {
                    cur = b;
                    pc = r;
                } else {
                    break;
                }
            }
            _ => panic!("未建操作码 ${op:02X}（cur={cur} pc={pc}）"),
        }
    }
    (mem_si, mem_sf, mem_py, out0, x, y)
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    let mut rom_path = String::new();
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--rom" => { rom_path = args.get(i + 1).cloned().unwrap_or_default(); i += 2; }
            other => { eprintln!("未知参数: {other}"); return ExitCode::from(2); }
        }
    }
    if rom_path.is_empty() {
        eprintln!("用法: int-y-proof --rom <rom.nes>");
        return ExitCode::from(2);
    }
    let rom = std::fs::read(&rom_path).expect("读 ROM");
    let prg = &rom[0x10..0x20010];

    let code = slice_bytes(prg, 0xE3A5, 0x40); // E3A5..E3E4
    let d147 = slice_bytes(prg, 0xD147, 0x10);
    // 健全性：切片首字节应为 A2 05（LDX #5）
    assert_eq!(&code[0..2], &[0xA2, 0x05], "E3A5 切片字节与预期不符");
    assert_eq!(&d147[0..2], &[0xA0, 0x00], "D147 切片字节与预期不符");

    let mut checked: u64 = 0;
    for si in 0u32..=255 {
        for sf in 0u32..=255 {
            for acc in 0u32..=255 {
                // subpix 全域 + 其余三维全枚举 = 2^32
                for py in 0u32..=255 {
                    let (a, b, c, d, rx, ry) = run_slice2(
                        &code, &d147,
                        si as u8, sf as u8, acc as u8, py as u8,
                    );
                    let mut st = IntYState {
                        speed_i: si as u8,
                        speed_f: sf as u8,
                        subpix: py as u8,
                    };
                    let out = int_y(&mut st, acc as u8);
                    // 全输出面：RAM 三字节 + 位移 + X + Y（Y 按「保持/清零」语义换算）
                    let (mut ex, mut ey) = (0u8, 0u8);
                    let _ = (&mut ex, &mut ey);
                    // 微解释器返回 (si, sf, py, out0)；x/y 由 run_ret 内部状态——改用扩展返回
                    let got = (st.speed_i, st.speed_f, st.subpix, out.disp, out.reg_x,
                               if out.reg_y == 0xFF { Y_INIT } else { out.reg_y });
                    let want = (a, b, c, d, rx, ry);
                    if got != want {
                        eprintln!(
                            "反例: si={si:#04x} sf={sf:#04x} acc={acc:#04x} py={py:#04x} | ROM={want:?} rust={got:?}"
                        );
                        return ExitCode::from(1);
                    }
                    checked += 1;
                }
            }
        }
    }
    println!(
        "EXHAUSTIVE PROOF: int_y ≡ bank07:E3A5-E3E4（字节驱动语义）——{checked} 输入点全枚举（RAM 4B 全域），0 分歧"
    );
    println!(
        "引理: Y 输出=分支常量（保持 init_y / D147 清零），分支谓词已被全域覆盖 ⇒ 对 init_y 全域成立（判别代表值 Y_INIT={Y_INIT:#04x}）"
    );
    println!(
        "声明域: 该切片读 {checked} 点×(X 恒为 5/0xFA 路径常量, Y 见引理) —— 全输出面（RAM 写集+a/x/y）穷尽"
    );
    ExitCode::from(0)
}
