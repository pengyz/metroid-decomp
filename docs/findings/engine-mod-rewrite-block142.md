# 1:1 Rust 改写·批产·块 142：engine::mod（s1_/s2_ 族未处理（剩余（3 员/批）））（改写实录）

> 【状态】1:1 Rust 改写（A 级实录）——s1_/s2_ 族未处理 101 条（剩余（3 员/批（块 142）））→`engine/frame.rs`（语义命名））
> 【refs 引用必写】bank:addr 引证（A 级原文）——纪律强化（用户裁定）+四项核（对抗复核核（证伪立场））+TDD（测试先行（实现后补）（强制完整））+实现复核（必须保证有效）+worktree 隔离（独立文件（避免并发写入）））
> 【生成】TDD 流程（测试先行（实现后补）（强制完整））→改写实录（逐项（非笼统））
> 【测试设计】穷举等价证明（≤3 重穷举（256^3（gate 预算（1200s（选项②（lead 裁定（协调者执行（3fc0c7f0））））））+边界条件（异常路径））——4 参数函数（3 重穷举+第 4 参数边界/异常（避免 gate 超时复发））

## TDD 流程（测试先行（实现后补）（强制完整））

### 第一步：写测试（穷举等价证明（全域穷举（≤3 重（256^3（gate 预算（1200s）））+边界条件（异常路径））））

- **测试 1-3**（穷举等价证明（全域穷举））：`test_get_table_ptr_full_domain/boundary/exception_path`（get_table_ptr 全域穷举（y/tbl_entry 全域（256^2）））
- **测试 4-6**（穷举等价证明（全域穷举））：`test_get_enemy_type_facing_full_domain/boundary/exception_path`（get_enemy_type_facing 全域穷举（x/v0405_x/v6b02_x 全域（256^3）））
- **测试 7-9**（穷举等价证明（全域穷举））：`test_apply_enemy_inc_dec_full_domain/boundary/exception_path`（apply_enemy_inc_dec 全域穷举（x/v6b01_x 全域（256^2）））

### 第二步：跑测试（cargo test（实测输出原文（A 级）））→**失败**（实现未完成）

```
error: could not compile `game-core` (lib test) due to 12 previous errors; 669 warnings emitted
E0425（get_table_ptr/get_enemy_type_facing/apply_enemy_inc_dec 未实现（TDD 预期失败（实现未完成）））
```

### 第三步：写实现（真规则（公式形（零字面特值））（refs asm 考证（bank01:8000（s1 族（prg1_brinstar.asm）））（**去地址名**（s1_f7d9_body→get_table_ptr（语义命名）+s1_f844_body→get_enemy_type_facing（语义命名）+s1_fb8a_body→apply_enemy_inc_dec（语义命名））））

- **真规则**（公式形（零字面特值））：`get_table_ptr`（0x97D1u16.wrapping_add（表指针）+写族（(ptr & 0xFF)/(ptr >> 8)））+`get_enemy_type_facing`（进位链（末 lsr 出位）+rol（(v6b02_x << 1) | carry））+`apply_enemy_inc_dec`（inc/dec 互抵（净效 0）+条件 $C3D4（TwosCompliment））
- **refs asm 考证**（bank:addr（bank01:8000（s1 族（prg1_brinstar.asm））（s1_f7d9_body（0x97D1u16.wrapping_add（表指针）+写族（(ptr & 0xFF)/(ptr >> 8)）））+s1_f844_body（$0405,x 符号（bpl 不移/bmi 三 lsr（末 lsr 出位））+lda $6B02,x（替换 A）+rol（过进位）→Y）+s1_fb8a_body（$F844 计算+$6B01,x 读+$6B03,x inc/dec 互抵+条件 $C3D4（TwosCompliment）+cmp #8 分段））（s1_/s2_ 族（实址（A 级原文）））））——s1_/s2_ 族（实址（非占位））
- **去地址名**（语义命名（data/naming 台账 A/B 级））：`s1_f7d9_body`（engine/mod.rs:550）→`get_table_ptr`（engine/frame.rs）+`s1_f844_body`（engine/mod.rs:558）→`get_enemy_type_facing`（engine/frame.rs）+`s1_fb8a_body`（engine/mod.rs:569）→`apply_enemy_inc_dec`（engine/frame.rs）

### 第四步：跑测试（cargo test（实测输出原文（A 级）））→**通过**（实现完成）

```
test result: ok. 1195 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 463.96s
```

### 第五步：重构（优化实现（保持测试通过））

- **实现插入位置**（mod tests_block142 前（作用域正确））——保持测试通过
- **测试设计**（≤3 重穷举（256^3（gate 预算（1200s））））——避免 gate 超时复发（4 参数函数（3 重穷举+第 4 参数边界/异常））

### 第六步：复核（对抗复核（11 项（必跑验证）））

- **对抗复核**（四项核（对抗复核核（证伪立场（独立重导证据（正确性核）））））：待 verifier 角色独立重导证据（11 项（必跑验证））

## 改写实录（逐项（非笼统））

| 项 | 内容 | 证据 |
|---|---|---|
| 1 | 写测试（穷举等价证明（全域穷举（≤3 重（256^3））+边界条件（异常路径））） | test_get_table_ptr/test_get_enemy_type_facing/test_apply_enemy_inc_dec×3（9 测试（全域穷举（256^2/256^3））） |
| 2 | 跑测试（失败（实现未完成）） | E0425（get_table_ptr/get_enemy_type_facing/apply_enemy_inc_dec 未实现（TDD 预期失败））（实测输出原文（A 级）） |
| 3 | 写实现（真规则（公式形（零字面特值））（去地址名（语义命名））） | get_table_ptr（0x97D1.wrapping_add（表指针））+get_enemy_type_facing（进位链（末 lsr 出位）+rol）+apply_enemy_inc_dec（inc/dec 互抵+条件 $C3D4）（s1_f7d9_body→get_table_ptr+s1_f844_body→get_enemy_type_facing+s1_fb8a_body→apply_enemy_inc_dec） |
| 4 | 跑测试（通过（实现完成）） | test result: ok. 1195 passed; 0 failed（实测输出原文（A 级）（463.96s（<1200s（gate 预算）））） |
| 5 | 重构（实现插入位置（mod tests_block142 前）+测试设计（≤3 重穷举（256^3））） | 作用域正确+gate 预算内（避免超时复发） |
| 6 | 复核（对抗复核（11 项（必跑验证））） | 待 verifier 角色独立重导证据 |

> 【refs 引用必写】bank:addr 引证（A 级原文）：`bank01:8000`（s1 族（prg1_brinstar.asm））（s1_f7d9_body（0x97D1.wrapping_add（表指针））+s1_f844_body（$0405,x 符号（bpl 不移/bmi 三 lsr（末 lsr 出位））+lda $6B02,x（替换 A）+rol（过进位）→Y）+s1_fb8a_body（inc/dec 互抵+条件 $C3D4（TwosCompliment））（s1_/s2_ 族（实址（A 级原文））））
> 【留档可追溯】docs commit（追溯径（refs 引用（bank:addr）+改写实录（逐项（非笼统））+实测输出原文（A 级）））
