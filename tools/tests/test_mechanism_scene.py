"""Task 5：机制级最小重现 runner 的纯函数/边界测试。"""

from __future__ import annotations

import subprocess
import sys

import pytest

from tools.movie_ref.mechanism_scene import (
    _choose_anchor,
    _run,
    _shorten_case,
    build_parser,
    compare,
    main,
    run_scene,
    state_offset_from_latch,
)


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
    assert "默认 status,room" in help_text
    assert "status,room,latch" not in help_text


def test_state_offset_from_latch_uses_measured_gap():
    doc = {"offsets": {"rust_to_mesen": 0}, "latch_offsets": {"rust_to_mesen": 1}}
    assert state_offset_from_latch(-1, doc) == 0


def test_compare_informative_false_for_constant_window():
    m = {f: {"status": 0} for f in range(0, 10)}
    r = {f: {"status": 0} for f in range(0, 10)}
    out = compare(m, r, ("status",))
    assert out["all_match"] is True
    assert out["informative"] is False


def test_compare_informative_true_for_varying_window():
    m = {0: {"status": 0}, 1: {"status": 1}, 2: {"status": 20}}
    r = {0: {"status": 0}, 1: {"status": 1}, 2: {"status": 20}}
    out = compare(m, r, ("status",))
    assert out["all_match"] is True
    assert out["informative"] is True


def test_choose_anchor_uses_latch_fallback_and_measured_gap():
    # 状态面常量（无法对齐），latch 有 4 个脉冲；rust 的 latch 比 mesen 早一帧
    # ⇒ align(mesen, rust) = -1，加上 Task 1 实测 gap +1 后 state offset = 0。
    mesen = {f: {"status": 0, "room": 0, "latch": 0} for f in range(20, 60)}
    for f in (25, 33, 41, 50):
        mesen[f]["latch"] = 0x01 if f % 2 else 0x81
    rust = {f: {"status": 0, "room": 0,
                "latch": mesen[f + 1]["latch"] if f + 1 in mesen else 0}
            for f in range(20, 60)}
    offset, anchor = _choose_anchor(mesen, rust, ("status", "room"))
    assert offset == 0
    assert anchor.startswith("latch(")


def test_shorten_case_relabels_latch_and_clips(tmp_path):
    case = tmp_path / "x.txt"
    case.write_text(
        "name x\nframes 900\nwatch 0014 Joy1Status\n"
        "press 700 start\nhold 700-900 right\ncapture 800 foo\n"
        "endFrame 900\n",
        encoding="utf-8",
    )
    name, text = _shorten_case(case, 120)
    assert name.startswith("mechanism-x-120-")
    assert "frames 120" in text
    assert "watch 0014 Joy1Latch" in text
    assert "press 700" not in text
    assert "capture 800" not in text
    assert "hold 700-900" not in text
    assert "watch_range 0000-00ff" in text
    assert "watch_range 0100-07ff" in text


def test_run_scene_missing_case_raises():
    with pytest.raises(FileNotFoundError, match="case 不存在"):
        run_scene("/definitely/not/a/case.txt", 120)


def test_run_command_failure_raises():
    with pytest.raises(RuntimeError, match="失败 rc=7"):
        _run([sys.executable, "-c", "import sys; sys.exit(7)"], 5.0, "闪失败")


def test_main_uninformative_exit_3_and_allow_constant(monkeypatch):
    import tools.movie_ref.mechanism_scene as ms

    def fake_run_scene(case, frames):
        rows = {f: {"status": 0, "room": 0, "latch": 0} for f in range(1, 11)}
        return {"mesen": rows, "rust": rows, "offset": 0,
                "anchor_field": "status", "trace_name": "fake"}

    monkeypatch.setattr(ms, "run_scene", fake_run_scene)
    assert ms.main(["--case", "x", "--frames", "10"]) == 3
    assert ms.main(["--case", "x", "--frames", "10", "--allow-constant"]) == 0


def test_run_scene_missing_mesen_trace_raises(monkeypatch, tmp_path):
    import tools.movie_ref.mechanism_scene as ms
    case = tmp_path / "x.txt"
    case.write_text("name x\nframes 10\nsnapshot none\n", encoding="utf-8")
    monkeypatch.setattr(ms, "_run", lambda *a, **k: None)
    with pytest.raises(FileNotFoundError, match="Mesen2 腿未产出 trace"):
        ms.run_scene(str(case), 10)


def test_state_offset_from_latch_rejects_doc_convention():
    doc = {"offsets": {"rust_to_mesen": 0}, "latch_offsets": {"rust_to_mesen": 1}}
    with pytest.raises(ValueError, match="口径不一致"):
        state_offset_from_latch(1, doc)  # 传了 doc 口径的 +1，不是 align 口径的 -1


def test_run_scene_missing_rust_binary_raises(monkeypatch, tmp_path):
    import tools.movie_ref.mechanism_scene as ms
    case = tmp_path / "x.txt"
    case.write_text("name x\nframes 10\nsnapshot none\n", encoding="utf-8")
    monkeypatch.setattr(ms, "_shorten_case",
                        lambda path, frames: ("fixed", "name fixed\nframes 10\nsnapshot none\n"))
    monkeypatch.setattr(ms, "_run", lambda *a, **k: None)
    monkeypatch.setattr(ms, "ROOT", tmp_path)
    trace = tmp_path / "analysis" / "traces" / "fixed.frames.jsonl"
    trace.parent.mkdir(parents=True)
    trace.write_text("{}\n", encoding="utf-8")
    with pytest.raises(FileNotFoundError, match="缺 Rust 腿二进制"):
        ms.run_scene(str(case), 10)
