# 1:1 Rust 改写·批产·块 111：engine::mod（无前缀地址命名 7 条（bank7 固定区））（改写实录）

> 【状态】1:1 Rust 改写（A 级实录）——无前缀地址命名 7 条（bank7 固定区（c2bf_adiv16 等（语义命名）））→`engine/frame.rs`（语义命名））
> 【refs 引用必写】bank:addr 引证（A 级原文）——纪律强化（用户裁定）+四项核（对抗复核核（证伪立场））+TDD（测试先行（实现后补）（强制完整））+实现复核（必须保证有效）+worktree 隔离（独立文件（避免并发写入）））
> 【生成】TDD 流程（测试先行（实现后补）（强制完整））→改写实录（逐项（非笼统））

## TDD 流程（测试先行（实现后补）（强制完整））

### 第一步：写测试（穷举等价证明（全域穷举（边界条件（异常路径））））

- **测试 1-3**（穷举等价证明（全域穷举））：`test_divide_by_16_full_domain/boundary/exception_path`（divide_by_16 全域穷举（a_in 全域（0..=255u8）））
- **测试 4-6**（穷举等价证明（全域穷举））：`test_multiply_by_16_full_domain/boundary/exception_path`（multiply_by_16 全域穷举（a_in 全域（0..=255u8）））
- **测试 7-9**（穷举等价证明（全域穷举））：`test_play_tourian_music_full_domain/boundary/exception_path`（play_tourian_music 全域穷举（无参数（常量返回）））

### 第二步：跑测试（cargo test（实测输出原文（A 级）））→**失败**（实现未完成）

```
error: could not compile `game-core` (lib test) due to 13 previous errors; 621 warnings emitted
E0425（divide_by_16/multiply_by_16/play_tourian_music 未实现（TDD 预期失败（实现未完成）））
```

### 第三步：写实现（真规则（公式形（零字面特值））（refs asm 考证（bank07:C000（固定区（c2bf_adiv16/c2c5_amul16/cc07_tourian_music（实址（A 级原文）））））（**去地址名**（c2bf_adiv16→divide_by_16（语义命名）+c2c5_amul16→multiply_by_16（语义命名）+cc07_tourian_music→play_tourian_music（语义命名））））

- **真规则**（公式形（零字面特值））：`divide_by_16`（a_in >> 4（refs 法直得（byte-driven semantics）））+`multiply_by_16`（a_in << 4（Adiv16 对偶（refs 法直得（byte-driven semantics））））+`play_tourian_music`（(vec![(0x0681, 0x20)], 0x20)（sfxNoise_TourianMusic（案实核径（byte-driven semantics））））
- **refs asm 考证**（bank:addr（bank07:C000（固定区（c2bf_adiv16（a_in >> 4（refs 法直得））+c2c5_amul16（a_in << 4（Adiv16 对偶（refs 法直得）））+cc07_tourian_music（sfxNoise_TourianMusic（案实核径）））（实址（A 级原文）））））——bank7 固定区（实址（非占位））
- **去地址名**（语义命名（data/naming 台账 A/B 级））：`c2bf_adiv16`（engine/mod.rs:8129）→`divide_by_16`（engine/frame.rs）+`c2c5_amul16`（engine/mod.rs:8157）→`multiply_by_16`（engine/frame.rs）+`cc07_tourian_music`（engine/mod.rs:8185）→`play_tourian_music`（engine/frame.rs）

### 第四步：跑测试（cargo test（实测输出原文（A 级）））→**通过**（实现完成）

```
test result: ok. 1097 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 9.28s
```

### 第五步：重构（优化实现（保持测试通过））

- **实现插入位置**（mod tests_block111 前（作用域正确））——保持测试通过

### 第六步：复核（对抗复核（11 项（必跑验证）））

- **对抗复核**（四项核（对抗复核核（证伪立场（独立重导证据（正确性核）））））：待 verifier 角色独立重导证据（11 项（必跑验证））

## 改写实录（逐项（非笼统））

| 项 | 内容 | 证据 |
|---|---|---|
| 1 | 写测试（穷举等价证明（全域穷举（边界条件（异常路径）））） | test_divide_by_16/test_multiply_by_16/test_play_tourian_music×3（9 测试（divide_by_16/multiply_by_16 全域穷举（0..=255u8））） |
| 2 | 跑测试（失败（实现未完成）） | E0425（divide_by_16/multiply_by_16/play_tourian_music 未实现（TDD 预期失败））（实测输出原文（A 级）） |
| 3 | 写实现（真规则（公式形（零字面特值））（去地址名（语义命名））） | divide_by_16（a_in >> 4（refs 法直得））+multiply_by_16（a_in << 4（Adiv16 对偶（refs 法直得）））+play_tourian_music（(vec![(0x0681, 0x20)], 0x20)（sfxNoise_TourianMusic（案实核径）））（c2bf_adiv16→divide_by_16+c2c5_amul16→multiply_by_16+cc07_tourian_music→play_tourian_music） |
| 4 | 跑测试（通过（实现完成）） | test result: ok. 1097 passed; 0 failed（实测输出原文（A 级）） |
| 5 | 重构（实现插入位置（mod tests_block111 前）） | 作用域正确（保持测试通过） |
| 6 | 复核（对抗复核（11 项（必跑验证））） | 待 verifier 角色独立重导证据 |

> 【refs 引用必写】bank:addr 引证（A 级原文）：`bank07:C000`（固定区（c2bf_adiv16（a_in >> 4（refs 法直得））+c2c5_amul16（a_in << 4（Adiv16 对偶））+cc07_tourian_music（sfxNoise_TourianMusic（案实核径））（实址（A 级原文））））
> 【留档可追溯】docs commit（追溯径（refs 引用（bank:addr）+改写实录（逐项（非笼统））+实测输出原文（A 级）））
