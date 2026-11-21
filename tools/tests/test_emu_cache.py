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


def test_cache_rejects_stale_trace_older_than_case(tmp_path):
    """新鲜度闸：用例比 trace 新 ⇒ 拒复用。

    病根（2026-11-21 实测）：观察面 `watch_range` 写在用例文本里，而锚只看输入；
    改观察面后一旦锚恰好相等，就会拿到"没有新观察键"的旧 trace（`w:{}` 空字典），
    表现为"改了观察面却看不到数据"。mtime 闸把这条堵死。
    """
    name = "case-cache-stale"
    _touch_emu_artifacts(name, CAPS)
    _write_prov("emu", name, dict(ANCHORS))
    scen = tmp_path / "case-cache-stale.txt"
    scen.write_text("frames 10\n", encoding="utf-8")
    trace = l4.ROOT / "analysis" / "traces" / f"{name}.frames.jsonl"
    import os
    import time

    os.utime(trace, (time.time() - 100, time.time() - 100))
    assert l4.emu_cache_hit(name, ANCHORS, CAPS, scen_path=scen) is False
    # 用例比 trace 旧 ⇒ 放行（正常缓存命中路径不受影响）
    os.utime(scen, (time.time() - 200, time.time() - 200))
    assert l4.emu_cache_hit(name, ANCHORS, CAPS, scen_path=scen) is True
