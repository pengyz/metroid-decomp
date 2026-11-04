#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""tools/tests/test_l4_flaky.py —— flaky 态落 l4 锚 schema 测试（TDD 先红后绿）。

背景（docs/findings/fade-blink-capture-race.md §3b；lead B 径裁定）：淡入窗 FC 奇偶翻格
族已由 C 径根修（采集截取缝化）；残差 flaky 格的**处置**须按 B 径落 schema：
**判据=多 run N 次观测记录**（≥min_runs 次观测且 verdict 不一致才可判 flaky），
flaky 格**不计分母不锚值**、显式留痕（观测记录随格）——绝不静默豁免。

覆盖：
1. `classify_flaky`（纯函数）：观测不足（<min_runs）不判 flaky；verdict 恒同不判；
   ≥min_runs 且 verdict 不一致 ⇒ flaky（含观测记录留痕）；
2. 矩阵处置（build_matrix）：flaky 格 state="flaky"+observations 留痕，**不计
   match/diff/blocked、不计分母**；
3. ratchet 豁免面（gates.cli.l4_ratchet_violations）：基线 match 格现态 flaky ⇒
   **不判①缩水**（flaky 不锚值）；但**普通回退照判**（match→diff 仍 FAIL——防弱化面）；
4. flaky 格仍计入 rows（留痕可查）——绝不删除/隐藏。

红态（本 commit）：classify_flaky 未实现 ⇒ ImportError/AttributeError 红。
绿态（下 commit）：实现后全绿。
"""
from __future__ import annotations

import importlib.util
import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parent.parent.parent
sys.path.insert(0, str(ROOT / "tools"))

import l4_diff_run as l4  # noqa: E402

_spec = importlib.util.spec_from_file_location("gates_cli_flaky_ut", ROOT / "gates" / "cli.py")
gates = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(gates)


def obs(case, frame, tag, verdict, run, code_state="sha_same"):
    return {"run_id": run, "case": case, "frame": frame, "tag": tag,
            "verdict": verdict, "code_state": code_state}


class TestClassifyFlaky:
    """多 run 观测记录=flaky 判定唯一依据（少于 min_runs 不判）。"""

    def test_insufficient_runs_not_flaky(self):
        rows = [obs("c", 560, "t", "match", "r1"), obs("c", 560, "t", "diff", "r2")]
        assert l4.classify_flaky(rows, min_runs=3) == {}

    def test_consistent_verdicts_not_flaky(self):
        rows = [obs("c", 560, "t", v, f"r{i}") for i, v in enumerate(["match"] * 4)]
        assert l4.classify_flaky(rows, min_runs=3) == {}

    def test_disagreeing_verdicts_flaky_with_record(self):
        rows = [obs("c", 560, "t", v, f"r{i}")
                for i, v in enumerate(["match", "match", "diff"])]
        fl = l4.classify_flaky(rows, min_runs=3)
        assert ("c", 560, "t") in fl
        rec = fl[("c", 560, "t")]
        assert sorted(rec["verdicts"]) == ["diff", "match", "match"]
        assert rec["n"] == 3


    def test_cross_code_state_not_flaky(self):
        """跨码态 verdict 漂移=修复轨迹非非确定性——不判 flaky（2026-11-04 根修）。"""
        rows = [obs("c", 560, "t", "diff", "r1", code_state="sha_old"),
                obs("c", 560, "t", "match", "r2", code_state="sha_new"),
                obs("c", 560, "t", "match", "r3", code_state="sha_new")]
        assert l4.classify_flaky(rows, min_runs=3) == {}

    def test_legacy_rows_no_code_state_excluded(self):
        """无 code_state 旧账行排除出判定（历史修复轨迹不可比——保留不删）。"""
        rows = [{"run_id": f"r{i}", "case": "c", "frame": 560, "tag": "t", "verdict": v}
                for i, v in enumerate(["match", "diff", "match"])]
        assert l4.classify_flaky(rows, min_runs=3) == {}


class TestMatrixFlakyDisposition:
    """矩阵处置：flaky 格留痕+不计分母不计 pass（防假绿与静默豁免双面）。"""

    def _matrix(self):
        rows = [{
            "case": "c", "case_file": "cases/c.txt",
            "scenario_sha256": "s", "rom_sha256": "r", "sched_sha256": "h",
            "artifacts": {}, "snapshot": None, "report": None,
            "legs": {"rust": "fresh", "emu": "fresh"}, "error": None,
            "cells": [
                {"frame": 560, "tag": "a", "state": "match",
                 "provenance": {"rust": "fresh", "emu": "fresh"}, "file": None,
                 "diff_pixels": 0, "total_pixels": 1, "reason": None},
                {"frame": 700, "tag": "b", "state": "match",
                 "provenance": {"rust": "fresh", "emu": "fresh"}, "file": None,
                 "diff_pixels": 0, "total_pixels": 1, "reason": None},
            ],
        }]
        return rows

    def test_flaky_cell_marked_and_excluded(self):
        rows = self._matrix()
        fl = {("c", 560, "a"): {"verdicts": ["match", "diff", "match"],
                               "runs": ["r1", "r2", "r3"], "n": 3}}
        m = l4.build_matrix(rows, flaky=fl)
        cells = {c["tag"]: c for c in m["rows"][0]["cells"]}
        assert cells["a"]["state"] == "flaky"
        assert cells["a"]["observations"] == fl[("c", 560, "a")]  # 留痕随格
        assert cells["b"]["state"] == "match"
        # flaky 不计分母不锚值：分母=1（只剩 b）；match=1
        assert m["denominator"] == 1
        assert m["match"] == 1 and m["diff"] == 0 and m["blocked"] == 0
        assert ("c", 560, "a") in {(x["case"], x["frame"], x["tag"])
                                  for x in m["flaky_cells"]}

    def test_no_flaky_unchanged(self):
        m = l4.build_matrix(self._matrix(), flaky={})
        assert m["denominator"] == 2 and m["match"] == 2


class TestMatrixMdFlakyRender:
    """【H3 回归锚（2026-11-04）】matrix_md flaky 态渲染——flaky schema 全链补装。

    红态实录（H3 修复轮）：classify_flaky 出 flaky 格后 matrix_md 图标表
    `{"match","diff","blocked"}[c["state"]]` 直接下标 ⇒ **KeyError: 'flaky'** ⇒
    run-all 全量拒跑（exit 2）——flaky schema（146b5ede）漏装 matrix_md 渲染面。
    """

    def test_matrix_md_renders_flaky_cell(self):
        rows = [{
            "case": "c", "case_file": "cases/c.txt",
            "scenario_sha256": "s", "rom_sha256": "r", "sched_sha256": "h",
            "artifacts": {}, "snapshot": None, "report": None,
            "legs": {"rust": "fresh", "emu": "fresh"}, "error": None,
            "cells": [
                {"frame": 560, "tag": "a", "state": "match",
                 "provenance": {"rust": "fresh", "emu": "fresh"}, "file": None,
                 "diff_pixels": 0, "total_pixels": 1, "reason": None},
            ],
        }]
        fl = {("c", 560, "a"): {"verdicts": ["match", "diff", "match"],
                               "runs": ["r1", "r2", "r3"], "n": 3}}
        m = l4.build_matrix(rows, flaky=fl)
        md = l4.matrix_md(m)  # 不得 KeyError: 'flaky'
        assert "flaky" in md
        assert "560" in md

    def test_matrix_md_renders_all_states(self):
        rows = [{
            "case": "c", "case_file": "cases/c.txt",
            "scenario_sha256": "s", "rom_sha256": "r", "sched_sha256": "h",
            "artifacts": {}, "snapshot": None, "report": None,
            "legs": {"rust": "fresh", "emu": "fresh"}, "error": None,
            "cells": [
                {"frame": 560, "tag": "a", "state": "match",
                 "provenance": {"rust": "fresh", "emu": "fresh"}, "file": None,
                 "diff_pixels": 0, "total_pixels": 1, "reason": None},
                {"frame": 700, "tag": "b", "state": "diff",
                 "provenance": {"rust": "fresh", "emu": "fresh"}, "file": None,
                 "diff_pixels": 3, "total_pixels": 1, "reason": None},
                {"frame": 800, "tag": "c", "state": "blocked",
                 "provenance": {"rust": "none", "emu": "none"}, "file": None,
                 "diff_pixels": None, "total_pixels": None, "reason": "x"},
            ],
        }]
        fl = {("c", 560, "a"): {"verdicts": ["match", "diff", "match"],
                               "runs": ["r1", "r2", "r3"], "n": 3}}
        md = l4.matrix_md(l4.build_matrix(rows, flaky=fl))
        assert "flaky" in md and "diff" in md and "blocked" in md
