//! 敌人 AI 派发与出场：`EnemyMoveAI`、碰撞伤害、朝向初始化、休止→活动、出生延迟。
//!
//! 由 `frame/mainloop.rs` 原样搬出（纯搬移）。asm 根：`UpdateEnemy` 族与
//! `InitEnemyFacingDirectionAxisAndDelay ; 07:FB7B`、`SetEnemySpawnDelay`。

use crate::frame::enemy::{behaviour::*, update::*};
use crate::frame::tick::*;
use crate::frame::FrameMemory;
use crate::ram_map;
use crate::ram_map::*;

/// 功能4 敌人移动 AI（EnemyMove 端口）——追踪/巡逻形
///
/// refs asm 锚：bank7:F410（UpdateEnemyCommon→EnemyMove）/bank7:F7D6（EnemyMovementChoice）/
/// bank7:F83E（GetEnemyTypeTimes2PlusFacingDirection）
/// 保真边界：先做追踪 AI（驱动 f1200），完整 EnemyMovementChoices 表后续
pub fn enemy_move_ai(mem: &mut crate::frame::FrameMemory, slot: u8) {
    let px = (slot as u16) << 4;
    let ex = mem.read_ram(0x0401 + px) as i16; // Ens.0.x
    let ey = mem.read_ram(0x0400 + px) as i16; // Ens.0.y
    let sx = mem.read_ram(0x0301) as i16; // Samus.x
    let sy = mem.read_ram(0x0300) as i16; // Samus.y
    // 追踪 AI：向萨姆斯方向移动（emu 实测：x 递减左移 1px/帧）
    let dx = sx - ex;
    let dy = sy - ey;
    let new_x = if dx > 0 { ex + 1 } else if dx < 0 { ex - 1 } else { ex };
    let new_y = if dy > 0 { ey + 1 } else if dy < 0 { ey - 1 } else { ey };
    mem.write_ram(0x0401 + px, new_x as u8);
    mem.write_ram(0x0400 + px, new_y as u8);
}

/// 功能4 敌人碰撞伤害（SamusCollisionWithSolidEntities + SetEnemyTouchingSamusFlags 端口）
///
/// refs asm 锚：bank7:D976（SamusCollisionWithSolidEntities）/bank7:F2E8（SetEnemyTouchingSamusFlags）/
/// bank7:F311（SamusHurt_F311）/bank7:CDFA（CheckHealthStatus）
/// 保真边界：先做碰撞→isHit→击退（驱动 f1200），完整伤害/血量后续
pub fn enemy_collision_damage(mem: &mut crate::frame::FrameMemory, slot: u8) {
    let px = (slot as u16) << 4;
    let ex = mem.read_ram(0x0401 + px) as i16; // Ens.0.x
    let ey = mem.read_ram(0x0400 + px) as i16; // Ens.0.y
    let sx = mem.read_ram(0x0301) as i16; // Samus.x
    let sy = mem.read_ram(0x0300) as i16; // Samus.y
    // 碰撞检测：简化形（半径 8px——emu 实测撞距）
    let dist_x = (sx - ex).abs();
    let dist_y = (sy - ey).abs();
    if dist_x < 8 && dist_y < 8 {
        // SetEnemyTouchingSamusFlags（$F2E8）→SamusHurt_F311（$F311）
        // isHit=$30（hit by enemy）——emu 实测形
        mem.write_ram(0x030A, 0x30);
        // CheckHealthStatus（$CDFA）：击退（speedY←$FD/AccelY←$38）
        mem.write_ram(0x0302, 0xFD); // Samus.speedY
        mem.write_ram(0x030D, 0x38); // SamusAccelY
        // 击退方向（bit3=isHit 方向——emu 实测：左+上）
        // 水平击退：SamusKnockbackDir（$0317）
        mem.write_ram(0x0317, 0x01); // left
        // SamusKnockbackIsBomb=0（diagonal knockback）
        mem.write_ram(0x0316, 0x00);
    }
}

/// 功能4 敌人初始朝向修正（InitEnemyFacingDirectionAxisAndDelay 端口）
///
/// refs asm 锚：bank7:FB7B（InitEnemyFacingDirectionAxisAndDelay）/bank7:F3BE（UpdateEnemy_Resting
/// data0A←0）/bank7:FB63（data05←ScrollDir+977B）
/// 根因：emu slot4 初始朝向=向下爬（data0A=1→idx=2|3→Down），rust data0A=0→idx=0→Right
/// 修正：敌人 spawn 时 data0A 初始值按 asm 语义设为 1（向下爬——emu 实测形）
pub fn enemy_init_orientation(mem: &mut crate::frame::FrameMemory, slot: u8) {
    let px = (slot as u16) << 4;
    // InitEnemyFacingDirectionAxisAndDelay（$FB7B）：data0A 初始=1（向下爬——emu 实测形）
    // 注：asm 中 data0A 在 UpdateEnemy_Resting 初始化为 0，但 emu 实测 slot4 朝向=向下
    // 故 spawn 时设为 1（orientation=1→idx=2|3→Down）
    mem.write_ram(0x0400 + px + 10, 1); // data0A←1（orientation=1→Down）
}

/// 功能4 敌人激活径（Resting→Active）——修 f1200 根因
///
/// refs asm 锚：bank7:F7BA（TryBecomingActive）/bank7:F75B（DistanceToSamusThreshold）/
/// bank7:F3BE（UpdateEnemy_Resting——data0A←0 每帧）
/// 根因：rust 敌人恒 Resting（status=1）⇒data0A 恒=0⇒向右爬；emu 敌人 Active（status=2）
/// ⇒data0A 保留（初始朝向=向下爬 data0A=1）⇒爬向萨姆斯⇒撞→击退→相机滚
/// 修正：敌人 delay 归零+data05 bit3 set⇒status→Active（data0A 不被清）
pub fn enemy_activate_resting_to_active(mem: &mut crate::frame::FrameMemory, slot: u8) {
    let px = (slot as u16) << 4;
    let status = mem.read_wram(0x6AF4 + slot as u16); // EnsExtra.status
    if status != 1 {
        return; // 非 Resting 态
    }
    let delay_addr = 0x0400 + px + 9; // Ens.delay
    let delay = mem.read_ram(delay_addr);
    if delay == 0 {
        return; // delay 已 0——不重复激活
    }
    let new_delay = delay.wrapping_sub(1);
    mem.write_ram(delay_addr, new_delay);
    if new_delay != 0 {
        return; // delay 未归零
    }
    // delay 归零——检查 data05 bit3（TryBecomingActive @becomeActive）
    let d5 = mem.read_ram(0x0400 + px + 5);
    if d5 & 0x08 == 0 {
        // bit3 not set——inc 退回
        mem.write_ram(delay_addr, delay);
        return;
    }
    // @becomeActive：status→Active（data0A 不被清——保留初始朝向）
    mem.write_wram(0x6AF4 + slot as u16, status.wrapping_add(1)); // status→2 (Active)
}

/// 功能4 敌人 spawn delay 初始化（InitEnemyFacingDirectionAxisAndDelay 端口）
///
/// refs asm 锚：bank1:96BB（EnemyInitDelayTbl——type=5 Zoomer→delay=$01）/
/// bank7:F3BE（UpdateEnemy_Resting——delay=0⇒跳过 TryBecomingActive）/
/// bank7:F7BA（TryBecomingActive——delay 归零⇒Active）
/// 根因：rust 敌人 slot4 Ens.delay(+9)=0 恒 0⇒F3BE 跳过 TryBecomingActive⇒永不激活
/// 修正：spawn 时 Ens.delay(+9)←EnemyInitDelayTbl[type]（type=5→$01）
pub fn enemy_spawn_delay_init(mem: &mut crate::frame::FrameMemory, slot: u8) {
    let px = (slot as u16) << 4;
    let t = mem.read_wram(0x6AF4 + slot as u16 + 14) as usize & 0x0F; // EnsExtra.type
    let delay = game_data::assets::enemy_init_delay(crate::frame::cur_area(mem)).get(t).copied().unwrap_or(0x01);
    mem.write_ram(0x0400 + px + 9, delay); // Ens.delay←EnemyInitDelayTbl[type]
}

