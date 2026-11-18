# Task 2 对抗复核：三方分叉 episode 分类器（commit `7377a754`）

- 复核对象：`7377a754 feat(tools): 实现三方分叉 episode 分类器`
  - 新增 `tools/movie_ref/classify_episodes.py`（567 行）、`tools/tests/test_classify_episodes.py`（8 条）、
    `data/status/leg_episodes.json`（1994 行）
  - 扩展 `tools/movie_ref/align_legs.py` 的 `load_state` 支持 5 字段（`$0300/$005A/$0106/$0107/$0049`）
- 判据来源：`docs/superpowers/plans/2026-11-18-three-leg-oracle-alignment.md` Task 2 小节
  + lead 补充口径（`class="phase"` iff 段内**每一帧**的值都能在对面 **±2 帧**内找到同值；
  `during_load` = 段内任一帧 `$005A != 0xFF`；偏移只从 `data/status/leg_offsets.json` 读）
- 立场：证伪。不复用 `classify_episodes.py` 的代码做判定，全部用 `/tmp` 自写解码重算。
- 未跑：`cargo test`、`gates/cli.py check`、L4（按任务约束）。
- 临时脚本：`/tmp/t2_verify.py`（重算 episode/分母/两套判据）、`/tmp/t2_detail.py`（逐段证据/输入相位）、
  `/tmp/leg_offsets_biz0.json`（偏移篡改副本）。

---

## 1. 独立重算：43 段（33 phase + 10 value）—— **PASS**

自己从原始轨迹解码（`biz` 读行内字段；`mesen` 从 `0000-00ff/0100-07ff` 取 `$005A/$0300/$0106/$0107/$0049`；
`rust` 从 `zp/iram` 取同样地址），按 `a[f] == b[f+offset]`（biz +1、rust 0）在 Mesen2 轴上聚合：

```
$ python3 /tmp/t2_verify.py
leg_offsets window/fit/effective: [20, 1200] [20, 1199] 1200 offsets {'biz': 1, 'rust': 0}
rows: {'biz': 28408, 'mesen': 1200, 'rust': 1179}
MINE episodes: 43 kind: Counter({'phase': 33, 'value': 10}) load: Counter({False: 35, True: 8})
COMMIT episodes: 43 kind: Counter({'phase': 33, 'value': 10})
only-mine: []       only-commit: []
FIELD/KIND DIFFS: NONE
```

逐段比对 `(leg,start,end,fields,kind,during_load)`：**零差异**。产物可复现：

```
$ python3 tools/movie_ref/classify_episodes.py ... --json-out /tmp/leg_episodes_check.json
EPISODES total=43 phase=33 value=10 during_load=8
DENOMINATORS total=1180 in=1147 load=33 missing=14 union=33
$ diff -q data/status/leg_episodes.json /tmp/leg_episodes_check.json
leg_episodes IDENTICAL
```

关于「43 vs 既有 33 段」：口径单位不同。把 43 个 per-leg 段按帧区间跨腿合并 → **34 段**
（`/tmp/t2_verify.py` 合并输出；与 §3.3 的 33 段差异来自旧文按单帧/跨腿混合列举）。
所以 43 不是回归，是「per-leg episode」这一更细的计数单位。

## 2. 逐帧 ±2 vs 段窗 [start-2,end+2] —— **PASS（附 finding）**

同一份数据分别用两套判据重算：

```
seg-window kind counts: Counter({'phase': 33, 'value': 10})
FLIPS frame->seg: []
```

**段窗判据不会把任何一段洗成 phase，两者在真实数据上结果完全相同。**
原因（逐段核过）：8 个争议段的失配值在**整个段窗**里也不存在——
`242` 在 mesen f673..680 的 room 序列里根本没有（240,241,**243**,244），
rust 的两帧段失配值是 `0`/`4`，mesen 段窗里只有 `1,1,1,4,1`/`1,1,1,0,1`。

哪条判据更符合语义：**逐帧 ±2 应是正解**，但理由不是「更严」在实际数据上生效（它没生效），
而是段窗在大段上会失真：一个 51 帧的持续差段，只要段内某处偶然出现一次该值，整段就被洗成 phase。
反证 fixture（我构造，非仓库测试）：rust f20..42 恒为 8、mesen 仅在 f43 出现一次 8 →
逐帧判 **value**、段窗判 **phase**（`/tmp` 复算已跑）。

⇒ finding：`classify_episodes.py:8-9` 的 docstring 说实现「取更严的逐帧口径」偏离计划，
但计划 Step 3 的段窗文字仍是 `[start-2, end+2]`；Task 3 落 spec 时必须二选一并写死，
否则 spec 与代码各说一套（实际后果：本数据无差异，换数据会有差异）。

## 3. 4 个 rust status 两帧段：**PASS（附 finding：是 1 帧瞬态，不是「值本身不同」）**

三方完整序列（我自算，`mesen=g`、`biz=g-1`、`rust=g`；`SamusStatus`：0=Stand 1=Run 3=Roll 4=PointUp）：

```
g1106 m=1 b=1 r=1 | g1107 m=1 b=1 r=1 | g1108 m=1 b=1 r=0 | g1109 m=4 b=1 r=1 | g1110 m=1 b=1 r=1
g1121 m=1 b=1 r=1 | g1122 m=1 b=1 r=1 | g1123 m=1 b=1 r=4 | g1124 m=0 b=1 r=1 | g1125 m=1 b=1 r=1
g1140 m=1 b=1 r=1 | g1141 m=1 b=1 r=1 | g1142 m=1 b=1 r=0 | g1143 m=4 b=1 r=1 | g1144 m=1 b=1 r=1
g1155 m=1 b=1 r=1 | g1156 m=1 b=1 r=1 | g1157 m=1 b=1 r=4 | g1158 m=0 b=1 r=1 | g1159 m=1 b=1 r=1
```

该区间 1100..1165 的全部 status 瞬态（非 1）：mesen 有 18 个、rust 有 18 个，
其中 **14 个位置两腿完全相同**（帧号+值都一致），只有这 4 处错开 1 帧且值互补（0↔4）：

```
mesen: 1101:0 1105:4 1109:4 1112:0 1116:0 1120:0 1124:0 1127:4 1131:4 1135:0 1139:4 1143:4 1146:0 1150:0 1154:0 1158:0 1161:4 1165:4
rust : 1101:0 1105:4 1108:0 1112:0 1116:0 1120:0 1123:4 1127:4 1131:4 1135:0 1139:4 1142:0 1146:0 1150:0 1154:0 1157:4 1161:4 1165:4
```

裁定：**不是纯相位**（纯 ±1 相位会把 18 个瞬态**全部**移位，实际只有 4 个移位），
**也不是持续性值分歧**（每段 1 帧，前后都回到 1）——是 1 帧瞬态的采样错位：
同一瞬态事件在两腿各被采到不同帧、且采到的成员值互补（0 与 4）。
biz 在这整段几乎全程 `status=1`（biz f1104..1161 的 `p1` 逐帧在 `0x01`/`0x09` 间交替，
即方向/按键位每帧翻转，见下），biz 记录相位看不到这些瞬态，**约束不了**该争议。

```
$ biz f1104..1112: p1=0x01,0x09,0x01,0x09,... status=1（全段）
```

⇒ finding：机械判据下它们确实是 `value`（值 0/4 不在对面 ±2），但把它读成「真差」会与
`docs/spec/oracle-verification-chain.md` §3.3 自己定的纪律冲突（「1 帧抖动段一律不计入缺陷，
也不得用来支撑『核不可用』的结论」）。建议给 `transient` 标记（见 §12 修复清单）。

> 【2026-11-18 更正（第二轮复核 e22aaeef 后）】上段只点名了 rust 侧 4 段；
> 「transient 应该有几段」的完整答案不是 4 而是 **31**。同一规则下 biz 侧还有 **27 个单帧 status 尖峰**
> （f1093..f1199，biz 恒 1=Run，mesen 在该帧恰有一个 0/4 尖峰、两侧邻帧为 1），
> 它们是 4 段 rust 段的镜像（本腿正常、对面多采一个瞬态）。旧实现把这些全判成 `phase`
> （因为 biz 的值 1 在对面 ±2 里必然出现）——正是 §2 指出的单向采样错位被吞掉的洞。
> 结论：31 = rust 4（本腿尖峰）+ biz 27（对面尖峰），两者都满足「≤2 帧 + 对面恰 1 帧异值」，
> 按同一 `transient` 规则均成立；本档 §3 的「4 段」表述仅对 rust 范围成立，作为全局结论不完整。

## 4. 分母：1147 / 1155 / 33 / 14 —— **PASS（附 finding：显示会误导）**

独立复算（`/tmp/t2_verify.py`）：

```
per_leg (missing, load, in_denom): {'biz': (0, 33, 1147), 'rust': (14, 11, 1155)}
union: load=33 missing=14 union=33 in_denom=1147 total=1180
committed: in_denominator 1147; by_leg biz 1147/33/0, rust 1155/11/14
missing subset of load? True
```

三个数都对：`|load ∪ missing| = 33`，因为 14 个 missing 帧全部落在 33 个 load 帧之内
（`missing=[20..26,482,483,673,674,675,946,947]`）；rust 的 25 = 11 load + 14 missing。

finding（展示层面，不是算术错）：
- 顶层 `in_denominator=1147` 是「两腿共同可比帧」，而 episode 是 per-leg 的，rust 自己的分母是 1155；
  一个顶层数字同时暗示两种分母，读者容易把 1147 当成 rust 的分母（差 8 帧）。
- `excluded.load=33`、`excluded.missing=14`、`excluded.union=33` 三值并列，`union == load`，
  「missing 被并进 union」正是 lead 担心的读法：看上去只排除了 33 帧。
- 但代码与 JSON 都显式单列了 `missing_frames`，且 `in_denominator` 只用于报告、
  未参与任何比率计算（`grep in_denominator` 仅 304/312/544 三处），所以**没有偷分母**。

## 5. 偏移确实从 JSON 读 —— **PASS**

`grep -nE "biz_to_mesen|rust_to_mesen|offset\s*=\s*[0-9]"` 仅命中 docstring 与输出键，无硬编码偏移。
篡改实验（副本在 `/tmp`，未动仓库）：

```
$ python3 -c "改 /tmp/leg_offsets_biz0.json 的 offsets.biz_to_mesen: 1 -> 0"
$ python3 tools/movie_ref/classify_episodes.py ... --offsets /tmp/leg_offsets_biz0.json --json-out /tmp/episodes_biz0.json
EPISODES total=52 phase=42 value=10 during_load=11
DENOMINATORS total=1180 in=1146 load=34 missing=14 union=34
tampered offsets: {'mesen': 0, 'biz': 0, 'rust': 0}; 首段 biz 从 24 变 24/26/28/165/265/355
```

43 → 52、分母 1147 → 1146，输出随 JSON 变化 ⇒ 真读。

## 6. Task 1 回归 —— **PASS**

```
$ python3 -m pytest tools/tests/test_align_legs.py -q      -> 18 passed in 0.03s
$ python3 tools/movie_ref/align_legs.py ... --out /tmp/leg_offsets_t2check.json  -> EXIT=0
$ diff -q data/status/leg_offsets.json /tmp/leg_offsets_t2check.json
leg_offsets IDENTICAL (byte-for-byte)
```

扩展 `load_state` 没有让 Task 1 对齐协议漂移。

## 7. 测试未被放宽 —— **PASS（附 finding：一条测试名不副实）**

```
$ python3 -m pytest tools/tests/test_classify_episodes.py -q  -> 8 passed in 0.18s
$ grep -nE "skip|xfail|pragma|environ|getenv|monkeypatch|DSH_" tools/tests/test_classify_episodes.py
（无命中）
```

抽查真考行为：
- `test_persistent_value_difference_is_value`（:51）设 biz f24=4、mesen 全 3 → 断言 `class=="value"`，真考分界。
- `test_load_window_is_flagged`（:44）设 mesen f30 room=0x09 → 断言 `during_load`，真考加载窗标记。
- `test_denominators_report_load_and_missing_frames`（:74）断言
  `in_denominator == total - |load ∪ missing|` 且 `load_frames==[30]`，真考公式。

finding：`test_phase_value_must_be_near_the_mismatch_frame`（:63-71）**名不副实**——
它的 fixture 只有 1 帧 episode（rust f20=7），段窗 `[18,22]` 与逐帧窗相同，两种判据都判 `value`，
所以把实现换成段窗它**照样通过**，卡不住它声称要卡的回归。我构造的真区分 fixture
（rust f20..42=8、mesen 仅 f43=8）在逐帧下 value、段窗下 phase，已跑通验证。

`test_real_traces_smoke...`（:156）把 `offsets == {"biz":1,"rust":0}` 写死在测试里，属 ratchet 型断言，
Task 1 偏移若合法变更该测试会红——可接受，但需在 Task 3 注明它是「锁定当前偏移」而非「验证偏移正确」。

## 8. 追加 finding：产物 provenance 指向 `/tmp`，不可复现

`data/status/leg_episodes.json` 与 `data/status/leg_offsets.json` 的 `provenance.rust.path`
都是 `/tmp/rust-states-t2.jsonl`（24 MB、mtime 2026-11-18T10:40:25），且**没有 sha256**。
`git show 7377a754` 只入库了三份产物 + 代码，该轨迹不入库也不归档，别人拿到仓库无法重跑。
本轮我用的就是这份 `/tmp` 文件（sha256 `a635d45f3e06c5747334…`），下轮可能已被清理。

---

## 9. 总 verdict：**pass-with-findings**

判据实现与产物正确（43/33/10 我可逐段复现，偏移真读 JSON，Task 1 未回归，测试未放宽）；
不判 reject 的理由：没有错误数字、没有判据放松、没有分母偷换。
不给裸 pass 的理由：① 4 个 1 帧瞬态被标 `value`，与 spec §3.3 的「1 帧抖动不计缺陷」纪律冲突，
需要 `transient` 标记；② 段窗 vs 逐帧的口径分歧（计划 vs 代码）未裁定；③ 分母展示会误导；
④ 一条测试名不副实；⑤ provenance 指向 `/tmp` 不可复现。

## 10. 可直接执行的修复清单

1. `tools/movie_ref/classify_episodes.py:223-224` `_episode_class`：保持二分类不变，
   在 `_build_evidence` 增加 `transient` 布尔字段——条件：`len(frames) <= 2` 且**对面腿在该段窗内
   存在 1 帧不同值尖峰**（即 `any(v != leg_value for v in opposite)`）；`summary` 增
   `transient_count` 与 `value_episodes[].transient`，让 4 个 rust 两帧段可被机械过滤，不再被当作「真差」。
2. `classify_episodes.py:8-12` docstring + 计划 Task 2 Step 3：统一为「逐帧 ±2」，
   把计划里的 `[start-2, end+2]` 改成「每个失配帧 g 的 ±2」；Task 3 的 spec 照此写死。
   （保留段窗实现则相反，但必须二选一；建议逐帧，见 §2 反证 fixture。）
3. `classify_episodes.py:309-323` `_denominators`：`excluded` 增
   `"missing_within_load": len(missing_frames & load_frames)` 与
   `"common_in_denominator"` 别名；CLI（:540-548）打印改为
   `in=1147 (load 33 ∪ missing 14, overlap 14)` + 每个 leg 自己的 `in_denominator`，
   避免顶层 1147 被误读为 rust 的分母。
4. `tools/tests/test_classify_episodes.py:63-71`：把 fixture 换成真区分用例
   （rust f20..42=8、mesen 仅 f43=8，断言 `class=="value"`），否则该测试卡不住段窗回归。
5. `classify_episodes.py:437-446` `_add_provenance`：增 `sha256`（对三份输入算），
   并把 `/tmp/rust-states-t2.jsonl` 归档到 `analysis/traces/.archive/`（同 Task 1 对 probe-axis 的做法），
   否则 Task 2 产物不可复现。
6. `docs/spec/oracle-verification-chain.md` §3.3：把「33 段、只有 2 段真差」更新为
   「per-leg 43 段（33 phase / 10 value）；持续值分歧仍是 biz f944..953 与 biz f1031..1081；
   另 4 段 rust status 单帧瞬态 + 4 段加载窗 room 动画值，均不得用于归因」，
   并注明 33 与 43 的差异是计数单位（跨腿合并 = 34 段），不是新回归。

---

## 附录 A：本轮跑过的命令与关键输出

```
python3 /tmp/t2_verify.py                    -> 43 段、两套判据 0 翻转、分母 per_leg/union、三方 status 序列
python3 /tmp/t2_detail.py                    -> 10 个 value 段的逐帧失配证据（leg 值 vs mesen ±2 窗）
python3 tools/movie_ref/classify_episodes.py ... --json-out /tmp/leg_episodes_check.json
                                             -> EPISODES 43/33/10/8；diff IDENTICAL
python3 tools/movie_ref/classify_episodes.py ... --offsets /tmp/leg_offsets_biz0.json
                                             -> EPISODES 52；in=1146 load=34
python3 -m pytest tools/tests/test_align_legs.py -q        -> 18 passed
python3 -m pytest tools/tests/test_classify_episodes.py -q  -> 8 passed
python3 tools/movie_ref/align_legs.py ... --out /tmp/leg_offsets_t2check.json
                                             -> EXIT=0；与入库 leg_offsets.json 逐字节相同
grep -nE "skip|xfail|pragma|environ|getenv|monkeypatch|DSH_" tools/tests/test_classify_episodes.py -> 无
grep -n in_denominator tools/movie_ref/classify_episodes.py -> 304/312/544（仅报告，不参与比率）
sha256sum /tmp/rust-states-t2.jsonl -> a635d45f3e06c5747334…
```

未触碰 `tools/` `crates/` `data/` `gates/`；本档为唯一新增文件（未 commit）。

---

# 第二轮终审：`e22aaeef`（三分类 + 序列匹配）

方法：独立重写分类器 `/tmp/t2_r2.py`（自写解码 + 自写判据，不 import 工具），逐段比对 class/shift/边界；
其余为 CLI 复跑、测试、常量 diff、窗口敏感性实验。约束同前（未跑 gates/cargo/L4）。

**1. 序列匹配真实现 —— PASS**
我的独立重算：**43 段，`class` 与 `evidence.shift` 与入库 JSON 零差异**（`CLASS/SHIFT DIFFS: NONE`，
边界 `(leg,start,end)` 集合也完全相同）。
`biz 24..29 shift=-2`：4 个字段的整段序列逐帧等于 mesen 平移 −2（room leg `[0,0,9,9,255,255]`
== mesen[22..27]），且候选 k 只有 −2；`rust 1176..1177 shift=+1` 同理唯一。
若仍是旧「值集合」逻辑，这两段只会得到 phase 而没有 shift，且 27 个 biz 尖峰会被吞成 phase。

**2. 4 个 rust 段落 `transient` —— PASS**
1108..1109 / 1123..1124 / 1142..1143 / 1157..1158 均 `transient`（len=2，`transient_fields=['status']`）。

**3. biz 1031..1081 —— PASS**
仍 `value`、`during_load=false`、51 帧、`value_fields=['status']`。
反例测试 `test_long_value_not_washed_by_one_coincidental_value`（:76）真考：段内 f43 出现 8、f44 出现 9
的巧合同值，旧值集合逻辑会判 phase，新逻辑仍 value（按旧规则推演确认，非空转）。

**4. `transient=31` —— PASS（已在 §3 原地更正）**
27 个 biz 单帧段（f1093..f1199）逐条核对：biz 值恒 1（Run）、mesen 在该帧恰有一个 0/4、两侧邻帧为 1，
全部满足「≤2 帧 + 对面恰 1 帧异值」；31 = rust 4 + biz 27，两向对称。

**5. W 与 provenance —— PASS（附一条非阻塞 finding）**
provenance 现有三腿 sha256 + rust `archive_path`（上轮 finding 5 已修）；W=2 由 `lag_histogram`
（`max_abs=1`）导出，不是裸 2。但 `nearest_equal_distance.q95=4` 与 W=2 并列，无一句解释——
而窗口敏感性实验表明这不是修饰性问题：

```
W=2 -> 6 phase / 31 transient / 6 value          （= 入库产物）
W=3 -> 18 phase / 15 transient / 6 value / 4 undecidable
W=4 -> 37 phase / 0 transient / 6 value          （4 段 shift=4）
```

即若按 q95=4 取 W，**transient 类会整体消失**。建议在 `phase_window` 加一句：
q95 是「失配值到最近同值的距离」这一诊断量、不参与 W 推导；W 只由相位滞后上界决定。

**6. 未放松 —— PASS**
`VALUE_MIN_SUSTAINED=3`、`VALUE_NEAR_RADIUS=5`、`TRANSIENT_MAX_FRAMES=2`、`PHASE_WINDOW_UPPER_BOUND=2`
与上轮建议一致（旧版无这些常量，属新增而非放宽）；`pytest tools/tests/test_classify_episodes.py -q`
→ **10 passed**，无 skip/xfail/环境开关；`align_legs.py` 重跑与入库 `leg_offsets.json` **逐字节相同**。

## 总 verdict：`pass`

判据可复现、双向对称、测试真考、Task 1 未回归；唯一 finding 是 provenance 里 q95 与 W 的说明缺失，
不影响产物正确性，建议在 Task 3 落 spec 时补一句。

**第二轮终审：pass，Task 2 完成**（2026-11-18，被复核 commit `e22aaeef`）。

