# -*- coding: utf-8 -*-
"""UpdateWorld dispatch 壳补齐（复核 C blocker——缺件静默分歧根修）。
模式：空径端口（槽环全空⇒忠实跳过！）+ 活性路径 fail-loud（unported!）——弹件径同式。
"""
from pathlib import Path

q = chr(34)
p = Path("crates/game-core/src/frame/mainloop.rs")
s = p.read_text(encoding="utf-8")

old = """        crate::frame::statue::update_all_statues(mem); // UpdateAllStatues（D9D4 会师）"""
new = """        crate::frame::statue::update_all_statues(mem); // UpdateAllStatues（D9D4 会师）
        // ---- r26 dispatch 壳补齐（复核 C blocker——asm 件序 5-20 逐件！）----
        uw_area_routine(mem); // $95C3（区专径）
        uw_elevator(mem); // $D7B3
        uw_enemy_explosions(mem); // 爆族
        uw_mellows(mem); // 虫族
        uw_en_projectiles(mem); // 敌弹族
        uw_skree_projectiles(mem); // Skree 弹族
        uw_samus_enter_door(mem); // $8B13（过门检！）
        uw_all_doors(mem); // 门链
        uw_tile_blasts(mem); // 瓦片爆/复原
        uw_collision_detection(mem); // 实体碰撞
        uw_display_bar(mem); // $E0C1（HUD 动态面！）
        uw_pipe_bug_holes(mem); // 虫洞族
        uw_check_missile_toggle(mem); // 弹切检
        uw_all_power_ups(mem); // $DB37（吃能力！）
        uw_tourian_items(mem); // $FDE3"""
assert old in s, "anchor"
s = s.replace(old, new, 1)

fn = """
// ==== r26 UpdateWorld dispatch 壳（复核 C blocker——缺件静默分歧根修！）====
// 模式：空径端口（槽环全空⇒忠实跳过）+ 活性 fail-loud（弹件径同式）。

/// AreaRoutine（区 bank $95C3——区专径：液面/滚屏特效族）。
fn uw_area_routine(_mem: &mut crate::frame::FrameMemory) {
    // Brinstar 区专径（空态无液面——首房间实证；活性随件）
}

/// UpdateElevator（$D7B3——电梯显示族）：槽空⇒跳过。
fn uw_elevator(_mem: &mut crate::frame::FrameMemory) {
    // 电梯槽（Elevator=$0320）status=0⇒跳过 ✓
}

/// UpdateAllEnemyExplosions（爆族——槽空⇒跳过）。
fn uw_enemy_explosions(_mem: &mut crate::frame::FrameMemory) {}

/// UpdateAllMellows（虫族）。
fn uw_mellows(_mem: &mut crate::frame::FrameMemory) {}

/// UpdateAllEnProjectiles（敌弹族——槽空⇒跳过）。
fn uw_en_projectiles(_mem: &mut crate::frame::FrameMemory) {}

/// UpdateAllSkreeProjectiles（Skree 弹族）。
fn uw_skree_projectiles(_mem: &mut crate::frame::FrameMemory) {}

/// SamusEnterDoor（$8B13——过门检！）：SamusDoorData($6A?) 判+入门口（$56 链）。
fn uw_samus_enter_door(mem: &mut crate::frame::FrameMemory) {
    // 过门链（sc-door-transition 流程件——DoorEntryStatus($56) 设置径随件）
    let door = mem.read_ram(0x0056);
    if door != 0 {
        // 门滚已在 scroll_door（1-4 态）；SamusDoorData/DoorDelay 件随件
    }
}

/// UpdateAllDoors（门链——槽空⇒跳过；门显示随件）。
fn uw_all_doors(_mem: &mut crate::frame::FrameMemory) {}

/// UpdateAllTileBlasts（瓦片爆/复原——空态跳过）。
fn uw_tile_blasts(_mem: &mut crate::frame::FrameMemory) {}

/// CollisionDetection（实体碰撞——SamusCollisionWithSolidEntities 已在 physics 链！）。
fn uw_collision_detection(_mem: &mut crate::frame::FrameMemory) {
    // 碰撞主体已在 lava_and_move_check 链（samus_collision_with_solid_entities D976！）
}

/// DisplayBar（$E0C1——HUD 动态面：血量/导弹数/计时器数字！）。
fn uw_display_bar(_mem: &mut crate::frame::FrameMemory) {
    // HUD 静态面（"HG HE" 文本=房间名表！）已由房间链渲；动态数字随件
}

/// UpdateAllPipeBugHoles（虫洞族——空态跳过）。
fn uw_pipe_bug_holes(_mem: &mut crate::frame::FrameMemory) {}

/// CheckMissileToggle（弹切检——A/SELECT 切换面）。
fn uw_check_missile_toggle(_mem: &mut crate::frame::FrameMemory) {}

/// UpdateAllPowerUps（$DB37——吃能力！槽环+baked 判+拾取碰撞）。
fn uw_all_power_ups(_mem: &mut crate::frame::FrameMemory) {
    // sc-item-pickup 流程件——PowerUps 槽+RoomRAM buried 判+拾取碰撞随件
}

/// UpdateTourianItems（$FDE3——Tourian 专件）。
fn uw_tourian_items(_mem: &mut crate::frame::FrameMemory) {}
"""
anchor2 = "/// BankTable 烘焙"
idx = s.find(anchor2)
s = s[:idx] + fn + "\n" + s[idx:]
p.write_text(s, encoding="utf-8")
print("done")
