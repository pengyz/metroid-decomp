//! Mother Brain（`MotherBrainStatusHandler ; 03:9DD4`）——Tourian 终局 Boss。
//!
//! 状态字节 `MotherBrainStatus`（`$0098`）十一态：
//! `0` 不在房内 / `1` 在房内（Idle）/ `2` 受击闪 / `3` 死亡闪+清场 / `4` 分五批消融 /
//! `5`|`8` "TIME BOMB SET" 字幕（同址同码）/ `6`|`9` 设时炸弹（造门 + 9999 帧倒计时）/
//! `7` 时炸弹爆炸 / `A` 已死（RTS）。
//!
//! 只在 Tourian（`InArea & $0F == 3`）出现；帧号/动画 id 取 Tourian 的
//! `EnFramePtrTable1` 顺序（`Pulsations0..3` = 19..22、`Eyes` = 23，见
//! `refs/m1disasm/SRC/tourian/enemy_sprite_data.asm:94-98` 的表序）。

use crate::frame::Manager;

/// `EnMotherBrain-Ens`（`constants_ram.asm:973-976`：`$0400` 起 16 字节槽，母脑 = 槽 `$E`）。
pub(crate) const SLOT_PAGE: u8 = 0xE0;

/// `MotherBrainAnimFrameTable ; 03:A06D`（Tourian 表序 = 帧 id）。
const MB_FRAMES: [u8; 5] = [19, 20, 21, 22, 23];

/// `MotherBrainFlashPaletteTable ; 03:9E41` = `[_id_Palette07+1, _id_Palette06+1]`
/// （Tourian 调色板表序 `00,01,03,02,04,05,06,07,08,09,0A…` ⇒ `07`=8、`06`=7）。
const FLASH_PALETTE: [u8; 2] = [8, 7];
/// `MotherBrain_Disappear_PaletteTable ; 03:9F00` = `[_id_Palette08+1, _id_Palette09+1]` ⇒ 9、10。
const DISAPPEAR_PALETTE: [u8; 2] = [9, 10];

/// `MotherBrainDeathString ; 03:A0A3`（相对 `$6144` 的 ptr.lo 增量）。
const DEATH_STRINGS: [&[u8]; 5] = [
    &[0x00, 0x02, 0x04, 0x06, 0x08, 0x40, 0x80, 0xC0, 0x48, 0x88, 0xC8, 0xFF],
    &[0x42, 0x81, 0xC1, 0x27, 0xFF],
    &[0x82, 0x43, 0x25, 0x47, 0xFF],
    &[0xC2, 0xC4, 0xC6, 0xFF],
    &[0x84, 0x45, 0x86, 0xFF],
];
/// `MotherBrainDeathStringOffsets ; 03:A0C1`。
const DEATH_OFFSETS: [u8; 5] = [0, 12, 17, 22, 26];

/// `@roomRAMPtrTable`（相对 `$6142`）——"TIME BOMB SET" 八片字幕的 RoomRAM 低字节。
const BOMB_MSG_PTR: [u8; 8] = [0x00, 0x40, 0x08, 0x48, 0x80, 0xC0, 0x88, 0xC8];
/// `@animFrameTable`——八片字幕的 VRAM 串 id。
const BOMB_MSG_ANIM: [u8; 8] = [0x08, 0x02, 0x09, 0x03, 0x0A, 0x04, 0x0B, 0x05];

/// `DoorSlots_ ; 03:9F65` = `[Doors.0, Doors.3, Doors.2, Doors.1] - Objects`
/// （`Doors.0` = `$0380` ⇒ 槽 8/11/10/9）。
const DOOR_SLOTS: [u8; 4] = [8, 11, 10, 9];
/// `ObjAnim_55 - ObjectAnimIndexTbl.b`（门的开合动画 id）。
const OBJ_ANIM_55: u8 = 0x55;
/// `_id_FrameNone`。
const FRAME_NONE: u8 = 0xF7;

/// `MotherBrainStatusHandler ; 03:9DD4`：`beq @RTS` + 11 项跳表。
pub(crate) fn status_handler(mem: &mut Manager) {
    match mem.mb_status() {
        0x00 | 0x0A => {} // 不在房内 / 已死：RTS
        0x01 => idle(mem),
        0x02 => hurt(mem),
        0x03 => killed(mem),
        0x04 => disappear(mem),
        0x05 | 0x08 => time_bomb_message(mem),
        0x06 | 0x09 => set_time_bomb(mem),
        0x07 => time_bomb_exploded(mem),
        _ => {}
    }
}

/// `MotherBrain_Idle ; 03:9E22`：吸血碰撞 → 受击处理 → 脑/眼动画 → 画精灵 → 清命中旗。
fn idle(mem: &mut Manager) {
    collide_with_samus(mem);
    handle_being_hit(mem);
    update_anim_brain(mem);
    update_anim_eye(mem);
    draw_sprites(mem);
    mem.set_mb_is_hit(0);
}

/// `MotherBrain_Idle_CollideWithSamus ; 03:9DF2`。
fn collide_with_samus(mem: &mut Manager) {
    if mem.samus().hi() ^ mem.mb_hi() != 0 {
        return;
    }
    // `Samus.x - $48` 有借位（x < $48）⇒ 退
    let sx = mem.samus().x();
    if sx < 0x48 || sx.wrapping_sub(0x48) >= 0x2F {
        return;
    }
    // `|Samus.y - $80| >= $20` ⇒ 退
    let sy = mem.samus().y();
    let d = sy.wrapping_sub(0x80);
    let ad = if d & 0x80 != 0 { d.wrapping_neg() } else { d };
    if ad >= 0x20 {
        return;
    }
    mem.set_health_change(0x0200);
    mem.samus().set_is_hit(0x38);
    crate::weapon::projectile::subtract_health(mem);
}

/// `MotherBrain_Idle_HandleBeingHit ; 03:9FED`。
fn handle_being_hit(mem: &mut Manager) {
    if mem.mb_is_hit() == 0 {
        return;
    }
    // SFXMultiInitFlags |= sfxMulti_BossHit（asm `$0684` 的 multi 队面）
    mem.or_sfx_multi_init(0x04); // sfxMulti_BossHit
    let qty = mem.mb_qty_hits().wrapping_add(1);
    mem.set_mb_qty_hits(qty);
    if qty < 0x20 {
        mem.set_mb_status(0x02);
        mem.set_mb_flash_delay(0x10);
    } else {
        // 清全部 13 个 TileBlast 的 routine
        for slot in 0..crate::world::tile_blast::SLOTS {
            mem.set_tb_routine(slot as u8, 0);
        }
        mem.set_mb_status(0x03);
        mem.set_mb_flash_delay(0x80);
    }
}

/// `MotherBrain_Idle_UpdateAnimBrain ; 03:A01B`。
fn update_anim_brain(mem: &mut Manager) {
    let d = mem.mb_brain_delay().wrapping_sub(1);
    mem.set_mb_brain_delay(d);
    if d != 0 {
        return;
    }
    mem.set_mb_frame_id(mem.random_number1() & 0x03);
    let delay = 0x20u8.wrapping_sub(mem.mb_qty_hits()) >> 1;
    mem.set_mb_brain_delay(delay);
}

/// `MotherBrain_Idle_UpdateAnimEye ; 03:A02E`。
fn update_anim_eye(mem: &mut Manager) {
    let d = mem.mb_eye_delay().wrapping_sub(1);
    mem.set_mb_eye_delay(d);
    // `lda delay / asl / bne` ⇒ 只有 0 或 $80 才继续
    if d.wrapping_mul(2) != 0 {
        return;
    }
    let v = (0x20u8.wrapping_sub(mem.mb_qty_hits()) | 0x80) ^ d;
    mem.set_mb_eye_delay(v);
}

/// `MotherBrain_DrawSprites ; 03:A041`。
fn draw_sprites(mem: &mut Manager) {
    let page = SLOT_PAGE;
    let hi = mem.mb_hi();
    let fid = mem.mb_frame_id();
    {
        let mut e = mem.ens_face(page);
        e.set_hi(hi);
        e.set_y(0x70);
        e.set_pos_x(0x48);
        e.set_anim_frame(MB_FRAMES[(fid & 0x07) as usize]);
    }
    crate::render::enemy_sprite::draw_enemy(mem, page);
    // 眼开的帧（`AnimEyeDelay` bit7 清）额外画眼睛帧
    if mem.mb_eye_delay() & 0x80 == 0 {
        mem.ens_face(page).set_anim_frame(MB_FRAMES[4]);
        crate::render::enemy_sprite::draw_enemy(mem, page);
    }
}

/// `UpdateMotherBrainFlashDelay ; 03:9E43`：返回 `y = (delay & $02) >> 1`。
fn update_flash_delay(mem: &mut Manager) -> u8 {
    let d = mem.mb_flash_delay().wrapping_sub(1);
    mem.set_mb_flash_delay(d);
    if d == 0 {
        mem.set_mb_status(0x01);
    }
    (d & 0x02) >> 1
}

/// `MotherBrain_Hurt ; 03:9E36`。
fn hurt(mem: &mut Manager) {
    let y = update_flash_delay(mem);
    let p = FLASH_PALETTE[(y & 1) as usize];
    mem.set_palette_pending(p);
    mem.set_mb_is_hit(0);
}

/// `MotherBrain_Killed ; 03:9E52`。
fn killed(mem: &mut Manager) {
    let y = update_flash_delay(mem);
    let p = FLASH_PALETTE[(y & 1) as usize];
    mem.set_palette_pending(p);
    // 屏幕竖直抖动：`ScrollY ← y*4`
    mem.camera().set_scroll_y(y.wrapping_mul(4));
    // `ldy MotherBrainStatus / dey / bne` ⇒ 只有退回 1（闪完）才继续
    if mem.mb_status().wrapping_sub(1) != 0 {
        draw_and_clear(mem);
        return;
    }
    mem.set_mb_qty_hits(0); // DeathStringID ← 0
    // 清 12 个敌槽（正常敌 0-5 + 敌弹 6-B）
    for slot in 0..12u8 {
        let page = slot.wrapping_mul(16);
        mem.ens_face(page).set_status(crate::enemy::state::EnemyStatus::NoEnemy);
    }
    mem.set_mb_status(0x04);
    mem.set_mb_flash_delay(0x28);
    crate::sound::hook::set_sfx_noise_init(mem, 0x20); // sfxNoise_SilenceMusic
    draw_and_clear(mem);
}

/// `MotherBrain_Idle@draw`：画精灵 + 清命中旗。
fn draw_and_clear(mem: &mut Manager) {
    draw_sprites(mem);
    mem.set_mb_is_hit(0);
}

/// `MotherBrain_Disappear ; 03:9E86`。
fn disappear(mem: &mut Manager) {
    crate::sound::hook::set_sfx_noise_init(mem, 0x10); // sfxNoise_BombExplode
    disintegrate(mem);
    let instr = mem.mb_brain_delay().wrapping_add(1);
    mem.set_mb_brain_delay(instr);
    let y = update_flash_delay(mem);
    // 前四个敌槽若是 Pickup ⇒ 清成 NoEnemy
    for slot in 0..4u8 {
        let page = slot.wrapping_mul(16);
        let mut e = mem.ens_face(page);
        if e.status() == crate::enemy::state::EnemyStatus::Pickup {
            e.set_status(crate::enemy::state::EnemyStatus::NoEnemy);
        }
    }
    if mem.vram_struct_index() == 0 {
        let p = DISAPPEAR_PALETTE[(y & 1) as usize];
        mem.set_palette_pending(p);
    }
    if mem.mb_status().wrapping_sub(1) != 0 {
        return;
    }
    // 一批消融完成
    mem.set_mb_brain_delay(0); // DeathInstrID ← 0
    mem.set_mb_status(0x04);
    mem.set_mb_flash_delay(0x1C);
    let id = mem.mb_qty_hits().wrapping_add(1); // DeathStringID++
    mem.set_mb_qty_hits(id);
    if id.wrapping_sub(1) == 4 {
        // 旧 id == 4 ⇒ 最后一批更短
        mem.set_mb_flash_delay(0x0E);
        return;
    }
    if id.wrapping_sub(1) > 4 {
        // `MusicInitFlags |= music_Escape`（asm `$0692` 的 music 队列面）
        mem.or_music_init_flags(0x20);
        mem.set_mb_status(0x05);
        mem.set_mb_qty_hits(0x80); // TimeBombCounter = -$80
    }
}

/// `MotherBrain_Disappear_Disintegrate ; 03:A072`。
fn disintegrate(mem: &mut Manager) {
    let id = mem.mb_qty_hits();
    if id == 0 {
        return;
    }
    let instr = mem.mb_brain_delay();
    // asm 是把五批串**首尾相接**放在一张表里，`y = DeathStringOffsets[ID-1] + InstrID`
    // 直接索引整表；这里按批取切片，等价于 `InstrID` 索引批内偏移。
    let _ = DEATH_OFFSETS; // 表只用于表达"五批串连续"，见上
    let bytes = DEATH_STRINGS[(id - 1).min(4) as usize];
    let Some(&b) = bytes.get(instr as usize) else { return };
    if b == 0xFF {
        mem.set_mb_brain_delay(instr.wrapping_sub(1));
        return;
    }
    // ptr.lo = b + $44；ptr.hi = (MotherBrainHi<<2)|$61 + 进位
    let lo = b.wrapping_add(0x44);
    let carry = lo < b;
    let hi_base = (mem.mb_hi() << 2) | 0x61;
    let hi = hi_base.wrapping_add(carry as u8);
    mem.set_tb_ptr(0, (hi as u16) << 8 | lo as u16);
    mem.set_tb_anim_frame(0, 0x00); // _id_VRAMString00（2×2 清）
    if crate::world::tile_blast::draw_tile_blast_slot(mem, 0) {
        // 画成功（asm 是 jmp DrawTileBlast，失败只影响 CF，不撤销 ptr 写）
    }
}

/// `MotherBrain_TimeBombMessage ; 03:9F02`（=`_ScrollBackOnScreen ; 03:9F02` 同址同码）。
fn time_bomb_message(mem: &mut Manager) {
    let counter = mem.mb_qty_hits();
    if counter & 0x80 != 0 {
        // counter < 0：等时炸弹"设置"流程继续推进
        mem.set_mb_qty_hits(counter.wrapping_add(1));
        return;
    }
    if counter == 8 {
        mem.set_mb_status(mem.mb_status().wrapping_add(1)); // → 6 或 9
        return;
    }
    let y = (counter & 0x07) as usize;
    let lo = BOMB_MSG_PTR[y].wrapping_add(0x42);
    let carry = lo < BOMB_MSG_PTR[y];
    let hi = ((mem.mb_hi() << 2) | 0x61).wrapping_add(carry as u8);
    mem.set_tb_anim_frame(0, BOMB_MSG_ANIM[y]);
    mem.set_tb_ptr(0, (hi as u16) << 8 | lo as u16);
    if mem.vram_struct_index() != 0 {
        return;
    }
    if !crate::world::tile_blast::draw_tile_blast_slot(mem, 0) {
        return;
    }
    mem.set_mb_qty_hits(counter.wrapping_add(1));
}

/// `MotherBrain_SetTimeBomb ; 03:9F49` / `_ScrollBackOnScreen ; 03:9FDA`。
fn set_time_bomb(mem: &mut Manager) {
    if !spawn_door(mem) {
        return; // CF 置 = 画瓦失败，下帧重试
    }
    if mem.mb_status() == 0x06 {
        mem.set_mb_status(0);
        mem.set_end_timer_pair(0x99);
        mem.set_end_timer_enemy_enabled(1);
        mem.set_end_timer_enemy_hi(mem.mb_hi());
    } else {
        mem.set_end_timer_enemy_hi(mem.mb_hi());
        mem.set_end_timer_enemy_enabled(1);
        mem.set_mb_status(0);
    }
}

/// `MotherBrain_SpawnDoor ; 03:9F69`——返回 true=瓦画出去了（`DrawTileBlast` CF 清）。
fn spawn_door(mem: &mut Manager) -> bool {
    let (x, y) = mem.map_pos();
    // `clc / adc / sec / rol / and #$03`
    let idx = (((x.wrapping_add(y)) << 1) | 1) & 0x03;
    let slot = DOOR_SLOTS[(idx & 0x03) as usize];
    let mb_hi = mem.mb_hi();
    {
        let mut d = mem.object(slot);
        d.set_radius_y(0x01); // DoorHitPoints
        d.set_door_type(0x01); // 蓝门
        d.set_status(0x03); // 开门
        d.set_hi(mb_hi);
        d.set_x(0x10);
        d.set_y(0x68);
        d.set_anim_reset_index(OBJ_ANIM_55);
        d.set_anim_index(OBJ_ANIM_55);
        d.set_anim_delay(0x00);
        d.set_anim_frame(FRAME_NONE);
    }
    mem.set_tb_anim_frame(0, 0x10); // _id_VRAMString10
    let hi = (mb_hi << 2) | 0x61;
    mem.set_tb_ptr(0, ((hi as u16) << 8) | 0x40);
    crate::world::tile_blast::draw_tile_blast_slot(mem, 0)
}

/// `MotherBrain_TimeBombExploded ; 03:9FC0`。
fn time_bomb_exploded(mem: &mut Manager) {
    crate::sound::hook::set_sfx_noise_init(mem, 0x10); // sfxNoise_BombExplode
    if mem.timer3() != 0 {
        return;
    }
    mem.samus().set_status(crate::samus::state::SamusStatus::Dead2);
    mem.set_mb_status(0x0A);
    mem.set_palette_pending(1); // _id_Palette00+1
}

#[cfg(test)]
mod tests_mother_brain {
    use crate::enemy::state::EnemyStatus;
    use crate::frame::Manager;

    fn mgr() -> Manager {
        let mut m = Manager::default();
        m.write_ram(crate::ram_map::IN_AREA, 0x13); // Tourian
        m.set_mb_hi(0x03); // 名表号 3（母脑房在 $65 页）
        m
    }

    /// MB 帧 id 必须在 Tourian 帧指针表里存在（`Pulsations0..3`=19..22、`Eyes`=23）。
    #[test]
    fn anim_frames_exist_in_tourian_frame_table() {
        for id in super::MB_FRAMES {
            assert!(
                game_data::assets::enemy_frame_off(3, id as usize).is_some(),
                "Tourian 帧 id {id} 必须存在"
            );
        }
    }

    /// 状态 0 / $0A 是空径（`beq @RTS` 与表项 Exit）。
    #[test]
    fn status_zero_and_dead_are_no_ops() {
        let mut m = mgr();
        m.set_mb_status(0);
        super::status_handler(&mut m);
        assert_eq!(m.mb_status(), 0);
        m.set_mb_status(0x0A);
        super::status_handler(&mut m);
        assert_eq!(m.mb_status(), 0x0A);
    }

    /// `MotherBrain_Idle_CollideWithSamus`：同页 + x≥$48 且 Δ<$2F + |y-$80|<$20 ⇒ 扣 20 点。
    #[test]
    fn idle_collides_and_drains_health() {
        let mut m = mgr();
        m.set_mb_status(0x01);
        m.samus().set_hi(0x03);
        m.samus().set_x(0x50);
        m.samus().set_y(0x80);
        m.samus().set_health(0x00C0);
        super::status_handler(&mut m);
        assert_eq!(m.health_change() & 0xFFFF, 0x0200, "HealthChange=$0200");
        assert_eq!(m.samus().is_hit(), 0x38, "Samus.isHit=$38");
        assert!(m.samus().health() < 0x00C0, "血被扣");
        // 名表号不同 ⇒ 不碰
        let mut m2 = mgr();
        m2.set_mb_status(0x01);
        m2.samus().set_hi(0x04);
        m2.samus().set_x(0x50);
        m2.samus().set_y(0x80);
        m2.samus().set_health(0x00C0);
        super::status_handler(&mut m2);
        assert_eq!(m2.samus().health(), 0x00C0, "跨页不碰撞");
    }

    /// `HandleBeingHit`：未满 32 击 ⇒ 状态 2/闪 16；满 32 ⇒ 状态 3/闪 128 + 清全部瓦片爆。
    #[test]
    fn being_hit_switches_state_and_clears_tile_blasts() {
        let mut m = mgr();
        m.set_mb_status(0x01);
        m.set_mb_is_hit(1);
        m.set_mb_qty_hits(5);
        m.set_tb_routine(0, 1);
        super::status_handler(&mut m);
        assert_eq!(m.mb_qty_hits(), 6);
        assert_eq!(m.mb_status(), 0x02);
        assert_eq!(m.mb_flash_delay(), 0x10);

        let mut m2 = mgr();
        m2.set_mb_status(0x01);
        m2.set_mb_is_hit(1);
        m2.set_mb_qty_hits(0x1F);
        m2.set_tb_routine(3, 1);
        super::status_handler(&mut m2);
        assert_eq!(m2.mb_qty_hits(), 0x20);
        assert_eq!(m2.mb_status(), 0x03);
        assert_eq!(m2.mb_flash_delay(), 0x80);
        assert_eq!(m2.tb_routine(3), 0, "13 个瓦片爆全清");
    }

    /// `Disappear_Disintegrate`：批次 id=1、指令 0 ⇒ 清 `$6144` 的 2×2（ptr.lo=$44）。
    #[test]
    fn disintegrate_writes_batch_tile_pointer() {
        let mut m = mgr();
        m.set_mb_qty_hits(1); // DeathStringID=1
        m.set_mb_brain_delay(0); // DeathInstrID=0
        super::disintegrate(&mut m);
        assert_eq!(m.tb_ptr(0) & 0xFF, 0x44, "lo = $6144-$6100");
        assert_eq!((m.tb_ptr(0) >> 8) as u8, 0x6D, "hi = (3<<2)|$61 = $6D");
        assert_eq!(m.tb_anim_frame(0), 0x00, "_id_VRAMString00");
    }

    /// `SetTimeBomb`（状态 6）：造门成功 ⇒ 9999 帧倒计时 + 逃逸计时器敌人启用。
    #[test]
    fn set_time_bomb_starts_end_timer() {
        let mut m = mgr();
        m.set_mb_status(0x06);
        m.set_map_pos(0x01, 0x02);
        super::status_handler(&mut m);
        assert_eq!(m.end_timer(), (0x99, 0x99));
        assert_eq!(m.end_timer_enemy_enabled(), 1);
        assert_eq!(m.end_timer_enemy_hi(), 0x03);
        assert_eq!(m.mb_status(), 0, "母脑状态回 0（不在房内）");
        // 门槽 = DOOR_SLOTS[((1+2)<<1|1)&3] = DOOR_SLOTS[3] = 槽 9
        assert_eq!(m.object(9).status(), 0x03, "蓝门开门态");
        assert_eq!(m.object(9).door_type(), 0x01);
        assert_eq!(m.object(9).y(), 0x68);
        assert_eq!(m.object(9).x(), 0x10);
    }

    /// `TimeBombMessage`：counter 到 8 ⇒ 状态 +1（→6）；负值时先推到 0。
    #[test]
    fn time_bomb_message_advances_counter() {
        let mut m = mgr();
        m.set_mb_status(0x05);
        m.set_mb_qty_hits(0x83); // -$7D
        super::status_handler(&mut m);
        assert_eq!(m.mb_qty_hits(), 0x84, "负值每帧 +1");
        let mut m2 = mgr();
        m2.set_mb_status(0x08);
        m2.set_mb_qty_hits(8);
        super::status_handler(&mut m2);
        assert_eq!(m2.mb_status(), 0x09, "状态 8 之后接 9");
    }
}
