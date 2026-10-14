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

/// 按 L2 用例名查移植函数（l2-diff 裁定器用）。
pub fn lookup(name: &str) -> Option<fn(&L2In) -> L2Out> {
    match name {
        "sub_C16D" => Some(sub_c16d),
        "sub_C376" => Some(sub_c376),
        "int_y_E3A5" => Some(int_y_e3a5),
        _ => None,
    }
}
