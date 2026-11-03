#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""tools/tests/test_capture_seam.py —— 采集截取缝契约 测试（TDD 先红后绿）。

背景（docs/findings/fade-blink-capture-race.md 定谳）：淡入闪现窗 FC 奇偶翻格
=emu.takeScreenshot 在 endFrame 帧界回调直读渲染面（vblank 起点翻转竞态窗
（ppu scanline 241 c25-33 实测锚））——同回调双读 4 次实录分歧（read2=FC 真值像）。

契约（C 径修）：
1. endFrame 采集块只**登记**截取请求（requestShot）——不读渲染面；
2. 截图在 **spin-exit 缝（$C0D3，prg7:218——NMI 后/帧工作前）兑现**（fulfillShot），
   缝位在竞态窗外数千周期；
3. 稳定双读（两次连续一致才落盘）+ SHOT-SEAM 显式留痕；
4. legacy endFrame 直读径（`local shot = saveShot(tag)`）机械拒跑（fail-loud）。

红态（本 commit）：harness 仍是 legacy 直读径 ⇒ test_harness_source_satisfies_seam_contract
红（契约检出 legacy 直读+缺 requestShot/fulfillShot）。绿态（下 commit）：harness 缝化后全绿。
"""
from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent.parent
sys.path.insert(0, str(ROOT / "tools"))

import lint_mesen_lua as lint  # noqa: E402

HARNESS = ROOT / "tools" / "scenario_capture.lua"

LEGACY_FORM = '''
emu.addEventCallback(function()
  local tag = capAt[frame]
  if tag then
    local shot = saveShot(tag)
  end
end, "endFrame")
'''

SEAM_FORM = '''
local function requestShot(path)
  pendingShot = { path = path }
end
local function fulfillShot()
  log("SHOT-SEAM f=" .. frame)
end
emu.addEventCallback(function()
  local tag = capAt[frame]
  if tag then
    local shot = requestShot(tag)
  end
end, "endFrame")
'''


class TestSeamLint:
    """契约机械检出面（禁用面同族——启动前 fail-loud）。"""

    def test_legacy_endframe_direct_shot_rejected(self):
        hits = lint.lint_seam(LEGACY_FORM, "legacy")
        assert any("saveShot" in h for h in hits), f"legacy 直读径应被拒: {hits}"
        assert any("requestShot" in h for h in hits), f"缺 requestShot 应被检出: {hits}"
        assert any("fulfillShot" in h for h in hits), f"缺 fulfillShot 应被检出: {hits}"

    def test_seam_form_accepted(self):
        assert lint.lint_seam(SEAM_FORM, "seam") == []


class TestHarnessSourceSeamContract:
    """harness 源面锚（红→绿）：tools/scenario_capture.lua 必须满足缝契约。"""

    def test_harness_source_satisfies_seam_contract(self):
        text = HARNESS.read_text(encoding="utf-8")
        hits = lint.lint_seam(text, "tools/scenario_capture.lua")
        assert hits == [], f"harness 未满足采集截取缝契约: {hits}"
