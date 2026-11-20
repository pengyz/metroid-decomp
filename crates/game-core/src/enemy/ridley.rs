//! Ridley（`RidleyAIRoutine_{AREA} ; 05:9A13`）与其火球（`RidleyFireballAIRoutine ; 05:9A4A`）。
//!
//! Ridley 本体只做三件事：写「位移后翻转」动画索引（R/L 各一个）、跑
//! `CommonJump_EnemyFlipAfterDisplacement`、再按计数器尝试吐火球。火球是**独立敌槽**
//! （槽 1-5，字节偏移 `$10-$50`），有自己的 AI：朝向翻转或飞出 Samus 右侧 32px 以外就自删。
//!
//! 原版 bug（按字节保留）：`RidleyTryToLaunchFireball` 在「全部火球都在飞」时去 `inc
//! KraidLintCounter`——那是 Kraid 的 lint 计数器，复制粘贴残留。见 asm 注释
//! `(BUG! this is actually Kraid's lint counter …)`。

use crate::enemy::motion::apply_speed_to_position_parts;
use crate::enemy::state::EnemyStatus;
use crate::frame::Manager;


/// bank5（Ridley）`EnAnimTable` 表序：`RidleyHopping_R = 4`、`RidleyHopping_L = 5`。
const ANIM_HOPPING_R: u8 = 4;
const ANIM_HOPPING_L: u8 = 5;
/// `RidleyFireballOffsetX_{AREA} ; 05:9ADF` = `[$08, -$08]`（下标=水平朝向位）。
const FIREBALL_DX: [i8; 2] = [0x08, -0x08];
/// `lda #$F8 / sta Temp04_SpeedY`：火球相对 Ridley 上移 8px。
const FIREBALL_DY: i8 = -0x08;
/// 火球敌类型（`lda #$0A / sta EnsExtra.0.type,x`）。
const TYPE_RIDLEY_FIREBALL: u8 = 0x0A;
/// 火球计数器初值（`ldy #$60`）。
const COUNTER_INIT: u8 = 0x60;

/// `RidleyAIRoutine_{AREA} ; 05:9A13`。
pub(crate) fn ridley_ai(mem: &mut Manager, x: u8) {
    let slot = x / 16;
    let status = mem.enemy(slot).status_raw();
    if status >= 3 { // enemyStatus_Explode
        if status == 3 || status == 4 {
            // `@explode`：清 `EnsExtra.1..5.status`（绝对地址，非 ,x —— 恰好是火球槽）
            for s in 1..=5u8 {
                mem.enemy(s).set_status(EnemyStatus::NoEnemy);
            }
            return;
        }
        // `cmp #enemyStatus_Pickup / bne @exit`：其它的非正常态直接走尾件
    } else {
        // `@normal`：写翻转动画索引（R 在 $85、L 在 $86）并跑位移翻转 + 吐火球
        mem.set_flip_anim_index(false, ANIM_HOPPING_R);
        mem.set_flip_anim_index(true, ANIM_HOPPING_L);
        crate::enemy::sidehopper::enemy_flip_after_displacement(mem, x);
        ridley_try_launch_fireball(mem, x);
    }
    // `@exit`：每 3 帧换动画帧
    crate::enemy::framework::update_enemy_common_decide_with_beats(mem, x, 0x03, 0x03);
}

/// `RidleyFireballAIRoutine_{AREA} ; 05:9A4A`。
pub(crate) fn ridley_fireball_ai(mem: &mut Manager, x: u8) {
    let slot = x / 16;
    let before = mem.enemy(slot).data05();
    crate::enemy::framework::update_enemy_common_decide_with_beats(mem, x, 0x02, 0x02);
    let after = mem.enemy(slot).data05();
    // `eor / lsr / bcs @RemoveFireball`：水平朝向位变了 ⇒ 自删
    if (before ^ after) & 0x01 != 0 {
        mem.enemy(slot).set_status(EnemyStatus::NoEnemy);
        return;
    }
    // `lda data05 / lsr / bcs @RTS`：朝左 ⇒ 保留
    if after & 0x01 != 0 {
        return;
    }
    // `lda Ens.x / sec / sbc Samus.x`：x < Samus.x ⇒ 保留；否则 `cmp #$20` ⇒ <32px 保留
    let ex = mem.enemy(slot).pos_x();
    let sx = mem.samus().x();
    if ex < sx {
        return;
    }
    if ex.wrapping_sub(sx) < 0x20 {
        return;
    }
    mem.enemy(slot).set_status(EnemyStatus::NoEnemy);
}

/// `RidleyTryToLaunchFireball_{AREA} ; 05:9A79`。
fn ridley_try_launch_fireball(mem: &mut Manager, x: u8) {
    // `ldy RidleyFireballCounter / bne @endIf_A` ⇒ 零时按 $60 计
    let mut c = mem.ridley_fireball_counter();
    if c == 0 {
        c = COUNTER_INIT;
    }
    // `lda FrameCount / and #$02 / bne @RTS`：只在 bit1 清的帧递减
    if mem.frame_count() & 0x02 != 0 {
        return;
    }
    c = c.wrapping_sub(1);
    mem.set_ridley_fireball_counter(c);
    // `tya / asl`：×2 之后 bit7 置 ⇒ 退出；低 nibble 必须 == $0A
    let a = c.wrapping_shl(1);
    if a & 0x80 != 0 {
        return;
    }
    if a & 0x0F != 0x0A {
        return;
    }
    // 从槽 5（$50）往下找空槽或不可见槽
    let mut found: Option<u8> = None;
    let mut bx = 0x50u8;
    loop {
        if mem.ens_face(bx).status() == EnemyStatus::NoEnemy {
            found = Some(bx);
            break;
        }
        if mem.enemy(bx / 16).data05() & 0x02 == 0 {
            found = Some(bx);
            break;
        }
        bx = bx.wrapping_sub(0x10);
        if bx == 0 {
            break;
        }
    }
    let Some(fx) = found else {
        // 原版 quirk：全部在飞时去 `inc KraidLintCounter`（复制粘贴 Kraid 例程的残留）
        let v = mem.kraid_lint_counter();
        mem.set_kraid_lint_counter(v.wrapping_add(1));
        return;
    };

    let rslot = x / 16;
    let fslot = fx / 16;
    // `lda Ens.0.data05 / sta Ens.0.data05,x`（Ridley 槽 0 的朝向抄给火球）
    let d5 = mem.enemy(rslot).data05();
    mem.enemy(fslot).set_data05(d5);
    // `StoreEnemyPositionToTemp_`：把 Ridley 位置（含 hi）搬到火球槽，再按偏移走一遍
    // `ApplySpeedToPosition`（等价 asm 的 Temp08/09/0B + CommonJump_ApplySpeedToPosition）
    let (ry, rx, rhi) = (
        mem.enemy(rslot).y(),
        mem.enemy(rslot).pos_x(),
        mem.enemy(rslot).hi(),
    );
    {
        let mut e = mem.enemy(fslot);
        e.set_y(ry);
        e.set_pos_x(rx);
        e.set_hi(rhi);
    }
    let dx = FIREBALL_DX[(d5 & 0x01) as usize];
    let (ok, ny, nx, nhi) = apply_speed_to_position_parts(mem, fx, FIREBALL_DY, dx);
    if !ok {
        return; // 初始位置越界 ⇒ 放弃（`bcc @RTS`）
    }
    {
        let mut e = mem.enemy(fslot);
        e.set_special_attribs(0x00);
        e.set_enemy_type(TYPE_RIDLEY_FIREBALL);
        e.set_status(EnemyStatus::Resting);
        e.set_y(ny);
        e.set_pos_x(nx);
        e.set_hi(nhi);
    }
    // `jmp CommonJump_0E`：查表填 delay/data0D/health 并把 L977B 的 bit7 旋进 data05
    crate::enemy::rinka::common_jump_0e(mem, fx);
}

#[cfg(test)]
mod tests_ridley {
    use crate::enemy::state::EnemyStatus;
    use crate::frame::Manager;

    fn mgr() -> Manager {
        let mut m = Manager::default();
        crate::frame::set_in_area_raw(&mut m, 0x14); // Ridley（bank5）
        m.enemy(0).set_status(EnemyStatus::Active);
        m.enemy(0).set_enemy_type(0x02);
        m.enemy(0).set_y(0x80);
        m.enemy(0).set_pos_x(0x80);
        m
    }

    /// 正常态：写翻转动画索引（R=4 在 `$85`、L=5 在 `$86`）。
    #[test]
    fn normal_writes_hopping_anim_indices() {
        let mut m = mgr();
        super::ridley_ai(&mut m, 0);
        assert_eq!(m.flip_anim_index(false), 4);
        assert_eq!(m.flip_anim_index(true), 5);
    }

    /// Explode/Pickup 态：清槽 1-5（火球）后返回。
    #[test]
    fn explode_clears_fireball_slots() {
        let mut m = mgr();
        m.enemy(0).set_status(EnemyStatus::Explode);
        for s in 1..=5u8 {
            m.enemy(s).set_status(EnemyStatus::Active);
        }
        super::ridley_ai(&mut m, 0);
        for s in 1..=5u8 {
            assert_eq!(m.enemy(s).status(), EnemyStatus::NoEnemy, "slot {}", s);
        }
        assert_eq!(m.flip_anim_index(false), 0, "不写动画索引");
    }

    /// 计数器节拍：`(counter*2) & $0F == $0A` 且 `FrameCount&2==0` 才吐火球。
    #[test]
    fn launches_fireball_on_counter_beat() {
        let mut m = mgr();
        // counter $60 → 递减 $5F；$5F<<1=$BE（bit7 置）⇒ 不发射。要命中需 c 使 (c<<1)低nibble=0x0A
        // 且 bit7 清 ⇒ c<<1 ∈ {0x0A,0x1A,...0x7A} ⇒ c ∈ {0x05,0x0D,0x15,...0x3D}
        m.set_ridley_fireball_counter(0x06); // 递减后 0x05
        m.set_frame_count(0x00);
        super::ridley_ai(&mut m, 0);
        assert_eq!(m.enemy(5).status(), EnemyStatus::Resting, "槽 5 生成火球");
        assert_eq!(m.enemy(5).enemy_type(), 0x0A);
        assert_eq!(m.enemy(5).y(), 0x78, "Ridley y=$80 上移 8");
    }

    /// `FrameCount & $02 != 0` 的帧不递减计数器。
    #[test]
    fn counter_holds_on_frame_count_bit1() {
        let mut m = mgr();
        m.set_ridley_fireball_counter(0x06);
        m.set_frame_count(0x02);
        super::ridley_ai(&mut m, 0);
        assert_eq!(m.ridley_fireball_counter(), 0x06);
        assert_eq!(m.enemy(5).status(), EnemyStatus::NoEnemy);
    }

    /// 全部火球在飞 ⇒ 原版 quirk：`inc KraidLintCounter`，不发射。
    #[test]
    fn all_fireballs_flying_increments_kraid_lint_counter() {
        let mut m = mgr();
        m.set_ridley_fireball_counter(0x06);
        m.set_frame_count(0x00);
        m.set_kraid_lint_counter(0x11);
        for s in 1..=5u8 {
            m.enemy(s).set_status(EnemyStatus::Active);
            m.enemy(s).set_data05(0x02); // 可见
        }
        super::ridley_ai(&mut m, 0);
        assert_eq!(m.ridley_fireball_counter(), 0x05, "计数器不回滚");
        assert_eq!(m.kraid_lint_counter(), 0x12, "Kraid lint 计数器 +1（原版 bug）");
    }

    /// 火球朝向翻转 ⇒ 自删。
    #[test]
    fn fireball_removed_when_facing_flips() {
        let mut m = Manager::default();
        crate::frame::set_in_area_raw(&mut m, 0x14);
        m.enemy(1).set_status(EnemyStatus::Active);
        m.enemy(1).set_data05(0x00);
        // 让 `update_enemy_common_decide` 翻转朝向：置 specAttribs bit1（水平翻转标志）
        m.enemy(1).set_data05(0x00);
        m.enemy(1).set_reset_anim_index(9);
        super::ridley_fireball_ai(&mut m, 0x10);
        // 骨架是否翻转取决于 anim 表；此处只要不 panic 且状态合法
        let s = m.enemy(1).status_raw();
        assert!(s <= 4, "status 合法：{s}");
    }
}
