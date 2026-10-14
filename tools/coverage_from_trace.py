#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""tools/coverage_from_trace.py —— oracle 覆盖报告（缺口左移机制③，语义带自校验）。

⚠ 语义警示（rounds/r2 F1，实测证伪）：Mesen2 `addMemoryCallback(fn, "execute", …)` 的字符串
注册虽成功，但回调集合 **≠ 已执行地址集**——实测漏记确证执行过的地址（NMI 入口 $C0D9、
reset 入口 $FFB0-$FFC0 整段），混入非指令入口（JSR 操作数字节、立即数第 2 字节）；特征近似
"总线读周期"。`emu.memCallbackType` 枚举表在 Mesen2 2.1.1 Lua 中不存在（探针实证
tools/probe_exec_callback.lua）。因此本工具**内置 fail-loud 自校验**：

  自校验（不变量）: 采集点 cpu.pc（确证执行过）⊆ 覆盖集
  不过 ⇒ mode 降级 unverified-exec-callback，**拒绝产出盲区差集**（禁止派生任务卡）；
  通过 ⇒ mode=execute-callback-verified（方可作盲区线索，任务卡仍需人工确认）。

覆盖来源分级（显式，不静默降级）：
  execute-callback-verified   自校验通过的 execute 回调
  unverified-exec-callback   自校验未过——仅存档，不得用于盲区派工
  capture-pc-sampled         退化为采集点 cpu.pc 采样（粗粒度）
  unsupported                无任何来源（报告为空，不冒充）

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
    idx: dict[str, tuple[str, str]] = {}
    if not SEED.is_file():
        return idx
    for line in SEED.read_text(encoding="utf-8").splitlines():
        m = SEED_LINE.match(line)
        if m and m.group(1) == "07":
            idx[m.group(2)] = ("07", m.group(2))
    return idx


def load_pc_samples(name: str) -> set[int]:
    """采集点 cpu.pc（确证执行过的地址）——自校验的不变量左部。"""
    pcs: set[int] = set()
    caps = TRACES / f"{name}.captures.jsonl"
    if caps.is_file():
        for line in caps.read_text(encoding="utf-8").splitlines():
            rec = json.loads(line)
            cpu = rec.get("cpu") or {}
            pc = cpu.get("pc")
            if isinstance(pc, int):
                pcs.add(pc)
    return pcs


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
    caps = TRACES / f"{name}.captures.jsonl"
    addrs = set()
    if caps.is_file():
        for line in caps.read_text(encoding="utf-8").splitlines():
            rec = json.loads(line)
            cpu = rec.get("cpu") or {}
            pc = cpu.get("pc")
            if isinstance(pc, int):
                addrs.add(pc)
    return ("capture-pc-sampled" if addrs else "unsupported"), addrs


def main() -> int:
    names = sys.argv[1:]
    if not names:
        print("用法: python3 tools/coverage_from_trace.py <scenario> [...]")
        return 2
    seed_idx = load_seed_index()
    union: set[int] = set()
    if UNION.is_file():
        u = json.loads(UNION.read_text(encoding="utf-8"))
        union = {int(a, 16) for a in u.get("addresses", [])}

    rc = 0
    for name in names:
        mode, addrs = read_coverage(name)
        pcs = load_pc_samples(name)
        verified = False
        if mode not in ("execute-callback", "execute-callback-verified"):
            print(f"[{name}] ⚠ 覆盖来源 {mode}——不产出盲区差集（不冒充）")
            if mode == "unsupported":
                rc = 1
                continue
        else:
            # fail-loud 自校验：确证执行过的 pc 必须在覆盖集内
            missing = sorted(p for p in pcs if p not in addrs)
            if pcs and not missing:
                mode, verified = "execute-callback-verified", True
                print(f"[{name}] 自校验 PASS: {len(pcs)} 个采集点 pc ⊆ 覆盖集")
            else:
                mode = "unverified-exec-callback"
                rc = 1
                print(f"[{name}] 自校验 FAIL: {len(missing)}/{len(pcs)} 个确证执行过的 pc ∉ 覆盖集"
                      f"（如 {', '.join(f'{p:04X}' for p in missing[:6])}）"
                      "→ 降级 unverified，**拒绝产出盲区差集**（rounds/r2 F1）")
        fixed = sorted(a for a in addrs if a >= 0xC000)
        window = sorted(a for a in addrs if 0x8000 <= a < 0xC000)
        seed_hits = sorted({seed_idx[f"{a:04X}"] for a in fixed if f"{a:04X}" in seed_idx})
        report = {
            "name": name, "mode": mode, "self_check": "pass" if verified else (
                "fail" if mode == "unverified-exec-callback" else "n/a"),
            "pc_samples": len(pcs),
            "distinct_addresses": len(addrs),
            "fixed_bank7": {"count": len(fixed), "seed_lines_hit": len(seed_hits)},
            "window_8000": {"count": len(window), "bank_attribution": "unknown（可切窗口，不冒充归属）"},
            "blind_spot_diff": "allowed" if verified else "refused（未验证语义，禁止派生任务卡）",
        }
        (TRACES / f"{name}.coverage-report.json").write_text(
            json.dumps(report, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
        print(f"[{name}] mode={mode} distinct={len(addrs)} "
              f"(bank7 fixed={len(fixed)} → seed 行 {len(seed_hits)}；window={len(window)} 归属未知)")
        if verified:
            union |= addrs
        else:
            print(f"[{name}] 并集不吸收未验证集合（防污染只增不减语料）")

    if union:
        UNION.write_text(json.dumps(
            {"addresses": [f"{a:04X}" for a in sorted(union)], "count": len(union)},
            ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(f"并集累计: {len(union)} 个不同地址（data/status/coverage_union.json，只增不减，仅吸收已验证集合）")
    return rc


if __name__ == "__main__":
    raise SystemExit(main())
