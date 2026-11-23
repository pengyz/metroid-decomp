//! sprite 管线（AnimDrawObject $DE47 族——r25 端口；对拍驱动）。
//!
//! asm 锚（prg7_engine.asm）：AnimDrawObject $DE47/UpdateObjAnim $DC8F/ObjDrawFrame
//! $DE4A/GetSpriteCntrlData $DCC3（控制字节 AABBXXXX）/DrawSpriteObject $DF19（帧流：
//! <$FC=tile/$FC=偏移/$FD=新控制/$FE=跳位/$FF=尾）/WriteSpriteRAM $DEE6（OAM 四元组）/
//! YDisplacement $DF6B/XDisplacement $DFA3（翻转=二补-8）/ReduceYRadius $DE3D。
//! 数据面=game-data::area::sprite_data（blob——表指针内嵌）。

/// sprite 数据面（资产包段；三表界常量与提取口径在 `game_data::assets`）。
mod sd {
    pub use game_data::assets::{OBJ_ANIM_INDEX_TBL, OBJ_FRAME_PTR_TABLE, OBJ_PLACE_PTR_TABLE};
    pub fn data() -> &'static [u8] {
        game_data::assets::sprite_data()
    }
}

const BLOB_BASE: u16 = 0x8572;

/// `ExplodeRotationTbl ; 07:DC8B`（ROM 实测 `00 80 C0 40`）——爆炸时每帧旋转精灵。
const EXPLODE_ROTATION: [u8; 4] = [0x00, 0x80, 0xC0, 0x40];

/// blob 内字节（指针→偏移——**OOB→0xFF**（环终止形——r26 安全化））。
fn blob(addr: u16) -> u8 {
    sd::data()
        .get(addr.wrapping_sub(BLOB_BASE) as usize)
        .copied()
        .unwrap_or(0xFF)
}

/// AnimDrawObject（$DE47）——对象→OAM（page_index=对象槽；Samus=0）。
/// 【goal-r16 修】ObjDrawFrame（$DE4A）**不更新动画**——asm SetSamusData 链：`jsr UpdateObjAnim`
/// (DC8F) 与 `jmp ObjDrawFrame`(DE4A) 分离。动画由 `update_obj_anim`（physics.rs CD6D 链首件，
/// 含 `sta animDelay`←A 初值重置）负责。旧形在此内联 UpdateObjAnim 且漏 `sta animDelay` 重置⇒
/// animDelay 恒 0⇒每帧换帧（动画过快）+与 update_obj_anim 双写冲突。
pub fn anim_draw_object(mem: &mut crate::frame::Manager, page_index: u8, a_in: u8) {
    // 【goal-轮106 regs.a 源 ID】anim_draw_object 入口插桩。
    // 【goal-轮107 a_in 显式传值修】asm AnimDrawObject 的 A=caller 显式传值（非 regs.a
    // ——regs.a 在 draw 时已被污染=255）——a_in 参数化。
    crate::samus::physics::update_obj_anim(mem, a_in);
    obj_draw_frame(mem, page_index); // ObjDrawFrame（$DE4A 纯绘——DE47=走表+本件）
}

/// ObjDrawFrame（$DE4A）——纯绘制（**无走表**）。AnimDrawObject（$DE47）=UpdateObjAnim
/// （$DC8F）+本件；SetSamusData（$CD6D）尾=`jmp ObjDrawFrame`（单走表形——
/// 【r14 双调根修】旧形尾调 DE47 形致同帧 2×DC8F（AD 周期 4→2 别名（dossier Q4）））。
/// GetNewControlByte（$DF3B）的取值律（纯函数，便于单测）：
/// - `ObjectCntrl` MSB=0 ⇒ **原值直用**（帧数据字节本身就是 OAM attr 位形）；
/// - MSB=1 ⇒ `SpriteAttrsOverride`（E038）`attr = (ctrl & $C0) | (ObjectCntrl & $7F)`。
///
/// 帧数据里 `FD <ctrl>` 是「换一组 sprite 属性」（门/物品的多段 metasprite），
/// 覆盖与否**只由 ObjectCntrl 的 MSB 决定**，与入口那次覆盖同一条律。
pub(crate) fn new_ctrl_byte(c: u8, obj_cntrl: u8) -> u8 {
    if obj_cntrl & 0x80 != 0 {
        (c & 0xC0) | (obj_cntrl & 0x7F)
    } else {
        c
    }
}

/// **帧流环唯一实现**（`DrawSpriteObject ; $DF19`）——对象（`obj_draw_frame`）与
/// 敌人（`crate::render::enemy_sprite::draw_enemy`）共用。
///
/// 历史教训：本环曾在三处各自复制（对象活件 / 敌活件 / 死件），三次漏分支各造成一个
/// 像素缺陷——`0xFC` 偏移只跳过不叠加（case-002 220px）、`0xFD` 覆盖形缺失
/// （f2084 159px）、敌侧 `$6B` MSB 覆盖整支未读（tough/冻结敌调色板恒错）。
/// 语义只允许在这里写一次：`<0xFC` 写 OAM、`0xFC` 加屏位偏移、`0xFD` 换控制字节、
/// `0xFE` 跳过一位件、`$FF` 帧终。
pub(crate) struct SpriteStream<'a> {
    /// 帧数据取字节（对象读 ROM 表、敌读按区资产段）
    pub frame_byte: &'a dyn Fn(u16) -> u8,
    /// 位件表取字节（同上）
    pub place_byte: &'a dyn Fn(u16) -> u8,
    /// 帧数据第 0 字节（AABBXXXX）
    pub ctrl: u8,
    /// `ObjectCntrl($6B)`：MSB=1 覆盖调色板、bit4=水平镜像
    pub obj_cntrl: u8,
    pub screen_x: u8,
    pub screen_y: u8,
}

pub(crate) fn draw_sprite_stream(mem: &mut crate::frame::Manager, s: SpriteStream<'_>) {
    // GetSpriteCntrlData（$DCC3）头：低 2 位=调色板、高 2 位=翻转、PRIORITY 恒置；
    // 尾：`ObjectCntrl` MSB=1 ⇒ SpriteAttrsOverride（$E038）直赋 `(ctrl&$C0)|(ObjectCntrl&$7F)`。
    let mut cntrl05 = if s.obj_cntrl & 0x80 != 0 {
        new_ctrl_byte(s.ctrl, s.obj_cntrl)
    } else {
        ((s.ctrl >> 4) & 0x03) | (s.ctrl & 0xC0) | 0x20
    };
    // 镜像：`$6B` bit4 → HFLIP($40)，逐枚 `attr = Temp05 ^ HFLIP`（WriteSpriteRAM $DEE6）
    let mirror = if s.obj_cntrl & 0x10 != 0 { 0x40u8 } else { 0 };
    let flip_flags = s.ctrl ^ mirror; // place_disp 的翻转口径（Y=$80/X=$40）
    let mut frame_i: u16 = 3; // 0=ctrl / 1=radiusY / 2=radiusX
    let mut place_i: u16 = 0;
    let mut sx = s.screen_x as i16;
    let mut sy = s.screen_y as i16;
    let mut x_sprite = mem.sprite_page_pos() as u16; // SpritePagePos
    // 【#27 v14c】载房窗内（draw_interrupted 且不是载房起拍）：对象**照走帧流**
    // 消耗页槽，但不写 OAM——`SpritePagePos` 照常进位。像素判据见
    // `docs/spec/scroll-work-accounting.md` §9.11：scrollseam f1100 266→0、
    // input-conflict c676 303→0、c678 32→0，door 六格维持 0；机制解释为 C 级。
    let dry = mem.draw_interrupted && !mem.load_started_this_frame;
    loop {
        let b = (s.frame_byte)(frame_i);
        if b < 0xFC {
            // WriteSpriteRAM（$DEE6）：OAM 四元组（y/tile/attr/x）
            let dy = place_disp((s.place_byte)(place_i), flip_flags, true);
            let dx = place_disp((s.place_byte)(place_i + 1), flip_flags, false);
            let oy = (sy as i16 + dy as i16 - 1) as u8; // PPU y+1 形
            let ox = (sx as i16 + dx as i16) as u8;
            let attr = cntrl05 ^ mirror;
            if x_sprite + 4 <= 0x100 {
                if !dry {
                    mem.write_ram(0x0200u16.wrapping_add(x_sprite), oy);
                    mem.write_ram(0x0201u16.wrapping_add(x_sprite), b);
                    mem.write_ram(0x0202u16.wrapping_add(x_sprite), attr);
                    mem.write_ram(0x0203u16.wrapping_add(x_sprite), ox);
                }
                x_sprite = x_sprite.wrapping_add(4);
            }
            place_i += 2;
            frame_i += 1;
        } else if b == 0xFC {
            // OffsetObjectPosition（$DF51）：屏位偏移，不绘制
            sy = sy.wrapping_add((s.frame_byte)(frame_i + 1) as i16);
            sx = sx.wrapping_add((s.frame_byte)(frame_i + 2) as i16);
            frame_i += 3;
        } else if b == 0xFD {
            // GetNewControlByte（$DF3B）：MSB=1 ⇒ E038 覆盖形，否则原值直用
            cntrl05 = new_ctrl_byte((s.frame_byte)(frame_i + 1), s.obj_cntrl);
            frame_i += 2;
        } else if b == 0xFE {
            // SkipPlacementData（$DF32）：跳一位件，不绘制
            place_i += 2;
            frame_i += 1;
        } else {
            break; // $FF = 帧终
        }
        if frame_i > 0x40 {
            break; // 流界护栏（合法帧远小于此）
        }
    }
    // `$FF` 帧终 ⇒ fallthrough 到 `ClearObjectCntrl ; 07:DF2D`（`lda #$00 / sta ObjectCntrl`）。
    // 这一步**不能省**：`$6B` 是「下一个对象/下一枚精灵」的覆盖状态，敌侧
    // `@setHurtPalette`/tough/冻结写入后必须由本环收尾清掉，否则会泄漏给
    // 同一帧后续敌人（录像 f2137：槽 0 的爬虫被槽 1 的受伤调色板污染）。
    mem.set_obj_cntrl(0);
    mem.set_sprite_page_pos(x_sprite as u8); // SpritePagePos 推进
}

/// `ObjDrawFrame` 的 Samus 爆炸特例（`$DE62-$DE8B`）——纯判据面，便于单测。
///
/// asm 逐字：
/// ```text
/// lda IsSamus / beq LDEBC           ; 只对 Samus（对象 0 号槽）
/// cpx #_id_ObjPlace7*2 / bne LDEBC  ; X=(ctrl&$0F)*2 ⇒ 位件 7＝爆炸
/// inc ObjectCounter                 ; 每帧 +1
/// ... and #~(VFLIP|HFLIP) / ora ExplodeRotationTbl,x / sta Temp05_Cntrl
/// cmp #$19 / bne LDEBC
///   lda #sa_Dead2 / sta status ; lda #$28 / sta animDelay
///   pla/pla ⇒ 本帧不画（直接返回调用者）
/// ```
/// `ExplodeRotationTbl ; 07:DC8B` = `00 80 C0 40`（ROM 实测）。
pub(crate) enum ExplodeDraw {
    /// 非 Samus 爆炸帧——按常规绘制。
    NotExploding,
    /// 满 25 帧：已切 `sa_Dead2` 并装 `animDelay=$28`，**本帧不画**。
    Skip,
    /// 旋转后的 OAM attr。
    Attr(u8),
}

pub(crate) fn samus_explode_draw(
    mem: &mut crate::frame::Manager,
    page_index: u8,
    ctrl_byte: u8,
    cntrl05: u8,
) -> ExplodeDraw {
    if page_index != 0 || (ctrl_byte & 0x0F) != 7 {
        return ExplodeDraw::NotExploding;
    }
    let counter = mem.object_counter().wrapping_add(1);
    mem.set_object_counter(counter);
    if counter == 0x19 {
        mem.samus()
            .set_status(crate::samus::state::SamusStatus::Dead2);
        mem.samus().set_anim_delay(0x28);
        mem.clear_obj_cntrl();
        return ExplodeDraw::Skip;
    }
    ExplodeDraw::Attr((cntrl05 & 0x3F) | EXPLODE_ROTATION[(counter & 0x03) as usize])
}

pub fn obj_draw_frame(mem: &mut crate::frame::Manager, page_index: u8) {
    let px = page_index as u16;
    // 只读 animFrame（动画更新在 update_obj_anim——此处不写 $0303/$0304/$0306）
    let anim_frame = mem.read_ram(0x0303 + px);
    // ---- ObjDrawFrame（$DE4A）----
    let frame_idx = anim_frame as u16 * 2;
    let fptr = (sd::data()[sd::OBJ_FRAME_PTR_TABLE as usize + frame_idx as usize] as u16)
        | ((sd::data()[sd::OBJ_FRAME_PTR_TABLE as usize + frame_idx as usize + 1] as u16) << 8);
    let ctrl_byte = blob(fptr);
    // GetSpriteCntrlData（$DCC3）：AABBXXXX——色位→$05 低 2 位；翻转位→$05 高位|PRIORITY($20)
    let mut cntrl05 = (ctrl_byte >> 4) & 0x03;
    let flip = ctrl_byte & 0xC0;
    cntrl05 |= flip | 0x20; // PRIORITY 位恒置（sprite 在 BG 后）
    // ObjectCntrl($6B) 镜像位（bit2→bit6 翻转）
    let obj_cntrl = mem.obj_cntrl();
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
    let pptr = (sd::data()[sd::OBJ_PLACE_PTR_TABLE as usize + place_idx as usize] as u16)
        | ((sd::data()[sd::OBJ_PLACE_PTR_TABLE as usize + place_idx as usize + 1] as u16) << 8);
    // ---- ObjDrawFrame 特例：Samus 爆炸（$DE62-$DE8B）----
    // asm 逐字（prg7_engine.asm，`ObjDrawFrame ; 07:DE4A` 内）：
    //   lda IsSamus / beq LDEBC           ; 只对 Samus
    //   cpx #_id_ObjPlace7*2 / bne LDEBC  ; X=(ctrl&$0F)*2 ⇒ 位件 7＝爆炸
    //   inc ObjectCounter                 ; 每帧 +1
    //   lda ObjectCounter / and #$03 / tax
    //   lda Temp05_Cntrl / and #~(VFLIP|HFLIP) / ora ExplodeRotationTbl,x / sta Temp05_Cntrl
    //   cmp #$19 / bne LDEBC              ; 满 25 帧切下一段
    //   lda #sa_Dead2 / sta Objects.0.status,x
    //   lda #$28 / sta Objects.0.animDelay,x
    //   pla/pla => **本帧不画**（直接返回调用者）
    // ExplodeRotationTbl ; 07:DC8B = 00 80 C0 40（ROM 实测）。
    match samus_explode_draw(mem, page_index, ctrl_byte, cntrl05) {
        ExplodeDraw::Skip => return,
        ExplodeDraw::Attr(a) => cntrl05 = a,
        ExplodeDraw::NotExploding => {}
    }
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
        mem.set_obj_cntrl(0); // ClearObjectCntrl（$DF2D）
        return;
    }
    // ---- DrawSpriteObject（$DF19）帧流环——**唯一实现**见 draw_sprite_stream ----
    draw_sprite_stream(
        mem,
        SpriteStream {
            frame_byte: &|i| blob(fptr.wrapping_add(i)),
            place_byte: &|i| blob(pptr.wrapping_add(i)),
            ctrl: ctrl_byte,
            obj_cntrl,
            screen_x,
            screen_y,
        },
    );
    mem.set_obj_cntrl(0); // 尾 ClearObjectCntrl（$DF2D）
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
    use crate::frame::Manager;

    /// Samus（槽 0）sprite 装载形：OAM 落字节（金侧 f520 萨姆斯 sprite 对形底座）。
    #[test]
    fn samus_sprite_writes_oam() {
        let mut m = Manager::default();
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
        let mut m = Manager::default();
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
        let mut m = Manager::default();
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
        let mut m = Manager::default();
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
        let fptr = (sd::data()[sd::OBJ_FRAME_PTR_TABLE as usize] as u16)
            | ((sd::data()[sd::OBJ_FRAME_PTR_TABLE as usize + 1] as u16) << 8);
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
        let mut m = crate::frame::Manager::default();
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

#[cfg(test)]
mod tests_sprite_stream {
    //! 帧流环**唯一实现**的等价锚（原 `r26_draw_enemy_tdd` 两条随死件删除后补于此处，
    //! 覆盖不变）：①有帧数据 ⇒ 落 OAM ②`animFrame==0xFF` ⇒ 不落 OAM。
    use super::*;
    use crate::frame::Manager;

    fn run(m: &mut Manager, frame: &[u8], place: &[u8]) {
        draw_sprite_stream(
            m,
            SpriteStream {
                frame_byte: &|i| *frame.get(i as usize).unwrap_or(&0xFF),
                place_byte: &|i| *place.get(i as usize).unwrap_or(&0),
                ctrl: frame[0],
                obj_cntrl: 0,
                screen_x: 100,
                screen_y: 60,
            },
        );
    }

    /// 有帧数据（ctrl/radiusY/radiusX/tile…/$FF）⇒ OAM 四元组落（y/tile/attr/x）。
    #[test]
    fn stream_writes_oam_quads() {
        let mut m = Manager::default();
        m.write_ram(0x005B, 0);
        run(&mut m, &[0x21, 0x0C, 0x08, 0xCE, 0xFF], &[0xF4, 0xFC]);
        assert_eq!(m.read_ram(0x0201), 0xCE, "tile 落位");
        assert_eq!(m.read_ram(0x0203), 0x60, "x=100-4（place dx=-4）");
        assert_eq!(m.read_ram(0x0200), 0x2F, "y=60-12-1（place dy=-12，PPU y+1 形）");
        assert_eq!(m.read_ram(0x005B), 4, "SpritePagePos 推进 4");
    }

    /// `$FF` 帧终 ⇒ `ClearObjectCntrl`（`$DF2D`）：`$6B` 必须归零。
    /// 否则调用方（敌 `@setHurtPalette`/tough/冻结）写入的覆盖态会**泄漏给
    /// 同一帧后续对象**——录像 f2137 实证：槽 1 受伤爬虫的 palette-0 覆盖
    /// 污染了后画的槽 0（attr 0x22→0xA0 系）。
    #[test]
    fn stream_clears_object_cntrl_at_frame_end() {
        let mut m = Manager::default();
        m.write_ram(0x006B, 0xA0);
        m.write_ram(0x005B, 0);
        draw_sprite_stream(
            &mut m,
            SpriteStream {
                frame_byte: &|i| *[0x21u8, 0x0C, 0x08, 0xCE, 0xFF].get(i as usize).unwrap_or(&0xFF),
                place_byte: &|i| *[0xF4u8, 0xFC].get(i as usize).unwrap_or(&0),
                ctrl: 0x21,
                obj_cntrl: 0xA0,
                screen_x: 100,
                screen_y: 60,
            },
        );
        assert_eq!(m.read_ram(0x006B), 0, "帧终清 ObjectCntrl");
        assert_eq!(m.read_ram(0x0202), 0x20, "覆盖形：attr=(ctrl&$C0)|($6B&$7F)=(00)|(20)");
    }

    /// 帧首字节即 `$FF`（animFrame=None 的帧面）⇒ 一个 sprite 都不落。
    #[test]
    fn stream_with_only_terminator_writes_nothing() {
        let mut m = Manager::default();
        m.write_ram(0x005B, 0);
        run(&mut m, &[0x21, 0x0C, 0x08, 0xFF], &[0, 0]);
        assert_eq!(m.read_ram(0x0201), 0, "无 sprite");
        assert_eq!(m.read_ram(0x005B), 0, "SpritePagePos 不推进");
    }

    /// `0xFC` 偏移必须**叠加**到后续 sprite 的基准屏位（case-002 f1160 根因：
    /// 旧形只 `frame_i += 3` 不叠加 ⇒ 后半组恒偏 4px）。帧面取自 Skree 帧 37：
    /// `21 0C 08 CE FC 00 FC DE EE FF` —— 第一枚画在 dx=0，之后 x-4 再画两枚。
    #[test]
    fn stream_offset_accumulates_for_following_sprites() {
        let mut m = Manager::default();
        m.write_ram(0x005B, 0);
        run(
            &mut m,
            &[0x21, 0x0C, 0x08, 0xCE, 0xFC, 0x00, 0xFC, 0xDE, 0xEE, 0xFF],
            &[0x04, 0x00, 0x04, 0x00, 0x04, 0x00],
        );
        assert_eq!(m.read_ram(0x0203), 0x64, "首枚 x=100+0");
        assert_eq!(m.read_ram(0x0207), 0x60, "第二枚 x=100-4（偏移累计）");
        assert_eq!(m.read_ram(0x020B), 0x60, "第三枚仍 100-4（累计不丢失）");
    }

    /// `0xFE` 跳过一位件（`SkipPlacementData ; $DF32`）：不绘制，但位件游标前进 2。
    #[test]
    fn stream_skips_placement_on_fe() {
        let mut m = Manager::default();
        m.write_ram(0x005B, 0);
        run(
            &mut m,
            &[0x21, 0x0C, 0x08, 0xFE, 0xCE, 0xFF],
            &[0x04, 0x00, 0x04, 0x08],
        );
        assert_eq!(m.read_ram(0x0201), 0xCE, "跳过一位件后才画");
        assert_eq!(m.read_ram(0x0203), 0x6C, "x=100+8（用了第 2 组的 dx）");
    }
}
/// Samus 爆炸特例（`$DE62-$DE8B`）的三条硬断言：计帧 / 旋转位 / 第 25 帧切换。
#[cfg(test)]
mod tests_samus_explode_draw {
    use super::*;
    use crate::samus::state::SamusStatus;

    #[test]
    fn increments_counter_and_rotates_attr() {
        let mut m = crate::frame::Manager::default();
        // 第 1 帧：counter 0→1 ⇒ 旋转表[1]=0x80
        match samus_explode_draw(&mut m, 0, 0x07, 0x20) {
            ExplodeDraw::Attr(a) => assert_eq!(a, 0x20 | 0x80, "剥翻转位后 OR 表[1]"),
            _ => panic!("应走旋转径"),
        }
        assert_eq!(m.object_counter(), 1);
        // 第 2 帧：表[2]=0xC0
        match samus_explode_draw(&mut m, 0, 0x07, 0x20) {
            ExplodeDraw::Attr(a) => assert_eq!(a, 0x20 | 0xC0),
            _ => panic!(),
        }
        assert_eq!(m.object_counter(), 2);
    }

    #[test]
    fn frame_25_switches_to_dead2_and_skips_draw() {
        let mut m = crate::frame::Manager::default();
        m.set_object_counter(0x18); // 下一帧即 0x19
        assert!(matches!(
            samus_explode_draw(&mut m, 0, 0x07, 0x20),
            ExplodeDraw::Skip
        ));
        assert_eq!(m.object_counter(), 0x19);
        assert_eq!(m.samus().status(), Some(SamusStatus::Dead2));
        assert_eq!(m.samus().anim_delay(), 0x28);
        assert_eq!(m.obj_cntrl(), 0, "ClearObjectCntrl");
    }

    #[test]
    fn non_explode_frames_untouched() {
        let mut m = crate::frame::Manager::default();
        // 非 Samus 页 / 非位件 7 ⇒ 不计数
        assert!(matches!(
            samus_explode_draw(&mut m, 1, 0x07, 0x20),
            ExplodeDraw::NotExploding
        ));
        assert!(matches!(
            samus_explode_draw(&mut m, 0, 0x03, 0x20),
            ExplodeDraw::NotExploding
        ));
        assert_eq!(m.object_counter(), 0);
    }
}
