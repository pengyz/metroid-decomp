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
  像素都要挂起、窗末一起落。所以「清标位」只能发生在装载窗排空那一拍，清早了则
  `|delta|>=2` 时第 2..n 个像素当拍落地（这正是复核 2026-11-19 P1-A 抓到的漏洞，已修）。
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
- `ScrollX==0` 的跨格迁移径（`GetRoomNum` / `MapPosX±1` / `@atRightBound` 回滚）已由
  `scroll_right_wraps_scroll_x_and_toggles_name_table` 恢复覆盖，但 `GetRoomNum` 的分支
  仍无断言。
