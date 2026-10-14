# 基于 AI 的全自动反编译管线（Metroid 版）

> 设计目标：把"ROM → 语义化规格 → Rust"的全过程变成**可自动迭代的回路**，
> AI 干活、门禁裁判、人只看门禁红灯和白名单。
> 纪律与红线以 [AGENTS.md](../AGENTS.md) 为准；本文只讲管线结构。
> 管线结构移植自 gunnac_remake（NES/6502 同构）。

## 1. 与 xiyou_remake / gunnac_remake 的关键差异（先想清楚再抄）

| 维度 | 前作 | 本项目（Metroid, NES 6502） | 应对 |
|---|---|---|---|
| 反编译可信度 | gunnac：无社区反汇编，投影全部自建 | **有社区匹配反汇编 m1disasm**（WLA-DX 构建与 ROM 逐字节等价，已验证） | 投影成本大降；但第三方命名是别人的语义结论，**照抄 = 未取证改名**（AGENTS.md §3.7），语义仍须自证落台账 |
| 参考版可运行 | 同 | 反汇编/伪代码不能直接跑 | oracle 用**原 ROM 跑在 Mesen2**；另建 **L2 子程序级差分**（见 §5） |
| 信息形态 | 6502 平铺内存 + bank 切换 + 数据/代码混排 | 同 + **MMC1 显式写序切 bank**（向 `$8000-$FFFF` 串行写 5 位）+ **CHR-RAM**（图案表动态上传） | S1 先建 bank 图与 PPU 上传 trace；数据表考证引 m1disasm `constants_ram.asm` 作线索 |
| 难点分布 | 反编译本身是主成本 | 反汇编本体已被社区解决，**主成本转到语义考证 + 行为规格 + 差分验证** | 管线重心在 S3–S6；完备性矩阵盯"还剩多少没语义化/没搬运" |
| 版本 | 汉化单版本 | NES_NTSC 单目标（D6），多官方变体存在 | 版本差异单列 findings（R4） |

## 2. 真值层级（所有角色共用的心智模型）

```
第 0 层 真相    rom/ 的字节 + Mesen2 执行行为（oracle）        ← 永不修改，门禁校验 sha256
第 1 层 投影    m1disasm（已证等价）/ IDA listing / Ghidra 伪代码 / trace 导出
                                            ← 可再生，冻结副本进 reference/seed/（第三方命名在旁路线）
第 2 层 语义    命名台账 + docs/spec/ 行为规则                   ← AI 再创作，必须带证据指针
第 3 层 实现    crates/ Rust                                    ← 由差分测试裁定等价性
```

冲突裁决永远向下：spec 与 trace 冲突 ⇒ spec 错；Rust 与 oracle 冲突 ⇒ Rust 错；
投影与 ROM 冲突 ⇒ 投影错（重生成）。

## 3. 阶段（S0–S6）

| 阶段 | 内容 | 主要工具 | 产物 | 阶段门禁 |
|---|---|---|---|---|
| **S0 ROM 取证** | iNES 头、MMC1 bank 图、向量表（NMI=$C0D9/RESET=$FFB0/IRQ=$FFB0）、m1disasm 等价性验证 | python + hexdump + WLA-DX | [docs/findings/rom-disasm-equivalence.md](findings/rom-disasm-equivalence.md)（✔ 已完成）、`docs/findings/rom-forensics.md` | rom-integrity 建立 ✔ |
| **S1 静态投影** | m1disasm 对照 + IDA（`m65`）建库 + Ghidra 出伪代码；bank 窗口/跳转表/数据表边界标注 | WLA-DX v10.8a、IDA 9.1 + ida_mcp、Ghidra | `reference/seed/metroid.asm`（冻结）、`analysis/ida/`、`analysis/ghidra/` | seed-integrity 建立 |
| **S2 动态证据** | Mesen2 Lua harness：输入回放、trace、逐帧 RAM dump、截图、CHR-RAM 上传记录 | Mesen2 2.1.1 | `oracle/scenarios/*.txt`、`analysis/traces/` | 场景可复现 |
| **S3 符号与数据表恢复** | LLM 逐子程序考证语义命名；数据表（敌人属性/门/房间表/密码表）识别与字段切分 | decomp-analyst 角色 + 台账 | `data/naming/`、`docs/findings/` | naming-ledger |
| **S4 规格书** | 行为规则提炼：移动/滚球/射击/战斗/门与房间切换/敌人 AI/密码系统/Boss | spec-writer 角色 | `docs/spec/`、`constants.md` | docs-links/format |
| **S5 Rust 移植** | game-data 解析 → game-core 纯逻辑（60Hz 固定 tick）→ 平台壳 | porter 角色 | `crates/` | rust-tests、L1–L3 |
| **S6 差分验证** | 子程序级 + 帧级 + 视觉级差分；完备性收敛 | verifier 角色 + oracle | `data/status/baselines/`、bug 台账 | trace/visual/all |

## 4. 全自动回路（"AI 全自动"的具体形态）

```
        ┌──────────────────────────────────────────────────────┐
        │ 完备性矩阵（S/D 面分母机械导出；B/R 面声明性）            │
        │  符号面: 子程序全集  数据面: 表+xref                     │
        │  行为面: 场景/系统清单           规则面: spec 条目覆盖    │
        └──────────────┬───────────────────────────────────────┘
                       ▼ 每轮取"未覆盖且依赖已清"的一格
        ┌──────────────────────────────────────────────────────┐
        │ 任务卡队列（prompts/README.md 模板）                    │
        │  decomp-analyst → spec-writer → porter → verifier     │
        │  （角色任务可并行，写路径互斥；依赖用任务依赖表达）        │
        └──────────────┬───────────────────────────────────────┘
                       ▼ 产出落仓库（台账/spec/代码），不落聊天
        ┌──────────────────────────────────────────────────────┐
        │ 门禁裁定（gates/cli.py）                                │
        │  绿 → 矩阵标记覆盖（带证据指针），进入下一轮              │
        │  红 → 四分类路由：移植错→porter / 规格缺口→spec-writer / │
        │       参考投影错→decomp-analyst / 平台差异→白名单(需人确认)│
        └──────────────────────────────────────────────────────┘
```

- **人只出现在三个位置**：白名单确认、门禁恒红的裁决、里程碑验收。
- **防钝化三约束**：新用例准入要声明覆盖面；覆盖只增不减；每轮报告给出"下一最优目标"。
- **绿 ≠ 完整**：测试全绿只说明"声明过的问题没回归"；还剩多少没移植看矩阵，不看测试。

### 4.1 完备性分母（四面）——缺口左移机制①③

对拍是**弱完备性探测器、强正确性裁判**：其覆盖上限=语料覆盖上限。因此「全覆盖」不靠红灯反推，
靠分母可见性左移——其中**独立于测试的机械分母仅限 S 符号面 / D 数据面**
（`tools/completeness_denoms.py`）；B/R 面为声明性分母（见表）：

| 面 | 分母来源（机械） | 分子登记（带证据，只升不降） | 性质 |
|---|---|---|---|
| **S 符号面** | `analysis/ida/*.json` funcs（有限清单；标注"待 trace 纠偏"） | 格状态机：`未实现 → 已实现 → 有 L2 用例 → 已验证` | **机械分母** |
| **D 数据面** | data items 连续段 + xref 消费者 | 字段切分 + 解析实现 + golden | **机械分母**（字段级分母待 S3，rounds/r2 F3a） |
| **B 行为面** | `oracle/scenarios/*.txt` **已登记场景清单** | 场景已对拍 | **声明性**：行为空间无机械分母（分母=语料清单本身） |
| **R 规则面** | `docs/spec/` 规则条目（S4 起） | 规则已实现 + 已验证 | **声明性**：规则集完备性不可证（没写成规则的行为=没有格） |

原则：**分母只登记 L0（入册未验证）**；`completeness_cells` 门禁度量的是已覆盖格（ratchet 只升不降）。

**未覆盖缺口类（显式声明，rounds/r2 F3）**——以下类别不进四面分母，仍可能以晚期红灯曝光，
处置靠四分类路由兜底（不冒充"已覆盖"）：
- **D 面字段切分**：连续段粒度 ≠ 字段粒度；字段错将以 L3 漂移晚期暴露（S3 出字段级分母后收编）；
- **跨模块交互**：调用序 / 共享 RAM 契约 / NMI-主循环同步（拟由 R 面在 S4 前置登记）；
- **时序边界**：帧相位 / NMI 时序 / quirk 时序（L3/L4 diff 兜底）；
- **R 面未写规则**：规则集之外的行为不可见（写作覆盖率永远不可证）。

**oracle 覆盖的语义警示（rounds/r2 F1，实测证伪）**：`addMemoryCallback("execute")` 回调集合
**≠ 已执行地址集**——实测漏记确证执行过的地址（NMI 入口 `$C0D9`、复位例程 `$FFB0-$FFC0`），
且混入非指令入口（JSR 操作数字节）。因此：
- `tools/coverage_from_trace.py` 内置 fail-loud 自校验（`captures cpu.pc ⊆ 覆盖集`），
  不过校验即降级 `unverified-exec-callback` 并**拒绝产出盲区差集**；
- 语义验证（微探针 → 修正采集）完成前，覆盖仅作**未验证线索**，**禁止直接派生任务卡**；
- 自校验缺口教训与 `docs/knowledge/gotcha-observation-granularity-fabricates-invariants.md` 同族。

### 4.2 缺口暴露形态表——缺口左移机制②④

Rust 侧约定：**未实现例程必须打 fail-loud 桩，禁止空实现/静默默认值**：

```rust
// crates/game-core 内（S5 落地）；桩 panic 携带 bank:addr，缺口落台账对应矩阵格
macro_rules! unported { ($loc:expr) => {{ $crate::gap::record($loc); panic!("unported @ {}", $loc) }} }
```

对拍红灯按**首分歧点形态**分类路由（缺口处置成本 = 一张任务卡，不进调试循环）：

| 失败形态 | 分类 | 路由 |
|---|---|---|
| panic @ `unported!(bank:addr)` | **实现缺口** | spec-writer 补规则 → porter 排期（矩阵格记 L0） |
| L3 状态向量漂移（已移植逻辑内静默分歧） | 移植错 | porter |
| 投影与 ROM 行为矛盾 | 投影错误 | decomp-analyst |
| 原版 quirk 重现 | 参考 bug | 记 `BUG-xxx`（Stage A 保真，TOLERATED） |
| 平台/渲染差异 | 平台差异 | 白名单（唯一合法例外，需用户确认） |

## 5. 差分金字塔（验证层，L2 是 NES 特有的强武器）

| 层 | 对象 | 方法 | 裁定物 |
|---|---|---|---|
| L1 | 资产解析 | CHR-RAM 上传内容/调色板/房间表 golden test（解析产物 vs 手工核对基准） | `data/golden/` |
| **L2** | **子程序级** | **6502 CPU 核 harness 直接执行 ROM 原字节**：给定入参 RAM/寄存器快照，跑单个子程序到 RTS，比对出参/内存写集 vs Rust 同名函数 | `oracle/subroutines/` |
| L3 | 帧级 | 同输入序列 + 固定 RNG，逐 tick 比对状态向量（Samus 坐标/弹数/敌槽位/门状态） | oracle trace vs game-core |
| L4 | 视觉级 | 截图逐像素（调色板映射白名单外） | `emu.takeScreenshot` vs Rust 帧缓冲 |
| L5 | 流程级 | 全流程自动跑（通关/死亡/密码读档/房间切换） | 场景脚本全绿 |

L2 的价值：把"一个函数是否移植正确"从"整局游戏跑出来才知道"降为**毫秒级单测**，
这是 6502 无 MMU、子程序边界清晰带来的独特红利。
注意隔离：子程序里若有 PPU/APU/MMC1 寄存器访问，harness 需 stub 这些地址的读写行为
（MMC1 写序副作用建模到 bank 状态即可）。

## 6. 工具映射（本机已验证状态）

| 能力 | 工具 | 状态 |
|---|---|---|
| 匹配反汇编（第四方投影） | **metroidret/m1disasm** + **WLA-DX v10.8a**（源码自建） | ✅ 已验证：构建产物与 ROM 逐字节等价（docs/findings/rom-disasm-equivalence.md） |
| 6502 反汇编 | IDA 9.1 `m65.so`（MOS 6502/65C02）+ **ida_mcp**（AI 直接驱动 IDA） | ✅ 本机 `/home/peng/ida-pro-9.1` |
| 6502 伪代码 | **Ghidra analyzeHeadless**（SLEIGH 反编译器） | 待装（Hex-Rays 无 6502，别指望） |
| 动态 trace / 输入注入 / 截图 / 内存回调 | **Mesen2 2.1.1 Lua**（gunnac 已跑通同款 harness） | 待迁移 |
| 子程序级执行 | 自研 6502 CPU 核 harness | 待建 |
| 资产抽取 | CHR-RAM 上传 trace、调色板/nametable 导出、FamiTracker（sound engine 后置） | 待建 |
| 文档 | NESdev Wiki（**MMC1**/PPU/APU）、Disch/Anomie docs | — |
| 通用二进制逆向 | REA（实测**不支持** `.nes` 格式，仅作证据登记外围，见 findings） | 不进主链路 |

## 7. 命名台账 schema（两层，移植自 xiyou）

- **机械层** `data/naming/remap-table.json`：确定性重映射（`sub_9A2C`→`f_042`、`unk_0320`→`v_0320`），
  不做语义判断，由 `tools/remap/run.py` 生成与校验。
- **语义层** `data/naming/ledger.jsonl`：一条 rename 一条记录：

```json
{
  "id": "N-XXXX", "kind": "subroutine|ram|table|const",
  "old": "<原始机械名>", "ref": "<机械层条目 id>",
  "new": "<语义名>",
  "evidence": "<可复现指针：bank:addr / 字节偏移 / trace id / 命令>",
  "confidence": 0, "level": "A|B", "date": "YYYY-MM-DD", "status": "applied|hypothesis|reverted"
}
```

> ⚠️ **以上为格式示例，字段值全部是虚构占位**，不代表任何真实符号的考证结论。

规则同前作：A/B 级才能 `applied`；C 级一律 `hypothesis`（代码用 `_todo_` 前缀）；
`evidence` 必须含可复现指针；证伪追加 `reverted` 不删历史；**第三方名（m1disasm 等）采用时必须自证**（§3.7）。

## 8. 风险登记

| ID | 风险 | 对策 |
|---|---|---|
| R1 | MMC1 bank 切换（串行写序）+ CHR-RAM 动态上传 | S1 建 bank 图 + PPU 上传 trace；spec 与 bank 实现解耦；Rust 侧复刻行为不复刻硬件 |
| R2 | 数据/代码混排，自动反汇编有噪声 | m1disasm + IDA/Ghidra 多投影交叉 + trace 确认执行过该地址 |
| R3 | L2 harness 中 PPU/APU/MMC1 依赖 | stub 寄存器行为；有副作用的子程序降级到 L3 验证 |
| R4 | 版本变体行为混入（PAL/FDS/MZM/CNS） | 只认 NES_NTSC（D6）；版本差异单列 findings，不混入"原版行为"结论 |
| R5 | 音乐（sound engine 89KB @ bank7）保真成本 | Stage A 软件 APU 原样渲染或静音占位（记 scope-decision）；Stage B 重制 |
| R6 | "AI 全自动"产出幻觉（编造行为） | 证据分级纪律 + 门禁裁定 + 完备性矩阵防"绿=完整"错觉 |
| R7 | **第三方命名污染**（m1disasm/MetroidMMC3 符号直接流入语义结论） | D5/§3.7：照抄 = 未取证改名；台账必填自证 evidence；命名核查列对抗复核清单 |
