//! Multiviola —— `refs/m1disasm/SRC/enemies/multiviola.asm`（11 行 / 10 条指令，
//! 逐行对读）。asm 原文（bank2 `02:9704` 起；本件被 prg2_norfair.asm:679 **与**
//! prg5_ridley.asm:654 include——本卡只接 Norfair 跳表 0x0C 项）：
//! ```text
//! MultiviolaAIRoutine_{AREA}:
//!     lda EnsExtra.0.status,x / cmp #enemyStatus_Active / bne @dontInitAnim
//!         jsr CommonJump_InitEnActiveAnimIndex_NoInitOffset    ; 07:FBCA
//! @dontInitAnim:
//!     lda #$02 / sta $00 / sta $01     ; 活动/休息 anim 节拍均 2
//!     jmp UpdateEnemyCommon_Decide     ; $0081 分流尾件
//! ```
//!
//! 语义要点：
//! - **读现役 status**（非 `$0081` 快照）——激活帧 try_becoming_active 已置 2 时可达。
//! - 动画初始化是 `NoInitOffset` 变体：不做 `EnemyActiveAnimInitOffset` 递减
//!   （对比 `InitEnActiveAnimIndex 07:F699`——framework.rs 两件并存）。
//! - 移动/反弹径不在本例程里（Mulitviola 的反弹走 movement 指令串），AI 只管
//!   动画 + Decide 分流。

use crate::enemy::framework;
use crate::frame::Manager;
use crate::ram_map;

/// 零页 anim 节拍预设槽（`$00`=活动 / `$01`=休息）。
/// `MultiviolaAIRoutine`。
pub(crate) fn multiviola_ai(mem: &mut Manager, x: u8) {
    let slot = x / 16;
    // `cmp #enemyStatus_Active / bne @dontInitAnim`——现役 status（非快照）。
    if mem.enemy(slot).status_raw() == 2 {
        framework::init_en_active_anim_index_no_init_offset(mem, x);
    }
    // `lda #$02 / sta $00 / sta $01` + `jmp UpdateEnemyCommon_Decide`
    // （零页槽是纯中间量 ⇒ 即时量直传 Decide 变体，见 framework 注释）。
    framework::update_enemy_common_decide_with_beats(mem, x, 0x02, 0x02);
}

#[cfg(test)]
mod tests_multiviola {
    use super::*;
    use crate::enemy::prep::{ENS_F_STATUS, ENS_F_TYPE};
    use crate::enemy::tables::{en_addr, ens_addr};
    use crate::ram_map::{ENEMY_STATUS_PRE_AI, IN_AREA};

    const X: u8 = 0x20;

    fn stage(m: &mut Manager, pre: u8, status: u8) {
        game_data::pack::init_from_env().ok();
        m.write_ram(IN_AREA, 1);
        m.write_ram(ENEMY_STATUS_PRE_AI, pre);
        m.write_wram(ens_addr(X, ENS_F_STATUS), status);
        m.write_wram(ens_addr(X, ENS_F_TYPE), 0x0C); // Multiviola
    }

    /// 现役 Active ⇒ NoInitOffset 初始化：resetAnimIndex/animIndex ← activeAnim[y]。
    #[test]
    fn active_inits_anim_no_init_offset() {
        let mut m = Manager::default();
        stage(&mut m, 2, 2);
        // want 在 AI **前**取：动画初始化发生在例程开头、Decide 改 facing 之前。
        let area = 1u8;
        m.set_page_index(X); // 敌面读取按 asm 同款页窗（与例程内一致）
        let y = crate::enemy::tables::enemy_type_times2_plus_facing(&mut m, X);
        let want = game_data::assets::enemy_active_anim(area)[y as usize & 0x1F];
        multiviola_ai(&mut m, X);
        assert_eq!(m.enemy(X / 16).reset_anim_index(), want, "resetAnimIndex ← activeAnim[y]");
        // animIndex 由 Decide 的动画 tick 推一格（delay=0 ⇒ 必进一格），reset 保持。
        assert!(m.enemy(X / 16).anim_index() == want || m.enemy(X / 16).anim_index() == want + 1,
            "animIndex 初始化为 want 后最多被 tick 推一格");
        // （animDelay 已被 Decide 的 tick 按表项重写，不再断言 0。）
    }

    /// 已在 active 动画（resetAnimIndex==activeAnim[y]）⇒ 不重写（asm `beq @RTS`）。
    #[test]
    fn already_active_anim_not_rewritten() {
        let mut m = Manager::default();
        stage(&mut m, 2, 2);
        // want 在 AI 前取（初始化发生在 Decide 改 facing 之前）。
        m.set_page_index(X); // 敌面读取按 asm 同款页窗（与例程内一致）
        let y = crate::enemy::tables::enemy_type_times2_plus_facing(&mut m, X);
        let want = game_data::assets::enemy_active_anim(1)[y as usize & 0x1F];
        m.enemy(X / 16).set_reset_anim_index(want);
        m.enemy(X / 16).set_anim_index(0x55);
        m.enemy(X / 16).set_anim_delay(7);
        multiviola_ai(&mut m, X);
        let ai = m.enemy(X / 16).anim_index();
        assert!(ai == 0x55 || ai == 0x56, "哨兵保留 ⇒ init 未重写（got {ai:#04X}）");
    }

    /// 非 Active（Resting pre 径）⇒ 不做 active 动画初始化（reset 保持预设值）。
    #[test]
    fn non_active_skips_anim_init() {
        let mut m = Manager::default();
        stage(&mut m, 1, 1);
        m.enemy(X / 16).set_reset_anim_index(0);
        multiviola_ai(&mut m, X);
        assert_eq!(m.enemy(X / 16).reset_anim_index(), 0, "非 Active 不初始化动画");
    }

    /// 快照径回归：pre==Resting 且现役 Active（激活帧形态）——本例程读**现役** status，
    /// 动画初始化仍发生（对比 crawler 读快照拦 Resting 的分叉）。
    #[test]
    fn pre_resting_active_still_inits_anim() {
        let mut m = Manager::default();
        stage(&mut m, 1, 2); // pre=Resting、现役已 Active
        multiviola_ai(&mut m, X);
        let y = crate::enemy::tables::enemy_type_times2_plus_facing(&mut m, X);
        let want = game_data::assets::enemy_active_anim(1)[y as usize & 0x1F];
        assert_eq!(m.enemy(X / 16).reset_anim_index(), want, "读现役 ⇒ 初始化发生");
    }
}
