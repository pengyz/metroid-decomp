#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""tools/tests/test_nes_map.py —— `nes-map` 静态腿 TDD（先红后绿）。

判定逻辑单源 = `tools/nes_map_check.py`（门禁只做 IO/退出码，见 `gates/cli.py: gate_nes_map`）。
覆盖 Task 4 brief 的三态与三条作弊路径：

1. Owner 重叠 ⇒ FAIL（`overlap $0307`，brief Step 1 原例）；
2. Unmodeled 抢 Owner/View/其他 Unmodeled 的位 ⇒ FAIL；
3. View 必须叠在 Owner 上（悬空 View ⇒ FAIL；同义多名 View 允许）；
4. 比对面内无任何声明 ⇒ Undeclared；账本必须**精确等于**实际空洞
   （账本外新空洞 / 已声明却仍记账 ⇒ 双向 FAIL）；
5. 每段 Undeclared 必须有非空、非敷衍、带代码指针的 reason；
6. `--write` 只许下调：上调拒绝、等长换洞拒绝、缩洞继承旧 reason；
7. 账本 history 严格递减 + 尾条 total/geometry 与当前一致（手工改账本可检出）。
"""
from __future__ import annotations

import copy
import json
import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parent.parent.parent
sys.path.insert(0, str(ROOT / "tools"))

import nes_map_check as nm  # noqa: E402

FACE = [{"start": 0, "len": 8}]


def span(entity: str, start: int, length: int, key: str = "owner", note=None) -> dict:
    return {"entity": entity, "start": start, "len": length, "key": key, "note": note}


def make_ledger(entries: list[dict], face=FACE, owner_bytes=(0, 0)) -> dict:
    """按实现契约造一份自洽账本（history 尾条 = 当前 total/geometry/owner_bytes）。"""
    total = sum(int(e["len"]) for e in entries)
    return {
        "schema": nm.SCHEMA,
        "face": copy.deepcopy(face),
        "face_total": sum(int(f["len"]) for f in face),
        "owner_bytes": list(owner_bytes),
        "undeclared": entries,
        "undeclared_total": total,
        "history": [{
            "ts": "2026-11-17T00:00:00+08:00",
            "total": total,
            "geometry_sha256": nm.geometry_sha256(entries),
            "owner_bytes": list(owner_bytes),
            "reason": "genesis",
            "prev_sha256": "0" * 64,
        }],
    }


# ---------------- 1. Owner 重叠（brief Step 1 原例） ----------------

def test_overlap_is_rejected():
    spans = [span("SamusState", 0x0300, 8), span("Object", 0x0307, 1)]
    errs = nm.violations(spans)
    assert len(errs) == 1, errs
    assert "overlap $0307" in errs[0], errs[0]


# ---------------- 2. Unmodeled 不得抢位 ----------------

def test_unmodeled_cannot_overlap_owner():
    spans = [span("SamusState", 0, 4), span("Legacy", 2, 2, "unmodeled", "旧面")]
    errs = nm.violations(spans)
    assert any("unmodeled-conflict $0002" in e for e in errs), errs


def test_unmodeled_cannot_overlap_view():
    spans = [span("SamusState", 0, 4), span("Obj", 0, 4, "view", "SamusState"),
             span("Legacy", 1, 1, "unmodeled", "旧面")]
    errs = nm.violations(spans)
    assert any("unmodeled-conflict $0001" in e for e in errs), errs


def test_unmodeled_cannot_overlap_unmodeled():
    spans = [span("A", 0, 4, "unmodeled", "旧面"), span("B", 3, 1, "unmodeled", "旧面")]
    errs = nm.violations(spans)
    assert any("unmodeled-conflict $0003" in e for e in errs), errs


# ---------------- 3. View 必须叠在 Owner 上 ----------------

def test_view_without_owner_is_rejected():
    errs = nm.violations([span("Obj", 0, 4, "view", "SamusState")])
    assert any("orphan-view $0000" in e for e in errs), errs


def test_synonym_views_over_same_owner_are_allowed():
    """§4 形态 1：同义多名 View 是合法形态（$0307 同时是 SamusOnElevator/DoorType）。"""
    spans = [span("SamusState", 0, 4), span("Obj", 0, 4, "view", "SamusState"),
             span("Door", 3, 1, "view", "SamusState")]
    assert nm.violations(spans) == []


# ---------------- 4. Undeclared 三态 ----------------

def test_undeclared_hole_is_listed():
    assert nm.undeclared_segments([span("A", 0, 4)], FACE) == [{"start": 4, "len": 4}]


def test_ledger_extra_hole_is_rejected():
    """作弊②/③：账本外新空洞（含手工把新洞写进账本但不改 history）必须红。"""
    spans = [span("A", 0, 4)]
    ledger = make_ledger([], owner_bytes=(4,))
    errs = nm.violations(spans, FACE, ledger)
    assert any("new-undeclared $0004" in e for e in errs), errs


def test_ledger_stale_hole_is_rejected():
    """收益未锁定（声明已覆盖，账本还记着）也必须红——否则账本可被当成假真相。"""
    spans = [span("A", 0, 8)]
    ledger = make_ledger([{"start": 4, "len": 4, "reason": "旧洞（x.rs:1 直写）"}], owner_bytes=(8,))
    errs = nm.violations(spans, FACE, ledger)
    assert any("stale-undeclared $0004" in e for e in errs), errs


# ---------------- 5. reason 纪律 ----------------

@pytest.mark.parametrize("reason", ["", "   ", "TODO", "unknown", "待定", "?"])
def test_placeholder_reason_is_rejected(reason):
    ledger = make_ledger([{"start": 4, "len": 4, "reason": reason}], owner_bytes=(4,))
    errs = nm.violations([span("A", 0, 4)], FACE, ledger)
    assert any("bad-reason $0004" in e for e in errs), errs


def test_reason_must_point_at_code_location():
    ledger = make_ledger([{"start": 4, "len": 4, "reason": "这里没人管"}], owner_bytes=(4,))
    errs = nm.violations([span("A", 0, 4)], FACE, ledger)
    assert any("bad-reason $0004" in e for e in errs), errs


def test_reason_with_code_pointer_is_accepted():
    ledger = make_ledger([{"start": 4, "len": 4, "reason": "由 world/update.rs 的门名表计数直写"}], owner_bytes=(4,))
    assert nm.violations([span("A", 0, 4)], FACE, ledger) == []


# ---------------- 6. --write 只许下调 ----------------

def test_write_refuses_raise():
    old = make_ledger([{"start": 0, "len": 4, "reason": "旧洞（x.rs:1 直写）"}], owner_bytes=(2,))
    dump = {"face": FACE, "spans": [span("A", 0, 2)], "coverage": [2, 0]}
    with pytest.raises(ValueError, match="上调"):
        nm.make_ledger(dump, old, reason="试图放大")


def test_write_refuses_equal_total_geometry_change():
    """等长换洞（洞从 0..4 挪到 4..8）不是下调，必须拒绝——否则可用同长换洞洗白。"""
    old = make_ledger([{"start": 0, "len": 4, "reason": "旧洞（x.rs:1 直写）"}], owner_bytes=(4,))
    dump = {"face": FACE, "spans": [span("A", 0, 4)], "coverage": [4, 0]}
    with pytest.raises(ValueError, match="等长换洞"):
        nm.make_ledger(dump, old, reason="换洞")


def test_write_carries_reason_when_hole_shrinks():
    old = make_ledger([{"start": 0, "len": 8, "reason": "旧洞（x.rs:1 直写）"}], owner_bytes=(4,))
    dump = {"face": FACE, "spans": [span("A", 0, 4)], "coverage": [4, 0]}
    new = nm.make_ledger(dump, old, reason=None)
    assert new["undeclared"] == [{"start": 4, "len": 4, "reason": "旧洞（x.rs:1 直写）"}]
    assert new["undeclared_total"] == 4
    assert len(new["history"]) == 2
    assert new["history"][-1]["total"] == 4
    assert new["history"][-1]["prev_sha256"] == nm.entry_sha256(new["history"][0])
    # 新账本必须自洽通过
    assert nm.violations(dump["spans"], FACE, new) == []


def test_write_refuses_new_hole_without_reason():
    # 旧洞在 0..4、新洞在 4..8（总长不变但几何变化）；两洞仅在边界相接 ⇒ 无 reason 可继承。
    old = make_ledger([{"start": 0, "len": 4, "reason": "旧洞（x.rs:1 直写）"}], owner_bytes=(4,))
    dump = {"face": FACE, "spans": [span("A", 0, 4)], "coverage": [4, 0]}
    with pytest.raises(ValueError, match="无 reason"):
        nm.make_ledger(dump, old, reason=None)


# ---------------- 7. history 防手工改账 ----------------

def test_history_tamper_is_detected():
    ledger = make_ledger([{"start": 4, "len": 4, "reason": "由 world/update.rs 直写"}], owner_bytes=(4,))
    bad = copy.deepcopy(ledger)
    bad["history"][-1]["total"] = 8  # 手工把尾条 total 改大（掩盖新洞）
    errs = nm.violations([span("A", 0, 4)], FACE, bad)
    assert any("history" in e for e in errs), errs


def test_history_missing_is_rejected():
    ledger = make_ledger([{"start": 4, "len": 4, "reason": "由 world/update.rs 直写"}], owner_bytes=(4,))
    ledger["history"] = []
    errs = nm.violations([span("A", 0, 4)], FACE, ledger)
    assert any("history" in e for e in errs), errs


def test_history_prev_sha_chain_is_checked():
    ledger = make_ledger([{"start": 4, "len": 4, "reason": "由 world/update.rs 直写"}], owner_bytes=(6,))
    ledger["history"].append({
        "ts": "2026-11-18T00:00:00+08:00", "total": 2,
        "geometry_sha256": nm.geometry_sha256([{"start": 4, "len": 2}]),
        "owner_bytes": [6],
        "reason": "下调", "prev_sha256": "f" * 64,
    })
    ledger["undeclared"] = [{"start": 4, "len": 2, "reason": "由 world/update.rs 直写"}]
    ledger["undeclared_total"] = 2
    errs = nm.violations([span("A", 0, 6)], FACE, ledger)
    assert any("prev_sha256" in e for e in errs), errs


# ---------------- 8. 真 dump ↔ 真账本（防实现漂移） ----------------

def test_real_dump_matches_ledger():
    ledger_path = ROOT / "data" / "status" / "nes_map.json"
    if not ledger_path.is_file():
        pytest.skip("账本尚未建立（先跑 python3 tools/nes_map_check.py --write）")
    dump = nm.load_dump()
    ledger = json.loads(ledger_path.read_text(encoding="utf-8"))
    assert nm.violations(dump["spans"], dump["face"], ledger) == []
    assert sum(f["len"] for f in dump["face"]) == 2272
