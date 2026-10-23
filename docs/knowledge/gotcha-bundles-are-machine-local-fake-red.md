---
type: gotcha
title: analysis/ 机器本地态缺失伪造 L3 确定性红——判别程序与 bundle 恢复配方（三案实证）
date: 2026-10-23
status: confirmed
---

# gotcha：换机器后 L3 三连红 ≠ 确定性破裂（bundle/缓存系机器本地产物）

## 现象

新机器或清空 `analysis/` 后跑 `tools/diff_traces.py`：多个 L3 用例报
「oracle trace sha256 不符 manifest（**确定性破裂/缓存陈旧/篡改**）」——三义并列的告警极易
被误判为代码回归或跨平台确定性破裂（2026-10-23 实录三连红：boot-title / bundle-ff /
sc-elevator-ride）。bundle 类场景另见 `bundle load FAILED: …不可读——fail-loud` 与 0 行 trace。

## 为什么

- `analysis/` 是 gitignored **机器本地工作区**：trace 缓存与 `analysis/bundles/*.mss`
  （bundle_capture 落包——MSS+RAM+regs 快照）都不入库，换机即缺；
- 缺包时 harness 按「空跑不得绿」设计 fail-loud（error() 停脚本 / 0 行产物 / 跑满超时），
  但**告警文案把三种病因并列**——陈旧缓存与真确定性破裂同文案，须实验判别；
- manifest 演进（场景 DSL 升级+新锚点）会让旧缓存必然对不上——旧 sha ≠ 破确定性。

## 何时使用

**判别程序（先判别后修，禁直接改锚点）**：
1. 重采该场景 → 重采 sha == manifest ⇒ **缓存陈旧**（确定性完好，收案）；
2. 重采 sha 稳定 ≠ manifest ⇒ 才进「确定性破裂」调查（真红）；
3. 0 行产物 + lua.log `bundle load FAILED` ⇒ **缺 .mss 包**，按生产者场景重放恢复。

**bundle 恢复配方（2026-10-23 实测锚定）**：
| 包 | 生产者 | 落点 |
|---|---|---|
| `mid-move` | `bundle-capture` | f900 |
| `sc-elevator-ride` | **`sc-sweep-3550`** | f2400（=map-connectivity-graph.md §46 f3800 房 19 mapX=10 态的净序列等价落点） |
其余 `sc-*` 包=各同名场景自产（f950-f1450 落包，DSL 内 `bundle_capture` 行即生产表）；
`sc-sweep-3650..3950` 为时序变体，其 f2400 包**不是** manifest 锚点来源。

## 证据指针

- 三案判别实录（2026-10-23）：boot-title 重采 `4f9ca791…`=manifest（陈旧实锤）；
  bundle-ff 产 mid-move 后 `2b3911b83965…` 逐字节吻合；sc-sweep-3550 落包后
  `a78050ec6179…` 逐字节吻合（5 sweep 搜索首中）——L3 22 条 0 失败，门禁 STATUS: OK；
- 包出处链：`docs/spec/map-connectivity-graph.md` §46-47（①d00ed9a f3800 落点
  ②8183f37 消费场景 ③porter 帧数制终止修）；
- fail-loud 面：`tools/scenario_capture.lua` bundle_load 缺包 error()、
  `tools/scenario_run.py` bundle_capture/load 绝对路径解析（Mesen CWD≠仓库根）。
