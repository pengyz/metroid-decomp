# R9 对抗复核 verdict——批 3（12351244 静止跳派发 / 6f6d5ddf 弹槽隔离 / 322f5545 BUG-090）+①接缝负结果

- 复核员：verifier-scroll（证伪立场，非实施方）
- 日期：2026-10-28
- **verdict：pass**

## 1. asm 核 —— PASS

- **SamusStand（$CC36，prg7_engine.asm:2735-2787）**：JUMP 径 `:CC65-:CC6D`
  （asm :2764-2768）status←sa_Jump(2)；`:CC6E` 段 `lda #$04/jsr SetSamusData`
  （:2769-2773）后 **JumpEngine 5 入口派发**（:2775-2783：`cmp #sa_Door/bcs RTS` +
  表 :2796-2801=[ExitSub, SetSamusRun $CC98, **SetSamusJump $CFC3**, SetSamusRoll
  $D0B5, SetSamusPntUp $CF77]）。SetSamusJump（:3434-3452）：anim/speedY 实录
  `lda #$FC/sta Samus.speedY`（:3450-3452）。**旧 rust 只写 status 漏派发 ⇒ 静止跳
  无冲量**——根因成立（移动跳经 LCD40 直调故活，与用户实测②症状吻合）。
- rust 修（handler.rs:777-791）：`st<5` 门 + match 0-4 表——与 asm 表逐项同形
  （3=Roll 未端口，DIVERGENCES 记账 no-op，防爆回退——合理记账）。
- **UpdateObjAnim（$DC8F，:5836-5838）**：`ldx PageIndex/ldy Objects.0.animDelay,x`
  ——槽式（$4B 索引）。rust 修（physics.rs:762）`base = 0x0300 + read_ram(0x004B)`
  ——同式（旧硬编码 $300=Samus 槽 ⇒ 弹径调用改写 Samus 字段+弹槽 anim 冻结，
  f810 槽态直比（animIndex emu=0x1C 推进/rust=0x1B 冻）实锤）。

## 2. 测有效性 —— PASS

- `standing_jump_triggers_entry`（handler.rs:1138-1153）：静止+JOY1_CHANGE=$80 ⇒
  status==2 且 **speedY==0xFC**。mutation：删派发块 ⇒ status 仍 2 但 speedY=0 ⇒ 红
  （speedY 为派发唯一写点锚）。
- `bullet_anim_advances`（handler.rs:1132-1146）：PageIndex=$D0 下推进断言 +
  **槽隔离断言** `$0306（Samus animIndex）==0`——旧硬编码形写 $0306 ⇒ 红。
  两测 mutation 有效。

## 3. L4 复跑 —— PASS（fresh，修复后）

- run-all：**match 34 / diff 19（34/53）**——旧 6 用例 **23/16 精确保持**（逐行核）
  + 新 case-scrollseam-probe 11/3。门链五点（002 f420-790）match、落地三点
  （001 f715/725/735）match、f1000/f1150 未动（11267/11256）。
- **①接缝负结果**：case-scrollseam-probe 右滚密集采点（f420 + r30-r240×8 +
  scroll-end = **10 点全 0/61440 match**）——列缝窗口瞬态无一处分歧 ⇒ 接缝非
  残差根因（负结果定谳成立）。（注记：负结果现仅 case+matrix 留档，未落 findings
  正文——卫生项，非阻塞。）
- 左滚段 l40 match；l80/l120/end = 473/1073/1020px（已知时序类族，BUG-089 面）。

## 4. BUG-090 合规 + "不修"定性 —— PASS

- 五要素齐：现象（全程无音）/定性（链全通至 PcAudio v1 桩）/处置（不修，lead 裁定
  +待用户裁）/影响（听觉面非像素验收面）/trace（apu_log 消费源指针）。
- 桩证据核：game-shell/game-shell-pc/src/lib.rs **:3**（「音频=v1 桩（cpal 合成随件）」）
  / **:69**（PcAudio「写流记数不合成」）/ :191-194（drain_apu 仅搬运 apu_log）——
  声音链 game-core 侧产出完备（域测钉）而壳层无合成输出=架构缺口头，定性成立；
  Stage A 不修（扩展缝）符合 D2/A 纪律。

## 5. 必跑验证 —— PASS

- `cargo test --workspace`：**441 passed / 0 failed**（game-core **407**=404+3 新测——
  任务书"407/0"即 game-core 口径 ✓）。
- `python3 gates/cli.py check`：**PASS=13 FAIL=0，CHECK: PASS**。

## 裁定

| 点 | 结论 |
|---|---|
| 1 asm 核（SamusStand 派发表+SetSamusJump $FC；$DC8F PageIndex 槽式） | pass |
| 2 测有效性（speedY 锚+槽隔离断言） | pass |
| 3 L4（34/53，旧面精确保持；右滚 10 点 0px=接缝负结果） | pass |
| 4 BUG-090 合规+不修定性（lib.rs:3/:69/:191 桩证据） | pass |
| 5 验证（441/0（game-core 407）；gates 13/0） | pass |

**verdict = pass**。

Evidence: prg7_engine.asm :2735-2801（SamusStand+JumpEngine 表）/:3434-3452
（SetSamusJump speedY=$FC）/:5836-5838（UpdateObjAnim ldx PageIndex）；
handler.rs :777-791/:1138-1153；physics.rs :762；bug-ledger.md BUG-090；
game-shell/game-shell-pc/src/lib.rs :3/:69/:191；shots_rust/matrix.md（修复后 fresh
run-all，34/53，旧面 23/16 逐行保持）；cargo test --workspace（441/0）；
gates check（PASS=13 FAIL=0）。
