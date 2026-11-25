# 房间数据格式与落位链（B5 前置规格）

> 只读考证：ROM 字节（`rom/Metroid (USA).nes`，sha256 `e6e6b7014685adae447ebb3833242815747bc1e5df83ade79f693fb67cf565b6`）
> + 社区匹配反汇编（`refs/m1disasm/SRC/`）+ 现实现（`crates/`）。**本文件不改任何代码。**
> 证据级别：**A** = 字节偏移/汇编行逐条一致；**B** = 代码语境推断；**C** = 未取证（只在 §6 假设区）。
> 卡面写「`InitRoom`」；本反汇编**没有**这个符号，实际入口是 **`SetupRoom ; 07:EA2B`**（§3 给调用链）。
> 卡面还假设记录里有「宽/高/scroll/房间类型/敌人数/门数」字段——实测**不存在**，见 §1.3 的负面结论。

## 0. 一句话结论

房间数据是**变长字节流**：`[调色板 1B] [结构记录 3B×n] $FD [敌/门记录 2–3B×m] $FF`；
房间没有尺寸/敌数/门数头，宽高与滚动范围由**世界地图 + 名表切换**决定（§1.6）。
敌/门记录写到 `Ens($0400+slot*16)` / `EnsExtra($6AF4+slot*16)`，坐标编码 `%yyyyxxxx`
经 `LEB4D` 变成 `y=(pos&$F0)|8`、`x=(pos<<4)|$0C`（A 级，§2.5）。
道具/特殊物品走**另一张链表**（`SpecItmsTbl`），布局见 §2.1–§2.3。

## 1. 房间表

### 1.1 指针表（A 级：asm 标签地址 + ROM 字节）

| 区 | `RoomPtrTable` | 项数 | 房间数据区 | 结构表 | 特殊物品表 |
|---|---|---|---|---|---|
| Brinstar | `01:A314` | **47** | `01:A441`–`01:AC84` | `01:A372`(50) | `01:A3D6` |
| Norfair | `02:A21B` | **46** | `02:A3AB`–`02:ACB9` | `02:A277`(49) | `02:A2D9` |
| Kraid | `04:A1D5` | 37 | `04:A2B7`–`04:AA33`+ | `04:A1DF` 附近 | `04:A26D` |
| Tourian | `03:A7D1` | 21 | — | — | `03:A83B` |
| Ridley | `05:A17F` | 42 | — | — | `05:A20D` |

换取规则（A 级）：CPU `$8000-$BFFF` → PRG 偏移 `bank*0x4000 + (addr-$8000)`。
`bank1` 的 `$A314` ⇒ `0x4000+(0xA314-0x8000)=0x6314`。各区项数用 `crates/game-data/src/assets.rs:180-217`
的烘焙表交叉核对一致（Br47/No46/Kr37/To21/Ri42）。

### 1.2 记录布局（A 级：`SetupRoom` / `DrawRoom` / `EndOfRoomObjects` 逐行）

```
[0]      调色板 palette（→ RoomPalette $68；低 2 位选 ATDataTable $F018 的 00/55/AA/FF）
[i..]    结构记录 × n，每条 3 B：
           B0 = %yyyyxxxx（y 高半字节、x 低半字节，单位=2 tile/行、2 tile/列）
           B1 = 结构号（索引 StructPtrTable）
           B2 = 属性表字节（与 palette 同值则跳过 ATTRIB 更新：UpdateAttrib 早退）
         终止符：$FD（转入敌/门环）
[敌/门]  记录 × m：
           B0 = 高半字节=对象槽号、低半字节=类型码（1 敌 / 2 门 / 4 电梯 / 6 雕像 / 7 管虫孔）
           敌：B1=敌型（bit7=tough、bit6=miniboss 门禁）、B2=%yyyyxxxx（→ LEB4D）；**3 B**
           门：B1=门信息（bit7-5 定侧、bit1-0 门色/类型、bit4 …由 SpawnDoorRoutine 用）；**2 B**
         终止符：$FF（→ EndOfRoom）
$FE      空占位（旧房删除留下的洞，跨 1 B 后继续）
$F0-$F4  房号不是「房」而是**命令**：SetupRoom 走 AttribTableWrite（$EA13）逐行写 PPU 属性表
```

**记录长度是变长的**，只能由「下一房指针 − 本房指针」求（最后一房到结构表首址）。

### 1.3 字段表（逐字段证据级别）

| 字段 | 偏移/编码 | 级别 | 证据 |
|---|---|---|---|
| 房间调色板 | B0，`RoomPalette←(RoomPtr),y` | **A** | `prg7_engine.asm:8708-8710`（RoomPalette 读入）；`InitTables` 用它选 ATDataTable |
| 结构 y/x | B0 `%yyyyxxxx` | **A** | `DrawRoomObject 07:EA60`（`Adiv16`→y 循环 +0x40/行，`and #$0F; asl`→x 每单位 2 tile） |
| 结构号 | B1 | **A** | 同上：`asl; tay; lda (StructPtrTable),y` |
| 结构属性 | B2 | **A** | `ObjectPalette`，`UpdateAttrib` 与 RoomPalette 相同则早退 |
| 对象槽号 | B0 高半字节 | **A** | `GetEnemyData 07:EB0C`：`and #$F0; tax` |
| 对象类型码 | B0 低半字节 | **A** | `DrawRoom 07:EAD6`：`and #$0F; jsr JumpEngine`（8 项表见 §3.4） |
| 敌型 | B1 | **A** | `GetEnemyType 07:EB28`：bit7→`specialAttribs`，bit6→miniboss 存活门禁，`and #$3F`→`EnsExtra.type` |
| 敌坐标 | B2 | **A** | `LEB4D 07:EB4D`：`y=(pos&$F0)|8`、`x=(pos<<4)|$0C`、`status=1`、`isHit=0`、`hi=(PPUCTRL^ScrollDir)&1` |
| 敌数量 | **无字段** | **A（负面）** | 数量=敌记录条数；`EnemyLoop` 逐条读至 `$FF` 才停（`07:EAD4-07:EAF3`） |
| 门 | B1（侧/色/类型） | **A** | `SpawnDoorRoutine 07:EB92`：`Amul16` 取侧、`and #$03` 门型、`DoorSlots` 索引 |
| 房间宽/高 | **无字段** | **A（负面）** | `SetupRoom` 全径只读 palette+对象；`DrawRoomObject` 的行进上限是 RoomRAM 0x3C0 边界，不是房宽 |
| 滚动范围/scroll | **无字段** | **A（负面）** | 由 `MapPosX/Y`+`WorldMap($00:A53E 32×32)`+`ScrollDir` 决定；见 §1.6 |
| 房间类型（普通/电梯/雕像） | **无字段** | **A（负面）** | 由**门/电梯记录**（类型码 2/4/6）与 `AreaRoutine` 表决定 |

### 1.4 记录长度统计（本章实测，可复跑）

| 区 | 房间数 | 数据区跨度 | 最短 | 中位 | 最长 | 合计 |
|---|---|---|---|---|---|---|
| Brinstar | 47 | `A441`–`AC84` = **2115 B** | 8 B | 44 B | 101 B | 2115 B |
| Norfair | 46 | `A3AB`–`ACB9` = **2318 B** | 8 B | 51 B | 74 B | 2318 B |

最短的 8 B 房（Br Room01 / No Room01）就是「一个调色板 + 一个结构 + `$FF`」的空房。

### 1.5 样本房逐字节解码（A 级，hexdump 见 §5）

**Brinstar Room00 `01:A441`**（19 B）

```
02 | 40 01 02 | 48 01 02 | 50 03 02 | 5F 03 02 | FD | 02 A1 | 02 B1 | FF
```
调色板=2；4 个结构：(y4,x0) 结构1、(y4,x8) 结构1、(y5,x0) 结构3、(y5,xF) 结构3，属性都=2（=palette ⇒ 属性写早退）；
`$FD` 后两条门记录（各 2 B）：槽0 门信息 `A1`、槽0 门信息 `B1`（`$A0|侧<<4|蓝门`）；
`$FF` 结束。与 `brinstar/rooms.asm` 的 `.byte re_Door, $A0|...` 两行逐字节一致。

**Brinstar Room02 `01:A45C`**（36 B，带 5 敌）

```
03 | 7×结构(结构号 0B) | FD | 01 03 42 | 11 83 8A | 21 03 B5 | 31 02 59 | 41 02 A3 | FF
```
7 条结构记录（`00 0B 03`…`AE 0B 03`）都是结构号 `0B`、属性 3；`$FD` 后 5 条敌记录，槽 0–4；
敌型 `03/83/03/02/02`（`83`=tough Ripper）；坐标 `42/8A/B5/59/A3`
→ 落位 `(y48,x2C) (y88,xAC) (yB8,x5C) (y58,x9C) (yA8,x3C)`（§2.5 公式）。
与 asm 注释 `Ripper/Waver` 行数、顺序一致。

**Norfair Room02 `02:A3C6`**（50 B）**没有 `$FD` 段**：16 条结构记录后直接 `$FF`。

```
03 | 16×结构 | FF
```
结构号序列 `0B/06/06/0B/07/07/08/08/07/09/07/06/06/0A/06/06`，属性字节 `03/03/03/03/03/03/01/01/03/00/03/03/03/03/03/03`
——属性 `00` 与调色板 `03` 不同 ⇒ 该条会真的写 ATTRIB（§1.2 的早退不触发）。
这条房说明：**同一房内可以没有敌/门段**，`$FD` 不是必需项（A 级：`DrawRoom` 结构环遇 `$FF` 直接进 `EndOfRoom`）。

### 1.6 房间「宽高/滚动」真正的来源（A 级）

- `GetRoomNum 07:E720`：`$0000 = MapPosY*32 + MapPosX + $7000`（WorldMap），取出房号写 `$5A`；`$FF`=无房。
- `GetNameTableAtScrollDir 07:EB85`：`(PPUCTRL_ZP ^ ScrollDir) & 1` ⇒ 名表 0（RoomRAMA `$6000`）或名表 3（RoomRAMB `$6400`）。
- `EndOfRoomHorizontal 07:E70C`：`Temp00_RoomRAMPtr = (ScrollX & $F8)/8` → `UpdateNameTable` 逐格补流送。
- 所以「一个房间」=**一张 32×30 名表**；跨两张名表的「大房间」是**门/走廊把两张名表连起来**，不是记录里有尺寸字段。
  `SelectRoomRAM` 只决定这次画进哪张。

## 2. 初始台 / 生成表

### 2.1 特殊物品表（道具、电梯、雕像、Rinka…）（A 级）

| 区 | `SpecItmsTbl` | 指针存放 |
|---|---|---|
| Brinstar | `01:A3D6` | `AreaPointers_ROM+$9598`（`prg1_brinstar.asm:85-86`） |
| Norfair | `02:A2D9` | `prg2_norfair.asm:83-84` |
| Kraid | `04:A26D` | `prg4_kraid.asm` 同址 |
| Tourian | `03:A83B` | `prg3_tourian.asm:83-84` |
| Ridley | `05:A20D` | `prg5_ridley.asm:91-92` |

布局是**两级链表**（A 级：`ScanForItems 07:ED98` + `items.asm` 注释）：

```
Y 节点： [y][nextLo][nextHi]            ; 按 MapPosY 升序，next=指针或 $FF.. 结束
X 链  ： [x][delta|$FF][type,sub,param][$00]  ; delta=到下一个 x 条目的字节数；$FF=本 y 行最后一个
```
`type`（低 4 位，`ChooseSpawningRoutine 07:EDD6` 的 11 项表）：
`1`=Squeept、`2`=PowerUp、`3`=Mellows/Memus、`4`=Elevator、`5`=MotherBrain 炮、`6`=MotherBrain、
`7`=Zebetite、`8`=Rinka、`9`=门、`$A`=调色板切换。

**Brinstar 首节点实测**（`01:A3D6`，**14 B**；§5 命令多打的 2 B 是下一个 Y 节点开头）

```
02 E4 A3 | 03 05 04 03 00 | 0F FF 02 05 37 00
```
=y 2、下一 Y 节点 `A3E4`；x=3 处的条目（delta 5）是 `type 4`（电梯）+ 参数 `$03`（去 Tourian）；
x=0x0F 处（delta `$FF`=末项）是 `type 2`（PowerUp）+ 子型 `$05`（Varia）+ 参数 `$37`。

### 2.2 与 `$0400` 敌槽的逐字段对应（敌记录 3 B → 12 字段）

| 记录字节 | 写点（asm） | 目标 | 值 |
|---|---|---|---|
| B0 高半字节 ×16 | `GetEnemyData 07:EB0C` `tax` | 槽基址 `$0400+x` | 槽 0–15 地址面；**实分配 12 槽**（`enemy/state.rs` 的 `SLOT_COUNT=12`，`$6AF4+12*16=$6BB4` 在 EnsExtra 区内；B 级） |
| B1 bit7 | `07:EB32` `and #$C0` | `Ens+$F` `specialAttribs` | `$80`=tough（血×2，Ripper 除外） |
| B1 bit6 | `07:EB35-07:EB45` | `KraidRidleyPresent` | miniboss 未死才装载，否则整条跳过 |
| B1 & `$3F` | `07:EB46-07:EB4B` | `EnsExtra+$0E` `type` | 0–63，索引敌表 |
| B2 | `LEB4D 07:EB4D-07:EB5A` | `Ens+0 y` / `Ens+1 x` | `y=(pos&$F0)|8`、`x=(pos<<4)|$0C` |
| — | `07:EB5C` | `EnsExtra+0 status` | `1`（`enemyStatus_Resting`） |
| — | `07:EB60` | `Ens+4 isHit` | 0 |
| — | `07:EB63-07:EB68` | `EnsExtra+7 hi` | 名表位 |
| — | `LEB6E 07:EB6E` | `data05`（`asl`+`ror`：bit7←L977B[type].bit7） | 朝向/竖向位 |
| — | `InitEnemyFacingDirectionAxisAndDelay` | `Ens+9 delay` | `InitDelayTbl[type]` |
| — | `InitEnemyForceSpeedTowardsSamusDelayAndHealth` | `Ens+$D data0D`、`Ens+$B health` | ForceTbl / HealthTbl（tough 双倍） |

**未写**：`movementIndex($0408)`、`speedY/speedX/subX` —— 出生径不碰（A 级：`GetEnemyData` 全径 `sta abs,x`
只出现 `$040F/$0400/$0401/$0404` 与 `EnsExtra $6AF4/$6AFB`；与 `room_draw.rs:823-900` 头注一致）。

### 2.3 槽数上限

- 地址面：记录 B0 高半字节 4 bit ⇒ **16 个可能的槽地址**（`$0400`…`$04F0`）。
- 实体面：RAM 只给 **12 槽**（`$0400`–`$04FF` 被 12 槽占 192 B，`EnsExtra` 同理）；B0 高半字节 >11
  的记录会写到未分配区/越界——**ROM 数据里是否出现过 >11 未取证**（§6 H2）。

## 3. 调用链（进房 → 写槽，A 级地址）

```
滚动/过门
  └ ScrollUp  07:E4F0s  ┐
  └ ScrollDown 07:E528s ├ jsr GetRoomNum (07:E720) → RoomNumber($5A) → CheckUpdateNameTable (07:E54A)
  └ ScrollLeft/Right ───┘        │
                                 └ jsr SetupRoom  ← **入口 07:EA2B**（xref=2：07:1894 MoreInit 环 / 07:7703 CheckUpdateNameTable）
                                      ├ RoomNumber=$FF ⇒ rts（未定义房）
                                      ├ RoomNumber=$FE ⇒ jmp DrawRoom（空占位）
                                      ├ RoomNumber≥$F0 ⇒ AttribTableWrite 07:EA13（逐行写 PPU 属性表，$F0..$F4 五次一条链）
                                      └ 正常房：
                                         DeleteOffscreenRoomSprites 07:EC9B
                                         ScanForItems            07:ED98   ← 特殊物品（§2.1）
                                         RoomPtrTable($3B/$3C 间接)→RoomPtr($33/$34)
                                         RoomPalette ← (RoomPtr),y
                                         SelectRoomRAM 07:EA05  → RoomRAMPtr $6000/$6400
                                         InitTables    07:EFF8  → 填 $FF + 属性表底色
                                         DrawRoom      07:EAAA
                                             结构环 → $FD → EndOfRoomObjects 07:EACA
                                                          → EnemyLoop 07:EAD4（JumpEngine 8 项）
                                                               1 LoadEnemy 07:EB06 → GetEnemyData/LEB4D（§2.2）
                                                               2 LoadDoor  07:EB8C → SpawnDoorRoutine 07:EB92
                                                               4 LoadElevator 07:EC04
                                                               6 LoadStatues   07:EC2F
                                                               7 LoadPipeBugHole 07:EC57
                                                          → $FF → EndOfRoom 07:EAF4：RoomNumber←$F0
```

**xref 计数（本章实测）**：`SetupRoom` 2 处（`prg7_engine.asm:1894`、`:7703`）；`GetRoomNum` 4 处
（`:7650` 上滚、`:7679` 下滚、`:7967` 左滚、`:7998` 右滚）；`ScanForItems` 1 处（`:8703`）；
`SelectRoomRAM` 1 处（`:8717`）；`InitTables` 1 处（`:8718`）；`DrawRoom` 1 处（`:EA5D` 的 `jmp`）。

## 4. 与现实现对照（只列事实，不改代码）

| 面 | ROM | Rust 现状 | 判定 |
|---|---|---|---|
| 房指针表/流 | `$A314` 等，运行时 `lda (RoomPtrTable),y` | `crates/game-data/src/assets.rs:180-217`（烘焙 `&[u8]`，头注自证与 ROM 逐字节同）；`assets.rs:227 room_ptr` | 等价（烘焙 vs 运行查表） |
| 载房壳 | `SetupRoom 07:EA2B` | `crates/game-core/src/world/room_load.rs:306 setup_room`（306-356；`$FF/$FE/≥$F0` 三分支、`room_ptr` 332、`palette` 345、`init_tables` 350、`draw_room` 352） | 已端口 |
| 结构环 / `$FD` / `$FE` | `DrawRoom 07:EAAA` | `world/room_draw.rs:95 draw_room`（`0xFF` :129 / `0xFE` :137 / `0xFD` :141 / 结构 :150-168） | 已端口 |
| 敌记录 3 B | `GetEnemyData`+`LEB4D` | `room_draw.rs:823 load_enemy`（字段写点逐行对齐 §2.2：`en+F` specialAttribs、`ex+0E` type、`en` y、`en+1` x、`ex+0` status、`en+4` isHit、`ex+7` hi、`en+5` data05、`en+9` delay、`en+D` data0D、`en+B` health） | 已端口 |
| 门记录 2 B | `SpawnDoorRoutine 07:EB92` | `room_draw.rs` 对象环 type 2 分支 | 已端口 |
| 电梯/雕像/管虫（4/6/7） | `LoadElevator/LoadStatues/LoadPipeBugHole` | `room_draw.rs:734-760`（`load_elevator` :784、`world/statue::load_statues`、`enemy::pipe_bug_hole::`） | 已端口 |
| 特殊物品表 | `ScanForItems 07:ED98` | `crates/game-core/src/pickup/spawn.rs:38 scan_for_items` | 已端口 |
| 鞘槽字段名 | `Ens`/`EnsExtra` 16 B | `crates/game-core/src/enemy/state.rs:19-58`（`ens::Y/X/.../WEAPON_ACTION/SPECIAL_ATTRIBS`）、EnsExtra 偏移表 | 已端口 |
| 「InitRoom」符号 | **不存在** | — | 卡面术语与实现术语不同（§0） |

**差异/待确认（不是缺陷，是口径）**：
1. Rust 把房流烘焙进资产包；ROM 是运行时查 `RoomPtrTable`。若将来改 ROM 内容需要重跑烘焙（`game-data` 提取器），
   否则会出现「ROM 改了、Rust 没变」的分叉——这是资产管线的固有约束，不是逻辑差。
2. Rust 的 `draw_room` 返回 `false` 时 `setup_room` 调 `unported!("bank07:EAD4")`；上面 4/6/7 三类已实现，
   但 `draw_room` 里仍有 `$EA??` 未覆盖类型码的 fail-loud 臂（`room_draw.rs:734-760` 注释）：B 级，未逐类型码穷举。
3. `SLOT_COUNT=12` 与记录高半字节 4 bit 的「16 地址槽」是两套口径（§2.3），移植时槽号 ≥12 的记录行为**未取证**。

## 5. 复现命令（每条都可原样跑，输出见括号）

```bash
# 1) 两个样本房逐字节（Br Room00 / Br Room02 / No Room02）
python3 - <<'PY'
import pathlib
rom=pathlib.Path('rom/Metroid (USA).nes').read_bytes(); prg=rom[16:16+128*1024]
def rd(b,a,n): o=b*0x4000+(a-0x8000); return prg[o:o+n]
def hx(x): return ' '.join(f'{v:02X}' for v in x)
print(hx(rd(1,0xA441,19)))   # 02 40 01 02 48 01 02 50 03 02 5F 03 02 FD 02 A1 02 B1 FF
print(hx(rd(1,0xA45C,36)))   # 03 ... FD 01 03 42 11 83 8A 21 03 B5 31 02 59 41 02 A3 FF
print(hx(rd(2,0xA3C6,50)))
PY

# 2) 记录长度统计（Br 47 / No 46；末房到结构表首址）
python3 - <<'PY'
import pathlib,statistics
rom=pathlib.Path('rom/Metroid (USA).nes').read_bytes(); prg=rom[16:16+128*1024]
def rd(b,a,n): o=b*0x4000+(a-0x8000); return prg[o:o+n]
for name,bank,ptr,n,end in (('Br',1,0xA314,47,0xAC84),('No',2,0xA21B,46,0xACB9)):
    t=rd(bank,ptr,2*n); a=[t[i]|(t[i+1]<<8) for i in range(0,2*n,2)]
    L=[(a[i+1] if i+1<n else end)-a[i] for i in range(n)]
    print(name,len(a),hex(a[0]),hex(a[-1]),min(L),statistics.median(L),max(L),sum(L))
PY    # Br 47 0xa441 0xac6a 8 44 101 2115 / No 46 0xa3ab 0xac99 8 51.0 74 2318

# 3) 坐标编码验算（LEB4D：y=(pos&$F0)|8, x=(pos<<4)|$C）
python3 -c "pos=0x8A;print(hex((pos&0xF0)|8),hex(((pos<<4)&0xFF)|0x0C))"   # 0x88 0xac

# 4) 特殊物品首节点（Br 01:A3D6）
python3 - <<'PY'
import pathlib
rom=pathlib.Path('rom/Metroid (USA).nes').read_bytes(); prg=rom[16:16+128*1024]
o=1*0x4000+(0xA3D6-0x8000); print(' '.join(f'{v:02X}' for v in prg[o:o+16]))
PY    # 02 E4 A3 03 05 04 03 00 0F FF 02 05 37 00 03 F3

# 5) 指针表项数（每区 `.word` 数，与烘焙表交叉）
grep -c '\.word Room' refs/m1disasm/SRC/brinstar/room_ptrs.asm refs/m1disasm/SRC/norfair/room_ptrs.asm
```

## 6. 假设区（C 级，未取证，不得当结论引用）

- **H1**：`$F0-$F4` 五次链 =「一张名表 4 行属性表 + 1 次收尾」逐行写，`and #$0F` 当行号；链尾 `RoomFinished` 置 `$FF`。
  代码路径已读（`AttribTableWrite 07:EA13`），但**未用 trace 对齐**帧级时序。
- **H2**：记录 B0 高半字节 ≥12 的槽号从未出现在原版数据里（只统计了 Br/No 两区的房，未全量扫描五区）。
- **H3**（原假设已否证，保留此处备查）：初稿把 Norfair Room02 的 `96 09 00` 当成「电梯记录（低半字节 9）」，
  逐字节复核后确认它是**第 10 条结构记录**（y=$96、结构号 `09`、属性 `00`），该房根本没有 `$FD` 段。
  `$EA??` JumpEngine 的奇数项行为（表条目 0/3/5 都是 `ExitSub`）**仍未逐类型码取证**，留作 B 级。
- **H4**：`$FE` 占位会不会导致「房号计数错位」（即 `$FE` 占一个房间号）——`DrawRoom` 里 `$FE` 只置 `RoomNumber←$FE`
  再 `AddToRoomPtr 1`，未见房间号自增；推断仅占字节，未用 trace 验证。

## 7. 未覆盖

1. Tourian / Ridley / Kraid 三区的房记录**未逐房解码**（只给指针表地址与项数）。
2. 结构定义（`structures.asm`：行头 nibble + 元 tile 索引 + `$FF`）只听 `DrawStruct` 代码，未做字节级样本表。
3. 元 tile 表（4 B/组）与 PPU 写入细节未展开（#42 时序件已覆盖 `$2007` 侧）。
4. `LoadStatues`（雕像）字段级布局未取证（与 §2.2 的敌记录共享 3 B 形状？未确认）。
5. 五区特殊物品表只解了 Brinstar 首节点。

## 8. 证据指针

| 结论 | 指针 |
|---|---|
| 房指针表/项数 | `refs/m1disasm/SRC/{brinstar,norfair,kraid,tourian,ridley}/room_ptrs.asm`；`crates/game-data/src/assets.rs:180-217` |
| 记录布局 | `refs/m1disasm/SRC/prg7_engine.asm:8693`（SetupRoom）、`:8724`（DrawRoomObject）、`:8808`（DrawRoom 环）、`:8844`（EndOfRoomObjects）、`:8875`（EndOfRoom）、`:8893`（GetEnemyData）、`:8913`（GetEnemyType）、`:8937`（LEB4D） |
| 门字段 | `prg7_engine.asm:9015`（LoadDoor）、`:9020`（SpawnDoorRoutine）、`:9120`（LoadElevator） |
| 属性表命令链 | `prg7_engine.asm:8672`（AttribTableWrite）+ `:7825`（WritePPUAttribTbl） |
| 特殊物品 | `prg7_engine.asm:9428`（ScanForItems）+ `brinstar/items.asm:40-90` |
| 房号与地图 | `prg7_engine.asm:8046`（GetRoomNum）、`prg0_title.asm:5118`（WorldMap `00:A53E`） |
| 名表选择/流送 | `prg7_engine.asm:8657`（SelectRoomRAM）、`:8718`（InitTables） |
| Rust 对照 | `crates/game-core/src/world/room_load.rs:306-356`；`world/room_draw.rs:95-170,823-900`；`enemy/state.rs:19-58`；`pickup/spawn.rs:38` |
| ROM 身份 | `sha256 e6e6b701…65b6`（本文所有 hexdump 的 ROM） |
