# prompts/ — 角色提示词与提示词纪律

新任务开工前**必须先选定角色 prompt**，把它作为该任务的完整工作契约（AGENTS.md §7）。
角色 prompt 是任务级提示词的模板；用户的具体任务以"任务卡"形式追加在角色 prompt 之后。
全自动回路（[docs/ai-decompile-pipeline.md](../docs/ai-decompile-pipeline.md) §4）中，
任务卡由完备性矩阵派生，同样遵循本模板。

## 角色一览

| 角色 | 文件 | harness 技能（稳定调用入口） | 适用阶段 | 产出归宿 |
|---|---|---|---|---|
| 反编译分析员 | [decomp-analyst.md](decomp-analyst.md) | `remake-decomp-analyst` | S1–S3 投影/命名/数据表考证 | `data/naming/` 台账 + `reference/` + `docs/findings/` |
| 规格作者 | [spec-writer.md](spec-writer.md) | `remake-spec-writer` | S4 理解逻辑 | `docs/spec/` |
| 改写工程师 | [porter.md](porter.md) | `remake-porter` | S5 Rust 移植 | `crates/` |
| 差分验证员 | [verifier.md](verifier.md) | `remake-verifier` | S6 验证/门禁 | `gates/` + `data/status/` + bug 台账 |

> **技能同步规则**：同名 skill（`~/.agents/skills/remake-*/SKILL.md`）是 harness 的稳定调用入口，
> 本目录文件是仓库内完整契约。修改任一侧必须同步另一侧；冲突时以本目录 + AGENTS.md 为准。

## 提示词纪律（所有角色通用）

1. **任务卡最小要素**：目标、范围（可改路径）、证据输入（已有 findings/台账条目）、完成判据（跑哪个门禁命令）。
2. **产出必须包含**：①证据引用（A/B 级，见 AGENTS.md §3）②完成判据的实际执行结果 ③遗留假设清单（C 级）。
3. **产出的归宿是仓库**：台账 / spec / 代码 / 知识库 / bug 台账。聊天记录里的结论**不许**作为后续任务依据。
4. **禁止事项继承**：角色 prompt 中的"禁止"叠加生效于 AGENTS.md 的禁令，不互相覆盖。
5. **完成≠声称**：角色任务结束前必须实际运行完成判据的门禁命令并贴输出（AGENTS.md §5）；
   门禁未启用（SKIP）时，完成判据退化为"人工可验证的最小证据"并在产出中注明。
6. **角色切换**：一个任务卡只用一个角色。需要切换角色（如移植中发现理解错误）→ 停下，
   按 AGENTS.md §4 三问定位层，另开对应角色的任务卡，禁止混着做。
7. **投影不背书**：IDA/Ghidra 的反汇编与伪代码是线索不是事实；引用时注明工具与地址，
   与 trace/ROM 字节冲突时以 ROM 为准（AGENTS.md D3/D5）。

## 任务卡模板

```markdown
### 任务卡 <id>
- 角色: <decomp-analyst|spec-writer|porter|verifier>
- 目标: <一句话>
- 范围: <可修改路径；未列出 = 不许改>
- 证据输入: <findings/台账/汇编地址 bank:addr/trace id/代码行指针>
- 完成判据: `python3 gates/cli.py <gate>` 全绿 + 产出物清单
- 禁止: <本任务特有禁令>
```

## 修复-复核-验收链条（实战验证版，2026-10-15 全队下发）

> 本链条每一环都被至少一次真实抓错验证过（括号=教训案数）。团队成员按此执行；
> 违反链规的产出在复审中直接判 needs_revision。

### 改写侧（实施者）
1. **TDD case 先行**：factory 生成/手写用例=RED（或三方先行打回），实现后 GREEN——
   禁止先写实现再补用例（EOR zp 案：实现错、穷举过，唯三方抓回）。
2. **修复≠声称修复**（4 案）：批量文本手术/代码修复后**逐条 grep 或实测验证**才许声称
   落地；python replace 静默未命中是惯犯——「print('完成')」不等于完成。
3. **Evidence=刚打印的输出原文**（2 案）：commit 里任何「N 条 0 失败」断言必须是该行
   上方刚打印的输出；禁止凭预期写数字。
4. **否定结论换工具复核**（2 案，扫描盲区）：「不存在/0 命中」有两种成因（真不存在/
   工具盲区）——listing grep ≠ ROM 扫描，反证必须双工具。
5. **测试器先过证伪**（2 案）：新测试/探针先用已知坏例验证它抓得住（突变敏感性），
   再信它的绿。竞态测试注意语义陷阱（flipper 移动语义吃源文件→压力归零）。

### 复核侧（verifier）
6. **独立对抗复核**：实施≠审核；verifier 以证伪立场**自重导**（自己跑命令、自建探针、
   ROM 字节级核）；**verdict=pass 才算步骤完成**（多轮 needs_revision 是常态，见 rounds/r3-r7）。
7. **复核输出格式**：`## verdict: pass|needs_revision` + `## findings（逐项复现输出）`
   + `## 遗留风险清单`——findings 分类（实现缺陷/契约违规/声明失实/工程卫生）+ 每条带
   复现命令与实际输出。
8. **突变敏感性抽查**：对用例集故意引入假想缺陷（漏变址/误读立即数/…）看打回率——
   判别力不足的「全绿」是假象（casegen 退化案：8 案 0 判别力）。

### 共同侧
9. **更正文化**：结论被推翻→原文**原地日期更正**（`> 【YYYY-MM-DD 更正】…`），
   不删除历史主张（假 quirk 案：更正注里写明「勿作 Stage B 修复项」）。
10. **红绿不混**：未实现不入活跃用例集（staging→活跃=与实现同 commit）；缺口走
    `unported!`/显式账，不挂红灯；基线只升不降、update 必带「确认无回退」。
11. **产出归宿是仓库**：聊天里的结论不作后续依据；证据指针必须可复现（字面路径、
    grep -F、命令与数字一并抄录）。
12. **先建目标后建引用**（11 衍生，2 案共享树阻塞）：引用未落库的文档/条目=悬空链接
    ——docs-links 全树扫描会卡住**所有人**的提交（共享工作树）。先落目标再落引用，
    或引用暂以纯文本占位。
13b. **共享树验证禁 stash**（2026-10-15 第七案，2026-10-16 立规）：验证「纯提交树构建」
    一律 `git worktree add` 独立检出（**禁 git stash -u**——会卷走他人在途工作，pop 半途
    失败即事故）；stat 对账升格**逐文件归属核对**；验证声明附方法自证。教训源=stash 摘净法
    （曾被 lead 背书为「新 checkout 语义等价」——方法在共享树上有害，原地更正：以 worktree 法为准。
13a. **共享账本 append 前查 max-ID**（2026-10-15 撞号案，共享文件并发第三类）：
    ledger.jsonl 类共享账本并行 append 前先查最大 ID 并续号（symbol 判别+ID 唯一断言随写）；
    撞号处置=后到者重排+事故注入记录。机械护栏=naming-ledger 门禁加 ID 唯一性检查。
14. **共享分支禁裸 amend**（1 案，r8 §1）：`git commit --amend` 改写的是**当时的 HEAD**
    ——共享分支上 4 分钟内就可能被他人提交插队，amend 会顶替**别人的 commit message**。
    须先核 `git log -1` 确认 HEAD 仍是自己的 commit；或改用「新 commit + 原地更正注」。
    已发事故零内容损失（TREES-IDENTICAL），但 message 顶替=声明失实风险。
13. **共享 index 卫生**（3 案）：`git add` 只加自己写路径的文件；commit 用 pathspec
    （`git commit -- <paths>`）只提交自己路径，**不动他人暂存**；提交遇外源负载/index
    竞争=守窗口重试，不 resort --no-verify。**撞 index.lock 先判活死**（ps 查持锁者）：
    活锁=等待其 COMMIT-OK 后重试（绝不删锁）；确认持锁进程已亡=死锁方可 rm 并留记录。

15. **两层架构纪律**（docs/spec/two-layer-architecture.md，2026-10-15 方法论更正）：
    产品层（域模块 struct+method）**禁裸地址**；地址只活在 memmap 映射层；语义命名走
    data/naming 台账（A/B 级+证据字段）；sub_* 机械名只在 shim/映射层注释回指 bank:addr。
    验证不变式一条不松：行为钉死=结构可换（L2 案在形态迁移中必须持续全绿）。

16. **TDD RED 窗口通报制**（3 案互卡立规）：RED 期合法但共享树 gate 红会卡住**所有人**
    的提交（pre-commit 全量含他人在途）。进 RED 窗口即通报全队（「TDD RED 中，预计 N 分钟」），
    转绿也报；他人遇互卡按「定位层次→转办→等待」，不越界修。

17. **已证等价原则**（docs/methodology.md 资源三层政策，2026-10-15）：refs/ asm 与 ROM
    字节等价已证 ⇒ **静态分析只读 asm**（单元 spec=引 asm 段+注释，不逐指令重解码）；
    ROM 字节仅作抽查佐证；Mesen2 仅三用途（裁判 golden / bug 定位 / 运行态确证）——
    每次场景运行在 commit/账本里带用途标签。违例判据：复审见「重解码表」而无 asm 段引用、
    或 Mesen2 调用无三用途标签。

18. **S5 单元对拍判据**（P28-F1 模型更正案立规，L3 上线前生效）：S5 帧引擎单元的「对拍」步
    = **vs 写点级捕获面**（decomp 场景 trace 可复跑面）——L2 单元有 golden 兜底，S5 单元
    单测与实现同源（模型错×单测绿=同源补偿），捕获面对表是其唯一独立裁判；比对面=写点
    值域/写序/时序，基准面=捕获实测值域。每 S5 单元 commit 附捕获面对表证据。
