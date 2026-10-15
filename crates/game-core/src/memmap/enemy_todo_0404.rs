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

/// 变换掩码（F2D5/F2AD 流）：((RAM[$10] ^ 3) << 3) & $FF。
/// 证据（A 级字节）：bank07:F340 `A5 10 49 03 60`（LDA zp $10 / EOR #$03 / RTS）+
/// bank07:C2C6 `0A 0A 0A 60`（ASL A ×3 / RTS——F332 尾 `4C C6 C2` JMP $C2C6 汇入）。
/// 变换逻辑住映射层（lead 口径）：产品层只见语义掩码。
pub fn mask10_transformed(ram: &[u8]) -> u8 {
    (mask10(ram) ^ 0x03).wrapping_shl(3) & 0xFF
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 变换掩码=((RAM[$10]^3)<<3)&$FF（F340 `49 03`+C2C6 `0A 0A 0A` 字节语义）。
    #[test]
    fn mask10_transformed_matches_rom_semantics() {
        let mut ram = vec![0u8; 0x800];
        ram[0x10] = 0x0F;
        assert_eq!(mask10_transformed(&ram), 0x60); // (0x0F^3)<<3
        ram[0x10] = 0x03;
        assert_eq!(mask10_transformed(&ram), 0x00); // 掩码 0（同值写事件面）
        ram[0x10] = 0x80;
        assert_eq!(mask10_transformed(&ram), 0x18); // 高位丢弃（8 位回绕）
    }
}
