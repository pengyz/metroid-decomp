# findings：A5 批产取证打包（bank03 域簇 24-45——预消化 v2：指令头直读）

> 状态：**A5 证据预备仓就位**（2026-10-18）。预消化升级：成员=addr+名+**指令头**
> （nmikstas 行直读——区域码名稀疏（A1 重叠 1/106）以指令头为主锚）。
> 跨区双向：出边（本域→bank7）+入边同表。

## 1. A5 簇清单（22 簇/57 员——指令头预消化）

| 簇 | 员数 | 成员（addr+名+指令头） |
|---|---|---|
| A5-23 | 3 | $9871 `JSR $9A06`; $987A `JSR $9A06`; $9A06 `LSR` |
| A5-24 | 3 | $9900 `JSR $9A10`; $9909 `JSR $9A10`; $9A10 `ASL` |
| A5-25 | 2 | $9926 `JSR $99C3`; $99C3 `LDA #$00` |
| A5-26 | 2 | $9929 `JSR $99BD`; $99BD `TXA` |
| A5-27 | 2 | $9958 `JSR $99E4`; $99E4 `LDA $030E` |
| A5-28 | 3 | $995E `JSR $99F4`; $99F4 `LDA $09`; $9AF1 `JSR $99F4` |
| A5-29 | 2 | $9961 `JMP $9967`; $9967 `LDA $6AF4,X` |
| A5-30 | 5 | $99B8 `JSR $9B1B`; $99BE `JSR $9B1B`; $9AFC `JSR $9B1B`; $9B0C `JSR $9B1B`; $9B1B `LSR` |
| A5-31 | 2 | $99D1 `STA $6AFF,X`; $9A3A `JSR $99D1` |
| A5-32 | 2 | $9A83 `JSR $9AF9`; $9AF9 `LDA $00` |
| A5-33 | 3 | $9B03 `JSR $9B20`; $9B13 `JSR $9B20`; $9B20 `ASL` |
| A5-34 | 2 | $9DF2 MotherBrain_Idle_CollideWithSamus `LDA $030C`; $9E22 MotherBrain_Idle `JSR $9DF2` |
| A5-35 | 2 | $9E25 `JSR $9FED`; $9FED MotherBrain_Idle_HandleBeingHit `LDA $9E` |
| A5-36 | 2 | $9E28 `JSR $A01B`; $A01B MotherBrain_Idle_UpdateAnimBrain `DEC $9A` |
| A5-37 | 2 | $9E2B `JSR $A02E`; $A02E MotherBrain_Idle_UpdateAnimEye `DEC $9B` |
| A5-38 | 3 | $9E2E `JSR $A041`; $9E83 `JMP $9E2E`; $A041 MotherBrain_DrawSprites `LDA #$E0` |
| A5-39 | 2 | $9E31 ClearMotherBrainIsHit `LDA #$00`; $9E3E `JMP $9E31` |
| A5-40 | 4 | $9E36 MotherBrain_Hurt `JSR $9E43`; $9E43 UpdateMotherBrainFlashDelay `DEC $9F`; $9E52 MotherBrain_Killed `JSR $9E43`; $9E93 `JSR $9E43` |
| A5-41 | 4 | $9E6C `JSR $9EF9`; $9EA4 `JSR $9EF9`; $9EF9 Xplus16 `TXA`; $A00C `JSR $9EF9` |
| A5-42 | 2 | $9E8E `JSR $A072`; $A072 MotherBrain_Disappear_Disintegrate `LDY MotherBrainHits` |
| A5-43 | 2 | $9ED0 `JMP $9ED6`; $9ED6 `LDA $0685` |
| A5-44 | 3 | $9F49; $9F69 MotherBrain_SpawnDoor `LDA $50`; $9FDA MotherBrain_SetTimeBomb_ScrollBackOnScreen `JSR $9F69` |

## 1.5 消化口径（F13 钉死）

**标签口径三级**：①名注优先（nmikstas Name: 或 m1disasm `Name: ; BB:XXXX`）
②L 行尾注次之（nmikstas 注释语义）③指令头兜底（操作码直读）。表头列
「addr+名+指令头」中「名」列=①②并取（有则显）；口径随包不变。

## 2. 跨区出边（bank03→bank7 引擎调用——残余随件）

| bank7 目标 | 调用次数 |
|---|---|
| $C3D4 | 5 次 |
| $E770 | 3 次 |
| $E77B | 3 次 |
| $C2C5 | 3 次 |
| $D2FD | 2 次 |
| $CBDA | 2 次 |
| $C2BF | 2 次 |
| $F410 | 1 次 |
| $F438 | 1 次 |
| $F416 | 1 次 |
| $F852 | 1 次 |
| $E094 | 1 次 |

## 3. 结构互核注

- 簇边界=632 边 union-find（task-43 口径）；名稀疏如实（指令头为主锚）；
- 每批验收=L2 增量+L3 回归+全案绿（施工图 §2 口径）。
