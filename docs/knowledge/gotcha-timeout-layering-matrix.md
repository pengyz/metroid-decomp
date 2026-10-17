---
type: gotcha
title: 超时/阻塞层叠矩阵（事故②防复发检查表）
date: 2026-10-17
evidence: docs/findings/incident2-blocking-primitives-review.md（事故②根因调查）
tags: [超时, 挂死, spawn, mcp, cargo-lock, 防复发]
---

# gotcha：超时/阻塞层叠矩阵（事故②防复发检查表）

> 【来源】事故②根因调查（docs/findings/incident2-blocking-primitives-review.md）+
> verifier 锁根因+CARGO_TARGET_DIR 独立化根治令（lead）——完整因果图三半合流。
> 【用法】任何新命令链/工具接线/子 agent 任务前对照本表：层×界态×真空面——
> **真空面行=必须显式补界或声明豁免理由**。
> 【教训主句】内层有护栏≠外层有界；tool 层不阻塞≠turn 层有收束；共享锁未隔离=隐性串行。

## 层叠矩阵（层 × 界态 × 真空面）

| # | 层 | 代表点（证据坐标） | 界态 | 真空面 | 防复发检查 |
|---|---|---|---|---|---|
| 1 | 平台 tool 调用 | bash 工具 timeoutMs（到期转后台 job） | **有界** | 无（后台 job 需手动 job_output 收） | 长命令显式传 timeoutMs；后台 job 记 id 必收 |
| 2 | **subagent turn** | subagent-dsh-sdk/run.ts（仅 caller abort+1s 清理界）；fork 后端零界 | **无 deadline** | **真空**：tool 早返（立即返 id）无触发点 × turn 无收束——子 agent 循环/挂住=无限 | 长任务子 agent 拆批+显式轮询；平台 turn deadline 落地前靠拆批收束 |
| 3 | MCP tool 调用 | mcp-client connection.ts:370 `timeout: toolCallTimeoutMs`（默认 60s） | **有界** | 无 | 保持默认；长查询显式调 toolCallTimeoutMs |
| 4 | **MCP connect/握手** | connection.ts:308 裸 `await generation.connect(transport)` | **无界** | **真空**：僵尸 worker（进程在/握手不回）无限挂——catch 只接 error 挂住不 throw | 僵尸 worker 场景=重挂 MCP 服务；connect deadline 落地前以此为先手处置 |
| 5 | **gates→runner 子进程** | gates/cli.py:776 裸 `subprocess.run([python, runner])` | **顶层无界** | **真空**：runner 自身挂死（内层护栏失效/盘停顿）时门禁无限等 | 门禁调用前确认无并发场景跑；runner timeout 落地前长挂=按层 4/6 处置 |
| 6 | 仓内脚本内层 | scenario_run.py:110/647（三层：poll+死线+告警）/:504 锁 LOCK_NB fail-fast/decode_sweep:394 单调环 | **有界** | 无 | 新脚本环必带：死线+存活检+单调推进三选全 |
| 7 | **共享锁（git/cargo）** | .git/index.lock（并发 commit 等待）；cargo 共享 target 构建锁（**竞争=卡死工作流机理面**） | **隐性串行** | **真空**：锁持有方挂住=全体等待无限 | **CARGO_TARGET_DIR=/tmp/tgt-<name> 独立化**（根治令已施）；git 锁=查进程确认活跃再等（禁盲删） |

## 既判案例锚

- **t24 事故**→scenario_run 串行锁（:482 docstring）；**t54 F3**→孤儿清扫（:521）——内层收束先例。
- **事故②两次卡死**→本表 #2/#4/#7 合成面：子 agent 卡 MCP 握手（#4）+turn 无收束（#2）
  +共享锁竞争（#7）——三真空链无外层界。
- **port2 红窗两案**：git index.lock 并发等待（查进程后等释放 30s ✓ 正确姿势）；
  cargo 共享 target 竞态（**根治=CARGO_TARGET_DIR 独立化**——同会话实施后全绿实证）。

## 新增代码/接线检查单（复制用）

```
□ 命令/子进程：显式 timeout= 或三层环（死线+存活检+单调推进）
□ 子 agent 任务：长活拆批+轮询收束（turn deadline 前）
□ MCP 接线：调用面 60s 保持；连接期挂死先手=重挂服务
□ 共享资源（锁/构建目录/输出目录）：独立化（CARGO_TARGET_DIR 式）或 fail-fast 锁
□ 长跑进程：后台 job 记 id+收口
□ 检查点：每个真空面要么补界要么在任务卡「遗留假设」声明豁免理由
```

【分类】A 级（源码行引文+根治令实证）；【置信】92
