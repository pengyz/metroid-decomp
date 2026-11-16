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

# 受监控的地址族：Samus 主体 22 字节 + 敌槽两基址 + 爆炸池两基址。
# 加新地址族前先确认它已有 typed 视图（否则门会逼人迁移一个还不存在的对象）。
ADDR_RE = re.compile(
    r"\b0x(?:0?3(?:0[0-9a-fA-F]|1[0-5])|0400|6AF4|04C0|6BB4)\b"
)

EXEMPT = {
    "memmap/views/mod.rs": "typed 层自己的实现面",
    "memmap/views/samus.rs": "typed 层自己的实现面",
    "memmap/views/enemy.rs": "typed 层自己的实现面",
    "memmap/storage.rs": "存储层（字节面本体）",
    "memmap/mod.rs": "存储层导出面",
    "ram_map.rs": "符号→地址的唯一定义处",
    "frame/memory.rs": "read_ram/write_ram 兼容辖（迁移完成后收口）",
}


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
            "受监控地址族：Samus $0300-$0315、敌槽基址 $0400/$6AF4、爆炸池 $04C0/$6BB4。"
            "迁移到 typed 视图后跑 tools/no_raw_addr.py --write 下调。"
        ),
        "monitored": ["$0300-$0315", "$0400", "$6AF4", "$04C0", "$6BB4"],
        "exempt": EXEMPT,
        "total": total,
        "files": dict(sorted(counts.items())),
    }
    LEDGER.parent.mkdir(parents=True, exist_ok=True)
    LEDGER.write_text(json.dumps(doc, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(f"账本已写：{LEDGER.relative_to(ROOT)}  总 {total} 处 / {len(counts)} 文件")
    return 0


def check() -> int:
    if not LEDGER.is_file():
        print("缺 data/status/no_raw_addr.json（先跑 --write 入账）", file=sys.stderr)
        return 2
    old = load_ledger()
    now = scan()
    problems: list[str] = []

    for f, n in sorted(now.items()):
        was = old["files"].get(f)
        if was is None:
            problems.append(f"新文件出现裸地址 {f}: {n} 处（未登记；请改用 typed 视图）")
        elif n > was:
            problems.append(f"{f}: {was} → {n}（只减不增）")

    total_now = sum(now.values())
    if total_now > old["total"]:
        problems.append(f"总量 {old['total']} → {total_now}（只减不增）")

    if problems:
        print("裸地址字面量增加（typed 视图已存在，请改用 mem.samus()/mem.enemy(slot)）：", file=sys.stderr)
        for p in problems:
            print(f"  - {p}", file=sys.stderr)
        return 1

    lowered = old["total"] - total_now
    tail = f"；可下调账本 {lowered} 处（--write）" if lowered > 0 else ""
    print(
        f"裸地址在账内：{total_now}/{old['total']} 处 / {len(now)} 文件"
        f"（受监控 {', '.join(old['monitored'])}；豁免 {len(old['exempt'])} 个边界文件）{tail}"
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
        return write_ledger(scan(), args.allow_raise)
    return check()


if __name__ == "__main__":
    raise SystemExit(main())
