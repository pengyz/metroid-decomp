# metroid-decomp — 《银河战士》(Metroid, NES/6502) 忠实 Rust 重制

以**证据链驱动**的逆向 → 规格 → Rust 移植 → 差分验证流水线。当前阶段：**行为对拍 + 实体化改造并行**
（ROM 取证 ✔ / 纪律骨架 ✔ / seed 冻结 ✔ / 数据代码分离 ✔ / 观测面与字节归属 ✔）。

> 纪律唯一权威：[AGENTS.md](AGENTS.md)（D1–D7 已锁定决策）；
> 管线设计：[docs/ai-decompile-pipeline.md](docs/ai-decompile-pipeline.md)；
> 字节归属规格：[docs/spec/entity-ownership-and-nesmap.md](docs/spec/entity-ownership-and-nesmap.md)；
> 进度看板：[docs/master-plan.md](docs/master-plan.md)。

## 当前状态（截至 2026-11-23）

> 除标注「未重测」的行外，下列读数均为 2026-11-23 实测（采样时 HEAD `495426ae`；其后 HEAD 前进到 `915763b9`，只新增 `docs/design/` 一份文档，未触及代码/门禁/基线，故读数仍成立）；取证命令与新旧值对照见 [master-plan 的文档口径同步记录](docs/master-plan.md#文档口径同步记录2026-11-23)。

| 轴 | 数字 | 说明 |
|---|---|---|
| L4 像素对拍 | 21 案 / 172 格 · **match 166 · diff 6 · blocked 0** | 2026-11-23 21:15 独跑 `python3 tools/l4_diff_run.py run-all`（采样时 HEAD `495426ae`，产物 `shots_rust/matrix.json`）；逐像素零容差，矩阵只防回退 |
| 100% 录像 | 连续 0px 到 **f4017 / 120540**（3.3%） | 输入来自社区 TAS，唯一持续产出真缺陷的仪器；2026-11-17 读数，本轮未重测。机器账本口径见 `data/status/movie_ledger.json`（当前记 `frames_synced` 3889 / 首分歧 f3890），两个数是不同轮读段，重测以 ledger 为准 |
| any% 录像 | f1039 / 28690（3.6%） | 2P 通道已通，段未推完；2026-11-17 读数，本轮未重测 |
| 测试 | **1,087 passed / 0 failed** | `cargo test --workspace`；门禁 rust-tests 基线 1,087 |
| 门禁 | 27 项：**PASS=23 · FAIL=0 · SKIP=3 · TOLERATED=1** | `python3 gates/cli.py status`；`check` 空载实测 8.1–8.4s（2026-11-23 本机两次：8.13s / 8.42s；与 emu 采集并行会显著变慢）；l4/visual 慢层不进 hook |
| 字节归属 | 面 2272 B：Owner **374** / Unmodeled 611 / Undeclared **1287** | 静态声明 |
| 动态触达 | **964 / 2272（42%）** | 三场景实测（2026-11-17，本轮未重测），见 findings |
| 例程分母 | code 1135 / ported 380 / **candidate 152** / unported 603 | 结构分母，非完备性证明（`python3 tools/routine_inventory.py --stats`）；candidate=仅命中 Rust 符号名、未按 asm 复核行为等价，不计入已端口（1135 = 380+152+603） |

## 红线（版权口径）

- **仓库不含 ROM 及其二进制衍生数据**（BYO-ROM：使用者自备合法取得的卡带 dump）。
  全部提交历史经审计零版权二进制；`rom/`、`shots/`、`analysis/`、`movies/` 永不入库。
- `reference/seed/`（冻结反汇编 listing）含全量 ROM 字节的十六进制列 —— 转公开前须按
  [docs/methodology.md](docs/methodology.md) §6 做权利评估。
- ROM 指纹与校验：[rom/README.md](rom/README.md) + `python3 tools/verify_rom.py <rom>`
  （USA/NTSC：CRC32 `A2C89CB9`，headerless `70080810`）。
- 游戏数据不入库：`assets/metroid.pack` 由 `crates/pack-tool` 从使用者自备 ROM 提取（gitignored）。

## 仓库结构

| 路径 | 内容 | 入库 |
|---|---|---|
| `AGENTS.md` `prompts/` | AI 纪律总纲 + 四角色契约（decomp-analyst / spec-writer / porter / verifier） | ✅ |
| `crates/game-core/` | 纯逻辑、确定性、无 I/O：`samus/` `enemy/` `world/` `title/` `render/` `sound/` `memmap/` | ✅ |
| `crates/game-data/` | 资产消费端（包格式、`assets` 唯一访问面）；`extract` feature 只服务提取侧 | ✅ |
| `crates/pack-tool/` | `extract` / `verify` / `list`（BYO-ROM 提取器与逐段校验） | ✅ |
| `docs/` | 方法论、管线、findings 考证、rounds 对抗复核、knowledge 知识库、spec 规格 | ✅ |
| `gates/` | 27 门禁 + ratchet 基线 + git hooks（`python3 gates/cli.py status`） | ✅ |
| `tools/` | 可再生工具链（seed 归一化、场景驱动、L4 对拍、覆盖/分母工具） | ✅ |
| `reference/seed/` | **冻结投影**：规范化 6502 反汇编（sha 锚定） | ✅ |
| `refs/` | 社区参考源码（**submodule 钉死**，只作线索，D5 命名不背书） | ✅ gitlink |
| `rom/` `shots/` `analysis/` `movies/` | ROM、截图产物、trace 工作区、TAS 录像 | ❌ |

## 常用命令

```bash
python3 gates/cli.py status      # 只读预检（SKIP/TOLERATED 显式打印）
python3 gates/cli.py check       # 快层总闸（实测约 8–9s）
python3 tools/l4_diff_run.py run-all            # 权威矩阵（遍历 cases/ 全用例；emu 腿按锚缓存复用）
#   ⚠️ 跑 run-all 时勿并行 cargo/门禁等重任务：emu 腿对 CPU 负载敏感，会采集失步（见 master-plan 同步记录）
python3 tools/l4_diff_run.py run movies/cases/x.txt --timeout 1800   # 单案长跑
python3 tools/raw_execution_report.py --check   # 字节面动态触达报告
cargo test --workspace                          # 全量测试
```

## 三条不可违的验收纪律

1. **判定权只在门禁**：asm 是唯一真相源（`refs/m1disasm`，逐行读完再实现），commit 带 `bank:addr` 锚。
2. **像素零容差**：L4 不用阈值、不用白名单兜底；`ram.hex` 是显微镜，不参与 pass/fail。
3. **缺证据 fail-loud**：不静默回退、不猜、不"顺手修 bug"（原版 quirk 一律登记 `docs/bug-ledger.md`）。
