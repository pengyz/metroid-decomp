//! HUD：状态栏 40 字节表拷贝、数字精灵写入、能量罐/导弹/逃生计时。
//!
//! 由 `frame/mainloop.rs` 原样搬出（纯搬移）。asm 根：`DisplayBar` 07:E0B2 邻域、
//! `HexToDec ; 07:E198`、`SPRWriteDigit`。

use crate::frame::tick::*;
use crate::frame::Manager;
use crate::ram_map;

pub(crate) const DATA_DISPLAY_TBL: [u8; 40] = [
    0x21, 0xA0, 0x01, 0x30, 0x21, 0xA0, 0x01, 0x38,
    0x2B, 0xFF, 0x01, 0x28, 0x2B, 0xFF, 0x01, 0x30,
    0x2B, 0xFF, 0x01, 0x38, 0x2B, 0x5E, 0x00, 0x18,
    0x2B, 0x5F, 0x00, 0x20, 0x21, 0x76, 0x01, 0x18,
    0x21, 0x7F, 0x01, 0x20, 0x21, 0x3A, 0x00, 0x28,
];

/// SPRWriteDigit（E173）：tile=A|$A0（数字图案区），X+4（Xplus4 E193）。
pub(crate) fn spr_write_digit(mem: &mut crate::frame::Manager, x: u8, a: u8) {
    mem.write_ram(0x0200u16 + x as u16 + 1, a | 0xA0);
}

/// HexToDec（E198+DivideByRepeatedSubtraction E1AD）：百/十/个位。
pub(crate) fn hex_to_dec(n: u8) -> (u8, u8, u8) {
    let hundreds = n / 100;
    let rem = n % 100;
    let tens = rem / 10;
    (hundreds as u8, tens as u8, rem % 10)
}

pub(crate) fn uw_display_bar(mem: &mut crate::frame::Manager) {
    // ① 拷表（E0C7 循环：40B→OAM[X..X+40)，$5B←X+40，pla 恢复原 X）
    let x0 = mem.sprite_page_pos();
    let mut x = x0;
    for i in 0..40usize {
        mem.write_ram(0x0200u16 + x as u16, DATA_DISPLAY_TBL[i]);
        x = x.wrapping_add(1);
    }
    mem.set_sprite_page_pos(x);
    x = x0;
    // ② 健康 2 位（E0E0 区：lda Health+1/and #$0F → SPRWriteDigit；lda Health/Adiv16 → ）
    spr_write_digit(mem, x, mem.read_ram(0x0107) & 0x0F);
    x = x.wrapping_add(4);
    spr_write_digit(mem, x, mem.read_ram(0x0106) >> 4);
    x = x.wrapping_add(4);
    // ③ 分派（E0EA：ldy $010B/iny/bne LE11C——≠$FF ⇒ 逃生）
    if mem.read_ram(0x010B) != 0xFF {
        // LE11C：逃生计时 3 位 + TI/ME
        spr_write_digit(mem, x, mem.read_ram(0x010B) >> 4);
        x = x.wrapping_add(4);
        spr_write_digit(mem, x, mem.read_ram(0x010B) & 0x0F);
        x = x.wrapping_add(4);
        spr_write_digit(mem, x, mem.read_ram(0x010A) >> 4);
        x = x.wrapping_add(4);
        let ti = 0x0200u16 + x as u16;
        mem.write_ram(ti, 0x58); // "TI"
        mem.write_ram(ti + 2, mem.read_ram(ti + 2).wrapping_add(1));
        if x < 0xFC {
            let me = 0x0200u16 + x.wrapping_add(4) as u16;
            mem.write_ram(me, 0x59); // "ME"
            mem.write_ram(me + 2, mem.read_ram(me + 2).wrapping_add(1));
        }
    } else if mem.read_ram(0x687A) != 0 {
        // 导弹 3 位（HexToDec E198）
        let (h, t, o) = hex_to_dec(mem.read_ram(0x6879));
        spr_write_digit(mem, x, h);
        x = x.wrapping_add(4);
        spr_write_digit(mem, x, t);
        x = x.wrapping_add(4);
        spr_write_digit(mem, x, o);
        x = x.wrapping_add(4);
    } else {
        // LE10A：无导弹 ⇒ 擦图标两半 tileID（X=原页+8 恰指表槽 5/6）
        if x < 0xF4 {
            mem.write_ram(0x0200u16 + x as u16 + 0x0D, 0xFF);
        }
        if x < 0xF0 {
            mem.write_ram(0x0200u16 + x as u16 + 0x11, 0xFF);
        }
        // bne LE14A
    }
    // ④ LE14A 能量罐
    let mut x = mem.sprite_page_pos(); // 重读 $5B（追加后位置）
    let tanks = mem.read_ram(0x6877);
    if tanks == 0 {
        return; // RTS_E172
    }
    mem.write_ram(0x0003, tanks);
    mem.write_ram(0x0000, 0x40); // 最右罐 X 坐标
    let mut tile: u8 = 0x6F; // 满罐
    let full = mem.read_ram(0x0107) >> 4;
    mem.write_ram(0x0001, full);
    if full == 0 {
        tile = tile.wrapping_sub(1); // dey ⇒ 空罐 $6E
    }
    loop {
        // AddOneTank(E17B)
        mem.write_ram(0x0200u16 + x as u16, 0x17); // y
        mem.write_ram(0x0200u16 + x as u16 + 1, tile); // tile
        mem.write_ram(0x0200u16 + x as u16 + 2, 0x01); // attr
        mem.write_ram(0x0200u16 + x as u16 + 3, mem.read_ram(0x0000)); // x 坐标
        mem.write_ram(0x0000, mem.read_ram(0x0000).wrapping_sub(0x0A));
        x = x.wrapping_add(4); // Xplus4
        mem.write_ram(0x0001, mem.read_ram(0x0001).wrapping_sub(1));
        if mem.read_ram(0x0001) == 0 {
            tile = tile.wrapping_sub(1); // 满罐尽⇒切空罐
        }
        mem.write_ram(0x0003, mem.read_ram(0x0003).wrapping_sub(1));
        if mem.read_ram(0x0003) == 0 {
            break;
        }
    }
    mem.set_sprite_page_pos(x);
}
