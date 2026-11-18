#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""tools/movie_ref/pixel_shift_probe.py —— 像素分歧的**平移探针**（诊断，不是判定）。

用途：当 `compare_pixels.py` 报某采集点有分歧时，判断分歧属于哪一类：
  ① 内容真分歧（任何整数平移都消不掉）；
  ② 整屏/局部**相位差**（某个 (dx,dy) 平移后几乎全同 ⇒ 屏幕相对 emu 偏了像素）；
  ③ 只有 sprite 区分歧（BG 全同、残余恰好在 sprite 包围盒内）。

实现：numpy 逐 (dx,dy) ∈ [-max_shift, max_shift]² 做重叠区比对，统计**全同像素数**与残差；
最后报告零平移与最优平移的对照。**不替代判定**：判定仍是 diff_visual.diff_frames（白名单外零容差）。

用法：
    python3 tools/movie_ref/pixel_shift_probe.py \
        --oracle-dir shots --rust-dir /tmp/rustshots-full \
        --name-re 'eien86-6725M_' --max-shift 3
"""
from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import diff_visual  # noqa: E402

FRAME_RE = re.compile(r"_f(\d{5})_")


def load_rgba(path: Path):
    import numpy as np

    w, h, _c, pix = diff_visual.decode_png(path.read_bytes())
    return np.frombuffer(pix, dtype=np.uint8).reshape(h, w, 4)


def eq_shift(a, b, dx: int, dy: int) -> tuple[int, int]:
    """把 a(x,y) 与 b(x-dx, y-dy) 在**重叠区**上比，返回 (全同像素数, 重叠像素数)。

    注意重叠区随平移缩小（dx=1 时少一列），故残差口径必须用「重叠区 − 全同」，
    不能拿整屏像素数去减（否则会把"被平移挤出去的那一列"误算成分歧）。
    """
    h, w = a.shape[:2]
    x0, x1 = max(0, dx), min(w, w + dx)
    y0, y1 = max(0, dy), min(h, h + dy)
    if x0 >= x1 or y0 >= y1:
        return 0, 0
    sa = a[y0:y1, x0:x1]
    sb = b[y0 - dy:y1 - dy, x0 - dx:x1 - dx]
    return int((sa == sb).all(axis=2).sum()), int(sa.shape[0] * sa.shape[1])


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description="像素分歧平移探针（诊断用）")
    ap.add_argument("--oracle-dir", required=True)
    ap.add_argument("--rust-dir", required=True)
    ap.add_argument("--name-re", default=".*")
    ap.add_argument("--max-shift", type=int, default=3)
    ap.add_argument("--only-diff", action="store_true", help="只探有分歧的点（复用 diff_frames 判定）")
    ap.add_argument("--json-out", default=None)
    args = ap.parse_args(argv)

    name_re = re.compile(args.name_re)

    def collect(d: Path):
        out = {}
        for p in sorted(d.glob("*.png")):
            if not name_re.search(p.name):
                continue
            m = FRAME_RE.search(p.name)
            if m:
                out[int(m.group(1))] = p
        return out

    od, rd = Path(args.oracle_dir), Path(args.rust_dir)
    oracle, rust = collect(od), collect(rd)
    frames = sorted(set(oracle) & set(rust))
    wl = []  # 严格：不放行任何白名单

    rows = []
    for f in frames:
        if args.only_diff:
            ok, _detail, _tol = diff_visual.diff_frames(
                oracle[f].name, oracle[f].read_bytes(), rust[f].read_bytes(), wl, [])
            if ok:
                continue
        a = load_rgba(oracle[f])
        b = load_rgba(rust[f])
        if a.shape != b.shape:
            rows.append({"f": f, "error": f"尺寸不一致 {a.shape} vs {b.shape}"})
            continue
        total = a.shape[0] * a.shape[1]
        z_eq, z_n = eq_shift(a, b, 0, 0)
        best = (-1.0, 0, 0, 0, 0)  # (全同率, dx, dy, 全同数, 重叠数)
        for dy in range(-args.max_shift, args.max_shift + 1):
            for dx in range(-args.max_shift, args.max_shift + 1):
                if dx == 0 and dy == 0:
                    continue
                e, n = eq_shift(a, b, dx, dy)
                rate = (e / n) if n else 0.0
                if rate > best[0]:
                    best = (rate, dx, dy, e, n)
        rows.append({"f": f, "total": total,
                     "zero_eq": z_eq, "zero_diff": total - z_eq,
                     "best_eq": best[3], "best_overlap": best[4],
                     "best_diff": best[4] - best[3],
                     "best_dx": best[1], "best_dy": best[2]})

    print(f"== 平移探针：{od} × {rd}（{len(rows)} 点，max_shift={args.max_shift}）==")
    print(f"{'帧':>6} {'零平移分歧':>10} {'最优(dx,dy)':>11} {'重叠区残差':>10}  判读")
    for r in rows:
        if "error" in r:
            print(f"{r['f']:>6} {'—':>10} {r['error']:>14}")
            continue
        if r["zero_diff"] == 0:
            verdict = "零平移全同"
        elif r["best_diff"] == 0:
            verdict = f"纯相位差（平移后全同）"
        elif r["best_diff"] * 20 < r["zero_diff"]:
            verdict = "以相位差为主"
        else:
            verdict = "内容分歧"
        print(f"{r['f']:>6} {r['zero_diff']:>10} "
              f"({r['best_dx']:>2},{r['best_dy']:>2}){'':>4} "
              f"{r['best_diff']:>5}/{r['best_overlap']:<6} {verdict}")

    if args.json_out:
        p = Path(args.json_out)
        p.parent.mkdir(parents=True, exist_ok=True)
        p.write_text(json.dumps(rows, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
        print(f"→ {p}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
