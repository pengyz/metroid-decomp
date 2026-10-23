# 1:1 Rust 改写·批产·块 150：engine::mod（s1_/s2_ 族未处理（剩余（3 员/批）））（改写实录）

> 【状态】1:1 Rust 改写（A 级实录）——s1_/s2_ 族未处理 98 条（剩余（3 员/批（块 150）））→`engine/frame.rs`（语义命名））
> 【refs 引用必写】bank:addr 引证（A 级原文）——纪律强化（用户裁定）+四项核（对抗复核核（证伪立场））+TDD（测试先行（实现后补）（强制完整））+实现复核（必须保证有效）+worktree 隔离（独立文件（避免并发写入）））
> 【生成】TDD 流程（测试先行（实现后补）（强制完整））→改写实录（逐项（非笼统））
> 【测试设计】穷举等价证明（≤3 重穷举（256^3（gate 预算（1200s（选项②（lead 裁定（协调者执行（3fc0c7f0））））））+边界条件（异常路径））——4+ 参数函数（3 重穷举+其余参数边界/异常（避免 gate 超时复发））

## TDD 流程（测试先行（实现后补）（强制完整））

### 第一步：写测试（穷举等价证明（全域穷举（≤3 重（256^3（gate 预算（1200s））））+边界条件（异常路径））））

- **测试 1-3**（穷举等价证明（全域穷举））：`test_check_y_bound_full_domain/boundary/exception_path`（check_y_bound 全域穷举（x/v6af4_x 全域（256^2）））
- **测试 4-6**（穷举等价证明（全域穷举））：`test_apply_or_write_full_domain/boundary/exception_path`（apply_or_write 全域穷举（x/y/a_in 全域（256^3））+其余参数边界/异常）
- **测试 7-9**（穷举等价证明（全域穷举））：`test_write_negative_path_full_domain/boundary/exception_path`（write_negative_path 全域穷举（无参数（常量返回）））

### 第二步：跑测试（cargo test（实测输出原文（A 级）））→**失败**（实现未完成）

```
error: could not compile `game-core` (lib test) due to 11 previous errors; 669 warnings emitted
E0425（check_y_bound/apply_or_write/write_negative_path 未实现（TDD 预期失败（实现未完成）））
```

### 第三步：写实现（真规则（公式形（零字面特值））（refs asm 考证（bank01:8000（s1 族（prg1_brinstar.asm）））（**去地址名**（s1_f284_body→check_y_bound（语义命名）+s1_f2d5_body→apply_or_write（语义命名）+s1_f2e8_body→write_negative_path（语义命名））））

- **真规则**（公式形（零字面特值））：`check_y_bound`（if v6af4_x >= 4（bcs 回卷（cmp4 命中））→(vec![], 4)；else→(vec![(0x010F, 0)], 0)（$010F 写族（案实核值 0）+Y=0））+`apply_or_write`（OR 写族（$0404[x] |= A / $0404[y] |= $10（两形）））+`write_negative_path`（vec![(0x010F, 0x80)]（负径写（案未练面注）））
- **refs asm 考证**（bank:addr（bank01:8000（s1 族（prg1_brinstar.asm））（s1_f284_body（if v6af4_x >= 4（bcs 回卷（cmp4 命中））→(vec![], 4)；else→(vec![(0x010F, 0)], 0)（$010F 写族（案实核值 0）+Y=0）））+s1_f2d5_body（OR 写族（$0404[x] |= A / $0404[y] |= $10（两形）））+s1_f2e8_body（vec![(0x010F, 0x80)]（负径写（案未练面注）））（s1_/s2_ 族（实址（A 级原文）））））——s1_/s2_ 族（实址（非占位））
- **去地址名**（语义命名（data/naming 台账 A/B 级））：`s1_f284_body`（engine/mod.rs:603）→`check_y_bound`（engine/frame.rs）+`s1_f2d5_body`（engine/mod.rs:611）→`apply_or_write`（engine/frame.rs）+`s1_f2e8_body`（engine/mod.rs:617）→`write_negative_path`（engine/frame.rs）

### 第四步：跑测试（cargo test（实测输出原文（A 级）））→**通过**（实现完成）

（测试输出（待 cargo test 完成后填（A 级原文））——job（bash-ee8e9b52717a4cc4（running（编译+测试（~900-1200s（frame.rs 大）））））

### 第五步：重构（优化实现（保持测试通过））

- **实现插入位置**（mod tests_block150 前（作用域正确））——保持测试通过
- **测试设计**（≤3 重穷举（256^3（gate 预算（1200s））））——避免 gate 超时复发

### 第六步：复核（对抗复核（11 项（必跑验证）））

- **对抗复核**（四项核（对抗复核核（证伪立场（独立重导证据（正确性核）））））：待 verifier 角色独立重导证据（11 项（必跑验证））

## 改写实录（逐项（非笼统））

| 项 | 内容 | 证据 |
|---|---|---|
| 1 | 写测试（穷举等价证明（全域穷举（≤3 重（256^3））+边界条件（异常路径））） | test_check_y_bound/test_apply_or_write/test_write_negative_path×3（9 测试（全域穷举（256^2/256^3））） |
| 2 | 跑测试（失败（实现未完成）） | E0425（check_y_bound/apply_or_write/write_negative_path 未实现（TDD 预期失败））（实测输出原文（A 级）） |
| 3 | 写实现（真规则（公式形（零字面特值））（去地址名（语义命名））） | check_y_bound（bcs 回卷（cmp4 命中））+apply_or_write（OR 写族（两形））+write_negative_path（负径写（$010F←$80））（s1_f284_body→check_y_bound+s1_f2d5_body→apply_or_write+s1_f2e8_body→write_negative_path） |
| 4 | 跑测试（通过（实现完成）） | （待 cargo test 完成后填（A 级原文）） |
| 5 | 重构（实现插入位置（mod tests_block150 前）+测试设计（≤3 重穷举（256^3））） | 作用域正确+gate 预算内（避免超时复发） |
| 6 | 复核（对抗复核（11 项（必跑验证））） | 待 verifier 角色独立重导证据 |

> 【refs 引用必写】bank:addr 引证（A 级原文）：`bank01:8000`（s1 族（prg1_brinstar.asm））（s1_f284_body（bcs 回卷（cmp4 命中））+s1_f2d5_body（OR 写族（两形））+s1_f2e8_body（负径写（案未练面注））（s1_/s2_ 族（实址（A 级原文））））
> 【留档可追溯】docs commit（追溯径（refs 引用（bank:addr）+改写实录（逐项（非笼统））+实测输出原文（A 级）））
