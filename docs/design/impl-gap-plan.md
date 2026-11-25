# 实现缺口评估与串行滚动实现/验证计划

> 生成日期 2026-11-25；采样 HEAD `6a48c05d`。本文件只做评估与派卡依据，不含代码改动。
> 所有数字均由下方命令在采样时点**原样输出**，不做二次相加（系统表除外，见 §2.1 的机械分组规则）。

## 0. 一页结论

1. **缺口规模**：例程分母 1135 条，已端口 **380**、候选 **164**、未端口 **591**；缺口集中在 `prg7_engine`（338 条未端口）与 `areas_common`（64 条）。
2. **内存面**：`nes-map` 2272 B = Owner **1348** + Unmodeled **387** + Undeclared **537**（View 23 B 已含在 Owner 面内，`view_only_bytes = 0`，**不重复计入**）；未端口工作量与 Unmodeled/Undeclared 高度重合（387+537=924 B）。
3. **外部可观测面**：L4 像素对拍 **171/172**；TAS 整片 **120540/120540 帧已回放**，真实首分歧 **f2084 `$0415`**（槽 1 `Ens.data05` bit2，emu `0x1E` / rust `0x1A`）。
4. **推进方式（Human 定，替换多 lane 并行）**：**单 worktree 串行滚动** —— 一次一张实现卡，走 `../wt-roll` 隔离，PMO cherry-pick 回主线并在主线复跑门禁后交验。
5. **最快推后 f2084 的优先序**：敌人槽 `data05` 位语义（#52 面已声明，语义未考）→ 敌人公共更新例程（`areas_common` 64 条未端口）→ 引擎帧账（`prg7_engine` 中与 `FrameCount`/等待相关的子集）。

## 1. 数字基线（命令 + 采样时点 + HEAD）

| 指标 | 读数 | 取数命令 |
|---|---|---|
| 例程分母 | 1135 = 380 ported + 164 candidate + 591 unported | `python3 tools/routine_inventory.py --stats` |
| 内存四面 | 2272 = 1348 Owner + 387 Unmodeled + 537 Undeclared（View 23 B 与 Owner 面重叠，`only-view = 0`，不重复计入） | `python3 tools/nes_map_check.py --list` |
| TAS 整片 | 120540 / 120540 帧；首分歧 f2084 `$0415` | `docs/findings/tas-100-round2.md`（回放产物在 `analysis/`） |
| L4 像素对拍 | match **171** / diff **1** / blocked 0（172 格） | `python3 tools/l4_diff_run.py run-all --skip-emu` |
| 门禁 | PASS=24 FAIL=0 SKIP=3 TOLERATED=1 | `python3 gates/cli.py check` |

**口径纪律**：`candidate` 只代表「Rust 侧有同名符号」，**未按 asm 逐条复核**，不得计入已端口（此坑在 #7 复核时踩过一次）。`unported` 才是硬缺口。

## 2. 缺口分类（按系统）

### 2.1 机械分组规则（可复算）

`routine_inventory --stats` 按 asm 文件分组输出 26 行；本表把 26 行按**固定映射**归并为 5 个系统（映射写死在生成脚本里，见本节末伪码），因此合计必然等于 1135/380/164/591：

```
标题/密码界面 ← prg0_title
声音          ← sound_engine
世界/房间     ← areas_common, prg1_brinstar, prg2_norfair, prg3_tourian, prg4_kraid, prg5_ridley
敌人          ← crawler dragon kraid metroid multiviola pipe_bug polyp ridley rinka rio ripper sidehopper skree squeept swooper waver
引擎/主循环   ← prg7_engine, reset
```

| 系统 | 例程 | 已端口 | 候选 | 未端口 | 分母来源（asm 文件） |
|---|---|---|---|---|---|
| 标题/密码界面 | 103 | 37 | 7 | 59 | prg0_title |
| 声音 | 132 | 0 | 78 | 54 | sound_engine |
| 世界/房间 | 163 | 42 | 19 | 102 | areas_common, prg1_brinstar, prg2_norfair, prg3_tourian, prg4_kraid, prg5_ridley |
| 敌人 | 72 | 30 | 5 | 37 | crawler, dragon, kraid, metroid, multiviola, pipe_bug, polyp, ridley, rinka, rio, ripper, sidehopper, skree, squeept, swooper, waver |
| 引擎/主循环 | 665 | 271 | 55 | 339 | prg7_engine, reset |
| **合计** | **1135** | **380** | **164** | **591** | 26 个 asm 分组 |

**说明**：`结算`（ending）没有独立 asm 分组——结局/字幕逻辑在 `prg7_engine` 与 `prg0_title` 内，故本表不单列，避免制造空类（卡面七类中的「结算」归入「引擎/主循环」与「标题/密码界面」两行，不另立类别）。

**说明 2**：`玩家`（Samus）逻辑同样没有独立 asm 分组——其例程按 label 分散在 `prg7_engine`（81 条）、`sound_engine`（8）、`areas_common`（5）、`prg0_title`（5）、`prg3_tourian`（3）、`metroid`（1）等分组内，**已计入上表对应行**，故 §2.1 不单列条数以免重复计数；其字节面与可观测面见 §2.2「玩家」行。上述 103 条是按 label 关键字（`samus|player`）匹配的**参考值，不是分母**，不参与 §2.1 的 1135 合计。

### 2.2 每类的字节面与可观测面

| 系统 | 相关内存面（`nes-map`） | 像素可观测（L4） | trace 可观测（frames.jsonl） | 备注 |
|---|---|---|---|---|
| 敌人 | `Enemy` 192 B + `ObjectSlots` 256 B（已声明，语义部分 C 级）+ `Explosion` 32 B | 是（多数敌人无独立采集点） | 是 | f2084 正落在 `Ens.data05` bit2 |
| 玩家 | `SamusState`（已声明）+ `ObjectGlobals` | 是（主采集面） | 是 | 现有 L4 覆盖最好的一类 |
| 世界/房间 | `Room`/`Camera`/`ScrollBlocks`/`RoomSpriteTables`/`VramBuffers` | 是（scrollseam / crawlprobe 用例） | 是 | `areas_common` 64 条未端口是最大单点 |
| 声音 | `SoundRam`(`$0600-$068D`) + `SoundInitFlags` | **否**（无像素面） | 仅 APU 写入 trace | 只能靠寄存器写入序列验，不能靠截图 |
| 标题/密码 | `VramBuffers` + `SpareScratch`（C 级） | 是（password 用例 14 格） | 是 | 已有 `password-dark-window` 系列定谳 |
| 引擎/帧账 | 面内 `$0000-$00FF` 各种 | 间接（时序错会显成相位差） | 是（`FrameCount`/NMI） | 分隔帧预算与逻辑预算的边界 |

## 3. 推进方式：单 worktree 串行滚动（Human 定，替代多 lane 并行）

> **本节口径**：原计划的「2–3 条 lane 并行」已由 Human 否决，替换为**单 worktree 串行滚动**。所有「并行」措辞在此一律改称「串行滚动」。**原卡面要求的「每条 lane 的成员 / writeScopes / blockedBy 表」随之取消**——改为派卡时逐卡在卡面填写（本文件只定义队列规则与写面隔离原则，不维护 lane 名单），这是有意替换、不是漏写。

### 3.1 协议（每条实现卡的开工/收口标准）

1. **固定一个 worktree**：`../wt-roll`，分支 `lane/roll`，每次从**最新主线 HEAD** 新建或 `reset --hard`；**同一时刻只有一张实现卡在里面**。
2. **谁进去**：凡改 `crates/` 的实现卡排队进 worktree，一次一人；`docs/spec`、`docs/findings` 类卡**不进 worktree**，继续在主线串行提交（不碰 `crates/`，不会冲突）。
3. **lane 内验证**：只跑本卡验收命令（`cargo test -p game-core`、单案 `tools/l4_diff_run.py run <case>`、必要的 `--skip-emu` 子集）；**不跑全量 L4、不占 Mesen 单实例**（要跑就排队，Mesen 单实例是硬约束）；不生成或修改 `data/status/*` 账本与 `gates/`。
4. **收口**：lane 分支上按逻辑拆 commit（一个 commit 一个可回滚改动），submit 时贴「分支名 + commit sha 列表 + lane 内验收输出 + 已知未验证项」；**开发者不自行 pick**。
5. **PMO pick**：主线干净时 `git cherry-pick`（按 commit 顺序）→ 主线复跑 `gates/cli.py check`（PASS24/0/3/1）+ 本卡验收命令 + `run-all` 不新增 → 交对影验**主线**（不是 worktree）。
6. **通过后**：worktree `reset --hard` 到最新 main（必要时删掉重建），才派下一张实现卡；不通过则 `git revert` 该 pick，worktree 留着继续修。
7. **不进 worktree 的工作**：`data/status/`（账本/基线/nes_map）、`gates/`、`rom/`、`analysis/traces` 采集 —— 永远在主线串行，由 PMO 统一做。

### 3.2 已知成本（写进来而不是省略）

- worktree 的 `target/` 不共享：每次 reset 后首轮 `cargo build` 要重编。建议固定 `CARGO_TARGET_DIR=../wt-roll-target` 复用（该目录不入库）。
- 串行滚动意味着**同一时刻只有一张实现卡**：吞吐由单卡时长决定，因此 §5 的批次出口指标必须按「卡」而不是「时间」定义。

### 3.3 每张卡必须写明「与 TAS 首分歧 f2084 的关系」

派卡时在卡面强制一栏，三选一（互斥）：

- **推后型**：预期把 f2084 往后推，卡面要写「预期新首分歧 ≥ fXXXX」与验证命令；
- **铺路型**：不直接触碰 f2084，但补齐它依赖的前置（如 `data05` 位语义、敌人公共更新）；
- **无关型**：既不影响也不铺垫（如声音寄存器封装），需说明为什么仍值得做。

不写这一栏的卡不进队列。

## 4. 验证前置（每类卡的验收命令 + 期望数字）

| 卡类型 | 验收命令 | 期望数字 | 备注 |
|---|---|---|---|
| `crates/game-core` 逻辑 | `cargo test -p game-core` | 全绿，且不少于当前基线（`gates` 的 `rust-tests` 行为准） | 单 crate 与 workspace 口径不同，两个数都贴 |
| 渲染/时序 | `python3 tools/l4_diff_run.py run cases/<case>.txt` | 目标格 0 px，其它格不回退 | 跑前确认无并发 Mesen |
| 采集/时序变更 | `python3 tools/l4_diff_run.py run-all --skip-emu` | match ≥171 / 172，无新增格 | 全量 emu 腿排队串行 |
| 内存声明 | `python3 tools/nes_map_check.py --list` | `Owner + Unmodeled + Undeclared == 2272`（View 与 Owner 重叠、only-view=0，不参与求和），Undeclared 只降不升 | 账本用 `--write` 生成，不手改 |
| 门禁/工具 | `python3 gates/cli.py check` + `selftest` | PASS=24 FAIL=0 SKIP=3 TOLERATED=1；selftest 全过 | 改门禁须单独 commit |
| 文档 | 同上（`docs-links`/`doc-sync` 门禁项） | 链接全有效、索引同步 | — |

### 4.1 「实现完成但验证未跑」的卡面标记

- submit 时 **summary 首行必须写** `验证状态: 未跑 / 部分跑 / 全跑`，未跑/部分跑要逐条列出缺哪条命令。
- 卡面状态只允许 `pending_acceptance`（等验），**禁止**在 summary 里写「已完成」而不给命令输出；PMO 提交时按 summary 的验证状态决定是否直接 pick。
- 对影的抽样复核方式：不看 worktree，只看**pick 后的主线**，随机抽 1–2 条验收命令复跑 + 读 diff（不阻塞实现卡队列）。

## 5. 三批路线（串行滚动，按队列推进）

| 批次 | 目标 | 出口指标（可量化） | 回滚/退出条件 |
|---|---|---|---|
| **B1 敌人语义铺路** | 把 `Ens` 关键字段的位语义考到 A/B 级（`data05` bit2、`specialAttribs`、`type`），并把 `areas_common` 未端口里与 f2084 相关的公共更新例程端口 | f2084 **后移**（新首分歧 > 2084）；`unported` 从 591 下降到 ≤560；L4 仍 ≥171/172 | 新首分歧前移或 L4 新增 diff ⇒ 回退该批最后一次 pick |
| **B2 世界/房间补齐** | `areas_common` 剩余未端口 + 房间过渡/滚动相关 | `unported` ≤500；`case-elev-norfair-transition`/scrollseam 用例保持全 0 | 任一 L4 格回退 ⇒ 停止并定位层 |
| **B3 引擎帧账与声音** | `prg7_engine` 中帧预算相关子集 + 声音寄存器封装（无像素面，靠 APU 写入 trace 验） | `Unmodeled` 387 → ≤300；`check` 保持 PASS24 | 时序类改动使 f2084 前移 ⇒ 回退 |

**必须串行的部分**：同目录改动（`crates/game-core/src/<dir>/`）、同账本改动（`data/status/nes_map.json`）、门禁改动（`gates/`）——这三类不允许两卡同时在途。

## 6. 待决与风险

1. **`candidate 164` 的口径**：目前只代表同名符号，未按 asm 复核；建议单独出一张「candidate 复核」卡，否则这个数会长期虚高。
2. **声音类无像素面**：验收只能靠 APU 寄存器写入序列，需要先有一份 §4 之外的「APU trace 契约」文档，否则声音卡无法定义出口指标。
3. **Mesen 单实例**是串行滚动的隐形瓶颈：全量 L4 必须排队，建议把「何时跑全量」写进批次出口而不是每卡。
4. **`SpareScratch` 155 B 仍是 C 级**（#52 交付）：若 f2084 收敛后发现与之相关，需回头考证。
