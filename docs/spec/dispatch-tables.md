# 跳转表分发语义（JumpEngine / LoadTableAt977B / EnemyMovementPtrs）

> 状态：B1 静态规格（本卡只写这一个文件）。
> 证据面：`refs/m1disasm/SRC/*.asm`（语义/命名，**线索**）+ `rom/Metroid (USA).nes`（字节，**真相**）+ `reference/seed/metroid.asm`（地址锚点）。
> 判据：凡是「ROM 在某个地址上是这些字节」的，A 级；凡是「这段代码意味着某个语义」但由命名/注释推定的，B 级；数据不足的写「未取证」。
> 复跑：本文所有 hexdump 都有 §6 的一条命令，`rom/` 只读。

## 0. 结论速览

| # | 表 | 位置 | 项宽 | 项数 | 索引来源 | 越界行为 |
|---|---|---|---|---|---|---|
| 1 | `JumpEngine` 的分发表（**不是一张表**：23 处字面调用、27 个机器副本，每个调用点后面各自跟一张） | 每个 `jsr` 的**下一条指令起**（`jsr $C27C` 或 `jsr CommonJump_JumpEngine`） | 2 B（小端 16 位地址） | 每处不同（4–28 项） | A（调用前由 `lda <状态字节>` 装入） | **无上界检查**：`asl` 只保留低 7 位，索引 ≥128 会回绕；超表尾会读表后的字节当地址 |
| 2 | `L977B`（敌人类型 → 字节属性表，**按区域 5 份**） | `01–05:977B` | 1 B | 16 | `EnsExtra.0.type`（每槽第 14 字节） | **无上界检查**：type≥16 会读进 `978B`（`EnProjectileRisingAnimIndexTable`） |
| 4 | 敌人 AI 分派表 `ChooseEnemyAIRoutine`（区域 5 份 + 母脑 1 份） | `01–05:95EB`（母脑 `03:9DDB`） | 2 B | 16（母脑 6） | `EnsExtra.0.type` / `MotherBrainStatus` | 无上界检查；bank1 的 type11–15 指向 `$0000` |
| 3 | `EnemyMovementPtrs`（移动串指针，**按区域 5 份**） | `01–05:96DB` | 2 B | 40 项（20 对，索引 0x00–0x13） | `((movementIndex×2)+朝向)×2` | **无上界检查**：`movementIndex≥18` 取到 `$0000`，随后被当零页指针用 |

三张表的共同点：**ROM 不做任何边界检查**，判断全在调用方（例如 `UpdateEnemy` 在跳转前比 `enemyStatus_Hurt+1`）。这一点是移植时最容易漏掉的「原版 quirk」，Stage A 必须照搬。

## 1. `JumpEngine`：从栈上取表址的间接跳转

### 1.1 代码（A 级，`07:C27C`，30 字节）

```
07:C27C  0A        ASL A
07:C27D  84 6A     STY TempY
07:C27F  86 69     STX TempX
07:C281  A8        TAY
07:C282  C8        INY
07:C283  68        PLA
07:C284  85 0C     STA JumpEngineRoutinePtr
07:C286  68        PLA
07:C287  85 0D     STA JumpEngineRoutinePtr+1
07:C289  B1 0C     LDA (JumpEngineRoutinePtr),Y
07:C28B  AA        TAX
07:C28C  C8        INY
07:C28D  B1 0C     LDA (JumpEngineRoutinePtr),Y
07:C28F  85 0D     STA JumpEngineRoutinePtr+1
07:C291  86 0C     STX JumpEngineRoutinePtr
07:C293  A6 69     LDX TempX
07:C295  A4 6A     LDY TempY
07:C297  6C 0C 00  JMP (JumpEngineRoutinePtr)
```

字节来源：`rom/` 07 银行 `$C27C` 起 30 字节 =
`0a 84 6a 86 69 a8 c8 68 85 0c 68 85 0d b1 0c aa c8 b1 0c 85 0d 86 0c a6 69 a4 6a 6c 0c 00`。

### 1.2 逐行语义（A 级：由字节与 6502 行为直接可得）

1. `ASL A`：索引 ×2（每项 2 字节）；**位 7 被移出丢弃** ⇒ A=$80 时索引变成 0（回绕），A=$C0 ⇒ 表项 0x40。
2. 保存 X/Y 到 `TempX/TempY`（`$69/$6A`），因为下面要用栈和 Y。
3. `PLA/PLA`：把 `jsr` 压入的返回地址**弹出来**，作为表基址。⇒ **表必须紧跟在 `jsr JumpEngine` 之后**（3 字节指令 + 表起址 = 返回地址）；中间插任何指令都会读到错位的「表」。
4. `LDA (ptr),Y` / `INY` / 再取高字节 ⇒ 组成 16 位目标地址，写回 `JumpEngineRoutinePtr`（`$0C/$0D`，见 `constants_ram.asm:479-480`）。
5. `JMP (JumpEngineRoutinePtr)`：**不再压返回地址** ⇒ 被调例程的 `rts` 直接返回到 `jsr JumpEngine` 的调用者（表项常写 `ExitSub`/`$C45C`＝纯 `rts`，即「本状态什么都不做」）。
6. 栈契约的副作用：`JumpEngine` 内部 `PLA` 两次，所以**它不能被 `jmp` 调用**（栈顶不是返回地址）；唯一的例外是 `CommonJump_JumpEngine`（`01–05:8024`）里的 `jmp JumpEngine` —— 那是「调用者已经 `jsr CommonJump_JumpEngine`」的场景，等价于 `jsr $C27C`（栈上是 `CommonJump` 的返回地址，紧跟在 `jmp` 后面的表由 `CommonJump_JumpEngine` 之后的字节提供，例如门表的 `$8B8F`）。

### 1.3 调用点清单（字面 23 处；机器副本 27 个）

字面出现分两种形式，共 **23 处**：
- 直接 `jsr/jmp JumpEngine`：**17 处**（`prg0_title.asm` 2、`areas_common.asm` 2、`prg7_engine.asm` 13）——`porting-backlog.md` 记的 17 就是这个口径。
- 经包装 `jsr CommonJump_JumpEngine`（`01–05:8024` 是 `jmp JumpEngine`）：**6 处**（`prg1_brinstar.asm:158`、`prg2_norfair.asm:156`、`prg3_tourian.asm:165`、`prg3_tourian.asm:1094`、`prg4_kraid.asm:162`、`prg5_ridley.asm:163`）。这 6 处同样满足「表紧跟 jsr」的栈契约：表在**区域 bank 里该 `jsr` 的下一条指令**，不是 `$8024` 之后。
- 机器副本 = 直接 jsr 的 21 个（bank0 2 + 门表 bank1–6 共 6 + bank7 13）+ 包装调用的 6 个 = **27 个**（`$8024` 本身只是 wrapper 的 `jmp`，不算调用点）。

| 调用点（文件:行） | 机器地址 | 形式 | 表地址 | 项数 | 索引来源（A） |
|---|---|---|---|---|---|
| `prg0_title.asm:55` | `00:8027` | jsr | `00:802A` | 28 | `TitleRoutine`（标题/片头状态机） |
| `prg0_title.asm:3732` | `00:9AC2` | jsr | `00:9AC5` | 8 | `RoomPtr`（结局例程编号） |
| `areas_common.asm:61` | `01–05:8024` | **jmp（wrapper）** | 由各调用者的返回地址决定 | — | 无索引（透传） |
| `areas_common.asm:1429` | `01–06:8B8C` | jsr | `01–06:8B8F` | 7 | `Objects.0.status`（门） |
| `prg1_brinstar.asm:158` | `01:95E8` | jsr 包装 | `01:95EB` | 16 | `EnsExtra.0.type`（敌人 AI 分派） |
| `prg2_norfair.asm:156` | `02:95E8` | jsr 包装 | `02:95EB` | 16 | 同上 |
| `prg3_tourian.asm:165` | `03:95E8` | jsr 包装 | `03:95EB` | 16 | 同上 |
| `prg3_tourian.asm:1094` | `03:9DD8` | jsr 包装 | `03:9DDB` | 6 | `MotherBrainStatus`（零页 `$98`） |
| `prg4_kraid.asm:162` | `04:95E8` | jsr 包装 | `04:95EB` | 16 | `EnsExtra.0.type` |
| `prg5_ridley.asm:163` | `05:95E8` | jsr 包装 | `05:95EB` | 16 | 同上 |
| `prg7_engine.asm:346` | `07:C13E` | jsr | `07:C141` | 10 | `MainRoutine` |
| `prg7_engine.asm:2721` | `07:CC1F` | jsr | `07:CC22` | 10 | `Samus.status`（横向/通用） |
| `prg7_engine.asm:2796` | `07:CC7A` | jsr | `07:CC7D` | 5 | `Samus.status`（`< sa_Door` 分支后） |
| `prg7_engine.asm:3789` | `07:D1D4` | jsr | `07:D1D7` | 10 | `Samus.status`（输入处理尾部） |
| `prg7_engine.asm:4284` | `07:D4D0` | jsr | `07:D4D3` | 12 | `Objects.0.status`（子弹/炸弹） |
| `prg7_engine.asm:4810` | `07:D7BA` | jsr | `07:D7BD` | 10 | `Elevator.status`（电梯） |
| `prg7_engine.asm:8676` | `07:EA19` | jsr | `07:EA1C` | 5 | `RoomNumber & $0F`（属性表按行写） |
| `prg7_engine.asm:8865` | `07:EAE1` | jsr | `07:EAE4` | 8 | `RoomNumber & $0F`（载房对象分派） |
| `prg7_engine.asm:9500` | `07:EDDF` | jsr | `07:EDE2` | 11 | `(SpecItmsTblPtr),y & $0F`（特殊道具/出生点） |
| `prg7_engine.asm:10688` | `07:F36B` | jsr | `07:F36E` | 7 | `EnsExtra.0.status`（敌人状态机） |
| `prg7_engine.asm:11896` | `07:F95A` | jsr | `07:F95D` | 6 | `EnsExtra.0.status`（敌人发射物） |
| `prg7_engine.asm:12485` | `07:FC9A` | jsr | `07:FC9D` | 4 | `Mellows.0.status` |
| `prg7_engine.asm:12844` | `07:FE2E` | jsr | `07:FE31` | 6 | `TileBlasts.0.routine` |

`07:D1D4` 这一处在 `reference/seed/metroid.asm` 的线性扫描里被并进了前一条指令流，`grep "20 7C C2"` 抓不到它；本卡用表签名 `55 cf 98 cc 5c c4 b5 d0` 在 ROM 里定位：`rom.find(...)` 命中的表址 `07:D1D7`，减去 3 即调用点（§6 第 5 条命令）。

`prg7_engine.asm:10688` 的边界检查（A 级，`07:F365` 附近）：
```
lda EnsExtra.0.status,x
sta EnemyStatusPreAI
cmp #enemyStatus_Hurt+1
bcs @invalidStatus        ; status ≥ 7 → jmp RemoveEnemy($FA18)
jsr JumpEngine            ; 0..6 才进表
```
这是全仓唯一「跳转表前自带上限检查」的调用点；其它 16 处都没有。

### 1.4 六张代表性表的逐项解码（A 级：字节→小端 16 位地址）

**(a) `TitleRoutinePtrTable`（`00:802A`，28 项）** — 首 16 项 + 尾 4 项（完整 28 项见 §6 命令）：

```
hex 71 80 d0 80 f9 80 2c 81 42 81 09 81 4d 81 63 81 82 81 d1 81 6e 80 6e 80 2e 82 43 82 63 82 83 82 68 80 a3 82 ed 82 f3 82 f3 82 ba 90 d7 90 1a 91 47 91 59 93 94 93 ba 90
dw  8071 80d0 80f9 812c 8142 8109 814d 8163 8182 81d1 806e 806e 822e 8243 8263 8283 8068 82a3 82ed 82f3 82f3 90ba 90d7 911a 9147 9359 9394 90ba
```
首 16 项：`InitializeAfterReset $8071`、`DrawIntroBackground $80D0`、`FadeInDelay $80F9`、`METROIDFadeIn $812C`、`LoadFlashTimer $8142`、`FlashEffect $8109`、`METROIDSparkle $814D`、`METROIDFadeOut $8163`、`Crosshairs $8182`、`MoreCrosshairs $81D1`、`IncTitleRoutine0A $806E`、`IncTitleRoutine0B $806E`、`ChangeIntroNameTable $822E`、`MessageFadeIn $8243`、`MessageFadeOut $8263`、`DelayIntroReplay $8283`。
尾 4 项（索引 24–27）：`EnterPassword $9147`、`DisplayPassword $9359`、`WaitForSTART $9394`、`StartContinueScreen1B $90BA`（索引 21 也是 `$90BA`，原版复用同一例程；`LoadPasswordScreen $911A` 在索引 23）。

**(b) `MainRoutinePtrTable`（`07:C141`，10 项）**

```
hex 01 c8 1d c8 d1 c8 2b c9 a6 c9 b1 c9 c4 c9 55 c1 d7 c9 94 c4
dw  c801 c81d c8d1 c92b c9a6 c9b1 c9c4 c155 c9d7 c494
```
`AreaInit $C801`、`MoreInit $C81D`、`SamusInit $C8D1`、`GameEngine $C92B`、`PrepareGameOver $C9A6`、`PauseMode $C9B1`、`GoPassword $C9C4`、`IncrementRoutine $C155`（只把 `MainRoutine` +1）、`SamusIntro $C9D7`、`WaitTimer $C494`。

**(c) Samus 状态表（`07:CC22`，10 项）**

```
hex 36 cc c2 cc 02 d0 e1 d0 98 d1 a8 d3 02 d0 1a d4 1f d4 23 d4
dw  cc36 ccc2 d002 d0e1 d198 d3a8 d002 d41a d41f d423
```
`SamusStand $CC36`、`SamusRun $CCC2`、`SamusJump $D002`、`SamusRoll $D0E1`、`SamusPntUp $D198`、`SamusDoor $D3A8`、`SamusJump(朝上) $D002`、`SamusDead $D41A`、`SamusDead2 $D41F`、`SamusElevator $D423`。

**(d) 敌人状态表（`07:F36E`，7 项）**

```
hex 5c c4 be f3 e6 f3 0d f4 3e f4 83 f4 ee f4
dw  c45c f3be f3e6 f40d f43e f483 f4ee
```
`0 ExitSub $C45C`、`1 UpdateEnemy_Resting $F3BE`、`2 UpdateEnemy_Active $F3E6`、`3 UpdateEnemy_Explode $F40D`、`4 UpdateEnemy_Frozen $F43E`、`5 UpdateEnemy_Pickup $F483`、`6 UpdateEnemy_Hurt $F4EE`。

**(e) 门状态表（`01–06:8B8F`，7 项，bank1 副本）**

```
hex 5c c4 9d 8b d5 8b 01 8c 84 8c c6 8c f0 8c
dw  c45c 8b9d 8bd5 8c01 8c84 8cc6 8cf0
```
`0 ExitSub`、`1 UpdateDoor_Init $8B9D`、`2 UpdateDoor_Closed $8BD5`、`3 UpdateDoor_Open $8C01`、`4 UpdateDoor_LetSamusIn $8C84`、`5 UpdateDoor_Scroll $8CC6`、`6 UpdateDoor_LetSamusOut $8CF0`。

**(f) 载房对象分派表（`07:EAE4`，8 项）**

```
hex 5c c4 06 eb 8c eb 5c c4 04 ec 5c c4 2f ec 57 ec
dw  c45c eb06 eb8c c45c ec04 c45c ec2f ec57
```
`0 ExitSub`、`1 LoadEnemy $EB06`、`2 LoadDoor $EB8C`、`3 ExitSub`、`4 LoadElevator $EC04`、`5 ExitSub`、`6 LoadStatues $EC2F`、`7 LoadPipeBugHole $EC57`。

## 2. `L977B` + `LoadTableAt977B`：敌人类型 → 属性字节

### 2.1 例程（A 级，`01–05:80B0`，8 字节）

```
80B0  BC 02 6B     LDY EnsExtra.0.type,X    ; EnsExtra.0 = $6AF4 起，type = +14 → $6B02,X
80B3  B9 7B 97     LDA L977B,Y              ; 区域表基址 $977B
80B6  0A           ASL A                    ; 返回 A = 字节×2（调用方据此判 bit6，或当偏移）
80B7  60           RTS
```
语义：**输入 X=敌人槽基址，输出 Y=类型、A=表值×2**，不改 X/P。返回的 `A` 有双重用途：
1. `bpl` 判 bit6（`A` 的 N 位 = 表字节 bit6）：0 ⇒ 用移动串（movement strings），1 ⇒ 用加速度+速度（例：`areas_common.asm:212-215` 的 `EnemyIfMoveFailedUp`）。
2. 有调用方把 `A` 当**字节偏移**再用：`areas_common.asm:428-431` 注释「2 lsr's to compensate for the asl」，即先 `lsr` 两次回到表字节，再按位取用。

### 2.2 五份表体（A 级；`01–05:977B`，各 16 B）

| bank | hex | sha256 前 12 |
|---|---|---|
| 01 Brinstar | `64 6c 21 01 04 00 4c 40 04 00 00 40 40 00 00 00` | `be5a484bf790` |
| 02 Norfair | `4c 4c 01 00 00 00 00 40 00 64 44 44 40 00 00 00` | `d1146b1aa69e` |
| 03 Tourian | `50 50 00 00 00 00 00 00 00 00 00 00 00 00 00 00` | `3cc664a795b4` |
| 04 Kraid | `64 6c 21 01 04 00 4c 40 04 00 00 40 40 00 00 00` | `be5a484bf790`（与 bank1 相同） |
| 05 Ridley | `4c 4c 64 6c 00 00 00 40 00 64 44 44 40 00 00 00` | `bae3a60d1e98` |

**bit 含义（B 级：来自 `areas_common.asm:176-183` 的注释；本文只核实了 bit6/bit1/bit0 在代码里被这样测试）**

| bit | 含义 | 代码证据 |
|---|---|---|
| 7 | 管道虫（pipe bug）相关标志 | 注释；未在本卡追到使用点 ⇒ 未取证 |
| 6 | `0`=用移动串，`1`=用加速度+速度+子像素 | A：`80B6 ASL A` 后 `bpl`（`areas_common.asm:213`） |
| 4 | `1`=Metroid | 注释；使用点未取证 |
| 3–2 | 静止敌人的 `data1F` bit7–6 初值 | 注释（B） |
| 1 | `EnemyIfMoveFailedUp/Down` 里翻转 `data05` bit2 | A：`areas_common.asm:241-245`（`lsr lsr bcc`） |
| 0 | `EnemyIfMoveFailedLeft/Right` 里翻转 `data05` bit0 | A：`areas_common.asm:363-367` |

### 2.3 越界行为（A 级）

`LDA L977B,Y` 没有任何上限判断。表后紧邻 `978B EnProjectileRisingAnimIndexTable`（16 B：`00 00 64 67 69 69 00 00 00 00 00 00 00 00 00 00`）⇒ `type=16` 读到 `$00`（A=0，按「无 bit6」处理），`type=17` 也 `$00`，`type=18` 读 `$64`（A=$C8 ⇒ 按 bit6=1 当「用加速度」）。**这是可观察行为**，Stage A 若要完全一致必须照搬同一读取窗口，而不是把 type 钳到 15。

### 2.4 调用点（字面 18 处：`areas_common.asm` 9 + `prg7_engine.asm` 9）

`areas_common.asm`：212（`EnemyIfMoveFailedUp`）、278（`EnemyIfMoveFailedDown`）、328（`EnemyIfMoveFailedLeft`）、388（`EnemyIfMoveFailedRight`）、431（同上，带两次 `lsr` 补偿）、490（`EnemyGetDeltaX`）、537（`EnemyGetDeltaY`）、581（`EnemyReactToSamusProjectile` 附近）、805（`EnemyGetMovementPtr` 之后的取速度段）。
`prg7_engine.asm`：5966（`UpdateEnemy_UpdateEnData05Bit6`）、10991/11066/11080/11124/11148（`UpdateEnemy` 的移动/被击退/碰撞分支）、11274（`EnemyFlipAfterDisplacement` 所在文件的取符号段）、11630（`EnemyMove`）、12260（同上）。
全部是同一条读法：`X=敌人槽 → jsr $80B0 → 用 Y/A`；**没有调用方传类型号**，所以类型只能来自 `$6B02,X`。

### 2.5 敌人 AI 分派表（第四张表族：`ChooseEnemyAIRoutine`，按区域 5 份 + 母脑 1 份）

`ChooseEnemyAIRoutine`（`01–05:95E5`，每份 0x26=38 字节）：
```
01:95E5  BD 02 6B     LDA EnsExtra.0.type,X
01:95E8  20 24 80     JSR CommonJump_JumpEngine   ; → jmp $C27C，表就在下一条指令
01:95EB  <16 个 .word>
```
索引 = **敌人类型** `EnsExtra.0.type`（0–15）⇒ **无边界检查**，type≥16 会读到表后的代码字节当地址。

| bank | 16 项（地址） |
|---|---|
| 01 Brinstar | `99B8 99D3 99E5 99D8 99FA 9A4C 9AF5 9B32 9BA2 9BD2 9C1A 0000 0000 0000 0000 0000` |
| 02 Norfair | `98D3 9908 98C0 9833 9833 9833 9996 9850 9833 9833 9833 9A64 9AD6 9AE9 9B64 9833` |
| 03 Tourian | `97F9 97F9 9A27 97DC 9A2C 97DC 97DC 97DC 97DC 97DC 97DC 97DC 97DC 97DC 97DC 97DC` |
| 04 Kraid | `991C 9937 95CB 993C 9949 999B 95CB 9A44 9AB4 9AE4 9B2C 95CB 95CB 95CB 95CB 95CB` |
| 05 Ridley | `98D7 990C 9847 9862 982A 982A 9967 9867 982A 9A13 9A4A 982A 9B03 982A 9B16 982A` |

- bank5 的符号（`M1_NES_NTSC.sym`）证实：项2=`SidehopperFloorAIRoutine_BANK5`、项3=`SidehopperCeilingAIRoutine_BANK5`、项6=`CrawlerAIRoutine_BANK5`、项7=`PipeBugAIRoutine_BANK5` ⇒ 这 4 项的名字是 A 级，其余项只有地址（A）但名称未取证（B/未取证）。
- **`0000` 是真实表项**：bank1 的类型 11–15 会跳到 `$0000`（CPU 读 `$0000..` 的 BRK 向量区，实际行为取决该区域当前 RAM）；这是原版 quirk，移植时不要「补一个安全默认」。
- 母脑分派（`03:9DD4 MotherBrainStatusHandler`，索引 `MotherBrainStatus`=`$98`，`03:9DD8` 调包装，表 `03:9DDB`，6 项）：`95CB 9E22 9E36 9E52 9E86 9F02`（Exit / Idle / Hurt / Killed / Disappear / TimeBombMessage，名称为 B 级，来自 asm 注释）。

## 3. `EnemyMovementPtrs`：移动串指针表（按区域 5 份）

### 3.1 消费点（A 级）

`areas_common.asm:555` `LoadEnemyMovementPtr`（`$822B`）：
```
lda Ens.0.data05,x      ; bit7=竖向朝向
bpl @horiz
  lsr
  lsr
@horiz:
lsr                     ; 朝向位进 carry
lda Ens.0.movementIndex,x
rol                     ; carry 进 bit0
asl                     ; ×2（项宽）
tay
lda EnemyMovementPtrs,y
sta EnemyMovementPtr    ; $81
lda EnemyMovementPtrs+1,y
sta EnemyMovementPtr+1  ; $82
```
即索引 = `(movementIndex×2 + 朝向) × 2`，**movementIndex 0–19**、朝向 0/1 ⇒ y=0–79。表被7处字面引用（5 份区域定义 + `areas_common.asm:571/573` 两行取指）；`EnemyMovementPtr`（`$81/$82`）在 SRC 里另有11处使用，是本表的下游。

### 3.2 bank1 表体（A 级，`01:96DB`，40 项 = 80 B）

```
hex ef 97 f2 97 f5 97 f5 97 f5 97 f5 97 f5 97 f5 97 f5 97 f5 97 f5 97 40 98 8b 98 8e 98 91 98 a5 98 b9 98 b9 98 b9 98 b9 98 b9 98 b9 98 b9 98 b9 98 b9 98 c0 98 c7 98 ce 98 d5 98 d8 98 db 98 f2 98 09 99 20 99 37 99 4e 99 00 00 00 00 00 00 00 00
```
按 (movementIndex, R/L) 解码（地址为 `01:xxxx`）：

| idx | R | L |
|---|---|---|
| 00 | `97EF` | `97F2` |
| 01 | `97F5` | `97F5` |
| 02 | `97F5` | `97F5` |
| 03 | `97F5` | `97F5` |
| 04 | `97F5` | `97F5` |
| 05 | `97F5` | `9840` |
| 06 | `988B` | `988E` |
| 07 | `9891` | `98A5` |
| 08 | `98B9` | `98B9` |
| 09 | `98B9` | `98B9` |
| 0A | `98B9` | `98B9` |
| 0B | `98B9` | `98B9` |
| 0C | `98B9` | `98C0` |
| 0D | `98C7` | `98CE` |
| 0E | `98D5` | `98D8` |
| 0F | `98DB` | `98F2` |
| 10 | `9909` | `9920` |
| 11 | `9937` | `994E` |
| 12–13 | `0000` | `0000` |
| 0x14+ | 超出 80 B 表体 | 超出 80 B 表体 |

上表只到索引 `0x13`（20 对 = 80 B）。`0x14` 起已越过表尾：`LoadEnemyMovementPtr` 不做上界检查，会读到紧随其后的数据；**具体读到的地址本卡未取证**（需要看 `01:972B` 之后是什么，本卡没有追）。

`0000` 是**真实表项**，不是未初始化：movementIndex 为 0x12/0x13（十进制 18/19）时 `EnemyMovementPtr=$0000`，调用方的 `lda (EnemyMovementPtr),y` 会读零页 `$0000..`（原版 quirk；是否触发取决于敌人数据，本卡未枚举全部敌人 ⇒ 触发面未取证）。

### 3.3 区域差异

5 份表的 sha256 前 12：bank1 `de1651a6bad0`、bank2 `de94d595f26f`、bank3 `d533d37bfb1c`、bank4 `812844014dd7`、bank5 `ecce3ab0f01b`。**bank5 的多数项是别名**（`03–17` 全部指向 `05:97F3`），因为 linker 把相同的移动串合并；bank1 则保留各自地址。⇒ 移植时不能假设「同一 movementIndex 在不同区域指向不同内容的串」，只能按区域各自存一份（与 ROM 布局一致），或按「串内容」去重但要保留别名关系。

## 4. 与 `porting-backlog.md` 的 B1 对齐

| 条目 | backlog 编号 | 本卡覆盖 | 迁移状态 |
|---|---|---|---|
| `JumpEngine` `07:C27C`（23 处字面 = 17 直接 + 6 包装） | 批 2 #13 | 机制 + 27 个机器副本 + 6 张代表表逐项解码 | 本卡给规格；`crates/` 里尚未有对应实现（未验证） |
| `LoadTableAt977B` `01–05:80B0`（18 xref） | 批 2 #12 | 例程 + 5 份表体 + 18 调用点 + 越界窗口 | 同上 |
| `EnemyMovementPtrs`（7 处引用，第三张表） | 未单列（与 `EnemyMove` 同族） | 40 项全解码 + 索引公式 + 别名/空指针 | 同上 |
| `ChooseEnemyAIRoutine` 敌人 AI 分派（6 处调用，第四张表族） | 未单列（与敌人 AI 同族） | 5 份 ×16 项 + 母脑 6 项全解码，`0000` quirk | 同上 |

剩余未覆盖（下一张卡建议）：
1. **表驱动的其它同类**：`EnProjectileMovementPtrTable`（8 处）、`VRAMStringPtrTable`、`L968B`（bit5 判定，18 处）——三者都是「按类型取指针/标志」的同构结构，可套本卡的取证模板。
2. **`JumpEngine` 的 stage-A 移植**：需要把「表紧跟 jsr」的栈契约显式化（Rust 里是 `match` 而非间接取址），并保留「索引 ≥128 回绕」「超表尾读后续字节」两条 quirk；`UpdateEnemy` 的 `cmp #7` 是唯一已有护栏。
3. **`L977B` 的 bit7/bit4 使用点**未追（标未取证）；`EnemyMovementPtrs` 的空指针触发面未枚举。

## 5. 分级清单

**A 级（字节级可复现）**
- 三张表的基址/项宽/项数/全部表项字节（§1.4、§2.2、§3.2、§3.3）；JumpEngine 的 27 个机器副本地址（§1.3）。
- `JumpEngine` 30 字节与逐行语义、栈契约、`ASL` 回绕（§1.1–1.2）。
- `LoadTableAt977B` 8 字节、`A=byte×2`、越界读进 `978B`（§2.1、§2.3）。
- 调用点地址与索引来源（§1.3、§2.4、§3.1）。
- `EnemyMovementPtrs` 索引公式与 `0000` 表项（§3.1–3.2）。

**B 级（由注释/命名推定，未经运行验证）**
- `L977B` 的 bit7 / bit4 / bit3–2 语义（bit6/bit1/bit0 已由代码测试核实，见 §2.2 表）。
- 「27 个机器副本」中 bank6 门表副本的实际执行路径（bank6 是图形 bank，本卡未追谁会切到它）。

**未取证**
- `L977B` bit7/bit4 的使用点、`EnemyMovementPtrs` 空指针的实际触发条件、`JumpEngine` 超表尾在实际游戏里是否可被触达（调用方都先夹了状态）。
- `VRAMStringPtrTable`、`L968B`、`EnProjectileMovementPtrTable` 的具体语义（本卡只列了引用数）。

## 6. 复跑命令

```bash
# 1) 三张表的原始字节（需要 rom/Metroid (USA).nes）
python3 - <<'EOF'
rom=open('rom/Metroid (USA).nes','rb').read()
def off(b,a): return 16+b*0x4000+((a-0xC000) if a>=0xC000 else (a-0x8000))
def rb(b,a,n): return rom[off(b,a):off(b,a)+n]
print('JumpEngine 07:C27C', rb(7,0xC27C,30).hex(' '))
print('Loader 01:80B0', rb(1,0x80B0,8).hex(' '))
for b in range(1,6):
    print(f'L977B {b:02X}:977B', rb(b,0x977B,16).hex(' '))
print('MovePtrs 01:96DB', rb(1,0x96DB,80).hex(' '))
EOF

# 2) 任一张 JumpEngine 表：表址 = 调用指令地址 + 3，项数见 §1.3
python3 - <<'EOF'
rom=open('rom/Metroid (USA).nes','rb').read()
def off(b,a): return 16+b*0x4000+((a-0xC000) if a>=0xC000 else (a-0x8000))
def words(b,a,n):
    bs=rom[off(b,a):off(b,a)+2*n]
    return [bs[i]|(bs[i+1]<<8) for i in range(0,2*n,2)]
for name,b,a,n in [('Title 00:802A',0,0x802A,28),('Main 07:C141',7,0xC141,10),('EnemyStatus 07:F36E',7,0xF36E,7)]:
    print(name, ' '.join(f'{w:04X}' for w in words(b,a,n)))
EOF

# 3) 调用点（字面 xref）
grep -rn "jsr JumpEngine\|jmp JumpEngine\|jsr CommonJump_JumpEngine\|jsr LoadTableAt977B" refs/m1disasm/SRC | wc -l   # 期望 23 + 18 = 41

# 4) 机器地址：直接 jsr/jmp $C27C 的 21 处在 seed 里可见（grep 到 25 行是因为含 $8024 包装体与 12 处 bank7）
grep -n "7C C2" reference/seed/metroid.asm | wc -l

# 5) seed 线性扫描漏掉的那一处（07:D1D4 → 表 07:D1D7），用表签名定位
python3 -c "rom=open('rom/Metroid (USA).nes','rb').read(); sig=bytes.fromhex('55cf98cc5cc4b5d0'); base=16+7*0x4000; i=rom.find(sig); print(hex(i), hex(i-base+0xC000), hex(i-base+0xC000-3))"
```

> 注：`grep "7C C2"` 数到 25 行 = bank0 的 2 处 + 门表 bank1–6 的 6 处 + `$8024` wrapper 体的 bank1–5 5 处 + bank7 的 12 处；bank7 第 13 处 `07:D1D4` 被 seed 线性扫描并进了前一条指令流，所以少一行。6 处 `jsr $8024`（包装调用）不在这个 grep 里 —— 它们的目标是 wrapper，表在各自区域 bank（§1.3）。
