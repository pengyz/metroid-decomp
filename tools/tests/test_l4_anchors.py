#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""tools/tests/test_l4_anchors.py —— Task 3 三 sha 锚定 + 复用校验 测试（TDD 先红后绿）。

覆盖（task-3-brief Step 1）：
1. l4report.json 含三 sha 锚：`scenario_sha256`（场景 DSL sha）+ `rom_sha256`（ROM sha）
   + `artifacts`（各截图 sha，键=shots_rust/<f> | shots/<f>）；（T5 消费同键名）；
2. `--skip-rust`/`--skip-emu` 复用旧产物前校验调度 hash（sched_sha256）+scenario/rom 锚：
   不符 → **拒跑**（exit≠0，不静默复用）；
3. 无锚产物（缺 provenance 文件）→ 拒跑（无锚可对=不能证明复用合法）。

复用产物 provenance 契约（本测试钉死的文件 schema）：
    <shots_rust|shots>/<name>.l4prov.json
    = {"side","name","scenario_sha256","rom_sha256","sched_sha256"}
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

FIXTURE_CASE = """\
# fixture：最小对拍用例（三要素格式）
name fixture-case
frames 10
snapshot none
capture 5 mid
capture 10 end
"""

CAPTURES = [(5, "mid", "fixture-case_f00005_mid.png"),
            (10, "end", "fixture-case_f00010_end.png")]


def sha256_file(p: Path) -> str:
    return hashlib.sha256(p.read_bytes()).hexdigest()


def make_png(path: Path, color=(10, 20, 30)):
    from PIL import Image
    Image.new("RGB", (8, 8), color).save(path)


def write_prov_file(base: Path, side: str, name: str,
                    sched_sha256: str, scenario_sha256: str, rom_sha256: str):
    """按契约写复用产物 provenance（测试=契约钉死者）。"""
    (base / f"{name}.l4prov.json").write_text(
        json.dumps({"side": side, "name": name,
                    "scenario_sha256": scenario_sha256,
                    "rom_sha256": rom_sha256,
                    "sched_sha256": sched_sha256}, indent=2) + "\n",
        encoding="utf-8")


@pytest.fixture()
def workdir(tmp_path, monkeypatch):
    """隔离工作区：假 ROM + 假双侧截图 + 真调度（cases 权威解析链）。"""
    shots_rust = tmp_path / "shots_rust"
    shots_emu = tmp_path / "shots"
    shots_rust.mkdir()
    shots_emu.mkdir()
    rom = tmp_path / "rom.nes"
    rom.write_bytes(b"FAKE-ROM-BYTES")
    case = tmp_path / "fixture-case.txt"
    case.write_text(FIXTURE_CASE, encoding="utf-8")

    monkeypatch.setattr(l4, "SHOTS_RUST", shots_rust)
    monkeypatch.setattr(l4, "SHOTS_EMU", shots_emu)
    monkeypatch.setattr(l4, "ROM", rom, raising=False)  # 实现未落地前先钉契约（TDD 红）

    # 假 Rust manifest（do_diff 消费格式）
    man = {"name": "fixture-case", "frames": 10, "gap": False,
           "captures": [{"frame": f, "tag": t, "file": fn, "status": "produced"}
                        for f, t, fn in CAPTURES],
           "blocked": []}
    (shots_rust / "fixture-case.rust.captures.json").write_text(
        json.dumps(man, ensure_ascii=False), encoding="utf-8")
    # 双侧假截图（同色=全 match）
    for _f, _t, fn in CAPTURES:
        make_png(shots_rust / fn)
        make_png(shots_emu / fn)

    # 当前调度 hash（契约=sched 文件字节 sha256；实现与测试同径：write_schedule 落盘后取 sha）
    import scenario_run as sr
    plan = sr.parse_scenario(case)
    joy = sr.expand_schedule(plan)
    caps = sorted((int(c["frame"]), c["tag"]) for c in plan.get("captures", []))
    sched_file = tmp_path / "current.sched"
    l4.write_schedule("fixture-case", int(plan["frames"]), joy, caps, sched_file)
    sched_sha = sha256_file(sched_file)
    return {
        "tmp": tmp_path, "case": case, "rom": rom,
        "shots_rust": shots_rust, "shots_emu": shots_emu,
        "sched_sha": sched_sha,
        "scenario_sha": sha256_file(case),
        "rom_sha": sha256_file(rom),
    }


def run_cli(case: Path, *flags: str) -> int:
    """跑 CLI main()，返回 exit code（main 以 SystemExit 传出）。"""
    argv = ["l4_diff_run.py", "run", str(case), *flags]
    monkey = pytest.MonkeyPatch()
    monkey.setattr(sys, "argv", argv)
    try:
        with pytest.raises(SystemExit) as exc:
            l4.main()
        return int(exc.value.code or 0)
    finally:
        monkey.undo()


def seed_prov(w: dict, *, sched_sha: str | None = None, side_rust: bool = True,
              side_emu: bool = True):
    s = sched_sha if sched_sha is not None else w["sched_sha"]
    if side_rust:
        write_prov_file(w["shots_rust"], "rust", "fixture-case",
                        s, w["scenario_sha"], w["rom_sha"])
    if side_emu:
        write_prov_file(w["shots_emu"], "emu", "fixture-case",
                        s, w["scenario_sha"], w["rom_sha"])


# ---------------- ① l4report 三 sha 锚 ----------------
def test_l4report_contains_three_sha_anchors(workdir):
    """l4report.json 必须含 scenario_sha256 + rom_sha256 + artifacts（各截图 sha）。"""
    seed_prov(workdir)
    rc = run_cli(workdir["case"], "--skip-rust", "--skip-emu")
    assert rc == 0, "合法锚+全 match 应 exit 0"
    rep = json.loads((workdir["shots_rust"] / "fixture-case.l4report.json")
                     .read_text(encoding="utf-8"))
    assert rep["scenario_sha256"] == workdir["scenario_sha"], "scenario_sha256=场景 DSL 文件 sha"
    assert rep["rom_sha256"] == workdir["rom_sha"], "rom_sha256=ROM 文件 sha"
    arts = rep["artifacts"]
    assert isinstance(arts, dict) and arts, "artifacts=各截图 sha 映射"
    for _f, _t, fn in CAPTURES:
        assert arts[f"shots_rust/{fn}"] == sha256_file(workdir["shots_rust"] / fn)
        assert arts[f"shots/{fn}"] == sha256_file(workdir["shots_emu"] / fn)


def test_l4report_rom_anchor_is_real_rom_sha(workdir):
    """rom_sha256 锚必须随 ROM 文件实测（换 ROM 字节=锚变）。"""
    seed_prov(workdir)
    run_cli(workdir["case"], "--skip-rust", "--skip-emu")
    rep = json.loads((workdir["shots_rust"] / "fixture-case.l4report.json")
                     .read_text(encoding="utf-8"))
    workdir["rom"].write_bytes(b"OTHER-ROM")
    assert rep["rom_sha256"] != sha256_file(workdir["rom"]), "锚=实测 ROM sha，非常量"


# ---------------- ② 复用校验：调度 hash 不符 → 拒跑 ----------------
def test_skip_rust_rejects_stale_schedule_hash(workdir):
    """--skip-rust 复用旧产物：产物 sched_sha256 ≠ 当前调度 hash → 拒跑（exit≠0）。"""
    seed_prov(workdir, sched_sha="0" * 64)  # 旧产物=别的调度生成
    rc = run_cli(workdir["case"], "--skip-rust", "--skip-emu")
    assert rc != 0, "调度 hash 不符必须拒跑（不静默复用旧产物出假绿）"
    assert not (workdir["shots_rust"] / "fixture-case.l4report.json").exists(), \
        "拒跑不得产出对拍报告"


def test_skip_emu_rejects_stale_schedule_hash(workdir):
    """--skip-emu 同理：emu 侧产物锚不符 → 拒跑。"""
    seed_prov(workdir, side_emu=False)
    write_prov_file(workdir["shots_emu"], "emu", "fixture-case",
                    "0" * 64, workdir["scenario_sha"], workdir["rom_sha"])
    rc = run_cli(workdir["case"], "--skip-rust", "--skip-emu")
    assert rc != 0, "emu 侧调度 hash 不符同样拒跑"


def test_reuse_without_provenance_refused(workdir):
    """无锚产物（pre-锚定时代产物）→ 无锚可对=拒跑（fail-closed）。"""
    seed_prov(workdir, side_rust=False, side_emu=False)
    rc = run_cli(workdir["case"], "--skip-rust", "--skip-emu")
    assert rc != 0, "缺 provenance 无法证明复用合法 → 拒跑"


def test_matching_anchor_reuse_accepted(workdir):
    """锚全符 → 复用放行（拒跑只挡伪锚，不挡合法复用）。"""
    seed_prov(workdir)
    rc = run_cli(workdir["case"], "--skip-rust", "--skip-emu")
    assert rc == 0
