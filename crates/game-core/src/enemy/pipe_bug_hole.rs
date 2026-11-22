//! PipeBugHole 族（`LoadPipeBugHole ; 07:EC57` / `UpdateAllPipeBugHoles ; 07:FAF2` /
//! `UpdatePipeBugHole ; 07:FAFF`）。
//!
//! asm 锚（`refs/m1disasm/SRC/prg7_engine.asm:9184-9216` 装载、`:10205-10260` 更新）：
//!
//! ```text
//! LoadPipeBugHole:                          ; 07:EC57
//!   ldx #_sizeof_PipeBugHoles               ; 32（字节跨度）
//! @loop: txa / sec / sbc #_sizeof_PipeBugHoles.0   ; −8
//!   bmi @exit                               ; x−8 < 0 ⇒ 退出（槽已全占）
//!   tax / ldy PipeBugHoles.0.status,x / iny / bne @loop   ; status+1!=0 ⇒ 占用
//!   ldy #$00 / lda (Temp00_RoomPtr),y / and #$F0 / sta enemySlot,x
//!   iny / lda (Temp00_RoomPtr),y / sta status,x
//!   iny / lda (Temp00_RoomPtr),y / tay / and #$F0 / ora #$08 / sta y,x
//!   tya / jsr Amul16 / sta x,x
//!   jsr GetNameTableAtScrollDir / sta hi,x
//! @exit: lda #$03 / bne LoadStatues@GotoEnemyLoop        ; 恒消耗 3 字节
//! ```
//!
//! ```text
//! UpdateAllPipeBugHoles:                    ; 07:FAF2
//!   ldy #_sizeof_PipeBugHoles-_sizeof_PipeBugHoles.0   ; 24 ⇒ 槽 3
//! @loop: jsr UpdatePipeBugHole
//!   lda PageIndex / sec / sbc #8 / tay / bne @loop     ; 24→16→8 后停在 0
//! ```
//!
//! **槽 0 永不更新**：循环体在 `PageIndex` 变成 0 时退出（`bne`），而 0 对应槽 0
//! ⇒ 与炮台 `UpdateAllCannons` 同类的原版 quirk，按 D4 保真。
//!
//! ```text
//! UpdatePipeBugHole:                        ; 07:FAFF
//!   sty PageIndex / ldx status,y / inx / beq @RTS        ; $FF ⇒ 无洞
//!   ldx enemySlot,y / lda EnsExtra.status,x / beq @endIf_A
//!     lda Ens.data05,x / and #$02 / bne …@RTS            ; 占用且可见 ⇒ 跳过
//! @endIf_A: sta Ens.isHit,x                              ; A=0
//!   lda #$FF / cmp EnsExtra.type,x / bne @clearEnemySlot ; type!=FF ⇒ 先清（跨帧）
//!   dec Ens.delay,x / bne …@RTS                          ; 延迟未到
//!   lda status,y / jsr GetEnemyType                      ; 生敌
//!   …位置/radius($18/$0C)…                               ; 从洞抄
//!   jsr GetObjectYSlotPosition/GetEnemyXSlotPosition/GetRadiusSums…/CheckCollision…
//!   bcc …@RTS                                            ; 与 Samus 重叠 ⇒ 本帧放弃
//!   lda #enemyStatus_Resting / sta delay,x / sta status,x
//!   and ScrollDir / asl / sta data05,x
//!   ldy type,x / jsr InitEnemyFacingDirectionAxisAndDelay
//!   jmp InitEnemyForceSpeedTowardsSamusDelayAndHealth
//! @clearEnemySlot: sta EnsExtra.type,x / lda #$01 / sta delay,x / jmp RemoveEnemy
//! ```
//!
//! 说明：`sta Ens.isHit,x` 的 A 在两径都是 0（可见活敌径已提前 `rts`），因此恒写 0。

use crate::enemy::en_projectile::remove_enemy;
use crate::enemy::state::EnemyStatus;
use crate::frame::Manager;

/// `LoadPipeBugHole ; 07:EC57`：房间流 `type=7` 条目（`b0`/`b1`/`b2` 三字节）。
///
/// 从**末槽往前**找第一个 `status == $FF` 的空槽；全占则什么都不写。
/// 恒消耗 3 字节（调用方 `i += 3`），与原版 `lda #$03 / bne GotoEnemyLoop` 一致。
pub fn load_pipe_bug_hole(mem: &mut Manager, b0: u8, b1: u8, b2: u8) {
    for slot in (0..4u8).rev() {
        if mem.pbh_status(slot).wrapping_add(1) != 0 {
            continue; // status != $FF ⇒ 占用
        }
        mem.set_pbh_enemy_slot(slot, b0 & 0xF0);
        mem.set_pbh_status(slot, b1);
        mem.set_pbh_y(slot, (b2 & 0xF0) | 0x08);
        mem.set_pbh_x(slot, b2.wrapping_shl(4));
        // `GetNameTableAtScrollDir`（`$EB85`）：`(PPUCTRL_ZP ^ ScrollDir) & $01`。
        let hi = mem.ppuctrl_zp() ^ mem.scroll_dir();
        mem.set_pbh_hi(slot, hi & 0x01);
        return;
    }
}

/// `UpdateAllPipeBugHoles ; 07:FAF2`：槽 3 → 1（**槽 0 永不更新**，见模块注释）。
pub fn update_all_pipe_bug_holes(mem: &mut Manager) {
    for slot in (1..4u8).rev() {
        update_pipe_bug_hole(mem, slot);
    }
}

/// `UpdatePipeBugHole ; 07:FAFF` 单槽。
fn update_pipe_bug_hole(mem: &mut Manager, slot: u8) {
    // `ldx status,y / inx / beq @RTS`：`$FF` ⇒ 无洞。
    if mem.pbh_status(slot).wrapping_add(1) == 0 {
        return;
    }
    let x = mem.pbh_enemy_slot(slot);
    debug_assert!(x % 16 == 0, "PipeBugHole.enemySlot 必须是 16 的倍数：{x}");
    let eslot = x / 16;
    let status = mem.enemy(eslot).status_raw();
    if status != 0 && mem.enemy(eslot).data05() & 0x02 != 0 {
        return; // 槽被可见活敌占用
    }
    mem.enemy(eslot).set_is_hit(0);
    // `lda #$FF / cmp EnsExtra.type,x / bne @clearEnemySlot`：type != $FF ⇒ 先清（下一帧才生）。
    if mem.enemy(eslot).enemy_type() != 0xFF {
        mem.enemy(eslot).set_enemy_type(0xFF);
        mem.enemy(eslot).set_delay(0x01);
        remove_enemy(mem, eslot);
        return;
    }
    // `dec Ens.delay,x / bne …@RTS`
    let d = mem.enemy(eslot).delay().wrapping_sub(1);
    mem.enemy(eslot).set_delay(d);
    if d != 0 {
        return;
    }

    // ---- 生敌 ----
    // `lda PipeBugHoles.status,y / jsr GetEnemyType`：specialAttribs←&$C0、type←&$3F。
    let hs = mem.pbh_status(slot);
    mem.enemy(eslot).set_special_attribs(hs & 0xC0);
    mem.enemy(eslot).set_enemy_type(hs & 0x3F);
    // 位置与名表位从洞抄。
    let (hy, hx, hhi) = (mem.pbh_y(slot), mem.pbh_x(slot), mem.pbh_hi(slot));
    mem.enemy(eslot).set_y(hy);
    mem.enemy(eslot).set_pos_x(hx);
    mem.enemy(eslot).set_hi(hhi);
    // `lda #$18 / sta radiusX` + `lda #$0C / sta radiusY`。
    mem.enemy(eslot).set_radius_x(0x18);
    mem.enemy(eslot).set_radius_y(0x0C);

    // `ldy #$00 / GetObjectYSlotPosition`＝Samus（`Objects` 槽 0）作 Y 槽。
    let ppuctrl = mem.ppuctrl_zp();
    let sy = mem.object(0).y();
    let sx = mem.object(0).x();
    let shi = (mem.object(0).hi() ^ ppuctrl) & 0x01;
    // `GetEnemyXSlotPosition`＝敌槽作 X 槽（`EnsExtra.hi` 同样 `eor PPUCTRL`）。
    let ey = mem.enemy(eslot).y();
    let ex = mem.enemy(eslot).pos_x();
    let ehi = (mem.enemy(eslot).hi() ^ ppuctrl) & 0x01;
    // `GetRadiusSumsOfEnXSlotAndObjYSlot`：敌半径 + Samus 半径。
    let ry = mem
        .enemy(eslot)
        .radius_y()
        .wrapping_add(mem.object(0).radius_y());
    let rx = mem
        .enemy(eslot)
        .radius_x()
        .wrapping_add(mem.object(0).radius_x());
    // `CheckCollisionOfXSlotAndYSlot` + `bcc …@RTS`：**重叠（太近）⇒ 本帧放弃**。
    let coll = crate::samus::physics::check_collision_of_slots(
        (ey, ex, ehi),
        (sy, sx, shi),
        ry,
        rx,
        mem.scroll_dir(),
    );
    if !coll.no_overlap {
        return;
    }

    // `lda #enemyStatus_Resting / sta delay,x / sta status,x`。
    mem.enemy(eslot).set_delay(0x01);
    mem.enemy(eslot).set_status(EnemyStatus::Resting);
    // `and ScrollDir / asl / sta data05,x` + `ror`（`InitEnemyFacingDirectionAxisAndDelay`）：
    // 净效果 `data05 = (L977B[type].bit7 << 7) | (ScrollDir & $01)`。
    let area = crate::frame::cur_area(mem);
    let t = mem.enemy(eslot).enemy_type();
    let raw977b = game_data::assets::enemy_facing(area)[t as usize & 0x3F];
    let d5 = (raw977b & 0x80) | (mem.scroll_dir() & 0x01);
    mem.enemy(eslot).set_data05(d5);
    // `ldy type,x / jsr InitEnemyFacingDirectionAxisAndDelay`：`delay ← EnemyInitDelayTbl[type]`。
    mem.enemy(eslot)
        .set_delay(game_data::assets::enemy_init_delay(area)[t as usize & 0x3F]);
    // `jmp InitEnemyForceSpeedTowardsSamusDelayAndHealth`：data0D + health（tough 双血）。
    mem.enemy(eslot)
        .set_data0d(game_data::assets::enemy_force_speed_delay(area)[t as usize & 0x3F]);
    let mut hp = game_data::assets::enemy_health(area)[t as usize & 0x3F];
    if mem.enemy(eslot).special_attribs() & 0x80 != 0 {
        hp = hp.wrapping_shl(1);
    }
    mem.enemy(eslot).set_health(hp);
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests_pipe_bug_hole {
    use super::*;

    fn boot_area0(m: &mut Manager) {
        game_data::pack::init_from_env().ok();
        m.set_in_area(0x10); // Brinstar
        m.set_ppuctrl_zp(0x00);
        m.set_scroll_dir(0x02); // 横滚
    }

    fn stage_hole(m: &mut Manager, slot: u8, status: u8, eslot_off: u8, y: u8, x: u8, hi: u8) {
        m.set_pbh_status(slot, status);
        m.set_pbh_enemy_slot(slot, eslot_off);
        m.set_pbh_y(slot, y);
        m.set_pbh_x(slot, x);
        m.set_pbh_hi(slot, hi);
    }

    /// 装载：全空 ⇒ 落在**末槽**（槽 3），五字段按 asm 形写。
    #[test]
    fn load_fills_last_slot_first() {
        let mut m = Manager::default();
        boot_area0(&mut m);
        for s in 0..4 {
            m.set_pbh_status(s, 0xFF);
        }
        load_pipe_bug_hole(&mut m, 0x10, 0x87, 0xA5);
        assert_eq!(m.pbh_status(3), 0x87, "末槽");
        assert_eq!(m.pbh_enemy_slot(3), 0x10);
        assert_eq!(m.pbh_y(3), (0xA5 & 0xF0) | 0x08);
        assert_eq!(m.pbh_x(3), 0xA5u8.wrapping_shl(4));
        assert_eq!(m.pbh_hi(3), (0x00 ^ 0x02) & 0x01);
        assert_eq!(m.pbh_status(2), 0xFF, "其余槽不动");
    }

    /// 装载：末槽占用 ⇒ 退到下一槽（从末往前找）。
    #[test]
    fn load_skips_occupied_slots() {
        let mut m = Manager::default();
        boot_area0(&mut m);
        for s in 0..4 {
            m.set_pbh_status(s, 0xFF);
        }
        m.set_pbh_status(3, 0x07);
        m.set_pbh_status(2, 0x07);
        load_pipe_bug_hole(&mut m, 0x20, 0x07, 0x30);
        assert_eq!(m.pbh_status(1), 0x07);
        assert_eq!(m.pbh_enemy_slot(1), 0x20);
        assert_eq!(m.pbh_status(0), 0xFF);
    }

    /// 装载：四槽全占 ⇒ **什么都不写**（asm `@exit`）。
    #[test]
    fn load_with_no_free_slot_writes_nothing() {
        let mut m = Manager::default();
        boot_area0(&mut m);
        for s in 0..4 {
            m.set_pbh_status(s, 0x07);
        }
        load_pipe_bug_hole(&mut m, 0x30, 0x07, 0x99);
        for s in 0..4 {
            assert_eq!(m.pbh_status(s), 0x07);
            assert_eq!(m.pbh_enemy_slot(s), 0x00);
        }
    }

    /// 更新：`status == $FF` ⇒ 无洞，什么都不做。
    #[test]
    fn update_ignores_hole_with_ff_status() {
        let mut m = Manager::default();
        boot_area0(&mut m);
        stage_hole(&mut m, 3, 0xFF, 0x10, 0x00, 0x00, 0x00);
        m.enemy(1).set_delay(0x05);
        update_pipe_bug_hole(&mut m, 3);
        assert_eq!(m.enemy(1).delay(), 0x05);
        assert_eq!(m.enemy(1).status_raw(), 0);
    }

    /// 更新：目标槽被**可见活敌**占用 ⇒ 整件返回（连 isHit 都不碰）。
    #[test]
    fn update_skips_slot_occupied_by_visible_enemy() {
        let mut m = Manager::default();
        boot_area0(&mut m);
        stage_hole(&mut m, 3, 0x07, 0x10, 0x18, 0x40, 0x00);
        m.enemy(1).set_status(EnemyStatus::Active);
        m.enemy(1).set_data05(0x02);
        m.enemy(1).set_delay(0x05);
        update_pipe_bug_hole(&mut m, 3);
        assert_eq!(m.enemy(1).delay(), 0x05, "未递减");
        assert_eq!(m.enemy(1).enemy_type(), 0x00, "未改写 type");
    }

    /// 更新：`type != $FF` ⇒ **先清槽**（type←$FF、delay←1、RemoveEnemy），本帧不生。
    #[test]
    fn update_clears_slot_on_frame_before_spawn() {
        let mut m = Manager::default();
        boot_area0(&mut m);
        stage_hole(&mut m, 3, 0x07, 0x10, 0x18, 0x40, 0x00);
        m.enemy(1).set_enemy_type(0x00);
        m.enemy(1).set_status(EnemyStatus::Active);
        update_pipe_bug_hole(&mut m, 3);
        assert_eq!(m.enemy(1).enemy_type(), 0xFF, "清成哨兵");
        assert_eq!(m.enemy(1).delay(), 0x01);
        assert_eq!(m.enemy(1).status_raw(), 0, "RemoveEnemy 释放槽");
    }

    /// 更新：`type == $FF` 且 delay != 0 ⇒ 只递减。
    #[test]
    fn update_decrements_delay_only() {
        let mut m = Manager::default();
        boot_area0(&mut m);
        stage_hole(&mut m, 3, 0x07, 0x10, 0x18, 0x40, 0x00);
        m.enemy(1).set_enemy_type(0xFF);
        m.enemy(1).set_delay(0x03);
        update_pipe_bug_hole(&mut m, 3);
        assert_eq!(m.enemy(1).delay(), 0x02);
        assert_eq!(m.enemy(1).status_raw(), 0);
    }

    /// 更新：`type == $FF` 且 delay 归零、Samus 远 ⇒ 生敌（R9 `type=7`）。
    #[test]
    fn update_spawns_pipe_bug_when_samus_is_far() {
        let mut m = Manager::default();
        boot_area0(&mut m);
        // 洞：槽 1（`$0400+$10`）、PipeBug（type 7）、位置 (y 0x18, x 0x40)。
        stage_hole(&mut m, 3, 0x07, 0x10, 0x18, 0x40, 0x00);
        m.enemy(1).set_enemy_type(0xFF);
        // asm 的跨帧节奏：清槽帧写 delay=1，下一帧 `dec` 到 0 才生敌。
        m.enemy(1).set_delay(0x01);
        // Samus（Objects 槽 0）在远处：dy=0x68 远超半径和 ⇒ 不重叠。
        m.object(0).set_y(0x80);
        m.object(0).set_x(0x40);
        m.object(0).set_radius_y(0x08);
        m.object(0).set_radius_x(0x08);
        update_pipe_bug_hole(&mut m, 3);
        assert_eq!(m.enemy(1).status_raw(), ENEMY_STATUS_RESTING);
        assert_eq!(m.enemy(1).enemy_type(), 0x07);
        assert_eq!(m.enemy(1).y(), 0x18);
        assert_eq!(m.enemy(1).pos_x(), 0x40);
        assert_eq!(m.enemy(1).radius_x(), 0x18);
        assert_eq!(m.enemy(1).radius_y(), 0x0C);
        // `data05 = (L977B[7].bit7 << 7) | (ScrollDir & 1)`。
        let raw = game_data::assets::enemy_facing(0)[7];
        assert_eq!(m.enemy(1).data05(), (raw & 0x80) | (0x02 & 0x01));
        // `delay ← EnemyInitDelayTbl[type]`。
        assert_eq!(
            m.enemy(1).delay(),
            game_data::assets::enemy_init_delay(0)[7]
        );
    }

    /// 更新：Samus 与洞重叠 ⇒ **本帧放弃生敌**（asm `bcc …@RTS`）。
    #[test]
    fn update_aborts_spawn_when_samus_overlaps_hole() {
        let mut m = Manager::default();
        boot_area0(&mut m);
        stage_hole(&mut m, 3, 0x07, 0x10, 0x18, 0x40, 0x00);
        m.enemy(1).set_enemy_type(0xFF);
        m.enemy(1).set_delay(0x01);
        m.object(0).set_y(0x18);
        m.object(0).set_x(0x40);
        m.object(0).set_radius_y(0x10);
        m.object(0).set_radius_x(0x10);
        update_pipe_bug_hole(&mut m, 3);
        assert_eq!(m.enemy(1).status_raw(), 0, "未生");
        assert_eq!(
            m.enemy(1).radius_x(),
            0x18,
            "半径已写（asm 位置在判碰撞之前）"
        );
    }

    /// 件序 quirk：`UpdateAllPipeBugHoles` 只跑槽 3/2/1，**槽 0 永不更新**。
    #[test]
    fn update_all_never_touches_slot_zero() {
        let mut m = Manager::default();
        boot_area0(&mut m);
        // 槽 0 与槽 1 都是「下一帧可生」的状态。
        for s in [0u8, 1u8] {
            stage_hole(&mut m, s, 0x07, 0x10, 0x18, 0x40, 0x00);
        }
        m.enemy(1).set_enemy_type(0xFF);
        m.enemy(1).set_delay(0x01);
        m.object(0).set_y(0x80);
        m.object(0).set_radius_y(0x08);
        m.object(0).set_radius_x(0x08);
        update_all_pipe_bug_holes(&mut m);
        assert_eq!(m.enemy(1).status_raw(), ENEMY_STATUS_RESTING, "槽 1 被处理");
        assert_eq!(m.enemy(1).enemy_type(), 0x07, "槽 1 生敌成功");
    }

    /// 冷启动 quirk（`prg7_engine.asm:1988-1991`，asm 自注 `BUG!`）：四个 `stx`
    /// 第二个写到 `PipeBugHoles.1.y` ⇒ **槽 3 的 status 从未被初始化**。
    /// 该 quirk 在 `frame/boot.rs` 的冷启动径里落，这里只钉「槽 3 = $00 时视为占用」的后果。
    #[test]
    fn uninitialised_slot3_status_is_occupied() {
        let mut m = Manager::default();
        boot_area0(&mut m);
        // 冷启动后槽 3 status=0（RAM 初值），其余槽 $FF。
        for s in 0..4 {
            m.set_pbh_status(s, 0xFF);
        }
        m.set_pbh_status(3, 0x00);
        load_pipe_bug_hole(&mut m, 0x10, 0x07, 0x40);
        assert_eq!(m.pbh_status(2), 0x07, "跳过 status=0 的槽 3");
        assert_eq!(m.pbh_status(3), 0x00, "槽 3 保持未初始化值");
    }

    const ENEMY_STATUS_RESTING: u8 = 0x01;
}
