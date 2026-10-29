---
type: gotcha
title: gates check 的 l4 门禁自跑 run-all——多会话并发即假 blocked FAIL（并行不可信家族第 5 员）
date: 2026-10-29
status: confirmed
---

# gotcha：gate_l4 自跑 run-all 与并发 run-all 撞车 → 假「新增 blocked」FAIL

## 现象

2026-10-29 r9 双会话并发窗内实测两次：会话 A 的 pre-commit `gates/cli.py check`
在另一会话 run-all 活跃时 **l4 门禁 FAIL**，失败形态恒为
`case-002-door-blue-interaction 6+1 采集点：新增 blocked（基线 未锚定 → 现态 blocked）
——回退 FAIL（blocked 只减不增）`（match/diff 数同步缩水）；而
- 该 case 自身 `analysis/traces/case-002-*.run.json`（并发方刚写）显示
  `done{captures:8}/timed_out:false` **干净完成**——blocked 是**读取面**产物，
  不是运行面失败；
- 同窗对方会话串行执行的 check 为 `PASS=13 FAIL=0`（e9ca3957 复核档 §4 实录）。
- ps 实录两次重叠：run-all #1（17:14–17:19）与 A 的 check 重叠；run-all #2
  （17:25 起）与 A 的第二次 check 重叠。

## 为什么

- `gate_l4` **自身 subprocess 跑一遍 `tools/l4_diff_run.py run-all`**
  （gates/cli.py:1187）——任何 check 都是一次全量 L4 重采+矩阵重写。
- 产物路径族（`analysis/traces/*.ram.hex`、`shots_rust/matrix.json`、
  `analysis/_gen/*.lua`、`shots/`）**跨会话共享且无跨进程串行锁**：防踩踏三件套的
  `analysis/.scenario.lock` 锁的是 oracle/scenarios 面，l4_diff_run 此路径族不取锁。
- 并发窗内两个 run-all 交错重写同名产物 ⇒ 先完成方/裁定方读到**半成品**
  ⇒ 采集点判 blocked ⇒ ratchet「blocked 只减不增」误报回退。
- 与 [[gotcha-concurrent-mesen-output-corruption]] 同族（并行不可信家族）：
  写入端无实例隔离 ⇒ 读端拿到拼接态。**诊断判别式**：FAIL 形态=「新增 blocked」
  且 case 自身 run.json `done` 干净 ⇒ 并发撞车（环境层）；FAIL 形态=match 缩水
  且双侧 run.json 一致 ⇒ 真内容分歧（产品层）——先分层再动手（AGENTS.md §4）。

## 何时使用

- **任何 `gates/cli.py check` / 手动 `run-all` 前先查活跃 run**：
  `ps aux | grep -E "l4_diff_run|scenario_run|mesen2/Mesen"`——多 AI 会话共用
  工作区时必查（本条目的直接成因）。
- l4 FAIL 且形态为「新增 blocked」时：先读涉事 case 的 `run.json`（done 块）
  与 ps 时间线定层，**禁止**当产品回退去动代码，也**禁止**盲目反复重跑。
- 【2026-10-29 落地】长期解已收口：`tools/l4_diff_run.py` 接 run 级串行锁
  `analysis/.l4_run.lock`（RunLock——独立于内层 `.scenario.lock` case 级锁；
  先非阻塞、被占显式轮询等待、上限 `L4_RUN_LOCK_TIMEOUT_S`（默认 600s）、超时
  fail-loud exit 2=gate 拒跑理由可见；仅 `__main__` 取，gates selftest 进程内
  探针不受影响）。等待+超时两径实测过（占锁 8s→取得继续；占锁 30s+限 4s→exit 2）。
  覆盖前归档由 scenario_run `archive_existing`（锁内）既有机制承担，不重复建。

## 证据指针

- `gates/cli.py:1187`（gate_l4 self-spawn run-all——机制锚，A 级代码读证）
- commit `e9ca3957`（同窗对方会话串行 check `PASS=13 FAIL=0` 的复核档实录——
  同树串行窗即绿，反证 FAIL 非产品层）
- 本会话（r9 双会话窗）两次失败输出与 ps 时间线（现象节实录；工作区
  analysis/ 不入库，run.json 为即时证据）
- 家族条目：`docs/knowledge/gotcha-concurrent-mesen-output-corruption.md`
  （并行不可信家族；本条为第 5 员）
