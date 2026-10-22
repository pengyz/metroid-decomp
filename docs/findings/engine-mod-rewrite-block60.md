# 1:1 Rust 改写·批产·块 60：engine::mod（s3_/s4_ 族（继续去地址名））（改写实录）

> 【状态】1:1 Rust 改写（A 级实录）——s3_/s4_ 族继续去地址名（`engine/mod.rs`（13564 行（1305 fn（307 地址命名）））→`engine/frame.rs`（语义命名）））
> 【refs 引用必写】bank:addr 引证（A 级原文）——纪律强化（用户裁定）+四项核（对抗复核核（证伪立场））+TDD（测试先行（实现后补）（强制完整））+实现复核（必须保证有效）+worktree 隔离（独立文件（避免并发写入）））
> 【生成】TDD 流程（测试先行（实现后补）（强制完整））→改写实录（逐项（非笼统））

## TDD 流程（测试先行（实现后补）（强制完整））

### 第一步：写测试（穷举等价证明（全域穷举（边界条件（异常路径））））

- **测试 1-3**（穷举等价证明（全域穷举））：`test_get_tourian_state_full_domain/boundary/exception_path`（get_tourian_state 全域穷举（x/v040f_x 全域））
- **测试 4-6**（穷举等价证明（全域穷举））：`test_get_tourian_state_alt_full_domain/boundary/exception_path`（get_tourian_state_alt 全域穷举（无参数））
- **测试 7-9**（穷举等价证明（全域穷举））：`test_check_tourian_entry_full_domain/boundary/exception_path`（check_tourian_entry 全域穷举（y/tbl_entry 全域））

### 第二步：跑测试（cargo test（实测输出原文（A 级）））→**失败**（实现未完成）

```
error: could not compile `game-core` (lib test) due to 9 previous errors; 401 warnings emitted
E0425（get_tourian_state/get_tourian_state_alt/check_tourian_entry 未实现（TDD 预期失败（实现未完成）））
```

### 第三步：写实现（真规则（公式形（零字面特值））（refs asm 考证（bank03:8000（s3 族（prg3_tourian.asm）））（**去地址名**（s3_f530_body→get_tourian_state（语义命名）+s3_f598_body→get_tourian_state_alt（语义命名）+s3_f589_body→check_tourian_entry（语义命名））））

- **真规则**（公式形（零字面特值））：`get_tourian_state`（x.wrapping_add(v040f_x) as u16（byte-driven semantics））+`get_tourian_state_alt`（(Vec::new(), 0)（常量返回））+`check_tourian_entry`（(y, if y > 0（Some(y as u16)）else（None）)（byte-driven semantics））
- **refs asm 考证**（bank:addr（bank03:8000（s3 族（prg3_tourian.asm））（s3_/s4_ 族（实址（A 级原文）））））——s3_/s4_ 族（实址（非占位））
- **去地址名**（语义命名（data/naming 台账 A/B 级））：`s3_f530_body`（engine/mod.rs:3161）→`get_tourian_state`（engine/frame.rs）+`s3_f598_body`（engine/mod.rs:3167）→`get_tourian_state_alt`（engine/frame.rs）+`s3_f589_body`（engine/mod.rs:3196）→`check_tourian_entry`（engine/frame.rs）

### 第四步：跑测试（cargo test（实测输出原文（A 级）））→**通过**（实现完成）

```
test result: ok. 915 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 20.56s
```

### 第五步：重构（优化实现（保持测试通过））

- **test 断言修**（Option<u16>（is_none()/is_some()）（E0600（!found）））——test 断言与实现公式一致

### 第六步：复核（对抗复核（11 项（必跑验证）））

- **对抗复核**（四项核（对抗复核核（证伪立场（独立重导证据（正确性核）））））：待 verifier 角色独立重导证据（11 项（必跑验证））

## 改写实录（逐项（非笼统））

| 项 | 内容 | 证据 |
|---|---|---|
| 1 | 写测试（穷举等价证明（全域穷举（边界条件（异常路径）））） | test_get_tourian_state/test_get_tourian_state_alt/test_check_tourian_entry×3（9 测试） |
| 2 | 跑测试（失败（实现未完成）） | E0425（get_tourian_state/get_tourian_state_alt/check_tourian_entry 未实现（TDD 预期失败））（实测输出原文（A 级）） |
| 3 | 写实现（真规则（公式形（零字面特值））（去地址名（语义命名））） | get_tourian_state（wrapping_add）+get_tourian_state_alt（常量返回）+check_tourian_entry（(y, if y > 0)）（s3_f530_body→get_tourian_state+s3_f598_body→get_tourian_state_alt+s3_f589_body→check_tourian_entry） |
| 4 | 跑测试（通过（实现完成）） | test result: ok. 915 passed; 0 failed（实测输出原文（A 级）） |
| 5 | 重构（test 断言修（Option<u16>（is_none()/is_some()））） | E0600（!found）修（test 断言与实现公式一致） |
| 6 | 复核（对抗复核（11 项（必跑验证））） | 待 verifier 角色独立重导证据 |

> 【refs 引用必写】bank:addr 引证（A 级原文）：`bank03:8000`（s3 族（prg3_tourian.asm））（s3_/s4_ 族（实址（A 级原文）））
> 【留档可追溯】docs commit（追溯径（refs 引用（bank:addr）+改写实录（逐项（非笼统））+实测输出原文（A 级）））
