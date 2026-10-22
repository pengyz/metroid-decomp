# 1:1 Rust 改写·批产·块 131：engine::mod（s1_/s2_ 族未处理（剩余（3 员/批）））（改写实录）

> 【状态】1:1 Rust 改写（A 级实录）——s1_/s2_ 族未处理 104 条（剩余（3 员/批（块 131）））→`engine/frame.rs`（语义命名））
> 【refs 引用必写】bank:addr 引证（A 级原文）——纪律强化（用户裁定）+四项核（对抗复核核（证伪立场））+TDD（测试先行（实现后补）（强制完整））+实现复核（必须保证有效）+worktree 隔离（独立文件（避免并发写入）））
> 【生成】TDD 流程（测试先行（实现后补）（强制完整））→改写实录（逐项（非笼统））

## TDD 流程（测试先行（实现后补）（强制完整））

### 第一步：写测试（穷举等价证明（全域穷举（边界条件（异常路径））））

- **测试 1-3**（穷举等价证明（全域穷举））：`test_apply_samus_slot_search_full_domain/boundary/exception_path`（apply_samus_slot_search 全域穷举（a_in/x 全域（0..=255u8×2）））
- **测试 4-6**（穷举等价证明（全域穷举））：`test_check_samus_table_compare_full_domain/boundary/exception_path`（check_samus_table_compare 全域穷举（y/x/tbl_val/v6af9_x 全域（0..=255u8×4）））
- **测试 7-9**（穷举等价证明（全域穷举））：`test_dispatch_samus_anim_base_full_domain/boundary/exception_path`（dispatch_samus_anim_base 全域穷举（a_in/x/v0409_x 全域（0..=255u8×3）））

### 第二步：跑测试（cargo test（实测输出原文（A 级）））→**失败**（实现未完成）

```
error: could not compile `game-core` (lib test) due to 13 previous errors; 658 warnings emitted
E0282/E0425（apply_samus_slot_search/check_samus_table_compare/dispatch_samus_anim_base 未实现（TDD 预期失败（实现未完成）））
```

### 第三步：写实现（真规则（公式形（零字面特值））（refs asm 考证（bank01:8000（s1 族（prg1_brinstar.asm）））（**去地址名**（s1_fa97_body→apply_samus_slot_search（语义命名）+s1_fbc7_body→check_samus_table_compare（语义命名）+s1_f3d5_body→dispatch_samus_anim_base（语义命名））））

- **真规则**（公式形（零字面特值））：`apply_samus_slot_search`（写族（0x6AF9/0x6AFA/0x6AF8）+尾跳（0xF97C））+`check_samus_table_compare`（比对段（等⇒回卷（0xF62E）；不等⇒写+jmp $F690））+`dispatch_samus_anim_base`（s1_f676 调用+写族（0x6AF9/0x6AFA/0x6AF8）+尾跳面（0xF7BA/0xF40A））
- **refs asm 考证**（bank:addr（bank01:8000（s1 族（prg1_brinstar.asm））（s1_fa97_body（root→尾跳 $F97C+槽搜族（$C0 by 8——$FAB4 调面注）））+s1_fbc7_body（jmp root（尾形）+比对段（$965B[y] vs $6AF9[x]——等⇒回卷/不等⇒写+jmp $F690））+s1_f3d5_body（s1_f676 调用+写族（0x6AF9/0x6AFA/0x6AF8）+尾跳面（0xF7BA/0xF40A）））（s1_/s2_ 族（实址（A 级原文）））））——s1_/s2_ 族（实址（非占位））
- **去地址名**（语义命名（data/naming 台账 A/B 级））：`s1_fa97_body`（engine/mod.rs:461）→`apply_samus_slot_search`（engine/frame.rs）+`s1_fbc7_body`（engine/mod.rs:468）→`check_samus_table_compare`（engine/frame.rs）+`s1_f3d5_body`（engine/mod.rs:512）→`dispatch_samus_anim_base`（engine/frame.rs）

### 第四步：跑测试（cargo test（实测输出原文（A 级）））→**通过**（实现完成）

```
test result: ok. 1171 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 647.50s
```

### 第五步：重构（优化实现（保持测试通过））

- **实现插入位置**（mod tests_block131 前（作用域正确））——保持测试通过

### 第六步：复核（对抗复核（11 项（必跑验证）））

- **对抗复核**（四项核（对抗复核核（证伪立场（独立重导证据（正确性核）））））：待 verifier 角色独立重导证据（11 项（必跑验证））

## 改写实录（逐项（非笼统））

| 项 | 内容 | 证据 |
|---|---|---|
| 1 | 写测试（穷举等价证明（全域穷举（边界条件（异常路径）））） | test_apply_samus_slot_search/test_check_samus_table_compare/test_dispatch_samus_anim_base×3（9 测试（全域穷举（0..=255u8×N））） |
| 2 | 跑测试（失败（实现未完成）） | E0282/E0425（apply_samus_slot_search/check_samus_table_compare/dispatch_samus_anim_base 未实现（TDD 预期失败））（实测输出原文（A 级）） |
| 3 | 写实现（真规则（公式形（零字面特值））（去地址名（语义命名））） | apply_samus_slot_search（写族+尾跳（0xF97C））+check_samus_table_compare（比对段（等⇒回卷（0xF62E）；不等⇒写+jmp $F690））+dispatch_samus_anim_base（s1_f676 调用+写族+尾跳面（0xF7BA/0xF40A））（s1_fa97_body→apply_samus_slot_search+s1_fbc7_body→check_samus_table_compare+s1_f3d5_body→dispatch_samus_anim_base） |
| 4 | 跑测试（通过（实现完成）） | test result: ok. 1171 passed; 0 failed（实测输出原文（A 级）） |
| 5 | 重构（实现插入位置（mod tests_block131 前）） | 作用域正确（保持测试通过） |
| 6 | 复核（对抗复核（11 项（必跑验证））） | 待 verifier 角色独立重导证据 |

> 【refs 引用必写】bank:addr 引证（A 级原文）：`bank01:8000`（s1 族（prg1_brinstar.asm））（s1_fa97_body（root→尾跳 $F97C+槽搜族（$C0 by 8——$FAB4 调面注））+s1_fbc7_body（jmp root（尾形）+比对段（$965B[y] vs $6AF9[x]——等⇒回卷/不等⇒写+jmp $F690））+s1_f3d5_body（s1_f676 调用+写族（0x6AF9/0x6AFA/0x6AF8）+尾跳面（0xF7BA/0xF40A））（s1_/s2_ 族（实址（A 级原文））））
> 【留档可追溯】docs commit（追溯径（refs 引用（bank:addr）+改写实录（逐项（非笼统））+实测输出原文（A 级）））
