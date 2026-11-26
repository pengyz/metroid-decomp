# 敌人类型分发与 AI 参数表（B10 前置规格）

> 范围：`etype → AI 例程` 的分发表、每区 10 张 16 项行为参数表、`status` 七态状态机与其写点、
> 与 Rust `crate::enemy` 的逐项对照。**只取证，不改代码**。
> 本卡不重复 #40（哪条例程没端口）、#63（敌人记录怎么落位）、#46（`JumpEngine`/`LoadTableAt977B` 的机器）。

## 0. 一页结论

| 项 | 结论 | 级 |
|---|---|---|
| 分发是**两层** | ①`status` 七态 → 框架例程（`07:F3BE` 起，共用，无表）；②`etype` → **每区一张** 16 项 `ChooseEnemyAIRoutine`（`01`–`05:95E5`）；③移动串由 `movementIndex` → `EnemyMovementPtrs`（40 词，R/L 成对） | A |
| 同一 `type` 号跨区含义不同 | etype 只是「区内序号」，`02` 在 Brinstar 是 Waver、在 Norfair 是 Ripper II、在 Ridley 是 dessgeega | A |
| 参数表是**每区 10 张 × 16 项** | `$962B` 血 / `$963B` 静止动画 / `$965B` 活动动画 / `$967B` 动画初偏移 / `$968B` 力属性 / `$969B` `data0D` / `$96AB` 距离阈 / `$96BB` 初始延迟 / `$96CB` 移动选择偏移 / `$977B` 朝向+加速位 | A |
| 血量不是单表 | `health = EnemyHealthTbl[type]`，若 `specialAttribs` bit7（tough）则 `<<1` | A |
| `$C0` 高两位有语义 | bit7 = tough（血×2、调色板3）；bit6 = Kraid/Ridley 本体（装载时查雕像状态） | A |
| 伤害不按 etype | 每区 2 字节 `AreaEnemyDamage`（`…:95CE`），读者 `07:F306`；五区 128/256/768/512/576 | A |
| Kraid / Ridley 血 | Kraid（Kraid 区 type8）`$60`=96；Ridley（Ridley 区 type9）`$8C`=140；斑马族 lint/nail `$FF`=255 | A |
| Metroid 血 | Tourian type0 红 / type1 绿都是 `$FF`（255，即无敌表值），不是 50 | A |
| Rust 侧 | 分发与参数表**基本已端口**（`enemy/ai_table.rs` 五区表、`assets::enemy_*` 12 个 per-area 访问器、`framework::choose_enemy_ai`） | A |
| Rust 缺口 | 无独立缺口；两处「行为待验」见 §4.3 | B |

## 1. 分发层

### 1.1 `etype` → AI 例程（`ChooseEnemyAIRoutine`）

**每区一份，地址相同 `…:95E5`，但表项指向本区 bank 里的例程**（`prg{1..5}_*.asm` 各 include 一次）。
机器是 **6 字节前缀 + 16 词**（前缀 `BD 02 6B`=`lda EnsExtra.0.type,x`、`20 24 80`=`jsr $8024` JumpEngine；表体从 `95EB` 起）：

```bash
python3 - <<'PY'
import pathlib
prg=pathlib.Path('rom/Metroid (USA).nes').read_bytes()[16:16+128*1024]
def w(bank,addr,n=32):
    o=bank*0x4000+(addr-0x8000); b=prg[o:o+n]
    return [b[2*i]|b[2*i+1]<<8 for i in range(n//2)]
for bank in range(1,6): print(bank, ' '.join('%04X'%x for x in w(bank,0x95EB)))
PY
```

实跑（读词起点 `95EB`，即前缀 6 字节之后）：

| type | `01` Brinstar | `02` Norfair | `03` Tourian | `04` Kraid | `05` Ridley |
|---|---|---|---|---|---|
| 00 | `99B8` SidehopperFloor | `98D3` Swooper00 | `97F9` **Metroid（红）** | `991C` SidehopperFloor | `98D7` Swooper00 |
| 01 | `99D3` SidehopperCeiling | `9908` Swooper01 | `97F9` **Metroid（绿）** | `9937` SidehopperCeiling | `990C` Swooper01 |
| 02 | `99E5` Waver | `98C0` RipperII | `9A27` 未命名（"takes 30 dmg with varia"） | `95CB` `RTS` | `9847` SidehopperFloor（dessgeega） |
| 03 | `99D8` Ripper | `9833` RemoveEnemy | `97DC` RemoveEnemy | `993C` Ripper | `9862` SidehopperCeiling |
| 04 | `99FA` Skree | `9833` RemoveEnemy | `9A2C` **Rinka** | `9949` Skree | `982A` RemoveEnemy |
| 05 | `9A4C` Crawler | `9833` RemoveEnemy | `97DC` RemoveEnemy | `999B` Crawler | `982A` RemoveEnemy |
| 06 | `9AF5` Rio | `9996` Crawler | `97DC` RemoveEnemy | `95CB` `RTS` | `9967` Crawler |
| 07 | `9B32` PipeBug | `9850` PipeBug | `97DC` RemoveEnemy | `9A44` PipeBug | `9867` PipeBug |
| 08 | `9BA2` Kraid（原版崩） | `9833` RemoveEnemy | `97DC` RemoveEnemy | `9AB4` **Kraid** | `982A` RemoveEnemy |
| 09 | `9BD2` KraidLint | `9833` RemoveEnemy | `97DC` RemoveEnemy | `9AE4` KraidLint | `9A13` **Ridley** |
| 0A | `9C1A` KraidNail | `9833` RemoveEnemy | `97DC` RemoveEnemy | `9B2C` KraidNail | `9A4A` RidleyFireball |
| 0B | `0000` 空指针 | `9A64` **Squeept** | `97DC` RemoveEnemy | `95CB` `RTS` | `982A` RemoveEnemy |
| 0C | `0000` | `9AD6` Multiviola | `97DC` | `95CB` | `9B03` Multiviola |
| 0D | `0000` | `9AE9` Dragon | `97DC` | `95CB` | `982A` RemoveEnemy |
| 0E | `0000` | `9B64` Polyp | `97DC` | `95CB` | `9B16` Polyp |
| 0F | `0000` | `9833` RemoveEnemy | `97DC` | `95CB` | `982A` RemoveEnemy |

三种「无事可做」在机器上不同，**不能合并**：

| 表项 | 机器 | 语义 | 备注 |
|---|---|---|---|
| `RemoveEnemy ; 07:FA18` | `lda #enemyStatus_NoEnemy / sta EnsExtra.0.status,x / rts` | 清槽 | Tourian/Norfair/Ridley 大量使用 |
| `RTS_95CB`（区 bank 的 `Exit__`） | 裸 `rts` | **什么都不做**，槽保持 | 只在 Kraid 区出现 |
| `$0000` | `jmp ($0000)` | 原版跳零页 ⇒ 行为未定（崩） | 只在 Brinstar `0B`–`0F`；Rust 记 `Null` 并计数 |

`JumpEngine`（`07:C27C`）取 `A=type` 后 `asl` 只保留低 7 位是 #46 的结论，本表 16 项不受影响。

### 1.2 `movementIndex` → 移动串（`EnemyMovementPtrs`）

AI 例程里「按指令移动」的那一步走 `LoadEnemyMovementPtr`（`areas_common.asm:546`，`$822B`）：

```
lda Ens.0.data05,x / bpl … / lsr lsr      ; bit7=竖直朝向，取高位
lsr                                        ; carry = 朝向位
lda Ens.0.movementIndex,x
rol / asl / tay                            ; y = ((idx*2)+facing)*2
lda EnemyMovementPtrs,y → EnemyMovementPtr
```

表本身 **20 对 / 40 词**（`…:96DB`，每区一份、内容同区不同）：`00`–`11` 为 `EnemyMovementNN_R/L`，`12`–`13` 两组 `$0000`（#46 已解 `12`–`13`；`14+` 超出表体）。
`movementIndex` 的实际来源是**每区的 `EnemyMovementChoiceOffset` 表（`$96CB`，16 B）** 加 `EnemyMovementChoices`（`$97D1` 起）里的指令串——`FrameCount/RNG` 组合选播，正是 #63 记的「激活时 RNG 选播」。

> 本卡只解码 `movementIndex` 的入口与 `$96CB` 的偏移值（§2 表 9 列），**不**逐条解释 `EnemyMovementChoices` 指令串（属移动系统，另卡）。

## 2. 行为参数表（每区 10 张 × 16 项）

十张表在每区 bank 的**同一批地址**上（`$962B`–`$977B`），每张 16 B，索引 = `etype & $3F`（`EnemyMovementChoices` 另算）：

| 序 | 标签 | 地址 | 长度 | 谁读 | 字段落点 |
|---|---|---|---|---|---|
| 0 | `EnemyHealthTbl` | `$962B` | 16 B | `InitEnemyForceSpeedTowardsSamusDelayAndHealth 07:F85A` | `Ens.health(+11)`，tough 时 `<<1` |
| 1 | `EnemyRestingAnimIndex` | `$963B` | 16 B | `InitEnRestingAnimIndex 07:F682` | `EnsExtra.animIndex(+6)` 初值 |
| 2 | `EnemyActiveAnimIndex` | `$965B` | 16 B | `InitEnActiveAnimIndex 07:F699` | 同上（激活时重写） |
| 3 | `EnemyActiveAnimIndexInitOffset` | `$967B` | 16 B | 同上 | `EnsExtra.resetAnimIndex(+5)` |
| 4 | `EnemyForceAttribs`（`L968B`） | `$968B` | 16 B | `LoadBit5ofTableAt968B` | 决定 `ForceSpeed` 是否提前退出（bit5） |
| 5 | `EnemyForceSpeedTowardsSamusDelayTbl` | `$969B` | 16 B | `07:F85A` | `Ens.data0D(+13)` |
| 6 | `EnemyDistanceToSamusThreshold` | `$96AB` | 16 B | `UpdateEnemy_EnData05DistanceToSamusThreshold` | 阈值比较 |
| 7 | `EnemyInitDelayTbl` | `$96BB` | 16 B | `UpdateEnemy_Active`（bit5 径） | `Ens.delay(+9)`，然后 Active→Resting |
| 8 | `EnemyMovementChoiceOffset` | `$96CB` | 16 B | 激活选播 | `movementIndex` 基值 |
| 9 | `L977B` | `$977B` | 16 B | `LoadTableAt977B` | 朝向/加速位：bit6=用加速度径、bit7=…（见 §3.3） |

**血量合成**（`07:F85A`–`07:F86F`）：`health ← table[type]`；再 `ldy specialAttribs`，`bpl`（bit7 清）则直接存，否则 `asl`（×2）再存。原版注释解释了 Rippers 的 `$FF/$FE` 特例：`FF` 无敌值经 `asl` 变成 `FE`——**强版 ripper 是 254 血而不是 255**。

### 2.1 三族取值（实跑字节，key 为 etype）

```bash
python3 - <<'PY'
import pathlib
prg=pathlib.Path('rom/Metroid (USA).nes').read_bytes()[16:16+128*1024]
def d(b,a,n):
    o=b*0x4000+(a-0x8000); return list(prg[o:o+n])
for bank,label,idxs in [(3,'Tourian',[0,1]),(4,'Kraid',[8,9,10]),(5,'Ridley',[2,3,9])]:
    print('==',label)
    for a,nm in [(0x962B,'health'),(0x968B,'force'),(0x969B,'data0D'),(0x96BB,'initDelay'),(0x96CB,'movChoice'),(0x977B,'facing')]:
        b=d(bank,a,16); print(nm, {i:b[i] for i in idxs})
PY
```

实跑输出（本机）：

| 区/型 | health | force(`$968B`) | data0D(`$969B`) | initDelay(`$96BB`) | movChoice(`$96CB`) | facing(`$977B`) |
|---|---|---|---|---|---|---|
| Tourian type0 红 Metroid | `FF` | `FE` | `01` | `01` | `00` | `50` |
| Tourian type1 绿 Metroid | `FF` | `FE` | `01` | `01` | `02` | `50` |
| Kraid type8 本体 | `60` | `83` | `01` | `10` | `0E` | `04` |
| Kraid type9 lint | `FF` | `00` | `01` | `00` | `11` | `00` |
| Kraid typeA nail | `FF` | `00` | `01` | `00` | `13` | `00` |
| Ridley type9 本体 | `8C` | `81` | `10` | `10` | `0A` | `64` |
| Ridley type2 dessgeega | `08` | `89` | `01` | `03` | `00` | `64` |

读数：Metroid 的 `force=FE` = bit7 与 bit6…bit1 全 1，只 bit0 清（朝向表里 `50` 的 bit6=1 ⇒ 用加速度径、bit4=1）；Kraid 本体 `0x83` 的 bit7=1、bit1=1（**不是** tough 位语义，`force` 与 `specialAttribs` 是两张不同的表，勿混）。

### 2.2 `specialAttribs`（`+15`）的两位才是 tough/miniboss

| 位 | 何时置 | 效果 | 证据 |
|---|---|---|---|
| bit7 | `GetEnemyType 07:EB28`：`type & $C0` | `health <<1`；`UpdateEnemyCommon@noMoveNoAnim` 里改 `ObjectCntrl` 调色板3 | `07:EB2A-07:EB2F`、`07:F41A-07:F425`、`07:F866` |
| bit6 | 同上（`type & $C0` 的 bit6） | 装载时若 `KraidStatueStatus` 已死则跳过该敌（Kraid/Ridley 语义） | `07:EB30-07:EB4C` |
| `$0A`/`$03` | 受击路径 `07:F5E0-07:F5EB` 覆写：普通敌 `0A`、迷你 Boss `03`（`Temp0A_EnSpecialAttribs` bit6） | 迷你 Boss 免击退/不同调色板路径 | `07:F5E0-F5EB` |

### 2.3 伤害：按区分档，不按 etype

十张表里**没有伤害表**；伤害来自每区 2 字节的 `AreaEnemyDamage`（`01`–`05:95CE`），由
`SetHealthChangeToAreaEnemyDamage ; 07:F306-07:F30F` 在「敌撞 Samus / 敌弹撞 Samus」时整字写进
`HealthChange`（16 位小端，`07:F309-F30F` 逐字节 `lda/sta`）：

| 区 | `AreaEnemyDamage` | 值（小端） | 含义 |
|---|---|---|---|
| `01` Brinstar | `80 00` | 128 | — |
| `02` Norfair | `00 01` | 256 | — |
| `03` Tourian | `00 03` | 768 | 最高 |
| `04` Kraid | `00 02` | 512 | — |
| `05` Ridley | `40 02` | 576 | — |

注意这是**基础伤害**：tough / miniboss / 特殊敌人的额外伤害走受击路径里的 `specialAttribs` 分支（§2.2），
不在这张表里；表本身也**不是** per-etype 的——卡面问的「伤害字段」在参数表里不存在，这就是该问题的答案。

## 3. 状态机

### 3.1 `status` 七态（`$6AF4+slot*16+0`，`EnsExtra.status`）

| 值 | 名字 | 处理例程 | 迁移写点（A） |
|---|---|---|---|
| 0 | `NoEnemy` | `rts` | `RemoveEnemy 07:FA18`；生成前置 0 |
| 1 | `Resting` | `UpdateEnemy_Resting 07:F3BE` | `07:F7C6` 若 `data05 bit3=0` 则 `inc` 退回；`07:F7D0+` 激活时 `inc`→2 |
| 2 | `Active` | `UpdateEnemy_Active 07:F3E6` | `07:F3FC` `dec`→1（`data05 bit5=1` 静息径） |
| 3 | `Explode` | `UpdateEnemy_Explode 07:F43E` 族 | 受击致死链 `07:F57x`；`CompleteDeath 07:DD75` 之后清槽 |
| 4 | `Frozen` | `UpdateEnemy_Frozen 07:F43E` | 冻结弹/斑马族自冻（Rust `metroid.rs:185` 同址语义） |
| 5 | `Pickup` | `UpdateEnemy_Pickup 07:F57D` | 拾取时由 `07:DD35` 家族置 |
| 6 | `Hurt` | `UpdateEnemy_Hurt 07:F4EE` | `07:F5E0` `lda #enemyStatus_Hurt / sta` |

`slot_cycle` 的遍历只会把 `status != 0 && status < 3` 的敌做「可见性早退」（`07:F351` `ldy status / beq / cpy #$03 / bcs`），所以 `3`–`6` 的敌不受离屏裁剪——Rust `enemy/slot_cycle.rs:31` 与之一致。

### 3.2 `data05`（`+5`）的位语义

| 位 | 语义 | 写点 |
|---|---|---|
| bit7 | 竖直朝向（`LoadEnemyMovementPtr` 里 `lsr lsr` 取） | `LEB4D`/`InitEnemyFacing` |
| bit6 | 30FPS 标记（`enemy_active` 里 `asl/bmi` 跳过本帧） | `UpdateEnData05Bit6`（每帧翻转） |
| bit5 | 静息标记：Active 里置 `delay=InitDelayTbl[type]` 后 `dec status` | `07:F3F4-F3FC` |
| bit4 | `RemoveIfInLava` 相关（`SpawnEnProjectile` 读） | 区域相关 |
| bit3 | 允许激活（Resting 延迟归零时判） | `07:F7C6` 读；`LEB6E` 写 |
| bit0 | 水平朝向 | `InitEnemyFacing` / `LEB6E` |

### 3.3 `L977B`（`$977B`，`+15` 的 `data1F` 来源）

`UpdateEnData1F 07:F676`：`L977B[type] <<1 <<3 & $C0` ⇒ `data1F` 的 bit6/bit7，供 `EnemyFlipAfterDisplacement` 与 `EnemyIfMoveFailed` 决定位移符号（#16 已修 Rust `sidehopper.rs` 的判据）。`LoadTableAt977B` 自身返回 `L977B[type] <<1`，故调用点测的是原字节 bit6。

## 4. Rust 对照（`crates/game-core/src/enemy/`）

| 面 | ROM 锚 | Rust | 状态 |
|---|---|---|---|
| 五区 AI 表 | `…:95E5` 16 项 ×5 | `enemy/ai_table.rs:29-193`（`Routine` 枚举 `:29`、五张 `[Routine;16]` `:83` 起、`table()`/`routine_for()` `:193/:209`） | 一致（含 `Null`/`Rts95CB`/`RemoveEnemy` 三臂区分） |
| 派发调用 | `07:F40D` | `framework.rs:549 choose_enemy_ai`（`resting`/`active` 尾部各一次，`framework.rs:610/637`） | 一致 |
| 参数表 10 张 | `$962B`–`$977B` | `game-data/src/assets.rs:322-346`（12 个 per-area 访问器；`enemy_table(area,idx)` 对非法区 fail-loud） | 一致 |
| 血量合成 | `07:F85A` | `kraid.rs:186`、`pipe_bug_hole.rs:170`、`rinka.rs:352`、`swooper.rs:166` 各点 | 一致 |
| `L977B`/`L968B` | `$977B`/`$968B` | `enemy/tables.rs:13 load_table_at_977b`、`l977b_raw`、`l968b_of` | 一致 |
| 移动指针表 | `$96DB` 40 词 | `assets::enemy_movement_ptrs` + `EN_MOV_CHOICES_LEN=[30,22,24,4,28]` | 一致 |
| 七态框架 | `07:F3BE`–`F59D` | `enemy/slot_cycle.rs:40-49` 的 `match status`（0–6 + `unported!`） | 一致 |
| Metroid/Kraid/Ridley 三族 AI | `03:97F9`/`04:9AB4`/`05:9A13` | `metroid.rs:102`/`kraid.rs:73`/`ridley.rs:29` | 一致 |
| `specialAttribs` bit7 血×2 | `07:F866` | 见上「血量合成」各点 | 待核对：本卡未逐点验证 `<<1` |
| `specialAttribs` bit6 雕像跳过 | `07:EB30` | `world/room_draw.rs:823 load_enemy` **无此段**（#67 A 级同结论） | **缺失**（归 #67 的卡，本卡只登记） |
| 受击覆写 `$0A/$03` | `07:F5E0` | `framework.rs` 受击径 | 待核对（未逐行比 `Temp0A` 分支） |

### 4.1 为什么「缺 bit6 段」值得单独点名

`GetEnemyType` 里 bit6 置位的敌，装载前会查 `KraidStatueStatus-1,y`（`InArea&6>>1`）：已击败则 `pla` 后直接 `rts` 跳过（`07:EB30-07:EB4C`）。
Rust `load_enemy`（`world/room_draw.rs:823`）没有这段，所以**打完 Kraid/Ridley 再进房会重新刷出 Boss**（#67 已作 A 级发现）。本卡不修，只把它列为「分发层的输入语义缺口」。

## 5. 复现命令（全部实跑过）

```bash
# 5.1 五区 AI 表（20 词/区）
python3 - <<'PY'
import pathlib
prg=pathlib.Path('rom/Metroid (USA).nes').read_bytes()[16:16+128*1024]
def w(b,a,n=32):
    o=b*0x4000+(a-0x8000); x=prg[o:o+n]; return [x[2*i]|x[2*i+1]<<8 for i in range(n//2)]
for b in range(1,6): print(b, ' '.join('%04X'%v for v in w(b,0x95EB)))
PY

# 5.2 三族参数（key 按 etype）
python3 - <<'PY'
import pathlib
prg=pathlib.Path('rom/Metroid (USA).nes').read_bytes()[16:16+128*1024]
def d(b,a,n):
    o=b*0x4000+(a-0x8000); return list(prg[o:o+n])
for b,l,ix in [(3,'Tourian',[0,1]),(4,'Kraid',[8,9,10]),(5,'Ridley',[2,3,9])]:
    print(l, 'health', {i:d(b,0x962B,16)[i] for i in ix},
          'force', {i:d(b,0x968B,16)[i] for i in ix},
          'facing', {i:d(b,0x977B,16)[i] for i in ix})
PY

# 5.3 状态枚举与写点（asm）
grep -n "enemyStatus_" refs/m1disasm/SRC/constants.asm
grep -n "sta EnsExtra.0.status,x" refs/m1disasm/SRC/prg7_engine.asm

# 5.4 Rust 侧接线
grep -rn "choose_enemy_ai" crates/game-core/src/enemy/framework.rs
grep -n "pub fn enemy_" crates/game-data/src/assets.rs
```

```bash
# 5.5 伤害（按区分档）
python3 - <<'PY'
import pathlib
prg=pathlib.Path('rom/Metroid (USA).nes').read_bytes()[16:16+128*1024]
for b in range(1,6):
    o=b*0x4000+(0x95CE-0x8000); print(b, prg[o:o+2].hex(' '))
PY
# 期望：1 80 00 / 2 00 01 / 3 00 03 / 4 00 02 / 5 40 02
```



## 6. 假设区（C 级，未取证）

- **C1**：Tourian type `02`（`9A27`）的语义未定。表注释写「30 damage with varia」，但 `HealthTbl` 给 `$01`，与「30 伤害」不自洽；可能是「特定伤害类型下取 30」。未取证，不据此改任何实现。
- **C2**：`force`（`$968B`）的 bit5 被 `LoadBit5ofTableAt968B` 用作「提前退出」，其余位（含 Metroid 的 bit6-7）本卡未找到唯一读者；不排除有间接读者（`Amul8` 等）读的是移位后值。
- **C3**：`RTS_95CB` 项若被激活，槽位永不清理（`status` 不变），长期存活可能与「原版某些敌人打不死」的传闻相关；未做运行时取证。

## 7. 未覆盖

- `EnemyMovementChoices` 指令串本体（每区长度不同，见 `EN_MOV_CHOICES_LEN`）只给了入口，未逐条解码。
- `UpdateEnemy_Explode`/`Frozen`/`Pickup`/`Hurt` 四个例程的内部逐行（本卡只给 `status` 写点）。
- Boss 三族的**专项**行为常量（Kraid 的 5 个手部槽、Ridley 的飞行曲线、Metroid 的吸附计时）属各自实现卡，不在本表。
