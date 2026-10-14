#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""tools/kb_pointer_scan.py —— knowledge 条目悬空指针扫描器（唯一口径实现）。

docs/knowledge/README.md 的悬空指针表由本脚本生成/复核，勿手写（rounds/r2 R2 教训：
brace 跨行写法 `docs/spec/{a,b,c}.md` 曾逃逸简易正则 → "无悬空"假断言）。

口径：
  - 扫描 docs/knowledge/*.md（README.md 除外）中的仓库路径引用：
    前缀 ∈ {docs, oracle, data, gates, tools, reference, crates, prompts}，
    含 `{a,b}`/`{a,b,c}` brace 展开；去 #fragment、尾标点后测本仓存在性。
  - 口径外指针（commit hash、tXX 任务号、.archive/ 快照）不判定——一律回源核对。
  - grep 关键词核验请限定**源文本**（py/md/json/lua/sh/txt）；__pycache__ 缓存不含。

用法：python3 tools/kb_pointer_scan.py [--json]
"""
from __future__ import annotations

import argparse
import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
KB = ROOT / "docs" / "knowledge"
PREFIXES = ("docs/", "oracle/", "data/", "gates/", "tools/", "reference/", "crates/", "prompts/")
RAW = re.compile(r"(?:docs|oracle|data|gates|tools|reference|crates|prompts)/[A-Za-z0-9_./-]*\{[^}]*\}[A-Za-z0-9_./-]*"
                 r"|(?:docs|oracle|data|gates|tools|reference|crates|prompts)/[A-Za-z0-9_./-]+")


def expand_braces(tok: str) -> list[str]:
    m = re.search(r"\{([^}]*)\}", tok)
    if not m:
        return [tok]
    out: list[str] = []
    for alt in m.group(1).split(","):
        out.extend(expand_braces(tok[: m.start()] + alt + tok[m.end():]))
    return out


def scan() -> list[dict]:
    rows = []
    for p in sorted(KB.glob("*.md")):
        if p.name == "README.md":
            continue
        text = p.read_text(encoding="utf-8")
        dangling: list[str] = []
        for raw in RAW.findall(text):
            for tok in expand_braces(raw):
                r = tok.split("#")[0].rstrip("/.,;:)")
                if not r or not r.startswith(PREFIXES):
                    continue
                if r in dangling:
                    continue
                if not (ROOT / r).exists():
                    dangling.append(r)
        rows.append({"entry": p.name, "dangling": dangling})
    return rows


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--json", action="store_true")
    args = ap.parse_args()
    rows = scan()
    n = sum(1 for r in rows if r["dangling"])
    if args.json:
        print(json.dumps({"entries": rows, "dangling_entries": n, "total": len(rows)},
                         ensure_ascii=False, indent=2))
    else:
        print(f"含路径级悬空指针条目: {n}/{len(rows)}")
        for r in rows:
            print(f"  {r['entry']}: {r['dangling'] if r['dangling'] else '（无悬空）'}")
        print("注：commit hash / tXX / .archive 类指针在口径外，一律回源核对。")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
