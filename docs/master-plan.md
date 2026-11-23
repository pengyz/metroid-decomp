# master-plan — Metroid（NES/6502）Rust 重制总规划

> 纪律以 [AGENTS.md](../AGENTS.md) 为准；管线以 [ai-decompile-pipeline.md](ai-decompile-pipeline.md) 为准。
> **长线目标与收敛回路以 [north-star.md](north-star.md) 为准**（稳定定义）；本文只排阶段与优先级，随进度滚动更新。

## 阶段状态（2026-11-23 实测口径）

> 采样：2026-11-23 21:00 前后，采样时 HEAD `495426ae`（其后 HEAD 前进到 `915763b9`，只新增 `docs/design/fc-memory-projection-architecture.md`，未触及代码/门禁/基线）。每行给出取证命令或仓库指针；未实测的事项标「未验证」，不写成已完成。旧值与取证对照见文末「文档口径同步记录（2026-11-23）」。

| 阶段 | 内容 | 状态 |
|---|---|---|
| S0 ROM 取证 | ROM 校验（sha256/MD5/CRC32/headerless 四锚）、m1disasm 等价性、向量/复位抽检 | ✔ 完成（findings/rom-disasm-equivalence.md） |
| S0.5 纪律骨架 | AGENTS.md / prompts / gates / 数据模板迁移（gunnac_remake → 本仓库） | ✔ 完成（`gates/` 现 27 项门禁） |
| S1 静态投影 | IDA 8-bank export → seed 归一化冻结（`reference/seed/`，重生成须 commit 说明） | ✔ 完成（c416d34；对抗复核 pass，rounds/r1） |
| S2 动态证据 | 已完成：Mesen2 oracle 迁移、L2 harness（r3 复审 pass）、首场景 trace；L3 **oracle 腿**已对锚两场景（`l3-sc-shoot-kill` 1183 帧、`l3-sc-first-room-full` 3177 帧，`gates/cli.py status` trace-diff）。未完成：**game-core 腿未启用**（S5 生成 `analysis/dump/*.frames.jsonl` 后自动启用）；`l3-samus-move3-oracle-determinism` oracle 缓存 0 字节 → TOLERATED；指令级覆盖语义根治（rounds/r2 F1/N3：CDL / Mesen Trace Logger / FCEUX 三选一）未定 | 🔨 进行中 |
| S3 语义考证 | 命名台账 738 条（naming-ledger，WARN 第 632–738 行缺 confidence）；typed 域迁移批次 1–3 已落（`$0300` status / `$0308-$0309-$0314-$0315` i8 / `$030C-$030E` Position，见 findings/typed-domain-migration.md；批次 4–5 待做）；D 面字段级分母已出（completeness faces data 193）；密码屏：`0x18 EnterPassword` 深路已臂化（`crates/game-core/src/title/mod.rs:1495` 批6b 专件，commit `4db83c43`；dispatch 测试 `title/mod.rs:1532` 断言 `unported==None`），L4 `case-password-justin` 余 f700(23px)/f1100(42px) 两格视觉残差——属**已端口面的残差**而非未端口（承接卡 #10；`title/mod.rs:718` 的「待 shell 组合点后续批」为过期注释，待随下批清理） | 🔨 进行中 |
| S4 规格书 | `docs/spec/` 30 篇（samus-movement / samus-action-state-machine / two-layer-architecture / entity-ownership-and-nesmap / l3-alignment-contract / l4-acceptance-fix-plan / oracle-verification-chain / completeness …）；完备性覆盖 57 / 932 格（faces：behavior 247 / symbol 492 / data 193 / rule 0，`data/status/completeness_matrix.json`）；R 面规则分母仍为 0（声明性完备，非机械证明） | 🔨 进行中 |
| S5 Rust 移植 | `crates/game-core` 49,071 行（不含 bin：`find crates/game-core/src -name '*.rs' -not -path '*/bin/*' \| xargs cat \| wc -l`）；例程分母 code 1135 / ported 380 / candidate 152 / **unported 603**（`python3 tools/routine_inventory.py --stats`：ported=已按 asm 复核等价，candidate=仅命中符号名、未复核；1135 = 380+152+603）；rust 测试 1,087 通过（基线 1,087）；实体化与 FC 内存投影契约补齐并行（docs/design/fc-memory-projection-architecture.md） | 🔨 进行中 |
| S6 差分验证 | L3 见 S2（oracle 腿已验证、game-core 腿未启）；L4：21 案 / 172 格 · match 166 · diff 6 · blocked 0（`python3 tools/l4_diff_run.py run-all`，2026-11-23 21:15 独跑，采样时 HEAD `495426ae`，产物 `shots_rust/matrix.json`）；visual-diff 慢层 SKIP、未跑（`gates/cli.py status`）；L4 门禁属慢层不进 hook | 🔨 进行中 |

## 仓库级待办（设计复审暴露，暂无归属阶段）

- [ ] **门禁 FAIL 归属线索**（port2 提议，低优先）：FAIL 文案打印引入红的文件/diff 归属
  （现排查靠人肉 git status 对圈）——共享树在途互踩族（三次事故）的排查成本优化；

- [ ] **decode-sweep 工具 + 分母 v2**（seed-decode-gap 发现）：系统性扫描全部数据岛的
  ROM 原字节解码（找 JSR/JMP 目标落界/回环模式），修正 S/D 分母错面后出 v2 导出；

- [ ] **里程碑编号矛盾**：north-star.md（M3=核心系统/M4=全内容含密码）与 methodology.md
  （M3=含密码/M4=差分收口）M3/M4 定义不一致——统一口径后删除本行（state-fixture v3 L7）
- [ ] **covered_rule 同步**（state-fixture v6 B1/M1）：completeness_matrix.json covered_rule 现为单判据
  「covered⇔strength≥L1」，与 state-fixture §3(c)「B-SCN covered 以 invariant (c) 通过为前提」
  未接通——schema 升级或加注后删除本行
- [ ] **completeness provenance 悬空**：completeness_matrix.json provenance 写 tools/matrix_sync.py
  --export（该文件不存在；实际导出器=completeness_denoms.py）——修正指向后删除本行
- [x] ~~docs/spec/completeness.md 悬空引用~~（2026-10-14 已建 docs/spec/completeness.md，§0/§6.1 对齐 schema）

## 里程碑 B（2026-10-15 用户裁定——集成期转向）

**主里程碑口径**：L3 双腿帧级对拍在 **boot-title + samus 移动 demo** 两场景通过
（含渲染 PPU 写入面；对齐「移动先行」原排序）。从分母序批量端口**转向执行序集成**：
帧引擎先跑，unported! 桩阵按真实执行序引爆缺口，每引爆一个端一个（L2 管线降为
组件库按需补给，证明机/三方/突变全保留）。batch-2（game-data 数据表）提前为帧引擎供料。
排期：task-10 帧引擎+boots-to-title → task-11 samus 移动 demo；task-12 执行路径调用图
（喂执行序队列）；task-13 L3 game-core 腿验证设计。bank7 剩余 176 入口=执行序触发积压池。

> **2026-11-23 现状**：帧引擎已落地并支撑 21 个 L4 用例（全帧对拍，`shots_rust/matrix.json`）；但 L3 game-core 腿仍未启用（见 S2/S6），task-13 的验证设计未完成。执行序引爆进度以 routine-inventory 的 unported 603 为准，不再以「176 入口积压池」计数。

## 里程碑 C 蓝图（2026-10-15 asm 架构考古产出——B 后启用）

域划分照抄游戏自身组织（m1disasm 结构=游戏的模块说明书）：产品层域树
core/frame（prg7）· world/{brinstar,norfair,tourian,kraid,ridley}（prg1-5）·
gfx（prg6）· title（prg0）· audio（sound_engine）。B 后从「按函数爬」转「按域收割」：
world 垂直（room_ptrs→metatiles→CHR→PPU 四级数据链）/ enemy 垂直（敌槽+分发 handler）/
audio 垂直——每域=L3 场景+两层形态域模块。
- **batch-2 蓝图**：关卡=数据四级引用链（room_ptrs/metatiles/palettes/items/
  enemy_sprite_data 五件套/区域 bank 同构）——schema 已知，game-data 直接按层级建模；
- **状态显式化（贯穿工作流）**：constants_ram 286 zp 名=typed view 候选矿，
  memmap 随消费随显式化（命名台账+两层架构联动）；
- 表驱动分发器（C27C A 值域）→ enum+match；宽放 18 overframe=Stage A 保真面。

## 批量批次表（completeness.md §2，batch-0 起步）

> 本表是 2026-10-14 的批次规划，已不作为进度口径；现状分母与覆盖以 `python3 gates/cli.py status`（routine-inventory / completeness-ratchet）与 `data/status/completeness_matrix.json` 为准。

| 批次 | 模块 | 状态 |
|---|---|---|
| batch-0 | 流水线自举：L2 case factory + 移动积分器切片 | 进行中 |
| batch-1 | 移动系统收尾（UpdateSamus/跳跃引擎） | 未排期 |
| batch-2 | D 面数据表解析（可与 batch-1 并行） | 未排期 |
| batch-3+ | 战斗/门/房间/HUD/音频/密码（按矩阵聚类续排） | 未排期 |

## 首切片（垂直打通）

reset（bank7:$FFB0）→ NMI/主循环 → 标题 → 进 Brinstar → Samus 移动/跳跃/滚球，
每步 L3 帧级差分对齐后进入下一块。

## 模块优先级（S4/S5 排序参考）

1. 引擎骨架：MMC1 bank 语义、NMI/主循环、房间加载、CHR-RAM 上传
2. Samus 物理与战斗（移动/跳跃/滚球/射击/受伤无敌）
3. 门与房间切换
4. 敌人 AI 通用框架 + Brinstar 敌人
5. 道具/能力获取、密码系统
6. Tourian / Boss / Mother Brain / 结局
7. 音频（sound engine，最后）

## 已锁定决策

见 AGENTS.md §0（D1 Rust / D2 两阶段 / D3 ROM+Mesen2 真值 / D4 保真 quirks /
D5 四方交叉投影 / D6 NES_NTSC 单目标）。


## 长线目标阶梯（量产阶段——2026-10-19 立）

> 每级=可验收的判据集；级间=流水线吞吐函数。协调者按级派发、按周迭代。

| 级 | 目标 | 判据 | 估时 |
|---|---|---|---|
| **G1 批产主体** | banks0-6 全域+bank7 池移植（S1-S7+A1-A9+176 池） | 12 批施工图走完+每批验收四条+L2 案族全并 | ~3 周 |
| **G2 玩法系统** | UpdateItems/敌深路径/门电梯/道具回环（T45 卡族+task-37 输入） | 玩法场景族真达成（记账面全清） | 随 G1 并行 |
| **G3 音频全量** | handler 细行+曲族+MusicLoop 深径（~35 件 5 波次） | 声域账面清+听感对拍（可选 L4 音频面） | ~1.5 周并行 |
| **G4 通关集成** | Tourian→MotherBrain→escape→ChooseEnding（T45-3/5/6） | 通关径场景包全达+L3 通关场景对拍 | ~1 周 |
| **G5 Stage A 收官** | 全游戏行为一致（含 quirk）+L1-L4 全绿+分母账面终形 | AGENTS Gate 全过+用户验收 | 收尾 |
| **G6 Stage B** | 现代化（分辨率/输入/音频/存档/UI 扩展缝） | 两层架构扩展面（行为面不动） | 另立项 |

**协调者循环**（持续迭代）：每卡无缝续令 → 每批回归+翻板 → 每周复盘（吞吐 vs 施工图
+优先序修正）→ 事故熔断（停手定位→根因→护栏→跑）。纪律=docs/spec/production-discipline.md。

## 文档口径同步记录（2026-11-23）

| 位置 | 旧值 | 新值 | 取证命令 | 采样时间 |
|---|---|---|---|---|
| README · L4 像素对拍 | 15 案 / 132 格 · match 126 · diff 6 · blocked 0 | 21 案 / 172 格 · match 166 · diff 6 · blocked 0 | `python3 tools/l4_diff_run.py run-all`（产物 `shots_rust/matrix.json`） | 2026-11-23 21:01 |
| README · 测试 | 741 passed | 1,087 passed | `python3 gates/cli.py status`（rust-tests）/ `cargo test --workspace` | 2026-11-23 21:00 |
| README · 门禁 | PASS=22，selftest 156/156，约 6s | 27 项：PASS=23 · FAIL=0 · SKIP=3 · TOLERATED=1；`check` 8.13s（本层，wall 8.25s，空载） | `gates/cli.py status`；`gates/cli.py check` | 2026-11-23 21:02 |
| README · 例程分母 | code 1135 / ported 30 / unported 880 | code 1135 / ported 380 / candidate 152 / unported 603（旧值 532 = ported 380 + candidate 152，把未复核的 candidate 读成了已端口——对影 #7 复核退回项） | `python3 tools/routine_inventory.py --stats` | 2026-11-23 21:00（21:30 按退回意见更正口径） |
| README · 仓库结构 / 常用命令 | gates「22 门禁」；`check` 约 6s；run-all「二次约 4s」 | 27 门禁；约 8–9s；遍历 cases 全用例 + emu 腿按锚缓存 | `gates/cli.py status`；本次实测耗时 | 2026-11-23 |
| master-plan · 标题 | 阶段状态（2026-10-14） | 阶段状态（2026-11-23 实测口径） | — | 2026-11-23 |
| master-plan · S3 密码屏 | 「深路未端口」，指针 `title/mod.rs ~908–929` | 深路已臂化（`:1495`，`4db83c43`）；f700/f1100 两格为已端口面残差（承接 #10）；`:718` 注释为过期注释 | `git log -S enter_password_body`；`title/mod.rs:1495`、`:1532`；`matrix.json` case-password-justin | 2026-11-23 21:06 |
| master-plan · S2–S6 | S2 进行中（带 r2 待研）；S3–S6 全部 ⬜ | 按实测逐行改写（S2–S6 均进行中，各附命令/指针） | `gates/cli.py status`；`shots_rust/matrix.json`；`data/status/completeness_matrix.json` | 2026-11-23 21:00 |
| 未重测声明 | — | README 两条录像进度与动态触达 964 B 保留 2026-11-17 读数 | 本轮未跑录像与 `tools/raw_execution_report.py --check` | — |
| L4 复跑可复现性 | — | 21 案 / 172 格 · match 166 · diff 6（21:15 独跑复现；diff 分布：input-conflict c673/c674/c676=84/84/303px、password f700/f1100=23/42px、scrollseam f1100=266px） | `python3 tools/l4_diff_run.py run-all`（独跑，不与 cargo/门禁并行） | 2026-11-23 21:20 |
| emu 采集负载敏感（本轮实测） | — | 21:06 与 `gates/cli.py check` 并行时 case-001 12 格报 12–21k px 假差（emu 采集失步，EN 计数/位姿不一致）；21:13 单案复跑 13/13 全 0、21:15 全量独跑恢复 166/6 | 对比 `shots_rust/case-001-*.l4report.json` 与单案重跑输出 | 2026-11-23 21:06–21:20 |
