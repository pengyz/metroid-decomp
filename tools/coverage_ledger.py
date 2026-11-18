#!/usr/bin/env python3
"""覆盖台账：机制条目的三态推进（unobserved -> route-only -> verified）。

台账只记录「覆盖到哪一层」，不持有行为判定权；判定仍由对拍/门禁负责。
JSON 采用固定键序 + 按 id 排序 + indent=2 + LF，避免无意义 diff。

基线有两个互斥入口，均不得降级：
- --init：只在基线不存在时建立 genesis 锚；基线已存在即 exit 2。
- --update --reason TEXT：从旧基线逐项取 max（counts/ids/states），
  当前台账任一项低于旧基线即 exit 2 且不写盘；reason 与 updated_at 入基线。
"""
from __future__ import annotations

import argparse
import json
import sys
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DEFAULT_LEDGER = ROOT / "data" / "status" / "coverage_ledger.json"
DEFAULT_BASELINE = ROOT / "data" / "status" / "coverage_ledger.baseline.json"
KINDS = ("mechanism",)
STATES = ("unobserved", "route-only", "verified")
STATE_RANK = {state: index for index, state in enumerate(STATES)}


class LedgerError(Exception):
    """台账/基线的可预期拒绝（CLI 映射为 exit 2，不裸抛 traceback）。"""


EVIDENCE = {
    "morphball-unroll": [
        "docs/findings/morphball-probe-route.md",
        "docs/findings/movie-6725m-f1031-unroll.md",
        "cases/case-morphball-probe.txt",
        "shots_rust/case-morphball-probe.l4report.json",
    ],
    "room-load-chain": [
        "data/status/leg_episodes.json",
        "docs/findings/movie-bizhawk-6725m.md",
        "data/status/leg_offsets.json",
    ],
    "door-transition": [
        "docs/findings/case-002-three-roots.md",
        "shots_rust/case-002-door-blue-interaction.sched",
        "shots_rust/case-002-door-blue-interaction.l4report.json",
    ],
}
SEED = (
    ("morphball-unroll", "route-only"),
    ("room-load-chain", "route-only"),
    ("door-transition", "route-only"),
)


def _entry(entry_id: str, kind: str, state: str, evidence: list[str] | None = None) -> dict:
    return {
        "id": entry_id,
        "kind": kind,
        "state": state,
        "evidence": list(evidence or []),
    }


def _validate_item(item: dict) -> None:
    if not isinstance(item, dict):
        raise ValueError("ledger item must be an object")
    if not isinstance(item.get("id"), str) or not item["id"].strip():
        raise ValueError("ledger item id must be a non-empty string")
    if item.get("kind") not in KINDS:
        raise ValueError(f"ledger item {item.get('id')!r}: kind must be one of {KINDS}")
    if item.get("state") not in STATE_RANK:
        raise ValueError(f"ledger item {item.get('id')!r}: state must be one of {STATES}")
    evidence = item.get("evidence")
    if not isinstance(evidence, list) or not all(isinstance(x, str) for x in evidence):
        raise ValueError(f"ledger item {item.get('id')!r}: evidence must be a list of strings")


class Ledger:
    """一个 JSON 文件就是一个台账；构造时读取，save() 时稳定落盘。"""

    def __init__(self, path: str | Path):
        self.path = Path(path)
        self.items: list[dict] = []
        self.load()

    def load(self) -> list[dict]:
        if not self.path.is_file():
            self.items = []
            return self.items
        try:
            data = json.loads(self.path.read_text(encoding="utf-8"))
        except json.JSONDecodeError as exc:
            raise ValueError(f"ledger JSON parse failed: {self.path}: {exc}") from exc
        if not isinstance(data, dict) or not isinstance(data.get("items"), list):
            raise ValueError("ledger root must be an object with an items list")
        seen: set[str] = set()
        items: list[dict] = []
        for item in data["items"]:
            _validate_item(item)
            if item["id"] in seen:
                raise ValueError(f"duplicate id in ledger: {item['id']}")
            seen.add(item["id"])
            items.append(_entry(item["id"], item["kind"], item["state"], item.get("evidence")))
        self.items = items
        return self.items

    def save(self) -> None:
        payload = json.dumps(
            {"items": sorted(self.items, key=lambda item: item["id"])},
            ensure_ascii=False,
            indent=2,
        ) + "\n"
        self.path.parent.mkdir(parents=True, exist_ok=True)
        self.path.write_text(payload, encoding="utf-8", newline="\n")

    def add(self, entry_id: str, kind: str = "mechanism", state: str = "unobserved",
            evidence: list[str] | None = None) -> dict:
        if kind not in KINDS:
            raise ValueError(f"kind must be one of {KINDS}")
        if state not in STATE_RANK:
            raise ValueError(f"state must be one of {STATES}")
        if any(item["id"] == entry_id for item in self.items):
            raise ValueError(f"duplicate id: {entry_id}")
        item = _entry(entry_id, kind, state, evidence)
        self.items.append(item)
        return item

    def promote(self, entry_id: str, state: str, evidence: str) -> None:
        if state not in STATE_RANK:
            raise ValueError(f"state must be one of {STATES}")
        item = next((item for item in self.items if item["id"] == entry_id), None)
        if item is None:
            raise ValueError(f"unknown id: {entry_id}")
        old_rank = STATE_RANK[item["state"]]
        new_rank = STATE_RANK[state]
        if new_rank < old_rank:
            raise ValueError(
                f"downgrade refused: {entry_id} {item['state']} -> {state}"
            )
        item["state"] = state
        if evidence and evidence not in item["evidence"]:
            item["evidence"].append(evidence)


def _counts(items: list[dict]) -> dict[str, int]:
    return {state: sum(1 for item in items if item["state"] == state) for state in STATES}


def _baseline_doc(items: list[dict], reason: str) -> dict:
    ordered = sorted(items, key=lambda item: item["id"])
    return {
        "schema": "coverage-ledger-baseline/v1",
        "counts": _counts(ordered),
        "ids": [item["id"] for item in ordered],
        "states": {item["id"]: item["state"] for item in ordered},
        "reason": reason,
        "updated_at": datetime.now(timezone.utc).isoformat(timespec="seconds"),
    }


PROVENANCE_KEYS = ("reason", "updated_at")


def _sync_anchors() -> None:
    """写盘后同步 SHA256SUMS 状态锚点（唯一实现留在 gates/cli.py，此处只调用）。"""
    import subprocess

    proc = subprocess.run(
        [sys.executable, str(ROOT / "gates" / "cli.py"), "sync-anchors"],
        capture_output=True,
        text=True,
        cwd=str(ROOT),
    )
    if proc.returncode != 0:
        raise LedgerError("锚点同步失败（gates/cli.py sync-anchors）：" + (proc.stderr or proc.stdout).strip())


def _write_baseline(path: Path, doc: dict) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        json.dumps(doc, ensure_ascii=False, indent=2) + "\n",
        encoding="utf-8",
        newline="\n",
    )


def _load_baseline(path: Path) -> dict:
    if not path.is_file():
        raise LedgerError(f"基线不存在：{path}")
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except json.JSONDecodeError as exc:
        raise LedgerError(f"基线 JSON 损坏：{path}: {exc}") from exc
    if not isinstance(data, dict):
        raise LedgerError("基线根必须是对象")
    counts = data.get("counts")
    if not isinstance(counts, dict) or set(counts) != set(STATES):
        raise LedgerError(f"基线 counts 必须含 {STATES}")
    for state in STATES:
        value = counts[state]
        if not isinstance(value, int) or isinstance(value, bool) or value < 0:
            raise LedgerError(f"基线 counts.{state} 非法：{value!r}")
    ids = data.get("ids")
    if not isinstance(ids, list) or not all(isinstance(x, str) for x in ids):
        raise LedgerError("基线 ids 必须是字符串列表")
    if len(set(ids)) != len(ids):
        raise LedgerError("基线 ids 含重复")
    states = data.get("states")
    if not isinstance(states, dict):
        raise LedgerError("基线缺 states 对象")
    if set(states) != set(ids):
        raise LedgerError("基线 states 的 id 集合必须与 ids 一致")
    if _counts([{"state": states[i]} for i in ids]) != counts:
        raise LedgerError("基线 counts 与 states 不一致")
    return data


def _downgrade_problems(old: dict, items: list[dict]) -> list[str]:
    problems: list[str] = []
    current = {item["id"]: item for item in items}
    old_ids = set(old["ids"])
    current_ids = set(current)
    missing = sorted(old_ids - current_ids)
    if missing:
        problems.append("条目 id 删除：" + ", ".join(missing))
    for entry_id in sorted(old_ids & current_ids):
        old_state = old["states"][entry_id]
        new_state = current[entry_id]["state"]
        if STATE_RANK[new_state] < STATE_RANK[old_state]:
            problems.append(
                f"条目 state 回退：{entry_id} {new_state} < 基线 {old_state}"
            )
    current_counts = _counts(items)
    old_counts = old["counts"]
    # 逐 state 的 exact count 在合法升级（route-only -> verified）时必然下降，
    # 不能作为单调判据；用累计口径：rank >= state 的条目数不得减少。
    for index, state in enumerate(STATES):
        current_at_least = sum(current_counts[s] for s in STATES[index:])
        old_at_least = sum(old_counts[s] for s in STATES[index:])
        if current_at_least < old_at_least:
            problems.append(
                f"state 累计计数回退：{state} {current_at_least} < 基线 {old_at_least}"
            )
    return problems


def _merged_doc(old: dict, items: list[dict], reason: str) -> dict:
    current = {item["id"]: item for item in items}
    merged_states: dict[str, str] = {}
    for entry_id in sorted(set(old["ids"]) | set(current)):
        old_state = old["states"].get(entry_id)
        new_state = current[entry_id]["state"] if entry_id in current else old_state
        if old_state is None:
            merged_states[entry_id] = new_state
        elif new_state is None:
            merged_states[entry_id] = old_state
        else:
            merged_states[entry_id] = (
                new_state
                if STATE_RANK[new_state] >= STATE_RANK[old_state]
                else old_state
            )
    ordered = sorted(merged_states)
    return {
        "schema": "coverage-ledger-baseline/v1",
        "counts": _counts([{"state": merged_states[i]} for i in ordered]),
        "ids": ordered,
        "states": {i: merged_states[i] for i in ordered},
        "reason": reason,
        "updated_at": datetime.now(timezone.utc).isoformat(timespec="seconds"),
    }


def init(ledger_path: str | Path = DEFAULT_LEDGER,
         baseline_path: str | Path = DEFAULT_BASELINE) -> dict:
    """genesis：仅当基线不存在时建立；基线已存在即拒绝（不重写、不降级）。"""
    ledger_path = Path(ledger_path)
    baseline_path = Path(baseline_path)
    if baseline_path.exists():
        raise LedgerError(f"基线已存在，拒绝 --init 重建：{baseline_path}")
    ledger = Ledger(ledger_path)
    for entry_id, state in SEED:
        existing = next((item for item in ledger.items if item["id"] == entry_id), None)
        if existing is None:
            ledger.add(entry_id, "mechanism", state, EVIDENCE[entry_id])
        elif STATE_RANK[state] > STATE_RANK[existing["state"]]:
            ledger.promote(entry_id, state, EVIDENCE[entry_id][0])
    ledger.save()
    baseline = _baseline_doc(ledger.items, "genesis: --init")
    _write_baseline(baseline_path, baseline)
    _sync_anchors()
    return baseline


def update(ledger_path: str | Path = DEFAULT_LEDGER,
           baseline_path: str | Path = DEFAULT_BASELINE,
           reason: str = "") -> dict:
    """从旧基线出发逐项取 max；任一项低于旧基线即拒绝且不写盘。"""
    ledger_path = Path(ledger_path)
    baseline_path = Path(baseline_path)
    reason = (reason or "").strip()
    if not reason:
        raise LedgerError("--update 必须 --reason 留说明（对齐 baseline_update.py）")
    old = _load_baseline(baseline_path)
    ledger = Ledger(ledger_path)
    problems = _downgrade_problems(old, ledger.items)
    if problems:
        raise LedgerError("拒绝降级（未写盘）：" + "；".join(problems))
    baseline = _merged_doc(old, ledger.items, reason)
    # 内容等价（除 reason/updated_at 外逐键相同）⇒ 不写盘，避免无意义 diff。
    if {k: v for k, v in old.items() if k not in PROVENANCE_KEYS} == {
        k: v for k, v in baseline.items() if k not in PROVENANCE_KEYS
    }:
        print("NO-OP: 基线已是上界，未写盘", file=sys.stderr)
        return old
    _write_baseline(baseline_path, baseline)
    return baseline


def check(ledger_path: str | Path = DEFAULT_LEDGER,
          baseline_path: str | Path = DEFAULT_BASELINE) -> int:
    """只读自检：当前台账相对基线是否有降级。"""
    old = _load_baseline(Path(baseline_path))
    ledger = Ledger(ledger_path)
    problems = _downgrade_problems(old, ledger.items)
    if problems:
        print("LEDGER CHECK: FAIL —— " + "；".join(problems), file=sys.stderr)
        return 1
    print("LEDGER CHECK: OK")
    return 0


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="覆盖台账工具")
    group = parser.add_mutually_exclusive_group(required=True)
    group.add_argument("--init", action="store_true", help="仅在基线不存在时建立 genesis 锚")
    group.add_argument("--update", action="store_true", help="从旧基线逐项取 max（只升不降）")
    group.add_argument("--check", action="store_true", help="只读检查当前台账是否低于基线")
    parser.add_argument("--reason", default="", help="--update 必填：确认无回退的说明")
    parser.add_argument("--ledger", default=str(DEFAULT_LEDGER))
    parser.add_argument("--baseline", default=str(DEFAULT_BASELINE))
    args = parser.parse_args(argv)
    try:
        if args.init:
            baseline = init(args.ledger, args.baseline)
        elif args.update:
            if not args.reason.strip():
                parser.error("--update 必须 --reason")
            baseline = update(args.ledger, args.baseline, args.reason)
        else:
            return check(args.ledger, args.baseline)
    except LedgerError as exc:
        print(f"COVERAGE LEDGER: {exc}", file=sys.stderr)
        return 2
    print(f"LEDGER: {args.ledger}")
    print(f"BASELINE: {args.baseline}")
    print("COUNTS: " + json.dumps(baseline["counts"], ensure_ascii=False, sort_keys=True))
    print("REASON: " + baseline["reason"])
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
