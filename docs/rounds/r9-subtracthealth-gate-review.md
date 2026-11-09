# R9 对抗复核——SubtractHealth 共享门位卡（verifier）

- 复核对象：474df16a（rust 共享门+TDD）+ 63e71504（findings §66），HEAD=63e71504
- 复核人：verifier（证伪立场，独立重导证据，未引用 porter 行号当真相）
- 日期：2026-11-09
- **verdict：needs_revision（轻——代码与测试结论全部维持；仅 findings §66 遗留注记含已被证伪的手推值，须原地更正后方可定谳入库）**
- Confidence：88

---

## 逐点结论

### 1. asm 逐行（门位序）——✓

独立读 `refs/m1disasm/SRC/prg7_engine.asm:3186-3229`（07:CE84-CEBF 区段）：

```
SubtractHealth: ; 07:CE92
    lda HealthChange / ora HealthChange+1 / beq Exit3   ;① HC==0⇒rts
    jsr IsSamusDead / beq GotoClearHealthChange          ;② 门（CE98-CEA0）
    ldy EndTimer+1 / iny / beq LCEA6                     ;③ EndTimer 免伤
GotoClearHealthChange: ; 07:CEA3  jmp ClearHealthChange  ;   →F323 清 HC 不扣血
LCEA6: lda MotherBrainStatus / cmp #$03 / bcs GotoClearHealthChange  ;④≥3 免伤
    lda SamusGear / and #gr_VARIA / beq LCEBF            ;⑤ Varia: lsr HC×2
    (carry 时 lda #$4F / adc HealthChange / sta HealthChange)
LCEBF: lda Health / sta $03 / lda HealthChange / sec / jsr Base10Subtract
       sta Health /（Health+1 同构，第二次调用无 sec——借位经 C 传递）  ;⑥ BCD
```

`IsSamusDead`（:3186-3199，07:CE84）逐行：`lda Samus.status / cmp #sa_Dead(7) beq Exit3 /
cmp #sa_Dead2(8) beq Exit3 / cmp #sa_Begin(255)`（Exit3=`rts`，Z 标志判 7/8/255）。
`ClearHealthChange`（:10459，07:F323）：`lda #$00 / sta HealthChange / sta HealthChange+1`。
**与 porter 主张①-⑥逐项一致；门位确在 Varia/BCD 前。✓**

### 2. 调用点全谱——✓（且比 porter 报告多一路径消费者，无遗漏径）

`grep -rn "SubtractHealth" refs/m1disasm/SRC/` 实测（排除定义行）：

| 调用点 | 形态 | 语义（实测上下文） |
|---|---|---|
| prg7_engine.asm:7049 | `jsr SubtractHealth`（:7048 `lda #$07` 熔 lava 伤） | 熔岩 ✓ |
| prg7_engine.asm:10056→10074 | `beq GotoSubtractHealth`→`jmp SubtractHealth`（F13D，炸弹径，:10050 IsSamusDead 先行跳过死亡态） | 弹敌+炸弹 ✓ |
| prg7_engine.asm:12232 | `lda #$50 / sta HealthChange / jsr SubtractHealth`（Skree 五伤径，FC43-FC52 区段，前置 SamusHurt_F311） | 五伤 ✓ |
| prg7_engine.asm:12716 | `lda #$50 / sta HealthChange / jmp SubtractHealth`（metatile respawn 压伤） | 五伤 ✓ |
| areas_common.asm:90 `$8042` | `CommonJump_SubtractHealth: jmp SubtractHealth` | 共享跳板 |
| prg3_tourian.asm:1121 | `lda #$02→HealthChange+1 / jmp CommonJump_SubtractHealth`（MotherBrain 径） | ✓ |
| enemies/metroid.asm:288 | `HealthChange=4 / CommonJump_SubtractHealth`（**metroid 吸血/PowerUp 径——porter 卡文未点名，实测亦走本例程**） | ✓（补全） |

另有 `SamusHurt_F311`（:10442）仅置击退/isHit，伤害本身径经上述径——无绕过径。**全谱闭合。✓**

### 3. rust 归一——✓

`git diff 474df16a^..63e71504` 实测仅 +69 行 `crates/game-core/src/frame/mainloop.rs`：
门插在 `hc0==0&&hc1==0⇒return`（:2627-2630）之后、Varia（:2640）与 BCD（:2648）之前；
`mem.read_ram(0x0300)`（Samus.status，与 :218/:331 既有用法同址）∈{7,8,0xFF} ⇒
`write_ram(0x006E/0x006F, 0)` + return——与 asm ClearHealthChange(F323) 逐字对齐。
绕径审计独立复测：`grep "0x0106|0x0107" write` 全部写点=init/reset（mainloop.rs:206/:309）、
血包 +30/+20（:1189/:1192，正向）、罐满血（pickup/spawn.rs:212-213，正向）、subtract_health
本体（:2652-2653）、测试桩——**无伤害径直写。✓** 与 PENDING-001 门（uw_collision_detection
域）作用于不同函数，幂等并存 ✓。

### 4. TDD 放宽裁定——✓ 不构成「调松掩盖」

裁定：**放行为守门卡范围内合理**。理由：
- 三条门语义用例（255/7/8）**字节面精确**断言（health 低/高字节 0x00/0x03 不变 + $6E/$6F
  清零）——门语义零放宽；
- 仅 stand(0) 对照用例放宽为行为面（≠30 即过）。该用例的职责是「门不得误滤存活态」，
  不是 BCD 扣减正确性；若钉字节面精确值（当前 bcd_sub 产出 0x92/0x02，见第 6 点），
  会把 §66 已声明另行卡裁的 BCD 布局问题隐性并入本卡——与注释声明一致，非掩盖。
- 防弱化审计：`git diff 247ac0a2..63e71504` 全查仅 +93 行（mainloop.rs + findings 文档），
  **零阈值/白名单/跳过测试/门禁改动**。gates/ 目录零触碰。
- minor：stand(0) 对照断言过弱（任何变化即过，退化扣减也放行）——建议后续把当前
  bcd_sub 行为（0x92/0x02）作为 characterization 精确值钉死，归入第 6 点的另开卡。

### 5. 全绿独立复跑——✓

- `cargo test --workspace`：**509 passed / 0 failed**（独立复跑实测汇总）；
  单跑 `cargo test -p game-core --lib subtract_health`：3 passed
  （sa_begin_255 / dead_7_8 / stand_0 三例全绿）。
- `python3 gates/cli.py check`：**PASS=13 FAIL=0 SKIP=1（proof-checks 显式打印）TOLERATED=2**
  （trace-diff/visual-diff 账内容忍，均为 2026-11-02 前既有登记，未新增未放宽）。
- L4 抽查（独立复跑）：case-001 **13/13 一致 diff 0**；case-shootprobe-probe（弹敌伤害径，
  恰覆盖本卡语义）**4/4 一致 diff 0**。
- minor：两条 ratchet WARN PENDING（knowledge_entries 44→45、rust_test_count 510→513，
  现值高于基线待 baseline update——只升不降方向，非弱化）。

### 6. BCD 遗留注记——✗ porter 手推值被证伪（本次复核唯一实质缺陷）

porter §66 主张：`BCD 0x00−0x08：rust=0x92 借位，asm 手推=0xF9`。

**证伪过程（A 级：asm 逐行忠实 6502 模拟，prg7_engine.asm:1072-1102）**：
关键在于 `ExtractNibbles`（$C41D）**尾部是 `lda $03 / and #$0F`**——进入 `sbc $01` 的 A
是 **Health 的个位 nibble**（porter 手推疑似漏看该尾部，误用 A=change 高 nibble）。
忠实模拟（健全性校验：0x25−0x08→(0x17,0x03)=325−8=317 ✓；0x00−0x08 跨字节借位→
高字节 0x03−1=0x02 ✓）：

```
输入 Health=(0x03,0x00) HealthChange=(0x00,0x08)：
asm  全序列输出 = (0x92, 0x02)
rust 全序列输出 = (0x92, 0x02)   # bcd_sub(0x00,0x08)=0x92 借位；bcd_sub(0x03,0x01)=0x02
```

**逐字节一致。「asm 手推=0xF9」为 porter 手推错误，不成立。**

影响与结论：
- L4 108/108 的根基**不受动摇**（对拍裁 Mesen 真值，经验绿如旧；BCD 疑问本就限于极端
  借位形态）。
- §66 遗留注记**必须原地更正**（AGENTS.md §3.6 证伪更正纪律）：删除/标注「asm=0xF9」，
  改记为：`0x00−0x08 实测 asm=rust=(0x92,0x02) 一致（复核 R9 模拟实证）`。
- 另开卡口径需重定：bcd_sub 本体对 Base10Subtract 一致；**真正待核的是 rust :2653 的
  `r1 & 0x9F | (h1 & 0x60)` 掩码——asm 无此对应物**（asm 直接 `sta Health+1` 保留计算
  结果 bit5/6）。当高字节计算结果 bit6/5 非零且旧值 bit6/5 不同时可分歧（B 级静态推断，
  未触发于现有 L4 采集面）——另开卡应以此为准，而非「bcd_sub vs Base10Subtract 0xF9」。

---

## 遗留缺陷列表

| # | 级别 | 内容 | 处置 |
|---|---|---|---|
| D1 | **major（docs）** | findings §66「asm 手推=0xF9」为手推错误，已证伪（asm=0x92 与 rust 一致）；§66 未更正前不可作为另开卡的证据基础 | lead 入库前先补原地更正注记 |
| D2 | minor（docs） | §66 调用点清单漏列 metroid.asm:288 吸血径（虽同样走 SubtractHealth，不影响「一处修齐」结论） | 更正时顺带补全 |
| D3 | minor（test） | stand(0) 对照断言过弱（任意变化即过）；建议钉当前行为 (0x92,0x02) 为 characterization | 归入 BCD 另开卡 |
| D4 | minor（code，B 级未触发） | mainloop.rs:2653 `r1 & 0x9F \| (h1 & 0x60)` 掩码无 asm 对应物，罐数位边缘形态潜在分歧（现有 L4 面未触发） | 另开卡（替代原「0xF9」口径） |
| D5 | minor（gates） | ratchet WARN：knowledge_entries 44→45、rust_test_count 510→513 待 baseline update（上行方向） | 常规 baseline update |
| D6 | minor（文本面） | 63e71504 commit message「:10074 F13D 弹敌」表述可，但 findings §66 未提 :10056 `beq GotoSubtractHealth` 为炸弹径入口（:10050 IsSamusDead 已先行过滤死亡态——与共享门重复但幂等） | 文字面，随 D1 一并修 |

## 结论

代码卡本体（asm 门序、调用点全谱、rust 归一、TDD、全绿）全部经独立重导证据证实；
唯一实质缺陷是 §66 遗留注记的手推值错误（D1）——**verdict=needs_revision（仅限 docs
更正，代码与测试无需返工）**。lead 完成更正后本卡即可转 pass 入库。
