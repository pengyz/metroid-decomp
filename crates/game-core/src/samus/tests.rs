//! 玩家受伤与击退链字段级用例（#123；规格 `docs/spec/player-damage-chain.md`）。
//!
//! 布景一律走 typed 句柄（`m.enemy(slot)` / `m.samus()` / `m.set_*`）：本文件是新增文件，
//! B0 架构耦合判据按文件记账，裸 `write_ram/write_wram` 布景会被判「新增直接依赖」。
//! 断言读回则用 typed getter 与内存位形成对——三条用例分别对应卡面 checklist 的三例：
//! HP 减 / 击退位移与帧数 / 无敌帧内不重复受伤。

use crate::state::Manager;

/// 把 Samus 与敌槽 4（字节 `$0440`）摆成重叠（照 `frame/tests.rs::frozen_enemy_deals_no_damage`）。
fn overlap_enemy(m: &mut Manager, enemy_status: crate::enemy::state::EnemyStatus) {
    m.set_ppuctrl_zp(1); // NT 位 0
    m.set_scroll_dir(0x02); // 横房
    {
        let mut e = m.enemy(4);
        e.set_status(enemy_status);
        e.set_y(49);
        e.set_pos_x(209);
        e.set_radius_y(8);
        e.set_radius_x(8);
        e.set_hi(0); // 与 Samus 同 hi
        e.set_enemy_type(0x05); // Zoomer（Brinstar）
    }
    let mut s = m.samus();
    s.set_y(49);
    s.set_x(209);
    s.set_radius_y(15);
    s.set_radius_x(4);
    s.set_hi(0);
}

/// ①受伤→HP 减：`SubtractHealth ; 07:CE92` 的 BCD 双字节减。
/// ROM 行为：`Health=0x0030`、`HealthChange=0x0005` ⇒ `0x0025`，尾件清 `HealthChange`。
#[test]
fn damage_reduces_health_bcd() {
    let mut m = Manager::default();
    m.samus().set_health(0x0030);
    m.set_health_change(0x0005);
    crate::weapon::projectile::subtract_health(&mut m);
    assert_eq!(m.samus().health(), 0x0025, "BCD 30-5=25（低字节借位链）");
    assert_eq!(m.health_change(), 0, "非死径尾件 ClearHealthChange");
}

/// ②击退位移与帧数：`CheckHealthStatus ; 07:CDFA`。
/// 受击帧写面：`$70←$32` 同调用递减 ⇒ 读回 `$31`；`$72←$FF` 后按 isHit bit3 定向；
/// `speedY←$FD`、`AccelY←$38`；X 推速 `A = i帧(dec 之前)>>4`。
/// 后续每帧 `dec $70`，50 帧依次 pre=50..1 ⇒ 档 `3:3 / 2:16 / 1:16 / 0:15`。
#[test]
fn knockback_frames_decay_x_speed() {
    let mut m = Manager::default();
    m.samus().set_is_hit(0x28); // bit5 命中 + bit3 向右
    m.samus().set_knockback_is_bomb(1); // $73=1 ⇒ 斜向击退（$77≠0）
    m.samus().set_accel_x(0); // AccelX=0 ⇒ 允许推速写入
    m.set_frame_count(1); // 奇帧 ⇒ 不触发闪烁隐帧
    crate::samus::physics::check_health_status(&mut m);
    assert_eq!(m.samus_invincible_delay(), 0x31, "$70←$32 同调用递减");
    assert_eq!(m.samus().speed_y(), -3, "击退上抛 speedY=$FD(-3)");
    assert_eq!(m.samus().accel_y(), 0x38, "击退重力 $38");
    assert_eq!(m.samus().speed_x(), 3, "首帧 A=0x32>>4=3（推右）");
    assert_eq!(m.samus().is_hit(), 0, "CheckHealthBeep 尾件清 isHit");

    // 余下 49 帧：记录每帧 speedX，统计各档帧数。
    let mut counts = [0usize; 4];
    counts[m.samus().speed_x() as usize] += 1; // 首帧
    for _ in 0..49 {
        m.set_frame_count(1);
        crate::samus::physics::check_health_status(&mut m);
        let s = m.samus().speed_x();
        assert!(s >= 0, "推右档位应为非负");
        counts[s as usize] += 1;
    }
    assert_eq!(counts[3], 3, "pre 0x32/0x31/0x30 >>4 = 3");
    assert_eq!(counts[2], 16, "pre 0x2F..0x20 >>4 = 2");
    assert_eq!(counts[1], 16, "pre 0x1F..0x10 >>4 = 1");
    assert_eq!(counts[0], 15, "pre 0x0F..0x01 >>4 = 0（0 帧不再递减）");
    assert_eq!(m.samus_invincible_delay(), 0, "50 帧后 i 帧归零");

    // 方向位：isHit 只有 bit5（bit3=0）⇒ $72=0，同一档位取补=推左。
    let mut m = Manager::default();
    m.samus().set_is_hit(0x20);
    m.samus().set_knockback_is_bomb(1);
    m.samus().set_accel_x(0);
    m.set_frame_count(1);
    crate::samus::physics::check_health_status(&mut m);
    assert_eq!(m.samus().speed_x(), -3, "推左：0-3 取补");

    // AccelX≠0 且 A<3 ⇒ 跳过 X 推速写入（Lx009），让加速径管。
    // 先走一帧受击面拿到 dir=1，再清 isHit 并把 i 帧压到 2（pre=2 ⇒ A=0<3）。
    m.samus().set_accel_x(0x18);
    m.samus().set_is_hit(0);
    m.set_samus_invincible_delay(0x02);
    let before = m.samus().speed_x();
    m.set_frame_count(1);
    crate::samus::physics::check_health_status(&mut m);
    assert_eq!(m.samus().speed_x(), before, "A<3 且 AccelX≠0 ⇒ 不写推速");
}

/// ③无敌帧内不重复受伤：碰撞径 `uw_collision_detection` 的 i 帧门（asm `07:F0D4`）。
/// i 帧非零 ⇒ 敌×萨判定整段跳过（零写）；归零 ⇒ 同一位形立刻受伤。
#[test]
fn invincible_frames_block_repeat_damage() {
    use crate::enemy::state::EnemyStatus;
    use crate::samus::state::SamusStatus;

    // i 帧内：零写
    let mut m = Manager::default();
    overlap_enemy(&mut m, EnemyStatus::Resting); // F282 的免伤门是 ≥Frozen(4)
    m.set_samus_invincible_delay(0x20);
    m.samus().set_health(0x0030);
    crate::weapon::projectile::uw_collision_detection(&mut m);
    assert_eq!(m.samus().is_hit(), 0, "i 帧门 ⇒ 不置 Samus.isHit");
    assert_eq!(m.samus().health(), 0x0030, "i 帧门 ⇒ 不扣血");
    assert_eq!(m.health_change(), 0, "i 帧门 ⇒ 不置 HealthChange");

    // i 帧归零：同一位形受伤（isHit bit5 + HP 下降 → Brinstar $0080 扣穿 ⇒ 死亡）
    let mut m = Manager::default();
    overlap_enemy(&mut m, EnemyStatus::Resting);
    m.set_samus_invincible_delay(0);
    m.samus().set_health(0x0030);
    crate::weapon::projectile::uw_collision_detection(&mut m);
    assert_ne!(m.samus().is_hit() & 0x20, 0, "命中 ⇒ isHit bit5 置（CDFA 的触发位）");
    assert_ne!(m.samus().health(), 0x0030, "HealthChange 非零 ⇒ 扣血");
    assert_eq!(m.samus().status(), Some(SamusStatus::Dead), "Brinstar 敌撞 $0080 扣穿 ⇒ sa_Dead");
}
