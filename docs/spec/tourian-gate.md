# Tourian 可达性规格：雕像/桥 → Tourian 开放条件与入口链

> 任务卡 #116（`tsk_01M8CCT4GE3Q7JE9WE2HSP`）。范围：**只读取证**，不改代码。
> 证据等级按 `AGENTS.md` §3；行号指本仓 `refs/m1disasm/SRC/` 里的社区匹配反汇编（submodule 钉死版本）。
> 本文与 `docs/findings/statue-bridge-chain.md`（#112）、`docs/spec/doors-elevators.md`（#61）交叉引用，不重复其已证部分。

## §0 结论摘要

1. **雕像 → 桥**：击杀小 Boss 只写「闪烁值」`1`（Kraid→`$687B`）、`2`（Ridley→`$687C`）；**bit7 由抬像例程写**（`UpdateStatue_Raise 07:DA3D`）。桥的开放判据是 **两个状态字节 bit7 相与为真**（`07:DADA` 的 `lda/and/bpl`），并且 `$54` bit7（`StatuesBridgeIsSpawned`）只允许生成一次。8 块 TileBlast 按 `delay=y*2`（8..16 帧）依次补瓦。
2. **桥 → Tourian 入口**：桥在 Brinstar **Room2B**（世界图 y2,x4）。过桥进左门到 **Room2C**（y2,x3），Room2C 里唯一一条门（右侧、蓝门）与 Room2B 左门对接；Room2C 的对象段由特殊物品表给出 **`it_Elevator, $03`**（`brinstar/items.asm:@y02 @@x03`，原注释「Elevator to Tourian」）。
3. **换区只能靠电梯**：`InArea` 全 ROM 有 **3 处写入**（不是「写点唯一」）——
   | 写点 | 地址 | 内容 |
   |---|---|---|
   | 密码导入 | `00:8D2F` | `lda PasswordByte+$08 / and #$3F / sta InArea`（`prg0_title.asm:1861-1863`，label `LoadPasswordData 00:8D12`） |
   | 开局 `InitBank1` | `07:C563` | `lda #$00 / sta MainRoutine / sta InArea / sta GamePaused`（`prg7_engine.asm:1409-1414`；按字节数算：C55F `A9 00`、C561 `85 1E`、**C563 `85 74`**） |
   | `SwitchBank` | `07:CA23` | 常规换区写（`prg7_engine.asm:2264`） |
   
   但 **「没有某扇门自动切区」的结论仍成立**，依据换成**调用面有限**：进入 `SwitchBank` 只能经 `IsEngineRunning 07:CA18`，而 `jsr IsEngineRunning` 全 ROM 只有两处——`AreaInit 07:C801`（A=#$10 载 Brinstar）与电梯 `ElevatorD8BF 07:D8BF`（`prg7_engine.asm:1839`、`:5018`）。密码路径也不经 `SwitchBank`：`InitializeGame 00:92D4` 用 `InArea&$0F` 查 `BankTable` 写 `BankSwitchPending`、`MainRoutine=_id_MoreInit`，**不会把密码区号覆盖成 `#$10`**（`prg0_title.asm:2734-2757`，`lda BankTable,y` 在 `:2756`）。
4. **电梯参数 → 目标区**（A 级，`ElevatorD8BF`）：低半字节=目标区，bit7=方向（1=上）；bit7=1 时目标区默认 0（Brinstar），仅 `$84`→1（Norfair）；`$8F` 是结局特例。因此 Brinstar 的 `$03`（bit7=0）确实去 **Tourian（区 3）**，Tourian 的 `$83`（bit7=1）回 Brinstar。
5. **到达房**：换区后 `AreaInit → MoreInit` 用新区的 `AreaMapPosX/Y` 写 `MapPos`，再 `GetRoomNum` 查全局世界图。Tourian 的 `AreaMapPosX/Y = $03/$04` ⇒ 世界图 (y4,x3) = **房间 $0E**。`Room0E` 无门对象（15 行高的竖井），离开方式是**竖直滚屏到相邻房**（上邻 (y3,x3)=Room01），不是门。
6. **Tourian 仍不可达不是「链没接上」而是「链尾没落地」**：雕像写点（#112 已修）与桥读点已存在；缺的是 ①MotherBrain 生成仍是占位（`pickup/spawn.rs` 的 `it_MotherBrain` 分支只 `DIVERGENCES.push(0xEEAE)`）②`LDD5B 07:DD5B` 的 Tourian 配额重置 `$93-$96` 未端口 ③一条可复跑的「双 Boss 已死 + 像已抬 + 走进电梯」L4 起点态。本卡第 §8 给出最短路径与帧窗推导，并在 §8.3 记录本卡两次实测未跑通的确切观测。

## §1 链路总览

```
[Kraid/Ridley 房] 击杀小 Boss
        │  EnemyCompleteDeath 07:DCF5  (prevStatus bit6 ⇒ LDD75)
        ▼
LDD75 07:DD75：PowerUpMusic → MiniBossKillDelayFlag=$0108=InArea&$0F
                → y=InArea>>1 → $687A+y ← y   （Kraid→$687B=1，Ridley→$687C=2）
                → AddToMaxMissiles(75)         （导弹/上限各 +75，两处钉 $FF）
        ▼（玩家回到 Brinstar 桥房）
LoadStatues 07:EC2F → UpdateAllStatues 07:D9D4 → UpdateStatue_Raise 07:DA3D
        │ 到位帧：KraidStatueStatus/RidleyStatueStatus ← (值 | $80)
        ▼
UpdateAllStatues_Bridge 07:DADA：$54 bit7=0 且 DoorEntryStatus=0 且 (Kraid & Ridley & $80)
        → 8 块 TileBlast（routine=3，delay=y*2，type=4，roomRAMPtr 由 Statue.hi 拼）
        ▼
Brinstar Room2B（y2,x4）桥补全 → 左门（蓝）→ Room2C（y2,x3）
        ▼
Room2C 特殊物品 it_Elevator,$03（bit7=0=下行）→ ElevatorIdle 需 SamusOnElevator+DOWN
        ▼
ElevatorScrollXToCenter → ElevatorFade(1f) → ElevatorMove(y1px/f) → ElevatorScrollY(ScrollY→0)
        → ElevatorD8BF 07:D8BF：A = $10|3 → IsEngineRunning → SwitchBank 07:CA23 → sta InArea
        ▼
Tourian AreaInit/MoreInit：MapPos ← AreaMapPos(3,4) → GetRoomNum → Room0E
        ▼
Room0E（竖井，无门）→ 向上滚屏 → Room01 → …（MB 房按 Tourian items 表继续）
```

## §2 雕像状态 → 桥（写侧与读侧）

### §2.1 写侧：死亡只写 1/2，bit7 由抬像写

| 写点 | 地址 | 写值 | 证据 |
|---|---|---|---|
| 小 Boss 死亡 | `07:DD75` 起（`LDD75`） | `$687A+y ← y`（y=1→`$687B`←1，y=2→`$687C`←2） | `prg7_engine.asm:6044-6059`；`EnemyCompleteDeath 07:DCF5` 的 `lda Ens.0.prevStatus,x / asl / bmi LDD75`（`:5958-5960`）判 **bit6** |
| 抬像到位 | `07:DA3D`（`UpdateStatue_Raise`） | `(status | $80)` 回写 | `prg7_engine.asm` 抬像段 |
| 桥已生成旗 | `07:DAE9` | `$54 ← (Kraid & Ridley & $80)` | `prg7_engine.asm:5431`（`sta StatuesBridgeIsSpawned`） |

> 本节是对 #112 卡面口径的再次确认：**「死亡即写 bit7」与 ROM 不符**；#112 findings §0 已就地更正，代码已按此实现（`enemy/framework.rs` 的 `mini_boss_killed` 写 1/2，`world/statue.rs` 抬像写 bit7）。

### §2.2 读侧：桥的开放判据

```asm
UpdateAllStatues_Bridge: ; 07:DADA
    lda StatuesBridgeIsSpawned
    bmi Exit0                 ; $54 bit7 已置 ⇒ 只生成一次
    lda DoorEntryStatus
    bne Exit0                 ; 门过渡中不补桥
    lda KraidStatueStatus
    and RidleyStatueStatus
    bpl Exit0                 ; 两个 bit7 相与后为负 ⇒ 都抬起；否则退出
```

- `p ` 结果直接 `sta StatuesBridgeIsSpawned`（`:5431`）⇒ `$54` 的高位等于「两像皆抬」。
- 8 块 TileBlast：`ldx #(8-1)*_sizeof_TileBlasts`、`ldy #$08`、`routine=3`、`delay=y*2`、`type=4`、`roomRAMPtr+1 = (Statue.hi<<2)|$62`、`roomRAMPtr = y*2+8`（`:5434-5455`）。
- Rust 侧读侧已完整：`crates/game-core/src/world/statue.rs` 的 `UpdateStatues_Bridge` 逐行对照（#112 已验收）。

### §2.3 「先杀谁/抬谁」的时序

- `LoadStatues` 只在**桥房装载时**跑，雕像 y 由状态决定：`RidleyStatueStatus` 为负 ⇒ Kraid 像 y=$30 否则$40；`KraidStatueStatus` 为负 ⇒ Ridley 像 y=$50 否则$60（`07:EC2F-07:EC4A`）。
- 抬像由 `UpdateAllStatues` 在房间内推进（`RaiseState $0364/$0365`），**与玩家是否站在像前无关**（本引擎的抬像是过场计时），因此最短路径只需「让两个状态字节的 bit7 到位」，不必真去两间 Boss 房打一遍——这正是 §8 起点态可以预置的依据（B 级：机制由代码路径给出，具体起始延迟未逐帧测）。

## §3 桥 → 入口房（门表与连通）

### §3.1 门记录字节的解码（A 级）

`SpawnDoorRoutine 07:EB92`：

```asm
    iny
    lda (Temp00_RoomPtr),y    ; 门信息字节 = $A0 | (side<<4) | doorType
    pha
    jsr Amul16                ; 左移4位；C = 原 bit4 = side（0=右,1=左）
    php
    lda MapPosX
    clc
    adc MapPosY
    plp
    rol
    and #$03                  ; (奇偶<<1) | side
    tay
    ldx DoorSlots,y           ; [Doors.0, Doors.3, Doors.2, Doors.1]
```

- `Amul16` 是「ASL×4」，第 4 次进位取原 bit4 ⇒ **bit4 = side（0 右 / 1 左）**，低 2 位 = 门类型。
- `DoorXs 07:EBFC = [$F0,$10]`（右/左），门 y 恒 `$68`。
- `DoorSlots 07:EC00 = [Doors.0−Objects, Doors.3−Objects, Doors.2−Objects, Doors.1−Objects]`，索引 = `((MapPosX+MapPosY)&1)<<1 | side`。

### §3.2 两间房的门表

| 房 | 世界格 | 门记录 | 解码 | 证据 |
|---|---|---|---|---|
| `Room2B`（桥房） | (y2,x4)=`$2B` | `$A0\|(0<<4)\|doorType_Red` | 右侧红门 | `brinstar/rooms.asm:853-…` |
| 同上 | | `$A0\|(1<<4)\|doorType_Blue` | 左侧蓝门 | 同上 |
| `Room2C`（电梯房） | (y2,x3)=`$2C` | `$A0\|(0<<4)\|doorType_Blue` | 右侧蓝门 | `brinstar/rooms.asm:01:AC24` 段尾 |

⇒ Room2B 的左门（x=$10 侧）与 Room2C 的右门（x=$F0 侧）是同一对门。**桥补全前走不过去**（桥是 Room2B 内部的瓦片，不是门）。

### §3.3 Room2C 的内容

- palette `$0`；结构 12 条，覆盖 (0,0)(0,7)(0,E)(1,F)(2,0)(4,0)(5,F)(8,0)(8,7)(8,9)(A,0)(A,F)；对象段只有 1 条门（§3.2），**没有其它对象**。
- 电梯来自**特殊物品表**（不是房间对象）：`brinstar/items.asm:@y02 @@x03` = `.byte it_Elevator, $03`，注释「Elevator to Tourian」。

## §4 电梯：参数、状态机、换区

### §4.1 `it_Elevator` 参数全表（A 级，逐文件 grep）

| 所在区 | 世界格 | 参数 | bit7 | 目标区（推导） | 回程目标房 |
|---|---|---|---|---|---|
| Brinstar | (y02,x03) 注释「Elevator to Tourian」 | `$03` | 0（下） | 3 Tourian | Tourian `$0E` |
| Brinstar | (y0B,x16) | `$01` | 0（下） | 1 Norfair | Norfair `$02` |
| Brinstar | (y12,x07) | `$02` | 0（下） | 2 Kraid | Kraid `$02` |
| Norfair | (y0B,x16) | `$81` | 1（上） | 0 Brinstar | Brinstar `$09` |
| Norfair | (y0D,x16) | `$81` | 1（上） | 0 Brinstar | Brinstar `$09` |
| Norfair | (y16,x19) | `$04` | 0（下） | 4 Ridley | Ridley `$02` |
| Kraid | (y12,x07) | `$81` | 1（上） | 0 Brinstar | Brinstar `$09` |
| Kraid | (y14,x07) | `$82` | 1（上） | 0 Brinstar | Brinstar `$09` |
| Ridley | (y18,x19) | `$84` | 1（上） | 1 Norfair | Norfair `$02` |
| Tourian | (y04,x03) | `$83` | 1（上） | 0 Brinstar | Brinstar `$09` |
| Tourian | (y03,x01) | `$8F` | 1 | —（结局） | `EndGame 00:9AA7` |

- 目标区判定代码：`ElevatorD8BF 07:D8BF` 的 `tya / bpl @endIf_B`（bit7=0 ⇒ 目标=低半字节）；bit7=1 ⇒ `ldy #$00`，仅 `cmp #$84` 时 `iny`；`$8F` 在更早的 `cmp` 分支进结局（`prg7_engine.asm:4926-4935`）。
- 到达房 = 目标区 `AreaMapPosX/Y` 查全局 `WorldMap 00:A53E`（32×32，`y*32+x`）：

| 区 | AreaMapPosX/Y | 世界图格 | 到达房号 | 证据 |
|---|---|---|---|---|
| Brinstar | `$03/$0E` | (y14,x3) | `$09` | `prg1_brinstar.asm:AreaMapPosX/Y` |
| Norfair | `$16/$0D` | (y13,x22) | `$02` | `prg2_norfair.asm` |
| Kraid | `$07/$14` | (y20,x7) | `$02` | `prg4_kraid.asm:AreaSamusMapPosX/Y` |
| Tourian | `$03/$04` | (y4,x3) | **`$0E`** | `prg3_tourian.asm:AreaMapPosX/Y` + `WorldMap` |
| Ridley | `$19/$18` | (y24,x25) | `$02` | `prg5_ridley.asm` |

> 复跑：`python3 - <<'PY' …` 解析 `prg0_title.asm:00:A53E` 的 32×32 字节表并按 `MapPosY*32+MapPosX` 取值（本文件 §9 给整段命令）。

### §4.2 电梯状态机（A 级）

跳表在 `UpdateElevator 07:D7B3`（`prg7_engine.asm:4809-4820`），**status 从 1 起**：

| status | 例程 | 行为 |
|---|---|---|
| 1 | `ElevatorIdle 07:D7D1` | 需 `SamusOnElevator` 且按方向键（`ElevatorType` bit7=1 用 UP，否则 DOWN）；启动后清 `SamusAccelY`、`Samus.status=sa_Elevator`、把 `Samus.x/y` 拉到屏幕中心 `($80, SCRN_VY/2-8)`，status++ |
| 2 | `ElevatorScrollXToCenter 07:D80E` | `ScrollX==0` 直接进 3；否则把 `MirrorCntrl` bit3 置竖直镜像、`ScrollDir&=1`，按 `Elevator.x` 与 `ScrollX` 差逐帧 `ScrollLeft/Right` |
| 3 | `ElevatorMove 07:D83D` | 每帧 y∓1；`cmp #SCRN_VY($F0)` 时回绕 0/240 并 `ToggleObjectXSlotHi`；**`cmp #$83` 相等 → 进 4** |
| 4 | `ElevatorScrollY 07:D870` | `ScrollY!=0` ⇒ `ElevScrollRoom`（bit7=1 `ScrollUp`，否则 `ScrollDown`）；`ScrollY==0` ⇒ 设萨姆斯淡出动画、`Timer1=$40`（FDS 残留）、status++ |
| 5 | `ElevatorFade 07:D8A3` | 「淡出萨姆斯」空壳（1 帧），status++ ⇒ 6 |
| 6 | `ElevatorD8BF 07:D8BF` | 真正的换区：写 `InArea`、调色板、`DestroyEnemies`、给萨姆斯/电梯设淡入动画、`Timer1=$40` |
| 7 | `ElevatorFade 07:D8A3` | 「淡入萨姆斯」空壳；到 8 时重设电梯帧与萨姆斯动画 |
| 8 | `ElevatorMove` | 换区后电梯继续走（新区的落位段） |
| 9 | `ElevatorStop` | 结束 |

- `SpawnElevatorRoutine 07:EC09`：电梯固定 `y=$83、x=$80`，`hi=GetNameTableAtScrollDir`，`status=1`。
- 因为 `ElevatorMove` 从 `$83` 起、`cmp #$83` 在一次 ±1 之后才比较，**下/上行都要走满一整屏（240 像素）再回绕到 $83 才进 ScrollY 段**：这就是 §8 帧窗的推导基础。

### §4.3 换区序列（`ElevatorD8BF`）

```
A = ElevatorType → 目标区(0..4) | $10 → IsEngineRunning → SwitchBank 07:CA23 → sta InArea
→ PaletteToggle ^= (Palette00+1)^(Palette05+1)（Tourian 强制 Palette00）
→ WaitNMIPass_ → SelectSamusPalette → StartMusic → ScreenOn
→ CopyAreaPointers → DestroyEnemies → PageIndex=$20（电梯槽）→ Samus/Elevator 淡入动画
→ Timer1=$40
```

随后主线回到 `AreaInit/MoreInit`（`07:C801`）：清滚动、`MainRoutine++`、`AreaMapPos → MapPos`、`RoomNumber=$FF`、`CopyAreaPointers`、`GetRoomNum`、`SetupRoom`（`07:C86F` 循环直到 `RoomNumber` 有效）。

### §4.4 Tourian 首房 `Room0E`

- `tourian/rooms.asm:Room0E_ ; 03:AAE3`：palette `$3`；15 行结构（(0,0)(0,1)(0,8)(0,F)(4,1)(4,E)(8,0)(8,6)(8,F)(9,1)(9,E)(B,B)(C,3)(E,1)(E,E)）；`$FF` 结束，**无对象、无门**。
- 电梯特殊物品在 (y4,x3) 同格（`tourian/items.asm:@y04 @@x03 = it_Elevator,$83`），所以到达即是电梯房；**离开靠竖井向上滚屏**：世界图 (y3,x3)=`$01`，即 `Room01`（Tourian 第二段），不需要门。
- 后续 MB 房在 (yB,x2)（`tourian/items.asm:@y0B @@x02`：`it_MotherBrain` + `it_Zebetite|$40` + `it_RinkaSpawner|$10` + 3 门），与 `prg3_tourian.asm` 的 MB 11 态规格（#107）衔接。

## §5 Tourian 配额 `$93-$96`（#112 §7 的第 1 条缺口）

`LDD5B 07:DD5B`（掉落物生成失败时的补救，`:6018-6040`）：

```asm
    ldx PageIndex
    lda InArea
    cmp #$13            ; 不在 Tourian ⇒ RemoveEnemy
    beq Lx140
Lx139: jmp RemoveEnemy
Lx140:                  ; 在 Tourian：重置「掉落配额」再试一次
    lda RandomNumber1
    ldy #$00
    sty EnergyPickupQtyCur     ; $96 = 0
    sty MissilePickupQtyCur    ; $95 = 0
    iny
    sty MissilePickupQtyMax    ; $93 = 1
    sty EnergyPickupQtyMax     ; $94 = 1
    bne LDD30                  ; 重新走生成
```

- 四个字节：`$93 MissilePickupQtyMax`、`$94 EnergyPickupQtyMax`、`$95 MissilePickupQtyCur`、`$96 EnergyPickupQtyCur`（`constants_ram.asm:697-703`）。
- 原版注释明确记了一条 **BUG**：「跳过小 Boss（NARPASSWORD）时该假设不成立」⇒ 在 Tourian 会**强制出掉落**；Stage A 按 D4 保真保留，未修。
- Rust 侧该支路未端口：`EnemyCompleteDeath` 的 Tourian 支路只 `DIVERGENCES.push(0x0DD5B)`（#112 已记，`enemy/framework.rs`）。

## §6 与 #112 未覆盖项逐条对照

| #112 §7 条目 | 本卡结论 | 等级 |
|---|---|---|
| Tourian 配额 `$93-$96` 未端口 | 已成规格：4 字节语义 + 重置值 + NARPASSWORD bug（§5）；Rust 仍是 `0x0DD5B` 记账 | A（ROM 原文）+ B（Rust 缺失） |
| `KraidRidleyPresent` 写入时机仅装载 | 保持：`GetEnemyType 07:EB28` 只在小 Boss 记录且状态为 0 时写 `$6987←1`；读点只有 `07:4124/4128`（暂停/地图显示），**与 Tourian 开放无关** | A |
| 桥「可走」无像素级 L4 | 未闭合：#112 验收 note 已记「桥可走性无像素级 L4 存档态证据」；本卡把链路补到「哪个门、哪台电梯、哪个房号」，但像素级验证仍需「双 Boss 已死 + 像已抬 + 站在桥房」的 L4 起点态，两次尝试未跑通（§8.3），列为缺口 | 缺口 |
| （本卡新增）入口链是否靠电梯 | 是。`InArea` 有三处写（`00:8D2F` 密码 / `07:C563` 开局 / `07:CA23` 常规换区），但 `jsr IsEngineRunning` 只有 `AreaInit 07:C801` 与 `ElevatorD8BF 07:D8BF` 两处 ⇒ 常游戏内换区只能靠电梯 | A |

## §7 「开放条件」一句话版

> Tourian 的物理开放 = **两座石像都抬起（`$687B`、`$687C` 的 bit7 同时为 1）→ 桥瓦补全 → 从 Room2B 左门进 Room2C → 踩电梯按 DOWN（`$03`，下行）→ 区号切 3 → 落 Tourian `Room0E`**。
> 与「杀 Boss」的关系是间接的：击杀先写闪烁值 1/2，抬像才写 bit7；只杀不打完抬像过场，桥不会出现。

## §8 最短可验证路径

### §8.1 起点态与命令

Rust 侧壳提供 `--start-*`（摆房号/MapPos/坐标）与 `--poke 帧:地址=值`（单次 RAM 写；#102 起的验收面）：

```bash
cd /home/peng/workspace/metroid-decomp
printf '9-9 START\n18-18 START\n900- DOWN\n' > /tmp/tourian.txt
./target/debug/game-shell-pc --headless --frames 1500 \
  --start-area 0 --start-room 0x2C --start-map 3,2 --start-pos 0x80,0x63 \
  --start-frame 890 \
  --poke 880:0x687B=0x81 --poke 880:0x687C=0x82 \
  --script /tmp/tourian.txt --state-out /tmp/tourian.csv
```

- `$687B=0x81` / `$687C=0x82` = 「已击杀 + 已抬起」（bit7 置位、低 3 位保留击杀值）；两字节都置位后桥的开放判据在下一帧成立。
- 起点选 Room2C（电梯房）而不是 Room2B，是为了把「过桥」从「电梯起飞」里解耦；两侧都能跑通后再串起来。
- 判读：`--state-out` 的 CSV 第 5 列 `room`、第 6/7 列 `map_x,map_y`、第 20 列 `in_area`。期望 `in_area` 从 `0` 变为 `3`、`room` 变为 `0x0E`、`map` 变为 `3,4`。

### §8.2 预期换区帧窗（推导，B 级——非实测）

按下 DOWN 后：

| 段 | 帧数 | 依据 |
|---|---|---|
| ScrollXToCenter | 0（Room2C 最右结构 x=$E ⇒ 240px < 256，`ScrollX` 恒 0） | `Room2C` 结构表；`07:D80E` 的 `lda ScrollX / bne` |
| ElevatorFade（status5，淡出） | 1 | `07:D8A3` 空壳单帧 |
| ElevatorMove | ≈240 | y 从 `$83` 起每帧 ±1，`cmp #SCRN_VY` 回绕，走满一屏后回到 `$83`（`07:D83D` 的 `cmp #$83`） |
| ElevatorScrollY | 0..≈240，取决于 `ScrollY` 初值；Room2C 高约 11 行（176px）⇒ 通常 0 | `07:D870` 的 `lda ScrollY / bne` |
| `D8BF` 换区 + status7 淡入 + MoreInit | ≈4..10 | `07:D8BF`、`07:D8A3`、`07:C801` |

⇒ **按下 DOWN 后约 240–260 帧内应换区**（典型 ≈245）。若 >300 帧仍未变，优先查 `SamusOnElevator` 是否被置位、以及 `ElevatorMove` 的 y 起点（起点若不是 `$83`，窗要顺移）。

### §8.3 本卡实测缺口（诚实记录）

| 尝试 | 命令要点 | 观测 | 结论 |
|---|---|---|---|
| A | `--start-room 0x2C --start-map 3,2 --start-pos 0x80,0x63 --start-frame 900`，DOWN 从 f1000 | f905 坐标落到 `(80,63)`；到 f1131 房间变 `0x2B`、MapPos 变 `4,2`，f1133-1135 房号出现 `0xF1/0xF3/0xFF`（无效房） | 起点态不成立：DOWN 没让电梯起飞，反而触发右门链到桥房，随后房号异常。**未产出可用帧窗** |
| B | `--start-room 0x2B --start-map 4,2 --start-pos 0xE0,0x6F --start-frame 890`，LEFT/带/不带 pokes | 房 `0x2B` 装载成功；`--start-pos` 未生效（坐标停在 `0x80,0xB0`），未走到桥/未进 2C | 摆位未生效，未完成 |

- 失败原因的最小候选：①`--start-pos` 的落笔条件（`frame > start.frame` 且任务机空闲）在 `Room2C` 之外不满足；②`SamusOnElevator($0307)` 需要在电梯碰撞段被置位，直接摆坐标可能不进「站顶」判据（`physics.rs` 的 `is_samus_on_top_effect` 需要 `dist_y+1==radiusY`）。
- **推翻方法**：在 `--state-out` 里补 `$0307`、电梯 `status/y`（`$0300+0x?` 槽15）两列；或按 §8.1 串成「Room2B → 左门 → Room2C → DOWN」一条脚本，先确认桥补全后能过门。

## §9 复跑命令

```bash
cd /home/peng/workspace/metroid-decomp

# 1) 世界图取值（到达房号）
python3 - <<'PY'
import re
src=open('refs/m1disasm/SRC/prg0_title.asm').read().splitlines()
i=next(k for k,l in enumerate(src) if 'WorldMap: ; 00:A53E' in l)
rows=[]
for l in src[i+1:i+33]:
    v=re.findall(r'\$([0-9A-F]{2})',l)
    if v: rows.append([int(x,16) for x in v])
for name,y,x in [("Brinstar",0x0E,3),("Norfair",0x0D,0x16),("Kraid",0x14,7),
                 ("Tourian",0x04,3),("Ridley",0x18,0x19)]:
    print(f"{name}: MapPosY={y:#04x} MapPosX={x:#04x} -> Room {rows[y][x]:#04x}")
PY

# 2) 电梯参数全表（每个区的 it_Elevator）
grep -rn "it_Elevator" refs/m1disasm/SRC/*/items.asm

# 3) 桥的开放判据
sed -n '5416,5432p' refs/m1disasm/SRC/prg7_engine.asm

# 4) 两个状态的写点
sed -n '6018,6060p' refs/m1disasm/SRC/prg7_engine.asm   # LDD5B / LDD75
sed -n '8920,8945p' refs/m1disasm/SRC/prg7_engine.asm   # GetEnemyType 的雕像门

# 5) 门解码常量
sed -n '8996,9115p' refs/m1disasm/SRC/prg7_engine.asm  # SpawnDoorRoutine + DoorXs/DoorSlots

# 6) 最短路径（§8.1 的命令）
```

期望读数：§9.1 打印 `Tourian: MapPosY=0x04 MapPosX=0x03 -> Room 0x0e`；§9.2 十行 `it_Elevator`（Brinstar3 + Norfair3 + Kraid2 + Ridley1 + Tourian2 = 11 行，含 Brinstar `$03` 注释 "Elevator to Tourian"）。

## §10 假设区（C 级，需实测推翻）

- **C1**：`ElevatorMove` 的 y 走满一屏是必要条件（因 `cmp #$83` 在 ±1 之后才比较）。若实际录像显示更短/更长，说明 `$83` 比较语义不同（例如它是「电梯停在屏幕底」的位置而非行程终点）。推翻方法：Mesen 断点 `07:D866` 记录 y 序列。
- **C2**：Room0E 向上滚屏能进 Room01（无门）。依据是世界图 (y3,x3)=`$01` 与 Room0E 无门对象；未被 trace 证实。推翻方法：从 Room0E 顶部入位，看 `RoomNumber` 是否变 `$01`。
- **C3**：§8.2 的 0 帧 ScrollY 段以「Room2C 高约 11 行」为前提；若房间实际高度 >15 行，ScrollY 初值非 0，窗要加 0..240 帧。
- **C4**：`$6987（KraidRidleyPresent）` 只服务暂停/地图显示，与 Tourian 门无关。读点只有 `07:4124/4128`，但未排除间接读取（扁平寻址）；推翻方法：全 ROM 搜 `$6987` 的绝对/索引寻址（当前 `grep -n "\$6987"` 0 条是符号名未出现在指令里，需查两条 `lda KraidRidleyPresent` 之外的数据表引用）。
