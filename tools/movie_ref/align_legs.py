#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""tools/movie_ref/align_legs.py —— 因果锚帧映射器（计划 Task 1）。

三条腿的帧号语义不同（谁在哪一帧"收工"不同）。本工具不再用"四字段打分最优"
猜偏移，而是用 $0014 输入锁存（JOY1_LATCH）的值跳变做因果锚，机械反解：

    a[f] == b[f + offset]

即 offset=+1 表示 b 腿比 a 腿晚一帧才看到同一个输入锁存值。

地址来源（本工具是唯一允许硬编码这两个地址的 tools/ 文件）：
- $0014：计划/任务称 JOY1_LATCH；crates/game-core/src/ram_map.rs:70 的具名常量
  实际叫 JOY1_STATUS（doc comment "Joy1Status=$14 … all buttons currently being
  pressed"）。本工具按 $0014 的字节语义读取，不另造地址。
- $005A：ROOM_NUMBER（crates/game-core/src/ram_map.rs:216）。加载窗
  （$005A != $FF）的排除属于 Task 2 的 classify_episodes.py 口径，本工具不消费。

判据（计划 Task 1 Interfaces + Step 3）：
- 只在 a 的"值变化帧"（a[f] != a[f-1] 且 f-1 也在 a 内）上比较；
- 对 offset ∈ [-max_lag, +max_lag] 计一致数，取最大且唯一；
- changes = 参与拟合的"输入跳变次数"（变化帧的最大连续段数；一次 1 帧按键
  的 press+revert 算 1 次跳变，而不是 2 个变化帧）。这与计划 Step 1 的
  期望 `align(a, b) == (1, 4)`（4 次按键 → 8 个变化帧）一致；
- changes < 3 → ValueError("insufficient anchor changes")；
- 多解 → ValueError("ambiguous offset")。绝不挑一个偏移。

CLI 失败口径：
- Mesen2 行缺 w["Joy1Latch"]、Rust 行缺 zp、业务行缺字段、JSON 坏行、重复帧、
  空轨迹：fail-loud（抛 ValueError / 非零退出），不静默跳过；
- 轨迹最后一行 f < --end：stderr 警告"轨迹可能被截断"，仍按已有帧出结论，
  但结论窗口只覆盖到实际最后一帧；
- 多解/锚不足：写出 verdict=insufficient 的 JSON，offsets/agree_rate 写 null，
  不伪造整数偏移。

用法：
    python3 tools/movie_ref/align_legs.py \
        --bizhawk analysis/traces/bizhawk-6725M-post.jsonl \
        --mesen analysis/traces/_probe-axis.frames.jsonl \
        --rust /tmp/rust-states-t2.jsonl \
        --out data/status/leg_offsets.json [--start 20] [--end 30000]
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

# $0014 = JOY1_LATCH（ram_map.rs:70 实名 JOY1_STATUS）。
ANCHOR_ADDR = 0x0014
ANCHOR_ADDR_NAME = "$0014"
# $005A = ROOM_NUMBER（ram_map.rs:216）；Task 2 加载窗分类用，本工具不消费。

LEGS = ("biz", "mesen", "rust")
MIN_ANCHOR_CHANGES = 3
MIN_WINDOW_START = 20  # f0-f19 两核 RAM 填充不同（0xFF vs 0x00），对齐不可信。
DEFAULT_MAX_LAG = 4


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


def _fit(
    seq_a: dict[int, int],
    seq_b: dict[int, int],
    max_lag: int = DEFAULT_MAX_LAG,
) -> tuple[int, int, int, int]:
    """返回 (offset, changes, matches, participants)。

    changes 是参与拟合的输入跳变次数（变化帧连续段数），不是变化帧原始个数。
    """
    if max_lag < 0:
        raise ValueError("max_lag must be >= 0")

    changes = _change_frames(seq_a)
    if not changes:
        raise ValueError("insufficient anchor changes")

    scored: list[tuple[int, int, list[int]]] = []
    for offset in range(-max_lag, max_lag + 1):
        participants = [f for f in changes if (f + offset) in seq_b]
        matches = sum(1 for f in participants if seq_a[f] == seq_b[f + offset])
        scored.append((matches, offset, participants))

    best_matches = max(item[0] for item in scored)
    if best_matches < MIN_ANCHOR_CHANGES:
        raise ValueError("insufficient anchor changes")

    winners = [item for item in scored if item[0] == best_matches]
    if len(winners) != 1:
        raise ValueError("ambiguous offset")

    matches, offset, participants = winners[0]
    runs = _count_runs(participants)
    if runs < MIN_ANCHOR_CHANGES:
        raise ValueError("insufficient anchor changes")
    return offset, runs, matches, len(participants)


def align(
    seq_a: dict[int, int],
    seq_b: dict[int, int],
    max_lag: int = DEFAULT_MAX_LAG,
) -> tuple[int, int]:
    """反解 a→b 的整数帧偏移，语义 a[f] == b[f + offset]。

    返回 (offset, changes)。changes 是参与拟合的输入跳变次数。
    - 锚不足：ValueError("insufficient anchor changes")
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
    start = 2 * ANCHOR_ADDR
    if len(zp) < start + 2:
        raise ValueError(
            f"{path}:{lineno}: rust w['zp'] too short for $0014 "
            f"(need >= {start + 2} hex chars, got {len(zp)})"
        )
    try:
        return int(zp[start:start + 2], 16)
    except ValueError as exc:
        raise ValueError(
            f"{path}:{lineno}: rust w['zp'] has non-hex bytes at $0014: {zp[start:start + 2]!r}"
        ) from exc


def _read_latch(path: str, leg: str) -> tuple[dict[int, int], int]:
    """流式读一条腿，返回 (frame -> $0014, 最后一行 f)。"""
    if leg not in LEGS:
        raise ValueError(f"unknown leg {leg!r}; expected one of {LEGS}")

    seq: dict[int, int] = {}
    last_f: int | None = None
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
            seq[f] = _extract_latch(row, leg, path, lineno)
            last_f = f
    if last_f is None:
        raise ValueError(f"{path}: empty trace")
    return seq, last_f


def load_latch(path: str, leg: str) -> dict[int, int]:
    """读一条腿的 frame -> $0014 值。leg ∈ {"biz","mesen","rust"}。

    缺字段 / 坏行 / 重复帧 / 空文件一律 fail-loud（ValueError）。
    """
    seq, _last_f = _read_latch(path, leg)
    return seq


def _window(seq: dict[int, int], start: int, end: int) -> dict[int, int]:
    return {f: value for f, value in seq.items() if start <= f <= end}


def _build_parser() -> argparse.ArgumentParser:
    ap = argparse.ArgumentParser(
        prog="tools/movie_ref/align_legs.py",
        description="用 $0014 输入锁存跳变机械反解三腿帧偏移（计划 Task 1）。",
    )
    ap.add_argument("--bizhawk", required=True, help="BizHawk quickerNES JSONL")
    ap.add_argument("--mesen", required=True, help="Mesen2 JSONL（须含 w['Joy1Latch']）")
    ap.add_argument("--rust", required=True, help="Rust 重制 JSONL（w['zp'] 扁平 hex）")
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

    traces: dict[str, dict[int, int]] = {}
    for leg, path in (
        ("biz", args.bizhawk),
        ("mesen", args.mesen),
        ("rust", args.rust),
    ):
        seq, last_f = _read_latch(path, leg)
        if last_f < args.end:
            print(
                f"WARN: {leg} 轨迹最后一帧 f={last_f} < --end={args.end}，"
                "轨迹可能被截断；结论只覆盖到实际最后一帧",
                file=sys.stderr,
            )
        traces[leg] = _window(seq, args.start, args.end)

    offsets: dict[str, int | None] = {"biz_to_mesen": None, "rust_to_mesen": None}
    agree_rate: dict[str, float | None] = {"biz": None, "rust": None}
    changes_seen: list[int] = []
    failures: dict[str, str] = {}

    for leg, out_key in (("biz", "biz_to_mesen"), ("rust", "rust_to_mesen")):
        try:
            offset, changes, matches, participants = _fit(
                traces[leg], traces["mesen"]
            )
        except ValueError as exc:
            failures[leg] = str(exc)
            print(f"FAIL {leg}->mesen: {exc}", file=sys.stderr)
            continue
        offsets[out_key] = offset
        agree_rate[leg] = round(matches / participants, 6)
        changes_seen.append(changes)
        print(
            f"OK {leg}->mesen: offset={offset:+d} changes={changes} "
            f"agree={matches}/{participants}",
            file=sys.stderr,
        )

    verdict = "aligned" if not failures else "insufficient"
    result = {
        "window": [args.start, args.end],
        "anchors": {
            "addr": ANCHOR_ADDR_NAME,
            "changes": min(changes_seen) if changes_seen else 0,
        },
        "offsets": offsets,
        "agree_rate": agree_rate,
        "verdict": verdict,
    }

    out_path = Path(args.out)
    out_path.parent.mkdir(parents=True, exist_ok=True)
    text = json.dumps(result, ensure_ascii=False, indent=2)
    out_path.write_text(text + "\n", encoding="utf-8")
    print(text)
    if verdict != "aligned":
        print(f"VERDICT: {verdict}（不挑偏移；详见 stderr）", file=sys.stderr)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
