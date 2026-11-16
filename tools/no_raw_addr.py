#!/usr/bin/env python3
"""no-raw-addr：域代码里裸地址字面量的棘轮账本（只减不增）。

## 为什么需要这个门

typed 域（`crates/game-core/src/memmap/views/`）已经把 Samus 的全部 22 字节
（`$0300-$0315`）与敌槽两基址（`$0400` / `$6AF4`）包成了语义对象，但**调用点**仍然
大量直接写 `mem.write_ram(0x0300, …)` / `mem.read_ram(0x6AF4 + x + 0)`。

后果不是"报错"，而是**类型安全形同虚设**：写的人看不见 `SamusStatus`，编译器也拦不住
把 `status` 当成任意 `u8` 用。更糟的是——如果哪天某个字段又变成"双表示"，只有 typed
一侧被改到，裸写那侧会被静默丢弃（这正是 `c875f3b9` 抓到的 dump 漏字段同族故障）。

所以门的定位是**止住增量**，不是逼立刻还债：
  * 现状一次入账（每文件一条上限）；
  * 之后任何文件**不得超过**账本数字，总量不得超过账本总量；
  * 迁移（把裸地址换成 `mem.samus()` / `mem.enemy(slot)`）之后跑 `--write` 把账本往下压。

## 豁免面（adapter / boundary，必须写理由）

裸地址在这些地方是**职责本身的形态**，不是债：
  * `memmap/views/**`、`memmap/storage.rs`、`memmap/mod.rs`：typed 层自己的地址映射；
  * `ram_map.rs`：符号→地址的唯一定义处；
  * `frame/memory.rs`：`read_ram`/`write_ram` 兼容辖，把地址转投 typed（迁移完成后收口）；
  * `bin/**`：独立二进制工具的 dump 面。

## 第二面：具名常量（2026-11-17 增）

只数字面量会漏掉同一件事的另一半写法：`mem.write_ram(ram_map::SAMUS_STATUS, …)`。
这样写**看起来**已经是符号，但它把 typed 字段当成任意 `u8` 用，编译器照样拦不住
（复核实测 165 处：写 87 / 读 78）。所以本门再记一本 `no_named_addr.json`：
凡是 `(read|write)_(ram|wram)(… ram_map::X …)` 且 `X` 的**值**落在受监控地址族内，
就计入；同样只减不增。名字→值从 `ram_map.rs` 的 `pub const` 解析（不另设名单）。

## 用法

    python3 tools/no_raw_addr.py            # 校验（CI / 门禁）
    python3 tools/no_raw_addr.py --list     # 打印每文件计数
    python3 tools/no_raw_addr.py --write    # 重写账本（只允许下调；上调需 --allow-raise）
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SRC = ROOT / "crates/game-core/src"
LEDGER = ROOT / "data/status/no_raw_addr.json"
LEDGER_NAMED = ROOT / "data/status/no_named_addr.json"
RAM_MAP = SRC / "ram_map.rs"

# 受监控的地址族：Samus 主体 22 字节 + 敌槽两基址 + 爆炸池两基址。
# 加新地址族前先确认它已有 typed 视图（否则门会逼人迁移一个还不存在的对象）。
ADDR_RE = re.compile(
    r"\b0x(?:0?3(?:0[0-9a-fA-F]|1[0-6])|0400|6AF4|04C0|6BB4)\b"
)

EXEMPT = {
    # 2026-11-17：typed 类从 memmap/views/ 迁入实体目录（聚合根/句柄不是内存映射的一部分）
    "samus/state.rs": "typed 层自己的实现面",
    "enemy/state.rs": "typed 层自己的实现面",
    "title/state.rs": "typed 层自己的实现面",
    "world/object.rs": "对象池本体：池基址/步长的唯一定义处（等价的 enemy/state.rs）",
    "enemy/explosion.rs": "爆炸池本体：两半存储（$04C0 8 B / $6BB4 16 B）基址与步长的唯一定义处",
    "memmap/storage.rs": "存储层（字节面本体）",
    "memmap/mod.rs": "存储层导出面",
    "ram_map.rs": "符号→地址的唯一定义处",
    "frame/memory.rs": "read_ram/write_ram 兼容辖（迁移完成后收口）",
}


CONST_RE = re.compile(r"pub const ([A-Z][A-Z0-9_]*)\s*:\s*u16\s*=\s*(0x[0-9A-Fa-f]+|[0-9]+)\s*;")
NAMED_USE_RE = re.compile(r"\b(?:read|write)_(?:ram|wram)\(\s*(?:crate::)?ram_map::([A-Z][A-Z0-9_]*)")


def monitored_names() -> set[str]:
    """`ram_map.rs` 里值落在受监控地址族的常量名（名字→值唯一源在代码，不另设名单）。"""
    text = RAM_MAP.read_text(encoding="utf-8", errors="replace")
    out: set[str] = set()
    for name, lit in CONST_RE.findall(text):
        v = int(lit, 16) if lit.lower().startswith("0x") else int(lit)
        base = v & 0x7FF  # 镜像归一后判：$0B00 与 $0300 是同一格
        if v in (0x0400, 0x06AF4, 0x04C0, 0x06BB4) or 0x300 <= base <= 0x316:
            out.add(name)
    return out


def rel(p: Path) -> str:
    return p.relative_to(SRC).as_posix()


def scan() -> dict[str, int]:
    counts: dict[str, int] = {}
    for p in sorted(SRC.rglob("*.rs")):
        r = rel(p)
        if r in EXEMPT or r.startswith("bin/"):
            continue
        n = len(ADDR_RE.findall(p.read_text(encoding="utf-8", errors="replace")))
        if n:
            counts[r] = n
    return counts


def scan_named() -> dict[str, int]:
    names = monitored_names()
    counts: dict[str, int] = {}
    for p in sorted(SRC.rglob("*.rs")):
        r = rel(p)
        if r in EXEMPT or r.startswith("bin/"):
            continue
        hits = NAMED_USE_RE.findall(p.read_text(encoding="utf-8", errors="replace"))
        n = sum(1 for h in hits if h in names)
        if n:
            counts[r] = n
    return counts


def load_ledger() -> dict:
    return json.loads(LEDGER.read_text(encoding="utf-8"))


def write_ledger(counts: dict[str, int], allow_raise: bool) -> int:
    total = sum(counts.values())
    if LEDGER.is_file() and not allow_raise:
        old = load_ledger()
        if total > old["total"]:
            print(
                f"拒绝上调：账本 {old['total']} → 现状 {total}。"
                f"裸地址只减不增；确需上调请走 --allow-raise 并写明理由。",
                file=sys.stderr,
            )
            return 2
        raised = {
            f: (counts[f], old["files"].get(f, 0))
            for f in counts
            if counts[f] > old["files"].get(f, 0)
        }
        if raised:
            print("拒绝上调（单文件）：", file=sys.stderr)
            for f, (now, was) in sorted(raised.items()):
                print(f"  - {f}: {was} → {now}", file=sys.stderr)
            return 2
    doc = {
        "schema": "no-raw-addr-v1",
        "note": (
            "域代码里裸地址字面量的棘轮上限（只减不增）。"
            "受监控地址族：Samus $0300-$0316、敌槽基址 $0400/$6AF4、爆炸池 $04C0/$6BB4。"
            "迁移到 typed 视图后跑 tools/no_raw_addr.py --write 下调。"
        ),
        "monitored": ["$0300-$0316", "$0400", "$6AF4", "$04C0", "$6BB4"],
        "exempt": EXEMPT,
        "total": total,
        "files": dict(sorted(counts.items())),
    }
    LEDGER.parent.mkdir(parents=True, exist_ok=True)
    LEDGER.write_text(json.dumps(doc, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(f"账本已写：{LEDGER.relative_to(ROOT)}  总 {total} 处 / {len(counts)} 文件")
    return 0


def _write_one(ledger: Path, counts: dict[str, int], allow_raise: bool, schema: str,
               note: str, monitored: list[str]) -> int:
    total = sum(counts.values())
    if ledger.is_file() and not allow_raise:
        old = json.loads(ledger.read_text(encoding="utf-8"))
        if total > old["total"]:
            print(f"拒绝上调（{ledger.name}）：{old['total']} → {total}。只减不增；确需上调走 --allow-raise 并写理由。", file=sys.stderr)
            return 2
        raised = {f: (counts[f], old["files"].get(f, 0)) for f in counts if counts[f] > old["files"].get(f, 0)}
        if raised:
            print(f"拒绝上调（{ledger.name}，单文件）：", file=sys.stderr)
            for f, (now, was) in sorted(raised.items()):
                print(f"  - {f}: {was} → {now}", file=sys.stderr)
            return 2
    doc = {"schema": schema, "note": note, "monitored": monitored, "exempt": EXEMPT,
           "total": total, "files": dict(sorted(counts.items()))}
    ledger.parent.mkdir(parents=True, exist_ok=True)
    ledger.write_text(json.dumps(doc, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(f"账本已写：{ledger.relative_to(ROOT)}  总 {total} 处 / {len(counts)} 文件")
    return 0


def _check_one(ledger: Path, counts: dict[str, int], what: str) -> list[str]:
    if not ledger.is_file():
        return [f"缺 {ledger.relative_to(ROOT)}（先跑 --write 入账）"]
    old = json.loads(ledger.read_text(encoding="utf-8"))
    problems: list[str] = []
    for f, n in sorted(counts.items()):
        was = old["files"].get(f)
        if was is None:
            problems.append(f"[{what}] 新文件出现 {f}: {n} 处（未登记；请改用 typed 视图）")
        elif n > was:
            problems.append(f"[{what}] {f}: {was} → {n}（只减不增）")
    total_now = sum(counts.values())
    if total_now > old["total"]:
        problems.append(f"[{what}] 总量 {old['total']} → {total_now}（只减不增）")
    return problems


def check() -> int:
    raw_now = scan()
    named_now = scan_named()
    problems = _check_one(LEDGER, raw_now, "裸地址") + _check_one(LEDGER_NAMED, named_now, "具名常量")
    if problems:
        print("typed 面被绕过（视图已存在：改用 mem.samus()/mem.enemy(slot)）：", file=sys.stderr)
        for p in problems:
            print(f"  - {p}", file=sys.stderr)
        return 1
    raw_old = json.loads(LEDGER.read_text(encoding="utf-8"))
    named_old = json.loads(LEDGER_NAMED.read_text(encoding="utf-8"))
    lowered = (raw_old["total"] - sum(raw_now.values())) + (named_old["total"] - sum(named_now.values()))
    tail = f"；可下调账本 {lowered} 处（--write）" if lowered > 0 else ""
    print(
        f"typed 绕过面在账内：裸地址 {sum(raw_now.values())}/{raw_old['total']} 处 / {len(raw_now)} 文件"
        f"；具名常量 {sum(named_now.values())}/{named_old['total']} 处 / {len(named_now)} 文件"
        f"（受监控 {', '.join(raw_old['monitored'])}；豁免 {len(EXEMPT)} 个边界文件）{tail}"
    )
    return 0


def main() -> int:
    ap = argparse.ArgumentParser(description="域代码裸地址字面量棘轮门")
    ap.add_argument("--write", action="store_true", help="重写账本（默认只允许下调）")
    ap.add_argument("--allow-raise", action="store_true", help="允许上调账本（需在 commit 里写明理由）")
    ap.add_argument("--list", action="store_true", help="打印每文件计数")
    args = ap.parse_args()

    if args.list:
        counts = scan()
        for f, n in sorted(counts.items(), key=lambda kv: (-kv[1], kv[0])):
            print(f"{n:4d}  {f}")
        print(f"----\n{sum(counts.values()):4d}  共 {len(counts)} 文件")
        return 0
    if args.write:
        rc = write_ledger(scan(), args.allow_raise)
        if rc:
            return rc
        return _write_one(
            LEDGER_NAMED, scan_named(), args.allow_raise, "no-named-addr-v1",
            "域代码里通过 ram_map::X 具名常量绕过 typed 面的棘轮上限（只减不增）。"
            "X 的取值落在受监控地址族内即计入；名字→值由 ram_map.rs 的 pub const 解析。",
            ["$0300-$0316", "$0400", "$6AF4", "$04C0", "$6BB4"],
        )
    return check()


if __name__ == "__main__":
    raise SystemExit(main())
