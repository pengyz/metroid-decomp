# L4 自动对拍系统补全 · 设计规格书

- 日期：2026-10-26
- 状态：待复核（设计已获对话批准，本 spec 待用户书面复核后→ writing-plans）
- 范围：脚本侧补全（游戏逻辑缺口 idle 冻结/D1EE/FIRE=B 为独立移植任务，不在本 spec）
- 纪律依据：AGENTS.md（单 DSL 解析器 §2/工具优先 §D5/门禁 §5/ratchet/白名单唯一例外 §5）；
  对抗复核结论：`docs/findings/l4-autodiff-plan-adversarial-review.md`（verdict=需修订，已据 H1-H4 修订）

---

## 1. 目标与成功判据

**目标**：把自动对拍从「真实可用的核心引擎」补成「完整自动对拍系统」——一键跑全对拍用例矩阵、
诚实出 match/diff/blocked、锚定防假绿、门禁可裁、ratchet 只升不降。

**成功判据**（全部满足才算达成）：
1. `python3 tools/l4_diff_run.py run-all` 遍历 `cases/*.txt` 全部对拍用例，出用例×采集点矩阵。
2. 每采集点裁 `match | diff | blocked`（三态，**blocked 计入分母，永不计 pass**）。
3. 对拍用例三要素全支持：① 输入脚本 ② 待加载存档（snapshot）③ 对拍时长。
4. 报告锚定 `scenario_sha256` + ROM sha + 产物 sha，复用旧产物前校验 hash 不符拒跑。
5. `gates/cli.py l4` 一键跑（visual/trace/diff 口径对齐），ratchet 逐采集点只升不降。
6. 假对拍（模拟器×模拟器）脚本清除，单一入口 `tools/l4_diff_run.py`，单 DSL 解析器、单输入展开源。

**"完整"口径**：对拍用例三要素之②=待加载存档。缺 snapshot 只能测"从 boot 一幕"，**不得称完整**。
故 snapshot（阶段 D）在本交付线内。

---

## 2. 载重事实（复核已核实，设计基础）

| 事实 | 证据 | 影响 |
|---|---|---|
| `FrameMemory::restore_mss(ram, wram, regs)` 已在 | `crates/game-core/src/frame/memory.rs:142` | Rust 恢复态入口已立，D 非绿地 |
| bundle 全链已实战 | `docs/findings/bundle-first-arbitrary-start.md`（2026-10-16）：MSS bundle_load + 三直映 JSON + 快进对齐 **102 帧逐字节忠实** + 残差显式 | D 真实剩余=L4 消费面+对齐验收，**小** |
| emu 采集/装载已在 | `tools/scenario_capture.lua:475-524`：`bundle_capture`→`.mss`+`.json`（ram/wram/regs/frame）；`bundle_load`→`emu.loadSavestate` | snapshot 混合路线的基础 |
| L3 已有双 sha 锚模式 | `tools/diff_traces.py:69-76`（`oracle_sha256`/`scenario_sha256`） | L4 复用此模式 |
| l4_* 全 untracked | `git status`：`?? tools/l4_*.py` 等 | 删除=永久销毁，**先存档 commit** |
| `gap-load-snapshot.md`"4 步待定义"已陈旧 | 被 bundle-first 证伪 | 须原地【更正】 |

---

## 3. H1-H4 修正摘要（相对初版方案）

| 硬伤 | 初版错误 | 修正 |
|---|---|---|
| **H1** | snapshot="待定义 research"、"中立 JSON 主格式"、可推迟 | snapshot 全链已立非绿地；**否决中立 JSON 主格式**（第二状态语义=漂移，三直映对 MSS 有损）；走**混合路线 MSS(emu 真源)+三直映 JSON(Rust restore_mss)+快进法作对齐验证器**；"加载成功≠对齐成功"，须逐字节对齐验收；D 前置 |
| **H2** | 只废 parser、build_schedule 与 addRange 双实现、snapshot/endFrame 语义未定就并入 | **输入展开单源**（Python 展开一次→Rust/Lua 同消费产物）；先落 `snapshot`≡`bundle_load` 语义合一 + `endFrame` 语义定死，再并入 parse_scenario |
| **H3** | 清理清单带"等"、未管 untracked | **先存档 commit**；清理**闭合清单**（逐文件 grep 活死）；`r7-porter-review.md:12666` 假"PASS 5/5" 原地【更正】 |
| **H4** | 报告无锚、`--skip-*` 静默复用 | l4report/matrix 锚三 sha；复用校验调度 hash 不符拒跑；重锚定混合金面 |

---

## 4. 分阶段设计（A0→A→D→B→C）

### A0 · 现状存档 commit（前置，H3）
- **做什么**：把当前 untracked 全量（`cases/`、`crates/game-core/src/bin/`、`tools/l4_*.py`、
  `tools/rust_screenshot.*`）以「现状存档」commit 入库（过 `gates/cli.py check`）。
- **为什么**：删除=永久销毁 untracked；须先留可回溯基线。
- **验收**：存档 commit 过门禁；`git ls-files` 可见全部待整理件。

### A · 语义决议 + 输入单源 + 用例源统一 + 锚定 + 安全清理（H2 H3 H4）
1. **语义决议**（先于并入 parse_scenario）：
   - `snapshot <path>` ≡ 现 `bundle_load` 同义 → 统一为 `snapshot`（对外 DSL 名），
     内部映射到 bundle 装载语义；消除同义双名。
   - `endFrame <frame>` 语义定死 = **终端采集点**（=一个 `capture <frame> end`），
     消除"隐式终点 vs 独立对拍点"两说。
2. **输入展开单源**：`scenario_run.py` 展开 press/hold/press_len/对向键 → 逐帧 joy 调度
   **一次**；Rust bin 与生成的 Lua **同消费该展开产物**（删 `l4_diff_run.py::build_schedule` 与
   `scenario_capture.lua::addRange` 两处重复展开，杜绝两侧输入漂移=假对拍）。
3. **用例源统一**：权威对拍用例 = `cases/*.txt`（三要素格式）；工具消费 `cases/`。
4. **重锚定混合金面**：`oracle/screens/` 13 个 case-001 金面混两套同名调度（1200/1800）→
   按 scenario_sha256 重锚，消除"旧考题产物静默对拍出绿"。
5. **锚定**：`*.l4report.json`/`matrix.json` 落 `scenario_sha256` + ROM sha + 产物 sha
   （复用 `diff_traces.py` 模式）；`--skip-emu/--skip-rust` 复用前校验调度 hash，不符拒跑。
6. **安全清理**（先存档后删，闭合清单——无"等"）：
   - **活**：`tools/l4_diff_run.py`、`tools/scenario_run.py`。
   - **死**（删）：`tools/l4_auto_diff.py`（emu×emu 假对拍）、`tools/l4_rust_vs_emulator.py`
     （旧 CLI `<scenario> --timeout` vs 现行 `--schedule`，必坏）、`tools/rust_screenshot_real.py`、
     `tools/rust_screenshot.py`（同断径）、`tools/rust_screenshot.rs`（不在 Cargo workspace members）、
     `tools/l4_unified_diff.py`（golden×shots 换皮假对拍）、`tools/l4_diff_layer.py`、
     `tools/l4_conclusion_layer.py`、`tools/l4_rust_runner.py`（stub）。
   - **先并语义再删**：`tools/l4_unified_format.py`（死，但 `ScenarioDSL` 是语义载体——
     其 snapshot/endFrame 语义先并入 parse_scenario 再删）。
   - 每个删除附 grep 证据（无引用）；`docs/rounds/r7-porter-review.md:12666` 原地【更正】
     假对拍"PASS 5/5"（证伪更正文化）。
- **验收**：grep 零命中「第二 DSL 解析器」+「第二输入展开」；`run` 读 `cases/`；三 sha 锚落库；
  清理附 grep 证据 + 先存档后删。

### D · snapshot L4 消费面 + 对齐验收（H1，前置，小）
- **格式**（混合，非中立）：`bundle_capture` 已产 `<out>.mss`（emu 真源）+ `<out>.json` 三直映
  `{mss_bytes, ram, wram, regs{a,x,y,sp,pc,ps}, frame}`。**不新设中立 JSON 游戏状态格式**。
- **Rust 消费面**：`rust_screenshot` bin 增 `--bundle <prefix>`（或调度表含 `snapshot <prefix>`）→
  `FrameMemory::restore_mss(ram, wram, regs)` 灌入 FrameEngine.mem。
  **渲染态处置（决策规则，非开放）**：先以 `bundle-first-arbitrary-start.md` 实证口径判定
  restore_mss 的渲染态来源——(a) 若 render（chr/nametable/palette/scroll）由 RAM/WRAM+游戏逻辑
  在后续帧重建（bundle-first 的 102 帧忠实即证），则**不扩快照**；(b) 若需独立 PPU 态才不掉帧，
  才扩三直映 JSON 至含 chr/nametable/palette/ppuctrl/scroll。默认走 (a)，(b) 仅在 (a) 对齐残差
  非 0 且定位为渲染态缺失时启用。
- **emu 消费面**：沿用 `bundle_load`（`.mss` + `emu.loadSavestate`）。
- **对齐验收**（防"假对拍"关键）：同一 bundle→两边载入→**前 N 帧逐字节对齐**（Rust restore 后
  连续帧 ↔ emu 快进帧）；残差=0 或**显式登记**（savestate 重放经典残差类）。"加载成功≠对齐成功"。
- **验收**：bundle 对齐 diff 过 + 残差显式；`gap-load-snapshot.md` 原地【更正】陈旧"待定义"。

### B · 批量对拍矩阵（H4）
- `run-all` 遍历 `cases/*` → 用例×采集点矩阵（**三态≠pass，blocked 计入分母**）→ `matrix.json` + md。
- 每行锚三 sha；`--skip-*` 校验调度 hash 不符拒跑。
- **验收**：矩阵含 snapshot 用例；blocked 不算 pass；双 sha 锚；分母含 blocked。

### C · 门禁 + 白名单 + ratchet（全）
- `gates/cli.py` 新增 `l4` 门禁（**单独 commit**，门禁改动不与功能混提）；调 `l4_diff_run`。
- 白名单 `gates/visual_whitelist.json` = 唯一例外（不复增 `diff_visual.py` 的 `visual_gap_map`）。
- **ratchet**：锚**逐采集点 match 集合** + blocked 只减不增（**非聚合一致率**——防 9/10→9/10 换坏不触发）。
- 回归基线复用 `data/status/` + `tools/baseline_update.py`；TOLERATED 走 `l2_gap_map` 登记
  （理由+退出路线+显式计数），不新增静默容忍面。
- **验收**：`gates/cli.py l4` 一键跑；ratchet 只升；TOLERATED 登记制。

---

## 5. 假绿四件套防线（贯穿）
1. **blocked ≠ pass**：分母含 blocked（现行 `l4_diff_run` exit code 已把 blocked 计 fail，批量层保持）。
2. **ratchet 锚逐采集点 match 集合**（非聚合百分比，防局部回退掩蔽）。
3. **TOLERATED 登记制**（l2_gap_map，理由+退出路线+计数），不新增静默容忍。
4. **白名单唯一例外**（不复增 visual_gap_map）。

## 6. 额外纳入
- 用例清单接 **D7 完备性矩阵分母**（绿≠完整）；游戏内区域覆盖进分母。
- **采样点盲区**（`gotcha-sampling-point-is-blind-spot`）进用例设计准则——F5 idle 冻结正是
  连续对比才现形，用例须含连续/多采样点设计。

## 7. CI / verb 口径
- `gates/cli.py check` 现不含 visual/trace/diff（GATES 表 `gates/cli.py:1307-1324`）；
  补全后 L4 门禁纳入 `check`/`all` 口径，并在 GATES 表登记。

## 8. 风险
| 风险 | 缓解 |
|---|---|
| Lua 读/写完整 PPU 态能力边界（渲染态入快照） | 以 bundle-first 实证为准；若 render 由 RAM/WRAM 重建则不扩快照 |
| 输入展开单源改造破坏现有可复现性 | A 阶段改后对拍回归（同输入同帧 hash 不变） |
| 清理误删仍被引用件 | 逐件 grep 证据 + 先存档 commit |
| 对齐残差非 0 | 显式登记残差类，不静默容忍 |

## 9. 关联
- `docs/findings/l4-autodiff-plan-adversarial-review.md`（本 spec 依据的对抗复核）
- `docs/findings/bundle-first-arbitrary-start.md`（D 的实证基础）
- `docs/knowledge/gap-load-snapshot.md`（D 后须更正）
- `docs/knowledge/pattern-l4-real-rust-vs-emulator-diff.md`（对拍引擎做法）
- `docs/knowledge/gotcha-sampling-point-is-blind-spot.md`（用例设计准则）
- AGENTS.md §2/§5（单解析器/门禁/ratchet/白名单）
