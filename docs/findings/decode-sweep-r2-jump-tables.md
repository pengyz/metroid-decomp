# findings：decode-sweep r2 深挖——C27C 跳转表入口 + 混合岛定谳（task-6 ①②③）

> 状态：**r2 已证实**（2026-10-15，ROM 原字节级）。正/负对照基准维持 **BASELINE: PASS**。
> 前篇：[decode-sweep-report.md](decode-sweep-report.md)（首轮，数字以本篇为准——前篇头部有更正注）。
> 复跑：`python3 tools/decode_sweep.py --json /tmp/decode_sweep_run2.json`（幂等只读）。

## 0. r2 方法增量（A 级，全部落工具可复跑）

1. **C27C 分发器语义**（bank7:$C27C 字节级解码，B 级语义推断）：
   `STY $6A/STX $69` 保存寄存器 → `PLA/PLA` 取 JSR 返回址-1 → `LDA (ptr),Y`
   （Y=调用者 A+1）读表低字节 → `TAX/INY` 读高字节 → `LDX/LDY` 恢复 → `JMP ($000C)`。
   ⇒ **表词 = site+3+2×index**；`$C45C = RTS`（共享空操作 handler，多站点 #0 复现）。
2. **跳转表入口发现**：扫 ROM 字节 `20 7C C2` 站点，逐词取表。表长静态未知，**双规则收口**：
   ①前缀规则（首个越窗/流不成立的词即表尾）；②结构规则（词表值命中表尾地址
   =表后紧跟内联 handler 首址——EA19/FC9A/FE2E 三站点实证）。
   *自纠实录*：r2 首版只有前缀规则，过读 $EA19#5=$FFA9（handler 字节当词读）把
   负对照 $FF86 洗成未决——**负对照第二次当场抓错**，结构规则即为其解。
3. **未覆盖段数据消费者核查**：未覆盖字节若被 abs/abx/aby 读改写指令命中 ⇒
   「被代码消费的内嵌表」（B 级定谳依据）；同时单列 C27C 表字节段（间接消费，
   直接扫描看不见——教训 §7.2）。
4. **P2-F1**：佐证类型四分类（fallthrough / xref / stream-xref / jump-table）入
   JSON `s_face_missing_detail` 与附录 A；**P2-F2**：一切记录按 bank 索引（镜像岛教训）。

## 1. r2 结论（输出原文）

```
[decode-sweep] verdict：code=35  data=233  未决=17（岛 285；被采纳流 178，被拒 240，入口候选 469；盲扫升级 5；C27C 跳转表入口 118）
  正对照 bank07:$F0A2-$F148 所在岛 → code ✓  9/9 调用点复现 ✓
  负对照 4/4 所在岛 → data ✓（判别力：已知缺口 vs 已知数据全对）
BASELINE: PASS
```

verdict 迁移（r1→r2，6 岛 未决→code，跳转表佐证立功）：

| 岛 | 大小 | r1 | r2 | 关键新佐证 |
|---|---|---|---|---|
| $D65E-$D77F | 290B | 未决 | code(1.0) | jump-table@D4D0#6/7/9/10 |
| $D80E-$D92B | 286B | 未决 | code(1.0) | jump-table@D7BA#3-8 |
| $EB8C-$EC92 | 263B | 未决 | code(0.97) | jump-table@EAE1#4/6/7 |
| $EDFE-$EEFF | 258B | 未决 | code(1.0) | jump-table@EDDF#3-10 |
| **$F3BE-$F74A** | 909B | 未决 | **code(0.993)** | jump-table@F36B#1-6（见 §4） |
| $FA5B-$FA9C | 66B | 未决 | code(1.0) | jump-table@F95A#4/5 |

## 2. C27C 跳转表证据（A 级：ROM 字节 + 表词清单）

14 站点 118 词（全清单附录 B）。bank7 站点：C13E(10) CC7A(5) D1D4(10) D4D0(12)
D7BA(10) EA19(5) EAE1(9) EDDF(11) F36B(7) F95A(6) FC9A(4) FE2E(6)；
banks0-6 站点：bank00 $8027(16)/$9AC2(7)。结构规则收口实例：

```
$EA19: 20 7c c2 | 5c c4 e2 e5 | 5c c4 e2 e5 | 26 ea | a9 ff 85 5a 60
       JSR 分发   表#0=C45C #1=E5E2 #2=C45C #3=E5E2 #4=EA26 → 表尾 EA26=A9 FF(内联 handler 首址)
$FC9A: 20 7c c2 | 5c c4 a5 fc b1 fc ba fc | 20 84 fd ...
       表#0..#3=C45C/FCA5/FCB1/FCBA → 表尾 FCA5=20 84 fd(内联 handler 首址)
```

## 3. FE79-FECB 定谳（task-6 ①）——**混合岛：代码段+内嵌表段**

| 段 | 判定 | 证据 |
|---|---|---|
| **$FE83-$FECA 代码**【2026-10-15 更正·边界】实为 **$FE83-$FECB（73B/34 条**——~~35 条~~；`JMP $CE92`（4C 92 CE）跨 $FEC9-$FECB 切末字节，m1disasm GetVRAMStringPtr:$FECC 佐证）| **A 级确定** | 入口=jump-table@FE2E#5（表词清单 A 级）；34 条连贯流至 `JMP $CE92`；调用 `JSR $F186`/$F1FA/$FF3C/**$F311**（F1xx 族同域；~~三员~~【更正】实为四员，漏 F311） |
| **$FE79-$FE82 表**（10B） | **B 级确定** | 直接消费者 `$FE6A:LDA`（ROM 字节 `B9 79 FE` = LDA $FE79,Y）——被代码当表读 |

r1 的「盲扫 0.904 强疑」假设闭合：入口确实是跳转表分发（C 级假设→A 级证实）。
**分母 v2 输入**：~~FE83-$FECA~~【边界更正】**$FE83-$FECB** 入 S 面（函数单元，入口 FE83），FE79-$FE82 留 D 面
（字段切分归 S3）。前篇 §4「0.904 强疑（B 级）」以此为准（更正注已加）。

## 4. F3BE-F74A 定谳（task-6 ②）——r1「内联跳转表混合区」担忧基本解除

- verdict=**code（0.993）**，33 个佐证入口（jump-table@F36B#1-6 + stream-xref 交织，
  与 F752/F870/F984/FB88 各族互调）；
- r1 担忧的「`JSR $C27C` 分发表就地内嵌」**不成立于岛内**：F36B 表在种子 code 区，
  岛内字节 99.3% 为指令流；
- 未覆盖仅 **$F438-$F43D（6B，无直接消费者、非表字节）**——小尾巴待查，不影响定谳。

## 5. 22 疑似格定谳（task-6 ③）——分解为 5 类

| 类 | 格（逐 id） | 定谳依据 |
|---|---|---|
| **迁「错面确定」（4）** | D-REG-07-015 / 031 / 043 / 046 | 对应岛 r2 code（§1 迁移表） |
| **数据面确认（5）** | D-REG-07-009 / 019 / 021 / 025 / 047 | 小岛盲扫 0.5=巧合流；未覆盖段**有直接消费者**（如 $D2E3 表消费者 $D2A0:LDA/$D2AB:LDA/$D2CE:LDY）=被代码读的真表 |
| **混合定谳（1）** | D-REG-07-051 | §3：FE83 代码段+FE79 表段，按段重切 |
| **基本定谳代码（1）** | D-REG-07-018 | $DAD2-$DAD9 cov 0.875，尾 1B（$DAD9）待查 |
| **残余待查（9，D7 显式缺口类）** | D-REG-05-011 / 06-024 / 07-007 / 011 / 012 / 027 / 028 / 048 / 050 | banks0-6 大岛不同质（05-011/06-024，前篇 §10.2 口径不变）；007/011/012 混合区未覆盖段无消费者；027（$E77B-$E782）/028（$E8FC-$E903）/048（$FBCA-$FBDC）/050（$FD08-$FD5E,$FD84-$FD8E）未覆盖段无直接消费者亦非 C27C 表字节——待查（§8） |

错面确定随 r2 增至 **27 格**（r1 24 + 015/031/043/046）：
D-REG-07-005/006/013/014/015/016/017/020/022/029/030/031/032/035/036/037/038/039/040/
041/042/043/044/045/046/051/052（051 按段重切）。

## 6. S/D 分母 r2 输入（估计值口径不变；P2-F1 佐证类型入账）

- S 面少计 r2=**176 个有佐证入口**（附录 A 逐条+source_types）：stream-xref 120、
  fallthrough 45、jump-table 41、xref 1（一入口多源计多类）；
- **估计值非定值**口径同前篇 §6.2（共享尾/续接入口不等于函数数）；
- **重切原则细化（v2.1）**：混合岛按 §3/§5 的段级证据切分（代码段入 S、表段留 D），
  未覆盖段三分类（内嵌表/表字节/待查）作为切分输入——登记归 lead（task-5）。

## 7. 教训（r2 新增两条）

1. **表长过读与结构收口**：前缀规则会被「垃圾词恰好流成立」击穿（$EA19#5=$FFA9
   洗白负对照 $FF86）；结构规则（表尾=内联 handler 首址）+负对照联防才收口。
   与前篇 §9.2「BRK 政策未落码」同族：**判别力要靠坏例联防持续证明**。
2. **「无直接消费者 ≠ 无消费」**：C27C 表经 `(ptr),Y` 间接读，abs 系消费者扫描看不见
   ⇒ 表字节段必须按「站点+3 起 2n 字节」单独登记（§0.3 单列项即为此设）。

## 8. 遗留（C 级假设区 + 显式待查）

1. 9 个残余待查格（§5）+ 小尾巴段（$F438-$F43D/$F83E-$F843/$F852-$F859/$F92A-$F92B/
   $EBFC-$EC03/$DAD9）——「待查」为 D7 显式缺口类，非隐式遗漏；
2. 非 C27C 模式的其它分发器未解析（如 $C13E 表所在的分发上下文、banks0-6 站点）；
3. banks0-6 大岛（05-011/06-024）按前篇 §10.2 单独立项；
4. 判据④（循环同构）仍未实现；跳转表 A 值范围（表长真值）静态未知——双规则是
   上界近似，动态 trace（Mesen2）可作最终收口（后续）。

## 附录 A（机械生成）：S 面少计 176 条 + 佐证类型（P2-F1）

生成命令：`python3 tools/decode_sweep.py --json /tmp/decode_sweep_run2.json`
（JSON 对账键：`denom_impact.s_face_missing_detail`；**估计值非定值**，口径见 §6）。

## 附录 B（机械生成）：C27C 跳转表 14 站点 118 词全清单

生成命令同上（JSON 对账键：`jump_table_entries`）。

### A.1 全清单（entry | source_types | sources）

- bank05:$8D3A  [fallthrough+island-start]  fallthrough-from-code@8D39; island-start
- bank06:$8C84  [fallthrough+island-start]  fallthrough-from-code@8C82; island-start
- bank07:$CFC3  [fallthrough+island-start+jump-table+xref]  fallthrough-from-code@CFC0; island-start; jump-table@CC7A#2; xref-from-code@CD40
- bank07:$D002  [fallthrough+island-start]  fallthrough-from-code@D001; island-start
- bank07:$D03C  [stream-xref]  stream-xref-from-D002
- bank07:$D055  [stream-xref]  stream-xref-from-D002
- bank07:$D07D  [stream-xref]  stream-xref-from-D055
- bank07:$D088  [stream-xref]  stream-xref-from-D055
- bank07:$D092  [stream-xref]  stream-xref-from-D055
- bank07:$D0B5  [fallthrough+island-start+jump-table]  fallthrough-from-code@D0B2; island-start; jump-table@CC7A#3; jump-table@D1D4#3
- bank07:$D0DB  [stream-xref]  stream-xref-from-D0B5
- bank07:$D150  [fallthrough+island-start]  fallthrough-from-code@D14F; island-start
- bank07:$D3A8  [fallthrough+island-start]  fallthrough-from-code@D3A5; island-start
- bank07:$D409  [stream-xref]  stream-xref-from-D3A8
- bank07:$D411  [stream-xref]  stream-xref-from-D3A8
- bank07:$D48C  [stream-xref]  stream-xref-from-D3A8
- bank07:$D4B4  [stream-xref]  stream-xref-from-D48C
- bank07:$D51F  [fallthrough+island-start+stream-xref]  fallthrough-from-code@D51E; island-start; stream-xref-from-D555
- bank07:$D522  [stream-xref]  stream-xref-from-D52C
- bank07:$D52C  [jump-table]  jump-table@D4D0#2
- bank07:$D555  [stream-xref]  stream-xref-from-D51F; stream-xref-from-D522; stream-xref-from-D52C
- bank07:$D5C1  [jump-table]  jump-table@D4D0#3
- bank07:$D5C8  [jump-table]  jump-table@D4D0#4
- bank07:$D5DA  [stream-xref]  stream-xref-from-D51F; stream-xref-from-D522; stream-xref-from-D52C
- bank07:$D5FC  [fallthrough+island-start+stream-xref]  fallthrough-from-code@D5FB; island-start; stream-xref-from-D51F; stream-xref-from-D522; stream-xref-from-D52C
- bank07:$D602  [stream-xref]  stream-xref-from-D624
- bank07:$D606  [stream-xref]  stream-xref-from-D609
- bank07:$D609  [stream-xref]  stream-xref-from-D555
- bank07:$D624  [stream-xref]  stream-xref-from-D555; stream-xref-from-D609
- bank07:$D65E  [fallthrough+island-start+jump-table]  fallthrough-from-code@D65D; island-start; jump-table@D4D0#5; jump-table@D4D0#8
- bank07:$D66B  [stream-xref]  stream-xref-from-D670; stream-xref-from-D691
- bank07:$D670  [jump-table]  jump-table@D4D0#6; jump-table@D4D0#9
- bank07:$D691  [jump-table]  jump-table@D4D0#10; jump-table@D4D0#7
- bank07:$D6A7  [stream-xref]  stream-xref-from-D691
- bank07:$D6E7  [stream-xref]  stream-xref-from-D6E8
- bank07:$D6E8  [stream-xref]  stream-xref-from-D6A7
- bank07:$D715  [stream-xref]  stream-xref-from-D6E8
- bank07:$D741  [stream-xref]  stream-xref-from-D715
- bank07:$D76A  [stream-xref]  stream-xref-from-D6A7; stream-xref-from-D6E8; stream-xref-from-D715
- bank07:$D77E  [stream-xref]  stream-xref-from-D715
- bank07:$D77F  [stream-xref]  stream-xref-from-D6E8; stream-xref-from-D715; stream-xref-from-D741
- bank07:$D78B  [fallthrough+island-start+stream-xref]  fallthrough-from-code@D788; island-start; stream-xref-from-D6A7; stream-xref-from-D715; stream-xref-from-D741
- bank07:$D80E  [fallthrough+island-start+jump-table]  fallthrough-from-code@D80B; island-start; jump-table@D7BA#2
- bank07:$D824  [stream-xref]  stream-xref-from-D80E
- bank07:$D837  [stream-xref]  stream-xref-from-D824
- bank07:$D83D  [jump-table]  jump-table@D7BA#3; jump-table@D7BA#8
- bank07:$D854  [stream-xref]  stream-xref-from-D83D
- bank07:$D866  [stream-xref]  stream-xref-from-D83D
- bank07:$D870  [jump-table]  jump-table@D7BA#4
- bank07:$D892  [stream-xref]  stream-xref-from-D870; stream-xref-from-D973
- bank07:$D89D  [stream-xref]  stream-xref-from-D892
- bank07:$D8A3  [jump-table]  jump-table@D7BA#5; jump-table@D7BA#7
- bank07:$D8BA  [stream-xref]  stream-xref-from-D8A3
- bank07:$D8BF  [jump-table]  jump-table@D7BA#6
- bank07:$D8DA  [stream-xref]  stream-xref-from-D8BF
- bank07:$D94E  [fallthrough+island-start+jump-table]  fallthrough-from-code@D94D; island-start; jump-table@D7BA#9
- bank07:$D973  [stream-xref]  stream-xref-from-D94E
- bank07:$DA39  [fallthrough+island-start]  fallthrough-from-code@DA36; island-start
- bank07:$DAD2  [fallthrough+island-start]  fallthrough-from-code@DAD1; island-start
- bank07:$DCB7  [fallthrough+island-start]  fallthrough-from-code@DCB4; island-start
- bank07:$E000  [fallthrough+island-start]  fallthrough-from-code@DFFE; island-start
- bank07:$E770  [fallthrough+island-start]  fallthrough-from-code@E76F; island-start
- bank07:$E783  [stream-xref]  stream-xref-from-E770
- bank07:$E8F1  [fallthrough+island-start]  fallthrough-from-code@E8F0; island-start
- bank07:$E904  [stream-xref]  stream-xref-from-E8F1
- bank07:$EB06  [fallthrough+island-start+jump-table]  fallthrough-from-code@EB03; island-start; jump-table@EAE1#1
- bank07:$EB0C  [stream-xref]  stream-xref-from-EB06
- bank07:$EB4D  [fallthrough+island-start+stream-xref]  fallthrough-from-code@EB4C; island-start; stream-xref-from-EB0C
- bank07:$EB7A  [stream-xref]  stream-xref-from-EB0C
- bank07:$EB8C  [fallthrough+island-start+jump-table]  fallthrough-from-code@EB8B; island-start; jump-table@EAE1#2
- bank07:$EB8F  [stream-xref]  stream-xref-from-EC04
- bank07:$EB92  [stream-xref]  stream-xref-from-EB8C; stream-xref-from-EEF4
- bank07:$EC04  [jump-table]  jump-table@EAE1#4
- bank07:$EC09  [stream-xref]  stream-xref-from-EEA1
- bank07:$EC2F  [jump-table]  jump-table@EAE1#6
- bank07:$EC54  [stream-xref]  stream-xref-from-EC57
- bank07:$EC57  [jump-table]  jump-table@EAE1#7
- bank07:$EDFE  [fallthrough+island-start+jump-table]  fallthrough-from-code@EDFC; island-start; jump-table@EDDF#2
- bank07:$EE41  [stream-xref]  stream-xref-from-EB92
- bank07:$EE4A  [stream-xref]  stream-xref-from-EB92; stream-xref-from-EDFE; stream-xref-from-EEAE; stream-xref-from-EEC6; stream-xref-from-EECA
- bank07:$EE63  [jump-table]  jump-table@EDDF#3
- bank07:$EE83  [stream-xref]  stream-xref-from-EEA1
- bank07:$EE86  [stream-xref]  stream-xref-from-EE63
- bank07:$EEA1  [jump-table]  jump-table@EDDF#4
- bank07:$EEA6  [jump-table]  jump-table@EDDF#5
- bank07:$EEAB  [stream-xref]  stream-xref-from-EEAE; stream-xref-from-EEC6
- bank07:$EEAE  [jump-table]  jump-table@EDDF#6
- bank07:$EEC6  [stream-xref]  stream-xref-from-EECA; stream-xref-from-EEEE; stream-xref-from-EEFA
- bank07:$EECA  [jump-table]  jump-table@EDDF#7
- bank07:$EEEE  [jump-table]  jump-table@EDDF#8
- bank07:$EEF4  [jump-table]  jump-table@EDDF#9
- bank07:$EEFA  [jump-table]  jump-table@EDDF#10
- bank07:$EF9A  [fallthrough+island-start]  fallthrough-from-code@EF99; island-start
- bank07:$F0A2  [fallthrough+island-start+jump-table]  fallthrough-from-code@F09F; island-start; jump-table@EAE1#8
- bank07:$F0ED  [stream-xref]  stream-xref-from-F0A2
- bank07:$F162  [fallthrough+island-start+stream-xref]  fallthrough-from-code@F15F; island-start; stream-xref-from-F0ED
- bank07:$F270  [fallthrough+island-start+stream-xref]  fallthrough-from-code@F26F; island-start; stream-xref-from-F2E8; stream-xref-from-F2ED
- bank07:$F282  [fallthrough+island-start+stream-xref]  fallthrough-from-code@F281; island-start; stream-xref-from-F0A2
- bank07:$F2AD  [fallthrough+island-start+stream-xref]  fallthrough-from-code@F2AA; island-start; stream-xref-from-F282
- bank07:$F2CA  [fallthrough+island-start+stream-xref]  fallthrough-from-code@F2C9; island-start; stream-xref-from-F0A2
- bank07:$F2D5  [stream-xref]  stream-xref-from-F2AD
- bank07:$F2D8  [stream-xref]  stream-xref-from-F2E8
- bank07:$F2DF  [stream-xref]  stream-xref-from-F2E8; stream-xref-from-F2ED
- bank07:$F2E8  [stream-xref]  stream-xref-from-F282
- bank07:$F2ED  [stream-xref]  stream-xref-from-F0ED
- bank07:$F332  [fallthrough+island-start+stream-xref]  fallthrough-from-code@F32F; island-start; stream-xref-from-F2CA; stream-xref-from-F2D5; stream-xref-from-F2E8; stream-xref-from-F2ED
- bank07:$F340  [fallthrough+island-start+stream-xref]  fallthrough-from-code@F33D; island-start; stream-xref-from-F2E8; stream-xref-from-F332
- bank07:$F3BE  [fallthrough+island-start+jump-table]  fallthrough-from-code@F3BD; island-start; jump-table@F36B#1
- bank07:$F3E6  [jump-table]  jump-table@F36B#2
- bank07:$F40A  [stream-xref]  stream-xref-from-F3BE
- bank07:$F40D  [jump-table]  jump-table@F36B#3
- bank07:$F410  [stream-xref]  stream-xref-from-F43E
- bank07:$F416  [stream-xref]  stream-xref-from-F43E; stream-xref-from-F4D4
- bank07:$F423  [stream-xref]  stream-xref-from-F4EE
- bank07:$F42D  [stream-xref]  stream-xref-from-F56C
- bank07:$F43E  [jump-table]  jump-table@F36B#4
- bank07:$F483  [jump-table]  jump-table@F36B#5
- bank07:$F4B7  [stream-xref]  stream-xref-from-F483
- bank07:$F4D4  [stream-xref]  stream-xref-from-F483
- bank07:$F4EE  [jump-table]  jump-table@F36B#6
- bank07:$F515  [stream-xref]  stream-xref-from-F4EE; stream-xref-from-F536
- bank07:$F518  [stream-xref]  stream-xref-from-F9B3
- bank07:$F51E  [stream-xref]  stream-xref-from-F3E6
- bank07:$F530  [stream-xref]  stream-xref-from-F56C
- bank07:$F536  [stream-xref]  stream-xref-from-F3E6; stream-xref-from-F40A; stream-xref-from-F43E
- bank07:$F56B  [stream-xref]  stream-xref-from-F51E; stream-xref-from-F536
- bank07:$F56C  [stream-xref]  stream-xref-from-F579
- bank07:$F579  [stream-xref]  stream-xref-from-F536
- bank07:$F673  [stream-xref]  stream-xref-from-F530
- bank07:$F676  [stream-xref]  stream-xref-from-F3BE
- bank07:$F682  [stream-xref]  stream-xref-from-F3BE
- bank07:$F68D  [stream-xref]  stream-xref-from-F579; stream-xref-from-F699; stream-xref-from-F870; stream-xref-from-F9B3; stream-xref-from-FA91; stream-xref-from-FBBF
- bank07:$F690  [stream-xref]  stream-xref-from-F870
- bank07:$F693  [stream-xref]  stream-xref-from-FB88
- bank07:$F699  [stream-xref]  stream-xref-from-F7CA
- bank07:$F6B8  [stream-xref]  stream-xref-from-F6B9
- bank07:$F6B9  [stream-xref]  stream-xref-from-F3BE; stream-xref-from-F3E6
- bank07:$F744  [stream-xref]  stream-xref-from-F6B9; stream-xref-from-F75B
- bank07:$F74A  [stream-xref]  stream-xref-from-F6B9
- bank07:$F752  [fallthrough+island-start+stream-xref]  fallthrough-from-code@F751; island-start; stream-xref-from-F6B9
- bank07:$F75B  [stream-xref]  stream-xref-from-F3BE; stream-xref-from-F3E6
- bank07:$F7B3  [stream-xref]  stream-xref-from-F6B9; stream-xref-from-F832; stream-xref-from-F839
- bank07:$F7BA  [stream-xref]  stream-xref-from-F3BE
- bank07:$F7CA  [stream-xref]  stream-xref-from-F7BA
- bank07:$F832  [stream-xref]  stream-xref-from-F7CA
- bank07:$F839  [stream-xref]  stream-xref-from-F7CA
- bank07:$F844  [stream-xref]  stream-xref-from-F579; stream-xref-from-F682; stream-xref-from-F699; stream-xref-from-FB88
- bank07:$F870  [fallthrough+island-start]  fallthrough-from-code@F86F; island-start
- bank07:$F8E8  [stream-xref]  stream-xref-from-F870
- bank07:$F8F7  [stream-xref]  stream-xref-from-F870
- bank07:$F8F8  [stream-xref]  stream-xref-from-F870
- bank07:$F91D  [stream-xref]  stream-xref-from-F870
- bank07:$F92C  [stream-xref]  stream-xref-from-F870
- bank07:$F96A  [fallthrough+island-start+jump-table]  fallthrough-from-code@F969; island-start; jump-table@F95A#1
- bank07:$F97C  [stream-xref]  stream-xref-from-F9B3; stream-xref-from-FA6B; stream-xref-from-FA91
- bank07:$F984  [fallthrough+island-start+stream-xref]  fallthrough-from-code@F981; island-start; stream-xref-from-F9B3
- bank07:$F987  [stream-xref]  stream-xref-from-F991
- bank07:$F991  [jump-table]  jump-table@F95A#2
- bank07:$F9B3  [stream-xref]  stream-xref-from-F984; stream-xref-from-F987; stream-xref-from-F991
- bank07:$FA1E  [fallthrough+island-start+stream-xref]  fallthrough-from-code@FA1D; island-start; stream-xref-from-F96A; stream-xref-from-F9B3
- bank07:$FA5B  [fallthrough+island-start+stream-xref]  fallthrough-from-code@FA5A; island-start; stream-xref-from-F96A; stream-xref-from-F984; stream-xref-from-F987; stream-xref-from-F991
- bank07:$FA60  [stream-xref]  stream-xref-from-F96A
- bank07:$FA6A  [stream-xref]  stream-xref-from-F9B3
- bank07:$FA6B  [jump-table]  jump-table@F95A#4
- bank07:$FA7D  [stream-xref]  stream-xref-from-FA1E
- bank07:$FA91  [jump-table]  jump-table@F95A#5
- bank07:$FB88  [fallthrough+island-start]  fallthrough-from-code@FB87; island-start
- bank07:$FBBF  [stream-xref]  stream-xref-from-FB88
- bank07:$FCC1  [fallthrough+island-start]  fallthrough-from-code@FCBE; island-start
- bank07:$FD5F  [stream-xref]  stream-xref-from-FCC1
- bank07:$FD6C  [stream-xref]  stream-xref-from-FCC1
- bank07:$FE59  [fallthrough+island-start+jump-table]  fallthrough-from-code@FE56; island-start; jump-table@FE2E#3
- bank07:$FE83  [jump-table]  jump-table@FE2E#5
- bank07:$FF3C  [fallthrough+island-start+stream-xref]  fallthrough-from-code@FF3B; island-start; stream-xref-from-FE83

### B.1 按站点（bank:site ×n → 词表）

- bank00:$8027 ×16  #0=$8071 #1=$80D0 #2=$80F9 #3=$812C #4=$8142 #5=$8109 #6=$814D #7=$8163 #8=$8182 #9=$81D1 #10=$806E #11=$806E #12=$822E #13=$8243 #14=$8263 #15=$8283
- bank00:$9AC2 ×7  #0=$9AD5 #1=$9B1C #2=$9B34 #3=$9B93 #4=$9BCD #5=$9BFC #6=$9A39
- bank07:$C13E ×10  #0=$C801 #1=$C81D #2=$C8D1 #3=$C92B #4=$C9A6 #5=$C9B1 #6=$C9C4 #7=$C155 #8=$C9D7 #9=$C494
- bank07:$CC7A ×5  #0=$C45C #1=$CC98 #2=$CFC3 #3=$D0B5 #4=$CF77
- bank07:$D1D4 ×10  #0=$CF55 #1=$CC98 #2=$C45C #3=$D0B5 #4=$C45C #5=$C45C #6=$CFBE #7=$C45C #8=$C45C #9=$C45C
- bank07:$D4D0 ×12  #0=$C45C #1=$D4EB #2=$D52C #3=$D5C1 #4=$D5C8 #5=$D65E #6=$D670 #7=$D691 #8=$D65E #9=$D670 #10=$D691 #11=$D4EB
- bank07:$D7BA ×10  #0=$C45C #1=$D7D1 #2=$D80E #3=$D83D #4=$D870 #5=$D8A3 #6=$D8BF #7=$D8A3 #8=$D83D #9=$D94E
- bank07:$EA19 ×5  #0=$C45C #1=$E5E2 #2=$C45C #3=$E5E2 #4=$EA26
- bank07:$EAE1 ×9  #0=$C45C #1=$EB06 #2=$EB8C #3=$C45C #4=$EC04 #5=$C45C #6=$EC2F #7=$EC57 #8=$F0A2
- bank07:$EDDF ×11  #0=$C45C #1=$EDF8 #2=$EDFE #3=$EE63 #4=$EEA1 #5=$EEA6 #6=$EEAE #7=$EECA #8=$EEEE #9=$EEF4 #10=$EEFA
- bank07:$F36B ×7  #0=$C45C #1=$F3BE #2=$F3E6 #3=$F40D #4=$F43E #5=$F483 #6=$F4EE
- bank07:$F95A ×6  #0=$C45C #1=$F96A #2=$F991 #3=$C45C #4=$FA6B #5=$FA91
- bank07:$FC9A ×4  #0=$C45C #1=$FCA5 #2=$FCB1 #3=$FCBA
- bank07:$FE2E ×6  #0=$C45C #1=$FE3D #2=$FE54 #3=$FE59 #4=$FE54 #5=$FE83
