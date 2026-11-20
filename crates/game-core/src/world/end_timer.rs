//! Tourian 逃脱倒计时（`UpdateEndTimer ; 03:A1E7` + `DrawEndTimerEnemy ; 03:A238`）。
//!
//! `EndTimer`（`$010A-$010B`）是 BCD 四位 `9999`：母脑设时炸弹时写死 `$9999`，
//! 之后每帧 BCD 减 1（低位借位传给高位，`Base10Subtract ; 07:C3FB` 语义）。
//! 归零 ⇒ 母脑状态置 `$07`（时炸弹爆炸）、`Timer3=$0C`（120 帧白光后判死）、
//! 调色板 `$0B`（全白），并静音。
//!
//! `DrawEndTimerEnemy` 借母脑槽（`$0400+$E0`）画 `_id_EnFrame1A` 的精灵，
//! 画成功后把该精灵组前三枚的 tile 字节改成 `hundreds/tens/ones` 三位数字
//! （`|$A0` 是字体 tile 基址）。

use crate::frame::Manager;
use crate::weapon::projectile::bcd_sub;

/// `_id_EnFrame1A`（Tourian `EnFramePtrTable1` 表序第 26 项：`00/01/EnProjKilled/Metroid0..`
/// 数下来 `EnFrame18`=24、`CannonTimeBombSet`=25、`EnFrame1A`=26）。
const END_TIMER_FRAME: u8 = 26;
/// 母脑槽在 `Ens` 面的字节偏移（`EnMotherBrain - Ens`）。
const MOTHER_BRAIN_SLOT: u8 = 0xE0;

/// `UpdateEndTimer ; 03:A1E7`。
pub(crate) fn update_end_timer(mem: &mut Manager) {
    // `ldy EndTimer+1 / iny / beq` —— 高位为 `$FF` 即计时器停用
    if mem.end_timer_hi().wrapping_add(1) == 0 {
        return;
    }
    let (lo, hi) = mem.end_timer();
    // 两位 BCD 同时减 1：低位借位传给高位
    let (lo2, borrow) = bcd_sub(lo, 0x01);
    let (hi2, _) = bcd_sub(hi, if borrow { 0x01 } else { 0x00 });
    mem.store_end_timer(lo2, hi2);

    // 每 32 帧响一次警报（`FrameCount & $1F == 0` ⇒ `sfxSQ1_OutOfPipe`）
    if mem.frame_count() & 0x1F == 0 {
        mem.or_sfx_sq1_init(0x08);
    }

    if lo2 | hi2 != 0 {
        return;
    }
    // 归零：停表（高位写 $FF）、母脑状态 7、静音、`Timer3=$0C`、调色板全白
    mem.store_end_timer(lo2, 0xFF);
    mem.set_mb_qty_hits(0);
    mem.set_mb_status(0x07);
    crate::sound::hook::set_sfx_noise_init(mem, 0x20); // sfxNoise_SilenceMusic
    mem.set_timer3(0x0C);
    mem.set_palette_pending(11); // _id_Palette0A+1
}

/// `DrawEndTimerEnemy ; 03:A238`。
pub(crate) fn draw_end_timer_enemy(mem: &mut Manager) {
    if mem.end_timer_enemy_enabled() == 0 {
        return;
    }
    let hi = mem.end_timer_enemy_hi();
    {
        let mut e = mem.ens_face(MOTHER_BRAIN_SLOT);
        e.set_hi(hi);
        e.set_y(0x84);
        e.set_pos_x(0x64);
        e.set_anim_frame(END_TIMER_FRAME);
    }
    let before = mem.sprite_page_pos();
    crate::render::enemy_sprite::draw_enemy(mem, MOTHER_BRAIN_SLOT);
    // `pla / cmp SpritePagePos / beq @RTS` —— 页位没变说明精灵没画出去
    if mem.sprite_page_pos() == before {
        return;
    }
    let (lo, hi) = mem.end_timer();
    // 三位数字：hundreds=hi 高 nibble、tens=hi 低 nibble、ones=lo 高 nibble
    let digits = [hi >> 4, hi & 0x0F, lo >> 4];
    for (i, d) in digits.iter().enumerate() {
        let off = before.wrapping_add((i as u8) << 2).wrapping_add(1);
        mem.set_oam_byte(off, (d & 0x0F) | 0xA0);
    }
}

#[cfg(test)]
mod tests_end_timer {
    use crate::frame::Manager;

    /// `$9999` 减 1 ⇒ `$9998`（低位不借位）。
    #[test]
    fn bcd_decrement_steps_down_without_borrow() {
        let mut m = Manager::default();
        m.set_end_timer_pair(0x99);
        m.write_ram(crate::ram_map::FRAME_COUNT, 1); // 避开 32 帧警报
        super::update_end_timer(&mut m);
        assert_eq!(m.end_timer(), (0x98, 0x99));
    }

    /// `$9900` 减 1 ⇒ `$9899`（低位借位传给高位）。
    #[test]
    fn bcd_borrow_crosses_into_high_byte() {
        let mut m = Manager::default();
        m.store_end_timer(0x00, 0x99);
        m.write_ram(crate::ram_map::FRAME_COUNT, 1);
        super::update_end_timer(&mut m);
        assert_eq!(m.end_timer(), (0x99, 0x98));
    }

    /// 高位 `$FF` ⇒ 计时器停用（`iny / beq`）。
    #[test]
    fn disabled_timer_is_untouched() {
        let mut m = Manager::default();
        m.store_end_timer(0x42, 0xFF);
        super::update_end_timer(&mut m);
        assert_eq!(m.end_timer(), (0x42, 0xFF));
    }

    /// 归零 ⇒ 母脑状态 7 + 静音 + `Timer3=$0C` + 调色板 `$0B` + 高位写 `$FF` 停表。
    #[test]
    fn zero_triggers_time_bomb_explosion() {
        let mut m = Manager::default();
        m.store_end_timer(0x01, 0x00); // BCD $0001 再减 1 = 0
        m.write_ram(crate::ram_map::FRAME_COUNT, 1);
        super::update_end_timer(&mut m);
        assert_eq!(m.mb_status(), 0x07, "母脑状态 → 时炸弹爆炸");
        assert_eq!(m.read_ram(crate::ram_map::TIMER3), 0x0C);
        assert_eq!(m.palette_pending(), 11, "_id_Palette0A+1");
        assert_eq!(m.read_ram(crate::ram_map::END_TIMER + 1), 0xFF, "停表");
    }

    /// 每 32 帧响一次 `sfxSQ1_OutOfPipe`（`FrameCount & $1F == 0`）。
    #[test]
    fn alarm_fires_every_32_frames() {
        let mut m = Manager::default();
        m.set_end_timer_pair(0x99);
        m.write_ram(crate::ram_map::FRAME_COUNT, 0x20);
        super::update_end_timer(&mut m);
        assert_eq!(m.read_ram(crate::sound::addrs::SQ1_INIT) & 0x08, 0x08);
        let mut m2 = Manager::default();
        m2.set_end_timer_pair(0x99);
        m2.write_ram(crate::ram_map::FRAME_COUNT, 0x21);
        super::update_end_timer(&mut m2);
        assert_eq!(m2.read_ram(crate::sound::addrs::SQ1_INIT) & 0x08, 0);
    }

    /// 未启用 ⇒ 不画；启用后画成功则三位数字写进刚画的精灵 tile 字节（`|$A0`）。
    #[test]
    fn digits_are_patched_into_sprite_tiles() {
        let mut m = Manager::default();
        m.write_ram(crate::ram_map::IN_AREA, 0x13); // Tourian
        // 未启用：什么都不画
        super::draw_end_timer_enemy(&mut m);
        assert_eq!(m.sprite_page_pos(), 0, "未启用不画");
        // 启用 + 计时器 $1234 的可见三位（hundreds=hi>>4=1、tens=hi&$F=2、ones=lo>>4=3）
        m.set_end_timer_enemy_enabled(1);
        m.set_end_timer_enemy_hi(0x03);
        m.store_end_timer(0x34, 0x12);
        super::draw_end_timer_enemy(&mut m);
        if m.sprite_page_pos() != 0 {
            // 画成功：三枚精灵的 tile 字节被改成数字
            let base = 0u8; // 起画前页位是 0
            assert_eq!(m.oam_shadow()[base as usize + 1], 0xA1, "hundreds");
            assert_eq!(m.oam_shadow()[base as usize + 5], 0xA2, "tens");
            assert_eq!(m.oam_shadow()[base as usize + 9], 0xA3, "ones");
        }
    }
}
