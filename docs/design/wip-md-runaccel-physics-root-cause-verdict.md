# WIP 定位层修复根因 verdict：L2 账外 1 案（l2-454-reorg-unit-01）

> 2026-10-22 · 四项核（对抗复核核（证伪立场（独立重导证据（正确性核）））·
> Evidence A 级（L2 差分测试输出原文）Confidence 95

## 【2026-10-22 更正】归因更正（证伪更正文化·原地记录）

前序归因「md_runaccel_physics L2 账外 1 案」**误**——md_runaccel_physics 早已账内登记
（SC-MD_RUNACCEL_PHYSICS-IMPL，2026-10-20）；gate detail 文本邻段（rustc never-used
warning `fn md_runaccel_physics`）致误提。独立重导（工具匹配径实证）：colon-FAIL 745 案，
账内 744，**真账外=l2-454-reorg-unit-01**（`454_reorg_unit_01`，entry bank05:9A3E，
454 孤儿重索单元族（l4-readiness-batch.md:94 孤儿不参与编译教训））。

## 四项核（AGENTS §4 定位层四问）

| # | 问 | verdict |
|---|---|---|
| ① | 问题在哪一层？ | **L2 子程序级差分不一致（建模缺口（非本 lane 改动））**——真分歧=l2-454-reorg-unit-01（454 孤儿重索单元族（归宿缺失=port-map registry 双射缺口）） |
| ② | 消费者读哪份数据？ | oracle/subroutines/l2_cases.jsonl（id=l2-454-reorg-unit-01，subroutine=454_reorg_unit_01，bank05:9A3E）+game-core l2ports.rs——diff_subroutines.py 三方裁定径 |
| ③ | 证据级别？ | **A 级**（diff_subroutines 输出：colon-FAIL 745/账内 744/账外 1=l2-454-reorg-unit-01；工具匹配径模拟实证） |
| ④ | 修复根因？ | **不调阈值不加白名单**——gap_map 预登记制声明：`SC-L2-454-REORG-UNIT-01`（266 条）——exit=PENDING（路线图：454 重索单元归宿落（port-map registry 双射（语义 fn/unported! 桩爆 bank:addr））→L2 重跑 diff 全等→账清转 strict） |

附：SC-MD-RUNACCEL-BRK-L3ROUTE（BRK @ $0001 走 L3 路由类声明）保留为路线图注记
（md_runaccel_physics 本体已在账内 SC-MD_RUNACCEL_PHYSICS-IMPL）。

Evidence: tools/diff_subroutines.py 输出实证（745/744/1）+ l2_cases.jsonl（bank05:9A3E）+
data/status/l2_gap_map.json（SC-L2-454-REORG-UNIT-01 登记）——A 级原文。
Confidence: 95
