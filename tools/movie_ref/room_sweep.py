#!/usr/bin/env python3
"""房间批量扫描（room sweep）——对每个 (area, room) 生成临时用例、跑 L4 对拍、记账。

用法：
    python3 tools/movie_ref/room_sweep.py --areas 0 --rooms all \
        --frames 120 --captures 60,120 --out data/status/room_sweep_0.json

判据完全复用 `tools/l4_diff_run.py`（像素零容差＋既有视觉白名单），本工具**不**改判据、
不加白名单、不放宽阈值。它只负责：造用例 → 调对拍器 → 解析产物 → 记账 → 清理临时用例。

`ok` 的定义（严格）：两腿都跑完（无 error/blocked/timeout）**且**每个采集点 diff==0
**且**注入的房号被引擎接受——接受 = ①`$005A` 在窗口内走完 F0→…→FF（载入链跑完）
**并且** ②有目标房身份证据（`$33/$34 RoomPtr` 落在资产包里该房房流的
`[起始, 起始+长度)` 区间内，或 trace 里采到 `$005A` == 目标房号；两者都拿不到
⇒ `unverified-identity`，不判 ok）
**并且** ③注入帧起 7 帧 `$001E MainRoutine == 3`（GameEngine）。
任一不满足 ⇒ `ok=false`，明细落 `status`/`notes`/`mismatch`。

分母显式化：`summary` 给出 planned / ran / ok / mismatch / blocked / error / timeout /
skipped / unverified_identity / left_gameengine / stale_report，并机械断言
`planned == ran + skipped` 与 `ran == ok+mismatch+blocked+error+timeout+
unverified_identity+left_gameengine`（违反 ⇒ 退出码非 0）。分母永远是「计划扫的房数」，
不是「跑成功的房数」。

幂等：同一 `--out` 重跑覆盖（原子写）。中断续跑：每房跑完即落盘，`--resume` 跳过已 ok 的房。
"""

from __future__ import annotations

import argparse
import concurrent.futures
import hashlib
import json
import os
import re
import signal
import struct
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
L4 = ROOT / "tools" / "l4_diff_run.py"
CASES = ROOT / "cases"
SHOTS_RUST = ROOT / "shots_rust"
TRACES = ROOT / "analysis" / "traces"
PACK = ROOT / "assets" / "metroid.pack"


def _run_token() -> str:
    """本次进程的临时用例命名 token（pid + 启动秒）。

    用例名带 token 有两个目的：①两个实例并发扫描时各自的临时用例互不覆盖；
    ②启动清理只按本 token 清残留，绝不碰另一个实例在飞的用例（P1 教训：
    旧版无差别 `_tmp_sweep_*.txt` 全删，把另一实例正在跑的 room 29 用例删了）。
    """
    return f"{os.getpid()}-{int(time.time())}"


TOKEN = _run_token()

# ---------------------------------------------------------------- 常量（全部带证据锚）

AREA_NAMES = {0: "Brinstar", 1: "Norfair", 2: "Kraid", 3: "Tourian", 4: "Ridley"}

# 房流房数——`crates/game-data/src/assets.rs:171-199`（BR/NO/KR/TO/RI_ROOMS 的
# `slices(..., N)` 参数 N），数据源 `assets/metroid.pack`
# （`cargo run -q -p pack-tool -- list --pack assets/metroid.pack` 里的
# `area.{br,no,kr,to,ri}.room_streams`）。
ROOM_COUNTS = {0: 47, 1: 46, 2: 37, 3: 21, 4: 42}

# 工具自身的白名单：只扫 Brinstar/Norfair。**这不是引擎的能力边界**——
# `crates/game-data/src/assets.rs:238-247` 的 `room_streams` 支持 0..=4 区，包里
# `area.{br,no,kr,to,ri}.room_streams` 五张表都在；`crates/game-core/src/world/room_draw.rs:67-77`
# 的 `unported!` 只在 assets 返回 `None` 时触发，对 0–4 区永不触发。
# 2/3/4 区未实测的原因是 entry_preset 只在 area 0 定稿（见 PRESET_VERIFIED_AREAS），
# 不是区派发没移植。room 43 blocked 的真实原因是 `bank07:EAD4`（敌兵路径 unported）。
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
    return f"_tmp_sweep_{TOKEN}_a{area}_r{room:02d}"


def build_case(area: int, room: int, frames: int, captures: list[int]) -> str:
    """生成临时用例文本。captures = 相对注入帧的偏移（落 DSL 绝对帧 = ENTRY_FRAME + c）。"""
    stem = case_name(area, room)
    lines = [
        f"name {stem}",
        "# room sweep 临时用例（tools/movie_ref/room_sweep.py 生成，跑完即删）",
        f"# area={area}({AREA_NAMES.get(area, '?')}) room={room} "
        f"entry=f{ENTRY_FRAME} preset={ENTRY_PRESET_VERSION}",
        f"frames {ENTRY_FRAME + frames}",
        # 载入链观测量：$005A 是否走完 F0→FF（房号被接受）、$001E 是否留在 3（GameEngine）、
        # $33/$34 RoomPtr 是否落在资产包里该房房流区间内（目标房身份证据）。
        "watch 001E MainRoutine",
        "watch 005A RoomNumber",
        "watch 0033 RoomPtrLo",
        "watch 0034 RoomPtrHi",
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

# ---------------------------------------------------------------- 目标房身份证据

_PACK_SECTIONS: dict[str, bytes] | None = None


def _pack_sections() -> dict[str, bytes]:
    """解析 `assets/metroid.pack` 的具名段（与 `crates/game-data/src/assets.rs` 同格式）。

    只读；解析失败直接抛——身份判据依赖这份资产表，拿不到应当 fail-loud，
    而不是静默退化成「没有身份证据」。
    """
    global _PACK_SECTIONS
    if _PACK_SECTIONS is not None:
        return _PACK_SECTIONS
    raw = PACK.read_bytes()
    if raw[:4] != b"MTPK":
        raise ValueError(f"{PACK} 不是 MTPK 资产包")
    _ver, _flags, _crc, _size, count = struct.unpack_from("<HHIIH", raw, 4)
    payload = struct.unpack_from("<I", raw, 28)[0]
    blobs: dict[int, bytes] = {}
    for i in range(count):
        key, _fl, off, ln, _crc32, _res = struct.unpack_from("<HHIIII", raw, 32 + i * 20)
        blobs[key] = raw[payload + off : payload + off + ln]
    meta = json.loads(blobs[0])
    _PACK_SECTIONS = {s["name"]: blobs[s["key"]] for s in meta["sections"]}
    return _PACK_SECTIONS


def expected_room_ptr(area: int, room: int) -> int | None:
    """房号 → `$33/$34 RoomPtr` 的**起始**值（LE u16）；表不在包里则返回 None。

    地址锚：`crates/game-core/src/ram_map.rs` 的 `ROOM_PTR = 0x0033`；
    写入点 `crates/game-core/src/world/room_load.rs`（SetupRoom 查 `room_ptrs`
    表后 `$33=lo, $34=hi`）；表来源 `crates/game-data/src/assets.rs` 的
    `room_ptr(area, idx)`，即包内 `area.<tag>.room_ptrs` 段（每项 2B LE）。
    """
    blob = _pack_sections().get(f"area.{_packtag(area)}.room_ptrs")
    if not blob or room < 0 or room * 2 + 2 > len(blob):
        return None
    return struct.unpack_from("<H", blob, room * 2)[0]


def room_stream_region(area: int, room: int) -> tuple[int, int] | None:
    """房号 → `[RoomPtr 起始, 起始+房流长度)`（`$33/$34` 的有效取值区间）。

    为什么不是「等于固定值」：`$33/$34` 是房间数据的**读游标**，SetupRoom 起始写
    起始地址，随后 `AddToRoomPtr` 随读随加（实测 room 0 在 f901..f904 采到
    0xA448/0xA44E，起始是 0xA441）。所以身份判据是「游标落在本房房流区间内」。
    区间机械来自包内 `area.<tag>.room_offs`（每房 8B：`off u32, len u32`，
    与 `crates/game-data/src/assets.rs` 的 `slices(body, offs, n)` 同构）。
    """
    base = expected_room_ptr(area, room)
    offs = _pack_sections().get(f"area.{_packtag(area)}.room_offs")
    if base is None or not offs or room < 0 or room * 8 + 8 > len(offs):
        return None
    _off, ln = struct.unpack_from("<II", offs, room * 8)
    return base, ln


def analyse_trace(stem: str, area: int, room: int) -> dict:
    """从 emu 帧级 trace 取载入链 / 目标房身份 / GameEngine 窗口三项证据。

    1. **载入链** `load_chain`：注入后出现 F0..F4 收口标记并以 `$FF` 结束
       （`attrib_table_write` 只在 SetupRoom 画完房后驱动）。
    2. **目标房身份** `identity_basis`：优先 `$33/$34` 落在资产包里该房房流的
       `[起始, 起始+长度)` 区间内（`room_ptr`——游标随读随加，不能要求等于起始值）；
       否则退到任一帧 `$005A` == 目标房号（`room_number_frame`，
       只认 `f >= ENTRY_FRAME`，避免起点房 0 在注入前就「天然相等」的假证据）；
       都没有 ⇒ `none`。
    3. **GameEngine 窗口** `mr_window_ok`：`ENTRY_FRAME..ENTRY_FRAME+6` 全 7 帧
       `$001E MainRoutine == 3` 且帧不缺。

    `accepted = load_chain and identity_ok`（GameEngine 窗口在 run_room 里单独判，
    以便给出 `left-gameengine` 这个更具体的状态）。
    """
    p = TRACES / f"{stem}.frames.jsonl"
    out: dict = {"trace": p.relative_to(ROOT).as_posix() if p.exists() else None,
                 "accepted": None, "load_chain": None, "identity_basis": "none",
                 "identity_ok": False, "expected_room_ptr": None,
                 "mr_at_entry": None, "mr_at_end": None, "mr_window_ok": None,
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
    by_frame = {f: w for f, w in rows}
    pre = by_frame.get(ENTRY_FRAME - 1)
    if pre is not None:
        out["mr_at_entry"] = pre.get("MainRoutine")
    out["scroll_x_end"] = rows[-1][1].get("ScrollX")
    if scroll_at := [w.get("ScrollX") for f, w in rows
                     if f > ENTRY_FRAME and w.get("ScrollX") is not None]:
        out["scroll_x_post_entry"] = scroll_at[:6]
    out["room_at_end"] = rows[-1][1].get("RoomNumber")
    out["mr_at_end"] = rows[-1][1].get("MainRoutine")

    # ---- 3. GameEngine 窗口：逐帧断言，任一缺失/不等即失败 ----
    window = {f: by_frame.get(f) for f in range(ENTRY_FRAME, ENTRY_FRAME + 7)}
    out["mr_window"] = {str(f): (w.get("MainRoutine") if w is not None else None)
                        for f, w in window.items()}
    out["mr_window_missing"] = [f for f, w in window.items() if w is None]
    out["left_gameengine_frames"] = [f for f, w in window.items()
                                     if w is not None and w.get("MainRoutine") != 3]
    out["mr_window_ok"] = (not out["mr_window_missing"]
                           and not out["left_gameengine_frames"])

    # ---- 1. 载入链：F0..F4 收口标记 + 回到 FF ----
    # 只看 F0..F4 标记（而不是「某帧房号 < $F0」）：注入后第一帧的采样相位在房号已被
    # SetupRoom 改成 F0 之后（实测 room 01 的 trace 从 901:F0 起），用采样值判接受会误判。
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
    out["load_chain"] = bool(markers and out["back_to_ff_frame"])

    # ---- 2. 目标房身份证据 ----
    # `$33/$34` 是房流读游标：判据 = 游标落在本房 `[起始, 起始+长度)` 内。
    region = room_stream_region(area, room)
    out["expected_room_ptr"] = f"{region[0]:04X}" if region else None
    out["room_ptr_region"] = (f"{region[0]:04X}..{region[0] + region[1]:04X}"
                              if region else None)
    ptr_hits: list[int] = []
    rn_hits: list[int] = []
    for f, w in rows:
        if f < ENTRY_FRAME:
            continue
        if w.get("RoomNumber") == room:
            rn_hits.append(f)
        lo, hi = w.get("RoomPtrLo"), w.get("RoomPtrHi")
        if region is not None and lo is not None and hi is not None:
            base, ln = region
            if base <= (lo | (hi << 8)) < base + ln:
                ptr_hits.append(f)
    out["room_ptr_hits"] = ptr_hits[:6]
    out["room_number_hits"] = rn_hits[:6]
    if ptr_hits:
        out["identity_basis"] = "room_ptr"
    elif rn_hits:
        out["identity_basis"] = "room_number_frame"
    else:
        out["identity_basis"] = "none"
    out["identity_ok"] = out["identity_basis"] != "none"
    out["accepted"] = bool(out["load_chain"] and out["identity_ok"])
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


def parse_report(stem: str, started_at: float, case_sha: str) -> tuple[dict | None, str]:
    """读本次跑的 l4report，并校验它确实属于本次运行（防陈旧报告假绿）。

    校验两个机械锚（**不**校验 `sched_sha256`：sched 文本由 `tools/l4_diff_run.py`
    的 `expand_schedule` 现场生成，工具没有对外暴露「只算 sched 文本」的入口，
    照抄一份等于把判据逻辑复制成两处——所以取能机械拿到的锚）：
      1. 报告 mtime ≥ 本次 subprocess 启动时刻（留 1s 文件系统容差）；
      2. 报告 `scenario_sha256` == 本次写出的 `cases/<stem>.txt` 的 sha256。
    调用方还会在跑前删同 stem 的旧报告，作为第三重保险。
    返回 `(报告, 失败原因)`；失败原因以 `stale:` 前缀表示陈旧/错配（计入 `stale_report`）。
    """
    p = SHOTS_RUST / f"{stem}.l4report.json"
    if not p.exists():
        return None, "缺 l4report（用例解析错误或腿失败，未产出报告）"
    try:
        rep = json.loads(p.read_text())
    except json.JSONDecodeError:
        return None, "stale: l4report 不是合法 JSON"
    try:
        mtime = p.stat().st_mtime
    except OSError:
        return None, "stale: l4report 无法 stat"
    if mtime + 1.0 < started_at:
        return None, f"stale: l4report mtime={mtime:.0f} 早于本次启动 {started_at:.0f}"
    got = rep.get("scenario_sha256")
    if got != case_sha:
        return None, f"stale: l4report scenario_sha256={got} 与本次用例 {case_sha} 不符"
    return rep, ""


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
    report_path = SHOTS_RUST / f"{stem}.l4report.json"
    rec: dict = {
        "area": area, "area_name": AREA_NAMES.get(area, "?"), "room": room,
        "status": "error", "ok": False, "diff_pixels": {}, "captures": [],
        "entry_preset_version": ENTRY_PRESET_VERSION, "entry_frame": ENTRY_FRAME,
        "notes": [],
    }
    t0 = time.time()
    try:
        case_text = build_case(area, room, frames, captures)
        case_sha = hashlib.sha256(case_text.encode("utf-8")).hexdigest()
        # 跑前删本 stem 的旧报告：l4_diff_run 在腿失败时不会覆盖旧报告，不删就会把
        # 上一轮的绿报告当成本次结果（P1 假绿）。
        try:
            report_path.unlink(missing_ok=True)
        except OSError:
            pass
        case_path.write_text(case_text)
        CURRENT_CASES.append(case_path)
        started_at = time.time()
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

        ent = analyse_trace(stem, area, room)
        rec["room_load"] = ent
        rec["identity_basis"] = ent.get("identity_basis")
        rec["l4_summary"] = parse_stdout_summary(out)
        rec["stdout_tail"] = _tail(out)

        # ---- 判定（严格，顺序即优先级）----
        # a) 载入链没跑完：这不是「跑失败」，而是注入没被引擎接受 ⇒ 不计入对拍结论。
        if not ent.get("load_chain"):
            rec["status"] = "skipped"
            rec["skip_reason"] = "room_number_not_ff"
            rec["notes"].append("注入后 $005A 未走完 F0→…→FF（房号未被接受/载入链未跑完）"
                                "——像素结果不计入结论")
            return rec
        # b) 对拍器退出码：exit 1 = 报告正常生成且 fail>0（有失配格，l4_diff_run.py:1336
        #    `sys.exit(0 if rep["fail"] == 0 else 1)`）⇒ 是正常判定输入，交给下面的
        #    报告解析与采集点判定；报告新鲜度由 parse_report 把关。非 0/1 才是腿失败
        #    或参数错（exit 2 族），报告不可信 ⇒ error。
        if rec.get("l4_exit") not in (0, 1, None):
            rec["status"] = "error"
            rec["notes"].append(f"l4_diff_run 退出码 {rec['l4_exit']}（腿失败/参数错）")
            return rec
        # c) 没有任何目标房身份证据：diff 再 0 也不能证明「扫的是目标房」。
        if not ent.get("identity_ok"):
            rec["status"] = "unverified-identity"
            rec["notes"].append(
                "拿不到目标房身份证据（$33/$34 RoomPtr 未采到该房房流区间内的值，且 $005A 未采到目标"
                f"房号 {room}）——diff 结果不可归因到目标房，不判 ok")
            return rec
        # d) GameEngine 窗口逐帧断言。
        if not ent.get("mr_window_ok"):
            rec["status"] = "left-gameengine"
            rec["notes"].append(
                f"$001E MainRoutine 在 f{ENTRY_FRAME}..f{ENTRY_FRAME + 6} 未全为 3："
                f"missing={ent.get('mr_window_missing')}, "
                f"非 3 帧={ent.get('left_gameengine_frames')}")
            return rec

        rep, why = parse_report(stem, started_at, case_sha)
        if rep is None:
            rec["status"] = "error"
            rec["notes"].append(why)
            if why.startswith("stale:"):
                rec["stale_report"] = True
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

        # ---- 采集点层判定 ----
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
    skips: list[dict] = []
    if spec.strip().lower() in ("all", "*", ""):
        want = list(range(n))
    else:
        want = []
        for r in parse_int_list(spec):
            if 0 <= r < n:
                want.append(r)
            else:
                skips.append({"area": area, "room": r,
                              "skip_reason": "room_index_out_of_range",
                              "detail": f"{AREA_NAMES[area]} 只有 {n} 个房（0..{n - 1}）"})
    if area not in PORTED_AREAS:
        # 2/3/4 区的门是**工具白名单**，不是区派发未移植（见 §0 更正）；先报这一层，
        # 免得把「工具没开」说成「引擎没接」。
        detail = _unported_detail(area)
        return [], skips + [{"area": area, "room": r,
                             "skip_reason": "area_not_in_tool_allowlist",
                             "detail": detail} for r in want]
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
                "资产/渲染侧对 0..=4 区都有表；跳过是本工具的白名单自限"
                "（PORTED_AREAS=(0,1)，entry_preset 只在 area 0 定稿），"
                "不是区派发未移植")
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
    judged_statuses = ("ok", "mismatch", "blocked", "error", "timeout",
                       "unverified-identity", "left-gameengine")
    counts = {k: 0 for k in judged_statuses}
    for r in records:
        counts[r["status"]] = counts.get(r["status"], 0) + 1
    skips = [s for s in skip_rows if s.get("area") == area]
    skips += [{"area": r["area"], "room": r["room"],
               "skip_reason": r.get("skip_reason"),
               "detail": (r.get("notes") or [""])[-1]}
              for r in records if r["status"] == "skipped"]
    hashes = {c.get("rust_sha256") for r in records for c in r.get("captures") or []
              if c.get("rust_sha256")}
    # 分母与台账必须机械自洽：planned 只数「具体房号」的行（area_unknown 这种
    # room=None 的区级跳过单独记 area_skips，不混进 planned）。
    planned = len([s for s in skip_rows if s.get("area") == area
                   and s.get("room") is not None]) + len(plan_rooms)
    ran = len([r for r in records if r["status"] != "skipped"])
    skipped = len([s for s in skips if s.get("room") is not None])
    area_skips = len([s for s in skips if s.get("room") is None])
    judged = sum(counts[k] for k in judged_statuses)
    violations = []
    if planned != ran + skipped:
        violations.append(f"planned({planned}) != ran({ran}) + skipped({skipped})")
    if ran != judged:
        violations.append(f"ran({ran}) != Σ判定状态({judged})")
    stale_report = len([r for r in records if r.get("stale_report")])
    note = getattr(args, "note_effective", None)
    doc = {
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
                          "且注入房号被接受（$005A 走完 F0→…→FF 且有目标房身份证据："
                          "$33/$34 RoomPtr 落在该房房流区间内，或采到 $005A==目标房号）"
                          "且 f900..f906 $001E MainRoutine 全为 3（GameEngine）"),
        "summary": {
            # 分母 = 该区「计划要扫」的房数（含被跳过的），不是「跑成功的」
            "planned": planned,
            "ran": ran,
            "ok": counts["ok"], "mismatch": counts["mismatch"],
            "blocked": counts["blocked"], "error": counts["error"],
            "timeout": counts["timeout"], "skipped": skipped,
            "unverified_identity": counts["unverified-identity"],
            "left_gameengine": counts["left-gameengine"],
            "stale_report": stale_report,
            "area_skips": area_skips,
            "denominator_note": "denominator = planned（计划扫的房数），不是 ran",
            "distinct_rust_capture_sha256": len(hashes),
            "arithmetic_ok": not violations,
            "arithmetic_violations": violations,
        },
        "skips": skips,
        "rooms": records,
    }
    if note:
        doc["note"] = note
        src = getattr(args, "note_source", None)
        if src:
            doc["note_source"] = src
        # 旧 JSON 里手工合并的 note_rerun 被沿用时要显式标出来源，否则重跑一次
        # 就会把「这条备注不是工具产出」这件事丢掉（复核 finding）。
        if src == "prev_note_rerun" or getattr(args, "manual_note", False):
            doc["manual_note"] = True
    return doc


def _flush(path: Path, area: int, args, plan_rooms: list[int],
           records: list[dict], skip_rows: list[dict], partial: bool) -> None:
    doc = build_doc(area, args, plan_rooms, records, skip_rows)
    if partial:
        doc["partial"] = True
    doc["elapsed_s"] = round(time.time() - START, 1)
    atomic_write_json(path, doc)


# 临时用例卫生：进程被 SIGINT/SIGTERM 杀掉时 finally 不执行 ⇒ 显式清理。
# 启动时的残留清理**只按本进程 token**（`_tmp_sweep_{TOKEN}_*.txt`），这样两个实例
# 并发扫描时不会互删在飞用例；本进程的用例在每房 finally 里删，进程级清理兜底。
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
    ap.add_argument("--note", default=None,
                    help="写入 JSON 顶层 note 的备注（不传时沿用 --out 里已有的 note/note_rerun）")
    args = ap.parse_args(argv)

    for sig in (signal.SIGINT, signal.SIGTERM):
        signal.signal(sig, _on_signal)
    # 只清**本进程 token** 的残留：两个实例并发时 token 不同，绝不误删对方在飞的用例
    # （P1 教训：旧版 `_tmp_sweep_*.txt` 无差别全删，删掉了另一实例的 room 29 用例）。
    stale = sorted(CASES.glob(f"_tmp_sweep_{TOKEN}_*.txt"))
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
          f"preset={ENTRY_PRESET_VERSION} jobs={args.jobs} token={TOKEN}", flush=True)
    all_skips: list[dict] = []
    all_violations: list[str] = []
    for area in args.areas:
        plan_rooms, bad = expand_rooms(args.rooms, area)
        all_skips.extend(bad)
        if not plan_rooms and bad:
            print(f"[area {area}] 跳过 {len(bad)} 房：{bad[0]['skip_reason']}", flush=True)

        out_path = out_path_for(args.out, area, len(args.areas))
        prev: dict = {}
        if out_path.exists():
            try:
                prev = json.loads(out_path.read_text())
            except (json.JSONDecodeError, OSError):
                prev = {}
        # 备注跨重跑保留：--note 优先；否则沿用旧 JSON 的 note / 手工合并的 note_rerun。
        if args.note:
            args.note_effective, args.note_source = args.note, "cli"
        elif prev.get("note"):
            args.note_effective, args.note_source = prev["note"], "prev_note"
        elif prev.get("note_rerun"):
            args.note_effective, args.note_source = prev["note_rerun"], "prev_note_rerun"
        else:
            args.note_effective, args.note_source = None, None
        args.manual_note = bool(prev.get("manual_note"))
        records: list[dict] = []
        if args.resume and prev:
            records = [r for r in (prev.get("rooms") or []) if r.get("ok")]
            if records:
                print(f"[area {area}] --resume：沿用已完成 {len(records)} 房", flush=True)
        done = {(r["area"], r["room"]) for r in records}
        todo = [r for r in plan_rooms if (area, r) not in done]

        def _one(room: int) -> dict:
            rec = run_room(area, room, args.frames, args.captures, args.timeout)
            tag = {"ok": "✅", "mismatch": "❌", "skipped": "⏭", "blocked": "🚧",
                   "error": "⚠", "timeout": "⏰", "unverified-identity": "🕵",
                   "left-gameengine": "💤"}.get(rec["status"], "?")
            d = ",".join(f"{k}={v}" for k, v in rec.get("diff_pixels", {}).items()) or "-"
            print(f"[area {area}] {tag} room {room:02d} {rec['status']:18s} "
                  f"diff[{d}] id={rec.get('identity_basis', '?')} "
                  f"{rec.get('wall_s', 0)}s", flush=True)
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
            all_violations.extend(f"area {area}: {v}" for v in s["arithmetic_violations"])
            try:
                shown = out_path.relative_to(ROOT)
            except ValueError:
                shown = out_path
            print(f"[area {area}] → {shown}")
            print(f"[area {area}] 分母 planned={s['planned']} ran={s['ran']} "
                  f"ok={s['ok']} mismatch={s['mismatch']} blocked={s['blocked']} "
                  f"error={s['error']} timeout={s['timeout']} skipped={s['skipped']} "
                  f"unverified_identity={s['unverified_identity']} "
                  f"left_gameengine={s['left_gameengine']} "
                  f"stale_report={s['stale_report']} "
                  f"distinct_shots={s['distinct_rust_capture_sha256']}", flush=True)
    print(f"总耗时 {time.time() - START:.1f}s")
    if all_violations:
        print("SUMMARY 自洽断言失败（退出码 3）：", file=sys.stderr)
        for v in all_violations:
            print("  - " + v, file=sys.stderr)
        return 3
    return 0


if __name__ == "__main__":
    sys.exit(main())
