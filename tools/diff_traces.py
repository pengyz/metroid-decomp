#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""tools/diff_traces.py —— L3 帧级状态向量差分（trace-diff 门禁入口）。

比对面：逐帧状态向量 = watch 字段（manifest 声明显式基址区间；hex 大小写归一后比对）。
两侧语义（pipeline §5）：
  左  oracle：Mesen2 场景 trace（`analysis/traces/<scenario>.frames.jsonl`，可再生）
  右  game-core：Rust 帧导出（`analysis/dump/<scenario>.frames.jsonl`，S5 落地后生成）
完整性设计（红线：ROM 衍生 trace 不入库）：
  `oracle/traces/l3_manifest.json` 只存**摘要**；缓存缺失 ⇒ scenario_run 响亮重采集 ⇒
  sha256 对 manifest（持续复验 oracle 确定性）。

裁判铁律（r3 F7「空跑不得绿」+ G5 复审 F1/F2 修正——判定权不留静默绿面）：
  - manifest 缺失/空用例/非法 JSON ⇒ exit 2
  - watch 为空 ⇒ exit 2；watch 字段**双侧存在性**逐一校验，缺任一 ⇒ FAIL
  - 重复帧行（同 f 多行）⇒ FAIL（防后行覆盖分歧行）
  - manifest.frames 与实测帧数不符 ⇒ FAIL
  - 首分歧报告=帧号 + 字段 + **CPU 地址（基址+偏移）** + 双侧值
  - hex 比对大小写归一（.upper()）；game-core 导出格式建议大写（归一后皆可）
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
    if proc.returncode != 0:
        tail = (proc.stdout + proc.stderr).strip().splitlines()[-5:]
        print(f"[regen] 重采集失败: {' | '.join(tail)}")
    return proc.returncode == 0


def load_frames(path: Path) -> tuple[dict[int, dict], list[int]]:
    """读帧 trace；返回 (帧字典, 重复 f 列表)。重复 f 由调用方判 FAIL（防覆盖掩盖分歧）。"""
    out: dict[int, dict] = {}
    dups: list[int] = []
    for line in path.read_text(encoding="utf-8").splitlines():
        r = json.loads(line)
        f = r["f"]
        if f in out:
            dups.append(f)
        out[f] = r["w"]
    return out, dups


def norm_hex(v: str) -> str:
    return v.upper()


def compare_frames(left: dict, right: dict, watch: list[dict], case_id: str) -> str | None:
    """逐帧逐 watch 字段比对；返回首分歧描述（None=全同）。watch=[{label,base(hex)}]。"""
    lframes, rframes = sorted(left), sorted(right)
    if lframes != rframes:
        only_l = sorted(set(lframes) - set(rframes))[:5]
        only_r = sorted(set(rframes) - set(lframes))[:5]
        return f"帧集合不一致（左缺={only_r} 右缺={only_l}）"
    for f in lframes:
        for w in watch:
            label, base = w["label"], int(w["base"], 16)
            a, b = left[f].get(label), right[f].get(label)
            if a is None or b is None:
                side = "左" if a is None else "右"
                return f"frame {f} watch 字段 {label} {side}侧缺失（存在性校验 FAIL）"
            a, b = norm_hex(a), norm_hex(b)
            if a != b:
                if len(a) != len(b):
                    return f"frame {f} 字段 {label} 长度不一致: 左={len(a)//2}B 右={len(b)//2}B（无法偏移定位）"
                off = next((i for i in range(0, len(a), 2) if a[i:i+2] != b[i:i+2]), None)
                if off is None:
                    return f"frame {f} 字段 {label} 分歧（归一后仍不等但无字节位差，疑结构差异）"
                byte_i = off // 2
                return (f"frame {f} 字段 {label} 分歧 @{label}(${base:04X})+0x{byte_i:03X}"
                        f"=CPU ${base + byte_i:04X}: 左={a[off:off+8]}… 右={b[off:off+8]}…")
    return None


def main() -> int:
    if not MANIFEST.is_file():
        print("L3 帧级: l3_manifest.json 缺失——fail-loud（空跑不得绿）")
        return 2
    try:
        cases = json.loads(MANIFEST.read_text(encoding="utf-8")).get("cases", [])
    except json.JSONDecodeError as e:
        print(f"L3 帧级: l3_manifest.json 非法 JSON——fail-loud: {e}")
        return 2
    if not cases:
        print("L3 帧级: 用例集为空——fail-loud（r3 F7 口径）")
        return 2

    failed = 0
    for case in cases:
        cid, scenario = case["id"], case["scenario"]
        watch = case.get("watch", [])
        if not watch:
            print(f"[FAIL] {cid}: watch 为空——空跑不得绿（G5 复审 F1）")
            failed += 1
            continue
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

        left, ldups = load_frames(cache)
        if ldups:
            print(f"[FAIL] {cid}: oracle trace 重复帧行 {sorted(set(ldups))[:5]}（防覆盖掩盖分歧）")
            failed += 1
            continue
        # watch 字段存在性：对 oracle 侧先行校验（dump 未启用时同样不许空跑绿）
        missing_left = sorted({w["label"] for w in watch} - set(next(iter(left.values()), {})))
        if missing_left:
            print(f"[FAIL] {cid}: watch 字段在 oracle trace 缺失 {missing_left}（字段名校验 FAIL）")
            failed += 1
            continue
        if "frames" in case and len(left) != case["frames"]:
            print(f"[FAIL] {cid}: 帧数 {len(left)} != manifest.frames {case['frames']}")
            failed += 1
            continue

        dump = DUMPS / f"{scenario}.frames.jsonl"
        if not dump.is_file():
            print(f"[PASS] {cid}: oracle 腿已验（sha256 对锚、{len(left)} 帧）；"
                  f"game-core 腿未启用（S5 落地生成 {dump.relative_to(ROOT)} 后自动启用）")
            continue

        right, rdups = load_frames(dump)
        if rdups:
            print(f"[FAIL] {cid}: game-core dump 重复帧行 {sorted(set(rdups))[:5]}（防覆盖掩盖分歧）")
            failed += 1
            continue
        divergence = compare_frames(left, right, watch, cid)
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
