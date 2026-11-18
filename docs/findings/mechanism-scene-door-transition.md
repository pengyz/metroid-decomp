# 机制场景：600 帧「滚动房号查找 + SetupRoom 载入链」（**不是**门过渡）

> 【2026-11-18 更正】本文件初版标题写「门过渡机制场景」并把覆盖台账 `door-transition`
> 升为 verified。独立复核（`docs/rounds/r9-task-door-transition-review.md`）证明该结论错：
> 窗口内 `SamusEnterDoor($8B13)` 根本没跑过。已撤回升级，文件名保留以便追溯，正文按实测重写。

## 0. 场景与结论

- 场景：`cases/case-mechanism-door-transition.txt`，600 帧，输入 = `movies/eien86-6725M.inputlog`
  前 600 帧（作者真实操作：标题 → 进游戏 → 右行 → 触边滚屏 → 邻房载入 → 邻房渲染）。
- 机制：`GetRoomNum / LE733`（`refs/m1disasm/SRC/prg7_engine.asm:8049,8062-8095`，
  `$005A ← WorldMapRAM[MapY*32+MapX]`）+ `SetupRoom ; 07:EA2B` + `CopyAreaPointers ; 07:C8B0`
  载入链。
- 机器报告：`data/status/mechanism_scene_door_transition.json`。
- 覆盖台账：本场景作为 **`room-load-chain`** 的补充证据，该条目**仍是 `route-only`**。

## 1. 为什么用录像输入

开场 440 帧演出吞输入（`SamusInit ; 07:C8D1` 置 `Timer3=$2C`，`UpdateTimer ; 07:C266` 每 10 帧减 1），
自造按键（`press 100 start` 之类）会落在标题阶梯 `$14..$18` 上——变形球场景第一版就栽在这。
本场景直接用录像前 600 帧，起手即作者操作。注意：inputlog 的 Start 在第 14/18 行，且落在吞输入窗内
（`tools/scenario_run.py` 的 lint 只是 WARN 级提示，别把它当「菜单一定到位」的证据）。

## 2. 观测到的时间线（mesen 轴，逐帧）

| 帧 | `$005A RoomNumber` | 含义 |
|---|---|---|
| 24 | `09` | 游戏内房号 |
| 26 | `FF` | 稳定态（无载入请求） |
| 482 | `17` | 房号查找写入目标房 |
| 484 / 485 / 486 | `F1` / `F2` / `F4` | 载入链阶段计数（`prg7_engine.asm:8673` 起 `inc RoomNumber`） |
| 487 | `FF` | 载入完成 |

rust 侧行 `484=F1 / 485=F2 / 486=F4 / 487=255` 与 mesen 一致；**482/483 不在观察面**
（房间载入期是 iteration span，不产状态行，见 `crates/game-core/src/frame/engine.rs:107-120`）。

## 3. 证据（能说什么、不能说什么）

**能说**：
- 像素面：`l4_diff_run.py run case-mechanism-door-transition` ⇒ 6/6 全 0 px
  （470/481/486/500/560/600；486 在载入链中段、500/560/600 在邻房）。
- 状态面：`mechanism_scene.py --case … --frames 600` ⇒ `OFFSET +0`、`compared=1168`、
  `mismatch=0`、`incomparable=16`。

**不能说**：
- 不能说「门过渡已验证」：窗口内 `$0056/$0058/$0059/$004E` 六采点全 `00`、status 从不出现 5
  （`SamusEnterDoor` 的三件签名：`$59←$12`、`$56|=$80`、status←5，
  `areas_common.asm:1385-1398`、`:1410-1413`）。
- 不能说「决定性写点两侧一致」：`$5A=$17` 只在 482/483，而这两帧不在任何观察面内；
  报告里的 `distinct{room:5}` 实为 `{00,FF,F1,F2,F4}`，**不含 0x17**。
- 不能说「载入链中间值有意义」：`F1/F2/F4` 是阶段计数，本仓已裁定加载链中间值的帧级比较无意义
  （`docs/findings/movie-6725m-residual-episodes.md:66-69`）。

## 4. 对照强弱记账

- **机制级**：单测 `samus_enter_door_not_triggered_without_door_data`
  （`crates/game-core/src/samus/physics.rs`）钉住 asm `areas_common.asm:1322-1325` 的
  「`DoorEntryStatus==0` 且 `SamusDoorData!=0` 才触发」判据。它证明**函数契约**，
  **不能**证明本场景里发生过门事件——本场景确实没发生。
- **场景级（弱）**：`cases/case-mechanism-door-negative.txt` 零输入 ⇒ 三采点 `room=00/status=00`
  （机器停在标题屏）。只能证明「没输入就不动」，对「门过渡需不需要进门动作」零判别力。

## 5. 局限 / 后续（未做）

1. 要重提 `door-transition`，必须补一个**真实门窗口**：status 出现 5 且 `$59=$12`、`$56≠0`，
   并把 `$0056/$0058/$0059` 加进 `tools/movie_ref/align_legs.py::FIELD_ADDRS` 与
   `mechanism_scene.py::STATE_FIELDS`（现在只有 status/room/health/scrolldir，结构上记不了门寄存器）。
2. `room-load-chain` 要升 verified，需先解决「决定性写点 482/483 不在观察面」——
   要么让采样面覆盖 iteration span，要么换一个写点落在可比帧上的窗口。
3. 采集点稀疏（6 点）不足以覆盖载入链每一拍。
