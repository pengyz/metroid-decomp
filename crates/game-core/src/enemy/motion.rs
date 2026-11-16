//! 通用移动壳：`EnemyMove($8058)` + 四个失败处理器 + `UpdateEnemyCommon(07:F410)` 三入口。
//!
//! asm 依据：areas_common.asm:112-178（本体）、:400-560（失败处理器/bounce/resting）、
//! prg7_engine.asm:10628（UpdateEnemyCommon）、prg1_brinstar.asm:863（_Decide，每区一份）。
//!
//! 本文件由 `frame/mainloop.rs` 原样搬出（纯搬移，逻辑/注释/测试均未改动）。

use crate::enemy::prep::*;
use crate::enemy::slot_cycle::{
    EN_DELAY, EN_INSTR, EN_MOV_IDX, EN_SUBPIXEL_X, EN_SUBPIXEL_Y,
};







/// `EnemyTriggerResting ; $81B8`：data05 bit5 置（A 恒 0）。
fn enemy_trigger_resting(mem: &mut crate::frame::Manager, x: u8) {
    let a = en_addr(x, EN_F_DATA05);
    mem.write_ram(a, mem.read_ram(a) | 0x20);
}

/// `EnemyTriggerResting_AndClearEnAccelY ; $81B1`。
fn enemy_trigger_resting_clear_accel_y(mem: &mut crate::frame::Manager, x: u8) {
    enemy_trigger_resting(mem, x);
    mem.write_wram(ens_addr(x, ENS_F_ACCEL_Y), 0);
}

/// `EnemyTriggerResting_AndClearEnAccelX ; $81C0`。
fn enemy_trigger_resting_clear_accel_x(mem: &mut crate::frame::Manager, x: u8) {
    enemy_trigger_resting(mem, x);
    mem.write_wram(ens_addr(x, ENS_F_ACCEL_X), 0);
}



/// `EnemyIfMoveFailedVertical_Bounce($81FC)` / `_Horizontal_Bounce($81C7)`。
fn enemy_if_move_failed_bounce(mem: &mut crate::frame::Manager, x: u8, vertical: bool) {
    // 1. `LoadBit5ofTableAt968B` 置 ⇒ 该类型不反弹。
    if l968b_of(mem, x) & 0x20 != 0 {
        return;
    }
    // 2. 翻该轴朝向。
    xor_en_data05(mem, x, if vertical { 0x04 } else { 0x01 });
    // 3. `@flipSpeedAndAccel`：accel 取负。
    let accel_f = if vertical { ENS_F_ACCEL_Y } else { ENS_F_ACCEL_X };
    let a = ens_addr(x, accel_f);
    mem.write_wram(a, twos(mem.read_wram(a)));
    // 4. `@flipSpeed` 再查一次 bit5（asm 原样，不是笔误）。
    if l968b_of(mem, x) & 0x20 != 0 {
        return;
    }
    // 5. 加速类先翻子像素；串类只翻速度。
    let uses_accel = l977b_raw(mem, x) & 0x40 != 0;
    let (sub_f, spd_f) = if vertical {
        (EN_SUBPIXEL_Y, EN_F_SPEED_Y)
    } else {
        (EN_SUBPIXEL_X, EN_F_SPEED_X)
    };
    if uses_accel {
        let sp = mem.read_ram(en_addr(x, sub_f));
        mem.write_ram(en_addr(x, sub_f), twos(sp));
    }
    let s = mem.read_ram(en_addr(x, spd_f));
    mem.write_ram(en_addr(x, spd_f), twos(s));
}

/// 竖直失败（`EnemyIfMoveFailedUp $80B8` / `_Down $80FB`）。
///
/// 注意 asm 的不对称（`$8110` 注释自问「不该与 Up 对称吗」）：`data1F=$40` 时
/// **Up 是「减半且向上」、Down 是「触发休止后 fallthrough 到 movementStrings」**，
/// 后者不清 speedY。这里按 ROM 保真（D4）。
fn enemy_if_move_failed_vertical(mem: &mut crate::frame::Manager, x: u8, up: bool) {
    if mem.read_ram(en_addr(x, EN_F_DATA05)) & 0x80 != 0 {
        enemy_if_move_failed_bounce(mem, x, true);
        return;
    }
    let raw = l977b_raw(mem, x);
    if raw & 0x40 == 0 {
        // movement-string 类：bit1 置才翻竖直朝向。
        if raw & 0x02 != 0 {
            xor_en_data05(mem, x, 0x04);
        }
        return;
    }
    let d1f = mem.read_wram(ens_addr(x, ENS_F_DATA1F));
    if d1f == 0 {
        enemy_if_move_failed_bounce(mem, x, true);
        return;
    }
    if d1f & 0x80 == 0 {
        if up {
            let (hi, lo) = ror16(
                mem.read_ram(en_addr(x, EN_F_SPEED_Y)),
                mem.read_ram(en_addr(x, EN_SUBPIXEL_Y)),
                true,
            );
            mem.write_ram(en_addr(x, EN_F_SPEED_Y), hi);
            mem.write_ram(en_addr(x, EN_SUBPIXEL_Y), lo);
        } else {
            enemy_trigger_resting_clear_accel_y(mem, x);
            let raw2 = l977b_raw(mem, x);
            if raw2 & 0x02 != 0 {
                xor_en_data05(mem, x, 0x04);
            }
        }
        return;
    }
    if up {
        enemy_trigger_resting_clear_accel_y(mem, x);
        mem.write_ram(en_addr(x, EN_F_SPEED_Y), 0);
        mem.write_ram(en_addr(x, EN_SUBPIXEL_Y), 0);
    } else {
        let (hi, lo) = ror16(
            mem.read_ram(en_addr(x, EN_F_SPEED_Y)),
            mem.read_ram(en_addr(x, EN_SUBPIXEL_Y)),
            false,
        );
        mem.write_ram(en_addr(x, EN_F_SPEED_Y), hi);
        mem.write_ram(en_addr(x, EN_SUBPIXEL_Y), lo);
    }
}

/// 水平失败（`EnemyIfMoveFailedRight $8134` / `_Left $816E`）。
///
/// 两侧都是**先查 `L977B` bit6**（加速/串类）：串类翻 `data05` bit0（`L977B` bit0 置才翻）；
/// 加速类再看 `data05` bit7（竖直朝向）与 `data1F`。
fn enemy_if_move_failed_horizontal(mem: &mut crate::frame::Manager, x: u8, right: bool) {
    let raw = l977b_raw(mem, x);
    if raw & 0x40 == 0 {
        if raw & 0x01 != 0 {
            xor_en_data05(mem, x, 0x01);
        }
        return;
    }
    if mem.read_ram(en_addr(x, EN_F_DATA05)) & 0x80 == 0 {
        enemy_if_move_failed_bounce(mem, x, false);
        return;
    }
    let d1f = mem.read_wram(ens_addr(x, ENS_F_DATA1F));
    if d1f == 0 {
        enemy_if_move_failed_bounce(mem, x, false);
        return;
    }
    if d1f & 0x80 == 0 {
        if right {
            enemy_trigger_resting_clear_accel_x(mem, x);
        } else {
            let (hi, lo) = ror16(
                mem.read_ram(en_addr(x, EN_F_SPEED_X)),
                mem.read_ram(en_addr(x, EN_SUBPIXEL_X)),
                true,
            );
            mem.write_ram(en_addr(x, EN_F_SPEED_X), hi);
            mem.write_ram(en_addr(x, EN_SUBPIXEL_X), lo);
        }
        return;
    }
    if right {
        let (hi, lo) = ror16(
            mem.read_ram(en_addr(x, EN_F_SPEED_X)),
            mem.read_ram(en_addr(x, EN_SUBPIXEL_X)),
            false,
        );
        mem.write_ram(en_addr(x, EN_F_SPEED_X), hi);
        mem.write_ram(en_addr(x, EN_SUBPIXEL_X), lo);
    } else {
        enemy_trigger_resting_clear_accel_x(mem, x);
        mem.write_ram(en_addr(x, EN_F_SPEED_X), 0);
        mem.write_ram(en_addr(x, EN_SUBPIXEL_X), 0);
    }
}

/// 取本帧 Δ（`EnemyGetDeltaY` + `EnemyGetDeltaX` 共用一次串步进——`$8318` 明确
/// 「Read the same sign-magnitude speed vector byte as in EnemyGetDeltaY」）。
fn enemy_delta_for_move(mem: &mut crate::frame::Manager, x: u8) -> (i8, i8) {
    let type_ = mem.read_wram(ens_addr(x, ENS_F_TYPE)) & 0x0F;
    let d05 = mem.read_ram(en_addr(x, EN_F_DATA05));
    let mov_idx = mem.read_ram(en_addr(x, EN_MOV_IDX));
    let instr = mem.read_ram(en_addr(x, EN_INSTR));
    let delay = mem.read_ram(en_addr(x, EN_DELAY));
    let sub_y = mem.read_ram(en_addr(x, EN_SUBPIXEL_Y));
    let speed_y = mem.read_ram(en_addr(x, EN_F_SPEED_Y));
    let accel_y = mem.read_wram(ens_addr(x, ENS_F_ACCEL_Y));
    let d1f = mem.read_wram(ens_addr(x, ENS_F_DATA1F));
    let can_up = enemy_check_move_up(mem, x);
    let can_down = enemy_check_move_down(mem, x);
    let (dy, dx, ndelay, ninstr, nd1f, trig) = crate::enemy::common::enemy_get_delta_xy(
        crate::frame::cur_area(mem),
        type_,
        d05,
        mov_idx,
        instr,
        delay,
        sub_y,
        speed_y,
        accel_y,
        d1f,
        can_up,
        can_down,
    );
    mem.write_wram(ens_addr(x, ENS_F_DATA1F), nd1f);
    mem.write_ram(en_addr(x, EN_DELAY), ndelay);
    mem.write_ram(en_addr(x, EN_INSTR), ninstr);
    if trig {
        // `EnemyGetDeltaY@fail ; $82FB`（areas_common.asm:773-797）：`L968B[type]` bit5 置 ⇒
        // `data05 = ((data05 ^ $05) | L968B) & $1F`（清 bits6-7；bit5 随后由 TriggerResting 置）。
        let t = mem.read_wram(ens_addr(x, ENS_F_TYPE)) & 0x0F;
        let l968b = game_data::assets::enemy_force_attribs(crate::frame::cur_area(mem))
            .get(t as usize)
            .copied()
            .unwrap_or(0);
        if l968b & 0x20 != 0 {
            let d5 = mem.read_ram(en_addr(x, EN_F_DATA05));
            mem.write_ram(en_addr(x, EN_F_DATA05), ((d5 ^ 0x05) | l968b) & 0x1F);
        }
        // `EnemyTriggerResting_AndClearEnAccelY ; $81B1`：data05 bit5 置 + accelY←0。
        let d5 = mem.read_ram(en_addr(x, EN_F_DATA05));
        mem.write_ram(en_addr(x, EN_F_DATA05), d5 | 0x20);
        mem.write_wram(ens_addr(x, ENS_F_ACCEL_Y), 0);
    }
    (dy, dx)
}

pub(crate) fn apply_enemy_delta_axis(mem: &mut crate::frame::Manager, x: u8, delta: i8, vertical: bool) {
    let mut qty = if delta < 0 {
        twos(delta as u8)
    } else {
        delta as u8
    };
    while qty != 0 {
        let ok = match (vertical, delta < 0) {
            (true, true) => enemy_move_one_pixel_up(mem, x),
            (true, false) => enemy_move_one_pixel_down(mem, x),
            (false, true) => enemy_move_one_pixel_left(mem, x),
            (false, false) => enemy_move_one_pixel_right(mem, x),
        };
        if !ok {
            if vertical {
                enemy_if_move_failed_vertical(mem, x, delta < 0);
            } else {
                enemy_if_move_failed_horizontal(mem, x, delta > 0);
            }
            // asm 失败径：`EnemyMovePixelQty←1`，随后的 `dec` 让本帧立即出环。
            break;
        }
        qty = qty.wrapping_sub(1);
    }
}

/// `EnemyMove ; $8058`。
pub(crate) fn enemy_move(mem: &mut crate::frame::Manager, x: u8) {
    // 门 1：`lda data05 / asl / bmi @RTS` ⇒ bit6 置（30FPS 半速帧）整件跳过。
    if mem.read_ram(en_addr(x, EN_F_DATA05)) & 0x40 != 0 {
        return;
    }
    // 门 2：只有 Active 状态移动。
    if mem.read_wram(ens_addr(x, ENS_F_STATUS)) != 0x02 {
        return;
    }
    let (dy, dx) = enemy_delta_for_move(mem, x);
    apply_enemy_delta_axis(mem, x, dy, true);
    apply_enemy_delta_axis(mem, x, dx, false);
}




#[cfg(test)]
mod tests_enemy_move_shell {
    use super::*;
    use crate::frame::Manager;

    fn stage(m: &mut Manager, x: u8, type_: u8, d05: u8, status: u8) {
        m.write_ram(en_addr(x, 0), 0x40); // y（非块界，避免碰撞判）
        m.write_ram(en_addr(x, 1), 0x40); // x
        m.write_ram(en_addr(x, EN_F_DATA05), d05);
        m.write_wram(ens_addr(x, ENS_F_STATUS), status);
        m.write_wram(ens_addr(x, ENS_F_TYPE), type_);
        m.write_ram(en_addr(x, 6), 0x40); // radiusY（让 (y-ry)&7 != 0）
        m.write_ram(en_addr(x, 7), 0x40); // radiusX
    }

    /// 在 0..16 里找一个满足位条件的敌类型（用真实 ROM 表，不硬编码假设）。
    fn find_type(m: &mut Manager, pred: impl Fn(u8) -> bool) -> Option<u8> {
        let saved = m.read_wram(ens_addr(0x10, ENS_F_TYPE));
        for t in 0u8..16 {
            m.write_wram(ens_addr(0x10, ENS_F_TYPE), t);
            if pred(l977b_raw(m, 0x10)) {
                m.write_wram(ens_addr(0x10, ENS_F_TYPE), saved);
                return Some(t);
            }
        }
        m.write_wram(ens_addr(0x10, ENS_F_TYPE), saved);
        None
    }

    #[test]
    fn enemy_move_skips_when_data05_bit6_set() {
        let mut m = Manager::default();
        stage(&mut m, 0x10, 4, 0x40, 0x02);
        m.write_ram(en_addr(0x10, 0), 0x50);
        enemy_move(&mut m, 0x10);
        assert_eq!(m.read_ram(en_addr(0x10, 0)), 0x50, "bit6 置 ⇒ 整件跳过");
    }

    #[test]
    fn enemy_move_skips_when_not_active() {
        let mut m = Manager::default();
        stage(&mut m, 0x10, 4, 0x00, 0x01);
        m.write_ram(en_addr(0x10, 0), 0x50);
        enemy_move(&mut m, 0x10);
        assert_eq!(m.read_ram(en_addr(0x10, 0)), 0x50, "非 Active 不移动");
    }

    /// `data1F=$40` + 竖直朝向 + 加速类 ⇒ Up 径：16 位 `sec/ror` 减半且向上。
    #[test]
    fn failed_up_with_data1f_40_halves_speed_upward() {
        let mut m = Manager::default();
        // data05 bit7=0（水平朝向）才走 @facingHorizontal 的 data1F 分叉
        stage(&mut m, 0x10, 0, 0x00, 0x02);
        let Some(t) = find_type(&mut m, |r| r & 0x40 != 0) else {
            panic!("本区 L977B 没有加速类类型，测试前提不成立");
        };
        m.write_wram(ens_addr(0x10, ENS_F_TYPE), t);
        m.write_wram(ens_addr(0x10, ENS_F_DATA1F), 0x40);
        m.write_ram(en_addr(0x10, EN_F_SPEED_Y), 0x08);
        m.write_ram(en_addr(0x10, EN_SUBPIXEL_Y), 0x00);
        enemy_if_move_failed_vertical(&mut m, 0x10, true);
        assert_eq!(m.read_ram(en_addr(0x10, EN_F_SPEED_Y)), 0x84);
        assert_eq!(m.read_ram(en_addr(0x10, EN_SUBPIXEL_Y)), 0x00);
    }

    /// `data1F=$40` + Down 径 ⇒ 触发休止（bit5 置、accelY 清零），且**不清 speedY**（asm 不对称）。
    #[test]
    fn failed_down_with_data1f_40_triggers_resting() {
        let mut m = Manager::default();
        stage(&mut m, 0x10, 0, 0x00, 0x02);
        let Some(t) = find_type(&mut m, |r| r & 0x40 != 0) else {
            panic!("本区 L977B 没有加速类类型，测试前提不成立");
        };
        m.write_wram(ens_addr(0x10, ENS_F_TYPE), t);
        m.write_wram(ens_addr(0x10, ENS_F_DATA1F), 0x40);
        m.write_wram(ens_addr(0x10, ENS_F_ACCEL_Y), 0x18);
        m.write_ram(en_addr(0x10, EN_F_SPEED_Y), 0x07);
        enemy_if_move_failed_vertical(&mut m, 0x10, false);
        assert_eq!(m.read_ram(en_addr(0x10, EN_F_DATA05)) & 0x20, 0x20);
        assert_eq!(m.read_wram(ens_addr(0x10, ENS_F_ACCEL_Y)), 0);
        assert_eq!(m.read_ram(en_addr(0x10, EN_F_SPEED_Y)), 0x07);
    }

    /// 串类 + `L977B` bit0 置 ⇒ 水平失败翻 `data05` bit0。
    #[test]
    fn failed_horizontal_string_class_flips_bit0_when_table_bit0_set() {
        let mut m = Manager::default();
        stage(&mut m, 0x10, 0, 0x00, 0x02);
        let Some(t) = find_type(&mut m, |r| r & 0x40 == 0 && r & 0x01 != 0) else {
            panic!("本区 L977B 没有 (bit6=0,bit0=1) 类型，测试前提不成立");
        };
        m.write_wram(ens_addr(0x10, ENS_F_TYPE), t);
        enemy_if_move_failed_horizontal(&mut m, 0x10, true);
        assert_eq!(m.read_ram(en_addr(0x10, EN_F_DATA05)) & 0x01, 0x01);
    }

    /// 串类 + `L977B` bit1 清 ⇒ 竖直失败**不翻**竖直朝向。
    #[test]
    fn failed_vertical_string_class_no_flip_when_table_bit1_clear() {
        let mut m = Manager::default();
        stage(&mut m, 0x10, 0, 0x00, 0x02);
        let Some(t) = find_type(&mut m, |r| r & 0x40 == 0 && r & 0x02 == 0) else {
            panic!("本区 L977B 没有 (bit6=0,bit1=0) 类型，测试前提不成立");
        };
        m.write_wram(ens_addr(0x10, ENS_F_TYPE), t);
        enemy_if_move_failed_vertical(&mut m, 0x10, true);
        assert_eq!(m.read_ram(en_addr(0x10, EN_F_DATA05)) & 0x04, 0x00);
    }

    /// bounce 本体：`L968B` bit5 清的类型 ⇒ 翻朝向 + 速度取负。
    ///
    /// 本区 `L968B` 16 项里没有 bit5 置位者（ROM 实测），因此「置位则不反弹」这一支
    /// 在本区**不可观测**；这里只钉可观测的一半，另一半留待有该属性的区域用例。
    #[test]
    fn bounce_flips_facing_and_negates_speed() {
        let mut m = Manager::default();
        stage(&mut m, 0x10, 0, 0x00, 0x02);
        let area = crate::frame::cur_area(&m);
        let attrs = game_data::assets::enemy_force_attribs(area);
        let t = (0u8..16)
            .find(|i| attrs.get(*i as usize).is_some_and(|v| v & 0x20 == 0))
            .expect("本区应有 bit5 清的类型");
        m.write_wram(ens_addr(0x10, ENS_F_TYPE), t);
        m.write_ram(en_addr(0x10, EN_F_SPEED_Y), 0x09);
        enemy_if_move_failed_bounce(&mut m, 0x10, true);
        assert_eq!(m.read_ram(en_addr(0x10, EN_F_SPEED_Y)), 0xF7); // -9
        assert_eq!(m.read_ram(en_addr(0x10, EN_F_DATA05)) & 0x04, 0x04); // 翻竖直朝向
    }
}

// ---- 逐像素移动原子与碰撞点（原 `prims.rs`，纯搬移）----

/// EnemyMoveOnePixelUp（**asm 内容锚** `areas_common.asm:989 $83F5`——r26）：
/// 顶界块界判（(y-radiusY)&7==0⇒EnemyCheckMoveUp）→y dec（y==0 换名表族）→
/// radiusY 界判→成功（carry=1）/失败（carry=0）。返回 bool（carry）。
pub(crate) fn enemy_move_one_pixel_up(mem: &mut crate::frame::Manager, x: u8) -> bool {
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
pub(crate) fn enemy_move_one_pixel_down(mem: &mut crate::frame::Manager, x: u8) -> bool {
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
pub(crate) fn enemy_move_one_pixel_left(mem: &mut crate::frame::Manager, x: u8) -> bool {
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
pub(crate) fn enemy_move_one_pixel_right(mem: &mut crate::frame::Manager, x: u8) -> bool {
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
pub(crate) fn make_room_ram_ptr(_mem: &mut crate::frame::Manager, y: u8, x: u8, hi: u8) -> u16 {
    // 【2026-11-17 收口】换算只此一份：`Room::addr`。对象面的 `hi` 用 **bit0** 选页
    // （`Temp0B_PositionHi & $04` 的等价形），房间面用 **bit2**——这里显式换算，
    // 避免两套公式各自演化（此前两份实现是"同结果、不同写法"，改一处必漂移）。
    let nt_page = if hi & 1 != 0 { 0x04 } else { 0x00 };
    crate::world::room_ram::Room::addr(y, x, nt_page)
}

/// 碰撞核（**asm 内容锚** `CheckMoveVertical: ; 07:E7BD`/Horizontal $E89B——r26）：
/// 逐点（块界）→MakeRoomRAMPtr→tile 值类：**<$80=固实（碰撞！）**/$4E=固实空（门/像）/
/// $80-$9F=可炸/≥$A0=可走。返回 carry（可走=true）。
pub(crate) fn collision_point_ok(mem: &mut crate::frame::Manager, y: u8, x: u8, hi: u8) -> bool {
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
pub(crate) fn enemy_check_move_up(mem: &mut crate::frame::Manager, x: u8) -> bool {
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
pub(crate) fn enemy_check_move_down(mem: &mut crate::frame::Manager, x: u8) -> bool {
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
pub(crate) fn enemy_check_move_left(mem: &mut crate::frame::Manager, x: u8) -> bool {
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
pub(crate) fn enemy_check_move_right(mem: &mut crate::frame::Manager, x: u8) -> bool {
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
