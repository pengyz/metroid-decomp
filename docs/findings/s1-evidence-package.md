# findings：S1 批产取证打包（共享引擎首批——Week 2 施工素材）

> 状态：**S1 证据预备仓就位**（2026-10-18）。簇=task-43 union-find 复现对表
> （387 簇域归属逐位吻合 ✓）；S1=共享引擎前 30 簇/160 员。
> 复现：`python3 tools/decode_sweep.py --json /tmp/sweep.json` + union-find。

## 1. S1 簇清单（30 簇/160 员——排序=簇规模降序）

| 簇 | 员数 | 成员（bank7:addr+nmikstas 标签） |
|---|---|---|
| S1-00 | 10 | $D498, $EB68, $EB85 GetNameTable, $EBD9, $EBEE, $EC1E …+4 |
| S1-01 | 10 | $D806, $D821, $D834, $D83A, $D86D, $D88F …+4 |
| S1-02 | 9 | $80B0, $F3D8, $F503, $F55C, $F56C, $F5AC …+3 |
| S1-03 | 8 | $F62E, $F649, $F68D PlaySnd3, $F6A4, $F8A2, $FA0A …+2 |
| S1-04 | 7 | $F3D5, $F628, $F682 PlaySnd3, $F699 PlaySnd3, $F7D9, $F844 PlaySnd3 …+1 |
| S1-05 | 6 | $CD6D SetSamusData, $D052, $D11B, $D144 SetSamusRoll, $D1CE, $D41C |
| S1-06 | 6 | $C2C5 Amul16, $EB56, $EB96, $EBC1, $EC81, $EE2B PowerUpHandler |
| S1-07 | 6 | $F284, $F2D5, $F2E8 Xminus16, $F300, $F332 Exit22, $F340 Exit22 |
| S1-08 | 6 | $F48A, $F4DF, $F52D, $FA18 KillObject, $FA77, $FA91 KillObject |
| S1-09 | 5 | $CF6B SetSamusAnim, $D07A, $D088, $D12B, $D8B4 |
| S1-10 | 5 | $D10A, $D633, $F924, $FCED, $FD8F Table18 |
| S1-11 | 5 | $D400, $D40E, $D447, $D465, $E249 ToggleSamusHi |
| S1-12 | 5 | $C3D4 TwosCompliment, $D57A, $F7A7, $F9D9, $FB9C |
| S1-13 | 5 | $D580, $D609 Table0D, $D694, $D6A7 Table0D, $D79F GetObjCoords |
| S1-14 | 5 | $D6BF, $D6E1, $D71A, $D761, $D78B Table0D |
| S1-15 | 5 | $D6ED, $D70F, $D735, $D746, $D77F Table0D |
| S1-16 | 5 | $EBCB, $EE14 PowerUpHandler, $EE4A CheckForItem, $EEB9, $EED7 |
| S1-17 | 4 | $D012, $D116, $D147 StopVertMovement, $D3F4 |
| S1-18 | 4 | $CF55 StopHorzMovement, $D04D, $D110, $D957 |
| S1-19 | 4 | $D488, $D66D, $D8BC, $DE47 AnimDrawObject |
| S1-20 | 4 | $D6E4, $D712, $D73E, $D76A Table0D |
| S1-21 | 4 | $E789, $E792 GetRoomNum, $E906, $F91F |
| S1-22 | 4 | $EAD4 EnemyLoop, $EB09 LoadEnemy, $EB8F, $EC54 |
| S1-23 | 4 | $EDD6 ChooseHandlerRoutine, $EE83, $EEAB, $EEF7 |
| S1-24 | 4 | $F0C9, $F0DF, $F140 FillRoomRAM, $F1BF DistFromEn0ToObj1 |
| S1-25 | 4 | $F0CF, $F112, $F1EE, $F8F0 |
| S1-26 | 4 | $CE84 IsSamusDead, $F0DA, $F101, $F11B |
| S1-27 | 4 | $F10C, $F146, $F1FA Xminus16, $FEBD |
| S1-28 | 4 | $F120, $F143, $F186 GetObject1CoordData, $FEAA |
| S1-29 | 4 | $F3E3, $F40A DoOneEnemy, $F43E DoOneEnemy, $F536 PickupMissile |

## 2. asm 锚预消化（S1 定位锚）

- 全员 bank7（$C000-$FFFF 固定引擎面）——nmikstas 标签消解率 70%+
  （?=区内中间标签，随移植按指令流细化）；
- 代表锚：S1-05 **SetSamusData**($CD6D，Samus 控制数据/动画设置——移动链共享根)
  /S1-06 **Amul16**($C2C5，×16 移位工具例——全库通用)；
- 顺序尾段附着（task-43 口径）：836 段不在调用边上——随执行序归入所属核。

## 3. 结构互核注

- 簇边界=调用连通（632 边 union-find）——**跨区边未显式**（bank1-5→bank7 引擎
  调用未入集——S1 批移植时跨区依赖表随批补）；
- 语义命名标签 1293=候选单元名线索；每批验收=L2 增量+L3 回归+全案绿。

## 4. 红门开放项收口（随窗——T45 余件全收）

**碰撞写入点定谳**：IsBlastTile($E9BC)——
```
ldy UpdtngPrjctl / beq exit   ← 门 1：仅投射物更新期
jsr $95BD / cpy #$98 / bcs exit ← 门 2：瓦值 <$98=可爆破（$4E 同族）
寻空槽（TileRoutine=0，$C0 起）→ inc TileRoutine,x ← 轰开触发写
TileWRAMLo/Hi=$04/$05 & $DE ← 瓦 WRAM 址（位掩码 $DE）
TileType: InArea==$11 特例=4；else (瓦值+$10)&$3C >>3 ← 瓦值→动画型公式
```
**TileType 表定谳**：公式即语义（6 档）；Table19[10]=[0x18,0x1C,0x20,0x00,0x04,
0x08,0x0C,0x10,0x24,0x14]=TileBlastAnim 帧组索引。
**红门链全通**：导弹（UpdtngPrjctl 态）×门瓦(<$98)→IsBlastTile→5 相状态机→门开。

## 5. 遗留

1. 标签 30% 待移植时按指令流细化；
2. 跨区依赖表（S1 簇×区域调用点）随批补；
3. Table19↔TileBlastAnim 帧组逐项对表（0.25d 随件）。
