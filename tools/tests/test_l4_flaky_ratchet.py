#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""tools/tests/test_l4_flaky_ratchet.py —— flaky 态 ratchet 豁免面 测试（门禁件配套）。

B 径裁定（fade-blink-capture-race §3b）：flaky 格（判据=多 run N 次观测记录）
**不锚值**——ratchet ①match 缩水/③未锚定 对 flaky 格豁免；**普通回退照判**
（match→diff 仍 FAIL——防弱化面）；其余格照常检查。处置记录=matrix.flaky_cells 留痕。
"""
from __future__ import annotations

import importlib.util
import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parent.parent.parent
sys.path.insert(0, str(ROOT / "tools"))

_spec = importlib.util.spec_from_file_location("gates_cli_flaky_rt", ROOT / "gates" / "cli.py")
gates = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(gates)


class TestRatchetFlakyExemption:
    """ratchet 豁免面：flaky 不锚值（①不判缩水）；普通回退照判（防弱化）。"""

    def _baseline(self, state="match"):
        return {"rom_sha256": "r", "cases": {
            "c": {"scenario_sha256": "s", "states": {"560:a": state, "700:b": "match"}}}}

    def _matrix(self, s560):
        return {"rom_sha256": "r", "rows": [{
            "case": "c", "scenario_sha256": "s",
            "cells": [
                {"frame": 560, "tag": "a", "state": s560},
                {"frame": 700, "tag": "b", "state": "match"},
            ]}]}

    def test_flaky_cell_exempt_from_match_shrink(self):
        viol = gates.l4_ratchet_violations(self._baseline(), self._matrix("flaky"))
        assert not any("560:a" in v for v in viol), viol

    def test_plain_regression_still_violates(self):
        viol = gates.l4_ratchet_violations(self._baseline(), self._matrix("diff"))
        assert any("560:a" in v and "match 集合缩水" in v for v in viol), viol

    def test_other_cells_still_checked_with_flaky_present(self):
        m = self._matrix("flaky")
        m["rows"][0]["cells"][1]["state"] = "diff"
        viol = gates.l4_ratchet_violations(self._baseline(), m)
        assert any("700:b" in v for v in viol), viol
