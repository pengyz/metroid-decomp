# 归档探针（不作对拍矩阵成员）

`tools/l4_diff_run.py run-all` 只遍历 `cases/*.txt`（非递归），本目录的用例不参与矩阵。

## 为什么放这里

`run-all` 有一条硬约束：**用例无采集点即拒跑**（分母静默缩水防护）。
下面两件是调查性探针，从未有 `capture`/`endFrame` 指令，留在 `cases/` 会让整个矩阵无法生成
（实测：2026-11-14 全矩阵被这两件挡住，矩阵产物停留在前一日的旧文件）。

| 文件 | 用途 |
|---|---|
| `probe-airjump.txt` | 空中跳取证（站立跳 status=2 的 SamusJump 径；进游戏后原地起跳、空中再按两次 A）|
| `probe-airjump2.txt` | 同上第二变体 |

要重新启用其中一支：给它加 `capture`/`endFrame` 采集点，再移回 `cases/` 并从 `data/status/l4_ratchet_baseline.json`
走 sanctioned 路径登记新格。
