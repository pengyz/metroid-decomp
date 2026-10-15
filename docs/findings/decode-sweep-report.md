# findings：decode-sweep 扫描报告——「数据岛实为代码」解码缺口全量清单（分母 v2 证据基础）

> 状态：**首轮已证实**（2026-10-15，ROM 原字节级）。工具 [tools/decode_sweep.py](../../tools/decode_sweep.py)
> 可复跑；回归基准 BASELINE: PASS（正对照 $F0A2-$F148 复现 + 9/9 调用点 + 4/4 负对照）。
> 独立对抗复核（verifier，AGENTS §5）为步骤完成的最终门——本报告为复核输入。
> 起点：[seed-decode-gap-f0a2-f148.md](seed-decode-gap-f0a2-f148.md)（首案定谳）。
>
> 【2026-10-15 更正/续】r2 深挖（C27C 跳转表入口发现 + 未覆盖段消费者核查）后
> 数字演进：code 29→35、未决 23→17、错面确定 24 格→27 格、S 面少计 120→176
> （估计值口径不变）；FE79-FECB 由「0.904 强疑（B 级）」**定谳**为混合岛
> （$FE83-$FECA 代码 A 级 + $FE79-$FE82 内嵌表 B 级）——一切以续篇
> [decode-sweep-r2-jump-tables.md](decode-sweep-r2-jump-tables.md) 为准，
> 本文保留首轮记录不改写（历史主张不删除）。

## 0. 范围与方法（判据与证据口径）

- **输入**：`rom/Metroid (USA).nes` PRG 原字节（工具内校验 sha256=`e6e6b7…565b6`，
  A 级身份锚点失守即硬失败）+ `reference/seed/metroid.asm` 的**数据岛区间标注**
  （每行 hex 逐字节回核 PRG，不信任投影文本）。listing 不参与指令解码。
- **自建 6502 指令长度表**（151 官方操作码，操作数类型以操作码字节为准——
  [gotcha-operand-type-by-opcode-byte](../knowledge/gotcha-operand-type-by-opcode-byte.md)）；
  BRK 与非官方操作码一律拒流。
- **判据**（任务书 ①②③；④循环同构=可选强判据，v1 未实现，见 §10 遗留）：
  ① 指令流连贯至 RTS/RTI/JMP（可跨出岛标注——岛边界是 seed 标注不是语义边界，
  实证 F282/F2ED 流跨标注才到终止指令；BRK/非法/512 条未终止=流不成立）；
  ② 分支目标分类：指令边界/锚点=强通过；数据岛内=缺口信号（送入口候选）；
  代码区内部=弱通过（跳进操作数的合法技巧）；RAM/未映射=失败；
  ③ JSR/JMP(abs) 目标同 ②（JMP 间接的操作数是指针存放处，不核查）。
- **入口佐证分级**：fallthrough-from-code / xref-from-code / stream-xref=有佐证；
  仅 island-start=无佐证。verdict：岛内字节覆盖 ≥0.9（有佐证流）→ code；
  有覆盖但不足 → 未决；无覆盖且盲扫无覆盖级命中 → data。
- **否定性结论双工具复核**（证据铁律，[gotcha-listing-grep-not-rom-scan](../knowledge/gotcha-listing-grep-not-rom-scan.md)
  同族纪律）：data 判定 = 入口驱动扫描 0 覆盖 **且** 全偏移盲扫最大覆盖 < 0.5；
  盲扫 ≥0.5 升级未决并登记残余命中。
- **环⑤ 测试器先过证伪**：正对照（已定谳缺口 F0A2-F148）+ 负对照（4 个真数据岛：
  $F018 `00 55 AA FF` 图案填充色、$FC5D 指针表、$FF86 索引表、$FFE9 "METROID" ASCII）
  必须全对，否则工具 exit 1——判别力不足的「全绿」不采信（§9 有自纠实录）。

复跑命令（幂等只读）：

```sh
python3 tools/decode_sweep.py --json /tmp/decode_sweep_run1.json
```

## 1. 首轮结论（数字=工具输出原文）

```
[decode-sweep] rom sha256 ok; seed 数据岛 285 个 / 91592 字节
[decode-sweep] verdict：code=29  data=233  未决=23（岛 285；被采纳流 121，被拒 240，入口候选 364；盲扫升级 6）
```

| verdict | 岛数 | 字节 | 含义 |
|---|---|---|---|
| **code** | 29 | 1416B | **解码缺口确定**（数据岛实为代码，A 级） |
| 未决 | 23 | 16449B | 有代码流但覆盖不足或无 xref 佐证（B 级线索，待人工复核） |
| data | 233 | 73727B | 双工具复核后仍判数据（否定性结论，见 §7） |

新增调用点（岛内被采纳流，此前 seed 解为数据因而不可见）：**JSR 161 处 + JMP 47 处，全在 bank7**。

## 2. 回归基准复现（输出原文，环②「修复≠声称修复」）

```
== 回归基准（正对照 F0A2-F148 + 9 调用点 / 4 负对照）==
  正对照 bank07:$F0A2-$F148 所在岛 → code ✓  9/9 调用点复现 ✓
  负对照 4/4 所在岛 → data ✓（判别力：已知缺口 vs 已知数据全对）
BASELINE: PASS
```

首案 $F0A2-$F148：coverage=1.0，佐证入口 `$F0A2/$F0ED/$F140`，终止 `$F0EA:JMP/$F13D:JMP/$F146:JMP`
（`JMP $F09F`@$F0EA 回环与 findings 首案逐项一致）。

## 3. 解码缺口清单（verdict=code，29 岛 / 1416B，A 级）

| 岛（bank07） | 大小 | 覆盖 | 佐证入口 | 终止点 |
|---|---|---|---|---|
| $CFC3-$CFC4 | 2B | 1.0 | $CFC3 | $D001:RTS |
| $D002-$D094 | 147B | 1.0 | $D002/$D03C/$D055/$D07D/$D088/$D092 | $D039:JMP $D052:JMP $D07A:JMP $D095:RTS |
| $D5FC-$D637 | 60B | 1.0 | $D5FC/$D602/$D606/$D609/$D624 | $D606:JMP $D621:JMP $D650:RTS |
| $D78B-$D797 | 13B | 1.0 | $D78B | $D79E:RTS |
| $D94E-$D975 | 40B | 1.0 | $D94E/$D973 | $D970:JMP $D973:JMP |
| $DA39-$DA3C | 4B | 1.0 | $DA39 | $DA7B:RTS |
| $DCB7-$DCC2 | 12B | 1.0 | $DCB7 | $DCC2:RTS |
| $E000-$E011 | 18B | 1.0 | $E000 | $E01B:RTS |
| $EB06-$EB23 | 30B | 1.0 | $EB06/$EB0C | $EB09:JMP $EB27:RTS |
| $EB4D-$EB84 | 56B | 1.0 | $EB4D/$EB7A | $EB77:JMP $EB84:RTS |
| $EF9A-$EF9D | 4B | 1.0 | $EF9A | $EFF3:RTS |
| **$F0A2-$F148** | 167B | 1.0 | $F0A2/$F0ED/$F140 | $F0EA:JMP $F13D:JMP $F146:JMP |
| $F162-$F171 | 16B | 1.0 | $F162 | $F16F:JMP |
| $F1B3-$F1BE | 12B | 1.0 | $F1B3 | $F1BC:JMP |
| $F1CB-$F1D1 | 7B | 1.0 | $F1CB | $F1D1:RTS |
| $F1E7-$F1ED | 7B | 1.0 | $F1E7 | $F1ED:RTS |
| $F270-$F276 | 7B | 1.0 | $F270 | $F276:RTS |
| $F282-$F297 | 22B | 1.0 | $F282 | $F2AA:JMP |
| $F2AD-$F2B3 | 7B | 1.0 | $F2AD | $F2C9:RTS |
| $F2CA-$F305 | 60B | 1.0 | $F2CA/$F2D5/$F2D8/$F2DF/$F2E8/$F2ED | $F2DE:RTS $F2E7:RTS $F310:RTS |
| $F332-$F337 | 6B | 1.0 | $F332 | $F335:JMP |
| $F340-$F344 | 5B | 1.0 | $F340 | $F344:RTS |
| $F752-$F859 | 264B | 0.947 | $F752/$F75B/$F7B3/$F7BA/$F7CA/$F832/$F839/$F844 | $F75A:RTS $F7B9:RTS $F7C9:RTS $F82F:JMP $F83B:JMP $F851:RTS |
| $F870-$F93A | 203B | 0.99 | $F870/$F8E8/$F8F7/$F8F8/$F91D/$F92C | $F8E5:JMP $F8F7:RTS $F91C:RTS $F927:JMP $F93A:RTS |
| $F96A-$F97D | 20B | 1.0 | $F96A/$F97C | $F981:JMP |
| $F984-$FA17 | 148B | 1.0 | $F984/$F9B3 | $F9B0:JMP $FA15:JMP |
| $FA1E-$FA40 | 35B | 1.0 | $FA1E | $FA5A:RTS |
| $FE59-$FE6C | 20B | 1.0 | $FE59 | $FE78:RTS |
| $FF3C-$FF53 | 24B | 1.0 | $FF3C | $FF53:RTS |

重点缺口群（超出首案范围的新发现）：

1. **$F2CA 族**（$F270/$F282/$F2AD/$F2CA/$F2D5/$F2D8/$F2DF/$F2E8/$F2ED 九入口）：
   多入口**共享 RTS 尾**（`B0 FD`@$F282 = BCS $F281 共享 F279 族 RTS；`B0 21`@$F2ED 与
   `90 15`@$F2F9 双双汇合 $F310 RTS），流跨 seed 岛标注三次——岛边界不可作语义边界（§9）。
2. **$F752-$F859 / $F870-$F93A**（264B/203B，多入口多 RTS/JMP）：bank7 敌人/武器行为族大段代码。
3. **$F984-$FA17 / $FA1E-$FA40 / $FF3C-$FF53**：尾段代码族（FF3C 为 RAM 位运算例程）。

## 4. 未决清单（23 岛 / 16449B，B 级线索）——**显式缺口类声明（D7 口径）**

> D7 未覆盖缺口类显式入账：本节 23 岛 =「**解码缺口·待复核**」缺口类的完整登记
> （对拍不证全不全、绿≠完整）。未决不隐式消失、不并入 S/D 任一面分子；
> 逐岛列账如下，收敛须过独立复核后另行登记。

| 岛 | 覆盖(有佐证) | 盲扫 | 要点 |
|---|---|---|---|
| bank05:$8D3A-$BFFF | 0.003 | 0 | 13KB 图材大岛内 1 流穿过（banks0-6 与 bank7 不同质，见 §10） |
| bank06:$8C84-$8CFA | 0.008 | 0 | 同上 |
| $D0B5-$D146 | 0.301 | 0 | 混合区 |
| $D150-$D1ED | 0.456 | 0 | 混合区 |
| $D2E3-$D2EA | 0 | 0.50 | 盲扫巧合流（8B） |
| $D3A8-$D4BE | 0.591 | 0 | 代码+表混合 |
| $D51F-$D5E3 | 0.599 | 0 | 代码+表混合 |
| $D65E-$D77F | 0.062 | 0 | 混合区 |
| $D80E-$D92B | 0.224 | 0 | 混合区 |
| $DAD2-$DAD9 | 0.875 | 0 | 强疑代码（差临门一脚） |
| $DC8B-$DC8E | 0 | 0.50 | 盲扫巧合流（4B） |
| $DE35-$DE3C | 0 | 0.50 | 盲扫巧合流（8B） |
| $E55E-$E563 | 0 | 0.50 | 盲扫巧合流（6B） |
| $E770-$E791 | 0.765 | 0 | 强疑代码 |
| $E8F1-$E90E | 0.733 | 0 | 强疑代码 |
| $EB8C-$EC92 | 0.426 | 0 | 代码+表混合 |
| $EDFE-$EEFF | 0.391 | 0 | 代码+表混合 |
| $F3BE-$F74A | 0.666 | 0 | **代码+内联跳转表混合**（`JSR $C27C` 分发表就地内嵌，见 §10） |
| $FA5B-$FA9C | 0.545 | 0 | 混合区 |
| $FAEA-$FAF1 | 0 | 0.50 | 盲扫巧合流（8B） |
| $FB88-$FBDC | 0.776 | 0 | 强疑代码 |
| $FCC1-$FD8E | 0.524 | 0 | 混合区 |
| **$FE79-$FECB** | 0 | **0.904** | **强疑代码**：盲扫流自 $FE81 连贯 35 条至 `JMP $CE92`，
  内含 `JSR $F186`/$F1FA/$FF3C（F1xx 族同域调用），静态无 xref——入口疑走跳转表（§10） |

（$DAD2/$E770/$E8F1/$FB88/$FE79 五岛为人工复核优先队列。）

## 5. 新增调用点清单（bank:addr → target）

全量 **JSR 161 处 / JMP 47 处**（全部 bank7；完整逐条清单见附录 A / 复跑输出）。
F1xx 族窗口（$F0A2-$F344，batch-1 移植直接相关）JSR **33 处**全清单：

```
bank07:$F0A8 → $F152   bank07:$F0C9 → $F140   bank07:$F0CC → $F2CA
bank07:$F0CF → $F1EE   bank07:$F0DA → $CE84   bank07:$F0DF → $F140
bank07:$F0E2 → $F282   bank07:$F0E5 → $F1F4   bank07:$F0EF → $F172
bank07:$F101 → $CE84   bank07:$F106 → $F1B3   bank07:$F109 → $F162
bank07:$F10C → $F1FA   bank07:$F10F → $F2ED   bank07:$F112 → $F1EE
bank07:$F11B → $CE84   bank07:$F120 → $F186   bank07:$F130 → $DC82
bank07:$F133 → $F311   bank07:$F136 → $F1F4   bank07:$F140 → $F1BF
bank07:$F143 → $F186   bank07:$F1B6 → $F1E7   bank07:$F284 → $F2E8
bank07:$F287 → $CD9C   bank07:$F2D2 → $F279   bank07:$F2D5 → $F332
bank07:$F2E8 → $F340   bank07:$F2EF → $F2DF   bank07:$F2F4 → $CD9C
bank07:$F300 → $F332   bank07:$F303 → $F270   bank07:$F332 → $F340
```

对照首案 findings 的调用点并集：全部命中，另**新增** $F0CC→$F2CA、$F0CF→$F1EE、$F0E2→$F282、
$F10F→$F2ED 等（首案清单未列的「已知子程序」调用，本工具全量登记）。
**对 task-1（F1xx 族移植）的直接影响**：F1B3/F1E7/F162/F2CA 族的新入口与调用点已在上表，
移植顺序须补 F2CA 族（九入口共享尾）与 F282/F2ED——依赖链比原计划长。

其余岛 JSR 计数（细节见附录 A）：$F3BE-$F74A 35 处、$D3A8-$D4BE 13 处、$D002-$D094 8 处、
$F870-$F93A 8 处、$D51F-$D5E3 7 处、$EB8C-$EC92 7 处、$F752-$F859 7 处、$F984-$FA17 7 处、…

## 6. S/D 分母影响评估（分母 v2 证据基础；登记归 lead，本报告只出数）

### 6.1 错面格清单（D 面多计=数据区混入代码）

- **错面确定（code 判定，24 格）**：D-REG-07-005 / 006 / 013 / 014 / 016 / 017 / 020 / 022 /
  029 / 030 / 032 / **035** / 036 / 037 / 038 / 039 / 040 / 041 / 042 / 043 / 044 / 045 / 051 / 052。
  ⚠ 首案 findings 只点了 036/038——**035（$F0A2-$F148 岛本体）同样是错面格**，
  且范围远超首案（043 格 $F3BE-$F859 混合区含 264B 纯代码段 $F752-$F859）。
- **疑似（未决判定，22 格）**：D-REG-05-011 / 06-024 / 07-007 / 009 / 011 / 012 / 014 / 015 /
  018 / 019 / 021 / 025 / 027 / 028 / 030 / 031 / 043 / 046 / 047 / 048 / 050 / 051。
- 现矩阵注记：仅 036/038 已注「错面登记」，**其余 22 个确定错面格待注记**（data/status
  归 lead；本工具 `--matrix` 只读输出该清单）。

### 6.2 S 面少计估计（**估计值非定值**，B 级）

- 有佐证岛内入口（被采纳流）不在 450 S-FUNC 分母：**120 个**——**全清单见附录 B**
  （机械生成，逐条 bank:addr，可与 `--json` 的 `denom_impact.s_face_missing_entries` 对账）。
- **估计口径**：入口=sweep 流的起始地址，佐证=fallthrough-from-code / xref-from-code /
  stream-xref 之一；比对对象=S-FUNC-{bank:02d}-{addr:04X} 格 id 全集（矩阵只读）。
- **为何是估计值（B 级，非定值）**：120 ≠ 新函数数——含共享尾/续接入口（如 $F2D5/$F2D8 是
  $F2CA 族内共享尾入口，非独立函数）；抽样核对：F1xx 窗口 33 处 JSR 去重后 20+ 目标
  与 120 清单交集覆盖全部实调目标（指针：附录 A.1 × 附录 B 交叉可核）。
  **保守下界**=被代码区/被采纳流 xref 直接调用的岛内 JSR 去重目标数；**上界**=120。
  分母 v2 重切时应按「入口语义聚类」收口，不能机械照搬 120。
- S 面多计风险同样存在：IDA 机械切分的 450 函数含解码错位产生的假函数（矩阵注记
  「机械切分待纠偏（rounds/r1）」）——v2 重切需双向修正。

### 6.3 分母 v2 建议口径（供 lead 裁定）

1. S/D 重切以 **decode-sweep verdict + 入口佐证** 为输入：code 岛并入 S 面（按入口聚类成
   函数单元），其字节从 D 面剔除；未决岛暂留 D 面但标「疑代码」，复核后再切。
2. 分母导出器仍单向机械（D7 不变），但**来源数据**从「IDA items」升级为
   「IDA items × decode-sweep 校正表」；校正表由本工具 `--json` 机械产出（可复跑）。
3. ratchet 处理：错面修正=分母口径变更（`completeness_denoms.py --force` 级），须单独
   commit 说明（AGENTS §9）；本报告不动产物登记。

## 7. 否定性结论的双工具复核记录（证据铁律，命令与输出原文）

复现命令（链规③：命令逐条原样可跑，数字与命令一并抄录）：

```sh
python3 tools/decode_sweep.py --json /tmp/decode_sweep_run1.json   # exit 0
grep -F 'data 岛' /tmp/decode_sweep_out.txt
```

输出原文：

```
== 否定结论换工具复核（data 岛全偏移盲扫）==
  data 岛 233 个全部经盲扫复核；升级未决 6 个；最大残余覆盖 top5：
  bank07:$FE79=0.90, bank07:$FAEA=0.50, bank07:$E55E=0.50, bank07:$DE35=0.50, bank07:$DC8B=0.50
```

- 233 个 data 岛 = 入口扫描 0 覆盖 **且** 盲扫最大覆盖 <0.5（两法同判）；
- 6 个盲扫命中升级未决（$FE79 0.90 入 §4 强疑；其余 5 个为 2-8B 小岛巧合流）；
- 残余巧合（<0.5）逐岛登记在 `--json` 的 `fail_reasons.bruteforce` 字段，可复核。

## 8. 证据分级汇总

| 结论 | 级 | 依据 |
|---|---|---|
| 29 岛实为代码（含首案复现） | **A** | ROM 原字节自建表解码 + 分支/调用目标核查 + 覆盖率；工具可复跑 |
| 161 JSR / 47 JMP 调用点 | **A** | ROM 原字节扫描（操作码 20/4C + 操作数），逐条 bank:addr |
| 233 岛为数据 | **A-**（否定性，双工具） | 入口扫描 + 全偏移盲扫同判 0 覆盖级命中 |
| 未决 23 岛的「强疑代码」排序 | **B** | 覆盖率+调用语境推断，未定谳 |
| S 面少计=120 | **B** | 入口计数估计，非函数数（§6.2 口径） |
| 跳转表入口推断（FE79 族） | **C** | 假设区，见 §10，不入结论 |

## 9. 教训与自纠实录（环②⑤）

1. **岛边界不是语义边界**：v1 判据「流不得出岛」直接打死了首案邻域的真代码
   （$F282/$F2ED 流跨 seed 标注才到 RTS/JMP）——首案基准 FAIL 教会了这一点。
   数据岛区间标注只当「扫描窗口」用，不当语义围墙。
2. **测试器先过证伪（环⑤）实战抓错**：BRK 拒流只写在 docstring 没写进代码
   （0x00 仍在长度表中被放行）——负对照 $FC5D 指针表的流穿过 `00` 字节漂进相邻代码
   达到 RTS，被 4/4 负对照当场打回（判别力自检=假绿探测器）。修复=显式 `if op == 0x00`
   拒流。教训落知识库：[gotcha-policy-docstring-not-enforced](../knowledge/gotcha-policy-docstring-not-enforced.md)。
3. **共享 RTS 尾 / 跳进操作数**（`B0 FD`→$F281、`B0 21`/`90 15`→$F310）：6502 合法技巧，
   线性流必须允许目标落他流边界/代码区内部，否则系统性误杀真代码（②③ 的弱链分级即为此设）。

## 10. 遗留假设与后续（C 级假设区 + 显式缺口）

1. **跳转表入口未解析（C 级假设）**：`JSR $C27C` 分发模式在返回地址后内嵌地址表
   （$F3BE-$F74A 混合区、$FE79 族均疑似此模式）——本工具不解析该表，导致表驱入口
   无 xref 佐证、只能盲扫命中（$FE79 0.90 即典型）。**后续**：解析 C27C 分发表 →
   表驱入口补佐证 → 未决收敛。
2. **banks0-6 大岛不同质**（D-REG-05-011 $8D3A-$BFFF 13KB 等）：banks0-6 为素材/关卡数据
   bank，seed 线性扫在此大面积失步，岛定义本身质量差；两岛的 0.003/0.008 穿流是噪声。
   **后续**：banks0-6 需按「素材表结构」单独立项考证，不适用本报告的 bank7 结论口径。
3. **判据④（与相邻代码区循环同构）未实现**（可选强判据）：当前 code 判定不依赖它；
   未决岛收敛时可作强判据补强。
4. **未决 23 岛人工复核队列**：优先 $FE79-$FECB / $DAD2-$DAD9 / $E770-$E791 / $E8F1-$E90E /
   $FB88-$FBDC（§4）。
5. 矩阵注记（22 个确定错面格）与分母 v2 登记**归 lead**（写路径互斥，本报告只出数）。
6. task-1（F1xx 族移植）范围需按 §5 扩：新增 F2CA 族九入口、F282、F2ED。

## 附录 A：新增调用点全量（JSR 161 处）

见文末机械生成段（`grep -F 'JSR' docs/findings/decode-sweep-report.md` 可核）；
或复跑 `python3 tools/decode_sweep.py` 的「新增调用点清单」节（输出与本附录同源）。

## 附录 A（机械生成）：新增调用点全量 —— JSR 161 处 / JMP 47 处

生成命令：`python3 tools/decode_sweep.py --json /tmp/decode_sweep_run1.json`（同源输出）

### A.1 JSR 全量（bank:addr → target）

- bank07:$D012 → $D147
- bank07:$D015 → $D055
- bank07:$D018 → $CF2E
- bank07:$D02B → $D09C
- bank07:$D036 → $CFC3
- bank07:$D048 → $CF77
- bank07:$D04D → $CF55
- bank07:$D088 → $CF6B
- bank07:$D194 → $CBA8
- bank07:$D3BA → $D48C
- bank07:$D3BD → $ED65
- bank07:$D3C0 → $95AB
- bank07:$D3C8 → $D92C
- bank07:$D3DD → $CC07
- bank07:$D3F4 → $D147
- bank07:$D400 → $E249
- bank07:$D40E → $E249
- bank07:$D411 → $CDFA
- bank07:$D414 → $CD92
- bank07:$D48F → $D4B4
- bank07:$D498 → $EB85
- bank07:$D49E → $D4A8
- bank07:$D530 → $D5FC
- bank07:$D533 → $D5DA
- bank07:$D560 → $8296
- bank07:$D56A → $832F
- bank07:$D57A → $C3D4
- bank07:$D580 → $D609
- bank07:$D585 → $D624
- bank07:$D609 → $D79F
- bank07:$D614 → $95C0
- bank07:$D61B → $D651
- bank07:$D630 → $E8BE
- bank07:$D633 → $FD8F
- bank07:$D660 → $D2FA
- bank07:$D831 → $E6A7
- bank07:$D837 → $E6D2
- bank07:$D897 → $E4F1
- bank07:$D89D → $E519
- bank07:$D957 → $CF55
- bank07:$D96B → $E252
- bank07:$E789 → $E792
- bank07:$E906 → $E792
- bank07:$EB06 → $EB0C
- bank07:$EB11 → $EB7A
- bank07:$EB19 → $EB28
- bank07:$EB20 → $EB4D
- bank07:$EB56 → $C2C5
- bank07:$EB68 → $EB85
- bank07:$EB74 → $FB7B
- bank07:$EB8C → $EB92
- bank07:$EB96 → $C2C5
- bank07:$EBC1 → $C2C5
- bank07:$EBC8 → $EE41
- bank07:$EBCB → $EE4A
- bank07:$EBD9 → $EB85
- bank07:$EBEE → $EB85
- bank07:$EE11 → $EE3D
- bank07:$EE14 → $EE4A
- bank07:$EE2B → $C2C5
- bank07:$EE33 → $EB85
- bank07:$F0A8 → $F152
- bank07:$F0C9 → $F140
- bank07:$F0CC → $F2CA
- bank07:$F0CF → $F1EE
- bank07:$F0DA → $CE84
- bank07:$F0DF → $F140
- bank07:$F0E2 → $F282
- bank07:$F0E5 → $F1F4
- bank07:$F0EF → $F172
- bank07:$F101 → $CE84
- bank07:$F106 → $F1B3
- bank07:$F109 → $F162
- bank07:$F10C → $F1FA
- bank07:$F10F → $F2ED
- bank07:$F112 → $F1EE
- bank07:$F11B → $CE84
- bank07:$F120 → $F186
- bank07:$F130 → $DC82
- bank07:$F133 → $F311
- bank07:$F136 → $F1F4
- bank07:$F140 → $F1BF
- bank07:$F143 → $F186
- bank07:$F1B6 → $F1E7
- bank07:$F284 → $F2E8
- bank07:$F287 → $CD9C
- bank07:$F2D2 → $F279
- bank07:$F2D5 → $F332
- bank07:$F2E8 → $F340
- bank07:$F2EF → $F2DF
- bank07:$F2F4 → $CD9C
- bank07:$F300 → $F332
- bank07:$F303 → $F270
- bank07:$F332 → $F340
- bank07:$F3CF → $F6B9
- bank07:$F3D2 → $F75B
- bank07:$F3D5 → $F682
- bank07:$F3D8 → $F676
- bank07:$F3E0 → $F7BA
- bank07:$F40A → $F536
- bank07:$F530 → $CBD2
- bank07:$F554 → $F515
- bank07:$F55C → $80B0
- bank07:$F56C → $80B0
- bank07:$F573 → $CBB8
- bank07:$F584 → $CBD6
- bank07:$F589 → $F74B
- bank07:$F598 → $CBD2
- bank07:$F59D → $CBB0
- bank07:$F5A2 → $CBB0
- bank07:$F5A7 → $CBCE
- bank07:$F5AC → $80B0
- bank07:$F5CC → $80B0
- bank07:$F623 → $DCFC
- bank07:$F628 → $F844
- bank07:$F62E → $F68D
- bank07:$F649 → $F68D
- bank07:$F676 → $80B0
- bank07:$F682 → $F844
- bank07:$F699 → $F844
- bank07:$F6A4 → $F68D
- bank07:$F6BD → $F74B
- bank07:$F6E2 → $F7B3
- bank07:$F6EB → $F752
- bank07:$F702 → $F744
- bank07:$F70C → $81DA
- bank07:$F711 → $F7B3
- bank07:$F71A → $F752
- bank07:$F735 → $F744
- bank07:$F761 → $F744
- bank07:$F7A7 → $C3D4
- bank07:$F7D1 → $CBA4
- bank07:$F7D9 → $F699
- bank07:$F7FC → $80B0
- bank07:$F82C → $81D1
- bank07:$F836 → $8206
- bank07:$F887 → $F8E8
- bank07:$F88F → $F92C
- bank07:$F8A2 → $F68D
- bank07:$F8BD → $F8F8
- bank07:$F8D3 → $F91D
- bank07:$F8F0 → $F1EE
- bank07:$F91F → $E792
- bank07:$F924 → $FD8F
- bank07:$F96A → $FA5B
- bank07:$F96D → $FA1E
- bank07:$F979 → $FA60
- bank07:$F991 → $FA5B
- bank07:$F9BE → $8296
- bank07:$F9C8 → $832F
- bank07:$F9D9 → $C3D4
- bank07:$F9F2 → $FA1E
- bank07:$FA0A → $F68D
- bank07:$FA0D → $F518
- bank07:$FA2A → $FA7D
- bank07:$FB8A → $F844
- bank07:$FB9C → $C3D4
- bank07:$FCC1 → $FD5F
- bank07:$FCED → $FD8F
- bank07:$FCFA → $FD6C
- bank07:$FF4E → $C2C6

### A.2 JMP(abs) 全量

- bank07:$D039 → $CD6B
- bank07:$D052 → $CD6D
- bank07:$D07A → $D088
- bank07:$D0D8 → $CBEF
- bank07:$D406 → $D411
- bank07:$D417 → $DE4A
- bank07:$D552 → $D522
- bank07:$D588 → $D4F8
- bank07:$D606 → $E81E
- bank07:$D621 → $E9BE
- bank07:$D66D → $DE47
- bank07:$D821 → $D806
- bank07:$D834 → $D806
- bank07:$D83A → $D806
- bank07:$D89A → $D806
- bank07:$D8A0 → $D806
- bank07:$D970 → $D806
- bank07:$D973 → $D892
- bank07:$E778 → $E783
- bank07:$E78F → $E7BD
- bank07:$E8F9 → $E904
- bank07:$E90C → $E89B
- bank07:$EB09 → $EAD4
- bank07:$EB77 → $F85A
- bank07:$EB8F → $EAD4
- bank07:$EE47 → $DC67
- bank07:$F0EA → $F09F
- bank07:$F13D → $CE92
- bank07:$F146 → $F1FA
- bank07:$F16F → $F193
- bank07:$F1BC → $F1CB
- bank07:$F335 → $C2C6
- bank07:$F3E3 → $F40A
- bank07:$F40D → $95E5
- bank07:$F533 → $F673
- bank07:$F568 → $95A8
- bank07:$F576 → $F42D
- bank07:$F741 → $820F
- bank07:$F82F → $F839
- bank07:$F83B → $F7B3
- bank07:$F8E5 → $F690
- bank07:$F927 → $FA49
- bank07:$F9B0 → $F987
- bank07:$FA15 → $F97C
- bank07:$FA8E → $E96A
- bank07:$FBBC → $F693
- bank07:$FBC7 → $F68D

## 附录 B（机械生成）：S 面少计估计的有佐证入口全清单（120 个，估计值非定值）

口径：被采纳流入口中带佐证（fallthrough-from-code / xref-from-code / stream-xref）且不在 S-FUNC 450 分母者；**估计值**——含共享尾/续接入口，非独立函数数（见 §6.2）。

- bank05:$8D3A
- bank06:$8C84
- bank07:$CFC3
- bank07:$D002
- bank07:$D03C
- bank07:$D055
- bank07:$D07D
- bank07:$D088
- bank07:$D092
- bank07:$D0B5
- bank07:$D0DB
- bank07:$D150
- bank07:$D3A8
- bank07:$D409
- bank07:$D411
- bank07:$D48C
- bank07:$D4B4
- bank07:$D51F
- bank07:$D555
- bank07:$D5DA
- bank07:$D5FC
- bank07:$D602
- bank07:$D606
- bank07:$D609
- bank07:$D624
- bank07:$D65E
- bank07:$D78B
- bank07:$D80E
- bank07:$D824
- bank07:$D837
- bank07:$D892
- bank07:$D89D
- bank07:$D94E
- bank07:$D973
- bank07:$DA39
- bank07:$DAD2
- bank07:$DCB7
- bank07:$E000
- bank07:$E770
- bank07:$E783
- bank07:$E8F1
- bank07:$E904
- bank07:$EB06
- bank07:$EB0C
- bank07:$EB4D
- bank07:$EB7A
- bank07:$EB8C
- bank07:$EB92
- bank07:$EDFE
- bank07:$EE41
- bank07:$EE4A
- bank07:$EF9A
- bank07:$F0A2
- bank07:$F0ED
- bank07:$F162
- bank07:$F1B3
- bank07:$F1CB
- bank07:$F1E7
- bank07:$F270
- bank07:$F282
- bank07:$F2AD
- bank07:$F2CA
- bank07:$F2D5
- bank07:$F2D8
- bank07:$F2DF
- bank07:$F2E8
- bank07:$F2ED
- bank07:$F332
- bank07:$F340
- bank07:$F3BE
- bank07:$F40A
- bank07:$F42D
- bank07:$F515
- bank07:$F518
- bank07:$F530
- bank07:$F536
- bank07:$F56B
- bank07:$F56C
- bank07:$F579
- bank07:$F673
- bank07:$F676
- bank07:$F682
- bank07:$F68D
- bank07:$F690
- bank07:$F693
- bank07:$F699
- bank07:$F6B8
- bank07:$F6B9
- bank07:$F744
- bank07:$F74A
- bank07:$F752
- bank07:$F75B
- bank07:$F7B3
- bank07:$F7BA
- bank07:$F7CA
- bank07:$F832
- bank07:$F839
- bank07:$F844
- bank07:$F870
- bank07:$F8E8
- bank07:$F8F7
- bank07:$F8F8
- bank07:$F91D
- bank07:$F92C
- bank07:$F96A
- bank07:$F97C
- bank07:$F984
- bank07:$F9B3
- bank07:$FA1E
- bank07:$FA5B
- bank07:$FA60
- bank07:$FA6A
- bank07:$FA7D
- bank07:$FB88
- bank07:$FBBF
- bank07:$FCC1
- bank07:$FD5F
- bank07:$FD6C
- bank07:$FE59
- bank07:$FF3C
