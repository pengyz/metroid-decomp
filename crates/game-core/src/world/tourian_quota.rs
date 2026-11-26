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
//!     bne LDD30              ; 恒跳：读的是上一条 `iny` 的 Z（Y=1⇒Z=0），与 A 无关
//! ```
//! 触发点是「掉落生成失败」的两处 `beq/bcc LDD5B`（`07:DD28` 的 1/16 无掉、
//! `07:DD34`/`07:DD46`/`07:DD5B` 的配额打满），全部在 `enemy::framework::enemy_complete_death`。
//!
//! 语义与卡面文字的差异（已报 PMO）：
//! 1. 卡面写 `InArea & 0x0F == 3`，ROM 是 `cmp #$13` **精确比较**。`InArea` 高半字节带
//!    「进区方向」等附加位（见 `frame::cur_area` 文档），`$10` 位有语义 ⇒ 这里按 ROM
//!    比对**原值**，不做 `& 0x0F`。
//! 2. ROM 原版 quirk：`lda RandomNumber1` 是**死读**。紧随其后的 `ldy #$00` 与三条
//!    `sty` 都不改标志，`bne LDD30` 读的是 `iny`（Y:0→1）留下的 Z=0 ⇒ **无条件跳回
//!    LDD30**。所以 Tourian 下这条径没有分支：复位配额 → 重试生成。
//!    （对拍 reviewer 2026-11-27 的 A 级字节核：`a5 2e / a0 00 / 84 96 / 84 95 / c8 /
//!    84 93 / 84 94 / d0 bb`，`d0` 的 Z 来自 `c8 iny`。）
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

/// `LDD5B` 的两种出口（`bne LDD30` 恒真，没有第三条路）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum QuotaReset {
    /// 非 Tourian：`jmp RemoveEnemy`（回收敌人槽）。
    Remove,
    /// Tourian：配额已复位（`$93/$94←1`、`$95/$96←0`），回 `LDD30` 重试一次生成。
    Retry,
}

/// `LDD5B ; 07:DD5B`：掉落生成失败后的配额复位。
///
/// 只写 `$93-$96` 四字节，不碰敌人槽、不碰音乐/石像（那两件在 `LDD75`，属另一段）。
pub(crate) fn ldd5b(mem: &mut Manager) -> QuotaReset {
    if !in_tourian(mem) {
        return QuotaReset::Remove;
    }
    // 顺序照 asm：先清两个 Cur，再写两个 Max=1。
    mem.set_pickup_energy_qty_cur(0);
    mem.set_pickup_missile_qty_cur(0);
    mem.set_pickup_missile_qty_max(1);
    mem.set_pickup_energy_qty_max(1);
    // `bne LDD30`：Z 由 `iny` 决定（Y=1），恒真 ⇒ 没有 `RandomNumber1 == 0` 的特殊出口。
    QuotaReset::Retry
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

        let out = ldd5b(&mut m);

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

        let out = ldd5b(&mut m);

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
        assert_eq!(ldd5b(&mut m), QuotaReset::Retry);
        // 重试后成功掉了一个导弹：Cur 涨到 1 == Max ⇒ 下一次失败会再进 LDD5B。
        m.set_pickup_missile_qty_cur(1);
        m.set_pickup_missile_qty_max(1);

        assert_eq!(ldd5b(&mut m), QuotaReset::Retry);
        assert_eq!(m.pickup_missile_qty_cur(), 0);
        assert_eq!(m.pickup_missile_qty_max(), 1);
    }

    /// ④`lda RandomNumber1` 是死读：`bne LDD30` 的 Z 来自 `c8 iny`（Y=1）⇒ 恒跳。
    /// A 级字节核（`07:DD66` 起）：`a5 2e / a0 00 / 84 96 / 84 95 / c8 / 84 93 / 84 94 / d0 bb`。
    #[test]
    fn bne_is_decided_by_iny_not_by_random1() {
        let mut m = tourian_with_full_quotas();
        m.set_random1(0x00); // 旧实现会误判成「落 LDD75 死循环」的那个取值

        let out = ldd5b(&mut m);

        assert_eq!(out, QuotaReset::Retry, "Rnd=0 也回 LDD30；`bne` 只认 iny 的 Z=0");
        assert_eq!(m.pickup_energy_qty_cur(), 0, "$96 清");
        assert_eq!(m.pickup_missile_qty_cur(), 0, "$95 清");
        assert_eq!(m.pickup_missile_qty_max(), 1, "$93←1");
        assert_eq!(m.pickup_energy_qty_max(), 1, "$94←1");
    }

    /// ⑤`InArea=$1F`（高半字节带位、低半字节仍是 3）：ROM 精确比较 ⇒ 非 Tourian。
    #[test]
    fn in_area_with_other_high_bits_is_not_tourian() {
        let mut m = tourian_with_full_quotas();
        m.set_in_area(0x1F);
        assert!(!in_tourian(&m));
        assert_eq!(ldd5b(&mut m), QuotaReset::Remove);
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

    /// Tourian + `r==0`：仍走重试生成（旧实现误判成死循环的那个取值）。
    #[test]
    fn tourian_zero_random_still_spawns_pickup() {
        if !game_data::assets::ensure_ready() {
            eprintln!("SKIP：assets/metroid.pack 未就绪（敌表读取需要它）");
            return;
        }
        let mut m = exploding_enemy(TOURIAN_IN_AREA, 0x00);
        crate::enemy::framework::enemy_complete_death(&mut m, PX);

        assert_eq!(m.pickup_missile_qty_max(), 1, "$93←1（复位发生）");
        assert_eq!(m.pickup_energy_qty_max(), 1, "$94←1");
        assert_eq!(
            m.enemy(slot_of(PX)).status_raw(),
            EnemyStatus::Pickup.to_byte(),
            "ROM 里 bne 由 iny 决定 ⇒ Rnd=0 也回 LDD30 重试生成"
        );
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
