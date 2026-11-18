# 房间批量扫描（room sweep）：Brinstar 47 房首轮

> 【状态】新基建 + 首轮真实数据（2026-11-19）。判据**原样复用** `tools/l4_diff_run.py`
> （L4 像素零容差 + 既有视觉白名单），本工具不改判据、不加白名单、不放宽阈值。
> 工具 `tools/movie_ref/room_sweep.py`；数据 `data/status/room_sweep_0.json`。

## 0. 结论摘要

| 项 | 值 |
|---|---|
| 分母（计划扫的房数） | `47` |
| 实际跑（两腿都起过） | `47` |
| `ok`（两腿跑完 且 每采集点 diff==0 且 房号被接受） | `15` |
| 有失配（`mismatch`） | `31` |
| 缺口中阻塞（`blocked`） | `1` |
| 错误 / 超时 | `0` / `0` |
| 跳过（带理由） | `0` |
| 单房均耗时 | `28.7 s（n=47）` |

- `entry_preset v1` = **4 个注入量**（帧 900，两条腿语义相同）：
  `$005A RoomNumber`、`$0049 ScrollDir=3`、`$00FD ScrollX=2`、`$0056 DoorEntryStatus=1`。
  逐字节理由与反证实验见 §3。
- 能力边界（必须先读）：本方法证明的是**房间数据 + 载入链 + 绘制 + 该注入坐标下的物理**，
  **不证明可达性/连通性**（见 §4）。
- 最大失配前 5 房见 §6；跳过清单见 §7。
- 采集点分层（94 点）：`nametable` **61** / `clean` **15** / `scroll_drift` **10** /
  `sprite` **6** / `unknown` **2**（room 43 阻塞）。
- **首轮最值得追的三条线索**（都在名表/滚动面，且已定位到字节级）：
  1. **房间边界列 tile**：emu 写 `$4E`、rust 写 `$FF`（空白），只出现在 `col 2`/`col 29`
     （19 + 9 次）⇒ 边界列填充/流式边缘列路径（§6.2）。
  2. **属性表数据源疑似串位**：16 个采集点**只有**属性表差且 tile 全同，emu 侧是整字节同色
     `$55/$5A/$FF`，rust 侧却是逐象限值 `$4B-$57`（tile 索引形态）⇒ 像是把 tile 数据写进了属性区（§6.2）。
  3. **滚动相位差一个 tick**：10 个采集点整屏平移 `dx=2` 后残差只剩 3-5%（§6.3）。

> 【2026-11-19 更正 · 任务书前提】任务书把代表房分成「普通房 / 竖滚房 / 宽房（横向滚）」。
> 该分类**不成立**：房间数据本身恒为**一屏**（§2 证据），「竖滚/横向宽」是**世界图邻域**属性
> （`GetRoomNum ; 07:E720` 按地图格取房号），不是房间数据的属性。代表房因此改为按内容选：
> room 20（稀疏）、room 36（物件最多，20 个）、room 22（含 type-7 管虫敌兵）；
> 竖向滚动路径另做一次**注入路径**实验（DoorEntryStatus=3），结果见 §3.3。

> 【2026-11-19 更正 · 任务书前提】任务书称「Kraid/Tourian/Ridley 房流尚未迁包」。
> 实测**五区房流都在资产包内**（`cargo run -q -p pack-tool -- list --pack assets/metroid.pack`
> ⇒ `area.kr.room_streams` 1985 B / `area.to.room_streams` 871 B / `area.ri.room_streams` 1928 B，
> 解析端 `crates/game-data/src/assets.rs:183-192,246-254` 也确有 KR/TO/RI 表）。
> 2/3/4 区今天仍然**不能扫**的真正原因是 Rust 侧**区派发未移植**：
> `crates/game-core/src/world/room_draw.rs` 只在 `area ∉ {0,1}` 时 `unported!`。
> 所以本工具把它们记成 `area_dispatch_unported`，而不是「房流未迁包」。

## 1. 方法与判据

### 1.1 一房一用例

对每个 `(area, room)` 生成临时用例 `cases/_tmp_sweep_a<area>_r<room:02d>.txt`（跑完即删，
进程被信号杀掉时也删；启动时清上一轮残留），内容形如：

```
name _tmp_sweep_a0_r20
frames 1020
watch 001E MainRoutine
watch 005A RoomNumber
watch 0049 ScrollDir
watch 0056 DoorEntryStatus
snapshot none
press 310 start
press 350 start
capture 900 pre
fixture 900 005A 14 invariant
fixture 900 0049 03 invariant
fixture 900 00FD 02 invariant
fixture 900 0056 01 evolving
capture 960 f60
capture 1020 f120
endFrame 1020
```

- **两次 `press ... start` 是必须的**：`press 310 start` 只过标题，`press 350 start` 才是选单的
  START GAME。只按一次时整局停在标题阶梯（`$001E MainRoutine=0`，所有观测量恒 0），
  此时 L4 会给出一个**毫无意义但漂亮的 0 px**（标题 vs 标题）。这是一次实测踩到的坑。
- `frames 1020` = 注入帧 900 + 窗口 120 帧。900 这个数不是猜的：`SamusInit ; 07:C8D1` 置
  `Timer3=$2C`，`UpdateTimer ; 07:C266` 每 10 帧减 1 ⇒ 开场 440 帧演出吞输入
  （与 `docs/findings/mechanism-scene-door-transition.md` §1 同一事实），加上标题/选单输入 ~40 帧，
  900 帧留足余量；实测 f460 前后已进入 `mr=3`。

### 1.2 为什么采集 60/120 两点、为什么窗口是 120 帧

门滚是 **2 px/帧**（`crates/game-core/src/frame/tick.rs:624-625` 每帧调两次
`scroll_door`，asm `ScrollDoor ; 07:E1F1`）。实测注入后 `$00FD ScrollX` 从 2 起每帧 +2：
f60 ⇒ 118、f120 ⇒ 238。

- 120 帧 × 2 px = 240 px < 256 ⇒ **窗口内 ScrollX 不回绕**，因此不会走到
  `ScrollRight ; 07:E6D2` 的 `lda ScrollX / bne @noNewRoom` 反向分支，也就**不会**触发
  `GetRoomNum` 把 `$005A` 改写成世界图里的邻房（这一点由 `V_no_scrollx` 反证，见 §3.2）。
  窗口一旦 ≥128 帧，注入的房号就有被覆盖的风险。
- 两个采集点各有分工：f60 时约一半屏幕仍是旧房内容（滚动进行中），f120 时注入房的内容
  已铺满可见区 ⇒ 同一房覆盖「滚动中」与「滚动停」两个相位。

### 1.3 `ok` 的严格定义与分母

```
ok = 两腿都跑完（无 error / blocked / timeout）
     且 每个采集点 diff_pixels == 0
     且 注入房号被引擎接受（$005A 走完 F0→…→FF 收口链，$001E 保持 3=GameEngine）
```

**分母恒为「计划扫的房数」**，不是「跑成功的房数」：

```
planned = Σ(每区计划房数)     # 由 --rooms 展开、受 ROOM_COUNTS 约束
ran     = 真正起过两腿的房数
skipped = 未跑（带 skip_reason）
```

`skip_reason` 枚举（全部落在 JSON 的 `skips[]` 里，不静默）：

| 理由 | 含义 |
|---|---|
| `area_dispatch_unported` | Rust 侧区派发未移植（area ∉ {0,1}），房流在包内也不跑 |
| `room_streams_not_in_pack` | 资产包内无该区房流 |
| `area_unknown` | 区号不在 0..4 |
| `room_index_out_of_range` | 房号越界（如 Brinstar 只有 0..46） |
| `room_number_not_ff` | 注入后 `$005A` 未走完 F0→…→FF ⇒ 房号未被接受 ⇒ 像素结果**不计入结论** |
| `entry_preset_unverified_area` | 该区的 `entry_preset` 未用反证实验定稿（目前只有 area 0 定稿），
整区按房逐个跳过而不是产出可疑数据（§8.1） |

每个被跳过的房**单独一行**（`room` 具体到房号）⇒ `planned = ran + skipped` 机械可核。

`ok` 与「房号被接受」必须同时成立，否则「0 px」可以来自「两边都没载入这个房」——
这就是 `room_number_not_ff` 单列的理由。

### 1.4 幂等与可中断

- 同一 `--out` 重跑**覆盖**（临时文件 + `os.replace` 原子写）。
- **每跑完一房就落盘**（`"partial": true`），中断后 `data/status/*.json` 仍有已完成部分；
  `--resume` 跳过其中已 `ok` 的房。
- 单房任何异常（用例错误、腿超时、缺报告）都记一条记录后**继续**，不中断整批。
- 信号处理：`SIGINT`/`SIGTERM` ⇒ 删正在用的临时用例 + `SystemExit(130)`。

### 1.5 除「像素差」之外，还把差异**分解到 PPU 层**

两条腿对每个采集点都 dump PPU 真值（emu `analysis/traces/<case>_f*_<tag>.ppu.hex`、
rust `shots_rust/<case>_f*_<tag>.ppu.hex`；格式 `$0000-$2FFF`（CHR 8KB + 名表 4KB）+ 调色板
`$3F00-$3F1F` + 尾 3B）。工具按区间比对：

| 层 | 区间 | 含义 |
|---|---|---|
| `chr` | `$0000-$1FFF` | 图案表（tile 像素来源） |
| `nt0..nt3` | `$2000-$2FFF` | 名表（含属性表 `$23C0`/`$27C0`/`$2BC0`/`$2FC0`）⇒ **房间数据面** |
| `palette` | `$3F00-$3F1F` | 调色板 |
| `sprite` | 推理层 | 以上全同、像素仍有差 ⇒ 只能来自 OAM（OAM 未 dump，故为**排除法**） |

> 尾 3B（scrollX/scrollY/ppuCtrl）**不参与比对**：emu 侧 `emu.getState()` 字段名不匹配
> （`tools/scenario_capture.lua` `dumpPpu` 对 `ppu.scrollX`/`ppu.scroll_x` 双形容错），实测恒为 `0,0,0`，
> 拿它比对会伪造「滚动分歧」。rust 侧真实值仅作元数据记在 `ppu_diff[*].rust_scroll`。

## 2. 房间 = 一屏（改写了「竖滚房/宽房」的前提）

**格式上限（A 级，asm 直读）**：房间数据是「物件三元组」流，位置字节形如 `%yyyyxxxx`：

- `DrawRoomObject ; 07:EA60`（`refs/m1disasm/SRC/prg7_engine.asm:8724-8790`）：y 循环每行加 `#$40`
  （= 2 tile 行），x 用 `and #$0F / asl / adc RoomRAMWorkPtr`（x 单位 = 2 tile 列）。
- 写入目标是 `RoomRAM`（`$6000` 或 `$6400`，由 `SelectRoomRAM` 按 `(PPUCTRL^ScrollDir)&1` 选），
  结构 = tile 区 `0x3C0`（32×30 tile）+ 属性表 `0x40` ⇒ 合计 1 KB = **一个名表 = 一屏**。
- 位置字节 4 bit/轴 ⇒ 坐标上限 16×16 单位 = 32×32 tile = 256×256 px，正好一屏。

**实测（46/47 房完整解码）**：所有物件 x 单位 ≤ 15、y 单位 ≤ 14 ⇒ 无一房超出一屏。
（room 43 的 `$FD` 敌兵段结构未解开，不影响本结论；扫描不依赖解码。）

**推论**：所谓「竖向滚动房」「横向宽房」是**世界图邻域**的现象——地图格 = 一屏，
格里的字节就是房号（`GetRoomNum ; 07:E720`：`MapY*32+MapX+$7000` 取字节写 `$5A`），
同一房号在多个格子里重复时，内容按格重画（`$6000`/`$6400` 交替）。
所以「跨屏可滚」必须在**注入路径**层面验证（§3.3），不能在**房间数据**层面分类。

## 3. `entry_preset` 定稿（每个字节为什么）

`entry_preset v1`（帧 900，一次性；`invariant` = 只写一次，`evolving` = 每帧重写）：

| # | 地址 | 名字（`ram_map.rs`） | 值 | 持久化 | 为什么必须 |
|---|---|---|---|---|---|
| 1 | `$005A` | `ROOM_NUMBER` | 目标房号 | invariant | `SetupRoom ; 07:EA2B` 的唯一输入房号；`CheckUpdateNameTable ; 07:E54A` 第一句就是 `jsr SetupRoom` |
| 2 | `$0056` | `DOOR_ENTRY_STATUS` | `01` | evolving | 唯一触发器：`ScrollDoor ; 07:E1F1` 读它，0 直接 `rts`，1 才 `jsr ScrollRight`。它会自己归零，所以每帧重写 |
| 3 | `$00FD` | `SCROLL_X` | `02` | invariant | 必须 ≠0。`ScrollRight` 在 `07:E6EE` 判 `ScrollX==0` 时会 `inc MapPosX / jsr GetRoomNum`，用**世界图邻房**覆盖 `$005A` ⇒ 注入的房号被丢掉（反证见 §3.2） |
| 4 | `$0049` | `SCROLL_DIR` | `03`(RIGHT) | invariant | `ScrollRight` 首句只接受 `2`(LEFT)/`3`(RIGHT)，其余 `sec/rts` 不滚；`CheckUpdateNameTable` 又用 `ScrollDir & 2` 选横向分支。注：**非严格必需**（dir=2 会被 `07:E6E1` 改成 3），保留是为了路径确定性 |

**被实验剔除的注入项**：`$030C SamusHi` / `$030D SamusY` / `$030E SamusX`。
任务书建议把 Samus 放进「房间内中下部空位」，实测**不需要**（§3.2 `H_no_samus`：像素与载入链
与全量版逐帧一致），剔除后注入面更小、结论面更窄，也避免把「Samus 站在墙里」的物理噪声
混进房间数据结论。注：本方法**不**依赖 Samus 位置，因为门滚本身会把 Samus 锁在门滚动姿态。

### 3.1 载入链的逐帧证据（room 20）

`$005A` 只有三种取值语义：`$FF`=无请求、`< $F0`=要载入的房号、`$F0..$F4`=收口链阶段计数
（`end_of_room ; 07:EAF4` 置 `$F0`，随后 `attrib_table_write` 每次 `inc RoomNumber`，行 4 时置 `$FF`）。
`CheckUpdateNameTable` 每帧被调 2 次（门滚双速）⇒ 收口链约 5 帧走完。

| 帧 | `$005A` | `$0049` | `$00FD` | 含义 |
|---|---|---|---|---|
| 899 | `FF` | 2 | 0 | 正常游戏中（起始房，无载入请求） |
| 900 | `14` | 3 | 2 | 注入生效（房 20） |
| 901 | — | 3 | 3 | 门滚开始，2 px/帧 |
| 902 | `F1` | 3 | — | 收口链（`attrib_table_write` 已走完第一行） |
| 904 | `F3` | 3 | — | 收口链 |
| 905 | `FF` | 3 | — | 载入完成，回到稳定态 |
| 960 | `FF` | 3 | `76`(118) | 采集点 f60 |
| 1020 | `FF` | 3 | `EE`(238) | 采集点 f120（未回绕） |

`$001E MainRoutine` 全程 3（GameEngine）⇒ 不是别的模式在跑。
机器可读：`data/status/room_sweep_0.json` 每房的 `room_load.room_frames` / `ff_markers` / `back_to_ff_frame`。

### 3.2 反证实验（决定「哪些字节必须注入」）

同一房（room 20）、同样 1020 帧，逐个去掉注入项：

| 变体 | 去掉 | 像素 | 载入链 | 结论 |
|---|---|---|---|---|
| `H_full` | — | 4/4 全 0 px | 905 回 `$FF`，`ScrollX` 终值 `EE` | 基线 |
| `H_no_samus` | `$030C/$030D/$030E` | 全 0 px | 同上 | Samus 位置**不需要** ⇒ 剔除 |
| `H_no_dir` | `$0049` | 全 0 px | 同上 | 非严格必需（dir 2 会被自动翻成 3） |
| `H_no_scrollx` | `$00FD`（⇒ 0） | 全 0 px | 905 也回 `$FF`，但 `ScrollX` 终值 `EC` | **陷阱**：`ScrollX==0` 走 `GetRoomNum`，`$005A` 被世界图邻房覆盖 ⇒ 测的不是目标房 ⇒ 必须注入 |
| `H_no_door` | `$0056` | 全 0 px | `$005A` 恒 `14`，`ScrollX` 恒 2，永不回 `$FF` | **唯一触发器** ⇒ 必须注入 |

`H_no_scrollx` / `H_no_door` 的「全 0 px」是**假绿**：前者比较的是邻房、后者比较的是
「什么都没发生」。这正是 §1.3 把「房号被接受」写进 `ok` 判据的原因。

### 3.3 竖向入口（`DoorEntryStatus=3`）单独实验：**不采用**

变体 `V_full`（`$0049=01` 下、`$00FC ScrollY=02`、`$0056=03`）：载入链**正常**
（905 回 `$FF`，`ScrollY` 终值 `EE`），但像素差异 f60 **392 px** / f120 **497 px**。
逐像素定位：差异全部落在 x=120..147 的 **Samus 精灵列**（bbox f60 `(121,0)-(147,207)`、
f120 `(120,48)-(143,163)`），房间背景像素**完全一致**。

⇒ 这是**精灵层**分歧（emu 在竖向门滚期间不画站立 Samus，rust 画了），不是房间数据问题。
v1 preset 因此**只做横向入口**；竖向路径作为独立缺口挂在 §8，不混进房间数据结论。

## 4. 注入法能证明什么 / 不能证明什么

**能证明（在注入坐标与注入窗口内）**：

- **房间数据的解码与绘制**：房流 → `struct_ptrs`/`structures`/`metatile_defs` → `RoomRAM` →
  名表 tile 与属性表，两条腿逐字节一致（§1.5 的 `nt0..nt3` 层比对）。
- **载入链**：`$005A` 请求 → `SetupRoom` → `end_of_room` → `attrib_table_write` 收口 → `$FF`。
- **滚动流式写入**：`UpdateNameTable` 按列推进 240 px 时的时序与内容（`ScrollDir == TempScrollDir` 门槛）。
- **该坐标下的物理与精灵**：Samus 在门滚动姿态下的位置/精灵，以及该房间敌兵在该窗口内的表现
  （精灵层差异会以 `sprite` 层单独记账，不与房间数据混淆）。

**不能证明（显式声明）**：

- **不证明可达性/连通性**。注入把 Samus 直接放进目标房间，绕过了门拓扑与行走路径；
  早期探针曾因直接注入 `RoomNumber` 把 Samus 埋进墙里，本方法**接受**这一点
  ——只要两腿注入相同 fixture，两边都埋墙，逐像素比较依然有效。
- 不证明该房在**世界图中的位置**、门的连接关系、以及「玩家从哪个入口进来」的滚动方向。
- 不证明**敌人合法性**：敌兵由房间流的 `$FD` 段生成，注入法不校验其与地图/道具状态的关系。
- 不证明**竖向滚动**路径正确（§3.3 已实测有分歧）。
- 不证明**采集窗口之外**的行为（120 帧 / 240 px 之外的流式写入、回绕、换房）。

## 5. 首轮结果（Brinstar 47 房，`--frames 120`，采集点 60/120）

| room | status | f900 注入前 | f60 | f120 | 层(pre/60/120) | 备注 |
|---|---|---|---|---|---|---|
| 30 | mismatch | 0 | 14467 | 14894 | clean/nametable/nametable | f60: tile 12B 列[2] emu→rust 4E→FF，attr 48B emu→rust 55→56；f120: tile 24B 列[2, 29] emu→rust 4E→FF，attr 48B emu→rust 55→56 |
| 29 | mismatch | 0 | 13147 | 14407 | clean/scroll_drift/scroll_drift | 层：f60=scroll_drift,f120=scroll_drift |
| 25 | mismatch | 0 | 11958 | 13370 | clean/nametable/nametable | f60: attr 64B emu→rust 55→4D；f120: attr 64B emu→rust 55→4D |
| 24 | mismatch | 0 | 12905 | 13071 | clean/nametable/nametable | f60: tile 12B 列[2] emu→rust 4E→FF，attr 34B emu→rust 55→02；f120: tile 24B 列[2, 29] emu→rust 4E→FF，attr 34B emu→rust 55→02 |
| 37 | mismatch | 0 | 13057 | 12958 | clean/nametable/nametable | f60: tile 24B 列[14] emu→rust FF→55；f120: tile 60B 列[29] emu→rust FF→56 |
| 38 | mismatch | 0 | 12239 | 10027 | clean/scroll_drift/scroll_drift | 层：f60=scroll_drift,f120=scroll_drift |
| 02 | mismatch | 0 | 10381 | 6036 | clean/scroll_drift/scroll_drift | 层：f60=scroll_drift,f120=scroll_drift |
| 41 | mismatch | 0 | 10069 | 7954 | clean/scroll_drift/scroll_drift | 层：f60=scroll_drift,f120=scroll_drift |
| 28 | mismatch | 0 | 9314 | 9877 | clean/nametable/nametable | f60: tile 12B 列[2] emu→rust 4E→FF，attr 20B emu→rust 00→A5；f120: tile 12B 列[2] emu→rust 4E→FF，attr 20B emu→rust 00→A5 |
| 14 | mismatch | 0 | 9132 | 6696 | clean/scroll_drift/scroll_drift | 层：f60=scroll_drift,f120=scroll_drift |
| 16 | mismatch | 0 | 3507 | 5603 | clean/nametable/nametable | f60: tile 12B 列[2] emu→rust 4E→FF，attr 32B emu→rust AA→12；f120: tile 24B 列[2, 29] emu→rust 4E→FF，attr 32B emu→rust AA→12 |
| 13 | mismatch | 0 | 0 | 3620 | clean/nametable/nametable | f120: tile 12B 列[29] emu→rust 4E→FF，attr 32B emu→rust AA→12 |
| 12 | mismatch | 0 | 2885 | 3010 | clean/nametable/nametable | f60: tile 12B 列[2] emu→rust 4E→FF，attr 16B emu→rust AA→12；f120: tile 12B 列[2] emu→rust 4E→FF，attr 16B emu→rust AA→12 |
| 35 | mismatch | 0 | 1015 | 2438 | clean/nametable/nametable | f60: attr 114B emu→rust AA→55；f120: attr 114B emu→rust AA→55 |
| 31 | mismatch | 0 | 1058 | 1336 | clean/nametable/nametable | f60: tile 12B 列[2] emu→rust 4E→FF，attr 54B emu→rust 55→56；f120: tile 12B 列[2] emu→rust 4E→FF，attr 54B emu→rust 55→56 |
| 32 | mismatch | 0 | 635 | 1210 | clean/nametable/nametable | f60: attr 106B emu→rust 55→57；f120: tile 12B 列[29] emu→rust 4E→FF，attr 106B emu→rust 55→57 |
| 15 | mismatch | 0 | 990 | 1146 | clean/nametable/nametable | f60: attr 8B emu→rust AA→80；f120: attr 8B emu→rust AA→80 |
| 42 | mismatch | 0 | 20 | 747 | clean/nametable/nametable | f60: attr 18B emu→rust 55→80；f120: attr 18B emu→rust 55→80 |
| 27 | mismatch | 0 | 289 | 703 | clean/nametable/nametable | f60: attr 28B emu→rust 00→80；f120: attr 28B emu→rust 00→80 |
| 08 | mismatch | 0 | 296 | 604 | clean/nametable/nametable | f60: attr 32B emu→rust FF→00；f120: attr 32B emu→rust FF→00 |
| 21 | mismatch | 0 | 210 | 345 | clean/sprite/sprite | 层：f60=sprite,f120=sprite |
| 05 | mismatch | 0 | 289 | 289 | clean/nametable/nametable | f60: tile 12B 列[2] emu→rust 4E→FF，attr 32B emu→rust FF→04；f120: tile 24B 列[2, 29] emu→rust 4E→FF，attr 32B emu→rust FF→04 |
| 11 | mismatch | 0 | 256 | 256 | clean/nametable/nametable | f60: tile 12B 列[2] emu→rust 4E→FF，attr 20B emu→rust 00→FF；f120: tile 12B 列[2] emu→rust 4E→FF，attr 20B emu→rust 00→FF |
| 22 | mismatch | 0 | 0 | 210 | clean/nametable/nametable | f120: attr 32B emu→rust 55→51 |
| 10 | mismatch | 0 | 0 | 184 | clean/nametable/nametable | f120: tile 12B 列[29] emu→rust 4E→FF，attr 16B emu→rust 00→5C |
| 07 | mismatch | 0 | 41 | 148 | clean/nametable/nametable | f60: attr 16B emu→rust FF→80；f120: attr 16B emu→rust FF→80 |
| 40 | mismatch | 0 | 146 | 135 | clean/sprite/nametable | f120: tile 12B 列[29] emu→rust 4E→FF |
| 39 | mismatch | 0 | 0 | 141 | clean/nametable/nametable | f120: tile 12B 列[2] emu→rust 4E→FF |
| 46 | mismatch | 0 | 141 | 141 | clean/nametable/nametable | f60: tile 12B 列[2] emu→rust 4E→FF，attr 16B emu→rust FF→04；f120: tile 12B 列[2] emu→rust 4E→FF，attr 16B emu→rust FF→04 |
| 33 | mismatch | 0 | 64 | 116 | clean/sprite/sprite | 层：f60=sprite,f120=sprite |
| 36 | mismatch | 0 | 0 | 92 | clean/clean/sprite | 层：f120=sprite |
| 00 | ok | 0 | 0 | 0 | clean/nametable/nametable | 两腿一致 |
| 01 | ok | 0 | 0 | 0 | clean/nametable/nametable | 两腿一致 |
| 03 | ok | 0 | 0 | 0 | clean/nametable/nametable | 两腿一致 |
| 04 | ok | 0 | 0 | 0 | clean/nametable/nametable | 两腿一致 |
| 06 | ok | 0 | 0 | 0 | clean/clean/clean | 两腿一致 |
| 09 | ok | 0 | 0 | 0 | clean/clean/clean | 两腿一致 |
| 17 | ok | 0 | 0 | 0 | clean/clean/clean | 两腿一致 |
| 18 | ok | 0 | 0 | 0 | clean/nametable/nametable | 两腿一致 |
| 19 | ok | 0 | 0 | 0 | clean/clean/nametable | 两腿一致 |
| 20 | ok | 0 | 0 | 0 | clean/clean/clean | 两腿一致 |
| 23 | ok | 0 | 0 | 0 | clean/clean/clean | 两腿一致 |
| 26 | ok | 0 | 0 | 0 | clean/nametable/nametable | 两腿一致 |
| 34 | ok | 0 | 0 | 0 | clean/clean/clean | 两腿一致 |
| 43 | blocked | 0 | None | None | clean/unknown/unknown | 采集点被 unported 缺口阻塞（见 §7） |
| 44 | ok | 0 | 0 | 0 | clean/nametable/nametable | 两腿一致 |
| 45 | ok | 0 | 0 | 0 | clean/clean/nametable | 两腿一致 |

## 6. 失配分层与最大失配

失配房 **31** 个（`ok` 15 / `blocked` 1 / 跳过 0）。按「最大单点像素差」降序，前 5 房：

| # | room | 最大失配 | 采集点明细 | 名表字节差分解（离线读两腿 PPU dump） |
|---|---|---|---|---|
| 1 | 30 | 14894 px | pre=0 · f60=14467 · f120=14894 | f60: tile 12B 集中列 [2] 值 4E→FF；attr 48B 行 [0, 3, 4, 7] 值 55→56,55→57,55→58 / f120: tile 24B 集中列 [2, 29] 值 4E→FF；attr 48B 行 [0, 3, 4, 7] 值 55→56,55→57,55→58 |
| 2 | 29 | 14407 px | pre=0 · f60=13147 · f120=14407 | f60: PPU 三层全同 / f120: PPU 三层全同 |
| 3 | 25 | 13370 px | pre=0 · f60=11958 · f120=13370 | f60: attr 64B 行 [0, 1, 4, 5] 值 55→4D,55→50,55→4B / f120: attr 64B 行 [0, 1, 4, 5] 值 55→4D,55→50,55→4B |
| 4 | 24 | 13071 px | pre=0 · f60=12905 · f120=13071 | f60: tile 12B 集中列 [2] 值 4E→FF；attr 34B 行 [0, 3, 4, 7] 值 55→02,55→03,55→01 / f120: tile 24B 集中列 [2, 29] 值 4E→FF；attr 34B 行 [0, 3, 4, 7] 值 55→02,55→03,55→01 |
| 5 | 37 | 13057 px | pre=0 · f60=13057 · f120=12958 | f60: tile 24B 集中列 [14] 值 FF→55,FF→57 / f120: tile 60B 集中列 [29] 值 FF→56,FF→58,FF→4C |

### 6.1 层分布（每个采集点算一次）

| 层 | 采集点数 | 含义 |
|---|---|---|
| `nametable` | 61 | 名表（tile/属性表）字节不同 ⇒ **房间渲染面** |
| `scroll_drift` | 10 | PPU 全同、像素差可由整屏平移解释 ⇒ **滚动时序面** |
| `sprite` | 6 | PPU 全同、平移也解释不了 ⇒ 只能是 OAM（排除法） |
| `clean` | 15 | 像素与 PPU 全同 |
| `unknown` | 2 | room 43 的两个采集点被 `blocked`（无 PPU 采样） |

合计 94 个采集点（47 房 × 2 点）。

### 6.2 名表差异的两条共性通道（离线统计全部失配采集点）

1. **tile 差异只落在房间的左右边界列**：所有出现 tile 差的采集点，差异列集中在 `col 2`（19 次）/ `col 29`（9 次），room 37 另有一次 `col 14`（该列随帧从 14 → 29 移动，是**流式边缘列**，见 §6.3）。值形态高度集中：emu 写 `$4E`、rust 写 `$FF`（空白）（23 次）。⇒ 指向**房间边界列填充/流式边缘列**路径，而不是整房数据错乱。
2. **属性表差异与 tile 差异互斥**：16 个采集点只有属性表差、tile 全同，且 f60 与 f120 的差异字节**完全相同** ⇒ 发生在**房间绘制阶段**（`init_tables` / `attrib_table_write ; 07:EA2B` 区），不是流式阶段。值形态同样集中：emu 侧是「整字节同色」的 `$55/$5A/$FF`，rust 侧却是**逐象限不同**的 `$4B-$57` 或低值 `$00-$05`⇒ 疑似属性表填充的**数据源或写指针串位**（把 tile 数据/错行写进属性区），需专项取证。

### 6.3 `scroll_drift`：两腿差一个滚动 tick（2 px）

| room | 采集点 | 平移 dx/dy | 平移后残差 | 原始像素差 |
|---|---|---|---|---|
| 02 | f60 | 2/0 | 381 px | 10381 px |
| 02 | f120 | 2/0 | 311 px | 6036 px |
| 14 | f60 | 2/0 | 253 px | 9132 px |
| 14 | f120 | 2/0 | 475 px | 6696 px |
| 29 | f60 | 2/0 | 431 px | 13147 px |
| 29 | f120 | 2/0 | 327 px | 14407 px |
| 38 | f60 | 2/0 | 285 px | 12239 px |
| 38 | f120 | 2/0 | 461 px | 10027 px |
| 41 | f60 | 2/0 | 253 px | 10069 px |
| 41 | f120 | 2/0 | 286 px | 7954 px |

2 px 恰好等于**一帧门滚**（`ScrollDoor ; 07:E1F1` 每帧 2 调 ⇒ 2 px/帧）。平移后残差降到原始的
3-5%（几百像素），说明**主体内容相同、只是滚动相位差一个 tick**；残差部分是叠加在平移上的
局部差异（精灵/局部 tile），尚未逐房归因。由于滚动寄存器（PPU dump 尾 3B）在 emu 侧恒不可读
（§1.5），这一层只能靠平移自证 ⇒ 它是**滚动时序面**的问题，不是房间数据问题。

### 6.4 一个反直觉现象：`ok` 房里也有 9 个「名表字节不同但像素相同」

room 0/1/3/4/18/19/26/44/45 判 `ok`（像素零差），但它们的 PPU 名表里有字节不同。
原因：名表差异落在**当前未被显示的那一对名表**（`SelectRoomRAM` 在 `$6000`/`$6400` 间交替，
另一对在屏外），或落在屏幕外区域。⇒ **像素判据对这类差异不敏感**，不能拿「像素 0」当
「房间数据完全一致」。本报告因此把 PPU 层比对作为并行证据（§1.5），而不是只报像素。

## 7. 跳过清单（分母显式化）

计划 47 房，`skipped` **0**（理由枚举见 §1.3）：

**空**：本轮 `--rooms all` 的 47 房全部进入执行，没有一房因「房号不被接受」等原因被跳过。

另有 `blocked`（**不是跳过**：两腿都跑了，但采集点被对拍器的 `unported` 缺口判为阻塞，按对拍器纪律**不计入通过**）：

| room | 报告 | 备注 |
|---|---|---|
| 43 | `{'match': 1, 'mismatch': 0, 'blocked': 2, 'error': 0}` | 采集点 error/blocked：f60=blocked, f120=blocked |

**其它区**：`--areas` 默认 `0,1`，但 area 1（Norfair，46 房）因 `entry_preset` 未定稿而被**逐房**记 `entry_preset_unverified_area`（本轮显式只跑 `--areas 0`，故不在上面 47 房的账里）；area 2/3/4 记 `area_dispatch_unported`。两类都不伪造 0 px，见 §8。


## 8. 如何扩展到 Norfair 与另三区

### 8.1 Norfair（area 1，46 房）——缺的不是房流，是**区切换的两条注入**

房数（`ROOM_COUNTS[1] = 46`）与资产都在册，`PORTED_AREAS` 也已含 1。真正的阻塞是本工具
目前只对 area 0 用反证实验定稿过 `entry_preset`（`PRESET_VERIFIED_AREAS = (0,)`），
所以 area 1 的房会被**逐房**记 `entry_preset_unverified_area` 跳过，而不是产出一份看着像
结论、其实是工具产物的分数。

原因是注入 area 1 需要**两条**而非一条：

1. `$0074 IN_AREA = 0x01`——`room_draw.rs:70` / `room_load.rs:49,168` 都按
   `IN_AREA & 0x0F` 选资产表；开局进的是 Brinstar（`IN_AREA = 0`），不注区号时
   画出来的仍是 **Brinstar 同号房**。
2. **区指针块**（`CopyAreaPointers ; 07:C8B0`，`room_load.rs:167`、`tick.rs:14`）——
   emu 侧的房间绘制走的是 `MoreInit ; 07:C81D` 当时拷进 WRAM 的**区指针块**
   （struct 指针 / metatile 定义表指针等）。只改 `$0074` 而不重拷指针块时：
   **emu 腿仍用 Brinstar 的表，rust 腿按 `IN_AREA` 直接取 Norfair 资产表**⇒
   两腿数据源不同，逐像素必然大面积失配，而那是**工具产物**，不是移植差异。

所以扩展顺序应当是：① 在 f900 的注入里加 `$0074 = 1`，**并且**在注入后重跑一次区指针拷贝
（或直接把该区指针块注入 WRAM）；② 用 `$005A` 收口链确认房号被接受；③ 用 §1.5 的名表层
比对确认两腿都画的是 Norfair 内容且像素为 0；④ 通过后把 1 加进 `PRESET_VERIFIED_AREAS`
并升 `entry_preset_version` 到 v2——**首轮 Brinstar 数据保持 v1 不动**，两者在 JSON 里可区分。

> 现状：未做（本轮分数只覆盖 Brinstar）。这是 8.1 唯一的硬阻塞。
> 有了 ④ 这一处常量开关，`--areas 0,1` 的默认值就真的能跑两区。

### 8.2 Kraid / Tourian / Ridley（area 2/3/4）——卡在**区派发移植**，不是数据

- 房流、struct、metatile **都已在资产包内**（§0 更正），解析端也已接好。
- 真正阻塞：`crates/game-core/src/world/room_draw.rs` 的 `draw_room`/`draw_struct`
  在 `area ∉ {0,1}` 时 `unported!`。所以今天扫这三区只会在 Rust 腿爆 `unported`，
  工具会如实记 `blocked`（而不是伪造 0 px）。
- 打通顺序建议：① 按 `refs/m1disasm` 的区派发地址把这三区接进 `room_draw`（先只接数据面：
  `struct_ptrs`/`metatile_defs`/调色板），② 用本工具 `--areas 2 --rooms all` 逐房对拍，
  ③ 每区的 `unported!` 消失后把该区从 `PORTED_AREAS` 挪进去（一处常量）。

### 8.3 其它区/更大规模时的性能

emu 腿是串行的（`analysis/.scenario.lock`）⇒ `--jobs > 1` 基本不提速，当前单房 ~29 s
（120 帧窗口）。整区 47 房 ≈ 23 min。若要覆盖 5 区（193 房）≈ 1.6 h，
可把窗口缩到 60 帧（单采集点，代价是丢掉「滚动停」相位）或按 §1.4 `--resume` 分批。

## 9. 无法确证 / 遗留

- **伪绿防线**：`47` 房 × 3 采集点里，rust 侧截图的唯一 hash 数是 f120=46、f60=45、pre=1。`pre`（f900）恒为 1 个 hash 是**设计使然**（所有房在注入前是同一屏开局），`f60`/`f120` 接近满值（45/47、46/47）说明多数房确实画出了不同的屏，少数重合的房需人工确认不是「都没载入」。
- **精灵层无法直接证实**：OAM 未 dump，`sprite` 层结论是排除法（§1.5、§6.1）。
- **滚动寄存器无法对拍**：emu 侧 PPU dump 尾 3B 恒 `0,0,0`（`tools/scenario_capture.lua` 的 `dumpPpu` 对 `ppu.scrollX`/`ppu.scroll_x` 双形容错失效），`scroll_drift` 只能靠整屏平移自证。
- **房号身份未做 RoomPtr 级复核**：只以 `$005A` 收口链 + `MainRoutine` 守恒判定「房号被接受」，没有额外比对 `$33/$34 RoomPtr`。
- **窗口外未覆盖**：120 帧 / 240 px 之外（含 `ScrollX` 回绕后的换房）不在本轮观测面。
- **竖向入口未纳入**：`DoorEntryStatus=3` 已知精灵层分歧（§3.3），v1 preset 只做横向入口。
- **Norfair 未跑**：`$0074 IN_AREA` 与 `CopyAreaPointers ; 07:C8B0` 的区指针块注入实验未做⇒ area 1 的房被逐房记 `entry_preset_unverified_area` 跳过（不产可疑数据），见 §8.1。
- **room 43 的 `$FD` 敌兵段未解码**：只影响 §2 的实测边界统计用了 46/47 房（格式上限本身不受影响）。
- **room 43 被 `blocked`**：该房在对拍器中命中 `unported` 缺口（报告 `gap/blocked` 计数 2），属**已知缺口**而非本工具问题；按对拍器纪律不算通过。

