//! game-core::memmap::tile_blast_face —— **TileBlasts 状态面（`$0500-$05CF`）的 typed 声明层**
//! （B1-a 首批：把「已实现但零声明」的 208 B 升格为可审计的 `Owner` 段）。
//!
//! 【为什么这一面此前是 Undeclared】行为是移植过的（`crate::world::tile_blast`：
//! `UpdateAllTileBlasts ; 07:FE1B` / `UpdateTileBlast ; 07:FE27` / `DrawTileBlast ; 07:FEDC` /
//! `UpdateTileBlastAnim ; 07:FF54`，bank7），但读写全走 `Manager::tb_*` 的**裸地址**面
//! （`crate::frame::memory::Manager::tb_addr(slot, off) = $0500 + slot*16 + off`），声明的
//! 字段布局只活在那 9 个 `F_*` 偏移常量里 ⇒ `nes-map` 审计看不到这 208 B 的归属。
//!
//! 【本模块只做两件事】①把 asm 结构升格成 typed 表示（字段名/类型/偏移，逐字段带 asm 锚）；
//! ②把该面登记进 [`crate::memmap::nesmap::entity_spans`]。**行为零变化**：
//! `Manager::tb_*` 仍是唯一读写路径；[`TileBlastFace::project`] 是同一批字节的编码出口
//! （与 `snapshot_nes()` 同源同值），[`TileBlastFace::decode`] 是解码入口，往返互检进单测。
//!
//! 【声明层为什么落在 `memmap/` 而不在 `world/`】B1-a 的硬约束是「只动 `memmap/`，不改业务层
//! 访问点」。`memmap/mod.rs` 的分层口径本就规定**地址常量与 codec 只活在本层**
//! （「语义面不在这里」指的是实体聚合根），本模块正是该口径下的「槽语义视图 + 字节偏移契约面」。
//!
//! 【P4 迁移方向（`docs/design/fc-memory-projection-architecture.md` §4.1）】该族转为常驻
//! typed 真源时：`Manager::tb_*` 改为委托本结构、`storage` 不再持有这 208 B、`world/
//! tile_blast.rs` 的 `F_*` 常量改为引用本模块（单一真相源）。在那之前本结构是**按需解码**，
//! 不持有跨帧状态——这也是 `Owner` 在本层的判据：这批字节的权威**解释**（字段布局）只有这里
//! 一份，字节真源仍是 `storage`。
//!
//! 【asm 证据】
//! - 槽布局：`refs/m1disasm/SRC/constants_ram.asm:36-47` 的 `.struct TileBlast` ——
//!   `routine db` / `spare01 dw` / `animFrame db` / `animDelay db` / `spare05 db` /
//!   `animIndex db` / `delay db` / `roomRAMPtr dw` / `type db` / `spare0B ds 5`（合计 16 B）。
//! - 槽数组：同文件 `:984-987` `.enum $0500 export` + `TileBlasts instanceof TileBlast $D
//!   startfrom 0`（13 槽，注释「Tile respawning」）。
//! - 字段语义（B 级：调用点语境 + `UpdateTileBlast` 的 6 项 `JumpEngine` 表）：
//!   · `routine`(+0)：五态机指针 —— 0 空 / 1 起爆等待 / 2 炸开动画 / 3 等待复原 /
//!     4 复原动画 / 5 复原完成；
//!   · `animFrame`(+3)：**VRAM 字符串 id**（`DrawTileBlast ; 07:FEDC` 经 `VRAMStringPtrTable`
//!     取按区不同的图案，见 `assets::vram_string`）；
//!   · `animDelay`(+4)：动画帧间隔计数（动画每 2 帧一换）；
//!   · `animIndex`(+6)：动画串内游标（`UpdateTileBlastAnim` 自增/回滚）；
//!   · `delay`(+7)：复原等待计数（`UpdateTileBlast_Init` 初值 `lda #$50`）；
//!   · `roomRAMPtr`(+8，小端)：RoomRAM 内元 tile 指针（`draw_tile_blast_slot` 的写入起点；
//!     占槽时 `and #$DE` 掩掉 bit0/bit5，见 `claim_tile_blast`）；
//!   · `type`(+10)：碎块类型（`TileBlastRespawnAnimIndexTable ; 07:FE79` 的下标）；
//!   · `spare01`/`spare05`/`spare0B`：asm 声明但全仓无引用（保留占位；字节仍归本面，
//!     按原样 round-trip，不赋予猜测语义）。

use crate::memmap::nesmap::{NesMapped, NesSpan, SpanKey};
use crate::memmap::storage::NesImage;

/// 面基址（`constants_ram.asm:984` 的 `.enum $0500`）。
pub const BASE: u16 = 0x0500;
/// 槽长（`_sizeof_TileBlast` = 16）。
pub const SLOT_LEN: u16 = 16;
/// 槽数（`instanceof TileBlast $D`）。
pub const SLOTS: usize = 13;
/// 面长（13 × 16 = 208 B，`$0500-$05CF`）。
pub const FACE_LEN: u16 = (SLOTS as u16) * SLOT_LEN;

// 槽内字段偏移（与 asm 的 `TileBlasts.0.<field>,x` 结构化访问逐项对齐）。
const F_ROUTINE: u16 = 0;
const F_SPARE01: u16 = 1; // dw
const F_ANIM_FRAME: u16 = 3;
const F_ANIM_DELAY: u16 = 4;
const F_SPARE05: u16 = 5;
const F_ANIM_INDEX: u16 = 6;
const F_DELAY: u16 = 7;
const F_ROOM_RAM_PTR: u16 = 8; // dw（小端）
const F_TYPE: u16 = 10;
const F_SPARE0B: u16 = 11; // ds 5

/// 单槽（`TileBlast` 16 B）的 typed 表示。
///
/// 字段与 `constants_ram.asm:36-47` 一一对应；未赋义的 `spare*` 按原字节保留。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TileBlastSlot {
    /// +0 `routine`：五态机指针（0 空 / 1 起爆等待 / 2 炸开动画 / 3 等待复原 / 4 复原动画 / 5 完成）。
    pub routine: u8,
    /// +1 `spare01`（asm `dw`，无引用）：按原字节保留。
    pub spare01: [u8; 2],
    /// +3 `animFrame`：VRAM 字符串 id（`VRAMStringPtrTable` 下标）。
    pub anim_frame: u8,
    /// +4 `animDelay`：动画帧间隔计数。
    pub anim_delay: u8,
    /// +5 `spare05`（无引用）：按原字节保留。
    pub spare05: u8,
    /// +6 `animIndex`：动画串内游标。
    pub anim_index: u8,
    /// +7 `delay`：复原等待计数（初值 `$50`）。
    pub delay: u8,
    /// +8 `roomRAMPtr`（小端）：RoomRAM 元 tile 指针。
    pub room_ram_ptr: u16,
    /// +10 `type`：碎块类型（`TileBlastRespawnAnimIndexTable` 下标）。
    pub kind: u8,
    /// +11 `spare0B`（`ds 5`，无引用）：按原字节保留。
    pub spare0b: [u8; 5],
}

impl TileBlastSlot {
    /// 槽内字节解码（`bytes` 长度必须 ≥ [`SLOT_LEN`]）。
    pub fn decode(bytes: &[u8]) -> Self {
        debug_assert!(bytes.len() >= SLOT_LEN as usize, "槽解码需要 16 B");
        Self {
            routine: bytes[usize::from(F_ROUTINE)],
            spare01: [bytes[usize::from(F_SPARE01)], bytes[usize::from(F_SPARE01) + 1]],
            anim_frame: bytes[usize::from(F_ANIM_FRAME)],
            anim_delay: bytes[usize::from(F_ANIM_DELAY)],
            spare05: bytes[usize::from(F_SPARE05)],
            anim_index: bytes[usize::from(F_ANIM_INDEX)],
            delay: bytes[usize::from(F_DELAY)],
            // 6502 小端：低字节在前（`lda TileBlasts.0.roomRAMPtr,x` / `... + 1,x`）。
            room_ram_ptr: u16::from_le_bytes([
                bytes[usize::from(F_ROOM_RAM_PTR)],
                bytes[usize::from(F_ROOM_RAM_PTR) + 1],
            ]),
            kind: bytes[usize::from(F_TYPE)],
            spare0b: [
                bytes[usize::from(F_SPARE0B)],
                bytes[usize::from(F_SPARE0B) + 1],
                bytes[usize::from(F_SPARE0B) + 2],
                bytes[usize::from(F_SPARE0B) + 3],
                bytes[usize::from(F_SPARE0B) + 4],
            ],
        }
    }

    /// 槽内字节编码（`bytes` 长度必须 ≥ [`SLOT_LEN`]）。
    pub fn encode(&self, bytes: &mut [u8]) {
        debug_assert!(bytes.len() >= SLOT_LEN as usize, "槽编码需要 16 B");
        bytes[usize::from(F_ROUTINE)] = self.routine;
        bytes[usize::from(F_SPARE01)] = self.spare01[0];
        bytes[usize::from(F_SPARE01) + 1] = self.spare01[1];
        bytes[usize::from(F_ANIM_FRAME)] = self.anim_frame;
        bytes[usize::from(F_ANIM_DELAY)] = self.anim_delay;
        bytes[usize::from(F_SPARE05)] = self.spare05;
        bytes[usize::from(F_ANIM_INDEX)] = self.anim_index;
        bytes[usize::from(F_DELAY)] = self.delay;
        let ptr = self.room_ram_ptr.to_le_bytes();
        bytes[usize::from(F_ROOM_RAM_PTR)] = ptr[0];
        bytes[usize::from(F_ROOM_RAM_PTR) + 1] = ptr[1];
        bytes[usize::from(F_TYPE)] = self.kind;
        bytes[usize::from(F_SPARE0B)..usize::from(F_SPARE0B) + 5].copy_from_slice(&self.spare0b);
    }
}

/// TileBlasts 整面（13 槽）的 typed 表示。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TileBlastFace {
    slots: [TileBlastSlot; SLOTS],
}

impl TileBlastFace {
    /// 从 2KB RAM 镜像面解码（`$0500-$05CF`，地址按 `& 0x7FF` 归一）。
    pub fn decode(ram: &[u8]) -> Self {
        let mut slots = [TileBlastSlot::default(); SLOTS];
        for (i, slot) in slots.iter_mut().enumerate() {
            let base = usize::from(BASE & 0x7FF) + i * usize::from(SLOT_LEN);
            *slot = TileBlastSlot::decode(&ram[base..base + usize::from(SLOT_LEN)]);
        }
        Self { slots }
    }

    /// 编码回 2KB RAM 镜像面（只写 `$0500-$05CF`）。
    pub fn encode(&self, ram: &mut [u8]) {
        for (i, slot) in self.slots.iter().enumerate() {
            let base = usize::from(BASE & 0x7FF) + i * usize::from(SLOT_LEN);
            slot.encode(&mut ram[base..base + usize::from(SLOT_LEN)]);
        }
    }

    /// 槽视图（越界返回 `None`，不 panic：审计与诊断常按外部下标取）。
    pub fn slot(&self, index: usize) -> Option<&TileBlastSlot> {
        self.slots.get(index)
    }

    /// 槽可变视图（越界返回 `None`）。
    pub fn slot_mut(&mut self, index: usize) -> Option<&mut TileBlastSlot> {
        self.slots.get_mut(index)
    }

    /// 槽 → 绝对地址（`$0500 + index*16`）。
    pub fn slot_addr(index: usize) -> u16 {
        BASE + (index as u16) * SLOT_LEN
    }

    /// 地址 → 槽号（不在本面返回 `None`）。
    pub fn slot_of(addr: u16) -> Option<usize> {
        let a = addr & 0x7FF;
        if !(BASE..BASE + FACE_LEN).contains(&a) {
            return None;
        }
        Some(usize::from((a - BASE) / SLOT_LEN))
    }

    /// 地址 → 字节（不在本面返回 `None`）；供将来接入 `Manager::read_ram` 分派用。
    pub fn get_byte(&self, addr: u16) -> Option<u8> {
        let index = Self::slot_of(addr)?;
        let off = usize::from((addr & 0x7FF) - Self::slot_addr(index));
        let mut bytes = [0u8; SLOT_LEN as usize];
        self.slots[index].encode(&mut bytes);
        Some(bytes[off])
    }

    /// 字节 → 地址（不在本面返回 `false`，调用方继续往下一域分派）。
    pub fn set_byte(&mut self, addr: u16, v: u8) -> bool {
        let Some(index) = Self::slot_of(addr) else {
            return false;
        };
        let off = usize::from((addr & 0x7FF) - Self::slot_addr(index));
        let mut bytes = [0u8; SLOT_LEN as usize];
        self.slots[index].encode(&mut bytes);
        bytes[off] = v;
        self.slots[index] = TileBlastSlot::decode(&bytes);
        true
    }
}

impl NesMapped for TileBlastFace {
    const DISCRIMINATOR: Option<u16> = None;

    /// 13 段 × 16 B 的 `Owner`（`$0500-$05CF`，逐槽一段：槽是 asm 的数组元素，
    /// 槽内字段不拆段——拆开只会让 `owner_spans` 噪声化，不增加可审计信息）。
    ///
    /// 手写 13 对（不循环生成）：`SPANS` 是 `const`，且 `--write` 的「既有 Owner span 不得
    /// 变形」判据按 `(start,len)` 精确比对，显式几何比生成式更容易被复核。
    const SPANS: &'static [(NesSpan, SpanKey)] = &[
        (NesSpan::value(BASE, SLOT_LEN), SpanKey::Owner),
        (NesSpan::value(BASE + 1 * SLOT_LEN, SLOT_LEN), SpanKey::Owner),
        (NesSpan::value(BASE + 2 * SLOT_LEN, SLOT_LEN), SpanKey::Owner),
        (NesSpan::value(BASE + 3 * SLOT_LEN, SLOT_LEN), SpanKey::Owner),
        (NesSpan::value(BASE + 4 * SLOT_LEN, SLOT_LEN), SpanKey::Owner),
        (NesSpan::value(BASE + 5 * SLOT_LEN, SLOT_LEN), SpanKey::Owner),
        (NesSpan::value(BASE + 6 * SLOT_LEN, SLOT_LEN), SpanKey::Owner),
        (NesSpan::value(BASE + 7 * SLOT_LEN, SLOT_LEN), SpanKey::Owner),
        (NesSpan::value(BASE + 8 * SLOT_LEN, SLOT_LEN), SpanKey::Owner),
        (NesSpan::value(BASE + 9 * SLOT_LEN, SLOT_LEN), SpanKey::Owner),
        (NesSpan::value(BASE + 10 * SLOT_LEN, SLOT_LEN), SpanKey::Owner),
        (NesSpan::value(BASE + 11 * SLOT_LEN, SLOT_LEN), SpanKey::Owner),
        (NesSpan::value(BASE + 12 * SLOT_LEN, SLOT_LEN), SpanKey::Owner),
    ];

    fn project(&self, out: &mut NesImage) {
        self.encode(&mut out.ram);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 面几何必须等于 asm：13 槽 × 16 B、起点 `$0500`、终点 `$05CF`。
    #[test]
    fn face_geometry_matches_asm_slots() {
        assert_eq!(BASE, 0x0500, "`.enum $0500 export`");
        assert_eq!(SLOT_LEN, 16, "`_sizeof_TileBlast`");
        assert_eq!(SLOTS, 13, "`instanceof TileBlast $D`");
        assert_eq!(FACE_LEN, 208);
        assert_eq!(TileBlastFace::slot_addr(0), 0x0500);
        assert_eq!(TileBlastFace::slot_addr(12) + SLOT_LEN, 0x05D0);
    }

    /// 声明表必须恰好是 13 段 `Owner`、连续覆盖 `$0500-$05D0`、无重叠。
    #[test]
    fn spans_are_thirteen_contiguous_owner_slots() {
        let spans = <TileBlastFace as NesMapped>::SPANS;
        assert_eq!(spans.len(), SLOTS);
        let mut expect = BASE;
        for (span, key) in spans {
            assert_eq!(span.start, expect, "槽段必须连续");
            assert_eq!(span.len, SLOT_LEN);
            assert!(matches!(key, SpanKey::Owner));
            expect += span.len;
        }
        assert_eq!(expect, BASE + FACE_LEN);
    }

    /// 字段偏移必须与 `.struct TileBlast` 逐项一致（每个字段填唯一哨兵值再定位）。
    #[test]
    fn field_offsets_match_asm_struct() {
        let mut bytes = [0u8; 16];
        let slot = TileBlastSlot {
            routine: 0x11,
            spare01: [0x22, 0x23],
            anim_frame: 0x33,
            anim_delay: 0x44,
            spare05: 0x55,
            anim_index: 0x66,
            delay: 0x77,
            room_ram_ptr: 0x9988, // 小端 ⇒ lo=0x88 @+8 / hi=0x99 @+9
            kind: 0xAA,
            spare0b: [0xB0, 0xB1, 0xB2, 0xB3, 0xB4],
        };
        slot.encode(&mut bytes);
        assert_eq!(bytes, [0x11, 0x22, 0x23, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xAA, 0xB0, 0xB1, 0xB2, 0xB3, 0xB4]);
        assert_eq!(TileBlastSlot::decode(&bytes), slot, "decode∘encode 必须还原");
    }

    /// 任意字节面往返：`encode(decode(ram)) == ram`（含 spare 字节，不丢信息）。
    #[test]
    fn face_round_trip_is_byte_exact() {
        let mut ram = [0u8; 0x800];
        // 三种固定形态 + LCG 伪随机（确定性：不引外部随机源）。
        for case in 0..6u32 {
            let mut seed = 0x1234_5678u32 ^ case;
            for byte in ram.iter_mut() {
                *byte = if case == 0 {
                    0x00
                } else if case == 1 {
                    0xFF
                } else {
                    seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                    (seed >> 24) as u8
                };
            }
            let face = TileBlastFace::decode(&ram);
            let mut out = ram;
            face.encode(&mut out);
            assert_eq!(out, ram, "case {case}：面内字节必须逐字节往返");
        }
    }

    /// 面外字节不受影响：`encode` 只写 `$0500-$05CF`。
    #[test]
    fn encode_touches_only_the_face() {
        let mut ram = [0x5Au8; 0x800];
        let face = TileBlastFace::decode(&ram); // 全 0x5A ⇒ 全槽 0x5A
        ram[usize::from(BASE & 0x7FF) - 1] = 0x11;
        ram[usize::from(BASE & 0x7FF) + usize::from(FACE_LEN)] = 0x22;
        let (before_lo, before_hi) = (
            ram[usize::from(BASE & 0x7FF) - 1],
            ram[usize::from(BASE & 0x7FF) + usize::from(FACE_LEN)],
        );
        face.encode(&mut ram);
        assert_eq!(ram[usize::from(BASE & 0x7FF) - 1], before_lo, "$04FF 不得被写");
        assert_eq!(
            ram[usize::from(BASE & 0x7FF) + usize::from(FACE_LEN)],
            before_hi,
            "$05D0 不得被写"
        );
        let after = TileBlastFace::decode(&ram);
        for i in 0..SLOTS {
            assert_eq!(after.slots[i].routine, 0x5A, "槽 {i} 仍应是 0x5A 面");
        }
    }

    /// `get_byte`/`set_byte` 覆盖整面且不越界（将来接入 `Manager::read_ram` 分派的契约）。
    #[test]
    fn byte_accessors_cover_exactly_the_face() {
        let mut face = TileBlastFace::default();
        for addr in BASE..BASE + FACE_LEN {
            assert!(face.get_byte(addr).is_some(), "{addr:#06X} 应在面内");
            assert!(face.set_byte(addr, (addr & 0xFF) as u8), "{addr:#06X} 应可写");
        }
        for addr in BASE..BASE + FACE_LEN {
            assert_eq!(face.get_byte(addr), Some((addr & 0xFF) as u8), "{addr:#06X} 值回读");
        }
        assert_eq!(face.get_byte(BASE - 1), None, "$04FF 不在本面");
        assert_eq!(face.get_byte(BASE + FACE_LEN), None, "$05D0 不在本面");
        assert!(!face.set_byte(BASE + FACE_LEN, 1), "面外写必须被拒");
        // 镜像归一：`$0D00` 与 `$0500` 是同一格（2KB 镜像面）。
        assert_eq!(face.get_byte(0x0D00), face.get_byte(BASE));
    }

    /// `project` 只写本面字节（`NesMapped` 契约）。
    #[test]
    fn project_writes_only_this_face() {
        let mut ram = [0xA5u8; 0x800];
        let face = TileBlastFace::decode(&ram);
        let mut img = NesImage {
            ram: [0x3Cu8; 0x800],
            wram: [0u8; 0x2000],
        };
        face.project(&mut img);
        for addr in 0..0x800usize {
            let in_face = (usize::from(BASE)..usize::from(BASE + FACE_LEN)).contains(&addr);
            if in_face {
                assert_eq!(img.ram[addr], 0xA5, "$ {addr:#06X} 属于本面，应被投影");
            } else {
                assert_eq!(img.ram[addr], 0x3C, "${addr:#06X} 不属于本面，不得被写");
            }
        }
    }
}
