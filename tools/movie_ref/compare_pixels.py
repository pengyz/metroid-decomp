#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""tools/movie_ref/compare_pixels.py —— 录像口径 L4 像素面对拍（Mesen2 腿 ↔ Rust 腿）。

**不改判定语义**：逐帧像素比走 `tools/diff_visual.diff_frames()`——与 L4 门禁同一实现
（白名单外零容差；本工具默认 `wl=[]`/`gaps=[]`，即比门禁更严：不放行任何白名单矩形）。

配对规则：两侧目录按文件名里的帧号 `_fNNNNN_` 对齐（同一帧号即同一采集点），
不看 tag——tag 只是同一帧的不同命名习惯。

用法（路径全部参数化）：
    python3 tools/movie_ref/compare_pixels.py \
        --oracle-dir shots --rust-dir /tmp/rustshots-full \
        --json-out analysis/traces/pixel-compare.json \
        --md-out analysis/traces/pixel-compare.md
"""
from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import diff_visual  # noqa: E402  单源：PNG 解码与像素判定只在 diff_visual

FRAME_RE = re.compile(r"_f(\d{5})_")


def collect(dirpath: Path, name_re: "re.Pattern[str]") -> dict[int, Path]:
    out: dict[int, Path] = {}
    for p in sorted(dirpath.glob("*.png")):
        if not name_re.search(p.name):
            continue
        m = FRAME_RE.search(p.name)
        if not m:
            continue
        f = int(m.group(1))
        if f in out:
            raise SystemExit(f"[pixels] {dirpath} 里帧 {f} 有多个 PNG：{out[f].name} / {p.name}")
        out[f] = p
    return out


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description="录像 L4 像素面对拍（复用 diff_visual 判定）")
    ap.add_argument("--oracle-dir", required=True, help="emu 腿 PNG 目录（Mesen2/BizHawk 采）")
    ap.add_argument("--rust-dir", required=True, help="Rust 腿 PNG 目录（rust_screenshot capture）")
    ap.add_argument("--use-whitelist", action="store_true",
                    help="启用 gates/visual_whitelist.json（默认禁用=严格零容差）")
    ap.add_argument("--name-re", default=".*",
                    help="只取文件名匹配该正则的 PNG（默认全取；目录里混他场景时用它筛）")
    ap.add_argument("--json-out", default=None)
    ap.add_argument("--md-out", default=None)
    ap.add_argument("--md-limit", type=int, default=20, help="stdout 明细行上限")
    ap.add_argument("--cluster-gap", type=int, default=10,
                    help="失配采集点聚类空隙（采集点步长 10，故默认 10 = 相邻采集点同段）")
    args = ap.parse_args(argv)

    od, rd = Path(args.oracle_dir), Path(args.rust_dir)
    for p in (od, rd):
        if not p.is_dir():
            raise SystemExit(f"[pixels] 目录不存在: {p}")

    name_re = re.compile(args.name_re)
    oracle = collect(od, name_re)
    rust = collect(rd, name_re)
    if not oracle:
        raise SystemExit(f"[pixels] {od} 无 PNG")
    if not rust:
        raise SystemExit(f"[pixels] {rd} 无 PNG")

    wl = diff_visual.load_whitelist() if args.use_whitelist else []
    gap_map = diff_visual.load_gap_map() if args.use_whitelist else []

    frames = sorted(set(oracle) & set(rust))
    only_oracle = sorted(set(oracle) - set(rust))
    only_rust = sorted(set(rust) - set(oracle))

    rows = []
    for f in frames:
        gname, rname = oracle[f].name, rust[f].name
        ok, detail, tol = diff_visual.diff_frames(gname, oracle[f].read_bytes(),
                                                  rust[f].read_bytes(), wl, gap_map)
        m = re.search(r"账外 (\d+) 像素", detail)
        dpix = int(m.group(1)) if m else (0 if ok else -1)
        rows.append({"f": f, "ok": ok, "tolerated": tol, "diff_px": dpix,
                     "oracle": gname, "rust": rname, "detail": detail})

    bad = [r for r in rows if not r["ok"]]
    zero = [r for r in rows if r["ok"]]

    # 采集点聚类（相邻采集点空隙 <= cluster_gap 归同段）
    runs = []
    for r in bad:
        if runs and r["f"] - runs[-1][-1]["f"] <= args.cluster_gap:
            runs[-1].append(r)
        else:
            runs.append([r])

    result = {
        "inputs": {"oracle_dir": str(od), "rust_dir": str(rd),
                   "whitelist_enabled": bool(args.use_whitelist)},
        "points": len(frames),
        "zero_px": len(zero),
        "diff_px_points": len(bad),
        "only_oracle": only_oracle,
        "only_rust": only_rust,
        "first_diff": ({"f": bad[0]["f"], "diff_px": bad[0]["diff_px"],
                        "detail": bad[0]["detail"]} if bad else None),
        "runs": [{"from": g[0]["f"], "to": g[-1]["f"], "points": len(g),
                  "max_diff_px": max(x["diff_px"] for x in g),
                  "frames": [x["f"] for x in g]} for g in runs],
        "rows": rows,
    }

    print("== 录像 L4 像素面对拍（复用 tools/diff_visual.diff_frames）==")
    print(f"  emu 腿 : {od}  {len(oracle)} PNG")
    print(f"  Rust   : {rd}  {len(rust)} PNG")
    print(f"  配对   : {len(frames)} 点   仅 emu: {len(only_oracle)}   仅 Rust: {len(only_rust)}")
    if only_oracle:
        print(f"           仅 emu 帧: {only_oracle[:20]}")
    if only_rust:
        print(f"           仅 Rust 帧: {only_rust[:20]}")
    print(f"  0 像素 : {len(zero)}/{len(frames)}")
    print(f"  有分歧 : {len(bad)}/{len(frames)}")
    if result["first_diff"]:
        print(f"  首个分歧采集点: f={result['first_diff']['f']}  {result['first_diff']['detail']}")
    else:
        print("  首个分歧采集点: 无（全部 0 像素）")
    if runs:
        print(f"  分歧采集点聚类: {len(runs)} 段")
        for i, g in enumerate(runs[:args.md_limit], 1):
            print(f"    [{i:>2}] f{g[0]['f']}..f{g[-1]['f']}  点数 {len(g)}  "
                  f"最大分歧 {max(x['diff_px'] for x in g)} px")
    shown = [r for r in rows if not r["ok"]][:args.md_limit]
    if shown:
        print("  分歧明细（前 %d 点）:" % len(shown))
        for r in shown:
            print(f"    f{r['f']}: {r['detail']}")

    if args.json_out:
        p = Path(args.json_out); p.parent.mkdir(parents=True, exist_ok=True)
        p.write_text(json.dumps(result, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
        print(f"  结构化结果 → {p}")

    if args.md_out:
        p = Path(args.md_out); p.parent.mkdir(parents=True, exist_ok=True)
        lines = ["# 录像 L4 像素面对拍（逐采集点全表）", "",
                 f"- emu 腿: `{od}`（{len(oracle)} PNG）",
                 f"- Rust: `{rd}`（{len(rust)} PNG）",
                 f"- 配对 {len(frames)} 点；0 像素 {len(zero)}；有分歧 {len(bad)}",
                 f"- 判定实现: `tools/diff_visual.diff_frames()`，"
                 f"白名单={'启用' if args.use_whitelist else '禁用（严格零容差）'}",
                 "", "| 帧 | 分歧像素 | 结果 |", "|---:|---:|---|"]
        for r in rows:
            lines.append(f"| {r['f']} | {r['diff_px']} | {'0px 全同' if r['ok'] else '分歧'} |")
        p.write_text("\n".join(lines) + "\n", encoding="utf-8")
        print(f"  逐点全表 → {p}")

    return 1 if bad else 0


if __name__ == "__main__":
    raise SystemExit(main())
