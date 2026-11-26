#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""#115：`tas_replay_diff.py --resume` 的 rust 腿锚自校验。

背景：段结论若只按 index 复用，改了 rust 腿（重建二进制 / 工作树带别人 WIP / 帧表被改动）
后 `--resume` 仍会跳过旧段、报旧首分歧 —— 验收会把「假绿」当成通过。
本测试只覆盖锚比较这一层（不跑 emu/rust）：
  1. 正例：报告锚与段锚一致 ⇒ 允许沿用；
  2. 缺报告级 `rust_anchor` ⇒ fail-loud（旧格式 coverage 不能直接 resume）；
  3. 缺段级 `rust_anchor_sha` ⇒ fail-loud；
  4. HEAD / rust 二进制 / rust 帧表任一变化 ⇒ 判「段结论失效」。
"""
from __future__ import annotations

import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parent.parent.parent
sys.path.insert(0, str(ROOT / "tools"))

import tas_replay_diff as trd  # noqa: E402


def _anchor(**over):
    a = {
        "git_head": "a" * 40,
        "rust_binary_sha256": "deadbeefdeadbeef",
        "frames": 200,
        "rust_trace_sha256": "f" * 64,
    }
    a["anchor_sha"] = trd._anchor_sha(a)
    a.update(over)
    return a


def _report(anchor):
    return {
        "rust_anchor": anchor,
        "segments": [
            {"index": 0, "from": 0, "to": 100, "status": "pass",
             "rust_anchor_sha": anchor["anchor_sha"]},
            {"index": 1, "from": 100, "to": 200, "status": "diverge",
             "rust_anchor_sha": anchor["anchor_sha"]},
        ],
    }


def test_resume_accepts_matching_anchor():
    a = _anchor()
    trd._check_resume_anchor(_report(a), dict(a))  # 不抛即通过
    # 锚指纹必须由内容决定，改一个字段就变
    assert trd._anchor_sha(a) == a["anchor_sha"]
    assert trd._anchor_sha(_anchor(frames=201)) != a["anchor_sha"]


def test_resume_rejects_report_without_anchor():
    a = _anchor()
    rep = _report(a)
    rep.pop("rust_anchor")
    with pytest.raises(trd.ReplayError, match="缺 rust 腿锚"):
        trd._check_resume_anchor(rep, dict(a))


def test_resume_rejects_segment_without_anchor():
    a = _anchor()
    rep = _report(a)
    rep["segments"][1].pop("rust_anchor_sha")
    with pytest.raises(trd.ReplayError, match="段 1 .*缺 rust 腿锚"):
        trd._check_resume_anchor(rep, dict(a))


@pytest.mark.parametrize(
    "field,value,needle",
    [
        ("git_head", "b" * 40, "HEAD"),
        ("rust_binary_sha256", "0" * 16, "rust 二进制 sha"),
        ("rust_trace_sha256", "1" * 64, "rust 帧表 sha"),
    ],
)
def test_resume_rejects_changed_rust_leg(field, value, needle):
    a = _anchor()
    rep = _report(a)
    cur = dict(a)
    cur[field] = value
    with pytest.raises(trd.ReplayError, match="段结论失效") as ei:
        trd._check_resume_anchor(rep, cur)
    assert needle in str(ei.value)
