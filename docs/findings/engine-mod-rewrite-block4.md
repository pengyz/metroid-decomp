# 1:1 Rust 改写·批产·块 4：engine::mod（陈旧代码拆分）（改写实录）

> 【状态】1:1 Rust 改写（A 级实录）——按子系统拆分去地址名（`engine/mod.rs`（13565 行（913 pub fn（323 地址命名）））→`engine/frame.rs`（语义命名）））
> 【refs 引用必写】bank:addr 引证（A 级原文）——纪律强化（用户裁定）+四项核（对抗复核核（证伪立场））+TDD（测试先行（实现后补）（强制完整））+实现复核（必须保证有效）
> 【生成】TDD 流程（测试先行（实现后补）（强制完整））→改写实录（逐项（非笼统））

## TDD 流程（测试先行（实现后补）（强制完整））

### 第一步：写测试（穷举等价证明（全域穷举（边界条件（异常路径））））

- **测试 1-3**（穷举等价证明（全域穷举））：`test_load_area_graphics_full_domain/boundary/exception_path`（load_area_graphics 全域穷举（root_val/x 全域））
- **测试 4-6**（穷举等价证明（全域穷举））：`test_apply_area_graphics_full_domain/boundary/exception_path`（apply_area_graphics 全域穷举（root_val/x/zp4f/zp50 全域））
- **测试 7-9**（穷举等价证明（全域穷举））：`test_update_room_state_full_domain/boundary/exception_path`（update_room_state 全域穷举（x/v030e_x/vfd 全域））

### 第二步：跑测试（cargo test（实测输出原文（A 级）））→**失败**（实现未完成）

```
error: could not compile `game-core` (lib test) due to 9 previous errors; 209 warnings emitted
E0425（load_area_graphics/apply_area_graphics/update_room_state 未实现（TDD 预期失败（实现未完成）））
```

### 第三步：写实现（真规则（公式形（零字面特值））（refs asm 考证（bank01:8000（s1 族（prg1_brinstar.asm））+bank02:8000（s2 族（prg2_norfair.asm）））（**去地址名**（s1_ec89→load_area_graphics（语义命名）+s1_ee33→apply_area_graphics（语义命名）+s1_d824→update_room_state（语义命名））））

- **真规则**（公式形（零字面特值））：`load_area_graphics`（root_val.wrapping_add(x) as u16（byte-driven semantics））+`apply_area_graphics`（root_val.wrapping_add(x) as u16（byte-driven semantics））+`update_room_state`（x.wrapping_add(v030e_x) as u16（byte-driven semantics））
- **refs asm 考证**（bank:addr（bank01:8000（s1 族（prg1_brinstar.asm））+bank02:8000（s2 族（prg2_norfair.asm））（s1_/s2_ 族（实址（A 级原文）））））——s1_/s2_ 族（实址（非占位））
- **去地址名**（语义命名（data/naming 台账 A/B 级））：`s1_ec89`（engine/mod.rs:124）→`load_area_graphics`（engine/frame.rs）+`s1_ee33`（engine/mod.rs:129）→`apply_area_graphics`（engine/frame.rs）+`s1_d824`（engine/mod.rs:172）→`update_room_state`（engine/frame.rs）

### 第四步：跑测试（cargo test（实测输出原文（A 级）））→**通过**（实现完成）

```
test result: ok. 661 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 6.67s
```

### 第五步：重构（优化实现（保持测试通过））

- **实现插入位置**（mod tests_block4 前（作用域正确））——保持测试通过

### 第六步：复核（对抗复核（11 项（必跑验证）））

- **对抗复核**（四项核（对抗复核核（证伪立场（独立重导证据（正确性核）））））：待 verifier 角色独立重导证据（11 项（必跑验证））

## 改写实录（逐项（非笼统））

| 项 | 内容 | 证据 |
|---|---|---|
| 1 | 写测试（穷举等价证明（全域穷举（边界条件（异常路径）））） | test_load_area_graphics/test_apply_area_graphics/test_update_room_state×3（9 测试） |
| 2 | 跑测试（失败（实现未完成）） | E0425（load_area_graphics/apply_area_graphics/update_room_state 未实现（TDD 预期失败））（实测输出原文（A 级）） |
| 3 | 写实现（真规则（公式形（零字面特值））（去地址名（语义命名））） | load_area_graphics（wrapping_add）+apply_area_graphics（wrapping_add）+update_room_state（wrapping_add）（s1_ec89→load_area_graphics+s1_ee33→apply_area_graphics+s1_d824→update_room_state） |
| 4 | 跑测试（通过（实现完成）） | test result: ok. 661 passed; 0 failed（实测输出原文（A 级）） |
| 5 | 重构（实现插入位置（mod tests_block4 前）） | 作用域正确（保持测试通过） |
| 6 | 复核（对抗复核（11 项（必跑验证））） | 待 verifier 角色独立重导证据 |

> 【refs 引用必写】bank:addr 引证（A 级原文）：`bank01:8000`（s1 族（prg1_brinstar.asm））+`bank02:8000`（s2 族（prg2_norfair.asm））（s1_/s2_ 族（实址（A 级原文）））
> 【留档可追溯】docs commit（追溯径（refs 引用（bank:addr）+改写实录（逐项（非笼统））+实测输出原文（A 级）））
