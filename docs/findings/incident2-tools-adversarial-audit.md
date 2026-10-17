# findings：事故②对抗审——tools/oracle 交互路径（全员调查令·decomp 视角）

> 状态：**审结**（2026-10-17）。判据：每条带复现命令。与锁根因合流分析见 §5。

## ① Mesen2/Lua 全交互面挂死清单（tools/*.lua 16 API 全分类）

| API（调用数） | 挂死风险 | 处置 |
|---|---|---|
| `emu.createSavestate`(3) | **无参=挂死**（v2 超时 124 实录=原生对话框）；带参须 exec 回调内（v3 错误串） | ✅ lint 禁用清单（task-38 ③） |
| `emu.loadSavestate`(2) | 无参同族面 | ✅ lint 同禁 |
| `emu.setState`(1) | ✅ 无挂死（v3 实证 ok=true 即返） | — |
| `emu.takeScreenshot`(3) | ✅ 返回字节串（非对话框） | — |
| `emu.getState/getCdlData/getAccessCounters`(20) | ✅ 查询即返（v1-v4 全程实证） | — |
| `emu.addMemoryCallback/addEventCallback`(31) | ✅ 注册即返 | — |
| `emu.read/write/setInput/stop`(16) | ✅ 即返 | — |
| **Mesen CLI 启动面** | ROM 路径错/无显示=**错误对话框挂死**（无 X 时对话框不可见=永等） | ✅ run 环 SIGTERM→SIGKILL 双段杀（10s+10s 有界）+全局 timeout_s |

**结论：lint 禁用清单 2 条已覆盖全部无参挂死面；剩余挂死面=CLI 对话框（杀局覆盖）。**
复现：`python3 tools/lint_mesen_lua.py /tmp/bad.lua`（exit 1）；`grep -c "emu\." tools/*.lua`。

## ② spawn 点超时覆盖矩阵（事故②后全量重导）

| spawn 点 | 用途 | 超时态 | 处置 |
|---|---|---|---|
| diff_subroutines:39 | l2-diff/l2-casegen | ✅ 600s（task-38） | — |
| **diff_traces:168** | scenario_run 重采集 | ❌ **外层无超时**（内层 --timeout 300 但外层永等=真空） | ✅ **本审补 360s**（超限=FAIL） |
| **int_x_smt_proof:131** | SMT 桥 | ❌ **无超时**（病态求解可永转）+**硬编码 ./target/release**（与 CARGO_TARGET_DIR 独立化冲突！） | ✅ **本审补 120s+TARGET_DIR 适配** |
| scenario_run:630-643 | Mesen/xvfb Popen | ✅ 全局 timeout_s+双段杀局 | — |
| scenario_run:539-551 | taskkill/pgrep 工具 | 🟡 无超时（一次性短命工具，pgrep/proc 面稳） | 登记（低危） |
| gates 族（lead 面） | cargo/test 面 | ✅ PROOF_*_TIMEOUT_S 已有 | 锁根因另面（§5） |

复现：`grep -n "subprocess.run\|Popen" tools/*.py`（本表源）；`grep -n "timeout" tools/diff_traces.py`。

## ③ IDA/REA 分析桥假说验证（重点）

**取证**（复现命令在下）：
```sh
ps -o pid,ppid,stat,etime,cmd -p 2962238   # [python] <defunct> Zs 06:18:17，PPID=2545800
ps aux | grep idalib_supervisor            # python -m ida_pro_mcp.idalib_supervisor
                                           # --host 127.0.0.1 --port 13337 --unsafe（Oct16 起活体）
ss -tlnp | grep 13337                      # LISTEN pid=2545800 ✓ 桥在服务
grep -rln "ida_pro_mcp\|idalib\|idapro" tools/ crates/ oracle/ gates/
# → 空（项目源零引用）
```

**判定**：
1. **ref_atlas/decode_sweep 不经此桥**（纯 Python ROM 解析——项目源零引用实证）✓；
2. **idapro=注册 MCP 工具面**（会话缓存实录：「mcp-client(idapro): server is disconnected」
   「idapro 直连通道全程参与裁决」「decompile session 1af5c89c」——他会话二进制裁决实况）；
3. **僵尸成因**（B 级推断）：idalib 分析 worker 死亡后监管器未 wait()=僵尸泄漏；若 MCP
   客户端调用在 worker 死后无超时=**调用方永等=工作流挂死**（事故②层真空面）；
4. **我项目调用资格**：agent 工具面（mcp__ida*）可调——本 session 未调；子 agent 工作流
   若调用即暴露于该真空。

## ④ 全部外部调用点独立重导

| 调用点 | 目标 | 防护 |
|---|---|---|
| scenario_run | Mesen 二进制（xvfb-run 包装） | 双段杀局+全局 timeout ✓ |
| scenario_run | xvfb-run/Xvfb | 同上 ✓ |
| diff_subroutines | cargo run l2-*（进程内） | 600s ✓+L2 MAX_STEPS 1M ✓ |
| diff_traces | python scenario_run（重采集） | **360s（本审补）** |
| diff_traces/diff_visual | 纯 stdlib 文件 IO | 无外部面 ✓ |
| int_x_smt_proof | int-x-bridge（z3） | **120s（本审补）** |
| gates（lead） | cargo 全家 | 超时有+**CARGO_TARGET_DIR 独立化**（§5） |
| agent 工具面 | idapro/rea MCP | **真空（harness 层——不在 tools 边界内）** |

## ⑤ 与锁根因合流分析（两面都要防）

- **①层（实锤）**：cargo artifact flock 等待（verifier 复现）——**CARGO_TARGET_DIR
  独立化根治**（`export CARGO_TARGET_DIR=/tmp/tgt-decomp`）；合流点=本审发现
  int_x_smt_proof 硬编码 `./target/release`（已适配环境变量）；
- **②层（真空）**：MCP 调用无超时护栏（idapro/rea 桥）——**harness/工作流层需加
  MCP 调用超时+僵尸回收**（idalib_supervisor wait() 修复）；tools 层同类真空
  （diff_traces 外层/int_x 桥）**本审已闭**；
- 两面教训同源：**每个 spawn/每个跨进程调用必须有界**（超时=FAIL 显式）——
  task-38 护栏原则外推到 MCP 层。
