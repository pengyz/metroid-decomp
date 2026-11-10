# BCD 掩码审计卡：SubtractHealth `r1 & 0x9F | (h1 & 0x60)` 定谳 + Varia 半伤公式勘误

> 【日期】2026-11-10 · porter（改写工程师）· 缘起：verifier cda2f2cb r9 复核遗留 B 级疑点
> 【定性结论】**掩码=错误实现（行为分歧，非等价投影亦非无害防御）**；审计过程连带发现
> Varia 半伤公式同域分歧，一并勘误。
> 【处置】TDD 红测先行（`bcd_mask_audit_tdd` 4 测试）→ 修 rust 对齐 asm → 绿。
> rust 现状：`crates/game-core/src/frame/mainloop.rs` `subtract_health`（掩码已移除、
> Varia 公式已对齐），测试模块 `mod bcd_mask_audit_tdd`（同文件）。

## 1. asm 真源链（A 级：bank:addr 原文）

- `SubtractHealth` `07:CE92`：refs/m1disasm/SRC/prg7_engine.asm:3203-3257。
  关键序：`lda HealthChange; sec; jsr Base10Subtract; sta Health`（:3221-3224）→
  `lda HealthChange+1`（**无 sec**，carry-in=上一次调用 carry-out）`jsr Base10Subtract;
  sta Health+1`（:3230-3234）。
- `Base10Subtract` `07:C3FB`：prg7_engine.asm:1072-1092 → `ExtractNibbles` `07:C41D`
  （:1094-1104）。借位机制（A 级原文语义）：
  - 低 nibble `sbc $01` 借位时 `adc #$0A`（回绕置 carry）→ `lda $02; adc #$0F`
    （carry=1 ⇒ **+$10 并入被减数高位域**）；
  - 高位域 `lda $03; and #$F0; sec; sbc $02` 借位时 `adc #$A0`（**+$A0 BCD 回绕调整**，
    可置 bit7/bit6-5）→ `clc`。
  ⇒ **Health+1 的 bit4-bit7（罐数 nibble 域）参与全部借位/回绕运算，结果可合法改写
  bit6-5 甚至 bit7**。
- Varia 半伤 `07:CEA9-CEBD`（prg7_engine.asm:3226-3235）【2026-11-10 r9 复核更正：原写
  :3216-3221 区段偏移 10 行，标签 LCEA9-CEBD 正确】：
  `lsr HealthChange`（进位被下一条 lsr 覆盖，无效）；`lsr HealthChange+1`（bit0 借出→
  carry）；`bcc` 退出；`lda #$4F; adc HealthChange`（carry=1 ⇒ **a = $4F + (a>>1) + 1**
  = 十位域补偿 +$50）。
- Health 布局（消费面佐证）：DisplayBar `07:E0C1` HUD 百位=`$0107&0xF`、十位=`$0106>>4`、
  个位=`$0106&0xF`；能量罐 `full=$0107>>4`（prg7_engine.asm:2714-2763 同构 rust 面）。

## 2. 逐指令对拍（模拟升级：B 级静态 → A 级）

模型：`analysis/bcd_mask_audit/diff_model.py`——asm 侧 6502 逐指令（ADC/SBC carry 语义
精确：`sbc`=A−M−(1−C)、借位补偿的回绕置 carry 链、第二次 Base10Subtract 无 sec 的
carry 链）；rust 侧=修订前 mainloop.rs 直译。域：Health/HealthChange 均按 BCD $00-$99、
罐数 nibble 0-5；枚举 3,360,000 组合（damage 100×100 全域 × Health 网格 × Varia 开关）。

结果：**分歧 2,705,540 组合**，分两类：

| 类别 | 组合数 | 机制 |
|---|---|---|
| 掩码分歧（差异仅在 $60 位） | 1,702,890 | 掩码剥离 asm 借位/回绕合法结果后回拼旧罐数 |
| Varia 公式分歧（varia=True 全域） | 1,002,650 | `(a+0x4F)>>1` ≠ `$4F+(a>>1)+1` |

锚例（asm=真值 / rust=修订前）：

| 场景 | Health | HealthChange | asm | rust(旧) | 备注 |
|---|---|---|---|---|---|
| BCD 回绕 | (00,00) | (00,21) | (00,**79**) | (00,**19**) | 借位补 $10+回绕 $A0 ⇒ 高 nibble=$7，掩码剥 $60 |
| Brinstar 可达 | (50,20) | (80,00) | (**70**,**19**) | (**70**,**39**) | 2 罐+伤 $0080：旧掩码回拼 $20 ⇒ $39 |
| Tourian 可达 | (00,00) | (00,03)×Varia | (**50**,**98**) | (**73**,**98**) | $0300×Varia：asm $0150=150、旧 rust $0127=127 |

可达性（A 级表原文）：Brinstar `AreaEnemyDamage=$0080`（prg1_brinstar.asm:124-125）、
Tourian `=$0300`（prg3_tourian.asm:131-132）、熔岩 `#$07`（prg7_engine.asm:7048-7049）、
弹敌/骷髅弹 `#$50`（:12231-12232/:12715-12716）。锚例 2、3 均为可玩路径实伤。

## 3. 定性

- 掩码**不是**等价投影：asm 无任何 Health+1 高位保稳逻辑；掩码在 1,702,890 组合上
  改写 asm 输出（含 Brinstar 实伤路径），且破坏 asm 内建死亡判定输入域
  （CEDE-CEE1 `lda Health; and #$F0; ora Health+1; beq` 读的是未掩码全字节）。
- 掩码**不是**无害防御：它把「借位合法吃进罐 nibble」的 asm 行为改成了
  「罐数 nibble 恒保」，属于凭 HUD 语义直觉（「高位=罐数」）发明的钳制——
  AGENTS.md §3-5「禁止试错式修改」反面教材，登记为错误实现并已修。
- Varia 公式分歧独立成 defect，同卡同修（asm 的 `+$4F+1` 是 BCD 百位借出
  （−100/2=−50）到十位域的补偿，非二进制四舍五入）。

## 4. 验证

- TDD：`mod bcd_mask_audit_tdd`（mainloop.rs 内，循 subtract_health_shared_gate_tdd
  先例——subtract_health 为私有 fn，tests/ 集成面不可达，卡面指定 tests/ 目录据实
  偏离并在此留痕）。先红（3 失败=预测字节差逐一对上）后绿（4 通过）。
- `cargo test --workspace` 517 全绿（513→517，`baseline update --reason` sanctioned
  上调留痕 baseline_history.jsonl，无回退：仅新增 4 测试，原 513 无一改动失败）。
- `python3 gates/cli.py check` PASS（13 PASS / 0 FAIL / SKIP 显式打印）。
- L4：`python3 tools/l4_diff_run.py run-all`（.l4_run.lock 串行纪律遵守）结果见 commit。
  口径注【2026-11-10 r9 复核登记】：porter 报「101 采集点」为总采集点数，gates l4
  ratchet 分母=98（match 98 diff 0 blocked 0）——判定不受影响，后续统一以 gates
  ratchet 口径为准。

## 5. 域局限（r9 复核新增，B 级登记）

全 BCD 域对拍（r9 verifier 独立复跑）：fixed rust 与 asm 模型在 349,680/3,360,000
组合仍分歧——Varia 半减产生非 BCD nibble 时（如 dmg 0x0015×Varia→halved 0x000A：
asm 借位链产出 $FF，rust 单次 +10 产出 $9F）。但穷举**全部游戏内 HealthChange
生产源**（5 张 AreaEnemyDamage $0080/$0100/$0300/$0200/$0240 + 熔岩 #$07 + 弹敌
#$50 + MotherBrain (00,02)）× 全 Health 域 × Varia 开关：192,000 组合 diverge=0。
即分歧域在原版不可达输入空间，属「简化形」潜在局限而非行为漂移——当前修复与
asm 在全部可达输入上行为一致；若未来引入自定义伤害源（Stage B），需回到 asm
全形实现。

Evidence: 07:CE92/C3FB/C41D 汇编原文 + diff_model.py 3.36M 组合逐指令对拍 + 锚例字节表
Confidence: 95（模拟=6502 指令语义确定性模型，非 Mesen 实机 trace；实机佐证由 L4 全矩阵兜底）
