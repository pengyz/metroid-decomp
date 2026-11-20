//! 世界推进分派：`UpdateWorld` 的逐件调用序 + 各对象更新薄壳（Uw*）+ 门过渡滚屏。
//!
//! 【2026-11-17 改名】原 `world/scroll.rs`。文件名曾承诺"滚动"，但它装的是
//! **世界推进的分派表**；相机滚动状态与行为归 `world/camera/`。
//! 判据：谁能写 `SCROLL_*`/`MAP_POS_*`——只有相机。

//!
//! 由 `frame/mainloop.rs` 原样搬出（纯搬移）。asm 根：`ScrollDoor ; 07:E1F1`、
//! `UpdateWorld` 分派、`Uw*` 各对象更新件。

use crate::frame::tick::*;
use crate::frame::Manager;
use crate::ram_map;

/// ScrollDoor（**asm 内容锚** `ScrollDoor: ; 07:E1F1`，`refs/m1disasm/SRC/prg7_engine.asm:7006`）：
///
/// DoorEntryStatus($56)：0=非门出 / 1=右滚 / 2=左滚（横） / 3=下滚 / 4=上滚（竖）。
/// 逐像素滚由 `world::camera::scroll` 执行；滚到底是什么状态由 **ScrollX/ScrollY 归 0** 判定
/// （asm `LE204: ldx ScrollX / bne Exit15`、`LE21B: ldx ScrollY / bne Exit15`）。
///
/// 【2026-11-17 修】旧形把"进度"记在 `$0026`/`$0025` 上并自增——而 `$0025` 是
/// **MMC1CTRL_ZP（MMC1 移位寄存器口）**，`$0026` 亦非滚屏计数器。那段代码既没滚屏、
/// 又往 mapper 控制口写值，是"看起来在滚门、实际在改 bank 寄存器"的典型。
pub fn scroll_door(mem: &mut Manager) {
    use crate::world::camera::scroll::{scroll_down, scroll_left, scroll_right, scroll_up};
    let d = mem.read_ram(ram_map::DOOR_ENTRY_STATUS);
    if d == 0 {
        return;
    }
    match d {
        // DoorEntryStatus=1：`jsr ScrollRight` → `jmp LE204`
        1 => {
            scroll_right(mem);
            if mem.camera().scroll_x() == 0 {
                do_one_door_scroll(mem, 5); // `ldx #$05`（出门）
            }
        }
        // DoorEntryStatus=2：`jsr ScrollLeft` → fallthrough 到 LE204 同一判据
        2 => {
            scroll_left(mem);
            if mem.camera().scroll_x() == 0 {
                do_one_door_scroll(mem, 5);
            }
        }
        3 => {
            scroll_down(mem);
            vertical_room_centered(mem);
        }
        4 => {
            scroll_up(mem);
            vertical_room_centered(mem);
        }
        _ => {} // 5+：出门序（SamusEnterDoor 面）
    }
}

/// `VerticalRoomCentered ; 07:E21B`：竖滚居中后清门名表数据，X=1（右手门）/2（左手门）。
fn vertical_room_centered(mem: &mut Manager) {
    if mem.camera().scroll_y() != 0 {
        return; // `bne Exit15`
    }
    mem.write_ram(ram_map::SCROLL_BLOCK_NT3, 0); // `stx ScrollBlockOnNameTable3`
    mem.write_ram(ram_map::SCROLL_BLOCK_NT0, 0); // `stx ScrollBlockOnNameTable0`
    let x = if mem.samus().x() & 0x80 != 0 { 1 } else { 2 }; // `lda Samus.x / bmi` 后 `inx`
    toggle_scroll_and_finish(mem, x);
}

/// `DoOneDoorScroll ; 07:E22C`：DoorDelay←$20，按 `SamusDoorData*8` 的进位与
/// `DoorScrollStatus<3` 决定是否 ToggleScroll，末了写 MirrorCntrl 与 DoorEntryStatus。
fn do_one_door_scroll(mem: &mut Manager, x: u8) {
    mem.write_ram(ram_map::DOOR_DELAY, 0x20); // `lda #$20 / sta DoorDelay`
    let data = mem.read_ram(ram_map::SAMUS_DOOR_DATA);
    let carry = data & 0xE0 != 0; // `jsr Amul8` 的进位 = A*8 溢出（A≥$20）
    if carry {
        mem.write_ram(ram_map::MIRROR_CNTRL, 0x47); // `lda #$47`（横滚镜）
        mem.write_ram(ram_map::DOOR_ENTRY_STATUS, x); // `stx DoorEntryStatus`
        return;
    }
    if mem.read_ram(ram_map::DOOR_SCROLL_STATUS) < 0x03 {
        toggle_scroll_and_finish(mem, x); // `cpy #$03 / bcc LE241`
        return;
    }
    mem.write_ram(ram_map::MIRROR_CNTRL, 0x47);
    mem.write_ram(ram_map::DOOR_ENTRY_STATUS, x);
}

/// `ToggleScroll ; 07:E252`：ScrollDir ^= $03；返回 MirrorCntrl ^ $08，随后
/// `sta MirrorCntrl / stx DoorEntryStatus`。
fn toggle_scroll_and_finish(mem: &mut Manager, x: u8) {
    let dir = mem.read_ram(ram_map::SCROLL_DIR) ^ 0x03;
    mem.write_ram(ram_map::SCROLL_DIR, dir);
    let mirror = mem.read_ram(ram_map::MIRROR_CNTRL) ^ 0x08;
    mem.write_ram(ram_map::MIRROR_CNTRL, mirror);
    mem.write_ram(ram_map::DOOR_ENTRY_STATUS, x);
}


/// UpdateWorld（**asm 内容锚** `UpdateWorld: ; 07:CB29`——r26 环壳：19 件渐进端口）。
/// 件序：SpritePagePos←0/UpdateAllEnemies($F345)/UpdateSamusProjectiles($D4BF)/
/// UpdateSamus($CC0D)/AreaRoutine($95C3)/UpdateElevator($D7B3)/UpdateAllStatues($D9D4)/
/// 爆/虫/弹/瓦碰/SamusEnterDoor($8B13)/UpdateAllDoors/TileBlasts/CollisionDetection/
/// DisplayBar($E0C1)/PipeBugHoles/CheckMissileToggle/UpdateAllPowerUps($DB37)/
/// UpdateTourianItems($FDE3)/尾清 sprite RAM（$F4 填）。
pub fn update_world(mem: &mut crate::frame::Manager) {
    mem.set_sprite_page_pos(0); // SpritePagePos←0
    update_all_enemies(mem); // F345（敌环——槽×6+状态 dispatch）
    // UpdateSamusProjectiles（D4BF——随件）
    update_samus_projectiles(mem); // D4BF（弹件环——3 槽状态机）
    #[allow(unreachable_code)]
    {
        crate::samus::update_samus(mem); // UpdateSamus（CC0D——samus 域件会师）
        crate::world::statue::update_all_statues(mem); // UpdateAllStatues（D9D4 会师）
        // ---- r26 dispatch 壳补齐（复核 C blocker——asm 件序 5-20 逐件！）----
        uw_area_routine(mem); // $95C3（区专径）
        uw_elevator(mem); // $D7B3
        uw_enemy_explosions(mem); // 爆族
        uw_mellows(mem); // 虫族
        uw_en_projectiles(mem); // 敌弹族
        uw_skree_projectiles(mem); // Skree 弹族（dieDelay 生命周期——goal-轮7 件）
        uw_samus_enter_door(mem); // $8B13（过门检！）
        uw_all_doors(mem); // 门链
        uw_tile_blasts(mem); // 瓦片爆/复原
        uw_collision_detection(mem); // 实体碰撞
        uw_display_bar(mem); // $E0C1（HUD 动态面！）
        uw_pipe_bug_holes(mem); // 虫洞族
        uw_check_missile_toggle(mem); // 弹切检
        uw_all_power_ups(mem); // $DB37（吃能力！）
        uw_tourian_items(mem); // $FDE3
        // 尾清 sprite RAM（$F4 填——asm CB6A 环）
        let mut x = mem.sprite_page_pos();
        while x != 0 {
            mem.write_ram(0x0200 + x as u16, 0xF4);
            x = x.wrapping_add(4);
        }
    }
}

/// AreaRoutine（区 bank $95C3——区专径）。
///
/// `AreaRoutine_Tourian ; 03:9B25`（`prg3_tourian.asm:566-575`）件序：
/// `jsr UpdateAllCannons` → `jsr MotherBrainStatusHandler` → `jsr UpdateEndTimer` →
/// `jsr DrawEndTimerEnemy` → `jsr UpdateAllZebetites` → `jmp UpdateAllRinkaSpawners`。
/// 本卡只落 **UpdateAllZebetites**；另外四件仍是缺口（炮台/MB 态/逃逸计时/生成器），
/// 用 `DIVERGENCES` 记账（fail-loud 但不阻断主循环），待 G-3/G-5/G-6 卡接入。
pub(crate) fn uw_area_routine(mem: &mut crate::frame::Manager) {
    // 【区号口径】`InArea & $0F`：0=Brinstar、1=Norfair、**2=Kraid、3=Tourian**、4=Ridley
    // （`BankSwitchPending` 表 `07:CA30` = `02 03 05 04 06` ⇒ idx3→bank3=Tourian）。
    if crate::frame::in_area_raw(mem) & 0x0F != 3 {
        return; // 非 Tourian：其余四区 AreaRoutine 尚无专径（Brinstar 空态已实证）
    }
    // UpdateAllCannons / MotherBrainStatusHandler / UpdateEndTimer / DrawEndTimerEnemy
    // / UpdateAllRinkaSpawners —— 未端口（缺口卡 G-3/G-5/G-6）
    crate::frame::DIVERGENCES.with(|d| d.borrow_mut().push(0x9B26));
    crate::enemy::zebetite::update_all_zebetites(mem);
}

/// UpdateElevator（`$D7B3`）——电梯十态（`frame::elevator`；槽空 status=0 ⇒ ExitSub 恒空径）。
pub(crate) fn uw_elevator(mem: &mut crate::frame::Manager) {
    crate::world::elevator::update_elevator(mem);
}

/// UpdateAllEnemyExplosions（爆族——槽空⇒跳过）。
pub(crate) fn uw_enemy_explosions(mem: &mut crate::frame::Manager) {
    crate::enemy::framework::update_all_enemy_explosions(mem);
}

/// UpdateAllMellows（虫族）。
pub(crate) fn uw_mellows(_mem: &mut crate::frame::Manager) {}

/// UpdateAllTileBlasts（瓦片爆/复原——空态跳过）。
pub(crate) fn uw_tile_blasts(mem: &mut crate::frame::Manager) {
    crate::world::tile_blast::update_all_tile_blasts(mem);
}

/// UpdateAllEnProjectiles（敌弹族——asm UpdateWorld 07:CB29 环位 :2507；槽空 status=0 ⇒
/// UpdateEnProjectile@RTS 恒空径）。
pub(crate) fn uw_en_projectiles(mem: &mut crate::frame::Manager) {
    crate::enemy::en_projectile::update_all_en_projectiles(mem);
}

/// SamusEnterDoor（$8B13——过门检）：全径实现=physics::samus_enter_door（goal-r29）。
pub(crate) fn uw_samus_enter_door(mem: &mut crate::frame::Manager) {
    crate::samus::physics::samus_enter_door(mem);
}

/// UpdateAllDoors（$8B79——`jsr` 点 prg7_engine.asm:2510，件序在 SamusEnterDoor 与
/// UpdateAllTileBlasts 之间；实件在 `crate::world::room_draw::update_all_doors`）。
/// 2026-11-14 更正：本函数曾是空壳（`{}`），导致门链实现齐备却从未接线——
/// A 方案把输入窗移出开场演出后，`case-002` 立即暴露（emu 门 status 2/anim $31
/// vs rust 永停 status 1 全零）。空壳期所有门单测仍绿，因为没有一条链路走到 `jsr` 点。
pub(crate) fn uw_all_doors(mem: &mut crate::frame::Manager) {
    crate::world::room_draw::update_all_doors(mem);
}

/// UpdateAllPipeBugHoles（虫洞族——空态跳过）。
pub(crate) fn uw_pipe_bug_holes(_mem: &mut crate::frame::Manager) {}

/// CheckMissileToggle（弹切检——A/SELECT 切换面）。
pub(crate) fn uw_check_missile_toggle(_mem: &mut crate::frame::Manager) {}

/// UpdateAllPowerUps（$DB37——吃能力！槽环+埋实心块判+拾取碰撞+授予）。
/// 【能力移植（2026-10-31）】全链端口（pickup::spawn——ScanForItems 上图/DB37 显示/
/// DC7F 触碰/DB2F 授予/EE4A 唯一物史）——sc-item-pickup 流程件。
pub(crate) fn uw_all_power_ups(mem: &mut crate::frame::Manager) {
    if let Some(item) = crate::pickup::spawn::update_all_power_ups(mem) {
        // 拾取音乐/延迟（CBF9 PowerUpMusic——声包随件记账）
        crate::frame::DIVERGENCES.with(|d| d.borrow_mut().push(0x0CBF9));
        let _ = item;
    }
}

/// UpdateTourianItems（$FDE3——Tourian 专件）。
pub(crate) fn uw_tourian_items(mem: &mut crate::frame::Manager) {
    crate::enemy::zebetite::update_tourian_items(mem);
}


#[cfg(test)]
mod tests_scroll_door {
    use super::*;

    /// `LE204: ldx ScrollX / bne Exit15` —— 归 0 才算滚完，且置 DoorDelay=$20、DoorEntryStatus=5。
    #[test]
    fn horizontal_door_scroll_finishes_at_scroll_x_zero() {
        let mut m = Manager::default();
        m.write_ram(ram_map::DOOR_ENTRY_STATUS, 1);
        m.camera().set_scroll_x(0);
        scroll_door(&mut m);
        assert_eq!(m.read_ram(ram_map::DOOR_ENTRY_STATUS), 5);
        assert_eq!(m.read_ram(ram_map::DOOR_DELAY), 0x20);
    }

    /// 未归 0 时不结束（继续逐像素滚）。
    #[test]
    fn horizontal_door_scroll_continues_when_not_centered() {
        let mut m = Manager::default();
        m.write_ram(ram_map::DOOR_ENTRY_STATUS, 1);
        m.camera().set_scroll_x(0x40);
        scroll_door(&mut m);
        assert_eq!(m.read_ram(ram_map::DOOR_ENTRY_STATUS), 1); // 未结束
    }

    /// `VerticalRoomCentered`：ScrollY 归 0 后清两个 name-table 区块标记。
    #[test]
    fn vertical_door_centering_clears_nt_block_flags() {
        let mut m = Manager::default();
        m.write_ram(ram_map::DOOR_ENTRY_STATUS, 3);
        m.camera().set_scroll_y(0);
        m.write_ram(ram_map::SCROLL_BLOCK_NT3, 0x33);
        m.write_ram(ram_map::SCROLL_BLOCK_NT0, 0x33);
        // 直接测 `VerticalRoomCentered`（`scroll_door` 会先滚一像素，ScrollY 已非 0）
        vertical_room_centered(&mut m);
        assert_eq!(m.read_ram(ram_map::SCROLL_BLOCK_NT3), 0);
        assert_eq!(m.read_ram(ram_map::SCROLL_BLOCK_NT0), 0);
    }

    /// 回归：旧实现往 `$0025`（MMC1CTRL_ZP，mapper 移位寄存器口）与 `$0026` 写"进度"。
    #[test]
    fn door_scroll_never_touches_mmc1_control_port() {
        let mut m = Manager::default();
        m.write_ram(ram_map::DOOR_ENTRY_STATUS, 1);
        m.camera().set_scroll_x(0);
        let before_25 = m.read_ram(0x0025);
        let before_26 = m.read_ram(0x0026);
        scroll_door(&mut m);
        assert_eq!(m.read_ram(0x0025), before_25);
        assert_eq!(m.read_ram(0x0026), before_26);
    }

    /// DoorEntryStatus=0（不在门里）⇒ 完全 no-op。
    #[test]
    fn no_door_entry_is_noop() {
        let mut m = Manager::default();
        scroll_door(&mut m);
        assert_eq!(m.read_ram(ram_map::DOOR_ENTRY_STATUS), 0);
        assert_eq!(m.read_ram(ram_map::DOOR_DELAY), 0);
    }
}
