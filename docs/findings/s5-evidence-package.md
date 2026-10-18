# findings：S5 批产取证打包（共享引擎 121-150 簇——同 S1 模板）

> 状态：**S5 证据预备仓就位**（2026-10-18）。跨区=m1disasm 名调用匹配法
> （S2 定型）；标签=m1disasm 名标签双源。
> 复现：`python3 tools/decode_sweep.py --json /tmp/sweep.json` + union-find + 切片。

## 1. S5 簇清单（30 簇/60 员）

| 簇 | 员数 | 成员（bank7:addr+名标签） |
|---|---|---|
| S5-120 | 2 | $EBC8, $EE41 LEE41 |
| S5-121 | 2 | $EE11, $EE3D PrepareItemID |
| S5-122 | 2 | $DC67 CreateItemID, $EE47 |
| S5-123 | 2 | $EE6B, $EE86 SpawnMellow |
| S5-124 | 2 | $95B1, $EEA6 SpawnCannon |
| S5-125 | 2 | $95B4, $EEAE SpawnMotherBrain |
| S5-126 | 2 | $95B7, $EECA SpawnZebetite |
| S5-127 | 2 | $95BA, $EEEE SpawnRinkaSpawner |
| S5-128 | 2 | $F0A8, $F152 GetEnemyXSlotPosition |
| S5-129 | 2 | $F0CC, $F2CA CollisionDetectionEnemy_ReactToCollisionWithSamusProjectile |
| S5-130 | 2 | $F0E2, $F282 CollisionDetectionEnemy_ReactToCollisionWithSamus |
| S5-131 | 2 | $F09F, $F0EA |
| S5-132 | 2 | $F0EF, $F172 GetObjectXSlotPosition |
| S5-133 | 2 | $F106, $F1B3 GetRadiusSumsOfObjXSlotAndEnYSlot |
| S5-134 | 2 | $F109, $F162 GetEnemyYSlotPosition |
| S5-135 | 2 | $F10F, $F2ED CollisionDetectionEnProjectile_ReactToCollisionWithSamus |
| S5-136 | 2 | $DC82 LDC82, $F130 |
| S5-137 | 2 | $F16F, $F193 GetYSlotPosition_Common |
| S5-138 | 2 | $F1B6, $F1E7 AddEnemyYSlotRadiusY |
| S5-139 | 2 | $F1BC, $F1CB AddEnemyYSlotRadiusX |
| S5-140 | 2 | $F279 SetSamusProjectileIsHit, $F2D2 |
| S5-141 | 2 | $F2DF CollisionDetectionEnProjectile_ReactToCollisionWithSamus_F2DF, $F2EF |
| S5-142 | 2 | $F270 LF270, $F303 |
| S5-143 | 2 | $F3E0, $F7BA UpdateEnemy_Resting_TryBecomingActive |
| S5-144 | 2 | $F407, $F51E RemoveEnemyIfItIsInLava |
| S5-145 | 2 | $95E5, $F40D UpdateEnemy_Explode |
| S5-146 | 2 | $E094 UpdateEnemyAnim, $F410 UpdateEnemyCommon |
| S5-147 | 2 | $8058, $F413 |
| S5-148 | 2 | $DD8B DrawEnemy, $F42A |
| S5-149 | 2 | $CEF9 AddHealth, $F4B1 |

## 2. asm 锚预消化

- 名标签消解 29/60（m1disasm 双源）；残余=区内中间标签（随移植细化）；
- 簇规模特征：辅助例族（均 3 员）。

## 3. 跨区依赖表（S5×area）

| 目标 | 名 | 调用方 |
|---|---|---|
| — | — | 无跨区调用 |

## 4. 结构互核注

- 簇边界=632 边 union-find（task-43 口径）；顺序尾段附着清单随执行序细化；
- 每批验收=L2 增量+L3 回归+全案绿（施工图 §2 口径）。
