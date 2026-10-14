#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""tools/completeness_denoms.py —— 完备性矩阵分母机械导出器（缺口左移机制①）。

原则（AGENTS.md D7）：**分母独立于测试、机械可导出**。本工具只登记分母（L0 入册未验证），
不产生任何覆盖分子；分子登记由角色任务带证据更新（ratchet：只升不降）。

分母来源：
  S 符号面  analysis/ida/bank*.json 的 funcs（IDA 机械 entry 切分；标注"待 trace 纠偏"）
  D 数据面  analysis/ida/bank*.json 的 data items 连续段 + xref 消费者计数
  B 行为面  oracle/scenarios/*.txt 场景清单（随 oracle 覆盖增长，见 coverage_from_trace.py）
  R 规则面  docs/spec/ 规则条目（S4 起；空册起步）

用法：python3 tools/completeness_denoms.py [--force]
  幂等：已存在的格子保留（不降级、不覆盖带证据的分子登记）；只追加新 id。
  --force：完全重建（丢弃已有格子；仅限分母口径变更，需 commit 说明）。
"""
from __future__ import annotations

import argparse
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
IDA_DIR = ROOT / "analysis" / "ida"
SCEN_DIR = ROOT / "oracle" / "scenarios"
OUT = ROOT / "data" / "status" / "completeness_matrix.json"

N_BANKS = 8
BANK_SIZE = 0x4000
BANK_ROLE = {0: "title", 1: "brinstar", 2: "norfair", 3: "tourian",
             4: "kraid", 5: "ridley", 6: "graphics", 7: "engine"}
# D 面连续段合并阈值（字节）：小于等于该间隔的数据项并入同段
DATA_MERGE_GAP = 16


def load_matrix() -> dict:
    if OUT.is_file():
        return json.loads(OUT.read_text(encoding="utf-8"))
    return {
        "schema_version": 1,
        "source_of_truth": "docs/spec/completeness.md（本文件由 tools/completeness_denoms.py 导出分母，角色任务登记分子）",
        "id_rule": "B=行为面 / S=符号面 / D=数据面 / R=规则面；格 ID 一经入分母永不复用",
        "strength_scale": "L0=入册未验证 L1=有证据指针 L2=子程序级对拍过 L3=帧级对拍过 L4=视觉/流程过",
        "covered_rule": "covered ⇔ strength >= L1（带证据指针的分子登记；L0=入分母未验证）",
        "cell_schema": ["id", "face", "module", "strength", "covered", "evidence", "note"],
        "faces": {"behavior": {}, "symbol": {}, "data": {}, "rule": {}},
        "cells": {},
    }


def cell(cid: str, face: str, module: str, evidence: str, note: str) -> dict:
    return {"id": cid, "face": face, "module": module, "strength": "L0",
            "covered": False, "evidence": evidence, "note": note}


def denoms_symbol_data() -> tuple[list[dict], list[dict]]:
    s_cells, d_cells = [], []
    if not IDA_DIR.is_dir():
        print("⚠ analysis/ida/ 不存在：S/D 面分母为空（先跑 tools/ida/run_all_banks.py）")
        return s_cells, d_cells
    for k in range(N_BANKS):
        p = IDA_DIR / f"bank{k:02d}.json"
        if not p.is_file():
            continue
        data = json.loads(p.read_text(encoding="utf-8"))
        base = data["base"]
        role = BANK_ROLE.get(k, "?")
        # S 面：机械 funcs（同 start 的别名入口去重：多个 entry_start 落在同一函数体
        # 时 IDA get_func 返回同一函数多次——分母单元 = 不同函数体，实测 585 入口→450 体）
        seen_funcs: set[int] = set()
        for f in data.get("funcs", []):
            st, en = f["start"], f["end"]
            if st in seen_funcs:
                continue
            seen_funcs.add(st)
            s_cells.append(cell(
                f"S-FUNC-{k:02d}-{st:04X}", "symbol", role,
                f"bank{k:02d}:{st:04X}-{en:04X}（IDA 机械切分 size={f['size']}）",
                "机械切分待纠偏（rounds/r1）：语义边界以 trace 证据为准"))
        # D 面：data items 连续段
        items = [i for i in data.get("items", []) if i["kind"] == "data"]
        segs: list[list[int]] = []  # [start, end)
        for it in items:
            st, en = it["ea"], it["ea"] + it["size"]
            if segs and st - segs[-1][1] <= DATA_MERGE_GAP:
                segs[-1][1] = en
            else:
                segs.append([st, en])
        xrefs = data.get("xrefs", [])
        for n, (st, en) in enumerate(segs):
            consumers = sorted({x["from"] for x in xrefs if st <= x["to"] < en})
            d_cells.append(cell(
                f"D-REG-{k:02d}-{n:03d}", "data", role,
                f"bank{k:02d}:{st:04X}-{en:04X}（data 连续段 {en - st}B）",
                f"xref 消费者 {len(consumers)} 处（机械口径）；字段切分待 S3 考证"))
    return s_cells, d_cells


def denoms_behavior() -> list[dict]:
    cells = []
    if SCEN_DIR.is_dir():
        for p in sorted(SCEN_DIR.glob("*.txt")):
            cells.append(cell(
                f"B-SCN-{p.stem}", "behavior", "scenario",
                f"oracle/scenarios/{p.name}",
                "场景语料入口；行为面分母随 oracle 覆盖增长（coverage_from_trace.py）"))
    return cells


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--force", action="store_true", help="完全重建（丢弃已有格子）")
    args = ap.parse_args()

    m = load_matrix()
    cells = m["cells"] if not args.force else {}
    s_cells, d_cells = denoms_symbol_data()
    b_cells = denoms_behavior()

    added = kept = 0
    for c in s_cells + d_cells + b_cells:
        if c["id"] in cells:
            kept += 1  # 已有格子保留（可能已被分子登记升级）
        else:
            cells[c["id"]] = c
            added += 1

    m["cells"] = cells
    by_face = {"behavior": 0, "symbol": 0, "data": 0, "rule": 0}
    covered = 0
    for c in cells.values():
        by_face[c["face"]] = by_face.get(c["face"], 0) + 1
        covered += 1 if c.get("covered") else 0
    m["faces"] = {f: {"denominator": n} for f, n in by_face.items()}

    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(json.dumps(m, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(f"分母导出: +{added} 新格 / {kept} 保留；"
          f"S={by_face['symbol']} D={by_face['data']} B={by_face['behavior']} R={by_face['rule']}；"
          f"covered={covered}（分母只登记 L0，分子由角色任务带证据升级）")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
