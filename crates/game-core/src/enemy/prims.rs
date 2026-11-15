//! 敌人底层原语：表读取、位工具、每槽预备件（可见性/朝向力/距离阈/休止动画索引）。
//!
//! 由 `frame/enemy/{update,move_shell}.rs` 原样搬出（纯搬移，语义分层 P1）。
//! asm 根：`LoadTableAt977B`、`EnemyCheckIfVisible`、`EnemyForceSpeedTowardsSamus ; 07:F6DF`、
//! `EnData05DistanceToSamusThreshold ; 07:F75B`、`InitEnRestingAnimIndex`。

use crate::frame::tick::*;
use crate::frame::FrameMemory;
use crate::ram_map;

pub(crate) use crate::enemy::tables::{en_addr, ens_addr, enemy_type_times2_plus_facing, l968b_of, l977b_raw, load_table_at_977b};

pub(crate) const EN_BASE: u16 = 0x0400;
pub(crate) const ENS_BASE: u16 = 0x6AF4;
pub(crate) const EN_F_DATA05: u16 = 5;
pub(crate) const EN_F_SPEED_Y: u16 = 2;
pub(crate) const EN_F_SPEED_X: u16 = 3;
pub(crate) const ENS_F_STATUS: u16 = 0;
pub(crate) const ENS_F_ACCEL_Y: u16 = 10;
pub(crate) const ENS_F_ACCEL_X: u16 = 11;
pub(crate) const ENS_F_TYPE: u16 = 14;
pub(crate) const ENS_F_DATA1F: u16 = 15;

// 【goal-轮37 记账】可见旗写面（asm :6178-6186 `and #$FD/ora (flag<<1)/sta data05`）
// 在**绘径**（非 F37F 检）——F37F 闩位读面已折；写面随件（DIVERGENCES 记账臂）。
pub(crate) fn enemy_check_if_visible_inner(mem: &mut crate::frame::FrameMemory, x: u8) -> bool {
    // 【goal-轮61 可见位判输入直证】hi/same_nt/carry（金方 bit1=0 在屏却判不可见源）。
    // IsObjectVisible（**asm 内容锚** `IsObjectVisible: ; 07:DFDF`——2026-10-31 忠实重建）：
    // 旧形横房分支 y<240 近似恒真且丢名表/横滚轴——载入邻名表敌（hi≠PPU 当前 NT）被误判
    // 可见 ⇒ Skree（slot2）提前可见+激活（crawlprobe f919-959 残差根因）。asm 判定链：
    // screenY=y−ScrollY/screenX=x−ScrollX → 横房走 HorzScrollCheck（E01C）：入 carry=
    // （x≥ScrollX）（ScreenX sbc 残留）；异名表+carry 置⇒隐形/异名表+carry 清⇒rx+screenX
    // <256 可见；同名表⇒carry 清隐形/rx<screenX 可见。纵房 VertScrollCheck（DFF6）同构
    // （cpy ScrollY 出 carry；异名表 sbc #$0F 带 C=0 ⇒ −16；y 轴检用 radiusX——asm quirk）。
    let y = mem.read_ram(0x0400 + x as u16 + 0);
    let xpos = mem.read_ram(0x0400 + x as u16 + 1);
    let hi = mem.read_wram(0x6AF4 + x as u16 + 7);
    let rx = mem.read_wram(0x6AF4 + x as u16 + 2); // Temp09_RadiusX（y 轴检也用 rx——asm 注 "why is $09"）
    let scroll_y = mem.read_ram(crate::ram_map::SCROLL_Y);
    let scroll_x = mem.read_ram(crate::ram_map::SCROLL_X);
    let screen_y = y.wrapping_sub(scroll_y); // Temp10_ScreenY
    let screen_x = xpos.wrapping_sub(scroll_x); // Temp0E_ScreenX
    let same_nt = (hi ^ mem.read_ram(0x00FF)) & 0x01 == 0;
    if mem.read_ram(crate::ram_map::SCROLL_DIR) & 0x02 != 0 {
        let carry = xpos >= scroll_x; // sec/sbc ScrollX 残留
        if !same_nt {
            if carry {
                return false; // bcs LE036（异名表+敌在滚轴右⇒隐形）
            }
            return (rx as u16 + screen_x as u16) < 0x100; // bcc RTS_E037=可见
        }
        if !carry {
            return false; // bcc LE036
        }
        return rx < screen_x; // bcc RTS_E037=可见
    }
    let carry = y >= scroll_y; // cpy ScrollY 残留
    if !same_nt {
        if carry {
            return false; // bcs LE01A
        }
        let sy2 = screen_y.wrapping_sub(16); // sbc #$0F（C=0 ⇒ −16）
        // asm `clc/adc` 8-bit 和会回绕（cmp 按回绕值判）——verifier 复核角部勘误（正常
        // 半径不可达，位精确对齐）。
        return rx.wrapping_add(sy2) < 0xF0; // cmp #$F0/bcc=可见
    }
    if !carry {
        return false; // bcc LE01A
    }
    rx < screen_y // cmp/bcc=可见
}

pub(crate) fn enemy_force_speed_x_axis(mem: &mut crate::frame::FrameMemory, x: u8, attribs: u8) {
    let e = 0x0400 + x as u16 + 5;
    mem.write_ram(e, mem.read_ram(e) & !0x01); // AndEnData05 #~$01
    let bit0 = enemy_facing_towards_samus_bit(mem, x);
    let d = mem.read_ram(e);
    mem.write_ram(e, d | bit0); // OrEnData05
    // lsr/ror：bit0 → bit7；再 eor speedX 的符号位
    let facing_left = bit0 != 0;
    let sx = mem.read_ram(0x0400 + x as u16 + 3); // Ens.speedX
    if facing_left == (sx & 0x80 != 0) {
        return; // bpl @endIf_D（已朝萨姆斯）
    }
    if attribs & 0x20 != 0 {
        return; // flipSpeed 首件：L968B bit5 ⇒ rts
    }
    let tbl = load_table_at_977b(mem, x); // A = L977B[type]（carry = 原字节 bit7）
    let mut c_in = 1u8; // sec
    if tbl & 0x80 != 0 {
        // **speedSubPixelX = En+7**（ROM 自证 `$81E8: A9 00 / FD 07 04 / 9D 07 04`；
        // `+9` 是 `delay`——r9 复核抓出本行原写 +9，会把 delay 取负）。
        let s = mem.read_ram(0x0400 + x as u16 + EN_SUBPIXEL_X);
        mem.write_ram(0x0400 + x as u16 + EN_SUBPIXEL_X, s.wrapping_neg());
        c_in = if s == 0 { 1 } else { 0 }; // sbc 借位传出（无借位 ⇔ s==0）
    }
    let v = sx.wrapping_neg().wrapping_sub((1 - c_in) as u8);
    mem.write_ram(0x0400 + x as u16 + 3, v);
}

/// x 轴朝向位（`07:F6F5`–`07:F70A`）：横滚房异名表 ⇒ `EnsExtra.hi ^ PPUCTRL_ZP`
/// 的 bit0；否则 `enX ≥ Samus.x`（`enX == Samus.x` 时先 `inc Enemy82($0082)`）。
pub(crate) fn enemy_facing_towards_samus_bit(mem: &mut crate::frame::FrameMemory, x: u8) -> u8 {
    let dir = mem.read_ram(crate::ram_map::SCROLL_DIR);
    if dir >= 0x02 {
        let en_hi = mem.read_wram(0x6AF4 + x as u16 + 7); // EnsExtra.hi
        let sam_hi = mem.read_ram(crate::ram_map::SAMUS_HI);
        if (en_hi ^ sam_hi) & 0x01 != 0 {
            return (en_hi ^ mem.read_ram(crate::ram_map::PPUCTRL_ZP)) & 0x01;
        }
    }
    let ex = mem.read_ram(0x0400 + x as u16 + 1);
    let sx = mem.read_ram(crate::ram_map::SAMUS_X);
    if ex == sx {
        let v = mem.read_ram(0x0082);
        mem.write_ram(0x0082, v.wrapping_add(1)); // inc Enemy82（不影响 carry）
    }
    (ex >= sx) as u8
}

pub(crate) fn enemy_force_speed(mem: &mut crate::frame::FrameMemory, x: u8) {
    mem.write_ram(0x0082, 0);
    let t = mem.read_wram(0x6AF4 + x as u16 + 14) as usize & 0x0F;
    let attribs = game_data::assets::enemy_force_attribs(crate::frame::cur_area(mem))[t];
    let status = mem.read_wram(0x6AF4 + x as u16);
    if status == 2 && attribs & 0x02 == 0 {
        return;
    }
    let d0d = mem.read_ram(0x0400 + x as u16 + 13);
    if d0d.wrapping_sub(1) != 0 {
        mem.write_ram(0x0400 + x as u16 + 13, d0d.wrapping_sub(1));
        return;
    }
    let delay = game_data::assets::enemy_force_speed_delay(crate::frame::cur_area(mem)).get(t).copied().unwrap_or(1);
    mem.write_ram(0x0400 + x as u16 + 13, delay);
    if attribs & 0x80 != 0 {
        // x 轴力（asm `UpdateEnemy_Active` 内 F6DF–F70F；占位臂已端口 2026-11-14）
        enemy_force_speed_x_axis(mem, x, attribs);
    }
    // @skipXAxis（F70F）y 轴力：data05 bit2 清+名表/位比较→bit2 置+速向力
    let e = 0x0400 + x as u16 + 5;
    let d = mem.read_ram(e);
    mem.write_ram(e, d & !0x04); // AndEnData05(~$04)
    let dir = mem.read_ram(crate::ram_map::SCROLL_DIR);
    let mut en_under = false;
    if dir < 0x02 {
        // 竖房：同名表判（简化径：y 比较）
        let en_y = mem.read_ram(0x0400 + x as u16 + 0);
        let sam_y = mem.read_ram(0x030D);
        if en_y != sam_y {
            en_under = en_y > sam_y; // carry=敌在萨姆斯下
        } else {
            mem.write_ram(0x0082, mem.read_ram(0x0082).wrapping_add(2)); // Enemy82 bit1
        }
    } else {
        let en_y = mem.read_ram(0x0400 + x as u16 + 0);
        let sam_y = mem.read_ram(0x030D);
        if en_y != sam_y {
            en_under = en_y > sam_y;
        } else {
            mem.write_ram(0x0082, mem.read_ram(0x0082).wrapping_add(2));
        }
    }
    if en_under {
        mem.write_ram(e, mem.read_ram(e) | 0x04); // OrEnData05 bit2
    }
    // 速向力（face bit2 vs speedY 符号——**异向⇒翻**（asm F73E `eor speedY/bpl RTS`：
    // bit2⊕sy₇=0 跳过）——【2026-10-31 条件取反修】旧形 `want_up=bit2==0` 把判据写反
    // ⇒敌在萨姆斯上方（bit2=0、speedY=0）时误翻 ⇒data02 残留 0x80（emu 恒 00——
    // crawlprobe f797 残差字节根因）。
    let d = mem.read_ram(e);
    let face_up = d & 0x04 != 0; // bit2=1=朝上（敌在萨姆斯下——OrEnData05 刚写）
    let sy = mem.read_ram(0x0400 + x as u16 + 2); // speedY
    if face_up != (sy & 0x80 != 0) {
        // flipSpeed（asm $820F 垂直形）：L968B bit5 门（置位⇒跳过——movement-string 型）
        // →977b 表 bit7 门（加速型⇒sub-pixel 取负）→speedY 取负。sbc 链带 sec 进位
        // （0−x=补码；sub-pixel sbc 的借位传入 speedY sbc——s≠0 时 speedY'=!sy quirk）。
        if attribs & 0x20 == 0 {
            let tbl = load_table_at_977b(mem, x);
            let mut c_in = 1u8; // sec（asm $821B）
            if tbl & 0x80 != 0 {
                // **speedSubPixelY = En+6**（与 movementInstrIndex 同字节别名；ROM 自证
                // `$8343: 7D 06 04 / 9D 06 04`、`$822C: FD 06 04 / 9D 06 04`；`+8` 是
                // movementIndex——r9 复核抓出本行原写 +8）。
                let s = mem.read_ram(0x0400 + x as u16 + EN_SUBPIXEL_Y);
                mem.write_ram(0x0400 + x as u16 + EN_SUBPIXEL_Y, s.wrapping_neg());
                c_in = if s == 0 { 1 } else { 0 }; // sbc 借位传出（无借位⇔s==0）
            }
            let v = sy.wrapping_neg().wrapping_sub((1 - c_in) as u8);
            mem.write_ram(0x0400 + x as u16 + 2, v); // speedY 取负（sbc 链忠实）
        }
    }
}

/// UpdateEnemy_EnData05DistanceToSamusThreshold（**asm 内容锚** `: 07:F75B`——r26 全径）：
/// ①data05 bits3/4 置+掩码默认 ②阈表（ENEMY_DISTANCE_THRESHOLD[type]）为 0⇒早出
/// （**Zoomer 径！**）③bit1（不可见）⇒清 bits3/4 ④bit7 轴选→掩 bit3/4 ⑤|敌-萨|>>4
/// vs 阈&$7F：距大⇒清对应位。Or/AndEnData05=bits3/4 掩写件。
pub(crate) fn enemy_distance_threshold(mem: &mut crate::frame::FrameMemory, x: u8) {
    let e = 0x0400 + x as u16 + 5; // Ens.data05
    let d = mem.read_ram(e);
    mem.write_ram(e, d | 0x18); // OrEnData05($18)
    let t = mem.read_wram(0x6AF4 + x as u16 + 14) as usize & 0x0F;
    let thr = game_data::assets::enemy_distance_threshold(crate::frame::cur_area(mem))[t];
    if thr == 0 {
        return; // 阈 0⇒早出（Zoomer 径！）
    }
    let d = mem.read_ram(e);
    if d & 0x02 == 0 {
        // 不可见⇒清 bits3/4（Lx345）
        mem.write_ram(e, d & !0x18);
        return;
    }
    // asm `ldy #~$08` 默认；`asl / bcs Lx342`——阈 bit7 置 ⇒ 掩 bit3，否则 `ldy #~$10` 掩 bit4。
    let mask = if thr & 0x80 != 0 { !0x08u8 } else { !0x10u8 };
    let thr_v = thr & 0x7F;
    let d = mem.read_ram(e);
    // asm：轴由 d05 bit7 选（置⇒y 轴，清⇒x 轴）
    let (sam_pos, en_pos) = if d & 0x80 != 0 {
        (mem.read_ram(crate::ram_map::SAMUS_Y), mem.read_ram(0x0400 + x as u16))
    } else {
        (mem.read_ram(crate::ram_map::SAMUS_X), mem.read_ram(0x0400 + x as u16 + 1))
    };
    // 【2026-11-15 修】Samus.hi 原读 $6AF4（=EnsExtra 槽 0 的 status）——真址 Objects 槽 0 的
    // hi = $030C（constants_ram Objects 结构 +0x0C）。错址令阈值判据整体失真。
    let sam_hi = mem.read_ram(crate::ram_map::SAMUS_HI) & 1;
    let en_hi = mem.read_wram(0x6AF4 + x as u16 + 7) & 1; // EnsExtra.hi（$6AFB 起）
    // asm `lsr / ror Temp`：位置右移 1 位、hi 位旋入 bit7 ⇒ 值 = (hi*256+pos)/2（8 位模）
    let sam = ((sam_hi << 7) | (sam_pos >> 1)) as u8;
    let en = ((en_hi << 7) | (en_pos >> 1)) as u8;
    // `sec / sbc` 8 位；负⇒二补取绝对值；再 `lsr`×3 ⇒ 合计 /16
    let diff = en.wrapping_sub(sam);
    let abs = if (diff as i8) < 0 { diff.wrapping_neg() } else { diff };
    let dist = abs >> 3;
    if dist >= thr_v {
        mem.write_ram(e, d & mask); // 距大⇒清对应位
    }
}

/// InitEnRestingAnimIndex（**asm 内容锚** `: 07:F682`——r26）：type×2+facing 索引→
/// ENEMY_RESTING_ANIM[y]；≠resetAnimIndex⇒写 resetAnimIndex/animIndex+animDelay←0。
pub(crate) fn init_en_resting_anim_index(mem: &mut crate::frame::FrameMemory, x: u8) {
    let y = enemy_type_times2_plus_facing(mem, x);
    let idx = game_data::assets::enemy_resting_anim(crate::frame::cur_area(mem))[y as usize & 0x1F];
    if idx == mem.read_wram(0x6AF4 + x as u16 + 5) {
        return; // 已同⇒RTS（animDelay 不清）
    }
    mem.write_wram(0x6AF4 + x as u16 + 5, idx); // resetAnimIndex
    mem.write_wram(0x6AF4 + x as u16 + 6, idx); // animIndex
    mem.write_wram(0x6AF4 + x as u16 + 4, 0); // animDelay←0
}

/// `XorEnData05 ; $856B`。
pub(crate) fn xor_en_data05(mem: &mut crate::frame::FrameMemory, x: u8, mask: u8) {
    let a = en_addr(x, EN_F_DATA05);
    mem.write_ram(a, mem.read_ram(a) ^ mask);
}

/// `sec/clc + ror hi + ror lo`：16 位右移一位，进位补进 hi 的 bit7。
/// `carry_in=true` = asm 的 `sec`（减半并把方向钉成负），`false` = `clc`（减半保持正）。
pub(crate) fn ror16(hi: u8, lo: u8, carry_in: bool) -> (u8, u8) {
    let v = ((hi as u16) << 8) | lo as u16;
    let r = (v >> 1) | if carry_in { 0x8000 } else { 0 };
    ((r >> 8) as u8, (r & 0xFF) as u8)
}

/// `twos_complement`：`lda #$00 / sbc v` ⇒ `0 - v`。
pub(crate) fn twos(v: u8) -> u8 {
    v.wrapping_neg()
}


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
    let reset = mem.read_wram(base + 5);
    let mut y = mem.read_wram(base + 6) as usize;
    for _ in 0..16u32 {
        let v = *game_data::assets::enemy_anim_table().get(y).unwrap_or(&0xFF);
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

/// EnemyMoveOnePixelUp（**asm 内容锚** `areas_common.asm:989 $83F5`——r26）：
/// 顶界块界判（(y-radiusY)&7==0⇒EnemyCheckMoveUp）→y dec（y==0 换名表族）→
/// radiusY 界判→成功（carry=1）/失败（carry=0）。返回 bool（carry）。
pub(crate) fn enemy_move_one_pixel_up(mem: &mut crate::frame::FrameMemory, x: u8) -> bool {
    let y = mem.read_ram(0x0400 + x as u16 + 0); // y=+0
    let ry = mem.read_wram(0x6AF4 + x as u16 + 1); // radiusY=+1
    if y.wrapping_sub(ry) & 0x07 == 0 {
        // 块界⇒碰撞判（EnemyCheckMoveUp——随件 fail-loud）
        if !enemy_check_move_up(mem, x) {
            mem.write_ram(0x00, 0); // $00=0（碰撞）
            return false;
        }
    }
    let ny = y.wrapping_sub(1);
    mem.write_ram(0x0400 + x as u16 + 0, ny);
    if y == 0 {
        // 【wrap 值勘误（asm :1011-1016 逐字：`ldy y/bne`⇒y==0⇒`ldy #SCRN_VY($F0)`）】
        // 旧形 0xEF 偏 1px——asm=**$F0**（屏高 240 名表换补偿）。
        mem.write_ram(0x0400 + x as u16 + 0, 0xF0);
        mem.write_ram(0x00, 1); // $00=1（离屏/wrap）
        return false;
    }
    // 成功：jumpDsplcmnt inc（横朝向时）
    if mem.read_ram(0x0400 + x as u16 + 5) & 0x80 == 0 {
        let j = mem.read_wram(0x6AF4 + x as u16 + 13); // jumpDsplcmnt=+13
        mem.write_wram(0x6AF4 + x as u16 + 13, j.wrapping_add(1));
    }
    true
}

/// EnemyMoveOnePixelDown（**asm 内容锚** `: 1066 $844B`——r26 同构（y inc））。
pub(crate) fn enemy_move_one_pixel_down(mem: &mut crate::frame::FrameMemory, x: u8) -> bool {
    // 【goal-轮104 move_down 调用计数直证】调用/施加径差。
    // 【goal-轮82 animIndex 推进率直证】帧面差源（敌 rx≤4 vs 8）。
    let y = mem.read_ram(0x0400 + x as u16 + 0);
    let ry = mem.read_wram(0x6AF4 + x as u16 + 1);
    if y.wrapping_add(ry) & 0x07 == 0 {
        if !enemy_check_move_down(mem, x) {
            mem.write_ram(0x00, 0); // $00=0（碰撞）
            // 【goal-轮101 ok 失败径直证】块撞。
            return false;
        }
    }
    // 【goal-轮101 ok 失败径直证】wrap。
    let ny = y.wrapping_add(1);
    // 【goal-轮162 animIndex 写源直证】EN_ANIM_TABLE[animIndex] 节拍（paradox 解径）
    // ——金方 animIndex 差 ⇒ animFrame 差 ⇒ fo0 差 ⇒ radiusY=1/9。
    mem.write_ram(0x0400 + x as u16 + 0, ny);
    // 【wrap 判式勘误（goal-轮9 asm :1087-1088 逐字：`ldy Ens.0.y/cpy #SCRN_VY-1($EF)`）
    // 】旧形 `ny==−radiusY` 偏离（wrap 触发点错位）——asm=**当前 y==$EF**（屏底名表换）。
    if y == 0xEF {
        // 【goal-轮110 asm 合形修】wrap 回滚（:1125 `dec Ens.0.y`——旧形缺回滚 ✗）。
        mem.write_ram(0x0400 + x as u16 + 0, y); // 回滚（asm dec）
        mem.write_ram(0x00, 1); // $00=1（离屏/wrap）
        return false;
    }
    // 【2026-10-31 jD 补装】asm $849D @success：`lda data05/bmi @endIf_D`（bit7 清）⇒
    // **dec jumpDsplcmnt**（垂直位移记账：UP=$8441 inc/DOWN=dec——下行爬虫每成功步 −1，
    // 含外角探针的下移步；emu 实录 00→FF(f814 首步)→FE→…；rust 旧形缺写⇒恒 00）。
    if mem.read_ram(0x0400 + x as u16 + 5) & 0x80 == 0 {
        let j = mem.read_wram(0x6AF4 + x as u16 + 13); // jumpDsplcmnt=+13
        mem.write_wram(0x6AF4 + x as u16 + 13, j.wrapping_sub(1));
    }
    true
}

/// EnemyMoveOnePixelLeft（**asm 内容锚** `: 1214 $84A7`——r26（x dec 同构））。
/// 【goal-轮10 asm :1142-1210 逐字】①8px 门检 ②x==0⇒名表换（横滚房 ScrollX 面）
/// ③dec x ④左界==0（x==radiusX）⇒**失败+回滚**（inc x——旧形缺回滚！）⑤@success：
/// jumpDsplcmnt inc（data05 bit7 置时）。
pub(crate) fn enemy_move_one_pixel_left(mem: &mut crate::frame::FrameMemory, x: u8) -> bool {
    let xpos = mem.read_ram(0x0400 + x as u16 + 1); // x=+1
    let rx = mem.read_wram(0x6AF4 + x as u16 + 2); // radiusX=+2
    if xpos.wrapping_sub(rx) & 0x07 == 0 {
        if !enemy_check_move_left(mem, x) {
            mem.write_ram(0x00, 0); // $00=0（碰撞）
            return false;
        }
    }
    mem.write_ram(0x00, 1); // $00=1（移动进行——inc $00（:1161））
    // x==0⇒名表换（:1162-1180——横滚房 ScrollX/GetOtherNameTableIndex 族简化=hi 翻）
    if xpos == 0 {
        let hi = mem.read_wram(0x6AF4 + x as u16 + 7);
        mem.write_wram(0x6AF4 + x as u16 + 7, hi ^ 1); // SwitchEnemyNameTable（简化形）
    }
    let nx = xpos.wrapping_sub(1);
    mem.write_ram(0x0400 + x as u16 + 1, nx);
    // 左界==0（dec 后 x==radiusX）⇒失败+**回滚**（inc x——:1184-1199 逐字）
    if nx == rx {
        mem.write_ram(0x0400 + x as u16 + 1, xpos); // inc x（回滚）
        mem.write_ram(0x00, 0);
        return false;
    }
    // @success（:1201-1208）：data05 bit7 置⇒jumpDsplcmnt inc
    if mem.read_ram(0x0400 + x as u16 + 5) & 0x80 != 0 {
        let j = mem.read_wram(0x6AF4 + x as u16 + 13);
        mem.write_wram(0x6AF4 + x as u16 + 13, j.wrapping_add(1));
    }
    true
}

/// EnemyMoveOnePixelRight（**asm 内容锚** `: 1214 $84FE`——r26（x inc 同构））。
/// 【goal-轮10】右界判（x+radiusX==$FF?——asm :1230-1245 同构回滚）+wrap（x==0 环绕）。
pub(crate) fn enemy_move_one_pixel_right(mem: &mut crate::frame::FrameMemory, x: u8) -> bool {
    let xpos = mem.read_ram(0x0400 + x as u16 + 1);
    let rx = mem.read_wram(0x6AF4 + x as u16 + 2);
    if xpos.wrapping_add(rx) & 0x07 == 0 {
        if !enemy_check_move_right(mem, x) {
            mem.write_ram(0x00, 0); // $00=0（碰撞）
            // 【goal-轮18 右移块碰直证】碰撞返回。
            return false;
        }
    }
    mem.write_ram(0x00, 1);
    let nx = xpos.wrapping_add(1);
    mem.write_ram(0x0400 + x as u16 + 1, nx);
    if nx == 0 {
        // wrap（x 环绕 00⇒名表换——ToggleSamusHi 同族）
        mem.write_ram(0x00, 1);
        let hi = mem.read_wram(0x6AF4 + x as u16 + 7);
        mem.write_wram(0x6AF4 + x as u16 + 7, hi ^ 1);
    }
    // 右界判（x+radiusX 界——asm :1230-1245 回滚同构）
    if nx.wrapping_add(rx) == 0 {
        mem.write_ram(0x0400 + x as u16 + 1, xpos); // 回滚
        mem.write_ram(0x00, 0);
        return false;
    }
    if mem.read_ram(0x0400 + x as u16 + 5) & 0x80 != 0 {
        let j = mem.read_wram(0x6AF4 + x as u16 + 13);
        mem.write_wram(0x6AF4 + x as u16 + 13, j.wrapping_add(1));
    }
    true
}

/// MakeRoomRAMPtr（**asm 内容锚** `: 07:E96A`——r26）：位置→RoomRAM 瓦址——
/// ptr=0x6000|名表页|((y&0xF8)<<2)|(x>>3)（y/8×32+x/8 形！）。
/// 【goal-轮15 页面径回正（asm E96A:8425-8434 逐字：`lda Temp0B_PositionHi/and #$04/
/// ora ptr+1`——**对象 hi 选页**）】轮5 的 RoomRAMPtr(\$38) 选页=偏离（对象住其房间页
/// ——hi==房页不变量（GetNameTableAtScrollDir==SelectRoomRAM 同式）；金腿实证：爬虫
/// hi=0×\$6000（首房固实瓦挡探 ✓）而 \$38=\$6400（现房空 FF）⇒我探径滑过=旋转根）。
pub(crate) fn make_room_ram_ptr(_mem: &mut crate::frame::FrameMemory, y: u8, x: u8, hi: u8) -> u16 {
    let idx = (((y & 0xF8) as u16) << 2) | ((x >> 3) as u16);
    let page = if hi & 1 != 0 { 0x0400u16 } else { 0x0000 };
    0x6000 | page | (idx & 0x3FF)
}

/// 碰撞核（**asm 内容锚** `CheckMoveVertical: ; 07:E7BD`/Horizontal $E89B——r26）：
/// 逐点（块界）→MakeRoomRAMPtr→tile 值类：**<$80=固实（碰撞！）**/$4E=固实空（门/像）/
/// $80-$9F=可炸/≥$A0=可走。返回 carry（可走=true）。
pub(crate) fn collision_point_ok(mem: &mut crate::frame::FrameMemory, y: u8, x: u8, hi: u8) -> bool {
    let ptr = make_room_ram_ptr(mem, y, x, hi);
    let tile = mem.read_wram(ptr);
    if tile == 0x4E {
        return false; // 固实空（门/像——碰撞！）
    }
    if tile < 0x80 {
        return false; // 固实 tile（< $80）——碰撞！
    }
    // $80-$9F=可炸（可走性=IsBlastTile 判——简化径：视为可走）
    // ≥$A0=可走（$A0/$A1=门 tile（Samus 专用判——随件））
    true
}

/// EnemyCheckMoveUp（**asm 内容锚** `: 07:E770`→EnemyCheckMoveVertical `07:E783`→
/// CheckMoveVertical **x 全距走环**——r9 转向族考证修（crawler-corner-divergence.md））：
/// staging=enemy::movement（distY=ry+8 忠实），探行=y−distY（进入块上块），
/// x−rx 起 n 列全距（GetNumBlocksToCheck 公式=vertical_span_cols 同构已验证）；
/// rx==0⇒免检（asm `bne Lx198/sec/rts`）。旧形单点 (y−ry+8, x) 双错：探点 +8 落出
/// 进入块+漏跨块列——爬虫 radius 跨块时转角决策分歧根因。
pub(crate) fn enemy_check_move_up(mem: &mut crate::frame::FrameMemory, x: u8) -> bool {
    let y = mem.read_ram(0x0400 + x as u16 + 0);
    let ry = mem.read_wram(0x6AF4 + x as u16 + 1);
    let xpos = mem.read_ram(0x0400 + x as u16 + 1);
    let rx = mem.read_wram(0x6AF4 + x as u16 + 2); // radiusX=+2
    let hi = mem.read_wram(0x6AF4 + x as u16 + 7); // hi=+7
    let st = crate::enemy::movement::enemy_check_move_up(rx, ry, xpos, y, hi);
    match crate::enemy::movement::check_move_boundary(st, false) {
        crate::enemy::movement::MoveCheckOutcome::NoCollision => true,
        crate::enemy::movement::MoveCheckOutcome::NeedsBgCollision(st) => {
            let row = st.pos_y.wrapping_sub(st.dist_to_center_y);
            let (left, n) = crate::samus::physics::vertical_span_cols(st.pos_x, st.radius_x);
            // 【goal-轮11 块判结果直证】逐列 collision_point_ok 结果。
            for i in 0..n {
                let col = left.wrapping_add((i as u8).wrapping_mul(8));
                if !collision_point_ok(mem, row, col, st.pos_hi) {
                    return false;
                }
            }
            true
        }
    }
}

/// EnemyCheckMoveDown（**asm 内容锚** `: 07:E77B`——r26）：底块判。
/// EnemyCheckMoveDown（**asm 内容锚** `: 07:E77B`→同上 x 全距走环——r9 转向族修）：
/// staging distY=0−ry（两位补码）⇒探行=y−(−ry)=y+ry（进入块下块）。
pub(crate) fn enemy_check_move_down(mem: &mut crate::frame::FrameMemory, x: u8) -> bool {
    let y = mem.read_ram(0x0400 + x as u16 + 0);
    let ry = mem.read_wram(0x6AF4 + x as u16 + 1);
    let xpos = mem.read_ram(0x0400 + x as u16 + 1);
    let rx = mem.read_wram(0x6AF4 + x as u16 + 2);
    let hi = mem.read_wram(0x6AF4 + x as u16 + 7);
    let st = crate::enemy::movement::enemy_check_move_down(rx, ry, xpos, y, hi);
    // 【goal-轮6 块判直证】敌B 穿地源：块判输入/输出。
    match crate::enemy::movement::check_move_boundary(st, false) {
        crate::enemy::movement::MoveCheckOutcome::NoCollision => true,
        crate::enemy::movement::MoveCheckOutcome::NeedsBgCollision(st) => {
            let row = st.pos_y.wrapping_sub(st.dist_to_center_y);
            let (left, n) = crate::samus::physics::vertical_span_cols(st.pos_x, st.radius_x);
            // 【goal-轮11 块判结果直证】逐列 collision_point_ok 结果。
            for i in 0..n {
                let col = left.wrapping_add((i as u8).wrapping_mul(8));
                if !collision_point_ok(mem, row, col, st.pos_hi) {
                    return false;
                }
            }
            true
        }
    }
}

/// EnemyCheckMoveLeft（**asm 内容锚** `: 07:E8F1`——r26）：左块判。
/// EnemyCheckMoveLeft（**asm 内容锚** `: 07:E8F1`→CheckMoveHorizontal **y 全距走环**
/// （07:E89B）——r9 转向族修）：staging distX=rx+8⇒探列=x−(rx+8)（进入块左块），
/// y−ry 起 n 行全距（horizontal_span_rows 同构）；ry==0⇒免检。
pub(crate) fn enemy_check_move_left(mem: &mut crate::frame::FrameMemory, x: u8) -> bool {
    let y = mem.read_ram(0x0400 + x as u16 + 0);
    let xpos = mem.read_ram(0x0400 + x as u16 + 1);
    let rx = mem.read_wram(0x6AF4 + x as u16 + 2); // radiusX=+2
    let ry = mem.read_wram(0x6AF4 + x as u16 + 1);
    let hi = mem.read_wram(0x6AF4 + x as u16 + 7);
    let st = crate::enemy::movement::enemy_check_move_left(rx, ry, xpos, y, hi);
    match crate::enemy::movement::check_move_boundary(st, true) {
        crate::enemy::movement::MoveCheckOutcome::NoCollision => true,
        crate::enemy::movement::MoveCheckOutcome::NeedsBgCollision(st) => {
            let col = st.pos_x.wrapping_sub(st.dist_to_center_x);
            let (top, n) = crate::samus::physics::horizontal_span_rows(st.pos_y, st.radius_y);
            for i in 0..n {
                let row = top.wrapping_add((i as u8).wrapping_mul(8));
                if !collision_point_ok(mem, row, col, st.pos_hi) {
                    return false;
                }
            }
            true
        }
    }
}

/// EnemyCheckMoveRight（**asm 内容锚** `: 07:E8FC`——r26）：右块判。
/// EnemyCheckMoveRight（**asm 内容锚** `: 07:E8FC`→CheckMoveHorizontal y 全距走环
/// ——r9 转向族修）：staging distX=0−rx⇒探列=x+rx（进入块右块），y−ry 起 n 行全距。
pub(crate) fn enemy_check_move_right(mem: &mut crate::frame::FrameMemory, x: u8) -> bool {
    let y = mem.read_ram(0x0400 + x as u16 + 0);
    let xpos = mem.read_ram(0x0400 + x as u16 + 1);
    let rx = mem.read_wram(0x6AF4 + x as u16 + 2);
    let ry = mem.read_wram(0x6AF4 + x as u16 + 1);
    let hi = mem.read_wram(0x6AF4 + x as u16 + 7);
    let st = crate::enemy::movement::enemy_check_move_right(rx, ry, xpos, y, hi);
    match crate::enemy::movement::check_move_boundary(st, true) {
        crate::enemy::movement::MoveCheckOutcome::NoCollision => true,
        crate::enemy::movement::MoveCheckOutcome::NeedsBgCollision(st) => {
            let col = st.pos_x.wrapping_sub(st.dist_to_center_x);
            let (top, n) = crate::samus::physics::horizontal_span_rows(st.pos_y, st.radius_y);
            for i in 0..n {
                let row = top.wrapping_add((i as u8).wrapping_mul(8));
                if !collision_point_ok(mem, row, col, st.pos_hi) {
                    return false;
                }
            }
            true
        }
    }
}

