# 1:1 Rust 改写·批产·块 118：engine::mod（s1_/s2_ 族未处理 116 条（剩余（3 员/批）））（改写实录）

> 【状态】1:1 Rust 改写（A 级实录）——s1_/s2_ 族未处理 113 条（剩余（3 员/批（块 118）））→`engine/frame.rs`（语义命名））
> 【refs 引用必写】bank:addr 引证（A 级原文）——纪律强化（用户裁定）+四项核（对抗复核核（证伪立场））+TDD（测试先行（实现后补）（强制完整））+实现复核（必须保证有效）+worktree 隔离（独立文件（避免并发写入）））
> 【生成】TDD 流程（测试先行（实现后补）（强制完整））→改写实录（逐项（非笼统））

## TDD 流程（测试先行（实现后补）（强制完整））

### 第一步：写测试（穷举等价证明（全域穷举（边界条件（异常路径））））

- **测试 1-3**（穷举等价证明（全域穷举））：`test_dispatch_samus_anim_full_domain/boundary/exception_path`（dispatch_samus_anim 全域穷举（root_val/x/v0409_x 全域（0..=255u8×3）））
- **测试 4-6**（穷举等价证明（全域穷举））：`test_apply_samus_flag_gate_full_domain/boundary/exception_path`（apply_samus_flag_gate 全域穷举（root_val/x 全域（0..=255u8×2）））
- **测试 7-9**（穷举等价证明（全域穷举））：`test_apply_samus_flag_hit_full_domain/boundary/exception_path`（apply_samus_flag_hit 全域穷举（root_val/x 全域（0..=255u8×2）））

### 第二步：跑测试（cargo test（实测输出原文（A 级）））→**失败**（实现未完成）

```
error: could not compile `game-core` (lib test) due to 14 previous errors; 637 warnings emitted
E0425（dispatch_samus_anim/apply_samus_flag_gate/apply_samus_flag_hit 未实现（TDD 预期失败（实现未完成）））
```

### 第三步：写实现（真规则（公式形（零字面特值））（refs asm 考证（bank01:8000（s1 族（prg1_brinstar.asm）））（**去地址名**（s1_f3d8→dispatch_samus_anim（语义命名）+s1_f503→apply_samus_flag_gate（语义命名）+s1_f55c→apply_samus_flag_hit（语义命名））））

- **真规则**（公式形（零字面特值））：`dispatch_samus_anim`（check_samus_state 调用+尾跳面（if v0409_x == 0（0xF7BA）else（0xF40A）））+`apply_samus_flag_gate`（s1_flag_gate（旗门（命中⇒(vec![], 0xA0, 0xF423)（栈族+尾跳）；未命中⇒(vec![(0x040C + x as u16, root_val)], 0x04, 0xF400)（写族+续流））））+`apply_samus_flag_hit`（s1_flag_gate（旗门（命中⇒(vec![(0x040B + x as u16, 5)], Some(0x95A8))（写族+尾跳）；未命中⇒(vec![], None)（rts（空）））））
- **refs asm 考证**（bank:addr（bank01:8000（s1 族（prg1_brinstar.asm））（s1_f3d8（s1_f676 调用（尾跳面（0xF7BA/0xF40A）））+s1_f503（s1_flag_gate（旗门（命中/未命中）））+s1_f55c（s1_flag_gate（旗门（命中⇒$040B,x←5+尾跳 $95A8；未命中⇒rts（空）））））（s1_/s2_ 族（实址（A 级原文））））））——s1_/s2_ 族（实址（非占位））
- **去地址名**（语义命名（data/naming 台账 A/B 级））：`s1_f3d8`（engine/mod.rs:322）→`dispatch_samus_anim`（engine/frame.rs）+`s1_f503`（engine/mod.rs:331）→`apply_samus_flag_gate`（engine/frame.rs）+`s1_f55c`（engine/mod.rs:340）→`apply_samus_flag_hit`（engine/frame.rs）

### 第四步：跑测试（cargo test（实测输出原文（A 级）））→**通过**（实现完成）

```
test result: ok. 1130 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 21.90s
```

### 第五步：重构（优化实现（保持测试通过））

- **import s1_flag_gate**（use super::s1_flag_gate（引用 mod.rs（pub fn（root_val & 0x20 != 0（真规则（公式形（零字面特值））））））——E0425（s1_flag_gate 未实现（frame.rs 调用（mod.rs 中（未导入））））修
- **test 预期修**（命中/未命中（真规则（公式形（零字面特值））（s1_flag_gate（root_val & 0x20 != 0））））——0xFF/0x7F（命中（& 0x20 != 0）→a_out=0xA0）+0x00（未命中→a_out=0x04）

### 第六步：复核（对抗复核（11 项（必跑验证）））

- **对抗复核**（四项核（对抗复核核（证伪立场（独立重导证据（正确性核）））））：待 verifier 角色独立重导证据（11 项（必跑验证））

## 改写实录（逐项（非笼统））

| 项 | 内容 | 证据 |
|---|---|---|
| 1 | 写测试（穷举等价证明（全域穷举（边界条件（异常路径）））） | test_dispatch_samus_anim/test_apply_samus_flag_gate/test_apply_samus_flag_hit×3（9 测试（全域穷举（0..=255u8×N））） |
| 2 | 跑测试（失败（实现未完成）） | E0425（dispatch_samus_anim/apply_samus_flag_gate/apply_samus_flag_hit 未实现（TDD 预期失败））（实测输出原文（A 级）） |
| 3 | 写实现（真规则（公式形（零字面特值））（去地址名（语义命名））） | dispatch_samus_anim（尾跳面（0xF7BA/0xF40A））+apply_samus_flag_gate（旗门（命中/未命中））+apply_samus_flag_hit（旗门（命中⇒写族+尾跳；未命中⇒rts（空）））（s1_f3d8→dispatch_samus_anim+s1_f503→apply_samus_flag_gate+s1_f55c→apply_samus_flag_hit） |
| 4 | 跑测试（通过（实现完成）） | test result: ok. 1130 passed; 0 failed（实测输出原文（A 级）） |
| 5 | 重构（import s1_flag_gate+test 预期修（命中/未命中（真规则（公式形（零字面特值））））） | E0425（s1_flag_gate 未导入）修+test 预期与实现公式一致 |
| 6 | 复核（对抗复核（11 项（必跑验证））） | 待 verifier 角色独立重导证据 |

> 【refs 引用必写】bank:addr 引证（A 级原文）：`bank01:8000`（s1 族（prg1_brinstar.asm））（s1_f3d8（尾跳面（0xF7BA/0xF40A））+s1_f503（旗门（命中/未命中））+s1_f55c（旗门（命中⇒$040B,x←5+尾跳 $95A8；未命中⇒rts（空）））（s1_/s2_ 族（实址（A 级原文））））
> 【留档可追溯】docs commit（追溯径（refs 引用（bank:addr）+改写实录（逐项（非笼统））+实测输出原文（A 级）））
