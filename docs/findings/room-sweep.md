# 房间批量扫描（room sweep）：Brinstar 47 房首轮

> 【状态】新基建 + 首轮真实数据（2026-11-19）。判据**原样复用** `tools/l4_diff_run.py`
> （L4 像素零容差 + 既有视觉白名单），本工具不改判据、不加白名单、不放宽阈值。
> 工具 `tools/movie_ref/room_sweep.py`；数据 `data/status/room_sweep_0.json`。
>
> 【2026-11-19 更正 · 判据收紧】独立对抗复核（pass-with-findings）指出旧 `ok` 判据有三处
> 假绿通道，已修（判据只收紧、未放宽）：① `ok` 现在要求**目标房身份证据**（`$33/$34 RoomPtr`
> 落在该房房流区间内，或采到 `$005A == 目标房号`），房号收口链本身不再算身份；②
> `f900..f906` 逐帧断言 `$001E MainRoutine==3`（旧版只在 899 帧记一次、且只记账不判定）；
> ③ 判 `ok` 要求 `l4_exit==0`，且报告必须是**本次**产物（跑前删本 stem 旧报告 + mtime
> 不早于本次启动 + `scenario_sha256` 与本次用例一致），陈旧报告记 `stale_report`。
> 本轮**未**重扫全部 47 房：只按最小规模复跑 room 0/1（仍判 `ok`，身份证据来源从「收口链」
> 升级为 `room_ptr` 区间）。其余 45 房的旧分数（`ok 15 / mismatch 31 / blocked 1`）是在**旧判据**
> 下产出的，尚未在新判据下重算；JSON 的 `criteria_version` 记录了这一点。

## 0. 结论摘要

| 项 | 值 |
|---|---|
| 分母（计划扫的房数） | `47` |
| 实际跑（两腿都起过） | `47` |
| `ok`（两腿跑完 且 每采集点 diff==0 且 房号被接受 且 有目标房身份证据 且 f900..f906 在 GameEngine） | `15`（**旧判据**下产出，新判据只复跑了 room 0/1） |
| 有失配（`mismatch`） | `31` |
| 缺口中阻塞（`blocked`） | `1` |
| 错误 / 超时 | `0` / `0` |
| 跳过（带理由） | `0` |
| 无身份证据（`unverified_identity`） | `0` |
| 离开 GameEngine（`left_gameengine`） | `0` |
| 陈旧/不符报告（`stale_report`） | `0` |
| 单房均耗时 | `28.7 s（n=47）` |

- `entry_preset v1` = **4 个注入量**（帧 900，两条腿语义相同）：
  `$005A RoomNumber`、`$0049 ScrollDir=3`、`$00FD ScrollX=2`、`$0056 DoorEntryStatus=1`。
  逐字节理由与反证实验见 §3。
- 能力边界（必须先读）：本方法证明的是**房间数据 + 载入链 + 绘制 + 该注入坐标下的物理**，
  **不证明可达性/连通性**（见 §4）。
- 最大失配前 5 房见 §6；跳过清单见 §7。
- 采集点分层（94 点）：`nametable` **61** / `clean` **15** / `scroll_drift` **10** /
  `sprite` **6** / `unknown` **2**（room 43 阻塞）。
- **首轮最值得追的三条线索**（都在名表/滚动面，且已定位到字节级；数字口径与离线脚本
  `tools/movie_ref/room_sweep_stats.py` 一致，唯一字节 = 镜像去重后只计 `nt0+nt1`）：
  1. **房间边界列 tile**：emu 写 `$4E`、rust 写 `$FF`（空白），只出现在 `col 2`（28 个采集点）/
     `col 29`（14 个采集点；仅 col2 23、仅 col29 9、两者都有 5），`col 14` 另有 1 点（12 B）；
     唯一 tile 差字节 **204**（col2 114 / col29 78 / col14 12），其中 `4E→FF` 对 **162** 次
     ⇒ 边界列填充/流式边缘列路径（§6.2）。
  2. **属性表数据源疑似串位**：**19 个采集点**（房 7/8/10/13/15/22/25/27/32/35/42）**只有**属性表差
     且 tile 全同，emu 侧是整字节同色（四象限同值）占 **323/369 = 87.5%**，rust 侧却是
     逐象限值、逐象限互异 **21/369**（tile 索引形态）⇒ 像是把 tile 数据写进了属性区（§6.2）。
     复核稿给的「296/349 = 85%、rust 22/349」无法从入库 dump 复算（同口径 323/369），
     统计脚本里标为 `review_unreproducible`，本报告采用可复算值。
  3. **滚动相位差一个 tick**：10 个采集点整屏平移 `dx=2` 后残差降到原始的 **2.3%–7.1%**（§6.3）。

> 【2026-11-19 更正 · 任务书前提】任务书把代表房分成「普通房 / 竖滚房 / 宽房（横向滚）」。
> 该分类**不是数据尺寸类别**：房间数据本身恒为**一屏**（§2 证据），「竖滚/横向宽」实际是
> **入口滚动轴**（`$0049 ScrollDir` / `$00FC ScrollY`）/ **世界图邻域**属性（`GetRoomNum ; 07:E720`
> 按地图格取房号），不是房间数据字节的属性。代表房因此改为按内容选：
> room 20（稀疏）、room 36（物件最多，20 个）、room 22（含 type-7 管虫敌兵）；
> 竖向滚动路径另做一次**注入路径**实验（DoorEntryStatus=3），结果见 §3.3。

> 【2026-11-19 更正 · 任务书前提】任务书称「Kraid/Tourian/Ridley 房流尚未迁包」。
> 实测**五区房流都在资产包内**（`cargo run -q -p pack-tool -- list --pack assets/metroid.pack`
> ⇒ `area.kr.room_streams` 1985 B / `area.to.room_streams` 871 B / `area.ri.room_streams` 1928 B，
> 解析端 `crates/game-data/src/assets.rs:183-192,246-254` 也确有 KR/TO/RI 表）。
> 2/3/4 区今天仍然**不能扫**的真正原因是**工具自己的白名单** `PORTED_AREAS=(0,1)`
> （`tools/movie_ref/room_sweep.py`；`entry_preset` 只在 area 0 用反证实验定稿）——
> **不是**区派发未移植：`crates/game-data/src/assets.rs:238-247` 的 `room_streams` 支持
> `0..=4`，`crates/game-core/src/world/room_draw.rs:67-77` 的 `unported!` 只在 assets 返回
> `None` 时触发，对 0–4 区永不触发。所以本工具把它们记成 `area_not_in_tool_allowlist`，
> 既不是「房流未迁包」，也不是「区派发未移植」。room 43 的 `blocked` 来自 `$FD` 敌兵路径
> （`bank07:EAD4`），与区派发无关（§7）。

## 1. 方法与判据

### 1.1 一房一用例

对每个 `(area, room)` 生成临时用例 `cases/_tmp_sweep_<token>_a<area>_r<room:02d>.txt`
（`token = <pid>-<启动 epoch 秒>`；每房跑完即删，进程被信号杀掉时也删；启动时**只清同
token 的残留**，不碰别的实例在飞的用例），内容形如：

```
name _tmp_sweep_<token>_a0_r20
frames 1020
watch 001E MainRoutine
watch 005A RoomNumber
watch 0033 RoomPtrLo
watch 0034 RoomPtrHi
watch 0049 ScrollDir
watch 00FD ScrollX
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
ok = 两腿都跑完（l4_exit == 0，无 error / blocked / timeout）
     且 每个采集点 diff_pixels == 0
     且 注入房号被引擎接受（$005A 走完 F0→…→FF 收口链）
     且 有目标房身份证据：$33/$34 RoomPtr 落在该房房流区间内，
        或采到 $005A == 目标房号（identity_basis ∈ {room_ptr, room_number_frame}）
     且 f900..f906 逐帧 $001E MainRoutine == 3（GameEngine）
     且 l4report 是本次产物（mtime ≥ 本次启动 且 scenario_sha256 == sha256(本次用例)）
```

**分母恒为「计划扫的房数」**，不是「跑成功的房数」，且两条台账恒等式机械断言
（违反时工具 stderr 打印并以非 0 退出码结束，JSON 里 `summary.arithmetic_ok=false`）：

```
planned = ran + skipped
ran     = ok + mismatch + blocked + error + timeout + unverified_identity + left_gameengine
```

`skip_reason` 枚举（全部落在 JSON 的 `skips[]` 里，不静默）：

| 理由 | 含义 |
|---|---|
| `area_not_in_tool_allowlist` | 区不在工具白名单 `PORTED_AREAS=(0,1)`（房流在包内、渲染侧也支持，见 §0 更正）；不是「区派发未移植」 |
| `room_streams_not_in_pack` | 资产包内无该区房流 |
| `area_unknown` | 区号不在 0..4（`room=None`，只进 `summary.area_skips`，不混进 `planned`） |
| `room_index_out_of_range` | 房号越界（如 Brinstar 只有 0..46） |
| `room_number_not_ff` | 注入后 `$005A` 未走完 F0→…→FF ⇒ 房号未被接受 ⇒ 像素结果**不计入结论** |
| `entry_preset_unverified_area` | 该区 `entry_preset` 未用反证实验定稿（目前只有 area 0）；整区按房逐个跳过，不产出可疑数据（§8.1） |

非跳过、但同样 `ok=false` 的两个新状态：`unverified-identity`（收口链跑完但拿不到身份证据）
与 `left-gameengine`（f900..f906 有帧 `MainRoutine≠3`）；报告陈旧记 `error` 并计入
`summary.stale_report`。三者都不算 `ok`，也不会被 `--resume` 沿用。

每个被跳过的房**单独一行**（`room` 具体到房号）⇒ `planned = ran + skipped` 机械可核。

`ok` 与「房号被接受」必须同时成立，否则「0 px」可以来自「两边都没载入这个房」——
这就是 `room_number_not_ff` 单列的理由。

### 1.4 幂等与可中断

- 同一 `--out` 重跑**覆盖**（临时文件 + `os.replace` 原子写）。
- **每跑完一房就落盘**（`"partial": true`），中断后 `data/status/*.json` 仍有已完成部分；
  `--resume` 跳过其中已 `ok` 的房。
- 单房任何异常（用例错误、腿超时、缺报告）都记一条记录后**继续**，不中断整批。
- 信号处理：`SIGINT`/`SIGTERM` ⇒ 删正在用的临时用例 + `SystemExit(130)`。
- 启动清理**按本次 token 限定**：只删 `cases/_tmp_sweep_<token>_*.txt`，两个实例并发扫描时
  不会互删在飞用例（首轮 room 29 的 error 就是旧版无差别清理造成的，见 JSON `note`）。

### 1.5 除「像素差」之外，还把差异**分解到 PPU 层**

两条腿对每个采集点都 dump PPU 真值（emu `analysis/traces/<case>_f*_<tag>.ppu.hex`、
rust `shots_rust/<case>_f*_<tag>.ppu.hex`；格式 `$0000-$2FFF`（CHR 8KB + 名表 4KB）+ 调色板
`$3F00-$3F1F` + 尾 3B）。工具按区间比对：

| 层 | 区间 | 含义 |
|---|---|---|
| `chr` | `$0000-$1FFF` | 图案表（tile 像素来源） |
| `nt0..nt3` | `$2000-$2FFF` | 名表（含属性表 `$23C0`/`$27C0`/`$2BC0`/`$2FC0`）⇒ **房间数据面** |
| `palette` | `$3F00-$3F1F` | 调色板 |
| `scroll_drift` | 推理层 | 以上全同、但整屏平移 `dx/dy ∈ [-4,4]` 后残差 ≤ 50% ⇒ **滚动相位差一个 tick** |
| `sprite` | 推理层 | 以上全同、平移也解释不了 ⇒ 只能来自 OAM（OAM 未 dump，故为**排除法**） |
| `clean` | — | 像素 0 且 PPU 各区间全同 |
| `unknown` | — | 缺 PPU dump（采集点被 `blocked`/`error`） |

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
机器可读：`data/status/room_sweep_0.json` 每房的 `room_load.room_frames` / `ff_markers` /
`back_to_ff_frame`，以及判据用的 `load_chain` / `identity_basis` / `room_ptr_region` /
`mr_window`（f900..f906）/ `mr_window_ok`。

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

> 本表所有**字节数均为「唯一字节」**：PPU dump 的垂直镜像 `nt2==nt0`、`nt3==nt1`（两腿都成立，
> 139 个采集点无一例外），旧稿按 `nt0..nt3` 四区求和把字节数**翻倍**了（如 room 30 f60 的
> 12+48=60 B 实为 6 tile + 24 attr = 30 B）。逐采集点的字节数相加 ≠ §6.2 的「每房并集唯一值」，
> 因为同一偏移会同时出现在 f60/f120（§6.2 给的是并集口径）。

| room | status | f900 注入前 | f60 | f120 | 层(pre/60/120) | 备注 |
|---|---|---|---|---|---|---|
| 30 | mismatch | 0 | 14467 | 14894 | clean/nametable/nametable | f60: tile 6B 列[2] emu→rust 4E→FF，attr 24B emu→rust 55→56；f120: tile 12B 列[2, 29] emu→rust 4E→FF，attr 24B emu→rust 55→56 |
| 29 | mismatch | 0 | 13147 | 14407 | clean/scroll_drift/scroll_drift | 层：f60=scroll_drift,f120=scroll_drift |
| 25 | mismatch | 0 | 11958 | 13370 | clean/nametable/nametable | f60: attr 32B emu→rust 55→4D；f120: attr 32B emu→rust 55→4D |
| 24 | mismatch | 0 | 12905 | 13071 | clean/nametable/nametable | f60: tile 6B 列[2] emu→rust 4E→FF，attr 17B emu→rust 55→02；f120: tile 12B 列[2, 29] emu→rust 4E→FF，attr 17B emu→rust 55→02 |
| 37 | mismatch | 0 | 13057 | 12958 | clean/nametable/nametable | f60: tile 12B 列[14] emu→rust FF→55/57；f120: tile 30B 列[29]（整列 30 行）emu→rust FF→56/58/4C/50/B8/B9/D6/CC |
| 38 | mismatch | 0 | 12239 | 10027 | clean/scroll_drift/scroll_drift | 层：f60=scroll_drift,f120=scroll_drift |
| 02 | mismatch | 0 | 10381 | 6036 | clean/scroll_drift/scroll_drift | 层：f60=scroll_drift,f120=scroll_drift |
| 41 | mismatch | 0 | 10069 | 7954 | clean/scroll_drift/scroll_drift | 层：f60=scroll_drift,f120=scroll_drift |
| 28 | mismatch | 0 | 9314 | 9877 | clean/nametable/nametable | f60: tile 6B 列[2] emu→rust 4E→FF，attr 10B emu→rust 00→A5；f120: tile 6B 列[2] emu→rust 4E→FF，attr 10B emu→rust 00→A5 |
| 14 | mismatch | 0 | 9132 | 6696 | clean/scroll_drift/scroll_drift | 层：f60=scroll_drift,f120=scroll_drift |
| 16 | mismatch | 0 | 3507 | 5603 | clean/nametable/nametable | f60: tile 6B 列[2] emu→rust 4E→FF，attr 16B emu→rust AA→12；f120: tile 12B 列[2, 29] emu→rust 4E→FF，attr 16B emu→rust AA→12 |
| 13 | mismatch | 0 | 0 | 3620 | clean/nametable/nametable | f120: tile 6B 列[29] emu→rust 4E→FF，attr 16B emu→rust AA→12 |
| 12 | mismatch | 0 | 2885 | 3010 | clean/nametable/nametable | f60: tile 6B 列[2] emu→rust 4E→FF，attr 8B emu→rust AA→12；f120: tile 6B 列[2] emu→rust 4E→FF，attr 8B emu→rust AA→12 |
| 35 | mismatch | 0 | 1015 | 2438 | clean/nametable/nametable | f60: attr 57B emu→rust AA→55；f120: attr 57B emu→rust AA→55 |
| 31 | mismatch | 0 | 1058 | 1336 | clean/nametable/nametable | f60: tile 6B 列[2] emu→rust 4E→FF，attr 27B emu→rust 55→56；f120: tile 6B 列[2] emu→rust 4E→FF，attr 27B emu→rust 55→56 |
| 32 | mismatch | 0 | 635 | 1210 | clean/nametable/nametable | f60: attr 53B emu→rust 55→57；f120: tile 6B 列[29] emu→rust 4E→FF，attr 53B emu→rust 55→57 |
| 15 | mismatch | 0 | 990 | 1146 | clean/nametable/nametable | f60: attr 4B emu→rust AA→80；f120: attr 4B emu→rust AA→80 |
| 42 | mismatch | 0 | 20 | 747 | clean/nametable/nametable | f60: attr 9B emu→rust 55→80；f120: attr 9B emu→rust 55→80 |
| 27 | mismatch | 0 | 289 | 703 | clean/nametable/nametable | f60: attr 14B emu→rust 00→80；f120: attr 14B emu→rust 00→80 |
| 08 | mismatch | 0 | 296 | 604 | clean/nametable/nametable | f60: attr 16B emu→rust FF→00；f120: attr 16B emu→rust FF→00 |
| 21 | mismatch | 0 | 210 | 345 | clean/sprite/sprite | 层：f60=sprite,f120=sprite |
| 05 | mismatch | 0 | 289 | 289 | clean/nametable/nametable | f60: tile 6B 列[2] emu→rust 4E→FF，attr 16B emu→rust FF→04；f120: tile 12B 列[2, 29] emu→rust 4E→FF，attr 16B emu→rust FF→04 |
| 11 | mismatch | 0 | 256 | 256 | clean/nametable/nametable | f60: tile 6B 列[2] emu→rust 4E→FF，attr 10B emu→rust 00→FF；f120: tile 6B 列[2] emu→rust 4E→FF，attr 10B emu→rust 00→FF |
| 22 | mismatch | 0 | 0 | 210 | clean/nametable/nametable | f120: attr 16B emu→rust 55→51 |
| 10 | mismatch | 0 | 0 | 184 | clean/nametable/nametable | f120: tile 6B 列[29] emu→rust 4E→FF，attr 8B emu→rust 00→5C |
| 07 | mismatch | 0 | 41 | 148 | clean/nametable/nametable | f60: attr 8B emu→rust FF→80；f120: attr 8B emu→rust FF→80 |
| 40 | mismatch | 0 | 146 | 135 | clean/sprite/nametable | f120: tile 6B 列[29] emu→rust 4E→FF |
| 39 | mismatch | 0 | 0 | 141 | clean/nametable/nametable | f120: tile 6B 列[2] emu→rust 4E→FF |
| 46 | mismatch | 0 | 141 | 141 | clean/nametable/nametable | f60: tile 6B 列[2] emu→rust 4E→FF，attr 8B emu→rust FF→04；f120: tile 6B 列[2] emu→rust 4E→FF，attr 8B emu→rust FF→04 |
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
| 43 | blocked | 0 | None | None | clean/unknown/unknown | 采集点被 unported 缺口阻塞（`bank07:EAD4`，见 §7） |
| 44 | ok | 0 | 0 | 0 | clean/nametable/nametable | 两腿一致 |
| 45 | ok | 0 | 0 | 0 | clean/clean/nametable | 两腿一致 |

## 6. 失配分层与最大失配

失配房 **31** 个（`ok` 15 / `blocked` 1 / 跳过 0）。按「最大单点像素差」降序，前 5 房：

| # | room | 最大失配 | 采集点明细 | 名表字节差分解（离线读两腿 PPU dump） |
|---|---|---|---|---|
| 1 | 30 | 14894 px | pre=0 · f60=14467 · f120=14894 | f60: tile 6B 集中在列 [2] 值 4E→FF；attr 24B（行 [0, 3, 4, 7]）值 55→56,55→57,55→58 / f120: tile 12B 集中在列 [2, 29] 值 4E→FF；attr 24B（行同上）值同 |
| 2 | 29 | 14407 px | pre=0 · f60=13147 · f120=14407 | f60: PPU 三层全同 / f120: PPU 三层全同 |
| 3 | 25 | 13370 px | pre=0 · f60=11958 · f120=13370 | f60: attr 32B（行 [0, 1, 4, 5]）值 55→4D,55→50,55→4B / f120: 同 |
| 4 | 24 | 13071 px | pre=0 · f60=12905 · f120=13071 | f60: tile 6B 集中在列 [2] 值 4E→FF；attr 17B（行 [0, 3, 4, 7]）值 55→02,55→03,55→01 / f120: tile 12B 集中在列 [2, 29] 值 4E→FF；attr 17B 值同 |
| 5 | 37 | 13057 px | pre=0 · f60=13057 · f120=12958 | f60: tile 12B 集中在列 [14] 值 FF→55,FF→57 / f120: tile 30B 集中在列 [29]（整列 30 行）值 FF→56,FF→58,FF→4C,FF→50,FF→B8,FF→B9,FF→D6,FF→CC |

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

1. **tile 差异只落在房间的左右边界列**：含 tile 差的采集点里，`col 2` **28 个**、`col 29` **14 个**
   （仅 col2 **23**、仅 col29 **9**、两者都有 **5**），`col 14` 另有 **1** 个采集点（room 37 f60，12 B；
   同一房 f120 的差异列已移到 col29，是**流式边缘列**，见 §6.3）。按每房 f60/f120 **并集**、
   镜像去重后的**唯一字节**：共 **204 B**（col2 **114** / col29 **78** / col14 **12**），值形态高度集中：
   emu 写 `$4E`、rust 写 `$FF`（空白）的 `4E→FF` 对 **162 次**；room 37 还有反向的 `FF→55/57`
   （f60，col14）与 f120 整列 `FF→56/58/4C/...`。⇒ 指向**房间边界列填充/流式边缘列**路径，
   而不是整房数据错乱。
   > 口径说明：`col2 114 / col29 78` 与复核稿的数字一致，但**必须**把 room 37 f120 那整列
   > 30 B（`nt1` col29 全 30 行）从「col29」改归「col2 类」才成立；按 `off % 32` 的自然口径是
   > col2 84 / col29 108 / col14 12。复核稿反推不出它用的是哪种归类，离线脚本
   > `tools/movie_ref/room_sweep_stats.py` 两种都打印（自然口径 84/108 作为断言，114/78 作为
   > 「复核归类」复算）。
2. **属性表差异与 tile 差异互斥**：**19 个采集点**（房 7/8/10/13/15/22/25/27/32/35/42）**只有**
   属性表差、tile 全同，且 f60 与 f120 的差异字节位置**完全相同** ⇒ 发生在**房间绘制阶段**
   （`init_tables` / `attrib_table_write ; 07:EA2B` 区），不是流式阶段。值形态：按每房并集、镜像
   去重后唯一属性差字节 **369**，其中 emu 侧「整字节同色」（四象限同值：`$00/$55/$AA/$FF`）
   **323/369 = 87.5%**，rust 侧逐象限互异 **21/369**（多为 `$4B-$57` 或低值 `$00-$05`）⇒ 疑似
   属性表填充的**数据源或写指针串位**（把 tile 数据/错行写进属性区），需专项取证。
   > 复核稿写作「296/349 = 85%、rust 逐象限不同 22/349」；按本仓库 dump、同口径复算是
   > 323/369（分母差 20 B，可能是采集点/房集合口径不同）。入库脚本把这组复核数字标为
   > `review_unreproducible` 并只打印、不断言——报告采用**可复算**的 323/369。

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

2 px 恰好等于**一帧门滚**（`ScrollDoor ; 07:E1F1` 每帧 2 调 ⇒ 2 px/帧）。平移后残差降到
原始的 **2.3%–7.1%**（最小 2.27% = room 29 f120 `327/14407`，最大 7.09% = room 14 f120
`475/6696`；只统计 `layer=="scroll_drift"` 的 10 个采集点，若把 nametable 层也算进来会得到
1.41%–10.83%，那是另一个口径），说明**主体内容相同、只是滚动相位差一个 tick**；残差部分是
叠加在平移上的局部差异（精灵/局部 tile），尚未逐房归因。由于滚动寄存器（PPU dump 尾 3B）在 emu 侧恒不可读
（§1.5），这一层只能靠平移自证 ⇒ 它是**滚动时序面**的问题，不是房间数据问题。

### 6.4 离线统计脚本（数字可复算）

`tools/movie_ref/room_sweep_stats.py`（只读，无新依赖）：输入 `data/status/room_sweep_0.json` +
`analysis/traces/<case>_f*_<tag>.ppu.hex` + `shots_rust/<case>_f*_<tag>.ppu.hex`，按本节口径
重算并以断言核对文档数字（任一不符打印 FAIL、退出码 1；镜像不成立退出码 2）。它只**打印**
clues JSON，不落文件——clues 的归宿是 `data/status/room_sweep_0.json` 的 `clues` 字段，
同轮写入，避免 data/status 下多出无主文件。

复算输出（本轮）：tile 采集点 38 · col2 28 / col29 14 / both 5 / only 23 / only 9 · 唯一 tile 204
（col2 84 / col29 108 / col14 12）· `4E→FF` 162 · attr 唯一 369 · emu 四象限同色 323 · rust 四象限
互异 21 · attr-only 19 点 · 层分布 61/15/10/6/2 · shift 残差 2.27%–7.09%。复核稿的
`296/349`、`114/78` 无法由入库 dump 机械复算，脚本里以 `review_unreproducible` 记录差异，
不作为断言。

### 6.5 一个反直觉现象：`ok` 房里也有 9 个「名表字节不同但像素相同」

room 0/1/3/4/18/19/26/44/45 判 `ok`（像素零差），但它们的 PPU 名表里有字节不同。
原因：名表差异落在**当前未被显示的那一对名表**（`SelectRoomRAM` 在 `$6000`/`$6400` 间交替，
另一对在屏外），或落在屏幕外区域。⇒ **像素判据对这类差异不敏感**，不能拿「像素 0」当
「房间数据完全一致」。本报告因此把 PPU 层比对作为并行证据（§1.5），而不是只报像素。

> **镜像双计提醒**：PPU dump 的名表两两垂直镜像（`nt2==nt0`、`nt3==nt1`，139 个采集点全部
> 成立），所以按 `nt0..nt3` 四区直接求和会把字节数**翻倍**。本报告与离线脚本的所有字节量
> 都是「唯一字节」（只计 `nt0+nt1`）。

## 7. 跳过清单（分母显式化）

计划 47 房，`skipped` **0**（理由枚举见 §1.3）：

**空**：本轮 `--rooms all` 的 47 房全部进入执行，没有一房因「房号不被接受」等原因被跳过。

另有 `blocked`（**不是跳过**：两腿都跑了，但采集点被对拍器的 `unported` 缺口判为阻塞，按对拍器纪律**不计入通过**）：

| room | 报告 | 备注 |
|---|---|---|
| 43 | `{'match': 1, 'mismatch': 0, 'blocked': 2, 'error': 0}` | 采集点 error/blocked：f60=blocked, f120=blocked；原因是 `$FD` 敌兵路径未移植（`crates/game-core/src/world/room_load.rs` 末端 `unported!("bank07:EAD4")`），与区派发无关 |

**其它区**：`--areas` 默认 `0,1`，但 area 1（Norfair，46 房）因 `entry_preset` 未定稿而被**逐房**记 `entry_preset_unverified_area`（本轮显式只跑 `--areas 0`，故不在上面 47 房的账里）；area 2/3/4 记 `area_not_in_tool_allowlist`（工具白名单 `PORTED_AREAS=(0,1)` 自限，房流与渲染表都在包内，见 §0 更正）。两类都不伪造 0 px，见 §8。


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

> **【2026-11-20 实测更正】按上述①②做过两轮，两轮都失败，且失败模式已定谳：
> 这不是「缺一条注入」，而是注入法对非 0 区**结构性不可用**。负结果留档如下（代码已回退，
> `tools/movie_ref/room_sweep.py` 保持 v1 单区形态）。**
> - **第一轮（同帧注入 `$0074` + 14 B 区指针块）**：三房全部 `$005A` 走不完链。逐帧读
>   `frames.jsonl`：`$005A` 到 `$F1` 后**整机冻结**（`ScrollX` 也停止递增）⇒ CPU 不是在等，
>   是跑进了垃圾代码。根因可解释：注入帧上 `$8000` 窗口里**映射的还是 Brinstar 的 bank**，
>   而区指针块写的是**本区 ROM 地址**（如 Norfair `$A21B`）⇒ `(RoomPtrTable),y` 用本区地址
>   去读旧 bank 的数据，取到垃圾房指针 ⇒ `DrawRoom` 在无 `$FF` 终止符的数据上走飞。
> - **第二轮（拆两段：t=0 注入 `$0074`，t=+2 再注入房号 + 指针块）**：链推进到 `$F3` 后
>   再次冻结（`$005A=243`、`ScrollX` 卡在 10）。说明「等 `CheckBankSwitch` 换完 bank」
>   并不成立：`CheckBankSwitch ; 07:C4DE` + `InitBankN` 只负责 bank/图形/调色板，
>   **`CopyAreaPointers ; 07:C8B0` 全仓只有两个调用点**——`AreaInit ; 07:C875-1891`
>   （`MainRoutine=0` 的区初始化）与电梯到达径（`prg7_engine.asm:5038`）。
>   从 `GameEngine` 直接改 `InArea` 永远追不到这两条岸。
> - **第三轮（合成电梯对象面把 bank 换过去再注房号）**：同样走不完链。电梯对象的目标区与
>   落点在 `LoadElevator ; 07:EC04` 时由**当前房流**写入，在别的房里凭空合成它没有可依据的
>   字段值 ⇒ 又是「伪造状态」。

> **结论**：非 0 区的房扫，**只有在真实到达该区之后**才有合法观测窗口。可行路径只剩两条：
> ①用 `cases/case-elev-norfair-transition.txt` 那条已验证的电梯径**先进 Norfair**，到达后再
> 注房号（需要把电梯径的 fixture 面按真实房流逐房重放，成本高）；②**放弃注入法**，改用
> TAS 录像 inputlog 走进真实房间（与 `docs/spec/acceptance-paradigm.md` 的验收范式同源）。
> 在此之前，`--areas 1..4` 继续逐房记 `entry_preset_unverified_area` 跳过——**不造假分数**。

> 现状：未做（Brinstar 47 房的数据仍是唯一有区身份证据的房扫账）。

### 8.2 Kraid / Tourian / Ridley（area 2/3/4）——卡在**工具白名单**，不是区派发

- 房流、struct、metatile **都已在资产包内**（§0 更正），解析端也已接好：
  `crates/game-data/src/assets.rs:238-247` 的 `room_streams` 对 `0..=4` 都有表。
- `crates/game-core/src/world/room_draw.rs:67-77` 的 `unported!` **只在 assets 返回 `None` 时**
  触发，对 0–4 区永不触发（旧结论「后三区卡在区派发」是错的，已在本节更正）。
- 真正的门是**工具自己的** `PORTED_AREAS=(0,1)`：2/3/4 区未实测（`entry_preset` 未定稿），
  所以按房记 `area_not_in_tool_allowlist`。打开它不需要改 `crates/`。
- 打通顺序建议：① 用 8.1 同一套注入实验给 2/3/4 区定稿 `entry_preset`；② 把该区加进
  `PORTED_AREAS` 与 `PRESET_VERIFIED_AREAS`（一处常量）；③ 用本工具 `--areas 2 --rooms all`
  逐房对拍；④ 若某房在 Rust 腿爆 `unported`，按爆点单独记缺口（如 room 43 的 `bank07:EAD4`），
  不等于区派发未移植。

### 8.3 其它区/更大规模时的性能

emu 腿是串行的（`analysis/.scenario.lock`）⇒ `--jobs > 1` 基本不提速，当前单房 ~29 s
（120 帧窗口）。整区 47 房 ≈ 23 min。若要覆盖 5 区（193 房）≈ 1.6 h，
可把窗口缩到 60 帧（单采集点，代价是丢掉「滚动停」相位）或按 §1.4 `--resume` 分批。

## 9. 无法确证 / 遗留

- **伪绿防线**：`47` 房 × 3 采集点里，rust 侧截图的唯一 hash 数是 f120=46、f60=45、pre=1。`pre`（f900）恒为 1 个 hash 是**设计使然**（所有房在注入前是同一屏开局），`f60`/`f120` 接近满值（45/47、46/47）说明多数房确实画出了不同的屏。**但 hash 只说明「屏不同」，不说明「是目标房」**——本轮修复后 `ok` 另有 `$33/$34` 房流区间 / `$005A==房号` 的身份证据（§1.3），所以「两腿都没载入目标房」这种假绿现在会落 `unverified-identity` 而不是 `ok`。少数 hash 重合的房仍需人工确认来源。
- **精灵层无法直接证实**：OAM 未 dump，`sprite` 层结论是排除法（§1.5、§6.1）。
- **滚动寄存器无法对拍**：emu 侧 PPU dump 尾 3B 恒 `0,0,0`（`tools/scenario_capture.lua` 的 `dumpPpu` 对 `ppu.scrollX`/`ppu.scroll_x` 双形容错失效），`scroll_drift` 只能靠整屏平移自证。
- **房号身份已升级为 RoomPtr 区间证据**（本轮修复）：`$33/$34` 是房流**读游标**（随读随加，
  实测 room 0 在 f901..f904 采到 `A448/A44E`，起始是 `A441`），所以判据是「游标落在该房
  `[起始, 起始+长度)` 内」；区间由资产包 `area.<tag>.room_ptrs` + `area.<tag>.room_offs`
  机械导出（纯 Python 解析 `assets/metroid.pack`，不新增依赖）。取不到区间时才回退到
  「任一帧 `$005A == 目标房号`」，两条都取不到 ⇒ `unverified-identity`，不判 `ok`。
- **sched_sha256 未在工具侧复核**：`l4report` 里还有 `sched_sha256`，但它由
  `tools/l4_diff_run.py` 的 `expand_schedule` 生成，工具侧没有公开入口可复算；照抄那段逻辑
  等于把判据复制两处。因此本工具只用「跑前删旧报告 + mtime 不早于本次启动 +
  `scenario_sha256 == sha256(本次用例)`」，并在代码注释里写明这个取舍。
- **`note_rerun` 是手工合并的**：首轮 JSON 的 `note_rerun`（room 29 被旧版启动清理误删在飞
  用例、单独重跑覆盖）不是工具产出。工具现已支持 `--note` 并在重跑时保留旧 `note`/`note_rerun`；
  旧 JSON 另标 `manual_note: true`，说明该字段来源。
- **窗口外未覆盖**：120 帧 / 240 px 之外（含 `ScrollX` 回绕后的换房）不在本轮观测面。
- **竖向入口未纳入**：`DoorEntryStatus=3` 已知精灵层分歧（§3.3），v1 preset 只做横向入口。
- **Norfair 未跑**：`$0074 IN_AREA` 与 `CopyAreaPointers ; 07:C8B0` 的区指针块注入实验未做⇒ area 1 的房被逐房记 `entry_preset_unverified_area` 跳过（不产可疑数据），见 §8.1。
- **room 43 的 `$FD` 敌兵段未解码**：只影响 §2 的实测边界统计用了 46/47 房（格式上限本身不受影响）。
- **room 43 被 `blocked`**：该房在对拍器中命中 `unported` 缺口（报告 `gap/blocked` 计数 2），属**已知缺口**而非本工具问题；按对拍器纪律不算通过。


## 8.2 更正：非 0 区注入**在过区之后可用**（2026-11-21）

§8.1 的结论「注入法对非 0 区结构性不可用」只在**冷启动注入**（`InArea` 直接改、
`CopyAreaPointers ; 07:C8B0` 从未跑）时成立。经合法电梯过区后情况不同：
`ElevatorDestination` 路径会跑 `InitBankN` + `CopyAreaPointers`，此后 `InArea=$02`、
`AreaPointers_RAM($3B-$48)` 已是 Norfair 的真值，再注入 `$005A` 走的是同一条
`SetupRoom` 正则装载链。

实测（`cases/case-elev-norf-room-load.txt`，12 采集点全 **0px**）：
合成电梯对象 → 电梯十态跑完 → 落地后 `fixture 1500 005a 00` → f1620/f1700
两帧与 emu 逐像素一致。⇒ **非 Brinstar 的像素验证有正规入口**，
`room_sweep` 可按「先过区、再注房号」扩展，不必走 TAS inputlog。

## 8.3 比较器缺陷更正：名表差异必须按**物理页**比（2026-11-22）

`ppu_layer_diff` 原先按 dump 页号直比 `nt0..nt3`，而两腿 dump 的 CIRAM 布局可能不同
（emu 水平镜像 `$2000≡$2400`、rust 竖直镜像 `$2000≡$2800`）⇒ **同一物理页的不同镜像位
被判成名表差异**，`classify_layer` 因此在名表字节差 >0 时抢先返回 `nametable`。

实证（Norfair room 15 / f1600）：原始 `nt1=471, nt2=471`，但按物理页归并后 **`nt_norm=0`**
（`nt_layout=horizontal/vertical`）⇒ 该房从来不是"名表 bug"，此前 `nametable 37/47`、
`20` 这类层统计**不可信**，需在修好的比较器上重算。

修正：新增 `_nt_pages/_mirror_pages/_nt_diff_normalized`，`ppu_layer_diff` 增加
`nt_norm/nt_layout/nt_trusted`；`classify_layer` 改用 `nt_norm`，布局判不出时返回
`nametable_notmirror`（显式标不可信）。锚测 `tools/tests/test_nt_mirror.py`（3 条）。

## 8.4 三区（Kraid/Tourian/Ridley）入口：**电梯类型 = 目标区号**（2026-11-23）

非 0 区的注入法已由 §8.2 证明可用（过区之后注入 `$005A`），本节的增量是**如何到达另三区**。

asm（`refs/m1disasm/SRC/prg7_engine.asm:4997-5010`，`ElevatorD8BF ; 07:D8BF` 落地段）：

```
tya
bpl @endIf_B          ; bit7 清 = 下行 ⇒ A 的低半字节**就是目标区号**
ldy #$00              ; 上行：默认 Brinstar
cmp #$84
bne @endIf_C
iny                   ; $84 ⇒ Norfair
@endIf_C
tya
@endIf_B
ora #$10              ; InArea 形态（bit4 = 游戏引擎内）
jsr IsEngineRunning    ; → SwitchBank(07:CA23)：sta InArea + BankTable,y(07:CA30)
```

⇒ **下行电梯 `ElevatorType` 的低半字节 = 目标区号**：`1=Norfair / 2=Kraid / 3=Tourian / 4=Ridley`；
上行电梯只可能是 Brinstar(`≠$84`) 或 Norfair(`=$84`)。落地后 `ora #$10` 得 `InArea = $1n`，
紧接着 `CopyAreaPointers ; 07:C8B0` 建区指针块——这正是 §8.1 缺的那两条注入，电梯链自动完成。

工具改动（`tools/movie_ref/room_sweep.py`）：

- `ELEVATOR_AREAS = {1: 1, 2: 2, 3: 3, 4: 4}`（area → ElevType），fixture 由写死 `01` 改成按区取；
- 启动打印 `entry=` 之前恒报 `f900`（误导），改为 `elev=1600` / `plain=900`。

反证（各 41 s，`--frames 120`、采点 `arrived(1400)/pre(1600)/prev60(1659)/f60(1660)/prevend(1719)/f120(1720)`）：

| 区 | room 0 | 备注 |
|---|---|---|
| Kraid(2) | **0px**（6/6） | `id=room_ptr` 命中 `A345` |
| Tourian(3) | **0px**（6/6） | |
| Ridley(4) | **0px**（6/6） | |

五房冒烟（rooms 0–4）：Tourian **5/5 ok**、Ridley **5/5 ok**、Kraid 3 ok / 2 blocked——blocked 的真因见 §8.5。

## 8.5 敌帧表复合段的 **bank→key 错配**（2026-11-23，三区扩展当场抓到的真 bug）

现象：Kraid room 3/4 在 `prev60/f60/prevend/f120` 全 `blocked`，gap 为
`unported @ bank07:E0BC-wrap idx=35 reset=35 len=38 x=40 type=05 status=02`。

判读：`Anim::step` 的 `Broken{len}` 是**实际传入表的项数**。Kraid 的 `EnAnimTable` 应有 **113** 项，
而报错说 38 —— 38 是 **Tourian** 的表长（见 `docs/findings/per-bank-assets.md` 五区基址表）。

取证（包内段头，`Entry 20B`，复合段头 6×u32 = anim/fptr/place/frames/places/…）：

| 段 | 修前 len / 头 | 修后 len / 头 |
|---|---|---|
| `KrEnSprites`(162) | 3305 / `(38,138,16,…)` ← **Tourian 的** | 3694 / `(113,151,16,…)` |
| `ToEnSprites`(163) | 3694 / `(113,151,16,…)` ← **Kraid 的** | 3305 / `(38,138,16,…)` |

根因（`crates/game-data/src/extract.rs::extract_enemy_sprites_all`）：`ENEMY_SPRITES` 按 **bank 升序**
（`bank1/2/3/4/5`）排列，而键表写成 `KEYS[i]` 用**枚举下标**取键，键表却是 **InArea 序**
（`[Br,No,Kr,To,Ri]`）。Kraid=bank4、Tourian=bank3（`BankInitTable ; 07:CA30 = 02 03 05 04 06` 即
InArea 1..5 → bank 2/3/5/4/6）⇒ bank3 的表写进了 `KrEnSprites`、bank4 的写进了 `ToEnSprites`。

修法：新增 `KEYS_BY_BANK = [Br, No, To, Kr, Ri]`（它就是 bank1..5 序），按 `spec.bank - 1` 取键。

意义：**这是同一类「area 序 ≠ bank 序」错误在本抽取器的第二次出现**（第一次是
`extract_enemy_tables_all` 的迭代序，见 `docs/findings/per-bank-assets.md`）。两次都只在
「该区第一次被真正观测」时暴露 —— 这正是补三区分母的直接回报：Norfair 44/46 全绿时这个 bug 完全隐形。

## 8.6 加速径 X 的两处真 bug（2026-11-23，Tourian 房扫当场抓到）

Tourian 21 房里 8 房（6/7/8/10/11/12/19/20）在四个判定采点全 `blocked`，rust 腿报
`attempt to add with overflow`。给 `rust_screenshot` 的 panic 钩子补 `info.location()` 后一次定位到
`crates/game-core/src/enemy/motion.rs:794`（`enemy_get_delta_x_using_acceleration`）：

```
let t = sub as u16 + (accel as i8 as i16 as u16);
```

`accelX` 为负（`$FC`）时符号扩成 `$FFFC`，与 `sub` 相加越过 `u16` 上限 ⇒ debug 构建 panic、
整案 blocked。asm 这条加法本就是 **16 位二补数相加、进位出界丢弃** ⇒ 改 `wrapping_add`。

顺着同一条链又抓到**第二处（行为错，不只是 panic）**：

```
let mut sp = (speed_x as i16).wrapping_add((t >> 8) as i16);   // 旧形
```

高字节 `$FF` 被当成 **+255** 加进 `speedX`（asm 负加速径是 `adc #$FF` = −1）⇒ 负加速敌人被算成
`+0x0E` 并被 ±0x0E 钳位。改为 `(t >> 8) as u8 as i8 as i16`。

锚测（`crates/game-core/src/enemy/motion.rs::tests_accel_x_overflow`）：

- `negative_accel_x_does_not_overflow`：`accel=$FC, sub=05` ⇒ `subPixelX 01`、`speedX 00`（不 panic）；
- `negative_accel_x_borrows_into_speed_x`：`accel=$FC, sub=00` ⇒ `subPixelX=$FC`、`speedX=$FF`。

修前第二条实测 `speed_subpixel_x()==0`（被钳位成 +0x0E 后清零）——据此定谳旧形行为错。

## 8.7 三区房间的「滚动期掉帧」节奏（2026-11-23 实测，未修）

### 现象族
`data/status/room_sweep_{2,3,4}.json` 里 7 房（Kraid 29/31、Tourian 6/10、Ridley 29/34/37/38）
在 `layer=nametable`、`shift.dx=4`、`explains=False`、15–18K px。同帧 `SamusX` 两侧相同，
`ScrollX` 差 = 像素偏移量（Kraid 31：emu `E8` / rust `EE`；numpy roll 搜索显示真偏移 **6px**，
dx=+6 时残差 920px，其余 dx 均 ≥8k）⇒ **是相机领先，不是瓦片写错**（`nt_norm` 仅 12 B）。

### 决定性读数（`watch 002D FC` 逐帧，entry=1600，载入后 ScrollX 每帧 +2）

| 区/房 | 载入窗冻结 | 滚动期丢帧（前 5 个） | 节奏 |
|---|---|---|---|
| a2 r31 (Kraid) | 1600,1601 | 1704, 1709, 1714, 1719, 1724 | 每 5 帧 |
| a4 r37 (Ridley) | 1600,1601,1602 | 1701, 1706, 1711, 1716, 1721 | 每 5 帧 |
| a3 r10 (Tourian) | 1600,1601,1602 | 1694, 1707, 1712, 1717, 1722, 1727 | 每 5 帧 |
| a0 r05 (Brinstar, 对照) | — | **无** | 不丢帧 |

对应 `dFC>0 且 dscroll ≈ dFC × 1.5`：a2 r31 160/164 (+6)、a4 r37 159/164 (+10)、
a3 r10 159/164 (+10)、a4 r34 156/164 (+16)、a2 r29 152/163 (+20)。

### 机理（推断，未定谳）
滚动期每帧工作量 ≈ 1.2 × `FRAME_CYCLES`(29780) ⇒ 每 5 帧累积出一整帧 ⇒ 丢一帧且该帧
`ScrollX` 不推进（实测 1705 帧 emu `scr` 与 1704 相同）。rust 只在**载入窗**记 `work_cycles`
（`LOAD_BASE_CYCLES` + 各 `LOAD_*_CYCLES`），滚动/绘制期不记 ⇒ 永不丢帧。
Brinstar 不丢帧是线索：要么对象/瓦片更少，要么 bank1 的滚动路径更轻。

### 实现前提（下一轮）
1. 把 `work_cycles` 从「仅载入窗」扩到**每帧**（`ScrollLeft/Right` + 列流 + 对象更新计费），
   累计 ≥ `FRAME_CYCLES` 时丢一帧并扣减。
2. 计费常量先用**单一标定量**（滚动列成本），以本表 4 房为标定点（每 5 帧 1 次、Brinstar 0 次）；
   契约写明这是 C 级拟合 + 可证伪判据（其它区换入后节奏仍须是 5 帧）。
3. 验收红线：本族 7 房 `dFC==0` 且像素归零；**Brinstar 47 房不得出现新丢帧**。

### 探针复现
`tools.movie_ref.room_sweep.build_case(area, room, 130, [60,120])` + 插 `watch 002D FC`
+ `name` 改成**非** `_tmp_sweep_` 前缀（否则产物被房扫清理）⇒ `tools/l4_diff_run.py run <name>`；
emu 逐帧 FC 读 `analysis/traces/<name>.frames.jsonl`，rust 侧 `--trace fc`（其数值是**十进制**）。
另外：`build_case` 的 `capture 1600 pre` 与 `f0`/`prev1` 撞帧会报「缺模拟器截图」⇒ 探针里删掉 pre 行。
