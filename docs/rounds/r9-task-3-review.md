# r9 Task 3 复核：spec R6/R7 + §3.4（HEAD `532a1647`，含 `0ee83609`）

复核员立场：证伪（非实施方）。对象 = `docs/spec/oracle-verification-chain.md` 的两个 docs commit；
但 §3.4 引用的产物由 `9edceb65 fix(tools): 标注 W 诊断量并写入敏感性实测` 生成，实际证据链是
`7377a754 → e22aaeef → 9edceb65 → 0ee83609 → 532a1647`（`git log --oneline`）。

约束遵守：只写本档与 `/tmp` 脚本；未跑 `cargo test` / `gates/cli.py check` / L4；未改 `tools/`、`data/`。

```
git log --oneline -5
532a1647 docs(spec): §3.3 原地更正——f943..952 经序列匹配改判加载窗瞬态
0ee83609 docs(spec): 补 R6 因果锚帧偏移与 R7 加载窗列账规则，并新增 §3.4 三分类实测
03818ed9 docs(rounds): r9 Task 2 两轮复核档案（启发式 -> 三分类序列匹配）
9edceb65 fix(tools): 标注 W 诊断量并写入敏感性实测
git show --stat 0ee83609  -> docs/spec/oracle-verification-chain.md | 33 +++
git show --stat 532a1647  -> docs/spec/oracle-verification-chain.md |  4 +
git status --porcelain    -> 空（工作树干净）
```

---

## 1. 每个数字能否追到产物 —— **PASS**

| spec 断言 | 产物出处 | 我的独立复算 |
|---|---|---|
| 状态面 `+1`/`0` | `leg_offsets.json.offsets` = `{biz_to_mesen:1, rust_to_mesen:0}` | 一致 |
| 锁存面 `+2`/`+1` | `leg_offsets.json.latch_offsets` = `{1:2, 0:1}` | 一致 |
| 锚残差 biz `10/1179` | `residuals.biz = {latch_mismatch:10, latch_participants:1179}` | 自写脚本重算：participants **1179**、residual **10** = `[661,663,665,667,669,671,672,673,1132,1133]`，逐帧相同 |
| 锚残差 rust `6/1166` | `residuals.rust = {6, 1166}` | 重算 **6** = `[662,664,666,668,670,672]`，且 1166 一致 |
| 残差"连在 f661..673 与 f1132..1133" | 同上 | biz 10 个全在两簇内；rust 6 个全在 662..672（属第一簇）。措辞"全部"成立（每点落入其一），但 1132/1133 仅 biz 一侧 |
| R7 `1180 / 33 / 14 / 1147` | `leg_episodes.json.denominators` = `total_frames:1180, excluded.load:33, missing:14, missing_within_load:14, union:33, in_denominator:1147` | 一致；`load_frames` 数组恰 **33** 项、`missing_frames` 恰 **14** 项 |
| R7 `by_leg biz 1147 / rust 1155` | `denominators.by_leg` = biz.in 1147、rust.in 1155 | 一致 |
| §3.4 `43 = 6/31/6/0` | `summary.by_class` = phase 6 / transient 31 / value 6 / undecidable 0，`total_episodes 43` | 一致 |
| "5/6 value 在加载窗" | `episodes` 中 6 个 value：675..678、676..678、944..953、948..951、1131..1136 均 `during_load=true`；1031..1081 为 `false` | 一致（5/6） |
| "唯一干净持续值差 biz f1031..1081（51 帧）" | value 且 `during_load=false` 的段只有该段；`end-start+1 = 51` | 一致（轴标注问题见 §3-C） |
| W 敏感性 `6/18/37`、`31/15/0`、`0/4/0` | `summary.phase_window.sensitivity` | 我直接调 `classify(..., window=W)` 复跑：W=2 6/31/6/0；W=3 18/15/6/4；W=4 37/0/6/0，与产物一致 |
| `p1↔latch 0 帧 96 次、±1 帧 1 次` | `phase_window.lag_histogram` = `p1_to_latch {0:96,-1:1}`、`latch_to_p1 {0:96,1:1}`、`max_abs:1` | 产物确为如此；但该口径是"最近邻"，见 §5 |
| `constants_ram.asm:589` `#$FF=no room requested` | `refs/m1disasm/SRC/constants_ram.asm:589` | 逐字命中：`RoomNumber db ;$5A ;Room number currently being loaded. #$FF=no room requested.` |

唯一对不上的数字是 R6 的"**滞后 2 帧**"：产物里最大的 `max_abs=1`，没有 2。我按因果定义独立复算
（每个 p1 变化 → 其后第一个 latch 变化）得 `{0:96, 2:1}`，max=**2**；实测触发点是 p1 f1132 变化 →
latch f1134 变化。**事实成立，但产物证不出来**（口径不同），详见 §5。

---

## 2. 规则是否可机械执行 —— **R6 FAIL / R7 PASS**

**R6 FAIL**（spec:33-39）——三处：

1. "帧偏移必须由因果锚反解，不得用一致率打分挑最优"与**产生这些数字的实现相反**：
   `align_legs.py:161-193` `_allframe_score` 明确"排序按 `agree_rate`"，`_field_resolution`（567-594）
   用它选出 status/room 的 offset。R6 引用的 `+1/0` 正是这个"一致率打分"算出来的。
2. 照字面执行只能得到锁存偏移 `+2/+1`——正是 R5（spec:27-31）明令**不得**当帧偏移的值。
   规则文本把读者引回 Task 1 首次交付被 reject 的那个缺陷。
3. 可机械检查的只有"唯一解 / `change_runs>=3` / fail-loud"这半句；`_fit` 与 `_field_resolution`
   确实实现了这三点（多解抛 `ambiguous-offset`，不足抛 `insufficient`），但这三点管不到"偏移从哪来"。

**R7 PASS**：三数（总帧 / 进分母 / 排除帧）与**逐帧列表**都在产物里——
`denominators.excluded.load_frames`（33 项）、`missing_frames`（14 项）、`reasons`（3 条按类别的原因）。
不写新工具即可核对，"禁止只报一个分母数字"是可检验的。

---

## 3. 与既有条款冲突 —— **FAIL**（3 处）

**A. R5 ↔ R6**：R5 说 `$0014` 不得当帧轴判据、跨腿 `offsets` 必须取状态面；R6 说"帧偏移必须由因果锚
反解"并把锁存偏移 `+2/+1` 与状态偏移 `+1/0` 并列。两句相邻，读者取哪个取决于是否从头读——这是规则
层面的歧义，不是措辞偏好。

**B. §3.3 结论句未被更正覆盖**：spec:116 的更正批注写"`f943..952` 是加载窗……不再计为干净行为分叉"，
但 spec:120 结论紧接着仍写"实质分歧段是 **f943..952** 与 **f1030..1080** 两段"。批注只声明覆盖
"表内『真差』二字"，管不到结论句——两行直接冲突。

**C. 轴混用**：`classify_episodes.py:149-151` 的循环是 `for g in range(start,end+1)`、取
`leg_row = legs[leg].get(g - offset)`；biz `offset=1` ⇒ episode 的 `g` 在 **mesen 轴**，biz 侧读的是
`biz[g-1]`。实测 `biz[1030]=4`、`mesen[1031]=3`，所以 §3.4 的 "biz f1031..1081" 对应的是
**biz 行 1030..1080**——恰是 §3.3 的 f1030..1080。文档把 mesen 轴区间冠以 "biz f" 前缀，正是 R4
（跨腿帧号不直接比）警告的混轴；`532a1647` 的 f943..952 同源（episode "biz 944..953" = biz 行 943..952）。

---

## 4. §3.4 是否越界 —— **FAIL**

spec:137 "另外那段 `f943..952` **被证认为**加载窗 room/status 瞬态" 强于证据：

- `during_load` 的语义只是"段内任一帧、任一侧 `$005A != $FF`"（`_during_load_frames`），它是
  **排除窗标记**，不是成因证明。它可以说明"该段落在 R7 的排除窗内"，不能说明"差异由加载造成"。
- 反证方向：我在 Task 2 实测 biz room 出现 242、mesen 全程无 242（±2 与整段窗都不存在）；
  biz 1131..1136 期间 mesen room 恒 255。这更像"加载动画序列本身不同"，而不是"瞬态噪声"——
  也就是说，它可能是**真实差异**，只是落在不可比较窗内。
- 因此正确措辞是"落在加载窗内、按 R7 不计入归因分母；成因未定"，而不是"被证认为瞬态"。
  "不再计为**干净**行为分叉"这半句是可以的（"干净"= 在分母内），前半句不行。

---

## 5. 有没有把诊断量当律法 —— **FAIL**

- spec:138 "`max_abs=1` ⇒ 上界 2" 不是推导，是加余量：同样口径下 max_abs=1 应得 W=1。
- 更深一层：产物的 `max_abs` 是**最近邻**统计（`classify_episodes.py:768-786`：
  `nearest = min(latch_changes, key=lambda g: (abs(g-frame), g))`）。对 p1 f1132 的变化，最近的 latch
  变化是 f1131（-1），于是记成 1；真正的因果事件是 p1 f1132 → latch **f1134**（**2 帧**）。
  我按"每个 p1 变化 → 其后第一个 latch 变化"复算：
  `{0: 96, 2: 1}`，max_abs = **2**。
- 结论：W=2 这个**值**是对的（正好等于真实前向滞后上界），R6 的"滞后 2 帧"也是对的；但
  §3.4 用 max_abs=1 推 2 是**巧合**，推导链是断的，且与 R6 自相矛盾（同一现象，文档里 1 和 2 并存）。
- 敏感性表本身写得对（"W=4 会让 transient 整体消失，故不采用"），但建议补一句"W 由因果滞后上界
  固定，敏感性只作诊断，不得按类别计数回选 W"——否则 6/18/37 仍会被读成"W 可选"。
- q95 那句（`nearest_equal_distance.q95=4 只是诊断量，不参与 W 推导`）写得对，保留。

---

## 6. prose 门禁式废话 —— **PASS**

```
git show 0ee83609 532a1647 -- docs/spec/oracle-verification-chain.md | grep "^+" \
  | grep -nE "严谨|务必|严格|禁止伪造|确保|严禁|认真|规范|必须严谨"   -> 无命中
```
唯一偏"不可检验"的句子是 R7 的"此时三腿的加载链进度不同，比较无意义"——它是**理由**不是规则，
但没有证据指针（未测过三腿加载链进度差），建议标注"（推测，未单测）"或补一条 trace 指针。

---

## 总 verdict：**reject（文本层）**

- 数据层无需改动：偏移、残差、分母、43 段计数、敏感性全部逐项对上产物，我独立重算一致。
- 规则层不能接受：R6 作为"律法"与其自身出处相反（照字面执行会回到被否决的 `+2/+1`）；§3.3 结论句
  与更正批注直接冲突；§3.4 有强于证据的因果断言；W 的推导链断（数字碰巧正确）。
- 这三处不改，Task 4 不能把 R6 / §3.3 结论 / §3.4 的"证认"当作判据引用。

## 修复清单（可直接执行）

1. **`docs/spec/oracle-verification-chain.md:33-39`** R6 拆成两条：
   - (a) 状态面：`status`/`room` 以**全帧一致率**求 offset，要求两字段共识且唯一（多解 fail-loud）——
     这是**唯一允许**用一致率的场合，明确写出来，避免与半句"不得用一致率"打架。
   - (b) 锁存面：`$0014` 变化帧唯一反解（`change_runs>=3`，多解/不足 fail-loud），只用于断言
     `latch_offset == state_offset + 1`；**禁止**把 `latch_offsets` 当帧偏移（引 R5）。
2. **`:120`** 结论句改为："实质分歧：`f1030..1080`（干净窗内唯一持续值差）；`f943..952` 落在加载窗
   （§3.4），不计入归因。"
3. **`:137`** "被证认为加载窗 room/status 瞬态" → "落在加载窗内（`during_load=true`），按 R7 不计入
   归因分母；该段差异成因未定，不得据此断言两核加载行为一致"。
4. **`:135`/`:137` 与 §3.3 表**：区间统一标注帧轴（episode 区间是 mesen 轴），"biz f1031..1081" 改为
   "biz 行 1030..1080 ≡ mesen 轴 f1031..1081"，避免 R4 混轴。
5. **`tools/movie_ref/classify_episodes.py:750-786`** `_lag_histogram` 增"前向"口径
   （p1 变化 → 其后第一个 latch 变化），重跑 `leg_episodes.json`；`:138` 改为
   "max 前向滞后 = 2 ⇒ W = 2（因果上界，不留额外余量）"，与 R6 的"滞后 2 帧"自洽。
6. **`:42`** R7 的"此时三腿的加载链进度不同，比较无意义"标"（推测，未单测）"或补 trace 指针。

---

本档为唯一新增文件（`?? docs/rounds/r9-task-3-review.md`，未 commit）；未触碰 `tools/` `data/` `crates/` `gates/`。

---

# 第二轮终审：`2c4487f6`（spec）+ `fc4e77cd`（工具/产物）

第二轮 4 项问卷全 PASS，总 verdict **pass**。以下证据均由我自跑，未 import 复核对象。

## 1. R6 双面口径与实现一致 —— PASS

- 状态面：`_allframe_score` + `_field_resolution`（`tools/movie_ref/align_legs.py:161-193` / `567-594`）
  按全帧一致率选唯一解、status/room 必须同 k 否则 `field-disagreement`，产出 `offsets {biz:+1, rust:0}`。
- 锁存面：`_fit` + `assert_latch_gap`，只做变化帧唯一反解并校验 `latch == state+1`（`+2/+1`），
  不写入 `offsets`；`latch_offsets=null` 时 offsets/per_field 仍保留（第三轮已验证）。
- 新 R6 文字与上述一一对应，R5 的矛盾已消除。

## 2. 我点名的其余 5 条全部落地 —— PASS

- §3.3 结论句已剔除 `f943..952`（改为"干净窗内实质分歧只剩 f1030..1080 status 一段"）。
- §3.4 改为"落在加载窗内…按 R7 不计入归因分母；成因未定（"是加载动画序列不同"只是未单测的假设）"。
- 新增"区间轴注意"：episode 记 Mesen2 轴、biz 取 `biz[g-1]`，故 mesen `f1031..1081` ≡ biz 行 `1030..1080`。
- W 推导改前向：`p1 变化 → 其后首个 latch 变化`，`0 帧 96 次 / 2 帧 1 次，max(forward)=2 ⇒ W=2`。
- R7 的"比较无意义"已标"（推测，未做单测）"，并补了"该窗内三腿 `$005A` 取值集合并不相同"的已证实部分。

## 3. "只改 provenance"成立 —— PASS

`git show 532a1647:data/status/leg_episodes.json` 与工作树逐项对比：

```
episodes identical: True (43/43)
by_class same: {phase:6, transient:31, value:6, undecidable:0}
by_leg same: {biz:34, rust:9}   by_leg_class same   during_load 8 -> 8
顶层 8 个键唯一差异: provenance
```

`phase_window` 实测：`value=2, upper_bound=2`；
`lag_histogram = {"forward":{"0":96,"2":1},"nearest":{"0":96,"-1":1},"latch_to_p1":{"0":96,"1":1},
"p1_changes":97,"latch_changes":97,"forward_skipped":0,"max_abs_forward":2,"max_abs_nearest":1}`，
`derivation="W = max(forward p1->latch lag) = 2; state transient displacement <=1; upper bound 2"`。
CLI 复跑 `/tmp/ep_r3.json` → `episodes/summary/full` 全 equal。

## 4. 未引入新矛盾 —— PASS

§3.3 标题"粗判 2 段真差，其中 1 段落在加载窗"+ 批注"表内真差二字请按 §3.4 读"+ 结论"只剩 f1030..1080"
三处口径一致（按 §3.6 保留历史主张、原地加更正，不算矛盾）；§3.4 标题"覆盖并修正 §3.3 的粗判"与新措辞不冲突。
`git diff 532a1647..HEAD --stat` 只 4 文件（`leg_episodes.json` 30 / spec 37 / `classify_episodes.py` 44 /
测试 38），**`data/status/leg_offsets.json` 未动**。

测试：`pytest tools/tests/test_align_legs.py tools/tests/test_classify_episodes.py -q` → **30 passed in 0.19s**
（18+12）。新测试 `test_lag_histogram_forward_not_nearest`（`tools/tests/test_classify_episodes.py:199`）
用 p1 f10 变、latch f9/f12 变构造 `forward={2:1}`/`nearest={-1:1}`，并断言 W 取自 forward 的 max
（旧实现 KeyError: forward），真能卡回归。

两条非阻塞 nit（不影响 pass）：①R6 残差表述"集中在 f661..673 与 f1132..1133"对 rust 不精确
（rust 6 个残差止于 672，1132/1133 仅 biz），建议补"（1132..1133 仅 biz）"；②§3.3 批注仍写
"room/status 加载瞬态"，与 §3.4"成因未定"有轻微张力，建议统一为"落在加载窗内"。

**第二轮终审：pass，Task 3 完成**（2026-11-18，被复核 commit `2c4487f6`+`fc4e77cd`）
