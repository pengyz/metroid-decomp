#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""tools/tests/test_l4_snapshot_align.py —— Task 4 对齐验收（加载成功≠对齐成功防线）测试。

覆盖（task-4-brief Step 1 + 验收②③）：
1. 对齐验收函数 `align_states`：restore 后前 N 帧 Rust↔emu 快进**逐字节**（zp/iram/wram
   面逐字节）diff 产出**逐帧**结果；残差=0（residual.status="none"）或**显式登记**
   （residual.status="registered"，含首分歧点/帧集）；
2. 「加载成功≠对齐成功」被验收捕获（验收③）：**造一个不对齐 case**（两侧都装载成功、
   帧 1 起状态即不同）→ 验收必须裁 residual-registered + 首分歧逐字节定位，
   **绝不**产出 verdict="aligned"（假绿防线）；
3. 缺行（采样窗缺失）显式登记，不静默吞、不冒充对齐；
4. 面覆盖（wram 只在 Rust 侧）显式登记，比较面=交集；
5. 无可比面（空跑）→ fail-loud 拒裁（空跑不得绿）。

行格式（与 oracle/l3-dump 帧 trace 同形——单一行格式）：
    {"f":<N>,"w":{"zp":"<hex>","iram":"<hex>","wram":"<hex>"}}
"""
from __future__ import annotations

import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parent.parent.parent
sys.path.insert(0, str(ROOT / "tools"))

import l4_diff_run as l4  # noqa: E402


def rows(*specs):
    """构造行集：specs=(f, zp, iram, wram|None)…（wram None=该侧无此面）。"""
    out = {}
    for spec in specs:
        f, zp, iram = spec[0], spec[1], spec[2]
        w = {"zp": zp, "iram": iram}
        if len(spec) > 3 and spec[3] is not None:
            w["wram"] = spec[3]
        out[f] = w
    return out


class TestAlignStatesResidualZero:
    """残差=0 面：逐帧全等 → verdict=aligned / residual.status=none。"""

    def test_identical_rows_aligned_residual_none(self):
        rust = rows((1, "AA" * 8, "10" * 8, "FF" * 8),
                    (2, "AB" * 8, "11" * 8, "FE" * 8),
                    (3, "AC" * 8, "12" * 8, "FD" * 8))
        rep = l4.align_states(rust, dict(rust), n=3, name="fixture")
        assert rep["verdict"] == "aligned"
        assert rep["residual"]["status"] == "none"
        assert rep["first_divergence"] is None
        assert rep["frames_compared"] == 3
        assert rep["aligned"] == 3
        assert rep["diverged"] == 0
        for fr in rep["per_frame"]:
            assert fr["status"] == "aligned", fr

    def test_first_n_only_compares_prefix(self):
        rust = rows((1, "AA" * 8, "10" * 8), (2, "AA" * 8, "10" * 8),
                    (3, "AA" * 8, "10" * 8))
        emu = rows((1, "AA" * 8, "10" * 8), (2, "AA" * 8, "10" * 8),
                   (3, "BB" * 8, "10" * 8))  # f=3 分歧但 N=2 不看
        rep = l4.align_states(rust, emu, n=2, name="fixture")
        assert rep["verdict"] == "aligned"
        assert rep["frames_compared"] == 2


class TestLoadSuccessIsNotAlignSuccess:
    """验收③核心：装载成功 ≠ 对齐成功——不对齐 case 必须被捕获。"""

    def test_constructed_misaligned_case_is_caught(self):
        # 两侧都"装载成功"（行 1 都在）但状态不同——典型"加载成功≠对齐成功"形态
        emu = rows((1, "01" * 8, "17" * 8), (2, "01" * 8, "18" * 8))
        rust = rows((1, "02" * 8, "17" * 8), (2, "02" * 8, "18" * 8))
        rep = l4.align_states(rust, emu, n=2, name="fixture")
        assert rep["verdict"] == "residual-registered"
        assert rep["residual"]["status"] == "registered"
        assert rep["first_divergence"] is not None
        fd = rep["first_divergence"]
        assert fd["f"] == 1 and fd["face"] == "zp"
        assert fd["off"] == 0
        assert fd["rust"] == "02" and fd["emu"] == "01"
        # 逐帧 diff 显式（不是布尔绿/红）
        assert rep["per_frame"][0]["status"] == "diverged"
        assert rep["per_frame"][0]["diff_bytes"] == 8
        assert rep["diverged"] == 2 and rep["aligned"] == 0

    def test_byte_level_divergence_located(self):
        emu = rows((1, "00" * 8, "00" * 8))
        rust = rows((1, "00" * 8, "00" * 8))
        # 唯一差异：zp 字节 5（0x05）
        zp = list(bytes.fromhex(rust[1]["zp"]))
        zp[5] = 0x94
        rust[1]["zp"] = bytes(zp).hex().upper()
        emu_zp = list(bytes.fromhex(emu[1]["zp"]))
        emu_zp[5] = 0x91
        emu[1]["zp"] = bytes(emu_zp).hex().upper()
        rep = l4.align_states(rust, emu, n=1, name="fixture")
        fd = rep["first_divergence"]
        assert fd == {"f": 1, "face": "zp", "off": 5, "rust": "94", "emu": "91"}
        assert rep["per_frame"][0]["diff_bytes"] == 1

    def test_length_mismatch_is_divergence_not_hidden(self):
        emu = rows((1, "AA" * 4, "10" * 8))   # zp 短面（4B）
        rust = rows((1, "AA" * 8, "10" * 8))  # zp 8B
        rep = l4.align_states(rust, emu, n=1, name="fixture")
        assert rep["verdict"] == "residual-registered"
        assert rep["per_frame"][0]["status"] == "diverged"
        assert "length" in rep["per_frame"][0].get("reason", "") + \
               str(rep["first_divergence"])


class TestMissingRowsAndFacesExplicit:
    """缺行/缺面显式登记（不静默吞、不冒充对齐）。"""

    def test_missing_row_registered_not_aligned(self):
        rust = rows((1, "AA" * 8, "10" * 8), (2, "AA" * 8, "10" * 8))
        emu = rows((1, "AA" * 8, "10" * 8))  # f=2 emu 缺行（采样窗缺失）
        rep = l4.align_states(rust, emu, n=2, name="fixture")
        assert rep["per_frame"][1]["status"] == "missing_emu"
        assert rep["verdict"] == "residual-registered"
        assert rep["residual"]["status"] == "registered"
        assert rep["missing_emu"] == [2]

    def test_missing_rust_row_registered(self):
        rust = rows((1, "AA" * 8, "10" * 8))
        emu = rows((1, "AA" * 8, "10" * 8), (2, "AA" * 8, "10" * 8))
        rep = l4.align_states(rust, emu, n=2, name="fixture")
        assert rep["per_frame"][1]["status"] == "missing_rust"
        assert rep["missing_rust"] == [2]

    def test_one_sided_face_registered_in_coverage(self):
        rust = rows((1, "AA" * 8, "10" * 8, "FF" * 8))  # 有 wram
        emu = rows((1, "AA" * 8, "10" * 8))             # 无 wram（bundle-ff 实测形态）
        rep = l4.align_states(rust, emu, n=1, name="fixture")
        assert rep["faces"]["common"] == ["zp", "iram"]
        assert rep["faces"]["rust_only"] == ["wram"]
        assert rep["faces"]["emu_only"] == []
        assert rep["verdict"] == "aligned"  # 交集面全等；wram 覆盖已显式登记

    def test_no_common_faces_fails_loud(self):
        rust = rows((1, "AA" * 8, "10" * 8))
        emu = {1: {"wram": "FF" * 8}}  # 面完全不交
        with pytest.raises(ValueError):
            l4.align_states(rust, emu, n=1, name="fixture")


class TestStateRowLoader:
    """行装载（{"f":N,"w":{…}} JSONL——oracle/l3-dump 同形单一格式）。"""

    def test_load_state_rows_jsonl(self, tmp_path):
        p = tmp_path / "x.frames.jsonl"
        p.write_text('{"f":1,"w":{"zp":"AA","iram":"BB"}}\n'
                     '{"f":2,"w":{"zp":"CC","iram":"DD"}}\n', encoding="utf-8")
        got = l4.load_state_rows(p)
        assert got == {1: {"zp": "AA", "iram": "BB"}, 2: {"zp": "CC", "iram": "DD"}}

    def test_load_state_rows_empty_fails_loud(self, tmp_path):
        p = tmp_path / "empty.frames.jsonl"
        p.write_text("", encoding="utf-8")
        with pytest.raises(ValueError):
            l4.load_state_rows(p)
