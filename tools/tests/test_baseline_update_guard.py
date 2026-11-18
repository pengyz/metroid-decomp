"""L4 基线更新器的「临时用例」防线（2026-11-18 事故回归）。

事故：单案 `l4_diff_run.py run <case>` 会覆盖 `shots_rust/matrix.json`；若此刻跑
`tools/baseline_update.py l4`，临时调试 case 会被锚进 ratchet，之后只能靠语义重基清洗
（实测 case-tmp-anim 混入基线）。防线判据=矩阵每个 case 必须有同名 `cases/<case>.txt`。
"""
import importlib.util
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]


def _load():
    spec = importlib.util.spec_from_file_location("baseline_update_guard", ROOT / "tools" / "baseline_update.py")
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


def _matrix(rows, baseline: Path) -> Path:
    doc = {
        "generated_by": "tools/l4_diff_run.py run-all",
        "rom_sha256": "e6e6b7014685adae447ebb3833242815747bc1e5df83ade79f693fb67cf565b6",
        "match": len(rows), "diff": 0, "blocked": 0,
        "cells_total": len(rows), "cases_total": len(rows),
        "rows": rows,
    }
    out = baseline.with_suffix(".matrix.json")
    out.write_text(json.dumps(doc), encoding="utf-8")
    return out


def _row(case: str) -> dict:
    return {"case": case, "case_file": f"cases/{case}.txt", "states": {"f100:cap": "match"}}


def test_rejects_case_without_same_named_case_file(tmp_path):
    bu = _load()
    m = _matrix([_row("case-001-game-start-samus-interaction"), _row("case-tmp-anim")], tmp_path / "b")
    try:
        bu.l4_update(m, tmp_path / "b.json", "临时用例不应入 ratchet")
    except ValueError as exc:
        assert "case-tmp-anim" in str(exc)
    else:  # pragma: no cover - 防线失效才会走到
        raise AssertionError("临时用例未被拒绝")


def test_accepts_all_known_cases(tmp_path):
    bu = _load()
    known = sorted(p.stem for p in (ROOT / "cases").glob("case-*.txt"))
    assert known, "cases/ 下应至少有一个 case-*.txt"
    m = _matrix([_row(c) for c in known], tmp_path / "b")
    rec = bu.l4_update(m, tmp_path / "b.json", "全部用例都有同名 case 文件")
    assert rec["match"] == len(known)
