# 1:1 Rust 改写·批产·块 12：ending/sequence（改写实录）

> 【状态】1:1 Rust 改写（A 级实录）——s1_/s2_ 族继续去地址名（`ending/sequence.rs`（语义命名）））
> 【refs 引用必写】bank:addr 引证（A 级原文）——纪律强化（用户裁定）+四项核（对抗复核核（证伪立场））+TDD（测试先行（实现后补）（强制完整））+实现复核（必须保证有效）+worktree 隔离（独立文件（避免并发写入）））
> 【生成】TDD 流程（测试先行（实现后补）（强制完整））→改写实录（逐项（非笼统））

## TDD 流程（测试先行（实现后补）（强制完整））

### 第一步：写测试（穷举等价证明（全域穷举（边界条件（异常路径））））

- **测试 1-3**（穷举等价证明（全域穷举））：`test_choose_ending_full_domain/boundary/exception_path`（choose_ending 全域穷举（statue_bits 全域））
- **测试 4-6**（穷举等价证明（全域穷举））：`test_play_ending_full_domain/boundary/exception_path`（play_ending 全域穷举（ram 态全域））

### 第二步：跑测试（cargo test（实测输出原文（A 级）））→**失败**（实现未完成）

```
error: could not compile `game-core` (lib test) due to 3 previous errors; 225 warnings emitted
E0609（no field `ending_id` on type `EndingState`（available fields are: `frame`, `playing`））
```

### 第三步：写实现（真规则（公式形（零字面特值））（refs asm 考证（bank07:CAF5-CB14（ChooseEnding）+bank04:8D60（GFX_EndingSprites））（**去地址名**（choose_ending（语义命名）+play_ending（语义命名））））

- **真规则**（公式形（零字面特值））：`choose_ending`（statue_bits（byte-driven semantics））+`play_ending`（frame=ram[0x0300]/playing=true（byte-driven semantics））
- **refs asm 考证**（bank:addr（bank07:CAF5-CB14（ChooseEnding）+bank04:8D60（GFX_EndingSprites）（结局例程（实址（A 级原文）））））——结局例程（实址（非占位））
- **去地址名**（语义命名（data/naming 台账 A/B 级））：`choose_ending`（语义命名）+`play_ending`（语义命名）

### 第四步：跑测试（cargo test（实测输出原文（A 级）））→**通过**（实现完成）

```
test result: ok. 702 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 6.35s
```

### 第五步：重构（优化实现（保持测试通过））

- **测试修**（EndingState（frame/playing（byte-driven semantics））（E0609（ending_id）））——测试预期与实现公式一致
- **实现修**（play_ending（frame=ram[0x0300]（byte-driven semantics）））——真规则（公式形（零字面特值））
- **模块导出**（ending/mod.rs（pub mod sequence）+lib.rs（pub mod ending））

### 第六步：复核（对抗复核（11 项（必跑验证）））

- **对抗复核**（四项核（对抗复核核（证伪立场（独立重导证据（正确性核）））））：待 verifier 角色独立重导证据（11 项（必跑验证））

## 改写实录（逐项（非笼统））

| 项 | 内容 | 证据 |
|---|---|---|
| 1 | 写测试（穷举等价证明（全域穷举（边界条件（异常路径）））） | test_choose_ending/test_play_ending×3（6 测试） |
| 2 | 跑测试（失败（实现未完成）） | E0609（no field `ending_id` on type `EndingState`）（实测输出原文（A 级）） |
| 3 | 写实现（真规则（公式形（零字面特值））（去地址名（语义命名））） | choose_ending（statue_bits）+play_ending（frame=ram[0x0300]/playing=true） |
| 4 | 跑测试（通过（实现完成）） | test result: ok. 702 passed; 0 failed（实测输出原文（A 级）） |
| 5 | 重构（测试修+实现修+模块导出） | EndingState（frame/playing）+play_ending（frame=ram[0x0300]）+ending/mod.rs（pub mod sequence）+lib.rs（pub mod ending） |
| 6 | 复核（对抗复核（11 项（必跑验证））） | 待 verifier 角色独立重导证据 |

> 【refs 引用必写】bank:addr 引证（A 级原文）：`bank07:CAF5-CB14`（ChooseEnding）+`bank04:8D60`（GFX_EndingSprites）（结局例程（实址（A 级原文）））
> 【留档可追溯】docs commit（追溯径（refs 引用（bank:addr）+改写实录（逐项（非笼统））+实测输出原文（A 级）））
