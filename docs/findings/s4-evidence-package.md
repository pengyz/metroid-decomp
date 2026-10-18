# findings：S4 批产取证打包（共享引擎 91-120 簇——同 S1 模板）

> 状态：**S4 证据预备仓就位**（2026-10-18）。跨区=m1disasm 名调用匹配法
> （S2 定型）；标签=m1disasm 名标签双源。
> 复现：`python3 tools/decode_sweep.py --json /tmp/sweep.json` + union-find + 切片。

## 1. S4 簇清单（30 簇/60 员）

| 簇 | 员数 | 成员（bank7:addr+名标签） |
|---|---|---|
| S4-90 | 2 | $D585, $D624 UpdateBullet_Move |
| S4-91 | 2 | $D4F8 CheckBulletStat, $D588 |
| S4-92 | 2 | $D4EB UpdateBullet, $D5C5 |
| S4-93 | 2 | $D517 DrawBullet, $D5D7 |
| S4-94 | 2 | $D606 GotoSamusProjectileHitDoorOrStatue, $E81E SamusProjectileHitDoorOrStatue |
| S4-95 | 2 | $95C0, $D614 |
| S4-96 | 2 | $D621, $E9BE IsBlastTile |
| S4-97 | 2 | $CB9A SFX_BombExplode, $D68B |
| S4-98 | 2 | $D779, $E9C2 |
| S4-99 | 2 | $D831, $E6A7 ScrollLeft |
| S4-100 | 2 | $D837, $E6D2 ScrollRight |
| S4-101 | 2 | $D851, $D866 |
| S4-102 | 2 | $CA18 IsEngineRunning, $D8E7 |
| S4-103 | 2 | $C43F WaitNMIPass_, $D8FA |
| S4-104 | 2 | $CB73 SelectSamusPalette, $D8FD |
| S4-105 | 2 | $C447 ScreenOn, $D903 |
| S4-106 | 2 | $C8B0 CopyAreaPointers, $D906 |
| S4-107 | 2 | $C8BB DestroyEnemies, $D909 |
| S4-108 | 2 | $D96B, $E252 ToggleScroll |
| S4-109 | 2 | $D892 ElevScrollRoom, $D973 Lx116 |
| S4-110 | 2 | $E778, $E783 EnemyCheckMoveVertical |
| S4-111 | 2 | $E78F, $E7BD CheckMoveVertical |
| S4-112 | 2 | $E8F9, $E904 EnemyCheckMoveHorizontalBranch |
| S4-113 | 2 | $E89B CheckMoveHorizontal, $E90C |
| S4-114 | 2 | $EB06 LoadEnemy, $EB0C GetEnemyData |
| S4-115 | 2 | $EB11, $EB7A IsSlotTaken |
| S4-116 | 2 | $EB19, $EB28 GetEnemyType |
| S4-117 | 2 | $EB20, $EB4D LEB4D |
| S4-118 | 2 | $EB74, $FB7B InitEnemyFacingDirectionAxisAndDelay |
| S4-119 | 2 | $EB77, $F85A InitEnemyForceSpeedTowardsSamusDelayAndHealth |

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
