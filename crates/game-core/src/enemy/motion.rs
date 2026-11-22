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



/// `CalculateFirstBGCollisionPoint ; 07:E90F` 的 **X 段**（水平查与垂直查都用）。
/// asm：`ldx #$00 / lda distX / bmi`（distX 负 ⇒ X 保持 0，否则 `dex` ⇒ X=$FF）→
/// `col = posX − distX` → `txa / adc #$00` 非零 ⇒ 回绕 ⇒ `ScrollDir & $02` 非零才 `inc hi`。
fn first_bg_point_x(pos_x: u8, dist_x: u8, hi: u8, scroll_dir: u8) -> (u8, u8) {
    let col = pos_x.wrapping_sub(dist_x);
    let negative = dist_x & 0x80 != 0;
    let borrow = pos_x < dist_x;
    let adjust = if negative { !borrow } else { borrow };
    if adjust && scroll_dir & 0x02 != 0 { (col, hi ^ 1) } else { (col, hi) }
}

/// `CalculateFirstBGCollisionPoint ; 07:E90F` 的 **Y 段**（垂直查）。
/// asm：distY≥0（上查）⇒ `row = posY − distY`，**借位** ⇒ `row −= $10`，且
/// `ScrollDir & $02 == 0`（竖滚房）才 `inc hi`；distY<0（下查）⇒ 无借位或 `row ≥ $F0`
/// ⇒ `row += $10`，同条件换页。
fn first_bg_point_y(pos_y: u8, dist_y: u8, hi: u8, scroll_dir: u8) -> (u8, u8) {
    let row = pos_y.wrapping_sub(dist_y);
    let negative = dist_y & 0x80 != 0;
    let borrow = pos_y < dist_y;
    let adjust = if negative { !borrow || row >= 0xF0 } else { borrow };
    if adjust {
        let row2 = if negative { row.wrapping_add(0x10) } else { row.wrapping_sub(0x10) };
        let hi2 = if scroll_dir & 0x02 == 0 { hi ^ 1 } else { hi };
        (row2, hi2)
    } else {
        (row, hi)
    }
}

/// `CalculateNextBGCollisionPoint ; 07:E98E` 的 **Y 半**：`row += offset`；
/// `row ≥ $F0`（或越 256）⇒ `row += $10`，且 `ScrollDir & $02 == 0` 才 `inc hi`
/// （横滚房允许回绕、不换页）。
fn next_point_y(row: u8, offset: u8, hi: u8, scroll_dir: u8) -> (u8, u8) {
    let (r, carry) = row.overflowing_add(offset);
    if carry || r >= 0xF0 {
        // asm `Lx212: txa / adc #$0F`——两条径进入时 carry 都是 1（adc 自身进位，
        // 或 `cpx #$F0` 在 X≥$F0 时置位）⇒ 实际是 `+ $10`。
        let r2 = r.wrapping_add(0x10);
        let hi2 = if scroll_dir & 0x02 == 0 { hi ^ 1 } else { hi };
        (r2, hi2)
    } else {
        (r, hi)
    }
}

/// `CalculateNextBGCollisionPoint ; 07:E98E` 的 **X 半**：`col += offset`，越 256（进位）⇒
/// `ScrollDir & $02` 非零才 `inc hi`（竖滚房不换页）。
fn next_point_x(col: u8, offset: u8, hi: u8, scroll_dir: u8) -> (u8, u8) {
    let (c, carry) = col.overflowing_add(offset);
    if carry && scroll_dir & 0x02 != 0 { (c, hi ^ 1) } else { (c, hi) }
}

/// `GetOtherNameTableIndex ; $8563`（`prg7_engine.asm`，`:1294`）：`A = (EnsExtra.hi ^ PPUCTRL_ZP) & $01`。
/// Z 置 ⇔ 该敌名表与屏幕当前名表**相同**；四个 `EnemyMoveOnePixel*` 的换名表分支据此判定
/// （原版语义：与屏上名表**不同**才允许换——左右上下的判等方向各不相同，逐点忠实）。
fn other_name_table_index(hi: u8, ctrl: u8) -> u8 {
    (hi ^ ctrl) & 0x01
}

/// `SwitchEnemyNameTable ; $855A`：`EnsExtra.hi ^= $01`。
fn switch_enemy_name_table(hi: u8) -> u8 {
    hi ^ 1
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
    let (dy, dx_chain, ndelay, ninstr, nd1f, trig) = crate::enemy::common::enemy_get_delta_xy(
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
    // `EnemyGetDeltaX ; $8318`：加速类 `jmp EnemyGetDeltaX_UsingAcceleration`
    // （`$8395`，speedX/accelX 积分面）。`enemy_get_delta_xy` 的加速径只积分 Y 并
    // 把 Δx 恒置 0（串径才读 `(movPtr),y` 的第二字节）——旧形因此让**任何非零
    // speedX 的加速类敌人永不横移**（Norfair room 19 的 slot5 = Swooper01，
    // emu x `48→3C→3A→…→00`，rust 恒 0x4C）。asm 调序=先 Y 后 X，此处同序。
    let area_now = crate::frame::cur_area(mem);
    let l977b_x = game_data::assets::enemy_facing(area_now)
        .get((type_ & 0x3F) as usize)
        .copied()
        .unwrap_or(0);
    // 加速类：Y 面同样必须走积分件（`$833F` 把 accelY 累进 speedY/speedSubPixelY 并写回
    // 敌面）——`enemy_get_delta_xy` 的加速径只是在纯函数里算了局部值、**从不写回**，
    // 等价于"加速度全丢"。asm 调序：先 EnemyGetDeltaY（写面）后 EnemyGetDeltaX。
    let dy = if l977b_x & 0x40 != 0 {
        enemy_get_delta_y_using_acceleration(mem, x)
    } else {
        dy
    };
    let dx = if l977b_x & 0x40 != 0 {
        enemy_get_delta_x_using_acceleration(mem, x)
    } else {
        dx_chain
    };
    mem.write_wram(ens_addr(x, ENS_F_DATA1F), nd1f);
    mem.write_ram(en_addr(x, EN_DELAY), ndelay);
    // `$0406` 一字节三名（`chunkyExplosionTimer` / `movementInstrIndex` / `speedSubPixelY`，
    // `refs/m1disasm/out/M1_NES_NTSC.sym:11118-11120`）：串径用 `movementInstrIndex`，
    // 加速径用 `speedSubPixelY`，**两者互斥**（原版不会同时用）。加速径若照串径回写
    // `ninstr`（=纯函数返回的入参，通常 0），会把刚积分出来的 `speedSubPixelY` 清零
    // ⇒ 亚像素全丢（room 19 slot5 的 `Ens+6` emu 0x80 / rust 0x00）。
    if l977b_x & 0x40 == 0 {
        mem.write_ram(en_addr(x, EN_INSTR), ninstr);
    }
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

    /// `EnemyGetDeltaY ; $82ED` 的加速类分支：加速类必须用
    /// `EnemyGetDeltaY_UsingAcceleration`（`$833F`，accelY 积分进 speedY/speedSubPixelY）。
    /// 旧形在 `enemy_get_delta_xy` 里**算完就丢**（局部 `sub`/`sp` 从不写回敌面，
    /// 只把 `sp` 当 Δ 返回）⇒ 速度恒定、加速度完全失效（Norfair room 19 slot5
    /// Swooper01：emu speedY `0C→…→07` 且 speedSubPixelY 累到 `$80`，rust 恒 `0C`/`00`）。
    #[test]
    fn accel_class_y_delta_integrates_speed_y() {
        let mut m = Manager::default();
        if !game_data::pack::is_ready() {
            let _ = game_data::pack::init_from_env();
        }
        stage(&mut m, 0x10, 0, 0x00, 0x02);
        let Some(t) = find_type(&mut m, |r| r & 0x40 != 0) else {
            eprintln!("SKIP（显式）：本区 L977B 没有加速类类型");
            return;
        };
        m.write_wram(ens_addr(0x10, ENS_F_TYPE), t);
        m.enemy(1).set_speed_y(0x0C);
        m.enemy(1).set_accel_y(-128); // Swooper01：朝天花板重力
        m.enemy(1).set_data06(0x00); // speedSubPixelY（$0406）
        let _ = enemy_delta_for_move(&mut m, 0x10);
        // 加速度必须落盘：speedY 变小（12 → 11）且 speedSubPixelY 累到 $80。
        assert!(
            (m.enemy(1).speed_y() as i16) < 0x0C,
            "加速类敌人的 speedY 必须被积分（旧形恒 {:#04X}）",
            m.enemy(1).speed_y()
        );
        assert_eq!(m.enemy(1).data06(), 0x80, "speedSubPixelY 必须累加 accelY 的低字节");
    }

    /// `EnemyGetDeltaX ; $8318` 的加速类分支：加速类（`L977B` bit6）必须用
    /// `EnemyGetDeltaX_UsingAcceleration`（speedX/accelX 积分面）算 Δx。
    /// 旧形在 `enemy_get_delta_xy` 的加速径把 Δx **恒置 0**（只有串径才读
    /// `(movPtr),y` 的第二字节）⇒ 非零 speedX 的敌人永不横移（Norfair room 19
    /// slot5 = Swooper01：emu x `48→3C→3A→…→00`，rust 恒 0x4C）。
    #[test]
    fn accel_class_x_delta_comes_from_speed_x_integrator() {
        let mut m = Manager::default();
        if !game_data::pack::is_ready() {
            let _ = game_data::pack::init_from_env();
        }
        stage(&mut m, 0x10, 0, 0x00, 0x02);
        let Some(t) = find_type(&mut m, |r| r & 0x40 != 0) else {
            eprintln!("SKIP（显式）：本区 L977B 没有加速类类型");
            return;
        };
        m.write_wram(ens_addr(0x10, ENS_F_TYPE), t);
        m.enemy(1).set_speed_x(0x10); // +16 px/s ⇒ 本帧 Δx 必须非 0
        let (_, dx) = enemy_delta_for_move(&mut m, 0x10);
        assert_ne!(dx, 0, "加速类敌人的 Δx 必须来自 speedX 积分（旧形恒 0）");
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
    let slot = x / 16;
    let xpos = mem.read_ram(0x0400 + x as u16 + 1); // x=+1
    let rx = mem.read_wram(0x6AF4 + x as u16 + 2); // radiusX=+2
    if xpos.wrapping_sub(rx) & 0x07 == 0 {
        if !enemy_check_move_left(mem, x) {
            mem.write_ram(0x00, 0); // $00=0（碰撞）
            return false;
        }
    }
    mem.write_ram(0x00, 1); // $00=1（移动进行——inc $00（:1161））
    // x==0 的换名表分支（asm $84C0-$84DA 逐字）：**仅横滚房**（`ScrollDir >= 2`）处理；
    // `ScrollX == 0` ⇒ 失败；`GetOtherNameTableIndex == 0`（与屏上名表相同）⇒ 失败
    // （「试图在屏外换名表」）；否则 `SwitchEnemyNameTable`。
    // 【2026-11-22 修（Norfair room 19 f1710）】旧形无条件 `hi ^= 1` 且**从不失败**。
    if xpos == 0 {
        let sd = mem.scroll_dir();
        if sd >= 0x02 {
            let hi = mem.enemy(slot).hi();
            if mem.scroll_x() == 0 || other_name_table_index(hi, mem.ppuctrl_zp()) == 0 {
                return false;
            }
            mem.enemy(slot).set_hi(switch_enemy_name_table(hi));
        }
    }
    let nx = xpos.wrapping_sub(1);
    mem.write_ram(0x0400 + x as u16 + 1, nx);
    // 左界==0（dec 后 x==radiusX）⇒失败+**回滚**（inc x——:1184-1199 逐字）
    // 【$00 语义修（2026-11-17，100% 录像 f2576 定谳）】asm 此径是 `inc x / clc / bcc @RTS`，
    // **不改 `$00`** ⇒ 它保持前面 `inc $00` 的 1（=「试图换名表/离屏」），
    // 而瓦撞径才是 `ldy #$00 / sty $00`（=0）。调用方 `CrawlerInsideCornerCheck` 正是用
    // `lda $00 / bne` 区分「翻向」与「贴墙转角」——旧形在此写 0 ⇒ 爬虫总走贴墙 ⇒ 位置分叉。
    if nx == rx {
        // asm $84DA-$84F0 逐字：`ScrollX == 0` ⇒ 失败；否则 `GetOtherNameTableIndex`
        // **非零（与屏上名表不同）⇒ 成功**（允许跨名表继续左移），相同才回滚失败。
        // 【2026-11-22 修（Norfair room 19 f1710）】旧形无条件回滚失败 ⇒ ScrollX≠0 且名表
        // 不同时误判撞墙，触发水平反弹（`data05 bit0` 翻转 + speedX 取负）⇒ 该敌从此右移。
        let hi = mem.enemy(slot).hi();
        if mem.scroll_x() == 0 || other_name_table_index(hi, mem.ppuctrl_zp()) == 0 {
            mem.enemy(slot).set_pos_x(xpos); // inc x（回滚）
            mem.write_ram(0x00, 1);
            return false;
        }
    }
    // @success（asm $84F4 逐字）：`lda data05 / bpl @endIf_E` ⇒ `bpl` 在 bit7 **清**时跳过，
    // 即 **bit7 置**才 inc jumpDsplcmnt。
    // 【2026-11-23 更正】上一版误读 `bpl` 为「bit7 清才 inc」并翻转了极性 ⇒ 已回退。
    if mem.read_ram(0x0400 + x as u16 + 5) & 0x80 != 0 {
        // asm `@success ; $84F4`：`lda data05 / bpl / **inc** jumpDsplcmnt`（左移）。
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
        // 同左径：【$00 语义修】asm `dec x / clc / bcc @RTS` 不改 `$00` ⇒ 保持 1（离屏判）。
        mem.write_ram(0x00, 1);
        return false;
    }
    if mem.read_ram(0x0400 + x as u16 + 5) & 0x80 != 0 {
        // asm `@success ; $8550`：`lda data05 / bpl @endIf_E` ⇒ **bit7 置**才 dec jumpDsplcmnt。
        // ——右移与左移（`$84F4` inc）**方向相反**；旧形照抄成 inc，竖直朝向的敌
        // 右移时位移记账符号反了。
        let j = mem.read_wram(0x6AF4 + x as u16 + 13);
        mem.write_wram(0x6AF4 + x as u16 + 13, j.wrapping_sub(1));
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
/// 逐点（块界）→MakeRoomRAMPtr→`world::room_ram::tile_blocks_non_projectile`（判据单源）：
/// `$4E`=门/像固实；`<$80`（Brinstar）/`<$70`（其余区）=固实；`>=$A0`=可走；
/// 可炸瓦 `$80-$9F`（Brinstar）/`$70-$9F`（其余区）**对非弹对象固实**。返回 carry（可走=true）。
pub(crate) fn collision_point_ok(mem: &mut crate::frame::Manager, y: u8, x: u8, hi: u8) -> bool {
    let ptr = make_room_ram_ptr(mem, y, x, hi);
    let tile = mem.read_wram(ptr);
    !crate::world::room_ram::tile_blocks_non_projectile(tile)
}

/// `EnemyGetDeltaY_UsingAcceleration ; $833F`（`areas_common.asm`）——PipeBug 等
/// 加速径敌专用：把 `accelY` 积分进 `speedY/speedSubPixelY`、按 ±14 钳位，再把
/// 小数累进 `subPixelY`，返回本帧 Δy（含进位）。
///
/// asm 逐段：`ldy #$0E`（正极速）→ `lda accelY / bmi @else_A`；正加速 `clc/adc
/// speedSubPixelY/sta`+`lda speedY/adc #$00/sta`；负加速先 `TwosComplement` 取
/// |accel| 再 16 位减；两径汇合后 `bpl @endIf_B`（速度仍负 ⇒ `TwosComplement` 取
/// 绝对速度、`ldy #$F2`＝−14）→ `cmp #$0E / bcc @endIf_C`（|speed| ≥ 14 ⇒ 钳位：
/// `speedSubPixelY←0`、`speedY←Y`）→ `subPixelY += speedSubPixelY`（进位链）→
/// `Delta = speedY + carry`。
pub(crate) fn enemy_get_delta_y_using_acceleration(
    mem: &mut crate::frame::Manager,
    x: u8,
) -> i8 {
    let slot = x / 16;
    let accel = mem.enemy(slot).accel_y();
    let mut sub = mem.enemy(slot).data06(); // $0406 == speedSubPixelY（三名同址）
    let mut sp = mem.enemy(slot).speed_y() as u8;
    if accel >= 0 {
        let t = sub as u16 + accel as u16;
        sub = t as u8;
        sp = sp.wrapping_add((t >> 8) as u8);
    } else {
        let a = (accel as u8).wrapping_neg(); // TwosComplement(|accel|)
        let t = (sub as i16) - (a as i16);
        sub = t as u8;
        if t < 0 {
            sp = sp.wrapping_sub(1);
        }
    }
    let mut cap: u8 = 0x0E;
    let mut abs = sp;
    if (sp as i8) < 0 {
        abs = (sp as i8).wrapping_neg() as u8;
        cap = 0xF2;
    }
    if abs >= 0x0E {
        sub = 0;
        sp = cap;
    }
    // `subPixelY += speedSubPixelY`（进位链）⇒ Delta = speedY + carry
    let (nsub, carry) = mem.enemy(slot).subpixel_y().overflowing_add(sub);
    mem.enemy(slot).set_subpixel_y(nsub);
    mem.enemy(slot).set_data06(sub);
    mem.enemy(slot).set_speed_y(sp as i8);
    (sp as i8).wrapping_add(carry as i8)
}

/// `EnemyGetDeltaX_UsingAcceleration ; $8395`（`areas_common.asm`）——与 Y 件同构，
/// 但用**有符号 16 位对**比较：`speedSubPixelX += accelX`（`accelX<0` 时高字节
/// `adc #$FF` 符号扩展）⇒ 负速取绝对值与 `(0x00,0x0E)` 比 ⇒ 超限钳 `±0x0E`。
pub(crate) fn enemy_get_delta_x_using_acceleration(
    mem: &mut crate::frame::Manager,
    x: u8,
) -> i8 {
    let slot = x / 16;
    let accel = mem.enemy(slot).accel_x();
    let mut sub = mem.enemy(slot).speed_subpixel_x();
    let t = sub as u16 + (accel as i8 as i16 as u16);
    sub = t as u8;
    let mut sp = (mem.enemy(slot).speed_x() as i16).wrapping_add((t >> 8) as i16);
    let sp8 = sp as u8;
    // 绝对速度 vs 绝对极速（(0x00,0x0E) 或 (0xFF,0xF2) 的绝对值同为 0x0E）
    let abs_sp: u16 = if sp < 0 {
        (0u16).wrapping_sub(sp as u16) & 0x00FF | ((sp as u8 == 0) as u16 * 0)
    } else {
        sp as u16 & 0x00FF
    };
    let _ = abs_sp;
    // 简化等价：|speedX|（含小数进位后）≥ 14 ⇒ 钳位到 ±0x0E（小数清零）。
    let abs_int = if sp < 0 { (-sp) as u16 } else { sp as u16 };
    let abs_pair = (abs_int << 8) | sub as u16; // 高=整数绝对值，低=小数
    if abs_pair >= (0x0E << 8) {
        sub = 0;
        sp = if sp < 0 { -0x0E } else { 0x0E };
    }
    let (nsub, carry) = mem.enemy(slot).subpixel_x().overflowing_add(sub);
    mem.enemy(slot).set_subpixel_x(nsub);
    mem.enemy(slot).set_speed_subpixel_x(sub);
    mem.enemy(slot).set_speed_x(sp as i8);
    let _ = sp8;
    (sp as i8).wrapping_add(carry as i8)
}

/// `ApplySpeedToPosition ; 07:FD8F`（`prg7_engine.asm:12693`）——把整段 Δ 一次性
/// 加到位（**不是逐像素碰撞**：此径只判屏幕边界回绕）。返回 true=成功。
///
/// asm 逐段：`ScrollDir&$02 → Temp02`（横滚标志）；y 正：`adc y`，`bcs || y>=$F0`
/// ⇒ `adc #$0F`（进位恒置＝+$10）且**横滚则失败**否则 `hi++`；y 负：`adc y`，
/// 无进位 ⇒ `sbc #$0F`（＝−$10）且横滚则失败否则 `hi++`；x 正：`adc x`，进位 ⇒
/// 竖滚则失败否则 `hi++`；x 负：无进位 ⇒ 同上。失败时**位置不写回**（asm 只改
/// Temp08/09/0B，敌面位置由调用方的 `LoadEnemyPositionFromTemp` 落盘）。
pub(crate) fn apply_speed_to_position(
    mem: &mut crate::frame::Manager,
    x: u8,
    dy: i8,
    dx: i8,
) -> bool {
    let (ok, ny, nx, hi) = apply_speed_to_position_parts(mem, x, dy, dx);
    if ok {
        let slot = x / 16;
        mem.enemy(slot).set_y(ny);
        mem.enemy(slot).set_pos_x(nx);
        mem.enemy(slot).set_hi(hi);
    }
    ok
}

/// `ApplySpeedToPosition`（`07:FD8F`）的**拆分变体**：只算 Temp08/09/0B 三个暂存值
/// （`(ok, ny, nx, hi)`），**不写回**敌面。`apply_speed_to_position` = ok 时落盘。
///
/// 需要「失败也执行 `LoadEnemyPositionFromTemp`」的调用方（Squeept 熔岩回吸，
/// `02:9ABD @then_C` 后 `02:9AC7 jsr LoadEnemyPositionFromTemp` 恒执行——
/// 越界时 asm 把环绕修正后的暂存值写回敌面）用本变体自行落盘。
pub(crate) fn apply_speed_to_position_parts(
    mem: &mut crate::frame::Manager,
    x: u8,
    dy: i8,
    dx: i8,
) -> (bool, u8, u8, u8) {
    let slot = x / 16;
    let e = mem.enemy(slot);
    let (y0, x0, hi0) = (e.y(), e.pos_x(), e.hi());
    apply_speed_to_position_raw(mem, y0, x0, hi0, dy, dx)
}

/// `ApplySpeedToPosition`（`07:FD8F`）的**原始三元组变体**：不读敌面，直接对给定
/// `(y,x,hi)` 计算环绕修正——asm 里 `Temp08/09/0B` 是纯暂存（`StoreEnemyPositionToTemp`
/// 之后才落 `ApplySpeedToPosition`），敌弹出生定位（07:F91D `SetEnProjectilePosition`：
/// temp=射主位置，偏移量施加后 `LoadEnemyPositionFromTemp` 写入**弹体**槽）走的就是
/// 暂存面而非敌面。
pub(crate) fn apply_speed_to_position_raw(
    mem: &mut crate::frame::Manager,
    y0: u8,
    x0: u8,
    hi0: u8,
    dy: i8,
    dx: i8,
) -> (bool, u8, u8, u8) {
    let horizontal = mem.read_ram(crate::ram_map::SCROLL_DIR) & 0x02 != 0;
    let mut hi = hi0;
    let mut ny = y0;
    let mut nx = x0;
    if dy != 0 {
        if dy > 0 {
            let (v, carry) = y0.overflowing_add(dy as u8);
            ny = v;
            if carry || ny >= 0xF0 {
                ny = ny.wrapping_add(0x10);
                if horizontal {
                    return (false, ny, nx, hi);
                }
                hi = hi.wrapping_add(1);
            }
        } else {
            let (v, carry) = y0.overflowing_add(dy as u8);
            ny = v;
            if !carry {
                ny = ny.wrapping_sub(0x10);
                if horizontal {
                    return (false, ny, nx, hi);
                }
                hi = hi.wrapping_add(1);
            }
        }
    }
    if dx != 0 {
        if dx > 0 {
            let (v, carry) = x0.overflowing_add(dx as u8);
            nx = v;
            if carry {
                if !horizontal {
                    return (false, ny, nx, hi);
                }
                hi = hi.wrapping_add(1);
            }
        } else {
            let (v, carry) = x0.overflowing_add(dx as u8);
            nx = v;
            if !carry {
                if !horizontal {
                    return (false, ny, nx, hi);
                }
                hi = hi.wrapping_add(1);
            }
        }
    }
    (true, ny, nx, hi)
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
            let sd = mem.read_ram(crate::ram_map::SCROLL_DIR);
            let (mut row, hi0) = first_bg_point_y(st.pos_y, st.dist_to_center_y, st.pos_hi, sd);
            let (left, n) = crate::samus::physics::vertical_span_cols(st.pos_x, st.radius_x);
            // 垂直查的 X 段另有一次回绕修正（distX = radiusX）：左边界借位且 `ScrollDir & $02`
            // 置 ⇒ 邻页。`first_bg_point_x` 就是这段（col 起点 = left）。
            let (mut col, mut hi_eff) = first_bg_point_x(st.pos_x, st.radius_x, hi0, sd);
            let _ = left;
            for i in 0..n {
                if !collision_point_ok(mem, row, col, hi_eff) {
                    return false;
                }
                if i + 1 < n {
                    // 垂直查：Y 每步 +0（Temp06=0）、X 每步 +8（Temp07=8）
                    let (r2, h2) = next_point_y(row, 0, hi_eff, sd);
                    let (c2, h3) = next_point_x(col, 8, h2, sd);
                    row = r2;
                    col = c2;
                    hi_eff = h3;
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
            let sd = mem.read_ram(crate::ram_map::SCROLL_DIR);
            let (mut row, hi0) = first_bg_point_y(st.pos_y, st.dist_to_center_y, st.pos_hi, sd);
            let (left, n) = crate::samus::physics::vertical_span_cols(st.pos_x, st.radius_x);
            // 垂直查的 X 段另有一次回绕修正（distX = radiusX）：左边界借位且 `ScrollDir & $02`
            // 置 ⇒ 邻页。`first_bg_point_x` 就是这段（col 起点 = left）。
            let (mut col, mut hi_eff) = first_bg_point_x(st.pos_x, st.radius_x, hi0, sd);
            let _ = left;
            for i in 0..n {
                if !collision_point_ok(mem, row, col, hi_eff) {
                    return false;
                }
                if i + 1 < n {
                    // 垂直查：Y 每步 +0（Temp06=0）、X 每步 +8（Temp07=8）
                    let (r2, h2) = next_point_y(row, 0, hi_eff, sd);
                    let (c2, h3) = next_point_x(col, 8, h2, sd);
                    row = r2;
                    col = c2;
                    hi_eff = h3;
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
            let sd = mem.read_ram(crate::ram_map::SCROLL_DIR);
            let (col, hi0) = first_bg_point_x(st.pos_x, st.dist_to_center_x, st.pos_hi, sd);
            let (mut row, mut hi_eff) = first_bg_point_y(st.pos_y, st.radius_y, hi0, sd);
            let (top, n) = crate::samus::physics::horizontal_span_rows(st.pos_y, st.radius_y);
            let _ = top;
            for i in 0..n {
                if !collision_point_ok(mem, row, col, hi_eff) {
                    return false;
                }
                if i + 1 < n {
                    // 水平查：Y 每步 +8（Temp06=8）、X 每步 +0（Temp07=0）
                    let (r2, h2) = next_point_y(row, 8, hi_eff, sd);
                    let (_c2, h3) = next_point_x(col, 0, h2, sd);
                    row = r2;
                    hi_eff = h3;
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
            let sd = mem.read_ram(crate::ram_map::SCROLL_DIR);
            let (col, hi0) = first_bg_point_x(st.pos_x, st.dist_to_center_x, st.pos_hi, sd);
            let (mut row, mut hi_eff) = first_bg_point_y(st.pos_y, st.radius_y, hi0, sd);
            let (top, n) = crate::samus::physics::horizontal_span_rows(st.pos_y, st.radius_y);
            let _ = top;
            for i in 0..n {
                if !collision_point_ok(mem, row, col, hi_eff) {
                    return false;
                }
                if i + 1 < n {
                    // 水平查：Y 每步 +8（Temp06=8）、X 每步 +0（Temp07=0）
                    let (r2, h2) = next_point_y(row, 8, hi_eff, sd);
                    let (_c2, h3) = next_point_x(col, 0, h2, sd);
                    row = r2;
                    hi_eff = h3;
                }
            }
            true
        }
    }
}

#[cfg(test)]
mod tests_jump_dsplcmnt_signs {
    use super::*;
    use crate::enemy::tables::{en_addr, ens_addr};

    fn stage(m: &mut crate::frame::Manager, x: u8) {
        m.write_ram(en_addr(x, 1), 0x20); // x 位置
        m.write_ram(en_addr(x, 5), 0x80); // data05 bit7 置（左右移 `bpl` 跳过 ⇒ 记账路径）
        m.write_wram(ens_addr(x, 2), 0x00); // radiusX=0 ⇒ 不触发块界判
        m.enemy(1).set_jump_dsplcmnt(0x00);
    }

    /// asm `$84F4 @success`：`lda data05 / bpl / **inc** jumpDsplcmnt`（左移）。
    #[test]
    fn left_move_increments_jump() {
        let mut m = crate::frame::Manager::default();
        stage(&mut m, 0x10);
        assert!(super::enemy_move_one_pixel_left(&mut m, 0x10));
        assert_eq!(m.enemy(1).jump_dsplcmnt(), 0x01);
    }

    /// asm `$8550 @success`：同形但 **dec**（右移）——旧形照抄左移的 inc，
    /// 竖直朝向敌人右移时位移记账符号反了。
    #[test]
    fn right_move_decrements_jump() {
        let mut m = crate::frame::Manager::default();
        stage(&mut m, 0x10);
        assert!(super::enemy_move_one_pixel_right(&mut m, 0x10));
        assert_eq!(m.enemy(1).jump_dsplcmnt(), 0xFF);
    }

    /// asm `$84DA-$84F0`：`dec x` 后 `x == radiusX`——`ScrollX != 0` 且
    /// `GetOtherNameTableIndex != 0`（本敌名表 ≠ 屏上名表）⇒ **移动成功**、不回滚。
    /// 修前此径无条件回滚失败 ⇒ 水平反弹（Norfair room 19 f1710 的根因）。
    #[test]
    fn left_move_wraps_when_scroll_x_and_nametable_differ() {
        let mut m = crate::frame::Manager::default();
        stage(&mut m, 0x10);
        m.write_wram(ens_addr(0x10, 2), 0x08); // radiusX=8
        m.enemy(1).set_pos_x(0x09); // dec 后 x == radiusX
        m.enemy(1).set_hi(0x01);
        m.set_ppuctrl_zp(0x00); // (1 ^ 0) & 1 == 1 ⇒ 名表不同
        m.set_scroll_x(0x40);
        m.set_scroll_dir(0x03);
        assert!(super::enemy_move_one_pixel_left(&mut m, 0x10));
        assert_eq!(m.enemy(1).pos_x(), 0x08, "不应回滚");
        assert_eq!(m.enemy(1).hi(), 0x01, "成功径不改 hi");
    }

    /// asm `$84DA-$84F0` 的第三种组合：`ScrollX != 0` 但**名表相同**
    /// （`GetOtherNameTableIndex == 0`）⇒ 回滚失败。
    #[test]
    fn left_move_rolls_back_when_nametable_is_same() {
        let mut m = crate::frame::Manager::default();
        stage(&mut m, 0x10);
        m.write_wram(ens_addr(0x10, 2), 0x08);
        m.enemy(1).set_pos_x(0x09);
        m.enemy(1).set_hi(0x01);
        m.set_ppuctrl_zp(0x01); // (1 ^ 1) & 1 == 0 ⇒ 名表相同
        m.set_scroll_x(0x40);
        m.set_scroll_dir(0x03);
        assert!(!super::enemy_move_one_pixel_left(&mut m, 0x10));
        assert_eq!(m.enemy(1).pos_x(), 0x09, "必须回滚到原位");
    }

    /// asm `$84C0-$84D8` 的 `x == 0` 分支：`ScrollDir < 2`（竖滚房）⇒ **整段跳过**
    /// （既不失败也不换名表），随后正常 `dec x` 回绕到 `$FF`。
    #[test]
    fn left_move_x_zero_skips_nametable_when_vertical_scroll() {
        let mut m = crate::frame::Manager::default();
        stage(&mut m, 0x10);
        m.write_wram(ens_addr(0x10, 2), 0x01); // radiusX=1 ⇒ (0-1)&7 != 0 ⇒ 跳过块判
        m.enemy(1).set_pos_x(0x00);
        m.enemy(1).set_hi(0x01);
        m.set_ppuctrl_zp(0x01); // 名表相同（若误入换表径就会失败）
        m.set_scroll_x(0x00);
        m.set_scroll_dir(0x01); // < 2 ⇒ 跳过
        assert!(super::enemy_move_one_pixel_left(&mut m, 0x10));
        assert_eq!(m.enemy(1).pos_x(), 0xFF);
        assert_eq!(m.enemy(1).hi(), 0x01, "竖滚房不动 hi");
    }

    /// asm 同上分支：`ScrollX == 0` ⇒ 移动失败（x 保持 0、不改 hi）。
    #[test]
    fn left_move_x_zero_fails_when_scroll_x_is_zero() {
        let mut m = crate::frame::Manager::default();
        stage(&mut m, 0x10);
        m.write_wram(ens_addr(0x10, 2), 0x01);
        m.enemy(1).set_pos_x(0x00);
        m.enemy(1).set_hi(0x01);
        m.set_ppuctrl_zp(0x00);
        m.set_scroll_x(0x00);
        m.set_scroll_dir(0x03);
        assert!(!super::enemy_move_one_pixel_left(&mut m, 0x10));
        assert_eq!(m.enemy(1).pos_x(), 0x00);
        assert_eq!(m.enemy(1).hi(), 0x01);
    }

    /// asm 同上分支：名表**相同**（`GetOtherNameTableIndex == 0`）⇒ 失败（「屏外换表」）。
    #[test]
    fn left_move_x_zero_fails_when_nametable_is_same() {
        let mut m = crate::frame::Manager::default();
        stage(&mut m, 0x10);
        m.write_wram(ens_addr(0x10, 2), 0x01);
        m.enemy(1).set_pos_x(0x00);
        m.enemy(1).set_hi(0x01);
        m.set_ppuctrl_zp(0x01);
        m.set_scroll_x(0x40);
        m.set_scroll_dir(0x03);
        assert!(!super::enemy_move_one_pixel_left(&mut m, 0x10));
        assert_eq!(m.enemy(1).pos_x(), 0x00);
    }

    /// asm 同上分支的成功形：`ScrollX != 0` 且名表不同 ⇒ `hi ^= 1` 后继续 `dec x`。
    #[test]
    fn left_move_x_zero_switches_nametable_when_different() {
        let mut m = crate::frame::Manager::default();
        stage(&mut m, 0x10);
        m.write_wram(ens_addr(0x10, 2), 0x01);
        m.enemy(1).set_pos_x(0x00);
        m.enemy(1).set_hi(0x01);
        m.set_ppuctrl_zp(0x00); // (1 ^ 0) & 1 == 1 ⇒ 不同
        m.set_scroll_x(0x40);
        m.set_scroll_dir(0x03);
        assert!(super::enemy_move_one_pixel_left(&mut m, 0x10));
        assert_eq!(m.enemy(1).hi(), 0x00, "SwitchEnemyNameTable: hi ^= 1");
        assert_eq!(m.enemy(1).pos_x(), 0xFF);
    }

    /// asm 同径的另一半：`ScrollX == 0` ⇒ 回滚失败（x 不变、返回 false）。
    #[test]
    fn left_move_rolls_back_when_scroll_x_is_zero() {
        let mut m = crate::frame::Manager::default();
        stage(&mut m, 0x10);
        m.write_wram(ens_addr(0x10, 2), 0x08);
        m.enemy(1).set_pos_x(0x09);
        m.enemy(1).set_hi(0x01);
        m.set_ppuctrl_zp(0x00);
        m.set_scroll_x(0x00);
        m.set_scroll_dir(0x03);
        assert!(!super::enemy_move_one_pixel_left(&mut m, 0x10));
        assert_eq!(m.enemy(1).pos_x(), 0x09, "必须回滚到原位");
    }
}

/// `CalculateFirstBGCollisionPoint ; 07:E90F` / `CalculateNextBGCollisionPoint ; 07:E98E`
/// 的像素点行走语义（Norfair room 19：x=6/radiusX=8 的垂直查在 x=$FE 列撞出**假碰撞**，
/// 原因是此前完全没有「回绕判 + 换名表页」这段）。
mod tests_bg_collision_point {
    use super::*;

    /// 上查且左边界借位（posY < distY）⇒ `row −= $10`；竖滚房（`ScrollDir & 2 == 0`）
    /// 才换页——横滚房允许回绕，不动 hi。
    #[test]
    fn first_point_y_up_borrow_shifts_row_and_page_only_in_vertical_rooms() {
        // posY=4, distY=16 ⇒ 借位
        assert_eq!(super::first_bg_point_y(0x04, 0x10, 0x01, 0x00), (0xE4, 0x00));
        // 横滚房（ScrollDir=2）：同一借位只移行、不换页
        assert_eq!(super::first_bg_point_y(0x04, 0x10, 0x01, 0x02), (0xE4, 0x01));
        // 无借位：两个字段都不变
        assert_eq!(super::first_bg_point_y(0x40, 0x10, 0x01, 0x00), (0x30, 0x01));
    }

    /// 下查（distY 为负）：结果越 256 或 ≥ $F0 ⇒ `row += $10`，竖滚房才换页。
    #[test]
    fn first_point_y_down_adjusts_only_near_bottom_edge() {
        assert_eq!(super::first_bg_point_y(0xE8, 0xF8, 0x01, 0x00), (0x00, 0x00));
        assert_eq!(super::first_bg_point_y(0x10, 0xF8, 0x01, 0x00), (0x18, 0x01));
    }

    /// X 段：distX 正（左查）⇒ 借位即回绕；distX 负（右查）⇒ 越界（无借位）才回绕；
    /// 只有 `ScrollDir & 2` 置（横滚房）才换页。
    #[test]
    fn first_point_x_wraps_on_borrow_for_left_and_on_carry_for_right() {
        assert_eq!(super::first_bg_point_x(0x06, 0x10, 0x01, 0x03), (0xF6, 0x00));
        assert_eq!(super::first_bg_point_x(0x06, 0x10, 0x01, 0x00), (0xF6, 0x01));
        assert_eq!(super::first_bg_point_x(0x30, 0x10, 0x01, 0x03), (0x20, 0x01));
        // 右查：radiusX=8 ⇒ distX=$F8，posX=$F8 时 posX+8 越 256 ⇒ 回绕
        assert_eq!(super::first_bg_point_x(0xF8, 0xF8, 0x01, 0x03), (0x00, 0x00));
    }

    /// 下一个碰撞点：X 每步 +8，进位且横滚房 ⇒ 换页（room 19 的第二列正是这样）。
    #[test]
    fn next_point_x_carry_switches_page_when_scrolling_horizontally() {
        assert_eq!(super::next_point_x(0xFE, 0x08, 0x00, 0x03), (0x06, 0x01));
        assert_eq!(super::next_point_x(0xFE, 0x08, 0x00, 0x00), (0x06, 0x00));
        assert_eq!(super::next_point_x(0x10, 0x08, 0x00, 0x03), (0x18, 0x00));
    }

    /// 下一个碰撞点：Y 每步 +8；`row ≥ $F0` ⇒ `row += $10`，竖滚房才换页。
    #[test]
    fn next_point_y_adjusts_bottom_edge_and_page() {
        assert_eq!(super::next_point_y(0xE8, 0x08, 0x01, 0x00), (0x00, 0x00));
        assert_eq!(super::next_point_y(0x10, 0x08, 0x01, 0x00), (0x18, 0x01));
        // 越 256（进位）⇒ 同样走调整径
        assert_eq!(super::next_point_y(0xF8, 0x10, 0x01, 0x02), (0x18, 0x01));
    }
}
