#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""tools/tests/test_l4_matrix.py —— Task 5 批量对拍矩阵（run-all）测试（TDD 先红后绿）。

覆盖（task-5-brief Step 1 + 验收①—④）：
1. `run-all` 遍历 cases/*.txt **全用例** → 用例×采集点矩阵（`shots_rust/matrix.json`
   （结构化）+ `shots_rust/matrix.md`（人读表））（验收①④）；
2. 矩阵三态 `match | diff | blocked`；**blocked 计入分母、永不计 pass**（防假绿硬
   约束）——分母=用例×采集点总数（含 blocked）（验收②：造含 blocked 用例验证分母）；
3. 矩阵**每行三 sha 锚**：`scenario_sha256` + `rom_sha256` + 产物 sha（`artifacts`，
   键=`shots_rust/<f>` | `shots/<f>`——复用 Task 3 键名契约）（验收③）；
4. snapshot 用例从快照起跑（`snapshot <prefix>`≡bundle_load；`snapshot none`=上电）；
   每格标明复用/新跑（`provenance`，诚实——不静默混用）。

matrix.json 契约（本测试钉死的 schema）：
    {"generated_by","rom_sha256","cases_total","cells_total","denominator",
     "denominator_note","match","diff","blocked","pass","pass_rate","verdict","rows"}
    rows[i]  = {"case","case_file","scenario_sha256","rom_sha256","sched_sha256",
                "artifacts","snapshot","report","legs","error","cells"}
    cells[j] = {"frame","tag","state","provenance","file","diff_pixels",
                "total_pixels","reason"}     # state ∈ {match,diff,blocked} 三态硬约束
    legs/provenance = {"rust": "fresh"|"reused"|"none", "emu": 同}
    verdict = "pass" 当且仅当 cells_total>0 且 pass==cells_total（blocked 永不计 pass）
"""
from __future__ import annotations

import hashlib
import json
import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parent.parent.parent
sys.path.insert(0, str(ROOT / "tools"))

import l4_diff_run as l4  # noqa: E402

# 三态硬约束：矩阵格只允许这三态（error/未知一并归 blocked——永不计 pass）
CELL_STATES = {"match", "diff", "blocked"}


def sha256_file(p: Path) -> str:
    return hashlib.sha256(p.read_bytes()).hexdigest()


def make_png(path: Path, color=(10, 20, 30)):
    from PIL import Image
    Image.new("RGB", (8, 8), color).save(path)


def write_prov_file(base: Path, side: str, name: str,
                    sched_sha256: str, scenario_sha256: str, rom_sha256: str):
    """按 Task 3 契约写复用产物 provenance（测试=契约钉死者）。"""
    (base / f"{name}.l4prov.json").write_text(
        json.dumps({"side": side, "name": name,
                    "scenario_sha256": scenario_sha256,
                    "rom_sha256": rom_sha256,
                    "sched_sha256": sched_sha256}, indent=2) + "\n",
        encoding="utf-8")


def seed_products(out_dir: Path, name: str, produced, blocked=(), frames: int = 20):
    """造 Rust manifest + 截图（produced=采集点列表；blocked=(f,tag,reason)）。"""
    man = {"name": name, "frames": frames, "gap": bool(blocked),
           "captures": [{"frame": f, "tag": t,
                         "file": f"{name}_f{f:05}_{t}.png", "status": "produced"}
                        for f, t in produced],
           "blocked": [{"frame": f, "tag": t, "reason": r} for f, t, r in blocked]}
    (out_dir / f"{name}.rust.captures.json").write_text(
        json.dumps(man, ensure_ascii=False), encoding="utf-8")
    for f, t in produced:
        make_png(out_dir / f"{name}_f{f:05}_{t}.png")


# ---------------- ② blocked 计入分母且永不计 pass（验收②核心） ----------------

class TestBlockedCountsInDenominator:
    def test_blocked_cell_in_denominator_but_not_pass(self):
        """blocked 格计入分母（用例×采集点总数）且**永不计 pass**。"""
        caps = [(5, "mid"), (7, "gap"), (10, "end")]
        report = {"name": "case-c", "results": [
            {"frame": 5, "tag": "mid", "status": "match", "match": True,
             "file": "case-c_f00005_mid.png", "diff_pixels": 0, "total_pixels": 64},
            {"frame": 7, "tag": "gap", "status": "blocked", "match": False,
             "reason": "unported gap @ f6: bank3:$C4DE"},
            {"frame": 10, "tag": "end", "status": "diff", "match": False,
             "file": "case-c_f00010_end.png", "diff_pixels": 3, "total_pixels": 64},
        ], "artifacts": {}}
        row = l4.matrix_row(
            "case-c", "cases/case-c.txt", caps,
            anchors={"scenario_sha256": "s" * 64, "rom_sha256": "r" * 64,
                     "sched_sha256": "h" * 64},
            legs={"rust": "fresh", "emu": "fresh"}, report=report)
        m = l4.build_matrix([row])
        assert m["cells_total"] == 3, "分母=用例×采集点总数（含 blocked 格）"
        assert m["denominator"] == 3
        assert m["blocked"] == 1 and m["match"] == 1 and m["diff"] == 1
        assert m["pass"] == 1, "blocked 永不计 pass（pass=match 数）"
        assert m["pass"] + m["diff"] + m["blocked"] == m["cells_total"]
        assert m["pass_rate"] == "1/3"
        st = {c["tag"]: c["state"] for c in row["cells"]}
        assert st == {"mid": "match", "gap": "blocked", "end": "diff"}

    def test_all_blocked_never_green(self):
        """全 blocked：pass=0、verdict≠pass（不许假绿）；分母仍=格数。"""
        caps = [(5, "a"), (9, "b")]
        report = {"name": "case-c", "results": [
            {"frame": 5, "tag": "a", "status": "blocked", "match": False,
             "reason": "unported gap @ f4: bank1:$8000"},
            {"frame": 9, "tag": "b", "status": "blocked", "match": False,
             "reason": "unported gap @ f4: bank1:$8000"},
        ], "artifacts": {}}
        row = l4.matrix_row(
            "case-c", "cases/case-c.txt", caps,
            anchors={"scenario_sha256": "s", "rom_sha256": "r", "sched_sha256": "h"},
            legs={"rust": "fresh", "emu": "fresh"}, report=report)
        m = l4.build_matrix([row])
        assert m["denominator"] == 2 and m["blocked"] == 2
        assert m["pass"] == 0, "blocked 计入分母但不计 pass——假绿防线"
        assert m["verdict"] != "pass"

    def test_missing_result_cell_blocked_denominator_shrinks_nothing(self):
        """DSL 采集点无对拍结果（产物缺）→ 格 blocked（分母不缩水、不计 pass）。"""
        caps = [(5, "mid"), (10, "end")]
        report = {"name": "case-c", "results": [
            {"frame": 5, "tag": "mid", "status": "match", "match": True,
             "file": "case-c_f00005_mid.png", "diff_pixels": 0, "total_pixels": 64},
        ], "artifacts": {}}
        row = l4.matrix_row(
            "case-c", "cases/case-c.txt", caps,
            anchors={"scenario_sha256": "s", "rom_sha256": "r", "sched_sha256": "h"},
            legs={"rust": "fresh", "emu": "fresh"}, report=report)
        m = l4.build_matrix([row])
        assert m["denominator"] == 2, "分母以 DSL 采集点为基准——产物缺不缩分母"
        assert m["pass"] == 1 and m["blocked"] == 1
        assert row["cells"][1]["state"] == "blocked"
        assert "无对拍结果" in row["cells"][1]["reason"]


# ---------------- 三态硬约束（match | diff | blocked） ----------------

class TestMatrixThreeStates:
    def test_error_result_maps_to_blocked_with_reason(self):
        """l4report status=error（缺截图）→ 归 blocked（三态硬约束）+ reason 保留。"""
        caps = [(5, "mid")]
        report = {"name": "case-c", "results": [
            {"frame": 5, "tag": "mid", "status": "error", "match": False,
             "reason": "缺 Rust 截图"},
        ], "artifacts": {}}
        row = l4.matrix_row(
            "case-c", "cases/case-c.txt", caps,
            anchors={"scenario_sha256": "s", "rom_sha256": "r", "sched_sha256": "h"},
            legs={"rust": "fresh", "emu": "fresh"}, report=report)
        assert row["cells"][0]["state"] == "blocked"
        assert "缺 Rust 截图" in row["cells"][0]["reason"]
        assert l4.build_matrix([row])["pass"] == 0

    def test_states_are_exactly_three(self):
        """所有格 state ∈ {match,diff,blocked}（三态硬约束——无第四态漏网）。"""
        caps = [(1, "a"), (2, "b"), (3, "c"), (4, "d")]
        report = {"name": "case-c", "results": [
            {"frame": 1, "tag": "a", "status": "match", "match": True,
             "file": "x", "diff_pixels": 0, "total_pixels": 1},
            {"frame": 2, "tag": "b", "status": "diff", "match": False,
             "file": "y", "diff_pixels": 1, "total_pixels": 1},
            {"frame": 3, "tag": "c", "status": "blocked", "match": False,
             "reason": "gap"},
            {"frame": 4, "tag": "d", "status": "error", "match": False,
             "reason": "缺模拟器截图"},
        ], "artifacts": {}}
        row = l4.matrix_row(
            "case-c", "cases/case-c.txt", caps,
            anchors={"scenario_sha256": "s", "rom_sha256": "r", "sched_sha256": "h"},
            legs={"rust": "fresh", "emu": "fresh"}, report=report)
        assert {c["state"] for c in row["cells"]} <= CELL_STATES

    def test_case_level_error_blocks_all_cells(self):
        """用例级失败（腿失败/复用拒跑）→ 全格 blocked（记原因）——不静默跳过、不缩分母。"""
        caps = [(5, "mid"), (10, "end")]
        row = l4.matrix_row(
            "case-c", "cases/case-c.txt", caps,
            anchors={"scenario_sha256": "s", "rom_sha256": "r", "sched_sha256": "h"},
            legs={"rust": "none", "emu": "none"}, report=None,
            error="模拟器运行失败，且回落复用未过锚校验——emu 侧复用校验失败")
        m = l4.build_matrix([row])
        assert m["denominator"] == 2 and m["pass"] == 0
        assert all(c["state"] == "blocked" for c in row["cells"])
        assert all("回落复用未过锚校验" in c["reason"] for c in row["cells"])
        assert row["error"]


# ---------------- ③ 三 sha 锚落矩阵 ----------------

class TestMatrixThreeShaAnchors:
    def test_row_carries_scenario_rom_artifact_anchors(self):
        """矩阵每行锚 scenario_sha256 + rom_sha256 + 产物 sha（Task 3 键名契约）。"""
        caps = [(5, "mid")]
        report = {"name": "case-c", "results": [
            {"frame": 5, "tag": "mid", "status": "match", "match": True,
             "file": "case-c_f00005_mid.png", "diff_pixels": 0, "total_pixels": 64},
        ], "artifacts": {"shots_rust/case-c_f00005_mid.png": "a" * 64,
                         "shots/case-c_f00005_mid.png": "b" * 64}}
        row = l4.matrix_row(
            "case-c", "cases/case-c.txt", caps,
            anchors={"scenario_sha256": "s" * 64, "rom_sha256": "r" * 64,
                     "sched_sha256": "h" * 64},
            legs={"rust": "fresh", "emu": "fresh"}, report=report)
        m = l4.build_matrix([row])
        assert row["scenario_sha256"] == "s" * 64
        assert row["rom_sha256"] == "r" * 64
        assert row["artifacts"] == {"shots_rust/case-c_f00005_mid.png": "a" * 64,
                                    "shots/case-c_f00005_mid.png": "b" * 64}
        assert m["rom_sha256"] == "r" * 64


# ---------------- run-all CLI（验收①④） ----------------

CASE_A = """\
name case-a
frames 20
snapshot none
capture 5 mid
capture 10 end
"""

CASE_B = """\
name case-b
frames 24
snapshot bundle-x
capture 6 snap-mid
capture 12 snap-end
"""


@pytest.fixture()
def workdir(tmp_path, monkeypatch):
    """隔离工作区：假 ROM + 独立 cases/（run-all 遍历面）+ 双侧产物目录。"""
    shots_rust = tmp_path / "shots_rust"
    shots_emu = tmp_path / "shots"
    shots_rust.mkdir()
    shots_emu.mkdir()
    cases = tmp_path / "cases"
    cases.mkdir()
    rom = tmp_path / "rom.nes"
    rom.write_bytes(b"FAKE-ROM-BYTES")
    monkeypatch.setattr(l4, "SHOTS_RUST", shots_rust)
    monkeypatch.setattr(l4, "SHOTS_EMU", shots_emu)
    monkeypatch.setattr(l4, "CASES_DIR", cases)
    monkeypatch.setattr(l4, "ROM", rom, raising=False)
    return {"tmp": tmp_path, "cases": cases, "rom": rom,
            "shots_rust": shots_rust, "shots_emu": shots_emu}


def sched_sha_for(name: str, frames: int, joy: dict, caps, snap, tmp: Path) -> str:
    """契约=sched 文件字节 sha256（write_schedule 落盘后取 sha——与实现同径）。"""
    p = tmp / f"{name}.anchor.sched"
    l4.write_schedule(name, frames, joy, caps, p, snapshot=snap)
    return sha256_file(p)


def seed_case(w: dict, name: str, case_text: str, produced, blocked=()):
    """种用例 + 双侧产物 + 双侧锚存证（合法复用前提）。"""
    import scenario_run as sr
    case = w["cases"] / f"{name}.txt"
    case.write_text(case_text, encoding="utf-8")
    plan = sr.parse_scenario(case)
    caps = sorted((int(c["frame"]), c["tag"]) for c in plan.get("captures", []))
    joy = sr.expand_schedule(plan)
    snap = (plan.get("bundle_load") or {}).get("path")
    sched_sha = sched_sha_for(name, int(plan["frames"]), joy, caps, snap, w["tmp"])
    scenario_sha = sha256_file(case)
    rom_sha = sha256_file(w["rom"])
    seed_products(w["shots_rust"], name, produced, blocked, frames=int(plan["frames"]))
    for f, t in produced:  # emu 侧同名截图（同色=match）
        make_png(w["shots_emu"] / f"{name}_f{f:05}_{t}.png")
    write_prov_file(w["shots_rust"], "rust", name, sched_sha, scenario_sha, rom_sha)
    write_prov_file(w["shots_emu"], "emu", name, sched_sha, scenario_sha, rom_sha)
    return {"name": name, "caps": caps, "snap": snap, "sched_sha": sched_sha,
            "scenario_sha": scenario_sha, "rom_sha": rom_sha}


def run_cli(*flags: str) -> tuple[int, str]:
    """跑 CLI main()，返回 (exit code, stderr 文本)（main 以 SystemExit 传出）。

    stderr 文本供 fail-loud 断言**按原因**验证（防 exit 2 巧合假红/假绿）。
    """
    import io
    argv = ["l4_diff_run.py", "run-all", *flags]
    monkey = pytest.MonkeyPatch()
    err = io.StringIO()
    monkey.setattr(sys, "argv", argv)
    monkey.setattr(sys, "stderr", err)
    try:
        with pytest.raises(SystemExit) as exc:
            l4.main()
        return int(exc.value.code or 0), err.getvalue()
    finally:
        monkey.undo()


class TestRunAllTraversal:
    def test_run_all_traverses_cases_produces_matrix_and_md(self, workdir):
        """验收①④：run-all 遍历 cases/ 全用例出用例×采集点矩阵 + 人读表。"""
        seed_case(workdir, "case-a", CASE_A, produced=[(5, "mid"), (10, "end")])
        seed_case(workdir, "case-b", CASE_B,
                  produced=[(6, "snap-mid"), (12, "snap-end")])
        rc, _err = run_cli("--skip-rust", "--skip-emu")
        assert rc == 0, "全 match 应 exit 0"
        m = json.loads((workdir["shots_rust"] / "matrix.json").read_text(encoding="utf-8"))
        assert m["cases_total"] == 2
        assert m["denominator"] == 4, "分母=用例×采集点总数"
        assert {r["case"] for r in m["rows"]} == {"case-a", "case-b"}
        for r in m["rows"]:
            assert [c["tag"] for c in r["cells"]] == \
                   {"case-a": ["mid", "end"], "case-b": ["snap-mid", "snap-end"]}[r["case"]]
            assert all(c["state"] == "match" for c in r["cells"])
        assert m["pass"] == 4 and m["verdict"] == "pass"
        md = (workdir["shots_rust"] / "matrix.md").read_text(encoding="utf-8")
        assert "matrix" in md.lower() and "分母" in md
        assert "case-a" in md and "snap-mid" in md

    def test_run_all_blocked_case_denominator(self, workdir):
        """验收②（CLI 面）：造含 blocked 用例——blocked 计入分母且不计 pass。"""
        seed_case(workdir, "case-a", CASE_A,
                  produced=[(5, "mid")],
                  blocked=[(10, "end", "unported gap @ f8: bank3:$C4DE")])
        rc, _err = run_cli("--skip-rust", "--skip-emu")
        assert rc == 1, "含 blocked 不得假绿 exit 0"
        m = json.loads((workdir["shots_rust"] / "matrix.json").read_text(encoding="utf-8"))
        assert m["denominator"] == 2, "blocked 格计入分母"
        assert m["match"] == 1 and m["blocked"] == 1
        assert m["pass"] == 1, "blocked 永不计 pass"
        gap = next(c for c in m["rows"][0]["cells"] if c["tag"] == "end")
        assert gap["state"] == "blocked"
        assert "unported gap" in gap["reason"]

    def test_run_all_snapshot_case_recorded(self, workdir):
        """snapshot 用例从快照起跑：行 snapshot=<前缀>；锚 sched_sha256 覆盖 snapshot 行。"""
        info = seed_case(workdir, "case-b", CASE_B,
                         produced=[(6, "snap-mid"), (12, "snap-end")])
        rc, _err = run_cli("--skip-rust", "--skip-emu")
        assert rc == 0
        m = json.loads((workdir["shots_rust"] / "matrix.json").read_text(encoding="utf-8"))
        row = m["rows"][0]
        assert row["snapshot"] == "bundle-x", "snapshot <prefix>≠none（快照起跑语义）"
        assert row["sched_sha256"] == info["sched_sha"]
        sched = (workdir["shots_rust"] / "case-b.sched").read_text(encoding="utf-8")
        assert "snapshot bundle-x" in sched, "调度表落 snapshot 行（Task 4 消费面）"

    def test_run_all_snapshot_none_is_power_on(self, workdir):
        info = seed_case(workdir, "case-a", CASE_A, produced=[(5, "mid"), (10, "end")])
        rc, _err = run_cli("--skip-rust", "--skip-emu")
        assert rc == 0
        m = json.loads((workdir["shots_rust"] / "matrix.json").read_text(encoding="utf-8"))
        assert m["rows"][0]["snapshot"] is None, "snapshot none=上电起跑"
        assert m["rows"][0]["sched_sha256"] == info["sched_sha"]


class TestRunAllProvenance:
    def test_reuse_marks_reused_fresh_marks_fresh(self, workdir):
        """诚实标注：--skip-* 复用格=reused；真跑格=fresh（不静默混用）。"""
        seed_case(workdir, "case-a", CASE_A, produced=[(5, "mid"), (10, "end")])
        rc, _err = run_cli("--skip-rust", "--skip-emu")
        assert rc == 0
        m = json.loads((workdir["shots_rust"] / "matrix.json").read_text(encoding="utf-8"))
        assert m["rows"][0]["legs"] == {"rust": "reused", "emu": "reused"}
        assert all(c["provenance"] == {"rust": "reused", "emu": "reused"}
                   for c in m["rows"][0]["cells"])

        # 真跑面：打桩双腿（不真跑 cargo/Mesen）→ fresh 标注
        import scenario_run as sr

        def fake_run_rust(name, sched, out_dir, state_out=None, bundle=None):
            caps = []
            for ln in Path(sched).read_text(encoding="utf-8").splitlines():
                toks = ln.split()
                if toks and toks[0] == "capture":
                    caps.append((int(toks[1]), toks[2]))
            seed_products(out_dir, name, caps)
            return True

        def fake_run_emu(name, timeout, scen_path):
            plan = sr.parse_scenario(Path(scen_path))
            for c in plan.get("captures", []):
                make_png(l4.SHOTS_EMU / f"{name}_f{int(c['frame']):05}_{c['tag']}.png")
            return True

        monkey = pytest.MonkeyPatch()
        monkey.setattr(l4, "run_rust", fake_run_rust)
        monkey.setattr(l4, "run_emu", fake_run_emu)
        try:
            rc, _err = run_cli()
            assert rc == 0
            m = json.loads((workdir["shots_rust"] / "matrix.json").read_text(encoding="utf-8"))
            assert m["rows"][0]["legs"] == {"rust": "fresh", "emu": "fresh"}
            assert all(c["provenance"] == {"rust": "fresh", "emu": "fresh"}
                       for c in m["rows"][0]["cells"])
        finally:
            monkey.undo()

    def test_reuse_refused_marks_blocked_not_silent_reuse(self, workdir):
        """锚不符复用被拒 → 格 blocked（记拒跑原因）——绝不静默复用旧产物。"""
        info = seed_case(workdir, "case-a", CASE_A, produced=[(5, "mid"), (10, "end")])
        write_prov_file(workdir["shots_rust"], "rust", "case-a",
                        "0" * 64, info["scenario_sha"], info["rom_sha"])  # 旧调度产物
        rc, _err = run_cli("--skip-rust", "--skip-emu")
        assert rc == 1
        m = json.loads((workdir["shots_rust"] / "matrix.json").read_text(encoding="utf-8"))
        row = m["rows"][0]
        assert row["error"] and "skip-rust" in row["error"]
        assert all(c["state"] == "blocked" for c in row["cells"])
        assert m["pass"] == 0 and m["verdict"] != "pass"


class TestRunAllFailLoud:
    def test_empty_cases_dir_refused(self, workdir):
        """cases/ 无用例=空矩阵不得绿——fail-loud 拒跑（exit 2，按原因断言）。"""
        rc, err = run_cli("--skip-rust", "--skip-emu")
        assert rc == 2
        assert "无对拍用例" in err, f"fail-loud 须指明原因（防空矩阵假绿）：{err!r}"

    def test_duplicate_case_names_refused(self, workdir):
        """同名用例产物/报告互相覆盖=矩阵失真——fail-loud 拒跑。"""
        seed_case(workdir, "case-a", CASE_A, produced=[(5, "mid"), (10, "end")])
        (workdir["cases"] / "case-a-again.txt").write_text(CASE_A, encoding="utf-8")
        rc, err = run_cli("--skip-rust", "--skip-emu")
        assert rc == 2
        assert "重复" in err and "case-a" in err, f"须指明同名冲突：{err!r}"

    def test_case_without_captures_refused(self, workdir):
        """用例无采集点=分母静默缩水风险——fail-loud 拒跑。"""
        (workdir["cases"] / "case-a.txt").write_text(
            "name case-a\nframes 20\nsnapshot none\n", encoding="utf-8")
        rc, err = run_cli("--skip-rust", "--skip-emu")
        assert rc == 2
        assert "无采集点" in err, f"须指明采集点缺失（分母缩水风险）：{err!r}"
