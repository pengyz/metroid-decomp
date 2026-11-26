//! Metroid（Tourian 的吸血水母）AI。
//!
//! **asm 内容锚**：`MetroidAIRoutine_{AREA}: ; 03:97F9`（`refs/m1disasm/SRC/enemies/metroid.asm`
//! 全文 378 行，含六件子例程与两张数据表）——只被 Tourian bank include
//! （`prg3_tourian.asm:166-167` 敌表 00/01 项）。`{AREA}` 宏在其它 bank 展开成空桩，
//! 故本件是**单一份**语义实现。
//!
//! ## 结构（逐行对读 asm）
//!
//! 1. **逃逸计时器门**：`ldy EndTimer+1 / iny / beq @endIf_A` —— `EndTimer+1+1 == 0`
//!    （即 `EndTimer+1 == $FF`）表示「逃逸序列进行中」，此时直接把自己删掉（`status ← 0`）。
//! 2. **动画参数**：`lda #$0F / sta $00 / sta $01`（Active 节拍 15、Resting 节拍 15）。
//! 3. 三个早退径：`data05` **bit7**（爆散动画帧）、`status == Explode`。
//! 4. **未吸附**（`MetroidLatch[slot] == 0`）时：把 `|speedY|`、`|speedX|` 钳到
//!    `MetroidMaxSpeed[movementIndex]`（red=2 / green=4），再按 `data05` 的 bit0/bit2 取
//!    加减速（`GetMetroidAccel`），最后维护「冻结 / 无敌」血量：
//!    `status == Frozen` 且 `health+1 == 0`（即 `$FF` 无敌值）⇒ `health ← 5`；
//!    非冻结 ⇒ `health ← $FF`（**无敌**）。
//! 5. **`@latchActive`**：AI 前快照是 Hurt 且当前已不是 Hurt ⇒ `status ← Frozen`
//!    （被打得停止受伤那一帧冻结）。
//! 6. **炸弹计数**：`isHit` bit5 且已吸附时，若武器是 `wa_Unknown7(7)` 或
//!    `wa_BombExplode($0A)` 且 `FrameCount` bit1 清 ⇒ 锁存 bits4-6 加 `$10`；
//!    命中 5 次（`(latch & $70) == $50`）⇒ 脱离：`prevStatus ← Active|specialAttribs`、
//!    `status ← Hurt`、`specialAttribs ← $20`、`data0D ← 1`。
//!    随后清 `isHit`、清锁存、清小数速度，并把速度设为**反向弹开**（`GetMetroidRepelSpeed`）。
//! 7. **吸附判定**：未吸附且 `isHit` bit2（碰到 Samus）⇒ 锁存 `← (speedX & $80) | $01`
//!    （bit7 记速度符号），并清速度。
//! 8. **吸附中**：锁存低 nibble 未到 `$0C` 就 +1；位置 = Samus 位置 + 表偏移
//!    （`MetroidLatchOffsetY[latch&$0F - 1]`、`x = 12 - (latch&$0F)`，锁存 bit7 清时 x 取负）。
//! 9. **`@metroidOnSamus`**：`status == Explode` ⇒ 清锁存；六槽锁存的 `& $0C` 有满 `$0C`
//!    且 Samus 血量非 0 ⇒ `HealthChange ← $0004`（1/4 点）+ `SubtractHealth`，
//!    `MetroidOnSamus ← 1`；`ObjectCntrl` bit7 清 ⇒ `ObjectCntrl ← type | $82 | $20`（优先级+调色）。
//! 10. 尾件 `UpdateEnemyCommon_Decide`（`lda #$0F` 两拍）。

use crate::enemy::framework::update_enemy_common_decide_with_beats;
use crate::enemy::motion::apply_speed_to_position_raw;
use crate::enemy::state::{slot_of, EnemyStatus};
use crate::frame::Manager;
use crate::ram_map::{END_TIMER, METROID_ON_SAMUS};

/// `MetroidRepelSpeed ; 07:C5A8+0`（`$77F0/$77F1`）——脱离吸附时的弹开速度，
/// 下标 = 加速字节的 bit6（`GetMetroidRepelSpeed ; 03:9A10`：`asl / rol / and #$01`）。
const METROID_REPEL_SPEED: [i8; 2] = [-8, 8];
/// `MetroidAccel ; 07:C5A8+2`（`$77F2-$77F5`）—— red 的 ±48、green 的 ±96；
/// 下标 = `(movementIndex << 1) | 方向位`（`GetMetroidAccel ; 03:9A06` 的 `rol`）。
const METROID_ACCEL: [i8; 4] = [0x30, -0x30, 0x60, -0x60];
/// `MetroidMaxSpeed ; 07:C5A8+6`（`$77F6/$77F7`）—— red=2、green=4。
const METROID_MAX_SPEED: [u8; 2] = [2, 4];
// `MetroidLatchOffsetY ; 03:99D8`——吸附后相对 Samus 的 y 偏移（下标 = 进度计数器 - 1），
// 已迁到资产包（`game_data::assets::metroid_latch_offset_y()`）。
/// `OAMDATA_PRIORITY`（`refs/m1disasm/SRC/hardware.asm:49`）= `%00100000`。
const OAMDATA_PRIORITY: u8 = 0x20;
/// 吸附进度满值（`and #$0F / cmp #$0C`）。
const LATCH_FULL: u8 = 0x0C;
/// 动画节拍（`lda #$0F / sta $00 / sta $01`）。
const ANIM_BEAT: u8 = 0x0F;

/// `GetMetroidAccel ; 03:9A06`：`lsr`（方向位进 C）→ `lda movementIndex` → `rol`
/// ⇒ 下标 = `(movementIndex << 1) | C`；表值即加速字节（方向位 1 ⇒ 负）。
fn metroid_accel(kind: u8, sign_bit: bool) -> i8 {
    let idx = ((kind << 1) | sign_bit as u8) & 0x03;
    METROID_ACCEL[idx as usize]
}

/// `GetMetroidRepelSpeed ; 03:9A10`：下标 = 加速字节 bit6。
fn metroid_repel_speed(accel: i8) -> i8 {
    METROID_REPEL_SPEED[((accel as u8 >> 6) & 0x01) as usize]
}

/// 速度钳位（`MetroidAIRoutine` 的 y/x 两段，`03:9839-986D`）。
///
/// 负速度径（`03:9840-9848` / `03:9859-985F`）：
/// ```text
/// lda #$00
/// cmp Ens.0.speedSubPixelX,x   ; 只置 C，A 不变（仍为 0）
/// sbc Ens.0.speedX,x           ; A = 0 - speed - (1-C) = -speed - (sub != 0)
/// ```
/// **CMP 不写 A** ⇒ 幅值低字节 = `0u8 - speed - (sub != 0)`，小数字节只通过借位影响结果。
/// 【2026-11-23 更正】旧实现按 16 位真幅值 `0 - (speed<<8 | sub)` 取低字节，等价于
/// `0u8 - sub`（CMP 被当成写入 A=0 的减法），`speed=$FF/sub=$D0` 时得 `$30` 而 ROM 得 `$00`
/// ⇒ 原版不钳位、Rust 钳位并清小数速度（Tourian10 首分歧，见
/// docs/findings/sprite-residual-tourian10-ridley37.md §4）。
/// 正速度径：直接比较速度字节。`>= max` 时把速度设为带符号的 max、小数位清 0。
/// 低字节比较是原版 quirk（整数速度恰好 ≥ max 时可能逃过钳位），按 asm 保留。
fn clamp_axis(speed: i8, sub: u8, max: u8) -> (i8, u8) {
    let magnitude_low = if speed < 0 {
        // lda #$00 / cmp sub（只置 C）/ sbc speed ⇒ 0 - speed - (sub != 0) 的低字节
        0u8.wrapping_sub(speed as u8)
            .wrapping_sub((sub != 0) as u8)
    } else {
        speed as u8
    };
    if magnitude_low >= max {
        (if speed < 0 { max.wrapping_neg() as i8 } else { max as i8 }, 0)
    } else {
        (speed, sub)
    }
}

/// Metroid AI（`MetroidAIRoutine_{AREA} ; 03:97F9`）。`x` 是敌槽的**字节偏移**。
pub(crate) fn metroid_ai(mem: &mut Manager, x: u8) {
    let slot = slot_of(x);

    // asm:2-7 —— 逃逸计时器进行中（EndTimer+1 == $FF）⇒ 删除自己
    let end_timer_hi = mem.end_timer_hi();
    if end_timer_hi.wrapping_add(1) != 0 {
        mem.enemy(slot).set_status(EnemyStatus::NoEnemy);
    }

    // asm:9-16 —— 三条早退径（动画参数 $0F/$0F 已在下面两处尾件里直传）
    // 【2026-11-22 更正】`asl / bmi`（ROM 03:97FB `0A` / 03:97FC `30`）判的是**原 bit6**：
    // ASL 先把 bit7 移进 C 丢弃，BMI 读的 N 来自 bit6（同 PipeBug 族，见 pipe_bug.rs）。
    // 旧形判 bit7 ⇒ 门永不命中 ⇒ 早退径失效（该位由 UpdateEnemy_EnData05Bit6 每帧翻转）。
    let data05 = mem.enemy(slot).data05();
    if data05 & 0x40 != 0 {
        update_enemy_common_decide_with_beats(mem, x, ANIM_BEAT, ANIM_BEAT);
        return;
    }
    if mem.enemy(slot).status_raw() == EnemyStatus::Explode.to_byte() {
        update_enemy_common_decide_with_beats(mem, x, ANIM_BEAT, ANIM_BEAT);
        return;
    }

    let mut latch = mem.metroid_latch(slot);

    // asm:18-127 —— 未吸附：限速 + 加速 + 冻结/无敌血量
    if latch == 0 {
        let kind = mem.enemy(slot).movement_index() & 0x01;
        let max = METROID_MAX_SPEED[kind as usize];

        let (sy, suby) = clamp_axis(
            mem.enemy(slot).speed_y(),
            mem.enemy(slot).speed_subpixel_y(),
            max,
        );
        mem.enemy(slot).set_speed_y(sy);
        mem.enemy(slot).set_speed_subpixel_y(suby);

        let (sx, subx) = clamp_axis(
            mem.enemy(slot).speed_x(),
            mem.enemy(slot).speed_subpixel_x(),
            max,
        );
        mem.enemy(slot).set_speed_x(sx);
        mem.enemy(slot).set_speed_subpixel_x(subx);

        // asm:96-107 —— 横加速用 data05 bit0、竖加速用 bit2（`lsr lsr`）
        mem.enemy(slot)
            .set_accel_x(metroid_accel(kind, data05 & 0x01 != 0));
        mem.enemy(slot)
            .set_accel_y(metroid_accel(kind, data05 & 0x04 != 0));

        // asm:110-127 —— 冻结时若还是无敌值（$FF）则给 5 点血；非冻结恒无敌
        if mem.enemy(slot).status_raw() == EnemyStatus::Frozen.to_byte() {
            if mem.enemy(slot).health().wrapping_add(1) == 0 {
                mem.enemy(slot).set_health(5);
            }
        } else {
            mem.enemy(slot).set_health(0xFF);
        }
    }

    // asm:@latchActive 03:9899 —— 受伤停止那一帧冻结
    let pre_ai = mem.enemy_status_pre_ai();
    if pre_ai == EnemyStatus::Hurt.to_byte()
        && mem.enemy(slot).status_raw() != EnemyStatus::Hurt.to_byte()
    {
        mem.enemy(slot).set_status(EnemyStatus::Frozen);
    }

    // asm:130-190 —— isHit bit5（武器命中）
    if mem.enemy(slot).is_hit() & 0x20 != 0 {
        latch = mem.metroid_latch(slot);
        if latch != 0 {
            let weapon = mem.enemy(slot).weapon_action();
            let by_bomb = weapon == 0x07 || weapon == 0x0A; // wa_Unknown7 / wa_BombExplode
            if by_bomb && (mem.frame_count() & 0x02) == 0 {
                latch = latch.wrapping_add(0x10);
                mem.set_metroid_latch(slot, latch);
                if latch & 0x70 == 0x50 {
                    // 五次炸弹命中 ⇒ 脱离吸附
                    let prev = EnemyStatus::Active.to_byte() | mem.enemy(slot).special_attribs();
                    mem.enemy(slot).set_prev_status(prev);
                    mem.enemy(slot).set_status(EnemyStatus::Hurt);
                    mem.enemy(slot).set_special_attribs(0x20);
                    mem.enemy(slot).set_data0d(1);
                }
            }
        }
        // asm:174-188 —— 放开 Samus：清命中位/锁存/小数速度，速度设成反向弹开
        mem.enemy(slot).set_is_hit(0);
        mem.set_metroid_latch(slot, 0);
        latch = 0;
        mem.enemy(slot).set_speed_subpixel_y(0);
        mem.enemy(slot).set_speed_subpixel_x(0);
        let (repel_y, repel_x) = (
            metroid_repel_speed(mem.enemy(slot).accel_y()),
            metroid_repel_speed(mem.enemy(slot).accel_x()),
        );
        mem.enemy(slot).set_speed_y(repel_y);
        mem.enemy(slot).set_speed_x(repel_x);
    }

    // asm:192-208 —— 未吸附且碰到 Samus（isHit bit2）⇒ 开始吸附
    if latch == 0 && mem.enemy(slot).is_hit() & 0x04 != 0 {
        latch = (mem.enemy(slot).speed_x() as u8 & 0x80) | 0x01;
        // `ClearMetroidSpeed`（`03:99C3`）：速度与小数全清，并落到
        // `ClearRinkaAcceleration`（清 accelX/accelY）
        mem.enemy(slot).set_speed_y(0);
        mem.enemy(slot).set_speed_x(0);
        mem.enemy(slot).set_speed_subpixel_x(0);
        mem.enemy(slot).set_speed_subpixel_y(0);
        mem.enemy(slot).set_accel_x(0);
        mem.enemy(slot).set_accel_y(0);
        mem.set_metroid_latch(slot, latch);
    } else if latch == 0 {
        // asm:@metroidOnSamus_clearLatch —— 未吸附且没碰到 ⇒ 清锁存（本就是 0）
        mem.set_metroid_latch(slot, 0);
    }

    // asm:@updateLatch 03:9932 —— 吸附进度推进 + 位置贴合 Samus
    if latch != 0 {
        let counter = latch & 0x0F;
        if counter != LATCH_FULL {
            latch = latch.wrapping_add(1);
            mem.set_metroid_latch(slot, latch);
        }
        // `MetroidLatchOffsetY-1,y`（y = 推进前的低 nibble）
        let oy = game_data::assets::metroid_latch_offset_y()[(counter.saturating_sub(1)) as usize & 0x0F] as i8;
        let mut ox = LATCH_FULL.wrapping_sub(counter);
        // `plp / bmi`：锁存 bit7 置位（速度负）时**不**取负
        if latch & 0x80 == 0 {
            ox = 0u8.wrapping_sub(ox);
        }
        // `StoreSamusPositionToTemp` + `ApplySpeedToPosition` + `LoadEnemyPositionFromTemp_`
        let (samus_y, samus_x, samus_hi) =
            (mem.samus().y(), mem.samus().x(), mem.samus().hi());
        let (_, ny, nx, hi) =
            apply_speed_to_position_raw(mem, samus_y, samus_x, samus_hi, oy, ox as i8);
        mem.enemy(slot).set_y(ny);
        mem.enemy(slot).set_pos_x(nx);
        mem.enemy(slot).set_hi(hi & 0x01);
    }

    // asm:@metroidOnSamus 03:9967
    if mem.enemy(slot).status_raw() == EnemyStatus::Explode.to_byte() {
        mem.set_metroid_latch(slot, 0);
    }

    let mut on_samus = false;
    let mut any_full = 0u8;
    for s in 0..6u8 {
        any_full |= mem.metroid_latch(s);
    }

    // 六槽锁存 OR 后的 `& $0C == $0C`（有完全吸附）+ Samus 未死 ⇒ 抽 1/4 点血
    if any_full & 0x0C == LATCH_FULL && mem.samus().health() != 0 {
        mem.set_health_change(0x0004);
        crate::weapon::projectile::subtract_health(mem);
        on_samus = true;
    }
    mem.set_metroid_on_samus(on_samus as u8);

    // asm:301-307 —— ObjectCntrl bit7 清时置 `type | $82 | $20`（调色位 + 优先级）
    if mem.obj_cntrl() & 0x80 == 0 {
        let v = mem.enemy(slot).enemy_type() | 0x82 | OAMDATA_PRIORITY;
        mem.set_obj_cntrl(v);
    }

    // asm:308 —— 尾件
    update_enemy_common_decide_with_beats(mem, x, ANIM_BEAT, ANIM_BEAT);
}

#[cfg(test)]
mod tests_metroid {
    use super::*;
    use crate::enemy::state::EnemyStatus;

    /// 基线：`EndTimer+1 = $FF`（逃逸序列**未**启动，asm `iny / beq` 的不删除径）、
    /// AI 前快照 = Resting（让尾件 `UpdateEnemyCommon_Decide` 走 noMove 径，
    /// 从而不会把被测 status 复活成 Active——那是壳的行为，不是本件的行为）。
    fn mgr() -> Manager {
        let mut m = Manager::default();
        m.write_ram(END_TIMER + 1, 0xFF);
        m.enemy(0).set_status(EnemyStatus::Active);
        m.enemy(0).set_y(0x80);
        m.enemy(0).set_pos_x(0x80);
        m.enemy(0).set_movement_index(0); // red
        crate::frame::set_enemy_status_pre_ai(&mut m, EnemyStatus::Resting.to_byte());
        m
    }

    /// asm:2-7 —— `EndTimer+1+1 != 0`（逃逸序列进行中）⇒ 直接删自己。
    #[test]
    fn escape_timer_deletes_metroid() {
        let mut m = mgr();
        m.write_ram(END_TIMER + 1, 0xFE);
        metroid_ai(&mut m, 0);
        assert_eq!(m.enemy(0).status_raw(), 0, "逃逸计时器进行中 ⇒ status=0");
    }

    /// asm:14-16 —— `lda data05 / asl / bmi`（ROM 03:97FB/03:97FC）判的是**原 bit6** ⇒ bit6 置
    /// 走早退径。旧测用 bit7（对 Metroid 不可达）把错误极性固化成规格。
    #[test]
    fn data05_bit6_is_the_early_exit_not_bit7() {
        let mut m = mgr();
        m.enemy(0).set_data05(0x40);
        m.enemy(0).set_health(1);
        metroid_ai(&mut m, 0);
        assert_eq!(m.enemy(0).health(), 1, "bit6 置 ⇒ 早退径，不得写无敌血");

        let mut m = mgr();
        m.enemy(0).set_data05(0x80);
        m.enemy(0).set_health(1);
        metroid_ai(&mut m, 0);
        assert_eq!(
            m.enemy(0).health(),
            0xFF,
            "bit7 与本门无关（ASL 已丢弃）⇒ 不得早退"
        );
    }

    /// asm:19-21 —— `status == Explode` ⇒ 早退。
    #[test]
    fn explode_status_early_exit() {
        let mut m = mgr();
        m.enemy(0).set_status(EnemyStatus::Explode);
        m.enemy(0).set_health(1);
        metroid_ai(&mut m, 0);
        assert_eq!(m.enemy(0).health(), 1);
    }

    /// asm:36-49 —— red Metroid 的 `|speedY|` 超 `MetroidMaxSpeed[0]=2` ⇒ 钳到 2、小数清 0。
    #[test]
    fn speed_is_clamped_to_max() {
        let mut m = mgr();
        m.enemy(0).set_speed_y(5);
        m.enemy(0).set_speed_subpixel_y(0x80);
        metroid_ai(&mut m, 0);
        assert_eq!(m.enemy(0).speed_y(), 2, "red metroid max speed = 2");
        assert_eq!(m.enemy(0).speed_subpixel_y(), 0, "钳位同时清小数位");
    }

    /// asm:52-57 —— 负速度径的幅值低字节 = `0u8 - speed - (sub != 0)`（CMP 只置 C、不写 A）。
    ///
    /// 【2026-11-23 更正】旧测宣称「`speedY=-3`、小数 0 ⇒ 低字节 0 ⇒ 不钳位」，
    /// 那是把 CMP 当成 `A ← 0 - sub` 的错算；ROM 实际得 `0 - $FD - 0 = $03 ≥ 2` ⇒ **钳到 -2**。
    #[test]
    fn negative_speed_low_byte_follows_rom_borrow_chain() {
        let mut m = mgr();
        m.enemy(0).set_speed_y(-3);
        m.enemy(0).set_speed_subpixel_y(0);
        metroid_ai(&mut m, 0);
        assert_eq!(
            m.enemy(0).speed_y(),
            -2,
            "$FD(sub=0) ⇒ 幅值低字节 $03 ≥ max 2 ⇒ 钳位（不许改成 i16 真幅值）"
        );
    }

    /// 同上，卡面矢量：`speedY=-1`、小数 `$D0` ⇒ `0 - $FF - 1 = 0` ⇒ **不钳位**、小数保留。
    /// （Tourian10 首分歧的字段形态：旧实现在这里得 `$30`，多钳一次并清掉小数速度。）
    #[test]
    fn negative_speed_with_subpixel_can_escape_clamp() {
        let mut m = mgr();
        m.enemy(0).set_speed_x(-1);
        m.enemy(0).set_speed_subpixel_x(0xD0);
        metroid_ai(&mut m, 0);
        assert_eq!(m.enemy(0).speed_x(), -1, "幅值低字节 $00 < max ⇒ 不钳位");
        assert_eq!(
            m.enemy(0).speed_subpixel_x(),
            0xD0,
            "未钳位 ⇒ 小数字节原样保留（ROM 不在未钳位径清小数）"
        );
    }

    /// 纯函数级矢量：`clamp_axis(-1, $D0, 2) == (-1, $D0)`；`clamp_axis(-3, 0, 2) == (-2, 0)`。
    #[test]
    fn clamp_axis_byte_form_vectors() {
        assert_eq!(clamp_axis(-1, 0xD0, 2), (-1, 0xD0));
        assert_eq!(clamp_axis(-3, 0, 2), (-2, 0));
        assert_eq!(clamp_axis(-1, 0, 2), (-1, 0), "$FF/0 ⇒ 幅值 $01 < 2 ⇒ 不钳位");
        assert_eq!(clamp_axis(5, 0x80, 2), (2, 0), "正速径不受影响");
    }

    /// asm:96-107 —— 横加速用 `data05` bit0、竖加速用 bit2；red 的 ±$30。
    #[test]
    fn accel_signs_follow_data05_facing_bits() {
        let mut m = mgr();
        m.enemy(0).set_data05(0x01); // bit0=1 面向左
        metroid_ai(&mut m, 0);
        assert_eq!(m.enemy(0).accel_x(), -0x30, "bit0 置 ⇒ 横加速取负");
        assert_eq!(m.enemy(0).accel_y(), 0x30, "bit2 清 ⇒ 竖加速为正");
    }

    /// asm:96-107 —— green Metroid（`movementIndex=1`）的加速是 ±$60、限速 4。
    #[test]
    fn green_metroid_uses_green_tables() {
        let mut m = mgr();
        m.enemy(0).set_movement_index(1);
        m.enemy(0).set_data05(0x04); // bit2=1 面向上
        m.enemy(0).set_speed_x(9);
        metroid_ai(&mut m, 0);
        assert_eq!(m.enemy(0).accel_y(), -0x60, "green 竖加速取负");
        assert_eq!(m.enemy(0).speed_x(), 4, "green max speed = 4（9 被钳到 4）");
    }

    /// asm:110-127 —— 非冻结 ⇒ 血量恒置 `$FF`（无敌）；冻结且仍是无敌值 ⇒ 给 5 点血。
    #[test]
    fn frozen_metroid_gets_five_health() {
        let mut m = mgr();
        m.enemy(0).set_status(EnemyStatus::Frozen);
        m.enemy(0).set_health(0xFF);
        metroid_ai(&mut m, 0);
        assert_eq!(m.enemy(0).health(), 5, "冻结时无敌值 ⇒ 5 点血");

        let mut m2 = mgr();
        m2.enemy(0).set_status(EnemyStatus::Frozen);
        m2.enemy(0).set_health(3);
        metroid_ai(&mut m2, 0);
        assert_eq!(m2.enemy(0).health(), 3, "已不是无敌值 ⇒ 保持");
    }

    /// asm:@latchActive —— AI 前快照是 Hurt 且当前已不是 Hurt ⇒ 冻结。
    #[test]
    fn hurt_to_not_hurt_freezes_metroid() {
        let mut m = mgr();
        m.enemy(0).set_health(0x20);
        crate::frame::set_enemy_status_pre_ai(&mut m, EnemyStatus::Hurt.to_byte());
        metroid_ai(&mut m, 0);
        assert_eq!(
            m.enemy(0).status_raw(),
            EnemyStatus::Frozen.to_byte(),
            "受伤停止那一帧 ⇒ Frozen"
        );
    }

    /// asm:130-172 —— 已吸附 + 炸弹命中（`wa_BombExplode`）+ FrameCount bit1 清
    /// ⇒ 锁存 bits4-6 加 `$10`；第 5 次（`& $70 == $50`）⇒ 脱离吸附三件套。
    #[test]
    fn five_bomb_hits_release_latch() {
        let mut m = mgr();
        m.set_metroid_latch(0, 0x41); // 4 次命中已计（bits4-6 = 4）且低 nibble=1
        m.enemy(0).set_is_hit(0x20);
        m.enemy(0).set_weapon_action(0x0A); // wa_BombExplode
        m.write_ram(crate::ram_map::FRAME_COUNT, 0x00); // bit1 清
        metroid_ai(&mut m, 0);
        assert_eq!(
            m.enemy(0).status_raw(),
            EnemyStatus::Hurt.to_byte(),
            "第五次炸弹命中 ⇒ status=Hurt（脱离）"
        );
        assert_eq!(m.enemy(0).special_attribs(), 0x20, "specialAttribs←$20");
        assert_eq!(m.enemy(0).data0d(), 1, "data0D←1");
        assert_eq!(m.metroid_latch(0), 0, "脱离后锁存清零");
    }

    /// asm:135-139 —— 非炸弹武器（普通束）不计入吸附次数。
    #[test]
    fn non_bomb_hit_does_not_count_toward_latch() {
        let mut m = mgr();
        m.set_metroid_latch(0, 0x41);
        m.enemy(0).set_is_hit(0x20);
        m.enemy(0).set_weapon_action(0x01); // 普通束
        metroid_ai(&mut m, 0);
        assert_eq!(m.enemy(0).status_raw(), EnemyStatus::Active.to_byte());
        assert_eq!(m.metroid_latch(0), 0, "命中后仍会清锁存（asm 174-177）");
    }

    /// asm:174-188 —— 放开 Samus 时把速度设成**反向弹开**：accelX=+$30 ⇒ repell=-8。
    #[test]
    fn release_sets_repel_speed() {
        let mut m = mgr();
        m.set_metroid_latch(0, 0x21);
        m.enemy(0).set_accel_x(0x30);
        m.enemy(0).set_accel_y(0x30);
        m.enemy(0).set_is_hit(0x20);
        m.enemy(0).set_weapon_action(0x01);
        metroid_ai(&mut m, 0);
        assert_eq!(m.enemy(0).speed_x(), -8, "accel bit6=0 ⇒ repel $-08");
        assert_eq!(m.enemy(0).speed_y(), -8);
    }

    /// asm:192-208 —— 未吸附 + 碰到 Samus（isHit bit2）⇒ 锁存 `(speedX & $80) | $01` 并清速度。
    #[test]
    fn touching_samus_starts_latch() {
        let mut m = mgr();
        m.enemy(0).set_is_hit(0x04);
        m.enemy(0).set_speed_x(-4);
        m.enemy(0).set_speed_y(3);
        metroid_ai(&mut m, 0);
        // 同帧 updateLatch 会把低 nibble 从 1 推到 2，bit7（速度负）保留
        assert_eq!(m.metroid_latch(0), 0x82, "锁存 bit7 记速度符号且进度 +1");
        assert_eq!(m.enemy(0).speed_x(), 0, "ClearMetroidSpeed 清速度");
        assert_eq!(m.enemy(0).speed_y(), 0);
    }

    /// asm:210-250 —— 吸附中位置贴合 Samus：Samus(0x80,0x80) + (oy=0, ox=$0B) ⇒
    /// 敌位置 = (0x80, 0x8B)（锁存 bit7 置位时 x 偏移不取负）。
    #[test]
    fn latched_metroid_snaps_to_samus_position() {
        let mut m = mgr();
        m.set_metroid_latch(0, 0x81); // 进度 1、bit7 置（不取负）
        m.samus().set_y(0x80);
        m.samus().set_x(0x80);
        metroid_ai(&mut m, 0);
        assert_eq!(m.enemy(0).y(), 0x80, "y 偏移表[0] = 0");
        assert_eq!(m.enemy(0).pos_x(), 0x8B, "x 偏移 = 12 - 1 = $0B");
    }

    /// asm:292-307 —— 有完全吸附（`latch & $0C == $0C`）且 Samus 未死 ⇒
    /// `HealthChange ← $0004` + `MetroidOnSamus ← 1`。
    #[test]
    fn fully_latched_metroid_drains_health() {
        let mut m = mgr();
        m.set_metroid_latch(0, 0x0C); // 进度满
        m.samus().set_health(0x0100);
        metroid_ai(&mut m, 0);
        assert_eq!(m.read_ram(METROID_ON_SAMUS), 1, "MetroidOnSamus ← 1");
        assert!(
            m.samus().health() < 0x0100,
            "完全吸附 ⇒ 抽血（健康值应下降）"
        );
    }

    /// asm:301-307 —— `ObjectCntrl` bit7 清 ⇒ 置 `type | $82 | $20`。
    ///
    /// 观测窗：正常帧里这个值会被绘制尾件 `ClearObjectCntrl ; 07:DF2D` 消费掉（存活到帧末
    /// 是 0），所以这里用「已被逃逸计时器删除（status=0 ⇒ noMove 径不绘制）」的帧来观测写入值。
    #[test]
    fn object_cntrl_gets_priority_and_palette_bits() {
        let mut m = mgr();
        m.write_ram(END_TIMER + 1, 0xFE); // ⇒ status ← 0（删除），本帧不绘制
        m.enemy(0).set_enemy_type(0x00);
        m.set_obj_cntrl(0x00);
        metroid_ai(&mut m, 0);
        assert_eq!(m.obj_cntrl(), 0x82 | 0x20, "type|$82|OAMDATA_PRIORITY");
    }

    /// 对照：正常帧结束时 `ObjectCntrl` 被绘制尾件清回 0（`ClearObjectCntrl`）。
    #[test]
    fn object_cntrl_is_consumed_by_draw_tail() {
        let mut m = mgr();
        m.set_obj_cntrl(0x00);
        metroid_ai(&mut m, 0);
        assert_eq!(m.obj_cntrl(), 0, "绘制尾件清覆盖态");
    }

    /// `GetMetroidAccel` 的表映射与方向位（纯函数直测，防表项被误改）。
    #[test]
    fn accel_table_mapping() {
        assert_eq!(metroid_accel(0, false), 0x30);
        assert_eq!(metroid_accel(0, true), -0x30);
        assert_eq!(metroid_accel(1, false), 0x60);
        assert_eq!(metroid_accel(1, true), -0x60);
        assert_eq!(metroid_repel_speed(0x30), -8, "$30 bit6=0 ⇒ $-08");
        assert_eq!(metroid_repel_speed(-0x30), 8, "$D0 bit6=1 ⇒ $+08");
    }
}
