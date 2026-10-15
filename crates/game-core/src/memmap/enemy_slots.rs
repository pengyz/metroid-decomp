//! 敌槽状态块 typed view ↔ WRAM $6AF4-$6BF3（地址只活在本层——two-layer-architecture.md §1）。
//!
//! 地址证据（A 级）：bank07:F28E `BD F4 6A` LDA $6AF4,X（字段 0）；bank07:F295 `BD 02 6B`
//! LDA $6B02,X（字段 14）——slot 内偏移自证 0x6B02-0x6AF4=0x0E=14。
//! 域几何：16B×16=256B（$6AF4-$6BF3）。
//! 双口径（lead 裁定）：槽语义视图（16B×16）+ 字节偏移契约面（L2 已证任意 X，
//! 写集推导走偏移口径）；两口径同源互检（roundtrip 单测）。

use crate::enemy::EnemySlot;

/// 敌槽状态块基址（WRAM）。
pub const ENEMY_SLOTS_BASE: u16 = 0x6AF4;

/// 域长度（$6AF4-$6BF3）。
pub const ENEMY_SLOTS_LEN: usize = 0x100;

/// 槽宽（16B 步长）。
pub const SLOT_STRIDE: usize = 16;

/// 槽数（16）。
pub const SLOT_COUNT: usize = ENEMY_SLOTS_LEN / SLOT_STRIDE;

/// 字节偏移契约面 → 槽语义视图（同源互检：decode/encode 往返恒等）。
pub fn decode_slots(region: &[u8; ENEMY_SLOTS_LEN]) -> [EnemySlot; SLOT_COUNT] {
    let mut slots = [EnemySlot::from_bytes([0; EnemySlot::SIZE]); SLOT_COUNT];
    for (i, s) in slots.iter_mut().enumerate() {
        let mut b = [0u8; EnemySlot::SIZE];
        b.copy_from_slice(&region[i * SLOT_STRIDE..(i + 1) * SLOT_STRIDE]);
        *s = EnemySlot::from_bytes(b);
    }
    slots
}

/// 槽语义视图 → 字节偏移契约面。
pub fn encode_slots(slots: &[EnemySlot; SLOT_COUNT]) -> [u8; ENEMY_SLOTS_LEN] {
    let mut region = [0u8; ENEMY_SLOTS_LEN];
    for (i, s) in slots.iter().enumerate() {
        region[i * SLOT_STRIDE..(i + 1) * SLOT_STRIDE].copy_from_slice(&s.to_bytes());
    }
    region
}

/// 字节偏移读（L2 契约面：任意 X，如 F28E 的 $6AF4+X / F295 的 $6B02+X）。
pub fn read_offset(region: &[u8; ENEMY_SLOTS_LEN], off: u8) -> u8 {
    region[off as usize]
}

/// 字段 0 字节偏移读（F282 流：`BD F4 6A` LDA $6AF4,X）。
pub fn read_field00(wram: &[u8], x: u8) -> u8 {
    wram[0xAF4 + x as usize]
}

/// 字段 14 字节偏移读（F282 流：`BD 02 6B` LDA $6B02,X）。
/// 注：X 大时越出 16B×16 槽视图（≤$6C01 仍在 WRAM 域）——**字节偏移契约面优先**
/// （L2 已证任意 X），槽视图为 256B 投影。
pub fn read_field0e(wram: &[u8], x: u8) -> u8 {
    wram[0xB02 + x as usize]
}

/// 字节偏移写（返回写址=BASE+off，供写集推导）。
pub fn write_offset(region: &mut [u8; ENEMY_SLOTS_LEN], off: u8, v: u8) -> (u16, u8) {
    region[off as usize] = v;
    (ENEMY_SLOTS_BASE + off as u16, v)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 双口径同源互检：槽语义视图 ⇄ 字节偏移契约面 encode/decode 往返恒等。
    #[test]
    fn slot_view_byte_view_roundtrip() {
        let mut region = [0u8; ENEMY_SLOTS_LEN];
        for (i, b) in region.iter_mut().enumerate() {
            *b = (i as u8).wrapping_mul(7).wrapping_add(3);
        }
        let slots = decode_slots(&region);
        let back = encode_slots(&slots);
        assert_eq!(region, back);
    }

    /// 字节偏移契约面与槽字段同源：字段 0/14 = 偏移 0/14（A 级触达证据的几何自检）。
    #[test]
    fn offset_view_matches_slot_fields() {
        let mut region = [0u8; ENEMY_SLOTS_LEN];
        region[0] = 0x41; // slot0 字段 0（$6AF4）
        region[14] = 0x42; // slot0 字段 14（$6B02）
        region[SLOT_STRIDE + 0] = 0x51; // slot1 字段 0（$6B04）
        let slots = decode_slots(&region);
        assert_eq!(slots[0].status, read_offset(&region, 0)); // status（N-0031）
        assert_eq!(slots[0].type_, read_offset(&region, 14)); // type（N-0044）
        assert_eq!(slots[1].status, read_offset(&region, SLOT_STRIDE as u8));
        assert_eq!(ENEMY_SLOTS_BASE + 14, 0x6B02); // 几何自证：字段 14 地址回归
    }
}
