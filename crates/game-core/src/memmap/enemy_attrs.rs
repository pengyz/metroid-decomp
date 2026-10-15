//! 敌属性数组 typed view ↔ RAM 字节域（地址只活在本层——two-layer-architecture.md §1）。
//!
//! 地址证据（A 级）：bank07:F270 `1D 0A 03` ORA $030A,X / `9D 0A 03` STA $030A,X；
//! bank07:F27B `19 0A 03` ORA $030A,Y（F279/F338 共享尾同域）。
//! 双口径（lead 裁定）：字节偏移契约面（L2 已证任意 X，写集推导走偏移口径）。

use crate::enemy::EnemyAttrs;

/// 敌属性数组基址（RAM）。
pub const ENEMY_ATTRS_BASE: u16 = 0x030A;

/// 字节偏移契约面长度（任意 X∈[0,255] 的落域；$30A+X≤$409，2KB 镜像归一无跨界）。
pub const ENEMY_ATTRS_LEN: usize = 0x100;

/// RAM 快照 → 域状态（字节偏移口径；`ram` 为 2KB 归一镜像面）。
pub fn decode_attrs(ram: &[u8]) -> EnemyAttrs {
    let mut entries = [0u8; ENEMY_ATTRS_LEN];
    for (i, e) in entries.iter_mut().enumerate() {
        *e = ram[(ENEMY_ATTRS_BASE as usize + i) & 0x7FF];
    }
    EnemyAttrs::new(entries)
}

/// 语义变更点 → 写事件（L2 契约面：**同值写也是事件**——6502 STA 即写，终值映射含同值写；
/// 地址仅出本层常量。语义方法的写足迹=其赋值字段，经本函数映射为 (addr,value)）。
pub fn write_event(idx: u8, v: u8) -> (u16, u8) {
    (ENEMY_ATTRS_BASE + idx as u16, v)
}

/// 状态变更 → 写集（字节偏移口径推导，**不手写地址**——地址仅出本层常量）。
/// 注：仅含**值变化**的址；L2 契约面需 write_event（同值写），本函数供一致性互检用。
pub fn diff_writes(before: &EnemyAttrs, after: &EnemyAttrs) -> Vec<(u16, u8)> {
    let (b, a) = (before.entries(), after.entries());
    let mut out = Vec::new();
    for i in 0..ENEMY_ATTRS_LEN {
        if b[i] != a[i] {
            out.push((ENEMY_ATTRS_BASE + i as u16, a[i]));
        }
    }
    out
}
