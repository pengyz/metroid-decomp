# r9 Task 4 对抗复核：覆盖台账 + coverage-ratchet 门

- 复核对象：HEAD `e761ea8b`（门禁）+ `d6f6ada7`（台账/工具/数据）
- 复核员立场：证伪；未参与实施，未改动工具/数据/门禁
- 复核日期：2026-11-18
- 方法：读代码 + 在 `/tmp` 造副本实证 + 直接调用门函数（不改仓库路径常量以外的任何东西）
- 未跑：`cargo test`、`gates/cli.py check --full`、L4（按任务约束）
- 已跑（任务许可）：`gates/cli.py selftest`、`pytest tools/tests/test_coverage_ledger.py`

## 总 verdict：**reject**

两条 P0（门可被单条命令自我洗白 / 门可被删文件关闭），三条 finding（evidence 不校验、plan 与实现状态不一致、morphball evidence 指错）。

---

## 1. `--seed` 是自证后门 —— **FAIL（P0）**

### 代码

`tools/coverage_ledger.py:148-166`：

```python
def seed(ledger_path=..., baseline_path=...):
    ledger = Ledger(ledger_path)
    for entry_id, state in SEED:        # 缺失就按硬编码 route-only 补回
        ...
    ledger.save()
    baseline = baseline_payload(ledger.items)   # 无条件按“当前台账”重写基线
    baseline_path.write_text(...)
```

基线是**从当前台账推导**的，不是从旧基线单调合并；且不检查旧基线是否存在。

### 实证（/tmp/t4 副本，未碰仓库）

场景：`morphball-unroll` 已合法升到 `verified` 并入库基线；随后有人把它删掉（或它因改名消失）：

```
A) 合法状态（verified 已入库）违规: []
B) 删条目后违规: ['state 计数回退：verified 0 < 基线 1',
                  '条目 id 集合减少：morphball-unroll']      ← FAIL，门有效
```

跑一次 `--seed` 后：

```
$ python3 tools/coverage_ledger.py --seed --ledger /tmp/t4/coverage_ledger.json \
       --baseline /tmp/t4/coverage_ledger.baseline.json
COUNTS: {"route-only": 3, "unobserved": 0, "verified": 0}

C) seed 后违规: []                                          ← 变绿
   台账 states: {'door-transition':'route-only','morphball-unroll':'route-only',...}
   基线 counts: {'unobserved':0,'route-only':3,'verified':0}
```

一次 `--seed` 就把「verified→route-only + 删条目」的回退洗成新基线，门报绿。
这与 AGENTS「基线只升不降、手工编辑 baseline 判 tamper」直接冲突；`gates/README.md:53-54`
还写着「唯一写入口 = … --seed，**只增不降**」——该描述与代码行为不符。

注：`--seed` 对当前入库状态是幂等的（副本重跑与入库逐字节相同，已实测），所以危险不在“现在坏了”，
而在**它把唯一的回退防线做成了可一条命令重定的量**。

### 修复要求

`seed()`（`tools/coverage_ledger.py:148-166`）拆成两条语义，且都不得降级：

- `--init`：仅在基线**不存在**时允许；存在则 exit 2「基线已存在，拒绝重建」。
- `--update --reason <text>`：新基线 = 旧基线与当前台账的**逐项 max**（counts 取 max、ids 取并集、
  states 取 rank 较大者）；若当前台账任一项低于旧基线，exit 2 并打印被拒项，禁止写盘。
- 落盘前把 `reason` 写进基线（`updated_at` / `reason`），对齐 `tools/baseline_update.py` 的
  「只升不降 + 强制 --reason 留痕」既有范式。

---

## 2. 基线唯一写入口 —— **PASS（但唯一入口就是上一条的漏洞）**

```
$ grep -rn "coverage_ledger.baseline" --include=*.py --include=*.md --include=*.json .
gates/cli.py:1379,1443,1444,1447        # 只读
gates/README.md:53                       # 文档
tools/coverage_ledger.py:15,161          # 唯一写入
tools/tests/test_coverage_ledger.py:59   # 测试临时目录
```

唯一写者确为 `tools/coverage_ledger.py:161`（`seed()`），`tools/baseline_update.py:118` 写的是中央
`baselines/baseline.json`，不碰本锚。门读的是 `COVERAGE_BASELINE_PATH`（`gates/cli.py:1379`），
不是台账自身，`coverage_ratchet_violations` 也只吃 `baseline` 参数。判 PASS。

---

## 3. 门在快层且真会红 —— **红路径 PASS；SKIP 语义 FAIL**

层级：`gates/cli.py:2497` 注册项 command=`"check"`；`SLOW_GATES = ("l4","visual-diff")`（`:75`），
`run_all_gates(fast_only=True)` 只跳过 `SLOW_GATES`（`:2565`），所以该门进 pre-commit 快层。
调用实测 **0.70 ms**（`gate_coverage_ratchet()` 直接计时），"0.00s" 成立。

四条路径（`gates/cli.py` 直接调用，路径常量指向 /tmp 副本）：

```
1) 台账缺失            -> SKIP  | 覆盖台账未建立（…--seed 后启用）
2) 台账在、基线缺失    -> FAIL  | 覆盖台账基线缺失…；拒绝无基线通过
3) 正常                -> PASS  | 覆盖条目 3（unobserved 0 / route-only 3 / verified 0）
4) 删 id               -> FAIL  | state 计数回退：route-only 2 < 基线 3；条目 id 集合减少：room-load-chain
5) 一升一降            -> FAIL  | state 计数回退：route-only 1 < 基线 3；条目 state 回退：…
6) JSON 损坏           -> FAIL  | 覆盖台账/基线 JSON 损坏：Expecting property name…
```

「逐条 rank 比计数更严」也实测成立（计数守恒场景）：

```
计数守恒下的一升一降 -> ['条目 state 回退：b unobserved < 基线 route-only']
```

**FAIL 点**：`gates/cli.py:1441` 只要台账文件不在就 SKIP，**即使基线还在**：

```
基线在、台账被删 -> ('SKIP', '覆盖台账未建立…')
```

即删掉 `data/status/coverage_ledger.json` 就能让门静默消失；再 `--seed` 会用硬编码三条重建台账
并重写基线（第 1 条），其它条目全丢而门仍绿。SKIP 只应适用于「首次启用、基线与台账都不存在」。

---

## 4. selftest 计数与真实性 —— **PASS（4 条用例真考门）**

```
$ python3 gates/cli.py selftest | tail -12
  [ok] coverage-ratchet 持平/新增条目 ⇒ 绿 —— []
  [ok] coverage-ratchet 删条目 id ⇒ 红 —— ['state 计数回退：route-only 1 < 基线 2', '条目 id 集合减少：b']
  [ok] coverage-ratchet 条目 rank 回退 ⇒ 红 —— [..., '条目 state 回退：b unobserved < 基线 route-only']
  [ok] coverage-ratchet 门禁实跑（本仓台账 vs 基线） —— 覆盖条目 3（…）
SELFTEST: 160/160 通过
```

160/160 与实施方声明一致；4 条用例断言的是 `coverage_ratchet_violations` 的具体违规串（`gates/cli.py:4075-4098`），
不是「函数存在」。`pytest tools/tests/test_coverage_ledger.py -q` → 6 passed。
不足（非阻塞但应补）：selftest 没有「删台账⇒SKIP」「seed 降级」两类用例，恰好是上面两个洞。

---

## 5. evidence 不校验文件存在 —— **FAIL（属机器可查事实，应校验）**

`coverage_ratchet_violations`（`gates/cli.py:1384-1436`）只读 `id`/`state`，完全不看 `kind`/`evidence`：

```
缺 kind/evidence 的伪造 verified 条目 -> []      ← 门报绿
```

「LLM 伪造事实无法防御」的裁定针对的是**语义真值**；文件是否存在是纯机械事实（`Path.exists()`），
不属该裁定豁免范围，且仓库自身纪律（AGENTS §8「引用前必须验证仍存在」）也要求它。
当前 9 条 evidence 路径**我逐条 stat 全部存在**（见 §6），所以不是当前数据坏了，而是门没有能力
发现下一次的腐坏。

修复：在 `coverage_ratchet_violations` 里对每条目校验 ①`kind ∈ KINDS` ②`evidence` 非空列表
③每个路径 `(ROOT / p).exists()` 且非目录；违规并入 problems（键名 `evidence 缺失/不存在：…`），
并在 selftest 增一条「evidence 指向不存在文件 ⇒ 红」。

---

## 6. 三条初始条目 route-only 是否过标 —— **PASS（door/room 成立；morphball evidence 指错）**

9 个 evidence 文件全部存在（`stat`）：

```
OK 16654 docs/findings/morphball-probe-route.md
OK  3524 cases/case-morphball-probe.txt
OK 10790 shots_rust/case-morphball-probe.l4report.json
OK  5696 docs/findings/case-002-three-roots.md
OK  4225 shots_rust/case-002-door-blue-interaction.sched
OK  4587 shots_rust/case-002-door-blue-interaction.l4report.json
OK 64075 data/status/leg_episodes.json
OK 46354 docs/findings/movie-bizhawk-6725m.md
OK  6157 data/status/leg_offsets.json
```

- **door-transition：成立且被低估**。`case-002-door-blue-interaction.sched` 在 f910..f943 连续开火，
  L4 报告的 8 个采集点是 `game-in f420 / at-door f1080 / bullet-out f1110 / door-open f1160 …`，
  8/8 match、`rom_sha256=e6e6b70…65b6`（与 AGENTS 锚一致）。确实是「真实路线上的门交互」，
  route-only 是保守而正确的标法（机制级最小重现是 Task 5）。
- **room-load-chain：成立**。`data/status/leg_episodes.json` 的加载窗 33 帧来自 6725M 三腿实录，
  加载链在真实路线上被走到过。
- **morphball-unroll：state 站得住，但 evidence 指错文件（finding）**。三条 evidence 里，
  `cases/case-morphball-probe.txt` 头注释写的是「…→拾取 Maru Mari→**形态(down)→滚行**」，
  probe 报告讲的是「到球房的路线 + 物品子系统未移植」，**都不涉及退球(unroll)**；
  真正记录 unroll 的是 `docs/findings/movie-6725m-f1031-unroll.md`（biz f1031 退球成功 status=04、
  Rust 留球 03），它没有出现在 evidence 里。建议补引该文件（并可去掉/保留 probe 报告）。

另：计划 `docs/superpowers/plans/2026-11-18-three-leg-oracle-alignment.md:234` 写「三条 state 全
`unobserved`」，实现是三条 `route-only`。证据支持 route-only（上两条成立），属**计划文本滞后**，
应改计划或在 commit 说明里显式登记该偏离；不能让「判据」与「产物」各说一套。

---

## 修复清单（按优先级）

1. **`tools/coverage_ledger.py:148-166`**：`seed()` 禁止无条件重写基线。落成 `--init`（基线存在即 exit 2）
   + `--update --reason`（只升不降：counts/ids/states 逐项 max；检出降级则 exit 2 不写盘；reason 入基线）。
2. **`gates/cli.py:1441-1444`**：重排 SKIP/FAIL——基线存在而台账缺失 ⇒ **FAIL**
   （`台账被删除（基线存在）：…`）；仅当台账与基线都不存在才 SKIP。
3. **`gates/cli.py:1384-1436`**：补 `kind ∈ KINDS`、`evidence` 非空、每个 evidence 路径存在（非目录）；
   同时核对 `data/status/coverage_ledger.json` 现有 3 条仍绿。
4. **`data/status/coverage_ledger.json`**：`morphball-unroll.evidence` 增
   `docs/findings/movie-6725m-f1031-unroll.md`。
5. **`docs/superpowers/plans/2026-11-18-three-leg-oracle-alignment.md:234`**：初始 state 由「全 `unobserved`」
   改为「全 `route-only`」，并写明 route-only/verified 的判定门槛（verified = 机制级最小重现 + 对拍）。
6. **`gates/README.md:53-54`**：「只增不降」在 1 落地前是假的，改成「`--init` 建锚、`--update --reason`
   只升不降」；并注明删台账不算启用。
7. **`gates/cli.py:4075-4098` 与 `tools/tests/test_coverage_ledger.py`**：补 3 条回归——
   删台账⇒FAIL、`--update` 想降级⇒exit 2、evidence 路径不存在⇒红。

---

## 附：本档只新增一个文件

```
$ git status --porcelain
?? docs/rounds/r9-task-4-review.md
```

未改动 `tools/`、`data/`、`gates/`、`crates/`；所有实验都在 `/tmp/t4/` 副本上。

---

# 第二轮终审：`a3195794` + `36823cd1`

第二轮总 verdict：**pass-with-findings**（P0 已修并实证；剩 3 条文字/噪声/完整性残余）。
所有实验仍在 `/tmp/t5/` 副本；仓库只多本档。

## 1. `--seed` 自证后门已拆 —— PASS

```
1) --init 基线已存在 -> exit=2
   stderr: COVERAGE LEDGER: 基线已存在，拒绝 --init 重建：/tmp/t5/coverage_ledger.baseline.json
2) --check 干净 -> LEDGER CHECK: OK, exit=0
3) 合法 promote door-transition: route-only -> verified
   --update --reason "..." -> exit=0
   COUNTS: {"route-only": 2, "unobserved": 0, "verified": 1}
   门判据: []
4) verified -> route-only -> exit=2（未写盘）
   拒绝降级（未写盘）：条目 state 回退：door-transition route-only < 基线 verified；
                       state 累计计数回退：verified 0 < 基线 1
   基线 sha256 前 90eb6cc1… 完全不变
5) 删条目 room-load-chain -> exit=2（未写盘）
   拒绝降级（未写盘）：条目 id 删除：room-load-chain；…；state 累计计数回退：route-only 2 < 基线 3
6) 全新目录 --init -> exit=0，genesis 三条 route-only
7) --update 缺 --reason -> argparse error, exit=2
```

## 2. 「exact 计数取 max」的反对意见 —— PASS（我的上轮口径确实错了）

- 实测：基线 `route-only=3`，合法 promote 后 exact `route-only=2`，**精确口径必然判回退**（2 < 3）。
  所以上轮修复清单里写的「counts 取 max」会误杀合法升级——实施方的论证成立。
- 累计口径 = `rank >= state` 的条目数（`_coverage_at_least`），在 promote 时 `at_least(route-only)`
  保持 3 不变，只让 exact route-only 减 1；实测合法 promote 后门 `[]`。
- baseline 仍存 exact counts（供审阅）＋ `states`（供逐条 rank 判据），两者都要保留。

## 3. 累计口径的两条洗白路径 —— 均被挡（并指出承重墙）

```
(a) 两条 route-only 塞进一条 verified（另一条悄悄退回 unobserved）:
    ['state 累计计数回退：route-only 2 < 基线 3',
     '条目 state 回退：c unobserved < 基线 route-only']        ← 两道都拦

(b) 老 verified 降级为 route-only + 新增一条 verified 使累计守恒:
    ['条目 state 回退：a route-only < 基线 verified']           ← 只有逐条 rank 拦住

(c) 实测 (b) 的累计量: 现 {unobserved:0, route-only:3, verified:1}
                       基线 {unobserved:0, route-only:2, verified:1}
    三项 at_least 全部相等 ⇒ 累计口径单独会放行 (b)。
```

结论：**累计口径本身不充分，(b) 全靠 `baseline.states` 逐条 rank 检查守住**。这是承重墙：
`coverage_ratchet_violations` 的 `base_states` 循环（`gates/cli.py:1455-1466`）不得删；
selftest 已有「条目 rank 回退 ⇒ 红」，另建议补一条「新增 verified 抵消老 verified 降级 ⇒ 红」把 (b) 钉死。

## 4. 门侧三项修复 —— PASS

```
两者都不存在            -> SKIP
基线在、台账删          -> FAIL '台账被删除（基线存在）：…；删台账不算停用，必须恢复台账或显式裁定'
正常                    -> PASS（0.18 ms）
evidence 路径不存在      -> FAIL 'evidence 不存在：door-transition -> docs/findings/does-not-exist.md'
evidence 为空            -> FAIL 'evidence 缺失：door-transition'
kind 非法               -> FAIL "台账 door-transition 的 kind 非法：'bogus'"
```

层位：注册项 `gates/cli.py:2537` command=`"check"`，`SLOW_GATES=("l4","visual-diff")`，仍在快层；
实测 `gate_coverage_ratchet()` **0.18 ms**。

## 5. selftest 164/164 与测试 —— PASS

```
$ python3 gates/cli.py selftest | tail -3
  [ok] coverage-ratchet 台账与基线均缺失 ⇒ SKIP ...
  [ok] coverage-ratchet 门禁实跑（本仓台账 vs 基线） —— 覆盖条目 3（…）；累计 state 计数与 id 集合只增不降，evidence 路径存在
SELFTEST: 164/164 通过
$ python3 -m pytest tools/tests/test_coverage_ledger.py -q
11 passed in 0.02s
```

coverage 用例从 4 条增到 8 条（`gates/cli.py:4121/4128/4135/4143/4150/4157/4161/4163`），
分别断言「持平绿」「合法升级绿（累计口径不误杀）」「删 id 红」「rank 回退红」「evidence 红」
「台账缺失红」「都缺失 SKIP」「实跑 PASS」；断言的是具体违规串，不是函数存在。测试无 skip/xfail。

## 6. 数据/计划/README —— PASS（两处文字残留，见 §8）

- `data/status/coverage_ledger.json`：`morphball-unroll.evidence` 已补
  `docs/findings/movie-6725m-f1031-unroll.md`（4 条路径）。
- `data/status/coverage_ledger.baseline.json`：含 `reason`/`updated_at`，exact counts 与 states 自洽。
- 计划 Step 3：改为 `--init`、三条 route-only、`verified 门槛=机制级最小重现 + Rust 同场景 0px 可复跑指针`；
  Step 4 期望 `11 passed` 与实测一致。
- `gates/README.md:54`：`--init`（仅首次建锚）/`--update --reason`（只升不降）+「删除台账不算停用」——
  与实现一致。

## 7. `--update` 无内容变化仍写盘 —— FINDING（低）

```
$ cp baseline base.before
$ python3 tools/coverage_ledger.py --update --reason "无变化重跑" --ledger … --baseline …
exit=0
$ diff -u base.before baseline
-  "reason": "补引 morphball-unroll 的 f1031 退球证据；只升不降，无条目删除/降级",
-  "updated_at": "2026-11-18T08:54:48+00:00"
+  "reason": "无变化重跑",
+  "updated_at": "2026-11-18T09:02:42+00:00"
```

内容等价时仍产生 reason/updated_at diff。建议 `update()`（`tools/coverage_ledger.py:299-309`）
在 `_merged_doc` 与旧基线等价（states/ids/counts 相同）时不写盘，打印 `NO-OP: 基线已是上界，未写盘`。

## 8. 残余（不阻塞）

1. **口径文字残留（应改）**：`gates/README.md:76` 与注册表 `gates/cli.py:2537` 仍写
   「覆盖台账三态计数只增不降」——累计口径下 exact 三态计数**可以**下降（promote 必然让 route-only −1）。
   建议改「累计计数（rank≥state 的条目数）只增不降」。selftest 只比对 name/state 对，改描述不影响。
2. **基线无完整性链（中低）**：手改 baseline 把三态全降到 `{unobserved:3}`（counts/states 自洽）后，
   `coverage_ratchet_violations` 返回 **`[]`**（实测）。中央 `baselines/baseline.json` 有 `baseline_sha256`
   链 + baseline-integrity 门；本锚没有。建议把该 baseline 的 sha 纳入既有链，或门内比对
   `git show HEAD:data/status/coverage_ledger.baseline.json`。
3. **nit**：`gate_coverage_ratchet` 的「台账被删除」文案硬编码仓库路径，传自定义路径时仍打印
   `data/status/coverage_ledger.json`。

## 9. 残余修复清单

1. `gates/cli.py:2537` + `gates/README.md:76`：改「累计计数（rank≥state）只增不降」。
2. `tools/coverage_ledger.py:299-309`：`update()` 内容等价时不写盘（避免无意义 reason/updated_at diff）。
3. （中低）基线完整性：`--init`/`--update` 写盘时记录 baseline sha256 并纳入既有 baseline-integrity 链，
   或门内与 `git show HEAD:` 比对，防手改降级。
4. （nit）`gate_coverage_ratchet` FAIL 文案使用传入路径。
5. 测试补两条：`--update` 等价内容不写盘；「新增 verified 抵消老 verified 降级」⇒ 红（钉死 §3(b)）。

---

# 第三轮终审：`2b8436d1` + `cf21930e`

第三轮总 verdict：**pass**（残余 #1/#2 修完，#3 以「基线须带 CLI 留痕」替代 sha 链并明确降低期望，#4 保留为 nit）。
未改动 `tools/`/`data/`/`gates/`；实验在 `/tmp/t6`、`/tmp/t7`；仓库只多本档。

## 1. 措辞改为累计口径 —— PASS

```
$ grep -rn "三态计数只增不降" --include=*.md --include=*.py .   # 仅本档历史记录命中
$ sed -n 76p gates/README.md
| `coverage-ratchet` | 覆盖台账**累计计数**（rank≥state 的条目数）只增不降、条目 id 不减、逐条 rank 不回退、evidence 路径须存在、基线须带 CLI 留痕 | 台账建立后启用 |
$ sed -n 2545,2547p gates/cli.py
    ("coverage-ratchet", "台账建立后启用", "check",
     "覆盖台账累计计数（rank≥state）只增不降、条目 id 不减、逐条 rank 不回退、evidence 路径须存在",
```
nit：注册表摘要少了「基线须带 CLI 留痕」半句（与 README 行不完全一致；selftest 只比对 name/state 对，故不报错）。

## 2. `--update` 等价不写盘 —— PASS

```
$ cp data/status/coverage_ledger*.json /tmp/t7/{l,b}.json
$ python3 tools/coverage_ledger.py --update --reason "noop probe 1" --ledger /tmp/t7/l.json --baseline /tmp/t7/b.json
exit=0；stderr: NO-OP: 基线已是上界，未写盘
$ sha256sum /tmp/t7/b.json ; stat -c %Y /tmp/t7/b.json      # 与更新前比较
sha 相同: YES | mtime 相同: YES
# 真升级仍写盘（door-transition route-only→verified）
$ python3 tools/coverage_ledger.py --update --reason "真升级" … -> exit=0, COUNTS {route-only 2, verified 1}, sha 变化: YES
```

`2b8436d1` 的 diff 只有 9 行新增（`PROVENANCE_KEYS` 常量 + `update()` 里的等价比对 + NO-OP 返回），
**`_downgrade_problems` 与 `_merged_doc` 未被触碰**（`git show 2b8436d1 --stat`：1 file, +9）。

## 3. 基线留痕校验 —— PASS（作为「防手滑」足够；作为「防蓄意手改」不足，已在下方给出补链位置）

```
A 合法 pairs                -> PASS
B 去掉 reason/updated_at    -> FAIL 覆盖台账基线缺 CLI 留痕（reason/updated_at）：基线只能由
                                    tools/coverage_ledger.py --init|--update --reason 产生，手工编辑不可采信
C 留痕补回                  -> PASS
D 手改三态全降但保留留痕    -> PASS   ← 实测 coverage_ratchet_violations(...) == []
```

- 不构成判据放松：这是**新增**的 FAIL 路径（严格更严），累计计数 / id 集合 / 逐条 rank / evidence 四条判据
  全部保留；`selftest` 165/165、pytest 11 passed、本仓门实跑 PASS。
- 但它只能抓「手改后忘了补留痕」。蓄意保留 `reason`/`updated_at` 再降 counts/states 仍绿（实测 D）。
- **补链该补在「SHA256SUMS 状态锚点」层，不要放门里跑 git**：`gates/cli.py:262` 的
  `STATE_ANCHOR_FILES` 已含 `baseline.json`/`baseline_history.jsonl`，`gate_rom_integrity`（:588-611）
  会对锚点里**每一个**条目做 sha256 校验（文本按 LF 归一）。只需：
  ① 把 `data/status/coverage_ledger.baseline.json` 加进 `STATE_ANCHOR_FILES`；
  ② `tools/coverage_ledger.py` 在 `_init`/`_update` 写盘后同步写 `data/status/baselines/SHA256SUMS`
     的对应行（格式 `<sha>  <relpath>`，勿新增 gate 依赖）；
  ③（可选）`gate_coverage_ratchet` 读锚点条目与基线实际 sha 比对，做快层 fail-fast。
  该模型与本仓 `baseline.json` 现有防护完全同构；代价是锚点文件本身仍是可改的文本（与现状同级），
  若要与 `baseline_history.jsonl` 那种 prev-sha 链同级，再单独排期，不阻塞本 task。

## 4. 路径文案硬编码 —— 保留为 nit（同意）

`gate_coverage_ratchet` FAIL 文案里的 `data/status/coverage_ledger.json` 是默认路径；
传自定义路径时印错。仅影响可读性，不影响判定。若要顺手改：用传入的 `ledger_path` 走 `rel()`。

## 5. 测试

```
$ python3 gates/cli.py selftest | tail -1
SELFTEST: 165/165 通过
$ python3 -m pytest tools/tests/test_coverage_ledger.py -q
11 passed in 0.05s
```
新增用例「基线缺 CLI 留痕（手编辑）⇒ 红」在 `gates/cli.py:4173-4178`，断言 `FAIL` 且 detail 含「CLI 留痕」，
走真实 `gate_coverage_ratchet(ledger, baseline)`，能卡住留痕校验被删的回归。

## 6. 给后续维护者的一句警告

> **`gates/cli.py` 里 `coverage_ratchet_violations` 的「逐条 rank 不回退」检查（`baseline.states` 循环）
> 是承重墙，不得删除或"简化"成只看累计计数**：累计口径单独挡不住「老 verified 降级为 route-only ＋
> 新增一条 verified 使累计守恒」这条洗白路径（实测三项 at_least 全相等、仅逐条 rank 拦住）。

**第三轮终审：pass，Task 4 完成（残余 #3 以留痕校验替代、#4 nit 保留）**
