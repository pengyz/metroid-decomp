# fade-blink FC 奇偶翻格根修 证据包（2026-11-03，lead 派复核用）

> 任务：**boot 帧计时初始化对齐根修**（FC 奇偶 flaky 族共因根）——先定位非确定性源，再修。
> 交付：三分判别定谳 + C 径根修 + B 径 flaky 处置 schema + 3 连跑 L4 判据验证。
> 定谳详录：[fade-blink-capture-race.md](fade-blink-capture-race.md)（A 级实证链）。

## 0. 一句话结论

淡入窗（SamusIntro :C9D7-:CA13，Timer3<31 窗 FC&1 奇帧画 Samus）FC 奇偶翻格
=**emu 侧 `emu.takeScreenshot()` 在 endFrame 帧界回调内读到未翻转的渲染面**
（vblank 起点渲染面翻转竞态；同回调双读实录 read1 旧像/read2 真值像）。
**rust 确定、emu RAM/state 面确定、配对层无罪**——「boot 帧计时初始化偏移」共因
假说**证伪**（翻格 run 与正常 run 的 FC/SpriteRAM/state 逐字节 0 差）。
修径=C 径（Mesen harness 帧对齐采样）+B 径（flaky 态 schema 留痕机制）。

## 1. 三分判别（任务 Step 1——先证后修）

| 判别面 | 实验 | 结果 | 裁定 |
|---|---|---|---|
| ① rust 确定性 | rust_screenshot 3 连跑（scrollseam 同 .sched） | f560/f700/f760 png sha+ram $2D（21/AD/E9）**3 次恒同**（≡run19:34 产物） | 纯代码确定（应然） |
| ② emu 确定性 | 226 归档 run 考古 + 3 连跑隔离 + 62 点×6 run 密集探针（3 干净 3 满载） | 翻格 run 实存（killenemy/scrollseam/jumpledge）；密集探针 5/62 格翻+**同回调双读 4 次分歧** | **emu png 面非确定** |
| ③ 配对层 | 翻格 run vs 正常 run 产物对账 | ram.hex 逐字节 **0 差**（含 SpriteRAM）；matrix fresh/fresh 同 run 对 | 无罪 |

- 双读实录（机制铁证）：run4 f645 read1=drawn-s2/read2=noSamus(真值)；run5 f750、
  run6 f680/f690 同型——**同回调 µs 间隔两读不同帧图像**，后读=FC 真值像。
- 帧锚实测：endFrame 回调=ppu scanline 241 c25-33（vblank 起点±ε=竞态窗内）；
  spin-exit 缝（$C0D3）在 NMI 例程完成后（窗外数千周期）。
- blink 真值面：png 变体完全由 parity(FC)×调色板级决定（jumpledge 6/6 锚）；
  翻格=该真值被采成对侧变体（含 noSamus↔drawn 双向）。

## 2. 修径与交付件

### 2a. C 径：采集截取缝化（主修——根因面）

| commit | 内容 |
|----|---|
| `03fb9458` | docs：定谳档（三分判别+机制+假说证伪更正注） |
| `fe3207d2` | test：TDD 红态锚——采集截取缝契约（lint_seam：endFrame 直读检出/缝兑现要求） |
| `2f7c1776` | fix：截图兑现移至 spin-exit 缝（$C0D3，prg7:218）——requestShot 登记/fulfillShot 稳定双读（≤8）/SHOT-SEAM·SHOT-UNSTABLE·SHOT-SEAM-MISS 留痕/末帧 DONE 顺延+兜底 |

- 语义保持：png(N)=帧 N 渲染像不变（只移读取点出竞态窗）——L4 基线值语义不变 ✓
- 首跑实证：密集探针 62 点 SHOT-SEAM×62 兑现、0 漏缝/不稳定；FC 奇偶真值对表
  **61/61 窗内格一致**（f830 T3=0 游戏屏=blink 真值面外）——旧径同探针 5/62 翻格。

### 2b. B 径：flaky 态落 l4 锚 schema（处置机制）

| commit | 内容 |
|---|----|
| `146b5ede` | feat(tools)：多 run 观测账（每格每 run 一行）+classify_flaky（判据=≥3 run 观测且 verdict 不一致）+矩阵处置（flaky 格 state=flaky+观测记录留痕、**不计分母不锚值**）+baseline_update 不锚 flaky |
| `c0290248` | gate(l4)：ratchet flaky 豁免（①③对 flaky 格）——**普通回退照判**（防弱化面钉死）；基线影响=无锚值变化 |

## 3. 判据验证

| 判据 | 结果 |
|---|---|
| 淡入窗格（killenemy f560/f590/scrollseam f560/f700/f760/jumpledge f730）3 连跑 L4 稳定 | **6/6 格 ×3 连跑全 match**（run#1=pre-commit 门禁内 run-all；run#2=独立跑；run#3=flaky-aware 跑）——逐格明细见证 §4 |
| 943/1383/445 帧 state 0 差基线保持 | **不触**（sampleTrace/state 行径零改动；本件纯截图读取面。walk-fc-capture-seam §3 基线原样） |
| 475 测全绿+gates check | cargo test 479 全绿（每 commit pre-commit 门禁内实跑）；新 pytest 11 件全绿（seam 3+flaky 8）；gates check=PASS（c0290248 件） |
| L4 无回退（flaky 格按 B 留痕） | 3 连跑 flaky 集**为空**（观测账 0 格不一致）；ratchet match 57 守住（每 commit 门禁实跑）；B 机制在位待命 |
| animI/f1200 收敛（加分项） | **不随本件收敛**（内容真差非采样翻格——定谳档 §2 更正注已注记；原案保留） |

## 4. 3 连跑淡入窗格逐格明细

| 格 | run#1 | run#2 | run#3 |
|----|----|---|---|
| killenemy f560 at-enemy | match | match | match |
| killenemy f590 hit-1 | match | match | match |
| scrollseam f560 scroll-r120 | match | match | match |
| scrollseam f700 scroll-end | match | match | match |
| scrollseam f760 scroll-l40 | match | match | match |
| jumpledge f730 rise | match | match | match |

矩阵基线（每 run）：match 57 · diff 51 · blocked 0（108 分母）——与 r14 锚一致，无回退。

**观测账增证**（l4_run_observations.jsonl）：3 run × 108 格 = 324 行（run 21:24/21:31/21:39），
classify_flaky 判定 flaky_cells=[]——**全 108 格 3 run verdict 恒同**（不止淡入窗格），
flaky 处置面零触发（机制在位待命）。

## 5. 复现命令（A 级可复现指针）

    # 三分判别实验产物：analysis/boot-det/（rust-1..3 / emu-1..3 / probe-1..6 / seamtest）
    # 定谳档 §4 复现命令逐条可跑
    # 验证：python3 tools/l4_diff_run.py run-all  ×3（淡入窗格逐格核对 matrix.md）
    # 契约测试：python3 -m pytest tools/tests/test_capture_seam.py tools/tests/test_l4_flaky.py \
    #           tools/tests/test_l4_flaky_ratchet.py

## 6. 遗留/移交

- **rust 腿 run 隔离戳缺位**（F 卡姊妹面）：shots_rust/ 原地覆盖无归档（emu 腿有
  .archive/<stamp>）——本次判别靠 3 连跑自证不受阻；建议 F 卡补 run 戳（产物带 run id）。
- test_l4_ratchet.py 5 处预存红（v2 A-lite states schema 漂移）+test_expand_schedule 2 处
  （case-001 joy hash 漂移）——clean 树同红（非本件面，另案登记）。
- rust_test_count 基线 PENDING（459→479——r14 H1/H2 测试增量）：待 baseline update
  （--reason 留痕）；非本件面。
- dumpPpu 尾 3B 恒 00 仪器面（walk-fc-capture-seam §2c 挂账）：仍挂账 harness 修。
