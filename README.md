# metroid-decomp — 《银河战士》(Metroid, NES/6502) 忠实 Rust 重制

以**证据链驱动**的逆向 → 规格 → Rust 移植 → 差分验证流水线。当前阶段：**行为对拍 + 实体化改造并行**
（ROM 取证 ✔ / 纪律骨架 ✔ / seed 冻结 ✔ / 数据代码分离 ✔ / 观测面与字节归属 ✔）。

> 纪律唯一权威：[AGENTS.md](AGENTS.md)（D1–D7 已锁定决策）；
> 管线设计：[docs/ai-decompile-pipeline.md](docs/ai-decompile-pipeline.md)；
> 字节归属规格：[docs/spec/entity-ownership-and-nesmap.md](docs/spec/entity-ownership-and-nesmap.md)；
> 进度看板：[docs/master-plan.md](docs/master-plan.md)。

## 当前状态（截至 2026-11-17）

| 轴 | 数字 | 说明 |
|---|---|---|
| L4 像素对拍 | 15 案 / 132 格 · **match 126 · diff 6 · blocked 0** | 逐像素零容差；矩阵只防回退 |
| 100% 录像 | 连续 0px 到 **f4017 / 120540**（3.3%） | 输入来自社区 TAS，唯一持续产出真缺陷的仪器 |
| any% 录像 | f1039 / 28690（3.6%） | 2P 通道已通，段未推完 |
| 测试 | **741 passed / 0 failed** | 运行时 + 静态双基线 |
| 门禁 | **PASS=22**，selftest 156/156，全量约 6s | l4/visual 慢层不进 hook |
| 字节归属 | 面 2272 B：Owner **374** / Unmodeled 611 / Undeclared **1287** | 静态声明 |
| 动态触达 | **964 / 2272（42%）** | 三场景实测，见 findings |
| 例程分母 | code 1135 / ported 30 / **unported 880** | 结构分母，非完备性证明 |

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
| `gates/` | 22 门禁 + ratchet 基线 + git hooks（`python3 gates/cli.py status`） | ✅ |
| `tools/` | 可再生工具链（seed 归一化、场景驱动、L4 对拍、覆盖/分母工具） | ✅ |
| `reference/seed/` | **冻结投影**：规范化 6502 反汇编（sha 锚定） | ✅ |
| `refs/` | 社区参考源码（**submodule 钉死**，只作线索，D5 命名不背书） | ✅ gitlink |
| `rom/` `shots/` `analysis/` `movies/` | ROM、截图产物、trace 工作区、TAS 录像 | ❌ |

## 常用命令

```bash
python3 gates/cli.py status      # 只读预检（SKIP/TOLERATED 显式打印）
python3 gates/cli.py check       # 快层总闸（约 6s）
python3 tools/l4_diff_run.py run-all            # 权威矩阵（emu 腿按锚缓存，二次约 4s）
python3 tools/l4_diff_run.py run movies/cases/x.txt --timeout 1800   # 单案长跑
python3 tools/raw_execution_report.py --check   # 字节面动态触达报告
cargo test --workspace                          # 全量测试
```

## 三条不可违的验收纪律

1. **判定权只在门禁**：asm 是唯一真相源（`refs/m1disasm`，逐行读完再实现），commit 带 `bank:addr` 锚。
2. **像素零容差**：L4 不用阈值、不用白名单兜底；`ram.hex` 是显微镜，不参与 pass/fail。
3. **缺证据 fail-loud**：不静默回退、不猜、不"顺手修 bug"（原版 quirk 一律登记 `docs/bug-ledger.md`）。
