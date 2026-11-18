#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""tools/movie_ref/align_legs.py —— 三腿帧映射器（计划 Task 1；2026-11-18 复核更正）。

三条腿的帧号语义不同（谁在哪一帧"收工"不同）。本工具不再用"四字段打分最优"
猜偏移，而是用**状态面**（$0300 SamusStatus 与 $005A RoomNumber）的同步值跳变
机械反解：

    a[f] == b[f + offset]

即 offset=+1 表示 b 腿比 a 腿晚一帧才看到同一个状态值。

为什么是状态面而不是 $0014：
- 2026-11-18 独立复核（docs/rounds/r9-task-1-review.md）证明 $0014 输入锁存面
  的最优点恒比状态面晚一行：latch_offset == state_offset + 1（biz 状态 +1/锁存 +2；
  rust 状态 0/锁存 +1；四组配对一致）。把锁存偏移当帧偏移会让 biz 多 8 个失配帧、
  rust 多 53 个，Task 2 的 phase/value 分类会被假相位淹掉。
- 因此本工具的主 `offsets` 只用状态面（status/room 共识）；`$0014` 单列
  `latch_offsets`，并做 `latch = state + 1` 自校验（不成立即 ValueError）。
- 地址来源（本工具是唯一允许硬编码这些地址的 tools/ 文件）：
  - $0014：计划/任务称 JOY1_LATCH；crates/game-core/src/ram_map.rs:70 的具名常量
    实际叫 JOY1_STATUS（doc comment "Joy1Status=$14 … all buttons currently being
    pressed"）。本工具按 $0014 的字节语义读取，不另造地址。
  - $005A：ROOM_NUMBER（crates/game-core/src/ram_map.rs:216）。
  - $0300：SAMUS_STATUS（crates/game-core/src/ram_map.rs:223）。
  - $0100-$07FF：IRAM 窗口；$0000-$00FF：ZP 窗口。
  - $0106 SAMUS_HEALTH（ram_map.rs:169）本工具不消费；保留给 Task 2 的
    classify_episodes.py。

判据（计划 Task 1 Interfaces + Step 3，2026-11-18 复核后的双计数口径）：
- `align()` 与锁存面：只在 a 的"值变化帧"（a[f] != a[f-1] 且 f-1 也在 a 内）
  上比较；对 offset ∈ [-max_lag, +max_lag] 计一致数，取最大且唯一；
- 状态面（status/room）：对 **全帧** 计 a[f] == b[f+offset] 的一致数。状态面
  大部分帧不变，只取变化帧会样本太少（复核的 1096/1180、1158/1180 就是全帧口径）；
- change_frames = 裸变化帧数；change_runs = 变化帧的最大连续段数（一次 1 帧按键
  的 press+revert 算 1 段，而不是 2 个变化帧）；状态面 per_field 也带这两个计数；
- 拒绝阈值用 change_runs < 3（不是裸帧数）；这与计划 Step 1 的
  `align(a, b) == (1, 4)`（4 次按键 → 8 个变化帧 / 4 段）一致；
- 多解 → ValueError("ambiguous offset")；不足 → ValueError("insufficient anchor changes")。
  绝不挑一个偏移。

CLI 失败口径：
- Mesen2 行缺 w["Joy1Latch"]、Rust 行缺 zp/iram、业务行缺字段、JSON 坏行、
  重复帧、空轨迹：fail-loud（抛 ValueError / 非零退出），不静默跳过；
- 轨迹最后一行 f < --end：stderr 警告"轨迹可能被截断"，仍按已有帧出结论，
  但结论窗口只覆盖到实际最后一帧；
- 多解/锚不足/字段分歧/锁存-状态间隔异常：写出对应 verdict 的 JSON，
  offsets/latch_offsets 的失败腿写 null，不伪造整数偏移；
- 只有 verdict="aligned" 时返回 0，其余返回 1。

用法（计划 Task 1 Step 5）：
    python3 tools/movie_ref/align_legs.py \
        --bizhawk analysis/traces/bizhawk-6725M-post.jsonl \
        --mesen analysis/traces/_probe-axis.frames.jsonl \
        --rust /tmp/rust-states-t2.jsonl \
        --out data/status/leg_offsets.json [--start 20] [--end 30000]
"""

from __future__ import annotations

import argparse
import json
import os
import sys
from datetime import datetime
from pathlib import Path

# $0014 = JOY1_LATCH（ram_map.rs:70 实名 JOY1_STATUS）。
ANCHOR_ADDR = 0x0014
ANCHOR_ADDR_NAME = "$0014"
# $005A = ROOM_NUMBER（ram_map.rs:216）。
ROOM_ADDR = 0x005A
# $0300 = SAMUS_STATUS（ram_map.rs:223）。
STATUS_ADDR = 0x0300
# $0106/$0107 = SAMUS_HEALTH 低/高字节（ram_map.rs:169 SAMUS_HEALTH = 0x0106）。
HEALTH_LO_ADDR = 0x0106
HEALTH_HI_ADDR = 0x0107
# $0049 = SCROLL_DIR（ram_map.rs:59）。
SCROLL_DIR_ADDR = 0x0049
# $005A = ROOM_NUMBER（ram_map.rs:216）；分类器用它判加载窗。
FIELD_ADDRS = {
    "status": STATUS_ADDR,
    "room": ROOM_ADDR,
    "health_lo": HEALTH_LO_ADDR,
    "health_hi": HEALTH_HI_ADDR,
    "scrolldir": SCROLL_DIR_ADDR,
}
# Rust iram 覆盖 $0100-$07FF；zp 覆盖 $0000-$00FF。
IRAM_BASE = 0x0100
IRAM_END = 0x0800
ZP_BYTES = 0x100
# Mesen2 全 RAM range 的键名（$0000-$00FF / $0100-$07FF）。
MESEN_RAM_LOW = "0000-00ff"
MESEN_RAM_HIGH = "0100-07ff"

LEGS = ("biz", "mesen", "rust")
STATE_FIELDS = ("status", "room")
MIN_ANCHOR_CHANGES = 3
MIN_WINDOW_START = 20  # f0-f19 两核 RAM 填充不同（0xFF vs 0x00），对齐不可信。
DEFAULT_MAX_LAG = 4


class AlignmentError(ValueError):
    """可带 verdict 的对齐错误；CLI 用它把失败原因写进 JSON。"""

    def __init__(self, message: str, verdict: str = "insufficient"):
        super().__init__(message)
        self.verdict = verdict


def _change_frames(seq: dict[int, int]) -> list[int]:
    """a 的"值变化帧"：f-1 也在 a 内且 a[f] != a[f-1]。"""
    return [f for f in sorted(seq) if (f - 1) in seq and seq[f] != seq[f - 1]]


def _count_runs(frames: list[int]) -> int:
    """最大连续段数（输入跳变次数）。frames 必须升序。"""
    if not frames:
        return 0
    runs = 1
    for prev, cur in zip(frames, frames[1:]):
        if cur != prev + 1:
            runs += 1
    return runs


def _score(
    seq_a: dict[int, int],
    seq_b: dict[int, int],
    max_lag: int = DEFAULT_MAX_LAG,
) -> dict:
    """对 a 的变化帧逐 offset 打分；返回原始变化帧数/段数与每个 offset 的明细。"""
    if max_lag < 0:
        raise ValueError("max_lag must be >= 0")

    changes = _change_frames(seq_a)
    scored: list[dict[str, int]] = []
    for offset in range(-max_lag, max_lag + 1):
        participants = [f for f in changes if (f + offset) in seq_b]
        matches = sum(1 for f in participants if seq_a[f] == seq_b[f + offset])
        scored.append(
            {
                "offset": offset,
                "matches": matches,
                "participants": len(participants),
            }
        )

    best = max((row["matches"] for row in scored), default=0)
    winners = [row for row in scored if row["matches"] == best]
    return {
        "changes": changes,
        "change_frames": len(changes),
        "change_runs": _count_runs(changes),
        "rows": scored,
        "best": best,
        "winners": winners,
    }


def _allframe_score(
    seq_a: dict[int, int],
    seq_b: dict[int, int],
    max_lag: int = DEFAULT_MAX_LAG,
) -> dict:
    """对 a 的**全部帧**逐 offset 计 a[f] == b[f+offset] 的一致数/一致率。

    这是状态面（status/room）的口径：它们在大部分帧不变，只取变化帧会样本太少
    （2026-11-18 复核的 1096/1180、1158/1180 等就是全帧口径）。锁存面 offset
    仍用 _score 的变化帧口径；两者不可混用。排序按 agree_rate 而非原始 matches，
    避免两端 offset 因 participants 少而被 raw matches 系统性压低。
    """
    if max_lag < 0:
        raise ValueError("max_lag must be >= 0")

    scored: list[dict] = []
    for offset in range(-max_lag, max_lag + 1):
        participants = [f for f in seq_a if (f + offset) in seq_b]
        matches = sum(1 for f in participants if seq_a[f] == seq_b[f + offset])
        agree_rate = matches / len(participants) if participants else None
        scored.append(
            {
                "offset": offset,
                "matches": matches,
                "participants": len(participants),
                "agree_rate": agree_rate,
            }
        )

    # 按一致率排序，不按原始 matches：offset 在两端时 participants 会少，
    # 用 raw matches 排序会系统性偏向"能覆盖更多帧"的 offset（2026-11-18 复核 finding 4）。
    best_rate = max(
        (row["agree_rate"] for row in scored if row["agree_rate"] is not None),
        default=None,
    )
    winners = [
        row
        for row in scored
        if best_rate is not None and row["agree_rate"] == best_rate
    ]
    best = max((row["matches"] for row in winners), default=0)
    return {
        "rows": scored,
        "best": best,
        "best_rate": best_rate,
        "winners": winners,
    }


def _fit(
    seq_a: dict[int, int],
    seq_b: dict[int, int],
    max_lag: int = DEFAULT_MAX_LAG,
) -> tuple[int, int, int, int]:
    """返回 (offset, change_runs, matches, participants)。

    change_runs 是 a 侧全部变化帧的最大连续段数；participants 是该 offset 下
    能落到 b 的变化帧数。二者分开，避免把 b 的覆盖差异当成跳变次数。
    """
    scored = _score(seq_a, seq_b, max_lag)
    if not scored["changes"]:
        raise ValueError("insufficient anchor changes")
    if scored["change_runs"] < MIN_ANCHOR_CHANGES:
        raise ValueError("insufficient anchor changes")
    if scored["best"] < MIN_ANCHOR_CHANGES:
        raise ValueError("insufficient anchor matches")
    if len(scored["winners"]) != 1:
        raise ValueError("ambiguous offset")

    winner = scored["winners"][0]
    return (
        winner["offset"],
        scored["change_runs"],
        winner["matches"],
        winner["participants"],
    )


def align(
    seq_a: dict[int, int],
    seq_b: dict[int, int],
    max_lag: int = DEFAULT_MAX_LAG,
) -> tuple[int, int]:
    """反解 a→b 的整数帧偏移，语义 a[f] == b[f + offset]。

    返回 (offset, changes)。changes 是 a 侧变化帧的最大连续段数（输入跳变次数）；
    裸变化帧数见 CLI JSON 的 change_frames。
    - 锚不足：ValueError("insufficient anchor changes")
    - 候选一致数不足：ValueError("insufficient anchor matches")
    - 多解：ValueError("ambiguous offset")
    """
    offset, changes, _matches, _participants = _fit(seq_a, seq_b, max_lag)
    return offset, changes


def _byte_value(value: object, path: str, lineno: int, label: str) -> int:
    if isinstance(value, bool) or not isinstance(value, int) or not 0 <= value <= 0xFF:
        raise ValueError(
            f"{path}:{lineno}: {label} must be an int in 0..255, got {value!r}"
        )
    return value


def _hex_byte(
    hexstr: object,
    offset: int,
    path: str,
    lineno: int,
    label: str,
    expected_len: int | None = None,
) -> int:
    """从扁平 hex 串取第 offset 个字节（offset 是 RAM 地址的低位偏移）。"""
    if not isinstance(hexstr, str):
        raise ValueError(f"{path}:{lineno}: {label} must be a hex string")
    if expected_len is not None and len(hexstr) != expected_len:
        raise ValueError(
            f"{path}:{lineno}: {label} must be {expected_len} hex chars, "
            f"got {len(hexstr)}"
        )
    start = 2 * offset
    if len(hexstr) < start + 2:
        raise ValueError(
            f"{path}:{lineno}: {label} too short for byte {offset:#04x} "
            f"(need >= {start + 2} hex chars, got {len(hexstr)})"
        )
    chunk = hexstr[start:start + 2]
    try:
        return int(chunk, 16)
    except ValueError as exc:
        raise ValueError(
            f"{path}:{lineno}: {label} has non-hex byte at {offset:#04x}: {chunk!r}"
        ) from exc


def _extract_latch(row: dict, leg: str, path: str, lineno: int) -> int:
    if leg == "biz":
        if "joy1_latch" not in row:
            raise ValueError(f"{path}:{lineno}: biz row missing joy1_latch")
        return _byte_value(row["joy1_latch"], path, lineno, "joy1_latch")

    w = row.get("w")
    if not isinstance(w, dict):
        raise ValueError(f"{path}:{lineno}: {leg} row missing object w")

    if leg == "mesen":
        if "Joy1Latch" not in w:
            # 明确拒绝从全 RAM range 里"猜"同名地址：缺 watch 就是缺证据。
            raise ValueError(
                f"{path}:{lineno}: mesen row missing w['Joy1Latch'] "
                "(Mesen2 leg must watch $0014; refusing to guess from a RAM range)"
            )
        return _byte_value(w["Joy1Latch"], path, lineno, "w['Joy1Latch']")

    # rust：zp 是扁平 hex 串，$0014 是第 0x14 个字节。
    zp = w.get("zp")
    if not isinstance(zp, str):
        raise ValueError(f"{path}:{lineno}: rust row missing string w['zp']")
    return _hex_byte(zp, ANCHOR_ADDR, path, lineno, "rust w['zp'] $0014")


def _extract_latch_optional(
    row: dict, leg: str, path: str, lineno: int
) -> int | None:
    """读状态面时顺带取 $0014；缺键返回 None，不阻断状态面（finding 2）。

    真正只算锁存面时仍走 _extract_latch：缺键/坏值一律 fail-loud。
    """
    if leg == "biz":
        if "joy1_latch" not in row:
            return None
        return _byte_value(row["joy1_latch"], path, lineno, "joy1_latch")

    w = row.get("w")
    if not isinstance(w, dict):
        raise ValueError(f"{path}:{lineno}: {leg} row missing object w")

    if leg == "mesen":
        if "Joy1Latch" not in w:
            return None
        return _byte_value(w["Joy1Latch"], path, lineno, "w['Joy1Latch']")

    zp = w.get("zp")
    if not isinstance(zp, str):
        return None
    return _hex_byte(zp, ANCHOR_ADDR, path, lineno, "rust w['zp'] $0014")


def _mesen_range_byte(
    w: dict,
    range_key: str,
    offset: int,
    field: str,
    path: str,
    lineno: int,
) -> int:
    """从 Mesen2 全 RAM range 的固定偏移取一个字节。"""
    rng = w.get(range_key)
    if not isinstance(rng, str):
        raise ValueError(
            f"{path}:{lineno}: mesen row missing w[{range_key!r}] "
            f"for state field {field}"
        )
    expected = (
        2 * ZP_BYTES
        if range_key == MESEN_RAM_LOW
        else 2 * (IRAM_END - IRAM_BASE)
    )
    return _hex_byte(
        rng,
        offset,
        path,
        lineno,
        f"mesen {field} via w[{range_key!r}]",
        expected_len=expected,
    )


def _mesen_state_byte(
    w: dict,
    name: str,
    range_key: str,
    offset: int,
    field: str,
    path: str,
    lineno: int,
) -> int:
    """Mesen2 状态面字节：优先命名 watch，缺则从全 RAM range 的固定偏移取。"""
    if name in w:
        return _byte_value(w[name], path, lineno, f"w[{name!r}]")
    return _mesen_range_byte(w, range_key, offset, field, path, lineno)


def _extract_state_fields(
    row: dict,
    leg: str,
    path: str,
    lineno: int,
    fields: tuple[str, ...],
) -> dict[str, int | None]:
    """读一行的状态面 + 锁存面；fields 为请求的字段名。"""
    for field in fields:
        if field not in FIELD_ADDRS:
            raise ValueError(
                f"{path}:{lineno}: unknown state field {field!r}; "
                f"expected one of {tuple(FIELD_ADDRS)}"
            )

    if leg == "biz":
        out: dict[str, int | None] = {}
        for field in fields:
            if field not in row:
                raise ValueError(f"{path}:{lineno}: biz row missing {field}")
            out[field] = _byte_value(row[field], path, lineno, field)
        out["latch"] = _extract_latch_optional(row, leg, path, lineno)
        return out

    w = row.get("w")
    if not isinstance(w, dict):
        raise ValueError(f"{path}:{lineno}: {leg} row missing object w")

    if leg == "mesen":
        out = {}
        for field in fields:
            if field == "status":
                out[field] = _mesen_state_byte(
                    w, "SamusStatus", MESEN_RAM_HIGH,
                    STATUS_ADDR - IRAM_BASE, field, path, lineno,
                )
            elif field == "room":
                out[field] = _mesen_state_byte(
                    w, "RoomNumber", MESEN_RAM_LOW,
                    ROOM_ADDR, field, path, lineno,
                )
            else:
                addr = FIELD_ADDRS[field]
                if addr < IRAM_BASE:
                    out[field] = _mesen_range_byte(
                        w, MESEN_RAM_LOW, addr, field, path, lineno
                    )
                else:
                    out[field] = _mesen_range_byte(
                        w, MESEN_RAM_HIGH, addr - IRAM_BASE, field, path, lineno
                    )
        out["latch"] = _extract_latch_optional(row, leg, path, lineno)
        return out

    # rust：zp 覆盖 $0000-$00FF，iram 覆盖 $0100-$07FF；两段都必须是精确长度。
    zp = w.get("zp")
    iram = w.get("iram")
    if not isinstance(zp, str):
        raise ValueError(f"{path}:{lineno}: rust row missing string w['zp']")
    if not isinstance(iram, str):
        raise ValueError(f"{path}:{lineno}: rust row missing string w['iram']")
    if len(zp) != 2 * ZP_BYTES:
        raise ValueError(
            f"{path}:{lineno}: rust w['zp'] must be {2 * ZP_BYTES} hex chars, "
            f"got {len(zp)}"
        )
    if len(iram) != 2 * (IRAM_END - IRAM_BASE):
        raise ValueError(
            f"{path}:{lineno}: rust w['iram'] must be "
            f"{2 * (IRAM_END - IRAM_BASE)} hex chars, got {len(iram)}"
        )
    out = {}
    for field in fields:
        addr = FIELD_ADDRS[field]
        if addr < IRAM_BASE:
            out[field] = _hex_byte(
                zp, addr, path, lineno, f"rust w['zp'] ${addr:04X}"
            )
        else:
            out[field] = _hex_byte(
                iram, addr - IRAM_BASE, path, lineno,
                f"rust w['iram'] ${addr:04X}",
            )
    out["latch"] = _extract_latch_optional(row, leg, path, lineno)
    return out


def _extract_state(
    row: dict, leg: str, path: str, lineno: int
) -> dict[str, int | None]:
    """读一行的状态面 + 锁存面：{status, room, latch}；latch 可缺失为 None。"""
    return _extract_state_fields(row, leg, path, lineno, STATE_FIELDS)


def _read_leg(path: str, leg: str, extractor) -> tuple[dict, int, int]:
    """流式读一条腿，返回 (frame -> 提取值, 最后一行 f, 数据行数)。"""
    if leg not in LEGS:
        raise ValueError(f"unknown leg {leg!r}; expected one of {LEGS}")

    seq: dict = {}
    last_f: int | None = None
    rows = 0
    with open(path, "r", encoding="utf-8") as fh:
        for lineno, raw in enumerate(fh, 1):
            line = raw.strip()
            if not line:
                continue
            try:
                row = json.loads(line)
            except json.JSONDecodeError as exc:
                raise ValueError(f"{path}:{lineno}: invalid JSON: {exc}") from exc
            if not isinstance(row, dict):
                raise ValueError(f"{path}:{lineno}: row is not a JSON object")
            if "f" not in row:
                raise ValueError(f"{path}:{lineno}: row missing f")
            f = row["f"]
            if isinstance(f, bool) or not isinstance(f, int):
                raise ValueError(f"{path}:{lineno}: f must be an int, got {f!r}")
            if f in seq:
                raise ValueError(f"{path}:{lineno}: duplicate frame f={f}")
            seq[f] = extractor(row, leg, path, lineno)
            last_f = f
            rows += 1
    if last_f is None:
        raise ValueError(f"{path}: empty trace")
    return seq, last_f, rows


def _read_latch(path: str, leg: str) -> tuple[dict[int, int], int]:
    seq, last_f, _rows = _read_leg(path, leg, _extract_latch)
    return seq, last_f


def load_latch(path: str, leg: str) -> dict[int, int]:
    """读一条腿的 frame -> $0014 值。leg ∈ {"biz","mesen","rust"}。

    缺字段 / 坏行 / 重复帧 / 空文件一律 fail-loud（ValueError）。
    """
    seq, _last_f = _read_latch(path, leg)
    return seq


def load_state(
    path: str,
    leg: str,
    fields: tuple[str, ...] = STATE_FIELDS,
) -> dict[int, dict[str, int | None]]:
    """读一条腿的 frame -> 状态字段，默认 {"status","room"}，另含 "latch"。

    fields 可扩展为 health_lo/health_hi/scrolldir（地址见 FIELD_ADDRS）；
    请求的状态字节缺键/长度不符 fail-loud。$0014 缺失记为 None（不阻断状态面），
    真正只读锁存面请用 load_latch。
    """
    fields = tuple(fields)
    if not fields:
        raise ValueError("fields must not be empty")

    def extractor(row: dict, leg: str, path: str, lineno: int) -> dict:
        return _extract_state_fields(row, leg, path, lineno, fields)

    seq, _last_f, _rows = _read_leg(path, leg, extractor)
    return seq


def _window(seq: dict[int, int], start: int, end: int) -> dict[int, int]:
    return {f: value for f, value in seq.items() if start <= f <= end}


def _window_records(
    seq: dict[int, dict[str, int | None]], start: int, end: int
) -> dict[int, dict[str, int | None]]:
    return {f: rec for f, rec in seq.items() if start <= f <= end}


def _field_resolution(
    field: str,
    a_state: dict[int, dict[str, int]],
    b_state: dict[int, dict[str, int]],
    max_lag: int,
) -> dict[str, int | float | None]:
    """状态面字段：全帧一致率求 offset；变化帧/段数只作可解性闸门与诊断。

    多解（一致率并列）报 AlignmentError("ambiguous-offset")，不偷偷挑一个。
    """
    a_field = {f: rec[field] for f, rec in a_state.items()}
    b_field = {f: rec[field] for f, rec in b_state.items()}
    changes = _change_frames(a_field)
    change_runs = _count_runs(changes)
    if not changes or change_runs < MIN_ANCHOR_CHANGES:
        raise AlignmentError(
            f"insufficient anchor changes for {field}", "insufficient"
        )
    scored = _allframe_score(a_field, b_field, max_lag)
    if scored["best"] < MIN_ANCHOR_CHANGES:
        raise AlignmentError(
            f"insufficient anchor matches for {field}", "insufficient"
        )
    if len(scored["winners"]) != 1:
        raise AlignmentError(
            f"ambiguous offset for {field}", "ambiguous-offset"
        )
    winner = scored["winners"][0]
    participants = winner["participants"]
    return {
        "offset": winner["offset"],
        "matches": winner["matches"],
        "participants": participants,
        "agree_rate": round(winner["matches"] / participants, 6)
        if participants
        else None,
        "change_frames": len(changes),
        "change_runs": change_runs,
    }


def assert_latch_gap(state_offset: int, latch_offset: int, leg: str) -> None:
    """锁存面必须恒比状态面晚一行；不成立即抛 AlignmentError。

    调用方（resolve_leg）捕获后必须保留状态面结果，只把 latch_offset 置 None、
    置 latch_gap_anomaly=True；本函数本身仍 fail-loud，不许静默用锁存偏移。
    """
    if latch_offset != state_offset + 1:
        raise AlignmentError(
            f"latch-state gap anomaly on {leg}: latch_offset={latch_offset:+d} "
            f"!= state_offset+1={state_offset + 1:+d}",
            "latch-state-gap-anomaly",
        )


def resolve_leg(
    leg: str,
    a_state: dict[int, dict[str, int | None]],
    b_state: dict[int, dict[str, int | None]],
    max_lag: int = DEFAULT_MAX_LAG,
    require_latch: bool = True,
) -> dict:
    """求一条腿相对 b 的状态面偏移、锁存面偏移、逐字段一致率与残差。

    返回:
      state_offset, latch_offset,
      per_field {status: {offset,matches,participants,agree_rate}, room: ...},
      latch {offset,matches,participants,agree_rate,change_frames,change_runs},
      state_joint {matches,participants,agree_rate},
      residuals {latch: [...] | None, state: {status: [...], room: [...]}},
      latch_gap_anomaly, latch_missing, latch_error

    状态面 status/room 必须给出同一个 offset，否则 AlignmentError
    ("field-disagreement")；状态面字段多解 -> ("ambiguous-offset")。
    锁存面是佐证面，不许一票否决状态面：gap 异常时保留状态结果，
    latch_offset=None、latch_gap_anomaly=True；缺 $0014 时保留状态结果、
    latch_offset=None、latch_missing=True（require_latch=True 时才抛
    "latch-unavailable"）。
    """
    if leg not in LEGS:
        raise ValueError(f"unknown leg {leg!r}; expected one of {LEGS}")

    per_field = {
        field: _field_resolution(field, a_state, b_state, max_lag)
        for field in STATE_FIELDS
    }
    if per_field["status"]["offset"] != per_field["room"]["offset"]:
        raise AlignmentError(
            "field-disagreement: "
            f"status={per_field['status']['offset']:+d} "
            f"room={per_field['room']['offset']:+d}",
            "field-disagreement",
        )
    state_offset = per_field["status"]["offset"]
    state_residuals = {
        field: [
            f
            for f in a_state
            if (f + state_offset) in b_state
            and a_state[f][field] != b_state[f + state_offset][field]
        ]
        for field in STATE_FIELDS
    }

    def make_latch_info(
        *,
        offset: int | None = None,
        matches: int | None = None,
        participants: int | None = None,
        agree_rate: float | None = None,
        change_frames: int = 0,
        change_runs: int = 0,
        fit_matches: int | None = None,
        fit_participants: int | None = None,
    ) -> dict:
        return {
            "offset": offset,
            "matches": matches,
            "participants": participants,
            "agree_rate": agree_rate,
            "change_frames": change_frames,
            "change_runs": change_runs,
            "fit_matches": fit_matches,
            "fit_participants": fit_participants,
        }

    a_latch = {f: rec["latch"] for f, rec in a_state.items()}
    b_latch = {f: rec["latch"] for f, rec in b_state.items()}
    latch_missing = any(v is None for v in a_latch.values()) or any(
        v is None for v in b_latch.values()
    )
    if require_latch and latch_missing:
        raise AlignmentError(
            f"latch unavailable for {leg}: row missing $0014", "latch-unavailable"
        )

    latch_offset: int | None = None
    latch_gap_anomaly = False
    latch_error: str | None = None
    latch_residuals: list[int] | None = None
    latch_info = make_latch_info()

    if latch_missing:
        latch_error = f"latch unavailable for {leg}: row missing $0014"
    else:
        latch_scored = _score(a_latch, b_latch, max_lag)
        try:
            (
                latch_offset,
                _latch_runs,
                latch_matches,
                latch_participants,
            ) = _fit(a_latch, b_latch, max_lag)
        except ValueError as exc:
            latch_error = str(exc)
            latch_info = make_latch_info(
                change_frames=latch_scored["change_frames"],
                change_runs=latch_scored["change_runs"],
            )
        else:
            latch_all = _allframe_score(a_latch, b_latch, max_lag)
            if len(latch_all["winners"]) != 1:
                latch_error = "ambiguous latch offset (all-frame)"
                latch_info = make_latch_info(
                    change_frames=latch_scored["change_frames"],
                    change_runs=latch_scored["change_runs"],
                    fit_matches=latch_matches,
                    fit_participants=latch_participants,
                )
                latch_offset = None
            else:
                latch_winner = latch_all["winners"][0]
                if latch_winner["offset"] != latch_offset:
                    latch_error = (
                        "latch offset disagreement: change-frame "
                        f"{latch_offset:+d} vs all-frame "
                        f"{latch_winner['offset']:+d}"
                    )
                    latch_info = make_latch_info(
                        change_frames=latch_scored["change_frames"],
                        change_runs=latch_scored["change_runs"],
                        fit_matches=latch_matches,
                        fit_participants=latch_participants,
                    )
                    latch_offset = None
                else:
                    try:
                        assert_latch_gap(state_offset, latch_offset, leg)
                    except AlignmentError as exc:
                        latch_gap_anomaly = True
                        latch_error = str(exc)
                        latch_info = make_latch_info(
                            change_frames=latch_scored["change_frames"],
                            change_runs=latch_scored["change_runs"],
                            fit_matches=latch_matches,
                            fit_participants=latch_participants,
                        )
                        latch_offset = None
                    else:
                        latch_agree = (
                            round(
                                latch_winner["matches"]
                                / latch_winner["participants"],
                                6,
                            )
                            if latch_winner["participants"]
                            else None
                        )
                        latch_info = make_latch_info(
                            offset=latch_offset,
                            matches=latch_winner["matches"],
                            participants=latch_winner["participants"],
                            agree_rate=latch_agree,
                            change_frames=latch_scored["change_frames"],
                            change_runs=latch_scored["change_runs"],
                            fit_matches=latch_matches,
                            fit_participants=latch_participants,
                        )
                        latch_residuals = [
                            f
                            for f in a_state
                            if (f + latch_offset) in b_state
                            and a_state[f]["latch"]
                            != b_state[f + latch_offset]["latch"]
                        ]

    joint_participants = [
        f for f in a_state if (f + state_offset) in b_state
    ]
    joint_matches = sum(
        1
        for f in joint_participants
        if a_state[f]["status"] == b_state[f + state_offset]["status"]
        and a_state[f]["room"] == b_state[f + state_offset]["room"]
    )
    state_joint = {
        "matches": joint_matches,
        "participants": len(joint_participants),
        "agree_rate": round(joint_matches / len(joint_participants), 6)
        if joint_participants
        else None,
    }

    return {
        "state_offset": state_offset,
        "latch_offset": latch_offset,
        "latch_gap_anomaly": latch_gap_anomaly,
        "latch_missing": latch_missing,
        "latch_error": latch_error,
        "per_field": per_field,
        "latch": latch_info,
        "state_joint": state_joint,
        "residuals": {"latch": latch_residuals, "state": state_residuals},
    }


def _provenance(path: str, rows: int, last_f: int) -> dict:
    stat = os.stat(path)
    return {
        "path": path,
        "rows": rows,
        "last_f": last_f,
        "mtime": datetime.fromtimestamp(stat.st_mtime)
        .astimezone()
        .isoformat(timespec="seconds"),
    }


def _build_parser() -> argparse.ArgumentParser:
    ap = argparse.ArgumentParser(
        prog="tools/movie_ref/align_legs.py",
        description="用状态面（status/room）与 $0014 锁存面机械反解三腿帧偏移（计划 Task 1）。",
    )
    ap.add_argument("--bizhawk", required=True, help="BizHawk quickerNES JSONL")
    ap.add_argument(
        "--mesen",
        required=True,
        help="Mesen2 JSONL（状态面优先 SamusStatus/RoomNumber，缺则 w['0100-07ff']/w['0000-00ff']；须含 w['Joy1Latch']）",
    )
    ap.add_argument(
        "--rust",
        required=True,
        help="Rust 重制 JSONL（w['zp'] 0x0000-00ff + w['iram'] 0x0100-07ff 扁平 hex）",
    )
    ap.add_argument("--out", required=True, help="输出 leg_offsets.json 路径")
    ap.add_argument("--start", type=int, default=MIN_WINDOW_START)
    ap.add_argument("--end", type=int, default=30000)
    return ap


def main(argv: list[str] | None = None) -> int:
    ap = _build_parser()
    args = ap.parse_args(argv)

    if args.start < MIN_WINDOW_START:
        ap.error(
            f"--start 必须 >= {MIN_WINDOW_START}：f0-f{MIN_WINDOW_START - 1} "
            "上电段两核 RAM 填充不同（0xFF vs 0x00），对齐不可信"
        )
    if args.end < args.start:
        ap.error("--end 必须 >= --start")

    records: dict[str, dict[int, dict[str, int]]] = {}
    meta: dict[str, dict[str, int | str]] = {}
    for leg, path in (
        ("biz", args.bizhawk),
        ("mesen", args.mesen),
        ("rust", args.rust),
    ):
        seq, last_f, rows = _read_leg(path, leg, _extract_state)
        if last_f < args.end:
            print(
                f"WARN: {leg} 轨迹最后一帧 f={last_f} < --end={args.end}，"
                "轨迹可能被截断；结论只覆盖到实际最后一帧",
                file=sys.stderr,
            )
        records[leg] = seq
        meta[leg] = {"path": path, "rows": rows, "last_f": last_f}

    effective_end = min(
        [args.end] + [int(m["last_f"]) for m in meta.values()]
    )
    if effective_end < args.start:
        ap.error(
            f"三腿没有共同帧：effective_end={effective_end} < --start={args.start}"
        )
    fit_end = effective_end - 1
    if fit_end < args.start:
        ap.error(
            f"可比窗口太短：fit_end={fit_end} < --start={args.start}（至少需要两帧）"
        )

    # a 侧只到 effective_end-1：保证 b 至少能提供 +1 帧；b 侧包含 effective_end。
    a_biz = _window_records(records["biz"], args.start, fit_end)
    a_rust = _window_records(records["rust"], args.start, fit_end)
    b_mesen = _window_records(records["mesen"], args.start, effective_end)

    resolved: dict[str, dict] = {}
    failures: dict[str, tuple[str, str]] = {}
    for leg, a_state in (("biz", a_biz), ("rust", a_rust)):
        try:
            resolved[leg] = resolve_leg(
                leg, a_state, b_mesen, require_latch=False
            )
        except AlignmentError as exc:
            failures[leg] = (str(exc), exc.verdict)
            print(f"FAIL {leg}->mesen: {exc}", file=sys.stderr)
        except ValueError as exc:
            failures[leg] = (str(exc), "insufficient")
            print(f"FAIL {leg}->mesen: {exc}", file=sys.stderr)
        else:
            result = resolved[leg]
            latch_offset = result["latch_offset"]
            latch_offset_text = (
                f"{latch_offset:+d}" if latch_offset is not None else "None"
            )
            latch = result["latch"]
            latch_agree_text = (
                f"{latch['matches']}/{latch['participants']}"
                if latch["matches"] is not None
                and latch["participants"] is not None
                else "None"
            )
            print(
                f"OK {leg}->mesen: state_offset={result['state_offset']:+d} "
                f"latch_offset={latch_offset_text} "
                f"state_agree={result['per_field']['status']['matches']}/"
                f"{result['per_field']['status']['participants']} "
                f"latch_agree={latch_agree_text}",
                file=sys.stderr,
            )

    offsets: dict[str, int | None] = {
        "biz_to_mesen": None,
        "rust_to_mesen": None,
    }
    latch_offsets: dict[str, int | None] = {
        "biz_to_mesen": None,
        "rust_to_mesen": None,
    }
    for leg, out_key in (("biz", "biz_to_mesen"), ("rust", "rust_to_mesen")):
        if leg in resolved:
            offsets[out_key] = resolved[leg]["state_offset"]
            latch_offsets[out_key] = resolved[leg]["latch_offset"]

    per_field = {leg: result["per_field"] for leg, result in resolved.items()}
    agree_rate: dict[str, float | None] = {"biz": None, "rust": None}
    state_joint_agree_rate: dict[str, float | None] = {"biz": None, "rust": None}
    latch_agree_rate: dict[str, float | None] = {"biz": None, "rust": None}
    for leg, result in resolved.items():
        agree_rate[leg] = result["per_field"]["status"]["agree_rate"]
        state_joint_agree_rate[leg] = result["state_joint"]["agree_rate"]
        latch_agree_rate[leg] = result["latch"]["agree_rate"]

    primary = resolved.get("biz") or resolved.get("rust")
    if primary is None:
        anchors = {"addr": ANCHOR_ADDR_NAME, "change_frames": 0, "change_runs": 0}
    else:
        anchors = {
            "addr": ANCHOR_ADDR_NAME,
            "change_frames": primary["latch"]["change_frames"],
            "change_runs": primary["latch"]["change_runs"],
            "by_leg": {
                leg: {
                    "change_frames": result["latch"]["change_frames"],
                    "change_runs": result["latch"]["change_runs"],
                }
                for leg, result in resolved.items()
            },
        }

    residuals: dict = {"biz": [], "rust": [], "state": {}}
    for leg in ("biz", "rust"):
        if leg in resolved:
            residuals[leg] = resolved[leg]["residuals"]["latch"]
            residuals["state"][leg] = resolved[leg]["residuals"]["state"]

    latch_gap_anomaly = any(
        result["latch_gap_anomaly"] for result in resolved.values()
    )
    latch_missing = any(result["latch_missing"] for result in resolved.values())
    latch_errors = {
        leg: result["latch_error"]
        for leg, result in resolved.items()
        if result["latch_error"] is not None
    }

    state_verdicts = {verdict for _message, verdict in failures.values()}
    if "ambiguous-offset" in state_verdicts:
        verdict = "ambiguous-offset"
    elif "field-disagreement" in state_verdicts:
        verdict = "field-disagreement"
    elif state_verdicts:
        verdict = "insufficient"
    elif latch_gap_anomaly:
        verdict = "latch-state-gap-anomaly"
    elif latch_missing or latch_errors:
        verdict = "latch-unavailable"
    else:
        verdict = "aligned"

    denominators = {
        "anchors": {
            "basis": "change_frames",
            "description": (
                "$0014 变化帧口径：a 侧 f 与 f-1 比较；change_frames 为裸变化帧数，"
                "change_runs 为最大连续段数"
            ),
            "by_leg": {
                leg: {
                    "change_frames": result["latch"]["change_frames"],
                    "change_runs": result["latch"]["change_runs"],
                }
                for leg, result in resolved.items()
            },
        },
        "per_field": {
            "basis": "all_frames",
            "description": (
                "状态面 status/room 全帧口径：每个 a 侧可比帧计一次一致/不一致；"
                "participants 是该 offset 下可映射到 b 的帧数"
            ),
            "by_leg": {
                leg: {
                    field: {
                        "matches": result["per_field"][field]["matches"],
                        "participants": result["per_field"][field]["participants"],
                    }
                    for field in STATE_FIELDS
                }
                for leg, result in resolved.items()
            },
        },
        "residuals": {
            "basis": "all_frames",
            "description": (
                "residuals[leg] 是锁存面全帧残差（latch_mismatch/latch_participants）；"
                "residuals.state[leg][field] 是状态面全帧残差"
            ),
            "by_leg": {
                leg: {
                    "latch_mismatch": (
                        len(result["residuals"]["latch"])
                        if result["residuals"]["latch"] is not None
                        else None
                    ),
                    "latch_participants": result["latch"]["participants"],
                    "state_participants": result["state_joint"]["participants"],
                }
                for leg, result in resolved.items()
            },
        },
    }

    result = {
        "window": [args.start, effective_end],
        "effective_end": effective_end,
        "fit_window": [args.start, fit_end],
        "anchors": anchors,
        "offsets": offsets,
        "latch_offsets": latch_offsets,
        "latch_gap_anomaly": latch_gap_anomaly,
        "latch_missing": latch_missing,
        "latch_errors": latch_errors,
        "per_field": per_field,
        "agree_rate": agree_rate,
        "state_joint_agree_rate": state_joint_agree_rate,
        "latch_agree_rate": latch_agree_rate,
        "residuals": residuals,
        "denominators": denominators,
        "provenance": {
            leg: _provenance(
                str(meta[leg]["path"]),
                int(meta[leg]["rows"]),
                int(meta[leg]["last_f"]),
            )
            for leg in LEGS
        },
        "verdict": verdict,
    }

    out_path = Path(args.out)
    out_path.parent.mkdir(parents=True, exist_ok=True)
    text = json.dumps(result, ensure_ascii=False, indent=2)
    out_path.write_text(text + "\n", encoding="utf-8")
    print(text)
    if verdict != "aligned":
        print(f"VERDICT: {verdict}（不挑偏移；详见 stderr）", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
