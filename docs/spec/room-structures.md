# 房间结构记录、属性命令与雕像/管道（B5-c 规格）

> 只读考证：ROM 字节（`rom/Metroid (USA).nes`，sha256 `e6e6b701…65b6`）+ 社区匹配反汇编（`refs/m1disasm/SRC/`）
> + 现实现（`crates/game-data/`、`crates/game-core/src/world/`）。**本文件不改任何代码。**
> 证据级别：**A** = 字节/汇编逐条一致；**B** = 代码语境推断；**C** = 未取证（§7）。
> 接 #59 `docs/spec/room-data-format.md`（房间表/落位）与 #61 `docs/spec/doors-elevators.md`（门/升降台）。

## 0. 结论与卡面口径更正

1. **结构记录 = 3 字节**：`[pos: yyyyxxxx][结构号][属性字节]`。`y = pos>>4`（单位 **2 个 tile 行 = 16 px**）、
   `x = (pos&$0F)*2`（单位 **2 个 tile 列 = 16 px**）。位置是「左上角」，不是尺寸。（A 级，§1.1）
2. **结构本体是行流**：每行首字节「低半字节=元 tile 个数（0 表示 16）/ 高半字节=x 偏移（元 tile 单位）」，
   随后是该行的元 tile 索引，`$FF` 结束。（A 级，§1.3）
3. **元 tile 定义 4 字节/组，顺序是 `[左下？]`——准确说是 `LR, LL, UR, UL`**（先写右下、再左下、再右上、再左上），
   来自 `DrawMetatile` 的 `TilePosTable` 反序读取。组索引 = 结构流里的字节 × 4。（A 级，§1.4）
4. **卡面说的「`$F0–$F4` 属性命令」不是房间数据里的字节**（全五区 `rooms.asm` 的 `.byte $F0…$F4` 命中 **0** 次）。
   `$F0–$F4` 是**运行时 `RoomNumber` 的阶段值**：`$F0` 起步 → `$F1` 写属性表第 1 行 → `$F2` 空转 → `$F3` 写第 2 行 →
   `$F4` 完成（置 `$FF`）。房间数据里真正的分节符是 `$FD`（对象段开始）与 `$FF`（结束），`$FE` 是空占位（§3）。
5. **对象段类型 = 首字节低半字节**（高半字节=对象槽号），类型 0–7 见 §3.1；**雕像记录只有 1 字节**（全游戏 1 条，§4），
   **管道虫洞记录 3 字节**（全游戏 42 条，§5）。
6. **更正 #61 的门记录计数**：Brinstar 是 **29** 条而不是 27 条——`rooms.asm` 里 Room03/Room0B 的门写成裸字节
   `.byte $02,$A1` / `.byte $02,$B1`（行 83/248），按 `re_Door` 宏计数会漏 2 条。其余四区不变（26/31/5/26）。

## 1. 结构记录

> 先说清：结构记录里**没有**「尺寸」与「碰撞」字段。位置只给左上角，尺寸由结构行流隐含；碰撞由 tile 号隐含（`$FF`=空白，其它 tile 的通行性见属性/瓦片表）。

### 1.1 3 字节记录（A 级）

`DrawRoom ; 07:EAAA` 逐字节分派：`$FF`=结束、`$FE`=空占位、`$FD`=对象段开始、其它=结构记录。
`DrawRoomObject ; 07:EA60`：

| 字节 | 位 | 含义 | 证据（`prg7_engine.asm`） |
|---|---|---|---|
| `pos` | 高半字节 | y 位置：**2 个 tile 行/单位**（每单位 RoomRAM +`$40`，即 32 字节=2 行） | `:8797-8812`（`Adiv16` 后循环 `adc #$40`） |
| `pos` | 低半字节 | x 位置：**2 个 tile 列/单位**（每单位 +2 字节=2 tile） | `:8817-8827`（`and #$0F; asl; adc`） |
| `wid` | — | 结构号：`StructPtrTable[area][idx]` 的 16 位指针 | `:8830-8838`（`asl;tay; lda (StructPtrTable),y`） |
| `attr` | — | 属性表值（0–3）：写进结构覆盖的每个 2×2 元 tile 的属性位；**等于房间调色板字节时整块跳过**（性能优化） | `:8836`（`sta ObjectPalette`）+ `UpdateAttrib ; 07:EF9E`（`:9976` `cmp RoomPalette; beq rts`） |

### 1.2 逐区表与计数（A 级：ROM 指针表 + 全量解析）

| 区 | RoomPtrTable | 房数 | StructPtrTable | 结构号数 | MetatileDefs | 元 tile 组数 | 结构记录总数 | 实际用到结构号 | 敌 | 门 | 电梯 | 雕像 | 管道 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| Brinstar | `01:A314` | 47 | `01:A372` | 50 | `01:AEF0` | 65 | **520** | 50 | 113 | 29 | 0 | **1** | 6 |
| Norfair | `02:A21B` | 46 | `02:A277` | 49 | `02:AEEC` | 62 | **604** | 48 | 102 | 26 | 0 | 0 | 5 |
| Kraid | `04:A1D5` | 37 | `04:A21F` | 39 | `04:AC32` | 41 | **539** | 37 | 51 | 31 | 0 | 0 | 16 |
| Tourian | `03:A7D1` | 21 | `03:A7FB` | 32 | `03:AE49` | 54 | **249** | 32 | 20 | 5 | 0 | 0 | 0 |
| Ridley | `05:A17F` | 42 | `05:A1D3` | 29 | `05:AB23` | 33 | **476** | 29 | 94 | 26 | 0 | 0 | 15 |

（表长与 ROM 地址另有现实现侧的等价声明：`crates/game-data/src/extract.rs:544-556`、`:578-579`、`:597-598`、`:616-617`。）

### 1.3 结构行流（A 级）

`DrawStruct ; 07:EF8C` / `DrawStructRow ; 07:EF13`：

```
结构流 := 行* $FF
行     := 长度字节 元tile索引×(低半字节 or 16)
长度字节: 低半字节 = 元 tile 个数（0⇒16）；高半字节 = 行内 x 偏移（元 tile 单位）
```

- 行末若越过当前 32 tile 行边界，`DrawStructRow` 会提前结束该行并把剩余元 tile 数带到下一行（`:9840-9852`）。
- 元 tile 是 **2×2 tile**；写 RoomRAM 的 4 个位置由 `TilePosTable`（`:9886`：`$21,$20,$01,$00`）给定。
- 结构流没有长度字段，靠 `$FF` 自界；`$FF` 在**行首**出现即结束（若在元 tile 索引位置出现则按数据读，造数据时须避免）。

### 1.4 元 tile 定义（4 B，顺序 `LR,LL,UR,UL`，A 级）

`@DrawMetatile ; 07:EF2B`（`:9822-9834`）：`X=4-1..0`，每次取 `MetatileDefs[idx*4+k]` 写入 `TilePosTable[X]`：

| 写入顺序 | k | `TilePosTable[X]` | 位置 |
|---|---|---|---|
| 1 | 0 | `$21` | 右下 |
| 2 | 1 | `$20` | 左下 |
| 3 | 2 | `$01` | 右上 |
| 4 | 3 | `$00` | 左上 |

所以 `MetatileDefs+idx*4` 的字节序是 **LR, LL, UR, UL**（不是常见的左上起）。例：Br 元 tile `$02`=`64 64 64 64`（实心），
`$04`=`a4 ff a4 ff`（竖条，`$FF` 是空白 tile），`$00`=`f1 f1 f1 f1`（默认底）。

### 1.5 结构记录样本表（A 级，Brinstar 10 条 + Norfair 10 条）

`pos` 列写作 `字节(y,x 单位)`；「结构行」按 §1.3 解码（`个数×元tile…`）。

| 区 | 房 | 房基址 | 记录偏移 | 字节 `pos/结构号/attr` | 解码位置 | 结构地址与行流 | 首元 tile 4 字节 |
|---|---|---|---|---|---|---|---|
| Br | R00 | `A441` | 1 | `40/01/2` | y=4, x=0 | `AC97`：8×02…02 → 1×1C → 1×1C → 1×1C → 8×02…02 | `64 64 64 64`（02） |
| Br | R00 | `A441` | 4 | `48/01/2` | y=4, x=16 | `AC97`：同上 | `64 64 64 64`（02） |
| Br | R00 | `A441` | 7 | `50/03/2` | y=5, x=0 | `ACC9`：1×06 → 1×06 → 1×06 | `a0 a0 a0 a0`（06） |
| Br | R00 | `A441` | 10 | `5F/03/2` | y=5, x=30 | `ACC9`：同上 | `a0 a0 a0 a0`（06） |
| Br | R01 | `A454` | 1 | `07/02/2` | y=0, x=14 | `ACB0`：8 行，每行 2×04,05 | `a4 ff a4 ff`（04） |
| Br | R01 | `A454` | 4 | `87/02/2` | y=8, x=14 | `ACB0`：同上 | `a4 ff a4 ff`（04） |
| Br | R02 | `A45C` | 1 | `00/0B/3` | y=0, x=0 | `AD0A`：2×36,36 → 2×28,08 → 2×08,52 → 2×52,52 → 2×08,08 | `60 61 62 63`（36） |
| Br | R02 | `A45C` | 4 | `0E/0B/3` | y=0, x=28 | `AD0A`：同上 | `60 61 62 63`（36） |
| Br | R02 | `A45C` | 7 | `50/0B/3` | y=5, x=0 | `AD0A`：同上 | `60 61 62 63`（36） |
| Br | R02 | `A45C` | 10 | `5E/0B/3` | y=5, x=28 | `AD0A`：同上 | `60 61 62 63`（36） |
| No | R00 | `A3AB` | 1 | `40/01/2` | y=4, x=0 | `ACCC`：8×02…02 → 1×0A → 1×0A → 1×0A → 8×02…02 | `64 64 64 64`（02） |
| No | R00 | `A3AB` | 4 | `48/01/2` | y=4, x=16 | `ACCC`：同上 | `64 64 64 64`（02） |
| No | R00 | `A3AB` | 7 | `50/04/2` | y=5, x=0 | `AD05`：1×07 → 1×07 → 1×07 | `a1 a1 a1 a1`（07） |
| No | R00 | `A3AB` | 10 | `5F/04/2` | y=5, x=30 | `AD05`：同上 | `a1 a1 a1 a1`（07） |
| No | R01 | `A3BE` | 1 | `07/02/2` | y=0, x=14 | `ACE5`：8 行，每行 2×04,05 | `a4 ff a4 ff`（04） |
| No | R01 | `A3BE` | 4 | `87/02/2` | y=8, x=14 | `ACE5`：同上 | `a4 ff a4 ff`（04） |
| No | R02 | `A3C6` | 1 | `00/0B/3` | y=0, x=0 | `AD39`：4×0F,0C,0C,0B → 4×0B,0F,0B,0C → 4×0C,0F,0C,0B → 4×0F,0B,0F,0C | `1f 20 21 22`（0F） |
| No | R02 | `A3C6` | 4 | `04/06/3` | y=0, x=8 | `AD10`：4×0B,0B,0B,0B | `10 10 10 10`（0B） |
| No | R02 | `A3C6` | 7 | `08/06/3` | y=0, x=16 | `AD10`：同上 | `10 10 10 10`（0B） |
| No | R02 | `A3C6` | 10 | `0C/0B/3` | y=0, x=24 | `AD39`：同上 | `1f 20 21 22`（0F） |

**其它三区抽样（各 2 条）**

| 区 | 房 | 基址 | 偏移 | 字节 | 结构地址与行流 | 首元 tile |
|---|---|---|---|---|---|---|
| Kr | R00 | `A2B7` | 1 | `40/01/3` | `AA7E`：8×02…02 → 1×1C → 1×1C → 1×1C → 8×02…02 | `64 64 64 64`（02） |
| Kr | R00 | `A2B7` | 7 | `50/04/2` | `AAB7`：1×07 → 1×07 → 1×07 | `a1 a1 a1 a1`（07） |
| To | R00 | `A8AF` | 1 | `40/01/3` | `AC29`：8×0A… → 8×02… → 1×1C → 8×03… → 8×0A… | `04 05 06 07`（0A） |
| To | R00 | `A8AF` | 7 | `50/03/2` | `AC69`：1×08 → 1×08 → 1×08 | `a0 a0 a0 a0`（08） |
| Ri | R00 | `A23F` | 1 | `07/03/2` | `A9E0`：8 行，每行 2×02,03 | `a4 ff a4 ff`（02） |
| Ri | R00 | `A23F` | 4 | `87/03/2` | `A9E0`：同上 | `a4 ff a4 ff`（02） |

> 「结构行」只列前几行；完整行流用 §6.2 的命令原样打印。元 tile 索引与 4 字节定义用 §6.3 打印。

## 2. 房间调色板字节与 attr 的关系（A 级）

- 房记录**第一个字节**是房间调色板（0–3），`SetupRoom` 先 `sta RoomPalette` 再 `InitTables` 用它填满整张属性表。
- 结构记录的 `attr` 若 **等于** `RoomPalette`，`UpdateAttrib` 直接返回（省掉属性表写）；
  不等时把该 2×2 元 tile 对应的 2 个属性位改成 `attr`（掩码表 `@attributeMaskTable ; 07:EFF4`：`FC/F3/CF/3F`）。
- 因此 `attr` 的语义是「这块结构的调色板编号（0–3）」，不是尺寸/碰撞。碰撞完全由 tile 号隐含（`$FF`=空白）。

## 3. 对象段（`$FD` 之后）

### 3.1 类型表（A 级：`EnemyLoop ; 07:EAD4` 的 JumpEngine 表）

分派前先 `and #$0F` ⇒ 低半字节是**类型**；高半字节是**对象槽号**（`$0,$1,…` 对应 `$0400+slot*16`）。

| 类型 | 处理例程 | 记录长度 | 说明 | 证据 |
|---|---|---|---|---|
| 0 | `ExitSub` | — | 数据里 **0 次**（若出现会不前进指针） | 表项 0 |
| 1 | `LoadEnemy` | **3 B** | `[槽\|1][敌型][pos]`；pos 语义同结构记录的 y/x（`LEB4D` 另有 +8/+12 偏移） | `:8877`/`:8893` |
| 2 | `LoadDoor` | **2 B** | 见 #61 §1 | `:8876`/`:9015` |
| 3 | `ExitSub` | — | 0 次 | 表项 3 |
| 4 | `LoadElevator` | **2 B** | 见 #61 §3（原版数据 0 次） | `:8878`/`:9120` |
| 5 | `ExitSub` | — | 0 次 | 表项 5 |
| 6 | `LoadStatues` | **1 B** | 只读类型字节；雕像位置/状态内建（§4） | `:8879`/`:9153` |
| 7 | `LoadPipeBugHole` | **3 B** | `[槽\|7][虫型][pos]`；复用敌槽（§5） | `:8880`/`:9184` |

**记录长度来源**：每个处理例程返回「要加到指针的字节数」——`LoadEnemy` 走 `GetEnemyData` 尾部 `lda #$03`；
`SpawnDoorRoutine`/`SpawnElevatorRoutine` 尾部 `lda #$02`；`LoadStatues` 尾部 A 保持 `#$01`（`lda #$01; sta Statue.status` 之后直接
`jmp EnemyLoop`）；`LoadPipeBugHole` 尾部 `lda #$03`。**按固定 2 B 解析雕像记录会错位**（复核实测会产生假门/假电梯）。

### 3.2 `$FD` / `$FF` / `$FE`

| 字节 | 出现位置 | 语义 | 证据 |
|---|---|---|---|
| `$FD` | 结构段末尾 | 结构段结束、对象段开始；`EndOfRoomObjects` 把指针复制到 `$0000` 后逐个派发 | `:8850-8862` |
| `$FF` | 结构段或对象段 | 房间结束；`EndOfRoom` 把 `RoomNumber` 置 `$F0` 进入属性表阶段 | `:8886-8890`、`:8892` |
| `$FE` | 结构段 | 「空占位」：把 `RoomNumber` 置 `$FE` 并**只前进 1 字节** | `:8888-8891`、`:8836-8848` |

> **B 级提醒**：`rooms.asm` 头注释说「删掉结构就把对应 3 字节换成 `$FE`」，但代码 `$FE` 只前进 1 字节。
> 全五区数据里 `$FE` 出现 **0 次**（`.byte $FE` 计数 0），所以这条注释没被实际验证过；若要造数据，请写 **3 个 `$FE`**。

## 4. 雕像（`LoadStatues ; 07:EC2F`，A 级）

### 4.1 记录与生成

- **记录 1 字节**（类型 6）。全游戏**只有 1 条**：Brinstar Room43（房基址 `ABBF`），对象段 =
  `FD | 02 A0 | 02 B1 | 06 | FF` ⇒ 两条门 + 一条雕像记录（门见 #61；`06` 的高半字节=0 无意义）。
- `LoadStatues` 不读记录里的槽号，直接：
  ```
  Statue.hi ← GetNameTableAtScrollDir
  RidleyStatueY ← $40，若 RidleyStatueStatus 的 bit7 置位（已升起）则 $30
  KraidStatueY  ← $60，若 KraidStatueStatus  的 bit7 置位（已升起）则 $50
  StatuesBridgeIsSpawned ← 0
  Statue.status ← 1        ; 第一批背景 tile 更新
  ```
- 两个雕像共用**一个对象槽**（`Statue instanceof Object`，`constants_ram.asm:873`），x 由 `StatueXTable ; 07:DA39` 给定
  （Kraid `$88`、Ridley `$68`），y 由上面两个变量给定。

### 4.2 状态位（A 级：`$687B/$687C` 与密码 `$6997`）

| 变量 | 地址 | 位 | 含义 |
|---|---|---|---|
| `KraidStatueStatus` | `$687B` | bit7 | 雕像已升起（raised） |
| | | bit0 | Kraid 已击败（期间雕像闪烁） |
| `RidleyStatueStatus` | `$687C` | bit7 | 已升起 |
| | | bit1 | Ridley 已击败 |
| `PasswordByte+$0F` | `$6997` | bit7/6 | Kraid 升起/击败 |
| | | bit5/4 | Ridley 升起/击败 |
| `StatuesBridgeIsSpawned` | `$54` | — | 0=未生成桥；bit7=已生成 |
| `KraidStatueRaiseState`/`RidleyStatueRaiseState` | 紧随 `$687C` | bit7 | 升起动画已完成 |

- 击败写入：`LDD75 ; 07:DD75`（`:6046`）在 miniboss 死亡时 `InArea&$0F` 折半得到索引
  （Kraid `$12`→1、Ridley `$14`→2），`sta KraidStatueStatus-1,y` ⇒ 写 **1（bit0）** 让它闪烁。
- 密码落盘/解码：`prg0_title.asm:1886-1929` 把 `PasswordByte+$0F` 的 4 位还原成上面两个 status 的 bit7/bit0(bit1)。
- 升起：`UpdateStatue_Raise ; 07:DA3D`（`:5290`）在 `RaiseState` 未完成时每帧 `dec <StatueY>` 直到低半字节为 0；
  完成后 `RaiseState |= $80` 并 `sta <Status>`（置 bit7）⇒ 桥才有生成条件。
- 桥：`UpdateAllStatues_Bridge ; 07:DADA`（`:5426`）要求 `KraidStatueStatus & RidleyStatueStatus` 的 **bit7 都置位**，
  且 `DoorEntryStatus==0`；满足后置 `StatuesBridgeIsSpawned` 并用 **8 个 `TileBlasts`** 造桥（delay=8,10,…,22；
  type=`$04`；名表指针 `Statue.hi*4|$62`，行偏移 `y*2+8`）。
- 闪烁显示：`UpdateStatue ; 07:DA1A`（`:5262`）当 status 非 0 且 bit7 未置位时只在 `FrameCount` 为奇数帧显示。

### 4.3 与 Kraid/Ridley 房间的关系（B 级）

`GetEnemyType ; 07:EB28`：敌型字节 bit6 置位（Kraid/Ridley 本体）时，用 `InArea&$06>>1` 查 `KraidStatueStatus-1`
判断 mini-boss 是否已死；已死则**跳过加载**该敌人，未死则置 `KraidRidleyPresent=1`。
即：雕像状态独立于「房间记录里有没有该敌人」，两者通过 `InArea` 与 status 位耦合。

## 5. 管道虫洞（`LoadPipeBugHole ; 07:EC57`，A 级）

- **记录 3 字节**：`[槽|7][虫型（写进 status）][pos]`；高半字节=要复用的敌槽（`$0…$5`），低半字节类型固定 7。
- 全游戏 **42 条**：Br 6（R21×2/R22×2/R33/R35）、No 5（R23/R26×2/R35/R37）、Kr 16、To 0、Ri 15。
  样本：Br R21 `37 04 14`、R21 `17 04 20`、R22 `07 04 30`、`47 04 38`（字节见 §6.4）。
- 生成逻辑：在 `PipeBugHoles`（4 槽 ×8 B，`$0728`）里找 `status==$FF` 的空槽；写入 `enemySlot`（记录首字节高半字节）、
  `status`（虫型）、`y=(pos&$F0)|8`、`x=(pos<<4)|$00`、`hi=GetNameTableAtScrollDir`；指针前进 3 B。
- 与敌槽的关系：虫洞不是独立槽，它**占用被指定敌槽的精灵位**（`enemySlot` 字段），所以「槽号」不是“管道自己的槽”。
  `PipeBugHoles` 的 RAM 布局见 `crates/game-core/src/memmap/room_sprite_table_face.rs:11,154-199`（已 typed，B1-b 已有）。

## 6. 与现实现对照（事实表，不改代码）

| 面 | ROM | Rust 现状 | 判定 |
|---|---|---|---|
| 结构指针表/元 tile 表/结构流 | `AreaPointers_ROM` + `StructPtrTable`/`MetatileDefs` | `crates/game-data/src/assets.rs:234 struct_ptr`、`:241 struct_ptrs`、`:248 metatile_defs`、`:266 structures`；烘焙源 `crates/game-data/src/extract.rs:544-556`（Br/Norfair）、`:578-579`（Kraid）、`:597-598`（Tourian）、`:616-617`（Ridley）；`assets.rs:183` 起拼接 `BR_STRUCTS` | 已端口 |
| 结构绘制 | `DrawRoomObject`/`DrawStruct`/`DrawStructRow`/`DrawMetatile`/`UpdateAttrib` | `crates/game-core/src/world/room_draw.rs:172-246`（行头/元 tile/边界守卫 `:247-260`）；`metatile_defs` 按区取（`:219-226`） | 已端口 |
| `$FD`/`$FF`/`$FE` 分节 | `DrawRoom ; 07:EAAA` | `room_draw.rs` 同一函数（§1.1 注释块） | 已端口 |
| 雕像 | `LoadStatues` / `UpdateAllStatues` / `UpdateStatue` / 桥 | **未实现**（`grep -rni statue crates/` 只有 `sound/mod.rs` 的 `statue_raise_sfx_init`） | **未端口** |
| 管道虫洞 | `LoadPipeBugHole` + `PipeBugHoles` 表 | 只有内存面 typed：`crates/game-core/src/memmap/room_sprite_table_face.rs:11,154-199`（4×8 B）；**没有** 记录加载/生成逻辑 | **未端口（仅 RAM 面）** |
| 房间调色板/属性表 | `InitTables`/`AttribTableWrite` | `room_draw.rs` 内的属性表阶段（同文件 `$F0–$F4` 注释） | 已端口 |

**差异列表**
1. 雕像：Rust 无 `Statue` 对象、无 `KraidStatueStatus`/`RidleyStatueStatus` 镜像、无桥生成 —— B5 后续卡。
2. 管道虫洞：Rust 只有 RAM 槽 typed，无记录解析/生成 —— 后续卡。
3. `attr` 与 `RoomPalette` 相等的跳过优化：Rust `room_draw` 已按同样的比较实现（见 §6 命令输出），无差异。

## 7. 复现命令（每条可原样跑，输出已核对）

```bash
# 7.1 结构记录 3 字节（Br R00 前 4 条 + 房基址/调色板）
python3 - <<'PY'
import pathlib
rom=pathlib.Path('rom/Metroid (USA).nes').read_bytes(); prg=rom[16:16+128*1024]
def rd(b,a,n): return prg[b*0x4000+(a-0x8000):][:n]
d=rd(1,0xA441,60); print('palette',d[0]); 
for i in range(4): print(d[1+i*3:1+i*3+3].hex(' '))
PY

# 7.2 结构行流解码（Br 结构 1 = AC97）
python3 - <<'PY'
import pathlib
rom=pathlib.Path('rom/Metroid (USA).nes').read_bytes(); prg=rom[16:16+128*1024]
def rd(b,a,n): return prg[b*0x4000+(a-0x8000):][:n]
SP=0xAC97; d=rd(1,SP,40); k=0
while d[k]!=0xFF:
    n=(d[k]&0x0F) or 16; xo=d[k]>>4; print('row n=',n,'xoff=',xo,'mets',list(d[k+1:k+1+n])); k+=1+n
PY

# 7.3 元 tile 定义与顺序（Br 元 tile 02/04/06）
python3 - <<'PY'
import pathlib
rom=pathlib.Path('rom/Metroid (USA).nes').read_bytes(); prg=rom[16:16+128*1024]
def rd(b,a,n): return prg[b*0x4000+(a-0x8000):][:n]
for idx in (0x02,0x04,0x06): print(hex(idx), rd(1,0xAEF0+idx*4,4).hex(' '))
PY

# 7.4 全五区对象段类型计数（雕像 1 / 管道 42 / 电梯 0）
python3 - <<'PY'
import pathlib
rom=pathlib.Path('rom/Metroid (USA).nes').read_bytes(); prg=rom[16:16+128*1024]
def rd(b,a,n): return prg[b*0x4000+(a-0x8000):][:n]
def word(b,a):
    x=rd(b,a,2); return x[0]|(x[1]<<8)
A=[('Br',1,0xA314,47),('No',2,0xA21B,46),('Kr',4,0xA1D5,37),('To',3,0xA7D1,21),('Ri',5,0xA17F,42)]
LEN={1:3,2:2,4:2,6:1,7:3}
for name,b,t,n in A:
    ps=[word(b,t+2*i) for i in range(n)]; c={'1':0,'2':0,'4':0,'6':0,'7':0}
    for p in ps:
        nx=min([q for q in ps if q>p]+[max(ps)+0x400]); d=rd(b,p,nx-p); j=1
        while d[j]!=0xFF:
            if d[j]==0xFD: j+=1; break
            if d[j]==0xFE: j+=1; continue
            j+=3
        while d[j]!=0xFF:
            t2=d[j]&0x0F
            if t2 not in LEN: break
            c[str(t2)]+=1; j+=LEN[t2]
    print(name,c)
PY

# 7.5 雕像房（Br R43）对象段与 status/密码常量
python3 - <<'PY'
import pathlib
rom=pathlib.Path('rom/Metroid (USA).nes').read_bytes(); prg=rom[16:16+128*1024]
def rd(b,a,n): return prg[b*0x4000+(a-0x8000):][:n]
d=rd(1,0xABBF,110); j=1
while d[j]!=0xFD: j+=3
print('obj@+%d'%j, d[j:j+8].hex(' '))
PY
grep -n "KraidStatueStatus  \|RidleyStatueStatus  \|StatuesBridgeIsSpawned" refs/m1disasm/SRC/constants_ram.asm
```

## 8. 假设区（C 级）

- **H1**：`$FE` 空占位的正确用法。代码只前进 1 字节，`rooms.asm` 头注释却说「替换 3 字节」；数据里 0 次出现，两者未被实测调和。
  （建议造数据时写 `$FE ×3`。）
- **H2**：雕像升起阈值（`RaiseState` 何时从 0→1）。`UpdateStatue_Raise` 只在 `RaiseState` 的 bit7 未置位且 statue 未升起时工作；
  触发条件（是否要求两个 mini-boss 都死）未逐帧 trace，仅从桥条件反推。
- **H3**：`Statue` 与 `Objects` 槽的关系（`lda #Statue - Objects` 作 `PageIndex`）——雕像占用了哪个 `$0400` 槽未在数据里指定，
  由编译期 `Objects` 结构布局决定；Rust 侧没有对应槽，故无法直接对照。

## 9. 未覆盖

1. 属性表两行写入的**帧相位**（`AttribTableWrite` 在 `SetupRoom` 跨帧被调用，具体哪帧写哪行）未 trace。
2. `UpdateStatueBGTiles`（低/高雕像背景 tile 批）的具体 tile 数据与批次数未解码。
3. `TileBlasts`（桥的 8 个爆炸块）与桥瓦片的最终外观未逐字节核对。
4. 管道虫洞的「虫型」取值枚举（记录第 2 字节）未逐条对照敌人表。
5. Tourian/Ridley/Kraid 的**逐房**结构记录只抽样，未全量列表（全量用 §7.4 命令输出）。
6. 结构流里 `$FF` 出现在元 tile 位置时 ROM 的行为（`DrawStructRow` 不判，按索引读）未取证。

## 10. 证据指针

| 结论 | 指针 |
|---|---|
| 结构记录 3 字节 / 位置单位 / 结构指针查表 | `refs/m1disasm/SRC/prg7_engine.asm:8766-8848`（DrawRoomObject） |
| 分节 `$FF/$FE/$FD` | `prg7_engine.asm:8886-8896`（DrawRoom）、`:8850-8862`（EndOfRoomObjects） |
| 结构行流/元 tile 顺序/属性跳过 | `prg7_engine.asm:9760-9838`（DrawStructRow/DrawStruct/@DrawMetatile）、`:9886`（TilePosTable）、`:9976-9999`（UpdateAttrib 与掩码表） |
| 对象类型 JumpEngine 表 | `prg7_engine.asm:8870-8880` |
| 记录长度（各例程返回 A） | `GetEnemyData :8927`（`lda #$03`）、`SpawnDoorRoutine :9119`（`lda #$02`）、`SpawnElevatorRoutine :9120-9148`、`LoadStatues :9153-9182`、`LoadPipeBugHole :9215-9224`（`lda #$03`） |
| 雕像 status 与密码 | `constants_ram.asm:569,1234-1235,1275`；`prg0_title.asm:1886-1929`；`prg7_engine.asm:6046-6056`（击败写 bit0） |
| 雕像升起/桥 | `prg7_engine.asm:5191-5280`（UpdateAllStatues 与 UpdateStatue）、`:5290-5320`（UpdateStatue_Raise）、`:5426-5465`（UpdateAllStatues_Bridge） |
| 管道虫洞 RAM 面 | `crates/game-core/src/memmap/room_sprite_table_face.rs:11,154-199` |
| 区表/元 tile 表 | `refs/m1disasm/SRC/prg{1,2,3,4,5}_*.asm` 的 `AreaPointers_ROM` 段；`crates/game-data/src/extract.rs:544-556,578-579,597-598,616-617` |
| Rust 绘制 | `crates/game-core/src/world/room_draw.rs:172-260` |
| 门计数更正（Br 29 vs #61 的 27） | `refs/m1disasm/SRC/brinstar/rooms.asm:83,248`（裸 `.byte $02,$A1`/`.byte $02,$B1`） |
| ROM 身份 | `sha256 e6e6b701…65b6` |
