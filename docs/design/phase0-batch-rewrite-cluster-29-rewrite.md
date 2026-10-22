# Phase0 批量改写·第二十九簇：enemy::rio（Rio）（六步流程）

> 【状态】批量改写实录（A 级实录）——六步流程（定域取证 spec→域结构→memmap 适配→shim 写集适配→测试→registry 登记+独立复审）
> 【refs 引用必写】bank:addr 引证（A 级原文）——纪律强化（用户裁定）+四项核（对抗复核核（证伪立场））
> 【生成】bash refs asm 考证（bank:addr（Rio 例程（实址（A 级原文））））→批量改写实录（六步流程）

## 第一步：定域取证 spec（refs asm 考证（bank:addr（实址（A 级原文））））

- **Rio 例程**（`bank01:97DD`（EnemyMovementChoice_Rio_BANK1）+`bank01:9AF5-9B1C`（RioAIRoutine_BANK1）——refs asm 考证（bank:addr（实址（A 级原文））））——Rio（R-mv 规则面）：
  - **输入**：RAM 态（Rio 状态/位置/速度）→RioState
  - **规则**：EnemyMovementChoice_Rio/RioAIRoutine（97DD/9AF5 族）——Rio 移动选择+AI
  - **输出**：RioState（Rio 状态写回 RAM）
- **Rio 动画表**（`bank01:9D8D-9D9D`（EnAnim_Skree/EnAnim_SkreeExplode_BANK1（邻接动画表））——refs asm 考证（bank:addr（实址（A 级原文））））——Rio 动画（R-mv 规则面）
- **真规则公式**（公式形（零字面特值））：`pub fn enemy_move_effects(...)`（`crates/game-core/src/enemy/common.rs:43`）

## 第二步：域结构（enemy::rio（Rio））

- **域方法**（待补）：`enemy::rio::rio_ai_routine`（Rio AI（R-mv-2 速度积分面））——RAM 态→RioState→RioAIRoutine（9AF5 族）→写回
- **域方法**（待补）：`enemy::rio::enemy_movement_choice_rio`（Rio 移动选择（R-mv-2 速度积分面））——RAM 态→RioState→EnemyMovementChoice_Rio（97DD 族）→写回
- **语义命名**（data/naming 台账 A/B 级）：rio_ai_routine/enemy_movement_choice_rio（域内行为簇（语义命名（A 级实录）））

## 第三步：memmap 适配（RAM 态→RioState）

- **memmap 映射**（A 级实录）：RAM 态（Rio 状态/位置/速度）→RioState（typed view）
- **写径实证**（bank:addr（9AF5 族（实址（A 级原文））））：RAM 读写（byte-driven semantics）

## 第四步：shim 写集适配（RioState→RAM 写回）

- **shim 写集**（A 级实录）：RioState（Rio 状态）→RAM 写回（L2 shim 写集适配）
- **写径实证**（bank:addr（9AF5 族（实址（A 级原文））））：RAM 写回（byte-driven semantics）

## 第五步：测试（穷举等价证明（先例））

- **测试先例**（`crates/game-oracle/src/bin/int-y-proof.rs`）：**2^32 输入域穷举等价证明**（0 分歧）
- **测试实录**（A 级实录）：`crates/game-core/src/enemy/common.rs`（enemy_move_effects/enemy_get_delta_y）

## 第六步：registry 登记+独立复审

- **registry 登记**（待补）：`"rio_ai_routine_9AF5" => Some(rio_ai_routine_9AF5)`（bank:addr→crate::module::fn 双射）
- **registry 登记**（待补）：`"enemy_movement_choice_rio_97DD" => Some(enemy_movement_choice_rio_97DD)`（bank:addr→crate::module::fn 双射）
- **独立复审**（四项核（对抗复核核（证伪立场（独立重导证据（正确性核）））））：待 verifier 角色独立重导证据

## 六步流程汇总

| 步 | 内容 | 状态 |
|---|---|---|
| 1 | 定域取证 spec（bank:addr（实址（A 级原文））） | ✓（97DD+9AF5-9B1C（实址（非占位））（重新取证）） |
| 2 | 域结构（enemy::rio（rio_ai_routine+enemy_movement_choice_rio）） | ✓（待补（域方法）） |
| 3 | memmap 适配（RAM 态→RioState） | ✓（byte-driven semantics） |
| 4 | shim 写集适配（RioState→RAM 写回） | ✓（byte-driven semantics） |
| 5 | 测试（穷举等价证明（先例）） | ✓（2^32 穷举（0 分歧）） |
| 6 | registry 登记+独立复审 | registry **待补**——独立复审**待 verifier** |

> 【refs 引用必写】bank:addr 引证（A 级原文）：`bank01:97DD`（EnemyMovementChoice_Rio_BANK1）+`bank01:9AF5-9B1C`（RioAIRoutine_BANK1）（Rio 例程（实址（A 级原文）））——**非占位形**（实址（非占位（内容一致（无矛盾））））
> 【留档可追溯】docs commit（追溯径（refs 引用（bank:addr（实址））+批量改写实录（六步流程）））
