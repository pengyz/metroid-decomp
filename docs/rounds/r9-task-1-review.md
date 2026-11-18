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
