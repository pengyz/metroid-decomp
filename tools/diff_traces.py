#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""tools/diff_traces.py —— L3 帧级状态向量差分（trace-diff 门禁入口）。

比对面：逐帧状态向量 = watch 字段（zp/iram hex 等，见 case watch 配置）。
两侧语义（pipeline §5）：
  左  oracle：Mesen2 场景 trace（`analysis/traces/<scenario>.frames.jsonl`，可再生）
  右  game-core：Rust 帧导出（`analysis/dump/<scenario>.frames.jsonl`，S5 落地后生成）
完整性设计（红线：ROM 衍生 trace 不入库）：
  `oracle/traces/l3_manifest.json` 只存**摘要**（sha256/帧数/watch 配置）；trace 本体在 analysis/。
  每次运行：缓存缺失 ⇒ 经 tools/scenario_run.py 重采集（响亮再生）⇒ sha256 对 manifest
  （**持续复验 oracle 确定性**——manifest 即确定性基线）。

裁定规则：
  - manifest 缺失/为空 ⇒ exit 2（空跑不得绿，r3 F7）
  - 缓存 sha 与 manifest 不符 ⇒ FAIL（确定性破裂/陈旧缓存/篡改）
  - game-core dump 存在 ⇒ 逐帧逐地址比对，报告首个分歧（帧/地址/两侧值）
  - game-core dump 不存在 ⇒ 该 case 记「oracle 腿已验，game-core 腿未启用（S5 后自动启用）」
用法：python3 tools/diff_traces.py
"""
from __future__ import annotations

import hashlib
import json
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
MANIFEST = ROOT / "oracle" / "traces" / "l3_manifest.json"
TRACES = ROOT / "analysis" / "traces"
DUMPS = ROOT / "analysis" / "dump"


def regen(scenario: str) -> bool:
    print(f"[regen] 缓存缺失——经 tools/scenario_run.py 重采集 {scenario}（响亮再生）")
    proc = subprocess.run(
        [sys.executable, str(ROOT / "tools" / "scenario_run.py"), "run", scenario, "--timeout", "300"],
        cwd=ROOT, capture_output=True, text=True)
    return proc.returncode == 0


def load_frames(path: Path) -> dict[int, dict]:
    out = {}
    for line in path.read_text(encoding="utf-8").splitlines():
        r = json.loads(line)
        out[r["f"]] = r["w"]
    return out


def compare_frames(left: dict, right: dict, watch: list[str], case_id: str) -> str | None:
    """逐帧逐 watch 字段比对；返回首分歧描述（None=全同）。"""
    lframes, rframes = sorted(left), sorted(right)
    if lframes != rframes:
        only_l = sorted(set(lframes) - set(rframes))[:5]
        only_r = sorted(set(rframes) - set(lframes))[:5]
        return f"帧集合不一致（左缺={only_r} 右缺={only_l}）"
    for f in lframes:
        for w in watch:
            a, b = left[f].get(w), right[f].get(w)
            if a != b:
                if isinstance(a, str) and isinstance(b, str):
                    diff_at = next((i for i in range(0, min(len(a), len(b)), 2) if a[i:i+2] != b[i:i+2]), None)
                    addr = f"{w}+0x{diff_at // 2:03X}" if diff_at is not None else w
                    return f"frame {f} 字段 {w} 分歧 @{addr}: 左={a[diff_at:diff_at+8] if diff_at is not None else '?'}…"
                return f"frame {f} 字段 {w} 分歧: 左={a!r} 右={b!r}"
    return None


def main() -> int:
    if not MANIFEST.is_file():
        print("L3 帧级: l3_manifest.json 缺失——fail-loud（空跑不得绿）")
        return 2
    cases = json.loads(MANIFEST.read_text(encoding="utf-8")).get("cases", [])
    if not cases:
        print("L3 帧级: 用例集为空——fail-loud（r3 F7 口径）")
        return 2

    failed = 0
    for case in cases:
        cid, scenario = case["id"], case["scenario"]
        cache = TRACES / f"{scenario}.frames.jsonl"
        if not cache.is_file():
            if not regen(scenario) or not cache.is_file():
                print(f"[FAIL] {cid}: oracle trace 重采集失败")
                failed += 1
                continue
        digest = hashlib.sha256(cache.read_bytes()).hexdigest()
        if digest != case["oracle_sha256"]:
            print(f"[FAIL] {cid}: oracle trace sha256 不符 manifest（确定性破裂/缓存陈旧/篡改）")
            failed += 1
            continue

        dump = DUMPS / f"{scenario}.frames.jsonl"
        if not dump.is_file():
            print(f"[PASS] {cid}: oracle 腿已验（sha256 对锚、{case['frames']} 帧）；"
                  f"game-core 腿未启用（S5 落地生成 {dump.relative_to(ROOT)} 后自动启用）")
            continue

        left = load_frames(cache)
        right = load_frames(dump)
        divergence = compare_frames(left, right, case.get("watch", ["zp", "iram"]), cid)
        if divergence:
            print(f"[FAIL] {cid}: 首分歧 {divergence}")
            failed += 1
        else:
            print(f"[PASS] {cid}: 逐帧状态向量全同（{len(left)} 帧）")

    total = len(cases)
    print(f"L3 帧级裁定: {total} 条用例, {failed} 失败")
    return 1 if failed else 0


if __name__ == "__main__":
    raise SystemExit(main())
