#!/usr/bin/env python3
"""裸字节读写面的**文件级**棘轮（`raw-surface`）。

定位：`crates/game-core/src/memmap/raw.rs` 里的 `read_ram/write_ram/read_wram/write_wram`
是 typed 域迁完之前的过渡面，也是将来要删除的入口点。本工具不要求任何人**现在**去改写
调用点（不硬推），只做两件事：

1. **不许长新面**：出现一个账本里没有的文件调用裸面 ⇒ FAIL（新代码应该直接用
   `mem.samus()/enemy(slot)/camera()/room()/ens_face(x)` 等语义视图）。
2. **总量不许涨**：全仓计数比账本高 ⇒ FAIL。

调用点变少不会 FAIL——迁完一批跑 `--write` 把账本往下压，即「只减不增」。

`--list` 打印按文件的明细（含测试面），用于决定下一批迁哪个文件。
"""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SRC = ROOT / "crates" / "game-core" / "src"
LEDGER = ROOT / "data" / "status" / "raw_surface.json"

CALL_RE = re.compile(r"\b(?:read|write)_(?:ram|wram)\s*\(")

# 边界/兼容面：这些文件本来就在裸字节层工作，不入账（理由写在账本 exempt 里）。
EXEMPT = {
    "memmap/raw.rs": "裸面本体（`read_ram/write_ram/read_wram/write_wram` 的唯一实现）",
    "memmap/storage.rs": "存储层（NesStorage 字节数组的持有者）",
    "memmap/observe/frame_face.rs": "观察面（把状态导出成 NES 字节，供对拍）",
    "frame/memory.rs": "兼容辖（`restore_mss` 回灌 / `write_io` 寄存器面 / 快照投影）",
    "state.rs": "`Manager` 本体（typed 字段 + 域句柄的持有者）",
    "bin/rust_screenshot.rs": "二进制边界（`--state-out` / ram.hex dump 的产出端）",
    "tests.rs": "测试面",
}


def is_test_surface(rel: str) -> bool:
    return rel.endswith("tests.rs") or rel.startswith("tests/") or "/tests/" in rel


def scan() -> tuple[dict[str, int], dict[str, int]]:
    """返回 (生产面 {文件: 处数}, 测试面 {文件: 处数})。"""
    prod: dict[str, int] = {}
    test: dict[str, int] = {}
    for f in sorted(SRC.rglob("*.rs")):
        rel = str(f.relative_to(SRC)).replace("\\", "/")
        if rel in EXEMPT:
            continue
        txt = f.read_text(encoding="utf-8", errors="replace")
        # 只数生产面：截掉文件内 `#[cfg(test)] mod` 之后的部分。
        cut = txt.find("#[cfg(test)]\nmod ")
        body = txt[:cut] if cut > 0 else txt
        n = len(CALL_RE.findall(body))
        if n:
            (test if is_test_surface(rel) else prod)[rel] = n
        tail = len(CALL_RE.findall(txt)) - n
        if tail:
            test[rel] = test.get(rel, 0) + tail
    return prod, test


def load_ledger() -> dict:
    if LEDGER.is_file():
        return json.loads(LEDGER.read_text(encoding="utf-8"))
    return {"schema": "raw-surface-v1", "files": {}, "exempt": EXEMPT}


def main(argv: list[str]) -> int:
    prod, test = scan()
    total_prod = sum(prod.values())
    total_test = sum(test.values())

    if "--list" in argv:
        for rel, n in sorted(prod.items(), key=lambda kv: -kv[1]):
            print(f"{n:5d}  {rel}")
        print(f"--- 生产面合计 {total_prod} 处 / {len(prod)} 文件；测试面 {total_test} 处")
        return 0

    if "--write" in argv:
        files = {rel: n for rel, n in sorted(prod.items())}
        LEDGER.write_text(
            json.dumps(
                {
                    "schema": "raw-surface-v1",
                    "note": (
                        "裸字节读写的**文件级**棘轮（只减不增）。语义视图已覆盖 "
                        "Samus/敌槽/爆炸池/对象池/相机/房间；剩余族（asm Temp、OAM 影子、"
                        "声音旗标、PageIndex 等伪寄存器）尚未视图化。"
                    ),
                    "total": total_prod,
                    "files": files,
                    "exempt": EXEMPT,
                },
                ensure_ascii=False,
                indent=2,
            )
            + "\n",
            encoding="utf-8",
        )
        print(f"[write] {LEDGER.relative_to(ROOT)}: {total_prod} 处 / {len(files)} 文件")
        return 0

    ledger = load_ledger()
    base_files: dict[str, int] = ledger.get("files", {})
    base_total = int(ledger.get("total", 0))
    problems: list[str] = []

    for rel, n in sorted(prod.items()):
        if rel not in base_files:
            problems.append(f"新文件接入裸面：{rel}（{n} 处）——请改用 mem.samus()/enemy(slot)/camera()/room() 等视图")
        elif n > base_files[rel]:
            problems.append(f"{rel}: {base_files[rel]} → {n} 处（文件内上调）")

    if total_prod > base_total:
        problems.append(f"总量上调：{base_total} → {total_prod}")

    if problems:
        print("[FAIL] raw-surface：")
        for p in problems:
            print(f"  - {p}")
        return 1

    delta = base_total - total_prod
    tail = f"；可下调账本 {delta} 处（--write）" if delta > 0 else ""
    print(
        f"[PASS] raw-surface：裸字节面 {total_prod}/{base_total} 处 / "
        f"{len(prod)} 文件（新文件不许接入、总量只减不增）{tail}"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
