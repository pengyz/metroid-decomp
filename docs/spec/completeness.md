# completeness：完备性方案——批量管线的分母、格状态机与模块批次

> 状态：v1（2026-10-14）。本文档=matrix schema `source_of_truth` 指向物（悬空引用已消）。
> 章节号（§0/§6.1）与 `data/status/completeness_matrix.json` 引用一致。

## 0. 定义（schema 引用点）

- **分母**：独立于测试、机械可导出——仅 S 符号面（机械初值 450 格 + decode-gap 补录格，
  现 455：F140/F149/F162/F1B3/F1CB/F1E7 补录）。
  **v2 估计（decode-sweep 首轮，docs/findings/decode-sweep-report.md §6.2/§6.3）**：
  S 面另有 ~176 个有佐证入口待补录（r2 深挖上调 120→176，估计值非定值，附录 B+P2-F1 佐证四分类）⇒ v2 约 630；
  定值待 decode-sweep 深挖（task-6）+ denoms 纠偏落地（task-5③）。
  D 数据面 193 格分解：错面/数据确认/混合/待查逐 id 清单以 decode-sweep-r2-jump-tables.md §5 为唯一来源（矩阵注记 26 条错面+1 混合+1 基本确定+5 数据确认，计数不在此复述防二遍算术漂移）。
  （`tools/completeness_denoms.py` 从 seed 导出）。B 行为面分母=已登记场景清单（声明性）、
  R 规则面完备性不可证（声明性）——D7 口径不变。
- **covered ⇔ strength ≥ L1**（矩阵轴 L0-L4；与差分金字塔 L1-L5 是**两条轴**，不换算）。
- **对拍只裁「对不对」、不证「全不全」**（绿≠完整）——本文档管「怎么把全不全变成可跟踪的账」。

## 1. 格状态机（缺口左移机制①的执行面）

```
S/D 面格：  未实现 → 已实现(有 unported! 清零记录) → 有L2(用例登记) → 已验证(三方全等)
                                          ↘ 复核 pass 后矩阵登记 covered+strength+证据指针
B 面格：    场景登记 → oracle 跑通 → (S5 后) game-core 对拍 → covered
R 面格：    spec 规则登记 → 可判定性标注 → 用例/场景背书
```

状态跃迁必须带证据指针（用例 id / commit / trace sha）；ratchet 只升不降。

## 2. 模块批次切分（分母 → 批次）

S 面 450 格按 **bank × 系统** 聚类成模块批次（一张批次表=分母的有序切片）：

| 批次 | 模块 | 分母范围（示例） | 依赖 |
|---|---|---|---|
| **batch-0** | 流水线自举：L2 case factory + 积分器切片 | S-FUNC 移动积分器区（E3A5/E3F9 两切片）+ 已有 2 格 | 无 |
| batch-1 | 移动系统收尾（UpdateSamus 分发/跳跃引擎 C27C 表） | S-FUNC bank7 CC0D-D0xx | batch-0 |
| batch-2 | D 面数据表（rooms/items/enemies 表解析） | D-ROOM-* / D-ITEM-* 等 193 格字段切分 | 无（可并行）|
| batch-3 | 战斗（射击/导弹/敌人交互） | bank7 E5xx+ / bank3 | batch-1 |
| batch-4 | 门/房间切换 + HUD | bank7 D2xx / D8xx | batch-2 |
| batch-N | 音频引擎/密码/… | 按矩阵聚类续排 | — |

批次表由矩阵导出器维护（`completeness_denoms.py --batches`，待实现）；未进批次的格
= 显式未排期（不是遗漏）。

## 3. 每模块循环（批量管线六步的模块级实例）

```
① spec-writer   R-rules + constants（A/B 级证据指针）→ docs/spec/<module>.md
② porter        实现进 crates/game-core（unported! 逐个清零，TDD：case 先行）
③ case          L2 case factory 生成用例（ROM 腿=golden 自动生成，抽样人工复核考题）
                → oracle/subroutines/generated/<module>.jsonl（staging）
④ verify        三方裁定全绿后，用例并入活跃集（与实现同 commit）；矩阵格逐格登记
⑤ review        独立对抗复核（verifier 证伪立场）——verdict=pass 才算完成
⑥ commit        小步提交 + Evidence/Confidence + baseline 更新
```

**staging→活跃集规则**：未实现例程的用例不得入活跃集（防红灯挂门禁）；并入与实现同 commit。
**case factory 与「先写用例再实现」**：factory 生成=写用例（ROM 腿独立裁决移植，golden
自 ROM 产出非手推——考题抽样人工复核防 same-source，r3 遗留项）。

## 4. 验证层级与模块的适配

| 层 | 适用 | 备注 |
|---|---|---|
| L2 | 纯逻辑函数（积分器/查表/状态位） | factory 主力；停止窗口=子程序边界 |
| L3 | 帧级组合行为（移动系统整体） | oracle 腿已验；game-core 腿 S5 后启用 |
| L4 | 渲染模块 | visual-diff S6 建立 |
| L5 | 流程级（全场景跑通） | 批次收尾验收 |

PPU/APU/mapper 副作用例程不设 L2（pipeline R3）——直接 L3 语境。

## 5. 批量铺开的就绪判定（2026-10-14 评估）

**可以铺开**（判定依据）：L2/L3 裁判经 5+ 轮对抗硬化 pass（判定权可信）；spec 方法实证
（R-mv-3 A 级解码）；case factory 落地（batch-0）；矩阵分母机械可导出；门禁 9/11 点亮。

**已知缺口（显式声明，不阻断 batch-0/1/2）**：
- game-core 帧引擎（S5）未建 ⇒ L3 右腿暂缺——**L2 是批量期主裁判**，L3 在 S5 后补验；
- naming-ledger 未建 ⇒ 批量期命名以候选名+证据注暂存，正式 rename 待台账（闸门自启）；
- 状态夹具 step 0 未跑 ⇒ 能力级 B 面场景暂用输入链驱动；
- D 面字段切分分母（r2 F3a）待 S3——D 批次以段级解析先行，字段级后补。

## 6. 登记规则

### 6.1 covered 登记（schema 引用点）
一格计 covered 的充要条件：
1. strength ≥ L1（矩阵轴）且证据指针可解析（用例 id / commit / trace sha）；
2. S/D 面：三方裁定（golden==ROM==game-core）或等价差分证据；
3. B 面：场景清单登记 + oracle 跑通（+S5 后 L3 对拍）；R 面：spec 规则 + 可判定性标注；
4. **夹具场景另需 state-fixture §3(c) invariant 持续性后置通过**（B-SCN covered 前提，
   schema 同步挂 master-plan 待办）；
5. 独立对抗复核 verdict=pass（AGENTS §5）。**绿≠完整**：covered 只登记已验格，
   未排期格显式留在分母。

## 7. 度量与通报

- ratchet：completeness_cells（covered 数）只升不降；每批次收尾 baseline update 留痕。
- 批次收尾必报：新增 covered 格数 / 未覆盖缺口类（D7 四类）状态 / 首分歧路由统计。
