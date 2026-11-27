# TAS 对拍基建落地方案（B1/B2/B4/B8）— 设计与评审材料

> Task #149（tsk_01M8E126RAVFDXY1D784EW）。只读调研 + 方案设计，**crates/tools/oracle 零代码改动**；
> 本文档是唯一交付物。评审通过后由 PMO 拆实现卡（写面 tools/ + oracle/）。
> 依据版本：8ca720e5（#143 门链修复）+ 0bede30c（tas-round7 更正——首分歧稳健口径 ≥f1574）；
> 树内另有未跟踪 supermetroiddecomp-study.md/undeclared-residual-plan.md（他人卡面，与本卡无关）。
> 上游研究：docs/findings/supermetroiddecomp-study.md（#147，B 清单行号 B1:262 / B2:274 / B4:297 / B8:339）；
> NMI 证据：docs/findings/nmi-frame-evidence.md（#105）。

## 0. 结论先行

- 现状 TAS 对拍以**视频帧号为唯一判等轴**，且 emu 腿采样缺帧**无记账**——#143 验收实测：真实首分歧
  f1574（$0406 1B）被静默右移报成 f1575（5B），两跑复现（/tmp/r143/post、/tmp/r143/post2 各 3962/4000 行，
  缺 38 帧，f1572–f1574 缺）。对影独立复跑（/tmp/v143，--allow-rust-missing）证实 f1574 1B。
  这是当前口径的最大风险，**建议第一批修（见 §3 B4 附带项）**。
- B1/B2（更新轴 + 读键边界采集）是治本方向，但动判等语义；建议以 `--axis` 兼容开关并存落地，
  不破坏既有 13 段 / 120540 帧产物。
- B8 走「显式记账」而非「消除」：NES lag = 逻辑帧真丢（#105），照搬 SuperMetroidDecomp 的消除会引入
  原版没有的行为（Stage A 禁止）。
- 推荐路线一（最小改造，2–3 人日，3 文件）；路线二（全量含 B3/B5/B6，5–8 人日）列后续卡。

## 1. 现状对标（每条带 文件:行 / 实测命令）

| 维度 | 现状 | 锚点（文件:行） | 缺口对应 |
|---|---|---|---|
| 判等单位 | 视频帧号对齐；交集判等 `common = sorted(set(emu) & set(rust))` | tools/tas_replay_diff.py:203（compare :199–235） | B1 |
| 按键相位 | 已知 1 帧滞后并显式对齐：emu[f]（应用前）vs rust[f-1]（应用后），raw 比只留溯源 | tools/tas_replay_diff.py:211–215；#105 证据 | B1（已半解决，仅按键面） |
| 判等面 | 单段 `$0400-$04FF` sha256；范围硬门禁只允许 `$0100-$07FF` | tools/tas_replay_diff.py:190–191；§6 复跑命令实测 | B4/B5 |
| 缺帧记账 | **rust 腿**缺帧显式记 `coverage.rust_missing` 并在报告列出；**emu 腿**缺帧无任何记账，交集判等静默跳过 | tools/tas_replay_diff.py:194–196（rust 侧）；:203（交集） | B4 |
| 采集契约 | emu 腿 `--format mesen2` + watch 面（`watch_range lo-hi tag`、`watch 0014 Joy1Status`）；帧号在 `inputPolled` 回调递增（第 N 次 poll=帧 N）；产物 traces/*.frames.jsonl/.captures.jsonl/_fNNNNN_<tag>.ram.hex（每采集点全量 2 KiB hex） | tools/tas_replay_diff.py:58（build_scenarios）、:153/:180；tools/scenario_run.py:18–40 | B2/B4 |
| 环境指纹 | 报告记 git_head + rust 二进制 sha（同 HEAD 不同工作树可分辨） | tools/tas_replay_diff.py:812–836（_env_fingerprint 全函数，git_head :823） | （已达标，保留） |
| 首分歧汇总 | 逐段 first_divergence 取全局最小帧，其后段标 post_divergence | tools/tas_replay_diff.py:845–852 | （已达标；但依赖交集判等 ⇒ 继承 B4 缺帧缺口） |
| 自测 | 差异检测自测（合成面注入 1 字节差必须报出） | tools/tas_replay_diff.py:906（cmd_selftest） | B4 迁移可扩展 |
| lag 建模 | Rust 侧显式模拟掉帧：`dropped=(BASE+work_cycles)/FRAME_CYCLES`；boot 多处 `iter_bogged(2)`（GFX 装载重活×2）；ledger 实测 3960 帧仅 25 个 lag、末个 f1756 | crates/game-core/src/frame/cycles.rs:5–6；crates/game-core/src/frame/boot.rs；data/status/movie_ledger.json:49 | B8 |
| 影片事实存放 | ledger 记来源/帧数/synced/分歧（如 usable_until=3889）；tas-round*.md 另述通过数 | data/status/movie_ledger.json:1–53 | B7（不在本卡范围，见 §6） |
| 采样相位 | 关屏帧「state 全同、像素相位差」（f4018 PrepareGameOver，$FE 两侧 0x06，emu 黑屏 vs rust 上一屏）已单列待定谳 | data/status/movie_ledger.json:49 | B1 动机实例 |
| 复跑命令 | `python3 tools/tas_replay_diff.py replay --inputlog movies/lordtom-metroid-100p.inputlog --frames 4000 --segments 4000 --range 0400-04FF --out <dir>`（#143 验收实测两腿 79.0s / emu 采样 3962 行） | docs/spec/f1551-fix-plan.md §6；/tmp/r143/post | — |

#147（supermetroiddecomp-study.md）的 B1/B2/B4/B8 结论与本表一致，本文不重复其证据链，只落到实现面。

## 2. B1/B2/B4/B8 逐条：现状 → 目标 → 迁移步骤 → 回滚点 → 依赖

### 2.1 B4 紧凑轨迹 + 缺帧记账（第一批，治口径漂移）

- **现状**：全量 2 KiB hex/采集点（上万份时体积与 I/O 大）；判等面单段 sha256；emu 缺帧无记账
  ⇒ first_divergence 依赖采样覆盖（f1574→f1575 实例）。
- **目标**：①`layout` 显式契约（初始全量 + 逐帧 XOR 差分 + 固定布局哈希；改面必须改布局哈希并重生成轨迹）；
  ②coverage.json 增 `emu_missing` 字段并在报告显式列出（对齐 rust_missing 既有行为 :194–196）。
- **迁移步骤**：a. tools 定义 layout json（默认=现判等面 $0400–$04FF，可扩展）；b. 差分编码/解码器 +
  往返自测（扩展 cmd_selftest :906）；c. tas_replay_diff.py 读新格式、兼容旧 hex；d. coverage/报告增
  emu_missing 列。
- **回滚点**：旧 hex 读取路径保留；emu_missing 只增字段不改判定 ⇒ 任一步可独立回退。
- **依赖**：改 tools（2 文件）；oracle Lua **dump 格式不变**（压缩在工具侧后处理）；13 段产物不重生成。
- **实现状态（#150 已交付，commit 待落）**：②已全量落地——`compare()`/`compare_window()` 在
  **两侧并集窗口**内显式列 `emu_missing_frames`/`rust_missing_frames` 清单 + `coverage_note`
  （tas_replay_diff.py:249/:874，报告行「emu 缺帧（不可比，判等已跳过）」）；write_coverage 表头增
  emu 缺帧/rust 缺帧/无输入跳过 3 列；run_segments entry 带同字段。①紧凑轨迹 `encode_compact_trace`/
  `decode_compact_trace`（initial 全量+逐帧 XOR+固定布局 sha256+content sha256 自校验，篡改必被
  content sha 抓出）+ `write/read_compact_trace`（.gz）+ `cmd_compact` 子命令已入库；
  往返自测已扩进 cmd_selftest（60 帧合成面）与 tools/tests/test_tas_replay_diff_gates.py（13 用例）。
  真实旧产物对照：/tmp/r143/post 4000 帧 trace encode→decode 往返 content_sha 一致；
  compare_window 在该产物上点名 emu_missing=[1574,1860,1903,1904,1928,…] 共 11 帧——含 f1574，
  即 #143/#148 首分歧右移的帧，本记账直接覆盖该盲区。

### 2.2 B2 读键边界采集（emu 腿）

- **现状**：每视频帧 poll 一次采样（scenario_run.py:18–40）≠ 实际消费；NMI `$1A≠0` 整段跳过（#105）；
  Mesen2 已有 `addMemoryCallback(...,"read",0x4016/0x4017)` 能力（#105 已用）。
- **目标**：在 $4016/$4017 读边界记「pc + 周期 + 值」，帧末 flush 成**更新清单**（谁消费、消费序）。
- **迁移步骤**：a. oracle Lua harness 增读回调打点（独立开关，默认关）；b. 产出 `updates.jsonl`
  （frame, pc, cycle, value 序列）；c. scenario_run 侧透传不解析。
- **回滚点**：Lua 开关默认关 ⇒ 不采则零影响；updates.jsonl 是新增文件。
- **依赖**：改 oracle Lua（新增打点，不改既有 watch 面）；dump 格式新增一种（jsonl），旧产物不动。

### 2.3 B1 更新轴判等（依赖 2.2 的更新清单）

- **现状**：帧号对齐 + 仅按键面 1 帧滞后对齐；跳过读键的帧会被当普通帧比较（采样相位实例：f4018）。
- **目标**：比较轴改为「更新号」，视频帧号降级为 sourceFrame 标识；无输入帧标 `no-input`；
  报告并列「更新号 + 源帧号」。
- **迁移步骤**：a. emu 腿 updates.jsonl（2.2 产物）→ 源帧→更新映射；b. rust 腿输出对应更新映射
  （NMI/逻辑帧，lag 帧联动 2.4）；c. tas_replay_diff.py 加 `--axis updates` 兼容模式，**默认帧轴不变**；
  d. 报告增 axis 字段与更新号列。
- **回滚点**：`--axis` 缺省=帧轴，13 段产物口径完全不变；更新轴单独出报告文件。
- **依赖**：改 tools（1 文件）；oracle 无新改动（用 2.2 产物）；不破坏既有产物。
- **实现状态（#150 兼容模式已交付，完整版待 2.2）**：`--axis {frames,updates}` 已进 argparse
  （缺省 frames，13 段产物口径不变）；updates 轴最小语义=**inputlog 输入字变更轴**
  （`input_change_frames` 逐行比较 p1/p2/cmd 相邻变化，全零轨迹 fail-loud）——是 2.2 更新清单
  落地前的工具侧近似，报告/coverage 均已标注该口径；axis_skipped_no_input_frames 显式入账
  （跳过的无输入变化帧不进判等也不静默）；报告强制 axis 字段 + 判等轴行；空更新帧清单/空可比帧
  fail-loud。120540 帧产物未重生成。

### 2.4 B8 lag 显式记账（不改行为，只加标记）

- **现状**：Rust 已显式模拟丢帧（cycles.rs:5–6、boot iter_bogged）；emu 侧 ledger 有 FrameCount 增量
  体检先例（3960 帧 25 个 lag，movie_ledger.json:49）；比较轴无 lag 标记。
- **目标**：lag 帧在轨迹/报告标 `no-input/no-update`，保留可审计记录，**不消除、不从时间轴删除**
  （NES lag=逻辑帧真丢；消除=引入原版没有的行为，Stage A 禁止，#147 B8:339 结论照抄）。
- **迁移步骤**：a. rust 腿逐帧输出 lag 标志（内部量透传）；b. emu 腿由 FrameCount 增量推 lag
  （ledger 已示范算法）；c. compare 对 lag 帧按更新轴跳过（与 2.3 联动）或单独列出不判等。
- **回滚点**：标志为独立字段，不读不影响任何现有判定。
- **依赖**：改 tools（1 文件）+ rust 腿 trace 输出（crates 零改动：`--trace` 已有每帧输出面，lag 标志
  可由现有 work_cycles 推导，实现卡再定——若必须动 crates 则升级为 PMO 决策项，见 §5 拍板清单）。

## 3. 路线对比与推荐

| 项 | 路线一（最小改造，推荐） | 路线二（全量改造） |
|---|---|---|
| 范围 | §2 全部 4 条，按 2.1→2.2→2.3→2.4 顺序 | 路线一 + B3 manifest + B5 catalog 集中 + B6 分歧上下文 |
| 文件 | tas_replay_diff.py、scenario_run.py、oracle Lua（3 处） | 另加 catalog/manifest 模块（5–6 处） |
| 估行数/人日 | 300–400 行 / 2–3 人日 | 800–1000 行 / 5–8 人日 |
| 判等语义 | 帧轴默认不变，更新轴并存 | 更新轴切为主轴，双轨过渡期 |
| 13 段/120540 帧产物 | 不重生成（新旧格式并存） | 需重生成或长期双轨 |
| L4 172 格基线 | 无影响 | 无影响（同为逻辑层） |
| check | PASS24/0/3/1 不变（tools/oracle 不在 rust-tests 基线） | 同左，但 docs 面新增多 |
| 主要风险 | 更新轴与帧轴并存的口径混乱 | 切换期两套报告并存、口径漂移放大 |

**推荐路线一**。理由：①当前最疼的缺口（emu 缺帧记账、判等面显式契约）都在 2.1，收益立得且零语义风险；
②B1/B2 治本但不急于切主轴——`--axis` 并存让 13 段基线继续可复跑、可对照；
③B3/B5/B6 价值真实但与 resultDefinition 圈定范围不符，列后续卡，避免单卡膨胀。

## 4. 成本与风险量化（≥3 风险 + 缓解）

1. **Lua 打点引入采样开销** → 打点只累积内存列表、帧末一次 flush；实现卡验收含 4000 帧耗时对照
   （基线 79.0s，容忍 ±10%，超了单独定谳）。
2. **双轴并存的口径混乱** → 报告顶层强制 `axis` 字段；更新轴报告独立文件名；tas-round 文档写明当次用的轴。
3. **XOR 差分编码 bug 导致假绿** → 往返自测进 cmd_selftest（合成面注入差必须复现）；新格式先跑
   4000 帧短程与旧格式全量逐字节对照后再切长程。
4. **emu 缺帧根因未除（本方案只记账）** → 记账先堵口径漂移；采样器丢帧根因（Mesen 腿 38/4000 稳定缺）
   单列待查项，不混入本卡验收。
5. **lag 标志若需动 crates**（2.4 步骤 a 的兜底）→ 升级为 PMO 决策项（见 §5-6），默认方案用 `--trace`
   既有输出推导，crates 零改动。

## 5. 评审拍板清单（对影复核 + PMO 决策）

1. 路线选择：路线一（推荐）/ 路线二 / 只做 §2.1（emu_missing+layout）。
2. B2 打点位置：$4016/$4017 读回调（#105 能力已证，推荐）vs ReadJoyPads 入口 pc-hook（07:C0FC）。
3. layout 默认面：维持 $0400–$04FF 单段（推荐）vs 直接上多段 layout 列表。
4. emu_missing 记账位置：coverage.json 顶层字段 + 报告缺帧清单（推荐，对影 #143 seq13 建议）。
5. lag 标志实现面：允许 tools 侧推导（crates 零改动）为准；如需 crates 输出 lag 字段，是否批准动
   crates（写面扩张需 PMO 授权）。
6. B3/B5/B6 是否另开后续卡（建议：B6 分歧上下文下一优先，对定界卡收益直接）。

## 6. 范围与边界

- 本卡唯一写面：本文档。crates/tools/oracle 零改动；既有 13 段产物、L4 基线、门禁基线不动。
- B7（影片事实 ticket 化）不在 resultDefinition，本方案不设计；现状 ledger 已承载大部分事实
  （data/status/movie_ledger.json）。
- 验收命令：`python3 gates/cli.py check`（预期 PASS=24 FAIL=0 SKIP=3 TOLERATED=1，HEAD 8ca720e5）。
