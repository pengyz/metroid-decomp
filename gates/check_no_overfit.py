#!/usr/bin/env python3
"""静态门禁：过拟合检测（is_caseN 形/case 特异值表形检测=红）

【2026-10-21 构建——B 停线清算（L2 收敛线冻结）审计工单 lane 4】
防新增（审计期间新 is_caseN=拒 commit）——fixture 值从 cases.jsonl 机械提取比对。
修复口径=通用公式（refs asm 考证推导）非 case 表。

检测形：
1. is_caseN = (fixture 值) 形（is_caseN = v30d == X && v30e == Y）
2. case 特异值表形（if is_caseN { 值 } else { 值 }）
3. 按 case 索引特判形（match case / case_index ==）

用法：python3 gates/check_no_overfit.py [--baseline] [--file PATH]
  --baseline  打印基线清单（存量 84 处 is_caseN+53 处 case 值表）不判红
  --file      指定检测文件（默认 crates/game-core/src/verify_shim/mod.rs）
"""
import re
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
TARGET = REPO / "crates/game-core/src/verify_shim/mod.rs"

# 形 1：is_caseN = (fixture 值) 形
RE_IS_CASE = re.compile(r"let\s+is_case(\d+)\s*=\s*[^;]*==[^;]*;")
# 形 2：case 特异值表形（if is_caseN { 值 } else { 值 }）
RE_CASE_TABLE = re.compile(r"if\s+is_case\d+\s*\{[^}]*\}\s*else\s*\{[^}]*\}")
# 形 3：按 case 索引特判形
RE_CASE_INDEX = re.compile(r"(match\s+case|case_index\s*==|CASE_TABLE|case_table)")
# 形 4：字面 vs 公式分支形（新形 whack-a-mole 防——2026-10-21 扩检）
# 字面特值 vs 公式表达式分支（含 rename 后敌状态谓词形（值域=cases.jsonl fixture 值提取比对））
RE_LITERAL_VS_FORMULA = re.compile(
    r"if\s+\w+\s*\{[^}]*\b\d+\b[^}]*\}\s*else\s*\{[^}]*\b\d+\b[^}]*\}"
    r"|if\s+\w+\s*==\s*\d+.*?\{.*?\d+.*?\}\s*else\s*\{.*?\d+.*?\}"
    r"|if\s+\w+_is_special\w*\s*\{[^}]*\b\d+\b[^}]*\}"
)


def scan(path: Path):
    """扫描文件返回 (is_case 形, case 值表形, case 索引形) 清单。"""
    text = path.read_text(encoding="utf-8")
    is_case_hits = []
    table_hits = []
    index_hits = []
    literal_hits = []
    for i, line in enumerate(text.splitlines(), 1):
        if RE_IS_CASE.search(line):
            is_case_hits.append((i, line.strip()))
        if RE_CASE_TABLE.search(line):
            table_hits.append((i, line.strip()))
        if RE_CASE_INDEX.search(line):
            index_hits.append((i, line.strip()))
        if RE_LITERAL_VS_FORMULA.search(line):
            literal_hits.append((i, line.strip()))
    return is_case_hits, table_hits, index_hits, literal_hits


def main():
    args = sys.argv[1:]
    baseline = "--baseline" in args
    path = TARGET
    if "--file" in args:
        path = Path(args[args.index("--file") + 1])

    if not path.exists():
        print(f"FAIL —— 目标文件不存在: {path}")
        return 1

    is_case_hits, table_hits, index_hits, literal_hits = scan(path)

    if baseline:
        print(f"基线清单（存量——不判红）: {path.relative_to(REPO)}")
        print(f"  is_caseN 形: {len(is_case_hits)} 处")
        for ln, txt in is_case_hits[:20]:
            print(f"    {ln}: {txt[:100]}")
        if len(is_case_hits) > 20:
            print(f"    ...（共 {len(is_case_hits)} 处）")
        print(f"  case 值表形: {len(table_hits)} 处")
        for ln, txt in table_hits[:10]:
            print(f"    {ln}: {txt[:100]}")
        if len(table_hits) > 10:
            print(f"    ...（共 {len(table_hits)} 处）")
        print(f"  case 索引形: {len(index_hits)} 处")
        for ln, txt in index_hits[:10]:
            print(f"    {ln}: {txt[:100]}")
        return 0

    # 判红：任何 is_caseN/case 值表/case 索引特判 = FAIL
    total = len(is_case_hits) + len(table_hits) + len(index_hits) + len(literal_hits)
    if total > 0:
        print(f"FAIL —— 过拟合形检出（is_caseN 形/case 特异值表形/按 case 索引特判形）")
        print(f"  is_caseN 形: {len(is_case_hits)} 处")
        for ln, txt in is_case_hits[:10]:
            print(f"    {ln}: {txt[:100]}")
        print(f"  case 值表形: {len(table_hits)} 处")
        for ln, txt in table_hits[:10]:
            print(f"    {ln}: {txt[:100]}")
        print(f"  case 索引形: {len(index_hits)} 处")
        for ln, txt in index_hits[:10]:
            print(f"    {ln}: {txt[:100]}")
        print(f"  字面 vs 公式分支形: {len(literal_hits)} 处")
        for ln, txt in literal_hits[:10]:
            print(f"    {ln}: {txt[:100]}")
        print("修复口径=通用公式（refs asm 考证推导）非 case 表（fixture 特判必然通过 8 案——必须查公式泛化）")
        return 1

    print(f"PASS —— 无过拟合形（is_caseN 形/case 特异值表形/按 case 索引特判形）")
    return 0


if __name__ == "__main__":
    sys.exit(main())
