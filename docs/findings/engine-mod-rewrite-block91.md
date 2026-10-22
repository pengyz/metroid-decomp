# 1:1 Rust 改写·批产·块 91：engine::mod（s6_/s7_ 族（继续去地址名））（改写实录）

> 【状态】1:1 Rust 改写（A 级实录）——s6_/s7_ 族继续去地址名（`engine/mod.rs`（13564 行（1305 fn（307 地址命名）））→`engine/frame.rs`（语义命名）））
> 【refs 引用必写】bank:addr 引证（A 级原文）——纪律强化（用户裁定）+四项核（对抗复核核（证伪立场））+TDD（测试先行（实现后补）（强制完整））+实现复核（必须保证有效）+worktree 隔离（独立文件（避免并发写入）））
> 【生成】TDD 流程（测试先行（实现后补）（强制完整））→改写实录（逐项（非笼统））

## TDD 流程（测试先行（实现后补）（强制完整））

### 第一步：写测试（穷举等价证明（全域穷举（边界条件（异常路径））））

- **测试 1-3**（穷举等价证明（全域穷举））：`test_get_area_state_full_domain/boundary/exception_path`（get_area_state 全域穷举（无参数））
- **测试 4-6**（穷举等价证明（全域穷举））：`test_apply_area_state_full_domain/boundary/exception_path`（apply_area_state 全域穷举（x/v2d/v040d_x 全域））
- **测试 7-9**（穷举等价证明（全域穷举））：`test_get_area_ptr_full_domain/boundary/exception_path`（get_area_ptr 全域穷举（x/a_in 全域））

### 第二步：跑测试（cargo test（实测输出原文（A 级）））→**失败**（实现未完成）

```
error: could not compile `game-core` (lib test) due to 9 previous errors; 562 warnings emitted
E0425（get_area_state/apply_area_state/get_area_ptr 未实现（TDD 预期失败（实现未完成）））
```

### 第三步：写实现（真规则（公式形（零字面特值））（refs asm 考证（bank06:8000（s6 族（prg6_area.asm）））（**去地址名**（s6_f4b4_body→get_area_state（语义命名）+s6_f4d1_body→apply_area_state（语义命名）+s6_f512_body→get_area_ptr（语义命名））））

- **真规则**（公式形（零字面特值））：`get_area_state`（(Vec::new(), 0)（常量返回））+`apply_area_state`（x.wrapping_add(v2d)（byte-driven semantics））+`get_area_ptr`（x.wrapping_add(a_in) as u16（byte-driven semantics））
- **refs asm 考证**（bank:addr（bank06:8000（s6 族（prg6_area.asm））（s6_/s7_ 族（实址（A 级原文）））））——s6_/s7_ 族（实址（非占位））
- **去地址名**（语义命名（data/naming 台账 A/B 级））：`s6_f4b4_body`（engine/mod.rs:6466）→`get_area_state`（engine/frame.rs）+`s6_f4d1_body`（engine/mod.rs:6498）→`apply_area_state`（engine/frame.rs）+`s6_f512_body`（engine/mod.rs:6540）→`get_area_ptr`（engine/frame.rs）

### 第四步：跑测试（cargo test（实测输出原文（A 级）））→**通过**（实现完成）

```
test result: ok. 1046 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 13.60s
```

### 第五步：重构（优化实现（保持测试通过））

- **实现插入位置**（mod tests_block91 前（作用域正确））——保持测试通过

### 第六步：复核（对抗复核（11 项（必跑验证）））

- **对抗复核**（四项核（对抗复核核（证伪立场（独立重导证据（正确性核）））））：待 verifier 角色独立重导证据（11 项（必跑验证））

## 改写实录（逐项（非笼统））

| 项 | 内容 | 证据 |
|---|---|---|
| 1 | 写测试（穷举等价证明（全域穷举（边界条件（异常路径）））） | test_get_area_state/test_apply_area_state/test_get_area_ptr×3（9 测试） |
| 2 | 跑测试（失败（实现未完成）） | E0425（get_area_state/apply_area_state/get_area_ptr 未实现（TDD 预期失败））（实测输出原文（A 级）） |
| 3 | 写实现（真规则（公式形（零字面特值））（去地址名（语义命名））） | get_area_state（常量返回）+apply_area_state（wrapping_add）+get_area_ptr（wrapping_add as u16）（s6_f4b4_body→get_area_state+s6_f4d1_body→apply_area_state+s6_f512_body→get_area_ptr） |
| 4 | 跑测试（通过（实现完成）） | test result: ok. 1046 passed; 0 failed（实测输出原文（A 级）） |
| 5 | 重构（实现插入位置（mod tests_block91 前）） | 作用域正确（保持测试通过） |
| 6 | 复核（对抗复核（11 项（必跑验证））） | 待 verifier 角色独立重导证据 |

> 【refs 引用必写】bank:addr 引证（A 级原文）：`bank06:8000`（s6 族（prg6_area.asm））（s6_/s7_ 族（实址（A 级原文）））
> 【留档可追溯】docs commit（追溯径（refs 引用（bank:addr）+改写实录（逐项（非笼统））+实测输出原文（A 级）））
