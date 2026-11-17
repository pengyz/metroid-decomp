#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""tools/tests/test_nes_map.py —— `nes-map` 静态腿 TDD（先红后绿）。

Fix round 1（m00386）把判据从「Undeclared 只减不增」升级为**四态恒等式**：

    Owner + View(only) + Unmodeled + Undeclared == face_total

并封死两条免检通道：
  * Unmodeled 总量只减不增（禁止 Undeclared → Unmodeled 平移）；
  * 既有 Owner span 的 (start,len) 不得变形（禁止加宽 span 吞洞）。

判定逻辑单源 = `tools/nes_map_check.py`；门禁只做 IO/退出码。
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
PTR = "由 world/update.rs:75 的门名表计数直写"


def span(entity: str, start: int, length: int, key: str = "owner", note=None) -> dict:
    return {"entity": entity, "start": start, "len": length, "key": key, "note": note}


def dump_of(spans: list[dict], face=FACE) -> dict:
    return {"schema": nm.DUMP_SCHEMA, "face": copy.deepcopy(face), "spans": spans,
            "coverage": [0, 0]}


def ledger_of(spans: list[dict], face=FACE, reason: str = PTR) -> dict:
    return nm.make_ledger(dump_of(spans, face), reason=reason)


# ---------------- 1. 结构：Owner 重叠 / View 有主 ----------------

def test_overlap_is_rejected():
    spans = [span("SamusState", 0x0300, 8), span("Object", 0x0307, 1)]
    errs = nm.violations(spans)
    assert len(errs) == 1, errs
    assert "overlap $0307" in errs[0], errs[0]


def test_view_without_owner_is_rejected():
    errs = nm.violations([span("Obj", 0, 4, "view", "SamusState")])
    assert any("orphan-view $0000" in e for e in errs), errs


def test_synonym_views_over_same_owner_are_allowed():
    spans = [span("SamusState", 0, 4), span("Obj", 0, 4, "view", "SamusState"),
             span("Door", 3, 1, "view", "SamusState")]
    assert nm.violations(spans) == []


# ---------------- 2. Unmodeled 不得抢位 / 不得包含 Owner ----------------

def test_unmodeled_overlapping_owner_is_rejected():
    spans = [span("SamusState", 0, 4), span("Legacy", 2, 2, "unmodeled", PTR)]
    errs = nm.violations(spans)
    assert any("unmodeled-overlap-owner $0002" in e for e in errs), errs


def test_unmodeled_containing_owner_is_rejected():
    spans = [span("SamusState", 2, 2), span("Legacy", 0, 8, "unmodeled", PTR)]
    errs = nm.violations(spans)
    assert any("unmodeled-contains-owner" in e for e in errs), errs


def test_unmodeled_overlapping_view_is_rejected():
    spans = [span("SamusState", 0, 4), span("Obj", 0, 4, "view", "SamusState"),
             span("Legacy", 1, 1, "unmodeled", PTR)]
    errs = nm.violations(spans)
    assert any("unmodeled-overlap-view $0001" in e for e in errs), errs


def test_unmodeled_overlapping_unmodeled_is_rejected():
    spans = [span("A", 0, 4, "unmodeled", PTR), span("B", 3, 1, "unmodeled", PTR)]
    errs = nm.violations(spans)
    assert any("unmodeled-conflict $0003" in e for e in errs), errs


def test_unmodeled_note_requires_code_pointer():
    spans = [span("A", 0, 4, "unmodeled", "整段未建模")]
    errs = nm.violations(spans)
    assert any("bad-unmodeled-note" in e for e in errs), errs
    spans[0]["note"] = "整段未建模（world/update.rs:75）"
    assert nm.violations(spans) == []


# ---------------- 3. Undeclared 三态 ----------------

def test_undeclared_hole_is_listed():
    assert nm.undeclared_segments([span("A", 0, 4)], FACE) == [{"start": 4, "len": 4}]


def test_ledger_extra_hole_is_rejected():
    ledger = ledger_of([span("A", 0, 8)])  # 账本记的是无洞
    errs = nm.violations([span("A", 0, 4)], FACE, ledger)
    assert any("new-undeclared $0004" in e for e in errs), errs


def test_ledger_stale_hole_is_rejected():
    ledger = ledger_of([span("A", 0, 4)])
    errs = nm.violations([span("A", 0, 8)], FACE, ledger)
    assert any("stale-undeclared $0004" in e for e in errs), errs


# ---------------- 4. reason 纪律 ----------------

@pytest.mark.parametrize("reason", ["", "   ", "TODO", "unknown", "待定", "?"])
def test_placeholder_reason_is_rejected(reason):
    ledger = ledger_of([span("A", 0, 4)])
    ledger["undeclared"][0]["reason"] = reason
    errs = nm.violations([span("A", 0, 4)], FACE, ledger)
    assert any("bad-reason $0004" in e for e in errs), errs


def test_reason_must_point_at_code_location():
    ledger = ledger_of([span("A", 0, 4)])
    ledger["undeclared"][0]["reason"] = "这里没人管"
    errs = nm.violations([span("A", 0, 4)], FACE, ledger)
    assert any("bad-reason $0004" in e for e in errs), errs


def test_reason_with_code_pointer_is_accepted():
    assert nm.violations([span("A", 0, 4)], FACE, ledger_of([span("A", 0, 4)])) == []

def _line_of(path: str, predicate) -> int:
    """在真实文件里找满足谓词的行号（1 基），供行级校验测试使用。"""
    lines = (ROOT / path).read_text(encoding="utf-8", errors="replace").splitlines()
    for i, text in enumerate(lines, start=1):
        if predicate(text):
            return i
    raise AssertionError(f"{path} 找不到满足谓词的行")


def _reason_errs(reason: str) -> list[str]:
    """把 reason 塞进账本空段（0..4 有主、4..8 是洞），返回 `bad-reason` 类错误。"""
    spans = [span("A", 0, 4)]
    ledger = ledger_of(spans)
    ledger["undeclared"][0]["reason"] = reason
    return [e for e in nm.violations(spans, FACE, ledger) if "bad-reason" in e]


def test_reason_pointer_to_empty_line_is_rejected():
    # Fix round 1 机制加固：指针必须落在**真实代码行**上；空行不是证据。
    lineno = _line_of("crates/game-core/src/world/update.rs", lambda t: not t.strip())
    errs = _reason_errs(f"见 world/update.rs:{lineno}")
    assert any("指向空行" in e for e in errs), errs


def test_reason_pointer_to_comment_line_is_rejected():
    # 纯注释行不是证据（`//` 起始行）：这正是旧 reason 把“文档注释”当生产读点的破口。
    lineno = _line_of("crates/game-core/src/world/update.rs",
                      lambda t: t.strip().startswith("//"))
    errs = _reason_errs(f"见 world/update.rs:{lineno}")
    assert any("指向纯注释行" in e for e in errs), errs


def test_reason_pointer_out_of_range_is_rejected():
    lines = (ROOT / "crates/game-core/src/world/update.rs").read_text(
        encoding="utf-8", errors="replace").splitlines()
    errs = _reason_errs(f"见 world/update.rs:{len(lines) + 5}")
    assert any("行号越界" in e for e in errs), errs


def test_reason_pointer_without_line_is_rejected():
    # 只给文件名等于没给证据（旧判据 `\.(rs|py)\b` 的免检口）。
    errs = _reason_errs("写点分散在 world/update.rs 与 frame/nmi.rs")
    assert any("缺 `path:line`" in e for e in errs), errs


def test_asm_pointer_requires_prefix_and_accepts_comment_line():
    # refs/ 下汇编必须带 `asm:` 前缀；汇编注释行（`;` 起始）本身承载地址锚，算证据。
    assert nm.reason_pointer_errors("命名见 refs/m1disasm/SRC/constants_ram.asm:568") != []
    assert nm.reason_pointer_errors(
        "命名见 asm:refs/m1disasm/SRC/constants_ram.asm:568") == []


# ---------------- 5. 四态恒等式 ----------------

def test_ledger_identity_mismatch_is_rejected():
    ledger = ledger_of([span("A", 0, 4)])
    ledger["unmodeled_total"] += 1  # 破坏 Owner+View+Unmodeled+Undeclared == face_total
    errs = nm.violations([span("A", 0, 4)], FACE, ledger)
    assert any("恒等式" in e for e in errs), errs


def test_history_identity_mismatch_is_rejected():
    ledger = ledger_of([span("A", 0, 4)])
    ledger["history"][0]["undeclared_total"] += 1
    errs = nm.violations([span("A", 0, 4)], FACE, ledger)
    assert any("恒等式" in e for e in errs), errs


# ---------------- 6. Unmodeled 只减不增 / --write 下调 ----------------

def test_write_refuses_unmodeled_increase():
    old = ledger_of([span("A", 0, 8)])  # 无空洞、无 Unmodeled
    new_dump = dump_of([span("Legacy", 0, 8, "unmodeled", PTR)])
    with pytest.raises(ValueError, match="Unmodeled 只减不增"):
        nm.make_ledger(new_dump, old, reason="试图整段标 Unmodeled")


def test_write_refuses_undeclared_to_unmodeled_shift():
    old = ledger_of([span("A", 0, 4)])  # 4 B Undeclared
    new_dump = dump_of([span("A", 0, 4), span("Legacy", 4, 4, "unmodeled", PTR)])
    with pytest.raises(ValueError, match="Unmodeled 只减不增"):
        nm.make_ledger(new_dump, old, reason="平移")


def test_write_allows_unmodeled_to_owner():
    old = ledger_of([span("A", 4, 4), span("Legacy", 0, 4, "unmodeled", PTR)])
    new_dump = dump_of([span("A", 4, 4), span("B", 0, 2), span("Legacy", 2, 2, "unmodeled", PTR)])
    new = nm.make_ledger(new_dump, old, reason="typed 化")
    assert new["unmodeled_total"] == 2
    assert sum(new["owner_bytes"]) == 6
    assert new["undeclared_total"] == 0
    assert nm.violations(new_dump["spans"], FACE, new) == []


def test_write_refuses_widening_existing_owner_span():
    old = ledger_of([span("A", 0, 4)])  # 洞 4..8
    new_dump = dump_of([span("A", 0, 8)])  # 加宽 Owner 吞洞
    with pytest.raises(ValueError, match="既有 Owner span 消失/变形"):
        nm.make_ledger(new_dump, old, reason="加宽")


def test_write_allows_new_owner_span_from_undeclared():
    old = ledger_of([span("A", 0, 4)])
    new_dump = dump_of([span("A", 0, 4), span("B", 4, 4)])
    new = nm.make_ledger(new_dump, old, reason="新增 typed 字段")
    assert new["undeclared_total"] == 0
    assert sum(new["owner_bytes"]) == 8


def test_write_refuses_new_hole_without_reason():
    old = ledger_of([span("A", 0, 8)])
    new_dump = dump_of([span("A", 0, 4)])
    with pytest.raises(ValueError, match="无 reason"):
        nm.make_ledger(new_dump, old, reason=None)


def test_write_carries_reason_when_hole_shrinks():
    old = ledger_of([span("A", 0, 4)])
    # 洞 4..8 缩小到 6..8：新增 Owner span 4..2 盖住洞的前半，旧 Owner span (0,4) 不变。
    new_dump = dump_of([span("A", 0, 4), span("B", 4, 2)])
    new = nm.make_ledger(new_dump, old, reason=None)
    assert new["undeclared"] == [{"start": 6, "len": 2, "reason": old["undeclared"][0]["reason"]}]
    assert new["undeclared_total"] == 2
    assert len(new["history"]) == 2
    assert new["history"][-1]["prev_sha256"] == nm.entry_sha256(new["history"][0])
    assert nm.violations(new_dump["spans"], FACE, new) == []


# ---------------- 7. history 防手工改账 ----------------

def test_history_tamper_is_detected():
    ledger = ledger_of([span("A", 0, 4)])
    ledger["undeclared"][0]["reason"] = PTR + "（改）"
    errs = nm.violations([span("A", 0, 4)], FACE, ledger)
    assert any("state_sha256" in e or "history" in e for e in errs), errs


def test_history_missing_is_rejected():
    ledger = ledger_of([span("A", 0, 4)])
    ledger["history"] = []
    errs = nm.violations([span("A", 0, 4)], FACE, ledger)
    assert any("history" in e for e in errs), errs


def test_history_prev_sha_chain_is_checked():
    old = ledger_of([span("A", 0, 4)])
    new_dump = dump_of([span("A", 0, 4), span("B", 4, 4)])
    new = nm.make_ledger(new_dump, old, reason="typed 化")
    new["history"][-1]["prev_sha256"] = "f" * 64
    errs = nm.violations(new_dump["spans"], FACE, new)
    assert any("prev_sha256" in e for e in errs), errs


def test_history_unmodeled_increase_is_rejected():
    ledger = ledger_of([span("A", 0, 4)])
    entry = copy.deepcopy(ledger["history"][-1])
    entry["unmodeled_total"] += 1
    entry["reason"] = "手工加 Unmodeled"
    entry["prev_sha256"] = nm.entry_sha256(ledger["history"][-1])
    entry["state_sha256"] = "f" * 64
    ledger["history"].append(entry)
    errs = nm.violations([span("A", 0, 4)], FACE, ledger)
    assert any("Unmodeled 只减不增" in e for e in errs), errs


# ---------------- 8. 真 dump ↔ 真账本（防实现漂移） ----------------

def test_real_dump_matches_ledger():
    ledger_path = ROOT / "data" / "status" / "nes_map.json"
    if not ledger_path.is_file():
        pytest.skip("账本尚未建立（先跑 python3 tools/nes_map_check.py --write）")
    dump = nm.load_dump()
    ledger = json.loads(ledger_path.read_text(encoding="utf-8"))
    assert nm.violations(dump["spans"], dump["face"], ledger) == []
    assert sum(f["len"] for f in dump["face"]) == 2272
    # 绝对值不写死：Owner 随 typed 域增长，恒等式才是判据（旧账 107/612/1553 已随 Task 5 变化）。
    assert ledger["view_bytes"] == 23
    assert ledger["view_only_bytes"] == 0
    assert (sum(ledger["owner_bytes"]) + ledger["view_only_bytes"]
            + ledger["unmodeled_total"] + ledger["undeclared_total"]) == 2272


def test_real_unmodeled_notes_have_code_pointers():
    dump = nm.load_dump()
    notes = [s["note"] for s in dump["spans"] if s["key"] == "unmodeled"]
    assert notes, "真 dump 应有 Unmodeled 段"
    for note in notes:
        assert nm.reason_pointer_errors(note) == [], note


def test_real_ledger_owner_spans_match_dump():
    ledger_path = ROOT / "data" / "status" / "nes_map.json"
    if not ledger_path.is_file():
        pytest.skip("账本尚未建立")
    dump = nm.load_dump()
    ledger = json.loads(ledger_path.read_text(encoding="utf-8"))
    expected = sorted(
        ({"start": s["start"], "len": s["len"]} for s in dump["spans"] if s["key"] == "owner"),
        key=lambda e: (e["start"], e["len"]),
    )
    assert sorted(ledger["owner_spans"], key=lambda e: (e["start"], e["len"])) == expected


def test_real_ledger_unmodeled_matches_dump():
    ledger_path = ROOT / "data" / "status" / "nes_map.json"
    if not ledger_path.is_file():
        pytest.skip("账本尚未建立")
    dump = nm.load_dump()
    ledger = json.loads(ledger_path.read_text(encoding="utf-8"))
    # 账本只登记比对面内的 Unmodeled（面外 Room $6000 缓冲不参与 P2 四态）。
    expected = nm._partition(dump["spans"], dump["face"])["unmodeled"]
    assert ledger["unmodeled"] == expected
