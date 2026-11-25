//! 死亡链（B4-b / #54）：`IsSamusDead` 谓词与 `CheckHealthStatus` 的 CE1E 死门。
//!
//! ## ROM 依据（A 级）
//!
//! `IsSamusDead ; 07:CE84`（`refs/m1disasm/SRC/prg7_engine.asm`）原文：
//!
//! ```text
//! IsSamusDead:
//!     lda Samus.status
//!     cmp #sa_Dead
//!     beq @RTS
//!     cmp #sa_Dead2
//!     beq @RTS
//!     cmp #sa_Begin       ; $FF
//! @RTS:
//!     rts
//! ```
//!
//! 调用方一律取 **Z 位**（`jsr IsSamusDead / bne <非死>`）⇒ 语义是
//! **`status ∈ {sa_Dead(7), sa_Dead2(8), sa_Begin($FF)}`**。末条 `cmp #sa_Begin` 让
//! `$FF` 也落进「死」一侧——这是原版 quirk，Stage A 保真保留。
//!
//! 三个调用者（xref 7，本例程被内联在伤害面之前的有多处，此处只列本轮相关的）：
//! - `CheckHealthStatus ; 07:CE1E` —— 命中面写完 `speedY=$FD`/`AccelY=$38` 之后判死，
//!   死则 `jmp CheckHealthBeep`（跳过 i 帧倒数与击退 X 速）。**本卡接线处见
//!   [`crate::samus::physics::check_health_status`]**。
//! - `SubtractHealth ; 07:CEA3` —— 门位在 Varia 半伤/BCD 扣减之前；Rust 侧早已内联于
//!   [`crate::weapon::projectile::subtract_health`]（早于本卡，不改）。
//! - `CollisionDetection ; 07:10076/10125/10179/10205` —— 碰撞面，属 `world/` 写面，本卡不改。
//!
//! ## 与「重开」的边界
//!
//! 本例程只回答「死没死」。死亡之后的计时（爆炸 25 帧 → `sa_Dead2` + `animDelay=$28` →
//! 归零 → 引擎门 `SilenceMusic` → `SetTimer($04, routine 4)` → `PrepareGameOver`）分散在
//! `render/oam.rs::samus_explode_draw`、`samus/handler.rs::samus_dead2`、`frame/tick.rs` 的
//! 引擎尾件，本轮不改；链图与逐帧表见 `docs/spec/player-death-chain.md`。

use crate::frame::Manager;
use crate::samus::state::SamusStatus;

/// `sa_Dead`（`Samus.status` 采样值 7）。
pub const SA_DEAD: u8 = 7;
/// `sa_Dead2`（爆炸结束后的第二段死亡态，8）。
pub const SA_DEAD2: u8 = 8;
/// `sa_Begin`（`$FF`）。ROM 的 `IsSamusDead` 用 `cmp #sa_Begin` 收尾，因此它也返回 Z=1。
pub const SA_BEGIN: u8 = 0xFF;

/// `IsSamusDead ; 07:CE84`：`true` 表示 Z 位为 1（死 / 死二段 / `sa_Begin`）。
///
/// 与 ROM 一样只看 `Samus.status` 一个字节，不读血量、不读计时器。
pub fn is_samus_dead(mem: &mut Manager) -> bool {
    let status = mem.samus().status_raw();
    status == SA_DEAD || status == SA_DEAD2 || status == SA_BEGIN
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ram_map::{SAMUS_ACCEL_Y, SAMUS_INVINCIBLE_DELAY};
    use crate::samus::physics::SAMUS_SPEED_Y_I;

    fn with_status(v: u8) -> Manager {
        let mut m = Manager::default();
        // `from_u8` 对未定义值 fail-loud（见 state.rs 的说明），测试要造 $FF/$FE 这类
        // 「ROM 行为空间外」的字节，因此用 `Undefined(v)` 显式构造（往返不丢原值）。
        let st = SamusStatus::from_u8(v).unwrap_or(SamusStatus::Undefined(v));
        m.samus().set_status(st);
        m
    }

    /// 真值表：ROM 原文三条 `cmp` 覆盖的取值 + 两个反例。
    #[test]
    fn is_samus_dead_truth_table() {
        for v in [0u8, 1, 2, 3, 4, 5, 6, 9, 0x10, 0xFE] {
            assert!(!is_samus_dead(&mut with_status(v)), "status=$v 不应算死");
        }
        for v in [SA_DEAD, SA_DEAD2, SA_BEGIN] {
            assert!(is_samus_dead(&mut with_status(v)), "status=$v 应算死（07:CE84）");
        }
        // 枚举面交叉验证：Dead/Dead2 与 `Undefined($FF)` 两条路径都要认。
        let mut m = Manager::default();
        m.samus().set_status(SamusStatus::Dead);
        assert!(is_samus_dead(&mut m));
        let mut m = Manager::default();
        m.samus().set_status(SamusStatus::Dead2);
        assert!(is_samus_dead(&mut m));
        assert!(!is_samus_dead(&mut Manager::default()), "默认 Standing 不算死");
    }

    /// CE1E 死门：死时**先**写击退初速（asm 顺序），然后跳过 i 帧倒数、直跳 CheckHealthBeep。
    #[test]
    fn dead_gate_skips_iframes_but_keeps_knockback_writes() {
        let mut m = with_status(SA_DEAD2);
        m.samus().set_is_hit(0x20);
        m.set_samus_invincible_delay(0x20);
        crate::samus::physics::check_health_status(&mut m);
        assert_eq!(m.read_ram(SAMUS_INVINCIBLE_DELAY), 0x32, "死门跳过 Lx006 ⇒ 只置 $32 不递减");
        assert_eq!(m.read_ram(SAMUS_SPEED_Y_I), 0xFD, "击退初速在判死之前写（asm CE1E 前）");
        assert_eq!(m.read_ram(SAMUS_ACCEL_Y), 0x38, "击退重力同上");
        assert_eq!(m.samus().is_hit(), 0, "CheckHealthBeep 末尾清 isHit");
    }

    /// 活体对照：同样的命中面，i 帧要递减（证明上面的断言来自死门、不是别处短路）。
    #[test]
    fn alive_gate_still_decrements_iframes() {
        let mut m = with_status(1); // Stand
        m.samus().set_is_hit(0x20);
        crate::samus::physics::check_health_status(&mut m);
        assert_eq!(m.read_ram(SAMUS_INVINCIBLE_DELAY), 0x31, "活体走 Lx006 递减（$32-1）");
        assert_eq!(m.read_ram(SAMUS_SPEED_Y_I), 0xFD);
    }

    /// 路径②：HP 打到 0 ⇒ `sa_Dead` + `SetSamusExplode ; 07:CC8B` 五个字段 + SFX 旗。
    #[test]
    fn hp_zero_triggers_dead_and_explode_fields() {
        let mut m = Manager::default();
        m.samus().set_health(0x0002); // BCD 2
        m.set_health_change(0x0002);
        crate::weapon::projectile::subtract_health(&mut m);
        assert_eq!(m.samus().health(), 0, "死径把 Health/Health+1 都清 0（asm :3263-3264）");
        assert_eq!(m.samus().status(), Some(SamusStatus::Dead), "status←sa_Dead");
        assert_eq!(m.tri_init_flags() & 0x80, 0x80, "SFX_SamusDie（sfxTri_SamusDie=$80）");
        // SetSamusExplode（asm `SetSamusExplode ; 07:CC8B`）
        assert_eq!(m.samus().jump_displacement(), 0x50);
        assert_eq!(m.samus().anim_reset_index(), 0x32);
        assert_eq!(m.samus().anim_index(), 0x32);
        assert_eq!(m.samus().anim_delay(), 0);
        assert_eq!(m.object_counter(), 0, "ObjectCounter 归零（爆炸帧计数从 0 起）");
    }

    /// 路径③：`sa_Dead2` 的 40 帧计时器（`SamusDead2 ; 07:D41F` = `dec Samus.animDelay`）。
    #[test]
    fn dead2_timer_counts_down_to_zero() {
        let mut m = Manager::default();
        m.samus().set_status(SamusStatus::Dead2);
        m.samus().set_anim_delay(2);
        crate::samus::handler::samus_dead2(&mut m);
        assert_eq!(m.samus().anim_delay(), 1);
        crate::samus::handler::samus_dead2(&mut m);
        assert_eq!(m.samus().anim_delay(), 0, "归零后引擎门（frame/tick.rs）才起 SilenceMusic/SetTimer");
    }
}
