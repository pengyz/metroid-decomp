# rounds/r2 — 完备性缺口左移机制 · 独立对抗复核（2026-10-14）

- 被审对象: `96c0926` feat(tools): 完备性缺口左移机制落地（D7）+ 试运行验证
- 审核方: 独立审核员（verifier，证伪立场，只读，数字全部自行重导）
- **verdict: needs_revision** → 本文件记录整改与根因证据，复审见 r2-fix

## 裁定摘要

试运行数字全部复现且诚实（S=450/D=193/B=1、7780/303/7477/211、幂等 +0、gates 0）；
**机制③核心推断被证伪**：coverage 集合 ≠「执行过的地址」；B/R 面「分母独立于测试」为过度声明。

| # | 分类 | 内容 | 整改 |
|---|---|---|---|
| F1（严重） | 实现缺陷 | string `"execute"` 回调集合 ≠ 执行地址集：漏记确证执行过的 NMI 入口 `$C0D9`、reset `$FFB0-$FFC0` 整段；混入非指令入口（JSR 操作数 pos2×54、立即数 pos1×38）；缺 `cpu.pc ⊆ 集合` 自校验 | ✅ containment：`coverage_from_trace.py` fail-loud 自校验（boot-title 实测 1/1 违例→降级 `unverified-exec-callback`+拒绝盲区差集+并集不吸收）；根因见下 §根因微探针；**指令级 oracle 覆盖在已验证 Mesen2 Lua API 内不可得** → 显式列为 S2 待研（CDL/trace logger/FCEUX trace） |
| F2（中） | 实现缺陷 | Xvfb 创建在 try/finally 外；清场 pgrep 只杀 Mesen | ✅ 已修：Xvfb/Mesen 创建全收进 try，finally 全程 None-guard；清场加 Xvfb 特征模式（附带风险已注释声明） |
| F3（中） | 方案缺口 | 「分母独立于测试」仅 S/D 成立；B/R 过度声明；D 字段切分/跨模块交互/时序边界/R 未写规则 未左移 | ✅ D7/§4.1 收窄措辞 + 四类缺口显式声明；D 字段级分母挂 S3；跨模块/时序拟由 R 面 S4 前置登记 |
| F4 | 纪律 | 无违规（Evidence/Confidence 真实；completeness_cells 口径未误抬；rom/ 未动） | —（保留意见：Confidence 未覆盖"coverage=执行地址"前提——已由本次根因证据补证伪） |
| F5 | 工程卫生 | "DONE 后挂起 10 分钟"未复现（run.json wall=16.2s）；knowledge 迁移条目 4 处指针指向 gunnac 仓库 | ✅ knowledge README 加来源声明；挂起待下次复现用 py-spy/strace 取证再入 gotcha |

## 根因微探针（tools/probe_exec_callback.lua，A 级证据）

同帧双路注册 + 全程采样 cpu.pc，实跑产物 `probe_meta.jsonl` **frames=7271**（`emu.stop()` 未止住
脚本持续计数——与 F5 挂起现象同族、未根因化；初稿叙述"2875 帧"系脚本设计停止帧，2026-10-14 更正，
落库数字以产物为准）：

| 接口 | 实测 | 结论 |
|---|---|---|
| `addMemoryCallback(fn, "execute", $8000-$FFFF)` | 注册成功；集合 8513 地址；**`$C0D9`（每帧确证执行）∉ 集**、`$FFB0`∉ 集、`$C0DA`∈ 集；含 JSR 操作数第 3 字节 | 语义 ≈ 总线读周期（含操作数/数据读、缺中断入口首指令），**不是指令覆盖** |
| `emu.memCallbackType.execute` 枚举 | **`emu.memCallbackType` = nil**（regB=false） | 2.1.1 无此枚举；执行器原 fallback 路径实为死代码 |
| `emu.getState` cpu.pc 采样（inputPolled+endFrame 各 1 次/帧 ×7271） | **distinct=1，恒为 `$C0D9`** | 回调上下文中 getState 恒报 NMI 入口——帧采样覆盖路线不可用 |

→ 机制③降级定位：oracle 覆盖 = **未验证线索**（自校验通过前禁止派生任务卡）；行为面分母维持
场景清单（声明性）。根治方向（显式待研，master-plan S2）：Mesen2 Trace Logger / CDL 文件接口、
FCEUX trace、或以 L2 子程序级执行覆盖率替代（精确但以 S 面为界）。

## 复现命令（摘录）

```
python3 tools/coverage_from_trace.py boot-title
# 自校验 FAIL: 1/1 个确证执行过的 pc ∉ 覆盖集（如 C0D9）→ 降级 unverified，拒绝盲区差集（exit 1）
python3 tools/probe_exec_callback.lua 重跑 → probeA.jsonl 8513 addr / probePC.jsonl {C0D9} / probe_meta {regA:true,regB:false,enum:"nil"}
```
