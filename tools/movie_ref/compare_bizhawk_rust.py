#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""tools/movie_ref/compare_bizhawk_rust.py —— BizHawk(quickerNES) ↔ Rust 逐帧 RAM 面对拍。

对拍面（五项，全部有仓库内地址锚，见 findings §4 表）：
    health_lo=$0106  health_hi=$0107  status=$0300  room=$005A  scrolldir=$0049
外加输入面 `joy1_latch=$0014`（不是判定项，是**帧轴对齐证据**：两侧同帧同值 ⇒ 无双帧错位）。

对齐协议（铁律，与 findings §6.1 一致）：
    Rust 帧 N 消费录像第 N-1 帧输入；BizHawk 轨迹行 f=N-1 即该帧输入。
    故两侧**同帧号直接比**（Rust 状态行 f=N ↔ 轨迹 f=N）。
    上电差异：quickerNES 上电 RAM=0xFF vs Rust=0x00 ⇒ 比对窗口从 `--start`（默认 20）起。
    缺状态行：Rust 侧 iteration-span/title-续段帧不落状态行（engine.rs:112-126），
    判为「无采样缝」并**单列**，不计入失配也不计入可比帧。

Rust `--state-out` 行契约（crates/game-core/src/memmap/observe/frame_face.rs:9-11,32-34）：
    zp   = $0000-$00FF 每字节 2 hex（地址 a → 偏移 a*2）
    iram = $0100-$07FF（地址 a → 偏移 (a-0x100)*2）

用法（路径全部参数化，无本机硬编码）：
    python3 tools/movie_ref/compare_bizhawk_rust.py \
        --bizhawk analysis/traces/bizhawk-6725M-full.jsonl \
        --rust /tmp/rust-states-full.jsonl \
        --inputlog movies/eien86-6725M.inputlog \
        --json-out analysis/traces/ram-compare-full.json \
        --md-out analysis/traces/ram-compare-full.md
"""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

FIELDS = ("health_lo", "health_hi", "status", "room", "scrolldir")

# Rust 状态行切片：(标签, 面, CPU 地址, 面 base)——面与 base 是 frame_face.rs 契约
SLICE = {
    "health_lo": ("iram", 0x0106, 0x0100),
    "health_hi": ("iram", 0x0107, 0x0100),
    "status": ("iram", 0x0300, 0x0100),
    "room": ("zp", 0x005A, 0x0000),
    "scrolldir": ("zp", 0x0049, 0x0000),
    "joy1_latch": ("zp", 0x0014, 0x0000),
}
FACE_LEN = {"zp": 512, "iram": 3584}  # 2 hex/byte × 256 / 1792


def die(msg: str) -> "NoReturn":  # noqa: F821
    print(f"[compare] {msg}", file=sys.stderr)
    raise SystemExit(2)


def load_jsonl(path: Path) -> list[dict]:
    """读 JSONL。**容忍末尾半行**（轨迹仍在写入时读到截断末行是正常现象），
    但中段坏行一律 fail-loud——静默跳行会伪造「可比帧」分母。"""
    lines = [ln.strip() for ln in path.read_text(encoding="utf-8").splitlines()]
    last_data = max((i for i, ln in enumerate(lines) if ln), default=-1)
    rows: list[dict] = []
    for i, line in enumerate(lines):
        if not line:
            continue
        try:
            rows.append(json.loads(line))
        except json.JSONDecodeError as e:
            if i == last_data:
                print(f"[compare] 警告：{path} 末尾半行（轨迹仍在写入？）已跳过 —— {line[:60]!r}",
                      file=sys.stderr)
                continue
            die(f"{path}:{i + 1}: JSON 解析失败（{e}）")
    return rows


def load_bizhawk(path: Path) -> dict[int, dict]:
    out: dict[int, dict] = {}
    for r in load_jsonl(path):
        if "f" not in r:
            die(f"{path}: 存在无 `f` 字段的行")
        out[int(r["f"])] = r
    return out


def load_rust(path: Path) -> dict[int, dict]:
    out: dict[int, dict] = {}
    for r in load_jsonl(path):
        if "f" not in r or "w" not in r:
            die(f"{path}: 状态行缺 `f`/`w` 字段: {r!r:.80}")
        w = r["w"]
        vals: dict[str, int | None] = {}
        for name, (face, addr, base) in SLICE.items():
            hexs = w.get(face)
            if not isinstance(hexs, str) or len(hexs) != FACE_LEN[face]:
                die(f"{path}: f{r['f']} 面 `{face}` 编码长度异常"
                    f"（期望 {FACE_LEN[face]}，实际 {None if not isinstance(hexs, str) else len(hexs)}）")
            off = (addr - base) * 2
            vals[name] = int(hexs[off:off + 2], 16)
        out[int(r["f"])] = vals
    return out


def group_runs(frames: list[int], gap: int = 0) -> list[tuple[int, int]]:
    """把有序帧号切成「连续区间」（允许 gap 帧空隙仍算同段；gap=0 即严格连续）。"""
    if not frames:
        return []
    runs = [[frames[0], frames[0]]]
    for f in frames[1:]:
        if f - runs[-1][1] <= gap + 1:
            runs[-1][1] = f
        else:
            runs.append([f, f])
    return [(a, b) for a, b in runs]


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description="BizHawk ↔ Rust 逐帧 RAM 面对拍")
    ap.add_argument("--bizhawk", required=True, help="BizHawk 逐帧轨迹 jsonl")
    ap.add_argument("--rust", required=True, help="rust_screenshot --state-out 产物 jsonl")
    ap.add_argument("--inputlog", default=None,
                    help="可选：.inputlog，用于校验两腿喂的是同一份逐帧输入")
    ap.add_argument("--start", type=int, default=20,
                    help="比对窗口起点（默认 20：跳过 quickerNES 上电 RAM=0xFF 填窗）")
    ap.add_argument("--end", type=int, default=None, help="比对窗口终点（缺省=两侧最小末帧）")
    ap.add_argument("--cluster-gap", type=int, default=0,
                    help="失配区间聚合容忍的空隙帧数（0=严格连续）")
    ap.add_argument("--json-out", default=None, help="完整结构化结果落盘路径")
    ap.add_argument("--md-out", default=None, help="失配区间完整表落盘路径（Markdown）")
    ap.add_argument("--md-limit", type=int, default=10, help="stdout 只打印前 N 段区间")
    ap.add_argument("--fail-on-mismatch", action="store_true",
                    help="有失配帧则 exit 1（默认只报告，exit 0）")
    args = ap.parse_args(argv)

    bh_path = Path(args.bizhawk)
    rs_path = Path(args.rust)
    if not bh_path.is_file():
        die(f"找不到 BizHawk 轨迹: {bh_path}")
    if not rs_path.is_file():
        die(f"找不到 Rust 状态行: {rs_path}")

    biz = load_bizhawk(bh_path)
    rust = load_rust(rs_path)
    if not biz:
        die(f"{bh_path}: 0 行")
    if not rust:
        die(f"{rs_path}: 0 行")

    end = args.end if args.end is not None else min(max(biz), max(rust))
    start = args.start
    if end < start:
        die(f"窗口非法: start={start} > end={end}")

    window = list(range(start, end + 1))
    both = [f for f in window if f in biz and f in rust]
    miss_rust = [f for f in window if f not in rust and f in biz]
    miss_biz = [f for f in window if f not in biz and f in rust]

    mism: dict[int, dict[str, tuple[int, int]]] = {}
    for f in both:
        diff = {}
        for k in FIELDS:
            rb, rv = rust[f][k], biz[f][k]
            if rb is None or rv is None:
                continue
            if int(rb) != int(rv):
                diff[k] = (int(rb), int(rv))
        if diff:
            mism[f] = diff

    per_field: dict[str, int] = {k: 0 for k in FIELDS}
    for f, d in mism.items():
        for k in d:
            per_field[k] += 1

    runs = group_runs(sorted(mism), args.cluster_gap)
    run_rows = []
    for i, (a, b) in enumerate(runs, 1):
        frames = [f for f in range(a, b + 1) if f in mism]
        fields = sorted({k for f in frames for k in mism[f]})
        samples = []
        for f in frames:
            samples.append({"f": f,
                            "diff": {k: {"rust": v[0], "biz": v[1]}
                                     for k, v in sorted(mism[f].items())}})
        run_rows.append({"index": i, "from": a, "to": b, "frames": len(frames),
                         "fields": fields, "detail": samples})

    # 输入面校验：$0014 锁存（同帧同值 ⇒ 帧轴无错位）
    latch_bad = [f for f in both if rust[f]["joy1_latch"] != biz[f]["joy1_latch"]]
    inputlog_bad: list[dict] = []
    if args.inputlog:
        sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
        import movie_import  # 单源：inputlog 语义只在 movie_import

        table = movie_import.read_inputlog(Path(args.inputlog))
        for f in window:
            if f not in biz:
                continue
            want = table.get(f + 1, (0, 0, 0))[0]   # 轨迹 f ↔ inputlog 第 f+1 行
            got = int(biz[f]["p1"])
            if want != got:
                inputlog_bad.append({"f": f, "inputlog_p1": want, "bizhawk_p1": got})

    first = None
    if runs:
        f0 = runs[0][0]
        first = {"f": f0,
                 "diff": {k: {"rust": v[0], "biz": v[1]} for k, v in sorted(mism[f0].items())},
                 "rust_all": {k: rust[f0][k] for k in FIELDS},
                 "biz_all": {k: biz[f0][k] for k in FIELDS}}

    # 分段统计（1000 帧一桶）：让「全片」不只一个总数，能看出分歧从哪一段起变成全程
    buckets = []
    for b0 in range(start, end + 1, 1000):
        b1 = min(b0 + 999, end)
        fr = [f for f in range(b0, b1 + 1)]
        cmp_f = [f for f in fr if f in biz and f in rust]
        eq_f = [f for f in cmp_f if f not in mism]
        buckets.append({"from": b0, "to": b1, "comparable": len(cmp_f),
                        "all_equal": len(eq_f), "mismatch": len(cmp_f) - len(eq_f)})

    eq_runs = group_runs([f for f in both if f not in mism], args.cluster_gap)

    result = {
        "inputs": {"bizhawk": str(bh_path), "rust": str(rs_path),
                   "inputlog": args.inputlog},
        "window": {"start": start, "end": end, "frames": len(window)},
        "coverage": {
            "bizhawk_rows": len(biz), "bizhawk_f_min": min(biz), "bizhawk_f_max": max(biz),
            "rust_rows": len(rust), "rust_f_min": min(rust), "rust_f_max": max(rust),
            "comparable": len(both), "no_rust_row": len(miss_rust),
            "no_bizhawk_row": len(miss_biz),
        },
        "verdict": {
            "all_equal": len(both) - len(mism),
            "mismatch_frames": len(mism),
            "per_field": per_field,
            "equal_only_4": sum(1 for f in both
                                if all(rust[f][k] == biz[f][k]
                                       for k in ("health_lo", "health_hi", "status", "room"))),
            "equal_with_scroll": len(both) - len(mism),
        },
        "buckets": buckets,
        "equal_runs": eq_runs,
        "missing_rust_runs": group_runs(miss_rust, 0),
        "missing_biz_runs": group_runs(miss_biz, 0),
        "mismatch_runs": run_rows,
        "first_mismatch": first,
        "input_alignment": {
            "latch_mismatch": len(latch_bad),
            "latch_first": latch_bad[0] if latch_bad else None,
            "inputlog_mismatch": len(inputlog_bad),
            "inputlog_first": inputlog_bad[0] if inputlog_bad else None,
        },
    }

    # ---------------- stdout ----------------
    cov = result["coverage"]
    ver = result["verdict"]
    print("== BizHawk(quickerNES) ↔ Rust RAM 面对拍 ==")
    print(f"  BizHawk : {bh_path}  {cov['bizhawk_rows']} 行  f={cov['bizhawk_f_min']}..{cov['bizhawk_f_max']}")
    print(f"  Rust    : {rs_path}  {cov['rust_rows']} 行  f={cov['rust_f_min']}..{cov['rust_f_max']}")
    print(f"  窗口    : f={start}..{end}（{len(window)} 帧）")
    print(f"  可比帧  : {cov['comparable']}   Rust 无状态行: {cov['no_rust_row']}"
          f"   BizHawk 无行: {cov['no_bizhawk_row']}")
    print(f"  全同帧  : {ver['all_equal']}（五字段）/ {ver['equal_only_4']}（仅四字段，不含 scrolldir）")
    print(f"  失配帧  : {ver['mismatch_frames']}")
    print("  逐字段  : " + "  ".join(f"{k}={v}" for k, v in per_field.items()))
    if result["missing_rust_runs"]:
        seg = "、".join(f"{a}" if a == b else f"{a}..{b}" for a, b in result["missing_rust_runs"])
        print(f"  Rust 无状态行区间（非失配，iteration span/title 续段）: {seg}")
    al = result["input_alignment"]
    print(f"  输入面  : $0014 锁存失配 {al['latch_mismatch']}/{cov['comparable']}"
          + (f"（首 {al['latch_first']}）" if al["latch_first"] is not None else ""))
    if args.inputlog:
        print(f"            inputlog↔BizHawk p1 失配 {al['inputlog_mismatch']}"
              + (f"（首 {al['inputlog_first']}）" if al["inputlog_first"] is not None else ""))

    if first:
        print(f"  首个失配: f={first['f']}  "
              + "  ".join(f"{k}: rust={v['rust']:02X}/biz={v['biz']:02X}"
                          for k, v in first["diff"].items()))
        print(f"            全字段 rust={ {k: f'{v:02X}' for k, v in first['rust_all'].items()} }")
        print(f"            全字段 biz ={ {k: f'{v:02X}' for k, v in first['biz_all'].items()} }")
    else:
        print("  首个失配: 无")

    if run_rows:
        print(f"  失配区间: {len(run_rows)} 段（前 {min(args.md_limit, len(run_rows))} 段）")
        for r in run_rows[: args.md_limit]:
            span = f"f{r['from']}" if r["from"] == r["to"] else f"f{r['from']}..{r['to']}"
            print(f"    [{r['index']:>3}] {span:<18} 帧数 {r['frames']:<5} 字段 {','.join(r['fields'])}")
            for d in r["detail"][:4]:
                print("          " + " ".join(f"{k} rust={v['rust']:02X}/biz={v['biz']:02X}"
                                                for k, v in d["diff"].items()))
            if len(r["detail"]) > 4:
                print(f"          …（本段余 {len(r['detail']) - 4} 帧见完整表）")

    # ---------------- 落盘 ----------------
    if args.json_out:
        p = Path(args.json_out)
        p.parent.mkdir(parents=True, exist_ok=True)
        p.write_text(json.dumps(result, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
        print(f"  结构化结果 → {p}")

    if buckets:
        print("  分段（每 1000 帧）：")
        for b in buckets:
            if b["comparable"] == 0:
                continue
            print(f"    f{b['from']:<6}..f{b['to']:<6} 可比 {b['comparable']:<5} "
                  f"全同 {b['all_equal']:<5} 失配 {b['mismatch']}")

    if args.md_out:
        p = Path(args.md_out)
        p.parent.mkdir(parents=True, exist_ok=True)
        lines = [
            "# BizHawk(quickerNES) ↔ Rust RAM 面对拍（全表）",
            "",
            f"- BizHawk: `{bh_path}`（{cov['bizhawk_rows']} 行，f={cov['bizhawk_f_min']}..{cov['bizhawk_f_max']}）",
            f"- Rust: `{rs_path}`（{cov['rust_rows']} 行，f={cov['rust_f_min']}..{cov['rust_f_max']}）",
            f"- 窗口: f={start}..{end}；可比 {cov['comparable']}；全同 {ver['all_equal']}；"
            f"失配 {ver['mismatch_frames']}",
            f"- 逐字段失配: " + "、".join(f"{k}={v}" for k, v in per_field.items()),
            "",
            "## 失配帧（逐帧）",
            "",
            "| f | 字段 | Rust | BizHawk |",
            "|---:|---|---|---|",
        ]
        for f in sorted(mism):
            for k, (rb, rv) in sorted(mism[f].items()):
                lines.append(f"| {f} | {k} | `{rb:02X}` | `{rv:02X}` |")
        lines += ["", "## 分段统计（每 1000 帧）", "",
                  "| 起 | 止 | 可比 | 全同 | 失配 |", "|---:|---:|---:|---:|---:|"]
        for b in buckets:
            lines.append(f"| {b['from']} | {b['to']} | {b['comparable']} "
                         f"| {b['all_equal']} | {b['mismatch']} |")
        lines += ["", "## 失配区间（聚类）", "",
                  "| # | 起 | 止 | 帧数 | 字段 |", "|---:|---:|---:|---:|---|"]
        for r in run_rows:
            lines.append(f"| {r['index']} | {r['from']} | {r['to']} | {r['frames']} "
                         f"| {', '.join(r['fields'])} |")
        if result["missing_rust_runs"]:
            lines += ["", "## Rust 无状态行区间（非失配）", ""]
            lines += [f"- {a}" if a == b else f"- {a}..{b}" for a, b in result["missing_rust_runs"]]
        p.write_text("\n".join(lines) + "\n", encoding="utf-8")
        print(f"  完整区间表 → {p}")

    return 1 if (args.fail_on_mismatch and mism) else 0


if __name__ == "__main__":
    raise SystemExit(main())
