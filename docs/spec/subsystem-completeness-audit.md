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
| 1 主流程状态机 | 表齐（10 / 31 / 9 项） | `MainRoutine` **7/10** 臂（#1 死后已补 4）；`TitleRoutine` **31/31**；`BankInitTable` **3/9** | **3 处断点**（#5 经查为死代码） |
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

`crates/game-core/src/frame/mainloop.rs` 的 `match mr` 原覆盖 `0/1/2/3/8/9`，
其余落入 `_ => unported!("bank07:{表目标地址}")` —— **panic，不是空实现**。
2026-11-13 已补 `mr=4`（见下方 #1 处标注），现存 `4 缺臂` 中的 5/6/7。

| 缺臂 | asm | 触发条件 | 后果 |
|---|---|---|---|
| **4 `PrepareGameOver $C9A6`** ✅ **已修（2026-11-13）** | `TitleRoutine←GameOver` / `BankSwitchPending←1` / `jmp ScreenOff` | `game_engine` 已正确置 `NextRoutine←4` + `WaitTimer(9)` 会消费它 | ~~死亡后 panic~~ → 已端口 `prepare_game_over`，链至 `GameMode←1` 由 `InitBank0` 接管（TDD `tests_prepare_game_over` 两测） |
| **5 `PauseMode $C9B1`** | 要求 `Joy2Status & (A\|UP)`、`EndTimer+1==0`、`GamePaused←0`、`inc MainRoutine`→6 | 游戏内按 START | 更上游就断了：`GoMainRoutine` 的 START 开关（`MainRoutine` 3↔5 + `GamePaused ^= 1` + `PauseMusic`）**完全未实现**（`game_main_routine` 头部无任何 `Joy1Change & START`），所以**按 START 静默无效** |
| **6 `GoPassword $C9C4`** ✅ **已修（2026-11-13）** | `TitleRoutine←DisplayPassword($19)` / `BankSwitchPending←1` / `SilenceMusic` / `jmp ScreenOff` | **`PauseMode $C9B1` 的尾 `inc MainRoutine`（5→6）**是唯一产生点 | ~~不可达~~ → 已端口 `go_password`（见下方二次更正） |
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
| **1** | 死亡 → GameOver（`mr=4` 缺） ✅ **已修** | ★★★ | ~~死亡后 panic~~ → 已端口 | `prepare_game_over`（三条写）+ 复用已有 `TitleRoutine 0x1C` 臂（`PrepareVram(0x93B9)`/`Timer3←$10`/相位→0x19 均已实现） | 小（已完成） |
| **2** | 游戏内暂停（`GoMainRoutine` START 开关 + `mr=5`） ✅ **已修** | ★★★ | ~~按 START 静默无效~~ → 已端口 | `start_pause_toggle`（3↔5 + `GamePaused ^= 1` + `PauseMusic`）+ `pause_mode`（2P A+UP 退出）+ `go_password`（mr=6） | 小-中（已完成） |
| **3** | 电梯 / 区域间移动 | ★★★ | 只有 2 个区可达，且**不能换区** | `UpdateElevator $D7B3` + `SpawnElevator $EEA1` + `InitBank3/4/5` | 大 |
| **4** | 后三区数据（Kraid/Tourian/Ridley） | ★★★ | 3/5 世界不可进 | 在 `crates/game-data/src/extract.rs` 增 3 区段（区序见 `AREA_ITEMS`/`AREA_SPEC`）+ 敌表分区（**先解 16B vs 64B 尺寸疑点**，见 `docs/findings/enemy-table-extent.md`） | 中 |
| **5** | 暂停→密码屏（`mr=5 PauseMode` + `mr=6 GoPassword` 缺） ✅ **已修（并入 #2）** | ★★ | ~~panic~~ → 已端口 `pause_mode`/`go_password` | `PauseMode` 需 2P 的 A+UP 同按 + 逃脱计时器未激活；成功则 `GamePaused←0` + `inc MainRoutine`(5→6) | 小（已完成） |
| **6** | 出门链（`MoveOutDoor $D3AE` 等 4 桩） ✅ **已实现 / 待差分验证** | ★★ | ~~走门 panic~~ → 已端口；**但无 L4 采集点走到该链**（见下行注） | `samus_door` 全链（D3A8 逐字节）+ `Door_DeleteOffscreenEnemies`（D48C，**保真两处 asm 自带 BUG**：敌环 `#$60`/`#$20` 而非 `#$50`/`#$10`；管道虫洞环 `bne` 致槽 0 不访问）+ `Doors_RemoveIfOffScreen`（ED65，4 门 $B0..$80）+ `GotoClearAllMetroidLatches`（$95AB 区 bank 桩表：仅 bank3 Tourian 有效，其余 `60 EA EA`）+ 音乐面（`ItemRoomMusicStatus`/`KraidRidleyPresent`→TourianMusic） | 中（已完成） |
| **7** | 声音内容（24/28 零触达） | ★ | 音效/音乐行为无证据 | ①补 `B7E7`/`B87F` 两件高频 ②建音频 RAM 语义列（`.sem` 扩 `$0600-$068D`） | 中 |
| **8** | FC +2 相位 | ★ | L4 两格 23/42px + sem password 案 45.6% | 周期会计（已裁定冻结，待裁） | 大 |

**优先级建议（2026-11-13 二次修订）**：#1（死亡链）✅ → #2 + #5（暂停→密码屏，同族三件）✅
→ #6（出门链，与 #3 共用机制）✅ → #3+#4（换区+后三区，最大块）→ #7 → #8。

> 【2026-11-14 更正】#6 的「已修」只到**实现层**：r9 对抗复核（verifier）查出
> `Doors_RemoveIfOffScreen` 曾把 `$0300,x`（绝对址 $0380..$03B0）写成零页偏移 `0x80`，
> 且循环用 `while x >= 0` 跑 12 轮（asm `bmi` 回跳 ⇒ 恰 4 轮）——两处已修并加零页哨兵断言。
> 更关键的是**覆盖**：现有 14 案 112 格里没有任何采集点会让 `SamusDoor` 被调用
> （唯一门案 `case-002-door-blue-interaction` 全程 `DoorEntryStatus=0`、`Samus.status` 从未出现 5），
> 故该链目前**只有单测证据、零对拍证据**。结项前须补一条门退出窗口内的采样点。

> 【2026-11-13 更正】上表 #5「密码屏返回 panic」的初判**不成立**：`_id_GoPassword` 全仓只出现在
> `WaitTimer` 的比较里（`refs/m1disasm/SRC/prg7_engine.asm:1229`），而 `stx NextRoutine` 唯一
> 出现在 `SetTimer`（:1245），其两个调用点分别是 `ldx #$03`（能力延迟，:2052）与 `ldx #$04`
> （死亡，:2074）——**没有任何路径把 `NextRoutine` 置为 6** ⇒ `GoPassword $C9C4` 结构性不可达
> （与 `AreaInit $C801` 同类）。原推论「`WaitTimer` 已能置 6」是把「能比较」误当「会写入」，
> 特此原地更正；端口它等于写死代码，保留 `unported!` fail-loud 桩符合 D7。
>
> 【2026-11-13 **二次更正**（推翻上面那条更正）】「`GoPassword` 结构性不可达」**仍然错了**。
> 我只查了 `NextRoutine` 的写点，漏了 `mr` 的另一条产生路径：`PauseMode $C9B1`
> （`prg7_engine.asm:2160-2164`）在「2P 的 A+UP 同按 ∧ `EndTimer+1==0`」满足时执行
> `sta GamePaused`（A=0）+ **`inc MainRoutine`（5→6 = GoPassword）**。
> 所以 mr=6 **可达**，路径是「游戏中 START 暂停 → 暂停画面上 2P 按 A+UP → 显示密码屏」。
> 第一次更正的错误在于把「`NextRoutine` 能否为 6」（确为死分支）当成了「`mr` 能否为 6」。
> 教训：查「某态能否到达」必须查**该状态变量的全部写点**（此处 `mr` 的写点有 `SetMainRoutine`/
> `SetTimer`/各例程的 `inc` 三类），不能只看其中一族。
>
> 因此 #2 的实际范围比原估大：START 开关（3↔5）+ `PauseMode`（5→6）+ `GoPassword`（6→切页），
> 三件同族一并端口（2026-11-13 完成）。
>
> 同一轮取证还澄清了**密码屏的真正触发链**（这才是玩家碰到的路径）：
> 死亡 → `GameEngine` 检 `sa_Dead2 ∧ animDelay==0` → `SilenceMusic` →
> `MotherBrainStatus==$0A ? inc MainRoutine : SetTimer(#$04, x=4)`（两条死亡径最终都落 mr=4）
> → `PrepareGameOver`（`TitleRoutine←0x1C` + `$24←1` + `ScreenOff`）→ `InitBank0`（`GameMode←1`）
> → `0x1C GameOver $939E`（GAME OVER 2.6s）→ `0x19 DisplayPassword`（`CalculatePassword` +
> `PasswordToScreen` + `PaletteDataPending←Palette12+1`）→ `0x1A WaitForSTART`
> → `jmp CheckPassword`（与密码屏输入同一入口）→ 有效则 `InitializeGame` 重开。

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
