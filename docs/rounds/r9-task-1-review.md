# R9 Task 1 对抗复核 verdict——因果锚帧映射器（commit `9b92c5e4`）

- 复核员：verifier（**证伪立场，非实施方**；未参与设计/实现）
- 日期：2026-11-18
- 复核对象：commit `9b92c5e4`（`tools/movie_ref/align_legs.py` + `tools/tests/test_align_legs.py` + `data/status/leg_offsets.json`）
- 验收标准：`docs/superpowers/plans/2026-11-18-three-leg-oracle-alignment.md` Task 1
- **总 verdict：`reject`（P0）**——`data/status/leg_offsets.json` 交付的 `offsets` 是**锁存面偏移**，
  不是状态面帧偏移。按它归一，biz / rust 两腿各错 1 帧，Task 2（plan:142 用同一 offset 逐帧比
  state 字段）会拿到系统性错相位。

> 本复核未 import、未调用 `align_legs` 的任何函数做测量；所有数字来自 `/tmp/verify*.py`
> 独立脚本（仅复现 CLI 时执行过一次工具本体，见 §附录 A-H）。

---

## 0. 方法与独立性

- 大文件（`/tmp/rust-states-t2.jsonl` 24 MB）全部逐行流式读，未 `read_text`。
- 对比口径统一为 `a[f] == b[f+k]`（与工具 `_fit` 同义），窗口 `f=20..1199`（`--start 20` 的硬下限）。
- 未执行（约束禁止）：`gates/cli.py check`、`cargo test`、L4。commit body 声称的
  `PASS=22 FAIL=0 SKIP=3 TOLERATED=1` **本复核未验证**（§9）。
- 未修改 `tools/` `crates/` `data/` `gates/` 下任何文件（`git status` 只有本文件新增）。

---

## 1. 偏移与既有结论矛盾 —— **FAIL（P0）**

### 1.1 逐字段最优 k（biz `post[f]` vs mesen `[f+k]`，f=20..1199）

命令：`python3 /tmp/verify1_fields.py`

| field | k=-2 | k=-1 | k=0 | k=+1 | k=+2 | 最优 k |
|---|---|---|---|---|---|---|
| `status` | 1076/1180 .911864 | 1084/1180 .918644 | 1089/1180 .922881 | **1096/1180 .928814** | 1091/1179 .925360 | **+1** |
| `room` | 1146/1180 .971186 | 1152/1180 .976271 | 1157/1180 .980508 | **1158/1180 .981356** | 1151/1179 .976251 | **+1** |
| `health_lo` | 1180/1180 1.0 | 1.0 | 1.0 | 1.0 | 1.0 | 退化（窗口恒 0，仅 1 个取值） |
| 锚 `joy1_latch` | 1124/1180 .952542 | 1069/1180 .905932 | 1140/1180 .966102 | 1086/1180 .920339 | **1169/1179 .991518** | **+2** |

biz 状态面联合（`status`+`room`+`scrolldir`）逐 k：-2 .883051 / -1 .894915 / 0 .904237 /
**+1 1074/1180 .910169（最优）** / +2 1065/1179 .903308。

### 1.2 rust `[f]` vs mesen `[f+k]`（f=20..1199，1179 行缺 21 帧）

命令：`python3 /tmp/verify2_rust.py`、`python3 /tmp/verify3_more.py`

| field | 最优 k | 数值 |
|---|---|---|
| `SamusStatus` | **0** | 1154/1166 .989708（+1 时 .947684） |
| `RoomNumber` | **0** | 1158/1166 .993139（+1 时 .990566） |
| `Health` | 退化 | 全 k = 1.0 |
| 锚 `zp[$0014]` | **+1** | 1160/1166 .994854（k=0 时 .921955） |

rust 状态面联合：**k=0 1146/1166 .982847（最优）**；k=+1 1093/1166 .937393。

### 1.3 结论：差恒为 +1，且四组独立配对全部复现

**状态面（status/room/联合）三面一致地给出 biz `+1`、rust `0`；锚面给出 biz `+2`、rust `+1`。
两者差值在两条腿上**都恰好是 +1**，不是巧合：

| 配对 | 状态面最优 k | 锚面最优 k | 差 |
|---|---|---|---|
| biz **post** ↔ mesen probe | +1 | +2 | +1 |
| biz **pre** ↔ mesen probe | 0 | +1 | +1 |
| rust ↔ mesen probe | 0 | +1 | +1 |
| biz **pre** ↔ `/tmp/tmpaxis2.jsonl`（另一次 Mesen2 录制） | 0 | +1 | +1 |

命令：`/tmp/verify1_fields.py`、`/tmp/verify2_rust.py`、以及 §附录 A-G 的 pre/tmpaxis2 交叉脚本。

这与仓库既有结论**完全一致**，不是"谁记错了"：

- `docs/spec/oracle-verification-chain.md:27` 规则 5 原文：**「$0014 输入锁存面不得当帧轴判据。
  它的最优点天然比状态面偏一行（两侧同属性），拿它对齐会假阳性」**——本复核用四组配对把这个
  "偏一行"量化成了恒等式：`latch_offset = state_offset + 1`。
- `docs/spec/oracle-verification-chain.md:41`：正确相位映射 **`post[f]` ↔ `emu[f+1]`**；
  §3.1 的 `post[f]==pre[f-1]` 28388/28388=100% 也说明 post 与 pre 差 1 帧。
- `plan:89` Step 5 Expected 写的是 `{"biz_to_mesen":+1|0,"rust_to_mesen":+1|0}`。

⇒ **工具错，不是 spec 错**：`+2/+1` 是锁存面的最优映射，被当成了状态面帧映射。

### 1.4 按工具偏移归一的实际代价（这不是风格问题）

若 Task 2 真的拿 `offsets` 去归一并逐帧比 state 字段：

| 腿 | 用工具偏移 | 用正确偏移 | 多出的失配帧 |
|---|---|---|---|
| biz | +2：1065/1179 = .903308 | +1：1074/1180 = .910169 | **+8**（106→114 失配） |
| rust | +1：1093/1166 = .937393 | 0：1146/1166 = .982847 | **+53**（20→73 失配） |

rust 腿会凭空多出 4.5 个百分点的失配底噪，且全部是 ±1 相位噪声——Task 2 的
`phase/value` 分类会被这批假 `phase` 段淹没，真正的一帧 value 差异可能被掩盖。
**这是 P0，不是 finding。**

---

## 2. 锁存能否当因果锚 —— **FAIL**

### 2.1 两侧取值集合不同

命令：`python3 /tmp/verify4_latch.py`（窗口 f=20..1199，直方图）

| 值 | biz `joy1_latch` | mesen `Joy1Latch` | rust `zp[$0014]` |
|---|---|---|---|
| 0 | 210 | 212 | 204 |
| 1 | 478 | 478 | 477 |
| 2 | 386 | 388 | 381 |
| 3 | **4** | **0** | **4** |
| 4 | 1 | 1 | 1 |
| 9 | 37 | 35 | 35 |
| 65 | 2 | 2 | 2 |
| 66 | **1** | **5** | 1 |
| 67 | **2** | **0** | **2** |
| 129 | 1 | 1 | 1 |
| 130 | 58 | 58 | 58 |

- `set(biz) == set(mesen)` → **False**；`biz − mesen = {3, 67}`，`mesen − biz = {}`。
- `set(rust) == set(mesen)` → **False**；`rust − mesen = {3, 67}`。
- mesen 从不采样 3/67，且 66 出现 5 次（biz 仅 1 次）⇒ **两侧对 `$0014` 的采样点不在同一子帧相位**，
  取值域不是同一集合。§5 的 7 个残差正是这批 `3/67 ↔ 2/66` 的混叠。

### 2.2 滞后分布

BizHawk 侧（`p1` 变化 → `joy1_latch` 变化，f=20..1199）：

- `p1` 变化帧 97 个；`joy1_latch` 变化帧 97 个；**同帧重合 96/97**。
- `next_latch_change − p1_change` 分布：`{0: 96, 2: 1}`；唯一分歧在 **f=1132**
  （p1 从 9→1，锁存仍是 9，直到 f=1134 才跟上，滞后 2 帧）。
- 即 BizHawk 侧锁存对输入的滞后是 **0 帧（97 中 96）**，偶发 2 帧。

Mesen2 侧**无法测**（probe-axis 轨迹只有 RAM watch，无输入列）。而 §1.3 已证明 mesen 的锁存相位
比状态面**晚 1 帧**（biz/rust 两腿同属性）。⇒ 两侧"锁存相对输入"的滞后分布**不可比**，
锚定法只能在"值同步变化"的意义上用，**分辨率上限 ±1 帧**，给不出精确帧映射。

⇒ 该项判 **FAIL**：把 `$0014` 当**唯一**帧轴锚，等价于用分辨率 ±1 的尺子去定 1 帧的偏移，
而它自身的系统性 +1 偏置又恰好被当成待求的偏移。

---

## 3. `changes` 语义 —— **PASS（附 finding）**

命令：`python3 /tmp/verify5_fit.py`、附录 A-I。

- biz 窗口内 `joy1_latch` **裸变化帧 97 个**，最大连续段 **13 段**；实现取 13。
- rust：96 帧 / 13 段。
- 语义确实被实现为"连续段数"（`_count_runs`，align_legs.py:67-76），docstring 与 commit body
  都明说了"变化帧最大连续段数"；计划 Step 1 的合成用例本身也期望 `4 次按键 → changes=4`
  （段数），而不是 4 个裸变化帧以外的别的口径。
- **不是判据弱化**：我实测 `2 次按键（4 个裸变化帧，2 段）→ ValueError: insufficient anchor changes`，
  `3 次按键 → (1,3)`。段语义在常规稠密场景下**比裸帧阈值更严**（正文写 "如果实现偷偷放宽了
  `changes<3`，必须在文档里显式说明"，此条不成立）。

但有两处必须修（finding，不单独构成 reject）：

1. **计划文字自相矛盾**：`plan:48` 写"跳变次数"，`plan:79` Step 3 写"值变化帧集合"，实现是"段数"。
   三处口径必须统一。
2. **JSON 单值 `changes:13` 掩盖了 97**：读者无法从交付物判断锚覆盖度（13 段可以是 13 帧，
   也可以是 97 帧）。应同时输出 `change_frames` 与 `change_runs`。
3. （次要）`_fit` 用 `participants` 算段（align_legs.py:109）：稀疏腿某些 offset 下participants
   会被切成更多段，从而让 `runs<3` 这道闸更容易过。应固定用 `a` 侧窗口内 change frame 全集算段。
   本例未触发（两腿 13 段），但属于口径不严。

---

## 4. 轨迹文件被重写 —— **PASS**

命令：`python3 /tmp/verify6_trace.py`。

- 旧文件 `analysis/traces/.archive/_probe-axis-20261118-140420/_probe-axis.frames.jsonl`：
  1200 行 f=1..1200，`w` 键只有 `0000-00ff`/`0100-07ff`（**确无 `Joy1Latch`**）；
  mtime **2026-11-18 11:56:55**，归档目录 mtime **14:04:20**，新文件 mtime **14:04:44**
  ⇒ 归档保留了旧 mtime，重生成发生在归档之后，与 commit body 描述一致。
- 新旧逐帧比对：
  - frame 集合：完全相同 `{1..1200}`。
  - `0000-00ff`：**1200/1200 全等**。
  - `0100-07ff`：**1200/1200 全等**。
  - 新 `w["Joy1Latch"]` == 新 `0000-00ff[$0014]`：**1200/1200**。
  - 新键集合是旧键集合的严格超集（仅新增 `Joy1Latch`）。
- ⇒ "重生成没有改变既有事实"成立；`0000-00ff` 与 `0100-07ff` 两个 range 串 1200/1200 全等。

**Caveat（不是 fail）**：旧文件虽无 `Joy1Latch` 键，但 `$0014` 的字节**本来就在 `0000-00ff` 里**
（`0x14*2` 位置），拒绝 range fallback 是设计选择（可辩护：避免"键名缺失=数据缺失"的静默降级），
但不能说"旧轨迹没有 $0014 证据"。

---

## 5. 唯一性论证 —— **PASS（附 finding）**

命令：`python3 /tmp/verify5_fit.py`。

biz→mesen 偏移表（97 个变化帧）：

| k | -4 | -3 | -2 | -1 | 0 | +1 | +2 | +3 | +4 |
|---|---|---|---|---|---|---|---|---|---|
| 一致 | 72 | 12 | 73 | 10 | 72 | 6 | **90** | 15 | 81 |
| 率 | .7423 | .1237 | .7526 | .1031 | .7423 | .0619 | **.9278** | .1546 | .8351 |

- 唯一最大成立：90 vs 次高 81（+4），差 9。
- **7 个残差全部连成一段 f=661..672**（12 帧）：f=661/663/665/667 `a=3 vs b=2`；
  f=669/671 `a=67 vs b=66`；f=672 `a=2 vs b=66`。
- **不是分段偏移**：四等分段（~295 帧）最优 offset 全是 +2（1/1、4/4、9/16、76/76）。
- **不是 ±1 抖动**：7 个残差里用邻居 offset（±1）能解释的 = **0/7**。
- rust 侧同理：+1 = 90/96，残差 6 个全在 f=662..672，分段全 +1，±1 解释 0/6。

⇒ 定量结论：**单一全局偏移能解释 90/97 = 92.8% 的锁存变化帧，不能解释全部 97 个**；残差不是
偏移漂移，而是 §2.1 的取值域混叠（mesen 不采样 3/67）在一个 12 帧爆发段里的集中体现。
"单一全局偏移"是**最佳简单模型**，但**不是精确模型**；残差段必须写进交付物，且它恰好证明了
"用锁存的逐帧相等去定 1 帧偏移"这件事本身有 7% 的不可消除噪声。

---

## 6. 判据未被放宽 —— **PASS**

命令：`python3 -m pytest tools/tests/test_align_legs.py -v`（5 passed in 0.05s）；
`grep -n "pragma|environ|getenv|skip|xfail|DSH_|monkeypatch" tools/tests/test_align_legs.py` → 无命中。

- 无 `pragma`、无环境变量开关、无 `skip`/`xfail`、无 monkeypatch；断言未被改弱。
- 计划 Step 1 的两条用例（`align(a,b)==(1,4)`、锚不足抛错）与计划原文逐字一致。
- 多解 fixture 的"修正"我独立复算过：`a={f: f%4==0 → 1 else 0, 20..59}`，`b[f]=a[f-4]`：
  - change frames = **19**（从 f=21 起，因为 `f-1` 必须在 `a` 里），runs = 10；
  - 打分：k=-4 **18/19**、k=0 **19/19**、k=+4 **19/19**，其余 ≤10/19。
  - ⇒ 并列解恰为 `{0, +4}` 两个，`-4` 是 18/19 **不是并列**。测试注释"0 与 +4 各 19/19"属实，
    该用例**真的**在考多解拒绝，不是把测试改到能过。

---

## 7. 总 verdict：`reject`

| # | 检查点 | 判定 |
|---|---|---|
| 1 | 偏移 vs 既有结论 | **FAIL（P0）**——工具输出的是 latch 偏移，状态面应为 biz +1 / rust 0 |
| 2 | 锁存可否当因果锚 | **FAIL**——取值域不同、滞后不可比、分辨率 ±1 |
| 3 | `changes` 语义 | PASS（附 finding：文档口径 + 双计数） |
| 4 | 轨迹重生成 | PASS |
| 5 | 唯一性 | PASS（附 finding：90/97，残差连续成段） |
| 6 | 判据未放宽 | PASS |

**reject 的唯一理由是 #1**：交付物 `data/status/leg_offsets.json` 会被 Task 2 直接消费
（plan:142），而它的 `offsets` 是**另一个物理量**（锁存面相位）的偏移。工具本身写得干净
（拒绝 fallback、不挑偏移、多解抛错、测试不弱化），但**输出语义错了**，且 commit body 自己
已承认"biz 实测 +2 与计划预期 +1|0 不同"——把与验收标准冲突的结果直接入库、留给复核，
不满足 Task 1 的 Expected。

---

## 8. 修复清单（可直接执行）

### F1 `tools/movie_ref/align_legs.py`（核心）

1. **新增状态面锚**（这才是帧轴判据）：
   - mesen 侧解码（range 串是扁平字节 hex）：
     - `$0300`（status）→ `w["0100-07ff"][2*(0x300-0x100) : +2]`
     - `$005A`（room）→ `w["0000-00ff"][2*0x05A : +2]`
     - `$0106`（health）→ `w["0100-07ff"][2*(0x106-0x100) : +2]`
   - biz 侧：行内 `status` / `room` / `health_lo`；rust 侧：`zp` 同址解码
     （`$0300` → `zp[2*0x300:+2]`，`$005A` → `zp[2*0x05A:+2]`，`$0106` → `zp[2*0x106:+2]`），
     或对 `/tmp/tmpaxis2.jsonl` 这类命名键轨迹直接用 `SamusStatus`/`RoomNumber`/`Health`。
2. **`offsets` 取状态面逐字段共识**：要求 `status`、`room` 的最优 k 一致才写 `offsets`，
   不一致则 `verdict="field-disagreement"` 并逐字段列出（这正是本次要求的"按字段报偏移"）。
3. **锁存偏移单独出字段** `latch_offsets`，并写死断言/说明 `latch_offset == state_offset + 1`
   （本复核四组配对实测恒等）；若某次不满足，`verdict="latch-state-gap-anomaly"`。
4. `data/status/leg_offsets.json` 的 `offsets` 改由 §2 产出（本次应为 `{"biz_to_mesen":1,"rust_to_mesen":0}`），
   `latch_offsets` 保留 `{"biz_to_mesen":2,"rust_to_mesen":1}`。
5. align_legs.py:109 `_count_runs(participants)` → `_count_runs([f for f in changes if start <= f <= end])`
   （段数口径固定在 a 侧全集，避免稀疏腿把一段切成多段而放宽 `runs<3`）。
6. align_legs.py:101 文案：`"insufficient anchor changes"` 在该分支实际比的是 matches，
   改成 `"insufficient anchor matches"`，或直接用 `len(changes)`。
7. align_legs.py:303 `return 0` → `return 0 if verdict == "aligned" else 1`（fail-loud；
   当前 insufficient 也返回 0，CI 会把"未对齐"当成功）。
8. JSON 增加：`change_frames`、`change_runs`、`effective_end`（实际参与拟合的最后一帧，本例 1200，
   而不是把 `--end 30000` 原样写进 `window`）、`provenance`（三输入路径 + sha256 或 mtime + last_f）。

### F2 `data/status/leg_offsets.json`（重生成）

建议 schema（本次实测值）：

```json
{
  "window": [20, 1200],
  "effective_end": 1200,
  "anchors": {"addr": "$0014", "change_frames": 97, "change_runs": 13},
  "offsets": {"biz_to_mesen": 1, "rust_to_mesen": 0},
  "latch_offsets": {"biz_to_mesen": 2, "rust_to_mesen": 1},
  "per_field": {
    "biz": {"status": 1, "room": 1},
    "rust": {"status": 0, "room": 0}
  },
  "agree_rate": {"biz": 0.928814, "rust": 0.989708},
  "residuals": {"biz": [661, 663, 665, 667, 669, 671, 672], "rust": [662, 664, 666, 668, 670, 672]},
  "provenance": {"bizhawk": "analysis/traces/bizhawk-6725M-post.jsonl", "mesen": "analysis/traces/_probe-axis.frames.jsonl", "rust": "/tmp/rust-states-t2.jsonl"},
  "verdict": "aligned"
}
```

### F3 计划文字（`docs/superpowers/plans/2026-11-18-three-leg-oracle-alignment.md`）

- `:43` 删除 `(gitignored)`——实测 `git check-ignore data/status/leg_offsets.json` 为空，
  文件已入库；文字与仓库状态相反。
- `:48` `changes` 语义改为与实现一致："`changes` 为参与拟合的**变化帧数**；连续段数单列
  `change_runs`，`change_runs < 3` 时拒"（二选一，全文统一）。
- `:79` Step 3"值变化帧集合"与 `:48` 统一口径。
- `:89` Expected 改成状态面 `+1|0`，并加一句"锁存面为 `+2|+1`，恒比状态面大 1；不得用锁存面
  偏移做帧归一"。
- `:142` 明确"`offsets` 只用于 state 字段；若要比 `$0014` 必须用 `latch_offsets`"。
- Task 1 增加一条验收：`per_field` 必须包含 `status` 与 `room` 且两者一致。

### F4 `docs/spec/oracle-verification-chain.md` §3.1/§3.2

把"锁存比状态面偏一行"从经验句升级为实测恒等式，并登记本次事故：
`latch_best_k = state_best_k + 1`，四组配对（pre↔probe、post↔probe、rust↔probe、pre↔tmpaxis2）
全部成立；2026-11-18 Task 1 曾把 `+2/+1` 当帧偏移入库，已由本轮复核拒绝。

### F5 若要保留"因果锚"叙事

锚必须定义在**状态面同步变化**（status/room 同时变化）上，锁存只作 corroboration；
或明确写"锁存锚只能给 ±1 分辨率"，并禁止单独用它产出整数偏移。

---

## 9. 未执行 / 不可执行项

- `python3 gates/cli.py check`：**约束禁止**，commit body 的 `PASS=22 FAIL=0 SKIP=3 TOLERATED=1`
  本复核**未验证**，不作为本 verdict 的依据。
- `cargo test`、L4：约束禁止。
- 未核 Task 2/3 的任何内容（不在本 task 范围）。

---

## 附录 A：全部命令与关键输出（可复现）

A. 复跑 CLI 并 diff 交付物（只写 `/tmp`，未碰 `data/`）：

```
python3 tools/movie_ref/align_legs.py \
  --bizhawk analysis/traces/bizhawk-6725M-post.jsonl \
  --mesen analysis/traces/_probe-axis.frames.jsonl \
  --rust /tmp/rust-states-t2.jsonl --out /tmp/leg_offsets_check.json
# stderr: 3 条 "轨迹可能被截断" WARN
# OK biz->mesen: offset=+2 changes=13 agree=90/97
# OK rust->mesen: offset=+1 changes=13 agree=90/96
diff -u data/status/leg_offsets.json /tmp/leg_offsets_check.json   # → IDENTICAL
```

B. 逐字段偏移：`python3 /tmp/verify1_fields.py`（§1.1 表）。
C. rust/联合：`python3 /tmp/verify2_rust.py`、`python3 /tmp/verify3_more.py`（§1.2 表）。
D. 锁存直方图/滞后：`python3 /tmp/verify4_latch.py`（§2 表；p1 97 帧、重合 96、唯一分歧 f=1132）。
E. 拟合/分段/残差：`python3 /tmp/verify5_fit.py`（§5 表；biz 全长 f=20..28407 为 7942 帧 / **2057 段**，
   与 commit body 的 2057 一致；窗口内 97/13）。
F. 轨迹新旧比对：`python3 /tmp/verify6_trace.py`（§4；两 range 各 1200/1200；Joy1Latch 1200/1200）。
G. pre/tmpaxis2 交叉：pre↔probe 联合状态 k=0、锁存 k=+1；pre↔tmpaxis2 `SamusStatus` k=0 (.9959)、
   `RoomNumber` k=0 (.9827)、`Joy1Latch` k=+1 (972/981=.9918)、k=0 时 960/981=.9786。
   （旁注：未入库的 `tools/movie_ref/compare_cores.py` 的 `latch_align` 只扫 `shift in range(0,8)`，
   看不到更好地解，`docs/findings/movie-bizhawk-6725m.md` §11.2 的 "shift=0 / 0.9786" 是这个
   单侧搜索的产物；该文件不属于本 commit，仅记录现象。）
H. changes 阈值探针（import 工具，只读）：`2 次按键(4 裸帧/2 段) → ValueError`；`3 次按键 → (1,3)`。
I. 测试：`python3 -m pytest tools/tests/test_align_legs.py -v` → `5 passed`。
J. 属性核查：`git show --name-only 9b92c5e4` 恰 3 文件；`git check-ignore -v data/status/leg_offsets.json`
   → 空（未忽略）。

---
---

# 第二轮复核（追加；被复核 commit `d3440ff9`，2026-11-18）

> 第一轮内容原样保留（含其 `reject` 结论，那是当时交付物的真实状态）。本节只判"修复是否解决 P0、
> 是否引入新问题"。复核员立场不变：证伪、非实施方。

- 复核对象：`d3440ff9 fix(tools): 按状态面求帧偏移并单列锁存偏移`（5 文件：align_legs.py
  799 行、test_align_legs.py 190 行、leg_offsets.json 256 行、spec、plan）
- 独立测量脚本：`/tmp/verify7_r2.py`（自写解码，**不 import** `align_legs`）、
  `/tmp/make_gap_synth.py`（合成轨迹逼出边界行为）。CLI 复跑仅用于复现交付物。
- 未执行（约束禁止）：`gates/cli.py check`、`cargo test`、L4。

## R2-1 主偏移是否已改为状态面 —— **PASS**

`python3 tools/movie_ref/align_legs.py --bizhawk analysis/traces/bizhawk-6725M-post.jsonl
--mesen analysis/traces/_probe-axis.frames.jsonl --rust /tmp/rust-states-t2.jsonl
--out /tmp/leg_offsets_r2.json`，再 `diff -u data/status/leg_offsets.json /tmp/leg_offsets_r2.json`
→ **IDENTICAL**；stderr：

```
OK biz->mesen: state_offset=+1 latch_offset=+2 state_agree=1096/1180 latch_agree=1169/1179
OK rust->mesen: state_offset=+0 latch_offset=+1 state_agree=1154/1166 latch_agree=1160/1166
```

我用 `/tmp/verify7_r2.py` 从原始轨迹**独立重算**（a 侧 20..1199，b 侧 20..1200，自写 hex 解码）：

| 量 | 本轮独立值 | 我上轮值 | 工具 JSON | 一致 |
|---|---|---|---|---|
| biz status | **+1** 1096/1180 = .928814 | +1 .9288 | +1 1096/1180 .928814 | ✓ |
| biz room | **+1** 1158/1180 = .981356 | +1 .9814 | 同 | ✓ |
| biz joint(status+room) | +1 1075/1180 = .911017 | 1074/1180* | 1075/1180 .911017 | ✓ |
| rust status | **0** 1154/1166 = .989708 | 0 .9897 | 同 | ✓ |
| rust room | **0** 1158/1166 = .993139 | 0 .9931 | 同 | ✓ |
| rust joint | 0 1146/1166 = .982847 | 1146/1166 | 同 | ✓ |
| latch biz | **+2** 90/97 (.991518 全帧 1169/1179) | +2 90/97 | +2 | ✓ |
| latch rust | **+1** 90/96 (.994854 全帧 1160/1166) | +1 90/96 | +1 | ✓ |

\* 上轮 joint 是 `status+room+scrolldir` 三字段（1074/1180）；工具与本节是 `status+room`
两字段（1075/1180）。差 1 帧是字段集不同，不是算法差异。

**地址解码对照实验**（证明 `$0300` 的 range 索引确实是 `2*0x200`，不是凭结果倒推）：

| 解码方式 | 最优 k | 一致率 |
|---|---|---|
| status @ `0100-07ff[2*0x200]`（=`$0300`） | +1 | **1096/1180 = .928814** |
| status @ `0100-07ff[2*0x300]`（=`$0400`，错址对照） | +2 | 10/1179 = .008482 |
| status @ `0100-07ff[2*0x100]`（=`$0200`，错址对照） | -2 | 0/1178 = .000000 |
| room @ `0000-00ff[2*0x5A]`（=`$005A`） | +1 | **1158/1180 = .981356** |
| room @ `0000-00ff[2*0x15A]`（越界） | — | 我的脚本取到空串报错（该 range 只有 256 B；工具侧另有 `expected_len` 校验） |

⇒ 错一个字节地址一致率立刻塌到 0.8%/0%，解码正确性被证伪式确认；`offsets` 已回到状态面，
与我的独立测量逐位相同。**上轮 P0 已解决。**

## R2-2 `latch = state + 1` 自校验 —— **PASS（附 finding：会把观测当律法，且会误杀）**

**不是循环论证**：`state_offset` 来自 `$0300`/`$005A`，`latch_offset` 来自 `$0014`，两组字节
不相交，各自独立拟合后才比较（align_legs.py:516-534）；没有用 `state+1` 去构造 `latch`。
这是"两个独立测量的相对相位一致性检查"。

**但它的检验力被高估**：它只验证**相对相位**，对**共同整体平移**不敏感——若两腿轨迹整体错 1 帧，
状态 offset 与锁存 offset 同时 -1，gap 仍是 +1，断言照样通过。所以它不能证明绝对帧映射正确，
只能抓"把锁存偏移当帧偏移"这一类错误（正是上轮 P0）。绝对锚仍然是状态面本身。

**误杀实证**（`/tmp/make_gap_synth.py`）：80 帧合成，状态 offset=+1、锁存 offset=+1（gap=0，
其余完全合法、状态面 100% 可解）：

```
FAIL biz->mesen: latch-state gap anomaly on biz: latch_offset=+1 != state_offset+1=+2
FAIL rust->mesen: latch-state gap anomaly on rust: latch_offset=+1 != state_offset+1=+2
VERDICT: latch-state-gap-anomaly
EXIT=1
```

输出 JSON：`offsets={"biz_to_mesen":null,"rust_to_mesen":null}`、`per_field={}`、
`agree_rate={"biz":null,"rust":null}`。**一个完全可解的状态面偏移被连带丢弃**——这就是"把观测
当律法"的后果。诚然它 fail-closed 不产生错误数字，但把主交付物清零。

**同族耦合**：`_extract_state` 对 mesen **无条件**调用 `_extract_latch`（align_legs.py:349），
所以缺 `Joy1Latch` 的轨迹连状态 offset 都出不来。用本仓库归档的旧轨迹实测：

```
$ ... --mesen analysis/traces/.archive/_probe-axis-20261118-140420/_probe-axis.frames.jsonl ...
ValueError: ...:1: mesen row missing w['Joy1Latch'] (Mesen2 leg must watch $0014; ...)
PYEXIT=1
```

可它的 `0000-00ff`/`0100-07ff` 本来就在，工具自己的 `_mesen_state_byte`（align_legs.py:290-320）
就是从 range 解状态字节的。**状态面不该被锁存面绑架。**

判定：本轮真实轨迹 gap 均为 +1、交付物正确，故不判 FAIL；但设计风险成立，列 finding（中）。
修复见 R2-8-1/2。

## R2-3 口径是否仍有偷换 —— **PASS（两处低风险残余）**

- **双计数已落地**：`anchors.change_frames=97`、`change_runs=13`、
  `by_leg={biz:{97,13}, rust:{96,13}}`；`per_field` 每个字段也各自带 `change_frames/change_runs`。
  我独立复算：biz 97/13、rust 96/13，与 JSON 一致。
- **`fit_window [20,1199]` 与 `effective_end 1200` 自洽**：a 侧截到 `effective_end-1`，
  保证 b 至少能提供 `+1` 帧（align_legs.py:674-683 有注释说明）；`window=[20,1200]` 是可比范围。
  实测 biz 在 k=+1 下参与者 1180、k=+2 下 1179，与代码截窗行为吻合。
- **tie → ambiguous 仍在**：`_fit`（align_legs.py:196-197）仍抛 `ambiguous offset`，测试覆盖。
- 残余 1（低）：状态面多解时 `_field_resolution` 抛 `AlignmentError(..., "field-disagreement")`
  （align_legs.py:466-469），把"多解"和"字段分歧"混成一个 verdict；`_fit` 那边却是
  `ambiguous offset`。语义应分开。
- 残余 2（低）：`_allframe_score` 按**原始 matches** 取最大（align_legs.py:174-176），
  而 participants 随 offset 变（+1 是 1180、+2 是 1179），不是一致率或固定分母。本轮按一致率
  复算最优不变（我脚本即按率选，结果相同），但小窗口/边缘 case 可能翻盘。

## R2-4 实施方两条"更正" —— **① 成立；② 半成立（我确有笔误，但不在 mesen）**

**① 残差 10 vs 7：成立。** 我上轮的 7 是"**变化帧拟合口径**"（97 个参与者里的失配）；
工具 JSON 的 10 是"**全帧口径**"，与 `latch_agree_rate 1169/1179` 自洽。我独立复算：

- 全帧失配 10：`[661,663,665,667,669,671,672,673,1132,1133]`
- 其中变化帧失配 7：`[661,663,665,667,669,671,672]`；多出 `673/1132/1133`
- `1132/1133` 对应我上轮已记录的 p1 9→1 锁存滞后 2 帧（f=1132 输入已变、锁存到 1134 才跟）

⇒ 我上轮"残差全部连成 661..672 一段"**只对变化帧口径成立**；全帧口径下 673/1132/1133 在段外。
表述缺分母限定，下方"第一轮更正"已标注。

**② `2*0x300` 笔误：对了一半。** 复核档 §8-F1 的 **mesen** 公式原文是
`w["0100-07ff"][2*(0x300-0x100):+2]`（= `2*0x200`，**正确**，与实现 `STATUS_ADDR - IRAM_BASE`
一致）。真正的笔误在同一段的 **rust 分支**：我写成 `zp[2*0x300:+2]`、`zp[2*0x106:+2]`——
`$0300/$0106` 不在 zp（`$0000-$00FF`），应为 `iram[2*(addr-0x100):+2]`。实施方把位置指到了
mesen，但**实施代码本身两处都对**（mesen 用 `0x200`、rust 用 `iram`/`zp` 分窗口），没有被我的
笔误带偏。⇒ 更正"有笔误"成立，更正"是 mesen 公式"不成立。

## R2-5 文档改动 —— **PASS**

- `docs/spec/oracle-verification-chain.md:27` 规则 5 已从经验句升级为**实测恒等式**
  `latch_offset == state_offset + 1`，列出四组配对，并**登记事故**（Task 1 首次交付把 +2/+1
  当帧偏移、被复核 reject、代价 biz +8 / rust +53 失配帧）。
- 同文件新增 **§3.1.1 帧映射规约**：state 全帧一致数、`offsets` vs `latch_offsets` 分工、
  双计数、`change_runs<3` 不可对齐、事故登记。
- `docs/superpowers/plans/2026-11-18-three-leg-oracle-alignment.md`：
  `:7` Architecture、`:16` 地址具名常量、`:30` Review Focus、`:43`（删掉错误的 gitignored）、
  `:48/:49/:52` Interfaces、`:80` Step 3、`:90` Step 5、`:171/:175` R6 —— 全部统一为
  "状态面求 offsets + 锁存面单列 corroboration"；Expected 已是 `+1/0` 与 `+2/+1`。
- grep 全文 `锁存/$0014/changes/gitignore` 未发现残留矛盾。

## R2-6 测试是否被削弱 —— **PASS**

`python3 -m pytest tools/tests/test_align_legs.py -q` → **13 passed in 0.02s**；
`grep -nE "skip|xfail|pragma|environ|getenv|monkeypatch|DSH_"` → **无命中**。
13 = 5 旧（计划原文用例 + 多解 + 两个 load_latch）+ 8 新。抽查 3 条新用例：

- `test_resolve_leg_state_offset_and_latch_gap`（合成 4 次脉冲，state=1/latch=2）：断言
  `state_offset==1`、`latch_offset==2`、state `participants==len(a)`（全帧 60）、
  `change_frames==8`/`change_runs==4`、latch `matches==len(a)`、`fit_participants==8`
  ⇒ **真在考"状态面全帧、锁存面变化帧拟合"两套口径**，不是自证。
- `test_resolve_leg_rejects_field_disagreement`（room_offset=0 vs status=1）⇒ 真考字段分歧拒绝。
- `test_resolve_leg_rejects_latch_state_gap`（latch_offset=1=state）⇒ 真考 gap 断言；
  但它同时把"gap=0 的合法场景"钉死为错误，与 R2-2 的 finding 同源。
- 另有 3 条状态解码用例（mesen range、mesen 命名键、rust zp/iram）与 2 条缺键报错用例，
  都是真行为；多解 fixture 未被改动（仍是我上轮独立复算过的 `{0,+4}` 并列）。

## R2-7 总 verdict：`pass-with-findings`

| # | 检查点 | 判定 |
|---|---|---|
| 1 | 主偏移改为状态面 | **PASS**（独立复算逐位一致；错址对照证伪式确认解码） |
| 2 | `latch=state+1` 自校验 | PASS + finding（非循环，但把观测当律法、会误杀状态偏移） |
| 3 | 口径（双计数/fit_window/ambiguous） | PASS（2 处低风险残余） |
| 4 | 对复核数据的两条更正 | ① 成立；② 半成立（笔误真实但在 rust，不在 mesen） |
| 5 | 文档与实测一致 | **PASS** |
| 6 | 测试未削弱 | **PASS** |

P0 已解决：`offsets` 与我的独立测量一致，交付物可直接喂 Task 2。剩余是一类**设计耦合**
（锁存面异常/缺失会连带清空状态面结论），不产生错误数字，但会在别的录像上失败。

## R2-8 修复清单（可直接执行）

1. `tools/movie_ref/align_legs.py:534` `assert_latch_gap(...)` 改为**非致命**：捕获
   `AlignmentError("latch-state-gap-anomaly")` 后保留 `resolve_leg` 已算出的 state 结果，
   令 `latch_offset=None`、`latch_gap_anomaly=true`；`main` 侧仍可写 `verdict=`
   `latch-state-gap-anomaly` 并 `return 1`，但 `offsets`/`per_field` 必须是状态面的真实值
   （当前实现把它们写成 null，见 R2-2 合成实验）。
2. `tools/movie_ref/align_legs.py:349` `_extract_state` 不再无条件 `_extract_latch`：
   mesen 行缺 `Joy1Latch` 时 `latch=None`，只在计算 latch 的路径 fail-loud。否则归档轨迹
   （range 齐全、只缺 latch watch）无法产出状态偏移（R2-2 实测 `ValueError` + exit 1）。
3. `tools/movie_ref/align_legs.py:466-469` 状态面多解的 verdict 从 `field-disagreement`
   改为 `ambiguous-offset`（或新增子类），与 `_fit` 的 `ambiguous offset` 文案统一。
4. `tools/movie_ref/align_legs.py:174-176` `_allframe_score` 改按**一致率**（或固定
   participants 交集）排序，避免分母随 offset 变化；否则小窗口下偏移可能因参与者多寡翻盘。
5. `data/status/leg_offsets.json` 增加分母说明（或用两组字段分开）：
   `residuals.biz/rust` 是**全帧**（10/1179、6/1166），`anchors.change_frames` 是**变化帧**（97/96），
   `per_field.*.matches/participants` 是**全帧**。三者混在同一层 JSON 容易被误读成同一分母。
6. 本档更正（见下）：rust 公式 `zp[2*0x300]` → `iram[2*(0x300-0x100)]`；
   "残差 7 帧连续"限定为变化帧口径。

## 第一轮更正（2026-11-18，第二轮复核发现）

> 【2026-11-18 更正】§8-F1 的 rust 分支原写 `$0300 → zp[2*0x300:+2]`、
> `$0106 → zp[2*0x106:+2]` 有误：`$0300/$0106` 属 IRAM（`$0100-$07FF`），不在 zp；
> 正确写法是 `iram[2*(0x300-0x100):+2]`、`iram[2*(0x106-0x100):+2]`。mesen 分支
> `2*(0x300-0x100)` 原本就是对的。实施方代码两处均正确，未被此笔误影响。
>
> 【2026-11-18 更正】§5 "7 个残差全部连成 f=661..672" 应限定为**变化帧拟合口径**
> （97 个参与者）；按**全帧**口径，在 latch offset=+2 下 biz 有 **10** 个失配帧
> `[661,663,665,667,669,671,672,673,1132,1133]`（1132/1133 即已知 p1→latch 2 帧滞后）。
> 两种口径都不假，但必须写明分母。

## 附录 B：第二轮命令与关键输出

1. `git log --oneline -3` → HEAD `d3440ff9`，父 `ca2fba07`（第一轮复核档入库）。
2. `git show --stat d3440ff9` → 5 文件：json 256+、spec 19、plan 23、align_legs 672+、test 133+。
3. `python3 -m pytest tools/tests/test_align_legs.py -q` → `13 passed in 0.02s`。
4. `python3 /tmp/verify7_r2.py` → R2-1 全表（biz/rust 状态与锁存逐 k、change_frames/runs、
   全帧与变化帧两套残差）。
5. CLI 复跑 + `diff` → IDENTICAL；stderr 两行 OK 见 R2-1。
6. 错址对照实验（§R2-1 表）：`$0400` 10/1179、`$0200` 0/1178。
7. `python3 /tmp/make_gap_synth.py` + CLI → `verdict=latch-state-gap-anomaly`、
   `offsets` 全 null、`per_field={}`、exit 1（R2-2 误杀实证）。
8. 归档无 latch 轨迹跑 CLI → `ValueError: ... mesen row missing w['Joy1Latch']`、exit 1、
   无 JSON 产出（R2-2 耦合实证）。
9. 文档 diff：`git show d3440ff9 -- docs/spec/oracle-verification-chain.md
   docs/superpowers/plans/2026-11-18-three-leg-oracle-alignment.md`（R2-5 各条）。
10. `git status --porcelain` → 本档 `M`（追加小节）+ 旧 docs 改动 + 未跟踪 `compare_cores.py`；
    未触碰 `tools/` `crates/` `data/` `gates/`。

---

**第三轮终审：pass，Task 1 完成**（2026-11-18，被复核 commit `da8a8fe6`）。

- 误杀实验两条均通过：gap=0 合成 → `verdict=latch-state-gap-anomaly`、`offsets={biz:1,rust:1}` 保留、
  `per_field` 两字段保留、`latch_offsets=null`、exit 1；无 `Joy1Latch` 归档轨迹 →
  `verdict=latch-unavailable`、`offsets={biz:1,rust:0}`、`latch_offsets=null`、exit 1。
- 交付物无旧值漂移：新 JSON 与我第二轮自算 `/tmp/leg_offsets_r2.json` 逐键 diff → **NONE**；
  仅新增 `denominators/latch_errors/latch_gap_anomaly/latch_missing`。
- 18 passed，无 skip/xfail/环境开关；抽查 `test_resolve_leg_keeps_state_on_latch_gap`、
  `test_cli_keeps_state_when_mesen_latch_missing`、`test_allframe_score_ranks_by_agreement_rate` 均真考行为。
- 判据未放松：`insufficient anchor changes/matches`、`ambiguous offset` 与 `ambiguous-offset`
  verdict 均保留，`MIN_ANCHOR_CHANGES=3`、`DEFAULT_MAX_LAG=4`、`--start>=20` 未动。
