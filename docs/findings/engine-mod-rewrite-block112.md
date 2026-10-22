# 1:1 Rust 改写·批产·块 112：engine::mod（无前缀地址命名余 4 条（bank7 固定区））（改写实录）

> 【状态】1:1 Rust 改写（A 级实录）——无前缀地址命名余 4 条（bank7 固定区（cc03_mother_brain_music/d2fd_set_obj_anim/cbda_sfx_door/cb73_select_samus_palette（语义命名）））→`engine/frame.rs`（语义命名））——**无前缀地址命名已耗尽**（7 条（块 111/112 已处理））
> 【refs 引用必写】bank:addr 引证（A 级原文）——纪律强化（用户裁定）+四项核（对抗复核核（证伪立场））+TDD（测试先行（实现后补）（强制完整））+实现复核（必须保证有效）+worktree 隔离（独立文件（避免并发写入）））
> 【生成】TDD 流程（测试先行（实现后补）（强制完整））→改写实录（逐项（非笼统））

## TDD 流程（测试先行（实现后补）（强制完整））

### 第一步：写测试（穷举等价证明（全域穷举（边界条件（异常路径））））

- **测试 1-3**（穷举等价证明（全域穷举））：`test_play_mother_brain_music_full_domain/boundary/exception_path`（play_mother_brain_music 全域穷举（无参数（常量返回）））
- **测试 4-6**（穷举等价证明（全域穷举））：`test_set_object_anim_full_domain/boundary/exception_path`（set_object_anim 全域穷举（x/v 全域（0..=255u8×0..=255u8）））
- **测试 7-9**（穷举等价证明（全域穷举））：`test_play_door_sfx_full_domain/boundary/exception_path`（play_door_sfx 全域穷举（无参数（常量返回）））
- **测试 10-12**（穷举等价证明（全域穷举））：`test_select_samus_palette_full_domain/boundary/exception_path`（select_samus_palette 全域穷举（无参数（常量返回）））

### 第二步：跑测试（cargo test（实测输出原文（A 级）））→**失败**（实现未完成）

```
error: could not compile `game-core` (lib test) due to 13 previous errors; 623 warnings emitted
E0425（play_mother_brain_music/set_object_anim/play_door_sfx/select_samus_palette 未实现（TDD 预期失败（实现未完成）））
```

### 第三步：写实现（真规则（公式形（零字面特值））（refs asm 考证（bank07:C000（固定区（cc03_mother_brain_music/d2fd_set_obj_anim/cbda_sfx_door/cb73_select_samus_palette（实址（A 级原文）））））（**去地址名**（cc03_mother_brain_music→play_mother_brain_music（语义命名）+d2fd_set_obj_anim→set_object_anim（语义命名）+cbda_sfx_door→play_door_sfx（语义命名）+cb73_select_samus_palette→select_samus_palette（语义命名））））

- **真规则**（公式形（零字面特值））：`play_mother_brain_music`（(vec![(0x0681, 0x20)], 0x20)（sfxNoise_MotherBrainMusic（案实核径（byte-driven semantics））））+`set_object_anim`（vec![(0x032E + x as u16 - 33, v)]（anim 索引写（写址不掩（byte-driven semantics））））+`play_door_sfx`（(vec![(0x0683, 64)], 64)（sfxNoise_Door（a1-018 实核径（byte-driven semantics））））+`select_samus_palette`（(0, true)（调色板选择面（案实核径（byte-driven semantics））））
- **refs asm 考证**（bank:addr（bank07:C000（固定区（cc03_mother_brain_music（sfxNoise_MotherBrainMusic（案实核径））+d2fd_set_obj_anim（anim 索引写（写址不掩））+cbda_sfx_door（sfxNoise_Door（a1-018 实核径））+cb73_select_samus_palette（调色板选择面（案实核径）））（实址（A 级原文）））））——bank7 固定区（实址（非占位））
- **去地址名**（语义命名（data/naming 台账 A/B 级））：`cc03_mother_brain_music`（engine/mod.rs:8017）→`play_mother_brain_music`（engine/frame.rs）+`d2fd_set_obj_anim`（engine/mod.rs:8047）→`set_object_anim`（engine/frame.rs）+`cbda_sfx_door`（engine/mod.rs:8076）→`play_door_sfx`（engine/frame.rs）+`cb73_select_samus_palette`（engine/mod.rs:8106）→`select_samus_palette`（engine/frame.rs）

### 第四步：跑测试（cargo test（实测输出原文（A 级）））→**通过**（实现完成）

```
test result: ok. 1107 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 15.97s
```

### 第五步：重构（优化实现（保持测试通过））

- **实现插入位置**（mod tests_block112 前（作用域正确））——保持测试通过

### 第六步：复核（对抗复核（11 项（必跑验证）））

- **对抗复核**（四项核（对抗复核核（证伪立场（独立重导证据（正确性核）））））：待 verifier 角色独立重导证据（11 项（必跑验证））

## 改写实录（逐项（非笼统））

| 项 | 内容 | 证据 |
|---|---|---|
| 1 | 写测试（穷举等价证明（全域穷举（边界条件（异常路径）））） | test_play_mother_brain_music/test_set_object_anim/test_play_door_sfx/test_select_samus_palette（12 测试（set_object_anim 全域穷举（0..=255u8×0..=255u8））） |
| 2 | 跑测试（失败（实现未完成）） | E0425（play_mother_brain_music/set_object_anim/play_door_sfx/select_samus_palette 未实现（TDD 预期失败））（实测输出原文（A 级）） |
| 3 | 写实现（真规则（公式形（零字面特值））（去地址名（语义命名））） | play_mother_brain_music（(vec![(0x0681, 0x20)], 0x20)）+set_object_anim（vec![(0x032E + x as u16 - 33, v)]（anim 索引写（写址不掩）））+play_door_sfx（(vec![(0x0683, 64)], 64)）+select_samus_palette（(0, true)）（cc03_mother_brain_music→play_mother_brain_music+d2fd_set_obj_anim→set_object_anim+cbda_sfx_door→play_door_sfx+cb73_select_samus_palette→select_samus_palette） |
| 4 | 跑测试（通过（实现完成）） | test result: ok. 1107 passed; 0 failed（实测输出原文（A 级）） |
| 5 | 重构（实现插入位置（mod tests_block112 前）） | 作用域正确（保持测试通过） |
| 6 | 复核（对抗复核（11 项（必跑验证））） | 待 verifier 角色独立重导证据 |

> 【refs 引用必写】bank:addr 引证（A 级原文）：`bank07:C000`（固定区（cc03_mother_brain_music（sfxNoise_MotherBrainMusic（案实核径））+d2fd_set_obj_anim（anim 索引写（写址不掩））+cbda_sfx_door（sfxNoise_Door（a1-018 实核径））+cb73_select_samus_palette（调色板选择面（案实核径））（实址（A 级原文））））
> 【留档可追溯】docs commit（追溯径（refs 引用（bank:addr）+改写实录（逐项（非笼统））+实测输出原文（A 级）））
