#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""tools/bake_gfx_blocks.py —— InitTitleGFX 七件 GFX 块 ROM 烘焙生成器（可再生）。

从 ROM bank6 提取 CopyGFXBlock 源面（GFXInfo 描述子 src/size 经 prg7_engine.asm
LoadGFX/CopyGFXBlock 语义定谳），生成 crates/game-data/src/render/gfx_blocks.rs。

用法：
    python3 tools/bake_gfx_blocks.py            # 生成 gfx_blocks.rs
    python3 tools/bake_gfx_blocks.py --check    # 只校验已生成文件与 ROM 一致（门禁径）

字节自证（一行复现，抽 TITLE 块首 16B）：
    python3 -c "p=open('rom/Metroid (USA).nes','rb').read()[0x10:]; print(p[6*0x4000+0x0BE0:6*0x4000+0x0BE0+16].hex())"

asm 引证（A 级原文）：refs/m1disasm/SRC/prg7_engine.asm:1786 `CopyGFXBlock: ; 07:C7D5`
（`lda (Temp01_GFXSource),y / sta PPUDATA`——ROM→PPU pattern table 逐字节）；
GFXInfo 表 prg[0x1C6E0+id*7]（bank07:C6E0——7B=bank/src2/dest2/size2）。
"""
from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
ROM = ROOT / "rom" / "Metroid (USA).nes"
OUT = ROOT / "crates" / "game-data" / "src" / "render" / "gfx_blocks.rs"

# InitTitleGFX 载入序（frame/gfx.rs::init_title_gfx 七件；JustInBailey 变体另径）：
# (符号名, GFXInfo id) —— 描述子真值=prg[0x1C6E0+id*7]（ROM 提取自证，禁手抄）
BLOCKS = [
    ("SAMUS_GFX", 0),
    ("INTRO_SPRITES_GFX", 20),
    ("TITLE_GFX", 21),
    ("SOLID_1FC0_GFX", 22),
    ("FONT_COMPLETE_GFX", 23),
    ("FONT_HUD_GFX", 24),
    ("SOLID_0FC0_GFX", 25),
    ("SAMUS_SUITLESS_GFX", 27),
]
GFXINFO_BASE = 0x1C6E0  # bank7 $C6E0（bank7 固定区 $C000 窗 → prg 偏移 0x1C000+0x6E0）


def read_rom() -> bytes:
    if not ROM.is_file():
        sys.exit(f"ROM 缺失: {ROM}（BYO-ROM，见 rom/README.md）")
    return ROM.read_bytes()[0x10:]  # 去 iNES 头（CPU↔file 换算显式：off=+$10）


def gfx_info(prg: bytes, idx: int) -> tuple[int, int, int, int]:
    e = prg[GFXINFO_BASE + idx * 7: GFXINFO_BASE + idx * 7 + 7]
    bank, slo, shi, dlo, dhi, zlo, zhi = e
    return bank, (shi << 8) | slo, (dhi << 8) | dlo, (zhi << 8) | zlo


def block_bytes(prg: bytes, idx: int) -> tuple[bytes, int, int, int]:
    bank, src, dest, size = gfx_info(prg, idx)
    off = bank * 0x4000 + (src - 0x8000)  # MMC1 16KB 窗 $8000 映射（bank0-6 可切）
    data = prg[off:off + size]
    assert len(data) == size, f"id{idx} 提取不足 {len(data)} != {size}"
    return data, src, dest, size


def emit(prg: bytes) -> str:
    lines = [
        "//! InitTitleGFX 七件 GFX 块烘焙面（CopyGFXBlock 源；**生成件勿手改**）。",
        "//!",
        "//! 生成器：`python3 tools/bake_gfx_blocks.py`（--check 校验径）。",
        "//! asm 引证（A 级原文）：refs/m1disasm/SRC/prg7_engine.asm:1786 `CopyGFXBlock: ; 07:C7D5`",
        "//! （`lda (Temp01_GFXSource),y / sta PPUDATA`）+ :1748 `LoadGFX: ; 07:C7AB`",
        "//! （GFXInfo 7B 描述子→$00-$06）+ GFXInfo 表 prg[0x1C6E0+id*7]（bank07:C6E0）。",
        "//! 字节自证（一行复现，抽 TITLE 块首 16B）：",
        "//!     python3 -c \"p=open('rom/Metroid (USA).nes','rb').read()[0x10:];"
        " print(p[6*0x4000+0x0BE0:6*0x4000+0x0BE0+16].hex())\"",
        "//!",
        "//! 描述子真值表（id→bank/src/dest/size——prg 提取自证，禁手抄）：",
    ]
    for name, idx in BLOCKS:
        bank, src, dest, size = gfx_info(prg, idx)
        lines.append(f"//!   {name} (id{idx}): bank{bank} src=${src:04X} dest=${dest:04X} size=${size:04X}")
    lines.append("")
    for name, idx in BLOCKS:
        data, _src, _dest, _size = block_bytes(prg, idx)
        lines.append(f"/// {name}（GFXInfo id{idx}；{len(data)}B——prg[bank*0x4000+src-0x8000..] 提取自证）")
        lines.append(f"pub const {name}: [u8; {len(data)}] = [")
        for i in range(0, len(data), 16):
            chunk = ", ".join(f"0x{b:02X}" for b in data[i:i + 16])
            lines.append(f"    {chunk},")
        lines.append("];")
        lines.append("")
    return "\n".join(lines)


def main() -> int:
    prg = read_rom()
    text = emit(prg)
    if "--check" in sys.argv:
        if not OUT.is_file():
            sys.exit(f"生成件缺失: {OUT}")
        cur = OUT.read_text(encoding="utf-8")
        if cur != text:
            sys.exit(f"生成件与 ROM 不一致: {OUT}（重跑生成器）")
        print(f"OK: {OUT.relative_to(ROOT)} 与 ROM 提取一致（{len(BLOCKS)} 块）")
        return 0
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(text, encoding="utf-8", newline="\n")
    total = sum(len(block_bytes(prg, i)[0]) for _, i in BLOCKS)
    print(f"生成 {OUT.relative_to(ROOT)}：{len(BLOCKS)} 块 {total}B")
    return 0


if __name__ == "__main__":
    sys.exit(main())
