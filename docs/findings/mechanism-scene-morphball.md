# 变形球机制最小重现（mechanism scene）

> 目的：把「变形球进入 → 球内滚行 → 出球」这一段从长片录像里拆出来，做成 600 帧内、
> 两腿（Mesen2 / Rust）可逐帧对拍的机制场景，用于把覆盖台账 `morphball-unroll`
> 从 `route-only` 推进到 `verified`。
>
> 结论一句话：**两腿在本场景逐帧全同**（compared=1168 / mismatch=0），且场景
> **informative**（status 9 个不同取值、room 5 个），因此它是有判别力的证据，
> 不是常量窗空对空。

## 1. 被覆盖的机制面

| 阶段 | SamusStatus | 观测点（两腿相同） |
|---|---|---|
| 标题演出 | `$14`→`$18` | f36/f176/f276/f366/f436 阶梯 |
| 进游戏 | `$FF`→`$00` | f476 MainRoutine 8→3 后 status 一度 `$FF`，f501 归 0 |
| 入球（Roll） | `$03` | **f506** 起，至出球前持续 |
| 球内滚行 | `$03` | f527..532 `RoomNumber($005A)` 链 `14→F1→F2→F4→FF`（滚行穿门后房间加载） |
| 出球（PntUp） | `$04` | **f563** 起（`SamusPntUp ; 07:D198` 入口态） |
| 回站立 | `$00` | **f570** 起至场景结束 |

## 2. asm 锚（A 级）

- 能力位：`refs/m1disasm/SRC/constants.asm:74` `gr_MARUMARI = %00010000`；
  本移植的入球闸门读 `SAMUS_GEAR($6878) & 0x10`（`crates/game-core/src/samus/handler.rs:1262`），
  地址常量 `crates/game-core/src/ram_map.rs:111` `SAMUS_GEAR: u16 = 0x6878`。
- 状态机两份例程：`SamusRoll: ; 07:D0E1`（`refs/m1disasm/SRC/prg7_engine.asm:3618`）、
  `SamusPntUp: ; 07:D198`（同文件 `:3737`）；分派表 `GoSamusHandler ; 07:CC1A` 10 态。
- 房间加载瞬态：`refs/m1disasm/SRC/constants_ram.asm:589`
  `RoomNumber db ;$5A ;Room number currently being loaded. #$FF=no room requested.`
  ⇒ 场景中 `$005A` 的 `14→F1→F2→F4→FF` 是**加载链**，不是玩法状态。

## 3. 时间基（全部实测，不假设）

- 开场演出：`movies/eien86-6725M.inputlog` 的起手是 **f22 / f26 两次 Start**（`$10`），
  其后 **f237** 才第一次左移；`analysis/traces/_probe-axis.frames.jsonl` 显示
  `MainRoutine($1E)` 在 f0..470 恒为 8（SamusIntro），**f480 起为 3**（GameEngine）、
  status 归 1 ⇒ 可操作点 f≈480。
- 因此场景把「能力注入 + 动作」全部安排在 f≥470，动作窗 100 帧（f500..600）足够覆盖
  入球 → 滚行 → 出球三次状态迁移。

## 4. 场景文件与命令

场景：`cases/case-mechanism-morphball.txt`（600 帧，`snapshot none`，`watch 0014 Joy1Latch`）。

```
python3 tools/movie_ref/mechanism_scene.py \
  --case cases/case-mechanism-morphball.txt --frames 600 \
  --json-out data/status/mechanism_scene_morphball.json
```

本次机器输出（`data/status/mechanism_scene_morphball.json`）：

- `OFFSET +0`（anchor=`latch(-1+gap)`，即 `mesen[f] == rust[f+0]`）
- `compared=1168`、`mismatch=0`、`incomparable=16`（字段缺行为常量段，不计失配）
- `distinct_values{status: 9, room: 5}` ⇒ `informative=true`
- `RESULT: all-match`

## 5. 能力前提为什么用 fixture（而不是作弊）

`fixture 470 6878 10` 注入的是 **`SamusGear` 的 gr_MARUMARI 位**——这是「萨姆斯已经
拿到变形球」这一**前置条件**，不是被判定字段。判定面只有 `status`（`$0300`）与
`room`（`$005A`），两者在本场景里全部由被模拟机器（两腿各自的核心）产生，注入点
早于动作窗且只注入一次（emu 侧 `fixtureAppliedAt[]`、rust 侧 `rust_screenshot.rs:493-502`
同语义，均为「帧 ≥ F 的首帧注入一次」）。

反过来说：如果不注入，游戏起始点萨姆斯没有变形球（`$6878 & 0x10 == 0`），
入球闸门会直接拒绝，场景退化成「按了 Down 但什么都没发生」的常量窗——
那才是真正的空对空。本场景的 informative 判定（status 9 个取值）就是防止这一点的护栏。

## 6. 局限（必须如实记录）

1. 本场景**不重现** 6725M 长片里 f1030..1080 的那段分歧；那段已判定为
   quickerNES↔Mesen2 的核心差（`docs/findings/movie-6725m-f1031-unroll.md` §9）。
   本场景证明的是「变形球状态机在本移植与 Mesen2 上行为一致」，不是「长片该段已修」。
2. 只覆盖 `Stand → Roll → PntUp → Stand` 一条链；球内左/右滚只走了右侧一条，
   球内受伤、球态下钻管道（`SamusPntUp` 的另一分支）未覆盖。
3. 帧轴对齐靠 `latch` 因果锚：本场景 latch 跳变 ≥3 次，满足 Task 1 的唯一性要求；
   若将来改输入导致跳变不足，runner 会 fail-loud（`change_runs < 3`）。

## 7. 与台账的关系

- `data/status/coverage_ledger.json` 的 `morphball-unroll` 由 `route-only` 升为 `verified`，
  evidence 增列：本文件、`cases/case-mechanism-morphball.txt`、
  `data/status/mechanism_scene_morphball.json`。
- 升级后必须走 `python3 tools/coverage_ledger.py --update --reason <text>` 抬高基线
  （只升不降，`--init` 在基线存在时 exit 2）。
- 覆盖面仍然很窄：台账 3 条里只有 1 条到 verified；`door-transition`、`room-load-chain`
  仍是 `route-only`，按同样方法（600 帧机制场景 + informative 护栏）逐条推进。
