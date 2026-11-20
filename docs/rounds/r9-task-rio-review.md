# r9 对抗复核：RioAI 端口（d4ad8d34）+ 整改（65b42ab5）

- 复核员：`reviewer-rio`（独立只读，`git archive d4ad8d34` → `/tmp/rv_head` 干净树复跑）
- verdict：**reject（blocker=B）** → 整改后由本条留档；整改 commit `65b42ab5`
- 复核对象：`d4ad8d34 feat(core): 端口 RioAIRoutine（bank1 01:9AF5）`

## 一、五项判定

| 项 | 判定 | 依据 |
|---|---|---|
| A asm 逐行 | PASS | `rio.asm:12-13` `lda #$80`=有符号 −128；`:15-16` `bmi` 在 data05 判之前；`:25-31` `TwosComplement_` 只在负差取补 ⇒ 比 \|Δy\|；`:35-36` `cmp #$10/bcs` 相等时不清重力。仅注释「25 行」不准（实 54 行 / 26 条指令） |
| B 等价性主张 | **FAIL（blocker）** | `rio.rs:50` 读 `status_raw()`（现役 `$6AF4` status）而非 asm 的 `$0081 EnemyStatusPreAI` 快照 |
| C 同族漏改 | PASS | `grep -rn "rio.asm" refs/m1disasm/SRC` 仅 `prg1_brinstar.asm:905`；其余区第 6 项为 Crawler/RemoveEnemy/Rts95CB/Crawler，无需改 |
| D 复跑 | 部分 PASS | 4 房 sweep 4/4 ok、room 21 `64/206/180/130` 精确复现；全矩阵在其环境跑出 148/12（emu 腿 f993 起分叉、`--fresh-emu` 后 3 采集点缺失、`dmesg` 有 Mesen segfault）⇒ 矩阵数字受 emu 环境支配，不能据此判回归 |
| E 测试有效性 | PASS | 变异 `set_accel_y(-128)→0` ⇒ 5 red / 2 green（与设计精确对应）；复原 sha 一致 |

## 二、blocker 的实质（B 项）

`$0081` 是 `UpdateEnemy` 在 `JumpEngine` 分派前写下的**快照**（asm `prg7:10511`
`lda EnsExtra.0.status / sta EnemyStatusPreAI`）。分派之后、AI 之前仍有三处改写现役 status：

- `framework.rs:539 try_becoming_active`（1→2）
- `framework.rs:559 dec_status`（2→1）
- `framework.rs:156 enemy_react_to_projectile`（→3/4/6）

因此读现役 status 在三类可达帧上与 asm 分叉：**激活帧**（asm 走 Resting 出口 anim8+noMove、
不写重力；旧形走 Active 写 −128）、**Active→Resting 帧**、**中弹/冻结/受伤帧**。

复核直证（临时测试，已复原）：`$0081=1`、现役 status=2、`accelY` 预置 7、`|Δy|=$10`
⇒ 旧形产出 `-128`，asm 应保持 `7`（`left: -128, right: 7`）。

仓库自身契约同向：`crawler.rs:54`、`skree.rs:14` 都读 `$0081`；`framework.rs:597
update_enemy_common_decide` 按 `$0081` 三出口分派；`frame/tests.rs:682-712` 的 mutation
钉注释明写「激活帧即跑 AI=缺门症」。

## 三、整改（`65b42ab5`）

- `ram_map.rs`：新增 `ENEMY_STATUS_PRE_AI = 0x0081`（`constants_ram.asm:633`）。
- `frame/memory.rs`：新增 `Manager::enemy_status_pre_ai()`——裸读收口在 `raw-surface`
  豁免的兼容辖（新 AI 模块不再各自拼地址）。
- `enemy/rio.rs`：改读快照；删「两者在 AI 入口恒等」错误注释并改写为分叉说明；
  文件头「25 行」→「54 行 / 26 条指令」。
- 测试：`stage()` 同步写 `$0081`；新增 `ai_reads_pre_ai_snapshot_not_live_status`
  （快照 Resting + 现役 Active ⇒ accelY 不被改写；反向对照快照 Active ⇒ 写 −128）。

验证：`cargo test --workspace` 808 passed；`gates/cli.py check` PASS=23；
`l4_diff_run.py run-all` 20 案 / 160 格 **match 154 · diff 6 · blocked 0**（与基线逐格同）。

## 四、遗留 findings 处置

| id | 级别 | 处置 |
|---|---|---|
| 1 blocker | `rio.rs:50` 读现役 status | **已修**（`65b42ab5`） |
| 2 high（证据） | 「全矩阵 154/6 零回归」在复核环境不可复现 | **已澄清**：干净环境重跑复现 154/6（本条 §三）；复核环境 emu 腿不稳（trace f993 分叉、fresh 跑缺采集点、Mesen segfault） |
| 3 low | 「25 行」实为 54 行 | **已修**（注释更正） |
| 4 info | Rio 的 Explode 出口在 Rust 是死代码：`framework.rs:368 update_enemy_explode` 直接 anim3+计时+CompleteDeath，不调 `choose_enemy_ai`；而 asm `prg7:10795 UpdateEnemy_Explode: jmp ChooseEnemyAIRoutine` 仍会进 AI 走 noMoveNoAnim | **登记为已知缺口**（框架级，非本 commit 引入）：`docs/findings/tile-collision-judgement.md` §5 同族「框架捷径」清单 |
| 5 info | room 21 仍 mismatch（PipeBug 未端口，现已端口） | 已随 `3d8d63dc` 处理，room 21 由 `210/345/326/276` 降至 `64/206/180/130` |
