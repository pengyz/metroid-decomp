# TAS 整片对拍 · 第五轮：门径载房窗 `3 vs 2` 的调用计数取证

> 状态：**取证收口（未改引擎代码）**。本轮的判定物是「emu/rust 两侧逐帧调用计数 + 载房迭代真实周期」，结论按计数表走，不按推断走。
> 树：`911cc6ae`（与 #124 同一树，`crates/game-core/src/world/` 自该提交起无变更）；`git status` 干净。

## 0 结论（先说结论，后给证据）

1. **窗长差 1 的落点不是「滚动调用次数」。** emu 在冻结窗内的滚动调用是 **1 次**（f1573），rust 在被 bog 的窗内是 **0 次**——两侧都低于常规帧的 2 次，`3 vs 2` 不是「谁多调了一次滚动」。
2. **真实差异是「载房迭代跨了几个 NMI 边界」。** emu 那一次主循环迭代实测 **112301 cycles ≈ 3.77 帧**（`cycleCount` 差），跨过 3 个 NMI 边界 ⇒ `$2D` 冻 3 帧；rust 用工作量模型 `dropped = ((LOAD_BASE_CYCLES + work_cycles) / FRAME_CYCLES).max(1)` 得 **2**（`(28454+55308)/29780 = 2.81`，取整向下）。
3. **工作量模型本身没少算。** rust 的 `LOAD_BASE_CYCLES + work_cycles = 83762`，emu 同一次迭代扣掉常规帧工作量后约 **83300**（112301−29000），差 **<1%**。⇒ 缺的不是周期数，而是**起点相位**：同样的工作量，从帧内某个相位开始跑会跨 `ceil(相位 + 工作量/帧长)` 个边界，向下取整会少一帧。
4. **f1301 未受影响**（本轮未改引擎代码，复跑读数见 §5）：首分歧仍是 **f1301 `$0454` emu=`00`/rust=`20`，2 字节**。
5. 修复方向因此改写：不是「补一次调用」，而是**给掉帧模型加相位项**（见 §6），并且必须用其它房间的既有 `dropped`（room0/23/8 = 1/2/3）做回归约束，不能整体改 `ceil`。

## 1 指纹与复现命令

| 项 | 值 |
|---|---|
| 主检出 | `/home/peng/workspace/metroid-decomp`，HEAD `911cc6ae` |
| emu 采集 | `analysis/scratch-fc/fcdoor_calls.lua`（`analysis/_gen/fcdoor.lua` 的副本 + 计数钩子；`tools/` 未改） |
| emu 场景 | 同 `analysis/scratch-fc/fcdoor.txt`：`frames 1700`、`inputlog movies/lordtom-metroid-100p.inputlog` |
| 探针产物 | `/tmp/r129/calls.txt`（逐帧 `CALLS f sd cu at nmi cycles`）、`/tmp/r129/iters.txt`（`ITER f cycle`，钩在 `07:C0C5 inc FrameCount`） |
| rust 状态 | `/tmp/r124/full/rust/full.state.jsonl`（#124 整片，`w.zp` 512 字节 hex） |
| rust 载房面 | `cargo run --release -p game-core --bin rust_screenshot -- --schedule /tmp/r129/t1600.sched --trace load` |

关键命令（只读）：

```
# emu（Xvfb 自管 + 隔离 HOME/XDG；必须拷 settings.json，否则 Mesen 弹 initial-setup 向导静默阻塞）
Xvfb :199 -screen 0 1280x1024x24 & DISPLAY=:199 \
  XDG_CONFIG_HOME=/tmp/r129/cfg TMPDIR=/tmp/r129/tmp HOME=/tmp/r129/home \
  tools/mesen2/Mesen "rom/Metroid (USA).nes" analysis/scratch-fc/fcdoor_calls.lua /DoNotSaveSettings

# rust（截断到 1600 帧的调度表）
cargo run --release -p game-core --bin rust_screenshot -- --schedule /tmp/r129/t1600.sched --trace load
```

## 2 emu 逐帧读数（A 级，探针实测）

计数钩子：`$E1F1 ScrollDoor` / `$E54A CheckUpdateNameTable` / `$EA13 AttribTableWrite` / `$C0D9 NMI`（exec 回调，只读）。

| 帧 | `$2D` | `$5A` | ScrollDoor | CheckUpdateNT | AttribWrite | 说明 |
|---|---|---|---|---|---|---|
| 1571 | 9 | FF | 1 | 1 | 0 | 门滚屏中，载房请求在此迭代内发生 |
| 1572 | **9** | 18 | **0** | 0 | **0** | 冻结帧：整帧无滚动调用（`SetupRoom` 正在跑） |
| 1573 | **9** | 18 | **1** | 1 | **1** | 冻结帧：**窗内唯一一次滚动调用 + 一步尾游标** |
| 1574 | **9** | F1 | **0** | 0 | 0 | 冻结帧：无调用，但尾游标已到 `F1`（上一步的落地） |
| 1575 | 10 | F1 | 2 | 2 | 2 | 恢复：常规双调用 |
| 1576 | 11 | F3 | 2 | 2 | 2 |  |
| 1577 | 12 | FF | 2 | 2 | 0 | 尾段结束（`RoomFinished`） |

主循环迭代周期（`07:C0C5 inc FrameCount` 的 `cpu.cycleCount` 差，A 级）：

| 迭代结束帧 | duration(cycles) | 帧数换算 |
|---|---|---|
| 1570 | 31636 | 1.06 |
| **1574** | **112301** | **3.77** |
| 1575 | 41332 | 1.39 |
| 1576 | 30068 | 1.01 |
| 1577 | 30822 | 1.03 |

⇒ 载房那一次迭代从 f1571 开始、f1574 结束，跨 **3 个 NMI 边界**，与 `$2D` 冻 3 帧完全一致。

## 3 rust 侧读数（A 级，trace）

`/tmp/r124/full/rust/full.state.jsonl`（`rl`=room_load_phase）：

| 帧 | `$2D` | `$5A` | rl | 备注 |
|---|---|---|---|---|
| 1571 | 9 | FF | 0 | 触发载房（同帧内 `setup_room`） |
| 1572 | — | — | — | **整帧缺失 = 被 bog** |
| 1573 | — | — | — | **整帧缺失 = 被 bog** |
| 1574 | 10 | F0 | 2 | 尾段开始（相位切 2、`$5A←F0`） |
| 1575 | 11 | F2 | 2 | 两格/帧（每帧 2 次滚动调用） |
| 1576 | 12 | F4 | 2 |  |
| 1577 | 13 | FF | 0 | 尾段结束（与 emu 同帧） |

`--trace load`（同树实跑）：`f1574 rn=F0 phase=2 tail=0 work=55308 sx=2`；`f1575 work=57908`（+2600/帧，滚动列写成本）。

## 4 机制对账

```
rust 掉帧公式   dropped = floor((LOAD_BASE_CYCLES 28454 + work_cycles 55308) / FRAME_CYCLES 29780)
                = floor(2.81) = 2                ← 与实测「bog 2 帧」一致
emu 实测迭代    112301 cycles = 3.77 帧；扣常规帧工作量(≈29000) ⇒ 载房增量 ≈ 83300
rust 模型总量  28454 + 55308 = 83762            ← 与 83300 差 <1%，模型没少算
差异            「2.81 帧的工作」从帧内相位 ≈0.4~0.5 开始 ⇒ 跨 0.5+2.81=3.31 ⇒ 3 个边界
                rust 只看总量、向下取整 ⇒ 2
```

旁证：emu 窗内仍有 1 次滚动调用（f1573），而 rust 的 bog 是「整拍吞掉、零调用」——这也支持「两侧不是同一个计数口径」，不能靠补一次调用来对齐。

## 5 f1301 复跑读数（本轮未改引擎代码）

```
python3 tools/tas_replay_diff.py replay --inputlog movies/lordtom-metroid-100p.inputlog \
  --frames 10000 --segments 10000 --out /tmp/r129/f1301 --allow-rust-missing --timeout 600
→ seg 0 [0,10000) diverge 178.6s 首分歧 {'frame': 1301, 'field': '$0454', 'offset': 84, 'emu': 0, 'rust': 32, 'diff_bytes': 2}
→ 覆盖率 f1301 之前的 1300 帧内容相等；首分歧点与 #122/#124 逐字节一致
```

## 6 下一步（修复建议，需单独一张卡）

**不给整体 `ceil`**：既有测试 `crates/game-core/src/world/room_draw.rs` 的 `assert_eq!((LOAD_BASE_CYCLES + work)/FRAME_CYCLES, want)` 已把 room0/23/8 的 `dropped` 钉成 1/2/3；整体改 `ceil` 会让这三间房各 +1，属行为回归。

**候选修法**（按可证伪顺序）：

1. **相位项**：给掉帧公式加「本帧已用相位」`phase`（＝触发载房时本迭代已耗周期 / FRAME_CYCLES 的小数部分），`dropped = floor(phase + (BASE+work)/FRAME)` 并保持既有三房不变——需要先能量出 phase（`work_cycles` 只在载房分支归零，触发点在 `check_update_name_table` 内，需登记「本迭代已跑多久」）。
2. **窗内尾游标步进**：即使修好窗长，emu 窗内 f1573 还有 1 次滚动调用（尾游标 F0→F1），rust 冻结窗内 0 次；这条要在相位修好后再对一次窗内 `$5A` 逐帧值。

**验收判据**（建议写进新卡）：门径窗 f1572–1574 三帧冻结；`$5A` 逐帧与 emu 一致（F1/F1/F3/FF）；**首分歧仍须 f1301 `$0454` 2 字节**；`cargo test --workspace` ≥1253、check PASS24/0/3/1、L4 不回退。

## 7 边界

- 未改 `crates/`（本轮是取证卡）；未改 `tools/`、`game-shell/`、`data/status/`。
- 未动 `LOAD_BASE_CYCLES`/`FRAME_CYCLES`。
- 探针脚本放在 `analysis/scratch-fc/`（不入库）。
- 不确定性：`work_cycles` 的「常规帧工作量 29000」是用同一次迭代 1570（31636）与恢复后 1575–1577（30068/30822）估的，误差约 ±3%，只用于量级对账，不作为修法判据。
