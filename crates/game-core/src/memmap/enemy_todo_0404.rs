//! 敌方 $0404 数组 typed view ↔ RAM 字节域（地址只活在本层——two-layer-architecture.md §1）。
//!
//! 地址证据（A 级）：bank07:F2D8 `1D 04 04`/`9D 04 04`（ORA/STA $0404,X）；
//! bank07:F2DF `19 04 04`/`99 04 04`（ORA/STA $0404,Y）。
//! 掩码源 zp：bank07:F2DF 起手 `A5 10` LDA zp $10——**语义未证（C 级）**，F2CA 族掩码源。
//! 双口径：字节偏移契约面（L2 已证任意 X/Y，写集推导走偏移口径）。

use crate::enemy::EnemyTodo0404;

/// 敌方 $0404 数组基址（RAM）。
pub const ENEMY_TODO_0404_BASE: u16 = 0x0404;

/// 字节偏移契约面长度（任意 X/Y∈[0,255]；$404+X≤$503，2KB 镜像归一无跨界）。
pub const ENEMY_TODO_0404_LEN: usize = 0x100;

/// 掩码源 zp（F2DF 流 `LDA zp $10` 起手；语义未证 C 级——F2CA 族掩码源）。
pub const MASK10_ZP: u16 = 0x0010;

/// RAM 快照 → 域状态（字节偏移口径）。
pub fn decode_todo_0404(ram: &[u8]) -> EnemyTodo0404 {
    let mut entries = [0u8; ENEMY_TODO_0404_LEN];
    for (i, e) in entries.iter_mut().enumerate() {
        *e = ram[(ENEMY_TODO_0404_BASE as usize + i) & 0x7FF];
    }
    EnemyTodo0404::new(entries)
}

/// 语义变更点 → 写事件（L2 契约面：同值写也是事件；地址仅出本层常量）。
pub fn write_event(idx: u8, v: u8) -> (u16, u8) {
    (ENEMY_TODO_0404_BASE + idx as u16, v)
}

/// 状态变更 → 写集（仅值变化面，供一致性互检；L2 契约面用 write_event）。
pub fn diff_writes(before: &EnemyTodo0404, after: &EnemyTodo0404) -> Vec<(u16, u8)> {
    let (b, a) = (before.entries(), after.entries());
    let mut out = Vec::new();
    for i in 0..ENEMY_TODO_0404_LEN {
        if b[i] != a[i] {
            out.push((ENEMY_TODO_0404_BASE + i as u16, a[i]));
        }
    }
    out
}

/// 掩码源读（zp $10；shim 侧调用——产品层零地址）。
pub fn mask10(ram: &[u8]) -> u8 {
    ram[MASK10_ZP as usize & 0x7FF]
}
