#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""tools/bake_sprite_data.py —— sprite 数据 blob ROM 烘焙（可再生；D 面禁手抄）。

ROM 锚（A 级）：common_sprite_data.asm（areas_common.asm:1311 .include——区 bank 共载
$8572 起）：ObjectAnimIndexTbl $8572/ObjFramePtrTable $860B/ObjPlacePtrTable $86DF/
ObjPlace*/frame data——**blob 整体烘焙**（指针内嵌自洽——端口按指针索引）。
字节自证：python3 -c "p=open('rom/Metroid (USA).nes','rb').read()[0x10:]; print(p[0x4572:0x4592].hex())"
"""
from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
ROM = ROOT / "rom" / "Metroid (USA).nes"
SRC = ROOT / "refs" / "m1disasm" / "SRC" / "common_sprite_data.asm"
OUT = ROOT / "crates" / "game-data" / "src" / "area" / "sprite_data.rs"

BLOB_ADDR = 0x4572  # bank1: $8572-0x8000+0x4000


def hexrow(bs, indent="        "):
    return "\n".join(indent + " ".join(f"0x{b:02X}," for b in bs[i:i + 16])
                     for i in range(0, len(bs), 16))


def blob_len() -> int:
    """common_sprite_data.asm 数据字节计（.byte/.word/.dword）。"""
    text = SRC.read_text(encoding="utf-8")
    n = 0
    for line in text.splitlines():
        s = line.split(";")[0]
        n += len(re.findall(r"\$[0-9A-Fa-f]{2}", s))
    return n


def gen() -> str:
    prg = ROM.read_bytes()[0x10:]
    n = blob_len()
    blob = prg[BLOB_ADDR:BLOB_ADDR + n]
    parts = []
    parts.append("//! sprite 数据 blob（r25 烘焙——tools/bake_sprite_data.py 生成；**禁手抄**）。\n")
    parts.append("//!\n//! ROM 锚（A 级）：common_sprite_data.asm（区 bank 共载 $8572 起）——\n")
    parts.append("//! ObjectAnimIndexTbl $8572/ObjFramePtrTable $860B/ObjPlacePtrTable $86DF/ObjPlace*/frame data。\n")
    parts.append('//! 字节自证：`python3 -c "p=open(\'rom/Metroid (USA).nes\',\'rb\').read()[0x10:]; print(p[0x4572:0x4592].hex())"`\n\n')
    parts.append(f"/// sprite 数据 blob（{n}B——指针内嵌自洽：端口按表指针索引）。\n")
    parts.append(f"pub const SPRITE_DATA: [u8; {n}] = [\n")
    parts.append(hexrow(blob) + "\n];\n\n")
    parts.append("/// 表偏移（blob 内——$8572 基）。\n")
    parts.append("pub const OBJECT_ANIM_INDEX_TBL: u16 = 0x0000; // $8572\n")
    parts.append("pub const OBJ_FRAME_PTR_TABLE: u16 = 0x0099; // $860B\n")
    parts.append("pub const OBJ_PLACE_PTR_TABLE: u16 = 0x016D; // $86DF\n")
    parts.append("""
#[cfg(test)]
mod tests {
    use super::*;

    /// ROM 锚：blob 尺寸+表偏移形（$8572/$860B/$86DF 三表界）。
    #[test]
    fn sprite_data_rom_anchor() {
        assert!(SPRITE_DATA.len() > 0x200);
        assert_eq!(OBJ_FRAME_PTR_TABLE - OBJECT_ANIM_INDEX_TBL, 0x860B - 0x8572);
        assert_eq!(OBJ_PLACE_PTR_TABLE - OBJ_FRAME_PTR_TABLE, 0x86DF - 0x860B);
    }
}
""")
    return "".join(parts)


def main() -> int:
    text = gen()
    if "--check" in sys.argv:
        cur = OUT.read_text(encoding="utf-8") if OUT.is_file() else ""
        ok = cur == text
        print(f"[{'PASS' if ok else 'FAIL'}] sprite_data 与 ROM 烘焙一致（--check）")
        return 0 if ok else 1
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(text, encoding="utf-8", newline="\n")
    print(f"[OK] sprite_data 生成 → {OUT.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
