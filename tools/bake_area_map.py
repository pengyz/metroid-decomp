#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""tools/bake_area_map.py —— 世界地图+区域起点 ROM 烘焙（可再生；D 面禁手抄）。

ROM 锚（A 级）：
- WorldMap $A53E（prg0_title.asm:5118——32×32=1KB 地图表）→ prg[0x253E]
- AreaMapPosX/Y $95D7/$95D8（各区域 bank——prg1_brinstar.asm:131 等）→ prg[N*0x4000+0x15D7]
- CopyMap $A93E（prg0_title.asm:5153——WorldMap→WorldMapRAM $7000-$73FF 4 页拷贝；
  调用点=InitBank0（prg7_engine.asm:1382——**复位链件**））

字节自证：
    python3 -c "p=open('rom/Metroid (USA).nes','rb').read()[0x10:]; print(p[0x253E:0x255E].hex())"
"""
from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
ROM = ROOT / "rom" / "Metroid (USA).nes"
OUT = ROOT / "crates" / "game-data" / "src" / "title" / "world_map.rs"


def hexrow(bs: bytes, indent: str = "        ") -> str:
    return "\n".join(indent + " ".join(f"0x{b:02X}," for b in bs[i:i + 16])
                     for i in range(0, len(bs), 16))


def gen() -> str:
    prg = ROM.read_bytes()[0x10:]
    world = prg[0x253E:0x253E + 0x400]
    pos = []
    for bank in range(1, 6):
        x = prg[bank * 0x4000 + 0x15D7]
        y = prg[bank * 0x4000 + 0x15D8]
        pos.append((x, y))
    pos_s = ", ".join("(%d, %d)" % (x, y) for x, y in pos)
    head = (
        "//! 世界地图+区域起点（r25 烘焙——tools/bake_area_map.py 生成；**禁手抄**）。\n"
        "//!\n"
        "//! ROM 锚（A 级）：WorldMap $A53E（prg0_title.asm:5118——32×32 表）；AreaMapPosX/Y\n"
        "//! $95D7/$95D8（区 bank 族）。字节自证：\n"
        '//! `python3 -c "p=open(\'rom/Metroid (USA).nes\',\'rb\').read()[0x10:]; print(p[0x253E:0x255E].hex())"`\n'
        "\n"
        "/// WorldMap（$A53E——1KB：行=MapY 列=MapX；值=房间号/$FF=无房）。\n"
        "pub const WORLD_MAP: [u8; 1024] = [\n"
    )
    mid = (
        "];\n\n"
        "/// 区域起点（AreaMapPosX/Y $95D7/$95D8 族——序=InArea&$0F：0=Brinstar…4=Ridley）。\n"
        "pub const AREA_MAP_POS: [(u8, u8); 5] = [" + pos_s + "];\n\n"
        "#[cfg(test)]\n"
        "mod tests {\n"
        "    use super::*;\n\n"
        "    /// ROM 锚：地图 1KB 尺寸+首行 FF 形+起点（Brinstar x=3/y=0E——金侧 f520 实证）。\n"
        "    #[test]\n"
        "    fn world_map_rom_anchor() {\n"
        "        assert_eq!(WORLD_MAP.len(), 1024);\n"
        "        assert_eq!(&WORLD_MAP[0..4], &[0xFF, 0xFF, 0xFF, 0xFF]);\n"
        '        assert_eq!(AREA_MAP_POS[0], (0x03, 0x0E), "Brinstar 起点（MapPos 03/0E）");\n'
        "        // 起始房：MapY=0E/MapX=03 → 地图[0E*32+3]=0x09（房间 9——金侧 RoomPtr=A5D5 对形）\n"
        "        assert_eq!(WORLD_MAP[0x0E * 32 + 3], 0x09);\n"
        "    }\n"
        "}\n"
    )
    return head + hexrow(world) + "\n" + mid


def main() -> int:
    text = gen()
    if "--check" in sys.argv:
        cur = OUT.read_text(encoding="utf-8") if OUT.is_file() else ""
        ok = cur == text
        print(f"[{'PASS' if ok else 'FAIL'}] world_map 与 ROM 烘焙一致（--check）")
        return 0 if ok else 1
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(text, encoding="utf-8", newline="\n")
    print(f"[OK] world_map 生成 → {OUT.relative_to(ROOT)}（1KB 地图+5 起点）")
    return 0


if __name__ == "__main__":
    sys.exit(main())
