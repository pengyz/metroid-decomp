# 敌人生成与槽初始化（B5-d 规格）

> 只读考证：ROM 字节 + 社区匹配反汇编（`refs/m1disasm/SRC/`）+ 现实现（`crates/game-data`、`crates/game-core`）。
> **本文件不改代码。** 证据级：**A**=字节/汇编逐条一致；**B**=代码语境推断；**C**=未取证（§7）。
> 上游：`docs/spec/room-data-format.md`（#59，房间记录/落位）、`docs/spec/room-structures.md`（#62，对象段 3B 类型表）。

## 0. 结论

1. **原版没有「敌人生成表」这种静态表**（按房号索引的生成清单）。敌人来源只有两条，且共用同一个 3 字节记录格式：
   - **房间对象段里的 type 1 记录**（`$FD` 之后）：全五区按**去重后的房间**计共 **377 条**（Br113 / No102 / Kr48 / To20 / Ri94；Kr/Ri 的指针表有重复项，见 §1.1 末）；
   - **Q 弹（Squeept）被击破时的随需生成**：`SpawnMapEnemy ; 07:EDF8` 直接调 `GetEnemyData`，数据从「特殊物品表」的记录读，格式与房间记录同源（首字节高半字节=槽、次字节=敌型、三字节=位置）。
   - 反证：`GetEnemyData` 全仓只有 **3** 处引用（定义 + `LoadEnemy` + `SpawnMapEnemy`），没有任何「按房号查敌数」的表；遍历 380 条记录即可完整复现每房敌人集合。
2. **记录里没有「数量」「延迟」字段**。数量 = 记录条数（同槽重复记录会被 `IsSlotTaken` 跳过）；延迟 = 每型一张 `EnemyInitDelayTbl`。
3. **每型属性表按区各一份、每份 16 项**（敌型 = 记录第 2 字节 `& $3F`，实际只 0–0x0A 有值）：地址固定在同一偏移
   `$962B`(血) / `$963B`(静止动画,2B×16) / `$965B`(活动动画,2B×16) / `$967B`(动画初偏移,2B×16) / `$968B`(AI 位组) /
   `$969B`(data0D 初值) / `$96AB`(距离阈值) / `$96BB`(初始延迟) / `$96CB`(移动选择偏移) / `$977B`(朝向/轴)，
   在 bank 1/2/3/5 原样存在；**bank 4（Kraid）的 `$969B` 是 `EnemyData0DTbl`**（值全 1，0x0C 为 `$40`），角色等价但表名不同。
4. **槽只有 6 个**：`Ens instanceof En 6 startfrom 0`（`constants_ram.asm:972`，`$0400`–`$045F`，16 B/槽），
   `EnsExtra instanceof EnExtra 6`（`:1294`，`$6AF4`–`$6B53`）；`$0400` 整块 16 槽 = Ens 6 + EnProjectiles 6 + EnExplosions 4（到 `$04FF`）。
   记录首字节高半字节就是槽号 `$00`–`$50`（×16）。
5. **槽初始化的写者只有 4 处**（A 级，见 §3.2）：`GetEnemyType($EB28)`、`LEB4D($EB4D)`、`LEB6E($EB6E)`→`InitEnemyFacingDirectionAxisAndDelay($FB7B)`、
   `InitEnemyForceSpeedTowardsSamusDelayAndHealth($F85A)`。其余 `$0400`/`$6AF4` 字节由**进房清零**负责（`DestroyEnemies` 清 `$6AF4` 起 256 B，
   覆盖 6+6+4+1+1=18 个 Extra 槽——原版过清，见 §3.3）。

## 1. 生成侧

### 1.1 房间对象段 type 1 记录（A 级）

`EnemyLoop ; 07:EAD4` 按首字节低半字节分派（类型表见 #62 §3.1）。type 1 → `LoadEnemy ; 07:EB06` → `GetEnemyData ; 07:EB0C`：

| 字节 | 位 | 字段 | 含义 | 证据（`prg7_engine.asm`） |
|---|---|---|---|---|
| `b0` | 高 4 位 | 槽号 | 占用哪个 `$0400+slot*16` 敌槽（`$00`–`$50`） | `:8894-8896`（`and #$F0; tax`） |
| `b0` | 低 4 位 | 类型 | 固定 `1`（LoadEnemy） | `:8870-8880` 表项 1 |
| `b1` | bit7–6 | 特殊属性 | 写进 `Ens.specialAttribs(+0xF)`：bit7=tough（血量×2），bit6=Kraid/Ridley 本体（走雕像存活检查） | `:8911-8918` |
| `b1` | bit5–0 | 敌型 | `EnsExtra.type ← b1 & $3F`（索引每型属性表） | `:8932-8934` |
| `b2` | 高 4 位 | y | `y = (b2 & $F0) \| 8`（单位 16 px，+8 偏移保证在屏） | `:8937-8942` |
| `b2` | 低 4 位 | x | `x = (b2 << 4) \| $0C`（单位 16 px，+12 偏移） | `:8943-8946` |

- 记录**没有**数量/延迟/血量/朝向字段；这些全来自 §2 的每型表与常量。
- 槽已被占（`IsSlotTaken ; 07:EB7A`：`status≠0` 且 `data05.bit1≠0`）时**静默跳过**该条记录（不前进指针错误，仍 +3）。
- 记录计数（本卡实测，按**去重房间**计）：Br **113** / No **102** / Kr **48** / To **20** / Ri **94**，合计 **377**。
- **房间指针表重复项（A 级，影响逐区计数）**：Kraid `04:A1D5[37]` 里 `$A647` 出现 **2 次**（37 项 / 36 个不同房），Ridley `05:A17F[42]` 里 `$A23F` 出现 **2 次**（42 项 / 41 个不同房）。按「表项数」逐房累加会把同一房间的敌人/结构算两遍。⇒ 本卡与 #62 的差异：Kraid 敌记录 **48**（不是 51）、结构记录 **523**（不是 539）；Ridley 结构记录 **474**（不是 476），敌记录 94 不变。复算命令见 §6.4。

### 1.2 Squeept 随需生成（A 级）

`ChooseSpawningRoutine ; 07:EDD6` 的 JumpEngine 表项 1 = `SpawnMapEnemy ; 07:EDF8`：

```
SpawnMapEnemy: jsr GetEnemyData      ; 数据来指针 $00/$01（AddToPtr00 已推进到随需记录）
               jmp ChooseSpawningRoutine
```

- 所以「Q 弹被打破后冒出的敌人」与房间记录**共用同一 3 字节格式与同一装载函数**；差别只在数据源（特殊物品表 vs 房间记录）。
- 这条路径的存在正是「没有静态生成表」的反证之一：生成 = 函数 + 记录，而不是查表。

## 2. 每型属性表（每区一份，16 项）

### 2.1 地址表（A 级：ROM 符号与字节都核过）

| 表 | 地址（bank 1/2/3/5） | 项宽 | 项数 | 消费者 |
|---|---|---|---|---|
| `EnemyHealthTbl` | `$962B` | 1 B | 16 | `InitEnemyForceSpeed...$F866`（tough 版 `asl` 双血） |
| `EnemyRestingAnimIndex` | `$963B` | 2 B | 16 | `InitEnAnimIndex`/`EnemyFlipAfterDisplacement`（近身 <8 px 时切静止动画） |
| `EnemyActiveAnimIndex` | `$965B` | 2 B | 16 | `InitEnActiveAnimIndex`（激活时切活动动画） |
| `EnemyActiveAnimIndexInitOffset` | `$967B` | 2 B | 16 | 动画初偏移（bit7=方向） |
| `L968B` | `$968B` | 1 B | 16 | 公共 AI 位组（bit5 等，`areas_common.asm:468/485/775-784`） |
| `EnemyForceSpeedTowardsSamusDelayTbl` | `$969B` | 1 B | 16 | `data0D` 初值（`:11711`）；**bank 4 为 `EnemyData0DTbl`** |
| `EnemyDistanceToSamusThreshold` | `$96AB` | 1 B | 16 | 追 Samus 的距离阈值（bit7=另一侧） |
| `EnemyInitDelayTbl` | `$96BB` | 1 B | 16 | `Ens.delay(+9)` 初值（`:12289`） |
| `EnemyMovementChoiceOffset` | `$96CB` | 1 B | 16 | 激活时查 `EnemyMovementChoices`（`:12400+`） |
| `L977B` | `$977B` | 1 B | 16 | bit7→`data05.bit7`（初始水平朝向）；bit6 语义见 §2.3（B 级） |

### 2.2 实 dump（ROM 原字节，Br / Kr）

```
Br $962B health : 08 08 04 ff 02 02 04 01 20 ff ff 04 01 00 00 00
Br $969B data0D : 01 01 01 01 01 01 01 01 20 01 01 01 40 00 00 00
Br $96BB initDly: 08 08 01 01 01 01 10 08 10 00 00 01 01 00 00 00
Br $977B facing : 64 6c 21 01 04 00 4c 40 04 00 00 40 40 00 00 00
Kr $962B health : 08 08 00 ff 02 02 00 01 60 ff ff 00 00 00 00 00
Kr $969B data0D : 01 01 01 01 01 01 01 01 01 01 01 01 40 00 00 00   ← 表名是 EnemyData0DTbl
Kr $96BB initDly: 08 08 01 01 01 01 10 08 10 00 00 01 01 00 00 00
Kr $977B facing : 64 6c 21 01 04 00 4c 40 04 00 00 40 40 00 00 00
```

敌型编号（各区注释一致）：`0` 侧跳虫 / `1` 天花板侧跳虫 / `2` 波（多数区未用）/ `3` 撕裂者 / `4` 俯冲虫 / `5` 爬虫 /
`6` 俯冲飞虫 / `7` 斑马虫(Zeb) 或 Geega / `8` Kraid / `9` Kraid 绒毛 / `A` Kraid 指甲 / `B`–`F` 空。

### 2.3 `L977B` 的位（bit7 A 级，bit6 B 级）

- `LoadTableAt977B ; 01–05:80B0`：`A = L977B[type]; asl`（左移一位，carry = 原 bit7，结果 bit7 = 原 bit6）。
- `InitEnemyFacingDirectionAxisAndDelay ; 07:FB7B` 紧接 `ror Ens.0.data05,x` ⇒ **原 bit7 落到 `data05.bit7`**（水平朝向；`areas_common.asm:201/556-559` 按 bit7 选水平/垂直方向）。A 级。
- 原 bit6 变成 asl 结果的 bit7，但 `FB7B` 没有消费该位；其语义只能从「哪些型的字节置位」反推（B 级，未逐型 trace）：
  置 bit6 的型集中在垂直/自定义移动（如 Br `0x40`=Zeb、`0x64`=侧跳虫、To `0x50`=Metroid）。**不据此改名或断言轴语义。**

## 3. 槽初始化

### 3.1 内存布局（A 级）

| 区 | 地址 | 槽数 | 槽宽 | 结构 |
|---|---|---|---|---|
| `Ens` | `$0400`–`$045F` | 6 | 16 B | y0,x1,speedY2,speedX3,isHit4,**data05**5,subY6,subX7,movementIndex8,**delay**9,data0A10,**health**11,prevStatus12,**data0D**13,weaponAction14,**specialAttribs**15 |
| `EnsExtra` | `$6AF4`–`$6B53` | 6 | 16 B | **status**0,rY1,rX2,animFrame3,animDelay4,resetAnimIndex5,animIndex6,**hi**7,subY8,subX9,accelY10,accelX11,jumpDsplcmnt12/13,type14,data1F15 |
| `EnProjectiles` | `$0460`–`$04BF` | 6 | 16 B | 同 `En` 布局（敌弹） |
| `EnExplosions` | `$04C0`–`$04FF` | 4 | 16 B | 同 `En` 布局（爆炸） |

`$0400`–`$04FF` 共 16 槽；**敌槽只有 6 个**——记录首字节的高半字节只出现 `$0`–`$5`（对应地址 `$00/$10/$20/$30/$40/$50`，即 `$0400`–`$0450` 步长 16）。

### 3.2 逐字段写者表（A 级）

| 字段（Ens/EnsExtra 偏移） | 写者 | 写法 | 来源 |
|---|---|---|---|
| `+0xF` specialAttribs | `GetEnemyType $EB28` | `etype & $C0` | §1.1 b1 bit7–6 |
| `+0xE` EnsExtra.type | `GetEnemyType $EB28` | `etype & $3F` | §1.1 b1 bit5–0 |
| `+0x0` y | `LEB4D $EB4D` | `(pos & $F0) \| 8` | §1.1 b2 高半字节 |
| `+0x1` x | `LEB4D $EB4D` | `(pos << 4) \| $0C` | §1.1 b2 低半字节 |
| `+0x0` EnsExtra.status | `LEB4D $EB4D` | `enemyStatus_Resting`(=1) | 常量 |
| `+0x4` isHit | `LEB4D $EB4D` | `0` | — |
| `+0x7` EnsExtra.hi | `LEB4D $EB4D` | `(PPUCTRL ^ ScrollDir) & 1`（`GetNameTableAtScrollDir $EB85`） | 名表 |
| `+0x5` data05 | `LEB6E $EB6E` + `InitEnemyFacing $FB7B` | 指令形：`asl Ens.0.data05,x`（`1E 05 04`）→ `ror Ens.0.data05,x`（`7E 05 04`），净效果 `data05 = (旧 & $7F) \| (L977B.bit7<<7)` | §2.3 |
| `+0x9` delay | `InitEnemyFacing $FB7B` | `EnemyInitDelayTbl[type]` | §2.1 |
| `+0xD` data0D | `InitEnemyForce $F85A` | `ForceSpeedTbl[type]`（Kraid=1） | §2.1 |
| `+0xB` health | `InitEnemyForce $F85A` | `HealthTbl[type]`，tough(`+0xF` bit7) 时 `asl` | §2.1 |

> `LEB6E` 首行 `asl Ens.0.data05,x`（`$0405`←`$0405×2`）与新槽 `data05=0` 组合无副作用；对**回收槽**则会把旧 bit6 移入 bit7，再由 `ror` 覆盖 —— 净效果仍是「位7=朝向」，A 级结论见 `room_draw.rs:869-880` 注释链。
>
> 时序：同一次 `LoadEnemy` 调用内**顺序**写 y/x/status/isHit/hi → data05/delay → data0D/health，中间无跳帧（`:8893-8956` 直落）。

### 3.3 进房清零（A 级）

`MoreInit ; 07:C81D`（进区/进房循环 2 次）里：

1. 大 RAM 清零环 `$7A`–`$DE` + `$0300`–`$03FE`（`LC830`，`:1830-1844`；含 `Objects.0.status,x` 覆盖所有 `$0300` 对象槽）；
2. `jsr EraseAllSprites`（OAM 影子）→ `jsr DestroyEnemies`；
3. `DestroyEnemies ; 07:C8BB`：清 `$0097`–`$00DE`（**原版 off-by-one，漏 `$DF`**）+ **`EnsExtra.0.status,x` 从 `$6AF4` 起清 256 字节** ⇒ 顺带清掉 `EnProjectilesExtra`(6)+`EnExplosionsExtra`(4)+`EnMotherBrainExtra`(1)+`EnMellowHandlerExtra`(1)，共 18 个 Extra 槽（多于 6 个敌对槽）；然后清 `MetroidOnSamus` 与 Metroid 锁存。
4. 注意：`$0400` 的 **Ens 本体**没有在这条链里被整块清；敌槽的 `data05/delay/health` 等由 §3.2 的写者在每次装载时显式覆盖，`EnsExtra` 其余字节由 DestroyEnemies 清零。（B 级：`$0400` 是否另有清零点未逐址搜尽，见 §7。）

## 4. 调用链与帧内顺序

```
进区 AreaInit 07:C801 → MoreInit 07:C81D（2 次循环）
  ├─ 大 RAM 清零（$7A-$DE / $0300-$03FE）
  ├─ ScreenOff → ClearNameTables → EraseAllSprites → DestroyEnemies
  ├─ CopyAreaPointers → GetRoomNum → SetupRoom 07:EA2B …
  │    └─ DrawRoom 07:EAAA → DrawRoomObject（结构段，$FD 止）→ EndOfRoomObjects 07:EACA
  │         └─ EnemyLoop 07:EAD4 → JumpEngine[type] → LoadEnemy 07:EB06 → GetEnemyData 07:EB0C
  └─（属性表阶段 $F0–$F4，见 #62）
每帧 UpdateAllEnemies 07:F345：X=$50→$00（步长 -$10，即槽 5→0，最多 6 敌）
```

xref 计数（`grep -c` 于 `refs/m1disasm/SRC`，含定义行）：`DrawRoom` 2、`EndOfRoomObjects` 2、`EnemyLoop` 4、
`GetEnemyData` 3、`LoadEnemy` 1（仅表项）、`InitEnemyFacingDirectionAxisAndDelay` 11、`InitEnemyForceSpeedTowardsSamusDelayAndHealth` 7。
后两者的多数 xref 来自 AI 侧（`areas_common`），不只装载径。

## 5. 与现实现对照

| 面 | ROM | Rust 现状 | 判定 |
|---|---|---|---|
| 房间记录 → 敌槽 | `EnemyLoop` type1 → `LoadEnemy` | `crates/game-core/src/world/room_draw.rs:823 load_enemy`（全径：`IsSlotTaken`/`GetEnemyType`/`LEB4D`/`LEB6E`/`InitEnemyFacing`/`InitEnemyForce`） | **一致** |
| 每型属性表 | §2.1 十张表 | `crates/game-data/src/assets.rs:322 enemy_health`、`:327 enemy_force_speed_delay`、`:329 enemy_init_delay`、`:330 enemy_facing`、`:1256 enemy_movement_choice_offset`、`:1384 enemy_movement_choices` | **一致**（按区取表） |
| 槽清零 | `DestroyEnemies 07:C8BB` | `crates/game-core/src/world/elevator.rs:416 destroy_enemies`（`$0097-$00DE` + `$6AF4` 起 256 B，逐址照录） | **一致**（含原版过清） |
| Squeept 随需生成 | `SpawnMapEnemy 07:EDF8` → `GetEnemyData` | `crates/game-core/src/pickup/spawn.rs:50 spawn_item` 的 type 1 分支是 **DIVERGENCES 记账桩**（压 `$EDF8`，不生成敌人） | **缺失（待移植）** |
| 6 敌上限 | `Ens instanceof En 6` | 敌槽常量 6，`room_draw.rs` 遍历 `0..6` | **一致** |
| `$0400` 本体进房清零 | 未在 `MoreInit`/`DestroyEnemies` 显式整块清 | 无对应整块清（依赖装载覆盖） | **待核对**（§7 H1） |

**差异结论**
1. 唯一真实缺口：**Squeept 生成的敌人不出现**（Rust 只记账）。这不会让既有对拍变红（Tourian/Norfair 的对拍未走到该路径），但 B5 完整移植需要补。
2. 无「生成表」需要移植——Rust 直接从房间记录生成，方向正确。

## 6. 复现命令（原样可跑，输出即文档数字）

```bash
# 6.1 一条房间敌记录 + 它的槽/类型/位置解码（Br R00，房基址 A441）
python3 - <<'PY'
import pathlib
rom=pathlib.Path('rom/Metroid (USA).nes').read_bytes(); prg=rom[16:16+128*1024]
d=prg[1*0x4000+(0xA441-0x8000):][:400]; i=1
while i<len(d) and d[i]!=0xFF: i+=3
i+=1
while d[i]!=0xFF:
    b0,b1,b2=d[i],d[i+1],d[i+2]; ty=b0&0x0F
    if ty==1: print(f"slot=${b0>>4:X} type={b1&0x3F:#04x} special=${b1&0xC0:#04x} y=(({b2:#04x}&F0)|8) x=(({b2:#04x}<<4)|0C)")
    i+= {1:3,2:2,6:1,7:3}.get(ty,1)
PY

# 6.4 房间指针表重复项（Kraid 0xA647×2 / Ridley 0xA23F×2）
python3 - <<'PY'
import pathlib
rom=pathlib.Path('rom/Metroid (USA).nes').read_bytes(); prg=rom[16:16+128*1024]
def rd(b,a,n): return prg[b*0x4000+(a-0x8000):][:n]
def word(b,a):
    x=rd(b,a,2); return x[0]|(x[1]<<8)
for name,b,t,n in (('Kr',4,0xA1D5,37),('Ri',5,0xA17F,42)):
    ps=[word(b,t+2*i) for i in range(n)]
    print(name,'entries',n,'distinct',len(set(ps)),'dups',{hex(p):ps.count(p) for p in set(ps) if ps.count(p)>1})
PY

# 6.2 每型表 dump（Brinstar / Kraid 对照，重点看 $969B）
python3 - <<'PY'
import pathlib
rom=pathlib.Path('rom/Metroid (USA).nes').read_bytes(); prg=rom[16:16+128*1024]
def rd(b,a,n): return prg[b*0x4000+(a-0x8000):][:n]
for name,b in (('Br',1),('Kr',4)):
    print(name,'health',rd(b,0x962B,16).hex(' '))
    print(name,'$969B ',rd(b,0x969B,16).hex(' '))
    print(name,'initDly',rd(b,0x96BB,16).hex(' '))
    print(name,'L977B  ',rd(b,0x977B,16).hex(' '))
PY

# 6.3 槽清零区间（DestroyEnemies：$6AF4 起 256 B，覆盖 18 个 Extra 槽）
grep -n "DestroyEnemies: ; 07:C8BB" -A 14 refs/m1disasm/SRC/prg7_engine.asm | sed -n '1,16p'
```

## 7. 假设区（C 级）

- **H1**：敌槽本体（`$0400`–`$045F`）在进房时是否被整块清零。`MoreInit` 的 RAM 环只到 `$03FE`，`DestroyEnemies` 只清 `$6AF4` 起 256 B；
  敌槽的 `status` 在 Extra 侧（`$6AF4`），所以「槽是否空闲」判据始终正确；但 `Ens` 本体的旧值（如 `prevStatus`、`animDelay`）理论上是残留，
  未逐址搜尽写点 → 记 C 级。
- **H2**：`L977B` 的 bit6 语义（§2.3）。只从置位型的移动方式反推，未逐型 trace。
- **H3**：Q 弹记录里的「敌型/位置」字段是否与房间记录完全同构。已核 `SpawnMapEnemy` 复用 `GetEnemyData`（A 级），
  但特殊物品表中 type1 记录的字节布局未逐条抽查（B 级）。

## 8. 未覆盖

1. 380 条敌记录未逐条列出（#62 已给逐区计数；全量清单可用 §6.1 脚本遍历）。
2. `EnemyMovementChoices` 的逐型取值（移动行为表）未解码——属 B3/B5 其它卡的 AI 面。
3. Squeept 生成的**时点**（击破后第几帧出现）未 trace。
4. 敌型 `2` 的「不能正确清除自身」怪癖（各区注释）未考证对应 ROM 行为。
5. `$0400` 本体清零点见 H1，未穷举写者。

## 9. 证据指针

| 结论 | 指针 |
|---|---|
| 记录 3B 字段与写者 | `refs/m1disasm/SRC/prg7_engine.asm:8889-8960`（LoadEnemy/GetEnemyData/GetEnemyType/LEB4D/LEB6E） |
| 类型分派表 | 同文件 `:8867-8880`（EnemyLoop JumpEngine 表） |
| 每型表地址与消费点 | `:11711`（data0D）、`:11721-11733`（health/tough×2）、`:12289`（init delay）、`:12257-12290`（facing/L977B）、`$962B…$977B` 表本体见 `prg{1,2,3,4,5}_*.asm` 同名标签 |
| Kraid 的 `$969B` 例外 | `prg4_kraid.asm:288`（`EnemyData0DTbl`）；ROM `04:969B` dump 见 §2.2 |
| 槽布局与槽数 | `refs/m1disasm/SRC/constants_ram.asm:972`（`Ens … En 6`）、`:1294`（`EnsExtra … EnExtra 6`）、`:875`（Doors 4）、`:877`（SamusProjectiles 3） |
| 进房清零 | `prg7_engine.asm:1830-1844`（MoreInit RAM 环）、`:1948-1965`（DestroyEnemies） |
| 每帧敌更新顺序 | `prg7_engine.asm:10660-10668`（UpdateAllEnemies，X=$50→$00） |
| 随需生成 | `prg7_engine.asm:9497-9517`（ChooseSpawningRoutine 表 + SpawnMapEnemy） |
| Rust 装载 | `crates/game-core/src/world/room_draw.rs:823-900` |
| Rust 清零 | `crates/game-core/src/world/elevator.rs:416-432` |
| Rust 表取数 | `crates/game-data/src/assets.rs:322/327/329/330/1256/1384` |
| Rust Q 弹缺口 | `crates/game-core/src/pickup/spawn.rs:50-75`（type1=DIVERGENCES 桩） |
| ROM 身份 | `sha256 e6e6b701…65b6` |
