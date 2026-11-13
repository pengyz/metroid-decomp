#!/usr/bin/env python3
"""全语义基线：把 L4 矩阵的**全部用例**接进 `.sem` 对拍，产出一张可比的表。

单个用例的语义对拍见 `tools/sem.py`（本工具只做批量编排与汇总，
判据一律转调 `sem.compare_sem`——**不在这里第二处实现判定**）。

## 为什么要有它

L4 像素矩阵只覆盖 14 个用例、112 个采集点；语义面**每帧 15 列**。把 14 个用例
全部接进来之后，「哪个用例在哪一帧开始语义分叉、分叉在哪个变量」才有全量基线——
这样后续任何修复（或任何新增拟合）都能立刻看出它动了哪些用例、哪个变量。

## 成本与纪律

- oracle 腿每案 ~55s（Mesen 单实例 ⇒ **必须串行**），rust 腿 ~15-20s，diff 毫秒级。
  14 案一次性 ~15 分钟；之后复用缓存（`--reuse`，默认），只跑 rust 腿。
- oracle 轨迹缓存在 `analysis/traces/<case>.sem.frames.jsonl`（gitignored）。
  缓存**新于**派生用例文本才复用——用例改了会自动重采，不会拿旧轨迹对新输入。
- `.sem` 内容不入库；入库的是本工具汇总出的基线表（见 docs/findings/）。

## 用法

    python3 tools/sem_baseline.py                 # 全部 L4 用例（串行，复用缓存）
    python3 tools/sem_baseline.py --cases a,b     # 指定
    python3 tools/sem_baseline.py --fresh         # 忽略缓存全部重采
    python3 tools/sem_baseline.py --json out.json # 另存机器可读汇总
"""

from __future__ import annotations

import json
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
WORK = ROOT / "analysis" / "sem"
TRACES = ROOT / "analysis" / "traces"

sys.path.insert(0, str(ROOT / "tools"))
import sem  # noqa: E402


def l4_cases() -> list[str]:
    """L4 矩阵里实际跑的用例（以 matrix.json 的 rows 为准，不猜 cases/ 目录）。"""
    mp = ROOT / "shots_rust" / "matrix.json"
    if not mp.exists():
        raise SystemExit(f"[base] FAIL 缺矩阵 {mp}（先跑一次 l4_diff_run.py run-all）")
    rows = json.load(mp.open(encoding="utf-8")).get("rows") or []
    return sorted({r["case"] for r in rows if r.get("case")})


def run(cmd: list[str], label: str) -> float:
    t0 = time.time()
    r = subprocess.run(cmd, cwd=ROOT)
    dt = time.time() - t0
    if r.returncode != 0:
        raise SystemExit(f"[base] FAIL {label} 退出码 {r.returncode}")
    return dt


def main(argv: list[str]) -> int:
    cases = l4_cases()
    if "--cases" in argv:
        cases = [c.strip() for c in argv[argv.index("--cases") + 1].split(",") if c.strip()]
    fresh = "--fresh" in argv
    out_json = None
    if "--json" in argv:
        out_json = Path(argv[argv.index("--json") + 1])

    WORK.mkdir(parents=True, exist_ok=True)
    results: list[dict] = []
    for i, case in enumerate(cases, 1):
        print(f"\n===== [{i}/{len(cases)}] {case} =====", flush=True)
        rec: dict = {"case": case}
        try:
            sem.cmd_oracle_case(case)
            txt = WORK / f"{case}.sem.txt"
            oracle = TRACES / f"{case}.sem.frames.jsonl"
            reuse = (not fresh) and oracle.exists() and oracle.stat().st_mtime > txt.stat().st_mtime
            if reuse:
                print(f"[base] 复用 oracle 缓存（{oracle.stat().st_size} B）")
                rec["oracle_s"] = 0.0
                rec["oracle_reused"] = True
            else:
                # Mesen 单实例：必须串行，且这里不开并发
                rec["oracle_s"] = round(run(
                    [sys.executable, "tools/scenario_run.py", "run", str(txt)], f"{case} oracle"), 1)
                rec["oracle_reused"] = False

            sched = sem._derive_sched(case)
            rust_jsonl = WORK / f"{case}.rust.jsonl"
            rec["rust_s"] = round(run(
                ["cargo", "run", "-q", "-p", "game-core", "--bin", "rust_screenshot", "--",
                 "--schedule", str(sched), "--out", str(WORK / f"{case}.rust.out"),
                 "--state-out", str(rust_jsonl)], f"{case} rust"), 1)

            a_sem = WORK / f"{case}.oracle.sem"
            b_sem = WORK / f"{case}.rust.sem"
            fa, na_a = sem.build_sem(oracle, a_sem)
            fb, na_b = sem.build_sem(rust_jsonl, b_sem)
            code, lines, st = sem.compare_sem(a_sem, b_sem)
            rec.update({
                "frames_oracle": fa, "frames_rust": fb,
                "na_oracle": {k: v for k, v in na_a.items() if v},
                "na_rust": {k: v for k, v in na_b.items() if v},
                "verdict": "match" if code == 0 else "diverge",
                **{k: st[k] for k in ("common", "matched", "diverged", "frames_only_oracle",
                                      "frames_only_rust", "cols", "not_covered",
                                      "phase_sensitive", "per_col", "first_diverge", "truncated")},
            })
            for ln in lines:
                print(ln)
        except SystemExit as e:  # 单案失败不中断整批（基线要全貌）
            rec["error"] = str(e)
            print(f"[base] {case} 失败：{e}", file=sys.stderr)
        results.append(rec)

    doc = {
        "schema": "sem-baseline/v1",
        "generated_at": time.strftime("%Y-%m-%dT%H:%M:%S%z"),
        "tool": "tools/sem.py compare_sem（判据唯一实现）",
        "cases": results,
    }
    bp = WORK / "baseline.json"
    bp.write_text(json.dumps(doc, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    if out_json:
        Path(out_json).write_text(json.dumps(doc, ensure_ascii=False, indent=2) + "\n",
                                  encoding="utf-8")

    # ---- markdown 汇总（可直接贴进 findings）----
    print("\n\n| 用例 | 帧(o/r) | 共同 | 全同 | 全同率 | 首个语义分叉 | 分歧列（帧数） |")
    print("|---|---|---|---|---|---|---|")
    tot_c = tot_m = 0
    for r in results:
        if "error" in r:
            print(f"| `{r['case']}` | — | — | — | — | **失败**：{r['error'][:60]} | — |")
            continue
        tot_c += r["common"]
        tot_m += r["matched"]
        fd = r.get("first_diverge")
        fdtxt = "—" if not fd else f"f{fd['frame']} " + ",".join(fd["cols"].keys())
        pc = sorted(r["per_col"].items(), key=lambda kv: -kv[1])
        pctxt = " ".join(f"{c}:{n}" for c, n in pc[:6]) or "—"
        rate = f"{100.0 * r['matched'] / r['common']:.1f}%" if r["common"] else "—"
        print(f"| `{r['case']}` | {r['frames_oracle']}/{r['frames_rust']} | {r['common']} | "
              f"{r['matched']} | {rate} | {fdtxt} | {pctxt} |")
    if tot_c:
        print(f"| **合计** | | **{tot_c}** | **{tot_m}** | **{100.0 * tot_m / tot_c:.1f}%** | | |")
    print(f"\n基线 → {bp}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
