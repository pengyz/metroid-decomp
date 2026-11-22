//! Squeept（熔岩跳虫）—— `refs/m1disasm/SRC/enemies/squeept.asm`（55 行 / 42 条指令，
//! 逐行对读）。asm 原文（bank2 `02:9A64`；**只有 Norfair include 本件**——
//! `refs/m1disasm/SRC/prg2_norfair.asm:675 .include "enemies/squeept.asm"`）：
//! ```text
//! SqueeptAIRoutine_{AREA}:
//!     lda EnemyStatusPreAI / cmp #enemyStatus_Resting / bne @endIf_A
//!     lda EnsExtra.0.status / cmp #enemyStatus_Explode / beq Exit_Resting
//!     cmp #enemyStatus_Active / bne @endIf_A
//!         ldy Ens.0.movementIndex / lda SqueeptSpeedYTable,y / sta speedY
//!         lda #$40 / sta accelY / lda #$00 / sta speedSubPixelY
//! @endIf_A:
//!     lda status / cmp #Explode / beq Exit_Resting          ; 爆走 Resting 出口
//!     lda PreAI / cmp #Resting / beq Exit_Resting
//!     cmp #Explode / beq Exit_Explode
//!     jsr CommonJump_EnemyGetDeltaY_UsingAcceleration       ; dy → $00
//!     lda #$00 / sta Temp05_SpeedX
//!     lda #EnAnim_SqueeptJumping - EnAnimTable              ; dy<0 ⇒ Jumping 0x1D
//!     ldy $00 / sty Temp04_SpeedY / bmi @endIf_B
//!         lda #EnAnim_SqueeptFalling - EnAnimTable          ; dy≥0 ⇒ Falling 0x20
//! @endIf_B: sta resetAnimIndex
//!     jsr StoreEnemyPositionToTemp / jsr CommonJump_ApplySpeedToPosition
//!     lda #$E8 / bcc @then_C / cmp Temp08_PositionY / bcs @endIf_C
//! @then_C: sta Temp08_PositionY ; data05 |= #$20            ; 触发回 Resting
//! @endIf_C: jsr LoadEnemyPositionFromTemp                   ; 恒执行
//! Exit_Resting:  lda #$02 / jmp CommonJump_UpdateEnemyCommon_noMove
//! Exit_Explode:  jmp CommonJump_UpdateEnemyCommon_noMoveNoAnim
//! SqueeptSpeedYTable_{AREA}: ; 02:9AD2  .byte $F6,$F8,$F6,$FA
//! ```
//!
//! 语义要点（易错处）：
//! - `accelY #$40` = **有符号 +64**（向下重力，与 Rio 的 −128 相反）。
//! - **首块读现役 status 与 PreAI 快照的组合**：`pre==Resting` 且现役 `Active`
//!   才初始化跳起（`try_becoming_active` 已把现役改 2 后才调 AI ⇒ 本帧可达）。
//! - 爆走（现役 Explode）出口是 **Resting 出口（anim2）**，不是 noMoveNoAnim——
//!   只有 `pre==Explode` 才走 Explode 出口。
//! - 熔岩判：`ApplySpeedToPosition` **失败（越界 carry）也进 @then_C**（`bcc`）；
//!   成功时 `$E8 < newY`（已沉到熔岩线以下）也进。两种情况都把 y 钉到 `$E8`
//!   （熔岩面）并 `data05 |= #$20`；**`LoadEnemyPositionFromTemp` 恒执行** ⇒
//!   越界径的环绕修正值也要写回敌面（用 `apply_speed_to_position_parts` 拆分变体）。

use crate::enemy::framework;
use crate::enemy::motion::{apply_speed_to_position_parts, enemy_get_delta_y_using_acceleration};
use crate::ram_map;

/// `EnemyStatusPreAI`（`$0081`）/ `EnsExtra.status` 的取值（asm `cmp #enemyStatus_*`）。
const PRE_RESTING: u8 = 1;
const ENEMY_ACTIVE: u8 = 2;
const ENEMY_EXPLODE: u8 = 3;
const PRE_EXPLODE: u8 = 3;

/// 熔岩面 y（asm `lda #$E8`——直接字面量，非表数据）。
const LAVA_Y: u8 = 0xE8;

/// `resetAnimIndex` 两个字面量（asm `#EnAnim_Squeept* - EnAnimTable`，`02:9BF7/9BFA`
/// 相对 `EnAnimTable_NORFAIR 02:9BDA` 的偏移；代码即时量，非数据段）。
const ANIM_OFF_JUMPING: u8 = 0x1D;
const ANIM_OFF_FALLING: u8 = 0x20;

/// `SqueeptAIRoutine`（`enemies/squeept.asm`，仅 Norfair 生成）。
pub(crate) fn squeept_ai(mem: &mut crate::frame::Manager, x: u8) {
    let slot = x / 16;
    let pre = mem.enemy_status_pre_ai();
    let status = mem.enemy(slot).status_raw();

    if pre == PRE_RESTING {
        // asm 首块：现役 Explode ⇒ Resting 出口（爆走先于 Active 判）。
        if status == ENEMY_EXPLODE {
            framework::update_enemy_common_no_move_anim(mem, x, 0x02);
            return;
        }
        // 现役 Active ⇒ 初始化跳起（激活帧：try_becoming_active 已置 2）。
        if status == ENEMY_ACTIVE {
            // `ldy movementIndex / lda SqueeptSpeedYTable,y / sta speedY`
            let mi = mem.enemy(slot).movement_index() as usize;
            let v = game_data::assets::squeept_speed_y(crate::frame::cur_area(mem))[mi] as i8;
            mem.enemy(slot).set_speed_y(v);
            // `lda #$40 / sta accelY`（+64 向下重力）
            mem.enemy(slot).set_accel_y(64);
            // `lda #$00 / sta speedSubPixelY`
            mem.enemy(slot).set_data06(0);
        }
    }

    // @endIf_A：现役 Explode ⇒ Resting 出口（anim2）。
    if status == ENEMY_EXPLODE {
        framework::update_enemy_common_no_move_anim(mem, x, 0x02);
        return;
    }
    if pre == PRE_RESTING {
        framework::update_enemy_common_no_move_anim(mem, x, 0x02);
        return;
    }
    if pre == PRE_EXPLODE {
        framework::update_enemy_common_no_move_no_anim(mem, x);
        return;
    }

    // 活动径：重力推进 + 跳/落动画选择。
    let dy = enemy_get_delta_y_using_acceleration(mem, x);
    let dx: i8 = 0;
    // `lda #Jumping / bmi @endIf_B ⇒ dy≥0 换 Falling`（dy<0 上升 ⇒ Jumping）。
    let anim_off = if dy < 0 { ANIM_OFF_JUMPING } else { ANIM_OFF_FALLING };
    mem.enemy(slot).set_reset_anim_index(anim_off);

    // `StoreEnemyPositionToTemp / ApplySpeedToPosition`——拆分变体拿暂存值
    // （失败也要 `LoadEnemyPositionFromTemp` 落盘，见模块注释）。
    let (ok, mut ny, nx, hi) = apply_speed_to_position_parts(mem, x, dy, dx);
    // `lda #$E8 / bcc @then_C / cmp Temp08_PositionY / bcs @endIf_C`
    // 失败（越界）或 `$E8 < newY`（低于熔岩）⇒ y 钉到熔岩面 + data05 bit5 置。
    if !ok || LAVA_Y < ny {
        ny = LAVA_Y;
        let d5 = mem.enemy(slot).data05();
        mem.enemy(slot).set_data05(d5 | 0x20);
    }
    // `LoadEnemyPositionFromTemp` 恒执行。
    mem.enemy(slot).set_y(ny);
    mem.enemy(slot).set_pos_x(nx);
    mem.enemy(slot).set_hi(hi);
    let _ = ok;
    // 【尾件（2026-11-22 修）】asm `SqueeptExit_Resting ; 02:9ACA`：活动径落盘后
    // **直落**该出口——`lda #$02 / jmp CommonJump_UpdateEnemyCommon_noMove`（`$02`=动画
    // 每 2 帧一步）。旧形活动径到此直接结束 ⇒ 槽既不走 anim 也不绘制 ⇒ Norfair 多房里
    // Squeept 整只不显示（sprite 层小面差、RAM 面 animDelay 停在 $02/animIndex 差 1）。
    framework::update_enemy_common_no_move_anim(mem, x, 0x02);
}

#[cfg(test)]
mod tests_squeept {
    use super::*;
    use crate::enemy::prep::{EN_F_DATA05, ENS_F_STATUS, ENS_F_TYPE};
    use crate::enemy::tables::{en_addr, ens_addr};
    use crate::frame::Manager;

    const X: u8 = 0x20;

    fn stage(m: &mut Manager, pre: u8, status: u8, mov_idx: u8) {
        game_data::pack::init_from_env().ok();
        m.write_ram(ram_map::IN_AREA, 1); // Norfair
        m.write_ram(ram_map::ENEMY_STATUS_PRE_AI, pre);
        m.write_wram(ens_addr(X, ENS_F_STATUS), status);
        m.write_wram(ens_addr(X, ENS_F_TYPE), 0x0B); // Squeept
        m.enemy(X / 16).set_movement_index(mov_idx);
        m.enemy(X / 16).set_y(0x80);
        m.enemy(X / 16).set_pos_x(0x50);
    }

    /// 激活帧（pre=Resting、现役 Active）：跳起初速按 movementIndex 查表
    /// `SqueeptSpeedYTable ; 02:9AD2`（F6 F8 F6 FA，assets 段）+ accelY=+64 + 亚像素清零。
    #[test]
    fn activation_loads_jump_velocity_from_assets_table() {
        let mut m = Manager::default();
        stage(&mut m, PRE_RESTING, ENEMY_ACTIVE, 1);
        m.enemy(X / 16).set_accel_y(0);
        squeept_ai(&mut m, X);
        assert_eq!(m.enemy(X / 16).speed_y(), -8, "movIdx=1 ⇒ $F8(−8)");
        assert_eq!(m.enemy(X / 16).accel_y(), 64, "accelY=$40（+64 向下重力）");
        assert_eq!(m.enemy(X / 16).data06(), 0, "speedSubPixelY 清零");
    }

    /// 表的另两项：movIdx=0/2 都是 $F6(−10)、movIdx=3 是 $FA(−6)——四项全走 assets。
    #[test]
    fn jump_table_all_four_entries() {
        for (mi, want) in [(0u8, -10i8), (2, -10), (3, -6)] {
            let mut m = Manager::default();
            stage(&mut m, PRE_RESTING, ENEMY_ACTIVE, mi);
            squeept_ai(&mut m, X);
            assert_eq!(m.enemy(X / 16).speed_y(), want, "movIdx={mi}");
        }
    }

    /// 现役 Explode 且 pre==Resting ⇒ **Resting 出口（anim2）**，且不初始化跳速。
    #[test]
    fn explode_while_resting_takes_resting_exit() {
        let mut m = Manager::default();
        stage(&mut m, PRE_RESTING, ENEMY_EXPLODE, 1);
        m.enemy(X / 16).set_speed_y(7);
        squeept_ai(&mut m, X);
        assert_eq!(m.enemy(X / 16).speed_y(), 7, "爆走不初始化跳速");
    }

    /// pre==Explode ⇒ Explode 出口（noMoveNoAnim——不 anim2）。
    #[test]
    fn pre_explode_takes_explode_exit() {
        let mut m = Manager::default();
        stage(&mut m, PRE_EXPLODE, ENEMY_ACTIVE, 1);
        m.enemy(X / 16).set_speed_y(7);
        squeept_ai(&mut m, X);
        // Explode 出口不写 resetAnimIndex（活动径才会写 0x1D/0x20）。
        assert_eq!(m.enemy(X / 16).reset_anim_index(), 0);
    }

    /// 非 Resting 期活动径：dy≥0（下落）⇒ resetAnimIndex=0x20（Falling），
    /// 位置在熔岩线上方 ⇒ 原样落盘、data05 bit5 不置。
    #[test]
    fn active_falling_above_lava_keeps_position() {
        let mut m = Manager::default();
        stage(&mut m, ENEMY_ACTIVE, ENEMY_ACTIVE, 1);
        // speedY 正（下落）+ accelY 正 ⇒ dy≥0；y=0x80 远在 $E8 之上。
        m.enemy(X / 16).set_speed_y(4);
        m.enemy(X / 16).set_data06(0);
        squeept_ai(&mut m, X);
        assert_eq!(m.enemy(X / 16).reset_anim_index(), 0x20, "下落 ⇒ Falling 0x20");
        assert_eq!(m.enemy(X / 16).y(), 0x84, "speedY=4 一步 ⇒ y+4（未到熔岩）");
        assert_eq!(m.enemy(X / 16).data05() & 0x20, 0, "bit5 不置");
    }

    /// 活动径 dy<0（上升）⇒ resetAnimIndex=0x1D（Jumping）。
    #[test]
    fn active_rising_uses_jumping_anim() {
        let mut m = Manager::default();
        stage(&mut m, ENEMY_ACTIVE, ENEMY_ACTIVE, 1);
        m.enemy(X / 16).set_speed_y(-4);
        m.enemy(X / 16).set_accel_y(0); // 无重力
        m.enemy(X / 16).set_data06(0xFF); // 亚像素满 ⇒ −1 步进得 dy<0
        squeept_ai(&mut m, X);
        assert_eq!(m.enemy(X / 16).reset_anim_index(), 0x1D, "上升 ⇒ Jumping 0x1D");
    }

    /// 熔岩回吸：y 沉过 `$E8` ⇒ y 钉回 $E8、data05 bit5 置（触发回 Resting）。
    #[test]
    fn below_lava_snaps_to_lava_and_sets_bit5() {
        let mut m = Manager::default();
        stage(&mut m, ENEMY_ACTIVE, ENEMY_ACTIVE, 1);
        m.enemy(X / 16).set_speed_y(8);
        m.enemy(X / 16).set_data06(0);
        m.enemy(X / 16).set_y(0xE4); // +8 ⇒ 0xEC > $E8
        squeept_ai(&mut m, X);
        assert_eq!(m.enemy(X / 16).y(), LAVA_Y, "低于熔岩 ⇒ 钉 $E8");
        assert_eq!(m.enemy(X / 16).data05() & 0x20, 0x20, "bit5 置 ⇒ 回 Resting");
    }

    /// 恰在熔岩线（newY==$E8）：`cmp Temp08 / bcs @endIf_C`——相等不进 @then_C
    /// ⇒ bit5 不置（asm `bcs` 含等）。
    #[test]
    fn exactly_at_lava_does_not_snap() {
        let mut m = Manager::default();
        stage(&mut m, ENEMY_ACTIVE, ENEMY_ACTIVE, 1);
        m.enemy(X / 16).set_speed_y(8);
        m.enemy(X / 16).set_data06(0);
        m.enemy(X / 16).set_y(0xE0); // +8 ⇒ 0xE8 恰等
        squeept_ai(&mut m, X);
        assert_eq!(m.enemy(X / 16).y(), 0xE8);
        assert_eq!(m.enemy(X / 16).data05() & 0x20, 0, "恰等 ⇒ 不置 bit5");
    }

    /// 活动径尾件（asm `SqueeptExit_Resting ; 02:9ACA` 的 `lda #$02 / jmp
    /// CommonJump_UpdateEnemyCommon_noMove`）：活动径必须**穿过**该出口——证据是绘径写的
    /// `data05` bit1 可见旗 + OAM 有实际写入。旧形活动径直接结束 ⇒ 两者皆无。
    #[test]
    fn active_path_falls_through_to_no_move_tail() {
        let mut m = Manager::default();
        game_data::pack::init_from_env().ok();
        stage(&mut m, ENEMY_ACTIVE, ENEMY_ACTIVE, 1);
        m.write_ram(ram_map::SCROLL_X, 0);
        m.write_ram(ram_map::SCROLL_DIR, 0x02); // 横滚房
        m.enemy(X / 16).set_anim_frame(0); // 有效帧（0xFF 会被绘径直接跳过）
        m.enemy(X / 16).set_speed_y(4);
        m.enemy(X / 16).set_data06(0);
        m.enemy(X / 16).set_data05(0);
        squeept_ai(&mut m, X);
        assert_eq!(m.enemy(X / 16).data05() & 0x02, 0x02, "绘径可见旗 bit1 置");
        assert_ne!(m.read_ram(0x0200), 0, "OAM 有实际写入（y 字节非 0）");
    }

    /// dispatch 级接线：type 0x0B（Norfair Squeept）走真实链。
    #[test]
    fn dispatch_wiring_reaches_squeept_for_type_0x0b() {
        let mut m = Manager::default();
        game_data::pack::init_from_env().ok();
        m.write_ram(ram_map::IN_AREA, 1);
        m.write_wram(ens_addr(X, ENS_F_STATUS), 0x02);
        m.write_wram(ens_addr(X, ENS_F_TYPE), 0x0B);
        m.write_ram(en_addr(X, 0), 0x40);
        m.write_ram(en_addr(X, 1), 0x50);
        m.write_ram(en_addr(X, EN_F_DATA05), 0x02);
        m.enemy(X / 16).set_movement_index(1);
        m.enemy(X / 16).set_speed_y(4);
        m.enemy(X / 16).set_data06(0);
        crate::enemy::slot_cycle::update_all_enemies(&mut m);
        // 活动径标记：resetAnimIndex ← Falling 0x20（只有 Squeept 例程写这个字面量）。
        assert_eq!(m.enemy(X / 16).reset_anim_index(), 0x20, "经 choose_enemy_ai 到 Squeept 径");
    }
}
