# findings：reference lead atlas——refs 三矿开采 + 结构对账（task-16）

> 状态：**r1 已开采**（2026-10-15）。背景（用户裁定·已证等价原则）：refs/ 与 ROM
> 已证字节等价 ⇒ nmikstas/m1disasm 的**结构划分升格为静态分析主文档**；
> **命名仍 C 级**（AGENTS D5 第三方命名不背书），消费方自证后才升 A/B 入台账。
> 复跑：`python3 tools/ref_atlas.py reconcile --sweep <decode_sweep --json>` /
> `python3 tools/ref_atlas.py leads --json <out>`。

## 0. 矿况与解析口径

| 矿 | 形态 | 解析产出 |
|---|---|---|
| nmikstas `Source_Files/BankNN.asm` | `Lxxxx` 程序计数标签+`.alias Name $ADDR`+`.byte/.word` | 逐指令走址（代码/数据逐段划分，bank NN 直接对应）+ 109 语义名 |
| m1disasm `SRC/**.asm` | `Name: ; $XXXX` 富语义标签+注释（域=子目录） | 565 命名候选（域标注） |
| MetroidMMC3 / nmikstas DOC | 数据表文档 | batch-2 素材摘录（§4） |

## 1. ① 结构对账（优先项——task-5③ 分母 v2 输入）

```
[ref_atlas] nmikstas 走址：code 段 4180 data 段 297；走址-标签分歧 4405
[ref_atlas] 结构对账 tally: {"错面确认(ref=code)": 0, "升级线索(ref=code)": 2, "边界分歧(ref=data)": 2, "对账一致": 238, "ref 无覆盖/混合": 43}
```

- **对账一致 238**：我们 decode-sweep 的 code/data 判定与已证等价源结构**系统性吻合**
  （42 code 岛中 20+ 逐岛 code 1.00 对齐；banks0-6 大数据岛 ref 同判 data）——
  decode-sweep 判别力获权威结构背书（negative-control 口径不变）；
- **升级线索 2（ref=code）**：`bank07:$E770-$E791`、`$E8F1-$E90E`——我们动态执行证据
  （task-7 r3 A 级字节）+ ref=code ⇒ **双重证据，可升 A 级定谳并入 task-5③**；
- **边界分歧 2（ref=data & ours=code，考证线索）**：
  - `bank07:$DA39-$DA3C`（4B，我们：静态流+跳转表入口 code；ref：data）
  - `bank07:$EF9A-$EF9D`（4B，同上）
  ⇒ 分歧=考证线索（可能 ref 漏解码/我们误判/表代码混合），task-5③ 切分前需人工定谳；
- **错面确认 0**：含义明确——此前 24/27「错面格」是相对 **IDA/seed 分母**的错面，
  不是相对社区结构的错面（社区同样解为 code）⇒ 分母 v2 口径：**错面修正以 IDA 分母
  为对象，nmikstas 结构=对账背书源**；
- ref 无覆盖/混合 43：多为 banks0-6 大岛（素材+代码混合）与走址漂移区，待查。

## 2. ② 函数边界图谱（C 级→等价性背书下边界=事实）

nmikstas `Lxxxx` 标签 3198+（Bank07 单文件）× 我们 call_graph/decode_sweep 交叉：
- 交叉命中例（地址即事实，命名仍 C 级）：`ChooseRoutine $C27C`=我们的 C27C 分发器、
  `ExitSub $C45C`=动态实证的共享 RTS handler、`WaitNMIPass $C42C`/`WaitNMIPass_
  $C43F`=task-15 ④ 成环面（$1A 轮询 $C42F/$C442、写者 $C43F jsr 语境逐点对上）、
  `UpdateEnemyAnim $E094`=P12 残余边 $F438→$E094 的目标名；
- 边界分歧即考证线索：nmikstas 标签 vs decode-sweep 岛入口/IDA funcs 起点的差集
  （json 产物 `divergences` + 边界差集待下一轮逐条列）。

## 3. ③ 命名候选线索表（C 级，节选——全表见 `--json`）

| 候选名 | ref 指针 | addr | 自证状态 |
|---|---|---|---|
| ExitSub | nmikstas/Source_Files/Bank00.asm:33 | $C45C | **可升 A**（r3 动态：多站点高频 RTS handler 实证） |
| WaitNMIPass / WaitNMIPass_ | Bank00.asm:29/31 | $C42C/$C43F | **可升 B**（task-15 ④ $1A 轮询/写者证据） |
| ChooseRoutine | Bank00.asm:18 | $C27C | **可升 A**（r3 字节级分发语义+A 值域动态） |
| NMI | Bank00.asm:11 | $C0D9 | **可升 A**（task-15 ① 帧界证据） |
| SubtractHealth | Bank03.asm:9 | $CE92 | C 级（语义自证待做） |
| UpdateEnemyAnim | Bank03.asm:12 | $E094 | C 级（P12 边目标） |
| …（m1disasm 565 条带域标注） | | | 全部 C 级 |

- **候选命中率 vs 台账 8 条：0/674**（台账 8 条地址与本矿地址无交集——台账现有人名
  在其他地址；矿=纯新增线索池）；
- 纪律（D5）：以上一律 C 级入线索库，**自证升级才入台账**；「可升」列=已有自证证据
  者的升级建议（由命名任务走台账流程，本工具不 rename）。

## 4. ④ 数据表文档摘录（batch-2 素材，C 级）

- m1disasm `constants_ram.asm`：「16 slots of 16 bytes ($6AF4-$6BF3)」敌槽布局
  （与 ram-region-6af4-contradiction 定谳一致，互证）；
- nmikstas `.alias` 内存名（RandomNumber1 $2E / RandomNumber2 $2F 等 zp 变量语义）——
  零页变量语义线索池，batch-2 字段切分可直接消费；
- MetroidMMC3 数据表文档摘录待续（下一轮）。

## 5. 限制（如实）

1. **走址-标签分歧 4405**：长无标签段的 zp/abs 判别漂移（别名/宏语境）——span 起点
   以标签锚定，覆盖率数字 ±误差（>1.00 显示=相邻段漂移重叠）；分歧逐条登记在
   `--json divergences`；
2. m1disasm 无址标注文件（enemies/*.asm 等）未入走址（命名候选仍收）；
3. 窗口地址（$8000-$BFFF）候选的 bank 归属未消歧（P12-F2 多 bank 展开法可复用）。

## 6. 并入清单

- task-5③：§1 升级线索 2（E770/E8F1 双证）+ 边界分歧 2（DA39/EF9A 待人工）+
  「错面对象=IDA 分母」口径澄清；
- task-10/11：§2/§3 命名候选（ExitSub/WaitNMIPass/NMI 等可升 A/B 者）；
- batch-2：§4 零页变量语义线索池。
