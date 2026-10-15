//! RNG 状态面 ↔ zp 字节域（地址只活在本层——two-layer-architecture.md §1）。
//!
//! 地址证据（A 级）：bank07:C004 `A5 2E` LDA zp $2E / C009 `85 2E` STA zp $2E（RN1 轴）；
//! bank07:C00B `A5 2F` / C010 `85 2F`（RN2 轴）；种子序 bank07:$C0AC `A9 11 85 2E A9 FF 85 2F`。
//! 命名台账 N-0012（GameRng 升级 A 级）；考证=docs/findings/game-rng-adjudication.md。
//! 面外读写=0：C000 全 26B 流只寻址 $2E/$2F（栈流量=机械，不入写集）。

use crate::rng::RngState;

/// RN1 轴 zp 地址（m1disasm 名 RandomNumber1）。
pub const RNG_RN1_ZP: u16 = 0x002E;

/// RN2 轴 zp 地址（m1disasm 名 RandomNumber2）。
pub const RNG_RN2_ZP: u16 = 0x002F;

/// RAM 快照 → 域状态（zp 域解码）。
pub fn decode_rng_state(ram: &[u8]) -> RngState {
    RngState {
        rn1: ram[RNG_RN1_ZP as usize & 0x7FF],
        rn2: ram[RNG_RN2_ZP as usize & 0x7FF],
    }
}

/// 语义变更点 → 写事件（RN1 轴；同值写也是事件）。
pub fn write_event_rn1(v: u8) -> (u16, u8) {
    (RNG_RN1_ZP, v)
}

/// 语义变更点 → 写事件（RN2 轴）。
pub fn write_event_rn2(v: u8) -> (u16, u8) {
    (RNG_RN2_ZP, v)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// decode ↔ write_event 往返（地址只出本层常量）。
    #[test]
    fn decode_and_write_events_roundtrip() {
        let mut ram = vec![0u8; 0x800];
        ram[0x2E] = 0x11;
        ram[0x2F] = 0xFF;
        let st = decode_rng_state(&ram);
        assert_eq!(st, RngState::boot_seed());
        assert_eq!(write_event_rn1(0x2A), (0x002E, 0x2A));
        assert_eq!(write_event_rn2(0x5E), (0x002F, 0x5E));
    }
}
