#!/usr/bin/env python3
"""验证 m1disasm 构建产物与 rom/ 下原版 ROM 的等价性（可复现证据）。

用法（先构建 refs/m1disasm，见 docs/findings/rom-disasm-equivalence.md）:
    python3 tools/check_disasm_equivalence.py

判据:
  1. headerless PRG+CHR（游戏内容真身）逐字节一致 → 等价成立
  2. iNES header 差异单独报告（元数据，MMC1 下镜像位由 mapper 运行时控制，无功能影响）
"""
import sys
import zlib
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
ROM = ROOT / "rom/Metroid (USA).nes"
BUILT = ROOT / "refs/m1disasm/out/M1_NES_NTSC.nes"


def main() -> int:
    if not ROM.is_file() or not BUILT.is_file():
        print(f"缺少文件: {ROM if not ROM.is_file() else BUILT}")
        return 1
    a, b = ROM.read_bytes(), BUILT.read_bytes()
    print(f"ROM   : {len(a)} bytes, PRG+CHR CRC32 {zlib.crc32(a[16:]) & 0xFFFFFFFF:08X}")
    print(f"构建  : {len(b)} bytes, PRG+CHR CRC32 {zlib.crc32(b[16:]) & 0xFFFFFFFF:08X}")
    hdr_same = a[:16] == b[:16]
    body_same = a[16:] == b[16:]
    print(f"header 一致: {hdr_same}  ROM={a[:16].hex(' ')}")
    if not hdr_same:
        print(f"             构建={b[:16].hex(' ')}")
    print(f"PRG+CHR 一致: {body_same}")
    if body_same:
        print("✔ 等价确认: 游戏内容与 m1disasm 严格对应（header 差异为元数据）")
        return 0
    diffs = [i for i in range(min(len(a), len(b)) - 16) if a[16 + i] != b[16 + i]]
    print(f"✘ 内容不一致，差异 {len(diffs)} 字节，首处: {diffs[:10]}")
    return 2


if __name__ == "__main__":
    raise SystemExit(main())
