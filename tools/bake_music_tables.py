#!/usr/bin/env python3
"""bake_music_tables —— 音乐数据表 ROM 烘焙（W3-1；D 面纪律：禁手抄数值）。

源（sound_engine.asm 锚）：
- MusicNotesTbl $BE77（64 音符×2B period 对——高/低）→ prg[0x3E77]
- NoteLengthsTbl $BEF7 三组（@4 11B / @6 12B / @7 intro-end 组）→ prg[0x3EF7]
（sound 域地址→文件偏移：addr-$8000——B31B↔0x331B 自证）

产出：crates/game-data/src/sound/music_tables.rs（含 ROM 锚注）。
--check：重算对比生成件（防漂移）。
"""
from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
ROM = ROOT / "rom" / "Metroid (USA).nes"
OUT = ROOT / "crates" / "game-data" / "src" / "sound" / "music_tables.rs"

NOTES_OFF = 0x3E77  # MusicNotesTbl $BE77
NOTES_COUNT = 64
LENGTHS_OFF = 0x3EF7  # NoteLengthsTbl $BEF7（@4=+0 11B / @6=+11 12B / @7=+23 剩）
LEN4, LEN6 = 11, 12


def load() -> bytes:
    data = ROM.read_bytes()
    return data[0x10:]  # 去 iNES 头


def gen(prg: bytes) -> str:
    notes = prg[NOTES_OFF:NOTES_OFF + NOTES_COUNT * 2]
    l4 = prg[LENGTHS_OFF:LENGTHS_OFF + LEN4]
    l6 = prg[LENGTHS_OFF + LEN4:LENGTHS_OFF + LEN4 + LEN6]
    # @7 组：探至组界（intro/end 组——取 12B（@7 $BF0E 起；长于 @6 1B 面随证））
    l7 = prg[LENGTHS_OFF + LEN4 + LEN6:LENGTHS_OFF + LEN4 + LEN6 + 12]

    def hexrow(bs: bytes, indent: str = "    ") -> str:
        return "\n".join(indent + " ".join(f"0x{b:02X}," for b in bs[i:i + 8])
                         for i in range(0, len(bs), 8))

    return f"""//! 音乐数据表（W3-1 烘焙——tools/bake_music_tables.py 生成；**禁手抄**）。
//!
//! ROM 锚（sound_engine.asm）：`MusicNotesTbl: ; $BE77`（64 音符×2B：高/低 period 对——
//! 低字节 0=休止）/ `NoteLengthsTbl: ; $BEF7`（三组：@4 11B（PowerUp/Kraid）/
//! @6 12B（区域曲族）/ @7 12B（intro/end）——组偏移=曲头 Byte0（$00/$0B/$17））。
//! 文件偏移口径=sound 域 addr-$8000（$B31B↔prg[0x331B] 自证）。

/// MusicNotesTbl（$BE77——64×2B 平铺；索引=音符字节）。
pub const MUSIC_NOTES_TBL: [u8; {NOTES_COUNT * 2}] = [
{hexrow(notes)}
];

/// NoteLengthsTbl@4（$BEF7——11B：PowerUp/Kraid 组）。
pub const NOTE_LENGTHS_4: [u8; {LEN4}] = [
{hexrow(l4)}
];

/// NoteLengthsTbl@6（$BF02——12B：区域曲族组）。
pub const NOTE_LENGTHS_6: [u8; {LEN6}] = [
{hexrow(l6)}
];

/// NoteLengthsTbl@7（$BF0E——12B：intro/end 组）。
pub const NOTE_LENGTHS_7: [u8; 12] = [
{hexrow(l7)}
];

/// 按**全表下标**取长度帧数（asm 语义：`Y=(A&$0F)+NoteLengthTblOffset` 直接索引全表
/// ——组偏移 0/0x0B/0x17 即组首下标（0/11/23），非分组切片）。
pub fn note_length_at(index: u8) -> u8 {{
    const ALL: [u8; {len(l4) + len(l6) + len(l7)}] = [
{hexrow(l4 + l6 + l7)}
    ];
    ALL.get(index as usize).copied().unwrap_or(0)
}}
"""


def main() -> int:
    prg = load()
    text = gen(prg)
    if "--check" in sys.argv:
        cur = OUT.read_text(encoding="utf-8") if OUT.is_file() else ""
        if cur == text:
            print("[PASS] music_tables 与 ROM 烘焙一致（--check 无漂移）")
            return 0
        print("[FAIL] music_tables 与 ROM 烘焙不一致（重跑生成）")
        return 1
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(text, encoding="utf-8", newline="\n")
    print(f"[OK] music_tables 生成 → {OUT.relative_to(ROOT)}"
          f"（notes {NOTES_COUNT * 2}B + lengths {LEN4}/{LEN6}/12B）")
    return 0


if __name__ == "__main__":
    sys.exit(main())
