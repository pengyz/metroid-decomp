# 三方对拍落地：帧映射规约 + 分叉分类 + 覆盖台账 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 把「A 路线腿(quickerNES) / B 仪器腿(Mesen2) / C 对照腿(Rust)」的逐帧对拍从手工脚本升级成有机械口径、可复跑、带棘轮的三件套工具链。

**Architecture:** 帧映射不再靠"打分最优"猜，而是用状态面（`$0300` `SamusStatus` + `$005A` `RoomNumber`）的全帧一致数反解每腿偏移，写进 `data/status/leg_offsets.json`；`$0014` 输入锁存面单列 `latch_offsets`，只作 ±1 自校验（2026-11-18 实测恒等于状态面 +1），不单独产出整数帧偏移；在此口径下把失配聚合成 episode 并二分类（相位抖动 / 值差），加载窗（`$005A != $FF`）单列不进分母；判据口径写进 `docs/spec/oracle-verification-chain.md` §3.1 作为唯一真相源，覆盖台账加棘轮门防止回退。

**Tech Stack:** Python 3（tools/ 侧，无第三方依赖）、Rust（`crates/game-core` 只读消费，不改）、Mesen2 Lua（已有 `tools/scenario_run.py`）、BizHawk/Mono Lua（已有 `tools/movie_ref/run_bizhawk_trace.sh`）、项目门禁 `gates/cli.py`。

**Spec:** `docs/spec/oracle-verification-chain.md`（三腿分工与五条仲裁规则）+ `docs/findings/movie-bizhawk-6725m.md` §10/§11（首轮数字与已知口误）

## Global Constraints

- asm（`refs/m1disasm`）是唯一真相源；任何机制结论必须给 `bank:addr` 锚。
- 字节地址一律用 `crates/game-core/src/ram_map.rs` 的具名常量，禁止在工具里新造魔法数（`$0014`=`JOY1_STATUS`，计划/任务称 `JOY1_LATCH`；`$005A`=`ROOM_NUMBER`；`$0300`=`SAMUS_STATUS`）。
- 判据不许调松：`--fail-on-mismatch` 之类的判定开关不得为过测而放宽；本计划的工具只做**映射与分类**，不裁定"谁对"。
- 分歧点之后的帧不计入缺陷（级联不是缺陷）。
- 跨腿帧号在未报相位的情况下不得直接比较。
- 临时产物留 `analysis/traces/`（gitignored），临时用例不得留 `cases/`。
- 修复完成必须 commit 并 push；commit message 禁裸反引号，触账本须带 `Evidence:` 与 `Confidence:`。
- 每个 task 结束前跑 `python3 gates/cli.py check`，任一 FAIL 不进下一个 task。

## 执行契约（用户 2026-11-18 裁定，优先级高于本文件其余流程描述）

每个 task 完成后**不得直接进入下一个 task**：必须先派一名未参与实施的子代理做**对抗性复核**（证伪立场：假设该 task 的结论是错的，独立重导证据、独立复跑命令），复核 verdict 为 pass 才算该 task 完成；若复核指出问题，实施方修复后**重跑复核**，直到 pass。复核档存 `docs/rounds/r9-task-<task>-review.md` 并入库。

## Review Focus

- **状态面/锁存全不变的长窗口**（沉静关卡、暂停、暗屏）：状态面与锁存面都没有跳变可对齐，工具必须明确报"无法反解"，而不是退回打分最优。
- **上电段 f0–f19**：两核 RAM 填充不同（quickerNES `0xFF` vs Rust `0x00`），任何对齐在 f<20 都不可信；窗口下界必须硬约束。
- **加载窗**（`$005A != $FF`）：房间请求字节在三条腿里推进速度不同，中间值集合都不一样；这类帧必须被排除出分母，且排除后要报剩余分母。
- **连续两帧以上的多值跳变**（电梯、死亡链）：单点锚定会解出"半帧"，需要连续跳变序列整体拟合。
- **轨迹文件中途被并发重写**：行数不等于请求帧数时必须 fail-loud，不许拿半截文件出判据。

---

### Task 1: 因果锚帧映射器 `align_legs.py`

**Files (T1):**
- Create: `tools/movie_ref/align_legs.py`
- Test: `tools/tests/test_align_legs.py`
- Output: `data/status/leg_offsets.json`（in-repo status artifact，非 gitignore）

**Interfaces (T1):**
- Consumes: 三腿轨迹 JSONL。BizHawk 行 `{"f","status","room","scrolldir","joy1_latch",...}`；Mesen2 行 `{"f","w":{"Joy1Latch":..,"0000-00ff":<hex>,"0100-07ff":<hex>}}`（命名键 `SamusStatus`/`RoomNumber` 可选，优先于 range）；Rust 行 `{"f","w":{"zp":<hex>,"iram":<hex>}}`（zp=$0000-$00FF，iram=$0100-$07FF，均扁平 hex）。
- Produces (T1):
  - `align(seq_a: dict[int, int], seq_b: dict[int, int], max_lag: int = 4) -> tuple[int, int]` 返回 `(offset, changes)`，语义 `a[f] == b[f + offset]`；`changes` 为 a 侧变化帧的最大连续段数（与 Step 1 的 `(1, 4)` 一致）；裸变化帧数单列 `change_frames`，段数单列 `change_runs`；`change_runs < 3` 时抛 `ValueError("insufficient anchor changes")`。
  - `load_latch(path: str, leg: str) -> dict[int, int]`，`leg ∈ {"biz","mesen","rust"}`，返回 `frame -> $0014` 值。
  - `load_state(path: str, leg: str) -> dict[int, dict[str, int]]`，返回 `frame -> {"status","room","latch"}`；缺键/长度不符一律 fail-loud。
  - CLI：`python3 tools/movie_ref/align_legs.py --bizhawk P --mesen P --rust P --out data/status/leg_offsets.json [--start 20] [--end 30000]`，输出 JSON：
    `{"window":[start,effective_end],"effective_end":N,"fit_window":[start,N],"anchors":{"addr":"$0014","change_frames":N,"change_runs":N,"by_leg":{...}},"offsets":{"biz_to_mesen":int,"rust_to_mesen":int},"latch_offsets":{...},"per_field":{...},"agree_rate":{"biz":float,"rust":float},"state_joint_agree_rate":{...},"latch_agree_rate":{...},"residuals":{...},"provenance":{...},"verdict":"aligned"|"insufficient"|"field-disagreement"|"latch-state-gap-anomaly"}`（offsets 为状态面共识；latch_offsets 单列 $0014，恒等于状态面 +1）。

- [ ] **Step 1 (T1): 写失败测试**

```python
from tools.movie_ref.align_legs import align

def test_align_recovers_known_offset():
    a = {f: 0 for f in range(20, 60)}; b = dict(a)
    for f in (25, 33, 41, 50):          # 四次输入跳变
        a[f] = 0x01 if f % 2 else 0x81
        b[f + 1] = a[f]                 # b 比 a 晚一帧
    assert align(a, b) == (1, 4)

def test_align_refuses_without_anchor():
    import pytest
    a = {f: 0 for f in range(20, 60)}
    with pytest.raises(ValueError, match="insufficient anchor changes"):
        align(a, dict(a))
```

- [ ] **Step 2 (T1): 跑测试确认红**

Run: `python3 -m pytest tools/tests/test_align_legs.py -v`
Expected: FAIL — `ModuleNotFoundError: tools.movie_ref.align_legs`

- [ ] **Step 3 (T1): 实现**

在 `tools/movie_ref/align_legs.py` 里实现 `align`：只在"值变化帧"集合上比较（`a[f] != a[f-1]` 的 f），对每个 `offset ∈ [-max_lag, max_lag]` 计一致数，取最大且唯一；`change_frames`/`change_runs` 双计数，`change_runs < 3` 或裸帧不足是多解/不足的拒绝条件。状态面 CLI 另用**全帧**一致数（status/room 大部分帧不变，只取变化帧样本太少）：`offsets` 取 status/room 共识，`$0014` 单列 `latch_offsets` 并断言 latch = state + 1。再实现 `load_latch`/`load_state` 与 CLI（`ram_map` 具名常量只用于文档字符串，工具侧硬编码 `$0014`/`$005A`/`$0300` 并注明来源）。

- [ ] **Step 4 (T1): 跑测试确认绿**

Run: `python3 -m pytest tools/tests/test_align_legs.py -v`
Expected: PASS（2 passed）

- [ ] **Step 5 (T1): 用真实轨迹生成偏移表**

Run: `python3 tools/movie_ref/align_legs.py --bizhawk analysis/traces/bizhawk-6725M-post.jsonl --mesen analysis/traces/_probe-axis.frames.jsonl --rust /tmp/rust-states-t2.jsonl --out data/status/leg_offsets.json`
Expected: 打印 `offsets={"biz_to_mesen":+1,"rust_to_mesen":0}` 与 `latch_offsets={"biz_to_mesen":+2,"rust_to_mesen":+1}` 且 `verdict=aligned`；把实际值与 agree_rate 记入本 task 的 commit body。offsets 只用于状态面；锁存面恒比状态面 +1，不得拿 latch_offsets 当帧轴。

- [ ] **Step 6 (T1): 提交**

```bash
git add tools/movie_ref/align_legs.py tools/tests/test_align_legs.py data/status/leg_offsets.json
git commit -F /tmp/msg-task1.txt   # body 写实测偏移与 agree_rate，禁裸反引号
```

---

### Task 2: 分叉分类器 `classify_episodes.py`

**Files (T2):**
- Create: `tools/movie_ref/classify_episodes.py`
- Test: `tools/tests/test_classify_episodes.py`

**Interfaces (T2):**
- Consumes: Task 1 的 `offsets`（`data/status/leg_offsets.json`）与三腿轨迹。
- Produces (T2):
  - `Episode = dict`，键 `{"start":int,"end":int,"fields":list[str],"kind":str,"during_load":bool}`，`kind ∈ {"phase","value"}`。
  - `classify(legs: dict[str, dict[int, dict]], offsets: dict[str, int], fields: tuple[str, ...], start: int, end: int, load_key: str = "room") -> list[Episode]`
  - CLI：`python3 tools/movie_ref/classify_episodes.py --bizhawk P --mesen P --rust P --offsets data/status/leg_offsets.json [--json-out analysis/traces/episodes.json]`

- [ ] **Step 1 (T2): 写失败测试**

```python
from tools.movie_ref.classify_episodes import classify

LEGS = {
    "mesen": {f: {"status": 3, "room": 0xFF} for f in range(20, 40)},
    "rust":  {f: {"status": 3, "room": 0xFF} for f in range(20, 40)},
}
def test_transient_is_phase():
    LEGS["mesen"][25] = {"status": 4, "room": 0xFF}
    eps = classify({"mesen": LEGS["mesen"], "rust": LEGS["rust"]},
                   {"mesen": 0, "rust": 0}, ("status",), 20, 39)
    assert len(eps) == 1 and eps[0]["kind"] == "phase"

def test_load_window_is_flagged():
    LEGS["mesen"][30] = {"status": 9, "room": 0x09}
    eps = classify({"mesen": LEGS["mesen"], "rust": LEGS["rust"]},
                   {"mesen": 0, "rust": 0}, ("status",), 20, 39)
    assert any(e["during_load"] for e in eps)
```

- [ ] **Step 2 (T2): 跑测试确认红**

Run: `python3 -m pytest tools/tests/test_classify_episodes.py -v`
Expected: FAIL — `ImportError: cannot import name align`

- [ ] **Step 3 (T2): 实现**

聚合规则：三腿按 offset 归一到 Mesen2 帧轴后逐帧比 `fields`；连续失配帧合成一段。`kind` 判据：段内每个失配字段，若该腿该字段的**同一个值**在另一腿的 `[start-2, end+2]` 内出现，则该字段为 phase，否则 value；一段只要有一个字段是 value，整段记 `value`。`during_load` = 段内任一帧任一条腿的 `load_key` 不等于 `0xFF`。

- [ ] **Step 4 (T2): 跑测试确认绿**

Run: `python3 -m pytest tools/tests/test_classify_episodes.py -v`
Expected: PASS（2 passed）

- [ ] **Step 5 (T2): 对真实数据出 33 段判定表**

Run: `python3 tools/movie_ref/classify_episodes.py --bizhawk analysis/traces/bizhawk-6725M-post.jsonl --mesen analysis/traces/_probe-axis.frames.jsonl --rust /tmp/rust-states-t2.jsonl --offsets data/status/leg_offsets.json --json-out analysis/traces/episodes.json`
Expected: 打印段数与 `phase/value/during_load` 计数；要求与已记录的 33 段量级一致，差异必须逐段写出原因。

- [ ] **Step 6 (T2): 提交**

```bash
git add tools/movie_ref/classify_episodes.py tools/tests/test_classify_episodes.py
git commit -F /tmp/msg-task2.txt
```

---

### Task 3: 判据口径落 spec（唯一真相源）

**Files (T3):**
- Modify: `docs/spec/oracle-verification-chain.md`（§3.1 增补「帧映射规约」小节；§3.3 表替换为 Task 2 的实跑输出）

**Interfaces (T3):**
- Consumes: Task 1 的 `data/status/leg_offsets.json` 实测值、Task 2 的 `analysis/traces/episodes.json` 计数。
- Produces: spec 内新增两条可引用编号规则 `R6`（状态面帧映射 + 锁存面 ±1 corroboration，禁打分最优）与 `R7`（加载窗排除、分母须报剩余量），供后续所有取证引用。

- [ ] **Step 1 (T3): 写规则 R6**

在 §3.1 增加：跨腿帧映射由 `tools/movie_ref/align_legs.py` 用状态面（`$0300` status + `$005A` room）全帧一致数反解，结果存 `data/status/leg_offsets.json`；`$0014` 锁存面单列 `latch_offsets`，实测恒等于状态面 +1，只作 corroboration；分辨率 ±1 帧；`change_runs < 3` 视为不可对齐。

- [ ] **Step 2 (T3): 写规则 R7**

在 §3.1 增加：`$005A != $FF`（`ROOM_NUMBER`，`constants_ram.asm:589` 语义为"正在加载的房间号"）的帧不进分母；报告必须给"总帧 / 分母 / 排除帧"三个数。

- [ ] **Step 3 (T3): 替换 §3.3 表**

把现有 33 段表换成 Task 2 的实跑计数（段数、phase 数、value 数、during_load 数），并在表下注明生成命令。

- [ ] **Step 4 (T3): 门禁自检**

Run: `python3 gates/cli.py docs-index`（若不存在则 `python3 gates/cli.py check`）
Expected: 无新增 FAIL（`doc_dup_line_violation` 不触发）

- [ ] **Step 5 (T3): 提交**

```bash
git add docs/spec/oracle-verification-chain.md
git commit -F /tmp/msg-task3.txt   # 必须含 Evidence: 指向 leg_offsets.json 与 episodes.json
```

---

### Task 4: 覆盖台账 + 棘轮门 `coverage-ratchet`

**Files (T4):**
- Create: `tools/coverage_ledger.py`, `data/status/coverage_ledger.json`
- Modify: `gates/cli.py`（新增 `coverage-ratchet` 门）
- Test: `tools/tests/test_coverage_ledger.py`

**Interfaces (T4):**
- Consumes: `analysis/traces/episodes.json`（Task 2）与 `docs/knowledge/` 现有房间/敌人清单之外的最小单元表（先只做"机制"面，房间/敌人面留后续）。
- Produces (T4):
  - 台账条目：`{"id":str,"kind":"mechanism","state":"unobserved"|"route-only"|"verified","evidence":list[str]}`。
  - `promote(entry_id: str, state: str, evidence: str) -> None`（只许升级：`unobserved→route-only→verified`，降级抛错）。
  - 门判据：state 计数单调不减；条目 id 集合不得减少。

- [ ] **Step 1 (T4): 写失败测试**

```python
import pytest
from tools.coverage_ledger import Ledger

def test_promote_is_monotonic(tmp_path):
    p = tmp_path / "l.json"; p.write_text('{"items":[]}')
    led = Ledger(p); led.add("morphball-unroll", "mechanism")
    led.promote("morphball-unroll", "route-only", "movie f1030..1080")
    with pytest.raises(ValueError, match="downgrade"):
        led.promote("morphball-unroll", "unobserved", "x")
```

- [ ] **Step 2 (T4): 跑测试确认红**

Run: `python3 -m pytest tools/tests/test_coverage_ledger.py -v`
Expected: FAIL — `ImportError: cannot import name classify`

- [ ] **Step 3 (T4): 实现台账与门**

实现 `Ledger`（`add/promote/load/save`，JSON 稳定排序）；在 `gates/cli.py` 加 `gate_coverage_ratchet()` 读台账并核对与 `data/status/coverage_ledger.baseline.json` 的计数关系，注册到 `check` 的快层；写 `tools/coverage_ledger.py --seed` 生成初始条目（先收 `morphball-unroll`、`room-load-chain`、`door-transition` 三条，state 全 `unobserved`）。

- [ ] **Step 4 (T4): 跑测试确认绿 + 门自检**

Run: `python3 -m pytest tools/tests/test_coverage_ledger.py -v && python3 gates/cli.py check`
Expected: 1 passed；`check` 出现 `coverage-ratchet PASS`

- [ ] **Step 5 (T4): 提交**

```bash
git add tools/coverage_ledger.py tools/tests/test_coverage_ledger.py data/status/coverage_ledger.json data/status/coverage_ledger.baseline.json gates/cli.py
git commit -F /tmp/msg-task4.txt   # 门禁改动单独 commit，body 写基线影响
```

---

### Task 5: 机制级最小重现 runner

**Files (T5):**
- Create: `tools/movie_ref/mechanism_scene.py`
- Test: `tools/tests/test_mechanism_scene.py`

**Interfaces (T5):**
- Consumes: `tools/scenario_run.py`（Mesen2 腿）与 `crates/game-core` 的 `rust_screenshot` 二进制（Rust 腿）；Task 1 的 `align()`。
- Produces (T5):
  - `run_scene(case_path: str, frames: int) -> dict`，返回 `{"mesen":dict[int,dict],"rust":dict[int,dict],"offset":int}`，实际调用两个已有 runner 后对齐。
  - CLI：`python3 tools/movie_ref/mechanism_scene.py --case cases/<name>.txt --frames 120 --json-out analysis/traces/scene-<name>.json`

- [ ] **Step 1 (T5): 写失败测试**

```python
from tools.movie_ref.mechanism_scene import compare

def test_compare_reports_first_diff():
    m = {f: {"status": 3} for f in range(0, 10)}
    r = {f: {"status": 3} for f in range(0, 10)}
    r[7]["status"] = 1
    out = compare(m, r, ("status",))
    assert out["first_diff"] == 7 and out["mismatch"] == 1
```

- [ ] **Step 2 (T5): 跑测试确认红**

Run: `python3 -m pytest tools/tests/test_mechanism_scene.py -v`
Expected: FAIL — `ImportError: cannot import name Ledger`

- [ ] **Step 3 (T5): 实现**

`compare(mesen, rust, fields)` 纯函数（首分歧帧 + 失配计数 + 每字段计数）；`run_scene` 只负责调子进程并把产物读成上述形状，帧数上限 600（超限抛错，防"又去碰长片"）。

- [ ] **Step 4 (T5): 跑测试确认绿**

Run: `python3 -m pytest tools/tests/test_mechanism_scene.py -v`
Expected: PASS

- [ ] **Step 5 (T5): 端到端跑一个现有短场景**

Run: `python3 tools/movie_ref/mechanism_scene.py --case cases/case-morphball.txt --frames 120 --json-out analysis/traces/scene-morphball.json`
Expected: 打印 `first_diff` 或 `all-match`；结果与 L4 矩阵 case-morphball 的既有结论不得矛盾（矛盾则如实记录为工具缺陷，不得改期望值）。

- [ ] **Step 6 (T5): 提交**

```bash
git add tools/movie_ref/mechanism_scene.py tools/tests/test_mechanism_scene.py
git commit -F /tmp/msg-task5.txt
```

---

### Task 6: 两段残留的判定归档

**Files (T6):**
- Create: `docs/findings/movie-6725m-residual-episodes.md`
- Modify: `docs/findings/movie-bizhawk-6725m.md`（§11.2/§11.3/§11.4 三处复核点名的硬错，按证伪更正文化原地标注）

**Interfaces (T6):**
- Consumes: Task 1 的 offset、Task 2 的 episodes、Task 5 的 scene 结果。
- Produces: 两条结论——`f943..952` 与 `f1030..1080` 各自的 `kind` 与 `during_load` 判定，以及"是否进机制级复现"的决定与理由。

- [ ] **Step 1 (T6): 写判定表**

三列：段 / `kind` / `during_load` / 是否进复现 / 理由（引用 Task 2 输出与 `$005A` 语义锚）。

- [ ] **Step 2 (T6): 改 §11.2/§11.3/§11.4**

按复核意见更正三处数字，原文保留并加 `> 【2026-11-18 更正】` 标注。

- [ ] **Step 3 (T6): 门禁自检**

Run: `python3 gates/cli.py check`
Expected: PASS（不引入重复行违规）

- [ ] **Step 4 (T6): 提交并推送**

```bash
git add docs/findings/movie-6725m-residual-episodes.md docs/findings/movie-bizhawk-6725m.md
git commit -F /tmp/msg-task6.txt
git push
```

- [ ] **Step 5 (T6): 派独立复核**

按 AGENTS.md §5 派 verifier 证伪复核 Task 1–6（重点：`align()` 的唯一性判据、`kind` 判据是否可被"对面随便出现同值"骗过、台账单调性是否可被删条目绕过），pass 后把复核档 `docs/rounds/r9-task-three-leg-alignment-review.md` 入库。

---

## 自检记录（写完计划后执行）

- **Spec 覆盖**：三腿分工→Task 1/3；级联判读→Task 2；覆盖台账→Task 4；机制级复现→Task 5；残留归档→Task 6。spec 五条仲裁规则中的①③④在 Task 2 的 `kind` 判据与 Task 3 的 R7 里各有一条机械落点；②（A 腿只出覆盖）由 Task 4 台账表达。
- **步粒度**：每个 step 都是"写一个测试 / 跑一条命令 / 改一处文档"，无"处理边界情况"这类空指令。
- **类型一致**：`align()` 返回 `(offset, changes)`，Task 5 的 `compare()` 用 `fields` 元组，Task 2 的 `Episode` 键名在 Task 4/6 里原样引用。
- **Review Focus**：五条各配有测试或硬约束（无跳变抛错→Task 1 Step 1 第二条测试；上电段下界→Task 1 CLI `--start 20`；加载窗→Task 2 `during_load`；多值跳变→Task 1 `align` 唯一性；半截轨迹→Task 1 Step 5 的行数校验）。
- **比例**：全文以接口签名与判据为主，实现体只出现在纯函数级算法（`align`/`classify`），未转录业务代码。
