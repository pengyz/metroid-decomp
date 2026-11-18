# 变形球机制最小重现（mechanism scene）

> 目的：把「变形球进入 → 球内滚行 → 出球」从长片录像里拆出来，做成 600 帧以内的机制场景，
> 两腿（Mesen2 / Rust）同场景对拍，用于把覆盖台账 `morphball-unroll` 从 `route-only`
> 推进到 `verified`。
>
> 结论一句话：**机制窗（f476..570）两腿状态面 0 失配、像素面 0 px**；机制特异信号是
> `SamusStatus=$03`（Roll）只在注入能力后出现（去 fixture 对照：从不出现）。

## 1. 场景窗口与被覆盖的状态链

场景 `cases/case-mechanism-morphball.txt` 的**声明窗**是 f0..570（`frames 570`、`endFrame 570`）。

| 阶段 | SamusStatus ($0300) | 观测点（两腿相同） |
|---|---|---|
| 标题演出 | `$14`→`$18` | f36/f176/f276/f366/f436 阶梯 |
| 进游戏 | `$FF`→`$00` | f476 MainRoutine 8→3，status 一度 `$FF`，f501 归 `$00` |
| 入球（Roll） | `$03` | **f506** 起，持续到 f562 |
| 球内滚行 | `$03` | f527..532 `RoomNumber($005A)` 链 `14→F1→F2→F4→FF` |
| 站立且 Up 仍按 | `$04` | **f563** 起（`SamusPntUp ; 07:D198` 落点态） |
| 回站立 | `$00` | **f570** 起 |

注：`$005A` 链是**门转场公共面**，不是变形球特异证据——去 fixture 对照里同样出现
（f528..532 `20→F1→F2→F4→FF`，此时 status=1 走路）。同理 status=`$04` 在无变形球时
也会出现（站着按 Up）。**唯一机制特异信号是 status=`$03`**。

## 2. asm 锚（A 级）

- 能力位：`refs/m1disasm/SRC/constants.asm:74` `gr_MARUMARI = %00010000`；
  入球闸门读 `SAMUS_GEAR($6878) & 0x10`（`crates/game-core/src/samus/handler.rs:1262`），
  地址常量 `crates/game-core/src/ram_map.rs:111` `SAMUS_GEAR: u16 = 0x6878`。
- 分派表：`GoSamusHandler ; 07:CC1A` 的 10 态表里 index 3=`SamusRoll`（Rolling）、
  index 4=`SamusPntUp`（Pointing up）（`refs/m1disasm/SRC/prg7_engine.asm:2725-2726`）。
- 例程：`SamusRoll: ; 07:D0E1`（`prg7_engine.asm:3618`）、`SamusPntUp: ; 07:D198`（`:3737`）。
  **退球（unroll）代码在 `SamusRoll` 的 `Lx031`**（`prg7_engine.asm:3625` 起 "break out of ball mode"
  → `SetSamusStand ; 07:CED1=:3369`，因 Up 仍按而落 `SetSamusPntUp`）——
  `SamusPntUp` 不是出球例程，是「已站起且 Up 仍按」的落点态。
- 房间加载瞬态：`refs/m1disasm/SRC/constants_ram.asm:589`
  `RoomNumber db ;$5A ;Room number currently being loaded. #$FF=no room requested.`

## 3. 时间基（实测，不假设）

- `movies/eien86-6725M.inputlog` 起手是 **f22 / f26 两次 Start**（`$10`），**f237** 才第一次左移。
- `analysis/traces/_probe-axis.frames.jsonl`：`MainRoutine($1E)` 在 f0..470 恒为 8（SamusIntro，
  `Timer3=$2C` 每 10 帧减 1），**f480 起为 3**（GameEngine）⇒ 可操作点 f≈480。
- 故动作窗安排在 f500..570：入球 f506、滚行 f512..556、出球 f562..568。

## 4. 场景与命令

```
python3 tools/movie_ref/mechanism_scene.py \
  --case cases/case-mechanism-morphball.txt --frames 570 \
  --json-out data/status/mechanism_scene_morphball.json

python3 tools/l4_diff_run.py run case-mechanism-morphball
```

机器输出（`data/status/mechanism_scene_morphball.json`，f0..570）：

- 对齐：`OFFSET +0`，anchor=`status`（状态面因果锚，latch 只作旁证）
- `compared=1108`（= 554 个可比帧 × 2 字段：status/room）
- `mismatch=0`、`incomparable=16`（**帧数**：rust 侧缺行 f2, f4..6, f23..25, f28..34, f527, f528）
- `distinct_values{status: 9, room: 5}`
- `RESULT: all-match`

像素面（`shots_rust/case-mechanism-morphball.l4report.json`）：

- f476 `pre` = **0 px**、f570 `ball-exit` = **0 px**（共 61440 像素/点）⇒ 声明窗内 **Rust 同场景 0 px**。

## 5. fixture 是前提，不是被判定字段

`fixture 470 6878 10` 注入 `SamusGear($6878)` 的 `gr_MARUMARI` 位（**能力前提**），
判定面只有 `status($0300)` 与 `room($005A)`。两腿同语义：emu 侧逐条
`fixtureAppliedAt[]`（`tools/scenario_capture.lua:1194-1213`，`frame >= fx.frame` 生效一次、
采样前调用），rust 侧逐条 `fixture_applied`（`crates/game-core/src/bin/rust_screenshot.rs:493-502`，
采集发射与 `tick_frame` 之前），注入点 f470 早于动作窗。

**负对照（关键，不能只靠 informative）**：把 `fixture` 行删掉重跑，结果仍是
`informative=true`、`all-match`（`compared=1108`），但**整场没有一次 status=`$03`**
（f513 起是 status=1 走路）。原因：`informative` 只判「某字段取值数 ≥2」
（`tools/movie_ref/mechanism_scene.py:115-118`），而 status 的 9 个取值里 6 个来自
**fixture 之前的标题演出**（`$14..$18`），room 的 5 个取值在无变形球时也照样出现。

⇒ 结论：`informative` 只是「非常量窗」护栏，**不是机制是否发生的判据**；
机制活性必须由 **status=`$03` 的出现/缺失**来判定。这也说明本场景真正的判别力来自
fixture 注入与状态的因果对应，而不是 runner 的常量窗检查。

## 6. 局限

1. 本场景**不重现** 6725M 长片 f1030..1080 的分歧；那段已判定为 quickerNES↔Mesen2 核心差
   （`docs/findings/movie-6725m-f1031-unroll.md` §9）。本场景只证明**状态机行为一致**。
2. 只走右侧滚行一条链；球内受伤、球态下钻管道（`SamusPntUp` 另一分支）、左滚未覆盖。
3. 只对拍 status/room 两个状态字段；Samus 位置、动画索引、球体半径未纳入判定面。
4. 帧轴对齐依赖状态面跳变（本场景 status 跳变 5 次、latch 跳变 ≥3 次）；若将来改输入
   导致跳变不足，runner 会 fail-loud（`change_runs < 3`）。

## 7. 声明窗之外：f571+ 的 Samus 精灵面分歧（未修，已留探针）

声明窗在 f570 收尾，但同一输入继续跑下去会暴露一处**新的真分歧**：

- 探针 `cases/case-morphball-tail-probe.txt`（同输入，`frames 600`，采集 576/588/600）
  L4 结果：**145 px / 61440**，三采集点恒定。
- 与 fixture 无关：删掉 `fixture` 行的变体为 **146 px**（同区域、同形态）。
- 差异区域 bbox `x=136..154, y=155..176`（Samus 精灵块）；OAM 影子槽 8..13（emu）
  逐字节不同：emu `(y,x,tile,attr)` = `(160,143,$14,$20)`、`(160,135,$14,$20)`、
  `(168,30→…,),…`；rust 同区域槽位多出 3 个候选且 tile 为 `$69` 等。
- 判读：状态面（status/room）两侧相同而精灵砖不同 ⇒ 属**动画索引或 Samus 图形/绘制路径**
  分歧，不是状态机分歧；已登记为待查（下一步工作项，未在本轮修复）。

## 8. 与台账的关系

- `data/status/coverage_ledger.json` 的 `morphball-unroll` 由 `route-only` 升 `verified`，
  evidence = 本文件 + `cases/case-mechanism-morphball.txt` + `data/status/mechanism_scene_morphball.json`
  + `shots_rust/case-mechanism-morphball.l4report.json`（新增 0 px 机器指针）+ 旧有 monball/f1031 三件。
- 门槛口径（`docs/superpowers/plans/2026-11-18-three-leg-oracle-alignment.md:234`）要求
  「机制级最小重现 + Rust 同场景 0 px」：本场景在**声明窗 f0..570** 内两采集点均 0 px，
  窗外的 f571+ 分歧用独立探针留证、不计入本条目（见 §7）。
- 基线经 `python3 tools/coverage_ledger.py --update --reason <text>` 抬高（只升不降），
  状态锚点走 `python3 gates/cli.py sync-anchors` 重签。
- 如实记账：基线的 `reason` 文本仍是首次升级时写的「600 帧 / compared=1168」口径；
  改成声明窗 570 后 `--update` 判 NO-OP（基线已是上界）不再写盘，故以本文件 §1/§4
  的口径为准，基线 `updated_at` 标记其历史性。
- 覆盖面仍窄：台账 3 条只有 1 条 verified；`door-transition`、`room-load-chain` 仍是
  `route-only`，按同样方法（声明窗 + 负对照 + 0 px 指针）逐条推进。
