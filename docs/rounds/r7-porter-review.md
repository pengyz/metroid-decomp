# rounds/r7-porter-review — porter 批产对抗复审账本（滚动）

> 对象：task-1 F1xx 族批量移植的逐批 commit。立场：证伪、自重导（ROM 字节级核 / 独立
> 模型 / 突变敏感性抽查——环⑥⑦⑧）；**verdict=pass 才算 porter 该批步骤完成**。
> 写路径互斥：本账本只记录复审结论；被审文件不落一字。

## 批 1：sub_F140（commit 05c9f10，9 文件一体）

### verdict: pass

语义逐 ROM 字节核、独立模型 11/11、突变判别（基模证真前置门后）全数通过；两条非阻断
finding（chat-only 主张未落仓 / 判别力盲区一角）不影响本批判定。

### findings（逐项复现输出）

**① sub_F140 语义逐 ROM 字节核 —— 通过 ✓（自重导，不信转述）**

```
prg[F140..]: 20 bf f1 20 86 f1 4c fa f1 …   ⇒ JSR $F1BF / JSR $F186 / JMP $F1FA ✓
prg[F1BF..]: bd f5 6a 20 e0 f1 bd f6 6a 4c d9 f1   ⇒ LDA $6AF5,X / JSR $F1E0 / LDA $6AF6,X / JMP $F1D9
prg[F1E0..]: 18 79 01 03 85 04 60   ⇒ CLC / ADC $0301,Y / STA $04 / RTS
prg[F1D9..]: 18 79 02 03 85 05 60   ⇒ CLC / ADC $0302,Y / STA $05 / RTS
prg[F186..]: b9 0d 03 85 06 b9 0e 03 85 08 b9 0c 03 45 ff 29 01 85 0a 60
             ⇒ LDA $030D,Y/STA $06；LDA $030E,Y/STA $08；LDA $030C,Y / EOR $FF / AND #$01 / STA $0A / RTS
```

逐字节导出组合语义 = **$04←WRAM[$6AF5+X]+RAM[$301+Y]、$05←WRAM[$6AF6+X]+RAM[$302+Y]、
$06←RAM[$30D+Y]、$08←RAM[$30E+Y]、$0A←(RAM[$30C+Y]^RAM[$FF])&1、末值 A=$0A、X/Y 透传**
——与 sub_f140 实现（f1bf∘f186 组合、writes 并集、a=f186.a）逐项一致。
**`45 FF` = EOR zp $FF 逐字节实锤（opcode 45 ≠ 49 立即数）**——EOR zp 陷阱（a91c95f 前科）
在本件正确处理。F1FA 碰撞边界：stops=F1FA 入口截断——11 案 golden 的 A∈{0,1}（=$0A），
若窗口越界执行 F1FA 首条 `A9 02`（LDA #$02）A 必为 2，黄金面自证截断正确（突变 m5 实测
11/11 打回）。

**② 三方真实性 + golden 独立复算 —— 通过 ✓**

- 腿结构核实（crates/game-oracle/src/bin/l2-diff.rs 头注释与报错分立）：
  `golden≠ROM执行` 与 `golden≠game-core` 分别报错——golden（case 内静态）↔ 6502 核
  执行 ROM 原字节（diff 时现算）↔ game-core 三值独立比对；A7 same-source 弱化在账
  （findings/l2-proof-assumptions.md）。
- **我方独立语义模型 × 11 案全量对账：11/11 全等（0 不符）**（基模断言前置门通过后开跑）：
  抽样手推互证 gen-f140-0（153+239→136、91+60→151、(46^123)&1=1）、b-f140-xyff
  （X=Y=0xFF 变址有效址 6BF4/400 页）、b-f140-ffmask（(112^255)&1=1）全对。
- 晋级一致性：batch0_cases.jsonl（gen-f140-0..7）+ batch0_boundary.jsonl（3 案）→
  l2_cases.jsonl **11/11 逐字段同源、0 不一致**。
- 门禁路径全量复跑（自跑）：`python3 tools/diff_subroutines.py` →
  「L2 三方裁定: 117 条用例, 0 失败（golden == ROM 执行 == game-core）」。

**③ 突变敏感性（基模证真前置门后开跑）—— 通过 ✓，一角盲区（P1-F2）**

| 突变 | 打回 | 幸存 |
|---|---|---|
| m1 末值 A=f1bf.A（$05 链结果） | 11/11 | — |
| m2b 漏变址 X/Y | 11/11 | — |
| m3 漏 AND #1 | 11/11 | — |
| m5 窗口越界执行 F1FA | 11/11 | — |
| m2a `45 FF` 误读立即数 | **5/11** | gen-0/1/2/3/5、b-f140-ffmask（ram[$FF] 奇≡^FF 同效，语义不可判别） |
| m1b【盲区探针】末值 A=$04 链结果 | 10/11 | **b-f140-ffmask**（a4==aA==1 巧合） |

- 其自报 m1「11 失败」被我方 m1 复刻证实；其 m2 为**打包突变**（漏变址+立即数误读），
  11/11 口径如实——单 EOR-zp 面的判别集恰为 5 条偶 ram[$FF] 案（含 wrap 的
  「0A:1≠0A:0」正打），b-f140-ffmask 为**故意等值界盲区标记案**，设计口径正确。
- 突变残留 grep：`MUTATION-TEST-ONLY` 全仓 0 命中（双工具）。

**④ 补录格 S-FUNC-07-F140 登记合规 —— 通过 ✓**

矩阵格实读：covered=true、strength=L2、evidence=用例 id + ROM 原字节指针
（bank07:F140-F148=20 BF F1/20 86 F1/4C FA F1）、note 含补录格依据（seed-decode-gap
A 级）+ denoms 待纠声明（decode-sweep v2 回填）+ 碰撞尾独立切片口径；基线 9→10 仅上调
（history 带 reason，baseline-integrity 绿）。covered 登记先于复审 pass 系
completeness §3 流程（④登记→⑤review）既有口径，非本批新伤。

**⑤ 其「python 逐 flag 模型 vs casegen golden 80 条 0 不符 / F1FA 碰撞 8/8」主张 —— 仓内无承载（P1-F1）**

双工具核查（grep oracle/ docs/ rounds/ crates/ + tools/*.py 清单）：「80 条」「8/8」
数字与该 python 模型**均不在仓库**（commit message 亦无）——环⑪「聊天结论不作数」。
实质已由复审替代验证（①②的字节核+独立模型 11/11+截断自证 8/8），但主张本身不可复跑。

### 本批 findings 汇总

| # | 分类 | 内容 | 处置 |
|---|---|---|---|
| P1-F1 | 声明失实（轻） | 「80 条/F1FA 8/8」chat-only 无仓内承载 | 回 porter：模型落 tools/ 或数字入 commit/账本；实质已由复审替代验证，非阻断 |
| P1-F2 | 判别力边界 | m1b 盲区（ffmask 幸存）+ EOR-zp 单突变判别集仅 5 案 | 建议 casegen 边界族补「a4≠aA ∧ a5≠aA ∧ ram[$FF] 偶」组合案 1-2 条；非阻断 |
| P1-F3 | 工程卫生（复审方自省） | 本账探针两案：大小写归一 bug 致假判别数、m1 突变定义错 | 立「基模证真前置门」规矩（assert 基模全等才许开突变矩阵）；进方法面遗留 |

## 批 2/3：sub_F1A7（ff1f0b0）+ sub_F1B3（522efe8）——JMP 共享尾变体对

### verdict: pass（两件）

### findings（逐项复现输出）

**ROM 字节核（5/5 自重导）**：

```
F1A7: bd 01 03 20 e0 f1 bd 02 03 4c d9 f1   ⇒ LDA $301,X / JSR $F1E0 / LDA $302,X / JMP $F1D9 ✓
F1B3: bd 01 03 20 e7 f1 bd 02 03 4c cb f1   ⇒ LDA $301,X / JSR $F1E7 / LDA $302,X / JMP $F1CB ✓
F1E7: 18 79 f5 6a 85 04 60   ⇒ CLC / ADC $6AF5,Y / STA $04 / RTS（WRAM 域 ✓）
F1CB: 18 79 f6 6a 85 05 60   ⇒ CLC / ADC $6AF6,Y / STA $05 / RTS（WRAM 域 ✓）
F1D9: 18 79 02 03 85 05 60   ⇒ CLC / ADC $0302,Y / STA $05 / RTS
```

组合语义与两件实现逐项一致：F1A7=$04←RAM[$301+X]+RAM[$301+Y]、$05←RAM[$302+X]+RAM[$302+Y]、
末值 A=$05；F1B3=$04←RAM[$301+X]+WRAM[$6AF5+Y]、$05←RAM[$302+X]+WRAM[$6AF6+Y]。
陷阱清单「`79 F5 6A`/`79 F6 6A`=abs,Y 读 WRAM」主踩点件处理正确（F1B3 的 WRAM 域
读解无误——RAM/WRAM 混淆类突变 m2 被 11/11 打回）。

**独立模型 × 22 案（前置断言门）**：f1a7 11/11 全等、f1b3 11/11 全等（0 不符）。

**突变判别矩阵**（基模证真后）：

| 突变 | f1a7 | f1b3 |
|---|---|---|
| Y 侧漏变址 | 11/11 | 11/11 |
| X 侧漏变址 | 11/11 | 10/11（幸存 b-f1b3-yff-x0：**X=0 固有盲点**，集合级仍判别） |
| 共享尾误配 / WRAM 当 RAM | 11/11 | 11/11 |
| 末值 A=$04 | 11/11 | 11/11 |

其自报 m1/m2 口径（各 11 失败 + 判别值如 b-f1a7-wrap 04:0≠04:8）与我方矩阵互证。

**登记与基线**：S-FUNC-07-F1A7=L0→covered（预存格升级，非补录）✓、S-FUNC-07-F1B3=
补录格 covered ✓（note 含 denoms 待纠声明）；covered 12 格、基线 12/12、
`python3 tools/diff_subroutines.py` → 「L2 三方裁定: **141 条用例, 0 失败**」自跑
（139+2=porter 对 P1-F2 的 b-f140-discrim 响应案在途）。proof-checks v2 触发面生产实拍：
「变更面不含 S_intx——delta=prompts/README.md」显式 SKIP。

**P1-F1 处置确认**：F1B3 commit body 已按裁定把模型核对数字入仓
（九件 LCG 72 条 0 不符 + 边界 6 条 0 不符，且如实标注「模型非仓内产物，最终裁决以
l2-diff 三方为准」）——处置到位。本复审独立模型现覆盖 f140/f1a7/f1b3 共 33 案。

### 批 2/3 findings 汇总

| # | 分类 | 内容 | 处置 |
|---|---|---|---|
| P3-F1 | 判别力边界（记录） | b-f1b3-yff-x0（X=0）对 X 侧漏变址固有盲——集合级 10/11 仍判别，非缺陷 | 记录在案；X=0 案的语义价值在 Y 侧独立性，不改 |
| P3-F2 | 处置确认 | P1-F1 数字入 body 完成；P1-F2 响应案 b-f140-discrim 系列在途 | 滚动观察，后续批核其判别力 |

### P1-F2 加固核验（6423ce5，2026-10-15 追记）

b-f140-discrim / b-f140-discrim2 组合案（a4≠aA ∧ a5≠aA ∧ ram[FF] 偶）判别力实测：
**m1b 原盲区 2/2 被打回、m2a 2/2 被打回**（基模 13/13 全等前置门后）——旧 11 案的
m1b/m2a 幸存面全部被新案封堵，**P1-F2 盲区闭合**。EOR-zp 单突变判别集由 5 案升 7 案。

## 批 4/5：sub_F1D2（0597021）+ sub_F1CB（813f34a）——常量装载 / WRAM 共享尾本体

### verdict: pass（两件）

### findings（逐项复现输出）

**ROM 字节核**：F1D2=`a9 04 20 e0 f1 a9 08 18 79 02 03 85 05`(+F1DF `60`) =
LDA #4 / JSR F1E0 / LDA #8 / 落 loc_F1D9 尾 ✓（**A9 立即数反向陷阱**：勿读 zp $04/$08）；
F1CB=`18 79 f6 6a 85 05 60` = CLC / ADC $6AF6,Y / STA $05 / RTS ✓（79 F6 6A=abs,Y 读
WRAM 域主踩点；入参 A 参与运算）。

**独立模型 + 突变矩阵**（基模证真前置门后）：

| 件 | 基模 | m1 | my/m2 | mA/m3 |
|---|---|---|---|---|
| F1D2（10 案） | 10/10 全等 | A9 误读 zp：10/10 打回 | Y 漏变址 10/10 | 末值 A=$04：10/10 |
| F1CB（10 案） | 10/10 全等 | WRAM 当 RAM：10/10 | 漏 Y 变址 10/10 | A 不参与：9/10（幸存 gen-f1cb-5：**A_in=0 固有盲点**，集合级判别） |

其自报 m1/m2 判别值互证（b-f1d2-wrap 04:0≠04:252、b-f1cb-wrap 05:4≠05:255）；wrap 手推：
4+252→0、8+249→1、A=255+5→4 全吻合。

**登记/基线**：S-FUNC-07-F1D2=L0→covered、S-FUNC-07-F1CB=补录格 covered；covered 14、
全量 `diff_subroutines` 自跑 **161 条 0 失败**。P1-F1 数字入 body 持续合规（gen-f1d2 8 条/
gen-f1cb 8 条 0 不符入总账）。

### 批 4/5 findings 汇总

| # | 分类 | 内容 | 处置 |
|---|---|---|---|
| P5-F1 | 判别力边界（记录） | gen-f1cb-5（A_in=0）对「A 不参与」类突变固有盲（同 b-f1b3-yff-x0 X=0 族） | 记录在案；集合级 9/10 判别充分 |
| — | 处置确认 | P1-F2 加固案（6423ce5）判别力实测通过（前讯追记） | 闭合 |

## 批 6：sub_F1E7（258dea4）——WRAM 共享尾（$04 侧）

### verdict: pass

字节 `18 79 f5 6a 85 04 60` ✓（79 F5 6A=abs,Y 读 WRAM 主踩点）；独立模型 10/10 全等；
突变矩阵 m1（WRAM 当 RAM）/m2（漏 Y 变址）/m3（A 不参与）**均 10/10 打回**（本批无
A_in=0 盲点案）；wrap 手推 A=255+5→4 吻合；S-FUNC-07-F1E7=补录格 covered、covered 15、
全量自跑 **171 条 0 失败**。其自报判别值互证；P1-F1 数字入 body 持续合规。

## 批 7：sub_F149（65eaba6）——入口组合件（JSR F186/JSR F1D2/JMP F1FA）

### verdict: pass

字节 `20 86 f1 20 d2 f1 4c fa f1` ✓（与 F140 同族碰撞尾：stops=F1FA 入口截断）；
独立模型 10/10 全等；突变矩阵：morder（末值 A 误取前段 f186.a）10/10（其自报 m1 判别值
b-f149-wrap a=2≠1 互证）、mimm（A9 误读 zp）10/10、mA 10/10、my（Y 漏变址）9/10——
幸存 b-f149-wrap（**X=Y=0 固有盲点**，同 X=0/A_in=0 族，集合级判别）、m2a（EOR 立即数）
4/10（偶 ram[FF] 判别集，同 f140 族）。其 m2「组合缺前段」=写集形状 5 键≠2 键，全案必打。
S-FUNC-07-F149=L0→covered、covered 16、全量自跑 **181 条 0 失败**；P1-F1 持续合规。

## 批 8：sub_F172（95886e0）——X 变体坐标/属性读取（经 loc_F17F 共享尾）

> 【账目更正注（2026-10-15）】95886e0 之后的 **de9ba99 message 失实**——其标题写
> 「feat(core): sub_F172…」但 diff 实为 docs/knowledge 指针校正（=37b6c6e 内容，
> amend 竞态顶替 message；双树哈希 a806a82d 两处一致，内容零损失）。**非重复移植**；
> 全记录见 rounds/r8 协调账本 + prompts 链规 14（共享分支禁裸 amend）。批 8 实体
> =95886e0 一个，pass 有效。我方独立核实：de9ba99 diff 仅涉 gotcha-negative-
> assertion-time-race.md（+9/-2）、双树哈希与 37b6c6e 逐一相符。

### verdict: pass

字节核 ✓（F172=BD 0D 03/85 07/BD 0E 03/85 09/BD 0C 03；loc_F17F=**45 FF**/29 01/85
0B/60——EOR 零页 $FF 高危同族主踩点正确）；独立模型 10/10 全等；突变矩阵：
m2a（EOR 误读立即数）**5/10，FAIL 名单与其申报逐字吻合**（gen-3/-4/-6 + b-f172-xff +
b-f172-ffzero——两条专用偶 ram[FF] 边界案在内；奇数等值=固有，其申报如实且复用
P1-F2 族结论）、mx（X 漏变址）10/10、m3（漏 AND#1）10/10、mA 9/10（gen-f172-3 幸存=
a7==aB 巧合固有盲）。S-FUNC-07-F172=L0→covered、covered 17、全量自跑 **191 条 0 失败**。
P1-F1 数字入 body 持续合规；其突变申报口径（如实报 5/10+逐案名单+固有等值解释）
为批产最佳实践样板。

## 批 9：sub_F19A（b79f875）——zp,X 8 位变址回绕主踩点（双陷阱同件）

### verdict: pass

字节核 ✓（F19A=B5 B1/85 07/B5 B2/85 09/B5 B3/4C 7F F1，尾汇 loc_F17F=45 FF/29 01/
85 0B/60）；独立模型（zp,X **8 位回绕**）11/11 全等；突变矩阵与其申报**精确互证**：
m1（16 位加误实现/漏回绕）8/11、幸存=[b-f19a-x0, gen-f19a-1, gen-f19a-3]（X 小值/X=0
固有等值）逐名吻合；m2a（EOR 误读立即数）6/11 吻合。回绕专用边界案
b-f19a-xff-wrap（X=$FF→$B0/$B1/$B2）/b-f19a-x50-wrap（X=$50→$01/$02/$03，16 位加
错读 $01Bx 域）在判别面内。S-FUNC-07-F19A=L0→covered、covered 18、全量自跑
**202 条 0 失败**。双陷阱唯一同件处理正确；突变申报口径延续批 8 水准。

## 批 10：sub_F162（f74ca15）——Y 变体（F152 对偶，decode-gap 争议件）

### verdict: pass

**解码锚点复认**：F109=`20 62 f1`（JSR $F162）实存 ✓——decode-gap 争议（ef80052 曾
推迟）的定谳锚不变，补录格登记引 findings/seed-decode-gap + samus-movement 2.2 更正
指针齐全。字节核 ✓（F162=B9 00 04/85 06/B9 01 04/85 08/B9 FB 6A/4C 93 F1——**WRAM
$6AFB+Y 一读**；F193 尾=45 FF/29 01/85 0A/60）；独立模型 10/10 全等；突变矩阵与其
申报**逐名互证**：m2a（EOR 误读）5/10、m2（WRAM 当 RAM）6/10、幸存=[gen-f162-2/-6/-7,
b-f162-wram-ff0] 精确一致（LSB=0 固有等值解释正确——w 仅经 &1 入 (w^ram[FF])&1）。
covered 19、全量自跑 **212 条 0 失败**。双/三陷阱族（45 FF+WRAM 域）处理正确。

## 批 11：sub_F1FA（89a1770）——碰撞逻辑全窗（最复杂件，任务书收尾件）

### verdict: pass

**全窗逐指令手推自重导**（F1FA-F261 104B + F262/F266/E449 三内联件）：两轴路由相反
（首轴 `LDA $03/BNE F224` vs 次轴 `LDA $03/BEQ F251`）字节级实锤；F262=`LDA $0B/SBC
$0A/STA $01/BPL/E449/INC $10/RTS` 进位续链（C1/C2 由 SEC/SBC $06 与 $09-$08 分别产生）；
E449=`A9 00 38 E5 00 85 00 A9 00 E5 01 85 01 60`=16 位取负 ✓；四出口 A 值（F22E/F236/
F259/F261）与写集路径子集一致。**独立模型 × 14 案全等（值+写集+出口 A，0 不符）**。

**突变矩阵**：m1（SBC 进位链断裂）我方宽口径（链路+符号扩展）8/14、其申报窄口径
（链路-only）4/14——**其 FAIL 名单 [gen-0/-7, chain/dec] ⊂ 我方名单**，作用域差不构成
矛盾（申报如实）；m2（E449 降级 8 位取负）**恰 2/14** ✓ 与其申报一致（gen-f1fa-1/-4）。

**F11-F1（判别力边界）**：专用取负案 **b-f1fa-negate 对「取负位宽」盲**——$00=0 巧合下
8 位/16 位取负同果（16 位取负 0x00FD→0x0003 vs 8 位仅动 $01→3 相同）；集合级由
gen-f1fa-1/-4（$00≠0）判别，但建议边界案把 $00≠0 纳入取负路径。
**F11-F2（工程卫生·记录）**：case 文件写集键形混用（2hex 46 条 / 4hex 164 条，runner
容错）——曾绊我探针一次（键宽归一后全等），建议 casegen 统一 4hex。

covered 20、全量自跑 **226 条 0 失败**；P1-F1 数字入 body 持续合规（其模型自核修正
2 处手推错误的披露=诚实文化样板）。

## task-1 队列收官记

F140/F1A7/F1B3/F1D2/F1CB/F1E7/F172/F19A/F162/F1FA **十件全数过审（批 1-11 全 pass）**；
226 案三方 0 失败；covered 20 格；P1-F1/F2 全闭环（F2 加固案判别力实测通过）。
verdict=pass ⇒ task-1 完成判据之「verifier 复审 pass」达成（其余判据由 porter 侧自证）。

## F11-F1/F11-F2 修复回归抽核（8ff7dc7/9f5c39d，2026-10-15 追记）——双闭合 ✓

- **F11-F2**：写集键形 census 修复后 **655/655 全 4hex**（修前 2hex 46/4hex 164）✓；
- **F11-F1**：b-f1fa-negate-lo（$00≠0 取负路径）入 **m2 取负位宽判别集**——
  判别集=[b-f1fa-negate-lo, gen-f1fa-1, gen-f1fa-4]（修前 2 案）；旧 b-f1fa-negate
  对 m2 仍盲（$00=0 同果，账内原记录不改）；F1FA 全族基模 15/15 全等（含新案）。
- 修复回归口径：判别集扩面实测（非「声称修了」）——P1-F2 加固案同款流程复用。

## task-8 批 12：sub_F270（551d307）——F2CA 族九入口第 1 件（ORA abs,X 自或回写）

### verdict: pass

字节 `1d 0a 03 9d 0a 03 60` ✓（ORA $030A,X / STA $030A,X / RTS 自或回写）；独立模型
11/11 全等；突变矩阵：m2（漏 X 变址）10/11、幸存=[b-f270-x0] **与其申报精确互证**；
m1（丢自或）我方口味 11/11 vs 其申报 10/11——突变定义口味差（直写语义变体），不构成
矛盾。边界案三件（X=$FF 有效址 $409 / A=0 掩码 / X=0 对照）设计齐。S-FUNC-07-F270=
补录格 covered、covered 21、全量自跑 **238 条 0 失败**。九入口共享 RTS 尾族开张顺。

## task-8 批 13：sub_F2DF（c523762）——Y 侧自或回写尾（F2ED 流 JSR 内尾）

### verdict: pass

字节 `a5 10 19 04 04 99 04 04 60` ✓（LDA $10 / ORA $0404,Y / STA $0404,Y / RTS——
入参 A 不影响）；独立模型 11/11 全等；突变矩阵与其申报互证：m1（LDA $10 误读入参 A）
10/11（b-f2df-mask a=242 判别值吻合，幸存 gen-f2df-4=ram[10]==A_in 巧合）、
m2（漏 Y）10/11 幸存=[b-f2df-y0] 精确互证。covered 22、全量自跑 **260 条 0 失败**。

> 链规 15 口径自本节后生效（lead 2026-10-15 下发，本批按旧形态核完不返工）：
> 批 14 起复审加三条——①产品层代码零地址字面量（grep 0x 可验）②命名台账 rename
> 带 A/B 级证据字段③行为钉死不变式（迁移前后同批 L2 案全绿）；方法论见
> docs/spec/two-layer-architecture.md。

## 批 14/15：F2D8（1d61521）+ 两层架构迁移轮（71cdebd/deed56e/4f63204/87b6b87/c2af8f8）

### verdict: pass（批 14 过渡窗裁定 + 迁移轮链规 15 三口径全过）

**批 14（F2D8）过渡窗裁定**：落库 13:22:22，两层架构 spec（bcb65fd）13:23:41——
**先于 spec 79 秒**，旧形态宽限（同 F2DF「已落件不返工」精神）。实质：字节
`1d 04 04 9d 04 04 60` ✓、基模 11/11、m1 11/11、mx 10/11（幸存 b-f2d8-x0 与申报互证）。

**迁移轮（two-layer 首落地 ×5 commit）链规 15 三口径**：

1. **产品层零地址字面量 ✓**：enemy/{attrs,slot,todo_0404,todo_040e}.rs 业务方法体零地址；
   21 处 hex 分类实测——0x100×12=数组 size、0xFF/0x81=测试数据值、地址仅存
   （a）注释引证（slot.rs:4、attrs.rs:52）（b）memmap 边界测试断言（write_event 地址）。
   两条 advisory：注释/断言中的地址可下沉 memmap 注释（严格口径「地址只活 memmap」）。
2. **命名台账 A/B 级证据 ✓**：data/naming/ledger.jsonl 8 条（N-0001..N-0008）逐条带
   evidence（A 级地址自证 bank:addr 逐字节 + B 级语义概要）+confidence；C 级假设按
   §3 纪律**降级 _todo_ 形态入账**（N-0005/N-0007「coords/state」之说未证不入名）——
   伪语义命名防线正确；naming-ledger 门禁自启实测 PASS（8 条逐条校验证据指针）。
3. **行为钉死不变式 ✓**：迁移前后同批 L2 案全绿——六族（f270/f2df/f2d8/f2d5/f2ad/f2ca）
   各 11 案全在活跃集（33 条切片期老案零丢失），全量自跑 **304 条 0 失败**；
   rust-tests 30（域层单测）；gates check 全绿。

结构面：enemy（产品）/memmap（地址唯一层，encode/decode+write_event 推导写集）/
verify_shim（委派）三层分离与 spec §1 一致；shim 侧仅 FLAG_C=0x01 标志位常量（非地址）。
N-0001 域几何 16B×16 等 A 级自证与 ROM 字节（F28E/F295）可对（其 evidence 已逐条列址）。

## 批 16/17：sub_F2ED（1b99278）+ sub_F2E8（e7372c7）——CD9C 探针路由 / Z 路由双流

### verdict: pass

**字节锚全通**（复杂件重手段）：F2ED=`b0 21 20 df f2 98 48 20 9c cd 68 a8 90 15 a9 80 …`
——C=1 直出（BCS）/ JSR F2DF or_writeback / **TYA-PHA-PLA 恢复链**（A=Y_in 出口语义
字节实锤）/ 探针 C=0 BCC 出口 / LDA #$80→STA $010F（**$010F=0x80 非 0x81**——其 ROM 腿
打回后修正的手推，与字节一致）/ JSR F332 全变换（F340=`a5 10 49 03 60`=LDA $10/EOR #3
半变换无 ASL ✓，F332=JSR F340+JMP C2C6 ASL×3 全变换）/ JSR F270 / F306 向量
`ad ce 95 85 6e ad cf 95 85 6f`——**ROM 向量 $95CE/$95CF=03 38**（window_bank=7 约定
prg[0x1D5CE]，我首读映射错已自纠）与申报一致。CD9C=`38 ac 00 03 88 d0 1b…`（SEC 起手
进位关键输出 ✓）。F2E8=`20 40 f3 d0 eb`（JSR F340 半变换/BNE F2D8 Z 路由）✓。

**口径核**：其「ROM 腿当场打回手推 golden 两处（A=Y_in 而非探针 A、$010F=0x80 非 0x81）
后修正」披露=诚实文化样板，两处均经我字节级独立证实。RED 演示工作流偏移如实记录。
covered 28、全量自跑 **315 条 0 失败**。

**残余（P16-F1，advisory）**：CD9C 四路出口逐路表未入折叠注——含 CDB5 BNE +8 落 CDBF
（RTS 后一字节）的出口语义、各出口终值 C/A 表；建议 memmap/cd9c.rs 折叠注补出口表，
供后续复审方模型化（本案经字节锚+三方案件钉住，不影响判定）。

## task-8 终审（ee2c7fb F282 收官 + 九入口全量）——verdict: pass ⇒ task-8 complete

### 特别裁定项：CD9C 出口表差 1 ——**我方模型错，其折叠注对**

字节自证逐址核（基准）：`CDB3=38 CDB4=D0 CDB5=08 CDB6=2C … CDBE=60`——BNE 操作码
（D0）在 **CDB4**、相对 0x08、下一指令址 CDB6 ⇒ 目标 **CDBE=RTS**。其 memmap/cd9c 折叠注
「BNE 在 CDB4→CDBE」正确；**我批 16/17 账目「CDB5 BNE +8 落 CDBF」错误**（指令起点
差 1 级联目标差 1）。
> 【2026-10-15 更正】批 16/17 P16-F1 中「含 CDB5 BNE +8 落 CDBF（RTS 后一字节）的
> 出口语义」——**误**，实为 CDB4 BNE +8 → CDBE（RTS），出口语义干净四路无悬空落点。
> 原文保留（更正文化）；本错计入复审方自省（本会话探针第五案：空跑绿/m1 定义/键宽/
> bank 映射/指令起点——「基模证真前置门」需扩展到**地址级自证**再写断言）。

### ① 九入口逐件补完 —— ✓

F2D5=`20 32 f3 1d 04 04 9d 04`（JSR F332 全变换+ORA/STA $0404,X）✓、F2AD=`a9 81 9d
0e 04 d0 21 b0 13 20 9c cd`（LDA #$81 直写 $040E,X + CD9C 探针）✓、F2CA=`b0 12 b9 00
03 9d 0e 04 20 79 f2 20 32 f3 1d 04 04 9d 04 04 60`（carry 敏感+set_entry+F279 共享尾+
or_writeback 三方法组合）✓——与申报语义逐条对上。F282=**哨兵窗**（`b0 fd` 回跳 F281
双重角色栈纪律）+五路出口；F338 尾 `a5 10 0a 0a 0a 4c 7b f2`=mask10_shl3 A 级 ✓；
ROM 表 $968B（window_bank=7）/F2A1 `29 10` 位测试 ✓。

### ② 突变矩阵汇总 + 两处实现缺陷修复实证 —— ✓

- **链规⑤活例**（L2 案打回实现两处，如实披露）：「组合未链化状态 + 写集未收敛」——
  修复=merge_writes（**同址 last-wins 终值映射**，verify_shim 在码 + run_subroutine
  final_writes 同义）+状态链化（F2AD 读 F2E8 写后态）。实证：b-f282-ad-tail 终值
  0x409:108 单事件收敛（打回时 core 双事件 12/96）；全量 **328 条 0 失败**。
- 突变专案 1/1 精准互证：m1（槽字段 ≥4 误作 >4）⇔ b-f282-f0-ge4（f0=4 边界）；
  m2（去状态链化）⇔ b-f282-ad-tail——两突变各被唯一专案正中（专设判别面设计好）。
- 九件累计突变申报全部与我方矩阵/字节核互证（批 1-18 无一虚报）。

### ③ 两层三口径 + 台账终查 —— ✓

- 产品层地址字面量终态：test 断言地址已下沉 memmap 常量面（advisory① 落实），
  余 hex=0x100 size×12+注释引证（advisory 级）；
- 命名台账 **8 条全带 evidence**（A×3/B×3/C×2，C 级 _todo_ 降级纪律正确）；
  remap 挂号 **9 条=九入口全量**；
- 行为钉死：迁移全程 66+ 案零丢失，328 条 0 失败；rust-tests 14 单测过；
  naming-ledger 门禁自启 PASS。

### task-8 终审记

F2CA 族九入口（F270/F2DF/F2D8/F2D5/F2AD/F2CA/F2ED/F2E8/F282）**9/9 全部落地并过审**
（批 12-18）；两层架构转型完整（enemy/memmap/verify_shim）；covered 29；判别力文化
（专设边界案 1/1 招招正中 + 突变如实申报）达到本会话最高水位。**verdict=pass ⇒
task-8 翻 complete**。

## 遗留风险清单

1. task-8（F2CA 族九入口）批产滚动中（批 12 起）；task-7（Mesen2 动态收口）交付后接
   复审（重点：动态证据场景覆盖面 + trace 可复现性）。
2. A7 same-source 弱化持续；独立模型抽核逐步扩面（现 33 案）。
3. P1-F2 盲区补案已闭（6423ce5 判别力实测 + F11 回归）；残余=各固有盲点案如实记账。
4. 复审方方法面：突变矩阵必须先过基模证真前置门（本案两度自纠：假 11/11、m1 定义错）
   ——「测试器先过证伪」在测试器自身的第二/第三案。
