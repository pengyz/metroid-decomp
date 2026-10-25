#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""tools/bake_palette_data.py —— 调色板 PalettePtrTable+数据体 ROM 烘焙（可再生；D 面禁手抄）。

ROM 锚（A 级）：PalettePtrTable $9560（**各 bank 同址**（bank0 标题/1-5 区）——prg1_brinstar
.asm:55 等；25 项（_id_Palette00-18）——Samus 淡入调色=项 19+（SamusFadeIn 路））。
数据体=VRAMStruct 形（[hi][lo][ctrl][data…] hi==0 终止——C20E/VRAMStructWrite 共享解码器）。
字节自证：python3 -c "p=open('rom/Metroid (USA).nes','rb').read()[0x10:]; print(p[0x5560:0x5574].hex())"
"""
from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
ROM = ROOT / "rom" / "Metroid (USA).nes"
OUT = ROOT / "crates" / "game-data" / "src" / "title" / "palette_data.rs"

TABLE_ADDR = 0x1560  # bank0: $9560-0x8000（各 bank +0x4000*n）
N_ENTRIES = 25


def hexrow(bs, indent="        "):
    return "\n".join(indent + " ".join(f"0x{b:02X}," for b in bs[i:i + 16])
                     for i in range(0, len(bs), 16))


def blob_at(prg, bank_off, addr):
    """VRAMStruct 体（hi==0 终止自界）。"""
    out = bytearray()
    off = bank_off + (addr - 0x8000)
    i = 0
    while i < 256:
        b = prg[off + i]
        out.append(b)
        i += 1
        if b == 0 and i % 1 == 0 and len(out) >= 3 and out[-3] == 0:
            break
        if b == 0 and (i - 1) % 1 == 0 and len(out) >= 1 and out[-1] == 0 and i > 3:
            # hi 字节=0 判（记录首字节）
            if (i - 1) % 1 == 0 and len(out) >= 3 and out[-3] == 0:
                break
    return bytes(out)


def blob_len(prg, bank_off, addr):
    off = bank_off + (addr - 0x8000)
    n = 0
    while n < 256:
        if prg[off + n] == 0 and n % 1 == 0:
            # 记录首字节 hi==0（终止）——记录=3+count 或 4（RLE）
            # 简化：扫描到 hi==0 的记录首（步长按记录形走）
            pass
        n += 1
    return n


def stream(prg, bank_off, addr):
    """VRAMStruct 记录流（记录形：[hi][lo][ctrl][data..]——hi==0 记录=终止）。"""
    out = bytearray()
    o = bank_off + (addr - 0x8000)
    while True:
        hi = prg[o]
        out.append(hi)
        if hi == 0:
            break
        lo = prg[o + 1]
        ctrl = prg[o + 2]
        out.append(lo)
        out.append(ctrl)
        count = ctrl & 0x3F
        if count == 0:
            count = 256
        if ctrl & 0x40:  # RLE
            out.append(prg[o + 3])
            o += 4
        else:
            for k in range(count):
                out.append(prg[o + 3 + k])
            o += 3 + count
    return bytes(out)


def gen() -> str:
    prg = ROM.read_bytes()[0x10:]
    parts = []
    parts.append("//! 调色板 PalettePtrTable+数据体（r25 烘焙——tools/bake_palette_data.py 生成；**禁手抄**）。\n")
    parts.append("//!\n//! ROM 锚（A 级）：PalettePtrTable \$9560（各 bank 同址——25 项；Samus 淡入=项 19+）。\n")
    parts.append("//! 字节自证：`python3 -c \"p=open('rom/Metroid (USA).nes','rb').read()[0x10:]; print(p[0x5560:0x5574].hex())\"`\n\n")
    parts.append("/// bank 序（0=标题/1=Brinstar/2=Norfair/3=Tourian/4=Kraid/5=Ridley）。\n")
    for bank in range(6):
        boff = bank * 0x4000
        tbl = [prg[TABLE_ADDR + boff + i * 2] | (prg[TABLE_ADDR + boff + i * 2 + 1] << 8)
               for i in range(N_ENTRIES)]
        parts.append(f"\n/// bank{bank}：PalettePtrTable（\$9560——25 项）。\n")
        parts.append(f"pub const PAL_PTRS_{bank}: [u16; 25] = [\n")
        parts.append(", ".join(f"0x{a:04X}" for a in tbl) + ",\n];\n")
        parts.append(f"/// bank{bank}：调色板数据体（指针→VRAMStruct 流拼接——按 PTRS 偏移索引）。\n")
        allb = bytearray()
        offs = []
        for a in tbl:
            offs.append(len(allb))
            allb += stream(prg, boff, a)
        parts.append(f"pub const PAL_DATA_{bank}: [u8; {len(allb)}] = [\n")
        parts.append(hexrow(bytes(allb)) + "\n];\n")
        parts.append(f"/// bank{bank}：各记录在 DATA 内偏移。\n")
        parts.append(f"pub const PAL_OFFS_{bank}: [u16; 25] = [\n")
        parts.append(", ".join(f"0x{o:04X}" for o in offs) + ",\n];\n")
    parts.append("""
#[cfg(test)]
mod tests {
    use super::*;

    /// ROM 锚：25 项界+bank1（Brinstar）淡入项 19+ 存在（Samus 淡入调色面）。
    #[test]
    fn palette_data_rom_anchor() {
        assert_eq!(PAL_PTRS_1.len(), 25);
        assert_ne!(PAL_PTRS_1[19], 0, "淡入调色项 19 在位");
        assert_ne!(PAL_DATA_1[0], 0, "数据体非空");
    }
}
""")
    return "".join(parts)


def main() -> int:
    text = gen()
    if "--check" in sys.argv:
        cur = OUT.read_text(encoding="utf-8") if OUT.is_file() else ""
        ok = cur == text
        print(f"[{'PASS' if ok else 'FAIL'}] palette_data 与 ROM 烘焙一致（--check）")
        return 0 if ok else 1
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(text, encoding="utf-8", newline="\n")
    print(f"[OK] palette_data 生成 → {OUT.relative_to(ROOT)}（6 bank×25 项）")
    return 0


if __name__ == "__main__":
    sys.exit(main())
