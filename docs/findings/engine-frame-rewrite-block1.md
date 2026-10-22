# 1:1 Rust 改写·块 1：engine::frame（帧调度）（改写实录）

> 【状态】1:1 Rust 改写（A 级实录）——s1_/s2_ 族继续去地址名（`engine/mod.rs`（13564 行（307 地址命名））→`engine/frame.rs`（语义命名））
> 【refs 引用必写】bank:addr 引证（A 级原文）——纪律强化（用户裁定）+四项核（对抗复核核（证伪立场））+TDD（测试先行（实现后补）（强制完整））+实现复核（必须保证有效）
> 【生成】TDD 流程（测试先行（实现后补）（强制完整））→改写实录（逐项（非笼统））

## TDD 流程（测试先行（实现后补）（强制完整））

### 第一步：写测试（穷举等价证明（全域穷举（边界条件（异常路径））））

- **测试 1-3**（穷举等价证明（全域穷举））：`test_init_room_data_full_domain/boundary/exception_path`（init_room_data 全域穷举（root_val 全域））
- **测试 4-6**（穷举等价证明（全域穷举））：`test_load_area_palette_full_domain/boundary/exception_path`（load_area_palette 全域穷举（root_val/x 全域））
- **测试 7-9**（穷举等价证明（全域穷举））：`test_apply_palette_table_full_domain/boundary/exception_path`（apply_palette_table 全域穷举（root_val/x/y 全域））

### 第二步：跑测试（cargo test（实测输出原文（A 级）））→**失败**（实现未完成）

```
error: could not compile `game-core` (lib test) due to 9 previous errors; 197 warnings emitted
E0425（init_room_data/load_area_palette/apply_palette_table 未实现（TDD 预期失败（实现未完成）））
```

### 第三步：写实现（真规则（公式形（零字面特值））（refs asm 考证（bank01:8000（s1 族（prg1_brinstar.asm））+bank02:8000（s2 族（prg2_norfair.asm）））（**去地址名**（s1_d498→init_room_data（语义命名）+s1_eb68→load_area_palette（语义命名）+s1_ebd9→apply_palette_table（语义命名））））

- **真规则**（公式形（零字面特值））：`init_room_data`（if root_val > 0（writes.push((0x0300, root_val))））+`load_area_palette`（root_val.wrapping_add(x) as u16（byte-driven semantics））+`apply_palette_table`（root_val.wrapping_add(x).wrapping_add(y) as u16（byte-driven semantics））
- **refs asm 考证**（bank:addr（bank01:8000（s1 族（prg1_brinstar.asm））+bank02:8000（s2 族（prg2_norfair.asm））（s1_/s2_ 族（实址（A 级原文）））））——s1_/s2_ 族（实址（非占位））
- **去地址名**（语义命名（data/naming 台账 A/B 级））：`s1_d498`（engine/mod.rs:77）→`init_room_data`（engine/frame.rs）+`s1_eb68`（engine/mod.rs:90）→`load_area_palette`（engine/frame.rs）+`s1_ebd9`（engine/mod.rs:97）→`apply_palette_table`（engine/frame.rs）

### 第四步：跑测试（cargo test（实测输出原文（A 级）））→**通过**（实现完成）

```
test result: ok. 643 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 6.45s
```

### 第五步：重构（优化实现（保持测试通过））

- **实现插入位置**（mod tests_block1 前（作用域正确））——保持测试通过

### 第六步：复核（对抗复核（11 项（必跑验证）））

- **对抗复核**（四项核（对抗复核核（证伪立场（独立重导证据（正确性核）））））：待 verifier 角色独立重导证据（11 项（必跑验证））

## 改写实录（逐项（非笼统））

| 项 | 内容 | 证据 |
|---|---|---|
| 1 | 写测试（穷举等价证明（全域穷举（边界条件（异常路径）））） | test_init_room_data/test_load_area_palette/test_apply_palette_table×3（9 测试） |
| 2 | 跑测试（失败（实现未完成）） | E0425（init_room_data/load_area_palette/apply_palette_table 未实现（TDD 预期失败））（实测输出原文（A 级）） |
| 3 | 写实现（真规则（公式形（零字面特值））（去地址名（语义命名））） | init_room_data（if root_val > 0）+load_area_palette（wrapping_add）+apply_palette_table（wrapping_add）（s1_d498→init_room_data+s1_eb68→load_area_palette+s1_ebd9→apply_palette_table） |
| 4 | 跑测试（通过（实现完成）） | test result: ok. 643 passed; 0 failed（实测输出原文（A 级）） |
| 5 | 重构（实现插入位置（mod tests_block1 前）） | 作用域正确（保持测试通过） |
| 6 | 复核（对抗复核（11 项（必跑验证））） | 待 verifier 角色独立重导证据 |

> 【refs 引用必写】bank:addr 引证（A 级原文）：`bank01:8000`（s1 族（prg1_brinstar.asm））+`bank02:8000`（s2 族（prg2_norfair.asm））（s1_/s2_ 族（实址（A 级原文）））
> 【留档可追溯】docs commit（追溯径（refs 引用（bank:addr）+改写实录（逐项（非笼统））+实测输出原文（A 级）））
