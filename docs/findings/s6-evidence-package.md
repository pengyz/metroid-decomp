# findings：S6 批产取证打包（共享引擎 151-175 簇——同 S1 模板）

> 状态：**S6 证据预备仓就位**（2026-10-18）。跨区=m1disasm 名调用匹配法
> （S2 定型）；标签=m1disasm 名标签双源。
> 复现：`python3 tools/decode_sweep.py --json /tmp/sweep.json` + union-find + 切片。

## 1. S6 簇清单（25 簇/50 员）

| 簇 | 员数 | 成员（bank7:addr+名标签） |
|---|---|---|
| S6-150 | 2 | $CBBC SFX_EnergyPickup `LDA #$40`; $F4B4 |
| S6-151 | 2 | $CBC0 SFX_MissilePickup `LDA #SFX_MSL_PKUP`; $F4D1 |
| S6-152 | 2 | $F423 `sta ObjectCntrl`; $F512 |
| S6-153 | 2 | $F533; $F673 |
| S6-154 | 2 | $95A8; $F568 |
| S6-155 | 2 | $CBB8 SFX_Metal `LDA #$20`; $F573 |
| S6-156 | 2 | $F42D `ldx PageIndex`; $F576 |
| S6-157 | 2 | $CBD6 SFX_BossHit `LDA #$02`; $F584 |
| S6-158 | 2 | $CBCE SFX_BigEnemyHit `LDA #$04`; $F5A7 |
| S6-159 | 2 | $DCFC `LDA InArea`; $F623 |
| S6-160 | 2 | $81DA; $F70C |
| S6-161 | 2 | $820F; $F741 |
| S6-162 | 2 | $CBA4 SFX_OutOfPipe `LDA #$08`; $F7D1 |
| S6-163 | 2 | $81D1; $F82C |
| S6-164 | 2 | $F82F; $F839 |
| S6-165 | 2 | $8206; $F836 |
| S6-166 | 2 | $F887; $F8E8 SpawnEnProjectile_FindSlot `ldy #$60` |
| S6-167 | 2 | $F88F; $F92C Table15 `lda #$02` |
| S6-168 | 2 | $F8BD; $F8F8 SpawnEnProjectile_F8F8 `lda $85` |
| S6-169 | 2 | $F8D3; $F91D SpawnEnProjectile_SetEnProjectilePosition `ldx PageIndex` |
| S6-170 | 2 | $F690 SetEnAnimIndex `sta EnAnimIndex,x`; $F8E5 |
| S6-171 | 2 | $F927; $FA49 LoadEnemyPositionFromTemp `lda $08` |
| S6-172 | 2 | $F979; $FA60 EnemyBecomePickup `lda #$00` |
| S6-173 | 2 | $F987 LF987 `inc $0408,x`; $F9B0 |
| S6-174 | 2 | $F518 LF518 `lda #$04`; $FA0D |

## 1.5 消化口径（F13 钉死——v3 法）

**标签口径三级**：①名注优先 ②L 行尾注次之 ③指令头兜底。

## 2. asm 锚预消化

- 名标签消解 16/50（m1disasm 双源）；残余=区内中间标签（随移植细化）；
- 簇规模特征：辅助例族（均 3 员）。

## 3. 跨区依赖表（S6×area）

| 目标 | 名 | 调用方 |
|---|---|---|
| — | — | 无跨区调用 |

## 4. 结构互核注

- 簇边界=632 边 union-find（task-43 口径）；顺序尾段附着清单随执行序细化；
- 每批验收=L2 增量+L3 回归+全案绿（施工图 §2 口径）。
