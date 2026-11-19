# 载房窗内的水平像素写点（f673 卡）

**结论**：`MoveSamusLeft/Right` 的像素写点（`dec/inc Samus.x`）在 asm 里排在
`jsr ScrollLeft/Right` **之后**；而 `ScrollLeft/Right` 内部的 `CheckUpdateNameTable`
首条指令就是 `jsr SetupRoom`，后者会**跑过帧边界**。因此「载房窗吞帧」的那一拍，
这次像素写不会落地，而是落到装载窗结束的那一帧。Rust 旧形把 `setup_room` 当同步调用、
当帧就 `set_x(x∓1)`，只靠 `iter_bogged` 跳后续迭代 ⇒ 像素提前 3 帧生效。

## 证据（A 级：asm 行 + ROM 字节）

| 事实 | 位置 | ROM 字节（bank7 定窗，file = 16 + 7·0x4000 + addr − 0xC000） |
|---|---|---|
| `MoveSamusLeft` 入口 | `prg7_engine.asm:7864`（`07:E626`） | — |
| `jsr ScrollLeft`（在像素写之前） | `prg7_engine.asm:7884` | `$E6C7` = `C6 FD 20 4A E5 18 60`（`dec ScrollX / jsr $E54A / clc / rts`） |
| 像素写 `Lx180: dec Samus.x` | `prg7_engine.asm:7895-7896`（`07:E65E`） | `0x1E66E` = `CE 0E 03 38 60` |
| 右移像素写 `inc Samus.x` | `prg7_engine.asm:7933`（`07:E692`） | — |
| `CheckUpdateNameTable`（首条 `jsr SetupRoom`） | `prg7_engine.asm:7702`（`07:E54A`） | `0x1E55A` = `20 2B EA` |
| `UpdateNameTable` | `prg7_engine.asm:7767`（`07:E590`） | — |
| `ScrollLeft` | `prg7_engine.asm:7951`（`07:E6A7`） | — |
| `SetupRoom` | `prg7_engine.asm:8693`（`07:EA2B`） | `0x1EA3B` = `A5 5A C9 FF F0 F9` |
| `RoomNumber=$FF` = 无请求 | `constants_ram.asm:589` | — |

## 逐帧实录（Mesen 指令级探针 + 两侧 dump）

用例 `cases/case-mechanism-input-conflict.txt`（inputlog，采点 661..700）：

| 帧 | emu x / FC / RoomNumber | rust（修后）x / FC / RoomNumber | 判读 |
|---|---|---|---|
| f672 | `70` / 143 / `FF` | `70` / 143 / `FF` | 同 |
| f673 | `70` / 143 / `08` | `70` / **144** / **`F0`** | 像素都不落；**载房相位仍不同** |
| f674 | `70` / 143 / `08` | `70` / 144 / `F0` | 同上 |
| f676 | `6F` / 144 / `F0` | `6F` / 144 / `F0` | 三字段同（像素在此落地） |
| f678 / f680 / f700 | x / FC / Room 全同 | 同 | 残差只剩 nametable 面 |

Mesen 探针（`E31A / ML / DECx(07:E65E) / DDECx / UPDNT / SETUP`）逐帧计数：
f672 `DECx=2 SETUP=2`；**f673 `DECx=0 SETUP=1`**（进例程、像素不落地）；f674/f675 全 0
（纯 lag）；**f676 `DECx=1`**（例程恢复、落地）。

## 修法

- `SamusState` 增两个**非 RAM** 相位字段：`load_consumed_frame`、`deferred_horz_px: i16`
  （带符号**累计**）。累计的**真实动机**：`MoveSamusLeft/Right` 在 asm 里是**逐像素循环**
  （`SamusMoveHorizontally` 每 1 px 调一次），而装载窗把整个循环一起冻住 ⇒ 窗内**每一个**
  像素都要挂起、窗末一起落。所以「清标位」只能发生在装载窗排空那一拍。

  **阈值实测更正（复核 2026-11-19 第二轮，`|delta|>=2` 的说法被实验证伪）**：把
  `defer_if_load_consumed` 回填成旧形（第一次即清标位）后，`|delta|=2` 仍然绿——因为窗内
  每像素循环会再走一次 `check_update_name_table`，`is_load` 仍真时把标位重新置起；
  **只有 `|delta|>=3` 才红**。因此旧形的真实后果是「一窗内第 3 个及以后的像素当拍落地」，
  不是「第 2 个」。判别测 `three_pixel_move_defers_three_pixels` 钉的就是这个阈值。
- `check_update_name_table` 的 `is_load` 分支置标位；`move_samus_left/right` 在 Scroll
  之后、`set_x` 之前把 ±1 累加进 `deferred_horz_px` 并 return。
- `Engine::end_of_tick_gate` 在迭代门队列**排空**那一拍兑现（`debug_assert` 钉住：队列非空
  时不许兑现）。

**证据分级**：顺序事实（像素写在滚动之后）是 A 级；「兑现点 = 门队列排空那一拍」是
**B 级模型推断**——asm 只给顺序，没给「窗末是第几拍」。该时点是从既有掉帧模型推出来的
（`dropped = (LOAD_BASE_CYCLES + work_cycles) / FRAME_CYCLES` 个 `iter_gates`，一拍排 1 项），
该公式本次未改，不是为凑 f676 调的。

## L4 影响面（实测，逐格）

state 级不变：**20 案 / 160 格 / match 148 · diff 12 · blocked 0**。
`diff_pixels` 有 3 格变动（基线 → 本发）：

| 格 | 基线 | 本发 | 变化 |
|---|---|---|---|
| `case-mechanism-input-conflict` `674:c674` | 326 | **330** | +4（恶化） |
| `case-mechanism-input-conflict` `676:c676` | 326 | **330** | +4（恶化） |
| `case-scrollseam-probe` `1100:scroll-r180` | 274 | **266** | −8（改善） |

**这不是「零回归」**：像素数确实动了两格，方向是变差。归因：deferral 把像素写从 f673 挪到
f676（与 emu 的 x 相位对齐），于是 f673/f674 的 Samus 少差分 1 px、f676 多 1 px，在「载房
相位本来就不同」的窗口里表现为 +4/−8 的此消彼长。真正的残差主体是未修的那张卡（见下）。

## 两条曾被踩错的措辞（原地保留，供后续别再踩）

1. **「单槽会被同窗第二拍覆盖」是不成立的动机**：`engine.rs:144-152` 的 span tick 不跑移动，
   同一窗内「第二拍再置位」在真实引擎不可达（只有测试手工置位才能构造）。真正的动机是
   上面那条「逐像素循环被整窗冻住」。
2. **「`$004C/$004D` 在 Rust 侧无消费者」只对 `$004C` 成立**：`$004D` 就是 `SamusDir`
   （`ram_map.rs:227 SAMUS_DIR = 0x004D`），消费点 15+ 处（`samus/handler.rs:166/279/327…`、
   `samus/physics.rs:247/980…`）。`nes_map.json` 该 undeclared 段的 refs 已同时列
   `$004D`（ram_map + handler.rs:166）与 `$004E`（ram_map + physics.rs:1761）。

## 未修（已登记，另开卡）

- **多帧载房任务机**：emu 的 `RoomNumber` 是装载游标（`08 → F1 → F2 → F4 → FF`），期间
  `RoomRAMWorkPtr` 在写、FC 冻结；Rust 把整段压成单帧（f673 rust FC=144/Room=`F0`，
  emu FC=143/Room=`08`）。
- f676 之后仍有 nametable 面残差（f678 16 B / f700 32 B）。
- **快照相位丢失**：`Manager::restore_mss` 用 `Self::default()` 重建 ⇒ 窗中打快照/恢复会丢掉
  `deferred_horz_px` 与标位（x 少 n px）。当前 20 案未触发。
- **`dropped==0` 的同 tick 推迟**：`LOAD_BASE_CYCLES=29000 < FRAME_CYCLES=29780` ⇒ 存在
  「`is_load` 为真但窗为空」的拍，像素被推到 tick 末兑现（同 tick 内 x 陈旧，幅度从 1 px 放大到 n px）。
- **兑现基准**：兑现是把增量加到「tick 末的 x」上；若同 tick 后续有 `set_x`（门传送等），偏移会
  落到新基准上。未证可达。
- `ScrollX==0` 的跨格迁移径（`GetRoomNum` / `MapPosX±1` / `@atRightBound` 回滚）已由
  `scroll_right_wraps_scroll_x_and_toggles_name_table` 恢复覆盖，但 `GetRoomNum` 的分支
  仍无断言。

## 多帧载房任务机（A-lite，2026-11-19 落地）

### 取证（Mesen 逐帧 `$2D/$5A/$33-$34`，用例由 `tools/movie_ref/room_sweep.py::build_case` 生成，f900 注入 `$005A=<room>`）

| room | 冻结帧（FC 不动） | 冻结期 RoomNumber | 帧边界尾段 | 注入→`FF` |
|---|---|---|---|---|
| 0 | f900–f901 | 恒 `0` | f902=`F1` / f903=`F3` / f904=`FF` | 4 帧 |
| 23 | f900–f902 | 恒 `23` | f903=`F1` / f904=`F3` / f905=`FF` | 5 帧 |
| 8 | f900–f903 | 恒 `8` | f904=`F1` / f905=`F3` / f906=`FF` | 6 帧 |

room8 冻结期 `RoomPtr` 逐帧 `A58E → A5A0 → A5B2`（末尾 A5B8）⇒ 画房循环被切在帧边界。
三条结论：①冻结帧数 == 既有 `dropped`（room23=2 / room8=3 / room0=1，与 `LOAD_BASE_CYCLES`
标定一致 ⇒ **不存在与掉帧模型重复计算**，`dropped` 就是任务机的帧数）②冻结期 `RoomNumber`
保持**要载的房号**（不是 `$F0`）③尾段固定三格 `F1 → F3 → FF`（`F0` 是空转 `ExitSub`，
`F2/F4` 在同拍内被跨过，故帧边界看不到）。

### 实现

- `Manager` 三个非 RAM 相位字段：`room_load_phase`（0 空闲 / 1 冻结窗 / 2 尾段）、
  `room_load_room`、`room_load_tail`（`state.rs`）。
- `world/room_load.rs::is_new_load_request`（**对抗复核 P1 修正**）：判据 = 房号 `< $F0` 且
  （任务机空闲 **或** 房号 ≠ 在装房号）。窗内同房号不算新请求（不重装）；尾段游标 `$F0..$FE`
  不满足 `< $F0`（尾段由 `room_load_gate_step` 独占驱动，避免与主循环双驱动）；尾段期间真来的
  新请求（`GetRoomNum ; 07:E808` 重写 `$5A`／门传送／电梯）**抢占**旧任务且照常记账掉帧窗
  （旧形用 `phase == 0` 单条件会把这次装载的 `dropped`/`work_cycles` 静默丢掉）。
- `world/room_load.rs::begin_room_load_window` / `room_load_gate_step` / `advance_room_load_tail`：
  窗由 `iter_gates` 计数（`dropped` 帧），帧门排空那一拍切进尾段，尾段每拍推进一格到目标值
  （`F1` / `F3` / `FF`），格内用 while 循环调到目标值；**硬上界 3 用尽仍不达目标 ⇒
  `unported!("bank07:EA13-tail-cursor")` fail-loud**（对抗复核 P2：原 `debug_assert` 在 release
  被编译掉，会把游标走成一个看似合法的房号，下一帧再整房重载）。
- 掉帧窗长度取 **`.max(1)`**（复核 P2）：`LOAD_BASE_CYCLES=29000 < FRAME_CYCLES=29780` 时公式
  得 0，而 Mesen 三房实测最小冻结帧数 = 1（`room_draw.rs:1493` 的测试也自证 BASE/FRAME==0）。
- `frame/memory.rs::restore_mss` 从 `$005A` **重建窗相位**（复核 P2）：窗内 RAM 里就是房号
  （`< $F0 && != $FF`）⇒ 恢复后置相位 1，避免把同一房间装载第二遍。精度边界：剩余窗长
  （`iter_gates`）本身不在 MSS 面，只能恢复相位、不能恢复剩余拍数。
- `world/camera/scroll.rs`：`is_load` 加相位闸 `&& room_load_phase == 0`（否则窗内每个像素
  都会重触发整房装载）；装载后 `begin_room_load_window` + 把 `RoomNumber` 改回房号。
- `frame/engine.rs::end_of_tick_gate`：门队列排空分支调用 `room_load_gate_step`。

### 效果（L4）

`match 148→151 / diff 12→9`：`case-morphball-probe` 的 `2080/2110/2160` 三格 **8px → 0px**
（即此前挂账的「morphball 3×8px」债），其余 157 格逐格不变、零回归。

### 仍未建模（已登记）

`draw_room` 的 `RoomPtr` 分块：emu 冻结期每帧前进一截，Rust 仍一次跑完（`RoomPtr` 冻结期不动、
窗末即为终值）。像素面目前不受影响（上表 L4 结果），但这是任务机里唯一还压平的部分。
