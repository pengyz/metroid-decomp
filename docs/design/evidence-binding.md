# 证据与代码树绑定（matrix / coverage 的 `head` + `at`）

> 2026-11-25 · 卡 #72 · owner 承宇 · 复核 对影
>
> 一句话：**没有 `head` 和 `at` 的数字不是证据，只是传闻。**
> 本文件是绑定规则的单一真相源；实现是 `tools/evidence_bind.py`（`sign` / `verify` / `selftest` / `rules`）。

## 0. 为什么要有这份规则

`shots_rust/matrix.json`（L4 对拍矩阵）与 `analysis/tas-100/coverage.json`（TAS 覆盖率报告）
都是**跨会话被别人拿去当判据**的产物，但它们此前：

| 产物 | 旧状态（实测） | 后果 |
|---|---|---|
| `shots_rust/matrix.json` | **没有 `head`、没有 `at`** | 「171/172 全绿」是哪棵树的 171/172？只能靠猜 |
| `analysis/tas-100/coverage.json` | `env.git_head = "e4d23d6f4d83"`（**12 位短 sha**），无 `at` | 短 sha 无法判定归属；实测 `e4d23d6f` **不是主线 HEAD 的祖先**（见 §4） |

两条都是「数字可复现、但复现对象不确定」。这类缺口不会让门禁变红，却会让后来人拿着错位读数
下结论——比报红更贵。

## 1. 规则一：哪些产物必须带 `head` / `at`

| 产物 | 落盘者 | 必带字段 |
|---|---|---|
| `shots_rust/matrix.json` | `tools/l4_diff_run.py run-all` | 顶层 `head`(40 位) + `at`(ISO)；**`rows[]` 每行、`rows[].cells[]` 每个帧级 cell 同值**（172 格逐个可引用） |
| `analysis/tas-100/coverage.json` | `tools/tas_replay_diff.py`（`env` 段） | `env.git_head`(40 位) + `env.at`(ISO) |

约定：

- `head` = `git rev-parse HEAD` 的 **40 位**全 sha，禁止短 sha、禁止 `null`。
- `at` = `YYYY-MM-DDTHH:MM:SSZ`（UTC）。
- **缺 HEAD 一律 fail-loud**（退出码 3），不写 `null` 占位——占位一旦写进产物，就会被下游当成「有时间字段」读过去。
- **脏树默认拒签**：工作树有未提交改动时，同一 sha 会对应两份不同代码，数字不可复现。
  确需在脏树上落盘时显式 `--allow-dirty`，工具会把脏文件清单写进 `matrix.dirty` / `env.dirty`，供读取方判风险。
- 幂等：`--deterministic` 保留既有 `at` 不更新，同一 HEAD 连跑两次产物逐字节相同。

## 2. 规则二：读取方如何校验

任何引用这两个产物的报告、任务卡、验收 note，必须先跑：

```bash
python3 tools/evidence_bind.py verify -f shots_rust/matrix.json
python3 tools/evidence_bind.py verify -f analysis/tas-100/coverage.json
# 要求「就是当前这棵树」时再加：
python3 tools/evidence_bind.py verify -f shots_rust/matrix.json --expect-head "$(git rev-parse HEAD)"
```

`verify` 查四件事：①字段存在且格式合法；②`head` 能在本仓解析为提交（`git rev-parse --verify <sha>^{commit}`）；
③（coverage）`env.git_head_generated` 若存在，必须是 `env.git_head` 的**分支祖先**；④（可选）与期望 HEAD 一致。
任一项不过 ⇒ 退出码 3 并逐项打印。

判定口径（互斥，直接可代入）：

| 情况 | 判 |
|---|---|
| `verify` OK 且 `--expect-head` 等于当前 HEAD | 这份数字就是当前这棵树的 |
| `verify` OK 但 head ≠ 当前 HEAD | 是**历史树**的数字：可当基线，不可当「当前状态」 |
| `verify` FAIL（缺字段/不可解析/非祖先） | 数字**来历不明**，不得引用为判据 |

### 2.1 提交之后：要 `--expect-head` OK 必须先 rebind

产物签在 `A`，之后又提交了 `B`（哪怕只改注释）——此时产物 head 仍是 `A`，
`--expect-head $(git rev-parse HEAD)` **必然 FAIL**，而且这是**正确**行为（产物确实出自 `A` 那棵树）。
所以：

- **当基线读**（最常见）：只跑 `verify`，看 OK 即可，别加 `--expect-head`；
- **要证明「就是当前这棵树」**：先重签，再验：

  ```bash
  python3 tools/evidence_bind.py sign -f shots_rust/matrix.json --rebind   # 旧 head 记入 head_rebound_from
  python3 tools/evidence_bind.py verify -f shots_rust/matrix.json --expect-head "$(git rev-parse HEAD)"
  ```

2026-11-25 现场：matrix 签在 `0cf5af8d`、随后提交 `f7da5517`（按内容兜底判类型），
我第一次给复核人的清单把「`--expect-head` 期望 OK」写成验收命令 —— 那一步当时应 FAIL。
重签后 `head=f7da5517…`、`head_rebound_from=0cf5af8d…`，两种验法才都成立。
**这类口径错误会直接让复核人对不上账，必须在文档里写死操作顺序，而不是靠记性。**

## 3. 规则三：lane 产物 pick 回主线后必须重签

单 worktree 串行滚动（`docs/design/impl-gap-plan.md` §3）下，`analysis/` 里的采集往往先在
lane 树上跑。产物回到主线时必须重签，否则读数是 lane 的、标签是主线的：

```bash
# lane 树上产出 → 进主线前重签（旧 sha 记进 *_generated / head_rebound_from，不丢证据）
python3 tools/evidence_bind.py sign -f analysis/tas-100/coverage.json --rebind
```

重签语义（写死，避免歧义）：

- `env.git_head` 换成**签回时的主线 HEAD**；
- 产生树的原值存 `env.git_head_generated`（升成 40 位）、签回时间存 `env.rebind_at`；
- `verify` 会检查 `git_head_generated` 是否为 `git_head` 的祖先。**不是祖先 ⇒ 判 FAIL**：
  说明这份数据产生于未进入主线历史的树，「重签」不能改变这一点，只能在主线重跑。

## 4. 现场证据：现行 coverage 就是「未 pick 的 lane 产物」

```bash
$ git rev-parse HEAD
35f779dfbd18f9dfb1d4430ecfdd8fed5af7d79e
$ git rev-parse e4d23d6f4d83
e4d23d6f4d838fe78dfc935dbd29852183629be0
$ git merge-base --is-ancestor e4d23d6f4d838fe78dfc935dbd29852183629be0 HEAD; echo rc=$?
rc=1        # ← 不是祖先：该 lane 提交未进入主线历史
```

所以 `analysis/tas-100/coverage.json` 现在的 `env.git_head` 只能标成「产生树」，
**它不能被合法重签为当前主线的产物**；要在主线上得到一份可引用的 100% 覆盖率报告，
必须在主线（或 pick 后的主线）重跑 `tools/tas_replay_diff.py`（约 37 分钟），本卡不改判据、不迁数据。

这条是规则的活例：`verify` 的祖先检查会把这类产物直接判成 FAIL，而不是让短 sha 混过去。

## 5. 与既有门禁的关系

| 门禁 | 关系 |
|---|---|
| `tools/l4_diff_run.py run-all`（矩阵生产） | 落盘前调用 `evidence_bind.stamp_matrix()`；脏树 ⇒ fail-loud（退出码 2），`--allow-dirty` 放行并留痕 |
| `tools/tas_replay_diff.py`（覆盖率生产） | `_env_fingerprint()` 直接产出 40 位 `git_head` + `at` |
| `gates/cli.py check`（24 项） | **未新增门禁项**（卡面要求 PASS=24 不变）；绑定校验由 producers + `verify` 承担 |
| `selftest`（`tools/evidence_bind.py selftest`） | 17 组断言：正常签名、伪造 sha、null 时间、未签名、期望不一致、短 sha 升级、非祖先来源、幂等、`--deterministic` 无 at 拒绝、换树未 `--rebind` 被拒、`--rebind` 保留旧值、cells 逐格写入 head/at、cells 绑定后可验、漏格 head 被拒、脏树判定 |

## 6. 复现命令（验收用）

```bash
python3 tools/evidence_bind.py selftest                      # 17 组断言
python3 tools/evidence_bind.py rules                         # 规则摘要
python3 tools/l4_diff_run.py run-all                         # 矩阵落盘即带 head/at
python3 tools/evidence_bind.py verify -f shots_rust/matrix.json            # 当基线读：只验不比对 HEAD
# 要「就是当前这棵树」，先重签再验（顺序不能反，见 §2.1）：
python3 tools/evidence_bind.py sign -f shots_rust/matrix.json --rebind
python3 tools/evidence_bind.py verify -f shots_rust/matrix.json --expect-head "$(git rev-parse HEAD)"
python3 tools/evidence_bind.py verify -f analysis/tas-100/coverage.json   # 现件会 FAIL（§4）
```

## 7. 已知缺口（不假装完整）

1. `analysis/` 与 `shots_rust/` 都被 gitignore：绑定字段随产物存在，**不进版本库**；
   跨机器共享时靠 `head` + `at` 重建信任，不靠文件本身。
2. 绑定只证明「这份产物出自哪棵树」，**不证明内容正确**——内容判据仍由 `check`/`l4`/`selftest` 负责。
3. 未覆盖其他产物（`analysis/traces/*.jsonl`、`shots_rust/*.divergences.json` 等）：
   它们由 `run-all` 与 `scenario_run` 产出，目前只能通过 `matrix.head` 间接定位；如需单件绑定，另开卡。
