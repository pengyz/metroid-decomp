# 事故②根因调查（调用链/阻塞原语对抗审——port2 独立重导）

> 【范围】全员对抗复合调查令之三视角：①全仓阻塞原语定界 ②MCP 挂死面 ③超时层叠真空。
> 【方法】独立重导（不看他人结论）；每条带字节/进程级证据（grep 行号+源码引文）。
> 【结论】**三层真空链**确证：tool 层早返（无触发点）+ turn 层无 deadline + MCP connect
> 层无界——子 agent 卡死=链上任一环挂住即无外层收束。

## ① 阻塞原语定界（metroid-decomp 仓）

| 原语 | 位置 | 护栏 | 失效面 |
|---|---|---|---|
| Xvfb 等待环 | tools/scenario_run.py:110-115 | **三层**（poll 存活检+timeout_s 死线+0.1s 节拍） | 无 |
| 串行锁 acquire | tools/scenario_run.py:504 | **fail-fast**（`flock(LOCK_EX\|LOCK_NB)`：OSError⇒ScenarioError 即抛） | 无（非阻塞语义） |
| 场景等待环 | tools/scenario_run.py:647-670 | **三层**（timeout_s 死线+proc.poll 20s 宽限+时长告警） | 无 |
| pgrep/taskkill/powershell 子进程 | tools/scenario_run.py:539-551 | **无显式 timeout** | 自然有界（本地短命令）；Windows WMI 卡死=未界（低概率面） |
| 扫描环 | tools/decode_sweep.py:394 | **单调推进**（find 位置递增+miss 即 break） | 无 |
| **gates→runner 子进程** | **gates/cli.py:776** | **无 timeout**（`subprocess.run([sys.executable, runner])` 裸调） | **顶层无界**：runner 自身挂死（内层护栏失效/文件系统停顿/Rust 死锁）时门禁无限等待——内层有护栏≠外层有界（护栏失效面真空） |

**进程级佐证**：t24/t54 事故后补的锁与清扫（:482 docstring「t24 事故」/:521「t54 F3 根因防护」）
=既往卡死事故已在内层收束；顶层 gates 层未同步补界=残余暴露面。

## ② MCP 工具调用挂死面（dsh 检出 packages/mcp/mcp-client）

| 调用面 | 位置 | 界 | 僵尸 worker（半残：进程在/握手不回）行为 |
|---|---|---|---|
| **tool 调用/资源请求** | connection.ts:370 `timeout: config.toolCallTimeoutMs`；index.ts:37 默认 **60_000ms** | **有界** | 超时返回（不挂） |
| **connect/握手** | **connection.ts:308 `await generation.connect(transport)`** | **无 timeout 包裹**（无 AbortSignal/deadline/race） | **无限挂起**——catch 只接 error，挂住不 throw ⇒ 注册期卡死 |
| 启动同步 | connection.ts:320 `await enqueueSync(generation, ...)` | 无显式界 | 同上 |
| 重连 | :239 reconnectTimer | 有计时 | 重连=新 generation（connect 期同样无界） |

**字节级证据**：`grep -n "connect(" connection.ts` → :308 裸 await；对照 :370 调用侧显式
`timeout:`——**调用有界/连接无界不对称**=僵尸 worker 挂死面的精确坐标。

## ③ 超时层叠真空（subagent 平台层 vs 脚本层）

| 层 | 位置 | 界 | 说明 |
|---|---|---|---|
| subagent tool 调用 | tool 契约（「immediately returns its id」） | **早返**——不阻塞 | **无 tool 级 timeout 触发点**（不等待⇒timeout 无意义/无从覆盖 turn） |
| subagent turn | subagent-dsh-sdk/run.ts | **无 deadline**——仅 request.signal abort（调用方主动）+shutdownTimeoutMs=1_000（清理界非运行界，:79） | 子 agent 循环/挂住=无限运行 |
| fork 后端 | subagent-fork-in-process/src/index.ts | grep timeout/deadline/abort **零命中** | 比 dsh-sdk 更薄 |
| 脚本层（子 agent 内 bash/工具） | bash 工具有 timeoutMs（到期转后台 job）/MCP 调用 60s | **局部有界** | 局部超时被 agent 行为（重试/换路）吸收——**turn 级无收束** |

**真空合成**：tool 层早返（无触发点）× turn 层无 deadline（无收束）× MCP connect 无界
（挂因）= 子 agent 若卡在 MCP 握手（②）或工具重试环（③）——**两层叠加无任何外层界**
= 事故②「两次卡死工作流」的链式根因形状。

## 修复建议（按界层）

1. **gates/cli.py:776** 补 `timeout=`（runner 层级界——如 600s；到期 FAIL+进程组杀）；
2. **connection.ts:308** connect 包 deadline（如 30s——AbortSignal.timeout 语义与 :370 对齐）；
3. **subagent turn 级 deadline**（spec 加 turnTimeoutMs 默认值（如 30min）+到期 abort——
   补 turn 层收束点）；fork 后端同步；
4. 两层声明文：docs/knowledge 落「超时层叠矩阵」（tool/turn/connect/脚本四层各自的界
   与真空点清单）。

【分类】A 级（源码行引文+字节级对照）；【置信】90
