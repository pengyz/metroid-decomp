# 1:1 Rust 改写·批产·块 121：engine::mod（s1_/s2_ 族未处理 116 条（剩余（3 员/批）））（改写实录）

> 【状态】1:1 Rust 改写（A 级实录）——s1_/s2_ 族未处理 110 条（剩余（3 员/批（块 121）））→`engine/frame.rs`（语义命名））
> 【refs 引用必写】bank:addr 引证（A 级原文）——纪律强化（用户裁定）+四项核（对抗复核核（证伪立场））+TDD（测试先行（实现后补）（强制完整））+实现复核（必须保证有效）+worktree 隔离（独立文件（避免并发写入）））
> 【生成】TDD 流程（测试先行（实现后补）（强制完整））→改写实录（逐项（非笼统））

## TDD 流程（测试先行（实现后补）（强制完整））

### 第一步：写测试（穷举等价证明（全域穷举（边界条件（异常路径））））

- **测试 1-3**（穷举等价证明（全域穷举））：`test_check_samus_flag_reverse_full_domain/boundary/exception_path`（check_samus_flag_reverse 全域穷举（root_val/x 全域（0..=255u8×2）））
- **测试 4-6**（穷举等价证明（全域穷举））：`test_check_samus_flag_compare_full_domain/boundary/exception_path`（check_samus_flag_compare 全域穷举（root_val/x/v040e_x/v6af4_x 全域（0..=255u8×4）））
- **测试 7-9**（穷举等价证明（全域穷举））：`test_apply_samus_write_base_full_domain/boundary/exception_path`（apply_samus_write_base 全域穷举（a_in/x/v6af4_x/v0a 全域（0..=255u8×4）））

### 第二步：跑测试（cargo test（实测输出原文（A 级）））→**失败**（实现未完成）

```
error: could not compile `game-core` (lib test) due to 12 previous errors; 650 warnings emitted
E0425（check_samus_flag_reverse/check_samus_flag_compare/apply_samus_write_base 未实现（TDD 预期失败（实现未完成）））
```

### 第三步：写实现（真规则（公式形（零字面特值））（refs asm 考证（bank01:8000（s1 族（prg1_brinstar.asm）））（**去地址名**（s1_f56c→check_samus_flag_reverse（语义命名）+s1_f5ac→check_samus_flag_compare（语义命名）+s1_f649_body→apply_samus_write_base（语义命名））））

- **真规则**（公式形（零字面特值））：`check_samus_flag_reverse`（s1_flag_gate（旗门反向（未命中⇒Some(0xCBB8)（子调+尾跳）；命中⇒None（判别段））））+`check_samus_flag_compare`（s1_flag_gate（旗门（命中⇒v040e_x == 0x0B（判别）；未命中⇒v6af4_x == 0x04（判别））））+`apply_samus_write_base`（写族（0x6AF9/0x6AFA/0x6AF8/0x0406/0x6AF4/0x0407（6 写事件）））
- **refs asm 考证**（bank:addr（bank01:8000（s1 族（prg1_brinstar.asm））（s1_f56c（旗门反向（未命中⇒Some(0xCBB8)（子调+尾跳（$F42D 面注））；命中⇒None（判别段（$FF 比较面注））））+s1_f5ac（旗门（命中⇒$040E,x==$0B 判别；未命中⇒$6AF4,x==$04 判别））+s1_f649_body（0x6AF9 + x as u16（写族）+0x6AF4 + x as u16（wrapping_add(1)）+0x0407 + x as u16（if (v0a as i8) < 0（3）else（0））））（s1_/s2_ 族（实址（A 级原文）））））——s1_/s2_ 族（实址（非占位））
- **去地址名**（语义命名（data/naming 台账 A/B 级））：`s1_f56c`（engine/mod.rs:349）→`check_samus_flag_reverse`（engine/frame.rs）+`s1_f5ac`（engine/mod.rs:358）→`check_samus_flag_compare`（engine/frame.rs）+`s1_f649_body`（engine/mod.rs:427）→`apply_samus_write_base`（engine/frame.rs）

### 第四步：跑测试（cargo test（实测输出原文（A 级）））→**通过**（实现完成）

```
test result: ok. 1136 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 417.78s
```

### 第五步：重构（优化实现（保持测试通过））

- **实现插入位置**（mod tests_block121 前（作用域正确））——保持测试通过

### 第六步：复核（对抗复核（11 项（必跑验证）））

- **对抗复核**（四项核（对抗复核核（证伪立场（独立重导证据（正确性核）））））：待 verifier 角色独立重导证据（11 项（必跑验证））

## 改写实录（逐项（非笼统））

| 项 | 内容 | 证据 |
|---|---|---|
| 1 | 写测试（穷举等价证明（全域穷举（边界条件（异常路径）））） | test_check_samus_flag_reverse/test_check_samus_flag_compare/test_apply_samus_write_base×3（9 测试（全域穷举（0..=255u8×N））） |
| 2 | 跑测试（失败（实现未完成）） | E0425（check_samus_flag_reverse/check_samus_flag_compare/apply_samus_write_base 未实现（TDD 预期失败））（实测输出原文（A 级）） |
| 3 | 写实现（真规则（公式形（零字面特值））（去地址名（语义命名））） | check_samus_flag_reverse（旗门反向（未命中⇒Some(0xCBB8)；命中⇒None））+check_samus_flag_compare（旗门（命中⇒v040e_x == 0x0B；未命中⇒v6af4_x == 0x04））+apply_samus_write_base（写族（6 写事件））（s1_f56c→check_samus_flag_reverse+s1_f5ac→check_samus_flag_compare+s1_f649_body→apply_samus_write_base） |
| 4 | 跑测试（通过（实现完成）） | test result: ok. 1136 passed; 0 failed（实测输出原文（A 级）） |
| 5 | 重构（实现插入位置（mod tests_block121 前）） | 作用域正确（保持测试通过） |
| 6 | 复核（对抗复核（11 项（必跑验证））） | 待 verifier 角色独立重导证据 |

> 【refs 引用必写】bank:addr 引证（A 级原文）：`bank01:8000`（s1 族（prg1_brinstar.asm））（s1_f56c（旗门反向（未命中⇒Some(0xCBB8)；命中⇒None））+s1_f5ac（旗门（命中⇒$040E,x==$0B 判别；未命中⇒$6AF4,x==$04 判别））+s1_f649_body（写族（6 写事件））（s1_/s2_ 族（实址（A 级原文））））
> 【留档可追溯】docs commit（追溯径（refs 引用（bank:addr）+改写实录（逐项（非笼统））+实测输出原文（A 级）））
