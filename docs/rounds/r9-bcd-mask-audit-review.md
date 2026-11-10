# r9 复核档：BCD 掩码审计卡（subtract_health 勘误）

- 复核对象：commit `250a5d12`（fix(core): subtract_health 对 asm 07:CE92 勘误——移除 Health+1 掩码 + Varia 半伤公式对齐）
- 实施方：porter（0fafc45e 会话）；复核方：verifier（427dddb2 会话，证伪立场独立重导）
- **verdict: pass（Confidence 92）**

## 逐点裁定（verifier 独立重导实录）

1. **掩码主张 ✓（A 级手推）**：独立逐指令手推 `prg7_engine.asm:1072-1104`（Base10Subtract 07:C3FB）：回绕锚 (00,00)×(00,21) → `sbc $01` 0xFF 借位→`adc #$0A`=0x109→`lda $02 adc #$0F`=0x30（+$10 借位并入高位域）→0xD0 借位→`adc #$A0`=0x170→`ora $01`=0x79（旧掩码⇒0x19）；Brinstar 锚 (50,20)×$0080（`prg1_brinstar.asm:124` `.word $0080` 原文核实）⇒(70,19)（旧掩码 0x39）；第二调用 carry-in 链语义逐例对齐。porter carry 语义推导逐条一致，证伪失败。
2. **Varia 公式 ✓**：手推 `:3226-3235`（LCEA9-CEBD）：lsr 链 carry 借出⇒`$4F+0+1=$50` ⇒ $0300→$0150（旧 rust $0127）；Tourian 表 `prg3_tourian.asm:131` 原文核实。
3. **对拍模型 ✓**：复跑 `analysis/bcd_mask_audit/diff_model.py`：total=3360000 mask_div=1702890 other_div=1002650——逐位一致。
4. **TDD ✓**：反例构造验证——掩码未移除时 test1/2/3 必红、test4 对照双绿；`cargo test -p game-core bcd_mask_audit_tdd`：4 passed。
5. **独立复跑 ✓**：cargo test --workspace runtime 513 passed 0 failed（静态 517，+4 恰为新增）；`gates/cli.py check` PASS=13 FAIL=0（SKIP=1/TOLERATED=2 显式）；gates l4 ratchet 分母 98 match 98 diff 0 无回退；baseline_history.jsonl sha 链一致。
6. **防弱化 ✓**：diff 面无 gates/tools 改动；基线仅 rust_test_count 513→517 sanctioned。
7. **留痕 ✓**：两勘误合并一 commit 偏离已留痕（ratchet 只升不降禁中间态）；findings A/B 分级清晰。

## verifier 新增发现（advisory，已收口或登记）

1. **域局限（B 级，已补档）**：全 BCD 域对拍 fixed rust vs asm 在 349,680/3,360,000 组合仍分歧（Varia 半减产生非 BCD nibble 时：asm 借位链 $FF vs rust +10 → $9F）；但穷举**全部游戏内 HealthChange 生产源**（5 张 AreaEnemyDamage $0080/$0100/$0300/$0200/$0240 + 熔岩 #$07 + 弹敌 #$50 + MotherBrain (00,02)）× 全 Health 域 × Varia：192,000 组合 diverge=0——分歧域在原版不可达输入空间，属「简化形」潜在局限非行为漂移。→ 已补入 findings 档案「域局限」节。
2. 引用 nit（已更正）：Varia 代码实际 `prg7_engine.asm:3226-3235`（标签 LCEA9-CEBD 正确），原 commit 写 :3216 区段偏移 10 行。
3. 口径 nit（登记）：porter 报「101 采集点 101/101」vs gates l4 ratchet 分母 98（match 98 diff 0）——总采集点 vs ratchet 分母口径差，判定不受影响；后续统一以 gates ratchet 口径为准。
4. 静态 #[test] 517 vs cargo runtime 513 的 4 测缺口为先存口径差（两侧 +4 一致），非本 commit 引入。

## 结论

掩码=错误实现、Varia 公式分歧、修复忠实 asm、基线无回退——核心结论全部经独立重导证实，证伪尝试均失败。复核档随 docs commit 入库。

Evidence: prg7_engine.asm:1072-1104/:3226-3235 手推 + diff_model.py 复跑 total=3360000 mask_div=1702890 + cargo/gates/l4 复跑实录
Confidence: 92
