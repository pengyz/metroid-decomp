# 事故②根因判决书（四线对质会合议，2026-10-17）

## 判决：串联放大链（三因非互斥，频率×持续×深度）

| 因 | 定位 | 证据 | 角色 |
|---|---|---|---|
| **A 锁等待**（机制本体） | cargo artifact 锁：A 长构建持锁→B pre-commit 无界 cargo test→无限阻塞 | verifier flock 确定性复现（"Blocking waiting for file lock" 实测） | 两次事故的直接触发 |
| **B 层叠真空**（持续条件） | gates:776 裸 subprocess / MCP connect:308 无界×调用 60s 不对称 / turn 无 deadline | port2 字节级三层（含 fork 后端零界 grep 证） | 为何等待挂得住（任何一层有界即自愈） |
| **C 机理环**（第二触发源） | P1 走带环无终止符（release exit 124 真挂实证）/P2 stdin 裸读/P3 时长面/P4 hook 裸 exec | porter 五原语枚举+135 测试零阻塞清单 | 独立挂死通路（运行时环类） |

## 焦点② 修复分类学（三靶类覆盖矩阵——task-38 复盘定论）

| 靶类 | task-38 覆盖 | 本次补面 | 状态 |
|---|---|---|---|
| 逻辑挂死 | ✅ 全治（watchdog/步数/内超时/lint） | — | 闭合 |
| 资源等待 | ❌ 零覆盖（=第二次卡死的真因） | CARGO_TARGET_DIR 独立化（根治）+gates 三处 timeout+spawn 两处补界 | 闭合 |
| 交互挂死 | ⚠️ 部分（Mesen lint） | Lua 16 API 全分类+CLI 对话框双段杀局 | 仓内闭合 |
| （跨界层） | — | **MCP 调用无超时+idalib_supervisor wait() 泄漏+turn 无 deadline=harness 层待修** | 显式外挂账 |

**教训**：修一类不防一类——修复必须按靶类分类验收（task-38 的四面全治「逻辑挂死」，与「资源等待」是两个靶类）。

## 焦点③ 防线实证判据（坏例注入验收）

| 靶类 | 坏例 | 打回形态 |
|---|---|---|
| 逻辑挂死 | P1 走带环 | 加界→exit 101 快败 |
| 资源等待 | flock 持锁→pre-commit | 独立 target/超时→即退 |
| 交互挂死 | P4 hook 裸 exec/MCP 无界 | 超时包裹→fail-loud |

> 【2026-10-17 更正（task-41 坏例验收抓获）】根治不完整：**真锁面=~/.cargo/.global-cache
> （fcntl F_SETLK）非 target 目录**——独立 target 下 cache 锁持有仍 124 打回（8s 实录）；
> flock≠fcntl 锁型（flock target 无效）；F_SETLK 非阻塞→cargo 重试自旋=「外观挂死」机制全释。
> **根治修正=target+CARGO_HOME 双独立**（判决书原文「CARGO_TARGET_DIR 独立化=根治」降为半治）。
> 好例证明：防线自己的坏例验收抓出修复缺口——「防线先过证伪」的自指价值实证。

## 已落修复（本判书确认面）
- verifier 76fbce1：gates 三处 timeout（30/600/900s）+超时=FAIL 显式
- decomp 367bac6：spawn 两处真空补界（360s/120s）+int_x 双形适配
- 全队：CARGO_TARGET_DIR 独立化（半治——见更正注）+ **CARGO_HOME 双独立**（补令，真根治）
  引导法：`cp -al ~/.cargo /tmp/cargo-home-<agent>` 硬链快照（免重下载、省盘）
- task-38 遗产：watchdog/步数/lint（逻辑类）

## 外挂账（harness/工作流层——不在仓边界，建议书）
1. MCP 调用级超时（connect+call 双侧对称）
2. idalib_supervisor wait() 回收（僵尸泄漏修）
3. subagent turn deadline（run.ts 层收束）
4. 超时层叠矩阵文档（层×界态×真空面检查表）

## 同源教训（知识条目）
**每个跨进程调用必须有界**（进程/MCP/锁/turn 四面同则）——无界=挂死只是时间问题。
