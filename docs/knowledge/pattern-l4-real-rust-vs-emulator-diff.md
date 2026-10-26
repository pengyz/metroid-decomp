---
type: pattern
title: L4 真对拍（Rust vs 模拟器）的正确做法——真渲染帧 + 单一 DSL 解析器 + 像素零容差
date: 2026-08-26
status: confirmed
---

# L4 真对拍（Rust vs 模拟器）——可复用做法

## 一句话
**L4 对拍必须是「Rust 真实渲染帧」vs「模拟器截图」**；任何「模拟器 vs 模拟器」都是错误实现
（假对拍）。真实对拍 = 真实 Rust 渲染链（FrameEngine→render_frame）产出帧 + 模拟器采集同一
输入脚本 + 像素零容差比对。

## 反面教训（曾踩）
早期 `tools/l4_auto_diff.py` 名为对拍、实为**模拟器 vs 模拟器**（两边 `shots/` 都由
`scenario_run.py` 产出）；`rust_screenshot.rs` 曾用 `fill_render_state`（模拟数据）+ 黑图，
像素 diff=12759（20.8%）纯属噪声。**判据**：对拍两侧截图必须来自**两条独立执行径**——
Rust 侧走 `game-core` 逻辑，模拟器侧走 Mesen2 执行 ROM。同一条执行径的两份产物比对无意义。

## 正确架构（本仓库落地件）
| 角色 | 文件 | 职责 |
|---|---|---|
| Rust 真渲染 | `crates/game-core/src/bin/rust_screenshot.rs` | FrameEngine 逐帧跑真游戏→`render_frame`（真 RenderState+调色板+OAM 影子）→PNG |
| 唯一 DSL 解析 | `tools/scenario_run.py::parse_scenario` | 场景 DSL 只在此解析（无第二解析器） |
| 调度展开 | `tools/l4_diff_run.py::build_schedule` | 依 `scenario_capture.lua` 语义展开逐帧 joy1（press/hold+press_len+按钮位+对向键消解） |
| 对拍裁定 | `tools/l4_diff_run.py::diff_pair` | PIL 像素零容差；失配帧产 `Rust\|Emu\|Diff` 三联图 |

**调度表格式**（Rust bin 消费，非 DSL）：`name/frames/joy <帧> <joy1hex>/capture <帧> <tag>`。
按钮位（`game-platform PadState` / `hardware.asm:89-96`）：A=0x80 B=0x40 Select=0x20
Start=0x10 Up=0x08 Down=0x04 Left=0x02 Right=0x01。PNG=Rust 自包含 stored-deflate（零依赖）。

## 关键校验点（避免假绿）
1. **Rust 侧非黑图**：帧须有真实调色板色（非全 0）——曾以黑图冒充渲染帧。
2. **像素逐点比对**：`numpy` 逐像素零容差；禁止"hash 相近"糊弄。
3. **两侧命名一致**：`<name>_f<NNNNN>_<tag>.png` 逐 capture 对齐。
4. **缺口诚实记账**：`unported!` fail-loud 缺口在**对拍工具层**用 `catch_unwind` 捕获→
   采集点标 `blocked`（非伪造帧），引擎/测试仍确定性引爆（fail-loud 语义不变）。

## 复现命令
```bash
cargo build -p game-core --bin rust_screenshot
python3 tools/l4_diff_run.py run case-001-game-start-samus-interaction --timeout 300
# → shots_rust/<name>.l4report.json + 逐 capture 报告
```

## 本次对拍实证结论（case-001，2026-08-26 真数据）
- f420 game-in：**0/61440 像素一致**（真对拍可达逐像素一致）。
- f720/f850/f1000：282/2385/17800px 失配。**根因（对抗复核锐化）**：`rust_sha(f420)==rust_sha(f720)`
  逐字节相同 ⇒ **Rust 在 idle 帧(f420→f720)游戏态冻结**（emu 有萨姆斯 beam-in/就绪动画、rust 整帧
  不变），表症才是「不画萨姆斯/敌人/HUD」；移动帧(f850/f1000)因输入才推进。属游戏逻辑移植缺口。
- f1100 起：`unported @ bank07:D1EE`（FireSamusProjectile）缺口阻塞后 4 采集点。
- **附带考证**（A 级 asm）：`handler.rs` FIRE 判 `fire & A`（0x80）**错**——asm `asl/bpl` 测
  bit6=FIRE=**BUTTON_B**（0x40）；故 `press a`（跳跃）误走 fire 路径引爆缺口。
- 独立对抗复核 verdict=PASS：见 `docs/findings/l4-diff-adversarial-review.md`。
