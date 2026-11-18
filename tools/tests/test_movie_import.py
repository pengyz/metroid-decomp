"""tools/movie_import.py 的位序锚。

为什么需要：bk2 的列序由 `Input Log.txt` 的 `LogKey:` 行声明，实测 6725M 是
`UDLRTSBA`，而旧实现按 fm2 的 `RLDUTSBA` 硬解 ⇒ 四个方向键整体错位一格
（Up↔Right、Down↔Left），Start/Select/B/A 因位次巧合仍对，属静默错误。
本测试用合成 bk2（不含任何 TASVideos 内容）钉死"列名→位"的映射。
"""

from __future__ import annotations

import sys
import zipfile
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

import movie_import as M  # noqa: E402

LOGKEY_BIZHAWK = (
    "LogKey:#Reset|Power|#P1 Up|P1 Down|P1 Left|P1 Right|P1 Start|P1 Select|"
    "P1 B|P1 A|#P2 Up|P2 Down|P2 Left|P2 Right|P2 Start|P2 Select|P2 B|P2 A|"
)


def _make_bk2(path: Path, rows: list[str], logkey: str = LOGKEY_BIZHAWK) -> Path:
    header = "MovieVersion BizHawk v2.0\nCore quickerNES\nPlatform NES\nSHA1 ABC\n"
    log = "[Input]\n" + logkey + "\n" + "\n".join(rows) + "\n"
    with zipfile.ZipFile(path, "w") as z:
        z.writestr("Header.txt", header)
        z.writestr("Input Log.txt", log)
    return path


def test_bk2_direction_columns_use_logkey_not_position(tmp_path: Path):
    # P1 组 8 列按 LogKey 是 U D L R Start Select B A：
    # 第 3 列（index 2）按下 = Left = bit1；旧实现在这里会给 bit2（Down）。
    row = "|..|..L.....|........|"
    mv = M.parse_bk2(_make_bk2(tmp_path / "a.bk2", [row]))
    p1, p2, cmd = mv["frames"][0]
    assert p1 == 0b0000_0010, f"Left 必须是 bit1，实际 {p1:#04x}"
    assert p2 == 0
    assert cmd == 0


def test_bk2_all_directions_and_buttons(tmp_path: Path):
    # 逐键单项：U/D/L/R/Start/Select/B/A 在 LogKey 列序下的位值。
    cases = {
        "U": 3,
        "D": 2,
        "L": 1,
        "R": 0,
        "T": 4,  # Start
        "S": 5,  # Select
        "B": 6,
        "A": 7,
    }
    for ch, bit in cases.items():
        cells = ["."] * 8
        cells[list("UDLRTSBA").index(ch)] = "x"
        row = "|..|" + "".join(cells) + "|........|"
        mv = M.parse_bk2(_make_bk2(tmp_path / f"{ch}.bk2", [row]))
        p1 = mv["frames"][0][0]
        assert p1 == 1 << bit, f"{ch} 应为 bit{bit}，实际 {p1:#04x}"


def test_bk2_command_columns_by_name(tmp_path: Path):
    # 命令列按名字映射：第 0 列 = #Reset（软复位），第 1 列 = Power（硬复位）。
    mv = M.parse_bk2(_make_bk2(tmp_path / "c.bk2", ["|r.|........|........|"]))
    assert mv["frames"][0][2] == M.CMD_SOFT_RESET
    mv = M.parse_bk2(_make_bk2(tmp_path / "d.bk2", ["|.p|........|........|"]))
    assert mv["frames"][0][2] == M.CMD_HARD_RESET


def test_bk2_without_logkey_is_rejected(tmp_path: Path):
    p = _make_bk2(tmp_path / "e.bk2", ["|..|........|........|"], logkey="")
    with pytest.raises(M.MovieError, match="LogKey"):
        M.parse_bk2(p)


def test_fm2_order_is_rldutsba():
    # fm2 的八字符列序 = RLDUTSBA（列序==位序），此路径不得被 bk2 的改动带偏。
    assert M.bits_from_text("R.......") == 0b0000_0001
    assert M.bits_from_text(".L......") == 0b0000_0010
    assert M.bits_from_text(".......A") == 0b1000_0000
    assert M.bits_from_text("........") == 0
