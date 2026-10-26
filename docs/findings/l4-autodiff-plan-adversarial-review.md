# 「完善自动对拍脚本」方案 · 独立对抗性评审（证伪立场）

- 评审对象：实施方《完善自动对拍系统》方案（A 用例源统一+DSL 单一化+清理 → B 批量矩阵 →
  C 门禁+白名单+ratchet → D snapshot 独立子项目；推荐①分阶段）
- 评审角色：remake-verifier + 架构评审，**证伪立场**（不是背书）；方案评审（未实施），只读未改代码
- 日期：2026-10-26
- **verdict：需修订**（方向大体正确——单入口/单解析器/门禁单独 commit 都对；但有 4 处硬伤 +
  若干漏项，按现状实施"补成完整自动对拍系统"的目标**不成立**，只"看起来全面"）

> 证据口径：每条结论附可复现命令或 文件:行。凡标【硬伤】者必须修订后才可开工。

---

## 0. 评审前先钉死的仓库现状（方案描述与实际不符处的根源）

| # | 现状事实 | 证据 |
|---|---|---|
| S1 | **整个 L4 真对拍层是未提交工作树**：`tools/l4_*.py`、`tools/rust_screenshot*`、`crates/game-core/src/bin/`、`cases/`、`docs/findings/l4-diff-adversarial-review.md`、case-001 场景/金面全部 untracked | `git status --short`：`?? cases/`、`?? crates/game-core/src/bin/`、`?? oracle/scenarios/case-001-…txt`、`?? oracle/screens/case-001-…png`；`git ls-files tools` 无任何 `l4_*` |
| S2 | **用例双源且已分叉**：`cases/` 仅 1 个文件且**无任何工具消费它**（grep `cases/` in tools/gates 无命中）；工具链全部读 `oracle/scenarios/`（238 个场景） | `tools/scenario_run.py:87`（`SCENARIO_DIR = ROOT/"oracle"/"scenarios"`）；`grep -rn "cases/" tools/ gates/` 无消费点 |
| S3 | **case-001 已出现"同名两套调度表"**：`cases/` 版（frames 1200，f500 就移动）vs `oracle/scenarios/` 版（frames 1800，等萨姆斯闪烁结束才动），输入时刻表实质不同 | `diff cases/case-001-…txt oracle/scenarios/case-001-…txt` 全段不同（hold 500-550 vs 750-850 等） |
| S4 | **金面已被两套调度混杂污染**：`oracle/screens/` 同时存在 `f00720_after-jump`（旧调度）与 `f00720_samus-ready`（新调度），共 13 个 case-001 金面来自两套不同输入脚本 | `ls oracle/screens/ | grep case-001`（f00550/f00650/f00720_after-jump/f00820/f00950 = 旧；f00720_samus-ready/f00850/f01000/f01220/f01400/f01800 = 新） |
| S5 | **emu 侧存档加载已实现**（MSS bundle_load + 三直映 JSON），**Rust 侧恢复腿已实现**（`restore_mss(ram, wram, regs)`），快进对齐法已实证（102 帧逐字节忠实 + 恢复相位残差显式） | `docs/findings/bundle-first-arbitrary-start.md` §1–2（`bundle captured … mss=11915B` + 三直映 JSON；`restore_mss 已装`；`102 帧逐字节忠实`）；`crates/game-core/src/frame/memory.rs:142`；`crates/game-oracle/src/bin/l3-dump.rs:96-119` |
| S6 | `docs/knowledge/gap-load-snapshot.md` 声称存档四步"均待定义"——**与 S5 直接矛盾**（陈旧文档未更正） | `gap-load-snapshot.md`「①②③④（待定义）」vs `bundle-first-arbitrary-start.md`（2026-10-16 全链机制落成） |

---

## 1. 分解是否合理（A/B/C/D 顺序与拆分）

**结论：顺序合理，拆分口径不成立——D 的推迟必须显式降级"完整"承诺，且方案对 D 的成本估计建立在陈旧文档上。**

- ✅ **A→B→C 顺序正确**（先单一用例源/单解析器才能批量；先有批量数据才能立门禁与 ratchet；
  门禁单独 commit 是纪律明文）。清理放 A 也对——但需安全化（见 §3）。
- ⚠️ **D 拆独立子项目本身可以是合理降风险，但当前方案里它是"基于失实前提的推迟"**：
  - 方案称 D 复用 gap-load-snapshot.md「4 步均待定义」——**事实错误**（S5/S6）。emu 加载
    （`bundle_load`，`tools/scenario_run.py:303-307`）与 Rust 加载（`restore_mss`）都已落
    地并实证过快进对齐。真正剩的是：① L4 消费面（`rust_screenshot` bin 目前只收
    `--schedule`，`crates/game-core/src/bin/rust_screenshot.rs:18`，不收 bundle）② 对齐
    验收固化。这远小于"格式→emu→Rust→对齐 4 步全待定义"的工程量。
  - **【硬伤 H1】"完整自动对拍系统"口径不成立**：对拍用例三要素之②就是待加载存档
    （`cases/case-001-…txt:4` 明文「对拍用例 = ① 输入脚本 + ② 待加载存档 + ③ 对拍时长」）。
    D 不落地 = 只能测"从 boot 的一幕"（gap-load-snapshot.md 自己写明影响：「无法测试特定
    游戏状态」）。方案若以"完整"为验收口径，就是把核心缺口（②）从分母里拿掉再宣布满分。
    **修正**：要么把 D0（L4 消费 bundle + 对齐验收）并入 A/C 交付线，要么把顶层目标改写为
    「完整自动对拍框架（存档维度显式缺口记账）」，且 B 矩阵必须为②存档维度留显式
    blocked/NA 行——不许隐没。
  - 用例三要素的缺口今天已经被 `snapshot none` 占位记账（cases/case-001:8），方案至少要
    保住这条记账线不被清理动作抹掉。

## 2. 是否守住纪律

| 纪律 | 判定 | 证据与细节 |
|---|---|---|
| DSL 语义只在 tools/scenario_run.py 解析（无第二解析器） | ✅ 方向对，⚠️ 不彻底 | 废 `l4_unified_format.py::ScenarioDSL`（tools/l4_unified_format.py:32-108）正确——它确是第二解析器。**但方案只砍了 parser，没砍第二套"语义"**：`tools/l4_diff_run.py:48-91 build_schedule` 重新实现了 press/hold/press_len/对向键消解，与 `tools/scenario_capture.lua:84-91` 双实现。两侧输入（emu 走 Lua 展开、Rust 走 build_schedule 展开）若语义漂移 = **两边跑的不是同一输入 = 假对拍**。【硬伤 H2】修正：把调度展开（press_len/冲突消解）并入 scenario_run（或从 scenario_run 生成的 PLAN 表直接派生调度表），两侧消费同一展开产物。 |
| 废第二解析器前先并语义 | ⚠️ 方案提了"snapshot/endFrame 并入 parse_scenario"，但**语义未定义就并**是危险的 | `parse_scenario` 今日不认识 `snapshot`/`endFrame`（ops 全表：tools/scenario_run.py:178-377；未知指令 `raise err(f"未知 DSL 指令")` :378-379）——`cases/case-001-…txt` **今天喂给唯一解析器会直接报错**。且 `snapshot` 与既有 `bundle_load`（:303-307）是同语义两名字；`endFrame` 在 scenario_run 语义里=隐式 frames 终点（:26、`sample_point` 注 :310-312），在 l4_unified_format 里=独立"对拍点"。语义三选一（alias/新语义/拒绝）没定就搬关键字 = AGENTS §3.5「不理解机制前不动解析器」。修正：先落语义决议（建议 `snapshot` = `bundle_load` 的别名或直接废 `snapshot` 关键字统一用 `bundle_load`；`endFrame` 显式校验 ≤ frames 且语义=对拍点），再并入，再删旧文件。 |
| 门禁改动单独 commit | ✅ | 方案 C 明文「单独 commit」，符合 AGENTS §2 gates 行/§9。 |
| ratchet 只升不降 | ✅ 方向，⚠️ 口径有掩蔽面 | "一致率 ratchet"若为**聚合百分比**，则 9/10→9/10（换一个坏）不触发回退——正是 verifier 纪律禁止的"整体相似度掩盖局部回退"。修正：ratchet 锚定**逐用例逐采集点的 match 集合**（+blocked 计数只减不增），聚合率仅作展示。另：分母含不含 blocked 必须定义（见 §6）。基线存放/更新复用 `data/status/baselines/baseline.json` + `tools/baseline_update.py`（现状 metrics 无 L4 项——新增项需写 commit 说明，`decision-baseline-raise-policy.md` 在案）。 |
| 视觉白名单=唯一例外 | ✅ 方向，⚠️ 例外面在增殖 | 白名单 `gates/visual_whitelist.json` 现为空（schema 1，逐条需用户确认）。但注意 `tools/diff_visual.py:127-134` 已有 `data/status/visual_gap_map.json`「账内容忍」，L2 有 gap_map TOLERATED（gates/cli.py:805-832）。方案新增的 blocked/TOLERATED 通道必须沿用同一登记纪律（逐条理由+退出路线+显式计数，"不阻塞≠绿"），**不得新增第四种静默容忍面**。 |

## 3. 清理是否安全（哪些活哪些死）

逐文件核对（`grep -rn` 全仓 .py/.md/.rs/.toml/.json 全量引用面）：

| 文件 | 判定 | 引用证据（除自身与历史文档外） |
|---|---|---|
| `tools/l4_diff_run.py` | **活**（真对拍核心，方案单一入口） | docs/findings/l4-diff-adversarial-review.md、pattern-l4-real-rust-vs-emulator-diff.md |
| `tools/scenario_run.py` | **活**（核心：DSL 唯一解析器 + emu 采集驱动） | l4_diff_run、diff_traces、probe_infra、decode_sweep_dyn 等多方消费 |
| `tools/l4_auto_diff.py` | **死——假对拍**（只跑一次 scenario_run，golden(emu)×shots(emu) hash 对比） | `tools/l4_auto_diff.py:84-110`；pattern 文档点名反面教材；仅 `docs/rounds/r7-porter-review.md:12666+` 历史提及 |
| `tools/l4_rust_vs_emulator.py` | **死——CLI 已断**（调 `rust_screenshot <scenario> --timeout`，与现行 bin `--schedule <path>` 不符，必然报错） | `tools/l4_rust_vs_emulator.py:99` vs `crates/game-core/src/bin/rust_screenshot.rs:18` |
| `tools/rust_screenshot_real.py` | **死——同旧 CLI** | `tools/rust_screenshot_real.py:64`（同上断径） |
| `tools/rust_screenshot.py` | **死——同旧 CLI wrapper** | `tools/rust_screenshot.py:60-88` |
| `tools/rust_screenshot.rs` | **死——stray 文件**（不在 Cargo workspace members） | `Cargo.toml:3-5` members 仅 crates/* |
| `tools/l4_unified_format.py` | **死但语义载体**——必须先并语义入 scenario_run 再删（方案顺序正确） | 无代码消费方（grep 无 import）；唯 `snapshot`/`endFrame` 语义在此 |
| `tools/l4_unified_diff.py` | **死**（golden×shots hash 对比，仍是 emu×emu 换皮） | `tools/l4_unified_diff.py:34-57`；无消费方 |
| `tools/l4_diff_layer.py` / `tools/l4_conclusion_layer.py` / `tools/l4_rust_runner.py` | **死**（无消费方；l4_rust_runner 甚至是「模拟 Rust 实现执行」stub） | `tools/l4_rust_runner.py:28-31`；grep 无 import |

**【硬伤 H3】清理安全两问方案都没答**：

1. **全部待删文件都是 untracked（S1）——删除=永久销毁，git 无法恢复**。修正：先落一个
   "现状存档 commit"（至少 l4_diff_run + rust_screenshot bin + case-001 场景/金面入库），
   再单独 commit 删死件；或先移 `analysis/quarantine/` 一个周期后删。
2. **方案用"等"字收尾，清单不闭合**。修正：删除清单必须逐文件列名 + 逐条 grep 证据
   （本节表格可直接采用），禁止"等"。
3. 附带文档义务（AGENTS 证伪更正文化）：`docs/rounds/r7-porter-review.md:12666` 把
   `l4_auto_diff` 的「PASS（5/5 通过）」当作对拍成果记录——那是假对拍的假绿。清理 commit
   应在该处**原地标注**【更正】，不删历史主张。

## 4. 有无漏项（对照 6 缺口 + 新漏项）

6 缺口覆盖核对：存档（→D，口径问题见 §1）✓、用例源（→A）✓、批量（→B）✓、门禁白名单
（→C）✓、游戏逻辑（显式排除，独立任务）✓、清理（→A）✓——**列是全的，但每列都有坑**。
新增漏项（方案未提）：

1. **【硬伤 H4】oracle_sha256 / scenario_sha256 锚定缺失**：L3 已有成熟口径
   （`tools/diff_traces.py:69-76,452` 校验 `oracle_sha256`/`scenario_sha256`；
   `oracle/traces/l3_manifest.json` 逐案落锚），而 L4 报告
   （`tools/l4_diff_run.py` do_diff 产物 `shots_rust/<name>.l4report.json`）**只有
   pass/fail/pixel 数，无任何锚**。叠加 `--skip-emu/--skip-rust` 静默复用旧截图
   （`tools/l4_diff_run.py:265-268`）与 S4 的金面混杂实证——**同名不同调度的旧产物可以被
   静默拿来对拍出"绿"**。修正：l4report 与 matrix 必须锚定 scenario_sha256 + ROM sha256 +
   金面/截图 sha；复用旧产物前校验调度表 hash，不符即拒绝（fail-loud）。
2. **stale 产物清理**：S4 的 13 个混合金面必须随 A 阶段重锚定/清理（保留哪套调度要先裁定：
   现行 `oracle/scenarios/` 版含"等萨姆斯闪烁"是更正确的考题，cases/ 版 f500 移动是过时假设）。
3. **用例清单与完备性矩阵的派生关系**（AGENTS §7「任务卡由完备性矩阵派生」、D7）：B 只从
   cases/* 枚举，case 清单本身的完备性无声明。修正：矩阵按 D7 口径记"已登记场景清单"分母，
   绿≠完整照旧显式打印。
4. **CI 集成口径**：`gates check`（pre-commit/CI 总闸）**不含** visual/trace/diff 三兄弟
   （GATES 表 `gates/cli.py:1307-1324`：visual 归 `visual` verb、trace 归 `trace`、
   subroutine 归 `all`）。新 L4 批量门禁挂哪个 verb、CI 耗时预算（Xvfb+Mesen+cargo 逐案
   跑 238 场景 × 采集点）方案未提。至少要写明：默认挂 `visual`/`all`，CI 用例子集+全量定时。
5. **回归基线存储**：matrix.json/md 落哪（建议 `data/status/`，AGENTS §2 表）、谁更新
   （`tools/baseline_update.py` 复用）、commit 说明义务——方案只说"matrix.json+md"。
6. **多 ROM/多区域**：多 ROM/PAL/FDS **不是**漏项——D6 已锁死保真期只做 NES_NTSC
   （AGENTS §0 D6）；但"游戏内区域"（Brinstar/Norfair/Tourian/…）的用例覆盖要接第 3 条
   的分母声明，否则批量矩阵绿了也只覆盖 boot-title 一幕。
7. **采样点盲区**：矩阵只按采集点判定，采集点之间是盲区（`docs/knowledge/gotcha-sampling-point-is-blind-spot.md`
   在案；l4-diff 复核 F5 正是靠 f420==f720 的逐字节比对才抓到 idle 冻结）。建议矩阵对
   关键过渡帧加分段连续帧采样或与 L3 trace 面交叉验证，至少在用例设计准则里写明。

## 5. snapshot 路线裁定（中立 JSON vs 快进 vs 从 boot 全跑）

**裁定：三选一是假分叉——仓库已用"混合路线"实证过正确答案；"中立 JSON 游戏状态快照"作
主格式是错误方向。**

- **emu 侧**：MSS bundle（`bundle_load`，全机态含 PPU/mapper 内部）= 真源，已实证
  （`bundle load f=1 bytes=11915 ok=true`，bundle-first-arbitrary-start.md §1②）。
- **Rust 侧**：三直映 JSON（ram/wram/regs）→ `restore_mss`（`crates/game-core/src/frame/memory.rs:142`）
  已实证生效（l3-dump `--bundle` 通道）。
- **对齐**：快进法（bundle-ff）就是对齐验证器——已证「102 帧逐字节忠实 + 首分歧=恢复相位
  残差（~100 帧后漂移，savestate 重放经典类）」。

据此逐项回答方案的三个候选：

1. **中立 JSON 游戏状态快照（作主格式）——不取**：它要么是有损（三直映不含 PPU/mapper
   内部态，Rust 恢复态与 emu MSS 恢复态在 JSON 外字段上必然漂移），要么就得发明第二套
   "游戏状态"语义模型（= 状态面的第二解析器，与单 DSL 解析器同一种病，字段双维护漂移）。
2. **快进法——取，但只作"对齐验证 + 短程取态"**：单一真相源（输入脚本）、零格式风险、
   天然非假对拍（同一执行径起点）；代价是深程状态（如 Norfair 中段）重放长。深程取态用
   bundle，别用快进硬扛。
3. **保持从 boot 全跑——仅作为 bundle 不可用时的兜底**，不是长期答案（它就是②存档维度
   缺失的现状）。

**Rust 侧不假对拍的验收判据**（写进 D0）：restore 后起跑的前 N 帧必须与 emu 快进序列逐帧
逐字节对齐（bundle-ff 口径），恢复相位残差窗要么测到 0，要么按 BUG/已知偏差显式登记退出
路线——**不许把"加载成功"当"对齐成功"**（gap-load-snapshot.md ④步必须是 diff 用例，不是
打印 ok）。

另：`gap-load-snapshot.md` 的四步"待定义"陈述已被 S5 证伪，按 AGENTS §3.6 证伪更正文化
**原地标注日期更正**，否则 D 阶段会照着错误前提重新发明已有机制。

## 6. 风险与验收判据（假绿面清单）

| 假绿面 | 触发条件 | 必须的修正 |
|---|---|---|
| blocked 计入通过 | 矩阵"一致率"=match/(match+diff)（排除 blocked）或 blocked 当 pass | 分母含 blocked（blocked=未验证，不是通过）；blocked 数只减不增进 ratchet；现状 `l4_diff_run.py:270-280` exit code 已把 blocked 计入 fail（`rep["fail"]=len-npass`）——**批量层必须保持这个口径，不许重算口径** |
| 一致率聚合 ratchet | 换一个坏的补一个好的，率不降 | ratchet 锚逐采集点 match 集合（§2） |
| 旧考题产物冒充 | `--skip-emu/--skip-rust` + 同名不同调度（S4 实证） | scenario_sha256 锚 + 不符拒跑（H4） |
| 金面混杂 | 13 个混合金面未清理就上批量 | A 阶段重锚定（§4.2） |
| 采集点稀疏 | 静止画面上采集点全绿（idle 冻结只在 f420==f720 这类连续对比现形） | 用例设计准则+过渡帧采样（§4.7） |
| 门禁吞明细 | runner 失败被 TOLERATED 无登记 | 沿用 l2_gap_map 登记制：逐条理由+BUG/缺口编号+退出路线，SUMMARY 显式计数（gates/cli.py:805-832 现行口径照抄） |

**逐阶段硬判据（方案缺，建议补为验收门）**：

- A：`grep -rn "ScenarioDSL\|l4_unified_format" tools/` 零命中；`python3 tools/scenario_run.py run-all` 全绿；
  `cases/` 与 `oracle/scenarios/` 单源裁定落 commit（含 case-001 二选一 + 金面重锚）；
  删除清单逐文件附 grep 证据；全部清理动作在"现状存档 commit"之后。
- B：matrix 含三态且 blocked≠pass；逐案锚 scenario_sha256/oracle_sha256；matrix.json 落
  data/status/ 并入库。
- C：`python3 gates/cli.py visual|all` 实跑贴输出；门禁改动单独 commit + 基线影响说明；
  白名单保持零条（或逐条用户确认记录）。
- D0：bundle 加载后快进对齐 diff 用例过（残差=0 或登记）；`rust_screenshot --bundle` 消费面
  实测；gap-load-snapshot.md 更正落盘。

---

## 7. 修正建议汇总

**必须（不改不许开工）**：
1. H1 顶层口径降级或 D0 前置：②存档维度要么进交付线，要么显式记账，"完整"字样不得覆盖它。
2. H2 输入语义单一化扩到调度展开层（build_schedule ↔ scenario_capture.lua 双实现合并）。
3. H3 清理安全化：先存档 commit 再删；删除清单闭合（本文 §3 表可直接用）；r7 假绿记录原地更正。
4. H4 L4 锚定：oracle_sha256/scenario_sha256/ROM sha 进 l4report 与 matrix；`--skip-*` 复用前
   校验调度 hash；清理 S4 混合金面。
5. D 前提更正：gap-load-snapshot.md 原地更正；D 改写为"L4 消费面+对齐验收"（机制已立），
   中立 JSON 主格式方案否决，走 MSS+三直映+快进对齐的既有混合路线。
6. B/C 的假绿四件套：blocked≠pass、ratchet 逐点锚、分母定义、TOLERATED 登记制。

**建议**：用例分母接完备性矩阵（D7）；CI verb/耗时口径写明；matrix 存放 data/status/ +
baseline_update.py 复用；游戏内区域覆盖进分母声明；采样点盲区进用例设计准则。

**可复现命令**：
```bash
git status --short && git ls-files tools | grep l4 || echo "l4_* 全部未入库"
diff cases/case-001-game-start-samus-interaction.txt oracle/scenarios/case-001-game-start-samus-interaction.txt
ls oracle/screens/ | grep case-001 | sort          # 13 金面混两套调度
grep -rn "cases/" tools/ gates/                     # 无消费方
sed -n '178,380p' tools/scenario_run.py             # 无 snapshot/endFrame；未知指令 raise
grep -rn "restore_mss" crates/ && sed -n '1,50p' docs/findings/bundle-first-arbitrary-start.md
grep -n "GATES\|visual-diff\|trace-diff" gates/cli.py | sed -n 1,10p
```
