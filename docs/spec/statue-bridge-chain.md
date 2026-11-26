# 雕像 / 桥 / 结局门链规格（`$687B`/`$687C` 写读链）

> 只读取证，只写本文件。所有数字可由 §7 命令复算；每条标 A/B/C。
> 关联：#67（进度链）、#69（雕像复核）；本文补的是「**通关门链**」这一条窄路径的写点/读点全表。

## 0. 结论（先看这五条）

1. **两个字节、四个 bit**：`KraidStatueStatus=$687B`（bit0=Kraid 已击败/闪像；bit7=雕像已抬起）、`RidleyStatueStatus=$687C`（bit1=Ridley 已击败/闪像；bit7=雕像已抬起）。中间值 `$01`/`$02` = 「Boss 死了、像在闪、可打」；终值 `$81`/`$82` = 「像已抬起」。（A，§1/§2）
2. **完整链**：打死 Boss → `LDD75`（`07:DD75`）写 `$01`/`$02` + `MiniBossKillDelayFlag` + 导弹上限 +75 → 像闪 → 玩家打像 → `isHit++` → `StartRaising` 先抬 1px → `Raise` 每帧 1px 到 16 界 → **`status ← raise|$80`** → **`UpdateAllStatues_Bridge`（`07:DADA`）双像 bit7 相与非 0 ⇒ 置 `$54=$80` 并造 8 块桥**（A，§4）
3. **Rust 现状：雕像子系统本体已端口（`world/statue.rs` 六个函数逐行对表），但链上有两个真断点**：
   - `enemy/framework.rs:520 enemy_complete_death` **没有** `prevStatus bit6 → LDD75` 分支 ⇒ Boss 死后 `$687B/$687C` 永不亮、`MiniBossKillDelayFlag` 永不置、导弹上限永不 +75；
   - `world/room_draw.rs:823 load_enemy` **没有** `GetEnemyType` 的 `InArea&6 → 雕像状态 → 非 0 跳过` 段 ⇒ 已击败的 Boss 会在进房时被重新装载（并重设 `KraidRidleyPresent`）。
   （A/B，§5）
4. **结局不读这两个字节**：`$8F` 结局电梯（`07:D8BF`）只写 `AtEnding+1`/`MainRoutine`/`TitleRoutine`；`ChooseEnding`（`07:CAF5`）只按 `SamusAge` 选 5 档结局。全 asm 对 `$687B/$687C` 的读点里**没有结局/标题结局路径**——进 Tourian 的门是**物理的桥**（瓦片），不是状态位检查。（A，§6）
5. **`$54`（`StatuesBridgeIsSpawned`）是易失的**：`LoadStatues`（`07:EC2F`，每次进有雕像的房间都跑）把它清 0 并把 `Statue.status←1`，所以桥只在「本房间内且双像 bit7」时造一次；`$687B/$687C` 才是持久态。（A）

## 1. 字节与位语义（A）

| 址 | 名称 | bit | 语义 | 写入点 |
|---|---|---|---|---|
| `$687B` | `KraidStatueStatus` | bit0 | Kraid 已击败（像在闪，可打） | `LDD75`（`07:DD75`） |
| `$687B` | 同上 | bit7 | Kraid 像已抬起 | `UpdateStatue_Raise`（**`07:DA63`**：`9d 1b 68` = `sta $681B,x`，x=`$60` ⇒ 落 `$687B`；Ridley x=`$61` ⇒ `$687C`） |
| `$687C` | `RidleyStatueStatus` | bit1 | Ridley 已击败（像在闪，可打） | `LDD75` |
| `$687C` | 同上 | bit7 | Ridley 像已抬起 | 同上 `07:DA63`（x=`$61` ⇒ `$681B+$61=$687C`） |
| `$0054` | `StatuesBridgeIsSpawned` | bit7 | 桥已生成（本房间内） | `UpdateAllStatues_Bridge`（`07:DADA`）；`LoadStatues` 清 0 |
| `$0364/$0365` | `Kraid/RidleyStatueRaiseState` | bit7 | 抬升完成（别名 `Statue.animDelay` / `Statue.animResetIndex`） | `UpdateStatue_Raise`（**`07:DA60`**：`9d 04 03` = `sta $0304,x`，x=`$60`/`$61` ⇒ `$0364`/`$0365`） |
| `$0366/$0367` | `Kraid/RidleyStatueIsHit` | — | 被打次数（`inc`；别名 `Statue.animIndex` / `Statue.data07`） | 弹门环兜底（`07:E86F` 区段） |

证据（ROM 字节，`07:DD75` 起 22 B）：

```
20 F9 CB      jsr PowerUpMusic
A5 74         lda InArea
29 0F         and #$0F
8D 08 01      sta MiniBossKillDelayFlag
4A            lsr            ; Kraid: $12&$0F=2 → 1；Ridley: $14&$0F=4 → 2
A8            tay
99 7A 68      sta $687A,y    ; ← LDD75 的地址算术 = KraidStatueStatus-1,y ⇒ $687B/$687C
A9 4B         lda #$4B       ; 75
20 97 DD      jsr AddToMaxMissiles
D0 D0         bne LDD5B
```

`InArea` 的取值是 `$12`=Kraid hideout / `$14`=Ridley hideout（`constants_ram.asm:627-629`）。注意写的是 **`$01`/`$02`（数值）**，不是 `|$80`——所以这两个 bit 与 bit7 是相互独立的状态位。

## 2. 写点全表（A）

| # | `bank:addr` | 位置 | 条件 | 写什么 |
|---|---|---|---|---|
| W1 | `07:DD75` | `LDD75`，`EnemyCompleteDeath`（`07:DCF5`）内 | **不是**（`InArea==$13` 且 type∈{`$02`,`$04`}）**且** `Ens.prevStatus` bit6 置（`asl; bmi LDD75`） | `$687B←$01` 或 `$687C←$02`；并置 `MiniBossKillDelayFlag=InArea&$0F` |
| W2 | `07:DA5B`–`07:DA66` | `UpdateStatue_Raise` 末段（符号 `07:DA3D` 起；`lda` 在 +`$1E`、两处 `sta` 在 +`$23`/`+$26`） | 本像 `Y&$0F` 减到 0（抬满 16px） | `DA5B lda $0304,x` → `DA5E ora #$80` → **`DA60 sta $0304,x`（写 RaiseState `$0364/$0365`）** → **`DA63 sta $681B,x`（x=`$60`/`$61` ⇒ 写 `$687B/$687C`）** → `DA66 inc $0304,x` |
| W3 | `00:8D3D` | `LoadTanksAndMissiles`（密码导入） | 读密码字节 `$6997` 的高 4 bit | `$687B←(bit7?$80)+(bit6?$01)`；`$687C←(bit5?$80)+(bit4?$02)` |
| W4 | `00:932B` | `InitializeStats`（新游戏） | 无条件 | 两字节清 0 |
| W5 | `07:DD97` 调用者 | `AddToMaxMissiles`（由 W1 调用） | — | `$6879/$687A` 各 +75 并钳 `$FF` |

**语义要点**（A）：W1 的判定用的是 `prevStatus` 的 **bit6**——`GetEnemyType` 装载时 `specialAttribs←etype&$C0`，Boss 记录的 etype 是 `$48`/`$49`（bit6 置），而 `EnemyReactToSamusProjectile` 每帧把 `specialAttribs` 的 bit6/7 `ora` 进 `prevStatus`（`07:F5C4`）；`asl` 后 bit6 变 bit7，`bmi` 即「这是 mini-boss」。（`LDD75` 在 asm 里的注释就是 “miniboss was just killed”。）

## 3. 读点全表（A）

| # | `bank:addr` | 位置 | 读法 | 用途 |
|---|---|---|---|---|
| R1 | `07:EB28` | `GetEnemyType` | `InArea&$06 → lsr → y`，`lda $687A,y`；非 0 ⇒ 弹掉当前敌人记录 | **已击败的 Boss 不再装载**；顺带写 `KraidRidleyPresent←1`（活着时） |
| R2 | `07:EC2F` | `LoadStatues`（房间流 type6） | `bpl` 判 bit7 | 定雕像初始 Y：抬过=`$30/$50`，未抬=`$40/$60`；并清 `$54`、`Statue.status←1` |
| R3 | `07:DA3D` | `UpdateStatue_Raise` | `bit7` ⇒ 直接退 | 已达抬起态就不动 |
| R4 | `07:DA7C` | `UpdateStatue_StartRaising` | `beq/bmi` | status=0（房内无像）或 bit7（已抬）⇒ 不清 isHit 以外什么都不做 |
| R5 | `07:DA1A` | `UpdateStatue` | `beq/bmi` | 0/bit7 ⇒ 每帧画；**否则只在奇数帧画（闪像）** |
| R6 | `07:DADA` | `UpdateAllStatues_Bridge` | `lda $687B; and $687C; bpl` | 双 bit7 ⇒ 造桥 |
| R7 | `00:8CCC` | 密码导出 | `and #$80` / `and #$01`（Kraid）、`#$20`/`#$10`（Ridley） | 写进密码字节 `$6997` 高 4 bit |
| R8 | `00:8DC8` | `LoadTanksAndMissiles` | `ldy $687B / beq`；`ldy $687C / beq` | 每个非 0 的雕像字节 +75 导弹上限（**只判非 0，不判 bit7**） |

**没有的读点**（A 级负面，见 §6）：结局/标题结局动画/`ChooseEnding`/`ElevatorD8BF` 全都不读 `$687B/$687C`；也没有「进 Tourian 需要 bit7」的判断——Tourian 门禁是桥瓦片铺出来的通路。

## 4. 完整调用链（A，附 Rust 对照）

```
[Brinstar 雕像房 Room2B / 0x2B]                 （rooms.asm:883-891：2 门 + .byte $06 Statues）
   │  进房：SetupRoom → 对象流 type6 → LoadStatues 07:EC2F
   │        （$54←0、Statue.status←1、两像 Y 按 bit7 定）
   ├─ UpdateAllStatues 07:D9D4（每帧，从 UpdateWorld 尾）
   │    ├ status==1：UpdateStatueBGTiles(0/1) 画“未抬起”两片瓦
   │    ├ status==2：若某像 bit7 ⇒ UpdateStatueBGTiles(2/3) 补画“抬起”瓦
   │    ├ UpdateStatue×2：Raise → StartRaising → 动画帧 → 闪像奇数帧画
   │    └ UpdateAllStatues_Bridge 07:DADA：$54 bit7? / DoorEntryStatus!=0? /
   │          ($687B & $687C) bit7? → 置 `$54←$80`（`sta StatuesBridgeIsSpawned` 直接复用 AND 结果：`$81 & $82 = $80`，恒为 `$80`，不存在 `$82` 分支）
   │          + 8×TileBlasts：routine←3、delay←y*2(y=8..1)、type←4、
   │            roomRAMPtr+1←(Statue.hi<<2)|$62、roomRAMPtr←(y*2)+8
   │
   ├─ [Kraid/Ridley 房] 打死 Boss：EnemyCompleteDeath 07:DCF5
   │      prevStatus bit6 → LDD75 07:DD75 → $687B←$01 / $687C←$02
   │      + MiniBossKillDelayFlag←InArea&$0F + AddToMaxMissiles(75)
   │      （GameEngine 07:C945 见到 flag ⇒ SetTimer(240 帧, routine 3)）
   │
   ├─ [回雕像房] 弹打闪像：SamusProjectileHitDoorOrStatue 门环尽
   │      → inc KraidStatueIsHit,x（`07:E86F` 区段，x=(Ptr>>3)&1；Rust `weapon/projectile.rs:56-58`）
   │      → 下一帧 StartRaising 先 dec Y 1px（并置 sfxTri_StatueRaise）
   │      → Raise 每帧 1px，到 16 界：status←raise|$80（$81/$82）
   │      → 桥：$54=$80 + 8 块 TileBlasts（桥 48px 宽？见 C1）
   │
   └─ [门 → 电梯 → Tourian] → 母脑 → Zebetite → 07:D8BF etype==$8F
          → AtEnding+1 / MainRoutine=IncRoutine / TitleRoutine=EndGame
          → ChooseEnding 07:CAF5（只按 SamusAge）→ 5 档结局
```

Rust 侧逐点对照：

| 链节 | Rust 落点 | 状态 |
|---|---|---|
| 进房装雕像（`LoadStatues`） | `world/room_draw.rs:747` → `world/statue.rs:191 load_statues` | **已端口**（Y 判 bit7、清 `$54`、status←1 全对） |
| 每帧更新＋造桥 | `world/update.rs:136` → `statue.rs:76 update_all_statues`（含 `:209 update_statues_bridge`） | **已端口**（双 AND 门 + 8 块 TileBlasts 有单测 `bridge_gate_and_tile_blasts`） |
| 弹打像登记 | `weapon/projectile.rs:56-58` 内联 `inc $0366+idx` | **已端口**（`statue.rs:202 statue_register_hit` 是同语义的重复实现、**无生产调用者**——不是缺口，只是命名重叠，建议后续收掉一个） |
| 抬升状态机 | `statue.rs:127 update_statue_raise` / `:152 update_statue_start_raising` | **已端口**（含 `status←raise|$80`） |
| **Boss 死亡写雕像** | `enemy/framework.rs:520 enemy_complete_death` | **断**：没有 `prevStatus bit6 → LDD75` 分支；`$687B/$687C` 不写、`MiniBossKillDelayFlag` 不置、+75 导弹不加 |
| **进房跳过已死 Boss** | `world/room_draw.rs:823 load_enemy` | **断**：没有 `InArea&6→雕像字节→非 0 跳过` 段，也没有 `KraidRidleyPresent←1`（`samus/handler.rs:1654` 只读/清它） |
| 密码导入雕像态 | `title/bridge.rs:775-789 load_tanks_and_missiles` | **已端口**（bit7/bit6/bit5/bit4 → `$80|$01`、`$80|$02`） |
| 结局入口 | `world/elevator.rs:274 elev_transition`（`etype==0x8F`） | **已端口**（`AT_ENDING+1`、`MAIN_ROUTINE`、`ROOM_PTR=0`、`BANK_SWITCH_PENDING=1`、`TITLE_ROUTINE←EndGame`） |

## 5. 断链的后果（B，可由代码直接读出）

- 因为 W1 缺：`$687B/$687C` 永远是 0（或密码给的 0/`$01`/`$02` 但无法再进位）⇒ **雕像不闪、打不抬、`$687B&$687C` 永不为 `$80`** ⇒ `update_statues_bridge` 的门判③永假 ⇒ **桥永远不生成** ⇒ Stage A 无法从 Brinstar 走到 Tourian 的电梯（除非密码已给 `$81/$82`）。
- 因为 R1 缺：**已击败的 Boss 房可以重复进**，Kraid/Ridley 会以满血重新装载；`KraidRidleyPresent` 也不会被写 1 ⇒ `SamusDoor`（`07:D3D8`/`samus/handler.rs:1654`）的 TourianMusic 分支永不触发。
- 因为 `MiniBossKillDelayFlag` 缺：原版「Boss 死后 240 帧空镜/音乐切换」不会发生（纯体验差，但属 Stage A 保真范围）。

## 6. 结局门（A 级负面结论）

- `07:D8BF`（`ElevatorD8BF`）里 `$8F` 分支与雕像状态**无关**：它只判断 `ElevatorType == $8F`，然后直接 `AtEnding++`、`TitleRoutine←EndGame`（ROM 字节见 §7）。
- `ChooseEnding`（`07:CAF5`）只读 `SamusAge`/`SamusAge+1` 与 `AgeTable`（`$7A/$16/$0A/$04`），产出 `EndingType` 1..5 与 `JustInBailey`（前两档脱装甲）。**不读** `$687B/$687C`。
- 因此「进 Tourian」不是状态位门，而是**地形门**：雕像房里的桥瓦片铺好后才能走到该房间的门（`Room2B` 的两个门：红 `$A0|0`、蓝 `$A0|1`，`rooms.asm:888-889`）。原版没有「没打双 Boss 就禁止进 Tourian」的检查——这点与代码事实一致，但我**未做运行时实测**（见 §8）。
- 相关但独立的旗：`MotherBrainStatus=$0A` 已死 ⇒ 死亡后直接 `inc MainRoutine` 进 GameOver（`07:C97B`），与雕像无关。

## 7. 复现命令（实跑输出已贴）

```bash
# ① LDD75 的 22 字节（看它写的是 $01/$02 数值，以及 $687A,y 这个地址算术）
python3 -c "import pathlib;prg=pathlib.Path('rom/Metroid (USA).nes').read_bytes()[16:16+128*1024];o=7*0x4000+(0xDD75-0xC000);print(' '.join(f'{b:02X}' for b in prg[o:o+22]))"
# → 20 F9 CB A5 74 29 0F 8D 08 01 4A A8 99 7A 68 A9 4B 20 97 DD D0 D0

# ② 桥门判 20 字节（$54 bit7 / DoorEntryStatus / ($687B & $687C) bpl）
python3 -c "import pathlib;prg=pathlib.Path('rom/Metroid (USA).nes').read_bytes()[16:16+128*1024];o=7*0x4000+(0xDADA-0xC000);print(' '.join(f'{b:02X}' for b in prg[o:o+20]))"
# → A5 54 30 38 A5 56 D0 34 AD 7B 68 2D 7C 68 10 2C 85 54 A2 70

# ③ 结局电梯 $8F 与 ChooseEnding（都不提雕像字节）
python3 -c "import pathlib;prg=pathlib.Path('rom/Metroid (USA).nes').read_bytes()[16:16+128*1024]
for a,n in ((0xD8BF,12),(0xCAF5,12)):o=7*0x4000+(a-0xC000);print(f'{a:04X}:',' '.join(f'{b:02X}' for b in prg[o:o+n]))"
# → D8BF: BD 0F 03 A8 C9 8F D0 13 A9 07 85 1E EE 83 68
# → CAF5: A0 01 AD 7F 68 D0 0D AD 7E 68 D9 17 CB B0 05

# ④ 读写点全集（数量对账：写 5 / 读 8，见 §2/§3）
grep -rn '\$687B\|\$687C\|KraidStatueStatus\|RidleyStatueStatus' refs/m1disasm/SRC/*.asm | wc -l   # 39
```

## 8. 未覆盖 / 假设区

- **C1**：桥的**可见形态**未取证——代码只生成 8 条 `TileBlasts`（`routine=3`=等待重生、`delay=y*2`=16..2 帧、`type=4`=通用可击砖动画、`roomRAMPtr` 指向 `(hi<<2)|$62` 行的 `(y*2)+8` 列），`Statue.hi` 是 nametable 选择（0/1）⇒ 桥横跨 8 个「砖」位、逐块 2 帧间隔出现。像素宽度与这 8 块是否连续成桥**未做 emu 实测**。
- **C2**：`KraidStatueIsHit` 的 `inc` 没有上限；若玩家连打多帧，`StartRaising` 每帧也只 dec 1px（`Raise` 再接管），多打是否等价——按代码是「被清 0 前只算一次」，未实测。
- **C3**：原版能否在桥未生成时用其它路径进 Tourian（如穿墙 glitch）——代码层没有禁令，但**未做 emu 实测**，不作为结论。
- 未做：emu 运行时复现「打死 Boss → 像闪」与「打像 → 桥生成」；Rust 侧四个断点的补丁（本文不替实现卡下结论）。

## 9. 证据指针

- asm（行号按 `grep -n '符号:'` 实测）：`prg7_engine.asm:5193`（UpdateAllStatues）、`:5260`（UpdateStatue）、`:5289`（Raise）、`:5341`（StartRaising）、`:5416`（Bridge）、`:5939`（EnemyCompleteDeath）、`:6044`（LDD75）、`:6018`（LDD5B）、`:6075`（AddToMaxMissiles）、`:8913`（GetEnemyType）、`:9153`（LoadStatues）、`:4980`（ElevatorD8BF）、`:2439`（ChooseEnding）。
- RAM 名：`constants_ram.asm:1234-1235`（`$687B/$687C`）、`:569`（`$54`）、`:923-926`（KraidRaise/RidleyRaise/KraidIsHit/RidleyIsHit 四个别名，一一对应 `$0364/$0365/$0366/$0367`）、`:627-628`（InArea 区值）、`:1256`（KraidRidleyPresent）。
- ROM 字节：§7 三条命令 + 输出。
- Rust：`world/statue.rs:76/127/152/191/202/209`、`world/room_draw.rs:747/823`、`world/update.rs:136`、`enemy/framework.rs:520`（调用点 `:242/:403`）、`weapon/projectile.rs:56-58`、`world/elevator.rs:274`、`title/bridge.rs:775-789`、`samus/handler.rs:1654`。
