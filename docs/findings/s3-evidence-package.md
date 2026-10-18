# findings：S3 批产取证打包（共享引擎 61-90 簇——同 S1 模板）

> 状态：**S3 证据预备仓就位**（2026-10-18）。跨区=m1disasm 名调用匹配法
> （S2 定型）；标签=m1disasm 名标签双源。
> 复现：`python3 tools/decode_sweep.py --json /tmp/sweep.json` + union-find + 切片。

## 1. S3 簇清单（30 簇/66 员）

| 簇 | 员数 | 成员（bank7:addr+名标签） |
|---|---|---|
| S3-60 | 3 | $CBD2 SFX_MetroidHit, $F530 Lx314, $F598 |
| S3-61 | 3 | $F589, $F6BD, $F74B ReadTableAt968B |
| S3-62 | 3 | $CBB0 SFX_EnemyHit, $F59D PlaySnd1, $F5A2 PlaySnd2 |
| S3-63 | 3 | $F6EB, $F71A, $F752 LoadEnHiToYAndLoadEorHiToCarry |
| S3-64 | 3 | $F96A UpdateEnProjectile_Resting, $F991 UpdateEnProjectile_Active, $FA5B EnemyBecomePickupIfHit |
| S3-65 | 3 | $F96D, $F9F2 Lx365, $FA1E EnemyBGCollideOrApplySpeed |
| S3-66 | 2 | $D015 Lx019, $D055 SamusJump_CheckHorzMovement |
| S3-67 | 2 | $D02B, $D09C SamusJump_CheckFire |
| S3-68 | 2 | $CFC3, $D036 |
| S3-69 | 2 | $CD6B, $D039 |
| S3-70 | 2 | $CF77 SetSamusPntUp, $D048 |
| S3-71 | 2 | $CBEF SFX_SamusBall, $D0D8 |
| S3-72 | 2 | $D0FA, $E7A2 ObjectCheckMoveUp |
| S3-73 | 2 | $D10D, $D638 LoadObjectPositionFromTemp |
| S3-74 | 2 | $CCB7 SetSamusRunAccel, $D130 |
| S3-75 | 2 | $D136, $D150 CheckBombLaunch |
| S3-76 | 2 | $CFB7 StopHorzMovement, $D13F |
| S3-77 | 2 | $CBA8 SFX_BombLaunch, $D194 |
| S3-78 | 2 | $D1C0, $D1EE FireSamusProjectile |
| S3-79 | 2 | $C27C JumpEngine, $D1D4 |
| S3-80 | 2 | $D3BA, $D48C Door_DeleteOffscreenEnemies |
| S3-81 | 2 | $D3BD, $ED65 Doors_RemoveIfOffScreen |
| S3-82 | 2 | $95AB, $D3C0 |
| S3-83 | 2 | $CC07 TourianMusic, $D3DD |
| S3-84 | 2 | $D417, $DE4A ObjDrawFrame |
| S3-85 | 2 | $D48F, $D4B4 |
| S3-86 | 2 | $D49E, $D4A8 |
| S3-87 | 2 | $D530, $D5FC UpdateBullet_DeleteIfOffScreen |
| S3-88 | 2 | $D533, $D5DA UpdateBullet_ExplodeIfHitSprite |
| S3-89 | 2 | $D522 LD522, $D552 |

## 2. asm 锚预消化

- 名标签消解 32/66（m1disasm 双源）；残余=区内中间标签（随移植细化）；
- 簇规模特征：辅助例族（均 3 员）。

## 3. 跨区依赖表（S3×area）

| 目标 | 名 | 调用方 |
|---|---|---|
| $C27C | JumpEngine | prg0_title.asm |

## 4. 结构互核注

- 簇边界=632 边 union-find（task-43 口径）；顺序尾段附着清单随执行序细化；
- 每批验收=L2 增量+L3 回归+全案绿（施工图 §2 口径）。
