---
type: gotcha
---

# gotcha：否定性断言有时效竞态面——换工具之外还要「换时刻」复核

## 现象
复核方 check 时点看到 docs-links 门禁红（报告引用的 knowledge 条目不存在，双工具
核实「确实不存在」——负向断言当时**成立**）；数分钟后同一命令自愈转绿——目标条目
在两次检查之间被落库。负向断言（「X 不存在 / 0 命中 / 门禁红」）的真值**随时间变化**，
单时点的双工具复核仍可能同时踩中同一竞态窗口。

## 教训（与 gotcha-listing-grep-not-rom-scan 成对）
- 否定结论三查：**换工具**（ROM 扫描≠listing grep）+ **换时刻**（本条）+
  **记录时点**（断言附 何时/何命令/何状态）；
- 竞态窗口典型场景：共享工作树多 agent 并行写 × 全树扫描门禁（本例：一方落库 vs
  另一方 check 交错）；CI 与本地的时点差同理；
- 「不存在」类结论的可复现性声称本身要可复现（与
  gotcha-evidence-grep-needs-fixed-string 同族）——审计侧看到「不存在/红」时先问
  「哪个时刻、谁在写」。

## 复现（2026-10-15，案情原文见 docs/rounds/r7-decode-sweep-review.md method 面）
```
t0: decomp 建 docs/findings/decode-sweep-report.md 引用
    ../knowledge/gotcha-policy-docstring-not-enforced.md（先引用后建目标——本体违规，
    教训：先建目标后建引用）
t1: verifier check → docs-links 红；双工具核实目标不存在（负向断言成立）
t2: decomp 落库 gotcha 条目
t3: verifier 复核 → 自愈；撤回「不存在」误报并记账本条教训
```
门禁红在 t1 为真、目标在 t2 后存在——两句都对，靠**时点**区分。

## 高危同族
- 共享工作树多 agent 期间任何「缺失/不存在/0 命中」类核查；
- 把瞬时红当持久红去「修复」（可能修一个已经不存在的问题，或误伤他人在途工作）。
