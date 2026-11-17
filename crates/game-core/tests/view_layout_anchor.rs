//! 视图偏移锚：**每个句柄 setter 落地的绝对地址 == asm 结构体声明的地址**。
//!
//! 为什么需要它：`no-raw-addr` 门只防「绕过视图」，防不住「视图自己把偏移写错」。
//! 2026-11-17 的 `Object` y/x 就是把 `$030D/$030E` 写成了 8/9（= speedY/speedX），
//! 直到业务代码开始用 `set_x/set_y` 才被门装载测试抓到。本文件把这类错误提前到
//! 视图层：写一次、读一次绝对地址，地址对不上立刻红。
//!
//! 权威来源（A 级）：
//! - `refs/m1disasm/SRC/constants_ram.asm:217-239`（`Object` 16 字段）
//! - `refs/m1disasm/SRC/constants_ram.asm:218-238`（`Samus` = 0 号槽，`$0300-$0316`）
//! - `refs/m1disasm/SRC/constants_ram.asm:161-215`（`Enemy`/`EnemyExtra`）

use game_core::frame::Manager;

/// 读绝对地址（测试专用：`read_ram` 是 crate 内部面，这里用快照投影读）。
fn byte_at(m: &Manager, addr: u16) -> u8 {
    let img = m.snapshot_nes();
    if (0x6000..0x8000).contains(&addr) {
        img.wram[(addr - 0x6000) as usize]
    } else {
        img.ram[(addr & 0x07FF) as usize]
    }
}

#[test]
fn object_view_offsets_match_asm_struct() {
    // Object .struct（constants_ram.asm:217-239）：status/radiusY/radiusX/animFrame/
    // animDelay/animResetIndex/animIndex/data07/speedY/speedX/isHit/onScreen/hi/y/x
    let mut m = Manager::default();
    m.object(8).set_status(0x11);
    assert_eq!(byte_at(&m, 0x0380), 0x11, "status 应在 $0300+ 槽*16 + 0");
    m.object(8).set_door_type(0x22);
    assert_eq!(byte_at(&m, 0x0387), 0x22, "doorType/data07 应是 +7");
    m.object(8).set_speed_y(-2);
    assert_eq!(byte_at(&m, 0x0388), 0xFE, "speedY 应是 +8（不是 y！）");
    m.object(8).set_speed_x(1);
    assert_eq!(byte_at(&m, 0x0389), 0x01, "speedX 应是 +9");
    m.object(8).set_is_hit(0x40);
    assert_eq!(byte_at(&m, 0x038A), 0x40, "isHit 应是 +$0A");
    m.object(8).set_on_screen(1);
    assert_eq!(byte_at(&m, 0x038B), 1, "onScreen 应是 +$0B");
    m.object(8).set_hi(1);
    assert_eq!(byte_at(&m, 0x038C), 1, "hi 应是 +$0C");
    m.object(8).set_y(0x68);
    assert_eq!(byte_at(&m, 0x038D), 0x68, "y 应是 +$0D");
    m.object(8).set_x(0xF0);
    assert_eq!(byte_at(&m, 0x038E), 0xF0, "x 应是 +$0E");
}

#[test]
fn samus_view_offsets_match_asm_struct() {
    // Samus = Object 的 0 号槽（$0300-$0316）。逐字节 setter→绝对地址。
    let mut m = Manager::default();
    m.samus().set_radius_y(0x0F);
    assert_eq!(byte_at(&m, 0x0301), 0x0F);
    m.samus().set_radius_x(0x04);
    assert_eq!(byte_at(&m, 0x0302), 0x04);
    m.samus().set_anim_frame(0x12);
    assert_eq!(byte_at(&m, 0x0303), 0x12);
    m.samus().set_anim_delay(0x01);
    assert_eq!(byte_at(&m, 0x0304), 0x01);
    m.samus().set_anim_reset_index(0x20);
    assert_eq!(byte_at(&m, 0x0305), 0x20, "animResetIndex 应是 $0305");
    m.samus().set_anim_index(0x21);
    assert_eq!(byte_at(&m, 0x0306), 0x21);
    m.samus().set_on_elevator(true);
    assert_eq!(byte_at(&m, 0x0307), 1);
    m.samus().set_speed_y(-2);
    assert_eq!(byte_at(&m, 0x0308), 0xFE);
    m.samus().set_speed_x(1);
    assert_eq!(byte_at(&m, 0x0309), 0x01);
    m.samus().set_is_hit(0x40);
    assert_eq!(byte_at(&m, 0x030A), 0x40);
    m.samus().set_hi(1);
    assert_eq!(byte_at(&m, 0x030C), 1);
    m.samus().set_y(0xA2);
    assert_eq!(byte_at(&m, 0x030D), 0xA2);
    m.samus().set_x(0xC2);
    assert_eq!(byte_at(&m, 0x030E), 0xC2);
    m.samus().set_jump_displacement(0x10);
    assert_eq!(byte_at(&m, 0x030F), 0x10);
    m.samus().set_accel_y(0x18);
    assert_eq!(byte_at(&m, 0x0314), 0x18, "accelY 应是 $0314");
    m.samus().set_accel_x(0x01);
    assert_eq!(byte_at(&m, 0x0315), 0x01, "accelX 应是 $0315");
    m.samus().set_horz_speed_max(0x03);
    assert_eq!(byte_at(&m, 0x0316), 0x03, "horzSpeedMax 应是 $0316");
}

#[test]
fn enemy_view_offsets_match_asm_struct() {
    // Enemy（$0400+i*16）与 EnemyExtra（$6AF4+i*16）。
    let mut m = Manager::default();
    m.enemy(3).set_y(0x50);
    assert_eq!(byte_at(&m, 0x0430), 0x50, "Ens.y 应是 $0400+槽*16+0");
    m.enemy(3).set_pos_x(0x60);
    assert_eq!(byte_at(&m, 0x0431), 0x60);
    m.enemy(3).set_delay(0x02);
    assert_eq!(byte_at(&m, 0x0439), 0x02, "Ens.delay 应是 +9");
    m.enemy(3).set_health(0x04);
    assert_eq!(byte_at(&m, 0x043B), 0x04, "Ens.health 应是 +$0B");
    m.enemy(3).set_data0d(0x40);
    assert_eq!(byte_at(&m, 0x043D), 0x40, "Ens.data0D 应是 +$0D");
    m.enemy(3).set_data05(0x1A);
    assert_eq!(byte_at(&m, 0x0435), 0x1A, "Ens.data05 应是 +5");
    // Extra 面（wram）
    m.enemy(3).set_status(game_core::enemy::state::EnemyStatus::Active);
    assert_eq!(byte_at(&m, 0x6B24), 2, "EnsExtra.status 应是 $6AF4+槽*16+0");
    m.enemy(3).set_radius_y(0x0C);
    assert_eq!(byte_at(&m, 0x6B25), 0x0C);
    m.enemy(3).set_anim_frame(0x27);
    assert_eq!(byte_at(&m, 0x6B27), 0x27, "EnsExtra.animFrame 应是 +3");
    m.enemy(3).set_anim_index(0x36);
    assert_eq!(byte_at(&m, 0x6B2A), 0x36, "EnsExtra.animIndex 应是 +6");
    m.enemy(3).set_hi(1);
    assert_eq!(byte_at(&m, 0x6B2B), 1, "EnsExtra.hi 应是 +7");
    m.enemy(3).set_enemy_type(5);
    assert_eq!(byte_at(&m, 0x6B32), 5, "EnsExtra.type 应是 +$0E");
}
