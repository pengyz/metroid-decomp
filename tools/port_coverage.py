#!/usr/bin/env python3
"""port_coverage —— port-map registry 校验（phase0 交付物③，G3 清偿）。

校验面：
① rust 归宿存在性（crate::module::fn 的 fn 名在源树中存在——grep 面近似）
② 双射性（同一 rust 归宿不得被两条 asm 记录独占声明——alias 须显式注记）
③ asm 锚形（bankN:HEX——正则面）

用法：python tools/port_coverage.py [--registry data/naming/port-map.toml]
判定：退出码 0=全过；1=有违例（fail-loud，不静默）。
"""
from __future__ import annotations

import argparse
import re
import sys
import tomllib
from collections import Counter
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
ASM_RE = re.compile(r"^bank\d+:[0-9A-Fa-f]{4}$")
SRC_ROOTS = [ROOT / "crates" / "game-core" / "src", ROOT / "crates" / "game-data" / "src"]


def rust_fn_exists(rust: str) -> bool:
    """crate::module::fn 尾名在源树 fn/item 声明面存在（近似——正典判据=编译，本件
    做注册时点的轻量自检；CI 面由 rust-tests 承担编译真值）。"""
    fn = rust.rsplit("::", 1)[-1]
    pat = re.compile(rf"\b(?:fn|struct|enum|const|type)\s+{re.escape(fn)}\b")
    for base in SRC_ROOTS:
        for p in base.rglob("*.rs"):
            if pat.search(p.read_text(encoding="utf-8", errors="replace")):
                return True
    return False


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--registry", default="data/naming/port-map.toml")
    args = ap.parse_args()
    reg_path = ROOT / args.registry
    if not reg_path.is_file():
        print(f"[FAIL] registry 缺失: {args.registry}")
        return 1
    data = tomllib.loads(reg_path.read_text(encoding="utf-8"))
    entries = data.get("map", [])
    problems: list[str] = []
    rusts: Counter[str] = Counter()
    for i, e in enumerate(entries):
        asm, sym, rust = e.get("asm", ""), e.get("sym", ""), e.get("rust", "")
        if not ASM_RE.match(asm):
            problems.append(f"#{i} asm 锚形违例: {asm!r}（bankN:HEX）")
        if not sym:
            problems.append(f"#{i} sym 空")
        if not rust.startswith("game_core::") and not rust.startswith("game_data::"):
            problems.append(f"#{i} rust 归宿须 game_core/game_data 域: {rust!r}")
        elif not rust_fn_exists(rust):
            problems.append(f"#{i} rust 归宿未见声明: {rust}（asm={asm}）")
        rusts[rust] += 1
    for rust, n in rusts.items():
        if n > 1:
            problems.append(f"双射违例: {rust} 被 {n} 条 asm 记录独占（alias 须显式注记）")
    if problems:
        print(f"[FAIL] port-map registry {len(entries)} 条，违例 {len(problems)}：")
        for p in problems[:10]:
            print("  -", p)
        return 1
    print(f"[PASS] port-map registry {len(entries)} 条：锚形 ✓ 归宿存在 ✓ 双射 ✓"
          f"（scope={data.get('scope', '?')}——D7：全不全以增量补齐承诺承担）")
    return 0


if __name__ == "__main__":
    sys.exit(main())
