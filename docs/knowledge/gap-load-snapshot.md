---
type: gap
status: superseded
---

# gap-load-snapshot（load_snapshot 缺口（存档格式定义（加载方式）））

> 【2026-10-26 更正】下文「①—④均待定义」「缺口：机制缺失」「状态：缺口（未实现）」
> 已被**证伪**，本条目 `status: superseded`（历史主张原样保留不删——证伪更正文化）：
>
> - **① 存档格式**：已定谳——**混合路线**（否决中立 JSON 主格式）：MSS（`emu.createSavestate()`
>   真源 `<out>.mss`）+ 三直映 JSON（`{mss_bytes, ram, wram, regs{a,x,y,sp,pc,ps}, frame}`
>   ——`tools/scenario_capture.lua::bundle_capture` 产物）；schema 接缝表见
>   [../spec/state-bundle-schema.md](../spec/state-bundle-schema.md) §4。
> - **② 模拟器加载**：已实战——`emu.loadSavestate`（exec 回调上下文约束实录）+
>   `bundle_load`/`snapshot` DSL；见 [../findings/bundle-first-arbitrary-start.md](../findings/bundle-first-arbitrary-start.md)（2026-10-16）。
> - **③ Rust 加载**：已落——`FrameMemory::restore_mss(ram, wram, regs)`
>   （crates/game-core/src/frame/memory.rs）+ `FrameEngine::restore_mss` 续跑面
>   （phase=MainLoop+skip 首 NMI——从快照态起跑，非重跑 boot）；消费面
>   `rust_screenshot --bundle <prefix>`（2026-10-26）。
> - **④ 两边对齐**：已立验收——restore 后**前 N 帧逐字节** Rust↔emu 快进 diff
>   （`tools/l4_diff_run.py align`，逐帧结果+残差**显式登记**）；「加载成功≠对齐成功」
>   实测捕获（mid-move bundle：f=1 装载态逐字节对齐 ✓，f=2 起残差登记）。
>
> **残留**（非本条目辖）：对齐残差=游戏逻辑分歧（L3 未一致现状，如 c4de_unit 早出路径
> 写 $0A/$0B——账清随 L3 移植推进）；渲染面快照扩展（chr/nametable/palette/ppuctrl/scroll）
> 仅当残差定位为渲染态缺失才启用（spec l4-autodiff-completion §4-D 决策规则，默认不扩）；
> snapshot 用例像素对拍/matrix 消费=L4 后续（run-all/门禁）。

## 缺口描述

对拍用例 = ① 输入脚本 + ② 待加载存档 + ③ 对拍时长

**缺口**：待加载存档（load_snapshot）机制缺失

## 缺口内容

### ① 存档格式定义（待定义）
- 游戏状态定义文件（格式）
- 状态字段定义（RAM/寄存器/游戏状态）
- 状态序列化/反序列化

### ② 模拟器加载方式（待定义）
- Mesen2 loadstate（加载存档）
- 状态一致性验证（模拟器状态）

### ③ Rust 加载方式（待定义）
- game-core load_state（加载存档）
- 状态一致性验证（Rust 状态）

### ④ 两边对齐（待定义）
- 状态一致性验证（模拟器 vs Rust 实现）
- 状态差异检测（差异报告）

## 当前状态

- **状态**：缺口（未实现）
- **影响**：无法测试特定游戏状态（只能测试游戏的一幕）
- **优先级**：中（后续增加）

## 实施计划

| # | 步骤 | 内容 | 优先级 |
|---|---|---|---|
| **①** | **定义存档格式**（游戏状态定义文件） | 格式设计 | 中 |
| **②** | **实现模拟器加载**（Mesen2 loadstate） | 模拟器侧 | 中 |
| **③** | **实现 Rust 加载**（game-core load_state） | Rust 侧 | 中 |
| **④** | **实现两边对齐**（状态一致性验证） | 对齐验证 | 中 |

## 关联知识

- [[gap-render-dump-coverage]]（渲染面缺失（render-dump 覆盖不全））
- [[gap-l4-auto-diff]]（L4 自动对拍工具（统一对拍））
