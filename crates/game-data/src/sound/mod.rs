//! sound 数据（数据内容在 game-data（架构约束））。

// music_tables 已迁资产包（`assets::{music_notes,note_lengths_4/6/7}`）
// music_streams 已迁资产包（`assets::{song_stream,stream_by_base,volume_envelope*,sfx_data}`）

#[cfg(test)]
mod music_tables_anchors {
    use crate::assets::*;

    /// ROM 锚抽钉（asm 注释频率面自证——sound_engine.asm:2477 起逐行注）：
    /// idx $30=C4 ⇒ ($01,$AB)；idx $02=No sound ⇒ ($00,$00)；idx $00=A1 ⇒ ($07,$F0)。
    #[test]
    fn notes_tbl_rom_anchors() {
        assert_eq!((music_notes()[0x30], music_notes()[0x31]), (0x01, 0xAB), "C4 idx $30");
        assert_eq!((music_notes()[0x02], music_notes()[0x03]), (0x00, 0x00), "休止 idx $02");
        assert_eq!((music_notes()[0x00], music_notes()[0x01]), (0x07, 0xF0), "A1 idx $00");
        assert_eq!((music_notes()[0x7E], music_notes()[0x7F]), (0x00, 0x27), "F7 idx $7E");
    }

    /// 长度表三组（@4 11B/@6 12B/@7 12B——组首全表下标=曲头 Byte0：0/0x0B/0x17）。
    #[test]
    fn note_length_groups() {
        assert_eq!(note_lengths_4().len(), 11);
        assert_eq!(note_lengths_6().len(), 12);
        assert_eq!(note_lengths_7().len(), 12);
        assert_eq!(note_lengths_4()[0], 0x04, "@4 首=About 1/16s");
        assert_eq!(note_lengths_6()[0], 0x06, "@6 首=About 3/32s");
        assert_eq!(note_lengths_7()[0], 0x07, "@7 首=About 7/64s（intro/end）");
        // 全表下标语义（asm `adc NoteLengthTblOffset` 直索全表）
        assert_eq!(note_length_at(0), 0x04);
        assert_eq!(note_length_at(0x0B), 0x06, "组首 11=@6 首");
        assert_eq!(note_length_at(0x17), 0x07, "组首 23=@7 首");
        assert_eq!(note_length_at(0x0D), 0x18, "idx 13=@6[2]（长度字节 0xB2+offset 0x0B）");
        assert_eq!(note_length_at(0xFF), 0x00, "越界=0（fail-safe 面）");
    }
}

#[cfg(test)]
mod music_streams_anchors {
    use crate::assets;

    /// W3-2.5-1 烘焙锚（迁包后口径不变）：流尾 SongEnd(0x00) 含尾；包络 1=NTSC 变体 asm 注逐字。
    #[test]
    fn streams_and_envelopes_rom_anchors() {
        let mut n = 0;
        for song in 0..12 {
            for ch in 0..4 {
                if let Some(s) = assets::song_stream(song, ch) {
                    assert_eq!(*s.last().unwrap(), 0x00, "流尾=SongEnd 含尾");
                    n += 1;
                }
            }
        }
        assert!(n >= 40, "流件数（41 实烘）: {n}");
        // 包络 1（NTSC——sound_engine.asm:2481 逐字注）
        assert_eq!(
            assets::volume_envelope(0),
            &[0x01, 0x02, 0x02, 0x03, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0xFF]
        );
        // 包络 2/3 尾=FF；4/5 尾=F0（killVolume 停驻）
        assert_eq!(assets::volume_envelope(1).last(), Some(&0xFF));
        assert_eq!(assets::volume_envelope(3).last(), Some(&0xF0));
        // Intro（0x0B）四通道面：SQ1 非空、Noise 空基址面（曲头 4 字自证）
        assert!(assets::song_stream(0x0B, 0).is_some(), "Intro SQ1 流在位");
        // 基址反查与曲号查表必须同源
        let b0 = u16::from_le_bytes([assets::stream_bases()[0], assets::stream_bases()[1]]);
        assert_eq!(assets::stream_by_base(b0), assets::song_stream(0, 0));
        // 包络指针表 5 word（$BCB0 原值）
        assert_eq!(assets::volume_envelope_ptrs().len(), 10);
        assert_eq!(assets::sfx_data().len(), 16);
    }
}
