# master-plan — Metroid（NES/6502）Rust 重制总规划

> 纪律以 [AGENTS.md](../AGENTS.md) 为准；管线以 [ai-decompile-pipeline.md](ai-decompile-pipeline.md) 为准。
> 本文只排阶段与优先级，随进度滚动更新。

## 阶段状态（2026-10-14）

| 阶段 | 内容 | 状态 |
|---|---|---|
| S0 ROM 取证 | ROM 校验、m1disasm 等价性验证、向量/复位抽检 | ✔ 完成（findings/rom-disasm-equivalence.md） |
| S0.5 纪律骨架 | AGENTS.md / prompts / gates / 数据模板迁移（gunnac_remake → 本仓库） | ✔ 完成 |
| S1 静态投影 | IDA 8-bank export → seed 归一化冻结 | ✔ 完成（c416d34；**对抗复核 pass**，rounds/r1） |
| S2 动态证据 | Mesen2 oracle 迁移；冒烟✔（标题/开局/Samus 三帧）；首场景（reset → 标题）trace | 🔨 进行中 |
| S3 语义考证 | 命名台账 + 数据表识别（constants_ram.asm 线索核）；**待办（rounds/r1）**：seed 区域角色标签证据化重措辞、items/funcs/xrefs trace 纠偏流程 | ⬜ |
| S4 规格书 | 首切片：Samus 移动/跳跃/滚球 | ⬜ |
| S5 Rust 移植 | crates/game-core 骨架 + 首切片 | ⬜ |
| S6 差分验证 | L2/L3 门禁点亮 | ⬜ |

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
