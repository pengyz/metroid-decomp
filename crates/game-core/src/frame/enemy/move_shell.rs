//! 通用移动壳：`EnemyMove($8058)` + 四个失败处理器 + `UpdateEnemyCommon(07:F410)` 三入口。
//!
//! asm 依据：areas_common.asm:112-178（本体）、:400-560（失败处理器/bounce/resting）、
//! prg7_engine.asm:10628（UpdateEnemyCommon）、prg1_brinstar.asm:863（_Decide，每区一份）。
//!
//! 本文件由 `frame/mainloop.rs` 原样搬出（纯搬移，逻辑/注释/测试均未改动）。

use crate::frame::enemy::update::{
    load_table_at_977b, EN_DELAY, EN_INSTR, EN_MOV_IDX, EN_SUBPIXEL_X, EN_SUBPIXEL_Y,
};
use crate::frame::mainloop::{
    enemy_check_move_down, enemy_check_move_up, enemy_move_one_pixel_down,
    enemy_move_one_pixel_left, enemy_move_one_pixel_right, enemy_move_one_pixel_up,
    update_enemy_anim, update_enemy_common_no_move_no_anim,
};

const EN_BASE: u16 = 0x0400;
const ENS_BASE: u16 = 0x6AF4;
const EN_F_DATA05: u16 = 5;
const EN_F_SPEED_Y: u16 = 2;
const EN_F_SPEED_X: u16 = 3;
const ENS_F_STATUS: u16 = 0;
const ENS_F_ACCEL_Y: u16 = 10;
const ENS_F_ACCEL_X: u16 = 11;
const ENS_F_TYPE: u16 = 14;
const ENS_F_DATA1F: u16 = 15;

fn en_addr(x: u8, f: u16) -> u16 {
    EN_BASE + x as u16 + f
}
fn ens_addr(x: u8, f: u16) -> u16 {
    ENS_BASE + x as u16 + f
}

/// `L977B[type]` 的**原始字节**。
///
/// 注意：asm 的 `LoadTableAt977B` 返回 `A = L977B[y] << 1`（后面的 `bpl/bmi` 因此测的是
/// **原字节 bit6**）。这里给原始值，各调用点用 raw 位判据，避免再被 `asl` 绕一次。
fn l977b_raw(mem: &mut crate::frame::FrameMemory, x: u8) -> u8 {
    load_table_at_977b(mem, x) >> 1
}

/// `L968B[type]`（`ReadTableAt968B`）——区域相关属性表。
fn l968b_of(mem: &mut crate::frame::FrameMemory, x: u8) -> u8 {
    let t = (mem.read_wram(ens_addr(x, ENS_F_TYPE)) & 0x0F) as usize;
    game_data::assets::enemy_force_attribs(crate::frame::cur_area(mem))
        .get(t)
        .copied()
        .unwrap_or(0)
}

/// `XorEnData05 ; $856B`。
fn xor_en_data05(mem: &mut crate::frame::FrameMemory, x: u8, mask: u8) {
    let a = en_addr(x, EN_F_DATA05);
    mem.write_ram(a, mem.read_ram(a) ^ mask);
}

/// `EnemyTriggerResting ; $81B8`：data05 bit5 置（A 恒 0）。
fn enemy_trigger_resting(mem: &mut crate::frame::FrameMemory, x: u8) {
    let a = en_addr(x, EN_F_DATA05);
    mem.write_ram(a, mem.read_ram(a) | 0x20);
}

/// `EnemyTriggerResting_AndClearEnAccelY ; $81B1`。
fn enemy_trigger_resting_clear_accel_y(mem: &mut crate::frame::FrameMemory, x: u8) {
    enemy_trigger_resting(mem, x);
    mem.write_wram(ens_addr(x, ENS_F_ACCEL_Y), 0);
}

/// `EnemyTriggerResting_AndClearEnAccelX ; $81C0`。
fn enemy_trigger_resting_clear_accel_x(mem: &mut crate::frame::FrameMemory, x: u8) {
    enemy_trigger_resting(mem, x);
    mem.write_wram(ens_addr(x, ENS_F_ACCEL_X), 0);
}

/// `sec/clc + ror hi + ror lo`：16 位右移一位，进位补进 hi 的 bit7。
/// `carry_in=true` = asm 的 `sec`（减半并把方向钉成负），`false` = `clc`（减半保持正）。
fn ror16(hi: u8, lo: u8, carry_in: bool) -> (u8, u8) {
    let v = ((hi as u16) << 8) | lo as u16;
    let r = (v >> 1) | if carry_in { 0x8000 } else { 0 };
    ((r >> 8) as u8, (r & 0xFF) as u8)
}

/// `twos_complement`：`lda #$00 / sbc v` ⇒ `0 - v`。
fn twos(v: u8) -> u8 {
    v.wrapping_neg()
}

/// `EnemyIfMoveFailedVertical_Bounce($81FC)` / `_Horizontal_Bounce($81C7)`。
fn enemy_if_move_failed_bounce(mem: &mut crate::frame::FrameMemory, x: u8, vertical: bool) {
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
fn enemy_if_move_failed_vertical(mem: &mut crate::frame::FrameMemory, x: u8, up: bool) {
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
fn enemy_if_move_failed_horizontal(mem: &mut crate::frame::FrameMemory, x: u8, right: bool) {
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
fn enemy_delta_for_move(mem: &mut crate::frame::FrameMemory, x: u8) -> (i8, i8) {
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
    let (dy, dx, ndelay, ninstr, nd1f, _trig) = crate::enemy::common::enemy_get_delta_xy(
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
    (dy, dx)
}

fn apply_enemy_delta_axis(mem: &mut crate::frame::FrameMemory, x: u8, delta: i8, vertical: bool) {
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
fn enemy_move(mem: &mut crate::frame::FrameMemory, x: u8) {
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

/// `@noMove ; 07:F438`（入口 3：anim + 尾件，无移动）。
fn update_enemy_common_no_move_anim(mem: &mut crate::frame::FrameMemory, x: u8, anim: u8) {
    update_enemy_anim(mem, x, anim);
    update_enemy_common_no_move_no_anim(mem, x);
}

/// `UpdateEnemyCommon ; 07:F410`（入口 1：anim + move）。
fn update_enemy_common_anim_move(mem: &mut crate::frame::FrameMemory, x: u8, anim: u8) {
    update_enemy_anim(mem, x, anim);
    enemy_move(mem, x);
    update_enemy_common_no_move_no_anim(mem, x);
}

/// `UpdateEnemyCommon_Decide`（prg1_brinstar.asm:863）：按 `EnemyStatusPreAI` 选入口；
/// 活动态 anim 索引取 `$00`、休止态取 `$01`（均由调用者预设）。
fn update_enemy_common_decide(mem: &mut crate::frame::FrameMemory, x: u8) {
    match mem.read_ram(0x0081) {
        1 => {
            let anim = mem.read_ram(0x0001);
            update_enemy_common_no_move_anim(mem, x, anim);
        }
        3 => update_enemy_common_no_move_no_anim(mem, x),
        _ => {
            let anim = mem.read_ram(0x0000);
            update_enemy_common_anim_move(mem, x, anim);
        }
    }
}

#[cfg(test)]
mod tests_enemy_move_shell {
    use super::*;
    use crate::frame::FrameMemory;

    fn stage(m: &mut FrameMemory, x: u8, type_: u8, d05: u8, status: u8) {
        m.write_ram(en_addr(x, 0), 0x40); // y（非块界，避免碰撞判）
        m.write_ram(en_addr(x, 1), 0x40); // x
        m.write_ram(en_addr(x, EN_F_DATA05), d05);
        m.write_wram(ens_addr(x, ENS_F_STATUS), status);
        m.write_wram(ens_addr(x, ENS_F_TYPE), type_);
        m.write_ram(en_addr(x, 6), 0x40); // radiusY（让 (y-ry)&7 != 0）
        m.write_ram(en_addr(x, 7), 0x40); // radiusX
    }

    /// 在 0..16 里找一个满足位条件的敌类型（用真实 ROM 表，不硬编码假设）。
    fn find_type(m: &mut FrameMemory, pred: impl Fn(u8) -> bool) -> Option<u8> {
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
        let mut m = FrameMemory::default();
        stage(&mut m, 0x10, 4, 0x40, 0x02);
        m.write_ram(en_addr(0x10, 0), 0x50);
        enemy_move(&mut m, 0x10);
        assert_eq!(m.read_ram(en_addr(0x10, 0)), 0x50, "bit6 置 ⇒ 整件跳过");
    }

    #[test]
    fn enemy_move_skips_when_not_active() {
        let mut m = FrameMemory::default();
        stage(&mut m, 0x10, 4, 0x00, 0x01);
        m.write_ram(en_addr(0x10, 0), 0x50);
        enemy_move(&mut m, 0x10);
        assert_eq!(m.read_ram(en_addr(0x10, 0)), 0x50, "非 Active 不移动");
    }

    /// `data1F=$40` + 竖直朝向 + 加速类 ⇒ Up 径：16 位 `sec/ror` 减半且向上。
    #[test]
    fn failed_up_with_data1f_40_halves_speed_upward() {
        let mut m = FrameMemory::default();
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
        let mut m = FrameMemory::default();
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
        let mut m = FrameMemory::default();
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
        let mut m = FrameMemory::default();
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
        let mut m = FrameMemory::default();
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
