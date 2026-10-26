# L4 假对拍工具安全清理清单（Task 3 · H3）

> **结论性质**：逐件**已证实**判定（A 级：grep 机械证据，命令可复现）。
> 本清单=2026-10-26 清理时点的活死账。现状存档 commit（Task 1 `b543a188`）已把
> untracked 全量入库，删除可逆（git 基线在）。**不删** `analysis/_gen/*.lua`
> 旧生成物（gitignored 可再生，删除不在本任务）。

## 判定标准（机械，无自由裁量）

| 判据 | 定义 |
|---|---|
| **活** | 存在代码级消费（`import`/调用/`cargo run --bin`）或测试 import，或=活 bin 被对拍工具驱动 |
| **死** | 全仓 grep 零 `import`/`require`/调用形态、零 markdown-link 形态；仅剩 ①自身 docstring ②docs 历史/分析/计划叙事提及 |
| 不计活引用 | `analysis/`（gitignored 生成物）、`target/`、`tools/mesen2/`、`tools/wla-dx-src/`、`__pycache__/` |

复现命令（每件同式，`<F>`=文件名）：

```sh
grep -rn --binary-files=without-match "<F>" . --exclude-dir=.git --exclude-dir=analysis \
  --exclude-dir=target --exclude-dir=mesen2 --exclude-dir=wla-dx-src --exclude-dir=__pycache__
grep -rn -E "(import|require|dofile|include)\s*\(?\s*['\"]?(<去后缀名>)" --include="*.py" --include="*.rs" --include="*.lua" --include="*.toml" --include="*.sh" .
grep -rn -E "\]\([^)]*<F>" .        # markdown-link 形态
```

## 一、保留（活）——3 件

| 文件 | 活证据（grep 机械实证） |
|---|---|
| `tools/l4_diff_run.py` | L4 真对拍主工具（Rust vs 模拟器）。消费方：`tools/tests/test_expand_schedule.py:29` `import l4_diff_run as l4`、`tools/tests/test_l4_anchors.py`（Task 3 新增锚定测试）、`tools/scenario_run.py:446` 交叉引用（.sched 写入面）；Task 4/5/6 在其上续作（snapshot 对齐/run-all/门禁） |
| `tools/scenario_run.py` | **唯一 DSL 解析器 + 唯一输入展开源**（`parse_scenario`/`expand_schedule`，Task 2 单源纪律）。消费方：`tools/l4_diff_run.py:321` `import_module("scenario_run")`、`tools/tests/test_expand_schedule.py:28` |
| `crates/game-core/src/bin/rust_screenshot.rs` | **活 bin**（FrameEngine→render_frame 真渲染帧，Task 4 续作消费面）。驱动方：`tools/l4_diff_run.py::run_rust` `cargo run -p game-core --bin rust_screenshot`。⚠️ 与删除件 `tools/rust_screenshot.rs` **同名不同物**——本清单只删 tools/ 下 stray 件 |

## 二、删除（死）——10 件（逐件 grep 证据）

**共性证据（10 件全同）**：`import`/`require`/调用形态 **零命中**；markdown-link 形态 **零命中**；
代码文件（`*.py/*.rs/*.lua/*.toml/*.sh`）引用 **零命中**。以下"命中"全部为自身 docstring 或
docs 历史/分析/计划叙事提及（非活引用——不指向运行路径）。

| # | 文件 | 活死 | grep 命中（全部非活） | 死因（已证实） |
|---|---|---|---|---|
| 1 | `tools/l4_auto_diff.py` | **死** | 自身 docstring；`docs/superpowers/plans/2026-10-26-l4-autodiff-completion.md:74`（计划 Delete 清单）；`docs/findings/l4-autodiff-plan-adversarial-review.md:67`（判定「死——假对拍」）；`docs/knowledge/pattern-l4-real-rust-vs-emulator-diff.md:16`（反面教材）；`docs/rounds/r7-porter-review.md:12666,12670,12672,12675,12680`（历史假"PASS 5/5"记录，Task 3 已原地【更正】）；`docs/spec/l4-autodiff-completion.md:77` | **emu×emu 假对拍**（golden(emu)×shots(emu) hash 对比——两侧同一模拟器产物，绿≠Rust 一致） |
| 2 | `tools/l4_rust_vs_emulator.py` | **死** | 自身 docstring；plan:74；`l4-autodiff-plan-adversarial-review.md:68`（判定「死——CLI 已断」）；`r7-porter-review.md:12696,12702`（历史提及）；spec:77 | 旧 CLI 断径（调 `rust_screenshot <scenario> --timeout`，与现行 bin `--schedule <path>` 不符，必然报错） |
| 3 | `tools/rust_screenshot_real.py` | **死** | 自身 docstring；plan:74；findings:69（同断径判定）；spec:78 | 同旧 CLI 断径 wrapper |
| 4 | `tools/rust_screenshot.py` | **死** | 自身 docstring；plan:74；findings:70；spec:79 | 同旧 CLI wrapper（调 `cargo run -- <scenario> --timeout` 旧接口） |
| 5 | `tools/rust_screenshot.rs` | **死（stray）** | 自身文件头；plan:74；findings:71（判定「死——stray 文件，不在 Cargo workspace members」）；spec:79；`pattern-l4-real-rust-vs-emulator-diff.md:17`（裸名历史叙事「曾用 fill_render_state 模拟数据」） | stray 文件，**不在** `Cargo.toml` workspace members（members 仅 `crates/*`，机械实证）；⚠️ 只删 tools/ 下这份，**不碰** `crates/game-core/src/bin/rust_screenshot.rs`（活 bin） |
| 6 | `tools/l4_unified_diff.py` | **死** | 自身 docstring；plan:74；findings:73（golden×shots 换皮 emu×emu）；spec:80 | 假对拍换皮（golden(emu)×shots(emu) hash 对比），无消费方 |
| 7 | `tools/l4_diff_layer.py` | **死** | 自身 docstring；plan:74；findings:74（无消费方）；spec:80 | 对拍层孤立件，grep 无 import |
| 8 | `tools/l4_conclusion_layer.py` | **死** | 自身 docstring；plan:74；findings:74；spec:81 | 结论层孤立件，grep 无 import |
| 9 | `tools/l4_rust_runner.py` | **死** | 自身 docstring；plan:74；findings:74（判定「stub——模拟 Rust 实现执行」）；spec:81 | 假 Rust 执行 stub（不跑真引擎），grep 无 import |
| 10 | `tools/l4_unified_format.py` | **死（语义已并入）** | 自身 docstring；plan:74；findings:53,72（判定「死但语义载体——先并语义再删」）；spec:82 | 第二解析器（`ScenarioDSL`）——**语义已并入 Task 2**：`tools/scenario_run.py:315-318` `snapshot ≡ bundle_load`、`endFrame → 终端采集点`（A 级：代码行 + `tools/tests/test_expand_schedule.py` 语义测试覆盖），删除安全 |

### 裸名 `rust_screenshot.rs` 归属消歧（⚠️ 同名不同物）

以下命中指**活 bin** `crates/game-core/src/bin/rust_screenshot.rs`，**不在删除清单**：
`docs/findings/l4-diff-adversarial-review.md:3,20`、`docs/knowledge/pattern-l4-real-rust-vs-emulator-diff.md:24`、
`docs/superpowers/plans/2026-10-26-l4-autodiff-completion.md:91`、`docs/findings/l4-autodiff-plan-adversarial-review.md:37,68`（显式 `crates/...` 路径或对拍链上下文）。

## 三、删除后复核（Step 6 机械实证）

- 删除后 `import`/调用/markdown-link 形态 grep 对 10 件名 **零命中**（无活引用残留）。
- 删除后全仓命中仅剩：docs 历史/分析/计划叙事（上表逐条列明，**不删历史**——证伪更正文化
  只加标注不抹除记录）+ 本清单自身证据表。
- `python3 -m pytest tools/tests/` 全绿；`python3 gates/cli.py check` PASS。

## 四、不在本任务

| 对象 | 状态 | 处置 |
|---|---|---|
| `analysis/_gen/*.lua`（231 件旧生成物） | gitignored、可由 cases/ 权威重生成 | **不删**（Task 2 裁定：留 Task 3 清理 lane 但按本任务范围豁免；下次全量重生成时自然覆盖） |
| `docs/` 内历史提及（plan/findings/spec/knowledge/rounds） | 历史/分析记录 | **不改不删**；`r7-porter-review.md` 假"PASS 5/5"另案原地【更正】（见该文件 12666 行处） |
