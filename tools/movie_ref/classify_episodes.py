#!/usr/bin/env python3
"""Task 2：把三腿逐帧失配聚合成 episode，并三分类为 phase / transient / value。

帧轴：Mesen2。`offsets` 语义来自 Task 1（`a[f] == b[f + offset]`），因此 Mesen2
帧 g 对应另一腿的帧 `g - offset[leg]`。偏移**只从** `data/status/leg_offsets.json`
读，不在本文件里硬编码。

分类判据（第二轮的整段序列口径，取代首轮的逐帧查值）：
- phase：存在 k ∈ [-W, W]，使本腿整段的每个字段值序列等于 Mesen2 平移 k 后的
  序列；命中 k 写入 evidence.shift。W 由实测 p1→latch 滞后分布导出，不硬编码；
  W 的上界是 2（实测 max_abs=1，状态瞬态错位最多 1 帧）。
- transient：段 ≤ 2 帧，且对面 ±W 内恰好出现 1 帧不同的值（一帧尖峰）。
- value：找不到任何 k，且分歧持续 ≥ 3 帧，或该值在对面 ±5 内完全不存在。
- undecidable：字段在整报告窗内两边各自只有一个值且不相等（如恒定的 health_lo），
  不默认判成 phase。

`during_load`：段内任一帧任一条腿的 `$005A`（ROOM_NUMBER）!= 0xFF。

加载窗帧按计划 R7 不进分母，但**仍然**聚合成 episode 并打 `during_load` 标记；
denominators 显式给出总帧 / 分母 / 排除帧及原因（load / missing / 二者重叠），
不许静默丢。同分歧跨腿各记一次，shared_with 与 merged_episode_count 负责链接。
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import shutil
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
# W 的上界由实测滞后分布导出（见 build_report 的 phase_window 证据）；
# 分类逻辑只使用 phase_window["value"]，不把 2 当无来源常量。
PHASE_WINDOW = 2
PHASE_WINDOW_UPPER_BOUND = 2
TRANSIENT_MAX_FRAMES = 2
VALUE_MIN_SUSTAINED = 3
VALUE_NEAR_RADIUS = 5
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
    window: int,
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
            leg, legs, offsets, frames, load_key, current["fields"], window,
            start, end,
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


def _constant_mismatch(
    legs: dict[str, dict[int, dict]],
    leg: str,
    offsets: dict[str, int],
    start: int,
    end: int,
    field: str,
) -> bool:
    """整个比较窗内两侧都只取一个值且不相等 —— 常量字段，不做 phase/value 归因。"""
    leg_values: set[int] = set()
    mesen_values: set[int] = set()
    for g in range(start, end + 1):
        leg_row = legs[leg].get(g - offsets[leg])
        mesen_row = legs["mesen"].get(g)
        if (
            leg_row is None
            or mesen_row is None
            or field not in leg_row
            or field not in mesen_row
        ):
            return False
        leg_values.add(leg_row[field])
        mesen_values.add(mesen_row[field])
    return (
        len(leg_values) == 1
        and len(mesen_values) == 1
        and leg_values != mesen_values
    )


def _sequence_for_field(
    legs: dict[str, dict[int, dict]],
    leg: str,
    offsets: dict[str, int],
    start: int,
    end: int,
    field: str,
) -> tuple[list[int], list[int]] | None:
    """取整段（含匹配帧）的 leg 值序列与 Mesen2 值序列；缺帧/缺字段返回 None。"""
    leg_values: list[int] = []
    mesen_values: list[int] = []
    for g in range(start, end + 1):
        leg_row = legs[leg].get(g - offsets[leg])
        mesen_row = legs["mesen"].get(g)
        if (
            leg_row is None
            or mesen_row is None
            or field not in leg_row
            or field not in mesen_row
        ):
            return None
        leg_values.append(leg_row[field])
        mesen_values.append(mesen_row[field])
    return leg_values, mesen_values


def _match_shift(
    leg_values: list[int],
    mesen_rows: dict[int, dict],
    start: int,
    field: str,
    shift: int,
) -> bool:
    """整段逐帧比较：leg[g] == mesen[g + shift]；缺帧即不匹配。"""
    for i, g in enumerate(range(start, start + len(leg_values))):
        mesen_row = mesen_rows.get(g + shift)
        if mesen_row is None or mesen_row.get(field) != leg_values[i]:
            return False
    return True


def _choose_shift(
    leg_values: list[int],
    mesen_rows: dict[int, dict],
    start: int,
    field: str,
    window: int,
) -> int | None:
    matches = [
        shift
        for shift in range(-window, window + 1)
        if _match_shift(leg_values, mesen_rows, start, field, shift)
    ]
    if not matches:
        return None
    return min(matches, key=lambda shift: (abs(shift), shift))


def _single_spike(
    leg_values: list[int],
    mesen_rows: dict[int, dict],
    start: int,
    end: int,
    field: str,
    window: int,
) -> bool:
    """段窗内恰有 1 帧出现「本段值集合之外」的值（对面尖峰）。"""
    leg_set = set(leg_values)
    different = 0
    for g in range(start - window, end + window + 1):
        row = mesen_rows.get(g)
        if row is None or field not in row:
            continue
        if row[field] not in leg_set:
            different += 1
    return different == 1


def _value_within(
    mesen_rows: dict[int, dict],
    frame: int,
    field: str,
    value: int,
    radius: int,
) -> bool:
    for g in range(frame - radius, frame + radius + 1):
        row = mesen_rows.get(g)
        if row is not None and row.get(field) == value:
            return True
    return False


def _nearest_same_distance(
    mesen_rows: dict[int, dict],
    frame: int,
    field: str,
    value: int,
    max_distance: int = 50,
) -> int | None:
    for distance in range(max_distance + 1):
        for g in (frame - distance, frame + distance):
            row = mesen_rows.get(g)
            if row is not None and row.get(field) == value:
                return distance
    return None


def _build_evidence(
    leg: str,
    legs: dict[str, dict[int, dict]],
    offsets: dict[str, int],
    frames: list[int],
    load_key: str,
    episode_fields: list[str],
    window: int,
    scan_start: int,
    scan_end: int,
) -> dict:
    offset = offsets[leg]
    mesen_rows = legs["mesen"]
    start, end = frames[0], frames[-1]

    sequences: dict[str, tuple[list[int], list[int]]] = {}
    constant_fields: list[str] = []
    incomplete_fields: list[str] = []
    for field in episode_fields:
        if _constant_mismatch(
            legs, leg, offsets, scan_start, scan_end, field
        ):
            constant_fields.append(field)
            continue
        seq = _sequence_for_field(legs, leg, offsets, start, end, field)
        if seq is None:
            incomplete_fields.append(field)
            continue
        sequences[field] = seq

    # 1 帧/2 帧尖峰优先于相位：避免把采样错位洗成 phase 或 value。
    transient_fields: list[str] = []
    for field, (leg_values, _mesen_values) in sequences.items():
        if (end - start + 1) <= TRANSIENT_MAX_FRAMES and _single_spike(
            leg_values, mesen_rows, start, end, field, window
        ):
            transient_fields.append(field)

    shifts: dict[str, int] = {}
    if not transient_fields:
        for field, (leg_values, _mesen_values) in sequences.items():
            shift = _choose_shift(
                leg_values, mesen_rows, start, field, window
            )
            if shift is not None:
                shifts[field] = shift
    common_shift: int | None = None
    if shifts and len(shifts) == len(sequences) and len(set(shifts.values())) == 1:
        common_shift = next(iter(shifts.values()))
    phase_fields = list(sequences) if common_shift is not None else []

    value_fields: list[str] = []
    value_examples: list[dict] = []
    if not transient_fields and common_shift is None:
        for field, (leg_values, _mesen_values) in sequences.items():
            sustained = (end - start + 1) >= VALUE_MIN_SUSTAINED
            absent = False
            for g in frames:
                leg_row = legs[leg].get(g - offset)
                mesen_row = mesen_rows.get(g)
                if leg_row is None or mesen_row is None:
                    continue
                leg_value = leg_row.get(field)
                mesen_value = mesen_row.get(field)
                if leg_value == mesen_value:
                    continue
                if not _value_within(
                    mesen_rows, g, field, leg_value, VALUE_NEAR_RADIUS
                ):
                    absent = True
                    if len(value_examples) < 3:
                        value_examples.append(
                            {
                                "frame": g,
                                "field": field,
                                "leg_value": leg_value,
                                "opposite_values": [
                                    mesen_rows[gg].get(field)
                                    for gg in range(
                                        g - window, g + window + 1
                                    )
                                    if gg in mesen_rows
                                ],
                            }
                        )
                    break
            if sustained or absent:
                value_fields.append(field)

    undecidable_fields = sorted(
        set(constant_fields)
        | set(incomplete_fields)
        | (
            set(sequences)
            - set(phase_fields)
            - set(transient_fields)
            - set(value_fields)
        )
    )
    during = _during_load_frames(legs, offsets, start, end, load_key)
    return {
        "rule": (
            "phase iff whole segment equals mesen shifted by one common k in "
            f"[-{window},{window}]; transient iff <=2 frames and mesen window has "
            "exactly one value outside the leg segment values; value iff no common "
            "k and (sustained>=3 frames or leg value absent in mesen +-5); "
            "otherwise undecidable"
        ),
        "phase_window": window,
        "mesen_frame_window": [start, end],
        "leg_frame_window": [start - offset, end - offset],
        "leg_offset": offset,
        "mismatch_frames": frames,
        "shift": common_shift,
        "field_shifts": shifts,
        "phase_fields": phase_fields,
        "transient": bool(transient_fields),
        "transient_fields": transient_fields,
        "value_fields": value_fields,
        "undecidable_fields": undecidable_fields,
        "value_examples": value_examples,
        "during_load_frames": during,
    }


def _episode_class(evidence: dict) -> str:
    if evidence["transient"]:
        return "transient"
    if evidence["phase_fields"]:
        return "phase"
    if evidence["value_fields"]:
        return "value"
    return "undecidable"


def classify(
    legs: dict[str, dict[int, dict]],
    offsets: dict[str, int],
    fields: tuple[str, ...],
    start: int,
    end: int,
    load_key: str = "room",
    window: int = PHASE_WINDOW,
) -> list[dict]:
    """把三腿逐帧失配聚合成 episode。返回按 (start, leg) 排序的段列表。

    legs 必须含 mesen；另含 biz/rust。offsets 可传 {"biz":1,...} 或
    {"biz_to_mesen":1,...}。每个 episode 同时带 `class` 与 `kind`（同值）。
    window 由 build_report 的 phase_window 证据传入，不由本函数硬编码。
    """
    if start > end:
        raise ValueError(f"start {start} > end {end}")
    if "mesen" not in legs:
        raise ValueError("legs must include mesen")
    if window < 0:
        raise ValueError(f"window must be >= 0, got {window}")
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
                    leg, legs, normalized, fields, start, end, load_key, window
                )
            )
    episodes.sort(key=lambda e: (e["start"], e["leg"]))
    _link_shared_episodes(episodes)
    return episodes


def _link_shared_episodes(episodes: list[dict]) -> None:
    """同一跨腿分歧在 biz/rust 各记一次时，用 shared_with 互相链接，不丢行。"""
    for episode in episodes:
        episode["shared_with"] = [
            {
                "leg": other["leg"],
                "start": other["start"],
                "end": other["end"],
                "fields": other["fields"],
            }
            for other in episodes
            if other["leg"] != episode["leg"]
            and not (
                other["end"] < episode["start"] or other["start"] > episode["end"]
            )
        ]


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
    missing_within_load = missing_frames & load_frames
    common_in_denominator = total - len(excluded_union)
    return {
        "total_frames": total,
        "in_denominator": common_in_denominator,
        "common_in_denominator": common_in_denominator,
        "excluded": {
            "load": len(load_frames),
            "missing": len(missing_frames),
            "missing_within_load": len(missing_within_load),
            "missing_outside_load": len(missing_frames - load_frames),
            "union": len(excluded_union),
            "load_frames": sorted(load_frames),
            "missing_frames": sorted(missing_frames),
            "reasons": {
                "load": f"任一侧 ${0x005A:04X} != 0xFF（ROOM_NUMBER，加载窗；计划 R7）",
                "missing": "该 Mesen2 帧或该腿映射帧缺失（状态行缝），无可比样本",
                "missing_within_load": "missing 与 load 的重叠，单列避免 union==load 的误读",
            },
        },
        "by_leg": per_leg,
    }


def _merged_episode_count(episodes: list[dict]) -> int:
    """跨腿合并：帧区间重叠且字段有交集的两个 leg 段视为同一分歧。"""
    parent = list(range(len(episodes)))

    def find(index: int) -> int:
        while parent[index] != index:
            parent[index] = parent[parent[index]]
            index = parent[index]
        return index

    def union(left: int, right: int) -> None:
        left, right = find(left), find(right)
        if left != right:
            parent[right] = left

    for i, left in enumerate(episodes):
        for j in range(i + 1, len(episodes)):
            right = episodes[j]
            if left["leg"] == right["leg"]:
                continue
            if left["end"] < right["start"] or right["end"] < left["start"]:
                continue
            if set(left["fields"]) & set(right["fields"]):
                union(i, j)
    return len({find(index) for index in range(len(episodes))})


def build_report(
    legs: dict[str, dict[int, dict]],
    offsets: dict[str, int],
    fields: tuple[str, ...],
    start: int,
    end: int,
    load_key: str = "room",
    window: int = PHASE_WINDOW,
    phase_window: dict | None = None,
) -> dict:
    """分类 + 分母 + 摘要，不读文件；CLI 负责 provenance。"""
    normalized = _normalize_offsets(offsets)
    episodes = classify(legs, normalized, fields, start, end, load_key, window)
    denominators = _denominators(legs, normalized, start, end, load_key)
    if phase_window is None:
        phase_window = {
            "value": window,
            "upper_bound": PHASE_WINDOW_UPPER_BOUND,
            "derivation": "default: upper bound 2; CLI supplies measured lag distribution",
        }

    classes = ("phase", "transient", "value", "undecidable")
    by_class = {name: 0 for name in classes}
    by_leg: dict[str, int] = {}
    by_leg_class: dict[str, dict[str, int]] = {}
    by_leg_load: dict[str, dict[str, int]] = {}
    for episode in episodes:
        leg = episode["leg"]
        cls = episode["class"]
        by_class[cls] += 1
        by_leg[leg] = by_leg.get(leg, 0) + 1
        slot = by_leg_class.setdefault(leg, {name: 0 for name in classes})
        slot[cls] += 1
        load_slot = by_leg_load.setdefault(leg, {"during_load": 0, "clean": 0})
        key = "during_load" if episode["during_load"] else "clean"
        load_slot[key] += 1

    def rows(kind: str, limit: int = 3) -> list[dict]:
        out = []
        for episode in [e for e in episodes if e["class"] == kind][:limit]:
            out.append(
                {
                    "leg": episode["leg"],
                    "start": episode["start"],
                    "end": episode["end"],
                    "frames": episode["frames"],
                    "fields": episode["fields"],
                    "shift": episode["evidence"].get("shift"),
                    "transient": episode["class"] == "transient",
                    "example": episode["evidence"]["value_examples"][:1],
                }
            )
        return out

    merged_count = _merged_episode_count(episodes)
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
        "comparison": {
            "axis": "mesen",
            "pairs": [["biz", "mesen"], ["rust", "mesen"]],
            "symmetry": "每个非 mesen 腿都对 mesen 检查；shared_with 链接同分歧跨腿重复",
        },
        "summary": {
            "total_episodes": len(episodes),
            "by_class": by_class,
            "by_leg": by_leg,
            "by_leg_class": by_leg_class,
            "by_leg_load": by_leg_load,
            "during_load": sum(1 for e in episodes if e["during_load"]),
            "clean": sum(1 for e in episodes if not e["during_load"]),
            "phase_window": phase_window,
            "phase_count": by_class["phase"],
            "transient_count": by_class["transient"],
            "value_count": by_class["value"],
            "undecidable_count": by_class["undecidable"],
            "merged_episode_count": merged_count,
            "phase_episodes": rows("phase"),
            "transient_episodes": rows("transient"),
            "value_episodes": rows("value"),
            "undecidable_episodes": rows("undecidable"),
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


def _sha256(path: str) -> str:
    digest = hashlib.sha256()
    with open(path, "rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def _lag_histogram(path: str, start: int, end: int) -> dict:
    """实测 p1 -> latch 的最近变化间隔分布（只用于导出 W，不参与判定）。"""
    rows: list[dict] = []
    with open(path, encoding="utf-8") as handle:
        for line in handle:
            line = line.strip()
            if not line:
                continue
            row = json.loads(line)
            if start <= row.get("f", -1) <= end:
                rows.append(row)
    p1_changes: list[int] = []
    latch_changes: list[int] = []
    for previous, current in zip(rows, rows[1:]):
        if current.get("p1") != previous.get("p1"):
            p1_changes.append(current["f"])
        if current.get("joy1_latch") != previous.get("joy1_latch"):
            latch_changes.append(current["f"])
    forward: dict[str, int] = {}
    backward: dict[str, int] = {}
    for frame in p1_changes:
        if latch_changes:
            nearest = min(latch_changes, key=lambda g: (abs(g - frame), g))
            forward[str(nearest - frame)] = forward.get(str(nearest - frame), 0) + 1
    for frame in latch_changes:
        if p1_changes:
            nearest = min(p1_changes, key=lambda g: (abs(g - frame), g))
            backward[str(nearest - frame)] = backward.get(str(nearest - frame), 0) + 1
    return {
        "p1_to_latch": forward,
        "latch_to_p1": backward,
        "p1_changes": len(p1_changes),
        "latch_changes": len(latch_changes),
        "max_abs": max(
            [abs(int(key)) for key in list(forward) + list(backward)] or [0]
        ),
    }


def _nearest_equal_distance(
    legs: dict[str, dict[int, dict]],
    offsets: dict[str, int],
    episodes: list[dict],
) -> dict:
    """失配帧/字段到对面最近同值帧的距离分位数（给 W 的实测依据）。"""
    distances: list[int] = []
    for episode in episodes:
        if episode["class"] in ("value", "undecidable"):
            continue
        offset = offsets[episode["leg"]]
        for g in episode["evidence"]["mismatch_frames"]:
            for field in episode["fields"]:
                leg_row = legs[episode["leg"]].get(g - offset)
                mesen_row = legs["mesen"].get(g)
                if leg_row is None or mesen_row is None:
                    continue
                leg_value = leg_row.get(field)
                if leg_value == mesen_row.get(field):
                    continue
                for distance in range(0, 51):
                    found = False
                    for gg in (g - distance, g + distance):
                        row = legs["mesen"].get(gg)
                        if row is not None and row.get(field) == leg_value:
                            found = True
                            break
                    if found:
                        distances.append(distance)
                        break
    distances.sort()

    def quantile(q: float) -> int | None:
        if not distances:
            return None
        index = min(len(distances) - 1, max(0, int(q * len(distances))))
        return distances[index]

    return {
        "count": len(distances),
        "q50": quantile(0.5),
        "q90": quantile(0.9),
        "q95": quantile(0.95),
        "q99": quantile(0.99),
        "max": distances[-1] if distances else None,
        "within_2_rate": (
            sum(1 for d in distances if d <= 2) / len(distances)
            if distances else None
        ),
        "within_5_rate": (
            sum(1 for d in distances if d <= 5) / len(distances)
            if distances else None
        ),
    }


def _phase_window_provenance(
    legs: dict[str, dict[int, dict]],
    offsets: dict[str, int],
    episodes: list[dict],
    histogram: dict,
) -> dict:
    measured_max = histogram.get("max_abs", 0)
    upper = max(PHASE_WINDOW_UPPER_BOUND, measured_max)
    return {
        "value": upper,
        "upper_bound": upper,
        "derivation": (
            "W derived from measured p1->latch / latch->p1 lag distribution "
            f"(max_abs={measured_max}); state transient displacement <=1; "
            f"upper bound {upper}"
        ),
        "lag_histogram": histogram,
        "nearest_equal_distance": _nearest_equal_distance(
            legs, offsets, episodes
        ),
    }


def _add_provenance(path: str, seq: dict, archive: bool = False) -> dict:
    frames = sorted(seq)
    out = {
        "path": path,
        "rows": len(seq),
        "last_f": frames[-1] if frames else None,
        "mtime": datetime.fromtimestamp(os.path.getmtime(path))
        .astimezone()
        .isoformat(timespec="seconds"),
        "sha256": _sha256(path),
    }
    if archive:
        archive_dir = Path("analysis/traces/.archive")
        archive_dir.mkdir(parents=True, exist_ok=True)
        archive_path = archive_dir / "rust-states-t2.jsonl"
        if Path(path).resolve() != archive_path.resolve():
            shutil.copy2(path, archive_path)
        out["archive_path"] = str(archive_path)
        out["archive_sha256"] = _sha256(str(archive_path))
    return out


def _build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description="把三腿失配聚合成 phase/transient/value/undecidable episode（帧轴 = Mesen2）"
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
        episodes_for_window = classify(
            legs, offsets, args.fields, start, end, args.load_key
        )
        histogram = _lag_histogram(args.bizhawk, start, end)
        phase_window = _phase_window_provenance(
            legs, offsets, episodes_for_window, histogram
        )
        report = build_report(
            legs, offsets, args.fields, start, end, args.load_key,
            window=phase_window["value"], phase_window=phase_window,
        )
        report["offsets"]["source"] = args.offsets
        report["window"]["fit_window"] = [start, end]
        report["window"]["effective_end"] = effective_end
        report["window"]["offsets_window"] = offsets_doc.get("window")
        report["provenance"] = {
            "biz": _add_provenance(args.bizhawk, legs["biz"]),
            "mesen": _add_provenance(args.mesen, legs["mesen"]),
            "rust": _add_provenance(args.rust, legs["rust"], archive=True),
            "phase_window": phase_window,
        }

        out = Path(args.json_out)
        out.parent.mkdir(parents=True, exist_ok=True)
        out.write_text(
            json.dumps(report, ensure_ascii=False, indent=2) + "\n",
            encoding="utf-8",
        )

        summary = report["summary"]
        classes = summary["by_class"]
        print(
            "EPISODES total={total} phase={phase} transient={transient} "
            "value={value} undecidable={undecidable} during_load={load}".format(
                total=summary["total_episodes"],
                phase=classes["phase"],
                transient=classes["transient"],
                value=classes["value"],
                undecidable=classes["undecidable"],
                load=summary["during_load"],
            )
        )
        excluded = report["denominators"]["excluded"]
        by_leg = report["denominators"]["by_leg"]
        print(
            "DENOMINATORS total={total} common_in={inside} "
            "(load {load} union missing {missing}, overlap {overlap}) "
            "biz_in={biz} rust_in={rust}".format(
                total=report["denominators"]["total_frames"],
                inside=report["denominators"]["in_denominator"],
                load=excluded["load"],
                missing=excluded["missing"],
                overlap=excluded["missing_within_load"],
                biz=by_leg.get("biz", {}).get("in_denominator"),
                rust=by_leg.get("rust", {}).get("in_denominator"),
            )
        )
        print(
            "PHASE_WINDOW value={value} upper_bound={upper} lag={lag}".format(
                value=summary["phase_window"]["value"],
                upper=summary["phase_window"]["upper_bound"],
                lag=summary["phase_window"]["lag_histogram"],
            )
        )
        for kind in ("phase", "transient", "value", "undecidable"):
            for row in summary[f"{kind}_episodes"]:
                print(
                    "{kind} {leg} f{start}..{end} frames={frames} "
                    "fields={fields} shift={shift}".format(
                        kind=kind.upper(),
                        leg=row["leg"],
                        start=row["start"],
                        end=row["end"],
                        frames=row["frames"],
                        fields=",".join(row["fields"]),
                        shift=row.get("shift"),
                    )
                )
        return 0
    except (ValueError, OSError, json.JSONDecodeError) as exc:
        print(f"ERROR: {exc}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
