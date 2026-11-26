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


CASE001_JOY_SHA256 = "28fd38817e91bcd290f012da213f65e6ec96a893aa580b34c87cc5bf6612b635"  # cases/（1610 帧；2026-11-16 A-重锚后重钉）
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
        assert plan["frames"] == 1610  # 【2026-11-16 更正】1200→1610：A-重锚后用例权威窗
        joy = sr.expand_schedule(plan)
        assert len(joy) == 163
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
        # 【2026-11-16 更正】A-重锚后权威采集点（旧表 550/650/720/820/950/1200 已被
        # 逐帧证据替换为下列 12 点；本断言此后随 cases/ 单源同步）
        assert sorted(caps) == [(420, "game-in"), (960, "after-right"), (1060, "after-left"),
                                (1125, "jump-shoot"), (1130, "after-jump"),
                                (1135, "landing-early"), (1145, "landing-settled"),
                                (1220, "bullet-midflight"), (1225, "bullet-mid-2"),
                                (1230, "after-shoot"), (1235, "bullet-late"),
                                (1360, "combo-action"), (1610, "end")]
        assert plan["bundle_load"] is None  # snapshot none


def test_expand_schedule_pads_two_player_from_inputlog(tmp_path):
    """2P 通道（2026-11-16）：`expand_pads` 返回 (joy1, joy2)；
    joy2 **只来自录像 p2 列**（手写 DSL 不造第二套语法）。
    asm 锚：`PauseMode ; 07:C9B1` 读 Joy2Status($15) 的 A|UP。"""
    log = tmp_path / "m.inputlog"
    log.write_text("# frames 3\n41 00 00\n00 88 00\n41 00 00\n",
                   encoding="utf-8", newline="\n")
    plan = synth_plan(presses=[])
    plan["input_log"] = str(log)
    joy1, joy2 = sr.expand_pads(plan)
    assert joy1 == {1: 0x41, 3: 0x41}, joy1
    assert joy2 == {2: 0x88}, "2P 只在 p2 非零帧落表"
    # 门面保持向后兼容：expand_schedule == joy1
    assert sr.expand_schedule(plan) == joy1


def test_sched_text_joy2_lines_only_when_present():
    """无 2P 的用例 .sched 字节不变（锚不漂移）；有 2P 才落 joy2 行。"""
    base = l4.schedule_text("n", 5, {1: 0x41}, [(5, "cap")])
    assert "joy2" not in base
    with2 = l4.schedule_text("n", 5, {1: 0x41}, [(5, "cap")], joy2={2: 0x88})
    assert "joy2 2 88" in with2
    assert with2.replace("joy2 2 88\n", "") == base


def test_expand_schedule_consumes_movie_reset_frame(tmp_path):
    """复位帧消费（#111）：inputlog 的 cmd bit0/bit1 是软/硬复位，FCEUX 播片时该帧
    只复位、不产生游戏输入帧 ⇒ 后续录像帧整体后移一帧（旧实现把复位帧当普通帧
    喂进去，整卷输入晚一帧；A 级证据见 docs/findings/tas-injection-phase.md §11）。"""
    log = tmp_path / "m.inputlog"
    log.write_text("# frames 4\n00 00 01\n41 00 00\n42 00 00\n00 00 00\n",
                   encoding="utf-8", newline="\n")
    plan = synth_plan(presses=[])
    plan["input_log"] = str(log)
    joy1, joy2 = sr.expand_pads(plan)
    # 帧1 是复位帧，被消费；原帧2/3 后移到帧3/4
    assert joy1 == {3: 0x41, 4: 0x42}, joy1
    assert joy2 == {}, joy2
    # 无 cmd 的录像不受影响（锚不漂移）
    log2 = tmp_path / "m2.inputlog"
    log2.write_text("# frames 3\n41 00 00\n42 00 00\n00 00 00\n",
                    encoding="utf-8", newline="\n")
    plan2 = synth_plan(presses=[])
    plan2["input_log"] = str(log2)
    assert sr.expand_pads(plan2)[0] == {1: 0x41, 2: 0x42}
