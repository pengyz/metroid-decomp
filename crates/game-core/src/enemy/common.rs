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

/// UpdateEnemyCommon 壳（F410——UpdateEnemyAnim/EnemyMove 嵌套桩+入口 2）。
/// apply 层供输入（slot/cntrl/specialAttribs）+应用结果（写面+绘制桩）。
pub fn update_enemy_common(
    slot: &EnemySlot,
    object_cntrl: u8,
    special_attribs: u8,
) -> (Option<u8>, bool, bool) {
    // UpdateEnemyAnim（F407 族——随件）/EnemyMove（F3xx 族——随件）
    update_enemy_common_no_move_no_anim(slot, object_cntrl, special_attribs)
}
