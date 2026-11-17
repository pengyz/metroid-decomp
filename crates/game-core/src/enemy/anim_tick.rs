//! 敌人动画推进的**数据面适配**：按区取表 → 分派到实体方法。
//!
//! 【为什么与绘制分开】走表推进是「纯自域」逻辑（已下沉到值对象 `Anim::step` 与实体
//! `Enemy::advance_anim`/`Explosion::advance_anim`），本模块只剩唯一的跨域输入——**按区表**。
//! 绘制（写 OAM、读相机）在 `crate::render::enemy_sprite`：判据是「谁被改」。
//!
//! 【双池共享的 asm 事实】`UpdateEnemyAnim ; 07:E094` 在 ROM 里被敌池与爆炸池共用：
//! 敌槽 `x` 步长 16、爆炸槽步长 8，靠 `$6AF4 + $C0 == $6BB4` 恰好接上。两个池各有具名
//! 入口把关，巧合被关在各自的换算里并由 pin 测守着（见文末测试）。

use crate::ram_map;

/// `UpdateEnemyAnim ; 07:E094`（走表动画推进）。
/// UpdateEnemyAnim（**asm 内容锚** `UpdateEnemyAnim: ; 07:E094`——敌人动画走表全径）：
/// status==Pickup(5)⇒RTS；delay 门（≠0 递减未归零⇒RTS）→`sta animDelay←A`→走
/// EN_ANIM_TABLE[animIndex]（区表——Brinstar $9D6A）：FF⇒animIndex←reset 续走；
/// 否则 animFrame(+3)←表值、animIndex+1。调用形：CrawlerAI @exit `lda #3/jsr $800C`。
pub(crate) fn update_enemy_anim(mem: &mut crate::frame::Manager, x: u8, a: u8) {
    update_anim_at(mem, x, a);
}

/// 爆炸池的同一例程入口（`UpdateEnemyExplosion` 尾件 `jmp AnimDrawEnemy` 的
/// `jsr UpdateEnemyAnim` 一半）。**槽号入参**，基址换算收在 `explosion::ens_offset`。
///
/// 【为什么要有这两个入口】`UpdateEnemyAnim` 是 asm 里被敌池与爆炸池共用的例程：
/// 敌槽 `x` 16 步长、爆炸槽 `x` 8 步长，而 `$6AF4 + $C0` 恰等于爆炸池额外面基址
/// `$6BB4`。此前调用方直接把爆炸偏移喂给"敌函数"，靠朴素的加法巧合生效——
/// 一旦有人把内部改成 `slot_of(x)` 就整条静默错位（2026-11-17 试改即被池测抓出）。
/// 现在两个池各有具名入口，巧合被关在 `update_anim_at` 里且有 pin 测守着。
pub(crate) fn update_explosion_anim(mem: &mut crate::frame::Manager, slot: u8, a: u8) {
    // 与敌侧同源：表由调用方按区取，推进在实体（`Explosion::advance_anim`，无 Pickup 门）。
    let area = crate::world::room_load::area_idx(mem.read_ram(ram_map::IN_AREA)) as u8;
    let table = game_data::assets::enemy_anim_table(area);
    mem.explosion(slot).advance_anim(table, a);
}

fn update_anim_at(mem: &mut crate::frame::Manager, x: u8, a: u8) {
    // 【2026-11-17 三层归位】走表逻辑已下沉到 `Anim::step`（值对象，纯函数），
    // Pickup 门与写回在 `Enemy::advance_anim`（实体）。本函数只剩「按区取表 + 分派到句柄」
    // 这一件跨域适配——资产是外部依赖，实体不该自己去查。
    let area = crate::world::room_load::area_idx(mem.read_ram(ram_map::IN_AREA)) as u8;
    let table = game_data::assets::enemy_anim_table(area);
    mem.enemy(crate::enemy::state::slot_of(x)).advance_anim(table, a);
}


#[cfg(test)]
mod tests_dual_pool_base {
    /// **钉住双池共享的地址恒等式**：`update_enemy_anim`/`draw_enemy` 的 `x` 同时用于
    /// 敌槽（0..=0xAF，步长 16）与爆炸池（0xC0..=0xD8，步长 8）。这条恒等式一旦被
    /// 重构者"修正"成纯敌槽换算，爆炸池的动画会静默错位（见函数头注释）。
    #[test]
    fn explosion_pool_base_is_reachable_from_ens_offset() {
        for slot in 0..crate::enemy::explosion::SLOT_COUNT {
            let ex = crate::enemy::explosion::ens_offset(slot);
            assert!(ex >= 0xC0, "爆炸槽的 asm x 必须在 $C0 之上（got {ex:#04X}）");
            assert_eq!(
                crate::enemy::state::ENS_EXTRA_BASE + ex as u16,
                crate::enemy::explosion::EXTRA_BASE + slot as u16 * 8,
                "`$6AF4 + x` 必须落在爆炸池额外面（slot {slot}）"
            );
            // 换算成敌槽号会落到 12 及以上（`$C0/16=12`）——即**超出敌槽 0..=11 的合法范围**，
            // 这正是不能把共享例程的 `x` 当成敌槽偏移套 `slot_of()` 的原因。
            assert!(
                ex as usize / 16 >= crate::enemy::state::SLOT_COUNT as usize,
                "爆炸槽 {slot} 换算成敌槽号会落进合法敌槽区间（x={ex:#04X}）"
            );
        }
    }
}
