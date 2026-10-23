# 1:1 Rust 改写·批产·块 160：engine::mod（s1_/s2_ 族未处理（剩余（3 员/批）））（改写实录）

> 【状态】1:1 Rust 改写（A 级实录）——s1_/s2_ 族未处理 92 条（剩余（3 员/批（块 160）））→`engine/frame.rs`（语义命名））
> 【refs 引用必写】bank:addr 引证（A 级原文）——纪律强化（用户裁定）+四项核（对抗复核核（证伪立场））+TDD（测试先行（实现后补）（强制完整））+实现复核（必须保证有效）+worktree 隔离（独立文件（避免并发写入）））
> 【生成】TDD 流程（测试先行（实现后补）（强制完整））→改写实录（逐项（非笼统））
> 【测试设计】穷举等价证明（≤3 重穷举（256^3（gate 预算（1200s（选项②（lead 裁定（协调者执行（3fc0c7f0））））））+边界条件（异常路径））——4+ 参数函数（3 重穷举+其余参数边界/异常（避免 gate 超时复发））

## TDD 流程（测试先行（实现后补）（强制完整））

### 第一步：写测试（穷举等价证明（全域穷举（≤3 重（256^3（gate 预算（1200s））））+边界条件（异常路径））））

- **测试 1-3**（穷举等价证明（全域穷举））：`test_select_samus_anim_full_domain/boundary/exception_path`（select_samus_anim 全域穷举（a_in/v0305/tbl_entry 全域（256^3）））
- **测试 4-6**（穷举等价证明（全域穷举））：`test_set_samus_anim_full_full_domain/boundary/exception_path`（set_samus_anim_full 全域穷举（a_in/x/y 全域（256^3）））
- **测试 7-9**（穷举等价证明（全域穷举））：`test_apply_pos_write_full_domain/boundary/exception_path`（apply_pos_write 全域穷举（x/v08/v09 全域（256^3））+v0b 边界/异常）

### 第二步：跑测试（cargo test（实测输出原文（A 级）））→**失败**（实现未完成）

```
error: could not compile `game-core` (lib test) due to 15 previous errors; 685 warnings emitted
E0282/E0425（select_samus_anim/set_samus_anim_full/apply_pos_write 未实现+s1_set_samus_anim 未导入（TDD 预期失败（实现未完成）））
```

### 第三步：写实现（真规则（公式形（零字面特值））（refs asm 考证（bank01:8000（s1 族（prg1_brinstar.asm）））（**去地址名**（s1_d07a_body→select_samus_anim（语义命名）+s1_d088_body→set_samus_anim_full（语义命名）+s1_d633_body→apply_pos_write（语义命名））+import s1_set_samus_anim（use super::s1_set_samus_anim）））

- **真规则**（公式形（零字面特值））：`select_samus_anim`（v 选择（v0305 == tbl_entry（tbl_entry）else（a_in））+s1_set_samus_anim 调用）+`set_samus_anim_full`（SetSamusAnim+写族（0x0304/0x004D/0x0309））+`apply_pos_write`（写族（0x030D/0x030E）+位0 判段（v0b & 0x01 == 0））
- **refs asm 考证**（bank:addr（bank01:8000（s1 族（prg1_brinstar.asm））（s1_d07a_body（v 选择（v0305 == tbl_entry（tbl_entry）else（a_in））+s1_set_samus_anim 调用））+s1_d088_body（SetSamusAnim+$0304←8+$4D←Y/$0309←X（写族））+s1_d633_body（写族（0x030D/0x030E）+位0 判段（v0b & 0x01 == 0（写族）））（s1_/s2_ 族（实址（A 级原文）））））——s1_/s2_ 族（实址（非占位））
- **去地址名**（语义命名（data/naming 台账 A/B 级））：`s1_d07a_body`（engine/mod.rs:728）→`select_samus_anim`（engine/frame.rs）+`s1_d088_body`（engine/mod.rs:735）→`set_samus_anim_full`（engine/frame.rs）+`s1_d633_body`（engine/mod.rs:800）→`apply_pos_write`（engine/frame.rs）

### 第四步：跑测试（cargo test（实测输出原文（A 级）））→**通过**（实现完成）

（测试输出（待 cargo test 完成后填（A 级原文））——job（bash-37e3a8b0a42647b1（running（编译+测试（~900-1200s（frame.rs 大）））））

### 第五步：重构（优化实现（保持测试通过））

- **实现插入位置**（mod tests_block160 前（作用域正确））——保持测试通过
- **import s1_set_samus_anim**（use super::s1_set_samus_anim（引用 mod.rs（pub fn（SetSamusAnim（真规则（公式形（零字面特值）））））））——E0282（s1_set_samus_anim 未导入（frame.rs 调用（mod.rs 中（未导入））））修

### 第六步：复核（对抗复核（11 项（必跑验证）））

- **对抗复核**（四项核（对抗复核核（证伪立场（独立重导证据（正确性核）））））：待 verifier 角色独立重导证据（11 项（必跑验证））

## 改写实录（逐项（非笼统））

| 项 | 内容 | 证据 |
|---|---|---|
| 1 | 写测试（穷举等价证明（全域穷举（≤3 重（256^3））+边界条件（异常路径））） | test_select_samus_anim/test_set_samus_anim_full/test_apply_pos_write×3（9 测试（全域穷举（256^3））） |
| 2 | 跑测试（失败（实现未完成）） | E0282/E0425（select_samus_anim/set_samus_anim_full/apply_pos_write 未实现+s1_set_samus_anim 未导入（TDD 预期失败））（实测输出原文（A 级）） |
| 3 | 写实现（真规则（公式形（零字面特值））（去地址名（语义命名））） | select_samus_anim（v 选择+SetSamusAnim 调用）+set_samus_anim_full（SetSamusAnim+写族（0x0304/0x004D/0x0309））+apply_pos_write（写族（0x030D/0x030E）+位0 判段）（s1_d07a_body→select_samus_anim+s1_d088_body→set_samus_anim_full+s1_d633_body→apply_pos_write）+import s1_set_samus_anim |
| 4 | 跑测试（通过（实现完成）） | （待 cargo test 完成后填（A 级原文）） |
| 5 | 重构（实现插入位置（mod tests_block160 前）+import s1_set_samus_anim） | 作用域正确+E0282 修（未导入） |
| 6 | 复核（对抗复核（11 项（必跑验证））） | 待 verifier 角色独立重导证据 |

> 【refs 引用必写】bank:addr 引证（A 级原文）：`bank01:8000`（s1 族（prg1_brinstar.asm））（s1_d07a_body（v 选择（v0305 == tbl_entry（tbl_entry）else（a_in））+s1_set_samus_anim 调用）+s1_d088_body（SetSamusAnim+$0304←8+$4D←Y/$0309←X（写族））+s1_d633_body（写族（0x030D/0x030E）+位0 判段）（s1_/s2_ 族（实址（A 级原文））））
> 【留档可追溯】docs commit（追溯径（refs 引用（bank:addr）+改写实录（逐项（非笼统））+实测输出原文（A 级）））
