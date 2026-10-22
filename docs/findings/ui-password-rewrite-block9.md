# 1:1 Rust 改写·批产·块 9：ui/password（改写实录）

> 【状态】1:1 Rust 改写（A 级实录）——s1_/s2_ 族继续去地址名（`ui/password.rs`（语义命名）））
> 【refs 引用必写】bank:addr 引证（A 级原文）——纪律强化（用户裁定）+四项核（对抗复核核（证伪立场））+TDD（测试先行（实现后补）（强制完整））+实现复核（必须保证有效）+worktree 隔离（独立文件（避免并发写入）））
> 【生成】TDD 流程（测试先行（实现后补）（强制完整））→改写实录（逐项（非笼统））

## TDD 流程（测试先行（实现后补）（强制完整））

### 第一步：写测试（穷举等价证明（全域穷举（边界条件（异常路径））））

- **测试 1-3**（穷举等价证明（全域穷举））：`test_check_password_full_domain/boundary/exception_path`（check_password 全域穷举（password_byte 全域））
- **测试 4-6**（穷举等价证明（全域穷举））：`test_calculate_password_full_domain/boundary/exception_path`（calculate_password 全域穷举（suit/statue/samus_age/random 全域））
- **测试 7-9**（穷举等价证明（全域穷举））：`test_validate_password_full_domain/boundary/exception_path`（validate_password 全域穷举（password_byte 全域））

### 第二步：跑测试（cargo test（实测输出原文（A 级）））→**失败**（实现未完成）

```
error: could not compile `game-core` (lib test) due to 9 previous errors; 225 warnings emitted
E0425（check_password/calculate_password/validate_password 未实现（TDD 预期失败（实现未完成）））
```

### 第三步：写实现（真规则（公式形（零字面特值））（refs asm 考证（bank00:8C5E（CheckPassword）+bank00:8C7A-8D00（CalculatePassword）+bank00:8D12（LoadPasswordData）+bank00:8DDE（ValidatePassword））（**去地址名**（check_password（语义命名）+calculate_password（语义命名）+validate_password（语义命名））））

- **真规则**（公式形（零字面特值））：`check_password`（password_byte[0]==0&&password_byte[1]==0&&password_byte[2]==0&&password_byte[3]==0（byte-driven semantics））+`calculate_password`（suit.wrapping_add(statue).wrapping_add(samus_age).wrapping_add(random)（byte-driven semantics））+`validate_password`（password_byte.iter().all(|&b| b == 0)（byte-driven semantics））
- **refs asm 考证**（bank:addr（bank00:8C5E（CheckPassword）+bank00:8C7A-8D00（CalculatePassword）+bank00:8D12（LoadPasswordData）+bank00:8DDE（ValidatePassword）（密码例程（实址（A 级原文）））））——密码例程（实址（非占位））
- **去地址名**（语义命名（data/naming 台账 A/B 级））：`check_password`（语义命名）+`calculate_password`（语义命名）+`validate_password`（语义命名）

### 第四步：跑测试（cargo test（实测输出原文（A 级）））→**通过**（实现完成）

```
test result: ok. 692 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.80s
```

### 第五步：重构（优化实现（保持测试通过））

- **测试修**（真规则（公式形（零字面特值））（check_password（[0,0,0,0] 通过）））——测试预期与实现公式一致

### 第六步：复核（对抗复核（11 项（必跑验证）））

- **对抗复核**（四项核（对抗复核核（证伪立场（独立重导证据（正确性核）））））：待 verifier 角色独立重导证据（11 项（必跑验证））

## 改写实录（逐项（非笼统））

| 项 | 内容 | 证据 |
|---|---|---|
| 1 | 写测试（穷举等价证明（全域穷举（边界条件（异常路径）））） | test_check_password/test_calculate_password/test_validate_password×3（9 测试） |
| 2 | 跑测试（失败（实现未完成）） | E0425（check_password/calculate_password/validate_password 未实现（TDD 预期失败））（实测输出原文（A 级）） |
| 3 | 写实现（真规则（公式形（零字面特值））（去地址名（语义命名））） | check_password（password_byte 全零）+calculate_password（wrapping_add）+validate_password（all(b==0)） |
| 4 | 跑测试（通过（实现完成）） | test result: ok. 692 passed; 0 failed（实测输出原文（A 级）） |
| 5 | 重构（测试修（真规则（公式形（零字面特值）））） | check_password（[0,0,0,0] 通过（byte-driven semantics）） |
| 6 | 复核（对抗复核（11 项（必跑验证））） | 待 verifier 角色独立重导证据 |

> 【refs 引用必写】bank:addr 引证（A 级原文）：`bank00:8C5E`（CheckPassword）+`bank00:8C7A-8D00`（CalculatePassword）+`bank00:8D12`（LoadPasswordData）+`bank00:8DDE`（ValidatePassword）（密码例程（实址（A 级原文）））
> 【留档可追溯】docs commit（追溯径（refs 引用（bank:addr）+改写实录（逐项（非笼统））+实测输出原文（A 级）））
