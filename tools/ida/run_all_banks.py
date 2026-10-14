#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""tools/ida/run_all_banks.py —— S1 IDA 批量导出驱动（逻辑唯一所在；shell 只做薄包装）。

职责：
1. 从 ROM 抽取 8×16KB PRG bank bin（Metroid MMC1：bank0-6 → CPU $8000，bank7 → $C000）；
2. 逐 bank 调 idat 批处理跑 tools/ida/export_bank.py 导出机械 JSON；
3. 汇报每 bank 导出结果（缺导出 = 显式报错，不静默）。

用法（跨平台）：
    python3 tools/ida/run_all_banks.py            # 全部 8 个 bank
    python3 tools/ida/run_all_banks.py 0 3 7      # 只跑指定 bank

环境变量：
    IDAT   idat 可执行文件（缺省自动探测：环境 IDAT → $IDA_PATH/idat[64][.exe] → PATH 上的 idat）
    ROM    原版 ROM 路径（缺省 rom/Metroid (USA).nes）
    OUTDIR 输出目录（缺省 analysis/ida）

输出（可再生，不入库）：analysis/ida/bankNN.bin / bankNN.json / bankNN.log
"""
from __future__ import annotations

import os
import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent.parent
ROM = Path(os.environ.get("ROM") or (ROOT / "rom" / "Metroid (USA).nes"))
OUTDIR = Path(os.environ.get("OUTDIR", ROOT / "analysis" / "ida"))
EXPORT = ROOT / "tools" / "ida" / "export_bank.py"

N_BANKS = 8
BANK_SIZE = 0x4000  # 16KB（MMC1）
PRG_SIZE = N_BANKS * BANK_SIZE


def base_for(bank: int) -> int:
    """canonical CPU base per bank（证据：docs/findings/rom-disasm-equivalence.md §3）。

    bank7 → $C000 固定；bank0-6 → MMC1 可切窗口 $8000。
    """
    return 0xC000 if bank == 7 else 0x8000


def find_idat() -> str:
    env = os.environ.get("IDAT")
    if env:
        return env
    ida_path = os.environ.get("IDA_PATH", "/home/peng/ida-pro-9.1")
    for cand in (Path(ida_path) / "idat64", Path(ida_path) / "idat",
                 Path(ida_path) / "idat64.exe", Path(ida_path) / "idat.exe"):
        if cand.is_file():
            return str(cand)
    for name in ("idat64", "idat", "idat64.exe", "idat.exe"):
        found = shutil.which(name)
        if found:
            return found
    raise SystemExit("找不到 idat：请设置 IDAT 或 IDA_PATH 环境变量")


def read_vectors(prg: bytes) -> str:
    """ROM 尾部向量表（$FFFA-$FFFF，PRG offset 0x1FFFA）→ 机械 entry 候选（NMI/RESET/IRQ）。"""
    v = prg[0x1FFFA:0x20000]
    if len(v) != 6:
        raise SystemExit("向量表读取失败")
    nmi, reset, irq = v[0] | (v[1] << 8), v[2] | (v[3] << 8), v[4] | (v[5] << 8)
    print(f"vectors: NMI=${nmi:04X} RESET=${reset:04X} IRQ=${irq:04X}")
    return f"0x{nmi:04X},0x{reset:04X},0x{irq:04X}"


def extract_banks(rom: Path, outdir: Path) -> str:
    data = rom.read_bytes()
    if data[:4] != b"NES\x1a":
        raise SystemExit(f"不是 iNES 文件: {rom}")
    prg = data[0x10:0x10 + PRG_SIZE]
    if len(prg) != PRG_SIZE:
        raise SystemExit(f"PRG 大小 {len(prg)} != {PRG_SIZE}")
    outdir.mkdir(parents=True, exist_ok=True)
    for k in range(N_BANKS):
        (outdir / f"bank{k:02d}.bin").write_bytes(prg[k * BANK_SIZE:(k + 1) * BANK_SIZE])
    print(f"extracted {N_BANKS} bank bins ({BANK_SIZE // 1024}KB each)")
    return read_vectors(prg)


def run_bank(idat: str, bank: int, outdir: Path, vectors: str) -> bool:
    k = f"{bank:02d}"
    base = base_for(bank)
    stem = outdir / f"bank{k}.bin"
    for suffix in (".i64", ".id0", ".id1", ".nam", ".til"):
        Path(str(stem) + suffix).unlink(missing_ok=True)
    env = dict(os.environ)
    env.update({
        "S1_BANK": str(bank),
        "S1_BASE": hex(base),
        "S1_OUT": str(outdir / f"bank{k}.json"),
        "S1_VECTORS": vectors,
    })
    cmd = [idat, "-A", "-pm6502", f"-S{EXPORT}", f"-L{outdir / f'bank{k}.log'}", str(stem)]
    subprocess.run(cmd, env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, check=False)
    log = outdir / f"bank{k}.log"
    if log.is_file():
        for line in log.read_text(encoding="utf-8", errors="replace").splitlines():
            if "S1 export" in line:
                print(line)
                return True
    print(f"bank{k}: EXPORT MISSING (see {log})")
    return False


def main() -> int:
    idat = find_idat()
    targets = [int(a) for a in sys.argv[1:]] or list(range(N_BANKS))
    for t in targets:
        if not 0 <= t < N_BANKS:
            raise SystemExit(f"bank 序号越界: {t}")
    if targets == list(range(N_BANKS)):
        vectors = extract_banks(ROM, OUTDIR)
    else:
        vectors = read_vectors(ROM.read_bytes()[0x10:0x20010])
    ok = all(run_bank(idat, k, OUTDIR, vectors) for k in targets)
    return 0 if ok else 1


if __name__ == "__main__":
    raise SystemExit(main())
