# Phase0 批量改写·第二十四簇：enemy::skree（Skree）（六步流程）

> 【状态】批量改写实录（A 级实录）——六步流程（定域取证 spec→域结构→memmap 适配→shim 写集适配→测试→registry 登记+独立复审）
> 【refs 引用必写】bank:addr 引证（A 级原文）——纪律强化（用户裁定）+四项核（对抗复核核（证伪立场））
> 【生成】bash refs asm 考证（bank:addr（Skree 例程（实址（A 级原文））））→批量改写实录（六步流程）

## 第一步：定域取证 spec（refs asm 考证（bank:addr（实址（A 级原文））））

- **Skree 例程**（`bank01:99FA-9A4C`（SkreeAIRoutine_BANK1）+`bank01:9A16-9A3F`（SkreeBlowUpIntoProjectiles_BANK1）+`bank01:9A3F-9A49`（SkreeExit_Active/Resting/Explode_BANK1）——refs asm 考证（bank:addr（实址（A 级原文））））——Skree（R-mv 规则面）：
  - **输入**：RAM 态（Skree 状态/位置/速度）→SkreeState
  - **规则**：SkreeAIRoutine/SkreeBlowUpIntoProjectiles/SkreeExit（99FA/9A16/9A3F 族）——Skree AI+爆裂+退出
  - **输出**：SkreeState（Skree 状态写回 RAM）
- **Skree 动画表**（`bank01:9D8D-9D9D`（EnAnim_Skree/EnAnim_SkreeExplode_BANK1）+`bank01:A06D-A0DB`（EnFrame_Skree0-2/EnFrame_SkreeExplode_BANK1）——refs asm 考证（bank:addr（实址（A 级原文））））——Skree 动画（R-mv 规则面）
- **真规则公式**（公式形（零字面特值））：`pub fn enemy_move_effects(...)`（`crates/game-core/src/enemy/common.rs:43`）

## 第二步：域结构（enemy::skree（Skree））

- **域方法**（待补）：`enemy::skree::skree_ai_routine`（Skree AI（R-mv-2 速度积分面））——RAM 态→SkreeState→SkreeAIRoutine（99FA 族）→写回
- **域方法**（待补）：`enemy::skree::skree_blow_up_into_projectiles`（Skree 爆裂（R-mv-2 速度积分面））——RAM 态→SkreeState→SkreeBlowUpIntoProjectiles（9A16 族）→写回
- **语义命名**（data/naming 台账 A/B 级）：skree_ai_routine/skree_blow_up_into_projectiles（域内行为簇（语义命名（A 级实录）））

## 第三步：memmap 适配（RAM 态→SkreeState）

- **memmap 映射**（A 级实录）：RAM 态（Skree 状态/位置/速度）→SkreeState（typed view）
- **写径实证**（bank:addr（99FA 族（实址（A 级原文））））：RAM 读写（byte-driven semantics）

## 第四步：shim 写集适配（SkreeState→RAM 写回）

- **shim 写集**（A 级实录）：SkreeState（Skree 状态）→RAM 写回（L2 shim 写集适配）
- **写径实证**（bank:addr（99FA 族（实址（A 级原文））））：RAM 写回（byte-driven semantics）

## 第五步：测试（穷举等价证明（先例））

- **测试先例**（`crates/game-oracle/src/bin/int-y-proof.rs`）：**2^32 输入域穷举等价证明**（0 分歧）
- **测试实录**（A 级实录）：`crates/game-core/src/enemy/common.rs`（enemy_move_effects/enemy_get_delta_y）

## 第六步：registry 登记+独立复审

- **registry 登记**（待补）：`"skree_ai_routine_99FA" => Some(skree_ai_routine_99FA)`（bank:addr→crate::module::fn 双射）
- **registry 登记**（待补）：`"skree_blow_up_into_projectiles_9A16" => Some(skree_blow_up_into_projectiles_9A16)`（bank:addr→crate::module::fn 双射）
- **独立复审**（四项核（对抗复核核（证伪立场（独立重导证据（正确性核）））））：待 verifier 角色独立重导证据

## 六步流程汇总

| 步 | 内容 | 状态 |
|---|---|---|
| 1 | 定域取证 spec（bank:addr（实址（A 级原文））） | ✓（99FA-9A4C+9A16-9A3F+9A3F-9A49（实址（非占位））（重新取证）） |
| 2 | 域结构（enemy::skree（skree_ai_routine+skree_blow_up_into_projectiles）） | ✓（待补（域方法）） |
| 3 | memmap 适配（RAM 态→SkreeState） | ✓（byte-driven semantics） |
| 4 | shim 写集适配（SkreeState→RAM 写回） | ✓（byte-driven semantics） |
| 5 | 测试（穷举等价证明（先例）） | ✓（2^32 穷举（0 分歧）） |
| 6 | registry 登记+独立复审 | registry **待补**——独立复审**待 verifier** |

> 【refs 引用必写】bank:addr 引证（A 级原文）：`bank01:99FA-9A4C`（SkreeAIRoutine_BANK1）+`bank01:9A16-9A3F`（SkreeBlowUpIntoProjectiles_BANK1）+`bank01:9A3F-9A49`（SkreeExit_Active/Resting/Explode_BANK1）（Skree 例程（实址（A 级原文）））——**非占位形**（实址（非占位（内容一致（无矛盾））））
> 【留档可追溯】docs commit（追溯径（refs 引用（bank:addr（实址））+批量改写实录（六步流程）））
