# findings：S7 批产取证打包（共享引擎 176-181 簇——同 S1 模板）

> 状态：**S7 证据预备仓就位**（2026-10-18）。跨区=m1disasm 名调用匹配法
> （S2 定型）；标签=m1disasm 名标签双源。
> 复现：`python3 tools/decode_sweep.py --json /tmp/sweep.json` + union-find + 切片。

## 1. S7 簇清单（6 簇/12 员）

| 簇 | 员数 | 成员（bank7:addr+名标签） |
|---|---|---|
| S7-175 | 2 | $FA2A, $FA7D GetEnemyRoomRAMPtr |
| S7-176 | 2 | $E96A MakeRoomRAMPtr, $FA8E |
| S7-177 | 2 | $F693 ClearEnAnimDelay, $FBBC |
| S7-178 | 2 | $FCC1 UpdateMellow_RunAI, $FD5F UpdateMellow_StorePositionToTemp |
| S7-179 | 2 | $FCFA, $FD6C UpdateMellow_LoadPositionFromTemp |
| S7-180 | 2 | $FE95, $FF3C GetPosAtNameTableAddr |

## 2. asm 锚预消化

- 名标签消解 7/12（m1disasm 双源）；残余=区内中间标签（随移植细化）；
- 簇规模特征：辅助例族（均 3 员）。

## 3. 跨区依赖表（S7×area）

| 目标 | 名 | 调用方 |
|---|---|---|
| — | — | 无跨区调用 |

## 4. 结构互核注

- 簇边界=632 边 union-find（task-43 口径）；顺序尾段附着清单随执行序细化；
- 每批验收=L2 增量+L3 回归+全案绿（施工图 §2 口径）。
