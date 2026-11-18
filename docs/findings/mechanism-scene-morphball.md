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

## 7. f571+ 的 Samus 精灵面分歧：已定位并修复（2026-11-18）

> 【2026-11-18 更正】本节此前写作「未修、待查」。根因已定谳为移植缺失并已修，声明窗随之
> 恢复为 `frames 600`。

**现象**：同一输入继续跑到 f572 起，两腿恒定差 **145 px / 61440**（bbox `x=136..154,
y=155..176`，OAM 槽 8..13；emu `(y,x,tile,attr)=(160,143,$14,$20)`，rust `(152,139,$69,$60)`）。
与 `fixture` 无关（删 fixture 变体 146 px，同区域同形态）。**状态面 status/room 两侧相同**。

**定位过程（全 A 级）**：

1. 起点：`f570` 像素仍 0 px；`f588/600` 恒定 **145 px**。`f571→f572` 的逐帧起点来自一次性
   临时用例扫描（该用例已删）⇒ 此细节按 B 级线索记账，A 级锚点是入库对照档
   `data/status/mechanism_scene_morphball_prefix.json`（修前/修后各三点：476/570/588/600，
   含两腿 `$0300`/`$0303`-`$0306` 原值与复现配方）。
2. 同帧 RAM 逐字节比（`ram.hex` 全 2048 B）：活跃 Samus 字段只有三处不同 ——
   `$0303`(animFrame) `emu $08 / rust $2B`、`$0305`(animResetIndex) `$07 / $27`、
   `$0306`(animIndex) `$07(→$08) / $28`。字段名锚：`refs/m1disasm/out/M1_NES_NTSC.sym:10265-10268`
   （`00000303 Samus.animFrame` / `00000305 Samus.animResetIndex` / `00000306 Samus.animIndex`）。
3. 时间线（`case-tmp-anim` 采样 558..576）：f564 起两侧同时进 `sa_PntUp`（`$0305/$0306=$27/$27`）；
   **f570 status 4→0（Stand）时两侧分岔**——emu 把 `animResetIndex/animIndex` 重置为 `$07`
   并在 f572 推进 animFrame 到 `$08`；rust 停在 PntUp 的 `$27/$28`。
4. asm 定谳（`refs/m1disasm/SRC/prg7_engine.asm`）：`SamusPntUp ; 07:D198` 松开 UP 时只写
   `Samus.status←sa_Stand`（`prg7_engine.asm:3744`），**真正的动画复位在尾件**：`lda Samus.status /
   jsr JumpEngine` 的十项表（`prg7_engine.asm:3789-3800`），表项 0 = `SetSamusStand`
   （`:3359`，动画置位指令在 `:3370`），其内部
   `NoHorzMoveNoDelay` → `lda #ObjAnim_SamusStand - ObjectAnimIndexTbl.b` → `SetSamusAnim`
   （`$0305←$07`）/ `SetSamusNextAnim`（`$0306←$07`、`$0304←0`）。
5. rust 侧缺陷：`crates/game-core/src/samus/handler.rs::samus_pnt_up` 的尾件只实现了表项 6
   （`SetSamusJumpPntUp`），**表项 0/1/3 被注释为「由各自径承担」而实际无人承担** ⇒ 站立动画
   不复位。修复=按 asm 十项表补齐派发（0→`set_samus_stand`、1→`set_samus_run`、
   3→`set_samus_roll`，其余 ExitSub）。

**修复后**：`cases/case-morphball-tail-probe.txt` 576/588/600 **全 0 px**；机制场景声明窗恢复
`frames 600`，4 个采集点（476/570/588/600）**全 0 px**，状态面 `compared=1168 mismatch=0`。

> 【2026-11-18 复核更正（独立复核 097733e1 的 P1）】原稿把「145 px / RAM 三字段」写成结论，
> 但其载体是已删除的临时用例，仓库里查不到产物。现补机器档
> `data/status/mechanism_scene_morphball_prefix.json`（修前形态 = `if status_raw()==6 { … }`，
> 与 `HEAD~2` 一致；**不能**用无条件调用冒充修前——那会让 status=0 也跑 SetSamusJumpPntUp，
> 得到错的 `$0305=$35`，实测过一次）。复跑该档脚本后，两点数字与本稿逐字一致：
> f570 = 0 px（但 `$0305/$0306`=`$27/$28` vs `$07/$07`、`animDelay`=`$01` vs `$00`）、
> f588/f600 = 145 px。

**教训**：asm 尾件的 `JumpEngine` 分派是**行为的一部分**，不能因为「另有径负责」就只实现
分支里当前用得上的那一项——注释里那句自我豁免（「改它们会移动已被 L4 冻结的行为」）本身就是
把测试当成了真相源；L4 未覆盖的路径恰恰是它保护不了的地方。

## 8. 与台账的关系

- `data/status/coverage_ledger.json` 的 `morphball-unroll` 由 `route-only` 升 `verified`，
  evidence = 本文件 + `cases/case-mechanism-morphball.txt` + `data/status/mechanism_scene_morphball.json`
  + `shots_rust/case-mechanism-morphball.l4report.json`（新增 0 px 机器指针）+ 旧有 monball/f1031 三件。
- 门槛口径（`docs/superpowers/plans/2026-11-18-three-leg-oracle-alignment.md:234`）要求
  「机制级最小重现 + Rust 同场景 0 px」：本场景在**声明窗 f0..570** 内两采集点均 0 px，
  窗外的 f571+ 分歧用独立探针留证、不计入本条目（见 §7）。
- 基线经 `python3 tools/coverage_ledger.py --update --reason <text>` 抬高（只升不降），
  状态锚点走 `python3 gates/cli.py sync-anchors` 重签。
- 【2026-11-18 更正】声明窗曾因 §7 的 145 px 分歧临时收窄到 570 帧；§7 修复后已恢复
  **600 帧**（4 个采集点 476/570/588/600 全 0 px，状态面 compared=1168 mismatch=0），
  台账 evidence 另加 `cases/case-morphball-tail-probe.txt` 与其 L4 报告作永久护栏。
  基线 `reason` 文本仍是首次升级时写的 600 帧口径（`--update` 判 NO-OP 不写盘），
  `updated_at` 标记其历史性。
- 覆盖面仍窄：台账 3 条只有 1 条 verified；`door-transition`、`room-load-chain` 仍是
  `route-only`，按同样方法（声明窗 + 负对照 + 0 px 指针）逐条推进。
