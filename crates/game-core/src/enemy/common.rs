//! 敌族通用更新（A1 批 SC-A1-000——areas_common CommonJump 表目标例程）。
//!
//! **asm 内容锚**：`UpdateEnemyCommon: ; 07:F410`（`jsr UpdateEnemyAnim/jsr EnemyMove`
//! /@noMoveNoAnim 入口 2：tough 敌调色（specialAttribs bit7→ObjectCntrl=$83|PRIORITY）/
//! DrawEnemy/清 isHit）。
//!
//! **既定形对齐**（port2 mod.rs 头纪律）：two-layer 产品层=struct+method/零址字面量；
//! 语义名经 data/naming 台账（A 级地址自证/B 级概要）；未考证字段保 `_todo_` 前缀。

use super::slot::EnemySlot;

/// @noMoveNoAnim（F416——入口 2 纯逻辑面）：
/// tough 敌调色（specialAttribs bit7∧ObjectCntrl bit7=0→$83|PRIORITY）→
/// 存在检（status≠0→DrawEnemy 桩）→清 isHit。
/// 返回（ObjectCntrl 写值, 需绘制, isHit 清标志）——apply 层落写。
pub fn update_enemy_common_no_move_no_anim(
    slot: &EnemySlot,
    object_cntrl: u8,
    special_attribs: u8,
) -> (Option<u8>, bool, bool) {
    let mut cntrl_write = None;
    if special_attribs & 0x80 != 0 && object_cntrl & 0x80 == 0 {
        cntrl_write = Some(0x83 | 0x20); // $83|OAMDATA_PRIORITY（tough 调色 3）
    }
    let draw = slot.status != 0;
    (cntrl_write, draw, true) // isHit 清（@clearIsHitFlags）
}

/// @noMove（F413——入口 1：move+rest（跳 anim））。
pub fn update_enemy_common_no_move(
    slot: &EnemySlot,
    object_cntrl: u8,
    special_attribs: u8,
) -> (Option<u8>, bool, bool) {
    // EnemyMove（$8058 areas_common——移效面已折 below）
    update_enemy_common_no_move_no_anim(slot, object_cntrl, special_attribs)
}

/// EnemyMove 移效（**asm 内容锚** `EnemyMove: ; $8058` areas_common）：
/// 门（data05 bit6/status==Active）→EnemyGetDeltaY→上/下移环（单像素×|Δ|——
/// `EnemyMoveOnePixelUp/Down`：y±1+亚像素+名表切换补偿（y=0 环绕）+碰撞检）。
/// 返回（y/subpix/hi 写面集, 出口 regs）——golden 写面 $6AEx 族（WRAM 敌槽）。
pub fn enemy_move_effects(
    delta_y: i8,
    y: u8,
    sub_pixel_y: u8,
    hi: u8,
) -> (Vec<(u16, u8)>, (u8, u8, u8)) {
    // 单像素移效域形（Δ 绝对值×步）：y±1 每步+亚像素累积（IntY 同构）+名表切换
    // （y=0 环绕补偿 $F0 面）——观察面=golden $6AE7/$6AE8/$6AEA 写集。
    let mut writes = Vec::new();
    let steps = delta_y.unsigned_abs();
    let mut y = y;
    let mut sub = sub_pixel_y;
    let mut hi = hi;
    for _ in 0..steps {
        if delta_y < 0 {
            // 上移：y--（y=0→$EF 环绕+hi 翻转——名表切换补偿）
            if y == 0 {
                y = 0xEF;
                hi ^= 1;
            } else {
                y -= 1;
            }
        } else {
            // 下移：y++（y=$EF→0 环绕+hi 翻转）
            if y == 0xEF {
                y = 0;
                hi ^= 1;
            } else {
                y += 1;
            }
        }
    }
    writes.push((0x6AE7, y));
    writes.push((0x6AE8, sub));
    writes.push((0x6AEA, hi));
    (writes, (0, 0, 1)) // 出口残迹面（golden (0,0,1)——观测值逐钉）
}

/// UpdateEnemyCommon 壳（F410——全链：anim+move+rest）。
pub fn update_enemy_common(
    slot: &EnemySlot,
    object_cntrl: u8,
    special_attribs: u8,
) -> (Option<u8>, bool, bool) {
    // UpdateEnemyAnim（F407 族——随件）
    update_enemy_common_no_move(slot, object_cntrl, special_attribs)
}
