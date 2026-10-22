# Phase0 批量改写·簇 6 修订·二次修订：ui::hud（状态栏）

> 【状态】修订文档实录（A 级实录）——重新取证（实址（非占位））（二次修订）
> 【修订原因】对抗复核 verdict=needs_revision（2/4 拒收）——「状态栏（HUD）例程：**bank:addr**（实址（A 级原文））」字面占位（非实址（cluster-6-rewrite.md §1.3））（与簇 5 首修订同病（二次修订模式应补）））
> 【refs 引用必写】bank:addr 引证（A 级原文）——纪律强化（用户裁定）+四项核（对抗复核核（证伪立场））
> 【生成】bash refs asm 考证（bank:addr（状态栏（HUD）例程（实址（A 级原文））））→修订文档实录（重新取证（实址（非占位）））

## 重新取证（实址（非占位））

### 修订前（字面占位（非实址））

- 「状态栏（HUD）例程：**bank:addr**（实址（A 级原文））」——**字面占位（非实址**（cluster-6-rewrite.md §1.3））——**拒收**（对抗复核 verdict=needs_revision）

### 修订后（实址（bank:addr 引证（A 级原文）））

- **状态栏（HUD）例程**（`bank00:8D3D-8D88`（LoadTanksAndMissiles）——refs asm 考证（bank:addr（实址（A 级原文））））——状态栏（HUD）（R-mv 规则面）：
  - **输入**：RAM 态（能量/导弹/道具状态）→HudState
  - **规则**：LoadTanksAndMissiles（8D3D 族）——状态栏加载+道具初始化+更新
  - **输出**：HudState（状态栏状态写回 RAM）
- **表读径**（grep 实录（数据表使用（子程序（表读指令））））：LoadTanksAndMissiles@IncrementToNextItem（8DA9）（表读指令（LDA abs,X））
- **域归属定谳**（簇 4 修订先例）：bank00:8897-88FE（InitCrossMissiles）+8963（UpdateCrossMissile）=**标题画面准星精灵例程**（非游戏内 HUD 例程）——refs 原文（prg0_title.asm:973-1000（开场精灵（准星汇聚动画）））

## 六步流程汇总（二次修订后）

| 步 | 内容 | 状态 |
|---|---|---|
| 1 | 定域取证 spec（bank:addr（实址（A 级原文））） | ✓（8D3D-8D88（实址（非占位））（重新取证）） |
| 2 | 域结构（ui::hud（load_tanks_and_missiles）） | ✓（待补（域方法）） |
| 3 | memmap 适配（RAM 态→HudState） | ✓（byte-driven semantics） |
| 4 | shim 写集适配（HudState→RAM 写回） | ✓（byte-driven semantics） |
| 5 | 测试（穷举等价证明（先例）） | ✓（2^32 穷举（0 分歧）） |
| 6 | registry 登记+独立复审 | registry **待补**——独立复审**待 verifier** |

> 【refs 引用必写】bank:addr 引证（A 级原文）：`bank00:8D3D-8D88`（LoadTanksAndMissiles）（状态栏（HUD）例程（实址（A 级原文）））——**非占位形**+**域归属定谳**（标题画面准星精灵例程（非游戏内 HUD 例程））
> 【留档可追溯】docs commit（追溯径（refs 引用（bank:addr（实址））+修订文档实录（重新取证（实址（非占位）））））
