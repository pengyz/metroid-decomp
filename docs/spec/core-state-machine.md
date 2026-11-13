# 核心状态机骨架规格（Metroid NES / 保真期）

> 2026-11-13 · 用户令（m13355）「先钉状态机骨架，纯取证不动代码」·
> 取证口径：`refs/m1disasm` 逐例程全形读完（bank:addr 锚），**不读 Rust 代码下结论**，
> Rust 现状只作「已实现/未实现」的对照栏（§9）·
> Evidence A（每条转移都有 bank:addr + asm 行号）Confidence 92
> （置信度不足处：§10 逐条标注 B/C 级与缺口）

本文是**游戏骨架的单一真相源**：所有「状态推进」类修复都必须先对照本文，
再看代码。它的目的不是描述现状，而是钉住**原版在任一时刻允许处于哪些状态、
以及只允许沿哪些边转移**——这样 Rust 侧任何「拟合出来的顺移」都能被立刻指认。

---

## 1. 三轴 + 六辅助：状态的全部自由度

原版没有「状态」这个概念，只有**三个互相独立的索引变量**和一组辅助旗。
一个游戏时刻 = `(GameMode, MainRoutine|TitleRoutine, 辅助)`。

| 变量 | 地址 | 语义（asm 注释原文） | 定义位置 |
|---|---|---|---|
| `GameMode` | `$1D` | `0 = Game is playing, 1 = At title/password screen` | `constants_ram.asm:482` |
| `MainRoutine` | `$1E` | `5 = Game paused, 3 = Game engine running`（表索引 0-9） | `constants_ram.asm:483` |
| `TitleRoutine` | `$1F` | `Stores title routine number currently running`（表索引 `$00-$1E`） | `constants_ram.asm:484` |
| `GamePaused` | `$31` | `#$00=Game running, #$01=Game paused` | `constants_ram.asm:504` |
| `NextRoutine` | `$20` | `Stores next routine to jump to after WaitTimer expires` | `constants_ram.asm:480` |
| `Timer3` | `$2C` | 每 **10 帧**递减一次（秒级计时） | `constants_ram.asm:495` |
| `BankSwitchPending` | `$24` | `Switch memory page. Page # = BankSwitchPending - 1` | `constants_ram.asm:491` |
| `CurrentBank` | `$23` | `0 thru 7. current memory page in lower memory block` | `constants_ram.asm:490` |
| `NMIStatus` | `$1A` | `0=NMI in progress. anything else, NMI not in progress` | `constants_ram.asm:475` |
| `FrameCount` | `$2D` | `Increments every frame(overflows every 256 frames)` | `constants_ram.asm:496` |

**关键不变量**：

- `GameMode ≠ 0` 时 **`MainRoutine` 完全不参与分派**（`GoMainRoutine` 直接 `jmp MainTitleRoutine`）。
  即：title/password 屏期间 `$1E` 是「冻结的残值」，不是状态。
- `GameMode = 0` 时 **`TitleRoutine` 不参与分派**（`MainTitleRoutine` 只在 `$1D≠0` 时被调）。
- 两条轴**从不并发推进**——这是 M1 时代最重要的结构事实，也是 §9 里 Rust 侧最容易走形的地方。

---

## 2. 帧骨架：MainLoop 一次迭代 = 一个（或两个）NMI

```
MainLoop: ; 07:C0BC                      （prg7_engine.asm:199-223）
    jsr CheckBankSwitch   ; 07:C4DE      ← $24 的唯一消费者
    jsr UpdateTimer       ; 07:C266      ← TimerDelay/Timer1/Timer2 每帧、Timer3 每 10 帧
    jsr GoMainRoutine     ; 07:C114      ← 轴分派
    inc FrameCount        ; 07:C0C5      ← **rom 字节 e6 2d**（asm 注释标的 :C0CB 偏 2 字节，已实证）
    lda #$00 / sta NMIStatus             ← 开门：允许下一个 NMI 干正事
@loop_waitNMIEnd: ; 07:C0CB
    lda NMIStatus / beq @loop_waitNMIEnd ← 自旋等 NMI 把 $1A 置非 0
    jsr RandomNumbers     ; 07:C000
    jmp MainLoop
```

### 2.1 `inc FrameCount` 何时不发生

`inc` 在 `GoMainRoutine` **返回之后**。所以只要例程体内**不返回**（自己再等一次 NMI），
本帧就不 `inc`：

1. `WaitNMIPass ; 07:C42C` = `ClearNMIStat` + 自旋等 `$1A≠0`（`prg7_engine.asm:1108`）。
   例程体内部调用它 ⇒ 该次 MainLoop 迭代跨 **2 个 NMI**，`inc FC` 只发生 1 次。
2. `WriteAndWait ; 07:C43D` = 写 PPUMASK 影子后落 `WaitNMIPass_`（同上）——
   `ScreenOff / ScreenOn / ScreenNmiOff` 全都经由它。
3. 长例程忙到 NMI 已发生但**尚未走到自旋点**——不会重复 `inc`（`inc` 只在本迭代一次），
   但会让**下一个** NMI 落在 `$1A=0` 窗口内被吞（`NMI: lda NMIStatus / bne @endIf_A`，
   `prg7_engine.asm:251`）⇒ 表现为掉帧。

> 这三条是「FC 冻结/掉帧」的**全部机制来源**。任何「FC 该不该 +1」的判断都必须落在这三条上，
> 不允许引入第四条（Rust 侧的 `title_spin`/`fc_inced_this_tick` 就是在试图替代它们，见 §9.3）。

### 2.2 采样相位（本项目 L4 口径）

`capture 帧 N` = **tick(N-1) 的两个相位**：
- `png/ppu.hex` = `on_spin_exit` 缝（NMI 已跑完、主循环工作开始前）；
- `ram.hex` = 收口活体（迭代完成，`$1A=00` 为判别位）。

取证记录见 `docs/findings/walk-fc-capture-seam.md` §2c。

---

## 3. GameMode 轴：`0` 与 `1` 的换页游戏

`GameMode` **只有两个取值**，切换全部由 `BankInitTable` 的初始化例程完成：

```
BankInitTable: ; 07:C51F                 （prg7_engine.asm:1363-1375，9 项 .word）
  [0] InitBank0 $C531   → GameMode←1  （标题页）
  [1] InitBank1 $C552   → GameMode←0  （Brinstar）
  [2] InitBank2 $C583   → GameMode←0  （Norfair）
  [3] InitBank3 $C590   → GameMode←0  （Tourian）
  [4] InitBank4 $C5B6   → GameMode←0  （Kraid）
  [5] InitBank5 $C5C3   → GameMode←0  （Ridley）
  [6..8] ExitSub $C45C  → rts（无效页）
```

### 3.1 换页的唯一通道

```
CheckBankSwitch ; 07:C4DE   （prg7_engine.asm:1296-1310）
    ldy BankSwitchPending / beq rts        ← $24=0 即无事
    jsr CheckBankSwitch_OK                 ← 清 $24、CurrentBank←Y-1、MMC1 串行写
    jmp GoBankInit                         ← A=bank# ⇒ BankInitTable[y*2]
```

- **`$24` 的写入者**（全谱）：`InitializeStats $932B`（→2，即 Brinstar）、
  `InitializeGame $92D4`（→`BankTable[InArea&$0F]`）、`PrepareGameOver $C9A6`（→1 标题）、
  `GoPassword $C9C4`（→1 标题）、`StartContinueScreen` 族、`InitEndGFX` 等。
- **`$24` 的唯一消费者**：`CheckBankSwitch`，且它只在 `MainLoop` 头部被调 ⇒
  **换页永远发生在帧边界，永远在分派之前**。这条顺序是 2026-11-11 修 `$1E=1` 存活 15 帧
  那个 bug 的根（Rust 侧曾在同迭代吃掉 `$24`）。

### 3.2 `InitBank0`（进标题）与 `InitBank1..5`（进区域）的形状差

| | `InitBank0 $C531` | `InitBank1..5 $C552..$C5C3` |
|---|---|---|
| `GamePaused` | ←0 | ←0 |
| `GameMode` | **←1** | **←0** |
| 屏幕 | `ScreenNmiOff` | `ScreenNmiOff` |
| 载入 | `CopyMap`（ROM→WRAM `$7000-$73FF`）+ `ClearNameTables` + 星点精灵 `$6E00` + `InitTitleGFX` | `InitBrinstarGFX`/`InitNorfairGFX`/… |
| 出口 | `jmp NMIOn` | `jmp NMIOn` |

**注意**：`InitBankN` **都不点亮屏幕**——点亮只发生在 `MoreInit` 尾的 `jmp ScreenOn`（§5.2）。

---

## 4. MainRoutine 轴：10 个游戏侧状态

`MainRoutinePtrTable: ; 07:C141`（`prg7_engine.asm:341-355`，10 项）

`GoMainRoutine ; 07:C114`（:301-340）的**前置逻辑**（分派之前）：

```
lda GameMode / beq @endIf_A
    jmp MainTitleRoutine                    ← GameMode≠0：整条 MainRoutine 表不参与
@endIf_A:
lda Joy1Change / and #BUTTON_START / beq @endIf_B
    lda MainRoutine
    cmp #_id_GameEngine(3) / beq → MainRoutine←5(PauseMode)   ← 3→5
    cmp #_id_PauseMode(5) / bne @endIf_B → MainRoutine←3      ← 5→3
    sta MainRoutine ; GamePaused ^= 1 ; jsr PauseMusic
@endIf_B:
lda MainRoutine / jsr JumpEngine            ← 表分派
```

> **暂停开关只在 `3 ↔ 5` 之间生效**，其它状态下按 START 不改变 `MainRoutine`。

| id | 例程 | bank:addr | 职责（asm 事实） | 出口（写 `$1E` 的边） |
|---|---|---|---|---|
| 0 | `AreaInit` | `07:C801` | 清 `ScrollX/Y`、PPUCTRL 名表 0、`inc $1E`→1、`ABStatus←Joy1&A|B`、清精灵、`lda #$10 / jsr IsEngineRunning` | **物理 fallthrough 进 MoreInit**（`$1E=1`，同迭代） |
| 1 | `MoreInit` | `07:C81D` | `PaletteDataPending←Palette00+1`、`ScreenOff`、清名表、清精灵、清敌、**房间装载环** `SetupRoom` 直到 `$5A+1==0`、RoomRAM 4 页→`$2000`、`inc $1E`→2 | `inc`→2 + `jmp ScreenOn` |
| 2 | `SamusInit` | `07:C8D1` | `$1E←8`、`Timer3←$2C`(44×10帧=440)、`IntroMusic`、`Samus.status←$14`、敌槽清、`InArea` 分支、`Samus.y←AreaSamusY`/`x←$80`、`Health←$030` | 直写 `$1E←8` |
| 3 | `GameEngine` | `07:C92B` | `ScrollDoor`×2、NARPASSWORD 作弊、`UpdateWorld`（Samus/敌/房间 tile）、迷你 Boss/道具延时 `SetTimer($18,x=3)`、死亡判定 → `SetTimer($04,x=4)` 或 `inc $1E`→4 | `SetTimer`→9、或 `inc`→4 |
| 4 | `PrepareGameOver` | `07:C9A6` | `TitleRoutine←GameOver`、`$24←1`、`jmp ScreenOff` | **换轴**：下次 `CheckBankSwitch` 进 `InitBank0` ⇒ `GameMode←1` |
| 5 | `PauseMode` | `07:C9B1` | 要求 `Joy2Status & (A|UP)` 全按、且 `EndTimer+1==0`（逃脱段禁止）；`GamePaused←0`、`inc $1E`→6 | `inc`→6 |
| 6 | `GoPassword` | `07:C9C4` | `TitleRoutine←DisplayPassword`、`$24←1`、`SFXNoiseInitFlags |= SilenceMusic`、`jmp ScreenOff` | **换轴**：进 `InitBank0` ⇒ `GameMode←1` |
| 7 | `IncrementRoutine` | `07:C155` | `inc MainRoutine / rts`（唯一职责） | `inc`→id+1 |
| 8 | `SamusIntro` | `07:C9D7` | `EraseAllSprites`；`Timer3≠0` ⇒ 淡入表 `$1E,$14,$0B,$04,$FF` 触发 `inc Samus.status + PaletteDataPending`、`FC&1` 决定画不画；`Timer3==0` ⇒ `Samus.status←sa_Begin`、`StartMusic`、`SelectSamusPalette`、`$1E←3` | 直写 `$1E←3` |
| 9 | `WaitTimer` | `07:C494` | `Timer3≠0` ⇒ rts（**空转，靠 `SetTimer` 进入**）；`==0` ⇒ `NextRoutine→$1E`（4/6 免起乐，其余先 `StartMusic`） | 直写 `$1E←NextRoutine` |

### 4.1 转移边清单（游戏侧，一条边一行）

| # | 从 | 边（asm 依据） | 到 |
|---|---|---|---|
| E1 | `reset` | `InitBankTable[0]` = `InitBank0 $C531`（`GameMode←1`） | 标题轴 |
| E2 | 标题轴 `$16` START | `jmp InitializeStats ; 00:932B`（尾 `$24←2`） | E4 |
| E3 | 标题轴 `$18` START 且密码有效 | `jmp InitializeGame ; 00:92D4`（尾 `$24←BankTable[InArea&$0F]`） | E4 |
| E4 | 任意 | `CheckBankSwitch $C4DE` → `GoBankInit $C510` → `InitBank1..5`（`GameMode←0`） | `$1E` 保持原值（通常 0） |
| E5 | `$1E=0` AreaInit | 无 `rts`，`jsr IsEngineRunning` 在 `$1E=1` 时直接返回后**物理落到** `$C81D` | `$1E=1` MoreInit（**同迭代**） |
| E6 | `$1E=1` MoreInit | 尾 `inc MainRoutine` | `$1E=2` SamusInit |
| E7 | `$1E=2` SamusInit | `lda #_id_SamusIntro / sta MainRoutine` | `$1E=8` SamusIntro |
| E8 | `$1E=8` SamusIntro | `Timer3` 递减到 0 ⇒ `MainRoutine←3` | `$1E=3` GameEngine |
| E9 | `$1E=3` GameEngine | `SetTimer($18, x=3)` ⇒ `MainRoutine←9` | `$1E=9` WaitTimer |
| E10 | `$1E=9` WaitTimer | `Timer3==0` ⇒ `SetMainRoutine`（`NextRoutine($20)→$1E`） | `$1E=NextRoutine` |
| E11 | `$1E=3` GameEngine | `Joy1Change & START`（3→5） | `$1E=5` PauseMode |
| E12 | `$1E=5` PauseMode | `Joy1Change & START`（5→3） | `$1E=3` GameEngine |
| E13 | `$1E=5` PauseMode | `Joy2Status & (A|UP)` 全按 **且** `EndTimer+1==0` ⇒ `inc MainRoutine` | `$1E=6` GoPassword |
| E14 | `$1E=3` GameEngine | 死亡动画结束：`MotherBrainStatus==$0A` ⇒ `inc MainRoutine`；否则 `SetTimer($04, x=4)` | `$1E=4` 或 `$1E=9` |
| E15 | `$1E=4` PrepareGameOver | `TitleRoutine←$1C` + `$24←1` + `ScreenOff` ⇒ E4 进 `InitBank0` | 标题轴 `$1C` GameOver |
| E16 | `$1E=6` GoPassword | `TitleRoutine←$19` + `$24←1` + `ScreenOff` ⇒ E4 进 `InitBank0` | 标题轴 `$19` DisplayPassword |
| E17 | `$1E=k` | `$1E=7` IncrementRoutine：`inc MainRoutine` | `$1E=k+1`（真实流程未走，见 §10.6） |

**单向前进为主**：`$1E` 的增长几乎全是 `inc`（0→1→2 或 1→2、3→4、5→6），
回退只发生在 `SetMainRoutine`（`WaitTimer` 尽时写 `NextRoutine`）与暂停开关（3↔5）。
**没有「跳过」状态**的机制——任何「Rust 一次迭代走了两步」都必须能在 §3.1/§5 找到 asm 依据。

### 4.2 `IsEngineRunning ; 07:CA18`（:2247-2254）

```
ldy MainRoutine
cpy #_id_IncrementRoutine(7) / beq rts     ← 过渡态：允许（不换页）
cpy #_id_GameEngine(3)      / beq SwitchBank
rts                                        ← 其余一律直接 rts
```

语义：**只有 `$1E==3`（游戏引擎正在跑）时才真的换页**；`AreaInit` 里 `lda #$10` 传的
「Brinstar 页号」在 `$1E=1` 时被丢掉（`A` 不被使用）——这就是
`AreaInit` 结构上不可能触发换页的原因（2026-11-10 定谳，见 `docs/findings/`）。

---

## 5. TitleRoutine 轴：31 个标题侧状态

`MainTitleRoutine ; 00:8000`（`prg0_title.asm:27-56`）：

```
lda TitleRoutine
cmp #_id_StartContinueScreen15 / bcs L8027      ← $1F ≥ $15：跳过 intro 快捷门
lda Joy1Change / and #BUTTON_START / beq L8022
    sty SpareMemD1/BB/B7/B8                     ← 清未用变量
    lda PPUCTRL_ZP / and #$FC / sta PPUCTRL_ZP  ← 名表 0
    lda #_id_StartContinueScreen1B / sta TitleRoutine
    bne L8027
L8022: jsr RemoveIntroSprites
L8027: jsr JumpEngine + TitleRoutinePtrTable: ; 00:802A
```

> **两族**：`$00-$14` = intro 演出族（带「START 跳服务区」快捷门）；
> `$15-$1E` = 服务族（START 不再跳转，START 的语义由各例程自己解释）。

### 5.1 表（`prg0_title.asm:56-86`，31 项）

| id | 例程 | bank:addr | 职责 |
|---|---|---|---|
| `$00` | `InitializeAfterReset` | `00:8071` | 复位后第一条；2 步入族（Step 2 换名表） |
| `$01` | `DrawIntroBackground` | `00:80D0` | 画地面 |
| `$02` | `FadeInDelay` | `00:80F9` | METROID 淡入延时 |
| `$03` | `METROIDFadeIn` | `00:812C` | METROID 淡入 |
| `$04` | `LoadFlashTimer` | `00:8142` | 闪光计时 |
| `$05` | `FlashEffect` | `00:8109` | METROID 闪烁 |
| `$06` | `METROIDSparkle` | `00:814D` | 上下火花 |
| `$07` | `METROIDFadeOut` | `00:8163` | 淡出 |
| `$08` | `Crosshairs` | `00:8182` | 准星特效 |
| `$09` | `MoreCrosshairs` | `00:81D1` | 准星续 |
| `$0A/$0B` | `IncTitleRoutine0A/0B` | `00:806E` | `inc TitleRoutine`（空步） |
| `$0C` | `ChangeIntroNameTable` | `00:822E` | 名表 0→1 |
| `$0D` | `MessageFadeIn` | `00:8243` | 剧情文字淡入 |
| `$0E` | `MessageFadeOut` | `00:8263` | 淡出 |
| `$0F` | `DelayIntroReplay` | `00:8283` | 重播延时 |
| `$10` | `ClearSpareMem` | `00:8068` | 清备用内存 |
| `$11` | `PrepIntroRestart` | `00:82A3` | 准备重播 |
| `$12` | `TitleScreenOff` | `00:82ED` | `ScreenOff` + `inc`（**注释自称 unreachable**） |
| `$13/$14` | `TitleRoutineReturn13/14` | `00:82F3` | 纯 `rts` |
| `$15` | `StartContinueScreen15` | `00:90BA` | START/CONTINUE 屏（首行 `jsr ClearAll`） |
| `$16` | `ChooseStartContinue` | `00:90D7` | 选择：START→`InitializeStats`；CONTINUE→`$1F←$17` |
| `$17` | `LoadPasswordScreen` | `00:911A` | `ClearAll` + 装载密码屏 + `inc $1F`→`$18` |
| `$18` | `EnterPassword` | `00:9147` | 输入；START→`CheckPassword`；A→写单字符段 |
| `$19` | `DisplayPassword` | `00:9359` | `ClearAll` + 显示密码 + `WaitNMIPass` |
| `$1A` | `WaitForSTART` | `00:9394` | 等 START |
| `$1B` | `StartContinueScreen1B` | `00:90BA` | 同 `$15`（同一入口，靠 `$1F` 区分） |
| `$1C` | `GameOver` | `00:939E` | `ClearAll` + "GAME OVER" |
| `$1D` | `EndGame` | `00:9AA7` | 结局演出 |
| `$1E` | `SetTimer` | `07:C4AA` | 延时（`TitleRoutine` 轴上的 `SetTimer` 复用） |

### 5.2 标题轴 ↔ 游戏轴的唯一交接口

游戏侧 → 标题侧只有 E15/E16 两条（`$24←1` + `ScreenOff`）；
标题侧 → 游戏侧只有 E2/E3 两条（写 `$24` = 目标 bank 号），二者都汇入 E4：

```
[标题轴] $16 START → InitializeStats(00:932B)  ─┐
[标题轴] $18 START → CheckPassword → InitializeGame(00:92D4) ─┤ 都写 $24
                                                              ├→ CheckBankSwitch(07:C4DE)
[游戏轴] $1E=4 PrepareGameOver → $24←1 ─┐                     │   → GoBankInit → InitBank1..5
[游戏轴] $1E=6 GoPassword      → $24←1 ─┴→ 进 InitBank0        │   → GameMode←0（游戏侧）
                                                              └→ 进 InitBank0 → GameMode←1（标题侧）
```

**反方向**（游戏→标题）只有两条，都是 `$24←1` + `ScreenOff`：
`PrepareGameOver`（`$1E=4`）与 `GoPassword`（`$1E=6`），进入后由
`InitBank0` 把 `GameMode←1`，`TitleRoutine` 已由它们预先设好（`$1C` / `$19`）。

### 5.3 `ClearAll ; 00:909F`（:2316-2328）——标题侧的重型转场原语

```
jsr ScreenOff        ; PPUMASK 影子 &= ~(BG|OBJ) → WriteAndWait（等 1 NMI）
jsr ClearNameTables  ; 4 张名表各 1024B $FF（07:C158）
jsr EraseAllSprites
lda PPUCTRL_ZP / and #$FC / sta
lda #$00 / sta ScrollY / sta ScrollX
jsr WaitNMIPass      ; 再等 1 NMI   ← **第二次帧等待**
jmp VBOffAndHorzWrite
```

⇒ 任何调 `ClearAll` 的转场臂**至少跨 2 个 NMI、`inc FrameCount` 至多 1 次**。
`$15` / `$17` / `$19` / `$1C` 都调它（`$17` 还会在 `ClearNameTables` 之后**再**等一次，
见 b204 记录的 `$1F=0x17` 三帧冻结）。

---

## 6. 屏幕开关：为什么「关」比「开」晚一帧

这是全项目最容易走形的一处，单列。

```
ScreenOff ; 07:C439 : PPUMASK_ZP &= ~(BG_ON|OBJ_ON)
ScreenOn  ; 07:C447 : PPUMASK_ZP |= (SHOW8BG|SHOW8OBJ|BG_ON|OBJ_ON)
WriteAndWait ; 07:C43D : sta PPUMASK_ZP            ← **只写影子 $FE**
                          WaitNMIPass_ ; 07:C43F   ← 等一个 NMI
WritePPUCtrl ; 07:C44D : lda PPUCTRL_ZP / sta PPUCTRL
                         lda PPUMASK_ZP / sta PPUMASK   ← **真寄存器唯一落点，在 NMI 里**
ScreenNmiOff ; 07:C45D : mask 关 → WriteAndWait → 再关 PPUCTRL VBLKNMI
```

- 全仓只有两处写 `PPUMASK_ZP`：`prg7_engine.asm:165`（boot 进标题态）与 `:1131`（`WriteAndWait`）。
- ⇒ **`ScreenOff/ScreenOn` 只改影子，真正的黑/亮发生在下一个 NMI 的 `WritePPUCtrl`**。
  所以 `MoreInit` 尾的 `jmp ScreenOn` 之后，**下一帧**才复明（emu 实测暗窗 f1191-f1197，
  `docs/findings/` 暗窗建模记录）。

---

## 7. 转移合法性判据（给复核用）

任何 Rust 侧的「状态推进」若声称忠实，必须能填满下表；填不满就是拟合：

| 检查项 | 合法来源 | 违规表现 |
|---|---|---|
| `$1E` 变化 | `inc`（0→1/1→2/3→4/5→6）、`SetMainRoutine`（`$20`→`$1E`）、暂停开关（3↔5）、`SamusInit`（→8）、`SamusIntro`（→3） | 一次迭代跨多步、跳过中间态、在 `GameMode≠0` 时改 `$1E` |
| `$1F` 变化 | `inc`（绝大多数）、`ChooseStartContinue`（→`$17` 或跳 `InitializeStats`）、`PrepareGameOver`（→`$1C`）、`GoPassword`（→`$19`）、`MainTitleRoutine` 快捷门（→`$1B`） | 在 `GameMode=0` 时改 `$1F` |
| `$1D` 变化 | 只在 `InitBank0`（→1）与 `InitBank1..5`（→0） | 其它任何位置写 `$1D` |
| `$24` 消费 | 只在 `MainLoop` 头 `CheckBankSwitch`，且**先清后换页** | 同迭代多次消费、跳帧消费、消费而不清 |
| `FC` 增加 | 每次 MainLoop 迭代恰好 1 次，位于分派之后 | 例程体内自增、跨相位补偿、`+2` 恒偏 |
| 屏幕开关 | 只改影子；真寄存器在 NMI | 直接写 `$2001`、同帧点亮/熄灭 |

---

## 8. 时序契约（状态机 ↔ 帧）

| 事件 | 发生位置 | 对状态的影响 |
|---|---|---|
| NMI 触发 | `prg7_engine.asm:233` | `$1A≠0` 时整块 NMI 工作被跳过（只写 `$1A←1`）；否则跑 PPU 写/`ReadJoyPads`/`SoundEngine` 等 |
| `ReadJoyPads ; 07:C215` | NMI 内（`:268`） | 写 `Joy1Change($12)/Joy1Status($14)/Joy1Retrig($16)` —— **输入每帧只在此更新一次** |
| `UpdateTimer ; 07:C266` | MainLoop 头 | `TimerDelay` 9→0 循环；`Timer3` 每 10 帧 `dec`（非 0 才减） |
| 长例程忙跨 | 任意例程体 | NMI 落在 `$1A=0` 窗口 ⇒ 被吞 ⇒ 视觉/输入掉帧（原版 quirk，D4 保留） |
| `WaitNMIPass` | 例程体内 | 本帧 `inc FC` 不发生 |

---

## 9. asm 形状 vs Rust 现状（对照，**不改代码**）

### 9.1 已实现且形状吻合

| asm | Rust 归宿 | 备注 |
|---|---|---|
| `GoMainRoutine ; 07:C114` 的 GameMode 早退 | `crates/game-core/src/frame/mainloop.rs` `game_main_routine` 头部 | 2026-11-11 补（A 级，`$1D≠0 ⇒ main_title_routine`） |
| `$24` 消费点（帧界、先清后换） | 同上 `initialize_game_pending_bank_switch` 分支 + `crate::frame::boot::c4de_unit` | 2026-11-11 补 |
| `MainRoutinePtrTable` 10 项 | 同上 `MAIN_ROUTINE_PTR_TABLE: [u16; 10]` | 值逐项与 `07:C141` 一致 |
| `TitleRoutinePtrTable` 31 项 | `crates/game-core/src/frame/title_bridge.rs` + `crates/game-core/src/title/mod.rs` | 表项语义分离在 `title/mod.rs` |
| `BankInitTable` 9 项 | `crates/game-core/src/frame/boot.rs` `init_bank0..5` | `InitBank2..5` 2026-11-11 补 |
| `AreaInit → MoreInit` fallthrough | `game_main_routine` 的 `0 => { area_init(); more_init() }` | **代码正确**，但同行注释写「无 RTS fallthrough」与实现矛盾（§10 更正项） |
| `WaitTimer` | `game_main_routine` 的 `9 => {…}` | `NextRoutine` 4/6 免起乐已按 asm |

### 9.2 未实现（`unported!` 或缺失）——**这是 §11 复测清单的输入**

| asm | 缺口类型 | 现状（2026-11-13 核查） |
|---|---|---|
| `GoMainRoutine` 前置的 START 暂停开关（3↔5）+ `jsr PauseMusic` | **完全缺失** | `mainloop.rs::game_main_routine` 头部无任何 `Joy1Change & START` 处理 ⇒ 游戏内按 START 不会暂停 |
| `PauseMode ; 07:C9B1`（`$1E=5`） | **完全缺失** | `match mr` 无 `5` 臂 ⇒ `unported!("bank07:C9B1")`；Joy2 `A+UP` 门与 `EndTimer+1` 门都未落地 |
| `PrepareGameOver ; 07:C9A6`（`$1E=4`） | **完全缺失** | 无臂、无纯函数层（全仓 grep 零命中） |
| `GoPassword ; 07:C9C4`（`$1E=6`） | **逻辑已实现，接线缺失** | `crates/game-core/src/ui/password.rs` 有 `go_password()` + `go_password_writes()`（3 写点 `$1F/$24/$0680`，含穷举测试），但 `match mr` 无 `6` 臂 ⇒ 未被调用 |
| `IncrementRoutine ; 07:C155`（`$1E=7`） | 无臂（`inc $1E` 一行） | 真实流程中从未被走（MoreInit 用内联 `inc`），见 §10.6 |
| `UpdateAge ; 07:C97E` | **完全缺失** | NMI 内年龄累加（`GameMode=0 ∧ $1E=3 ∧ FC==0` 时 24 位递增，最低位 `$D0` 溢出）未端口 |
| `EndGame ; 00:9AA7`（`$1F=$1D`） | 未读 / 未端口 | 结局演出，Stage A 尾部 |

### 9.3 结构性替代（**拟合，最高风险**）

Rust 侧用一套自己的机构替代了 §2.1 的三条原版机制：

| Rust 机构 | 声明语义 | 与原版的关系 | 风险 |
|---|---|---|---|
| `title_cursor` / `title_cursor_phase` | 把一条 asm 例程切成多次迭代的「段」 | 原版是**例程体内自旋/等待**，不切段 | 段的切法由人手定 ⇒ 时序可任意拟合 |
| `title_spin` / `SpinWait(n)` | 代表例程体内的 `WaitNMIPass` | 方向对，但**次数是写的不是算的** | 每次新臂都要手工数等待 |
| `title_no_return`（白名单） | 该臂体不返回 ⇒ 本帧 `inc FC` 不执行 | 原版判据是「例程体是否自己等 NMI」 | 白名单漏一项=FC 恒偏（已发生：`$17` 三帧冻结） |
| `iter_gates` / `end_of_tick_gate` | 模拟 `NMIStatus` 握手与掉帧窗 | 原版由 `$1A` 单一位置决定 | 与 `work_cycles` 估算耦合 |
| `fc_inced_this_tick` | 抑制重复 `inc FC` | 原版不存在此概念 | 掩盖真实的多/少 inc |
| `check_password_pending` | `CheckPassword` 链延一拍 | 原版因 `UnscramblePassword` 忙跨自然延后 | 固定 1 拍（对零忙密码过量，已登记 L1） |

> 这六项**互相耦合**，是本项目「拟合成分」的主产地（审计 `/tmp/fit-audit.md` Top10 之 L 级）。
> 它们不是错误，但**每一个都必须在本文 §2.1 里能找到对应的原版机制**；找不到的即为待替换项。

---

## 10. 取证缺口与更正项

1. **`$1F` 表项 `$12 TitleScreenOff` 自称 unreachable**（`prg0_title.asm:604-607` 注释原文 `This routine should not be reached.`）：本仓未验证其是否真不可达；Rust 侧有对应臂。
2. **`MainRoutine` 注释措辞歧义**：`mainloop.rs` 的 `0` 臂注释写「asm AreaInit **无 RTS fallthrough** 进 MoreInit」——
   本意是「AreaInit 结尾没有 `rts`，因此物理落到 `MoreInit`」（正确），但该措辞可被读成
   「AreaInit 不会 fallthrough」。**代码与 asm 一致**，建议改写为
   「AreaInit 尾 `jsr IsEngineRunning` 在 `$1E=1` 时直接 `rts`，随后物理落到 `C81D`」，docs-only。
3. **`InitBank0` 的 `CopyMap` → WRAM `$7000-$73FF`**：Rust 侧 `world_map` 烘焙已有，但 WRAM 装载路径未见明确接线记录（2026-11-09 `read_wram` 分派修复只解决读，未核「写」）。
4. **`PauseMusic ; $CB92`** 与 `PauseMode`/`GoPassword` 的声链交互未取证。
5. **`EndGame ; 00:9AA7`** 整条未读（结局演出，属 Stage A 尾部）。
6. **`$1E=7 IncrementRoutine`** 在真实流程里从未被走（MoreInit 用内联 `inc`）——它是表项完整性还是死项，未定谳（B 级）。
7. `UpdateAge` 的 24 位年龄域（`SamusAge` 4 字节，最低位 `$D0` 溢出）未在 Rust 侧落地。

---

## 11. 快速对拍机制（学 `super-metroid-decomp` 的 `.sem`）

### 11.1 为什么需要它

L4（像素）能裁「画面对不对」，但它有两个致命低效：
①单轮矩阵 **440s**（97% 的门禁耗时，已按用户裁定移出 git hook）；
②失败只给「第 N 帧第 M 个像素不同」，**不给状态**——定位要回落内存 dump 手比。

`super-metroid-decomp` 的做法（`docs/plan/two-verification-stacks.md`）是
**两套栈分离**：语义轨迹（`.sem`，每帧一行、纯文本、毫秒级 diff）与帧截图（像素）
各自独立、互不合并。理由：分辨率不同（每帧一条 vs 稀疏关键帧）、
失败语义不同（首分歧帧 vs 该帧画错）、依赖不同（语义不需要 PPU）。

**这正是本项目缺的一层**：我们已经有两个半成品（Mesen 侧 `watch` 的 `.frames.jsonl`、
Rust 侧 `--state-out`/`DSH_FC_TRACE`），但没有统一格式、没有 diff 工具、没有指纹。

### 11.2 列契约（提议，**首版 16 列**）

只放**状态机与核心机制**的观测量，不放渲染：

| 列 | 来源 | 语义 |
|---|---|---|
| `f` | 迭代序号 | 帧号（两侧同锚） |
| `fc` | `$2D` | FrameCount |
| `gm` | `$1D` | GameMode |
| `mr` | `$1E` | MainRoutine |
| `tr` | `$1F` | TitleRoutine |
| `gp` | `$31` | GamePaused |
| `nr` | `$20` | NextRoutine |
| `t3` | `$2C` | Timer3 |
| `cb` | `$23` | CurrentBank |
| `bsp` | `$24` | BankSwitchPending |
| `ia` | `$74` | InArea |
| `rm` | `$5A` | RoomNumber |
| `st` | `$0300` | Samus.status |
| `sy` | `$030D` | Samus.y |
| `sx` | `$030E` | Samus.x |
| `hp` | `$0106/$0107` | Health（BCD 两字节合并为整数展示） |

### 11.3 纪律（照搬社区三条，逐条有本仓对应物）

1. **`.sem` 内容不入库**，入库三样：场景 DSL（`cases/*.txt`，已存在）+ `<name>.sem.meta.json`
   指纹（`frames/cols/rom_sha256/sha256/runtime/note`）+ 重生成器。
   理由：快照会立刻腐烂，且诱导「改数字让它变绿」。
2. **只写真正追踪的列**，不写 0 冒充（AGENTS §3）。两侧列集不同时，`sem_subset_diff`
   只比**交集**。
3. **金面前置 = ≥3 连跑 sha256 全同**（社区实证 rng 由游戏逻辑驱动、非墙钟）。

### 11.4 与 L4 的分工（不合并）

| | `.sem` 语义对拍（新建） | L4 像素对拍（现役） |
|---|---|---|
| 判据 | 逐帧 16 列全等 | 采集点整屏像素 0 差 |
| 粒度 | 每帧 | 稀疏关键帧 |
| 耗时目标 | **< 2s/用例**（无 Mesen 截图、无渲染） | 440s/矩阵 |
| 感知不到 | 渲染、VRAM 内容、OAM | 状态错但画面恰好一致 |
| 门禁位置 | pre-commit 快层（若达标） | 显式命令 / 里程碑 / CI |

---

## 12. 待办（本文落地后）

1. **本文入库**（docs-only，不改代码）。
2. **`tools/sem_diff.py`**：两侧 `.sem` 子集 diff（照 `sem_subset_diff.py` 口径），
   先能读现有 `analysis/traces/*.frames.jsonl` 与 Rust 侧 `--state-out` 产物。
3. **两侧 `.sem` 生成**：Mesen 侧从 `watch` 直出；Rust 侧从 `state_row` 直出。
4. **§9.2 缺口清单**按「玩家可感知度 × 代价」排序，逐项立卡（每卡 = 建模 → 接线 → 复测）。
5. **§10 更正项**：`mainloop.rs` 的 `0` 臂注释（docs-only commit）。

---

## 附：本文引用的 asm 锚点索引

| 例程 | 位置 |
|---|---|
| `MainLoop` | `07:C0BC`（`prg7_engine.asm:199-223`） |
| `inc FrameCount` | `07:C0C5`（ROM 字节 `e6 2d`） |
| `NMI` | `prg7_engine.asm:233-292` |
| `GoMainRoutine` / `MainRoutinePtrTable` | `07:C114` / `07:C141`（:301-355） |
| `IncrementRoutine` | `07:C155`（:357-362） |
| `UpdateTimer` | `07:C266`（:654-682） |
| `JumpEngine` | `07:C27C`（:687-713） |
| `WaitNMIPass` / `WriteAndWait` / `ScreenOff` / `ScreenOn` / `WritePPUCtrl` / `ScreenNmiOff` | `07:C42C` / `07:C43D` / `07:C439` / `07:C447` / `07:C44D` / `07:C45D` |
| `WaitTimer` / `SetTimer` / `SetMainRoutine` | `07:C494` / `07:C4AA` / `07:C4A7`（:1219-1245） |
| `CheckBankSwitch` / `GoBankInit` / `BankInitTable` | `07:C4DE` / `07:C510` / `07:C51F`（:1296-1375） |
| `InitBank0..5` | `07:C531` / `07:C552` / `07:C583` / `07:C590` / `07:C5B6` / `07:C5C3` |
| `AreaInit` / `MoreInit` / `SamusInit` / `GameEngine` / `PrepareGameOver` / `PauseMode` / `GoPassword` / `SamusIntro` | `07:C801` / `07:C81D` / `07:C8D1` / `07:C92B` / `07:C9A6` / `07:C9B1` / `07:C9C4` / `07:C9D7` |
| `UpdateAge` | `07:C97E`（:2088-2121） |
| `IsEngineRunning` | `07:CA18`（:2247-2254） |
| `MainTitleRoutine` / `TitleRoutinePtrTable` | `00:8000` / `00:802A`（`prg0_title.asm:27-86`） |
| `ClearAll` | `00:909F`（`prg0_title.asm:2316-2328`） |
| `CheckPassword` / `InitializeGame` / `InitializeStats` | `00:8C5E` / `00:92D4` / `00:932B` |
