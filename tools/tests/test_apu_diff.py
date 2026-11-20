#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""tools/tests/test_apu_diff.py —— APU 寄存器面对拍工具测试（判据仪器 2026-11-20）。

覆盖（tools/apu_diff.py 契约面）：
1. parse_apu_text："%04X %02X" 逐行解析；格式漂移 fail-loud（ValueError）；
2. load_side：目录按帧号 _f(\\d+)_ 收集；缺目录/缺产物=错误；
3. diff_frame：全等/逐写差异/长度差异首分歧定位；
4. main 退出码契约：0=全等 / 1=分歧或缺帧 / 2=参数或格式错。
"""
from __future__ import annotations

import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parent.parent.parent
sys.path.insert(0, str(ROOT / "tools"))

import apu_diff  # noqa: E402


def test_parse_apu_text_ok_and_strict():
    assert apu_diff.parse_apu_text("4000 0F\n4015 01\n", "t") == [(0x4000, 0x0F), (0x4015, 0x01)]
    assert apu_diff.parse_apu_text("\n", "t") == []  # 空桶
    with pytest.raises(ValueError):
        apu_diff.parse_apu_text("4000:0F\n", "t")  # 格式漂移拒


def test_load_side_dir_and_missing(tmp_path: Path):
    d = tmp_path / "emu"
    d.mkdir()
    (d / "case_f00005_mid.apu.hex").write_text("4000 0F\n", encoding="ascii")
    (d / "case_f00010_end.apu.hex").write_text("", encoding="ascii")
    faces = apu_diff.load_side(d)
    assert faces == {5: [(0x4000, 0x0F)], 10: []}
    empty = tmp_path / "none"
    empty.mkdir()
    with pytest.raises(ValueError):
        apu_diff.load_side(empty)  # 无产物 fail-loud
    with pytest.raises(ValueError):
        apu_diff.load_side(tmp_path / "nope")  # 非文件非目录


def test_diff_frame_first_divergence():
    a = [(0x4000, 0x01), (0x4015, 0x03)]
    assert apu_diff.diff_frame(a, a) is None
    b = [(0x4000, 0x01), (0x4015, 0x02)]
    out = apu_diff.diff_frame(a, b)
    assert out is not None and "写#1" in out, "首分歧写#1（值差）"
    out = apu_diff.diff_frame(a, [(0x4000, 0x01)])
    assert out is not None and "写#1" in out, "首分歧写#1（缺写）"


def test_collect_events_register_face():
    # mute=$4015 00；enable=非零位形；lenload=$4003/$4007/$400B/$400F 装载
    w = [(0x4015, 0x00), (0x4015, 0x0F), (0x4003, 0x08), (0x400B, 0x10), (0x4000, 0xBF)]
    assert apu_diff.collect_events(w) == ["mute", "enable=0F", "lenload=4003,400B"]
    assert apu_diff.collect_events([(0x4000, 0xBF)]) == []  # 无事件帧
    # 纯寄存器面纪律：不做 music/sfx 语义断言（enable 只带位形）


def test_event_compare_lists_mismatch(capsys):
    emu = {5: [(0x4015, 0x0F)], 7: [(0x4015, 0x00)]}
    rust = {5: [(0x4015, 0x0F)], 8: [(0x4015, 0x00)]}
    apu_diff.event_compare(emu, rust)
    text = capsys.readouterr().out
    assert "emu 事件帧 2" in text and "rust 事件帧 2" in text
    assert "f5" not in text.splitlines()[1], "同帧同事件不刷明细"
    assert "f7" in text and "f8" in text, "差异帧逐条对照"


def test_main_exit_codes(tmp_path: Path, capsys):
    emu = tmp_path / "emu"
    rust = tmp_path / "rust"
    for d in (emu, rust):
        d.mkdir()
    (emu / "c_f00005_t.apu.hex").write_text("4000 0F\n", encoding="ascii")
    (rust / "c_f00005_t.apu.hex").write_text("4000 0F\n", encoding="ascii")
    assert apu_diff.main(["apu_diff", str(emu), str(rust)]) == 0  # 全等
    (rust / "c_f00005_t.apu.hex").write_text("4000 0E\n", encoding="ascii")
    assert apu_diff.main(["apu_diff", str(emu), str(rust)]) == 1  # 分歧
    (rust / "c_f00006_t.apu.hex").write_text("", encoding="ascii")
    assert apu_diff.main(["apu_diff", str(emu), str(rust)]) == 1  # 缺帧也算 1
    assert apu_diff.main(["apu_diff", str(emu)]) == 2  # 参数错
    (rust / "bad_f00007_t.apu.hex").write_text("zz\n", encoding="ascii")
    assert apu_diff.main(["apu_diff", str(emu), str(rust)]) == 2  # 格式错=2
    text = capsys.readouterr().out
    assert "汇总：全等" in text and "首分歧帧" in text
