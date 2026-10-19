//! 敌方 $040E 数组 typed view ↔ RAM 字节域（地址只活在本层——two-layer-architecture.md §1）。
//!
//! 地址证据（A 级）：bank07:F2AD `9D 0E 04` STA $040E,X（值=$81，`A9 81` LDA 立即数）；
//! bank07:F2CF `9D 0E 04` STA $040E,X（值←RAM[$0300+Y]，F2CA 流）。
//! 字节偏移契约面（L2 已证任意 X）。

use crate::enemy::EnemyState;

/// 敌方 $040E 数组基址（RAM）。
pub const ENEMY_STATE_BASE: u16 = 0x040E;

/// 字节偏移契约面长度（任意 X∈[0,255]；$40E+X≤$50D）。
pub const ENEMY_TODO_040E_LEN: usize = 0x100;

/// F2AD 流初始化值（bank07:F2AD `A9 81` LDA #$81——立即数，语义未证 C 级）。
pub const F2AD_STATE_INIT: u8 = 0x81;

/// RAM 快照 → 域状态（字节偏移口径）。
pub fn decode_enemy_state(ram: &[u8]) -> EnemyState {
    let mut entries = [0u8; ENEMY_TODO_040E_LEN];
    for (i, e) in entries.iter_mut().enumerate() {
        *e = ram[(ENEMY_STATE_BASE as usize + i) & 0x7FF];
    }
    EnemyState::new(entries)
}

/// 语义变更点 → 写事件（L2 契约面：同值写也是事件；地址仅出本层常量）。
pub fn write_event(idx: u8, v: u8) -> (u16, u8) {
    (ENEMY_STATE_BASE + idx as u16, v)
}

/// F2CA 流状态值源读（abs,Y 16 位加：RAM[$0300+Y]）。
/// 证据（A 级）：bank07:F2CC `B9 00 03` LDA $0300,Y → F2CF `9D 0E 04` STA $040E,X。
pub fn state_source(ram: &[u8], y: u8) -> u8 {
    ram[(0x0300u16 + y as u16) as usize & 0x7FF]
}

/// 状态变更 → 写集（仅值变化面，供一致性互检；L2 契约面用 write_event）。
pub fn diff_writes(before: &EnemyState, after: &EnemyState) -> Vec<(u16, u8)> {
    let (b, a) = (before.entries(), after.entries());
    let mut out = Vec::new();
    for i in 0..ENEMY_TODO_040E_LEN {
        if b[i] != a[i] {
            out.push((ENEMY_STATE_BASE + i as u16, a[i]));
        }
    }
    out
}
