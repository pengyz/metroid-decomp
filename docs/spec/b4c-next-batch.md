# B4-c：backlog 剩余面与下一批规格

> 用途：让下一轮 PMO 直接照本文建卡，不用再翻 `docs/spec/porting-backlog.md` 全文。
> 本文不重复 backlog 的描述，只做「已完成到哪 / 还剩什么 / 下一批取哪几条 / 怎么验收」。
> 数据源：`docs/spec/porting-backlog.md` §2（24 条）+ `git log` + 对 `crates/` 的 grep（2026-11-25）。

## 1. 现况基线（已 done 的卡与它们覆盖的条目）

| 卡 | 覆盖 backlog 条目 | 交付物 | 我核对的证据 |
|---|---|---|---|
| #40 | 不覆盖条目（产 backlog 本身） | `docs/spec/porting-backlog.md` | 154→155 行，603 分母 |
| #41 | 条目 **4**、**7–11**（`AddYToPtr00/Amul16/Adiv16/Amul8/Adiv8/TwosComplement`） | `crates/game-core/src/arith.rs` | 函数全部命中；`git log -- arith.rs` 最近一次改动=提交 `6a48c05d`（#41 批量）。**B 级**：函数是否全部由该批引入未逐函数追溯，只确认「当前树里有、且 #41 动过该文件」 |
| #42 | 条目 **1–3**（`WaitNMIPass/ScreenOff/ScreenOn`，规格层） | `docs/spec/timing-primitives.md` | 三例程逐行语义 + k 次调用契约；**代码**侧仍在 `frame/nmi` 内联，没有同名 `fn wait_nmi_pass`（见 §5） |
| #46 | 条目 **12–13**（`LoadTableAt977B`/`JumpEngine`，规格层） | `docs/spec/dispatch-tables.md` | 23/27 处调用点、表体逐项解码；**代码**侧 `LoadTableAt977B` 尚未成为单一 typed 实现（见 §2 #12） |
| #51 | 条目 **14–15**（`UpdateEnemyCommon` 三入口，B3） | `crates/game-core/src/enemy/framework.rs` + `docs/spec/enemy-common.md` | 五 bank 跳板 sha `c0af1d4d…`、实现 46 B `2ad4f045…`；typed 化 + 5 单测 |
| #53 | 条目 **21–22**（`InitSFXData`/`LoadSFXData`） | `crates/game-core/src/sound/sfx.rs` + `docs/spec/sfx-init.md` | 六 bank 同字节（39 B `398a9814…` / 65 B `a7b95cba…`）+ 5 单测 |
| （无本系列卡） | 条目 **23**（`IsSamusDead`） | `crates/game-core/src/samus/death.rs` | `fn is_samus_dead` 命中；`git log -S'fn is_samus_dead'` 指向更早的 1:1 族端口批（`d9fb5447` 一带），**不归属 #41–#53 任何一张** |
| #16 / #27 / #31 / #36 / #38 | **不覆盖 backlog 条目**（TAS 对拍/残差定界旁支） | 各自 findings | — |
| #47 / #49 | **不覆盖 backlog 条目**（TAS 修复：f1787 等） | `docs/findings/tas-divergence-f1787.md` 等 | — |

**B3 的「代码已在」不等于「条目已收」**：条目 16–19 的实现散落在
`enemy/motion.rs::apply_speed_to_position`、`enemy/sidehopper.rs::enemy_flip_after_displacement`、
`render/enemy_sprite.rs::draw_enemy`、`world/tile_blast.rs::draw_tile_blast`——它们有实现但没有
「同址核对表 + 单测 + spec 段」。所以本文把它们列为**已在但未收口**，不是「未开工」。

## 2. 剩余条目（backlog §2 中仍缺代码或未收口的）

| # | `bank:addr` | 例程 | 调用者数 | 系统 | 依赖前置 | 现状（grep/产物） |
|---|---|---|---|---|---|---|
| 5 | `07:F1EE` | `Yplus16` | 5 | collision | 无（`arith` 已有 `Yplus16` 的同族语义但无具名件） | **缺实现** |
| 6 | `03:9EF9` | `Xplus16` | 4 | collision（Tourian） | #5 同族；Mother Brain 战代码只到 memmap typed 面 | **缺实现** |
| 12 | `01–05:80B0` | `LoadTableAt977B` | 18 | enemy data | #46 已解表（A 级） | **语义已内联三处**（`world/room_draw.rs:861` 注释、`enemy/kraid.rs:163`、`frame/cycles.rs:239`），**无单一 typed 实现 + 无单测** |
| 16 | `01–05:8027` | `CommonJump_ApplySpeedToPosition` | 7 | enemy | #51 typed 面 | 已实现 `enemy/motion.rs::apply_speed_to_position`，**未收口**（无同址表/单测） |
| 17 | `01–05:801B` | `CommonJump_EnemyFlipAfterDisplacement` | 6 | enemy | #16 已修 data1F 符号 | 已实现（`sidehopper.rs::enemy_flip_after_displacement`），**未收口** |
| 18 | `01–05:803C` | `CommonJump_DrawEnemy` | 4 | enemy/render | #24 载房窗 OAM 态 | 已实现（`render/enemy_sprite.rs::draw_enemy`），**未收口** |
| 19 | `01–05:803F` | `CommonJump_DrawTileBlast` | 5 | enemy/render | #24 | 已实现（`world/tile_blast.rs::draw_tile_blast`），**未收口** |
| 20 | `01–05:8563` | `GetOtherNameTableIndex` | 8 | room/ppu | `enemy_move_one_pixel` 已有 | **缺实现** |
| 23 | `07:CE84` | `IsSamusDead` | 7 | player | 无 | 已实现（`samus/death.rs::is_samus_dead`），**未收口** |
| 24 | `07:E8BE` | `StoreObjectPositionToTemp` | 5 | player | 无 | 已实现（`samus/physics.rs::store_object_position_to_temp`），**未收口** |

**备选池（backlog 文末，同价值）**：`ClearAll`(0:909F,5)、`PrepareWriteVRAMStringRAM`(0:9450,5)、
`LoadRowAndColumn`(0:91BF,3)、`UpdateCrossMissile`(0:8963,7)、`InitPasswordFontGFX`(7:C6D6,3)、
`ReadTableAt968B`(7:F74B,4) —— 六条都**缺实现**，调用者数 3–7。

## 3. 下一批 3–6 条（按「无前置 → 有前置」排序）

排序原则：先挑**无前置、调用者多、且已有 L4 用例能立刻看出回归**的；把依赖 Tourian/Mother Brain
的放在后面（Tourian 段没有专属 L4 像素格，只能用 trace）。

### 批 A（建议本轮就开，3 条）

| 顺序 | 条目 | 依赖 | 可验收出口（命令 + 期望） | L4 覆盖档 |
|---|---|---|---|---|
| A1 | **#12 `LoadTableAt977B`**（18 调用者） | #46 规格 done | `cargo test -p game-core --lib load_table_at977b` 新增 ≥4 例（type 0–15 取表、`asl` 进位、`type≥16` 越界读）；`python3 tools/l4_diff_run.py run-all --skip-emu` 仍 **match 171 · diff 1**；`gates/cli.py check` PASS24 | **有像素格**（enemy 移动/被击退：`case-killenemy-probe`、`case-crawlprobe`） |
| A2 | **#20 `GetOtherNameTableIndex`**（8 调用者） | `enemy_move_one_pixel` 已有 | 单测 ≥3 例（左右/上下跨 NT、边界 x=0/$FF）；`run-all --skip-emu` 仍 171/1/0 | **有像素格**（`case-scrollseam-probe`） |
| A3 | **#5 `Yplus16`**（5 调用者） | 无 | 单测 = 与 `Xplus16` 同族的加法向量（`$F1EE` 语义：16 位 Y 加 8 位偏移）；`run-all` 171/1/0 | **有像素格**（`case-collide-probe`） |

### 批 B（批 A 落地后开，2 条）

| 顺序 | 条目 | 依赖 | 可验收出口 | L4 覆盖档 |
|---|---|---|---|---|
| B1 | **#6 `Xplus16`**（4 调用者） | A3 同族（可复用向量） | 单测 4 例（Mother Brain 玻璃/Zebetite 碰撞坐标）；`run-all` 171/1/0 | **仅 trace**——Tourian 段无专属 L4 格（backlog §5 缺口），出口改为 `python3 gates/cli.py trace` 不新增长度 + 单测 |
| B2 | **`UpdateCrossMissile`**(0:8963,7)（备选池） | 无 | 单测 ≥3 例（十字弹四向、撞墙消失）；`run-all` 171/1/0 | **有像素格**（`case-shootprobe-probe`） |

### 收口批（不新开实现，只补「已实现未收口」的 6 条）

条目 16/17/18/19/24（+ `ClearAll` 若选）不写新逻辑，只补「同址核对表 + ≥2 单测 + spec 段」。
出口统一：每条一张表 + `cargo test` 增量 + `run-all` 不回退。**不建议排在 A/B 前面**——它们不改行为，
占用写面却换不来覆盖增量；等 A/B 做完再一起收口。

### 为什么「无外部可观测」档这次是空的

本轮候选全都能落进 L4 像素格或 trace 面（见上表）。若后续要开播放侧条目（如 `ClearAll` 的屏幕
清空），才需要「无外部可观测」档——那时判据只能是「写序 + 状态快照」，要单独说明为什么还值得做。

## 4. 去重声明（防把已做的再列一遍）

- 条目 **4、7–11** 已由 #41 落地（`arith.rs`），本文未再列为剩余。
- 条目 **1–3** 的规格由 #42 完成；本文只把「代码侧仍是内联、无同名函数」记为 §5 备注，**不重开规格**。
- 条目 **12–13** 的**规格**由 #46 完成；本文只把 #12 的「代码未收口」列为剩余（#13 `JumpEngine` 已在
  `frame/` 内联并有过 trace 验证，不列）。
- 条目 **14–15** 由 #51 落地；本文不列。
- 条目 **21–22** 由 #53 落地；本文不列。
- 条目 **23**（`IsSamusDead`）代码已在 `samus/death.rs`，但**不是** #41–#53 某张卡的产物（见 §1 第 5 行）；本文按「已在、未收口」处理，不列为「未开工」。
- #47/#49 是 TAS 修复，不属 backlog 条目，本文不把它们算作「已收的 backlog 项」。
- **已实现但未收口**的 16/17/18/19/23/24 在 §2 单列一行「未收口」，既不冒充「未开工」，也不冒充「已 done」。
- 归属口径：只有 #40（backlog 本身）/ #42 / #46（两份规格）/ #51 / #53 有明确的本系列卡归属；其余实现来自更早的 1:1 族端口批，本文不按卡号追认。

## 5. 备注与未覆盖

- **条目 1–3 的代码位置**：`WaitNMIPass/ScreenOff/ScreenOn` 的语义已在 `frame/nmi.rs`/`frame/cycles.rs`
  内联（#42 只做规格），没有 `fn wait_nmi_pass` 这类具名件。若下一轮要「同名函数化」，那是一次
  纯重构，验收只看 `run-all` 与 `gates check` 不回退，不新增行为覆盖。
- **#13 `JumpEngine`**：`frame/` 内已实现且有 trace 佐证，本文不把它列为剩余；若要补 spec 段，
  与 #46 的表解码合并即可。
- **Tourian 缺口**：条目 6 与 Mother Brain 相关路径没有专属 L4 像素格（backlog §5 已声明），
  下一批只能给「trace 不回退 + 单测」出口，不硬造像素判据。
- **调用者数**口径：`grep -rE '^[[:space:]]*(jsr|jmp)[[:space:]]+<Label>\b' refs/m1disasm/SRC --include=*.asm | wc -l`
  （只计直接 `jsr/jmp`，不含条件跳转/包装）。复跑：`Yplus16`=5、`GetOtherNameTableIndex`=8、`LoadTableAt977B`=18、`Xplus16`=4。
  §2 初版把 `Yplus16` 写成 7（从别处抄来的数），已按复核改为 5——两处（§2 表、§3 A3）同改。
- **`docs/spec/porting-backlog.md:124` 那句「`Xplus16/Yplus16` 已在批 1」指的是「已列进批 1 的**计划**」**，
  不是已实现：`grep -rl 'fn y_plus16\|fn x_plus16' crates/` 零命中，§2 的「缺实现」以 grep 为准。
  下一位读者若拿 :124 来质疑 §2，按本条解释即可（不改 backlog 原文，避免与它自己的口径冲突）。
