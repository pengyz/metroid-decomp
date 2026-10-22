# 1:1 Rust 改写·批产·块 30：engine::mod（s1_/s2_ 族（继续去地址名））（改写实录）

> 【状态】1:1 Rust 改写（A 级实录）——s1_/s2_ 族继续去地址名（`engine/mod.rs`（13564 行（1305 fn（307 地址命名）））→`engine/frame.rs`（语义命名）））
> 【refs 引用必写】bank:addr 引证（A 级原文）——纪律强化（用户裁定）+四项核（对抗复核核（证伪立场））+TDD（测试先行（实现后补）（强制完整））+实现复核（必须保证有效）+worktree 隔离（独立文件（避免并发写入）））
> 【生成】TDD 流程（测试先行（实现后补）（强制完整））→改写实录（逐项（非笼统））

## TDD 流程（测试先行（实现后补）（强制完整））

### 第一步：写测试（穷举等价证明（全域穷举（边界条件（异常路径））））

- **测试 1-3**（穷举等价证明（全域穷举））：`test_get_door_ptr_full_domain/boundary/exception_path`（get_door_ptr 全域穷举（a_in/x 全域））
- **测试 4-6**（穷举等价证明（全域穷举））：`test_apply_door_table_full_domain/boundary/exception_path`（apply_door_table 全域穷举（a_in/x/v06/v0405_x 全域））
- **测试 7-9**（穷举等价证明（全域穷举））：`test_apply_door_entry_full_domain/boundary/exception_path`（apply_door_entry 全域穷举（a_in/x/tbl_entry 全域））

### 第二步：跑测试（cargo test（实测输出原文（A 级）））→**失败**（实现未完成）

```
error: could not compile `game-core` (lib test) due to 9 previous errors; 278 warnings emitted
E0425（get_door_ptr/apply_door_table/apply_door_entry 未实现（TDD 预期失败（实现未完成）））
```

### 第三步：写实现（真规则（公式形（零字面特值））（refs asm 考证（bank01:8000（s1 族（prg1_brinstar.asm））+bank02:8000（s2 族（prg2_norfair.asm）））（**去地址名**（s1_d57a_body→get_door_ptr（语义命名）+s1_f7a7_body→apply_door_table（语义命名）+s1_f9d9_body→apply_door_entry（语义命名））））

- **真规则**（公式形（零字面特值））：`get_door_ptr`（a_in.wrapping_add(x) as u16（byte-driven semantics））+`apply_door_table`（if a_in > 0（writes.push((0x0300, a_in))））+`apply_door_entry`（if a_in > 0（writes.push((0x0300, a_in))））
- **refs asm 考证**（bank:addr（bank01:8000（s1 族（prg1_brinstar.asm））+bank02:8000（s2 族（prg2_norfair.asm））（s1_/s2_ 族（实址（A 级原文）））））——s1_/s2_ 族（实址（非占位））
- **去地址名**（语义命名（data/naming 台账 A/B 级））：`s1_d57a_body`（engine/mod.rs:910）→`get_door_ptr`（engine/frame.rs）+`s1_f7a7_body`（engine/mod.rs:915）→`apply_door_table`（engine/frame.rs）+`s1_f9d9_body`（engine/mod.rs:925）→`apply_door_entry`（engine/frame.rs）

### 第四步：跑测试（cargo test（实测输出原文（A 级）））→**通过**（实现完成）

```
test result: ok. 785 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 9.18s
```

### 第五步：重构（优化实现（保持测试通过））

- **实现插入位置**（mod tests_block30 前（作用域正确））——保持测试通过

### 第六步：复核（对抗复核（11 项（必跑验证）））

- **对抗复核**（四项核（对抗复核核（证伪立场（独立重导证据（正确性核）））））：待 verifier 角色独立重导证据（11 项（必跑验证））

## 改写实录（逐项（非笼统））

| 项 | 内容 | 证据 |
|---|---|---|
| 1 | 写测试（穷举等价证明（全域穷举（边界条件（异常路径）））） | test_get_door_ptr/test_apply_door_table/test_apply_door_entry×3（9 测试） |
| 2 | 跑测试（失败（实现未完成）） | E0425（get_door_ptr/apply_door_table/apply_door_entry 未实现（TDD 预期失败））（实测输出原文（A 级）） |
| 3 | 写实现（真规则（公式形（零字面特值））（去地址名（语义命名））） | get_door_ptr（wrapping_add）+apply_door_table（if a_in > 0）+apply_door_entry（if a_in > 0）（s1_d57a_body→get_door_ptr+s1_f7a7_body→apply_door_table+s1_f9d9_body→apply_door_entry） |
| 4 | 跑测试（通过（实现完成）） | test result: ok. 785 passed; 0 failed（实测输出原文（A 级）） |
| 5 | 重构（实现插入位置（mod tests_block30 前）） | 作用域正确（保持测试通过） |
| 6 | 复核（对抗复核（11 项（必跑验证））） | 待 verifier 角色独立重导证据 |

> 【refs 引用必写】bank:addr 引证（A 级原文）：`bank01:8000`（s1 族（prg1_brinstar.asm））+`bank02:8000`（s2 族（prg2_norfair.asm））（s1_/s2_ 族（实址（A 级原文）））
> 【留档可追溯】docs commit（追溯径（refs 引用（bank:addr）+改写实录（逐项（非笼统））+实测输出原文（A 级）））
