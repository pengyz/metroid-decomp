#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""tools/bake_gfx.py —— GFX 装载表+数据体 ROM 烘焙（可再生；D 面禁手抄）。

ROM 锚（A 级）：GFXInfo $C6E0（prg7_engine.asm:1669——7B 条目（macros.asm:101
GFXInfoEntry）：[bank][src lo/hi][dest lo/hi][size lo/hi]——自描述）；LoadGFX $C7AB/
CopyGFXBlock $C7D5（bank 切→PPUDATA 走带）。
字节自证：python3 -c "p=open('rom/Metroid (USA).nes','rb').read()[0x10:]; print(p[0x1C6E0:0x1C6EC].hex())"
"""
from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
ROM = ROOT / "rom" / "Metroid (USA).nes"
OUT = ROOT / "crates" / "game-data" / "src" / "title" / "gfx_data.rs"

GFX_INFO = 0x1C6E0  # bank7: $C6E0-$C000+0x1C000
N_ENTRIES = 29      # $C6E0-$C7AB = 203B = 29×7


def hexrow(bs, indent="        "):
    return "\n".join(indent + " ".join(f"0x{b:02X}," for b in bs[i:i + 16])
                     for i in range(0, len(bs), 16))


def gen() -> str:
    prg = ROM.read_bytes()[0x10:]
    parts = []
    parts.append("//! GFX 装载表+数据体（r25 烘焙——tools/bake_gfx.py 生成；**禁手抄**）。\n")
    parts.append("//!\n//! ROM 锚（A 级）：GFXInfo $C6E0（prg7:1669——7B 条目：[bank][src lo/hi][dest lo/hi][size lo/hi]）。\n")
    parts.append('//! 字节自证：`python3 -c "p=open(\'rom/Metroid (USA).nes\',\'rb\').read()[0x10:]; print(p[0x1C6E0:0x1C6EC].hex())"`\n\n')
    parts.append(f"/// GFXInfo（$C6E0——{N_ENTRIES} 项 7B：bank/src/dest/size）。\n")
    tbl = prg[GFX_INFO:GFX_INFO + N_ENTRIES * 7]
    parts.append(f"pub const GFX_INFO: [u8; {len(tbl)}] = [\n")
    parts.append(hexrow(tbl) + "\n];\n\n")
    parts.append("/// GFX 数据体（各条目 data——按 INFO 的 bank/src/size 索引拼接）。\n")
    data = bytearray()
    offs = []
    for i in range(N_ENTRIES):
        e = tbl[i * 7:(i + 1) * 7]
        bank, s_lo, s_hi, _d_lo, _d_hi, z_lo, z_hi = e
        src = s_lo | (s_hi << 8)
        size = z_lo | (z_hi << 8)
        offs.append((len(data), size))
        boff = bank * 0x4000
        o = boff + (src - (0xC000 if bank == 7 else 0x8000))
        data += prg[o:o + size]
    parts.append(f"pub const GFX_DATA: [u8; {len(data)}] = [\n")
    parts.append(hexrow(bytes(data)) + "\n];\n\n")
    parts.append("/// 各条目在 DATA 内（偏移, 尺寸）。\n")
    parts.append("pub const GFX_OFFS: [(u32, u32); 29] = [\n")
    parts.append(", ".join(f"({o}, {s})" for o, s in offs) + ",\n];\n")
    parts.append("""
#[cfg(test)]
mod tests {
    use super::*;

    /// ROM 锚：29 项+条目自描述形（BrinBG1=entry 3——dest $1000 面）。
    #[test]
    fn gfx_data_rom_anchor() {
        assert_eq!(GFX_INFO.len() / 7, 29);
        // entry 3（BrinBG1）：bank=6/src=$9DA0/dest=$1000
        assert_eq!(GFX_INFO[21], 6, "entry3 bank");
        assert_eq!(GFX_INFO[22] as u16 | ((GFX_INFO[23] as u16) << 8), 0x9DA0, "entry3 src");
        assert_eq!(GFX_INFO[24] as u16 | ((GFX_INFO[25] as u16) << 8), 0x1000, "entry3 dest");
    }
}
""")
    return "".join(parts)


def main() -> int:
    text = gen()
    if "--check" in sys.argv:
        cur = OUT.read_text(encoding="utf-8") if OUT.is_file() else ""
        ok = cur == text
        print(f"[{'PASS' if ok else 'FAIL'}] gfx_data 与 ROM 烘焙一致（--check）")
        return 0 if ok else 1
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(text, encoding="utf-8", newline="\n")
    print(f"[OK] gfx_data 生成 → {OUT.relative_to(ROOT)}（{N_ENTRIES} 项）")
    return 0


if __name__ == "__main__":
    sys.exit(main())
