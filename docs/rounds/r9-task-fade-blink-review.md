# R9 对抗复核 verdict——fade-blink 竞态根修 6 commit（03fb9458/fe3207d2/2f7c1776/146b5ede/c0290248/bec01494）

- 复核员：verifier-scroll（证伪立场，非实施方）
- 日期：2026-11-03
- **verdict：pass**（附两处小注记）

## 1. 竞态定谳核（最重） —— PASS

- **证据链 A 级且可复现**：①226 归档 run 考古（翻格 run 实存）②62 点×6 run
  密集探针（干净/满载双组，5/62 格翻且集中在满载组）③**同回调双读 4 次实录**
  （read1≠read2 且后读=FC 真值一致像；run6 f675 双读同翻=窗可跨双读⇒「双读
  一致」不充分的反例意识在案）。本复核验证判据面：blink=parity(FC at capture)
  6/6 逐点映射（§1b 表）+翻格 run vs 正常 run ram.hex 逐字节全同（§1a——state
  面无罪、png 面单侧现象）✓。
- **帧锚论证**：endFrame(N) 回调 ppu scanline=241 cycle≈25-28（vblank 起点竞态
  窗内）vs spin-exit 缝（$C0D3，NMI 后数千周期）窗外——与双相位模型（png(N)≡
  on_spin_exit 缝）对齐 ✓。探针可复跑（saveShot 双读留痕机制在案）。

## 2. 根修正 —— PASS

- 实现实读（scenario_capture.lua）：endFrame 采集块只 **requestShot 登记**
  （不读渲染面）；**fulfillShot** 于 spin-exit 缝兑现（:699）；稳定双读
  （SHOT_STABLE_MAX=8，两次一致才落盘）+SHOT-SEAM/SHOT-UNSTABLE 显式留痕；
  **SHOT-SEAM-MISS+即时兜底读**（:214-215，不静默丢帧）+末帧 frames+1 兜底
  （:751）——兜底路径完备 ✓。
- png 语义不变论证：缝位=on_spin_exit(tick N-1)——与双相位模型 png(N)≡缝
  同语义（帧 N 渲染像），非语义切换 ✓。

## 3. flaky schema + gate 件 —— PASS（无弱化）

- 判据：classify_flaky（≥min_runs=3 次观测且 verdict 不一致）——观测账唯一
  依据源（l4_run_observations.jsonl，**fail-loud 损坏抛错** :780）✓。
- gate 件（c0290248）防弱化审计：豁免面仅 ①match 缩水/③未锚定 对 **已分类
  flaky** 格（需 N-run 不一致证据账支撑）；**普通回退（match→diff）照判 FAIL**
  +②blocked+rom 锚漂移原样——ratchet 未削弱（test_l4_flaky_ratchet 钉死）✓。

## 4. 判据复跑 —— PASS

- **本复核独立 3 连跑**（run1/2/3 fade 窗 10 格：jumpledge 550/730/755/780/800+
  scrollseam 560/590/700/760/800）：**三 run 逐格全同（diff 空="3-RUN STABLE"）**
  ——9 match/1 时序残（800 段已知族），淡入窗翻格已消 ✓。
- 观测账 800 行（含 lead 3 run×108=324 行 run 21:24/21:31/21:39 增证）；
  matrix **flaky_cells=[]** ✓（0 格不一致——B 机制在位待命）。
- `cargo test --workspace`：**475 passed / 0 failed**；`gates/cli.py check` 干净
  顺序复跑：**PASS=13 FAIL=0，CHECK: PASS**（本复核首次 check 与并行 run-all
  竞态报 l4 FAIL——同 r9 已登记 artifact，非产品分歧）。
- sampleTrace 零改动核：rust 3 连跑 png sha 恒同（定谳 §0 判别 1）+三场景
  ram.hex 跨 run 逐字节全同（§1a）——943/1383/445 基线不触 ✓。

## 5. 预存红划界 —— PASS（计数小注记）

- clean 树（69279292 worktree）同红复现：test_l4_ratchet 红 + test_expand_
  schedule 2 红（同名单）——**非本件引入**确认 ✓。
- **小注记**：证据包记「test_l4_ratchet 5 处」vs 本复跑全量 **3 failed/31
  passed**——计数差（可能含参数化口径/时点差），material 主张（预存非引入）
  不受影响；建议证据包计数以 pytest 输出为准修正。

## 6. rust_test_count PENDING —— 确认

- gates 输出显式打印：`基线 459 / 现值 479 —— PENDING（上调并 commit 说明）`
  ——待办确认（未静默通过）✓；基线仍待 baseline update（登记在案）。

## 裁定

| 点 | 结论 |
|---|---|
| 1 竞态定谳（A 级链+可复现探针+帧锚窗外论证） | pass |
| 2 根修正（request/fulfill 拆分+SEAM/UNSTABLE/MISS 兜底完备） | pass |
| 3 flaky schema+gate 件（判据+防弱化） | pass |
| 4 判据复跑（3 连跑逐格稳定+flaky_cells=[]+基线不触） | pass |
| 5 预存红划界（clean 树同红；计数 5 vs 3 小注记） | pass |
| 6 rust_test_count PENDING 确认 | pass |

**verdict = pass**。

Evidence: docs/findings/fade-blink-capture-race.md（三分判别+双读实录+帧锚）；
scenario_capture.lua :142-215/:699/:751（修径 C 实读）；tools/l4_diff_run.py
:742-790（观测账/classify_flaky）+gates/cli.py :1132-1135（flaky 豁免面）；
本复核 3 连跑 fade 窗 10 格逐格对照（/tmp/fade_cells_run{1,2,3}.txt，diff 空）；
shots_rust/l4_run_observations.jsonl（800 行）；matrix flaky_cells=[]；
clean 树 69279292 worktree 同红复现；cargo test（475/0）；gates check（干净
复跑 PASS=13 FAIL=0）。
