#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""tools/nes_map_check.py —— `nes-map` 静态腿（P2）。

判据（Task 4 brief 追加要求 + Fix round 1 / m00386）：

* **Owner / View / Unmodeled**：`NesMapped::SPANS` 里有声明（Owner 是 typed 真源，
  View 是同一批字节的第二解释，Unmodeled 是“已声明但尚未 typed”的面）。
* **Undeclared**：比对面内**没有任何声明**的字节。登记进 `data/status/nes_map.json`，
  每段必须带指向真实代码位置的 `reason`。
* **四态恒等式**：`Owner + View(only) + Unmodeled + Undeclared == face_total`
  （byte 级；账本与 history 每一条都要成立）。
* **Unmodeled 只减不增**：与 Undeclared 同一条棘轮；`--write` 只允许两者下降，
  且下降只能来自「转成 Owner/View」（`owner_bytes`/`view_bytes` 的增加量对账）。
* **既有 Owner span 不得变形**：`--write` 不允许把某个 Owner span 的 `(start,len)`
  改大/挪位来吞洞；新增 typed 字段必须是**新 span**，且只能盖住旧的
  Undeclared/Unmodeled 字节。
* 重叠 / 跨域冲突 / 账本外新空洞 / Unmodeled 包含 Owner / 手工改账本：FAIL。

空洞的真相**不来自账本**：本工具每次跑 `cargo run -q -p game-core --bin nesmap-dump`，
用 Rust 侧 `NesMapped::SPANS` 与 `memmap::observe::dump::EXTRA_RANGE`（= oracle ram.hex
同一比对面，2272 B）机械重算；账本只是“已批准的空洞/未 typed 清单”，被逐段与重算
结果**精确比对**。

用法：
    python3 tools/nes_map_check.py            # 校验（门禁用；0=通过，1=FAIL）
    python3 tools/nes_map_check.py --list     # 列出实际空洞/Unmodeled 与账本 reason
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
SCHEMA = "nes-map-v2"
DUMP_SCHEMA = "nesmap-dump-v1"
DUMP_CMD = ["cargo", "run", "-q", "-p", "game-core", "--bin", "nesmap-dump"]
DUMP_TIMEOUT_S = 180
MAX_ERRORS = 24

# 创世锚（P2 比对面 2272 B）：Fix round 1 建账时的四态与状态指纹。
# 只对真实比对面（face_total==2272）生效；用于检出「手工改 history[0] 洗白新洞」。
GENESIS_OWNER_TOTAL = 107
GENESIS_VIEW_BYTES = 23
GENESIS_VIEW_ONLY_BYTES = 0
GENESIS_UNMODELED_TOTAL = 612
GENESIS_UNDECLARED_TOTAL = 1553
GENESIS_STATE_SHA256 = "c56cb4693657ebd1372d130042c38b87017edf855019f902a072b9f2a121bdb4"  # Fix round 1 创世状态

KEYS = {"owner", "view", "unmodeled", "case", "mode", "ephemeral"}
PLACEHOLDER_REASONS = {
    "", "todo", "tbd", "fixme", "unknown", "n/a", "na", "?", "??",
    "待定", "未知", "不详", "待补", "无", "none", "null",
}
# reason 必须指向真实代码位置（可 grep 的 file.rs / file.py）；单说“没人管”不算。
CODE_POINTER_RE = re.compile(r"\.(rs|py)\b")

# 建账时每段 Undeclared 的初始 reason（真实代码指针，可 grep 验证）。
# Fix round 1 把原 `$0108-$02FF` 的 504 B 拆成三段：延时旗 / 6502 栈页 / OAM 影子页，
# 不再用一句“整页写入”盖住半真半假的 504 B。
DEFAULT_REASONS = [
    (0x0000, 0x0033,
     "零页系统面（Joy1Status=$14/Joy2Status=$15/GameMode=$1D/FrameCount=$2D 等），"
     "常量与证据见 crates/game-core/src/ram_map.rs，写者分散在 world/update.rs、frame/nmi.rs"),
    (0x0034, 0x0003,
     "RoomPtr hi=$34 由 world/room_load.rs:236 写；$35/$36 为随证临时量（Room 声明只覆盖 $33/$37-$3A）"),
    (0x003B, 0x000E,
     "AreaPointers_RAM $3B-$48 由 world/room_load.rs:169 copy_area_pointers 写（14 B）"),
    (0x004A, 0x0005,
     "PAGE_INDEX=$4B/SAMUS_DIR=$4D/SAMUS_DOOR_DIR=$4E 由 world/object_globals.rs:26 读写"
     "（常量见 crates/game-core/src/ram_map.rs；ScrollDir=$49 已由 Camera 声明）"),
    (0x0053, 0x0019,
     "对象/房间门名面：world/object_globals.rs:26-57 写 PAGE_INDEX/ROOM_NUMBER=$5A/"
     "SPRITE_PAGE_POS=$5B/OBJECT_CNTRL=$6B；title/state.rs:70 写 IS_SAMUS=$55；"
     "world/room_load.rs:247 写 RoomPalette=$68"),
    (0x006D, 0x008F,
     "Samus/滚动零散面：world/room_load.rs:175 读 $6C/$6D ScrollBlock；"
     "samus/state.rs:942 写 HEALTH_CHANGE=$6E；world/elevator.rs:424 写 METROID_ON_SAMUS=$92；"
     "world/update.rs:75 写 MIRROR_CNTRL=$FA；$FC/$FD 已由 Camera 声明，不在本段"),
    (0x00FE, 0x0008,
     "PPU 零页镜像 PPUMASK_ZP=$FE/PPUCTRL_ZP=$FF（render/mod.rs:128、frame/nmi.rs）"
     "与 $0100-$0105 的 6502 栈；栈由 frame/nmi.rs 的 JSR/中断路径使用"),
    (0x0108, 0x0002,
     "MiniBossKillDelayFlag=$0108/PowerUpDelayFlag=$0109 由 frame/tick.rs:637 读写"
     "（asm C4AA SetTimer 路径，constants_ram:809-810）"),
    (0x010A, 0x00F6,
     "6502 硬件栈页剩余部分（$0100-$01FF）；Rust 侧不模拟 JSR/RTS 的栈字节，"
     "frame/boot.rs:581 断言 $01FF 不被触碰；$0108/$0109 是原版借栈页放的延时旗，已单列"),
    (0x0200, 0x0100,
     "OAM 影子页：frame/boot.rs:144 erase_all_sprites 整页 256 B 填 $F0；"
     "render/oam.rs:107 draw_sprite_stream 按 SpritePagePos 写四元组（最远 $02FF）；"
     "frame/nmi.rs 读 OAM"),
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


def _segments(addrs) -> list[dict]:
    out: list[dict] = []
    for addr in sorted(addrs):
        if out and out[-1]["start"] + out[-1]["len"] == addr:
            out[-1]["len"] += 1
        else:
            out.append({"start": addr, "len": 1})
    return out


def _is_placeholder(reason) -> bool:
    if not isinstance(reason, str):
        return True
    text = reason.strip()
    return text.lower() in PLACEHOLDER_REASONS or not CODE_POINTER_RE.search(text)


def geometry_sha256(entries) -> str:
    """空洞几何指纹（与 reason 无关）：按 start 排序的 `start:len` 串的 sha256。"""
    canon = ",".join(
        f"{int(e['start'])}:{int(e['len'])}"
        for e in sorted(entries, key=lambda e: int(e["start"]))
    )
    return hashlib.sha256(canon.encode("utf-8")).hexdigest()


def state_sha256(ledger: dict) -> str:
    """四态状态指纹：Owner span 几何 + View 计数 + Unmodeled/Undeclared（含 reason）。"""
    payload = {
        "owner_spans": ledger.get("owner_spans"),
        "view_bytes": ledger.get("view_bytes"),
        "view_only_bytes": ledger.get("view_only_bytes"),
        "unmodeled": ledger.get("unmodeled"),
        "undeclared": ledger.get("undeclared"),
    }
    canon = json.dumps(payload, sort_keys=True, ensure_ascii=False, separators=(",", ":"))
    return hashlib.sha256(canon.encode("utf-8")).hexdigest()


def entry_sha256(entry: dict) -> str:
    canon = json.dumps(entry, sort_keys=True, ensure_ascii=False, separators=(",", ":"))
    return hashlib.sha256(canon.encode("utf-8")).hexdigest()


def undeclared_segments(spans, face) -> list[dict]:
    """比对面内**任何键都没有声明**的字节，合并成极大连续段。"""
    part = _partition(spans, face)
    return part["undeclared"]


def _owner_bytes(spans, face) -> list[int]:
    counts = [0] * len(face)
    for index, (start, length) in enumerate(_face_pairs(face)):
        face_set = set(range(start, start + length))
        for span in spans:
            if span.get("key") == "owner":
                counts[index] += len(set(_span_bytes(span)) & face_set)
    return counts


def _partition(spans, face) -> dict:
    """把声明机械分成四态：Owner / View(only) / Unmodeled / Undeclared。"""
    face_set = _face_bytes(face)
    owner_spans = [s for s in spans if s.get("key") == "owner"]
    owner_set: set[int] = set()
    view_set: set[int] = set()
    unmodeled_set: set[int] = set()
    other_set: set[int] = set()
    for span in spans:
        key = span.get("key")
        if key == "owner":
            owner_set.update(_span_bytes(span))
        elif key == "view":
            view_set.update(_span_bytes(span))
        elif key == "unmodeled":
            unmodeled_set.update(_span_bytes(span))
        elif key in ("case", "mode", "ephemeral"):
            other_set.update(_span_bytes(span))
    owner_set &= face_set
    view_set &= face_set
    unmodeled_set &= face_set
    other_set &= face_set
    declared = owner_set | view_set | unmodeled_set | other_set
    undeclared = _segments(face_set - declared)
    view_only = view_set - owner_set
    # 账本只登记**比对面内**的 Unmodeled；面外声明（如 Room 的 $6000-$67FF WRAM 缓冲）
    # 不参与 P2 四态恒等式，但仍由结构/重叠检查覆盖。
    unmodeled: list[dict] = []
    for s in sorted(
        (s for s in spans if s.get("key") == "unmodeled"),
        key=lambda s: int(s["start"]),
    ):
        for piece in _segments(set(_span_bytes(s)) & face_set):
            unmodeled.append({
                "start": piece["start"],
                "len": piece["len"],
                "reason": s.get("note"),
            })
    owner_geometry = [
        {"start": int(s["start"]), "len": int(s["len"])}
        for s in sorted(owner_spans, key=lambda s: int(s["start"]))
    ]
    return {
        "face_total": sum(length for _, length in _face_pairs(face)),
        "owner_bytes": _owner_bytes(spans, face),
        "owner_total": len(owner_set),
        "owner_spans": owner_geometry,
        "view_bytes": len(view_set),
        "view_only_bytes": len(view_only),
        "unmodeled_total": len(unmodeled_set),
        "unmodeled": unmodeled,
        "undeclared_total": sum(h["len"] for h in undeclared),
        "undeclared": undeclared,
        "identity_total": len(owner_set) + len(view_only) + len(unmodeled_set) + sum(
            h["len"] for h in undeclared
        ),
        "declared": declared,
        "undeclared_set": face_set - declared,
    }


def _structure_errors(spans) -> list[str]:
    errs: list[str] = []
    for span in spans:
        entity = span.get("entity", "?")
        if span.get("key") not in KEYS:
            errs.append(
                f"bad-key ${int(span.get('start', 0)):04X}: {entity} 的 key={span.get('key')!r} "
                f"不在 {sorted(KEYS)}"
            )
        if span.get("key") == "unmodeled":
            note = span.get("note")
            if _is_placeholder(note):
                errs.append(
                    f"bad-unmodeled-note ${int(span.get('start', 0)):04X}: {entity} 的 Unmodeled "
                    f"必须带非占位、可 grep 的代码指针（*.rs/*.py），不接受 TODO/unknown/"
                    f"整段未建模——整段标 Unmodeled 不是免检通道"
                )
        if not isinstance(span.get("len"), int) or int(span["len"]) <= 0:
            errs.append(
                f"bad-len ${int(span.get('start', 0)):04X}: {entity} 的 len={span.get('len')!r} 必须为正"
            )
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
    for span in spans:
        if span.get("key") not in ("case", "mode", "ephemeral"):
            continue
        missing = [a for a in _span_bytes(span) if a not in owner]
        if missing:
            errs.append(
                f"orphan-overlay ${int(span['start']):04X}: {span['key']}({span['entity']}) 有 "
                f"{len(missing)} B 没有 Owner 覆盖（首个 ${missing[0]:04X}）"
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
    owner_spans = [s for s in spans if s.get("key") == "owner"]
    errs: list[str] = []
    for span in spans:
        if span.get("key") != "unmodeled":
            continue
        start = int(span["start"])
        end = start + int(span["len"]) - 1
        for addr in _span_bytes(span):
            if addr in view:
                errs.append(
                    f"unmodeled-overlap-view ${addr:04X}: Unmodeled({span['entity']}) 与 "
                    f"view({view[addr]['entity']}) 抢同一字节"
                )
                break
            if addr in owner:
                errs.append(
                    f"unmodeled-overlap-owner ${addr:04X}: Unmodeled({span['entity']}) 与 "
                    f"owner({owner[addr]['entity']}) 抢同一字节——已声明面不能退化成 Unmodeled"
                )
                break
            others = [s for s in unm.get(addr, []) if s is not span]
            if others:
                errs.append(
                    f"unmodeled-conflict ${addr:04X}: Unmodeled({span['entity']}) 与 "
                    f"Unmodeled({others[0]['entity']}) 重叠"
                )
                break
        for os in owner_spans:
            ostart = int(os["start"])
            oend = ostart + int(os["len"]) - 1
            if start <= ostart and oend <= end:
                errs.append(
                    f"unmodeled-contains-owner ${start:04X}: Unmodeled({span['entity']}) "
                    f"覆盖 ${ostart:04X}-${oend:04X} 的 Owner({os['entity']})——未 typed 面不能包住 typed 真源"
                )
    return errs


def _validate_ledger(ledger, part: dict, face) -> list[str]:
    errs: list[str] = []
    if not isinstance(ledger, dict):
        return ["ledger: 账本必须是 JSON 对象"]
    if ledger.get("schema") != SCHEMA:
        errs.append(f"ledger: schema={ledger.get('schema')!r}，期望 {SCHEMA!r}")
    if _face_pairs(ledger.get("face") or []) != _face_pairs(face):
        errs.append("ledger: face 与 dump 的比对面不一致（比对面只能由 Rust 侧 EXTRA_RANGE 定义）")
    if int(ledger.get("face_total", -1)) != part["face_total"]:
        errs.append(f"ledger: face_total={ledger.get('face_total')!r}，期望 {part['face_total']}")
    if ledger.get("owner_bytes") != part["owner_bytes"]:
        errs.append(
            f"owner-bytes: 账本 {ledger.get('owner_bytes')!r} ≠ 实际 Owner {part['owner_bytes']!r}"
            f"（Owner 是 typed 真源，覆盖只能随 typed 字段显式变更，不能靠加宽 span 吞洞）"
        )
    if ledger.get("owner_spans") != part["owner_spans"]:
        errs.append(
            "owner-spans: 账本 Owner span 几何与 dump 不一致"
            "（手工改 span 或加宽吞洞都会在这里红）"
        )
    if ledger.get("view_bytes") != part["view_bytes"]:
        errs.append(f"view-bytes: 账本 {ledger.get('view_bytes')!r} ≠ 实际 View {part['view_bytes']}")
    if ledger.get("view_only_bytes") != part["view_only_bytes"]:
        errs.append(
            f"view-only: 账本 {ledger.get('view_only_bytes')!r} ≠ 实际 View(only) "
            f"{part['view_only_bytes']}"
        )
    unmodeled = ledger.get("unmodeled")
    if not isinstance(unmodeled, list):
        errs.append("ledger: unmodeled 必须是数组")
        unmodeled = []
    for entry in unmodeled:
        if not isinstance(entry, dict):
            errs.append("ledger: unmodeled 每项必须是对象")
            continue
        start = int(entry.get("start", -1))
        if _is_placeholder(entry.get("reason")):
            errs.append(
                f"bad-unmodeled-note ${start:04X}: 账本 Unmodeled 的 reason 必须带可 grep 的代码指针"
            )
    if unmodeled != part["unmodeled"]:
        errs.append(
            "unmodeled: 账本 Unmodeled 段/理由与 dump 声明不一致"
            "（未 typed 面必须与 Rust 侧 SPANS 逐段同源）"
        )
    unmodeled_total = sum(int(e.get("len", 0)) for e in unmodeled if isinstance(e, dict))
    if int(ledger.get("unmodeled_total", -1)) != unmodeled_total:
        errs.append(f"ledger: unmodeled_total={ledger.get('unmodeled_total')!r}，实际 {unmodeled_total}")
    if unmodeled_total != part["unmodeled_total"]:
        errs.append(
            f"unmodeled-total: 账本 {unmodeled_total} B ≠ 实际 {part['unmodeled_total']} B"
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
    # 账本可以把一个极大空洞拆成多段（不同 reason），所以按**字节集合**比对，
    # 而不是按 (start,len) 元组——拆段/并段只要覆盖的字节完全一致就合法。
    actual = set(part["undeclared_set"])
    recorded: set[int] = set()
    for entry in entries:
        if isinstance(entry, dict) and "start" in entry and "len" in entry:
            recorded.update(range(int(entry["start"]), int(entry["start"]) + int(entry["len"])))
    for gap in _segments(actual - recorded):
        errs.append(
            f"new-undeclared ${gap['start']:04X}-${gap['start'] + gap['len'] - 1:04X}: "
            f"比对面内无任何声明，但账本未登记"
            f"（账本外新空洞；若确属未解释面，跑 --write 并写 reason）"
        )
    for gap in _segments(recorded - actual):
        errs.append(
            f"stale-undeclared ${gap['start']:04X}-${gap['start'] + gap['len'] - 1:04X}: "
            f"账本登记但比对面已无空洞（收益未锁定；跑 --write 下调）"
        )

    # 四态恒等式（账本当前状态）
    identity = (
        sum(part["owner_bytes"])
        + part["view_only_bytes"]
        + part["unmodeled_total"]
        + part["undeclared_total"]
    )
    if identity != part["face_total"]:
        errs.append(
            f"恒等式不成立：Owner {sum(part['owner_bytes'])} + View(only) {part['view_only_bytes']} + "
            f"Unmodeled {part['unmodeled_total']} + Undeclared {part['undeclared_total']} = "
            f"{identity} ≠ face_total {part['face_total']}"
        )
    ledger_identity = (
        sum(ledger.get("owner_bytes") or [])
        + int(ledger.get("view_only_bytes", -1))
        + int(ledger.get("unmodeled_total", -1))
        + int(ledger.get("undeclared_total", -1))
    )
    if ledger_identity != part["face_total"]:
        errs.append(
            f"恒等式不成立（账本）：Owner {sum(ledger.get('owner_bytes') or [])} + "
            f"View(only) {ledger.get('view_only_bytes')} + Unmodeled {ledger.get('unmodeled_total')} + "
            f"Undeclared {ledger.get('undeclared_total')} = {ledger_identity} ≠ face_total {part['face_total']}"
        )

    history = ledger.get("history")
    if not isinstance(history, list) or not history:
        errs.append("history: 缺失或为空——四态恒等式/只减不增的下界不可验证")
        return errs
    prev = None
    for index, record in enumerate(history):
        if not isinstance(record, dict):
            errs.append(f"history[{index}]: 必须是对象")
            continue
        if not str(record.get("ts", "")).strip():
            errs.append(f"history[{index}]: 缺 ts")
        if not str(record.get("reason", "")).strip():
            errs.append(f"history[{index}]: 缺 reason")
        try:
            totals = {
                "owner": int(record["owner_total"]),
                "view": int(record["view_bytes"]),
                "view_only": int(record["view_only_bytes"]),
                "unmodeled": int(record["unmodeled_total"]),
                "undeclared": int(record["undeclared_total"]),
            }
        except (KeyError, TypeError, ValueError):
            errs.append(f"history[{index}]: 四态计数缺失/非法")
            continue
        hist_identity = totals["owner"] + totals["view_only"] + totals["unmodeled"] + totals["undeclared"]
        if hist_identity != part["face_total"]:
            errs.append(
                f"history[{index}]: 恒等式不成立 "
                f"{totals['owner']}+{totals['view_only']}+{totals['unmodeled']}+{totals['undeclared']}"
                f"={hist_identity} ≠ face_total {part['face_total']}"
            )
        if index == 0:
            if record.get("prev_sha256") != "0" * 64:
                errs.append("history[0]: prev_sha256 必须是 64 个 0（创世锚）")
        elif prev is not None and record.get("prev_sha256") != entry_sha256(prev):
            errs.append(f"history[{index}]: prev_sha256 链断——前一条记录被手工改过")
        if prev is not None:
            try:
                p = {
                    "owner": int(prev["owner_total"]),
                    "view_only": int(prev["view_only_bytes"]),
                    "unmodeled": int(prev["unmodeled_total"]),
                    "undeclared": int(prev["undeclared_total"]),
                }
            except (KeyError, TypeError, ValueError):
                p = None
            if p is not None:
                if totals["unmodeled"] > p["unmodeled"]:
                    errs.append(
                        f"history[{index}]: Unmodeled 只减不增（{p['unmodeled']} → {totals['unmodeled']} B）"
                    )
                if totals["undeclared"] > p["undeclared"]:
                    errs.append(
                        f"history[{index}]: Undeclared 只减不增（{p['undeclared']} → {totals['undeclared']} B）"
                    )
                if totals["owner"] + totals["view_only"] < p["owner"] + p["view_only"]:
                    errs.append(f"history[{index}]: Owner/View 回退（typed 覆盖不能减少）")
                lhs = (totals["owner"] + totals["view_only"]) - (p["owner"] + p["view_only"])
                rhs = (p["unmodeled"] + p["undeclared"]) - (totals["unmodeled"] + totals["undeclared"])
                if lhs != rhs:
                    errs.append(
                        f"history[{index}]: 转换等式不成立（Owner/View +{lhs} ≠ "
                        f"Unmodeled/Undeclared -{rhs}）——下降只能来自转成 Owner/View"
                    )
        prev = record

    tail = history[-1] if isinstance(history[-1], dict) else {}
    current = {
        "owner_total": sum(part["owner_bytes"]),
        "view_bytes": part["view_bytes"],
        "view_only_bytes": part["view_only_bytes"],
        "unmodeled_total": part["unmodeled_total"],
        "undeclared_total": part["undeclared_total"],
        "state_sha256": state_sha256(ledger),
    }
    for field, value in current.items():
        if tail.get(field) != value:
            errs.append(
                f"history: 尾条 {field}={tail.get(field)!r} 与当前账本 {value!r} 不一致"
                f"（账本被手工改过；请用 --write 追加 history）"
            )
    if part["face_total"] == 2272 and isinstance(history[0], dict):
        anchor = {
            "owner_total": GENESIS_OWNER_TOTAL,
            "view_bytes": GENESIS_VIEW_BYTES,
            "view_only_bytes": GENESIS_VIEW_ONLY_BYTES,
            "unmodeled_total": GENESIS_UNMODELED_TOTAL,
            "undeclared_total": GENESIS_UNDECLARED_TOTAL,
        }
        for field, value in anchor.items():
            if int(history[0].get(field, -1)) != value:
                errs.append(
                    f"history[0]: {field}={history[0].get(field)!r} 与创世锚 {value} 不一致"
                    f"（创世记录被手工改过；四态只能从后续 history 演进）"
                )
        if GENESIS_STATE_SHA256 and history[0].get("state_sha256") != GENESIS_STATE_SHA256:
            errs.append("history[0]: state_sha256 与创世锚不一致（创世记录被手工改过）")
    return errs


def violations(spans, face=None, ledger=None) -> list[str]:
    """纯判定：返回违规列表（空 = 通过）。`face=None` 时只做结构/重叠检查。"""
    errs = _structure_errors(spans)
    errs += _overlap_errors(spans)
    errs += _view_errors(spans)
    errs += _unmodeled_errors(spans)
    if face is not None:
        part = _partition(spans, face)
        if ledger is None:
            for hole in part["undeclared"]:
                errs.append(
                    f"undeclared ${hole['start']:04X}-${hole['start'] + hole['len'] - 1:04X}: "
                    f"比对面内无任何声明（{hole['len']} B）"
                )
        else:
            errs += _validate_ledger(ledger, part, face)
    if len(errs) > MAX_ERRORS:
        rest = len(errs) - MAX_ERRORS
        errs = errs[:MAX_ERRORS] + [f"... 其余 {rest} 条违规省略"]
    return errs


def _split_by_defaults(hole: dict) -> list[dict]:
    """把一个极大空洞按 DEFAULT_REASONS 的边界切开（reason 粒度可细于空洞粒度）。"""
    lo = int(hole["start"])
    hi = lo + int(hole["len"])
    cuts = {lo, hi}
    for start, length, _ in DEFAULT_REASONS:
        end = start + length
        if lo < start < hi:
            cuts.add(start)
        if lo < end < hi:
            cuts.add(end)
    points = sorted(cuts)
    return [{"start": a, "len": b - a} for a, b in zip(points, points[1:]) if b > a]


def _default_reason(hole: dict) -> str | None:
    for start, length, reason in DEFAULT_REASONS:
        if hole["start"] == start and hole["len"] == length:
            return reason
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


def _old_bytes(entries) -> set[int]:
    out: set[int] = set()
    for entry in entries:
        out.update(range(int(entry["start"]), int(entry["start"]) + int(entry["len"])))
    return out


def make_ledger(dump, old=None, reason=None) -> dict | None:
    """由 dump 重算账本。只许下调：上调 / 等长改账 / Owner span 变形 / 新洞无 reason 均抛 ValueError。"""
    face = dump["face"]
    spans = dump["spans"]
    # v1 账本没有 Unmodeled/View 四态字段，无法做同口径棘轮；Fix round 1 一次性重建为 v2。
    # 这是 schema 升级，不是"下调"：重建后的四态恒等式从创世锚重新起算。
    if isinstance(old, dict) and old.get("schema") != SCHEMA:
        old = None
    part = _partition(spans, face)
    old_entries = list(old.get("undeclared") or []) if isinstance(old, dict) else []
    old_unmodeled = list(old.get("unmodeled") or []) if isinstance(old, dict) else []
    entries = []
    for hole in part["undeclared"]:
        for piece in _split_by_defaults(hole):
            text = _inherited_reason(piece, old_entries)
            if text is None and old is None:
                # 内置 reason 只用于创世建账；后续任何新洞必须显式 --reason 或继承旧洞。
                text = _default_reason(piece)
            if text is None and isinstance(reason, str):
                text = reason
            if not isinstance(text, str) or not text.strip():
                raise ValueError(
                    f"新洞 ${piece['start']:04X}-${piece['start'] + piece['len'] - 1:04X} 无 reason："
                    f"Undeclared 每段必须写明真实代码位置（不接受 TODO/unknown）"
                )
            entries.append({"start": piece["start"], "len": piece["len"], "reason": text})
    total = sum(e["len"] for e in entries)

    if isinstance(old, dict):
        old_unmodeled_total = int(old.get("unmodeled_total", 0))
        if part["unmodeled_total"] > old_unmodeled_total:
            raise ValueError(
                f"Unmodeled 只减不增：{old_unmodeled_total} → {part['unmodeled_total']} B"
                f"（禁止把 Undeclared 平移成 Unmodeled；要转只能转成 Owner/View）"
            )
        old_total = int(old.get("undeclared_total", 0))
        if total > old_total:
            raise ValueError(f"上调拒绝：Undeclared {old_total} → {total} B（只减不增）")

        old_owner = {(int(e["start"]), int(e["len"])) for e in (old.get("owner_spans") or [])}
        new_owner = {(int(e["start"]), int(e["len"])) for e in part["owner_spans"]}
        removed = old_owner - new_owner
        if removed:
            sample = sorted(removed)[0]
            raise ValueError(
                f"既有 Owner span 消失/变形：${sample[0]:04X}+{sample[1]}"
                f"（--write 不允许加宽/移动已有 Owner span 来吞洞；新增 typed 字段应是新 span）"
            )
        old_holes = _old_bytes(old_entries)
        old_unm = _old_bytes(old_unmodeled)
        old_owner_bytes = _old_bytes(old.get("owner_spans") or [])
        old_view_bytes = _old_bytes([
            {"start": s["start"], "len": s["len"]}
            for s in (old.get("view_spans") or [])
        ])
        for start, length in sorted(new_owner - old_owner):
            covered = set(range(start, start + length))
            if covered & old_owner_bytes:
                raise ValueError(
                    f"新增 Owner span ${start:04X}+{length} 与旧 Owner 重叠——Owner 重叠不是合法新增"
                )
            if covered & old_view_bytes:
                raise ValueError(
                    f"新增 Owner span ${start:04X}+{length} 与旧 View 重叠——View 只能叠在 Owner 上"
                )
            if not covered <= (old_holes | old_unm):
                raise ValueError(
                    f"新增 Owner span ${start:04X}+{length} 覆盖了旧账本未列为空洞/Unmodeled 的字节"
                )

        old_owner_total = sum(int(v) for v in (old.get("owner_bytes") or []))
        old_view_only = int(old.get("view_only_bytes", 0))
        delta_owner = sum(part["owner_bytes"]) - old_owner_total
        delta_view = part["view_only_bytes"] - old_view_only
        delta_unmodeled = part["unmodeled_total"] - old_unmodeled_total
        delta_undeclared = total - old_total
        if delta_owner + delta_view != -(delta_unmodeled + delta_undeclared):
            raise ValueError(
                f"转换等式不成立：Owner/View +{delta_owner + delta_view} ≠ "
                f"Unmodeled/Undeclared -{-(delta_unmodeled + delta_undeclared)} B"
                f"（下降只能来自转成 Owner/View）"
            )

        old_state = state_sha256(old)
        candidate = {
            "schema": SCHEMA,
            "face": [{"start": s, "len": n} for s, n in _face_pairs(face)],
            "face_total": part["face_total"],
            "owner_bytes": part["owner_bytes"],
            "owner_spans": part["owner_spans"],
            "view_bytes": part["view_bytes"],
            "view_only_bytes": part["view_only_bytes"],
            "unmodeled_total": part["unmodeled_total"],
            "unmodeled": part["unmodeled"],
            "undeclared_total": total,
            "undeclared": entries,
            "history": list(old.get("history") or []),
        }
        new_state = state_sha256(candidate)
        if total == old_total and part["unmodeled_total"] == old_unmodeled_total and new_state != old_state:
            raise ValueError(
                f"等长改账拒绝：Undeclared/Unmodeled 总量未下降但几何/理由变化"
                f"（{old_state[:12]} → {new_state[:12]}）——同长换洞不是下调"
            )
        if total == old_total and part["unmodeled_total"] == old_unmodeled_total and new_state == old_state:
            return None

    history = list(old.get("history") or []) if isinstance(old, dict) else []
    history_entry = {
        "ts": _now(),
        "reason": reason or (
            "genesis：Fix round 1 建账（四态恒等式 + Unmodeled 只减不增）"
            if not history else
            f"下调 Undeclared {int(old.get('undeclared_total', 0))} → {total} B / "
            f"Unmodeled {int(old.get('unmodeled_total', 0))} → {part['unmodeled_total']} B"
        ),
        "owner_total": sum(part["owner_bytes"]),
        "view_bytes": part["view_bytes"],
        "view_only_bytes": part["view_only_bytes"],
        "unmodeled_total": part["unmodeled_total"],
        "undeclared_total": total,
        "state_sha256": "",
        "prev_sha256": entry_sha256(history[-1]) if history else "0" * 64,
    }
    ledger = {
        "schema": SCHEMA,
        "face": [{"start": s, "len": n} for s, n in _face_pairs(face)],
        "face_total": part["face_total"],
        "owner_bytes": part["owner_bytes"],
        "owner_spans": part["owner_spans"],
        "view_bytes": part["view_bytes"],
        "view_only_bytes": part["view_only_bytes"],
        "unmodeled_total": part["unmodeled_total"],
        "unmodeled": part["unmodeled"],
        "undeclared_total": total,
        "undeclared": entries,
        "history": history + [history_entry],
    }
    history_entry["state_sha256"] = state_sha256(ledger)
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
    part = _partition(dump["spans"], dump["face"])
    ledger = load_ledger()
    recorded = {(int(e["start"]), int(e["len"])): e for e in (ledger or {}).get("undeclared", [])}
    print(
        f"[INFO] 比对面 {part['face_total']} B；Owner {sum(part['owner_bytes'])} B / "
        f"View {part['view_bytes']} B（only {part['view_only_bytes']}）/ "
        f"Unmodeled {part['unmodeled_total']} B / Undeclared {part['undeclared_total']} B"
    )
    for hole in part["undeclared"]:
        mark = "账本" if (hole["start"], hole["len"]) in recorded else "未登记"
        reason = recorded.get((hole["start"], hole["len"]), {}).get("reason", "（无）")
        print(f"[{mark}] ${hole['start']:04X}-${hole['start'] + hole['len'] - 1:04X} ({hole['len']} B): {reason}")
    for entry in part["unmodeled"]:
        print(f"[Unmodeled] ${entry['start']:04X}-${entry['start'] + entry['len'] - 1:04X} "
              f"({entry['len']} B): {entry['reason']}")
    if ledger:
        print(
            f"[INFO] 账本 Undeclared={ledger.get('undeclared_total')} B / "
            f"Unmodeled={ledger.get('unmodeled_total')} B；history {len(ledger.get('history') or [])} 条"
        )
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
        print("[write] 无变化：Undeclared/Unmodeled 未下调，账本未改写")
        return 0
    LEDGER.parent.mkdir(parents=True, exist_ok=True)
    LEDGER.write_text(json.dumps(new, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    before_u = old.get("undeclared_total") if old else "（新账本）"
    before_m = old.get("unmodeled_total") if old else "（新账本）"
    print(
        f"[write] 账本已更新：Undeclared {before_u} → {new['undeclared_total']} B / "
        f"Unmodeled {before_m} → {new['unmodeled_total']} B"
    )
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
    part = _partition(dump["spans"], dump["face"])
    print(
        f"[PASS] nes-map：比对面 {part['face_total']} B / "
        f"Owner {sum(part['owner_bytes'])} B / View {part['view_bytes']} B（only {part['view_only_bytes']}）/ "
        f"Unmodeled {part['unmodeled_total']} B / Undeclared {part['undeclared_total']} B"
        f"（{len(part['undeclared'])} 段）；四态恒等式成立，Unmodeled/Undeclared 已锁定"
    )
    return 0


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description="nes-map 静态腿（Owner/View/Unmodeled/Undeclared 四态）")
    group = parser.add_mutually_exclusive_group()
    group.add_argument("--list", action="store_true", help="列出实际空洞/Unmodeled 与账本 reason")
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
