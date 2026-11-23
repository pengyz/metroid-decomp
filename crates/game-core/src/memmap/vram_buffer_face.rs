//! game-core::memmap::vram_buffer_face —— **PPU 写缓冲面（`$0780-$0786` + `$07A0-$07EF`）的 typed 声明层**
//! （B1-b 第二批：把「已实现但零声明」的 87 B 升格为可审计的 `Owner` 段）。
//!
//! 【这一面是什么】`constants_ram.asm:1185-1204` 的 `.enum $0700` 尾部两块：
//!
//! | 地址 | asm | 长 | 语义 |
//! |---|---|---|---|
//! | `$0780` | `VRAMStringRAMDimensions db` | 1 B | 4 MSB = 高（tile）、4 LSB = 宽（tile） |
//! | `$0781-$0786` | `VRAMStringRAMBuffer ds 6` | 6 B | 待写进 PPU 的 tile id 串 |
//! | `$07A0` | `VRAMStructBufferIndex db` | 1 B | `VRAMStructBuffer` 已用字节数（`#$4F` 上限；写用、读以 `#$00` 终止） |
//! | `$07A1-$07EF` | `VRAMStructBuffer ds $4F` | 79 B | `VRAMStruct` 串（见下协议） |
//!
//! 【VRAMStruct 协议（asm 注释 + 移植侧证据）】每个 `VRAMStructData` 的前两字节是 PPU 目标
//! 地址（高、低），第三字节是配置字节：bit7 = 每字节写后 PPU +32（竖写），清 = +1（横写）；
//! bit0-5 = 要写的数据字节数；bit6 = 重复下一字节；`#$00` 终止整个缓冲。移植侧同一协议的
//! 写点：`crate::title` 的 `WritePPUByte`（`bank07:C36B`，满 `cpx #$4F` 时回填段首）与
//! `EndVRAMStruct`（`bank07:C376`，在当前 X 写 `$00` 终止符），见
//! `crates/game-core/src/title/mod.rs:1601-1670`（含 `unported!` fail-loud 的越界分支）。
//!
//! 【为什么这一面此前是 Undeclared】两块缓冲的读写全走裸地址：`crates/game-core/src/title/mod.rs:1376`
//! （读 `$07A0` 游标）、`:1607`（`let idx = ram[0x07A0 & 0x7FF]`）、`:1611-1618`（按 `idx`
//! 逐字节写 `$07A1+n`）、`:1642`（写 `$0781` tile 与 `$0780` 维面 `$11` = 1×1）、
//! `:1670`（回写 `$07A0` 尾指针）。字段布局与协议只活在那些字面量里 ⇒ `nes-map` 审计看不到
//! 这 87 B 的归属。
//!
//! 【本模块只做两件事】①把 asm 结构与协议升格成 typed 表示（字段名/类型/偏移，逐字段带
//! asm 锚；维面拆字节提供一个纯函数 helper）；②把该面登记进
//! [`crate::memmap::nesmap::entity_spans`]。**行为零变化**：业务层仍是唯一读写路径；
//! [`VramBufferFace::project`] 是同一批字节的编码出口，[`VramBufferFace::decode`] 是解码入口，
//! 往返互检进单测。**本模块不解析 VRAMStruct 串**（`WritePPUByte`/`EndVRAMStruct` 的游标
//! 语义仍只在移植侧一份）：把缓冲当不透明字节串保存，避免出现第二份协议实现。
//!
//! 【故意不声明的 25 B】`$0787-$079F` 是 asm 的 `SpareMem0787 ds $19`——**纯填充**，
//! 全仓无引用、也不在 `VRAMStringRAMBuffer` 与 `VRAMStructBufferIndex` 之间承担任何语义。
//! 把它登记成 `Owner` 会把「没人解释的填充」包装成「已解释的字段」，与 `nes-map` 审计
//! 「Undeclared = 无任何声明」的本来用途背离，故留在 `Undeclared` 账本里（续卡再议）。
//! `$07F0-$07FF`（16 B）同样不在任何 `.enum` 内，也不声明。
//!
//! 【声明层为什么落在 `memmap/` 而不在 `title/`】与 B1-a/B1-b 同因：`memmap/mod.rs` 的分层
//! 口径规定**地址常量与 codec 只活在本层**；本卡硬约束也只动 `memmap/`。

use crate::memmap::nesmap::{NesMapped, NesSpan, SpanKey};
use crate::memmap::storage::NesImage;

/// `VRAMStringRAM` 维面字节（`$0780`）。
pub const STRING_DIMENSIONS_ADDR: u16 = 0x0780;
/// `VRAMStringRAMBuffer` 起点（`$0781`）。
pub const STRING_BUFFER_ADDR: u16 = 0x0781;
/// `VRAMStringRAMBuffer` 长（`ds 6`）。
pub const STRING_BUFFER_LEN: u16 = 6;
/// `VRAMStringRAM` 整块长（维面 1 + 串 6 = `$0780-$0786`）。
pub const STRING_FACE_LEN: u16 = 1 + STRING_BUFFER_LEN;

/// `VRAMStructBufferIndex` 地址（`$07A0`）。
pub const STRUCT_INDEX_ADDR: u16 = 0x07A0;
/// `VRAMStructBuffer` 起点（`$07A1`）。
pub const STRUCT_BUFFER_ADDR: u16 = 0x07A1;
/// `VRAMStructBuffer` 长（`ds $4F` = 79 B，`$07A1-$07EF`）。
pub const STRUCT_BUFFER_LEN: u16 = 0x4F;

/// 维面字节的高 4 位 = 高（tile 数）。
pub const DIMENSIONS_HEIGHT_SHIFT: u8 = 4;
/// 维面字节的低 4 位掩码 = 宽（tile 数）。
pub const DIMENSIONS_WIDTH_MASK: u8 = 0x0F;

/// 把 `$0780` 维面字节拆成 `(height_tiles, width_tiles)`（asm 注释：4 MSBs / 4 LSBs）。
pub fn dimensions_tiles(dims: u8) -> (u8, u8) {
    (dims >> DIMENSIONS_HEIGHT_SHIFT, dims & DIMENSIONS_WIDTH_MASK)
}

/// `VRAMStringRAM` 两块（维面 + 串）的 typed 表示。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct VramStringRam {
    /// `$0780` `VRAMStringRAMDimensions`：4 MSB 高、4 LSB 宽（见 [`dimensions_tiles`]）。
    pub dims: u8,
    /// `$0781-$0786` `VRAMStringRAMBuffer`：待写进 PPU 的 tile id 串（6 B）。
    pub buffer: [u8; STRING_BUFFER_LEN as usize],
}

impl VramStringRam {
    /// 维面拆解（`(height_tiles, width_tiles)`）。
    pub fn tiles(&self) -> (u8, u8) {
        dimensions_tiles(self.dims)
    }
}

/// `VRAMStruct` 写队列（游标 + 串）的 typed 表示。
///
/// 串内容按 asm 的 `VRAMStructData` 协议编码（地址高/低、配置字节、数据、`$00` 终止），
/// 但本结构**不解析**它——解析器只在移植侧（`crate::title` 的 `WritePPUByte`/`EndVRAMStruct`）
/// 一份，避免第二份协议实现漂移。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VramStructQueue {
    /// `$07A0` `VRAMStructBufferIndex`：已用字节数（`#$4F` 上限；读端以 `$00` 终止）。
    pub index: u8,
    /// `$07A1-$07EF` `VRAMStructBuffer`（79 B，未用部分按原字节保留）。
    pub buffer: [u8; STRUCT_BUFFER_LEN as usize],
}

// `[u8; 79]` 没有 `Default`（std 只为 ≤32 与小尺寸数组实现），手写零值。
impl Default for VramStructQueue {
    fn default() -> Self {
        Self {
            index: 0,
            buffer: [0u8; STRUCT_BUFFER_LEN as usize],
        }
    }
}

impl VramStructQueue {
    /// 容量（`#$4F` 字节）；`WritePPUByte` 在 `X >= #$4F` 时回填段首。
    pub const CAPACITY: u16 = STRUCT_BUFFER_LEN;

    /// 当前游标位置对应的绝对地址（`$07A1 + index`）；越界返回 `None`。
    pub fn cursor_addr(&self) -> Option<u16> {
        if u16::from(self.index) >= STRUCT_BUFFER_LEN {
            return None;
        }
        Some(STRUCT_BUFFER_ADDR + u16::from(self.index))
    }
}

/// PPU 写缓冲整面（两块，共 87 B）的 typed 表示。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct VramBufferFace {
    string_ram: VramStringRam,
    struct_queue: VramStructQueue,
}

impl VramBufferFace {
    /// 地址是否落在本面两块之一（`$0780-$0786` 或 `$07A0-$07EF`）。
    pub fn contains(addr: u16) -> bool {
        let a = addr & 0x7FF;
        (STRING_DIMENSIONS_ADDR..STRING_DIMENSIONS_ADDR + STRING_FACE_LEN).contains(&a)
            || (STRUCT_INDEX_ADDR..STRUCT_INDEX_ADDR + 1 + STRUCT_BUFFER_LEN).contains(&a)
    }

    /// 从 2KB RAM 镜像面解码（地址按 `& 0x7FF` 归一）。
    pub fn decode(ram: &[u8]) -> Self {
        let dims = ram[usize::from(STRING_DIMENSIONS_ADDR & 0x7FF)];
        let sb = usize::from(STRING_BUFFER_ADDR & 0x7FF);
        let mut buffer = [0u8; STRING_BUFFER_LEN as usize];
        buffer.copy_from_slice(&ram[sb..sb + STRING_BUFFER_LEN as usize]);

        let index = ram[usize::from(STRUCT_INDEX_ADDR & 0x7FF)];
        let qb = usize::from(STRUCT_BUFFER_ADDR & 0x7FF);
        let mut qbuffer = [0u8; STRUCT_BUFFER_LEN as usize];
        qbuffer.copy_from_slice(&ram[qb..qb + STRUCT_BUFFER_LEN as usize]);

        Self {
            string_ram: VramStringRam { dims, buffer },
            struct_queue: VramStructQueue {
                index,
                buffer: qbuffer,
            },
        }
    }

    /// 编码回 2KB RAM 镜像面（只写本面两块）。
    pub fn encode(&self, ram: &mut [u8]) {
        ram[usize::from(STRING_DIMENSIONS_ADDR & 0x7FF)] = self.string_ram.dims;
        let sb = usize::from(STRING_BUFFER_ADDR & 0x7FF);
        ram[sb..sb + STRING_BUFFER_LEN as usize].copy_from_slice(&self.string_ram.buffer);

        ram[usize::from(STRUCT_INDEX_ADDR & 0x7FF)] = self.struct_queue.index;
        let qb = usize::from(STRUCT_BUFFER_ADDR & 0x7FF);
        ram[qb..qb + STRUCT_BUFFER_LEN as usize].copy_from_slice(&self.struct_queue.buffer);
    }

    /// `VRAMStringRAM` 视图。
    pub fn string_ram(&self) -> &VramStringRam {
        &self.string_ram
    }

    /// `VRAMStringRAM` 可变视图。
    pub fn string_ram_mut(&mut self) -> &mut VramStringRam {
        &mut self.string_ram
    }

    /// `VRAMStruct` 队列视图。
    pub fn struct_queue(&self) -> &VramStructQueue {
        &self.struct_queue
    }

    /// `VRAMStruct` 队列可变视图。
    pub fn struct_queue_mut(&mut self) -> &mut VramStructQueue {
        &mut self.struct_queue
    }

    /// 地址 → 字节（不在本面返回 `None`）；供将来接入 `Manager::read_ram` 分派用。
    pub fn get_byte(&self, addr: u16) -> Option<u8> {
        if !Self::contains(addr) {
            return None;
        }
        let a = addr & 0x7FF;
        if a == STRING_DIMENSIONS_ADDR {
            return Some(self.string_ram.dims);
        }
        if (STRING_BUFFER_ADDR..STRING_BUFFER_ADDR + STRING_BUFFER_LEN).contains(&a) {
            return Some(self.string_ram.buffer[usize::from(a - STRING_BUFFER_ADDR)]);
        }
        if a == STRUCT_INDEX_ADDR {
            return Some(self.struct_queue.index);
        }
        Some(self.struct_queue.buffer[usize::from(a - STRUCT_BUFFER_ADDR)])
    }

    /// 字节 → 地址（不在本面返回 `false`，调用方继续往下一域分派）。
    pub fn set_byte(&mut self, addr: u16, v: u8) -> bool {
        if !Self::contains(addr) {
            return false;
        }
        let a = addr & 0x7FF;
        if a == STRING_DIMENSIONS_ADDR {
            self.string_ram.dims = v;
            return true;
        }
        if (STRING_BUFFER_ADDR..STRING_BUFFER_ADDR + STRING_BUFFER_LEN).contains(&a) {
            self.string_ram.buffer[usize::from(a - STRING_BUFFER_ADDR)] = v;
            return true;
        }
        if a == STRUCT_INDEX_ADDR {
            self.struct_queue.index = v;
            return true;
        }
        self.struct_queue.buffer[usize::from(a - STRUCT_BUFFER_ADDR)] = v;
        true
    }
}

impl NesMapped for VramBufferFace {
    const DISCRIMINATOR: Option<u16> = None;

    /// 4 段 `Owner`：`$0780`(1) + `$0781-$0786`(6) + `$07A0`(1) + `$07A1-$07EF`(79) = 87 B。
    ///
    /// 两块之间故意留 `$0787-$079F`（`SpareMem0787`，25 B 纯填充）不声明，见模块头。
    const SPANS: &'static [(NesSpan, SpanKey)] = &[
        (NesSpan::value(STRING_DIMENSIONS_ADDR, 1), SpanKey::Owner),
        (NesSpan::value(STRING_BUFFER_ADDR, STRING_BUFFER_LEN), SpanKey::Owner),
        (NesSpan::value(STRUCT_INDEX_ADDR, 1), SpanKey::Owner),
        (NesSpan::value(STRUCT_BUFFER_ADDR, STRUCT_BUFFER_LEN), SpanKey::Owner),
    ];

    fn project(&self, out: &mut NesImage) {
        self.encode(&mut out.ram);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 两块几何必须等于 asm 的逐项地址与 `ds` 长度。
    #[test]
    fn face_geometry_matches_asm_enum() {
        assert_eq!(STRING_DIMENSIONS_ADDR, 0x0780, "`VRAMStringRAMDimensions`");
        assert_eq!(STRING_BUFFER_ADDR, 0x0781, "`VRAMStringRAMBuffer` 紧随其后");
        assert_eq!(STRING_BUFFER_LEN, 6, "`ds 6`");
        assert_eq!(STRING_FACE_LEN, 7, "`$0780-$0786` = 7 B");
        assert_eq!(STRUCT_INDEX_ADDR, 0x07A0, "`VRAMStructBufferIndex`");
        assert_eq!(STRUCT_BUFFER_ADDR, 0x07A1, "`VRAMStructBuffer` 紧随其后");
        assert_eq!(STRUCT_BUFFER_LEN, 0x4F, "`ds $4F` = 79 B");
        assert_eq!(
            STRUCT_BUFFER_ADDR + STRUCT_BUFFER_LEN,
            0x07F0,
            "`$07A1-$07EF`，末字节 +1 = `$07F0`"
        );
        assert_eq!(VramStructQueue::CAPACITY, 0x4F, "asm 注释「#$4F bytes max」");
    }

    /// 声明表必须是 4 段 `Owner`，两块内部各自连续，合计 87 B。
    #[test]
    fn spans_cover_both_blocks() {
        let spans = <VramBufferFace as NesMapped>::SPANS;
        assert_eq!(spans.len(), 4);
        let total: u16 = spans.iter().map(|(s, _)| s.len).sum();
        assert_eq!(total, 87, "7 + 80 = 87 B");
        assert_eq!(spans[0].0.start, STRING_DIMENSIONS_ADDR);
        assert_eq!(spans[0].0.start + spans[0].0.len, spans[1].0.start, "串紧跟维面");
        assert_eq!(spans[1].0.start + spans[1].0.len, STRING_DIMENSIONS_ADDR + STRING_FACE_LEN);
        assert_eq!(spans[2].0.start, STRUCT_INDEX_ADDR);
        assert_eq!(spans[2].0.start + spans[2].0.len, spans[3].0.start, "缓冲紧跟游标");
        for (_, key) in spans {
            assert!(matches!(key, SpanKey::Owner));
        }
    }

    /// 维面拆字节：4 MSB = 高、4 LSB = 宽（`$11` ⇒ 1×1，与 `title` 的写入一致）。
    #[test]
    fn dimensions_split_matches_asm_comment() {
        assert_eq!(dimensions_tiles(0x11), (1, 1), "`$11` = 高 1 × 宽 1");
        assert_eq!(dimensions_tiles(0x20), (2, 0));
        assert_eq!(dimensions_tiles(0x0F), (0, 15));
        assert_eq!(dimensions_tiles(0xFF), (15, 15));
        let s = VramStringRam {
            dims: 0x23,
            buffer: [0u8; STRING_BUFFER_LEN as usize],
        };
        assert_eq!(s.tiles(), (2, 3));
    }

    /// 任意字节面往返：`encode(decode(ram)) == ram`（含未用字节，不丢信息）。
    #[test]
    fn face_round_trip_is_byte_exact() {
        for case in 0..6u32 {
            let mut ram = [0u8; 0x800];
            let mut seed = 0x5DEE_CE66u32 ^ case;
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
            let face = VramBufferFace::decode(&ram);
            let mut out = ram;
            face.encode(&mut out);
            assert_eq!(out, ram, "case {case}：两块内字节必须逐字节往返");
        }
    }

    /// 面外字节不受影响：`encode` 只写 `$0780-$0786` 与 `$07A0-$07EF`。
    #[test]
    fn encode_touches_only_the_two_blocks() {
        let mut ram = [0x5Au8; 0x800];
        let face = VramBufferFace::decode(&ram);
        face.encode(&mut ram);
        // 夹缝 `$0787-$079F`（SpareMem0787）与 `$07F0-$07FF` 必须保持哨兵值。
        for addr in 0x0787usize..0x07A0 {
            assert_eq!(ram[addr], 0x5A, "${addr:#06X} 属于 SpareMem0787，不得被写");
        }
        for addr in 0x07F0usize..0x0800 {
            assert_eq!(ram[addr], 0x5A, "${addr:#06X} 不在任何 .enum 内，不得被写");
        }
        assert_eq!(ram[0x077F], 0x5A, "$077F（Zebetites 末）不得被写");
        assert_eq!(ram[0x0780], 0x5A, "维面应原样往返");
    }

    /// `get_byte`/`set_byte` 恰好覆盖两块、夹缝为空。
    #[test]
    fn byte_accessors_cover_exactly_the_two_blocks() {
        let mut face = VramBufferFace::default();
        for addr in STRING_DIMENSIONS_ADDR..STRING_DIMENSIONS_ADDR + STRING_FACE_LEN {
            assert!(face.set_byte(addr, (addr & 0xFF) as u8), "{addr:#06X} 应可写");
        }
        for addr in STRUCT_INDEX_ADDR..STRUCT_INDEX_ADDR + 1 + STRUCT_BUFFER_LEN {
            assert!(face.set_byte(addr, (addr & 0xFF) as u8), "{addr:#06X} 应可写");
        }
        for addr in STRING_DIMENSIONS_ADDR..STRING_DIMENSIONS_ADDR + STRING_FACE_LEN {
            assert_eq!(face.get_byte(addr), Some((addr & 0xFF) as u8), "{addr:#06X} 值回读");
        }
        for addr in STRUCT_INDEX_ADDR..STRUCT_INDEX_ADDR + 1 + STRUCT_BUFFER_LEN {
            assert_eq!(face.get_byte(addr), Some((addr & 0xFF) as u8), "{addr:#06X} 值回读");
        }
        // 夹缝与两端：不在本面。
        assert_eq!(face.get_byte(0x0787), None, "`SpareMem0787` 不声明为本面");
        assert_eq!(face.get_byte(0x079F), None);
        assert_eq!(face.get_byte(0x07F0), None, "`$07F0` 不在缓冲内");
        assert!(!face.set_byte(0x0787, 1), "夹缝写必须被拒");
        assert_eq!(face.get_byte(0x077F), None, "`$077F` 属房间精灵面，不在这");
        // 镜像归一：`$0F80` 与 `$0780` 同一格。
        assert_eq!(face.get_byte(0x0F80), face.get_byte(STRING_DIMENSIONS_ADDR));
    }

    /// `project` 只写两块字节（`NesMapped` 契约）。
    #[test]
    fn project_writes_only_this_face() {
        let ram = [0xA5u8; 0x800];
        let face = VramBufferFace::decode(&ram);
        let mut img = NesImage {
            ram: [0x3Cu8; 0x800],
            wram: [0u8; 0x2000],
        };
        face.project(&mut img);
        for addr in 0..0x800usize {
            if VramBufferFace::contains(addr as u16) {
                assert_eq!(img.ram[addr], 0xA5, "${addr:#06X} 属于本面，应被投影");
            } else {
                assert_eq!(img.ram[addr], 0x3C, "${addr:#06X} 不属于本面，不得被写");
            }
        }
    }

    /// 游标地址契约与 `title` 的 `WritePPUByte`/`EndVRAMStruct` 口径一致。
    #[test]
    fn cursor_addr_matches_business_layer_writes() {
        let mut q = VramStructQueue::default();
        for i in 0u16..STRUCT_BUFFER_LEN {
            q.index = i as u8;
            assert_eq!(q.cursor_addr(), Some(0x07A1 + i), "游标 {i}");
        }
        q.index = STRUCT_BUFFER_LEN as u8; // $4F：asm `cpx #$4F/bcc` 判满（回填段首）
        assert_eq!(q.cursor_addr(), None, "满位时不再指向缓冲内");
        // `title/mod.rs:1611-1618` 的 `e.push((0x07A1 + (x & 0x7F), ..))` 起点。
        assert_eq!(STRUCT_BUFFER_ADDR + u16::from(VramStructQueue::default().index), 0x07A1);
    }
}
