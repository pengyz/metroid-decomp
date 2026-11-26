//! 房 5 门格到达性验收（#104）——**测试面文件**（`no-raw-addr` 棘轮对本文件不计入生产面）。
//!
//! A 级 ROM 依据（`refs/m1disasm/SRC/`）：
//! - `brinstar/rooms.asm:113` `Room05_{AREA}` 左右门列都引 `_id_Structure03`；
//! - `brinstar/structures.asm:52-56` `Structure03_{AREA}` = `01 06 / 01 06 / 01 06 / FF`；
//! - `brinstar/metatiles.asm:34-37` metatile 6 = `.byte $A0,$A0,$A0,$A0`、metatile 7 = `$A1`；
//! - `prg7_engine.asm` 07:E7E6-07:E813：逐点读房间瓦，`cmp #$A0` ⇒ `SamusDoorData` 加 1，
//!   `cmp #$A1` ⇒ 加 2；`room_ram::tile_blocks_non_projectile` 的判据是 `tile < $A0`。
//!
//! ⇒ 房 5 的门格瓦是 **`$A0`**（不是 `$A1`）；`$A1` 只出现在 metatile 7（别的房间的门结构）
//! 与门对象信息字节 `$A0 | doorType_Blue(=1)` 里。
//!
//! 本文件所有读值都走 typed 访问器（`mem.samus()` / `set_in_area` / `door_entry_status` /
//! `read_joy_pads`），不裸写 NES 地址。

use crate::frame::Manager;
use crate::samus::state::SamusStatus;

/// 房 5 + Samus 站位（站立半径 `radiusY=16`；球态 8，`SamusRoll` 起立 `radiusY += 8`）。
fn room5_with_samus_at(x: u8, y: u8) -> Manager {
    let mut m = Manager::default();
    m.set_in_area(0); // Brinstar
    m.set_room_number(0);
    crate::world::room_draw::select_room_ram(&mut m);
    crate::world::room_draw::init_tables(&mut m);
    assert!(crate::world::room_draw::draw_room(&mut m, 5), "房 5 应能画");
    m.samus().set_y(y);
    m.samus().set_x(x);
    m.samus().set_radius_y(16);
    m.samus().set_radius_x(8);
    m.samus().set_accel_y(0);
    m.samus().set_hi(0);
    m.samus().set_status(SamusStatus::Stand);
    m.set_scroll_dir(0);
    m.set_scroll_x(0);
    m.camera().set_scroll_y(0);
    m
}

/// 按住 LEFT 一帧（走 NMI 的真采样路径：`read_joy_pads` 把 `mem.joy1` 落到 `Joy1Status`）。
fn press_left_frame(m: &mut Manager) {
    m.joy1 = 0x02; // BUTTON_LEFT
    crate::frame::nmi::read_joy_pads(m);
}

/// 房 5 画出的门瓦就是 `$A0`：左门列 col 0-1、右门列 col 30-31 的 row 10-15 共 24 瓦；
/// 全房 `$A1` 为 0。
#[test]
fn room5_door_tiles_are_a0_not_a1() {
    let mut m = room5_with_samus_at(40, 111);
    let mut a0 = 0usize;
    let mut a1 = 0usize;
    for row in 0..30usize {
        for col in 0..32usize {
            let tile = m.room().tile((row * 8) as u8, (col * 8) as u8, 0);
            if tile == 0xA0 {
                a0 += 1;
                assert!(col <= 1 || col >= 30, "`$A0` 只应出现在左右门列，实际 col={col} row={row}");
                assert!((10..=15).contains(&row), "门列只应占 row 10-15，实际 row={row}");
            }
            if tile == 0xA1 {
                a1 += 1;
            }
        }
    }
    assert_eq!(a0, 24, "房 5 左门 12 瓦 + 右门 12 瓦 = 24 个 `$A0`");
    assert_eq!(a1, 0, "房 5 没有 `$A1`（metatile 7 属别的房间的门结构）");
}

/// 纯逻辑走到门格：从 x=40 按住 LEFT，≤30 帧后 `$58`(SamusDoorData) 置 1（进入左门列），
/// 此时 x 已进入门列范围（x ≤ 24，门列 col 0-1 = x 0..15，碰撞按脚部/身体边缘判）。
/// 全程无 poke：只写 `mem.joy1` 并走 NMI 采样 + `update_samus`。
#[test]
fn walking_left_reaches_door_column_without_poke() {
    let mut m = room5_with_samus_at(40, 111);
    let mut hit = None;
    let mut last_x = m.samus().x();
    for f in 0..30 {
        press_left_frame(&mut m);
        crate::samus::handler::update_samus(&mut m);
        last_x = m.samus().x();
        let dd = m.samus_door_data();
        if dd != 0 {
            hit = Some((f, last_x, dd));
            break;
        }
    }
    let (f, x, dd) = hit.expect("30 帧内应走进左门列并置 `$58`");
    assert!(f <= 25, "应在 25 帧内到达，实际 f={f}（x={x}）");
    assert_eq!(dd, 1, "左门 `cmp #$A0` 只加 1（右门才加 2），实际 {dd}");
    assert!(x <= 24, "到门格时 x 应进入左门列范围，实际 x={x}");
}

/// 门格 → 进门：引擎每帧在 Samus 之后调 `SamusEnterDoor($8B13)`；走到门列后
/// `DoorEntryStatus` 应置 bit7（`$80`）、`Samus.status` 切到 `sa_Door`(5)。
///
/// 这是本卡能达成的**收窄口径**（门格可达 + 触发）。房间号/MapPos 切换需要门对象链
/// （`uw_all_doors` → `scroll_door` 的 `$56` 1..5 滚动状态机）与真实载房，见测试末尾断言。
#[test]
fn door_tile_triggers_enter_door_state() {
    let mut m = room5_with_samus_at(40, 111);
    let room_before = m.room_number();
    let mut entered = None;
    let mut door_data_seen = 0u8;
    for f in 0..30 {
        press_left_frame(&mut m);
        crate::samus::handler::update_samus(&mut m);
        // 引擎顺序：Samus 之后跑对象/门链
        crate::world::update::uw_samus_enter_door(&mut m);
        let dd = m.samus_door_data();
        if dd != 0 {
            door_data_seen = dd;
        }
        if m.door_entry_status() != 0 {
            entered = Some((f, m.door_entry_status(), m.samus().status_raw()));
            break;
        }
    }
    let (f, d56, st) = entered.expect("走到门格后 SamusEnterDoor 应置 `$56`");
    assert_ne!(door_data_seen, 0, "进门前 `$58` 必须先非 0（触发源）");
    assert_eq!(d56, 0x80, "`$56` bit7 = SamusInDoor");
    assert_eq!(st, 5, "status 应切到 sa_Door");
    assert!(f >= 1);
    // 收窄口径的边界：本测试不推进滚动状态机，房间号不应变化。
    assert_eq!(
        m.room_number(),
        room_before,
        "合成 setup 无门对象槽，房间号应保持不变（完整换房需 uw_all_doors + 载房）"
    );
}

/// **正对照（诊断口径，不构成可玩证据）**：房 5 左门，从 `x=40` 按住 LEFT 走到门格（无 poke），
/// 触发进门链并跑到「请求载房」为止。记录：
/// `$58`=1 → `$56`=0x80（+`$80` 位）→ `$57`=2（水平左滚）→ `status`=5(sa_Door) → `$58`=(1<<4)|1=0x11
/// → DoorDelay 0x12 每帧 −2、Samus 被移出门（x 19→1）→ `$56`=0 → `RoomNumber`=$F0（载房请求哨兵）。
///
/// 为什么停在 $F0：本测试的站位/地图格是合成的（`set_room_number(0)`、MapPos 0/0），
/// 载房窗没有可解析的邻房；真实换房要真实地图位置 + 载房窗（见 findings §5.2/§5.3）。
#[test]
fn door_chain_runs_from_tile_to_load_request() {
    use crate::ram_map::{
        DOOR_DELAY, DOOR_ENTRY_STATUS, DOOR_SCROLL_STATUS, MAP_POS_X, ROOM_NUMBER, SAMUS_DOOR_DATA,
    };
    let mut m = room5_with_samus_at(40, 111);
    m.set_scroll_dir(0x02); // 水平滚（左门）
    let mut hit = None;
    for f in 0..30 {
        press_left_frame(&mut m);
        crate::samus::handler::update_samus(&mut m);
        if m.read_ram(SAMUS_DOOR_DATA) != 0 {
            hit = Some(f);
            break;
        }
    }
    let hit = hit.expect("按住 LEFT 30 帧内应碰到左门格（纯逻辑，无 poke）");
    assert_eq!(m.read_ram(SAMUS_DOOR_DATA), 1, "左门 `$A0` 命中 ⇒ SamusDoorData=1");
    assert!(m.samus().x() <= 24, "命门时 x 已进门列范围，实际 {}", m.samus().x());

    crate::world::update::uw_samus_enter_door(&mut m);
    assert_eq!(m.read_ram(DOOR_ENTRY_STATUS), 0x80, "SamusInDoor：$56 |= $80");
    assert_eq!(m.read_ram(DOOR_SCROLL_STATUS), 0x02, "左门水平滚 ⇒ DoorScrollStatus=2");
    assert_eq!(m.samus().status_raw(), 5, "status ← sa_Door(5)");
    assert_eq!(m.read_ram(SAMUS_DOOR_DATA), 0x11, "$58 ← ($58<<4)|status = 0x11");
    assert_eq!(m.read_ram(DOOR_DELAY), 0x12, "DoorDelay ← 18 帧");

    let x_before = m.samus().x();
    for _ in 0..20 {
        m.joy1 = 0;
        crate::frame::nmi::read_joy_pads(&mut m);
        crate::samus::handler::update_samus(&mut m);
        crate::world::update::update_world(&mut m);
    }
    assert_eq!(m.read_ram(DOOR_ENTRY_STATUS), 0x00, "出门序收尾：$56 清 0");
    assert!(m.samus().x() < x_before, "出门序把 Samus 往门外移（{} → {}）", x_before, m.samus().x());
    assert_eq!(m.read_ram(ROOM_NUMBER), 0xF0, "收尾请求载房：RoomNumber ← $F0 哨兵");
    let _ = MAP_POS_X;
    let _ = hit;
}
