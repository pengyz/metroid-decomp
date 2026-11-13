# 萨姆斯动作状态机（Samus action state machine）

> **取证基准**：`refs/m1disasm/SRC/prg7_engine.asm`（ROM 逐字节等价的反汇编）。
> 本文只写**已逐行读完**的部分；未逐行取证的态在 §4 末尾单独列出，不猜。
> 触发本文的问题报告：跳跃过程中按跳跃键可以再次起跳（空中跳）。

## 1. 状态表（`Samus.status = $0300`，10 态 + 一个哨兵值）

分派点：`GoSamusHandler: ; 07:CC1A`（`prg7_engine.asm:2716`）

```text
UpdateSamus: ; 07:CC0D
    jsr GoSamusHandler
GoSamusHandler: ; 07:CC1A
    lda Samus.status
    bmi SamusStand          ; bit7=1（$80-$FF）⇒ 直接按站立处理
    jsr JumpEngine          ; 否则查表
SamusHandlerPtrTable: ; 07:CC28  —— 10 项
```

| status | 名称 | 入口 | 职责（一句话） |
|---|---|---|---|
| 0 | `sa_Stand` | `SamusStand $CC36` | 站立：转向/变球/上指/开火/**起跳** |
| 1 | `sa_Run` | `SamusRun $CCC2` | 跑动：地面径 + 空中径（同一例程两分支） |
| 2 | `sa_Jump` | `SamusJump $D002` | 跳跃：切断/空控/落地判/**岩浆重跳** |
| 3 | `sa_Roll` | `SamusRoll $D0E1` | 变球滚动 |
| 4 | `sa_PntUp` | `SamusPntUp $D198` | 向上瞄准（站立） |
| 5 | `sa_Door` | `SamusDoor $D3A8` | 门内滚屏（锁定输入） |
| 6 | `sa_PntJump` | **`SamusJump $D002`** | 上指跳——**与 status=2 同一例程** |
| 7 | `sa_Dead` | `SamusDead $D41A` | 死亡初段 |
| 8 | `sa_Dead2` | `SamusDead2 $D41F` | 死亡动画段（`animDelay==0` 触发 GameOver 链） |
| 9 | `sa_Elevator` | `SamusElevator $D423` | 电梯 |
| 255 | `sa_Begin` | — | `bmi` 哨兵：未进场/不可受伤（恒按站立处理） |

- 状态枚举来源：`refs/m1disasm/SRC/constants.asm:88-176`（`sa_Stand`…`sa_Elevator`）。
- 表 2 与表 6 指向**同一个例程**：上指跳不是独立实现，而是 `SamusJump` 内部读
  `Samus.animResetIndex == ObjAnim_SamusJumpPntUp` 走不同动画/投射方向。
- `bmi SamusStand` 的后果：任何 bit7 置位的 status（含 `sa_Begin=$FF`、"死亡中不可控"等）
  都按站立跑一遍 `SamusStand` —— 这是原版写法，不是我们的简化。

## 2. 帧骨架（每帧一次，NMI 之外）

```text
MainLoop (07:C0BC)
  jsr CheckBankSwitch ($C4DE)     ; 消费 BankSwitchPending($24)
  jsr UpdateTimer     ($C266)     ; Timer3 每 10 帧减 1
  jsr GoMainRoutine   ($C114)     ; GameMode 门 → MainRoutine 表
      └ GameEngine ($C92B) → jsr UpdateSamus → GoSamusHandler → 本状态机
  inc FrameCount      ($C0C5)     ; 真址 C0C5（注释标的 C0CB 偏 2 字节）
  lda #0 / sta NMIStatus          ; 清握手位
  @loop_waitNMIEnd: 自旋等 NMIStatus≠0
  jsr RandomNumbers   ($C000)
  jmp MainLoop
```

要点：

- **状态机在 `MainRoutine == 3 (GameEngine)` 之下才跑**；其它 MainRoutine（标题/初始化/暂停/
  死亡延迟）不调 `UpdateSamus`，此时 `Samus.status` 冻结。
- 一帧只跑**一个**状态的分派体（`JumpEngine` 尾跳，`pla/pla` 丢掉返回地址），
  但**同帧内可以连跳两级**：`SamusStand` 改 `status` 后走 LCC77 派发表再调状态件。
- 没有「输入缓冲」：跳跃只看 `Joy1Change`（本帧刚按下的沿），不看 `Joy1Status`（持续按住）。

## 3. 跳跃触发门（全部合法入口，共 3 条）

`SetSamusJump: ; 07:CFC3`（`prg7_engine.asm:3431`）是**唯一的起跳冲量件**：

```text
SetSamusJump:
    sty Samus.animResetIndex        ; 跳/上指跳动画
    dey / sty Samus.animIndex       ; 过渡动画
    lda #$04 / sta Samus.animDelay
    SamusJumpDsplcmnt($030F) ← 0    ; 跳跃位移累计清零
    Samus.speedY($0308)      ← $FC  ; 向上 -4（定点）
    ldx Samus.status / dex / bne Lx017
        ; 仅「站立起跳」（进来时 status 已被改成 2）才查 Screw Attack
    SamusAccelY ← $18               ; 重力加速度；有 gr_HIGHJUMP ⇒ $12（更小=更高）
```

三条合法入口：

| # | 位置 | 条件 | 说明 |
|---|---|---|---|
| 1 | `SamusStand` LCC65（`:2761`） | `bit Joy1Change / bpl` ⇒ A 按下沿 | 站立起跳：先 `status←2`，同帧经 LCC77 派发表调 `SetSamusJump` |
| 2 | `SamusRun` 地面径 LCD40（`:2937`） | `Joy1Change bit7` | 跑动起跳：`status` **保持 1**（跑态自带跳/落动画机），`HorzSpeedMax←$12` |
| 3 | `SamusJump` Lx020（`:3496`） | **`SamusInLava($64) ≠ 0`** 且 `Joy1Change bit7` | **岩浆里可以在空中再跳**——这是原版的合法"空中跳" |

**非岩浆的空中态没有任何 `SetSamusJump` 调用点**。空中态（status=2/6）与跑动态的空中分支
都只处理：切断（松手/撞顶）、横向空控、开火、上指切换、落地判定。因此「跳跃中再按跳跃」
在 asm 里**不可能再次起跳**。

### 3.1 为什么「撞顶/松手」不会让人被误判落地

`StopVertMovement: ; 07:D147`（`:3674`）全形：

```text
StopVertMovement:
    ldy #$00
    sty Samus.speedY            ; $0308 ← 0
    sty SamusSpeedSubPixelY     ; $0312 ← 0（速 frac）
    rts
```

**只清速度面两字节，不动 `SamusAccelY($030B)`**。而落地判据是
`SamusJump` 的 `lda SamusAccelY / bne Lx023`（AccelY==0 才算落地）⇒ 撞顶/松手都不会
把 AccelY 归零 ⇒ `status` 不会跳回 Stand ⇒ 空中不存在"被判落地后再起跳"的路径。

`AccelY` 归零的唯一途径是重力积分落地（`SamusJump` 落地判命中后转 Stand/PntUp）。

## 4. 逐态要点

### 4.1 `SamusStand $CC36`（status 0）

1. `Joy1Status & ~(SELECT|START)` 非零 ⇒ 播站立呼吸/落脚动画。
2. `Joy1Status & (DOWN|LEFT|RIGHT)` 非零 或 `Joy1Change & UP` ⇒ `BitScan` →
   `ActionTable $CC87 = [sa_Run, sa_Run, sa_Roll, sa_PntUp]` ⇒ 改 status。
3. **LCC5B**：`(Joy1Change|Joy1Retrig) bit6`（B）⇒ `FireSamusProjectile`。
4. **LCC65**：`bit Joy1Change / bpl` ⇒ A 按下沿 ⇒ `status ← sa_Jump`。
5. LCC6E：`lda #$04 / jsr SetSamusData($CD6D)`（动画帧延迟）。
6. 收尾 `cmp #sa_Door / bcs RTS` ⇒ `JumpEngine` + 派发表
   `[ExitSub, SetSamusRun, **SetSamusJump**, SetSamusRoll, SetSamusPntUp]`
   ⇒ 第 4 步刚改成 2，本帧立刻拿到起跳冲量。

### 4.2 `SamusRun $CCC2`（status 1）——地面/空中双径

判别式：`ldx SamusDir / lda SamusAccelY / beq samL07`

- **空中径 samL01-06**（`AccelY ≠ 0`）：下落 ⇒ 位移<`$18` 用 Jump 动画；上升 ⇒
  ≥`$18` 切 Salto、≥`$20` 且按住 UP ⇒ JumpPntUp、且**松开 A ⇒ `StopVertMovement`（切断）**；
  然后三件（`SamusRun_SetAnim` 空径 / `SamusJump_CheckFire` / `LCF2E` 固实反应）；
  **唯一的起跳门**：`SamusInLava ≠ 0` 且 `Joy1Change bit7` ⇒ LCD40；`animDelay ← 2`。
- **地面径 samL07-12**（`AccelY == 0`）：`SetSamusRunAccel` → `SamusRun_SetAnim` →
  走音节拍 → `LCF2E` → **`Joy1Change bit7` ⇒ LCD40 起跳**（`HorzSpeedMax ← $12`）→
  B 开火 → 无方向键 ⇒ `SetSamusStand` → 转身（`SetSamusNextAnim`）；`animDelay ← 3`。

> 跑动起跳后 `status` **仍是 1**，靠 `AccelY` 切到空中径——所以"跑跳"看起来像"还在跑"，
> 但空中径里没有起跳门（除岩浆）。

### 4.3 `SamusJump $D002`（status 2/6）

顺序固定，每帧一遍：

1. **切断**：`SamusJumpDsplcmnt` 与 `speedY` 同号（上升中）且位移 ≥`$20` 且 A 已松开
   ⇒ `StopVertMovement`。
2. `SamusJump_CheckHorzMovement $D055`：空中横控，X∈{1,0,−1}；转向 ⇒ 换动画 +
   `animDelay ← 8` + `SamusDir ← Y`。
3. `LCF2E`：固实实体接触反应（贴墙/贴顶时清水平动量）。
4. 按住 UP ⇒ `animResetIndex ← ObjAnim_SamusJumpPntUp` + `status ← sa_PntJump(6)`。
5. `SamusJump_CheckFire $D09C`：B 开火（上指跳则向上射）。
6. **岩浆重跳**：`SamusInLava ≠ 0` 且 `Joy1Change bit7` ⇒ `SetSamusJump` +
   `SetSamusData_3FrameAnimDelay`（`animDelay ← 3`）。
7. **落地判 Lx021**：`SamusAccelY == 0` ⇒ 落地：status==6 ⇒ `SetSamusPntUp`，
   否则 `SetSamusStand`。
8. Lx023：`lda #$03 / jmp SetSamusData`。

> 第 6 步与第 7 步**互斥且顺序敏感**：岩浆重跳先跑，所以岩浆中按 A 不会先被判落地。

### 4.4 未逐行取证的态（本文不猜）

`SamusRoll $D0E1`(3) / `SamusPntUp $D198`(4) / `SamusDoor $D3A8`(5) /
`SamusDead $D41A`(7) / `SamusDead2 $D41F`(8) / `SamusElevator $D423`(9)——
本文只确认它们的入口地址与分派序号。要改这些态的行为，先单独取证。

## 5. asm 与 Rust 实现对照

Rust 侧单点：`crates/game-core/src/samus/handler.rs`
（`handler_addr()` / `go_samus_handler` / 各 `samus_*` 函数）。

| asm | Rust | 一致性 |
|---|---|---|
| `status & 0x80 ⇒ Stand` | `go_samus_handler`：`status & 0x80 != 0 ⇒ Stand` | ✓ |
| 派发表 2 与 6 同件 | `SamusHandler::Jump \| JumpPntUp => samus_jump` | ✓ |
| 站立 LCC65 起跳 | `samus_stand`：`JOY1_CHANGE & A ⇒ SAMUS_STATUS = 2` | ✓ |
| 站立 LCC77 派发 | `if st < 5 { 2 => set_samus_jump(mem) }` | ✓ |
| 跑动地面 LCD40 | `samus_run` 地面径（`accel_y == 0` 分支） | ✓ |
| 跑动空中岩浆门 | `samus_run` 空中径调 `lava_jump_allowed` | ✓ |
| 跳跃态岩浆门 | `samus_jump_tail` 调 `lava_jump_allowed` | **本次修复**（原读 `$0609`） |
| 跳跃态空控只调一次 | `samus_jump_tail` 首行调用；`samus_jump` 内重复调用已删 | **本次修复** |
| `StopVertMovement` 只清速度面 | `physics::stop_vert_movement`：只写 `SAMUS_SPEED_Y_I`/`_F` | ✓ |

`set_samus_jump` 的四个调用点（`crates/game-core/src/samus/handler.rs`）：
`:259`（samus_run 地面径）、`:303`（samus_run 空中径岩浆门）、`:984`（samus_stand 派发表）、
`:1099`（samus_jump_tail 岩浆门）——与 asm 的三入口 + 同帧派发一一对应，**无第五条路径**。

### 5.1 本次修复：`0x0609` 错址（潜在无限跳）

旧形（`samus_jump_tail`）：

```rust
if mem.read_ram(0x0609) != 0 && mem.read_ram(JOY1_CHANGE) & A != 0 { set_samus_jump(mem); }
```

`$0609` = `MusicTriPeriodHigh`（`constants_ram.asm:993`，"Loaded into TRI_HI when playing
music"）——**音乐播放中恒非 0**；正源是 `SamusInLava = $0064`（`constants_ram.asm:574`
`SamusInLava db`）。后果：只要有音乐，空中再按 A 就重跑 `SetSamusJump`
（`speedY ← $FC` + 位移清零）⇒ **无限跳**，与用户报的症状完全一致。

已修：抽纯谓词 `lava_jump_allowed(mem)`（`SAMUS_IN_LAVA != 0 && JOY1_CHANGE & 0x80 != 0`）
+ `samus_jump_tail` 改调 + TDD
`lava_jump_gate_reads_samus_in_lava_not_music_period`（三断言：音乐在放+空中按 A ⇒ 拒绝；
岩浆中按 A ⇒ 允许；岩浆中未按 ⇒ 拒绝）。

## 6. 实测证据（双侧语义对拍，`tools/sem.py`）

| 场景 | 用例 | 结果 |
|---|---|---|
| 跑动起跳 + 空中再按 A 两次 | `cases/probe-airjump.txt`（素：hold right 820-1000 / press 1000,1012,1060 A） | `MATCHED 995/1184`，首个分歧 `f1030 fc oracle=F3 rust=F5` = 已知 FC+2 相位族 |
| 站立起跳 + 空中再按 A 两次 | 同上（无方向键） | **`MATCHED 1184/1184`**（全同） |
| 连按 12 次 A 压力探针 | `cases/probe-airjump2.txt`（hold right 900-1400 + 12×A + B） | `set_samus_jump` **仅 4 次调用，全部 `status=01 / AccelY=00 / lava=00`**（fc=BB/E7/17/75）⇒ 每次起跳都在地面 |

`$0609` 双侧实测（同一用例）：emu **703/1200 帧非零**（f497 起值 `4`，Brinstar 低音在响）；
rust **1184 帧全为 0**。

> 结论：`$0609` 错址在当前 Rust 里是**潜伏 bug**——因为 Rust 的音乐 TRI 面本身还没发声
> （`$0609` 恒 0），错址不会误命中。一旦声音子系统把 TRI 周期写进来，这个错址会立刻
> 表现为无限跳。这也是"为什么用户报告了但我复现不出来"的最可能解释（旧构建 / 声音面差异）。

## 7. 未闭合

1. **用户报告的空中跳未能在当前 HEAD 复现**（两个跳跃场景 + 压力探针全同 emu）。
   需要精确 repro：哪个构建、哪个房间、是否变球/岩浆/上指、按 A 的节奏。
2. 变球态（status=3）在空中解球后是否允许起跳：`SamusRoll` 未逐行取证，暂不结论。
3. `SamusAccelY` 归零时机与重力积分的精确等价性：本次只确认"切断/撞顶不归零"，
   未验证每一帧的积分式与 asm 逐字节等价。

## 8. asm 锚点索引

| 锚 | 地址 | 行号（`prg7_engine.asm`） |
|---|---|---|
| `UpdateSamus` | `07:CC0D` | 2705 |
| `GoSamusHandler` | `07:CC1A` | 2716 |
| `SamusHandlerPtrTable` | `07:CC28` | 2730 区 |
| `SamusStand` | `07:CC36` | 2724 |
| `ActionTable` | `07:CC87` | 2804 区 |
| `SamusRun` | `07:CCC2` | 2855 |
| `SamusJump` | `07:D002` | 3467 |
| `SamusJump_CheckHorzMovement` | `07:D055` | 3520 |
| `SamusJump_CheckFire` | `07:D09C` | 3565 |
| `SetSamusJump` | `07:CFC3` | 3431 |
| `StopVertMovement` | `07:D147` | 3674 |
| `SetSamusData` | `07:CD6D` | 2990 区 |
| `SetSamusStand` | `07:CF55` | 3405 区 |
| `SetSamusPntUp` | `07:CF77` | 3420 区 |
