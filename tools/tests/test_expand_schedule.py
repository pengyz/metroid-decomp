#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""tools/tests/test_expand_schedule.py —— Task 2 输入单源 + 语义决议 测试（TDD 先红后绿）。

覆盖（task-2-brief Step 1）：
1. expand_schedule(plan) -> {frame: joy1} 逐帧展开（press/hold/press_len/对向键后写覆盖）；
2. anti-drift 硬判据：**同一 plan，Rust 侧与 Lua 侧消费的逐帧 joy1 完全相同**
   ——expand_schedule 产物 == 生成 Lua（PLAN.schedule）消费的调度 == Rust .sched 的 joy 行；
3. 语义决议：`snapshot <path>` ≡ `bundle_load <path>`（bundle_load=别名保留）、
   `endFrame <f>` ≡ `capture <f> end`（同帧已有 capture 则去重）；
4. 无回归锚：改造前后同输入同帧 joy1 hash 不变（冻结值=改造前 tools/l4_diff_run.py::build_schedule
   实测输出，2026-10-26 取样）；
5. 单源守卫：build_schedule/addRange 已删，生成 Lua 持有 raw presses/holds 不再可展开。
"""
from __future__ import annotations

import hashlib
import json
import re
import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parent.parent.parent
sys.path.insert(0, str(ROOT / "tools"))

import scenario_run as sr  # noqa: E402
import l4_diff_run as l4  # noqa: E402

CASES = ROOT / "cases"
HARNESS = ROOT / "tools" / "scenario_capture.lua"


# ---------------- 冻结基线（改造前 build_schedule 实测输出——无回归锚） ----------------
def joy_canon(joy: dict) -> bytes:
    """逐帧 joy1 的规范序列化（与 2026-10-26 基线取样同式）。"""
    return json.dumps({str(k): v for k, v in sorted(joy.items())}, sort_keys=True).encode()


CASE001_JOY_SHA256 = "03778026161b919c5dde79582684548e569b5894040107415498fcf41609dd9a"  # cases/（1200 帧）
ORACLE001_JOY_SHA256 = "e4b43358b669b59b71895f66b4b595f71cc2c73c68f799581f9c532a487538f9"  # oracle（1800 帧）


def synth_plan(**kw) -> dict:
    base = {"frames": 100, "press_len": 2, "presses": [], "holds": [], "captures": []}
    base.update(kw)
    return base


def write_scen(tmp_path: Path, text: str) -> Path:
    p = tmp_path / "scen.txt"
    p.write_text(text, encoding="utf-8")
    return p


# ---------------- ① expand_schedule 逐帧展开 ----------------

class TestExpandSchedule:
    def test_press_basic(self):
        joy = sr.expand_schedule(synth_plan(presses=[{"frame": 3, "buttons": ["a"]}]))
        assert dict(sorted(joy.items())) == {3: 0x80, 4: 0x80}

    def test_hold_basic(self):
        joy = sr.expand_schedule(synth_plan(holds=[{"from": 10, "to": 12, "buttons": ["right"]}]))
        assert dict(sorted(joy.items())) == {10: 0x01, 11: 0x01, 12: 0x01}

    def test_press_len(self):
        joy = sr.expand_schedule(synth_plan(press_len=3, presses=[{"frame": 5, "buttons": ["b"]}]))
        assert dict(sorted(joy.items())) == {5: 0x40, 6: 0x40, 7: 0x40}

    def test_multi_button_or(self):
        joy = sr.expand_schedule(synth_plan(presses=[{"frame": 2, "buttons": ["a", "b", "start"]}]))
        assert dict(sorted(joy.items())) == {2: 0x80 | 0x40 | 0x10, 3: 0x80 | 0x40 | 0x10}

    def test_opposing_left_right_last_write_wins(self):
        # hold 1-10 left + press 5 right（press 晚处理=后写覆盖）→ 帧 5-6 right 胜
        joy = sr.expand_schedule(synth_plan(
            holds=[{"from": 1, "to": 10, "buttons": ["left"]}],
            presses=[{"frame": 5, "buttons": ["right"]}]))
        assert dict(sorted(joy.items())) == {
            1: 0x02, 2: 0x02, 3: 0x02, 4: 0x02, 5: 0x01, 6: 0x01,
            7: 0x02, 8: 0x02, 9: 0x02, 10: 0x02}

    def test_opposing_same_line_last_listed_wins(self):
        # press 3 left right → 右（列后者）胜，帧 3-4
        joy = sr.expand_schedule(synth_plan(presses=[{"frame": 3, "buttons": ["left", "right"]}]))
        assert dict(sorted(joy.items())) == {3: 0x01, 4: 0x01}

    def test_opposing_up_down(self):
        joy = sr.expand_schedule(synth_plan(presses=[{"frame": 7, "buttons": ["up", "down"]}]))
        assert dict(sorted(joy.items())) == {7: 0x04, 8: 0x04}

    def test_non_conflicting_press_span(self):
        joy = sr.expand_schedule(synth_plan(
            holds=[{"from": 5, "to": 6, "buttons": ["right"]}],
            presses=[{"frame": 1, "buttons": ["left"]}]))
        assert dict(sorted(joy.items())) == {1: 0x02, 2: 0x02, 5: 0x01, 6: 0x01}

    def test_button_bits_mapping(self):
        # 按钮位（game-platform PadState / hardware.asm:89-96）：A=bit7 … Right=bit0
        assert sr.BUTTON_BITS == {
            "right": 0x01, "left": 0x02, "down": 0x04, "up": 0x08,
            "start": 0x10, "select": 0x20, "b": 0x40, "a": 0x80}


# ---------------- ② 无回归锚（同输入同帧 hash 不变） ----------------

class TestNoRegression:
    def test_cases_case001_joy_hash_frozen(self):
        plan = sr.parse_scenario(CASES / "case-001-game-start-samus-interaction.txt")
        assert plan["frames"] == 1200
        joy = sr.expand_schedule(plan)
        assert len(joy) == 161
        assert hashlib.sha256(joy_canon(joy)).hexdigest() == CASE001_JOY_SHA256

    def test_oracle_case001_joy_hash_frozen(self):
        plan = sr.parse_scenario(ROOT / "oracle" / "scenarios" /
                                 "case-001-game-start-samus-interaction.txt")
        assert plan["frames"] == 1800
        joy = sr.expand_schedule(plan)
        assert len(joy) == 311
        assert hashlib.sha256(joy_canon(joy)).hexdigest() == ORACLE001_JOY_SHA256


# ---------------- ③ anti-drift：Rust 侧与 Lua 侧消费同一产物 ----------------

class TestAntiDrift:
    def test_lua_plan_schedule_equals_expand_schedule(self):
        """生成 Lua 嵌入的 PLAN.schedule == expand_schedule 产物（逐帧 joy1 完全相同）。"""
        plan = sr.parse_scenario(CASES / "case-001-game-start-samus-interaction.txt")
        joy = sr.expand_schedule(plan)
        src, _sha = sr.generate_lua(plan, CASES / "case-001-game-start-samus-interaction.txt")
        m = re.search(r'\["schedule"\]=\{([^}]*)\}', src)
        assert m, "生成 Lua 未嵌入 [\"schedule\"]（Python 预展开产物）"
        embedded = {int(f): int(v) for f, v in re.findall(r'\[(\d+)\]=(\d+)', m.group(1))}
        assert embedded == joy

    def test_rust_sched_joys_equal_expand_schedule(self, tmp_path):
        """Rust 侧 .sched 的 joy 行 == expand_schedule 产物（逐帧 joy1 完全相同）。"""
        plan = sr.parse_scenario(CASES / "case-001-game-start-samus-interaction.txt")
        joy = sr.expand_schedule(plan)
        caps = [(int(c["frame"]), c["tag"]) for c in plan["captures"]]
        caps.sort()
        sched = tmp_path / "t.sched"
        l4.write_schedule(plan["name"], int(plan["frames"]), joy, caps, sched)
        got = {}
        for line in sched.read_text(encoding="utf-8").splitlines():
            tok = line.split()
            if tok and tok[0] == "joy":
                got[int(tok[1])] = int(tok[2], 16)
        assert got == joy

    def test_both_sides_same_product(self, tmp_path):
        """硬判据显式化：Lua 嵌入调度 == Rust .sched 调度（同 plan 不再各自展开）。"""
        plan = sr.parse_scenario(CASES / "case-001-game-start-samus-interaction.txt")
        src, _ = sr.generate_lua(plan, CASES / "case-001-game-start-samus-interaction.txt")
        m = re.search(r'\["schedule"\]=\{([^}]*)\}', src)
        assert m
        lua_sched = {int(f): int(v) for f, v in re.findall(r'\[(\d+)\]=(\d+)', m.group(1))}
        caps = [(int(c["frame"]), c["tag"]) for c in plan["captures"]]
        caps.sort()
        sched = tmp_path / "t.sched"
        l4.write_schedule(plan["name"], int(plan["frames"]), sr.expand_schedule(plan), caps, sched)
        rust_sched = {}
        for line in sched.read_text(encoding="utf-8").splitlines():
            tok = line.split()
            if tok and tok[0] == "joy":
                rust_sched[int(tok[1])] = int(tok[2], 16)
        assert lua_sched == rust_sched

    def test_generated_lua_has_no_raw_inputs_to_reexpand(self):
        """生成 Lua 不再携带 raw presses/holds（Lua 物理上无法二次展开）。"""
        plan = sr.parse_scenario(CASES / "case-001-game-start-samus-interaction.txt")
        src, _ = sr.generate_lua(plan, CASES / "case-001-game-start-samus-interaction.txt")
        assert '["schedule"]=' in src
        assert '["presses"]=' not in src
        assert '["holds"]=' not in src
        assert "addRange" not in src

    def test_harness_consumes_only(self):
        """scenario_capture.lua 只消费 PLAN.schedule：addRange 与逐段展开循环已删。"""
        text = HARNESS.read_text(encoding="utf-8")
        assert "addRange" not in text
        assert "ipairs(PLAN.presses)" not in text
        assert "ipairs(PLAN.holds)" not in text
        assert "PLAN.schedule" in text

    def test_no_second_expansion_source(self):
        """第二输入展开源零命中（build_schedule 已删；l4 侧改调 expand_schedule）。"""
        l4_src = (ROOT / "tools" / "l4_diff_run.py").read_text(encoding="utf-8")
        assert "build_schedule" not in l4_src
        assert "expand_schedule" in l4_src
        sr_src = (ROOT / "tools" / "scenario_run.py").read_text(encoding="utf-8")
        assert sr_src.count("def expand_schedule") == 1


# ---------------- ④ 语义决议：snapshot ≡ bundle_load、endFrame ≡ capture ----------------

class TestSemanticDecisions:
    def test_snapshot_maps_to_bundle_load(self, tmp_path):
        p = write_scen(tmp_path, "frames 10\nsnapshot data/bundles/foo\n")
        plan = sr.parse_scenario(p)
        assert plan["bundle_load"] == {"path": "data/bundles/foo"}

    def test_bundle_load_alias_still_works(self, tmp_path):
        p = write_scen(tmp_path, "frames 10\nbundle_load data/bundles/foo\n")
        plan = sr.parse_scenario(p)
        assert plan["bundle_load"] == {"path": "data/bundles/foo"}

    def test_snapshot_none_means_no_snapshot(self, tmp_path):
        p = write_scen(tmp_path, "frames 10\nsnapshot none\n")
        plan = sr.parse_scenario(p)
        assert plan["bundle_load"] is None

    def test_endframe_is_terminal_capture(self, tmp_path):
        p = write_scen(tmp_path, "frames 200\nendFrame 200\n")
        plan = sr.parse_scenario(p)
        assert plan["captures"] == [{"frame": 200, "tag": "end"}]

    def test_endframe_dedupes_existing_capture_same_frame(self, tmp_path):
        p = write_scen(tmp_path, "frames 200\ncapture 200 end-state\nendFrame 200\n")
        plan = sr.parse_scenario(p)
        assert plan["captures"] == [{"frame": 200, "tag": "end-state"}]

    def test_endframe_other_frame_adds_capture(self, tmp_path):
        p = write_scen(tmp_path, "frames 200\ncapture 150 mid\nendFrame 150\nendFrame 200\n")
        plan = sr.parse_scenario(p)
        assert plan["captures"] == [{"frame": 150, "tag": "mid"},
                                    {"frame": 200, "tag": "end"}]

    def test_case001_authoritative_captures(self):
        """cases/case-001 权威定义：采集点对齐 + endFrame 终端采集点（tag=end）。"""
        plan = sr.parse_scenario(CASES / "case-001-game-start-samus-interaction.txt")
        caps = [(int(c["frame"]), c["tag"]) for c in plan["captures"]]
        assert sorted(caps) == [(420, "game-in"), (550, "after-right"), (650, "after-left"),
                                (720, "after-jump"), (820, "after-shoot"),
                                (950, "combo-action"), (1200, "end")]
        assert plan["bundle_load"] is None  # snapshot none
