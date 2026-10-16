---
type: gotcha
title: teammate 执行会话信号噪声——分类法与甄别规则
verified: true
---

# gotcha: teammate 会话信号噪声（failed/finished 翻转类）

## 现象
lead 频繁收到 `Background subagent <id> failed before it finished` / `finished` 通知，
带碎片 closing message 或无消息；同一 id 多次交替出现（82d0d97b/347f8998/1e2137bd/d00b9004 等）。

## 成因（三层）
1. teammate=持久实体+逐轮执行会话，每轮结束必发信号（正常收口也会发）；
2. 平台会话状态机抖动=重复/陈旧信号（r8 §2 记档的「信号翻转」）；
3. 长任务会话中断=failed+碎片（碎片=最后流式输出，常含有效进度线索）。

## 甄别规则（lead 处置）
- **无内容信号** → 忽略（占绝大多数）；
- **带碎片** → 提取进度线索（如「批 4b 中段/已落 X」），不作状态判定；
- **真丢工作判据** = 仓内实况核（git log/board/工作树）：声明的 commit 不在=真丢→重派；
- 会话翻转 ≠ 工作丢失（durable teammate 跨会话存活；小步提交保工作树持久）。

## 处置
源头在 harness 层（信号去重/陈旧抑制是 DSH 仓库改进面）；应用层靠甄别纪律无害化。
「平台噪声以仓内实况定损」（r8 §2 原则）为权威处置口径。
