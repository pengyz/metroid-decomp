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
/// 【goal-r16 修】ObjDrawFrame（$DE4A）**不更新动画**——asm SetSamusData 链：`jsr UpdateObjAnim`
/// (DC8F) 与 `jmp ObjDrawFrame`(DE4A) 分离。动画由 `update_obj_anim`（physics.rs CD6D 链首件，
/// 含 `sta animDelay`←A 初值重置）负责。旧形在此内联 UpdateObjAnim 且漏 `sta animDelay` 重置⇒
/// animDelay 恒 0⇒每帧换帧（动画过快）+与 update_obj_anim 双写冲突。
pub fn anim_draw_object(mem: &mut crate::frame::FrameMemory, page_index: u8, a_in: u8) {
    // 【goal-轮106 regs.a 源 ID】anim_draw_object 入口插桩。
    if std::env::var("DSH_SPRITE_TRACE").is_ok() {
        use std::sync::atomic::Ordering;
        let n = crate::frame::mainloop::DIAG_ITER.load(Ordering::Relaxed);
        if n >= 812 && n <= 820 {
            eprintln!("[ado] n={} pi={} a_in={}", n, page_index, a_in);
        }
    }
    // 【goal-轮107 a_in 显式传值修】asm AnimDrawObject 的 A=caller 显式传值（非 regs.a
    // ——regs.a 在 draw 时已被污染=255）——a_in 参数化。
    crate::samus::physics::update_obj_anim(mem, a_in);
    obj_draw_frame(mem, page_index); // ObjDrawFrame（$DE4A 纯绘——DE47=走表+本件）
}

/// ObjDrawFrame（$DE4A）——纯绘制（**无走表**）。AnimDrawObject（$DE47）=UpdateObjAnim
/// （$DC8F）+本件；SetSamusData（$CD6D）尾=`jmp ObjDrawFrame`（单走表形——
/// 【r14 双调根修】旧形尾调 DE47 形致同帧 2×DC8F（AD 周期 4→2 别名（dossier Q4）））。
pub fn obj_draw_frame(mem: &mut crate::frame::FrameMemory, page_index: u8) {
    let px = page_index as u16;
    // 只读 animFrame（动画更新在 update_obj_anim——此处不写 $0303/$0304/$0306）
    let anim_frame = mem.read_ram(0x0303 + px);
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
    // 【物品闪帧相位卡·SpriteAttrsOverride（E038）】GetSpriteCntrlData（DCC3）尾：
    // ObjectCntrl **MSB=1⇒attr=(ctrl_byte&0xC0)|ObjectCntrl**（低 2 位=闪帧调色板
    // ((FC>>1)&3——UpdateOnePowerUp :5547-5554 每帧写 $6B；PRIORITY 位不入覆盖形
    // ——asm E038 `and #HFLIP|VFLIP / ora ObjectCntrl` 直赋 Temp05）。旧形漏此覆盖⇒
    // 物品调色板恒取 ctrl_byte 静态位⇒emu 闪（FC/2 轮换 4 色）/rust 不闪——七格
    // 118px 恒定残差根（diff 图唯一绿点=物品球）。
    if obj_cntrl & 0x80 != 0 {
        // asm E038 `asl/lsr ObjectCntrl` 对=剥 MSB（0x82→0x02）——attr 不含 0x80
        // （MSB 仅=覆盖有效标；尾 `ora #$80` 还原供下件）。PRIORITY 位不入覆盖形。
        cntrl05 = (ctrl_byte & 0xC0) | (obj_cntrl & 0x7F);
    }
    let mut flip_flags = ctrl_byte;
    if obj_cntrl & (0x40 >> 2) != 0 {
        flip_flags ^= 0x40;
    }
    // 位移表索引（低半字节×2）
    let place_idx = (ctrl_byte & 0x0F) as u16 * 2;
    let pptr = (sd::SPRITE_DATA[sd::OBJ_PLACE_PTR_TABLE as usize + place_idx as usize] as u16)
        | ((sd::SPRITE_DATA[sd::OBJ_PLACE_PTR_TABLE as usize + place_idx as usize + 1] as u16) << 8);
    // 半径（帧数据 1/2 字节）
    // 【径源重模卡（2026-11-06——findings §19）】帧格式字节 1/2=radiusY/X（敌径写源
    // 6294 同式）——但 asm **DE4A 全径无径写**（Samus 径=init 15/4+roll 瞬态（D0E1
    // +8）；emu 径恒 15/4=目标（D4 保真））。0xF7=animFrame（walk 表合法帧号 247——
    // DC8F 仅 0xFF=终标记）但帧数据字节 1/2=流垃圾 0xFF⇒径 0xFF⇒span 64 摸地板阻断
    // （每-6 dX=0 根）。**守卫**：animFrame≥0xF0（标记/越界段）跳径写——径保持
    // （asm 语义：DE4A 不写径）；合法帧（<0xF0）径写保留（格式语义——敌/item 兼容）。
    let anim_frame = mem.read_ram(0x0303 + px);
    if anim_frame < 0xF0 {
        mem.write_ram(0x0301 + px, blob(fptr + 1));
        mem.write_ram(0x0302 + px, blob(fptr + 2));
    }
    let radius_x = mem.read_ram(0x0302 + px); // 现径（守卫后——可见性检用）
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
    // 可见性门——**IsObjectVisible(DFDF)→HorzScrollCheck(E01C)** 按 asm 全径（非简化核）：
    // ① 名表判 `lda Temp06_PositionHi/eor PPUCTRL_ZP/and #$01`：同名表⇒LE02E，异⇒异径。
    // ② `sec/sbc ScrollX`⇒carry=(pos≥scrollX)、screenX。
    // ③ 同名表(LE02E)：`bcc LE036`（carry 清⇒不可见）+`lda radiusX/cmp screenX/bcc RTS`（radiusX<screenX⇒可见）。
    // ④ 异名表：`bcs LE036`（carry 置⇒不可见）+`lda radiusX/clc/adc screenX/bcc RTS`（radiusX+screenX<256⇒可见）。
    // 旧形简核（screenY<0xF0 && 名表同）缺 X 界+radiusX⇒f815 起 onScreen=0（左走角色消失根因）。
    let (screen_x_v, borrow) = x_pos.overflowing_sub(scroll_x); // borrow=(x_pos<scroll_x)
    let carry = !borrow; // asm carry=(x_pos≥scroll_x)——overflowing_sub 返回 borrow 非 carry！
    let same_nt = (hi ^ mem.read_ram(0x00FF)) & 1 == 0;
    let visible = if same_nt {
        // LE02E：carry(pos≥scrollX) 且 radiusX<screenX
        carry && radius_x < screen_x_v
    } else {
        // 异名表：carry 清(pos<scrollX) 且 radiusX+screenX<256
        if !carry {
            (radius_x as u16 + screen_x_v as u16) < 0x100
        } else {
            false // bcs LE036（不可见）
        }
    };
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
            if obj_cntrl & 0x10 != 0 {
                attr ^= 0x40; // ObjectCntrl **bit4**=水平镜像（$6B——constants_ram「Bit 4 controls
                              // object horizontal mirroring」；CD92 SamusDir bit0→bit4）。旧形误用
                              // bit5(0x20=priority 位)⇒左朝 sprite 不翻转（贴图错）。
            }
            mem.write_ram(0x0200u16.wrapping_add(x_sprite), oy);
            // 【帧流深挖卡·OAM 写点归因探针（临时——2026-11-06）】
            if std::env::var("DSH_SPRITE_TRACE").is_ok() {
                let n = crate::frame::mainloop::DIAG_ITER.load(std::sync::atomic::Ordering::Relaxed);
                if n >= 1175 && n <= 1195 {
                    eprintln!("[oamw] n={} obj_draw addr=0x{:04X} tile={:02X}", n, 0x0200u16.wrapping_add(x_sprite), b);
                }
            }
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
        anim_draw_object(&mut m, 0, 0);
        let oam: Vec<u8> = (0..0x20).map(|i| m.read_ram(0x0200 + i)).collect();
        assert!(oam.iter().any(|&b| b != 0), "OAM 有 sprite 落");
    }

    /// 【物品闪帧相位卡】GetSpriteCntrlData（DCC3）尾：ObjectCntrl MSB=1⇒
    /// SpriteAttrsOverride（E038）`attr=(ctrl_byte&0xC0)|ObjectCntrl`——低 2 位=
    /// 闪帧调色板 ((FC>>1)&3)（UpdateOnePowerUp :5547-5554 每帧写）。旧形漏此覆盖⇒
    /// 物品调色板恒取 ctrl_byte 静态位⇒emu 闪/rust 不闪（118px 恒定残差根）。
    #[test]
    fn sprite_attrs_override_flash_palette() {
        let mut m = FrameMemory::default();
        m.write_ram(0x0055, 1); // IsSamus（槽 0 绘）
        m.write_ram(0x0303, 0); // animFrame=0
        m.write_ram(0x030C, 0); // hi
        m.write_ram(0x00FF, 0); // PPUCTRL_ZP
        m.write_ram(0x030D, 0x80); // y
        m.write_ram(0x030E, 0x80); // x
        m.write_ram(0x006B, 0x82); // ObjectCntrl=MSB|闪帧调色板 2
        anim_draw_object(&mut m, 0, 0);
        let attr = m.read_ram(0x0202);
        assert_eq!(attr & 0x03, 2, "调色板位=ObjectCntrl 低 2 位（E038 覆盖）");
    }

    /// 【径源重模卡】0xF7 帧（animFrame≥0xF0 标记段）跳径写（径保持——asm DE4A 无
    /// 径写语义）；常规帧（<0xF0）径写保留（帧格式字节 1/2）。
    #[test]
    fn radius_guard_skips_marker_frames() {
        let mut m = FrameMemory::default();
        m.write_ram(0x0055, 1); // IsSamus
        m.write_ram(0x030C, 0); // hi
        m.write_ram(0x00FF, 0); // PPUCTRL_ZP
        m.write_ram(0x030D, 0x80); // y
        m.write_ram(0x030E, 0x80); // x
        m.write_ram(0x0301, 15); // 预置径 15/4（init 常量）
        m.write_ram(0x0302, 4);
        m.write_ram(0x0303, 0xF7); // 标记段帧号
        anim_draw_object(&mut m, 0, 0);
        assert_eq!(m.read_ram(0x0301), 15, "0xF7 帧跳径写（ry 保持 15）");
        assert_eq!(m.read_ram(0x0302), 4, "0xF7 帧跳径写（rx 保持 4）");
        m.write_ram(0x0303, 0); // 常规帧
        m.write_ram(0x0301, 99); // 预置非帧值（验写生效）
        anim_draw_object(&mut m, 0, 0);
        let ry = m.read_ram(0x0301);
        assert_ne!(ry, 99, "常规帧径写生效（帧格式字节 1/2）");
    }

    /// 【守门】ObjectCntrl MSB=0⇒无覆盖（调色板=ctrl_byte 静态位）。
    #[test]
    fn sprite_attrs_no_override_without_msb() {
        let mut m = FrameMemory::default();
        m.write_ram(0x0055, 1);
        m.write_ram(0x0303, 0);
        m.write_ram(0x030C, 0);
        m.write_ram(0x00FF, 0);
        m.write_ram(0x030D, 0x80);
        m.write_ram(0x030E, 0x80);
        m.write_ram(0x006B, 0x02); // 低 2 位=2 但 MSB=0（无覆盖标）
        anim_draw_object(&mut m, 0, 0);
        let attr = m.read_ram(0x0202);
        // 帧 0 静态 ctrl_byte（SPRITE_DATA[OBJ_FRAME_PTR_TABLE] 指针）——期望=其位 4-5
        let fptr = (sd::SPRITE_DATA[sd::OBJ_FRAME_PTR_TABLE as usize] as u16)
            | ((sd::SPRITE_DATA[sd::OBJ_FRAME_PTR_TABLE as usize + 1] as u16) << 8);
        let static_palette = (blob(fptr) >> 4) & 0x03;
        assert_eq!(attr & 0x03, static_palette, "MSB=0⇒调色板=ctrl_byte 静态位");
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
        anim_draw_object(&mut m, 0, 0);
        for i in 0..8u16 {
            let b = 0x0200 + i * 4;
            println!("[oam] sprite{i}: y={:02X} tile={:02X} attr={:02X} x={:02X}",
                m.read_ram(b), m.read_ram(b+1), m.read_ram(b+2), m.read_ram(b+3));
        }
    }
}

/// DrawEnemy（$DD8B）真帧径（**贴图根修**（2026-10-31 用户截图定谳：占位 tile=0x01
/// 人形帧≠真敌帧））：animFrame==FrameNone⇒ClearObjectCntrl；animFrame*2→
/// EnFramePtrTable1/2（carry 选表（asm :6108-6121））→帧流环（DrawSpriteObject DF19
/// 同构）→OAM。数据面=game_data::area::enemy_gfx（r26 烘焙：EN_FRAME_OFFS/EN_FRAMES
/// +EN_PLACE_OFFS/EN_PLACES）。
///
/// 敌结构（constants_ram:85-149——A 级）：Ens=$0400 步距 16（y+0/x+1）；EnsExtra=$6AF4
/// 步距 16（status+0/radiusY+1/radiusX+2/**animFrame+3**/animDelay+4/resetAnimIndex+5/
/// animIndex+6/**hi+7**/type+14）。
pub fn draw_enemy(mem: &mut crate::frame::FrameMemory, page_index: u8) {
    use game_data::area::enemy_gfx as eg;
    let px = (page_index as u16) * 16; // 槽×16B（Ens/EnsExtra 同步距）
    let blob_e = |a: u16| -> u8 { mem.read_ram(a) };
    // DrawEnemy（$DD8B）：animFrame==FrameNone($FF)→ClearObjectCntrl
    let anim_frame = blob_e(0x6AF4 + px + 3); // EnsExtra.0.animFrame
    if anim_frame == 0xFF {
        mem.write_ram(0x006B, 0);
        return;
    }
    // 帧指针（:6108-6121）：animFrame*2→表 1/2（carry=bit7 选表）——烘焙偏移表直映
    let idx = (anim_frame & 0x7F) as usize + if anim_frame & 0x80 != 0 { 128 } else { 0 };
    let (f0, flen) = eg::EN_FRAME_OFFS.get(idx).copied().unwrap_or((0, 0));
    if flen == 0 {
        mem.write_ram(0x006B, 0);
        return;
    }
    let fblob = |i: u16| -> u8 {
        let o = f0 as usize + i as usize;
        *eg::EN_FRAMES.get(o).unwrap_or(&0xFF)
    };
    // y/x/hi（:6096-6105——Ens/EnsExtra）
    let y_pos = blob_e(0x0400 + px);
    let x_pos = blob_e(0x0401 + px);
    let hi = blob_e(0x6AF4 + px + 7); // EnsExtra.0.hi
    let radius_x = blob_e(0x6AF4 + px + 2);
    // GetSpriteCntrlData（DCC3）：ctrl=AABBXXXX
    // 【X 形根修同构】镜像=ObjectCntrl($6B) **bit4**→HFLIP($40) eor（WriteSpriteRAM
    // :6324-6328 逐字）——非 ctrl 位交叉。
    let ctrl_byte = fblob(0);
    let mut cntrl05 = (ctrl_byte >> 4) & 0x03;
    let flip = ctrl_byte & 0xC0;
    cntrl05 |= flip | 0x20;
    let mirror_flags = if mem.read_ram(0x006B) & 0x10 != 0 { 0x40u8 } else { 0 };
    let flip_flags = ctrl_byte ^ mirror_flags;
    // 位件指针（:6124-6130）：(ctrl&$0F)*2→EnPlacePtrTable——烘焙偏移表直映
    let place_idx = (ctrl_byte & 0x0F) as usize;
    let (p0, plen) = eg::EN_PLACE_OFFS.get(place_idx).copied().unwrap_or((0, 0));
    let pblob = |i: u16| -> u8 {
        let o = p0 as usize + i as usize;
        *eg::EN_PLACES.get(o).unwrap_or(&0xFF)
    };
    // 屏位（ScrollY=$FC/ScrollX=$FD 勘误径——anim_draw_object 同构）
    let scroll_y = mem.read_ram(crate::ram_map::SCROLL_Y);
    let scroll_x = mem.read_ram(crate::ram_map::SCROLL_X);
    let screen_y = y_pos.wrapping_sub(scroll_y);
    let screen_x = x_pos.wrapping_sub(scroll_x);
    let horz = mem.read_ram(crate::ram_map::SCROLL_DIR) & 0x02 != 0;
    let screen_y = if horz { y_pos } else { screen_y };
    // 可见性门（IsObjectVisible DFDF→HorzScrollCheck E01C——同构）
    let (screen_x_v, borrow) = x_pos.overflowing_sub(scroll_x);
    let carry = !borrow;
    let same_nt = (hi ^ mem.read_ram(0x00FF)) & 1 == 0;
    let visible = if same_nt {
        carry && radius_x < screen_x_v
    } else if !carry {
        (radius_x as u16 + screen_x_v as u16) < 0x100
    } else {
        false
    };
    if !visible {
        mem.write_ram(0x006B, 0);
        return;
    }
    // ---- DrawSpriteObject（DF19）帧流环（与 anim_draw_object 同构）----
    let mut frame_i: u16 = 3; // 第 4 字节起（0=ctrl/1=radiusY/2=radiusX）
    let mut place_i: u16 = 0;
    let mut sx = screen_x as i16;
    let mut sy = screen_y as i16;
    let mut x_sprite = mem.read_ram(0x005B) as u16; // SpritePagePos
    let _ = plen;
    loop {
        let b = fblob(frame_i);
        if b < 0xFC {
            let dy = place_disp(pblob(place_i), flip_flags, true);
            let dx = place_disp(pblob(place_i + 1), flip_flags, false);
            let oy = (sy as i16 + dy as i16 - 1) as u8;
            let ox = (sx as i16 + dx as i16) as u8;
            let attr = cntrl05 ^ mirror_flags; // WriteSpriteRAM（:6324-6328）形
            if x_sprite + 4 <= 0x100 {
                mem.write_ram(0x0200u16.wrapping_add(x_sprite), oy);
                // 【帧流深挖卡·draw_enemy OAM 写点归因（临时——2026-11-06）】
                if std::env::var("DSH_SPRITE_TRACE").is_ok() {
                    let n = crate::frame::mainloop::DIAG_ITER.load(std::sync::atomic::Ordering::Relaxed);
                    if n >= 1175 && n <= 1195 {
                        eprintln!("[oamw2] n={} draw_enemy addr=0x{:04X} tile={:02X}", n, 0x0200u16.wrapping_add(x_sprite), b);
                    }
                }
                mem.write_ram(0x0201u16.wrapping_add(x_sprite), b);
                mem.write_ram(0x0202u16.wrapping_add(x_sprite), attr);
                mem.write_ram(0x0203u16.wrapping_add(x_sprite), ox);
                x_sprite = x_sprite.wrapping_add(4);
            }
            place_i += 2;
            frame_i += 1;
        } else if b == 0xFC {
            sy = sy.wrapping_add(fblob(frame_i + 1) as i16);
            sx = sx.wrapping_add(fblob(frame_i + 2) as i16);
            frame_i += 3;
        } else if b == 0xFD {
            frame_i += 1;
            cntrl05 = fblob(frame_i);
            frame_i += 1;
        } else if b == 0xFE {
            place_i += 2;
            frame_i += 1;
        } else {
            break; // 0xFF=帧终
        }
        if frame_i > 0x40 {
            break; // 流界护栏
        }
    }
    mem.write_ram(0x005B, x_sprite as u8); // SpritePagePos 推进
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
