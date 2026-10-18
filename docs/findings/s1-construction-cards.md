# findings：S1 批产施工卡清单（单元卡×验收面——Week 2 开工令预备件）

> 状态：**施工卡拆分定稿**（2026-10-18）。S1=共享引擎前 30 簇（S1 取证包
> s1-evidence-package.md 同源）——30 张单元卡（~15.5d 合计）。
> 验收面：L2 案对表（s1-pregen 152 案——到簇并入 l2_cases.jsonl 三方开启）
> +L3 回归（14 案全绿保持）。

## 1. 单元卡清单（30 卡）

| 卡 | 员 | 首址 | L2 案族（s1-pregen） | 估 |
|---|---|---|---|---|
| SC-S1-00 | 10 | $D498+9 | s1-00-case (0 案) | 0.75d |
| SC-S1-01 | 10 | $D806+9 | s1-01-case (0 案) | 0.75d |
| SC-S1-02 | 9 | $80B0+8 | s1-02-case (0 案) | 0.7d |
| SC-S1-03 | 8 | $F62E+7 | s1-03-case (0 案) | 0.65d |
| SC-S1-04 | 7 | $F3D5+6 | s1-04-case (0 案) | 0.6d |
| SC-S1-05 | 6 | $CD6D+5 | s1-05-case (0 案) | 0.55d |
| SC-S1-06 | 6 | $C2C5+5 | s1-06-case (0 案) | 0.55d |
| SC-S1-07 | 6 | $F284+5 | s1-07-case (0 案) | 0.55d |
| SC-S1-08 | 6 | $F48A+5 | s1-08-case (0 案) | 0.55d |
| SC-S1-09 | 5 | $CF6B+4 | s1-09-case (0 案) | 0.5d |
| SC-S1-10 | 5 | $D10A+4 | s1-10-case (0 案) | 0.5d |
| SC-S1-11 | 5 | $D400+4 | s1-11-case (0 案) | 0.5d |
| SC-S1-12 | 5 | $C3D4+4 | s1-12-case (0 案) | 0.5d |
| SC-S1-13 | 5 | $D580+4 | s1-13-case (0 案) | 0.5d |
| SC-S1-14 | 5 | $D6BF+4 | s1-14-case (0 案) | 0.5d |
| SC-S1-15 | 5 | $D6ED+4 | s1-15-case (0 案) | 0.5d |
| SC-S1-16 | 5 | $EBCB+4 | s1-16-case (0 案) | 0.5d |
| SC-S1-17 | 4 | $D012+3 | s1-17-case (0 案) | 0.45d |
| SC-S1-18 | 4 | $CF55+3 | s1-18-case (0 案) | 0.45d |
| SC-S1-19 | 4 | $D488+3 | s1-19-case (0 案) | 0.45d |
| SC-S1-20 | 4 | $D6E4+3 | s1-20-case (0 案) | 0.45d |
| SC-S1-21 | 4 | $E789+3 | s1-21-case (0 案) | 0.45d |
| SC-S1-22 | 4 | $EAD4+3 | s1-22-case (0 案) | 0.45d |
| SC-S1-23 | 4 | $EDD6+3 | s1-23-case (0 案) | 0.45d |
| SC-S1-24 | 4 | $F0C9+3 | s1-24-case (0 案) | 0.45d |
| SC-S1-25 | 4 | $F0CF+3 | s1-25-case (0 案) | 0.45d |
| SC-S1-26 | 4 | $CE84+3 | s1-26-case (0 案) | 0.45d |
| SC-S1-27 | 4 | $F10C+3 | s1-27-case (0 案) | 0.45d |
| SC-S1-28 | 4 | $F120+3 | s1-28-case (0 案) | 0.45d |
| SC-S1-29 | 4 | $F3E3+3 | s1-29-case (0 案) | 0.45d |

## 2. 验收面（每卡通用）

- **L2 三方对拍**：卡簇移植完成→s1-pregen 对应案族并入 l2_cases.jsonl→
  `golden == ROM 执行 == game-core` 全等（l2-diff 裁定）——11 失败簇（案族 0 案）
  卡=L2 面留白（跳板桩簇——入口=桩体 bank7 侧（task-14 消歧后补案））；
- **L3 回归**：移植后 `python3 tools/diff_traces.py` 14 案全绿保持（记账面照旧）；
- **L4 可选**：渲染面无涉（S1=引擎例——L4 二腿随 S6 系）；
- **勾格清单**（completeness 增量）：卡完成→矩阵格 `已实现→有 L2→已验证` 走级。

## 3. 卡序建议

- 卡序=簇规模降序（S1-00 起——大簇先立根）；**S1-05 SetSamusData/S1-06 Amul16
  建议前置**（移动链共享根/全库工具例——被调频顶）；
- 每卡估时 0.25-0.75d（员数线性）——30 卡 ~15.5d（与施工图 S1-S2 合计 2d 对表）。

## 4. 复现命令

```sh
# 卡清单源
cat oracle/subroutines/s1-pregen/funcs.jsonl | wc -l   # 30 簇案族
# 每卡验收
cargo run -p game-oracle --bin l2-diff -- --cases oracle/subroutines/l2_cases.jsonl
python3 tools/diff_traces.py   # L3 回归
```
