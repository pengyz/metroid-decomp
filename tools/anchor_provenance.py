#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""tools/anchor_provenance.py —— golden 溯源锚工具（D1 落实——仪器与被测物分离）。

职责（production-discipline.md D1（golden 溯源锚定））：
1. schema 扩：l2_cases.jsonl 每案补 capture_id/trace_digest 字段（溯源锚（pending 初值））；
2. 注入径：从捕获 run 清单（capture manifest（case_id→capture_id/trace_digest 映射））
   注入溯源锚（P1 仪器与被测物分离——**只加溯源字段，绝不改期望值**）；
3. 审计径：溯源锚覆盖审计（无锚案清单（pending 数/覆盖率））。

用法：
    python3 tools/anchor_provenance.py schema          # schema 扩（补字段（幂等））
    python3 tools/anchor_provenance.py inject <manifest.json>   # 注入（manifest: [{case_id, capture_id, trace_digest}]）
    python3 tools/anchor_provenance.py audit            # 覆盖审计（pending 数/覆盖率/无锚清单）

红线（P1）：inject 只写 capture_id/trace_digest 字段——期望值（out.writes 等）一个字节不改。
"""

import json
import sys
from pathlib import Path

CASES = Path("oracle/subroutines/l2_cases.jsonl")


def load_cases():
    cases = []
    with open(CASES) as f:
        for line in f:
            line = line.strip()
            if line:
                cases.append(json.loads(line))
    return cases


def save_cases(cases):
    with open(CASES, "w") as f:
        for c in cases:
            f.write(json.dumps(c, ensure_ascii=False, sort_keys=True) + "\n")


def cmd_schema():
    cases = load_cases()
    added = 0
    for c in cases:
        if "capture_id" not in c:
            c["capture_id"] = None
            added += 1
        if "trace_digest" not in c:
            c["trace_digest"] = None
            added += 1
    save_cases(cases)
    print(f"schema 扩实录：{len(cases)} 案（字段补 {added} 个（capture_id/trace_digest（pending 初值）））")


def cmd_inject(manifest_path):
    manifest = json.loads(Path(manifest_path).read_text())
    by_id = {m["case_id"]: m for m in manifest}
    cases = load_cases()
    injected = 0
    for c in cases:
        m = by_id.get(c.get("id"))
        if m:
            # P1 红线：只写溯源字段
            c["capture_id"] = m["capture_id"]
            c["trace_digest"] = m["trace_digest"]
            injected += 1
    save_cases(cases)
    print(f"注入实录：{injected} 案 capture_id/trace_digest 注入（manifest: {manifest_path}）——期望值零改（P1 红线）")


def cmd_audit():
    cases = load_cases()
    total = len(cases)
    anchored = sum(1 for c in cases if c.get("capture_id") and c.get("trace_digest"))
    pending = total - anchored
    print(f"溯源锚覆盖审计（A 级实录）：")
    print(f"  总案数={total}｜已锚={anchored}｜pending={pending}｜覆盖率={anchored}/{total}={anchored/total:.4f}")
    if pending:
        unanchored = [c.get("id", "?") for c in cases if not (c.get("capture_id") and c.get("trace_digest"))]
        print(f"  无锚案样本（首 10）：{unanchored[:10]}")


def main():
    if len(sys.argv) < 2:
        print(__doc__)
        sys.exit(1)
    cmd = sys.argv[1]
    if cmd == "schema":
        cmd_schema()
    elif cmd == "inject":
        if len(sys.argv) < 3:
            print("用法：inject <manifest.json>")
            sys.exit(1)
        cmd_inject(sys.argv[2])
    elif cmd == "audit":
        cmd_audit()
    else:
        print(f"未知命令：{cmd}")
        sys.exit(1)


if __name__ == "__main__":
    main()
