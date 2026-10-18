# findings：S4 批产取证打包（共享引擎 91-120 簇——同 S1 模板）

> 状态：**S4 证据预备仓就位**（2026-10-18）。跨区=m1disasm 名调用匹配法
> （S2 定型）；标签=m1disasm 名标签双源。
> 复现：`python3 tools/decode_sweep.py --json /tmp/sweep.json` + union-find + 切片。

## 1. S4 簇清单（30 簇/60 员）

| 簇 | 员数 | 成员（bank7:addr+名标签） |
|---|---|---|
| S4-90 | 2 | $D585; $D624 UpdateBullet_Move `ldx PageIndex` |
| S4-91 | 2 | $D4F8; $D588 |
| S4-92 | 2 | $D4EB; $D5C5 |
| S4-93 | 2 | $D517; $D5D7 |
| S4-94 | 2 | $D606; $E81E IsWalkableTile `ldx UpdtngPrjctl` |
| S4-95 | 2 | $95C0; $D614 |
| S4-96 | 2 | $D621; $E9BE |
| S4-97 | 2 | $CB9A SFX_BombExplode `LDA #$10`; $D68B |
| S4-98 | 2 | $D779; $E9C2 IsBlastTile `tay` |
| S4-99 | 2 | $D831; $E6A7 |
| S4-100 | 2 | $D837; $E6D2 |
| S4-101 | 2 | $D851; $D866 |
| S4-102 | 2 | $CA18 IsEngineRunning `ldy MainRoutine`; $D8E7 |
| S4-103 | 2 | $C43F WaitNMIPass_ `JSR ClearNMIStat`; $D8FA |
| S4-104 | 2 | $CB73 SelectSamusPal `tya`; $D8FD |
| S4-105 | 2 | $C447 ScreenOn `LDA PPUCNT1ZP`; $D903 |
| S4-106 | 2 | $C8B0; $D906 |
| S4-107 | 2 | $C8BB; $D909 |
| S4-108 | 2 | $D96B; $E252 ToggleScroll `lda ScrollDir` |
| S4-109 | 2 | $D892; $D973 |
| S4-110 | 2 | $E778; $E783 EnemyCheckMoveVertical `sta $02` |
| S4-111 | 2 | $E78F; $E7BD CheckMoveDown `bne +` |
| S4-112 | 2 | $E8F9; $E904 EnemyCheckMoveHorizontalBranch `sta $03` |
| S4-113 | 2 | $E89B CheckMoveRight `bne +`; $E90C |
| S4-114 | 2 | $EB06 LoadEnemy `jsr GetEnemyData`; $EB0C GetEnemyData `lda ($00),y` |
| S4-115 | 2 | $EB11; $EB7A IsSlotTaken `lda EnStatus,x` |
| S4-116 | 2 | $EB19; $EB28 GetEnemyType `pha` |
| S4-117 | 2 | $EB20; $EB4D LEB4D `tay` |
| S4-118 | 2 | $EB74; $FB7B InitEnemyFacingDirectionAxisAndDelay `jsr $80B0` |
| S4-119 | 2 | $EB77; $F85A InitEnemyForceSpeedTowardsSamusDelayAndHealth `ldy EnDataIndex,x` |

## 1.5 消化口径（F13 钉死——v3 法）

**标签口径三级**：①名注优先 ②L 行尾注次之 ③指令头兜底。

## 2. asm 锚预消化

- 名标签消解 30/60（m1disasm 双源）；残余=区内中间标签（随移植细化）；
- 簇规模特征：辅助例族（均 3 员）。

## 3. 跨区依赖表（S4×area）

| 目标 | 名 | 调用方 |
|---|---|---|
| $C43F | WaitNMIPass_ | prg0_title.asm |
| $C447 | ScreenOn | prg0_title.asm |

## 4. 结构互核注

- 簇边界=632 边 union-find（task-43 口径）；顺序尾段附着清单随执行序细化；
- 每批验收=L2 增量+L3 回归+全案绿（施工图 §2 口径）。
