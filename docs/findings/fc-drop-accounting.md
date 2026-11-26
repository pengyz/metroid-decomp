# FC 丢帧记账：逐事件预算的可行性与上游依赖

> 卡片：#114「A-2 根因修复：FC 丢帧记账改为逐事件预算（消 drift −53）」
> 写面：`crates/game-core/src/frame/` + 本文件。
> **2026-11-27 修订**：第一版（rev3 提交）的两个核心论断被验收打回，均成立；本版按 AGENTS.md §3.6
> 保留原文并就地标注作废，结论与读数全部换成干净工件重做的结果。

## 0. 结论（2026-11-27 修订，取代原 §0）

1. **原「逐帧 FC 不可观测」作废**。逐帧 FrameCount 可直接采：`tools/scenario_run.py run` +
   `watch 002d`，30000 帧 511 秒产出 `analysis/traces/fc30000.frames.jsonl`（30000 行，逐帧
   `$002D`），**不需要改 `tools/`**。验收人 600 帧 21.3 秒的抽样（f100=85 / f300=0x1D / f600=0x47）
   与我这轮全量一致。
2. **原「变体矩阵」作废**。原表读数建立在两代工件混跑的脏基线上：`analysis/tas-main-f22895/emu/`
   是 11-26 19:02 的旧段（早于 #111 的输入修复 `5fe39b88`），rust 腿却是当前 HEAD 新跑的。
   干净基线（全新 out、不用 `--resume`）的首分歧是 **f1301 `$0454`（2 B）**，不是 f22895。
3. **新的、与观测能力无关的不可能性论证**：`account_iteration_work`（`tick.rs:726` 调用、`:743-757`
   模型）的成本模型在本片**从未触发**——把 `work` 写进 spare `$07F0/$07F1` 后跑满 29936 帧，
   取值只有 `{19200×20670, 18900×7392, 18600×500, 18000×272, 19500×205, 0×452}`，
   **最大 28000 < `FRAME_CYCLES` 29780**，`iter_bogged(1)` 一次都没执行。
   ⇒ rust 的 64 次丢帧**全部**来自声明的迭代门（WaitNMIPass / 载房窗 / 卡死重试），与成本模型无关；
   **调 `BASE_FRAME_CYCLES`、`COLUMN_COST`、`enemy_frame_cost` 这类参数在结构上不可能改变丢帧位置**。
   这正是「逐事件预算」在本卡写面内做不到的机制理由（原 §0 给的是「观测不到」，方向错了）。
4. **丢帧账本已做成双边逐帧可对照（§3）**：emu 76 / rust 64，其中 **33 帧完全重合**（开机段
   `2,4,5,6,11-13,21-27,482,483,672-674,947,…`），`emu_only` 43、`rust_only` 31；
   **6001–18000 两万帧双方都零丢帧** ⇒ 漂移不是「每帧都差一点」，而是集中在载房/未定义房窗。
5. **上游要改的不是观察面**（已可观测），而是 `world/room_load` 声明的等待门数量与相位：
   主导分歧类为 `MainRoutine∈{3,4} ∧ RoomNumber==255`（`emu_only` 27 帧、`rust_only` 26 帧）。
   本卡写面是 `frame/`，改不到那里（§5 给了可派卡的改法与验收锚）。

## 1. 基线（2026-11-27 重做；原 §1 读数作废）

```
rm -rf /tmp/a2-fresh
python3 tools/tas_replay_diff.py replay \
  --inputlog movies/lordtom-metroid-100p.inputlog \
  --frames 30000 --segments 12000 --range 0400-04FF --tag ens \
  --out /tmp/a2-fresh          # 注意：**不带 --resume**（它会复用旧段结论，不与新 rust 腿重比）
```

- rust 腿 17.3 秒重建，29936 行；seg0 `compared=11962`，首个真实分歧 **f1301 `$0454` offset 84
  （emu `0x00` / rust `0x20`，2 B）**。
- seg1/seg2 从 f12000 / f24000 起整段分歧（28 B）。
- **2026-11-27 作废**：原文写的 f22895 `$0405`（8 B）与「rust 丢帧@f12000=27 / emu=36」都取自脏基线，
  撤回；干净基线的对应读数是 §3 的 43 / 54。

## 2. 变体矩阵（2026-11-27 重做；原 §2 全表作废）

原文 P1(/P2/V1 三个变体读数一并作废：它们跑在脏基线之上，且 P2 的「+400 cycles」在干净 HEAD 上
不产生任何差异（验收人复现），我把它当「数量凑对」的反例是不成立的。

在干净基线上重测（只跑 rust 腿：`rust_screenshot --schedule /tmp/a2-fresh/rust/full.sched`，每次 ~20 秒，
emu 侧用 §3 的逐帧真值作靶，不必重跑 emu）：

| # | 变体 | 改法（都在 `crates/game-core/src/frame/`） | rust 丢帧 | 与 emu 的逐帧 XOR |
|---|---|---|---|---|
| P0 | 基线 | 无 | 64 | 74 |
| VE | 末门同拍扩展到所有径 | `engine.rs:249` 去掉 `&& self.title_entry_mode` | **62** | **72** |

- VE 只动了 2 帧，且 **18001–24000 窗口一点没变（rust 21 / emu 12）** ⇒ 多出来的丢帧不在「末门收口」处。
- 结论：引擎侧门时相微调的杠杆只有 ±2 帧，无法改变量的分布；分布由生产侧声明的门队列决定。

## 3. 丢帧账本（新增，A 级）

**口径**：emu 侧 = `fc30000.frames.jsonl` 相邻帧 `$002D` 相等（NMI 内块跑了但没走到 `inc FrameCount`）；
rust 侧 = 状态行缺失（每次迭代恰好一行；29936 行 / 30000 帧 = 64 帧无行）。两侧用同一件事做锚。

| 窗口 | emu 丢帧 | rust 丢帧 | Δ |
|---|---|---|---|
| 1–6000 | 54 | 43 | −11 |
| 6001–12000 | 0 | 0 | 0 |
| 12001–18000 | 0 | 0 | 0 |
| 18001–24000 | 12 | 21 | +9 |
| 24001–30000 | 10 | 0 | −10 |
| 合计 | **76** | **64** | |

- 累计：f6000 54/43、f12000 54/43、f18000 54/43、f24000 66/64、f30000 **76/64**。
- 集合：`both` 33 帧（开机段全同：`2,4,5,6,11,12,13,21-27,482,483,672-674,947,948,…`）、
  `emu_only` 43 帧（首个 **f1574**，随后 1860/1903/1904/1928/2138/2165/2166/2346/2347…）、
  `rust_only` 31 帧（首个 **f1906**，随后 1907/2168/2169/2268/2269/2461/2462/4639/4640…）。
- **按 emu 侧状态分类（`emu_only`，即 emu 丢而 rust 不丢）**：
  `MR4/Room255/GM1` 18、`MR3/Room255/GM0` 7、`MR3/Room19` 4、`MR3/Room241` 3、`MR4/Room255/GM0` 2、
  `MR3/Room25` 2、`MR3/Room240` 2、`MR3/Room20` 2、`MR1/Room0` 2、`MR3/Room18` 1。
- **按 emu 侧状态分类（`rust_only`，即 rust 丢而 emu 不丢）**：`MR3/Room255` **25**、
  `MR3/Room243/241/242` 各 1、`MR2/Room255` 1、`MR8/Room255` 1、`MR1/Room0/GM1` 1。
- 局部形态：rust 的丢帧几乎总是**成对两帧**（1572/1573、1906/1907、2168/2169、2268/2269、2461/2462…），
  emu 在同一事件上多为**单帧**，且相位差 1–3 帧（例：rust 在 1572–1573 丢两帧、emu 在 1574 丢一帧；
  emu 在 1903–1904 丢两帧、rust 在 1906–1907 才丢）。
- 旁证（同一数据的另一投影）：`lag(f) = (f − FC(f)) mod 256` 在 f6000/12000/18000/24000/30000 上
  emu = 55/55/55/67/77、rust = 37/37/37/51/51，Δ = −18/−18/−18/−16/−26。它与上面的丢帧计数不同源
  （mod 256 + 开机初相），只作交叉校验：两万帧 ±0 与「6001–18000 零丢帧」互相印证。

## 4. 丢帧到底由谁决定（代码面）

- 迭代门：`crates/game-core/src/frame/engine.rs:236-257` `end_of_tick_gate` 每 tick 弹一门，
  `NMI_STATUS=0`（等待帧，内块照跑）/`=1`（bogged，内块跳过）；`:203-234` `settle_iteration` 是排空拍。
- span 收口：`engine.rs:228-233` `iter_span_fc` 单增；`tick.rs:329-332` 的 `title_chain_iter` 分支。
- 生产侧声明源：`engine.rs:217` 调 `crate::world::room_load::room_load_gate_step`；载房窗门数由
  `world/room_load.rs` 决定 —— **不在本卡写面**。
- 成本模型：`tick.rs:743-757`，本片最大值 28000 < 29780（§0.3），从不触发 ⇒ 不是杠杆。

## 5. 上游改法（可派卡；与 #110 §10.7 不同，这里给的是本卡实测出的、更窄的靶）

1. **靶已量化**：把 §3 的 emu 丢帧集合（76 帧，逐帧列表随 `analysis/traces/fc30000.frames.jsonl` 可再生）
   当作 `world/room_load` 等待门计数的验收锚，要求 **逐帧 XOR = 0**（比卡面的「|Δ|≤1」更强）。
2. **改法**：按 ROM 三个已知等待点逐段核对载房/卡死转场窗内到底空转几拍——
   `00:909F` `ClearAll`（2 个等待）、`07:E54A` `check_update_name_table`、`07:C114` `ScreenOn`
   （各自的自旋循环数），把 `world/room_load.rs` 声明的 `iter_wait` 数量与相位与之对齐；
   rust 现在的形态是「每个事件丢 2 帧」，emu 是「1 帧、相位差 1–3 帧」，差的就是这一处。
3. **验收**：§3 两列累计数在 5 个窗口上全等（54/0/0/12/10），且 0..30000 的 ens 首分歧 > f1301 或 null；
   `password-justin` 14/14；`run-all` ≥171/172。
4. 可选：给 `tools/scenario_run.py` 的 `watch` 加一个「丢帧账本」汇总（相邻帧 `watch` 值相等即记一帧），
   免得每张卡都自己写比对脚本。**本轮不改 `tools/`**。

## 6. 复跑命令

```
# 1) 干净基线（不带 --resume）
rm -rf /tmp/a2-fresh && python3 tools/tas_replay_diff.py replay \
  --inputlog movies/lordtom-metroid-100p.inputlog --frames 30000 \
  --segments 12000 --range 0400-04FF --tag ens --out /tmp/a2-fresh

# 2) emu 逐帧 FC（约 8.5 分钟）
python3 tools/scenario_run.py run analysis/scratch-fc/fc30000.txt --timeout 1800
#    → analysis/traces/fc30000.frames.jsonl（30000 行，逐帧 $002D/$002E）

# 3) rust 单腿（~20 秒，不重跑 emu）
cargo run -q --release -p game-core --bin rust_screenshot -- \
  --schedule /tmp/a2-fresh/rust/full.sched --out /tmp/x/shots --state-out /tmp/x/rust.jsonl
```

## 7. 自查

- 本卡**零生产代码改动**：变体跑完全部还原（`git diff -- crates/game-core/src/frame/` 为空）。
  取证期间在 `tick.rs` 临时写过 `[PROBE-114]`（把 `work` 写进 `$07F0/$07F1`），**已删除**。
- 作废清单（按 §3.6 就地标注）：原 §0（不可观测论）、原 §1 基线读数、原 §2 变体矩阵、原 §3 三条
  「不可观测」理由中的第 2/3/4 条（12 checkpoints / ens 探针 / bit6 翻转数）——后三条作为「辅助面不可用」
  仍然成立，但**不再是瓶颈**，因为逐帧 FC 已可直接采。
- 回归：见提交证据（`check`、`cargo test --workspace`、`password-justin`、`run-all`）。

## 8. 2026-11-27 #119 补测：f1574 之前两侧**逐段完全对齐**（改锚 A 的取证）

> 锚定依据：#119 thread seq3「PMO 自动决策：按最高价值建议执行」选 A——本卡验收锚改为「f1574 前逐帧 XOR=0 + 逐过渡窗口长度表 + 结构性事实（事件类别→窗口长度），f1574 之后归 #120」。
> 本节全部读数在 HEAD `df899424` 上取得；`world/`、`tools/` 零改动；探针只写 `analysis/`（gitignored）。

### 8.1 数据面与取数口径

| 工件 | 命令 | 行数 | 说明 |
|---|---|---|---|
| emu 逐帧 | `tools/scenario_run.py run analysis/scratch-fc/fcstate.txt`（`watch 002d 001e 001d 005a 001a`） | 30000 | `analysis/traces/fcstate.frames.jsonl`，wall 510.9s |
| emu 逐帧（含门态） | `tools/scenario_run.py run analysis/scratch-fc/fcdoor.txt`（`watch 002d 0056 0058 001e 001a 005a`） | 1700 | `analysis/traces/fcdoor.frames.jsonl`，wall 39.6s（#119 新增，用于区分门径/滚屏径） |
| rust 单腿 | `cargo run -q -p game-core --bin rust_screenshot -- --schedule … --state-out /tmp/a2-fresh/rust/full.state.jsonl` | 29936 | 干净 HEAD 重建，未用 `--resume`（#115 后 `--resume` 才绑 rust 腿锚） |

**丢帧定义（两侧必须同义，否则读数不可比）**

- emu：相邻帧 `$002D`（FrameCount）相等 ⇒ 该帧没有推进一次迭代（ROM 每完成一次主循环迭代才 `inc FrameCount`，`prg7_engine.asm:205-209`）。
- rust：状态行缺失 ⇒ `FrameEngine` 在 span 内抑制行导出，一行对应一次迭代完成。

### 8.2 关键读数

```
emu drops ≤f1573 = 27   rust drops ≤f1573 = 27   XOR = 0      ← 逐帧集合相同，不是数量凑巧
emu drops ≤f30000 = 76  rust drops ≤f30000 = 64  XOR = 74  首个不一致 f1574
（emu_only 43 / rust_only 31）
```

复算脚本（无依赖，直接跑）：

```bash
python3 - <<'EOF'
import json
emu=[(o['f'],o['w']['002d']) for o in map(json.loads,open('analysis/traces/fcstate.frames.jsonl'))]
ed={b[0] for a,b in zip(emu,emu[1:]) if b[0]==a[0]+1 and a[1]==b[1]}
rust={json.loads(l)['f'] for l in open('/tmp/a2-fresh/rust/full.state.jsonl')}
rd={f for f in range(1,30001) if f not in rust}
E=lambda s:{f for f in s if f<=1573}
assert len(E(ed))==len(E(rd))==27 and not (E(ed)^E(rd))
assert len(ed)==76 and len(rd)==64 and len(ed^rd)==74 and min(ed^rd)==1574
EOF
```

### 8.3 逐过渡窗口长度表（f1574 之前，两侧逐段相同）

`len` = 本次载房被吞掉的帧数；`$56/$58` 取自 `fcdoor` 探针在窗口起始帧的值；`room` 是**目标房号**（屏关期间 `$005A` 已经写入新房号，不是 `$FF`）。

| 窗口起点 | len | 事件类别 | `$56` | `$58` | 目标房 | ROM 依据 |
|---:|---:|---|---:|---:|---:|---|
| 2 | 1 | 开机（`MainRoutine=0`） | 0 | 0 | 0 | `00:909F ClearAll` 的等待 |
| 4 | 3 | 开机（`MainRoutine=0`） | 0 | 0 | 0 | `00:92D4 InitializeGame` 链 |
| 11 | 3 | 开机（`MainRoutine=0`） | 0 | 0 | 0 | 同上（第二段装载） |
| 21 | 7 | 开机→标题（`MR 0→1→2→8`） | 0 | 0 | 0/9 | 7 次 NMI 等待（最长的初始化链） |
| 482 | 2 | 滚屏载房（`MR=3`） | 0 | 0 | 23 | `07:C7xx` MoreInit 链 + `07:E54A` |
| 672 | 3 | 滚屏载房（`MR=3`） | 0 | 0 | 8 | 同上，**多 1 帧**（见 §8.4） |
| 947 | 2 | 滚屏载房 | 0 | 0 | 9 | 同上 |
| 1136 | 2 | 滚屏载房 | 0 | 0 | 20 | 同上 |
| 1309 | 2 | 滚屏载房 | 0 | 0 | 19 | 同上 |
| 1572 | 3 | **门径载房**（`$56=1`、`$58=17`） | 1 | 17 | 24 | 门入场多走一格尾段（`room_load_tail==2 && $56 != 0`） |

### 8.4 窗口长度不是常数：它由事件类别决定

表里有 4 种长度（1/2/3/7），当前模型**已全部正确表达**。两条可直接核对的区分：

1. **门径 vs 滚屏径**：`f1572` 的窗口 `$56=1`、`$58=17` ⇒ 是穿门载房，长度 3；同类的滚屏载房（`f482/f947/f1136/f1309`）长度都是 2。差的那 1 帧对应 `crates/game-core/src/world/room_load.rs` 的「门入场时 `room_load_tail==2 && $56 != 0` 多补一格」。
2. **`f672`（目标房 8）也是 3 帧但 `$56=0`**：它不是门径，而是**该房 `SetupRoom` 本身就跨 3 帧**。A 级依据（不是我本轮新证，是既有的 Mesen 逐房实录）：`crates/game-core/src/world/room_load.rs:210` 记录「Mesen 逐帧实测 room0/23/8 = 冻结 1/2/3 帧」，同文件 `:380` 与 `docs/rounds/r9-task-room-load-task-machine-review.md:31` 复用同一组数。
   > 【2026-11-27 就地更正】本节初稿把这条写成「属性表超单格 3 次 `attrib_table_write` 的 B 级猜测」——**该猜测作废**：正确依据是已有的逐房帧数实录（room0=1、room23=2、room8=3），与属性表容量无关。本轮表格中的 `482→房23=2`、`672→房8=3` 与该实录逐项吻合，可作为交叉验证。

### 8.5 为什么 f1574 之后的差异不归本卡

- 首个**内容**分歧是 f1301（`$0454` offset 84，emu `0x00` / rust `0x20`，2 字节），属 #120。
- 首个**丢帧**不一致是 f1574：emu 那次载房吞 3 帧（1572 起，跨到 1574），rust 只吞 2 帧（1572-1573）。
  > 【2026-11-27 就地更正，见 §8.8】本节初稿把这条写成「载荷状态在 f1301 之后已经跑偏，窗口长度差 1 是结果不是原因」——**该说法作废**。逐字节比对显示载房**起点的状态两侧完全相同**（f1568–f1571 的 `$56/$58/$005A/$001E/$002D` 逐帧相等），所以这 1 帧差是**载房窗本身的长度差**（门径 3 帧 vs 2 帧），属本卡题面，不是 #120 的下游。
- 因此「30000 帧 76 帧 XOR=0」在 #120 修好之前不可达；硬对会把「拟合窗口长度」当成功。

### 8.6 建议的回归棘轮（可守住、不依赖 #120）

```bash
# f1574 之前逐帧丢帧集合必须完全相同（27/27，XOR=0）
python3 tools/fc_drop_ledger.py --emu analysis/traces/fcstate.frames.jsonl \
                                --rust /tmp/a2-fresh/rust/full.state.jsonl --until 1573
```

`fc_drop_ledger.py` 尚未落地（本卡写面只到 `world/` 与本文档）；若 PMO 要把它做成常驻棘轮，需另开一张 `tools/` 卡。在那之前，用 §8.2 的 heredoc 复算等价。

### 8.7 未覆盖 / 仍需上游

| 缺口 | 状态 |
|---|---|
| f1574 之后 50 帧差异 | 归 #120（f1301 内容分歧） |
| 逐房冻结帧数目前是**实测表**（room0=1/23=2/8=3）而非派生规则 | A 级事实，但若要在新房里推广需补全表或找出派生量（`RoomPtr` 分块数） |
| 常驻棘轮脚本 `tools/fc_drop_ledger.py` | 未落地（写面外） |

### 8.8 f1574 的最小复现：门径载房窗少 1 帧（★ 推翻 §8.5 初稿的「下游」判断）

**比较方法**：取同一组零页地址（`$56` 门入口状态、`$58` 门数据、`$5A` 房号、`$1E` MainRoutine、`$2D` FrameCount），emu 用 `analysis/traces/fcdoor.frames.jsonl`、rust 用同一帧的 `zp` 段（均在零页，不是 iram）。

| 帧 | emu `$56 $58 $5A $1E $2D` | rust `$56 $58 $5A $1E $2D` | 备注 |
|---:|---|---|---|
| 1568 | 129 17 255 3 6 | 129 17 255 3 6 | 完全相同 |
| 1569 | 129 17 255 3 7 | 129 17 255 3 7 | 相同 |
| 1570 | 129 17 255 3 8 | 129 17 255 3 8 | 相同 |
| 1571 | **1** 17 255 3 9 | **1** 17 255 3 9 | 进门瞬间也相同（`$56` 129→1） |
| 1572 | 1 17 **24** 3 **9** | （无行） | emu 冻结；rust 此处也在冻结 |
| 1573 | 1 17 24 3 9 | （无行） | 同上 |
| 1574 | 1 17 24 3 **9** | 1 17 **241** 3 **10** | **分歧点**：emu 还冻着、rust 已收口 |
| 1575 | 1 17 241 3 10 | 1 17 255 3 11 | 错位保持 |

**结论（A 级）**

1. 载房**起点状态完全一致**（f1568–f1571 五元组逐帧相等，包括 `$56` 由 129→1 的进门瞬间）⇒ 分歧不是上游内容跑偏的结果。
2. **emu 的门径载房窗 = 3 帧**（FC 在 f1572/1573/1574 三次冻结），**rust = 2 帧**（f1572/1573）⇒ 本卡题面的「等待门计数」在这条径上**确实少一格**。
3. 少的那一格正是代码里已声明、但**在此径未触发**的规则：`crates/game-core/src/world/room_load.rs` 的「门入场时 `room_load_tail==2 && $56 != 0` 多补一格」。

**为什么没触发（待验，需写面）**：`$56` 在整个窗里恒为 1（见上表），所以最可能是**判定时机**问题——该条件在 `room_load_tail` 走到 2 的那一拍求值，而那一拍 `$56` 是否已从 129 落到 1、或 `room_load_tail` 是否真的经过 2，需要在 `room_load_gate_step` 里打点确认。**验证与修复都需要 `crates/game-core/src/world/` 写权限（本卡写面内），但当前 `claim` 被 `blockedBy=#117` 拒，尚未进行。**

**这条同时改写本卡结论**：不是「f1574 之前对、之后归 #120」，而是「f1574 之前对；f1574 起是**门径窗长少 1 帧**，属本卡，可在 `world/room_load.rs` 修」。#120（f1301 的 `$0454` 2 字节）是另一条独立的、更早的分歧，不解释这里。

**实验（否证，2026-11-27 在 `HEAD 620e8057` 的临时 worktree `/tmp/wt-119` 上做，主线未动）**：把门径的「多补一格」从「同一拍连推两格」改成「多占一拍」（`room_load_tail=4` 后下一拍收口），重跑 1700 帧 → **窗口长度完全没变**（仍是 1572-1573 两帧，f1574 的 FC 仍为 10）。

⇒ **结论：载房窗长度不是由 `room_load_tail` 的相位步数决定的，而是由 `iter_gates` 里推入的门数决定**（`crates/game-core/src/frame/engine.rs:236` 每拍弹一个门，门没弹完就不 `settle_iteration`）。所以正确的修点在**门数**（`room_load.rs:37/72/105` 三处 `iter_wait`/`iter_bogged` 推入门队列的地方，门入场时是否少推一个），不在相位推进。

**下一次探针要打的位置**（本轮没做，因为写面未解锁）：在 `iter_wait`/`iter_bogged` 的推入点连同 `iter_gates.len()` 一起打点，对比「滚屏载房 2 门 / 门径载房 emu 需 3 门」在推入时刻的差异。

### §8.9 缺陷 A 的第一个修复尝试（有改进但不够，附实测序列）

按「一帧一次 `attrib_table_write`」改 `advance_room_load_tail`（把 `while room_number()!=target` 改成「没到目标就只调一次并 return」），在 `HEAD a61a84e2` 的临时 worktree 里跑 1700 帧，与 emu 逐帧比 `$5A`：

| 载房点 | emu | 改后 rust | 判读 |
|---|---|---|---|
| f672 | `F0 F1 F2 F3 F4 F4 FF` | `F1 F1 F2 F3 F3 F4 FF` | **中间步回来了**（F2/F4 都出现），但整体后移一格、`F3` 多占一帧 |
| f1309 | `F0 F2 F3 FF` | `F1 F1 F2 F3 F3…` | 出现 `F0` 缺失、`F1` 占两帧 |
| f1572（门径） | `F1 F1 F3 FF` | `F1 F1 F2 F3 F4 FF` | 反而多出 `F2/F4` 两帧 |

**结论**：粒度从「一帧一整格」改成「一帧一次调用」方向是对的（中间步确实回来了），但**仍对不上**，因为 Rust 现在的游标是「目标值表」驱动（`match tail { 0=>0xF1, 1=>0xF3, _=>0xFF }`），而 ROM 的游标是**自增 + `and #$0F` 查表**（`AttribTableWrite ; 07:EA13`）：每个值都可能是 ExitSub（返回、本帧到此为止）或写一列。

⇒ 正确的实现应把尾段改成**逐值状态机**：每帧调一次，`inc RoomNumber` → `and #$0F` 查 5 项表 → ExitSub 就结束本帧、WritePPUAttribTbl 写一列、RoomFinished 收口。这样 `F0/F1/F2/F3/F4` 各自的行为由表决定，不再需要 `target` 表，也不会出现「多一帧/少一帧」的整格误差。

本轮实验的 worktree 与产物已删除，主线未动。下一步（需写面）按「逐值状态机」重写 `advance_room_load_tail`，验收锚仍是上表 emu 三列。

### §8.10 缺陷 A 的第二版修复：**f672 逐帧全等**（补丁已存盘，未上主线）

读 ROM 原文 `AttribTableWrite ; 07:EA13`：

```asm
AttribTableWrite:
    lda RoomNumber
    and #$0F          ; 用**旧值**选表
    inc RoomNumber    ; 先加一
    jsr JumpEngine
        .word ExitSub            ; 0
        .word WritePPUAttribTbl  ; 1
        .word ExitSub            ; 2
        .word WritePPUAttribTbl  ; 3
        .word RoomFinished       ; 4（RoomNumber=$FF）
```

⇒ 一次调用只前进一个值，动作由**旧值**决定。据此改了两处（补丁 `analysis/scratch-fc/tail_state_machine.patch`，gitignored）：

1. 相位 1→2 那一拍**只置 `$F0`、不再同拍推进游标**（emu 在采样点上先看到 `$F0` 整整一帧）；
2. `advance_room_load_tail` 改成**一帧一次 `attrib_table_write`**（不再 `while` 走完整格）。

实测（`HEAD a61a84e2` 临时 worktree，跑 1700 帧）：

| 载房点 | emu | 改后 rust | 结果 |
|---|---|---|---|
| **f672** | `F0 F1 F2 F3 F4 F4 FF` | `F0 F1 F2 F3 F4 F4 FF` | **逐帧全等** |
| f1572（门径） | `F1 F1 F3 FF` | `F0 F1 F3 FF` | 只差开头一帧（= 缺陷 B 的冻结帧数差） |
| f1309 | `F0 F2 F3 FF` | `F0 F1 F2 F3 F4 FF` | emu 在该处少两次调用（`F1`/`F4` 未出现），是「一帧两次 ScrollDoor 调用」的差异，需另查 |

**结论**：缺陷 A 的修法已被证实（f672 逐帧全等），剩下的是
- 门径那条差一帧 = 缺陷 B（冻结帧数 3 vs 2，`scroll.rs:75` 的 `dropped`）；
- f1309 那条 = 每帧 `AttribTableWrite` 的**调用次数**（asm `:2027-2028` 一帧两次 `jsr ScrollDoor`），不是游标粒度。

补丁未上主线（本卡写面未解锁），`git apply` 即可用。

## 8.13 逐值状态机（变体 B）：一次调用只推一格 —— 实测一轮

§8.10 的修法是「一帧一次 `attrib_table_write`，相位切换那拍只置 `$F0`」，f672 已逐帧全等。
本轮把 `advance_room_load_tail` 里「目标值表 + while」彻底去掉，改成**逐值状态机**：

- `advance_room_load_tail` = **一次 `attrib_table_write`（ROM `AttribTableWrite ; 07:EA13` 的一次 `jsr`）+ 计数**；
- 相位 1→2 那拍只置 `$F0`、不调用；
- 相位 2 每拍一次调用，`RoomNumber == $FF`（`RoomFinished`）即收口；
- 去掉 #104 时期加的「门径多补一格」。

实测（HEAD `a61a84e2` + 该改动，rust 腿 29936 行，emu 侧 `analysis/traces/fcdoor.frames.jsonl`）：

| 载房点 | emu `$5A` | 变体 B |
|---|---|---|
| **f672** | `F0 F1 F2 F3 F4 F4 FF`（675–681） | `F0 F1 F2 F3 F4 **FF** FF`（675–680）**只差 f680 那一格** |
| f1309 | `F0 F2 F3 FF`（1311–1314） | `F0 F1 F2 F3 F4 FF`（1311–1316） |
| f1572（门径） | `F1 F1 F3 FF`（1574–1577） | `F0 F1 F2 F3 F4 FF`（1574–1579） |

三点的读法（按 §8.3 的窗口长度表对齐，不按帧号硬比 —— 该区间两侧有丢帧差）：

1. **尾段序列本身是对的，只差「`$F4` 多停一帧」**：emu 在 `F4` 上停两帧才到 `FF`，Rust 一帧就到 `FF`。
   ROM 侧解释只能来自「那一帧没有发生 `jsr SetupRoom`」（`CheckUpdateNameTable` 被调用次数不足），
   不是在 `AttribTableWrite` 内部（`4 → RoomFinished` 是 `sta $5A = $FF` 一条拍内完成，
   见 `crates/game-core/src/world/room_draw.rs:980-983`）。
2. **f1309 / f1572 的序列比 emu「长」**：emu 那两处总共只走了 2–3 步就 `FF`，Rust 走了 5 步。
   与 §8.12 同因：emu 在该帧窗口里 `jsr ScrollDoor` 的次数更少（`F1`/`F4` 这两格整帧缺席），
   属**调用次数**问题，不是游标粒度问题 —— 因此本变体在这两点上仍是超集。
3. **没有回归面**：该变体只改 `crates/game-core/src/world/room_load.rs` 一个函数，
   f672 的正面成果保留（`F0 F1 F2 F3 F4` 五步全对）。

结论：**「逐值状态机」是正确方向且已在最难的那条锚（f672）上验证，
但剩下的两处差异（`$F4` 多停一帧 / 一帧内 `ScrollDoor` 调用次数）都由
「一帧发生了几次 `jsr SetupRoom`」决定，属调用方（`camera/scroll.rs` 的 `dropped` 与
`ScrollDoor/ScrollRight` 的调用节奏），不在 `room_load.rs` 内部能闭合。**

补丁：`analysis/scratch-fc/tail_percall_b.patch`（65 行，仅 `room_load.rs`；本卡写面未解锁，未上主线）。

## 8.14 变体 C：把尾段改由「每次滚动调用」驱动（ROM 形）—— 形状对上了，起点差一帧

§8.13 的结论是「差异由每帧 `jsr SetupRoom` 次数决定」。本轮把它实现成 ROM 形：
`CheckUpdateNameTable ; 07:E54A` 首件是**无条件** `jsr SetupRoom`，尾段时 `SetupRoom ; 07:EA2B`
直接 `bcs AttribTableWrite` ⇒ **每次滚动调用推一格**。改动两处：

- `crates/game-core/src/world/camera/scroll.rs`：`check_update_name_table` 开头加
  「相位 2 ⇒ 调一次 `attrib_table_write` + 计数，`$FF` 即收口并返回」；
- `crates/game-core/src/world/room_load.rs`：相位 2 的门步**不再推进尾游标**（只保留收口前状态）。

实测（HEAD `a61a84e2`，rust 腿 29936 行 vs `analysis/traces/fcdoor.frames.jsonl`）：

| 载房点 | emu | 变体 B（一帧一格） | 变体 C（一次调用一格） |
|---|---|---|---|
| f672 | `F0 F1 F2 F3 F4 F4 FF` | `F0 F1 F2 F3 F4 FF`（少一次 `F4`） | `F1 F2 F3 F4 FF`（整体早一帧、少一次 `F4`） |
| f1309 | `F0 F2 F3 FF` | `F0 F1 F2 F3 F4 FF`（多两格） | `F1 F3 F4 FF`（早一帧） |
| f1572 | `F1 F1 F3 FF` | `F0 F1 F2 F3 F4 FF` | `F1 F3 FF`（早一帧、少一次 `F1`） |

判读：

1. **形状对了**：C 在 f1309/f1572 上第一次出现「一帧推 2 格」（`F1→F3→F4`），这正是 emu 的形态；
   B 永远一帧一格，结构上不可能匹配。
2. **起点差一帧**：C 在窗口结束的那一拍就开始推（`F1`），emu 那一拍停在 `F0`（即「窗末帧不再发生
   `jsr SetupRoom`」，要等下一帧的第一个滚动调用）。修法应是「相位 1→2 的那一拍登记
   `tail_armed=false`，下一帧第一次滚动调用才置 true 并开始推」，属**本地可改**。
3. **还剩「一次重复帧」**：emu 在 `F4`（f672）/`F1`（f1572）各多停一帧 ⇒ 每个窗口比 C 少一次调用；
   与 §8.12 同因（emu 该帧 `ScrollX` 没跨过 kick 阈值 ⇒ 那帧零调用）。这一条要靠
   「调用次数的判据」而不是「推几格」。

结论：**两条腿都已验证到位——B 证明「一格=一次调用」的粒度正确（f672 前五步全对），
C 证明「一帧几次调用=走几格」的结构正确（f1309/f1572 的形态首次对上）。
合成版 = C + 窗末帧 arm 延迟 + 每帧调用次数按 `ScrollX` kick 判据，是本卡写面内可完成的收口路径。**

补丁：`analysis/scratch-fc/tail_percall_c.patch`（38 行，`scroll.rs` + `room_load.rs`）。

## 8.15 变体 D：C + 窗末 arm 延迟 —— 读数与 C 完全相同（18/1700 失配）

把「窗末那一拍只 arm、不推格」也加上（`room_load_tail == 0` 作未 arm 哨），实测与 C **逐帧相同**
（0–1700 帧失配 18 格，三个锚的序列一字不差）。说明：

1. **窗末帧本来就没有滚动调用**：phase 1→2 发生在 `Engine::end_of_tick_gate`（主循环之后），
   那一拍的 `jsr ScrollDoor/ScrollRight` 早在相位 1 里跑完了 ⇒ arm 标志是空转。
2. **C/D 相对 emu「早一帧 + 少一次重复」的原因不在尾段驱动**：`$5A` 序列的起点由**载房冻结窗有多长**
   决定，而 Rust 在该处的 drop 行数与 emu 不同（f672–674 三条锚上 rust 缺行、emu 有值）。
   这正是缺陷 B（门径窗 emu 3 帧 / rust 2 帧）与 `camera/scroll.rs:75` 的 `dropped` 公式。
3. 因此「尾游标」这条线已探到底：**驱动方式（一次调用一格）与粒度（一格=一次调用）都验过了，
   余下差异全部可归因到冻结窗长度**——那是同一个窗口的另一半，必须和 `dropped` 一起修，
   单改尾段不可能收敛。

补丁：`analysis/scratch-fc/tail_percall_d.patch`（与 C 等价，留档备查）。

## §8.16 #121：推进点从「每帧」搬到「每次滚动调用」（实测）

### 改动（`crates/game-core/src/world/`，13+/30−）

- `camera/scroll.rs::check_update_name_table`：尾段（`room_load_phase != 1`）也调 `setup_room`——ROM `CheckUpdateNameTable ; 07:E54A` 首件就是 `jsr SetupRoom`，`RoomNumber >= $F0` 时由 `SetupRoom ; 07:EA2B` 的 `bcs AttribTableWrite` 走 `AttribTableWrite ; 07:EA13` 一步。
- `room_load.rs::room_load_gate_step` 相位 2：**不再**每帧推一格，只判收口（`RoomNumber == $FF` → 相位归 0）。

### 三锚实测（`HEAD a61a84e2` + 本改动；rust 腿 `rust_screenshot --state-out`，29936 行、7.2s）

| 锚 | emu（`analysis/traces/fcdoor.frames.jsonl`） | rust（改后） | 结论 |
|---|---|---|---|
| f672（675–681） | `F0 F1 F2 F3 F4 F4 FF` | `F0 F1 F2 F3 F4 F4 FF` | 逐帧全等 |
| f1309（1311–1314） | `F0 F2 F3 FF` | `F0 F2 F3 FF` | 逐帧全等 |
| f1572（1574–1577） | `F1 F1 F3 FF` | `F0 F2 F4 FF` | 起点差一格（见下） |

≤f1700 的逐帧失配：**21 → 7**（`484–487`、`1574–1576`；载房窗内因 FC 冻结被吞的行不计）。

### 残余差异的机制（不是粒度问题）

ROM 里载房在**某一次滚动调用内**完成，该帧**后续的滚动调用**继续推游标；Rust 用丢帧门吞掉整拍，完成帧只剩一次调用：

- 完成帧只有 1 次滚动调用（f672 room8 / f1309 room19）⇒ 无差，两锚全等；
- 完成帧有 2 次滚动调用（f484 room23 / f1572 门径 room24）⇒ rust 少推 1 格，表现为起点 F0 vs F1。

判据与实测支撑：`crates/game-core/src/world/camera/scroll.rs:75` 的 `dropped` 只在载房帧推门，`crates/game-core/src/frame/engine.rs:236` 每拍只弹一门 ⇒ 冻结期内的后续调用被整体吞掉，而 ROM 的 `jsr SetupRoom` 是逐调用生效的。

### 未做（需 `frame/` 写面）

要让 f1572/f484 也对上，需要「完成帧内剩余滚动调用数」这个量，而它只能由 `frame/`（迭代/门队列）提供；本卡写面（`world/`）内拿不到每帧调用计数。故按卡面第 3 条以「调用次数问题」收口。

## §8.17 返工（2026-11-27）：契约更新、探针真值、死代码挂回

白砚退回三条（2 阻断 + 1 清理）后逐条处置。

### 8.17.1 阻断 1：3 条单测按新契约改写（不是放宽断言）

旧契约「窗末一拍推一格、之后每拍一格」被新推进点（每次滚动调用一格）取代，旧断言必然红。
改法是**把断言改成新契约并给出 emu 依据**，未删除任何覆盖：

| 测试 | 旧断言 | 新断言 |
|---|---|---|
| `tail_walks_f1_f3_ff` → `tail_advances_one_step_per_setup_room_call` | gate_step 三次 ⇒ F1→F3→FF | 窗末只置 `$F0`（不推）⇒ 五次 `setup_room` ⇒ F1→F2→F3→F4→FF ⇒ gate_step 收口 |
| `door_scroll_compensates_one_tail_step` → `tail_does_not_step_in_gate_step_even_for_door_scroll` | 门入场时补推一格（F3→FF） | 门入场/非门入场**都不由 gate_step 推进**；一次 `setup_room` 一步（补偿失去对象，见 8.17.3） |
| `task_returns_to_idle_and_allows_next_load` | 三次 gate_step 走到 FF | 五次 `setup_room` 走到 FF ⇒ gate_step 收口 ⇒ `room_load_tail == 5` ⇒ 新请求可见 |

`cargo test -p game-core --lib world::room_load::tests_room_load_task` ⇒ **6/6 绿**（旧 3 红全消）。

### 8.17.2 阻断 2：`dropped` 探针真值（临时插桩，取数后已移除）

`world/camera/scroll.rs` 的 `while` 前一拍打印 `(LOAD_BASE_CYCLES, work_cycles, dropped)`，跑 29936 帧实测：

```
[PROBE-121] load_base=28454 work=46456 dropped=2
[PROBE-121] load_base=28454 work=71090 dropped=3
[PROBE-121] load_base=28454 work=43378 dropped=2
[PROBE-121] load_base=28454 work=31446 dropped=2   （另有 5 条同为 dropped=2）
```

⇒ `LOAD_BASE_CYCLES = 28454`、`work_cycles ∈ [31446, 71090]`、`dropped ∈ {2,3}`
（`(28454+71090)/29780 = 3.34` ⇒ 3；其余 `(28454+~45000)/29780 ≈ 2.4` ⇒ 2）。
与 emu 实测窗长一致（room8 = 3 帧、其余 = 2 帧）。**未改 `LOAD_BASE_CYCLES`**。

### 8.17.3 清理项：`advance_room_load_tail` 挂回调用点（不再是死代码）

改为由 `setup_room` 的 `RoomNumber >= $F0` 分支调用——那正是 ROM `jsr SetupRoom ; 07:EA2B`
的 `bcs AttribTableWrite ; 07:EA13` 分支，也是 `room_load_tail` 计步的唯一真源。
旧「门入场补推一格」的补偿随之删除：推进已按调用计，补偿会把同一事件算两次。

### 8.17.4 回归

- `gates/cli.py check` → **PASS=24 FAIL=0 SKIP=3 TOLERATED=1**（9.86s）。
- 三锚：f672、f1309 逐帧全等；f1572 `F0 F2 F4 FF` vs emu `F1 F1 F3 FF`（起点差一格，
  机制见 §8.16 三：载房完成帧的后续滚动调用被丢帧门吞掉）。≤f1700 失配 7 帧。
- `arch-coupling`：新测试里 `write_ram(DOOR_ENTRY_STATUS)` 被判新增直接依赖 ⇒ 改用
  typed 访问器 `set_door_entry_status()`，门禁回到 PASS。

## §9 #119 收口（2026-11-27）：world/ 侧已对齐的部分 + 残余 72 帧的归因

### 9.1 修复前/后的丢帧账本（同一口径，逐帧）

工件：emu 逐帧 FC/状态 `analysis/traces/fcstate.frames.jsonl`（30000 帧，watch `002d 001e 001d 005a 001a`）；
rust 腿 `rust_screenshot --schedule /tmp/a2-fresh/rust/full.sched --state-out /tmp/r119.jsonl`（29938 行，HEAD `a61a84e2` + 本卡/#121 的 `world/` 改动）。
丢帧定义（两侧同义）：emu = 相邻帧 `$002D` 相等；rust = 该帧无状态行（一次迭代被吞）。

| 读数 | emu | rust | 交集 | XOR |
|---|---:|---:|---:|---:|
| 丢帧总数 | **76** | **62** | 33 | **72** |
| 首个不一致 | — | — | — | **f1574** |
| 分窗 emu_only / rust_only | 1–6000：27 / 14 | 6001–12000：0 / 0 | 12001–18000：0 / 0 | 18001–24000：6 / 15 | 24001–30000：10 / 0 |

与 #110 基线（76 / 64 / XOR 74）相比，本卡与 #121 的 `world/` 改动把 XOR 从 74 降到 **72**（rust 少丢 2 帧），方向正确但**未达 XOR=0**。

### 9.2 已落地（`crates/game-core/src/world/`，本卡写面）

| ROM | 语义 | Rust 落点 |
|---|---|---|
| `CheckUpdateNameTable ; 07:E54A` | 每帧首件无条件 `jsr SetupRoom` | `camera/scroll.rs::check_update_name_table` 尾段（`room_load_phase != 1` 也调） |
| `SetupRoom ; 07:EA2B` | `RoomNumber >= $F0` ⇒ `bcs AttribTableWrite` | `room_load.rs::setup_room` 的 `RoomNumber >= $F0` 分支调 `advance_room_load_tail` |
| `AttribTableWrite ; 07:EA13` | 一次调用前进一格（`and #$0F` 查 5 项表） | `advance_room_load_tail` 每次调用 `attrib_table_write` 一次 |
| `00:909F ClearAll` / `07:C114` | 开机与载房的固定等待点 | `frame/boot.rs:287/289/308/319/332/334/347/349/365/383`、`world/room_load.rs:37/105` |

验收锚（`$5A` 逐帧）：f672 `F0 F1 F2 F3 F4 F4 FF` **全等**；f1309 `F0 F2 F3 FF` **全等**；f1572 仍差一格（见 9.4）。

### 9.3 残余 72 帧的分类（按两侧状态真值）

emu_only 43 帧，按 emu `(MainRoutine, RoomNumber)`：`(4,255)`×20、`(3,255)`×7、`(3,19)`×4、`(3,241)`×3、`(3,25)`×2、`(3,240)`×2、`(1,0)`×2、`(3,20)`×2、`(3,18)`×1。
rust_only 29 帧，按最近可用 rust 行：`(4,255)`×17、`(3,255)`×5、`(3,240)`×5、`(2,255)`×2。

两个特征：
1. **主体在 `RoomNumber=$FF` 的卡死重试窗**（`MainRoutine` 3/4）——这正是不在 `account_iteration_work` 覆盖内的第三类窗；
2. **中间两万帧两侧都零丢帧**（6001–18000），说明常态路径已对齐，差异集中在**转场/载房窗**。

### 9.4 为什么 XOR=0 在 `world/` 内不可达（两条硬边界，均已实测）

1. **成本模型对载房窗完全无效**：`crates/game-core/src/frame/tick.rs:739` 在 `room_load_phase != 0` 时直接 `return`，不记账。实测：把 `AttribTableWrite` 记 4100 周期后重跑 1700 帧，9 个载房点一个都没变。
2. **完成帧的后续滚动调用被丢帧门吞掉**：`world/camera/scroll.rs:75` 的 `iter_bogged(dropped)` 与 `frame/engine.rs:236` 的「每拍弹一门」让整拍被吞，ROM 却是「该帧内继续调 `jsr ScrollDoor`」。这解释 f1572 的 `F0` vs `F1`（起点差一格）。

两条都在 `frame/` 写面。按卡面第 3 条的后半句（「或明确列出剩余差异帧与原因」）在此列明；**不退回拟合**（未调 `LOAD_BASE_CYCLES`、未加阈值）。

### 9.5 给上游的可派卡判据（`frame/` 写面）

- 改 `frame/tick.rs:739`：载房窗内也进 `account_iteration_work`（或给窗内单独的丢弃预算），使 `dropped` 由 `work_cycles` 决定；
- 验收锚：三锚窗长 f672=3 / f1309=2 / f1572=3 帧；12 个 checkpoint 的 `lag` 差 ≤±1；本文件 §9.1 的 XOR 由 72 降到 0；
- 回归同本卡五件。

### 9.6 回归（本树实测）

`gates/cli.py check` PASS=24/0/3/1；`cargo test --workspace` 1250 passed / 0 failed；`run-all --skip-emu` 171/172（唯一 c674=84）；`case-password-justin` 14/14。
