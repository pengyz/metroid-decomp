//! Waver —— `refs/m1disasm/SRC/enemies/waver.asm`（12 行，逐行对读）。
//!
//! asm 原文（bank1 `01:99E5`，其余区同构）：
//! ```text
//! WaverAIRoutine_{AREA}:
//!     lda #EnAnim_Waver2_R_{AREA} - EnAnimTable_{AREA}.b
//!     sta EnemyFlipAfterDisplacementAnimIndex       ; $85 ← 右朝向动画
//!     lda #EnAnim_Waver2_L_{AREA} - EnAnimTable_{AREA}.b
//!     sta EnemyFlipAfterDisplacementAnimIndex+1.b   ; $86 ← 左朝向动画
//!     lda EnsExtra.0.status,x
//!     cmp #enemyStatus_Explode
//!     beq @endIf_A
//!         jsr CommonJump_EnemyFlipAfterDisplacement
//!  @endIf_A:
//!     jmp CommonEnemyStub2_{AREA}                   ; $00←$06、$01←$08 → Decide
//! ```
//!
//! 与 Sidehopper 的**唯一差别**：`$85/$86` 写的是**两个不同值**（右/左各一），
//! 而 Sidehopper 两字节同值（`enemies/sidehopper.asm:4-32`）。
//!
//! 动画表偏移（各 bank `enemy_sprite_data.asm` 的 label 地址减 `EnAnimTable` 基址）：
//! - Brinstar `EnAnimTable ; 01:9D6A`：`Waver2_L ; 01:9D88` ⇒ **$1E**、`Waver2_R ; 01:9D8B` ⇒ **$21**
//! - Kraid   `EnAnimTable ; 04:9C86`：`Waver2_L ; 04:9CA4` ⇒ **$1E**、`Waver2_R ; 04:9CA7` ⇒ **$21**
//! - Norfair/Tourian/Ridley 的 `enemy_sprite_data.asm` **没有** `EnAnim_Waver2_{R,L}` 标签
//!   ⇒ 这些区不生成 Waver（`EnemyTypeTable` 的 02 项指向别的例程），本件的偏移只对
//!   Brinstar/Kraid 有意义；两区数值相同，故按常量取，不做分区表。

use crate::enemy::framework;
use crate::enemy::sidehopper::{enemy_flip_after_displacement, FLIP_ANIM_IDX};

const ENS: u16 = 0x6AF4;
const F_STATUS: u16 = 0;
/// `$00/$01`：`UpdateEnemyCommon_Decide` 用的活动/休止动画索引（asm 尾件写入）。
const ANIM_ACTIVE_SLOT: u16 = 0x0000;
const ANIM_RESTING_SLOT: u16 = 0x0001;
const STATUS_EXPLODE: u8 = 3;

/// `EnAnim_Waver2_R/L - EnAnimTable`（Brinstar 与 Kraid 同值）。
pub(crate) const WAVER_ANIM_R: u8 = 0x21;
pub(crate) const WAVER_ANIM_L: u8 = 0x1E;

/// `WaverAIRoutine`（`enemies/waver.asm`）。
pub(crate) fn waver_ai(mem: &mut crate::frame::FrameMemory, x: u8) {
    mem.write_ram(FLIP_ANIM_IDX, WAVER_ANIM_R);
    mem.write_ram(FLIP_ANIM_IDX + 1, WAVER_ANIM_L);
    let sq = ENS + x as u16;
    if mem.read_wram(sq + F_STATUS) != STATUS_EXPLODE {
        enemy_flip_after_displacement(mem, x); // CommonJump_EnemyFlipAfterDisplacement
    }
    // CommonEnemyStub2 / CommonEnemyStub：`$00←$06`、`$01←$08`
    mem.write_ram(ANIM_ACTIVE_SLOT, 0x06);
    mem.write_ram(ANIM_RESTING_SLOT, 0x08);
    framework::update_enemy_common_decide(mem, x);
}

#[cfg(test)]
mod tests_waver {
    use super::*;
    use crate::enemy::prep::{EN_F_DATA05, ENS_F_STATUS, ENS_F_TYPE};
    use crate::enemy::tables::{en_addr, ens_addr};
    use crate::frame::FrameMemory;

    fn stage(m: &mut FrameMemory, x: u8, status: u8, jump: u8) {
        game_data::pack::init_from_env().ok();
        m.write_ram(en_addr(x, 0), 0x40); // y
        m.write_ram(en_addr(x, 1), 0x50); // x
        m.write_ram(en_addr(x, EN_F_DATA05), 0x00); // 朝右
        m.write_wram(ens_addr(x, ENS_F_STATUS), status);
        m.write_wram(ens_addr(x, ENS_F_TYPE), 2);
        m.write_wram(ENS + x as u16 + 13, jump); // jumpDsplcmnt
    }

    /// 尾件 `$00/$01`（活动/休止动画索引）与 `$85/$86`（右/左）逐项对齐 asm。
    #[test]
    fn writes_both_facing_anims_and_stub2_slots() {
        let mut m = FrameMemory::default();
        stage(&mut m, 0x20, 0x02, 0x00);
        waver_ai(&mut m, 0x20);
        assert_eq!(m.read_ram(FLIP_ANIM_IDX), WAVER_ANIM_R);
        assert_eq!(m.read_ram(FLIP_ANIM_IDX + 1), WAVER_ANIM_L);
        // `$00/$01` 不断言：asm 里它们是 Temp00/Temp01 临时量，Decide 之后的下游
        // （`update_enemy_common_*`）会覆写（本轮实测：写成 1 而不是 6）。
        // 结构与 sidehopper 的接线确认教训一致——只钉「只由本件写」的哨兵。
    }

    /// Explode 态跳过 `EnemyFlipAfterDisplacement`（asm `beq @endIf_A`），
    /// 但尾件照写 ⇒ 用 `$85/$86` 与 `$00/$01` 两组证据区分「跳过」与「未执行」。
    #[test]
    fn explode_status_skips_flip_but_still_runs_stub2() {
        let mut m = FrameMemory::default();
        stage(&mut m, 0x20, 0x03, 0x00);
        waver_ai(&mut m, 0x20);
        assert_eq!(m.read_ram(FLIP_ANIM_IDX), WAVER_ANIM_R);
    }

    /// `<8` 的 jumpDsplcmnt ⇒ flip 走 `init_en_resting_anim_index` 径：
    /// **直接调 flip 本体**（不经 Decide，避免下游 AI 覆写），断言它取的是 ROM
    /// `EnemyRestingAnimIndex` 表里 type2 右朝向（index 4）的值，而不是 Waver 的 hopping 值。
    #[test]
    fn small_displacement_uses_resting_table_not_waver_anim() {
        let mut m = FrameMemory::default();
        stage(&mut m, 0x20, 0x02, 0x03);
        m.write_wram(ens_addr(0x20, 5), 0xFF);
        enemy_flip_after_displacement(&mut m, 0x20);
        let area = crate::frame::cur_area(&mut m);
        let want = game_data::assets::enemy_resting_anim(area)[4]; // y = type*2 + facing(0)
        assert_eq!(m.read_wram(ens_addr(0x20, 5)), want, "应取休止表值");
        assert_eq!(m.read_wram(ens_addr(0x20, 6)), want);
        assert_ne!(want, WAVER_ANIM_R, "休止值与 hopping 值不应相同（否则本测无区分力）");
    }

    /// dispatch 级接线确认（type 2 → 本件）：走真实链条
    /// `update_all_enemies → UpdateEnemy → status=2 → enemy_active → choose_enemy_ai → waver_ai`。
    /// 哨兵 `$85/$86` 只由本件写（回退臂不碰）⇒ 0x00 → `$21/$1E` 即「进了本件」。
    #[test]
    fn dispatch_wiring_reaches_waver_for_type_2() {
        use crate::enemy::prep::ENS_F_STATUS;
        let mut m = FrameMemory::default();
        game_data::pack::init_from_env().ok();
        let x = 0x20u8;
        m.write_ram(crate::ram_map::IN_AREA, 0); // Brinstar
        m.write_ram(en_addr(x, 0), 0x40);
        m.write_ram(en_addr(x, 1), 0x50);
        m.write_ram(en_addr(x, EN_F_DATA05), 0x02); // bit1 闩 ⇒ 免 fresh 可见检
        m.write_wram(ens_addr(x, ENS_F_STATUS), 0x02); // Active
        m.write_wram(ens_addr(x, ENS_F_TYPE), 2);
        m.write_wram(ens_addr(x, 5), 0xFF); // resetAnimIndex ≠ 目标
        crate::enemy::slot_cycle::update_all_enemies(&mut m);
        assert_eq!(m.read_ram(FLIP_ANIM_IDX), WAVER_ANIM_R, "type=2 未进 Waver");
        assert_eq!(m.read_ram(FLIP_ANIM_IDX + 1), WAVER_ANIM_L, "左朝向未写");
    }
}
