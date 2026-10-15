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
