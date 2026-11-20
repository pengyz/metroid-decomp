# 结局链（EndGame ; 00:9AA7）现状与施工图

> 取证口径：`refs/m1disasm/SRC/prg0_title.asm`（逐行读，A 级）+ ROM 逐字节自证。
> 本文只写「已证实」与「明确未做」两类，不写推测。

## 1. 触发链（已实现，A 级）

`ElevatorD8BF ; 07:D8BF`（`prg7_engine.asm:4980-4998`）：

```
lda ElevatorType - (Elevator - Objects),x
cmp #$8F            ; 通向结局的电梯
bne @endIf_A
lda #_id_IncrementRoutine.b / sta MainRoutine
inc AtEnding
ldy #$00 / sty RoomPtr
iny / sty BankSwitchPending     ; 切 bank0
lda #_id_EndGame.b / sta TitleRoutine   ; 0x1D
```

Rust 侧已在 `crates/game-core/src/world/elevator.rs` 的 `elev_transition` 里逐项落地
（`MAIN_ROUTINE` / `AT_ENDING` / `ROOM_PTR` / `BANK_SWITCH_PENDING` / `TITLE_ROUTINE`）。

## 2. EndGame 七段（asm 事实）

`EndGame ; 00:9AA7` 每帧头：

- `jsr LoadEndStarSprites`（156 B → `$0270`；段在包内 `title.end_star`）。
- `IsCredits == 0 && FrameCount & $0F == 0` ⇒ `inc PaletteDataPending`，
  到 `_id_EndGamePalette08+1`（= 9）回 `_id_EndGamePalette00+1`（= 1）。
- `lda RoomPtr / jsr JumpEngine` 七项：

| RoomPtr | 例程 | 语义要点 |
|---|---|---|
| 0 | `LoadEndGFX ; 00:9AD5` | `ClearAll` + `InitEndGFX`（`GameMode=1` + `InitGFX6`）+ `EndingType=(JustInBailey?4:0)` + `SpritePointerIndex=EndingType*2` + `PreparePPUProcess(VRAMStruct_EndBackground=$A052)` + `NMIOn` + `SFXMultiInit|=sfxMulti_EndMusic` + `Timer3=$60` + `SpriteByteCounter=$36` + 清 `SpriteAttribByte/ColorCntIndex/IsCredits/EndMsgWrite/HideShowEndMsg/CreditPageNumber` + `PaletteDataPending=_id_EndGamePalette00+1` + `ClrChangeCounter=$08` + `inc RoomPtr` + `ScreenOn` |
| 1 | `ShowEndSamus ; 00:9B1C` | `LoadEndSamusSprites`；`Timer3==0` ⇒ `inc RoomPtr`；`Timer3==$50` ⇒ `EndMsgWrite=1`；`Timer3==$01` ⇒ `HideShowEndMsg=1` |
| 2 | `EndSamusFlash ; 00:9B34` | `FrameCount&$1F==0` ⇒ `inc ColorCntIndex`；`==8` ⇒ `ChooseEnding($CAF5)`+`CalculatePassword($8C7A)`+`SpritePointerIndex=EndingType*2`+`SpriteByteCounter=$36`；`==$10` ⇒ `Timer3=$10`、`SpriteAttribByte=(EndingType>=$04)`、`inc RoomPtr`、`EraseAllSprites`；尾部 `dec ClrChangeCounter`（到 0 取 `PaletteChangeTable[ColorCntIndex]` = `08 07 06 05 04 03 02 01 01 02 03 04 05 06 07 08`，并 `inc SpriteAttribByte`，到 3 归 0）；尾 `LoadEndSamusSprites` |
| 3 | `SamusWave ; 00:9B93` | `Timer3!=0` ⇒ 套装/无套装：`EndingType>=4` ⇒ `WaveSpritePointer=(EndingType-4)*4`、`FrameCount&$08` 决定手型 ±2、`SpriteByteCounter=$2D`、`LoadWaveSprites`；`Timer3==0` ⇒ `Timer3=$10`、`PaletteDataPending=_id_EndGamePalette07+1`、`inc RoomPtr` |
| 4 | `EndFadeOut ; 00:9BCD` | `Timer3==0` ⇒（`IsCredits==0` ⇒ `PaletteDataPending=_id_EndGamePalette07+1`、`inc IsCredits`）；`FrameCount&$07==0` ⇒ `inc PaletteDataPending`，到 `_id_EndGamePalette0B+1`（= 0x0C）⇒ `Timer3=$10`、`inc RoomPtr`；尾部按 `EndingType` 选 `LoadEndSamusSprites`/`LoadWaveSprites` |
| 5 | `RollCredits ; 00:9BFC` | `Timer3==2` ⇒ `ScreenOff`+`ClearNameTables@nameTable0`+`EraseAllSprites`+`PaletteDataPending=_id_EndGamePalette0B+1+1`+`ScreenOn`+`WaitNMIPass_`；`CreditPageNumber==0` ⇒ `inc`；`==6 && ScrollY>=$88` ⇒ `inc RoomPtr`；否则每 4 帧滚一行（`CreditsPtrTable` 六页 + `ScrollY`） |
| 6 | `Restart ; 00:9A39` | 重开（`TitleRoutine` 回起始 + `ClearAll` 族） |

`PaletteChangeTable ; 00:9B83` = `08 07 06 05 04 03 02 01 01 02 03 04 05 06 07 08`。

## 3. 数据面（已入包，A 级）

| 段 | 来源 | 长度 |
|---|---|---|
| `title.end_star` | `EndStar ; 00:9EB8` | 156 B |
| `title.end_background` | `VRAMStruct_EndBackground ; 00:A052` | 360 B（17 记录 + 终止） |
| `title.end_palettes` | `EndGamePalette00..0A ; 00:9F9B` | 205 B（11×u16 偏移 + 183 B 页体，长度 36/12×8/27/21） |
| `title.end_credits` | `CreditsPtrTable ; 00:A291` 六指针 | 114 B（6×u16 + 102 B；页长 18/31/1/18/12/22，页 2 只有终止符） |
| `title.end_samus` | `EndSamusAddrTbl ; 00:9D5A` 六指针 | 350 B（6×u16 + 338 B；每帧 54 B） |
| `title.end_wave` | `WavePointerTable ; 00:9D12` 四表 | 64 B（每表 4 精灵 × 4 B） |

`assets.rs` 访问器：`end_background()` / `end_palette(n)` / `end_credits(page)` /
`end_samus(idx)` / `end_wave(table)`。`prepare_vram` 已有 `0xA052` 臂。

## 4. Rust 侧现状（**未完成**，如实记录）

`crates/game-core/src/title/mod.rs` 的 `0x1D` 臂当前只实现「每帧头」：

- 装载 `title_end_star()` 156 B；
- 调色板循环（`IsCredits==0 && FC&0x0F==0` ⇒ `inc $1C`，到 9 回 1）；
- **然后按 `RoomPtr` 直接 `jumped = Some(目标)`**，`targets = [0x9AD5, 0x9B1C, 0x9B34, 0x9B93, 0x9BCD]`
  —— 只有 5 项，段 5（`RollCredits` = `0x9BFC`）与段 6（`Restart` = `0x9A39`）**连目标都没有**。

⇒ 七段例程体一个都没执行：结局流程会停在段 0 的入口。

## 5. 施工顺序（下一步）

1. **批 A（段 0–2）**：段 0 需要 `ClearAll` 三段自旋（复用 `title::clear_all_seg0_events` /
   `clear_all_seg1_events` + `clear_all_seg2_events`）、`ShellCall::PrepareVram(0xA052)`、
   `NMIOn`、`ScreenOn`，以及 `Timer3=$60` / 计数器清零 / `$1C←1` / `inc $33`；
   段 1 只需 `Timer3` 三阈值 + 两个标志位；段 2 需 `FrameCount&$1F` 门 + `ColorCntIndex`
   到 8 时调 `ChooseEnding`（`$CAF5`，尚未端口）+ `CalculatePassword`（`$8C7A`，已有密码链）
   + `PaletteChangeTable` 闪色 + `LoadEndSamusSprites`（用 `assets::end_samus`）。
2. **批 B（段 3–6）**：挥手四表、淡出（`$1C` 到 0x0C）、职员表滚动（`assets::end_credits`
   六页 + `ScrollY`）、`Restart`。
3. 段 2 的 `ChooseEnding ; 00:CAF5` 是独立未端口件（按 `EndingType` 选 0–5，取决于
   通关时间/道具），需要在批 A 之前或同期取证。

## 6. 诚实边界

- 目前**没有任何像素证据**：结局只能从 `$8F` 电梯进入 Tourian 通关后触发，现有 20 个 L4
  用例无一走到 Tourian，房扫对非 0 区结构性不可用（`room-sweep.md` §8.1）。
- 段 5/6 的目标地址缺失说明这段代码从未被执行过——不是「行为等价但未验证」，
  而是「行为缺失」。
