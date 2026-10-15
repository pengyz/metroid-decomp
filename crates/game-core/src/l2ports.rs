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
        "sub_F152" => Some(sub_f152),
        "sub_F186" => Some(sub_f186),
        "sub_F1BF" => Some(sub_f1bf),
        "sub_F140" => Some(sub_f140),
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

/// sub_F152 —— bank07:F152-F185（经 loc_F17F 共享尾）：坐标/属性读取器（X 变体）。
/// $07 ← RAM[$400+X]；$09 ← RAM[$401+X]；$0B ← (WRAM[$6AFB+X] ^ **RAM[$FF]**) & 1（EOR zp）。
/// 语义证据（A 级）：seed 逐指令；$6AFx 域=WRAM 暂存区（findings/ram-region-6af4-contradiction.md 定谳）。
pub fn sub_f152(input: &L2In) -> L2Out {
    let x = input.x as u16;
    let v7 = input.ram[(0x0400 + x) as usize & 0x7FF];
    let v9 = input.ram[(0x0401 + x) as usize & 0x7FF];
    let w = input.wram.get(0x6AFB + x as usize - 0x6000).copied().unwrap_or(0);
    // F17F `45 FF` = EOR **零页 $FF**（RAM），非立即数——操作数类型以清单 # 前缀为准
    let vb = (w ^ input.ram[0x00FF]) & 1; // ^ RAM[$FF]（EOR zp，非立即数）
    L2Out {
        a: vb,
        x: input.x,
        y: input.y,
        writes: vec![(0x0007, v7), (0x0009, v9), (0x000B, vb)],
    }
}

/// sub_F186 —— bank07:F186-F199（Y 变体坐标/属性读取）：
/// $06 ← RAM[$30D+Y]；$08 ← RAM[$30E+Y]；$0A ← (RAM[$30C+Y] ^ RAM[$FF]) & 1。
/// 语义证据（A 级）：seed 逐指令；F193 `45 FF` = EOR **零页 $FF**（同 F17F 教训）。
pub fn sub_f186(input: &L2In) -> L2Out {
    let y = input.y as u16;
    let v6 = input.ram[(0x030D + y) as usize & 0x7FF];
    let v8 = input.ram[(0x030E + y) as usize & 0x7FF];
    let va = (input.ram[(0x030C + y) as usize & 0x7FF] ^ input.ram[0x00FF]) & 1;
    L2Out {
        a: va,
        x: input.x,
        y: input.y,
        writes: vec![(0x0006, v6), (0x0008, v8), (0x000A, va)],
    }
}

/// sub_F140 —— bank07:F140-F148 入口组合件（`JSR F1BF / JSR F186 / JMP F1FA`）。
/// L2 窗口止于 F1FA 入口（碰撞尾独立切片 S-FUNC-07-F1FA，stops=bank07:F1FA）。
/// 语义（A 级：ROM 原字节 `20 BF F1 20 86 F1 4C FA F1` 逐指令 + seed 各子例程）：
/// 先 F1BF（$04←WRAM[$6AF5+X]+RAM[$301+Y]、$05←WRAM[$6AF6+X]+RAM[$302+Y]），
/// 后 F186（$06←RAM[$30D+Y]、$08←RAM[$30E+Y]、$0A←(RAM[$30C+Y]^RAM[$FF])&1，EOR zp）。
/// 末值 A=$0A；写集 {$04,$05,$06,$08,$0A} 无重叠；X/Y 透传。
pub fn sub_f140(input: &L2In) -> L2Out {
    let f1bf = sub_f1bf(input);
    let f186 = sub_f186(input);
    let mut writes = f1bf.writes;
    writes.extend(f186.writes);
    L2Out {
        a: f186.a,
        x: input.x,
        y: input.y,
        writes,
    }
}

/// sub_F1BF —— bank07:F1BF-F1DF（WRAM 源坐标累加，经 sub_F1E0/loc_F1D9 共享尾）：
/// $04 ← WRAM[$6AF5+X] + RAM[$301+Y]；$05 ← WRAM[$6AF6+X] + RAM[$302+Y]。
/// 语义证据（A 级）：seed 逐指令；$6AFx=WRAM 域（findings 定谳）。
pub fn sub_f1bf(input: &L2In) -> L2Out {
    let x = input.x as usize;
    let y = input.y as u16;
    let w1 = input.wram.get(0x6AF5 + x - 0x6000).copied().unwrap_or(0);
    let w2 = input.wram.get(0x6AF6 + x - 0x6000).copied().unwrap_or(0);
    let v4 = w1.wrapping_add(input.ram[(0x0301 + y) as usize & 0x7FF]);
    let v5 = w2.wrapping_add(input.ram[(0x0302 + y) as usize & 0x7FF]);
    L2Out {
        a: v5,
        x: input.x,
        y: input.y,
        writes: vec![(0x0004, v4), (0x0005, v5)],
    }
}
