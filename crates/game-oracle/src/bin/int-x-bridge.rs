//! int-x-bridge —— SMT 证明的公式↔Rust 桥接探针。
//! stdin 每行 "si sf acc py m"（hex 2 位），stdout 每行 "i f p disp x y lim1"。
use game_core::samus::integrators::{int_x, IntXState};
use std::io::{BufRead, Write};

fn main() {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    for line in stdin.lock().lines() {
        let line = line.unwrap();
        let v: Vec<u8> = line
            .split_whitespace()
            .map(|s| u8::from_str_radix(s, 16).unwrap())
            .collect();
        if v.len() != 5 {
            continue;
        }
        let mut st = IntXState { speed_i: v[0], speed_f: v[1], subpix: v[3] };
        let r = int_x(&mut st, v[2], v[4]);
        writeln!(
            out,
            "{:02x} {:02x} {:02x} {:02x} {:02x} {:02x} {:02x}",
            st.speed_i, st.speed_f, st.subpix, r.disp, r.reg_x, r.reg_y, r.lim_hi_final
        )
        .unwrap();
    }
}
