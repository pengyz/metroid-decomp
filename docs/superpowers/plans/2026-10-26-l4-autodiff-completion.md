# L4 自动对拍系统补全 · 实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 把自动对拍从「真实核心引擎」补成「完整自动对拍系统」——一键跑全用例矩阵、诚实 match/diff/blocked、三 sha 锚定防假绿、门禁可裁、ratchet 只升不降。

**Architecture:** 复用既有 bundle 链（MSS+三直映 JSON+`restore_mss`）补 snapshot L4 消费面；输入展开单源（Python 展开一次→Rust/Lua 同消费）；权威对拍用例源=`cases/`；报告锚三 sha；接入 gates l4 门禁 + 白名单 + 逐采集点 ratchet。游戏逻辑缺口（idle 冻结/D1EE/FIRE=B）不在本计划。

**Tech Stack:** Rust（game-core FrameEngine/render_frame/rust_screenshot bin）、Python 3（l4_diff_run/scenario_run，PIL+numpy）、Mesen2（oracle）、gates/cli.py。

**Spec:** docs/spec/l4-autodiff-completion.md

## Global Constraints
- DSL 语义只在 `tools/scenario_run.py::parse_scenario` 解析（无第二解析器、无第二输入展开源）。
- 对拍=Rust 真渲染帧 vs 模拟器，逐像素零容差；blocked 永不计 pass（分母含 blocked）。
- 报告锚 `scenario_sha256` + ROM sha + 产物 sha；复用旧产物前校验调度 hash，不符拒跑。
- 白名单唯一例外=`gates/visual_whitelist.json`；TOLERATED 走 `l2_gap_map` 登记（理由+退出路线+计数）。
- ratchet 锚逐采集点 match 集合 + blocked 只减不增（非聚合一致率）。
- 门禁改动单独 commit + 基线说明；改 `reference//rom/` 需 Evidence 字段。
- 清理前先「现状存档」commit（untracked 删除=永久销毁）。

## Review Focus
- **输入展开漂移**：若 Rust 与 Lua 各自展开 press/hold，两侧输入不一致=假对拍 → 单源展开测试（Task 2）。
- **blocked 被当 pass**：批量矩阵分母若漏 blocked=假绿 → 矩阵含 blocked 且不计 pass 测试（Task 5）。
- **锚定失效静默复用**：`--skip-*` 复用旧截图无 hash 校验 → hash 不符拒跑测试（Task 3）。
- **snapshot 加载成功≠对齐**：restore 后未逐字节对齐 → 对齐验收测试（Task 4）。
- **清理误删**：删 untracked 可能毁活件 → 先存档+逐件 grep 证据（Task 1/3）。

---

### Task 1: 恢复提交能力（门禁现状解封 · A0+option-a）

**Files:**
- Modify: `gates/cli.py:74`（`KNOWLEDGE_TYPES` 加 `gap`）
- Modify: `docs/knowledge/README.md`（类型表加 `gap`）
- Create: `data/status/l4_status_quo_baseline.json`（trace/visual 现状基线登记）
- Modify: `gates/cli.py`（trace/visual 现状判 TOLERATED 的读取点——沿用 l2_gap_map/账内容忍机制）

**Interfaces:**
- Produces: `gates/cli.py check` 结果=PASS（knowledge PASS、trace/visual TOLERATED 显式）；一次可落库 commit。
- Produces: `data/status/l4_status_quo_baseline.json` schema：`{trace:{<case>:{"reason","exit","count"}}, visual:{<frame>:{"reason","exit","count"}}}`。

- [ ] **Step 1: 写失败测试**（`gates/selftest` 或新增 `gates/tests/test_knowledge_types.py`）：断言 `KNOWLEDGE_TYPES` 含 `"gap"`，且 `gap-load-snapshot.md`（type:gap）通过 `gate_knowledge_format`。
- [ ] **Step 2: 跑测试确认 FAIL**（当前 `type='gap' 不在 [...]`）。
- [ ] **Step 3: 实现** `gates/cli.py:74` `KNOWLEDGE_TYPES` 加 `"gap"`；`docs/knowledge/README.md` 类型表加 `gap` 行（收「已知缺口/deficiency」）。
- [ ] **Step 4: 跑 `python3 gates/cli.py check`**——knowledge-format 应 PASS。
- [ ] **Step 5: 登记 trace/visual 现状基线**：把当前 `trace-diff` 1 失败 + `visual-diff` 13 失败逐条登记入 `l4_status_quo_baseline.json`（reason="游戏未 L3/L4 一致（现状）"、exit="移植推进账清转 strict"、count 显式）；接 gate 读取点判 TOLERATED（账内容忍，非静默）。
- [ ] **Step 6: 跑 `check`** 确认三闸全绿/TOLERATED（显式打印）。
- [ ] **Step 7: 现状存档 commit**（untracked 全量入 git）+ 门禁改动**单独 commit**（写基线影响）。Evidence/Confidence 按需。

### Task 2: 语义决议 + 输入单源 + cases/ 权威（A）

**Files:**
- Modify: `tools/scenario_run.py`（parse_scenario 认识 `snapshot`≡`bundle_load`、`endFrame`→capture；新增**唯一**逐帧输入展开函数 `expand_schedule(plan)->{frame:joy1}`）
- Modify: `tools/l4_diff_run.py`（删 `build_schedule`，改调 `scenario_run.expand_schedule`）
- Modify: `tools/scenario_capture.lua`（删 `addRange` 展开，改读 Python 预展开的逐帧调度）
- Modify: `cases/case-001-*.txt`（统一为权威格式；采集点对齐）

**Interfaces:**
- Produces: `scenario_run.expand_schedule(plan: dict) -> dict[int,int]`（frame→joy1，press/hold/press_len/对向键消解唯一实现）。
- Produces: parse_scenario 认 `snapshot <path>`（≡bundle_load）、`endFrame <f>`（=终端 capture）。

- [ ] **Step 1: 写失败测试**（`tools/tests/test_expand_schedule.py`）：给定 plan，`expand_schedule` 产出逐帧 joy；断言 Rust 侧与 Lua 侧消费同一产物（对同一 plan 不再各自展开）。语义测试：`snapshot` 映射 bundle_load、`endFrame` 等价 capture。
- [ ] **Step 2: 跑测试 FAIL**（build_schedule/addRange 尚在；parse_scenario 不认 snapshot/endFrame）。
- [ ] **Step 3: 实现** parse_scenario 语义（snapshot≡bundle_load、endFrame→capture）+ `expand_schedule` 唯一展开；删 `l4_diff_run.build_schedule`、`scenario_capture.lua::addRange`，改同消费 `expand_schedule` 产物。
- [ ] **Step 4: 跑测试 PASS** + 跑 `l4_diff_run.py run case-001 --skip-emu` 确认同输入同帧 hash 不变（输入单源无回归）。
- [ ] **Step 5: Commit**。

### Task 3: 三 sha 锚定 + 安全清理（A · H3 H4）

**Files:**
- Modify: `tools/l4_diff_run.py`（l4report/matrix 写 `scenario_sha256`+ROM sha+产物 sha；`--skip-*` 复用校验 hash）
- Modify: `docs/rounds/r7-porter-review.md:12666`（假"PASS 5/5"原地【更正】）
- Delete: `tools/l4_auto_diff.py`、`tools/l4_rust_vs_emulator.py`、`tools/rust_screenshot_real.py`、`tools/rust_screenshot.py`、`tools/rust_screenshot.rs`、`tools/l4_unified_diff.py`、`tools/l4_diff_layer.py`、`tools/l4_conclusion_layer.py`、`tools/l4_rust_runner.py`、`tools/l4_unified_format.py`（语义已并入 Task 2 后）
- Create: `docs/findings/l4-cleanup-inventory.md`（逐件 grep 活死证据）

**Interfaces:**
- Produces: `l4report.json` 含 `{"scenario_sha256","rom_sha256","artifacts":{<file>:sha}}`；`--skip-emu/--skip-rust` 复用前校验，不符 exit≠0。

- [ ] **Step 1: 写失败测试**：l4report 含三 sha；`--skip-rust` 传入调度 hash 不符 → 拒跑（exit≠0）。
- [ ] **Step 2: 跑 FAIL**。
- [ ] **Step 3: 实现** 三 sha 锚定 + 复用 hash 校验。
- [ ] **Step 4: 清理**：逐件 `grep -rn <file> 全仓` 确认无引用 → 记 `l4-cleanup-inventory.md` → 删（先 Task 1 存档 commit 已留基线）。
- [ ] **Step 5: r7-porter-review.md:12666 原地【更正】**（证伪更正文化）。
- [ ] **Step 6: 跑测试 PASS + grep 零命中第二解析器/第二展开/已删件引用**。
- [ ] **Step 7: Commit**。

### Task 4: snapshot L4 消费面 + 对齐验收（D · H1）

**Files:**
- Modify: `crates/game-core/src/bin/rust_screenshot.rs`（增 `--bundle <prefix>`/调度表 `snapshot` → `restore_mss`）
- Modify: `crates/game-core/src/frame/memory.rs`（若需：`restore_mss` 渲染面——按 spec §4-D 决策规则，默认 render 由 RAM/WRAM 重建）
- Modify: `tools/l4_diff_run.py`（snapshot 用例对齐验收）
- Modify: `docs/knowledge/gap-load-snapshot.md`（原地更正陈旧"待定义"）

**Interfaces:**
- Produces: `rust_screenshot --schedule <s> --bundle <prefix>`（从快照态起跑）。
- Produces: 对齐验收=restore 后前 N 帧 Rust↔emu 快进逐字节 diff，残差=0 或显式登记。

- [ ] **Step 1: 写失败测试**：构造 bundle（三直映 JSON）→ `rust_screenshot` restore 后首帧 == 期望帧；对齐验收函数产出逐帧 diff（残差显式）。
- [ ] **Step 2: 跑 FAIL**（bin 不收 bundle）。
- [ ] **Step 3: 实现** `--bundle`+`restore_mss` 灌入 + 对齐验收（前 N 帧逐字节比对，残差登记）。
- [ ] **Step 4: 用 `bundle-first` 实证 bundle 跑对齐 diff**，确认「加载成功≠对齐成功」被验收捕获。
- [ ] **Step 5: gap-load-snapshot.md 原地更正**（陈旧"待定义"→引用 bundle-first 实证）。
- [ ] **Step 6: 跑测试 PASS + `cargo build -p game-core --bin rust_screenshot`**。
- [ ] **Step 7: Commit**。

### Task 5: 批量矩阵（B · H4）

**Files:**
- Modify: `tools/l4_diff_run.py`（`run-all` action）
- Create: `shots_rust/matrix.json` + `shots_rust/matrix.md`（产物）

**Interfaces:**
- Produces: `run-all` 遍历 `cases/*.txt` → 矩阵 `{case:{capture:match|diff|blocked}}`，分母含 blocked。

- [ ] **Step 1: 写失败测试**：`run-all` 产矩阵；blocked 计入分母且不计 pass；三 sha 锚落矩阵。
- [ ] **Step 2: 跑 FAIL**。
- [ ] **Step 3: 实现** `run-all` + 矩阵输出（三态）。
- [ ] **Step 4: 跑 `run-all`**（含 snapshot 用例）确认矩阵正确。
- [ ] **Step 5: Commit**。

### Task 6: 门禁 + 白名单 + ratchet（C）

**Files:**
- Modify: `gates/cli.py`（新增 `l4` 门禁，GATES 表登记——**单独 commit**）
- Modify: `gates/visual_whitelist.json`（唯一例外）
- Modify: `data/status/`（ratchet 基线：逐采集点 match 集合）

**Interfaces:**
- Produces: `gates/cli.py l4` 一键跑 l4_diff_run；ratchet 逐采集点只升不降。

- [ ] **Step 1: 写失败测试**：`gate_l4` 调 l4_diff_run 出裁定；ratchet 逐采集点 match 集合只升（换坏一采集点触发）。
- [ ] **Step 2: 跑 FAIL**。
- [ ] **Step 3: 实现** `gate_l4` + 白名单接入 + 逐采集点 ratchet（非聚合率）。
- [ ] **Step 4: 跑 `gates/cli.py l4` 实跑贴输出** + ratchet 反向测试（降一采集点应 FAIL）。
- [ ] **Step 5: 门禁改动单独 commit + 基线说明**。

---

## Self-Review 记录
- **Spec 覆盖**：A0→Task1；A→Task2/3；D→Task4；B→Task5；C→Task6。option(a) 门禁解封→Task1。覆盖完整。
- **类型一致**：`expand_schedule(dict)->dict[int,int]`（Task2 产，Task2/5 消）；三 sha 键名（Task3 产，Task5 消）；`restore_mss(ram,wram,regs)`（已有，Task4 消）。
- **比例**：本计划=决策+测试+签名（非 transcript）。
