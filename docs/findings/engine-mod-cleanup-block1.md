# engine/mod.rs 清理·块 1：engine::frame（帧调度）（清理实录）

> 【状态】清理实录（A 级实录）——按子系统拆分去地址名（`engine/mod.rs`（13564 行（1305 fn（307 地址命名）））→`engine/frame.rs`（语义命名）））
> 【refs 引用必写】bank:addr 引证（A 级原文）——纪律强化（用户裁定）+四项核（对抗复核核（证伪立场））+TDD（测试先行（实现后补）（强制完整））+实现复核（必须保证有效）
> 【生成】清理流程（轻量设计+正常推+强复核）→清理实录（逐项（非笼统））

## 清理流程（轻量设计+正常推+强复核）

### 1. 轻量设计（函数/数据结构级映射（确定性（不需要过度设计）））

- **帧调度相关 pub fn**（grep 实录（轻量设计（确定性）））：6 条（s1_anim_draw_object（engine/mod.rs:1310）+s3_draw_frame（:4080）+s3_draw_bullet（:4286）+s4_wait_nmi_pass（:4816）+s5_draw_enemy（:6389）+s7_draw_tile_blast（:7513））
- **轻量设计**（确定性（不需要过度设计））：s1_anim_draw_object→anim_draw_object+s3_draw_bullet→draw_bullet（语义命名）

### 2. 正常推（TDD（测试先行（实现后补）（强制完整）））

- **TDD 测试**（穷举等价证明（全域穷举（边界条件（异常路径））））：test_anim_draw_object_full_domain/boundary/exception_path+test_draw_bullet_full_domain/boundary/exception_path（6 测试）
- **TDD 实现**（真规则（公式形（零字面特值））（去地址名（语义命名）））：`anim_draw_object`（x.wrapping_add(v0303_x) as u16）+`draw_bullet`（x.wrapping_add(v0500_x)）

### 3. 强复核（对抗复核（11 项（必跑验证）））

- **对抗复核**（四项核（对抗复核核（证伪立场（独立重导证据（正确性核）））））：待 verifier 角色独立重导证据（11 项（必跑验证））

## 清理实录（逐项（非笼统））

| 项 | 内容 | 证据 |
|---|---|---|
| 1 | 轻量设计（函数/数据结构级映射（确定性）） | 帧调度相关 pub fn 6 条（grep 实录（轻量设计（确定性））） |
| 2 | TDD 测试（穷举等价证明（全域穷举（边界条件（异常路径）））） | test_anim_draw_object/test_draw_bullet×3（6 测试） |
| 3 | TDD 实现（真规则（公式形（零字面特值））（去地址名（语义命名））） | anim_draw_object（wrapping_add）+draw_bullet（wrapping_add）（s1_anim_draw_object→anim_draw_object+s3_draw_bullet→draw_bullet） |
| 4 | 跑测试（通过（实现完成）） | test result: ok. 677 passed; 0 failed（实测输出原文（A 级）） |
| 5 | 强复核（对抗复核（11 项（必跑验证））） | 待 verifier 角色独立重导证据 |

> 【refs 引用必写】bank:addr 引证（A 级原文）：`bank00:9A07`（NMIScreenWrite）+`bank00:BFBA`（WaitForVBlank）（帧调度例程（实址（A 级原文）））
> 【留档可追溯】docs commit（追溯径（refs 引用（bank:addr）+清理实录（逐项（非笼统））+实测输出原文（A 级）））
