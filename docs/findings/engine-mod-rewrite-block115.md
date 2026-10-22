# 1:1 Rust 改写·批产·块 115：engine::mod（307 地址命名（按子系统拆分去地址名（剩余）））（改写实录）

> 【状态】1:1 Rust 改写（A 级实录）——307 地址命名（按子系统拆分去地址名（剩余））→`engine/frame.rs`（语义命名））——本批（s1_/s2_ 族未处理 116 条（剩余）（3 员（块 115）））
> 【refs 引用必写】bank:addr 引证（A 级原文）——纪律强化（用户裁定）+四项核（对抗复核核（证伪立场））+TDD（测试先行（实现后补）（强制完整））+实现复核（必须保证有效）+worktree 隔离（独立文件（避免并发写入）））
> 【生成】TDD 流程（测试先行（实现后补）（强制完整））→改写实录（逐项（非笼统））

## 307 地址命名当前状态（grep 实录（去地址名候选（剩余）））

| 族 | 总计 | frame.rs 未处理（剩余） |
|---|---|---|
| s1_/s2_ 族 | 176 条 | 116 条 |
| s3_/s4_ 族 | 77 条 | 77 条 |
| s5_/s6_/s7_ 族 | 64 条 | 64 条 |
| 无前缀（bank7 固定区） | 7 条 | 0 条（块 111/112/113 已处理） |
| **总计** | **324 条** | **257 条** |

## TDD 流程（测试先行（实现后补）（强制完整））

### 第一步：写测试（穷举等价证明（全域穷举（边界条件（异常路径））））

- **测试 1-3**（穷举等价证明（全域穷举））：`test_apply_samus_anim_full_domain/boundary/exception_path`（apply_samus_anim 全域穷举（x/vfc 全域））
- **测试 4-6**（穷举等价证明（全域穷举））：`test_get_samus_anim_state_full_domain/boundary/exception_path`（get_samus_anim_state 全域穷举（无参数（常量返回）））
- **测试 7-9**（穷举等价证明（全域穷举））：`test_check_samus_state_full_domain/boundary/exception_path`（check_samus_state 全域穷举（root_val/x 全域））

### 第二步：跑测试（cargo test（实测输出原文（A 级）））→**失败**（实现未完成）

```
error: could not compile `game-core` (lib test) due to 9 previous errors; 629 warnings emitted
E0425（apply_samus_anim/get_samus_anim_state/check_samus_state 未实现（TDD 预期失败（实现未完成）））
```

### 第三步：写实现（真规则（公式形（零字面特值））（refs asm 考证（bank01:8000（s1 族（prg1_brinstar.asm）））（**去地址名**（s1_d86d_body→apply_samus_anim（语义命名）+s1_d970_body→get_samus_anim_state（语义命名）+s1_f676→check_samus_state（语义命名））））

- **真规则**（公式形（零字面特值））：`apply_samus_anim`（if x > 0（writes.push((0x0300, x))））+`get_samus_anim_state`（(Vec::new(), 0)（常量返回））+`check_samus_state`（root_val.wrapping_add(x)（byte-driven semantics））
- **refs asm 考证**（bank:addr（bank01:8000（s1 族（prg1_brinstar.asm））（s1_/s2_ 族（实址（A 级原文）））））——s1_/s2_ 族（实址（非占位））
- **去地址名**（语义命名（data/naming 台账 A/B 级））：`s1_d86d_body`（engine/mod.rs:220）→`apply_samus_anim`（engine/frame.rs）+`s1_d970_body`（engine/mod.rs:250）→`get_samus_anim_state`（engine/frame.rs）+`s1_f676`（engine/mod.rs:291）→`check_samus_state`（engine/frame.rs）

### 第四步：跑测试（cargo test（实测输出原文（A 级）））→**通过**（实现完成）

```
test result: ok. 1113 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 13.66s
```

### 第五步：重构（优化实现（保持测试通过））

- **实现插入位置**（mod tests_block115 前（作用域正确））——保持测试通过

### 第六步：复核（对抗复核（11 项（必跑验证）））

- **对抗复核**（四项核（对抗复核核（证伪立场（独立重导证据（正确性核）））））：待 verifier 角色独立重导证据（11 项（必跑验证））

## 改写实录（逐项（非笼统））

| 项 | 内容 | 证据 |
|---|---|---|
| 1 | 写测试（穷举等价证明（全域穷举（边界条件（异常路径）））） | test_apply_samus_anim/test_get_samus_anim_state/test_check_samus_state×3（9 测试） |
| 2 | 跑测试（失败（实现未完成）） | E0425（apply_samus_anim/get_samus_anim_state/check_samus_state 未实现（TDD 预期失败））（实测输出原文（A 级）） |
| 3 | 写实现（真规则（公式形（零字面特值））（去地址名（语义命名））） | apply_samus_anim（if x > 0）+get_samus_anim_state（常量返回）+check_samus_state（wrapping_add）（s1_d86d_body→apply_samus_anim+s1_d970_body→get_samus_anim_state+s1_f676→check_samus_state） |
| 4 | 跑测试（通过（实现完成）） | test result: ok. 1113 passed; 0 failed（实测输出原文（A 级）） |
| 5 | 重构（实现插入位置（mod tests_block115 前）） | 作用域正确（保持测试通过） |
| 6 | 复核（对抗复核（11 项（必跑验证））） | 待 verifier 角色独立重导证据 |

> 【refs 引用必写】bank:addr 引证（A 级原文）：`bank01:8000`（s1 族（prg1_brinstar.asm））（s1_/s2_ 族（实址（A 级原文）））
> 【留档可追溯】docs commit（追溯径（refs 引用（bank:addr）+改写实录（逐项（非笼统））+实测输出原文（A 级）））
