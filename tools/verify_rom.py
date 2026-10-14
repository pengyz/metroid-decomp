#!/usr/bin/env python3
"""校验 Metroid (NES) 原版 ROM 并报告其结构。

用法:
    python3 tools/verify_rom.py rom/Metroid\ \(USA\).nes

功能:
  1. 解析 iNES header（mapper、PRG/CHR 大小、区域线索）
  2. 计算整文件与 headerless(PRG+CHR) 的 CRC32 / MD5 / SHA-256
  3. 与社区已知版本指纹比对，指出对应的 m1disasm 构建目标
  4. 按 16KB 切分 PRG banks，输出每 bank 的 CRC32（供后续 bank 级对照）

已知指纹来源: mstan/MetroidNESRecomp README (CRC32 表)。
"""
import sys
import zlib
import hashlib
from pathlib import Path

# (名称, m1disasm build target, headerless PRG+CHR CRC32, 整文件 .nes CRC32)
KNOWN = [
    ("Metroid (USA) / NTSC",     "NES_NTSC", 0x70080810, 0xA2C89CB9),
    ("Metroid (Europe) / PAL",   "NES_PAL",  0x7751588D, 0xA591CC24),
]


def fmt(b: bytes) -> str:
    return hashlib.sha256(b).hexdigest()


def main() -> int:
    if len(sys.argv) != 2:
        print(__doc__)
        return 2
    p = Path(sys.argv[1])
    if not p.is_file():
        print(f"错误: 找不到文件 {p}")
        return 1

    data = p.read_bytes()
    if data[:4] != b"NES\x1a":
        print("错误: 不是 iNES 格式（缺少 'NES<1A>' magic）。若为 headerless 或 FDS 格式请另行处理。")
        return 1

    prg_units, chr_units, flags6, flags7 = data[4], data[5], data[6], data[7]
    mapper = (flags7 & 0xF0) | (flags6 >> 4)
    prg_size, chr_size = prg_units * 16384, chr_units * 8192
    has_trainer = bool(flags6 & 0x04)
    body = data[16:]
    if has_trainer:
        body = body[512:]
    rom_body = body[: prg_size + chr_size]

    print(f"文件           : {p}")
    print(f"大小           : {len(data)} bytes")
    print(f"iNES mapper    : {mapper} ({'MMC1/SxROM' if mapper == 1 else '非 MMC1，请注意!'})")
    print(f"PRG            : {prg_size} bytes ({prg_units} x 16KB)")
    print(f"CHR            : {chr_size} bytes ({chr_units} x 8KB)")
    print(f"Trainer        : {'有' if has_trainer else '无'}")
    print()
    print(f"整文件  CRC32  : {zlib.crc32(data) & 0xFFFFFFFF:08X}")
    print(f"整文件  MD5    : {hashlib.md5(data).hexdigest()}")
    print(f"整文件  SHA256 : {hashlib.sha256(data).hexdigest()}")
    print(f"PRG+CHR CRC32  : {zlib.crc32(rom_body) & 0xFFFFFFFF:08X}  (headerless)")
    print(f"PRG+CHR SHA256 : {fmt(rom_body)}")
    print()

    hit = None
    for name, target, crc_body, crc_file in KNOWN:
        if (zlib.crc32(rom_body) & 0xFFFFFFFF) == crc_body and (zlib.crc32(data) & 0xFFFFFFFF) == crc_file:
            hit = (name, target)
            break
    if hit:
        print(f"✔ 匹配已知版本: {hit[0]}  →  m1disasm 构建目标: {hit[1]}")
    else:
        print("✘ 与已知 USA/PAL 指纹不匹配（可能是 FDS 版、MZM 内嵌版、翻译版或坏 dump）。")
        print("  仍可用作参考，但对照结果需自行标注版本。")

    print()
    print("PRG banks (16KB) CRC32:")
    for i in range(prg_units):
        bank = rom_body[i * 16384:(i + 1) * 16384]
        role = "Game Engine (固定 $C000)" if i == 7 else "可切 bank ($8000)"
        print(f"  bank{i:02d}  {zlib.crc32(bank) & 0xFFFFFFFF:08X}   {role}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
