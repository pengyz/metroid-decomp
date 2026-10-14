# AGENTS.md — AI 纪律总纲（单一真相源）

> 面向所有 AI 实例（DSH/Claude Code/Codex/其他）。本文件是纪律的**唯一权威**；
> 其他文档与本文件冲突时，以本文件为准并提交更正。方法论背景见
> [docs/methodology.md](docs/methodology.md)，自动化管线设计见
> [docs/ai-decompile-pipeline.md](docs/ai-decompile-pipeline.md)。
> 纪律框架移植自 gunnac_remake（NES/6502 同构场景，再上溯 xiyou_remake/PAL3Decomp 教训）。

## 0. 项目与已锁定决策

《银河战士 / Metroid》（Nintendo R&D1，1986，NES USA/NTSC，Mapper 1/MMC1，
PRG 128KB（8×16KB bank）+ CHR-RAM 8KB）重制：**以社区匹配反汇编与 ROM 取证为证据基础** →
理解 → **Rust 改写为 PC 原生游戏** → 差分验证。
两阶段：**Stage A 行为一致** → **Stage B 现代化**。

**ROM 身份锚点**（`rom/Metroid (USA).nes`，BYO-ROM，仓库不含版权数据）：

| 项 | 值 |
|---|---|
| SHA-256 | `e6e6b7014685adae447ebb3833242815747bc1e5df83ade79f693fb67cf565b6` |
| MD5 / 整文件 CRC32 | `d77c8053168da14b360bf5caeccc5964` / `A2C89CB9` |
| headerless PRG+CHR CRC32 | `70080810`（内容指纹，社区通用） |
| 结构 | 8×16KB PRG：bank0–6 映射 `$8000` 切换，bank7 固定 `$C000`；CHR-RAM（图案表由 PPU 写入） |

| # | 决策 | 内容 |
|---|---|---|
| D1 | 语言 | Rust（桌面优先，预留 wasm）。C++/TS 已否决 |
| D2 | 路线 | 先行为一致后现代化；A 阶段预留扩展缝（分辨率/输入/音频/存档/UI） |
| D3 | 锚点 | **ROM 字节 + Mesen2 执行是最终真相**（`rom/` 永不修改，sha256 门禁校验）。IDA/Ghidra/反汇编与伪代码是**可再生投影**，不是真相；`reference/seed/` 投影冻结为基准 |
| D4 | Bug | Stage A **保真保留原版 quirks**（8 sprites/scanline、闪烁、掉帧、溢出、密码系统怪癖），只登记 [docs/bug-ledger.md](docs/bug-ledger.md)（`BUG-xxx`），Stage B 修复 |
| D5 | 反编译路线 | 6502 **没有 Hex-Rays**，四方交叉：**m1disasm 社区匹配反汇编（已验证与 ROM 逐字节等价）** + IDA（`m65` 模块）静态分析/数据表/xref + Ghidra（6502 SLEIGH）伪代码 + Mesen2 动态 trace；**LLM 负责语义化，判定权在门禁**。第三方命名只作线索，语义结论必须自证（台账隔离） |
| D6 | 版本范围 | 保真期只做 **NES_NTSC**（等价性证据：[docs/findings/rom-disasm-equivalence.md](docs/findings/rom-disasm-equivalence.md)）。PAL/FDS/MZM/CNS 等官方变体差异不进保真期，单列 findings |
| D7 | 完备性 | **分母独立于测试、机械可导出**（矩阵四面 S/D/B/R，`tools/completeness_denoms.py`）；对拍只裁「对不对」、不证「全不全」（绿≠完整）。缺口左移四机制：① L2 勾格清单（有限分母逐格走 `未实现→已实现→有L2→已验证`）② Rust 侧 `unported!` fail-loud 桩（缺口确定性爆在 bank:addr，形态上与移植错可分）③ oracle 覆盖驱动语料（`coverage` 只增不减，盲区=下一条场景目标）④ 首分歧点四分类路由（缺口=一张任务卡，不进调试循环）。详见 [docs/ai-decompile-pipeline.md](docs/ai-decompile-pipeline.md) §4 |

## 1. 哲学（决策优先级）

**逻辑一致性 > 代码优雅 > 工具便利 > 个人偏好。**

- **裁判是差分测试，不是"看起来对"**。6502→Rust 搬运的正确性由 trace/状态快照/截图 diff 判定；
  人肉 review 只能发现风格问题，不能证明等价。
- **机器码是最终真相，反汇编/伪代码只是投影**。反编译是有损的：我们不是"恢复"原源码，
  而是再创作一个行为完全一致的程序。任何投影与 ROM 行为冲突时，以 ROM 为准。
- **参考轨道永久存活**：`rom/` 与 Mesen2 oracle 一切验证的基准，永不删除、永不迁就 Rust 侧。
- **保真期的行为只增不减**：原版有的行为（含 bug）必须保留；原版没有的行为（含修复）禁止引入。

## 2. 仓库结构与红线

| 路径 | 职责 | 红线 |
|---|---|---|
| `rom/` | 原始 ROM 只读归档（唯一资产真相源） | **任何字节修改 = 严重违规**；`gates/cli.py rom` 校验 sha256 |
| `reference/seed/` | 冻结投影：规范化 6502 反汇编 listing | 不可变基准，重生成需 commit 说明 |
| `reference/src/` | 命名/注释后的参考汇编轨道（可汇编、可比对） | 只允许语义等价的改名/注释/拆分；行为变化必须差分证据不变 |
| `refs/` | 社区参考源码（m1disasm / MetroidMMC3 / nmikstas），只读 | **只作线索**；命名不直接采信（D5），不修改、不入 git（可重建） |
| `oracle/` | Mesen2 自动化宿主：Lua harness、输入回放、trace/RAM dump、截图 | 比对逻辑不得为"通过测试"放宽阈值 |
| `analysis/` | 可再生工作区（IDA idb、Ghidra project、trace 导出） | 不入库；由 `rom/` + 工具可复现 |
| `docs/spec/` | 规格书（S4 产物） | 每条结论必须带证据指针（汇编地址/字节偏移/trace id） |
| `docs/findings/` | 考证记录 | **必须区分"已证实 / 假设"**，假设不得混入结论 |
| `docs/knowledge/` | 跨会话共享知识库 | 写入规则见其 README；引用前必须验证仍存在 |
| `data/naming/` | 命名台账（rename 证据账本，两层：机械 remap + 语义 ledger） | 每条 rename 必须有 evidence 字段（可复现指针） |
| `data/golden/`、`data/status/baselines/` | golden 与门禁基线（ratchet） | 基线只升不降，更新必须写 commit 说明 |
| `crates/` | game-data（资产解析）/ game-core（纯逻辑）/ game-platform（trait）/ game-oracle（差分宿主） | game-core 无 I/O、确定性（虚拟时钟 + GameRng）；禁止平台依赖 |
| `gates/` | 门禁工具与 git hooks | 不得绕过/弱化/删除门禁；改门禁必须单独 commit 并写明基线影响 |
| `prompts/` | 角色提示词 | 新任务必须先选角色 prompt（见 §7） |
| `tools/` | 可再生工具链（WLA-DX、seed 归一化、diff 裁定器、Mesen harness） | 工具自身不持有判定权；判定逻辑只在 `gates/` |
| `assets/` `target/` | 可再生工作区 | 不入库 |

## 3. 证据纪律（本项目最核心的纪律）

证据分级（与台账/commit message 中的 `Confidence` 配套）：

| 级 | 定义 | 例 |
|---|---|---|
| **A** | 硬证据：字节偏移、汇编行（bank:addr）、Mesen2 trace、RAM dump、资源 hexdump | `bank7:$FFB0` 处 `sei/cld/…/jmp $C01A` 与 ROM 字节 `78 D8 … 4C 1A C0` 逐字节一致 |
| **B** | 代码语境 + 调用图/数据流推断（可复现的静态推理） | 子程序只被 NMI 路径调用 ⇒ 帧同步逻辑；数据表两端被同一索引寄存器寻址 |
| **C** | 直觉/剧情猜测/命名相似 | "这个表可能是敌人速度表"（仅凭名字像） |

规则：

1. **A/B 级才能落台账、改名、写入 spec 结论**；C 级只能进 findings 的"假设区"，禁止据此 rename。
2. **禁止伪语义命名**：`table_0320`、`enemyThing2` 这类地址/序号后缀伪装的名字没有语义价值。
   不确定含义就保留 `_todo_`，并在台账记 C 级假设。
3. **禁止猜测性格式结论**：数据表/资源格式的"已证实"必须能给出字节级复现命令（hexdump/python 一行验证）
   或运行时 trace 佐证。
4. **禁止批量未验证 rename**：一个符号一条台账记录，证据独立可查。
5. **禁止试错式修改**：不理解机制前不动解析器/核心逻辑。先考证（hexdump → 反汇编 → Mesen2 插桩 trace），
   画出完整使用图景，再动手。
6. **证伪更正文化**：结论被推翻后，在原文**原地标注日期更正**（`> 【YYYY-MM-DD 更正】…`），
   不删除历史主张。
7. **第三方命名不背书**（D5）：m1disasm/MetroidMMC3 的符号名是别人的语义结论，
   采用前必须自证并落台账；直接照抄第三方名 = 未取证改名。

## 4. 动手前三问（跨层修改的强制预检）

修改任何跨层内容（ROM 投影/参考轨道/oracle/核心逻辑/门禁/台账规则）前，必须**只读地**回答，
并把答案写进 commit message：

1. **问题在哪一层？**（反汇编投影错？数据表理解错？Rust 移植错？oracle/门禁本身错？）
2. **消费者实际读的是哪个文件/哪份数据？**（解析器读 ROM 原始字节还是转换产物？golden 从哪来？）
3. **证据是什么级别？**（答不出 A/B 级证据 ⇒ 不许动手，先去取证。）

## 5. 强制检查点（门禁表）

先跑只读预检 `python3 gates/cli.py status`。**SKIP 与 TOLERATED 必须显式打印，绝不静默通过。**

| 时机 | 门禁 | 命令 | 失败处理 |
|---|---|---|---|
| 动手前 | 状态预检 | `python3 gates/cli.py status` | 看清 SKIP/恒红项再动 |
| **任何 commit** | 总闸（pre-commit 自动） | `python3 gates/cli.py check` | 任一 FAIL ⇒ 不提交 |
| 改 `data/naming/` `data/status/` `reference/` `rom/` | 证据门禁（commit-msg 自动） | commit message 含 `Evidence`/`证据` | 缺证据字段 ⇒ 拒绝 commit |
| **任何步骤完成后** | **独立对抗复核**（用户裁定，继承自 gunnac_remake） | 审核 agent（verifier 角色，**不得为实施方**）以**证伪立场**独立重导证据 | **复核 verdict=pass 才算步骤完成** |
| 改参考投影/轨道后 | 投影回归 | `python3 gates/cli.py seed` | seed sha 不变；src 轨道 trace 不变 |
| 改 game-data/game-core 后 | 差分（L1–L3） | `python3 gates/cli.py trace` | 任何回退 ⇒ 停止，先定位层（§4） |
| 改渲染后 | 视觉（L4） | `python3 gates/cli.py visual` | 白名单外像素必须一致 |
| 里程碑收尾 | 全量 | `python3 gates/cli.py all` | 对照管线文档阶段 Gate |

- **门禁失败不是"多跑几次"能解决的**：按 §4 定位层，修复根因，不许调阈值/加白名单掩盖。
- 视觉白名单（字体/调色板映射等）是**唯一**允许的例外清单，变更需用户确认并记录。

## 6. Bug 台账纪律（D4 的执行细则）

- 运行/差分中发现原版可疑行为 ⇒ 记入 `docs/bug-ledger.md`（现象/复现/影响/trace id），
  代码侧以 `BUG-xxx` 注释引用，**Stage A 不修**。
- 差分测试遇到已登记 bug：登记为"已知偏差"，基线显式 TOLERATED 并注明 BUG 编号。
- 禁止"顺手修 bug"——那会直接击穿一致性验证。

## 7. 提示词纪律

详见 [prompts/README.md](prompts/README.md)。要点：

- 新任务必须先选定角色 prompt（decomp-analyst / spec-writer / porter / verifier）；
- 任何 prompt 的产出必须包含：证据引用、完成判据（跑哪个门禁）、遗留假设清单；
- **产出的归宿是仓库**（台账/spec/代码/知识库），不是聊天记录——聊天里的结论不许作为后续依据；
- 全自动回路中任务卡由完备性矩阵派生（[docs/ai-decompile-pipeline.md](docs/ai-decompile-pipeline.md) §4）。

## 8. 知识库

`docs/knowledge/`：跨会话共享知识（6 类：architecture/gotcha/pattern/debug/decision/reference）。
遇到平台坑（Mesen2/IDA/WLA-DX/6502 banking/MMC1 写序）、门禁失效、被纠正的方法、确认有效的取舍时写入；
**引用前必须验证**文件/命令仍存在。

## 9. 提交规范

```
<type>(<scope>): <subject>          # type: feat/fix/docs/kb/gate/refactor/test/chore
                                    # scope: rom/ref/oracle/data/core/platform/gates/docs/tools
<空行>
<正文：改了什么、为什么、证据与影响>
<触及 data/naming/ / data/status/ / reference/ / rom/ 时必须含：>
Evidence: <A/B 级证据描述>
Confidence: <0-100>
```

小步提交：一个 commit 一个逻辑变更。门禁改动单独成 commit，不得与功能变更混提。
安全网：本地 `pre-commit`（gates check）+ `commit-msg`（证据门禁）实时拦截；CI 在 push/PR 时跑同等检查。
