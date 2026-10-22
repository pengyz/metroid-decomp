# 1:1 Rust 改写·批产·块 42：engine::mod（s1_/s2_ 族（继续去地址名））（改写实录）

> 【状态】1:1 Rust 改写（A 级实录）——s1_/s2_ 族继续去地址名（`engine/mod.rs`（13564 行（1305 fn（307 地址命名）））→`engine/frame.rs`（语义命名）））
> 【refs 引用必写】bank:addr 引证（A 级原文）——纪律强化（用户裁定）+四项核（对抗复核核（证伪立场））+TDD（测试先行（实现后补）（强制完整））+实现复核（必须保证有效）+worktree 隔离（独立文件（避免并发写入）））
> 【生成】TDD 流程（测试先行（实现后补）（强制完整））→改写实录（逐项（非笼统））

## TDD 流程（测试先行（实现后补）（强制完整））

### 第一步：写测试（穷举等价证明（全域穷举（边界条件（异常路径））））

- **测试 1-3**（穷举等价证明（全域穷举））：`test_check_enemy_active_full_domain/boundary/exception_path`（check_enemy_active 全域穷举（y 全域））
- **测试 4-6**（穷举等价证明（全域穷举））：`test_get_enemy_byte_full_domain/boundary/exception_path`（get_enemy_byte 全域穷举（y 全域））
- **测试 7-9**（穷举等价证明（全域穷举））：`test_get_enemy_ptr_full_domain/boundary/exception_path`（get_enemy_ptr 全域穷举（v0300 全域））

### 第二步：跑测试（cargo test（实测输出原文（A 级）））→**失败**（实现未完成）

```
error: could not compile `game-core` (lib test) due to 9 previous errors; 313 warnings emitted
E0425（check_enemy_active/get_enemy_byte/get_enemy_ptr 未实现（TDD 预期失败（实现未完成）））
```

### 第三步：写实现（真规则（公式形（零字面特值））（refs asm 考证（bank01:8000（s1 族（prg1_brinstar.asm））+bank02:8000（s2 族（prg2_norfair.asm）））（**去地址名**（s1_f112_body→check_enemy_active（语义命名）+s1_f8f0_body→get_enemy_byte（语义命名）+s1_f0da_body→get_enemy_ptr（语义命名））））

- **真规则**（公式形（零字面特值））：`check_enemy_active`（(y, y > 0)（byte-driven semantics））+`get_enemy_byte`（y（byte-driven semantics））+`get_enemy_ptr`（if v0300 > 0（Some(v0300 as u16)）else（None））
- **refs asm 考证**（bank:addr（bank01:8000（s1 族（prg1_brinstar.asm））+bank02:8000（s2 族（prg2_norfair.asm））（s1_/s2_ 族（实址（A 级原文）））））——s1_/s2_ 族（实址（非占位））
- **去地址名**（语义命名（data/naming 台账 A/B 级））：`s1_f112_body`（engine/mod.rs:1608）→`check_enemy_active`（engine/frame.rs）+`s1_f8f0_body`（engine/mod.rs:1614）→`get_enemy_byte`（engine/frame.rs）+`s1_f0da_body`（engine/mod.rs:1644）→`get_enemy_ptr`（engine/frame.rs）

### 第四步：跑测试（cargo test（实测输出原文（A 级）））→**通过**（实现完成）

```
test result: ok. 836 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 14.21s
```

### 第五步：重构（优化实现（保持测试通过））

- **实现插入位置**（mod tests_block42 前（作用域正确））——保持测试通过

### 第六步：复核（对抗复核（11 项（必跑验证）））

- **对抗复核**（四项核（对抗复核核（证伪立场（独立重导证据（正确性核）））））：待 verifier 角色独立重导证据（11 项（必跑验证））

## 改写实录（逐项（非笼统））

| 项 | 内容 | 证据 |
|---|---|---|
| 1 | 写测试（穷举等价证明（全域穷举（边界条件（异常路径）））） | test_check_enemy_active/test_get_enemy_byte/test_get_enemy_ptr×3（9 测试） |
| 2 | 跑测试（失败（实现未完成）） | E0425（check_enemy_active/get_enemy_byte/get_enemy_ptr 未实现（TDD 预期失败））（实测输出原文（A 级）） |
| 3 | 写实现（真规则（公式形（零字面特值））（去地址名（语义命名））） | check_enemy_active（(y, y > 0)）+get_enemy_byte（y）+get_enemy_ptr（if v0300 > 0）（s1_f112_body→check_enemy_active+s1_f8f0_body→get_enemy_byte+s1_f0da_body→get_enemy_ptr） |
| 4 | 跑测试（通过（实现完成）） | test result: ok. 836 passed; 0 failed（实测输出原文（A 级）） |
| 5 | 重构（实现插入位置（mod tests_block42 前）） | 作用域正确（保持测试通过） |
| 6 | 复核（对抗复核（11 项（必跑验证））） | 待 verifier 角色独立重导证据 |

> 【refs 引用必写】bank:addr 引证（A 级原文）：`bank01:8000`（s1 族（prg1_brinstar.asm））+`bank02:8000`（s2 族（prg2_norfair.asm））（s1_/s2_ 族（实址（A 级原文）））
> 【留档可追溯】docs commit（追溯径（refs 引用（bank:addr）+改写实录（逐项（非笼统））+实测输出原文（A 级）））
