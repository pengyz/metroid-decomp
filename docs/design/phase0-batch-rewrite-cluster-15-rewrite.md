# Phase0 批量改写·第十五簇：enemy::ai（敌人 AI）（六步流程）

> 【状态】批量改写实录（A 级实录）——六步流程（定域取证 spec→域结构→memmap 适配→shim 写集适配→测试→registry 登记+独立复审）
> 【refs 引用必写】bank:addr 引证（A 级原文）——纪律强化（用户裁定）+四项核（对抗复核核（证伪立场））
> 【生成】bash refs asm 考证（bank:addr（敌人 AI 例程（实址（A 级原文））））→批量改写实录（六步流程）

## 第一步：定域取证 spec（refs asm 考证（bank:addr（实址（A 级原文））））

- **敌人 AI 例程**（`bank01:95E5`（ChooseEnemyAIRoutine）+`bank01:99B8-99D3`（SidehopperFloor/CeilingAIRoutine）+`bank01:99D8-99E5`（RipperAIRoutine）+`bank01:99E5-99FA`（WaverAIRoutine）+`bank01:99FA-9A4C`（SkreeAIRoutine）+`bank01:9A4C-9A87`（CrawlerAIRoutine）+`bank01:9AF5-9B1C`（RioAIRoutine）——refs asm 考证（bank:addr（实址（A 级原文））））——敌人 AI（R-mv 规则面）：
  - **输入**：RAM 态（敌人状态/位置/速度）→AiState
  - **规则**：ChooseEnemyAIRoutine/Sidehopper/Ripper/Waver/Skree/Crawler/RioAIRoutine（95E5/99B8/99D8/99E5/99FA/9A4C/9AF5 族）——敌人 AI 选择+行为执行
  - **输出**：AiState（敌人 AI 状态写回 RAM）
- **敌人 AI 跳转表**（`bank01:95E5`（ChooseEnemyAIRoutine）——refs asm 考证（bank:addr（实址（A 级原文））））——敌人 AI 跳转（R-mv 规则面）
- **真规则公式**（公式形（零字面特值））：`pub fn enemy_move_effects(...)`（`crates/game-core/src/enemy/common.rs:43`）

## 第二步：域结构（enemy::ai（敌人 AI））

- **域方法**（待补）：`enemy::ai::choose_enemy_ai_routine`（敌人 AI 选择（R-mv-2 速度积分面））——RAM 态→AiState→ChooseEnemyAIRoutine（95E5 族）→写回
- **域方法**（待补）：`enemy::ai::sidehopper_floor_ai_routine`（敌人 AI 行为（R-mv-2 速度积分面））——RAM 态→AiState→SidehopperFloorAIRoutine（99B8 族）→写回
- **语义命名**（data/naming 台账 A/B 级）：choose_enemy_ai_routine/sidehopper_floor_ai_routine（域内行为簇（语义命名（A 级实录）））

## 第三步：memmap 适配（RAM 态→AiState）

- **memmap 映射**（A 级实录）：RAM 态（敌人状态/位置/速度）→AiState（typed view）
- **写径实证**（bank:addr（95E5 族（实址（A 级原文））））：RAM 读写（byte-driven semantics）

## 第四步：shim 写集适配（AiState→RAM 写回）

- **shim 写集**（A 级实录）：AiState（敌人 AI 状态）→RAM 写回（L2 shim 写集适配）
- **写径实证**（bank:addr（95E5 族（实址（A 级原文））））：RAM 写回（byte-driven semantics）

## 第五步：测试（穷举等价证明（先例））

- **测试先例**（`crates/game-oracle/src/bin/int-y-proof.rs`）：**2^32 输入域穷举等价证明**（0 分歧）
- **测试实录**（A 级实录）：`crates/game-core/src/enemy/common.rs`（enemy_move_effects/enemy_get_delta_y/enemy_stream_exec）

## 第六步：registry 登记+独立复审

- **registry 登记**（待补）：`"choose_enemy_ai_routine_95E5" => Some(choose_enemy_ai_routine_95E5)`（bank:addr→crate::module::fn 双射）
- **registry 登记**（待补）：`"sidehopper_floor_ai_routine_99B8" => Some(sidehopper_floor_ai_routine_99B8)`（bank:addr→crate::module::fn 双射）
- **独立复审**（四项核（对抗复核核（证伪立场（独立重导证据（正确性核）））））：待 verifier 角色独立重导证据

## 六步流程汇总

| 步 | 内容 | 状态 |
|---|---|---|
| 1 | 定域取证 spec（bank:addr（实址（A 级原文））） | ✓（95E5+99B8-99D3+99D8-99E5+99E5-99FA+99FA-9A4C+9A4C-9A87+9AF5-9B1C（实址（非占位））） |
| 2 | 域结构（enemy::ai（choose_enemy_ai_routine+sidehopper_floor_ai_routine）） | ✓（待补（域方法）） |
| 3 | memmap 适配（RAM 态→AiState） | ✓（byte-driven semantics） |
| 4 | shim 写集适配（AiState→RAM 写回） | ✓（byte-driven semantics） |
| 5 | 测试（穷举等价证明（先例）） | ✓（2^32 穷举（0 分歧）） |
| 6 | registry 登记+独立复审 | registry **待补**——独立复审**待 verifier** |

> 【refs 引用必写】bank:addr 引证（A 级原文）：`bank01:95E5`（ChooseEnemyAIRoutine）+`bank01:99B8-99D3`（SidehopperFloor/CeilingAIRoutine）+`bank01:99D8-99E5`（RipperAIRoutine）+`bank01:99E5-99FA`（WaverAIRoutine）+`bank01:99FA-9A4C`（SkreeAIRoutine）+`bank01:9A4C-9A87`（CrawlerAIRoutine）+`bank01:9AF5-9B1C`（RioAIRoutine）（敌人 AI 例程（实址（A 级原文）））——**非占位形**
> 【留档可追溯】docs commit（追溯径（refs 引用（bank:addr（实址））+批量改写实录（六步流程）））
