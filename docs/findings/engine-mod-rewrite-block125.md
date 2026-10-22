# 1:1 Rust 改写·批产·块 125：engine::mod（s1_/s2_ 族未处理（剩余（3 员/批）））（改写实录）

> 【状态】1:1 Rust 改写（A 级实录）——s1_/s2_ 族未处理 107 条（剩余（3 员/批（块 125）））→`engine/frame.rs`（语义命名））
> 【refs 引用必写】bank:addr 引证（A 级原文）——纪律强化（用户裁定）+四项核（对抗复核核（证伪立场））+TDD（测试先行（实现后补）（强制完整））+实现复核（必须保证有效）+worktree 隔离（独立文件（避免并发写入）））
> 【生成】TDD 流程（测试先行（实现后补）（强制完整））→改写实录（逐项（非笼统））

## TDD 流程（测试先行（实现后补）（强制完整））

### 第一步：写测试（穷举等价证明（全域穷举（边界条件（异常路径））））

- **测试 1-3**（穷举等价证明（全域穷举））：`test_apply_samus_table_write_full_domain/boundary/exception_path`（apply_samus_table_write 全域穷举（a_in/x/tbl_val/v6afa_x 全域（0..=255u8×4）））
- **测试 4-6**（穷举等价证明（全域穷举））：`test_apply_samus_table_write_y_full_domain/boundary/exception_path`（apply_samus_table_write_y 全域穷举（a_in/y/tbl_entry 全域（0..=255u8×3）））
- **测试 7-9**（穷举等价证明（全域穷举））：`test_apply_samus_table_write_tail_full_domain/boundary/exception_path`（apply_samus_table_write_tail 全域穷举（a_in/x 全域（0..=255u8×2）））

### 第二步：跑测试（cargo test（实测输出原文（A 级）））→**失败**（实现未完成）

```
error: could not compile `game-core` (lib test) due to 12 previous errors; 653 warnings emitted
E0425（apply_samus_table_write/apply_samus_table_write_y/apply_samus_table_write_tail 未实现（TDD 预期失败（实现未完成）））
```

### 第三步：写实现（真规则（公式形（零字面特值））（refs asm 考证（bank01:8000（s1 族（prg1_brinstar.asm）））（**去地址名**（s1_f6a4_body→apply_samus_table_write（语义命名）+s1_f8a2_body→apply_samus_table_write_y（语义命名）+s1_fa0a_body→apply_samus_table_write_tail（语义命名））））

- **真规则**（公式形（零字面特值））：`apply_samus_table_write`（tbl_val & 0x7F（掩码）+写族（0x6AF9/0x6AF8/0x6AFA（wrapping_sub(n)）））+`apply_samus_table_write_y`（写族（0x6AF9/0x6AFA/0x6AF8/0x6AF4/0x0403/0x0402（6 写事件）））+`apply_samus_table_write_tail`（写族（0x6AF9/0x6AFA/0x6AF8/0x0409）+尾跳（0xF97C））
- **refs asm 考证**（bank:addr（bank01:8000（s1 族（prg1_brinstar.asm））（s1_f6a4_body（tbl_val & 0x7F（掩码）+0x6AF9 + x as u16（写族）+0x6AFA + x as u16（wrapping_sub(n)）））+s1_f8a2_body（root+$6AF4[y]←1+$F92A 表载→$0403[y]+$0402[y]←0）+s1_fa0a_body（root+$0409[x]←$0A→尾跳 $F97C））（s1_/s2_ 族（实址（A 级原文）））））——s1_/s2_ 族（实址（非占位））
- **去地址名**（语义命名（data/naming 台账 A/B 级））：`s1_f6a4_body`（engine/mod.rs:441）→`apply_samus_table_write`（engine/frame.rs）+`s1_f8a2_body`（engine/mod.rs:449）→`apply_samus_table_write_y`（engine/frame.rs）+`s1_fa0a_body`（engine/mod.rs:455）→`apply_samus_table_write_tail`（engine/frame.rs）

### 第四步：跑测试（cargo test（实测输出原文（A 级）））→**通过**（实现完成）

```
test result: ok. 1154 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 551.14s
```

### 第五步：重构（优化实现（保持测试通过））

- **实现插入位置**（mod tests_block125 前（作用域正确））——保持测试通过

### 第六步：复核（对抗复核（11 项（必跑验证）））

- **对抗复核**（四项核（对抗复核核（证伪立场（独立重导证据（正确性核）））））：待 verifier 角色独立重导证据（11 项（必跑验证））

## 改写实录（逐项（非笼统））

| 项 | 内容 | 证据 |
|---|---|---|
| 1 | 写测试（穷举等价证明（全域穷举（边界条件（异常路径）））） | test_apply_samus_table_write/test_apply_samus_table_write_y/test_apply_samus_table_write_tail×3（9 测试（全域穷举（0..=255u8×N））） |
| 2 | 跑测试（失败（实现未完成）） | E0425（apply_samus_table_write/apply_samus_table_write_y/apply_samus_table_write_tail 未实现（TDD 预期失败））（实测输出原文（A 级）） |
| 3 | 写实现（真规则（公式形（零字面特值））（去地址名（语义命名））） | apply_samus_table_write（tbl_val & 0x7F（掩码）+写族（wrapping_sub(n)））+apply_samus_table_write_y（写族（6 写事件））+apply_samus_table_write_tail（写族+尾跳（0xF97C））（s1_f6a4_body→apply_samus_table_write+s1_f8a2_body→apply_samus_table_write_y+s1_fa0a_body→apply_samus_table_write_tail） |
| 4 | 跑测试（通过（实现完成）） | test result: ok. 1154 passed; 0 failed（实测输出原文（A 级）） |
| 5 | 重构（实现插入位置（mod tests_block125 前）） | 作用域正确（保持测试通过） |
| 6 | 复核（对抗复核（11 项（必跑验证））） | 待 verifier 角色独立重导证据 |

> 【refs 引用必写】bank:addr 引证（A 级原文）：`bank01:8000`（s1 族（prg1_brinstar.asm））（s1_f6a4_body（tbl_val & 0x7F（掩码）+写族（wrapping_sub(n)））+s1_f8a2_body（写族（6 写事件））+s1_fa0a_body（写族+尾跳（0xF97C））（s1_/s2_ 族（实址（A 级原文））））
> 【留档可追溯】docs commit（追溯径（refs 引用（bank:addr）+改写实录（逐项（非笼统））+实测输出原文（A 级）））
