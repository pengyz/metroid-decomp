# 密码提交后的暗窗帧会计：入口迭代应冻 4 帧、ClearAll 应冻 7 帧

结论：`case-password-justin` 的 `f1191 dark-window`（2063 px）不是"字符/光标没画"，
也不是调色板或名表问题，而是**密码提交径的帧会计整段早了 1–2 帧**：

- emu：`FrameCount($2D)` = `153` 跨 f1188–f1191（**4 帧**）→ `154` 跨 f1192–f1198（**7 帧**）
  → `155` f1199 → `156` f1200；`$00FE`（PPUMASK 影）`1E`→`06` 落点在 f1191→f1192 之间。
- rust（基线）：`153` 只跨 1 帧、`154` 1 帧、`156` 跨 f1191–f1198（**8 帧**），
  `$00FE` 熄灯早 1 帧 ⇒ f1191 这一采集点上 emu 仍亮、rust 已黑 ⇒ 逐像素差 2063。

即：**入口那次迭代（`InitializeGame`）少冻 1 帧、`ClearAll` 多冻 1 帧，两处叠加成"整段早 1–2 帧"。**

## 1. 症状与起点

| 采集点 | 改前 | 说明 |
|---|---|---|
| f1150 pw-done / f1190 submit / f1197 last-dark | 0 px | 暗窗**两端**都对 |
| f1191 dark-window | **2063 px** | 唯一失配；bbox x32–230 / y32–190 |

逐像素（自写 zlib PNG 解码 + filter 还原）：

- emu 侧该 bbox 三色：`(92,228,48)` 1307 px、`(255,254,255)` 400 px、`(100,176,255)` 356 px；
- rust 侧同样的 2063 个坐标全 `(0,0,0)`。

⇒ 不是"某层多画少画"，而是**整屏熄灯相位差**。

## 2. 三层取证（都可复算）

### 2.1 PPU dump 逐字节

布局 = CHR 8 KB + 4×1 KB 名表 + 32 B palette + 3 B tail（`scroll_x/scroll_y/ppuctrl`）。
同帧 `ppu.hex` 对比：f1191 差 **1086 B**，其中 **CHR 1085 B**、名表 **0**、palette **0**、
tail 最后一字节（ppuctrl）emu `00` vs rust `90`。
参照 f1150 / f1190 各只差 1 B（同一 tail 字节）⇒ f1191 的 CHR 差是新写入的前兆，不是内容错。

### 2.2 RAM dump

`$00FE`（PPUMASK 影子，`0x18`=BG+OBJ 渲染位）：

| 帧 | 1188 | 1189 | 1190 | 1191 | 1192 | 1193–1198 | 1199 |
|---|---|---|---|---|---|---|---|
| emu | 1E | 1E | 1E | 1E | **06** | 06 | 1E |
| rust（基线） | 1E | 1E | **06** | 06 | 06 | 06 | 1E |

### 2.3 FrameCount 逐帧（本文件的核心读数）

rust 侧自建 per-frame 采集（把调度表 capture 行改成 `capture <帧> <tag>` 单帧语法，逐帧带 `.ram.hex`）；
emu 侧用既有 `analysis/traces/case-password-justin.frames.jsonl` 的 `w.FC`。

| 帧 | 1188 | 1189 | 1190 | 1191 | 1192 … 1197 | 1198 | 1199 | 1200 |
|---|---|---|---|---|---|---|---|---|
| emu | 153 | 153 | 153 | 153 | 154 | 154 | 155 | 156 |
| rust（基线） | 153 | 154 | 155 | 156 | 156 | 156 | 157 | 158 |

首分歧 **f1189**；到 f1200 累计 **+2 常差**。

**emu 侧真值自校验**：另跑一个只改 `name` 的探针场景（capture 1188..1200），
其 f1190 / f1191 / f1197 三张图与金标 `shots/case-password-justin_f0119{0,1,7}_*.png`
**逐字节 diff = 0** ⇒ 探针相位 = 金标相位，以上读数可直接作为判据。

## 3. 机制（A 级 asm）

- `MainLoop`（`refs/m1disasm/SRC/prg7_engine.asm:196-225`）：`GoMainRoutine` → **`inc FrameCount`（:207）**
  → `sta NMIStatus` → 自旋等 NMI。⇒ **FC 是"主循环迭代"计数，冻结=主循环阻塞在某个 `WaitNMIPass` 里**。
- `ScreenOff ; 07:C439`：`PPUMASK_ZP &= ~(BG_ON|OBJ_ON)` → `WriteAndWait ; 07:C43D` → `WaitNMIPass_ ; 07:C43F`（等 1 个 NMI）。
  `ScreenNmiOff ; 07:C45D` 同形并清 PPUCTRL NMI 位。
- 密码提交径：`EnterPassword ; 00:9147` → `CheckPassword ; 00:8C5E`
  （`ConsolidatePassword` / `ValidatePassword` / `UnscramblePassword` 双层滚字节忙循环）→ valid ⇒ `InitializeGame ; 00:92D4`
  （`ClearRAM_33_DF` / `ClearSamusStats` / `LoadPasswordData` / **`ScreenNmiOff`** / `LoadSamusGFX` / `NMIOn` / 置 `BankSwitchPending`）
  → 下一迭代 `MoreInit` 走 `ClearAll ; 00:909F`（**`ScreenOff`** → 清名表 → 清精灵 → **`ScreenOn`**）。
- ⇒ 一次"提交→载房"共 3 处等待：`ScreenNmiOff`(1) + `ClearAll` 的 `ScreenOff`(1) + `ScreenOn`(1)，
  在 emu 上表现为"入口迭代 4 帧 + ClearAll 迭代 7 帧"。

## 4. 已实测的两条修法与其否决理由

| 编号 | 改法 | 实测结果 | 判定 |
|---|---|---|---|
| V1 | `title/bridge.rs::initialize_game` 的 `screen_nmi_off` 后补 `iter_wait()` | 暗窗变 1191–1198（起点不变、尾巴推后 1），f1198 由亮变黑 | **否决**：只把尾巴推后，相位没对 |
| V5 | 消费 `check_password_pending` 的那拍声明 `iter_bogged(3)+iter_span_fc` | 熄灯 1190、**暗窗一直不开**、FC 停 155 | **否决** |
| V6/V7 | 改 `exec_shell_call(InitializeGame)` 臂声明 3 / 5 帧 | 两次逐帧输出**完全相同** | **否决**：该臂在密码径根本未执行 |
| V2 | 把"链拍"与"InitializeGame 拍"拆成两 tick（新增 `state.rs` 字段） | **f1188–f1198 与 emu 逐像素全 0**；但 FC 恒 +1，f1430 出现 **169 px** | 有待，见 §5 |
| A | START 消耗后 `title_spin=2` + `world/room_load.rs:72` `iter_bogged(2)→1` | `f1191` **0 px ✓**；`f1430` **169 px ✗**；`case-elev-norfair-transition` **10/10 → 2/10** | **否决**：打穿电梯转入径 |

### 4.1 为什么 f1430 一定会跟着红

`crates/game-core/src/title/bridge.rs:60-62` 已有定谳（2026-11-12）：

> 变体 B：不置 `title_no_return`——emu FC 冻结跨 4 dump 帧，但 **piece④ 前未建模也 match（f1430=0）**；
> 置 true 反引入 **f1430 169px 回归（NT B 未写）**。

本文件把这条从"注释"升级为"实测互通"：**基线 f1430 的绿依赖"FC 早 1 帧"这个错误巧合**。
任何把入口迭代按 emu 冻住的写法都会消掉这个巧合，从而暴露 piece④ 的 **NT B 写入缺口**
（f1430 bbox x121–134 / y94–125，rust 多画 `(92,0,126)` 76 px + `(11,72,0)` 55 px + `(86,29,0)` 38 px，emu 该区全黑）。
⇒ 想同时绿，必须先补 piece④ NT B 模型；仅调帧门不可能。

> 【2026-11-24 更正，#36】本节「NT B 未写」结论**作废**：169px 已定谳为「错 FC 相位下的出场闪烁采样差」
> （见 §7：f1430 的 PPU dump 四页名表/palette/CHR 全 0 差、OAM 8 条逐项同、capture 1426–1432 逐帧逐像素差 0）。
> HEAD 上 f1430=0，「piece④ NT B 写入缺口」不成立，#36 以否证结案。本节保留作历史主张，不得再作为后继依据。

### 4.2 为什么 `iter_bogged(2)` 不能单边改

`crates/game-core/src/world/room_load.rs:72` 的 `iter_bogged(2)` 是**共享门**，
同时服务「密码提交 → 载房」与「电梯/Norfair 转入」两条径。改成 1 让密码径对 1 帧，
就让电梯径少 1 帧：`case-elev-norfair-transition` 由 **10/10 全 0**（02:04 矩阵）掉到 **2/10**。

## 5. 结论与后继判据

1. **f1191 的病因已定谳**：入口迭代帧会计少 1 + `ClearAll` 窗多 1，两侧叠加。
2. **本卡范围内无法同时满足「f1191=0 且 f1430=0 且无回归」**：三者互锁，缺的是 piece④ NT B 模型。

   > 【2026-11-24 更正，#36】本条的「缺 piece④ NT B 模型」**作废**：f1430 在 HEAD 上本来就是 0，
   > 不构成第三个互锁条件。实际只剩 f1191 一项（见 §5.1 与 §7.3）。
3. 后继改动（任何一张新卡）必须同时满足：
   - `case-password-justin` 14/14（f1191 与 f1430 同时 0）；
   - `case-elev-norfair-transition` 保持 **10/10**（本卡实测过的回归面）；
   - `run-all` 不新增失配（基线 170/2）；
   - 验收前按 `docs/findings/bank-switch-frame-accounting.md §7` 的读法纪律核 FC——**FC 是下游门控变量，
     先查 FC 常差再查内容层**。
4. 建议拆分（不在本卡）：
   - ~~**piece④ NT B 写入建模**（f1430 的正解，属新模型不是调参）~~ ——【2026-11-24 更正，#36】**作废**：
     该缺口经实测不成立（§7），无后继卡需要做；
   - **`room_load` 门按径拆分**（把 `iter_bogged(2)` 的"载房窗"与"转入窗"分成两套门，
     避免共享门单边调打穿另一条径）。

## 6. 未覆盖

- 只取了 f1188–f1200 一段；提交前的 `UnscramblePassword` 忙循环在更早帧的相位未单独测量。
- emu 侧 FC 取自既有 `frames.jsonl`（watch 字段），未逐帧重采 OAM/名表；内容面对齐用的是自采探针截图。
- piece④ NT B 的 asm 写入点未考证（属后继卡）。
  > 【2026-11-24 更正，#36】本条**作废**：NT B 写入本身两侧逐字节一致（§7.1），无需再考证。

## 7. 【2026-11-24 补充】f1430 的 169 px 是变体实验读数，不是 HEAD 残差

卡 #36 的前提是「f1430 有 169 px，需要补 piece④ NT B 写入时序」。**该前提不成立**，原因是我在
本文件 §4 里记录的变体读数被当成了 HEAD 基线：

- §4 里 V2/V5/engine 三处改造的实测（`f1430 = 169 px`）跑在 `/tmp` 副本上，**那批改动已全部回退，从未进入 HEAD**。
- 干净树上（HEAD `2bf74902`，`git status --porcelain -- crates/` 输出为空）实跑：

```
python3 tools/l4_diff_run.py run cases/case-password-justin.txt
  1191 dark-window   ❌失配  2063 / 61440
  1430 spawn-t4s     ✅一致     0 / 61440
  汇总：一致 13 · 失配 1（共 14）
```

⇒ **HEAD 上 f1430 = 0 px**，唯一残差是 §1–§5 已定谳并验收的 f1191。

### 7.1 那 169 px 是什么：出场闪烁，且两侧同相位

逐帧相邻探针（capture 1426–1432，emu 走 `tools/scenario_run.py`，rust 走 `rust_screenshot --schedule`），
逐像素比对结果：

| 帧 | emu 非黑像素 | rust 非黑像素 | 逐像素差 |
|---|---|---|---|
| 1426 | 11431 | 11431 | 0 |
| 1427 | 11600 | 11600 | 0 |
| 1428 | 11431 | 11431 | 0 |
| 1429 | 11600 | 11600 | 0 |
| 1430 | 11431 | 11431 | 0 |
| 1431 | 11600 | 11600 | 0 |
| 1432 | 11431 | 11431 | 0 |

这 169 px（bbox x121–134 / y94–125）是萨姆斯出场动画的闪烁位：两侧都在 11431/11600 之间**逐帧交替**，
相位一致。169 px 全部落在精灵调色板 0（`pal[16:20] = 0F 04 09 07`）；8 条 OAM 精灵（y=93/101/109/117，
x=120/128，tile 09/19/1A/29/2A/39，attr 20/60）在两侧逐字节相同，attr bit5 置位即「在背景之后」，
可见性取决于其下 BG 是否透明——这也是为什么在错误的 FC 相位下它会露出来。

### 7.2 记账纪律（本次教训）

**变体读数与基线读数必须分栏记账**：在副本里试参数得到的数字，只能写「变体 X 实测」，不能写成
「基线残差」。本卡开卡就是因为我混写了这两类数字。后继同类任务：贴读数时必须同时给
`git status --porcelain -- crates/` 的输出与 HEAD sha。

### 7.3 本卡结论

f1430 无需修复；§5 的后继判据中「f1430=0」这一条在 HEAD 已自然成立。真正剩下的只有
f1191（2063 px，已登记于 §4.1/§5），以及 §5.4 列的两件后继工作。

---

## 8. #37 取证补记：入场忙等 3 帧（A 级）与「span ≠ 门数」硬约束（2026-11-25）

### 8.1 入场那次迭代 = 1 拍消费 + 3 拍纯 CPU 忙等（A 级，逐帧 PWByte）

数据源：`analysis/traces/case-password-justin.frames.jsonl` 的 `w` 字段（emu 每帧 watch 快照）。
复算命令：
```bash
python3 - <<'PY'
import json
for line in open('analysis/traces/case-password-justin.frames.jsonl'):
    r=json.loads(line); w=r.get('w') or {}
    f=r.get('f');
    if f and 1186<=f<=1193: print(f, w.get('FC'), w.get('PW'), w.get('JIB'), w.get('InArea'))
PY
```
| f | `FC` | `PWByte00-03` | `PWByte04-05` | `JIB` / `InArea` |
|---|---|---|---|---|
| 1188 | 153 | `00 00 00 00` | `00 00` | 0 / 0 |
| 1189 | 153 | `FF FF FF FF` | `00 00` | 0 / 0 |
| 1190 | 153 | `FF FF FF FF` | `FF FF` | 0 / 0 |
| 1191 | 153 | `A6 F3 8E A4` | `B9 65` | **1 / 17** |
| 1192 | 154 | 同上 | 同上 | 1 / 17 |

判读：`ConsolidatePassword`/`UnscramblePassword` 的双层滚字节循环**逐帧推进 3 帧**
（f1189 / f1190 / f1191 各推一段），`InitializeGame` 在同一迭代内完成（f1191 已
`JIB=1`、`InArea=$11`）。⇒ 入口那次迭代 = **1 拍 START 消费 + 3 拍忙等 = 4 帧**
（`FC=153` 跨 f1188–f1191）。这**推翻**了 `title/bridge.rs` 旧注释「Unscramble 跨 2 tick」
的说法；忙等是纯 CPU（NMI 内块被跳），语义上是 bogged 而非等待门。

### 8.2 硬约束：`iter_span_left` 不能单独偏移（引擎结构，实测）

`crates/game-core/src/frame/engine.rs` 里两个计数器**不同源**：

- `iter_gates`：逐 tick 弹一项的队列（`end_of_tick_gate` 的 `else` 支
  `let g = self.mem.iter_gates.remove(0);`，约 :195）；
- `iter_span_left`：自减计数，只用于「是否在 span 内」的判定（`in_span()`）；
- 迭代真正的结束判据是 **`iter_gates.is_empty()`**（:165 那个分支=排空点）。

实测后果：把 span 改成「门数-2」后 span 先归零而队列仍剩 2 项 ⇒ 引擎判「不在 span」⇒
又跑一次 `main_loop_iter`，新迭代撞上仍为 `true` 的 `deferred_screen_on`，把声明拍的
`FC-1` 再执行一遍 ⇒ 采集表尾巴出现 `FC=153/152` 的掉值。复算：`[T37]` 逐 tick 打印
（临时，已撤）显示 `n=1197 span=0 gates=1`、`n=1198 fc=0x99`。

**结论**：任何「只调 span 长度」的修法都会与门队列脱同步；要改变关屏窗长度必须让
「窗长」与 `iter_gates` 同源派生，或引入独立的显式窗长量。

### 8.3 剩余 1 帧的精确落点（本卡未达成部分）

- 入场忙等做出来后，熄灯点可精确落在 `n=1191`（capture f1192 暗），与 emu 一致；
- 仍差的 1 帧在 **ClearAll 迭代的最后一门**：`iter_gates` = 7 项 ⇒ 暗窗 8 帧（emu 7 帧）。
  要让最后一帧亮起，等价于「ScreenOn 自己的 `iter_wait`（`crates/game-core/src/world/room_load.rs:105`）
  不再额外占 1 帧」；
- 该门是**共享门**：`#31` 已实测单边改 `iter_bogged(2)` 会把 `case-elev-norfair-transition`
  从 10/10 打穿到 2/10（见 §4.2），因此本卡不动它。

后继判据（给下一张卡）：在**不动** `room_load` 共享门的前提下，若能让关屏窗长与
`iter_gates` 同源派生（例如把窗长声明为该迭代门数的函数并显式记录），则应满足：
`run cases/case-password-justin.txt` 14/14 全 0、`run cases/case-elev-norfair-transition.txt`
10/10、`run-all` 不新增。
