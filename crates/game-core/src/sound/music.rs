//! `game-core::sound::music` —— 音乐引擎（批量改写·第十簇 sound::music（六步流程））。
//!
//! 【step1 定域取证 spec】asm 内容锚（refs/m1disasm/SRC/sound_engine.asm——A 级引证）：
//! - `MusicRoutineTbl: ; $BC26`（:2083）——8 条目 .word 派发表（歌曲→GotoMusicNNInit）
//! - `RunMusicLoopRoutine: ; $BC36`（:2095）——A←MusicLoopFlags（$065D）（9th SFX cycle）
//! - `RunMusicInitRoutine: ; $BC3D`（:2102）——A←MusicInitFlags（$0685）（10th cycle）→@Common
//! - `ContinueMusic: ; $BC4B`（:2118）——A←MusicContFlags（$068D）（11th/last cycle）；
//!   beq 出口（无播放）；jmp UpdateAllMusicChannels（$BAA5）
//! - `FindMusicInitIndex: ; $BC53`（:2128）——MusicInitIndex（$065E）←$FF；
//!   CurrentSoundFlags（$064D）==0 ⇒ RTS；@loop: inc MusicInitIndex/asl/bcc @loop
//!   （MSB 首位扫描——bit7→0 … bit0→7）
//!
//! MusicInitIndex 语义（:2137 注释）：00=Ridley 01=Tourian 02=Item room 03=Kraid
//! 04=Norfair 05=Escape 06=Mother brain 07=Brinstar 08=Fade in 09=Power up
//! 0A=End game 0B=Intro。
//!
//! 【step2 域结构】MusicRoutine（$BC26 表值域）+MusicInitIndex 扫描面；
//! 【step3 memmap 适配】RAM 面=constants_ram.asm:1069/1081/1082/1129/1141（$064D/$065D/
//!   $065E/$0685/$068D）；【step4 shim 写集】MusicInitIndex→$065E 写回；
//! 【step5 测试】flags 全域 0..=255 穷举等价（下 `mod tests`）；
//! 【step6 registry 登记】l2ports.rs `s4_cluster_music_init_index`。

/// $BC26 MusicRoutineTbl 值域（.word 目标=GotoMusicNNInit 派生面）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MusicRoutine {
    /// GotoMusic04Init（$BC26[0] Ridley area music）
    Music04Init,
    /// GotoMusic00Init（$BC26[1/2/3/6] Tourian/Item room/Kraid/Mother brain）
    Music00Init,
    /// GotoMusic03Init（$BC26[4/7] Norfair/Brinstar）
    Music03Init,
    /// GotoMusic02Init（$BC26[5] Escape）
    Music02Init,
}

/// $BC26 MusicRoutineTbl（8 条目逐钉——索引→派生面）。
pub fn music_routine_tbl(index: u8) -> Option<MusicRoutine> {
    match index {
        0 => Some(MusicRoutine::Music04Init), // Ridley area music
        1 | 2 | 3 | 6 => Some(MusicRoutine::Music00Init), // Tourian/Item room/Kraid/Mother brain
        4 | 7 => Some(MusicRoutine::Music03Init), // Norfair/Brinstar
        5 => Some(MusicRoutine::Music02Init), // Escape
        _ => None,
    }
}

/// FindMusicInitIndex（$BC53）：MSB 首位扫描（bit7→0 … bit0→7）；flags==0 ⇒ None
/// （MusicInitIndex 保持 $FF 出口——asm `beq @RTS`）。
pub fn find_music_init_index(current_sound_flags: u8) -> Option<u8> {
    if current_sound_flags == 0 {
        return None; // @RTS（MusicInitIndex=$FF 语义面）
    }
    let mut index: u8 = 0xFF;
    let mut acc = current_sound_flags;
    loop {
        index = index.wrapping_add(1); // inc MusicInitIndex
        let carry = acc & 0x80 != 0; // asl → carry=bit7
        acc <<= 1;
        if carry {
            return Some(index); // bcc @loop 不转 ⇒ rts
        }
    }
}

/// ContinueMusic 门（$BC4B）：MusicContFlags==0 ⇒ 无播放（beq 出口）；else 更新信号。
pub fn continue_music_gate(music_cont_flags: u8) -> bool {
    music_cont_flags != 0
}

/// shim 写集适配（step4）：MusicInitIndex→$065E 写回（None=$FF 保真）。
pub fn music_init_index_writes(index: Option<u8>) -> Vec<(u16, u8)> {
    vec![(0x065E, index.unwrap_or(0xFF))]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 【step5 穷举等价证明】flags 全域 0..=255——MSB 首位扫描逐钉（对照独立参考径）。
    #[test]
    fn exhaustive_find_music_init_index() {
        for f in 0u8..=255 {
            // 独立参考径（证伪立场）：最高位首置位序号（bit7→0）
            let reference = (0..8).find(|b| f & (0x80 >> b) != 0);
            assert_eq!(find_music_init_index(f), reference, "flags={:#010b}", f);
            // 语义恒等：Some(idx) ⇒ bit(7-idx) 置位
            if let Some(i) = find_music_init_index(f) {
                assert_ne!(f & (0x80 >> i), 0);
            }
        }
    }

    /// $BC26 表 8 条目逐钉+域外 None。
    #[test]
    fn music_routine_tbl_entries() {
        assert_eq!(music_routine_tbl(0), Some(MusicRoutine::Music04Init));
        assert_eq!(music_routine_tbl(1), Some(MusicRoutine::Music00Init));
        assert_eq!(music_routine_tbl(2), Some(MusicRoutine::Music00Init));
        assert_eq!(music_routine_tbl(3), Some(MusicRoutine::Music00Init));
        assert_eq!(music_routine_tbl(4), Some(MusicRoutine::Music03Init));
        assert_eq!(music_routine_tbl(5), Some(MusicRoutine::Music02Init));
        assert_eq!(music_routine_tbl(6), Some(MusicRoutine::Music00Init));
        assert_eq!(music_routine_tbl(7), Some(MusicRoutine::Music03Init));
        assert_eq!(music_routine_tbl(8), None);
    }

    /// ContinueMusic 门+写集（$065E/$FF 保真）。
    #[test]
    fn continue_gate_and_writes() {
        assert!(!continue_music_gate(0));
        assert!(continue_music_gate(1));
        assert_eq!(music_init_index_writes(Some(3)), vec![(0x065E, 3)]);
        assert_eq!(music_init_index_writes(None), vec![(0x065E, 0xFF)]);
    }
}
