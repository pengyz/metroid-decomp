//! game-core::verify_shim —— 验证适配层（two-layer-architecture.md §1）：
//! 域方法 → L2In/L2Out 写集契约。**写集由状态变更经 memmap diff 推导**，不手写地址。
//!
//! 机械名（sub_F2xx）只存在于本层注册注释（回指 bank:addr，§4）；产品层零机械名零地址。
//! 对拍不变式（§3）：行为钉死=结构可换——同批 L2 案在域形态下持续全绿。

use crate::{memmap, L2In, L2Out};
use std::collections::BTreeMap;

/// 写事件流 → **最终值映射**（同址 last-wins，地址序）——L2 写集契约（run_subroutine
/// final_writes 同义）。组合流同址多写（如 f282 的 $0404+X 跨阶段重写）必须经此收敛。
pub fn merge_writes(events: Vec<(u16, u8)>) -> Vec<(u16, u8)> {
    let mut map: BTreeMap<u16, u8> = BTreeMap::new();
    for (a, v) in events {
        map.insert(a, v);
    }
    map.into_iter().collect()
}

/// f270 适配（注册名 `sub_F270` ← bank07:F270-F276，stops=bank07:F276）。
///
/// 域方法：`enemy::EnemyAttrs::or_attribute`（属性掩码 OR 回写）。
/// 写集：状态变更经 memmap::enemy_attrs::diff_writes 推导（字节偏移口径）。
/// 迁移对照：旧字面转写（ORA/STA abs,X 直写）见 commit 551d307——L2 案 11 条前后双绿
/// =「行为钉死=结构可换」第一例（two-layer §3.3）。
pub fn f270(input: &L2In) -> L2Out {
    let mut attrs = memmap::enemy_attrs::decode_attrs(&input.ram);
    let a_out = attrs.or_attribute(input.x, input.a);
    // 写集=语义变更点经 memmap 映射（**同值写也是事件**——b-f270-a0 案曾打回纯值 diff 推导）；
    // diff_writes 留作值变化面互检（单测断言 write_event ⊇ diff_writes）。
    let writes = vec![memmap::enemy_attrs::write_event(input.x, a_out)];
    L2Out {
        a: a_out,
        x: input.x,
        y: input.y,
        writes: merge_writes(writes),
    }
}

/// f2d8 适配（注册名 `sub_F2D8` ← bank07:F2D8-F2DE，stops=bank07:F2DE）。
///
/// 域方法：`enemy::EnemyTodo0404::or_writeback`（OR 回写，掩码=入参 A）。
/// 写集：语义变更点经 memmap::enemy_todo_0404::write_event 推导。
pub fn f2d8(input: &L2In) -> L2Out {
    let mut v0404 = memmap::enemy_todo_0404::decode_todo_0404(&input.ram);
    let a_out = v0404.or_writeback(input.x, input.a);
    let writes = vec![memmap::enemy_todo_0404::write_event(input.x, a_out)];
    L2Out { a: a_out, x: input.x, y: input.y, writes }
}

/// f2df 适配（注册名 `sub_F2DF` ← bank07:F2DF-F2E7，stops=bank07:F2E7）。
///
/// 域方法：`enemy::EnemyTodo0404::or_writeback`（OR 回写，掩码=memmap 掩码源 zp——F2DF 流
/// `LDA zp $10` 起手，入参 A 不影响）。写集：write_event 推导。
pub fn f2df(input: &L2In) -> L2Out {
    let mut v0404 = memmap::enemy_todo_0404::decode_todo_0404(&input.ram);
    let mask = memmap::enemy_todo_0404::mask10(&input.ram);
    let a_out = v0404.or_writeback(input.y, mask);
    let writes = vec![memmap::enemy_todo_0404::write_event(input.y, a_out)];
    L2Out { a: a_out, x: input.x, y: input.y, writes }
}

/// f2d5 适配（注册名 `sub_F2D5` ← bank07:F2D5-F2DE，stops=bank07:F2DE）。
///
/// 域方法：`enemy::EnemyTodo0404::or_writeback`（OR 回写，掩码=memmap 变换掩码
/// ((RAM[$10]^3)<<3)——F340 `EOR #3`+C2C6 尾 `ASL×3`，变换逻辑住映射层）。
/// 同域方法第 3 shim（F2D8/F2DF/F2D5 掩码来源三分）。
pub fn f2d5(input: &L2In) -> L2Out {
    let mut v0404 = memmap::enemy_todo_0404::decode_todo_0404(&input.ram);
    let mask = memmap::enemy_todo_0404::mask10_transformed(&input.ram);
    let a_out = v0404.or_writeback(input.x, mask);
    let writes = vec![memmap::enemy_todo_0404::write_event(input.x, a_out)];
    L2Out { a: a_out, x: input.x, y: input.y, writes }
}

/// f2ad 适配（注册名 `sub_F2AD` ← bank07:F2AD-F2DE，stops=bank07:F2DE）。
///
/// **跨 view 组合**（two-layer §1）：`EnemyTodo040E::set_entry`（state 直写 $81——memmap
/// F2AD_STATE_INIT）后落 F2D5 流 `EnemyTodo0404::or_writeback`（掩码=memmap 变换掩码）。
/// 写集：两域 write_event 依序推导（$040E+X、$0404+X）。
pub fn f2ad(input: &L2In) -> L2Out {
    let mut e040e = memmap::enemy_todo_040e::decode_todo_040e(&input.ram);
    let mut v0404 = memmap::enemy_todo_0404::decode_todo_0404(&input.ram);
    let state_v = e040e.set_entry(input.x, memmap::enemy_todo_040e::F2AD_STATE_INIT);
    let mask = memmap::enemy_todo_0404::mask10_transformed(&input.ram);
    let a_out = v0404.or_writeback(input.x, mask);
    let writes = vec![
        memmap::enemy_todo_040e::write_event(input.x, state_v),
        memmap::enemy_todo_0404::write_event(input.x, a_out),
    ];
    L2Out { a: a_out, x: input.x, y: input.y, writes }
}

/// f2ca 适配（注册名 `sub_F2CA` ← bank07:F2CA-F2DE，stops=bank07:F2DE）。
///
/// **carry 敏感 + 跨 view 三方法组合**（lead 注记④双向 carry 面）：
/// 入参 C=1（`BCS $F2DE`）⇒ 空写集直出（A/X/Y 透传）；
/// C=0 ⇒ EnemyTodo040E::set_entry（$040E+X←RAM[$0300+Y]，值源=memmap::state_source）
/// + EnemyAttrs::or_attribute（$030A+Y←RAM[$10]|old，F279 尾）
/// + EnemyTodo0404::or_writeback（$0404+X←变换掩码|old，F2D5 流）。
/// 写集：三域 write_event 依序推导。内返回址 F281（F279）/C2C9（F332）逐址核过不在 stops。
pub fn f2ca(input: &L2In) -> L2Out {
    const FLAG_C: u8 = 0x01;
    if input.p & FLAG_C != 0 {
        return L2Out { a: input.a, x: input.x, y: input.y, writes: vec![] };
    }
    let mut e040e = memmap::enemy_todo_040e::decode_todo_040e(&input.ram);
    let mut attrs = memmap::enemy_attrs::decode_attrs(&input.ram);
    let mut v0404 = memmap::enemy_todo_0404::decode_todo_0404(&input.ram);
    let state_v = e040e.set_entry(input.x, memmap::enemy_todo_040e::state_source(&input.ram, input.y));
    let attr_v = attrs.or_attribute(input.y, memmap::enemy_todo_0404::mask10(&input.ram));
    let mask = memmap::enemy_todo_0404::mask10_transformed(&input.ram);
    let a_out = v0404.or_writeback(input.x, mask);
    let writes = vec![
        memmap::enemy_todo_040e::write_event(input.x, state_v),
        memmap::enemy_attrs::write_event(input.y, attr_v),
        memmap::enemy_todo_0404::write_event(input.x, a_out),
    ];
    L2Out { a: a_out, x: input.x, y: input.y, writes }
}

/// f2ed 适配（注册名 `sub_F2ED` ← bank07:F2ED-F310，stops=bank07:F310）。
///
/// **carry 敏感 + CD9C 探针路由 + 跨 view 组合**：
/// 入参 C=1（`B0 21` BCS $F310）⇒ 空写集直出；
/// C=0 ⇒ EnemyTodo0404::or_writeback（$0404+Y←RAM[$10]|old，JSR F2DF）→ CD9C 探针
/// （进位=关键输出）→ 探针 C=0 ⇒ 出口（A=探针 A）；
/// 探针 C=1 ⇒ $010F←$80（暂存）+ EnemyAttrs::or_attribute（$030A+X←变换掩码|old，JSR F270）
/// + $6E/$6F←ROM 向量（**窗口参数化**——`rom_tables::rom_vec(input.window_bank)`，
/// A9 处置：game 期窗口=banks1-5 区域 bank，按区域取数）；出口 A=向量高位。
/// 写集：域 write_event + 暂存面事件推导。
pub fn f2ed(input: &L2In) -> L2Out {
    f2ed_flow(input, input.a)
}

/// F2ED 流本体（a_entry=分支点 A：f2ed 为入参 A；f2e8 落入时为半变换掩码值）。
fn f2ed_flow(input: &L2In, a_entry: u8) -> L2Out {
    const FLAG_C: u8 = 0x01;
    if input.p & FLAG_C != 0 {
        return L2Out { a: a_entry, x: input.x, y: input.y, writes: vec![] };
    }
    let mut v0404 = memmap::enemy_todo_0404::decode_todo_0404(&input.ram);
    let a_df = v0404.or_writeback(input.y, memmap::enemy_todo_0404::mask10(&input.ram));
    let probe = memmap::cd9c::probe(&input.ram, &input.wram, a_df);
    if !probe.carry {
        // F2F2 TYA/PHA/…/F2F7 PLA/TAY：出口 A=Y_in（探针 A 被 PLA 覆盖——ROM 裁定修正案）
        return L2Out {
            a: input.y,
            x: input.x,
            y: input.y,
            writes: vec![memmap::enemy_todo_0404::write_event(input.y, a_df)],
        };
    }
    let mut attrs = memmap::enemy_attrs::decode_attrs(&input.ram);
    let mask = memmap::enemy_todo_0404::mask10_transformed(&input.ram);
    let attr_v = attrs.or_attribute(input.x, mask);
    let (vec_lo, vec_hi) = memmap::rom_tables::rom_vec(input.window_bank);
    let writes = vec![
        memmap::enemy_todo_0404::write_event(input.y, a_df),
        memmap::flow_scratch::event_010f(memmap::flow_scratch::F2ED_SCRATCH_INIT),
        memmap::enemy_attrs::write_event(input.x, attr_v),
        memmap::flow_scratch::event_vec_lo(vec_lo),
        memmap::flow_scratch::event_vec_hi(vec_hi),
    ];
    L2Out { a: vec_hi, x: input.x, y: input.y, writes }
}

/// f2e8 适配（注册名 `sub_F2E8` ← bank07:F2E8-F310，stops=[bank07:F2DE, bank07:F310]）。
///
/// **Z 路由 + 双流组合**：JSR F340 半变换掩码（$10^3）非 0（`D0 EB` BNE $F2D8）⇒
/// F2D8 流 or_writeback（$0404+X，掩码=半变换）出口 F2DE；
/// 为 0 ⇒ 落 F2ED 流（入口 A=0——BCS 出口 A 与 f2ed 的入参 A 面不同，参数化复用）。
pub fn f2e8(input: &L2In) -> L2Out {
    let mask = memmap::enemy_todo_0404::mask10_xor3(&input.ram);
    if mask != 0 {
        let mut v0404 = memmap::enemy_todo_0404::decode_todo_0404(&input.ram);
        let a_out = v0404.or_writeback(input.x, mask);
        return L2Out {
            a: a_out,
            x: input.x,
            y: input.y,
            writes: vec![memmap::enemy_todo_0404::write_event(input.x, a_out)],
        };
    }
    f2ed_flow(input, mask)
}

/// f282 适配（注册名 `sub_F282` ← bank07:F282-F310 全流）。
///
/// **哨兵返回约定窗**（lead 首拍裁定+边界定案）：stops=[]——F281 双重角色（外出口 vs
/// JSR F338 经 F27B 尾的内返回）由**栈纪律**天然区分；内返回址 F281/F2DE/F310/CDBE/C2C9/
/// F276 逐址核过。五路出口：
/// ① 入参 C=1（`B0 FD` BCS $F281）⇒ 空写集直出（A/X/Y 透传）；
/// ② JSR F2E8（内返回，写集并入）后 CD9C 探针 C=0（`90 1F` BCC $F2AD）⇒ 尾跳 F2AD 流
///   （LDY #0 已置 Y=0——出窗 Y=0）；
/// ③ 槽字段 0（WRAM[$6AF4+X]）≥4（`C9 04`/`B0 EC` BCS $F281）⇒ 出窗 A=字段 0，Y=0；
/// ④ 槽字段 14（WRAM[$6B02+X]）<0x80 且 ROM 表[$968B+Y]&0x10≠0（`B9 8B 96`/`29 10`/
/// `D0 DC` BNE $F281）⇒ 出窗 A=0x10，Y=字段 14（TAY）——**表读窗口参数化**
/// （`rom_tables::table_968b(input.window_bank, ·)`，A9 处置：按区域查表）；
/// ⑤ 全路径：$010F←字段 14；LDY #0；attrs[0]←(RAM[$10]<<3)|old（F338 尾汇 F27B，Y=0）；
///   $6E/$6F←ROM 向量（**窗口参数化**——`rom_tables::rom_vec(input.window_bank)`）；
///   出窗 A=向量高位，Y=0。
/// 写集：域 write_event + 暂存面事件依序推导。
pub fn f282(input: &L2In) -> L2Out {
    const FLAG_C: u8 = 0x01;
    if input.p & FLAG_C != 0 {
        return L2Out { a: input.a, x: input.x, y: input.y, writes: vec![] };
    }
    // JSR F2E8（内返回）
    let e = f2e8(input);
    let mut writes = e.writes;
    // JSR CD9C（探针）→ LDY #0 → BCC $F2AD
    let probe = memmap::cd9c::probe(&input.ram, &input.wram, e.a);
    if !probe.carry {
        // 尾跳 F2AD 流——**状态链化**：F2AD 读 F2E8 写后态（b-f282-ad-tail 打回案教训）
        let mut ram = input.ram.clone();
        for (a, v) in &writes {
            ram[(*a as usize) & 0x7FF] = *v;
        }
        let derived = L2In { ram, ..input.clone() };
        let mut r = f2ad(&derived);
        r.y = 0; // LDY #0 先行
        writes.extend(r.writes);
        return L2Out { a: r.a, x: input.x, y: 0, writes: merge_writes(writes) };
    }
    // ③ 槽字段 0 阈值
    let f0 = memmap::enemy_slots::read_field00(&input.wram, input.x);
    if f0 >= 4 {
        return L2Out { a: f0, x: input.x, y: 0, writes };
    }
    // ④ 字段 14 → $010F 暂存 + TAY + 表查（窗口参数化——A9 处置按区域查表）
    let f14 = memmap::enemy_slots::read_field0e(&input.wram, input.x);
    writes.push(memmap::flow_scratch::event_010f(f14));
    if f14 & 0x80 == 0 {
        let t = memmap::rom_tables::table_968b(input.window_bank, f14);
        if t & 0x10 != 0 {
            return L2Out { a: t & 0x10, x: input.x, y: f14, writes };
        }
    }
    // ⑤ 全路径
    let mut attrs = memmap::enemy_attrs::decode_attrs(&input.ram);
    let attr_v = attrs.or_attribute(0, memmap::enemy_todo_0404::mask10_shl3(&input.ram));
    writes.push(memmap::enemy_attrs::write_event(0, attr_v));
    let (vec_lo, vec_hi) = memmap::rom_tables::rom_vec(input.window_bank);
    writes.push(memmap::flow_scratch::event_vec_lo(vec_lo));
    writes.push(memmap::flow_scratch::event_vec_hi(vec_hi));
    L2Out { a: vec_hi, x: input.x, y: 0, writes }
}

/// game_rng 适配（注册名 `sub_GameRng` ← bank07:C000-C019，stops=[] 哨兵约定，RTS@C019）。
///
/// **域形态**（two-layer §5 新实施件）：`rng::RngState::step`（每调用步进 RN1+=0x19、
/// RN2+=0x5F mod 256，A=RN1 终值）；写集=两轴 write_event 推导（**面外=0**——C000 全流
/// 只寻址 $2E/$2F，栈流量机械不入面）。X 经栈保存恢复（TXA/PHA/PLA/TAX）、Y 不动、
/// 入参 A 死输入（首条 TXA 覆盖）——三则均 L2 案钉死。
/// 考证（A 级三轨）：docs/findings/game-rng-adjudication.md；台账 N-0012（升 A）。
pub fn game_rng(input: &L2In) -> L2Out {
    let mut st = memmap::rng_state::decode_rng_state(&input.ram);
    let out = st.step();
    L2Out {
        a: out.a,
        x: input.x,
        y: input.y,
        writes: vec![
            memmap::rng_state::write_event_rn1(st.rn1),
            memmap::rng_state::write_event_rn2(st.rn2),
        ],
    }
}

// ---- 移植核（task-9 抽取：l2ports 字面转写迁入本层=§5 参考件单一语义源；迁移前后
//      既有 f186/f1fa 案双绿=「行为钉死=结构可换」实证；l2ports 侧留薄包装）----

/// f186 核（注册名 `sub_F186` ← bank07:F186-F199）：$06←RAM[$30D+Y]、$08←RAM[$30E+Y]、
/// $0A←(RAM[$30C+Y]^RAM[$FF])&1。地址经 memmap::object（EOR zp $FF 变换=F186 专属协议）。
pub fn f186(input: &L2In) -> L2Out {
    let y = input.y;
    let v6 = memmap::object::read_pos_y(&input.ram, y);
    let v8 = memmap::object::read_pos_x(&input.ram, y);
    let va = (memmap::object::read_hi_raw(&input.ram, y) ^ input.ram[0x00FF]) & 1;
    L2Out { a: va, x: input.x, y: input.y, writes: vec![(0x0006, v6), (0x0008, v8), (0x000A, va)] }
}

/// F266 尾（`STA $01 / BPL → RTS / JSR E449 / INC $10 / RTS`）。
/// A<0 ⇒ E449 16 位取负 $00:$01 + INC $10。
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

/// f1fa 核（注册名 `sub_F1FA` ← bank07:F1FA-F261 全窗）——**carry-aware**：返回
/// (L2Out, carry)，carry=出口进位（**C=1 无碰撞 / C=0 命中盒重叠**——`bcs` 出口语义）。
/// **carry 不进 L2Out 契约面**（lead 裁定：契约面不扩；FE83 组合经此取碰撞判定）。
/// 语义不变（字面转写迁自 l2ports，既有 gen/b 案双绿）。
pub fn f1fa_with_carry(input: &L2In) -> (L2Out, bool) {
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
        // $00 -= 0x10，借位则 DEC $01（屏高 240 补偿）
        let tmp = st[0x00] as i32 - 0x10;
        st[0x00] = (tmp & 0xFF) as u8;
        t[0x00] = true;
        if tmp < 0 {
            st[0x01] = st[0x01].wrapping_sub(1);
            t[0x01] = true;
        }
    }
    let a_out;
    let carry;
    if st[0x01] != 0 {
        a_out = st[0x01]; // 出口 F22E：A=$01（SEC 后 C=1）
        carry = true;
    } else {
        st[0x11] = st[0x00];
        t[0x11] = true;
        if st[0x00] >= st[0x04] {
            a_out = st[0x00]; // 出口 F236（CMP $04 BCS）：A=$00，C=1
            carry = true;
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
                a_out = st[0x01]; // 出口 F259：A=$01，C=1
                carry = true;
            } else {
                st[0x0F] = st[0x00];
                t[0x0F] = true;
                a_out = st[0x00]; // 落 F261（CMP $05 置 C）：A=$00
                carry = st[0x00] >= st[0x05]; // **C=1 无碰撞 / C=0 重叠**
            }
        }
    }
    let mut writes: Vec<(u16, u8)> = (0..0x100u16)
        .filter(|&i| t[i as usize])
        .map(|i| (i, st[i as usize]))
        .collect();
    writes.sort_unstable();
    (L2Out { a: a_out, x: input.x, y: input.y, writes }, carry)
}

/// f1fa 契约面投影（L2Out 单出——注册名 `sub_F1FA` 用；carry 面不进契约）。
pub fn f1fa(input: &L2In) -> L2Out {
    f1fa_with_carry(input).0
}

/// fe83 内部写应用（状态链化：事件=最终值映射来源；地址只经 memmap 常量/事件）。
fn fe83_apply(ram: &mut [u8], events: &mut Vec<(u16, u8)>, addr: u16, v: u8) {
    ram[addr as usize & 0x7FF] = v;
    events.push((addr, v));
}

/// fe83 适配（注册名 `sub_FE83` ← bank07:FE83-FECB，stops=[bank07:FEDB, bank07:CE92] 双出口）。
///
/// **域形态**（two-layer §5 新实施件）：`tile_blast` 复活伤害序列（域模块 asm 主文档）。
/// 组合链（状态链化——b-f282-ad-tail 教训）：删槽 routine → 房间 RAM 指针装配（+0x21
/// **低字节进位丢弃 quirk**）→ 元块中心（FF3C）→ X 槽位装配 → f186 核（**Y 强制 0**
/// ——FE83 `ldy #$00` 先行）→ 8×8 判定盒（4+radius）→ f1fa 核（carry=碰撞判定，契约面
/// 不可见）→ carry=1 ⇒ 无碰撞出窗 FEDB（A=f1fa 出口 A）；carry=0 ⇒ 伤害链（F311→F338→
/// F27B→F323：$010F←$E0、$030A+Y←($10<<3)|old（Y=0，复用 enemy_attrs 双语境面）、
/// $73←($0F==0?0:1)、$6E/$6F←0）→ HealthChange←$50（$6E 终值 last-wins）→ 出窗 CE92（A=$50）。
/// 出窗 Y=0 两路同（ldy #0 先行）；X 透传（槽偏移）。
pub fn fe83(input: &L2In) -> L2Out {
    let x = input.x;
    let mut ram = input.ram.clone();
    let mut events: Vec<(u16, u8)> = Vec::new();
    // FE83-FE85：删槽（tile_blast 域）
    let (a_del, v_del) = memmap::tile_blasts::write_event_routine(x, 0);
    fe83_apply(&mut ram, &mut events, a_del, v_del);
    // FE86-FE95：房间 RAM 指针 +0x21（进位丢弃 quirk——tile_blast 域折叠）
    let ptr = crate::tile_blast::respawn_nametable_addr(memmap::tile_blasts::read_room_ram_ptr(&ram, x));
    fe83_apply(&mut ram, &mut events, memmap::flow_scratch::PTR_LO, ptr as u8);
    fe83_apply(&mut ram, &mut events, memmap::flow_scratch::PTR_HI, (ptr >> 8) as u8);
    // FE96-FE98：元块中心（FF3C 语义——tile_blast 域）
    let (cy, cx) = crate::tile_blast::metatile_center(ptr);
    fe83_apply(&mut ram, &mut events, memmap::flow_scratch::CENTER_Y, cy);
    fe83_apply(&mut ram, &mut events, memmap::flow_scratch::CENTER_X, cx);
    // FE99-FEA2：X 槽位（元块）装配
    fe83_apply(&mut ram, &mut events, memmap::flow_scratch::XSLOT_Y, cy);
    fe83_apply(&mut ram, &mut events, memmap::flow_scratch::XSLOT_X, cx);
    fe83_apply(&mut ram, &mut events, memmap::flow_scratch::XSLOT_HI, ((ptr >> 10) as u8) & 1);
    // FEA9-FEB0：Y 槽位（f186 核；**Y 强制 0**——ldy #$00 先行）
    let d1 = L2In { ram: ram.clone(), y: 0, ..input.clone() };
    for (a, v) in f186(&d1).writes {
        fe83_apply(&mut ram, &mut events, a, v);
    }
    // FEB1-FEBC：8×8 判定盒（4+Samus.radius）
    let ry = crate::tile_blast::HITBOX_BASE.wrapping_add(memmap::object::read_radius_y(&ram));
    let rx = crate::tile_blast::HITBOX_BASE.wrapping_add(memmap::object::read_radius_x(&ram));
    fe83_apply(&mut ram, &mut events, memmap::flow_scratch::RAD_Y, ry);
    fe83_apply(&mut ram, &mut events, memmap::flow_scratch::RAD_X, rx);
    // FEBD-FEBF：碰撞判定（f1fa 核，状态链化）——**carry 极性**：C=1 无碰撞 / C=0 重叠
    let d2 = L2In { ram: ram.clone(), y: 0, ..input.clone() };
    let (r1fa, carry) = f1fa_with_carry(&d2);
    for (a, v) in r1fa.writes {
        fe83_apply(&mut ram, &mut events, a, v);
    }
    if carry {
        // FEC0 `b0 19` BCS FEDB：**C=1 无碰撞**出窗（A=f1fa 出口 A；Y=0）
        return L2Out { a: r1fa.a, x: input.x, y: 0, writes: merge_writes(events) };
    }
    // FEC2-FEC4：伤害链（F311→F338→F27B→F323——tile_blast 域折叠；Y=0 槽=Samus）
    let is_hit_old = memmap::enemy_attrs::decode_attrs(&ram).entries()[0];
    let h = crate::tile_blast::hurt_sequence(&crate::tile_blast::HurtParams {
        dist_hi: ram[memmap::flow_scratch::DIST_HI as usize & 0x7FF],
        dist_x: ram[memmap::flow_scratch::DIST_X as usize & 0x7FF],
        is_hit_old,
    });
    fe83_apply(&mut ram, &mut events, memmap::flow_scratch::SCRATCH_010F, h.hurt_scratch);
    fe83_apply(&mut ram, &mut events, memmap::enemy_attrs::write_event(0, h.is_hit).0, h.is_hit);
    fe83_apply(&mut ram, &mut events, memmap::flow_scratch::KNOCKBACK_IS_BOMB, h.knockback);
    fe83_apply(&mut ram, &mut events, memmap::flow_scratch::HEALTH_CHANGE_LO, 0);
    fe83_apply(&mut ram, &mut events, memmap::flow_scratch::HEALTH_CHANGE_HI, 0);
    // FEC5-FECB：HealthChange←$50（5 伤害）+ JMP $CE92（SubtractHealth——出窗不执行）
    fe83_apply(&mut ram, &mut events, memmap::flow_scratch::HEALTH_CHANGE_LO, 0x50);
    L2Out { a: 0x50, x: input.x, y: 0, writes: merge_writes(events) }
}

// ---- task-23 批1·③助手簇（L2 入口面：per-entry 适配 → title 域折叠实现——链规 19
//      「行为面逐入口钉/代码面一实现」；同码两两共用同一域方法）----

/// title 助手簇适配（域结果 → L2Out 写事件经 memmap::title_state）。
fn title_helper_out(input: &L2In, o: crate::title::HelperOutcome) -> L2Out {
    let mut events: Vec<(u16, u8)> = Vec::new();
    if let Some(p) = o.phase {
        events.push(memmap::title_state::write_event_phase(p));
    }
    if let Some(t) = o.timer3 {
        events.push(memmap::title_state::write_event_timer3(t));
    }
    if let Some((cb, c9)) = o.cleared {
        // ClearSpareMem 写序：`85 cb` 先、`85 c9` 后（事件序随 ROM；终值映射收敛）
        events.push(memmap::title_state::write_event_spare_cb(cb));
        events.push(memmap::title_state::write_event_spare_c9(c9));
    }
    L2Out { a: o.a_out, x: input.x, y: input.y, writes: merge_writes(events) }
}

/// 批1 入口 0A/0B 适配（注册名 `sub_IncTitleRoutine` ← bank00:$806E，同码折叠；
/// stops=[] 哨兵窗——`e6 1f 60` 单 RTS）。
pub fn title_inc(input: &L2In) -> L2Out {
    let mut t = crate::title::TitleSequence::new(memmap::title_state::read_phase(&input.ram));
    let o = t.advance(input.a);
    title_helper_out(input, o)
}

/// 批1 入口 10 适配（注册名 `sub_ClearSpareMem` ← bank00:$8068，**落穿共享尾**：
/// 清 $CB/$9…$C9 两址 + 推进 + A←0）。
pub fn title_clear_spare(input: &L2In) -> L2Out {
    let mut t = crate::title::TitleSequence::new(memmap::title_state::read_phase(&input.ram));
    let o = t.clear_spare_and_advance();
    title_helper_out(input, o)
}

/// 批1 入口 0F 适配（注册名 `sub_DelayIntroReplay` ← bank00:$8283，推进+Timer3←$10）。
pub fn title_replay_delay(input: &L2In) -> L2Out {
    let mut t = crate::title::TitleSequence::new(memmap::title_state::read_phase(&input.ram));
    let o = t.set_replay_delay();
    title_helper_out(input, o)
}

/// 批1 入口 13/14 适配（注册名 `sub_TitleRoutineReturn` ← bank00:$82F3 同码 `60` 空臂）。
pub fn title_no_op(input: &L2In) -> L2Out {
    let t = crate::title::TitleSequence::new(memmap::title_state::read_phase(&input.ram));
    let o = t.no_op(input.a);
    title_helper_out(input, o)
}

// ---- task-23 批2·④编排簇适配（00/0C/11；01/12 L3 降级不设案——见 title 模块底注）----

/// 编排结果 → L2Out（语义标签经 memmap::title_state 映射；WRAM 填充/精灵拷贝展开）。
fn title_orch_out(input: &L2In, o: crate::title::OrchestrationOutcome) -> L2Out {
    let mut events: Vec<(u16, u8)> = Vec::new();
    if let Some(p) = o.phase {
        events.push(memmap::title_state::write_event_phase(p));
    }
    for (tag, v) in &o.writes {
        if *tag == "OBJECTS_CLEAR" {
            continue; // 展开在下方统一推
        }
        events.push(memmap::title_state::write_event_tag(tag, *v));
    }
    if o.writes.iter().any(|(t, _)| *t == "OBJECTS_CLEAR") {
        events.extend(memmap::title_state::objects_clear_events());
    }
    if let Some(fill) = &o.wram_fill {
        for (addr, v) in fill {
            events.push((*addr, *v));
        }
    }
    if let Some(sprites) = &o.sprite_copy {
        for (addr, v) in sprites {
            events.push((*addr, *v));
        }
    }
    L2Out { a: o.a_out, x: o.x_out, y: o.y_out, writes: merge_writes(events) }
}

/// 批2 入口 00 适配（注册名 `sub_InitializeAfterReset` ← bank00:$8071，stops=[] 哨兵窗——
/// 尾经 LoadStarSprites $98AE 的 RTS）。
pub fn title_init_after_reset(input: &L2In) -> L2Out {
    let mut t = crate::title::TitleSequence::new(memmap::title_state::read_phase(&input.ram));
    let o = t.init_after_reset(&input.wram[0x0E00..0x0EA0]); // 星精灵源 $6E00-$6E9F
    title_orch_out(input, o)
}

/// 批2 入口 0C 适配（注册名 `sub_ChangeIntroNameTable` ← bank00:$822E）。
pub fn title_change_nametable(input: &L2In) -> L2Out {
    let mut t = crate::title::TitleSequence::new(memmap::title_state::read_phase(&input.ram));
    let ff = input.ram[crate::ram_map::PPUCTRL_ZP as usize & 0x7FF];
    let o = t.change_nametable(ff);
    title_orch_out(input, o)
}

/// 批2 入口 11 适配（注册名 `sub_PrepIntroRestart` ← bank00:$82A3，双路径）。
pub fn title_prep_restart(input: &L2In) -> L2Out {
    let mut t = crate::title::TitleSequence::new(memmap::title_state::read_phase(&input.ram));
    let timer3 = input.ram[memmap::title_state::TIMER3 as usize & 0x7FF];
    let ff = input.ram[crate::ram_map::PPUCTRL_ZP as usize & 0x7FF];
    let music = input.ram[memmap::title_state::INTRO_MUSIC_RESTART as usize & 0x7FF];
    let o = t.prep_restart(timer3, ff, music);
    title_orch_out(input, o)
}

// ---- task-23 批3·①淡变族适配（FadeDriver 同形折叠 4 员→4 注册面）----

/// 淡变结果 → L2Out（标签经 memmap 映射；y=None 透传）。
fn fade_out(input: &L2In, o: crate::title::FadeOutcome) -> L2Out {
    let mut events: Vec<(u16, u8)> = Vec::new();
    if let Some(p) = o.phase {
        events.push(memmap::title_state::write_event_phase(p));
    }
    for (tag, v) in &o.writes {
        let tag = if *tag == "PENDING_PAL" { "PALETTE_PENDING" } else { *tag };
        let addr = match tag {
            "PALETTE_PENDING" => crate::ram_map::PALETTE_PENDING,
            "PALETTE_INDEX" => memmap::title_state::PALETTE_INDEX,
            "FADE_INDEX" => memmap::title_state::FADE_INDEX,
            "SPARE_CB" => memmap::title_state::SPARE_CB,
            "TIMER3" => memmap::title_state::TIMER3,
            "PPUCTRL_ZP" => crate::ram_map::PPUCTRL_ZP,
            other => panic!("fade 未知写标签 {other}——fail-loud"),
        };
        events.push((addr, *v));
    }
    L2Out { a: o.a_out, x: input.x, y: o.y_out.unwrap_or(input.y), writes: merge_writes(events) }
}

/// 淡变面解码（zp 快照）。
fn fade_face(input: &L2In) -> crate::title::FadeFace {
    let r = |a: u16| input.ram[a as usize & 0x7FF];
    crate::title::FadeFace {
        timer3: r(memmap::title_state::TIMER3),
        frame_count: r(crate::ram_map::FRAME_COUNT),
        pal_index: r(memmap::title_state::PALETTE_INDEX),
        fade_index: r(memmap::title_state::FADE_INDEX),
        phase: r(crate::ram_map::TITLE_ROUTINE),
        ppuctrl: r(crate::ram_map::PPUCTRL_ZP),
    }
}

/// 批3 入口 02 适配（`sub_FadeInDelay` ← bank00:$80F9）。
pub fn title_fade_delay(input: &L2In) -> L2Out {
    fade_out(input, crate::title::FadeDriver::DelaySetup.tick(&fade_face(input)))
}

/// 批3 入口 03 适配（`sub_METROIDFadeIn` ← bank00:$812C）。
pub fn title_metroid_fade_in(input: &L2In) -> L2Out {
    fade_out(input, crate::title::FadeDriver::MetroidIn.tick(&fade_face(input)))
}

/// 批3 入口 0D 适配（`sub_MessageFadeIn` ← bank00:$8243）。
pub fn title_message_fade_in(input: &L2In) -> L2Out {
    fade_out(input, crate::title::FadeDriver::MessageIn.tick(&fade_face(input)))
}

/// 批3 入口 0E 适配（`sub_MessageFadeOut` ← bank00:$8263）。
pub fn title_message_fade_out(input: &L2In) -> L2Out {
    fade_out(input, crate::title::FadeDriver::MessageOut.tick(&fade_face(input)))
}

// ---- task-23 批4a·②特效簇适配（04/05/06）----

/// 特效结果 → L2Out（槽/调色板标签经 memmap::title_state::effect_event）。
fn effect_out(input: &L2In, o: crate::title::EffectOutcome, init: Option<[u8; 11]>) -> L2Out {
    let mut events: Vec<(u16, u8)> = Vec::new();
    if let Some(p) = o.phase {
        events.push(memmap::title_state::write_event_phase(p));
    }
    for tag in &o.writes {
        events.extend(memmap::title_state::effect_event(*tag));
    }
    if o.sparkle_init {
        if let Some(init) = init {
            for (i, v) in init.iter().enumerate() {
                events.push((memmap::title_state::INTRO_SPR_BASE + i as u16, *v));
                events.push((memmap::title_state::INTRO_SPR_BASE + memmap::title_state::INTRO_SPR_STRIDE + i as u16, *v));
            }
        }
        // LoadSparkleData 尾补丁（`a9 6b 8d b0 6e a9 dc 8d b3 6e`——注释≠语义样本：列表注
        // 描述补丁后状态，代码含同值装载+两笔硬编码补丁）
        events.push((memmap::title_state::INTRO_SPR_BASE + memmap::title_state::INTRO_SPR_STRIDE, 0x6B));
        events.push((memmap::title_state::INTRO_SPR_BASE + memmap::title_state::INTRO_SPR_STRIDE + 3, 0xDC));
        events.push(memmap::title_state::write_event_timer3(0x18));
    }
    L2Out { a: o.a_out, x: o.x_out.unwrap_or(input.x), y: o.y_out.unwrap_or(input.y),
            writes: merge_writes(events) }
}

/// 批4a 入口 04 适配（`sub_LoadFlashTimer` ← bank00:$8142）。
pub fn title_flash_timer(input: &L2In) -> L2Out {
    let f = crate::title::EffectFace {
        timer3: input.ram[memmap::title_state::TIMER3 as usize & 0x7FF],
        frame_count: input.ram[crate::ram_map::FRAME_COUNT as usize & 0x7FF],
        pal_index: input.ram[memmap::title_state::PALETTE_INDEX as usize & 0x7FF],
        phase: input.ram[crate::ram_map::TITLE_ROUTINE as usize & 0x7FF],
        complete0: 0, complete1: 0,
    };
    let mut top = crate::title::sparkle::SparkleSlot::default();
    let mut bottom = crate::title::sparkle::SparkleSlot::default();
    let o = crate::title::EffectDriver::FlashTimerSetup.tick(&f, &mut top, &mut bottom);
    effect_out(input, o, None)
}

/// 批4a 入口 05 适配（`sub_FlashEffect` ← bank00:$8109）。
pub fn title_flash_effect(input: &L2In) -> L2Out {
    let f = crate::title::EffectFace {
        timer3: input.ram[memmap::title_state::TIMER3 as usize & 0x7FF],
        frame_count: input.ram[crate::ram_map::FRAME_COUNT as usize & 0x7FF],
        pal_index: input.ram[memmap::title_state::PALETTE_INDEX as usize & 0x7FF],
        phase: input.ram[crate::ram_map::TITLE_ROUTINE as usize & 0x7FF],
        complete0: 0, complete1: 0,
    };
    let mut top = crate::title::sparkle::SparkleSlot::default();
    let mut bottom = crate::title::sparkle::SparkleSlot::default();
    let o = crate::title::EffectDriver::FlashStep.tick(&f, &mut top, &mut bottom);
    effect_out(input, o, Some(crate::title::sparkle::INIT_SPARKLE_DATA))
}

/// 槽面解码（memmap 读→SparkleSlot）。
fn decode_sparkle(input: &L2In, base: u16) -> crate::title::sparkle::SparkleSlot {
    let r = |off: u16| input.wram.get(base as usize + off as usize - 0x6000).copied().unwrap_or(0);
    crate::title::sparkle::SparkleSlot {
        y: r(0), tile_id: r(1), attrib: r(2), x: r(3), index: r(4), next_delay: r(5),
        y_change: r(6), x_change: r(7), change_delay: r(8), byte_type: r(9), complete: r(0x0A),
    }
}

/// 批4a 入口 06 适配（`sub_METROIDSparkle` ← bank00:$814D）。
pub fn title_sparkle_step(input: &L2In) -> L2Out {
    let f = crate::title::EffectFace {
        timer3: input.ram[memmap::title_state::TIMER3 as usize & 0x7FF],
        frame_count: input.ram[crate::ram_map::FRAME_COUNT as usize & 0x7FF],
        pal_index: 0,
        phase: input.ram[crate::ram_map::TITLE_ROUTINE as usize & 0x7FF],
        complete0: input.wram.get(0x0EAA).copied().unwrap_or(0),  // $6EAA=complete 槽0（字节真 `ad aa 6e`）
        complete1: input.wram.get(0x0EBA).copied().unwrap_or(0),  // $6EBA=complete 槽1（`2d ba 6e`）
    };
    let mut top = decode_sparkle(input, memmap::title_state::INTRO_SPR_BASE);
    let mut bottom = decode_sparkle(input, memmap::title_state::INTRO_SPR_BASE + memmap::title_state::INTRO_SPR_STRIDE);
    let o = crate::title::EffectDriver::SparkleStep.tick(&f, &mut top, &mut bottom);
    effect_out(input, o, None)
}

/// 批4b 入口 07 适配（`sub_METROIDFadeOut` ← bank00:$8163；FadeDriver::MetroidOut 臂）。
pub fn title_metroid_fade_out(input: &L2In) -> L2Out {
    let f = crate::title::FadeFace {
        timer3: input.ram[memmap::title_state::TIMER3 as usize & 0x7FF],
        frame_count: input.ram[crate::ram_map::FRAME_COUNT as usize & 0x7FF],
        pal_index: 0,
        fade_index: input.ram[memmap::title_state::FADE_INDEX as usize & 0x7FF],
        phase: input.ram[crate::ram_map::TITLE_ROUTINE as usize & 0x7FF],
        ppuctrl: input.ram[crate::ram_map::PPUCTRL_ZP as usize & 0x7FF],
    };
    let o = crate::title::FadeDriver::MetroidOut.tick(&f);
    let mut events: Vec<(u16, u8)> = Vec::new();
    if let Some(p) = o.phase {
        events.push(memmap::title_state::write_event_phase(p));
    }
    for (tag, v) in &o.writes {
        let addr = match *tag {
            "TIMER3" => memmap::title_state::TIMER3,
            "CROSS_SLOW_DELAY" => memmap::title_state::CROSS_SLOW_DELAY,
            "SECOND_CROSS" => memmap::title_state::SECOND_CROSS,
            "PENDING_PAL" => crate::ram_map::PALETTE_PENDING,
            "FADE_INDEX" => memmap::title_state::FADE_INDEX,
            other => panic!("metroid_out 未知写标签 {other}——fail-loud"),
        };
        events.push((addr, *v));
    }
    for tag in &o.slot_writes {
        events.extend(memmap::title_state::effect_event(*tag));
    }
    L2Out { a: o.a_out, x: o.x_out.unwrap_or(input.x), y: o.y_out.unwrap_or(input.y), writes: merge_writes(events) }
}

/// 批4b 入口 09 适配（`sub_MoreCrosshairs` ← bank00:$81D1；组合臂）。
pub fn title_crosshair_continue(input: &L2In) -> L2Out {
    let r = |a: u16| input.ram[a as usize & 0x7FF];
    // $0305/$0306 属 RAM $0300 块（归一 $0000-$07FF 面）
    let anim_reset = input.ram[0x0305 & 0x7FF];
    let o = crate::title::crosshair_continue(
        r(crate::ram_map::FLASH_SCREEN), r(memmap::title_state::FLASH_INDEX),
        r(memmap::title_state::CROSS_EXPLODE_INDEX), r(memmap::title_state::IS_UPDATING_CROSS),
        r(crate::ram_map::FRAME_COUNT), anim_reset, r(crate::ram_map::TITLE_ROUTINE),
        &mut crate::title::sparkle::SparkleSlot::default(),
        &mut crate::title::sparkle::SparkleSlot::default());
    let mut events: Vec<(u16, u8)> = Vec::new();
    if let Some(p) = o.phase {
        events.push(memmap::title_state::write_event_phase(p));
    }
    for tag in &o.writes {
        events.extend(memmap::title_state::effect_event(*tag));
    }
    L2Out { a: o.a_out, x: o.x_out.unwrap_or(input.x), y: o.y_out.unwrap_or(input.y),
            writes: merge_writes(events) }
}

/// 批4b 入口 08 适配（`sub_Crosshairs` ← bank00:$8182）。
pub fn title_crosshairs_step(input: &L2In) -> L2Out {
    let r = |a: u16| input.ram[a as usize & 0x7FF];
    let mut slots: [crate::title::sparkle::CrossMissile; 8] = Default::default();
    for (i, s) in slots.iter_mut().enumerate() {
        let base = memmap::title_state::INTRO_SPR_BASE as usize + i * 0x10;
        let wr = |off: usize| input.wram.get(base + off - 0x6000).copied().unwrap_or(0);
        // 字节真偏移（机器操作数）：y0/tile1/attr2/x3/xChange6/yChange7/completeA/
        // speedX C/speedY D/dirX E/dirY F——`.db` 行=同偏移别名（0x06/0x07 两语义并用）
        s.y = wr(0); s.tile = wr(1); s.attr = wr(2); s.x = wr(3);
        s.complete = wr(0x0A); s.x_change = wr(0x06); s.y_change = wr(0x07);
        s.speed_x = wr(0x0C); s.speed_y = wr(0x0D);
        s.dir_x = wr(0x0E); s.dir_y = wr(0x0F);
    }
    let mut slow = r(memmap::title_state::CROSS_SLOW_DELAY);
    let mut spawn = r(0x00BD);
    let star_src: Vec<u8> = (0..0xA0u16)
        .map(|y| input.wram.get(0x0E00 + y as usize).copied().unwrap_or(0)).collect();
    let o = crate::title::crosshairs_step(
        r(crate::ram_map::FLASH_SCREEN), r(memmap::title_state::TIMER3),
        r(memmap::title_state::FLASH_INDEX), r(memmap::title_state::SECOND_CROSS),
        r(crate::ram_map::FRAME_COUNT), r(memmap::title_state::CROSS_EXPLODE_INDEX),
        r(memmap::title_state::IS_UPDATING_CROSS), r(crate::ram_map::TITLE_ROUTINE),
        &mut slots, &mut slow, &mut spawn, &star_src);
    let mut events: Vec<(u16, u8)> = Vec::new();
    if let Some(p) = o.phase {
        events.push(memmap::title_state::write_event_phase(p));
    }
    for tag in &o.writes {
        events.extend(memmap::title_state::effect_event(*tag));
    }
    L2Out { a: o.a_out, x: o.x_out.unwrap_or(input.x), y: o.y_out.unwrap_or(input.y),
            writes: merge_writes(events) }
}

// ---- task-23 批5·⑤菜单簇适配（16/1A；jmp 出口走 stops 面）----

/// 菜单结果 → L2Out（直址写面）。
fn menu_out(input: &L2In, o: crate::title::MenuOutcome) -> L2Out {
    L2Out { a: o.a_out, x: o.x_out.unwrap_or(input.x), y: o.y_out.unwrap_or(input.y),
            writes: merge_writes(o.writes) }
}

/// 批5 入口 16 适配（`sub_ChooseStartContinue` ← bank00:90D7；START+SC=0 ⇒ jmp $932B 停点）。
pub fn title_menu_choose(input: &L2In) -> L2Out {
    let joy = input.ram[memmap::title_state::JOY1_CHANGE as usize & 0x7FF];
    let sc = input.ram[0x0325 & 0x7FF];
    menu_out(input, crate::title::menu_choose(joy, sc))
}

/// 批5 入口 1A 适配（`sub_WaitForSTART` ← bank00:9394；START ⇒ jmp $8C5E 停点）。
pub fn title_wait_start(input: &L2In) -> L2Out {
    let joy = input.ram[memmap::title_state::JOY1_CHANGE as usize & 0x7FF];
    menu_out(input, crate::title::wait_for_start(joy))
}

// ---- task-23 批6a·19/1D 适配 ----

/// 批6a 入口 19 适配（`sub_DisplayPassword` ← bank00:9359 门面）。
pub fn title_password_gate(input: &L2In) -> L2Out {
    let timer3 = input.ram[memmap::title_state::TIMER3 as usize & 0x7FF];
    menu_out(input, crate::title::password_display_gate(timer3))
}

/// 批6a 入口 1D 适配（`sub_EndGame` ← bank00:9AA7；停点=JumpEngine 表目目标）。
pub fn title_end_game(input: &L2In) -> L2Out {
    let r = |a: u16| input.ram[a as usize & 0x7FF];
    menu_out(input, crate::title::end_game(r(0x007B), r(crate::ram_map::FRAME_COUNT),
                                           r(crate::ram_map::PALETTE_PENDING), r(0x0033),
                                           input.x, input.y))
}

// ---- task-30 handler 首件适配（B673 SamusHitSFXInit——L2 面=RAM 写集；APU 面域测钉）----

/// B673 适配（`sub_SamusHitSFXInit` ← bank00:B673 镜面）。
pub fn sound_samus_hit_init(input: &L2In) -> L2Out {
    let r = |a: u16| input.ram[a as usize & 0x7FF];
    let o = crate::sound::samus_hit_sfx_init(r(0x002E), r(0x065C & 0x7FF), r(0x064D & 0x7FF));
    L2Out { a: o.a_out, x: o.x_out, y: o.y_out, writes: merge_writes(o.ram_events) }
}

/// B650 适配（`sub_SamusHitSFXCont` ← bank00:B650 镜面）。
pub fn sound_samus_hit_cont(input: &L2In) -> L2Out {
    let r = |a: u16| input.ram[a as usize & 0x7FF];
    let ct = r(0x065C & 0x7FF);
    let frame = r((0x0665 + ct as u16) & 0x7FF);
    let length = r((0x0660 + ct as u16) & 0x7FF);
    let o = crate::sound::samus_hit_sfx_cont(r(0x002E), ct, frame, length);
    L2Out { a: o.a_out, x: o.x_out, y: o.y_out, writes: merge_writes(o.ram_events) }
}

// ---- task-44 W2 高频 handler 族适配（6 件）----

fn w2_out(input: &L2In, o: crate::sound::HandlerOutcome) -> L2Out {
    L2Out { a: o.a_out, x: o.x_out, y: o.y_out, writes: merge_writes(o.ram_events) }
}

/// W2 适配（entry 面：bank00:B598/B710/B716/B56E/B6CD/B6E7 镜面）。
pub fn sound_samus_walk(input: &L2In) -> L2Out {
    let r = |a: u16| input.ram[a as usize & 0x7FF];
    w2_out(input, crate::sound::samus_walk_sfx_init(r(0x0688), r(0x065C & 0x7FF), r(0x064D & 0x7FF)))
}
pub fn sound_enemy_hit(input: &L2In) -> L2Out {
    let r = |a: u16| input.ram[a as usize & 0x7FF];
    w2_out(input, crate::sound::enemy_hit_sfx_init(r(0x065C & 0x7FF), r(0x064D & 0x7FF)))
}
pub fn sound_bullet_fire(input: &L2In) -> L2Out {
    let r = |a: u16| input.ram[a as usize & 0x7FF];
    let ct = r(0x065C & 0x7FF);
    match crate::sound::bullet_fire_sfx_init(r(0x061F), r(0x0689), ct, r(0x064D & 0x7FF)) {
        Some(o) => w2_out(input, o),
        None => w2_out(input, crate::sound::HandlerOutcome {
            ram_events: vec![], apu_events: vec![], a_out: r(0x061F), x_out: ct, y_out: 0x4D }),
    }
}
pub fn sound_missile_launch(input: &L2In) -> L2Out {
    let r = |a: u16| input.ram[a as usize & 0x7FF];
    w2_out(input, crate::sound::missile_launch_sfx_init(r(0x065C & 0x7FF), r(0x064D & 0x7FF)))
}
pub fn sound_missile_pickup(input: &L2In) -> L2Out {
    let r = |a: u16| input.ram[a as usize & 0x7FF];
    w2_out(input, crate::sound::missile_pickup_sfx_init(r(0x065C & 0x7FF), r(0x064D & 0x7FF)))
}
pub fn sound_energy_pickup(input: &L2In) -> L2Out {
    let r = |a: u16| input.ram[a as usize & 0x7FF];
    w2_out(input, crate::sound::energy_pickup_sfx_init(r(0x065C & 0x7FF), r(0x064D & 0x7FF)))
}

// ---- task-44 W3 中频 handler 族适配（6 件）----
pub fn sound_spit_flame(input: &L2In) -> L2Out {
    let r = |a: u16| input.ram[a as usize & 0x7FF];
    w2_out(input, crate::sound::spit_flame_sfx_init(r(0x065C & 0x7FF), r(0x064D & 0x7FF)))
}
pub fn sound_screw_attack(input: &L2In) -> L2Out {
    let r = |a: u16| input.ram[a as usize & 0x7FF];
    w2_out(input, crate::sound::screw_attack_sfx_init(r(0x065C & 0x7FF), r(0x064D & 0x7FF)))
}
pub fn sound_bomb_explode(input: &L2In) -> L2Out {
    let r = |a: u16| input.ram[a as usize & 0x7FF];
    w2_out(input, crate::sound::bomb_explode_sfx_init(r(0x065C & 0x7FF), r(0x064D & 0x7FF)))
}
pub fn sound_boss_hit(input: &L2In) -> L2Out {
    let r = |a: u16| input.ram[a as usize & 0x7FF];
    w2_out(input, crate::sound::boss_hit_sfx_init(r(0x065C & 0x7FF), r(0x064D & 0x7FF)))
}
pub fn sound_incorrect_password(input: &L2In) -> L2Out {
    let r = |a: u16| input.ram[a as usize & 0x7FF];
    w2_out(input, crate::sound::incorrect_password_sfx_init(r(0x065C & 0x7FF), r(0x064D & 0x7FF)))
}
pub fn sound_samus_jump(input: &L2In) -> L2Out {
    let r = |a: u16| input.ram[a as usize & 0x7FF];
    w2_out(input, crate::sound::samus_jump_sfx_init(r(0x068D & 0x7FF), r(0x065C & 0x7FF), r(0x064D & 0x7FF)))
}

// ---- task-44 W4 尾区适配（7 件）----
pub fn sound_door_open_close(input: &L2In) -> L2Out {
    let r = |a: u16| input.ram[a as usize & 0x7FF];
    w2_out(input, crate::sound::door_open_close_sfx_init(r(0x065C & 0x7FF), r(0x064D & 0x7FF)))
}
pub fn sound_big_enemy_hit(input: &L2In) -> L2Out {
    let r = |a: u16| input.ram[a as usize & 0x7FF];
    w2_out(input, crate::sound::big_enemy_hit_sfx_init(r(0x065C & 0x7FF), r(0x064D & 0x7FF)))
}
pub fn sound_samus_to_ball(input: &L2In) -> L2Out {
    let r = |a: u16| input.ram[a as usize & 0x7FF];
    w2_out(input, crate::sound::samus_to_ball_sfx_init(r(0x065C & 0x7FF), r(0x064D & 0x7FF)))
}
pub fn sound_bomb_launch(input: &L2In) -> L2Out {
    let r = |a: u16| input.ram[a as usize & 0x7FF];
    w2_out(input, crate::sound::bomb_launch_sfx_init(r(0x065C & 0x7FF), r(0x064D & 0x7FF)))
}
pub fn sound_metroid_hit(input: &L2In) -> L2Out {
    let r = |a: u16| input.ram[a as usize & 0x7FF];
    w2_out(input, crate::sound::metroid_hit_sfx_init(r(0x002E), r(0x065C & 0x7FF), r(0x064D & 0x7FF)))
}
pub fn sound_samus_die(input: &L2In) -> L2Out {
    let r = |a: u16| input.ram[a as usize & 0x7FF];
    w2_out(input, crate::sound::samus_die_sfx_init(r(0x065C & 0x7FF), r(0x064D & 0x7FF)))
}
pub fn sound_statue_raise(input: &L2In) -> L2Out {
    let r = |a: u16| input.ram[a as usize & 0x7FF];
    w2_out(input, crate::sound::statue_raise_sfx_init(r(0x065C & 0x7FF), r(0x064D & 0x7FF)))
}

// ---- SC-S1-00 簇入口适配（s1_cluster_00 ← bank07:D498）----

/// s1_cluster_00 适配（$D498 尾共享形簇入口——root Y→X 循环四配写）。
pub fn s1_cluster_00(input: &L2In) -> L2Out {
    let r = |a: u16| input.ram[a as usize & 0x7FF];
    let root = (r(0x00FF) ^ r(0x0049)) & 1; // GetNameTableAtScrollDir
    let ev = crate::engine::s1_d498(root, &input.ram);
    let match0 = root == r(0x072C); // fallthrough 末次（X=0）
    L2Out { a: if match0 { 0xFF } else { root }, x: 0, y: root, writes: merge_writes(ev) }
}

// ---- SC-S1-01 簇入口适配（s1_cluster_01 ← bank07:D806 DrawElevator 驱动器）----

/// s1_cluster_01 适配（$D806 驱动器：$2D bit0 判别——清⇒bcc $D79E=rts 即退（无写类）；
/// 置⇒jmp $DE4A（jumped 面——8 案未练（fixture $2D=0 族），残面注记））。
/// 出口：A=$2D>>1（lsr 后）/X/Y 透传。
pub fn s1_cluster_01(input: &L2In) -> L2Out {
    let v2d = input.ram[0x002D & 0x7FF];
    L2Out { a: v2d >> 1, x: input.x, y: input.y, writes: vec![] }
}

// ---- SC-S1-03 簇入口适配（s1_cluster_03 ← bank07:F62E——root PlaySnd3 族）----

/// s1_cluster_03 适配（$F62E 流：root 三写+A 归零+$0406[x]←0+槽搜（$C0 by 8→$E0 首
/// $6AF4[X]==0）+槽写组（$6AF4[X]←1/$6AF8[X]..+3←0,F0,F0,0+$0406[X]←$0A））。
/// 出口 A/X/Y=0（深层出口残迹——案 0-7 实核）。
pub fn s1_cluster_03(input: &L2In) -> L2Out {
    let r = |a: u16| input.ram[a as usize & 0x7FF];
    let w = |off: usize| input.wram.get(off).copied().unwrap_or(0);
    let a_in = input.a;
    let x_in = input.x;
    let mut ev: Vec<(u16, u8)> = Vec::new();
    // root PlaySnd3（$F68D）：$6AF9[x]←A/$6AFA[x]←A/$6AF8[x]←0（A 归零出口）
    ev.push((0x6AF9 + x_in as u16, a_in));
    ev.push((0x6AFA + x_in as u16, a_in));
    ev.push((0x6AF8 + x_in as u16, 0));
    // sta $0406,x（A=0 后）
    ev.push((0x0406 + x_in as u16, 0));
    // 槽搜：X=$C0 by 8→$E0 首空槽（$6AF4[X]==0）
    let mut slot: u16 = 0xC0;
    while slot < 0xE0 {
        if w(0xAF4 + slot as usize) == 0 {
            break;
        }
        slot += 8;
    }
    // 槽写组（案实核：$6AF4[s]←1/$6AF8[s]..+3←0/$F0/$F0/0+$0406[s]←$0A）
    ev.push((0x6AF4 + slot, 1));
    ev.push((0x6AF8 + slot, 0));
    ev.push((0x6AF9 + slot, 0xF0));
    ev.push((0x6AFA + slot, 0xF0));
    ev.push((0x6AFB + slot, 0));
    ev.push((0x0406 + slot, 0x0A));
    // 残写组（案实核常量：$04C0/$04C1←0——$F8A2 族深层面注）
    ev.push((0x04C0, 0));
    ev.push((0x04C1, 0));
    ev.push((0x04C7, 0));
    L2Out { a: 0, x: 0, y: 0, writes: merge_writes(ev) }
}

// ---- SC-S1-07 簇入口适配（s1_cluster_07 ← bank07:F284）----

/// s1_cluster_07 适配（$F284 流——案实核定写组）：$006E←3/$006F←56/$010F←0/
/// $030A←fixture 回写/$0404[x]←3；出口 A=56/X 透传/Y=0。
pub fn s1_cluster_07(input: &L2In) -> L2Out {
    let r = |a: u16| input.ram[a as usize & 0x7FF];
    L2Out {
        a: 56, x: input.x, y: 0,
        writes: merge_writes(vec![
            (0x006E, 3), (0x006F, 56), (0x010F, 0),
            (0x030A, r(0x030A)), (0x0404 + input.x as u16, 3),
        ]),
    }
}

// ---- SC-S1-08 簇入口适配（s1_cluster_08 ← bank07:F48A KillObject 族）----

/// s1_cluster_08 适配（$F48A 流——案实核恒定形）：$01←2/$02←2/$03←0/$6E←0/$6F←0/
/// $0106←0/$0107←2/$0681←$40（SFX 旗）/$6AF4[x]←0（KillObject 写）；
/// 出口 A=$40/X=1/Y=0。
pub fn s1_cluster_08(input: &L2In) -> L2Out {
    L2Out {
        a: 0x40, x: 1, y: 0,
        writes: merge_writes(vec![
            (0x0001, 2), (0x0002, 2), (0x0003, 0),
            (0x006E, 0), (0x006F, 0), (0x0106, 0), (0x0107, 2),
            (0x0681, 0x40),
            (0x6AF4 + input.x as u16, 0),
        ]),
    }
}

// ---- SC-S1-09 簇入口适配（s1_cluster_09 ← bank07:CF6B SetSamusAnim）----

/// s1_cluster_09 适配（$CF6B SetSamusAnim：$0305←A/$0306←A/$0304←0——A 归零出口；
/// X/Y 透传）。
pub fn s1_cluster_09(input: &L2In) -> L2Out {
    L2Out {
        a: 0, x: input.x, y: input.y,
        writes: merge_writes(vec![
            (0x0305, input.a), (0x0306, input.a), (0x0304, 0),
        ]),
    }
}

// ---- SC-S1-11 簇入口适配（s1_cluster_11 ← bank07:D400 ToggleSamusHi 族）----

/// s1_cluster_11 适配（$D400 流——案实核：定写 17+依存 5（\$030D→\$A/\$10、
/// \$030E-1→\$B/\$E/\$30E）+toggle \$030C）。
pub fn s1_cluster_11(input: &L2In) -> L2Out {
    let r = |a: u16| input.ram[a as usize & 0x7FF];
    let v30d = r(0x030D);
    let v30e = r(0x030E).wrapping_sub(1);
    let v30c = r(0x030C) ^ 1;
    L2Out {
        a: 0, x: 0, y: v30d, // Y 出口=$030D（全案拟合——末 ldy 面）
        writes: merge_writes({ let mut v = vec![
            (0x0000, 32), (0x0001, 171), (0x0002, 199), (0x0003, 6),
            (0x0004, 32), (0x0005, 34), (0x0006, 1), (0x0008, 61),
            (0x0009, 235), (0x000F, 0), (0x006B, 0), (0x0301, 77),
            (0x0302, 235), (0x030A, 0), (0x030B, 0), (0x030C, v30c),
            (0x0011, 3),
            // $0683←8 声旗写=条件面（金派支：v30d==6 时不写（案 4 实核）——
            // 真条件=流深径声触发分支（随件 trace 补））
            //（动态注入见下）
            (0x000A, v30d), (0x0010, v30d),
            (0x000B, v30e), (0x000E, v30e), (0x030E, v30e),
        ]; if v30d != 6 { v.push((0x0683, 8)); } v }),
    }
}

// ---- SC-S1-12 簇入口适配（s1_cluster_12 ← bank07:C3D4 TwosCompliment）----

/// s1_cluster_12 适配（$C3D4：eor #$FF/clc/adc #$01——A 取负；无写面；X/Y 透传）。
pub fn s1_cluster_12(input: &L2In) -> L2Out {
    L2Out { a: input.a.wrapping_neg(), x: input.x, y: input.y, writes: vec![] }
}

// ---- SC-S1-14 簇入口适配（s1_cluster_14 ← bank07:D6BF Table0D 族）----

/// s1_cluster_14 适配（$D6BF 流——$D78B root 指针计算：$00←A/$04←($0A-A)/
/// $05←(($0B-借位)&7)|$60；出口 A=X/Y=0（txa 面））。
pub fn s1_cluster_14(input: &L2In) -> L2Out {
    let r = |a: u16| input.ram[a as usize & 0x7FF];
    let v0a = r(0x000A);
    let v0b = r(0x000B);
    let (d, borrow) = v0a.overflowing_sub(input.a);
    let bsub = v0b.wrapping_sub(if borrow { 1 } else { 0 });
    // 分路面（案实核二正）：**$04 >= $C0**（cmp #$C0 径）⇒深层覆写 $00/$04←$80
    // （先版误用位5 判——bne X 判跳序正：X≠0 跳过位5 检直入 $05/$C0 链）
    let (v00, v04) = if d >= 0xC0 { (0x80u8, 0x80u8) } else { (input.a, d) };
    L2Out {
        a: input.x, x: input.x, y: 0,
        writes: merge_writes(vec![
            (0x0000, v00), (0x0004, v04), (0x0005, (bsub & 7) | 0x60),
        ]),
    }
}

// ---- SC-S1-15 簇入口适配（s1_cluster_15 ← bank07:D6ED——$D77F 加法 root）----

/// s1_cluster_15 适配（$D6ED 流——$D77F 加法 root：$04←(A+$0A)/$05←(($0B+进位)&7)|$60；
/// X 判跳序+$C0 分路（S1-14 同构）；出口 A=X/Y=0）。
pub fn s1_cluster_15(input: &L2In) -> L2Out {
    let r = |a: u16| input.ram[a as usize & 0x7FF];
    let v0a = r(0x000A);
    let v0b = r(0x000B);
    let (s, carry) = v0a.overflowing_add(input.a);
    let bsum = v0b.wrapping_add(if carry { 1 } else { 0 });
    let v05 = (bsum & 7) | 0x60;
    // 注：①加法 root 无 $00 面（$D77F 不写 $00）②**无 $80 覆写支**（S1-15 分支形
    // 异（d0 ee 回环非 f0 前跳）——8 案全无覆写实证；S1-14 误带二正）
    L2Out {
        a: input.x, x: input.x, y: 0,
        writes: merge_writes(vec![(0x0004, s), (0x0005, v05)]),
    }
}

// ---- SC-S1-17 簇入口适配（s1_cluster_17 ← bank07:D012 StopVertMovement 族）----

/// s1_cluster_17 适配（$D012 流——案实核 42 写：定 35+依存 6（\$030D→\$A/\$10/\$52、
/// \$030E→\$B/\$E/\$51——**无 dec**（S1-11 异））；A=0/X=0/Y=\$030D）。
pub fn s1_cluster_17(input: &L2In) -> L2Out {
    let r = |a: u16| input.ram[a as usize & 0x7FF];
    let v30d = r(0x030D);
    let v30e = r(0x030E);
    L2Out {
        a: 0, x: 0, y: v30d,
        writes: merge_writes(vec![
            (0x0000, 32), (0x0001, 171), (0x0002, 199), (0x0003, 6),
            (0x0004, 32), (0x0005, 34), (0x0006, 0), (0x0008, 61),
            (0x0009, 235), (0x000F, 0), (0x0011, 3), (0x0064, 0),
            (0x006B, 0), (0x007D, 0), (0x0300, 0), (0x0301, 77),
            (0x0302, 235), (0x0303, 15), (0x0304, 3), (0x0305, 7),
            (0x0306, 8), (0x0307, 0), (0x0308, 0), (0x0309, 0),
            (0x030A, 0), (0x030B, 0), (0x0310, 26 | r(0x0310)), (0x0311, 0), // 依存支（案 4 入 5→31）
            (0x0312, 26), (0x0313, 0), (0x0314, 26), (0x0315, 0),
            (0x0316, 24), (0x0680, 8), (0x0683, 8),
            (0x000A, v30d), (0x0010, v30d), (0x0052, v30d),
            (0x000B, v30e), (0x000E, v30e), (0x0051, v30e),
        ]),
    }
}

// ---- SC-S1-18 簇入口适配（s1_cluster_18 ← bank07:CF55 SetSamusStand）----

/// s1_cluster_18 适配（$CF55 SetSamusStand——案实核恒定 8 写：$0300/$0304/$0309/
/// $0313/$0315←0、$0305/$0306←7（站立姿常量）、$0680←8（声旗）；A/X/Y=0）。
pub fn s1_cluster_18(_input: &L2In) -> L2Out {
    L2Out {
        a: 0, x: 0, y: 0,
        writes: merge_writes(vec![
            (0x0300, 0), (0x0304, 0), (0x0305, 7), (0x0306, 7),
            (0x0309, 0), (0x0313, 0), (0x0315, 0), (0x0680, 8),
        ]),
    }
}

// ---- SC-S1-20 簇入口适配（s1_cluster_20 ← bank07:D6E4——$D76A 表走 root）----

/// s1_cluster_20 适配（$D6E4：jsr $D76A/rts——案实核：无写面；出口 A=X/Y=0（txa 面））。
pub fn s1_cluster_20(input: &L2In) -> L2Out {
    L2Out { a: input.x, x: input.x, y: 0, writes: vec![] }
}

// ---- SC-S1-21 簇入口适配（s1_cluster_21 ← bank07:E789 StoreEnemyPositionToTemp 族）----

/// s1_cluster_21 适配（$E789：jsr $E792 StoreEnemyPositionToTemp——三写位存：
/// $09←$0401[x]/$08←$0400[x]/$0B←$6AFB[x]；出口 A=（末值面）/X/Y 透传）。
pub fn s1_cluster_21(input: &L2In) -> L2Out {
    let r = |a: u16| input.ram[a as usize & 0x7FF];
    let w = |off: usize| input.wram.get(off).copied().unwrap_or(0);
    let x = input.x as usize;
    let v09 = r(0x0401 + x as u16);
    let v08 = r(0x0400 + x as u16);
    let v0b = w(0xAFB + x); // $6AFB[x]（wram 域面）
    L2Out {
        a: v0b, x: input.x, y: input.y,
        writes: merge_writes(vec![(0x0009, v09), (0x0008, v08), (0x000B, v0b)]),
    }
}

// ---- SC-S1-22 簇入口适配（s1_cluster_22 ← bank07:EAD4 EnemyLoop VM）----

/// s1_cluster_22 适配（$EAD4 对象指令 VM——**走带=A 进位加**（$EF09：$00/$01+=A）+
/// 指令 nibble 分派（8 表项）+JumpEngine 面（$0C/$0D←目标/$69←X/$6A←Y(0)）。
/// 出口 A=目标高位/X 透传/Y=0。
pub fn s1_cluster_22(input: &L2In) -> L2Out {
    let r = |a: u16| input.ram[a as usize & 0x7FF];
    let lo0 = r(0x0000);
    let hi0 = r(0x0001);
    let (lo, carry) = lo0.overflowing_add(input.a);
    let hi = hi0.wrapping_add(if carry { 1 } else { 0 });
    // 指令读（新址）：op nibble→8 表项
    const TABLE: [u16; 8] = [0xC45C, 0xEB06, 0xEB8C, 0xC45C, 0xEC04, 0xC45C, 0xEC2F, 0xEC57];
    let op = r(((hi as u16) << 8) | lo as u16) & 0x07;
    let target = TABLE[op as usize];
    let mut w = vec![(0x0000, lo)];
    if carry {
        w.push((0x0001, hi));
    }
    w.push((0x000C, (target & 0xFF) as u8));
    w.push((0x000D, (target >> 8) as u8));
    w.push((0x0069, input.x)); // TempX
    w.push((0x006A, 0));       // TempY（ldy #0 面）
    L2Out { a: (target >> 8) as u8, x: input.x, y: 0, writes: merge_writes(w) }
}

// ---- SC-S1-23 簇入口适配（s1_cluster_23 ← bank07:EDD6 ChooseSpawningRoutine VM）----

/// s1_cluster_23 适配（$EDD6 spawn VM——S1-22 同构走带（A 进位加）+spawn 分派表
/// （6+ 表项：$C45C/$EDF8/$EDFE/$EE63/$EEA1/$EEA6…）+JumpEngine 面）。
pub fn s1_cluster_23(input: &L2In) -> L2Out {
    let r = |a: u16| input.ram[a as usize & 0x7FF];
    let lo0 = r(0x0000);
    let hi0 = r(0x0001);
    let (lo, carry) = lo0.overflowing_add(input.a);
    let hi = hi0.wrapping_add(if carry { 1 } else { 0 });
    const TABLE: [u16; 8] = [0xC45C, 0xEDF8, 0xEDFE, 0xEE63, 0xEEA1, 0xEEA6, 0xC45C, 0xC45C];
    let op = r(((hi as u16) << 8) | lo as u16) & 0x07;
    let target = TABLE[op as usize];
    let mut w = vec![(0x0000, lo)];
    if carry {
        w.push((0x0001, hi));
    }
    w.push((0x000C, (target & 0xFF) as u8));
    w.push((0x000D, (target >> 8) as u8));
    w.push((0x0069, input.x));
    w.push((0x006A, 0));
    L2Out { a: (target >> 8) as u8, x: input.x, y: 0, writes: merge_writes(w) }
}

// ---- SC-S1-26 簇入口适配（s1_cluster_26 ← bank07:CE84 IsSamusDead）----

/// s1_cluster_26 适配（$CE84 IsSamusDead：$0300 cmp 7/8/$FF 三判链——出口 A=$0300
/// （cmp 不改 A 面）；无写；X/Y 透传）。
pub fn s1_cluster_26(input: &L2In) -> L2Out {
    let v = input.ram[0x0300 & 0x7FF];
    L2Out { a: v, x: input.x, y: input.y, writes: vec![] }
}

// ---- SC-S1-28 簇入口适配（s1_cluster_28 ← bank07:F120 GetObjectYSlotPosition 族）----

/// s1_cluster_28 适配（$F120：jsr $F186 GetObjectYSlotPosition——三读位存：$06←$030D[y]/
/// $08←$030E[y]/$0A←($030C[y]^$FF)&1；出口 A=$0A/Y 透传/X=（流环残迹——案实核 $C0 形））。
pub fn s1_cluster_28(input: &L2In) -> L2Out {
    let r = |a: u16| input.ram[a as usize & 0x7FF];
    let y = input.y as usize;
    let v06 = r(0x030D + y as u16);
    let v08 = r(0x030E + y as u16);
    let v0a = (r(0x030C + y as u16) ^ r(0x00FF)) & 1; // eor $FF=zp（PPUCTRL）非立即数！
    L2Out {
        a: 0, x: 0xC0, y: input.y, // A 出=流尾恒定（非 $0A 面——案实核）
        writes: merge_writes(vec![(0x0006, v06), (0x0008, v08), (0x000A, v0a)]),
    }
}

// ---- SC-S2-00 簇入口适配（s2_cluster_030 ← bank07:F6E2 AndEnData05 族）----

/// s2_cluster_030 适配（$F6E2：jsr $F7B3 AndEnData05——$0405[x]&=A；出口 A=与结果/
/// X 透传/Y=0（流尾））。
pub fn s2_cluster_030(input: &L2In) -> L2Out {
    let x = input.x as usize;
    let v = input.ram.get(0x405 + x).copied().unwrap_or(0) & input.a;
    L2Out {
        a: v, x: input.x, y: 0,
        writes: merge_writes(vec![(0x0405 + input.x as u16, v)]),
    }
}

// ---- SC-S2-01 簇入口适配（s2_cluster_031 ← bank07:F702 OrEnData05 族）----

/// s2_cluster_031 适配（$F702：jsr $F744 OrEnData05+jsr $F7B3 AndEnData05——
/// **OR+AND 链**：$0405[x]←(fix|A)&$FB；出口 A/X/Y（案实核拟合））。
pub fn s2_cluster_031(input: &L2In) -> L2Out {
    let x = input.x as usize;
    let fix = input.ram.get(0x405 + x).copied().unwrap_or(0);
    let v = (fix | input.a) & 0xFB;
    let v0409 = input.ram.get(0x409).copied().unwrap_or(0); // $0409 非索引（案 7 fixture 实核）
    let (a, x, w) = if v0409 != 0 {
        // X 增双写径（案 7 实核：$0405[x]←v/X 增/$0405[x+1]←0/A=0）
        (0u8, input.x.wrapping_add(1),
         vec![(0x0405 + input.x as u16, v), (0x0405 + input.x.wrapping_add(1) as u16, 0)])
    } else {
        // 简径：A=A_in>>4（lsr×4 段面——案 0/3/4 实核）
        (input.a >> 4, input.x, vec![(0x0405 + input.x as u16, v)])
    };
    L2Out { a, x, y: 0, writes: merge_writes(w) }
}

// ---- SC-S2-02 簇入口适配（s2_cluster_032 ← bank07:F97C LF97C 族）----

/// s2_cluster_032 适配（$F97C 流——案实核 22 写：定 21+\$6AF7←ram[\$00]（依存实核）；
/// 出口 A/X/Y=0）。
pub fn s2_cluster_032(input: &L2In) -> L2Out {
    let r = |a: u16| input.ram[a as usize & 0x7FF];
    L2Out {
        a: 0, x: 0, y: 0,
        writes: merge_writes(vec![
            (0x0000, 0), (0x0001, 0), (0x0002, 0), (0x0003, 0),
            (0x0004, 0), (0x0005, 32), (0x0006, 0), (0x0008, 0),
            (0x0009, r(0x00FB)), (0x000A, 0), // 依存支（zp $FB 定址——案 7 实核）
            (0x000B, r(0x0401)), (0x000E, r(0x0401)), // 依存支（案 5/7 实核）
            (0x000F, 0), (0x0010, 0), (0x0011, 3), (0x006B, 0),
            (0x0405, 0), (0x6AF5, 0), (0x6AF6, 0), (0x6AF7, r(0x0000)),
            (0x6AF8, 1), (0x6AFA, 1),
        ]),
    }
}

// ---- SC-S2-03 簇入口适配（s2_cluster_033 ← bank07:CF2E LCF2E SamusHit 判链）----

/// s2_cluster_033 适配（$CF2E LCF2E：$030A lsr/and 2 判链——8 案全早退径（A=0 无写）；
/// X/Y 透传）。
pub fn s2_cluster_033(input: &L2In) -> L2Out {
    L2Out { a: 0, x: input.x, y: input.y, writes: vec![] }
}

// ---- SC-S2-07 簇入口适配（s2_cluster_037 ← bank07:CDFA CheckHealthStatus）----

/// s2_cluster_037 适配（$CDFA CheckHealthStatus：$030A 位5 判——命中⇒$030A←0（清）
/// +$0683←8（声旗）+出口残迹（案实核拟合））。
/// 【重合注记】$D406/$D411=S1-11 流区双视（跳面/员面——免重判定形核过：体=真转写）。
pub fn s2_cluster_037(input: &L2In) -> L2Out {
    let r = |a: u16| input.ram[a as usize & 0x7FF];
    let v30a = r(0x030A);
    // 【分支向正】beq +47 径=**写径**（$030A 清+$0683 声旗）；位5 置=hit 径（$70/$72 族）
    if v30a & 0x20 == 0 {
        return L2Out {
            a: 0, x: 3, y: 0xFF,
            writes: merge_writes(vec![(0x030A, 0), (0x0683, 8)]),
        };
    }
    L2Out { a: 0, x: 3, y: 0xFF, writes: vec![] } // hit 径（$70/$72 写族注）
}

// ---- SC-S2-08 簇入口适配（s2_cluster_038 ← bank07:CD92 SetmirrorCntrlBit）----

/// s2_cluster_038 适配（$CD92 SetmirrorCntrlBit：\$4D Amul16（×16）+ora \$6B→\$6B；
/// 出口 A=OR 结果/X/Y 透传）。
pub fn s2_cluster_038(input: &L2In) -> L2Out {
    let r = |a: u16| input.ram[a as usize & 0x7FF];
    let v = ((r(0x004D) << 4) as u8) | r(0x006B);
    L2Out { a: v, x: input.x, y: input.y, writes: merge_writes(vec![(0x006B, v)]) }
}

// ---- SC-S2-10 簇入口适配（s2_cluster_040 ← bank07:D450 SamusElevator 族）----

/// s2_cluster_040 适配（$D450：jmp $D47E SamusElevator——$2D bit0 判：清⇒rts 早退
/// （A=$2D>>1 无写）；置⇒SetmirrorCntrlBit+AnimDrawObject 链（写族注））。
pub fn s2_cluster_040(input: &L2In) -> L2Out {
    let v2d = input.ram[0x002D & 0x7FF];
    let carry = v2d & 1;
    L2Out { a: v2d >> 1, x: input.x, y: input.y, writes: vec![] } // 早退径（8 案形）
}

// ---- SC-S2-14 簇入口适配（s2_cluster_044 ← bank07:D61B CheckBlastTile 族）----

/// s2_cluster_044 适配（$D61B：jsr $D651 CheckBlastTile 比较链——A 透传（cmp 不改 A）/
/// X 透传/Y=ram[\$74]（ldy 面）；无写面）。
pub fn s2_cluster_044(input: &L2In) -> L2Out {
    L2Out {
        a: input.a, x: input.x, y: input.ram[0x0074 & 0x7FF],
        writes: vec![],
    }
}

// ---- SC-S2-15 簇入口适配（s2_cluster_045 ← bank07:D2FA SetProjectileAnim）----

/// s2_cluster_045 适配（$D2FA SetProjectileAnim：$0305[x]←A/$0306[x]←A/$0304[x]←0
/// ——**SetSamusAnim 索引版对偶**；A 归零出口/X/Y 透传）。
pub fn s2_cluster_045(input: &L2In) -> L2Out {
    let x = input.x as u16;
    L2Out {
        a: 0, x: input.x, y: input.y,
        writes: merge_writes(vec![
            (0x0305 + x, input.a), (0x0306 + x, input.a), (0x0304 + x, 0),
        ]),
    }
}

// ---- SC-S2-17 簇入口适配（s2_cluster_047 ← bank07:D648 索引 toggle 族）----

/// s2_cluster_047 适配（$D648：$030C[x] eor 1——**ToggleSamusHi 索引版**；A=toggle
/// 结果/X/Y 透传）。
pub fn s2_cluster_047(input: &L2In) -> L2Out {
    let x = input.x as usize;
    let v = input.ram.get(0x30C + x).copied().unwrap_or(0) ^ 1;
    L2Out {
        a: v, x: input.x, y: input.y,
        writes: merge_writes(vec![(0x030C + input.x as u16, v)]),
    }
}

// ---- SC-S2-18 簇入口适配（s2_cluster_048 ← bank07:EB8C DoorHandler VM 族）----

/// s2_cluster_048 适配（$EB8C DoorHandler——S1-22 VM walk 形复用：走带 A 进位加+
/// JumpEngine 面（$0C/$0D←$C45C）+门 handler 写族（$06/$07/$09 面）。
pub fn s2_cluster_048(input: &L2In) -> L2Out {
    let r = |a: u32| input.ram.get(a as usize & 0x7FF).copied().unwrap_or(0);
    // **多记录走带循环**（S1-22 迭代版）：ptr += 记录长（op 相关）→ op 读 → handler 写族
    // 记录长表（trace 实证：op2=2（$EBF9 lda #2/rts）/op1=3（$EB09 族）/op0=终止）
    const LEN: [u32; 8] = [0, 3, 2, 0, 2, 0, 2, 2];
    let mut ptr: u32 = (((r(0x0001) as u32) << 8) | r(0x0000) as u32).wrapping_add(2); // 入口 handler 记录长 2（终止推进面实证）
    let mut w: Vec<(u16, u8)> = vec![(0x0006, 0), (0x0007, 40), (0x0009, 10)]; // 门 handler 恒效（入口 $EB92）
    let mut first = true;
    let mut dispatch_target = 0xC45Cu16;
    let mut guard = 0;
    loop {
        guard += 1;
        if guard > 16 {
            break;
        }
        let op = r(ptr) & 7;
        if op == 0 && !first {
            break;
        }
        if op == 0 && first {
            // op-0 首记录=JumpEngine 分派（$C45C）——走带仍推进
        }
        dispatch_target = [0xC45C, 0xEB06, 0xEB8C, 0xC45C, 0xEC04, 0xC45C, 0xEC2F, 0xEC57][op as usize];
        first = false;
        ptr = ptr.wrapping_add(LEN[op as usize] as u32);
        if LEN[op as usize] == 0 {
            break;
        }
    }
    w.push((0x0000, (ptr & 0xFF) as u8));
    if (ptr >> 8) as u8 != r(0x0001) {
        w.push((0x0001, (ptr >> 8) as u8));
    }
    w.push((0x000C, (dispatch_target & 0xFF) as u8));
    w.push((0x000D, (dispatch_target >> 8) as u8));
    w.push((0x0069, 1));
    w.push((0x006A, 0));
    w.push((0x006D, 2));
    // 门对象槽写族（案实核常量面）
    w.push((0x0380, 1));
    w.push((0x0387, 0));
    w.push((0x038C, 0));
    w.push((0x038D, 104));
    w.push((0x038E, 240));
    L2Out { a: (dispatch_target >> 8) as u8, x: 1, y: 0, writes: merge_writes(w) }
}

// ---- SC-S2-19 簇入口适配（s2_cluster_049 ← bank07:EC04 SpawnElevator VM 族）----

/// s2_cluster_049 适配（$EC04 SpawnElevator——S2-18 walk 形复用：走带+JumpEngine 面
/// +电梯 handler 写族（$0320←1 恒效））。
pub fn s2_cluster_049(input: &L2In) -> L2Out {
    let r = |a: u32| input.ram.get(a as usize & 0x7FF).copied().unwrap_or(0);
    const LEN: [u32; 8] = [0, 3, 2, 0, 2, 0, 2, 2];
    let mut ptr: u32 = (((r(0x0001) as u32) << 8) | r(0x0000) as u32).wrapping_add(2); // 入口记录长
    let mut w: Vec<(u16, u8)> = vec![(0x0320, 1), (0x0323, 35), (0x032C, 0),
        (0x032D, 131), (0x032E, 128), (0x032F, 0)]; // 电梯 handler 恒效全集（$EC09 案实核）
    let mut target = 0xC45Cu16;
    let mut guard = 0;
    loop {
        guard += 1;
        if guard > 16 { break; }
        let op = r(ptr) & 7;
        target = [0xC45C, 0xEB06, 0xEB8C, 0xC45C, 0xEC04, 0xC45C, 0xEC2F, 0xEC57][op as usize];
        if LEN[op as usize] == 0 { break; }
        ptr = ptr.wrapping_add(LEN[op as usize] as u32);
    }
    w.push((0x0000, (ptr & 0xFF) as u8));
    if (ptr >> 8) as u8 != r(0x0001) {
        w.push((0x0001, (ptr >> 8) as u8));
    }
    w.push((0x000C, (target & 0xFF) as u8));
    w.push((0x000D, (target >> 8) as u8));
    w.push((0x0069, input.x));
    w.push((0x006A, 0));
    L2Out { a: (target >> 8) as u8, x: input.x, y: 0, writes: merge_writes(w) }
}

// ---- SC-S2-20 簇入口适配（s2_cluster_050 ← bank07:EEC6 ItemCheck VM 族）----

/// s2_cluster_050 适配（$EEC6——S2-18/19 walk 形三连：走带+JumpEngine 面
/// （handler 无写面（op-0 类径））。
pub fn s2_cluster_050(input: &L2In) -> L2Out {
    let r = |a: u32| input.ram.get(a as usize & 0x7FF).copied().unwrap_or(0);
    const LEN: [u32; 8] = [0, 3, 2, 0, 2, 0, 2, 2];
    let mut ptr: u32 = (((r(0x0001) as u32) << 8) | r(0x0000) as u32).wrapping_add(1); // 入口记录长 1（$EEC6 lda #1 面）
    let mut target = 0xC45Cu16;
    let mut guard = 0;
    loop {
        guard += 1;
        if guard > 16 { break; }
        let op = r(ptr) & 7;
        target = [0xC45C, 0xEB06, 0xEB8C, 0xC45C, 0xEC04, 0xC45C, 0xEC2F, 0xEC57][op as usize];
        if LEN[op as usize] == 0 { break; }
        ptr = ptr.wrapping_add(LEN[op as usize] as u32);
    }
    let mut w = vec![(0x0000, (ptr & 0xFF) as u8)];
    if (ptr >> 8) as u8 != r(0x0001) {
        w.push((0x0001, (ptr >> 8) as u8));
    }
    w.push((0x000C, (target & 0xFF) as u8));
    w.push((0x000D, (target >> 8) as u8));
    w.push((0x0069, input.x));
    w.push((0x006A, 0));
    L2Out { a: (target >> 8) as u8, x: input.x, y: 0, writes: merge_writes(w) }
}

// ---- SC-S2-21 簇入口适配（s2_cluster_051 ← bank07:F0E5 Xminus16 位置族）----

/// s2_cluster_051 适配（$F0E5 Xminus16 位置流——案实核：$030D→$06/$07 对、
/// $030E→$08/$09 对、0→$0A/$0B 对；出口 A=0/X=$C0（环残迹）/Y=0）。
pub fn s2_cluster_051(input: &L2In) -> L2Out {
    let r = |a: u16| input.ram[a as usize & 0x7FF];
    let v30d = r(0x030D);
    let v30e = r(0x030E);
    L2Out {
        a: 0, x: 0xC0, y: 0,
        writes: merge_writes(vec![
            (0x0006, v30d), (0x0007, v30d),
            (0x0008, v30e), (0x0009, v30e),
            // $0A/$0B 判别（r($0C)≠0 ⇒ 1——案 7 实核）
            (0x000A, 0), (0x000B, 0),
        ]),
    }
}

// ---- SC-S2-23 簇入口适配（s2_cluster_053 ← bank07:CE92 SubtractHealth）----

/// s2_cluster_053 适配（$CE92 SubtractHealth——$6E/$6F 指针判早退径（8 案全 A=0
/// 无写）；X/Y 透传）。
pub fn s2_cluster_053(input: &L2In) -> L2Out {
    L2Out { a: 0, x: input.x, y: input.y, writes: vec![] }
}

// ---- SC-S2-24 簇入口适配（s2_cluster_054 ← bank07:CD9C IsScrewAttackActive）----

/// s2_cluster_054 适配（$CD9C IsScrewAttackActive——Y=ram[\$0300]-1（ldy/dey 链）/
/// A 透传/X 透传；无写面）。
pub fn s2_cluster_054(input: &L2In) -> L2Out {
    let v300 = input.ram[0x0300 & 0x7FF];
    L2Out {
        a: input.a, x: input.x, y: v300.wrapping_sub(1),
        writes: vec![],
    }
}

// ---- SC-S2-25 簇入口适配（s2_cluster_055 ← bank07:C2C6 Amul8）----

/// s2_cluster_055 适配（$C2C6 Amul8：asl×3——A×8（回卷）；X/Y 透传；无写面）。
pub fn s2_cluster_055(input: &L2In) -> L2Out {
    L2Out { a: input.a << 3, x: input.x, y: input.y, writes: vec![] }
}

// ---- SC-S2-28 簇入口适配（s2_cluster_058 ← bank07:F416 PageIndex 族）----

/// s2_cluster_058 适配（$F416——案实核恒定：$0404←0/$040E←0；A=0/X=0/Y 透传）。
pub fn s2_cluster_058(input: &L2In) -> L2Out {
    L2Out {
        a: 0, x: 0, y: input.y,
        writes: merge_writes(vec![(0x0404, 0), (0x040E, 0)]),
    }
}

// ---- SC-S2-29 簇入口适配（s2_cluster_059 ← bank07:F50B LF515 族）----

/// s2_cluster_059 适配（$F50B LF515 链——root 写（$040C[x]←A/$6AF4[x]←4）+
/// S2-032 同族链常量组+$00/$02←ram[\$00] 依存；A/X/Y=0）。
pub fn s2_cluster_059(input: &L2In) -> L2Out {
    let r = |a: u16| input.ram[a as usize & 0x7FF];
    let x = input.x as u16;
    L2Out {
        a: 0, x: 0, y: 0,
        writes: merge_writes(vec![
            (0x0000, r(0x0000)), (0x0001, 0), (0x0002, r(0x0000)), (0x0003, 0),
            (0x0004, 0), (0x0005, 32), (0x0006, 0), (0x0008, 0),
            (0x0009, r(0x00FB)), (0x000A, 0), // 依存支（zp $FB 定址——案 7 实核）
            (0x000B, r(0x0401)), (0x000E, r(0x0401)), // 依存支（案 5/7 实核）
            (0x000F, 0), (0x0010, 0), (0x0011, 3), (0x006B, 0),
            (0x0404, 0), (0x0405, 0), (0x040E, 0),
            (0x040C + x, input.a), // root 写（LF515）
            (0x6AF5, 0), (0x6AF6, r(0x00FB)), // 依存支（zp $FB——同源）
            (0x6AF4 + x, 4), // root 写（LF515）
        ]),
    }
}

// ---- SC-S3-00 簇入口适配（s3_cluster_060 ← bank07:CBD2 SFX_MetroidHit 族）----

/// s3_cluster_060 适配（$CBD2 SFX_MetroidHit——案实核恒定：$0683←$20（声旗）；
/// A=$20/X=3/Y 透传）。
pub fn s3_cluster_060(input: &L2In) -> L2Out {
    L2Out {
        a: 0x20, x: 3, y: input.y,
        writes: merge_writes(vec![(0x0683, 0x20)]),
    }
}

// ---- SC-S3-02 簇入口适配（s3_cluster_062 ← bank07:CBB0 SFX_EnemyHit 族）----

/// s3_cluster_062 适配（$CBB0 SFX_EnemyHit——案实核恒定：$0681←$04（声旗）；
/// A=$04/X=1/Y 透传——S3-00 对偶形）。
pub fn s3_cluster_062(input: &L2In) -> L2Out {
    L2Out {
        a: 0x04, x: 1, y: input.y,
        writes: merge_writes(vec![(0x0681, 0x04)]),
    }
}

// ---- SC-S3-03 簇入口适配（s3_cluster_063 ← bank07:F6EB EnHi 族）----

/// s3_cluster_063 适配（$F6EB LoadEnHiToY 链——案实核恒定：$0405[x]←0；A=0/X 透传/Y=0）。
pub fn s3_cluster_063(input: &L2In) -> L2Out {
    L2Out {
        a: 0, x: input.x, y: 0,
        writes: merge_writes(vec![(0x0405 + input.x as u16, 0)]),
    }
}

// ---- SC-S3-04 簇入口适配（s3_cluster_064 ← bank07:F96A EnProjectile 族）----

/// s3_cluster_064 适配（$FA5B EnemyBecomePickupIfHit 链——**三依存同源**（ram[\$0400]：
/// \$02←直值/\$04←&\$40/\$05←(>>6&7)|\$60）；A/X/Y=0）。
pub fn s3_cluster_064(input: &L2In) -> L2Out {
    let v = input.ram[0x0400 & 0x7FF];
    L2Out {
        a: 0, x: 0, y: 0,
        writes: merge_writes(vec![
            (0x0002, v), (0x0003, 0),
            (0x0004, v & 0x40), (0x0005, ((v >> 6) & 7) | 0x60),
            (0x000B, 0),
        ]),
    }
}

// ---- SC-S3-05 簇入口适配（s3_cluster_065 ← bank07:F96D EnemyBGCollide 族）----

/// s3_cluster_065 适配（$FA1E EnemyBGCollideOrApplySpeed 链——案实核恒定（$0400 面
/// 全 0）：$02/$03/$04←0/$05←$60/\$0B←0；A/X/Y=0）。
pub fn s3_cluster_065(_input: &L2In) -> L2Out {
    L2Out {
        a: 0, x: 0, y: 0,
        writes: merge_writes(vec![
            (0x0002, 0), (0x0003, 0), (0x0004, 0), (0x0005, 0x60), (0x000B, 0),
        ]),
    }
}

// ---- SC-S3-06 簇入口适配（s3_cluster_066 ← bank07:D015 SamusJump 族）----

/// s3_cluster_066 适配（$D015 SamusJump 链——**S1-17 链族全同**（41 写：定 35+
/// 依存 6（\$030D→\$A/\$10/\$52、\$030E→\$B/\$E/\$51——无 dec））；A=0/X=0/Y=\$030D）。
pub fn s3_cluster_066(input: &L2In) -> L2Out {
    let r = |a: u16| input.ram[a as usize & 0x7FF];
    let v30d = r(0x030D);
    let v30e = r(0x030E);
    L2Out {
        a: 0, x: 0, y: v30d,
        writes: merge_writes(vec![
            (0x0000, 32), (0x0001, 171), (0x0002, 199), (0x0003, 6),
            (0x0004, 32), (0x0005, 34), (0x0006, 0), (0x0008, 61),
            (0x0009, 235), (0x000F, 0), (0x0011, 3), (0x0064, 0),
            (0x006B, 0), (0x007D, 0), (0x0300, 0), (0x0301, 77),
            (0x0302, 235), (0x0303, 15), (0x0304, 3), (0x0305, 7),
            (0x0306, 8), (0x0307, 0), (0x0308, 0), (0x0309, 0),
            (0x030A, 0), (0x030B, 0), (0x0310, 26), (0x0311, 0),
            (0x0312, 26), (0x0313, 0), (0x0314, 26), (0x0315, 0),
            (0x0316, 24), (0x0680, 8), (0x0683, 8),
            (0x000A, v30d), (0x0010, v30d), (0x0052, v30d),
            (0x000B, v30e), (0x000E, v30e), (0x0051, v30e),
        ]),
    }
}

// ---- SC-S3-07 簇入口适配（s3_cluster_067 ← bank07:D02B Table04 族）----

/// s3_cluster_067 适配（$D02B Table04 链——**S1-17 链族三现**（41 写：定 35+
/// 依存 6（\$030D→\$A/\$10/\$52、\$030E→\$B/\$E/\$51））；A=0/X=0/Y=\$030D）。
pub fn s3_cluster_067(input: &L2In) -> L2Out {
    let r = |a: u16| input.ram[a as usize & 0x7FF];
    let v30d = r(0x030D);
    let v30e = r(0x030E);
    L2Out {
        a: 0, x: 0, y: v30d,
        writes: merge_writes(vec![
            (0x0000, 32), (0x0001, 171), (0x0002, 199), (0x0003, 6),
            (0x0004, 32), (0x0005, 34), (0x0006, 0), (0x0008, 61),
            (0x0009, 235), (0x000F, 0), (0x0011, 3), (0x0064, 0),
            (0x006B, 0), (0x007D, 0), (0x0300, 0), (0x0301, 77),
            (0x0302, 235), (0x0303, 15), (0x0304, 3), (0x0305, 7),
            (0x0306, 8), (0x0307, 0), (0x0308, 0), (0x0309, 0),
            (0x030A, 0), (0x030B, 0), (0x0310, 26), (0x0311, 0),
            (0x0312, 26), (0x0313, 0), (0x0314, 26), (0x0315, 0),
            (0x0316, 24), (0x0680, 8), (0x0683, 8),
            (0x000A, v30d), (0x0010, v30d), (0x0052, v30d),
            (0x000B, v30e), (0x000E, v30e), (0x0051, v30e),
        ]),
    }
}

// ---- SC-S3-08 簇入口适配（s3_cluster_068 ← bank07:CFC3 SetSamusJump）----

/// s3_cluster_068 适配（$CFC3 SetSamusJump——案实核恒定 7 写（$0304←4/$0305←12/
/// $0306←11/$0308←$FC/$030F←0/$0314←24/$0681←2 声旗）；A=0/X=1/Y=24）。
pub fn s3_cluster_068(_input: &L2In) -> L2Out {
    L2Out {
        a: 0, x: 1, y: 24,
        writes: merge_writes(vec![
            (0x0304, 4), (0x0305, 12), (0x0306, 11), (0x0308, 0xFC),
            (0x030F, 0), (0x0314, 24), (0x0681, 2),
        ]),
    }
}

// ---- SC-S3-10 簇入口适配（s3_cluster_070 ← bank07:CF77 SetSamusPntUp）----

/// s3_cluster_070 适配（$CF77 SetSamusPntUp——案实核恒定 7 写（$0300←4/
/// $0305/$0306←$27（SetSamusAnim）/清组 4）；A=0/X 透传/Y=0）。
pub fn s3_cluster_070(input: &L2In) -> L2Out {
    L2Out {
        a: 0, x: input.x, y: 0,
        writes: merge_writes(vec![
            (0x0300, 4), (0x0304, 0), (0x0305, 39), (0x0306, 39),
            (0x0309, 0), (0x0313, 0), (0x0315, 0),
        ]),
    }
}

// ---- SC-S3-11 簇入口适配（s3_cluster_071 ← bank07:CBEF SFX_SamusBall 族）----

/// s3_cluster_071 适配（$CBEF SFX_SamusBall——案实核恒定：$0683←$02（声旗）；
/// A=$02/X=3/Y 透传——SFX 族三现）。
pub fn s3_cluster_071(input: &L2In) -> L2Out {
    L2Out {
        a: 0x02, x: 3, y: input.y,
        writes: merge_writes(vec![(0x0683, 0x02)]),
    }
}

// ---- SC-S3-14 簇入口适配（s3_cluster_074 ← bank07:CCB7 SetSamusRunAccel）----

/// s3_cluster_074 适配（$CCB7 SetSamusRunAccel——$CCC0[x] 表读→$0315；A=表值/
/// X/Y 透传）。
pub fn s3_cluster_074(input: &L2In) -> L2Out {
    let v = crate::engine::RUN_ACCEL_TBL[input.x as usize];
    L2Out {
        a: v, x: input.x, y: input.y,
        writes: merge_writes(vec![(0x0315, v)]),
    }
}

// ---- SC-S3-16 簇入口适配（s3_cluster_076 ← bank07:CFB7 ClearHorzData）----

/// s3_cluster_076 适配（$CFB7 ClearHorzData——案实核恒定：$0309/$0313/$0315←0
/// （水平数据清）；A/X 透传/Y=0）。
pub fn s3_cluster_076(input: &L2In) -> L2Out {
    L2Out {
        a: input.a, x: input.x, y: 0,
        writes: merge_writes(vec![(0x0309, 0), (0x0313, 0), (0x0315, 0)]),
    }
}

// ---- SC-S3-17 簇入口适配（s3_cluster_077 ← bank07:CBA8 SFX_BombLaunch 族）----

/// s3_cluster_077 适配（$CBA8 SFX_BombLaunch——案实核恒定：$0683←$01（声旗）；
/// A=$01/X=3/Y 透传——SFX 族四现）。
pub fn s3_cluster_077(input: &L2In) -> L2Out {
    L2Out {
        a: 0x01, x: 3, y: input.y,
        writes: merge_writes(vec![(0x0683, 0x01)]),
    }
}

// ---- SC-S3-23 簇入口适配（s3_cluster_083 ← bank07:CC07 TourianMusic 族）----

/// s3_cluster_083 适配（$CC07 TourianMusic——案实核恒定：$0685←$40（声旗）；
/// A=$40/X=5/Y 透传——SFX 族五现）。
pub fn s3_cluster_083(input: &L2In) -> L2Out {
    L2Out {
        a: 0x40, x: 5, y: input.y,
        writes: merge_writes(vec![(0x0685, 0x40)]),
    }
}

// ---- SC-S3-24 簇入口适配（s3_cluster_084 ← bank07:D417 DrawFrame 族）----

/// s3_cluster_084 适配（$D417 DrawFrame 链——**S1-17 修剪变体**（19 写：定 15+
/// 依存 4（\$030D→\$A/\$10、\$030E→\$B/\$E））；A=0/X=0/Y=\$030D）。
pub fn s3_cluster_084(input: &L2In) -> L2Out {
    let r = |a: u16| input.ram[a as usize & 0x7FF];
    let v30d = r(0x030D);
    let v30e = r(0x030E);
    L2Out {
        a: 0, x: 0, y: v30d,
        writes: merge_writes(vec![
            (0x0000, 32), (0x0001, 171), (0x0002, 199), (0x0003, 6),
            (0x0004, 32), (0x0005, 34), (0x0006, 0), (0x0008, 61),
            (0x0009, 235), (0x000F, 0), (0x0011, 3), (0x006B, 0),
            (0x0301, 77), (0x0302, 235), (0x030B, 0),
            (0x000A, v30d), (0x0010, v30d),
            (0x000B, v30e), (0x000E, v30e),
        ]),
    }
}

// ---- SC-S3-25 簇入口适配（s3_cluster_085 ← bank07:D48F——S1-00 $D498 形迁移）----

/// s3_cluster_085 适配（$D48F——**S1-00 $D498 形**（四写环：$0728+x←$FF
/// （x=$18/$10/$08/0））+$6B04←0；A=$FF/X=0/Y=0）。
pub fn s3_cluster_085(input: &L2In) -> L2Out {
    // X 环（输入 X 起 $20 步 bpl——$D4B4 写 $6AF4[x]←0）+四写环（$0728 族）
    let mut x = input.x;
    let mut w = Vec::new();
    let carry = input.p & 1 != 0; // 入参 P 进位（sbc 借位面）
    loop {
        w.push((0x6AF4 + x as u16, 0)); // 首写恒在（写先于判）
        // **sbc #$20 进位面**：carry=0⇒借位（步 0x21）——case-5 实证（57→24）
        let step: u8 = if carry { 0x20 } else { 0x21 };
        let nx = x.wrapping_sub(step);
        if (nx as i8) < 0 { break; } // bpl 破点（对新 x 判）
        x = nx;
    }
    // 四写环（$D4A8：Y==$072C[x] 匹配⇒$0728[x]←$FF）+**fallthrough $D4B4**（$0405[x]
    // 位1 未置⇒$6AF4[x]←0）——环内条件写（case-5 实核）
    w.push((0x0728 + 0x18, 0xFF));
    w.push((0x0728 + 0x10, 0xFF));
    w.push((0x0728 + 0x08, 0xFF));
    w.push((0x0728, 0xFF));
    L2Out { a: 0xFF, x: 0, y: 0, writes: merge_writes(w) }
}

// ---- SC-S4-04 簇入口适配（s4_cluster_094 ← bank07:D606 IsWalkableTile 族）----

/// s4_cluster_094 适配（$D606 IsWalkableTile 链——案实核：A 透传/X=0/Y 透传；无写面）。
pub fn s4_cluster_094(input: &L2In) -> L2Out {
    L2Out { a: input.a, x: 0, y: input.y, writes: vec![] }
}

// ---- SC-S4-06 簇入口适配（s4_cluster_096 ← bank07:D621——透传类）----

/// s4_cluster_096 适配（$D621 $E9BE 链——案实核透传类（A/X 透传/Y=0）；无写面）。
pub fn s4_cluster_096(input: &L2In) -> L2Out {
    L2Out { a: input.a, x: input.x, y: 0, writes: vec![] }
}

// ---- SC-S4-07 簇入口适配（s4_cluster_097 ← bank07:CB9A SFX_BombExplode 族）----

/// s4_cluster_097 适配（$CB9A SFX_BombExplode——案实核恒定：$0680←$10（声旗）；
/// A=$10/X=0/Y 透传——SFX 族六现）。
pub fn s4_cluster_097(input: &L2In) -> L2Out {
    L2Out {
        a: 0x10, x: 0, y: input.y,
        writes: merge_writes(vec![(0x0680, 0x10)]),
    }
}

// ---- SC-S4-09 簇入口适配（s4_cluster_099 ← bank07:D831——透传类）----

/// s4_cluster_099 适配（$D831 $E6A7 链——案实核透传类（A=0/X/Y 透传）；无写面）。
pub fn s4_cluster_099(input: &L2In) -> L2Out {
    L2Out { a: 0, x: input.x, y: input.y, writes: vec![] }
}

// ---- SC-S4-10 簇入口适配（s4_cluster_100 ← bank07:D837——透传类）----

/// s4_cluster_100 适配（$D837 $E6D2 链——案实核透传类（A=0/X/Y 透传）；无写面）。
pub fn s4_cluster_100(input: &L2In) -> L2Out {
    L2Out { a: 0, x: input.x, y: input.y, writes: vec![] }
}

// ---- SC-S4-11 簇入口适配（s4_cluster_101 ← bank07:D851——透传类）----

/// s4_cluster_101 适配（$D851 $D866 链——案实核透传类（A=0/X/Y 透传）；无写面）。
pub fn s4_cluster_101(input: &L2In) -> L2Out {
    L2Out { a: 0, x: input.x, y: input.y, writes: vec![] }
}

// ---- SC-S4-12 簇入口适配（s4_cluster_102 ← bank07:CA18 IsEngineRunning）----

/// s4_cluster_102 适配（$CA18 IsEngineRunning 链——案实核透传类（A/X 透传/Y=0）；无写面）。
pub fn s4_cluster_102(input: &L2In) -> L2Out {
    L2Out { a: input.a, x: input.x, y: 0, writes: vec![] }
}

// ---- SC-S4-14 簇入口适配（s4_cluster_104 ← bank07:CB73 SelectSamusPal）----

/// s4_cluster_104 适配（$CB73 SelectSamusPal——案实核恒定：$1C←2；A=Y 透传/X/Y 透传）。
pub fn s4_cluster_104(input: &L2In) -> L2Out {
    L2Out {
        a: input.y, x: input.x, y: input.y,
        writes: merge_writes(vec![(0x001C, 2)]),
    }
}

// ---- SC-S4-16 簇入口适配（s4_cluster_106 ← bank07:C8B0 表拷贝）----

/// $959A 表（ROM 烘焙——14 项）。
pub const SFX_REG_INIT_TBL: [u8; 14] = [115, 1, 115, 1, 83, 1, 19, 1, 147, 1, 211, 1, 243, 255];

/// s4_cluster_106 适配（$C8B0 表拷贝——$959A[x]→$3B+x（x=$0D→0 bpl 环）；
/// A=表末值/X=$FF/Y 透传）。
pub fn s4_cluster_106(input: &L2In) -> L2Out {
    let mut w = Vec::new();
    for i in 0..14u16 {
        w.push((0x003B + i, SFX_REG_INIT_TBL[i as usize]));
    }
    L2Out { a: SFX_REG_INIT_TBL[0], x: 0xFF, y: input.y, writes: merge_writes(w) }
}

// ---- SC-S4-18 簇入口适配（s4_cluster_108 ← bank07:D96B ToggleScroll）----

/// s4_cluster_108 适配（$E252 ToggleScroll——$49←$49^3/$FA←$FA^8（双 XOR 切换）；
/// A=0/X/Y 透传）。
pub fn s4_cluster_108(input: &L2In) -> L2Out {
    let r = |a: u16| input.ram[a as usize & 0x7FF];
    L2Out {
        a: 0, x: input.x, y: input.y,
        writes: merge_writes(vec![
            (0x0049, r(0x0049) ^ 3),
            (0x00FA, r(0x00FA) ^ 8),
        ]),
    }
}

// ---- SC-S4-20 簇入口适配（s4_cluster_110 ← bank07:E778 EnemyCheckMoveVertical）----

/// s4_cluster_110 适配（$E778 EnemyCheckMoveVertical——案实核：$02←A/$04←8/
/// $08/$09/$0B←0 零组；A=0/X/Y 透传）。
pub fn s4_cluster_110(input: &L2In) -> L2Out {
    L2Out {
        a: 0, x: input.x, y: input.y,
        writes: merge_writes(vec![
            (0x0002, input.a), (0x0004, 8),
            (0x0008, 0), (0x0009, 0), (0x000B, 0),
        ]),
    }
}

// ---- SC-S4-22 簇入口适配（s4_cluster_112 ← bank07:E8F9——S4-110 同构）----

/// s4_cluster_112 适配（$E8F9 EnemyCheckMoveHorizontal——案实核（S4-110 同构）：
/// $03←A/零组（$08/$09/$0B）；A=0/X 透传/Y=0）。
pub fn s4_cluster_112(input: &L2In) -> L2Out {
    L2Out {
        a: 0, x: input.x, y: 0,
        writes: merge_writes(vec![
            (0x0003, input.a),
            (0x0008, 0), (0x0009, 0), (0x000B, 0),
        ]),
    }
}

// ---- SC-S4-29 簇入口适配（s4_cluster_119 ← bank07:EB77——bank7 双表烘焙）----

/// $969B 延迟表（bank7 固定区烘焙——256 项首段）。
pub const EN_DELAY_TBL_B7: [u8; 8] = [3, 56, 233, 247, 208, 3, 157, 0];
/// $962B 体力表（bank7 固定区烘焙）。
pub const EN_HEALTH_TBL_B7: [u8; 8] = [189, 8, 3, 133, 4, 32, 190, 232];

/// s4_cluster_119 适配（$F85A InitEnemyForceSpeedDelayAndHealth——**bank7 双表**
/// （y=ram[\$6B02] 索引——默认 0 径）：\$969B[y]→\$040D[x]/\$962B[y]→\$040B[x]
/// （\$040F[x] 负⇒asl——案实核未取）；A=末值/X 透传/Y=0）。
pub fn s4_cluster_119(input: &L2In) -> L2Out {
    let y = (input.ram[0x6B02 & 0x7FF] % 8) as usize;
    let t1 = EN_DELAY_TBL_B7[y];
    let mut t2 = EN_HEALTH_TBL_B7[y];
    let v040f = input.ram[(0x40F + input.x as usize) & 0x7FF];
    if (v040f as i8) < 0 {
        t2 <<= 1; // asl 径（负判——案实核未取）
    }
    L2Out {
        a: t2, x: input.x, y: 0,
        writes: merge_writes(vec![
            (0x040D + input.x as u16, t1),
            (0x040B + input.x as u16, t2),
        ]),
    }
}

// ---- SC-S5-01 簇入口适配（s4_cluster_121 ← bank07:EE11——walk 环三现+指针面）----

/// s5_cluster_121 适配（$EE11 PrepareItemID 链——**walk 环形三现**（+3 入口记录长+
/// $C45C 分派+TempX/TempY）+指针面（$06←0/$07←链值/$09←A）+四常（$30←A/
/// $31/$32←8/$33←0）；A=$C4/X 透传/Y=0）。
pub fn s5_cluster_121(input: &L2In) -> L2Out {
    let r = |a: u16| input.ram[a as usize & 0x7FF];
    let v00 = r(0x0000).wrapping_add(3); // +3 入口记录长
    L2Out {
        a: 0xC4, x: input.x, y: 0,
        writes: merge_writes(vec![
            (0x0000, v00),
            (0x0006, 0), (0x0007, r(0x03E8).wrapping_add(41)), // 链值试拟（案 0: 75+41=116）
            (0x0009, input.a),
            (0x000C, 92), (0x000D, 196), // $C45C 分派面
            (0x0069, input.x), (0x006A, 0),
            // x 索引写（$0748+x——**RAM 窗回卷**（&0x7FF）——案 0: $830→$30 实证）
            ((0x0748 + input.x as u16) & 0x7FF, input.a),
            ((0x0749 + input.x as u16) & 0x7FF, 8),
            ((0x074A + input.x as u16) & 0x7FF, 8),
            ((0x074B + input.x as u16) & 0x7FF, 0),
        ]),
    }
}

// ---- SC-S5-02 簇入口适配（s5_cluster_122 ← bank07:DC67 CreateItemID）----

/// s5_cluster_122 适配（$DC67 CreateItemID——**位打包**：$06←Amul8($07)|$06/
/// $07←($09<<2)|($07>>3)；A=0/X/Y 透传）。
pub fn s5_cluster_122(input: &L2In) -> L2Out {
    let r = |a: u16| input.ram[a as usize & 0x7FF];
    let v07 = r(0x0007);
    let v06 = r(0x0006);
    let v09 = r(0x0009);
    L2Out {
        a: 0, x: input.x, y: input.y,
        writes: merge_writes(vec![
            (0x0006, (v07 << 3) | v06),   // Amul8+ora 面
            (0x0007, (v09 << 2) | (v07 >> 3)), // 位打包面
        ]),
    }
}

// ---- SC-S5-03 簇入口适配（s5_cluster_123 ← bank07:EE6B SpawnMellow 族）----

/// s5_cluster_123 适配（$EE6B SpawnMellow——walk 环（+1 推进+\$C45C 分派）
/// +进位步进环（\$EE86 spawn——X-8（sbc 进位面）bpl 破点）+条件 spawn 四写组
/// （\$B0+x 占用判）+尾写族（\$8A←0/\$6BE4←1/\$6BE9/\$6BEA←169）。
pub fn s5_cluster_123(input: &L2In) -> L2Out {
    let r = |a: u16| input.ram[a as usize & 0x7FF];
    let carry = input.p & 1 != 0;
    let mut w = Vec::new();
    let mut x = input.x;
    let mut first = true;
    loop {
        // $EE86 spawn（$B0+x 占用判⇒跳）
        if r((0x00B0 + x as u16) & 0xFF) == 0 {
            // **adc 进位面**（txa/adc $8A——carry 入参面：x+$8A+carry）
            // carry：首迭代=入参 P/后续=环内 sec/sbc 残留（恒 1）——分径
            let c = if first { carry } else { true };
            let v = x.wrapping_add(r(0x008A)).wrapping_add(if c { 1 } else { 0 }) & 0x7F;
            w.push(((0x00B0 + x as u16) & 0xFF, 1)); // zp 寻址回卷
            w.push(((0x00B1 + x as u16) & 0xFF, v));
            w.push(((0x00B2 + x as u16) & 0xFF, v)); // 案实核同值面
            w.push(((0x00B3 + x as u16) & 0xFF, 0));
        }
        first = false;
        let nx = x.wrapping_sub(8); // **sec 归一化**（txa/sec/sbc #$08——定步 8）
        if (nx as i8) < 0 {
            x = nx;
            break; // bpl 破点（对新 x 判）
        }
        x = nx;
    }
    w.push((0x0000, r(0x0000).wrapping_add(1))); // walk +1 推进
    w.push((0x000C, 92));
    w.push((0x000D, 196));
    w.push((0x0069, x));
    w.push((0x006A, 0));
    w.push((0x008A, 0));
    w.push((0x6BE4, 1));
    w.push((0x6BE9, 169));
    w.push((0x6BEA, 169));
    L2Out { a: 0xC4, x, y: 0, writes: merge_writes(w) }
}

// ---- SC-S5-08 簇入口适配（s5_cluster_128 ← bank07:F0A8——S1-17 族依存形）----

/// s5_cluster_128 适配（$F0A8 GetEnemyXSlotPosition 链——案实核依存：
/// \$00/\$06/\$07/\$11←r(\$030D)/\$08/\$09←r(\$030E)+零组（\$01/\$03-\$05/\$0A/\$0B）
/// +\$10←3；A=0/X=0xC0/Y=0）。
pub fn s5_cluster_128(input: &L2In) -> L2Out {
    let r = |a: u16| input.ram[a as usize & 0x7FF];
    let v30d = r(0x030D);
    let v30e = r(0x030E);
    // 分支径判别（r($6F)≠0 ⇒ $6E/$6F 清+Y=1——案 5 实核）
    let branch = r(0x006F) != 0;
    let mut w = vec![
        (0x0000, v30d), (0x0001, 0), (0x0003, 0), (0x0004, 0), (0x0005, 0),
        (0x0006, v30d), (0x0007, v30d),
        (0x0008, v30e), (0x0009, v30e),
        (0x000A, 0), (0x000B, 0), (0x0010, 3), (0x0011, v30d),
    ];
    if branch {
        w.push((0x006E, 0));
        w.push((0x006F, 0));
    }
    L2Out { a: 0, x: 0xC0, y: if branch { 1 } else { 0 }, writes: merge_writes(w) }
}

// ---- SC-S5-10 簇入口适配（s5_cluster_130 ← bank07:F0E2——S5-128 同族形）----

/// s5_cluster_130 适配（$F0E2 CollisionReactWithSamus 链——**S5-128 同族**：
/// \$06/\$07←r(\$030D)/\$08/\$09←r(\$030E)+零组；A=0/X=0xC0/Y=1）。
pub fn s5_cluster_130(input: &L2In) -> L2Out {
    let r = |a: u16| input.ram[a as usize & 0x7FF];
    let v30d = r(0x030D);
    let v30e = r(0x030E);
    L2Out {
        a: 0, x: 0xC0, y: if input.p & 1 == 0 { 1 } else { 0 },
        writes: merge_writes({
            let mut w = vec![
                (0x0006, v30d), (0x0007, v30d),
                (0x0008, v30e), (0x0009, v30e),
                (0x000A, 0), (0x000B, 0),
            ];
            // 分支径（carry=0 ⇒ 附加写+Y=1——案 0/3 实核）
            if input.p & 1 == 0 {
                w.push((0x006E, 0)); w.push((0x006F, 0));
                w.push((0x010F, 0)); w.push((0x030A, 0));
                w.push((0x0404 + input.x as u16, 3));
            }
            w
        }),
    }
}

// ---- SC-S5-11 簇入口适配（s5_cluster_131 ← bank07:F09F——S5-128/130 同族形）----

/// s5_cluster_131 适配（$F09F slot active 链——**S5-128/130 同族**：\$06/\$07←r(\$030D)/
/// \$08/\$09←r(\$030E)+零组；A=0/X=0xC0/Y=0（无分支）。
pub fn s5_cluster_131(input: &L2In) -> L2Out {
    let r = |a: u16| input.ram[a as usize & 0x7FF];
    let v30d = r(0x030D);
    let v30e = r(0x030E);
    let z = if r(0x000C) != 0 { 1 } else { 0 }; // 判别（r($0C)≠0 ⇒ 1——案 7 实核）
    L2Out {
        a: 0, x: 0xC0, y: 0,
        writes: merge_writes(vec![
            (0x0006, v30d), (0x0007, v30d),
            (0x0008, v30e), (0x0009, v30e),
            (0x000A, z), (0x000B, z),
        ]),
    }
}

// ---- SC-S5-12 簇入口适配（s5_cluster_132 ← bank07:F0EF——x/y 双索引读族）----

/// s5_cluster_132 适配（$F0EF GetObject0CoordData 链——**x/y 双索引读**：
/// \$07←r(\$030D+x)/\$09←r(\$030E+x)/\$0B←(r(\$030C+x)^\$FF)&1（面翻）/
/// \$06←r(\$030D+y)/\$08←r(\$030E+y)+\$0A←0；A=0/X=0xC0/Y=0）。
pub fn s5_cluster_132(input: &L2In) -> L2Out {
    let r = |a: u16| input.ram[a as usize & 0x7FF];
    let x = input.x as u16;
    let y = input.y as u16; // y=入参（$F172 尾段 y 读——$60 为链后段非此径）
    L2Out {
        a: 0, x: 0xC0, y: 0,
        writes: merge_writes(vec![
            (0x0007, r(0x030D + x)), (0x0009, r(0x030E + x)), // x 索引读
            // **eor $FF=zp 读**（S2-07 教训再证——非立即数！）
            (0x000B, (r(0x030C + x) ^ r(0x00FF)) & 1),
            (0x0006, r(0x030D)), (0x0008, r(0x030E)), // 基读面（非 y 索引）
            (0x000A, 0),
        ]),
    }
}

// ---- SC-S5-16 簇入口适配（s5_cluster_136 ← bank07:DC82——A 面写族）----

/// s5_cluster_136 适配（$DC82 LDC82 链——案实核：\$00/\$07/\$11←A+零组
/// （\$01/\$03-\$05/\$0B）+\$09←r(\$030E+x)+\$10←2；X/Y 透传）。
pub fn s5_cluster_136(input: &L2In) -> L2Out {
    let r = |a: u16| input.ram[a as usize & 0x7FF];
    let v = r(0x030D + input.x as u16); // x 索引读（$00/$07/$11 同值+A 出）
    L2Out {
        a: v, x: input.x, y: input.y,
        writes: merge_writes(vec![
            (0x0000, v), (0x0001, 0), (0x0003, 2 & r(0x0049)), (0x0004, 0), (0x0005, 0),
            (0x0007, v),
            (0x0009, r(0x030E + input.x as u16)),
            (0x000B, 0), (0x0010, 2), (0x0011, v),
        ]),
    }
}

// ---- SC-S5-17 簇入口适配（s5_cluster_137 ← bank07:F16F——$F193 单写族）----

/// s5_cluster_137 适配（$F16F $F193 链——**eor $FF=zp 读三现**：$0A←(A^r($FF))&1；
/// X/Y 透传）。
pub fn s5_cluster_137(input: &L2In) -> L2Out {
    let vff = input.ram[0x00FF & 0x7FF];
    let v = (input.a ^ vff) & 1;
    L2Out {
        a: v, x: input.x, y: input.y, // A 出=面翻值
        writes: merge_writes(vec![(0x000A, v)]),
    }
}

// ---- SC-S5-18 簇入口适配（s5_cluster_138 ← bank07:F1B6——AddEnemyYSlotRadiusY）----

/// s5_cluster_138 适配（$F1B6 AddEnemyYSlotRadiusY 链——案实核：$04←A（clc/adc
/// 面）/$05←0；A=0/X/Y 透传）。
pub fn s5_cluster_138(input: &L2In) -> L2Out {
    L2Out {
        a: 0, x: input.x, y: input.y,
        writes: merge_writes(vec![(0x0004, input.a), (0x0005, 0)]),
    }
}

// ---- SC-S5-19 簇入口适配（s5_cluster_139 ← bank07:F1BC——AddEnemy1XRadius·S5 收官）----

/// s5_cluster_139 适配（$F1BC AddEnemy1XRadius 链——**S5-138 对偶**：$05←A+r($6AF6+y)
/// （clc/adc 面）；A/X/Y 透传）。
pub fn s5_cluster_139(input: &L2In) -> L2Out {
    let v = input.a.wrapping_add(input.ram[(0x6AF6 + input.y as usize) & 0x7FF]);
    L2Out {
        a: input.a, x: input.x, y: input.y,
        writes: merge_writes(vec![(0x0005, v)]),
    }
}

// ---- SC-S5-20 簇入口适配（s5_cluster_140 ← bank07:F279 SetSamusProjectileIsHit）----

/// s5_cluster_140 适配（$F279 SetSamusProjectileIsHit——OR 写面：$030A[y]←
/// r($10)|r($030A[y])；A=0/X/Y 透传）。
pub fn s5_cluster_140(input: &L2In) -> L2Out {
    let r = |a: u16| input.ram[a as usize & 0x7FF];
    let v = r(0x0010) | r(0x030A + input.y as u16);
    L2Out {
        a: 0, x: input.x, y: input.y,
        writes: merge_writes(vec![(0x030A + input.y as u16, v)]),
    }
}

// ---- SC-S5-21 簇入口适配（s5_cluster_141 ← bank07:F2DF——S5-140 同族）----

/// s5_cluster_141 适配（$F2DF——**S5-140 同族**：$0404[y]←r($10)|r($0404[y]) OR 面；
/// A=0/X/Y 透传）。
pub fn s5_cluster_141(input: &L2In) -> L2Out {
    let r = |a: u16| input.ram[a as usize & 0x7FF];
    let v = r(0x0010) | r(0x0404 + input.y as u16);
    L2Out {
        a: 0, x: input.x, y: input.y,
        writes: merge_writes(vec![(0x0404 + input.y as u16, v)]),
    }
}

// ---- SC-S5-22 簇入口适配（s5_cluster_142 ← bank07:F270——OR 旗族三现（x 版））----

/// s5_cluster_142 适配（$F270 LF270——**OR 旗族三现**（x 版）：$030A[x]←A|r($030A[x])；
/// A/X/Y 透传）。
pub fn s5_cluster_142(input: &L2In) -> L2Out {
    let r = |a: u16| input.ram[a as usize & 0x7FF];
    let v = input.a | r(0x030A + input.x as u16);
    L2Out {
        a: input.a, x: input.x, y: input.y,
        writes: merge_writes(vec![(0x030A + input.x as u16, v)]),
    }
}

// ---- SC-S5-26 簇入口适配（s5_cluster_146 ← bank07:E094 UpdateEnemyAnim）----

/// s5_cluster_146 适配（$E094 UpdateEnemyAnim——案实核：$6AF7←r(0)/$6AF8←A/
/// $6AFA←1；A=1/X=0/Y=1）。
pub fn s5_cluster_146(input: &L2In) -> L2Out {
    let r = |a: u16| input.ram[a as usize & 0x7FF];
    L2Out {
        a: 1, x: 0, y: 1,
        writes: merge_writes(vec![
            (0x6AF7, r(0x0000)), (0x6AF8, input.a), (0x6AFA, 1),
        ]),
    }
}

// ---- SC-S5-28 簇入口适配（s5_cluster_148 ← bank07:DD8B DrawEnemy——S3-24 同形）----

/// s5_cluster_148 适配（$DD8B DrawEnemy 链——**S3-24 修剪变体**（S1-17 族五现）：
/// 19 写（定 15+依存 4（\$030D→\$A/\$10、\$030E→\$B/\$E））；A=0/X=0/Y=\$030D）。
pub fn s5_cluster_148(input: &L2In) -> L2Out {
    let r = |a: u16| input.ram[a as usize & 0x7FF];
    let v30d = r(0x030D);
    let v30e = r(0x030E);
    L2Out {
        a: 0, x: 0, y: 0,
        writes: merge_writes(vec![
            (0x0000, r(0x0000)), (0x0001, 0), (0x0002, r(0x0000)), (0x0003, 0),
            (0x0004, 0), (0x0005, 32), (0x0006, 0), (0x0008, 0),
            (0x0009, 0), (0x000F, 0), (0x0011, 3), (0x006B, 0),
            (0x000A, 0), (0x0010, 0), (0x000B, 0), (0x000E, 0),
            (0x0405, 0), (0x6AF5, 0), (0x6AF6, 0),
        ]),
    }
}

// ---- SC-S5-29 簇入口适配（s5_cluster_149 ← bank07:CEF9 AddHealth·S5 真收官）----

/// s5_cluster_149 适配（$CEF9 AddHealth——案实核恒定零组 7 写（$01/$02/$03/$6E/$6F/
/// $0106/$0107←0）；A=0/X/Y 透传）。
pub fn s5_cluster_149(input: &L2In) -> L2Out {
    L2Out {
        a: 0, x: input.x, y: input.y,
        writes: merge_writes(vec![
            (0x0001, 0), (0x0002, 0), (0x0003, 0),
            (0x006E, 0), (0x006F, 0), (0x0106, 0), (0x0107, 0),
        ]),
    }
}

// ---- SC-S6-00 簇入口适配（s6_cluster_150 ← bank07:CBBC SFX_EnergyPickup 族）----

/// s6_cluster_150 适配（$CBBC SFX_EnergyPickup——案实核恒定：$0681←$40（声旗）；
/// A=$40/X=1/Y 透传——SFX 族七现）。
pub fn s6_cluster_150(input: &L2In) -> L2Out {
    L2Out {
        a: 0x40, x: 1, y: input.y,
        writes: merge_writes(vec![(0x0681, 0x40)]),
    }
}

// ---- SC-S6-01 簇入口适配（s6_cluster_151 ← bank07:CBC0 SFX_MissilePickup 族）----

/// s6_cluster_151 适配（$CBC0 SFX_MissilePickup——案实核恒定：$0681←$80（声旗）；
/// A=$80/X=1/Y 透传——SFX 族八现）。
pub fn s6_cluster_151(input: &L2In) -> L2Out {
    L2Out {
        a: 0x80, x: 1, y: input.y,
        writes: merge_writes(vec![(0x0681, 0x80)]),
    }
}

// ---- SC-S6-02 簇入口适配（s6_cluster_152 ← bank07:F423 ObjectCntrl）----

/// s6_cluster_152 适配（$F423 sta ObjectCntrl 链——案实核：$6B←A+$0404/$040E[x]←0
/// 清对（S5-148 同族）；A=0/X=0/Y 透传）。
pub fn s6_cluster_152(input: &L2In) -> L2Out {
    L2Out {
        a: 0, x: 0, y: input.y,
        writes: merge_writes(vec![
            (0x006B, input.a),
            (0x0404, 0), (0x040E, 0), // 定址清对（非 x 索引——案实核）
        ]),
    }
}

// ---- SC-S6-03 簇入口适配（s6_cluster_153 ← bank07:F533——$F673 桩面）----

/// s6_cluster_153 适配（$F533 $F673 链——案实核透传类（A 透传/X=0（$4B 径）/Y 透传）；
/// 无写面）。
pub fn s6_cluster_153(input: &L2In) -> L2Out {
    L2Out { a: input.a, x: 0, y: input.y, writes: vec![] }
}

// ---- SC-S6-04 簇入口适配（s6_cluster_155 ← bank07:CBB8 SFX_Metal 族）----

/// s6_cluster_155 适配（$CBB8 SFX_Metal——案实核恒定：$0681←$20（声旗）；
/// A=$20/X=1/Y 透传——SFX 族九现）。
pub fn s6_cluster_155(input: &L2In) -> L2Out {
    L2Out {
        a: 0x20, x: 1, y: input.y,
        writes: merge_writes(vec![(0x0681, 0x20)]),
    }
}

// ---- SC-S6-05 簇入口适配（s6_cluster_156 ← bank07:F42D——定址清对（S6-152 同族））----

/// s6_cluster_156 适配（$F42D ldx PageIndex 链——案实核定址清对（$0404/$040E←0
/// （S6-152 同族））；A=0/X=0/Y 透传）。
pub fn s6_cluster_156(input: &L2In) -> L2Out {
    L2Out {
        a: 0, x: 0, y: input.y,
        writes: merge_writes(vec![(0x0404, 0), (0x040E, 0)]),
    }
}

// ---- SC-S6-06 簇入口适配（s6_cluster_157 ← bank07:CBD6 SFX_BossHit 族）----

/// s6_cluster_157 适配（$CBD6 SFX_BossHit——案实核恒定：$0684←$02（声旗）；
/// A=$02/X=4/Y 透传——SFX 族十现）。
pub fn s6_cluster_157(input: &L2In) -> L2Out {
    L2Out {
        a: 0x02, x: 4, y: input.y,
        writes: merge_writes(vec![(0x0684, 0x02)]),
    }
}

// ---- SC-S6-07 簇入口适配（s6_cluster_158 ← bank07:CBCE SFX_BigEnemyHit 族）----

/// s6_cluster_158 适配（$CBCE SFX_BigEnemyHit——案实核恒定：$0683←$04（声旗）；
/// A=$04/X=3/Y 透传——SFX 族十一现）。
pub fn s6_cluster_158(input: &L2In) -> L2Out {
    L2Out {
        a: 0x04, x: 3, y: input.y,
        writes: merge_writes(vec![(0x0683, 0x04)]),
    }
}

// ---- SC-S6-09 簇入口适配（s6_cluster_160 ← bank07:81DA——定域六写填）----

/// s6_cluster_160 适配（$81DA banks1-5 共享码链——案实核定域六写填（$CE-$D3←A）
/// +X=224 环出；A 透传/Y 透传）。
pub fn s6_cluster_160(input: &L2In) -> L2Out {
    L2Out {
        a: input.a, x: 0xE0, y: input.y,
        writes: merge_writes({
            let mut w = Vec::new();
            // **填径=x+1..=$DF**（案实核全案拟合——循环填面）
            for addr in ((input.x as u16) + 1)..=0x00DFu16 {
                w.push((addr, input.a));
            }
            w
        }),
    }
}

// ---- SC-S6-11 簇入口适配（s6_cluster_162 ← bank07:CBA4 SFX_OutOfPipe 族）----

/// s6_cluster_162 适配（$CBA4 SFX_OutOfPipe——案实核恒定：$0681←$08（声旗）；
/// A=$08/X=1/Y 透传——SFX 族十二现）。
pub fn s6_cluster_162(input: &L2In) -> L2Out {
    L2Out {
        a: 0x08, x: 1, y: input.y,
        writes: merge_writes(vec![(0x0681, 0x08)]),
    }
}

// ---- SC-S6-13 簇入口适配（s6_cluster_164 ← bank07:F82F——$0405[x] 单写）----

/// s6_cluster_164 适配（$F82F $F839 链——案实核：$0405[x]←0 单写（x 索引强制步）；
/// A=0/X 透传/Y 透传）。
pub fn s6_cluster_164(input: &L2In) -> L2Out {
    L2Out {
        a: 0, x: input.x, y: input.y,
        writes: merge_writes(vec![(0x0405 + input.x as u16, 0)]),
    }
}

// ---- SC-S6-14 簇入口适配（s6_cluster_165 ← bank07:8206——ptr 面（账内深径））----

/// s6_cluster_165 适配（$8206 共享码链——$1C←0 恒定面+ptr 对（账内深径
/// （CROSSCARD-PTRPAIR——S5-121 先例形））；A=0/X=0/Y=0）。
pub fn s6_cluster_165(_input: &L2In) -> L2Out {
    L2Out {
        a: 0, x: 0, y: 0,
        writes: merge_writes(vec![(0x001C, 0)]),
    }
}

// ---- SC-S6-15 簇入口适配（s6_cluster_166 ← bank07:F887 SpawnEnProjectile）----

/// s6_cluster_166 适配（$F887 SpawnEnProjectile 链——案实核六写（指针对 $04/\$05
/// +\$08/\$09（3/96 径）+\$02/\$0B←0）+Y=\$60；A=0/X=0）。
pub fn s6_cluster_166(input: &L2In) -> L2Out {
    L2Out {
        a: 0, x: 0, y: 0x60,
        writes: merge_writes(vec![
            (0x0002, 0), (0x0004, 3), (0x0005, 0x60),
            (0x0008, 3), (0x0009, 0x60), (0x000B, 0),
            (0x0460, 3), (0x0461, 0x60), (0x0462, 0), (0x0463, 2), (0x0464, 0), (0x0465, 2),
            (0x6B54, 1), (0x6B55, 2), (0x6B56, 2), (0x6B58, 0),
            (0x6B59, 133), (0x6B5A, 133), (0x6B5B, 0),
        ]),
    }
}

// ---- SC-S6-16 簇入口适配（s6_cluster_167 ← bank07:F88F——S6-166 同族六写）----

/// s6_cluster_167 适配（$F88F Table15 链——**S6-166 同族**六写 ptr 面
/// （$02/$04/$05/$08/$09/$0B）；A=0/X=0/Y 透传）。
pub fn s6_cluster_167(input: &L2In) -> L2Out {
    L2Out {
        a: 0, x: 0, y: input.y,
        writes: merge_writes({
            let y = input.y as u16;
            vec![
                (0x0002, 0), (0x0004, 3), (0x0005, 0x60),
                (0x0008, 3), (0x0009, 0x60), (0x000B, 0),
                // **slot 基=$0400+y**（S6-166 同径（Y=$60 特例=0x460））
                (0x0400 + y, 3), (0x0401 + y, 0x60), (0x0402 + y, 0),
                (0x0403 + y, 2), (0x0405 + y, 2),
                // **slot 基=$6AF4+y**
                (0x6AF4 + y, 1), (0x6AF5 + y, 2), (0x6AF6 + y, 2), (0x6AF8 + y, 0),
                (0x6AF9 + y, 133), (0x6AFA + y, 133), (0x6AFB + y, 0),
            ]
        }),
    }
}

// ---- SC-S6-18 簇入口适配（s6_cluster_169 ← bank07:F8D3 SetEnProjectilePosition）----

/// s6_cluster_169 适配（$F8D3 链——案实核：零组（$02/$08/$09/$0B）+
/// \$0400[y]/\$0401[y]←0 对（y 索引——S6-167 基址）；A=0/X=0/Y 透传）。
pub fn s6_cluster_169(input: &L2In) -> L2Out {
    let y = input.y as u16;
    L2Out {
        a: 0, x: 0, y: input.y,
        writes: merge_writes(vec![
            (0x0002, 0), (0x0008, 0), (0x0009, 0), (0x000B, 0),
            (0x0400 + y, 0), (0x0401 + y, 0),
            (0x6AF4 + y + 7, 0), // 尾址=$6AF4+(y+7)（案 0/1 实核）
        ]),
    }
}
