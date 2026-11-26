//! Tourian 掉落配额复位（`LDD5B ; 07:DD5B`）——#117。
//!
//! A 级 ROM 依据（`refs/m1disasm/SRC/prg7_engine.asm:6018-6042`）：
//! ```text
//! LDD5B:                     ; 07:DD5B  pickup failed to spawn
//!     ldx PageIndex
//!     lda InArea
//!     cmp #$13               ; ← 精确比较，不是掩码
//!     beq Lx140
//! Lx139:  jmp RemoveEnemy    ; ($FA18) 非 Tourian：直接回收敌人槽
//! Lx140:                     ; 在 Tourian：把「配额打满」判据强行清掉，再生成一次
//!     lda RandomNumber1
//!     ldy #$00
//!     sty EnergyPickupQtyCur ; $96 ← 0
//!     sty MissilePickupQtyCur; $95 ← 0
//!     iny
//!     sty MissilePickupQtyMax; $93 ← 1
//!     sty EnergyPickupQtyMax ; $94 ← 1
//!     bne LDD30              ; 用 A=Rnd≠0 判；A==0 时**不跳**、落进 LDD75
//! ```
//! 触发点是「掉落生成失败」的两处 `beq/bcc LDD5B`（`07:DD28` 的 1/16 无掉、
//! `07:DD34`/`07:DD46`/`07:DD5B` 的配额打满），全部在 `enemy::framework::enemy_complete_death`。
//!
//! 语义与卡面文字的差异（已报 PMO）：
//! 1. 卡面写 `InArea & 0x0F == 3`，ROM 是 `cmp #$13` **精确比较**。`InArea` 高半字节带
//!    「进区方向」等附加位（见 `frame::cur_area` 文档），`$10` 位有语义 ⇒ 这里按 ROM
//!    比对**原值**，不做 `& 0x0F`。
//! 2. `bne LDD30` 之后若 `RandomNumber1 == 0`，`bne` 不成立 ⇒ 落入 `LDD75`
//!    （播 PowerUpMusic、`MiniBossKillDelayFlag←InArea&$0F`、Kraid/Ridley 石像闪烁位、
//!    `AddToMaxMissiles(75)`），其末尾 `bne LDD5B` 又跳回本段，而 `RandomNumber1` 不变
//!    ⇒ ROM 在这条径上是**死循环**（每圈 +75 导弹并重播音乐）。本模块只把它归类成
//!    [`QuotaReset::HungQuirk`]，不自行决定「是否终止循环」。
//!
//! 四个字节（`constants_ram.asm`）：
//! `$93 MissilePickupQtyMax`、`$94 EnergyPickupQtyMax`、
//! `$95 MissilePickupQtyCur`、`$96 EnergyPickupQtyCur`。

use crate::frame::Manager;

/// ROM `cmp #$13` 的**精确**目标值（Tourian；`$10` 位参与比较）。
pub(crate) const TOURIAN_IN_AREA: u8 = 0x13;

/// `lda InArea / cmp #$13 / beq Lx140`：只有原值 `$13` 走 Tourian 支路。
pub(crate) fn in_tourian(mem: &Manager) -> bool {
    mem.in_area_raw() == TOURIAN_IN_AREA
}

/// `LDD5B` 的三种出口。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum QuotaReset {
    /// 非 Tourian：`jmp RemoveEnemy`（回收敌人槽）。
    Remove,
    /// Tourian 且 `RandomNumber1 != 0`：配额已复位，回到 `LDD30` 重试一次生成。
    Retry,
    /// Tourian 且 `RandomNumber1 == 0`：`bne LDD30` 不成立 ⇒ 落进 `LDD75`，
    /// 而 `LDD75` 末尾 `bne LDD5B` 又跳回本段 ⇒ ROM 死循环（见模块文档第 2 条）。
    HungQuirk,
}

/// `LDD5B ; 07:DD5B`：掉落生成失败后的配额复位。
///
/// 只写 `$93-$96` 四字节，不碰敌人槽、不碰音乐/石像（那两件在 `LDD75`，属另一段）。
pub(crate) fn ldd5b(mem: &mut Manager, random1: u8) -> QuotaReset {
    if !in_tourian(mem) {
        return QuotaReset::Remove;
    }
    // 顺序照 asm：先清两个 Cur，再写两个 Max=1。
    mem.set_pickup_energy_qty_cur(0);
    mem.set_pickup_missile_qty_cur(0);
    mem.set_pickup_missile_qty_max(1);
    mem.set_pickup_energy_qty_max(1);
    if random1 == 0 {
        QuotaReset::HungQuirk
    } else {
        QuotaReset::Retry
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 进 Tourian（`InArea=$13`）并把配额预置成「已打满」的典型值：
    /// 区初始化/门入场会把 Max 写成 $27/$2F 或 `Rnd&$0F` 族、Cur 清 0。
    fn tourian_with_full_quotas() -> Manager {
        let mut m = Manager::default();
        m.set_in_area(TOURIAN_IN_AREA);
        m.set_pickup_missile_qty_max(0x2F);
        m.set_pickup_energy_qty_max(0x2F);
        m.set_pickup_missile_qty_cur(0x2F);
        m.set_pickup_energy_qty_cur(0x2F);
        m
    }

    /// ①Tourian 首次进入（掉落失败）：`$96/$95 ← 0`、`$93/$94 ← 1`。
    #[test]
    fn tourian_first_entry_resets_quotas() {
        let mut m = tourian_with_full_quotas();
        assert!(in_tourian(&m));

        let out = ldd5b(&mut m, 0x2A);

        assert_eq!(out, QuotaReset::Retry, "Rnd=0x2A≠0 ⇒ 回 LDD30 重试");
        assert_eq!(m.pickup_energy_qty_cur(), 0, "$96 EnergyPickupQtyCur");
        assert_eq!(m.pickup_missile_qty_cur(), 0, "$95 MissilePickupQtyCur");
        assert_eq!(m.pickup_missile_qty_max(), 1, "$93 MissilePickupQtyMax");
        assert_eq!(m.pickup_energy_qty_max(), 1, "$94 EnergyPickupQtyMax");
    }

    /// ②非 Tourian（`InArea=$03`）：四个字节一个都不许改，结论=RemoveEnemy。
    #[test]
    fn non_tourian_leaves_quotas_untouched() {
        let mut m = Manager::default();
        m.set_in_area(0x03); // 低半字节=3（Tourian 区号）但原值≠$13：ROM 走 RemoveEnemy
        m.set_pickup_missile_qty_max(0x2F);
        m.set_pickup_energy_qty_max(0x2F);
        m.set_pickup_missile_qty_cur(0x05);
        m.set_pickup_energy_qty_cur(0x07);

        let out = ldd5b(&mut m, 0x2A);

        assert_eq!(out, QuotaReset::Remove);
        assert_eq!(m.pickup_missile_qty_max(), 0x2F);
        assert_eq!(m.pickup_energy_qty_max(), 0x2F);
        assert_eq!(m.pickup_missile_qty_cur(), 0x05);
        assert_eq!(m.pickup_energy_qty_cur(), 0x07);
    }

    /// ③重复失败：ROM 每次都复位（不是「只第一次」）——第二次仍把 Max 拉回 1。
    /// 这是卡面「重复进入不重复重置」的反例，按 ROM 为准（见模块文档）。
    #[test]
    fn repeated_failure_resets_again() {
        let mut m = tourian_with_full_quotas();
        assert_eq!(ldd5b(&mut m, 0x2A), QuotaReset::Retry);
        // 重试后成功掉了一个导弹：Cur 涨到 1 == Max ⇒ 下一次失败会再进 LDD5B。
        m.set_pickup_missile_qty_cur(1);
        m.set_pickup_missile_qty_max(1);

        assert_eq!(ldd5b(&mut m, 0x2A), QuotaReset::Retry);
        assert_eq!(m.pickup_missile_qty_cur(), 0);
        assert_eq!(m.pickup_missile_qty_max(), 1);
    }

    /// ④`RandomNumber1 == 0`：配额仍复位，但结论是 HungQuirk（ROM 落 LDD75 后死循环）。
    #[test]
    fn random_zero_reports_hung_quirk_after_reset() {
        let mut m = tourian_with_full_quotas();

        let out = ldd5b(&mut m, 0x00);

        assert_eq!(out, QuotaReset::HungQuirk);
        assert_eq!(m.pickup_energy_qty_cur(), 0, "复位仍发生（在 bne 之前）");
        assert_eq!(m.pickup_missile_qty_cur(), 0);
        assert_eq!(m.pickup_missile_qty_max(), 1);
        assert_eq!(m.pickup_energy_qty_max(), 1);
    }

    /// ⑤`InArea=$1F`（高半字节带位、低半字节仍是 3）：ROM 精确比较 ⇒ 非 Tourian。
    #[test]
    fn in_area_with_other_high_bits_is_not_tourian() {
        let mut m = tourian_with_full_quotas();
        m.set_in_area(0x1F);
        assert!(!in_tourian(&m));
        assert_eq!(ldd5b(&mut m, 0x2A), QuotaReset::Remove);
        assert_eq!(m.pickup_missile_qty_max(), 0x2F, "非 Tourian 不复位");
    }
}

#[cfg(test)]
mod hook_tests {
    //! 挂钩验收：直接走 ROM 的真实入口 `enemy::framework::enemy_complete_death`
    //! （`bcc LDD5B` 那里），确认 Tourian 下不再是「直接回收」，而是复位配额后重试生成。

    use super::*;
    use crate::enemy::state::{slot_of, EnemyStatus};

    const PX: u8 = 0x40;

    fn exploding_enemy(area: u8, rnd: u8) -> Manager {
        let mut m = Manager::default();
        m.set_in_area(area);
        m.set_random1(rnd);
        m.enemy(slot_of(PX)).set_status(EnemyStatus::Explode);
        m.enemy(slot_of(PX)).set_prev_status(0x01); // Resting，无小 Boss 位（bit6）
        m
    }

    /// Tourian + `r<0x10`（1/16 无掉）：配额复位 + 敌人转成拾取物（不再 NoEnemy）。
    #[test]
    fn tourian_drop_failure_retries_pickup() {
        if !game_data::assets::ensure_ready() {
            eprintln!("SKIP：assets/metroid.pack 未就绪（敌表读取需要它）");
            return;
        }
        let mut m = exploding_enemy(TOURIAN_IN_AREA, 0x05);
        crate::enemy::framework::enemy_complete_death(&mut m, PX);

        assert_eq!(m.pickup_missile_qty_cur(), 0, "$95 清");
        assert_eq!(m.pickup_energy_qty_cur(), 0, "$96 清");
        assert_eq!(m.pickup_missile_qty_max(), 1, "$93←1");
        assert_eq!(m.pickup_energy_qty_max(), 1, "$94←1");
        assert_eq!(
            m.enemy(slot_of(PX)).status_raw(),
            EnemyStatus::Pickup.to_byte(),
            "重试生成 ⇒ status=Pickup（不是 NoEnemy）"
        );
        assert_eq!(m.enemy(slot_of(PX)).anim_frame(), 0x89, "PickupTypeTable[0x05&7=5]");
    }

    /// 非 Tourian + `r<0x10`：保持原行为（回收槽、配额一字不动）。
    #[test]
    fn non_tourian_drop_failure_still_removes() {
        let mut m = exploding_enemy(0x02, 0x05);
        m.set_pickup_missile_qty_max(0x2F);
        crate::enemy::framework::enemy_complete_death(&mut m, PX);

        assert_eq!(m.pickup_missile_qty_max(), 0x2F, "非 Tourian 不复位");
        assert_eq!(
            m.enemy(slot_of(PX)).status_raw(),
            EnemyStatus::NoEnemy.to_byte(),
            "非 Tourian ⇒ RemoveEnemy"
        );
    }
}
