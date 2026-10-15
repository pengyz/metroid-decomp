# rounds/r7 — 设计线第 11 轮闭环复审账本（state-fixture v9+十轮修集）

> 复审对象：[docs/design/state-fixture-verification.md](../design/state-fixture-verification.md)
> 当前版（v9 + 十轮最小修集，commit 链 c68e933 → 34e7736 → ef80052）。
> 范围限定（任务卡）：①十轮最小修集三件（gotcha 复现方程 / 版本轮次单源 / F15 明细口径）
> ② F2 假 quirk 更正是否闭合 ③ 无新矛盾。
> 立场：证伪、自重导（复现方程/golden 自建断言全部亲跑）；只读复核，不动被审文件。
> 环⑦格式：verdict / findings（逐项复现输出）/ 遗留风险清单。

## verdict: needs_revision

> 【2026-10-15 修后追记】R11-F1（阻断）/R11-F2 修集经 lead 裁定（最小修法 a：头句删尽
> 计数词 + 立「文档头部禁放计数词」规矩）落地并修后复测全绿——**本账 verdict：
> needs_revision → pass（闭环）**。R11-F3（非阻断）补丁文本在案，落点 docs/findings 属
> decomp task-2 写路径，已路由（见遗留 1）。修集落地记录与修后输出见文末节。

三件最小修集中 **gotcha 复现方程、F15 明细口径真闭合**（亲跑复现+双向指针核实）；
**版本轮次单源未闭合**——十轮修集在同一 commit 内「先改头、后补第 10 轮行」，修复自身
再欠 1，头句「九轮」与唯一计数源（r4 轮次表 10 行）构成 9≠10 残余双轨（恰是第 8/9/10
轮连续三轮狩猎的同一缺陷类）。F2 假 quirk 更正**内容面闭合**（golden 自重导决定性证实），
仅证据指针欠精确（R11-F3，非阻断）。

## findings（逐项复现输出）

### R11-F1【声明失实·版本轮次单源未闭合】阻断

头句轮计数与唯一记账来源行数脱钩。文档自述单源规则（design 尾更正注）：
「设计稿版本=commit 标题版本（头为准），**复审轮计数=rounds/r4 轮次表行数（计数唯一来源）**」
⇒ 计数=10；而头句写「v1→v9 **九轮**复审修正史」=9。

复现（原样可跑）：

```sh
grep -Fn 'v1→v9 九轮复审修正史' docs/design/state-fixture-verification.md
# 3:> 状态：设计稿 **v9**（2026-10-14）——v1→v9 九轮复审修正史与计数见 [rounds/r4]…
grep -c '^| [0-9]' docs/rounds/r4-state-fixture-design-review.md
# 10
```

根因（机制性必腐）：c68e933 把「八轮→九轮」对齐的是**改动时点**的 9 行，同一 commit 的
「非阻断同批：r4 补第 10 轮行」随后把行数加到 10——「四票同源」在 commit 落地时即失效。
头句内嵌计数词 + 手工同步 ⇒ 每新增一轮复审必再欠 1（本轮已是同型第三案）。

- 分类：声明失实（残余双轨，第 8 轮 N-C / 第 9 轮 F2-F3 / 第 10 轮 finding-2 同族复发）。
- 处置：回设计线 owner。最小修法**首选头句去计数词**——
  「v1→v9 复审修正史与计数见 [rounds/r4]（唯一记账来源）」（计数只存在于 r4，内嵌
  计数词从机制上消灭）；次选改「十轮」并接受下轮再腐。
- 门禁：本项为文档一致性，机械面暂无门禁覆盖（可选：selftest 加「头句不含轮计数词」
  或「头句计数==r4 行数」断言——属 gates 变更，另行立项）。

### R11-F2【工程卫生·证据可复现性】非阻断

单源规则句跨行断词：「…复审轮计数=rounds/r4 轮次表⏎行数（计数唯一来源）」——
`grep -F` 整句 0 命中（实测），正是本设计线自家 gotcha（第 9 轮 F1：证据 grep 静默 0）
的复发面：**声称的可复现性本身要可复现**。

复现：

```sh
grep -Fn '复审轮计数=rounds/r4 轮次表行数' docs/design/state-fixture-verification.md
# （0 命中，exit 1）
grep -n '复审轮计数' docs/design/state-fixture-verification.md
# 238:> 本次追加修正单源化：设计稿版本=commit 标题版本（头为准），复审轮计数=rounds/r4 轮次表
```

- 分类：工程卫生（关键规则句不可 grep -F 复现）。
- 处置：重排该行令「复审轮计数=rounds/r4 轮次表行数（计数唯一来源）」单行完整；
  可与 R11-F1 同一次修文一并落。

### R11-F3【工程卫生·证据指针欠精确】非阻断

[l2-proof-assumptions.md](../findings/l2-proof-assumptions.md) A6 证据列写
「已证实（ROM 三方 8 判别案 + 自验脚本）」——「8 判别案」清单与「自验脚本」均未落仓
（双工具负扫无痕），指针不可复跑。**实质不受影响**：等价判别证据持久化于
`oracle/subroutines/generated/int_y_verif_3780.jsonl`（三判别输入各 35/39/35 条，
经 l2-diff 全量三方 0 失败，rounds/r6 账），且本轮由我方 golden 自重导复核（见下）。

复现：

```sh
grep -rln '判别\|自验' oracle/ tools/ crates/   # 命中皆无关（decode_sweep/casegen/proof 头注释）
# python 双工具扫描 .py/.rs/.lua/.jsonl：无「8 判别案」清单/「自验脚本」落仓
```

- 分类：工程卫生（证据指针指向临时资产）。
- 处置：A6 证据列改指 `oracle/subroutines/generated/int_y_verif_3780.jsonl`（字面路径）
  + 活跃集判别案 id；「自验脚本」若保留主张须落仓（产出归宿是仓库，环⑪）。

## 已闭合项实证（scoped 四件逐项）

### ① gotcha 复现方程 —— 闭合 ✓（全部亲跑）

对象：[gotcha-evidence-grep-needs-fixed-string.md](../knowledge/gotcha-evidence-grep-needs-fixed-string.md)。

```sh
printf 'a**b\n' | grep -c 'a**b'          # 1  （旧最小例实测 BRE=1——不复现失败，更正注如实）
printf 'a **b\n' | grep -c 'a **b'        # 0  （新失败例 BRE 静默未命中）
printf 'a **b\n' | grep -cF 'a **b'       # 1  （-F 命中）
grep -c '情形；**升级出口（M-C）**' docs/design/state-fixture-verification.md   # 0（exit 1）
grep -Fc '情形；**升级出口（M-C）**' docs/design/state-fixture-verification.md  # 1
for s in 'a**b' 'ab' 'a*b'; do printf '%s\n' "$s" | grep -c 'a**b'; done       # 1 1 1（机理句三串各 1）
```

出处声明核实：9fecd8a Evidence 第 12 行确含 `grep -c '情形；**升级出口（M-C）**' design → 1`
——声称 1 而原样跑 0，「命令不可复现」精确复现，gotcha 真实首例主张**属实**。
机理句「上下文相关、不可预测——一律 grep -F」经三串/两串对照实证成立。

### ② F15 明细口径 —— 闭合 ✓

- design:209「F4-F14 明细见下表（F1-F3 修法锚点在 rounds/r4），F15=时序措辞并入 F3 处置」
  ↔ r4:40「F4-F14 明细在 design §10 v5 表，F15 并入 F3——双向指针」——双向指针在位；
- 表内 F4-F14 全 11 号各有处置行（F4/F5、F6/F7/F8、F9、F10/F14、F11/F12/F13）；
- F15 实质（时序措辞）落 r4 v5 修法 F3「判定时机=窗口末（后置门，(a)(b) 即时门）」
  与 design §3 对应句一致；「半真」口径（旧「F1-F14 明细见下表」越权）已收窄为
  「F4-F14 明细 + F1-F3 锚点指 r4」，与 210 行更正注自洽。

### ③ F2 假 quirk 更正 —— 内容面闭合 ✓（golden 自重导决定性证实）

文本面（双工具负扫 0 残句：grep 管道 + python 全仓扫描，7 命中皆 D4 通用词/更正注引文）：
A6 更正注（含 **Stage B 禁止「修 quirk」**）+[samus-movement.md](../spec/samus-movement.md)
51-54 更正注+禁修令双处在位。

实质面——我方对 3780 golden 自建断言（accel=0 子集，速度变化只来自钳位）：

```
-6.004 $F9:$FF 钳: 5/5 条出参速度写值==(250, 0)      # frac 0xFF→0x00、int 0xF9→0xFA = 钳到 -6.0
-6.0 $FA:$00 含边界钳: 9/9 条出参速度写值==(250, 0)   # |v|≥6.0 含边界成立
-5.996 $FA:$01 不钳: 5/5 条出参速度写值==(250, 1)     # 逃逸值保持 $FA:$01
```

旧假 quirk（「-6.0 钳而 -6.004 不钳」）预测 -6.004 不钳——被 5/5 golden 直接证伪。
更正后语义（干净单调 |v|≥6.0 含边界 + 上下限不对称 +5.0/-6.0）与持久化证据一致；
且 int_y 2^32 穷举证明（proof-checks 门禁今日复跑 4294967296 点 0 分歧）全域覆盖
该行为。**Stage B 禁修令在位——不得按「修 quirk」处理**。

> 探针口径教训（自查）：首版组内断言漏 accel 维（10/35 等假阴性）——积分效应混入钳位
> 判定；补 accel=0 条件后 5/5、9/9、5/5 全一致。断言的测试器也要过证伪（环⑤）。

### ④ 无新矛盾 —— 除 R11-F1 外 ✓

- [$6AFx 矛盾](../findings/ram-region-6af4-contradiction.md)已定谳（$6000-$6Cxx=WRAM
  暂存区，A 级观测三则+实跑证据），「定谳后处置（替代原移植冻结）」与 a91c95f
  （WRAM 域落地+sub_F152）/3751e62（F186/F1BF+F152 补登记）移植链自洽——冻结→解冻→
  移植时序无矛盾；
- design:75「covered_rule 同步待办已落 master-plan（M1）」核实 master-plan:26 在位
  （d1794af 教训：声称落地必须 grep 验证）；
- F15/计数/更正注三处（design 209/210、r4 编号总表）互引一致；gotcha 机理句与实测一致。

## proof-checks v2 设计（2026-10-15 立项；实施待 commit 通路解锁，单独 commit+基线影响）

> 背景：环境 CPU 饥饿（xiyou_remake 负载；用户裁决=等待不干预）暴露 v1 两设计点。
> 先例（指针已核正）：**限流/基础设施故障不进四分类差异路由**
> [gotcha-rate-limit-is-infrastructure-not-diff-routing.md](../knowledge/gotcha-rate-limit-is-infrastructure-not-diff-routing.md)
> ——lead 转述「rounds/r2 限流」实为该 knowledge 条目（迁移件，内指针尚指旧仓
> round-008）+ r2-fix 遗留 5「DONE 挂起现象未根因化」+ 本轮 porter 挂起案。
> 环境故障 ≠ 被测对象的裁决——本设计即该原则在 proof-checks 上的落实。

### ① 按变更面触发（S_intx 触发集）

- **S_intx**（可影响证书三面——Side A 公式 / Rust 实现 / 桥接·穷举探针——的文件全入面，
  实存已核 5/5）：
  `crates/game-core/src/integrators.rs`、`crates/game-core/src/l2ports.rs`、
  `tools/int_x_smt_proof.py`、`crates/game-oracle/src/bin/int-x-bridge.rs`、
  `crates/game-oracle/src/bin/int-y-proof.rs`。
- **判定输入** delta = `git diff --cached --name-status` ∪ `git diff --name-status`
  触及名单（改名两侧均计，沿 gates parse_name_status 既有语义）：
  - delta ∩ S_intx ≠ ∅ ⇒ 全量跑（SMT+桥接；`GATES_PROOF_FULL=1` 加 int_y 穷举）；
  - delta ∩ S_intx = ∅ ⇒ **显式 SKIP 行**：`proof-checks: SKIP（显式：变更面不含 S_intx
    ——delta=<摘要>；GATES_PROOF_FORCE=1 强制复跑）`（docs-only commit 的 43s 延迟消除）。
- **强制通道**：`GATES_PROOF_FORCE=1` 无视变更面全量（CI/里程碑/欠账销账用）。
- **fail-safe 三底线**（防「跳过」成为新的静默绿面）：
  1. git 不可得/delta 不可判 ⇒ 按「含面」处理（跑全量）——**不确定绝不导向跳过**；
  2. S_intx 任一文件消失（删/改名）⇒ FAIL『前置对象消失』，不得 SKIP
     （证书锚面即前置对象，与台账/seed 同构守卫）；
  3. selftest 突变用例：delta 含 integrators.rs ⇒ 必须全量；delta 纯 docs ⇒ SKIP 行
     必须出现且带 delta 摘要；S_intx 文件缺 ⇒ FAIL。
- **基线影响**：无 metrics 变更；README 状态列文案改「启用（按变更面触发；SMT+桥接内联；
  int_y 穷举留 CI/里程碑）」，与注册表逐字同步（selftest 比对）。

### ② 环境超时 ≠ 证书失败（DEFERRED 语义，防绿底线不动）

- **裁决值四元化**：PASS / FAIL / SKIP / **DEFERRED(环境)**——新词刻意与 baseline
  `TOLERATED`（BUG-xxx 已知偏差专用，AGENTS §6）分离，防语义混用。
- **分类线**：
  - **证书坏 = FAIL**（无环境豁免）：证书标记丢（SMT PROOF/unsat、BRIDGE…分歧 0、
    EXHAUSTIVE…4294967296…0 分歧）、unsat 不成立、分歧非零、exit≠0 非超时；
  - **执行不能 = DEFERRED(环境)**：wall-clock 超时 / OOM / 负载饿死。流程：
    ① 同 run 内**原样重试 1 次**；② 仍超时 ⇒ DEFERRED 行 + 复检欠账登记；
    ③ **绝不计 PASS、绝不静默**：SUMMARY 单列 DEFERRED 计数；存在 DEFERRED 时收尾行
    打 `CHECK: PASS-WITH-DEFERRED（复检欠账 N，显式）`，不打裸 PASS。
- **解锁判据（机械销账，防复审抓「静默绿」）**：
  - 欠账行落 `gates/proof_deferred.jsonl`（gates 自有运行态：环境事件账，不是尺子——
    与 baseline 分离；每条含 ts / 触发面 / 超时细节 / 解锁条件 / settled 字段）；
  - **销账** = 此后任一次 proof-checks 全量 PASS（surface 或 FORCE 通道）的输出原文
    写入该行 `settled`；未销账行在每次 check **持续显式 WARN**（欠账不静默消失，
    与「前置对象不得静默消失」同构）；
  - 含 S_intx 变更的 commit 在 DEFERRED 态放行时，commit message **必须含
    `ProofDeferred: <欠账行 id>`**（commit-msg 钩子扩展：触及 S_intx 且当前有未销账
    欠账 ⇒ 强制字段，缺则拒；docs-only commit 本就 SKIP 不涉欠账）。
- **边界**：DEFERRED 永不解锁「N 条 0 失败」类 Evidence 断言（Evidence 纪律独立）；
  证书坏永远不得借 DEFERRED 降级——retry 一旦跑出输出即按输出裁 FAIL。
- **实施切片**：v2.0 = ①触发 + DEFERRED 语义 + 解锁判据打印；v2.1 = 欠账文件机械销账
  + commit-msg 扩展。均待 commit 通路恢复后落 gates（单独 commit + 基线影响声明）。

## 【2026-10-15 修后追记】v11 修集落地记录与修后复测

- **R11-F1**（lead 裁定最小修法 a）：design 头句改「——复审修正史与计数见 rounds/r4
  （唯一记账来源）」，计数词删尽；更正注立规矩「自本注起所有文档头部禁放计数词」。
  修后复测：`grep -Fn '复审修正史与计数见'` → 行 3 1 命中；全文计数词仅存更正注引文
  行 243（历史不改，引文按更正文化保留）。
- **R11-F2**：单源规则句独占一行——`grep -Fn '复审轮计数=rounds/r4 轮次表行数（计数唯一
  来源）'` → 行 239 精确命中（修前 0 命中）。
- **R11-F3**：补丁=把 A6 证据列「ROM 三方 8 判别案 + 自验脚本」改为
  「ROM 三方（oracle/subroutines/generated/int_y_verif_3780.jsonl 三判别组 5/5+9/9+5/5，
  accel=0 子集）+ 活跃集三方」——落点 docs/findings/l2-proof-assumptions.md 属
  **decomp task-2 写路径（写路径互斥）**，已提交 lead 路由，verifier 不越界落笔。
- 链规⑦自证：本追记所有断言均为上方刚打印输出原文；修文经 lead 复核批准（2026-10-15
  裁定），落地由 verifier 执行并 grep 自证——如需完全体独立性可再开第 12 轮抽查（择期）。

## 遗留风险清单（长期持有，不阻断本裁决）

1. R11-F3 一行补丁已备（见修后追记），路由 decomp 落笔后即全清；R11-F1/F2 已闭环。
2. r4 遗留 1-9 续持：域外状态审查模板未定 / 「写集⊆watch」未强制 / 密码链 F_a 谓词
   模板待定 / 场景帧数前置未机械校验 / 「直接副作用链」未操作化 / knowledge 旧词等。
3. A7 same-source 弱化仍靠 3780 独立模型补强（考题抽样人工复核制度化前）。
4. **int_y 穷举证书防腐半径**：proof-checks v1（f51d3b8）默认只跑 SMT+桥接，2^32 穷举留
   `GATES_PROOF_FULL=1`（CI/里程碑）——若 CI 侧无固定带开关的定期跑，int_y 证书仍可
   静默腐烂；v2 的 FORCE 通道与欠账销账不覆盖此缺口，建议里程碑 Gate 或 CI 周期任务
   显式排它（见上「proof-checks v2 设计」）。
5. 本账本方法面：负向结论均双工具（grep 管道+python 扫描/golden 断言）复核；
   hex 串在十进制字段面的 grep 盲区（3780 三判别值首扫 0 命中）系
   gotcha-listing-grep-not-rom-scan 同族第四案——跨格式检索须按字段面建模式。
6. 先例转述指针漂移微案：lead 口径「rounds/r2 限流」实指 knowledge 迁移条目
   （其内指针尚指旧仓 round-008）——转述引用亦须回字面路径核对（本节已核正）。
