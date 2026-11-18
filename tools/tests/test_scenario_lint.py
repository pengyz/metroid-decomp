"""scenario lint：长场景「前 700 帧内至少一次 Start」不变量的两路面。

判据是机械的（展开后的输入位），不是文本启发式：
- `press 22 start` 路径（原有）；
- `inputlog` 路径（2026-11-18 新增：门过渡机制场景直接用录像前 600 帧，作者在 f22/f26 按 Start，
  静态数 `presses` 会误 WARN）；
- 两路都没有 ⇒ 仍 WARN（真误删形态）。
"""
from __future__ import annotations

import io
import sys
from contextlib import redirect_stderr
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "tools"))
import scenario_run as sr  # noqa: E402


def _write(tmp_path: Path, name: str, body: str) -> Path:
    p = tmp_path / name
    p.write_text(body, encoding="utf-8")
    return p


def _warns(path: Path) -> bool:
    buf = io.StringIO()
    with redirect_stderr(buf):
        sr.parse_scenario(path)
    return "长场景前 700 帧无 start press" in buf.getvalue()


def test_press_start_satisfies_lint(tmp_path):
    p = _write(tmp_path, "c.txt", "name c\nframes 600\npress 22 start\ncapture 100 a\n")
    assert _warns(p) is False


def test_inputlog_start_satisfies_lint(tmp_path):
    # 合成 inputlog：每行一帧（隐式 1-based 帧号，3 列 `p1 p2 cmd`，见
    # tools/movie_import.py::read_inputlog），只有第 22 帧按 Start（bit4）。
    log = tmp_path / "mini.inputlog"
    lines = ["00 00 00"] * 22
    lines[21] = "10 00 00"
    log.write_text("\n".join(lines) + "\n", encoding="utf-8")
    p = _write(tmp_path, "c.txt", f"name c\nframes 600\ninputlog {log}\ncapture 100 a\n")
    assert _warns(p) is False


def test_no_input_still_warns(tmp_path):
    p = _write(tmp_path, "c.txt", "name c\nframes 600\ncapture 100 a\n")
    assert _warns(p) is True
