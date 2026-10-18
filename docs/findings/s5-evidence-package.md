# findings：S5 批产取证打包（共享引擎 121-150 簇——同 S1 模板）

> 状态：**S5 证据预备仓就位**（2026-10-18）。跨区=m1disasm 名调用匹配法
> （S2 定型）；标签=m1disasm 名标签双源。
> 复现：`python3 tools/decode_sweep.py --json /tmp/sweep.json` + union-find + 切片。

## 1. S5 簇清单（30 簇/60 员）

| 簇 | 员数 | 成员（bank7:addr+名标签） |
|---|---|---|
| S5-120 | 2 | $EBC8; $EE41 LEE41 `sta $07` |
| S5-121 | 2 | $EE11 ($EE3D)Get unique item ID. `jsr PrepareItemID`; $EE3D PrepareItemID `sta $09` |
| S5-122 | 2 | $DC67 CreateItemID `LDA $07`; $EE47 ($DC67)Get unique item ID. `jmp CreateItemID` |
| S5-123 | 2 | $EE6B; $EE86 SpawnMellow `lda $B0,x` |
| S5-124 | 2 | $95B1; $EEA6 CannonHandler `jsr $95B1` |
| S5-125 | 2 | $95B4; $EEAE MotherBrainHandler `jsr $95B4` |
| S5-126 | 2 | $95B7; $EECA ZeebetiteHandler `jsr $95B7` |
| S5-127 | 2 | $95BA; $EEEE RinkaHandler `jsr $95BA` |
| S5-128 | 2 | $F0A8; $F152 GetEnemyXSlotPosition `lda EnYRoomPos,x` |
| S5-129 | 2 | $F0CC; $F2CA CollisionDetectionEnemy_ReactToCollisionWithSamusProjectile `bcs +++` |
| S5-130 | 2 | $F0E2; $F282 CollisionDetectionEnemy_ReactToCollisionWithSamus `bcs Exit17` |
| S5-131 | 2 | $F09F slot active? `lda EnStatus,x`; $F0EA |
| S5-132 | 2 | $F0EF; $F172 GetObject0CoordData `lda ObjectY,x` |
| S5-133 | 2 | $F106; $F1B3 DistFromObj0ToEn1 `lda ObjRadY,x` |
| S5-134 | 2 | $F109; $F162 GetEnemyYSlotPosition `lda EnYRoomPos,y` |
| S5-135 | 2 | $F10F; $F2ED CollisionDetectionEnProjectile_ReactToCollisionWithSamus `bcs +` |
| S5-136 | 2 | $DC82 LDC82 `JSR LF172`; $F130 |
| S5-137 | 2 | $F16F; $F193 GetYSlotPosition_Common `eor PPUCNT0ZP` |
| S5-138 | 2 | $F1B6; $F1E7 AddEnemyYSlotRadiusY `clc` |
| S5-139 | 2 | $F1BC; $F1CB AddEnemy1XRadius `clc` |
| S5-140 | 2 | $F279 SetSamusProjectileIsHit `lda $10`; $F2D2 |
| S5-141 | 2 | $F2DF CollisionDetectionEnProjectile_ReactToCollisionWithSamus_F2DF `lda $10`; $F2EF |
| S5-142 | 2 | $F270 LF270 `ora $030A,x`; $F303 |
| S5-143 | 2 | $F3E0; $F7BA UpdateEnemy_Resting_TryBecomingActive `dec EnDelay,x` |
| S5-144 | 2 | $F407; $F51E RemoveEnemyIfItIsInLava `lda ScrollDir` |
| S5-145 | 2 | $95E5; $F40D |
| S5-146 | 2 | $E094 UpdateEnemyAnim `ldx PageIndex`; $F410 UpdateEnemyCommon `jsr UpdateEnemyAnim` |
| S5-147 | 2 | $8058; $F413 |
| S5-148 | 2 | $DD8B DrawEnemy `ldx PageIndex`; $F42A |
| S5-149 | 2 | $CEF9 AddHealth `LDA HealthLo`; $F4B1 |

## 1.5 消化口径（F13 钉死——v3 法）

**标签口径三级**：①名注优先 ②L 行尾注次之 ③指令头兜底。

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
