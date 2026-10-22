# Phase 0 架构设计：改写策略 + 验证方案

> 2026-10-22 · 用户裁定（认可（按计划推进））· 纪律强化（refs 引用必写+更新留档可追溯）·
> 四项核（对抗复核核（证伪立场（独立重导证据）））· Evidence A 级（bank:addr 引证）Confidence 95

## 0. 改写单元识别（refs 实证）

**规则：一条 asm 例程含全部标签 → 一个组合方法（内部标签不外露）**。

实证（refs/m1disasm/SRC/prg7_engine.asm）：
- `SamusJump: ; 07:D002`（:3467）一条例程内含 `Lx019 ; 07:D015`、`Lx021 ; 07:D03C`、
  `Lx023 ; 07:D050`、`SamusJump_CheckHorzMovement ; 07:D055`（:3480-3516）——内部标签/内嵌
  子入口全部封装进一个 `samus_jump()` 方法，Lx0xx 标签不进 Rust 公开面（仅行内注释留档）。
- 同族例程（prg7_engine.asm）：`SamusRun ; 07:CCC2`（:2858）、`SamusPntUp ; 07:D198`（:3736）、
  `SamusDoor ; 07:D3A8`（:4092）、`SamusDead ; 07:D41A`（:4157）、`SamusElevator ; 07:D423`（:4168）。
- jsr 子入口（`SamusJump_CheckHorzMovement`/`SamusJump_CheckFire`/`SetSamusJump`，:3481/3491/3497）
  若被其他例程共享 → 独立方法登记 registry；仅单例程内用 → 组合进宿主方法。

## 1. 改写策略：六步流程（每行为簇）

| 步 | 内容 | 产出/判据 |
|---|---|---|
| 1 定域取证 spec | refs asm 逐行取证（bank:addr 引证）+D3 ROM/Mesen2 实证 | spec 文档（A 级引证齐） |
| 2 域结构 | Samus/敌/OAM 域结构建模（字段=RAM 映射名） | crates/game-core 域类型 |
| 3 memmap 适配 | bank 切换语义→Rust 侧数据表直读（bank7 固定 $C000 不切；$8000 槽按 bank 号索引） | rom_tables 直读径 |
| 4 shim 写集适配 | l2ports shim 写集（out.writes）与新域结构字段对齐 | shim 适配 commit |
| 5 测试 | L2 逐函数 + L3 场景 + L4 像素（§2 三层缺一不可） | 实测 diff 全绿输出原文 |
| 6 registry 登记+独立复审 | port-map registry v0 登记 + verifier 独立复审（证伪立场） | registry 行+复审 verdict=pass |

## 2. 验证方案（L2/L3/L4 三层·缺一不可）

| 层 | 判据 | 角色 |
|---|---|---|
| L2 函数级 | golden==ROM 执行==game-core（tools/diff_subroutines.py 三方裁定） | **降级安全网**（L3/L4 不可用时兜底，不替代上层） |
| L3 模块级场景对拍 | RAM 快照 diff（全 RAM/零页逐字节，白名单外全等） | 行为簇级等价 |
| L4 像素 diff | framebuffer 逐像素 diff（白名单外全等） | **最终判据** |
| S 面×registry 双射 | S 符号面每符号 ↔ registry 每行一一对应（无遗漏无多余） | 完备性收口（tools/port_coverage.py 机械核） |

## 3. port-map registry v0（tools/port_map_registry.jsonl 初版行形）

每行：`{symbol, bank, addr, rust_module, rust_fn, cluster, l2_case_ids, l3_scenarios, l4_scenarios, status}`
- status：`未实现→已实现→有L2→已验证`（D7 勾格清单左移四态）
- 双射核：`tools/port_coverage.py` 比对 S 面分母（completeness_denoms.py）×registry 行集

## 4. 零页映射二分表（fields→Rust 域字段）

实证锚（refs/m1disasm/SRC/constants_ram.asm）：
- `SamusJumpDsplcmnt = Samus.data0F ;$030F`（:869）——跳位移（垂直位移像素数）
- `Samus.speedY`（:226，$0308 注）——y 速度（1/16 像素/帧，MSB=上行 $FA max/下行 $05 max）
- 表读径算例：`lda #ObjAnim_SamusJumpPntUp - ObjectAnimIndexTbl.b`（prg7_engine.asm:3486）
  ——表索引差值算术（动画索引=表基址差，Rust 侧直读表值不做差算）

二分表形：`zero_page_map.jsonl` 行 `{nes_addr, symbol, rust_field, cluster, verified}`——
逐行 refs 引证（constants_ram.asm 行号），未证字段留 `_todo_`（伪语义命名禁用纪律）。

## 5. 帧预算记账方案

- 帧循环=60Hz tick（虚拟时钟）；每 tick 记账：`{frame, nmi_writes, mainloop_writes, oam_dma_bytes}`
- NMI 侧写点与主循环侧写点分账（a3-028 洞类教训：捕获窗必须覆盖主循环写段）
- 记账差超阈 ⇒ L3 diff FAIL（帧预算回归）；基线只升不降（data/status/baselines/ ratchet）

## 6. 光栅器裁定项（L4 像素 diff 白名单候选——需用户确认）

| 项 | 处置 | 依据 |
|---|---|---|
| 字体/调色板映射 | 白名单（唯一允许例外类） | AGENTS §5 视觉白名单 |
| 扫描线抖动/8 sprites 限制 | **保真保留**（D4 quirks，禁修） | bug-ledger 登记 |
| CHR-RAM 图案表写时序 | 对拍窗对齐后全等（不进白名单） | 帧相位一致前提 |

## 7. 场景分层清单 v0（L3/L4 场景矩阵骨架）

| 层 | 场景簇 | 依赖数据表（D 面） |
|---|---|---|
| S1 移动 | 跑/跳/蹲/瞄准/开火 | 跳跃/变球表（Samus 域，D 面考证件 113-126 系列） |
| S2 门/梯 | 门开关/电梯 | SamusDoor/SamusElevator 族 |
| S3 战斗 | 敌碰撞/伤害/无敌帧 | 敌状态表/OAM 链 |
| S4 变球 | 变球/炸弹 | 变球音效表（SFXData@SamusBall 00:b26d 等） |
| S5 通关 | Tourian→MotherBrain→escape→ChooseEnding | G4 集成场景（T45-3/5/6） |

## 8. refs 引证汇总（bank:addr）

prg7_engine.asm:2858（SamusRun 07:CCC2）/3467-3516（SamusJump 07:D002 含内标签）/
3736（SamusPntUp 07:D198）/4092（SamusDoor 07:D3A8）/4157（SamusDead 07:D41A）/
4168（SamusElevator 07:D423）；constants_ram.asm:226（speedY $0308）/869（SamusJumpDsplcmnt
$030F）；memory_layout.asm:11-18（$8000 切换槽+$C000 固定槽+8×16KB bank）。

Evidence: 上表 bank:addr 全量引证（refs/m1disasm/SRC/ 实测行号）。
Confidence: 95
