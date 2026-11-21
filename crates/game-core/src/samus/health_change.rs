//! `HealthChange`（`$006E` 低 / `$006F` 高）的 **typed 真源**。
//!
//! 证据（A 级）：
//! - `refs/m1disasm/SRC/constants_ram.asm` `HealthChange ; 06:006E`（dw：「Amount to
//!   add/subtract from Health」——扣血/回血共用的一对字节，低字节在前）。
//! - 读者：`weapon/projectile.rs::subtract_health`（`SubtractHealth ; 07:CECC`）——
//!   `hc0=$6E/hc1=$6F`，Varia 半伤走 `lsr` 对（丢进位/借出 bit0），再交给
//!   `Base10Subtract ; 07:C3FB` 做 BCD 逐位减。
//! - 写者：`samus/state.rs`（`SetSamusExplode` 清、`$07` 起步量）、
//!   `weapon/projectile.rs`（`$50`/`$80` 伤害量）、各处清零点。
//!
//! 形态纪律：asm 里这对字节是**一个 `dw`**，所以真相面是 `u16` 而不是两个独立 u8；
//! 但消费端按 BCD/半伤规则逐字节用，故 [`Self::get_byte`]/[`Self::set_byte`] 保留字节面
//! 等价映射（裸面 `$006E` 仍返回低字节）。地址常量取 `ram_map::HEALTH_CHANGE`
//! （地址只有一个家）。

use crate::memmap::nesmap::{NesMapped, NesSpan, SpanKey};

/// `HealthChange` 的两个字节地址（低、高）。
pub const HEALTH_CHANGE_ADDRS: [u16; 2] = [crate::ram_map::HEALTH_CHANGE, crate::ram_map::HEALTH_CHANGE + 1];

/// 扣血/回血量的 typed 真源（`$006E` 低 / `$006F` 高）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct HealthChange {
    word: u16,
}

impl HealthChange {
    /// 迁入 typed 的地址清单（`restore_mss` 回灌/清零与 `snapshot_nes` 投影共用）。
    pub const ADDRS: [u16; 2] = HEALTH_CHANGE_ADDRS;

    /// 字面值（低字节在 `$006E`）。
    pub fn word(&self) -> u16 {
        self.word
    }

    pub fn set_word(&mut self, v: u16) {
        self.word = v;
    }

    /// 地址 → 字节（不在本域返回 `None`）。
    pub fn get_byte(&self, addr: u16) -> Option<u8> {
        if addr == Self::ADDRS[0] {
            Some(self.word as u8)
        } else if addr == Self::ADDRS[1] {
            Some((self.word >> 8) as u8)
        } else {
            None
        }
    }

    /// 字节 → 地址（不在本域返回 `false`，调用方继续往下一域分派）。
    pub fn set_byte(&mut self, addr: u16, v: u8) -> bool {
        if addr == Self::ADDRS[0] {
            self.word = (self.word & 0xFF00) | v as u16;
            true
        } else if addr == Self::ADDRS[1] {
            self.word = (self.word & 0x00FF) | ((v as u16) << 8);
            true
        } else {
            false
        }
    }
}

impl NesMapped for HealthChange {
    const DISCRIMINATOR: Option<u16> = None;

    /// 一段 2 B `Owner`（`$006E-$006F`）——asm 侧是一个 `dw`，typed 侧是一个 `u16`。
    const SPANS: &'static [(NesSpan, SpanKey)] = &[(NesSpan::value(Self::ADDRS[0], 2), SpanKey::Owner)];

    fn project(&self, out: &mut crate::memmap::storage::NesImage) {
        let lo = (Self::ADDRS[0] & 0x7FF) as usize;
        out.ram[lo] = self.word as u8;
        out.ram[lo + 1] = (self.word >> 8) as u8;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame::Manager;

    /// brief Step 1 用例：写 `0x0080` ⇒ 低字节 `$006E=0x80`、高字节 `$006F=0x00`，
    /// 且字节面不持有 typed 副本（读径/投影同源）。
    #[test]
    fn health_change_word_is_typed() {
        let mut m = Manager::default();
        m.set_health_change(0x0080);
        assert_eq!(m.read_ram(0x006E), 0x80);
        assert_eq!(m.read_ram(0x006F), 0x00);
        assert_eq!(m.ram()[0x006E], 0, "字节面不持有 typed 字节");
        assert_eq!(m.snapshot_nes().ram[0x006E], 0x80, "投影与 typed 同源");
        assert_eq!(m.snapshot_nes().ram[0x006F], 0x00);
        // 字节面写回 typed（WRAM 之外的地址面写径也走 typed）。
        m.write_ram(0x006F, 0x12);
        assert_eq!(m.health_change(), 0x1280);
        assert_eq!(m.read_ram(0x006E), 0x80);
    }

    /// `restore_mss` 回灌后 typed 与字节输入一致、storage 清零（清单覆盖本域）。
    #[test]
    fn health_change_is_restored_and_cleared_from_mss() {
        let mut ram = vec![0u8; 0x800];
        ram[0x006E] = 0x34;
        ram[0x006F] = 0x12;
        let wram = vec![0u8; 0x2000];
        let m = Manager::restore_mss(&ram, &wram, crate::state::Registers::default());
        assert_eq!(m.health_change(), 0x1234);
        assert_eq!(m.ram()[0x006E], 0, "typed 格在 storage 清零");
        assert_eq!(m.ram()[0x006F], 0);
    }
}


/// `AddHealth ; 07:CEF9`（`refs/m1disasm/SRC/prg7_engine.asm:3282-3328`）。
///
/// 语义（逐行对齐）：
/// 1. `Health` 低字节 BCD 加 `HealthChange` 低字节；高字节同法，**各自独立进位丢弃**；
/// 2. 上限 = `(EnergyTankCount << 4) | 0x0F`（`Amul16` = asl×4 ⇒ ×16，再 `ora #$0F`）；
///    `max >= Health+1` ⇒ 不变；否则 `Health+1 ← max & #$F9`、`Health ← #$99`；
/// 3. 尾件 `jmp ClearHealthChange`（把 `$006E/$006F` 清零）。
pub fn add_health(mem: &mut crate::state::Manager) {
    use crate::weapon::projectile::bcd_add;

    let hc = mem.health_change();
    let h = mem.samus().health();
    let (lo, _) = bcd_add((h & 0xFF) as u8, (hc & 0xFF) as u8);
    let (hi, _) = bcd_add((h >> 8) as u8, (hc >> 8) as u8);

    let max_hi = (mem.read_wram(crate::ram_map::ENERGY_TANK_COUNT) << 4) | 0x0F;
    let (hi, lo) = if max_hi >= hi {
        (hi, lo)
    } else {
        (max_hi & 0xF9, 0x99)
    };

    mem.samus().set_health(u16::from(lo) | (u16::from(hi) << 8));
    mem.set_health_change(0);
}

impl crate::state::Manager {
    /// `HealthChange`（`$006E/$006F`）——扣血/回血量的 typed 视图。
    pub fn health_change(&self) -> u16 {
        self.health_change.word()
    }

    pub fn set_health_change(&mut self, v: u16) {
        self.health_change.set_word(v);
    }
}
