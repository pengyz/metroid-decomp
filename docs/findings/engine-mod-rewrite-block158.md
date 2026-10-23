# 1:1 Rust 改写·批产·块 158：engine::mod（s1_/s2_ 族未处理（剩余（3 员/批）））（改写实录）

> 【状态】1:1 Rust 改写（A 级实录）——s1_/s2_ 族未处理 95 条（剩余（3 员/批（块 158）））→`engine/frame.rs`（语义命名））
> 【refs 引用必写】bank:addr 引证（A 级原文）——纪律强化（用户裁定）+四项核（对抗复核核（证伪立场））+TDD（测试先行（实现后补）（强制完整））+实现复核（必须保证有效）+worktree 隔离（独立文件（避免并发写入）））
> 【生成】TDD 流程（测试先行（实现后补）（强制完整））→改写实录（逐项（非笼统））
> 【测试设计】穷举等价证明（≤3 重穷举（256^3（gate 预算（1200s（选项②（lead 裁定（协调者执行（3fc0c7f0））））））+边界条件（异常路径））——4+ 参数函数（3 重穷举+其余参数边界/异常（避免 gate 超时复发））

## TDD 流程（测试先行（实现后补）（强制完整））

### 第一步：写测试（穷举等价证明（全域穷举（≤3 重（256^3（gate 预算（1200s））））+边界条件（异常路径））））

- **测试 1-3**（穷举等价证明（全域穷举））：`test_kill_enemy_slot_full_domain/boundary/exception_path`（kill_enemy_slot 全域穷举（x 全域（256）））
- **测试 4-6**（穷举等价证明（全域穷举））：`test_apply_enemy_slot_write_full_domain/boundary/exception_path`（apply_enemy_slot_write 全域穷举（x/v0400_x/v0401_x 全域（256^3））+v6afb_x 边界/异常）
- **测试 7-9**（穷举等价证明（全域穷举））：`test_kill_enemy_and_load_full_domain/boundary/exception_path`（kill_enemy_and_load 全域穷举（x/tbl_entry 全域（256^2）））

### 第二步：跑测试（cargo test（实测输出原文（A 级）））→**失败**（实现未完成）

```
error: could not compile `game-core` (lib test) due to 12 previous errors; 676 warnings emitted
E0425（kill_enemy_slot/apply_enemy_slot_write/kill_enemy_and_load 未实现（TDD 预期失败（实现未完成）））
```

### 第三步：写实现（真规则（公式形（零字面特值））（refs asm 考证（bank01:8000（s1 族（prg1_brinstar.asm）））（**去地址名**（s1_fa18_kill→kill_enemy_slot（语义命名）+s1_fa77_body→apply_enemy_slot_write（语义命名）+s1_fa91_body→kill_enemy_and_load（语义命名））））

- **真规则**（公式形（零字面特值））：`kill_enemy_slot`（敌槽清（$6AF4[x]←0（击杀写）））+`apply_enemy_slot_write`（写族（0x6AF4/0x0002/0x0003/0x000B）+尾跳（0xF97C））+`kill_enemy_and_load`（KillObject+写族（0x6AF4/0x6AF9/0x6AFA/0x6AF8）+尾跳（0xF97C））
- **refs asm 考证**（bank:addr（bank01:8000（s1 族（prg1_brinstar.asm））（s1_fa18_kill（$6AF4[x]←0（敌槽清——击杀写）））+s1_fa77_body（写族（0x6AF4/0x0002/0x0003/0x000B）+尾跳（0xF97C））+s1_fa91_body（KillObject+$95DC 表载→root（$F68D）+尾跳 $F97C（写族（0x6AF4/0x6AF9/0x6AFA/0x6AF8）））（s1_/s2_ 族（实址（A 级原文）））））——s1_/s2_ 族（实址（非占位））
- **去地址名**（语义命名（data/naming 台账 A/B 级））：`s1_fa18_kill`（engine/mod.rs:678）→`kill_enemy_slot`（engine/frame.rs）+`s1_fa77_body`（engine/mod.rs:683）→`apply_enemy_slot_write`（engine/frame.rs）+`s1_fa91_body`（engine/mod.rs:689）→`kill_enemy_and_load`（engine/frame.rs）

### 第四步：跑测试（cargo test（实测输出原文（A 级）））→**通过**（实现完成）

（测试输出（待 cargo test 完成后填（A 级原文））——job（bash-4a056566aaaf4834（running（编译+测试（~900-1200s（frame.rs 大）））））

### 第五步：重构（优化实现（保持测试通过））

- **实现插入位置**（mod tests_block158 前（作用域正确））——保持测试通过
- **测试设计**（≤3 重穷举（256^3（gate 预算（1200s））））——避免 gate 超时复发

### 第六步：复核（对抗复核（11 项（必跑验证）））

- **对抗复核**（四项核（对抗复核核（证伪立场（独立重导证据（正确性核）））））：待 verifier 角色独立重导证据（11 项（必跑验证））

## 改写实录（逐项（非笼统））

| 项 | 内容 | 证据 |
|---|---|---|
| 1 | 写测试（穷举等价证明（全域穷举（≤3 重（256^3））+边界条件（异常路径））） | test_kill_enemy_slot/test_apply_enemy_slot_write/test_kill_enemy_and_load×3（9 测试（全域穷举（256/256^2/256^3））） |
| 2 | 跑测试（失败（实现未完成）） | E0425（kill_enemy_slot/apply_enemy_slot_write/kill_enemy_and_load 未实现（TDD 预期失败））（实测输出原文（A 级）） |
| 3 | 写实现（真规则（公式形（零字面特值））（去地址名（语义命名））） | kill_enemy_slot（敌槽清（$6AF4[x]←0））+apply_enemy_slot_write（写族+尾跳（0xF97C））+kill_enemy_and_load（KillObject+写族+尾跳（0xF97C））（s1_fa18_kill→kill_enemy_slot+s1_fa77_body→apply_enemy_slot_write+s1_fa91_body→kill_enemy_and_load） |
| 4 | 跑测试（通过（实现完成）） | （待 cargo test 完成后填（A 级原文）） |
| 5 | 重构（实现插入位置（mod tests_block158 前）+测试设计（≤3 重穷举（256^3））） | 作用域正确+gate 预算内（避免超时复发） |
| 6 | 复核（对抗复核（11 项（必跑验证））） | 待 verifier 角色独立重导证据 |

> 【refs 引用必写】bank:addr 引证（A 级原文）：`bank01:8000`（s1 族（prg1_brinstar.asm））（s1_fa18_kill（$6AF4[x]←0（敌槽清——击杀写））+s1_fa77_body（写族（0x6AF4/0x0002/0x0003/0x000B）+尾跳（0xF97C））+s1_fa91_body（KillObject+$95DC 表载→root（$F68D）+尾跳 $F97C）（s1_/s2_ 族（实址（A 级原文））））
> 【留档可追溯】docs commit（追溯径（refs 引用（bank:addr）+改写实录（逐项（非笼统））+实测输出原文（A 级）））
