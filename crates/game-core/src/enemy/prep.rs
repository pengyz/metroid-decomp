//! 敌人每槽预备件与位工具（原 `prims.rs`，语义分层）。
//!
//! 由 `frame/enemy/{update,move_shell}.rs` 原样搬出（纯搬移，语义分层 P1）。
//! asm 根：`LoadTableAt977B`、`EnemyCheckIfVisible`、`EnemyForceSpeedTowardsSamus ; 07:F6DF`、
//! `EnData05DistanceToSamusThreshold ; 07:F75B`、`InitEnRestingAnimIndex`。

use crate::frame::tick::*;
use crate::frame::Manager;
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
pub(crate) fn enemy_check_if_visible_inner(mem: &mut crate::frame::Manager, x: u8) -> bool {
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

pub(crate) fn enemy_force_speed_x_axis(mem: &mut crate::frame::Manager, x: u8, attribs: u8) {
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
pub(crate) fn enemy_facing_towards_samus_bit(mem: &mut crate::frame::Manager, x: u8) -> u8 {
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

pub(crate) fn enemy_force_speed(mem: &mut crate::frame::Manager, x: u8) {
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
pub(crate) fn enemy_distance_threshold(mem: &mut crate::frame::Manager, x: u8) {
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
pub(crate) fn init_en_resting_anim_index(mem: &mut crate::frame::Manager, x: u8) {
    use crate::enemy::anim_idx::AnimIdx;
    let y = enemy_type_times2_plus_facing(mem, x);
    let area = crate::frame::cur_area(mem);
    let idx = AnimIdx::at(area, game_data::assets::enemy_resting_anim(area)[y as usize & 0x1F]);
    let slot = crate::enemy::state::slot_of(x);
    if idx.off() == mem.enemy(slot).reset_anim_index() {
        // asm `beq ClearEnAnimDelay@RTS` —— 相等时**仍清 animDelay**（旧实现直接 return，漏了这一步）
        mem.enemy(slot).set_anim_delay(0);
        return;
    }
    mem.enemy(slot).set_reset_anim_index(idx.off()); // resetAnimIndex
    mem.enemy(slot).set_anim_index(idx.off()); // animIndex
    mem.enemy(slot).set_anim_delay(0); // animDelay←0
}

/// `XorEnData05 ; $856B`。
pub(crate) fn xor_en_data05(mem: &mut crate::frame::Manager, x: u8, mask: u8) {
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


