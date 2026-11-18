#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""tools/tests/test_align_legs.py —— 因果锚帧映射器测试（TDD 先红后绿）。

覆盖计划 Task 1 Step 1 的两条合成用例（不含 TAS 内容），另加：
- 多解必须抛 ambiguous offset（不许偷偷挑一个偏移）；
- Mesen2 行缺 w["Joy1Latch"] 必须 fail-loud（不许返回空 dict / 静默跳过）；
- Rust zp 扁平 hex 的 $0014 字节按第 0x14 个字节读取。
"""

import json

import pytest

from tools.movie_ref.align_legs import align, load_latch, load_state, resolve_leg


def test_align_recovers_known_offset():
    a = {f: 0 for f in range(20, 60)}
    b = dict(a)
    for f in (25, 33, 41, 50):          # 四次输入跳变
        a[f] = 0x01 if f % 2 else 0x81
        b[f + 1] = a[f]                 # b 比 a 晚一帧
    assert align(a, b) == (1, 4)


def test_align_refuses_without_anchor():
    a = {f: 0 for f in range(20, 60)}
    with pytest.raises(ValueError, match="insufficient anchor changes"):
        align(a, dict(a))


def test_align_refuses_ambiguous_offset():
    # 周期 4 的输入跳变；b 是 a 平移 4 帧并补齐两端。offset 0 与 +4 平票 ⇒ 多解。
    a = {f: (0x01 if f % 4 == 0 else 0x00) for f in range(20, 60)}
    b = {f: (a[f - 4] if (f - 4) in a else 0x00) for f in range(16, 64)}
    with pytest.raises(ValueError, match="ambiguous offset"):
        align(a, b)


def test_load_latch_rust_reads_zp_byte_0x14(tmp_path):
    zp = bytearray(0x800)
    zp[0x14] = 0x5A
    p = tmp_path / "rust.jsonl"
    p.write_text(
        json.dumps({"f": 7, "w": {"zp": zp.hex(), "iram": ""}}) + "\n",
        encoding="utf-8",
    )
    assert load_latch(str(p), "rust") == {7: 0x5A}


def test_load_latch_mesen_missing_joy1latch_raises(tmp_path):
    p = tmp_path / "mesen.jsonl"
    p.write_text(
        json.dumps({"f": 1, "w": {"SamusStatus": 0}}) + "\n",
        encoding="utf-8",
    )
    with pytest.raises(ValueError, match="Joy1Latch"):
        load_latch(str(p), "mesen")


def test_load_state_mesen_decodes_status_room_from_ranges(tmp_path):
    low = bytearray(0x100)
    high = bytearray(0x700)
    low[0x5A] = 0x12
    high[0x300 - 0x100] = 0x34
    p = tmp_path / "mesen.jsonl"
    p.write_text(
        json.dumps(
            {
                "f": 5,
                "w": {
                    "0000-00ff": low.hex(),
                    "0100-07ff": high.hex(),
                    "Joy1Latch": 0x02,
                },
            }
        )
        + "\n",
        encoding="utf-8",
    )
    assert load_state(str(p), "mesen") == {
        5: {"status": 0x34, "room": 0x12, "latch": 0x02}
    }


def test_load_state_mesen_accepts_named_keys(tmp_path):
    p = tmp_path / "mesen-named.jsonl"
    p.write_text(
        json.dumps(
            {
                "f": 2,
                "w": {
                    "SamusStatus": 0x34,
                    "RoomNumber": 0x12,
                    "Joy1Latch": 0x02,
                },
            }
        )
        + "\n",
        encoding="utf-8",
    )
    assert load_state(str(p), "mesen") == {
        2: {"status": 0x34, "room": 0x12, "latch": 0x02}
    }


def test_load_state_mesen_missing_state_range_raises(tmp_path):
    p = tmp_path / "mesen-bad.jsonl"
    p.write_text(
        json.dumps({"f": 1, "w": {"Joy1Latch": 0}}) + "\n",
        encoding="utf-8",
    )
    with pytest.raises(ValueError, match="0100-07ff"):
        load_state(str(p), "mesen")


def test_load_state_rust_decodes_zp_and_iram(tmp_path):
    zp = bytearray(0x100)
    iram = bytearray(0x700)
    zp[0x5A] = 0x12
    zp[0x14] = 0x02
    iram[0x300 - 0x100] = 0x34
    p = tmp_path / "rust.jsonl"
    p.write_text(
        json.dumps({"f": 5, "w": {"zp": zp.hex(), "iram": iram.hex()}}) + "\n",
        encoding="utf-8",
    )
    assert load_state(str(p), "rust") == {
        5: {"status": 0x34, "room": 0x12, "latch": 0x02}
    }


def test_load_state_rust_missing_iram_raises(tmp_path):
    p = tmp_path / "rust-bad.jsonl"
    p.write_text(
        json.dumps({"f": 1, "w": {"zp": bytearray(0x100).hex()}}) + "\n",
        encoding="utf-8",
    )
    with pytest.raises(ValueError, match="iram"):
        load_state(str(p), "rust")


def _pulse_pair(state_offset=1, latch_offset=2, room_offset=None):
    """合成 a/b：4 次 1 帧按键；状态面与锁存面各自平移。"""
    pulses = (25, 33, 41, 50)
    a = {f: {"status": 0, "room": 0, "latch": 0} for f in range(20, 81)}
    b = {f: {"status": 0, "room": 0, "latch": 0} for f in range(20, 86)}
    for i, p in enumerate(pulses):
        a[p] = {"status": 0x10 + i, "room": 0x20 + i, "latch": 0x30 + i}
    for i, p in enumerate(pulses):
        b[p + state_offset]["status"] = a[p]["status"]
        b[p + state_offset]["room"] = a[p]["room"]
        b[p + latch_offset]["latch"] = a[p]["latch"]
    if room_offset is not None:
        for rec in b.values():
            rec["room"] = 0
        for i, p in enumerate(pulses):
            b[p + room_offset]["room"] = a[p]["room"]
    return a, b


def test_resolve_leg_state_offset_and_latch_gap():
    a, b = _pulse_pair(state_offset=1, latch_offset=2)
    got = resolve_leg("biz", a, b)
    assert got["state_offset"] == 1
    assert got["latch_offset"] == 2
    assert got["per_field"]["status"]["offset"] == 1
    assert got["per_field"]["room"]["offset"] == 1
    # 状态面/锁存面的 agree_rate 必须按全帧分母算；变化帧只作可解性闸门。
    assert got["per_field"]["status"]["participants"] == len(a)
    assert got["per_field"]["status"]["change_frames"] == 8
    assert got["per_field"]["status"]["change_runs"] == 4
    assert got["latch"]["participants"] == len(a)
    assert got["latch"]["matches"] == len(a)
    assert got["latch"]["change_frames"] == 8
    assert got["latch"]["change_runs"] == 4
    assert got["latch"]["fit_participants"] == 8


def test_resolve_leg_rejects_field_disagreement():
    a, b = _pulse_pair(state_offset=1, latch_offset=2, room_offset=0)
    with pytest.raises(ValueError, match="field-disagreement"):
        resolve_leg("biz", a, b)


def test_resolve_leg_rejects_latch_state_gap():
    a, b = _pulse_pair(state_offset=1, latch_offset=1)
    with pytest.raises(ValueError, match="latch-state gap"):
        resolve_leg("biz", a, b)
