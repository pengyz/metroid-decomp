//! game-data —— 四级数据链解析（task-17 v1，schema 见 docs/spec/game-data-schema.md）。
//!
//! 解析层读 ROM/prg 原字节（memmap 域：bank NN 窗口 $8000 / bank7 固定 $C000）；
//! 纯函数 `&[u8] → 结构`。命名纪律（AGENTS D5）：本 crate 一律 C 级候选名——
//! 字段名用解构名/`_todo_` 前缀，语义自证后走 data/naming 台账，不在此处伪语义命名。
//!
//! 字节级复现命令（判据）：
//!   cargo test -p game-data
//!   python3 - <<'EOF'
//!   # L1 指针表首词 == m1disasm Room00 注址 0xA441（对拍素材见 findings）
//!   rom = open('rom/Metroid (USA).nes','rb').read(); prg = rom[0x10:0x20010]
//!   b1 = prg[0x4000:0x8000]
//!   w = b1[0xA314-0x8000] | (b1[0xA315-0x8000] << 8)
//!   print(hex(w))  # 期望 0xa441
//!   EOF

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoomObject {
    pub pos: u8,        // yyyyxxxx（tile 坐标，C 级拆分待自证）
    pub struct_idx: u8, // StructPtrTbl 索引
    pub attr: u8,       // 属性表字节
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoomDef {
    pub base_attr: u8,
    pub objects: Vec<RoomObject>,
    pub tail: Vec<u8>, // 敌/门变长段（$FF 终；_todo_ 语义）
}

pub const ROOM_OBJ_END: u8 = 0xFD;
pub const ROOM_END: u8 = 0xFF;

/// 解析房间定义（objects 段已证；tail 原样保留）。
/// 返回 (RoomDef, 消耗字节数)。
pub fn parse_room(data: &[u8]) -> Option<(RoomDef, usize)> {
    let mut it = data.iter().enumerate();
    let base_attr = *it.next()?.1;
    let mut objects = Vec::new();
    let mut i = 1usize;
    loop {
        let b = *data.get(i)?;
        if b == ROOM_OBJ_END {
            i += 1;
            break;
        }
        if b == 0xFE {
            // 空占位三字节（m1disasm 注释：$FE=占位符，整段跳过）
            i += 3;
            continue;
        }
        let pos = b;
        let struct_idx = *data.get(i + 1)?;
        let attr = *data.get(i + 2)?;
        objects.push(RoomObject { pos, struct_idx, attr });
        i += 3;
    }
    let mut tail = Vec::new();
    loop {
        let b = *data.get(i)?;
        tail.push(b);
        i += 1;
        if b == ROOM_END {
            break;
        }
    }
    Some((RoomDef { base_attr, objects, tail }, i))
}

/// L3 结构定义：行=1B 行宽 + nB 元块索引；$FF 终。
/// metatile 索引 ×4 = MetatileDefs 下标（m1disasm 注释源）。
/// 返回 (行向量, 消耗字节数)。
pub fn parse_structure(data: &[u8]) -> Option<(Vec<Vec<u8>>, usize)> {
    let mut rows = Vec::new();
    let mut i = 0usize;
    loop {
        let n = *data.get(i)?;
        if n == 0xFF {
            i += 1;
            break;
        }
        let n = n as usize;
        let row = data.get(i + 1..i + 1 + n)?.to_vec();
        rows.push(row);
        i += 1 + n;
    }
    Some((rows, i))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MetatileDef {
    pub lr: u8,
    pub ll: u8,
    pub ur: u8,
    pub ul: u8,
}

pub fn parse_metatile(data: &[u8]) -> Option<(MetatileDef, usize)> {
    Some((MetatileDef { lr: data[0], ll: data[1], ur: data[2], ul: data[3] }, 4))
}

/// L1 指针表：n 个 word（小端）。
pub fn parse_ptr_table(data: &[u8], n: usize) -> Option<(Vec<u16>, usize)> {
    let mut v = Vec::with_capacity(n);
    for k in 0..n {
        let o = k * 2;
        v.push(u16::from_le_bytes([*data.get(o)?, *data.get(o + 1)?]));
    }
    Some((v, n * 2))
}


/// VRAMStruct 记录族（m1disasm macros.asm 锚证，A 级）：
/// - `VRAMStructData`（macros.asm:32-56）：`[hi][lo]` PPU 大端址 + `[len]`(1..=256) + data[len]；
/// - `VRAMStructDataRepeat`（macros.asm:58-70）：`[hi][lo]` + `[reps|$40]` + 1B 值；
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VramRecord {
    Data { ppu_addr: u16, data: Vec<u8> },
    Repeat { ppu_addr: u16, reps: u8, value: u8 },
}

pub const VRAM_STRUCT_END: u8 = 0x00;

/// 解析整条 VRAMStruct（记录序列至 $00 终）。返回 (记录表, 消耗字节数)。
pub fn parse_vram_struct(data: &[u8]) -> Option<(Vec<VramRecord>, usize)> {
    let mut recs = Vec::new();
    let mut i = 0usize;
    loop {
        let b = *data.get(i)?;
        if b == VRAM_STRUCT_END {
            return Some((recs, i + 1));
        }
        let hi = b;
        let lo = *data.get(i + 1)?;
        let flag = *data.get(i + 2)?;
        let ppu_addr = ((hi as u16) << 8) | lo as u16;
        if flag & 0x40 != 0 {
            let reps = flag & 0xBF;
            let value = *data.get(i + 3)?;
            recs.push(VramRecord::Repeat { ppu_addr, reps, value });
            i += 4;
        } else {
            let n = flag as usize;
            if n == 0 || n > 256 {
                return None;
            }
            let payload = data.get(i + 3..i + 3 + n)?.to_vec();
            recs.push(VramRecord::Data { ppu_addr, data: payload });
            i += 3 + n;
        }
    }
}

#[cfg(test)]
mod gfx_tests {
    use super::*;

    // fixture=ROM bank7:$C6E0 首两项（asm 互核：GFX_Samus 06:8000 dest $0000 /
    // GFX_EndingSprites，prg7_engine.asm:1669-1673 注释）
    #[test]
    fn gfx_info_table_records() {
        let raw = [0x06, 0x00, 0x80, 0x00, 0x00, 0xA0, 0x09,
                   0x04, 0x60, 0x8D, 0x00, 0x00, 0x20, 0x05];
        let (t, n) = parse_gfx_info(&raw, 2).unwrap();
        assert_eq!(n, raw.len());
        assert_eq!(t[0], GfxInfo { bank: 6, src: 0x8000, dest: 0x0000, size: 0x09A0 });
        assert_eq!(t[1], GfxInfo { bank: 4, src: 0x8D60, dest: 0x0000, size: 0x0520 });
    }

    #[test]
    fn tile_chr_offset_2bpp() {
        assert_eq!(tile_chr_offset(0), 0);
        assert_eq!(tile_chr_offset(0x02), 32);
        assert_eq!(tile_chr_offset(0xFF), 4080); // 256 tiles × 16B = 4KB 面
    }
}

#[cfg(test)]
mod enemy_tests {
    use super::*;

    // fixture 抄自 enemy_sprite_data.asm:34-41（EnAnim_11/EnAnim_Waver0_L 双链）
    #[test]
    fn enemy_anim_chain() {
        let raw = [0x0B, 0x0C, 0xFF, 0x0D, 0xFF]; // Waver1_L, Ripper_L?… 按 id 序 + FrameEnd
        let (a, n) = parse_enemy_anim(&raw, 0x9D7B).unwrap();
        assert_eq!(n, 3); // 首链至首个 $FF 消耗 3B（其余为下链——共享尾模型见 inspect 互核）
        assert_eq!(a, EnemyAnim { addr: 0x9D7B, frames: vec![0x0B, 0x0C] });
    }

    #[test]
    fn enemy_anim_seq_two_chains() {
        let raw = [0x0B, 0x0C, 0xFF, 0x0D, 0xFF];
        let (chains, n) = parse_enemy_anim_seq(&raw, 0x9D7B, 2).unwrap();
        assert_eq!(n, raw.len());
        assert_eq!(chains.len(), 2);
        assert_eq!(chains[1], EnemyAnim { addr: 0x9D7E, frames: vec![0x0D] });
    }
}

#[cfg(test)]
mod item_tests {
    use super::*;

    // fixture 抄自 ROM bank1:$A3D6 起（@y02 章+@y03 章，末章 word 修为 $FFFF 闭合）；
    // asm 标签互核：@@x03=$A3D9/@@x0F=$A3DE/@y03=$A3E4/@@x18=$A3E7/@@x1B=$A3ED
    #[test]
    fn spec_items_first_chapter_walk() {
        let raw = [0x02, 0xE4, 0xA3,
                   0x03, 0x05, 0x04, 0x03, 0x00,
                   0x0F, 0xFF, 0x02, 0x05, 0x37, 0x00,
                   0x03, 0xFF, 0xFF,
                   0x18, 0x06, 0x02, 0x09, 0x67, 0x00,
                   0x1B, 0xFF, 0x02, 0x08, 0x87, 0x00];
        let (items, n) = parse_spec_items(&raw, 0xA3D6).unwrap();
        assert_eq!(n, raw.len());
        assert_eq!(items, vec![
            SpecialItem { addr: 0xA3D9, y: 0x02, x: 0x03, item_type: 0x04, data: vec![0x03] },
            SpecialItem { addr: 0xA3DE, y: 0x02, x: 0x0F, item_type: 0x02, data: vec![0x05, 0x37] },
            SpecialItem { addr: 0xA3E7, y: 0x03, x: 0x18, item_type: 0x02, data: vec![0x09, 0x67] },
            SpecialItem { addr: 0xA3ED, y: 0x03, x: 0x1B, item_type: 0x02, data: vec![0x08, 0x87] },
        ]);
    }

    #[test]
    fn spec_items_offset_formula() {
        // 寻径公式（实证）：next_entry = entry_start + off1
        // @@x03@$A3D9 off1=05 → $A3DE=@@x0F（asm 标签互核）
        let raw = [0x02, 0xFF, 0xFF, 0x03, 0x05, 0x04, 0x03, 0x00,
                   0x0F, 0xFF, 0x02, 0x05, 0x37, 0x00];
        let (items, _) = parse_spec_items(&raw, 0xA3D6).unwrap();
        assert_eq!(items.len(), 2);
    }
}

#[cfg(test)]
mod palette_tests {
    use super::*;

    // fixtures 抄自 palettes.asm（m1disasm SRC/brinstar/palettes.asm 注址互证）
    #[test]
    fn palette01_power_suit_record() {
        // Palette01_{AREA} ; 01:A295 → VRAMStructData $3F12, $19,$27 / VRAMStructEnd
        let raw = [0x3F, 0x12, 0x02, 0x19, 0x27, 0x00];
        let (recs, n) = parse_vram_struct(&raw).unwrap();
        assert_eq!(n, raw.len());
        assert_eq!(recs, vec![VramRecord::Data { ppu_addr: 0x3F12, data: vec![0x19, 0x27] }]);
    }

    #[test]
    fn palette00_default_room_32b() {
        let mut raw = vec![0x3F, 0x00, 32];
        raw.extend([0x0F, 0x22, 0x12, 0x1C, 0x0F, 0x22, 0x12, 0x1C,
                    0x0F, 0x27, 0x11, 0x07, 0x0F, 0x22, 0x12, 0x1C,
                    0x0F, 0x16, 0x19, 0x27, 0x0F, 0x12, 0x30, 0x21,
                    0x0F, 0x27, 0x2A, 0x3C, 0x0F, 0x15, 0x21, 0x38]);
        raw.push(0x00);
        let (recs, n) = parse_vram_struct(&raw).unwrap();
        assert_eq!(n, raw.len());
        assert_eq!(recs.len(), 1);
        match &recs[0] {
            VramRecord::Data { ppu_addr, data } => {
                assert_eq!(*ppu_addr, 0x3F00);
                assert_eq!(data.len(), 32);
            }
            _ => panic!("应为 Data 记录"),
        }
    }

    #[test]
    fn repeat_record_and_multi_record_struct() {
        // VRAMStructDataRepeat $3F00,4,$0F 后接 Data 记录再 $00（macros.asm:58-70）
        let raw = [0x3F, 0x00, 0x44, 0x0F, 0x3F, 0x04, 0x02, 0x19, 0x27, 0x00];
        let (recs, n) = parse_vram_struct(&raw).unwrap();
        assert_eq!(n, raw.len());
        assert_eq!(recs, vec![
            VramRecord::Repeat { ppu_addr: 0x3F00, reps: 4, value: 0x0F },
            VramRecord::Data { ppu_addr: 0x3F04, data: vec![0x19, 0x27] },
        ]);
    }

    #[test]
    fn missing_end_fails() {
        assert!(parse_vram_struct(&[0x3F, 0x12, 0x02, 0x19, 0x27]).is_none());
    }
}

/// 特殊物品链（m1disasm SRC/brinstar/items.asm:20-43 注释源，A 级锚+实证公式）：
/// 章头（长条目）=`[Y][word 下章址]` + 短条目体；条目=`[X][off1][type][data..$00]`；
/// **寻径公式（ROM 字节实证）**：下一短条目 = 本条目首址 + off1（asm 注「offset-1」措辞
/// 与字节不符，以互核为准——结构互核注）；off1=$FF=章终；下章址=$FFFF=表终。
/// type 语义表（items.asm:45-54 注释，C 级候选名）：$01=Squeept $02=Power up
/// $03=Mellows/Memus/Melias $04=Elevator $05=MB cannon $06=Mother brain
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpecialItem {
    pub addr: u16, // 条目首址（asm 标签互核坐标）
    pub y: u8,
    pub x: u8,
    pub item_type: u8,
    pub data: Vec<u8>,
}

pub const ITEM_CHAPTER_END: u8 = 0xFF;
pub const ITEM_TABLE_END: u16 = 0xFFFF;

/// 解析特殊物品表（含章链寻径）。返回 (条目表, 消耗字节数)。
pub fn parse_spec_items(data: &[u8], base: u16) -> Option<(Vec<SpecialItem>, usize)> {
    let mut items = Vec::new();
    let mut pos = 0usize;
    let mut used = 0usize;
    loop {
        let y = *data.get(pos)?;
        let next = u16::from_le_bytes([*data.get(pos + 1)?, *data.get(pos + 2)?]);
        pos += 3;
        loop {
            let entry = pos;
            let x = *data.get(pos)?;
            let off1 = *data.get(pos + 1)?;
            let item_type = *data.get(pos + 2)?;
            pos += 3;
            let mut payload = Vec::new();
            loop {
                let b = *data.get(pos)?;
                pos += 1;
                if b == 0 {
                    break;
                }
                payload.push(b);
            }
            items.push(SpecialItem {
                addr: base.wrapping_add(entry as u16), y, x, item_type, data: payload,
            });
            if off1 == ITEM_CHAPTER_END {
                break;
            }
            pos = entry + off1 as usize; // 实证寻径公式（结构互核注）
        }
        used = pos;
        if next == ITEM_TABLE_END {
            break;
        }
        let next_off = (next as usize).checked_sub(base as usize)?;
        if next_off <= 0 || next_off >= data.len() {
            return None;
        }
        pos = next_off;
    }
    Some((items, used))
}

/// 敌人动画/帧链（m1disasm SRC/brinstar/enemy_sprite_data.asm 锚+实证）：
/// - EnAnim=`[frame_id...] $FF 终`（_id_FrameEnd=$FF，constants.asm:116）；
///   **共享尾结构（结构互核实证）**：标签=任意入口，链可顺流进下一标签无 FF（如
///   EnAnim_11@$9D7B 流入 EnAnim_Waver0_L@$9D7D）——非断链序列，逐标签入口验证为准；
///   frame_id=EnFramePtrTable1 表索引（PtrTableEntry 自动编号，macros.asm:75-86）；
/// - EnFramePtrTable1=148 词（asm 计数）；帧体（EnFrame）线格式=`[place|pal][?][?]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnemyAnim {
    pub addr: u16,        // 链首址（asm 标签互核坐标）
    pub frames: Vec<u8>,  // 帧 id 序列（$FF 已剥）
}

pub const ANIM_FRAME_END: u8 = 0xFF;

/// 解析单条 EnAnim 链。返回 (链, 消耗字节数)。
pub fn parse_enemy_anim(data: &[u8], addr: u16) -> Option<(EnemyAnim, usize)> {
    let mut frames = Vec::new();
    let mut i = 0usize;
    loop {
        let b = *data.get(i)?;
        i += 1;
        if b == ANIM_FRAME_END {
            return Some((EnemyAnim { addr, frames }, i));
        }
        frames.push(b);
    }
}

/// 连续 EnAnim 链序列解析（链首=addr 增长对齐；用于表后动画区批量走链）。
/// 返回 (链表, 消耗字节数)。前提：链在内存中连续排布（asm 标签序列实证）。
pub fn parse_enemy_anim_seq(data: &[u8], addr: u16, count: usize) -> Option<(Vec<EnemyAnim>, usize)> {
    let mut out = Vec::new();
    let mut off = 0usize;
    for _ in 0..count {
        let (a, n) = parse_enemy_anim(&data[off..], addr.wrapping_add(off as u16))?;
        out.push(a);
        off += n;
    }
    Some((out, off))
}

/// GFX 转运表（prg7_engine.asm:1665-1669 线格式注 + macros.asm:101-107 宏体，A 级锚）：
/// 7B/项=`[bank][src 16le][dest 16le][len 16le]`——src=PRG 内 gfx 起址（bank:src），
/// dest=PPU 目标（$0000=精灵面/$1000+=背景面），len=字节数。语义名（Samus/Brinstar
/// rooms 等注释）C 级不入字段。L4→CHR 面：元块图块索引（0-255）×16B=NES 2bpp
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GfxInfo {
    pub bank: u8,
    pub src: u16,
    pub dest: u16,
    pub size: u16,
}

/// 解析 GFX 转运表（n 项×7B）。返回 (表, 消耗字节数)。
pub fn parse_gfx_info(data: &[u8], n: usize) -> Option<(Vec<GfxInfo>, usize)> {
    let mut v = Vec::with_capacity(n);
    for k in 0..n {
        let o = k * 7;
        let bank = *data.get(o)?;
        let src = u16::from_le_bytes([*data.get(o + 1)?, *data.get(o + 2)?]);
        let dest = u16::from_le_bytes([*data.get(o + 3)?, *data.get(o + 4)?]);
        let size = u16::from_le_bytes([*data.get(o + 5)?, *data.get(o + 6)?]);
        v.push(GfxInfo { bank, src, dest, size });
    }
    Some((v, n * 7))
}

/// L4→CHR：图块索引（0-255）→ NES 2bpp 图样字节偏移（16B/tile，A 级平台语义）。
pub fn tile_chr_offset(tile: u8) -> usize {
    tile as usize * 16
}

/// brinstar（bank1）锚点——m1disasm `SRC/brinstar/*.asm` 注址（A 级线索，等价性背书）。
pub mod brinstar {
    /// `room_ptrs.asm` RoomPtrTable_{AREA} ; 01:A314
    pub const ROOM_PTRS: (u8, u16) = (1, 0xA314);
    /// `rooms.asm` Room00_{AREA} ; 01:A441
    pub const ROOM00: (u8, u16) = (1, 0xA441);
    /// `structure_ptrs.asm` StructPtrTable_{AREA} ; 01:A372
    pub const STRUCT_PTRS: (u8, u16) = (1, 0xA372);
    /// `structures.asm` Structure00_{AREA} ; 01:AC84
    pub const STRUCTURE00: (u8, u16) = (1, 0xAC84);
    /// `metatiles.asm` MetatileDefs_{AREA} ; 01:AEF0
    pub const METATILE_DEFS: (u8, u16) = (1, 0xAEF0);
    /// `prg1_brinstar.asm:55` PalettePtrTable ; 01:9560
    pub const PALETTE_PTRS: (u8, u16) = (1, 0x9560);
    /// `palettes.asm` Palette00_{AREA} ; 01:A271（PtrTableEntryArea 注址互证 ($A271)）
    pub const PALETTE00: (u8, u16) = (1, 0xA271);
    /// `palettes.asm` Palette01_{AREA} ; 01:A295
    pub const PALETTE01: (u8, u16) = (1, 0xA295);
    /// 元块总数=65（metatiles.asm 260 数据项/4，asm 数得）
    pub const METATILE_COUNT: usize = 65;
    /// 调色板表项数=28（prg1_brinstar.asm PtrTableEntryArea 计数）
    pub const PALETTE_COUNT: usize = 28;
}

#[cfg(test)]
mod tests {
    use super::*;

    // fixtures 字节抄自 m1disasm SRC/brinstar/rooms.asm:32-41（Room00，双工具交叉：
    // 解析器输入=注释源 .byte 序列，断言=注释源结构语义）
    const ROOM00: &[u8] = &[
        0x02, 0x40, 0x01, 0x02, 0x48, 0x01, 0x02, 0x50, 0x03, 0x02,
        0x5F, 0x03, 0x02, 0xFD, 0x02, 0xA1, 0x02, 0xB1, 0xFF,
    ];

    #[test]
    fn room00_objects_and_tail() {
        let (r, n) = parse_room(ROOM00).unwrap();
        assert_eq!(n, ROOM00.len());
        assert_eq!(r.base_attr, 0x02);
        assert_eq!(r.objects.len(), 4);
        assert_eq!(r.objects[0], RoomObject { pos: 0x40, struct_idx: 0x01, attr: 0x02 });
        assert_eq!(r.objects[3], RoomObject { pos: 0x5F, struct_idx: 0x03, attr: 0x02 });
        // 敌/门段（C 级语义）：Blue door ×2 + $FF 终，原样保留
        assert_eq!(r.tail, vec![0x02, 0xA1, 0x02, 0xB1, 0xFF]);
    }

    // fixture 抄自 structures.asm Structure00 首两行（01:AC84）
    #[test]
    fn structure_rows() {
        let s: &[u8] = &[0x08, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01,
                         0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xFF];
        let (rows, n) = parse_structure(s).unwrap();
        assert_eq!(n, s.len());
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].len(), 8);
        assert_eq!(rows[1], vec![0u8; 8]);
    }

    // fixture 抄自 metatiles.asm MetatileDefs 前两项（01:AEF0）
    #[test]
    fn metatile_quad_order() {
        let (m, _) = parse_metatile(&[0xF1, 0xF1, 0xF1, 0xF1]).unwrap();
        assert_eq!(m, MetatileDef { lr: 0xF1, ll: 0xF1, ur: 0xF1, ul: 0xF1 });
        let (m2, _) = parse_metatile(&[0xFF, 0xFF, 0xF0, 0xF0]).unwrap();
        assert_eq!(m2, MetatileDef { lr: 0xFF, ll: 0xFF, ur: 0xF0, ul: 0xF0 });
    }

    // L1 对拍（m1disasm Room00 注址 0xA441；指针表 fixture=该词）
    #[test]
    fn room_ptr_table_matches_m1_label() {
        let (v, n) = parse_ptr_table(&[0x41, 0xA4], 1).unwrap();
        assert_eq!(n, 2);
        assert_eq!(v[0], brinstar::ROOM00.1);
    }
}

pub mod rom_tables; // 分界线修（rom_tables（数据表）→game-data（资产解析））
pub mod world; // 数据内容在 game-data（架构约束）——世界数据（world_map+room_data）
pub mod samus; // 数据内容在 game-data（架构约束）——Samus 数据
pub mod enemy; // 数据内容在 game-data（架构约束）——敌人数据
pub mod sound; // 数据内容在 game-data（架构约束）——音频数据
pub mod title; // 数据内容在 game-data（架构约束）——标题数据
pub mod render; // 数据内容在 game-data（架构约束）——渲染数据
