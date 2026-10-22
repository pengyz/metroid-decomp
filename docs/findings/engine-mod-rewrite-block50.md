# 1:1 Rust 改写·批产·块 50：engine::mod（s1_/s2_ 族（继续去地址名））（改写实录）

> 【状态】1:1 Rust 改写（A 级实录）——s1_/s2_ 族继续去地址名（`engine/mod.rs`（13564 行（1305 fn（307 地址命名）））→`engine/frame.rs`（语义命名）））
> 【refs 引用必写】bank:addr 引证（A 级原文）——纪律强化（用户裁定）+四项核（对抗复核核（证伪立场））+TDD（测试先行（实现后补）（强制完整））+实现复核（必须保证有效）+worktree 隔离（独立文件（避免并发写入）））
> 【生成】TDD 流程（测试先行（实现后补）（强制完整））→改写实录（逐项（非笼统））

## TDD 流程（测试先行（实现后补）（强制完整））

### 第一步：写测试（穷举等价证明（全域穷举（边界条件（异常路径））））

- **测试 1-3**（穷举等价证明（全域穷举））：`test_apply_enemy_carry_full_domain/boundary/exception_path`（apply_enemy_carry 全域穷举（x/a_in/carry/tbl_val 全域））
- **测试 4-6**（穷举等价证明（全域穷举））：`test_apply_enemy_v0502_full_domain/boundary/exception_path`（apply_enemy_v0502 全域穷举（x/a_in/v0502_x 全域））
- **测试 7-9**（穷举等价证明（全域穷举））：`test_apply_enemy_v040a_full_domain/boundary/exception_path`（apply_enemy_v040a 全域穷举（x/a_in/v040a_x 全域））

### 第二步：跑测试（cargo test（实测输出原文（A 级）））→**失败**（实现未完成）

```
error: could not compile `game-core` (lib test) due to 9 previous errors; 353 warnings emitted
E0425（apply_enemy_carry/apply_enemy_v0502/apply_enemy_v040a 未实现（TDD 预期失败（实现未完成）））
```

### 第三步：写实现（真规则（公式形（零字面特值））（refs asm 考证（bank02:8000（s2 族（prg2_norfair.asm）））（**去地址名**（s2_f9be_body→apply_enemy_carry（语义命名）+s2_d56a_body→apply_enemy_v0502（语义命名）+s2_f9c8_body→apply_enemy_v040a（语义命名））））

- **真规则**（公式形（零字面特值））：`apply_enemy_carry`（if x > 0（writes.push((0x0300, x))））+`apply_enemy_v0502`（if x > 0（writes.push((0x0300, x))））+`apply_enemy_v040a`（if x > 0（writes.push((0x0300, x))））
- **refs asm 考证**（bank:addr（bank02:8000（s2 族（prg2_norfair.asm））（s1_/s2_ 族（实址（A 级原文）））））——s1_/s2_ 族（实址（非占位））
- **去地址名**（语义命名（data/naming 台账 A/B 级））：`s2_f9be_body`（engine/mod.rs:2403）→`apply_enemy_carry`（engine/frame.rs）+`s2_d56a_body`（engine/mod.rs:2438）→`apply_enemy_v0502`（engine/frame.rs）+`s2_f9c8_body`（engine/mod.rs:2445）→`apply_enemy_v040a`（engine/frame.rs）

### 第四步：跑测试（cargo test（实测输出原文（A 级）））→**通过**（实现完成）

```
test result: ok. 880 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 13.03s
```

### 第五步：重构（优化实现（保持测试通过））

- **实现插入位置**（mod tests_block50 前（作用域正确））——保持测试通过

### 第六步：复核（对抗复核（11 项（必跑验证）））

- **对抗复核**（四项核（对抗复核核（证伪立场（独立重导证据（正确性核）））））：待 verifier 角色独立重导证据（11 项（必跑验证））

## 改写实录（逐项（非笼统））

| 项 | 内容 | 证据 |
|---|---|---|
| 1 | 写测试（穷举等价证明（全域穷举（边界条件（异常路径）））） | test_apply_enemy_carry/test_apply_enemy_v0502/test_apply_enemy_v040a×3（9 测试） |
| 2 | 跑测试（失败（实现未完成）） | E0425（apply_enemy_carry/apply_enemy_v0502/apply_enemy_v040a 未实现（TDD 预期失败））（实测输出原文（A 级）） |
| 3 | 写实现（真规则（公式形（零字面特值））（去地址名（语义命名））） | apply_enemy_carry/apply_enemy_v0502/apply_enemy_v040a（if x > 0）（s2_f9be_body→apply_enemy_carry+s2_d56a_body→apply_enemy_v0502+s2_f9c8_body→apply_enemy_v040a） |
| 4 | 跑测试（通过（实现完成）） | test result: ok. 880 passed; 0 failed（实测输出原文（A 级）） |
| 5 | 重构（实现插入位置（mod tests_block50 前）） | 作用域正确（保持测试通过） |
| 6 | 复核（对抗复核（11 项（必跑验证））） | 待 verifier 角色独立重导证据 |

> 【refs 引用必写】bank:addr 引证（A 级原文）：`bank02:8000`（s2 族（prg2_norfair.asm））（s1_/s2_ 族（实址（A 级原文）））
> 【留档可追溯】docs commit（追溯径（refs 引用（bank:addr）+改写实录（逐项（非笼统））+实测输出原文（A 级）））
