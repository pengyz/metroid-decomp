#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""tools/bake_vram_structs.py —— VRAMStruct blob ROM 烘焙生成器（可再生）。

从 ROM 提取 PreparePPUProcess/VRAMStructWrite 消费的 VRAMStruct 数据体，
生成 crates/game-data/src/title/vram_structs.rs。

用法：
    python3 tools/bake_vram_structs.py           # 生成 vram_structs.rs
    python3 tools/bake_vram_structs.py --check   # 校验生成件与 ROM 一致

格式（macros.asm + prg7_engine.asm:813 `VRAMStructWriteData: ; 07:C2E4` 消费语义）：
  记录 = [hi, lo, ctrl, ...]；hi==0 ⇒ VRAMStructEnd（含 1B 终止符）。
  ctrl bit7=PPU 增量 32 位（SetPPUInc CF 面）；bit6=RLE 位；低 6 位=计数（0⇒256）。
  非 RLE：记录长=3+count；RLE：记录长=4（单字节重复 count 次）。

字节自证（一行复现，抽首记录）：
    python3 -c "p=open('rom/Metroid (USA).nes','rb').read()[0x10:]; print(p[0x02F4:0x02F4+36].hex())"

asm 引证（A 级原文）：refs/m1disasm/SRC/prg0_title.asm:618
`VRAMStruct_DrawIntroBackground: ; 00:82F4`（intro 背景 tilemap——属性表+$22E0 起地面/正文行）
+ :180 `DrawIntroBackground: ; 00:80D0`（`ldx #<.b/ldy #>.b/jsr PreparePPUProcess`）。
"""
from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
ROM = ROOT / "rom" / "Metroid (USA).nes"
OUT = ROOT / "crates" / "game-data" / "src" / "title" / "vram_structs.rs"

# (符号名, CPU 地址, bank) —— bank0: $8000 窗 = prg[0x0000..]（boot 态 PRG reg=0 自证）
BLOBS = [
    ("INTRO_BG_VRAM_STRUCT", 0x82F4, 0),
    # L9984（prg0_title.asm:3534——StartContinueScreen15/1B :2333 `ldx #<L9984` 装载件：
    # START/Continue 选单屏 tilemap）。r25 补（title→游戏链首抓手）。
    ("START_CONTINUE_VRAM_STRUCT", 0x9984, 0),
]


def read_rom() -> bytes:
    if not ROM.is_file():
        sys.exit(f"ROM 缺失: {ROM}（BYO-ROM，见 rom/README.md）")
    return ROM.read_bytes()[0x10:]


def extract(prg: bytes, addr: int, bank: int) -> bytes:
    """按消费语义解析记录至 hi==0 终止（含终止字节）——禁盲扫 0x00。"""
    off = bank * 0x4000 + (addr - 0x8000)
    i = off
    while True:
        hi = prg[i]
        if hi == 0:
            return prg[off:i + 1]  # 含 VRAMStructEnd 终止符
        ctrl = prg[i + 2]
        if ctrl & 0x40:
            i += 4  # RLE
        else:
            count = ctrl & 0x3F
            i += 3 + (256 if count == 0 else count)


def emit(prg: bytes) -> str:
    lines = [
        "//! VRAMStruct blob 烘焙面（PreparePPUProcess 源；**生成件勿手改**）。",
        "//!",
        "//! 生成器：`python3 tools/bake_vram_structs.py`（--check 校验径）。",
        "//! 格式与消费语义：prg7_engine.asm:813 `VRAMStructWriteData: ; 07:C2E4`（记录三态：",
        "//! [hi,lo,ctrl,…]；hi=0 终止；ctrl bit7=增量32/bit6=RLE/低6位=计数（0⇒256））。",
        "//! 字节自证（一行复现，抽首记录）：",
        "//!     python3 -c \"p=open('rom/Metroid (USA).nes','rb').read()[0x10:];"
        " print(p[0x02F4:0x02F4+36].hex())\"",
        "//!",
        "//! blob 真值表（CPU 址→prg 偏移——提取自证）：",
    ]
    for name, addr, bank in BLOBS:
        data = extract(prg, addr, bank)
        lines.append(f"//!   {name}: ${addr:04X} (bank{bank}) → prg[0x{bank * 0x4000 + addr - 0x8000:04X}] {len(data)}B（含终止）")
    lines.append("")
    for name, addr, bank in BLOBS:
        data = extract(prg, addr, bank)
        lines.append(f"/// {name}（${addr:04X} bank{bank}；{len(data)}B 含 VRAMStructEnd 终止）")
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
        if not OUT.is_file() or OUT.read_text(encoding="utf-8") != text:
            sys.exit(f"生成件与 ROM 不一致: {OUT}（重跑生成器）")
        print(f"OK: {OUT.relative_to(ROOT)} 与 ROM 提取一致（{len(BLOBS)} blob）")
        return 0
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(text, encoding="utf-8", newline="\n")
    sizes = ", ".join(f"{n}={len(extract(prg, a, b))}B" for n, a, b in BLOBS)
    print(f"生成 {OUT.relative_to(ROOT)}：{sizes}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
