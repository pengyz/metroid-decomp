# R9 复核报告：PENDING-001 工具战役+闭合（porter 0ca89b88，commit 链 5e6220ab..24dee571）

- 复核员：独立对抗 verifier（证伪立场）
- 日期：2026-11-09
- 复核对象：3f28a061（tools/T1）/ 61034aab（gate baseline）/ 9f375789（core/T3）/ 24dee571（docs）
- **verdict：pass**（3 项 minor 文字面缺陷，不触判定有效性；见 §M）

## T1 git diff 逐 commit 审

- 3f28a061 仅触 `tools/scenario_capture.lua`（+103/-31）：整数枚举注册（emu.callbackType.{read=0,write=1,exec=2}）、ctread 双参 (addr,value)、pc-range 落盘门、ctwrite 块、CTDIAG summary。无夹带、无判定逻辑改动、无门禁弱化。✓
- 61034aab 仅触 baselines 三件（507→510，tamper_recovered 留痕沿 5e6220ab 先例，sha 链闭合）。✓
- 9f375789 仅触 `crates/game-core/src/frame/mainloop.rs`（+72：门+TDD mod）。✓
- 24dee571 仅触 findings §65 追加 + pending-cards 关闭标注（原档案保留不删）。✓

## T2 工具面实证核（独立重读产物）

- `analysis/traces/case-killenemy-ctdiag.lua.log` ctdiag summary（末跑）：`exec F034=304 F09D=304 F09F=1824 F0D4=608 F0DA=608 F0DD=608 F084=1216 F0ED=304 CB29=304 C0BC=1085`；F0DF/F140/F146/F1FA/F282/F311 exec **缺席（=0）**。✓ beq 608/608 全走由 F0DF=0 直接实证。
- exec 断点真每帧命中：早前探针 summary `exec C0D9=1099 exec C0BC=1085 exec C0D3=1085`（1100 帧场景，≈每帧，差 1-15 帧为 NMI 相位缺测，可接受）。✓
- `ctread.log`：`addr=300 val=FF pc=CE87` 共 **123/123**（窗 975-1015=41 帧，全量无例外）；pc=CE87=IsSamusDead `lda Samus.status` ✓。`addr=F1FA val=A9 pc=F1FA`=3936（96/帧×41）vs `ctwrite` $10（F1FC STA，首无条件写）=123 ⇒ F1FA 真执行 123≪3936，「96×/帧执行」证伪成立（判别器证据成立）。✓
- 确定性主张：14/14 ram/ppu hex dump（f00420..f00800×2）probe vs ctdiag **逐字节一致**（cmp 实测）；captures.jsonl 仅路径前缀/用例名字面异。✓

## T3 asm 逐行对读 + rust 等价性

- `refs/m1disasm/SRC/prg7_engine.asm:10069-10071`（Lx274 07:F0D4）：`lda SamusInvincibleDelay/bne NextEnemy` → `jsr IsSamusDead/beq NextEnemy` → F0DF。与 rust mainloop.rs:2543（i 帧外门）+2546（IsSamusDead 门内嵌）**次序逐字一致**。✓
- IsSamusDead（asm :3189 起，07:CE84）：`lda Samus.status/cmp #sa_Dead/beq/cmp #sa_Dead2/beq/cmp #sa_Begin/Exit3: rts`——cmp 相等置 Z；constants.asm:95-98 `sa_Dead=7/sa_Dead2=8/sa_Begin=255`。rust `status==7||8||0xFF⇒continue` 等价 ✓（sa_Stand=0 不滤，TDD 对照测实证）。
- 另一径核查：asm 敌弹×萨径（07:F0F4 Lx276）**亦有** IsSamusDead 门（:10037-10039）；asm SkreeProjectile 5 伤径（07:FC43-FC52）**无**显式门，但其伤害经 SubtractHealth（07:CE98 `jsr IsSamusDead/beq GotoClearHealthChange`）。rust 对应：敌弹径与 Skree 径（mainloop.rs:2465）均未补门，且 **rust subtract_health（mainloop.rs:2623）未实现 asm 07:CE98 的 IsSamusDead/EndTimer/MotherBrain 三门**——porter 已将此登记为遗留①（pending-cards/findings §65），本卡不阻断；建议遗留①按「SubtractHealth 共享门位」口径推进（一处门覆盖全部 HealthChange 消费径，比逐径补门更贴 asm 源）。

## T4 独立复跑实录

- `cargo test --workspace`：**runtime 506 passed / 0 failed**（21 个 suite 逐 suite 求和；game_core=466 ✓；pending001 3 测点名实测 PASS）。0 败 ✓。
- `python3 gates/cli.py check`：**PASS=13 FAIL=0 SKIP=1（显式打印 proof-checks）TOLERATED=2（显式带 reason/exit/count）**；`ratchet:rust_test_count 基线 510/现值 510 OK`（gate 口径=crates 内 `#[test]` 静态计数，实测 510 ✓）；baseline-integrity sha 链一致。✓
- `python3 tools/l4_diff_run.py run-all`：**match 108 · diff 0 · blocked 0，pass 108/108**。✓

## T5 防弱化审计

- 门只滤 status∈{7,8,255}；L4 13 用例存活态（status=0）不受影响——108/108 复跑（含门后）零回退实证。TDD stand(0) 对照测保证门不误滤存活。既有格无削弱。✓
- 基线只升不降：507→510 唯一上移；tamper_recovered 留痕规范。✓

## T6 遗留登记核

pending-cards.md PENDING-001 关闭标注 + 遗留①（Skree/PowerUp 径门核查）+ 遗留②（sa_Begin 存续机制）均落账；原档案保留不删。✓

## M Minor（不阻断，建议 lead 顺手处理）

1. **§62 未原地 dated 更正**：§62.5/62.6 的「exec 枚举注册不可达」「read 注册三证失败」结论已被本次证伪，但更正文字只在 §65/§62 引用处，§62 卡内无【2026-11-09 更正】原地标注（AGENTS.md 证据纪律 6「在原文原地标注」）。建议在 §62 卡头补一行 dated 更正指针。
2. **read 回调语义表述自相矛盾**：findings §65/commit 3f28a061 称「read=仅数据总线读不含取指」，而重写后 lua 头注释（tools/scenario_capture.lua ctread 块）称「含 opcode fetch/操作数读全部分发」；且实录 read@F1FA val=A9 pc=F1FA 96/帧 ≫ 真取指 3/帧（write 判别器 123/窗），说明该回调含幻影重复触发，「不含取指」的定性未被自身样本支持。判定结论不受影响（判别器=write 计数），但该句定性建议 dated 更正为「read 回调含幻影重复触发/语义未定谳，判定依赖 write 判别器」。
3. **commit/基线文字「cargo test --workspace 510 passed」与 runtime 实测不符**：runtime=506 passed，510 是 gate 的静态 `#[test]` 计数口径。gate 本身合规（其口径就是静态计数），仅 commit message 措辞应区分两口径。

## Confidence

97（T1-T4 全部独立重导：diff 逐 commit、asm 逐行、trace 产物重读、三命令重跑实录）。
