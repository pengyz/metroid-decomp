# findings：S3 批产取证打包（共享引擎 61-90 簇——同 S1 模板）

> 状态：**S3 证据预备仓就位**（2026-10-18）。跨区=m1disasm 名调用匹配法
> （S2 定型）；标签=m1disasm 名标签双源。
> 复现：`python3 tools/decode_sweep.py --json /tmp/sweep.json` + union-find + 切片。

## 1. S3 簇清单（30 簇/66 员）

| 簇 | 员数 | 成员（bank7:addr+名标签） |
|---|---|---|
| S3-60 | 3 | $CBD2 SFX_MetroidHit `LDA #$20`; $F530; $F598 |
| S3-61 | 3 | $F589; $F6BD; $F74B ReadTableAt968B `ldy EnDataIndex,x` |
| S3-62 | 3 | $CBB0 SFX_EnemyHit `LDA #$04`; $F59D; $F5A2 |
| S3-63 | 3 | $F6EB; $F71A; $F752 LoadEnHiToYAndLoadEorHiToCarry `lda EnNameTable,x` |
| S3-64 | 3 | $F96A UpdateEnProjectile_Resting `jsr LFA5B`; $F991 UpdateEnProjectile_Active `jsr LFA5B`; $FA5B EnemyBecomePickupIfHit `lda $0404,x` |
| S3-65 | 3 | $F96D; $F9F2; $FA1E EnemyBGCollideOrApplySpeed `lda InArea` |
| S3-66 | 2 | $D015; $D055 SamusJump `ldx #$01` |
| S3-67 | 2 | $D02B; $D09C Table04 `lda Joy1Change` |
| S3-68 | 2 | $CFC3 SetSamusJump `LDY #an_SamusJump`; $D036 |
| S3-69 | 2 | $CD6B `lda #$03`; $D039 |
| S3-70 | 2 | $CF77 SetSamusPntUp `* LDA #sa_PntUp`; $D048 |
| S3-71 | 2 | $CBEF SFX_SamusBall `LDA #$02`; $D0D8 |
| S3-72 | 2 | $D0FA; $E7A2 CheckMoveUp `ldx PageIndex` |
| S3-73 | 2 | $D10D; $D638 LoadObjectPositionFromTemp `lda $08` |
| S3-74 | 2 | $CCB7 SetSamusRunAccel `lda RunAccelerationTbl`; $D130 |
| S3-75 | 2 | $D136; $D150 |
| S3-76 | 2 | $CFB7 ClearHorzData `JSR ClearHorzMvmntData`; $D13F |
| S3-77 | 2 | $CBA8 SFX_BombLaunch `LDA #$01`; $D194 |
| S3-78 | 2 | $D1C0; $D1EE FireWeapon `lda Joy1Status` |
| S3-79 | 2 | $C27C ChooseRoutine `ASL`; $D1D4 |
| S3-80 | 2 | $D3BA; $D48C Door_DeleteOffscreenEnemies `ldx #$60` |
| S3-81 | 2 | $D3BD; $ED65 Doors_RemoveIfOffScreen `ldx #$B0` |
| S3-82 | 2 | $95AB; $D3C0 |
| S3-83 | 2 | $CC07 TourianMusic `LDA #$40`; $D3DD |
| S3-84 | 2 | $D417; $DE4A DrawFrame `ldx PageIndex` |
| S3-85 | 2 | $D48F; $D4B4 `lda $0405,x` |
| S3-86 | 2 | $D49E; $D4A8 `tya` |
| S3-87 | 2 | $D530; $D5FC UpdateBullet_DeleteIfOffScreen `lda ObjectOnScreen,x` |
| S3-88 | 2 | $D533; $D5DA Table0D `lda $030A,x` |
| S3-89 | 2 | $D522 DrawBullet `inc $0500,x`; $D552 |

## 1.5 消化口径（F13 钉死——v3 法）

**标签口径三级**：①名注优先 ②L 行尾注次之 ③指令头兜底。

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
