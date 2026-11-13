# 载房掉帧窗模型：多点校准（止血件 2）

**结论一句话**：`LOAD_BASE_CYCLES = 29000` 不是"三点夹逼的中值拟合"，而是在
**五点 emu 实录 + 一条上界约束**下的可行区间 `[28114, 29780)`（宽 1666c = 5.6% 帧）
内的取值，距区间中点 28947 仅 +53c（0.18% 帧）。**常数本身不是主要误差源**；
真正的脆弱点是区间上界余量只有 780c（2.6% 帧）——任何子步骤周期低估都会翻窗。

> 本文更正 2026-11-05 缝会计卡的注记「BASE=29000 是 [25850,29780) 中值，区间宽
> 3930c=13%，属拟合」。该区间来自只取 room8 一条下界的粗估，漏掉了 room20 这条
> 更紧的下界。原文不删，此处原地更正（AGENTS §3.6 证伪更正文化）。

## 1. 问题

载房工作跨 NMI 边界 ⇒ `NMIStatus=1` 内块跳（`asm prg7_engine.asm:251 lda NMIStatus/bne @endIf_A`）
= D4 保真掉帧。窗长 = 载房工作周期 / 帧周期（29780c）。模型：

```
dropped = (LOAD_BASE_CYCLES + work_cycles) / FRAME_CYCLES
```

`work_cycles` 由各子步骤常量累加（`crates/game-core/src/frame/room.rs:116/120/158/173/509`、
`crates/game-core/src/frame/mainloop.rs:3145`）。`LOAD_BASE_CYCLES` 是"常规迭代体"
的基底——它不可直接测量（没有"空工作"的载房帧），只能用**约束夹逼**。

## 2. 方法：emu 实测掉帧窗 + rust 权威路径

**为什么必须新增实测点**：既有校准只有 3 个房间（room20/23/8），且都只是"某个 work
值 → 某个掉帧数"。要真正定位 BASE，需要更多 (work, dropped) 对，特别是能收紧区间
两端的点。

### 2.1 emu 侧（A 级）

探针 `/tmp/probes/probe-loadwin.txt`（未入库）：`frames 2400` + `watch 002D FC, 005A RoomNum,
001E MainRoutine, 0074 InArea` + 走 3 个房门的动作序列。

```
python3 tools/scenario_run.py run /tmp/probes/probe-loadwin.txt
# → 2400 帧 / wall 51.4s / analysis/traces/probe-loadwin.frames.jsonl
```

掉帧窗直接测 = FrameCount(`$2D`) 停滞段：

```
f1 停1 / f3 停3 / f310 停3 / f351 停7 / f895 停2 / f1067 停2 / f1286 停2
```

房间装载入口（`RoomNumber` 0xFF→真值）：**f896 room=0x14(20)**、**f1068 room=0x13(19)**、
**f1287 room=0x09(9)**，均 `MainRoutine=3` / `InArea=0`。选后房号走 $F0-$F4 属性链再回 $FF
（`asm AttribTableWrite` 4×inc + `RoomFinished ; 07:EA26`）。

### 2.2 rust 侧（必须走权威展开路径）

`tools/scenario_run.py:548-549` 明文规定「Rust 侧与 Lua 侧同消费本函数产物——**禁止第二处
展开**（两处实现漂移=两侧跑不同输入=假对拍）」。因此生成 sched 必须走：

```python
plan = scenario_run.parse_scenario(Path(".../probe-loadwin.txt"))
sched = scenario_run.expand_schedule(plan)     # 与 Lua 侧同源
l4_diff_run.write_schedule(sched, "/tmp/...sched")  # l4_diff_run.py:186 唯一序列化面
```

> 教训：我首版手写 DSL→sched 转换器，直接违反了这条规定。**双跑对拍的输入必须来自单一路径**，
> 否则测出来的差异无法归因到实现。

rust 侧掉帧窗 = `DSH_FC_TRACE` 的**缺失帧段**（`crates/game-core/src/bin/rust_screenshot.rs:469`
在 span 帧不打印）：
`f2(1) / f4-6(3) / f311-313(3) / f352-358(7) / **f896-897(2)** / **f1068-1069(2)** / f1258-1259(2)`

装载点 work 值（临时探针 `[loadwin]`，取证后已 `git checkout` 回退）：
room20 `work=31446 → dropped=2`、room19 `work=36620 → 2`、room9 `work=43378 → 2`。

### 2.3 双侧对齐验证

| 装载点 | emu 停滞 | rust span | 对齐 |
|---|---|---|---|
| room20 | f895 停 2 | f896-897(2) | ✅ |
| room19 | f1067 停 2 | f1068-1069(2) | ✅ |
| room9 | f1286 停 2 | f1258-1259(2) | ❌ 早 28 帧 |

前两点完全对齐。**首次 FC 分叉在 f1260**（emu=217 / rust=215，差 -2），rust 在 f1258-1259
多出一段 span 而 emu 那两帧 FC 正常递增；f1288 自行收敛回 0，此后 2400 帧未再分叉。
该分叉属**转场时序**问题（另案），不影响本节 BASE 的约束推导。

## 3. 五点约束区间

每点给出 BASE 的可行半开区间（`dropped = (BASE+work)/29780`，整数除法）：

| 点 | work | 实测 dropped | BASE 约束 |
|---|---|---|---|
| room20 | 31446 | 2 | [28114, 57894) |
| room19 | 36620 | 2 | [22940, 52720) |
| room9 | 43378 | 2 | [16182, 45962) |
| room23 | 46456 | 2 | [13104, 42884) |
| room8 | 71946 | 3 | [17394, 47174) |
| **常规帧** | **0** | **0** | **[0, 29780)** |

**交集 = [28114, 29780)**，宽 **1666c = 5.6% 帧**。
- 下界由 **room20** 单独确定（28666c 的 work 就要 2 帧掉帧）
- 上界由 **work=0 ⇒ dropped=0** 单独确定
- 现值 29000：距下界 +886c（3.0%），距上界 −780c（2.6%），距中点 +53c（0.18%）

### 3.1 为什么新增两点没有收紧区间

room19 / room9 的约束区间 `[22940,52720)` / `[16182,45962)` **包含** room20 的 `[28114,57894)`
的下界部分，因此对交集零贡献。这不是采样无效，而是**普通房间的 work 值域天然受限**：

- work 更大（room23/room8）→ 上界更松（`dropped` 更大，除法更宽容）
- work 更小（room19/room9 仍 ≥ 28114）→ 仍是 2 帧，无法把下界往上推

要再收紧，需要「**work 更小但仍掉 2 帧**」（推下界）或「**work 更大但只掉 2 帧**」（压上界）
的房间——这类房间若存在，其诊断信息会在 §5 的脆弱点分析里出现。

## 4. 与社区方法论对照

社区项目 `~/workspace/super-metroid-decomp` 的
`docs/findings/s2-command-overhead-calibration.md` 用合成压缩流做了同类校准，结论是：

> 同一组常数不能同时拟合三种命令构成；存在未建模的二阶项（命令长度分支/指针推进/
> 小 len 内层分支次数）。验收标准改为「**装载总帧数**误差 ≤1」，不是每常数精确。

本项目得到的是同构结论的**另一个形态**：
- 单个常数的精确值**不可辨识**（可行区间 5.6% 帧宽）——与其追求拟合并把 BASE 拧到某个
  小数，不如接受区间并把**判据放在帧级**（`dropped` 整数）
- 真正的风险不在 BASE，而在**子步骤周期的系统低估**（§5）

差异：社区项目校准的是压缩流解码器的命令开销（二阶项来自命令构成多样性），本项目
校准的是载房工作的周期累加（二阶项来自子步骤分支多样性，如 UpdateAttrib 早退/全径）。
两者都无法用单常数消除，只能限定验收判据的粒度。

## 5. 残留风险（真正的脆弱点）

上界余量仅 **780c = 2.6% 帧**。这意味着：

| 子步骤 | 现值 | 2.6% 帧容差 |
|---|---|---|
| LOAD_INIT_CYCLES | 9262 | ±780c（8.4% 自身） |
| LOAD_MT_FULL | 349 | 约 2 个元 tile 的偏差 |
| LOAD_ENEMY_CYCLES | 300 | 约 2.6 个敌人 |

任一子步骤被系统性低估 ≥780c，就会让某类房间的 `dropped` 少算 1 帧 ⇒ 时间线整体前移
1 帧 ⇒ 大面积像素差（这正是 2026-11-05 修前 `room20=1` 的形态：9.4k 像素差）。

**因此后续加固方向不是继续调 BASE**（区间已收 5.6%，中点距现值 0.18%），而是：
1. 用 exec 探针对每个子步骤（FillRoomRAM / AttribFill / DrawMetatile / UpdateAttrib /
   DrawObject 壳 / LoadEnemy）做**独立计数校准**，把每个常量的误差上界压到 ±100c 内；
2. 或改用**真周期累加**（6502 逐条周期表），彻底取消"基底 + 子步骤常量"的两段式近似
   ——这正是路线丙第二步「周期计数底座」的内容。

## 6. 判定与固化

- 代码：`crates/game-core/src/frame/room.rs` 的 `load_window_model_calibrated` 已改为
  **五点 + 上界**的机械断言（`POINTS` 表 + `lo/hi` 区间断言），测试名不变（既有 CI 引用不变）。
- 判据保持帧级（`dropped` 整数），不对 BASE 做小数级断言——与社区 s2 结论一致。

## 7. 复现

```bash
# emu 腿（约 51s）
python3 tools/scenario_run.py run /tmp/probes/probe-loadwin.txt
# → analysis/traces/probe-loadwin.frames.jsonl，看 $2D 停滞段

# rust 腿（须走权威展开，禁止手写 sched 转换）
python3 - <<'PY'
import sys; sys.path.insert(0, "tools")
from pathlib import Path
import scenario_run, l4_diff_run
plan = scenario_run.parse_scenario(Path("/tmp/probes/probe-loadwin.txt"))
sched = scenario_run.expand_schedule(plan)
l4_diff_run.write_schedule(sched, "/tmp/probes/probe-loadwin.authoritative.sched")
PY
DSH_FC_TRACE=1 cargo run -q -p game-core --bin rust_screenshot \
  -- --schedule /tmp/probes/probe-loadwin.authoritative.sched --out /tmp/rsauth
# → 缺失帧段即 span（掉帧窗）

# 契约测试
cargo test -p game-core --lib load_window
```

## 8. 遗留

1. f1260 首次 FC 分叉（room9 前一房转场）：rust 多 2 帧 span，f1288 自愈——**转场时序另案**。
2. 子步骤常量尚未独立校准（见 §5），区间上界余量 2.6% 帧是当前最大的模型风险。
3. 探针 `probe-loadwin.txt` 仍在 `/tmp`，未入库（不属于 L4 用例集）。
