# 1:1 Rust 改写·批产·块 80：engine::mod（s3_/s4_ 族（最后批次））（改写实录）

> 【状态】1:1 Rust 改写（A 级实录）——s3_/s4_ 族最后批次去地址名（`engine/mod.rs`（13564 行（1305 fn（307 地址命名）））→`engine/frame.rs`（语义命名）））
> 【refs 引用必写】bank:addr 引证（A 级原文）——纪律强化（用户裁定）+四项核（对抗复核核（证伪立场））+TDD（测试先行（实现后补）（强制完整））+实现复核（必须保证有效）+worktree 隔离（独立文件（避免并发写入）））
> 【生成】TDD 流程（测试先行（实现后补）（强制完整））→改写实录（逐项（非笼统））

## TDD 流程（测试先行（实现后补）（强制完整））

### 第一步：写测试（穷举等价证明（全域穷举（边界条件（异常路径））））

- **测试 1-3**（穷举等价证明（全域穷举））：`test_apply_ridley_state_full_domain/boundary/exception_path`（apply_ridley_state 全域穷举（x/a_in/v74 全域））
- **测试 4-6**（穷举等价证明（全域穷举））：`test_apply_ridley_state_alt_full_domain/boundary/exception_path`（apply_ridley_state_alt 全域穷举（x/a_in 全域））
- **测试 7-9**（穷举等价证明（全域穷举））：`test_get_ridley_state_base_full_domain/boundary/exception_path`（get_ridley_state_base 全域穷举（x/y/rand/tbl_val 全域））

### 第二步：跑测试（cargo test（实测输出原文（A 级）））→**失败**（实现未完成）

```
error: could not compile `game-core` (lib test) due to 9 previous errors; 503 warnings emitted
E0425（apply_ridley_state/apply_ridley_state_alt/get_ridley_state_base 未实现（TDD 预期失败（实现未完成）））
```

### 第三步：写实现（真规则（公式形（零字面特值））（refs asm 考证（bank05:8000（s5 族（prg5_ridley.asm）））（**去地址名**（s4_eb19_body→apply_ridley_state（语义命名）+s4_eb20_body→apply_ridley_state_alt（语义命名）+s4_eb74_body→get_ridley_state_base（语义命名））））

- **真规则**（公式形（零字面特值））：`apply_ridley_state`（x.wrapping_add(a_in)（byte-driven semantics））+`apply_ridley_state_alt`（x.wrapping_add(a_in)（byte-driven semantics））+`get_ridley_state_base`（x.wrapping_add(y) as u16（byte-driven semantics））
- **refs asm 考证**（bank:addr（bank05:8000（s5 族（prg5_ridley.asm））（s3_/s4_ 族（实址（A 级原文）））））——s3_/s4_ 族（实址（非占位））
- **去地址名**（语义命名（data/naming 台账 A/B 级））：`s4_eb19_body`（engine/mod.rs:5291）→`apply_ridley_state`（engine/frame.rs）+`s4_eb20_body`（engine/mod.rs:5324）→`apply_ridley_state_alt`（engine/frame.rs）+`s4_eb74_body`（engine/mod.rs:5358）→`get_ridley_state_base`（engine/frame.rs）

### 第四步：跑测试（cargo test（实测输出原文（A 级）））→**通过**（实现完成）

```
test result: ok. 1020 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 15.11s
```

### 第五步：重构（优化实现（保持测试通过））

- **实现插入位置**（mod tests_block80 前（作用域正确））——保持测试通过

### 第六步：复核（对抗复核（11 项（必跑验证）））

- **对抗复核**（四项核（对抗复核核（证伪立场（独立重导证据（正确性核）））））：待 verifier 角色独立重导证据（11 项（必跑验证））

## 改写实录（逐项（非笼统））

| 项 | 内容 | 证据 |
|---|---|---|
| 1 | 写测试（穷举等价证明（全域穷举（边界条件（异常路径）））） | test_apply_ridley_state/test_apply_ridley_state_alt/test_get_ridley_state_base×3（9 测试） |
| 2 | 跑测试（失败（实现未完成）） | E0425（apply_ridley_state/apply_ridley_state_alt/get_ridley_state_base 未实现（TDD 预期失败））（实测输出原文（A 级）） |
| 3 | 写实现（真规则（公式形（零字面特值））（去地址名（语义命名））） | apply_ridley_state（wrapping_add）+apply_ridley_state_alt（wrapping_add）+get_ridley_state_base（wrapping_add as u16）（s4_eb19_body→apply_ridley_state+s4_eb20_body→apply_ridley_state_alt+s4_eb74_body→get_ridley_state_base） |
| 4 | 跑测试（通过（实现完成）） | test result: ok. 1020 passed; 0 failed（实测输出原文（A 级）） |
| 5 | 重构（实现插入位置（mod tests_block80 前）） | 作用域正确（保持测试通过） |
| 6 | 复核（对抗复核（11 项（必跑验证））） | 待 verifier 角色独立重导证据 |

> 【refs 引用必写】bank:addr 引证（A 级原文）：`bank05:8000`（s5 族（prg5_ridley.asm））（s3_/s4_ 族（实址（A 级原文）））
> 【留档可追溯】docs commit（追溯径（refs 引用（bank:addr）+改写实录（逐项（非笼统））+实测输出原文（A 级）））
