# R9 三腿对拍（Task 1–6）独立复核

- 复核角色：verifier（**非实施方**），证伪立场，只读审计。
- 对象：`docs/superpowers/plans/2026-11-18-three-leg-oracle-alignment.md` Task 1–6，HEAD `b2496a31`，工作树干净。
- 时间：2026-11-18。所有实验产物落 `/tmp/r9probe*`，未修改仓库任何文件。
- 未执行项：`cargo test`、L4、`tools/tests/test_l4_ratchet.py`（既有 4–5 条 schema v2 遗留失败，按任务约定不计入）。

## 总 verdict

**pass-with-findings**

判据层（能否伪造偏移 / 能否让空对空变绿 / 能否让台账回退）全部扛住了攻击：多解 fail-loud、短序列 fail-loud、边界假偏移在真实三腿数据下被两字段共识拦下、删条目与降级被门禁拦截、常量窗 exit 3、latch 不进 informative、无 skip/xfail/pragma。**不需要 reject。**

findings 集中在两类：
1. **文档证据句**（Task 6 + §11 更正）有 5 处数字/方向/区间轴错误，其中 2 处与本轮自行实测的轨迹直接冲突（`post[f]==pre[f-1]`、`quickerNES status=04`）；
2. **两处工程护栏缺失**：`_field_resolution` 不带边界/裕度断言，`--out` 在 verdict≠aligned 时仍无条件写盘。

两者都不改变"三腿已机械对齐、43 段分类自洽、唯一干净差在 quickerNES 侧、台账只升不降"这一组结论。

## 逐条判定表

| # | 攻击点 | 判定 | 一句话结论 |
|---|---|---|---|
| 1 | `align()` 唯一性 / `changes<3` / `max_lag` 截断 | pass（2 条 P1） | 多解与短序列都 fail-loud；边界假偏移单字段可被接受（无边界闸、无一致率下限），三腿真实数据下被两字段共识拦下 |
| 2 | `kind` 判据能否被"对面随便同值"骗过 | pass | 独立重算 2×value + 2×transient 全部站得住；`biz 1031..1081` 在 k∈[-4,4] 全域 0/51，无任何公共位移 |
| 3 | 台账单调性绕过 / 承重墙 / 基线可手改 | pass-with-findings | 删条目、降级均被拦（实测 rc=1）；手改基线 + `sync-anchors` 可洗白（前轮已登记残余） |
| 4 | `informative` 重开空对空 / latch / `--allow-constant` | pass | 600 帧常量窗 EXIT=3；latch 35 个失配仍不进 informative；`--allow-constant` 输出显式标注 |
| 5 | 数字一致性（分母/判定表/§11 三处更正） | fail（文档层） | 数据层 4 个分母与 43=6+31+6 全对；文档层 5 处证据句错误，§1 表 4 行 kind 与 JSON 冲突 |
| 6 | 判据有无放宽（skip/xfail/pragma/合成数据） | pass | 四个测试文件零 skip/xfail/pragma；diff 未改任何既有门限；合成数据只在单测 |

支持证据（本轮实跑）：`python3 gates/cli.py check` → `PASS=23 FAIL=0 SKIP=3 TOLERATED=1`（37.99 s）：
`[PASS] coverage-ratchet … 覆盖条目 3（unobserved 0 / route-only 3 / verified 0）`、`[PASS] rom-integrity 4 个锚点文件 sha256 一致`；
`python3 -m pytest tools/tests/test_align_legs.py tools/tests/test_classify_episodes.py tools/tests/test_mechanism_scene.py tools/tests/test_coverage_ledger.py -q` → `58 passed in 1.15s`。

## 攻击点 1：`align()` 的唯一性判据

### 1.1 多解 → fail-loud（pass）
```
$ python3 -c "... align({f: [0,0,1,1][f%4] for f in range(40)}, same_shifted, 4) ..."
ValueError: ambiguous offset: candidates=[-4, 0, 4]
```
周期序列 `[0,0,1,1]×6` 三个 offset 同分，`_fit`（`tools/movie_ref/align_legs.py:210-236`）在 `len(winners)!=1` 时抛 `ambiguous offset`，**不会静默取第一个**。这是本工具最重要的承重墙之一。

### 1.2 `changes<3` 能否挡住短序列（pass，但语义需注意）
`_change_frames`（`:111`）返回**连续变化帧**，`_count_runs`（`:116`）数独立段，`MIN_ANCHOR_CHANGES=3`（`:98`）作用在段数上：

| 输入 | 结果 |
|---|---|
| 6 帧交替 `0,1,0,1,0,1` | `insufficient anchor changes` |
| 30 帧全交替 | `insufficient anchor changes`（runs=1） |
| 10 帧仅 1 次跳变 / 4 帧 3 跳变 | 同上 |
| 6 帧 3 段独立跳变 `0,1,1,0,0,1` | **通过**（matches=5） |

⇒ 闸门语义是"≥3 个独立跳变段"，不是"窗口够长"。这是合理设计（交替序列本质含多种位移），但文档若宣称"少于 3 帧不可对齐"需改口径。

### 1.3 `max_lag` 截断会不会把真偏移挤到界外还判 aligned（P1，真实数据未触发）
单字段层面**会**：
```
真偏移 -6、max_lag=4、随机 7 值 run-length 3：
align()                       -> (-4, 60)             # 无任何报错
_field_resolution('status',...) -> offset=-4, matches=78/198, agree_rate=0.393939
```
`_field_resolution`（`:567-605`）只要求 `change_runs>=3`、`best>=3`、`len(winners)==1`，**没有**：
- 边界断言（winner 落在 ±max_lag 时不报 `offset-at-boundary`）；
- 一致率下限（0.39 也接受）；
- participants 一致性（见 1.4）。

端到端层面**没被触发**：把 `/tmp/rust-states-t2.jsonl` 帧号整体 −k（k=5/6/7，真偏移越界）跑真实 CLI：

| k | biz→mesen | rust→mesen | verdict | exit |
|---|---|---|---|---|
| 5 | `+1` / latch `+2` | status `+1` room `+4` | field-disagreement | 1 |
| 6 | 同上 | status `+2` room `+4` | field-disagreement | 1 |
| 7 | 同上 | status `+3` room `+4` | field-disagreement | 1 |

⇒ `resolve_leg`（`:622-700`）要求 status/room **同一个 k**，room 被边界钉在 `+4` 而 status 落在 1/2/3，于是 `field-disagreement`、`offsets.rust_to_mesen=null`、exit 1。**两字段共识是当前唯一挡住边界假偏移的机制**，一旦某场景两字段都只剩弱信号，就会退化到 1.3 的单字段行为。

### 1.4 一致率排序把"分母少 1 帧"当证据（P1）
`_allframe_score`（`:161-208`）按 `agree_rate` 排序，participants 随 offset 变化（`f+offset in seq_b`）。实测：

| 字段 | 次优 | winner | 裕度 |
|---|---|---|---|
| biz room | `0`: 1157/1180 = .980508 | **`+1`: 1157/1179 = .981340** | 0.00083（**原始 matches 完全相同，只靠分母少 1 帧取胜**） |
| biz status | `0`: 1089/1180 = .922881 | `+1`: 1095/1179 = .928753 | 0.0059 |
| rust status | `+4`: 1125/1162 = .968158 | `0`: 1154/1166 = .989708 | 0.0216（次优是边界值） |
| rust room | `-1`: 1155/1166 = .990566 | `0`: 1158/1166 = .993139 | 0.0026 |

`docs/rounds/r9-task-1-review.md` 的 R2-3 曾建议"改一致率或固定 participants 交集"，现行实现选了前者（`:161`），**保留了分母漂移**。biz room 这一票实际上等于抛硬币，只是恰好与 status 一致。

### 1.5 `--out` 在失败时仍写盘（P1）
`align_legs.py:1086-1097` 的顺序是"先 `out_path.write_text(...)`，再 `if verdict != "aligned": return 1`"。上面 k=5/6/7 三例的 `/tmp/r9probe/out-shift{k}.json` **都写出来了**，内容含 `rust_to_mesen: null`。而 `classify_episodes.py` 读 offsets 时不校验 verdict（`:191`/`:221`/`:327`/`:528`/`:815`/`:976-978`）：`null` 会崩（fail-loud，尚可），但若出现"两字段同时命中边界的假偏移"，会被静默吃进 43 段分类。

## 攻击点 2：`kind` 判据是否可被骗

判据链（`classify_episodes.py`）：`transient`（段≤2 帧 + `_single_spike`，`:316`/`:267`）→ `phase`（每字段整段精确位移且**所有字段同一个 k**，`:360-375`）→ `value`（`sustained>=3` 或对面 ±5 内完全不存在，`:316`）→ `undecidable`。优先级 `:441`。

### 2.1 独立重算 2 个 value
- **biz 1031..1081 status**（JSON `shift=null`、51 帧）：全窗 k 扫描（biz 帧 g+1 对 mesen 帧 g+k）：k=-4..0 **0/51**，k=1 1/51，k=2 2/51，k=3 3/51，k=4 4/51。**不存在任何 k∈[-2,2] 的公共位移**，`value` 成立。原始值：biz[1030]=4、biz[1031..1081]=1；mesen[1031..1080]=3、[1081]=0。[对面 ±5 内不存在]也成立（对面只有 0/3，biz 是 1）→ 非"对面随便出现同值"。
- **biz 1131..1136 status+room**（`shift=null`）：status 最高 5/6（k=-2/+1/+2），room 最高 4/6，**不同字段不同 k** → 无 common shift，`value` 成立。注意 status 单字段 5/6 是 near-miss，说明"必须全字段同 k"这道闸确实在承重。

### 2.2 独立重算 2 个 transient
- **biz 1093**（单帧）：biz[1092]=1 vs mesen[1093]=0；mesen 在 [1091,1095] 内值不在 leg 集合的帧**恰 1 帧** → `_single_spike` 成立。160; 这是"真实的一帧抖动"还是"采样尖峰"不可分，但判据规则明确。
- **rust 1108..1109**（2 帧）：rust=[0,1]，mesen=[1,4]，最好 k=+1 也只 1/2；mesen[1109]=4 是窗口内唯一在 {0,1} 之外的值 → transient 成立。

### 2.3 诚实边界（应写进维护者警告）
`transient` 占 **31/43**，其本质是"**段≤2 帧 + 对面窗口恰一帧异值**"——即**真实的一帧行为差也会被判成采样尖峰**。这不是漏洞（文档与 spec 都声明这 31 段不进归因分母、不升台账），但任何"43 段里 31 段是 transient ⇒ 三腿基本一致"的措辞都要带上"这意味着 1 帧分歧被系统性吸收"。

另：JSON 的 `fields` 是 `['status','room','health_lo','health_hi','scrolldir']` **五字段**（`data/status/leg_episodes.json`），而计划/R6 口径写的是状态面 `status+room`。只喂两字段重跑得 42 段，唯一差异是 `biz 940`（JSON 里 `fields=['scrolldir']`）。这不是数据错，是**分类面比对齐面宽且未在文档声明**（P2）。

## 攻击点 3：台账单调性

### 3.1 删条目 / 降级：实测拦住（pass）
在 `/tmp/r9probe` 副本上（未碰仓库）：

| 场景 | 命令 | 结果 |
|---|---|---|
| A 未改动 | `coverage_ledger.py --check` | `LEDGER CHECK: OK` rc=0 |
| B 删 `morphball-unroll` | 同上 | rc=1：`条目 id 删除：morphball-unroll；state 累计计数回退：route-only 2 < 基线 3` |
| C 该条 route-only→unobserved | 同上 | rc=1：`条目 state 回退：morphball-unroll unobserved < 基线 route-only` |
| E 用删条台账跑 `--update` | `--update --reason probe` | rc=2：`拒绝降级（未写盘）` |

### 3.2 承重墙在哪一行（代码引用）
`gates/cli.py:1399-1473` `coverage_ratchet_violations`：
- **`:1462-1465`** `missing=[id for id in base_ids if id not in seen]` → `条目 id 集合减少` —— **这条挡住"删条目变绿"**；
- **`:1466-1473`** `if COVERAGE_STATE_RANK[states[eid]] < COVERAGE_STATE_RANK[old_state]` → `条目 state 回退` —— **这条挡住"老 verified 降级 + 新增 verified 守恒"**（累计计数口径挡不住这个守恒式攻击）；
- `:1441-1459` 累计计数 `sum(counts[s] for s in COVERAGE_STATES[index:]) >= floor` —— 只做数量级兜底；
- `:1429-1437` evidence 路径必须存在且非目录 —— 机械核验证据文件。

`tools/coverage_ledger.py` 侧：`add` 拒绝重复 id（`:118`）、`promote` 拒绝降级（`:130`）、`init` 拒绝重建（`:276`）、`update` 必须先过 `_downgrade_problems` 才写盘（`:297-320`）。

### 3.3 基线可被手工编辑绕过（pass-with-findings，前轮已登记）
- 只改 `data/status/coverage_ledger.baseline.json` → `rom-integrity` 抓到（`gates/cli.py:612-619`，`STATE_ANCHOR_FILES` 在 `:261-266` 含该文件，本轮 sha256 `e4d5e9f4…8929` 与 `SHA256SUMS` 第 4 行一致）。
- 但 `sync-anchors`（`:269` / `:4215` 注册 / `:4256-4257` 调用）**无前置条件**地按当前文件重签：手改基线 + 跑一次 `sync-anchors` 即洗白。实测 D 场景：

```
手改基线（删 id + counts {0,2,0}）+ 台账同步删条
coverage_ratchet_violations(...) -> []
gate_coverage_ratchet(...) -> ('PASS', '覆盖条目 2 …')
```

⇒ 机械防线是"改动可见"（基线 + SHA256SUMS 两个文件同时变），不是"改动被拒"。`docs/rounds/r9-task-4-review.md` 已把这条记为**已知残余**并写明"锚点文件本身仍是可改的文本，与现状同级，不阻塞本 task"——本轮确认该描述仍然准确，不应被读成"基线不可改"。
- 另：门体 PASS 文本与代码注释声称"手工编辑不可采信"（`:1475-1512`），但实际只校验 `reason`/`updated_at` 为非空字符串。**声明强于实现**（P2）。

## 攻击点 4：`mechanism_scene.informative`

```
$ python3 tools/movie_ref/mechanism_scene.py --case cases/case-password-justin.txt --frames 600
OFFSET: +0（anchor=latch(-1+gap); mesen[f] == rust[f+0]）
COMPARE: compared=1180 mismatch=0 incomparable=10 first_diff=None
RESULT: uninformative（常量窗，无判别力）        EXIT=3      （real 30.3 s）
```
- `--fields status,room,latch` → `compared=1770 mismatch=35 incomparable=10 first_diff=300`，仍 `RESULT: uninformative`，**EXIT=3**：latch 的 35 个失配进了 mismatch 但没进 informative。
- `compare()` 直调：只 latch 变化（status/room 恒 0）→ `informative=False`；status 变化 → `True`；未请求字段不参与（`compared=20` 只按请求字段计）。
- `--allow-constant` → `RESULT: uninformative（常量窗，无判别力；--allow-constant 显式接受）`，mismatch=0 时 EXIT=0。

实现依据：`mechanism_scene.py:115-120` `informative=any(len(v)>=2 for f,v in distinct.items() if f != 'latch')`，注释写明"latch 计进 distinct 会让常量窗因 latch 变化被判 informative，空对空被放行"。**空对空没被重开。**

一处小的语义不一致（P2）：informative 排除 latch，但 mismatch 仍计入 latch（35 个失配），所以 `--allow-constant` 在有 latch 失配时给出 exit 1 而非 0——方向安全（更严），但输出里"uninformative"与"mismatch=35"并列，读者会困惑。

## 攻击点 5：数字一致性

### 5.1 数据层全对（pass）
| 量 | 文档 | `data/status/leg_episodes.json` |
|---|---|---|
| 总帧 | 可比 1180 | `total_frames 1180` |
| 进分母 | 1147（rust 1155） | `in_denominator 1147`、`by_leg.rust 1155` |
| 排除 | 33（含 14 缺行） | `excluded.load 33`、`excluded.missing 14`、`missing_within_load 14` |
| 段数 | 43 = 6+31+6 | `total 43`、`phase 6 / transient 31 / value 6 / undecidable 0` |
| value 5 段 during_load | spec §3.4 | `during_load 8 / clean 35`，6 个 value 里 5 段 load=true ✓ |
| count 恒等 | — | 1147+33=1180；1180−11−14=1155；biz 34 + rust 9 = 43 ✓ |

`docs/spec/oracle-verification-chain.md` 规则 7 的"窗口 1180 帧，排除加载窗 33 帧（其中含 14 帧对侧缺失），进分母 1147；by_leg biz 1147 / rust 1155"与 JSON 逐项一致。

### 5.2 文档层 5 处错误（fail）
| # | 位置 | 写的 | 实测/JSON | 影响 |
|---|---|---|---|---|
| 1 | residual L52、§11.2（L675-681） | `post[f] == pre[f-1]` 28 388/28 388 全等 | `post[f]==pre[f+1]`：**1200/1200**（前 1200 帧）、**28407/28407**（全片，对 `full[f+1]`）；`pre[f-1]` 只有 1115/1201 | 方向写反，数字不可复现；"该假说源自旧文件行内不一致、改写后已不是 +1"的**论据**失效（结论另有 rust_to_mesen=0 支撑） |
| 2 | residual L54 | f1030..1080 quickerNES status=**04**（退球） | **04 只在 biz f1030 一帧**；biz f1031..1081 是 **01**（01 持续 51 帧，mesen/rust 是 03） | 机制描述错；"一侧核心差"的结论仍成立 |
| 3 | §11.3（L697-701） | 唯一干净持续值差是 `biz f1031..1081 status`（**= mesen 轴 1031..1081**） | biz_to_mesen=+1 ⇒ biz 行 1030..1080 ↔ mesen 轴 1031..1081 | 区间轴错；与 residual L16/L25 自相矛盾 |
| 4 | residual L28-31 | 672 / 675..678 / 676..678 = transient / phase；940 = transient；1131..1136 = phase；1176..1177、1191..1192 = transient | JSON：phase / value / value；**phase**（shift=1）；**value**；**phase**（shift=1） | §1 判定表 7 行错 4 行；被误标段都在 during_load 或单帧，可执行结论未翻 |
| 5 | residual L39 | 「总帧 1200｜窗口 f20..1199」 | 窗口是 1180 帧（20..1199），1200 是轨迹行数 | 自相矛盾；spec 规则 7 写的是 1180（正确） |

### 5.3 复现命令不可执行（P1）
residual §6（L78-79）：
```
python3 tools/movie_ref/align_legs.py --json-out /tmp/off.json
```
`align_legs.py` 的 CLI 只有 `--bizhawk/--mesen/--rust/--out/--start/--end`（`:840-853`），**没有 `--json-out`**，命令必失败；紧跟的 `classify_episodes.py --json-out …` 缺三个腿输入与 `--offsets`，同样必失败。这与计划 Task 6 的"可复跑"要求直接冲突。

### 5.4 §11 与 residual 的其余一致性
- §11.2「shift=0 口径作废、新映射 biz_to_mesen=+1、锁存 +2、boot −2 是原生相位差」⇒ 与 `leg_offsets.json`（+1/+2、`latch_gap_anomaly false`）一致 ✓
- §11.3「961 可比/15 失配是旧口径」⇒ 新口径 43 段 ✓
- §11.4「room 失配全部 during_load=true、迁移结论不变」⇒ JSON 中 room 分歧段全 load=true ✓
- §11.5「Rust 在 tick945 内完成 FF→09→F0→F1，09 两帧驻留未建模，见 room-5a-transition-verdict.md §5/§8」⇒ 与本轮未复算项，登记为**未验证引用**（不在本轮失败范围）

## 攻击点 6：判据有没有被放宽

- `grep -E "skipif|xfail|@pytest.mark|pragma"` 于 `tools/tests/test_align_legs.py`、`test_classify_episodes.py`、`test_mechanism_scene.py`、`test_coverage_ledger.py` → **零命中**。
- `git diff 03818ed9~1..b2496a31 --stat` → 20 files, +2913/−33；`gates/cli.py` 的改动全是**新增**（`COVERAGE_STATES`、`coverage_ratchet_violations`、`gate_coverage_ratchet`、`sync-anchors` 子命令），**没有任何既有门限被改**；central `data/status/baselines/baseline.json` 未动；`SHA256SUMS` 仅 +1 行（coverage baseline）。
- 无 `std::env::var` 式测试钩子（AGENTS §3.8）；`mechanism_scene` 的所有开关都是显式 CLI 参数。
- "合成数据当真实证据"检查：合成序列只出现在单测与我的探针里；`data/status/*.json` 全部由真实轨迹（BizHawk post / Mesen2 frames / Rust states）生成，`provenance` 记录 rows/last_f。
- 唯一"放宽"是设计内的：`--allow-constant` 显式接受常量窗（明确标注、非默认）、`transient` 吸收 ≤2 帧分歧（明写在 spec 与文档 §5）。均不属于为变绿而放松。

## 修复清单

### P0
无。

### P1
1. **`_field_resolution` 加边界与裕度闸**（`align_legs.py:567-605`）：winner 落在 `±max_lag` 时报 `offset-at-boundary`；次优同分或 participants 差 1 帧致胜时报 `ambiguous-offset`（或统一改为固定 participants 交集后再比 matches，消除 1.4 的分母漂移）。
   - 证据：单字段 `offset=-4, matches 78/198, rate 0.393939` 被当唯一 winner 接受；biz room `1157/1179` 以 0.00083 击败 `1157/1180`。
2. **`--out` 在 verdict≠aligned 时不写盘**（`align_legs.py:1086-1097`），或写 `<out>.rejected`；**`classify_episodes.py` 读 offsets 时校验 `verdict=="aligned"`**（`:191`/`:221`/`:327`/`:528`/`:815`/`:976-978`）。
   - 证据：k=5/6/7 三例 exit 1 但 out JSON 全部写出，含 `rust_to_mesen: null`。
3. **文档证据句校正**（Task 6 交付物）：
   - residual L52 / §11.2：`post[f]==pre[f-1] 28388/28388` → `post[f]==pre[f+1]`（1200/1200；全片 28407/28407），并改写"因此已不是 +1 帧"的论据；
   - residual L54：`quickerNES 04` → 04 仅 f1030 一帧，f1031..1081 为 01；
   - §11.3：`biz f1031..1081（= mesen 轴 1031..1081）` → biz 行 1030..1080 = mesen 轴 1031..1081；
   - residual §1 表 4 行 kind 与 JSON 对齐（672 phase / 675..678 value / 676..678 value / 940 phase / 1131..1136 value / 1176..1177 与 1191..1192 phase）；
   - residual L39「总帧 1200」→「轨迹 1200 行｜窗口 1180 帧」；
   - residual §6 复现命令补齐三腿输入（或指向 `data/status/*.json` 的 provenance 字段）。

### P2
1. `leg_episodes.json` 的 `fields` 是五字段（比 R6 状态面宽），需在 spec/文档声明；否则读者无法从"43 段"反推"状态面几段"。
2. `docs/spec/oracle-verification-chain.md` 全文无「R6」字面（规则用 5./6./7. 编号，仅 `:127`/`:149` 出现「按 R7」），与计划 R6/R7 的编号无法机械对应。
3. `provenance` 依赖 `/tmp/rust-states-t2.jsonl`（未归档、会消失）；`leg_episodes.json` 有 sha256 但 `leg_offsets.json` 只有 path/rows/last_f/mtime。建议把 Rust 状态轨落 `analysis/traces/` 或记录生成命令。
4. `gate_coverage_ratchet` 注释/PASS 文本「手工编辑不可采信」强于实现（只查 `reason`/`updated_at` 非空）；要么加"基线 ↔ 台账上一次 update 留痕"一致性校验，要么改措辞。
5. `transient` 占 31/43，建议在 spec 写明"transient 不是'无差'，只是不进归因分母"，防止下游把 31 段读成"已验证一致"。
6. `--allow-constant` 下 latch 失配会让 exit=1，与"uninformative"并列易误解；建议输出里分开"判别力"与"latch 锚一致性"两行。

## 给维护者的警告（承重墙，不得为让短场景变绿而放宽）

1. **`_fit` 的 `len(winners)!=1`**（`align_legs.py:210-236`）：多解必须失败。任何"短场景测不出来就取第一个/取绝对值最小"的改动，等于让 `[0,0,1,1]` 这类周期序列产生随机偏移。
2. **`MIN_ANCHOR_CHANGES=3` 且按"独立跳变段"计**（`:98`/`:116`）：交替序列必须继续被拒（runs=1）。若改成"帧数≥3"或"跳变次数≥3"，全交替序列会通过并伪装成锚点。
3. **加强 `_field_resolution` 只能加闸，不能减闸**：边界断言、裕度断言、fixed-participants 三者任选其一以上；**不要**为了消掉 `biz room 1157/1179` 这种一分之差就去"取 max matches"（那会让 offset=0 反超，等于换个方向被分母骗）。
4. **`coverage_ratchet_violations` 的 `:1462-1465`（id 集合）与 `:1466-1473`（逐条 rank）**：累计计数（`:1441-1459`）单独存在时挡不住「老 verified 降级 + 新增 verified 守恒」。删任一检查都等于重开攻击面。
5. **`gate_coverage_ratchet` 的"台账缺→FAIL / 基线缺→FAIL"**（`:1475-1512`）：不得改成 SKIP。"删台账"必须继续是失败而不是"停用"。
6. **`informative` 里 `field != 'latch'`**（`mechanism_scene.py:115-120`）：latch 是帧轴锚，不是判定字段。让 latch 进 distinct 会让常量窗因 latch 变化变绿（`case-password-justin` 600 帧即会从 exit 3 变成 exit 0）。
7. **exit 3 的语义**：常量窗 ≠ 通过。`--allow-constant` 是人工显式接受，**不得**写进 CI 默认参数——那会把"无判别力"永久变成绿。
8. **标注轴**：三腿都有各自的帧编号（biz/pre/post/mesen/rust 互相差 0/1/2 帧）。任何"f1030..1080"式区间必须写轴；本轮 5 处文档错误里有 2 处就是轴混用。

## 本轮未验（明确标注，不得读成"已验证"）

| 项 | 原因 |
|---|---|
| `cargo test` / `gates/cli.py l4` / `visual` / `all` | 任务禁跑（>120 s 或需 L4 采集） |
| `tools/tests/test_l4_ratchet.py` 的 4–5 条失败 | 既有 schema v2 遗留，任务约定不计入 |
| `leg_episodes.json` 的五字段重跑（含 `health_lo/health_hi` 段） | 只跑了 `status,room` 两字段（42 段）并与 JSON 逐段比对；唯一差异 `biz 940`（`scrolldir`）由 JSON 字段名推断，未重跑五字段 |
| §11.5「Rust 在 tick945 内完成 FF→09→F0→F1、09 两帧驻留未建模」 | 引用 `docs/findings/room-5a-transition-verdict.md` §5/§8，本轮未复算该机制 |
| spec §3.2「全片五字段一致率 1809/28295=6.39%」 | 只复算了对齐窗口 1180 帧，未复算 28408 全片 |
| Task 5 的 Mesen2 副产物清理 / `--timeout` 行为 | 未构造超时与清理场景 |
| `--allow-constant` 在 mismatch>0 时的退出码 | 只测了 mismatch=0（exit 0）；mismatch>0 的 exit 1 由代码路径推断 |

## 附：本次审计复算的原始读数（供下轮对照）

```
post[f] == pre[f+1]      1200/1200     post[f] == pre[f-1]   1115/1201
post[f] == full[f+1]    28407/28407    full[f] == post[f]   19095/28407
biz status f1027-1029 = 3/3/3  f1030 = 4  f1031..1081 = 1  f1082~ = 1
mesen status f1030..1080 = 3   f1081 = 0   f1082~ = 1
rust  status f1030..1080 = 3   f1081 = 0   f1082~ = 1
biz 1031..1081 vs mesen, status k∈[-4..4] = 0,0,0,0,0 / 1 / 2 / 3 / 4 (每 /51)
leg_offsets.json: biz→mesen +1 (status 1096/1180=.928814), rust→mesen 0 (1154/1166)
mechanism_scene 600 帧: compared=1180 mismatch=0 incomparable=10 → EXIT 3
gates check: PASS 23 / FAIL 0 / SKIP 3 / TOLERATED 1
```
