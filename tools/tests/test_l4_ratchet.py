#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""tools/tests/test_l4_ratchet.py —— Task 6 L4 门禁 + 白名单 + ratchet 测试（TDD 先红后绿）。

覆盖（task-6-brief Step 1 + 验收②③）：
1. **ratchet 逐采集点锚（非聚合率）**（验收②核心反向测试）：
   - 某采集点 match→diff = 回退 ⇒ **FAIL**（含「9/10→9/10 换坏」等聚合率不变案）；
   - 新增 blocked = 回退 ⇒ **FAIL**（blocked 只减不增）；
   - match 集合只增不减：新增 match/缩小 blocked/改善 diff→match ⇒ 不违规；
   - 基线缺/坏、rom 锚漂移 ⇒ fail-closed（禁 vacuous 通过）。
2. **白名单接 diff**（验收③——`gates/visual_whitelist.json` 唯一例外）：
   白名单 rect 内像素差异排除后判 match；rect 外/文件 glob 不命中照常 diff；
   空白名单=零容差；**不得**引入 visual_gap_map 之类账内容忍（防容忍面增殖，源级守卫）。
3. **gate_l4 调 l4_diff_run run-all 出裁定**（验收①的判据面）：run-all 产物矩阵 +
   ratchet ⇒ PASS/FAIL；拒跑（fail-loud exit 2）⇒ FAIL；前置未立 ⇒ SKIP（已立后消失⇒FAIL）。
4. **基线唯一写入口**（tools/baseline_update.py l4）：只升不降（缩水/新增 blocked 拒绝）
   + 强制 --reason 留痕。

判定逻辑单源在 gates/（AGENTS：工具自身不持有判定权）——本测试钉死
`gates/cli.py: l4_matrix_states / l4_ratchet_violations / load_l4_ratchet_baseline / gate_l4`
与 `tools/baseline_update.py: l4_update` 的契约。
"""
from __future__ import annotations

import importlib.util
import json
import sys
import types
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parent.parent.parent
sys.path.insert(0, str(ROOT / "tools"))

import l4_diff_run as l4  # noqa: E402

# 判定逻辑单源=gates/cli.py（工具不持判定权——importlib 载入，非第二份实现）
_spec = importlib.util.spec_from_file_location("gates_cli_ut", ROOT / "gates" / "cli.py")
gates = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(gates)

import baseline_update as bu  # noqa: E402


# ---------------- 造数助手（矩阵/基线=同一 schema 契约） ----------------

def make_matrix(states_by_case: dict[str, dict[str, str]], rom_sha: str = "r" * 64) -> dict:
    """states_by_case={case: {"<frame>:<tag>": match|diff|blocked}} → matrix.json 契约 dict。"""
    rows = []
    for case, states in states_by_case.items():
        cells = []
        for key, state in states.items():
            frame_s, tag = key.split(":", 1)
            cells.append({"frame": int(frame_s), "tag": tag, "state": state,
                          "provenance": {"rust": "fresh", "emu": "fresh"},
                          "file": f"{case}_{frame_s}_{tag}.png",
                          "diff_pixels": 0 if state == "match" else 3,
                          "total_pixels": 64, "reason": None})
        rows.append({"case": case, "case_file": f"cases/{case}.txt",
                     "scenario_sha256": "s" * 64, "rom_sha256": rom_sha,
                     "sched_sha256": "h" * 64, "artifacts": {}, "snapshot": None,
                     "report": None, "legs": {"rust": "fresh", "emu": "fresh"},
                     "error": None, "cells": cells})
    total = sum(len(r["cells"]) for r in rows)
    match = sum(1 for r in rows for c in r["cells"] if c["state"] == "match")
    diff = sum(1 for r in rows for c in r["cells"] if c["state"] == "diff")
    blocked = sum(1 for r in rows for c in r["cells"] if c["state"] == "blocked")
    return {"generated_by": "tools/l4_diff_run.py run-all", "rom_sha256": rom_sha,
            "cases_total": len(rows), "cells_total": total, "denominator": total,
            "denominator_note": "分母=用例×采集点总数（含 blocked）",
            "match": match, "diff": diff, "blocked": blocked, "pass": match,
            "pass_rate": f"{match}/{total}",
            "verdict": "pass" if total > 0 and match == total else "not-pass",
            "rows": rows}


def make_baseline(states_by_case: dict[str, dict[str, str]], rom_sha: str = "r" * 64) -> dict:
    return {"schema": "l4-ratchet/v1", "doc": "test", "rom_sha256": rom_sha,
            "cases": {case: {"scenario_sha256": "s" * 64, "states": dict(states)}
                      for case, states in states_by_case.items()},
            "history": []}


def make_png(path: Path, size=(8, 8), color=(10, 20, 30)):
    from PIL import Image
    Image.new("RGB", size, color).save(path)


def make_png_with_pixel(path: Path, xy, color, size=(8, 8), bg=(10, 20, 30)):
    from PIL import Image
    im = Image.new("RGB", size, bg)
    im.putpixel(xy, color)
    im.save(path)


# ---------------- ① ratchet 逐采集点锚（验收②反向测试——先红） ----------------

class TestRatchetPerCapturePoint:
    def test_match_to_diff_is_regression(self):
        """反向①：某采集点 match→diff（match 集合缩水）⇒ 违规 FAIL。"""
        base = make_baseline({"c": {"1:a": "match", "2:b": "match"}})
        cur = make_matrix({"c": {"1:a": "diff", "2:b": "match"}})
        viol = gates.l4_ratchet_violations(base, cur)
        assert viol, "match→diff 必须判回退（非聚合率——锚逐采集点）"
        assert any("1:a" in v and "缩水" in v for v in viol), viol

    def test_match_to_blocked_is_regression(self):
        """match→blocked ⇒ 违规（match 缩水 + 新增 blocked 双向触发）。"""
        base = make_baseline({"c": {"1:a": "match"}})
        cur = make_matrix({"c": {"1:a": "blocked"}})
        viol = gates.l4_ratchet_violations(base, cur)
        assert viol, "match→blocked 必须判回退"
        assert any("1:a" in v for v in viol), viol

    def test_new_blocked_is_regression(self):
        """反向②：新增 blocked（blocked 只减不增）⇒ 违规 FAIL。"""
        base = make_baseline({"c": {"1:a": "diff", "2:b": "diff"}})
        cur = make_matrix({"c": {"1:a": "diff", "2:b": "blocked"}})
        viol = gates.l4_ratchet_violations(base, cur)
        assert viol, "新增 blocked 必须判回退"
        assert any("2:b" in v and "blocked" in v for v in viol), viol

    def test_swapped_regression_caught_at_same_aggregate(self):
        """防 9/10→9/10 换坏不触发：两采集点 match/diff 互换（一致率不变）⇒ 仍 FAIL。"""
        states_b = {f"{i}:p": ("match" if i <= 9 else "diff") for i in range(1, 11)}
        states_c = dict(states_b)
        states_c["1:p"], states_c["10:p"] = "diff", "match"   # 换坏：1 号 match→diff
        viol = gates.l4_ratchet_violations(make_baseline({"c": states_b}),
                                           make_matrix({"c": states_c}))
        assert viol and any("1:p" in v for v in viol), \
            f"聚合率 9/10 不变但逐采集点换坏必须触发: {viol}"

    def test_match_growth_not_violation(self):
        """match 集合只增不减：新增 match/新采集点 ⇒ 不违规。"""
        base = make_baseline({"c": {"1:a": "match", "2:b": "diff"}})
        cur = make_matrix({"c": {"1:a": "match", "2:b": "match", "3:c": "match"}})
        assert gates.l4_ratchet_violations(base, cur) == [], "match 增长=改善，不违规"

    def test_blocked_shrink_not_violation(self):
        """blocked 只减不增：blocked→match/diff（缩小）⇒ 不违规。"""
        base = make_baseline({"c": {"1:a": "blocked", "2:b": "blocked"}})
        cur = make_matrix({"c": {"1:a": "match", "2:b": "diff"}})
        assert gates.l4_ratchet_violations(base, cur) == [], "blocked 缩小=改善，不违规"

    def test_diff_persisting_not_violation(self):
        """diff 现状持平（锚定非聚合率——不是一致率百分比只升）⇒ 不违规。"""
        base = make_baseline({"c": {"1:a": "match", "2:b": "diff", "3:c": "diff"}})
        cur = make_matrix({"c": {"1:a": "match", "2:b": "diff", "3:c": "diff"}})
        assert gates.l4_ratchet_violations(base, cur) == []

    def test_vanished_match_point_is_regression(self):
        """基线 match 采集点在现矩阵消失（DSL 改删）⇒ match 缩水照常 FAIL（防删点逃逸）。"""
        base = make_baseline({"c": {"1:a": "match", "2:b": "match"}})
        cur = make_matrix({"c": {"2:b": "match"}})
        viol = gates.l4_ratchet_violations(base, cur)
        assert viol and any("1:a" in v for v in viol), viol

    def test_rom_anchor_drift_is_violation(self):
        """基线 rom_sha256 与矩阵锚漂移 ⇒ fail-closed 违规。"""
        base = make_baseline({"c": {"1:a": "match"}}, rom_sha="a" * 64)
        cur = make_matrix({"c": {"1:a": "match"}}, rom_sha="b" * 64)
        viol = gates.l4_ratchet_violations(base, cur)
        assert viol and any("rom" in v.lower() or "锚" in v for v in viol), viol


class TestRatchetBaselineFailClosed:
    def test_missing_baseline_raises(self, tmp_path):
        """基线缺失 ⇒ BaselineFormatError（禁 vacuous 通过——删基线不得静默绿）。"""
        with pytest.raises(gates.BaselineFormatError):
            gates.load_l4_ratchet_baseline(tmp_path / "nope.json")

    def test_broken_baseline_raises(self, tmp_path):
        p = tmp_path / "b.json"
        p.write_text("{broken json\n", encoding="utf-8")
        with pytest.raises(gates.BaselineFormatError):
            gates.load_l4_ratchet_baseline(p)

    def test_bad_state_value_raises(self, tmp_path):
        p = tmp_path / "b.json"
        p.write_text(json.dumps({"schema": "l4-ratchet/v1", "rom_sha256": "r",
                                 "cases": {"c": {"states": {"1:a": "green"}}}}),
                     encoding="utf-8")
        with pytest.raises(gates.BaselineFormatError):
            gates.load_l4_ratchet_baseline(p)

    def test_real_baseline_loads(self):
        """实仓 data/status/l4_ratchet_baseline.json 载入合法（建立后）。"""
        p = ROOT / "data" / "status" / "l4_ratchet_baseline.json"
        if not p.is_file():
            pytest.skip("ratchet 基线尚未引导（Task 6 Step 5 建立后自动覆盖）")
        doc = gates.load_l4_ratchet_baseline()
        assert doc["schema"] == "l4-ratchet/v1"
        assert doc["cases"], "基线须锚逐采集点（cases 非空）"


# ---------------- ② gate_l4 调 run-all 出裁定（验收①判据面） ----------------

class TestGateL4Verdict:
    @pytest.fixture()
    def gate_env(self, tmp_path, monkeypatch):
        """假 ROOT：tools/l4_diff_run.py + cases/ + 矩阵 + 基线；subprocess.run 记录命令并
        **于调用时落盘矩阵**（模拟 run-all 本轮真落盘——gate 矩阵新鲜度校验（C1）的前提）。"""
        (tmp_path / "tools").mkdir()
        (tmp_path / "tools" / "l4_diff_run.py").write_text("# runner", encoding="utf-8")
        (tmp_path / "cases").mkdir()
        (tmp_path / "cases" / "case-x.txt").write_text("name case-x\n", encoding="utf-8")
        (tmp_path / "shots_rust").mkdir()
        (tmp_path / "data" / "status").mkdir(parents=True)
        calls: list[list[str]] = []
        env: dict = {"root": tmp_path, "calls": calls, "matrix_payload": None}

        def fake_run(cmd, **kw):
            calls.append([str(c) for c in cmd])
            if env["matrix_payload"] is not None:  # run-all 本轮落盘同名新产物
                # temp+rename 原子落盘（与 l4_diff_run.atomic_write_text 同契约——指纹
                # inode 变化=本轮真落盘；ext4 mtime 粒度 ~1ms，裸重写同内容可能指纹不变）
                l4.atomic_write_text(tmp_path / "shots_rust" / "matrix.json",
                                     json.dumps(env["matrix_payload"], ensure_ascii=False))
            return types.SimpleNamespace(returncode=1, stdout="L4 对拍矩阵 …\n", stderr="")

        monkeypatch.setattr(gates, "ROOT", tmp_path)
        monkeypatch.setattr(gates.subprocess, "run", fake_run)
        return env

    def write_matrix(self, env, states, rom_sha="r" * 64, rewrite_on_run=True):
        """落 matrix.json：rewrite_on_run=True=run-all 本轮真落盘（fresh）；False=崩溃不落盘
        （旧/committed 残留——C1 新鲜度必须拒裁）。"""
        payload = make_matrix(states, rom_sha)
        (env["root"] / "shots_rust" / "matrix.json").write_text(
            json.dumps(payload, ensure_ascii=False), encoding="utf-8")
        env["matrix_payload"] = payload if rewrite_on_run else None

    def write_baseline(self, env, states, rom_sha="r" * 64):
        p = env["root"] / "data" / "status" / "l4_ratchet_baseline.json"
        p.write_text(json.dumps(make_baseline(states, rom_sha), ensure_ascii=False),
                     encoding="utf-8")

    def test_gate_l4_runs_run_all(self, gate_env):
        """gate_l4 调 `tools/l4_diff_run.py run-all` 出裁定（决策逐字）。"""
        self.write_matrix(gate_env, {"c": {"1:a": "match"}})
        self.write_baseline(gate_env, {"c": {"1:a": "match"}})
        gates.gate_l4()
        assert gate_env["calls"], "gate_l4 必须实跑 run-all（非读旧产物出裁定）"
        cmd = gate_env["calls"][0]
        assert cmd[1].endswith("l4_diff_run.py") and cmd[2] == "run-all", cmd

    def test_gate_l4_pass_when_ratchet_held(self, gate_env):
        """ratchet 守住（含 diff 现状持平）⇒ PASS；detail 显式含 ratchet/白名单字样。"""
        states = {"c": {"1:a": "match", "2:b": "diff", "3:c": "diff"}}
        self.write_matrix(gate_env, states)
        self.write_baseline(gate_env, states)
        verdict, detail = gates.gate_l4()
        assert verdict == gates.PASS, detail
        assert "ratchet" in detail and "白名单" in detail, detail

    def test_gate_l4_fail_on_match_shrink(self, gate_env):
        """反向①门禁面：match 集合缩水 ⇒ FAIL（换坏一采集点即触发）。"""
        self.write_matrix(gate_env, {"c": {"1:a": "diff", "2:b": "match"}})
        self.write_baseline(gate_env, {"c": {"1:a": "match", "2:b": "match"}})
        verdict, detail = gates.gate_l4()
        assert verdict == gates.FAIL, detail
        assert "1:a" in detail, detail

    def test_gate_l4_fail_on_new_blocked(self, gate_env):
        """反向②门禁面：新增 blocked ⇒ FAIL。"""
        self.write_matrix(gate_env, {"c": {"1:a": "match", "2:b": "blocked"}})
        self.write_baseline(gate_env, {"c": {"1:a": "match", "2:b": "diff"}})
        verdict, detail = gates.gate_l4()
        assert verdict == gates.FAIL, detail
        assert "2:b" in detail, detail

    def test_gate_l4_fail_on_refused_run(self, gate_env, monkeypatch):
        """run-all fail-loud 拒跑（exit 2：分母完整性受损）⇒ FAIL（非静默）。"""
        self.write_matrix(gate_env, {"c": {"1:a": "match"}})
        self.write_baseline(gate_env, {"c": {"1:a": "match"}})

        def fake_run(cmd, **kw):
            return types.SimpleNamespace(returncode=2, stdout="", stderr="cases/ 无对拍用例")

        monkeypatch.setattr(gates.subprocess, "run", fake_run)
        verdict, detail = gates.gate_l4()
        assert verdict == gates.FAIL and "拒跑" in detail, detail

    def test_gate_l4_fail_on_stale_committed_matrix_after_crash(self, gate_env):
        """C1 锁①：run-all 崩溃（exit 1）+旧/committed 矩阵残留（mtime 早于本轮）⇒ FAIL
        ——绝不判旧矩阵出绿（2026-10-26 终审假绿后门封堵）。"""
        self.write_matrix(gate_env, {"c": {"1:a": "match"}}, rewrite_on_run=False)
        self.write_baseline(gate_env, {"c": {"1:a": "match"}})
        import os
        import time
        p = gate_env["root"] / "shots_rust" / "matrix.json"
        old = time.time() - 3600
        os.utime(p, (old, old))
        verdict, detail = gates.gate_l4()
        assert verdict == gates.FAIL and "绝不判旧矩阵" in detail, detail
        assert "ratchet 守住" not in detail, "崩溃+旧矩阵判 [PASS]=假绿后门（C1）"

    def test_gate_l4_fail_on_matrix_not_rewritten_this_run(self, gate_env):
        """C1 锁②：本轮未落盘（指纹未变——run-all 崩溃残留旧产物）⇒ FAIL。"""
        self.write_matrix(gate_env, {"c": {"1:a": "match"}}, rewrite_on_run=False)
        self.write_baseline(gate_env, {"c": {"1:a": "match"}})
        verdict, detail = gates.gate_l4()
        assert verdict == gates.FAIL and "绝不判旧矩阵" in detail, detail

    def test_gate_l4_fail_on_missing_baseline(self, gate_env):
        """基线缺失 ⇒ FAIL（禁 vacuous——删基线不得静默绿）。"""
        self.write_matrix(gate_env, {"c": {"1:a": "match"}})
        verdict, detail = gates.gate_l4()
        assert verdict == gates.FAIL and "基线" in detail, detail

    def test_gate_l4_fail_on_missing_matrix(self, gate_env):
        """run-all 未产出矩阵（或不可解析）⇒ fail-closed FAIL。"""
        self.write_baseline(gate_env, {"c": {"1:a": "match"}})
        verdict, detail = gates.gate_l4()
        assert verdict == gates.FAIL and "matrix" in detail, detail

    def test_gate_l4_skip_without_prereq(self, gate_env):
        """前置未立（cases/ 无用例）⇒ 显式 SKIP；基线已立后消失 ⇒ FAIL『前置对象消失』。"""
        (gate_env["root"] / "cases" / "case-x.txt").unlink()
        verdict, detail = gates.gate_l4()
        assert verdict == gates.SKIP, detail
        self.write_baseline(gate_env, {"c": {"1:a": "match"}})
        verdict, detail = gates.gate_l4()
        assert verdict == gates.FAIL and "前置对象消失" in detail, detail


# ---------------- ③ 白名单接 diff（visual_whitelist.json 唯一例外） ----------------

class TestWhitelistInDiff:
    def diff_pair_of(self, tmp_path: Path, wl, xy=(2, 3), glob="c_*.png"):
        """造一对仅 xy 像素不同的图（8×8）→ diff_pair 结果（wl 条目 file=glob）。"""
        a = tmp_path / "c_1_a.png"
        b = tmp_path / "emu_c_1_a.png"
        make_png_with_pixel(a, xy, (255, 0, 0))
        make_png_with_pixel(b, xy, (0, 0, 255))
        Image, np = l4.load_pil()
        rules = [{"file": glob, "rect": e, "reason": "t", "approved_by": "u"}
                 if isinstance(e, list) else e for e in wl]
        return l4.diff_pair(a, b, None, np, Image, wl=rules)

    def test_whitelisted_rect_diff_becomes_match(self, tmp_path):
        """白名单 rect 内像素差异排除 ⇒ match（唯一例外面接入 diff_pair）。"""
        res = self.diff_pair_of(tmp_path, [[0, 0, 8, 8]])
        assert res["match"] is True, res
        assert res["diff_pixels"] == 0
        assert res.get("whitelist_hits", 0) >= 1, res

    def test_diff_outside_whitelist_still_fails(self, tmp_path):
        """rect 外（或 glob 不命中）差异照常 diff——白名单外零容差不破。"""
        res = self.diff_pair_of(tmp_path, [[0, 0, 4, 4]], xy=(6, 6))
        assert res["match"] is False and res["diff_pixels"] == 1, res

    def test_empty_whitelist_zero_tolerance(self, tmp_path):
        """空白名单（现状）= 零容差：1 像素差异即 diff。"""
        a = tmp_path / "c_1_a.png"
        b = tmp_path / "emu_c_1_a.png"
        make_png_with_pixel(a, (1, 1), (255, 0, 0))
        make_png_with_pixel(b, (1, 1), (0, 0, 255))
        Image, np = l4.load_pil()
        res = l4.diff_pair(a, b, None, np, Image, wl=[])
        assert res["match"] is False and res["diff_pixels"] == 1, res

    def test_file_glob_scope_respected(self, tmp_path):
        """条目 file glob 不命中本帧 ⇒ 不排除（白名单按帧按 rect 定界，不外溢）。"""
        res = self.diff_pair_of(tmp_path, [[0, 0, 8, 8]], glob="other_*.png")
        assert res["match"] is False and res["diff_pixels"] == 1, res

    def test_whitelist_loaded_from_visual_whitelist_json(self, tmp_path, monkeypatch):
        """缺省 wl=None 时读 gates/visual_whitelist.json（唯一例外清单文件本身）。"""
        p = tmp_path / "visual_whitelist.json"
        p.write_text(json.dumps({"schema": 1, "whitelist": [
            {"file": "c_*.png", "rect": [0, 0, 8, 8], "reason": "t", "approved_by": "u"}]}),
            encoding="utf-8")
        monkeypatch.setattr(l4.diff_visual, "WHITELIST", p)
        a = tmp_path / "c_1_a.png"
        b = tmp_path / "emu_c_1_a.png"
        make_png_with_pixel(a, (2, 3), (255, 0, 0))
        make_png_with_pixel(b, (2, 3), (0, 0, 255))
        Image, np = l4.load_pil()
        res = l4.diff_pair(a, b, None, np, Image)
        assert res["match"] is True and res.get("whitelist_hits", 0) >= 1, res

    def test_no_gap_map_tolerance_in_l4_diff(self):
        """防容忍面增殖（决策：不得复增 visual_gap_map 之类账内容忍）：
        l4_diff_run 对拍面只允许 visual_whitelist.json 一个例外源。"""
        src = (ROOT / "tools" / "l4_diff_run.py").read_text(encoding="utf-8")
        assert "visual_gap_map" not in src and "load_gap_map" not in src, \
            "l4_diff_run 不得引入 gap_map 账内容忍（白名单=唯一例外）"


# ---------------- ④ 基线唯一写入口（tools/baseline_update.py l4） ----------------

class TestBaselineUpdateL4:
    def test_bootstrap_creates_baseline_with_reason(self, tmp_path):
        matrix = make_matrix({"c": {"1:a": "match", "2:b": "diff"}})
        mp = tmp_path / "matrix.json"
        mp.write_text(json.dumps(matrix, ensure_ascii=False), encoding="utf-8")
        bp = tmp_path / "l4_ratchet_baseline.json"
        rec = bu.l4_update(mp, bp, reason="Task 6 引导：初始锚")
        doc = json.loads(bp.read_text(encoding="utf-8"))
        assert doc["schema"] == "l4-ratchet/v1"
        assert doc["cases"]["c"]["states"] == {"1:a": "match", "2:b": "diff"}
        assert doc["history"] and doc["history"][-1]["reason"] == "Task 6 引导：初始锚"
        assert rec["reason"] == "Task 6 引导：初始锚"

    def test_update_refuses_match_shrink(self, tmp_path):
        mp = tmp_path / "matrix.json"
        mp.write_text(json.dumps(make_matrix({"c": {"1:a": "diff"}})), encoding="utf-8")
        bp = tmp_path / "l4_ratchet_baseline.json"
        bp.write_text(json.dumps(make_baseline({"c": {"1:a": "match"}})), encoding="utf-8")
        with pytest.raises(Exception) as exc:
            bu.l4_update(mp, bp, reason="试图下调")
        assert "1:a" in str(exc.value), exc.value

    def test_update_refuses_new_blocked(self, tmp_path):
        mp = tmp_path / "matrix.json"
        mp.write_text(json.dumps(make_matrix({"c": {"1:a": "blocked"}})), encoding="utf-8")
        bp = tmp_path / "l4_ratchet_baseline.json"
        bp.write_text(json.dumps(make_baseline({"c": {"1:a": "diff"}})), encoding="utf-8")
        with pytest.raises(Exception) as exc:
            bu.l4_update(mp, bp, reason="试图膨胀 blocked")
        assert "1:a" in str(exc.value), exc.value

    def test_update_refuses_empty_reason(self, tmp_path):
        mp = tmp_path / "matrix.json"
        mp.write_text(json.dumps(make_matrix({"c": {"1:a": "match"}})), encoding="utf-8")
        bp = tmp_path / "l4_ratchet_baseline.json"
        with pytest.raises(ValueError):
            bu.l4_update(mp, bp, reason="   ")

    def test_update_accepts_growth_and_records(self, tmp_path):
        """改善（match 增/blocked 减）⇒ 接受并留痕（states 重锚当前矩阵逐采集点）。"""
        mp = tmp_path / "matrix.json"
        mp.write_text(json.dumps(make_matrix({"c": {"1:a": "match", "2:b": "match"}})),
                      encoding="utf-8")
        bp = tmp_path / "l4_ratchet_baseline.json"
        bp.write_text(json.dumps(make_baseline({"c": {"1:a": "match", "2:b": "diff"}})),
                      encoding="utf-8")
        bu.l4_update(mp, bp, reason="2:b 改善 diff→match——确认无回退")
        doc = json.loads(bp.read_text(encoding="utf-8"))
        assert doc["cases"]["c"]["states"]["2:b"] == "match"
        assert doc["history"][-1]["reason"] == "2:b 改善 diff→match——确认无回退"
