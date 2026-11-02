# R9 对抗复核 verdict——6c7214da（B0/B1：l380 回归坐实定谳）

- 复核员：verifier-scroll（证伪立场，非实施方）
- 日期：2026-11-02
- **verdict：pass**（附一处证据归档 should-fix）

## 1. B0 复现 —— PASS（附本复核独立旁证）

- HEAD l380=**12085px×5/5 零方差**（findings §7 在档）——确定性真值、±44px
  方差带不适用（非 flaky）论证成立：方差带源自 emu 腿重采相位（双侧 fresh
  才出现），单腿 rust 渲染确定性下 5/5 同值=真值。
- 本复核独立旁证：审计轮 fresh matrix 同读 **12085**（与本轮无关的另一次
  run-all）——≥2 独立观测同值 ✓。

## 2. B1 方法核（关键） —— PASS

- **worktree 钉版实证**：`git worktree list` = `/tmp/b1-0a99466f @0a99466f
  (detached)`；worktree 内 tools/l4_diff_run.py+scenario_capture.lua+rust bin
  均为该 commit 版本（整钉 ✓）；worktree 干净（仅 cases 双探针 untracked——
  即「现版 case」注入面）。
- **现版 case 验证**：worktree 内 case-leftwalk-probe2.txt sha256=b90418ab…
  ==主树现行 case（逐字节同）——「旧码×现版 case」交叉成立。
- **emu 金面共享真值论证成立**：emu 腿=主库共享 shots/（现行工具采集——旧
  tools 采不全 20s 死有案）+.l4prov.json 锚+调度 hash fail-closed
  （sched 场景 sha b90418ab 校验在案）——diff 比对仅像素级（PNG），工具演进
  不影响像素语义；旧码 rust 腿直渲由 **PNG sha 异同实证**（worktree 渲染
  49d6858d… ≠ 主树 4a8e8968…——非同一产物，确系旧码重渲）。
- **626 复现实读**：worktree l4report.json l380 `diff_pixels=626/61440`
  （且 l580=498——与 kick-restore 轮值逐点吻合，三重历史互证）。
- 可复现性：worktree 检出+主库 emu 金面+rust 腿 cargo run——载体链现全部
  可从 git 复原（0a99466f 为 commit、case b90418ab 已入库）——**审计「626
  出处=工作树未入库」疑点解除**。

## 3. 结论核 —— PASS

- 626（0a99466f）vs 12085（HEAD）=**回归坐实**（0a99466f..HEAD 98 commit 间
  ~19x 劣化）——判定链：emu 金面恒定（共享）×rust 腿双点（旧 626/新 12085）
  ⇒ 差异全部归于 rust 侧代码演进 ✓。
- 载体疑点闭环：B1 用入库 commit+入库 case 复现历史值——不可复现缺口消除。
- 下一步序（B2 watch→bisect 首查 5 嫌疑）合理（与本复核 R2/R9 修订意见一致）。

## 4. findings §7 归档 —— PASS（附 should-fix）

- §7 三段（B0 方差带/B1 真值+方法注/下一步序）齐、与实测一致 ✓。
- **should-fix（非阻塞）**：B1 判值工件（worktree l4report.json 626 记录+双侧
  PNG sha 对照）存于 /tmp worktree——**易失**。处方：将 l4report（或 §7 引用
  的逐字节摘要+sha 清单）拷入 analysis 归档或 docs 附录，防 worktree 清理后
  判值无载体。

## 裁定

| 点 | 结论 |
|---|---|
| 1 B0（5/5 零方差+独立旁证 12085） | pass |
| 2 B1 方法（钉版+现版 case+emu 共享真值+PNG sha 异同证旧码重渲；626 实读） | pass |
| 3 结论（回归坐实 ~19x；载体疑点闭环） | pass |
| 4 findings §7 归档（+B1 工件归档 should-fix） | pass |

**verdict = pass**。

Evidence: git worktree list（/tmp/b1-0a99466f@0a99466f detached）；worktree 内
tools 三件版本核（0a99466f 检出）；case sha 双侧同（b90418ab）；worktree
l4report.json l380=626/l580=498 实读；双侧 rust PNG sha256 异同（49d6858d vs
4a8e8968）；findings §7；本复核审计轮独立 12085 观测。
