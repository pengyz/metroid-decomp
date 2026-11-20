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

//==============================================================================
// Rinka 生成器（`UpdateAllRinkaSpawners ; 03:A15E` + `SpawnRinkaSpawnerRoutine ; 03:9D6C`）
//==============================================================================

/// `RinkaSpawnPosTbl ; 03:A1DB`：12 B = 2 生成器 × 6 位置，每字节高 nibble=Y、低 nibble=X
/// （`SpawnRinka_InitPositionXY ; 03:9EE7` 把高 nibble 映射成 `(hi<<4)|7`、低 nibble 映射成
/// `(lo<<4)|7`）。生成器 `status` 选偶/奇列（0⇒偶、1⇒奇）。
const SPAWN_POS_TBL: [u8; 12] = [
    0x22, 0x2A, 0x2A, 0xBA, 0xB2, 0x2A, 0xC4, 0x2A, 0xC8, 0xBA, 0xBA, 0xBA,
];

/// 生成器槽数（`RinkaSpawners instanceof RinkaSpawner 2`）。
pub(crate) const SPAWNER_COUNT: u8 = 2;

/// `RinkaSpawnSpawner ; 07:EEEE` + `SpawnRinkaSpawnerRoutine ; 03:9D6C`：
/// 先试生成器 1、再试 0；`status` 的 bit7 置（`$FF`）表示空闲 ⇒ 写入
/// `status ← 特项字节 >> 4`（0/1，选偶数/奇数列）、`hi ← (PPUCTRL_ZP ^ ScrollDir) & 1`。
/// 返回 `$FF`（特项消耗），与 asm 出口 A 一致。
pub(crate) fn spawn_rinka_spawner(mem: &mut crate::frame::Manager, item_type: u8) -> u8 {
    for slot in [1u8, 0u8] {
        if mem.rinka_spawner_status(slot) < 0x80 {
            continue; // `bpl @RTS`：已有生成器（或本轮已建）⇒ 换下一个
        }
        mem.set_rinka_spawner_status(slot, item_type >> 4);
        let nt = (mem.read_ram(crate::ram_map::PPUCTRL_ZP) ^ mem.read_ram(crate::ram_map::SCROLL_DIR)) & 0x01;
        mem.set_rinka_spawner_hi(slot, nt);
        break;
    }
    0xFF
}

/// `SpawnRinka_InitPositionXY ; 03:9EE7`：`y = (byte & $F0) | $07`、`x = (byte << 4) | $07`。
fn init_rinka_position(mem: &mut crate::frame::Manager, x: u8, byte: u8) {
    let slot = x / 16;
    mem.enemy(slot).set_y((byte & 0xF0) | 0x07);
    mem.enemy(slot).set_pos_x(byte.wrapping_shl(4) | 0x07);
}

/// `CommonJump_0E ; 07:EB6E` 的净效果（`asl data05` + `InitEnemyFacing…` 的 `ror`）：
/// `data05 ← (d05 & $7F) | (L977B[type].bit7 << 7)`、`delay ← EnemyInitDelayTbl[type]`、
/// `data0D ← ForceSpeedDelay[type]`、`health ← HealthTbl[type]`（`specialAttribs` bit7 时双倍）。
fn common_jump_0e(mem: &mut crate::frame::Manager, x: u8) {
    let slot = x / 16;
    let t = mem.enemy(slot).enemy_type();
    let area = crate::frame::cur_area(mem);
    let raw977b = game_data::assets::enemy_facing(area)[(t & 0x3F) as usize];
    let d5 = (mem.enemy(slot).data05() & 0x7F) | if raw977b & 0x80 != 0 { 0x80 } else { 0 };
    mem.enemy(slot).set_data05(d5);
    mem.enemy(slot).set_delay(game_data::assets::enemy_init_delay(area)[t as usize]);
    mem.enemy(slot).set_data0d(game_data::assets::enemy_force_speed_delay(area)[t as usize]);
    let mut hp = game_data::assets::enemy_health(area)[t as usize];
    if mem.enemy(slot).special_attribs() & 0x80 != 0 {
        hp = hp.wrapping_shl(1);
    }
    mem.enemy(slot).set_health(hp);
}

/// `UpdateAllRinkaSpawners ; 03:A15E`：逃脱计时器停用（`EndTimer+1 == $FF`）时才跑；
/// 先处理生成器 1（`y=$03`）再处理 0；每个生成器四道门（`status` bit7 / `hi ^ FrameCount`
/// bit0 交替 / `MotherBrainStatus >= 4` / `FrameCount & $06`），命中就找前三个敌槽
/// （`x=$20,$10,$00`，空槽或不可见槽可复用）生成一枚 Rinka。
pub(crate) fn update_all_rinka_spawners(mem: &mut crate::frame::Manager) {
    if mem.end_timer_hi().wrapping_add(1) != 0 {
        return; // 计时器活动中：不再生成
    }
    for slot in [1u8, 0u8] {
        update_one_spawner(mem, sheet_off(slot));
    }
}

/// asm `PageIndex`：生成器 1 用 `$03`、生成器 0 用 `$00`（`RinkaSpawners.0.status,y` 的 y）。
fn sheet_off(slot: u8) -> u8 {
    if slot == 1 {
        0x03
    } else {
        0x00
    }
}

fn update_one_spawner(mem: &mut crate::frame::Manager, page_off: u8) {
    // `PageIndex = page_off` 时这些地址就是 `RinkaSpawners.{slot}.{status,hi,posIndex}`
    let slot = page_off / 3;
    let status = mem.read_ram(0x008B + page_off as u16);
    if status & 0x80 != 0 {
        return; // 生成器未启用
    }
    // `hi ^ FrameCount` 的 bit0：两生成器交替
    if (mem.rinka_spawner_hi(slot) ^ mem.frame_count()) & 0x01 == 0 {
        return;
    }
    if mem.mb_status() >= 0x04 {
        return; // 母脑将死/已死
    }
    if mem.frame_count() & 0x06 != 0 {
        return;
    }
    // 找空槽：status==0 或 data05 bit1 清（不可见）
    let mut target: Option<u8> = None;
    for x in [0x20u8, 0x10, 0x00] {
        let s = mem.ens_face(x).status();
        if s == crate::enemy::state::EnemyStatus::NoEnemy {
            target = Some(x);
            break;
        }
        if mem.ens_face(x).data05() & 0x02 == 0 {
            target = Some(x);
            break;
        }
    }
    let Some(x) = target else { return };
    let s = x / 16;
    // `@slotFound`：Resting + type=4(Rinka) + specialAttribs=0 + isHit=0
    mem.enemy(s).set_status(crate::enemy::state::EnemyStatus::Resting);
    mem.enemy(s).set_enemy_type(0x04);
    mem.enemy(s).set_special_attribs(0x00);
    mem.enemy(s).set_is_hit(0x00);
    common_jump_0e(mem, x);
    // `animFrame = _id_FrameNone`（逐渐淡入）
    mem.ens_face(x).set_anim_frame(0xF7);
    // `hi ← spawner.hi`、位置由 (posIndex*2 | status) 索引位表
    let hi = mem.rinka_spawner_hi(slot);
    mem.ens_face(x).set_hi(hi);
    let idx = mem.rinka_spawner_pos_index(slot);
    let byte = SPAWN_POS_TBL[((idx << 1) | (status & 0x01)) as usize];
    init_rinka_position(mem, x, byte);
    // posIndex 递增，到 6 回绕
    let next = idx.wrapping_add(1);
    mem.set_rinka_spawner_pos_index(slot, if next == 6 { 0 } else { next });
}

#[cfg(test)]
mod tests_rinka_spawners {
    use crate::enemy::state::EnemyStatus;
    use crate::frame::Manager;

    fn mgr() -> Manager {
        let mut m = Manager::default();
        m.write_ram(crate::ram_map::IN_AREA, 0x13); // Tourian
        m.write_ram(crate::ram_map::END_TIMER + 1, 0xFF); // 计时器停用
        m.write_ram(crate::ram_map::PPUCTRL_ZP, 0x00);
        m.write_ram(crate::ram_map::SCROLL_DIR, 0x00);
        m
    }

    fn arm(m: &mut Manager, slot: u8, status: u8, hi: u8) {
        m.set_rinka_spawner_status(slot, status);
        m.set_rinka_spawner_hi(slot, hi);
        m.set_rinka_spawner_pos_index(slot, 0);
    }

    /// 生成器创建：先占 1 号、再占 0 号；`status` 取特项字节高 nibble。
    #[test]
    fn spawn_creates_generator_one_first() {
        let mut m = mgr();
        m.set_rinka_spawner_status(0, 0xFF);
        m.set_rinka_spawner_status(1, 0xFF);
        assert_eq!(super::spawn_rinka_spawner(&mut m, 0x18), 0xFF);
        assert_eq!(m.rinka_spawner_status(1), 0x01, "先占 1 号，status=byte>>4");
        assert_eq!(m.rinka_spawner_status(0), 0xFF, "0 号仍未启用");
        assert_eq!(m.rinka_spawner_hi(1), 0x00, "hi=(PPUCTRL^ScrollDir)&1");
    }

    /// `hi ^ FrameCount` bit0 为 0 ⇒ 该帧不处理（两生成器交替）。
    #[test]
    fn generators_alternate_on_frame_count_bit0() {
        let mut m = mgr();
        arm(&mut m, 0, 0x00, 0x00); // hi=0
        m.write_ram(crate::ram_map::FRAME_COUNT, 0x00); // 0^0=0 ⇒ 跳过
        super::update_all_rinka_spawners(&mut m);
        assert_eq!(m.ens_face(0x20).status(), EnemyStatus::NoEnemy, "未生成");
        m.write_ram(crate::ram_map::FRAME_COUNT, 0x02); // 0^2 bit0=1 ⇒ 处理；且 &6==2 ⇒ 仍被门挡
        super::update_all_rinka_spawners(&mut m);
        assert_eq!(m.ens_face(0x20).status(), EnemyStatus::NoEnemy, "FrameCount&6 挡");
        m.write_ram(crate::ram_map::FRAME_COUNT, 0x01); // bit0=1 且 &6==0 ⇒ 生成
        super::update_all_rinka_spawners(&mut m);
        assert_eq!(m.ens_face(0x20).status(), EnemyStatus::Resting, "生成 Rinka");
        assert_eq!(m.enemy(2).enemy_type(), 0x04, "type=4");
    }

    /// `MotherBrainStatus >= 4` ⇒ 不再生成（母脑将死）。
    #[test]
    fn no_spawn_when_mother_brain_is_dying() {
        let mut m = mgr();
        arm(&mut m, 0, 0x00, 0x00);
        m.write_ram(crate::ram_map::FRAME_COUNT, 0x01);
        m.set_mb_status(0x04);
        super::update_all_rinka_spawners(&mut m);
        assert_eq!(m.ens_face(0x20).status(), EnemyStatus::NoEnemy);
    }

    /// 逃脱计时器活动中（`EndTimer+1 != $FF`）整件跳过。
    #[test]
    fn timer_active_blocks_all_spawning() {
        let mut m = mgr();
        arm(&mut m, 0, 0x00, 0x00);
        m.write_ram(crate::ram_map::FRAME_COUNT, 0x01);
        m.store_end_timer(0x99, 0x99);
        super::update_all_rinka_spawners(&mut m);
        assert_eq!(m.ens_face(0x20).status(), EnemyStatus::NoEnemy);
    }

    /// 位置来自位表：`posIndex=0,status=0 ⇒ $22` ⇒ y=`$27`、x=`$27`；status=1 ⇒ `$2A`。
    #[test]
    fn position_comes_from_spawn_table() {
        let mut m = mgr();
        arm(&mut m, 0, 0x00, 0x00);
        m.write_ram(crate::ram_map::FRAME_COUNT, 0x01);
        super::update_all_rinka_spawners(&mut m);
        assert_eq!(m.enemy(2).y(), 0x27, "(0x22 & $F0)|7");
        assert_eq!(m.enemy(2).pos_x(), 0x27, "(0x22<<4)|7");
        assert_eq!(m.rinka_spawner_pos_index(0), 1, "posIndex 递增");

        let mut m2 = mgr();
        arm(&mut m2, 0, 0x01, 0x00); // status=1 ⇒ 奇数列
        m2.write_ram(crate::ram_map::FRAME_COUNT, 0x01);
        super::update_all_rinka_spawners(&mut m2);
        assert_eq!(m2.enemy(2).y(), 0x27, "表项 $2A 的 Y");
        assert_eq!(m2.enemy(2).pos_x(), 0xA7, "表项 $2A 的 X=(0xA<<4)|7");
    }

    /// `posIndex` 到 6 回绕到 0。
    #[test]
    fn pos_index_wraps_at_six() {
        let mut m = mgr();
        arm(&mut m, 0, 0x00, 0x00);
        m.set_rinka_spawner_pos_index(0, 5);
        m.write_ram(crate::ram_map::FRAME_COUNT, 0x01);
        super::update_all_rinka_spawners(&mut m);
        assert_eq!(m.rinka_spawner_pos_index(0), 0);
    }

    /// 屏幕内已占用槽（data05 bit1 置）被跳过；三槽都占用则不生成。
    #[test]
    fn occupied_visible_slots_block_spawn() {
        let mut m = mgr();
        arm(&mut m, 0, 0x00, 0x00);
        m.write_ram(crate::ram_map::FRAME_COUNT, 0x01);
        for slot in 0..3u8 {
            m.enemy(slot).set_status(EnemyStatus::Active);
            m.enemy(slot).set_data05(0x02);
        }
        super::update_all_rinka_spawners(&mut m);
        assert_eq!(m.enemy(2).enemy_type(), 0x00, "槽 2 未被覆写");
    }
}
