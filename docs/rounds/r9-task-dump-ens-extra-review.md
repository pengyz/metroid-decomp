# R9 对抗复核 verdict——8c76333b（EnsExtra dump 覆域扩展+data05 位谱系全钉+两修撤回注证）

- 复核员：verifier-scroll（证伪立场，非实施方）
- 日期：2026-10-29
- **verdict：pass**

## 1. dump 扩展核 —— PASS

- 双侧同构：.ram.hex **2176B**（2048 RAM+128 EnsExtra $6AF4-$6B73）双侧一致
  （lua `for a=0x6AF4,0x6B73` + rust `0x6AF4..=0x6B73` 同式——commit diff 在案）。
- **radiusY/X emu=08 08 vs rust=00 00 复现**（case-collide-probe f800 实读）：
  slot4（EnsExtra+64）emu[65/66]=08/08 vs rust=00/00；slot5（+80/81/82）同形——
  **敌半径从不写实证**（asm :6156-6168 帧记字 1/2→radius 写点 rust 缺）✓。
- 附证：animDelay/animIndex 相位差同步现身（emu +68=06 vs rust +02；+86: 36/35；
  slot5 +83/+84 rust 缺）——与 §10「3 字段相位差」声明一致。

## 2. §10 位谱系抽查 —— PASS

- **bit6=帧奇偶标**：UpdateEnemy_UpdateEnData05Bit6（$F3AA，:10555-10572）实录
  `txa/Adiv16（slot>>4）/eor FrameCount/lsr` ⇒ carry=(slot>>4)^FrameCount bit0，
  ror 链入 bit6——与 §10 式逐字对应 ✓。
- **bit1=上帧可见性闩**：绘脸 :6178-6186 实录 `jsr IsObjectVisible / txa / asl /
  and #$FD / ora → data05`（可见旗<<1 写 bit1）+ CheckIfVisible（$F37F，:6188 面）
  `lda data05 / and #$02 / bne @exit` 借闩跳检 ✓。

## 3. 撤回正当性 —— PASS

- 连锁机理核：bit1 闩/radius 写实装 ⇒ 敌进入全帧处理/命中检 ⇒ 射击帧敌爆
  （status3）⇒ **F40D（ChooseEnemyAIRoutine-explode 爆亡链）未端口** ⇒ blocked
  ——依赖单链且 DE05 族（EnemyCompleteDeath）同缺（:8090 邻证）——撤回
  （「不强修停机待裁」先例形）合理。
- **注证留代码原位**：mainloop.rs 撤回注块在案（commit diff 实读：「双暂撤
  （注证留原位）——待 F40D/DE05 补齐一并回装」）——§3.6 合规；findings §10=
  回装证据包。
- FC 奇偶陀螺论证：bit6=(slot>>4)^FrameCount——FrameCount 全域奇偶 ⇒ 修 bit6
  写点会致全敌槽 bit6 逐帧同步翻转（槽间互换假象）——与 f800 实测「data05 位 6
  槽间互换」现象一致（前轮 1A/5A vs 58/18 XOR=0x42 位 6 差即此面）✓。

## 4. 必跑验证 —— PASS

- `cargo test --workspace`：**447 passed / 0 failed**（game-core **413** ✓）。
- L4 fresh run-all：**42/67 无回退**（门链五点/落地三点保持）。
- `gates/cli.py check`：**PASS=13 FAIL=0，CHECK: PASS**。

## 裁定

| 点 | 结论 |
|---|---|
| 1 dump 扩展（2176B 同构+radius 08 08 vs 00 00 复现） | pass |
| 2 §10 位谱系（bit6 $F3AA/bit1 :6178-6186/$F37F asm 对读） | pass |
| 3 撤回正当性（连锁链+原位注证+FC 陀螺论证） | pass |
| 4 验证（447/0；gates 13/0；42/67） | pass |

**verdict = pass**。

Evidence: case-collide-probe f00800 双侧 .ram.hex（2176B；EnsExtra 段逐字节实读
——slot4/5 radius 08 08 vs 00 00+anim 相位差清单）；prg7_engine.asm :10555-10572
（$F3AA）/:6178-6186/:6156-6168/:6188 面（$F37F）；mainloop.rs 撤回注块
（commit diff）；findings §10；cargo test（447/0）；gates check（PASS=13 FAIL=0）；
shots_rust/matrix.md（fresh 42/67）。
