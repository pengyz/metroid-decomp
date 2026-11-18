"""Task 2 分叉分类器的 TDD 测试。

合成用例只构造帧/字段字典，不读 TAS；真实轨迹冒烟用仓库内固定路径，
用于锁住「offsets 从 JSON 读、三腿解码复用 Task 1」这两条契约。
"""

from __future__ import annotations

import json
from pathlib import Path

import pytest

from tools.movie_ref.align_legs import load_state
from tools.movie_ref.classify_episodes import (
    _lag_histogram,
    _phase_window_provenance,
    _sensitivity,
    build_report,
    classify,
)

ROOT = Path(__file__).resolve().parents[2]
REAL = {
    "biz": ROOT / "analysis/traces/bizhawk-6725M-post.jsonl",
    "mesen": ROOT / "analysis/traces/_probe-axis.frames.jsonl",
    "rust": Path("/tmp/rust-states-t2.jsonl"),
}
ALL_FIELDS = ("status", "room", "health_lo", "health_hi", "scrolldir")


def _legs():
    return {
        "mesen": {f: {"status": 3, "room": 0xFF} for f in range(20, 40)},
        "rust": {f: {"status": 3, "room": 0xFF} for f in range(20, 40)},
    }


def test_transient_spike_is_transient():
    legs = _legs()
    legs["mesen"][25] = {"status": 4, "room": 0xFF}
    eps = classify(legs, {"mesen": 0, "rust": 0}, ("status",), 20, 39)
    assert len(eps) == 1
    assert eps[0]["class"] == "transient"
    assert eps[0]["evidence"]["transient"] is True
    assert eps[0]["leg"] == "rust"
    assert eps[0]["start"] == 25 and eps[0]["end"] == 25


def test_load_window_is_flagged():
    legs = _legs()
    legs["mesen"][30] = {"status": 9, "room": 0x09}
    eps = classify(legs, {"mesen": 0, "rust": 0}, ("status",), 20, 39)
    assert any(e["during_load"] for e in eps)


def test_persistent_value_difference_is_value():
    legs = _legs()
    legs["mesen"] = {f: {"status": 3, "room": 0xFF} for f in range(20, 40)}
    legs["biz"] = {f: {"status": 3, "room": 0xFF} for f in range(20, 40)}
    legs["biz"][24] = {"status": 4, "room": 0xFF}  # biz offset +1 => mesen f25
    eps = classify(legs, {"mesen": 0, "biz": 1, "rust": 0}, ("status",), 20, 39)
    assert len(eps) == 1
    assert eps[0]["leg"] == "biz"
    assert eps[0]["start"] == 25 and eps[0]["end"] == 25
    assert eps[0]["class"] == "value"


def test_phase_sequence_recovers_shift():
    """phase 必须是整段序列平移匹配，不是「值集合里出现过」。"""
    mesen = {f: {"status": val, "room": 0xFF} for f, val in zip(range(20, 30), [1, 2, 3, 4, 5, 6, 7, 8, 9, 10])}
    rust = {f: {"status": mesen[f + 1]["status"], "room": 0xFF} for f in range(20, 29)}
    eps = classify(
        {"mesen": mesen, "rust": rust}, {"mesen": 0, "rust": 0},
        ("status",), 20, 28,
    )
    assert len(eps) == 1
    assert eps[0]["class"] == "phase"
    assert eps[0]["evidence"]["shift"] == 1


def test_long_value_not_washed_by_one_coincidental_value():
    """51 帧持续差 + 中途一次巧合同值，仍必须是 value（段窗判据的反例）。"""
    legs = _legs()
    legs["mesen"] = {f: {"status": 3, "room": 0xFF} for f in range(20, 70)}
    legs["rust"] = {f: {"status": 8, "room": 0xFF} for f in range(20, 70)}
    legs["rust"][43] = {"status": 9, "room": 0xFF}
    legs["mesen"][43] = {"status": 8, "room": 0xFF}  # 段内巧合值 8，但 f43 仍失配
    legs["mesen"][44] = {"status": 9, "room": 0xFF}  # 段内巧合值 9
    eps = classify(legs, {"mesen": 0, "rust": 0}, ("status",), 20, 65)
    assert len(eps) == 1
    assert eps[0]["class"] == "value"


def test_constant_field_is_undecidable():
    """health_lo 这类常量字段不默认 phase，标 undecidable。"""
    legs = _legs()
    legs["mesen"] = {f: {"status": 3, "health_lo": 1} for f in range(20, 30)}
    legs["rust"] = {f: {"status": 3, "health_lo": 0} for f in range(20, 30)}
    eps = classify(legs, {"mesen": 0, "rust": 0}, ("health_lo",), 20, 29)
    assert len(eps) == 1
    assert eps[0]["class"] == "undecidable"
    assert eps[0]["evidence"]["undecidable_fields"] == ["health_lo"]


def test_denominators_report_load_and_missing_frames():
    legs = _legs()
    legs["mesen"][30] = {"status": 9, "room": 0x09}
    legs["rust"].pop(35)
    report = build_report(legs, {"mesen": 0, "rust": 0}, ("status",), 20, 39)
    den = report["denominators"]
    assert den["total_frames"] == 20
    assert den["excluded"]["load"] >= 1
    assert den["excluded"]["missing"] >= 1
    assert den["excluded"]["load_frames"] == [30]
    assert 35 in den["excluded"]["missing_frames"]
    assert den["in_denominator"] == den["total_frames"] - len(
        set(den["excluded"]["load_frames"]) | set(den["excluded"]["missing_frames"])
    )
    assert den["common_in_denominator"] == den["in_denominator"]
    assert "missing_within_load" in den["excluded"]
    assert "missing_outside_load" in den["excluded"]


def test_summary_lists_first_three_value_episodes():
    legs = _legs()
    legs["biz"] = {f: {"status": 3, "room": 0xFF} for f in range(20, 60)}
    for g in (22, 25, 28, 31):
        legs["biz"][g] = {"status": 9, "room": 0xFF}
    report = build_report(legs, {"mesen": 0, "biz": 0, "rust": 0}, ("status",), 20, 39)
    value = report["summary"]["value_episodes"]
    assert report["summary"]["value_count"] == 4
    assert 1 <= len(value) <= 3
    assert all("leg" in row and "start" in row for row in value)


def _hex(bytes_: bytearray) -> str:
    return bytes_.hex()


def test_load_state_decodes_extended_fields(tmp_path):
    low = bytearray(0x100)
    low[0x5A] = 0x12
    low[0x49] = 0x03
    high = bytearray(0x700)
    high[0x300 - 0x100] = 0x34
    high[0x106 - 0x100] = 0x05
    high[0x107 - 0x100] = 0x01
    mesen = {
        "f": 20,
        "w": {
            "0000-00ff": _hex(low),
            "0100-07ff": _hex(high),
            "Joy1Latch": 2,
        },
    }
    zp = bytearray(0x100)
    zp[0x5A] = 0x12
    zp[0x49] = 0x03
    zp[0x14] = 2
    iram = bytearray(0x700)
    iram[0x300 - 0x100] = 0x34
    iram[0x106 - 0x100] = 0x05
    iram[0x107 - 0x100] = 0x01
    rust = {"f": 20, "w": {"zp": _hex(zp), "iram": _hex(iram), "wram": ""}}
    biz = {
        "f": 20,
        "health_lo": 5,
        "health_hi": 1,
        "status": 0x34,
        "room": 0x12,
        "scrolldir": 3,
        "joy1_latch": 2,
    }
    for name, row in (("mesen", mesen), ("rust", rust), ("biz", biz)):
        path = tmp_path / f"{name}.jsonl"
        path.write_text(json.dumps(row) + "\n", encoding="utf-8")
        got = load_state(str(path), name, fields=ALL_FIELDS)
        assert got == {
            20: {
                "status": 0x34,
                "room": 0x12,
                "health_lo": 5,
                "health_hi": 1,
                "scrolldir": 3,
                "latch": 2,
            }
        }


def test_sensitivity_reports_windows_and_note():
    legs = _legs()
    out = _sensitivity(legs, {"mesen": 0, "rust": 0}, ("status",), 20, 39, "room")
    assert set(out) == {"W=2", "W=3", "W=4", "note"}
    assert out["note"] == "W=4 会让 transient 类整体消失，故不采用。"
    assert set(out["W=2"]) == {"phase", "transient", "value", "undecidable"}


def test_lag_histogram_forward_not_nearest(tmp_path):
    """W 必须取 p1 变化之后首个 latch 变化；最近邻会把滞后 2 记成 -1。"""
    path = tmp_path / "biz-lag.jsonl"
    rows = []
    p1 = 0
    latch = 0
    for frame in range(1, 16):
        if frame == 9:
            latch = 1
        if frame == 10:
            p1 = 1
        if frame == 12:
            latch = 2
        rows.append({"f": frame, "p1": p1, "joy1_latch": latch})
    path.write_text("".join(json.dumps(row) + "\n" for row in rows), encoding="utf-8")
    hist = _lag_histogram(str(path), 1, 20)
    assert hist["forward"] == {"2": 1}
    assert hist["nearest"] == {"-1": 1}
    assert hist["max_abs_forward"] == 2
    assert hist["max_abs_nearest"] == 1
    provenance = _phase_window_provenance(
        {"biz": {}, "mesen": {}, "rust": {}},
        {"biz": 1, "mesen": 0, "rust": 0},
        [],
        hist,
    )
    assert provenance["value"] == 2
    assert "max(forward" in provenance["derivation"]


def test_real_traces_smoke_reads_offsets_and_finds_value_segment():
    offsets_doc = json.loads((ROOT / "data/status/leg_offsets.json").read_text())
    offsets = {
        "biz": offsets_doc["offsets"]["biz_to_mesen"],
        "rust": offsets_doc["offsets"]["rust_to_mesen"],
    }
    assert offsets == {"biz": 1, "rust": 0}
    legs = {name: load_state(str(path), name, fields=ALL_FIELDS) for name, path in REAL.items()}
    report = build_report(legs, offsets, ALL_FIELDS, 20, 1200)
    assert report["summary"]["total_episodes"] == len(report["episodes"])
    assert report["summary"]["total_episodes"] > 0
    assert report["summary"]["by_class"]["value"] >= 1
    assert all("class" in e and "kind" in e and "during_load" in e for e in report["episodes"])
    assert "phase_window" in report["summary"]
    assert report["summary"]["phase_window"]["upper_bound"] == 2


def test_main_rejects_offsets_with_non_aligned_verdict(tmp_path):
    """产物守卫：verdict != aligned 的 offsets JSON 不得被消费（否则多解被当合法偏移）。"""
    import pytest

    from tools.movie_ref import classify_episodes

    offsets = json.loads((ROOT / "data/status/leg_offsets.json").read_text())
    offsets["verdict"] = "ambiguous-offset"
    bad = tmp_path / "offsets.json"
    bad.write_text(json.dumps(offsets), encoding="utf-8")
    argv = [
        "--bizhawk", str(REAL["biz"]),
        "--mesen", str(REAL["mesen"]),
        "--rust", str(REAL["rust"]),
        "--offsets", str(bad),
        "--json-out", str(tmp_path / "eps.json"),
    ]
    rc = classify_episodes.main(argv)
    assert rc == 2
    assert not (tmp_path / "eps.json").exists()
