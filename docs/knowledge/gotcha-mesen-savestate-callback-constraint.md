---
type: gotcha
title: Mesen savestate 只能在 start-frame 回调中调用；getState/setState 不含 APU/cart
date: 2026-11-09
status: confirmed-unverified-locally
---

# gotcha：Mesen savestate 回调约束

**来源**：Mesen 官方仓库 issue #259（https://github.com/SourMesen/Mesen/issues/259 ，社区扫描
2026-11-09 发现，见 docs/findings/community-sources-scan-2026-11-09.md 方向 2）。

## 坑
1. **savestate 调用时机**：`savestate` 相关 API 只能在 start-frame 回调中调用——在其他回调
   （如内存回调/指令回调）里调用会失败或产生未定义状态。
2. **`getState`/`setState` 覆盖面**：不包含 APU 与 cartridge 状态——用它们做
   「保存→恢复→继续跑」的回滚测试时，音频/器状态静默丢失，长程 trace 可能出现
   音频相关的假差异。

## 与本项目的关系
- oracle/ 的回放与采集 harness 若未来引入 savestate 快照回滚（如「从中间态复跑末段」的
  差分定位），必须：①只在 start-frame 回调快照；②明示 APU/cart 态不在快照内，
  涉音频的判定不可用该径。
- 与既有 gotcha-mesen2-callbacktype-string-enum-split.md（回调枚举字符串坑）、
  gotcha-concurrent-mesen-output-corruption.md（并发产物污染）同族——Mesen Lua API 的
  语义陷阱面。

## 验证状态
本条目语义**未本地复现**（来自上游 issue，未经我们 harness 实测踩实）——按 README 准入
纪律标注为 confirmed-unverified-locally：引用其做设计决策前，先在 oracle/ 用最小脚本
踩实；若实测发现语义不符，原地更正本条目。
