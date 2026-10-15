---
type: gotcha
---

# gotcha：静默 0 类缺陷要类级清扫——修一个实例 ≠ 修一类

## 现象
P7-F2 修了 `decode_sweep_dyn` 的「traces 前缀不存在静默按 0」后声称修复——但**同一类
缺陷在姊妹工具里原样健在**：`call_graph --repeat` 第二腿路径错误时静默输出「跑2 0」，
复审方按错误路径实测当场命中（P12-F1）。「空跑绿」家族：**空数据与真 0 在输出上不
可区分**，路径错/工件缺/时序竞态都会伪装成「结果为 0」。

## 教训
- **修一个实例≠修一类**：静默 0/空的修复必须伴随**消费点全量审计**（grep 同类模式：
  `is_file()` 后静默 continue/return None/[]、`json.loads` 前无存在性断言、
  「文件不存在就当空」的 except 吞掉）——逐点负测试（传错误路径看是否 FATAL）
  才算类级关闭；
- 统一语义：**显式输入路径不存在 = FATAL**（`require_file` 一类助手），设计性可选
  输入（默认缺省=跳过）显式区分并注明；
- 与 gotcha-negative-assertion-time-race 同族：「0/空」类结论先问「输入真为 0 还是
  根本没读到」；与 gotcha-policy-docstring-not-enforced 同族：**声称的修复要可复现**
  ——负测试清单（错误输入→期望 FATAL）入册才可信。

## 复现（2026-10-15，P12-F1 实测 + 修复后 8 项负测试）
```sh
# 修复前（静默 0）：
python3 tools/call_graph.py merge --run <ok> --static <ok> --repeat <ok> /nonexistent/run2
#   → 「跑1 43 跑2 0」（假绿，工件缺失不可见）
# 修复后类级清扫（8 项负测试全 FATAL，清单见 docs/findings/execution-call-graph.md §7）：
#   T2 → FATAL: traces 前缀无任何产物文件: /nonexistent/run2.entries/.edges.jsonl
```

## 高危同族
- 任何「文件不存在就当空集继续」的数据加载点（trace/矩阵/清单/台账）；
- 双跑/多腿对照的每条腿都要等强度校验（单腿 FATAL、另一腿静默=半套防护）；
- 合并/聚合工具的分母为 0 输出——先证输入存在。
