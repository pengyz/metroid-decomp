---
type: gotcha
title: Mesen takeScreenshot/getPixel 与内存 dump 无固定相位关系（lag 0-4 帧浮动）
date: 2026-11-12
status: confirmed
---

# Mesen 呈现管线滞后——截图/取像素都不与内存同刻

**症状**：像素对拍中 emu PNG 与同帧 PPU/RAM dump 不自洽（如 OAM 已全 `f0` 清空、PNG 仍画着光标精灵），产生「两侧 dump 逐字节全等但 PNG 差几十 px」的**假移植错**。

**实证**（metroid-decomp case-password-justin f694-f705/f1094-f1105 窗，详见
docs/findings/oracle-present-pipeline-lag.md 六步证据链）：

| 取面法 | 转场 lag | 结论 |
|---|---|---|
| `emu.takeScreenshot` 双读稳定 | 0-3 帧浮动 | 双读稳定策略非根因（单读同差） |
| `emu.takeScreenshot` 单读 | 同差 | 固定滞后假说证伪（延迟一缝更差） |
| `emu.getPixel(x,y)` | 4/1 帧浮动 | 与 takeScreenshot 同一呈现面 |

**结论**：Mesen2 呈现管线（视频输出缓冲）整体落后 emulation 状态 0-4 帧，浮动无规律；
Lua API 面不存在与内存 dump 同刻的取面法。`emu.read(addr, memType)`（dumpPpu 路径）
读的是当时内存，**与 PNG 语义不同源**。

**处置**（本项目）：
- 像素差若「两侧 dump 全等但 PNG 有差」⇒ 先查呈现滞后（内容组+lag 分布），勿开移植卡。
- 本项目裁 B：残差挂账（findings 登记），不容忍/不锚定/不白名单。
- 重开条件：找到内存同刻取面法（savestate+离屏渲染候选）或 Mesen 升级后重验。

**关联坑**：`gotcha-frame-compare-sampling-staleness.md`（dump 相位）、
`gotcha-mesen-savestate-callback-constraint.md`（回调约束）。
