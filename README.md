# metroid-decomp — 《银河战士》(Metroid, NES/6502) 忠实 Rust 重制

以**证据链驱动**的逆向→规格→Rust 移植→差分验证流水线。当前阶段：**S2 动态证据**
（S0 ROM 取证 ✔ / S0.5 纪律骨架 ✔ / S1 seed 冻结 ✔ + 对抗复核 pass）。

> 纪律唯一权威：[AGENTS.md](AGENTS.md)（D1–D7 已锁定决策）；
> 管线设计：[docs/ai-decompile-pipeline.md](docs/ai-decompile-pipeline.md)；
> 进度看板：[docs/master-plan.md](docs/master-plan.md)。

## ⚠ 红线（版权口径）

- **仓库不含 ROM 及其二进制衍生数据**（BYO-ROM：使用者自备合法取得的卡带 dump）。
  全部提交历史经审计零版权二进制；`rom/`、`shots/`、`analysis/` 永不入库。
- 本仓库当前为**私有**。`reference/seed/`（冻结反汇编 listing）含全量 ROM 字节的十六进制列
  ——转公开前须按 [docs/methodology.md](docs/methodology.md) §6 做权利评估。
- ROM 指纹与校验：[rom/README.md](rom/README.md) + `python3 tools/verify_rom.py <rom>`
  （USA/NTSC：CRC32 `A2C89CB9`，headerless `70080810`）。

## 仓库结构

| 路径 | 内容 | 入库 |
|---|---|---|
| `AGENTS.md` `prompts/` | AI 纪律总纲 + 四角色契约（decomp-analyst / spec-writer / porter / verifier） | ✅ |
| `docs/` | 方法论、管线、master-plan、findings 考证、rounds 对抗复核、knowledge 知识库、spec 规格 | ✅ |
| `gates/` | 11 门禁 + ratchet 基线 + git hooks（`python3 gates/cli.py status`） | ✅ |
| `tools/` | 可再生工具链脚本（IDA 导出、seed 归一化、场景驱动、覆盖/分母工具） | ✅ |
| `reference/seed/` | **冻结投影**：规范化 6502 反汇编（110,548 行，sha 锚定） | ✅ |
| `oracle/scenarios/` | 场景 DSL（对拍语料） | ✅ |
| `rom/` | 原版 ROM（只读归档） | ❌ 只入 README |
| `refs/` `analysis/` `shots/` `tools/mesen2/` `tools/wla-dx-src/` | 社区参考源码 / 可再生工作区 / 截图 / 模拟器与汇编器二进制 | ❌ 可重建 |

## 工具获取途径（全部可重建，故不入库）

| 工具 | 用途 | 获取 | 备注 |
|---|---|---|---|
| **Mesen2 2.1.1** | oracle（trace/RAM/截图/输入注入） | 官网 [mesen.ca](https://www.mesen.ca/) / GitHub `SourMesen/Mesen2` Releases（**Windows/Linux 各有构建**） | 放 `tools/mesen2/Mesen`（Windows 为 `Mesen.exe`，见下）；须在 `~/.config/Mesen2/settings.json`（Windows：注册表/配置目录）设 `Preferences.AllowIoOsAccess=true`，否则 Lua 无法写采集文件 |
| **WLA-DX ≥10.7** | 构建参考反汇编 m1disasm（checksum 复验） | [github.com/vhelin/wla-dx](https://github.com/vhelin/wla-dx)（源码构建，实测 v10.8a；Releases 有 Windows 二进制） | `wla-6502`/`wlalink` 放 PATH；参考源在 `refs/m1disasm`（git clone metroidret/m1disasm） |
| **IDA Pro 9.1 + `m65` 模块** | seed 主轨导出（`tools/ida/run_all_banks.py`） | 商业软件（本机 `/home/peng/ida-pro-9.1`） | **可选**：无 IDA 时 m1disasm 反汇编可作替代投影；`IDAT`/`IDA_PATH` 环境变量可指路径（Windows `idat.exe` 已适配） |
| **Python ≥3.10** | 全部工具脚本 | python.org / 系统包 | **仅标准库**，无第三方依赖 |
| **Xvfb** | Linux headless 跑模拟器 | `apt install xvfb`（仅 Linux） | 驱动自管 DISPLAY；macOS/Windows 见下 |
| **Ghidra**（可选） | 6502 SLEIGH 伪代码交叉 | [ghidra-sre.org](https://ghidra-sre.org/) | Hex-Rays 无 6502，别指望 |
| **ROM** | 唯一资产真相源 | **BYO-ROM**（自购卡带 dump / 合法备份） | 见 [rom/README.md](rom/README.md) |

### Windows 用户须知

- **推荐 WSL2**：场景驱动 `tools/scenario_run.py` 依赖 `fcntl` 文件锁 / 进程组信号 / 自管 Xvfb，
  均为 POSIX 语义（原生 Windows 不可用）；WSL 内 apt 装 xvfb 后全流程可用。
- 模拟器本身在 Windows 原生运行无碍（日常手玩/调试可用 `Mesen.exe`）；
  自动化采集请走 WSL，二进制放 `tools/mesen2/Mesen`（或软链）。
- IDA `idat.exe` 路径探测已适配（`IDA_PATH` 指向安装目录即可）。

## 快速开始

```bash
# 0) 门禁预检（SKIP/TOLERATED 显式打印）
python3 gates/cli.py status

# 1) ROM 校验（自备 rom/Metroid (USA).nes）
python3 tools/verify_rom.py "rom/Metroid (USA).nes"

# 2) 参考投影复验（需 WLA-DX + refs/m1disasm）
cd refs/m1disasm && python3 build.py && cd ../..
python3 tools/check_disasm_equivalence.py     # 构建产物 vs ROM 逐字节比对

# 3) 冒烟：headless 跑 ROM 截图（需 Mesen2 + xvfb）
python3 tools/scenario_run.py run boot-title  # 产 trace/覆盖/截图
```

## 方法论速览

- **真值层级**：ROM 字节 + Mesen2 执行 = 最终真相；反汇编/伪代码 = 可再生投影；
  语义命名/规格 = 带证据指针的再创作；Rust = 由差分测试裁定等价。
- **完备性（D7）**：对拍只裁「对不对」不证「全不全」；S 符号面/D 数据面有机械分母
  （`tools/completeness_denoms.py`），B/R 面为声明性；四类缺口显式声明；
  覆盖采集语义带 fail-loud 自校验（rounds/r2 实证教训）。
- **对抗复核**：每步完成须独立审核员证伪复核，verdict=pass 才算完成（docs/rounds/）。
