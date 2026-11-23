# FC 对齐房扫（FrameCount-aligned room sweep）方案

- 日期：2026-11-23
- 状态：**方案（未执行）**。本文件是本轮唯一写入；仓库代码、台账、门禁**零改动**。
- 角色：decomp-analyst / verifier 交叉（对抗复核见 §4）
- 影响面：`tools/movie_ref/room_sweep.py`、`tools/l4_diff_run.py`、`tools/scenario_capture.lua`（只读确认，不改）、`crates/game-core/src/bin/rust_screenshot.rs`（只读确认，不改）、`tools/tests/`、`data/status/room_sweep_*.json` 的 schema。

---

## P0 结果（2026-11-23 实测，go/no-go = **GO**）

四支探针（`cases/case-p0-*`，跑完即删）：`a0_r02`（shift1 族）与 `a2_r31`（掉帧族），
各跑 `sample_point endFrame` 与 `spin-exit` 两版，120 帧窗 + 一支 170 帧长窗；rust 侧用
`--state-out` 逐帧读 `zp[0x2D]`，emu 侧读 `analysis/traces/<name>.frames.jsonl` 的 `FC`。

| 探针 | 窗口 | Δ = FC_emu − FC_rust | 同 FC 相邻帧 |
|---|---|---|---|
| a0 r02 end | 920–1005 | 恒 **−1**（85/85 帧） | 0 |
| a0 r02 spin | 920–1005 | 恒 **−1**（85/85 帧） | 0 |
| a2 r31 end | 1620–1700 | 恒 **0**（80/80 帧） | 0 |
| a2 r31 spin | 1620–1700 | 恒 **0**（80/80 帧） | 0 |
| a2 r31 spin（长窗） | 1620–1775 | 0（1620–1704）→ **−1…−8**（每 5 帧降 1） | **0** |

三问逐条回答：

1. **Δ 是否房间无关常量？** 不是全局常量，但**在每个房间内是分段常量**（长窗里只在丢帧帧处跳变，
   两次跳变之间恒定）。a0 r02 的恒 −1 与 a2 r31 的前段 0 说明：跨房差异来自**载房窗的
   `dropped` 差**（rust 比 emu 多/少消费一次迭代），不是采样相位抖动。
   两种采样点（endFrame / spin-exit）在本批探针上**结果完全一致** ⇒ 采样相位不是本族主因。
2. **是否还有「同 FC 双行」？** 长窗内 **0 处**（含丢帧段）。先前在 `endFrame` 口径下
   观察到的 `f1719`/`f1720` 同 FC 不同画面，在 `spin-exit` 下不复现 ⇒ **FC → 帧 是单值映射**，
   FC 对齐在实现上可行（仍保留多候选检查作护栏，见正文 §5 的 `fc_phase_ambiguous`）。
3. **Kraid31 赤字是否仍 3→4？** 是，且更长：到 f1741 时已是 **−8**，此后维持 248（= −8）。

结论：**GO**。落地时须把两类偏移分开记账——
- **载房窗常量偏移**（a0 r02 的 −1，即 22 房 shift1 族的来源）：属 `dropped` 会计差异，
  是真缺陷候选，**不得**被 FC 对齐吸收；
- **运行期累积偏移**（a2 r31 的 −1 级台阶）：属掉帧，由 FC 对齐消除。
两个偏移必须在台账里分字段记录（建议 `fc_base_shift` 与 `fc_lag_steps`），否则无法区分。

## 0. 结论先行

1. **用户裁决成立**：不该在引擎里模拟掉帧；判据改成按游戏时间（`FrameCount` = `$002D`）对齐，引擎零改动。
2. **但"只对齐 FC 就能让像素归零"这个前提是错的**，本轮实测已给出反例：`a2_r31` 的 emu `f1719` 与 `f1720` 的 `FrameCount` **都是 160**，画面却相差 **11612 px**（§2.1）。原因是两腿的**采样相位**不同（emu `endFrame` vs rust 缝），且 emu 侧 `FC` 与画面在掉帧边界上不是一一对应。
3. **所以方案的第一性前提不是"对齐 FC"，而是"先把采样相位钉死再对齐 FC"**。这是本方案与任务书设计的最大分歧，也是它能不能落地的分水岭（§3.1）。
4. 现有判据里已经存在一个**被数据揭露的既有问题**：156 个 ok 房中有 **22 房**靠 `best_pairing="shift1"`（硬编码 1 帧时间位移）通过，而这 22 房的**游戏内丢帧数全部为 0**（§2.4）。即"1 帧时间容差"已在悄悄吸收一个与掉帧无关的差异。FC 对齐必须**收窄**它而不是**推广**它。
5. 窗口不能是 `[c-12, c]`：方向由实测 FC 赤字决定，`a2_r31` 需要 `g≈c+5`，`a2_r29` 需要 `g≈c+13`，且 `a2_r31` 在 1 帧内赤字从 3 跳到 4（§2.3）⇒ **窗口要按实测 Δ 定量外延并同步外延跑分地平线，而不是写死 ±12**。

---

## 1. 问题陈述

### 1.1 判据现状

- 对拍器 `tools/l4_diff_run.py` 把同一份输入分别喂 Mesen2（emu，裁定者）与 Rust，逐采集点比 PNG 像素，要求 0px。
  - `diff_pair(rust_png, emu_png, ...)`：`tools/l4_diff_run.py:352`，只允许 `gates/visual_whitelist.json` 白名单例外。
  - 单例执行入口 `execute_case(...)`：`tools/l4_diff_run.py:1024`；读 rust manifest `{name}.rust.captures.json`、逐采集点产出 `results[]`。
- 房扫器 `tools/movie_ref/room_sweep.py`：
  - 用例生成 `build_case(area, room, frames, captures)`：`tools/movie_ref/room_sweep.py:128`；`watch` 块 `:139-146`；采集点与跨腿参考帧 `:168-177`；`endFrame` `:178`。
  - 判据优先级（身份 → GameEngine → 报告新鲜度 → 逐点 → 汇总）：`:676-807`；`blocked/error` 先于 `ok` 判定。
  - 现状判定不是"严格同帧"：`c["best_pairing"] = "same"|"shift1"`、`c["best_diff_pixels"] = min(same, aligned)`，全采集点 0 才 `ok`（`tools/movie_ref/room_sweep.py:739-807` 一带）。`aligned_diff` `:484`；`shift_probe`（±4 px 空间平移搜索）`:514`；`classify_layer` `:558`。
  - 台账 `data/status/room_sweep_<area>.json`：`atomic_write_json` `:881`，`build_doc` `:896`，`judged_statuses` `:899`。

### 1.2 掉帧事实（已定谳，不再论证）

- emu 在主循环单次迭代工作量 > 29780 cycles 时周期性丢显示帧；`FrameCount($002D)` 推进慢于视频帧。
- Rust 无逐帧 cycle 模型（只有**载房窗**掉帧模型：`crates/game-core/src/world/room_draw.rs:74` 的 `LOAD_BASE_CYCLES` 与 `crates/game-core/src/world/camera/scroll.rs:62-75` 的 `mem.iter_bogged(...)`）⇒ 稳态永不丢帧。
- 于是跑 60 个视频帧后两腿**游戏时间**可差 6~12 帧，像素必然错位。
- 用户裁决：**不在引擎里模拟掉帧**，改判据对齐方式。

### 1.3 目标

把房扫判定从「视频帧对齐」升级为「**FrameCount 对齐**」，引擎零改动，可落地、可验证、红线机械可查。

---

## 2. 本轮实测（全部只读取得，是本方案的地基）

数据来源：直接解析既有产物 `analysis/traces/<stem>_f<NNNNN>_<tag>.ram.hex`（emu，`dumpRam` = `readRangeWithEnsExtra(0x0000,0x07FF)`，`tools/scenario_capture.lua:305`）、`shots_rust/<stem>_f<NNNNN>_<tag>.ram.hex`（rust，同形）、`shots/` 与 `shots_rust/` 的 PNG、以及 `<stem>.frames.jsonl`。`stem` 取自台账 `rooms[].room_load.trace` 的 basename。`$2D`=FrameCount，`$FD`=ScrollX。两腿 ram.hex 长度均 4544 hex 字符，按 `addr*2` 定位，已用 `$5A`/`$FD` 交叉验证偏移正确。

### 2.1 致命发现：emu 侧 `FrameCount` 与画面不是一一对应（相位歧义）

`a2_r31`（Kraid 31，`data/status/room_sweep_2.json`）：

| 视频帧 | FC(emu) | ScrollX(emu) | FC(rust) | ScrollX(rust) | same 像素差 | align1 像素差 |
|---|---|---|---|---|---|---|
| f1659 prev60 | 0x67=103 | 0x74=116 | 103 | 116 | 0 | — |
| f1660 f60 | 0x68=104 | 118 | 104 | 118 | **0** | 11170 |
| f1719 prevend | 0xA0=160 | 230 | 163 | 236 | 14960 | — |
| f1720 f120 | 0xA0=160 | 232 | 164 | 238 | 14755 | 13113 |

- **同一房间内，emu `f1719` 与 `f1720` 的 FC 完全相同（160），但 ScrollX 从 230 走到 232，两张 emu 截图相差 11612 px。**
- 同一窗口 emu 侧 ScrollX 在 `1715→1716` 保持 224（一次掉帧，`frames.jsonl` 实测：`1712..1720 = 218,220,222,224,224,226,228,230,232`）。
- 结论（A 级，可复现）：**`FrameCount` 相等不构成"同一画面"的等价类**。在掉帧边界上，emu 的画面/滚动已经推进到第 N+1 次迭代，而 `$002D` 还停在 N。任何"找 FC 相等的 g 然后比像素"的算法，在掉帧房间都必须在**多个候选**里选，而选错就直接产生假失配或假通过。

根因：两腿采样相位不同。

- emu 的逐帧 trace 默认 `samplePoint = "endFrame"`（`tools/scenario_capture.lua:1128`），`sampleTrace()` 在 `:1187` 被 endFrame 路径调用；`dumpRam/dumpPpu` 也在 **endFrame** 读（`:1316-1330`）。而 PNG 是在**缝**上兑现的（`onSpinExitSeam` `:1220`）。
- rust 的状态行/采集点在**缝**上（`crates/game-core/src/frame/engine.rs:139-145`：只有 `!in_span && !title_continuation && phase∈{PostSpin,MainLoop}` 才发缝回调）。
- ⇒ 同一个"视频帧编号"上，emu 的 RAM 面与 PNG 相位可以差一次迭代；rust 的 RAM 面与 PNG 也有自己的错位（`capture(N) = tick(N-1)` 的缝快照，见 `tools/movie_ref/room_sweep.py:174` 注释与 `crates/game-core/src/bin/rust_screenshot.rs:559-570`）。

### 2.2 rust 稳态 FC 严格 1:1

实测所有可读房间：`FC_rust(f) = f − const`（area0 const=784，area1/2/3/4 const=1556 或 1557），例：`a2_r31` f1660→104、f1719→163、f1720→164（1660−104=1556，1720−164=1556）。跨度窗（载房窗）外**没有**逐帧丢帧。

### 2.3 赤字方向与时变（决定窗口）

- `a2_r31`：f1719 赤字 3，f1720 赤字 **4**（一帧内 +1）。
- `a2_r29`：f1720 emu ScrollX 216 vs rust 238 ⇒ 约 **11** 帧赤字。
- 冻结帧普查（`ScrollX` 步进为 0 的帧，窗口 `[entry, entry+120]`，entry=900/1600）：各房最大冻结数 area0=4、area1=7、area2=12（room29）、area3=9、area4=9；ok 房的冻结全部落在载房窗内。
- ⇒ 判定期末 `c=frames` 的所需配对帧 `g` **大于** `frames`（`a2_r31` 需要 `g≈1725`，`a2_r29` 需要 `g≈1733`，跑分地平线是 1720）⇒ **必须外延 emu 跑分地平线**，任何只在本轮窗口内搜索的方案在最难的房间上必然 `fc_align_failed`。

### 2.4 现有 `shift1` 判据 = 硬编码 ±1 的 FC 等价判据

- `a0_r02`（ok，shift1）：f960 emu FC=175 / rust=176；ScrollX 118/120；same=10381、align1=**0**。
- `a0_r21`（ok，same）：f960 FC 175/175；ScrollX 118/118；same=**0**、align1=12242。
- `a1_r04`（ok，shift1）：f1660 FC 103/104；f1720 FC 163/164；align1=0。
- `a2_r31`（mismatch）：f1660 FC **相等**且 same=**0** —— 即"FC 相等 ⇒ 像素相等"在**同一相位**下成立（f1660 两点都是缝附近的同相位采样）。
- **普查：156 个 ok 房中有 22 房靠 `shift1` 通过；这 22 房的游戏内丢帧数全部为 0**（脚本核对：`ScrollX` 步进 2 为主的窗口内零个 0 步进，22/22）。
- ⇒ 这 22 房的"1 帧时间差"**不是掉帧造成的**，而是**同帧 FC 差 1**（`a0_r02`、`a1_r04` 的实测）。该差 1 的成因有两个竞争假设，**未定谳**（§7.2）：H1 rust 载房窗 `dropped` 差 1（对照 `docs/findings/norfair-room-sweep.md` §1c：emu 冻 1、rust 冻 2）；H2 endFrame 采样相位不稳（§2.1 同源）。P0 用 `a0_r02` 一房即可判别。
- ⇒ **任务书里"已 ok 房 `fc_shift` 必须为 0"的红线被数据推翻**：22 房的正确 `fc_shift` 是 ±1。红线必须改写（§3.5），否则要么误判回归、要么逼人把 1 帧差强行压成 0（掩盖真差异）。

### 2.5 相位一致时 FC 相等 ⇒ 像素相等（可用的正面证据）

在 `f1660`（`a2_r31`/`a2_r28`）、`f960`（`a0_r21`/`a0_r01`）等采集点上，FC 相等 ⇔ 0px，FC 差 1 ⇔ ~11K px（= 256 列 × 45 行量级的整屏 2px 位移）。**但成立条件是两腿采样相位一致**（§2.1）。这条正面证据是本方案"钉死相位后按 FC 对齐"唯一的立论基础，且它只在 **6 个房间 × 2~3 个采集点** 上被验证过，不是证明。

---

## 3. 设计

### 3.1 相位先决条件（新增，任务书未列，但缺它整个方案不成立）

**定义**：一次"缝采样"= 一次主循环迭代完成点（emu `$C0D3`，rust `engine.rs:139-145`）。FC 对齐只在**缝采样**得到的 (视频帧号, FC) 序列上进行。

- emu 侧：retry 遍的用例必须带 `sample_point spin-exit`（DSL 支持：`tools/scenario_run.py:386-391`；采样点分支：`tools/scenario_capture.lua:1128`、`sampleTrace` 调用点 `:1187`/`:1220` 一带）。
- rust 侧：缝采样本来就是默认；`--trace fc` 的输出**就在缝回调里**（`crates/game-core/src/bin/rust_screenshot.rs:585-587`，打印 `[fc] {frame} {FrameCount} {$1F}`），相位与采集点天然一致。这是 rust 侧拿 FC 的**零成本**通道（对比：`--state-out` 每帧 ≈20 KB，`crates/game-core/src/frame/memory.rs:206-220`，禁止为读 FC 开它）。
- emu 侧每帧 FC 的成本：DSL 加一行 `watch 002D FrameCount`（`tools/movie_ref/room_sweep.py:139-146` 的 watch 块），每帧 +2 hex 字符，可忽略。

> **未定谳（P0 必须实测）**：`sample_point spin-exit` 之后，*没有产生缝的视频帧*是否仍有 trace 行？若有（沿用上一行），则"行存在≠有缝"，所有候选必须再用 FC 序列的"是否推进"过滤；若无，则行集就是缝集，可直接当候选集。两种情形下算法不同（§3.3 给出兼容写法）。

### 3.2 采集侧：两遍跑分

第一遍（现状，不动）：

- `room_sweep.py` 全量跑，只多做两件零成本的事：① `build_case` 加 `watch 002D`；② 把 `--trace fc` 透传给 rust 腿（`tools/l4_diff_run.py:315` 的 `run_rust` 需要新增 `--rust-trace` 透传参数，`room_sweep.py` 侧新增同名 CLI 参数）。rust 侧 `tracing` 输出落 `analysis/traces/<stem>.rustfc.log`（stderr 重定向），**不进 manifest**。
- 第一遍仍然按现有规则出 `ok/mismatch/blocked/...`，**判定结果与今天完全一致**（这是红线，见 §3.5）。
- 代价（必须写进排期）：加 `watch 002D` 会改动用例文本 ⇒ `scenario_sha256`/`sched_sha256` 变（`tools/l4_diff_run.py:223 compute_anchors`、`:252 check_reuse`）⇒ **emu 缓存全量失效，156 房必须重跑**，按 40.8 s/房 ≈ **1.8 小时**。这是唯一一次全量重跑成本；之后 FC 只在 retry 房上增量。
- **命名隔离（隐患）**：retry 遍若沿用 `case_name(area, room)`（`tools/movie_ref/room_sweep.py:130`），会覆盖 `analysis/traces/<stem>.frames.jsonl`，而台账 `rooms[].room_load.trace` 正指向它 ⇒ 第一遍的载入链证据被 retry 遍的 `spin-exit` 采样覆盖。retry 用例 stem 必须带后缀（如 `<stem>_fcretry`），台账新增 `room_load.trace_fc` 单列。

第二遍（retry，只跑第一遍 `mismatch` 的房）：

- 对每个判定采集点 `c`（`pre` 与 `prev*` 除外），取 `Δ_c = FC_rust(c) − FC_emu(c)`（两腿同相位采样，第一遍已有）。
- 窗口：`W_c = [c − (max(0,−Δ_c) + M), c + (max(0,Δ_c) + M)]`，`M = 8`（余量，覆盖窗口内生新掉帧）。**不是固定 ±12**：`a2_r29` 需要 +13，`a0_r02` 需要 +1，若某房 rust 冻得比 emu 多则 Δ 为负、窗口向左（这是 22 房 shift1 的形态）。
- 地平线：`endFrame` 从 `entry+frames` 外延到 `entry + max_c(hi_c) + 1`（`a2_r29` 约 +14 帧，≈ +12% emu 时长）。同时 `capture` 目标帧必须在 `frames` 之内，否则被 `scenario_run` 丢弃 ⇒ 外延是硬要求，不是优化。
- 窗口内每一帧发一个采集点，tag 形如 `fcw{c}_{±i}`：`capture {f} fcw{c}_p{i}`。每个 `c` 约 2M+1+|Δ| ≈ 17~25 个采集点；每点成本 = 一次 `takeStableShot`（`tools/scenario_capture.lua:218-232`，最多 8 次截图）+ ram 2272B + ppu 12KB + PNG。
  - 磁盘：≈ 20 点 × 2 点 × (PNG ~15KB + ram 4.5KB + ppu 24KB) ≈ 2.2 MB/房。
  - 时间：截图是主要开销；按现状 40.8 s/房（Kraid 37 房 1509.5 s，`data/status/room_sweep_2.json` `elapsed_s`）估，retry 房 +3~6 s（+9~15%），但**只对失败房**跑（当前 Kraid 4/37、Tourian 8/21、Ridley 22/42，全量 retry ≈ 34 房 ≈ +2~3 分钟）。
- 允许两次迭代求不动点：若 retry 遍仍 `fc_align_failed`，用 retry 遍的新 FC 序列重算 Δ 再外延一次；上限 2 次，超限记 `fc_align_failed`（fail loud，不静默 ok）。
- 输入语义：窗口中多出来的帧仍由 `joy {f} {hex}` 逐帧定义（`tools/l4_diff_run.py:186` `schedule_text`），方向键固定按住 ⇒ 无按键沿，撤不掉"键沿落在被吞帧"的问题（那是 §3.6 的 lag-induced 面，本轮不判）。

### 3.3 对齐算法

输入：`emu: [(video_frame, fc)]`（缝采样，按帧号升序）、`rust: [(video_frame, fc)]`（`[fc]` 行）、目标采集点 `c` 及其 `fc_rust(c)`。

步骤：

1. **展开（unwrap）**：`u[0] = fc[0]`；`u[i] = u[i−1] + ((fc[i] − fc[i−1]) & 0xFF)`。合法性：每步必须 ∈{0,1}（无 NMI 的载房窗可能 >1 ⇒ 该步记 `fc_jump` 并作废该房的对齐，fail loud）。**这一步顺带解决"8 位模 256 回绕"**：回绕时 `(0 − 255) & 0xFF = 1`，天然正确；**禁止**用裸 8 位值比较大小（`250 > 3` 为真）。
2. **候选**：`C = { j : u_e[j] == u_r(c) }` ∩ 窗口。若 `sample_point spin-exit` 下"无缝帧也有行"，先用 `fc` 是否推进把行压缩成缝集再取候选。
3. **选择**：
   - `|C| == 0` ⇒ `fc_align_failed`（**fail loud**，绝不回落成 ok/nearest）。
   - `|C| ≥ 2` ⇒ 取**最小 j**（缝锚：第一个到达该 FC 的缝）为配对 `g`；其余候选逐个比像素，全部记入 `fc_px_by_candidate`。若 `px[g] != 0` 但存在 `j>g` 使 `px[j]==0` ⇒ `fc_phase_ambiguous`（**不算 ok**，因为这意味着两腿在"哪次迭代先落地"上不一致）。
   - 记录 `fc_shift = g − c`。
4. **RAM 面预门（廉价、先于像素）**：配对上比较 `$00FD`(ScrollX)/`$0019`/`$001A`(scroll/nt) 与 `$2D`。任一面不等 ⇒ `fc_ram_desync`（fail loud）。理由是 §2.1：同 FC 不同画面的房间，ScrollX 一定会先把分歧暴露出来，用它把"相位错"与"逻辑错"分开，不浪费像素比较的解释成本。
5. **像素**：`diff_pair` 复用（`tools/l4_diff_run.py:352`），白名单不变。
6. **禁止叠加空间平移**：见 §3.5 的防掩盖规则。

### 3.4 判定与台账

新状态值：`ok_fc_aligned`（**不与 `ok` 混**；`ok` 的定义严格保持"视频帧对齐 0px"）。

采集点级新增字段（`data/status/room_sweep_<area>.json` → `rooms[].captures[]`）：

| 字段 | 类型 | 含义 |
|---|---|---|
| `alignment` | `"frame"` \| `"fc"` | 该点最终采用的判据；旧台账缺省视为 `"frame"` |
| `fc_ref` | int \| null | `FC_rust(c)`（展开后） |
| `fc_emu` | int \| null | 同帧 `FC_emu(c)`；`Δ = fc_ref − fc_emu` |
| `fc_shift` | int \| null | `g − c`，正=emu 需向后取帧 |
| `fc_candidates` | list[int] | 窗口内所有候选视频帧号 |
| `fc_px_by_candidate` | dict[str,int] | 每个候选的像素差（审计用，防"挑最小的那个"） |
| `fc_ram_desync` | list[str] | 不一致的 RAM 面名（§3.3 步骤 4） |
| `fc_pass` | bool | 该点 FC 对齐是否通过 |

房间级：`status`、`fc_alignment: {pass, rooms?}`、`notes[]` 追加一行 `FC 对齐：Δ=+k g=f_xx 平移=0`。

汇总层：`build_doc`（`tools/movie_ref/room_sweep.py:896`）`summary` 增 `ok_fc_aligned`；`judged_statuses`（`:899`）加入 `ok_fc_aligned`；`planned == ran + skipped`、`ran == Σjudged` 的自洽断言必须同步（否则 `:899` 的算术检查会炸）。终端 tag 表（`:1093`）加 ✅(FC)。

向后兼容：

- 顶层加 `"schema_version": 2`；**旧台账（无 `schema_version`）读入时全部按 `alignment="frame"` 解释**，不重算、不改判。
- `--resume`（`:1019`/`:1084`）只认 `ok`；`ok_fc_aligned` 是否算"已完成"必须显式决定：**不算**（它需要 retry 遍的产物才能重放，算已完成会让重跑静默跳过）。在 `--resume` 帮助文本里写明。
- 台账 diff 的可读性：`mismatch[]`（`{tag,best_diff_pixels}`）保持旧含义（`ok` 判据下的取小值），FC 对齐的失败另列 `fc_mismatch[]`。

### 3.5 红线（机械化，写进测试与门禁）

任务书的红线（"Brinstar 47 房与 Norfair 44 房结果不变；`fc_shift==0`"）按 §2.4 必须改写为：

- **R1 结果不变**：对改动前的台账快照（`git show <base>:data/status/room_sweep_0.json`，当前 `0..4` 全绿/已定），所有 `status=="ok"` 的房，点级 `diff_pixels` 与 `best_pairing` **不得变化**，房间 `status` 不得变成 `mismatch/error/blocked`。第一遍跑分不改判据 ⇒ 这条在结构上自动成立，测试只是把它钉死。
- **R2 `fc_shift` 按房分类注册**（替代"必须为 0"）：
  - 旧台账里 `best_pairing=="same"` 的房 ⇒ `fc_shift == 0`；
  - 旧台账里 `best_pairing=="shift1"` 的 22 房 ⇒ `fc_shift ∈ {+1,−1}` 且 `|fc_shift|==1`（实测形态，§2.4），**且 retry 后像素仍为 0**；
  - 其余任何房间出现 `|fc_shift| > 1` ⇒ **回归**（不是"掉帧解释"），测试红。
    - 依据：ok 房实测零游戏内丢帧（§2.4 普查）；丢帧房现在都是 mismatch，不在 R2 范围内。
- **R3 掉帧不是万能解释**：若某房从 `mismatch` 变 `ok_fc_aligned` 但 `fc_shift == 0` ⇒ **判回归**（原因不是掉帧）。这条正是任务书那条，保留。
- **R4 防掩盖（空间平移 × FC 位移互斥）**：`fc_shift != 0` 的配对准入要求 `shift.dx == 0 and shift.dy == 0`（`shift_probe` 的结果，`tools/movie_ref/room_sweep.py:514`）；`shift.dx/dy != 0` 的配对准入要求 `fc_shift == 0`。二者同时非零 ⇒ `masked_ambiguous`（红）。
  - 依据：BR 与 NO 的 ok 房存在 `shift.dx=2, dy=0` 的滚动位移族（台账 `shift.explains=true`）；若同时允许空间平移和 1~12 帧时间位移，两参数的自由度足以吃掉一个真实的 2px 滚动 bug。
- **R5 不许"取小"**：`min` 只在 `same|shift1` 二选一的老语义里存在；FC 对齐必须**单一决定**（候选选择规则固定），不得在候选集合上再取 `min(px)`。
- **R6 blocked 不被吞**：`blocked`（rust `manifest.blocked`，`tools/l4_diff_run.py:1024` → `do_diff`）在 `:676-807` 的优先级里先于 ok；retry 遍**不得**对 `blocked` 采集点做任何对齐尝试，也不得因为"另一点对齐成功"把房间判 ok。
- **R7 fail-loud 集合**：`fc_align_failed` / `fc_phase_ambiguous` / `fc_ram_desync` / `fc_jump` / `masked_ambiguous` 五种状态都必须是**非 ok 状态**并计入 `mismatch` 的兄弟计数（建议单列 `fc_failed`），自洽断言 `planned == ran + skipped` 与 `ran == Σjudged` 必须把新状态算进 `judged_statuses`。

### 3.6 lag-induced 分类（本轮只定接口，不实现）

问题：emu 第 k 次迭代消费 `input[k+lag_k]`，rust 消费 `input[k]`。被吞帧的输入与相邻帧相同时完全等价；按键沿恰落在被吞帧时 emu 看不到跳变 ⇒ 不可复现的 1 拍差。

接口（预留，`tools/movie_ref/room_sweep.py` 新增纯函数）：

```
def classify_lag_induced(room_rec, emu_fc, rust_fc, emu_joy, rust_joy) -> dict:
    # 输入：两腿缝级 FC 序列 + 逐视频帧 joy 字节（emu: `joy` 行即输入；rust: plan.joy）
    # 输出：{"class": "lag-induced"|"logic"|"undecided",
    #        "first_divergence_fc": int|None,
    #        "lag_frame": bool,        # 该缝对应视频帧是否是被吞帧（FC 在相邻视频帧未推进）
    #        "edge_in_lag": bool,      # 被吞帧的 joy 是否与前一帧不同（有沿）
    #        "evidence": {...}}
```

判定规则（写死，避免"含糊说法"）：

- `lag_frame = (FC_emu(f−1) == FC_emu(f))`（被吞帧 ⇒ `FC` 在该视频帧未推进，§2.1 实测形态）。
- `edge_in_lag = joy_emu(f) != joy_emu(f−1)`。
- 仅当 **首个分歧缝** 满足 `lag_frame and edge_in_lag` 才允许记 `lag-induced`；否则 `logic`。`undecided` 只用于 FC 序列缺口。
- 房扫当前输入是固定按住方向键（`build_case` 只有 `press 310/350`，`tools/movie_ref/room_sweep.py:149-150`）⇒ 判定期内 `edge_in_lag` 恒假 ⇒ 本轮所有非 FC 对齐失败的房一律 `logic`，不会有房被误判成 lag-induced（这也是本轮的验收自检）。

### 3.7 测试（`tools/tests/`，pytest 风格，参照 `tools/tests/test_l4_ratchet.py` 的 `sys.path` 注入法）

新增 `tools/tests/test_room_sweep_fc_align.py`：

1. **模 256 展开**（单测，纯函数）：
   - `unwrap([250,255,0,3]) == [250,255,256,259]`；
   - 步进 5 ⇒ 抛 `FcSeriesError`（对应 `fc_jump`）；
   - 裸 8 位比较的反例：`250 > 3` 不得被用作排序键。
2. **窗口内多候选选择**：构造 `u_e=[…,100,100,100,101]`、`u_r(c)=100` ⇒ 候选 `{g0,g1,g2}`，断言选**最小** `g0`、`fc_shift` 正确、`fc_px_by_candidate` 完整；`px[g0]!=0 & px[g1]==0` ⇒ `fc_phase_ambiguous`（**不是** ok）。
3. **失败路径**：候选空 ⇒ `fc_align_failed`；`fc_ram_desync` 的 ScrollX 不一致 ⇒ 非 ok；`fc_shift!=0` 且 `shift.dx=2` ⇒ `masked_ambiguous`（R4）。
4. **台账向后兼容**：喂一段无 `schema_version`/无 `alignment` 的旧房记录（从 `data/status/room_sweep_0.json` 摘 2 房当 fixture），断言 `alignment=="frame"`、`status` 与 `diff_pixels` 原值不变。
5. **红线回归**：读 git base 版 `room_sweep_{0..4}.json` 与当前版，逐房断言 R1/R2/R3（不跑 emu，纯数据）。
6. **真实产物集成校验**（用 Kraid 31 的既有 stem，`skipif` 缺产物）：
   - `f1660`（同相位、FC 相等）⇒ `fc_shift==0`、`diff_pixels==0`；
   - `f1719`/`f1720`（`frames=120` 内无配对帧）⇒ 在**第一遍产物**上必须得到 `fc_align_failed`（证明失败路径真的会红，且证实地平线外延是必需的）；retry 遍产物落地后改为断言 `fc_shift≈+5`、`fc_emu(1720)=160`、且配对点 `diff_pixels==0`；
   - 断言 `fc_px_by_candidate` 里至少有两个候选（因 §2.1 的同 FC 双帧），用来钉死"多候选"路径被真实数据覆盖。

---

## 4. 对抗复核（证伪立场）

### Q1「只对齐 FC 就能让像素归零」的漏洞在哪？

**站不住的部分**：`FC` 相等 **不是**画面等价类。实测反例（§2.1）：emu `f1719`/`f1720` FC 都是 160，画面差 11612 px。原因是 `$002D` 的递增点在主循环尾部（`$C0BC` 一带，`inc FrameCount` 之后才 spin），而画面/滚动在本轮迭代里**先**被写；一个慢迭代跨两个视频帧时，两个视频帧拿到同一个 FC 但画面不同。

**能站住的部分**：**同相位**（缝）下 FC 相等 ⇒ 像素相等，在 6 房 2~3 点上无反例（§2.5）。

**因此必须加的护栏**（本方案的 P0/P1）：

- G1：emu 侧采样点从 `endFrame` 改成 `spin-exit`（`sample_point spin-exit`），把相位钉死；P0 用 1 房 2 相位对拍验证"改相位后同帧 FC 差是否从 0/1 变成稳定值"。
- G2：候选若 >1，必须 fail loud 或按"最小 g"唯一决定并记录全部候选像素差（§3.3），禁止 `min()`。
- G3：相位没钉死之前，`fc_shift` 这一列**不得**参与判定（只进审计）。

### Q2 emu 在 g 上的画面是"重复帧"时，像素应该相同吗？——可判定条件

给出三条机械条件（全部可自动判）：

- **C1 相位条件**：`g` 必须是缝采样；`f1719`/`f1720` 这种"endFrame 采样 + FC 未推进"的对，在缝采样下**不会**产生同 FC 的双缝（一次迭代一个缝），因此 C1 成立时候选集天然收缩为单元素。若仍出现 `|C|>1`，说明 FC 序列在缝间被重复采样 ⇒ `fc_phase_ambiguous`。
- **C2 状态条件**：配对点的 RAM 面 `$2D/$00FD/$0019/$001A` 全等（§3.3 步骤 4）。不等就不是"同一游戏时间"，没资格比像素。
- **C3 像素条件**：在 C1∧C2 下要求 `diff_pair == 0`。

**"重复帧"的正确理解**：掉帧时 emu 的**显示**会保持（渲染器没有新内容），但**游戏状态**仍在推进（下一次迭代的滚动已写）。所以"重复帧"只是显示层的重复，不是状态的重复；拿它跟 rust 比，正确比法是比**同 FC 的缝**，而不是比"看起来没变的那两帧"。若某房在 C1∧C2 下仍非零 ⇒ 是**显示层**的真实差异（例如 rust 在掉帧时刻没有保持显示），属于逻辑缺陷 ⇒ 记 `mismatch`，不是 lag 解释。

### Q3 窗口 ±12 够不够？

**不够，而且方向都不对**：

- 赤字是**单向累积**的：rust 稳态严格 1:1（§2.2），emu 每帧 +0/+1 ⇒ `FC_rust(c) ≥ FC_emu(c)`，需要取**更晚**的 emu 帧（`g > c`）。对称的 `[c−12, c]` 窗口里根本没有答案。
- 量级：`a2_r29` 需要 `g ≈ c + 13`（f1720 ScrollX 216 vs 238 ⇒ 11 游戏帧 + 期间掉帧），`a2_r31` 需 `c + 5`；`a2_r31` 在一帧内赤字 3→4（§2.3）⇒ 窗口必须有冗余。
- 且判定期末 `c=frames` 时 `g > frames`，**地平线本身就不够**（§2.3）。

替代方案（本方案采纳）：**按实测 Δ 定量外延地平线 + 窗口 [`c−(max(0,−Δ)+8)`, `c+(max(0,Δ)+8)`]，最多两次迭代求不动点，找不到就 `fc_align_failed`**。逆向窗口（Δ<0）确实需要，因为 22 房 shift1 的形态就是 `FC_emu > FC_rust`（rust 载房窗多冻了一帧）。

### Q4 FC 对齐与既有 `shift_probe` 会不会互相掩盖？

**会**。`shift_probe` 已在 ±4px 上暴力搜索并按 `residual <= raw*0.5` 判 `explains`（`tools/movie_ref/room_sweep.py:514`），BR/NO 的 `dx=2` 滚动族已经在用这个自由度。若再叠一个 1~13 帧的时间位移，两个自由度可以互相顶替：一个真实的"rust 滚动慢 2px"完全可以用 `fc_shift=1` 吸收。

防掩盖设计（R4 + R5）：

- **互斥**：`fc_shift != 0` ⇒ 要求 `shift.dx==dy==0`；`shift.dx|dy != 0` ⇒ 要求 `fc_shift==0`；同时非零 ⇒ `masked_ambiguous`（红）。
- **唯一决定**：候选不得再取 `min(px)`。
- **审计留痕**：`fc_px_by_candidate`、`shift{}` 与 `diff_pixels` 三者都进台账，任何"通过但两个位移都非零"的房在评审时一眼可见。
- **分层复核**：`classify_layer`（`:558`）必须仍然跑；`ok_fc_aligned` 的房若 `layer != clean` ⇒ 记 `notes`（至少肉眼可见）。

### Q5 这套对齐会不会把 `blocked`（缺口 panic）误判成 ok？

**当前结构不会，但新增代码有两个入口会**：

- 现状 `blocked` 在 `:676-807` 的优先级里先于 `ok`，且 `do_diff` 把 `man["blocked"]` 单列（`tools/l4_diff_run.py:1024`）⇒ 天然不会被 ok 吞掉。
- 风险入口 ①：retry 遍对 windows 内每帧发采集点 ⇒ 同一采集点可能既有 blocked 又有 FC 对齐成功点。必须按 tag↔原始采集点映射回 `c`，**只要 rust 在 `c` 上是 blocked，该点就 `blocked`**（R6）。
- 风险入口 ②：`fc_align_failed` 若被实现成"跳过该点"（不计入分母），分母会缩水 ⇒ 假绿。必须计入 `fc_failed` 计数并进 `judged_statuses`（R7），且 `reconcile_denominator`（`tools/l4_diff_run.py:593`）的"缺/多即 fail"语义要覆盖 retry 采集点。

### Q6 其他被推翻的假设

- ~~"Rust 没有 cycle 模型 ⇒ 永不丢帧"~~：**部分错**。rust 有**载房窗**掉帧模型（`room_draw.rs:74` + `scroll.rs:62-75`）。因此"某房两腿 FC 差 1 且无游戏内丢帧"完全可能是 rust 载房窗 `dropped` 差 1 —— 这是**rust 侧缺陷**，FC 对齐**不得**吸收它。→ R2/R3 让这类房必须报出来。
- ~~"22 房 shift1 = emu 掉帧"~~：普查证伪（22/22 丢帧为 0）。
- ~~"`fc_shift` 恒为 0"~~：22 房反例。

---

## 5. 分阶段落地（每步：改哪里 / 验收命令 / 期望）

### P0 相位定标（半天，1 房，必须最先做）

- 改：无代码。手工在 `/tmp` 造两版用例（`endFrame` vs `sample_point spin-exit`），对 `a0_r02`（shift1 族）与 `a2_r31`（掉帧族）各跑一次：`python3 tools/l4_diff_run.py run cases/_tmp_fc_probe_a0_r02.txt --timeout 900`（用例源文本从 `build_case` 的等价输出改两行得到；不改仓库用例目录）。
- 另加 `watch 002D FrameCount` 到这两版用例。
- 验收：从 `analysis/traces/*.frames.jsonl` + `shots_rust/*.ram.hex` 读出两腿缝级 FC 表，回答三个问题：
  1. spin-exit 下同帧 `FC_emu − FC_rust` 是否变成**房间无关常量**？（若是 ⇒ 相位已钉死，§2.1 的 0/1 抖动是 endFrame 造成的）
  2. spin-exit 下是否还有"同 FC 双行"？（若有 ⇒ §3.3 的多候选路径必须保留）
  3. `a2_r31` 的赤字在缝采样下是否仍为 3→4？
- 期望输出：一份 3 行结论 + 两版 trace 的 FC 序列（存 `/tmp`）。**这是全案 go/no-go 闸门**：若 Q1 的答案是"仍是 0/1 抖动、与房间相关"，则 FC 对齐方案必须降级为"只做状态面（RAM）对齐 + 像素仅作提示"，不允许作为判定。

### P1 零成本仪表（1 天，不影响任何判定）

- 改 `tools/movie_ref/room_sweep.py:139-146` 的 watch 块加 `watch 002D FrameCount`；`tools/l4_diff_run.py:315` `run_rust` 增 `--rust-trace` 透传；`room_sweep.py` 增同名 CLI 参数并落 `<stem>.rustfc.log`；`build_case` 增参数 `sample_point: str = "endFrame"`（默认不变 ⇒ 判定零变化）。
- 改 `data/status/room_sweep_*.json` schema：加 `schema_version: 2`、采集点 `fc_ref/fc_emu`（只读审计，不参与判定）。
- 验收：`python3 -m pytest tools/tests/test_room_sweep_fc_align.py -q`（此时只有向后兼容 + 纯函数用例）；`python3 tools/movie_ref/room_sweep.py --areas 0 --rooms 21 --dry-run` 输出计划不变；`python3 gates/cli.py status` 无新增 FAIL。
- 期望：判定/台账 `status` 全量不变；`data/status/room_sweep_0.json` 除新增字段外 diff 为空。

### P2 对齐内核（1~2 天，纯函数 + 单测）

- 新增 `tools/movie_ref/fc_align.py`：`unwrap_fc`、`seam_table`、`pick_candidate`、`align_capture`、`FcSeriesError`；全部纯函数、无 I/O。
- 验收：`python3 -m pytest tools/tests/test_room_sweep_fc_align.py -q` 全绿（含 §3.7 的 1/2/3 项与模 256 用例）。
- 期望：覆盖 `fc_align_failed`/`fc_phase_ambiguous`/`fc_ram_desync`/`masked_ambiguous` 四条红路径。

### P3 第一遍接线（判定不变）

- 改 `run_room`（`tools/movie_ref/room_sweep.py:632`）与 `:676-807` 的判据块：读两腿 FC，写 `fc_ref/fc_emu`；**不引入 `ok_fc_aligned`**。
- 验收：`python3 tools/movie_ref/room_sweep.py --areas 2 --rooms 28,29,31 --frames 120 --captures 60,120 --out /tmp/rs_a2.json`
- 期望：三房 `status` 仍为 `mismatch`，`fc_emu/fc_ref` 与 §2.1 表一致（1720: 160/164），无 `ok` 变化。

### P4 retry 遍 + `ok_fc_aligned`

- 改 `build_case`（`:128`）支持窗口捕获与地平线外延；`run_room` 增 retry 分支；`build_doc`（`:896`）/`judged_statuses`（`:899`）/终端 tag（`:1093`）接 `ok_fc_aligned` 与 `fc_failed`。
- 验收 1（单房）：`python3 tools/movie_ref/room_sweep.py --areas 2 --rooms 31 --frames 120 --captures 60,120 --out /tmp/rs_a2_31.json`
- 期望 1：`status == "ok_fc_aligned"`，`fc_shift` ≈ +5，配对点 `diff_pixels == 0`，`shift.dx == 0`，`notes` 有 `Δ=+4`。
- 验收 2（红线）：`python3 -m pytest tools/tests/test_room_sweep_fc_align.py -q`
- 期望 2：R1/R2/R3 全绿（BR 47 / NO 44 一个不改判；22 房 `|fc_shift|==1`）。
- 验收 3（分区回归）：`python3 tools/movie_ref/room_sweep.py --areas 0 --rooms all --out /tmp/rs_a0.json`（BR 47 房，约 40 s×47 ≈ 32 min，建议后台）
- 期望 3：47 房 `status` 与 `data/status/room_sweep_0.json` 逐房一致；`fc_shift` 分布 = {0: 36 房, ±1: 11 房}（area0 的 shift1 房 = 2/14/24/25/28/29/30/37/38/41/43，§2.4 普查；分布不同即为回归）。

### P5 台账定版 + lag-induced 接口

- 改：`data/status/room_sweep_{0..4}.json` 重跑落库（含 `schema_version: 2`）；`tools/movie_ref/fc_align.py` 加 `classify_lag_induced` 桩（`NotImplementedError` 或返回 `undecided`，不接判定）。
- 验收：`python3 gates/cli.py status`、`python3 gates/cli.py check` 全绿；`docs/findings/` 增一页记录 §2 实测（否则这些只活在计划文档里，违反"结论的归宿是仓库"）。
- 期望：门禁无新增 FAIL；FC 对齐的失败房清单（Kraid 4、Tourian 8、Ridley 22 中仍红的）逐房带 `fc_*` 字段可审计。

---

## 6. 风险与回退

| 风险 | 触发信号 | 回退 |
|---|---|---|
| P0 证伪（相位钉死后 FC 仍与房间相关地抖 0/1） | P0 问题 1 答"仍抖动" | 判定退回"仅状态面对齐"，像素只作提示；`ok_fc_aligned` 不引入 |
| 地平线外延失控（某房赤字 ≫ 20） | retry 两次仍 `fc_align_failed` | 该房判 `fc_align_failed`（红），单列清单交人工；不得放宽窗口继续加长烧算力 |
| retry 让某房"变绿但不是掉帧" | R3 触发（`fc_shift==0` 却变绿） | 判回归，回退该房，进 `docs/bug-ledger.md` 或 rust 缺陷任务卡 |
| 采样相位改动悄悄改了 Tahoe/Ridley 的既有 ok | R1 触发 | 立即回退 P1 的 `sample_point` 默认值（保持 `endFrame`），改相位只在 retry 遍启用 |
| 分母缩水 | `reconcile_denominator`（`tools/l4_diff_run.py:593`）报"缺/多" | 修复 tag↔采集点映射，不得改断言 |
| 磁盘/时长膨胀 | 单房 retry > +30 s 或 > +5 MB | 窗口内只采 `ram+ppu`（12KB+2KB），PNG 只对候选帧采（候选集在 retry 前可由第一遍 FC 预测 ≈ 1~3 帧） |
| retry 覆盖第一遍 trace | 台账 `room_load.trace` 内容被 `spin-exit` 采样改写 | retry stem 加 `_fcretry` 后缀（§3.2），`room_load.trace` 指向第一遍 |
| 加 watch 导致 emu 缓存全失效 | `check_reuse` 拒绝复用（`tools/l4_diff_run.py:252`） | 预期内的一次性 1.8 小时重跑；排期时按全量算 |
| 与 `--resume` 交互错 | `ok_fc_aligned` 被 `resume` 跳过 | 显式把 `ok_fc_aligned` 排除在 `r.get("ok")` 之外（`:1085` 只认 `ok`），帮助文本写明 |

回退粒度：P1（仪表）与 P3/P4（判据）分 commit；`ok` 的定义在 P1~P3 保持逐字节不变，P4 只**新增**状态值不改旧值 ⇒ 任何时候 `git revert` P4 即回到今天的行为。

---

## 7. 未定谳清单（不得当结论用）

1. **spin-exit 采样下"无缝视频帧"是否仍有 trace 行**——影响候选集构造（§3.1、§3.3 步骤 2）。P0 实测。
2. **22 房 `shift1` 的真实成因**：载房窗 `dropped` 差 1（rust 缺陷）vs endFrame 相位不稳（仪表伪差）。P0 用 `a0_r02` 一房即可判定：若 spin-exit 下 `FC_emu − FC_rust` 变 0，则是仪表伪差；若仍为 −1，则是 rust 载房窗模型差一帧 ⇒ 应开 rust 侧 root-cause 任务卡（`room_draw.rs:74` 的 `LOAD_BASE_CYCLES` 标定面），而不是让判据吸收。
3. **`a2_r29` 的 11 帧赤字是否为该房上限**：目前只有 ScrollX 反推（216 vs 238），未取 `$2D` 实测（该房 stem 的 ram.hex 需复核）。窗口 `M=8` 是否够也依赖这一点。
4. **emu 掉帧是否可能"集中爆发"**（例如一帧内连丢 2 次）⇒ 窗口内生新掉帧速率未知，`M=8` 是经验余量，无证据。
5. **lag-induced 的 `input[k+lag_k]` 映射**未在真实 TAS 上验证过（本轮无 TAS 端到端用例）。
