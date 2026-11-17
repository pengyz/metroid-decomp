//! 名表门挡字（`$6C/$6D`，asm `ScrollBlockOnNameTable3` / `ScrollBlockOnNameTable0`）。
//!
//! asm 侧是两个**相邻**的全局字节（`constants_ram.asm:609/610`），语义是「两张名表上
//! 还压着哪几个门」——门挡位置位时 `GetRoomNum` 视为不可滚（`sec/bne RTS_E76F`），
//! 所以它们既被房间装载清、也被门绘制置位/滚动清位，是典型的「多写者单语义」字节。
//!
//! 收成 typed（而不是留在 Camera 的 Unmodeled）的理由：
//! 1. **一对相邻字节是一个逻辑量**——bit0/bit1 分别是左右门挡，单独声明一半
//!    （`$6C` Owner、`$6D` Undeclared）会让门挡语义被切成两半，读径 `$6C+y`
//!    横跨两个域；
//! 2. 写者虽多（`room_draw` 置位/清位、`update` 双清、`room_load` 区域初清、
//!    `samus/physics` 单表清），但全部经 `read_ram/write_ram` 汇聚到本模块的
//!    `get_byte/set_byte`，没有绕行；
//! 3. 名字以 asm 为准：`$53` 叫 `WalkSoundDelay`（不是 `ScrollBlockOnNameTable0`），
//!    见 `refs/m1disasm/SRC/constants_ram.asm:568`。
//!
//! asm 锚：
//! - `MoreInit ; 07:C81D`（`prg7_engine.asm:1876-1877`）：`stx ScrollBlockOnNameTable3 /
//!   stx ScrollBlockOnNameTable0`（X=0）——区域初始化双清；
//! - `GetRoomNum ; 07:E720`（`room_load.rs:189` 的 `$6C+y`）：按 `PPUCTRL_ZP&3` 选表，
//!   门挡位置位 ⇒ 不滚不载房；
//! - `DrawDoor`（`room_draw.rs:261`）：按左右门置位 `cur | block`。

use crate::memmap::nesmap::{NesMapped, NesSpan, SpanKey};
use crate::state::Manager;

/// 名表门挡字地址（`$6C` 名表 3 / `$6D` 名表 0）——相邻，故是一个 2 B `Owner` 段。
pub const SCROLL_BLOCK_ADDRS: [u16; 2] =
    [crate::ram_map::SCROLL_BLOCK_NT3, crate::ram_map::SCROLL_BLOCK_NT0];

/// 名表门挡字 typed 真源（`$6C/$6D`）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ScrollBlocks {
    nt3: u8,
    nt0: u8,
}

impl ScrollBlocks {
    /// 迁入 typed 的地址清单（`restore_mss` 回灌/清零与 `snapshot_nes` 投影共用）。
    pub const ADDRS: [u16; 2] = SCROLL_BLOCK_ADDRS;

    /// `$6C ScrollBlockOnNameTable3`——名表 3 的门挡位（bit0=左门/bit1=右门）。
    pub fn nt3(&self) -> u8 {
        self.nt3
    }

    pub fn set_nt3(&mut self, v: u8) {
        self.nt3 = v;
    }

    /// `$6D ScrollBlockOnNameTable0`——名表 0 的门挡位（同 bit 语义）。
    pub fn nt0(&self) -> u8 {
        self.nt0
    }

    pub fn set_nt0(&mut self, v: u8) {
        self.nt0 = v;
    }

    /// 地址 → 字节（不在本域返回 `None`）。
    pub fn get_byte(&self, addr: u16) -> Option<u8> {
        match addr {
            a if a == Self::ADDRS[0] => Some(self.nt3),
            a if a == Self::ADDRS[1] => Some(self.nt0),
            _ => None,
        }
    }

    /// 字节 → 地址（不在本域返回 `false`，调用方继续往下一域分派）。
    pub fn set_byte(&mut self, addr: u16, v: u8) -> bool {
        match addr {
            a if a == Self::ADDRS[0] => self.nt3 = v,
            a if a == Self::ADDRS[1] => self.nt0 = v,
            _ => return false,
        }
        true
    }
}

impl NesMapped for ScrollBlocks {
    const DISCRIMINATOR: Option<u16> = None;

    /// 一对**相邻**字节 ⇒ 一个 2 B `Owner`（asm 是两个全局变量，但地址连续、读径
    /// `$6C+y` 横跨两者，拆开会让同一个逻辑量跨域）。
    const SPANS: &'static [(NesSpan, SpanKey)] = &[(
        NesSpan::value(Self::ADDRS[0], 2),
        SpanKey::Owner,
    )];

    fn project(&self, out: &mut crate::memmap::storage::NesImage) {
        out.ram[(Self::ADDRS[0] & 0x7FF) as usize] = self.nt3;
        out.ram[(Self::ADDRS[1] & 0x7FF) as usize] = self.nt0;
    }
}

impl Manager {
    /// `$6C ScrollBlockOnNameTable3`——名表 3 门挡位。
    pub(crate) fn scroll_block_nt3(&self) -> u8 {
        self.scroll_blocks.nt3()
    }

    pub(crate) fn set_scroll_block_nt3(&mut self, v: u8) {
        self.scroll_blocks.set_nt3(v);
    }

    /// `$6D ScrollBlockOnNameTable0`——名表 0 门挡位。
    pub(crate) fn scroll_block_nt0(&self) -> u8 {
        self.scroll_blocks.nt0()
    }

    pub(crate) fn set_scroll_block_nt0(&mut self, v: u8) {
        self.scroll_blocks.set_nt0(v);
    }
}

#[cfg(test)]
mod typed_tests {
    use super::*;
    use crate::frame::Manager;

    /// typed 是唯一真相：typed 写 ⇒ 字节面读回同值、storage 不留副本、
    /// `snapshot_nes()` 投影同源；字节面写 ⇒ typed 立刻可见。
    #[test]
    fn scroll_blocks_are_typed() {
        let mut m = Manager::default();
        m.set_scroll_block_nt3(0x02);
        m.set_scroll_block_nt0(0x01);
        assert_eq!(m.read_ram(crate::ram_map::SCROLL_BLOCK_NT3), 0x02);
        assert_eq!(m.read_ram(crate::ram_map::SCROLL_BLOCK_NT0), 0x01);
        // storage 不持有副本（typed 域辖字节清零）。
        assert_eq!(m.ram()[crate::ram_map::SCROLL_BLOCK_NT3 as usize], 0);
        assert_eq!(m.ram()[crate::ram_map::SCROLL_BLOCK_NT0 as usize], 0);
        // 投影同源。
        let img = m.snapshot_nes();
        assert_eq!(img.ram[crate::ram_map::SCROLL_BLOCK_NT3 as usize], 0x02);
        assert_eq!(img.ram[crate::ram_map::SCROLL_BLOCK_NT0 as usize], 0x01);
        // 字节面写 ⇒ typed 可见（写径也走 typed）。
        m.write_ram(crate::ram_map::SCROLL_BLOCK_NT0, 0x03);
        assert_eq!(m.scroll_block_nt0(), 0x03);
    }

    /// MSS 回灌：字节面是外部真相 ⇒ 恢复后 typed 重建；storage 随后清零。
    #[test]
    fn scroll_blocks_are_restored_and_cleared_from_mss() {
        let mut ram = vec![0u8; 0x800];
        ram[crate::ram_map::SCROLL_BLOCK_NT3 as usize] = 0x02;
        ram[crate::ram_map::SCROLL_BLOCK_NT0 as usize] = 0x01;
        let m = Manager::restore_mss(&ram, &vec![0u8; 0x2000], Default::default());
        assert_eq!(
            (m.scroll_block_nt3(), m.scroll_block_nt0()),
            (0x02, 0x01)
        );
        assert_eq!(m.ram()[crate::ram_map::SCROLL_BLOCK_NT3 as usize], 0);
        assert_eq!(m.ram()[crate::ram_map::SCROLL_BLOCK_NT0 as usize], 0);
    }
}
