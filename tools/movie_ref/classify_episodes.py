#!/usr/bin/env python3
"""Task 2：把三腿逐帧失配聚合成 episode，并二分类为 phase / value。

帧轴：Mesen2。`offsets` 语义来自 Task 1（`a[f] == b[f + offset]`），因此 Mesen2
帧 g 对应另一腿的帧 `g - offset[leg]`。偏移**只从** `data/status/leg_offsets.json`
读，不在本文件里硬编码。

分类判据（按 Task 2 补充口径的逐帧 ±2；计划 Step 3 写的是段窗
[start-2, end+2]，本实现取更严的逐帧口径，窗宽固定 2，不得调）：
- 段内任一帧任一失配字段，若该腿的值出现在 Mesen2 同字段 [g-2, g+2] 内，该帧该字段为 phase；
- 一段只要有一个失配帧/字段是 value，整段记 value；phase_fields 只保留全段均为 phase 的字段。
- `during_load`：段内任一帧任一条腿的 `$005A`（ROOM_NUMBER）!= 0xFF。

加载窗帧按计划 R7 不进分母，但**仍然**聚合成 episode 并打 `during_load` 标记；
denominators 里显式给出总帧 / 分母 / 排除帧及原因，不许静默丢。
"""

from __future__ import annotations

import argparse
import json
import os
import sys
from datetime import datetime
from pathlib import Path

# 直接以 `python3 tools/movie_ref/classify_episodes.py` 运行时，sys.path[0]
# 是 tools/movie_ref；补仓库根以便 `from tools...` 生效。
_ROOT = Path(__file__).resolve().parents[2]
if str(_ROOT) not in sys.path:
    sys.path.insert(0, str(_ROOT))

from tools.movie_ref.align_legs import LEGS, load_state

FIELDS = ("status", "room", "health_lo", "health_hi", "scrolldir")
PHASE_WINDOW = 2
LOAD_ROOM = 0xFF
DEFAULT_START = 20


def _normalize_offsets(offsets: dict[str, int]) -> dict[str, int]:
    """接受 {"biz":1,...} 或 Task 1 JSON 的 {"biz_to_mesen":1,...}。"""
    out: dict[str, int] = {"mesen": 0}
    for key, value in offsets.items():
        if key == "mesen":
            if value not in (0, None):
                raise ValueError(f"mesen offset must be 0, got {value!r}")
            continue
        leg = key[:-len("_to_mesen")] if key.endswith("_to_mesen") else key
        if leg not in LEGS or leg == "mesen":
            raise ValueError(f"unknown offset key {key!r}")
        if isinstance(value, bool) or not isinstance(value, int):
            raise ValueError(f"offset for {leg!r} must be an int, got {value!r}")
        out[leg] = value
    return out


def _record(legs: dict[str, dict[int, dict]], leg: str, frame: int) -> dict | None:
    rows = legs.get(leg)
    if rows is None:
        return None
    return rows.get(frame)


def _mismatch_fields(
    mesen_row: dict,
    leg_row: dict,
    fields: tuple[str, ...],
) -> list[str]:
    out = []
    for field in fields:
        if leg_row.get(field) != mesen_row.get(field):
            out.append(field)
    return out


def _during_load_frames(
    legs: dict[str, dict[int, dict]],
    offsets: dict[str, int],
    start: int,
    end: int,
    load_key: str,
) -> list[int]:
    out = []
    for g in range(start, end + 1):
        for leg, rows in legs.items():
            frame = g - (0 if leg == "mesen" else offsets.get(leg, 0))
            row = rows.get(frame)
            if row is not None and load_key in row and row.get(load_key) != LOAD_ROOM:
                out.append(g)
                break
    return out


def _classify_one_leg(
    leg: str,
    legs: dict[str, dict[int, dict]],
    offsets: dict[str, int],
    fields: tuple[str, ...],
    start: int,
    end: int,
    load_key: str,
) -> list[dict]:
    offset = offsets.get(leg)
    if offset is None:
        raise ValueError(f"offsets missing leg {leg!r}")
    if leg not in legs:
        return []
    mesen_rows = legs.get("mesen")
    if not mesen_rows:
        raise ValueError("legs must include a non-empty mesen leg")

    episodes: list[dict] = []
    current: dict | None = None

    def flush() -> None:
        nonlocal current
        if current is None:
            return
        frames = current.pop("_frames")
        current["frames"] = len(frames)
        current["evidence"] = _build_evidence(
            leg, legs, offsets, fields, frames, load_key, current["fields"]
        )
        current["class"] = _episode_class(current["evidence"])
        current["kind"] = current["class"]
        current["during_load"] = bool(current["evidence"]["during_load_frames"])
        episodes.append(current)
        current = None

    for g in range(start, end + 1):
        mesen_row = mesen_rows.get(g)
        leg_row = legs[leg].get(g - offset)
        if mesen_row is None or leg_row is None:
            flush()
            continue
        mismatch = _mismatch_fields(mesen_row, leg_row, fields)
        if not mismatch:
            flush()
            continue
        if current is None:
            current = {
                "leg": leg,
                "start": g,
                "end": g,
                "_frames": [],
                "fields": [],
                "during_load": False,
            }
        current["end"] = g
        current["_frames"].append(g)
        for field in mismatch:
            if field not in current["fields"]:
                current["fields"].append(field)
        current["during_load"] = current["during_load"] or bool(
            _during_load_frames(legs, offsets, g, g, load_key)
        )
    flush()
    return episodes


def _build_evidence(
    leg: str,
    legs: dict[str, dict[int, dict]],
    offsets: dict[str, int],
    fields: tuple[str, ...],
    frames: list[int],
    load_key: str,
    episode_fields: list[str],
) -> dict:
    offset = offsets[leg]
    mesen_rows = legs["mesen"]
    phase_fields: set[str] = set()
    value_fields: set[str] = set()
    examples: list[dict] = []

    for g in frames:
        mesen_row = mesen_rows.get(g)
        leg_row = legs[leg].get(g - offset)
        if mesen_row is None or leg_row is None:
            continue
        for field in episode_fields:
            leg_value = leg_row.get(field)
            if leg_value == mesen_row.get(field):
                continue
            opposite = []
            for gg in range(g - PHASE_WINDOW, g + PHASE_WINDOW + 1):
                row = mesen_rows.get(gg)
                if row is not None:
                    opposite.append(row.get(field))
            found = leg_value in opposite
            if found:
                phase_fields.add(field)
            else:
                value_fields.add(field)
                if len(examples) < 3:
                    examples.append(
                        {
                            "frame": g,
                            "field": field,
                            "leg_value": leg_value,
                            "opposite_values": opposite,
                        }
                    )

    during = _during_load_frames(
        legs, offsets, frames[0], frames[-1], load_key
    )
    return {
        "rule": "phase iff for every mismatched frame/field the leg value "
                f"occurs in mesen within +-{PHASE_WINDOW} frames",
        "mesen_frame_window": [frames[0], frames[-1]],
        "leg_frame_window": [frames[0] - offset, frames[-1] - offset],
        "leg_offset": offset,
        "mismatch_frames": frames,
        "phase_fields": [f for f in episode_fields if f not in value_fields],
        "value_fields": [f for f in episode_fields if f in value_fields],
        "value_examples": examples,
        "during_load_frames": during,
    }


def _episode_class(evidence: dict) -> str:
    return "value" if evidence["value_fields"] else "phase"


def classify(
    legs: dict[str, dict[int, dict]],
    offsets: dict[str, int],
    fields: tuple[str, ...],
    start: int,
    end: int,
    load_key: str = "room",
) -> list[dict]:
    """把三腿逐帧失配聚合成 episode。返回按 (start, leg) 排序的段列表。

    legs 必须含 mesen；另含 biz/rust。offsets 可传 {"biz":1,...} 或
    {"biz_to_mesen":1,...}。每个 episode 同时带 `class` 与 `kind`（同值），
    兼容计划接口与 Task 2 补充口径。
    """
    if start > end:
        raise ValueError(f"start {start} > end {end}")
    if "mesen" not in legs:
        raise ValueError("legs must include mesen")
    fields = tuple(fields)
    if not fields:
        raise ValueError("fields must not be empty")
    for field in fields:
        if field not in FIELDS:
            raise ValueError(f"unknown field {field!r}; expected one of {FIELDS}")

    normalized = _normalize_offsets(offsets)
    episodes: list[dict] = []
    for leg in ("biz", "rust"):
        if leg in legs:
            if leg not in normalized:
                raise ValueError(f"offsets missing leg {leg!r}")
            episodes.extend(
                _classify_one_leg(
                    leg, legs, normalized, fields, start, end, load_key
                )
            )
    episodes.sort(key=lambda e: (e["start"], e["leg"]))
    return episodes


def _denominators(
    legs: dict[str, dict[int, dict]],
    offsets: dict[str, int],
    start: int,
    end: int,
    load_key: str,
) -> dict:
    total = end - start + 1
    load_frames: set[int] = set()
    missing_frames: set[int] = set()
    per_leg: dict[str, dict] = {}

    for leg in ("biz", "rust"):
        if leg not in legs:
            continue
        offset = offsets[leg]
        leg_load: set[int] = set()
        leg_missing: set[int] = set()
        for g in range(start, end + 1):
            mesen_row = legs["mesen"].get(g)
            leg_row = legs[leg].get(g - offset)
            if mesen_row is None or leg_row is None:
                leg_missing.add(g)
                missing_frames.add(g)
                continue
            if (
                load_key in mesen_row
                and load_key in leg_row
                and (
                    mesen_row.get(load_key) != LOAD_ROOM
                    or leg_row.get(load_key) != LOAD_ROOM
                )
            ):
                leg_load.add(g)
                load_frames.add(g)
        per_leg[leg] = {
            "total": total,
            "in_denominator": total - len(leg_load | leg_missing),
            "excluded_load": len(leg_load),
            "excluded_missing": len(leg_missing),
        }

    excluded_union = load_frames | missing_frames
    return {
        "total_frames": total,
        "in_denominator": total - len(excluded_union),
        "excluded": {
            "load": len(load_frames),
            "missing": len(missing_frames),
            "union": len(excluded_union),
            "load_frames": sorted(load_frames),
            "missing_frames": sorted(missing_frames),
            "reasons": {
                "load": f"任一侧 ${0x005A:04X} != 0xFF（ROOM_NUMBER，加载窗；计划 R7）",
                "missing": "该 Mesen2 帧或该腿映射帧缺失（状态行缝），无可比样本",
            },
        },
        "by_leg": per_leg,
    }


def build_report(
    legs: dict[str, dict[int, dict]],
    offsets: dict[str, int],
    fields: tuple[str, ...],
    start: int,
    end: int,
    load_key: str = "room",
) -> dict:
    """分类 + 分母 + 摘要，不读文件；CLI 负责 provenance。"""
    normalized = _normalize_offsets(offsets)
    episodes = classify(legs, normalized, fields, start, end, load_key)
    denominators = _denominators(legs, normalized, start, end, load_key)

    by_class = {"phase": 0, "value": 0}
    by_leg: dict[str, int] = {}
    by_leg_class: dict[str, dict[str, int]] = {}
    by_leg_load: dict[str, dict[str, int]] = {}
    for episode in episodes:
        leg = episode["leg"]
        cls = episode["class"]
        by_class[cls] += 1
        by_leg[leg] = by_leg.get(leg, 0) + 1
        slot = by_leg_class.setdefault(leg, {"phase": 0, "value": 0})
        slot[cls] += 1
        load_slot = by_leg_load.setdefault(leg, {"during_load": 0, "clean": 0})
        key = "during_load" if episode["during_load"] else "clean"
        load_slot[key] += 1

    value_episodes = [e for e in episodes if e["class"] == "value"]
    value_rows = [
        {
            "leg": e["leg"],
            "start": e["start"],
            "end": e["end"],
            "frames": e["frames"],
            "fields": e["fields"],
            "example": e["evidence"]["value_examples"][:1],
        }
        for e in value_episodes[:3]
    ]

    report = {
        "window": {
            "start": start,
            "end": end,
            "fit_window": [start, end],
            "effective_end": end,
            "basis": "mesen_frame_axis",
        },
        "offsets": {
            "biz_to_mesen": normalized.get("biz"),
            "rust_to_mesen": normalized.get("rust"),
            "by_leg": normalized,
        },
        "fields": list(fields),
        "denominators": denominators,
        "episodes": episodes,
        "summary": {
            "total_episodes": len(episodes),
            "by_class": by_class,
            "by_leg": by_leg,
            "by_leg_class": by_leg_class,
            "by_leg_load": by_leg_load,
            "during_load": sum(1 for e in episodes if e["during_load"]),
            "clean": sum(1 for e in episodes if not e["during_load"]),
            "value_count": len(value_episodes),
            "phase_count": by_class["phase"],
            "value_episodes": value_rows,
        },
    }
    return report


def _fields_arg(value: str) -> tuple[str, ...]:
    fields = tuple(part.strip() for part in value.split(",") if part.strip())
    if not fields:
        raise argparse.ArgumentTypeError("--fields must not be empty")
    for field in fields:
        if field not in FIELDS:
            raise argparse.ArgumentTypeError(
                f"unknown field {field!r}; expected one of {FIELDS}"
            )
    return fields


def _default_window(offsets_doc: dict) -> tuple[int, int, int]:
    """分类用 Task 1 的 fit_window（a 侧可比帧），有效末帧单列。"""
    fit = offsets_doc.get("fit_window")
    if isinstance(fit, list) and len(fit) == 2:
        start, end = fit
    else:
        window = offsets_doc.get("window")
        if isinstance(window, list) and len(window) == 2:
            start, end = window
        elif isinstance(window, dict):
            start = window.get("start")
            end = window.get("end")
        else:
            raise ValueError("offsets JSON missing fit_window/window")
    effective = offsets_doc.get("effective_end", end)
    if isinstance(start, bool) or not isinstance(start, int):
        raise ValueError("offsets JSON fit window start must be int")
    if isinstance(end, bool) or not isinstance(end, int):
        raise ValueError("offsets JSON fit window end must be int")
    if isinstance(effective, bool) or not isinstance(effective, int):
        raise ValueError("offsets JSON effective_end must be int")
    return start, end, effective


def _add_provenance(path: str, seq: dict) -> dict:
    frames = sorted(seq)
    return {
        "path": path,
        "rows": len(seq),
        "last_f": frames[-1] if frames else None,
        "mtime": datetime.fromtimestamp(os.path.getmtime(path))
        .astimezone()
        .isoformat(timespec="seconds"),
    }


def _build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description="把三腿失配聚合成 phase/value episode（帧轴 = Mesen2）"
    )
    parser.add_argument("--bizhawk", required=True, help="biz 轨迹 JSONL")
    parser.add_argument("--mesen", required=True, help="mesen 轨迹 JSONL")
    parser.add_argument("--rust", required=True, help="rust 状态 JSONL")
    parser.add_argument("--offsets", required=True, help="Task 1 的 leg_offsets.json")
    parser.add_argument(
        "--json-out",
        default="data/status/leg_episodes.json",
        help="输出 JSON（默认 data/status/leg_episodes.json）",
    )
    parser.add_argument("--start", type=int, default=None, help="Mesen2 帧起点（>=20）")
    parser.add_argument("--end", type=int, default=None, help="Mesen2 帧终点")
    parser.add_argument(
        "--fields",
        type=_fields_arg,
        default=FIELDS,
        help=f"逗号分隔的字段，默认 {','.join(FIELDS)}",
    )
    parser.add_argument(
        "--load-key",
        default="room",
        help="加载窗判据字段（默认 room = $005A）",
    )
    return parser


def main(argv: list[str] | None = None) -> int:
    parser = _build_parser()
    args = parser.parse_args(argv)

    try:
        offsets_doc = json.loads(Path(args.offsets).read_text(encoding="utf-8"))
        raw_offsets = offsets_doc.get("offsets")
        if not isinstance(raw_offsets, dict):
            raise ValueError("offsets JSON missing object offsets")
        offsets = {}
        for leg in ("biz", "rust"):
            if leg in raw_offsets:
                offsets[leg] = raw_offsets[leg]
            elif f"{leg}_to_mesen" in raw_offsets:
                offsets[leg] = raw_offsets[f"{leg}_to_mesen"]
        start, end, effective_end = _default_window(offsets_doc)
        if args.start is not None:
            start = args.start
        if args.end is not None:
            end = args.end
        if start < DEFAULT_START:
            parser.error(
                f"--start must be >= {DEFAULT_START}（f0-f19 两核 RAM 填充不同）"
            )
        if start > end:
            raise ValueError(f"start {start} > end {end}")

        legs = {
            "biz": load_state(args.bizhawk, "biz", fields=args.fields),
            "mesen": load_state(args.mesen, "mesen", fields=args.fields),
            "rust": load_state(args.rust, "rust", fields=args.fields),
        }
        report = build_report(
            legs, offsets, args.fields, start, end, args.load_key
        )
        report["offsets"]["source"] = args.offsets
        report["window"]["fit_window"] = [start, end]
        report["window"]["effective_end"] = effective_end
        report["window"]["offsets_window"] = offsets_doc.get("window")
        report["provenance"] = {
            "biz": _add_provenance(args.bizhawk, legs["biz"]),
            "mesen": _add_provenance(args.mesen, legs["mesen"]),
            "rust": _add_provenance(args.rust, legs["rust"]),
        }

        out = Path(args.json_out)
        out.parent.mkdir(parents=True, exist_ok=True)
        out.write_text(
            json.dumps(report, ensure_ascii=False, indent=2) + "\n",
            encoding="utf-8",
        )

        summary = report["summary"]
        print(
            "EPISODES total={total} phase={phase} value={value} "
            "during_load={load}".format(
                total=summary["total_episodes"],
                phase=summary["by_class"]["phase"],
                value=summary["by_class"]["value"],
                load=summary["during_load"],
            )
        )
        print(
            "DENOMINATORS total={total} in={inside} load={load} missing={missing} "
            "union={union}".format(
                total=report["denominators"]["total_frames"],
                inside=report["denominators"]["in_denominator"],
                load=report["denominators"]["excluded"]["load"],
                missing=report["denominators"]["excluded"]["missing"],
                union=report["denominators"]["excluded"]["union"],
            )
        )
        for row in summary["value_episodes"]:
            print(
                "VALUE {leg} f{start}..{end} frames={frames} fields={fields}".format(
                    leg=row["leg"],
                    start=row["start"],
                    end=row["end"],
                    frames=row["frames"],
                    fields=",".join(row["fields"]),
                )
            )
        return 0
    except (ValueError, OSError, json.JSONDecodeError) as exc:
        print(f"ERROR: {exc}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
