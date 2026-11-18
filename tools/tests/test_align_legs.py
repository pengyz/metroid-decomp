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

from tools.movie_ref.align_legs import align, load_latch


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
