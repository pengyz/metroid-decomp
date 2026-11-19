"""flaky 判定：blocked 不得作为"判定分歧"证据（分母静默缩水的根因）。

2026-11-19 事故：case-mechanism-input-conflict 在锚不符期间被判 4 次 blocked、其后 1 次
正常 ⇒ 同码态 verdicts=[blocked×4, match] 被判 flaky ⇒ 该案 11 格移出分母
（cells_total 160→149）。blocked 是基础设施失败，不是"同输入两答"，必须排除。
"""
import importlib.util
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]


def _load():
    spec = importlib.util.spec_from_file_location("l4", ROOT / "tools" / "l4_diff_run.py")
    mod = importlib.util.module_from_spec(spec)
    sys.modules["l4"] = mod
    spec.loader.exec_module(mod)
    return mod


l4 = _load()
CS = "0123456789ab+deadbeef"


def obs(verdicts, case="c", frame=100, tag="t", cs=CS):
    return [{"case": case, "frame": frame, "tag": tag, "verdict": v,
             "run_id": f"r{i}", "code_state": cs} for i, v in enumerate(verdicts)]


class BlockedIsNotFlakyEvidence(unittest.TestCase):
    def test_blocked_plus_match_is_not_flaky(self):
        got = l4.classify_flaky(obs(["blocked", "blocked", "blocked", "blocked", "match"]))
        self.assertEqual(got, {}, "blocked 被当成分歧证据 ⇒ 整案移出分母")

    def test_blocked_plus_diff_is_not_flaky(self):
        got = l4.classify_flaky(obs(["blocked", "diff", "blocked", "diff"]))
        self.assertEqual(got, {})

    def test_real_disagreement_is_still_flaky(self):
        got = l4.classify_flaky(obs(["match", "diff", "match"]))
        self.assertIn(("c", 100, "t"), got, "真分歧（match/diff 交替）必须仍判 flaky")
        self.assertEqual(got[("c", 100, "t")]["n"], 3)

    def test_single_verdict_repeated_is_not_flaky(self):
        self.assertEqual(l4.classify_flaky(obs(["match", "match", "match"])), {})

    def test_below_min_runs_is_not_flaky(self):
        self.assertEqual(l4.classify_flaky(obs(["match", "diff"])), {})


if __name__ == "__main__":
    unittest.main()
