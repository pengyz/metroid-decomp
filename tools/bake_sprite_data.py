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
    """common_sprite_data.asm 数据字节计（.byte/.word 项计——含符号形）。

    【证伪更正（2026-10-29）】旧形只计 $XX 字面——符号形 `.byte _id_ObjFrame_*`
    （anim 表/帧表）全漏⇒915B（真 1426B（$8572-$8B03 址跨））⇒**烘焙截断**
    （帧记录 $890A+ 越界→blob 0xFF 兜底⇒radius/ctrl 全 FF⇒弹不可见⇒「打不出子弹」
    （用户实测（帧 0x28 记录 ROM=$890A（04 02 02 30…）实证）））。
    """
    text = SRC.read_text(encoding="utf-8")
    n = 0
    max_addr = 0
    for line in text.splitlines():
        ann = re.search(r"; \$([0-9A-Fa-f]{4})", line)
        if ann:
            max_addr = max(max_addr, int(ann.group(1), 16))
        s = line.split(";")[0].strip()
        m = re.search(r"\.(byte|db)\s+(.*)", s)
        if m:
            n += len([x for x in m.group(2).split(",") if x.strip()])
            continue
        m = re.search(r"\.(word|dw)\s+(.*)", s)
        if m:
            n += 2 * len([x for x in m.group(2).split(",") if x.strip()])
    # 址跨兜底（声明计漏形防护（blob=ROM 直切（多切无害（指针索引））））：
    # 尾注 max 址+32 裕（末记录长）——n 取二者大（截断根治（帧记录 $890A+ 覆盖））。
    if max_addr:
        n = max(n, max_addr + 32 - 0x8572)
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
