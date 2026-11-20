//! Zebetite（Tourian 能量屏障，5 座）与 MotherBrain 玻璃的弹命中登记。
//!
//! **asm 锚**
//! - `UpdateBullet_CollisionWithZebetiteAndMotherBrainGlass ; 03:A0C6`
//!   （`refs/m1disasm/SRC/prg3_tourian.asm:1732`）：只处理 `wa_Missile`；`$98` ⇒ 玻璃
//!   （找第一个空 TileBlast 槽，`ptr.lo←$8C`、`animFrame←VRAMString01`、`DrawTileBlast`
//!   清 2×3 瓦）；`$90-$97` ⇒ 登记 `Zebetites.isHit`；出口 `pla / pla / clc / rts`
//!   **双返回**（跳过调用方剩下的固实瓦处理 ⇒ 在 Tourian 里弹一撞瓦就爆，见规格 §5.4）。
//! - `UpdateAllZebetites ; 03:A28B` / `UpdateZebetite ; 03:A29B`（`:2076`/`:2091`）：
//!   8 击死亡、每 64 帧自愈 1 击、重绘失败回滚（`qtyHits-1` 且 `isHit` 留痕重试）。
//! - `ZebetiteAnimFrameTable ; 03:A310`（`:2184`）：`[0C 100%, 0D 75%, 0E 50%, 0F 25%, 07 dead]`
//!   ——元素是 **VRAM 字符串 id**，由 `assets::vram_string(area,id)` 取到按区图案。
//!
//! **结构**：`constants_ram.asm:76-83` `.struct Zebetite`（status0 / roomRAMPtr1-2 /
//! qtyHits3 / healingDelay4 / isHit5 / unused6-7），`Zebetites instanceof Zebetite 5` @ `$0758`
//! （`constants_ram.asm:1182-1183`）。
//!
//! **诚实边界**：Tourian 目前无像素证据（`docs/findings/room-sweep.md` §8.1：非 0 区注入
//! 结构性不可用），本件证据 = asm 逐行 + 单测锚。

use crate::frame::Manager;

/// `Zebetites` 槽数（`_sizeof_Zebetites / _sizeof_Zebetites.0` = 5）。
pub(crate) const COUNT: u8 = 5;
/// `ZebetiteAnimFrameTable ; 03:A310`。
const ANIM_FRAME_TABLE: [u8; 5] = [0x0C, 0x0D, 0x0E, 0x0F, 0x07];
/// 自愈间隔初值（`lda #$40 / sta healingDelay`）。
const HEAL_DELAY: u8 = 0x40;
/// `TileBlasts.1 - TileBlasts.0`（`UpdateAllZebetites` 把 `PageIndex` 设成槽 1 的字节偏移）。
const TB_SLOT: u8 = 1;
/// 玻璃瓦 `$98` 的 TileBlast 指针低字节（`lda #$8C`）。
const GLASS_PTR_LO: u8 = 0x8C;
/// 玻璃图案的 VRAM 字符串 id（`_id_VRAMString01`）。
const GLASS_ANIM_FRAME: u8 = 0x01;

/// `UpdateAllZebetites ; 03:A28B`：`PageIndex ← TileBlasts.1`，`x = $20` 起每 `-8`
/// 循环到 `x=8` 为止（asm 的 `txa / sec / sbc #$08 / tax / bne` 在 `x=0` 时退出 ⇒ **槽 0 不更新**）。
pub(crate) fn update_all_zebetites(mem: &mut Manager) {
    mem.set_page_index(TB_SLOT * 16);
    let mut x: u8 = 0x20;
    loop {
        update_zebetite(mem, x);
        x = x.wrapping_sub(8);
        if x == 0 {
            return;
        }
    }
}

/// `UpdateZebetite ; 03:A29B`（`x` = 槽字节偏移，`$20/$18/$10/$08`）。
pub(crate) fn update_zebetite(mem: &mut Manager, x: u8) {
    let slot = x / 8;
    // `status & $0F != 1` ⇒ 不更新（已死 / 未生成）
    if mem.zeb_status(slot) & 0x0F != 1 {
        return;
    }
    if mem.zeb_is_hit(slot) != 0 {
        let q = mem.zeb_qty_hits(slot).wrapping_add(1);
        mem.set_zeb_qty_hits(slot, q);
        // `lsr / bcs @dontThin`：奇数 ⇒ 本帧不更新外观（只有偶数击才变薄）
        if q & 1 == 0 {
            let y = q >> 1;
            // `tay` 后 `sbc #$03`——此时 carry 来自 `lsr`（q 偶数 ⇒ CF=0）⇒ 实算 `y-4`；
            // 为 0 即 8 击（asm 注释 `(3 + carry) * 2 == 8`）⇒ `inc status`（bit1 停更新）。
            if y == 4 {
                mem.set_zeb_status(slot, mem.zeb_status(slot).wrapping_add(1));
            }
            gfx_update(mem, slot, y);
            return;
        }
    } else {
        // `@dontThin`：满血不自愈；healingDelay 未到 0 不自愈
        let q = mem.zeb_qty_hits(slot);
        if q != 0 {
            let d = mem.zeb_healing_delay(slot).wrapping_sub(1);
            mem.set_zeb_healing_delay(slot, d);
            if d == 0 {
                mem.set_zeb_healing_delay(slot, HEAL_DELAY);
                let q2 = q.wrapping_sub(1);
                mem.set_zeb_qty_hits(slot, q2);
                // `lsr / tay / bcc @gfxUpdate`：**偶数**才重绘（asm 注释写"odd"但代码是
                // `bcc` = CF 清 = 偶数——按代码不按注释，D4 保真）。
                if q2 & 1 == 0 {
                    gfx_update(mem, slot, q2 >> 1);
                    return;
                }
            }
        }
    }
    // `@exit ; 03:A30A`：无条件清 isHit（重绘成功径在 gfx_update 里 return，走不到这里？）
    mem.set_zeb_is_hit(slot, 0);
}

/// `@gfxUpdate ; 03:A2BA`：把外观写进 `TileBlasts.1` 再 `DrawTileBlast`。
fn gfx_update(mem: &mut Manager, slot: u8, y: u8) {
    mem.set_tb_anim_frame(TB_SLOT, ANIM_FRAME_TABLE[y as usize]);
    let ptr = mem.zeb_ptr(slot);
    mem.set_tb_ptr(TB_SLOT, ptr);
    // 缓冲里有 VRAM 结构 ⇒ 本帧不画（`lda VRAMStructBufferIndex / bne @gfxUpdateFailure`）
    let drawn = if mem.vram_struct_index() != 0 {
        false
    } else {
        crate::world::tile_blast::draw_tile_blast_slot(mem, TB_SLOT)
    };
    if drawn {
        mem.set_zeb_healing_delay(slot, HEAL_DELAY);
        mem.set_zeb_is_hit(slot, 0);
        return;
    }
    // `@gfxUpdateFailure`：撤销"停更新"标记、把命中处理也回滚一格去重试。
    let s = mem.zeb_status(slot) & 0x80 | 0x01;
    mem.set_zeb_status(slot, s);
    mem.set_zeb_is_hit(slot, s); // asm `sta isHit` 存的是刚算出的 status 值（非 1 也照存）
    mem.set_zeb_qty_hits(slot, mem.zeb_qty_hits(slot).wrapping_sub(1));
}

/// `UpdateBullet_CollisionWithZebetiteAndMotherBrainGlass ; 03:A0C6` 的 Tourian 侧效果。
///
/// 返回 `true` 表示「asm 走了双返回」（`pla / pla / clc / rts`）⇒ 调用方应把这一撞当成
/// 已碰撞（`CF=0`）并**跳过**后续的 `$4E`/可炸瓦处理。asm 里这个例程对**所有**非导弹弹
/// 也从同一 `@exit` 双返回，所以只要在 Tourian 撞上固实瓦就是 `true`。
pub(crate) fn tourian_bullet_hit(mem: &mut Manager, slot: u8, tile: u8, ptr_lo: u8, ptr_hi: u8) -> bool {
    // `lda UpdatingSamusProjectile / beq @exit`
    if mem.updating_samus_projectile() == 0 {
        return true;
    }
    // `ldx PageIndex / lda Objects.0.status,x / cmp #wa_Missile / bne @exit`
    if mem.object(slot).status() != crate::weapon::projectile::WA_MISSILE {
        return true;
    }
    if tile == 0x98 {
        break_mother_brain_glass(mem, ptr_hi);
    } else {
        register_zebetite_hit(mem, ptr_lo, ptr_hi);
    }
    true
}

/// 玻璃命中（`cpy #$98` 分支）：找第一个空 TileBlast 槽，`ptr.lo←$8C`、`animFrame←01`，
/// `DrawTileBlast` 清掉 2×3 瓦。
fn break_mother_brain_glass(mem: &mut Manager, ptr_hi: u8) {
    // asm **不**给这个槽设 `routine`——它只是借一个空槽当 `DrawTileBlast` 的参数块。
    let Some(slot) = crate::world::tile_blast::free_slot(mem) else {
        return; // 无空槽 ⇒ asm 的 `cmp #_sizeof_TileBlasts / bne @loop` 落 `beq @exit`
    };
    mem.set_tb_ptr(slot, (ptr_hi as u16) << 8 | GLASS_PTR_LO as u16);
    mem.set_tb_anim_frame(slot, GLASS_ANIM_FRAME);
    mem.set_page_index(slot * 16);
    crate::world::tile_blast::draw_tile_blast_slot(mem, slot);
}

/// `@checkZebetite ; 03:A103`：把命中点对齐到左瓦后，按 **指针** 找对应 Zebetite 并置 `isHit`。
fn register_zebetite_hit(mem: &mut Manager, ptr_lo: u8, ptr_hi: u8) {
    // `lda Temp04 / lsr / bcc @endIf_andFE / dec Temp04`：`lsr` 把 bit0 送 C，`bcc` 在 C=0
    // （**偶数**地址）时跳过 ⇒ 只有**奇数**地址才 `dec`（对齐到左瓦）。
    let lo = if ptr_lo & 1 != 0 {
        ptr_lo.wrapping_sub(1)
    } else {
        ptr_lo
    };
    // `ldy #$00 / lda (Temp04),y / lsr / bcs @exit`：左瓦 bit0 置 ⇒ 不是 Zebetite 瓦；
    // 再 `cmp #$90>>1 / bcc`、`cmp #$98>>1 / bcs` ⇒ 必须在 `$90-$97`。
    let tile = mem.room().tile_raw((ptr_hi as u16) << 8 | lo as u16);
    if tile & 1 != 0 || !(0x90..=0x97).contains(&tile) {
        return;
    }
    for slot in 0..COUNT {
        if mem.zeb_status(slot) == 0 {
            continue; // `lda Zebetites.status,y / beq next`
        }
        let z = mem.zeb_ptr(slot);
        // `(Temp04_RoomRAMPtr & $9E) == roomRAMPtr` 且高字节相等
        if (lo & 0x9E) == (z as u8) && ptr_hi == (z >> 8) as u8 {
            mem.set_zeb_is_hit(slot, 1);
        }
    }
}

#[cfg(test)]
mod tests_zebetite {
    use super::*;
    use crate::world::tile_blast;

    fn mgr_with_zeb(slot: u8, ptr: u16, qty: u8) -> Manager {
        let mut m = Manager::default();
        m.write_ram(crate::ram_map::IN_AREA, 0x13); // Tourian（区号 3；vram_string 走 17 条真串）
        m.set_zeb_status(slot, 0x01);
        m.set_zeb_ptr(slot, ptr);
        m.set_zeb_qty_hits(slot, qty);
        m.set_zeb_healing_delay(slot, HEAL_DELAY);
        m
    }

    /// 结构锚：字段落在 `constants_ram.asm:76-83` 的偏移上（status0 / ptr1-2 / qty3 /
    /// healing4 / isHit5），槽长 8、基址 `$0758`。
    #[test]
    fn struct_offsets_match_asm() {
        let mut m = Manager::default();
        m.set_zeb_status(2, 0x81);
        m.set_zeb_ptr(2, 0x6542);
        m.set_zeb_qty_hits(2, 7);
        m.set_zeb_healing_delay(2, 0x40);
        m.set_zeb_is_hit(2, 1);
        let base = 0x0758u16 + 2 * 8;
        let raw: Vec<u8> = (base..base + 8).map(|a| m.read_ram(a)).collect();
        assert_eq!(raw, vec![0x81, 0x42, 0x65, 7, 0x40, 1, 0, 0], "Zebetite 槽字节序");
    }

    /// `UpdateAllZebetites` 循环 `x = $20/$18/$10/$08` ⇒ **槽 0 不更新**（asm `bne` 跳过）。
    #[test]
    fn update_all_skips_slot_zero() {
        let mut m = mgr_with_zeb(0, 0x6542, 2);
        m.set_zeb_is_hit(0, 1);
        update_all_zebetites(&mut m);
        assert_eq!(m.zeb_qty_hits(0), 2, "槽 0 不参与更新 ⇒ 命中未被消费");
        assert_eq!(m.zeb_is_hit(0), 1, "槽 0 的 isHit 也保持");
    }

    /// 偶数击才变薄：qty 1→2 触发重绘（`animFrame ← 0D`），3→4 触发（`0E`），奇数额不触发。
    #[test]
    fn even_hits_thin_the_barrier() {
        let mut m = mgr_with_zeb(4, 0x6542, 1);
        m.set_zeb_is_hit(4, 1);
        update_zebetite(&mut m, 0x20);
        assert_eq!(m.zeb_qty_hits(4), 2);
        assert_eq!(m.tb_anim_frame(1), 0x0D, "2 击 ⇒ 75% 厚度");
        // 3 击（奇数）⇒ 不改外观
        m.set_tb_anim_frame(1, 0xEE);
        m.set_zeb_qty_hits(4, 2);
        m.set_zeb_is_hit(4, 1);
        update_zebetite(&mut m, 0x20);
        assert_eq!(m.zeb_qty_hits(4), 3);
        assert_eq!(m.tb_anim_frame(1), 0xEE, "奇数额不变薄");
    }

    /// 8 击死亡：`status` bit1 置（`$0F` 掩码不再是 1 ⇒ 之后不再更新），外观 `07`（dead）。
    #[test]
    fn eight_hits_kill_the_barrier() {
        let mut m = mgr_with_zeb(4, 0x6542, 7);
        m.set_zeb_is_hit(4, 1);
        update_zebetite(&mut m, 0x20);
        assert_eq!(m.zeb_qty_hits(4), 8);
        assert_eq!(m.zeb_status(4) & 0x0F, 2, "status bit1 停更新");
        assert_eq!(m.tb_anim_frame(1), 0x07, "dead 图案");
        // 再调一次 ⇒ 直接返回，qty 不动
        m.set_zeb_is_hit(4, 1);
        update_zebetite(&mut m, 0x20);
        assert_eq!(m.zeb_qty_hits(4), 8);
    }

    /// 重绘失败回滚：`VRAMStructBufferIndex != 0` ⇒ 不画、`status=(status&$80)|1`、
    /// `isHit` 存同值（非 1，下帧还会重试）、`qtyHits-1`。
    #[test]
    fn gfx_failure_rolls_back_the_hit() {
        let mut m = mgr_with_zeb(4, 0x6542, 1);
        m.set_zeb_is_hit(4, 1);
        m.write_ram(crate::ram_map::VRAM_STRUCT_INDEX, 0x10); // 缓冲非空
        update_zebetite(&mut m, 0x20);
        assert_eq!(m.zeb_qty_hits(4), 1, "命中被回滚");
        assert_eq!(m.zeb_status(4), 0x01, "status 保持活");
        assert_eq!(m.zeb_is_hit(4), 0x01, "isHit 留痕（下帧重试）");
    }

    /// 自愈：`healingDelay` 从 `$40` 递减到 0 时 `qtyHits-1`、且**剩余为偶数**才重绘。
    #[test]
    fn heals_one_hit_per_delay() {
        let mut m = mgr_with_zeb(4, 0x6542, 2);
        m.set_zeb_healing_delay(4, 1);
        m.set_zeb_is_hit(4, 0);
        update_zebetite(&mut m, 0x20);
        assert_eq!(m.zeb_qty_hits(4), 1, "自愈一击");
        assert_eq!(m.zeb_healing_delay(4), HEAL_DELAY, "延时重置");
        // 剩余 1 是奇数 ⇒ 不重绘（asm `bcc` 才跳重绘，即偶数）
        // 再自愈 1→0：偶数 ⇒ 重绘到 100%（y=0）
        m.set_zeb_healing_delay(4, 1);
        update_zebetite(&mut m, 0x20);
        assert_eq!(m.zeb_qty_hits(4), 0);
        assert_eq!(m.tb_anim_frame(1), 0x0C, "满血 100% 图案");
    }

    /// 命中登记用**指针**比较：`(lo & $9E) == zeb.ptr.lo` 且 hi 相等；对齐到左瓦。
    #[test]
    fn missile_registers_hit_by_pointer() {
        let mut m = mgr_with_zeb(3, 0x6502, 0);
        m.write_ram(crate::ram_map::UPDATING_SAMUS_PROJECTILE, 1);
        m.object(0xD).set_status(crate::weapon::projectile::WA_MISSILE);
        m.room().set_tile_raw(0x6542, 0x92); // 左瓦是 Zebetite 瓦
        // 命中点 lo=$43（奇数 ⇒ dec 成 $42）⇒ `$42 & $9E = $02` == ptr.lo($42) ⇒ 命中
        let hit = tourian_bullet_hit(&mut m, 0xD, 0x92, 0x43, 0x65);
        assert!(hit, "Tourian 走双返回");
        assert_eq!(m.zeb_is_hit(3), 1);
        // 换一个不相干的指针 ⇒ 不登记
        let mut m2 = mgr_with_zeb(3, 0x6502, 0);
        m2.write_ram(crate::ram_map::UPDATING_SAMUS_PROJECTILE, 1);
        m2.object(0xD).set_status(crate::weapon::projectile::WA_MISSILE);
        m2.room().set_tile_raw(0x6582, 0x92);
        tourian_bullet_hit(&mut m2, 0xD, 0x92, 0x82, 0x65);
        assert_eq!(m2.zeb_is_hit(3), 0, "(lo&$9E=$82) != ($42&$9E=$02)");
    }

    /// 玻璃：`tile=$98` ⇒ 占一个空 TileBlast 槽、`ptr.lo=$8C`、`animFrame=01`（VRAMString01）。
    #[test]
    fn glass_breaks_into_tile_blast() {
        let mut m = Manager::default();
        m.write_ram(crate::ram_map::IN_AREA, 0x13); // Tourian
        m.write_ram(crate::ram_map::UPDATING_SAMUS_PROJECTILE, 1);
        m.object(0xD).set_status(crate::weapon::projectile::WA_MISSILE);
        tourian_bullet_hit(&mut m, 0xD, 0x98, 0x40, 0x65);
        // asm **不**设 routine ⇒ 槽仍是空的，但参数块被写了（ptr/animFrame）
        let slot = tile_blast::free_slot(&m).expect("槽仍空");
        assert_eq!(m.tb_anim_frame(slot), 0x01, "玻璃图案 = VRAMString01");
        assert_eq!(m.tb_ptr(slot) as u8, 0x8C, "ptr.lo=$8C");
        assert_eq!((m.tb_ptr(slot) >> 8) as u8, 0x65, "ptr.hi 取命中点高字节");
    }

    /// 非导弹弹：Tourian 里同样走双返回（爆炸消失），但不登记命中。
    #[test]
    fn non_missile_still_double_returns() {
        let mut m = mgr_with_zeb(3, 0x6542, 0);
        m.object(0xD).set_status(1); // wa_Bullet（常规弹）
        m.write_ram(crate::ram_map::UPDATING_SAMUS_PROJECTILE, 1);
        let hit = tourian_bullet_hit(&mut m, 0xD, 0x92, 0x43, 0x65);
        assert!(hit, "asm 的 @exit 对所有非导弹也双返回");
        assert_eq!(m.zeb_is_hit(3), 0, "不登记");
    }
}

/// `UpdateTourianItems ; 07:FDE3`（`prg7_engine.asm:12783`）——Tourian 两件"记史"副作用：
///
/// 1. 逃逸计时器**第一帧**（`EndTimer+1 == $99` 且 `clc / sbc EndTimer` 为 0）⇒ 把
///    `ui_MOTHERBRAIN`（`%001110<<10` ⇒ id `($00,$38)`）写进唯一物历史。
/// 2. 循环 Zebetite 槽（`x = $20/$18/$10/$08`，`sbc #8 / bne` 使槽 0 不参与）调
///    `CheckZebetite ; 07:FE05`：`status == 2`（`UpdateZebetite` 的 8 击置的 bit1）⇒
///    `status ← 3` 并把 `ui_ZEBETITE1 + 槽号`（`($00, $3C + slot*4)`）写进历史。
pub(crate) fn update_tourian_items(mem: &mut Manager) {
    // `lda EndTimer+1 / cmp #$99 / bne` ⇒ 之后 `clc / sbc EndTimer` 判低字节为 0
    if mem.end_timer_hi() == 0x99 && mem.read_ram(crate::ram_map::END_TIMER) == 0 {
        mem.unique_item_push((0x00, 0x38)); // ui_MOTHERBRAIN
    }
    let mut x: u8 = 0x20;
    loop {
        check_zebetite_defeated(mem, x);
        x = x.wrapping_sub(8);
        if x == 0 {
            return;
        }
    }
}

/// `CheckZebetite ; 07:FE05`：`status-2 == 0` 才动作（`sec / sbc #$02 / bne @RTS`）。
fn check_zebetite_defeated(mem: &mut Manager, x: u8) {
    let slot = x / 8;
    if mem.zeb_status(slot).wrapping_sub(2) != 0 {
        return;
    }
    mem.set_zeb_status(slot, 3); // `inc status`
    mem.unique_item_push((0x00, 0x3C + slot * 4)); // ui_ZEBETITE1 + slot
}

#[cfg(test)]
mod tests_tourian_items {
    use crate::frame::Manager;

    fn mgr() -> Manager {
        let mut m = Manager::default();
        m.write_ram(crate::ram_map::IN_AREA, 0x13); // Tourian
        m
    }

    /// `UpdateTourianItems`：8 击后的 `status=2` 被改成 3，并把 `ui_ZEBETITE1+槽*4` 记史。
    #[test]
    fn defeated_zebetites_are_recorded_once() {
        let mut m = mgr();
        m.set_zeb_status(4, 2); // 槽 4（x=$20）
        m.set_zeb_qty_hits(4, 8);
        super::update_tourian_items(&mut m);
        assert_eq!(m.zeb_status(4), 3, "status 2→3");
        // 历史里应出现 ($00,$3C+16=$4C) = ui_ZEBETITE5
        assert!(m.unique_item_find((0x00, 0x4C)), "ui_ZEBETITE5 入史");
        // 幂等：再跑一次不会重复入史（status 已不是 2）
        super::update_tourian_items(&mut m);
        assert_eq!(m.zeb_status(4), 3);
    }

    /// 逃逸计时器第一帧（`EndTimer+1=$99` 且低字节 0）⇒ `ui_MOTHERBRAIN` 入史。
    #[test]
    fn end_timer_first_frame_records_mother_brain() {
        let mut m = mgr();
        m.write_ram(crate::ram_map::END_TIMER + 1, 0x99);
        m.write_ram(crate::ram_map::END_TIMER, 0x00);
        super::update_tourian_items(&mut m);
        assert!(m.unique_item_find((0x00, 0x38)), "ui_MOTHERBRAIN 入史");
        // 低字节非 0 ⇒ 不是第一帧
        let mut m2 = mgr();
        m2.write_ram(crate::ram_map::END_TIMER + 1, 0x99);
        m2.write_ram(crate::ram_map::END_TIMER, 0x99);
        super::update_tourian_items(&mut m2);
        assert!(!m2.unique_item_find((0x00, 0x38)), "99.99 之后的帧不重复记");
    }
}
