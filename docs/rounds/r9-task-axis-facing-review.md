# R9 对抗复核 verdict——d41491a0（axis-facing 修+data05 演化径定性）

- 复核员：verifier-scroll（证伪立场，非实施方）
- 日期：2026-10-29
- **verdict：pass**（附一处测试力注记，非阻塞）

## 1. asm 核 —— PASS

- GetEnemyTypeTimes2PlusFacingDirection（$F844，prg7_engine.asm:11500-11509）实录：
  `lda Ens.0.data05,x / bpl @common / lsr / lsr / @common: lsr / lda type / rol / tay`
  —— 迹位追踪：bit7 置（bpl 不跳）⇒ 三连 lsr ⇒ carry=原 **bit3**；bit7 清（bpl 跳）
  ⇒ 单 lsr ⇒ carry=原 **bit0**；`rol` ⇒ type×2+facing。rust
  `if d05&0x80 != 0 {(d05>>3)&1} else {d05&1}` ——逐位同式 ✓。
- f800 案 bit7 清径论证：实测双侧 data05 bit7 均 0（下条）——该案残差非本修
  直接因（axis 修作用于 bit7 置的竖轴爬虫错帧族）；两支独立归因成立。

## 2. data05 直比证据 —— PASS（仓库工件复现）

- 复现载体：case-collide-probe f00800_l240 .ram.hex（leftwalk/end、jumpledge
  land-settled 三 case 同值——三重独立）：
  - slot4 $445：emu=**1A** / rust=**58**，XOR=0x42 ✓；slot5 $455：emu=**5A** /
    rust=**18**，XOR=0x42 ✓——位 6/1 双爬一致分歧实锤。
  - bit7 双侧均 0（位 6/1 差，非 bit7）——与 bit7 清径论证一致。
- rust 独有写复现：slot4 全槽 dump rust `[28 BE 80 00 00 58 …00 01 …]` vs emu
  `[28 BD 00 … 1A …00 00 …]`——**$442=80（radiusX）/$449=01（delay）rust 独有**
  （$452/$459 同）+x 字节 BD/BE（±1，FC 域 AI 轨迹）——声明逐项命中。
- 写集来源：定性为「敌状态演化径分歧」（上游 AI/spawn 径，未修、开卡待续）——
  定性留档合规（不混入本修结论）。

## 3. 修本体测 —— PASS（附注记）

- `axis_facing_bit7_uses_bit3`：data05=0x88（bit7+bit3 置，bit0 清）⇒ 期望 5；
  旧形读 bit0 ⇒ 4 ⇒ **红**——mutation 有效（真锚）。
- **注记**：`axis_facing_bit7_clear_uses_bit0` 对旧形**不具区分力**（旧形同读
  bit0 ⇒ 同绿）——系语义钉（文档化 bit7 清径），非 mutation 锚；可接受
  （真锚在 test1），登记备查。

## 4. dump 覆域缺口 + 终钉方案 —— PASS

- dumpRam 覆域=$0000-$07FF（2KB，scenario_capture.lua:122 面）——EnsExtra
  $6AF4（WRAM）不在 .ram.hex 证据链 ✓（论证成立）。nuance：bundle-capture 路径
  （:542 readRange $6000-7FFF）含 WRAM，但仅一次性存档面非逐点采集——
  终钉方案（扩 dump 覆 EnsExtra/WRAM 或启用 bundle 面）合理。

## 5. 必跑验证 —— PASS

- `cargo test --workspace`：**447 passed / 0 failed**（game-core **413**=411+2 新测 ✓）。
- L4 fresh run-all：**42/67 无回退**（门链五点/落地三点保持）。
- `gates/cli.py check`：**PASS=13 FAIL=0，CHECK: PASS**。

## 裁定

| 点 | 结论 |
|---|---|
| 1 asm 核（$F844 迹位追踪；f800 bit7 清径独立归因） | pass |
| 2 data05 证据（三 case 工件复现 1A/5A vs 58/18 + 独有写逐项） | pass |
| 3 修本体测（test1 真锚；test2 语义钉注记） | pass |
| 4 dump 缺口论证+终钉方案 | pass |
| 5 验证（447/0；gates 13/0；42/67） | pass |

**verdict = pass**。

Evidence: prg7_engine.asm :11500-11509；case-collide-probe/leftwalk/jumpledge
f800 双侧 .ram.hex（data05 逐槽+slot4/5 全槽 dump 实读）；physics（handler）新测
diff；scenario_capture.lua :122/:542；shots_rust/matrix.md（fresh 42/67）；
cargo test（447/0）；gates check（PASS=13 FAIL=0）。
