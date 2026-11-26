//! `SamusVitals`（血量 `$0106` 低 / `$0107` 高）的 **typed 真源**——B1 最小 slice。
//!
//! 证据（A 级）：
//! - `refs/m1disasm/SRC/constants_ram.asm`：`Health` 两字节（`$0106` 低 / `$0107` 高），
//!   值域是**三位 BCD**（`Base10Subtract ; 07:C3FB` 逐位减、`DrawHealth ; 07:C8xx` 逐位画）。
//! - 读者：`crates/game-core/src/ui/hud.rs:42/:44/:91`（HUD 三位数字）、
//!   `crates/game-core/src/samus/physics.rs:981-982`（`SamusHurt ; 07:F306` 的减血/扛伤径）。
//! - 写者：`crates/game-core/src/samus/state.rs` 的 `set_health()`（`SamusHurt` 后回写）。
//!
//! 形态纪律：asm 里这两个字节是**一对值**（低字节在前），且所有读者都是「读成 u16 再拆位」，
//! 所以真相面是 `u16` 而不是两个独立 `u8`；[`Self::get_byte`]/[`Self::set_byte`] 保留字节面
//! 等价映射，让兼容辖的裸面读取口（`Manager` 的 `read_ram`/`write_ram`）不用改。
//!
//! 与既有两个「看着像血量」的面的区别（本模块刻意**不**碰它们）：
//! - `$006E/$006F` 是 `HealthChange`（扣血/回血**量**），已由 [`crate::samus::health_change`] typed；
//! - `$0301/$0302` 是 `Samus.radiusY/radiusX`（碰撞半径），已由 `SamusState` 的 `Radius` 承载。

use crate::memmap::nesmap::{NesMapped, NesSpan, SpanKey};

/// 血量两个字节地址（低、高）——地址只有一个家：`samus::state` 的 `HEALTH_LO/HEALTH_HI`。
pub const HEALTH_ADDRS: [u16; 2] = [
    crate::samus::state::HEALTH_LO,
    crate::samus::state::HEALTH_HI,
];

/// 血量（`$0106` 低 + `$0107` 高）的 typed 真源。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SamusVitals {
    health: u16,
}

impl SamusVitals {
    /// 迁入 typed 的地址清单（`snapshot_nes` 投影与 `restore_mss` 回灌共用）。
    pub const ADDRS: [u16; 2] = HEALTH_ADDRS;

    /// 血量原值（低字节在 `$0106`）。注意这是 **BCD**：`0x0299` 表示 299 点。
    pub fn health(&self) -> u16 {
        self.health
    }

    pub fn set_health(&mut self, v: u16) {
        self.health = v;
    }

    /// 十进制位数（BCD 解码，HUD 画数字用；越界位按 `>9` 原样返回 `None`）。
    pub fn health_digits(&self) -> [u8; 3] {
        [
            (self.health & 0x000F) as u8,
            ((self.health >> 4) & 0x000F) as u8,
            ((self.health >> 8) & 0x000F) as u8,
        ]
    }

    /// 字节面 → 结构（投影回读：`decode` 必须与写入的 `u16` 相等，低字节在前）。
    pub fn decode(bytes: &[u8]) -> Self {
        let lo = bytes[0] as u16;
        let hi = bytes[1] as u16;
        Self {
            health: lo | (hi << 8),
        }
    }

    /// 结构 → 字节面（低字节在前；`out` 长度必须 ≥ 2）。
    pub fn encode(&self, out: &mut [u8]) {
        out[0] = self.health as u8;
        out[1] = (self.health >> 8) as u8;
    }

    /// 地址 → 字节（不在本域返回 `None`）。
    pub fn get_byte(&self, addr: u16) -> Option<u8> {
        if addr == Self::ADDRS[0] {
            Some(self.health as u8)
        } else if addr == Self::ADDRS[1] {
            Some((self.health >> 8) as u8)
        } else {
            None
        }
    }

    /// 字节 → 地址（不在本域返回 `false`，调用方继续往下一域分派）。
    pub fn set_byte(&mut self, addr: u16, v: u8) -> bool {
        if addr == Self::ADDRS[0] {
            self.health = (self.health & 0xFF00) | v as u16;
            true
        } else if addr == Self::ADDRS[1] {
            self.health = (self.health & 0x00FF) | ((v as u16) << 8);
            true
        } else {
            false
        }
    }
}

impl NesMapped for SamusVitals {
    const DISCRIMINATOR: Option<u16> = None;

    /// 一段 2 B `Owner`（`$0106-$0107`）——asm 侧是两个相邻 `db`，typed 侧是一个 `u16`。
    const SPANS: &'static [(NesSpan, SpanKey)] =
        &[(NesSpan::value(Self::ADDRS[0], 2), SpanKey::Owner)];

    fn project(&self, out: &mut crate::memmap::storage::NesImage) {
        let lo = (Self::ADDRS[0] & 0x7FF) as usize;
        out.ram[lo] = self.health as u8;
        out.ram[lo + 1] = (self.health >> 8) as u8;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memmap::nesmap::NesMapped as _;

    /// 三例边界：满血 299（`0x0299`）、1 点（`0x0001`）、受伤后 246（`0x0246`）。
    const CASES: [(u16, [u8; 2]); 3] = [
        (0x0299, [0x99, 0x02]),
        (0x0001, [0x01, 0x00]),
        (0x0246, [0x46, 0x02]),
    ];

    #[test]
    fn health_round_trips_through_bytes_low_byte_first() {
        for (health, bytes) in CASES {
            let mut v = SamusVitals::default();
            v.set_health(health);
            let mut out = [0u8; 2];
            v.encode(&mut out);
            assert_eq!(out, bytes, "编码：低字节在前");
            assert_eq!(SamusVitals::decode(&bytes), v, "解码：与写入值相等");
        }
    }

    #[test]
    fn project_writes_exactly_the_two_vital_bytes() {
        for (health, bytes) in CASES {
            let mut v = SamusVitals::default();
            v.set_health(health);
            let mut img = crate::memmap::storage::NesImage {
                ram: [0xEE; 0x800],
                wram: [0u8; 0x2000],
            };
            v.project(&mut img);
            assert_eq!(&img.ram[0x106..0x108], &bytes, "投影落 $0106/$0107");
            // 只碰这两个字节：相邻字节保持原值。
            assert_eq!(img.ram[0x105], 0xEE);
            assert_eq!(img.ram[0x108], 0xEE);
        }
    }

    #[test]
    fn byte_accessors_cover_exactly_the_vitals() {
        let mut v = SamusVitals::default();
        assert!(v.set_byte(HEALTH_ADDRS[0], 0x46));
        assert!(v.set_byte(HEALTH_ADDRS[1], 0x02));
        assert_eq!(v.health(), 0x0246, "字节写入拼成 u16");
        assert_eq!(v.get_byte(HEALTH_ADDRS[0]), Some(0x46));
        assert_eq!(v.get_byte(HEALTH_ADDRS[1]), Some(0x02));
        assert_eq!(v.get_byte(0x0108), None, "域外返回 None");
        assert!(!v.set_byte(0x0108, 0), "域外写入返回 false");
    }

    #[test]
    fn digits_decode_bcd_for_hud() {
        let mut v = SamusVitals::default();
        v.set_health(0x0246);
        assert_eq!(v.health_digits(), [6, 4, 2], "BCD 逐位：个/十/百");
    }
}
