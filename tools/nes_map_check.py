#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""tools/nes_map_check.py —— `nes-map` 静态腿（P2）。

三态判据（Task 4 brief 追加要求 = 控制器裁定）：

* **Owner / View / Unmodeled**：`NesMapped::SPANS` 里有声明（Owner 是 typed 真源，
  View 是同一批字节的第二解释，Unmodeled 是“已声明但尚未 typed”的面）。
* **Undeclared**：比对面内**没有任何声明**的字节。登记进 `data/status/nes_map.json`，
  每段必须带指向真实代码位置的 `reason`，且总长**只减不增**。
* **重叠 / 跨域冲突 / 账本外新空洞**：FAIL。

空洞的真相**不来自账本**：本工具每次跑 `cargo run -q -p game-core --bin nesmap-dump`，
用 Rust 侧 `NesMapped::SPANS` 与 `memmap::observe::dump::EXTRA_RANGE`（= oracle ram.hex
同一比对面，2272 B）机械重算；账本只是“已批准的空洞清单”，被逐段与重算结果**精确比对**。

用法：
    python3 tools/nes_map_check.py            # 校验（门禁用；0=通过，1=FAIL）
    python3 tools/nes_map_check.py --list     # 列出实际空洞与账本 reason
    python3 tools/nes_map_check.py --write    # 把实际空洞写入账本（只许下调）
"""
from __future__ import annotations

import argparse
import hashlib
import json
import re
import subprocess
import sys
from datetime import datetime, timedelta, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
LEDGER = ROOT / "data" / "status" / "nes_map.json"
SCHEMA = "nes-map-v1"
DUMP_SCHEMA = "nesmap-dump-v1"
DUMP_CMD = ["cargo", "run", "-q", "-p", "game-core", "--bin", "nesmap-dump"]
DUMP_TIMEOUT_S = 180
MAX_ERRORS = 24

# 创世锚：Task 4 建账时的 Undeclared 总长与几何（P2 比对面 2272 B）。
# 只对真实比对面（face_total==2272）生效；用于检出「手工改 history[0] 洗白新洞」。
GENESIS_TOTAL = 1553
GENESIS_GEOMETRY_SHA256 = "4419dc40f5014cd559499bc56eba56399864e0d9c20eb40e0aea7b6ffecb3304"

KEYS = {"owner", "view", "unmodeled", "case", "mode", "ephemeral"}
PLACEHOLDER_REASONS = {
    "", "todo", "tbd", "fixme", "unknown", "n/a", "na", "?", "??",
    "待定", "未知", "不详", "待补", "无", "none", "null",
}
# reason 必须指向真实代码位置（可 grep 的 file.rs / 符号）；单说“没人管”不算。
CODE_POINTER_RE = re.compile(r"\.(rs|py)\b")

# 建账时每段 Undeclared 的初始 reason（真实代码指针，可 grep 验证）。
# 段边界 = 实际空洞；后续 --write 只会在洞缩小/消失时继承/丢弃这些 reason。
DEFAULT_REASONS = [
    (0x0000, 0x0033,
     "零页系统面（Joy1Status=$14/Joy2Status=$15/GameMode=$1D/FrameCount=$2D 等），"
     "常量与证据见 crates/game-core/src/ram_map.rs，写者分散在 world/update.rs、frame/nmi.rs"),
    (0x0034, 0x0003,
     "RoomPtr hi=$34 由 world/room_load.rs:236 写；$35/$36 为随证临时量（Room 声明只覆盖 $33/$37-$3A）"),
    (0x003B, 0x000E,
     "AreaPointers_RAM $3B-$48 由 world/room_load.rs:169 copy_area_pointers 写（14 B）"),
    (0x004A, 0x0005,
     "ScrollDir=$49 之外：PAGE_INDEX=$4B/SAMUS_DIR=$4D/SAMUS_DOOR_DIR=$4E 由 "
     "world/object_globals.rs:26 读写（常量见 crates/game-core/src/ram_map.rs）"),
    (0x0053, 0x0019,
     "对象/房间门名面：world/object_globals.rs:26-57 写 PAGE_INDEX/ROOM_NUMBER=$5A/"
     "SPRITE_PAGE_POS=$5B/OBJECT_CNTRL=$6B；title/state.rs:70 写 IS_SAMUS=$55；"
     "world/room_load.rs:247 写 RoomPalette=$68"),
    (0x006D, 0x008F,
     "Samus/滚动零散面：world/room_load.rs:175 读 $6C/$6D ScrollBlock；"
     "samus/state.rs:942 写 HEALTH_CHANGE=$6E；world/elevator.rs:424 写 METROID_ON_SAMUS=$92；"
     "world/update.rs:75 写 MIRROR_CNTRL=$FA；render/oam.rs:257 写 SCROLL_X=$FD/SCROLL_Y=$FC"),
    (0x00FE, 0x0008,
     "PPU 零页镜像 PPUMASK_ZP=$FE/PPUCTRL_ZP=$FF（render/mod.rs:128、frame/nmi.rs）"
     "与 $0100-$0105 的 6502 栈；栈由 frame/nmi.rs 的 JSR/中断路径使用"),
    (0x0108, 0x01F8,
     "6502 栈 $0108-$01FF 与 OAM 影子 $0200-$02FF（render/oam.rs 整页写入）；"
     "原版把栈与 OAM 当硬件面，未进 NesMapped typed 表"),
    (0x04E0, 0x0320,
     "声引擎/房间绘制/VRAM 缓冲工作区：sound/state.rs:25 的 SFX_PAUSED=$0602/SFX_NOISE_INIT=$0680/"
     "MUSIC_INIT=$0685、sound/hook.rs:314 写 $068D；world/room_draw.rs:324-326 的 MEM0700/"
     "PIPE_BUG_HOLES=$0728/POWER_UPS=$0748；pickup/spawn.rs:24 的 POWERUPS_BASE=$0748；"
     "title/intro.rs:80-113 与 frame/nmi.rs:116 的 VRAMStructBuffer=$07A0；"
     "$04E0-$05FF 原版未用保留面（Mesen2 RAM dump 恒 0，比对面由 memmap/observe/dump.rs 界定）"),
]


def _now() -> str:
    return datetime.now(timezone(timedelta(hours=8))).isoformat(timespec="seconds")


def _face_pairs(face) -> list[tuple[int, int]]:
    out = []
    for f in face:
        if isinstance(f, dict):
            out.append((int(f["start"]), int(f["len"])))
        else:
            out.append((int(f[0]), int(f[1])))
    return out


def _face_bytes(face) -> set[int]:
    out: set[int] = set()
    for start, length in _face_pairs(face):
        out.update(range(start, start + length))
    return out


def _span_bytes(span: dict):
    start = int(span["start"])
    return range(start, start + int(span["len"]))


def _is_placeholder(reason) -> bool:
    if not isinstance(reason, str):
        return True
    text = reason.strip()
    return text.lower() in PLACEHOLDER_REASONS or not CODE_POINTER_RE.search(text)


def geometry_sha256(entries) -> str:
    """空洞几何指纹（与 reason 无关）：按 start 排序的 `start:len` 串的 sha256。"""
    canon = ",".join(f"{int(e['start'])}:{int(e['len'])}" for e in sorted(entries, key=lambda e: int(e["start"])))
    return hashlib.sha256(canon.encode("utf-8")).hexdigest()


def entry_sha256(entry: dict) -> str:
    canon = json.dumps(entry, sort_keys=True, ensure_ascii=False, separators=(",", ":"))
    return hashlib.sha256(canon.encode("utf-8")).hexdigest()


def undeclared_segments(spans, face) -> list[dict]:
    """比对面内**任何键都没有声明**的字节，合并成极大连续段。"""
    covered: set[int] = set()
    for span in spans:
        covered.update(_span_bytes(span))
    holes = sorted(_face_bytes(face) - covered)
    out: list[dict] = []
    for addr in holes:
        if out and out[-1]["start"] + out[-1]["len"] == addr:
            out[-1]["len"] += 1
        else:
            out.append({"start": addr, "len": 1})
    return out


def _structure_errors(spans) -> list[str]:
    errs: list[str] = []
    for span in spans:
        entity = span.get("entity", "?")
        if span.get("key") not in KEYS:
            errs.append(f"bad-key ${int(span.get('start', 0)):04X}: {entity} 的 key={span.get('key')!r} 不在 {sorted(KEYS)}")
        if span.get("key") == "unmodeled":
            note = span.get("note")
            if not isinstance(note, str) or not note.strip() or note.strip().lower() in PLACEHOLDER_REASONS:
                errs.append(
                    f"bad-unmodeled-note ${int(span.get('start', 0)):04X}: {entity} 的 Unmodeled "
                    f"必须有非空理由（不接受 TODO/unknown）——整段标 Unmodeled 不是免检通道"
                )
        if not isinstance(span.get("len"), int) or int(span["len"]) <= 0:
            errs.append(f"bad-len ${int(span.get('start', 0)):04X}: {entity} 的 len={span.get('len')!r} 必须为正")
            continue
        if int(span["start"]) < 0 or int(span["start"]) + int(span["len"]) > 0x10000:
            errs.append(f"out-of-space ${int(span['start']):04X}: {entity} 越过 16 位地址空间")
    return errs


def _overlap_errors(spans) -> list[str]:
    by_byte: dict[int, list[dict]] = {}
    for span in spans:
        if span.get("key") != "owner":
            continue
        for addr in _span_bytes(span):
            by_byte.setdefault(addr, []).append(span)
    errs: list[str] = []
    reported: set[tuple[int, ...]] = set()
    for addr in sorted(by_byte):
        claims = by_byte[addr]
        if len(claims) < 2:
            continue
        key = tuple(sorted(id(c) for c in claims))
        if key in reported:
            continue
        reported.add(key)
        names = " 与 ".join(c["entity"] for c in claims)
        errs.append(f"overlap ${addr:04X}: Owner 由 {names} 同时认领（D-03：每字节恰一个 owner）")
    return errs


def _view_errors(spans) -> list[str]:
    owner: dict[int, dict] = {}
    for span in spans:
        if span.get("key") == "owner":
            for addr in _span_bytes(span):
                owner[addr] = span
    errs: list[str] = []
    for span in spans:
        if span.get("key") != "view":
            continue
        missing = [a for a in _span_bytes(span) if a not in owner]
        if missing:
            errs.append(
                f"orphan-view ${int(span['start']):04X}: View({span['entity']}) 有 {len(missing)} B "
                f"没有 Owner 覆盖（首个 ${missing[0]:04X}）——View 只能叠在 typed 真源上"
            )
    return errs


def _unmodeled_errors(spans) -> list[str]:
    owner: dict[int, dict] = {}
    view: dict[int, dict] = {}
    unm: dict[int, list[dict]] = {}
    for span in spans:
        key = span.get("key")
        for addr in _span_bytes(span):
            if key == "owner":
                owner[addr] = span
            elif key == "view":
                view[addr] = span
            elif key == "unmodeled":
                unm.setdefault(addr, []).append(span)
    errs: list[str] = []
    for span in spans:
        if span.get("key") != "unmodeled":
            continue
        for addr in _span_bytes(span):
            other = owner.get(addr) or view.get(addr)
            if other is not None:
                errs.append(
                    f"unmodeled-conflict ${addr:04X}: Unmodeled({span['entity']}) 与 "
                    f"{other['key']}({other['entity']}) 抢同一字节——已声明面必须 typed 或 View，不能退化成 Unmodeled"
                )
                break
            others = [s for s in unm.get(addr, []) if s is not span]
            if others:
                errs.append(
                    f"unmodeled-conflict ${addr:04X}: Unmodeled({span['entity']}) 与 "
                    f"Unmodeled({others[0]['entity']}) 重叠"
                )
                break
    return errs


def _validate_ledger(ledger, holes, face, spans) -> list[str]:
    errs: list[str] = []
    if not isinstance(ledger, dict):
        return ["ledger: 账本必须是 JSON 对象"]
    if ledger.get("schema") != SCHEMA:
        errs.append(f"ledger: schema={ledger.get('schema')!r}，期望 {SCHEMA!r}")
    if _face_pairs(ledger.get("face") or []) != _face_pairs(face):
        errs.append("ledger: face 与 dump 的比对面不一致（比对面只能由 Rust 侧 EXTRA_RANGE 定义）")
    face_total = sum(length for _, length in _face_pairs(face))
    if int(ledger.get("face_total", -1)) != face_total:
        errs.append(f"ledger: face_total={ledger.get('face_total')!r}，期望 {face_total}")
    actual_owner = _owner_bytes(spans, face)
    if ledger.get("owner_bytes") != actual_owner:
        errs.append(
            f"owner-bytes: 账本 {ledger.get('owner_bytes')!r} ≠ 实际 Owner {actual_owner!r}"
            f"（Owner 是 typed 真源，覆盖只能随 typed 字段显式变更，不能靠加宽 span 吞洞）"
        )
    entries = ledger.get("undeclared")
    if not isinstance(entries, list):
        return errs + ["ledger: undeclared 必须是数组"]
    for entry in entries:
        if not isinstance(entry, dict):
            errs.append("ledger: undeclared 每项必须是对象")
            continue
        start, length = int(entry.get("start", -1)), int(entry.get("len", 0))
        if start < 0 or length <= 0:
            errs.append(f"ledger: undeclared 段非法 start={start} len={length}")
            continue
        if any(a not in _face_bytes(face) for a in range(start, start + length)):
            errs.append(f"ledger: undeclared ${start:04X}-${start + length - 1:04X} 越出比对面")
        if _is_placeholder(entry.get("reason")):
            errs.append(
                f"bad-reason ${start:04X}: Undeclared 的 reason 必须非空、非 TODO/unknown，"
                f"且指向真实代码位置（如 world/update.rs 的符号）"
            )
    ordered = sorted(entries, key=lambda e: int(e.get("start", 0)))
    for prev, cur in zip(ordered, ordered[1:]):
        if int(prev["start"]) + int(prev["len"]) > int(cur["start"]):
            errs.append(
                f"ledger: undeclared 段重叠 ${int(cur['start']):04X}（${int(prev['start']):04X} 段尾越过）"
            )
    total = sum(int(e.get("len", 0)) for e in entries)
    if int(ledger.get("undeclared_total", -1)) != total:
        errs.append(f"ledger: undeclared_total={ledger.get('undeclared_total')!r}，实际 {total}")
    actual = {(h["start"], h["len"]) for h in holes}
    recorded = {(int(e["start"]), int(e["len"])) for e in entries if "start" in e and "len" in e}
    for start, length in sorted(actual - recorded):
        errs.append(
            f"new-undeclared ${start:04X}-${start + length - 1:04X}: 比对面内无任何声明，但账本未登记"
            f"（账本外新空洞；若确属未解释面，跑 --write 并写 reason）"
        )
    for start, length in sorted(recorded - actual):
        errs.append(
            f"stale-undeclared ${start:04X}-${start + length - 1:04X}: 账本登记但比对面已无空洞"
            f"（收益未锁定；跑 --write 下调）"
        )
    history = ledger.get("history")
    if not isinstance(history, list) or not history:
        errs.append("history: 缺失或为空——Undeclared 只减不增的下界不可验证")
    else:
        prev_total = None
        prev_entry = None
        for index, record in enumerate(history):
            if not isinstance(record, dict):
                errs.append(f"history[{index}]: 必须是对象")
                continue
            if not str(record.get("ts", "")).strip():
                errs.append(f"history[{index}]: 缺 ts")
            if not str(record.get("reason", "")).strip():
                errs.append(f"history[{index}]: 缺 reason")
            if record.get("owner_bytes") != actual_owner:
                errs.append(f"history[{index}]: owner_bytes 与当前实际不一致（Owner 覆盖被手工改过）")
            try:
                rec_total = int(record["total"])
            except (KeyError, TypeError, ValueError):
                errs.append(f"history[{index}]: total 非法")
                continue
            if prev_total is not None and rec_total >= prev_total:
                errs.append(f"history[{index}]: total {prev_total} → {rec_total} 未严格下降（只减不增）")
            if index == 0:
                if record.get("prev_sha256") != "0" * 64:
                    errs.append(f"history[0]: prev_sha256 必须是 64 个 0（创世锚）")
            elif prev_entry is not None and record.get("prev_sha256") != entry_sha256(prev_entry):
                errs.append(f"history[{index}]: prev_sha256 链断——前一条记录被手工改过")
            prev_total = rec_total
            prev_entry = record
        if prev_total is not None and prev_total != total:
            errs.append(f"history: 尾条 total={prev_total} 与 undeclared_total={total} 不一致")
        last_geom = history[-1].get("geometry_sha256") if isinstance(history[-1], dict) else None
        if last_geom != geometry_sha256(entries):
            errs.append("history: 尾条 geometry_sha256 与当前 undeclared 几何不一致（账本被手工改过）")
        if face_total == 2272 and isinstance(history[0], dict):
            if int(history[0].get("total", -1)) != GENESIS_TOTAL:
                errs.append(
                    f"history[0]: total={history[0].get('total')!r} 与创世锚 {GENESIS_TOTAL} 不一致"
                    f"（创世记录被手工改过；只减不增只能从后续 history 追加）"
                )
            if history[0].get("geometry_sha256") != GENESIS_GEOMETRY_SHA256:
                errs.append("history[0]: geometry_sha256 与创世锚不一致（创世记录被手工改过）")
    return errs


def violations(spans, face=None, ledger=None) -> list[str]:
    """纯判定：返回违规列表（空 = 通过）。`face=None` 时只做结构/重叠检查。"""
    errs = _structure_errors(spans)
    errs += _overlap_errors(spans)
    errs += _view_errors(spans)
    errs += _unmodeled_errors(spans)
    if face is not None:
        holes = undeclared_segments(spans, face)
        if ledger is None:
            for hole in holes:
                errs.append(
                    f"undeclared ${hole['start']:04X}-${hole['start'] + hole['len'] - 1:04X}: "
                    f"比对面内无任何声明（{hole['len']} B）"
                )
        else:
            errs += _validate_ledger(ledger, holes, face, spans)
    if len(errs) > MAX_ERRORS:
        rest = len(errs) - MAX_ERRORS
        errs = errs[:MAX_ERRORS] + [f"... 其余 {rest} 条违规省略"]
    return errs


def _owner_bytes(spans, face) -> list[int]:
    counts = [0] * len(face)
    for index, (start, length) in enumerate(_face_pairs(face)):
        face_set = set(range(start, start + length))
        for span in spans:
            if span.get("key") == "owner":
                counts[index] += len(set(_span_bytes(span)) & face_set)
    return counts


def _default_reason(hole: dict) -> str | None:
    for start, length, reason in DEFAULT_REASONS:
        if start <= hole["start"] and hole["start"] + hole["len"] <= start + length:
            return reason
    return None


def _inherited_reason(hole: dict, old_entries) -> str | None:
    best = None
    best_overlap = 0
    for entry in old_entries:
        lo = max(int(entry["start"]), hole["start"])
        hi = min(int(entry["start"]) + int(entry["len"]), hole["start"] + hole["len"])
        if hi - lo > best_overlap:
            best_overlap = hi - lo
            best = entry.get("reason")
    return best


def make_ledger(dump, old=None, reason=None) -> dict | None:
    """由 dump 重算账本。只许下调：上调 / 等长换洞 / 新洞无 reason 均抛 ValueError。"""
    face = dump["face"]
    spans = dump["spans"]
    holes = undeclared_segments(spans, face)
    old_entries = list(old.get("undeclared") or []) if isinstance(old, dict) else []
    entries = []
    for hole in holes:
        text = _inherited_reason(hole, old_entries)
        if text is None and old is None:
            # 内置 reason 只用于创世建账；后续任何新洞必须显式 --reason 或继承旧洞。
            text = _default_reason(hole)
        if text is None and isinstance(reason, str):
            text = reason
        if not isinstance(text, str) or not text.strip():
            raise ValueError(
                f"新洞 ${hole['start']:04X}-${hole['start'] + hole['len'] - 1:04X} 无 reason："
                f"Undeclared 每段必须写明真实代码位置（不接受 TODO/unknown）"
            )
        entries.append({"start": hole["start"], "len": hole["len"], "reason": text})
    total = sum(e["len"] for e in entries)
    old_total = int(old.get("undeclared_total", 0)) if isinstance(old, dict) else None
    if old_total is not None:
        if total > old_total:
            raise ValueError(f"上调拒绝：Undeclared {old_total} → {total} B（只减不增）")
        if total == old_total:
            if geometry_sha256(entries) == geometry_sha256(old_entries):
                return None
            raise ValueError(
                f"等长换洞拒绝：总长仍为 {total} B 但几何变化（{geometry_sha256(old_entries)[:12]} → "
                f"{geometry_sha256(entries)[:12]}）——同长换洞不是下调"
            )
    owner = _owner_bytes(spans, face)
    if isinstance(old, dict) and old.get("owner_bytes") != owner:
        raise ValueError(
            f"Owner 覆盖变化拒绝：{old.get('owner_bytes')!r} → {owner!r}；"
            f"--write 只下调 Undeclared，新增 typed 字段请单独提交账本 owner_bytes 并说明"
        )
    history = list(old.get("history") or []) if isinstance(old, dict) else []
    if not history:
        history = [{
            "ts": _now(),
            "total": total,
            "geometry_sha256": geometry_sha256(entries),
            "owner_bytes": owner,
            "reason": reason or "genesis：Task 4 建账（P2 比对面内尚无声明的字节）",
            "prev_sha256": "0" * 64,
        }]
    else:
        history.append({
            "ts": _now(),
            "total": total,
            "geometry_sha256": geometry_sha256(entries),
            "owner_bytes": owner,
            "reason": reason or f"下调 {old_total} → {total} B",
            "prev_sha256": entry_sha256(history[-1]),
        })
    ledger = {
        "schema": SCHEMA,
        "face": [{"start": start, "len": length} for start, length in _face_pairs(face)],
        "face_total": sum(length for _, length in _face_pairs(face)),
        "owner_bytes": owner,
        "undeclared": entries,
        "undeclared_total": total,
        "history": history,
    }
    errs = violations(spans, face, ledger)
    if errs:
        raise ValueError("生成的账本自检失败：" + "；".join(errs[:3]))
    return ledger


def load_dump() -> dict:
    proc = subprocess.run(
        DUMP_CMD, cwd=str(ROOT), capture_output=True, text=True,
        encoding="utf-8", errors="replace", timeout=DUMP_TIMEOUT_S,
    )
    if proc.returncode != 0:
        tail = "\n".join((proc.stderr or proc.stdout or "").strip().splitlines()[-8:])
        raise RuntimeError(f"nesmap-dump 失败（exit {proc.returncode}）：\n{tail}")
    lines = [line for line in proc.stdout.splitlines() if line.strip()]
    if not lines:
        raise RuntimeError("nesmap-dump 没有输出")
    data = json.loads(lines[-1])
    if data.get("schema") != DUMP_SCHEMA:
        raise RuntimeError(f"nesmap-dump schema={data.get('schema')!r}，期望 {DUMP_SCHEMA!r}")
    return data


def load_ledger() -> dict | None:
    if not LEDGER.is_file():
        return None
    return json.loads(LEDGER.read_text(encoding="utf-8"))


def cmd_list() -> int:
    dump = load_dump()
    holes = undeclared_segments(dump["spans"], dump["face"])
    ledger = load_ledger()
    recorded = {(int(e["start"]), int(e["len"])): e for e in (ledger or {}).get("undeclared", [])}
    print(f"[INFO] 比对面 {sum(f['len'] for f in dump['face'])} B；"
          f"Owner {dump['coverage'][0]} B + {dump['coverage'][1]} B；Undeclared {sum(h['len'] for h in holes)} B / {len(holes)} 段")
    for hole in holes:
        mark = "账本" if (hole["start"], hole["len"]) in recorded else "未登记"
        reason = recorded.get((hole["start"], hole["len"]), {}).get("reason", "（无）")
        print(f"[{mark}] ${hole['start']:04X}-${hole['start'] + hole['len'] - 1:04X} ({hole['len']} B): {reason}")
    if ledger:
        print(f"[INFO] 账本 undeclared_total={ledger.get('undeclared_total')} B；history {len(ledger.get('history') or [])} 条")
    return 0


def cmd_write(reason: str | None) -> int:
    dump = load_dump()
    old = load_ledger()
    try:
        new = make_ledger(dump, old, reason=reason)
    except ValueError as exc:
        print(f"[FAIL] --write 拒绝：{exc}")
        return 1
    if new is None:
        print("[write] 无变化：Undeclared 未下调，账本未改写")
        return 0
    LEDGER.parent.mkdir(parents=True, exist_ok=True)
    LEDGER.write_text(json.dumps(new, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    before = old.get("undeclared_total") if old else "（新账本）"
    print(f"[write] 账本已更新：Undeclared {before} → {new['undeclared_total']} B（{len(new['undeclared'])} 段）")
    return 0


def cmd_check() -> int:
    try:
        dump = load_dump()
    except (RuntimeError, json.JSONDecodeError, subprocess.TimeoutExpired) as exc:
        print(f"[FAIL] 无法取得 nesmap-dump：{exc}")
        return 1
    ledger = load_ledger()
    if ledger is None:
        print(f"[FAIL] 缺账本 {LEDGER.relative_to(ROOT)}（先跑 python3 tools/nes_map_check.py --write）")
        return 1
    errs = violations(dump["spans"], dump["face"], ledger)
    if errs:
        for line in errs:
            print(f"[FAIL] {line}")
        return 1
    holes = undeclared_segments(dump["spans"], dump["face"])
    print(
        f"[PASS] nes-map：比对面 {sum(f['len'] for f in dump['face'])} B / "
        f"Owner {dump['coverage'][0]}+{dump['coverage'][1]} B / "
        f"Undeclared {sum(h['len'] for h in holes)} B（{len(holes)} 段，账本已锁定）；"
        f"Owner 无重叠、View 全有主、Unmodeled 不抢位"
    )
    return 0


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description="nes-map 静态腿（Owner/View/Unmodeled/Undeclared 三态）")
    group = parser.add_mutually_exclusive_group()
    group.add_argument("--list", action="store_true", help="列出实际空洞与账本 reason")
    group.add_argument("--write", action="store_true", help="把实际空洞写入账本（只许下调）")
    parser.add_argument("--reason", default=None, help="--write 时新洞的 reason（缺省用内置真实代码指针）")
    args = parser.parse_args(argv)
    if args.list:
        return cmd_list()
    if args.write:
        return cmd_write(args.reason)
    return cmd_check()


if __name__ == "__main__":
    sys.exit(main())
