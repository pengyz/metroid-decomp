//! 炸弹子系统：`CheckBombLaunch ; 07:D150` / `BombInit ; 07:D65E` /
//! `BombCountdown ; 07:D670` / `BombExplode ; 07:D691` /
//! `BombExplosion_CollisionWithBG ; 07:D6A7`（五相位炸瓦，bank7）。
//!
//! **状态号**（`constants.asm:118-127`）：`wa_LayBomb=8` / `wa_BombCount=9` /
//! `wa_BombExplode=0A`。`DoOneSamusProjectile ; 07:D4CB` 的 12 项跳表把
//! `5/8 → BombInit`、`6/9 → BombCountdown`、`7/10 → BombExplode`（两套半程各一份）。
//!
//! **五相位**（`SamusProjectileDieDelay` = 引爆后的帧计数，`BombExplode` 每帧 +1）：
//! 1 中心格 / 2 上两行 / 3 下两行 / 4 左移两格 / 5 右移两格；越界到属性表时按
//! 滚动方向决定是退出还是跳到对面名表。每格走 `CheckBlastTile` 阈值门
//! 与 `IsBlastTile` 的「$98 以上不炸」门，通过则占一个 `TileBlasts` 槽。

use crate::frame::Manager;

/// Samus 的三个弹槽（`SamusProjectiles instanceof Object 3 startfrom 0` ⇒ 13..15）。
pub(crate) const BOMB_SLOTS: [u8; 3] = [13, 14, 15];
/// `wa_LayBomb`（`constants.asm:124`）。
pub(crate) const WA_LAY_BOMB: u8 = 0x08;
/// `wa_BombCount`。
pub(crate) const WA_BOMB_COUNT: u8 = 0x09;
/// `ObjAnim_BombTick - ObjectAnimIndexTbl` = `$85F1 - $8572`（见 `samus/physics.rs`
/// 的 `OBJECT_ANIM_INDEX_TBL` 烘焙：index 0 = `$8572`）。
const OBJ_ANIM_BOMB_TICK: u8 = 0x7F;
/// `ObjAnim_BombExplode - ObjectAnimIndexTbl` = `$85F4 - $8572`。
const OBJ_ANIM_BOMB_EXPLODE: u8 = 0x82;
/// `ObjAnim_SamusRunPntUp - ObjectAnimIndexTbl` = `$85A9 - $8572`——
/// asm 在 `status != wa_BombCount` 时用它（注释自问「?」，是原版原样）。
const OBJ_ANIM_SAMUS_RUN_PNT_UP: u8 = 0x37;
/// 引信长度（`BombInit`：`lda #$18 / sta SamusProjectileDieDelay,x`）。
const FUSE: u8 = 0x18;
/// `_id_FrameNone`（`constants.asm:130`）。
const ID_FRAME_NONE: u8 = 0xF7;

/// `CheckBombLaunch ; 07:D150`：滚球态按 B 埋弹（三槽依次找空位）。
///
/// 门：①`SamusGear` bit1（有炸弹）②`Joy1Change|Joy1Retrig` 的 FIRE 位 ③
/// `Samus.speedY == 0 && SamusOnElevator == 0`。
pub fn check_bomb_launch(mem: &mut Manager) {
    if mem.samus_gear() & 0x02 == 0 {
        return; // 没有炸弹能力
    }
    let fire = mem.joy1_change() | mem.joy1_retrig();
    if fire & 0x40 == 0 {
        return; // FIRE 未按（bit6）
    }
    if mem.samus().speed_y() != 0 || mem.samus().on_elevator() {
        return; // 在空中或电梯上
    }
    let slot = match BOMB_SLOTS.iter().find(|s| mem.object(**s).status() == 0) {
        Some(s) => *s,
        None => return, // 三槽全占
    };
    let (hi, y, x) = {
        let s = mem.samus();
        (s.hi(), s.y(), s.x())
    };
    {
        let mut o = mem.object(slot);
        o.set_hi(hi);
        o.set_x(x);
        o.set_y(y.wrapping_add(4)); // 比 Samus 中心低 4px
        o.set_status(WA_LAY_BOMB);
    }
    // `jsr SFX_BombLaunch ; 07:CBA8` = `sfxTri_BombLaunch(%00000001)` 置位
    let f = mem.tri_init_flags();
    mem.set_tri_init_flags(f | 0x01);
}

/// `BombInit ; 07:D65E`：装引信 + 播「滴」动画，status 推进一格。
pub fn bomb_init(mem: &mut Manager) {
    let slot = bomb_slot(mem);
    {
        let mut o = mem.object(slot);
        o.set_anim_reset_index(OBJ_ANIM_BOMB_TICK); // InitObjectXSlotAnimIndex
        o.set_anim_index(OBJ_ANIM_BOMB_TICK);       // SetObjectXSlotAnimIndex
        o.set_anim_delay(0);
        let st = o.status().wrapping_add(1);
        o.set_status(st);
    }
    mem.object(slot).set_die_delay(FUSE); // SamusProjectileDieDelay
    draw_bomb(mem);
}

/// `BombCountdown ; 07:D670`：**只在奇数帧**递减引信；归零即起爆。
pub fn bomb_countdown(mem: &mut Manager) {
    // `lda FrameCount / lsr / bcc` ⇒ bit0 为 0 时跳过（只奇数帧动）
    if mem.frame_count() & 1 == 0 {
        draw_bomb(mem);
        return;
    }
    let slot0 = bomb_slot(mem);
    let d = mem.object(slot0).die_delay().wrapping_sub(1);
    mem.object(slot0).set_die_delay(d);
    if d != 0 {
        draw_bomb(mem);
        return;
    }
    // 起爆：动画切到爆炸帧（status==wa_BombCount 时；另一半程用 SamusRunPntUp——原样）
    let slot = bomb_slot(mem);
    {
        let mut o = mem.object(slot);
        let st = o.status();
        let anim = if st == WA_BOMB_COUNT { OBJ_ANIM_BOMB_EXPLODE } else { OBJ_ANIM_SAMUS_RUN_PNT_UP };
        o.set_anim_reset_index(anim);
        o.set_anim_index(anim);
        o.set_anim_delay(0);
        o.set_status(st.wrapping_add(1));
    }
    // `jsr SFX_BombExplode ; 07:CB9A` = `sfxNoise_BombExplode(%00010000)`
    crate::sound::hook::set_sfx_noise_init(mem, 0x10);
    draw_bomb(mem);
}

/// `BombExplode ; 07:D691`：每帧推进爆心相位，动画播完即删槽。
pub fn bomb_explode(mem: &mut Manager) {
    let slot0 = bomb_slot(mem);
    let d = mem.object(slot0).die_delay().wrapping_add(1);
    mem.object(slot0).set_die_delay(d);
    bomb_explosion_collision_with_bg(mem, d);
    let slot = bomb_slot(mem);
    let frame = mem.object(slot).anim_frame();
    if frame == ID_FRAME_NONE {
        mem.object(slot).set_status(0);
        return;
    }
    draw_bomb(mem);
}

/// `DrawBomb ; 07:D66B` = `lda #$03 / jmp AnimDrawObject`（每 3 帧换一帧）。
fn draw_bomb(mem: &mut Manager) {
    let slot = bomb_slot(mem);
    let page = slot * 16;
    mem.set_page_index(page);
    crate::render::oam::anim_draw_object(mem, page, 3);
}

/// `PageIndex / 16` 反推槽号（`DoOneSamusProjectile` 用 `ldx PageIndex`）。
fn bomb_slot(mem: &Manager) -> u8 {
    let page = mem.page_index();
    debug_assert_eq!(page % 16, 0, "槽号由 PageIndex 反推，必须是 16 的倍数");
    page / 16
}

/// `BombExplosion_CollisionWithBG ; 07:D6A7`：按爆心相位炸瓦。
fn bomb_explosion_collision_with_bg(mem: &mut Manager, die_delay: u8) {
    let slot = bomb_slot(mem);
    let (y, x, hi) = {
        let o = mem.object(slot);
        (o.y(), o.x(), o.hi())
    };
    let mut ptr = crate::enemy::motion::make_room_ram_ptr(mem, y, x, hi);
    match die_delay {
        1 => bomb_current_tile(mem, ptr), // 中心格
        2 => {
            // 上两行：先减 $40
            let mut p = ld_add(ptr, -0x40);
            // asm：`txa / bne` 用上一步的 X（$40）非零 ⇒ 恒跳；再查 `lo & $20`
            if (p as u8) & 0x20 != 0 {
                return;
            }
            if underflowed_to_attributes(p) {
                if mem.camera().dir() & 0x02 != 0 {
                    return; // 横滚房退出
                }
                p = ld_add(p, -0x80); // 跳过属性表到对面名表
            }
            ptr = p;
            bomb_current_tile(mem, ptr);
        }
        3 => {
            // 下两行：加 $40
            let mut p = ld_add(ptr, 0x40);
            if (p as u8) & 0x20 == 0 {
                return;
            }
            if underflowed_to_attributes(p) {
                if mem.camera().dir() & 0x02 != 0 {
                    return;
                }
                p = ld_add(p, 0x80);
            }
            ptr = p;
            bomb_current_tile(mem, ptr);
        }
        4 => {
            // 左两格：减 2
            let mut p = ld_add(ptr, -2);
            if (p as u8) & 1 != 0 {
                return; // `lsr / bcc` 的等价：lo 为奇数 ⇒ 跨出元 tile
            }
            if (p as u8) & 0x1F >= 0x1E {
                if mem.camera().dir() & 0x02 == 0 {
                    return; // 竖滚房才继续
                }
                p = ld_add(p, 0x1E); // $20-$02
                p ^= crate::ram_map::NAMETABLE_BIT; // 切对面名表
            }
            ptr = p;
            bomb_current_tile(mem, ptr);
        }
        5 => {
            // 右两格：加 2
            let mut p = ld_add(ptr, 2);
            if (p as u8) & 1 != 0 {
                return;
            }
            if (p as u8) & 0x1F >= 0x02 {
                bomb_current_tile(mem, p);
                return;
            }
            if mem.camera().dir() & 0x02 == 0 {
                return;
            }
            p = ld_add(p, 0x1E); // `lda #$20-$02 / jsr LD78B`
            p ^= crate::ram_map::NAMETABLE_BIT;
            bomb_current_tile(mem, p);
        }
        _ => {}
    }
}

/// `BombCurrentTile ; 07:D76A`：该格够门槛且不到 $A0、也未到 $98 ⇒ 占一个瓦片爆槽。
fn bomb_current_tile(mem: &mut Manager, ptr: u16) {
    let tile = mem.room().tile_raw(ptr);
    let in_area = crate::frame::in_area_raw(mem);
    // `jsr CheckBlastTile ; 07:D651`：CF=1 ⇔ tile ≥ 阈值（Brinstar $80，其余 $70）
    let threshold = if in_area == 0x10 { 0x80 } else { 0x70 };
    if tile < threshold {
        return;
    }
    if tile >= 0xA0 {
        return; // 可走格不必炸
    }
    // `jsr IsBlastTile@SkipCheckUpdatingSamusProjectile ; 07:E9C2`：
    // `cpy #$98 / bcs @exit`（$98-$9F 不炸）+ 找空槽占位。
    if tile >= 0x98 {
        return;
    }
    let slot = match crate::world::tile_blast::free_slot(mem) {
        Some(s) => s,
        None => return, // 13 槽全占 ⇒ 不炸（asm `bne @exit`）
    };
    crate::world::tile_blast::claim_tile_blast(mem, slot, ptr, tile, in_area);
}

/// `LD77F ; 07:D77F` / `LD78B ; 07:D78B` 的算术：16 位加/减后
/// `and #$07 / ora #>RoomRAMA($60)` 回到 RoomRAM 高字节。
fn ld_add(ptr: u16, delta: i16) -> u16 {
    let (lo, hi) = (ptr as u8, (ptr >> 8) as u8);
    let v = ((hi as i32) << 8 | lo as i32) + delta as i32;
    pack_ptr(v as u16)
}

/// `LD798 ; 07:D798`：`and #$07 / ora #$60`。
fn pack_ptr(ptr: u16) -> u16 {
    let (lo, hi) = (ptr as u8, (ptr >> 8) as u8);
    ((((hi & 0x07) | 0x60) as u16) << 8) | lo as u16
}

/// 「下溢/溢出到属性表」判据（`hi & $03 == $03` 且 `lo >= $C0`）。
fn underflowed_to_attributes(ptr: u16) -> bool {
    let (lo, hi) = (ptr as u8, (ptr >> 8) as u8);
    hi & 0x03 == 0x03 && lo >= 0xC0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn m() -> Manager {
        Manager::default()
    }

    fn give_bomb_gear(mem: &mut Manager) {
        mem.set_samus_gear(0x02);
    }

    fn press_fire(mem: &mut Manager) {
        mem.set_joy1_change(0x40);
    }

    /// 没炸弹齿轮 ⇒ 按 B 不出弹（asm `SamusGear lsr / bcc @RTS`）。
    #[test]
    fn launch_requires_bomb_gear() {
        let mut mem = m();
        press_fire(&mut mem);
        check_bomb_launch(&mut mem);
        assert_eq!(mem.object(13).status(), 0);
    }

    /// 齿轮 + FIRE ⇒ 在槽 D 埋弹：y 比 Samus 低 4px、hi/x 复制、status=wa_LayBomb、音效位置位。
    #[test]
    fn launch_places_bomb_four_pixels_below_samus() {
        let mut mem = m();
        give_bomb_gear(&mut mem);
        press_fire(&mut mem);
        mem.samus().set_y(0x50);
        mem.samus().set_x(0x40);
        mem.samus().set_hi(0x01);
        check_bomb_launch(&mut mem);
        let o = mem.object(13);
        assert_eq!(o.status(), WA_LAY_BOMB);
        assert_eq!(o.y(), 0x54, "y = Samus.y + 4");
        assert_eq!(o.x(), 0x40);
        assert_eq!(o.hi(), 0x01);
        assert_eq!(mem.tri_init_flags() & 0x01, 0x01, "SFX_BombLaunch 置 tri 位");
    }

    /// 空中（speedY != 0）按 B 无效。
    #[test]
    fn launch_blocked_in_air() {
        let mut mem = m();
        give_bomb_gear(&mut mem);
        press_fire(&mut mem);
        mem.samus().set_speed_y(2);
        check_bomb_launch(&mut mem);
        assert_eq!(mem.object(13).status(), 0);
    }

    /// 三槽全占 ⇒ 不出新弹。
    #[test]
    fn launch_blocked_when_all_three_slots_busy() {
        let mut mem = m();
        give_bomb_gear(&mut mem);
        press_fire(&mut mem);
        for s in BOMB_SLOTS {
            mem.object(s).set_status(WA_LAY_BOMB);
        }
        check_bomb_launch(&mut mem);
        for s in BOMB_SLOTS {
            assert_eq!(mem.object(s).status(), WA_LAY_BOMB, "既有弹未被覆盖");
        }
    }

    /// 引信只在**奇数帧**递减（asm `FrameCount lsr / bcc`）。
    #[test]
    fn countdown_ticks_only_on_odd_frames() {
        let mut mem = m();
        mem.set_page_index(13 * 16);
        mem.object(13).set_status(WA_BOMB_COUNT);
        mem.object(13).set_die_delay(0x10);
        mem.set_frame_count(0x20); // 偶数帧
        bomb_countdown(&mut mem);
        assert_eq!(mem.object(13).die_delay(), 0x10, "偶数帧不动");
        mem.set_frame_count(0x21); // 奇数帧
        bomb_countdown(&mut mem);
        assert_eq!(mem.object(13).die_delay(), 0x0F, "奇数帧减一");
    }

    /// 引信归零 ⇒ 切爆炸动画、status 前进一步、请求 noise 音效。
    #[test]
    fn countdown_zero_arms_explosion() {
        let mut mem = m();
        mem.set_page_index(13 * 16);
        mem.object(13).set_status(WA_BOMB_COUNT);
        mem.object(13).set_die_delay(0x01);
        mem.set_frame_count(0x21);
        bomb_countdown(&mut mem);
        assert_eq!(mem.object(13).status(), WA_BOMB_COUNT + 1);
        // 回绕点是 BombExplode；`animIndex` 随后被本帧的 `AnimDrawObject`（UpdateObjAnim）
        // 推进一步 ⇒ 只钉回绕点与它的起点关系（不钉被绘制推进后的游标）。
        assert_eq!(mem.object(13).anim_reset_index(), OBJ_ANIM_BOMB_EXPLODE);
        assert_eq!(mem.object(13).anim_index(), OBJ_ANIM_BOMB_EXPLODE + 1);
        assert_eq!(mem.noise_init_flags() & 0x10, 0x10);
    }

    /// 爆炸动画走到 `_id_FrameNone` ⇒ 槽被释放（status=0）。
    #[test]
    fn explode_releases_slot_at_last_frame() {
        let mut mem = m();
        mem.set_page_index(13 * 16);
        mem.object(13).set_status(0x0A);
        mem.object(13).set_anim_frame(ID_FRAME_NONE);
        mem.object(13).set_die_delay(0x03);
        bomb_explode(&mut mem);
        assert_eq!(mem.object(13).status(), 0);
    }

    /// 相位 1 只炸中心格：可炸瓦（$84）⇒ 占一个瓦片爆槽，指针 = 弹所在 RoomRAM 地址。
    #[test]
    fn explode_phase_one_claims_center_tile() {
        let mut mem = m();
        mem.set_page_index(13 * 16);
        mem.object(13).set_status(0x0A);
        mem.object(13).set_y(0x50);
        mem.object(13).set_x(0x40);
        mem.object(13).set_hi(0x00);
        let ptr = crate::enemy::motion::make_room_ram_ptr(&mut mem, 0x50, 0x40, 0x00);
        mem.room().set_tile_raw(ptr, 0x84);
        crate::frame::set_in_area_raw(&mut mem, 0x10); // Brinstar
        mem.object(13).set_die_delay(0x00); // +1 ⇒ 相位 1
        mem.object(13).set_anim_frame(0x00);
        bomb_explode(&mut mem);
        assert_eq!(crate::world::tile_blast::active_count(&mem), 1);
        // TileBlasts 槽 0 的指针字段：+9(lo) / +10(hi)
        let got = mem.tb_ptr(0);
        assert_eq!(got, ptr & 0xFFDE, "槽里存的是掩码后的 RoomRAM 地址");
    }

    /// 可走瓦（≥$A0）不炸：相位 1 后没有瓦片爆槽。
    #[test]
    fn explode_phase_one_skips_walkable_tile() {
        let mut mem = m();
        mem.set_page_index(13 * 16);
        mem.object(13).set_status(0x0A);
        mem.object(13).set_y(0x50);
        mem.object(13).set_x(0x40);
        let ptr = crate::enemy::motion::make_room_ram_ptr(&mut mem, 0x50, 0x40, 0x00);
        mem.room().set_tile_raw(ptr, 0xC0);
        crate::frame::set_in_area_raw(&mut mem, 0x10);
        mem.object(13).set_die_delay(0x00);
        bomb_explode(&mut mem);
        assert_eq!(crate::world::tile_blast::active_count(&mem), 0);
    }

    /// `$98-$9F` 不炸（asm `IsBlastTile` 首件 `cpy #$98 / bcs @exit`）。
    #[test]
    fn explode_skips_tiles_at_or_above_98() {
        let mut mem = m();
        mem.set_page_index(13 * 16);
        mem.object(13).set_status(0x0A);
        mem.object(13).set_y(0x50);
        mem.object(13).set_x(0x40);
        let ptr = crate::enemy::motion::make_room_ram_ptr(&mut mem, 0x50, 0x40, 0x00);
        mem.room().set_tile_raw(ptr, 0x9A);
        crate::frame::set_in_area_raw(&mut mem, 0x10);
        mem.object(13).set_die_delay(0x00);
        bomb_explode(&mut mem);
        assert_eq!(crate::world::tile_blast::active_count(&mem), 0);
    }
}
