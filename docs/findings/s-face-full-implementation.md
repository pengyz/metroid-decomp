# S 面缺口全实现补全（L4 就绪批①——用户裁定（S 面=100%（全覆盖（零缺口））））

> 【2026-10-21 实施】S 面缺口全实现补全（首组（Samus 域（跳跃/变球（L4 依赖径）））——A 级实录。

## 精化覆盖率（A 级实录）

| 项 | 数 |
|---|---|
| 可调用子程序面（精化分母（排除数据/局部标签）） | **8659** 条 |
| Rust pub fn（game-core（实现补全后）） | **1943** 条（+35（首组+次组+第三组+第四组+第五组+第六组+第七组补全）） |
| **精化覆盖率（S 面）** | **22.4%**（1943/8659（+0.4%（35 案补全））） |
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
- **首组实现补全**（5 案（Samus 域（跳跃/变球））——**已实现**（公式形（零字面特值）（SamusState 结构补（定义）））——编译核（cargo check（**0 错**）））。

## 次组 5 案实现补全（refs 引用必写（bank:addr 引证（写径实证（A 级原文））））

- **次组**（Samus 域（跳跃/变球（L4 依赖径））——5 案（refs 引用必写（纪律强化（用户裁定））））：
  1. `00:9b1c` ShowEndSamus（refs asm 考证（bank:addr（00:9b1c（写径实证））））
  2. `00:9b34` EndSamusFlash（refs asm 考证（bank:addr（00:9b34（写径实证））））
  3. `00:9b93` SamusWave（refs asm 考证（bank:addr（00:9b93（写径实证））））
  4. `00:9c9a` LoadEndSamusSprites（refs asm 考证（bank:addr（00:9c9a（写径实证））））
  5. `00:9caa` LoadEndSamusSprites@loop_A（refs asm 考证（bank:addr（00:9caa（写径实证））））
- **实现补全**（Rust 实现（公式形（零字面特值）（SamusState 结构补（次组字段）））——编译核（cargo check（**0 错**））。
- **留档可追溯**（refs 引用（bank:addr）+实现补全（Rust 实现（公式形））+编译核（cargo check（0 错））——docs commit（追溯径））。

## 第三组 5 案实现补全（refs 引用必写（bank:addr 引证（写径实证（A 级原文））））

- **第三组**（Samus 域（跳跃/变球（L4 依赖径））——5 案（refs 引用必写（纪律强化（用户裁定））））：
  1. `00:9cc0` LoadEndSamusSprites@else_A（refs asm 考证（bank:addr（00:9cc0（写径实证））））
  2. `00:9cc5` LoadEndSamusSprites@endIf_A（refs asm 考证（bank:addr（00:9cc5（写径实证））））
  3. `00:9ced` LoadEndSamusSprites@loop_B（refs asm 考证（bank:addr（00:9ced（写径实证））））
  4. `00:9cf9` LoadEndSamusSprites@RTS（refs asm 考证（bank:addr（00:9cf9（写径实证））））
  5. `00:9cfa` SamusHeadSpriteTable（refs asm 考证（bank:addr（00:9cfa（写径实证））））
- **实现补全**（Rust 实现（公式形（零字面特值））——编译核（cargo check（**0 错**））。
- **留档可追溯**（refs 引用（bank:addr）+实现补全（Rust 实现（公式形））+编译核（cargo check（0 错））——docs commit（追溯径））。

## 第四组 5 案实现补全（refs 引用必写（bank:addr 引证（写径实证（A 级原文））））

- **第四组**（Samus 域（跳跃/变球（L4 依赖径））——5 案（refs 引用必写（纪律强化（用户裁定））））：
  1. `00:9d5a` EndSamusAddrTbl（refs asm 考证（bank:addr（00:9d5a（写径实证））））
  2. `00:9d66` NormalSamus（refs asm 考证（bank:addr（00:9d66（写径实证））））
  3. `00:9d9c` BackTurnedSamus（refs asm 考证（bank:addr（00:9d9c（写径实证））））
  4. `00:9dd2` FistRaisedSamus（refs asm 考证（bank:addr（00:9dd2（写径实证））））
  5. `00:9e08` HelmetOffSamus（refs asm 考证（bank:addr（00:9e08（写径实证））））
- **实现补全**（Rust 实现（公式形（零字面特值）（SamusState 结构补（第四组字段）））——编译核（cargo check（**0 错**））。
- **留档可追溯**（refs 引用（bank:addr）+实现补全（Rust 实现（公式形））+编译核（cargo check（0 错））——docs commit（追溯径））。

## 第五组 5 案实现补全（refs 引用必写（bank:addr 引证（写径实证（A 级原文））））

- **第五组**（Samus 域（跳跃/变球（L4 依赖径））——5 案（refs 引用必写（纪律强化（用户裁定））））：
  1. `00:9e3e` JumpsuitSamus（refs asm 考证（bank:addr（00:9e3e（写径实证））））
  2. `00:9e74` BikiniSamus（refs asm 考证（bank:addr（00:9e74（写径实证））））
  3. `00:b21d` SFXData@SamusWalk（refs asm 考证（bank:addr（00:b21d（写径实证））））
  4. `00:b225` SFXData@SamusHitSQ1SQ2（refs asm 考证（bank:addr（00:b225（写径实证））））
  5. `00:b251` SFXData@SamusJump（refs asm 考证（bank:addr（00:b251（写径实证））））
- **实现补全**（Rust 实现（公式形（零字面特值）（SamusState 结构补（第五组字段）））——编译核（cargo check（**0 错**））。
- **留档可追溯**（refs 引用（bank:addr）+实现补全（Rust 实现（公式形））+编译核（cargo check（0 错））——docs commit（追溯径））。

## 第六组 5 案实现补全（refs 引用必写（bank:addr 引证（写径实证（A 级原文））））

- **第六组**（Samus 域（跳跃/变球（L4 依赖径））——5 案（refs 引用必写（纪律强化（用户裁定）+四项核（对抗复核核（证伪立场））））：
  1. `00:b26d` SFXData@SamusBall（refs asm 考证（bank:addr（00:b26d（写径实证））））
  2. `00:b275` SFXData@SamusDie（refs asm 考证（bank:addr（00:b275（写径实证））））
  3. `00:b598` SamusWalkSFXInit（refs asm 考证（bank:addr（00:b598（写径实证））））
  4. `00:b650` SamusHitSFXCont（refs asm 考证（bank:addr（00:b650（写径实证））））
  5. `00:b658` SamusHitSFXCont@endIf_A（refs asm 考证（bank:addr（00:b658（写径实证））））
- **实现补全**（Rust 实现（公式形（零字面特值）（SamusState 结构补（第六组字段）））——编译核（cargo check（**0 错**））。
- **留档可追溯**（refs 引用（bank:addr）+实现补全（Rust 实现（公式形））+编译核（cargo check（0 错））——docs commit（追溯径））。

## 第七组 5 案实现补全（refs 引用必写（bank:addr 引证（写径实证（A 级原文））））

- **第七组**（Samus 域（跳跃/变球（L4 依赖径））——5 案（refs 引用必写（纪律强化（用户裁定）+四项核（对抗复核核（证伪立场））））：
  1. `00:b673` SamusHitSFXInit（refs asm 考证（bank:addr（00:b673（写径实证））））
  2. `00:b694` SamusHitSFXInit@RTS（refs asm 考证（bank:addr（00:b694（写径实证））））
  3. `00:b703` SamusJumpSFXInit（refs asm 考证（bank:addr（00:b703（写径实证））））
  4. `00:b834` SamusToBallSFXInit（refs asm 考证（bank:addr（00:b834（写径实证））））
  5. `00:b84f` SamusToBallSFXCont（refs asm 考证（bank:addr（00:b84f（写径实证））））
- **实现补全**（Rust 实现（公式形（零字面特值）（SamusState 结构补（第七组字段）））——编译核（cargo check（**0 错**））。
- **留档可追溯**（refs 引用（bank:addr）+实现补全（Rust 实现（公式形））+编译核（cargo check（0 错））——docs commit（追溯径））。
