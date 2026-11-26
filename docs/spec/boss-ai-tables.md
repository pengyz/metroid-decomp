# 三只 Boss 的 AI 状态机与常量表（Kraid / Ridley / MotherBrain）

> 任务：#107（B11 前置）。范围：三只 Boss 的状态字节、每状态每帧动作、hp/伤害/计时常量、出现前置，
> 以及与 `crates/game-core/src/enemy/` 已端口部分的逐项对照。
>
> 口径说明（与 `docs/design/typed-struct-plan.md` 的批次名对齐）：计划里的「B3」是敌人槽 typed 化，
> 本卡按**内容**论证「Boss 行为规格」，不按批次名。卡面写「B11 前置」，即 B11（Boss 实现）开工前的取证。
>
> 证据级别：本文所有状态转移、常量数值、写点均为 A 级（asm 原文 + 本机 ROM hexdump 逐字节对上，
> 第 §6 给复跑命令）；「未找到写点/依赖运行期」的少数条目降到 B/C 级并显式标注。

## 0. 结论摘要

1. **Kraid 与 Ridley 没有独立的状态变量。** 它们是普通敌槽（`Ens`/`EnsExtra`，16 B/槽）+ 房间敌记录里的
   **bit6 = 小 Boss 标记**（`$48` = Kraid、`$49` = Ridley），状态字就是 `EnsExtra.0.status,x`（`$6AF4+x`）。
   找 `KraidStatus`/`RidleyStatus` 会落空——ROM 里没有这两个符号。
2. **MotherBrain 是独立状态机**：`MotherBrainStatus`（`$98`）十一态，`03:9DD4` 跳表分发；血量用
   `MotherBrainQtyHits`（`$99`）计到 `$20`（32 次）为止。
3. **出现前置有两条不同的门**：Kraid/Ridley 由房间敌记录的 bit6 + `GetEnemyType 07:EB28` 读
   `KraidStatueStatus-1,y`（`$687B/$687C`）决定装载；MotherBrain 不是敌记录，由 Tourian 区的
   **特殊物品** `it_MotherBrain($6)` 触发 `SpawnMotherBrainRoutine 03:9D21`。
4. 血量：Kraid `$60`(96)、Ridley `$8C`(140)（各自区 `EnemyHealthTbl`）；小 Boss 受击为
   **导弹 −4 / 波束 −2 / 其余 −1**。接触伤害是各区常量 `AreaEnemyDamage`：Kraid `$0200`(20)、
   Ridley `$0240`(24)、Tourian `$0300`(30)。
5. Rust 侧三只 AI 主路径都已端口（`enemy/kraid.rs`、`enemy/ridley.rs`、`enemy/mother_brain.rs`），
   但**缺两处前置**：房间敌记录的 bit6 雕像门（`world/room_draw.rs` 的 `load_enemy`）、
   MotherBrain 的生成例程（`pickup/spawn.rs:67` 仍是 divergence 占位）。详见 §7。

## 1. 三只 Boss 的共性

### 1.1 敌槽与状态枚举

- `Ens = $0400`、`EnsExtra = $6AF4`，每槽 16 B（`Ens` 前 7 B 为 `EnExtra` 前半，`EnsExtra` 是后 9 B）。
  三只 Boss 分别占：Kraid/Ridley 槽 0（`$0400`/`$6AF4`），MotherBrain 固定槽 `$E`（`$04E0`/`$6BD4`）。
- 状态枚举（`constants.asm:135-142`）：

| 值 | 符号 | 含义 |
|---|---|---|
| 0 | `enemyStatus_NoEnemy` | 空槽 |
| 1 | `enemyStatus_Resting` | 静置（未激活） |
| 2 | `enemyStatus_Active` | 活动 |
| 3 | `enemyStatus_Explode` | 爆炸/死亡 |
| 4 | `enemyStatus_Frozen` | 冻结 |
| 5 | `enemyStatus_Pickup` | 变成掉落物 |
| 6 | `enemyStatus_Hurt` | 受击 |

> 大于 6 视同空槽（`constants.asm:142` 注释）。

### 1.2 房间敌记录里的「小 Boss」位

`prg7_engine.asm:8913 GetEnemyType`（`07:EB28`）原文：

```
GetEnemyType: ; 07:EB28
    pha                             ;Store enemy type.
    and #$C0                        ;If MSB is set, the "tough" version of the enemy
    sta Ens.0.specialAttribs,x      ;is to be loaded(double health, except rippers).
    asl
    bpl Lx228                       ;If bit 6 is set, the enemy is either Kraid or Ridley.
        lda InArea
        and #$06
        lsr
        tay
        lda KraidStatueStatus-1,y
        beq Lx227
            pla
            pla                     ;Mini boss is dead so pull enemy info and last address off
            jmp Lx225               ;stack so next enemy/door item can be loaded.
        Lx227: ; 07:EB41
        lda #$01
        sta KraidRidleyPresent
    Lx228: ; 07:EB46
    pla
    and #$3F
    sta EnsExtra.0.type,x
    rts
```

要点：

- `specialAttribs ← etype & $C0`：**bit7 = tough（双血）**，**bit6 = 小 Boss**（Kraid/Ridley）；
- `asl/bpl` 测的是 `specialAttribs` 的 bit6（`asl` 把 bit6 送进 N）；
- 雕像索引 `y = (InArea & $06) >> 1`：Kraid 区 `InArea&$0F=2` ⇒ `y=1` ⇒ `KraidStatueStatus`（`$687B`）；
  Ridley 区 `=4` ⇒ `y=2` ⇒ `KraidStatueStatus-1+2 = RidleyStatueStatus`（`$687C`）；
- 雕像状态非 0 ⇒ 直接 `pla/pla` 把调用方返回地址与参数丢掉，**跳过这条敌记录**（不是置空槽，是不装载）；
- 为 0 ⇒ 置 `KraidRidleyPresent=1`（`$6987`，`constants_ram.asm:1234`），供 Tourian 音乐切换用；
- 最后 `type ← etype & $3F`，之后的表查全部用 6 位索引。

### 1.3 小 Boss 的伤害路径

`prg7_engine.asm:10760 EnemyReactToSamusProjectile`（`07:F536`）相关段：

```
    lda Ens.0.health,x
    cmp #$FF
    beq @ineffective            ; health=FF ⇒ 完全免疫
    ...
    ; branch if enemy is a miniboss
    asl
    bmi @endIf_D                ; specialAttribs bit6（小 Boss）⇒ 不走「一击必杀」
    ...
    lda #$0A                    ; 非小 Boss：specialAttribs ← $0A
    bit Temp0A_EnSpecialAttribs
    bvc @endIf_E
        lda #$03                ; 小 Boss：specialAttribs ← $03
    @endIf_E:
    sta Ens.0.specialAttribs,x
    cpy #wa_WaveBeam
    beq @damage_2               ; 波束 ⇒ 落到两连减
    bit Temp0A_EnSpecialAttribs
    bvc @damage_1
    ldy Ens.0.weaponAction,x
    cpy #wa_Missile
    bne @damage_1
    dec Ens.0.health,x           ; 小 Boss 被导弹打：连减 4
    beq ExplodeEnemy
    dec Ens.0.health,x
    beq ExplodeEnemy
@damage_2:
    dec Ens.0.health,x           ; 再减 1（波束总 −2）
    beq ExplodeEnemy
@damage_1:
    dec Ens.0.health,x           ; 再减 1（普通弹总 −1）
    bne ExplodeEnemy@exit
```

结论（A 级）：小 Boss 受到的伤害 = **导弹 4、波束 2、普通弹/其它 1**；`health=$FF` 判无效（Lint/Nail/Fireball 用 FF）。
非小 Boss 的普通弹/导弹是「一击必杀」（`ExplodeEnemy` 直跳），这条**不适用**于 Kraid/Ridley。

### 1.4 接触伤害

`AreaEnemyDamage`（各区 `:95CE` 的 `.word`），由 `07:F2A5` 的
`SetHealthChangeToAreaEnemyDamage` 写进 `HealthChange`：

| 区 | bank:addr | 字节 | 数值 | 折算能量 |
|---|---|---|---|---|
| Kraid | `04:95CE` | `00 02` | `$0200` | 20 |
| Ridley | `05:95CE` | `40 02` | `$0240` | 24 |
| Tourian | `03:95CE` | `00 03` | `$0300` | 30 |

> 折算按 `#105` 已定谳的 BCD 解包：`能量 = ((hi&$0F)*10) + (lo>>4)`。

## 2. Kraid（含 Lint / Nail）

源文件：`refs/m1disasm/SRC/enemies/kraid.asm`（完整 348 行），`prg4_kraid.asm:758` `.include`，
bank4 = Kraid 区（`InArea&$0F==2`）。ASM 里自身标了地址：`KraidNailAIRoutine ; L9B2C`、
`KraidUpdateAllProjectiles ; L9B2F`、`KraidProjectileOffsetX ;9BD1`、`KraidProjectileType ; L9BDB`。

### 2.1 状态机

`KraidAIRoutine`（`04:9AB4`，kraid.asm:4-34）：

```
    lda EnsExtra.0.status,x
    cmp #enemyStatus_Explode
    bcc KraidBranch_Normal      ; 0, 1, 2
    beq KraidBranch_Explode     ; 3
    cmp #enemyStatus_Pickup
    bne KraidBranch_Exit        ; 4(Frozen), 6(Hurt)
    ; 5 ⇒ fallthrough 到 Explode
```

| status | 分支 | 动作 |
|---|---|---|
| 0/1/2 | Normal | 更新 5 个弹体槽 → 尝试发 Lint → 尝试发 Nail → 尾件 |
| 3 | Explode | 清槽 1..5 的 `status` 为 0（**绝对地址，无 `,x`**），然后 return |
| 4（Frozen） | Exit | 只跑尾件（动画/公共更新），不发射 |
| 5（Pickup） | Explode | 与 3 同（fallthrough） |
| 6（Hurt） | Exit | 只跑尾件 |

- `KraidBranch_Explode` 的清除写法（kraid.asm:16-22）：
  `lda #enemyStatus_NoEnemy / sta EnsExtra.1.status / … / sta EnsExtra.5.status / beq Exit`——
  无论哪个槽进 Explode，清掉的都是 **1..5** 号槽（Kraid 自己槽 0 不清；留在 Explode 由公共死亡流程收）。
- 尾件：`lda #$0A / sta $00 / jmp CommonEnemyStub`（`enemies/sidehopper.asm` 的洞：
  `lda #$08 / sta $01 / jmp UpdateEnemyCommon_Decide`）。即 E1 动画节拍 `$0A`、E2 节拍 `$08`。

### 2.2 每帧动作（status 0/1/2）

1. `KraidUpdateAllProjectiles`（`04:9B2F`）：`ldx #$50` 起，每轮 `-$10`，`bne` 停 ⇒
   **x = $50,$40,$30,$20,$10（槽 1..5）**，不含槽 0。
2. `KraidTryToLaunchLint`（`04:9BE0`）：见 §2.4。
3. `KraidTryToLaunchNail`（`04:9C19`）：见 §2.4。
4. 尾件（见上）。

### 2.3 Lint / Nail（弹体）的 AI 与初始化

`KraidLintAIRoutine`（`04:9AE4`，kraid.asm:38-87）：

| 条件 | 动作 |
|---|---|
| `Ens.0.data05,x & $02 == 0`（不可见） | 清槽（`status←0`），去画 |
| `status == 3`（Explode） | 清槽，去画 |
| `data05` 的 `asl/bmi`（**bit6**）置位 | 只画 |
| `status != 2`（非 Active） | 只画 |
| 否则 | 算 ΔY/ΔX 存 `speedY/speedX`，`EnemyBGCollideOrApplySpeed`；**移动失败（撞墙/出界）⇒ `status←3`（自爆）** |

- 画：`lda #$01 / jsr UpdateEnemyAnim` + `jmp UpdateEnemyCommon_noMoveNoAnim`。
- `KraidNailAIRoutine ;04:9B2C` = `jmp KraidLintAIRoutine`（**同一段代码**，只有 type/表不同）。
- 初始化路径在 `KraidUpdateProjectile @resting`（kraid.asm:149-219）：
  - 从 Kraid 的 `data05` 复制方向位到弹体；
  - `y = x/16`（槽号），查 `OffsetY[y-1]` / `Type[y-1]`；
  - `y' = (槽号)*2 + data05.bit0`（朝向），查 `OffsetX[y'-2]`；
  - 用 `StoreEnemyPositionToTemp` / `ApplySpeedToPosition` 算出生坐标；出界 ⇒ 退出；
  - 成功 ⇒ `status ← status+1`（0→1，即 Resting），写回坐标与 `hi`。
- **`@remove` 的顺序 quirk**：先 `jsr CommonJump_0E`（用**旧** type 查 health/initDelay/forceSpeed 表），
  之后才在 `@resting` 用 `KraidProjectileType` 覆盖 type ⇒ 新槽的 health/delay 来自旧 type（刚出生时 type=0）。
- **Brinstar 专属 bug**（`kraid.asm:180-202` 的 `.if BANK != 1`）：bank1 版本按下 X 时不保存，
  注释写明「The Brinstar Kraid code makes an incorrect assumption about X, which leads to a crash」。
  Stage A 不涉及 Brinstar Kraid（bank1 里 Kraid 房不可达），只登记。

### 2.4 发射节奏（定量）

`KraidTryToLaunchLint`（kraid.asm:248-298）：

```
    ldy KraidLintCounter      ; #$80 if zero
    bne @endIf_A
        ldy #$80
    @endIf_A:
    lda FrameCount
    and #$02
    bne @RTS                  ; 每 2 帧才走一次
    dey
    sty KraidLintCounter
    tya
    asl                       ; ×2，补偿隔帧
    bmi @RTS                  ; ≥$80 ⇒ 停顿段
    and #$0F
    cmp #$0A
    bne @RTS                  ; 低半字节==$A 才发
```

⇒ 计数器从 `$80` 递减，`×2` 后 `bmi` 挡住 `$80..$C0` 段；进入 `$7F` 以下后每 16 帧命中一次
（低半字节 `$A`），连续 8 次，然后回到 `$80` 段**停 128 帧**。发射时把目标臂的 `delay ← $08`
（静置 8 帧后真正发射）。

`KraidTryToLaunchNail`（kraid.asm:303-347）：同结构，初值 `$60`，判据是
`(counter*2 & $0F) == 0`（每 16 帧一次）；两臂 `$40/$50`（槽 4/5）；`delay ← $08`。

槽位分工（从常量推）：Lint 槽 `$10/$20/$30`（槽 1/2/3），Nail 槽 `$40/$50`（槽 4/5）。

### 2.5 Kraid 常量表（ROM 已核）

| 表 | bank:addr | 字节 | 含义 |
|---|---|---|---|
| `KraidProjectileOffsetY` | `04:9BCC` | `F5 FD 05 F6 FE` | `[-11,-3,5,-10,-2]`，按槽号 1..5 |
| `KraidProjectileOffsetX` | `04:9BD1` | `0A F6 0C F4 0E F2 F8 08 F4 0C` | `[10,-10,12,-12,14,-14,-8,8,-12,12]`，第一列朝右 |
| `KraidProjectileType` | `04:9BDB` | `09 09 09 0A 0A` | 前 3 个 Lint（$09）、后 2 个 Nail（$0A） |
| `EnemyHealthTbl[8]` | `04:962B+8` | `60` | Kraid hp 96 |
| `EnemyHealthTbl[9]/[10]` | `04:962B+9` | `FF FF` | Lint/Nail 免疫常规伤害 |
| `EnemyInitDelayTbl[8]` | `04:96BB+8` | `10` | 出生静置 16 帧 |
| `L968B[8]` | `04:968B+8` | `83`=`%10000011` | bit7=1（碰撞伤害判定相关位清 0）、bit1/0=1 |

> `$04:9BD1` 与 `$01:9CBD` 是同一张表的副本（#93 已实测两侧字节相等）；本卡以 Kraid 区副本为准。

## 3. Ridley（含 Fireball）

源文件：`refs/m1disasm/SRC/enemies/ridley.asm`（完整 163 行），bank5 = Ridley 区（`InArea&$0F==4`）。
asm 自带地址：`RidleyAIRoutine ; 05:9A13`、`@explode ; 05:9A20`、`@normal ; 05:9A33`、
`RidleyFireballAIRoutine ; 05:9A4A`、`RidleyTryToLaunchFireball ; 05:9A79`、`RidleyFireballOffsetX ; 05:9ADF`。

### 3.1 状态机

与 Kraid **同构**（ridley.asm:2-33）：`status<3` ⇒ Normal；`==3` ⇒ Explode（清槽 1..5，绝对地址）；
`==5` ⇒ fallthrough 到 Explode；`4/6` ⇒ 只跑尾件。

| status | 分支 | 动作 |
|---|---|---|
| 0/1/2 | Normal | 写翻转动画索引 → `EnemyFlipAfterDisplacement` → 尝试吐火球 → 尾件 |
| 3 / 5 | Explode | 清槽 1..5 的 status，return |
| 4 / 6 | Exit | 尾件 |

Normal 段（ridley.asm:20-27）：

```
    lda #EnAnim_RidleyHopping_R - EnAnimTable.b
    sta EnemyFlipAfterDisplacementAnimIndex      ; $85
    lda #EnAnim_RidleyHopping_L - EnAnimTable.b
    sta EnemyFlipAfterDisplacementAnimIndex+1    ; $86
    jsr CommonJump_EnemyFlipAfterDisplacement
    jsr RidleyTryToLaunchFireball
```

尾件：`lda #$03 / sta $00 / sta $01 / jmp UpdateEnemyCommon_Decide`（E1/E2 节拍都是 3）。

### 3.2 Fireball 的 AI

`RidleyFireballAIRoutine`（`05:9A4A`）：

1. 压栈保存开头的 `data05`；
2. 跑 `UpdateEnemyCommon_Decide`（节拍 `$02/$02`）；
3. `pla / eor data05 / lsr / bcs` ⇒ **水平朝向位变了就自删**（`status←0`）；
4. `data05 & $01` 置位（朝左）⇒ 保留；
5. `Ens.x - Samus.x < 0`（火球在 Samus 左边）⇒ 保留；
6. `(Ens.x - Samus.x) < $20`（32px 内）⇒ 保留；
7. 否则（右侧且 ≥32px）⇒ 自删。

### 3.3 发射逻辑与常量

`RidleyTryToLaunchFireball`（`05:9A79`）：

- 计数器 `RidleyFireballCounter`（`$80`，`constants_ram.asm` 内已具名），初值 `$60`；
- 与 Kraid 同构：每 2 帧递减、×2、`bmi` 挡 `$80..`、`&$0F==$0A` 命中（16 帧一次 ×8 连发、停 128 帧）；
- 找槽：`x=$50..$10`，条件是 `status==0` **或** `data05&$02==0`（不可见可复用）；
- 全占用 ⇒ **`inc KraidLintCounter`（原版 bug，复制粘贴 Kraid 的残留，asm 注释自认）**；
- 找到槽：Ridley 坐标进 temp、`data05 ← Ridley 的 data05`（继承朝向）、
  `OffsetX = [8,-8][朝向]`、`OffsetY=$F8`(−8)、`ApplySpeedToPosition`；出界 ⇒ 退出；
  成功 ⇒ `specialAttribs←0`、`type←$0A`、`status←1(Resting)`、写回坐标、`jsr CommonJump_0E`。

| 表 | bank:addr | 字节 | 含义 |
|---|---|---|---|
| `RidleyFireballOffsetX` | `05:9ADF` | `08 F8` | `[+8,-8]`，按朝向位 |
| `EnemyHealthTbl[9]` | `05:962B+9` | `8C` | Ridley hp 140 |
| `EnemyHealthTbl[0A]` | `05:962B+10` | `FF` | Fireball 免疫 |
| `EnemyInitDelayTbl[9]` | `05:96BB+9` | `10` | 出生静置 16 帧 |
| `EnemyInitDelayTbl[0A]` | `05:96BB+10` | `01` | 火球静置 1 帧 |
| `EnemyForceSpeedTowardsSamusDelayTbl[9]` | `05:969B+9` | `10` | 每 16 帧追一次 |
| `L968B[9]` | `05:968B+9` | `81`=`%10000001` | bit7=1、bit0=1 |

## 4. MotherBrain

源文件：`refs/m1disasm/SRC/prg3_tourian.asm:1089-1728`（Tourian 区专属）。
`MotherBrainStatusHandler ; 03:9DD4`。

### 4.1 状态机（11 态）

```
MotherBrainStatusHandler: ; 03:9DD4
    lda MotherBrainStatus
    beq @RTS
    jsr CommonJump_JumpEngine
        .word Exit__                                  ; 00 不在房内
        .word MotherBrain_Idle                        ; 01 在房内
        .word MotherBrain_Hurt                        ; 02 受击闪
        .word MotherBrain_Killed                      ; 03 死亡闪+清场
        .word MotherBrain_Disappear                   ; 04 分五批消融
        .word MotherBrain_TimeBombMessage             ; 05 字幕（gone）
        .word MotherBrain_SetTimeBomb                 ; 06 设时炸弹
        .word MotherBrain_TimeBombExploded            ; 07 时炸弹爆炸
        .word MotherBrain_TimeBombMessage_ScrollBackOnScreen ; 08 = 05 同址同码
        .word MotherBrain_SetTimeBomb_ScrollBackOnScreen     ; 09 = 06 的一部分
        .word Exit__                                  ; 0A 已死
```

| status | 含义 | 动作（每帧） |
|---|---|---|
| 0 | 不在房内 | RTS |
| 1 | Idle | 碰撞判定 → 受击处理 → 脑动画 → 眼动画 → 画精灵 → 清 `IsHit` |
| 2 | Hurt | 闪调色板（`UpdateMotherBrainFlashDelay`，16 帧）→ 清 `IsHit` |
| 3 | Killed | 闪调色板 + 垂直抖屏（`ScrollY = idx*4`）→ 闪烁完（status 回 1）时：清 0..$B0 全部敌槽、`status←4`、`FlashDelay←$28`(40)、静音音乐 |
| 4 | Disappear | 每帧 BombExplode SFX → 消融一块瓦 → `DeathInstrID++` → 递减延迟；延迟到 0 时进下一批：`FlashDelay←$1C`(28)、`DeathStringID++`；第 4 批（>=4）后延迟减半（14）；>4 ⇒ 进 status5、`TimeBombCounter←$80`、播 escape 音乐 |
| 5/8 | TimeBombMessage | 逐片画 "TIME BOMB SET GET OUT FAST!" 八段字幕；计数器到 8 ⇒ `status++`（→6/9） |
| 6/9 | SetTimeBomb | 造门 → `status←0`、`EndTimer←9999`（`$99,$99`，166.65 秒）、`EndTimerEnemyIsEnabled←1`、`EndTimerEnemyHi←MBHi` |
| 7 | TimeBombExploded | 每帧 BombExplode SFX；`Timer3==0` 时 `Samus.status←sa_Dead2`、`status←$0A`、调色板回 0 |
| A | 已死 | RTS |

### 4.2 各子件细节

- **碰撞（`03:9DF2`）**：名表相同 + `Samus.x-$48 < $2F` + `|Samus.y-$80| < $20` ⇒
  `HealthChange←$0200`(20)、`Samus.isHit←$38`、`SubtractHealth`。
- **受击（`03:9FED`）**：`IsHit` 为 0 直接退；否则 BossHit SFX、`QtyHits++`；
  `QtyHits < $20` ⇒ `status←2`、`FlashDelay←$10`(16)；`>= $20` ⇒ 清全部 TileBlast、
  `status←3`、`FlashDelay←$80`(128)。
- **脑动画（`03:A01B`）**：`AnimBrainDelay--`；到 0 时 `frameID ← RandomNumber1 & 3`、
  `delay ← ($20 - QtyHits) >> 1`。
- **眼动画（`03:A02E`）**：`AnimEyeDelay--`；只有值为 `0` 或 `$80` 时动作；
  `delay ← (($20-QtyHits)|$80) ^ old`（bit7=闭眼）。
- **画精灵（`03:A041`）**：母脑固定坐标 `y=$70, x=$48`；`animFrame ← AnimFrameTable[frameID]`；
  若 `AnimEyeDelay` bit7 清（睁眼）再画一帧 `AnimFrameTable+4`（眼睛）。
- **消融（`03:A072`）**：用 `DeathStringID` 选一条指令串，逐字节把 `$6144+byte` 处 2×2 瓦清掉；
  遇到 `$FF` 指令串结束、`DeathInstrID` 回退（不前进）。
- **造门（`03:9F69`）**：`lda MapPosX / clc / adc MapPosY / sec / rol / and #$03`——注意 `sec` 让 `rol` 的进位恒 1，
  结果是 `((MapPosX+MapPosY)*2 + 1) & 3`，**只可能取 1 或 3**（和偶数⇒1、奇数⇒3），
  即 `DoorSlots[1]=Doors.3`（槽 11）或 `DoorSlots[3]=Doors.1`（槽 9）——**母脑的逃生门只落在左侧两个门上**，
  `DoorSlots_` 表里另外两个（`Doors.0/Doors.2`，右侧）是未用项。Rust `spawn_door` 用同样的
  `(((x+y) << 1) | 1) & 3` 复现了这一点。门立即关闭（`DoorHitPoints←1`）、蓝色（`DoorType←1`）、`status←3(open)`、
  坐标 `hi=MBHi, x=$10, y=$68`；第一帧 `_id_FrameNone`；随后画 `VRAMString10` 门瓦。

### 4.3 MotherBrain 常量表（ROM 已核）

| 表 | bank:addr | 字节 | 含义 |
|---|---|---|---|
| `MotherBrainAnimFrameTable` | `03:A06D` | `13 14 15 16 17` | Tourian 帧 id 19–23（脑 4 帧 + 眼） |
| `MotherBrainFlashPaletteTable` | `03:9E41` | `08 07` | 受击/死亡闪的两张调色板 id（+1 形） |
| `MotherBrain_Disappear_PaletteTable` | `03:9F00` | `09 0A` | 消融期两张调色板 |
| `@roomRAMPtrTable`（字幕位置） | `03:9F39` | `00 40 08 48 80 C0 88 C8` | 相对 `$6142` 的低字节 |
| `@animFrameTable`（字幕瓦） | `03:9F41` | `08 02 09 03 0A 04 0B 05` | 8 段 VRAM 串 id |
| `MotherBrainDeathString@1` | `03:A0A3` | `00 02 04 06 08 40 80 C0 48 88 C8 FF` | 第一批消融偏移（相对 `$6144`） |
| `MotherBrainDeathStringOffsets` | `03:A0C1` | `00 0C 11 16 1A` | 5 条指令串起点 |
| `DoorSlots_` | `03:9F65` | `80 B0 A0 90` | `Doors.0/3/2/1 - Objects`（槽 8/11/10/9） |
| 计时常量 | 代码立即数 | `$10`/`$80`/`$28`/`$1C`/`$20` | 受击闪 16 / 死亡闪 128 / 首批 40 / 批次 28 / 脑眼初值32 |

> `TIME BOMB` 计时：`EndTimer=$99,$99`（9999 帧）。字幕完成前 `TimeBombCounter` 从 `$80`（−128）
> 计到 0 才逐片推进——这段是「母脑死后到设炸弹」的等待，不是倒计时本体。

### 4.4 出现前置（MotherBrain）

- 触发：Tourian 区特殊物品记录 `it_MotherBrain`（`constants.asm:35`，值 `$6`）；
  `tourian/items.asm:129-134`（`@y0B @@x02`）一条：
  `.byte it_MotherBrain` + `it_Zebetite|$40` + `it_RinkaSpawner|$10` + 3 门 `it_Cannon`。
- 调用链：`prg7_engine.asm:9683 jsr GotoSpawnMotherBrainRoutine`（`03:95B4` 跳板）
  → `SpawnMotherBrainRoutine 03:9D21`。
- 生成时写入：`MotherBrainStatus←1`；`MotherBrainHi←GetNameTableAtScrollDir`；
  `ScrollBlockOnNameTable3` 的对应位置 1（**锁住母脑身后的横向滚动**）；
  `AnimBrainDelay=AnimEyeDelay=$20`。
- 房间：Tourian 区 `MapPosY=$0B`、物品列 `x=2`（`tourian/items.asm:120-134`）。
  母脑**不是**房间敌记录，所以没有「敌记录 bit6」门。

## 5. 出现前置汇总

| Boss | 载体 | 位置 | 门控 |
|---|---|---|---|
| Kraid | 房间敌记录 `.byte $01,$48,$95` | `kraid/rooms.asm:705`，房间 `Room1D`（`04:A8C7`） | `GetEnemyType` 读 `KraidStatueStatus($687B)`，非 0 ⇒ 不装载；为 0 ⇒ `KraidRidleyPresent←1` |
| Ridley | 房间敌记录 `.byte $01,$49,$66` | `ridley/rooms.asm:396`，房间 `Room12`（`05:A555`） | 同上，读 `RidleyStatueStatus($687C)` |
| MotherBrain | Tourian 特殊物品 `it_MotherBrain($6)` | `tourian/items.asm:129`（`MapPosY=$0B`、`x=2`） | 无雕像门；`SpawnMotherBrainRoutine` 直接置 status=1 |

- 记录字节含义：`$01` = 对象类型「敌」；`$48/$49` = `%0100_1000`/`%0100_1001`（bit6 小 Boss + 索引 8/9）；
  `$95/$66` = 坐标字节 `(y<<4)|x`，装载时 `LEB4D` 解成 `y=(pos&$F0)|8, x=(pos<<4)|$0C`。
- 雕像状态位的语义与写点见 `docs/spec/statue-bridge-chain.md`（#76）：`$687B` bit0=已击败、bit7=已抬起；
  本卡只引用，不重复。
- Kraid 房的门在记录里是 `$A0|(0<<4)|doorType_Red`（红门，`kraid/rooms.asm:704`）；
  Ridley 房是蓝门 + 10 导弹门（`ridley/rooms.asm:394-395`）。

## 6. 常量表复现命令

ROM 已知 sha256 `e6e6b7014685adae447ebb3833242815747bc1e5df83ade79f693fb67cf565b6`，16 字节 iNES 头。
bank0–6 的 `$8000` 对应文件偏移 `16 + bank*0x4000`，bank7 的 `$C000` 对应 `16 + 7*0x4000`。

```bash
python3 - <<'PY'
rom = open('rom/Metroid (USA).nes','rb').read()
def off(b,a):
    return 16 + (7*0x4000 + (a-0xC000) if b==7 else b*0x4000 + (a-0x8000))
def dump(b,a,n,label):
    o=off(b,a); print(label, "bank%02X:%04X off=0x%X"%(b,a,o),
                     " ".join("%02X"%x for x in rom[o:o+n]))
dump(4,0x962B,9,'HealthKr0..8')     # …08 60 ⇒ Kraid hp=$60
dump(5,0x962B,10,'HealthRi0..9')    # …01 8C ⇒ Ridley hp=$8C
dump(4,0x9BCC,5,'KraidOffsetY')     # F5 FD 05 F6 FE
dump(4,0x9BD1,10,'KraidOffsetX')    # 0A F6 0C F4 0E F2 F8 08 F4 0C
dump(4,0x9BDB,5,'KraidProjType')    # 09 09 09 0A 0A
dump(5,0x9ADF,2,'RidleyFireballDX') # 08 F8
dump(3,0xA06D,5,'MBanimFrame')      # 13 14 15 16 17
dump(3,0xA0A3,12,'MBdeathStr1')     # 00 02 04 06 08 40 80 C0 48 88 C8 FF
dump(3,0xA0C1,5,'MBdeathOff')       # 00 0C 11 16 1A
dump(3,0x9F39,8,'MBmsgPtr')         # 00 40 08 48 80 C0 88 C8
dump(3,0x9F41,8,'MBmsgAnim')        # 08 02 09 03 0A 04 0B 05
dump(3,0x9F65,4,'MBDoorSlots')      # 80 B0 A0 90
dump(7,0xEB28,12,'GetEnemyType')    # 48 29 C0 9D 0F 04 0A 10 15 A5 74 29
PY
```

本机实跑输出（与上表一致；`00 02` 是 `.word $0200` 的小端存储）：

```
AreaEnemyDamage(Kr) bank04:95CE off=0x115DE 00 02
AreaEnemyDamage(Ri) bank05:95CE off=0x155DE 40 02
AreaEnemyDamage(To) bank03:95CE off=0xD5DE 00 03
HealthKr0..8 bank04:962B off=0x1163B 08 08 00 FF 02 02 00 01 60
HealthRi0..9 bank05:962B off=0x1563B 08 08 08 08 01 01 02 01 01 8C
KraidOffsetY bank04:9BCC off=0x11BDC F5 FD 05 F6 FE
KraidOffsetX bank04:9BD1 off=0x11BE1 0A F6 0C F4 0E F2 F8 08 F4 0C
KraidProjType bank04:9BDB off=0x11BEB 09 09 09 0A 0A
RidleyFireballDX bank05:9ADF off=0x15AEF 08 F8
MBStatusHandler bank03:9DD4 off=0xDDE4 A5 98 F0 19 20 24 80 CB 95 22 9E 36 9E 52 9E 86
MBflashPal bank03:9E41 off=0xDE51 08 07
MBdisPal bank03:9F00 off=0xDF10 09 0A
MBmsgPtr bank03:9F39 off=0xDF49 00 40 08 48 80 C0 88 C8
MBmsgAnim bank03:9F41 off=0xDF51 08 02 09 03 0A 04 0B 05
MBanimFrame bank03:A06D off=0xE07D 13 14 15 16 17
MBdeathStr1 bank03:A0A3 off=0xE0B3 00 02 04 06 08 40 80 C0 48 88 C8 FF
MBdeathOff bank03:A0C1 off=0xE0D1 00 0C 11 16 1A
MBDoorSlots bank03:9F65 off=0xDF75 80 B0 A0 90
GetEnemyType bank07:EB28 off=0x1EB38 48 29 C0 9D 0F 04 0A 10 15 A5 74 29
```

入口字节核对（用于确认 §2/§3 的地址锚）：

```
KraidAI   bank04:9AB4 off=0x11AC4 BD F4 6A C9 03 90 19 F0
RidleyAI  bank05:9A13 off=0x15A23 BD F4 6A C9 03 90 19 F0
SpawnMB   bank03:9D21 off=0xDD31 A9 01 85 98 20 88 9D 85
MBSpawnDoor bank03:9F69 off=0xDF79 A5 50 18 65 4F 38 2A 29
```

`BD F4 6A` = `lda $6AF4,x`（`EnsExtra.0.status`）、`C9 03` = `cmp #3`、`90 19` = `bcc`、`F0` = `beq`——
与 kraid.asm:5-8 / ridley.asm:3-6 的 asm 原文逐字节一致；Kraid 与 Ridley 的**首 8 字节完全相同**（同构状态门）。
`SpawnMB` 的 `A9 01 85 98` = `lda #1 / sta $98`（`MotherBrainStatus←1`）；
`MBSpawnDoor` 的 `A5 50 18 65 4F 38 2A 29` = `lda MapPosX / clc / adc MapPosY / sec / rol / and`（§4.2 的门的槽位推导源）。

`GetEnemyType` 头 12 字节解码：`48`=pha、`29 C0`=and #$C0、`9D 0F 04`=sta $040F,x（`specialAttribs`）、
`0A`=asl、`10 15`=bpl +$15、`A5 74`=lda `InArea`、`29 …`=and #$06——与 §1.2 的 asm 原文逐字节一致。

## 7. 与 Rust 已端口部分的对照

对照范围：`crates/game-core/src/enemy/{kraid,ridley,mother_brain}.rs`、`enemy/framework.rs`、
`world/room_draw.rs`（敌记录装载）、`pickup/spawn.rs`（特殊物品分派）。只列差异，不判对错。

### 7.1 已一致（逐项核对过锚点）

| 项 | asm | Rust |
|---|---|---|
| Kraid 状态门（<3 Normal / 3 Explode / 5 fallthrough / 4,6 Exit） | `kraid.asm:4-34` | `enemy/kraid.rs` `kraid_ai` 同构 |
| Kraid Explode 清槽 1..5（绝对地址） | `kraid.asm:16-22` | 同一处 `for s in 1..=5` |
| Kraid 弹体槽 `$50..$10`（不含 0） | `kraid.asm:96-105` | `kraid_update_all_projectiles` 同 |
| Lint `data05 asl/bmi` 测 bit6 | `kraid.asm:56-58` | 注释与实现均按 bit6 |
| `@remove` 先 `CommonJump_0E` 后覆盖 type | `kraid.asm:143-170` | 注释显式保留顺序 |
| Ridley 状态门 / 翻转动画索引 / 尾件节拍 3 | `ridley.asm:2-33` | `enemy/ridley.rs` 同构 |
| Ridley 全槽占用 `inc KraidLintCounter`（原版 bug） | `ridley.asm:117-119` | 实现 + 单测保留 |
| Fireball 自删四条件 | `ridley.asm:46-64` | 同构 |
| MB 11 态分发（5 与 8 同址同码；6/9 各自独立例程） | `03:9DD4`/`03:9F02`/`03:9F49`/`03:9FDA` | `mother_brain.rs::status_handler` |
| MB 32 次受击 / 16 帧闪 / 128 帧死闪 / 40/28/14 批延迟 | §4.2 | 常量 `0x20/0x10/0x80/0x28/0x1C` 全在 |
| MB 消融 5 串 + 偏移表 + 2×2 清瓦 | `03:A0A3/A0C1` | `DEATH_STRINGS`/`DEATH_OFFSETS` 一致 |
| MB 造门槽只取 11/9（`sec`+`rol` 使索引恒奇）、坐标 `$10/$68`、`FrameNone` | `03:9F65`/`03:9F69` | `DOOR_SLOTS`/`FRAME_NONE` 一致 |
| MB `$99` union 三义（QtyHits/DeathStringID/TimeBombCounter） | `constants_ram.asm:715-731` | `mb_qty_hits` 访问器注释已写明 union |

### 7.2 缺（asm 有、Rust 无）

| # | 缺项 | asm 证据 | Rust 现状 | 影响 |
|---|---|---|---|---|
| M1 | **房间敌记录的 bit6 雕像门** | `07:EB28` 的 `asl/bpl` + `KraidStatueStatus-1,y` + `pla/pla` 跳过装载 | `world/room_draw.rs:823 load_enemy` 只写 `specialAttribs=etype&$C0`、`type=etype&$3F`，之后直接 `LEB4D` 装载；全仓 `0x687B/0x687C` 的写/读点只有：`world/statue.rs`（抬升与渲染，本地常量 `KRAID_STATUS/RIDLEY_STATUS`）、`title/bridge.rs:171`（结局判定传参）、`frame/tick.rs:224/238`（测试）、`ram_map.rs:148/149`（常量）；**装载径 `load_enemy` 无读点** | 雕像已倒的房间仍会刷出 Kraid/Ridley；与「Boss 已击败」状态不一致 |
| M2 | **`KraidRidleyPresent←1` 的写点** | `07:EB41 lda #$01 / sta KraidRidleyPresent` | `ram_map.rs:315 KRAID_RIDLEY_PRESENT` 只被 `samus/handler.rs:1681` 读、`:1684` 清；**无置 1 写点** | Tourian 音乐切换（Boss 在场 → TourianMusic）永不触发 |
| M3 | **MotherBrain 生成例程** | `SpawnMotherBrainRoutine 03:9D21`（status←1、`MBHi`、锁滚动、延迟 $20） | `pickup/spawn.rs:67` 对 `it_MotherBrain($06)` 只 `DIVERGENCES.push(0xEEAE)` 占位，无实现 | 母脑永不出现（status 停在 0），Tourian 终局不可达 |
| M4 | Kraid 的 Brinstar `X` 保存 bug | `kraid.asm:180-202` `.if BANK != 1` | 未端口（bank1 版本整体未做） | Stage A 无影响（bank1 Kraid 房不可达）；仅登记 |

### 7.3 多（Rust 有、asm 无）

本次对照未发现「Rust 凭空多出状态分支」；`kraid.rs`/`ridley.rs` 的函数集与 asm 标签一一对应。
但注意 `mother_brain.rs` 的测试辅助 `set_mb_hi(0x03)` 只在 `#[cfg(test)]` 里，不属于运行期多写。

### 7.4 未覆盖

- 未逐字节审计三只 Boss 的**物理/碰撞**子路径（`EnemyBGCollideOrApplySpeed`、`ApplySpeedToPosition` 内部），
  这些属公共敌件（#48/#80 已有规格），本卡只覆盖「每帧调了哪些件、传了什么参数」。
- 未覆盖 Kraid/Ridley 死亡后的 `LDD75` 奖励链（#67 已规格化：雕像升起、导弹上限 +75），本卡只引用。
- MB 的 Zebetite 屏障/玻璃机制属 `zebetite.rs` 与 `#76`，本卡未展开。

## 8. 复跑命令

```bash
# 1) 常量表逐字节（第 §6 的 python 片段）
python3 - <<'PY'
rom=open('rom/Metroid (USA).nes','rb').read()
def off(b,a): return 16+(7*0x4000+(a-0xC000) if b==7 else b*0x4000+(a-0x8000))
print(" ".join("%02X"%x for x in rom[off(4,0x962B):off(4,0x962B)+9]))
PY

# 2) asm 原文（状态机/写点）
sed -n '1,120p' refs/m1disasm/SRC/enemies/kraid.asm
sed -n '1,80p'  refs/m1disasm/SRC/enemies/ridley.asm
sed -n '1089,1730p' refs/m1disasm/SRC/prg3_tourian.asm

# 3) 雕像门控（GetEnemyType）
sed -n '8913,8950p' refs/m1disasm/SRC/prg7_engine.asm

# 4) Boss 出现记录
sed -n '700,710p' refs/m1disasm/SRC/kraid/rooms.asm
sed -n '392,398p' refs/m1disasm/SRC/ridley/rooms.asm
sed -n '120,140p' refs/m1disasm/SRC/tourian/items.asm

# 5) Rust 缺口定位
grep -rn '0x687B\|0x687C\|KRAID_RIDLEY_PRESENT' crates/game-core/src
grep -n '0xEEAE\|it_MotherBrain' -r crates/game-core/src/pickup/
```

## 9. 假设区（C 级，均带推翻方法）

| # | 假设 | 级别 | 推翻方法 |
|---|---|---|---|
| C1 | Kraid/Ridley 的 hp（96/140）与导弹 −4 意味着「Kraid 24 发、Ridley 35 发」 | C | 跑 Mesen，统计从首次命中到 Explode 的导弹数；若与算式不符，说明还有别的减血路径（如每帧多段判定） |
| C2 | Lint/Nail 的 `$10/$20/$30`、`$40/$50` 槽位分工是固定的 | B | 在 `@primeForLaunch` 前后插 trace 看实际 `x`；asm 里三处 `cmp` 顺序已给出线索 |
| C3 | MB 的 `TimeBombCounter` 从 `$80` 到 0 的推进「每帧 1」 | C | trace `$99` 在 status=4 之后的值序列；若跳号则说明部分帧被 NMI 跳过（`NMIStatus` 忙） |
| C4 | `KraidRidleyPresent` 只影响音乐切换，不影响 Boss 行为 | C | grep 全部读点（当前只见 `samus/handler.rs:1681`），若后续出现别的读者需重估 |
