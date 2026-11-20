# Tourian 终局全链规格书（区域 3：Metroid / Rinka / Zebetite / MotherBrain / 自毁 / 结局）

> 状态：**规格定稿（asm-first，只读调研产物）** — 2026-11-20。
> 用途：给后续移植卡当实现依据（行为契约 + 数据面清单 + 缺口卡 + 无像素证据下的验收判据）。
> 铁律：asm 是唯一真相源；本文每条结论带 `bank:addr` + asm 行号；A 级（已证实）与 C 级（假设）
> 分节登记（见 §10、§11）——C 级不得据此改名或写死常量。
> 复现口径：`refs/m1disasm/SRC/prg3_tourian.asm`（2430 行）+ `refs/m1disasm/SRC/enemies/metroid.asm`（378 行）
> + `refs/m1disasm/SRC/enemies/rinka.asm`（222 行）+ 被引引擎件 `refs/m1disasm/SRC/prg7_engine.asm`（13169 行）
> 与 `refs/m1disasm/SRC/prg0_title.asm`（结局屏）。

## 1. 范围、证据约定与判据

### 1.1 本文覆盖的链（从雕像桥到结局屏）

| # | 环节 | 产物 | 章节 |
|---|---|---|---|
| 1 | 入口：Kraid/Ridley 像抬起 → 开桥 → 进 Tourian | 门判/桥对象 | §4.4 |
| 2 | Tourian 走廊与竖井：Rinka 生成器、Metroid、门炮 | 敌表/生成源 | §3、§4.1 |
| 3 | 5 座 Zebetite（能量屏障）→ 玻璃 → MotherBrain | Boss 链 | §4.2、§4.3、§4.5 |
| 4 | MotherBrain 死亡 → `TIME BOMB SET GET OUT FAST!` → 自毁计时 | 结局触发链 | §4.3、§6.1 |
| 5 | 逃逸（原路上行）→ 结局电梯 `$8F` | 计时/逃逸 | §6.2 |
| 6 | 结局屏（Samus 无装甲出场 → 闪 → 挥手 → 淡出 → Staff → 重开） | TitleRoutine 状态机 | §6.3 |
| 7 | 失败：计时归零 → 全白 → 死亡 → `GameOver`（密码屏） | 死亡链 | §6.1、§6.5 |

Tourian **没有**普通可破坏方块（§5.4 双返回），破坏只发生在 Zebetite、玻璃、`TIME BOMB` 字形、门瓦四处——
这是本区最反直觉的结构，也是移植时最容易写成"通用爆瓦"的地方。

### 1.2 证据记号与读法

| 记号 | 含义 | 例 |
|---|---|---|
| `03:XXXX` | CPU 地址（bank3 = Tourian 区 bank，运行时映射 `$8000`） | `03:9E43` |
| `03:XXXX / :NNNN` | 同上一节文件的**汇编行号**（省略文件名= `prg3_tourian.asm`） | `03:9E43 / :1166` |
| `07:XXXX / prg7:N` | 引擎件（bank7 固定段）地址 + `prg7_engine.asm` 行号 | `07:CE92 / prg7:3203` |
| `00:XXXX / prg0:N` | 标题/结局屏（bank0） | `00:9AA7 / prg0:3712` |
| 数据表 ROM 字节 | `prg 偏移 = bank*0x4000 + (addr-0x8000)`，ROM 文件去掉前 16 B iNES 头 | bank3 `$960B` → `0xC000+0x160B=0xD60B` |

两条换算口径（全文一致，勿混）：

- **区 bank 号**：Brinstar=0、Norfair=1、Tourian=**3**、Kraid=4、Ridley=5（`BankTable` 值 = bank+1，见 `crates/game-core/src/samus/handler.rs:1473` 实测桩表）。
- **`roomRAMPtr` 是"tile 指针"不是坐标**：命中判据直接比较指针（`Temp04_RoomRAMPtr`），因此移植必须保留
  "从当前 tile 指针做掩码比较"的写法，不能改写成"格坐标相等"（§10 陷阱二）。

### 1.3 A 级 / C 级判据（与 AGENTS.md §3 一致）

| 级 | 本文用法 |
|---|---|
| **A** | 逐行读过并给出地址+行号；或数据表给出全部字节；或给出可复现命令 |
| **C** | 需要靠房间数据反解才能定的几何/身份问题，单独登记于 §10，**不得写进实现** |

本文所有"确定值"（32 击、5 串死亡串、9999 帧、512 帧冻结、40 帧指令延迟…）都是 A 级常量，
来源见对应表格的证据列；C 级只有 12 条（§10），全部是"语义身份/几何"而非常量。

### 1.4 Rust 现状判据（ported / candidate / unported）

现状唯一机械来源：[data/status/routine_inventory.json](../../data/status/routine_inventory.json)（顶层 dict，
`entries` 为 26 组；三组相关：`prg3_tourian` 60 条、`metroid` 11 条、`rinka` 4 条），
并以 `grep -rn` 在 `crates/game-core/src/` 交叉确认。

| 状态 | 判据（三条同时成立才算） |
|---|---|
| `ported` | 清单标 ported；Rust 有同名语义函数；**该函数在 tourian 区可达路径上被调用** |
| `candidate` | 清单标 candidate；Rust 有同义实现，但只在别的区路径（或只做桩分派）可达 |
| `unported` | 清单标 unported；且 `grep` 只命中注释/unported! 桩/`DIVERGENCES` 记账 |

复核命令（任一条都可独立复现清单行号漂移）：

```bash
python3 -c "import json;d=json.load(open('data/status/routine_inventory.json'))['entries'];print({k:len(v) for k,v in d.items()})"
grep -rn "tourian\|Tourian" crates/game-core/src/enemy/ai_table.rs | head
```

### 1.5 参与本链的源码文件清单（A 级：行数+角色）

| 文件 | 行数 | bank / 段 | 角色 |
|---|---|---|---|
| [refs/m1disasm/SRC/prg3_tourian.asm](../../refs/m1disasm/SRC/prg3_tourian.asm) | 2430 | bank3 `$8000` | 区 bank：敌表、Boss、炮台、计时、结局触发；含 GFX/Palette/区指针表 |
| [refs/m1disasm/SRC/enemies/metroid.asm](../../refs/m1disasm/SRC/enemies/metroid.asm) | 378 | bank3 `$97F9`（include） | Metroid AI：吸附/吸血/冻结/脱离 |
| [refs/m1disasm/SRC/enemies/rinka.asm](../../refs/m1disasm/SRC/enemies/rinka.asm) | 222 | bank3 `$9A2C`（include） | Rinka AI：一次性定速 8.8 定点 |
| [refs/m1disasm/SRC/prg7_engine.asm](../../refs/m1disasm/SRC/prg7_engine.asm) | 13169 | bank7 固定 `$C000` | 伤害/冻结/碰撞/特殊项扫描/生成派发/结局电梯 |
| [refs/m1disasm/SRC/prg0_title.asm](../../refs/m1disasm/SRC/prg0_title.asm) | 4000+ | bank0 | 结局屏 7 态、密码屏、GameOver |
| `refs/m1disasm/SRC/tourian/*.asm` | 见 §7.1 | bank3 `$A406`–`$AF49` | 房流/结构/元件/调色板/精灵/属性表（9 个 include） |

## 2. 例程清单表（label → `03:$XXXX` → 行号 → 行数 → Rust 状态）

行数列 = 到下一个 label 的行差（含局部标签区间，仅作规模参考）。`—` = 无 Rust 锚。

### 2.1 区 bank 例程（`prg3_tourian.asm`，60 条）

| label | addr | asm 行 | 行数 | Rust | Rust 锚 |
|---|---|---|---|---|---|
| `GotoClearCurrentMetroidLatchAndMetroidOnSamus` | `03:95A8` | 97 | 2 | unported | — |
| `GotoClearAllMetroidLatches` | `03:95AB` | 99 | 2 | candidate | crates/game-core/src/samus/handler.rs:1289 |
| `GotoDeleteOffscreenRoomSprites_Tourian` | `03:95AE` | 101 | 2 | unported | — |
| `GotoSpawnCannonRoutine` | `03:95B1` | 103 | 2 | unported | — |
| `GotoSpawnMotherBrainRoutine` | `03:95B4` | 105 | 2 | unported | — |
| `GotoSpawnZebetiteRoutine` | `03:95B7` | 107 | 2 | unported | — |
| `GotoSpawnRinkaSpawnerRoutine` | `03:95BA` | 109 | 2 | unported | — |
| `GotoUpdateBullet_CollisionWithZebetiteAndMotherBrainGlass` | `03:95BD` | 111 | 2 | unported | — |
| `GotoUpdateBullet_CollisionWithMotherBrain` | `03:95C0` | 113 | 3 | unported | — |
| `AreaRoutine` | `03:95C3` | 116 | 4 | unported | — |
| `TwosComplement_` | `03:95C6` | 120 | 4 | unported | — |
| `Exit__` | `03:95CB` | 124 | 3 | unported | — |
| `ChooseEnemyAIRoutine` | `03:95E5` | 163 | 21 | **ported** | crates/game-core/src/enemy/ai_table.rs:1 |
| `RemoveEnemy_` | `03:97DC` | 530 | 28 | unported | — |
| `UpdateEnemyCommon_Decide` | `03:97E2` | 535 | 23 | candidate | crates/game-core/src/enemy/framework.rs:426 |
| `AreaRoutine_Tourian` | `03:9B25` | 568 | 9 | unported | — |
| `UpdateAllCannons` | `03:9B37` | 577 | 10 | unported | — |
| `UpdateCannon` | `03:9B4C` | 595 | 25 | unported | — |
| `UpdateCannon_RunInstructions` | `03:9B70` | 620 | 14 | unported | — |
| `Cannon_ShootEnProjectile` | `03:9BAF` | 666 | 21 | unported | — |
| `DrawCannon_Normal` | `03:9C2B` | 750 | 3 | unported | — |
| `DrawCannon_Escape` | `03:9C31` | 753 | 13 | unported | — |
| `UpdateCannon_CheckIfOnScreen` | `03:9C4D` | 766 | 26 | unported | — |
| `DeleteOffscreenRoomSprites_Tourian` | `03:9C6F` | 799 | 89 | unported | — |
| `SpawnCannonRoutine` | `03:9CE6` | 907 | 13 | unported | — |
| `SpawnMotherBrainRoutine` | `03:9D21` | 950 | 19 | unported | — |
| `SpawnZebetiteRoutine` | `03:9D3D` | 977 | 24 | unported | — |
| `GetRoomRAMPtrHi` | `03:9D64` | 1001 | 10 | unported | — |
| `SpawnRinkaSpawnerRoutine` | `03:9D6C` | 1011 | 22 | unported | — |
| `GetNameTableAtScrollDir_` | `03:9D88` | 1036 | 6 | unported | — |
| `MotherBrainStatusHandler` | `03:9DD4` | 1091 | 15 | unported | — |
| `MotherBrain_Idle_CollideWithSamus` | `03:9DF2` | 1110 | 32 | unported | — |
| `MotherBrain_Idle` | `03:9E22` | 1142 | 5 | unported | — |
| `ClearMotherBrainIsHit` | `03:9E31` | 1149 | 6 | unported | — |
| `MotherBrain_Hurt` | `03:9E36` | 1155 | 8 | unported | — |
| `UpdateMotherBrainFlashDelay` | `03:9E43` | 1166 | 24 | unported | — |
| `MotherBrain_Killed` | `03:9E52` | 1190 | 40 | unported | — |
| `MotherBrain_Disappear` | `03:9E86` | 1230 | 65 | unported | — |
| `SpawnRinka_InitPositionXY` | `03:9EE7` | 1314 | 13 | unported | — |
| `Xplus16` | `03:9EF9` | 1327 | 10 | unported | — |
| `MotherBrain_TimeBombMessage` | `03:9F02` | 1341 | 1 | unported | — |
| `MotherBrain_TimeBombMessage_ScrollBackOnScreen` | `03:9F02` | 1342 | 45 | unported | — |
| `MotherBrain_SetTimeBomb` | `03:9F49` | 1412 | 17 | unported | — |
| `MotherBrain_SpawnDoor` | `03:9F69` | 1438 | 50 | unported | — |
| `MotherBrain_TimeBombExploded` | `03:9FC0` | 1488 | 18 | unported | — |
| `MotherBrain_SetTimeBomb_ScrollBackOnScreen` | `03:9FDA` | 1510 | 14 | unported | — |
| `MotherBrain_Idle_HandleBeingHit` | `03:9FED` | 1528 | 35 | unported | — |
| `MotherBrain_Idle_UpdateAnimBrain` | `03:A01B` | 1567 | 17 | unported | — |
| `MotherBrain_Idle_UpdateAnimEye` | `03:A02E` | 1588 | 16 | unported | — |
| `MotherBrain_DrawSprites` | `03:A041` | 1608 | 30 | unported | — |
| `MotherBrain_Disappear_Disintegrate` | `03:A072` | 1648 | 17 | unported | — |
| `UpdateBullet_CollisionWithZebetiteAndMotherBrainGlass` | `03:A0C6` | 1732 | 94 | unported | — |
| `UpdateBullet_CollisionWithMotherBrain` | `03:A142` | 1834 | 21 | unported | — |
| `UpdateAllRinkaSpawners` | `03:A15E` | 1862 | 12 | unported | — |
| `UpdateEndTimer` | `03:A1E7` | 1963 | 53 | unported | — |
| `DrawEndTimerEnemy` | `03:A238` | 2021 | 51 | unported | — |
| `UpdateAllZebetites` | `03:A28B` | 2076 | 15 | unported | — |
| `UpdateZebetite` | `03:A29B` | 2091 | 29 | unported | — |
| `ClearAllMetroidLatches` | `03:A315` | 2193 | 9 | unported | — |
| `ClearCurrentMetroidLatchAndMetroidOnSamus` | `03:A320` | 2202 | 165 | unported | — |

表中三条 Rust 状态的实测差异（**判据现场**，供后续卡核对）：

1. `ChooseEnemyAIRoutine` 是 ported 但**只 port 了派发表**：`crates/game-core/src/enemy/ai_table.rs` 把
   `routine_for(3,0)`/`(3,4)` 返回 `Unported("MetroidAIRoutine")`/`Unported("RinkaAIRoutine")`，
   即"表对了、目标不对"——不能算 metroid/rinka AI 已 port。
2. `GotoClearAllMetroidLatches` 的清单 Rust 锚写 `handler.rs:1289`，实际函数在
   `crates/game-core/src/samus/handler.rs:1481`（`goto_clear_all_metroid_latches`，含区 bank 桩分派
   `03:95AB` 实测字节）；清单行号已漂移（记账缺口，见 G-9）。
3. `UpdateEnemyCommon_Decide` candidate 的锚 `enemy/framework.rs:426` 实为 `UpdateEnemy_Hurt` 区段，
   共用的 `UpdateEnemyCommon` 在引擎件侧，属"半 port"。

### 2.2 敌人 AI 例程（`metroid.asm` 11 条 + `rinka.asm` 4 条，全 unported）

| label | addr | asm 行 | 行数 | Rust | 备注 |
|---|---|---|---|---|---|
| `MetroidAIRoutine` | `03:97F9` | 1 | 118 | unported | 红/绿共用，靠 `movementIndex` 分表 |
| `ClearCurrentMetroidLatch` | `03:99AE` | 301 | 2 | unported | 单槽清 |
| `ClearMetroidLatch` | `03:99B1` | 303 | 5 | unported | `y` 为 0..5 的 latch 槽 |
| `LoadEnemySlotIDIntoY` | `03:99B7` | 308 | 6 | unported | `slot/16` |
| `LoadEnemySlotIDIntoX` | `03:99BD` | 314 | 6 | unported | 同上，写 X |
| `ClearMetroidSpeed` | `03:99C3` | 320 | 6 | unported | 同时用于 Rinka |
| `ClearRinkaAcceleration` | `03:99D1` | 326 | 6 | unported | 与上者同体 |
| `StoreSamusPositionToTemp` | `03:99E4` | 335 | 10 | unported | 吸血定位用 |
| `LoadEnemyPositionFromTemp_` | `03:99F4` | 345 | 11 | unported | `hi&1` 折回 |
| `GetMetroidAccel` | `03:9A06` | 356 | 12 | unported | 表 `$77F2` |
| `GetMetroidRepelSpeed` | `03:9A10` | 368 | 11 | unported | `$F8`/`$08` 二值 |
| `RinkaAIRoutine` | `03:9A2C` | 1 | 133 | unported | 只在 Active+prevResting 定速一次 |
| `SetRinkaSpeed` | `03:9AF9` | 183 | 26 | unported | 8.8 定点（§3.4） |
| `Adiv16_` | `03:9B1B` | 209 | 7 | unported | 右移 4 |
| `Amul16_` | `03:9B20` | 216 | 7 | unported | 左移 4（`SpawnRinka_InitPositionXY` 用） |

### 2.3 跨 bank 被引件（本链实际消费的引擎/结局例程）

| label | addr | asm 行 | 角色（本链内） |
|---|---|---|---|
| `InitBank3` | `07:C590` | prg7:1449 | 把 14 B `TourianMetroidData` 抄进 `$77F0`（§3.3） |
| `SubtractHealth` | `07:CE92` | prg7:3203 | 全部伤害入口（4 道守卫，§5.3） |
| `UpdateBullet_CollisionWithBG` | `07:D609` | prg7:4506 | 弹对 BG：先 MB 钩再爆瓦判定 |
| `CheckBlastTile` | `07:D651` | prg7:4554 | 可爆瓦区段（Brinstar `$80-$97`，其余 `$70-$97`） |
| `IsBlastTile` | `07:E9BE` | prg7:8595 | 非 Tourian 通用爆瓦；Tourian 走双返回（§5.4） |
| `ScanForItems` | `07:ED98` | prg7:9428 | 按 `MapPosY/X` 扫特殊项 |
| `ChooseSpawningRoutine` | `07:EDD6` | prg7:9495 | `type&$0F` → 10 路跳表（§4.1） |
| `SpawnMotherBrain` | `07:EEAE` | prg7:9682 | 已死重进 → `status=8`/`counter=0` |
| `SpawnZebetite` | `07:EECA` | prg7:9702 | 已死重进 → `status=$81`/`isHit=1`/`qtyHits=7` |
| `SpawnRinkaSpawner` | `07:EEEE` | prg7:9722 | 生成源派发 |
| `UpdateAllEnProjectiles` | `07:F93B` | prg7:11878 | 炮台弹的 Active 移动（`data0A&$FE` 字节偏移） |
| `EnemyReactToSamusProjectile` | `07:F536` | prg7:11038 | 冰/导弹/无敌分派（§5.3） |
| `FreezeEnemy` | `07:F518` | prg7:11010 | `data0D=$40`（512 帧） |
| `UpdateEnemy_Hurt` | `07:F4EE` | prg7:10975 | 受击硬直结束回 prevStatus（Metroid 保持冻结） |
| `UpdateEnemy_Frozen` | `07:F43E` | prg7:10840 | 每 8 帧减 `data0D`，到 0 解冻 |
| `UpdateStatue_StartRaising` | `07:DA7C` | prg7:5341 | Kraid/Ridley 像抬起 |
| `UpdateAllStatues_Bridge` | `07:DADA` | prg7:5416 | 双 bit7 → 桥（§4.4） |
| `ElevatorD8BF` | `07:D8BF` | prg7:4980 | `$8F` 结局电梯（§6.2） |
| 死亡链（函数体在 `Samus_Dead2` 后段） | `07:C95F` | prg7:2055 | `sa_Dead2`→`GameOver`/`inc MainRoutine` |
| `UpdateAge` | `07:C97E` | prg7:2088 | 每 256 帧记年龄（结局等级用） |
| `ChooseEnding` | `07:CAF5` | prg7:2439 | 年龄表 → `EndingType 1..5` |
| `EndGame` | `00:9AA7` | prg0:3712 | 结局屏 7 态（§6.3） |
| `LoadEndGFX`…`RollCredits` | `00:9AD5`–`00:9BFC` | prg0:3742–3972 | 7 态各自的处理器 |

### 2.4 覆盖率快照（三组分母固定）

| 组 | 条目 | ported | candidate | unported | 覆盖率 |
|---|---|---|---|---|---|
| `prg3_tourian` | 60 | 1 | 2 | 57 | 1.7 % |
| `metroid` | 11 | 0 | 0 | 11 | 0 % |
| `rinka` | 4 | 0 | 0 | 4 | 0 % |
| 合计 | 75 | 1 | 2 | 72 | 1.3 % |

结论要写成一句能被门禁读的话：**Tourian 整条链除"AI 派发表"外，代码面 0 实装**；
Rust 侧现有的 `RinkaSpawners←$FF`（`crates/game-core/src/frame/tick.rs:510`）只是 `SamusInit` 的清零动作，
不是生成器本身。

## 3. Tourian 敌表（16 项）

### 3.1 派发表（`ChooseEnemyAIRoutine`，A 级）

`03:95E5 / :163`：`lda EnsExtra.0.type,x` → `CommonJump_JumpEngine`，16 个 `.word`：

| type | 例程 | 语义（读指令得出） |
|---|---|---|
| `$00` | `MetroidAIRoutine` | Metroid（红），`movementIndex=0`（`EnemyMovementChoiceOffset[0]`） |
| `$01` | `MetroidAIRoutine` | Metroid（绿），`movementIndex=1` |
| `$02` | `L9A27`（`03:9A27 / :558`） | `lda #$01` → `UpdateEnemyCommon_noMove`：原地不动、anims 走通用；血 1；表注释「i dunno but it takes 30 damage with varia」 |
| `$03` | `RemoveEnemy` | 一出现即删（死亡动画 `EnAnim_18`，表注释「disappears」） |
| `$04` | `RinkaAIRoutine` | Rinka（生成器产物），速度自算 |
| `$05`–`$0F` | `RemoveEnemy` | 未使用槽（表注释「same as 3」） |

**"每区自己的词表"是硬约束**：`type` 号只在区 bank 内有效。引擎侧的 `ChooseEnemyAIRoutine` 地址每区都在
`$95E5`，但派发目标不同（`crates/game-core/src/enemy/ai_table.rs:11` 的注释表已按 area 分行）；Rust 现状
`routine_for(3,0)`/`(3,4)` 仍返回 `Unported`，所以"能选对表"不等于"能跑对 AI"。

### 3.2 每 type 的属性（9 张表合并，A 级；全部 16 项字节见 `:184-412`）

| type | 血 | 死亡动画 | Resting | Active | InitOff | `L968B` | ForceDelay | InitDelay | `L977B` | 语义 |
|---|---|---|---|---|---|---|---|---|---|---|
| `$00` | `$FF` | MetroidExplode | Metroid | Metroid | 0 | `%11111110` | 1 | 1 | `%01010000` | 红 Metroid |
| `$01` | `$FF` | MetroidExplode | Metroid | Metroid | 0 | `%11111110` | 1 | 1 | `%01010000` | 绿 Metroid |
| `$02` | `$01` | `EnAnim_16` | `EnAnim_16` | `EnAnim_16` | 0 | 0 | 0 | 0 | 0 | 不动靶（`L9A27`） |
| `$03` | `$FF` | `EnAnim_18` | `EnAnim_18` | `EnAnim_18` | 0 | 0 | 0 | 0 | 0 | 立即消失 |
| `$04` | `$01` | RinkaExplode | RinkaSpawning | Rinka | 2 | `%11000000` | 1 | 1 | 0 | Rinka |
| `$05`–`$0F` | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 未使用 |

`EnemyDistanceToSamusThreshold` 全 16 项为 0（`:310`）：**本区没有任何"靠近才激活"的阈值**。
`L977B`（`:395`）bit7 置位才启用"加速+初速"系统，本区只有 `$00/$01` 置位；
`L968B`（`:274`）`%11111110` 的位语义由引擎侧消费（`crates/game-core/src/enemy/framework.rs:426`
已按 bit4 复现 Metroid 冻结保持，见 §5.3），bit0 为 0 是 16 项里唯一"红/绿 Metroid"共有的形态。

运动初值（`03:972B`/`973F`/`9753`/`9767`，`:386-393`）——`movementIndex` 为索引：

| idx | AccelY | AccelX | SpeedY | SpeedX | 归属 |
|---|---|---|---|---|---|
| 0 | `$18` | `$18` | `$00` | `$00` | Metroid 红 |
| 1 | `$30` | `$30` | `$00` | `$00` | Metroid 绿 |
| 3 | `$C0` | `$00` | `$04` | `$02` | 未用（表内其它敌） |
| 4 | `$D0` | `$00` | `$02` | `$02` | 未用 |
| 7/8 | `$7F`/`$80` | 0 | 0 | `$02` | 未用 |
| 9/10 | `$58`/`$54` | 0 | `$0C`/`$FC` | `$02` | 未用 |

`EnemyMovementChoices`（`:458`）只有两条有效项：`MetroidRed` = `EnemyMovementChoiceEntry $00`（字节 `00 00`）、
`MetroidGreen` = `$01`（`00 01`）、`Rinka` = 空。**18 对 `EnemyMovement00..11_R/L` 全部落在 `$97D5`
（同一个"无移动"地址）**——本区敌人不靠移动脚本移动，Metroid 靠加速系统、Rinka 靠自算速度（§3.3/§3.4）。

### 3.3 Metroid：吸附 / 吸血 / 冻结 / 脱离（A 级，逐行）

前置（`03:97F9 / metroid.asm:1-20`）：`EndTimer+1; iny; beq` 若**计时已启动**（`EndTimer+1 != $FF`）→
`status=0` 自杀。⇒ **逃逸阶段房间里不再有 Metroid**（自毁计时一开始全场清除，见 §6.1）。

| 行为 | 触发 | 机制 | 证据 |
|---|---|---|---|
| 限速 | `latch==0` | `abs(speedY) > MetroidMaxSpeed[movementIndex]` → 钳位并清 subpixel；X 同理 | `:9822-9870`，表 `$77F6` |
| 加速 | `latch==0` | `GetMetroidAccel` 按 `data05` 符号位 + `movementIndex` 查 `$77F2` → 写 `accelX/accelY` | `:9A06`，Init 抄表 `07:C590` |
| 无敌 | `latch==0` 且非 Frozen | 每帧 `health=$FF`；Frozen 且 `health+1==0` → `health=5` | `:9850-9895` |
| 冻结（被动） | `EnemyStatusPreAI==Hurt` 且当前 `status!=Hurt` | `status=Frozen`（引擎侧 `FreezeEnemy` 已把 `data0D` 置 `$40`=512 帧，每 8 帧 −1） | `:9899`、`07:F518` |
| 吸附 | `latch==0` 且 `isHit&$04`（接触 Samus） | `latch = (speedX&$80) 或 $01`（记 x 速度符号 + 附着开始），并 `ClearMetroidSpeed` | `:9932-9945` |
| 附着定位 | 每帧 latch 有效 | 低 nibble 帧计数 `0..$0C`（`<$0C` 时 +1）；`Y=MetroidLatchOffsetY[count-1]`、`X=$0C-count`（按 latch bit7 取反）→ 位置 = Samus + 偏移 | `:9946-9963`，表 `$99D8` = `00,FC,F9,F7,F6,F6,F5,F5,F5,F6,F6,F8` |
| 吸血 | 六 latch OR 后 `&$0C==$0C`（至少一只完全附着）且 `Health!=0` | `HealthChange=$0004`（注释 1/4 格）+ `SubtractHealth` + `MetroidOnSamus=1` | `:9971-999E` |
| 炸弹脱离 | 已 latch 且 `isHit&$20`（武器命中） | 仅 `wa_Unknown7($07)`/`wa_BombExplode($0A)` 计数，且 `FrameCount bit1==0`；`latch+=$10`；`(latch&$70)==$50`（5 次）→ `prevStatus=Active 加 specialAttribs`、`status=Hurt`、`specialAttribs=$20`、`data0D=1`、清 latch、按 `accel` 符号取 `$F8/$08` 反弹速 | `:9899-9930`、`GetMetroidRepelSpeed 03:9A10` |

吸血速率的实测（python 复刻 6502 BCD 减法，非十六进制）：30 格能量 **73 帧**归零（≈2.4 帧/格），
10 格 23 帧，4 格 8 帧；每帧只有一次 `$0004` 减法（六只同时吸附也是每帧 1/4 格，不叠加）。
`MetroidOnSamus` 供 HUD 与音效使用（`ObjectCntrl` 未置位时写 `type|$82|优先级`）。

### 3.4 Rinka：生成源、上限、刷新条件、运动（A 级）

**谁 spawn**：`UpdateAllRinkaSpawners`（`03:A15E / :1862`）——每帧最多产生一只，条件是全部成立：

| 条件 | 值/式 | 证据 |
|---|---|---|
| 自毁计时未启动 | `EndTimer+1 == $FF`（`iny` 后为零才继续） | `:1863-1866` |
| 生成器存在 | `RinkaSpawners.status >= 0`（`$FF`=空） | `:1871` |
| 帧奇偶与 nametable 交替 | `(spawner.hi ^ FrameCount) & 1 == 1` | `:1875-1878` |
| MotherBrain 仍活 | `MotherBrainStatus < 4` | `:1880-1882` |
| 时限节流 | `FrameCount & $06 == 0`（即 `FrameCount mod 8 ∈ {0,1}`） | `:1884-1886` |

**槽上限**：只在前 3 个敌槽里找"空槽或不可见"（`x=$20,$10,$00`；`Ens.data05 bit1=0` 视为不可见），
三槽都占则本帧不生成（`:1887-1900`）。⇒ **同屏 Rinka 上限 = 3**（生成器 2 个、每屏交替出让）。
**生成动作**：`status=Resting`、`type=$04`、`specialAttribs=0`、`isHit=0`、`animFrame=FrameNone`（淡入），
`hi=spawner.hi`，位置 = `RinkaSpawnPosTbl[(posIndex<<1)|spawner.status]`（`03:A1DB / :1954`）：

| posIndex | 偶(status=0) | 奇(status=1) | 解码 (y,x) |
|---|---|---|---|
| 0 | `$22` | `$2A` | (y `$27`,x `$27`) / (`$27`,`$A7`) |
| 1 | `$2A` | `$BA` | (`$27`,`$A7`) / (`$B7`,`$A7`) |
| 2 | `$B2` | `$2A` | (`$B7`,`$27`) / (`$27`,`$A7`) |
| 3 | `$C4` | `$2A` | (`$C7`,`$47`) / (`$27`,`$A7`) |
| 4 | `$C8` | `$BA` | (`$C7`,`$87`) / (`$B7`,`$A7`) |
| 5 | `$BA` | `$BA` | (`$B7`,`$A7`) / 同 |

（坐标解码见 `SpawnRinka_InitPositionXY 03:9EE7 / :1314`：`y=(hi nibble*16)|7`、`x=(lo nibble*16)|7`；
`posIndex` 自增到 6 归零。）两个生成器共用一个 6 项表、两套奇偶列 ⇒ 12 个不重复出点。

**运动**：`RinkaAIRoutine`（`03:9A2C / rinka.asm:1`）只在 `status==Active && EnemyStatusPreAI==Resting`
这一帧定速一次：`distX=Samus.x-x`（按 `facing bit0` 取符号）、`distY=Samus.y-y`（按 `bit2`），
两距离一起右移到最高有效位落在 bit4（即按 `8/4/2/1` 归一），再由 `SetRinkaSpeed`（`03:9AF9 / :183`）
把 `dist` 拆成 `speed=dist>>4`、`subPixel=dist&$0F`（**8.8 定点：每帧走剩余距离的 1/16**，上限 `15.15`），
按 `facing` 取负。之后每帧 `subPixel+=speedSubPixel`、`speed+=carry` 推进；`ApplySpeedToPosition` 越界
→ `status=0` 删除；`data05 bit6` 置位时按 30 FPS 跳帧（每 2 帧动一次）；动画每 8 帧一换。

**为什么这很重要**：Rinka 是"一次性制导"——定速后**不再追踪 Samus**，所以玩家原地等它飞过去即可，
但它的速度完全取决于生成瞬间的 Samus 位置差。移植若按"每帧朝 Samus 飞"实现，行为会持续发散（§10 陷阱三）。

## 4. Boss 链

### 4.1 入口条件（三个 `Spawn*` 各自的触发点）

生成不是"进房就摆"，而是**逐格扫描特殊项表**后按 `type&$0F` 分派（A 级）：

```
ScanForItems 07:ED98 → ChooseSpawningRoutine 07:EDD6  (type & $0F)
  $01 敌 / $02 道具 / $03 Mellow / $04 电梯 / $05 炮台
  $06 MotherBrain / $07 Zebetite / $08 Rinka 生成器 / $09 门 / $0A 调色板
```

| 生成器 | 入口 | 关键初始化 | 已死/已清时的重进语义 |
|---|---|---|---|
| `SpawnMotherBrain` `07:EEAE / prg7:9682` | 特殊项 `type=$06` | `status=1`、`hi=GetNameTableAtScrollDir_`、锁横滚 `ScrollBlockOnNameTable3[x]` 置位 `$01`、两个动画延迟 `$20` | `CheckForItem(ui_MOTHERBRAIN)` 命中 → `status=8`、`TimeBombCounter=0`（重画 `TIME BOMB` 字幕） |
| `SpawnZebetite` `07:EECA / prg7:9702` | 特殊项 `type=$07` | 槽 `x=(byte&$F0)>>1`；`roomRAMPtr.lo = (((byte&$F0)<<1 & $10) ^ $10) 或 $84`；`roomRAMPtr.hi = GetRoomRAMPtrHi`（`$61`/`$65`）；`status=1`、三项计数清 0 | `CheckForItem(ui_ZEBETITE1+x/2)` 命中 → `status=$81`、`isHit=1`、`qtyHits=7`（=看起来"打到只剩一层"） |
| `SpawnRinkaSpawner` `07:EEEE / prg7:9722` | 特殊项 `type=$08` | **先试槽 1，槽 1 已存在才退槽 0**；`status=byte/16`（0/1 选 `RinkaSpawnPosTbl` 偶/奇列）；`hi=GetNameTableAtScrollDir_` | 两槽都占 → 本项丢弃（`bpl @RTS`） |

三个入口的共同点：`hi` 一律来自 `GetNameTableAtScrollDir_`（`03:9D88 / :1036`：`(PPUCTRL_ZP ^ ScrollDir) & 1`），
不用 Samus 的 hi。**移植时必须用"滚动方向+PPUCTRL"推 nametable**，直接抄 Samus.hi 会在横滚过半屏时错位（§10 陷阱一）。

### 4.2 Zebetite（5 座能量屏障）

| 项 | 值 | 证据 |
|---|---|---|
| 槽数 | 5（`Zebetites` = `$0758` 起，`_sizeof_Zebetites.0 = 8`） | `03:A28B / :2076`（`x=$20` 起每 `-8` 到 0） |
| 存活判据 | `status & $0F == 1` | `03:A29B / :2091` |
| 命中登记 | 弹型=`wa_Missile`；tile `$90..$97`（偶地址）；`Temp04_RoomRAMPtr & $9E == Zebetites.roomRAMPtr` 且 `+1` 相等 | `03:A103-A13D` |
| 计数 | `qtyHits++`；每**偶数**击更新外观 | 同上 |
| 外观 5 档 | `VRAMString0C`(100%)/`0D`(75%)/`0E`(50%)/`0F`(25%)/`07`(dead)，索引=`qtyHits>>1` | `03:A310 / :2184` |
| 死亡 | `qtyHits==8`（`(3+carry)*2` 的 `sbc #$03` 归零）→ `status++`（=2，停止更新） | `UpdateZebetite 03:A29B / :2091`（尾段） |
| 自愈 | 每次成功重绘置 `healingDelay=$40`（64 帧）；未被击时逐帧减，到 0 → `qtyHits--` 并按需重绘 | `03:A2EB-A309` |
| 重绘失败回滚 | `VRAMStructBufferIndex != 0` 或 `DrawTileBlast` 失败 → 撤 `status` bit1、`isHit=1`、`qtyHits--`（下帧重试） | `03:A2DA` |
| `isHit` 清零 | 每次 `UpdateZebetite` 出口无条件清零 | `03:A30A` |

⇒ 语义：**8 发导弹打掉一座**，但每 64 帧没挨打就自愈一发；自愈只回"厚度"不回"完全死亡"
（进了 `status` bit1 就不再更新）。玻璃后面的 MotherBrain 只有五座全灭后才可被导弹碰到？——不是硬门禁：
Zebetite 是**实体墙的瓦块**，物理阻挡由 tile 本身完成，代码里没有"必须先打 Zebetite"的判断（§4.5）。

### 4.3 MotherBrain 状态机（`MotherBrainStatus = $0098`）

`MotherBrainStatusHandler 03:9DD4 / :1091` 的 11 态（全态，A 级）：

| status | 名字 | 每帧行为 | 出口 |
|---|---|---|---|
| 0 | 不在房内 | `rts`（`AreaRoutine_Tourian` 继续跑其余子系统） | — |
| 1 | `Idle` | 碰撞判定 → 受击判定 → 脑动画 → 眼动画 → 绘制 → 清 `IsHit` | 被击中→2；32 击→3 |
| 2 | `Hurt` | 闪 16 帧（`Palette07/06` 交替），`ScrollY` 不动 | `FlashDelay` 到 0 → 1 |
| 3 | `Killed` | 闪 + **震屏**（`ScrollY = (delay&2>>1)*4`）；闪完那一帧做死亡交接 | → 4 |
| 4 | `Disappear` | 每帧 `BombExplode` 音效、逐段消融背景、闪 `Palette08/09` | 5 段死亡串画完 → 5 |
| 5 | `TimeBombMessage` | 用 `TileBlasts.0` 一段段拼 `TIME BOMB SET GET OUT FAST!` | `counter==8` → 6 |
| 6 | `SetTimeBomb` | 生成蓝门 + 启动 9999 帧计时（失败则下帧重试） | `status=0` |
| 7 | `TimeBombExploded` | 每帧爆炸音效；`Timer3`（120 帧）走完 → Samus 死亡 | → `$0A` |
| 8 | `TimeBombMessage_ScrollBackOnScreen` | 与 5 同体（重画字幕，`counter` 从 0） | `counter==8` → 9 |
| 9 | `SetTimeBomb_ScrollBackOnScreen` | 重新生成门、重启计时器使能（**不重置 9999**） | `status=0` |
| `$0A` | 已死 RTS | `rts`（死亡链据此走 `GameOver`） | — |

**碰撞**（`03:9DF2 / :1110`）：同 nametable；`Samus.x-$48 < $2F`（`$48..$76`）；
`|Samus.y-$80| < $20`（注释 `$61..$9F`）→ `HealthChange=$0200`（低字节 `$00`、高字节 `$02`，
BCD 十位=`2` ⇒**20 格**）+ `Samus.isHit=$38` + `CommonJump_SubtractHealth`。

**受击**（`03:9FED / :1528`）：`IsHit` 才处理；`SFXMulti |= BossHit`；`QtyHits++`；
`<32` → `status=2`、`FlashDelay=$10`；`>=32` → **清全部 13 个 TileBlast**（`routine=0`）、`status=3`、`FlashDelay=$80`。

**死亡交接**（`03:9E52 / :1190`，`Killed` 闪完那帧）：`DeathStringID=0`；清 `EnsExtra.status`
（`x=0..$B0` 步 `$10`，即 6 敌槽 + 6 弹槽）；`status=4`；`FlashDelay=$28`（40 帧）；`SFXNoise |= SilenceMusic`。

**消融**（`03:9E86 / :1230`）：5 段（`DeathString id 0..4`），每段用 `DeathStringOffsets` 取 2×2 清除块，
`InstrID` 每帧自增扫完一段；每段完成 → `DeathStringID++`、`FlashDelay=$1C`（28 帧），旧 id==4 时
`lsr` 成 `$0E`（14 帧）；id>4 → **`MusicInitFlags |= music_Escape`、`status=5`、`TimeBombCounter=-$80`**
（注释 2.6 s）。

**字幕**（`03:9F02 / :1341`）：`counter<0` 等；`==8` → `status++`；否则画第 `counter` 段：

| idx | `roomRAMPtr`（+`$6142`，带 hi 进位） | 内容串 | 画面文字 |
|---|---|---|---|
| 0 | `$6142` | `VRAMString08` | `TIME B` |
| 1 | `$6182` | `VRAMString02` | `GET OU` |
| 2 | `$614A` | `VRAMString09` | `OMB SET` |
| 3 | `$618A` | `VRAMString03` | `T FAST!` |
| 4 | `$61C2` | `VRAMString0A` | 清/延伸行 |
| 5 | `$6202` | `VRAMString04` | 清/延伸行 |
| 6 | `$61CA` | `VRAMString0B` | 清/延伸行 |
| 7 | `$620A` | `VRAMString05` | 清/延伸行 |

绘制条件：`VRAMStructBufferIndex==0` 且 `DrawTileBlast` 成功才 `counter++`（否则下帧重试同一段）。

**计时启动**（`03:9F49 / :1412`）：`SpawnDoor` 失败即 `rts`；成功 → `status=0`、`EndTimer/EndTimer+1=$99`
（BCD 9999，注释 166.65 s）、`EndTimerEnemyIsEnabled=1`、`EndTimerEnemyHi=MotherBrainHi`。

**门**（`03:9F69 / :1438`）：槽 = `rol(MapPosX+MapPosY) & $03`（顺序表 `Doors.0/3/2/1`）；`DoorHitPoints=1`（立即关）、
`DoorType=1`（蓝）、`Objects.status=3`（开）、`hi=MotherBrainHi`、`x=$10`、`y=$68`、`ObjAnim_55`、
`TileBlast.animFrame=VRAMString10`（`$A0`×12 门瓦）、`roomRAMPtr = $40 | ((MotherBrainHi<<2 | $61)<<8)`。

**爆炸**（`03:9FC0 / :1488`）：`Timer3` 走完 → `Samus.status = sa_Dead2`、`MotherBrainStatus=$0A`、
`Palette00`；asm 注释自陈 `Bug? Doesn't set ObjAnimDelay to 0`（D4：**保真保留**，登记而非修）。

### 4.4 Statue（Kraid/Ridley 像 → 开桥）

| 项 | 内容 | 证据 |
|---|---|---|
| 抬起 | 各像自身状态 `bit7=已抬/bit0=已击败`（Kraid `$687B`、Ridley `$687C`） | `07:DA7C / prg7:5341`，详见 [docs/findings/t45-1-statue-gate-verification.md](../findings/t45-1-statue-gate-verification.md) |
| 桥门判 | `StatuesBridgeIsSpawned` 已置 → 退；`DoorEntryStatus!=0` → 退；`KraidStatueStatus & RidleyStatueStatus` 结果**为正**（任一不满足 bit7）→ 退 | `07:DADA / prg7:5416` |
| 开桥动作 | `StatuesBridgeIsSpawned` 写回 `$80` 起（bit7 置位）；生成 **8 个 TileBlast**（`routine=3`、`delay=y*2`、`type=4`、`roomRAMPtr.hi=$62 或 (Statue.hi<<2)`、`.lo=y*2+8`，y=8..1 递减） | `07:DAF0-DB10` |
| 进 Tourian 的门 | Brinstar `(05,02)` 的 Item3 红导弹门，目标 `.word $2882` | [docs/findings/tourian-entry-path.md](../findings/tourian-entry-path.md) |

⇒ 桥不是"对象"而是 8 次定时瓦块重绘；Tourian 入口只在桥生成并走过之后可达。

### 4.5 玻璃（MotherBrain 的罩子，tile `$98`）

`03:A0C6 / :1732` 的玻璃分支：`UpdatingSamusProjectile!=0`、`Objects.status==wa_Missile`、命中 tile `$98` →
找空 `TileBlast`（13 槽，步 `$10`）；`roomRAMPtr.lo` **强制 `$8C`**、`.hi = 命中轴 hi + 1`；
`animFrame = VRAMString01`（`$32` + `FF×6` = **3 列 × 2 行**清除）；交给 `DrawTileBlast`。
无空槽 → 什么都不做（导弹仍按"撞上"处理，§5）。

⇒ 一次命中清 3×2 格；玻璃整体有多高、需要几发、`$8C` 这个固定锚为何成立——属几何反解，
登记为 C 级（§10 C-04），不影响"照抄指针+串号"的实现。

## 5. 武器交互（弹 → 背景/玻璃/Zebetite/MotherBrain）

### 5.1 调用链（A 级，逐行）

```
DoOneSamusProjectile (07:D4CB)          ; 槽 $D0/$E0/$F0
 └ UpdateBullet (07:D4EB)               ; UpdatingSamusProjectile=1
    └ UpdateBullet_CollisionWithBG (07:D609)
       ├ tile >= $A0 → UpdateBullet_Move（空气，直接移动）
       ├ jsr GotoUpdateBullet_CollisionWithMotherBrain   ; 03:A142（对所有实心 tile 都调用）
       ├ tile == $4E → SamusProjectileHitDoorOrStatue
       ├ jsr CheckBlastTile (07:D651)   ; C=1 表示可爆瓦
       └ clc; jmp IsBlastTile (07:E9BE)
          └ jsr GotoUpdateBullet_CollisionWithZebetiteAndMotherBrainGlass  ; 03:A0C6
             └ @exit: pla pla clc rts   ; 双返回：跳过 IsBlastTile 的非 Tourian 尾部
```

双返回的真实效果（`07:D4F8` 之后）：回到 `UpdateBullet` 的 `CheckBulletStat`，carry=0 ⇒ **弹进入
`UpdateBullet_Explode`**（导弹换成 `ObjAnim_MissileExplode`）。所以"在 Tourian 打瓦"=弹消失、瓦不动。

### 5.2 弹型 × 目标 效果表（A 级）

| 目标 tile | 光束（1/2/3/81） | 导弹（`wa_Missile=11`） | 证据 |
|---|---|---|---|
| `$A0`–`$FF` 空气 | 继续移动 | 继续移动 | `07:D615` |
| 实心不可爆（`$00`–`$6F`、`$9E`–`$9F` 等） | C=0 → 爆炸消失，无副作用 | 同左 | `07:D64D` 分支 |
| `$4E` 门/像 | 交给 `SamusProjectileHitDoorOrStatue` | 同左 | `07:D61D` |
| 可爆瓦 `$70`–`$97`、**非** Tourian | 破坏该瓦、弹继续飞（C=1） | 同左 | `07:E9DB` 起 |
| 可爆瓦 `$70`–`$97`、**Tourian** | 双返回：爆炸消失、瓦不破坏 | 同左（另有下列专门效果） | `07:E9C2` 注释 + `03:A13E` |
| `$98` 玻璃 | 双返回（无效） | 3×2 清除（§4.5） | `03:A0D3-A102` |
| `$90`–`$97` Zebetite | 双返回（无效） | 命中登记 `isHit=1`（§4.2） | `03:A103-A13D` |
| `$5E`–`$71` MotherBrain 本体 | 无伤害（只有 `IsHit` 判定被跳过） | `MotherBrainIsHit=1` | `03:A142 / :1834` |

注意 `$5E`–`$71` 与可爆区 `$70`–`$97` 有重叠（`$70/$71`）：那两格**先**记 MB 命中、**再**走爆瓦判定，
所以打在这两格上的导弹既算 MB 命中也会爆炸消失；`$5E`–`$6F` 不可爆，只算命中。

### 5.3 伤害与音效请求（A 级）

| 事件 | 写入 | 效果 |
|---|---|---|
| 触碰 MotherBrain | `HealthChange=$0200`、`Samus.isHit=$38` | BCD 十位 2 ⇒ **20 格**；`SubtractHealth` |
| Metroid 吸血 | `HealthChange=$0004`（每帧一次，非叠加） | 1/4 格/帧 |
| 触碰普通敌（Rinka 等） | `AreaEnemyDamage=$0300`（`03:95CE`） | 十位 3 ⇒ **30 格** |
| 打中 MotherBrain | `SFXMultiInitFlags` 置位 `sfxMulti_BossHit` | 音效请求（不直接播） |
| MotherBrain 死亡 | `SFXNoiseInitFlags` 置位 `sfxNoise_SilenceMusic` | 静音 |
| MotherBrain 消融中 | 每帧 `sfxNoise_BombExplode` | 连续爆炸声 |
| 逃逸计时运行 | 每 32 帧 `SFXSQ1InitFlags` 置位 `sfxSQ1_OutOfPipe` | 逃逸曲中即"警报"音 |
| Metroid 无效命中 | `SFX_MetroidHit`（未冻结）/`SFX_Metal`（真无敌） | `07:F536 / prg7:11038` |

`SubtractHealth`（`07:CE92 / prg7:3203`）的四道守卫，**任何一条命中就不扣血**：
①`HealthChange==0`；②Samus 已死；③`EndTimer+1 != $FF`（**逃逸计时已启动 = 无敌**）；④`MotherBrainStatus>=3`。
Varia 减半：`lsr HealthChange; lsr HealthChange+1; bcc` 时跳过补偿，否则高字节 `+$4F`（十位域补偿）。

### 5.4 三个必须照抄的机制（否则一定发散）

1. **双返回**：`pla pla clc rts` 不是"提前 return"，它把 `UpdateBullet_CollisionWithBG` 的返回地址也吃掉
   并强制 carry=0。改成普通 `rts`（carry 保留）会让子弹在 Tourian **穿过瓦块**。
2. **弹型判定在读 tile 之前**：`Objects.status != wa_Missile` 直接走 `@exit`（双返回），
   所以光束打 Zebetite/玻璃也是"爆炸消失"而不是"无效穿过"。
3. **`$9E` 掩码比较**：Zebetite 命中用的是 `Temp04_RoomRAMPtr & $9E` 与 `roomRAMPtr` 的**指针**比较，
   不是坐标比较；`$9E` 保留 bit1-4 与 bit7，把 8px 粒度折成 16px 粒度的列配对。

## 6. 结局与逃逸链

### 6.1 自毁计时（`EndTimer` = `$010A/$010B`，BCD 9999）

| 阶段 | 行为 | 证据 |
|---|---|---|
| 启动 | `MotherBrain_SetTimeBomb` 写 `$99/$99`、`EndTimerEnemyIsEnabled=1`、`EndTimerEnemyHi=MB.hi` | `03:9F49 / :1412` |
| 递减 | `UpdateEndTimer`：低字节 BCD `-1`，高字节 BCD `-0`（借位经 carry 链，`Base10Subtract 07:C3FB`） | `03:A1E7 / :1963` |
| 警报 | `FrameCount & $1F == 0` → `SFXSQ1` 置位 `OutOfPipe` | `03:A216` |
| 数字显示 | `DrawEndTimerEnemy`：借 `EnMotherBrain` 槽画 `EnFrame1A`（`y=$84,x=$64`），成功后把 3 个 OAM tile 改成 `$A0+digit`（百/十位取 `EndTimer+1` 高/低 nibble，个位取 `EndTimer` 高 nibble） | `03:A238 / :2021` |
| 归零 | `EndTimer+1--`（=`$FF` 停用）、`MotherBrainQtyHits=0`、`status=7`、`SilenceMusic`、`Timer3=$0C`（120 帧=2 s）、`Palette0A`（全白） | `UpdateEndTimer 03:A1E7 / :1963`（尾段） |
| 爆炸结束 | `TimeBombExploded` 等 `Timer3==0` → `Samus.status=sa_Dead2`、`status=$0A`、`Palette00` | `03:9FC0 / :1488` |

**逃逸期全场静默规则**（容易被漏，A 级）：
- Rinka 生成器停（`EndTimer` 有效时 `UpdateAllRinkaSpawners` 直接退，`:1863`）；
- Metroid 自杀（`metroid.asm:2-8`）；
- 炮台只闪不射（`UpdateCannon` 走 `@escape` 分支，`:9B5F-9B6B`）；
- Samus 吃不到任何伤害（`SubtractHealth` 守卫③）。

⇒ 逃逸段的唯一失败源是**计时**，不是敌人。这与"边跑边被打"的直觉相反。

### 6.2 逃逸径（A 级锚点 + 门/电梯表）

| 环节 | 内容 | 证据 |
|---|---|---|
| 出口门 | MotherBrain 房内 `x=$10,y=$68` 生成蓝门（关状态由 `DoorHitPoints=1` 保证立即"关→开"动画） | `03:9F69` |
| 竖井 | 5 个 Rinka 生成器（`(07,04)`、`(07,09)`、`(08,0A)`+$10、`(09,0A)`、`(0A,0A)`+$10）分布在上行竖井各屏（另有 3 个在 MB 长厅） | `tourian/items.asm`（§7.2） |
| 竖井底门 | `(0B,01)` 蓝门 `$A0`+(0<<4) + 音乐切换 | 同上 |
| 结局电梯 | `ElevatorType==$8F` → `MainRoutine=_id_IncrementRoutine`、`inc AtEnding`、`RoomPtr=0`、`BankSwitchPending=1`、`TitleRoutine=_id_EndGame` | `07:D8BF / prg7:4980` |
| 回 Brinstar 电梯 | `(04,03)` `ElevatorType $83`（非结局出口） | `items.asm` |

`AtEnding` 是全局结局标志（HUD/音乐/密码惩罚都用它）；`TitleRoutine=_id_EndGame` 才把控制权交给结局屏。

### 6.3 结局屏状态机（`EndGame`，7 态，A 级）

`00:9AA7 / prg0:3712` 用 `RoomPtr` 当下标跳表，态与转移：

| 态 | 处理器 | 关键动作（常量全为 A 级实测） |
|---|---|---|
| 0 | `LoadEndGFX 00:9AD5` | `ClearAll`/`InitEndGFX`；`EndingType` = `JustInBailey?4:0`（0=有装甲、4=无装甲）；`SpritePointerIndex=EndingType*2`；`VRAMStruct_EndBackground`；`NMIOn`；`SFXMultiInitFlags=EndMusic`；`Timer3=$60`（960 帧=16 s）；`SpriteByteCounter=$36`；`ClrChangeCounter=8`；清 `IsCredits/EndMsgWrite/HideShowEndMsg/CreditPageNumber`；`ScreenOn` |
| 1 | `ShowEndSamus 00:9B1C` | `Timer3==0` → 下一态；`==$50` → `EndMsgWrite++`（注释 160 帧/2.6 s 后写字幕）；`==$01` → `HideShowEndMsg++`（注释 950 帧/15.8 s 后擦字幕） |
| 2 | `EndSamusFlash 00:9B34` | 每 32 帧 `ColorCntIndex++`；`ClrChangeCounter` 每帧 −1，到 0 时取 `PaletteChangeTable[ColorCntIndex]`（`00:9B83` = `08..01,01..08`）并 `SpriteAttribByte` 0..3 循环（闪光节奏）；`ColorCntIndex==8` → **`ChooseEnding`+`CalculatePassword`**、`SpritePointerIndex=EndingType*2`、`SpriteByteCounter=$36`；`==$10` → `Timer3=$10`（160 帧）、`SpriteAttribByte=EndingType>=4?1:0`、`RoomPtr++`、`EraseAllSprites` |
| 3 | `SamusWave 00:9B93` | `Timer3==0` → `Timer3=$10`、`EndGamePalette07`、`RoomPtr++`；**只有 `EndingType>=4` 才挥手**（`WaveSpritePointer=(EndingType-4)<<2`，`FrameCount bit3` 决定手抬/落，每 8 帧，`SpriteByteCounter=$2D`）；`<4` 只重载站姿 |
| 4 | `EndFadeOut 00:9BCD` | `Timer3==0` 后每 8 帧 `PaletteDataPending++`，直到 `EndGamePalette0B+1` → `Timer3=$10`、`RoomPtr++`；期间 `IsCredits==0` 时先置 `EndGamePalette07`+`IsCredits=1`（asm 注「*This code does not appear to be used」）；尾段按 `EndingType` 选站姿/挥手精灵 |
| 5 | `RollCredits 00:9BFC` | `Timer3` 剩 `$02` 时 `ScreenOff`→清 nametable0+精灵→`EndGamePalette0B+1`→`ScreenOn`；`CreditPageNumber 1..6`、`ScrollY` 每 4 帧 `+1`（第 6 页到 `$88` 结束）；换 nametable 边界见 `00:9C17` 起 |
| 6 | `Restart 00:9A39` | 等 START → 清 `PasswordByte`/`UniqueItemHistory` → `InitializeGame` |

结局面字写入由 `NMIScreenWrite 00:9A07 / prg0:3590` 负责，条件 `TitleRoutine >= $1D`；
`EndMsgWrite 1..4` 写、`HideShowEndMsg 1..4` 擦（指针表 `EndMessageWritePtrTable`/`EndMessageErasePtrTable`）。

### 6.4 结局等级与年龄（`ChooseEnding` + `UpdateAge`）

| 项 | 内容 | 证据 |
|---|---|---|
| 年龄累计 | `UpdateAge` 每 256 帧 `SamusAge++`（最低字节到 `$D0` 归零进位，非 256） | `07:C97E / prg7:2088` |
| 等级判定 | `SamusAge+2 != 0` → 最差；否则 `SamusAge+1` 与 `AgeTable-1,y` 比较定 `EndingType 1..5` | `07:CAF5 / prg7:2439` |
| 年龄表 | `AgeTable 07:CB18 = $7A,$16,$0A,$04`（122/22/10/4，秒级阈值） | 同上 |
| 无装甲 | `EndingType >= 4` → `JustInBailey=1` | 同上 |
| 密码回填 | `EndSamusFlash` 的 `ColorCntIndex==8` 调 `ChooseEnding`+`CalculatePassword` | `00:9B34` |
| 已知 BUG（D4 保真） | 判最差结局时只查 `SamusAge+2`，asm 注释自陈「BUG! Should be checking SamusAge+3 here too」⇒ >37 小时也会落最差档（行为无差，但最高字节被忽略） | `07:CAFB / prg7:2442` |

`docs/findings/ending-sequence-rewrite-block12.md`（[链接](../findings/ending-sequence-rewrite-block12.md)）声称
`ending/sequence.rs` 已实现 `choose_ending`/`play_ending`——**实测已被删除**
（`git show --stat 6aae8b32` 显示 `crates/game-core/src/ending/sequence.rs | 127 ---------`，
现存 `crates/game-core/src/ending/mod.rs` 只有 1 行注释、无函数、无调用者）。结局链当前**代码面为 0**，
该 findings 的状态行需要更正（G-8 的一部分）。

### 6.5 失败链（计时归零 → 死亡 → GameOver → 密码屏）

| 步骤 | 行为 | 证据 |
|---|---|---|
| 死亡判 | `Samus.status==sa_Dead2` 且 `animDelay==0` → `SilenceMusic` | `07:C95F / prg7:2055` |
| 分支 | `MotherBrainStatus==$0A` → **`inc MainRoutine`（直接 GameOver）**；否则 `SetTimer(A=$04,X=$04)`（`Timer3=4`=40 帧，`NextRoutine=4`） | 同上 |
| 40 帧后 | `PrepareGameOver 07:C9A6 / prg7:2126` → `TitleRoutine=GameOver 00:939E` | 同上 |
| 密码屏 | `DisplayPassword 00:9359 / prg0:2795`，等 START → `CheckPassword` | 同上 |

⚠ 现存 Rust 缺陷（本次调研发现，A 级反证）：`crates/game-core/src/frame/tick.rs:660` 把
`mem.read_ram(0x687B)` 当 `MotherBrainStatus`，但 `MotherBrainStatus=$0098`
（`constants_ram.asm:708`），`$687B=KraidStatueStatus`（`constants_ram.asm:1234`）。
后果：站在 Tourian 里死亡时"MB 已死 → 立即 GameOver"分支永不触发，一律走 40 帧计时器。
（§8 G-9；本文只登记，不改代码。）

## 7. 数据面

### 7.1 三份 asm 及其 include 的数据区（A 级：地址/行数）

| 数据 | 地址 | asm 行 | 规模 | ROM 偏移（bank3） |
|---|---|---|---|---|
| `GFX_KraidSprites` | `03:8D60` | 32 | `$400`（CHR，本区房复用 Kraid 精灵） | `0xD60+0x0D60`？见下注 |
| `GFX_RidleySprites` | `03:9160` | 42 | `$400` | 同上 |
| `PalettePtrTable` | `03:9560` | 53 | 27 项（`$A718` 起） | — |
| `SpecItmsTblPtr` | `03:9598` | 83 | 指针 → `$A83B` | — |
| `EnemyDeathAnimIndex` 等 9 表 | `03:960B`–`03:977B` | 184–412 | 每表 16 B | `0x160B` 起 |
| `EnProjectileMovementPtrTable` | `03:97A7` | 433 | 4 字 | `0x17A7` |
| `VRAMStringPtrTable` + `VRAMString00..10` | `03:97AF` / `03:A32B` | 439 / 2210 | 17 项 + 11 串 | `0x17AF`/`0x232B` |
| `CannonInstrLists` / `AnimFrameTable` / 偏移表 | `03:9D99`–`03:9DD1` | 1057–1084 | 5 组 + 8 向 | `0x1D99` |
| `RinkaSpawnPosTbl` | `03:A1DB` | 1954 | 12 B | `0x21DB` |
| `ZebetiteAnimFrameTable` | `03:A310` | 2184 | 5 B | `0x2310` |
| `MotherBrainDeathString` + Offsets | `03:A0A3` / `03:A0C1` | 1686 / 1723 | 5 串 + 偏移 | `0x20A3` |
| 房/结构/元件/调色板/精灵/道具 | `03:A406`–`03:AF49` | include | 见 §7.2 | `0x2406` 起 |

注：ROM 偏移按 `prg 偏移 = 3*0x4000 + (addr-0x8000)` 计算，`$8D60` → `0xC000+0x0D60=0xCD60`；
读法示例：

```bash
python3 - <<'PY'
rom=open("rom/Metroid (USA).nes","rb").read()[16:]
def at(bank,addr,n=16):
    off=bank*0x4000+(addr-0x8000); return rom[off:off+n].hex(' ')
print(at(3,0x960B,16))   # EnemyDeathAnimIndex 首 16 B
print(at(3,0xA1DB,12))   # RinkaSpawnPosTbl
PY
```

### 7.2 特殊项表全文（`tourian/items.asm`，`SpecItmsTbl_ 03:A83B`，A 级）

格式：长项 = `MapY` + `word 下个不同 Y 项` + `MapX` + `本 Y 段下项偏移-1` + `type`；短项省略 word；
`$FF` 段终止、`$00` 全表结束。type 词表：`01` 敌、`02` 道具、`03` Mellow、`04` 电梯、`05` 炮台、
`06` MotherBrain、`07` Zebetite、`08` Rinka 生成器、`09` 门、`0A` 调色板。

| 坐标 (Y,X) | 项 | 解码 |
|---|---|---|
| (03,01) | 电梯 `$8F` | 结局电梯「Elevator to end」 |
| (04,03) | 电梯 `$83` | 回 Brinstar「to Brinstar」 |
| (07,03) | 门 `$A0`+`0<<4`+TenMissile | 十发导弹门 |
| (07,04) | Rinka 生成器 | 竖井 |
| (07,09) | Rinka 生成器 | 竖井 |
| (08,0A) | Rinka 生成器 `+$10` | 竖井（奇列） |
| (09,0A) | Rinka 生成器 | 竖井 |
| (0A,0A) | Rinka 生成器 `+$10` | 竖井（奇列） |
| (0B,01) | 门 `$A0`+`0<<4`+BlueMusicChange | 竖井底（音乐切换） |
| (0B,02) | MotherBrain + Zebetite `+$40` + Rinka 生成器 `+$10` + 炮 `$49/$4B/$3E` | MB 房首屏（Zebetite 槽 4） |
| (0B,03) | Zebetite `+$30`、`+$20` + Rinka + 炮 `$41/$43/$36/$49/$4B/$3E` | MB 房中屏（槽 3、2，6 炮） |
| (0B,04) | 门 `$A0`+`1<<4`+BlueMusicChange、Zebetite `+$10`、`+$00` + Rinka + 同 6 炮 | MB 房末屏（槽 1、0） |
| (0B,05) | 门 `$A0`+`1<<4`+BlueMusicChange | MB 房右门 |

炮台字节解码（`SpawnCannonRoutine 03:9CE6 / :907`：`instrListID=(b&$F0)/16`、`y=(b&$F0)|7`、`x=((b*16)&$FF)|7`）：

| byte | listID | 位置 (y,x) |
|---|---|---|
| `$49` | 4 | (`$47`,`$97`) |
| `$4B` | 4 | (`$47`,`$B7`) |
| `$3E` | 3 | (`$37`,`$E7`) |
| `$41` | 4 | (`$47`,`$17`) |
| `$43` | 4 | (`$47`,`$37`) |
| `$36` | 3 | (`$37`,`$67`) |

房号：`RoomPtrTable_ 03:A7D1` 共 21 项，`Room00..Room14`（`0x00`–`0x14`）。
Zebetite 槽号即高 nibble `>>1`：`$00/$10/$20/$30/$40` → 槽 0/1/2/3/4，
其 `roomRAMPtr.lo` 按 `((n&$F0)<<1 & $10)^$10 | $84` 得 `$94/$84/$94/$84/$94`（§4.1）。

### 7.3 资产包现状（A 级：`crates/game-data/src/pack.rs` 段清单）

| 数据面 | 包内键（`PackKey`） | 状态 |
|---|---|---|
| Tourian 房指针/房流/结构/元件 | `ToRoomPtrs`(`area.to.room_ptrs`)、`ToStructPtrs`、`ToMetatiles`、`ToRoomStreams`、`ToRoomOffs`、`ToStructures`、`ToStructOffs` | **已有** |
| Tourian 特殊项 | `PackKey::ToItems`（`area.to.items`） | **已有** |
| 敌属性 9 表 | `ToEnemyHealth`、`ToEnemyFacing`、`ToEnMovPtrs`、`ToEnMovStr`、`ToEnemyDeathAnim` | **已有** |
| 敌精灵/动画 | `ToEnSprites`、`AreaToEnAnimIdx`、`AreaToEnMovChoices`、`AreaToEnProjMoveStr` | **已有** |
| Metroid 14 B 初值（`InitBank3` 抄 `$77F0`） | `TourianMetroidData`（`tourian.metroid_data`） | **已有** |
| 区调色板切换值 | `AreaPalToggle`（`area.pal_toggle`） | **已有** |
| 全局调色板/精灵 CHR | `PalPtrs`/`PalData`/`PalOffs`、`SpriteData`、`GfxInfo`/`GfxData` | **已有** |
| 结局屏背景/字体/结局面字/Staff 文本 | 无专用键（现仅 `EndStar` 一颗星） | **缺**，见 G-8 |
| `VRAMString00..10`（Tourian 专用字串） | 无键 | **缺**（可现算，也可建键），G-4 |

结论：**Tourian 区域数据面已经齐**（房流/结构/元件/道具/9 表/精灵/调色板/metroid 初值），
缺的只有"结局屏 + Tourian VRAMString 表"。`docs/findings/room-sweep.md` §8.2 也确认
`crates/game-data/src/assets.rs:238-247` 对 `0..=4` 区都有 room_streams。

## 8. 缺口清单（按依赖排序，每项一张可独立提交的卡）

依赖关系：G-1 → G-2/G-3/G-4/G-5 → G-6 → G-7 → G-8；G-9 独立（可并行，属修缺陷）。

| ID | 依赖 | 范围（目录/文件） | asm 锚 | 验收 |
|---|---|---|---|---|
| **G-1** | — | `crates/game-data/src/assets.rs`、`crates/game-data/src/area/enemies.rs`：把 `ToItems`+9 表+`TourianMetroidData` 暴露给 game-core；`frame/tick.rs` `InitBank3` 抄 `$77F0` | `03:9598`、`03:960B-977B`、`07:C590 / prg7:1449` | 单测：9 表逐字节等于 ROM（`at(3,0x960B)` 起）；`$77F0..$77FD` 14 B 等于 `TourianMetroidData` |
| **G-2** | G-1 | `crates/game-core/src/enemy/`：`metroid_ai.rs`、`rinka_ai.rs`；`ai_table.rs` 的 `routine_for(3,0/1/4)` 去掉 `Unported` | `03:97F9`（metroid.asm 全 378 行）、`03:9A2C`（rinka.asm 222 行） | 分支枚举测：latch 0..$0F × 六槽组合 → `MetroidOnSamus` 与 `HealthChange` 逐帧等于 asm 复刻；Rinka 定速一次后速度不再变（改 Samus 位置不影响） |
| **G-3** | G-1 | `crates/game-core/src/enemy/`：`cannon.rs`（16 槽更新/指令流/弹生成） | `03:9B37`–`03:9DC6` | 指令流回放：List0..4 在 40 帧步进下的 `(angle,instrID)` 序列与 asm 一致；`MotherBrainStatus>=4` 后不再出弹 |
| **G-4** | G-1 | `crates/game-core/src/enemy/`：`zebetite.rs` + `weapons` 侧的 `zebetite_glass_hit` | `03:A28B`、`03:A29B`、`03:A0C6`、`03:A142` | 单测：8 发导弹死/64 帧自愈/重绘失败回滚；玻璃命中生成 `VRAMString01` 的 3×2；光束命中只爆炸不登记 |
| **G-5** | G-4 | `crates/game-core/src/enemy/`：`mother_brain.rs`（11 态 + 门 + 字幕 + 消融） | `03:9DD4`–`03:A0A3` | 状态机驱动测：32 击 → 3 → 4（5 段）→ 5（8 段）→ 6（门+9999）→ 0；已死重进 → 8 → 9 → 0 |
| **G-6** | G-5 | `crates/game-core/src/frame/`：`end_timer.rs`（BCD 递减/报警/数字/归零）+ `escape.rs`（四个子系统停摆规则） | `03:A1E7`、`03:A238`、`03:9EE7`、`:1863` | BCD 单测：9999 逐帧递减到 0000 的完整序列；归零那一帧 `status=7`/`Timer3=$0C`/`Palette0A`；逃逸期 `AreaRoutine_Tourian` 只剩计时与绘制 |
| **G-7** | G-6 | `crates/game-core/src/world/elevator.rs`：`$8F` 分支（`AtEnding`/`TitleRoutine=_id_EndGame`） | `07:D8BF / prg7:4980` | 单测：`ElevatorType=$8F` → `MainRoutine=_id_IncrementRoutine`、`RoomPtr=0`、`BankSwitchPending=1`、`TitleRoutine=_id_EndGame` |
| **G-8** | G-7 | `crates/game-core/src/ending/`（重建 `sequence.rs`）+ `crates/game-data/src/pack.rs` 新增结局段 | `00:9AA7`–`00:9BFC / prg0:3712-3972`、`07:CAF5` | 7 态转移序列测（`Timer3` 边界 `$60/$50/$10/$01/0`）；`ChooseEnding` 年龄表 4 阈值 × `EndingType 1..5` 全枚举；缺段时 fail-loud |
| **G-9** | — | `crates/game-core/src/frame/tick.rs:660` 错址；`data/status/routine_inventory.json` 锚漂移 | `constants_ram.asm:708/1234`、`03:95AB` | 修后：MB=`$0A` 死亡走 `inc MainRoutine`，否则 40 帧 `NextRoutine=4`；清单 3 条锚重新逐条核对 |

每张卡的"验收方式"都必须在**无像素证据**下可执行——见 §9.2 的四级替代判据。

## 9. 验收现实约束与替代判据

### 9.1 为什么现在拿不到 Tourian 的像素证据（结构性约束，引自既有定谳）

[docs/findings/room-sweep.md](../findings/room-sweep.md) §8.1 的结论（本文复述，不新增主张）：

| 约束 | 内容 |
|---|---|
| 允许集 | `PRESET_VERIFIED_AREAS=(0,)`：只有 Brinstar 有"开场直达"的合法注入点 |
| 区 1（Norfair） | 逐房记 `entry_preset_unverified_area`（`entry_preset` 未定稿，不是"还没测"） |
| 区 2/3/4 | 记 `area_not_in_tool_allowlist`（工具白名单 `PORTED_AREAS=(0,1)` 自限；房流与渲染表都在包内） |
| 三轮注入实验 | 全部失败：同帧注入 `$0074`+14 B 区指针块 → `$005A` 到 `$F1` 后整机冻结（注入帧 `$8000` 窗口仍是 Brinstar bank ⇒ `(RoomPtrTable),y` 读垃圾 ⇒ DrawRoom 走飞） |
| 拆段注入 | 仍冻结：`CopyAreaPointers 07:C8B0` 只在 `AreaInit 07:C875` 与电梯到达径（`prg7_engine.asm:5038`）被调 |
| 工具侧门 | 房扫工具 `PORTED_AREAS=(0,1)` ⇒ 2/3/4 区记 `area_not_in_tool_allowlist` |
| 结论 | 非 0 区只有**真实到达后**才有合法观测窗口；替代 = TAS inputlog 真走，或电梯径 fixture 重放 |

同时可见 `crates/game-data/src/assets.rs:238-247` 对 `0..=4` 区都有 `room_streams`，
`crates/game-core/src/world/room_draw.rs:67-77` 的 `unported!` 只在 `assets` 返回 `None` 时触发
——**数据面已通、门在工具允许集**，所以"能不能渲染 Tourian 房"和"能不能拿到 Tourian 帧证据"是两件事。

### 9.2 四级替代判据（强度递增；能证明什么/不能证明什么）

| 级 | 判据 | 做法 | 能证明 | 不能证明 |
|---|---|---|---|---|
| **L0 字节锚** | ROM/asm 字节一致 | `at(bank,addr,n)` 与 asm 数据表逐字节比；常量用 `grep -n` 钉行号 | 数据表、常量、字符串、表长度 | 行为 |
| **L1 单帧判别测** | 状态面注入 → 跑 1 帧 → RAM 快照等于期望 | 直接写 `Manager` 的 RAM（不经房间渲染），调单个 routine | 该 routine 的输入/输出契约、边界（如 8 击/32 击/9999→0000） | 跨子系统顺序、真实触发路径 |
| **L2 状态面 sem 对拍** | Mesen 跑到 Tourian 后逐帧 dump RAM，与 Rust 同起点 diff | 需先有 L3 的观测窗口；diff 面 = `$0098-$009F`、`$010A-$010D`、`$0758-$077F`、`$0500-$05CF`、`$008B-$0090` | 帧级等价（含 quirk） | — |
| **L3 观测窗口** | TAS inputlog 真走到 Tourian（或电梯径 fixture 重放） | `oracle/` 输入回放 + 电梯 `(04,03)` 到达 | 合法窗口本身 | 像素白名单之外的视觉细节 |
| **L4 像素** | `gates/cli.py visual` | 依赖 L3 | 渲染 | 当前不可得（§9.1） |

### 9.3 到什么程度算可交付（按卡型给硬门槛）

| 卡型 | 可交付门槛（全部满足） |
|---|---|
| 数据面卡（G-1） | L0 全通过：9 表 + 14 B `TourianMetroidData` + `ToItems` 逐字节等于 ROM；包缺失时 `panic!` 文本含段名 |
| 单例程卡（G-2/G-3/G-4 的纯函数部分） | L1 分支枚举：每个 `cmp`/`bcc` 的两侧各有至少 1 个用例；asm 里出现的常量全部出现在测试断言里 |
| 状态机卡（G-5/G-6） | L1 全态迁移 + L2 的 `$0098-$009F`/`$010A-$010D` 区段 diff 为 0（拿到 L3 后补齐） |
| 接线卡（G-6/G-7） | L1：`AreaRoutine_Tourian` 六步顺序可用调用序断言（录 trace 的调用序，不比对像素） |
| 结局卡（G-8） | L1：7 态转移 + `ChooseEnding` 全域枚举；`EndingType` 5 值 × 年龄 4 阈值的 20 格矩阵全覆盖 |
| 修缺陷卡（G-9） | L1 两条分支各一例：`MB==$0A` 走 `inc MainRoutine`；否则 `Timer3=4`/`NextRoutine=4` |

**一句话验收口径**：没有像素时，"可交付"= **L0 逐字节 + L1 分支枚举 + 调用序/状态面闭包**；
一旦 TAS 走到 Tourian（L3 到手），同一张卡必须补 L2 的 RAM diff，否则只能算"逻辑就绪、未对拍"。

### 9.4 已知会拖慢验收的三件事

1. `MotherBrain_TimeBombMessage` 依赖 `DrawTileBlast` 的**成功/失败**（VRAM 缓冲区满就下帧重试），
   纯逻辑层必须把"绘制是否成功"建模成返回值，否则 `counter` 推进节奏会错（30 FPS 抖动）。
2. `DrawEndTimerEnemy` 借 `EnMotherBrain` 槽绘制并**改写 OAM 三个 tile**——纯逻辑层要对
   "SpriteRAM 写入"建立可断言面，否则数字显示无法验证。
3. 5 段消融的 `DeathStringOffsets` 与 `$6144 + hi*$400 + byte` 计算必须逐字节照抄，
   用"相对偏移"重写一次就会在第二屏错位。

## 10. C 级假设登记（不得写进实现）

| ID | 假设 | 为何只能是 C | 升 A 的做法 |
|---|---|---|---|
| C-01 | `type=$02` 的实体身份（表注释「i dunno but it takes 30 damage with varia」） | 派发与属性已知，但没有任何房间数据被反解出"哪个敌位用了 type02" | 反解 `tourian/rooms.asm` 的敌位表，统计出现的 type 字节 |
| C-02 | `type=$03`「disappears」是否被任何房间引用 | 同上 | 同上 |
| C-03 | 玻璃（tile `$98`）的总高度与所需发数；`roomRAMPtr.lo=$8C` 这个固定锚的几何含义 | 只知"每次命中清 3×2、锚点恒 `$8C`" | 反解 `tourian/metatiles.asm` 中含 `$98` 的四象限，再数房间流里的出现位置 |
| C-04 | 5 座 Zebetite 是否"横向铺满走廊"从而构成硬门 | 命中与自愈已证；阻挡来自 tile 本身（`tile<$A0` 即实心，A 级），但"是否必须全灭"取决于排布 | 同一房三屏的 tile 图（C-03 的产物） |
| C-05 | 逃逸径的具体房序（哪几屏 → 电梯） | `items.asm` 只给生成点坐标，不含连通 | 反解 `rooms.asm` 的门位/连通，或 L3 实录轨迹 |
| C-06 | Metroid 的**生成源**（哪张敌位表、哪些坐标） | 本区敌表只有属性；敌位数据在房间结构里，未反解 | 同 C-01 |
| C-07 | `EnemyMovementChoiceEntry` 的选择公式（`FrameCount eor RandomNumber1` + 掩码）对本区红/绿 Metroid 的实际影响 | 公式在引擎侧已读，但本区两项只有 `00/01` 单索引，随机性退化为常量；"是否真的退化为常量"未实测 | L1：注入不同 FrameCount/RandomNumber1，观察 movementIndex |
| C-08 | `L968B` 除 bit4 之外各位的语义 | 只有 bit4 被 Rust 侧消费（`framework.rs:426`），其余位无调用点证据 | 全仓 grep `L968B`/`$968B` 消费点 + Mesen 单步 |
| C-09 | Zebetite 死亡（`status` bit1）是否可被"出房再进"恢复 | `SpawnZebetite` 的已死分支只在 `CheckForItem` 命中时生效（=道具已取），与"打爆但未取道具"不同 | L1：`status=2` 后重跑 `SpawnZebetite` 观察 |
| C-10 | 门字节高 nibble（`$A0`/`(0<<4)`/`(1<<4)`）与 `BlueMusicChange` 的联动链 | `SpawnDoorRoutine` 在引擎件，未逐行 | 读 `07:` 侧 `SpawnDoorRoutine` + 音乐初始化标志 |
| C-11 | 结局面字（`EndMessage*PtrTable`）的具体文本与行数 | 数据在 bank0，未反解 | hexdump `00:` 侧指针表 + VRAM 串解码 |
| C-12 | 六门炮三屏排布的"左→右"顺序 | 坐标已解（§7.2），但房间内左右顺序取决于 nametable 与 ScrollDir | L3 实录或房间 tile 图 |

C 级共同点：**全部是"身份/几何/排布"，没有一条是数值常量**。所以 G-1..G-9 的实现不需要等它们；
它们只影响"验收时该期待什么画面"。

## 11. A 级结论登记（逐条可查，共 63 条）

| ID | 结论（一行） | 证据 |
|---|---|---|
| A-01 | 区例程每帧顺序：炮台→MB 状态机→计时→计时数字→Zebetite→Rinka 生成器 | `03:9B25 / :568` |
| A-02 | `ChooseEnemyAIRoutine` 16 项：0/1=Metroid、2=`L9A27`、3=删、4=Rinka、5-F=删 | `03:95E5 / :163` |
| A-03 | 敌属性 9 表均 16 项，起始 `$960B`，末表 `$977B` | `03:960B-03:977B / :184-412` |
| A-04 | `EnemyHealthTbl` = `FF,FF,01,FF,01,00…` | `03:962B / :202` |
| A-05 | `type=$02` → `lda #$01; jmp UpdateEnemyCommon_noMove` | `03:9A27 / :558` |
| A-06 | `L977B` bit7 置位才启用加速/初速系统；本区仅 type 0/1 | `03:977B / :395` |
| A-07 | 逃逸计时启动后 Metroid 自杀（`status=0`） | `metroid.asm:2-8` |
| A-08 | Metroid 限速用 `MetroidMaxSpeed[movementIndex]`，超限钳位并清 subpixel | `metroid.asm:40-90`、`$77F6` |
| A-09 | 非 Frozen 的 Metroid 每帧 `health=$FF`；Frozen 且 `health+1==0` → `health=5` | `metroid.asm:100-118` |
| A-10 | latch 位：低 nibble=附着帧计数、bit4-6=炸弹数、bit7=x 速符号 | `$77F8-$77FD`、`metroid.asm:216-260` |
| A-11 | 吸附入口：未 latch 且 `isHit&$04`（接触 Samus）→ `latch=(speedX&$80) or $01` | `metroid.asm:9932-9945` |
| A-12 | 附着偏移：Y 表 11 值、X=`$0C-帧计数`（按 bit7 取反） | `03:99D8 / :332` |
| A-13 | 六 latch OR 后 `and $0C == $0C` 且 `Health!=0` → `HealthChange=$0004`+`MetroidOnSamus=1`（每帧一次） | `metroid.asm:9967-999E` |
| A-14 | 5 次炸弹命中（`latch&$70==$50`）→ `status=Hurt`、`specialAttribs=$20`、`data0D=1`、反弹速 `$F8/$08` | `metroid.asm:9899-9930`、`03:9A10` |
| A-15 | Rinka 定速：`speed=dist>>4`、`subPixel=dist&$0F`（8.8 定点，一次性） | `03:9AF9 / :183` |
| A-16 | Rinka `ApplySpeedToPosition` 越界 → `status=0` 删除 | `rinka.asm:134-208` |
| A-17 | Rinka 生成 5 条件（计时未启/生成器在/nametable 交替/MB<4/`FrameCount&6==0`） | `03:A15E / :1862` |
| A-18 | 只在前 3 敌槽找空位 ⇒ 同屏上限 3 | `03:A188` |
| A-19 | `RinkaSpawnPosTbl` 12 B = `22,2A,2A,BA,B2,2A,C4,2A,C8,BA,BA,BA` | `03:A1DB / :1954` |
| A-20 | Rinka 出点解码 `y=(hi nibble*16) or 7`、`x=(lo nibble*16) or 7` | `03:9EE7 / :1314` |
| A-21 | 三个生成器的 `hi` 都取 `(PPUCTRL_ZP ^ ScrollDir) & 1` | `03:9D88 / :1036` |
| A-22 | Rinka 生成器优先槽 1，槽 1 占用才退槽 0；两槽都占则丢弃 | `03:9D6C / :1011` |
| A-23 | Zebetite 5 槽，槽号 = 特殊项高 nibble `>>1` | `03:A28B`、`03:9D3D` |
| A-24 | Zebetite 命中匹配：`Temp04 & $9E == roomRAMPtr` 且 `+1` 相等 | `03:A103-A13D` |
| A-25 | `qtyHits==8` → `status++`（死）；偶数击才更新外观 | `UpdateZebetite 03:A29B / :2091`（尾段） |
| A-26 | 外观 5 档串号 `0C/0D/0E/0F/07`（100/75/50/25%/dead） | `03:A310 / :2184` |
| A-27 | 未被击时 `healingDelay` 递减，到 0 → `qtyHits--` 并重绘（自愈 64 帧） | `03:A2EB-A309` |
| A-28 | 重绘失败 → 回滚 `status` bit1/`isHit`/`qtyHits`（下帧重试） | `03:A2DA` |
| A-29 | MB 11 态（0-`$0A`）与各态入口表 | `03:9DD4 / :1091` |
| A-30 | MB 碰撞框 `x $48-$76`、`y $61-$9F`，伤害 `HealthChange=$0200`（20 格）+`isHit=$38` | `03:9DF2 / :1110` |
| A-31 | MB 32 击死亡；平时闪 16 帧、死亡闪 128 帧 | `03:9FED / :1528` |
| A-32 | `Killed` 完成时清 12 个敌/弹槽、`status=4`、`FlashDelay=$28`、静音 | `03:9E52 / :1190` |
| A-33 | 消融 5 段（id 0..4），段间 28 帧、第 4 段后 14 帧 | `03:9E86 / :1230` |
| A-34 | 消融结束 → `music_Escape`、`status=5`、`TimeBombCounter=-$80` | `03:9ED6` |
| A-35 | `TIME BOMB` 8 段的指针表与串号表（`$6142+…` / `08,02,09,03,0A,04,0B,05`） | `03:9F39`、`03:9F41` |
| A-36 | 计时启动 = `EndTimer=$99/$99`（9999）+ 使能 + `EndTimerEnemyHi=MB.hi` | `03:9F49 / :1412` |
| A-37 | 门槽 = `rol(MapPosX+MapPosY)&3`，坐标 `x=$10,y=$68`，`DoorType=1`、`DoorHitPoints=1` | `03:9F69 / :1438` |
| A-38 | 爆炸 120 帧后 `Samus.status=sa_Dead2`、`status=$0A`、`Palette00` | `03:9FC0 / :1488` |
| A-39 | 计时 BCD 递减（低字节 -1、高字节借位） | `03:A1E7 / :1963`、`07:C3FB` |
| A-40 | 警报每 32 帧（`FrameCount&$1F==0`）请求 `OutOfPipe` | `03:A216` |
| A-41 | 计时数字 3 位，tile=`$A0+digit`，位置 `y=$84,x=$64`（借 MB 槽） | `03:A238 / :2021` |
| A-42 | 逃逸期三停摆：Rinka 生成器/Metroid/炮台（炮台只闪） | `03:1863`、`metroid.asm:2`、`03:9B5F` |
| A-43 | 玻璃命中（tile `$98`+导弹）→ `roomRAMPtr.lo=$8C`、`VRAMString01`（3×2） | `03:A0D3-A102` |
| A-44 | 导弹打 `$5E-$71` → `MotherBrainIsHit=1`（在爆瓦判定之前） | `03:A142 / :1834` |
| A-45 | Tourian 的爆瓦钩 `pla pla clc rts` 双返回，强制 carry=0（弹爆炸、瓦不破坏） | `03:A13E`、`07:E9C2` |
| A-46 | `CheckBlastTile`：Brinstar `$80-$97`，其余区 `$70-$97` | `07:D651 / prg7:4554` |
| A-47 | `SubtractHealth` 四守卫（`HealthChange==0`/已死/计时活/MB>=3） | `07:CE92 / prg7:3203` |
| A-48 | Varia 减半（`lsr` 两次链），否则高字节 `+$4F` 十位补偿 | 同上 |
| A-49 | 血量布局：十位=`Health+1&$0F`、个位=`Health>>4`、罐=`Health+1>>4`、低 nibble=借位域；开局 30 = `$00/$03` | `07:E0C1`、`crates/game-core/src/frame/tick.rs:533` |
| A-50 | `AreaEnemyDamage=$0300`（十位 3 ⇒ 30 格） | `03:95CE / :131` |
| A-51 | 电梯 `$8F` → `AtEnding=1`、`RoomPtr=0`、`BankSwitchPending=1`、`TitleRoutine=_id_EndGame` | `07:D8BF / prg7:4980` |
| A-52 | `ChooseEnding`：`AgeTable=$7A,$16,$0A,$04`，`EndingType 1..5`，`>=4` 无装甲 | `07:CAF5 / prg7:2439`、`07:CB18` |
| A-53 | 结局 8 项跳表（7 态 + `ExitSub`）由 `RoomPtr` 选择 | `00:9AA7 / prg0:3712` |
| A-54 | `EndSamusFlash`：每 32 帧 `ColorCntIndex++`，`==8` 调 `ChooseEnding`+`CalculatePassword`，`==$10` 进下一态（`Timer3=$10`） | `00:9B34 / prg0:3815` |
| A-55 | 只有 `EndingType>=4` 才挥手（`WaveSpritePointer=(EndingType-4)*4`） | `00:9B93 / prg0:3882` |
| A-56 | Staff 6 页，`ScrollY` 每 4 帧 +1，第 6 页到 `$88` 结束 | `00:9BFC / prg0:3972` |
| A-57 | 死亡链：`MB==$0A` → `inc MainRoutine`；否则 40 帧后 `GameOver` | `07:C95F / prg7:2055` |
| A-58 | `InitBank3` 把 14 B Metroid 初值抄到 `$77F0`（`F8,08,30,D0,60,A0,02,04` + 6×0），源表 `MetroidData` | `07:C590 / prg7:1449`、`MetroidData 07:C5A8 / prg7:1465` |
| A-59 | 桥 = 双像 bit7 AND 后 8 个 TileBlast（`routine=3`、`delay=y*2`、`type=4`） | `07:DADA / prg7:5416` |
| A-60 | 特殊项表全文（13 项 / 7 个 Y 段、5 座 Zebetite、6+6+6 门炮、8 个 Rinka 生成器） | `tourian/items.asm`、`03:A83B` |
| A-61 | 资产包已有 Tourian 房流/9 表/道具/精灵/Metroid 初值等键（枚举块 + 包内字符串键两层编号，勿混用） | `crates/game-data/src/pack.rs:95-280`、`:387-525` |
| A-62 | 结局链代码已被删除（`ending/sequence.rs 127 行` 删除，现存 `mod.rs` 仅 1 行注释） | `git show --stat 6aae8b32` |
| A-63 | `tick.rs:660` 读 `$687B` 当 `MotherBrainStatus` 是错址（真址 `$0098`） | `constants_ram.asm:708/1234` 反证 |

## 12. 复现命令集（全部只读）

```bash
# 1) 清单三组规模与状态
python3 -c "import json;d=json.load(open('data/status/routine_inventory.json'))['entries'];print({k:(len(v),sum(1 for e in v if e['status']=='ported')) for k,v in d.items() if k in ('prg3_tourian','metroid','rinka')})"

# 2) 敌表字节（16 项 × 9 表）
sed -n '184,412p' refs/m1disasm/SRC/prg3_tourian.asm | grep -c '^    \.byte'

# 3) ROM 数据面（须本地 rom/Metroid (USA).nes）
python3 -c "r=open('rom/Metroid (USA).nes','rb').read()[16:];o=lambda b,a,n:r[b*0x4000+a-0x8000:b*0x4000+a-0x8000+n];print(o(3,0x960B,16).hex(' '));print(o(3,0xA1DB,12).hex(' '))"

# 4) 双返回与爆瓦钩
sed -n '8595,8605p' refs/m1disasm/SRC/prg7_engine.asm
sed -n '1732,1740p;1826,1832p' refs/m1disasm/SRC/prg3_tourian.asm

# 5) 结局链现状（代码面 0 的反证）
wc -l crates/game-core/src/ending/mod.rs
git show --stat 6aae8b32 | grep -i ending

# 6) 既有定谳（结构性约束）
sed -n '1,40p' docs/findings/room-sweep.md
```

## 13. 遗留与交接

- 本文只做规格与缺口，**不含 diff、不 commit**（唯一新增文件即本文件）。
- §10 的 12 条 C 级假设全部需要"房间数据反解"或"L3 观测窗口"，建议合成**一张**取证卡
  （不要在实现卡里顺手猜），产物写 `docs/findings/`，再回填本文并升 A。
- §8 的 G-1..G-9 已按依赖排序；G-9 与其余无依赖关系，可立即单独提交。
- 三处最容易被误实现的地方（实现方必读）：双返回的 carry 语义（§5.4-1）、
  生成器 `hi` 取自 `PPUCTRL^ScrollDir`（§4.1）、Rinka 一次性定速（§3.4）。

