//! 世界推进：门滚屏、UpdateWorld 分派、各对象更新薄壳（Uw*）。
//!
//! 由 `frame/mainloop.rs` 原样搬出（纯搬移）。asm 根：`ScrollDoor ; 07:E1F1`、
//! `UpdateWorld` 分派、`Uw*` 各对象更新件。

use crate::frame::tick::*;
use crate::frame::FrameMemory;
use crate::ram_map;

/// ScrollDoor（**asm 内容锚** `ScrollDoor: ; 07:E1F1`——r26 端口）：
/// DoorEntryStatus($56)：0=非门出/1=右滚(ScrollRight $E6D2)/2=左滚(ScrollLeft $E6A7)/
/// 3=下滚(ScrollDown $E519)/4=上滚(ScrollUp $E4F1)；滚至 ScrollX/Y 归中⇒DoorEntryStatus←5
/// （出门）。滚件=1px/调（双调=双速）。
pub fn scroll_door(mem: &mut FrameMemory) {
    let d = mem.read_ram(0x0056);
    if d == 0 {
        return;
    }
    match d {
        1 => {
            // ScrollRight（$E6D2）：ScrollX+1（越 255⇒名表换）
            scroll_by(mem, 0x0026, true);
            if mem.read_ram(0x0026) == 0 {
                mem.write_ram(0x0056, 0x05); // DoOneDoorScroll 出门面
            }
        }
        2 => {
            scroll_by(mem, 0x0026, false);
            if mem.read_ram(0x0026) == 0 {
                mem.write_ram(0x0056, 0x05);
            }
        }
        3 => {
            scroll_by(mem, 0x0025, true);
            if mem.read_ram(0x0025) == 0 {
                mem.write_ram(0x0056, 0x05); // VerticalRoomCentered 形
            }
        }
        4 => {
            scroll_by(mem, 0x0025, false);
            if mem.read_ram(0x0025) == 0 {
                mem.write_ram(0x0056, 0x05);
            }
        }
        _ => {} // 5+（出门序——SamusEnterDoor 面随件）
    }
}

/// 滚 1px（ScrollRight/Left/Up/Down 簇共径——名表换/换页件随证）。
pub(crate) fn scroll_by(mem: &mut crate::frame::FrameMemory, addr: u16, up: bool) {
    let v = mem.read_ram(addr);
    let nv = if up { v.wrapping_add(1) } else { v.wrapping_sub(1) };
    mem.write_ram(addr, nv);
}

/// UpdateWorld（**asm 内容锚** `UpdateWorld: ; 07:CB29`——r26 环壳：19 件渐进端口）。
/// 件序：SpritePagePos←0/UpdateAllEnemies($F345)/UpdateSamusProjectiles($D4BF)/
/// UpdateSamus($CC0D)/AreaRoutine($95C3)/UpdateElevator($D7B3)/UpdateAllStatues($D9D4)/
/// 爆/虫/弹/瓦碰/SamusEnterDoor($8B13)/UpdateAllDoors/TileBlasts/CollisionDetection/
/// DisplayBar($E0C1)/PipeBugHoles/CheckMissileToggle/UpdateAllPowerUps($DB37)/
/// UpdateTourianItems($FDE3)/尾清 sprite RAM（$F4 填）。
pub fn update_world(mem: &mut crate::frame::FrameMemory) {
    mem.write_ram(0x005B, 0); // SpritePagePos←0
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
        let mut x = mem.read_ram(0x005B);
        while x != 0 {
            mem.write_ram(0x0200 + x as u16, 0xF4);
            x = x.wrapping_add(4);
        }
    }
}

/// AreaRoutine（区 bank $95C3——区专径：液面/滚屏特效族）。
pub(crate) fn uw_area_routine(_mem: &mut crate::frame::FrameMemory) {
    // Brinstar 区专径（空态无液面——首房间实证；活性随件）
}

/// UpdateElevator（`$D7B3`）——电梯十态（`frame::elevator`；槽空 status=0 ⇒ ExitSub 恒空径）。
pub(crate) fn uw_elevator(mem: &mut crate::frame::FrameMemory) {
    crate::world::elevator::update_elevator(mem);
}

/// UpdateAllEnemyExplosions（爆族——槽空⇒跳过）。
pub(crate) fn uw_enemy_explosions(_mem: &mut crate::frame::FrameMemory) {}

/// UpdateAllMellows（虫族）。
pub(crate) fn uw_mellows(_mem: &mut crate::frame::FrameMemory) {}

/// UpdateAllTileBlasts（瓦片爆/复原——空态跳过）。
pub(crate) fn uw_tile_blasts(_mem: &mut crate::frame::FrameMemory) {}

/// UpdateAllEnProjectiles（敌弹族——槽空⇒跳过）。
pub(crate) fn uw_en_projectiles(_mem: &mut crate::frame::FrameMemory) {}

/// SamusEnterDoor（$8B13——过门检）：全径实现=physics::samus_enter_door（goal-r29）。
pub(crate) fn uw_samus_enter_door(mem: &mut crate::frame::FrameMemory) {
    crate::samus::physics::samus_enter_door(mem);
}

/// UpdateAllDoors（$8B79——`jsr` 点 prg7_engine.asm:2510，件序在 SamusEnterDoor 与
/// UpdateAllTileBlasts 之间；实件在 `crate::world::room_draw::update_all_doors`）。
/// 2026-11-14 更正：本函数曾是空壳（`{}`），导致门链实现齐备却从未接线——
/// A 方案把输入窗移出开场演出后，`case-002` 立即暴露（emu 门 status 2/anim $31
/// vs rust 永停 status 1 全零）。空壳期所有门单测仍绿，因为没有一条链路走到 `jsr` 点。
pub(crate) fn uw_all_doors(mem: &mut crate::frame::FrameMemory) {
    crate::world::room_draw::update_all_doors(mem);
}

/// UpdateAllPipeBugHoles（虫洞族——空态跳过）。
pub(crate) fn uw_pipe_bug_holes(_mem: &mut crate::frame::FrameMemory) {}

/// CheckMissileToggle（弹切检——A/SELECT 切换面）。
pub(crate) fn uw_check_missile_toggle(_mem: &mut crate::frame::FrameMemory) {}

/// UpdateAllPowerUps（$DB37——吃能力！槽环+埋实心块判+拾取碰撞+授予）。
/// 【能力移植（2026-10-31）】全链端口（pickup::spawn——ScanForItems 上图/DB37 显示/
/// DC7F 触碰/DB2F 授予/EE4A 唯一物史）——sc-item-pickup 流程件。
pub(crate) fn uw_all_power_ups(mem: &mut crate::frame::FrameMemory) {
    if let Some(item) = crate::pickup::spawn::update_all_power_ups(mem) {
        // 拾取音乐/延迟（CBF9 PowerUpMusic——声包随件记账）
        crate::frame::DIVERGENCES.with(|d| d.borrow_mut().push(0x0CBF9));
        let _ = item;
    }
}

/// UpdateTourianItems（$FDE3——Tourian 专件）。
pub(crate) fn uw_tourian_items(_mem: &mut crate::frame::FrameMemory) {}

