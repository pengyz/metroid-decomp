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

## r3 续审（746567c，task-7 C27C 动态收口）——2026-10-15

### verdict: pass

### findings（逐项复现输出）

**① 探针七连链「exec 枚举=真执行」——通过 ✓（工件在位）**
`analysis/probe_dispatch_diag7.log` 原文复核：`count C0BF: str=0 enum=55`、
`count C0D9: str=0/enum=59`（申报吻合）+ **C0C0 str=0/enum=0 对照点**（非命中对照设计
好）；v1-v7 过程的 getAccessCounters 报错如实留痕（七连证伪链可考古）。r2 F1 更正
（「指令级覆盖不可得」→「字符串注册不可得」）口径精确。

**② A9 证伪——通过 ✓（.run1 工件独立复推 + 全 PRG 交叉确证）**
- 窗口指纹复推（window.jsonl 1600 帧逐帧离线比对各 bank 首 16B）：**bank0=541 帧、
  区域 bank（banks1-5 同形歧义）=1059 帧、bank7=0 命中**——与申报（541/1059/0）逐数
  吻合；banks1-5 首 16B 同形自证其「banks=[1,2,3,4,5]」歧义记法准确。
- **交叉确证（我方独立全 PRG 扫描）**：`4C 7C C2` 蹦床恰在 **banks1-5 的 $8024**（5 处）
  且 bank7 无——与证伪结论「游戏期窗口=区域 bank 非 bank7」互锁；`20 7C C2` 分发
  站点 bank0 $8027（标题侧场景口径吻合）。A9 更正注在账（l2-proof-assumptions
  「信任→已证伪（动态 A 级）」）。
- 前向影响确认：F282/F2E8/F2ED 的 const 表窗口参数化（task-8/9 处置）成立——
  L2 等价不受影响（两侧同 window_bank=7 契约），集成层须参数化。

**③ 027/028 A 级执行入口——通过 ✓**：E77B=`a6 4b`（LDX $4B）/E77D=`a9 00`/E77F=`38`
（SEC 单字节）/E780=`fd …`（SBC abs,X）——指令入口与操作数解释自洽；E8FC 族同构 ✓。

**④ 静态三修对 BASELINE 判别力——通过 ✓**：BASELINE: PASS 复现（正对照 code+9/9、
负对照 data 4/4——尾分发终止语义修正未伤判别力）；code=42/未决=19/跳转表词 253 与
申报一致。

### 本账 findings 汇总（r3）

| # | 分类 | 内容 | 处置 |
|---|---|---|---|
| P7-F1 | 工程卫生（命名） | ~~双跑第二腿工件未留仓~~ **【2026-10-15 更正】工件双腿俱在**——`analysis/traces/c27c-dispatch.*` 与 `.run1/` 两整套（我方以真路径复现「跑1 34/跑2 34；仅跑1 0，仅跑2 0」+两腿 summary 逐字节同）。残余=两腿路径命名非对称（.run1 vs 裸名）曾致我误判缺失 | 建议 run1/run2 对称命名；非阻断 |
| P7-F2 | 工具面（静默降级） | decode_sweep_dyn 对错误/缺失 traces2 路径**静默按 0 处理**（跑2=0 不报错）——验证工具的静默绿面类（我方误路径实证） | 改 fail-loud（缺 traces2 ⇒ 报错非 0 对照）；非阻断 |
| P7-F3 | 复审方自省 | 本轮探针四案（bank 映射坑×2：$8024/$95CE 同族、win 键名取错、run2 命名假设致工件误判） | 「地址级自证」前置门扩至**映射约定+工件路径实寻**再断言；累计第六至九案 |

## task-12 续审（da90f66，执行路径调用图+端口队列）——2026-10-15

### verdict: pass

### findings（逐项复现输出）

**① entry_probe 执行序/双跑——通过 ✓（真路径复现）**
汇总三行逐字复现（入口 264/动态边 562（jsr 345）/静态边 1056；交叉 55+2）；
换时刻双跑 **43/43 全同** 以真路径复现（第二腿=`analysis/traces/.ep-boot1/`——实寻
定位，命名非对称二连但工件双腿俱在）。
**② 伪边剔除+55 边抽核——通过 ✓，附消歧增值（P12-F2）**
188 条=栈帧残留伪边类（判据清楚）；附录 B 节选 20/55 如实标注 ✓；抽核 4 条
「静态未见」边（$9A7E→$9AE2 等）做**多 bank 展开**：JSR 字节 **4/4 唯一落 bank1**
（0x1A7E 等偏移，无第二 bank 命中）——「bank 归属多解」对这批样本实为可机械消歧；
动态观测（A9：运行时窗口=banks1-5）+ 静态唯一命中 = 这 4 边可升「静态证实@bank1」。
**③ 深度启发式诚实性——通过 ✓**：B 级声明、d=-1=16 间接可达如实标注（输出实测）；
IDA funcs 包含映射+顺流边+xref/跳转表边口径写明。
**⑤ 55 边窗口消歧遗留显式性——通过 ✓**：「登记不隐没」+0 命中类一律「路径受限」
标注（门/Boss/道具路径未覆盖写明）。
**④ P7-F2 修复复验——通过 ✓**：`FATAL: traces 前缀无任何产物文件: …nonexistent.*.jsonl`
（exit 1）实测复现。**但（P12-F1）**：同一静默 0 面**未迁移到新工具**——call_graph.py
`--repeat` 对缺失第二腿前缀静默「跑2 0」不 FATAL（我方误路径实测）。

### findings 汇总

| # | 分类 | 内容 | 处置 |
|---|---|---|---|
| P12-F1 | 工具面（P7-F2 未迁移） | call_graph --repeat 缺第二腿静默按 0（无 FATAL） | 同 P7-F2 方案改 fail-loud；非阻断 |
| P12-F2 | 复审增值（消歧） | 55 边抽核 4/4 多 bank 展开唯一落 bank1——可机械消歧批量推进 | 建议 call_graph 加多 bank 展开消歧步；非阻断 |
| P12-F3 | 工程卫生（命名） | 双跑腿命名非对称二连（.ep-boot1 vs 裸名；前案 .run1） | 建议 run1/run2 对称约定；非阻断 |

## P12 三件修复回归（7535bb1，2026-10-15 追记）——全闭 ✓

- **P12-F1**：call_graph --repeat 缺腿现 `FATAL: traces 前缀无任何产物文件:
  analysis/traces/nonexistent.entries/.edges.jsonl`（exit 1）——静默 0 类级清扫 ✓；
- **P12-F2**：多 bank 消歧落码——「动态 jsr 边静态未见」**55→1**（静态边 1056→1812
  展开），我方抽核的 bank1 唯一命中方法批量生效 ✓（余 1 条=真多解/未决，如实留）；
- **P12-F3**：对账口径修正随之落（工具输出对账面）。
- 回归口径：判据扩面实测（FATAL 复现+消歧数字复跑），非「声称修了」。

## task-17 r1 续审（abbadc6，game-data 四级数据链+brinstar 样例）——2026-10-15

### verdict: pass

### findings（逐项复现输出）

**① 复现命令原样跑通（链规③）+ ROM 自重导 ✓**：
`RoomPtrTable[0] = 0xa441`（其命令原文自跑命中期望）；Room00 19B 逐字节
`02 40 01 02 48 01 02 50 03 02 5f 03 02 fd 02 a1 02 b1 ff`——**逐字节一致: True**
（我方独立比对）；m1disasm 双工具互证表逐行 ✓。
**② D5 命名纪律 ✓**：字段全解构名/_todo_（RoomObject/RoomDef/MetatileDef/tail「_todo_
语义」注）；「Room00/StructPtrTable」仅存证据注不入产品名——「语义自证后走
data/naming 台账」口径正确。
**③ 新文件落库核（清单第 6 项）✓**：5 文件全在 commit——无漏 add。
**④ 单测 4/4 ✓**。findings 三族（r10）：0 新。schema 四级链+区域五件套=task-5 分母
v2 的 D 面素材，逐字段自证状态标注诚实。

## task-17 r2 续审（7a1607f，全房间批解析+tail 聚类）——2026-10-15

### verdict: pass

### findings（逐项复现输出）

**① 批解析工具自跑复现 ✓**（cargo run 带 ROM 参）：L1 47+50 词（[0]=$A441/$AC84 ✓）、
L2 47/47 全过 635 对象段、L3 50/50（130 行/440 元块）、tail $FF 终止 47/47——
与 Evidence 数字逐项吻合。
**② tail 聚类自证（C→B 素材）✓**：长度 16 簇（3-21B）实数吻合；首字节主簇
{1:8, 2:25, 7:2}=35/47 ✓；type=2×Room00「Blue door $02」注释互证（注释线索×批量
分布）；type=49/65/81 三簇（12 房=1+4+7）留 C 级=诚实降级 ✓。
**③ 新文件落库核（清单第 6 项）✓**：game-data-inspect.rs+findings §6 在 commit。
**④ L4 $AEF0 锚点+「索引闭合校验待全表长度自证」=显式待办** ✓。
findings 三族（r10）：0 新。

## task-5③ 续审（2c9b6fb，denoms v2 纠偏机制）——2026-10-15

### verdict: pass

### findings（逐项复现输出）

**① denoms 运行原文复现 ✓**：「补录 +6 S 格；错面待迁 27；段切 1；残余待查 2 格 + 1
尾段（显式）」+「S=464 D=193 B=11 R=0；covered=29」逐字命中。
**② 纠偏清单证据面 ✓**：denoms_corrections.json 七键结构（s_supplement/d_wrongface/
segment_splits/residuals/v2_totals…）+ findings/decode-sweep 指针 11 处——四类逐条
带证据指针=claim 兑现。
**③ 新文件落库核（清单第 6 项）✓**：denoms_corrections.json 在 commit（4 文件
stat=意图一致）。
**④ 「v2 定值机制」grep=1 ✓**（completeness.md 口径分离：估计值 vs 定值账面）。
**机制评**：纠偏清单机制（非工具内嵌硬编码）=权威记录与消费解耦——补录格自动入面/
错面/段切/残余显式打印（不静默），机械导出+清单=可复现分母 ✓。
findings 三族（r10）：0 新。

## r2 边界更正快记（9d3e6c6，FE83 族）——2026-10-15

### verdict: pass（更正注类快核）

边界算术核：$FE83-$FECB=**73B** 逐数吻合；段首字节抽读在位（变长条目族=34 条随
r2 口径）；调用集补 F311 四员=更正注随行。更正文化规范（原地注+不删历史）✓。

## task-17 r6 续审（f9bc0ec，CHR 面——四级链闭合·主目标达成）——2026-10-16

### verdict: pass

### findings（逐项复现输出）

**① GFX 转运表复现 ✓**：24 项（168B）全解析+首项 `bank:6 src:32768 dest:0 size:2464`
=asm 互核 GFX_Samus=06:8000 dest $0000 逐字；7B/项结构（[bank][src][dest][len]）
双锚（prg7_engine 注+macros 宏体）✓。
**② 四级链闭合 ✓**：room_ptrs→rooms→structure_ptrs→structures→metatiles→tile×16→
GFX 块（24 项）——全链地址锚+字节级对拍（r1-r6 各节）=**task-17 主目标达成**（分母
D 面 193 格/covered=44 实读）。
**③ 测面 ✓**：14/14；tile_chr_offset（NES 2bpp×16B）=A 级平台语义。
**④ 面分布=dest 语义 C 级显式标注 ✓**（精灵 2/背景 16/其它 6——不越级）。
findings 三族（r10）：0 新。

## 遗留风险清单

1. S 面 120 为 B 级估计——分母 v2 实际登记（lead task-5）以 decode-sweep 复跑 + 逐条
   佐证标注（P2-F1 落地后）为准，勿把 120 当定值用。
2. 未决 23 岛 / FE79-FECB 0.90 强疑（B 级）/ 跳转表入口（C 级假设区）按报告 §4/§10
   显式缺口类挂账——走 D7 口径，勿并入 covered。
3. 判据④（循环同构强判据）v1 未实现（报告自declared 遗留）——后续版本可加。
