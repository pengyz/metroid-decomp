#!/usr/bin/env python3
"""跨模拟器逐帧比对：quickerNES（BizHawk 轨迹）↔ Mesen2（scenario frames 轨迹）。

两侧 schema 不同，本脚本只做**映射 + 统计**，不裁定「谁对」：

  BizHawk（analysis/traces/bizhawk-*.jsonl，tools/movie_ref/bizhawk_ref.lua 产出）
    {"f": N, "health_lo": .., "health_hi": .., "status": .., "room": .., "scrolldir": .., ...}
  Mesen2（analysis/traces/*.frames.jsonl，tools/scenario_run.py 产出）
    {"f": N, "w": {"Health": .., "SamusStatus": .., "RoomNumber": .., "Joy1Latch": ..}}

帧轴：两侧都从 0/1 起但 boot 相位不同，**必须**用 `--shift` 显式声明
（语义：`biz f = mesen f - shift`）。`--latch-align` 用 `$0014` 锁存序列反解 shift，
但 boot 段 quickerNES 有 1~2 帧额外上电相位，反解值只在 f>=20 有意义 —— 见
`docs/findings/movie-bizhawk-6725m.md` §11。

用法：
  python3 tools/movie_ref/compare_cores.py \
      --bizhawk analysis/traces/bizhawk-6725M.jsonl \
      --mesen /tmp/tmpaxis2.jsonl --start 20 --end 1000 --shift 0
  python3 tools/movie_ref/compare_cores.py ... --latch-align   # 反解 shift 再比
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

# 侧字段名 → Mesen w 里的键
MESEN_KEYS = {
    "health_lo": "Health",
    "status": "SamusStatus",
    "room": "RoomNumber",
}
FIELDS = ("health_lo", "health_hi", "status", "room", "scrolldir")


def _load_jsonl(path: Path) -> list[dict]:
    rows = []
    with path.open("r", encoding="utf-8") as fh:
        for ln, line in enumerate(fh, 1):
            line = line.strip()
            if not line:
                continue
            try:
                rows.append(json.loads(line))
            except json.JSONDecodeError as exc:
                raise SystemExit(f"{path}:{ln}: 非法 JSON 行：{exc}") from exc
    if not rows:
        raise SystemExit(f"{path}: 空文件（0 行）——拒绝在空轨迹上比对")
    return rows


def load_bizhawk(path: Path) -> dict[int, dict]:
    out = {}
    for row in _load_jsonl(path):
        if "f" not in row:
            raise SystemExit(f"{path}: 行缺 'f' 字段，schema 不符")
        out[int(row["f"])] = row
    return out


def load_mesen(path: Path) -> dict[int, dict]:
    out = {}
    for row in _load_jsonl(path):
        if "w" not in row or "f" not in row:
            raise SystemExit(f"{path}: 行缺 'f'/'w'，不是 scenario frames 轨迹（可能传了 ctwrite 日志）")
        out[int(row["f"])] = row["w"]
    return out


def latch_align(biz: dict[int, dict], mesen: dict[int, dict], start: int, end: int) -> tuple[int, float]:
    """用 $0014 锁存序列反解 shift：返回 (shift, 一致率)。"""
    best = (0, -1.0)
    for shift in range(0, 8):
        ok = n = 0
        for f in range(start, end + 1):
            m = mesen.get(f - shift)
            b = biz.get(f)
            if m is None or b is None or "joy1_latch" not in b:
                continue
            n += 1
            if m.get("Joy1Latch") == b["joy1_latch"]:
                ok += 1
        if n and ok / n > best[1]:
            best = (shift, ok / n)
    return best


def compare(biz: dict[int, dict], mesen: dict[int, dict], shift: int, start: int, end: int):
    per_field = {k: 0 for k in FIELDS}
    mismatches = []
    comparable = 0
    for f in range(start, end + 1):
        m = mesen.get(f - shift)
        b = biz.get(f)
        if m is None or b is None:
            continue
        comparable += 1
        row = {}
        for field in FIELDS:
            mkey = MESEN_KEYS.get(field)
            if mkey is None or field not in b:
                continue
            if m.get(mkey) != b[field]:
                per_field[field] += 1
                row[field] = {"mesen": m.get(mkey), "bizhawk": b[field]}
        if row:
            mismatches.append({"f": f, "fields": row})
    return comparable, per_field, mismatches


def group(f: int) -> str:
    if f < 40:
        return "boot(f<40)"
    for lo, hi, name in ((482, 487, "attrib-482"), (672, 679, "attrib-672"), (944, 953, "attrib-944")):
        if lo <= f <= hi:
            return name
    return "other"


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser(description="quickerNES ↔ Mesen2 逐帧比对（映射+统计）")
    ap.add_argument("--bizhawk", required=True, type=Path)
    ap.add_argument("--mesen", required=True, type=Path)
    ap.add_argument("--start", type=int, default=20)
    ap.add_argument("--end", type=int, default=1000)
    ap.add_argument("--shift", type=int, default=0, help="biz f = mesen f - shift")
    ap.add_argument("--latch-align", action="store_true", help="先用 $0014 反解 shift")
    ap.add_argument("--json-out", type=Path, default=None)
    ap.add_argument("--show", type=int, default=40, help="打印前 N 个失配帧")
    ap.add_argument("--fail-on-mismatch", action="store_true")
    args = ap.parse_args(argv)

    biz = load_bizhawk(args.bizhawk)
    mesen = load_mesen(args.mesen)
    shift = args.shift
    if args.latch_align:
        shift, rate = latch_align(biz, mesen, args.start, args.end)
        print(f"[latch-align] 最优 shift={shift}（$0014 一致率 {rate:.4f}）")

    comparable, per_field, mismatches = compare(biz, mesen, shift, args.start, args.end)
    groups: dict[str, int] = {}
    for item in mismatches:
        groups[group(item["f"])] = groups.get(group(item["f"]), 0) + 1

    print(f"窗口 f={args.start}..{args.end}，shift={shift}，可比 {comparable} 帧")
    print("逐字段失配：" + "  ".join(f"{k}={v}" for k, v in per_field.items()))
    print(f"含失配帧 {len(mismatches)}；分组 {json.dumps(groups, ensure_ascii=False, sort_keys=True)}")
    for item in mismatches[: args.show]:
        detail = "  ".join(
            f"{k}: mesen={v['mesen']} biz={v['bizhawk']}" for k, v in item["fields"].items()
        )
        print(f"  f={item['f']}  {detail}")
    if len(mismatches) > args.show:
        print(f"  …其余 {len(mismatches) - args.show} 帧省略")

    if args.json_out:
        args.json_out.parent.mkdir(parents=True, exist_ok=True)
        payload = {
            "window": [args.start, args.end],
            "shift": shift,
            "comparable": comparable,
            "per_field": per_field,
            "groups": groups,
            "mismatches": mismatches,
        }
        args.json_out.write_text(json.dumps(payload, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
        print(f"json: {args.json_out}")

    if args.fail_on_mismatch and mismatches:
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
