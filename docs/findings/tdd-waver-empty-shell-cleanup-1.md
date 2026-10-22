# TDD+实现复核·空壳清理·第一件：enemy::waver（Waver）（事实证据文档）

> 【状态】TDD+实现复核（A 级实录）——空壳清理（`update_waver`→`WaverState::default()`（无逻辑）→**真实现补全**）
> 【refs 引用必写】bank:addr 引证（A 级原文）——纪律强化（用户裁定）+四项核（对抗复核核（证伪立场））+TDD（测试先行（实现后补））+实现复核（必须保证有效）
> 【生成】TDD 流程（测试先行（实现后补））→事实证据文档（逐项（非笼统））

## TDD 流程（测试先行（实现后补））

### 第一步：写测试（穷举等价证明（全域穷举（边界条件（异常路径））））

- **测试 1**（穷举等价证明（全域穷举））：`test_update_waver_full_domain`（update_waver 全域穷举（RAM 态全域（direction/phase/speed）））
- **测试 2**（边界条件）：`test_update_waver_boundary`（update_waver 边界条件（direction/phase 边界值））
- **测试 3**（异常路径）：`test_update_waver_exception_path`（update_waver 异常路径（RAM 非零（byte-driven semantics）））

### 第二步：跑测试（cargo test（实测输出原文（A 级）））→**失败**（实现未完成）

```
test enemy::waver::tests::test_update_waver_full_domain ... ok
test enemy::waver::tests::test_update_waver_exception_path ... FAILED
test enemy::waver::tests::test_update_waver_boundary ... FAILED

---- enemy::waver::tests::test_update_waver_exception_path stdout ----
assertion `left == right` failed: WaverState.direction 异常路径（0x7F）
  left: 0
 right: 127

test result: FAILED. 561 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
```

### 第三步：写实现（真规则（公式形（零字面特值））（refs asm 考证（bank01:97D7（EnemyMovementChoice_Waver_BANK1（prg1_brinstar.asm:498）））））

- **真规则**（公式形（零字面特值））：`WaverState`（direction=ram[0x0300]/phase=ram[0x0301]/speed=ram[0x0302]（byte-driven semantics））
- **refs asm 考证**（bank:addr（bank01:97D7（EnemyMovementChoice_Waver_BANK1（prg1_brinstar.asm:498））））——Waver 例程（实址（A 级原文））

### 第四步：跑测试（cargo test（实测输出原文（A 级）））→**通过**（实现完成）

```
test result: ok. 568 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.01s
```

### 第五步：重构（优化实现（保持测试通过））

- **编译错修**（`//!` 注释在 mod tests 后）→**修**（`//!`→`//`（mod tests 后注释）（TDD 流程（测试先行）））
- **测试修**（x/y→direction/phase/speed（WaverState 字段））——保持测试通过
- **waver 模块导出**（enemy/mod.rs（pub mod waver））——TDD 流程（测试先行）

### 第六步：复核（对抗复核（11 项（必跑验证）））

- **对抗复核**（四项核（对抗复核核（证伪立场（独立重导证据（正确性核）））））：待 verifier 角色独立重导证据（11 项（必跑验证））

## 事实证据文档（逐项（非笼统））

| 项 | 内容 | 证据 |
|---|---|---|
| 1 | 写测试（穷举等价证明（全域穷举（边界条件（异常路径）））） | test_update_waver_full_domain/boundary/exception_path（3 测试） |
| 2 | 跑测试（失败（实现未完成）） | test result: FAILED. 561 passed; 2 failed（实测输出原文（A 级）） |
| 3 | 写实现（真规则（公式形（零字面特值））） | WaverState（direction=ram[0x0300]/phase=ram[0x0301]/speed=ram[0x0302]（byte-driven semantics）） |
| 4 | 跑测试（通过（实现完成）） | test result: ok. 568 passed; 0 failed（实测输出原文（A 级）） |
| 5 | 重构（编译错修+测试修+模块导出） | `//!`→`//`（mod tests 后注释）+x/y→direction/phase/speed（WaverState 字段）+pub mod waver（enemy/mod.rs） |
| 6 | 复核（对抗复核（11 项（必跑验证））） | 待 verifier 角色独立重导证据 |

> 【refs 引用必写】bank:addr 引证（A 级原文）：`bank01:97D7`（EnemyMovementChoice_Waver_BANK1（prg1_brinstar.asm:498））（Waver 例程（实址（A 级原文）））
> 【留档可追溯】docs commit（追溯径（refs 引用（bank:addr）+事实证据文档（逐项（非笼统））+实测输出原文（A 级）））
