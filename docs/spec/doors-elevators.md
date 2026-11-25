# 门与升降台：数据表与转移链（B5-b 规格）

> 只读考证：ROM 字节（`rom/Metroid (USA).nes`，sha256 `e6e6b701…65b6`）+ 社区匹配反汇编（`refs/m1disasm/SRC/`）
> + 现实现（`crates/game-core/src/world/`）。**本文件不改任何代码。**
> 证据级别：**A** = 字节/汇编行逐条一致；**B** = 代码语境推断；**C** = 未取证（§6）。
> 接 #59 `docs/spec/room-data-format.md`：房间记录对象段的类型码 `2`=门、`4`=升降台（`DrawRoom ; 07:EAAA` 的 JumpEngine 表项）。

## 0. 一句话结论

房间记录里**没有「目标房间」和「目标门索引」字段**——门记录只有 2 字节（`B0` 槽号+类型、`B1` 侧+门型），
目标房间是**走出画面一格后由 `WorldMap` 查回来的**（`ScrollLeft/Right` 里 `MapPosX±1 → GetRoomNum`，A 级，§2）。
升降台在房间记录里是 2 字节（`B0`、`B1`=方向+目的区），但**原版 5 区房间数据里 0 处 `re_Elevator` 记录**；
真正常用的是**特殊物品表 type=4 的 11 条电梯条目**（`it_Elevator, param`，§3.2）。

## 1. 门数据表

### 1.1 记录形状与字段（A 级）

门记录 = 房间对象段里的 `B0` + 1 字节 `B1`（共 2 B；敌记录是 3 B，见 #59 §1.2）。

| 字节 | 位 | 含义 | 级别 | 证据 |
|---|---|---|---|---|
| `B0` | 高半字节 | 目标门对象槽号（写到 `Doors.n`；4 个门槽固定用 0/1/2/3 号槽） | A | `GetEnemyData ; 07:EB0C`（`and #$F0; tax`）+ `DoorSlots` |
| `B0` | 低半字节 | 类型码 `2`（门） | A | `DrawRoom` JumpEngine 表（`07:EAD6`）：`1 LoadEnemy / 2 LoadDoor / 4 LoadElevator / 6 LoadStatues / 7 LoadPipeBugHole`，0/3/5 = `ExitSub` |
| `B1` | bit4 | 门侧：`0`=右、`1`=左 | A | `SpawnDoorRoutine`：`jsr Amul16`（×16 后进位=原 bit4）→ `php` 存进位 |
| `B1` | bit1–0 | 门型：`0` 红 / `1` 蓝 / `2` 十发导弹 / `3` 蓝+换音乐 | A | `pla; and #$03; sta DoorType,x`；枚举 `constants.asm:50-53` |
| `B1` | bit7、bit5 | 恒为 `1`（数据里 `$A0`/`$B0` 基址），代码**不读** | A（负面） | `SpawnDoorRoutine` 只取 bit4 与 bit1–0；`rooms.asm` 全都写 `$A0 \| (side<<4) \| doorType_*` |
| 目标房间 | — | **不存在**；由 WorldMap 步进决定 | A（负面） | §2.4 |
| 屏幕位置 | — | 右门 `x=$F0`、左门 `x=$10`、`y` 恒 `$68` | A | `DoorXs 07:EBFC` / `lda #$68; sta y` |
| 名表位 | — | `hi=(PPUCTRL^ScrollDir)&1` | A | `GetNameTableAtScrollDir 07:EB85` |
| 滚动封条 | — | 右门封 `$02`、左门封 `$01` 到「另一张名表」的 `ScrollBlockOnNameTable3` | A | `DoorScrollBlocks 07:EBFE` + `eor #$01` 取反后按名表 OR |

**门槽选择表**（`DoorSlots 07:EC00`，A 级字节）

| 索引 | 含义 | 槽 |
|---|---|---|
| 0 | 白格（`(MapPosX+MapPosY)` 偶）+ 右门 | `Doors.0` |
| 1 | 白格 + 左门 | `Doors.3` |
| 2 | 黑格 + 右门 | `Doors.2` |
| 3 | 黑格 + 左门 | `Doors.1` |

索引算法（A 级）：`idx = ((MapPosX+MapPosY)&1)<<1 | side`（`lda MapPosX; adc MapPosY; rol`（把侧位旋进 bit0）`; and #$03; tay`）。

### 1.2 门型与开闭判定（A 级）

- 蓝门（`1`）与蓝门换音乐（`3`）：**不检查道具**，直接 `status←1`（初始化开门动画）。
- 红门（`0`）/十发导弹门（`2`）：先查 `pu_MISSILEDOOR` 的唯一道具记录
  —— `Temp09_ItemType←pu_MISSILEDOOR`，`Y = MapPosX`，**左门时 `Y=MapPosX-1`**（相邻两门共享同一把钥匙），
  `jsr PrepareItemID@sameItemType; jsr CheckForItem`；已持有 ⇒ 跳 `status←1`（不重播开门），否则 `status←1` 走开门动画。
- 十发导弹门与红门在数据里唯一区别是 `B1&3` 的值；`DoorType` 被写到 `Doors.data07`（`constants_ram.asm:939`）。

### 1.3 逐区门记录统计（A 级：`rooms.asm` 符号行逐条正则统计）

| 区 | 门记录数 | 右/左 · 蓝 | 右/左 · 红 | 十发导弹 | 蓝+换音乐 |
|---|---|---|---|---|---|
| Brinstar | **27** | 11 / 12 | 2 / 2 | 0 | 0 |
| Norfair | **26** | 11 / 13 | 1 / 1 | 0 | 0 |
| Kraid | **31** | 13 / 12 | 4 / 2 | 0 | 0 |
| Tourian | **5** | 0 / 0 | 2 / 2 | 1 | 0 |
| Ridley | **26** | 10 / 12 | 1 / 1 | 1 / 1 | 0 |
| 合计 | **115** | | | **3** | **0** |

**另有一类「门」不在房间记录里**：特殊物品表 `type=$09 it_Door` 共 **9 条**（Norfair 5、Tourian 4）
——`SpawnDoor ; 07:EEF4` 直接 `jsr SpawnDoorRoutine`，用**同一套 `B1` 语义**但从道具表读参。

### 1.4 样本逐字节（A 级，命令见 §5.1）

**Brinstar Room00 `01:A441`**（19 B）对象段：`FD | 02 A1 | 02 B1 | FF`

```
02 A1 → 槽0、门型 A1: bit4=0(右) type=1(蓝)  → DoorSlots[(0+? )] 右白/右黑由 MapPos 定
02 B1 → 槽0、门型 B1: bit4=1(左) type=1(蓝)
```
两条门都写**同一个槽号 0**（`B0` 高半字节=0）：这不是 bug——`DoorSlots[idx]` 才决定真正落哪个对象槽，
`B0` 高半字节只是房间记录里的「门组槽号」，两条记录在**不同名表**上（右门在当前表、左门在另一表）。
> B 级：`GetEnemyData` 用 `B0` 高半字节直接当 `$0400` 槽基址，而 `SpawnDoorRoutine` 又用 `DoorSlots` 覆盖 ⇒
> 门对象槽位最终由 `DoorSlots` 决定；`B0` 高半字节对门实际不生效（待 trace 复核，§6 H2）。

**Norfair Room04 `02:A42F`**（68 B）对象段：`FD | 02 A0 | FF`

```
02 A0 → 槽0、bit4=0(右)、type=0(红门)   ← 与 rooms.asm 的 `$A0 | (0<<4) | doorType_Red` 一致
```
该房整条记录：`00`（调色板）+ 21 条结构（到 `FD`）+ 1 条门 + `FF`。

## 2. 转移链：踩门 → 切房（A 级，逐指令）

### 2.1 进门瞬间（`SamusEnterDoor ; $8B13`，`areas_common.asm:1317`）

触发条件：`DoorEntryStatus==0`（不在门里）且 `SamusDoorData!=0`（刚碰到门的碰撞砖）。

```
DoorDelay ← $12(18 帧)
DoorEntryStatus |= $80        ; 标记「刚进门」
Samus.status ← sa_Door(=$0C)
SamusDoorData ← (旧 SamusDoorData×16) | 旧 Samus.status
```
`SamusDoorData` 高位语义（`constants_ram.asm:581` 注释 + `prg7_engine.asm:8216-8227`，A 级）：
碰到可走砖 `$A0` 时 `SamusDoorData=1`、碰到 `$A1` 时 `=2`；`1` ⇒ 出门后滚动方向翻转，`2` ⇒ 出门后设为水平滚动
（道具房/桥房不翻转）。

### 2.2 滚屏（`ScrollDoor ; 07:E1F1` = `prg7_engine.asm:7006`）

`DoorScrollStatus`（`$57`，`constants_ram.asm:573-578`）决定每次滚 1 像素的方向：

| 值 | 进入方式 | 动作 |
|---|---|---|
| 1 | 水平区右门 | `jsr ScrollRight` |
| 2 | 水平区左门 | `jsr ScrollLeft` |
| 3 | 竖直区，需先居中 | `jsr ScrollDown` → `VerticalRoomCentered` |
| 4 | 竖直区，已居中 | `jsr ScrollUp` |

`ScrollX==0`（滚满一屏）⇒ `x=5`，进 `DoOneDoorScroll ; 07:E22C`。

`DoOneDoorScroll`（`prg7_engine.asm:7049`）：
`DoorDelay←$20`；`SamusDoorData×8` 的进位（=原 bit4？实测语义见下）与 `DoorScrollStatus<3` 决定是否
`ToggleScroll`；写 `MirrorCntrl`；`DoorEntryStatus←5`。
> B 级：`Amul8` 三次 `asl` 的进位 = `SamusDoorData` bit5；而 `SamusDoorData` 高 4 位是 1/2 ⇒ 只有 `2` 会让进位为 1，
> 与 `constants_ram.asm:581` 注释「2=设为水平滚动（道具房）」一致；未用 trace 逐帧核。

### 2.3 切房点（在 `ScrollLeft/ScrollRight` 里，A 级）

`ScrollLeft ; 07:E6A7`（`:7951`）在 `ScrollX==0` 时：

```
dec MapPosX
jsr GetRoomNum      ; $5A = WorldMap[MapPosY*32 + MapPosX]；$FF=无房、CF=1=边界
bcs @atLeftBound    ; 边界 ⇒ 回滚 MapPosX，停止滚动
jsr ToggleNameTable ; 切换 $6000/$6400 两张名表
```
`ScrollRight ; 07:E6D2`（`:7983`）对称（`inc MapPosX`）。
随后每帧 `jsr CheckUpdateNameTable ; 07:E54A`（`:7702`）在列到边时 `jsr SetupRoom`（`:7703`，即 #59 的入口）把
`$5A` 指定的房装进来。**这就是「目标房间」的全部来源**：没有门→房映射表，只有 WorldMap 的一格邻接关系。

### 2.4 出门（`SamusDoor ; 07:D3A8` = `:4093`）

`DoorEntryStatus≥5` ⇒ `dec DoorDelay` 到 0 后：

```
asl; bcc …; lsr; sta DoorEntryStatus   ; 清 0x80
jsr Door_DeleteOffscreenEnemies / Doors_RemoveIfOffScreen
Samus.status ← SamusDoorData & $0F     ; 恢复进门前的动作
SamusDoorData ← 0; DoorEntryStatus ← 0
jsr MoveOutDoor ; 07:D3F7
```
`MoveOutDoor` 按 `SamusDoorDir`（0=右、1=左，`prg7_engine.asm:7343-7348` 由移动方向反推）把 Samus 往外推 1 像素，
越过名表边界时 `ToggleSamusHi`。

### 2.5 切房时被改写的 RAM（汇总，A 级）

| 字段 | 写点 | 值 |
|---|---|---|
| `MapPosX $6D` / `MapPosY $6E` | `ScrollLeft :7962/7966`、`ScrollRight :7976/7993/7997` | ±1 或回滚 |
| `RoomNumber $5A` | `GetRoomNum ; 07:E720`（返回在 `$5A`） | `WorldMap` 查表；`$FF`=无房 |
| 名表选择（PPUCTRL 位） | `ToggleNameTable` | 0↔3 |
| `ScrollX $FD` / `ScrollY $FC` | 滚动例程 | 每帧 ±1 |
| `ScrollBlockOnNameTable3` | `SpawnDoorRoutine` / `VerticalRoomCentered` / `SamusEnterDoor` | 门封条位 |
| `RoomPalette $68` / `$6000`–`$67FF` 名表 | `SetupRoom`（#59 §3） | 由房记录重画 |
| `DoorDelay $59` / `DoorEntryStatus $56` / `SamusDoorData $58` | §2.1/§2.2/§2.4 | 见上 |
| `MirrorCntrl $FA`、`PaletteDataPending`、`InArea`、`BankSwitchPending` | 只电梯径（§3.5） | 见 §3.5 |

## 3. 升降台

### 3.1 房间记录里的升降台（2 B，A 级）

`LoadElevator ; 07:EC04`（`:9120`）→ `SpawnElevatorRoutine ; 07:EC09`（`:9124`）：

```
Elevator.status != 0 ⇒ 直接返回（一台梯只生成一次）
iny; lda (RoomPtr),y → ElevatorType($032F)   ; 即记录 B1
Elevator.y ← $83、x ← $80、hi ← 名表位、animFrame ← $23(_id_ObjFrame_Elevator)
inc Elevator.status   ; 0→1（Idle）
```
**记录只需 B1**；`B0` 的低半字节 `4` 仅是类型码。**原版 5 区 `rooms.asm` 里 `re_Elevator` 记录数 = 0（A 级 grep）**
⇒ 无数据使用这条路径（B 级：实现存在但未被原版数据使用；不排除有未符号化的原始字节形式，未全量扫描，§6 H1）。

### 3.2 特殊物品表里的电梯（11 条，A 级：`items.asm` + ROM）

`SpecItmsTbl` 的 `it_Elevator` 条目：`[x][delta|$FF][$04, param][$00]`，`param` = `ElevatorType` 字节。
`ChooseSpawningRoutine ; 07:EDD6` 的表项 4 → `SpawnElevator ; 07:EEA1`（`:9700`）→ 同一个 `SpawnElevatorRoutine`，
此时 `(ptr),y` 读到的是条目里的 `param`。

| 区（表） | 地图 y | 地图 x | param | bit7 | 目的区（代码判定） | asm 注释 |
|---|---|---|---|---|---|---|
| Brinstar | `02` | `03` | `$03` | 0 下 | 3=Tourian | Elevator to Tourian |
| Brinstar | `0B` | `16` | `$01` | 0 下 | 1=Norfair | Elevator to Norfair |
| Brinstar | `0E` | `07` | `$02` | 0 下 | 2=Kraid | Elevator to Kraid |
| Norfair | `0B` | `16` | `$81` | 1 上 | Brinstar（默认） | Elevator from Brinstar |
| Norfair | `0D` | `16` | `$81` | 1 上 | Brinstar（默认） | Elevator to Brinstar |
| Norfair | `15` | `19` | `$04` | 0 下 | 4=Ridley | Elevator to Ridley hideout |
| Kraid | `12` | `07` | `$81` | 1 上 | Brinstar（默认） | Elevator from Brinstar |
| Kraid | `14` | `07` | `$82` | 1 上 | Brinstar（默认，低半字节被忽略） | Elevator to Brinstar |
| Tourian | `03` | `01` | `$8F` | 1 上 | **结局**（`AtEnding++`） | Elevator to end |
| Tourian | `04` | `03` | `$83` | 1 上 | Brinstar（默认） | Elevator to Brinstar |
| Ridley | `18` | `19` | `$84` | 1 上 | 1=Norfair（唯一特例） | （对应 Norfair 的 `$04`） |

**参数编码（A 级，`ElevatorD8BF ; 07:D8BF` = `:4980`）**：
- bit7=1（上行）：默认目的 = **Brinstar**；`cmp #$84` 相等 ⇒ **Norfair**；`$8F` 在更前面被拦成结局。
- bit7=0（下行）：目的区 = **低半字节**（0=Br、1=No、2=Kraid、3=To、4=Ri），代码直接 `ora #$10` → `InArea=$1X`。
- 因此 `$81/$82/$83` 在「上行」时全落 Brinstar——`$82`（Kraid）与 `$83`（Tourian）的低半字节**确实被忽略**，
  与 `items.asm` 注释的「to Brinstar」一致（不是笔误，A 级代码 + A 级注释互证）。

### 3.3 电梯状态机（`UpdateElevator ; 07:D7B3` = `:4806`，JumpEngine **10 项**：0–9）

| status | 例程（地址） | 行为 |
|---|---|---|
| 0 | `ExitSub` | 空 |
| 1 | `ElevatorIdle 07:D7D1`（`:4822`） | 等 Samus 站上并按 上/下（按 `ElevatorType` bit7 取 `BUTTON_UP/DOWN`）；起步时 `Samus.status←sa_Elevator`、Samus 居中 `x=128,y=112` |
| 2 | `ElevatorScrollXToCenter 07:D80E`（`:4862`） | 把横滚收到 `ScrollX==0`；置 `MirrorCntrl\|=$08`（竖向排布名表）、`ScrollDir &= 1` |
| 3 | `ElevatorMove 07:D83D`（`:4891`） | 每帧 1 像素走到 `y=$83`（回绕时 `ToggleObjectXSlotHi`） |
| 4 | `ElevatorScrollY 07:D870`（`:4928`） | 竖滚到 `ScrollY==0`；置 Samus/电梯 fade-out 动画；`Timer1←$40`（注释自述为残留） |
| 5 | `ElevatorFade 07:D8A3`（`:4963`） | 淡出：只递增 status，`5→6` |
| 7 | `ElevatorFade`（同例程，JumpEngine 第 7 项） | 淡入：`7→8`；与 5 同一例程，两次调用分别是换区前淡出/换区后淡入 |
| 6 | `ElevatorD8BF 07:D8BF`（`:4980`） | **换区**：判目的区 → `InArea←$10\|area` → `IsEngineRunning`(⇒`SwitchBank`) → 调色板/音乐/屏幕 → `CopyAreaPointers` → `DestroyEnemies` → fade-in |
| 8 | `ElevatorMove`（复用） | 继续走到 `y=$83` |
| 9 | `ElevatorStop 07:D94E`（`:5089`） | `ScrollY==0` 时站定：`Samus.status←sa_Stand`，电梯 `status←1`，`ElevatorType ^= $80`（换向）；到上层时 `ToggleScroll` |

### 3.4 电梯与 Samus 移动（`SamusElevator ; 07:D423` = `:4169`）

只有 `Elevator.status∈{3,8}` 时接管 Samus：下行看 `Samus.y-ScrollY ≥ $84` 才 `ScrollDown`，上行 `< $64` 才 `ScrollUp`；
`Samus.y` 到边界时 `ToggleSamusHi` 回绕（`SCRN_VY-1`/`SCRN_VY`）。

### 3.5 换区时被改写的 RAM（A 级）

| 字段 | 写点 | 值 |
|---|---|---|
| `ElevatorType $032F` | `SpawnElevatorRoutine` :9130 | 记录/条目 param |
| `InArea $3E` | `ElevatorD8BF` → `SwitchBank 07:CA23`（`:2263`） | `$10 \| 目的区索引`（0 Br、1 No、2 Kr、3 To、4 Ri） |
| `BankSwitchPending $25` | `BankTable 07:CA30`（`:2274`）：`02 03 05 04 06` | `bank+1`（Br=2,No=3,Kr=5,To=4,Ri=6） |
| `RoomPtr $33/$34` | 结局径 `$8F`：`RoomPtr←0` | 仅结局 |
| `MainRoutine` / `AtEnding` | 结局径 | `$8F` ⇒ `MainRoutine←IncrementRoutine`、`AtEnding++`、`TitleRoutine←EndGame` |
| `PaletteToggle`/`PaletteDataPending` | `ElevatorD8BF`（`:5010-5024`） | `eor #$07`；Tourian(`InArea≥$12`) ⇒ 调色板 0 |
| `MirrorCntrl $FA` | `ElevatorScrollXToCenter` | `\|= $08` |
| `ScrollDir $4A` | 同上 | `&= $01` |
| 名表/房数据 | `CopyAreaPointers 07:C8B0`（`:1936`）→ `DestroyEnemies` → 下一帧 `AreaInit` 用 `AreaMapPosX/Y` 设 `MapPosX/Y` 并 `SetupRoom` | 见 #59 §3 |

> B 级：`AreaInit`（`prg7_engine.asm:1870-1900`）在换区后把 `AreaMapPosX/Y` 写进 `MapPosX/Y`；具体相位（换区当帧还是下一帧）
> 未用 trace 逐帧核，故标 B。

## 4. 与现实现对照（只列事实，不改代码）

| 面 | ROM | Rust 现状 | 判定 |
|---|---|---|---|
| 门记录解析/槽位/侧/型 | `SpawnDoorRoutine 07:EB92` | `crates/game-core/src/world/room_draw.rs:274 load_door`（`DOOR_SLOTS=[0x00,0x30,0x20,0x10]`、`DOOR_XS=[0xF0,0x10]`、`side=(info>>4)&1`、`type=info&3`、导弹门 `MapPosX-1`、`status←1` 分支） | 已端口，逐字段一致 |
| 升降台记录 | `SpawnElevatorRoutine 07:EC09` | `world/room_draw.rs:784 load_elevator`（`$032F/$032D/$032E/$032C/$0323`、`status←1`、已有梯则返回） | 已端口 |
| 门滚屏 | `ScrollDoor 07:E1F1` / `DoOneDoorScroll 07:E22C` | `world/update.rs:24 scroll_door` / `:70 do_one_door_scroll` | 已端口 |
| 进门/出门 | `SamusEnterDoor $8B13` / `SamusDoor 07:D3A8` / `MoveOutDoor 07:D3F7` | `samus/physics.rs:1395 samus_enter_door`、`samus/handler.rs:1618 samus_door`、`world/door.rs:39 move_out_door` | 已端口 |
| 电梯状态机/换区 | `UpdateElevator 07:D7B3` 9 项 | `world/elevator.rs:92 update_elevator`（`elev_idle:129`、`elev_scroll_x_to_center:157`、`elev_move:182`、`elev_scroll_y:215`、`elev_scroll_room:231`、`elev_fade:243`、`elev_transition:272`、`elev_stop:334`） | 已端口 |
| 换区 → 载房 | `IsEngineRunning 07:CA18` / `SwitchBank 07:CA23` / `BankTable 07:CA30` | `world/elevator.rs:272 elev_transition`（`InArea=$1X` + `BankSwitchPending` 表 `02 03 05 04 06`）；`room_load.rs:227 is_new_load_request`（`camera/scroll.rs:44`）→ `room_load.rs:306 setup_room` | 已端口 |
| 目标房间 | `GetRoomNum 07:E720`（WorldMap） | `world/room_load.rs:182 get_room_num` | 已端口 |

**差异/口径（不是缺陷）**：
1. 卡面假设的「目标房间/目标门索引」字段在 ROM 里不存在；Rust 也按「WorldMap 步进」实现，两边一致。
2. `B0` 高半字节对门不生效（被 `DoorSlots` 覆盖）——Rust 的 `load_door` 同样只用 `DoorSlots`，行为一致；文档层面写清即可。
3. 电梯 `$82/$83` 上行时低半字节被忽略 → 目的仍 Brinstar；这是 ROM 行为，Rust `elev_transition` 按 `$84`/`$8F` 特例实现，需保持。

## 5. 复现命令（每条可原样跑）

> **读记录前必看**：房间对象段的**记录长度按 object type 分流**——`1`（Enemy）与 `7`（Enemy spawner，如 Geega）是 **3 B**，
> `2`（Door）/`4`（Elevator）/`6`（Statues）是 **2 B**，`0`/`3`/`5` 是 `ExitSub`（不占字节）。
> 若按「非 1 即 2 B」推进会从第一个 type7 记录起错位，把后续字节误读成门/电梯（复核时实测会得出 Br 29 门、Kr 3 台电梯的错误结果）。

```bash
# 5.1 两个样本房的门记录（Br Room00 / No Room04）
python3 - <<'PY'
import pathlib
rom=pathlib.Path('rom/Metroid (USA).nes').read_bytes(); prg=rom[16:16+128*1024]
def rd(b,a,n): return prg[b*0x4000+(a-0x8000):][:n]
hx=lambda x:' '.join(f'{v:02X}' for v in x)
print('Br00 A441', hx(rd(1,0xA441,19)))  # 02 40 01 02 ... FD 02 A1 02 B1 FF
print('No04 A42F', hx(rd(2,0xA42F,68)))  # 00 ... FD 02 A0 FF
PY

# 5.2 11 条电梯条目（区/坐标/param）
python3 - <<'PY'
import re
for a in ['brinstar','norfair','kraid','tourian','ridley']:
    L=open(f'refs/m1disasm/SRC/{a}/items.asm',encoding='utf-8',errors='replace').read().splitlines()
    y=None
    for i,l in enumerate(L):
        m=re.match(r'\s*\.byte\s*\$([0-9A-Fa-f]{2})\s*$',l)
        if m and i+1<len(L) and '.word @y' in L[i+1]: y=m.group(1)
        m=re.search(r'it_Elevator,\s*\$([0-9A-Fa-f]{2})',l)
        if m:
            x=next((re.match(r'\s*\.byte\s*\$([0-9A-Fa-f]{2}),',L[j]).group(1)
                    for j in range(i-1,max(i-6,0),-1) if re.match(r'\s*\.byte\s*\$([0-9A-Fa-f]{2}),',L[j])),'?')
            print(a,'y',y,'x',x,'param $'+m.group(1))
PY

# 5.3 五区门记录按 (侧,门型) 统计
python3 - <<'PY'
import re
from collections import Counter
for a in ['brinstar','norfair','kraid','tourian','ridley']:
    t=open(f'refs/m1disasm/SRC/{a}/rooms.asm',encoding='utf-8',errors='replace').read()
    d=re.findall(r're_Door,\s*\$([0-9A-F]{2})\s*\|\s*\((\d)\s*<<\s*4\)\s*\|\s*doorType_(\w+)',t)
    print(a,len(d),Counter((s,ty) for _,s,ty in d))
PY

# 5.4 xref 计数（门/电梯例程）
for s in SpawnDoorRoutine SpawnElevatorRoutine UpdateElevator ScrollDoor SamusEnterDoor UpdateDoorData; do
  printf "%s %s\n" $s "$(grep -c "jsr $s" refs/m1disasm/SRC/prg7_engine.asm refs/m1disasm/SRC/areas_common.asm | awk -F: '{s+=$2}END{print s}')"
done   # 2 2 1 2 1 2
```

## 6. 假设区（C 级）

- **H1**：房间记录里的 `re_Elevator`（类型码 4）在原版数据里 0 处使用；是否有未符号化的原始字节使用该路径**未全量扫描**。
- **H2**：门记录 `B0` 高半字节对门对象槽不生效（被 `DoorSlots` 覆盖）；推断为「记录里门组槽号」的遗留字段，未用 trace 核。
- **H3**：`DoOneDoorScroll` 用 `SamusDoorData×8` 进位决定 `ToggleScroll`；进位=bit5 的推导来自 `Amul8` 三次 `asl`，
  与 `constants_ram.asm:581` 注释互证，但未做逐帧 trace。

## 7. 未覆盖

1. 门的**开门/关门动画**（`UpdateAllDoors`、门瓦片重画、`DoorEntryStatus` 与 `Objects.status` 的时序）只列了入口，未逐帧。
2. `it_Door` 9 条（Norfair 5 / Tourian 4）的逐条坐标与 `B1` 未解。
3. Tourian/Ridley/Kraid 的门记录未逐房 decode（只有类别统计）。
4. 电梯换区后 `AreaInit` 的相位（同帧/下一帧）未 trace。
5. `LoadStatues`/`LoadPipeBugHole` 记录长度未取证（本文件只涉及门/电梯）。

## 8. 证据指针

| 结论 | 指针 |
|---|---|
| 门记录 2 B / 槽位 / 侧 / 型 | `refs/m1disasm/SRC/prg7_engine.asm:9015-9119`（LoadDoor/SpawnDoorRoutine/DoorXs/DoorScrollBlocks/DoorSlots） |
| 门型枚举 | `refs/m1disasm/SRC/constants.asm:50-53`；`constants_ram.asm:939`（`DoorType $0307`） |
| 门槽/侧表字节 | `prg7_engine.asm:9105-9119` |
| 进门 | `refs/m1disasm/SRC/areas_common.asm:1317-1408`（`SamusEnterDoor`/`SetDoorEntryInfo`/`SamusInDoor`） |
| `SamusDoorData` 语义 | `constants_ram.asm:581-588`；`prg7_engine.asm:8216-8227`（碰撞砖 `$A0`/`$A1` → 1/2） |
| 门滚屏 | `prg7_engine.asm:7006-7070`（ScrollDoor / VerticalRoomCentered / DoOneDoorScroll） |
| 切房点 | `prg7_engine.asm:7951-8000`（ScrollLeft/ScrollRight 的 `MapPosX±1 → GetRoomNum`）；`:7702` CheckUpdateNameTable |
| 出门 | `prg7_engine.asm:4093-4148`（SamusDoor/MoveOutDoor） |
| 升降台记录 | `prg7_engine.asm:9120-9145`（LoadElevator/SpawnElevatorRoutine） |
| 电梯条目 | `refs/m1disasm/SRC/{brinstar,norfair,kraid,tourian,ridley}/items.asm`（`it_Elevator` 共 11 条）；`constants.asm:33` |
| 电梯状态机 | `prg7_engine.asm:4806-5120`（UpdateElevator 表 + 9 例程）、`:4169`（SamusElevator） |
| 换区 | `prg7_engine.asm:4980-5030`（ElevatorD8BF）、`:2247-2280`（IsEngineRunning/SwitchBank/BankTable） |
| Rust 对照 | `world/room_draw.rs:274,784`；`world/update.rs:24,70`；`world/elevator.rs:92-334`；`world/door.rs:39`；`samus/physics.rs:1395`；`samus/handler.rs:1618`；`world/room_load.rs:182,227,306`；`world/camera/scroll.rs:44` |
| 房间记录布局（衔接） | `docs/spec/room-data-format.md` §1.2/§3 |
| ROM 身份 | `sha256 e6e6b701…65b6` |
