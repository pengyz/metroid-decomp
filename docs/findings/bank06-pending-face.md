# findings：bank0/6 待补面定谳（R1 批开闸前置——簇基数终值 388）

> 状态：**待补面定谳**（2026-10-19，task-43 遗留收口）。复现：decode_sweep 岛况
> +nmikstas Bank00/06 调用点补扫→并查集重计。
> 复现命令：`python3 tools/decode_sweep.py --json /tmp/sweep.json` + 补扫脚本
> （本档 §3）。

## 1. bank0/6 岛况实录（decode_sweep 285 岛中 122 岛）

| bank | verdict | 数 | 说明 |
|---|---|---|---|
| 0 | data | 87 | 含 $A200-$C000 7680B 大数据段（表/图形）——真数据 |
| 6 | data | 34 | 同型 |
| 6 | 未决 | 1 | $8C84-$8CFB（119B，cov 0.008，入口 fallthrough@$8C82——疑码） |

## 2. 补簇定谳（+N=1——基数终值 388）

- 补扫口径=nmikstas Bank00/06 的 JSR/JMP **同 bank 边**（$8000-$BFFF 窗内+
  bank0 上半 $C000+）——与 task-43 632 边同口径；
- 补扫实得：**1 条边→+1 簇**（387→388）；
- **判**：bank0/6 调用结构=**跨区跳板主导**（$8000 窗 JMP $Fxxx 尾调 bank7——
  cross-bank-thunk-abi.md 90 桩同族）——同 bank 边天然稀少；task-43 预估
  「+N<50」上界保守，实值 N=1；
- 那 1 条新边所在簇=bank0 代码区（ProcessUniqueItems 族内部调用）。

## 3. 复现命令

```sh
python3 tools/decode_sweep.py --json /tmp/sweep.json
# 补扫：nmikstas Bank00/06 的 JSR|JMP 目标 ∈ 同 bank 窗 → 并查集并边 → 重计
# （task-43 union-find 同法——call_sites (bank,addr)↔(bank,target)）
```

## 4. R1 批开闸条件

- **簇清单终形=388 簇**（387+1）——施工图 R1 批（bank0/6 数据资产域+补簇 1）
  可开工；
- R1 内容焦点（施工图 §2）：bank6 全数据段（parse 面已好——game-data 集成）
  +bank0 补簇 1（ProcessUniqueItems 族）；
- bank6 未决岛（$8C84-$8CFB）随 R1 移植时现场判（119B——入口 fallthrough 补证）。
