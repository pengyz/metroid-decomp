# FC 对齐判据 P1 实施规划（含对抗复核定谳）

- 上游：`docs/superpowers/plans/2026-11-23-fc-aligned-room-sweep.md`（P0 方案 + P0 结果 GO）
- 本文件定位：**P0 的判据方向成立，但 P1 的实现设计有 3 处致命错**。本文件先证伪、再给可执行阶段。
- 基线：HEAD=`f8a1a2d2`；台账 `data/status/room_sweep_{0..4}.json`；P0 探针原始件在 `/tmp/p0/`
  （`*.state.jsonl` = rust `--state-out` seam 面；`analysis/traces/case-p0-*.frames.jsonl` = emu seam 面）。
- 纪律：只读复核；唯一写入 = 本文件。未跑全量 `room_sweep.py` / `gates/cli.py l4` / `cargo test`。

---

## 0. 结论摘要（先看这三条）

| # | 原 P1 主张 | 定谳 | 处置 |
|---|---|---|---|
| 1 | 偏移分两账：`fc_base_shift`（常量 −1，载房窗 dropped 缺陷）+ `fc_lag_steps`（掉帧，由对齐消除） | **不存在常量相位**。两腿 seam 相位直接可比，唯一量是 `D(f)=FC_rust(f)−FC_emu(f)`＝累计丢帧赤字；`D` 在载房窗内就变号（a0：f784 −1 → f903 **+1**，emu 净丢 2 iteration），判定窗内继续阶跃（r31：f1706 起每 5 帧 +1，f1741 达 **+8**）。「−1」只是 a0 首采点的 `D` 取值 | 取消两账；`D` 单列，且**恒落账**（D4 保真偏差），不作对齐参数 |
| 2 | 对齐算法：FC 模 256 展开 → 候选集 → 取最小 g → RAM 预门 → 像素 | **候选集结构性 ≤1**：spin-exit trace 只在缝帧出行且 FC 每值恰一行（a0 spin 1004 行/1020 帧，全部相邻步进=1、重复 0）⇒ 「多候选/min g/`fc_phase_ambiguous`」全部不可达。真正需要的不是搜索，而是**用等迭代号把两腿对上**：`g` 由 `FC_emu(g)==FC_rust(c)` 唯一确定 | 删模 256 展开与 min-g；改为 seam 表单点查表 + 等式断言 |
| 3 | RAM 面预门 `$2D/$FD/$19/$1A` + 第二遍 `sample_point spin-exit` | 两处硬伤：(a) `analyse_trace` 的 `mr_window_ok` 要求 `entry..entry+6` **逐帧**有行（`tools/movie_ref/room_sweep.py:306-313`），spin-exit 下无缝帧无行 ⇒ 重跑的每一房都会被判 `left-gameengine`，**采样点不能改**；(b) emu 采集点的 RAM 是 endFrame 相位、PNG 是缝相位，非缝帧还是**混态**（r31 f1720 ram `$2D=160` 停在 f1719，`$FD` 已推到 232；`.lua.log` 实录 `SHOT-SEAM f=1720 at=1721 ... reason=frames+1-fallback`）⇒ 预门只能在缝帧上做，且必须用 trace 面而非采集点 ram.hex | 保持 endFrame；缝帧 = endFrame 行里 FC 值**首次出现**的帧（结构可证）；RAM 预门改在 trace 面（两腿 seam 面）执行 |

附带发现（与 FC 无关，但会污染一切「采集点 error」推断）：

- **192/192 个 `status=="error"` 采集点全部是同一个 bug**：`build_case` 对每个 `captures` 偏移发
  `capture entry+c-1 prev{c}`（`room_sweep.py:176`），又发 `capture entry+frames-1 prevend`
  （`:177`）。当 `c==frames` 时两行**落在同一帧**（a0 各房 1019、a1/a2 等同理），Lua `capAt[frame]=tag`
  后写覆盖（`tools/scenario_capture.lua:1316-1321`）⇒ `prev{c}` 永不出图 ⇒ 台账记 `error`。
  实测：`prev120` 共 47+46+37+21+41=192 点，与 error 总数 192 精确相等；重复帧 192 处，无一例外。
  即「采集点 error」从来不是 harness 抖动，是**用例生成撞车**。
  另：`prev*` 标签在判定里被显式跳过（`room_sweep.py:784` "prev* 只作 rust 参考帧，不参与判定"），
  所以这 192 点**不影响任何房间的 ok/mismatch**（红线因此天然安全），但会污染任何「按 error 计数推断稳定性」的分析。

---

## 1. 证据基座（本重复核新增，全部可复算）

### 1.1 两腿 seam 面均可直接读，且相位一致

- rust：`crates/game-core/src/bin/rust_screenshot.rs:585-587` 在 `engine.tick_frame_with` 回调内打印
  `[fc] <frame> FC $1F`，并在同一点 push `seam_snap`；`--state-out` 的每行 `{"f":N,"w":{"zp":"<512hex>"}}`
  即该帧 seam 时刻的 256B 零页。实测 a0_r02：f959=(`$2D`=175,`$FD`=118,`$19`=8,`$1A`=1)、
  f960=(176,120,7,1)、f903=(119,6,8,1)、f899=(116,0,3,1)、f900=(117,2,2,1)。
- emu：`analysis/traces/<case>.frames.jsonl` 的 `w.FC`/`w.ScrollX` 是同相位 seam 值。
  实测 a0_r02：f783 FC=0 → f784 FC=1 → f903 FC=118 → f959=174 → f960=175。
- **info**：`--trace` 的 `[fc]` 行到不了台账判据（`tools/l4_diff_run.py:315 run_rust` 只回显
  `[rust]`/`[bundle]` 行，不传 `--trace`）。rust 面要进判据只有 `--state-out` 一条路，而
  `execute_case`（`:1024`）不传 `state_out`；只有 `do_run` 的 `align` 动作（`:1293-1310`）传。
  ⇒ P1.2 必须在 `execute_case` 里补 `state_out`，否则「内存面对齐」在台账链路里根本取不到数。

### 1.2 唯一偏移量 D(f) 的实测形状

| 房 | 区间 | 事实 | 推论 |
|---|---|---|---|
| a0_r02 | f784..f903 | emu FC 1→118（119 帧 117 次递增，**丢 2**）；rust 同区间逐帧 +1（1005 行 state 全无重复） | `D(f784)=−1`、`D(f903)=+1`；`D` 在**载房窗内就变号**，不是常量 |
| a2_r31 | f1554..f1603 | emu FC 0→47（49 帧 47 次，丢 2）；f1603 两腿同为 47 | 该段 `D` 两端都为 0（丢帧与复位点错位相抵），常量假设在此段「碰巧成立」 |
| a2_r31 | f1603..f1741 | emu f1706=149 vs rust 150，此后每 5 帧再丢 1，f1741 落后 8 | 判定窗内 `D` 阶跃上升；**同一房内前后两个采点的正确偏移不同** |

**跨腿 seam (FC,ScrollX) 精确双射**（r31，1715–1725 逐点命中，无一例外）：

| rust seam | (FC,SX) | = emu seam | 视频帧差 |
|---|---|---|---|
| f1715 | (159,228) | f1718 | 3 |
| f1717 | (161,232) | f1721 | 4 |
| f1719 | (163,236) | f1723 | 4 |
| f1720 | (164,238) | f1724 | 4 |
| f1721 | (165,240) | f1726 | 5 |
| f1725 | (169,248) | f1731 | 6 |

⇒ 两腿在 seam 面是**同一台状态机**，视频帧差 = 累计丢帧赤字；`$FD` 在等 FC 对上也逐值相等
（236↔236、238↔238）。所以 `$FD` 可作独立校核，不需要任何「相位约定常数」。

### 1.3 现有 `aligned` 字段语义已死

`room_sweep.py:788-791` 对 `ok` 房**无条件**置 `aligned=True, aligned_offset=-1`；实测 a0 的 47 房
中有 36 房所有判定点 `best_pairing=="same"`（a1 38/6、a2 30/3、a3 12/1、a4 18/1），却照样带
`aligned_offset=-1`。⇒ 该字段记录的是「考虑过 −1 对齐」，不是「用了 −1」，**不能作为「已吸收」的证据**，
也不能拿它做 ratchet。P1 必须显式写 `fc_pair`（实际用的 g/c）而不是沿用此字段。

### 1.4 22 个 shift1 点 = 时间错位，不是空间自由度

所有 `best_pairing=="shift1"` 点的 `shift` 都是 `dx=2, dy=0, explains=true`；`shift_probe`
（`room_sweep.py:514-556`）拿**同帧同名**两张 PNG 做 ±4 平移搜索，而滚动是 2px/帧 ⇒ 它找到的
`dx=2` 就是把「差 1 帧」解释成「差 2 像素」。22 个点分布在 a0 的 11 房
（room 2,14,24,25,28,29,30,37,38,41,43），不是计划文档写的「22 房」。

### 1.5 采集点相位与 shot_at 可回溯

- 缝帧采集：`.lua.log` 实录 `SHOT-SEAM f=1719 at=1719 tag=prevend ... reason=spin-exit-seam`（at==f）。
- 非缝帧采集：`SHOT-SEAM f=1720 at=1721 tag=f120 ... reason=frames+1-fallback`（at=f+1）。
- `captures.jsonl` 有 `f/tag/shot/ram/cpu`，**无 shot_at**（`scenario_capture.lua:1322-1327`）；
  但 `analysis/traces/<stem>.lua.log` 里 `SHOT-SEAM/at/reason` 全在 ⇒ `shot_at` 可**离线反解**，
  不需要为「拿相位证据」重跑任何房。

### 1.6 采集点 RAM 的相位实测（决定「预门拿谁跟谁比」）

同一帧的**两腿采集点 ram.hex 本来就不该相等**（差 D 个迭代），实测（`shots_rust/` = rust 腿，
`analysis/traces/` = emu 腿，同一 stem）：

| 房 | 点 | rust capture ram (`$2D`,`$FD`,`$19`,`$1A`) | emu capture ram | 结论 |
|---|---|---|---|---|
| a0_r02 | f00959 prev60 | (175,118,1,0) | (174,116,2,0) | 同帧不等（差 1 迭代） |
| a0_r02 | f00960 f60 | (176,120,8,0) | (175,118,1,0) | 同帧不等 |
| a0_r02 | 跨腿对 | rust f959 (175,118) | emu f960 (175,118) | **等迭代对成立**（且该对像素 0） |
| a2_r31 | f01720 f120 | (164,238,3,0) | (160,232,4,1) | 同帧不等（差 4 迭代；emu 侧还是混态） |

⇒ 「预门」必须是**配对后**的等迭代比较（rust c ↔ emu c+D），不能是「同帧 ram 相等」。
Kraid31 的等迭代对已实测：rust capture(1720) RAM=`$2D 164/$FD 238`（`shots_rust/_tmp_sweep_278073-1795412324_a2_r31_f01720_f120.ram.hex`）
== emu 长窗 seam f1724=(164,238)（`analysis/traces/case-p0-2-31-long.frames.jsonl`）⇒ **state 门必过**，
只剩像素未测。`$19` 在同帧/跨腿都出现 1 与 8 的差（倒计数随迭代内相位变化），**不可进硬门**。

---

## 2. 对抗复核：逐条证伪与替代

### A1 采集（改动面与窗口）

**原设计**：第一遍加 `watch 002D`（称零成本），只对 mismatch 房跑第二遍，窗口按实测 Δ 定量，
`g ≥ c`，外延判定期末地平线（Kraid31≈1725、Kraid29≈1733）。

**证伪**：

1. **「零成本」不成立**。`watch` 进 case 文本会改 `scenario` 锚（`tools/l4_diff_run.py:223
   compute_anchors`），`emu_cache_hit`（`:287`）随即全失效 ⇒ 第一遍 193 房（ran=192，a4 1 skipped）的 emu 腿全重跑（约 1.8h），
   而第一遍本来**不需要** FC（ok 房已判完）。替代：**第一遍一字不改**；FC 只进重跑用例。
2. **「会不会覆盖第一遍 trace」不会**。`_run_token`（`room_sweep.py:52`）= `pid-启动秒`，
   `case_name`（`:124`）带 token ⇒ 任何重跑天然是新 stem，旧 `analysis/traces/<stem>.*` 不被改写。
   反过来说，「用 `--fresh-emu`」也**不需要**：新 case 的 sha 变了，缓存本来就不命中。
3. **真实风险是 `shots/` 与 `analysis/traces/` 的同名碰撞**只发生在**同进程同 stem 重跑**（`:1045-1050`
   只清 `<TOKEN>` 自己的 `cases/*.txt`）；跨进程无碰撞。P1.2 应在跑前对第一遍全部产物（193 房）的
   `(path,size,mtime)` 拍快照，跑后比对，作为「未污染」的机械证据。
4. **窗口必须包含 `g`，而 `g` 只能来自「等迭代号」而不是固定 Δ**（见 A2）。`scenario_run`/CLI 侧
   `--captures` 必须落在 `1..--frames`（`room_sweep.py:1048-1050`），所以外延只需 `--frames ≥ max(g)-entry`。
   Kraid31 的正确地平线：目标 = **rust 采集点自身的 `$2D`**（不是 trace 行），rust capture(1720) 的
   `$2D`=164 ⇒ emu seam FC=164 在 **f1724**（P0 长窗 trace 实测 (164,238)），窗口到 `captures=128`
   才安全；计划写的 1725 方向对、推导错（它用 trace 行 164/163 混算），Kraid29 **未定谳**（无 P0 长窗）。
5. **采样点不能改 spin-exit**（见摘要 #3a）：`mr_window_ok` 逐帧要求（`room_sweep.py:306-313`）
   会被缺行打成 `left-gameengine`；`analyse_trace` 还依赖 `ENTRY_FRAME-1` 的 `MainRoutine` 单点
   （`:295-297`）。替代：保持 endFrame，缝帧用「FC 值首次出现」导出（见 A2）。

**替代方案（P1.2 定案）**：重跑用例 = 原 case + `watch 002D FC`（+`watch 0019`/`watch 001A` 供诊断），
captures = `[c, c+1, …, c+10]`，**不发 `prev{c}`**（新增 `aligned_prev=False` 参数，默认 True 保持第一遍字节不变）。
理由：等迭代配对只需要 emu 侧在 `c+D` 有图，rust 侧 `c` 已有图；`prev` 机制和 FC 无关，发了只会撞车。

### A2 对齐（模 256 / 候选 / 最小 g / RAM 预门）

**证伪**：

1. **候选集 ≤1**。spin-exit trace 只在缝帧出行（`scenario_capture.lua:1218-1224`，`samplePoint=="spin-exit"`
   才 `sampleTrace`），且 FC 每值恰一行（实测 a0 1004 行/1020 帧、相邻步进恒 1、重复 0）。endFrame 的
   「同 FC 双行」是**掉帧帧与前一帧同值**，不是真的两个缝（r31 f1720 无 seam 行即为证）。
   ⇒ 「模 256 展开 + 取最小 g + `fc_phase_ambiguous`」是给不存在的问题写的代码。**删**。
2. **模 256 只在窗口 > 255 iteration 时才需要**。重跑窗口 ≤ 133 帧 ⇒ 断言
   `frames − entry < 256`，超了直接 `fc_window_exceeds_256` fail-loud，不做展开。
3. **`fc_base_shift≠0` 时「最小 g」判据确实不成立**：`D` 在同一窗内变化（r31 f60 的 D=0、f120 的 D=4），
   任何「g−c 取最小」都等价于「取 D 最小值」，会把正确的等迭代对排除。替代：**等式**，不是极小化。
4. **RAM 预门会挡真差异**：现状 `classify_layer`（`:558`）是在**同帧同名**对上算的，本来就含 1 帧错位；
   若预门再拿 `$2D/$FD` 不等直接判失败，则任何 D>0 的房都会变成「RAM 不同 ⇒ 失败」，像素判定永远跑不到。
   替代：预门**只在等 FC 的 seam 对上**做，且只作**证据**（`state_match: {2d,fd,19}`），不单独输出 verdict；
   判定权仍归像素（`diff_pair` 零容差）。`layer` 分类只在配对确定后重算（P1.4）。

**定案算法**（`tools/movie_ref/fc_align.py`，纯函数）：

```
seam_table(rows)            -> [(f, FC)] 仅缝帧：endFrame 行取 FC 首次出现的 f；断言步进 ∈{0,1}
target(emucap_c, ruststate) -> FC_rust(capture c).zp[0x2D]        # 采集点自身 $2D，不是 trace 行
g                           -> 唯一 f 使 FC_emu(f) == target      # 查表；无则 fc_horizon_short
sanity                      -> g - c == FC_rust(c) - FC_emu(c)    # D 自洽；不等则 fc_jump
state_gate                  -> $FD_emu(g) == $FD_rust(c)          # 实测 236↔236、238↔238
pixel                       -> diff_pair(emu_png(g), rust_png(c)) == 0
deficit                     -> fc_deficit = g - c                 # 恒落账
```

> 说明：`D(f)`（同视频帧比）与 `lag = g−c`（配对帧差）是同一个数的两种读法；台账只记 `fc_deficit`，
> 另记 `fc_deficit_profile`（窗内逐点）以暴露阶跃。

### A3 防掩盖（空间平移 vs FC 位移互斥）

**证伪**：R4 的互斥规则会把 22 个 shift1 点判成 `masked_ambiguous` —— 它们正是 `dx=dy=0` 的
候选位（`shift` 是 probe **算出来的最优平移**，不是输入），而 `dx=2` 已被证明是错 1 帧的滚动泄漏
（§1.4）。若按 R4 执行，11 个已 ok 的 Brinstar 房会被打成含糊态，直接违反红线。

**可判定仲裁（定案）**：

1. 先配对（A2），**配对后**才允许 `shift_probe`，且只在**等 FC 对**上跑。等迭代 ⇒ 滚动寄存器必然相同
   ⇒ 最优平移必须 `dx=dy=0`。
2. 于是三类可判定：
   - `dx=dy=0, diff=0` → `ok_fc_aligned`；
   - `dx=dy=0, diff>0` → `fc_align_failed`（真内容差）；
   - `(dx,dy)≠0 且 residual≤raw*0.5` → `fc_pair_suspect`（配对错，回到 A2 重查 `$FD`/D），**不是**
     `masked_ambiguous`，因为掩盖只可能发生在**错配**上。
3. `masked_ambiguous` 仅在「等 FC 对不可得（`fc_horizon_short`）**且** 平移能解释同帧同名差」时成立
   —— 这是唯一能同时说明「为什么没对齐」与「为什么看起来像对齐」的组合，可从现有产物算出。
4. `fc_shift` 不再需要「≠0 即要求 dx=dy=0」这种硬互斥：`fc_shift` 本身就是 `lag`，与 `dx/dy` 不同维度
   （时间 vs 空间），把两者写成互斥是把「错 1 帧」误建模成「平移」。

### A4 fail-loud 五态可判定性

| 状态 | 可判定？ | 判据（可算自哪个产物） | 缺仪表 |
|---|---|---|---|
| `fc_align_failed` | 可 | 等 FC 对上的像素差 >0（现有 `diff_pair`） | — |
| `fc_phase_ambiguous` | **不可达，删** | 需 \|C\|≥2；实测 \|C\|≤1（§A2-1） | — |
| `fc_ram_desync` | 可（需改） | `$2D` 相等但 `$FD` 差 >2，或 seam 表在等 FC 处 `$19` 方向不符 | 需 rust `state_out`（A1-1 补） |
| `fc_jump` | 可 | 窗内 `D(f)` 非单调，或 `g−c ≠ FC_rust(c)−FC_emu(c)` | 需 `watch 002D` |
| `masked_ambiguous` | 可（按 A3-3 收紧后） | `fc_horizon_short` ∧ 平移可解释同帧同名差 | 需 `shot_at`（lua.log 可回溯） |
| **`fc_horizon_short`（新增）** | 可 | 表内无 `FC==target`，或 `g>c+10`（窗口不够） | — |
| **`fc_capture_phase_off`（新增）** | 可 | 该点 `shot_at ≠ f`（mismatch 房 24 个点可离线查出） | 无（lua.log） |
| **`fc_window_exceeds_256`（新增）** | 可 | `frames−entry ≥ 256` | — |

> `fc_phase_ambiguous` 删除的理由要写进 P1 代码注释：留着它会让人以为「多候选」是现实风险，
> 从而在算法里塞模 256 展开与 min-g 逻辑（本复核正是在此发现设计漂移）。

### A5 台账兼容

- 现 schema（`data/status/room_sweep_2.json` 实读）：顶层 `tool/entry_preset_version/entry_frame/
  entry_preset/generated_at/args/area/area_name/ok_definition/summary/skips/rooms/note/note_source/elapsed_s`；
  `rooms[]` 含 `status/ok/diff_pixels/captures/.../aligned/aligned_offset`；`captures[]` 含
  `frame/tag/status/diff_pixels/shift/layer/aligned_diff_pixels/best_pairing/best_diff_pixels`。
- `build_doc`（`:896`）`judged_statuses` 元组（`:899-901`）是**算术自洽的闭合集**：`planned==ran+skipped`、
  `ran==Σjudged`（`:918-925`）。新增状态必须同时进 `judged_statuses` + `summary` 初始化，否则
  `counts.get` 静默吞掉、`already_ok` 断言炸。
- 旧台账可读性：新字段一律**可选**，缺省 `None`；`schema_version` 只在新文件里写；
  旧房记录原样透传（`--resume` 只沿用 `r.get("ok")`，`:1084-1086`）。
- 门禁：`gates/` 全目录 grep 无 `room_sweep`；`gate_l4` 只跑 `tools/l4_diff_run.py run-all` +
  `data/status/l4_ratchet_baseline.json`，与本台账无耦合 ⇒ **无 ratchet 会被新状态打爆**。
- 唯一会受影响的消费方：`tools/movie_ref/room_sweep_stats.py` 按 `status=="ok"`/`"mismatch"` 精确取值
  ⇒ `ok_fc_aligned` 会被**静默漏计**（不是门禁，但统计会骗人）。P1.4 必须同步改它，或在 `summary`
  里显式给 `ok_aligned` 计数并断言 `ok+ok_fc_aligned == 原 ok 数`（红线机械化的另一半）。

### A6 红线（Brinstar 47 / Norfair 44 判定不得变化）

机械判据（两条都要过，缺一不可）：

1. `git diff --exit-code data/status/room_sweep_0.json data/status/room_sweep_1.json`
   —— P1 输出写**独立文件** `data/status/room_sweep_fc_<area>.json`，主台账不被触碰，此条应恒为 0。
2. `python3 tools/movie_ref/fc_redline.py --base 0,1 --new data/status/room_sweep_fc_0.json,...`：
   对 a0/a1 每房断言 `status` 集合不变（a0 全 ok、a1 44 ok + 2 mismatch），且逐采集点
   `(frame,tag,status,diff_pixels)` 四元组全等；任何不等即 exit≠0。允许新增的只有 `fc_*` 字段。

---

## 3. 分阶段实施

> 每阶段末尾都要求 `python3 gates/cli.py status` 仍为 STATUS: OK（实测现状 PASS=23 FAIL=0 SKIP=3
> TOLERATED=1）。**不跑** `gates/cli.py l4`（慢层，与本改动无关）。

### P1.0 先修「采集点 error 全是撞车」（30 min，零重跑）

- 改：`tools/movie_ref/room_sweep.py:173-175`（`prev{c}` 生成）与 `:177`（`prevend`）——
  `build_case(..., aligned_prev=True)`，`aligned_prev=False` 时不发 `prev{c}`；默认 True 保持第一遍字节不变。
  同时判据块（`:676-807`）对「同帧双 tag」显式记 `status="dup_frame"` 并给 `reason`，不再落 `error`。
- 验收：`python3 -c "import tools.movie_ref.room_sweep as rs; t=rs.build_case(0,0,120,[60,120]);
  import re; print(len(re.findall(r'^capture (\\d+) ', t, re.M))); ..."` 断言 `capture` 帧号无重复。
- 期望：帧号集合大小 == 行数；`prev120` 不再与 `prevend` 同帧。
- 回退：单 commit 只动这两处；若判据改动影响现有 ok 房，撤回判据侧、只留 `build_case` 参数。

### P1.1 仪表（1h，零重跑）

- `tools/l4_diff_run.py:1024 execute_case`：把 `state_out=` 传给 `run_rust`（`:315` 已支持），
  产物落 `analysis/traces/<stem>.state.jsonl`（与现有 `align` 动作同格式，`load_state_rows` `:416` 可读）。
- `tools/scenario_capture.lua:1322-1327`：`capF` 行加 `shot_at`/`shot_reason`（`fulfillShot` 已知道，
  `:269-305`）；不改采样点、不改帧序。
- `tools/movie_ref/room_sweep.py:139-146`：`build_case(..., fc_watch=False)`，True 时插
  `watch 002D FC` / `watch 0019` / `watch 001A`；默认 False ⇒ 第一遍 case 文本不变。
- 验收：`python3 tools/l4_diff_run.py run <stem>` 后 `<stem>.state.jsonl` 首行可解出 `zp`（512 hex），
  且 `captures.jsonl` 每行有 `shot_at`。
- 期望：state 行数 == frames；`shot_at` 与 `.lua.log` 的 `SHOT-SEAM ... at=` 逐点相等。
- 回退：`state_out` 是既有能力，失败则退回 `align` 动作产出的 state 文件（同一 `load_state_rows`）。

### P1.2 窗口采集（1.5h，~36 房 × 1 腿）

- 新 `tools/movie_ref/fc_retry.py`：读 `data/status/room_sweep_<area>.json` 取 `status=="mismatch"`
  的房（a1=2, a2=4, a3=8, a4=22，共 **36**；a0=0），逐房 `build_case(..., fc_watch=True,
  aligned_prev=False)`，captures = `[c, c+1, ..., c+10]`，`frames = max(captures)+3`，走
  `l4_diff_run.run`，输出 `data/status/room_sweep_fc_<area>.json`。
- 验收（单房先跑）：`python3 tools/movie_ref/fc_retry.py --areas 2 --rooms 31 --jobs 1 --limit 1`。
- 期望：caps 全产（11 点）、`state.jsonl` 存在、trace 里 `FC` 列存在且缝帧可导出；`g` 落在窗口内
  （Kraid31 预期 g=1724，lag=4）。
- 回退：窗口不够（`fc_horizon_short`）时把 captures 上界 +5（`c+15`）重跑该房；仍不够则该房
  记 `fc_horizon_short`，不伪造。

### P1.3 对齐算法 + 单测（2h）

- 新 `tools/movie_ref/fc_align.py`：`seam_table / target / find_candidate / state_gate / diff_pair /
  judge`（纯函数，无 I/O；输入 = 行列表与三张 PNG 路径）。
- 改 `tools/movie_ref/room_sweep.py:674-807`：mismatch 房走 `fc_align.judge`；`ok` 房路径**不动**
  （保证红线）。判定输出 `fc_pair{emu_frame,rust_frame,fc_deficit,state_match,shift,shot_at}`。
- 验收：`python3 -m pytest tools/tests/test_fc_align.py -q`。
- 期望：§4 全部用例绿。
- 回退：算法与判据解耦（`judge` 只返回结构化结果），可单独关闭，台账退回现状。

### P1.4 判定与台账（1.5h）

- 改 `room_sweep.py:899-901` `judged_statuses` 增 `("ok_fc_aligned","fc_align_failed","fc_horizon_short",
  "fc_ram_desync","fc_jump","fc_capture_phase_off","fc_window_exceeds_256","masked_ambiguous","dup_frame")`
  并同步 `summary` 初始化（`:902-905`）。
- 新 `tools/movie_ref/fc_merge.py`：把 `room_sweep_fc_<area>.json` 合进主台账，写 `fc_alignment`
  顶层块（`schema_version=2`、`fc_deficit_basis`、`redline`），**默认 dry-run**，`--commit` 才落盘。
- 改 `tools/movie_ref/room_sweep_stats.py`：`ok`/`ok_fc_aligned` 都计入，并断言
  `ok + ok_fc_aligned == 合并前 ok 数`。
- 验收：`python3 tools/movie_ref/fc_merge.py --dry-run` → 打印 a0 47/47、a1 44/44 不变；`--commit` 后
  `python3 tools/movie_ref/fc_redline.py --all` exit 0。
- 期望：a2 的 4 个 mismatch 里至少 Kraid31 转 `ok_fc_aligned`（fc_deficit=4），其余按像素真实结果定。
- 回退：`git checkout data/status/room_sweep_*.json` 即回滚（合并器只写这些文件）。

### P1.5 回归与红线（1h）

- 跑 `python3 tools/movie_ref/fc_redline.py --all` + `python3 gates/cli.py status` + 
  `python3 -m pytest tools/tests -q -k "fc or room_sweep"`。
- 写 `docs/findings/fc-deficit-cadence.md`：登记 `D(f)` 实测（a0 载房窗净丢 2、r31 窗内 +8），
  并把「rust 不模拟掉帧」按 D4 记入 `docs/bug-ledger.md`（`BUG-xxx`，Stage A 不修）。
- 验收：redline exit 0、gates status OK、bug-ledger 有对应条目。

---

## 4. 单测清单（`tools/tests/test_fc_align.py`，pytest 9.1.1 已可用）

1. `test_seam_table_drop`：endFrame 行含重复 FC（r31 f1719/f1720 形）→ 只保留首次出现帧；断言值唯一、步进 ∈{0,1}。
2. `test_seam_table_missing_row`：缺行（spin-exit 形）→ 显式报错，不静默补。
3. `test_find_candidate_unique`：目标存在 → 唯一 g；断言 `g−c == FC_rust(c)−FC_emu(c)`。
4. `test_horizon_short`：目标在窗外 → `fc_horizon_short`，不返回最近邻。
5. `test_mod256_guard`：`frames−entry=300` → `fc_window_exceeds_256`；`=255` 正常。
6. `test_deficit_profile`：a0 形（−1→+1）与 r31 形（0→+8）→ 返回阶跃列表，不是常量。
7. `test_pair_state_gate`：用真实 RAM 串（a0 f960=(175,118,1) / f959=(175,118,1)）→ state_match 真；
   `$FD` 改 124 → `fc_ram_desync`。
8. `test_align_failed_pixels`：等 FC 对 diff=11170（r31 f60 `aligned` 实测值）→ `fc_align_failed`。
9. `test_jump`：D 从 4 跳到 1（非单调）→ `fc_jump`；`g−c` 与 FC 差不等同判。
10. `test_pair_suspect_not_masked`：错配对上 `dx=2` 可解释 → `fc_pair_suspect`（**不**是 masked）。
11. `test_prev_collision_regression`：`build_case(0,0,120,[60,120])` → capture 帧号无重复；
    `aligned_prev=False` 时无 `prev` 行。
12. `test_ledger_backward_compat`：`build_doc` 混入一条旧 schema 房（无 `fc_pair`）+ 一条新 → 算术自洽、
    旧字段原样保留、`schema_version` 存在。

---

## 5. Kraid31 集成验收（用真实产物）

```bash
python3 tools/movie_ref/fc_retry.py --areas 2 --rooms 31 --jobs 1 \
        --captures 60,120,121,122,123,124,125,126,127,128
python3 tools/movie_ref/fc_judge.py --area 2 --room 31 --show-pairs
```

期望（可由 P0 产物预演，全部有实测锚）：

| 采点 | emu f | rust f | FC | `$FD` | 像素 | 说明 |
|---|---|---|---|---|---|---|
| 60 | 1660 | 1660 | 104/104 | 118/118 | 0 | D=0，现有 `same` 已对 |
| 120 | **1724** | 1720 | 164/164 | 238/238 | **待测** | D=4；现有 `aligned(-1)` 差 11170、`same` 差 14755 都是错配对 |

- 通过：`fc_pair{emu_frame:1724, rust_frame:1720, fc_deficit:4, state_match:true, shift:[0,0]}` + `diff=0`
  ⇒ `ok_fc_aligned`；窗内 `fc_deficit_profile` 显示 60→0、120→4。
- 不通过：diff>0 ⇒ `fc_align_failed` 并把该对 PNG 落 `analysis/fc_pairs/`（人工看图），
  **不**改回 `aligned_offset`。
- 预演（不跑）用 P0 长窗即可确认表的前两列：emu seam f1724=(164,238)（`case-p0-2-31-long.frames.jsonl`）
  == rust seam f1720=(164,238)（`/tmp/p0/long.jsonl`）⇒ 等迭代对存在且唯一。
- 已实测（state 面）：rust capture(1720) RAM=`(164,238)`（`shots_rust/…_f01720_f120.ram.hex`）
  == emu 长窗 seam f1724=`(164,238)` ⇒ §A2 的 `state_gate` 必过。
- 未定谳：该对的**像素**结论尚未测过（P0 只测了 state 面，原台账里 1720 只有错配对的 14755/13113），
  故集成验收必须真跑；若像素非 0，则是**真内容差**，按 `fc_align_failed` 落账，不得回退成 `aligned_offset`。

---

## 6. 风险与回退（含 2 天收口不了的降级）

| 风险 | 概率 | 影响 | 缓解/回退 |
|---|---|---|---|
| rust `state_out` 让每房多写 20MB（120 帧） | 高 | 磁盘/IO | 只对 36 房开；跑完即删或 gzip；state 只留 `$2D/$FD/$19/$1A` 摘要 |
| `D` 在同一房内非线性（r31 每 5 帧 +1） | 已发生 | 单点对可能仍错 | 每采点独立求 g；`fc_deficit_profile` 暴露阶跃；不做全局拟合 |
| 36 房重跑撞上 1.8h 级成本 | 中 | 时间 | 窗口只 11 点、`frames≈133`；a0/a1 完全不动 ⇒ 实际 ≈ 36×0.7min |
| `ok_fc_aligned` 被误读成「与原版等价」 | 高 | 纪律 | 台账 `ok_definition` 追加一句：FC 对齐只证「等迭代内容一致」，**不证**掉帧节奏一致；节奏差异记 `fc_deficit` + bug-ledger |
| 2 天内收不了口 | — | — | **降级路径 D1（推荐）**：只做 P1.0+P1.1+P1.3 的判据部分，让 mismatch 房输出 `fc_deficit` 与 `fc_pair` **诊断字段**，房间状态保持 `mismatch`，像素仅作提示（`fc_align_hint`），不改判定权；`masked/ram_desync` 五态缩减为三态（`fc_horizon_short`/`fc_align_failed`/`fc_jump`）。降级版仍能回答「这些 mismatch 是内容差还是节奏差」，只是不把结论转成 pass。 |

---

## 7. 未定谳（必须显式保留，不许写成结论）

1. **Kraid29 的地平线**（计划写 1733）：无 P0 长窗 trace，`g` 未算；须 P1.2 实测后再填。
2. **r31 f120 的像素对是否真为 0**：state 面对已证（(164,238) 唯一），像素面从未测过。
3. **`$19` 能否进硬门**：a0 匹配对上 `$19` 相等（1/1），但它是倒计数、且跨房样本 1 例，样本不足；
   P1.3 只作 `state_match` 的**记录项**，不作 `fc_ram_desync` 的触发条件。
4. **38 个 error 点（非 prev120）**：本次普查显示 error 恰等于 prev120 数（192），但 a4 有 1 房 `skipped`，
   其 `error` 归因未逐房验证；P1.0 的 `dup_frame` 重判后须复核「error 归零」。
5. **emu 侧 `$19/$1A` 的 watch 是否会改变 case sha 进而让 36 房以外产物失效**：会（新 stem），但旧 stem
   不被改写；「不失效」仅指旧证据仍在盘上，非「仍可复用」。

## P1.0 黄金用例实测（2026-11-23，Kraid 31，一次性验证三件事）

探针 `cases/case-p0b-r31`（`build_case(2,31,140,[60,120..128])`，删掉全部 `prev{c}` 行，
加 `watch 002D/0019/001A`；跑完已删）。配对规则 = **等 `(FC, ScrollX)` 唯一确定 g**：

| rust 帧 c | FC | emu 配对 g | lag | 等 FC 配对 diff | 同帧号 diff |
|---|---|---|---|---|---|
| 1660 | 104 | 1660 | 0 | **0** | 0 |
| 1720 | 164 | 1724 | 4 | **0** | 14755 |
| 1721 | 165 | 1726 | 5 | **699** | 16202 |
| 1722 | 166 | 1727 | 5 | **0** | 16351 |
| 1723 | 167 | 1728 | 5 | **0** | 16362 |
| 1724+ | — | 1729+（超出采集集） | 5+ | 需扩采集 | — |

结论：
1. 等 `(FC, ScrollX)` 在 r31 上**唯一**（每帧恰一候选），复核的「等 FC 唯一确定 g」成立；
2. r31 的 15–18K px 失配中，**可验证的 5 个采集点有 4 个在等 FC 配对下归零** ⇒ 该房失配主因是节奏（掉帧），不是内容差；
3. **c=1721 残留 699px** 是唯一候选真差，需单独取证（也正是 P1.5 之后第一个要查的点）；
4. `lag` 从 0 涨到 5–6，与 P0 的赤字曲线一致。

## P1.0b 扩窗实测（同日，Kraid 31，captures 120..140）

等 `(FC, ScrollX)` 配对，逐帧结果：**18 个可验证帧里 15 个 0px**，残留三帧：

| rust c | FC | emu g | lag | diff | bbox |
|---|---|---|---|---|---|
| 1721 | 165 | 1726 | 5 | 699 | (32,80)-(221,183) |
| 1725 | 169 | 1731 | 6 | 718 | (24,80)-(213,183) |
| 1729 | 173 | 1736 | 7 | 697 | (16,80)-(205,182) |

规律：**每 4 帧一次**（1721/1725/1729），diff 量恒 ~700px，bbox 左边界随滚动每 4 帧左移 8px
（32→24→16），高度恒 y80–183。滚动速度 2px/帧 ⇒ 4 帧正好一个 8px **瓦片列**。

未定谳的两个候选：
- (a) **显示滞后一迭代**：emu 慢迭代跨两视频帧，其中一帧画面仍是上一迭代的渲染 ⇒ 该帧的
  像素比游戏时间晚一拍。三帧残留的周期性（4 帧）与丢帧周期（5 帧）不成整除，不符合简单滞后，
  需要用「该 g 是否落在 `SHOT-SEAM ... at=g+1` 回退帧」来判定（`tools/scenario_capture.lua`
  的 seam 日志已打印 `at=`）。
- (b) **真内容差**：某个只在瓦片列边界更新的元素（例如列流送写入的瓦片/属性）差一格。

判定方法（下一步）：对 c∈{1721,1725,1729} 各取 emu `g` 与 `g-1`、`g+1` 三帧与 rust `c` 比，
若 `g±1` 中存在 0px，则 (a) 成立；否则 (b)。

## P1.0c 邻帧判定（同日，Kraid 31，endFrame 口径）

对残留三帧各扫 emu `g..g+9`：

| rust c（状态） | emu 同状态帧 | diff | 邻帧最小 diff |
|---|---|---|---|
| 1721（165,240） | 1726 | 699 | **699**（1725 也是 699） |
| 1725（164,240） | 1725/1726 | 699 | **718**（1730/1731 都是 718） |
| 1729（168,246） | 1735/1736 | 697 | **697** |

两条硬事实：
1. **endFrame 口径下存在「同 FC 相邻帧」**：emu 1724=(164,238) 与 1725=(164,240) 的 `FC` 同为 164，
   仅 `ScrollX` 不同；1729 与 1730 同为 168。⇒ P0 里「同 FC 相邻帧 0 处」只在 **spin-exit** 口径下成立，
   endFrame 口径不成立（P0 那批探针正是 spin-exit 采的）。
2. 残留 699/718/697 **跨越一对重复帧恒定**（1725 与 1726 同值、1730 与 1731 同值）⇒ 画面在两帧内
   不变，说明这 ~700px 是「同一游戏时间下 emu 显示比我方晚一拍渲染」的残差，而不是随时间放大的内容差；
   其量级恒定也支持这一点（若为内容差，滚动会让 bbox 与像素继续累积）。

结论（对算法的影响）：**等 `(FC, ScrollX)` 配对不足以唯一确定显示帧**，必须叠加**缝条件**
（`sample_point spin-exit` 或 Lua 的 `SHOT-SEAM at=`），即「同一迭代取缝那一帧」；
否则 ~700px 的渲染残差会以每 4 帧一次的频率混进判定。这也解释了为何 `prev*`（重复帧）
在房扫里本来就被排除（`room_sweep.py:784`）。

## P1.0d 残差定谳（复核 r9-fc-approach-review 佐证 + 自验）

同帧（rust c=1721 / emu g=1726）三面实测：

| 面 | 结果 |
|---|---|
| `ram.hex` 的 OAM 影子（64 槽 × 4B） | **0 槽不同** |
| `ScrollX($00FD)` / `FC($002D)` | 均相同（0xF0 / 165） |
| `ppu.hex`（CHR 8K + NT 4K + palette 32） | **完全相同**；仅尾 3B regs 差（emu 侧恒 `000000` 的 dump 伪影） |
| PNG | 699px，两簇：门 x32–46（230px）+ Kraid x180–226（469px）；背景带 x0–32 / x46–180 / x226– 全 0px |
| 全屏 roll ±2/±4 | 不收敛（11792 / 10850 / 13768 …）⇒ 不是整屏位移 |

⇒ **渲染输入（OAM/CHR/NT/palette/scroll）全部相同，输出却在两个精灵簇上偏 2px，背景 0px**。
唯一自洽解释就是复核指出的**采集双相位**：`crates/game-core/src/bin/rust_screenshot.rs:544-566`
里 PNG/PPU 取 `seam_snap(key=N-1)`（上一 tick 的工作前缝），`ram.hex` 取活体（本 tick 工作后）；
背景用绘制时刻的活体 `ScrollX`，精灵用缝里的旧 OAM ⇒ 精灵相对背景偏一帧滚动量（2px/帧）。

因此**撤销 P1.0c 的「显示晚一拍」定谳**（那条把仪器相位说成了显示滞后），
并确认：这 699px **不是内容差、也不是显示滞后**，是**采集相位不一致**产生的伪残差。
下一步（决定性、约 1 天）：让单帧采集单相位（PNG 与 OAM/scroll 同一时刻），重跑
c=1721/1725/1729；归零 ⇒ 仪器 bug 结案；不归零 ⇒ 才开 Rust 精灵相位卡。

## P1.0e 单相位实验（`--capture-live`）：残差确认为采集相位产物

新增诊断开关 `--capture-live`（`crates/game-core/src/bin/rust_screenshot.rs`，启动参数，非 env 钩子）：
强制 PNG/PPU/OAM 与 `ram.hex` 都取本 tick 工作后的活体，跳过 `seam_snap`。
用同一 `.sched`（captures 120..140）重跑 rust，与 emu 按等 `(FC,ScrollX)` 配对：

| rust c | emu g | 缝相位（默认） | 活体相位（`--capture-live`） |
|---|---|---|---|
| 1720 | 1724 | **0** | 699 |
| 1721 | 1726 | 699 | **0** |
| 1722 | 1727 | **0** | 699 |
| 1725 | 1731 | 718 | **0** |
| 1729 | 1736 | 697 | 24818（载房窗，活体不能匹配 emu 的重复帧） |

**互补**：缝相位消掉 1720/1722，活体相位消掉 1721/1725；两个相位对同一「游戏时间」给出
两张不同图像，其中恰有一张与 emu 一致 ⇒

- **699px 残差 100% 是采集相位产物，不是 Rust 精灵逻辑差**（P1.0c/P1.0d 都只对了一半：
  P1.0c 说「显示滞后」不准确，P1.0d 归因到「双相位」则被这张互补表坐实）；
- 但**「哪个相位对」不是常量**：不能简单改成活体（1720/1722 会变差，1729 更崩），
  也不能保持缝（1721/1725 会差）。需要一条机械的相位判定规则；
- 候选规则（待验证）：按 emu 该帧是否为「重复帧」（`FC(g)==FC(g-1)`，即 lag 帧显示上一迭代）
  决定取活体还是缝。实测 g=1726 的 `FC=165` 与 g=1725 的 `164` 不同（非重复帧）却要活体，
  而 g=1724 的 `FC=164` 与 g=1723 的 `163` 不同（也非重复帧）却要缝 ⇒ **该规则被这一对反例否掉**，
  正确判据未定谳。

⇒ 结论：判据面（房扫 ok/mismatch）仍按缝相位不变；FC 对齐只做诊断列（与复核建议一致）。
下一步两条并行：(a) 用 Kraid31 的 18 帧逐帧表反解「哪个相位对」的规律（纯数据，不需新跑）；
(b) 不追这条线时，把它登记为 `capture-phase-pairing` 已知项，回到三区 sprite 失配族。

## P1.0f 相位规律定谳（正是「重复帧」判据）

把 seam 与 live 两个相位分别与 emu 的配对帧 g 比，并与 emu 自身的重复帧结构对照：

| rust c | emu g | seam diff | live diff | **emu(g−1)==emu(g)** |
|---|---|---|---|---|
| 1720 | 1724 | **0** | 699 | 11374（不等） |
| 1721 | 1726 | 699 | **0** | **0（相等）** |
| 1722 | 1727 | **0** | 699 | 10692 |
| 1723 | 1728 | **0** | 699 | 11378 |
| 1724 | 1729 | **0** | 699 | 11384 |
| 1725 | 1731 | 718 | **0** | **0（相等）** |
| 1726 | 1732 | **0** | 697 | 10714 |
| 1727 | 1733 | **0** | 697 | 11403 |
| 1728 | 1734 | **0** | 697 | 11652 |
| 1729 | 1736 | 697 | 24818 | **0（相等）** |
| 1730 | 1737 | 0 | 0 | 25595 |

**规律成立**：`emu(g−1)==emu(g)`（g 是重复帧对里的**第二帧**，即该迭代超预算、显示面沿用
上一帧）⇒ 取 **live** 相位；否则取 **seam** 相位。9/10 帧符合（1720–1728、1730）。

唯一例外 c=1729：它是重复帧（应按规律取 live），但落在**载房窗**（`room_load_phase==1`），
此时 live 会画出「活体已推进、显示未推进」的错帧（24818px），必须保留载房窗的
「复用最近缝」分支。⇒ 规则三分支：

1. 载房窗（`room_load_phase==1`）⇒ 复用最近缝（既有行为，不动）；
2. 否则若该 emu 帧是重复帧对第二帧 ⇒ **live** 相位；
3. 其余 ⇒ **seam** 相位（既有默认）。

注：重跑相位判定需要 emu 侧 `(FC,ScrollX)` 相邻比较，故这条规则属于**判定工具**（房扫/`fc_align`）
而非引擎；引擎侧只需同时可供两相位产物（现有 `--capture-live` 已够），不需要引入 cycle 模型。

## P1.0g 复核三条收口 + `rl` 顶层键（`8ab49d31`、本 commit）

- 复核 verdict「需改后可用（仅诊断面）」；三条必改全落：tag 只认 `f{c}`（禁止把
  `pre`/`prev*` 当判定点，实测曾让 `ok` 全部来自参考帧）、三目录一次索引 + `zp` 长度断言、
  失败可见（`errors` 计数 + 逐条打印 + rc=1）。Kraid31 因此由「13 ok」修正为
  `frames=21 ok=11 nonzero=1 errors=9`。
- `--state-out` 新增顶层键 `rl`（`mem.room_load_phase`）：`w` 仍与 emu 腿同形，`rl` 专供
  `fc_align` 判「载房窗强制 seam」，不再依赖人工 `--room-load`。
- **out-of-sample 验证未完成**：Tourian 10 只有 2 个采点（c=1660/1720），配对帧 g=1661 未被
  采集 ⇒ `missing_png`。结论：出样本验证要求**采点覆盖 `[c, c+lag_max]`**（本房 lag 可到 5–13），
  即判定采点必须成段（如 60..140 连续）而不是稀疏两点。这是 `fc_align` 跑批前的硬前置。
