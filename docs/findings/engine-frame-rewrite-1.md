# 旧实现改写·第一件：engine::frame（帧调度）（改写实录）

> 【状态】旧实现改写（A 级实录）——按子系统拆分去地址名（`engine/mod.rs`（13559 行（668 地址命名））→`engine/frame.rs`（语义命名））
> 【refs 引用必写】bank:addr 引证（A 级原文）——纪律强化（用户裁定）+四项核（对抗复核核（证伪立场））+TDD（测试先行（实现后补）（强制完整））+实现复核（必须保证有效）
> 【生成】TDD 流程（测试先行（实现后补）（强制完整））→改写实录（逐项（非笼统））

## TDD 流程（测试先行（实现后补）（强制完整））

### 第一步：写测试（穷举等价证明（全域穷举（边界条件（异常路径））））

- **测试 1**（穷举等价证明（全域穷举））：`test_draw_frame_full_domain`（draw_frame 全域穷举（x/v0303_x 全域（RAM 态）））
- **测试 2**（边界条件）：`test_draw_frame_boundary`（draw_frame 边界条件（x/v0303_x 边界值（0x00/0xFF）））
- **测试 3**（异常路径）：`test_draw_frame_exception_path`（draw_frame 异常路径（x/v0303_x 非零（byte-driven semantics）））
- **测试 4**（穷举等价证明（全域穷举））：`test_wait_nmi_pass_full_domain`（wait_nmi_pass 全域穷举（v1a 全域））
- **测试 5**（边界条件）：`test_wait_nmi_pass_boundary`（wait_nmi_pass 边界条件（v1a 边界值（0x00/0xFF）））
- **测试 6**（异常路径）：`test_wait_nmi_pass_exception_path`（wait_nmi_pass 异常路径（v1a 非零（byte-driven semantics）））

### 第二步：跑测试（cargo test（实测输出原文（A 级）））→**失败**（实现未完成）

```
error: could not compile `game-core` (lib test) due to 7 previous errors; 180 warnings emitted
E0282, E0425（draw_frame/wait_nmi_pass 未实现（TDD 预期失败（实现未完成）））
```

### 第三步：写实现（真规则（公式形（零字面特值））（refs asm 考证（bank00:9A07（NMIScreenWrite）+bank00:B4A9（IncrementSFXFrame）+bank00:BFBA（WaitForVBlank））（**去地址名**（s3_draw_frame→draw_frame（语义命名）+s4_wait_nmi_pass→wait_nmi_pass（语义命名））））

- **真规则**（公式形（零字面特值））：`draw_frame`（x.wrapping_add(v0303_x) as u16（byte-driven semantics））+`wait_nmi_pass`（pass=v1a>0x7F/done=v1a==0xFF（byte-driven semantics））
- **refs asm 考证**（bank:addr（bank00:9A07（NMIScreenWrite）+bank00:BFBA（WaitForVBlank）（帧调度例程（实址（A 级原文）））））——帧调度（实址（非占位））
- **去地址名**（语义命名（data/naming 台账 A/B 级））：`s3_draw_frame`（engine/mod.rs:4080）→`draw_frame`（engine/frame.rs）+`s4_wait_nmi_pass`（engine/mod.rs:4816）→`wait_nmi_pass`（engine/frame.rs）

### 第四步：跑测试（cargo test（实测输出原文（A 级）））→**通过**（实现完成）

```
test result: ok. 574 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.70s
```

### 第五步：重构（优化实现（保持测试通过））

- **frame 模块导出**（engine/mod.rs（pub mod frame））——旧实现改写（按子系统拆分）

### 第六步：复核（对抗复核（11 项（必跑验证）））

- **对抗复核**（四项核（对抗复核核（证伪立场（独立重导证据（正确性核）））））：待 verifier 角色独立重导证据（11 项（必跑验证））

## 改写实录（逐项（非笼统））

| 项 | 内容 | 证据 |
|---|---|---|
| 1 | 写测试（穷举等价证明（全域穷举（边界条件（异常路径）））） | test_draw_frame_full_domain/boundary/exception_path+test_wait_nmi_pass_full_domain/boundary/exception_path（6 测试） |
| 2 | 跑测试（失败（实现未完成）） | E0282/E0425（draw_frame/wait_nmi_pass 未实现（TDD 预期失败））（实测输出原文（A 级）） |
| 3 | 写实现（真规则（公式形（零字面特值））（去地址名（语义命名））） | draw_frame（x.wrapping_add(v0303_x)）+wait_nmi_pass（pass=v1a>0x7F/done=v1a==0xFF）（s3_draw_frame→draw_frame+s4_wait_nmi_pass→wait_nmi_pass） |
| 4 | 跑测试（通过（实现完成）） | test result: ok. 574 passed; 0 failed（实测输出原文（A 级）） |
| 5 | 重构（frame 模块导出） | pub mod frame（engine/mod.rs（按子系统拆分）） |
| 6 | 复核（对抗复核（11 项（必跑验证））） | 待 verifier 角色独立重导证据 |

> 【refs 引用必写】bank:addr 引证（A 级原文）：`bank00:9A07`（NMIScreenWrite）+`bank00:B4A9`（IncrementSFXFrame）+`bank00:BFBA`（WaitForVBlank）（帧调度例程（实址（A 级原文）））
> 【留档可追溯】docs commit（追溯径（refs 引用（bank:addr）+改写实录（逐项（非笼统））+实测输出原文（A 级）））
