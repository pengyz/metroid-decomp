"""#150：tas_replay_diff B4 紧凑轨迹 / emu_missing 记账 / B1 --axis updates 单测。

只测纯函数层（不跑 emu/rust 腿）；风格沿用 test_tas_resume_anchor.py。
"""
from __future__ import annotations

import json
import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parent.parent.parent
sys.path.insert(0, str(ROOT / "tools"))
import tas_replay_diff as trd  # noqa: E402


# ---------------------------------------------------------------- B4 紧凑轨迹

def _mk_rows(n: int = 60, w_bytes: int = 16) -> list[dict]:
    """合成逐帧面：range 逐帧递增（保证每帧都有差分），Joy1Status 恒 9。"""
    return [{"f": f,
             "w": {"ens": bytes((f + i) & 0xFF for i in range(w_bytes)).hex(),
                   "Joy1Status": 0x09}}
            for f in range(1, n + 1)]


def test_compact_roundtrip(tmp_path: Path):
    rows = _mk_rows()
    obj = trd.encode_compact_trace(rows)
    assert obj["format"] == trd.COMPACT_FORMAT
    assert obj["layout"] == {"ens": 16}
    rt = trd.decode_compact_trace(obj)
    assert trd._rows_content_sha(rt) == trd._rows_content_sha(rows)
    # w_extra（非 hex 面）与行级附加键都要原样往返
    rows2 = [dict(r, extra_field="x") for r in rows]
    rows2[0]["w"]["Joy1Status"] = 0x07
    obj2 = trd.encode_compact_trace(rows2)
    rt2 = trd.decode_compact_trace(obj2)
    assert trd._rows_content_sha(rt2) == trd._rows_content_sha(rows2)


def test_compact_tamper_caught(tmp_path: Path):
    obj = trd.encode_compact_trace(_mk_rows())
    for mutate in (
        lambda o: o["frames"][9]["x"]["ens"].__setitem__(0, [0, "ff"]),
        lambda o: o["initial"].__setitem__("ens", "00" * 16),
        lambda o: o["frames"].pop(5),
    ):
        bad = json.loads(json.dumps(obj))
        mutate(bad)
        with pytest.raises(trd.ReplayError, match="content_sha256 不一致"):
            trd.decode_compact_trace(bad)


def test_compact_layout_sha_and_drift_fail_loud():
    obj = trd.encode_compact_trace(_mk_rows())
    bad = json.loads(json.dumps(obj))
    bad["layout"]["ens"] = 15  # 布局被改
    with pytest.raises(trd.ReplayError, match="layout_sha256"):
        trd.decode_compact_trace(bad)
    # 布局漂移：同一面在后续行长度变化，编码时就 fail-loud
    rows = _mk_rows()
    rows[3]["w"]["ens"] = "00" * 15
    with pytest.raises(trd.ReplayError, match="字节长漂移"):
        trd.encode_compact_trace(rows)


def test_compact_xor_out_of_range_fail_loud():
    obj = trd.encode_compact_trace(_mk_rows())
    bad = json.loads(json.dumps(obj))
    bad["frames"][2]["x"]["ens"] = [[15, "ffff"]]  # 越界（15+2>16）
    bad["content_sha256"] = trd._rows_content_sha(
        trd.decode_compact_trace(obj))  # 哈希对齐，让越界先炸
    with pytest.raises(trd.ReplayError, match="差分段越界"):
        trd.decode_compact_trace(bad)


def test_compact_gz_roundtrip(tmp_path: Path):
    rows = _mk_rows()
    obj = trd.encode_compact_trace(rows)
    gz = tmp_path / "trace.json.gz"
    trd.write_compact_trace(gz, obj)
    obj2 = trd.read_compact_trace(gz)
    assert trd._rows_content_sha(trd.decode_compact_trace(obj2)) == trd._rows_content_sha(rows)


# ---------------------------------------------------------- emu_missing 记账

def _mk_faces(n: int = 60) -> dict[int, dict[str, bytes]]:
    return {f: {"range": bytes((f + i) & 0xFF for i in range(16)), "Joy1Status": bytes([9])}
            for f in range(1, n + 1)}


def test_compare_reports_emu_missing_not_silent():
    faces = _mk_faces()
    hole = {f: dict(d) for f, d in faces.items() if f not in (10, 20, 30)}
    rep = trd.compare(hole, dict(faces), 0x400, 60)
    assert rep["emu_missing_frames"] == [10, 20, 30]
    assert rep["rust_missing_frames"] == []
    assert "emu 缺 3 帧" in rep["coverage_note"]
    assert "首分歧可能右移" in rep["coverage_note"]
    assert rep["compared"] == 57


def test_selftest_style_injection_still_detected():
    faces = _mk_faces()
    bad = {f: {k: bytes(v) for k, v in d.items()} for f, d in faces.items()}
    bad[33]["range"] = bad[33]["range"][:5] + bytes([bad[33]["range"][5] ^ 1]) + bad[33]["range"][6:]
    rep = trd.compare(dict(bad), dict(faces), 0x400, 60)
    assert rep["first_divergence"]["frame"] == 33
    assert rep["first_divergence"]["field"] == "$0405"


# ------------------------------------------------------------ B1 --axis updates

def test_input_change_frames(tmp_path: Path):
    # 第 n 行 = 帧 n 消费的输入；0 行输入 = 全零。
    lines = ["# head"]
    lines += ["00 00 00"] * 3                      # 帧 1-3 全零
    lines += ["01 00 00"]                          # 帧 4 变化（更新帧）
    lines += ["01 00 00"] * 2                      # 帧 5-6 不变
    lines += ["00 00 00"]                          # 帧 7 变化（更新帧）
    lines += ["03 00 00"]                          # 帧 8 变化（更新帧）
    il = tmp_path / "m.inputlog"
    il.write_text("\n".join(lines) + "\n", encoding="utf-8")
    got = trd.input_change_frames(il, 8)
    assert got == {4, 7, 8}


def test_input_change_frames_all_zero_fails(tmp_path: Path):
    il = tmp_path / "m.inputlog"
    il.write_text("00 00 00\n00 00 00\n", encoding="utf-8")
    with pytest.raises(trd.ReplayError, match="无输入变化"):
        trd.input_change_frames(il, 2)


def test_compare_updates_axis_skips_no_input_explicitly():
    faces = _mk_faces()
    upd = {11, 23}
    rep = trd.compare(dict(faces), dict(faces), 0x400, 60,
                      axis="updates", update_frames=upd)
    assert rep["axis"] == "updates"
    assert rep["compared"] == 2
    assert rep["axis_skipped_no_input_frames"] == \
        [f for f in range(1, 61) if f not in upd]
    assert "跳过 58 个无输入变化帧" in rep["coverage_note"]


def test_compare_updates_axis_empty_update_frames_fails():
    faces = _mk_faces()
    with pytest.raises(trd.ReplayError, match="缺更新帧清单"):
        trd.compare(dict(faces), dict(faces), 0x400, 60, axis="updates", update_frames=set())


def test_compare_window_updates_axis():
    faces = _mk_faces()
    upd = {5, 7}
    rep = trd.compare_window(dict(faces), dict(faces), 0, 0x400, rust_base=0,
                             axis="updates", update_frames=upd)
    assert rep["compared"] == 2
    assert set(rep["axis_skipped_no_input_frames"]) == set(faces) - upd
    assert rep["emu_missing_frames"] == []  # 窗口内无缺帧


def test_compare_window_reports_missing_in_union_window():
    faces = _mk_faces()
    hole = {f: dict(d) for f, d in faces.items() if f != 12}
    rep = trd.compare_window(hole, dict(faces), 0, 0x400, rust_base=0)
    assert rep["emu_missing_frames"] == [12]
    assert rep["rust_missing_frames"] == []
