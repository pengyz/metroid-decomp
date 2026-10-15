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

- [ ] **门禁 FAIL 归属线索**（port2 提议，低优先）：FAIL 文案打印引入红的文件/diff 归属
  （现排查靠人肉 git status 对圈）——共享树在途互踩族（三次事故）的排查成本优化；

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

## 里程碑 B（2026-10-15 用户裁定——集成期转向）

**主里程碑口径**：L3 双腿帧级对拍在 **boot-title + samus 移动 demo** 两场景通过
（含渲染 PPU 写入面；对齐「移动先行」原排序）。从分母序批量端口**转向执行序集成**：
帧引擎先跑，unported! 桩阵按真实执行序引爆缺口，每引爆一个端一个（L2 管线降为
组件库按需补给，证明机/三方/突变全保留）。batch-2（game-data 数据表）提前为帧引擎供料。
排期：task-10 帧引擎+boots-to-title → task-11 samus 移动 demo；task-12 执行路径调用图
（喂执行序队列）；task-13 L3 game-core 腿验证设计。bank7 剩余 176 入口=执行序触发积压池。

## 里程碑 C 蓝图（2026-10-15 asm 架构考古产出——B 后启用）

域划分照抄游戏自身组织（m1disasm 结构=游戏的模块说明书）：产品层域树
core/frame（prg7）· world/{brinstar,norfair,tourian,kraid,ridley}（prg1-5）·
gfx（prg6）· title（prg0）· audio（sound_engine）。B 后从「按函数爬」转「按域收割」：
world 垂直（room_ptrs→metatiles→CHR→PPU 四级数据链）/ enemy 垂直（敌槽+分发 handler）/
audio 垂直——每域=L3 场景+两层形态域模块。
- **batch-2 蓝图**：关卡=数据四级引用链（room_ptrs/metatiles/palettes/items/
  enemy_sprite_data 五件套/区域 bank 同构）——schema 已知，game-data 直接按层级建模；
- **状态显式化（贯穿工作流）**：constants_ram 286 zp 名=typed view 候选矿，
  memmap 随消费随显式化（命名台账+两层架构联动）；
- 表驱动分发器（C27C A 值域）→ enum+match；宽放 18 overframe=Stage A 保真面。

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
