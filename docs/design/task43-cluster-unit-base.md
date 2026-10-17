# task-43：banks0-6 代码簇聚类定尺寸——单元基数定谳（milestone C 排程输入）

日期：2026-10-17 ｜ 方法：asm-first（nmikstas 标签锚+m1disasm 对照+调用图聚类）｜
判据：复现命令+真单元数

## 结论（定谳）

**真单元数=387 调用连通簇**（1019 节点/632 JSR/JMP 边）——**~200-400 假说确认**
（区间上沿 387）。域归属：

| 域形 | 簇数 | 归属 |
|---|---|---|
| 纯 bank7 | 181 | 共享引擎单元（SoundEngine/敌人/调色板等公用面） |
| 纯 bank1-5 | 206（44/36/45/43/38） | 区域专属单元（brinstar/norfair/tourian/kraid/ridley 尾段） |
| 纯 bank0/6 | **0（待补面）** | 尾段调用点未入集（岛 verdict data/未决 或目标未解析） |
| 跨区 | 0 | 调用边均为区内（decode_sweep 岛内解析口径） |

规模分布：2-3 节点簇 330｜4-10 节点簇 57｜无单点/无 >10——**小簇为主**（每指令
标签切分的自然形态聚合后）。

## 与 1855 段的关系

1855=nmikstas 逐指令标签段数（每指令切分，均长 8-9B）；387=调用连通聚合核。
段→单元的附着：调用核周边的顺序流段（836 段不在调用边上）随执行序归入所属核
（顺序附着不产生新单元）——**移植排程基数取 387**（核）+顺序尾段附着清单随
执行序细化。语义命名标签 1293（五 bank）=候选单元名线索（一单元多标签/数据标签
含于其中，非 1:1）。

## 复现命令

```sh
python3 tools/decode_sweep.py --json /tmp/sweep.json   # 基线 PASS + 285 岛/632 call_sites
# union-find over call_sites (bank,addr)↔(bank,target)：387 簇
```

## 口径与遗留

- 调用边来源=decode_sweep 岛内 JSR/JMP 静态解析（A 级字节）；动态调用序（task-12
  execpath）可对照加固；
- bank0/6 调用点待补（其岛 verdict= data/未决 居多）——基数可能 +N（预计 <50）；
- 跨 bank 调用（bank1-5→bank7 引擎）在 call_sites 同 bank 口径下未显式——跨区
  依赖表随排程补。

Evidence: decode_sweep JSON（285 岛/632 call_sites/基线 PASS）/union-find 387 簇
输出/nmikstas 标签计数（4391/4259/5182/4258/4233 L 标签+1293 命名）——本账内
Confidence: 90
