# 机制场景：门过渡（door transition）+ 房间载入链

- 用例：`cases/case-mechanism-door-transition.txt`（600 帧，输入=6725M any% 录像前 600 帧）
- 负对照：`cases/case-mechanism-door-negative.txt`（同窗口、零输入）
- 机器报告：`data/status/mechanism_scene_door_transition.json`
- 台账：`data/status/coverage_ledger.json` 的 `door-transition`（route-only → verified）

## 1. 为什么用录像输入而不是自造按键

本作开场有 440 帧演出吞输入（`SamusInit ; 07:C8D1` 置 `Timer3=$2C`，`UpdateTimer ; 07:C266`
每 10 帧减 1），自造路线很容易在"还没进游戏"的阶段空转——变形球机制场景第一版就栽在
`press 100/140 start` 上（当时 status 停在标题阶梯 `$14..$18`、`RoomNumber` 恒 `$FF`）。
门过渡需要"进游戏 → 走到门 → 进门"三段连续成立，录像前 600 帧正好覆盖，且是作者真实操作。

## 2. 机制时间线（两侧逐帧一致）

`RoomNumber`（`$005A`，`constants_ram.asm:589` `#$FF = no room requested`）的写入链：

| 帧 | 值 | 含义 |
|---|---|---|
| 482 | `$17`（23） | 写入目标房（进门的瞬间） |
| 484 | `$F1`（241） | 载入链第 1 步 |
| 485 | `$F2`（242） | 第 2 步 |
| 486 | `$F4`（244） | 第 3 步 |
| 487 | `$FF` | 载入完成、清位 |

Rust 侧逐帧同值（482/483 属 `incomparable` 的 16 帧集合——那是 Rust 侧 iteration span 不产状态行的帧，
不是分歧）。载入链之后房间已经是新房间（`f560` 起像素与状态面全同）。

## 3. 证据

- **像素面**：`tools/l4_diff_run.py run case-mechanism-door-transition` ⇒ **6/6 全 0 px**
  （470 approach / 481 door-trigger / 486 load-chain / 500 transition / 560 in-room / 600 end）。
- **状态面**：`tools/movie_ref/mechanism_scene.py --case … --frames 600` ⇒
  `OFFSET +0（anchor=status）`、`compared=1168`（584 可比帧 ×2）、`mismatch=0`、
  `incomparable=16`、`informative=true`、`distinct{status:8, room:5}`。
  即 **整个窗口内两侧 `status`/`room` 逐帧相同**，不是只对上了采集点。

## 4. 负对照（同窗口、零输入）

`cases/case-mechanism-door-negative.txt` 不给任何输入 ⇒ `RoomNumber` 全程常量，
`mechanism_scene.py` 报 `insufficient anchor changes (mesen=600 rust=596 changes=0/0)` 无法对齐。
这正是对照要的结果：**房间写点不是自主/环境触发，而是输入驱动**。

这条对照证明的是"输入驱动"，**不是**"路线唯一"——要证明后者需要第二条进房路线（未做）。

## 5. 局限（如实记账）

1. 只覆盖**一条门**（目标房 `$17`）；蓝门/红门/导弹门与竖门未在本场景内区分。
2. 窗口含 470 帧前置（标题 + 行走），是**能跑通的最短录像片段**，不是理论上最小帧数。
3. 像素面只取 6 个采集点；状态面覆盖 584 帧。
4. 末态 `doorEntry`（`$005B`）在两侧的取值随相位变化，本场景只用 `status`/`room` 做判据
   （`DoorEntryStatus` 的语义锚见 `SamusEnterDoor`）。

## 6. asm 锚

- `SamusEnterDoor ; $8B13`（`refs/m1disasm/SRC/areas_common.asm:1317`）：判据=`DoorEntryStatus`
  为 0 且 `SamusDoorData` 非 0 ⇒ 刚进门；随后重置导弹/能量拾取计数、按 `PPUCTRL_ZP` 位擦名表门数据、
  按 `ScrollDir & $02` 分横/纵滚。
- `SetupRoom ; 07:EA2B` / `CopyAreaPointers ; 07:C8B0`：载入链入口（见
  `docs/findings/per-bank-assets.md` 的按区指针表）。
- `RoomNumber` 的 `$FF = no room requested` 语义：`constants_ram.asm:589`。

## 7. 顺带修的工具问题

`tools/scenario_run.py` 的长场景 lint 原来只数 `press ... start`，用 `inputlog` 输入的用例会被误
WARN（"疑菜单误删"）。扩口径为"**展开后的前 700 帧里至少有一次 Start 位**"（press 与 inputlog
同一判据，仍机械可判），并补 `tools/tests/test_scenario_lint.py` 三条测试（press 路径 / inputlog
路径 / 零输入仍 WARN）。
