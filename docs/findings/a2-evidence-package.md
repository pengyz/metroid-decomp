# findings：A2 批产取证打包（bank01 域簇 23-44——预消化 v2：指令头直读）

> 状态：**A2 证据预备仓就位**（2026-10-18）。预消化升级：成员=addr+名+**指令头**
> （nmikstas 行直读——区域码名稀疏（A1 重叠 1/106）以指令头为主锚）。
> 跨区双向：出边（本域→bank7）+入边同表。

## 1. A2 簇清单（22 簇/50 员——指令头预消化）

| 簇 | 员数 | 成员（addr+名+指令头） |
|---|---|---|
| A2-22 | 2 | $8CBB `JSR $CC07`; $CC07 |
| A2-23 | 2 | $8CC0 `JSR $CC03`; $CC03 |
| A2-24 | 2 | $8CDD `JSR $D2FD`; $D2FD |
| A2-25 | 2 | $8CE0 `JSR $CBDA`; $CBDA |
| A2-26 | 2 | $8CE3 `JSR $CB73`; $CB73 |
| A2-27 | 2 | $95C6 TwosCompliment_ `EOR #$FF`; $9B19 `JSR $95C6` |
| A2-28 | 2 | $99A1 `LDA $81`; $99D0 `JMP $99A1` |
| A2-29 | 2 | $99BA `STA $85`; $99D5 `JMP $99BA` |
| A2-30 | 3 | $99C8 `LDA #$06`; $99E2 `JMP $99C8`; $99F7 `JMP $99C8` |
| A2-31 | 2 | $99CC `LDA #$08`; $9BCF `JMP $99CC` |
| A2-32 | 2 | $9A74 `JSR $9ABD`; $9ABD `LDA $0405,X` |
| A2-33 | 3 | $9A7E `JSR $9AE2`; $9AC9 `JSR $9AE2`; $9AE2 `LDY $0405,X` |
| A2-34 | 2 | $9A81 `JSR $9AA8`; $9AA8 `LDX PageIndex` |
| A2-35 | 4 | $9A84 `JSR $9AC6`; $9AC6 `JSR $9ADA`; $9AD0 `JSR $9ADA`; $9ADA `LDY $040A,X` |
| A2-36 | 3 | $9A8F `LDA $0405,X`; $9ABA `JMP $9A8F`; $9AD6 `JSR $9A8F` |
| A2-37 | 3 | $9B8A `JSR $9CA8`; $9C84 `JSR $9CA8`; $9CA8 `LDA EnYRoomPos,X` |
| A2-38 | 2 | $9B92 `JSR $9C96`; $9C96 `LDA $08` |
| A2-39 | 2 | $9BC2 `JSR $9C1D`; $9C1D `LDX #$50` |
| A2-40 | 2 | $9BC5 `JSR $9CCC`; $9CCC `LDY $7E` |
| A2-41 | 2 | $9BC8 `JSR $9D05`; $9D05 `LDY $7F` |
| A2-42 | 2 | $9BD2 `LDA $0405,X`; $9C1A `JMP $9BD2` |
| A2-43 | 2 | $9C1F `JSR $9C2A`; $9C2A `LDY EnStatus,X` |

## 2. 跨区出边（bank01→bank7 引擎调用——残余随件）

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
