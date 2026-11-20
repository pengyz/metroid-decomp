//! Rinka（Tourian 的浮游射击弹球敌）AI。
//!
//! **asm 内容锚**：`RinkaAIRoutine_{AREA}: ; 03:9A2C`（`refs/m1disasm/SRC/enemies/rinka.asm`
//! 全文 222 行）——它只被 Tourian bank include（`prg3_tourian.asm` 的敌表项引用），
//! 且 `SetRinkaSpeed_{AREA}` / `LoadEnemyPositionFromTemp_` 是**按区**标签（`{AREA}` 宏）。
//!
//! ## 行为骨架（逐行对读 asm）
//!
//! 1. `ldy EnsExtra.status / cpy #enemyStatus_Active / bne @moveRinka` —— 非 Active 直接落地移动段。
//! 2. `dey`（把 Active(2) 折成 1）；`cpy EnemyStatusPreAI / bne @moveRinka` —— **只有
//!    「本帧状态 Active 且 AI 前快照是 Resting」那一帧**才做速度初始化。这是 Rinka 唯一
//!    的定速时刻：写成「每帧朝 Samus 飞」会让行为持续发散（`docs/spec/tourian-endgame.md` §13）。
//! 3. 定速帧：`ClearRinkaAcceleration`（清 `accelX/accelY`，asm 自注「无用，因为 Rinka 不走
//!    加速模型」）+ 清 `subPixelX/Y`；然后算 **相对 Samus 的位移向量**并按「面向」取符号：
//!    `data05` bit0 = 水平面向（1=朝左 ⇒ 取负），bit2 = 垂直面向（1=朝上 ⇒ 取负）。
//! 4. 归一化：把 `|Δx|`、`|Δy|` 一起右移，直到两者都没有 bit7/6/5（即都 < $20）。
//!    asm 用「OR 后找最高置位算 y」再 `dey;bmi` 循环 y 次——等价于「右移到都 < $20」。
//! 5. `SetRinkaSpeed_{AREA}`：高 4 位 → `speedY/X`（整数），低 4 位左移 4 → `speedSubPixelY/X`
//!    （8.4 定点的小数部分）。
//! 6. 按面向位取负（`lda #$00 / sbc …` 的补码取负，含小数部分）。
//! 7. `@moveRinka`：`lda data05 / asl / bmi @endIf_E` —— **原字节 bit6 置位就整帧不移动**
//!    （30FPS 门）。否则把定点速度累加进 `subPixelY/X`，进位进 `speedY/X`，再调
//!    `ApplySpeedToPosition`（`07:FD8F`，越界环绕语义见 `motion::apply_speed_to_position_parts`）。
//!    失败 ⇒ `status ← 0`（原版直接删掉这只 Rinka）；随后 `LoadEnemyPositionFromTemp_`
//!    **恒**把暂存取回敌面（含失败时的 ±$10 环绕修正值）。
//! 8. 尾件：`lda #$08 / jmp CommonJump_UpdateEnemyCommon_noMove` —— 每 8 帧换一次动画帧。

use crate::enemy::framework::update_enemy_common_no_move_anim;
use crate::enemy::motion::apply_speed_to_position_parts;
use crate::enemy::state::{slot_of, EnemyStatus};
use crate::frame::Manager;

/// `enemyStatus_Active`（`refs/m1disasm/SRC/constants.asm:135-142`）。
const ENEMY_STATUS_ACTIVE: u8 = 2;
/// `enemyStatus_Resting`——Rinka 的定速判据比较的是「AI 前快照」的这一个值。
const ENEMY_STATUS_RESTING: u8 = 1;
/// `@endIf_E` 尾件 `lda #$08`：动画帧每 8 帧推进一次。
const RINKA_ANIM_BEAT: u8 = 8;

/// Rinka AI（`RinkaAIRoutine_{AREA} ; 03:9A2C`）。`x` 是敌槽的**字节偏移**（= 槽号 × 16）。
pub(crate) fn rinka_ai(mem: &mut Manager, x: u8) {
    let slot = slot_of(x);
    let status = mem.enemy(slot).status_raw();
    let pre_ai = mem.enemy_status_pre_ai();

    // asm:1-2 —— 两条「非定速帧」早退径
    let init_frame = status == ENEMY_STATUS_ACTIVE && pre_ai == ENEMY_STATUS_RESTING;
    if init_frame {
        // asm:12-16 `ClearRinkaAcceleration` + 清 sub-pixel
        mem.enemy(slot).set_accel_x(0);
        mem.enemy(slot).set_accel_y(0);
        mem.enemy(slot).set_subpixel_y(0);
        mem.enemy(slot).set_subpixel_x(0);

        // asm:19-70 —— 相对位移向量（按 data05 的面向位取符号）
        let data05 = mem.enemy(slot).data05();
        let samus_x = mem.samus().x();
        let samus_y = mem.samus().y();
        let enemy_x = mem.enemy(slot).pos_x();
        let enemy_y = mem.enemy(slot).y();
        // $01 = Samus.x − Ens.x；facing 左（bit0=1）时取负
        let mut dist_x = samus_x.wrapping_sub(enemy_x);
        if data05 & 0x01 != 0 {
            dist_x = 0u8.wrapping_sub(dist_x);
        }
        // $00 = Samus.y − Ens.y；facing 上（bit2=1）时取负
        let mut dist_y = samus_y.wrapping_sub(enemy_y);
        if data05 & 0x04 != 0 {
            dist_y = 0u8.wrapping_sub(dist_y);
        }

        // asm:72-83 —— 两者都右移到「没有 bit7/6/5」（即都 < $20）
        while dist_y | dist_x >= 0x20 {
            dist_y >>= 1;
            dist_x >>= 1;
        }

        // asm:86-88 `jsr SetRinkaSpeed_{AREA}`：高 4 位进整数速度、低 4 位左移 4 进小数
        let mut speed_y = (dist_y >> 4) as i8;
        let mut sub_y = dist_y << 4;
        let mut speed_x = (dist_x >> 4) as i8;
        let mut sub_x = dist_x << 4;

        // asm:91-113 —— 面向位取负（含小数部分）。
        //
        // **原版 quirk（D4 保真，不许"修"）**：面向位在第一段（`$01/$00`）已经取过一次负，
        // 而归一化用的是 `lsr`（**逻辑**右移，对补码值是错算），所以第二段再取负并不回到
        // 原值。实测 Δx=+$20 面向左：`$01=$E0` → 3 次 lsr ⇒ `$1C` ⇒ speed=$1、sub=$C0 ⇒
        // 再取负 ⇒ speed=−1、sub=$40（即 -1.25 px/帧），而面向右同距离是 1.00 px/帧。
        // `tests_rinka::facing_left_keeps_rom_double_negation_quirk` 把这个不对称钉死。
        if data05 & 0x01 != 0 {
            sub_x = 0u8.wrapping_sub(sub_x);
            speed_x = 0i8.wrapping_sub(speed_x);
        }
        if data05 & 0x04 != 0 {
            sub_y = 0u8.wrapping_sub(sub_y);
            speed_y = 0i8.wrapping_sub(speed_y);
        }
        mem.enemy(slot).set_speed_y(speed_y);
        mem.enemy(slot).set_speed_subpixel_y(sub_y);
        mem.enemy(slot).set_speed_x(speed_x);
        mem.enemy(slot).set_speed_subpixel_x(sub_x);
    }

    // asm:@moveRinka `03:9AB0` —— `lda data05 / asl / bmi` 测**原字节 bit6**（30FPS 门）
    let data05 = mem.enemy(slot).data05();
    if data05 & 0x40 == 0 {
        // 8.4 定点：小数累加，进位进整数速度（asm 用 `lda speed / adc #$00` 取进位）
        let (sub_y, carry_y) = mem
            .enemy(slot)
            .subpixel_y()
            .overflowing_add(mem.enemy(slot).speed_subpixel_y());
        mem.enemy(slot).set_subpixel_y(sub_y);
        let dy = mem.enemy(slot).speed_y().wrapping_add(carry_y as i8);

        let (sub_x, carry_x) = mem
            .enemy(slot)
            .subpixel_x()
            .overflowing_add(mem.enemy(slot).speed_subpixel_x());
        mem.enemy(slot).set_subpixel_x(sub_x);
        let dx = mem.enemy(slot).speed_x().wrapping_add(carry_x as i8);

        // `jsr CommonJump_ApplySpeedToPosition` + 恒执行的 `LoadEnemyPositionFromTemp_`
        let (ok, ny, nx, hi) = apply_speed_to_position_parts(mem, x, dy, dx);
        mem.enemy(slot).set_y(ny);
        mem.enemy(slot).set_pos_x(nx);
        mem.enemy(slot).set_hi(hi & 0x01);
        if !ok {
            // asm:137-140 `lda #$00 / sta EnsExtra.0.status,x`（原版直接删这只 Rinka）
            mem.enemy(slot).set_status(EnemyStatus::Undefined(0));
        }
    }

    // asm:@endIf_E 尾件 `lda #$08 / jmp CommonJump_UpdateEnemyCommon_noMove`
    update_enemy_common_no_move_anim(mem, x, RINKA_ANIM_BEAT);
}

#[cfg(test)]
mod tests_rinka {
    use super::*;
    use crate::enemy::state::EnemyStatus;

    /// `EnemyStatusPreAI`（`$0081`）是 AI 分派前的状态快照——测试里直接摆值。
    /// （`frame/memory.rs` 目前只提供读口 `enemy_status_pre_ai`；写口在 stage 路径里，
    /// 这里用兼容面显式写，属测试面。）
    fn set_pre_ai(m: &mut Manager, v: u8) {
        crate::frame::set_enemy_status_pre_ai(m, v);
    }

    /// 造一个 Active Rinka：`x=0`（槽 0）、位置 (y,x)=(0x80,0x80)、data05 面向右/下。
    fn active_rinka() -> Manager {
        let mut m = Manager::default();
        m.enemy(0).set_status(EnemyStatus::Active);
        m.enemy(0).set_y(0x80);
        m.enemy(0).set_pos_x(0x80);
        m.enemy(0).set_data05(0x00);
        m
    }

    /// asm:1-2 —— 状态不是 Active 时**不得**定速（速度保持原值），但仍走移动段。
    #[test]
    fn non_active_rinka_does_not_initialize_speed() {
        let mut m = Manager::default();
        m.enemy(0).set_status(EnemyStatus::Resting);
        m.enemy(0).set_y(0x80);
        m.enemy(0).set_pos_x(0x80);
        m.enemy(0).set_speed_x(7);
        m.enemy(0).set_speed_y(9);
        m.write_ram(crate::ram_map::IN_AREA, 0x30); // Tourian（InArea 位 4 用于 CheckBlastTile）
        rinka_ai(&mut m, 0);
        assert_eq!(m.enemy(0).speed_x(), 7, "非 Active 不得改 speedX");
        assert_eq!(m.enemy(0).speed_y(), 9, "非 Active 不得改 speedY");
    }

    /// asm:2-3 —— Active 但 **AI 前快照不是 Resting** 时也不得定速（这是「只定速一次」的闸）。
    #[test]
    fn active_but_pre_ai_not_resting_does_not_initialize_speed() {
        let mut m = active_rinka();
        m.enemy(0).set_speed_x(-3);
        set_pre_ai(&mut m, 2); // 快照 Active（不是 Resting）
        rinka_ai(&mut m, 0);
        assert_eq!(m.enemy(0).speed_x(), -3, "快照非 Resting 不得定速");
    }

    /// 归一化边界：Δx=$1F（<$20）**不**右移 ⇒ speedX=1、小数位 $F0（`Amul16_` = <<4）。
    #[test]
    fn delta_below_0x20_keeps_subpixel_fraction() {
        let mut m = active_rinka();
        set_pre_ai(&mut m, 1);
        m.samus().set_x(0x80 + 0x1F);
        m.samus().set_y(0x80);
        rinka_ai(&mut m, 0);
        assert_eq!(m.enemy(0).speed_x(), 1, "$1F>>4 = 1");
        assert_eq!(m.enemy(0).speed_subpixel_x(), 0xF0, "$1F<<4 = $F0");
    }

    /// asm:19-88 + `SetRinkaSpeed` —— Active 且快照 Resting 的那一帧定速：
    /// Δx=$20 触发**一次**右移（`@loop_A/@loop_B` 对 bit7/6/5 归零）⇒ $10 ⇒ speedX=$10>>4=1。
    #[test]
    fn init_speed_from_normalized_delta() {
        let mut m = active_rinka();
        set_pre_ai(&mut m, 1);
        m.samus().set_x(0xA0); // Δx = +0x20
        m.samus().set_y(0x80); // Δy = 0
        rinka_ai(&mut m, 0);
        assert_eq!(m.enemy(0).speed_x(), 1, "$20 右移一次到 $10 ⇒ $10>>4=1");
        assert_eq!(m.enemy(0).speed_subpixel_x(), 0, "低 4 位为 0 ⇒ 小数部分 0");
        assert_eq!(m.enemy(0).speed_y(), 0);
        assert_eq!(m.enemy(0).accel_x(), 0, "ClearRinkaAcceleration 清 accelX");
        assert_eq!(m.enemy(0).accel_y(), 0, "ClearRinkaAcceleration 清 accelY");
    }

    /// asm:72-83 —— 位移 ≥ $20 要右移归一化：Δx=$40 ⇒ 两次右移 ⇒ $10 ⇒ speedX=1（不是 4）。
    #[test]
    fn large_delta_is_shifted_until_below_0x20() {
        let mut m = active_rinka();
        set_pre_ai(&mut m, 1);
        m.samus().set_x(0x80u8.wrapping_add(0x40));
        m.samus().set_y(0x80);
        rinka_ai(&mut m, 0);
        assert_eq!(m.enemy(0).speed_x(), 1, "$40 归一化到 $10 ⇒ speedX=1（未归一化会是 4）");
    }

    /// asm:91-113 —— 面向左（data05 bit0=1）时向量与速度都取负。
    #[test]
    fn facing_left_negates_x_speed() {
        let mut m = active_rinka();
        m.enemy(0).set_data05(0x01);
        set_pre_ai(&mut m, 1);
        m.samus().set_x(0xA0);
        m.samus().set_y(0x80);
        rinka_ai(&mut m, 0);
        assert_eq!(m.enemy(0).speed_x(), -1, "面向左 ⇒ speedX 取负");
    }

    /// asm:91-113 —— **原版双重取负 quirk**（不允许被"清理"成对称）：
    /// 面向左 + Δx=+$20 ⇒ 小数位 $40（−1.25 px/帧），而面向右同距离是 $00（1.00 px/帧）。
    #[test]
    fn facing_left_keeps_rom_double_negation_quirk() {
        let mut m = active_rinka();
        m.enemy(0).set_data05(0x01);
        set_pre_ai(&mut m, 1);
        m.samus().set_x(0xA0);
        m.samus().set_y(0x80);
        rinka_ai(&mut m, 0);
        assert_eq!(m.enemy(0).speed_x(), -1, "取负后整数位 -1");
        assert_eq!(
            m.enemy(0).speed_subpixel_x(),
            0x40,
            "逻辑右移后再取负 ⇒ 小数位 $40（不是对称的 $00）"
        );
    }

    /// asm:91-113 —— 面向上（data05 bit2=1）时 Y 速度取负。
    #[test]
    fn facing_up_negates_y_speed() {
        let mut m = active_rinka();
        m.enemy(0).set_data05(0x04);
        set_pre_ai(&mut m, 1);
        m.samus().set_x(0x80);
        m.samus().set_y(0xA0);
        rinka_ai(&mut m, 0);
        assert_eq!(m.enemy(0).speed_y(), -1, "面向上 ⇒ speedY 取负");
    }

    /// asm:119-129 —— `data05` bit6 置位（30FPS）⇒ 整帧不移动，但尾件照跑。
    #[test]
    fn bit6_gate_skips_movement_this_frame() {
        let mut m = active_rinka();
        m.enemy(0).set_data05(0x40);
        m.enemy(0).set_y(0x80);
        m.enemy(0).set_pos_x(0x80);
        set_pre_ai(&mut m, 2); // 不定速
        m.enemy(0).set_speed_x(0x10);
        rinka_ai(&mut m, 0);
        assert_eq!(m.enemy(0).pos_x(), 0x80, "bit6 置位不得移动");
    }

    /// asm:130-141 —— 移动失败（越界且方向不允许环绕）⇒ `status ← 0`。
    /// 竖滚房里 x 进位（水平越界）会失败：speedX=$10 且 x 已到 $F8 ⇒ x+$10 进位 ⇒ 失败。
    #[test]
    fn failed_move_removes_rinka() {
        let mut m = Manager::default();
        m.enemy(0).set_status(EnemyStatus::Active);
        m.enemy(0).set_data05(0x00);
        m.enemy(0).set_y(0x80);
        m.enemy(0).set_pos_x(0xF8);
        set_pre_ai(&mut m, 2); // 不定速
        m.enemy(0).set_speed_x(0x10);
        m.write_ram(crate::ram_map::SCROLL_DIR, 0x00); // 竖滚 ⇒ x 进位失败
        rinka_ai(&mut m, 0);
        assert_eq!(m.enemy(0).status_raw(), 0, "移动失败 ⇒ status=0（删除）");
    }

    /// asm:@endIf_E 尾件 —— 动画节拍恒为 8（`lda #$08`），且本件必须是 noMove 入口。
    #[test]
    fn anim_beat_is_eight_frames() {
        assert_eq!(RINKA_ANIM_BEAT, 8);
    }
}
