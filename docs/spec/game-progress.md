# 进度链与结局规格（Boss 击破 → 雕像 → 通路 → Tourian → 结局）

> 证据级：**[A]** = ROM 字节/汇编逐行可复现；**[B]** = 代码语境推断（给了推理链）；**[C]** = 假设，仅进 §6。
> 本文只写规格，不改代码；所有地址是 NES CPU 地址，PRG 偏移换算见 §5。
> 卡面提到「`$6Axx` 区状态」——**该地址段不是进度状态**：`$69B4-$6A73` 是 `ItemHistory`（存档用，`constants_ram.asm:1287`，注释明写 "not used"）；真正的 Boss/雕像状态在 `$687B/$687C`。见 §1.1。

## 0. 结论速览

| # | 结论 | 级 |
|---|---|---|
| 1 | Boss 是否已死只看 `KraidStatueStatus=$687B` / `RidleyStatueStatus=$687C` 的**非零**；bit0（Kraid）/bit1（Ridley）=已击败，bit7=雕像已升起 | A |
| 2 | 进 Boss 房时 `GetEnemyType` 用 `InArea&6>>1` 索引这两个字节，非零就**跳过生成** Boss 记录（Boss 不会再出现） | A |
| 3 | Boss 死亡时 `prevStatus` bit7（=记录里的 bit6「tough/迷你 Boss」）走 `EnemyCompleteDeath→LDD75`：写雕像状态=闪烁、置 `MiniBossKillDelayFlag`、导弹上限 +75、播道具音乐 | A |
| 4 | 雕像升起不是自动的：闪烁态下要**玩家子弹命中雕像**（`KraidStatueIsHit`）才开始逐帧上移 15 px，到低半字节 0 时置 bit7 | A |
| 5 | 「通路解锁」= Brinstar 雕像房 lava 上的**桥**：两座雕像 bit7 都置位 ⇒ `StatuesBridgeIsSpawned=1` + 8 个 TileBlast 自建；没有锁门需要开 | A |
| 6 | 进 Tourian 靠 Brinstar 的电梯（`items.asm` `it_Elevator,$03`，y=2,x=3），不是门；雕像状态不影响这部电梯本身 | A |
| 7 | 结局只由「走 `$8F` 结局电梯」触发：`AtEnding+1`、`TitleRoutine=EndGame`；结局等级由 `SamusAge` 与 `AgeTable` 决定（`<1.2h` 最好），与血量/道具无关 | A |
| 8 | Tourian 链：母脑 32 击后进入死亡态；Zebetite 5 座各 8 击、每 64 帧自愈 1 击；逃逸计时归零 ⇒ 母脑 `$07` 时炸弹爆炸 | A |

## 1. Boss 击破

### 1.1 Boss 记录与血量（A）

| 区 | 房间记录 | 槽 | type | 位置 | type 索引 | 血量 |
|---|---|---|---|---|---|---|
| Kraid | `kraid/rooms.asm:705` `.byte $01,$48,$95` | `$10` | `$48` | `$95` | `$48&$3F=8` | `$60`＝96（`prg4_kraid.asm:213`） |
| Ridley | `ridley/rooms.asm:396` `.byte $01,$49,$66` | `$10` | `$49` | `$66` | `$49&$3F=9` | `$8C`＝140（`prg5_ridley.asm:214`） |

- `type` 的 bit6-7 被 `GetEnemyType`（`prg7_engine.asm:8925`）搬到 `Ens.0.specialAttribs` 的 bit6-7（`and #$C0` / `asl`），这就是「tough / 迷你 Boss」标记：`$48=$40|8`、`$49=$40|9`。
- 血量装载：`load_enemy` 取 `EnemyHealthTbl[type&$3F]`，`specialAttribs bit7`（tough 版，记录 bit7）再 `asl` 翻倍。`$48/$49` 只有 bit6，所以**不翻倍**。
- 伤害（`EnemyReactToSamusProjectile@notFreezingProjectile` 起，`prg7_engine.asm:11076-11195`）：非迷你 Boss 吃导弹/旋转攻击=秒杀；迷你 Boss 吃导弹 −4/发、波束 −2/发、其它 −1/发，减到 0 走 `ExplodeEnemy`。⇒ Kraid 需 24 发导弹（96/4）、Ridley 35 发（140/4）〔B：按上述减法推算，未逐帧对拍〕。

### 1.2 生成门（A）

`GetEnemyType`（`prg7_engine.asm:8925-8945`）：

```
lda InArea / and #$06 / lsr / tay
lda KraidStatueStatus-1,y / beq 需要生成
（非零 ⇒ pla/pla 跳过本条记录）
```

- `InArea`：`$10` Brinstar、`$11` Norfair、**`$12` Kraid、`$13` Tourian、`$14` Ridley**（`constants_ram.asm:627`）。
- 跳过分支走 `Lx225`，**不置** `KraidRidleyPresent=$6987`；只有 Boss 未死、正常生成那条路（`Lx227`）才置 present。这个字节由 `samus/handler.rs:1654` 消费（门/电梯转场时清屏判据），别读成「雕像没升也置 present」。
- Kraid：`$12&6=2 → y=1 → $687B`；Ridley：`$14&6=4 → y=2 → $687C`。字节非零（不论 bit0/bit1/bit7）都跳过。⇒ **「已击败」和「雕像已升」共用同一个非零判据**，Boss 房重进时两态都不会重生。

### 1.3 死亡出口（A）

`ExplodeEnemy`（`07:F611`，`:11201`）把 `status` 置 `Explode` 并播死亡动画（`EnemyDeathAnimIndex`：Kraid 索引 8、Ridley 索引 9，`prg4_kraid.asm:196` / `prg5_ridley.asm:196`）。爆炸动画结束 ⇒ `EnemyCompleteDeath`（`07:DCF5`，`:5939`）：

```
lda Ens.0.prevStatus,x / asl / bmi LDD75   ; bit7=迷你 Boss ⇒ 跳过掉落，走击杀结算
```

`prevStatus` 的 bit7 来自 `specialAttribs` 的 `ora Temp0A_EnSpecialAttribs`（`:11130-11143`）——即 §1.1 的 bit6 标记。

### 1.4 击杀结算 LDD75（A，`07:DD75`，`:6044`）

| 动作 | 指令 | 结果 |
|---|---|---|
| 播道具音乐 | `jsr PowerUpMusic` | — |
| 置延时旗 | `lda InArea / and #$0F / sta MiniBossKillDelayFlag($0108)` | Kraid=$2、Ridley=$4 |
| 雕像转闪烁 | `lsr / tay / sta KraidStatueStatus-1,y` | Kraid 写 `$687B=$01`、Ridley 写 `$687C=$02` |
| 导弹上限 +75 | `lda #75 / jsr AddToMaxMissiles` | `MaxMissiles(+$687A)+75`（封顶 255） |

- 延时旗由 `GameEngine`（`07:C945-:2056`）消费：`MiniBossKillDelayFlag|PowerUpDelayFlag` 非零 ⇒ 清零两旗 + `SetTimer($18=240 帧)` + 引擎例程 ← 3。**这是「击杀后 4 秒停顿」的来源**。
- 注意写的是**闪烁态**（1/2），不是 bit7。bit7 由 §1.5 的雕像升起流程追加。

### 1.5 雕像升起（A，`07:D9D4`/`07:DA3D`/`07:DA7C`）

| 阶段 | 触发 | 动作 |
|---|---|---|
| `status=1` | `LoadStatues` 建槽（`07:EC2F`，`:9153`） | 先画「降下态」背景块（`UpdateStatueBGTiles` 表 `$DAD2`），成功 ⇒ `status=2` |
| `status=2` | — | 若 `$687B/$687C` bit7 已置，画「升起态」背景块，成功 ⇒ `status=3` |
| 逐帧更新 | `UpdateStatue`（`07:DA1A`） | `Raise` + `StartRaising` + 按槽选帧 + 闪烁（`FrameCount` 奇偶） |
| 开始上升 | `StartRaising`：`status` 非零且为正（闪烁）、`RaiseState==1`、**`KraidStatueIsHit≠0`**（被子弹打中） | `dec KraidStatueY`（首像素）、播 `sfxTri_StatueRaise`，然后清 isHit |
| 继续上升 | `Raise`：`RaiseState` bit7 未置、`KraidStatueY & $0F ≠ 0` | `RaiseState←2`、`dec Y`；到 `&$0F==0` ⇒ `RaiseState|=0x80`、`sta KraidStatueStatus`（**bit7 置位**）、画升起背景 |
| 每帧位置 | `StartRaising` 尾部 | `Statue.y=KraidStatueY`（升=$50/$30 降=$60/$40）、`Statue.x=$88`(Kraid)/`$68`(Ridley) |

- `Statue` 槽在 `$0360`（`constants_ram.asm:922-933`）：`animDelay`=Kraid 升态、`animResetIndex`=Ridley 升态、`animIndex`/`data07`=命中标记、`data0F`/`$0370`=Y。
- 结论：**看雕像不会自己升**——必须打死 Boss（转闪烁）后去打雕像一枪（或子弹碰到），才有 15 像素的升起动画。

### 1.6 桥（A，`07:DADA`，`:5416`）

```
lda StatuesBridgeIsSpawned($54) / bmi 退出
lda DoorEntryStatus / bne 退出
lda KraidStatueStatus / and RidleyStatueStatus / bpl 退出   ; 任一 bit7=0 ⇒ 不建
sta StatuesBridgeIsSpawned                                  ; 记已生成
8 × TileBlast：routine←3、delay←Y*2(=$10..$02)、type←4、指向 $60xx 名表
```

⇒ 通路解锁**唯一**条件 = 两座雕像 bit7 都置位 + 玩家不在门里（`DoorEntryStatus==0`）。桥本身是 8 个延时 TileBlast，逐块出现。

## 2. 通路与换区

| 项 | 事实 | 级 |
|---|---|---|
| 门锁 | Boss 房→外部的门**没有按 Boss 状态加锁**；门规则见 `docs/spec/doors-elevators.md` §2（`DoorSlots`/`DoorXs`/滚动像素） | A（本文件核对了 `rooms.asm` 的门记录里没有状态分支） |
| 进度门 | 只有 lava 桥（§1.6）——桥不生成时无法走到 Brinstar→Tourian 电梯 | A |
| 进 Tourian | Brinstar 特殊物品表 `01:A3D6` 的 y=2/x=3：`it_Elevator,$03`（低半字节 3=Tourian） | A |
| Tourian 出 | `To(03,01)$8F` 结局梯；`To(04,03)$83` 回 Brinstar（`items.asm`，见 doors-elevators §2.4） | A |

## 3. Tourian 与结局链

### 3.1 区专径（A，`03:9B25`，`prg3_tourian.asm:566`）

`AreaRoutine_Tourian` = `UpdateAllCannons → MotherBrainStatusHandler → UpdateEndTimer → DrawEndTimerEnemy → UpdateAllZebetites → UpdateAllRinkaSpawners`。母脑与 Zebetite 都是**特殊物品表**生成的（`it_MotherBrain=$6`、`it_Zebetite=$7`），不是房间敌记录。

### 3.2 母脑状态机（A，`03:9DD4`，`:1091`）

`MotherBrainStatus`（`$0098`）11 项跳表：`0` 不在房、`1` idle、`2` 被打中闪、`3` 濒死、`4` 消失、`5` 时炸弹对话、`6` 装弹、`7` 爆炸、`8/9` 已死重进、`A` 已死。

| 迁移 | 条件 | 写点 |
|---|---|---|
| 1→2 | `MotherBrainIsHit≠0` | `MotherBrain_Idle_HandleBeingHit`：`inc MotherBrainQtyHits`、`status=2`、`flashDelay=$10` |
| 2→1 | `flashDelay` 减到 0 | `UpdateMotherBrainFlashDelay`：`status=1` |
| 1/2→3 | `QtyHits ≥ $20`（32 击） | 同上：清全部 TileBlast、`status=3`、`flashDelay=$80` |
| 3→4 | 闪烁结束 | `MotherBrain_Killed`：`DeathStringID=0`、清 0–$BF 敌槽（rinka/炮台弹）、`status=4`、`flashDelay=$28`、静音 |
| 4→5/6→7 | 消失完成/装弹/计时归零 | `MotherBrain_Disappear`/`SetTimeBomb`/`TimeBombExploded` |

- 母脑本体槽在 `EnMotherBrain=$04E0`；`MotherBrainHi` 与当前名表不一致时状态被清 0（`:853-869`）。
- 碰到母脑：`MotherBrain_Idle_CollideWithSamus` 判 Samus 在 x∈[$48,$76]、|y−$80|<$20 且同名表 ⇒ `HealthChange=0200`（20 点）+ `isHit=$38`。

### 3.3 Zebetite（A，`03:A28B`，`:2076`）

5 座、每座 8 击（`qtyHits` 递减）、每 64 帧自愈 1 击（`healingDelay`）、重绘失败回滚；表 `ZebetiteAnimFrameTable=$A310`=[`0C`100%、`0D`75%、`0E`50%、`0F`25%、`07`死]。子弹撞 Zebetite/母脑玻璃（`03:A0C6`）：只处理导弹；`$98` ⇒ 玻璃 2×3 瓦清除；`$90-$97` ⇒ 登记命中。**双返回**（`pla/pla/clc/rts`）⇒ 在 Tourian 里子弹一撞瓦就爆。

### 3.4 逃逸计时（A，`03:A1E7`，`:1963`）

`EndTimer`（BCD 两字节）每帧 `Base10Subtract` 递减；每 32 帧播警报；归零 ⇒ `dec EndTimer+1`（置 $FF）、`MotherBrainQtyHits=0`、`MotherBrainStatus=7`、静音、120 帧后清屏。计时期间 Samus 无敌（`SubtractHealth` 开头 `EndTimer+1 / iny / beq` 直接清伤害）。

### 3.5 结局触发（A）

`ElevatorD8BF`（`07:D8BF`，`:4980`）：电梯类型 `$8F` ⇒

```
MainRoutine ← IncrementRoutine；inc AtEnding；RoomPtr←0；BankSwitchPending←1；TitleRoutine←EndGame
```

`LoadEndGFX`（`00:9AD5`，`prg0_title.asm:3743`）：`ClearAll` → 载结局 GFX → `EndingType←(JustInBailey?4:0)` → … → 960 帧后进入 `EndSamusFlash`；每 32 帧 `ColorCntIndex+1`，到 8 时 `ChooseEnding` + `CalculatePassword`。

`ChooseEnding`（`07:CAF5`，`:2439`）+ `AgeTable`（`07:CB18`，`:2471`）：

| `SamusAge+2` | 判定 |
|---|---|
| ≠0 | 结局 1（最差，>37h） |
| =0 且 `SamusAge+1 ≥ $7A` | 结局 1 |
| `≥ $16` | 结局 2（≤6.7h） |
| `≥ $0A` | 结局 3（≤3.0h） |
| `≥ $04` | 结局 4（≤1.2h） |
| `< $04` | 结局 5（最好） |

`EndingType≥4` ⇒ `JustInBailey=1`（无服）。**与血量/道具数无关**〔A：`ChooseEnding` 只读 `SamusAge`〕；`SamusAge` 单位=256 帧（`constants_ram.asm:1232`）。

## 4. 与 Rust 实现的差异表

| 项 | Rust 落点 | 状态 |
|---|---|---|
| 雕像状态机（3 态/升起/命中/桥） | `world/statue.rs:76-229`（`update_all_statues`/`update_statue_raise`/`update_statue_start_raising`/`load_statues`/`statue_register_hit`/`update_statues_bridge`） | 已端口（逐指令对照一致） |
| 雕像状态字节地址 | `world/statue.rs:36` `KRAID_STATUS=0x687B`、`ram_map::RIDLEY_STATUE_STATUS` | 一致 |
| 桥生成门槛 | `world/statue.rs:216` `both = status(0)&status(1)` | 一致 |
| 结局电梯 | `world/elevator.rs:274` `etype==0x8F` ⇒ `AT_ENDING+1`、`TITLE_ROUTINE` | 一致 |
| 结局等级 | `title/mod.rs:1941 choose_ending` + `AGE_TABLE=[0x7A,0x16,0x0A,0x04]` | 一致（含 `SamusAge+2` 优先的 BUG 注释） |
| 结局场景 | `title/mod.rs:1033/1090/1145`（EndingType/SpritePointerIndex/Timer3） | 一致 |
| 母脑状态机 | `enemy/mother_brain.rs`（11 态表、32 击、闪色） | 已端口（单测锚） |
| Zebetite | `enemy/zebetite.rs`（5 座、8 击、自愈、玻璃） | 已端口（Tourian 无像素证据，诚实边界见文件头） |
| 逃逸计时 | `world/end_timer.rs`（`update_end_timer`/`draw_end_timer_enemy`） | 已端口 |
| 母脑/炮台/生成器接线 | `world/update.rs:196-202 uw_area_routine` | 一致（Tourian 专径六件全接） |
| **Boss 击杀结算（LDD75）** | `enemy/framework.rs:520 enemy_complete_death` | **缺失/语义不一致**：直接走掉落分支，没有 `prevStatus bit7 ⇒ 雕像转闪烁 + MiniBossKillDelayFlag + MaxMissiles+75 + 道具音乐` |
| **Boss 房重生门（GetEnemyType bit6）** | `world/room_draw.rs:823 load_enemy` | **缺失**：没有读 `$687B/$687C` 跳过 Boss 记录 |
| 击杀后 240 帧停顿 | `frame/tick.rs:672-673`（延时旗→SetTimer） | 已端口（但缺 §1.4 的旗设置方，实际不触发） |

> 结论：玩家侧「打 Boss→雕像→桥」这条链在 Rust 里**断开两处**：击杀结算不写 `$687B/$687C`，进门也不读它。雕像/桥/结局本体都已移植，缺的是两个连接点。

## 5. 复现命令

```bash
cd <repo>
python3 - <<'PY'
import pathlib
r=pathlib.Path('rom/Metroid (USA).nes').read_bytes(); prg=r[16:16+128*1024]
def rd(b,a,n):
    base=0xC000 if b==7 else 0x8000
    return prg[b*0x4000+(a-base):][:n]
print('Kr health', hex(rd(4,0x962B+8,1)[0]), 'Ri health', hex(rd(5,0x962B+9,1)[0]))
print('Kr deathanim', hex(rd(4,0x960B+8*2,1)[0]), 'Ri', hex(rd(5,0x960B+9*2,1)[0]))
print('DoorSlots', rd(7,0xEC00,4).hex(' '))
print('statueX', rd(7,0xDA39,2).hex(' '))
PY
# 实跑输出（本文件写作时）：
#   Kr health 0x60 Ri health 0x8c
#   Kr deathanim 0x6d Ri 0x11        ← 相对 EnAnimTable 的偏移，不是绝对地址
#   DoorSlots 80 b0 a0 90            ← $80/$B0/$A0/$90 = Doors.{0,3,2,1}
#   statueX 88 68                    ← Kraid 雕像 X=$88、Ridley X=$68
```

```bash
grep -n "LDD75:" -A 14 refs/m1disasm/SRC/prg7_engine.asm   # 击杀结算四件事
grep -n "UpdateAllStatues_Bridge:" -A 12 refs/m1disasm/SRC/prg7_engine.asm
grep -n "ChooseEnding:" -A 24 refs/m1disasm/SRC/prg7_engine.asm
grep -n "ElevatorD8BF:" -A 12 refs/m1disasm/SRC/prg7_engine.asm
```

## 6. 假设区（C，仅线索）

| # | 假设 | 为什么只是 C |
|---|---|---|
| H1 | 击杀后 240 帧停顿是「Boss 死亡演出」的一部分，不是掉帧 | 未对拍 240 帧内画面 |
| H2 | 母脑在 Zebetite 未清完时不可被伤害 | 未在 `MotherBrain_Idle_HandleBeingHit` 找到 Zebetite 计数判据；只看到命中计数 |
| H3 | `EndTimer` 归零后的 120 帧是「屏闪/爆炸展示」 | 只读到 `SetTimer` 未追消费方 |

## 7. 未覆盖

1. 结局动画的完整分支（`EndSamusFlash`→`SamusWave`→`RollCredits`）只核到入口，未逐指令。
2. Tourian 母脑/ Zebetite 无 Mesen 像素证据（`room-sweep.md` §8.1：非 0 区注入结构性不可用）——本文这些段是 asm 逐行 + 既有单测锚，缺动态对拍。
3. `$6Axx`（`ItemHistory`）在通关流程中确无写入点，但我没有全 RAM 写点扫描；若有人发现写点，属新证据。
