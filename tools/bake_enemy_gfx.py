#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""tools/bake_enemy_gfx.py —— 敌帧/位数据面 ROM 烘焙（可再生；D 面禁手抄）。

ROM 锚（A 级）：AreaPointers_ROM 7 字 DSTRUCT（prg1:88——RoomPtrTable \$A314/
StructPtrTable \$A372/MetatileDefs \$AEF0/**EnFramePtrTable1 \$9DE0/EnFramePtrTable2
\$9EE0/EnPlacePtrTable \$9F0E/EnAnimTable \$9D6A**）；CopyAreaPointers \$C8B0
（prg7:1936——ROM→RAM \$3B-\$48）。帧流=[ctrl][radiusY][radiusX][流…\$FF]（sprite
域同形！）；位流=y/x 对×n（ObjPlace 同形）。
字节自证：python3 -c "p=open('rom/Metroid (USA).nes','rb').read()[0x10:]; print(p[0x5DE0:0x5DF0].hex())"
"""
from __future__ import annotations

import struct
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
ROM = ROOT / "rom" / "Metroid (USA).nes"
OUT = ROOT / "crates" / "game-data" / "src" / "area" / "enemy_gfx.rs"

# bank1（Brinstar）：偏移=0x4000+(addr-$8000)
BASE = 0x4000
ANIM_TABLE = 0x9D6A  # EnemyAnimIndexTbl
FRAME_PTRS1 = 0x9DE0
FRAME_PTRS2 = 0x9EE0
PLACE_PTRS = 0x9F0E
N_FRAMES = 256  # 两表各 128 项（carry 选表）
N_PLACES = 128


def hexrow(bs, indent="        "):
    return "\n".join(indent + " ".join(f"0x{b:02X}," for b in bs[i:i + 16])
                     for i in range(0, len(bs), 16))


def frame_end(prg, addr):
    """帧流长（[ctrl][ry][rx][流…] 至 \$FF）。
    流形：<\$FC=tile/=\$FC 偏移 3B/=\$FD 新 ctrl 2B/=\$FE 跳位 1B/=\$FF 尾。"""
    o = BASE + (addr - 0x8000)
    n = 3
    while n < 256:
        b = prg[o + n]
        if b == 0xFF:
            return n + 1
        if b < 0xFC:
            n += 1
        elif b == 0xFC:
            n += 3
        elif b == 0xFD:
            n += 2
        elif b == 0xFE:
            n += 1
        else:
            n += 1
    return n


def place_end(prg, addr, nxt):
    """位件长（y/x 对——至下一指针或合理界）。"""
    return max(4, (nxt - addr)) if nxt > addr else 16


def gen() -> str:
    prg = ROM.read_bytes()[0x10:]
    parts = []
    parts.append("//! 敌帧/位数据面（r26 烘焙——tools/bake_enemy_gfx.py 生成；**禁手抄**）。\n//!\n")
    parts.append("//! ROM 锚（A 级）：AreaPointers_ROM 7 字（prg1:88）；CopyAreaPointers $C8B0。\n")
    parts.append('//! 字节自证：`python3 -c "p=open(\'rom/Metroid (USA).nes\',\'rb\').read()[0x10:]; print(p[0x5DE0:0x5DF0].hex())"`\n\n')

    # 1. 四表（指针表本体）
    # 【证伪更正（2026-10-29）】64B=截断（EnemyDeathAnimIndex（ROM $960B）max idx=
    # 0x74（0x72/0x74）+序（0xFF 终符）⇒≥0x80——死 anim 走表越界（E0BC-wrap 实证）。
    # 尺寸锚=max 参照 idx（0x74）+序裕⇒0x80（128B）。
    anim = prg[BASE + (ANIM_TABLE - 0x8000):BASE + (ANIM_TABLE - 0x8000) + 128]
    parts.append(f"/// EnemyAnimIndexTbl（$9D6A——64B 动画索引族）。\npub const EN_ANIM_TABLE: [u8; 128] = [\n")
    parts.append(hexrow(anim) + "\n];\n\n")
    f1 = prg[BASE + (FRAME_PTRS1 - 0x8000):BASE + (FRAME_PTRS1 - 0x8000) + 256]
    f2 = prg[BASE + (FRAME_PTRS2 - 0x8000):BASE + (FRAME_PTRS2 - 0x8000) + 256]
    parts.append(f"/// EnFramePtrTable1（$9DE0——128 项×2B 帧指针）。\npub const EN_FRAME_PTRS1: [u8; 256] = [\n")
    parts.append(hexrow(f1) + "\n];\n\n")
    parts.append(f"/// EnFramePtrTable2（$9EE0——128 项×2B）。\npub const EN_FRAME_PTRS2: [u8; 256] = [\n")
    parts.append(hexrow(f2) + "\n];\n\n")
    pl = prg[BASE + (PLACE_PTRS - 0x8000):BASE + (PLACE_PTRS - 0x8000) + 256]
    parts.append(f"/// EnPlacePtrTable（$9F0E——128 项×2B 位件指针）。\npub const EN_PLACE_PTRS: [u8; 256] = [\n")
    parts.append(hexrow(pl) + "\n];\n\n")

    # 2. 帧/位数据体（指针→数据——帧流至 FF 自界）
    frames = bytearray()
    foffs = []
    for tbl in (f1, f2):
        for i in range(0, 256, 2):
            addr = tbl[i] | (tbl[i + 1] << 8)
            if addr == 0:
                foffs.append((0, 0))
                continue
            ln = frame_end(prg, addr)
            foffs.append((len(frames), ln))
            frames += prg[BASE + (addr - 0x8000):BASE + (addr - 0x8000) + ln]
    parts.append(f"/// 帧数据体（256 项——帧流 [ctrl][ry][rx][流…FF]）。\npub const EN_FRAMES: [u8; {len(frames)}] = [\n")
    parts.append(hexrow(bytes(frames)) + "\n];\n\n")
    parts.append("/// 帧项（偏移, 长）。\npub const EN_FRAME_OFFS: [(u32, u32); 256] = [\n")
    parts.append(", ".join(f"({o}, {s})" for o, s in foffs) + ",\n];\n\n")

    places = bytearray()
    poffs = []
    addrs = [(pl[i] | (pl[i + 1] << 8)) for i in range(0, 256, 2)]
    for i, addr in enumerate(addrs):
        if addr == 0:
            poffs.append((0, 0))
            continue
        nxt = addrs[i + 1] if i + 1 < len(addrs) and addrs[i + 1] else addr + 32
        ln = place_end(prg, addr, nxt)
        poffs.append((len(places), ln))
        places += prg[BASE + (addr - 0x8000):BASE + (addr - 0x8000) + ln]
    parts.append(f"/// 位件数据体（128 项——y/x 对流）。\npub const EN_PLACES: [u8; {len(places)}] = [\n")
    parts.append(hexrow(bytes(places)) + "\n];\n\n")
    parts.append("/// 位件项（偏移, 长）。\npub const EN_PLACE_OFFS: [(u32, u32); 128] = [\n")
    parts.append(", ".join(f"({o}, {s})" for o, s in poffs) + ",\n];\n")

    parts.append("""
#[cfg(test)]
mod tests {
    use super::*;

    /// ROM 锚：四表址形（EnFramePtrTable1 $9DE0 首项非零）。
    #[test]
    fn enemy_gfx_rom_anchor() {
        assert_eq!(EN_FRAME_PTRS1.len(), 256);
        assert_ne!(EN_FRAME_PTRS1[0] as u16 | ((EN_FRAME_PTRS1[1] as u16) << 8), 0);
        assert!(!EN_FRAMES.is_empty());
    }
}
""")
    return "".join(parts)


def main() -> int:
    text = gen()
    if "--check" in sys.argv:
        cur = OUT.read_text(encoding="utf-8") if OUT.is_file() else ""
        ok = cur == text
        print(f"[{'PASS' if ok else 'FAIL'}] enemy_gfx 与 ROM 烘焙一致（--check）")
        return 0 if ok else 1
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(text, encoding="utf-8", newline="\n")
    print(f"[OK] enemy_gfx 生成 → {OUT.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
