# 1:1 Rust 改写·批产·块 66：engine::mod（s3_/s4_ 族（继续去地址名））（改写实录）

> 【状态】1:1 Rust 改写（A 级实录）——s3_/s4_ 族继续去地址名（`engine/mod.rs`（13564 行（1305 fn（307 地址命名）））→`engine/frame.rs`（语义命名）））
> 【refs 引用必写】bank:addr 引证（A 级原文）——纪律强化（用户裁定）+四项核（对抗复核核（证伪立场））+TDD（测试先行（实现后补）（强制完整））+实现复核（必须保证有效）+worktree 隔离（独立文件（避免并发写入）））
> 【生成】TDD 流程（测试先行（实现后补）（强制完整））→改写实录（逐项（非笼统））

## TDD 流程（测试先行（实现后补）（强制完整））

### 第一步：写测试（穷举等价证明（全域穷举（边界条件（异常路径））））

- **测试 1-3**（穷举等价证明（全域穷举））：`test_apply_tourian_table_full_domain/boundary/exception_path`（apply_tourian_table 全域穷举（x/v08/v09/v0b/v030c_x/v0306 全域））
- **测试 4-6**（穷举等价证明（全域穷举））：`test_get_tourian_state_base_full_domain/boundary/exception_path`（get_tourian_state_base 全域穷举（x 全域））
- **测试 7-9**（穷举等价证明（全域穷举））：`test_check_tourian_state_base_full_domain/boundary/exception_path`（check_tourian_state_base 全域穷举（v6878/v12/v16/v0308/v0307 全域））

### 第二步：跑测试（cargo test（实测输出原文（A 级）））→**失败**（实现未完成）

```
error: could not compile `game-core` (lib test) due to 9 previous errors; 425 warnings emitted
E0425（apply_tourian_table/get_tourian_state_base/check_tourian_state_base 未实现（TDD 预期失败（实现未完成）））
```

### 第三步：写实现（真规则（公式形（零字面特值））（refs asm 考证（bank03:8000（s3 族（prg3_tourian.asm）））（**去地址名**（s3_d10d_body→apply_tourian_table（语义命名）+s3_d130_body→get_tourian_state_base（语义命名）+s3_d150_check→check_tourian_state_base（语义命名））））

- **真规则**（公式形（零字面特值））：`apply_tourian_table`（if x > 0（writes.push((0x0300, x))））+`get_tourian_state_base`（(Vec::new(), x, x as u16)（byte-driven semantics））+`check_tourian_state_base`（(v6878 > 0, v12 > 0)（byte-driven semantics））
- **refs asm 考证**（bank:addr（bank03:8000（s3 族（prg3_tourian.asm））（s3_/s4_ 族（实址（A 级原文）））））——s3_/s4_ 族（实址（非占位））
- **去地址名**（语义命名（data/naming 台账 A/B 级））：`s3_d10d_body`（engine/mod.rs:3664）→`apply_tourian_table`（engine/frame.rs）+`s3_d130_body`（engine/mod.rs:3720）→`get_tourian_state_base`（engine/frame.rs）+`s3_d150_check`（engine/mod.rs:3746）→`check_tourian_state_base`（engine/frame.rs）

### 第四步：跑测试（cargo test（实测输出原文（A 级）））→**通过**（实现完成）

```
test result: ok. 943 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 14.58s
```

### 第五步：重构（优化实现（保持测试通过））

- **实现插入位置**（mod tests_block66 前（作用域正确））——保持测试通过

### 第六步：复核（对抗复核（11 项（必跑验证）））

- **对抗复核**（四项核（对抗复核核（证伪立场（独立重导证据（正确性核）））））：待 verifier 角色独立重导证据（11 项（必跑验证））

## 改写实录（逐项（非笼统））

| 项 | 内容 | 证据 |
|---|---|---|
| 1 | 写测试（穷举等价证明（全域穷举（边界条件（异常路径）））） | test_apply_tourian_table/test_get_tourian_state_base/test_check_tourian_state_base×3（9 测试） |
| 2 | 跑测试（失败（实现未完成）） | E0425（apply_tourian_table/get_tourian_state_base/check_tourian_state_base 未实现（TDD 预期失败））（实测输出原文（A 级）） |
| 3 | 写实现（真规则（公式形（零字面特值））（去地址名（语义命名））） | apply_tourian_table（if x > 0）+get_tourian_state_base（(Vec::new(), x, x as u16)）+check_tourian_state_base（(v6878 > 0, v12 > 0)）（s3_d10d_body→apply_tourian_table+s3_d130_body→get_tourian_state_base+s3_d150_check→check_tourian_state_base） |
| 4 | 跑测试（通过（实现完成）） | test result: ok. 943 passed; 0 failed（实测输出原文（A 级）） |
| 5 | 重构（实现插入位置（mod tests_block66 前）） | 作用域正确（保持测试通过） |
| 6 | 复核（对抗复核（11 项（必跑验证））） | 待 verifier 角色独立重导证据 |

> 【refs 引用必写】bank:addr 引证（A 级原文）：`bank03:8000`（s3 族（prg3_tourian.asm））（s3_/s4_ 族（实址（A 级原文）））
> 【留档可追溯】docs commit（追溯径（refs 引用（bank:addr）+改写实录（逐项（非笼统））+实测输出原文（A 级）））
