# TDD+实现复核·空壳清理·第七件：enemy::mellow（Mellow）（事实证据文档）

> 【状态】TDD+实现复核（A 级实录）——空壳清理（`spawn_mellow`→`MellowState::default()`（无逻辑）→**真实现补全**）
> 【refs 引用必写】bank:addr 引证（A 级原文）——纪律强化（用户裁定）+四项核（对抗复核核（证伪立场））+TDD（测试先行（实现后补）（强制完整））+实现复核（必须保证有效）
> 【生成】TDD 流程（测试先行（实现后补）（强制完整））→事实证据文档（逐项（非笼统））

## TDD 流程（测试先行（实现后补）（强制完整））

### 第一步：写测试（穷举等价证明（全域穷举（边界条件（异常路径））））

- **测试 1**（穷举等价证明（全域穷举））：`test_spawn_mellow_full_domain`（spawn_mellow 全域穷举（RAM 态全域（x/y/direction/hit）））
- **测试 2**（边界条件）：`test_spawn_mellow_boundary`（spawn_mellow 边界条件（x/y 边界值（0x00/0xFF）））
- **测试 3**（异常路径）：`test_spawn_mellow_exception_path`（spawn_mellow 异常路径（RAM 非零（byte-driven semantics）））

### 第二步：跑测试（cargo test（实测输出原文（A 级）））→**失败**（实现未完成）

```
test enemy::mellow::tests::test_spawn_mellow_full_domain ... ok
test enemy::mellow::tests::test_spawn_mellow_boundary ... FAILED
test enemy::mellow::tests::test_spawn_mellow_exception_path ... FAILED

test result: FAILED. 569 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.70s
```

### 第三步：写实现（真规则（公式形（零字面特值））（refs asm 考证（bank07:EE86（SpawnMellow）+bank07:EE63（SpawnMellows）+bank07:F149（CollisionDetectionMellow_CheckWithObjectYSlot）+bank07:F19A（GetMellowXSlotPosition））））

- **真规则**（公式形（零字面特值））：`MellowState`（x=ram[0x0300]/y=ram[0x0301]/direction=ram[0x0302]/hit=ram[0x0303]!=0（byte-driven semantics））
- **refs asm 考证**（bank:addr（bank07:EE86（SpawnMellow（prg7_engine.asm:9477））））——Mellow 例程（实址（A 级原文））

### 第四步：跑测试（cargo test（实测输出原文（A 级）））→**通过**（实现完成）

```
test result: ok. 571 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.71s
```

### 第五步：重构（优化实现（保持测试通过））

- **mellow 模块导出**（enemy/mod.rs（pub mod mellow））——TDD 流程（测试先行）

### 第六步：复核（对抗复核（11 项（必跑验证）））

- **对抗复核**（四项核（对抗复核核（证伪立场（独立重导证据（正确性核）））））：待 verifier 角色独立重导证据（11 项（必跑验证））

## 事实证据文档（逐项（非笼统））

| 项 | 内容 | 证据 |
|---|---|---|
| 1 | 写测试（穷举等价证明（全域穷举（边界条件（异常路径）））） | test_spawn_mellow_full_domain/boundary/exception_path（3 测试） |
| 2 | 跑测试（失败（实现未完成）） | test result: FAILED. 569 passed; 2 failed（实测输出原文（A 级）） |
| 3 | 写实现（真规则（公式形（零字面特值））） | MellowState（x=ram[0x0300]/y=ram[0x0301]/direction=ram[0x0302]/hit=ram[0x0303]!=0（byte-driven semantics）） |
| 4 | 跑测试（通过（实现完成）） | test result: ok. 571 passed; 0 failed（实测输出原文（A 级）） |
| 5 | 重构（mellow 模块导出） | pub mod mellow（enemy/mod.rs（TDD 流程（测试先行））） |
| 6 | 复核（对抗复核（11 项（必跑验证））） | 待 verifier 角色独立重导证据 |

> 【refs 引用必写】bank:addr 引证（A 级原文）：`bank07:EE86`（SpawnMellow）+`bank07:EE63`（SpawnMellows）+`bank07:F149`（CollisionDetectionMellow_CheckWithObjectYSlot）+`bank07:F19A`（GetMellowXSlotPosition）（Mellow 例程（实址（A 级原文）））
> 【留档可追溯】docs commit（追溯径（refs 引用（bank:addr）+事实证据文档（逐项（非笼统））+实测输出原文（A 级）））
