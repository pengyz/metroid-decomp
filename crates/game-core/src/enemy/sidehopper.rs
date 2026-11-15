//! Sidehopper / Dessgeega / Ripper —— `enemies/sidehopper.asm`（32 行）+ `enemies/ripper.asm`（18 行）。
//!
//! asm 事实（A 级，逐行对读）：
//! - `SidehopperFloorAIRoutine`：取 `EnAnim_SidehopperFloorHopping` 的表偏移存入
//!   `EnemyFlipAfterDisplacementAnimIndex`（`$85/$86` 两字节，右/左朝向同一值），
//!   status≠Explode 时 `jsr CommonJump_EnemyFlipAfterDisplacement`，然后
//!   `$00←$06`（活动动画索引）、`$01←$08`（休止动画索引）→ `UpdateEnemyCommon_Decide`。
//! - `SidehopperCeilingAIRoutine`：同上，只是动画换 Ceiling 变体。
//! - bank5（Ridley）同址是 Dessgeega：动画换 `EnAnim_Dessgeega{,Ceiling}Hopping`。
//! - `RipperAIRoutine`：status≠Explode 时 `InitEnActiveAnimIndex_NoInitOffset`；bank1/4
//!   直接 `jmp CommonEnemyStub2`（= Sidehopper 尾件）；bank2 是 Ripper II（喷焰），
//!   `$00←$03`、`$01←$03` 后走 Decide。
//!
//! 动画表偏移（由各 bank `enemy_sprite_data.asm` 的 label 地址减 `EnAnimTable` 基址算出）：
//! bank1(Brinstar) `$9D6A` 基 → floor `$9D73`−基=**$09**、ceiling `$9D79`−基=**$0F**；
//! bank5(Ridley) `$9B85` 基 → floor `$9BC7`−基=**$42**、ceiling `$9BCD`−基=**$48**。

use crate::enemy::anim_idx::AnimIdx;
use crate::enemy::{framework, prep, tables};

const ENS: u16 = 0x6AF4;
const F_STATUS: u16 = 0;
const F_ANIM_DELAY: u16 = 4;
const F_RESET_ANIM: u16 = 5;
const F_ANIM_INDEX: u16 = 6;
const F_JUMP_DSPL: u16 = 13;
/// `EnemyFlipAfterDisplacementAnimIndex dw ;$85`（`constants_ram.asm:639`）——
/// 右朝向在 `$85`、左朝向在 `$86`（sidehopper 两条写同值）。
pub(crate) const FLIP_ANIM_IDX: u16 = 0x0085;
/// `$00/$01`：`UpdateEnemyCommon_Decide` 用的活动/休止动画索引（asm 尾件写入）。
const ANIM_ACTIVE_SLOT: u16 = 0x0000;
const ANIM_RESTING_SLOT: u16 = 0x0001;
/// `enemyStatus_Explode`（`constants_ram.asm`）——爆炸态跳过动画切换。
const STATUS_EXPLODE: u8 = 3;

/// 跳跃动画索引（各 bank 的 `EnAnim_*Hopping` 相对 `EnAnimTable` 的字节偏移）。
fn hopping_anim(area: u8, ceiling: bool) -> u8 {
    match (area, ceiling) {
        (4, false) => 0x42, // Ridley(bank5) Dessgeega floor
        (4, true) => 0x48,  // Ridley(bank5) Dessgeega ceiling
        (_, false) => 0x09, // bank1/4 Sidehopper floor
        (_, true) => 0x0F,  // bank1/4 Sidehopper ceiling
    }
}

/// `EnemyFlipAfterDisplacement ; 07:FB88`（逐行）：
/// 取 `|jumpDsplcmnt|`（负则二补）——
/// `< $08` ⇒ 回到休止动画（`InitEnAnimIndex` 径）；
/// `$08..=$0F` ⇒ 切到 hopping 动画（`animIndex←idx-1`、`resetAnimIndex←idx`、清 delay）；
/// `≥ $10` ⇒ 什么都不做（保留当前动画）。
pub(crate) fn enemy_flip_after_displacement(mem: &mut crate::frame::FrameMemory, x: u8) {
    let sq = ENS + x as u16;
    let mut d = mem.read_wram(sq + F_JUMP_DSPL);
    if (d as i8) < 0 {
        d = (!d).wrapping_add(1); // TwosComplement（$C3D4）
    }
    if d < 0x08 {
        prep::init_en_resting_anim_index(mem, x);
        return;
    }
    if d >= 0x10 {
        return;
    }
    // tya / and #$01 / tay ⇒ 水平朝向位
    let y = tables::enemy_type_times2_plus_facing(mem, x);
    let idx = mem.read_ram(FLIP_ANIM_IDX + (y & 1));
    if idx == mem.read_wram(sq + F_RESET_ANIM) {
        return; // 已是该动画
    }
    mem.write_wram(sq + F_ANIM_INDEX, idx.wrapping_sub(1)); // sta animIndex / dec animIndex
    mem.write_wram(sq + F_RESET_ANIM, idx); // InitEnResetAnimIndex
    mem.write_wram(sq + F_ANIM_DELAY, 0); // ClearEnAnimDelay
}

/// `SidehopperFloorAIRoutine` / `SidehopperCeilingAIRoutine`（`enemies/sidehopper.asm:4-32`）。
pub(crate) fn sidehopper_ai(mem: &mut crate::frame::FrameMemory, x: u8, ceiling: bool) {
    let area = crate::frame::cur_area(mem);
    let anim = hopping_anim(area, ceiling);
    mem.write_ram(FLIP_ANIM_IDX, anim);
    mem.write_ram(FLIP_ANIM_IDX + 1, anim);
    let sq = ENS + x as u16;
    if mem.read_wram(sq + F_STATUS) != STATUS_EXPLODE {
        enemy_flip_after_displacement(mem, x); // CommonJump_EnemyFlipAfterDisplacement
    }
    // CommonEnemyStub2 / CommonEnemyStub：$00←$06、$01←$08
    mem.write_ram(ANIM_ACTIVE_SLOT, 0x06);
    mem.write_ram(ANIM_RESTING_SLOT, 0x08);
    framework::update_enemy_common_decide(mem, x);
}

/// `InitEnActiveAnimIndex_NoInitOffset ; 07:FBCA`——与 `InitEnActiveAnimIndex` 的差别是
/// **不做 InitOffset 递减**（`sta resetAnimIndex` 后直接 `SetEnAnimIndex`）。
fn init_en_active_anim_index_no_init_offset(mem: &mut crate::frame::FrameMemory, x: u8) {
    let area = crate::frame::cur_area(mem);
    let y = tables::enemy_type_times2_plus_facing(mem, x);
    let idx = AnimIdx::at(area, game_data::assets::enemy_active_anim(area)[y as usize & 0x1F]);
    let sq = ENS + x as u16;
    if idx.off() == mem.read_wram(sq + F_RESET_ANIM) {
        return;
    }
    mem.write_wram(sq + F_RESET_ANIM, idx.off());
    mem.write_wram(sq + F_ANIM_INDEX, idx.off());
    mem.write_wram(sq + F_ANIM_DELAY, 0);
}

/// `RipperAIRoutine`（**bank1/4 版**，`enemies/ripper.asm`）：`InitEnActiveAnimIndex_
/// NoInitOffset` 后 `jmp CommonEnemyStub2`（= Sidehopper 尾件，floor 动画）。
pub(crate) fn ripper_i_ai(mem: &mut crate::frame::FrameMemory, x: u8) {
    let sq = ENS + x as u16;
    if mem.read_wram(sq + F_STATUS) != STATUS_EXPLODE {
        init_en_active_anim_index_no_init_offset(mem, x);
    }
    sidehopper_ai(mem, x, false);
}

/// `RipperAIRoutine`（**bank2 版 = Ripper II**）：`$00←$03`/`$01←$03` 后直接 Decide
/// （喷焰型；帧步进 3 而非 Sidehopper 的 6/8）。
pub(crate) fn ripper_ii_ai(mem: &mut crate::frame::FrameMemory, x: u8) {
    let sq = ENS + x as u16;
    if mem.read_wram(sq + F_STATUS) != STATUS_EXPLODE {
        init_en_active_anim_index_no_init_offset(mem, x);
    }
    mem.write_ram(ANIM_ACTIVE_SLOT, 0x03);
    mem.write_ram(ANIM_RESTING_SLOT, 0x03);
    framework::update_enemy_common_decide(mem, x);
}

#[cfg(test)]
mod tests_sidehopper {
    use super::*;
    use crate::enemy::prep::{EN_F_DATA05, ENS_F_STATUS, ENS_F_TYPE};
    use crate::enemy::tables::{en_addr, ens_addr};
    use crate::frame::FrameMemory;

    fn stage(m: &mut FrameMemory, x: u8, type_: u8, status: u8) {
        game_data::pack::init_from_env().ok();
        m.write_ram(en_addr(x, 0), 0x40); // y
        m.write_ram(en_addr(x, 1), 0x40); // x
        m.write_ram(en_addr(x, EN_F_DATA05), 0x00); // 朝右
        m.write_wram(ens_addr(x, ENS_F_STATUS), status);
        m.write_wram(ens_addr(x, ENS_F_TYPE), type_);
    }

    /// `SidehopperFloorAIRoutine` 尾件：`$85/$86` = floor hopping 动画偏移、`$00/$01` = 6/8。
    #[test]
    fn floor_sets_hopping_anim_and_common_slots() {
        for (area, want) in [(0u8, 0x09u8), (2, 0x09), (4, 0x42)] {
            let mut m = FrameMemory::default();
            stage(&mut m, 0x20, 0, 1);
            m.write_ram(crate::ram_map::IN_AREA, area);
            sidehopper_ai(&mut m, 0x20, false);
            assert_eq!(m.read_ram(FLIP_ANIM_IDX), want, "area={area}");
            assert_eq!(m.read_ram(FLIP_ANIM_IDX + 1), want, "左朝向同值");
            assert_eq!(m.read_ram(ANIM_ACTIVE_SLOT), 0x06);
            assert_eq!(m.read_ram(ANIM_RESTING_SLOT), 0x08);
        }
    }

    /// Ceiling 变体用另一条动画（bank1 `$0F`、bank5 `$48`）。
    #[test]
    fn ceiling_uses_ceiling_anim() {
        for (area, want) in [(0u8, 0x0Fu8), (4, 0x48)] {
            let mut m = FrameMemory::default();
            stage(&mut m, 0x20, 1, 1);
            m.write_ram(crate::ram_map::IN_AREA, area);
            sidehopper_ai(&mut m, 0x20, true);
            assert_eq!(m.read_ram(FLIP_ANIM_IDX), want, "area={area}");
        }
    }

    /// 位移 8..15 ⇒ 切 hopping：animIndex=idx-1、resetAnimIndex=idx、animDelay=0。
    #[test]
    fn displacement_8_to_15_switches_to_hopping() {
        let mut m = FrameMemory::default();
        stage(&mut m, 0x20, 0, 2);
        m.write_ram(crate::ram_map::IN_AREA, 0);
        m.write_ram(FLIP_ANIM_IDX, 0x09);
        m.write_ram(FLIP_ANIM_IDX + 1, 0x09);
        m.write_wram(ens_addr(0x20, F_RESET_ANIM), 0x01);
        m.write_wram(ens_addr(0x20, F_ANIM_DELAY), 0x02);
        m.write_wram(ens_addr(0x20, F_JUMP_DSPL), 0x0B); // 11
        enemy_flip_after_displacement(&mut m, 0x20);
        assert_eq!(m.read_wram(ens_addr(0x20, F_RESET_ANIM)), 0x09);
        assert_eq!(m.read_wram(ens_addr(0x20, F_ANIM_INDEX)), 0x08);
        assert_eq!(m.read_wram(ens_addr(0x20, F_ANIM_DELAY)), 0x00);
    }

    /// 位移 <8 ⇒ 回休止动画（InitEnAnimIndex 径，取 ROM 表值）。
    #[test]
    fn displacement_below_8_returns_to_resting_anim() {
        let mut m = FrameMemory::default();
        stage(&mut m, 0x20, 0, 2);
        m.write_ram(crate::ram_map::IN_AREA, 0);
        m.write_wram(ens_addr(0x20, F_JUMP_DSPL), 0x03);
        enemy_flip_after_displacement(&mut m, 0x20);
        let area = crate::frame::cur_area(&m);
        let y = tables::enemy_type_times2_plus_facing(&mut m, 0x20);
        let want = game_data::assets::enemy_resting_anim(area)[y as usize & 0x1F];
        assert_eq!(m.read_wram(ens_addr(0x20, F_RESET_ANIM)), want);
        assert_eq!(m.read_wram(ens_addr(0x20, F_ANIM_INDEX)), want);
    }

    /// 位移 ≥16 ⇒ 不动（保留当前动画）。
    #[test]
    fn displacement_16_or_more_leaves_anim_alone() {
        let mut m = FrameMemory::default();
        stage(&mut m, 0x20, 0, 2);
        m.write_ram(crate::ram_map::IN_AREA, 0);
        m.write_wram(ens_addr(0x20, F_RESET_ANIM), 0x11);
        m.write_wram(ens_addr(0x20, F_ANIM_INDEX), 0x12);
        m.write_wram(ens_addr(0x20, F_JUMP_DSPL), 0x10);
        enemy_flip_after_displacement(&mut m, 0x20);
        assert_eq!(m.read_wram(ens_addr(0x20, F_RESET_ANIM)), 0x11);
        assert_eq!(m.read_wram(ens_addr(0x20, F_ANIM_INDEX)), 0x12);
    }

    /// 负位移取绝对值：-10（$F6）应落在 8..15 区间 ⇒ 切 hopping。
    #[test]
    fn negative_displacement_is_absoluted() {
        let mut m = FrameMemory::default();
        stage(&mut m, 0x20, 0, 2);
        m.write_ram(crate::ram_map::IN_AREA, 0);
        m.write_ram(FLIP_ANIM_IDX, 0x09);
        m.write_ram(FLIP_ANIM_IDX + 1, 0x09);
        m.write_wram(ens_addr(0x20, F_RESET_ANIM), 0x01);
        m.write_wram(ens_addr(0x20, F_JUMP_DSPL), 0xF6); // -10
        enemy_flip_after_displacement(&mut m, 0x20);
        assert_eq!(m.read_wram(ens_addr(0x20, F_RESET_ANIM)), 0x09);
    }

    /// `RipperAIRoutine`（bank1）复用 Sidehopper 尾件 ⇒ 与 floor 同形。
    #[test]
    fn ripper_in_brinstar_reuses_sidehopper_tail() {
        let mut m = FrameMemory::default();
        stage(&mut m, 0x20, 3, 1);
        m.write_ram(crate::ram_map::IN_AREA, 0);
        ripper_i_ai(&mut m, 0x20);
        assert_eq!(m.read_ram(FLIP_ANIM_IDX), 0x09);
        assert_eq!(m.read_ram(ANIM_ACTIVE_SLOT), 0x06);
    }

    /// Ripper II（Norfair bank2）：`$00/$01` 都写 3（喷焰帧步进）。
    ///
    /// 用 Explode 态（asm `@dontInitAnim` 同路）验证槽位写入——Norfair 的活动/休止
    /// 动画表值与本件动画流的搭配尚未取得像素证据，真机走 Resting 径会在帧表回绕守卫
    /// 上 fail-loud（`bank07:E0BC-wrap`），这是刻意的：Norfair 敌人一旦进用例必须先取证。
    #[test]
    fn ripper_ii_in_norfair_uses_three_frame_anim() {
        let mut m = FrameMemory::default();
        stage(&mut m, 0x20, 2, 3);
        m.write_ram(crate::ram_map::IN_AREA, 1);
        ripper_ii_ai(&mut m, 0x20);
        assert_eq!(m.read_ram(ANIM_ACTIVE_SLOT), 0x03);
        assert_eq!(m.read_ram(ANIM_RESTING_SLOT), 0x03);
    }

    /// **接线确认（dispatch 面，不是直接调本模块）**：走真实调用链
    /// `update_all_enemies → UpdateEnemy → choose_enemy_ai → sidehopper_ai/ripper_ai`，
    /// 证明 type 0/1/3 的敌人确实进到本件（而不只是「函数存在且自测通过」）。
    #[test]
    fn dispatch_wiring_reaches_this_module_for_types_0_1_3() {
        for (t, want_anim) in [(0u8, 0x09u8), (1, 0x0F), (3, 0x09)] {
            let mut m = FrameMemory::default();
            game_data::pack::init_from_env().ok();
            let x = 0x20u8;
            m.write_ram(crate::ram_map::IN_AREA, 0); // Brinstar
            m.write_ram(en_addr(x, 0), 0x40); // y
            m.write_ram(en_addr(x, 1), 0x50); // x
            m.write_ram(en_addr(x, EN_F_DATA05), 0x02); // bit1 闩 ⇒ 免 fresh 可见检
            m.write_wram(ens_addr(x, ENS_F_STATUS), 0x02); // Active
            m.write_wram(ens_addr(x, ENS_F_TYPE), t);
            m.write_wram(ens_addr(x, F_RESET_ANIM), 0xFF); // ≠ 目标 ⇒ flip 必写
            crate::enemy::slot_cycle::update_all_enemies(&mut m);
            // `$85/$86` 只由本件写（回退臂 `update_enemy_common_no_move_no_anim` 不碰它）
            // ⇒ 哨兵 0x00 变成目标值即为「进了本件」的充分证据。
            // 注意 `$00/$01` 是 asm 的临时量（后续 AI 会覆盖），不能在此断言。
            assert_eq!(m.read_ram(FLIP_ANIM_IDX), want_anim, "type={t} 未进本件");
            assert_eq!(m.read_ram(FLIP_ANIM_IDX + 1), want_anim, "type={t} 左朝向未写");
        }
    }
}
