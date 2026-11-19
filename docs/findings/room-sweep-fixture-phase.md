# 房扫 dx=2 族：fixture 相位是红鲱鱼（负结果留档）

日期：2026-11-20
对象：`crates/game-core/src/frame/fixture.rs`（相位契约锚，**行为零变化**）+ `bin/rust_screenshot.rs`（改调该契约）
诊断输入：只读诊断子代理 `diag-scrollphase`（结论「注入相位早一帧」）+ 我的反证实验
前置：`docs/findings/room-tail-cursor-fix.md`、`docs/findings/room-sweep.md`

## 1. 待验假设（来自诊断子代理）

47 房扫里十房（`2/14/24/25/28/29/30/37/38/41`）失配签名一致：`ppu` 四区全 0 差、`shift.dx = +2`、`explains = true`。子代理的结论是**采集腿 fixture 注入相位错配**：

- emu 腿在 `tools/scenario_capture.lua` 的 `endFrame(F)` 帧末注入（work(F) 已跑完）⇒ 引擎 work(F+1) 消费；
- rust 腿在循环头、`tick(F)` 之前注入（`frame >= trigger`）⇒ work(F) 消费；
- ⇒ rust 的载入链与门滚动**整体早一帧**，捕获时 `ScrollX` 多走一个 +2 步。

它的证据只在 room 2：把 sched 的 `fixture 900` 改成 `fixture 901` 后 `shift.dx` 由 2 变 0，像素 `10381→198`（f60）/`6036→253`（f120）。

## 2. 我按该假设实施（全局 `frame > trigger`）后，47 房全量实测**推翻**它

| 族 | 旧形 `>=`（改前） | 新形 `>`（改后） |
|---|---|---|
| 原本 `ok`（0px）的房 | 0px | **`dx = -2`、`explains = true`、~9-25k px**（21 房） |
| 原本 `dx = +2` 的房 | +2 | `dx = 0`，余 70-496px（sprite 层） |
| 汇总（前 25 房实测） | `ok 18 / mismatch 6`（含边界守卫修复后的部分跑） | **`ok 0 / mismatch 25`** |

原始数据：`data/status/room_sweep_0.json`（改后跑，note=`post-fixture-phase`，已回退不入库）与 `/tmp/sweep_fix_dx.json`（三房）。

判读：**每个房要么「相位对齐」（旧形 0px），要么「rust 早一帧」（旧形 dx=+2）**。全局平移一帧只是把两种状态互换——若相位真是全局错配，不可能只有 10/47 房表现出 +2。

⇒ 十房的 +2 是**逐房差异**（载入窗长度 / 冻结帧数 / 首帧滚动量在个别房与 emu 不同），不是采集工具的全局契约问题。

## 3. 为什么「状态行」与「像素」给出相反结论

同一份输入在两侧的**显示帧**定义不同：

| 腿 | 注入点 | 帧 F 的画面内容 |
|---|---|---|
| emu | `endFrame(F)` 帧末 | work(F) 之后 |
| rust | 循环头 F | `capture(F) = tick(F-1)` ⇒ work(F-1) |

所以 `frame >= trigger`（同帧注入）恰好让两侧**像素面**对齐——rust 在循环头 F 注入、work(F) 消费，其画面出现在 `capture(F+1)`，与 emu `endFrame(F)` 之后那一屏等价。子代理是用 `--state-out` 的**状态行**（rust 行 N = work(N)、emu 行 N = work(N)）推出的 `>`，那对状态行成立、对像素面不成立；L4 裁的是像素。

## 4. 结论与留下的护栏

- **行为零变化**：`crates/game-core/src/frame/fixture.rs` 的 `fixture_due(frame, trigger)` 就是 `frame >= trigger`，`bin/rust_screenshot.rs` 改调它；4 条锚测锁住相位（同帧注入 / 更早不注入 / 跳帧不丢 / 多 trigger 独立）。
- 模块文档写明「改成 `>` 会让整屏落后一帧」与本次 21 房实录，防止下一次再按状态行口径「修正」。
- 十房 dx=+2 的真因仍未定谳，下一卡方向：逐房比对**载入窗长度**（rust `dropped` / `iter_bogged` 帧数 vs emu 冻结帧数）与**首帧 ScrollX**，而不是动 fixture 相位。

## 5. 方法论（复用）

- **只在单房做对照实验的结论不能直接推全体**：子代理只测 room 2 就给出全局契约修复，全量一扫立刻暴露 21 房回归。凡「全局相位/时序」类假设，必须先在**至少一个原本 ok 的房**上验证不回归，再全量。
- 同一现象在两套判据（状态行 vs 像素）下可能给出**相反**的修复方向；本项目 L4 判据优先，状态行只作定位辅助。
