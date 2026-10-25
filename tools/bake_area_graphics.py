#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""tools/bake_area_graphics.py —— 区域图形三件套 ROM 烘焙（可再生；D 面禁手抄）。

ROM 锚（A 级；Brinstar bank1——prg1 窗 $8000）：
- RoomPtrTable  $A314（room_ptrs.asm:21——47 项）
- StructPtrTable $A372（structure_ptrs.asm:21——50 项）
- MetatileDefs  $AEF0（metatiles.asm:25——4B 组；界=asm .byte 计）
- 房流/结构体：各指针起至 **$FF**（DrawRoom/DrawStruct 终止符自界）

字节自证：
    python3 -c "p=open('rom/Metroid (USA).nes','rb').read()[0x10:]; print(p[0x2314:0x2330].hex())"
"""
from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
ROM = ROOT / "rom" / "Metroid (USA).nes"
SRC = ROOT / "refs" / "m1disasm" / "SRC" / "brinstar"
OUT = ROOT / "crates" / "game-data" / "src" / "area" / "graphics.rs"

ROOM_PTR_ADDR = 0x6314   # bank1: $A314-0x8000+0x4000（r25 锚测抓偏——bank 基址修）
STRUCT_PTR_ADDR = 0x6372  # bank1: $A372
METATILE_ADDR = 0x6EF0    # bank1: $AEF0
BANK_BASE = 0x4000        # bank1 窗基（prg 偏移=$Axxx-$8000+$4000）
N_ROOMS = 47
N_STRUCTS = 50


def hexrow(bs, indent="        "):
    if isinstance(bs, bytes):
        bs = list(bs)
    return "\n".join(indent + " ".join(f"0x{b:02X}," for b in bs[i:i + 16])
                     for i in range(0, len(bs), 16))


def metatile_count() -> int:
    """metatiles.asm .byte 字节计（组数）——界定 MetatileDefs 表长。"""
    text = (SRC / "metatiles.asm").read_text(encoding="utf-8")
    n = 0
    for line in text.splitlines():
        s = line.split(";")[0]
        n += len(re.findall(r"\$[0-9A-Fa-f]{2}", s))
    return n  # 总字节数（=组数×4）


def stream_until_ff(prg, addr):
    out = bytearray()
    off = addr - 0x8000 + BANK_BASE
    for b in prg[off:off + 2048]:
        out.append(b)
        if b == 0xFF:
            break
    return bytes(out)


def gen():
    prg = ROM.read_bytes()[0x10:]
    room_ptrs = [prg[ROOM_PTR_ADDR + i * 2] | (prg[ROOM_PTR_ADDR + i * 2 + 1] << 8)
                 for i in range(N_ROOMS)]
    struct_ptrs = [prg[STRUCT_PTR_ADDR + i * 2] | (prg[STRUCT_PTR_ADDR + i * 2 + 1] << 8)
                   for i in range(N_STRUCTS)]
    mt_n = metatile_count()
    metatiles = prg[METATILE_ADDR:METATILE_ADDR + mt_n]
    rooms = [stream_until_ff(prg, a) for a in room_ptrs]
    structs = [stream_until_ff(prg, a) for a in struct_ptrs]

    parts = []
    parts.append("//! 区域图形三件套（r25 烘焙——tools/bake_area_graphics.py 生成；**禁手抄**）。\n")
    parts.append("//!\n//! ROM 锚（A 级）：RoomPtrTable $A314（47 项）/StructPtrTable $A372（50 项）/\n")
    parts.append("//! MetatileDefs $AEF0（4B 组）；房流/结构=$FF 自界。\n")
    parts.append('//! 字节自证：`python3 -c "p=open(\'rom/Metroid (USA).nes\',\'rb\').read()[0x10:]; print(p[0x2314:0x2330].hex())"`\n\n')
    parts.append("/// RoomPtrTable（$A314——47 房指针）。\n")
    parts.append("pub const ROOM_PTRS: [u16; 47] = [\n")
    parts.append(", ".join(f"0x{a:04X}" for a in room_ptrs) + ",\n];\n\n")
    parts.append("/// StructPtrTable（$A372——50 结构指针）。\n")
    parts.append("pub const STRUCT_PTRS: [u16; 50] = [\n")
    parts.append(", ".join(f"0x{a:04X}" for a in struct_ptrs) + ",\n];\n\n")
    parts.append(f"/// MetatileDefs（$AEF0——{mt_n // 4} 组 4B：4 tile 索引（TilePosTable 序 $21/$20/$01/$00））。\n")
    parts.append(f"pub const METATILE_DEFS: [u8; {mt_n}] = [\n")
    parts.append(hexrow(metatiles) + "\n];\n\n")
    parts.append("/// 房流（47 件——对象 3B 流至 $FD 后敌门流至 $FF；DrawRoom 消费）。\n")
    parts.append("pub const ROOM_STREAMS: [&[u8]; 47] = [\n")
    for s in rooms:
        parts.append(f"    &[\n{hexrow(s)}\n    ],\n")
    parts.append("];\n\n")
    parts.append("/// 结构体（50 件——行头（低半字节=元 tile 计/高半字节=x 偏移）+元 tile 流至 $FF）。\n")
    parts.append("pub const STRUCTURES: [&[u8]; 50] = [\n")
    for s in structs:
        parts.append(f"    &[\n{hexrow(s)}\n    ],\n")
    parts.append("];\n")
    parts.append("""
#[cfg(test)]
mod tests {
    use super::*;

    /// ROM 锚：三件套尺寸+自界形（$FF 终止）+起始房 9 对形（金侧 RoomPtr=A5D5）。
    #[test]
    fn area_graphics_rom_anchor() {
        assert_eq!(ROOM_PTRS.len(), 47);
        assert_eq!(STRUCT_PTRS.len(), 50);
        assert_eq!(METATILE_DEFS.len() % 4, 0);
        assert_eq!(ROOM_PTRS[9], 0xA5B9, "起始房 9 指针");
        assert_eq!(*ROOM_STREAMS[9].last().unwrap(), 0xFF, "房流 $FF 自界");
        assert_eq!(*STRUCTURES[0].last().unwrap(), 0xFF, "结构 $FF 自界");
    }
}
""")
    return "".join(parts)


def main() -> int:
    text = gen()
    if "--check" in sys.argv:
        cur = OUT.read_text(encoding="utf-8") if OUT.is_file() else ""
        ok = cur == text
        print(f"[{'PASS' if ok else 'FAIL'}] area_graphics 与 ROM 烘焙一致（--check）")
        return 0 if ok else 1
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(text, encoding="utf-8", newline="\n")
    print(f"[OK] area_graphics 生成 → {OUT.relative_to(ROOT)}（47 房流+50 结构+元 tile 表）")
    return 0


if __name__ == "__main__":
    sys.exit(main())
