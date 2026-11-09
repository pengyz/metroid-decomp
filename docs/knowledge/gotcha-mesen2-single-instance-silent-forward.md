---
type: gotcha
title: Mesen2 per-user 单实例——并发启动静默转发后 exit 0（不报错无日志）
date: 2026-11-09
status: confirmed
---

# gotcha：Mesen2 单实例并发互吞

**来源**：AccuracyCoin 卡实测（AccuracyCoin 采证时两 agent 并行跑 Mesen 复现）+ 源码
Mesen2 Program.cs:78 `SingleInstance` 佐证。留档人：lead（porter 0ca89b88 发现并广播）。

## 坑
1. **per-user 单实例**：任何 agent 的 Mesen 已在跑时，第二个进程启动会把 ROM 参数经
   CoreFxPipe **转发给已有实例后 exit 0 静默退出**——不报错、无日志、无产物。
   这解释了历史上多起「Mesen 秒退无产物」假象。
2. **全新 HOME 无 settings.json**：走 wizard-restart 分支同样秒退（headless/CI 环境
   首跑必踩）。

## 处置范式
- 并发跑 Mesen 前抢槽：`pgrep -x Mesen` 确认无实例才启动 + 启动后 ~6s 存活确认
  （参考实现 /tmp/acc_slot.sh；持久化版本应进 tools/，勿依赖 /tmp）。
- headless/CI 环境先保证 HOME 有已初始化的 settings.json。
- 产物缺失时先查「是否有别的 Mesen 实例活着」，再查 harness 本身。

## 与既有 gotcha 同族
gotcha-concurrent-mesen-output-corruption.md（并发产物污染）、
gotcha-mesen2-callbacktype-string-enum-split.md（回调枚举坑）、
gotcha-mesen-savestate-callback-constraint.md（savestate 约束）——Mesen 并发/自动化
陷阱面第四件。
