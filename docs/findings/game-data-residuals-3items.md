# findings：game-data 残余三件收尾——gfx 归属/EnPlace 语义/items type 自证（task-26）

> 状态：**三件已考证**（2026-10-16，五环节奏：asm 数据段+双工具+A/B/C 分级）。
> 基线：task-17 r1-r6（[game-data-brinstar-sample.md](game-data-brinstar-sample.md)）。

## 1. ① gfx 块↔区域归属（A 级调用面证据）

**asm 锚**（prg7_engine.asm:1480-1620）：区域 gfx 加载=Init*GFX 例程族 ×
`ldy #_id_GFX_*.b / jsr LoadGFX` 调用序列（_id_*=GFXInfo 表自动编号，macros.asm:101）。
**归属表**（B 级语义=A 级调用面+例程名线索）：

| Init 例程（址） | 加载集（_id_GFX_*） |
|---|---|
| InitTitleGFX ($C5D7) | Title, Samus(+Suitless 变体), IntroSprites, Font_Complete, Font_Hud, Solid_0FC0, Solid_1FC0 |
| InitBrinstarGFX ($C604) | BrinBG1, CREBG1, CREBG2, BrinstarSprites, Solid_0FC0, Solid_1FC0 |
| InitNorfairGFX ($C622) | CREBG1, CREBG2, NorfBG1, NorfBG2, NorfairSprites, Solid×2 |
| InitTourianGFX ($C645) | CREBG2, BossBG, TourBG, Zebetite, TourianFont, TourianSprites, Font_Tourian, ExclamationPoint, Solid×2 |
| InitKraidGFX ($C677) | CREBG1, CREBG2, BossBG, KraiBG2, KraiBG3, KraidSprites, Solid×2 |
| InitRidleyGFX ($C69F) | （同族，尾段同构） |
| InitEndGFX ($C5D0) | 经 InitGFX6（结束画面族） |

- **公共块**（CREBG1/CREBG2/Solid_0FC0/Solid_1FC0=跨区共享——r5 「共享」语义实证
  的 gfx 面版本）；BossBG=Tourian/Kraid 共享（Boss 区族）；
- JustInBailey($…)=服装变体开关（Samus/SamusSuitless 分支，A 级调用面）；
- **判定**：24 项 GFXInfo 全归属落位（表↔调用面双源）——**A 级定谳**（归属语义 B）。

## 2. ② EnPlace 语义（帧体残留收口——AABBXXXX 控制字节自证）

**asm 锚**（prg7_engine.asm:5883-5890 注释块+GetSpriteCntrlData $DCC3 指令流）：
- **帧体字节 0=精灵控制字节 AABBXXXX**（A 级注释锚+代码结构互核）：
  **AA**=H/V 镜像位、**BB**=色位、**XXXX=PlacePtrTbl 条目号**（放置段索引）；
- 解码流（$DCC3，逐字节 A 级）：`lda (FramePtr),y` → flip/colour/priority 提取
  （OAMDATA_HFLIP/VFLIP/PRIORITY 掩码）+ ObjectCntrl($…) 镜像覆盖 →
  **`and #$0F / asl / tax` = 放置条目×2**（词表步长 2）→ `(EnPlacePtrTable),y`
  取段指针（$45=EnPlacePtrTable RAM 指针，constants_ram:213）；
- **放置段=有符号偏移数组**（EnPlaceB 14B=`F8 FC 00 F8 F4 F4 FC F4 00 00 F4 04 FC 04`
  =-8/-4/0/-12/+4 族有符号像素偏移；EnPlaceC-F 共享 4B 段=「共享尾」gfx 面同款）；
- ObjFrame/EnFrame **同构**（common_sprite_data.asm ObjFrame_SamusRun2 同 AABBXXXX+
  tile 序+$FD/$FE/$FF 控制标记——tile 序控制字节族：$FD=OAM 属性切换/$FF=帧终；
  $FE 语义 C 级待考）。
**判定**：控制字节格式+索引机制 **A 级**；偏移段布局（X/Y 对）**B 级**（数值符号族+
消费点推断）；$FE 控制标记语义 C 级留。

## 3. ③ items type $01-$0A 自证（值域→语义升级 C→**B**）

**双源互证**（关键自证：分发结构=独立于注释的证据）：
- 源 1（items.asm:45-54 注释型表）；
- 源 2（nmikstas Bank07.asm:7216-7226 **type 分发跳转表**，经 C27C ChooseRoutine
  ——`and #$0F / jsr ChooseRoutine` @LEDDD）：

| type | 分发表条目（址） | handler（体址） | 语义（自证级） |
|---|---|---|---|
| $00 | LEDE2 | ExitSub ($C45C) | 空操作（共享 RTS） |
| $01 | LEDE4 | SqueeptHandler ($EDF8) | Squeept 敌（体=GetEnemyData） |
| $02 | LEDE6 | PowerUpHandler ($EDFE) | 能量块/道具 |
| $03 | LEDE8 | SpecEnemyHandler ($EE63) | 特殊敌（Mellows/Melias/Memus 族） |
| $04 | LEDEA | ElevatorHandler ($EEA1) | 电梯 |
| $05 | LEDEC | CannonHandler ($EEA6) | MB 房炮 |
| $06 | LEDEE | MotherBrainHandler ($EEAE) | Mother Brain |
| $07 | LEDF0 | ZeebetiteHandler ($EECA) | Zeebetite |
| $08 | LEDF2 | RinkaHandler ($EEEE) | Rinka |
| $09 | LEDF4 | DoorHandler ($EEF4) | 门 |
| $0A | LEDF6 | PaletteHandler ($EEFA) | 背景调色板切换 |

- **自证判定**：type 值域→handler 体地址映射=**B 级语义定谳**（代码语境+调用图：
  跳转表结构独立于注释且 handler 体行为可溯）；items.asm 注释型表与分发表
  **逐条吻合**（双源互证）；
- **台账候选**（D5：名=第三方，语义已自证 B——可按 rename 流程走）：
  type 语义映射表本身可入 spec（值→行为）；handler 名照抄需台账逐条登记。

## 4. 分级汇总

| 结论 | 级 |
|---|---|
| ①24 项 gfx 归属（调用面×表） | **A**（证据）/归属语义 **B** |
| ②AABBXXXX 控制字节+PlacePtrTbl 索引机制 | **A** |
| ②放置段=有符号偏移数组 | **B** |
| ③type→handler 体映射（11 词） | **B**（双源互证定谳） |
| $FE 控制标记/Ridley 尾段细节 | **C**（留） |

## 5. 遗留

1. $FE 控制标记语义（帧 tile 序控制字节族）；
2. 台账候选三条走 rename 流程（gfx 归属名/EnPlace 名/type 语义名——需台账逐条）；
3. EnPlace 偏移段 X/Y 对布局定案（需消费环逐字节）。
