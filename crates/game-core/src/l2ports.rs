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
        "sub_F149" => Some(sub_f149),
        "sub_F1A7" => Some(sub_f1a7),
        "sub_F1B3" => Some(sub_f1b3),
        "sub_F1D2" => Some(sub_f1d2),
        "sub_F1CB" => Some(sub_f1cb),
        "sub_F1E7" => Some(sub_f1e7),
        "sub_F172" => Some(sub_f172),
        "sub_F19A" => Some(sub_f19a),
        "sub_F162" => Some(sub_f162),
        "sub_F1FA" => Some(sub_f1fa),
        "sub_F270" => Some(sub_f270),
        "sub_F2DF" => Some(sub_f2df),
        "sub_F2D5" => Some(sub_f2d5),
        "sub_F2D8" => Some(sub_f2d8),
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

/// sub_F162 —— bank07:F162-F171（`LDA $400,Y / STA $06 / LDA $401,Y / STA $08 / LDA $6AFB,Y /`
/// 经 F193 共享尾：Y 变体坐标/属性读取，F152 的对偶）。
/// L2 窗口 stops=bank07:F199（F193 尾 RTS 址截断）。
/// 语义（A 级：ROM 原字节 `B9 00 04 85 06 B9 01 04 85 08 B9 FB 6A 4C 93 F1`；F193 尾
/// `45 FF 29 01 85 0A 60`）：$06←RAM[$400+Y]；$08←RAM[$401+Y]；
/// $0A←(WRAM[$6AFB+Y] ^ **RAM[$FF]**)&1（45 FF=EOR 零页 $FF 非立即数，同 F152/F186 教训）。
/// 末值 A=$0A；X/Y 透传。注：$F109 `JSR $F162` 实存（samus-movement.md 2.2 更正）。
pub fn sub_f162(input: &L2In) -> L2Out {
    let y = input.y as u16;
    let v6 = input.ram[(0x0400 + y) as usize & 0x7FF];
    let v8 = input.ram[(0x0401 + y) as usize & 0x7FF];
    let w = input.wram.get(0x6AFB + y as usize - 0x6000).copied().unwrap_or(0);
    let va = (w ^ input.ram[0x00FF]) & 1;
    L2Out {
        a: va,
        x: input.x,
        y: input.y,
        writes: vec![(0x0006, v6), (0x0008, v8), (0x000A, va)],
    }
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

/// sub_F19A —— bank07:F19A-F1A6（`LDA $B1,X / STA $07 / LDA $B2,X / STA $09 / LDA $B3,X /`
/// 经 loc_F17F 共享尾：zp,X 变体坐标/属性读取）。
/// L2 窗口 stops=bank07:F185（尾 RTS 址截断）。
/// 语义（A 级：ROM 原字节 `B5 B1 85 07 B5 B2 85 09 B5 B3 4C 7F F1`）：
/// **B5 B1-B3 = zp,X 8 位变址回绕**：有效址=(base+X)&$FF（X=$FF 时 $B1→$B0；16 位加会错读
/// $01Bx 域）；$07←RAM[($B1+X)&$FF]、$09←RAM[($B2+X)&$FF]、$0B←(RAM[($B3+X)&$FF]^**RAM[$FF]**)&1
/// （尾 45 FF=EOR 零页 $FF 非立即数）。末值 A=$0B；X/Y 透传。
pub fn sub_f19a(input: &L2In) -> L2Out {
    let x = input.x;
    let a1 = 0xB1u8.wrapping_add(x) as usize;
    let a2 = 0xB2u8.wrapping_add(x) as usize;
    let a3 = 0xB3u8.wrapping_add(x) as usize;
    let v7 = input.ram[a1];
    let v9 = input.ram[a2];
    let vb = (input.ram[a3] ^ input.ram[0x00FF]) & 1;
    L2Out {
        a: vb,
        x: input.x,
        y: input.y,
        writes: vec![(0x0007, v7), (0x0009, v9), (0x000B, vb)],
    }
}

/// sub_F172 —— bank07:F172-F185（`LDA $30D,X / STA $07 / LDA $30E,X / STA $09 / LDA $30C,X /`
/// 经 loc_F17F 共享尾：X 变体坐标/属性读取）。
/// L2 窗口 stops=bank07:F185（尾 RTS 址截断，与 gen-f152 同约定）。
/// 语义（A 级：ROM 原字节 `BD 0D 03 85 07 BD 0E 03 85 09 BD 0C 03 45 FF 29 01 85 0B 60`）：
/// $07 ← RAM[$30D+X]；$09 ← RAM[$30E+X]；$0B ← (RAM[$30C+X] ^ **RAM[$FF]**) & 1
/// （F17F `45 FF` = EOR **零页 $FF**，非立即数——同 F152/F186 教训）。
/// 末值 A=$0B；X/Y 透传。
pub fn sub_f172(input: &L2In) -> L2Out {
    let x = input.x as u16;
    let v7 = input.ram[(0x030D + x) as usize & 0x7FF];
    let v9 = input.ram[(0x030E + x) as usize & 0x7FF];
    let vb = (input.ram[(0x030C + x) as usize & 0x7FF] ^ input.ram[0x00FF]) & 1;
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

/// sub_F1CB —— bank07:F1CB-F1D1（`CLC / ADC $6AF6,Y / STA $05 / RTS`，F1B3 共享尾本体）。
/// L2 窗口 stops=bank07:F1D1（尾 RTS 址截断）。
/// 语义（A 级：ROM 原字节 `18 79 F6 6A 85 05 60` 逐指令）：
/// **79 F6 6A = ADC abs,Y 读 WRAM $6AFx 域**（陷阱清单主踩点）：$05 ← A + WRAM[$6AF6+Y]。
/// 末值 A=$05；X/Y 透传；入参 A 参与运算（与 F1D2 不同）。
pub fn sub_f1cb(input: &L2In) -> L2Out {
    let y = input.y as usize;
    let w6 = input.wram.get(0x6AF6 + y - 0x6000).copied().unwrap_or(0);
    let v5 = input.a.wrapping_add(w6);
    L2Out { a: v5, x: input.x, y: input.y, writes: vec![(0x0005, v5)] }
}

/// sub_F1E7 —— bank07:F1E7-F1ED（`CLC / ADC $6AF5,Y / STA $04 / RTS`，F1B3 共享尾本体）。
/// L2 窗口 stops=bank07:F1ED（尾 RTS 址截断）。
/// 语义（A 级：ROM 原字节 `18 79 F5 6A 85 04 60` 逐指令）：
/// **79 F5 6A = ADC abs,Y 读 WRAM $6AFx 域**（陷阱清单主踩点）：$04 ← A + WRAM[$6AF5+Y]。
/// 末值 A=$04；X/Y 透传；入参 A 参与运算。
pub fn sub_f1e7(input: &L2In) -> L2Out {
    let y = input.y as usize;
    let w5 = input.wram.get(0x6AF5 + y - 0x6000).copied().unwrap_or(0);
    let v4 = input.a.wrapping_add(w5);
    L2Out { a: v4, x: input.x, y: input.y, writes: vec![(0x0004, v4)] }
}

/// sub_F1D2 —— bank07:F1D2-F1DF（`LDA #4 / JSR F1E0 / LDA #8 /` 落 loc_F1D9 共享尾）。
/// L2 窗口 stops=bank07:F1DF（尾 RTS 址截断，与 gen-f1bf 同约定）。
/// 语义（A 级：ROM 原字节 `A9 04 20 E0 F1 A9 08 18 79 02 03 85 05 60` 逐指令）：
/// **A9 04/A9 08 = LDA #立即数**（反向陷阱，勿读 zp $04/$08）：
/// $04 ← 4 + RAM[$301+Y]（F1E0 尾 CLC/ADC $301,Y/STA $04）；
/// $05 ← 8 + RAM[$302+Y]（loc_F1D9 CLC/ADC $302,Y/STA $05）。
/// 末值 A=$05；X/Y 透传；入参 A 不影响（窗口首指令即 LDA #4）。
pub fn sub_f1d2(input: &L2In) -> L2Out {
    let y = input.y as u16;
    let v4 = 4u8.wrapping_add(input.ram[(0x0301 + y) as usize & 0x7FF]);
    let v5 = 8u8.wrapping_add(input.ram[(0x0302 + y) as usize & 0x7FF]);
    L2Out { a: v5, x: input.x, y: input.y, writes: vec![(0x0004, v4), (0x0005, v5)] }
}

/// sub_F149 —— bank07:F149-F151 入口组合件（`JSR F186 / JSR F1D2 / JMP F1FA`）。
/// L2 窗口止于 F1FA 入口（碰撞尾独立切片 S-FUNC-07-F1FA，stops=bank07:F1FA）。
/// 语义（A 级：ROM 原字节 `20 86 F1 20 D2 F1 4C FA F1` 逐指令 + seed 各子例程）：
/// 先 F186（$06←RAM[$30D+Y]、$08←RAM[$30E+Y]、$0A←(RAM[$30C+Y]^RAM[$FF])&1，EOR zp），
/// 后 F1D2（$04←4+RAM[$301+Y]、$05←8+RAM[$302+Y]，LDA #立即数）。
/// 末值 A=$05；写集 {$04,$05,$06,$08,$0A} 无重叠；X/Y 透传。
pub fn sub_f149(input: &L2In) -> L2Out {
    let f186 = sub_f186(input);
    let f1d2 = sub_f1d2(input);
    let mut writes = f186.writes;
    writes.extend(f1d2.writes);
    L2Out {
        a: f1d2.a,
        x: input.x,
        y: input.y,
        writes,
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

/// sub_F1A7 —— bank07:F1A7-F1B2（`LDA $301,X / JSR F1E0 / LDA $302,X / JMP loc_F1D9` 共享尾变体）。
/// L2 窗口 stops=bank07:F1DF（F1D9 尾 RTS 址截断，与 gen-f1bf 同约定）。
/// 语义（A 级：ROM 原字节 `BD 01 03 20 E0 F1 BD 02 03 4C D9 F1` 逐指令）：
/// $04 ← RAM[$301+X] + RAM[$301+Y]（F1E0 尾 CLC/ADC $301,Y/STA $04）；
/// $05 ← RAM[$302+X] + RAM[$302+Y]（loc_F1D9 CLC/ADC $302,Y/STA $05）。
/// 末值 A=$05；X/Y 透传；abs,X/abs,Y 16 位加（$301+X≤$400，无 zp 回绕）。
pub fn sub_f1a7(input: &L2In) -> L2Out {
    let x = input.x as u16;
    let y = input.y as u16;
    let v4 = input.ram[(0x0301 + x) as usize & 0x7FF]
        .wrapping_add(input.ram[(0x0301 + y) as usize & 0x7FF]);
    let v5 = input.ram[(0x0302 + x) as usize & 0x7FF]
        .wrapping_add(input.ram[(0x0302 + y) as usize & 0x7FF]);
    L2Out { a: v5, x: input.x, y: input.y, writes: vec![(0x0004, v4), (0x0005, v5)] }
}

/// sub_F1B3 —— bank07:F1B3-F1BE（`LDA $301,X / JSR F1E7 / LDA $302,X / JMP loc_F1CB` 共享尾变体）。
/// L2 窗口 stops=bank07:F1D1（loc_F1CB 尾 RTS 址截断）。
/// 语义（A 级：ROM 原字节 `BD 01 03 20 E7 F1 BD 02 03 4C CB F1` 逐指令）：
/// $04 ← RAM[$301+X] + WRAM[$6AF5+Y]（F1E7 尾 CLC/ADC $6AF5,Y/STA $04）；
/// $05 ← RAM[$302+X] + WRAM[$6AF6+Y]（loc_F1CB CLC/ADC $6AF6,Y/STA $05）。
/// 末值 A=$05；X/Y 透传；ADC **abs,Y 读 WRAM $6AFx 域**（16 位加，$6AF5+Y≤$6BF4）。
pub fn sub_f1b3(input: &L2In) -> L2Out {
    let x = input.x as u16;
    let y = input.y as usize;
    let w5 = input.wram.get(0x6AF5 + y - 0x6000).copied().unwrap_or(0);
    let w6 = input.wram.get(0x6AF6 + y - 0x6000).copied().unwrap_or(0);
    let v4 = input.ram[(0x0301 + x) as usize & 0x7FF].wrapping_add(w5);
    let v5 = input.ram[(0x0302 + x) as usize & 0x7FF].wrapping_add(w6);
    L2Out { a: v5, x: input.x, y: input.y, writes: vec![(0x0004, v4), (0x0005, v5)] }
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

// ---- sub_F1FA 碰撞逻辑（F1FA-F261 全窗，含内联 sub_F262/sub_F266/sub_E449）----

/// F266 尾（sub_F262 先做 `LDA $0B / SBC $0A` 续 F205/F23D 进位后落入本尾）：
/// `STA $01 / BPL → RTS / JSR E449 / INC $10 / RTS`。
/// A<0 ⇒ E449 16 位取负 $00:$01（LDA #0/SEC/SBC $00/STA $00/LDA #0/SBC $01/STA $01）+ INC $10。
fn f1fa_f266(st: &mut [u8; 0x100], t: &mut [bool; 0x100], a: u8) {
    st[0x01] = a;
    t[0x01] = true;
    if a & 0x80 != 0 {
        let mut c = 1i32; // SEC
        let tmp = 0 - st[0x00] as i32 - (1 - c);
        c = if tmp >= 0 { 1 } else { 0 };
        st[0x00] = (tmp & 0xFF) as u8;
        t[0x00] = true;
        let tmp = 0 - st[0x01] as i32 - (1 - c);
        st[0x01] = (tmp & 0xFF) as u8;
        t[0x01] = true;
        st[0x10] = st[0x10].wrapping_add(1);
        t[0x10] = true;
    }
}

/// sub_F1FA —— bank07:F1FA-F261 碰撞逻辑全窗（stops=bank07:F261，唯一出口 F261）。
/// 语义（A 级：ROM 原字节 F1FA-F261 + F262/F266/E449 全区逐指令；两轴路由相反——
/// 首轴 `LDA $03/BNE F224`（$03≠0→符号扩展路）、次轴 `LDA $03/BEQ F251`（$03==0→符号扩展路）：
/// ① $10←2、$03←2&RAM[$49]；$00←RAM[$07]-RAM[$06]（SEC/SBC，进位 C1 入 16 位链）。
/// ② 链路（$03==0∧$0B≠$0A 首轴 / $03≠0∧$0B≠$0A 次轴）：$01←RAM[$0B]-RAM[$0A]-!C（SBC 续借位），
///    负⇒E449 取负+INC $10；首轴另做 $00-=0x10（借位则 DEC $01）。
///    符号扩展路：$01←0-0-!C（=借位符号扩展），负⇒同上取负。
/// ③ $01≠0 ⇒ 出口（A=$01）；否则 $11←$00，$00<$04 才继续（否则出口 A=$00）；ASL $10；
///    次轴 $00←RAM[$09]-RAM[$08]（同构）；$01≠0 ⇒ 出口 A=$01；否则 $0F←$00，落 F261（A=$00）。
/// 写集=实写 zp 最终值映射（$00/$01/$03/$0F/$10/$11 按路径子集）；X/Y 透传。
pub fn sub_f1fa(input: &L2In) -> L2Out {
    let mut st = [0u8; 0x100];
    st.copy_from_slice(&input.ram[..0x100]);
    let mut t = [false; 0x100];
    st[0x10] = 2;
    t[0x10] = true;
    let a3 = 2 & st[0x49];
    st[0x03] = a3;
    t[0x03] = true;
    // 首轴：SEC/SBC $06
    let c1 = if st[0x07] >= st[0x06] { 1i32 } else { 0i32 };
    st[0x00] = st[0x07].wrapping_sub(st[0x06]);
    t[0x00] = true;
    if a3 != 0 {
        // F224：LDA #0 / SBC #0（进位=首轴借位）/ JSR F266
        let a = (0i32 - (1 - c1)) as u8 & 0xFF;
        f1fa_f266(&mut st, &mut t, a);
    } else if (st[0x0B] ^ st[0x0A]) == 0 {
        let a = (0i32 - (1 - c1)) as u8 & 0xFF;
        f1fa_f266(&mut st, &mut t, a);
    } else {
        // JSR F262：LDA $0B / SBC $0A（续 c1 借位）/ 落 F266
        let tmp = st[0x0B] as i32 - st[0x0A] as i32 - (1 - c1);
        let a = (tmp & 0xFF) as u8;
        f1fa_f266(&mut st, &mut t, a);
        // $00 -= 0x10，借位则 DEC $01
        let tmp = st[0x00] as i32 - 0x10;
        st[0x00] = (tmp & 0xFF) as u8;
        t[0x00] = true;
        if tmp < 0 {
            st[0x01] = st[0x01].wrapping_sub(1);
            t[0x01] = true;
        }
    }
    let a_out;
    if st[0x01] != 0 {
        a_out = st[0x01]; // 出口 F22E：A=$01
    } else {
        st[0x11] = st[0x00];
        t[0x11] = true;
        if st[0x00] >= st[0x04] {
            a_out = st[0x00]; // 出口 F236（CMP $04 BCS）：A=$00
        } else {
            st[0x10] = st[0x10] << 1;
            t[0x10] = true;
            // 次轴：SEC/SBC $08
            let c2 = if st[0x09] >= st[0x08] { 1i32 } else { 0i32 };
            st[0x00] = st[0x09].wrapping_sub(st[0x08]);
            t[0x00] = true;
            if a3 == 0 || (st[0x0B] ^ st[0x0A]) == 0 {
                // F251：SBC #0 / JSR F266
                let a = (0i32 - (1 - c2)) as u8 & 0xFF;
                f1fa_f266(&mut st, &mut t, a);
            } else {
                let tmp = st[0x0B] as i32 - st[0x0A] as i32 - (1 - c2);
                let a = (tmp & 0xFF) as u8;
                f1fa_f266(&mut st, &mut t, a);
            }
            if st[0x01] != 0 {
                a_out = st[0x01]; // 出口 F259：A=$01
            } else {
                st[0x0F] = st[0x00];
                t[0x0F] = true;
                a_out = st[0x00]; // 落 F261（CMP $05 仅置标志）：A=$00
            }
        }
    }
    let mut writes: Vec<(u16, u8)> = (0..0x100u16)
        .filter(|&i| t[i as usize])
        .map(|i| (i, st[i as usize]))
        .collect();
    writes.sort_unstable();
    L2Out { a: a_out, x: input.x, y: input.y, writes }
}

// ---- task-8 F2CA 族（九入口共享 RTS 尾；decode-sweep r1 §3，切片边界定案见各 doc 注）----

/// sub_F2D8 —— bank07:F2D8-F2DE 注册名（机械名只在注册注释，two-layer §4）。
/// **域形态**（§5 迁移件）：`enemy::EnemyTodo0404::or_writeback`（OR 回写，掩码=入参 A）
/// 经 `verify_shim::f2d8` 适配；旧字面转写见 commit 1d61521（§5 参考件）——同批 L2 案双绿。
/// L2 窗口 stops=bank07:F2DE（尾 RTS 址截断）。
/// 语义（A 级：ROM 原字节 `1D 04 04 9D 04 04 60`）：$0404+X ← A|old（入参 A 参与——与 F2DF
/// 的 LDA $10 起手不同）；末值 A=A|old；X/Y 透传；abs,X 16 位加（$404+X≤$503）。
pub fn sub_f2d8(input: &L2In) -> L2Out {
    crate::verify_shim::f2d8(input)
}

/// sub_F2DF —— bank07:F2DF-F2E7 注册名（机械名只在注册注释，two-layer §4）。
/// **域形态**（§5 迁移件）：`enemy::EnemyTodo0404::or_writeback`（OR 回写，掩码=memmap 掩码源
/// zp——LDA zp $10 起手，入参 A 不影响）经 `verify_shim::f2df` 适配；旧字面转写见 commit c523762。
/// L2 窗口 stops=bank07:F2E7（尾 RTS 址截断）。
/// 语义（A 级：ROM 原字节 `A5 10 19 04 04 99 04 04 60`）：$0404+Y ← RAM[$10]|old（LDA zp $10
/// 起手——入参 A 不影响），末值 A=RAM[$10]|old；X/Y 透传；abs,Y 16 位加（$404+Y≤$503）。
pub fn sub_f2df(input: &L2In) -> L2Out {
    crate::verify_shim::f2df(input)
}

/// sub_F270 —— bank07:F270-F276 注册名（机械名只在注册注释，two-layer §4）。
/// L2 窗口 stops=bank07:F276（尾 RTS 址截断）。
/// **域形态**（two-layer §5 迁移件）：行为=`enemy::EnemyAttrs::or_attribute`（属性掩码 OR 回写），
/// 经 `verify_shim::f270` 适配（写集由状态变更推导）；旧字面转写（ORA/STA abs,X 直写）见
/// commit 551d307（§5 参考件）——迁移前后同批 L2 案 11 条双绿=「行为钉死=结构可换」实证。
/// 语义证据（A 级：ROM 原字节 `1D 0A 03 9D 0A 03 60`）：$030A+X ← A|old，末值 A=A|old。
pub fn sub_f270(input: &L2In) -> L2Out {
    crate::verify_shim::f270(input)
}

/// sub_F2D5 —— bank07:F2D5-F2DE 注册名（机械名只在注册注释，two-layer §4）。
/// **域形态**（§5 新实施件）：`enemy::EnemyTodo0404::or_writeback`（OR 回写，掩码=memmap
/// 变换掩码 ((RAM[$10]^3)<<3)——F340 EOR#3+C2C6 尾 ASL×3）经 `verify_shim::f2d5` 适配。
/// L2 窗口 stops=bank07:F2DE（JSR F332 内返回址 C2C9 不在 stops，逐址核过）。
pub fn sub_f2d5(input: &L2In) -> L2Out {
    crate::verify_shim::f2d5(input)
}
