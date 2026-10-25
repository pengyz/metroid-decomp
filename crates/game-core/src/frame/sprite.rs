//! sprite 管线（AnimDrawObject $DE47 族——r25 端口；对拍驱动）。
//!
//! asm 锚（prg7_engine.asm）：AnimDrawObject $DE47/UpdateObjAnim $DC8F/ObjDrawFrame
//! $DE4A/GetSpriteCntrlData $DCC3（控制字节 AABBXXXX）/DrawSpriteObject $DF19（帧流：
//! <$FC=tile/$FC=偏移/$FD=新控制/$FE=跳位/$FF=尾）/WriteSpriteRAM $DEE6（OAM 四元组）/
//! YDisplacement $DF6B/XDisplacement $DFA3（翻转=二补-8）/ReduceYRadius $DE3D。
//! 数据面=game-data::area::sprite_data（blob——表指针内嵌）。

use game_data::area::sprite_data as sd;

const BLOB_BASE: u16 = 0x8572;

/// blob 内字节（指针→偏移）。
fn blob(addr: u16) -> u8 {
    sd::SPRITE_DATA
        .get(addr.wrapping_sub(BLOB_BASE) as usize)
        .copied()
        .unwrap_or(0)
}

/// AnimDrawObject（$DE47）——对象→OAM（page_index=对象槽；Samus=0）。
pub fn anim_draw_object(mem: &mut crate::frame::FrameMemory, page_index: u8) {
    let px = page_index as u16;
    // ---- UpdateObjAnim（$DC8F）：animDelay 递减→animIndex 走 ObjectAnimIndexTbl ----
    let mut anim_delay = mem.read_ram(0x0304 + px);
    let mut anim_index = mem.read_ram(0x0306 + px);
    let anim_reset = mem.read_ram(0x0305 + px);
    let mut anim_frame = mem.read_ram(0x0303 + px);
    if anim_delay == 0 || {
        anim_delay = anim_delay.wrapping_sub(1);
        anim_delay == 0
    } {
        // 走表（$8572 起）：$FF=回卷
        loop {
            let b = sd::SPRITE_DATA
                .get(sd::OBJECT_ANIM_INDEX_TBL as usize + anim_index as usize)
                .copied()
                .unwrap_or(0xFF);
            if b == 0xFF {
                anim_index = anim_reset;
                continue;
            }
            anim_frame = b;
            anim_index = anim_index.wrapping_add(1);
            break;
        }
    }
    mem.write_ram(0x0304 + px, anim_delay);
    mem.write_ram(0x0306 + px, anim_index);
    mem.write_ram(0x0303 + px, anim_frame);
    // ---- ObjDrawFrame（$DE4A）----
    let frame_idx = anim_frame as u16 * 2;
    let fptr = (sd::SPRITE_DATA[sd::OBJ_FRAME_PTR_TABLE as usize + frame_idx as usize] as u16)
        | ((sd::SPRITE_DATA[sd::OBJ_FRAME_PTR_TABLE as usize + frame_idx as usize + 1] as u16) << 8);
    let ctrl_byte = blob(fptr);
    // GetSpriteCntrlData（$DCC3）：AABBXXXX——色位→$05 低 2 位；翻转位→$05 高位|PRIORITY($20)
    let mut cntrl05 = (ctrl_byte >> 4) & 0x03;
    let flip = ctrl_byte & 0xC0;
    cntrl05 |= flip | 0x20; // PRIORITY 位恒置（sprite 在 BG 后）
    // ObjectCntrl($6B) 镜像位（bit2→bit6 翻转）
    let obj_cntrl = mem.read_ram(0x006B);
    let mut flip_flags = ctrl_byte;
    if obj_cntrl & (0x40 >> 2) != 0 {
        flip_flags ^= 0x40;
    }
    // 位移表索引（低半字节×2）
    let place_idx = (ctrl_byte & 0x0F) as u16 * 2;
    let pptr = (sd::SPRITE_DATA[sd::OBJ_PLACE_PTR_TABLE as usize + place_idx as usize] as u16)
        | ((sd::SPRITE_DATA[sd::OBJ_PLACE_PTR_TABLE as usize + place_idx as usize + 1] as u16) << 8);
    // 半径（帧数据 1/2 字节）
    let radius_y = blob(fptr + 1);
    let radius_x = blob(fptr + 2);
    mem.write_ram(0x0301 + px, radius_y);
    mem.write_ram(0x0302 + px, radius_x);
    let _ = radius_y.wrapping_sub(0x10); // ReduceYRadius（$DE3D——Temp08 面）
    // 位置→屏幕（IsObjectVisible $DFDF 核）
    let y_pos = mem.read_ram(0x030D + px);
    let x_pos = mem.read_ram(0x030E + px);
    let hi = mem.read_ram(0x030C + px);
    let scroll_y = mem.read_ram(0x0025);
    let scroll_x = mem.read_ram(0x0026);
    let screen_y = y_pos.wrapping_sub(scroll_y);
    let screen_x = x_pos.wrapping_sub(scroll_x);
    // 可见性门（简核：屏界内——全边界径随件）
    let visible = screen_y.wrapping_sub(scroll_y) < 0xF0 && hi & 1 == mem.read_ram(0x00FF) & 1;
    mem.write_ram(0x030B + px, visible as u8); // onScreen
    if !visible {
        mem.write_ram(0x006B, 0); // ClearObjectCntrl（$DF2D）
        return;
    }
    // ---- DrawSpriteObject（$DF19）帧流环 ----
    let mut frame_i: u16 = 3; // 第 4 字节起
    let mut place_i: u16 = 0;
    let mut sx = screen_x as i16;
    let mut sy = screen_y as i16;
    let mut x_sprite = mem.read_ram(0x005B) as u16; // SpritePagePos
    loop {
        let b = blob(fptr + frame_i);
        if b < 0xFC {
            // WriteSpriteRAM（$DEE6）：OAM 四元组（y/tile/attr/x）
            let dy = place_disp(blob(pptr + place_i), flip_flags, true);
            let dx = place_disp(blob(pptr + place_i + 1), flip_flags, false);
            let oy = (sy as i16 + dy as i16 - 1) as u8; // PPU y+1 形
            let ox = (sx as i16 + dx as i16) as u8;
            let mut attr = cntrl05;
            if obj_cntrl & 0x20 != 0 {
                attr ^= 0x40; // ObjectCntrl 镜像覆写（bit6）
            }
            mem.write_ram(0x0200 + x_sprite, oy);
            mem.write_ram(0x0201 + x_sprite, b);
            mem.write_ram(0x0202 + x_sprite, attr);
            mem.write_ram(0x0203 + x_sprite, ox);
            x_sprite = x_sprite.wrapping_add(4);
            place_i += 2;
            frame_i += 1;
        } else if b == 0xFC {
            // OffsetObjectPosition（$DF51）：y/x 偏移量调整
            sy = sy.wrapping_add(blob(fptr + frame_i + 1) as i16);
            sx = sx.wrapping_add(blob(fptr + frame_i + 2) as i16);
            frame_i += 3;
        } else if b == 0xFD {
            // GetNewControlByte（$DF3B）：新控制字节=**原值直用**（asm `sta Temp05_Cntrl`
            // ——AABB 已在 OAM attr 位形（r25 勘误修：旧形 >>4 再解致组位错（组 2 误）））
            frame_i += 1;
            let c = blob(fptr + frame_i);
            cntrl05 = c;
            frame_i += 1;
        } else if b == 0xFE {
            // SkipPlacementData（$DF32）
            place_i += 2;
            frame_i += 1;
        } else {
            // $FF：尾（stx SpritePagePos/ClearObjectCntrl）
            mem.write_ram(0x005B, x_sprite as u8);
            mem.write_ram(0x006B, 0);
            return;
        }
    }
}

/// 位移符号形（Y/XDisplacement 正常径——翻转=二补-8（$DFB1 `eor #$FF/adc #$F8`））。
fn place_disp(b: u8, flip_flags: u8, is_y: bool) -> i8 {
    let mut v = b as i8;
    let flip_bit = if is_y { 0x80 } else { 0x40 };
    if flip_flags & flip_bit != 0 {
        v = (!v).wrapping_add(0xF8u8 as i8) as i8; // eor #$FF + adc #$F8（含进位 1）
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame::FrameMemory;

    /// Samus（槽 0）sprite 装载形：OAM 落字节（金侧 f520 萨姆斯 sprite 对形底座）。
    #[test]
    fn samus_sprite_writes_oam() {
        let mut m = FrameMemory::default();
        m.write_ram(0x0055, 1); // IsSamus
        m.write_ram(0x0303, 0); // animFrame=0（帧 0）
        m.write_ram(0x030C, 0); // hi
        m.write_ram(0x00FF, 0); // PPUCTRL_ZP
        m.write_ram(0x030D, 0x80); // y
        m.write_ram(0x030E, 0x80); // x
        anim_draw_object(&mut m, 0);
        let oam: Vec<u8> = (0..0x20).map(|i| m.read_ram(0x0200 + i)).collect();
        assert!(oam.iter().any(|&b| b != 0), "OAM 有 sprite 落");
    }
}
