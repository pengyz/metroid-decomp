# R9 对抗复核 verdict——r14 H1+H2 三 commit（104ec609/91289931/cfd2fd0f）

- 复核员：verifier-scroll（证伪立场，非实施方）
- 日期：2026-11-03
- **verdict：pass**（附 L4 锚语义一条建议）

## 1. 非确定性主张核（最重） —— PASS（HEAD 稳定；交替属修前时代）

- **复跑 2 次实测**（本复核独立双跑）：scrollseam f760 与 jumpledge f730
  **两跑全 ✅match 0px**（无交替——H1/H2 落地后该两格已稳定 match）。
- 交替主张的时域定位：重基 evidence（10:09）引「f760 diff 实录（emu 无 Samus/
  rust 有）+jumpledge f730 282px（15:26 归档）」均属 **H1/H2 落地前树**；我
  turn-18 run（14b8d82b，亦修前）两格=match——修前时代三观测（match/diff/match）
  与「FC 奇偶相位交替」相容；修后两跑恒 match=**修复消振**（非定谳错误）。
- **L4 锚语义建议（登记）**：flaky 格（run 间交替）应记 `flaky` 态（非 match
  非 diff——不计分母不锚值），并对该格附 N-run 观测记录——现 schema 无此态，
  建议并入 A-full schema v2。

## 2. for_rebase 口径核 —— PASS

- 留痕实录（l4_ratchet_baseline history）：reason=「FC 奇偶相位差（emu/rust 帧
  计时面）真值重基」+evidence 含 **rust ram.hex f760: Timer3=04/status=18/
  FC=E9/InvDelay=00（无 i-frame——blink 修不涉此格）** 实录+asm :C9D7-:CA13
  淡入奇帧闪现指针——**真差显形（非已解决）语义明确** ✓。
- 无「真差记成已解决」违规：重基后两格状态=diff（真值）；H1/H2 后自然翻回
  match（match 只增，合法路径）——语义链干净 ✓。

## 3. H1/H2 asm 抽查（全例程通读标准） —— PASS

- **CheckHealthStatus Lx009（:CE53-:CE64）**：全段通读——`lda
  SamusKnockbackIsBomb77/bpl CheckHealthBeep`（bombed⇒不闪）+`lda FrameCount/
  and #$01/bne CheckHealthBeep`（奇 FC⇒不闪=偶帧隐）+`tay/sty animDelay（A=0）/
  ldy #_id_FrameNone/sty animFrame`——H2 修（bombed+偶 FC⇒delay←0/frame←$F7）
  逐行同形 ✓（A=0 由 and #$01 后 bne 出口保真）。
- **SamusIntro（:C9D7）**：`lda FrameCount/lsr…`「Only display Samus on odd
  frames [the blink effect]」——FC&1 奇帧闪现族 ✓（淡入相位差机制同源）。
- **SetSamusNextAnim（$CF6E，:3374-3378）**：`sta animIndex/lda #$00/sta
  animDelay`——「New animation to take effect immediately」——H1 修（转身过渡
  animDelay←0 补写）asm 依据 ✓（cfd2fd0f diff 与此逐行对读）。

## 4. f1200=16069px 归因链 —— PASS（dx 重配准快验实跑）

- 本复核独立重配准实验（PIL/numpy 逐像素）：dx=-2/-1/0/+1/+2 →
  16089/**12698**/16069/17680/18490；dy=-1/0/+1 → 18667/16069/18667。
  **结论：存在 ~1px 横偏成分（dx=-1 降 21%）但主体 12698px 为真内容差**——
  纯 1px 偏假说**不成立**；FC 相位级联（Samus 位置差级联）归因与重配准结果
  相容（精灵错位不可重配准消除）✓。
- 机制衔接：H1/H2/hit 链修复后 Samus i-frame/转身相位对齐 ⇒ 级联源消除方向
  与「f1200 属级联族」一致（后续轮以 f1200 收敛值复核）。

## 5. 必跑验证 —— PASS

- `cargo test --workspace`：**475 passed / 0 failed**（game-core --lib **440**
  ✓——任务书"440/0"口径）。
- L4 双跑：**match/diff 稳定**（两跑逐格一致）；943 帧基线（game-start 7 match
  面保持）✓。
- `gates/cli.py check`：**PASS=13 FAIL=0，CHECK: PASS**。

## 裁定

| 点 | 结论 |
|---|---|
| 1 非确定性主张（HEAD 两跑恒 match=修后消振；交替属修前；flaky 态建议） | pass |
| 2 for_rebase 口径（真差显形语义+InvDelay=00 实录，无违规） | pass |
| 3 H1/H2 asm（Lx009/SamusIntro/CF6E 全例程通读同形） | pass |
| 4 f1200 归因（1px 偏成分存在但非主因；级联归因相容） | pass |
| 5 验证（475/0（game-core 440）；gates 13/0；L4 双跑稳定） | pass |

**verdict = pass**。

Evidence: 本复核独立双跑 matrix（f760/f730 两格 0px×2）；prg7_engine.asm
:CE53-:CE64（Lx009 全段）/:C9D7-:CA13（SamusIntro blink）/:3374-3378（CF6E）；
l4_ratchet_baseline history（rebase evidence InvDelay=00 实录）；本复核 PIL/numpy
重配准实验（dx=-1:12698 vs dx=0:16069）；cargo test（475/0；lib 440）；gates
check（PASS=13 FAIL=0）。
