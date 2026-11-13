# 五大子系统：数据建模与主干流程完备性审计

> 2026-11-13 · 用户令（m13669）「抢救性修复：看五个最重要子系统的状态——
> 数据建模完整吗？主干流程完备吗？」·
> 取证=**读代码 + 读 asm + 已有实测基线**（`docs/findings/sem-baseline-all-cases.md`
> 的 16300 帧语义基线、L4 112 格）· Evidence **A/B** 混合，逐条标注 · Confidence 90

**一句话结论**：**数据建模在「表/结构」层面基本齐（唯一大面积缺口=场景数据 2/5 区），
但主干流程有 4 处硬断点**——死亡→GameOver、暂停、区域间移动、后三区数据。
这四处都在「玩家必然会走到」的路径上，且其中三处是 **panic 级**（`unported!` 直接炸）。

---

## 0. 结论速览

| 子系统 | 数据建模 | 主干流程 | 判定 |
|---|---|---|---|
| 1 主流程状态机 | 表齐（10 / 31 / 9 项） | `MainRoutine` **6/10** 臂；`TitleRoutine` **31/31**；`BankInitTable` **3/9** | **4 处断点** |
| 2 Samus 状态 | Object 15 字段齐、`sa_` 11 态齐、`wa_` 11 态齐 | 走/跑/跳/球/爬/门/电梯态在；出门链 4 桩、岩浆伤 1 桩 | **主循环可用，边角有桩** |
| 3 场景加载/状态 | AreaPointers **5/5**、特殊道具 **5/5**、房流 **2/5**、敌四表 **1/5**、音乐数据已烘 | Brinstar/Norfair 内完整；电梯=空桩；后三区 `unported!` | **2/5 世界可玩** |
| 4 声音 | 39 个音效 ID + 音乐表/流已烘；28 handler 臂 | `SoundEngine` 骨架完整；**24/28 臂零触达**、2 件高频走未接径 | **骨架在、内容缺、无裁判** |
| 5 渲染 | CHR/NT/OAM/PAL 四面齐；GFX 全 29 项间接读；调色板表齐 | NMI 渲染管线完整；L4 **110/112 格 0px** | **最完整** |

---

## 1. 游戏主流程状态机

### 数据建模：齐

三轴 + 六辅助变量全部建模（`docs/spec/core-state-machine.md` §1，逐条 bank:addr 锚）：

| 轴 | 地址 | 项数 | Rust 覆盖 |
|---|---|---|---|
| `GameMode` | `$1D` | 2 值（0 游戏 / ≠0 标题） | ✓ 门在 `game_main_routine` 头部 |
| `MainRoutine` | `$1E` | 10（表 `07:C141`） | **6/10** |
| `TitleRoutine` | `$1F` | 31（表 `00:802A`） | **31/31 ✓** |
| `BankInitTable` | `07:C51F` | 9（InitBank0-5 + ExitSub×3） | **3/9** |

> 更正记录：初次 grep 把 `TitleRoutine` 数成 22 臂，实为合并臂写法
> （`0x0A | 0x0B`、`0x0D | 0x0E | 0x02 | 0x03 | 0x07`、`0x13 | 0x14`、`0x15 | 0x1B`）
> ——**实际 31/31 全覆盖**，标题轴无缺口。

### 主干流程：4 处硬断点

`crates/game-core/src/frame/mainloop.rs:109` 的 `match mr` 只覆盖 `0/1/2/3/8/9`，
其余落入 `_ => unported!("bank07:{表目标地址}")` —— **panic，不是空实现**：

| 缺臂 | asm | 触发条件 | 后果 |
|---|---|---|---|
| **4 `PrepareGameOver $C9A6`** | `TitleRoutine←GameOver` / `BankSwitchPending←1` / `jmp ScreenOff` | `game_engine` 已正确置 `NextRoutine←4`（`mainloop.rs:2960` 区）+ `WaitTimer(9)` 会消费它 | **死亡后 panic**（`bank07:C9A6`） |
| **5 `PauseMode $C9B1`** | 要求 `Joy2Status & (A\|UP)`、`EndTimer+1==0`、`GamePaused←0`、`inc MainRoutine`→6 | 游戏内按 START | 更上游就断了：`GoMainRoutine` 的 START 开关（`MainRoutine` 3↔5 + `GamePaused ^= 1` + `PauseMusic`）**完全未实现**（`game_main_routine` 头部无任何 `Joy1Change & START`），所以**按 START 静默无效** |
| **6 `GoPassword $C9C4`** | `TitleRoutine←DisplayPassword` / `BankSwitchPending←1` / `SilenceMusic` / `jmp ScreenOff` | `NextRoutine←6`（`WaitTimer` 已能正确置） | panic（逻辑在 `ui/password.rs::go_password()` 已实现，**只缺接线**） |
| **7 `IncrementRoutine $C155`** | `inc MainRoutine / rts` | 真实流程不经过（`MoreInit` 尾是内联 `inc`） | 影响低；但 `is_engine_running` 依赖 `mr==7` 语义（已按判据实现） |

`GamePaused($31)` 的状态更说明问题：**全仓只写不读**（`crates/game-core/src/frame/boot.rs:311`
初始化为 0，此后无任何读取点）——数据位建了，语义没建。

`BankInitTable` 只接 `0/1/2`：`InitBank3/4/5`（Kraid/Tourian/Ridley）与 `ExitSub×3` 未接。

---

## 2. Samus 状态

### 数据建模：齐

- **Object 结构 15 字段**（`constants_ram.asm:217-239`）：`status/radiusY/radiusX/animFrame/
  animDelay/animResetIndex/animIndex/data07/speedY/speedX/isHit/onScreen/hi/y/x` —— Rust 有
  对应 `SAMUS_*` / 槽基址 `$0300+n*0x10` 映射。
- **`sa_` 11 态**（`constants.asm:88-98`）：`Stand0/Run1/Jump2/Roll3/PntUp4/Door5/PntJump6/
  Dead7/Dead2(8)/Elevator9/Begin(255)` —— 全部有对应处理（`handler.rs`）。
- **`wa_` 11 态**（`constants.asm:100-111`）+ `wa_ScrewAttack=$81`。
- **`enemyStatus` 7 态**（`:119-127`）：`NoEnemy/Resting/Active/Explode/Frozen/Pickup/Hurt` ——
  `mainloop.rs:555-559` 五臂 + 其余按 asm 语义（>6 同 NoEnemy）。
- **`isHit` 位语义**（`constants_ram.asm:227-233`）：`$20` 炸弹伤 / `$30` 敌伤 / `+$08` 右向 /
  `$44` 实体接触 / `+$01` 右侧 / `+$02` 下方 —— 已按位使用，**组合语义未逐条对证**（B 级）。

### 主干流程：主循环可用，边角有桩

**可用**：走/跑/跳/球/爬（`handler.rs` + `physics.rs`，13 个语义用例 100% 全同覆盖）、
门（`sa_Door` + `DoorDelay`）、电梯**态**（`sa_Elevator` 状态机）、碰撞（`collision_point_ok`）、
跳台边（`jumpledge`）、变球（`roll`）、射击杀敌（`shoot/kill`）。

**桩（`unported!`，走到就炸）**：`samus/handler.rs` 的 `MoveOutDoor $D3AE`、
`$D3BA`（完成相敌清链）、`$D42E`（方向/滚动链）——**出门链**；`samus/physics.rs` 的
`$E269`（岩浆伤余件）。

**影响判定**：主循环（在同一房间内移动/战斗）可用；**跨房间移动（走门）会炸**。

---

## 3. 场景加载 / 场景状态

### 数据建模：一处大面积缺口

| 数据 | 覆盖 | 证据 |
|---|---|---|
| `AreaPointers`（7 指针 ×5 区） | **5/5 ✓** | `mainloop.rs` `AREA_POINTERS_BY_AREA[[u8;14];5]`（cde9ba9c，ROM 逐字节核验） |
| 特殊道具表 `SpecItmsTbl` | **5/5 ✓** | `game-data/src/area/items.rs` 五常量 |
| 房流 / 结构 / 元 tile / 结构指针 | **2/5** | `game-data/src/area/mod.rs` `room_ptrs()` 只有 area 0（Brinstar）、1（Norfair），2/3/4 → `None` → 调用方 `unported!` |
| 敌四表（Health/ForceSpeed/InitDelay/Facing） | **1/5** | `room.rs:604-606` 直接读 `area::graphics::ENEMY_*`（Brinstar 单区，无分发） |
| 音乐表 / 音乐流 | 已烘 ✓ | `game-data/src/sound/{music_tables,music_streams,sound_data}.rs` |

> 敌表**尺寸疑点未解**（`docs/findings` 之外的登记）：asm `EnemyHealthTbl ; 01:962B` 起
> `ForceSpeed`/`DistanceThreshold`/`InitDelay` 三段各 16B，而 Rust 烘成 `[u8;64]`
> —— 可能是跨段拼读，未对证。分区前必须先解。

### 主干流程：Brinstar/Norfair 内完整，跨区断

- `SetupRoom → DrawRoom → DrawObject → DrawStruct → DrawMetatile → FillRoomRAM →
  AttribTableWrite` 全链已端口（`room.rs`，per-area 分发 cde9ba9c）。
- `RoomNumber` 特殊值链（`$FF` 无房 / `$FE` / `$F0-$F4` 属性链）✓。
- **电梯 = 空桩**：`mainloop.rs:2436 uw_elevator(_mem)` 注释「槽空⇒跳过」，函数体为空。
  **区域间移动（Brinstar↔Norfair↔…）没有实现**——而这正是原版的主进程路径。
- 后三区进不去（两重原因叠加）：`InitBank3/4/5` 未接 + 房流未烘。

---

## 4. 声音子系统

### 数据建模：ID 与数据齐

- **39 个音效 ID**（`constants.asm:130-176`）：`sfxNoise` 7 / `sfxSQ1` 8 / `sfxTri` 8 /
  `sfxMulti` 8 / `music` 8。
- 音乐数据表 + 音乐流已烘（`game-data/src/sound/` 三件）。
- 音频 RAM 契约（`$0600-$068D` 27 个活跃地址）已实证（C→A）。

### 主干流程：骨架完整，内容大面积未验证

- `SoundEngine ; $B3B4` 骨架已端口（NMI 内：`lda SFXNoiseInitFlags / lsr / bcs 静音径` →
  `GameIsPaused` 径 → 四族 init + 四族 cont + `MusicLoop` + `ClearInitFlags`）。
- **28 个 handler 臂已接，只有 4 个被任何用例触达**（`docs/findings/sound-handler-exposure.md`）：
  `B7D9 BeepSFXInit`、`B878 BombLaunchSFXInit`、`BC7A GotoMusic01Init`、`BC80 GotoMusic03Init`。
  零触达 24 件含 SpitFlame/MissileLaunch/BombExplode/SamusWalk/SamusJump/EnemyHit/BulletFire/
  DoorOpenClose/SamusDie/StatueRaise/MetroidHit…（每件带 `sound_engine.asm` 行号锚）。
- **两件高频未接**：`B7E7 BeepSFXCont`（password 案 526 次）、`B87F BombLaunchSFXCont`（109 次）
  走 `_ => None` → 进 `DIVERGENCES` 记账（**不是静默**，但账内容忍且无增长检测——已由
  「增长即红门」补上 `data/status/handler_touchlist.json` 契约）。
- `GameIsPaused` 径仍 `unported!("bank07:B3EB-paused")`（MR==5 触发，当前不可达）。

**关键定性**：**声音面没有任何裁判**。L4 只比像素（声音不影响画面），sem 15 列不含音频 RAM。
所以「28 臂接了」只有代码存在性证明，没有行为等价证明。

---

## 5. 渲染子系统

### 数据建模：最完整

- **CHR / NT / OAM / PAL 四面**：竖镜映射（`$2000≡$2400`、`$2800≡$2C00`，`render.rs`
  `write_ppudata` + `render/mod.rs bg_lookup` + dump 出口三处一致）、CIRAM 初值 `$FF`、
  属性表 32B 整行（`WritePPUAttribTbl $E5E2`）。
- **GFX 全 29 项经 ROM 表间接读**（`gfx.rs` `GFX_INFO[id*7..]` + `GFX_DATA/GFX_OFFS`，
  d154544a 去重；三份等价性 29/29 逐字节 True）。
- **调色板表**：`PalettePtrTable` bank0=19 项 / bank1-5=28 项（C1FF 卡定谳）。
- 精灵：`render/sprite.rs`（优先级、翻转、8×16 形）。
- 原版 quirk（8 sprites/scanline、闪烁）按 D4 保真登记，不修。

### 主干流程：完备

NMI 渲染管线逐项端口：`NMIScreenWrite / CheckPaletteWrite / CheckVRAMStructBufferWrite /
WritePPUCtrl（唯一点亮真 PPUMASK 的地方）/ WriteScroll / ReadJoyPads`。

**证据强度最高**：L4 112 格 **110 格 0px**，剩 2 格（`f700=23px`/`f1100=42px`）已定谳为
**FC 相位差**（emu f700 ≡ rust f698 逐像素相同），**不是渲染错**。

---

## 6. 抢救清单（按「玩家必碰 × 后果严重度」排序）

| # | 断点 | 严重度 | 后果 | 修法 | 规模 |
|---|---|---|---|---|---|
| **1** | 死亡 → GameOver（`mr=4` 缺） | ★★★ | 死亡后 **panic** | 端口 `PrepareGameOver $C9A6`（4 行）+ `GameOver $939E`（`ClearAll`+`PreparePPUProcess($93B9)`+`NMIOn`+`ScreenOn`，后者已烘） | 小 |
| **2** | 游戏内暂停（`GoMainRoutine` START 开关 + `mr=5`） | ★★★ | 按 START **静默无效** | 端口 3↔5 切换 + `GamePaused ^= 1` + `PauseMusic`；再端口 `PauseMode $C9B1` | 小-中 |
| **3** | 电梯 / 区域间移动 | ★★★ | 只有 2 个区可达，且**不能换区** | `UpdateElevator $D7B3` + `SpawnElevator $EEA1` + `InitBank3/4/5` | 大 |
| **4** | 后三区数据（Kraid/Tourian/Ridley） | ★★★ | 3/5 世界不可进 | 参数化 `bake_area_graphics.py` 烘 3 区 + 敌四表分区（**先解 16B vs 64B 尺寸疑点**） | 中 |
| **5** | 密码屏返回（`mr=6` 缺） | ★★ | panic（但逻辑已在 `ui/password.rs`） | 只加一个 `mr==6` 臂 | 极小 |
| **6** | 出门链（`MoveOutDoor $D3AE` 等 4 桩） | ★★ | 走门 panic | 逐件端口 | 中 |
| **7** | 声音内容（24/28 零触达） | ★ | 音效/音乐行为无证据 | ①补 `B7E7`/`B87F` 两件高频 ②建音频 RAM 语义列（`.sem` 扩 `$0600-$068D`） | 中 |
| **8** | FC +2 相位 | ★ | L4 两格 23/42px + sem password 案 45.6% | 周期会计（已裁定冻结，待裁） | 大 |

**优先级建议**：#5（极小，先清一个 panic）→ #1（死亡链，核心循环）→ #2（暂停，最常用）
→ #6（出门链，与 #3 共用机制）→ #3+#4（换区+后三区，最大块）→ #7 → #8。

---

## 7. 证据体系能证什么、不能证什么

| 证据 | 覆盖 | 缺口 |
|---|---|---|
| L4 像素（112 格） | 渲染面（110/112 0px） | 声音、状态数值、未走过的路 |
| sem 语义（16300 帧 ×15 列） | 主流程轴 + Samus 坐标/状态 + 房间号 | **音频 RAM、敌人槽细节、道具/电梯、动画帧索引** |
| 单测（547） | 局部规则 | 端到端 |

**本次审计的最大结构性发现**：§6 的 8 项里 **7 项都没有任何自动化证据**会在它们坏掉时报警——
`unported!` 只在**被执行到**时才炸，而当前 14 个用例**从不执行**它们（这正是它们还活着的原因）。
「绿灯」与「完备」之间隔着的正是这张清单。

---

## 8. 取证缺口（未做，明确登记）

- **敌四表尺寸**：asm 三段各 16B vs Rust `[u8;64]` —— 分区前必须先对证（否则 3 区敌属性全错）。
- **`isHit` 位组合语义**：6 种组合（`$20/$30/+$08/$44/+$01/+$02`）只做了位级使用，未逐条对证。
- **电梯机制全形**：`UpdateElevator $D7B3` / `SpawnElevator $EEA1` 未逐行读（仅知是空桩）。
- **`MainRoutine` 7（`IncrementRoutine`）真实可达性**：`MoreInit` 尾是内联 `inc`，表项 7 是否永不可达未定谳。
- **`BankInitTable` ExitSub×3** 的语义（表尾占位还是真出口）未读。
