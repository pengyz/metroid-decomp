# 结局链规格：`EndGame` / `ChooseEnding` / `SamusAge`

> 卡：#78（B9 前置）。写面仅本文件。证据级：**A** = ROM 字节/逐行反汇编；**B** = 调用图与数据流推断；**C** = 假设（只进 §8）。
> 上游：`docs/spec/statue-bridge-chain.md` 已证「结局门与 `$687B/$687C` 无关；进 Tourian 靠桥铺出的物理通路」。本文从「结局怎么触发、画面怎么放」接着写，不重复桥链。

## 0. 结论速览与地址速查

| 关键词 | 地址 | 说明 |
|---|---|---|
| `EndGame` | `00:9AA7`（`prg0_title.asm:3712`） | 结局主例程，每帧一次；`RoomPtr` 作段号（0–6） |
| 段分派 | `00:9AC3` `jsr JumpEngine` | 7 段：`9AD5/9B1C/9B34/9B93/9BCD/9BFC/9A39`，第 7 项 `C45C ExitSub` |
| `ChooseEnding` | `07:CAF5`（`:2439`） | 按 `SamusAge+2/+1` 选 `EndingType` 1–5，并写 `JustInBailey` |
| `AgeTable` | `07:CB18`（`:2471`） | `7A 16 0A 04`（37h / 6.7h / 3.0h / 1.2h） |
| `UpdateAge` | `07:C97E`（`:2088`） | NMI 内每 256 帧递增一次 `SamusAge` 低字节 |
| `SamusAge` | `$687D`–`$6880`（`constants_ram.asm:1236-1239`） | 4 字节，低位是「256 帧单位」，到 `$D0` 进位 |
| `EndingType` | `$6872`（`:1223`） | 1=最差 … 5=最好 |
| `JustInBailey` | `$69B3`（`:1286`） | 0=有服 / 1=无服（`EndingType>=4`） |
| `AtEnding` | `$6883`（`:1243`） | 进结局链的标记；`$8F` 电梯分支 `inc`，`InitializeStats`/`room_load` 清 0 |
| `IsCredits` | `$7B`（`:677`） | 0=没在滚职员表 / 1=在滚 |
| `CreditPageNumber` | `$80`（`:682`） | 0–6，`>6` 停 |
| `Timer3` | `$2C`（`:518`） | 每 10 帧减 1 的延时计数器（本文用作 960/160 帧门） |
| `ScrollY` | `$FC`（`:817`） | 职员表滚动位置，到 `$F0` 换名表 |
| 结局 GFX 结构 | `VRAMStruct_EndBackground 00:A052` | 地面 5 行 + 属性 + 「STAFF」等文本 |
| 职员表指针表 | `CreditsPtrTable 00:A291` | **44 项**（`00–2B`），每项一个 VRAMStruct |
| 结局精灵指针 | `EndSamusAddrTbl 00:9D5A` | 6 帧：有服/背身/举拳/摘盔/连体衣/比基尼 |

**一句话**：`$8F` 电梯把 `TitleRoutine` 切到 `0x1D EndGame`；`EndGame` 用 `RoomPtr` 走 7 段固定脚本（载 GFX → 站着 → 闪 → 挥手 → 淡出 → 职员表 → 重开）；`ChooseEnding` 在段 2 调到一半时按游戏时长定 1–5 档，决定精灵帧与是否无服。

## 1. 调用链（A）

### 1.1 触发：`$8F` 电梯（`07:D8BF`，`:4980`）

```
ElevatorD8BF:
  lda ElevatorType-(Elevator-Objects),x
  tay
  cmp #$8F
  bne @endIf_A
    lda #_id_IncrementRoutine.b / sta MainRoutine
    inc AtEnding            ; $6883 += 1
    ldy #$00 / sty RoomPtr  ; 段号归 0
    iny / sty BankSwitchPending   ; 切 bank0（title 页）
    lda #_id_EndGame.b / sta TitleRoutine   ; 0x1D
    rts
```
ROM 字节（`07:D8BF` 起 22 B）：`BD 0F 03 A8 C9 8F D0 13 A9 07 85 1E EE 83 68 A0 00 84 33 C8 84 24`（复核方在 #76 已逐字节核过同一段）。

`TitleRoutine=0x1D` 后由 bank0 主循环分派：`TitleRoutinePtrTable`（`prg0_title.asm:60` 起，项 0x1D）→ `EndGame 00:9AA7`。

### 1.2 `EndGame` 每帧骨架（`00:9AA7`，`:3712`）

```
EndGame:
  jsr LoadEndStarSprites        ; 星精灵
  lda IsCredits / bne @jumpEngine
  lda FrameCount / and #$0F / bne @jumpEngine
    inc PaletteDataPending      ; 每 16 帧换一档结局调色板
    cmp #_id_EndGamePalette08+1 / bne @jumpEngine
    lda #0 / sta PaletteDataPending
@jumpEngine:
  lda RoomPtr / jsr JumpEngine
    .word LoadEndGFX 9AD5, ShowEndSamus 9B1C, EndSamusFlash 9B34,
          SamusWave 9B93, EndFadeOut 9BCD, RollCredits 9BFC,
          Restart 9A39, ExitSub C45C
```
跳转表 ROM 字节（`00:9AC5` 起 16 B，即 8 个指针）：`D5 9A 1C 9B 34 9B 93 9B CD 9B FC 9B 39 9A 5C C4`。

### 1.3 七段各自的「推进条件」（A，逐段读）

| 段 | 标签 | 行为要点 | 进下一段的条件 |
|---|---|---|---|
| 0 | `LoadEndGFX 9AD5` | `ClearAll` → `InitEndGFX`（`GameMode←1`）→ `EndingType←(JustInBailey?$04:0)`、`SpritePointerIndex←EndingType*2` → 背景 VRAM `A052` → `NMIOn` → 结局音乐 → `Timer3←$60`(960 帧) → `SpriteByteCounter←$36` → 清 `SpriteAttribByte/ColorCntIndex/IsCredits/EndMsgWrite/HideShowEndMsg/CreditPageNumber` → `PaletteDataPending←1`、`ClrChangeCounter←8` → **`inc RoomPtr`** | 本帧必进段 1 |
| 1 | `ShowEndSamus 9B1C` | `LoadEndSamusSprites`；三个 `Timer3` 门：`==$50` ⇒ `EndMsgWrite←1`；`==1` ⇒ `HideShowEndMsg←1` | `Timer3==0`（960 帧后）⇒ `inc RoomPtr` |
| 2 | `EndSamusFlash 9B34` | `FrameCount&$1F==0` ⇒ `inc ColorCntIndex`；`==8` ⇒ **`jsr ChooseEnding` + `jsr CalculatePassword`** + 重设 `SpritePointerIndex/SpriteByteCounter`；`==$10` ⇒ `Timer3←$10`、`SpriteAttribByte←(EndingType>=4)`、`inc RoomPtr`、`EraseAllSprites`；其余帧按 `PaletteChangeTable[ColorCntIndex]` 递减 `ClrChangeCounter` 并循环 `SpriteAttribByte 0→3` | `ColorCntIndex` 到 `$10` |
| 3 | `SamusWave 9B93` | `Timer3==0` ⇒ `Timer3←$10`、`PaletteDataPending←7`、`inc RoomPtr`；否则 `EndingType<4` 就只重载普通帧，`>=4` 时 `(EndingType-4)<<2` 作 `WaveSpritePointer`，`FrameCount&8` 决定手到手背（+2） | 先等 160 帧，再进段 4 |
| 4 | `EndFadeOut 9BCD` | `Timer3==0` 后每 8 帧 `PaletteDataPending+1`，到 `$0C` ⇒ `Timer3←$10`、`inc RoomPtr`（另有一段 asm 自称「appears not to be used」的 `IsCredits` 置位，照 ROM 保留） | 淡出完成 |
| 5 | `RollCredits 9BFC` | `Timer3==2` ⇒ 清屏换 `$0D` 调色板、`WaitNMIPass`；`CreditPageNumber==6 && ScrollY>=$88` ⇒ `inc RoomPtr`；`FrameCount&3==0` ⇒ `ScrollY+1`，到 `$F0` ⇒ 换名表（`PPUCTRL^=$02`）、翻页、`ScrollY←0`；`LoadCredits` 在每帧 NMI 里补滚动露出的 4 行 | 6 页滚完 |
| 6 | `Restart 9A39` | 按 START ⇒ 清 `PasswordByte00–11` 与 `UniqueItemHistory`、存装备、`jmp InitializeGame` | 回新游戏 |

**说明（B）**：`Timer3` 是每 10 帧减 1 的通用延时器（`constants_ram.asm:518` 注释），所以 `$60`=960 帧（16 秒）、`$10`=160 帧（2.6 秒），与 asm 注释一致；段 2 的 `PaletteChangeTable` 让闪烁「慢→快→慢」。

**`AtEnding` 的角色（A）**：全仓仅三处写（`$8F` 分支 `inc`、`InitializeStats` 清 0、`room_load` 清 0），没有代码读它来改结局判定——它是「进结局链」的标记与清场依据，不是分支条件。

## 2. `ChooseEnding` 与阈值（A）

```
ChooseEnding ; 07:CAF5
  ldy #$01
@loop:
  lda SamusAge+2 / bne @endingFound     ; +2 非 0 ⇒ 超过 37 小时，直接最差
  lda SamusAge+1 / cmp AgeTable-1,y / bcs @endingFound
  iny / cpy #$05 / bne @loop            ; 一路过 4 档 ⇒ 最好
@endingFound:
  sty EndingType                        ; $6872 ← 1..5
  lda #$00 / cpy #$04 / bcc @endIf_A / lda #$01
@endIf_A:
  sta JustInBailey                      ; $69B3
```
ROM 字节（`07:CAF5` 起 **35 B**，`$60` 的 rts 为末字节，`0x23` 之后即 `AgeTable`）：`A0 01 AD 7F 68 D0 0D AD 7E 68 D9 17 CB B0 05 C8 C0 05 D0 EE 8C 72 68 A9 00 C0 04 90 02 A9 01 8D B3 69 60`。
`AgeTable`（`07:CB18`）：`7A 16 0A 04`——`SamusAge+1 >= $7A` ⇒ 1；`>= $16` ⇒ 2；`>= $0A` ⇒ 3；`>= $04` ⇒ 4；否则 5。`>=4` ⇒ 无服。

**保真 bug（A，D4）**：asm 注释自承「BUG! Should be checking `SamusAge+3` here too」——只查 `SamusAge+2`，高字节溢出到 `+3` 时不会判成最差结局。Rust 已按原样保留（`title/mod.rs:1972-1979` 的 `rom_overflow_bug_is_preserved`）。

**时间单位（A）**：`SamusAge` 低字节到 `$D0` 进位（见 §3），所以：
`+0` 单位 = 256 帧 ≈ 4.267 s；`+1` 单位 = `$D0`×256 帧 = 53248 帧 ≈ 14.8 min；`+2` 单位 = 256×53248 帧 ≈ 63 h。asm 注释把 `+1` 档写「6.7 h」是按 `$16×53248/60/60` 算的近似，本文不采用该近似值，只采用阈值字节。

## 3. `SamusAge` 的写读链（A）

### 3.1 每帧递增（`UpdateAge 07:C97E`，`:2088`）

```
UpdateAge:
  lda GameMode / bne @RTS          ; 标题/密码屏不涨
  lda MainRoutine / cmp #_id_GameEngine / bne @RTS   ; 非游戏运行不涨
  ldx FrameCount / bne @RTS        ; 只在 FrameCount==0 这一帧（每 256 帧）
  inc SamusAge,x                   ; 低字节 +1
  lda SamusAge / cmp #$D0 / bcc @RTS
  lda #$00 / sta SamusAge          ; 到 $D0 清零
@loop:
  cpx #$03 / bcs @RTS
  inx / inc SamusAge,x / beq @loop ; 进位链
```
ROM 字节（`07:C97E` 起 40 B）：`A5 1D D0 23 A5 1E C9 03 D0 1D A6 2D D0 19 FE 7D 68 AD 7D 68 C9 D0 90 0F A9 00 8D 7D 68 E0 03 B0 06 E8 FE 7D 68 F0 F6 60`。
⇒ 低字节不是十进制 BCD，而是「到 `$D0` 进位」的自定义进制。

### 3.2 写点/读点清单

| 方向 | `bank:addr` | 位置 | 条件/语义 | 级 |
|---|---|---|---|---|
| 写 | `07:C97E` | `UpdateAge`（NMI 内） | `GameMode==0 && MainRoutine==GameEngine && FrameCount==0` ⇒ 低字节+1，`$D0` 进位 | A |
| 写 | `00:8D33` | `LoadPasswordData` | 密码解码：`PasswordByte+$0B..$0E` → `SamusAge..+3` | A |
| 写 | `00:932B` 区段（`:2784-2788`） | `InitializeStats` | 新游戏清 0 | A |
| 读 | `00:8CF7` | `CalculatePassword` | `SamusAge..+3` → `PasswordByte+$0B..$0E` | A |
| 读 | `07:CAF5` | `ChooseEnding` | 只用 `+1`/`+2`（`+3` 不读——见 §2 bug） | A |
| 清 | `07:DDxx` 区段 | `room_load` 清场（Rust `room_load.rs:19` 同址集） | 进房清 `AtEnding` 等 | B |

`AtEnding $6883`：写 `07:D8BF`（`inc`）；清 `00:932B` 区段、`room_load`；无读取分支。

## 4. 结局相关字节（汇总）

| 址 | 名 | 语义 | 写点 | 读点 |
|---|---|---|---|---|
| `$6872` | `EndingType` | 1–5，5 最好 | `07:CAF5` `sty`；`00:9AD5` 段 0 初始按 `JustInBailey` | 段 2/3/4、`LoadEndSamusSprites`、密码侧 `$08` |
| `$687D–$6880` | `SamusAge` | 时长，4 字节 | §3.2 | §3.2 |
| `$6883` | `AtEnding` | 进结局链标记 | `07:D8BF`；清 2 处 | 无 |
| `$69B3` | `JustInBailey` | 有无服 | `07:CAF5`；密码 `$08` bit7 | 段 0、`CalculatePassword` |
| `$7B` | `IsCredits` | 职员表滚动中 | 段 0 清 0、段 4 置 1 | `EndGame` 头（跳过调色板循环） |
| `$80` | `CreditPageNumber` | 职员表页码 0–6 | 段 0 清 0、段 5 翻页、`LoadCredits` | 段 5 |
| `$2C` | `Timer3` | 10 帧/格延时 | 段 0 `$60`、段 2/3/4 `$10` | 各段门 |
| `$FC` | `ScrollY` | 职员表滚动 | 段 5 | 段 5、`LoadCredits` |
| `GameMode` `$1D` | 页切换 | 段 0 `InitEndGFX` 置 1 | `UpdateAge` 早退判据 |

## 5. 数据表（A）

### 5.1 VRAMStruct 记录布局（`macros.asm:32-73`）

```
[ppuAddrHi][ppuAddrLo][count|$40 若重复] [数据...]
...
$00    ; VRAMStructEnd
```
`VRAMStructData` 写 `count` 个字节；`VRAMStructDataRepeat` 写 `count|$40` 表示重复 1 个字节 `count` 次。

### 5.2 结局背景 + 文本（`VRAMStruct_EndBackground 00:A052`）

表头 ROM 字节（`00:A052` 起 10 B）：`23 00 20 30 31 30 31 30 31 30`——即 `$2300` 写 32 B 的 `30/31` 交替地面砖；随后 `$2320/$2340/$2360/$2380/$23A0` 五行；`$23F0` 重复 `$10` 次 `$FF`（属性全满）；然后往名表 2 写 `$282E "STAFF"`、`$28A8 "SCENARIO WRITTEN BY"` 等。

### 5.3 调色板

- 循环：`EndGame` 每 16 帧 `PaletteDataPending+1`，到 `$09`（`_id_EndGamePalette08+1`）回 `$01`。
- 指针表 `EndGamePalettePtrTable 00:9F81` 共 **13 项**（索引 0–`$0C`）：`9F9B / 9FBF / 9FCB / 9FD7 / 9FE3 / 9FEF / 9FFB / A007 / A013 / A02E / A049 / A049 / 9806`（`$0A` 重复两次；`$0B` 落 `00:9806 EndGamePalette0B`）。
- 淡出：段 4 每 8 帧到 `$0C`；职员表：段 5 用 `$0D`。

### 5.4 结局精灵

- 指针表 `EndSamusAddrTbl 00:9D5A`：`66 9D 9C 9D D2 9D 08 9E 3E 9E 74 9E` ⇒ `NormalSamus 9D66 / BackTurned 9D9C / FistRaised 9DD2 / HelmetOff 9E08 / Jumpsuit 9E3E / Bikini 9E74`。
- 每帧 3 字节 `[y][tile|$80=水平翻转][x]`，属性字节由 `SpriteAttribByte` 统一给（翻转时 `eor #$40`）。
- `SpriteByteCounter=$36`（54 B = 18 个 3 字节）= 6×3 精灵。
- 摘盔结局（`EndingType==3`）后半程用 `SamusHeadSpriteTable 00:9CFA`（6×4 B）覆盖头部，防头部一起闪。
- 挥手表 `WavePointerTable 00:9D12` 4 项（`9D1A/9D2A/9D3A/9D4A`：连体衣手上/下、比基尼手上/下），每表 4×4 B，每 8 帧换一次手型。

### 5.5 职员表

- `CreditsPtrTable 00:A291`，**44 项**（`00:A291`–`00:A2E8`，索引 0–`$2B`）。
- 页与索引公式（`LoadCredits 00:9C45`，`:4041`）：
  - `CreditPageNumber==0` 或 `>6` ⇒ 不写；
  - `ScrollY<128`：`idx=(Page-2)*8+4+(ScrollY&3)`；
  - `ScrollY>=128`：`idx=(Page-1)*8+(ScrollY&3)`；
  - 每 4 帧滚 1 行（`FrameCount&3==0`），到 `$F0` 换名表（`PPUCTRL^=$02`）、`ScrollY←0`、`Page+1`。

## 6. Rust 对照（`grep` 实测行号）

| 环节 | Rust | 状态 |
|---|---|---|
| 触发 `AtEnding++` 等五写 | `world/elevator.rs:274-284` | 已端口（逐写对表） |
| `EndGame` 每帧骨架 | `title/mod.rs:689 end_game` + `0x1D` 分支 `:1004` | 已端口 |
| 段 0 `ClearAll` 三段 | `title/mod.rs:726/734/766`（`clear_all_seg0/1/2_events`） | 已端口 |
| 段 0 余下（GFX/音乐/Timer3/清旗/`inc RoomPtr`） | `title/mod.rs:1033-1077` | 已端口 |
| 段 1 三个 `Timer3` 门 | `title/mod.rs:1093-1096` | 已端口；精灵装载为 shell 调用 |
| 段 2 闪烁 + `ChooseEnding` | `title/mod.rs:1099-1163` | 已端口 |
| 段 3 挥手 | `title/mod.rs:1166-1199` | 已端口（`LoadWaveSprites` shell） |
| 段 4 淡出 | `title/mod.rs:1181-1210` | 已端口 |
| 段 5 职员表滚动 | `title/mod.rs:1211-1240` | **部分实现**：滚动逻辑在，但页码/滚动字节的写入地址见 §6.1 D1；未接 `LoadCredits` 的 VRAM 结构 |
| 段 6 重开 | `title/mod.rs:1241-1260` | 已端口 |
| `ChooseEnding` | `title/mod.rs:1941 choose_ending`（+测试 `:1961-1985`） | 已端口，含原版 bug 保真 |
| `UpdateAge` | `frame/nmi.rs:385 update_age`（测试 `:595-609`） | 已端口 |
| 精灵装载 | `title/ending_sprites.rs:59 load_end_samus_sprites`、`:48` 附近 `load_wave` | 已端口（OAM 写面） |
| 密码 `SamusAge` 编解码 | `title/bridge.rs:172-174`（读）、`:745`（写） | 已端口 |
| 结局背景/文本/职员表 VRAM | **无** | **未端口**：`A052` 五个地面行与 `CreditsPtrTable` 44 条文本都还没进 Rust（`grep -rn "STAFF" crates/` 零命中） |
| `ending/` 模块 | `ending/mod.rs` 仅 1 行注释 | **空壳**（无实现） |

### 6.1 实现侧缺陷（A 级，供实现卡；本文只取证）

`TitleStep.events` 是 `(addr, u8)` 真 RAM 写（`title/state.rs:270 apply_title_step` 直接 `write_ram`，域外 `$0000-$07FF`/`$6000-$7FFF` 之外 panic）。而 `0x1D` 分支里用了一批**私有槽号**当地址：

| Rust 位置 | 写入地址 | 注释说的是 | ROM 真地址 | 后果 |
|---|---|---|---|---|
| `title/mod.rs:1043` | `0x0F` | EndingType | `$6872` | 结局档位没落真字节 |
| `:1044` | `0x10` | SpritePointerIndex | `$007D` | 同上 |
| `:1046` | `0x11` | SpriteByteCounter | `$007C` | 同上 |
| `:1047` | `0x12` | SpriteAttribByte | `$007E` | 同上 |
| `:1048` | `0x13` | ColorCntIndex | `$007F` | 同上 |
| `:1049` | `0x14` | IsCredits | `$007B` | **写进 `$0014 Joy1Status`** |
| `:1050` | `0x15` | EndMsgWrite | `$007A` | **写进 `$0015 Joy2Status`** |
| `:1051` | `0x16` | HideShowEndMsg | `$0081` | 落在零页 |
| `:1052` | `0x17` | CreditPageNumber | `$0080` | 落在零页 |
| `:1054` | `0x18` | ClrChangeCounter | `$0082` | 落在零页 |

同页还有两处内部不一致，说明这批数字是**内部槽位而非地址**：
1. `:1090`/`:1159` 读 `0x13`/`0x18`，`:1145` 读 `0x0F`，`:1216` 又用 `0x19` 当 `CreditPageNumber`（而 `:1052` 用的是 `0x17`）——同一状态两个槽。
2. `choose_ending` 的调用点 `:1101` 读 `face.wram[0x69B0/0x69B1/0x69B2]`，注释是 `SamusAge`；真地址是 `$687D/$687E/$687F`（`$69B0` 是密码旁的空闲页，`CalculatePassword` 才在 `ChooseEnding` **之后**跑，见 §1.3 段 2）。

复现（无需 emu）：

```bash
grep -n "events.push((0x1[0-9],\|events.push((0x0F," crates/game-core/src/title/mod.rs
grep -n "fn apply_title_step" -A6 crates/game-core/src/title/state.rs
grep -n "0x69B0" crates/game-core/src/title/mod.rs
```
实跑：前两条分别命中 `:1043-1054` 那十行与 `state.rs:270-277`（`mem.write_ram(addr,v)`，`addr<0x2000` 直接落零页）；第三条命中 `:1101` 的 `face.wram[0x69B0]`。

**结论**：结局链的**逻辑骨架**已按 asm 分段移植，但**状态字节没有落到 ROM 地址**，且会覆盖手柄状态（`$0014/$0015`）。（已查 `crates/game-core/src/` 无任何「槽号→真地址」映射层：`apply_title_step` 是唯一应用面；若后续发现映射层，请在实现卡更正本条。）L4 目前没有拍到结局段，所以门禁不会红——这与 `#67` 的「本体已端口、连接点缺失」同类。属实现卡要修的 A 级缺陷，本文不改代码。

**行为待验（B）**：段 5 的滚动只在 `title/mod.rs` 里改 `SCROLL_Y`/`PPUCTRL`，真正的「露出的 4 行由 NMI 补写」依赖 `LoadCredits`；Rust 侧尚无对应 NMI 面，L4 在结局段会出现整屏文本缺失（须在实现卡里接 `CreditsPtrTable`）。

## 7. 复现命令（实跑输出已核对）

```bash
# 1) ChooseEnding + AgeTable 字节
python3 - <<'EOF'
p=open('rom/Metroid (USA).nes','rb').read()[16:16+128*1024]
b=p[7*0x4000+(0xCAF5-0xC000):][:35]   # 35 B，第 36 字节已属 AgeTable
print(' '.join(f'{x:02X}' for x in b)); print('agetbl', list(p[7*0x4000+(0xCB18-0xC000):][:4]))
EOF
# 2) UpdateAge 字节（$D0 进位链）
python3 -c "p=open('rom/Metroid (USA).nes','rb').read()[16:16+128*1024];print(' '.join(f'{x:02X}' for x in p[7*0x4000+(0xC97E-0xC000):][:40]))"
# 3) EndGame 跳转表（8 个指针 → 7 段 + ExitSub）
python3 -c "p=open('rom/Metroid (USA).nes','rb').read()[16:16+128*1024];print(' '.join(f'{x:02X}' for x in p[(0x9AC5-0x8000):][:16]))"
# 4) 精灵指针表 / 职员表条数
python3 - <<'EOF'
p=open('rom/Metroid (USA).nes','rb').read()[16:16+128*1024]
o=0x9D5A-0x8000; print('endSamus', ['%04X'%(p[o+2*i]|p[o+2*i+1]<<8) for i in range(6)])
o=0xA291-0x8000; n=0
while True:
    v=p[o+2*n]|p[o+2*n+1]<<8
    if not (0x9C00<=v<0xB000): break
    n+=1
print('credits entries', n)
EOF
```

实跑值：命令 1 → `A0 01 AD 7F 68 D0 0D AD 7E 68 D9 17 CB B0 05 C8 C0 05 D0 EE 8C 72 68 A9 00 C0 04 90 02 A9 01 8D B3 69 60`（35 B）、`agetbl [122, 22, 10, 4]`；命令 2 → `A5 1D D0 23 A5 1E C9 03 D0 1D A6 2D D0 19 FE 7D 68 AD 7D 68 C9 D0 90 0F A9 00 8D 7D 68 E0 03 B0 06 E8 FE 7D 68 F0 F6 60`；命令 3 → `D5 9A 1C 9B 34 9B 93 9B CD 9B FC 9B 39 9A 5C C4`；命令 4 → `endSamus ['9D66','9D9C','9DD2','9E08','9E3E','9E74']`、`credits entries 44`。

## 8. 假设区（C，禁止据此下实现结论）

- **C1**：段 4 `EndFadeOut` 里被 asm 标「does not appear to be used」的 `IsCredits←1`，推测只在某种被裁剪构建里生效；原版是否会走到（与段 5 的早退条件冲突）未验证。
- **C2**：`PaletteDataPending` 在段 1 期间由 `EndGame` 头每 16 帧推进，但段 0 只置 1——首帧到首次推进之间是否有一帧旧调色板，未看 trace。
- **C3**：`$6A00`（卡面提到「$6A00 等结局相关状态」）在 asm 里未见读写；`constants_ram.asm` 未给它名字。按「未取证」处理，不主张它是结局状态。

## 9. 未覆盖

1. `EndGamePalette00–0B` 十三张调色板的**逐字节内容**未逐张 list（只给了指针与地址）。
2. 结局画面**像素级**时间轴（哪一帧出现哪行文本）未做 trace；本文只给逻辑门。
3. `LoadCredits` 的 44 条文本内容未逐条抄录（只给表地址、条数与索引公式）。
4. 段 6 `Restart` 的密码/道具史清零逐字节未展开（属 #70 的道具面）。
5. PAL/MZM 等变体的 `Timer3` 常量（`$38/$30/$08`）未纳入（D6 只做 NTSC）。
6. 结局段（`TitleRoutine=0x1D`）没有任何 L4 采集点/门禁覆盖——`cases/*.txt` 里无结局用例，所以 §6.1 的地址错至今不会让门禁变红。
