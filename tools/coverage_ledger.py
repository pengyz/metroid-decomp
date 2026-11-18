#!/usr/bin/env python3
"""覆盖台账：机制条目的三态推进（unobserved -> route-only -> verified）。

台账只记录「覆盖到哪一层」，不持有行为判定权；判定仍由对拍/门禁负责。
JSON 采用固定键序 + 按 id 排序 + indent=2 + LF，避免无意义 diff。
"""
from __future__ import annotations

import argparse
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DEFAULT_LEDGER = ROOT / "data" / "status" / "coverage_ledger.json"
DEFAULT_BASELINE = ROOT / "data" / "status" / "coverage_ledger.baseline.json"
KINDS = ("mechanism",)
STATES = ("unobserved", "route-only", "verified")
STATE_RANK = {state: index for index, state in enumerate(STATES)}

EVIDENCE = {
    "morphball-unroll": [
        "docs/findings/morphball-probe-route.md",
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


def baseline_payload(items: list[dict]) -> dict:
    ordered = sorted(items, key=lambda item: item["id"])
    return {
        "schema": "coverage-ledger-baseline/v1",
        "counts": _counts(ordered),
        "ids": [item["id"] for item in ordered],
        "states": {item["id"]: item["state"] for item in ordered},
    }


def seed(ledger_path: str | Path = DEFAULT_LEDGER,
         baseline_path: str | Path = DEFAULT_BASELINE) -> dict:
    ledger = Ledger(ledger_path)
    for entry_id, state in SEED:
        existing = next((item for item in ledger.items if item["id"] == entry_id), None)
        if existing is None:
            ledger.add(entry_id, "mechanism", state, EVIDENCE[entry_id])
        elif STATE_RANK[state] > STATE_RANK[existing["state"]]:
            ledger.promote(entry_id, state, EVIDENCE[entry_id][0])
    ledger.save()
    baseline = baseline_payload(ledger.items)
    baseline_path = Path(baseline_path)
    baseline_path.parent.mkdir(parents=True, exist_ok=True)
    baseline_path.write_text(
        json.dumps(baseline, ensure_ascii=False, indent=2) + "\n",
        encoding="utf-8",
        newline="\n",
    )
    return baseline


def main() -> int:
    parser = argparse.ArgumentParser(description="覆盖台账工具")
    parser.add_argument("--seed", action="store_true", help="生成/补齐初始三条机制条目与基线")
    parser.add_argument("--ledger", default=str(DEFAULT_LEDGER))
    parser.add_argument("--baseline", default=str(DEFAULT_BASELINE))
    args = parser.parse_args()
    if not args.seed:
        parser.error("当前只支持 --seed")
    baseline = seed(args.ledger, args.baseline)
    print(f"LEDGER: {args.ledger}")
    print(f"BASELINE: {args.baseline}")
    print("COUNTS: " + json.dumps(baseline["counts"], ensure_ascii=False, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
