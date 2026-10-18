# findings：A5 批产取证打包（bank03 域簇 24-45——预消化 v2：指令头直读）

> 状态：**A5 证据预备仓就位**（2026-10-18）。预消化升级：成员=addr+名+**指令头**
> （nmikstas 行直读——区域码名稀疏（A1 重叠 1/106）以指令头为主锚）。
> 跨区双向：出边（本域→bank7）+入边同表。

## 1. A5 簇清单（22 簇/57 员——指令头预消化）

| 簇 | 员数 | 成员（addr+名+指令头） |
|---|---|---|
| A5-23 | 3 | $9871, $987A, $9A06 |
| A5-24 | 3 | $9900, $9909, $9A10 |
| A5-25 | 2 | $9926, $99C3 |
| A5-26 | 2 | $9929, $99BD |
| A5-27 | 2 | $9958, $99E4 |
| A5-28 | 3 | $995E, $99F4, $9AF1 |
| A5-29 | 2 | $9961, $9967 |
| A5-30 | 5 | $99B8, $99BE, $9AFC, $9B0C, $9B1B |
| A5-31 | 2 | $99D1, $9A3A |
| A5-32 | 2 | $9A83, $9AF9 |
| A5-33 | 3 | $9B03, $9B13, $9B20 |
| A5-34 | 2 | $9DF2 MotherBrain_Idle_CollideWithSamus, $9E22 MotherBrain_Idle |
| A5-35 | 2 | $9E25, $9FED MotherBrain_Idle_HandleBeingHit |
| A5-36 | 2 | $9E28, $A01B MotherBrain_Idle_UpdateAnimBrain |
| A5-37 | 2 | $9E2B, $A02E MotherBrain_Idle_UpdateAnimEye |
| A5-38 | 3 | $9E2E, $9E83, $A041 MotherBrain_DrawSprites |
| A5-39 | 2 | $9E31 ClearMotherBrainIsHit, $9E3E |
| A5-40 | 4 | $9E36 MotherBrain_Hurt, $9E43 UpdateMotherBrainFlashDelay, $9E52 MotherBrain_Killed, $9E93 |
| A5-41 | 4 | $9E6C, $9EA4, $9EF9 Xplus16, $A00C |
| A5-42 | 2 | $9E8E, $A072 MotherBrain_Disappear_Disintegrate |
| A5-43 | 2 | $9ED0, $9ED6 |
| A5-44 | 3 | $9F49 MotherBrain_SetTimeBomb, $9F69 MotherBrain_SpawnDoor, $9FDA MotherBrain_SetTimeBomb_ScrollBackOnScreen |

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
