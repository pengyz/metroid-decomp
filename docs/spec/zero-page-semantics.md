# 零页 `$0034-$00FD` 语义与 typed/ephemeral 分档（B4 取证）

> 任务 #100。**范围**：`$0034-$00FD` 共 202 B 的零页变量面。
> **证据级**：符号/布局/写点均有 asm 行或字节级锚点（A/B），推断处单列 C 级。
> **样本快照**：`analysis/traces/*.ram.hex` 共 11116 份（2026-11-26 采集；数与日期绑定，重跑会漂）。

## 0. 口径与两处卡面更正

### 0.1 批次名：本文按范围论证，不按 `typed-struct-plan.md` 的批次号
`docs/design/typed-struct-plan.md` 里 **B3 = 敌人槽**（`Enemy` + `Explosion` + `$0317-$03FF`，见该文件 `:159-166`），零页在计划里属 **B4**（`$0000-$0032` + `$0108-$01FF`）。卡面标题写「B3 取证：零页」与计划的批次划分不一致；本文按**范围**（零页 `$0034-$00FD`）交付，批次号以计划为准。

### 0.2 `$0034-$003D` 不是 SpareMem（A 级更正）
`refs/m1disasm/SRC/constants_ram.asm:528-545`：

```
RoomPtr                dw   ;$33     ;Low byte of pointer to room data.
; RoomPtr+1                  $34     ;High byte of pointer to room data.
StructPtr              dw   ;$35
RoomRAMWorkPtr         dw   ;$37
RoomRAMPtr             dw   ;$39
AreaPointers_RAM       instanceof AreaPointersStruct ;$3B-$48
ScrollDir              db   ;$49     ;0=Up, 1=Down, 2=Left, 3=Right.
```

即 `$34` 是 `RoomPtr` 的高字节，`$35-$48` 是 3 个 dw + 7 组区指针表，`$49` 是滚动方向。**真正的 SpareMem** 是 `$21/$26/$30/$32`（已由 `memmap/zero_page_face.rs` 覆盖）与 `$75/$78/$89/$B0/$B7/$B8/$BB/$BE/$C5/$C9/$CB-$CF/$D0-$D3/$D7/$D9/$E8/$EE/$F1-$F9/$FB`。

### 0.3 `$006E/$006F`
`HealthChange dw`（`constants_ram.asm:618`）。#77 已建模，本卡只登记归属，不重复建模；它在本文档里仍计入「可 typed」字节数。

## 1. 命名真源

零页 **唯一命名真源**是 `refs/m1disasm/SRC/constants_ram.asm:483-822`：`.enum $12 export` … `.ende`，`$12-$FF` 每字节都有 asm 名，含 3 个 `.union` 块（4 个 `.nextu` 入口）的别名段。

- 地址定法：asm 行尾注释 `;$XX`（如 `RoomRAMPtr dw ;$39`），**不使用** enum 顺序累加——union 段会回跳，累加会错位。
- 解析结果 `analysis/zp/zp_syms.json`：结束地址正好 `$100`（256 B 全覆盖）；脚本报 `aliases-backjumps 8`——即 8 处地址回跳/别名（`constants_ram.asm` 里 3 个 `.union` 块、4 个 `.nextu` 入口产生别名窗口），按「同址多别名」处理而不是当错位。
- 结构体（`constants_ram.asm`）：
  - `AreaPointersStruct`（`:206-215`）：`$3B RoomPtrTable / $3D StructPtrTable / $3F MetatileDefs / $41 EnFramePtrTable1 / $43 EnFramePtrTable2 / $45 EnPlacePtrTable / $47 EnAnimTable`，各 dw。
  - `SkreeProjectile`：`dieDelay/y/x/hi`，4 B × 4 槽 = `$A0-$AF`。
  - `Mellow`：`status/y/x/hi/attackState/attackTimer/isHit/unused`，8 B × 4 槽 = `$B0-$CF`。
  - `RinkaSpawner`：`status/hi/posIndex`，3 B × 2 槽 = `$8B-$90`。

## 2. 普查方法与样本

三条互相独立的观测面（都可用仓库内脚本复跑，脚本产物在 `analysis/`，不入库）：

| 面 | 脚本 → 产物 | 口径与局限 |
|---|---|---|
| 静态指令级 | `analysis/zp/census2.py` → `analysis/zp/census2.json` | 扫 `refs/m1disasm/SRC/*.asm` 的指令行，按 `(mnemonic, 操作数基址)` 归类 R/W/RMW；`areas_common`/`enemies` 各 include 5 次 ⇒ 一条 asm 行 = bank1–5 各 1 物理副本。结果 **2344 次访问 / 177 个地址**。地址精度=**所在例程标签**(bank:addr)，不是指令精确地址（见 §2.2）。 |
| seed 地址对齐 | `analysis/zp/seed_map.py` → `analysis/zp/seed_sites.json` | 用 `(mnem, 地址)` 对与 SRC 求交，得 **982 条** `bank:addr` 精确样例，仅用于逐段表「代表地址」列。 |
| 运行期采集点 | `analysis/zp/runtime.py` → `analysis/zp/runtime.txt` | 11116 份采集点快照，按文件名前缀+帧号分组算「非零率 nz / 帧间变化率 chg / 取值种数」。**采集点是帧边界快照**（`tools/scenario_run.py:36` 记「采集点全量 2KB RAM」），不是每帧全过程；某变量在该相位为 0 **不能**推「零写」，判定以静态面为准。 |

### 2.1 高字节访问
74 行以 `Name+1` 形式访问 dw 高字节（`grep -rnE "(RoomPtr|StructPtr|RoomRAMWorkPtr|RoomRAMPtr|AreaPointers_RAM\.[A-Za-z0-9]+)\s*\+\s*1"`，如 `prg7_engine.asm:8731 sta RoomRAMWorkPtr+1`、`:8750 inc RoomRAMWorkPtr+1`）⇒ 指针组必须按 **16 位字段**声明，只建低字节会漏掉一半写点。

### 2.2 地址精度说明
SRC 行级地址重建（`analysis/zp/addrs2.py`）在 8707 个标签里有 656 个漂移（多为 IDA 把代码当数据的区、宏展开差），故**本文的 bank:addr 只有两种来源**：① seed 对齐出的精确地址（982 条）；② 指令所在**例程标签**地址（`census2.json` 的 `sites[].routine`）。两者在表里可区分：精确地址写作 `bank07:E21F STX $6C`，例程级写作 `07:EB92`（= 该行所在例程入口）。

### 2.3 `$34-$FD` 内静态零访问的 61 字节（逐条）
`$5E $5F $60 $61 $62 $63 $78 $86 $89 $8E $8F $90 $A4 $A5 $A6 $A7 $A8 $A9 $AA $AB $AC $AD $AE $AF $B9 $BA $BE $CA $D4 $D5 $D6 $D9 $DA $DB $DC $DD $DE $DF $E8 $E9 $EE $EF $F1 $F2 $F3 $F4 $F5 $F6 $F7 $F8 $F9 $FB`

分三类（不是「都没用」）：
- **结构体槽 1–3 的索引访问面**：`$5E-$63`（`DoorRoomRAMPtr dsw 4`，`:593` 起 4 个 word：只有槽 0 的低字节 `$5C` 被直接寻址，槽 0 高字节 `$5D` 与槽 1–3 的 `$5E-$63` 全走索引寻址——`areas_common.asm:1737 sta DoorRoomRAMPtr,y` / `:1739 sta DoorRoomRAMPtr+1,y`、`prg7_engine.asm:8250 eor DoorRoomRAMPtr+1,x`）、`$8E-$90`（RinkaSpawners 槽 1，基址仍是 `$8B`）、`$A4-$AF`（SkreeProjectiles 槽 1–3，`:737` 的 `instanceof ... 4 startfrom 0`，全部经 `LDA SkreeProjectiles,x`，x=4/8/12 访问）、`$B1-$CF` 非首槽字段（Mellow 同理：`:25` 结构体 8 B，`:741` 的 `instanceof Mellow 4`）⇒ 字节「零直接访问」≠ 语义未用，**必须按结构体整段 typed**。
- **dw 高字节**：`$86`（`EnemyFlipAfterDisplacementAnimIndex+1`）等。
- **真 spare/未用**：见 §5 的 U 类。

## 3. 逐段表（`$0033-$00FD` 共 203 B；卡面范围 `$0034-$00FD` 为 202 B，无缺口）

类：**P**=可 typed 的指针/dw；**S**=可 typed 的标量或位域；**E**=只能 ephemeral（逐帧/逐调用派生或游标）；**U**=不可声明（SpareMem / 零访问 / 仅 title 写）。表由脚本从 `zp_syms.json`+`census2.json`+`runtime.txt` 生成，未手抄。

| 字节 | asm 名/语义 | 类 | 静态 R/W/RMW | 采集点 nz/chg（段首字节） | 代表 bank:addr |
|---|---|---|---|---|---|
| `$33-$34` | RoomPtr（dw：$33 低/$34 高） | P | 8/4/8 | — | `bank07:EA4A STA $34` |
| `$35-$36` | StructPtr dw | P | 3/3/2 | 96/15 | `bank07:EA9B STA $35` |
| `$37-$38` | RoomRAMWorkPtr dw | P | 1/5/7 | 96/10 | `bank07:EA64 STA $37` |
| `$39-$3A` | RoomRAMPtr dw | P | 5/2/0 | 0/0 | `bank07:C87B LDY $39` |
| `$3B-$48` | AreaPointers_RAM：7×dw（:206-215） | P | 12/0/0 | 96/0 | `bank07:EA43 LDA ($3B),Y` |
| `$49` | ScrollDir：0上/1下/2左/3右 | S | 41/4/9 | 87/14 | `bank00:8201 STA $49` |
| `$4A` | TempScrollDir | S | 2/1/0 | 96/17 | `bank07:E57E CPX $4A` |
| `$4B` | PageIndex：对象循环游标 | E | 88/29/0 | 94/1 | `bank00:92E1 STY $4B` |
| `$4C` | ItemIndex：0/8 | S | 3/1/0 | 94/0 | `bank07:DB42 STX $4C` |
| `$4D` | SamusDir：0右/1左 | S | 12/6/1 | 2/1 | `bank00:8035 STA ($4D,X)` |
| `$4E` | SamusDoorDir | S | 1/1/0 | 0/0 | `bank07:E362 STA $4E` |
| `$4F-$50` | MapPosY/MapPosX | S | 11/2/15 | 96/0 | `bank07:C85E STA $4F` |
| `$51-$52` | SamusScrX/Y：每帧由世界坐标导出 | E | 4/2/4 | 54/45 | `bank07:E323 STA $51` |
| `$53` | WalkSoundDelay（#89 已核：$53 非 $010F） | S | 0/2/1 | 11/5 | `bank07:CC9A STA $53` |
| `$54` | StatuesBridgeIsSpawned | S | 1/2/0 | 0/0 | `bank07:DADA LDA $54` |
| `$55` | IsSamus | S | 4/2/1 | 0/0 | `bank00:82AB STA $55` |
| `$56` | DoorEntryStatus：0-5 + MSB=入过门 | S | 8/5/0 | 73/13 | `bank01:8B76 STA $56` |
| `$57` | DoorScrollStatus：1-4 | S | 1/1/0 | 0/0 | `bank01:8B54 STA $57` |
| `$58` | SamusDoorData：高半字节滚动模式/低半字节动作 | S | 6/6/2 | 0/0 | `bank01:8B17 LDY $58` |
| `$59` | DoorDelay | S | 0/2/1 | 9/4 | `bank00:805B STA ($59),Y` |
| `$5A` | RoomNumber：$FF=无请求 | S | 6/5/1 | 95/26 | `bank07:C867 STA $5A` |
| `$5B` | SpritePagePos： sprite RAM 装载游标 | E | 8/7/0 | 95/48 | `bank00:81F4 STA $5B` |
| `$5C-$63` | DoorRoomRAMPtr[4] 4×dw：门槽房间 RAM 指针 | P | 0/2/2 | 0/0 | `bank01:8D31 STA $5C,Y` |
| `$64` | SamusInLava | S | 6/1/0 | 0/0 | `bank07:CD0E LDA $64` |
| `$65` | ObjectCounter | S | 2/6/6 | 39/8 | `bank00:84F8 ADC ($65,X)` |
| `$66` | EnemyMovePixelQty | S | 0/8/4 | 0/0 | `bank01:8071 STA $66` |
| `$67-$68` | ObjectPalette/RoomPalette：派生 attrib | E | 4/2/0 | 92/11 | `bank07:EA94 STA $67` |
| `$69-$6A` | TempX/TempY | E | 2/2/0 | 99/1 | `bank07:C27F STX $69` |
| `$6B` | ObjectCntrl：bit7 有效/bit5 优先/bit4 镜像/bit1-0 调色板 | S | 7/15/8 | 0/0 | `bank00:92E3 STY $6B` |
| `$6C-$6D` | ScrollBlockOnNameTable3/0：bit0 左门/bit1 右门 | S | 0/8/4 | 28/8 | `bank01:8B32 STA $6C,Y` |
| `$6E-$6F` | HealthChange dw（#77 已建模，只登记归属） | S | 5/12/4 | 0/0 | `bank00:803D STA ($6E,X)` |
| `$70` | SamusInvincibleDelay | S | 5/3/1 | 9/9 | `bank07:C8E3 STX $70` |
| `$71` | UpdatingSamusProjectile | S | 4/4/1 | 0/0 | `bank07:C82D STX $71` |
| `$72` | SamusKnockbackDir：0左/1右/$FF 不推 | S | 1/3/0 | 46/0 | `bank07:CE07 STA $72` |
| `$73` | SamusKnockbackIsBomb：bit7 炸弹/bit0 垂直 | S | 1/2/0 | 94/0 | `bank07:CE09 LDA $73` |
| `$74` | InArea：$10-$14 | S | 13/3/0 | 45/0 | `bank00:8C9E LDA $74` |
| `$75` | SpareMem75（AreaInit 置 $FF，从不读） | U | 0/1/0 | 96/0 | `bank07:C823 STX $75` |
| `$76` | PaletteToggle | S | 2/3/0 | 96/0 | `bank07:C863 STA $76` |
| `$77` | SamusKnockbackIsBomb77 | S | 1/1/0 | 50/0 | `bank07:CE0B STA $77` |
| `$78` | SpareMem78（零访问） | U | 0/0/0 | 0/0 | `（seed 未解码；见 census2.json）` |
| `$79` | ItemRoomMusicStatus：0/1/$80/$81 | S | 4/6/0 | 0/0 | `bank07:C9E1 STA $79` |
| `$7A-$7B` | union：SpareMem7A7B（游戏期未用）/EndMsgWrite+IsCredits（结局期） | S | 3/3/2 | 0/0 | `bank00:9A10 LDA $7A` |
| `$7C-$7E` | union：SpriteByteCounter等（结局）/Boss 计数（游戏） | S | 7/9/2 | 0/0 | `bank00:9AFF STA $7C` |
| `$7F-$80` | union：ColorCntIndex（结局）/KraidNailCounter、RidleyFireballCounter | S | 5/2/3 | 0/0 | `bank00:9B05 STA $7F` |
| `$81-$82` | union：EnemyMovementPtr dw/EnemyStatusPreAI；HideShowEndMsg、ClrChangeCounter | S | 10/7/5 | 27/12 | `bank00:9A24 LDA $81` |
| `$83-$86` | union：SpawnEnProjectile_AnimIndex dw / EnemyFlipAfterDisplacementAnimIndex dw / WaveSprite 指针与计数 | S | 6/3/2 | 0/0 | `bank00:9BB0 STA $83` |
| `$87-$88` | SpawnEnProjectile_ExpectedStatus / EnData0A（asm 注：该功能未使用，期望状态恒等于当前状态） | S | 4/0/0 | 1/0 | `（seed 未解码；见 census2.json）` |
| `$89` | SpareMem89（零访问） | U | 0/0/0 | 0/0 | `（seed 未解码；见 census2.json）` |
| `$8A` | MellowRandomNumber | S | 0/3/4 | 0/0 | `bank01:86A3 ROL $8A` |
| `$8B-$90` | RinkaSpawners[2] 2×3（status/hi/posIndex） | S | 8/6/2 | 96/0 | `bank07:C8F8 STX $8B` |
| `$91` | DoorPaletteChangeDir | S | 1/3/1 | 0/0 | `bank07:C8A4 STX $91` |
| `$92` | MetroidOnSamus | S | 3/3/0 | 0/0 | `bank07:C8CC STX $92` |
| `$93-$96` | 导弹/能量掉落上限与当前计数 | S | 4/10/2 | 96/0 | `bank01:8B23 STA $93` |
| `$97` | CannonIndex | S | 3/2/0 | 0/0 | `bank07:C8C2 STA $97,X` |
| `$98` | MotherBrainStatus：0-$0A | S | 8/12/1 | 0/0 | `bank07:C96E LDA $98` |
| `$99` | union：QtyHits（活）/DeathStringID（消散）/TimeBombCounter（死） | S | 4/4/5 | 0/0 | `（seed 未解码；见 census2.json）` |
| `$9A` | union：AnimBrainDelay / DeathInstrID | S | 0/3/4 | 0/0 | `（seed 未解码；见 census2.json）` |
| `$9B` | MotherBrainAnimEyeDelay：bit7=眼开 | S | 2/2/2 | 0/0 | `（seed 未解码；见 census2.json）` |
| `$9C` | MotherBrainAnimFrameTableID | S | 1/1/0 | 0/0 | `（seed 未解码；见 census2.json）` |
| `$9D` | MotherBrainHi | S | 8/1/1 | 0/0 | `（seed 未解码；见 census2.json）` |
| `$9E` | MotherBrainIsHit | S | 1/2/0 | 0/0 | `（seed 未解码；见 census2.json）` |
| `$9F` | MotherBrainFlashDelay | S | 1/3/2 | 0/0 | `bank06:84F4 STA $9F,X` |
| `$A0-$AF` | SkreeProjectiles[4] 4×4（dieDelay/y/x/hi） | S | 5/4/1 | 0/0 | `bank07:FBEC LDA $A0,X` |
| `$B0-$CF` | Mellows[4] 4×8（status/y/x/hi/attackState/attackTimer/isHit/unused） | S | 36/71/11 | 0/0 | `bank07:ECCD STA $B0,X` |
| `$D0-$D7` | SpareMemD0-D7（title 写、从不访问） | U | 0/8/0 | 0/0 | `bank00:807D STY $D0` |
| `$D8` | IntroMusicRestart | S | 1/2/1 | 3/0 | `bank00:808C STA $D8` |
| `$D9-$DF` | SpareMemD9 ds7（零访问） | U | 0/0/0 | 0/0 | `（seed 未解码；见 census2.json）` |
| `$E0-$E1` | APURegisterPtr/SoundRoutineTablePtr dw | S | 2/4/0 | 99/0 | `（seed 未解码；见 census2.json）` |
| `$E2-$E3` | SFXDataPtr/SoundRoutinePtr dw | S | 1/4/0 | 99/0 | `（seed 未解码；见 census2.json）` |
| `$E4-$E5` | GetSoundRoutineDataPtr dw | S | 2/2/0 | 99/0 | `（seed 未解码；见 census2.json）` |
| `$E6-$E7` | MusicChannelBase dw | S | 2/2/0 | 87/0 | `（seed 未解码；见 census2.json）` |
| `$E8-$E9` | SpareMemE8 dw | U | 0/0/0 | 0/0 | `（seed 未解码；见 census2.json）` |
| `$EA` | Cntrl0Data | S | 2/4/1 | 99/62 | `（seed 未解码；见 census2.json）` |
| `$EB` | VolumeEnvelopeIndex | S | 0/1/1 | 0/0 | `bank07:D4E8 DEC $EB,X` |
| `$EC-$ED` | VolumeEnvelopePtr dw | S | 1/2/0 | 99/1 | `（seed 未解码；见 census2.json）` |
| `$EE-$EF` | SpareMemEE dw | U | 0/0/0 | 0/0 | `（seed 未解码；见 census2.json）` |
| `$F0` | ABStatus（AreaInit 存 A/B，从不读） | S | 0/2/0 | 5/0 | `bank00:80D2 STA $F0` |
| `$F1-$F9` | SpareMemF1 ds9 | U | 0/0/0 | 0/0 | `（seed 未解码；见 census2.json）` |
| `$FA` | MirrorCntrl：bit3=1 水平镜像 | S | 4/5/0 | 99/5 | `bank07:C097 STA $FA` |
| `$FB` | SpareMemFB | U | 0/0/0 | 0/0 | `（seed 未解码；见 census2.json）` |
| `$FC-$FD` | ScrollY/ScrollX：PPU 滚动寄存器值 | S | 37/9/11 | 0/0 | `bank00:90B0 STA $FC` |

> 注：表中比卡面多 1 行字节——`$33-$34` 的 `$33` 在卡面范围外（`$0033` 是 `RoomPtr` 低字节，现为 Unmodeled 1 B）。本卡建议**顺带纳入 1 B**，否则 dw 只能建半截（见 §6）。

## 4. 位域布局（5 段，逐字节可复核）

### 4.1 `$58 SamusDoorData`（1 B，两段位域）
asm 自述（`constants_ram.asm:581-587`，摘录）：

```
SamusDoorData db ;$58  ;The upper 4 bits store either 1 or 2. If 1 is stored(bit 4
                       ;set), the scrolling after Samus exits the door is toggled.
                       ;If 2 is stored(bit 5 set), the scrolling is set to
                       ;horizontal scrolling after Samus exits the door.
                       ;The lower 4 bits store Samus' action status as she enters the door.
```

| 位 | 宽 | 语义 | 证据 |
|---|---|---|---|
| 7-5 | 3 | 恒 0（asm：upper 4 bits only 1 or 2） | asm 注释（`:581-587`） |
| 5 | 1 | 出门后强制**水平**滚动（多为道具房） | asm 注释 |
| 4 | 1 | 出门后**切换**滚动方向 | asm 注释 + `DoOneDoorScroll 07:E22C` 的 `lda SamusDoorData / jsr Amul8`（`prg7_engine.asm:7052-7053`） |
| 3-0 | 4 | 进门时的 Samus 动作状态（0–4，用于出门后保持外观） | `constants_ram.asm:585-587` 注释（`Samus' action status as she enters the door`） |

写点：`07:C81D stx`（进房初始化）、`07:E663/07:E6A2 sta`（门流程）、`07:E800 sty/inc`（动作推进），seed 精确 `bank01:8B17 LDY $58`、`bank01:8B65 STA $58`。采集点快照 `$58` 恒 0（帧边界相位），不能据此判未用——静态面 6 读/6 写/2 RMW（§3 表）。

### 4.2 `$6B ObjectCntrl`（1 B，4 个位域 + 无效位）
asm 自述（`constants_ram.asm:603-607`）：

```
ObjectCntrl db ;$6B  ;Controls object properties such as mirroring and color bits.
                     ;Bit 4 controls object horizontal mirroring.
                     ;If bit 7 set, these attributes apply:
                     ;bit 5 is priority
                     ;bit 0 and bit 1 is for the color palette
```

| 位 | 语义 | 代码锚点 |
|---|---|---|
| 7 | 属性有效（MSB） | `prg7_engine.asm:5552-5555`：`and #$03`（:5552）/ `ora #$80`（:5554）/ `sta ObjectCntrl`（:5555）（例程 `07:DB42`） |
| 5 | 优先级 | 注释；`asl ObjectCntrl`（`:5925`，例程 `07:DCC3`）把它送进位 → OAM 属性 bit5 |
| 4 | 水平镜像 | `:6241-6242` `and #~(OAMDATA_HFLIP>>2) / sta ObjectCntrl`（例程 `07:DE4A`，清镜像位给电梯/载具） |
| 1-0 | 调色板（0–3） | `:5552 and #$03` |
| 6,3,2 | 未定义/清零 | 无读点依赖 |

采集点取值实测：2 种，`$00`×11112、`$B1`×4。`$B1 = 1011_0001` 恰好 bit7+bit5+bit4+bit0 全置 ⇒ 与位域表一致。

### 4.3 `$6C/$6D ScrollBlockOnNameTable3/0`（各 1 B，2 个门标志位）
asm 自述（`constants_ram.asm:609-616`）：`bit 0: left door on name table`；`bit 1: right door on name table`。

写点：`seed bank07:C84C STX $6C`、`bank07:C84E STX $6D`（精确）、`07:E21B`、`bank01:8B32 STA $6C,Y`（门槽循环）、`07:EB92 ora/sta ScrollBlockOnNameTable3,x`。

采集点取值实测（强证据）：
- `$6C`：4 种值 `{0,1,2,3}`（top：0×7972、1×1356、2×1062）——正好 bit0/bit1 的 4 种组合，无其它位。
- `$6D`：2 种值 `{0,2}`（0×10360、2×756）——只出现 bit1（右门），与「NT0 只挂右门」一致。

### 4.4 `$56 DoorEntryStatus`（1 B，枚举 + MSB 标志）
asm 自述（`constants_ram.asm:571-580`）：`0=Not in door, 1=In right door, 2=In left door, 3=Scroll up, 4=Scroll down, 5=Exit door, MSB set=Door entered`。

| 值 | 语义 |
|---|---|
| 0 | 不在门里 |
| 1 / 2 | 在右门 / 左门 |
| 3 / 4 | 进竖向井后需上/下滚屏 |
| 5 | 出门 |
| bit7 | 「由门进入」（与低 7 位并存） |

代码：`ScrollDoor 07:E1F1` 的 `ldx DoorEntryStatus`（`prg7_engine.asm:7007`）、`DoOneDoorScroll 07:E22C` 的 `stx`（`:7066`）、`areas_common.asm:1322/1411/1519/1607/1658/1688`（seed 精确 `bank01:8B76 STA $56`、`bank01:8C01 LDA $56` 等）。采集点 5 种值，top `$01`×7164、`$00`×2942、`$05`×1002。

### 4.5 `$73 SamusKnockbackIsBomb`（1 B，2 个有效位）
asm 自述（`constants_ram.asm:625-627`）：`bit 7: 0=samus was hurt, 1=samus was bombed`；`bit 0: 0=diagonal knockback, 1=vertical knockback`。

写点：`07:F034 sta`（`:10062`）、`07:F311 sta`（`:10630`）；读点 `07:CDFA lda`（`:3094`）。采集点 3 种值：`$FF`×10501、`$00`×614、`$1B`×1 ⇒ **bit7/bit0 之外的位会带垃圾**（`$1B` 的 bit3/bit4 无定义），typed 声明只取 bit7/bit0，不能整字节当枚举。

### 4.6 附：`$FA MirrorCntrl`
asm：`bit 3 set = horizontal mirroring, clear = vertical`（`constants_ram.asm:811-813`）。采集点 5 种值 top `$2F`×5680、`$27`×5033、`$47`×398 —— **bit3 恒置**（0x2F/0x27/0x47 都含 bit3），其它位为噪声 ⇒ 同 `$73`，只判 bit3。

## 5. 三档判据与结果

判据（先定判据再看数据）：

- **可 typed（P/S）** 同时满足：① asm 有语义名（非 `TempNN` 别名、非自述 spare）；② 静态面 ≥1 读且 ≥1 写（指针组按 dw 整体算，允许只有低字节直接写）；③ 值域可枚举或为定点数值，能落成枚举/整数/位域；④ 跨帧保持（不是同一表达式内的中间量）。P = 指针组（必须 dw 建模，兼容 `+1` 访问）；S = 标量/位域/结构体数组。
- **只能 ephemeral（E）**：语义上是**派生量或游标**——每帧由世界坐标重算（`$51/$52`）、装载游标（`$5B`）、循环下标（`$4B`）、派生 attrib（`$67/$68`）、CPU 暂存（`$69/$6A`）。双相位 union（`$7A-$88`）不归 E：它有真实字段语义，按 S 声明、相位名分视图（见 §8 C1）。按计划 §4 的硬约束，**E 不降 Undeclared**（`tools/nes_map_check.py:903-915` 的转换等式），只作为「不要 typed」的理由。
- **不可声明（U）**：asm 自述 `SpareMem` / `Unused`，或静态面零访问且无任何读点。共 32 B：`$75 $78 $89 $D0-$D3 $D4-$D6 $D7 $D9-$DF $E8-$E9 $EE-$EF $F1-$F9 $FB`。

结果（卡面 202 B 全覆盖、无缺口，机械核对：P 29 + S 133 + E 8 + U 32 = 202；表里多出的 1 B 是 `$33`）：

| 档 | 字节数 | 构成 |
|---|---|---|
| 可 typed（P 29 + S 133） | **162 B** | 指针组 5 组（RoomPtr/StructPtr/RoomRAMWorkPtr/RoomRAMPtr/AreaPointers）+ DoorRoomRAMPtr[4]；§4 五段标量/位域 + 结构体 Skree[4]/Mellow[4]/RinkaSpawners[2] + MotherBrain 8 B + 声音指针 12 B 等 |
| 只能 ephemeral | **8 B** | `$4B $51 $52 $5B $67 $68 $69 $6A` |
| 不可声明 | **32 B** | 见上 U 清单 |

> `$6E/$6F HealthChange` 计入 typed，但归属 #77（已建模），本卡不新增它的实现。

## 6. memmap 落点与四态预期

**落点**：扩展现有 `crates/game-core/src/memmap/zero_page_face.rs`（当前 `BASE=0x0000`、`FACE_LEN=0x0033`、`SYMBOLS` 41 项、`impl NesMapped` `:319`）。做法与它同构：

1. `FACE_LEN` 改到 `0x00FE`；`SYMBOLS` 追加本文 §3 表的 P/S 行（每行带 `constants_ram.asm:行号` 来源）。
2. `SPANS` 按 P/S 分段声明，E/U 段留空并写 reason（保持「不接管行为」口径：本层只声明布局，不改业务读写）。
3. 建议顺带把 `$0033`（RoomPtr 低字节，现 Unmodeled 1 B）纳入，否则 dw 半截。

**四态预期（先算后验，用 `nes_map_check.py` 复核）**：

现状（本机实测）：Owner 1449 / View 23 / Unmodeled 385 / Undeclared 438（四项和 2295；比对面 2272，两者不是同一个量，别混用）。

| 方案 | 新增 Owner | Unmodeled | Undeclared | Owner 终值 | 校验 |
|---|---|---|---|---|---|
| 只覆盖卡面 `$34-$FD`（typed 162 B：已 Owner 7、Undeclared 146、Unmodeled 9） | +155 | 385→**376** | 438→**292** | 1449→**1604** | +155 = −(−146−9) ✓；四项和 2295 不变 |
| 顺带纳入 `$33`（typed 163 B） | +156 | 385→**375** | 438→**292** | 1449→**1605** | +156 = −(−146−10) ✓ |

剩余 39 B（E 7 + U 32）留在 Undeclared，逐段在 `data/status/nes_map.json` 里写 reason（E=派生/游标、U=spare 或零访问），不占 Owner。

## 7. 复跑命令（不依赖模拟器）

```bash
# 1) 命名表重建（应打印 end=0x100）
python3 analysis/zp/enum_parse.py

# 2) 静态普查（应打印 addrs 177 accesses 2344）
python3 analysis/zp/census2.py

# 3) 采集点统计（样本数随 traces 目录漂移，脚本会打印 total）
python3 analysis/zp/runtime.py

# 4) 四态现值（先看 INFO 行，再对本文 §6）
python3 tools/nes_map_check.py --list

# 5) 位域取值直方图（可换成 §4 任一地址）
python3 - <<'PYEOF'
import glob, collections
for a in (0x56, 0x58, 0x6B, 0x6C, 0x6D, 0x73, 0xFA):
    c = collections.Counter()
    for p in glob.glob('analysis/traces/*.ram.hex'):
        b = bytes.fromhex(open(p).read().strip())
        if len(b) > 0xFF:
            c[b[a]] += 1
    print(f'${a:02X}', len(c), c.most_common(6))
PYEOF
```

## 8. C 级假设与未决

- **C1**：`$7A-$90` 是「游戏期敌人计数 / 结局期结局变量」的**双相位 union**（`constants_ram.asm:642` `.union` … `:673` `.nextu` … `:684` `.endu`）。游戏期字段 `$7A-$90`（`$7A/$7B` 为 spare、`$7D OnFrozenEnemy`、`$7E-$80` Boss 计数、`$81-$86` 生成器指针/动画索引、`$8B-$90` RinkaSpawners）；结局期字段 `$7A-$84`（`EndMsgWrite/IsCredits/SpriteByteCounter/…/WaveSpriteCounter`）。本文按 S 归入 typed，但**字段名必须随相位**。建议实现做成「相位 tag + 两个视图」，不要拍一个名字。
- **C2**：`$A0-$AF` / `$B0-$CF` 的槽内字段零直接访问（`$A4-$AF`、`$B1-$CF` 多数），本文按结构体整段 typed；**槽 1–3 是否真被 `,x` 访问**需实现卡用 trace 验证（当前采集点 `$A0` nz 0%、`$B0` nz 0%，相位不明）。
- **C3**：采集点相位盲区：`$58/$79/$98/$FC` 在快照里为 0 或近 0，但静态面分别有 6/4/8/37 个读点（§3 表的 R 列）。判「是否使用」一律以静态面为准；运行期只用于值域与位域佐证。
- **未决**：`$D4-$D6/$D9-$DF` 是 `ds` 空洞、没有独立 asm 名（`SpareMemD3 ds 4` 覆盖 `$D3-$D6`、`SpareMemD9 ds 7` 覆盖 `$D9-$DF`），只能按 U 处理；若后续发现被 `,x` 索引访问，需单独取证再改档。
