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
