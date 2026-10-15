//! game-core::verify_shim —— 验证适配层（two-layer-architecture.md §1）：
//! 域方法 → L2In/L2Out 写集契约。**写集由状态变更经 memmap diff 推导**，不手写地址。
//!
//! 机械名（sub_F2xx）只存在于本层注册注释（回指 bank:addr，§4）；产品层零机械名零地址。
//! 对拍不变式（§3）：行为钉死=结构可换——同批 L2 案在域形态下持续全绿。

use crate::{memmap, L2In, L2Out};

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
        writes,
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
/// + $6E/$6F←ROM 向量常量（window_bank=7 约定 A9）；出口 A=ROM_VEC_HI。
/// 写集：域 write_event + 暂存面事件推导。
pub fn f2ed(input: &L2In) -> L2Out {
    const FLAG_C: u8 = 0x01;
    if input.p & FLAG_C != 0 {
        return L2Out { a: input.a, x: input.x, y: input.y, writes: vec![] };
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
    let writes = vec![
        memmap::enemy_todo_0404::write_event(input.y, a_df),
        memmap::flow_scratch::event_010f(memmap::flow_scratch::F2ED_SCRATCH_INIT),
        memmap::enemy_attrs::write_event(input.x, attr_v),
        memmap::flow_scratch::event_vec_lo(memmap::flow_scratch::ROM_VEC_LO),
        memmap::flow_scratch::event_vec_hi(memmap::flow_scratch::ROM_VEC_HI),
    ];
    L2Out { a: memmap::flow_scratch::ROM_VEC_HI, x: input.x, y: input.y, writes }
}
