#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""tools/l4_diff_run.py —— L4 真正对拍（**Rust vs 模拟器**，像素零容差）。

这是「真正的对拍」：一边是 Rust 实现（game-core FrameEngine→render_frame 真实渲染帧），
另一边是 Mesen2 模拟器执行同一 ROM 同一输入脚本的截图。**非模拟器 vs 模拟器**。

流程（工具优先——单一 DSL 解析器 + 输入单源纪律）：
1. cases/ 为 L4 对拍用例**权威源**（三要素格式：①输入脚本+②待加载存档+③对拍时长，
   含 snapshot/endFrame 语义）——只读 cases/<name>.txt（唯一前缀名可缩写），不读
   oracle/scenarios/（同名旧定义=双源假对拍）；
2. 用 scenario_run.parse_scenario（**唯一 DSL 解析器**）解析 → scenario_run.expand_schedule
   （**唯一逐帧 joy1 展开**，press/hold/press_len/对向键后写覆盖）；
3. 写调度表（与 Lua 侧 PLAN.schedule 同源同产物）→ 跑 Rust bin rust_screenshot
   （FrameEngine→render_frame）→ shots_rust/ 真实截图；
4. 跑模拟器（tools/scenario_run.py run <cases 路径>，headless Xvfb+Mesen）→ shots/ 截图；
5. 逐 capture 像素对拍（PIL，白名单外零容差）→ 报告 + 失配帧 diff 图。

用法：
    python3 tools/l4_diff_run.py run case-001 [--timeout N] [--skip-emu]
    python3 tools/l4_diff_run.py diff case-001   # 只对拍已有截图

三 sha 锚定（H4 防假绿——报告必须可证"对的是哪次输入/哪个 ROM/哪些产物"）：
    l4report.json 落 `scenario_sha256`（场景 DSL 文件 sha）+ `rom_sha256`
    （rom/Metroid (USA).nes 实测 sha）+ `artifacts`（各截图 sha，键=shots_rust/<f> | shots/<f>）。
    复用旧产物（--skip-rust/--skip-emu/腿失败回落/diff）前**校验锚**（sched_sha256 调度 hash
    为主锚 + scenario/rom 锚）：不符或无锚 → **拒跑 exit 2**（不静默复用旧截图出假绿）。
    每条腿成功生成产物后落 <side 产物目录>/<name>.l4prov.json（复用校验的锚存证）。

输出：
    shots_rust/<name>_f<NNNNN>_<tag>.png       Rust 真实渲染帧
    shots_rust/<name>.l4report.json           对拍结果（结构化，含三 sha 锚）
    shots_rust/<name>_f<NNNNN>_<tag>.diff.png 失配帧可视化（Rust|Emu|Diff）
"""
from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "tools"))

# 按钮位映射唯一定义=scenario_run.BUTTON_BITS（task-2 输入单源——本文件不再复制位表）

CASES_DIR = ROOT / "cases"
SHOTS_EMU = ROOT / "shots"
SHOTS_RUST = ROOT / "shots_rust"
ROM = ROOT / "rom" / "Metroid (USA).nes"  # ROM 身份锚（D3；sha 见 AGENTS.md）

# 三 sha 锚键名（task-3 产 / task-5 matrix 消——键名契约勿改）
ANCHOR_KEYS = ("scenario_sha256", "rom_sha256", "sched_sha256")


def case_path_of(name: str) -> Path:
    """对拍用例解析（task-2 cases/ 权威）：cases/<name>.txt 或唯一前缀 cases/<name>-*.txt。

    只读 cases/（L4 对拍用例权威源=三要素格式）；**不回落 oracle/scenarios/**——同名
    1800 帧旧定义与 cases/ 1200 帧权威定义并存，回落即两侧跑不同用例=假对拍。
    """
    p = Path(name)
    if p.is_file():
        return p
    direct = CASES_DIR / f"{name}.txt"
    if direct.is_file():
        return direct
    matches = sorted(set(CASES_DIR.glob(f"{name}-*.txt")) | set(CASES_DIR.glob(f"{name}_*.txt")))
    if len(matches) == 1:
        return matches[0]
    if not matches:
        raise FileNotFoundError(f"找不到对拍用例 {name!r}（试过 {direct} 与 cases/{name}-*.txt）")
    raise FileNotFoundError(f"对拍用例 {name!r} 前缀不唯一: {[m.name for m in matches]}")


def load_pil():
    from PIL import Image  # noqa
    import numpy as np  # noqa
    return Image, np


def schedule_text(name: str, frames: int, joy: dict, caps: list) -> str:
    """调度表文本（**唯一**序列化面）——.sched 落盘与调度 hash 同源同 bytes。"""
    lines = [f"name {name}", f"frames {frames}"]
    for f in sorted(joy):
        lines.append(f"joy {f} {joy[f]:02X}")
    for f, tag in caps:
        lines.append(f"capture {f} {tag}")
    return "\n".join(lines) + "\n"


def write_schedule(name: str, frames: int, joy: dict, caps: list, path: Path):
    """展开结果落 .sched（Rust 侧消费）——joy=scenario_run.expand_schedule 产物（同源）。"""
    path.write_text(schedule_text(name, frames, joy, caps),
                    encoding="utf-8", newline="\n")  # 禁 CRLF 翻译：sched hash 按 LF 计


def sha256_file(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def compute_anchors(scen_path: Path, sched_text: str) -> dict:
    """三 sha 锚 + 调度 hash（H4）：scenario（DSL 文件）/rom（实测 ROM）/sched（展开调度）。

    ROM 缺失=fail-loud（l4report 必须锚 ROM sha256，D3 ROM=最终真相，不容无锚报告）。
    """
    if not ROM.is_file():
        raise FileNotFoundError(f"缺 ROM 锚文件 {ROM}——l4report 须锚 rom_sha256，拒跑")
    return {
        "scenario_sha256": sha256_file(scen_path),
        "rom_sha256": sha256_file(ROM),
        "sched_sha256": hashlib.sha256(sched_text.encode("utf-8")).hexdigest(),
    }


def prov_path(side: str, name: str) -> Path:
    """复用产物锚存证路径：rust 腿在 shots_rust/，emu 腿在 shots/（随产物走）。"""
    base = SHOTS_RUST if side == "rust" else SHOTS_EMU
    return base / f"{name}.l4prov.json"


def write_prov(side: str, name: str, anchors: dict) -> None:
    """腿成功生成产物后落锚存证（复用校验的对账依据）。"""
    doc = {"side": side, "name": name, **{k: anchors[k] for k in ANCHOR_KEYS}}
    p = prov_path(side, name)
    p.parent.mkdir(parents=True, exist_ok=True)
    p.write_text(json.dumps(doc, ensure_ascii=False, indent=2) + "\n",
                 encoding="utf-8", newline="\n")


def check_reuse(side: str, name: str, anchors: dict) -> str | None:
    """复用旧产物前锚校验（H4）：返回错误描述（拒跑理由）或 None（放行）。

    调度 hash（sched_sha256）为主锚——不符=输入调度已变，旧截图不可再代表当前用例；
    scenario/rom 锚同检（快照语义/ROM 变更同样使旧产物失效）。无锚存证=无锚可对 → 拒跑。
    """
    p = prov_path(side, name)
    if not p.is_file():
        return (f"{side} 侧产物无锚（缺 {p.name}）——无锚可对，无法验证调度 hash；"
                f"拒跑（先完整跑一次该用例生成带锚产物）")
    try:
        prov = json.loads(p.read_text(encoding="utf-8"))
    except Exception as exc:  # 损坏锚=不可信
        return f"{side} 侧锚存证损坏（{p.name}: {exc}）——拒跑"
    for k in ANCHOR_KEYS:
        got = prov.get(k)
        want = anchors[k]
        if not isinstance(got, str) or got.lower() != want.lower():
            return (f"{side} 侧复用校验失败：{k} 不符（产物 {str(got)[:16]}… vs 当前 "
                    f"{want[:16]}…）——输入已变，旧产物拒复用（防假绿）")
    return None


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


def run_emu(name: str, timeout: float, scen_path: Path) -> bool:
    # cases/ 权威定义**直传路径**（禁止按名回落 oracle/scenarios——同名旧定义=双源假对拍）
    cmd = [sys.executable, str(ROOT / "tools" / "scenario_run.py"), "run", str(scen_path),
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


def do_diff(name: str, anchors: dict) -> dict:
    Image, np = load_pil()
    SHOTS_RUST.mkdir(parents=True, exist_ok=True)
    rust_caps = SHOTS_RUST / f"{name}.rust.captures.json"
    if not rust_caps.is_file():
        print(f"[diff] 缺 Rust manifest {rust_caps}", file=sys.stderr)
        sys.exit(1)
    man = json.loads(rust_caps.read_text(encoding="utf-8"))
    results = []
    artifacts = {}  # 三 sha 锚之三：各截图 sha（键=仓库逻辑相对路径）
    npass = 0
    for c in man["captures"]:
        fname = c["file"]
        rust_png = SHOTS_RUST / fname
        emu_png = SHOTS_EMU / fname
        tag = c["tag"]
        frame = c["frame"]
        if rust_png.is_file():
            artifacts[f"shots_rust/{fname}"] = sha256_file(rust_png)
        if emu_png.is_file():
            artifacts[f"shots/{fname}"] = sha256_file(emu_png)
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
    report = {"name": name, **{k: anchors[k] for k in ANCHOR_KEYS},
              "artifacts": artifacts,
              "captures": len(results), "pass": npass,
              "fail": len(results) - npass, "gap": man.get("gap", False),
              "results": results}
    (SHOTS_RUST / f"{name}.l4report.json").write_text(
        json.dumps(report, ensure_ascii=False, indent=2), encoding="utf-8")
    return report


def print_report(rep: dict):
    print("\n" + "=" * 78)
    print(f"L4 对拍结果（**Rust vs 模拟器**，像素零容差）：{rep['name']}")
    print("=" * 78)
    print(f"锚：scenario={rep.get('scenario_sha256', '?')[:16]}… "
          f"rom={rep.get('rom_sha256', '?')[:16]}… "
          f"sched={rep.get('sched_sha256', '?')[:16]}… "
          f"产物 {len(rep.get('artifacts', {}))} 件")
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
    ap.add_argument("--skip-emu", action="store_true",
                    help="跳过模拟器运行（复用已有 shots/——复用前校验锚，不符/无锚 exit 2）")
    ap.add_argument("--skip-rust", action="store_true",
                    help="跳过 Rust 运行（复用已有 shots_rust/——复用前校验锚，不符/无锚 exit 2）")
    args = ap.parse_args()

    import importlib
    sr = importlib.import_module("scenario_run")
    try:
        scen_path = case_path_of(args.name)
    except FileNotFoundError as exc:
        print(f"[!] {exc}", file=sys.stderr)
        sys.exit(1)
    plan = sr.parse_scenario(scen_path)
    name = plan.get("name") or args.name
    frames = int(plan["frames"])
    joy = sr.expand_schedule(plan)  # 唯一展开源（task-2 输入单源——Rust/Lua 同产物）
    caps = sorted((int(c["frame"]), c["tag"]) for c in plan.get("captures", []))
    print(f"[plan] {name}: frames={frames} 非零输入帧={len(joy)} 采集点={len(caps)}")

    # 三 sha 锚（H4）：scenario/rom/sched——报告与复用校验同源
    try:
        anchors = compute_anchors(scen_path, schedule_text(name, frames, joy, caps))
    except FileNotFoundError as exc:
        print(f"[!] {exc}", file=sys.stderr)
        sys.exit(2)
    print(f"[anchor] scenario={anchors['scenario_sha256'][:16]}… "
          f"rom={anchors['rom_sha256'][:16]}… sched={anchors['sched_sha256'][:16]}…")

    if args.action == "run":
        # fail-fast：所有复用腿先过锚校验再动手（不静默复用旧产物出假绿）
        if args.skip_rust:
            err = check_reuse("rust", name, anchors)
            if err:
                print(f"[!] --skip-rust {err}", file=sys.stderr)
                sys.exit(2)
        if args.skip_emu:
            err = check_reuse("emu", name, anchors)
            if err:
                print(f"[!] --skip-emu {err}", file=sys.stderr)
                sys.exit(2)
        sched = SHOTS_RUST / f"{name}.sched"
        SHOTS_RUST.mkdir(parents=True, exist_ok=True)
        write_schedule(name, frames, joy, caps, sched)
        print(f"[sched] → {sched}")
        if not args.skip_rust:
            if not run_rust(name, sched, SHOTS_RUST):
                sys.exit(1)
            write_prov("rust", name, anchors)  # 腿成功 → 落锚存证
        if not args.skip_emu:
            if run_emu(name, args.timeout, scen_path):
                write_prov("emu", name, anchors)
            else:
                # 腿失败回落复用=同属复用路径 → 同样过锚校验（不静默）
                err = check_reuse("emu", name, anchors)
                if err:
                    print(f"[!] 模拟器运行失败，且回落复用未过锚校验——{err}", file=sys.stderr)
                    sys.exit(2)
                print("[!] 模拟器运行失败——回落复用已有 shots/"
                      "（锚校验通过：三 sha 与当前一致；产物未更新，prov 不重写）")
    else:  # diff：纯复用双侧产物 → 双侧过锚校验
        for side in ("rust", "emu"):
            err = check_reuse(side, name, anchors)
            if err:
                print(f"[!] diff {err}", file=sys.stderr)
                sys.exit(2)

    rep = do_diff(name, anchors)
    print_report(rep)
    sys.exit(0 if rep["fail"] == 0 else 1)


if __name__ == "__main__":
    main()
