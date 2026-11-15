#!/usr/bin/env python3
"""bankvars —— 按区（per-bank）变量机械分母：扫 asm 标签 + 比 ROM 字节，得出
「同一地址在各区 bank 内容不同」的清单，并与登记账本对账。

为什么需要（docs/findings/per-bank-assets.md）：NES 的 `$8000-$FFFF` 是 16 KB 窗口，
同一个地址（如 `$95E5`）在五个区 bank 各有一份**不同**的数据。Rust 侧一旦用单张表
承载，某个区就会读到别人的字节——这类错在测试里看着全绿（因为测试也只喂一个区）。

本工具的判据全部机械：
  1. 地址区（分母定义，不是人工挑的「危险清单」）：
     - `$95C3-$95E5`  区头（AreaRoutine/AreaMusicFlag/AreaEnemyDamage/…/ChooseEnemyAI）
     - `$960B-$97D5`  敌表区（EnemyActiveAnimIndex/EnemyHealthTbl/EnMovement…）
     - 五区各自的 EnAnimTable / EnFramePtrTable1 / EnPlacePtrTable 起始段
     （基址见 crates/game-data/src/extract.rs::ENEMY_SPRITES）
  2. 扫描 `refs/m1disasm/SRC/prg{1..5}_*.asm` 的标签行 `Name: ; BB:XXXX`，取落在上述
     地址区的标签；同一地址在 ≥2 个区出现时，比 ROM 在该 bank 的字节窗口。
  3. 只要有一个区的字节与别区不同 ⇒ 该地址是**按区变量**，必须进账本。
  4. 账本（data/status/bankvars.json）每项状态二选一：
     - `covered`：已有按区访问器（写明 accessor 指针）；
     - `pending`：尚未按区化——**只减不增**（与 unported 同口径）。

用法：
    python3 tools/bankvars.py            # 校验（门禁用）
    python3 tools/bankvars.py --list     # 打印发现的按区变量（人读）
    python3 tools/bankvars.py --write    # 重写账本骨架（新增项一律 pending；不删已有项）
"""
from __future__ import annotations

import json
import re
import sys
from hashlib import sha256
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SRC = ROOT / "refs" / "m1disasm" / "SRC"
LEDGER = ROOT / "data" / "status" / "bankvars.json"
ROM = ROOT / "rom" / "Metroid (USA).nes"

BANK_FILES = {
    0: "prg1_brinstar.asm",
    1: "prg2_norfair.asm",
    2: "prg4_kraid.asm",   # 注意：goal 区序 2=Kraid（bank4）
    3: "prg3_tourian.asm", # 区序 3=Tourian（bank3）
    4: "prg5_ridley.asm",
}
# 区序 → bank 号（BankTable 07:CA30 = 02 03 05 04 06，减一即内层 bank 号 1..5）
AREA_BANK = {0: 1, 1: 2, 2: 4, 3: 3, 4: 5}

# 五区 EnAnimTable / EnFramePtrTable1 / EnPlacePtrTable 基址（extract.rs::ENEMY_SPRITES）
ENEMY_SPRITES = [
    (0x9D6A, 118, 0x9DE0, 0x9F0E),
    (0x9BDA, 138, 0x9C64, 0x9D78),
    (0x9C86, 113, 0x9CF7, 0x9E25),
    (0xA406, 38, 0xA42C, 0xA540),
    (0x9B85, 107, 0x9BF0, 0x9D04),
]

LABEL_RE = re.compile(r"^([A-Za-z_][A-Za-z0-9_]*):\s*;\s*(\d\d):([0-9A-Fa-f]{4})\s*$")

# 比较窗口；区域头/敌表区按 16 B 窗口，anim/fptr/place 段按表长（受窗口上限约束）
WINDOW = 16


def regions() -> list[tuple[int, int, str]]:
    out = [(0x95C3, 0x95E6, "area-header"), (0x960B, 0x97D5, "enemy-tables")]
    for (anim, anim_len, fptr, place) in ENEMY_SPRITES:
        out.append((anim, anim + min(anim_len, 0x80), "en-anim"))
        out.append((fptr, fptr + WINDOW, "en-fptr"))
        out.append((place, place + WINDOW, "en-place"))
    return out


def in_region(addr: int) -> str | None:
    for lo, hi, name in regions():
        if lo <= addr < hi:
            return name
    return None


def load_rom() -> bytes:
    raw = ROM.read_bytes()
    return raw[16:]  # 去 iNES header：prg 128 KB


def rom_off(bank: int, addr: int) -> int:
    return (bank - 1) * 0x4000 + (addr - 0x8000)


def label_addrs() -> dict[int, dict[int, str]]:
    """区序 → {addr: 首个标签名}（同地址多标签取第一个，仅作人读名）。"""
    out: dict[int, dict[int, str]] = {}
    for area, fname in BANK_FILES.items():
        p = SRC / fname
        if not p.is_file():
            raise SystemExit(f"缺 asm 源：{p.relative_to(ROOT)}")
        table: dict[int, str] = {}
        for line in p.read_text(encoding="utf-8", errors="replace").splitlines():
            m = LABEL_RE.match(line.strip())
            if not m:
                continue
            addr = int(m.group(3), 16)
            if in_region(addr) is None:
                continue
            table.setdefault(addr, m.group(1))
        out[area] = table
    return out


def discover(prg: bytes) -> list[dict]:
    """返回按区变量清单（地址在 ≥2 区出现且字节不全同）。"""
    labels = label_addrs()
    all_addrs: set[int] = set()
    for t in labels.values():
        all_addrs.update(t)
    found: list[dict] = []
    for addr in sorted(all_addrs):
        areas = sorted(a for a, t in labels.items() if addr in t)
        if len(areas) < 2:
            continue
        region = in_region(addr)
        digest: dict[str, str] = {}
        for a in areas:
            bank = AREA_BANK[a]
            off = rom_off(bank, addr)
            win = prg[off:off + WINDOW]
            digest[str(a)] = sha256(win).hexdigest()[:16]
        if len(set(digest.values())) == 1:
            continue  # 五区同字节 ⇒ 共享数据，不是按区变量
        name = labels[areas[0]][addr]
        found.append({
            "name": name,
            "region": region,
            "addr": f"{addr:04X}",
            "areas": areas,
            "sha16": digest,
        })
    return found


def load_ledger() -> dict:
    if not LEDGER.is_file():
        return {"schema": "bankvars-v1", "vars": []}
    return json.loads(LEDGER.read_text(encoding="utf-8"))


def main(argv: list[str]) -> int:
    prg = load_rom()
    found = discover(prg)
    ledger = load_ledger()
    known = {(v["addr"], tuple(v["areas"])): v for v in ledger.get("vars", [])}

    if "--list" in argv:
        print(f"按区变量（ROM 字节不同）：{len(found)} 项")
        for v in found:
            st = known.get((v["addr"], tuple(v["areas"])), {}).get("status", "（未登记）")
            print(f"  {v['addr']} {v['name']:<28} {v['region']:<13} areas={v['areas']} {st}")
        return 0

    if "--write" in argv:
        merged = []
        for v in found:
            key = (v["addr"], tuple(v["areas"]))
            old = known.get(key, {})
            merged.append({
                **v,
                "status": old.get("status", "pending"),
                "accessor": old.get("accessor"),
                "note": old.get("note", ""),
            })
        # 账本里已不复现的项保留并标 stale（可能是 asm 标签改名/地址移动——要人看）
        keys = {(v["addr"], tuple(v["areas"])) for v in found}
        for k, old in known.items():
            if k not in keys:
                merged.append({**old, "status": old.get("status", "pending"), "stale": True})
        LEDGER.parent.mkdir(parents=True, exist_ok=True)
        LEDGER.write_text(json.dumps({"schema": "bankvars-v1", "vars": merged}, ensure_ascii=False, indent=2) + "\n",
                          encoding="utf-8")
        print(f"写入 {LEDGER.relative_to(ROOT)}：{len(merged)} 项（新增一律 pending）")
        return 0

    # 校验模式
    problems: list[str] = []
    for v in found:
        key = (v["addr"], tuple(v["areas"]))
        old = known.get(key)
        if old is None:
            problems.append(f"新增按区变量未登记：{v['addr']} {v['name']}（areas={v['areas']}，region={v['region']}）")
            continue
        if old.get("sha16") and old["sha16"] != v["sha16"]:
            problems.append(f"ROM 字节与账本不符：{v['addr']} {v['name']}（账本 {old['sha16']} vs 现值 {v['sha16']}）")
        if old.get("status") not in ("covered", "pending"):
            problems.append(f"账本状态非法：{v['addr']} {v['name']} status={old.get('status')!r}")
        if old.get("status") == "covered" and not old.get("accessor"):
            problems.append(f"covered 项必须写明 accessor：{v['addr']} {v['name']}")
    pending = [v for v in ledger.get("vars", []) if v.get("status") == "pending"]
    covered = [v for v in ledger.get("vars", []) if v.get("status") == "covered"]
    if problems:
        print(f"[FAIL] bankvars：{len(problems)} 个问题")
        for p in problems:
            print("  - " + p)
        print("  账本更新：python3 tools/bankvars.py --write（新增项一律 pending，只减不增）")
        return 1
    print(f"[PASS] bankvars：发现 {len(found)} 项按区变量；covered {len(covered)} / pending {len(pending)}（pending 只减不增）")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
