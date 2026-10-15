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

/// 房间结构段：yyyyxxxx 位置 + StructPtrTbl 索引 + 属性字节（m1disasm 注释源）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoomObject {
    pub pos: u8,        // yyyyxxxx（tile 坐标，C 级拆分待自证）
    pub struct_idx: u8, // StructPtrTbl 索引
    pub attr: u8,       // 属性表字节
}

/// L2 房间定义（敌/门段变长语义 C 级——原样保留 tail）。
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

/// L4 元块：4B（LR,LL,UR,UL 图块索引）。
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
