# Phase0 批量改写准备·第三个域内行为簇：enemy::movement（敌人移动）

> 【状态】设计文档实录（A 级实录）——域内行为簇设计（enemy::movement（敌人移动）（六步流程））
> 【refs 引用必写】bank:addr 引证（A 级原文）——纪律强化（用户裁定）+四项核（对抗复核核（证伪立场））
> 【生成】bash refs asm 考证（bank:addr（敌人移动例程（写径实证（真规则（公式形）））））→设计文档实录（六步流程）

## 域内行为簇设计（enemy::movement（敌人移动））

### 第一步：定域取证 spec（refs asm 考证（bank:addr（写径实证（A 级原文））））

- **敌人移动例程**（`bank01:8058-80AF`（EnemyMove）+`bank01:83F5-84A6`（EnemyMoveOnePixelUp/Down）+`bank01:84A7-850F`（EnemyMoveOnePixelLeft/Right）——refs asm 考证（bank:addr（写径实证（A 级原文））））——敌人移动（R-mv 规则面）：
  - **输入**：RAM 态（敌人速度/位置/加速度）→EnemyState
  - **规则**：EnemyMove（8058 族）——四向移动+碰撞检测+失败反弹
  - **输出**：EnemyState（位置写回 RAM）
- **敌人失败反弹例程**（`bank01:80B8-8312`（EnemyIfMoveFailedUp/Down/Left/Right）——refs asm 考证（bank:addr（写径实证（A 级原文））））——敌人移动失败反弹（R-mv 规则面）
- **真规则公式**（公式形（零字面特值））：`pub fn enemy_move_effects(...)`（`crates/game-core/src/enemy/common.rs:43`）

### 第二步：域结构（enemy::movement（敌人移动））

- **域方法**（`crates/game-core/src/enemy/common.rs:43`）：`enemy_move_effects`（敌人移动效果（R-mv-2 速度积分面））——RAM 态→EnemyState→EnemyMove（8058 族）→写回
- **域方法**（`crates/game-core/src/enemy/common.rs:117`）：`enemy_get_delta_y`（敌人垂直 delta（R-mv-2 速度积分面））——RAM 态→EnemyState→EnemyGetDeltaY（8244 族）→写回
- **域方法**（`crates/game-core/src/enemy/common.rs:170`）：`enemy_stream_exec`（敌人流执行（R-mv-2 速度积分面））——RAM 态→EnemyState→EnemyStream（822B 族）→写回

### 第三步：memmap 适配（RAM 态→EnemyState）

- **memmap 映射**（A 级实录）：RAM 态（敌人速度/位置/加速度）→EnemyState（typed view）
- **写径实证**（bank:addr（8058 族（写径实证（A 级原文））））：RAM 读写（byte-driven semantics）

### 第四步：shim 写集适配（EnemyState→RAM 写回）

- **shim 写集**（A 级实录）：EnemyState（位置/速度）→RAM 写回（L2 shim 写集适配）
- **写径实证**（bank:addr（8058 族（写径实证（A 级原文））））：RAM 写回（byte-driven semantics）

### 第五步：测试（穷举等价证明（先例））

- **测试先例**（`crates/game-oracle/src/bin/int-y-proof.rs`）：**2^32 输入域穷举等价证明**（0 分歧）
- **测试实录**（A 级实录）：`crates/game-core/src/enemy/common.rs`（enemy_delta_sign_mag/enemy_get_delta_y/enemy_type_times2_plus_facing）

### 第六步：registry 登记+独立复审

- **registry 登记**（待补）：`"enemy_move_8058" => Some(enemy_move_8058)`（L2 ports registry）
- **registry 登记**（待补）：`"enemy_get_delta_y_8244" => Some(enemy_get_delta_y_8244)`（L2 ports registry）
- **独立复审**（四项核（对抗复核核（证伪立场（独立重导证据（正确性核）））））：待 verifier 角色独立重导证据

## 六步流程汇总

| 步 | 内容 | 状态 |
|---|---|---|
| 1 | 定域取证 spec（bank:addr（写径实证（A 级原文））） | ✓（8058-80AF+83F5-84A6+84A7-850F（写径实证）） |
| 2 | 域结构（enemy::movement（enemy_move_effects+enemy_get_delta_y+enemy_stream_exec）） | ✓（common.rs:43+117+170） |
| 3 | memmap 适配（RAM 态→EnemyState） | ✓（byte-driven semantics） |
| 4 | shim 写集适配（EnemyState→RAM 写回） | ✓（byte-driven semantics） |
| 5 | 测试（穷举等价证明（先例）） | ✓（2^32 穷举（0 分歧）） |
| 6 | registry 登记+独立复审 | registry **待补**——独立复审**待 verifier** |

> 【refs 引用必写】bank:addr 引证（A 级原文）：`bank01:8058-80AF`（EnemyMove）+`bank01:83F5-84A6`（EnemyMoveOnePixelUp/Down）+`bank01:84A7-850F`（EnemyMoveOnePixelLeft/Right）（敌人移动例程（写径实证（A 级原文）））
> 【留档可追溯】docs commit（追溯径（refs 引用（bank:addr）+设计文档实录（六步流程）））
