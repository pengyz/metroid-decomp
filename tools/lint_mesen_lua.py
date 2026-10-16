#!/usr/bin/env python3
"""task-38 ③：Mesen Lua 禁用面机械防（createSavestate 无参=挂死实录）。

依据：tools/probe_savestate_load.lua v2 实录——emu.createSavestate() 无参调用挂死
（timeout 124，疑原生对话框）。机械禁用清单=启动前 fail-loud 拒跑。

用法：
  python3 tools/lint_mesen_lua.py <lua 文件...>     # 违反 exit 1
  python3 tools/lint_mesen_lua.py --text '<lua>'    # 内联文本扫描
"""
import re
import sys
from pathlib import Path

# 禁用清单（违反=挂死/降级面）
BANNED = [
    (r"emu\.createSavestate\s*\(\s*\)",
     "createSavestate 无参=挂死面（probe v2 超时 124 实录）——须 exec 回调内调用"),
    (r"emu\.loadSavestate\s*\(\s*\)",
     "loadSavestate 无参=同族挂死面——须带 blob 参数"),
]


def lint_text(text: str, label: str) -> list[str]:
    hits = []
    for pat, why in BANNED:
        for m in re.finditer(pat, text):
            line = text[:m.start()].count("\n") + 1
            hits.append(f"{label}:{line}: 禁用面 {pat} ——{why}")
    return hits


def main() -> int:
    if len(sys.argv) >= 3 and sys.argv[1] == "--text":
        hits = lint_text(sys.argv[2], "<text>")
    elif len(sys.argv) >= 2:
        hits = []
        for f in sys.argv[1:]:
            hits += lint_text(Path(f).read_text(encoding="utf-8", errors="replace"), f)
    else:
        print(__doc__)
        return 2
    for h in hits:
        print(f"[task-38 lint] FAIL {h}")
    if hits:
        return 1
    print(f"[task-38 lint] PASS（{len(sys.argv) - 1} 目标，禁用面 {len(BANNED)} 条）")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
