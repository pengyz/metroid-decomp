#!/usr/bin/env python3
"""房间批量扫描（room sweep）——对每个 (area, room) 生成临时用例、跑 L4 对拍、记账。

用法：
    python3 tools/movie_ref/room_sweep.py --areas 0 --rooms all \
        --frames 120 --captures 60,120 --out data/status/room_sweep_0.json

判据完全复用 `tools/l4_diff_run.py`（像素零容差＋既有视觉白名单），本工具**不**改判据、
不加白名单、不放宽阈值。它只负责：造用例 → 调对拍器 → 解析产物 → 记账 → 清理临时用例。

`ok` 的定义（严格）：两腿都跑完（无 error/blocked/timeout）**且**每个采集点 diff==0
**且**注入房号被引擎接受（`$005A` 在窗口内走完 F0→…→FF，`MainRoutine` 保持 3=GameEngine）。
任一不满足 ⇒ `ok=false`，明细落 `status`/`notes`/`mismatch`。

分母显式化：`summary` 给出 planned / ran / ok / mismatch / blocked / error / skipped
与每个 skip 的理由；分母永远是「计划扫的房数」，不是「跑成功的房数」。

幂等：同一 `--out` 重跑覆盖（原子写）。中断续跑：每房跑完即落盘，`--resume` 跳过已 ok 的房。
"""

from __future__ import annotations

import argparse
import concurrent.futures
import json
import os
import re
import signal
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
L4 = ROOT / "tools" / "l4_diff_run.py"
CASES = ROOT / "cases"
SHOTS_RUST = ROOT / "shots_rust"
TRACES = ROOT / "analysis" / "traces"

# ---------------------------------------------------------------- 常量（全部带证据锚）

AREA_NAMES = {0: "Brinstar", 1: "Norfair", 2: "Kraid", 3: "Tourian", 4: "Ridley"}

# 房流房数——`crates/game-data/src/assets.rs:171-199`（BR/NO/KR/TO/RI_ROOMS 的
# `slices(..., N)` 参数 N），数据源 `assets/metroid.pack`
# （`cargo run -q -p pack-tool -- list --pack assets/metroid.pack` 里的
# `area.{br,no,kr,to,ri}.room_streams`）。
ROOM_COUNTS = {0: 47, 1: 46, 2: 37, 3: 21, 4: 42}

# Rust 侧已移植区派发：`crates/game-core/src/world/room_draw.rs` 只在 area ∉ {0,1} 时
# `unported!`（区派发未移植）⇒ 2/3/4 区即使房流已在包内也必须跳过并记账。
PORTED_AREAS = (0, 1)
# entry_preset 已用实验定稿的区号。0 之外的区需要额外注入「区号 + 区指针块」
# （InArea=$74 与 CopyAreaPointers ; 07:C8B0），否则 emu 腿仍用旧区的
# struct/metatile 指针、rust 腿却按新区的资产表绘制 ⇒ 两腿数据源不一致，
# 得到的失配是工具产物而不是真差异。见 docs/findings/room-sweep.md §8.1。
PRESET_VERIFIED_AREAS = (0,)

# 注入帧：`press 310 start`（标题）→ `press 350 start`（START GAME）→ 07:C81D MoreInit
# 载起始房 → 07:C8D1 SamusInit 置 Timer3=$2C（07:C266 UpdateTimer 每 10 帧减 1）⇒ 440 帧
# 开场演出期间按键被吞（见 cases/case-001 头部注释）。900 帧 > 演出 440 + 标题/选单
# 输入 ~40，留足余量；case-001 首个有效动作在 f910。
ENTRY_FRAME = 900

ENTRY_PRESET_VERSION = "v1"


def _preset(room: int) -> list[tuple[int, int, str]]:
    """entry_preset v1 = `(地址, 值, 持久化)`。

    逐项理由（asm/引擎行为）见 docs/findings/room-sweep.md §2。`evolving` = 每帧重写；
    `invariant` = 只在注入帧写一次。
    """
    return [
        (0x005A, room, "invariant"),   # RoomNumber：SetupRoom(07:EA2B) 的输入
        (0x0049, 0x03, "invariant"),   # ScrollDir=RIGHT：ScrollRight(07:E6D2) 的方向前置
        (0x00FD, 0x02, "invariant"),   # ScrollX≠0：跳过 GetRoomNum(07:E720) 覆盖分支
        (0x0056, 0x01, "evolving"),    # DoorEntryStatus=1：ScrollDoor(07:E1F1) ⇒ 右滚（门闩）
    ]


# ---------------------------------------------------------------- 用例生成

def case_name(area: int, room: int) -> str:
    return f"_tmp_sweep_a{area}_r{room:02d}"


def build_case(area: int, room: int, frames: int, captures: list[int]) -> str:
    """生成临时用例文本。captures = 相对注入帧的偏移（落 DSL 绝对帧 = ENTRY_FRAME + c）。"""
    stem = case_name(area, room)
    lines = [
        f"name {stem}",
        "# room sweep 临时用例（tools/movie_ref/room_sweep.py 生成，跑完即删）",
        f"# area={area}({AREA_NAMES.get(area, '?')}) room={room} "
        f"entry=f{ENTRY_FRAME} preset={ENTRY_PRESET_VERSION}",
        f"frames {ENTRY_FRAME + frames}",
        # 载入链观测量：$005A 是否走完 F0→FF（房号被接受）、$001E 是否留在 3（GameEngine）。
        "watch 001E MainRoutine",
        "watch 005A RoomNumber",
        "watch 0049 ScrollDir",
        "watch 00FD ScrollX",
        "watch 0056 DoorEntryStatus",
        "snapshot none",
        "press 310 start",
        "press 350 start",
        f"capture {ENTRY_FRAME} pre",
    ]
    for addr, val, persist in _preset(room):
        lines.append(f"fixture {ENTRY_FRAME} {addr:04X} {val:02X} {persist}")
    for c in captures:
        lines.append(f"capture {ENTRY_FRAME + c} f{c}")
    lines.append(f"endFrame {ENTRY_FRAME + frames}")
    return "\n".join(lines) + "\n"


# PPU dump 布局（两腿同格式）：`$0000-$2FFF`（12KB：CHR 8KB + 名表 4KB）+ `$3F00-$3F1F`
# 调色板 32B + 尾 3B（scrollX/scrollY/ppuCtrl）。写入端：
# tools/scenario_capture.lua `dumpPpu`（emu）与 crates/game-core/src/bin/rust_screenshot.rs:466。
# 用它把「像素差」分解到层：CHR / 名表（房间数据）/ 调色板 / 滚动寄存器 / 剩余⇒精灵。
PPU_REGIONS = (
    ("chr", 0x0000, 0x2000),
    ("nt0", 0x2000, 0x2400),
    ("nt1", 0x2400, 0x2800),
    ("nt2", 0x2800, 0x2C00),
    ("nt3", 0x2C00, 0x3000),
    ("palette", 0x3000, 0x3020),
)
# 注意：dump 尾 3B（scrollX/scrollY/ppuCtrl）**不参与比对**——emu 侧 `emu.getState()` 的
# 字段名不匹配（tools/scenario_capture.lua `dumpPpu` 对 "ppu.scrollX"/"ppu.scroll_x" 双形
# 容错），实测恒为 0,0,0；拿它比对会伪造「滚动分歧」。rust 侧的真实值只作元数据记录。

# ---------------------------------------------------------------- 产物解析

def analyse_trace(stem: str) -> dict:
    """从 emu 帧级 trace 取「房号被接受」证据（$005A 回到 $FF、$001E 保持 3）。"""
    p = TRACES / f"{stem}.frames.jsonl"
    out: dict = {"trace": p.relative_to(ROOT).as_posix() if p.exists() else None,
                 "accepted": None, "mr_at_entry": None, "mr_at_end": None,
                 "room_at_end": None, "back_to_ff_frame": None, "room_frames": []}
    if not p.exists():
        return out
    rows = []
    for line in p.read_text().splitlines():
        line = line.strip()
        if not line:
            continue
        try:
            d = json.loads(line)
        except json.JSONDecodeError:
            continue
        rows.append((int(d["f"]), d.get("w") or {}))
    if not rows:
        return out
    rows.sort(key=lambda r: r[0])
    pre = [r for r in rows if r[0] == ENTRY_FRAME - 1]
    if pre:
        out["mr_at_entry"] = pre[-1][1].get("MainRoutine")
    out["scroll_x_end"] = rows[-1][1].get("ScrollX")
    if scroll_at := [w.get("ScrollX") for f, w in rows
                     if f > ENTRY_FRAME and w.get("ScrollX") is not None]:
        out["scroll_x_post_entry"] = scroll_at[:6]
    out["room_at_end"] = rows[-1][1].get("RoomNumber")
    out["mr_at_end"] = rows[-1][1].get("MainRoutine")
    # 接受判据：注入后必须看到「房号走完 F0→F4 收口链」的**收口标记**，并以 $FF 结束。
    # 只看 F0..F4 标记（而不是「某帧房号 < $F0」）：注入后第一帧的采样相位在房号已被
    # SetupRoom 改成 F0 之后（实测 room 01 的 trace 从 901:F0 起），用采样值判接受会误判。
    # 收口链 `attrib_table_write` 只在 SetupRoom 成功画完房之后才被驱动
    # （asm 07:EAD4 end_of_room ⇒ $5A=$F0；后续 F1/F2/F3/F4 ⇒ $FF）。
    markers = []
    for f, w in rows:
        if f <= ENTRY_FRAME:
            continue
        rn = w.get("RoomNumber")
        if rn is None:
            continue
        out["room_frames"].append(f"{f}:{rn:02X}")
        if 0xF0 <= rn <= 0xF4:
            markers.append((f, rn))
        elif rn == 0xFF and markers:
            out["back_to_ff_frame"] = f
            break
    out["room_frames"] = out["room_frames"][:12]
    out["ff_markers"] = [f"{f}:{rn:02X}" for f, rn in markers]
    out["accepted"] = bool(markers and out["back_to_ff_frame"])
    return out


def ppu_layer_diff(stem: str, frame: int, tag: str) -> dict | None:
    """对比两腿同帧 PPU dump，把差异分解到层（名表=房间数据 / CHR / 调色板 / 滚动）。"""
    emu = TRACES / f"{stem}_f{frame:05d}_{tag}.ppu.hex"
    rust = SHOTS_RUST / f"{stem}_f{frame:05d}_{tag}.ppu.hex"
    if not (emu.exists() and rust.exists()):
        return None
    try:
        a = bytes.fromhex(emu.read_text().strip())
        b = bytes.fromhex(rust.read_text().strip())
    except ValueError:
        return None
    if len(a) != len(b):
        return {"error": f"dump 长度不一致 {len(a)}/{len(b)}"}
    out: dict = {}
    for name, lo, hi in PPU_REGIONS:
        n = sum(1 for i in range(lo, min(hi, len(a))) if a[i] != b[i])
        out[name] = n
    out["rust_scroll"] = list(b[0x3020:0x3023]) if len(b) >= 0x3023 else None
    return out


def _png_pair(stem: str, frame: int, tag: str):
    """取两腿同一采集点的 PNG（emu 在 shots/，rust 在 shots_rust/）。"""
    a = ROOT / "shots" / f"{stem}_f{frame:05d}_{tag}.png"
    b = SHOTS_RUST / f"{stem}_f{frame:05d}_{tag}.png"
    return (a, b) if (a.exists() and b.exists()) else None


def shift_probe(stem: str, frame: int, tag: str, raw_px: int,
                max_shift: int = 4) -> dict | None:
    """PPU 全同但像素有差时，测「整屏平移几像素」能否解释差异。

    门滚是 2 px/帧（`ScrollDoor ; 07:E1F1` 每帧 2 调），两腿若差一个滚动 tick，
    表现就是整屏水平差 2 px —— 而滚动寄存器（dump 尾 3B）在 emu 侧恒为 0 不可比，
    所以必须靠平移自证，否则会被误判成精灵层差异。
    """
    pair = _png_pair(stem, frame, tag)
    if pair is None or raw_px <= 0:
        return None
    try:
        import numpy as np
        from PIL import Image
    except ImportError:
        return None
    try:
        a = np.asarray(Image.open(pair[0]).convert("RGB")).astype(np.int16)
        b = np.asarray(Image.open(pair[1]).convert("RGB")).astype(np.int16)
    except OSError:
        return None
    if a.shape != b.shape:
        return None
    h, w = a.shape[:2]
    best = (0, 0, None)
    for dy in range(-max_shift, max_shift + 1):
        for dx in range(-max_shift, max_shift + 1):
            ax0, ax1 = max(0, dx), w + min(0, dx)
            bx0, bx1 = max(0, -dx), w - max(0, dx)
            ay0, ay1 = max(0, dy), h + min(0, dy)
            by0, by1 = max(0, -dy), h - max(0, dy)
            sub_a = a[ay0:ay1, ax0:ax1]
            sub_b = b[by0:by1, bx0:bx1]
            if sub_a.size == 0:
                continue
            n = int((sub_a != sub_b).any(axis=2).sum())
            if best[2] is None or n < best[2]:
                best = (dx, dy, n)
    dx, dy, residual = best
    return {"dx": dx, "dy": dy, "residual_px": residual, "raw_px": raw_px,
            "explains": bool((dx or dy) and residual is not None
                             and residual <= raw_px * 0.5)}


def classify_layer(px: int, ppu: dict | None, shift: dict | None = None) -> str:
    """像素差归层：`clean` / `nametable` / `chr` / `palette` /
    `scroll_drift`（PPU 全同但整屏平移可解释）/ `sprite`（排除法）。"""
    if ppu is None or ppu.get("error"):
        return "unknown"
    if ppu.get("chr"):
        return "chr"
    if (ppu.get("nt0", 0) + ppu.get("nt1", 0) + ppu.get("nt2", 0)
            + ppu.get("nt3", 0)):
        return "nametable"
    if ppu.get("palette"):
        return "palette"
    if not px:
        return "clean"
    if shift and shift.get("explains"):
        return "scroll_drift"
    # CHR/名表/调色板全同 ⇒ 剩余像素差只能来自精灵面（OAM 未 dump，故为「排除法」结论）
    return "sprite"


def parse_report(stem: str) -> dict | None:
    p = SHOTS_RUST / f"{stem}.l4report.json"
    if not p.exists():
        return None
    try:
        return json.loads(p.read_text())
    except json.JSONDecodeError:
        return None


_SUMMARY_RE = re.compile(r"汇总：一致 (\d+) · 失配 (\d+) · 缺口阻塞 (\d+) · 错误 (\d+)")


def parse_stdout_summary(text: str) -> dict | None:
    m = None
    for m in _SUMMARY_RE.finditer(text or ""):
        pass
    if not m:
        return None
    return {"match": int(m.group(1)), "mismatch": int(m.group(2)),
            "blocked": int(m.group(3)), "error": int(m.group(4))}


def _tail(text: str, n: int = 10) -> list[str]:
    return [ln for ln in (text or "").splitlines() if ln.strip()][-n:]


# ---------------------------------------------------------------- 单房执行

def run_room(area: int, room: int, frames: int, captures: list[int],
             timeout: float) -> dict:
    stem = case_name(area, room)
    case_path = CASES / f"{stem}.txt"
    rec: dict = {
        "area": area, "area_name": AREA_NAMES.get(area, "?"), "room": room,
        "status": "error", "ok": False, "diff_pixels": {}, "captures": [],
        "entry_preset_version": ENTRY_PRESET_VERSION, "entry_frame": ENTRY_FRAME,
        "notes": [],
    }
    t0 = time.time()
    try:
        case_path.write_text(build_case(area, room, frames, captures))
        CURRENT_CASES.append(case_path)
        try:
            p = subprocess.run(
                [sys.executable, str(L4), "run", stem, "--timeout", str(timeout)],
                cwd=ROOT, capture_output=True, text=True, timeout=timeout + 300)
            out = (p.stdout or "") + (p.stderr or "")
            rec["l4_exit"] = p.returncode
        except subprocess.TimeoutExpired as exc:
            rec["status"] = "timeout"
            rec["notes"].append(f"对拍器超时（>{timeout + 300:.0f}s）")
            rec["stdout_tail"] = _tail(getattr(exc, "stdout", "") or "")
            return rec

        ent = analyse_trace(stem)
        rec["room_load"] = ent
        rec["l4_summary"] = parse_stdout_summary(out)
        rec["stdout_tail"] = _tail(out)
        rep = parse_report(stem)
        if rep is None:
            rec["status"] = "error"
            rec["notes"].append("缺 l4report（用例解析错误或腿失败，未产出报告）")
            if re.search(r"ScenarioError", out):
                rec["notes"].append("用例解析错误（ScenarioError）")
            return rec

        rec["anchors"] = {k: rep.get(k) for k in
                          ("scenario_sha256", "rom_sha256", "sched_sha256")}
        arts = rep.get("artifacts") or {}
        results = []
        for r in rep.get("results") or []:
            f = r.get("file")
            cap = {"frame": r.get("frame"), "tag": r.get("tag"),
                   "status": r.get("status"), "diff_pixels": r.get("diff_pixels"),
                   "total_pixels": r.get("total_pixels"),
                   "whitelist_hits": r.get("whitelist_hits"),
                   "file": f,
                   "rust_sha256": arts.get(f"shots_rust/{f}")}
            ppu = (ppu_layer_diff(stem, int(r["frame"]), str(r["tag"]))
                   if r.get("frame") is not None and r.get("tag") else None)
            px_n = cap.get("diff_pixels") or 0
            sh = (shift_probe(stem, int(r["frame"]), str(r["tag"]), px_n)
                  if r.get("frame") is not None and r.get("tag") else None)
            cap["ppu"] = ppu
            cap["shift"] = sh
            cap["layer"] = classify_layer(px_n, ppu, sh)
            results.append(cap)
            if r.get("tag"):
                rec["diff_pixels"][str(r["tag"])] = r.get("diff_pixels")
                rec.setdefault("layers", {})[str(r["tag"])] = cap["layer"]
                if ppu and not ppu.get("error"):
                    rec.setdefault("ppu_diff", {})[str(r["tag"])] = ppu
        rec["captures"] = results
        for key in ("pass", "fail", "gap"):
            if key in rep:
                rec[f"l4_{key}"] = rep[key]
        if "captures" in rep:
            rec["l4_captures"] = rep["captures"]

        # ---- 判定（严格）----
        if not ent.get("accepted"):
            rec["status"] = "skipped"
            rec["skip_reason"] = "room_number_not_ff"
            rec["notes"].append("注入后 $005A 未走完 F0→…→FF（房号未被接受/载入链未跑完）"
                                "——像素结果不计入结论")
            return rec
        if ent.get("mr_at_entry") not in (None, 3):
            rec["notes"].append(
                f"注入帧前 MainRoutine={ent['mr_at_entry']}（期望 3=GameEngine）")
        bad = [c for c in results if c["status"] in ("error", "blocked")]
        if bad:
            rec["status"] = ("blocked" if all(c["status"] == "blocked" for c in bad)
                             else "error")
            rec["notes"].append("采集点 error/blocked："
                                + ", ".join(f"{c['tag']}={c['status']}" for c in bad))
            return rec
        diffs = [(c["diff_pixels"] or 0) for c in results]
        if results and all(d == 0 for d in diffs):
            rec["status"] = "ok"
            rec["ok"] = True
        else:
            rec["status"] = "mismatch"
            rec["mismatch"] = [c for c in results if (c["diff_pixels"] or 0) != 0]
            rec["notes"].append(f"最大失配 {max(diffs or [0])} px")
        return rec
    except Exception as exc:  # 单房失败不得中断整批
        rec["status"] = "error"
        rec["notes"].append(f"{type(exc).__name__}: {exc}")
        return rec
    finally:
        rec["wall_s"] = round(time.time() - t0, 1)
        try:
            case_path.unlink(missing_ok=True)
        except OSError:
            pass
        if case_path in CURRENT_CASES:
            CURRENT_CASES.remove(case_path)


# ---------------------------------------------------------------- 批次

def parse_int_list(text: str) -> list[int]:
    return [int(x) for x in re.split(r"[,\s]+", str(text).strip()) if x]


def expand_rooms(spec: str, area: int) -> tuple[list[int], list[dict]]:
    """`--rooms` 展开 ⇒ (计划房号, 需立即跳过的房)。

    每个被跳过的房**单独占一行**（room 具体到房号），这样 `planned = ran + skipped`
    在 JSON 里是机械可核的，不会出现「跳过 1 条 = 跳过 46 房」的含糊。
    """
    n = ROOM_COUNTS.get(area)
    if n is None:
        return [], [{"area": area, "room": None, "skip_reason": "area_unknown",
                     "detail": f"未知区号 {area}"}]
    if spec.strip().lower() in ("all", "*", ""):
        want = list(range(n))
    else:
        want, skips = [], []
        for r in parse_int_list(spec):
            if 0 <= r < n:
                want.append(r)
            else:
                skips.append({"area": area, "room": r,
                              "skip_reason": "room_index_out_of_range",
                              "detail": f"{AREA_NAMES[area]} 只有 {n} 个房（0..{n - 1}）"})
    if area in PRESET_VERIFIED_AREAS:
        return want, skips if not (spec.strip().lower() in ("all", "*", "")) else []
    detail = ("entry_preset 只在 area 0 用反证实验定稿过。扩到本区须补两条注入："
              "$0074 InArea 与 CopyAreaPointers ; 07:C8B0 的区指针块——只注区号时 "
              "emu 腿仍用旧区的 struct/metatile 指针，rust 腿却按本区资产表绘制，"
              "得到的失配是工具产物（见 docs/findings/room-sweep.md §8.1）")
    return [], [{"area": area, "room": r, "skip_reason": "entry_preset_unverified_area",
                 "detail": detail} for r in want]


def _packtag(area: int) -> str:
    return {0: "br", 1: "no", 2: "kr", 3: "to", 4: "ri"}.get(area, "?")


def _unported_detail(area: int) -> str:
    if area in ROOM_COUNTS:
        return (f"{AREA_NAMES.get(area, area)} 房流已在资产包内"
                f"（assets/metroid.pack `area.{_packtag(area)}.room_streams`），"
                "但 Rust 侧区派发未移植（crates/game-core/src/world/room_draw.rs "
                "区 ∉ {0,1} ⇒ unported!）")
    return "资产包内无该区房流"


def atomic_write_json(path: Path, obj) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    tmp = path.with_suffix(path.suffix + ".tmp")
    tmp.write_text(json.dumps(obj, ensure_ascii=False, indent=1) + "\n")
    os.replace(tmp, path)


def out_path_for(template: str, area: int, n_areas: int) -> Path:
    if "{area}" in template:
        return ROOT / template.replace("{area}", str(area))
    if n_areas > 1:
        raise SystemExit(f"--out 含多个区时必须带 {{area}} 占位：{template}")
    return ROOT / template


def build_doc(area: int, args, plan_rooms: list[int], records: list[dict],
              skip_rows: list[dict]) -> dict:
    records = sorted(records, key=lambda r: (r["room"] is None, r["room"]))
    counts = {k: 0 for k in ("ok", "mismatch", "blocked", "error", "timeout")}
    for r in records:
        counts[r["status"]] = counts.get(r["status"], 0) + 1
    skips = [s for s in skip_rows if s.get("area") == area]
    skips += [{"area": r["area"], "room": r["room"],
               "skip_reason": r.get("skip_reason"),
               "detail": (r.get("notes") or [""])[-1]}
              for r in records if r["status"] == "skipped"]
    hashes = {c.get("rust_sha256") for r in records for c in r.get("captures") or []
              if c.get("rust_sha256")}
    return {
        "tool": "tools/movie_ref/room_sweep.py",
        "entry_preset_version": ENTRY_PRESET_VERSION,
        "entry_frame": ENTRY_FRAME,
        "entry_preset": [{"addr": f"{a:04X}", "value": f"{v:02X}", "persistence": p}
                         for a, v, p in _preset(0)],
        "generated_at": time.strftime("%Y-%m-%dT%H:%M:%S%z"),
        "args": {"areas": args.areas, "rooms": args.rooms, "frames": args.frames,
                 "captures": args.captures, "timeout": args.timeout, "jobs": args.jobs},
        "area": area, "area_name": AREA_NAMES.get(area, "?"),
        "ok_definition": ("两腿都跑完（无 error/blocked/timeout）且每采集点 diff==0 "
                          "且注入房号被接受（$005A 走完 F0→…→FF、MainRoutine 保持 3）"),
        "summary": {
            # 分母 = 该区「计划要扫」的房数（含被跳过的），不是「跑成功的」
            "planned": len([s for s in skip_rows if s.get("area") == area
                            and s.get("room") is not None]) + len(plan_rooms),
            "ran": len(records),
            "ok": counts["ok"], "mismatch": counts["mismatch"],
            "blocked": counts["blocked"], "error": counts["error"],
            "timeout": counts["timeout"], "skipped": len(skips),
            "denominator_note": "denominator = planned（计划扫的房数），不是 ran",
            "distinct_rust_capture_sha256": len(hashes),
        },
        "skips": skips,
        "rooms": records,
    }


def _flush(path: Path, area: int, args, plan_rooms: list[int],
           records: list[dict], skip_rows: list[dict], partial: bool) -> None:
    doc = build_doc(area, args, plan_rooms, records, skip_rows)
    if partial:
        doc["partial"] = True
    doc["elapsed_s"] = round(time.time() - START, 1)
    atomic_write_json(path, doc)


# 临时用例卫生：进程被 SIGINT/SIGTERM 杀掉时 finally 不执行 ⇒ 显式清理，
# 并在启动时清掉上一轮被硬杀留下的残留（仅本工具前缀，绝不碰别的用例）。
CURRENT_CASES: list[Path] = []


def _purge_cases(*_args) -> None:
    for p in list(CURRENT_CASES):
        try:
            p.unlink(missing_ok=True)
        except OSError:
            pass
    try:
        CURRENT_CASES.clear()
    except AttributeError:
        pass


def _on_signal(signum, _frame):
    _purge_cases()
    print(f"\n收到信号 {signum}：已清理临时用例，退出（已完成的房已落盘）",
          file=sys.stderr, flush=True)
    raise SystemExit(130)


START = time.time()


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description="房间批量扫描（L4 对拍分母工具）")
    ap.add_argument("--areas", default="0,1", help="区号列表（0=Br 1=No 2=Kr 3=To 4=Ri）")
    ap.add_argument("--rooms", default="all", help="all 或房号列表（如 12,13,14）")
    ap.add_argument("--frames", type=int, default=120, help="注入后扫描窗口帧数")
    ap.add_argument("--captures", default="60,120", help="注入后的相对采集帧，逗号分隔")
    ap.add_argument("--out", default="data/status/room_sweep_{area}.json",
                    help="输出 JSON（多区必须含 {area} 占位）")
    ap.add_argument("--jobs", type=int, default=1,
                    help="并发房数（emu 腿被 scenario 串行锁串行化，>1 基本不提速）")
    ap.add_argument("--timeout", type=float, default=900.0, help="单腿超时秒数（传对拍器）")
    ap.add_argument("--resume", action="store_true", help="跳过 --out 里已 ok 的房")
    ap.add_argument("--dry-run", action="store_true", help="只打印计划，不跑")
    args = ap.parse_args(argv)

    for sig in (signal.SIGINT, signal.SIGTERM):
        signal.signal(sig, _on_signal)
    stale = sorted(CASES.glob("_tmp_sweep_*.txt"))
    for p in stale:
        try:
            p.unlink()
        except OSError:
            pass
    if stale:
        print(f"清理上一轮残留临时用例 {len(stale)} 个", flush=True)

    args.areas = parse_int_list(args.areas)
    args.captures = parse_int_list(args.captures)
    if not args.areas:
        print("--areas 为空", file=sys.stderr)
        return 2
    if not args.captures:
        print("--captures 为空", file=sys.stderr)
        return 2
    if min(args.captures) < 1 or max(args.captures) > args.frames:
        print("--captures 必须落在 1..--frames", file=sys.stderr)
        return 2
    if not L4.exists():
        print(f"缺对拍器 {L4}", file=sys.stderr)
        return 2

    print(f"room sweep：areas={args.areas} rooms={args.rooms} entry=f{ENTRY_FRAME} "
          f"frames={args.frames} captures={args.captures} "
          f"preset={ENTRY_PRESET_VERSION} jobs={args.jobs}", flush=True)
    all_skips: list[dict] = []
    for area in args.areas:
        plan_rooms, bad = expand_rooms(args.rooms, area)
        all_skips.extend(bad)
        if plan_rooms and area not in PORTED_AREAS:
            why = ("area_dispatch_unported" if area in ROOM_COUNTS
                   else "room_streams_not_in_pack")
            detail = _unported_detail(area)
            for r in plan_rooms:
                all_skips.append({"area": area, "room": r,
                                  "skip_reason": why, "detail": detail})
            print(f"[area {area}] 跳过 {len(plan_rooms)} 房：{why}", flush=True)
            plan_rooms = []

        out_path = out_path_for(args.out, area, len(args.areas))
        records: list[dict] = []
        if args.resume and out_path.exists():
            try:
                prev = json.loads(out_path.read_text())
                records = [r for r in (prev.get("rooms") or []) if r.get("ok")]
            except (json.JSONDecodeError, OSError):
                records = []
            if records:
                print(f"[area {area}] --resume：沿用已完成 {len(records)} 房", flush=True)
        done = {(r["area"], r["room"]) for r in records}
        todo = [r for r in plan_rooms if (area, r) not in done]

        def _one(room: int) -> dict:
            rec = run_room(area, room, args.frames, args.captures, args.timeout)
            tag = {"ok": "✅", "mismatch": "❌", "skipped": "⏭", "blocked": "🚧",
                   "error": "⚠", "timeout": "⏰"}.get(rec["status"], "?")
            d = ",".join(f"{k}={v}" for k, v in rec.get("diff_pixels", {}).items()) or "-"
            print(f"[area {area}] {tag} room {room:02d} {rec['status']:8s} "
                  f"diff[{d}] {rec.get('wall_s', 0)}s", flush=True)
            return rec

        if args.dry_run:
            print(f"[area {area}] dry-run：计划 {len(todo)} 房"
                  f"（{todo[:8]}{'…' if len(todo) > 8 else ''}）")
            for sk in [x for x in all_skips if x.get("area") == area]:
                print(f"[area {area}] dry-run 跳过 room={sk.get('room')} "
                      f"理由={sk.get('skip_reason')}：{sk.get('detail')}")
        elif todo:
            if args.jobs <= 1:
                for room in todo:
                    records.append(_one(room))
                    _flush(out_path, area, args, plan_rooms, records, all_skips, True)
            else:
                with concurrent.futures.ThreadPoolExecutor(
                        max_workers=args.jobs) as ex:
                    for fut in concurrent.futures.as_completed(
                            [ex.submit(_one, r) for r in todo]):
                        records.append(fut.result())
                        _flush(out_path, area, args, plan_rooms, records,
                               all_skips, True)
        if not args.dry_run:
            _flush(out_path, area, args, plan_rooms, records, all_skips, False)
            s = build_doc(area, args, plan_rooms, records, all_skips)["summary"]
            try:
                shown = out_path.relative_to(ROOT)
            except ValueError:
                shown = out_path
            print(f"[area {area}] → {shown}")
            print(f"[area {area}] 分母 planned={s['planned']} ran={s['ran']} "
                  f"ok={s['ok']} mismatch={s['mismatch']} blocked={s['blocked']} "
                  f"error={s['error']} timeout={s['timeout']} skipped={s['skipped']} "
                  f"distinct_shots={s['distinct_rust_capture_sha256']}", flush=True)
    print(f"总耗时 {time.time() - START:.1f}s")
    return 0


if __name__ == "__main__":
    sys.exit(main())
