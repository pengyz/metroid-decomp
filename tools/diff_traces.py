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

裁判铁律（r3 F7「空跑不得绿」+ G5 复审 F1/F2 + 二轮 NEW-1/2/3——判定权不留静默绿面）：
  exit 2（manifest schema 级，先于一切比对）：
  - manifest 缺失 / 非法 JSON / cases 非 list[dict] / case 缺必填键或类型错
  - watch 为空 / watch 条目非 dict / label 非空 str / base 1-4 位 hex / range 格式非法
  exit 1（case 级 FAIL）：
  - watch 字段**逐帧**存在性 + 值合法性（str、非空、偶数长、纯 hex）——oracle 侧先行，dump 侧同检
  - range span 与字段字节长不符 / 重复帧行（同 f 多行）/ manifest.frames 与实测不符
  - sha 不符 / 帧集不一致 / 首分歧
  首分歧报告=帧号 + 字段 + CPU 地址（基址+偏移）+ 双侧值；hex 比对大小写归一（.upper()）。
用法：python3 tools/diff_traces.py
"""
from __future__ import annotations

import hashlib
import json
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
MANIFEST = ROOT / "oracle" / "traces" / "l3_manifest.json"
TRACES = ROOT / "analysis" / "traces"
DUMPS = ROOT / "analysis" / "dump"

HEX64 = re.compile(r"^[0-9a-fA-F]{64}$")
HEXSTR = re.compile(r"^[0-9a-fA-F]*$")
BASERE = re.compile(r"^[0-9a-fA-F]{1,4}$")
RANGERE = re.compile(r"^([0-9a-fA-F]{1,4})-([0-9a-fA-F]{1,4})$")


def schema_error(msg: str) -> int:
    print(f"L3 帧级: manifest schema 非法——fail-loud（exit 2）: {msg}")
    return 2


def check_case_schema(case) -> str | None:
    """manifest schema 机械校验（R1/R3）；返回错误描述或 None。"""
    if not isinstance(case, dict):
        return f"case 非对象: {case!r}"
    for key in ("id", "scenario", "oracle_sha256"):
        v = case.get(key)
        if not isinstance(v, str) or not v.strip():
            return f"case.{key} 缺失或非非空字符串"
    if not HEX64.match(case["oracle_sha256"]):
        return f"case.oracle_sha256 非 64 位 hex: {case['oracle_sha256']!r}"
    if "frames" in case:
        fv = case["frames"]
        if isinstance(fv, bool) or not isinstance(fv, int) or fv <= 0:
            return f"case.frames 须为正整数（缺省可省略，给了就必严）: {fv!r}"
    watch = case.get("watch")
    if not isinstance(watch, list) or not watch:
        return "case.watch 缺失或为空（空跑不得绿）"
    for w in watch:
        if not isinstance(w, dict):
            return f"watch 条目非对象: {w!r}"
        label, base = w.get("label"), w.get("base")
        if not isinstance(label, str) or not label.strip():
            return f"watch.label 缺失或非非空字符串: {w!r}"
        if not isinstance(base, str) or not BASERE.match(base):
            return f"watch.base 须为 1-4 位 hex 字符串: {w!r}"
        rng = w.get("range")
        if rng is not None:
            m = RANGERE.match(rng) if isinstance(rng, str) else None
            if not m:
                return f"watch.range 格式非法（须 'HHHH-HHHH'）: {rng!r}"
            span = int(m.group(2), 16) - int(m.group(1), 16) + 1
            if span <= 0:
                return f"watch.range 区间倒置: {rng!r}"
            w["_span"] = span
    return None


def field_illegal(value) -> str | None:
    """值合法性（R2）：str、非空、偶数长、纯 hex。"""
    if not isinstance(value, str):
        return f"非字符串: {value!r}"
    if not value:
        return "空串（0 字节比对=空跑绿）"
    if len(value) % 2 != 0:
        return f"奇数长度 hex: {value[:20]!r}…"
    if not HEXSTR.match(value):
        return f"非 hex 字符: {value[:20]!r}…"
    return None


def check_frames_side(frames: dict, watch: list[dict], side: str) -> str | None:
    """逐帧存在性 + 值合法性（R2）+ range span 一致性；返回首个违规描述。"""
    for f in sorted(frames):
        w = frames[f]
        for entry in watch:
            label = entry["label"]
            if label not in w:
                return f"frame {f} watch 字段 {label} {side}侧缺失（逐帧存在性校验）"
            bad = field_illegal(w[label])
            if bad:
                return f"frame {f} 字段 {label} {side}侧值违规: {bad}"
            if "_span" in entry and len(w[label]) // 2 != entry["_span"]:
                return (f"frame {f} 字段 {label} {side}侧字节长 {len(w[label]) // 2} "
                        f"!= range span {entry['_span']}")
    return None


def regen(scenario: str) -> bool:
    print(f"[regen] 缓存缺失——经 tools/scenario_run.py 重采集 {scenario}（响亮再生）")
    proc = subprocess.run(
        [sys.executable, str(ROOT / "tools" / "scenario_run.py"), "run", scenario, "--timeout", "300"],
        cwd=ROOT, capture_output=True, text=True)
    if proc.returncode != 0:
        tail = (proc.stdout + proc.stderr).strip().splitlines()[-5:]
        print(f"[regen] 重采集失败: {' | '.join(tail)}")
    return proc.returncode == 0


def load_frames(path: Path) -> tuple[dict[int, dict], list[int], str | None]:
    """读帧 trace；返回 (帧字典, 重复 f 列表, 行级违规)。
    重复 f 判 FAIL（防后行覆盖分歧行）；畸形行（非 JSON/缺键/类型错）= 行级违规。"""
    out: dict[int, dict] = {}
    dups: list[int] = []
    for lineno, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        if not line.strip():
            continue
        try:
            r = json.loads(line)
        except json.JSONDecodeError as e:
            return out, dups, f"第 {lineno} 行非 JSON: {e}"
        if not isinstance(r, dict) or "f" not in r or "w" not in r:
            return out, dups, f"第 {lineno} 行缺 f/w 键或非对象"
        f = r["f"]
        if isinstance(f, bool) or not isinstance(f, int):
            return out, dups, f"第 {lineno} 行 f 非整数: {f!r}"
        if not isinstance(r["w"], dict):
            return out, dups, f"第 {lineno} 行 w 非对象"
        if f in out:
            dups.append(f)
        out[f] = r["w"]
    return out, dups, None


def compare_frames(left: dict, right: dict, watch: list[dict]) -> str | None:
    """逐帧逐 watch 字段比对（值已过合法性校验）；返回首分歧描述或 None。"""
    lframes, rframes = sorted(left), sorted(right)
    if lframes != rframes:
        only_l = sorted(set(lframes) - set(rframes))[:5]
        only_r = sorted(set(rframes) - set(lframes))[:5]
        return f"帧集合不一致（左缺={only_r} 右缺={only_l}）"
    for f in lframes:
        for w in watch:
            label, base = w["label"], int(w["base"], 16)
            a = left[f][label].upper()
            b = right[f][label].upper()
            if a != b:
                if len(a) != len(b):
                    return (f"frame {f} 字段 {label} 长度不一致: 左={len(a) // 2}B 右={len(b) // 2}B"
                            f"（无法偏移定位）")
                off = next((i for i in range(0, len(a), 2) if a[i:i + 2] != b[i:i + 2]), None)
                if off is None:
                    return f"frame {f} 字段 {label} 分歧（归一后仍不等但无字节位差，疑结构差异）"
                byte_i = off // 2
                return (f"frame {f} 字段 {label} 分歧 @{label}(${base:04X})+0x{byte_i:03X}"
                        f"=CPU ${base + byte_i:04X}: 左={a[off:off + 8]}… 右={b[off:off + 8]}…")
    return None


def main() -> int:
    if not MANIFEST.is_file():
        print("L3 帧级: l3_manifest.json 缺失——fail-loud（空跑不得绿）")
        return 2
    try:
        doc = json.loads(MANIFEST.read_text(encoding="utf-8"))
    except json.JSONDecodeError as e:
        return schema_error(f"JSON 解析失败: {e}")
    cases = doc.get("cases") if isinstance(doc, dict) else None
    if not isinstance(cases, list):
        return schema_error(f"cases 须为数组: {type(cases).__name__}")
    if not cases:
        return schema_error("用例集为空（空跑不得绿）")
    for case in cases:
        err = check_case_schema(case)
        if err:
            return schema_error(err)

    failed = 0
    for case in cases:
        cid, scenario = case["id"], case["scenario"]
        watch = case["watch"]
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

        left, ldups, lerr = load_frames(cache)
        if lerr:
            print(f"[FAIL] {cid}: oracle trace {lerr}")
            failed += 1
            continue
        if ldups:
            print(f"[FAIL] {cid}: oracle trace 重复帧行 {sorted(set(ldups))[:5]}（防覆盖掩盖分歧）")
            failed += 1
            continue
        if "frames" in case and len(left) != case["frames"]:
            print(f"[FAIL] {cid}: 帧数 {len(left)} != manifest.frames {case['frames']}")
            failed += 1
            continue
        # oracle 侧逐帧存在性+值合法性（R2；dump 缺席时同样不许空跑绿）
        v = check_frames_side(left, watch, "oracle")
        if v:
            print(f"[FAIL] {cid}: {v}")
            failed += 1
            continue

        dump = DUMPS / f"{scenario}.frames.jsonl"
        if not dump.is_file():
            print(f"[PASS] {cid}: oracle 腿已验（sha256 对锚、{len(left)} 帧、逐帧字段合法）；"
                  f"game-core 腿未启用（S5 落地生成 {dump.relative_to(ROOT)} 后自动启用）")
            continue

        right, rdups, rerr = load_frames(dump)
        if rerr:
            print(f"[FAIL] {cid}: game-core dump {rerr}")
            failed += 1
            continue
        if rdups:
            print(f"[FAIL] {cid}: game-core dump 重复帧行 {sorted(set(rdups))[:5]}（防覆盖掩盖分歧）")
            failed += 1
            continue
        v = check_frames_side(right, watch, "game-core")
        if v:
            print(f"[FAIL] {cid}: {v}")
            failed += 1
            continue
        divergence = compare_frames(left, right, watch)
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
