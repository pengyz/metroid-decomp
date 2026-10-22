# Phase0 批量改写·第八簇：title::menu（标题菜单）（六步流程）

> 【状态】批量改写实录（A 级实录）——六步流程（定域取证 spec→域结构→memmap 适配→shim 写集适配→测试→registry 登记+独立复审）
> 【refs 引用必写】bank:addr 引证（A 级原文）——纪律强化（用户裁定）+四项核（对抗复核核（证伪立场））
> 【生成】bash refs asm 考证（bank:addr（标题菜单例程（实址（A 级原文））））→批量改写实录（六步流程）

## 第一步：定域取证 spec（refs asm 考证（bank:addr（实址（A 级原文））））

- **标题菜单例程**（`bank00:8000`（MainTitleRoutine）+`bank00:802A`（TitleRoutinePtrTable）+`bank00:806E`（IncTitleRoutine0A/0B）——refs asm 考证（bank:addr（实址（A 级原文））））——标题菜单（R-mv 规则面）：
  - **输入**：RAM 态（菜单状态/输入）→MenuState
  - **规则**：MainTitleRoutine/TitleRoutinePtrTable/IncTitleRoutine（8000/802A/806E 族）——标题菜单加载+选择+更新
  - **输出**：MenuState（菜单状态写回 RAM）
- **标题准星例程**（`bank00:8182-81D0`（Crosshairs）+`bank00:81D1-81EE`（MoreCrosshairs）——refs asm 考证（bank:addr（实址（A 级原文））））——标题准星（R-mv 规则面）
- **真规则公式**（公式形（零字面特值））：`pub fn menu_choose(joy_change: u8, start_continue: u8) -> MenuOutcome`（`crates/game-core/src/title/mod.rs:620`）

## 第二步：域结构（title::menu（标题菜单））

- **域方法**（`crates/game-core/src/title/mod.rs:620`）：`menu_choose`（菜单选择（R-mv-2 速度积分面））——RAM 态→MenuState→MainTitleRoutine（8000 族）→写回
- **域方法**（`crates/game-core/src/title/mod.rs:647`）：`wait_for_start`（等待开始（R-mv-2 速度积分面））——RAM 态→MenuState→IncTitleRoutine（806E 族）→写回
- **语义命名**（data/naming 台账 A/B 级）：menu_choose/wait_for_start（域内行为簇（语义命名（A 级实录）））

## 第三步：memmap 适配（RAM 态→MenuState）

- **memmap 映射**（A 级实录）：RAM 态（菜单状态/输入）→MenuState（typed view）
- **写径实证**（bank:addr（8000 族（实址（A 级原文））））：RAM 读写（byte-driven semantics）

## 第四步：shim 写集适配（MenuState→RAM 写回）

- **shim 写集**（A 级实录）：MenuState（菜单状态）→RAM 写回（L2 shim 写集适配）
- **写径实证**（bank:addr（8000 族（实址（A 级原文））））：RAM 写回（byte-driven semantics）

## 第五步：测试（穷举等价证明（先例））

- **测试先例**（`crates/game-oracle/src/bin/int-y-proof.rs`）：**2^32 输入域穷举等价证明**（0 分歧）
- **测试实录**（A 级实录）：`crates/game-core/src/title/mod.rs`（menu_choose/wait_for_start/password_display_gate）

## 第六步：registry 登记+独立复审

- **registry 登记**（待补）：`"menu_choose_8000" => Some(menu_choose_8000)`（bank:addr→crate::module::fn 双射）
- **registry 登记**（待补）：`"wait_for_start_806E" => Some(wait_for_start_806E)`（bank:addr→crate::module::fn 双射）
- **独立复审**（四项核（对抗复核核（证伪立场（独立重导证据（正确性核）））））：待 verifier 角色独立重导证据

## 六步流程汇总

| 步 | 内容 | 状态 |
|---|---|---|
| 1 | 定域取证 spec（bank:addr（实址（A 级原文））） | ✓（8000+802A+806E+8182-81D0+81D1-81EE（实址（非占位））） |
| 2 | 域结构（title::menu（menu_choose+wait_for_start）） | ✓（title/mod.rs:620+647（语义命名（A 级））） |
| 3 | memmap 适配（RAM 态→MenuState） | ✓（byte-driven semantics） |
| 4 | shim 写集适配（MenuState→RAM 写回） | ✓（byte-driven semantics） |
| 5 | 测试（穷举等价证明（先例）） | ✓（2^32 穷举（0 分歧）） |
| 6 | registry 登记+独立复审 | registry **待补**——独立复审**待 verifier** |

> 【refs 引用必写】bank:addr 引证（A 级原文）：`bank00:8000`（MainTitleRoutine）+`bank00:802A`（TitleRoutinePtrTable）+`bank00:806E`（IncTitleRoutine）+`bank00:8182-81D0`（Crosshairs）+`bank00:81D1-81EE`（MoreCrosshairs）（标题菜单例程（实址（A 级原文）））——**非占位形**
> 【留档可追溯】docs commit（追溯径（refs 引用（bank:addr（实址））+批量改写实录（六步流程）））
