//! `PipeBugAIRoutine`（`refs/m1disasm/SRC/enemies/pipe_bug.asm`，25 行逐行端口）。
//!
//! **四区各 include 一份**（`prg1_brinstar.asm:909`、`prg2_norfair.asm:610`、
//! `prg4_kraid.asm:754`、`prg5_ridley.asm:618`）——逻辑同构，唯一分区差异是
//! `@speedXTable`：Brinstar `$04/-$04`，其余三区 `$08/-$08`。
//!
//! 行为概要：管道虫先在管道里**向上飞**（`accelY` 由 $00 起，靠 `$7F` 重力使
//! `speedY` 转正），当 `Samus.y - y < $40` 或竖直速度转正后**转为水平前进**
//! （`speedX = ±table[data05&1]`），此后只按加速度积分移动；越界（屏幕回绕）
//! 即**自删**（`status ← 0`）。

use crate::enemy::framework;
use crate::enemy::motion;

/// `enemyStatus_Active`（`constants.asm:135-142`）。
const STATUS_ACTIVE: u8 = 0x02;

/// `@speedXTable`——按 bank 分区（`.if BANK == 1`）。
fn speed_x_table(area: u8) -> [i8; 2] {
    if area & 0x0F == 0 {
        [0x04, -0x04] // Brinstar
    } else {
        [0x08, -0x08] // Norfair / Kraid / Ridley
    }
}

/// `PipeBugAIRoutine_{AREA}`。
pub(crate) fn pipe_bug_ai(mem: &mut crate::frame::Manager, x: u8) {
    let slot = x / 16;
    let area = crate::frame::cur_area(mem);
    let active = mem.enemy(slot).status_raw() == STATUS_ACTIVE;
    // `lda status / cmp #Active / bne @applySpeed`——非激活跳过速度逻辑。
    // 之后 `lda speedX / bne @applySpeed`——已在水平前进则不再改速度。
    if active && mem.enemy(slot).speed_x() == 0 {
        // `lda accelY / bne @checkIfGoForwards`
        if mem.enemy(slot).accel_y() == 0 {
            // `Samus.y - Ens.0.y / cmp #$40 / bcs @applySpeed`——还差 ≥4 格就继续升。
            let d = mem.samus().y().wrapping_sub(mem.enemy(slot).y());
            if d < 0x40 {
                // `lda #$7F / sta EnsExtra.0.accelY,x`——重力终将使 speedY 转正。
                mem.enemy(slot).set_accel_y(0x7F);
            }
        } else if mem.enemy(slot).speed_y() >= 0 {
            // @checkIfGoForwards：`bmi @applySpeed` 未取 ⇒ 竖直速度已不为负。
            // 停竖直：`speedY←0`、`speedSubPixelY←0`、`accelY←0`。
            mem.enemy(slot).set_speed_y(0);
            mem.enemy(slot).set_data06(0);
            mem.enemy(slot).set_accel_y(0);
            // 转水平：`data05 & $01` 选向 → `@speedXTable,y`。
            let facing = (mem.enemy(slot).data05() & 0x01) as usize;
            let vx = speed_x_table(area)[facing];
            mem.enemy(slot).set_speed_x(vx);
        }
    }
    // @applySpeed
    // `lda data05 / asl / bmi @exit`（ROM 01:9B74 `0A` / 01:9B75 `30`）——ASL 先把 bit7
    // 移进 C 丢弃，BMI 读的 N 来自**原 bit6** ⇒ 判的是 bit6（`UpdateEnemy_EnData05Bit6 ;
    // 07:F3AA` 每帧翻转 ⇒ 该敌位移按 30FPS 半速）。
    // 【2026-11-22 更正】旧形判 bit7（并照抄 pipe_bug.asm:48 的第三方误注"bit 7"）——
    // bit7 对本敌不可达 ⇒ 门永不命中 ⇒ 每帧都落 4px（房扫 room21 f75 起 204px 的根因）。
    if mem.enemy(slot).data05() & 0x40 != 0 {
        framework::update_enemy_common_no_move_anim(mem, x, 0x03);
        return;
    }
    // `lda status / cmp #Active / bne @exit`
    if mem.enemy(slot).status_raw() != STATUS_ACTIVE {
        framework::update_enemy_common_no_move_anim(mem, x, 0x03);
        return;
    }
    // `jsr EnemyGetDeltaY_UsingAcceleration` / `jsr EnemyGetDeltaX_UsingAcceleration`
    let dy = motion::enemy_get_delta_y_using_acceleration(mem, x);
    let dx = motion::enemy_get_delta_x_using_acceleration(mem, x);
    // `jsr StoreEnemyPositionToTemp / jsr ApplySpeedToPosition`：失败（越界回绕）⇒ @remove。
    if !motion::apply_speed_to_position(mem, x, dy, dx) {
        mem.enemy(slot)
            .set_status(crate::enemy::state::EnemyStatus::NoEnemy);
        return;
    }
    // @exit：`lda #$03 / jmp CommonJump_UpdateEnemyCommon_noMove`
    framework::update_enemy_common_no_move_anim(mem, x, 0x03);
}

#[cfg(test)]
mod tests_pipe_bug {
    use super::*;
    use crate::enemy::state::{ens, ens_extra, EnemyStatus};
    use crate::frame::Manager;

    const X: u8 = 0x20;
    const SLOT: u8 = X / 16;

    fn stage(m: &mut Manager, status: u8, speed_x: i8, accel_y: i8, speed_y: i8) {
        game_data::pack::init_from_env().ok();
        m.write_ram(crate::ram_map::IN_AREA, 0x10); // Brinstar（测试夹具面，生产径走 cur_area）
        m.enemy(SLOT).set_status(EnemyStatus::from_byte(status));
        m.enemy(SLOT).set_speed_x(speed_x);
        m.enemy(SLOT).set_accel_y(accel_y);
        m.enemy(SLOT).set_speed_y(speed_y);
        m.enemy(SLOT).set_y(0x40);
        m.enemy(SLOT).set_pos_x(0x40);
        m.enemy(SLOT).set_hi(0);
    }

    /// `Samus.y - y >= $40` ⇒ 不改 `accelY`（继续以 0 重力上升）。
    #[test]
    fn far_below_samus_keeps_accel_zero() {
        let mut m = Manager::default();
        stage(&mut m, 2, 0, 0, -4);
        m.samus().set_y(0x40 + 0x40);
        pipe_bug_ai(&mut m, X);
        assert_eq!(m.enemy(SLOT).accel_y(), 0, "差 ≥4 格时不该启动重力");
    }

    /// `Samus.y - y < $40` ⇒ `accelY ← $7F`。
    #[test]
    fn near_samus_starts_gravity() {
        let mut m = Manager::default();
        stage(&mut m, 2, 0, 0, -4);
        m.samus().set_y(0x40 + 0x3F);
        pipe_bug_ai(&mut m, X);
        assert_eq!(m.enemy(SLOT).accel_y(), 0x7F);
    }

    /// 竖直速度已转正 + `accelY != 0` ⇒ 停竖直并转水平（朝向表）。
    #[test]
    fn positive_speed_y_turns_into_horizontal_run() {
        let mut m = Manager::default();
        stage(&mut m, 2, 0, 0x7F, 2);
        m.enemy(SLOT).set_data05(0x00);
        pipe_bug_ai(&mut m, X);
        assert_eq!(m.enemy(SLOT).speed_y(), 0);
        assert_eq!(m.enemy(SLOT).accel_y(), 0);
        assert_eq!(m.enemy(SLOT).speed_x(), 0x04, "Brinstar 朝右 = +4");

        let mut m2 = Manager::default();
        stage(&mut m2, 2, 0, 0x7F, 2);
        m2.enemy(SLOT).set_data05(0x01);
        pipe_bug_ai(&mut m2, X);
        assert_eq!(m2.enemy(SLOT).speed_x(), -0x04, "Brinstar 朝左 = −4");

        let mut m3 = Manager::default();
        stage(&mut m3, 2, 0, 0x7F, 2);
        m3.write_ram(crate::ram_map::IN_AREA, 0x01); // Norfair
        m3.enemy(SLOT).set_data05(0x00);
        pipe_bug_ai(&mut m3, X);
        assert_eq!(m3.enemy(SLOT).speed_x(), 0x08, "非 Brinstar 区 = ±8");
    }

    /// 已水平前进（`speedX != 0`）⇒ 不再进入速度选择分支。
    #[test]
    fn already_running_skips_speed_selection() {
        let mut m = Manager::default();
        stage(&mut m, 2, 0x04, 0x7F, 2);
        pipe_bug_ai(&mut m, X);
        assert_eq!(m.enemy(SLOT).speed_x(), 0x04, "不得重写 speedX");
        assert_eq!(m.enemy(SLOT).speed_y(), 2, "不得清 speedY");
    }

    /// `0A/30`（ROM 01:9B74/9B75）判的是 **bit6**：ASL 把 bit7 移进 C 丢弃，BMI 读原 bit6。
    /// bit6 由 `UpdateEnemy_EnData05Bit6 ; 07:F3AA` 每帧翻转 ⇒ 该敌位移 30FPS 半速。
    /// 旧测用 `data05=0x80`（对 PipeBug 不可达）把错误极性固化成了规格。
    #[test]
    fn data05_bit6_is_the_move_gate_not_bit7() {
        let mut m = Manager::default();
        stage(&mut m, 2, 0, 0, -4); // Active、speedX=0、accelY=0、speedY=-4
        m.samus().set_y(0x40 + 0x40); // 差 ≥ $40 ⇒ accelY 保持 0

        m.enemy(SLOT).set_data05(0x00); // bit6=0 ⇒ 本帧执行位移（dy=-4）
        pipe_bug_ai(&mut m, X);
        assert_eq!(m.enemy(SLOT).y(), 0x3C, "bit6=0 帧必须 -4");

        m.enemy(SLOT).set_data05(0x40); // bit6=1 ⇒ 跳到 @exit，整帧不移动
        pipe_bug_ai(&mut m, X);
        assert_eq!(m.enemy(SLOT).y(), 0x3C, "bit6=1 帧必须整帧不移动（每 2 帧 4px）");
        assert_eq!(m.enemy(SLOT).data06(), 0, "跳过帧不得推进 speedSubPixelY");

        m.enemy(SLOT).set_data05(0x80); // bit7 与本门无关（ASL 已丢弃）
        pipe_bug_ai(&mut m, X);
        assert_eq!(m.enemy(SLOT).y(), 0x38, "bit7 置不得冻结移动（旧测极性是错的）");
    }

    /// 非 Active ⇒ 不移动。
    #[test]
    fn inactive_does_not_move() {
        let mut m = Manager::default();
        stage(&mut m, 1, 0x04, 0x7F, 2);
        let x0 = m.enemy(SLOT).pos_x();
        pipe_bug_ai(&mut m, X);
        assert_eq!(m.enemy(SLOT).pos_x(), x0);
        assert_eq!(m.enemy(SLOT).accel_y(), 0x7F, "Resting 不进速度分支");
    }

    /// 横滚房 + y 越界回绕 ⇒ `@remove`（status ← 0）。
    #[test]
    fn out_of_bounds_removes_bug() {
        let mut m = Manager::default();
        stage(&mut m, 2, 0x04, 0x00, 0x0E);
        m.enemy(SLOT).set_y(0xE8);
        m.write_ram(crate::ram_map::SCROLL_DIR, 0x03); // 横滚
        pipe_bug_ai(&mut m, X);
        assert_eq!(m.enemy(SLOT).status_raw(), 0, "越界应自删");
    }

    /// 加速径：`accelY=$7F` 每帧把小数累进 `speedSubPixelY`，进位推 `speedY`；
    /// 6 帧后整数速度应 >0，且 Δy == speedY（无小数余量时）。
    #[test]
    fn acceleration_integrates_into_speed() {
        let mut m = Manager::default();
        stage(&mut m, 2, 0x04, 0x7F, 0x00);
        let mut last = 0i8;
        for _ in 0..6 {
            last = motion::enemy_get_delta_y_using_acceleration(&mut m, X);
        }
        let sp = m.enemy(SLOT).speed_y();
        assert!(sp > 0, "6 帧后整数速度应 >0，实得 {sp}");
        // asm：`Delta = speedY + carry`（小数累加的进位也算进本帧位移）⇒ 差 0 或 1。
        let d = last as i16 - sp as i16;
        assert!(d == 0 || d == 1, "Δy 应等于 speedY 或 speedY+1（进位），实得 {last} vs {sp}");
        // 极速钳：连跑 40 帧不得越过 0x0E。
        for _ in 0..40 {
            motion::enemy_get_delta_y_using_acceleration(&mut m, X);
        }
        assert_eq!(m.enemy(SLOT).speed_y(), 0x0E, "应钳在 ±14");
        let _ = ens::SPEED_Y;
        let _ = ens_extra::SUB_PIXEL_Y;
    }
}
