"""emu 腿缓存（矩阵 18 分钟 → 3.9 秒的关键件）的判据测试。

缓存必须同时满足两条才放行：①产物锚（用例/ROM/展开调度）匹配 ②产物文件确实在。
缺任一条 ⇒ 重跑 Mesen。少第一条=拿旧输入当新结论（假绿），少第二条=diff 读不到图（假 blocked）。
"""

import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

import l4_diff_run as l4  # noqa: E402


def _write_prov(side: str, name: str, anchors: dict) -> None:
    p = l4.prov_path(side, name)
    p.parent.mkdir(parents=True, exist_ok=True)
    p.write_text(json.dumps(anchors), encoding="utf-8")


def _touch_emu_artifacts(name: str, caps: list) -> None:
    l4.SHOTS_EMU.mkdir(parents=True, exist_ok=True)
    traces = l4.ROOT / "analysis" / "traces"
    traces.mkdir(parents=True, exist_ok=True)
    for f, tag in caps:
        (l4.SHOTS_EMU / f"{name}_f{f:05d}_{tag}.png").write_bytes(b"png")
    (traces / f"{name}.frames.jsonl").write_text("", encoding="utf-8")


ANCHORS = {"scenario_sha256": "a" * 64, "rom_sha256": "b" * 64, "sched_sha256": "c" * 64}
CAPS = [(100, "a"), (200, "b")]


def test_cache_hit_requires_anchor_match():
    name = "case-cache-anchor"
    _touch_emu_artifacts(name, CAPS)
    _write_prov("emu", name, {"scenario_sha256": "0" * 64, "rom_sha256": "b" * 64,
                              "sched_sha256": "c" * 64})
    assert l4.emu_cache_hit(name, ANCHORS, CAPS) is False


def test_cache_hit_requires_artifacts_present():
    name = "case-cache-files"
    _touch_emu_artifacts(name, CAPS)
    _write_prov("emu", name, ANCHORS)
    assert l4.emu_cache_hit(name, ANCHORS, CAPS) is True
    (l4.SHOTS_EMU / f"{name}_f00100_a.png").unlink()
    assert l4.emu_cache_hit(name, ANCHORS, CAPS) is False


def test_cache_miss_without_provenance():
    name = "case-cache-noprov"
    _touch_emu_artifacts(name, CAPS)
    l4.prov_path("emu", name).unlink(missing_ok=True)
    assert l4.emu_cache_hit(name, ANCHORS, CAPS) is False
