# gates/ — 门禁工具链

单一真相源是 `gates/cli.py`（AGENTS.md §5 的门禁表即其命令映射）。
**禁止**内嵌第二套扫描逻辑、禁止绕过/弱化/删除门禁；改门禁必须单独成 commit 并写明基线影响。
设计原则移植自 xiyou_remake（PAL3Decomp 教训）。

## 命令

| 命令 | 作用 | 何时跑 |
|---|---|---|
| `python3 gates/cli.py status` | 只读预检：门禁状态 + 基线对照（**动手前必跑**） | 每次会话开工 |
| `python3 gates/cli.py check` | 总闸（pre-commit 自动调用） | 任何 commit |
| `python3 gates/cli.py rom` | ROM sha256 完整性 | 怀疑 `rom/` 被改 |
| `python3 gates/cli.py seed` | `reference/seed/` 投影完整性（D3 冻结基准） | 怀疑投影基准被改 |
| `python3 gates/cli.py naming` | 命名台账证据完整性 | 改 `data/naming/` 后 |
| `python3 gates/cli.py trace` | 差分 L1–L3（含子程序级 L2） | 改 game-data/game-core 后 |
| `python3 gates/cli.py visual` | 视觉 L4 | 改渲染后 |
| `python3 gates/cli.py all` | 全量（所有门禁 + selftest） | 里程碑收尾 |
| `python3 gates/cli.py selftest` | 工具链自证：ratchet 只升不降 / 基线↔历史完整性（历史下界·快照覆盖·sha 链·状态锚点）/ commit-msg 证据门禁（含 ProofDeferred）/ SKIP·TOLERATED 显式打印 / 注册表↔本文件清单 + 命令面双向相等 / proof 证书标记校验 + 触发判定 + DEFERRED 欠账（突变打回用例） | 改门禁后；`all` 内含 |
| `python3 gates/cli.py baseline show\|update\|tolerate` | 基线管理：show=基线+历史；update=只升不降+强制 `--reason` 留痕；tolerate=登记显式 TOLERATED（强制 `--bug` BUG-xxx） | 基线/tolerance 变更 |
| `python3 gates/cli.py commit-msg <msgfile>` | commit-msg 钩子入口（证据门禁本体） | 钩子自动调用 |

> 锚点与基线位置：ROM sha256 锚点 = `rom/SHA256SUMS`（若存在）否则 `data/status/baselines/SHA256SUMS`
> （sha256sum 兼容格式）；ratchet 基线 = `data/status/baselines/baseline.json`，
> 更新留痕 = `data/status/baselines/baseline_history.jsonl`（每条必须带 reason，且携带全量
> `metrics` 快照 + `baseline_sha256` 链）。**历史文件是下界锚，禁止删除/清空**；新仓库 bootstrap
> 需先落 genesis 记录（带 reason + metrics 快照），否则 baseline-integrity 按『历史缺失』FAIL。

## 门禁清单（随阶段启用，SKIP 显式打印）

| 门禁 | 内容 | 状态 |
|---|---|---|
| `rom-integrity` | `rom/` sha256 对照 SHA256SUMS | 启用（P0） |
| `seed-integrity` | `reference/seed/` 反汇编投影 sha+行数对照 baseline | S1 交付后启用 |
| `assets-manifest` | CHR/PRG 资产清单 ratchet（对照 baseline.json） | 启用 |
| `naming-ledger` | 每条 rename 带 evidence 可复现指针 | 台账创建后自动启用 |
| `knowledge-format` | 知识库 frontmatter/type/文件名前缀 | 启用 |
| `docs-links` | 文档相对链接有效性 | 启用 |
| `rust-tests` | `cargo test --workspace` + 测试数 ratchet | crates/ 创建后自动启用 |
| `subroutine-diff` | L2：ROM 原字节执行 vs Rust 函数出入参比对 | oracle/subroutines 建立后启用 |
| `trace-diff` | L3：帧级状态向量逐 tick 比对（oracle vs Rust） | 启用（oracle 腿；game-core 腿 S5 自动启用） |
| `visual-diff` | L4：截图逐像素（白名单外） | S6 启用 |
| `completeness-ratchet` | 完备性矩阵覆盖只增不降 | 矩阵建立后启用 |
| `proof-checks` | int_x SMT 证明+公式↔Rust 桥接复跑（证书防腐） | 启用（按变更面触发；SMT+桥接内联；int_y 穷举留 CI/里程碑） |
| `tree-coherence` | Rust mod 引用⊆提交树（未跟踪件掩盖破树侦测） | 启用 |
| `doc-sync` | 双写片段逐字同文（pipeline↔design，F3 类漂移拦截） | 启用 |

### proof-checks（证书防腐 v2，2026-10-15）

证明证书（SMT unsat / 桥接 0 分歧 / 穷举 0 分歧）入库后若无机器复跑机制，代码改动后旧声明
**静默腐烂**（int-x 复审遗留 3）。v2（设计真相源：rounds/r7 proof-checks v2 设计节）：

- **按变更面触发**：delta（staged∪worktree-vs-HEAD）∩ S_intx = ∅ ⇒ 显式 `SKIP` 行
  （docs-only commit 免 ~43s 全量）。S_intx = integrators.rs / l2ports.rs /
  int_x_smt_proof.py / int-x-bridge.rs / int-y-proof.rs（可影响证书三面的文件全入面）；
  `GATES_PROOF_FORCE=1 python3 gates/cli.py check` 无视变更面强制全量（CI/里程碑/销账）。
  fail-safe：git 不可判 ⇒ 跑全量（**不确定绝不导向跳过**）；S_intx 文件消失 ⇒
  FAIL『前置对象消失』。
- **常驻腿**：`tools/int_x_smt_proof.py`（SMT miter 2^40 + 公式↔Rust 桥接 66,976 点）；
  前置 `cargo build --release -p game-oracle --bin int-x-bridge` **新鲜构建**——stale 桥接
  二进制=假绿，正是腐烂面本身。
- **可选腿**：int_y 2^32 穷举（实测 ~4 分钟）——`GATES_PROOF_FULL=1` 启用（CI/里程碑）；
  默认显式 `SKIP` 打印，绝不静默。
- **证书输出标记校验**：`SMT PROOF…unsat`、`BRIDGE…分歧 0`、`EXHAUSTIVE PROOF…
  4294967296…0 分歧`——exit 0 但标记消失/枚举面缩水/分歧非零一律 FAIL（selftest 含突变
  打回用例）。
- **环境超时 ≠ 证书失败（DEFERRED 欠账制）**：证书坏=FAIL（无豁免）；wall-clock 超时等
  执行不能=原样重试 1 次后 `DEFERRED(环境)` + 欠账登记 `gates/proof_deferred.jsonl`。
  防绿底线：DEFERRED 绝不计 PASS——收尾行打 `CHECK: PASS-WITH-DEFERRED` 不打裸 PASS；
  欠账不静默消失（未销账持续显式 WARN；欠账文件入 sha 锚点，删账=rom-integrity FAIL）。
  **解锁判据**：任一次全量 proof-checks PASS 自动销账（settled 落 PASS 输出原文）；
  欠账期含 S_intx 变更的 commit 必须带 `ProofDeferred: <欠账 id>`（commit-msg 强制）。

## 设计原则

1. **SKIP/TOLERATED 显式打印**：静默通过的门禁等于没有门禁。
2. **基线 ratchet**：`data/status/baselines/baseline.json` 只升不降；更新基线必须在 commit 中说明
   "确认无回退"的证据。**手工改写/删历史机器可检出（t10 R1 + t14 F1 修复）**，缺一不可：
   - 唯一写入口 `baseline update --reason`（拒绝下调、拒绝低于历史下界、拒绝空说明）；
   - `baseline_history.jsonl` **存在性前置**：缺失/为空/损坏 ⇒ FAIL『历史缺失：基线单调性下界不可验证』
     （禁 vacuous 通过）；每条记录带全量 `metrics` 快照 + `baseline_sha256`（sha 链），
     任一 metric 无下界可依 ⇒ FAIL；『metric 现值 < 历史下界』⇒ FAIL『基线被手工调低』；
     『当前 baseline.json sha ≠ 历史链尾』⇒ FAIL 断链；损坏 JSON ⇒ 显式 [FAIL] 解析失败行（禁裸 traceback）；
   - `baseline.json` 与 `baseline_history.jsonl` 均纳入 `SHA256SUMS` sha 锚点（rom-integrity 同验：
     删/改任一状态文件即 FAIL）。
   非自洽/单侧改写不复存在（协调一致的多侧伪造仍靠 `data/status/` 证据门禁与评审把关）：
   污染状态上跑 `baseline update` 会在历史永久留下 `tamper_recovered` 痕迹；
   降基线的提交本身也走 `data/status/` 敏感路径证据门禁（见 hooks）。
3. **失败先定位层**（AGENTS.md §4）：门禁红 ≠ 反复重跑；也 ≠ 调阈值/加白名单。
4. **白名单唯一例外**：`gates/visual_whitelist.json`（S6 建立），变更需用户确认。
5. **门禁命令必须可本地复跑**：CI 与本地 `check` 完全等效，不许存在"只有 CI 会跑"的检查。

## hooks

```bash
bash gates/install-hooks.sh   # 挂接 pre-commit + commit-msg
```

- `pre-commit` → `gates/cli.py check`
- `commit-msg` → 触及 `data/naming/`、`data/status/`、`reference/`、`rom/` 的提交必须含
  `Evidence`/`Confidence` 字段（别名集合与本表严格一致：仅 `Evidence:`/`证据:` 与
  `Confidence:`/`置信度:`，不接受未文档化别名如『信心:』/『置信:』；缺失即拒）

敏感路径集合以 `gates/cli.py` 的 `SENSITIVE_PREFIXES` 为唯一源（文案由集合派生，防漂移）。
`data/status/` 纳入（t14 F1）：基线/历史是裁判的尺子，"降基线无痕提交"必须带证据字段；
baseline 更新本就强制 `--reason`，摩擦可接受。

触及语义（t10 R4②）：删除（D）与改名（R/C）的新旧两侧路径均计入"触及"，删敏感路径同样要证据字段。
另有『前置对象不得静默消失』守卫：门禁前置对象（台账/知识库/seed 等）一旦建立（对应 baseline 指标 >0
即登记 `established`），此后对象消失必须 FAIL『前置对象消失』，不得静默回 SKIP/PASS。

钩子本体是 `gates/hooks/pre-commit`、`gates/hooks/commit-msg` 薄封装，只 `exec python3 gates/cli.py …`；
扫描/裁定逻辑唯一存在于 `gates/cli.py`（无第二套扫描逻辑）。挂接幂等，外来 hook 自动备份。

## backlog（低优先）

- FAIL 文案归属线索（port2 提议，lead 2026-10-15 入账）：门禁 FAIL 时标注「哪个文件/谁的 diff
  引入的红」——现排查靠人肉对圈；实现面=FAIL detail 附相关文件路径/近期改动 hint。
