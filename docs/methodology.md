# 银河战士（Metroid）Rust 重写 —— 方法论调研

> 状态：调研稿 v1（2026-10-14，社区资产调研结论已并入）｜
> 目标：不是写模拟器，而是把游戏逻辑用 Rust 改写成 PC 原生游戏
## 资源利用三层政策（2026-10-15 用户裁定——已证等价原则）

**前提（已证明）**：refs/ 反汇编与 ROM 字节等价（rom-disasm-equivalence.md +
check_disasm_equivalence.py 匹配构建验证）⇒ **asm = ROM 的完整可读形式**。

| 层 | 资源 | 用途 |
|---|---|---|
| 静态分析主文档 | refs/ 结构化 asm（m1disasm 等） | 语义理解/函数边界/调用结构/移植 spec——像读源码；不再逐指令重解码 |
| 交叉校验 | IDA/Ghidra 投影 + ROM 字节抽查 | 四方交叉（D5）；边界分歧=考证线索 |
| 模拟器（收缩三用途） | Mesen2 | ①裁判（L2/L3 差分 golden，验证面）②bug 定位（首分歧）③运行态/动态确证（mapper 窗口/时序/RNG 类静态不可答问题） |

纪律不变：第三方命名不背书（候选自证后入台账）；模拟器作为裁判的 golden 生成
不属「分析用模拟器」——分析读 asm，验证跑模拟器，各归其位。


## 0. 目标识别（已完成）

| 项 | 结论 | 证据 |
|---|---|---|
| 游戏 | **Metroid（メトロイド，银河战士）** | ROM 尾部数据块 ASCII `"METROID"` @ bank7:$FFE9（file 131065）；`docs/findings/rom-disasm-equivalence.md` §3 |
| 开发/发行 | Nintendo R&D1（横井军平监制），1986 | 官方资料 / ROM 内 MAIN PROGRAMMERS 注释（m1disasm `reset.asm` 转录） |
| 类型 | 横版动作探索（Metroidvania 鼻祖）：无地图、密码存档、区域门系统、Boss 房 | 玩法共识 + 待 spec 考证 |
| ROM 格式 | iNES，PRG 128KB（8×16KB）+ **CHR-RAM 8KB**，**Mapper 1 (MMC1)**，无 trainer | `tools/verify_rom.py` 输出；header `4E 45 53 1A 08 00 10 00…` |
| 版本 | **NES USA/NTSC 单一修订版**（无 v1.0/v1.1）；另有 PAL/FDS/MZM/CNS 官方变体（D6 范围外） | 构建指纹表见 `docs/findings/rom-disasm-equivalence.md` §5 |

## 1. 社区资产现状（2026-10 调研，决定我们的起点）

社区把"反汇编"这一步**已完成**，且质量极高：

| 资产 | 成色 | 我们的用法 |
|---|---|---|
| [metroidret/m1disasm](https://github.com/metroidret/m1disasm) | **逐字节匹配**（WLA-DX，checksum 校验），多版本目标，`constants_ram.asm` 结构化 RAM 定义，DOC 4 份专题 | **主力投影**（已验证等价）+ 数据表考证线索；但命名不直接采信（AGENTS.md §3.7） |
| ZaneDubya/MetroidMMC3、nmikstas/metroid-disassembly | 注释最系统的可读版 | 语义理解辅助 |
| mstan/MetroidNESRecomp（nesrecomp） | 6502→**生成式 C**（逐指令转译，可玩但不可读） | 行为 oracle 候选 / 交叉参考；**不是**可维护实现 |
| snesrev/sm（Super Metroid）+ 帧级协同仿真 | 手写 C 复刻 + 差分验证先例 | 方法论范本（目标是 SNES，不是初代） |
| Metroid Planets（vacantshade） | GameMaker 粉丝再创作，**无源码**，且有意改手感 | 产品形态参考；不作行为参考 |

**结论：Metroid 1 的"人类可读、行为忠实的 C/Rust 还原"是社区空白**——正是本项目。

## 2. 本项目路线：混合（规格恢复 + Rust 重写 + 差分验证）

```
S0 ROM 取证        ROM 校验 + m1disasm 等价性验证（✔ 已完成）
S1 静态投影        seed 冻结（IDA 归一化为主轨，m1disasm 对照）+ bank 图
S2 动态证据        Mesen2 trace / RAM dump / CHR-RAM 上传记录
S3 语义考证        命名台账 + 数据表识别（敌人/门/房间/密码表）
S4 规格恢复        行为规则：移动/滚球/射击/战斗/门与房间切换/敌人 AI/密码/Boss
S5 Rust 引擎骨架   60Hz 固定 tick、tile/sprite 渲染、输入、碰撞、对象池
S6 差分验证        L2 子程序级 + L3 帧级 + L4 视觉级 + L5 流程级
Stage B 现代化     宽屏、UI、音频重制、modding 缝（保真验收之后）
```

核心原则（与 AGENTS.md §1 一致）：

1. **先规格后代码**：数值和规则全部从 ROM/运行时证据里来。
2. **固定时间步 60Hz**：NES 的一切手感都是每帧 1/60 秒的离散语义。
3. **数据驱动**：敌人属性、房间表、密码表外置数据表（TOML/RON），便于 diff 与后续 mod。
4. **Mesen2 当 oracle**：真值来源；参考投影与 Rust 冲突时投影错。
5. **忠实 vs 修正分离**：8 sprites/scanline、闪烁、掉帧等硬件痕迹 Stage A 复刻，Stage B 做开关。

## 3. 工具清单

### 3.1 静态逆向

| 工具 | 用途 | 本机状态 |
|---|---|---|
| **m1disasm + WLA-DX v10.8a** | 匹配反汇编构建/对照（5 目标 checksum 校验） | ✅ `refs/`、`tools/wla-dx-src/`，已验证等价 |
| **IDA Pro 9.1**（`m65`）+ ida_mcp | seed 主轨（bank export 归一化）、数据表/xref | ✅ `/home/peng/ida-pro-9.1` |
| Ghidra | 6502 SLEIGH 伪代码 | 待装 |
| IDA loader | 无 iNES loader，raw binary 手工分段（bank0-6@`$8000`、bank7@`$C000`） | 脚本待写 |

### 3.2 动态分析（真值来源）

| 工具 | 用途 | 本机状态 |
|---|---|---|
| **Mesen2 2.1.1 Lua** | trace / 输入注入 / 截图 / 内存回调（gunnac 已验证同款接口） | 待迁移 `tools/mesen2/` |
| BizHawk | 备选输入录像回放 | — |

### 3.3 资产与文档

- **CHR-RAM**：无静态 CHR 可抽——图案表由游戏运行时写 PPU，走**上传 trace**（L1 golden）；
  调色板/房间 nametable 从 trace 导出。
- 音乐：`SRC/sound_engine.asm`（89KB @ bank7）+ `SRC/songs/`，后期软件 APU 渲染。
- 文档：**NESdev Wiki（MMC1 写序/PPU/APU）**、Disch/Anomie；m1disasm `DOC/`（area banks /
  enemies / enemy systems / sound engine）；密制 wiki、Data Crystal（可达性差，存档备查）。

### 3.4 Rust 技术栈

同 gunnac：bevy 或 sdl2/macroquad + wgpu；cpal/kira；serde + TOML/RON；glam；
`game-core`（纯逻辑）+ `game-platform`（trait）+ `game-oracle`（差分宿主）分层。

## 4. Metroid 特定难点

1. **MMC1**：串行 5 位写序切 bank + 运行时镜像控制；Rust 侧复刻行为不复刻芯片，但 spec 要写清 bank 语义。
2. **CHR-RAM 上传时序**：图案表在 NMI/vblank 期间批量写入，直接决定画面切换行为；差分要覆盖。
3. **密码系统**（代替存档）：密码 = 状态编码 + 校验/加密逻辑，是独立高价值模块（社区已有多个密码工具可交叉验证）。
4. **门/房间系统**：区域门、单向门、滚动房间切换是探索感核心，也是 L3/L5 差分富矿。
5. **敌人 AI 与战斗**：`SRC/enemies/` 已被社区独立拆分，行为状态机待考证。
6. **硬件痕迹**：8 sprites/scanline 闪烁、掉帧——Stage A 保真（D4），登记 BUG-xxx。
7. **版本变体**：PAL/FDS/MZM/CNS 差异单列（D6/R4），不混入原版行为。

## 5. 工作量粗估

| 里程碑 | 内容 | 粗估 |
|---|---|---|
| M0 | 纪律骨架 + seed 冻结 + oracle 迁移（本期） | 1 周 |
| M1 | 引擎骨架：60Hz tick、tile 渲染、Samus 移动/跳跃/滚球手感对齐 | 2–3 周 |
| M2 | 战斗/敌人 AI/门房间切换 + Brinstar 切片 | 3–4 周 |
| M3 | 其余区域 + Tourian + Boss + 密码系统 | 6–10 周 |
| M4 | 差分体系收口 + Stage B（宽屏/UI/音频） | 3–4 周 |

## 6. 合规提示

Metroid 为任天堂有效版权物（NSO 在售）。个人学习/内部研究没问题；
**仓库不含 ROM 及其衍生数据**（BYO-ROM，sha256 门禁）；公开发布走
"clean-room Rust 重写 + 自制素材"路线，且不得包含任天堂资产。
社区参考源码仅作线索阅读；m1disasm 等仓库许可条款引入前需核查。

## 7. 本期已落地的基建

- `AGENTS.md` —— AI 纪律总纲（Metroid 版，D1–D6 已锁定）
- `prompts/` —— 四角色提示词（与 `remake-*` skill 同步）
- `gates/` —— 11 门禁 + ratchet 基线 + git hooks（移植自 gunnac_remake）
- `docs/ai-decompile-pipeline.md` —— S0–S6 管线 + 完备性回路 + 风险登记 R1–R7
- `rom/Metroid (USA).nes` —— BYO-ROM，已校验（sha256 `e6e6b701…`）
- `docs/findings/rom-disasm-equivalence.md` —— m1disasm ↔ ROM 等价性证据（✔）
- `refs/` —— m1disasm / MetroidMMC3 / nmikstas 三套社区参考（只读）
- `tools/verify_rom.py`、`tools/check_disasm_equivalence.py`、`tools/wla-dx-src/`
