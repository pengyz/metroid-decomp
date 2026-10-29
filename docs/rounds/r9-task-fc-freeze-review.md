# R9 对抗复核 verdict——db745dbd（FC 冻结 porter 修：title 内旋族臂不返回帧不自增 FrameCount）

- 复核员：verifier-fc-freeze（证伪立场，非实施方；全部证据独立重导，未采信实施方叙述）
- 日期：2026-10-30
- **verdict：needs_revision**（功能面全证通过；**debug eprintln 残留触 game-core 无 I/O 红线**——修订面窄，见裁定表 R1-R3）

## 1. asm 锚核 —— PASS（A 级，ROM 字节直证）

- **唯一 `inc FrameCount`**：prg7_engine.asm:207，位于 MainLoop（07:C0BC）迭代尾
  `jsr GoMainRoutine` 之后、`lda #$00/sta NMIStatus` 之前；全反汇编 grep 仅此一处 `inc FrameCount`
  ——「例程不返回⇒迭代未完成⇒FC 不自增」的 asm 结构前提成立。
- **ROM 直证**（bank7 基 0x10+7*0x4000=0x1C010）：
  - 07:C0BC=`20 DE C4`（jsr CheckBankSwitch）/C0BF=`20 66 C2`（UpdateTimer）/
    C0C2=`20 14 C1`（GoMainRoutine）/**C0C5=`E6 2D`（inc FrameCount）**/C0C7=`A9 00`/
    C0C9=`85 1A`（sta NMIStatus）/C0CB=`A8 A5 1A D0 03`（spin 头；bne+3→C0D3——
    复核首算 D0 02 系自误，ROM 与 listing 一致）/C0D0=`4C CB C0`（spin 回跳）。
  - FrameCount=$2D（constants_ram.asm:498）。
- **内旋族结构直证**：WaitNMIPass（07:C42C）/WaitNMIPass_（07:C43F）=
  `jsr ClearNMIStat / lda NMIStatus($1A) / beq 自旋 / rts`（ROM：`20 34 C4 A5 1A F0 FC`）；
  ScreenOff/ScreenOn 落入 WriteAndWait→WaitNMIPass_——**段尾内旋不返回主循环**。
  - $01 臂 DrawIntroBackground（00:80D0）：`inc TitleRoutine`（→0x02）在例程**体内**，
    尾 `jmp ScreenOn`（内旋）——整臂一个 MainLoop 迭代、相位推进在迭代内、末尾自旋 ✓。
  - $15|$1B 臂（00:90BA，双入口）：`jsr ClearAll`（$909F：ROM 实录
    `20 39 C4 / 20 58 C1 / 20 A3 C1 / … / 20 2C C4(jsr WaitNMIPass) / 4C 7D C4(jmp VBOffAndHorzWrite)`）
    →PreparePPUProcess_→**`sta TitleRoutine`（→$16 ChooseStartContinue）**→TurnOnDisplay
    （NMIOn+jmp ScreenOn 内旋尾）——段间无独立帧、推进在末工作段 ✓。
- **占位组表址顺正直证**（JumpEngine 表 00:802A，`.word ptr` 直存）：
  table[0x12]=`ED 82`（→$82ED TitleScreenOff）/table[0x17]=`1A 91`（→$911A
  LoadPasswordScreen）/table[0x1B]=`BA 90`（$90BA）/table[0x1C]=`9E 93`（GameOver
  $939E——**旧形把 $1C 的表址误配给 0x12/0x17，本次顺正与 ROM 逐字一致**）/table[0x01]=`D0 80`。

## 2. 代码读证 —— 旗生命周期闭合 PASS；但发现调试残留（R1/R2）

- **旗生命周期**：写点 3 处（title_bridge.rs:25 入口先于 spin 消费——覆盖 spin tick 早退；
  :81 head_hit 命中帧置 true——命中帧=转换例程体同帧执行）＋清点 1 处
  （mainloop.rs:52 无条件清，紧随 :49-51 FC 门控消费）。非 title tick 旗恒 false（上拍已清）
  ⇒ FC 正常自增；闭合，无遗漏写点。
- **死代码**（R2）：title_bridge.rs:36 的第二次 `matches!(phase, 0x01|0x15|0x1B)` 与 :25
  等值（:25→:31 间无 TITLE_ROUTINE 写点，spin 路径早退）——纯冗余重赋值。
- **R1（红线）**：mainloop.rs:48 `eprintln!("[dbg-f] …")` 与 title_bridge.rs:32
  `eprintln!("[dbg-t] …")` 为**库热路径无条件下调试打印**（git log -S 确证 db745dbd 引入，
  每帧每 tick 刷 stderr）——触 AGENTS.md §2 「game-core 无 I/O」红线（既有先例：打印只在
  `#[cfg(test)]`（render.rs:77 区）或 bin/ 壳（rust_screenshot.rs）——库运行面零先例）。
  不影响游戏状态确定性，但属违纪残留，必须剥离。

## 3. 实测判据一（fcprobe 逐帧 FC）—— PASS（独立复现 0 分歧）

- 前置：`ps aux` 无并发 Mesen/l4 进程。fresh `python3 tools/l4_diff_run.py run case-fcprobe`：
  L4 双采集点（f420 game-in/f460 end）✅ 一致 0px×2。
- **独立解析**（自写 python，非实施方脚本结论）：
  `analysis/traces/case-fcprobe.frames.jsonl`（emu 460 帧 w.FC）×
  `shots_rust/case-fcprobe.state.jsonl`（rust 452 帧 zp：$2D=偏移 0x5A/$1F=0x3E）
  → 公共 452 帧，**FC 分歧帧数=0** ✓（判据一达成属实）。
- 两窗实录（双侧逐帧抽点）：f4/f5 FC=1 冻、f6 FC=1 冻且相位 01→02 冻帧内推进、f7 FC=2 复增；
  f311/f312/f313 FC=49 三连冻（press310 后 3 帧）、f313 冻帧内相位 1B→16、f314 复增——
  与 findings §2 A 级记录逐帧吻合。
- **遗留登记核实**：TitleRoutine($1F) 分歧仅 1 帧（f7：emu 02 vs rust 03），f8 起双侧同 03
  自愈——findings §5「0x02 臂单帧相位差、f8 自愈」如实 ✓。

## 4. 测有效性（fc_frozen_on_no_return_title_arms）—— PASS

- 回退旧形（每 tick 无条件 +1）推演：$01 段 tick1/2 必然 FC=1/2 ⇒ 首断言
  `FC==0` 必红；$1B 段同理 `FC==1` 必红——**主门控 mutation 必杀**。
- 剥族成员（去掉 0x15|0x1B）⇒ $1B 段 FC 提前自增必红；回退融合段（恢复独立尾段）
  ⇒ `TITLE_ROUTINE==0x16` 断言必红——**族归属/融合两面同钉**。
- 局限（如实）：head_hit 置旗路径不在本单测覆盖（需 Start 按键面）——由判据一
  fcprobe 逐帧差分实证兜底（f311 首冻帧复现），分层接受。

## 5. 验证面实跑 —— PASS（数字辨析见 R3）

- `cargo test --workspace`：**453 passed / 0 failed**（全捕获精确累计；较上轮
  screenx 复核实录 452 +1=新钉 fc_frozen_on_no_return_title_arms ✓）。
- `python3 gates/cli.py check`：**SUMMARY: PASS=13 FAIL=0 SKIP=1 TOLERATED=2
  CHECK: PASS**（SKIP=proof-checks 变更面不含 S_intx 显式跳过；TOLERATED=trace/visual
  既有账内登记，均显式打印，无静默）。rust-tests 项：测试数 457（基线 457）
  ——静态 `#[test]` 计数面。
- **L4 零缩水**：l4 门 PASS——「分母 76（blocked 0）｜match 50 diff 26 blocked 0」
  与实施主张逐字一致；match 集合只增不减由 ratchet 机械裁定 ✓。

## 6. 差异分类（判据二/case-001）—— 登记如实 PASS

- fresh `run case-001`：一致 7 / 失配 6（f810/f815/f820/f825+f950+f1200）。
- 独立抽点（双侧 ram.hex）：**$0441（slot4 x）f810：emu C4(196) vs rust BE(190)；
  f820：emu C8(200) vs rust BE(190)**——findings §5「f810 起 rust 190 vs emu 196/200」
  逐值吻合；slot5（$0451：emu 65/68 vs rust 5E）镜像同步差。
- 槽内状态字节另见独立分歧面（0x442: emu 00 vs rust 80；0x448: emu 00/01 vs rust 03）
  ——与「转向/撞角族独立分歧面、FC 之外」定性一致。FC 已对齐（判据一）而轨迹仍分叉
  ⇒ 判据二未达成为**事实**，路由新考证任务（本档不修）=verifier 纪律正确执行 ✓。

## 裁定表

| 点 | 结论 |
|---|---|
| 1 asm 锚核（prg7:207 迭代尾+ROM E6 2D；内旋族结构；表址 82ED/911A 顺正） | pass |
| 2 旗生命周期（3 写 1 清闭合；head_hit 覆盖点正确） | pass |
| 2' 调试残留（mainloop.rs:48/title_bridge.rs:32 eprintln；:36 冗余重赋值） | **needs_revision（R1/R2）** |
| 3 判据一（452 帧 FC 0 分歧独立复现；f7 相位 1 帧差=遗留登记属实） | pass |
| 4 测有效性（门控/族归属/融合三面 mutation 必红） | pass |
| 5 验证面（cargo 453/0；gates PASS=13 FAIL=0；L4 50/26/0 分母 76 零缩水） | pass |
| 6 差异分类（$0441 C4/C8 vs BE 抽点吻合；判据二未达成登记如实、路由正确） | pass |

**verdict = needs_revision**。核心 FC 冻结修**功能面全部证通过**（判据一独立复现、
门禁零缩水、遗留与新分歧面登记如实）；修订面窄且机械：

- **R1**：剥离 mainloop.rs:48 `eprintln!`（§2 game-core 无 I/O 红线；库运行面零先例）。
- **R2**：剥离 title_bridge.rs:32 `eprintln!`＋:36 冗余重赋值（保留 :25 先于 spin 的
  置位与 :81 head_hit 置位——二者为语义必需）。
- **R3**（证据叙述更正，原地标注）：①commit/findings §5「head_hit 帧置旗（emu f310
  冻实录）」——A 级实录为 **f311-f313 冻**（f310 实增 48→49，双侧同；findings §2
  自身「press310 后 3 帧」准确），叙述差一帧；②commit「cargo test 457 过 0 败」——
  runtime 实跑 **453 过 0 败**，457=静态 `#[test]` 计数（ratchet 面），两口径须分写。
- 修订后无需重开功能论证：剥打印属零行为变更，重跑 `gates/cli.py check`（预期
  PASS=13 FAIL=0 不变）＋cargo test 即收口。

观察（不裁、非本 commit 改动面）：`data/status/baselines/rust_test_count.json`=605
（tools/baseline_update.py 遗留 sidecar，2026-10-22「工具误用更正」串）与权威
baseline.json=457 不一致——遗留面另立任务，不阻塞本 verdict。

Evidence: ROM 直证 bank7:C0C5=`E6 2D`（off 0x1C0D5）+JumpEngine 表 00:802A 逐项
（0x12=ED 82/0x17=1A 91/0x1B=BA 90/0x1C=9E 93）+ClearAll $909F 字节链
（`20 2C C4`/`4C 7D C4`）+WaitNMIPass_ `20 34 C4 A5 1A F0 FC`——python 直读
`rom/Metroid (USA).nes`（sha e6e6b7014685adae… 对锚）；本复核 fresh 实测：
case-fcprobe（L4 2/2 一致；独立解析 452 帧 FC 0 分歧、TR 仅 f7 一帧差）、
case-001（7/13；$0441 f810=C4/BE、f820=C8/BE）、cargo test 453/0、
`gates/cli.py check` SUMMARY PASS=13 FAIL=0 SKIP=1 TOLERATED=2（L4 match 50/
diff 26/blocked 0，分母 76）；git log -S 确证 eprintln 残留引入面=db745dbd。
Confidence: 95
