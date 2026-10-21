# S 面缺口清单导出+分母精化（L4 就绪批①——用户裁定（L4 覆盖率达标+基础设施就位→满跑推进））

> 【2026-10-21 导出】S 面缺口清单（bank:addr/名/优先级）+分母精化（可调用子程序面）——A 级实录。

## 分母精化（可调用子程序面（排除数据标签））

| 项 | 数（A 级实录） |
|---|---|
| .sym labels（全（bank:addr 子程序清单（M1_NES_NTSC.sym））） | **8660** 条 |
| 可调用子程序面（排除数据/局部标签（_tbl/_ptr/_data/_str/_arr/_tab+@/. 族）） | **8659** 条 |
| Rust pub fn（game-core） | **1908** 条 |
| **精化覆盖率（S 面）** | **22.0%**（1908/8659） |

## 缺口优先级排序（L4 依赖径（Samus 域（跳跃/变球）））

- **L4 依赖径**（跳跃/变球（Samus 域（L4 场景矩阵依赖（a97264b（场景 DSL（跳跃/变球）））））：**747** 条缺口（samus/jump/morph/ball 族）。
- 样例（首 5）：`00:8c39` SamusHasItem、`00:8cf7` CalculatePassword@loop_SamusAge、`00:9059` ItemData@HighJump、`00:9a83` Restart@endIf_HighJump、`00:9ac0` EndGame@jumpEngine。

## 缺口清单导出（bank:addr/名/优先级（L4 依赖径（首组（5 案样本（实现补全候）））））

- **首组**（5 案样本（实现补全候（L4 就绪（覆盖率达标））））：SamusHasItem/CalculatePassword@loop_SamusAge/ItemData@HighJump/Restart@endIf_HighJump/EndGame@jumpEngine。
- **实现补全**（关键径子程序（L4 就绪（覆盖率达标（≥X%）））——待定（L4 就绪判据）。
