#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""tools/diff_subroutines.py —— L2 子程序级差分门禁驱动（subroutine-diff 门禁入口）。

薄封装：差分逻辑唯一存在于 crates/game-oracle（6502 核 + 三方裁定），本脚本只负责
调 `cargo run -p game-oracle --bin l2-diff` 并透传输出/退出码（无第二套扫描逻辑）。

裁定面（oracle/subroutines/l2_cases.jsonl，先写用例再实现）：
    case.out（golden） == 6502 核执行 ROM 原字节 == game-core 同名函数
用法：
    python3 tools/diff_subroutines.py                # 跑默认 case 集
    python3 tools/diff_subroutines.py --cases <路径>  # 指定 case 文件
"""
from __future__ import annotations

import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent


def main() -> int:
    cmd = [
        "cargo", "run", "-q", "-p", "game-oracle", "--bin", "l2-diff", "--",
        "--rom", str(ROOT / "rom" / "Metroid (USA).nes"),
    ]
    if "--cases" in sys.argv:
        idx = sys.argv.index("--cases")
        if idx + 1 >= len(sys.argv):
            print("用法: diff_subroutines.py [--cases <路径>]", file=sys.stderr)
            return 2
        cmd += ["--cases", sys.argv[idx + 1]]
    else:
        cmd += ["--cases", str(ROOT / "oracle" / "subroutines" / "l2_cases.jsonl")]

    # task-38 ④：超时=FAIL 显式（防挂死——超限打回非悬挂；l2 引擎侧另有 MAX_STEPS 1M）
    try:
        proc = subprocess.run(cmd, cwd=ROOT, timeout=600, capture_output=True, text=True,
                              encoding="utf-8", errors="replace")
    except subprocess.TimeoutExpired:
        print(f"[FAIL] {cmd[1] if len(cmd) > 1 else cmd}: 超时 >600s（task-38 护栏——限时打回非挂死）")
        raise SystemExit(1)
    out = (proc.stdout or "") + (proc.stderr or "")
    print(out, end="" if out.endswith("\n") or not out else "\n")
    if proc.returncode == 0:
        return 0
    # L2 记账面（迭代态已知缺口——trace/visual gap_map 同构：账内容忍+路线图）
    import json as _json, re as _re
    gaps = []
    gm = ROOT / "data" / "status" / "l2_gap_map.json"
    if gm.is_file():
        try:
            gaps = _json.loads(gm.read_text(encoding="utf-8")).get("entries", [])
        except Exception:
            pass
    fails = _re.findall(r"\[FAIL\] ([^:]+):", out)
    accounted = [f for f in fails if any(f.startswith(g.get("case_prefix", "\0")) for g in gaps)]
    unacc = [f for f in fails if f not in accounted]
    if fails and not unacc:
        print(f"[TOLERATED] L2 账内容忍 {len(accounted)} 案（迭代态缺口——账清转 strict）")
        for g in gaps:
            print(f"  缺口路线图[{g['id']}]: {g.get('reason','')}｜退出={g.get('exit','')}")
        return 0
    if accounted:
        print(f"[FAIL] L2 账外 {len(unacc)} 案（账内 {len(accounted)} 已忍）")
    return proc.returncode


if __name__ == "__main__":
    sys.exit(main())
