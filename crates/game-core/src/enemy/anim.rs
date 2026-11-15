//! 敌人绘制与动画实体化（`DrawEnemy` 07:F42E 邻域）。

use crate::enemy::framework::*;
use crate::enemy::motion::*;
use crate::enemy::prep::*;
use crate::frame::tick::*;
use crate::ram_map;

/// DrawEnemy（**asm 内容锚** `DrawEnemy: ; 07:DD8B`——r26 端口）：animFrame==FrameNone⇒
/// ClearObjectCntrl；帧指针（EnFramePtrTable1/2——animFrame bit7 carry 选表！）→
/// GetSpriteCntrlData→位指针（EnPlacePtrTable）→sprite 管线写（place_disp/四元组）。
pub(crate) fn draw_enemy(mem: &mut crate::frame::FrameMemory, x: u8) {
    // 【goal-轮73 draw 调用直证】双写源（n=803 清→置双写）。
    use game_data::assets;
    // 【②回装（2026-10-29）】asm :6156-6168：帧记字 1/2→EnsExtra.radiusY/X（IsObject-
    // Visible 用半径（序=半径写先行））+[:6178-6186] bit1 逐帧闩（and #$FD/ora（txa<<1））。
    // 爆亡链件已补（F40D/DCF5/F483 全径（415 测））⇒连锁闭环（§10 回装判据达成）。
    // **按区**取敌帧/位数据面（`docs/findings/per-bank-assets.md` §1：五区基址/表长各异）。
    let area = crate::world::room_load::area_idx(mem.read_ram(ram_map::IN_AREA)) as u8;
    let frame0 = mem.read_wram(0x6AF4 + x as u16 + 3);
    if frame0 != 0xFF {
        let fi0 = ((frame0 & 0x7F) as usize) + if frame0 & 0x80 != 0 { 128 } else { 0 };
        let (fo0, fl0) = assets::enemy_frame_off(area, fi0 as usize).unwrap_or((0, 0));
    // 【goal-轮94 fo0→radiusY 直证】帧面 radiusY（金方 1/9 vs 我 8）。
        if fl0 > 2 {
            mem.write_wram(0x6AF4 + x as u16 + 1, assets::enemy_frames(area)[fo0 as usize + 1]);
            mem.write_wram(0x6AF4 + x as u16 + 2, assets::enemy_frames(area)[fo0 as usize + 2]);
        }
    }
    // 【goal-轮75 asm 合形修】draw 走 fresh 检（:6177 `jsr IsObjectVisible` 逐字——
    // 旧形走闩径（免 fresh）=偏离）——闩径 F37F 在 update 面（非 draw）。
    let visible = enemy_check_if_visible_inner(mem, x);
    let e5 = 0x0400 + x as u16 + 5;
    let d5 = mem.read_ram(e5);
    mem.write_ram(e5, (d5 & 0xFD) | ((visible as u8) << 1)); // bit1 闩
    if !visible {
        mem.write_ram(0x006B, 0);
        return;
    }
    let frame = mem.read_wram(0x6AF4 + x as u16 + 3); // animFrame=+3
    if frame == 0xFF {
        mem.write_ram(0x006B, 0);
        return;
    }
    let table2 = frame & 0x80 != 0;
    let fi = ((frame & 0x7F) as usize) + if table2 { 128 } else { 0 };
    let (fo, fl) = assets::enemy_frame_off(area, fi as usize).unwrap_or((0, 0));
    let blob = |i: usize| -> u8 {
        if fl == 0 || i >= fl as usize { 0 } else { assets::enemy_frames(area)[fo as usize + i] }
    };
    let ctrl = blob(0);
    // 【X 形根修（2026-10-31 asm WriteSpriteRAM（DEE6）逐字）】$6B=**对象控制字节**
    // （SetMirrorCntrlBit/敌 tough 面写）**非帧 ctrl**——旧形 `sta $6B=帧ctrl` 覆写
    // +镜像映射交叉（bit6→0x80）⇒四精灵翻位成 X 形（金=星芒——贴图错根）。
    // 正解：attr=Temp05 ^（ObjectCntrl bit4<<2 & HFLIP($40)）（asm :6324-6328）；
    // 镜像面 Temp04=ctrl ^ 同源（GetSpriteCntrlData :5913-5920）。
    let obj_mirror = mem.read_ram(0x006B) & 0x10; // bit4=水平镜像控制
    let mirror_flags = if obj_mirror != 0 { 0x40u8 } else { 0 };
    let place_idx = ((ctrl & 0x0F) as usize).min(127);
    let (po, pl) = assets::enemy_place_off(area, place_idx as usize).unwrap_or((0, 0));
    let pblob = |i: usize| -> u8 {
        if pl == 0 || i >= pl as usize { 0 } else { assets::enemy_places(area)[po as usize + i] }
    };
    let y = mem.read_ram(0x0400 + x as u16 + 0); // y=+0
    let xpos = mem.read_ram(0x0400 + x as u16 + 1); // x=+1
    let scroll_y = mem.read_ram(crate::ram_map::SCROLL_Y);
    let horz = mem.read_ram(crate::ram_map::SCROLL_DIR) & 0x02 != 0;
    let sy = (if horz { y } else { y.wrapping_sub(scroll_y) }) as i16;
    // 【勘误修（2026-10-29（问题 A 根因））】敌屏位 x=房位−**ScrollX($FD)**——旧形减
    // $26（SpareMem26（constants:488（$25=MMC1CTRL_ZP 同族误读（sprite.rs 勘误注
    // 早已证）））——走滚径写 $FD（scroll_right）而 0x26 停旧值⇒敌屏位不减镜头⇒
    // **随镜恒距**（用户截图症（爬虫悬空错位））。
    let sx = xpos.wrapping_sub(mem.read_ram(crate::ram_map::SCROLL_X)) as i16;
    let mut cntrl05 = ((ctrl >> 4) & 0x03) | (ctrl & 0xC0) | 0x20;
    let mut place_i = 0usize;
    let mut frame_i = 3usize;
    let mut x_sprite = mem.read_ram(0x005B) as u16;
    loop {
        let b = blob(frame_i);
        if b < 0xFC {
            let dy = crate::render::oam::place_disp(pblob(place_i), ctrl ^ mirror_flags, true);
            let dx = crate::render::oam::place_disp(pblob(place_i + 1), ctrl ^ mirror_flags, false);
            // WriteSpriteRAM（:6324-6328）：attr=Temp05 ^ (ObjectCntrl bit4→HFLIP)
            let attr = cntrl05 ^ mirror_flags;
            let oy = (sy + dy as i16 - 1) as u8;
            let ox = (sx + dx as i16) as u8;
            mem.write_ram(0x0200 + x_sprite, oy);
            mem.write_ram(0x0201 + x_sprite, b);
            mem.write_ram(0x0202 + x_sprite, attr);
            mem.write_ram(0x0203 + x_sprite, ox);
            x_sprite = x_sprite.wrapping_add(4);
            place_i += 2;
            frame_i += 1;
        } else if b == 0xFC {
            frame_i += 3;
        } else if b == 0xFD {
            frame_i += 1;
            cntrl05 = blob(frame_i);
            frame_i += 1;
        } else if b == 0xFE {
            place_i += 2;
            frame_i += 1;
        } else {
            break;
        }
    }
    mem.write_ram(0x005B, x_sprite as u8);
}

/// `UpdateEnemyAnim ; 07:E094`（走表动画推进）。
/// UpdateEnemyAnim（**asm 内容锚** `UpdateEnemyAnim: ; 07:E094`——敌人动画走表全径）：
/// status==Pickup(5)⇒RTS；delay 门（≠0 递减未归零⇒RTS）→`sta animDelay←A`→走
/// EN_ANIM_TABLE[animIndex]（区表——Brinstar $9D6A）：FF⇒animIndex←reset 续走；
/// 否则 animFrame(+3)←表值、animIndex+1。调用形：CrawlerAI @exit `lda #3/jsr $800C`。
pub(crate) fn update_enemy_anim(mem: &mut crate::frame::FrameMemory, x: u8, a: u8) {
    let base = 0x6AF4u16 + x as u16;
    if mem.read_wram(base) == 5 {
        return; // Pickup 态不更
    }
    let delay = mem.read_wram(base + 4);
    if delay != 0 {
        let d = delay - 1;
        mem.write_wram(base + 4, d);
        if d != 0 {
            return;
        }
    }
    mem.write_wram(base + 4, a); // sta animDelay←A
    let area = crate::world::room_load::area_idx(mem.read_ram(ram_map::IN_AREA)) as u8;
    let reset = mem.read_wram(base + 5);
    let mut y = mem.read_wram(base + 6) as usize;
    for _ in 0..16u32 {
        let v = *game_data::assets::enemy_anim_table(area).get(y).unwrap_or(&0xFF);
        if v == 0xFF {
            y = reset as usize; // 回绕续走
            continue;
        }
        mem.write_wram(base + 3, v); // animFrame←表值
        mem.write_wram(base + 6, y.wrapping_add(1) as u8);
        // 【goal-轮85 animIndex 初值直证】ENEMY_*_ANIM 表径（帧面差源）。
        return;
    }
    crate::unported!("bank07:E0BC-wrap"); // 表不变量破裂——fail-loud（update_obj_anim 同式）
}
