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
    python3 tools/l4_diff_run.py run-all [--timeout N] [--skip-emu] [--skip-rust]
        # 批量矩阵（Task 5·H4）：遍历 cases/*.txt **全用例** → 用例×采集点矩阵；
        # 产物 shots_rust/matrix.json（结构化）+ shots_rust/matrix.md（人读表）；
        # 矩阵三态 match|diff|blocked，**blocked 计入分母、永不计 pass**
        # （分母=用例×采集点总数）——防假绿硬约束；每格标 provenance
        # （fresh|reused——复用/新跑诚实标注）；snapshot 用例从快照起跑
        # （snapshot none=上电）。exit 0=全 match；1=有 diff/blocked；2=fail-loud。
    python3 tools/l4_diff_run.py align <name> [--frames N] [--bundle <前缀>] \
            [--emu-trace <path>] [--residual-class <标签>]
        # 对齐验收（Task 4·H1）：snapshot 装载后前 N 帧 Rust↔emu 快进**逐字节** diff；
        # 「加载成功≠对齐成功」防线——残差=0 或**显式登记**（exit 0/1/2=对齐/登记/拒裁）；
        # --bundle=构造不对齐 case 实验面（验收③：换快照探验收有效性）。

三 sha 锚定（H4 防假绿——报告必须可证"对的是哪次输入/哪个 ROM/哪些产物"）：
    l4report.json 落 `scenario_sha256`（场景 DSL 文件 sha）+ `rom_sha256`
    （rom/Metroid (USA).nes 实测 sha）+ `artifacts`（各截图 sha，键=shots_rust/<f> | shots/<f>）。
    复用旧产物（--skip-rust/--skip-emu/腿失败回落/diff）前**校验锚**（sched_sha256 调度 hash
    为主锚 + scenario/rom 锚）：不符或无锚 → **拒跑 exit 2**（不静默复用旧截图出假绿）。
    每条腿成功生成产物后落 <side 产物目录>/<name>.l4prov.json（复用校验的锚存证）。
    run-all 矩阵**每行**同锚（scenario_sha256 + rom_sha256 + 产物 sha——键名契约同 l4report）。

输出：
    shots_rust/<name>_f<NNNNN>_<tag>.png       Rust 真实渲染帧
    shots_rust/<name>.l4report.json           对拍结果（结构化，含三 sha 锚）
    shots_rust/<name>_f<NNNNN>_<tag>.diff.png 失配帧可视化（Rust|Emu|Diff）
    shots_rust/matrix.json + shots_rust/matrix.md   run-all 矩阵（结构化+人读表）

拒跑面（fail-loud，绝不空跑出绿/假对拍/崩溃假绿——2026-10-26 终审 findings 封堵）：
    · I1 采集点分母对账：0 采集点（run/diff）或产物采集点（captures∪blocked）与 DSL
      capture 数缺/多不对账 ⇒ exit 2——results=[] 会 fail=0 exit 0『✅ 全部一致』=空跑出绿；
    · I2 输入对称性：plan 含 trigger/nav（运行时条件输入仅模拟器侧叠加执行，Rust 无对应面）
      ⇒ 全 action exit 2——两侧跑不同输入=假对拍，绝不静默；
    · C1 崩溃语义：run-all 任何未捕获异常 ⇒ exit 2 拒跑（与拒跑同码——exit 1 会被 gate 读作
      "跑完"去判矩阵，崩溃残留旧 matrix 被判过=假绿后门）；矩阵落盘走 temp+rename 原子写。
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "tools"))

import diff_visual  # noqa: E402 —— 白名单唯一例外消费面单源（load_whitelist/in_rect；Task 6）

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


def schedule_text(name: str, frames: int, joy: dict, caps: list,
                  snapshot: str | None = None) -> str:
    """调度表文本（**唯一**序列化面）——.sched 落盘与调度 hash 同源同 bytes。

    snapshot=<prefix>（Task 4）：有快照才落行（非快照用例字节不变——锚不漂移）；
    `snapshot none`=显式无存档由 cases/ 语义消解（parse_scenario→bundle_load=None），
    调度表不落行（上电起跑同语义）。
    """
    lines = [f"name {name}", f"frames {frames}"]
    if snapshot:
        lines.append(f"snapshot {snapshot}")
    for f in sorted(joy):
        lines.append(f"joy {f} {joy[f]:02X}")
    for f, tag in caps:
        lines.append(f"capture {f} {tag}")
    return "\n".join(lines) + "\n"


def write_schedule(name: str, frames: int, joy: dict, caps: list, path: Path,
                   snapshot: str | None = None):
    """展开结果落 .sched（Rust 侧消费）——joy=scenario_run.expand_schedule 产物（同源）。"""
    path.write_text(schedule_text(name, frames, joy, caps, snapshot),
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


def run_rust(name: str, sched: Path, out_dir: Path,
             state_out: Path | None = None, bundle: str | None = None) -> bool:
    out_dir.mkdir(parents=True, exist_ok=True)
    cmd = ["cargo", "run", "-q", "-p", "game-core", "--bin", "rust_screenshot",
           "--", "--schedule", str(sched), "--out", str(out_dir)]
    if bundle:  # 【Task 4】显式 --bundle（CLI 面（验收①）——覆盖调度表 snapshot 行）
        cmd += ["--bundle", bundle]
    if state_out:  # 【Task 4】状态行输出（对齐验收输入）
        cmd += ["--state-out", str(state_out)]
    print(f"[rust] {' '.join(cmd)}")
    r = subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True)
    # cargo 的 warning 走 stderr；只在真正失败时报错
    if r.returncode != 0:
        print(r.stdout)
        print(r.stderr, file=sys.stderr)
        print(f"[rust] 失败 rc={r.returncode}", file=sys.stderr)
        return False
    for line in r.stderr.splitlines():
        if line.startswith("[rust]") or line.startswith("[bundle]"):
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


def diff_pair(rust_png: Path, emu_png: Path, diff_png: Path | None, np, Image,
              wl: list[dict] | None = None):
    """逐像素对拍（白名单外零容差）。返回 dict。

    白名单=gates/visual_whitelist.json（**唯一**例外清单——AGENTS §5；条目
    {file: glob, rect:[x,y,w,h], reason, approved_by}）：glob 命中本帧 ∧ 像素落在
    rect 内 ⇒ 排除（记 whitelist_hits，状态可判 match）；**白名单外 1 像素差异即 diff**。
    wl=None ⇒ 读 gates/visual_whitelist.json（load_whitelist 单源=tools/diff_visual.py）。
    **不得**另立 gap_map/账内容忍类排除面（防容忍面增殖——Task 6 裁定，白名单=唯一例外）。"""
    if wl is None:
        wl = diff_visual.load_whitelist()
    rules = [r for r in wl if Path(rust_png.name).match(r.get("file", ""))]
    r = np.asarray(Image.open(rust_png).convert("RGB"), dtype=np.int16)
    e = np.asarray(Image.open(emu_png).convert("RGB"), dtype=np.int16)
    if r.shape != e.shape:
        return {"match": False, "reason": f"尺寸不同 rust{r.shape} vs emu{e.shape}",
                "diff_pixels": -1, "total_pixels": int(r.shape[0] * r.shape[1]),
                "whitelist_hits": 0}
    d = np.abs(r - e).sum(2)
    raw_mask = d > 0
    total = int(raw_mask.size)
    excl = np.zeros(raw_mask.shape, dtype=bool)
    for rule in rules:
        rx, ry, rw, rh = (int(v) for v in rule["rect"])
        excl[max(ry, 0):max(ry + rh, 0), max(rx, 0):max(rx + rw, 0)] = True
    whitelist_hits = int((raw_mask & excl).sum())
    mask = raw_mask & ~excl
    diff_pixels = int(mask.sum())
    if mask.any() and diff_png is not None:
        # 可视化：左=Rust、中=Emu（**自然游戏色**），右=差异诊断掩码（绿=差异，**非游戏色**）。
        # 顶栏标注各片用途，避免把诊断掩码误读为调色板偏差。
        h, w = mask.shape
        gap, hdr = 4, 20
        r_img = np.asarray(Image.open(rust_png).convert("RGB"))
        e_img = np.asarray(Image.open(emu_png).convert("RGB"))
        # 差异片：非差异=暗灰(保留场景语境)，差异=亮绿(诊断色，游戏里不会有纯绿)，
        # 白名单内已排除=橙（例外显形，不参与 diff 计数）
        diag = np.full((h, w, 3), 32, dtype=np.uint8)
        diag[mask] = (0, 255, 0)
        diag[raw_mask & excl] = (255, 165, 0)
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
                f"DIFF 掩码(绿=差异,橙=白名单内已排除) 账外{diff_pixels}px/白名单{whitelist_hits}px",
                fill=(0, 255, 0))
        im.save(diff_png)
    return {"match": diff_pixels == 0, "diff_pixels": diff_pixels, "total_pixels": total,
            "whitelist_hits": whitelist_hits}


# ---------------- 对齐验收（Task 4 · H1：加载成功≠对齐成功防线） ----------------

# 状态面规范序（frame_face 唯一编码三面——zp/iram/wram）；额外面按名序接尾
FACE_ORDER = ("zp", "iram", "wram")


def load_state_rows(path) -> dict:
    """帧状态行装载（`{"f":N,"w":{face:hex,…}}` JSONL——oracle/l3-dump 帧 trace 同形
    **单一格式**）。空/无行=fail-loud（空跑不得绿）。返回 {frame: {face: 大写hex}}。"""
    path = Path(path)
    rows: dict = {}
    for ln, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        line = line.strip()
        if not line:
            continue
        try:
            d = json.loads(line)
        except json.JSONDecodeError as exc:
            raise ValueError(f"{path}:{ln} 行 JSON 解析失败: {exc}") from exc
        f = d.get("f")
        w = d.get("w")
        if not isinstance(f, int) or not isinstance(w, dict):
            raise ValueError(f"{path}:{ln} 行缺 f/w 面（帧状态行契约）")
        if f in rows:
            raise ValueError(f"{path}:{ln} 帧 {f} 重复行（行集唯一契约）")
        rows[f] = {str(k): str(v).upper() for k, v in w.items()}
    if not rows:
        raise ValueError(f"{path} 无状态行（空跑不得绿——对齐验收无输入）")
    return rows


def align_states(rust_rows: dict, emu_rows: dict, n: int,
                 name: str = "", residual_class: str | None = None) -> dict:
    """对齐验收：restore 后**前 N 帧逐字节**（各面 hex 逐字节）Rust↔emu 快进对齐。

    「加载成功≠对齐成功」防线：装载成功不等于对齐——逐帧逐字节裁，分歧**绝不**裁
    aligned；残差=0（residual.status="none"）或**显式登记**（residual.status=
    "registered"，首分歧逐字节定位+分歧帧集；`residual_class`=登记残差类标签，
    如 savestate 重放经典残差类（恢复相位面）——bundle-first §1 实证类）。

    缺行（采样窗缺失）显式登记（missing_*），不冒充对齐；单侧面（如 emu 腿无 wram）
    记入 faces 覆盖（比较面=交集）。无可比面=拒（ValueError，空跑不得绿）。
    """
    def face_order(keys):
        known = [f for f in FACE_ORDER if f in keys]
        return known + sorted(k for k in keys if k not in FACE_ORDER)

    r_faces = set().union(*(w.keys() for w in rust_rows.values())) if rust_rows else set()
    e_faces = set().union(*(w.keys() for w in emu_rows.values())) if emu_rows else set()
    common = face_order(r_faces & e_faces)
    if not common:
        raise ValueError(f"无可比状态面（rust={sorted(r_faces)} emu={sorted(e_faces)}）"
                         f"——对齐验收无输入，拒裁（空跑不得绿）")

    per_frame = []
    first_div = None
    missing_rust, missing_emu = [], []
    aligned = diverged = 0
    residual_frames = []
    for f in range(1, n + 1):
        if f not in rust_rows:
            per_frame.append({"f": f, "status": "missing_rust",
                              "reason": "Rust 侧无此帧状态行（采样窗/缺口）"})
            missing_rust.append(f)
            residual_frames.append(f)
            continue
        if f not in emu_rows:
            per_frame.append({"f": f, "status": "missing_emu",
                              "reason": "emu 侧无此帧状态行（采样窗缺失）"})
            missing_emu.append(f)
            residual_frames.append(f)
            continue
        entry = {"f": f, "status": "aligned", "diff_bytes": 0}
        r, e = rust_rows[f], emu_rows[f]
        for face in common:
            a, b = r.get(face, ""), e.get(face, "")
            if len(a) != len(b):
                entry.update({"status": "diverged",
                              "reason": f"length mismatch face={face} rust={len(a)//2}B "
                                        f"emu={len(b)//2}B"})
                if first_div is None:
                    first_div = {"f": f, "face": face, "off": None,
                                 "rust": f"{len(a)//2}B", "emu": f"{len(b)//2}B"}
                break
            ndiff = 0
            for i in range(0, len(a), 2):
                if a[i:i + 2] == b[i:i + 2]:
                    continue
                ndiff += 1
                if first_div is None or first_div["f"] > f:
                    first_div = {"f": f, "face": face, "off": i // 2,
                                 "rust": a[i:i + 2], "emu": b[i:i + 2]}
                if entry["status"] == "aligned":
                    entry.update({"status": "diverged",
                                  "first": {"face": face, "off": i // 2,
                                            "rust": a[i:i + 2], "emu": b[i:i + 2]}})
            entry["diff_bytes"] = entry.get("diff_bytes", 0) + ndiff
        if entry["status"] == "aligned":
            aligned += 1
        else:
            diverged += 1
            residual_frames.append(f)
        per_frame.append(entry)

    registered = bool(residual_frames)
    residual = {
        "status": "registered" if registered else "none",
        "class": residual_class,
        "frames": residual_frames,
        "first": first_div,
        "note": ("残差显式登记（不静默容忍）——加载成功≠对齐成功" if registered
                 else "残差=0（前 N 帧逐字节对齐）"),
    }
    return {
        "name": name,
        "n": n,
        "faces": {"common": common,
                  "rust_only": face_order(r_faces - e_faces),
                  "emu_only": face_order(e_faces - r_faces)},
        "frames_compared": aligned + diverged,
        "aligned": aligned,
        "diverged": diverged,
        "missing_rust": missing_rust,
        "missing_emu": missing_emu,
        "per_frame": per_frame,
        "first_divergence": first_div,
        "residual": residual,
        "verdict": "residual-registered" if registered else "aligned",
    }


def print_align_report(rep: dict) -> None:
    """对齐验收输出（逐帧 diff——残差显式）。"""
    print("\n" + "=" * 78)
    print(f"对齐验收（加载成功≠对齐成功防线）：{rep['name']}（前 N={rep['n']} 帧）")
    print("=" * 78)
    print(f"比较面：common={rep['faces']['common']} "
          f"rust_only={rep['faces']['rust_only']} emu_only={rep['faces']['emu_only']}"
          f"（单侧面=覆盖登记，不参与逐字节比对）")
    print(f"{'帧':>6} {'状态':<16} {'逐字节 diff':<44}")
    print("-" * 78)
    for fr in rep["per_frame"]:
        st = fr["status"]
        icon = {"aligned": "✅对齐", "diverged": "❌分歧",
                "missing_rust": "⛔Rust缺行", "missing_emu": "⛔emu缺行"}.get(st, st)
        if st == "diverged" and "first" in fr:
            x = fr["first"]
            detail = (f"{x['face']}+0x{x['off']:03X} rust={x['rust']} emu={x['emu']}"
                      f"（共 {fr.get('diff_bytes', '?')}B）")
        else:
            detail = fr.get("reason", "")
        print(f"{fr['f']:>6} {icon:<16} {detail:<44}")
    print("-" * 78)
    print(f"汇总：对齐 {rep['aligned']} · 分歧 {rep['diverged']} · "
          f"Rust 缺行 {len(rep['missing_rust'])}{rep['missing_rust'][:8]} · "
          f"emu 缺行 {len(rep['missing_emu'])}{rep['missing_emu'][:8]}")
    res = rep["residual"]
    if res["status"] == "none":
        print("残差：0（前 N 帧逐字节对齐——加载成功且对齐成功）")
    else:
        fd = res.get("first")
        fd_s = (f"首分歧 f={fd['f']} {fd['face']}+0x{fd['off']:03X} "
                f"rust={fd['rust']} emu={fd['emu']}" if fd and fd.get("off") is not None
                else f"首分歧 {fd}")
        print(f"残差：**显式登记**（class={res.get('class') or '未标注'}）{fd_s}；"
              f"分歧/缺行帧集 {res['frames'][:16]}")
        print("      ——加载成功≠对齐成功（残差登记不等于容忍；账清路径=残差类定位）")
    print("=" * 78)


# ---------------- 拒跑守卫（I1 分母对账 / I2 输入对称性 / C1 原子落盘） ----------------

def atomic_write_text(path: Path, text: str) -> None:
    """原子落盘（temp + rename）：崩溃绝不留半成品顶名产物（C1——gate 矩阵新鲜度配套）。

    matrix.json 是 gate_l4 的裁定输入：半成品/截断 JSON 不得顶名旧产物（旧产物宁可留着
    被 gate 新鲜度拒裁，也不可以"半个新产物"冒充本轮落盘）。
    """
    tmp = path.with_name(path.name + ".tmp")
    tmp.write_text(text, encoding="utf-8", newline="\n")
    os.replace(tmp, path)


def reconcile_denominator(man: dict, caps: list) -> None:
    """run 产物采集点对账 DSL 分母（I1）：产物点集（captures∪blocked）须与 DSL 采集点集逐一相等。

    分母唯一基准=DSL capture 数（capture 三要素之③）：缺（DSL 有产物无——含空产物=空跑
    results=[] → fail=0 exit 0『✅ 全部一致』出绿）/多（产物有 DSL 无=锚外结果漏 diff）一律
    ValueError 拒跑——缺/多即 fail，绝不静默缩分母/漏 diff。
    """
    prod = {(int(c.get("frame", -1)), c.get("tag")) for c in (man.get("captures") or [])}
    prod |= {(int(c.get("frame", -1)), c.get("tag")) for c in (man.get("blocked") or [])}
    dsl = {(int(f), t) for f, t in caps}
    missing, extra = sorted(dsl - prod), sorted(prod - dsl)
    if missing or extra:
        raise ValueError(
            f"产物采集点与 DSL 采集点不对账（缺 {missing} 多 {extra}）——run 产物采集点数须=DSL "
            f"capture 数，缺/多即 fail（防空跑出绿/缩分母/锚外结果）")


def check_symmetric_inputs(plan: dict, name: str) -> None:
    """两侧输入对称性守卫（I2 防漂移假对拍）：plan 含 trigger/nav ⇒ ValueError 拒跑。

    trigger/nav 是**运行时条件输入**（scenario_capture.lua 依 RAM 条件叠加按键；scenario_run
    parse 语义），Rust 侧 Plan 无对应执行面（expand_schedule 只展开静态 press/hold）——含
    trigger/nav 的用例两侧跑**不同输入**=假对拍（2026-10-26 终审 I2 漂移陷阱）。fail-loud
    拒跑（全 action：run/diff/align/run-all），绝不静默两侧跑不同输入。
    """
    n_trig = len(plan.get("triggers") or [])
    n_nav = len(plan.get("navs") or [])
    if n_trig or n_nav:
        raise ValueError(
            f"用例 {name} 含 trigger×{n_trig}/nav×{n_nav}——trigger/nav 两侧输入不对称"
            f"（运行时条件输入仅模拟器侧叠加执行，Rust 无对应面）不支持对拍，fail-loud 拒跑"
            f"（绝不静默两侧跑不同输入出假对拍）")


def do_diff(name: str, anchors: dict, caps: list) -> dict:
    """逐采集点对拍出 l4report（分母基准=DSL caps——I1 拒空跑/对账在前，绝不 exit 0 出绿）。

    ValueError（fail-loud 拒跑）：① caps 空（0 采集点——results=[] 会 fail=0『✅ 全部一致』
    空跑出绿）；② 产物采集点与 DSL 采集点缺/多不对账（reconcile_denominator）。"""
    if not caps:
        raise ValueError(f"用例 {name} 无采集点（DSL capture 全缺）——0 采集点空跑不得绿，拒跑（I1）")
    Image, np = load_pil()
    SHOTS_RUST.mkdir(parents=True, exist_ok=True)
    rust_caps = SHOTS_RUST / f"{name}.rust.captures.json"
    if not rust_caps.is_file():
        print(f"[diff] 缺 Rust manifest {rust_caps}", file=sys.stderr)
        sys.exit(1)
    man = json.loads(rust_caps.read_text(encoding="utf-8"))
    reconcile_denominator(man, caps)  # I1：产物采集点==DSL 采集点（缺/多即拒）
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


# ---------------- 批量对拍矩阵（Task 5 · H4：run-all 用例×采集点） ----------------

# 矩阵三态硬约束：每格 match|diff|blocked——**blocked 计入分母、永不计 pass**（防假绿）。
# l4report 的 error（缺截图）/未知第四态一并归 blocked 并保留原因（三态外不许漏网）。
CELL_STATE_OF = {"match": "match", "diff": "diff", "blocked": "blocked", "error": "blocked"}


def matrix_row(name: str, case_file: str, caps: list, *,
               anchors: dict, legs: dict, report: dict | None = None,
               snapshot: str | None = None, error: str | None = None) -> dict:
    """用例×采集点 → 矩阵行（cells 以 **DSL 采集点为分母基准**——产物缺/跑失败也占格）。

    - 三态映射：l4report status match|diff|blocked 原样；error/未知 → blocked（原因保留）；
      DSL 采集点无对拍结果 → blocked（"无对拍结果"）——**分母不因产物缺而缩水**；
    - 每格标 `provenance`（腿来源 fresh|reused|none——复用/新跑诚实标注，不静默混用）；
    - 每行锚三 sha（键名=l4report 契约，Task 3 键名勿改）：`scenario_sha256` +
      `rom_sha256` + 产物 sha（`artifacts`，键=shots_rust/<f> | shots/<f>）。
    error≠None（腿失败/复用拒跑）→ 全格 blocked 记原因（绝不静默跳过、不缩分母）。
    """
    results, extra = {}, []
    for r in (report or {}).get("results", []):
        key = (int(r.get("frame", -1)), r.get("tag"))
        if key in results:
            extra.append(dict(r))
        else:
            results[key] = r
    cells = []
    for f, tag in caps:
        r = results.pop((f, tag), None)
        if error is not None or r is None:
            reason = (f"用例未跑成：{error}" if error is not None
                      else "无对拍结果（产物/结果缺——分母不缩水，不计 pass）")
            cells.append({"frame": f, "tag": tag, "state": "blocked",
                          "provenance": dict(legs), "file": None,
                          "diff_pixels": None, "total_pixels": None, "reason": reason})
            continue
        status = r.get("status")
        reason = r.get("reason")
        if status not in CELL_STATE_OF:
            reason = f"未知结果态 {status!r} → 归 blocked（三态硬约束，不计 pass）"
        cells.append({"frame": f, "tag": tag, "state": CELL_STATE_OF.get(status, "blocked"),
                      "provenance": dict(legs), "file": r.get("file"),
                      "diff_pixels": r.get("diff_pixels"),
                      "total_pixels": r.get("total_pixels"), "reason": reason})
    for r in list(results.values()) + extra:
        # DSL 外/重复结果（锚校验下不应出现）——显式占格防漏 diff
        r = dict(r)
        status = r.get("status")
        cells.append({"frame": r.get("frame"), "tag": r.get("tag"),
                      "state": CELL_STATE_OF.get(status, "blocked"),
                      "provenance": dict(legs), "file": r.get("file"),
                      "diff_pixels": r.get("diff_pixels"),
                      "total_pixels": r.get("total_pixels"),
                      "reason": (r.get("reason") or "") +
                                "（锚外结果：DSL 无此采集点——显式占格防漏 diff）" or None})
    cells.sort(key=lambda c: (c["frame"] if isinstance(c["frame"], int) else 1 << 30,
                              str(c["tag"])))
    return {
        "case": name,
        "case_file": str(case_file),
        "scenario_sha256": anchors["scenario_sha256"],
        "rom_sha256": anchors["rom_sha256"],
        "sched_sha256": anchors["sched_sha256"],
        "artifacts": dict((report or {}).get("artifacts") or {}),
        "snapshot": snapshot,
        "report": f"shots_rust/{name}.l4report.json" if report else None,
        "legs": dict(legs),
        "error": error,
        "cells": cells,
    }


def build_matrix(rows: list) -> dict:
    """汇总矩阵——**分母=用例×采集点总数（含 blocked）**；pass=match 数：
    blocked 计入分母、**永不计 pass**（防假绿硬约束）。
    verdict="pass" 当且仅当 cells_total>0 且 pass==cells_total（空矩阵兜底不假绿）。
    """
    match = diff = blocked = 0
    roms = set()
    for row in rows:
        roms.add(row["rom_sha256"])
        for c in row["cells"]:
            if c["state"] == "match":
                match += 1
            elif c["state"] == "diff":
                diff += 1
            else:
                blocked += 1
    if len(roms) > 1:
        raise ValueError(f"rows rom_sha256 锚不一致 {sorted(roms)}——矩阵须锚同一 ROM")
    total = match + diff + blocked
    return {
        "generated_by": "tools/l4_diff_run.py run-all",
        "rom_sha256": next(iter(roms)) if roms else None,
        "cases_total": len(rows),
        "cells_total": total,
        "denominator": total,
        "denominator_note": ("分母=用例×采集点总数（含 blocked；blocked 计入分母、"
                             "永不计 pass——防假绿硬约束）"),
        "match": match, "diff": diff, "blocked": blocked,
        "pass": match,
        "pass_rate": f"{match}/{total}",
        "verdict": "pass" if total > 0 and match == total else "not-pass",
        "rows": rows,
    }


def matrix_md(m: dict) -> str:
    """人读表（matrix.md）：分母规则置顶 + 用例行（三 sha 锚）+ 采集点格明细（三态+来源）。"""
    lines = ["# L4 对拍矩阵（用例×采集点）", ""]
    lines.append("- 生成：`python3 tools/l4_diff_run.py run-all`（tools/l4_diff_run.py）")
    lines.append("- 三 sha 锚（每行）：`scenario_sha256` + `rom_sha256` + 产物 sha"
                 "（`artifacts`，逐件 sha 见 matrix.json `rows[].artifacts`）；"
                 f"rom_sha256=`{m['rom_sha256']}`")
    lines.append(f"- **分母 = 用例×采集点 = {m['denominator']}**（含 blocked {m['blocked']}）"
                 "——**blocked 计入分母、永不计 pass**（防假绿硬约束）")
    lines.append(f"- 汇总：match {m['match']} · diff {m['diff']} · blocked {m['blocked']}"
                 f" ｜ **pass {m['pass_rate']}**（pass=match 数）")
    lines.append(f"- 总评：{'✅ 全 match（verdict=pass）' if m['verdict'] == 'pass' else '❌ verdict=not-pass（blocked/diff 均在分母内且不计 pass）'}")
    lines.append("")
    lines.append("## 用例行（三 sha 锚定）")
    lines.append("")
    lines.append("| 用例 | scenario_sha256 | rom_sha256 | sched_sha256 | 产物sha | "
                 "snapshot | 腿来源 rust/emu | match/diff/blocked |")
    lines.append("|---|---|---|---|---|---|---|---|")
    for r in m["rows"]:
        nm = sum(1 for c in r["cells"] if c["state"] == "match")
        nd = sum(1 for c in r["cells"] if c["state"] == "diff")
        nb = sum(1 for c in r["cells"] if c["state"] == "blocked")
        legs = f"{r['legs']['rust']}/{r['legs']['emu']}"
        snap = r["snapshot"] or "none(上电起跑)"
        lines.append(f"| {r['case']} | `{r['scenario_sha256']}` | `{r['rom_sha256']}` | "
                     f"`{r['sched_sha256']}` | {len(r['artifacts'])} 件 | {snap} | "
                     f"{legs} | {nm}/{nd}/{nb} |")
    lines.append("")
    lines.append("## 采集点格（三态：match | diff | blocked；来源=腿 provenance）")
    lines.append("")
    lines.append("| 用例 | frame | tag | 状态 | 来源 rust/emu | 差异像素/总 | 说明 |")
    lines.append("|---|---|---|---|---|---|---|")
    for r in m["rows"]:
        for c in r["cells"]:
            icon = {"match": "✅match", "diff": "❌diff", "blocked": "⛔blocked"}[c["state"]]
            prov = f"{c['provenance']['rust']}/{c['provenance']['emu']}"
            dp = "" if c["diff_pixels"] is None else f"{c['diff_pixels']}/{c['total_pixels']}"
            reason = (c["reason"] or "").replace("|", "\\|")
            lines.append(f"| {r['case']} | {c['frame']} | {c['tag']} | {icon} | {prov} | "
                         f"{dp} | {reason} |")
    lines.append("")
    return "\n".join(lines) + "\n"


def execute_case(*, name: str, scen_path: Path, frames: int, joy: dict, caps: list,
                 snap: str | None, anchors: dict, skip_rust: bool, skip_emu: bool,
                 timeout: float) -> dict:
    """跑双腿+对拍（`run` / `run-all` 共用核心）——**不 exit**，返回记账 dict：
    {"legs": {"rust": "fresh|reused|none", "emu": …}, "report": l4report|None,
     "error": str|None, "exit_code": int|None}。

    复用路径（--skip-* / emu 腿失败回落）一律先过 check_reuse 锚校验——**不静默复用**；
    error≠None 时 exit_code 沿袭 run 语义（2=拒跑/锚不可信、1=腿运行失败）：
    run 动作直接 exit（行为不变），run-all 记 blocked 格继续扫（分母不缩水）。
    """
    legs = {"rust": "none", "emu": "none"}
    # fail-fast：所有复用腿先过锚校验再动手（不静默复用旧产物出假绿）
    if skip_rust:
        err = check_reuse("rust", name, anchors)
        if err:
            return {"legs": legs, "report": None, "error": f"--skip-rust {err}",
                    "exit_code": 2}
    if skip_emu:
        err = check_reuse("emu", name, anchors)
        if err:
            return {"legs": legs, "report": None, "error": f"--skip-emu {err}",
                    "exit_code": 2}
    sched = SHOTS_RUST / f"{name}.sched"
    SHOTS_RUST.mkdir(parents=True, exist_ok=True)
    write_schedule(name, frames, joy, caps, sched, snapshot=snap)  # 与锚同源字节
    print(f"[sched] → {sched}")
    if skip_rust:
        legs["rust"] = "reused"
    else:
        if not run_rust(name, sched, SHOTS_RUST):
            return {"legs": legs, "report": None, "error": "Rust 腿运行失败",
                    "exit_code": 1}
        write_prov("rust", name, anchors)  # 腿成功 → 落锚存证
        legs["rust"] = "fresh"
    if skip_emu:
        legs["emu"] = "reused"
    else:
        if run_emu(name, timeout, scen_path):
            write_prov("emu", name, anchors)
            legs["emu"] = "fresh"
        else:
            # 腿失败回落复用=同属复用路径 → 同样过锚校验（不静默）
            err = check_reuse("emu", name, anchors)
            if err:
                return {"legs": legs, "report": None,
                        "error": f"模拟器运行失败，且回落复用未过锚校验——{err}",
                        "exit_code": 2}
            print("[!] 模拟器运行失败——回落复用已有 shots/"
                  "（锚校验通过：三 sha 与当前一致；产物未更新，prov 不重写）")
            legs["emu"] = "reused"
    if not (SHOTS_RUST / f"{name}.rust.captures.json").is_file():
        return {"legs": legs, "report": None,
                "error": f"缺 Rust manifest（{name}.rust.captures.json）——产物不全",
                "exit_code": 1}
    try:
        rep = do_diff(name, anchors, caps)
    except ValueError as exc:  # I1 拒空跑/分母对账失败——fail-loud exit 2（不空跑出绿）
        return {"legs": legs, "report": None,
                "error": f"对拍分母对账拒跑——{exc}", "exit_code": 2}
    return {"legs": legs, "report": rep, "error": None, "exit_code": None}


def do_run_all(args) -> int:
    """批量矩阵入口（C1 崩溃语义封堵）：任何未捕获异常 ⇒ **exit 2 拒跑**。

    崩溃必须与拒跑同码（exit 2）——exit 1 会被 gate_l4 读作『跑完』去判 matrix.json，
    崩溃残留的旧/committed 矩阵被判过=假绿后门（2026-10-26 终审 C1）。矩阵落盘走
    temp+rename 原子写（半成品不顶名），gate 侧再以新鲜度校验确保崩溃绝不判旧矩阵。
    """
    try:
        return _do_run_all(args)
    except Exception as exc:  # noqa: BLE001 —— 全量兜底（含 OSError/JSON/断言类）
        print(f"[!] run-all 内部异常（fail-loud 拒跑 exit 2——不留半成品/旧矩阵当裁定输入）: "
              f"{type(exc).__name__}: {exc}", file=sys.stderr)
        return 2


def _do_run_all(args) -> int:
    """批量矩阵：遍历 cases/*.txt **全用例** → 用例×采集点矩阵（三态）。

    分母=用例×采集点总数（含 blocked）——blocked **永不计 pass**（防假绿硬约束）。
    用例级失败（腿失败/复用拒跑）不中断扫掠：该用例全格 blocked 记原因（分母不缩水），
    继续下例；exit 0 当且仅当全格 match（verdict=pass）。fail-loud（exit 2）：cases/ 空/
    用例解析失败/同名冲突/无采集点/trigger-nav 输入不对称/ROM 缺——分母完整性受损或
    两侧输入不对称一律拒跑（不静默缩分母、不静默两侧跑不同输入）。
    """
    import importlib
    sr = importlib.import_module("scenario_run")
    case_files = sorted(CASES_DIR.glob("*.txt"))
    if not case_files:
        print(f"[!] cases/ 无对拍用例（{CASES_DIR}）——空矩阵不得绿，拒跑", file=sys.stderr)
        return 2
    rows, seen = [], {}
    for cf in case_files:
        try:
            plan = sr.parse_scenario(cf)
        except Exception as exc:
            print(f"[!] 用例解析失败 {cf}——采集点不可知（分母完整性受损），fail-loud 拒跑: "
                  f"{exc}", file=sys.stderr)
            return 2
        name = plan.get("name") or cf.stem
        if name in seen:
            print(f"[!] 用例名重复 {name!r}（{seen[name].name} 与 {cf.name}）——"
                  f"产物/报告互相覆盖=矩阵失真，拒跑", file=sys.stderr)
            return 2
        seen[name] = cf
        try:
            check_symmetric_inputs(plan, name)  # I2：trigger/nav 两侧输入不对称 ⇒ 拒跑
        except ValueError as exc:
            print(f"[!] {exc}", file=sys.stderr)
            return 2
        frames = int(plan["frames"])
        joy = sr.expand_schedule(plan)  # 唯一展开源（task-2 输入单源）
        caps = sorted((int(c["frame"]), c["tag"]) for c in plan.get("captures", []))
        if not caps:
            print(f"[!] 用例 {name} 无采集点（capture/endFrame 全缺）——分母会静默缩水，"
                  f"拒跑", file=sys.stderr)
            return 2
        snap = (plan.get("bundle_load") or {}).get("path")
        sched_text = schedule_text(name, frames, joy, caps, snap)
        try:
            anchors = compute_anchors(cf, sched_text)
        except FileNotFoundError as exc:
            print(f"[!] {exc}", file=sys.stderr)
            return 2
        out = execute_case(name=name, scen_path=cf, frames=frames, joy=joy, caps=caps,
                           snap=snap, anchors=anchors, skip_rust=args.skip_rust,
                           skip_emu=args.skip_emu, timeout=args.timeout)
        try:
            case_file = str(cf.relative_to(ROOT))
        except ValueError:
            case_file = str(cf)
        row = matrix_row(name, case_file, caps, anchors=anchors, legs=out["legs"],
                         report=out["report"], snapshot=snap, error=out["error"])
        rows.append(row)
        if out["error"] is not None:
            print(f"[case] {name}: 未跑成（{out['error']}）——全格 blocked 记账（不缩分母）",
                  file=sys.stderr)
        else:
            n = {"match": 0, "diff": 0, "blocked": 0}
            for c in row["cells"]:
                n[c["state"]] += 1
            print(f"[case] {name}: rust={out['legs']['rust']} emu={out['legs']['emu']} "
                  f"snapshot={snap or 'none(上电起跑)'} → match {n['match']} diff {n['diff']} "
                  f"blocked {n['blocked']}")
    m = build_matrix(rows)
    SHOTS_RUST.mkdir(parents=True, exist_ok=True)
    out_json = SHOTS_RUST / "matrix.json"
    out_md = SHOTS_RUST / "matrix.md"
    # C1：temp+rename 原子落盘——崩溃绝不留半成品顶名矩阵（gate 新鲜度配套）
    atomic_write_text(out_json, json.dumps(m, ensure_ascii=False, indent=2) + "\n")
    atomic_write_text(out_md, matrix_md(m))
    print("\n" + "=" * 78)
    print("L4 对拍矩阵（用例×采集点）—— run-all（遍历 cases/）")
    print("=" * 78)
    print(f"用例 {m['cases_total']} · 采集点格 {m['cells_total']}"
          f"（**分母=用例×采集点总数，含 blocked {m['blocked']}**——blocked 永不计 pass）")
    print(f"match {m['match']} · diff {m['diff']} · blocked {m['blocked']} ｜ "
          f"pass {m['pass_rate']}")
    if m["verdict"] == "pass":
        print("总评：✅ 全部 match（L4 批量对拍通过）")
    else:
        print("总评：❌ 未全 match（diff/blocked 计入分母且不计 pass——防假绿）")
    print(f"矩阵 → {out_json} / {out_md}")
    print("=" * 78)
    return 0 if m["verdict"] == "pass" else 1


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("action", choices=["run", "diff", "align", "run-all"])
    ap.add_argument("name", nargs="?", default=None,
                    help="用例名/路径（run-all 不接——全量遍历 cases/）")
    ap.add_argument("--timeout", type=float, default=240.0)
    ap.add_argument("--skip-emu", action="store_true",
                    help="跳过模拟器运行（复用已有 shots/——复用前校验锚，不符/无锚 exit 2）")
    ap.add_argument("--skip-rust", action="store_true",
                    help="跳过 Rust 运行（复用已有 shots_rust/——复用前校验锚，不符/无锚 exit 2）")
    ap.add_argument("--bundle", default=None,
                    help="[align] 显式快照前缀（覆盖用例 snapshot——造不对齐 case 实验面（验收③））")
    ap.add_argument("--emu-trace", default=None,
                    help="[align] 用既有 emu 帧 trace（跳过模拟器重跑）")
    ap.add_argument("--frames", type=int, default=0,
                    help="[align] 对齐前 N 帧（缺省=用例 frames）")
    ap.add_argument("--residual-class", default=None,
                    help="[align] 残差登记类标签（如 'restore-phase/savestate-replay-class'）")
    args = ap.parse_args()

    if args.action == "run-all":
        # 批量矩阵（Task 5）：不接 name——遍历 cases/*.txt 全用例
        if args.name:
            print("[!] run-all 不接 name（全量遍历 cases/）", file=sys.stderr)
            sys.exit(2)
        sys.exit(do_run_all(args))
    if not args.name:
        print(f"[!] {args.action} 需要 name（用例名/路径）", file=sys.stderr)
        sys.exit(2)

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
    # I1：run/diff 拒空采集点（0 采集点 → results=[] 会 fail=0 exit 0『✅ 全部一致』=空跑出绿）
    if args.action in ("run", "diff") and not caps:
        print(f"[!] 用例 {name} 无采集点（capture/endFrame 全缺）——0 采集点空跑不得绿，拒跑（I1）",
              file=sys.stderr)
        sys.exit(2)
    # I2：trigger/nav 两侧输入不对称（Rust 无对应面）⇒ 全 action 拒跑（绝不静默两侧跑不同输入）
    try:
        check_symmetric_inputs(plan, name)
    except ValueError as exc:
        print(f"[!] {exc}", file=sys.stderr)
        sys.exit(2)
    # snapshot（Task 4）：用例 snapshot <prefix> 为单源；--bundle 显式覆盖（align 实验面）。
    case_snap = (plan.get("bundle_load") or {}).get("path")
    snap = case_snap
    if args.bundle:
        snap = args.bundle
        if args.action != "align":
            print("[!] --bundle 仅 align 实验面（run 用例 snapshot 为单源——双腿不得异快照）",
                  file=sys.stderr)
            sys.exit(2)
        if case_snap and case_snap != snap:
            print(f"[!] --bundle 覆盖用例 snapshot（{case_snap} → {snap}）——"
                  f"emu 腿仍跑用例快照：**人为不对齐 case**（验收③实验面）")
    print(f"[plan] {name}: frames={frames} 非零输入帧={len(joy)} 采集点={len(caps)} "
          f"snapshot={snap or 'none(上电起跑)'}")

    # 三 sha 锚（H4）：scenario/rom/sched——报告与复用校验同源（sched 文本含 snapshot 行）
    sched_text = schedule_text(name, frames, joy, caps, snap)
    try:
        anchors = compute_anchors(scen_path, sched_text)
    except FileNotFoundError as exc:
        print(f"[!] {exc}", file=sys.stderr)
        sys.exit(2)
    print(f"[anchor] scenario={anchors['scenario_sha256'][:16]}… "
          f"rom={anchors['rom_sha256'][:16]}… sched={anchors['sched_sha256'][:16]}…")

    if args.action == "align":
        # 【Task 4】snapshot 用例对齐验收：restore 后前 N 帧 Rust↔emu 快进**逐字节**对齐；
        # 「加载成功≠对齐成功」防线——残差=0 或**显式登记**（exit 1=残差已登记，非假绿）。
        n = args.frames or frames
        sched = SHOTS_RUST / f"{name}.sched"
        SHOTS_RUST.mkdir(parents=True, exist_ok=True)
        write_schedule(name, frames, joy, caps, sched, snapshot=snap)
        print(f"[sched] → {sched}")
        state_path = SHOTS_RUST / f"{name}.state.jsonl"
        if not run_rust(name, sched, SHOTS_RUST, state_out=state_path, bundle=snap):
            sys.exit(1)
        if args.emu_trace:
            emu_trace = Path(args.emu_trace)
            print(f"[align] emu 状态行=既有 trace {emu_trace}")
        else:
            if not run_emu(name, args.timeout, scen_path):
                print("[!] 模拟器腿失败——对齐验收无 emu 输入（无迹不裁，fail-loud）",
                      file=sys.stderr)
                sys.exit(2)
            emu_trace = Path(sr.TRACES_DIR) / f"{name}.frames.jsonl"
        try:
            rust_rows = load_state_rows(state_path)
            emu_rows = load_state_rows(emu_trace)
            rep = align_states(rust_rows, emu_rows, n, name=name,
                               residual_class=args.residual_class)
        except (OSError, ValueError) as exc:
            print(f"[!] 对齐验收输入失败（fail-loud）: {exc}", file=sys.stderr)
            sys.exit(2)
        rep.update({k: anchors[k] for k in ANCHOR_KEYS})
        out_json = SHOTS_RUST / f"{name}.align.json"
        out_json.write_text(json.dumps(rep, ensure_ascii=False, indent=2) + "\n",
                            encoding="utf-8")
        print_align_report(rep)
        print(f"[align] 报告 → {out_json}")
        sys.exit(0 if rep["verdict"] == "aligned" else 1)

    if args.action == "run":
        # 双腿+对拍（与 run-all 共用 execute_case——复用腿先过锚校验，不静默复用）
        out = execute_case(name=name, scen_path=scen_path, frames=frames, joy=joy,
                           caps=caps, snap=snap, anchors=anchors,
                           skip_rust=args.skip_rust, skip_emu=args.skip_emu,
                           timeout=args.timeout)
        if out["error"] is not None:
            print(f"[!] {out['error']}", file=sys.stderr)
            sys.exit(out["exit_code"] or 1)
        rep = out["report"]
    else:  # diff：纯复用双侧产物 → 双侧过锚校验
        for side in ("rust", "emu"):
            err = check_reuse(side, name, anchors)
            if err:
                print(f"[!] diff {err}", file=sys.stderr)
                sys.exit(2)
        try:
            rep = do_diff(name, anchors, caps)
        except ValueError as exc:  # I1 拒空跑/分母对账失败——fail-loud exit 2
            print(f"[!] {exc}", file=sys.stderr)
            sys.exit(2)

    print_report(rep)
    sys.exit(0 if rep["fail"] == 0 else 1)


if __name__ == "__main__":
    main()
