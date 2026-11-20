//! 结局画面的精灵装载件（`LoadEndSamusSprites ; 00:9C9A` / `LoadWaveSprites ; 00:9C7F`）。
//!
//! 两者都直接写 OAM 影子页（`SpriteRAM` = `$0200`），被段 1–4 共用：
//!
//! ```text
//! LoadWaveSprites ; 00:9C7F
//!   ldx #SpriteRAM.8-SpriteRAM      ; $20
//!   ldy #$00
//! @loop:
//!   lda (WaveSpritePointer),y
//!   sta SpriteRAM,x
//!   inx / iny
//!   cpy WaveSpriteCounter
//!   bne @loop
//!
//! LoadEndSamusSprites ; 00:9C9A
//!   ldx #SpriteRAM.12-SpriteRAM     ; $30
//!   ldy #$00
//! @loop:
//!   lda (EndSamusSpritePointer),y   ; 源 = EndSamusAddrTbl[SpritePointerIndex] + y
//!   bmi @hflip                      ; bit7 置 ⇒ 精灵水平翻转
//!   sta SpriteRAM,x
//!   jmp @next
//! @hflip:
//!   and #$7F
//!   sta SpriteRAM,x
//!   lda SpriteRAM+2,x               ; 属性字节
//!   eor #OAMDATA_HFLIP              ; $40
//!   sta SpriteRAM+2,x
//! @next:
//!   inx / iny
//!   cpy SpriteByteCounter           ; $36
//!   bne @loop
//! ```
//!
//! 记录里 bit7 的那位出现在 **tile 字节**（每 4 字节的第 2 个），asm 用 `bmi` 在流水线上就判断了，
//! 所以本移植也逐字节判——不能先按 4 字节分组再处理，否则 `SpriteByteCounter` 非 4 倍数时会多写。

use crate::state::Manager;

/// `SpriteRAM.8 - SpriteRAM`（波表装载起始偏移）。
const WAVE_BASE: u8 = 8 * 4;
/// `SpriteRAM.12 - SpriteRAM`（结局 Samus 装载起始偏移）。
const SAMUS_BASE: u8 = 12 * 4;
/// `OAMDATA_HFLIP`。
const HFLIP: u8 = 0x40;

/// `LoadWaveSprites ; 00:9C7F`：把 `end_wave()[table*8 ..]` 的 `count` 字节拷进 OAM。
pub(crate) fn load_wave_sprites(mem: &mut Manager, src_off: u8, count: u8) {
    let src = game_data::assets::end_wave();
    let off = src_off as usize;
    for i in 0..count as usize {
        let Some(&v) = src.get(off + i) else { break };
        mem.set_oam_byte(WAVE_BASE.wrapping_add(i as u8), v);
    }
}

/// `LoadEndSamusSprites ; 00:9C9A`：把 `end_samus(idx)` 的前 `count` 字节写进 OAM（含 bit7 翻转）。
pub(crate) fn load_end_samus_sprites(mem: &mut Manager, idx: u8, count: u8) {
    let Some(src) = game_data::assets::end_samus(idx) else { return };
    for i in 0..count as usize {
        let Some(&v) = src.get(i) else { break };
        let dst = SAMUS_BASE.wrapping_add(i as u8);
        if v & 0x80 != 0 {
            mem.set_oam_byte(dst, v & 0x7F);
            // 属性字节在 tile 之后一格（同一 4 字节记录内）。
            let attr = mem.oam_shadow()[dst.wrapping_add(2) as usize];
            mem.set_oam_byte(dst.wrapping_add(2), attr ^ HFLIP);
        } else {
            mem.set_oam_byte(dst, v);
        }
    }
}

#[cfg(test)]
mod tests_ending_sprites {
    use super::*;

    fn mgr() -> Manager {
        Manager::default()
    }

    #[test]
    fn wave_loader_copies_from_src_offset_zero() {
        let mut m = mgr();
        load_wave_sprites(&mut m, 0, 8);
        let src = game_data::assets::end_wave();
        for i in 0..8 {
            assert_eq!(m.oam_shadow()[WAVE_BASE as usize + i], src[i]);
        }
    }

    #[test]
    fn wave_loader_honours_byte_offset() {
        // `WaveSpritePointer` 是**字节偏移**（0/4 选表，再 +2 换手型），不是表序号：
        // ROM $9D1A/$9D2A/$9D3A/$9D4A 四表各 16 B，故表 1 从 +0x10 起。
        let mut m = mgr();
        load_wave_sprites(&mut m, 0x10, 8);
        let src = game_data::assets::end_wave();
        for i in 0..8 {
            assert_eq!(m.oam_shadow()[WAVE_BASE as usize + i], src[0x10 + i]);
        }
    }

    #[test]
    fn samus_loader_writes_plain_bytes() {
        let mut m = mgr();
        load_end_samus_sprites(&mut m, 0, 4);
        let src = game_data::assets::end_samus(0).expect("frame 0");
        for i in 0..4 {
            let expect = if src[i] & 0x80 != 0 { src[i] & 0x7F } else { src[i] };
            assert_eq!(m.oam_shadow()[SAMUS_BASE as usize + i], expect);
        }
    }

    #[test]
    fn samus_loader_flips_attribute_on_high_tile_bit() {
        let mut m = mgr();
        let src = game_data::assets::end_samus(0).expect("frame 0");
        // 找一个 bit7 置位的字节（原版用它表达"这格水平翻转"）。
        let Some(pos) = (0..src.len()).find(|&i| src[i] & 0x80 != 0) else {
            // 该帧没有翻转位时不制造假断言。
            return;
        };
        let before = m.oam_shadow()[SAMUS_BASE as usize + pos + 2];
        load_end_samus_sprites(&mut m, 0, (pos + 1) as u8);
        let after = m.oam_shadow()[SAMUS_BASE as usize + pos + 2];
        assert_eq!(after, before ^ HFLIP, "bit7 置位必须翻转属性字节的 HFLIP 位");
    }

    #[test]
    fn samus_loader_stops_at_frame_len() {
        let mut m = mgr();
        let len = game_data::assets::end_samus(0).expect("frame 0").len();
        // count 超过帧长时不得 panic（真实 asm 由 SpriteByteCounter 保证不越界），
        // 且写入结果必须与「正好 count=帧长」逐字节一致 —— 越界源字节不产生任何写入。
        let mut exact = Manager::default();
        load_end_samus_sprites(&mut exact, 0, len as u8);
        load_end_samus_sprites(&mut m, 0, (len + 20) as u8);
        assert_eq!(m.oam_shadow()[..], exact.oam_shadow()[..]);
    }

    #[test]
    fn samus_loader_ignores_out_of_range_index() {
        let mut m = mgr();
        load_end_samus_sprites(&mut m, 6, 8);
        // OAM 影子初值为 0：越界索引不得写任何字节。
        for i in 0..8 {
            assert_eq!(m.oam_shadow()[SAMUS_BASE as usize + i], 0x00);
        }
    }
}
