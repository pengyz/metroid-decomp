#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""tools/l4_diff_run.py —— L4 真正对拍（**Rust vs 模拟器**，像素零容差）。

这是「真正的对拍」：一边是 Rust 实现（game-core FrameEngine→render_frame 真实渲染帧），
另一边是 Mesen2 模拟器执行同一 ROM 同一输入脚本的截图。**非模拟器 vs 模拟器**。

流程（工具优先——单一 DSL 解析器纪律）：
1. 用 scenario_run.parse_scenario（**唯一 DSL 解析器**）解析 oracle/scenarios/<name>.txt；
2. 依 scenario_capture.lua 语义展开成逐帧 joy1 调度（press/hold + press_len + 按钮位映射）；
3. 写调度表 → 跑 Rust bin rust_screenshot（FrameEngine→render_frame）→ shots_rust/ 真实截图；
4. 跑模拟器（tools/scenario_run.py run <name>，headless Xvfb+Mesen）→ shots/ 截图；
5. 逐 capture 像素对拍（PIL，白名单外零容差）→ 报告 + 失配帧 diff 图。

用法：
    python3 tools/l4_diff_run.py run case-001-game-start-samus-interaction [--timeout N] [--skip-emu]
    python3 tools/l4_diff_run.py diff case-001-game-start-samus-interaction   # 只对拍已有截图

输出：
    shots_rust/<name>_f<NNNNN>_<tag>.png       Rust 真实渲染帧
    shots_rust/<name>.l4report.json           对拍结果（结构化）
    shots_rust/<name>_f<NNNNN>_<tag>.diff.png 失配帧可视化（Rust|Emu|Diff）
"""
from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "tools"))

# 按钮位映射（game-platform PadState / hardware.asm:89-96——A=bit7 … Right=bit0）
BUTTON_BITS = {
    "right": 0x01, "left": 0x02, "down": 0x04, "up": 0x08,
    "start": 0x10, "select": 0x20, "b": 0x40, "a": 0x80,
}

SHOTS_EMU = ROOT / "shots"
SHOTS_RUST = ROOT / "shots_rust"


def load_pil():
    from PIL import Image  # noqa
    import numpy as np  # noqa
    return Image, np


def build_schedule(plan: dict) -> tuple[int, dict, list]:
    """展开 DSL plan → (frames, {frame: joy1}, [(frame, tag)])。

    语义严格对齐 scenario_capture.lua：
      · hold <a-b> <btn>：帧 a..b（含）按住
      · press <f> <btn>：帧 f..f+press_len-1 按住（press_len 默认 2）
      · 同帧多按钮 OR 成 joy1；对向键冲突（left/right、up/down）后写覆盖（btnOrder 序）
    """
    frames = int(plan["frames"])
    press_len = int(plan.get("press_len") or 2)
    # 每帧记录 (frame -> {btn: order}) 以复现 btnOrder 冲突消解
    per_frame: dict[int, dict[str, int]] = {}
    seq = 0

    def add(a: int, b: int, btns: list[str]):
        nonlocal seq
        for f in range(a, b + 1):
            d = per_frame.setdefault(f, {})
            for btn in btns:
                seq += 1
                d[btn] = seq

    for e in plan.get("holds", []):
        add(int(e["from"]), int(e["to"]), e["buttons"])
    for e in plan.get("presses", []):
        add(int(e["frame"]), int(e["frame"]) + press_len - 1, e["buttons"])

    joy: dict[int, int] = {}
    for f, d in per_frame.items():
        # 冲突消解（对向键后写覆盖——btnOrder 小者被剔）
        if d.get("left") and d.get("right"):
            if d["left"] < d["right"]:
                d.pop("left")
            else:
                d.pop("right")
        if d.get("up") and d.get("down"):
            if d["up"] < d["down"]:
                d.pop("up")
            else:
                d.pop("down")
        bits = 0
        for btn in d:
            bits |= BUTTON_BITS[btn]
        if bits:
            joy[f] = bits

    caps = [(int(c["frame"]), c["tag"]) for c in plan.get("captures", [])]
    caps.sort()
    return frames, joy, caps


def write_schedule(name: str, frames: int, joy: dict, caps: list, path: Path):
    lines = [f"name {name}", f"frames {frames}"]
    for f in sorted(joy):
        lines.append(f"joy {f} {joy[f]:02X}")
    for f, tag in caps:
        lines.append(f"capture {f} {tag}")
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")


def run_rust(name: str, sched: Path, out_dir: Path) -> bool:
    out_dir.mkdir(parents=True, exist_ok=True)
    cmd = ["cargo", "run", "-q", "-p", "game-core", "--bin", "rust_screenshot",
           "--", "--schedule", str(sched), "--out", str(out_dir)]
    print(f"[rust] {' '.join(cmd)}")
    r = subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True)
    # cargo 的 warning 走 stderr；只在真正失败时报错
    if r.returncode != 0:
        print(r.stdout)
        print(r.stderr, file=sys.stderr)
        print(f"[rust] 失败 rc={r.returncode}", file=sys.stderr)
        return False
    for line in r.stderr.splitlines():
        if line.startswith("[rust]"):
            print(line)
    return True


def run_emu(name: str, timeout: float) -> bool:
    cmd = [sys.executable, str(ROOT / "tools" / "scenario_run.py"), "run", name,
           "--timeout", str(timeout)]
    print(f"[emu] {' '.join(cmd)}")
    r = subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True)
    print(r.stdout[-2000:] if r.stdout else "")
    if r.returncode != 0:
        print(r.stderr[-2000:], file=sys.stderr)
        print(f"[emu] 失败 rc={r.returncode}", file=sys.stderr)
        return False
    return True


def diff_pair(rust_png: Path, emu_png: Path, diff_png: Path | None, np, Image):
    """逐像素对拍（白名单外零容差）。返回 dict。"""
    r = np.asarray(Image.open(rust_png).convert("RGB"), dtype=np.int16)
    e = np.asarray(Image.open(emu_png).convert("RGB"), dtype=np.int16)
    if r.shape != e.shape:
        return {"match": False, "reason": f"尺寸不同 rust{r.shape} vs emu{e.shape}",
                "diff_pixels": -1, "total_pixels": int(r.shape[0] * r.shape[1])}
    d = np.abs(r - e).sum(2)
    mask = d > 0
    diff_pixels = int(mask.sum())
    total = int(mask.size)
    if diff_pixels and diff_png is not None:
        # 可视化：左=Rust、中=Emu（**自然游戏色**），右=差异诊断掩码（绿=差异，**非游戏色**）。
        # 顶栏标注各片用途，避免把诊断掩码误读为调色板偏差。
        h, w = mask.shape
        gap, hdr = 4, 20
        r_img = np.asarray(Image.open(rust_png).convert("RGB"))
        e_img = np.asarray(Image.open(emu_png).convert("RGB"))
        # 差异片：非差异=暗灰(保留场景语境)，差异=亮绿(诊断色，游戏里不会有纯绿)
        diag = np.full((h, w, 3), 32, dtype=np.uint8)
        diag[mask] = (0, 255, 0)
        vis = np.zeros((h + hdr, w * 3 + gap * 2, 3), dtype=np.uint8)
        vis[:] = (24, 24, 24)
        vis[hdr:, :w] = r_img
        vis[hdr:, w + gap:2 * w + gap] = e_img
        vis[hdr:, 2 * w + 2 * gap:] = diag
        im = Image.fromarray(vis)
        from PIL import ImageDraw
        dr = ImageDraw.Draw(im)
        dr.text((4, 5), "RUST (game-core, 自然色)", fill=(255, 255, 255))
        dr.text((w + gap + 4, 5), "EMU (Mesen2, 自然色)", fill=(255, 255, 255))
        dr.text((2 * w + 2 * gap + 4, 5),
                f"DIFF 掩码(绿=差异,非游戏色) {diff_pixels}px", fill=(0, 255, 0))
        im.save(diff_png)
    return {"match": diff_pixels == 0, "diff_pixels": diff_pixels, "total_pixels": total}


def do_diff(name: str) -> dict:
    Image, np = load_pil()
    SHOTS_RUST.mkdir(parents=True, exist_ok=True)
    rust_caps = SHOTS_RUST / f"{name}.rust.captures.json"
    if not rust_caps.is_file():
        print(f"[diff] 缺 Rust manifest {rust_caps}", file=sys.stderr)
        sys.exit(1)
    man = json.loads(rust_caps.read_text(encoding="utf-8"))
    results = []
    npass = 0
    for c in man["captures"]:
        fname = c["file"]
        rust_png = SHOTS_RUST / fname
        emu_png = SHOTS_EMU / fname
        tag = c["tag"]
        frame = c["frame"]
        if not rust_png.is_file():
            results.append({"frame": frame, "tag": tag, "match": False, "status": "error",
                            "reason": "缺 Rust 截图"})
            continue
        if not emu_png.is_file():
            results.append({"frame": frame, "tag": tag, "match": False, "status": "error",
                            "reason": "缺模拟器截图"})
            continue
        diff_png = SHOTS_RUST / f"{name}_f{frame:05}_{tag}.diff.png"
        res = diff_pair(rust_png, emu_png, diff_png, np, Image)
        res.update({"frame": frame, "tag": tag, "file": fname,
                    "status": "match" if res["match"] else "diff"})
        results.append(res)
        if res["match"]:
            npass += 1
    # 缺口阻塞的采集点（unported fail-loud——诚实记账，非伪造）
    for c in man.get("blocked", []):
        results.append({"frame": c["frame"], "tag": c["tag"], "match": False,
                        "status": "blocked", "reason": c.get("reason", "unported gap")})
    results.sort(key=lambda r: r["frame"])
    report = {"name": name, "captures": len(results), "pass": npass,
              "fail": len(results) - npass, "gap": man.get("gap", False),
              "results": results}
    (SHOTS_RUST / f"{name}.l4report.json").write_text(
        json.dumps(report, ensure_ascii=False, indent=2), encoding="utf-8")
    return report


def print_report(rep: dict):
    print("\n" + "=" * 78)
    print(f"L4 对拍结果（**Rust vs 模拟器**，像素零容差）：{rep['name']}")
    print("=" * 78)
    print(f"{'帧':>6} {'tag':<16} {'状态':<10} {'差异像素':>12} {'/总像素':>10}")
    print("-" * 78)
    for r in rep["results"]:
        st = r.get("status", "?")
        icon = {"match": "✅一致", "diff": "❌失配", "blocked": "⛔缺口", "error": "⚠错误"}.get(st, st)
        dp = r.get("diff_pixels", "-")
        tp = r.get("total_pixels", "-")
        reason = r.get("reason", "")
        extra = f"  {reason}" if reason else ""
        print(f"{r['frame']:>6} {r['tag']:<16} {icon:<10} {str(dp):>12} {str(tp):>10}{extra}")
    print("-" * 78)
    nm = sum(1 for r in rep["results"] if r.get("status") == "match")
    nd = sum(1 for r in rep["results"] if r.get("status") == "diff")
    nb = sum(1 for r in rep["results"] if r.get("status") == "blocked")
    ne = sum(1 for r in rep["results"] if r.get("status") == "error")
    print(f"汇总：一致 {nm} · 失配 {nd} · 缺口阻塞 {nb} · 错误 {ne}（共 {rep['captures']}）")
    if nd == 0 and nb == 0 and ne == 0:
        print("总评：✅ 全部一致（L4 帧对拍通过）")
    else:
        print(f"总评：❌ 未全一致（{nm}/{rep['captures']} 通过"
              + (f"，{nb} 采集点被 unported 缺口阻塞" if nb else "")
              + (f"，{nd} 采集点像素失配" if nd else "") + "）")
    print("=" * 78)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("action", choices=["run", "diff"])
    ap.add_argument("name")
    ap.add_argument("--timeout", type=float, default=240.0)
    ap.add_argument("--skip-emu", action="store_true", help="跳过模拟器运行（用已有 shots/）")
    ap.add_argument("--skip-rust", action="store_true", help="跳过 Rust 运行（用已有 shots_rust/）")
    args = ap.parse_args()

    import importlib
    sr = importlib.import_module("scenario_run")
    scen_path = sr.scenario_path_of(args.name)
    if not scen_path.is_file():
        print(f"[!] 场景不存在: {scen_path}", file=sys.stderr)
        sys.exit(1)
    plan = sr.parse_scenario(scen_path)
    name = plan.get("name") or args.name
    frames, joy, caps = build_schedule(plan)
    print(f"[plan] {name}: frames={frames} 非零输入帧={len(joy)} 采集点={len(caps)}")

    if args.action == "run":
        sched = SHOTS_RUST / f"{name}.sched"
        SHOTS_RUST.mkdir(parents=True, exist_ok=True)
        write_schedule(name, frames, joy, caps, sched)
        print(f"[sched] → {sched}")
        if not args.skip_rust:
            if not run_rust(name, sched, SHOTS_RUST):
                sys.exit(1)
        if not args.skip_emu:
            if not run_emu(name, args.timeout):
                print("[!] 模拟器运行失败——改用已有 shots/ 对拍", file=sys.stderr)

    rep = do_diff(name)
    print_report(rep)
    sys.exit(0 if rep["fail"] == 0 else 1)


if __name__ == "__main__":
    main()
