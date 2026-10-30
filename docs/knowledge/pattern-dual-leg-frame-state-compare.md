---
type: pattern
title: 双腿逐帧状态对拍——TD 仪器+机械对拍（帧级分歧定位的标准方法）
date: 2026-10-30
status: confirmed
---

# pattern：双腿逐帧状态对拍（TD 仪器+机械对拍）

## 方法（四步，全部机械化——手工推演在本域已三次证伪，禁止）

1. **选状态计数器（TD 仪器）**：找待测机制**直接读写**的状态字节/计数器（例：
   prescaler $29、帧计数 $2D、delay 计数器），双腿用例各加 watch（emu `watch 0029
   TimerDelay`；rust 腿 state.jsonl 的 zp/iram 全量自带）。判据：该计数器由机制的
   调用/节拍驱动 ⇒ 其逐帧时间线就是**调用/跳帧集的直接录像**。
2. **声明采样缝**：用例显式 `sample_point spin-exit`（或 endFrame——但**跨腿对比必须
   同缝**，见 gotcha-frame-compare-sampling-staleness）。双侧采样缝不同 ⇒ 行比较无定义。
3. **机械对拍**：`python3 tools/frame_seq_diff.py --emu <frames.jsonl> --rust
   <state.jsonl> --field NAME=EMU_KEY:RUST_SPEC ...`——固定归因（row[N]→row[N+1]
   差分记帧 N）、逐字段输出首分歧/分歧帧清单/缺行集。**禁止手工帧算术**（采样沿
   语义已三次干扰推演）。
4. **跳帧集=结构指纹**：TD 恒定帧集与 asm 例程的 WaitNMIPass_/内旋结构一一对应——
   跳帧集差异即臂结构差异，按帧图逐臂整形后重测收敛。

## 探针模板（新探针最小集）

```
name case-xxx-probe
frames <N>
snapshot none
sample_point spin-exit      # 跨腿对拍必须；缺省 endFrame 与 rust 缝错位
watch 002D FC               # 待测计数器
watch_range <起>-<止> <标签> # 可选：结构域（敌槽等）
press ...                   # 与被测流程同相的输入
capture <f> <tag>           # ≥1 个采集点（I1 拒空）
endFrame <N>
```

## 何时使用

- 帧级时序分歧（激活时机/prescaler 相位/fade 步进）的首分歧定位；
- 例程内旋/跨帧结构的臂形整形验收（改臂→重跑→对跳帧集）；
- 任何"双轨产物逐帧状态序列"的对拍（Mesen lua watch vs rust state 快照）。

## 实例（本方法首次全程应用）

- 爬虫激活差 3 帧：TD 仪器 951 帧 → TD 相位差自 f2（boot 前移 1 帧根因）+ 转换臂
  内旋帧数差 → GameEngine 进入差 3 帧闭合——docs/findings/crawler-corner-divergence.md
  与 frame-alignment-plan.md（Step 0-6）。

## 证据指针

- `tools/frame_seq_diff.py`（机械对拍器，--selftest 自证）
- cases/case-crawlprobe.txt / case-fcprobe.txt（探针实例）
- docs/findings/crawler-corner-divergence.md（951 帧×5 字段分歧地图）
- docs/findings/frame-alignment-plan.md（§1 采样缝错位定谳）
