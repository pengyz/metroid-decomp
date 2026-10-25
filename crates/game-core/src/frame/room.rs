//! 房间绘制引擎（DrawRoom $EAAA 族——r25 端口；对拍驱动）。
//!
//! asm 锚（prg7_engine.asm）：DrawRoom $EAAA（对象环）/DrawObject $EA60（3B 条目：
//! pos(%yyyyxxxx)/struct 索引/属性）/DrawStruct $EF8C（行流至 $FF）/DrawStructRow
//! $EF13（行头：低半字节=元 tile 计（0=16）/高半字节=x 偏移）/DrawMetatile $EF2B（4B
//! 组→TilePosTable 序 $21/$20/$01/$00）/UpdateAttrib $EF9E（属性位对：x=quadrant）。
//! 数据面=game-data::area::graphics（三件套烘焙）。

use game_data::area::graphics as gfx;

/// TilePosTable（$EF9A——元 tile 4 tile 偏移：右下/左下/右上/左上）。
const TILE_POS: [u16; 4] = [0x21, 0x20, 0x01, 0x00];

/// 属性位掩码（公式形：清 x 位对——asm AttribMaskTable 语义）。
fn attrib_mask(x: u8) -> u8 {
    !(0x03u8 << (x * 2))
}

/// DrawRoom（$EAAA）——房对象环→RoomRAM 缓冲（base=RoomRAMA $6000）。
/// 返回 false=走敌门环/属性表尾（未端口面——调用方 fail-loud）。
pub fn draw_room(mem: &mut crate::frame::FrameMemory, room: u8) -> bool {
    let Some(stream) = gfx::ROOM_STREAMS.get(room as usize) else {
        return false;
    };
    let base = 0x6000u16; // RoomRAMA（SelectRoomRAM 随件——起始房缓冲 A）
    mem.write_ram(0x0037, 0x00); // RoomRAMPtr
    mem.write_ram(0x0038, 0x60);
    let room_palette = stream.first().copied().unwrap_or(0); // RoomPalette（首字节随考证）
    let mut i = 0usize;
    loop {
        let Some(&b) = stream.get(i) else { return false };
        match b {
            0xFF => {
                // EndOfRoom（$EAF4）：RoomNumber←$F0（属性表尾命令——循环续面）
                mem.write_ram(0x005A, 0xF0);
                return true;
            }
            0xFE => {
                mem.write_ram(0x005A, 0xFE); // 空对象占位（LEABC）
                i += 1;
            }
            0xFD => return false, // EndOfObjs→敌门环（$EAD4——未端口）
            _ => {
                // DrawObject：pos/struct 索引/属性 3B
                let pos = b;
                let struct_idx = stream.get(i + 1).copied().unwrap_or(0);
                let obj_palette = stream.get(i + 2).copied().unwrap_or(0);
                let y = (pos >> 4) as u16;
                let x = (pos & 0x0F) as u16;
                // RoomRAMWorkPtr = base + y*0x40 + x*2（y 单元=2 行/x 单元=2 tile）
                let work = base + y * 0x40 + x * 2;
                draw_struct(mem, work, struct_idx, obj_palette, room_palette);
                i += 3;
            }
        }
    }
}

/// DrawStruct（$EF8C）——结构行流（行头+元 tile 流至 $FF）。
fn draw_struct(
    mem: &mut crate::frame::FrameMemory,
    work: u16,
    struct_idx: u8,
    obj_palette: u8,
    room_palette: u8,
) {
    let Some(&s) = gfx::STRUCT_PTRS.get(struct_idx as usize) else { return };
    let _ = s;
    let Some(data) = gfx::STRUCTURES.get(struct_idx as usize) else { return };
    let mut idx = 0usize;
    let mut work = work;
    loop {
        let Some(&head) = data.get(idx) else { return };
        if head == 0xFF {
            return; // DrawStruct @RTS
        }
        idx += 1;
        // DrawStructRow：行头（低半字节=元 tile 计（0=16）/高半字节=x 偏移）
        let count = match head & 0x0F {
            0 => 16u8,
            n => n,
        };
        let x_off = ((head >> 4) as u16) * 2;
        let mut row_ptr = work + x_off;
        let mut counter = count;
        loop {
            // DrawMetatile
            let Some(&mt) = data.get(idx) else { return };
            idx += 1;
            let mt_base = (mt as usize) * 4;
            for k in 0..4u16 {
                let tile = gfx::METATILE_DEFS
                    .get(mt_base + k as usize)
                    .copied()
                    .unwrap_or(0);
                let at = row_ptr.wrapping_add(TILE_POS[k as usize]);
                mem.write_wram(at, tile);
            }
            update_attrib(mem, row_ptr, obj_palette, room_palette);
            row_ptr = row_ptr.wrapping_add(2);
            // 行满（&$1F==0）⇒提前换行（AdvanceRow）；否则计数递减
            if row_ptr & 0x1F == 0 {
                // struct index 前跳剩余（asm：struct index+counter-1→AdvanceRow）
                idx += counter as usize - 1;
                break;
            }
            counter -= 1;
            if counter == 0 {
                break;
            }
        }
        // AdvanceRow（$EF78）：work+=$40（下一行）
        work = work.wrapping_add(0x40);
    }
}

/// UpdateAttrib（$EF9E）——属性位对写（x=quadrant 0..3；属性字节=RoomRAM $63C0 面）。
fn update_attrib(
    mem: &mut crate::frame::FrameMemory,
    ptr: u16,
    obj_palette: u8,
    room_palette: u8,
) {
    if obj_palette == room_palette {
        return; // 同室调色⇒无改（asm @RTS）
    }
    // 属性字节址（asm 算式）：(ptr>>2) 低 3 位 | (ptr>>4)&$38 | $C0，高字节 $63|(ptr_hi&4)
    let rel = ptr.wrapping_sub(0x6000) & 0x0FFF; // RoomRAM 相对
    let a0 = rel >> 2;
    let off = (a0 & 0x07) | ((a0 >> 2) & 0x38) | 0xC0;
    let hi = 0x63u16 | ((ptr >> 8) & 0x04);
    let addr = (hi << 8) | off;
    // 位对（x）：bit6/2 判定（asm bit/and 形）
    let mut x = 0u8;
    if ptr & 0x40 != 0 {
        x += 2;
    }
    if ptr & 0x02 != 0 {
        x += 1;
    }
    let cur = mem.read_wram(addr);
    let cleared = cur & attrib_mask(x);
    let new = cleared | ((obj_palette & 0x03) << (x * 2));
    mem.write_wram(addr, new);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame::FrameMemory;

    /// 起始房（房 9）绘制形：RoomRAM 有 tile 落（金侧 f520 房景对形的底座）；
    /// 敌门环=下一缺环（$EAD4——返回 false 面）。
    #[test]
    fn draw_start_room_writes_tiles() {
        let mut m = FrameMemory::default();
        assert!(!draw_room(&mut m, 9), "房 9 达敌门环（$EAD4——未端口面）");
        let tiles: Vec<u8> = (0..0x3C0u16).map(|i| m.read_wram(0x6000 + i)).collect();
        assert!(tiles.iter().any(|&t| t != 0), "RoomRAM 有 tile 落");
    }

    /// 属性位公式形（掩码对——x 位对清+置）。
    #[test]
    fn attrib_mask_formula() {
        assert_eq!(attrib_mask(0), 0xFC);
        assert_eq!(attrib_mask(1), 0xF3);
        assert_eq!(attrib_mask(2), 0xCF);
        assert_eq!(attrib_mask(3), 0x3F);
    }
}
