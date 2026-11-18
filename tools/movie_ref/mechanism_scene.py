#!/usr/bin/env python3
"""Task 5：机制级最小重现 runner（Mesen2 腿 + Rust 腿 → 归一状态 → 对齐）。

设计边界（用户 m01188 的硬口径）：

- **不重写两腿 runner**：Mesen2 腿走既有的 ``tools/scenario_run.py run``；
  Rust 腿走已构建的 ``target/debug/rust_screenshot --schedule --out --state-out``。
  本模块只做「写一个可用调度/临时 case → 起两个子进程 → 读产物 → 归一形状」。
- ``compare(mesen, rust, fields)`` 是纯函数、无 I/O；只比较传入字段；缺行/缺字段
  计入 ``incomparable``，不计入失配，也不冒充 ``compared``。
- 帧数硬上限 600。机制级最小重现不应退化成跑长片；超限直接抛 ``ValueError``，
  帮助文本里写明理由。
- 对齐使用 Task 1 的 ``align()``：优先状态面字段 ``status`` / ``room``，仅当它们
  都没有足够的跳变时才退回 ``latch``。返回的 ``offset`` 语义与 Task 1 一致：
  ``mesen[f] == rust[f + offset]``。

Mesen2 腿的观测面：临时 case 自动补 ``watch_range 0000-00ff`` 与
``watch_range 0100-07ff``，这样 Task 1/2 的 ``load_state`` 能直接解码
``status``/``room``/``latch``；不依赖音视频窗口或截图。
"""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
import tempfile
import uuid
from pathlib import Path
from typing import Iterable, Mapping

_ROOT = Path(__file__).resolve().parents[2]
if str(_ROOT) not in sys.path:
    sys.path.insert(0, str(_ROOT))

from tools.l4_diff_run import schedule_text  # noqa: E402
from tools.movie_ref.align_legs import _change_frames, align, load_state  # noqa: E402
from tools.scenario_run import ROOT, expand_pads, parse_scenario  # noqa: E402

MAX_FRAMES = 600
DEFAULT_FIELDS = ("status", "room")
# load_state 的 fields 只接受状态面字节；latch 由它固定附带（$0014 可缺失为 None）。
STATE_FIELDS = ("status", "room")
OFFSETS_PATH = ROOT / "data" / "status" / "leg_offsets.json"
_MESEN_WATCH_RANGES = ("0000-00ff", "0100-07ff")


def build_parser() -> argparse.ArgumentParser:
    p = argparse.ArgumentParser(
        description="机制级最小重现 runner：同一短 case 跑 Mesen2 与 Rust，按状态面字段对拍。",
        epilog=(
            "帧数硬上限 600：机制级最小重现不允许退化成跑长片；"
            "超限直接拒绝，避免把覆盖不足伪装成机制证据。"
        ),
    )
    p.add_argument("--case", required=True, help="现有 case DSL（或临时 case）路径")
    p.add_argument("--frames", type=int, default=120, help="实际运行帧数（1..600）")
    p.add_argument("--fields", default=",".join(DEFAULT_FIELDS),
                   help="逗号分隔的比较字段，默认 status,room；latch 只作对齐锚")
    p.add_argument("--json-out", default=None, help="JSON 报告输出路径")
    p.add_argument("--timeout", type=float, default=300.0, help="Mesen2 腿超时（秒）")
    p.add_argument("--allow-constant", action="store_true",
                   help="常量窗（无判别力）也返回 0；默认 exit 3")
    p.add_argument("--keep-trace", action="store_true",
                   help="保留本次 analysis/traces 下的 Mesen 副产品；默认清理")
    return p


def compare(mesen: Mapping[int, Mapping[str, object]],
            rust: Mapping[int, Mapping[str, object]],
            fields: Iterable[str]) -> dict:
    """纯函数：在同一帧号上比较指定字段。

    - ``compared``：实际比较过的 (帧, 字段) 对数；
    - ``incomparable``：缺行/缺字段的对数（分 ``incomparable_frames`` 与
      ``incomparable_fields`` 两个子计数），不计入失配；
    - ``mismatch``：失配的 (帧, 字段) 对数；``first_diff`` 是最小失配帧；
    - ``all_match``：至少比较过一对，且没有任何失配。
    """
    fields = tuple(fields)
    frames = sorted(set(mesen) | set(rust))
    compared = 0
    mismatch = 0
    per_field = {f: 0 for f in fields}
    incomparable_frames = 0
    incomparable_fields = 0
    mismatch_frames: list[int] = []
    first_diff: int | None = None
    distinct: dict[str, set] = {f: set() for f in fields}
    for frame in frames:
        left = mesen.get(frame)
        right = rust.get(frame)
        if left is None or right is None:
            incomparable_frames += 1
            continue
        bad_row = False
        for field in fields:
            if field not in left or field not in right:
                incomparable_fields += 1
                continue
            compared += 1
            distinct[field].add(left[field])
            distinct[field].add(right[field])
            if left[field] != right[field]:
                mismatch += 1
                per_field[field] += 1
                bad_row = True
                if first_diff is None or frame < first_diff:
                    first_diff = frame
        if bad_row:
            mismatch_frames.append(frame)
    informative = any(len(values) >= 2 for values in distinct.values())
    return {
        "informative": informative,
        "distinct_values": {f: len(v) for f, v in distinct.items()},
        "first_diff": first_diff,
        "mismatch": mismatch,
        "per_field": per_field,
        "all_match": mismatch == 0 and compared > 0,
        "compared": compared,
        "incomparable": incomparable_frames + incomparable_fields,
        "incomparable_frames": incomparable_frames,
        "incomparable_fields": incomparable_fields,
        "mismatch_frames": mismatch_frames,
        "fields": list(fields),
    }


def _safe_name(stem: str) -> str:
    return re.sub(r"[^A-Za-z0-9_.-]+", "_", stem)[:40] or "scene"


def _shorten_case(case_path: Path, frames: int) -> tuple[str, str]:
    """把 case 截到 frames 帧，返回 (name, 临时 case 文本)。

    case 仍是唯一输入语义源：只裁 ``frames`` 与越界采集/按键，不重写输入。
    Mesen2 观测面补成两段 RAM range（不改变输入行为）。
    """
    name = f"mechanism-{_safe_name(case_path.stem)}-{frames}-{uuid.uuid4().hex[:8]}"
    src_lines = case_path.read_text(encoding="utf-8").splitlines()
    out: list[str] = []
    has_frames = False
    has_name = False
    has_low = False
    has_high = False
    has_latch = False
    for raw in src_lines:
        body = raw.split("#", 1)[0].strip()
        if not body:
            out.append(raw)
            continue
        toks = body.split()
        op = toks[0]
        if op == "name":
            out.append(f"name {name}")
            has_name = True
        elif op == "frames":
            out.append(f"frames {frames}")
            has_frames = True
        elif op == "capture":
            if len(toks) >= 3 and toks[1].isdigit() and int(toks[1]) <= frames:
                out.append(raw)
        elif op == "endFrame":
            if len(toks) >= 2 and toks[1].isdigit() and int(toks[1]) <= frames:
                out.append(raw)
        elif op == "press":
            if len(toks) >= 2 and toks[1].isdigit() and int(toks[1]) <= frames:
                out.append(raw)
        elif op == "hold":
            if len(toks) >= 2 and "-" in toks[1]:
                lo_s, hi_s = toks[1].split("-", 1)
                if lo_s.isdigit() and hi_s.isdigit():
                    lo, hi = int(lo_s), int(hi_s)
                    if lo <= frames:
                        out.append(" ".join([op, f"{lo}-{min(hi, frames)}", *toks[2:]]))
                    continue
            out.append(raw)
        elif op == "press_every":
            if len(toks) >= 3 and "-" in toks[2]:
                lo_s, hi_s = toks[2].split("-", 1)
                if lo_s.isdigit() and hi_s.isdigit():
                    lo, hi = int(lo_s), int(hi_s)
                    if lo <= frames:
                        out.append(" ".join([op, toks[1], f"{lo}-{min(hi, frames)}", *toks[3:]]))
                    continue
            out.append(raw)
        elif op == "watch_range":
            if len(toks) >= 2 and toks[1] == "0000-00ff":
                has_low = True
            if len(toks) >= 2 and toks[1] == "0100-07ff":
                has_high = True
            out.append(raw)
        elif op == "watch":
            if len(toks) >= 2 and toks[1].lower() == "0014":
                # 统一成 Task 1 load_state 认识的锁存面标签；只改观测名，不改输入。
                out.append("watch 0014 Joy1Latch")
                has_latch = True
            else:
                out.append(raw)
        else:
            out.append(raw)
    if not has_name:
        out.insert(0, f"name {name}")
    if not has_frames:
        out.insert(0, f"frames {frames}")
    if not has_low:
        out.append("watch_range 0000-00ff")
    if not has_high:
        out.append("watch_range 0100-07ff")
    if not has_latch:
        # load_state 的 mesen 分支优先读命名锁存键；只有 range 时 latch=None，无法按
        # Task 1 的因果锚对齐。补一个观察面 watch，不改输入行为。
        out.append("watch 0014 Joy1Latch")
    return name, "\n".join(out) + "\n"


def _run(cmd: list[str], timeout: float, what: str) -> None:
    proc = subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True,
                          encoding="utf-8", errors="replace", timeout=timeout)
    if proc.returncode != 0:
        tail = (proc.stderr or proc.stdout or "").strip()[-2000:]
        raise RuntimeError(f"{what} 失败 rc={proc.returncode}: {tail}")


def _load_offsets() -> dict:
    """读 Task 1 的 leg_offsets.json；缺文件/缺字段一律 fail-loud。"""
    try:
        return json.loads(OFFSETS_PATH.read_text(encoding="utf-8"))
    except (OSError, ValueError) as exc:
        raise ValueError(f"读不到 Task 1 偏移表 {OFFSETS_PATH}: {exc}") from exc


def state_offset_from_latch(latch_offset_align: int, offsets_doc: dict) -> int:
    """把 align(mesen, rust) 的 latch offset 换成 state offset。

    入参口径必须是 ``align(mesen, rust)`` 的返回值（``mesen[f] == rust[f+o]``），
    不是 ``data/status/leg_offsets.json`` 的 ``rust_to_mesen`` 口径；两者符号约定
    相反。函数内用 Task 1 实测 gap 转换，并断言结果等于
    ``-offsets.rust_to_mesen``，防止两套约定混用。
    """
    gap = int(offsets_doc["latch_offsets"]["rust_to_mesen"]) - int(
        offsets_doc["offsets"]["rust_to_mesen"])
    state = int(latch_offset_align) + gap
    expected = -int(offsets_doc["offsets"]["rust_to_mesen"])
    if state != expected:
        raise ValueError(
            f"state offset 口径不一致：latch_offset_align={latch_offset_align} + gap={gap} "
            f"= {state}，但 leg_offsets 期望 {expected}"
        )
    return state


def _choose_anchor(mesen: dict, rust: dict, fields: tuple[str, ...]):
    """优先状态面字段对齐；状态面跳变不足时用 latch + Task 1 实测 gap。

    **不能**直接把 latch offset 当 state offset（Task 1 的 P0 教训）；latch 只
    用来借一帧相位，最终 offset 一定是状态面口径。
    """
    order = [f for f in ("status", "room") if f in fields]
    order += [f for f in fields if f not in order and f not in ("status", "room", "latch")]
    last_error: str | None = None
    for field in order:
        a = {f: row[field] for f, row in mesen.items() if field in row and row[field] is not None}
        b = {f: row[field] for f, row in rust.items() if field in row and row[field] is not None}
        try:
            offset, _changes = align(a, b, max_lag=4)
            return offset, field
        except ValueError as exc:
            last_error = (f"{field}: {exc} (mesen={len(a)} rust={len(b)} "
                          f"changes={len(_change_frames(a))}/{len(_change_frames(b))})")
    # 状态面不足：借 latch 定相位，再用 Task 1 实测 gap 换回状态面口径。
    a = {f: row["latch"] for f, row in mesen.items()
         if "latch" in row and row["latch"] is not None}
    b = {f: row["latch"] for f, row in rust.items()
         if "latch" in row and row["latch"] is not None}
    try:
        latch_offset, _changes = align(a, b, max_lag=4)
    except ValueError as exc:
        raise ValueError(
            f"状态面字段跳变不足，latch 也无法对齐（{last_error or '无可用字段'}；latch: {exc}）"
        ) from exc
    return state_offset_from_latch(latch_offset, _load_offsets()), f"latch({latch_offset:+d}+gap)"
def run_scene(case_path: str, frames: int) -> dict:
    """跑一个短 case：Mesen2 + Rust，返回归一后的两腿状态与对齐偏移。

    ``mesen``/``rust`` 均为 ``{frame: {field: value}}``；``offset`` 满足
    ``mesen[f] == rust[f + offset]``（Task 1 ``align`` 语义）。不在这里做比较——
    比较由 ``compare`` 纯函数完成。
    """
    if isinstance(frames, bool) or not isinstance(frames, int) or frames <= 0:
        raise ValueError("frames 必须是正整数")
    if frames > MAX_FRAMES:
        raise ValueError(
            f"frames={frames} 超过硬上限 {MAX_FRAMES}：机制级最小重现不允许退化成跑长片"
        )
    src = Path(case_path)
    if not src.is_file():
        raise FileNotFoundError(f"case 不存在: {src}")
    with tempfile.TemporaryDirectory(prefix="mechanism-scene-") as td:
        tmp = Path(td)
        name, text = _shorten_case(src, frames)
        temp_case = tmp / f"{name}.txt"
        temp_case.write_text(text, encoding="utf-8")
        try:
            plan = parse_scenario(temp_case)
            if plan["frames"] != frames:
                raise ValueError(f"临时 case frames={plan['frames']} != 请求 {frames}")
            if plan.get("frames", 0) > MAX_FRAMES:
                raise ValueError(f"case frames={plan['frames']} 超过硬上限 {MAX_FRAMES}")
            # 1) Mesen2 腿：既有 runner，不复制其逻辑
            mesen_cmd = [sys.executable, str(ROOT / "tools" / "scenario_run.py"),
                         "run", str(temp_case), "--timeout", "300"]
            _run(mesen_cmd, timeout=360.0, what="Mesen2 scenario_run")
            trace = ROOT / "analysis" / "traces" / f"{plan['name']}.frames.jsonl"
            if not trace.is_file():
                raise FileNotFoundError(f"Mesen2 腿未产出 trace: {trace}")
            # 2) Rust 腿：既有 rust_screenshot，调度表与 Lua 同源（expand_pads + schedule_text）
            joy1, joy2 = expand_pads(plan)
            caps = [(int(c["frame"]), c["tag"]) for c in plan["captures"]
                    if int(c["frame"]) <= frames]
            snapshot = plan["bundle_load"]["path"] if plan.get("bundle_load") else None
            sched_text = schedule_text(plan["name"], frames, joy1, caps,
                                       snapshot=snapshot, fixtures=plan["fixtures"], joy2=joy2)
            sched = tmp / f"{plan['name']}.sched"
            sched.write_text(sched_text, encoding="utf-8", newline="\n")
            shots = tmp / "shots"
            state_out = tmp / f"{plan['name']}.states.jsonl"
            rust_bin = ROOT / "target" / "debug" / "rust_screenshot"
            if not rust_bin.is_file():
                raise FileNotFoundError(f"缺 Rust 腿二进制 {rust_bin}（先构建 game-core）")
            _run([str(rust_bin), "--schedule", str(sched), "--out", str(shots),
                  "--state-out", str(state_out)], timeout=300.0, what="Rust rust_screenshot")
            if not state_out.is_file():
                raise FileNotFoundError(f"Rust 腿未产出 state-out: {state_out}")
            # 3) 归一（Task 1 load_state：Mesen ranges / Rust zp+iram），4) 对齐
            mesen = load_state(str(trace), "mesen", fields=STATE_FIELDS)
            rust = load_state(str(state_out), "rust", fields=STATE_FIELDS)
            offset, anchor = _choose_anchor(mesen, rust, DEFAULT_FIELDS)
            return {"mesen": mesen, "rust": rust, "offset": offset,
                    "anchor_field": anchor, "trace_name": plan["name"]}
        finally:
            # 临时 case 不留在 cases/；trace/state 在 analysis（gitignored）中留作证据
            if temp_case.exists():
                temp_case.unlink()


def _cleanup_trace(out: dict) -> None:
    """删本次 run 在 analysis/traces 下的 Mesen 副产品（默认；--keep-trace 保留）。"""
    name = out.get("trace_name")
    if not name:
        return
    for p in (ROOT / "analysis" / "traces").glob(f"{name}.*"):
        try:
            p.unlink()
        except OSError:
            pass


def main(argv: list[str] | None = None) -> int:
    args = build_parser().parse_args(argv)
    fields = tuple(f.strip() for f in args.fields.split(",") if f.strip())
    if not fields:
        print("错误：--fields 不能为空", file=sys.stderr)
        return 2
    try:
        out = run_scene(args.case, args.frames)
    except (ValueError, FileNotFoundError, RuntimeError, TimeoutError) as exc:
        print(f"ERROR: {exc}", file=sys.stderr)
        return 2
    offset = int(out["offset"])
    # mesen[f] == rust[f + offset] ⇒ 对齐后的 rust_aligned[f] = rust[f + offset]
    # = 把 raw rust 的键 g 搬到 g - offset。
    rust_aligned = {g - offset: row for g, row in out["rust"].items()}
    cmp = compare(out["mesen"], rust_aligned, fields)
    print(f"OFFSET: {offset:+d}（anchor={out.get('anchor_field', '?')}; mesen[f] == rust[f{offset:+d}]）")
    print(f"COMPARE: compared={cmp['compared']} mismatch={cmp['mismatch']} "
          f"incomparable={cmp['incomparable']} first_diff={cmp['first_diff']}")
    print(f"FIELDS: {', '.join(fields)}")
    if not cmp["informative"]:
        if not args.allow_constant:
            print("RESULT: uninformative（常量窗，无判别力）")
            print("提示：常量窗没有对照信息量；确需接受请加 --allow-constant", file=sys.stderr)
            result_code = 3
        else:
            print("RESULT: uninformative（常量窗，无判别力；--allow-constant 显式接受）")
            result_code = 0 if cmp["all_match"] else 1
    elif cmp["all_match"]:
        print("RESULT: all-match")
        result_code = 0
    else:
        print(f"RESULT: DIFF first_diff={cmp['first_diff']} per_field={cmp['per_field']}")
        result_code = 1
    report = {
        "case": str(args.case),
        "frames": args.frames,
        "offset": offset,
        "anchor_field": out.get("anchor_field"),
        "compare": cmp,
        "mesen": out["mesen"],
        "rust": out["rust"],
    }
    if args.json_out:
        p = Path(args.json_out)
        p.parent.mkdir(parents=True, exist_ok=True)
        p.write_text(json.dumps(report, ensure_ascii=False, indent=2, sort_keys=True) + "\n",
                     encoding="utf-8")
        print(f"JSON: {p}")
    if not args.keep_trace:
        _cleanup_trace(out)
    return result_code


if __name__ == "__main__":
    raise SystemExit(main())
