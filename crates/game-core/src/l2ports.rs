//! l2ports —— L2 子程序级移植面（与 oracle/subroutines/l2_cases.jsonl 一一对应）。
//!
//! 每个函数 = 一条 L2 用例的 game-core 侧实现；语义 golden 手推自 A 级反汇编（reference/seed）。
//! 未实现的例程不在此登记（缺口走矩阵 L0 + unported! 桩），**禁止发明规格未覆盖的行为**。

use crate::{L2In, L2Out};

/// bank07:C16D（8B，seed：`LDA #1/STA $01/LDA #$FF/STA $00`，止于 C175）。
/// 语义：写 $01←$01、$00←$FF，A=FF；X/Y 不变。（A 级：reference/seed bank07:C16D-C174）
pub fn sub_c16d(input: &L2In) -> L2Out {
    L2Out {
        a: 0xFF,
        x: input.x,
        y: input.y,
        writes: vec![(0x0001, 0x01), (0x0000, 0xFF)],
    }
}

/// bank07:C376（止于 C37B，PLA/PLA/RTS 尾部用 stops 截断不入比对面）。
/// 语义：A=0，写 `$07A1+X`←$00；X/Y 不变。（A 级：reference/seed bank07:C376-C37A）
pub fn sub_c376(input: &L2In) -> L2Out {
    L2Out {
        a: 0x00,
        x: input.x,
        y: input.y,
        // r3 F2：契约要求写地址归一 $0000-$07FF（2KB RAM 镜像）
        writes: vec![((0x07A1 + input.x as u16) & 0x7FF, 0x00)],
    }
}

/// int_y_E3A5 —— bank07:E3A5-E3E4 垂直积分切片（证明级等价，见 integrators）。
pub fn int_y_e3a5(input: &L2In) -> L2Out {
    let mut st = crate::integrators::IntYState {
        speed_i: input.ram[0x308 & 0x7FF],
        speed_f: input.ram[0x312 & 0x7FF],
        subpix: input.ram[0x310 & 0x7FF],
    };
    let accel = input.ram[0x314 & 0x7FF];
    let out = crate::integrators::int_y(&mut st, accel);
    L2Out {
        a: out.disp,
        x: out.reg_x,
        y: if out.reg_y == 0xFF { input.y } else { out.reg_y },
        writes: vec![
            (0x0312, st.speed_f),
            (0x0308, st.speed_i),
            (0x0310, st.subpix),
            (0x0000, out.disp),
        ],
    }
}

/// int_x_E3E5 —— bank07:E3E5-E448 水平积分切片（SMT 证明级，见 integrators）。
pub fn int_x_e3e5(input: &L2In) -> L2Out {
    let mut st = crate::integrators::IntXState {
        speed_i: input.ram[0x309 & 0x7FF],
        speed_f: input.ram[0x313 & 0x7FF],
        subpix: input.ram[0x311 & 0x7FF],
    };
    let accel = input.ram[0x315 & 0x7FF];
    let max_src = input.ram[0x316 & 0x7FF];
    let out = crate::integrators::int_x(&mut st, accel, max_src);
    L2Out {
        a: out.disp,
        x: out.reg_x,
        y: out.reg_y,
        writes: vec![
            (0x0313, st.speed_f),
            (0x0309, st.speed_i),
            (0x0311, st.subpix),
            (0x0002, max_src << 4),
            (0x0003, max_src >> 4),
            (0x0001, out.lim_hi_final),
            (0x0000, out.disp),
        ],
    }
}

/// 按 L2 用例名查移植函数（l2-diff 裁定器用）。
pub fn lookup(name: &str) -> Option<fn(&L2In) -> L2Out> {
    match name {
        "sub_C16D" => Some(sub_c16d),
        "sub_C376" => Some(sub_c376),
        "int_y_E3A5" => Some(int_y_e3a5),
        "int_x_E3E5" => Some(int_x_e3e5),
        "sub_F1F4" => Some(sub_f1f4),
        "sub_F1EE" => Some(sub_f1ee),
        "sub_F1E0" => Some(sub_f1e0),
        _ => None,
    }
}

// ---- batch-1 F1xx 族微切片（对拍级：三方裁定；指令清单 5-7 条可人核）----

/// sub_F1F4 —— bank07:F1F4-F1F9（TXA/SEC/SBC #$10/TAX）：X -= 0x10，A=X 结果，Y 透传。
/// 语义证据（A 级）：seed metroid.asm 逐指令（5 条）；无 RAM 写。
pub fn sub_f1f4(input: &L2In) -> L2Out {
    let v = input.x.wrapping_sub(0x10);
    L2Out { a: v, x: v, y: input.y, writes: vec![] }
}

/// sub_F1EE —— bank07:F1EE-F1F3（TYA/CLC/ADC #$10/TAY）：Y += 0x10，A=Y 结果，X 透传。
/// 语义证据（A 级）：seed 逐指令（4 条）；无 RAM 写。
pub fn sub_f1ee(input: &L2In) -> L2Out {
    let v = input.y.wrapping_add(0x10);
    L2Out { a: v, x: input.x, y: v, writes: vec![] }
}

/// sub_F1E0 —— bank07:F1E0-F1E6（CLC/ADC $301,Y/STA $04）：$04 ← A + RAM[$0301+Y]。
/// 语义证据（A 级）：seed 逐指令（4 条）；写集 {$0004}；绝对,Y 寻址无跨界（$0301-$0400）。
pub fn sub_f1e0(input: &L2In) -> L2Out {
    let addr = (0x0301 + input.y as u16) & 0x7FF;
    let v = input.a.wrapping_add(input.ram[addr as usize]);
    L2Out { a: v, x: input.x, y: input.y, writes: vec![(0x0004, v)] }
}
