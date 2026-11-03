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


# 采集截取缝契约（2026-11-03 fade-blink-capture-race 定谳）：emu.takeScreenshot 在
# endFrame 帧界回调直读=渲染面翻转竞态窗（vblank 起点（ppu scanline 241 c25-33 实测锚））
# ——淡入窗 FC&1 奇偶翻格根因。截图兑现点必须移至 spin-exit 缝（$C0D3（prg7:218
# NMI 后/帧工作前——竞态窗外））：endFrame 只登记请求（requestShot），缝兑现（fulfillShot）。
SEAM_REQUIRED = [
    (r"local function requestShot\s*\(", "截图请求登记面缺失（endFrame 只登记不读渲染面）"),
    (r"local function fulfillShot\s*\(", "截图缝兑现面缺失（spin-exit 缝读渲染面）"),
    (r"SHOT-SEAM", "缝兑现留痕标记缺失（兑现须显式日志 SHOT-SEAM）"),
]
SEAM_FORBIDDEN = [
    (r"local shot = saveShot\s*\(", "legacy endFrame 直读截图——帧界竞态窗（翻格根）拒跑"),
]


def lint_text(text: str, label: str) -> list[str]:
    """禁用面清单（通用 lua——不含采集截取缝契约（那是 harness 专属面，见 lint_seam）。"""
    hits = []
    for pat, why in BANNED:
        for m in re.finditer(pat, text):
            line = text[:m.start()].count("\n") + 1
            hits.append(f"{label}:{line}: 禁用面 {pat} ——{why}")
    return hits


def lint_seam(text: str, label: str) -> list[str]:
    """采集截取缝契约（scenario_capture.lua 专属——fade-blink-capture-race 修径 C 机械防）。"""
    hits = []
    for pat, why in SEAM_REQUIRED:
        if not re.search(pat, text):
            hits.append(f"{label}: 缺采集截取缝契约 {pat} ——{why}")
    for pat, why in SEAM_FORBIDDEN:
        for m in re.finditer(pat, text):
            line = text[:m.start()].count("\n") + 1
            hits.append(f"{label}:{line}: 采集截取缝契约 {pat} ——{why}")
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
