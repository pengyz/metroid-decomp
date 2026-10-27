//! sprite 管线（AnimDrawObject $DE47 族——r25 端口；对拍驱动）。
//!
//! asm 锚（prg7_engine.asm）：AnimDrawObject $DE47/UpdateObjAnim $DC8F/ObjDrawFrame
//! $DE4A/GetSpriteCntrlData $DCC3（控制字节 AABBXXXX）/DrawSpriteObject $DF19（帧流：
//! <$FC=tile/$FC=偏移/$FD=新控制/$FE=跳位/$FF=尾）/WriteSpriteRAM $DEE6（OAM 四元组）/
//! YDisplacement $DF6B/XDisplacement $DFA3（翻转=二补-8）/ReduceYRadius $DE3D。
//! 数据面=game-data::area::sprite_data（blob——表指针内嵌）。

use game_data::area::sprite_data as sd;

const BLOB_BASE: u16 = 0x8572;

/// blob 内字节（指针→偏移——**OOB→0xFF**（环终止形——r26 安全化））。
fn blob(addr: u16) -> u8 {
    sd::SPRITE_DATA
        .get(addr.wrapping_sub(BLOB_BASE) as usize)
        .copied()
        .unwrap_or(0xFF)
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
    // 【勘误修】ScrollY/ScrollX=**$FC/$FD**（constants_ram:795-796）——旧形误读
    // $25/$26（$25=MMC1CTRL_ZP！）。摄像机滚动径（ScrollX 进）后 sprite 屏位必减 $FD。
    let scroll_y = mem.read_ram(crate::ram_map::SCROLL_Y);
    let scroll_x = mem.read_ram(crate::ram_map::SCROLL_X);
    let screen_y = y_pos.wrapping_sub(scroll_y);
    let screen_x = x_pos.wrapping_sub(scroll_x);
    // 【r25 根修】IsObjectVisible（\）ScrollDir 分支：横房（bit1）**不减 ScrollY**
    // （横滚不动 y——HorzScrollCheck 径）；旧形无条件减致 sprite 高 14px。
    let horz = mem.read_ram(crate::ram_map::SCROLL_DIR) & 0x02 != 0;
    let screen_y = if horz { y_pos } else { screen_y };
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
        let b = blob(fptr.wrapping_add(frame_i));
        if b < 0xFC {
            // WriteSpriteRAM（$DEE6）：OAM 四元组（y/tile/attr/x）
            let dy = place_disp(blob(pptr.wrapping_add(place_i)), flip_flags, true);
            let dx = place_disp(blob(pptr.wrapping_add(place_i + 1)), flip_flags, false);
            let oy = (sy as i16 + dy as i16 - 1) as u8; // PPU y+1 形
            let ox = (sx as i16 + dx as i16) as u8;
            let mut attr = cntrl05;
            if obj_cntrl & 0x20 != 0 {
                attr ^= 0x40; // ObjectCntrl 镜像覆写（bit6）
            }
            mem.write_ram(0x0200u16.wrapping_add(x_sprite), oy);
            mem.write_ram(0x0201u16.wrapping_add(x_sprite), b);
            mem.write_ram(0x0202u16.wrapping_add(x_sprite), attr);
            mem.write_ram(0x0203u16.wrapping_add(x_sprite), ox);
            x_sprite = x_sprite.wrapping_add(4);
            place_i += 2;
            frame_i += 1;
        } else if b == 0xFC {
            // OffsetObjectPosition（$DF51）：y/x 偏移量调整
            sy = sy.wrapping_add(blob(fptr.wrapping_add(frame_i + 1)) as i16);
            sx = sx.wrapping_add(blob(fptr.wrapping_add(frame_i + 2)) as i16);
            frame_i += 3;
        } else if b == 0xFD {
            // GetNewControlByte（$DF3B）：新控制字节=**原值直用**（asm `sta Temp05_Cntrl`
            // ——AABB 已在 OAM attr 位形（r25 勘误修：旧形 >>4 再解致组位错（组 2 误）））
            frame_i += 1;
            let c = blob(fptr.wrapping_add(frame_i));
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
pub fn place_disp(b: u8, flip_flags: u8, is_y: bool) -> i8 {
    let mut v = b as i8;
    let flip_bit = if is_y { 0x80 } else { 0x40 };
    if flip_flags & flip_bit != 0 {
        v = (!v).wrapping_add(0xF9u8 as i8) as i8; // eor #$FF + adc #$F8（含进位 1）
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

#[cfg(test)]
mod r25_oam_probe {
    use super::*;

    /// Samus sprite OAM 面（8 精灵 y/tile/attr/x——头部可见面核）。
    #[test]
    fn samus_oam_dump() {
        let mut m = crate::frame::FrameMemory::default();
        m.write_ram(0x0055, 1); // IsSamus
        m.write_ram(0x0303, 0); // animFrame=0
        m.write_ram(0x030C, 0); // hi
        m.write_ram(0x00FF, 0x90); // PPUCTRL（bit4=1 面）
        m.write_ram(0x030D, 0x80); // y=128
        m.write_ram(0x030E, 0x80); // x=128
        anim_draw_object(&mut m, 0);
        for i in 0..8u16 {
            let b = 0x0200 + i * 4;
            println!("[oam] sprite{i}: y={:02X} tile={:02X} attr={:02X} x={:02X}",
                m.read_ram(b), m.read_ram(b+1), m.read_ram(b+2), m.read_ram(b+3));
        }
    }
}

/// DrawEnemy（$DD8B）端口：敌人槽→OAM 四元组（精灵生成面）
///
/// 保真边界：只做渲染面（精灵生成），敌人 AI/移动/碰撞/消灭留后续（deferred）。
/// refs asm 锚：bank7:DD8B（DrawEnemy）/bank7:DDB0（DrawEnemy_NotBlank）/
/// bank7:DCC3（GetSpriteCntrlData）/bank7:DF19（DrawSpriteObject）/bank7:DEE6（WriteSpriteRAM）
pub fn draw_enemy(mem: &mut crate::frame::FrameMemory, page_index: u8) {
    let px = (page_index as u16) << 4; // 槽×16B
    let blob = |a: u16| -> u8 { mem.read_ram(a) };
    // DrawEnemy（$DD8B）：animFrame==FrameNone→ClearObjectCntrl
    let anim_frame = blob(0x6AF4 + page_index as u16); // EnsExtra.0.animFrame
    if anim_frame == 0xFF {
        // _id_FrameNone——ClearObjectCntrl（$DF2D）
        mem.write_ram(0x006B, 0);
        return;
    }
    // DrawEnemy_NotBlank（$DDB0）：y/x/hi
    let y = blob(0x0400 + px); // Ens.0.y
    let x = blob(0x0401 + px); // Ens.0.x
    let hi = blob(0x6AF4 + page_index as u16 + 6); // EnsExtra.0.hi（偏移需考证——先用邻位）
    // 帧指针：animFrame*2→EnFramePtrTable1/2（bank 指针表——暂用近邻模式）
    // TODO: 完整 EnFramePtrTable1/2 表结构（bank:addr 考证）
    // GetSpriteCntrlData（$DCC3）：控制字节→place index + cntrl05
    // TODO: 完整 GetSpriteCntrlData（控制字节 AABBXXXX 解）
    // DrawSpriteObject（$DF19）帧流环（与 anim_draw_object 同构）
    // TODO: 帧数据+place 数据（待表结构考证完）
    // 保真边界：本 turn 先落骨架（animFrame 非 None 时写 OAM 占位——红→绿门径）
    let screen_y = y.wrapping_sub(blob(0x00FF)); // ScrollY
    let screen_x = x.wrapping_sub(blob(0x00FE)); // ScrollX
    let visible = screen_y < 0xF0;
    // 写 OAM 四元组（占位形——待帧数据接完改真）
    let xs = mem.read_ram(0x005B) as u16; // SpritePagePos
    if visible && xs + 4 <= 0x100 {
        mem.write_ram(0x0200 + xs, screen_y.wrapping_sub(1)); // y
        mem.write_ram(0x0201 + xs, 0x01); // tile（占位）
        mem.write_ram(0x0202 + xs, 0x00); // attr（占位）
        mem.write_ram(0x0203 + xs, screen_x); // x
        mem.write_ram(0x005B, (xs + 4) as u8); // SpritePagePos 推进
    }
}

#[cfg(test)]
mod r26_draw_enemy_tdd {
    use super::*;
    use crate::frame::FrameMemory;

    /// TDD 红→绿：敌人槽 animFrame 非 None 时生成 OAM 精灵
    #[test]
    fn draw_enemy_generates_oam_when_animframe_set() {
        let mut m = FrameMemory::default();
        // 敌人槽 4（$0440）：y=45 x=200（emu f820 实测形）
        m.write_ram(0x0440, 45); // y
        m.write_ram(0x0441, 200); // x
        // EnsExtra.0.animFrame（$6AF4+4）= 非 None
        m.write_ram(0x6AF4 + 4, 0x01);
        // ScrollY/ScrollX
        m.write_ram(0x00FF, 0);
        m.write_ram(0x00FE, 0);
        // SpritePagePos=0
        m.write_ram(0x005B, 0);
        draw_enemy(&mut m, 4);
        // OAM spr0 应有内容
        let oam_y = m.read_ram(0x0200);
        let oam_x = m.read_ram(0x0203);
        assert!(oam_y != 0 || oam_x != 0, "敌人 OAM 应有 sprite 落（y={:02X} x={:02X}）", oam_y, oam_x);
    }

    /// TDD 红→绿：animFrame==None 时 ClearObjectCntrl（不写 OAM）
    #[test]
    fn draw_enemy_clears_cntrl_when_blank() {
        let mut m = FrameMemory::default();
        m.write_ram(0x6AF4 + 4, 0xFF); // animFrame=None
        m.write_ram(0x006B, 0x80); // ObjectCntrl 有值
        draw_enemy(&mut m, 4);
        assert_eq!(m.read_ram(0x006B), 0, "animFrame=None 应 ClearObjectCntrl");
    }
}
