#!/usr/bin/env python3
"""bake_music_streams —— 乐数据流+包络表 ROM 烘焙（W3-2.5；D 面禁手抄）。

源锚（sound_engine.asm）：
- SongHeaderOffsetTable $BBFA（12B 组偏移）→ SongHeaders $BD31（12 头×13B）
  头[5..13]=4 通道基址字（SQ1/SQ2/Tri/Noise——0=空通道）
- 流体=基址起至 0x00（SongEnd）**含尾**（指令流语义：0x00=曲终字节）
- VolumeEnvelopePtrTable $BCB0（5 word）→ VolumeEnvelope1-5（至 $FF/$F0 终端字节含尾）

偏移口径=sound 域 addr-$8000（与 bake_music_tables.py 同源自证）。
产出：crates/game-data/src/sound/music_streams.rs。--check 防漂移。
"""
from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
ROM = ROOT / "rom" / "Metroid (USA).nes"
OUT = ROOT / "crates" / "game-data" / "src" / "sound" / "music_streams.rs"

HDR_OFF_TBL = 0x3BFA  # SongHeaderOffsetTable $BBFA
SONG_HEADERS = 0x3D31  # SongHeaders $BD31
ENV_PTRS = 0x3CB0  # VolumeEnvelopePtrTable $BCB0
N_SONGS, N_ENVS = 12, 5


def load() -> bytes:
    return ROM.read_bytes()[0x10:]


def stream_at(prg: bytes, base: int) -> bytes:
    if base == 0:
        return b""
    off = base - 0x8000
    out = bytearray()
    for b in prg[off:off + 1024]:
        out.append(b)
        if b == 0x00:  # SongEnd 含尾
            break
    return bytes(out)


def env_at(prg: bytes, base: int) -> bytes:
    off = base - 0x8000
    out = bytearray()
    for b in prg[off:off + 64]:
        out.append(b)
        if b in (0xFF, 0xF0):  # 终端字节含尾（keepPrev/killVolume——noInc 停驻）
            break
    return bytes(out)


def hexrow(bs: bytes, indent: str = "        ") -> str:
    return "\n".join(indent + " ".join(f"0x{b:02X}," for b in bs[i:i + 8])
                     for i in range(0, len(bs), 8))


def gen(prg: bytes) -> str:
    parts = ["//! 乐数据流+包络表（W3-2.5 烘焙——tools/bake_music_streams.py 生成；**禁手抄**）。\n",
             "//! ROM 锚：SongHeaderOffsetTable $BBFA→SongHeaders $BD31 头[5..13]=4 通道基址；\n",
             "//! 流=基址起至 0x00（SongEnd）含尾；VolumeEnvelopePtrTable $BCB0→Envelope1-5\n",
             "//! （至 $FF/$F0 终端含尾）。偏移口径=addr-$8000。\n\n"]
    # 逐曲逐通道流
    parts.append("/// 12 曲×4 通道流（None=基址 0 空通道）。\n")
    parts.append("pub const SONG_STREAMS: [[Option<&[u8]>; 4]; 12] = [\n")
    bases_rows = []
    for s in range(N_SONGS):
        off = prg[HDR_OFF_TBL + s]
        hdr = prg[SONG_HEADERS + off:SONG_HEADERS + off + 13]
        row = []
        parts.append("    [\n")
        for c in range(4):
            base = hdr[5 + c * 2] | (hdr[6 + c * 2] << 8)
            row.append(base)
            st = stream_at(prg, base)
            if not st:
                parts.append("        None,\n")
            else:
                parts.append(f"        Some(&[\n{hexrow(st)}\n        ]),\n")
        bases_rows.append(row)
        parts.append("    ],\n")
    parts.append("];\n\n")
    parts.append("/// 12 曲×4 通道基址（头[5..13] 原值——流反查键）。\n")
    parts.append("pub const STREAM_BASES: [[u16; 4]; 12] = [\n")
    for row in bases_rows:
        parts.append("    [" + ", ".join(f"0x{b:04X}" for b in row) + "],\n")
    parts.append("];\n\n")
    parts.append("/// 按曲号（SongHeaderOffsetTable 序）×通道（0=SQ1/1=SQ2/2=Tri/3=Noise）取流。\n")
    parts.append("pub fn song_stream(song: usize, ch: usize) -> Option<&'static [u8]> {\n")
    parts.append("    SONG_STREAMS.get(song).and_then(|s| s.get(ch)).copied().flatten()\n")
    parts.append("}\n\n")
    parts.append("/// 按**基址**反查流（忠实径：乐循环读 $0630+2X 基址字取流——$065E 曲号会被\n")
    parts.append("/// 音乐 cycle 的 FindMusicInitIndex 重置，不可作键）。\n")
    parts.append("pub fn stream_by_base(base: u16) -> Option<&'static [u8]> {\n")
    parts.append("    for (song, row) in STREAM_BASES.iter().enumerate() {\n")
    parts.append("        for (ch, b) in row.iter().enumerate() {\n")
    parts.append("            if *b == base && base != 0 {\n")
    parts.append("                return song_stream(song, ch);\n")
    parts.append("            }\n")
    parts.append("        }\n")
    parts.append("    }\n")
    parts.append("    None\n")
    parts.append("}\n\n")
    # 包络
    parts.append("/// VolumeEnvelopePtrTable（$BCB0——5 word；$EC/$ED 写面源值）。\n")
    env_bases = [prg[ENV_PTRS + i * 2] | (prg[ENV_PTRS + i * 2 + 1] << 8) for i in range(N_ENVS)]
    ptrs = ", ".join(f"0x{b:04X}" for b in env_bases)
    parts.append(f"pub const VOLUME_ENVELOPE_PTRS: [u16; 5] = [{ptrs}];\n\n")
    parts.append("/// VolumeEnvelope1-5（$BCBA 族——至 $FF/$F0 终端含尾）。\n")
    parts.append("pub const VOLUME_ENVELOPES: [&[u8]; 5] = [\n")
    for b in env_bases:
        e = env_at(prg, b)
        parts.append(f"    &[\n{hexrow(e)}\n    ],\n")
    parts.append("];\n\n")
    # SFXData 鼓点表（$B200 16B——SongNoteNoise `lda SFXData,y`：y=01/04/07/0A 三字节族）
    sfx = prg[0x3200:0x3210]
    parts.append("/// SFXData 鼓点表（$B200 16B——y=01/04/07/0A 三字节族；SongNoteNoise 源）。\n")
    parts.append(f"pub const SFX_DATA: [u8; 16] = [\n{hexrow(sfx)}\n];\n")
    return "".join(parts)


def main() -> int:
    prg = load()
    text = gen(prg)
    if "--check" in sys.argv:
        cur = OUT.read_text(encoding="utf-8") if OUT.is_file() else ""
        if cur == text:
            print("[PASS] music_streams 与 ROM 烘焙一致（--check 无漂移）")
            return 0
        print("[FAIL] music_streams 与 ROM 烘焙不一致（重跑生成）")
        return 1
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(text, encoding="utf-8", newline="\n")
    n = sum(1 for s in range(N_SONGS)
            for c in range(4)
            if stream_at(prg, (lambda h: h[5 + c * 2] | (h[6 + c * 2] << 8))(
                prg[SONG_HEADERS + prg[HDR_OFF_TBL + s]:][:13])))
    print(f"[OK] music_streams 生成 → {OUT.relative_to(ROOT)}（流 {n} 件+包络 5 件）")
    return 0


if __name__ == "__main__":
    sys.exit(main())
