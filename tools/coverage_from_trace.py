#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""tools/coverage_from_trace.py —— oracle 覆盖报告（缺口左移机制③）。

场景语料不靠想象编：跑原版（oracle）收集**被证明存在的行为**（执行过的地址），
未触达区域 = 行为面盲区清单 → 下一条场景的覆盖目标。

覆盖来源（按可信度显式分级，不静默降级）：
  execute-callback   scenario_capture.lua 的 execute 内存回调（指令级，首选）
  capture-pc-sampled 回调不可用时退化为采集点 cpu.pc 采样（粗粒度，报告中显式标注）

地址语义（诚实报告，不强行归属）：
  $C000-$FFFF → bank7 固定窗，可精确映射 seed 行（bank07:ADDR）
  $8000-$BFFF → 可切窗口，运行时 bank 不明——按窗口地址计数，不冒充 bank 归属

用法：python3 tools/coverage_from_trace.py <name> [<name> ...]
输出：analysis/traces/<name>.coverage-report.json + 合并并集 data/status/coverage_union.json
"""
from __future__ import annotations

import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
TRACES = ROOT / "analysis" / "traces"
SEED = ROOT / "reference" / "seed" / "metroid.asm"
UNION = ROOT / "data" / "status" / "coverage_union.json"

SEED_LINE = re.compile(r"^bank(\d\d):([0-9A-F]{4})\s")


def load_seed_index() -> dict[str, tuple[str, str]]:
    """bank07 固定窗地址 → (bank, addr) seed 行定位索引。"""
    idx: dict[str, tuple[str, str]] = {}
    if not SEED.is_file():
        return idx
    for line in SEED.read_text(encoding="utf-8").splitlines():
        m = SEED_LINE.match(line)
        if m and m.group(1) == "07":
            idx[m.group(2)] = ("07", m.group(2))
    return idx


def read_coverage(name: str) -> tuple[str, set[int]]:
    cov = TRACES / f"{name}.coverage.jsonl"
    if cov.is_file():
        mode, addrs = "unknown", set()
        for i, line in enumerate(cov.read_text(encoding="utf-8").splitlines()):
            rec = json.loads(line)
            if i == 0:
                mode = rec.get("mode", "unknown")
            elif "a" in rec:
                addrs.add(int(rec["a"], 16))
        return mode, addrs
    # 退化：采集点 cpu.pc 采样
    caps = TRACES / f"{name}.captures.jsonl"
    addrs = set()
    if caps.is_file():
        for line in caps.read_text(encoding="utf-8").splitlines():
            rec = json.loads(line)
            cpu = rec.get("cpu") or {}
            pc = cpu.get("pc")
            if isinstance(pc, int):
                addrs.add(pc)
    return "capture-pc-sampled", addrs


def main() -> int:
    names = sys.argv[1:]
    if not names:
        print("用法: python3 tools/coverage_from_trace.py <scenario> [...]")
        return 2
    seed_idx = load_seed_index()
    union = set()
    if UNION.is_file():
        u = json.loads(UNION.read_text(encoding="utf-8"))
        union = {int(a, 16) for a in u.get("addresses", [])}

    for name in names:
        mode, addrs = read_coverage(name)
        if mode == "unsupported":
            print(f"[{name}] ⚠ execute 回调不可用且无采集点 PC——覆盖为空（不冒充）")
            continue
        fixed = sorted(a for a in addrs if a >= 0xC000)
        window = sorted(a for a in addrs if 0x8000 <= a < 0xC000)
        seed_hits = sorted({seed_idx[f"{a:04X}"] for a in fixed if f"{a:04X}" in seed_idx})
        report = {
            "name": name, "mode": mode,
            "distinct_addresses": len(addrs),
            "fixed_bank7": {"count": len(fixed), "seed_lines_hit": len(seed_hits)},
            "window_8000": {"count": len(window), "bank_attribution": "unknown（可切窗口，不冒充归属）"},
        }
        out = TRACES / f"{name}.coverage-report.json"
        out.write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
        print(f"[{name}] mode={mode} distinct={len(addrs)} "
              f"(bank7 fixed={len(fixed)} → seed 行 {len(seed_hits)}；window={len(window)} 归属未知)")
        union |= addrs

    UNION.write_text(json.dumps(
        {"addresses": [f"{a:04X}" for a in sorted(union)], "count": len(union)},
        ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(f"并集累计: {len(union)} 个不同地址（data/status/coverage_union.json，只增不减）")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
