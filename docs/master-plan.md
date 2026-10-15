# master-plan — Metroid（NES/6502）Rust 重制总规划

> 纪律以 [AGENTS.md](../AGENTS.md) 为准；管线以 [ai-decompile-pipeline.md](ai-decompile-pipeline.md) 为准。
> **长线目标与收敛回路以 [north-star.md](north-star.md) 为准**（稳定定义）；本文只排阶段与优先级，随进度滚动更新。

## 阶段状态（2026-10-14）

| 阶段 | 内容 | 状态 |
|---|---|---|
| S0 ROM 取证 | ROM 校验、m1disasm 等价性验证、向量/复位抽检 | ✔ 完成（findings/rom-disasm-equivalence.md） |
| S0.5 纪律骨架 | AGENTS.md / prompts / gates / 数据模板迁移（gunnac_remake → 本仓库） | ✔ 完成 |
| S1 静态投影 | IDA 8-bank export → seed 归一化冻结 | ✔ 完成（c416d34；**对抗复核 pass**，rounds/r1） |
| S2 动态证据 | **Gate-in G2 ✔（L2 harness，r3 复审 pass）**；Mesen2 oracle 迁移✔；冒烟✔；首场景 trace✔；**待研（rounds/r2 F1/N3）**：指令级覆盖语义根治——CDL / Mesen Trace Logger / FCEUX trace 三选一，达成前 oracle 覆盖=仅线索 | 🔨 进行中 |
| S3 语义考证 | 命名台账 + 数据表识别（constants_ram.asm 线索核）；**待办（rounds/r1）**：seed 区域角色标签证据化重措辞、items/funcs/xrefs trace 纠偏流程；**待办（rounds/r2 F3a）**：D 面字段级分母；**密码系统逆向**（密码域覆盖面考证 + state-fixture 降级触发器裁决点，north-star M2 收口前判定） | ⬜ |
| S4 规格书 | 首切片：Samus 移动/跳跃/滚球；**前置登记（rounds/r2 F3b/c）**：跨模块交互（调用序/共享 RAM 契约/NMI 同步）与时序边界 → R 面规则登记 | ⬜ |
| S5 Rust 移植 | crates/game-core 骨架 + 首切片 | ⬜ |
| S6 差分验证 | L2/L3 门禁点亮 | ⬜ |

## 仓库级待办（设计复审暴露，暂无归属阶段）

- [ ] **decode-sweep 工具 + 分母 v2**（seed-decode-gap 发现）：系统性扫描全部数据岛的
  ROM 原字节解码（找 JSR/JMP 目标落界/回环模式），修正 S/D 分母错面后出 v2 导出；

- [ ] **里程碑编号矛盾**：north-star.md（M3=核心系统/M4=全内容含密码）与 methodology.md
  （M3=含密码/M4=差分收口）M3/M4 定义不一致——统一口径后删除本行（state-fixture v3 L7）
- [ ] **covered_rule 同步**（state-fixture v6 B1/M1）：completeness_matrix.json covered_rule 现为单判据
  「covered⇔strength≥L1」，与 state-fixture §3(c)「B-SCN covered 以 invariant (c) 通过为前提」
  未接通——schema 升级或加注后删除本行
- [ ] **completeness provenance 悬空**：completeness_matrix.json provenance 写 tools/matrix_sync.py
  --export（该文件不存在；实际导出器=completeness_denoms.py）——修正指向后删除本行
- [x] ~~docs/spec/completeness.md 悬空引用~~（2026-10-14 已建 docs/spec/completeness.md，§0/§6.1 对齐 schema）

## 批量批次表（completeness.md §2，batch-0 起步）

| 批次 | 模块 | 状态 |
|---|---|---|
| batch-0 | 流水线自举：L2 case factory + 移动积分器切片 | 进行中 |
| batch-1 | 移动系统收尾（UpdateSamus/跳跃引擎） | 未排期 |
| batch-2 | D 面数据表解析（可与 batch-1 并行） | 未排期 |
| batch-3+ | 战斗/门/房间/HUD/音频/密码（按矩阵聚类续排） | 未排期 |

## 首切片（垂直打通）

reset（bank7:$FFB0）→ NMI/主循环 → 标题 → 进 Brinstar → Samus 移动/跳跃/滚球，
每步 L3 帧级差分对齐后进入下一块。

## 模块优先级（S4/S5 排序参考）

1. 引擎骨架：MMC1 bank 语义、NMI/主循环、房间加载、CHR-RAM 上传
2. Samus 物理与战斗（移动/跳跃/滚球/射击/受伤无敌）
3. 门与房间切换
4. 敌人 AI 通用框架 + Brinstar 敌人
5. 道具/能力获取、密码系统
6. Tourian / Boss / Mother Brain / 结局
7. 音频（sound engine，最后）

## 已锁定决策

见 AGENTS.md §0（D1 Rust / D2 两阶段 / D3 ROM+Mesen2 真值 / D4 保真 quirks /
D5 四方交叉投影 / D6 NES_NTSC 单目标）。
