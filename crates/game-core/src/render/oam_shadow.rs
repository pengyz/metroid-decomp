//! OAM 影子（`$0200-$02FF`，64 槽 × 4 B）的 **typed 真源**。
//!
//! 证据（A 级）：
//! - `refs/m1disasm/SRC/constants_ram.asm` `SpriteRAM ; 06:0200`（`dsb $100`——256 B
//!   页，即硬件 OAM 的 CPU 侧镜像；NMI `sta OAMADDR / sta OAMDMA` 整页 DMA）。
//! - 槽布局：`OAM[slot*4 + {0,1,2,3}] = {Y, tile, attributes, X}`——写者
//!   `render/oam.rs::draw_sprite_stream`（`PlaceSprite` 族，逐槽 4 B quad）、
//!   `ui/hud.rs`（HUD 精灵表拷贝）、`world/update.rs:132`（`$F4` 关屏哨兵）、
//!   `frame/boot.rs::erase_all_sprites`（`$F0` 整页初始化，`ClearSpriteRam`）。
//! - 读者：`frame/nmi.rs::sprite_state`（OAMDMA 前的整页源像）、渲染腿的
//!   `render_frame(..., oam)`。
//!
//! 形态纪律：asm 侧是一页 256 B 的连续 RAM，渲染腿/对拍腿都按字节页消费，故真相面
//! 保留 `[u8; 0x100]` 页；`OamEntry` 只是 4 B 槽的**语义视图**（不复制真相）。
//! 未 typed 前这页在 `storage.ram`，迁后 `read_ram/write_ram` 命中本域、`storage`
//! 不再持有副本（`snapshot_nes()` 单向投影回字节面）。

use crate::memmap::nesmap::{NesMapped, NesSpan, SpanKey};

/// OAM 页基址（`SpriteRAM=$0200`）。
pub const OAM_BASE: u16 = 0x0200;
/// OAM 页字节数（64 槽 × 4 B）。
pub const OAM_LEN: usize = 0x100;
/// 槽数。
pub const OAM_SLOTS: usize = 64;
/// 每槽字节数（Y/tile/attributes/X）。
pub const OAM_STRIDE: usize = 4;

/// 单个精灵槽的 4 字节语义视图（顺序即硬件 OAM 字节序）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct OamEntry {
    /// Y 坐标（PPU 侧带 +1 行偏置，`render/mod.rs` 处理）。
    pub y: u8,
    /// 图案号。
    pub tile: u8,
    /// 属性（调色板/翻转/优先级）。
    pub attr: u8,
    /// X 坐标。
    pub x: u8,
}

impl OamEntry {
    pub fn to_bytes(self) -> [u8; OAM_STRIDE] {
        [self.y, self.tile, self.attr, self.x]
    }

    pub fn from_bytes(b: [u8; OAM_STRIDE]) -> Self {
        Self {
            y: b[0],
            tile: b[1],
            attr: b[2],
            x: b[3],
        }
    }
}

/// OAM 页 typed 真源（`$0200-$02FF`）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OamShadow {
    page: [u8; OAM_LEN],
}

impl OamShadow {
    /// 整页（渲染腿/对拍腿的直接消费面）。
    pub fn page(&self) -> &[u8; OAM_LEN] {
        &self.page
    }

    /// 从整页字节构造（`restore_mss` 回灌用；调用方保证页长）。
    pub fn from_page(page: [u8; OAM_LEN]) -> Self {
        Self { page }
    }

    /// 页内字节（偏移按 0xFF 回绕——镜像语义由 `read_ram` 归一，这里只做域内取址）。
    pub fn get(&self, off: usize) -> u8 {
        self.page[off & 0xFF]
    }

    pub fn set(&mut self, off: usize, v: u8) {
        self.page[off & 0xFF] = v;
    }

    /// 槽视图（越界槽按 64 回绕，与 `$100` 页取址一致）。
    pub fn entry(&self, slot: usize) -> OamEntry {
        let base = (slot & (OAM_SLOTS - 1)) * OAM_STRIDE;
        OamEntry {
            y: self.page[base],
            tile: self.page[base + 1],
            attr: self.page[base + 2],
            x: self.page[base + 3],
        }
    }

    pub fn set_entry(&mut self, slot: usize, e: OamEntry) {
        let base = (slot & (OAM_SLOTS - 1)) * OAM_STRIDE;
        self.page[base..base + OAM_STRIDE].copy_from_slice(&e.to_bytes());
    }

    /// 整页填充（`ClearSpriteRam` 的 `$F0` 初始化、关屏 `$F4` 哨兵由调用方写单槽）。
    pub fn clear(&mut self, fill: u8) {
        self.page = [fill; OAM_LEN];
    }

    /// 地址 → 字节（不在本域返回 `None`）。
    pub fn get_byte(&self, addr: u16) -> Option<u8> {
        let off = addr.wrapping_sub(OAM_BASE) as usize;
        if off < OAM_LEN {
            Some(self.page[off])
        } else {
            None
        }
    }

    /// 字节 → 地址（不在本域返回 `false`，调用方继续往下一域分派）。
    pub fn set_byte(&mut self, addr: u16, v: u8) -> bool {
        let off = addr.wrapping_sub(OAM_BASE) as usize;
        if off < OAM_LEN {
            self.page[off] = v;
            true
        } else {
            false
        }
    }

    /// 迁入 typed 的地址清单（`restore_mss` 清零与 `snapshot_nes` 投影共用）。
    pub fn addrs() -> impl Iterator<Item = u16> {
        (0..OAM_LEN as u16).map(|i| OAM_BASE + i)
    }
}

impl Default for OamShadow {
    fn default() -> Self {
        Self {
            page: [0u8; OAM_LEN],
        }
    }
}

impl NesMapped for OamShadow {
    const DISCRIMINATOR: Option<u16> = None;

    /// 一段 256 B `Owner`（`$0200-$02FF`）——asm 侧就是一整页 `SpriteRAM`。
    const SPANS: &'static [(NesSpan, SpanKey)] =
        &[(NesSpan::value(OAM_BASE, OAM_LEN as u16), SpanKey::Owner)];

    fn project(&self, out: &mut crate::memmap::storage::NesImage) {
        let base = (OAM_BASE & 0x7FF) as usize;
        out.ram[base..base + OAM_LEN].copy_from_slice(&self.page);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame::Manager;

    /// 写径/读径都落 typed；字节面不持有 OAM 副本，投影同源；槽视图按 4 B 编址。
    #[test]
    fn oam_page_is_typed_and_projected() {
        let mut m = Manager::default();
        m.write_ram(0x0200, 0x2F);
        m.write_ram(0x0201, 0x18);
        m.write_ram(0x0202, 0x21);
        m.write_ram(0x0203, 0x40);
        assert_eq!(m.read_ram(0x0200), 0x2F);
        assert_eq!(m.read_ram(0x02FF), 0x00);
        assert_eq!(m.ram()[0x0200], 0, "字节面不持有 typed 字节");
        let e = m.oam_shadow_entry(0);
        assert_eq!((e.y, e.tile, e.attr, e.x), (0x2F, 0x18, 0x21, 0x40));
        let img = m.snapshot_nes();
        assert_eq!(&img.ram[0x200..0x204], &[0x2F, 0x18, 0x21, 0x40]);
        assert_eq!(img.ram[0x2FF], 0);
        // 整页视图与读口同源。
        assert_eq!(m.oam_shadow()[0], 0x2F);
    }

    /// `restore_mss` 回灌后整页 typed 与字节输入一致、storage 清零。
    #[test]
    fn oam_page_is_restored_and_cleared_from_mss() {
        let mut ram = vec![0u8; 0x800];
        ram[0x0200] = 0x12;
        ram[0x02FF] = 0xF0;
        let wram = vec![0u8; 0x2000];
        let m = Manager::restore_mss(&ram, &wram, crate::state::Registers::default());
        assert_eq!(m.oam_shadow()[0x00], 0x12);
        assert_eq!(m.oam_shadow()[0xFF], 0xF0);
        assert_eq!(m.ram()[0x0200], 0, "typed 格在 storage 清零");
        assert_eq!(m.ram()[0x02FF], 0);
    }

    /// 整页填充 = `ClearSpriteRam` 的 `$F0` 形态；槽编号跨 64 回绕。
    #[test]
    fn oam_page_clear_and_slot_wrap() {
        let mut o = OamShadow::default();
        o.clear(0xF0);
        assert!(o.page().iter().all(|&b| b == 0xF0));
        o.set_entry(0, OamEntry::from_bytes([1, 2, 3, 4]));
        assert_eq!(o.entry(0).to_bytes(), [1, 2, 3, 4]);
        assert_eq!(o.entry(OAM_SLOTS).to_bytes(), [1, 2, 3, 4], "槽号按 64 回绕");
    }
}

impl crate::state::Manager {
    /// OAM 整页（`$0200-$02FF`）typed 视图——渲染腿/对拍腿的唯一取页入口。
    pub fn oam_shadow(&self) -> &[u8; OAM_LEN] {
        self.oam.page()
    }

    /// 单槽语义视图（调试/装配用；生产写径走 `write_ram`）。
    pub fn oam_shadow_entry(&self, slot: usize) -> OamEntry {
        self.oam.entry(slot)
    }
}
