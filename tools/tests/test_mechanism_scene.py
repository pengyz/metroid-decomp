"""Task 5：机制级最小重现 runner 的纯函数/边界测试。"""

from __future__ import annotations

import pytest

from tools.movie_ref.mechanism_scene import build_parser, compare, run_scene, state_offset_from_latch


def test_compare_reports_first_diff():
    m = {f: {"status": 3} for f in range(0, 10)}
    r = {f: {"status": 3} for f in range(0, 10)}
    r[7]["status"] = 1
    out = compare(m, r, ("status",))
    assert out["first_diff"] == 7
    assert out["mismatch"] == 1
    assert out["per_field"] == {"status": 1}
    assert out["all_match"] is False
    assert out["compared"] == 10


def test_compare_ignores_fields_not_requested():
    m = {0: {"status": 3, "room": 0x12}}
    r = {0: {"status": 3, "room": 0x34}}
    out = compare(m, r, ("status",))
    assert out["first_diff"] is None
    assert out["mismatch"] == 0
    assert out["all_match"] is True
    assert out["per_field"] == {"status": 0}


def test_compare_missing_row_is_incomparable_not_mismatch():
    m = {1: {"status": 3}, 2: {"status": 3}}
    r = {2: {"status": 3}, 3: {"status": 3}}
    out = compare(m, r, ("status",))
    assert out["first_diff"] is None
    assert out["mismatch"] == 0
    assert out["compared"] == 1
    assert out["incomparable_frames"] == 2
    assert out["incomparable_fields"] == 0
    assert out["all_match"] is True


def test_compare_missing_field_is_incomparable_not_mismatch():
    m = {1: {"status": 3}}
    r = {1: {"room": 0x12}}
    out = compare(m, r, ("status", "room"))
    assert out["first_diff"] is None
    assert out["mismatch"] == 0
    assert out["compared"] == 0
    assert out["incomparable_fields"] == 2
    assert out["all_match"] is False


def test_run_scene_rejects_frames_above_cap_without_touching_inputs():
    with pytest.raises(ValueError, match="600"):
        run_scene("/definitely/not/a/case.txt", 601)


def test_cli_help_documents_frame_cap_reason():
    help_text = build_parser().format_help()
    assert "600" in help_text
    assert "机制级" in help_text


def test_state_offset_from_latch_uses_measured_gap():
    doc = {"offsets": {"rust_to_mesen": 0}, "latch_offsets": {"rust_to_mesen": 1}}
    assert state_offset_from_latch(-1, doc) == 0
