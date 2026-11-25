#!/usr/bin/env python3
"""handler 触达清单（`data/status/handler_touchlist.json`）的唯一生成入口。

清单是什么：L4 腿 sidecar（`shots_rust/<case>.divergences.json` 的 `gap_map`）里出现过的
`unported!` 桩地址白名单 + 逐 addr 的触达计数。门禁侧判据在 `gates/cli.py::l4_touchlist_violations`
（判定逻辑只允许在 gates/，AGENTS §2），本工具**只负责重生成**，不持有判定权：

- 头部聚合 `cases_covered` / `total_events` / `unique_addrs` 必须能由 sidecar 机械重算；
  否则就是「手改头数字」的静默漂移（gates 侧会拦，2026-11-18 复核 097733e1 的 P2）。
- 某 addr 因合法用例变更/推点接线而不再触发时，**不删条目**：把 `n_events` 归零、清空
  `cases`，并写 `stale_since` + `stale_note` 说明成因（gates 侧对 `n_events>0` 的项判「计数陈旧」）。
  条目本身保留，作为「这个桩曾经被记账、现在已接线」的历史证据。
- sidecar 里出现清单外 addr ⇒ exit 2 且**不写盘**：新账目必须人先补 `asm_label`/`push_site`/
  `asm_ref` 归因（这是「账外=红」的补登记路径，不是自动放行）。

用法：
    python3 tools/handler_touchlist.py                 # --check：只报告漂移，exit 1 表示需重生成
    python3 tools/handler_touchlist.py --write --reason "…"   # 重生成（写盘）
    python3 tools/handler_touchlist.py --write --reason "…" --dry-run   # 打印新内容不写

写盘是幂等的：连续跑两次，第二次 `--check` 为 0 漂移。
JSON 采用固定键序 + entries 按 addr 升序 + indent=1 + 末尾 LF（沿用本文件既有排版，
避免整份重排的无意义 diff）。
"""
from __future__ import annotations

import argparse
import json
import sys
from datetime import date
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DEFAULT_LEDGER = ROOT / "data" / "status" / "handler_touchlist.json"
DEFAULT_MATRIX = ROOT / "shots_rust" / "matrix.json"
DEFAULT_SIDECAR_DIR = ROOT / "shots_rust"
SIDECAR_SUFFIX = ".divergences.json"
SENTINEL_FIRST_TICK = 1000000000  # 历史哨兵：当时没有逐事件 tick，用「未知」占位
EMPTY_AGGS = ""


class TouchlistError(Exception):
    """可预期拒绝（CLI 映射为 exit 2，不裸抛 traceback）。"""


def load_sidecars(matrix_path: Path, sidecar_dir: Path) -> tuple[list[str], dict[int, int], dict[int, int], dict[int, dict[str, int]], dict]:
    """按矩阵内案子收集 sidecar，返回 (案名, 逐 addr 事件数, 逐 addr 首 tick, 逐 addr 逐案事件数, 矩阵头)。

    只对**矩阵内**的案记账：`movies/cases/*` 的探索性探针也会落 sidecar，其记账 addr 是
    「还没接的 handler」的勘探结果，不应污染 L4 账。判据取自矩阵本身（rows[].case），
    不另设名单——名单会漂移（与 gates 侧同口径）。
    """
    if not matrix_path.is_file():
        raise TouchlistError(f"矩阵缺失：{matrix_path}（先跑 tools/l4_diff_run.py run-all）")
    matrix = json.loads(matrix_path.read_text(encoding="utf-8"))
    if matrix.get("generated_by") != "tools/l4_diff_run.py run-all":
        raise TouchlistError(f"非 run-all 矩阵产物（generated_by={matrix.get('generated_by')!r}）")
    cases = {r.get("case") for r in (matrix.get("rows") or []) if r.get("case")}
    if not cases:
        raise TouchlistError("矩阵 rows 为空——没有可记账的案子")
    sidecars = [p for p in sorted(sidecar_dir.glob(f"*{SIDECAR_SUFFIX}"))
                if p.name[: -len(SIDECAR_SUFFIX)] in cases]
    totals: dict[int, int] = {}
    first_tick: dict[int, int] = {}
    per_case: dict[int, dict[str, int]] = {}
    for sp in sidecars:
        case = sp.name[: -len(SIDECAR_SUFFIX)]
        doc = json.loads(sp.read_text(encoding="utf-8"))
        for g in doc.get("gap_map") or []:
            addr = int(g["addr"])
            n = int(g.get("n", 0))
            totals[addr] = totals.get(addr, 0) + n
            per_case.setdefault(addr, {})
            per_case[addr][case] = per_case[addr].get(case, 0) + n
        for ev in doc.get("events") or []:
            addr = int(ev["addr"])
            tick = int(ev["tick"])
            if addr not in first_tick or tick < first_tick[addr]:
                first_tick[addr] = tick
    return sorted(case_names(sidecars)), totals, first_tick, per_case, matrix


def case_names(sidecars: list[Path]) -> list[str]:
    return [p.name[: -len(SIDECAR_SUFFIX)] for p in sidecars]


def build(ledger: dict, totals: dict[int, int], per_case: dict[int, dict[str, int]],
          cases_covered: int, reason: str, stale_note: str, matrix: dict) -> tuple[dict, list[str]]:
    """按 sidecar 重算清单；返回 (新清单, 变化说明)。

    只刷新「可机械重算」的字段：头部 cases_covered/total_events/unique_addrs、逐条目
    n_events/cases、零事件条目的 stale_*。归因字段（addr/asm_label/push_site/asm_ref）与
    first_tick 原样保留——first_tick 是早期无逐事件 tick 时留下的历史值，本工具不做回填
    （逐事件 tick 以 sidecar `events` 为准）。"""
    changes: list[str] = []
    today = date.today().isoformat()
    by_addr: dict[int, dict] = {int(str(e["addr"]), 16): dict(e) for e in ledger.get("entries") or []}
    unknown = sorted(set(totals) - set(by_addr))
    if unknown:
        raise TouchlistError(
            "sidecar 出现清单外 addr（账外=红，须先补归因再重生成）："
            + ", ".join(f"0x{a:04X}" for a in unknown))
    for addr, entry in by_addr.items():
        observed = totals.get(addr, 0)
        old = int(entry.get("n_events", 0))
        if observed:
            entry["n_events"] = observed
            entry["cases"] = dict(sorted(per_case.get(addr, {}).items()))
            entry.pop("stale_since", None)
            entry.pop("stale_note", None)
        else:
            entry["n_events"] = 0
            entry["cases"] = {}
            if old > 0 or "stale_since" not in entry:
                entry["stale_since"] = today
                entry["stale_note"] = stale_note or "本次 sidecar 零事件（计数陈旧）：需写清是推点接线还是用例变更"
                changes.append(f"0x{addr:04X}: {old} -> 0（转 stale）")
        if observed and observed != old:
            changes.append(f"0x{addr:04X}: n_events {old} -> {observed}")
    entries = [by_addr[a] for a in sorted(by_addr)]
    new = dict(ledger)
    new["schema"] = ledger.get("schema", "handler-touchlist/v1")
    new["cases_covered"] = cases_covered
    new["unique_addrs"] = len(entries)
    new["total_events"] = sum(int(e.get("n_events", 0)) for e in entries)
    new["entries"] = entries
    run_at = matrix.get("at") or matrix.get("generated_at") or ""
    origin = f"python3 tools/l4_diff_run.py run-all @ {run_at}" if run_at else "tools/l4_diff_run.py run-all"
    new["source_run"] = (f"{origin}（{cases_covered} 案 / {matrix.get('cells_total')} 格，"
                         f"match {matrix.get('match')}·diff {matrix.get('diff')}·blocked {matrix.get('blocked')}）；"
                         f"重生成：{reason}")
    return new, changes


def drift(ledger: dict, new: dict) -> list[str]:
    """清单与 sidecar 重算结果的漂移（空=一致）。只比可机械重算的字段。"""
    out: list[str] = []
    for key in ("cases_covered", "total_events", "unique_addrs"):
        if ledger.get(key) != new.get(key):
            out.append(f"头部 {key}: 清单 {ledger.get(key)} ≠ 重算 {new.get(key)}")
    old_entries = {int(str(e["addr"]), 16): int(e.get("n_events", 0)) for e in ledger.get("entries") or []}
    new_entries = {int(str(e["addr"]), 16): int(e.get("n_events", 0)) for e in new.get("entries") or []}
    for addr in sorted(set(old_entries) | set(new_entries)):
        if old_entries.get(addr, 0) != new_entries.get(addr, 0):
            out.append(f"0x{addr:04X}: n_events 清单 {old_entries.get(addr, 0)} ≠ 重算 {new_entries.get(addr, 0)}")
    stale_bad = [f"0x{int(str(e['addr']), 16):04X}" for e in new.get("entries") or []
                 if int(e.get("n_events", 0)) > 0 and e.get("stale_since")]
    if stale_bad:
        out.append("非零计数却挂着 stale 标记：" + ", ".join(stale_bad))
    return out


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description="handler 触达清单唯一生成入口（只重算，不判绿）")
    ap.add_argument("--check", action="store_true", help="只报告漂移（默认行为）；exit 1=需重生成")
    ap.add_argument("--write", action="store_true", help="按 sidecar 重生成清单")
    ap.add_argument("--reason", default=EMPTY_AGGS, help="--write 必填：本次重生成的原因（进 source_run）")
    ap.add_argument("--stale-note", default=EMPTY_AGGS, help="零事件条目转 stale 时写入 stale_note 的成因（默认同 --reason）")
    ap.add_argument("--dry-run", action="store_true", help="与 --write 合用：打印新清单但不写盘")
    ap.add_argument("--ledger", type=Path, default=DEFAULT_LEDGER)
    ap.add_argument("--matrix", type=Path, default=DEFAULT_MATRIX)
    ap.add_argument("--sidecar-dir", type=Path, default=DEFAULT_SIDECAR_DIR)
    args = ap.parse_args(argv)

    try:
        ledger = json.loads(args.ledger.read_text(encoding="utf-8"))
        cases, totals, _first_tick, per_case, matrix = load_sidecars(args.matrix, args.sidecar_dir)
        new, changes = build(ledger, totals, per_case, len(cases), args.reason,
                             args.stale_note or args.reason, matrix)
    except (TouchlistError, OSError, json.JSONDecodeError) as exc:
        print(f"[handler-touchlist] {exc}", file=sys.stderr)
        return 2

    diffs = drift(ledger, new)
    if not args.write:
        if diffs:
            print("[handler-touchlist] 漂移（需 --write 重生成）：")
            for d in diffs:
                print(f"  - {d}")
            print("[handler-touchlist] 处置：python3 tools/handler_touchlist.py --write --reason \"<成因>\"")
            return 1
        print(f"[handler-touchlist] 无漂移：{len(new['entries'])} 条 / total_events={new['total_events']}"
              f" / cases_covered={new['cases_covered']}")
        return 0

    if not args.reason:
        print("[handler-touchlist] --write 需要 --reason（写清差异来源：新增/删除/改址/接线）", file=sys.stderr)
        return 2
    for c in changes:
        print(f"  {c}")
    if args.dry_run:
        print(json.dumps(new, ensure_ascii=False, indent=2))
        return 0
    if not diffs:
        print(f"[handler-touchlist] 无变化：{len(new['entries'])} 条 / total_events={new['total_events']}"
              f"（未改写 {args.ledger}）")
        return 0
    args.ledger.write_text(json.dumps(new, ensure_ascii=False, indent=1) + "\n", encoding="utf-8")
    print(f"[handler-touchlist] 已重写 {args.ledger}：{len(new['entries'])} 条 / "
          f"total_events={new['total_events']} / cases_covered={new['cases_covered']}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
