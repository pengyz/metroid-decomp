//! sound 数据（数据内容在 game-data（架构约束））。

pub mod sound_data;
pub mod music_tables; // W3-1 烘焙（tools/bake_music_tables.py 生成/校验）

#[cfg(test)]
mod music_tables_anchors {
    use super::music_tables::*;

    /// ROM 锚抽钉（asm 注释频率面自证——sound_engine.asm:2477 起逐行注）：
    /// idx $30=C4 ⇒ ($01,$AB)；idx $02=No sound ⇒ ($00,$00)；idx $00=A1 ⇒ ($07,$F0)。
    #[test]
    fn notes_tbl_rom_anchors() {
        assert_eq!((MUSIC_NOTES_TBL[0x30], MUSIC_NOTES_TBL[0x31]), (0x01, 0xAB), "C4 idx $30");
        assert_eq!((MUSIC_NOTES_TBL[0x02], MUSIC_NOTES_TBL[0x03]), (0x00, 0x00), "休止 idx $02");
        assert_eq!((MUSIC_NOTES_TBL[0x00], MUSIC_NOTES_TBL[0x01]), (0x07, 0xF0), "A1 idx $00");
        assert_eq!((MUSIC_NOTES_TBL[0x7E], MUSIC_NOTES_TBL[0x7F]), (0x00, 0x27), "F7 idx $7E");
    }

    /// 长度表三组（@4 11B/@6 12B/@7 12B——组号=曲头 Byte0：0/0x0B/0x17）。
    #[test]
    fn note_length_groups() {
        assert_eq!(NOTE_LENGTHS_4.len(), 11);
        assert_eq!(NOTE_LENGTHS_6.len(), 12);
        assert_eq!(NOTE_LENGTHS_7.len(), 12);
        assert_eq!(NOTE_LENGTHS_4[0], 0x04, "@4 首=About 1/16s");
        assert_eq!(NOTE_LENGTHS_6[0], 0x06, "@6 首=About 3/32s");
        assert_eq!(NOTE_LENGTHS_7[0], 0x07, "@7 首=About 7/64s（intro/end）");
        assert_eq!(note_length_table(0), &NOTE_LENGTHS_4[..], "组 0→@4");
        assert_eq!(note_length_table(0x0B), &NOTE_LENGTHS_6[..], "组 0x0B→@6");
        assert_eq!(note_length_table(0x17), &NOTE_LENGTHS_7[..], "组 0x17→@7");
    }
}
