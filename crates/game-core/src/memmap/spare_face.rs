//! game-core::memmap::spare_face —— **未命名尾段面（155 B）的 typed 声明层**（B1-d）。
//!
//! 【这一面是什么】NES RAM 里三段 asm **没有命名**的字节，长期挂在 `nes-map` 的
//! Undeclared 上（合计 155 B）：
//! - `$068E-$06FF`（114 B）：asm 无命名，C 级假设，未取证。位置在`refs/m1disasm/SRC/constants_ram.asm:999-1165` 的 `.enum $0600`
//!   （声引擎 RAM，`SoundRamFace` 认领到 `$068D`）**之后**、`:1169` 的 `.enum $0700`
//!   （房间精灵/PPU 写队列表）**之前**。asm 无符号 ⇒ 语义未知。
//! - `$0787-$079F`（25 B）：asm 无命名，C 级假设，未取证。同文件 `:1188` 的 `SpareMem0787 ds $19 ;$0787`（asm 自述「spare」）。
//! - `$07F0-$07FF`（16 B）：asm 无命名，C 级假设，未取证。`.enum $0700`（`:1169-1206`）结束于 `$07EF`，之后到 `$0800` 之间
//!   **不在任何 `.enum`** 内。
//!
//! 【证据级别与命名纪律】三段都**没有可用语义**：全部字段按 `_todo_` 命名并登记 C 级
//! （AGENTS §3.2 禁止伪语义命名；任务卡第 3 条允许「不确定语义保留 `_todo_` + C 级」）。
//! 本模块**不猜测**这些字节的用途，只做三件事：①给出可审计的字节边界与 asm 指针；
//! ②提供 typed 读写（供未来 P4 迁移与调试探针使用）；③把它们登记进
//! [`crate::memmap::nesmap::entity_spans`]，使 `nes-map` 的四态恒等式把这 155 B 计入 Owner。
//!
//! 【为什么声明它们比留在 Undeclared 好】Undeclared 的语义是「**没有任何模块声明归属**」；
//! 而这 155 B 属于「已知在 RAM 面内、已知边界、但语义未考证」的第三类。把它们登记成 Owner
//! 后，`nes-map` 的 Undeclared 就只剩「尚未确认真假的洞」，审计信号更锐利；语义考据仍可
//! 后续单独开卡，届时只需在本模块内把 `_todo_` 改成实名（不牵动业务层）。
//!
//! 【不变量】本模块只提供**按需解码**的视图，不持有跨帧状态；字节真源仍是 `storage`。
//! `decode`/`encode` 以 `&0x7FF` 镜像视图访问，与 `snapshot_nes()` 同源同值。

use crate::memmap::nesmap::{NesMapped, NesSpan, SpanKey};
use crate::memmap::storage::NesImage;

/// `$068E-$06FF`：asm 无命名，C 级假设，未取证（`.enum $0600` 之后、`.enum $0700` 之前）。
pub const TODO_068E_BASE: u16 = 0x068E;
/// 见 [`TODO_068E_BASE`]。
pub const TODO_068E_LEN: usize = 0x06FF - 0x068E + 1;
/// `$0787-$079F`：asm 无命名，C 级假设，未取证（对应 `SpareMem0787 ds $19`）。
pub const TODO_0787_BASE: u16 = 0x0787;
/// 见 [`TODO_0787_BASE`]。
pub const TODO_0787_LEN: usize = 0x19;
/// `$07F0-$07FF`：asm 无命名，C 级假设，未取证（`.enum $0700` 结束于 `$07EF`）。
pub const TODO_07F0_BASE: u16 = 0x07F0;
/// 见 [`TODO_07F0_BASE`]。
pub const TODO_07F0_LEN: usize = 0x10;

/// 本面三段合计字节数（`_list` 里 Owner 应因此净增这么多）。
pub const FACE_LEN: usize = TODO_068E_LEN + TODO_0787_LEN + TODO_07F0_LEN;

/// 未命名尾段面（C 级：字段语义待考）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpareFace {
    /// `$068E-$06FF`（114 B，C 级）。
    pub _todo_068e: [u8; TODO_068E_LEN],
    /// `$0787-$079F`（25 B，asm `SpareMem0787`，C 级）。
    pub _todo_0787: [u8; TODO_0787_LEN],
    /// `$07F0-$07FF`（16 B，C 级）。
    pub _todo_07f0: [u8; TODO_07F0_LEN],
}

impl Default for SpareFace {
    fn default() -> Self {
        Self {
            _todo_068e: [0; TODO_068E_LEN],
            _todo_0787: [0; TODO_0787_LEN],
            _todo_07f0: [0; TODO_07F0_LEN],
        }
    }
}

impl SpareFace {
    /// 该地址是否落在本面三段之内。
    pub fn contains(addr: u16) -> bool {
        Self::segment_of(addr).is_some()
    }

    /// 地址落在哪一段：`(段基址, 段长)`。
    fn segment_of(addr: u16) -> Option<(u16, usize)> {
        let segs: [(u16, usize); 3] = [
            (TODO_068E_BASE, TODO_068E_LEN),
            (TODO_0787_BASE, TODO_0787_LEN),
            (TODO_07F0_BASE, TODO_07F0_LEN),
        ];
        segs.into_iter()
            .find(|&(base, len)| addr >= base && (addr as usize) < base as usize + len)
    }

    /// 从 2 KB RAM 视图解出本面（越界缺失按 0 填）。
    pub fn decode(ram: &[u8]) -> Self {
        let mut face = Self::default();
        for (i, b) in face._todo_068e.iter_mut().enumerate() {
            *b = ram[((TODO_068E_BASE as usize + i) & 0x7FF)];
        }
        for (i, b) in face._todo_0787.iter_mut().enumerate() {
            *b = ram[((TODO_0787_BASE as usize + i) & 0x7FF)];
        }
        for (i, b) in face._todo_07f0.iter_mut().enumerate() {
            *b = ram[((TODO_07F0_BASE as usize + i) & 0x7FF)];
        }
        face
    }

    /// 写回 2 KB RAM 视图（只触碰本面三段）。
    pub fn encode(&self, ram: &mut [u8]) {
        for (i, b) in self._todo_068e.iter().enumerate() {
            ram[(TODO_068E_BASE as usize + i) & 0x7FF] = *b;
        }
        for (i, b) in self._todo_0787.iter().enumerate() {
            ram[(TODO_0787_BASE as usize + i) & 0x7FF] = *b;
        }
        for (i, b) in self._todo_07f0.iter().enumerate() {
            ram[(TODO_07F0_BASE as usize + i) & 0x7FF] = *b;
        }
    }

    /// 面内索引化访问：`addr` 必须落在本面内，否则 `None`。
    pub fn get_byte(&self, addr: u16) -> Option<u8> {
        let (base, _) = Self::segment_of(addr)?;
        let off = (addr - base) as usize;
        Some(match base {
            TODO_068E_BASE => self._todo_068e[off],
            TODO_0787_BASE => self._todo_0787[off],
            _ => self._todo_07f0[off],
        })
    }

    /// 面内索引化写入：`addr` 不在本面内则返回 `false` 且不落值。
    pub fn set_byte(&mut self, addr: u16, value: u8) -> bool {
        let Some((base, _)) = Self::segment_of(addr) else {
            return false;
        };
        let off = (addr - base) as usize;
        match base {
            TODO_068E_BASE => self._todo_068e[off] = value,
            TODO_0787_BASE => self._todo_0787[off] = value,
            _ => self._todo_07f0[off] = value,
        }
        true
    }
}

impl NesMapped for SpareFace {
    const DISCRIMINATOR: Option<u16> = None;

    /// 三段各一条 `Owner` span（与 asm 无命名边界一致：不合并成一条，避免假装连续）。
    const SPANS: &'static [(NesSpan, SpanKey)] = &[
        (NesSpan::value(TODO_068E_BASE, TODO_068E_LEN as u16), SpanKey::Owner),
        (
            NesSpan::value(TODO_0787_BASE, TODO_0787_LEN as u16),
            SpanKey::Owner,
        ),
        (
            NesSpan::value(TODO_07F0_BASE, TODO_07F0_LEN as u16),
            SpanKey::Owner,
        ),
    ];

    fn project(&self, out: &mut NesImage) {
        self.encode(&mut out.ram);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn image() -> NesImage {
        NesImage {
            ram: [0u8; 0x800],
            wram: [0u8; 0x2000],
        }
    }

    #[test]
    fn face_geometry_matches_asm_boundaries() {
        assert_eq!(FACE_LEN, 155, "三段合计必须 155 B（114 + 25 + 16）");
        assert_eq!(TODO_068E_BASE, 0x068E);
        assert_eq!(TODO_068E_BASE as usize + TODO_068E_LEN - 1, 0x06FF);
        assert_eq!(TODO_0787_BASE + TODO_0787_LEN as u16 - 1, 0x079F);
        assert_eq!(TODO_07F0_BASE + TODO_07F0_LEN as u16 - 1, 0x07FF);
    }

    #[test]
    fn spans_cover_three_segments_without_overlap() {
        let mut covered = Vec::new();
        for (span, key) in SpareFace::SPANS {
            assert!(matches!(key, SpanKey::Owner), "三段都必须登记 Owner");
            for a in span.start..span.start + span.len {
                covered.push(a);
            }
        }
        covered.sort_unstable();
        let before = covered.len();
        covered.dedup();
        assert_eq!(before, covered.len(), "span 之间不得重叠");
        assert_eq!(covered.len(), FACE_LEN, "span 覆盖必须恰好等于面长");
    }

    #[test]
    fn byte_accessors_cover_exactly_the_face() {
        let mut face = SpareFace::default();
        let mut covered = 0usize;
        for a in 0u16..0x0800 {
            let inside = SpareFace::contains(a);
            assert_eq!(inside, face.get_byte(a).is_some(), "contains 与 get_byte 必须同判");
            if inside {
                covered += 1;
                assert!(face.set_byte(a, (a & 0xFF) as u8));
                assert_eq!(face.get_byte(a), Some((a & 0xFF) as u8));
            } else {
                assert!(!face.set_byte(a, 0xAA), "面外地址不得被写入");
            }
        }
        assert_eq!(covered, FACE_LEN);
    }

    #[test]
    fn face_round_trip_is_byte_exact() {
        for seed in [0u8, 0xFF, 0x37, 0x91] {
            let mut ram = [0u8; 0x800];
            let mut v = seed.wrapping_mul(31).wrapping_add(7);
            for b in ram.iter_mut() {
                v = v.wrapping_mul(37).wrapping_add(11);
                *b = v;
            }
            let face = SpareFace::decode(&ram);
            let mut out = [0u8; 0x800];
            face.encode(&mut out);
            for a in 0u16..0x0800 {
                if SpareFace::contains(a) {
                    assert_eq!(out[a as usize], ram[a as usize], "面内 {a:#06X} 往返必须逐字节相等");
                } else {
                    assert_eq!(out[a as usize], 0, "面外 {a:#06X} 不得被写");
                }
            }
        }
    }

    #[test]
    fn project_writes_only_this_face() {
        let mut ram = [0u8; 0x800];
        for (i, b) in ram.iter_mut().enumerate() {
            *b = (i & 0xFF) as u8;
        }
        let face = SpareFace::decode(&ram);
        let mut img = image();
        face.project(&mut img);
        for a in 0u16..0x0800 {
            if SpareFace::contains(a) {
                assert_eq!(img.ram[a as usize], ram[a as usize]);
            } else {
                assert_eq!(img.ram[a as usize], 0, "project 只写本面三段");
            }
        }
    }
}
