# rounds/r7-decode-sweep-review — task-2 解码缺口扫描器复审账本

> 对象：commit d058c40（tools/decode_sweep.py + docs/findings/decode-sweep-report.md +
> gotcha-policy-docstring-not-enforced）。范围（lead 指定）：①BRK 判据实现与 docstring
> 一致性 ②正/负对照基准自跑 ③S 面 120 估计口径与附录 B 抽核；附带把报告 §5 F1xx 33
> 调用点全清单核成 porter 后续件的可信对照素材。立场：证伪、自重导。

## verdict: pass

工具可复跑（0.7s）、基准可复现且**有真判别力**（BRK 突变当场打回）、报告口径诚实
（B 级估计明标）、F1xx 33 处调用点 33/33 逐字节吻合。两条非阻断 finding（逐条佐证
标注缺口 / bank 镜像检索陷阱记录）不阻断 task-2 完成判据。

## findings（逐项复现输出）

### ① BRK 判据：docstring ↔ 落码一致 + 突变承重实证 —— 通过 ✓

- docstring（tools/decode_sweep.py 头）：「BRK/非法操作码/512 条未终止 = 流不成立」；
  落码 L240-241：`if op == 0x00: sw.fail = "BRK @ …（BRK 不作代码流）"`——一致。
- **突变打回**（/tmp 副本拆 BRK 拒流，ROOT 锚定后）：

```
FAIL 负对照失败：bank07:$FC5D 所在岛 verdict=未决（期望 data）——判别力不足
BASELINE: FAIL
[mutant-exit: 1]
```

  BRK 拒流是承重判据（拆除即翻车），gotcha（政策写 docstring 未落码被负对照当场打回）
  的教训已真整改——非「修复≠声称修复」。
- 附带：正对照 $F0A2-$F148 复现「code ✓ 9/9 调用点 ✓」（其中 $F109 `20 62 F1`=JSR $F162
  与 findings/seed-decode-gap 一致）。

### ② 基准自跑 —— 通过 ✓

```
正对照 bank07:$F0A2-$F148 所在岛 → code ✓  9/9 调用点复现 ✓
负对照 4/4 所在岛 → data ✓（判别力：已知缺口 vs 已知数据全对）
BASELINE: PASS
[exit: 0]（real 0.725s）
```

数字面与 commit Evidence 一致：code=29 data=233 未决=23（岛 285）、S 少计 120、
错面格 [待注记/已注记] 随矩阵实时态变化（队友并行注记，非工具写盘——「只读工具」属实）。

### ③ S 面 120 估计口径 + 附录 B 抽核 —— 通过 ✓（两条记录）

- **附录 B 计数断言 120/120**（唯一计数，前置断言后开核）；口径文字明标：
  「估计值非定值——含共享尾/续接入口，非独立函数数（§6.2，B 级）」——诚实降级到位。
- 分母抽核 4/4：$CFC3/$D002/$D150/$8D3A 均**不在** S-FUNC 450 分母（451 格含补录格）✓。
- 调用点佐证抽核：$CFC3 有 JSR×2（bank7:$CD40/$D036）、$D150 有 JSR×1（$D136）；
  $D002/$8D3A 无 JSR/JMP 直调但属 fallthrough/stream-xref 佐证类（口径明示三类）——
  $8D3A 本尊（bank5）记录核实：`entries:[{entry:$8D3A, justified:true, cover:45}]`、
  `terminators:["$8D66:RTS"]`、`weak_links:["$8D3A:BPL@$8D3B→$8D9D@island"]`——
  流贯至 RTS、佐证实存，非虚报。
- **P2-F2（检索陷阱记录，非缺陷）**：初核时 $8D3A 出现「BRK @ $8D61」疑点——实为
  **bank1-4 同偏移镜像岛**的 fail_reasons（同址不同 bank 内容）；bank 镜像同址=检索陷阱，
  记录以免后续复审重复踩。

### ④ §5 F1xx 33 调用点全清单 —— 33/33 逐字节核 ✓（porter 对照素材已认证）

```
清单对数: 33（前置断言 =33）
字节核: 33/33 = 20 lo hi（JSR→目标）吻合
```

每条 `bank07:$AAAA → $BBBB` 均实测 prg[$AAAA]=0x20 且操作数小端=$BBBB。该清单可作
task-1 后续件（F1B3/F1E7/F162/F2CA/F282/F2ED 等）复审对照素材；报告「F2CA 族九入口
共享尾、依赖链比原计划长」的移植顺序提示与清单自洽。

## 本账 findings 汇总

| # | 分类 | 内容 | 处置 |
|---|---|---|---|
| P2-F1 | 工程卫生（可审性） | 附录 B / JSON `s_face_missing_entries` 无逐条佐证类型标注（fallthrough/xref/stream-xref 不可区分），抽核须回工具内部 | 建议 decode-sweep 输出加 evidence 字段；非阻断 |
| P2-F2 | 检索陷阱记录 | bank1-4 镜像同偏移 fail_reasons 与 bank5 本尊混淆（同址不同内容） | 记录在案（本文件即落点）；后续按 bank 索引取记录 |
| P2-F3 | 复审方自省 | 本复审探针三度自纠：空跑绿（对数 0 打印 0/33 ✓）、range 越界、bank 镜像误读 | 「前置断言门」第三次生效：断言不过不许开核；进方法面遗留 |

## r2 续审（e526ad4，decode-sweep-r2-jump-tables）——2026-10-15

### verdict: pass

### findings（逐项复现输出）

**① 跳转表双规则判别力（突变承重）——通过 ✓**
/tmp 副本拆结构规则（表尾=内联 handler 首址）后立即复现首版事故：

```
FAIL 负对照失败：bank07:$FF86 所在岛 verdict=未决（期望 data）——判别力不足
BASELINE: FAIL
[mutant-exit: 1]
```

结构规则是承重判据；前缀规则+结构规则双收口真实有效（负对照联防第二次立功，
BASELINE 自带判别力）。基准自跑复现：code=35 data=233 未决=17、BASELINE: PASS。

**② FE79-FECB 段级定谳证据——通过 ✓（字节级）**

```
FE2E#5 表词（FE2E+3+2*5=FE3B）: 83 fe     ⇒ $FE83 = 跳转表入口 ✓
$FE6A 消费者: b9 79 fe                     ⇒ LDA $FE79,Y ✓（表被代码消费）
FE79-FE82: 18 1c 20 00 04 08 0c 10 24 14  ⇒ 表词形态
FE83 起: a9 00 9d 00 05 bd…               ⇒ 代码形态（LDA #$00/STA $0500,X…）
```

**③ 051 按段重切边界 + 22 格分解——通过 ✓**
FE83 入 S / FE79-FE82 留 D 的边界证据（表词 vs 代码双形态 + 消费者）字节级成立；
§5 分解 4+5+1+1+9=22 逐 id 有据；「数据面确认 5」消费者证据抽核：$D2A0=`bd e7 d2`、
$D2AB=`bd e9 d2`、$D2CE=`bc e3 d2`——三条**消费站点**对 $D2E3 表区的索引读实存 ✓
（注：我方首读把站点误当目标地址，自查更正——报告写法本身准确）。

**④ 两条新教训——字节级实证 ✓**
- 表长过读结构收口：突变实证如 ①；
- 「无直接消费者≠无消费」：C27C 分发器 `… 68 85 0c 68 85 0d b1 0c …` = PLA/PLA→STA
  $0C/$0D→**LDA ($0C),Y** 间接读表 ✓（表词=site+3+2·index 与工具公式一致；#0 词
  =C45C 共享 RTS handler ✓）。

**P2-F1/P2-F2 落地确认**：附录 A 逐条带 source_types 四分类
（[fallthrough+island-start+jump-table+xref] 及 sources 具体指针）；bank 索引纪律
入 docstring。S 少计 176（估计值口径不变，B 级声明保留）。

## 遗留风险清单

1. S 面 120 为 B 级估计——分母 v2 实际登记（lead task-5）以 decode-sweep 复跑 + 逐条
   佐证标注（P2-F1 落地后）为准，勿把 120 当定值用。
2. 未决 23 岛 / FE79-FECB 0.90 强疑（B 级）/ 跳转表入口（C 级假设区）按报告 §4/§10
   显式缺口类挂账——走 D7 口径，勿并入 covered。
3. 判据④（循环同构强判据）v1 未实现（报告自declared 遗留）——后续版本可加。
