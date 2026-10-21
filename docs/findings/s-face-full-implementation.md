# S 面缺口全实现补全（L4 就绪批①——用户裁定（S 面=100%（全覆盖（零缺口））））

> 【2026-10-21 实施】S 面缺口全实现补全（首组（Samus 域（跳跃/变球（L4 依赖径）））——A 级实录。

## 精化覆盖率（A 级实录）

| 项 | 数 |
|---|---|
| 可调用子程序面（精化分母（排除数据/局部标签）） | **8659** 条 |
| Rust pub fn（game-core） | **1908** 条 |
| **精化覆盖率（S 面）** | **22.0%**（1908/8659） |
| **目标**（用户裁定（S 面=100%（全覆盖（零缺口）））） | **100%**（8659/8659） |

## 缺口清单导出（bank:addr/名/优先级（L4 依赖径（首组）））

- **首组**（Samus 域（跳跃/变球（L4 场景矩阵依赖（a97264b（场景 DSL（跳跃/变球）））））——5 案（实现补全候））：
  1. `00:8c39` SamusHasItem
  2. `00:8cf7` CalculatePassword@loop_SamusAge
  3. `00:9059` ItemData@HighJump
  4. `00:9a83` Restart@endIf_HighJump
  5. `00:9ac0` EndGame@jumpEngine

## 实现补全（逐组实现（refs asm 考证→Rust 实现→L2 并案））

- **径**（refs asm 考证（bank:addr（写径实证（真规则（公式形））））→Rust 实现（game-core（Samus 域（engine/enemy/samus）））→L2 并案（可选（fixture 级回归）））。
- **首组实现补全**（5 案（Samus 域（跳跃/变球））——**待实现**（refs asm 考证（bank:addr（写径实证））→Rust 实现））。
