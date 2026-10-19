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

## task-10 批 19：S5 帧引擎骨架（c73ab66）——60Hz tick/全内存/unported! 桩阵/l3-dump

### verdict: pass

**① 引爆面三自证复现 ✓**：`cargo test -p game-core frame` 4/4（boot_prologue_writes_no_ram
/ game_rng_detonates_until_verified / unported_detonates_at_bank_addr /
ram_mirror_and_watch_face）；l3-dump 引爆 CLI 复现原文
`panicked … unported @ bank07:C01A`（引爆即不落盘=禁半成品绿 ✓）。FFB0 reset 序
`78 d8 a2 00 … 8d 00 c0`（SEI/CLD/PPU 关/双 vblank/MMC1 复位写）+ 向量 FFFC=FFB0/
FFFA=C0D9 逐字节吻合；C01A=`a9 00 8d 00` 主初始化首址 ✓。
**② FrameMemory 镜像契约同源 ✓**：`addr & 0x7FF` 2KB 归一两侧逐字同（frame/memory.rs
与 l2ports.rs L2In 读写面），watch 面 zp=512/iram=3584 大写 hex 与 l3_manifest 同形。
**③ GameRng C 级 fail-loud 姿势 ✓**：算法未证不发明（N-0012 入账+unported! 桩+
game_rng_detonates_until_verified 单测钉死）——与 task-13 §2「L3 双腿启用硬前置」
一致。台账 N-0009..N-0012 四条证据指针齐（A/B/C 级分布如实）。
**④ 栈页残迹口径——实质成立，两处精度偏（P10-F1）**：主导帧态 **19 非零字节 ✓**
（$01EB-$01FF）；帧数实测 **292**（申报 293，偏离帧=0-7 开机段）；「$01FE 单字节漂移」
实为 **$01FC-$01FE 三字节值漂移**（主导窗内值有变位置实测）——措辞/计数需校，
残迹存在与调用影子结论不变。

### findings 汇总

| # | 分类 | 内容 | 处置 |
|---|---|---|---|
| P10-F1 | 声明精度 | 栈页残迹：293 帧→实测 292；「$01FE 单字节漂移」→实为 $01FC-$01FE 三字节 | 回 porter 校 doc 口径；非阻断（实质成立） |

两层三口径终态：frame/ 产品层零地址（13 hex=size/mask）✓；全闸绿（rust-tests 34、
naming 12、proof-checks 触发面 SKIP 显式）。

## task-10 批 20：C158 单元端口（faab3ac）——VRAM 清屏双遍 + L2 组件反哺首例

### verdict: pass

字节核全通：C158=`20 6d c1 a5 1d f0 06 a5 1f c9 1d f0 04 a9 02 d0 06 a9…`——
**双遍结构**实锤（首遍 JSR sub_c16d=验证件反哺：C16D=`a9 01 85 01 a9 ff 85 00`
与 l2-c16d-set-ptr-lo-hi 契约面 a=FF/$00=FF/$01=1 逐字对上；次遍经 C16F 尾入
STA $01）；分派 `C9 1D`=**CMP 立即数**（陷阱同族正确）⇒ $01∈{2,3}；尾共段
`a9 ff 85 00`=$00=FF；PPU 面 `ae 02 20/a5 ff 29 fb 85 ff/8d 00 20/a6 01 ca/bd 9f
c1/8d 06`（$FF&=!4/STA $2000/表[$01-1]→$2006）✓；C19F 表=`20 24 28 2c` ✓。
引爆点迁移自证复现：`unported @ bank07:C1A3`（C158 已消化）✓。单测 21+8 全绿、
全闸 PASS、基线 37。**L2 组件反哺首例**（C16D 44 案时代验证件经 l2_apply 桥喂帧引擎）
=两层「判定层不变、结构可换」实证；PPU 面不在 RAM watch、渲染面保真留 task-13
=口径诚实。无新 findings。

## task-10 批 21：C1A3 单元端口（5843753）——OAM 影子清屏 + write_io 面隔离

### verdict: pass

字节核 ✓：C1A3=`a0 02 84 01 a0 00 84 00 a0 00 a9 f0 91 00 c8 d0 fb a5 1d f0 03 4c…`
（ptr=$0200、$F0×256 填充循环、$1D≠0 尾跳）；$988A（bank0 窗口约定）=`a5 1f c9 1d
b0 1d a5 c8 f0 19` 双检+boot 短路 → $98AD=`60` RTS ✓；bank0 归属自证口径（FFB0 序
MMC1 control=$FF mode3 + C01A 序 PRG=0）成立。引爆点迁移复现：`unported @ bank07:C4B2`。
**链规⑤活例第 3 案（测试器实抓）**：write_ram 的 `&0x7FF` 镜像归一将 PPU_CTRL=$90
经 $2000 写**漏染 RAM $0000**（tick_frame assert 144≠0 当场抓）——修复=write_io
寄存器面唯一入口（PPU/APU/手柄不入 RAM），回归测 `write_io_does_not_leak_into_ram_mirror`
钉住（我方复跑 ✓）。单测 24+8、基线 40、全闸 PASS。游戏态 $988A 路径未端口=显式
unported! fail-loud ✓。无新 findings。

## task-10 批 22：C4B2 单元端口（dd8093e）——MMC1 control 串行写

### verdict: pass

字节核 ✓：C4B2=`ea ea a9 47 4a 4a 4a 29 01 85 00 a5 25 29 fe 05 00 85 25 8d…`
（bit 提取→$25 位合成→STA $8000 串行）、C4D9 变体=`a5 fa 4c b6 c4`（尾跳 $C4B6，
未入 boot 序 doc 注记 ✓）；boot 态 control=0x0E（mode3）与 $988A bank0 归属自证
一致 ✓。引爆点迁移复现：`unported @ bank07:C4DE`。残桩复验：grep 双形 0 命中 ✓
（其自录链规②反面教材——首击静默未命中被测试器抓、grep 自证归零——处置闭环）。
单测 25 全绿、基线 41。无新 findings。

## 滚动复审检查清单（增补，2026-10-15）

1. 语义逐 ROM 字节核（含映射约定声明后再读数）；
2. 三方真实性 + 独立模型（基模证真前置门）；
3. 突变敏感性（高危族×判别值域双门 + mutation_verified）；
4. **链规 17 已证等价原则**：①单元 spec 必须引 asm 段+注释——只现「重解码表」而无
   asm 段引用 = 违例判据 ②Mesen2 调用须带三用途标签（裁判 golden / bug 定位 /
   运行态确证），账本可查；
5. findings 三族标注（r10 数据源）：**读解误读族 / 契约族 / 声明族**；
6. **新文件落库核**：commit 声称的「（新）」文件 vs `git show --stat` 文件清单逐一对账
   + `git ls-files` 在册核（批 25 漏检教训：工作区未跟踪件掩盖提交树破）；
7. **S5 单元捕获面对表（判据增补，2026-10-15 立规级）**：S5 帧引擎单元在 L3 上线前的
   「对拍」步 = **vs 写点级捕获面**（decomp 场景 trace 可复跑面）——L2 单元有 golden
   兜底，S5 单元单测与实现同源（**模型错×单测绿=同源补偿**，P28-F1 升模型更正案
   实证：8AC7 TYA 位错而单测绿）——**捕获面对表就是它的裁判**；比对面=写点值域/
   写序/时序（如 $07A0 真机写值域 {0,6,12}/{33,35,68} 基准面）；
8. **产品层业务结构核（链规 19，镜像形态=违例）**：产品层按业务结构组织（状态机
   enum/类型化状态/方法族/惯用错误处理），**禁 1:1 镜像 asm**（入口数=方法数镜像
   映射/共享尾复刻/scratch 页直搬）——同行为面多 asm 入口应收敛为域方法族/状态枚举
   （F2CA 折叠/BootPhase=正面先例；task-23 链规 19 首证=同码折叠 6→4）。行为保真
   （含 quirk）≠代码形态保真。

## task-10 批 23：CheckBankSwitch/C4DE（e3da228）——链规 17 首件

### verdict: pass

四查全过：①字节核 ✓（C4DE=`a4 24 f0 2d 20 e8 c4 4c 10 c5`、C531=`a0 00 84 31 c8 84 1d`
与其 ROM 抽查佐证逐字同）②**链规 17① asm 引证主体** ✓——prg7_engine.asm:1296/1305/
1351/1363/1376 五行抽核逐行命中（符号+注释在位）③链规 17② Mesen2 三用途标签 ✓
（本笔明示「裁判」用途仅 L2 工厂 golden、无新调用）④引爆点迁移复现（C45D）+桩位
替换带断言（C4B2 教训固化 ✓）。C4B2 过渡注（asm :1256 PrepVertMirror 补注 offer）记档。
findings 三族标注（r10）：本批 **0 新 findings**。基线 42。

## task-10 批 24：CopyMap/A93E（89dcd24）——WorldMap 拷贝 + 命名同 commit 首例

### verdict: pass

四查全过：①ROM 抽查 ✓（WorldMap prg[0x253E..0x293E] 1024B、**非 FF=511** 逐数吻合；
4 迭代×256B+指针终态 inc×4 语义自证）②链规 17① asm 引证抽核 ✓（prg0_title.asm:5153
`CopyMap: ; 00:A93E`/5118 `WorldMap: ; 00:A53E`/constants_ram:1298 `WorldMapRAM ds $400
;$7000-$73FF` 三行命中）③命名同 commit（裁定①首例）✓——N-0019..N-0021 证据指针齐
（WorldMap/WorldMapRAM=A 级 asm 注证）④桩位表实态：CopyMap 桩已消化 ✓（余 C5D7 链次
桩+C0D9 NMI 面桩+988A/C51F 游戏态桩）——实跑先触 C0D9=场景执行序使然，与链次申报
不矛盾但措辞注：链序≠实跑序，自证以桩位表+单测为准。findings（r10 三族）：0 新。

> 【方法面补录】否定结论第三查「**换相位**」（decomp
> gotcha-sampling-point-is-blind-spot，源自 C0D9 冲突案）：负向断言复核=换工具×
> 换时刻×**换相位**三维——采样点相位盲区会把「零写」变假阴性。入检查清单第 5 条并列项。

## task-10 批 25：NMI 面单元/C0D9（945fe41）——§8.1 契约实施件

> 【2026-10-15 更正·撤回 pass】复验发现 **commit 不完整**：message 声称「frame/nmi.rs
> （新）」但 `git show --stat` 实际 8 文件无 nmi.rs（盘上有、git 未跟踪）——mod.rs 已
> `pub use nmi::nmi_handler;` ⇒ **HEAD 提交树构建破**（工作区被未跟踪件掩盖，我方
> 单测跑在工作区故未觉）。原 verdict=pass **撤回改判 needs_revision**（待 nmi.rs 补
> 入库后复验翻 pass）。两层教训：①手术灾难回滚后漏 git add（链规 14 漏步，文本手术
> 家族+1）②**复审方漏检「声称新文件 vs git show --stat 落库对账」**——检查清单已补
> （「声称≠落地」家族第五案，实施/复审双层盲）。

### verdict: needs_revision → **pass（修后复验，82d627a）**

> 【2026-10-15 修后追记】nmi.rs 补入库（82d627a，四重自证）——我方独立复验：
> git ls-files 在册 ✓ / git show HEAD:nmi.rs=76 行 ✓ / create mode 自证 ✓ / nmi 三测
> 复跑过 ✓。批 25 实施面质量维持原评（spin 状态机/契约断言/asm 引证全过），
> 唯落库缺失已闭。结构性加固另落 gates（commit-tree 机械门，见 gates commit）。

四查+契约核全过：①asm 引证抽核 ✓（:251 超帧门 `lda NMIStatus`、:279-280
`ldy #$01/sty NMIStatus` 恒量写、constants_ram:474 NMIStatus=$1A 三处命中）②
**spin 跨帧状态机** ✓（BootPhase: PreSpin→AwaitNmi→PostSpin→MainLoop；tick 序=
NMI 尾写 1→spin 唤醒→帧工作→清 0→spin）与 §8.1①定案逐句对上 ③**契约断言三件套**
✓（nmi_tail_writes_constant_one / f=1 帧界锚 / 超帧门双测实跑过；栈页排除=比较器侧
tools/ 面——lead 移交 verifier 落，scoping 诚实）④引爆面场景序 ✓（tick2=NMI 先行→
`unported @ bank00:9A07` 精确引爆，与我批 24「链序≠实跑序」注自洽）。
工程注（文本手术三案并档）：切片锚序倒置→空切片→replace("",new) 插文件头——
编译 E0753 拆穿+回滚+锚序断言（ia<ib）重做，教训「切片手术先断言锚序」入案 ✓
诚实文化满分。findings（r10 三族）：0 新。基线 47、全闸 PASS。

## v4 路由两实施项修复回归（7e94719，2026-10-15 追记）——双闭 ✓

- **stale fail-open 防御**：l3-dump run 起点删旧（清理失败=fail-loud 拒起跑）+临时名
  +rename 原子写（半成品绿面双层封）——`[stale-clean]` 行+dump 条目数 0 实证在码 ✓；
- **GameRng 桩带址**：`unported!("bank07:C000")`——asm 自证 `RandomNumbers: ; 07:C000`
  （prg7_engine 内容锚，主环每帧 jsr）行核 ✓；引爆行检测面形态合规（无址违规已纠）。
- stat 对账：3 文件（engine/rng/l3-dump）与意图一致（清单第 6 项检查过）。
- decomp 侧 5aeb48d（「二选一」错账→三家 $C0C9/$C436/$C10B 更正注）同步收讫 ✓。

## task-10 批 26：NMIScreenWrite/9A07（95e43a5）——boot 态短路件 + 命名同 commit

### verdict: pass

四查过：①ROM/asm 锚 ✓（9A07=`a5 1f c9 1d`（LDA $1F/CMP #$1D）字节吻合；内容锚
`NMIScreenWrite: ; 00:9A07`+「If titleRoutine not at end game, exit」注+
MainRoutine/TitleRoutine=$1E/$1F 常量注址命中；boot/title 态短路零 RAM 写=
单测 `assert_eq!(m.ram, [0u8; 0x800])` 钉死）②链规 17 ✓（asm 引证主体+Mesen2 无新调用）
③命名同 commit ✓（N-0025..N-0027）④引爆迁移复现 `unported @ bank07:C1E0` ✓+
stat 对账（3 源文件+台账+基线=意图一致，无新文件声称）。
**P26-F1（声明精度·轻，行号族）**：Exit100 行号「:3608」实漂（refs 在 3594/3613、
定义其后）——内容锚 `Exit100: rts` 在，行号不作引证依据（F6 口径）；轻症记档。
findings 三族标注（r10）：1 轻（声明族）。基线 49、单测 33 全绿。

## task-10 批 27：CheckPaletteWrite/C1E0（128e0a6）——NMI 调色板三路路由

### verdict: pass

四查过：①asm/ROM 锚 ✓（C1E0 三路路由字节面+内容锚 `CheckPaletteWrite: ; 07:C1E0`/
`StarPaletteSwitch: ; 00:8AC7`/`EndGamePaletteWrite: ; 00:9F54` 命中）；**_id 表序
自数证复核**（我方重数）：StartContinueScreen15=第 22 位（0x15 ✓）、EndGame=第 30 位
（0x1D ✓）——与 NMIScreenWrite `cmp #$1D` 阈值互证自洽，三点结论全真 ②链规 17 ✓
③命名同 commit ✓（N-0028..N-0030）④引爆迁移复现 `unported @ bank00:8AC7` ✓（次桩=
CheckVRAMStructBufferWrite 与声明一致）。
**P27-F1（声明精度·轻，计数族）**：表总长「30 项」实为 **31 项**（第 31=SetTimer，
宏行总数 31 实数）——EndGame=第 30 位的索引结论不受影响，总数计错一位记档。
findings 三族（r10）：1 轻（声明族）。基线 50、单测 34 全绿。

## task-18 批 28a：EnExtra 命名债清偿（211c623）——15 真名+残余显式入账

### verdict: pass

四点核：①**15 字段三重证据形制 ✓**（N-0031..N-0045 逐条抽核：A 行为触达
（bank07:F28E `BD F4 6A`/`C9 04` status=frozen 面）+ B Object 同构（constants_ram
`.struct EnExtra`/`.struct Object` 同名同注）+ B 用例静态（nmikstas/prg3_tourian
`sta EnsExtra…` 用例面）——证据指针 18/18 在位）②**社区 Unknown 四字段自证 ✓**：
subPixel 族=areas_common.asm:898 内容锚「apply sub-pixel speed to sub-pixel position」
（引文逐字命中）+ IntX/IntY 同构；accel 族=`lda EnAccelYTable,y`（prg7_engine 内容锚
命中）表喂互证——C→B 升级路径成立 ③**真不知 5 项 ✓**：N-0046 data1F=**拒采**（§3.2
伪语义名纪律执行面）+「移动指令位模式修饰器假设，待行为自证升级」——假设级显式、
**未越级当结论** ✓；N-0047/0404、N-0048/040E 留 _todo_+C 级假设注（三矿补查
m1disasm 无名/nmikstas 用例面随记）=纪律正确 ④P27-F1 计数补正：当前无新引用面
（grep 30/31 项零命中）——**下次引用 TitleRoutinePtrTable 须带 31 项**（记档待办）。
轻症 2：areas_common 引证路径粒度（实址 SRC/areas_common.asm 非 brinstar/ 族，
内容锚在不碍复现）；stat 对账 6 文件=意图一致 ✓。

findings 三族（r10）：0 阻断、2 轻（声明族）。基线 48→55、命名 30→48（含批 26/27 后续）。

## task-10 批 28b：StarPaletteSwitch/8AC7（ef1c9c3）——intro 星闪轮换（施工令五步节奏）

### verdict: pass

五点核：①asm 内容锚 ✓（`StarPaletteSwitch: ; 00:8AC7`+「every 16th frame/Is any
other PPU data waiting」注+双门（FrameCount&0xF/VRAMStructBufferIndex）+表游标
（IntroStarOffset&7→asl→tay）+表查+inc 链逐段命中）②**ROM 表提取逐字 True**：
PtrTable prg[0x0AFF..] 8 项=8B0F/8B19/8B23/8B2D/8B37/8B41/8B4B/8B55 与烘焙面同源 ✓
③单测 37 绿（双门+表游标态）+引爆迁移 `unported @ bank07:C2CA` ✓——**与执行序发现
自洽**（消费端 C2CA 先于生产端 C37E=实测下一件即 C2CA）④intro.rs 在册 ✓（git ls-files
命中+stat 对账 3 处——「第五案 add 失实」修复实证+checklist 第 6 项立规执行面）。
**P28-F1（声明族·中，交叉核项）**：「清零环 Y 型填充使 $07A0=0xA0（**真机同态**）」
在仓内工件面**不可复证**——boot-title frames 300 帧 $07A0 全 0（中间态在 f=1 采样窗外
或未捕获）；主张现仅引擎侧单测支撑。处置：与 decomp VRAMStruct 宏族线**成对对表**
（其捕获面是否见 0xA0 中间态）或补 boot 段中间态捕获；「真机同态」措辞在对表前降级
「引擎侧同态」。
findings 三族（r10）：1 中（声明族）。基线 61。

## task-10 批 28b 重裁（50af369 TYA 更正后）+ 批 29（e9dd8c4）——2026-10-15

### 批 28b verdict: pass（原 pass 撤回后重裁通过）

> 【模型更正案全程记】P28-F1 升级模型更正案（8AC7 TYA 位错一位=「值=Y」误译）——
> **模型错×单测绿=同源补偿**实证；新判据（链规 18 捕获面对表）即由此案立规。

三点重裁：①**TYA 修正核 ✓**：asm 结构实证 `tya ;A = 0` 在 @loop_A **环外**（BNE 锚
@loop_B=STA 环写 A=0）——零填充模型正确、旧 Y 型=解码错位一位（第六案入档）②
**撤回注核 ✓**：intro.rs 更正注 3 处+N-0049/N-0052 台账注含更正（原地留痕）；执行序
发现撤回（「消费端先于生产端」为误读产物——生产端先行=真实序）③**捕获面对表 ✓**：
零填充模型 vs 真机写值域 {0,6,12}/{33,35,68}（9e39d0d 写点级）——模型写值∈捕获域、
0xA0 零命中互证；引爆断言回 `bank07:C37E`（生产端先行）复现 ✓。测 40 绿。

### 批 29 verdict: pass（CheckVRAMStructBufferWrite/C2CA，e9dd8c4）

五查+第 7 查：①asm 内容锚 ✓（`CheckVRAMStructBufferWrite: ; 07:C2CA`「Exit if no PPU
data」/「start at address $07A1」/「data…now erased」注+清三态体）②**NMI 工作序漏桩
自查补全 ✓**（WritePPUCtrl C44D/WriteScroll C29A/ReadJoyPads C215/SoundEngine B3B4
逐位对齐 asm 调用序——自查抓回跳步=执行序失真根治）③单测 7 绿（boot 静默/pending
引爆双面）+引爆迁移 `bank07:C44D` 与消费端序自洽 ④PPU_DATA_PENDING 社区注自相矛盾
如实记录（取生产端 sty 语义）✓ ⑤**捕获面对表（链规 18 首用）✓**：清三态模型写 0∈
捕获域、boot 静默（$1B=0 生产端未跑）与捕获 boot 相一致。
findings（r10）：0 新（28b 模型更正案已结）。基线 63。

## task-10 批 30：PrepPPUPaletteString/C37E（09f3918）——VRAM 写串解释器（结构互核注首件）

### verdict: pass

四查+第 7 查：①asm 内容锚链 ✓（PrepPPUPaletteString/LC385 [hi][lo] 序/WritePaletteStringByte/
WritePPUByte/NextPPUByte（$4F 上限）/EndVRAMStruct（pla/pla 逃逸）/SeparateControlBits
（and #$BF/#$3F）/AddYToPtr02 逐符号命中）②**数据岛自证 ✓**：IntroStarPalette 8×10B=
prg[0x0B0F..0x0B5F] 80B 整（我方提取长核）+双流几何（stream1→$3F19/stream2→$3F1D
10B 块）③单测 2 绿（16 帧门+双 call 缓冲 13B 逐字节断言）+引爆迁移 `bank07:C2E4`
=渲染面下一件与声明一致 ④**捕获面对表（链规 18）✓**：[hi][lo][ctrl|data] 线格式与
decomp A 级格式锚（VRAMStructData/Repeat/End 宏族 28/28 解析）双证——解释器输出
几何∈捕获面格式域。
**结构互核注三面**（施工令②补款首件）：y 复用三相/分支目标互核（bne 回数据节/
bvc-bvs 双 iny 路径）/数据岛边界——第六案「解码错位一位类」防线制度化 ✓。
端口过程两案自录（链规②）：①段间 iny 双计越界（相位误读族又一实例，互核注固化）
②测试期望 lo/hi 序写反（**期望值错非模型错**——[hi][lo] 双证裁明）——诚实分级 ✓。
findings（r10）：0 新。基线 66。

## task-10 批 31：VRAMStructWrite/C2E4（3fdfc7b）——消费走带（对偶镜像）

### verdict: pass

四查+第 7 查：①asm 锚三处命中 ✓（`VRAMStructWriteData: ; 07:C2E4`/`VRAMStructWrite:
; 07:C30C`（ldx PPUSTATUS 清 flip-flop/jmp WriteScroll 尾跳）/`SetPPUInc: ; 07:C318`
（sta PPUCTRL+PPUCTRL_ZP）——地址与内容锚逐对）②**结构互核注②件 ✓**：段内 y 推进
与生产端（批 30）互为镜像（非 RLE 每字节/RLE 值一次）——成对对表点自洽；尾跳栈深
一层语义（WriteScroll RTS 回清理段）核对成立 ③单测 3 绿（走带镜像+ $FF 增量位断言+
尾跳引爆）④引爆迁移 `bank07:C29A` ✓——**同帧生产→消费全通→尾跳**=真实执行序
（C37E→C2E4→C29A 链贯通）。SetPPUInc 动 $FF=RAM watch 面可观测副作用显式标注 ✓。
findings（r10）：0 新。基线 67。

## port2 task-14（fc9acbb）：A9 处置——ROM 窗口表烘焙+window_bank 契约多窗口族

### verdict: pass

### findings（逐项复现输出）

**① 突变敏感性——通过 ✓（我方独立判别力实测）**：基模证真先行「L2 三方裁定:
360 条用例, 0 失败」复现 ✓；其实现侧突变（window≡7）RED 26/32 引用在案+巧合双案
（b-f282 w0/w3）自报诚实；**我方输入侧翻窗突变**（32 案 window_bank+1 mod 8，
/tmp 拷贝非侵树）：**31/32 打回**——幸存=b-f282-table-exit-w7（巧合族对称，与其
自报巧合面互证）；族分布 8 bank×4 均布 ✓。判别力承重实证。
**② ROM 字节级——通过 ✓（我方重导逐字）**：ROM_VEC_BY_BANK 8 向量两两不同=True
（w7=03 38 与我 A9 时代先证一致）；TABLE_968B 抽样跨窗异值 ✓；烘焙面逐字吻合+
逐 bank prg 偏移注在位。
**③ 契约面——通过 ✓**：verify_shim 四站全窗口参数化；l2-diff `None=>bank_of
(&case.entry)`（缺省=既有行为钉死，360/0 复现）+`w<8` 越界 fail-loud ✓；
l2-casegen 显式携带才出键 ✓。bank 标签≠窗口 bank 两面分离如实。
**④ 链规 15——通过（含 P14-F1）**：ROM 常量住 memmap/rom_tables（正位）✓、
flow_scratch 原地更正注 ✓。**P14-F1（声明族·中）**：「产品层零地址零机械名未破」
字面不实 repo 级——frame/ 体代码累计 132 处 ≥4 位 hex 字面量（io 命名常量块=正位，
但 `read_ram(0x001D)` 类体使用=S5 积累态，非本 commit 所致）；处置：声明收窄
「task-14 未新增」（为真）或另开 frame/ 体命名清扫工单——非本件阻断。
**⑤ 证据面——通过 ✓**：CHECK: PASS（PASS=11 FAIL=0 SKIP=2）复现；基线 67→70、
60→61 status 实读仅上调 ✓。

（本批残余：①frame/ 体字面量清扫决策——随 S5 L3 上线前一并裁定 ②b-f282 巧合族
（表 bit4 同侧）=固有盲点已自报，突变族谱补录。）

## task-10 批 32：WriteScroll/C29A（92b47f6）——滚动寄存器件（双路汇聚）

### verdict: pass

四查：①asm 锚 ✓（`WriteScroll: ; 07:C29A` 体四行 lda PPUSTATUS/ScrollX→PPUSCROLL/
ScrollY→PPUSCROLL/rts 逐句；constants ScrollX($FD)/ScrollY($FC)）②结构互核注 ✓
（零 RAM 写只读 $FC/$FD+双路汇聚同一 RTS 语义——尾跳路栈深一层回清理段/直调路回
NMI 体，域形折叠核对成立）③走带清理段首次通达（三态清 $07A0/$07A1/$1B 可执行=
镜像/pending 测完成态断言）④测面 44 绿（树愈后补跑——初测时 WIP 半重构 window_bank
破树与本件无关，定损分离）。findings（r10）：0 新。基线 67。

## port2 task-20（0ae446a）：GameRng 算法定谳——N-0012 升 A（S5 硬前置解锁件）

### verdict: pass

### findings（逐项复现输出）

**① v5 谓词机械可检（验收核心）——通过 ✓**：我方谓词三则实跑——`N-0012.level=A` ∧
evidence 命中 `bank07:C000`（bankNN:AAAA 具体正则）∧ sub_GameRng 族 8 案 ⇒
**三则全真: True——S5 启用门谓词首次真开**（v5 机械门与本次交付闭合）。
**② 闭式 vs 26B 逐条核——通过 ✓（我方解码）**：C000-C019 逐字节解码=环 +0x05×5
（RN1+=0x19）/环 +0x13×5（RN2+=0x5F）✓、`d0 ef` 偏移自证（$EF 非 $F1 自纠核实）✓、
返回 A=RN1 ✓、种子序 C0AC=0x11/0xFF ✓。k=调用计数非帧号=③实证支持。
**③ L2 真实性+突变——通过 ✓（真突变手术）**：基模 368/0 复现；真突变（RN1_STEP
0x19→0x1A 备份-改-跑-还原）：单测 2 红+**L2 族 8/8 全打回**（判别力承重）；还原
368/0 ✓（自省注：首突变 sed 空转=定义错自查拦截，P1-F3 又一案）。
**动态独立重导**：f=1 种子 0x11/0xFF ✓；帧差 299=295 调用+4 零调帧（boot 前缀）
与申报 295 吻合；闭式违例（调用帧口径）=0 ✓。**P20-F1（声明精度·轻）**：「增量集
单元素」=调用差集口径（全帧差集={0,0x19}）——建议措辞带口径。
**④ every frame 注样本——通过 ✓**：「调用聚合注」分类正确（4 零调帧=注≠语义具体
反证）⑤矩阵格 S-FUNC-07-C000 L2/covered+证据指针 ✓。

（本批残余：frame::GameRng 句柄仍桩——算法件已立（src/rng.rs），S5 引擎接线
（句柄→RngState）待续，与 L3 实施序 §5 步骤 3 衔接。）

## port2 task-9（6760260）：UpdateTileBlast_Respawned/FE83 域化+F311 伤害链随钉

### verdict: pass

### findings（逐项复现输出）

**① 段界/调用集更正核——通过 ✓（申报精确）**：址精读定谳 `FEC9:4c FECA:92
FECB:ce`=JMP $CE92 **恰跨 FEC9-FECB**、`FECC:bd`=下标签 GetVRAMStringPtr 双证——
73B/34 条边界更正成立（r2 72B 实为失实，第六案族对称更正规范）；段内 jsr 目标
重导=**[FF3C, F186, F1FA, F311] 四员逐字命中**（r2 漏 F311 实锤）；段首 `a9 00 9d
00 05…`+**`18 69 21`（CLC ADC #+0x21=进位丢弃 quirk 结构自证）**。
（自省注：我方探针两错——bank7 映射第五案+尾端独占切片——正址重导后申报全对。）
**② 迁移双绿——通过 ✓**：f186/f1fa 核抽取后 377/0 复现（既有 368+9 新案全绿）；
薄包装形制（l2ports 委托 verify_shim）=「行为钉死=结构可换」实证。
**③ carry 面+极性突变——通过 ✓（手术实测）**：`f1fa_with_carry` C=1 无碰撞/C=0
重叠显式不入 L2Out（契约不扩照裁定）；**极性翻转突变（if carry→!carry）：L2 9/9
全打回**（恰 fe83 族，判别力承重）+还原 377/0 ✓。（观察：单测 54 对极性不敏感=
L2 golden 才是该轴裁判——链规 18 同源原理的 L2 侧镜像。）
**④ 伤害链写面入 golden——通过 ✓**：collision-horiz 案 golden writes 含
**010F/030A/0073/006E/006F** 五键（申报链面全数）+指针槽位；quirk 夹具（$0508=$FF）
+单测 `respawn_addr_drops_carry` 钉死 ✓。
**⑤ 账面——通过 ✓**：S-FUNC-07-FE83 补录格+段切更正注同 commit；基线 82/65 实读
（77→82/63→65 仅上调）；N-0064/0065 指针在位。
findings（r10）：0 新（含其自报的 r2 双失实更正=第六案族规范样本）。

## task-10 批 33：WritePPUCtrl/C44D（b1014c6）——PPU 控制同步+共享尾抽取

### verdict: pass

四查：①asm 锚 4/4 命中 ✓（WritePPUCtrl: ; 07:C44D/ExitSub: ; 07:C45C 共享 RTS/
SetPPUMirror: ; 07:C4B6（lsr×3/and #01/ora 串行）/PrepPPUMirror: ; 07:C4D9——
共享尾三路汇入（C4B2/C4D9/C44D）结构复刻=域形共享 fn ✓ ②共享尾副作用面显式
（动 $00/$25=MMC1CTRL_ZP 可观测；寄存器面 write_io 同步）③测 56 绿（c4b2 共享
复用后全过=结构可换实证）④引爆迁移 `bank07:C215`（ReadJoyPads 输入面次件）✓。
**事故注·共享账本撞号**（自录）：port2 并行 append 撞 N-0061..N-0064 序列——
重排 N-0066..N-0069+ID 唯一断言自证（我方复核 69 条唯一 ✓）；新纪律「共享账本
append 前查 max-ID」=共享 git index 同族并行面，入案 ✓。
findings（r10）：0 新。基线 69。

## port2 task-22（4e7346d）：L3 实施槽机械面——v5 §5 对应项勾销核

### verdict: pass

### findings（逐项复现输出）

**四槽↔v5 §5 勾销**：①严重-6 ✓：memmap::frame_face=字段面唯一源（zp/iram/wram
编码器+WATCH_FACES；三面 512/3584/16384 hex 冒烟）+watch_snapshot 薄委托（lead
例外授权=只动结构不动语义）+l3-dump 声明集过滤（未声明标签 fail-loud exit 2）
——〔归属纠偏：frame_face.rs=port2 task-22 新件（f3855d2 pathspec 吸入 in-flight
mod.rs 声明而漏 untracked 新件=破树根因，94be180 补入；非 porter 面——第五变体
「新件与声明同刻 add」两方各吃）〕②严重-5 ✓：**wram 0x6000-0x7FFF 全 8KB** 实读
（双侧同表；软上界作废=全状态原则；与 FrameMemory 8KB 同框）③严重-7 ✓：**缺
frame_keys 坏例现场 exit 2**（「可为空数组缺键不行」实跑）+w 标签⊆声明集+坏例
2/2 打回链 ④严重-10 ✓：scenario_sha256 对锚 True 实测+溯源链 note。
**零回退 ✓**：trace-diff 1 案 0 失败复现；85 测。**v5 §5 剩余槽在册**：3/3b（采样
迁移）/per-leg sample_point 键/5/6。

（方法面：stat 对账升格**逐文件归属核对**（第五变体）；共享树禁 stash（第七案）
——我方验证改 worktree 法；FAIL 文案归属线索=gates backlog 低优先。）

## task-10 批 34：UpdateAge/C97E（7e4e916）——NMI 体收官（全通里程碑）

### verdict: pass

四查：①asm 锚 ✓（`UpdateAge: ; 07:C97E`+三重门体（GameMode/MainRoutine==GameEngine/
FrameCount 节拍）+SamusAge 使用面命中）②**速核更正自录 ✓**（假设值 0x34 作废→
真址 $687D ds 4 WRAM 进位族——constants_ram 内容锚；三面同刻更正常/测/账=纪律好）
③单测 59 绿（标题零写 WRAM 断言+游戏路径引爆）④引爆迁移 `bank07:C5D7` ✓——
**NMI 体全通里程碑实证**（C0D9 体八件 boot 短路面贯通，引爆点移出 NMI 体进
boot_post_spin 链）。findings（r10）：0 新。基线 83。

## port2 task-23 批1（8a587fc）：title ③助手簇——链规 19 首证（同码折叠 6→4）

### verdict: pass

### findings（逐项复现输出）

**① 链规 19 形态核（首证）——通过 ✓**：三对同码**字节级实证**（我方 ROM 重导）：
`806E: e6 1f 60`（0A/0B 折 1）、`82F3: 60`（13/14 折 1）、15/1B 同址 $90BA（指针表
同目标）；`8068: a9 00 85 cb 85 c9 e6 1f 60` = **9 字节连续**（ClearSpareMem 六字节
落穿 806E 共享尾）——落穿实证逐字。折叠单元=4 域方法非镜像 fn ✓（注册面折叠/用例
id 分入口=行为与代码解耦正形）。
**② L2 逐入口钉——通过 ✓**：384 条 0 失败（377 零回退+7 新案：0A/0B/13/14 各自
golden+回绕/Timer3 覆盖/落穿边界）；测 64 绿。
**③ 结构互核注——通过 ✓**：「160 frames」经 Timer3 递减速率三重自证=**成立**
（非「注≠语义」反例——口径精确）；SpareMemC9 dw vs zp 写面以字节为准 ✓。
**④ 写面教训 ✓**：no_op 臂误发写事件被 golden 当场抓（「不写≠同值写」b-f270-a0
族）→Option 写面修正；突变抽查（注掉落穿推进）→b-title-clear-10 打回。
**⑤ 过程诚实注 ✓**：突变还原 git checkout 误吞修复（共享树 checkout 风险族=
链规 13b 同族）——重施复绿随记。
**批 2 绿灯信号**：无系统性问题——链规 19 形态/同码证据标准/逐入口钉解耦面均为
正样本，批量放大安全。findings（r10）：0 新。

## port2 task-23 批2（05b130a）：title ④编排簇——3 L2+2 L3 降级显式

### verdict: pass

### findings（逐项复现输出）

**① 写面填充模式钉——通过 ✓**：389 条 0 失败（384 零回退+5 新案含 2230 写面全量
逐址）；b-title 族 12 案。
**② 语义差注（全逐字核）——通过 ✓**：`82A3 段全解码`——prep_restart 的 `sty $1F`
Y=2=**INY 链（c8×2）置位非推进**逐字实证（LDY #$1F 清环→Y=$FF→c8→Y=0 十一项
zp 清→c8→Y=1→c8→Y=2→`84 cf 84 1f`）；`a9 10 8d 84 06`（SFXMultiInitFlags=$0684
**RAM 地址非 APU**）+`a9 02 85 d8/c6 d8` 音乐双式逐字吻合。
**③ L3 降级显式核——通过 ✓**：01/12 矩阵格 L1+「L3 降级（R3）」note 非静默
（12=NMI 等待自旋=单子程序模型内死循环不设案=正确裁定）；域实施待 shell 组合点
（链规 19 不硬造镜像 ✓）。
**④ 生成器自纠 ✓**（X/Y 透传误写被模型抓+ROM 确认）。**链规 19 第 8 查**：3 方法+
2 显式降级=业务结构形 ✓（无镜像形态）。
**批 3 绿灯**：形态延续安全。findings（r10）：0 新。

## port2 task-23 批3（01da1ad）：title ①淡变族——FadeDriver 同形折叠正样

### verdict: pass

### findings（逐项复现输出）

**① chain 19 正样核——通过 ✓**：4 入口→1 tick 骨架（Timer3 门→FrameCount 掩→
端测/步进二择）+4 枚举臂——非镜像 fn（第 8 查过）。
**② 双表结构互核（本批主发现）——通过 ✓（我方 ROM 重导）**：`8B6D 表 12B=
0d 0e 0f 10 01 | ff | 01 10 0f 0e 0d | ff`——[0..4] fade-out 序/[5]=$FF/[6..10]
fade-in 镜像序/[11]=$FF **逐字实证**；0D `cmp #$0B`=终止符位索引（表象矛盾解）
+0E 端测==5 后置索引←6 循环闭合=结构读法成立。
**③ Y 语义钉**：步进器 ldy 覆写（步进=索引/门端=透传）逐案 golden；PalSelect
idx10/11 双 0c 字节为准 ✓。**④ window_bank=0 显式钉窗**（A9 契约面复用）✓。
**⑤ 流程自首裁量（链规③家族）**：commit 引「400 条…刚打印」实为落地后推算
（时点打印=389 行）——**数字真（我方复现 400/0）、标注滑**（「刚打印」时点不实）。
裁量=**轻注**（E 类边缘免升级三由：自首+数字实证真+修复纪律入案「commit 引数
只取 commit 前最后一次 check 完整行」）；r12 Evidence 计数失实同族记档。
findings（r10）：1 轻（声明族·已自首）。批 4（②特效族 6 件）绿灯延续。

## port2 task-23 批4a（3f5f15e）：title ②特效簇——「asm=规格非模板」金样本

### verdict: pass

### findings（逐项复现输出）

**① struct 偏移机器码重导——通过 ✓（金样本成立）**：`ad aa 6e`（LDA $6EAA）=
bank0:$8151 实存（complete=0x0A 非列表 0x0C ✓）；bank0 6E 页绝对,X 操作数偏移集
={A0..AA,AC,AD,B0,E0}——**xChange=7/changeDelay=8/byteType=9/complete=0x0A 全在集**
✓（列表声明 `.db` 非布局分配=「asm=规格来源非代码模板」的教科书实证；偏移从机器
操作数提取=正方法）。
**② 注释≠语义 ×2——通过 ✓**：尾补丁 `a9 6b 8d b0 6e`@bank0:$87B9 实存（「$6EB0=$6B」
注=补丁后状态、代码=同值装载+尾补丁——节选遮蔽实锤）；byteType「branch always」注
=数据主张非字节事实（口径精确）。
**③ BUG 保真 ×2（D4）——通过 ✓**：tileID eor #3（Should be C6/C7）+尾补丁行为按
有 bug 行为出 golden（Stage A 不修+N-0090 注记）=保真纪律执行面。
**④ 运动链 26 写面全逐址+落穿双跑 4 串行**（第二跑读写后态）✓。
**⑤ TDD 自纠四轮**（struct 偏移/X-Y/尾补丁/相位写）均打回定位——模型独立性战绩续 ✓。
409 条 0 失败（400 零回退）复现 ✓。findings（r10）：0 新。
**批 4b 绿灯**：形态延续安全（BUG 保真注记口径照此）。

## task-10 批 37：InitTitleGFX/C5D7（4b5c80c）——GFX 载入序列+描述子烘焙面

### verdict: pass

四查：①GFXInfo 8 件×7B ROM 提取 ✓（**[0]=06 00 80 00 00 a0 09=bank6:8000→dest0
size 2464——与 task-17 r6 GFX 转运表首项互证**（跨 task 数字对上=数据链×载入链同源）；
8 件 hex 逐字在案）②结构互核注三面 ✓（7B 描述子=宏体自证/BankSwitch 双切（$00 终态=
CurrentBank 回切）/数据岛边界）+速核更正链第 2 例（JustInBailey 假设 $32 作废→$69B3
三面同刻）③单测 1 绿（末件 Solid_1FC0 终态断言）④引爆行 `bank07:C0BC`=**成环件
先行于链序申报**（「次桩=C487」=boot 链序；实跑序先触 C0BC 主循环相=桩表双桩并存，
措辞注同批 24 族；C5D7 已消化 ✓）。findings（r10）：0 新（措辞注不计）。基线 98。

## task-10 批 38：NMIOn/C487（c07e23d）——boot 链全通里程碑

### verdict: pass

四查：①asm 锚 3/3 ✓（`NMIOn: ; 07:C487`/`LC481: ; 07:C481`/`bne LC481 Branch always`
逐行命中；PPUCTRL_VBLKNMI_ON 面）②结构互核注 ✓（PPUSTATUS 轮询瞬时级=序幕双 vblank
先例衔接/NMI 开关配对面 ScreenNmiOff↔NMIOn（$FF bit7+write_io $2000+ nmi_enabled 复启）
三面同步）③测 72 绿（三 tick 引爆断言=tick2 boot 链全通+tick3 主循环相）④**boot 链
全通里程碑 ✓**——reset→C01A→C158→C1A3→C4B2→C4DE→C531→ScreenNmiOff（spin 跨帧）→
CopyMap→InitTitleGFX→NMIOn 全链贯通；我方实测引爆已推进至 `bank00:8000`（C0BC 分发壳
已被在途成环件消化=里程碑超前兑现，与 port2 title 簇会师点即至）。
findings（r10）：0 新。基线 99。

## task-24 L3 首跑 A 引爆报告（e4548e9）——检测面四证据复现

### verdict: pass

A 跑四证据**我方独立复现**：预置 888 帧伪件→`[stale-clean] 旧 dump 已清` ✓（stale
防御运行时复验=7e94719 配套兑现）→`unported @ bank00:8000` 精确爆 ✓（go_main_routine
会师点）→dump 条目数 0 ✓（禁半成品绿）。**=我 v5 §3 检测面设计（exit 101 ∧本次无
新 dump）的运行时形态兑现**。三 tick 语义复核=契约 §8.1 tick 内序运行时自证 ✓；
B 跑备忘（首分歧三面+恒 1 断言）随行。findings（r10）：0 新。

## task-24 L3 采样迁移（546e961）——v5 §5 槽 3b/③/⑤ 勾销

### verdict: pass

三面复现 ✓：①**双跑换锚**（run1/run2 **对称命名**=P7-F1 教训兑现——295 行逐字节
一致；oracle_sha256 4655→4f9ca791 换锚+「帧末 $1A」口径作废明示=v5 3b 要求全落）
②**per-leg sample_point 键**（legs.oracle.sample_point=spin-exit；混态禁置全局键=
§5.3c 口径照办）③行集口径改口（spin-exit 腿=295 行（缺 f=2,4,5,6+尾帧=5 缺帧
算术合）；legacy 腿全集=两制并存显式）+trace 门 1 案 0 失败 ✓。采样点=$C0D3
（@exitLoop_waitNMIEnd）=**§8.1④ spin 出口同源 ✓**。
②游戏侧导出器共享面待缝=显式未完（findings §5 请求唤醒点快照缝——诚实降级）。
归档 glob 修 .run1./.run2. 排除=对照件被吞自反三案收口 ✓。
findings（r10）：0 新。**v5 §5 剩余槽**：game-core 导出器缝（显式待）/4 键余项/5/6。

## task-24 最后件：spin 出口采样缝 tick_frame_with（6a858c9）——B 解锁前置

### verdict: pass

三查：①缝位语义测 ✓（73 测含 tick1=(0,0,false)/tick2=(1,0) 双面断言=$1A 恒 1+
FrameCount 未增=**契约断言①的缝位运行时形态**）②委托形 ✓（tick_frame 委托无操作
closure，旧调用面不变=最小 API）③链规 13c ✓（文件级 pathspec 显式清单+无新件 stat
核）。采样点=oracle $C0D3 同点（NMI 尾写 1 后/帧工作前）=§8.1④ 同源 ✓；契约引证
§8.1（0172725）随文 ✓。**v5 §5 槽 3 缝面勾销**——余 decomp 侧接（迁同点+dump_manifest
sample_point=spin-exit）即 B 解锁。findings（r10）：0 新。基线 103。

## task-24 ② 收尾（3260447）——task-24 翻板 complete

### verdict: pass（快核）

三点 ✓：①缝同点语义（l3-dump 采样迁 tick_frame_with 缝=NMI 尾写 1 后/帧工作前=
oracle $C0D3 同点）②legs.game-core.sample_point=spin-exit 落键（B 跑行集对账注
随行）③**双腿采样对齐**（oracle+game-core 双 spin-exit）+trace 门 1 案 0 失败。
**task-24 五件全落（3b/3/3c/④/⑤）——翻板 complete**；B 跑唯一剩余=移植面放行
（bank00:8000 title 簇）。

## task-25 首件：samus/handler 状态机脊柱（021a692）——链规 19 域形第三次用

### verdict: pass

三查：①**10 态表整段命中 bank7:CC22** ✓（GoSamusHandler `.word` 十项声明序逐字——
Stand CC36/Run CCC2/Jump D002/Roll D0E1/PntUp D198/Door D3A8/JumpPntUp 复用 D002/
Dead D41A/Dead2 D41F/Elevator D423；首扫未中=我索引二次偏移 bug 自纠后 seg.find 中）
②结构互核注三面 ✓（10 态表=GoMainRoutine/title 31 项同构法**第三次用**=脊柱范式成形；
bmi 位 7=站立旗（Object $0300 面扩展）；JumpEngine 栈返回读=match 分派复刻）——
**链规 19 第 8 查过**（枚举+分发壳+10 态桩阵=业务结构形，非镜像）③测 2 绿（表序
全覆盖+落态/站立直入引爆）。R-mv 挂点（spec 源=samus-movement.md v0.1 随笔）注记 ✓。
findings（r10）：0 新。基线 105。

## task-25 次件：samus/physics 域首件（af6ed55）——坐标热件+VertAccelerate 头

### verdict: pass

四查：①asm 锚五处全中 ✓（Xminus16: F1F4/Xplus4: E193/VertAccelerate: E37A/
ObjectCheckMoveDown: E7AD/SamusCollisionWithSolidEntities: D976——与 task-19 考证件
三址**同源互证**）②E37A 体链字节面 ✓（AccelY 空中跳过→maxspeed 0x18→8 像素对齐
几何节流→双卫件桩）=R-mv-6 条件集 spec §2.3 对齐 ✓ ③测 2 绿 ④命名批 14 条 D5
登记（items type 11 词双源 B 级）+链规 19 域形 ✓。
findings（r10）：0 新。基线 107。

## task-23 批4b（9e29210）：crosshairs 三件——BUG 保真 D4 焦点复核（链规 21 我域）

### verdict: pass（BUG quirk 焦点面+抽检制）

**焦点面（BUG quirk/D4）**：①`cmp PPUCTRL_ZP` 字节形实证 ✓——bank0:$88A0=`c5 ff`
（CMP zp 形）对照 bank0 五处 `c9 ff`（立即数形）——「supposed to be #$FF」bug 的
字节面钉死；②夹具双钉判别力 ✓（RAM[$FF]=$90⇒载 0xFF 面（bug 行为）/$FF⇒跳过
——双 fixture 分踩 bug 两臂=判别力设计在案）③A 毒化语义（`lda #$00` 使二组完成判
当帧必败）+死码残迹写面（$030D/$030E/$0306 abandoned 仍写）=D4 保真执行面 ✓。
**抽检面（链规 20 同形批）**：423 条 0 失败（409 零回退+14 新案）✓；字节真偏移二证
（xChange@0x06/yChange@0x07 同偏移双语义=.db 别名之谜解——「asm=规格非模板」续案）
✓。findings（r10）：0 新。

## task-23 批5（e7c4547）：title ⑤菜单簇——链规 20 轻仪式首批（焦点复核）

### verdict: pass（quirk 面+契约面+仪式形）

**焦点面**：①**both-quirk 字节实证** ✓——cmp #$10@$90DB/cmp #$20@$90EB 双路实存
（ChooseStartContinue $90D7 区），同按掩后 $30 **两路皆不入**（$30≠$10 亦≠$20）
=字节语义保真 ✓；②BUTTON 位自证 ✓（hardware.asm:93-94 START=%00010000/SELECT=
%00100000——**非标准布局**，社区 $08/$04 说不适用本作=跨域考证入账）③**stops 契约面
首用**（$932B/$8C5E 停点=L2 stops 机制启用）✓。
**仪式形（链规 20 轻仪式首例）**：证据纪律保持——430/0 零回退+TDD 双抓（停点 reg
误写/生成器接线漏）+naming max-ID 纪律（N-0122）+基线仅上调 ✓。
**计数修正入案**（31=26 L2+5 L3 降级）=口径精确 ✓。findings（r10）：0 新。

## task-23 批6a（77b60c1）抽检 + 三批连报合账（4b/5/6a）——链规 20 分级仪式制

### verdict: pass（批6a 抽检+三批合账）

**批6a 抽检（链规 20）**：调色板回绕**结构实证** ✓——`e6 1c a5 1c c9 09 d0 04 a9 01
85 1c`（INC $1C→CMP #$09→BNE 越过→LDA #$01 回 1）全链字节面（我邻接搜索形误自纠
后结构读法定谳）；JumpEngine 分发面三注+434/0 零回退 ✓。
**三批合账**：批4b（BUG 保真 D4 全仪式=64257c2）｜批5（轻仪式首批焦点=118709c）｜
批6a（本账抽检）——**31 项 title 系收口（26 L2+5 L3）**，模型自纠战绩（六/二/三轮
+y-polarity 反写/marker 掩码吞槽=模型侧判别力代表样本）记档。findings（r10）：0 新。

## 会师接线桥（65c830d）：bank00:8000 穿越——B 跑放行件①（关键件焦点）

### verdict: pass

**穿越实证 ✓**：l3-dump 实跑爆点移至 `unported @ bank00:802A-phase`（title_bridge.rs
未武装臂面=TitleRoutinePtrTable 分发）——**8000 引爆点消除**（tick3 主循环相 phase 0
臂推进）；tick4 爆点=未武装臂面=B 跑分歧面前沿 ✓。测 81 绿（穿越测+outcome 应用面）。
**契约三答对表 ✓**（port2 ②契约）：①分发形状（臂直调形→其 dispatch_step 切换注）
②outcome 纯委托（write_event_tag 标签解析+objects_clear_events 复合展开=桥责边界
清晰）③未武装臂=表址返回（per-entry 爆）。armed/占位/真未武装三态表（批1-6a armed；
01/12/15/1B/1C/17 NmiYield 占位=B 跑首分歧预期面；0x18/0x1E 真未武装）=显式 ✓。
链规 13c 新件同块 add ✓。findings（r10）：0 新。基线 111。

## 桥二段（87fdd02）：dispatch_step 统一入口——B 跑放行件②终形（契约面）

### verdict: pass

**统一入口三步契约形 ✓**（解码 TitleFace→title::dispatch_step 分发→apply_title_step
唯一源应用）——两层分工归位（映射在其域/28 面旧臂直调形删除）；unported 表址爆
per-entry 形+jumped 转接面（爆=转接件随接显式）。测 82 绿。
**B 跑首分歧面预言字面命中 ✓**：实跑爆点 `unported @ bank00:80D0`=01 占位臂
（DrawIntroBackground/NmiYield 标记面）——port2「B 跑预期首分歧=bank00:80D0」
**字面命中**（B 跑 300 帧首跑就绪，首分歧前沿已知）。
findings（r10）：0 新。基线 112。**B 跑放行件①②双过——首跑 B 就绪态确认**。

## L3 首跑 B 首分歧分类报告（1ce1546）——v5 §3 分类模板首个实战件（设计面核）

### verdict: pass

**分类模板实战 ✓**：两轮迭代各出一类，lead 判据「首分歧可分类，每类一张任务卡」达成。
**b1（采样点相位对齐·已修闭环）** ✓——run B1 首分歧=帧 1 $001A=0x00（boot tick 误采）
被**我 §8.1③ 恒 1 断言运行时抓获**（断言输出原文=NMI 面退化信号引证）=契约检测面
实战验证；限 MainLoop 相后双腿恒 1 全过（oracle 295/game-core 297）。
**b2（帧编号集合对齐·派卡）** ✓——run B2 首分歧=帧集合不一致（左缺=[4,5,6,300]/
右缺=[1,3]）与 TOLERATED 登记签名**逐字一致**（812d0f8 登记面绑定核）；场景帧号 vs
tick 序号键面=decomp f 键任务卡。修复面（缝语义限主循环 spin 出口）=§8.1④ 契约一致 ✓。
测 83 绿；**测丢失案自录**（缝语义测误删恢复+「重构全量重写的测试保全检查」入自查
清单）=诚实文化续。findings（r10）：0 新。

## 会师契约半件（5352035）：dispatch_step+TitleStep/ShellCall ABI——契约面合账

### verdict: pass

三查：①**ABI 契约一致 ✓**——ShellCall::NmiYield=「§8.1 spin 跨帧建模口径：桥侧按帧界
让出」（我契约①原文入 ABI 文档，content anchor 核）；armed 臂 L2 件单源委托
（verify_shim 适配器=B 跑/L2 行为面零分叉）②三态表照落（armed/占位六件 NmiYield+
表址/真未武装 0x18、0x1E）③测 83（armed 不回 Unported/未武装回表址两面）。
**依赖维度案注**（f3855d2 同族第二维）：桥 :34 实调 dispatch_step 而半件曾悬空 HEAD
——symbol 级悬空 tree-coherence 不覆盖（mod 面才覆盖），rust-tests 编译面拦之；
案例注记：**新件依赖链须同块或先行落**（与「新件与声明同刻 add」并列）。
findings（r10）：0 新。基线 113。

## task-29（2f789cf）：b2 帧键对齐+缺口记账排除面+TOLERATED 撤销——生命周期闭环

### verdict: pass

三核：①**TOLERATED 撤销合法性 ✓**——退出条件成就（f 键对齐）；baseline tolerant
1→0 实读+历史留痕（tolerated-removed+sha 链）+「TOLERATED:（无）」——**我 812d0f8
登记的生命周期闭环**（登记→退出成就→撤销留痕）②**b2 修复正道 ✓**：行序+head_skip
（oracle=1/game-core=0）对齐+**FC($002D) 逐对自校验**（真帧身份锚——同 f≠同游戏帧
的错位假象解）；FC 集合全同+逐行递增；原「f=7 首分歧 35 地址」=键错位假象原地降级 ✓
③**缺口记账排除面（ii）边界核 ✓**：账内分歧=TOLERATED-by-accounting 全打印路线图
（$80D0×297 缺口图）；**账外/账清照红**；账清⇒strict 自然转绿——记账面非放行面 ✓。
实测：「0 失败, 1 记账容忍」+CHECK: PASS=12。findings（r10）：0 新。

## 双卫件②（9d4d8e7）：SamusCollisionWithSolidEntities/D976——全仪式关键件

### verdict: pass

四查：①asm 锚 ✓（D976 双旗清（`lda #$00/sta SamusOnElevator/sta OnFrozenEnemy`=
同值写事件先行=b-f270-a0 族复用）+冻敌环/电梯面双路）②**is_samus_on_top 共享判定件**
✓（@isSamusOnTop 双 jsr 实存=冻敌/电梯共用；一像素重合判据+下方排除）③Xminus16
走槽实证（17k+ hits 来源=敌槽遍历自证）④测 8 绿（判定三面+双旗清先行+环引爆）+
VertAccelerate 卫件接线（D976 桩→单元）。
**归属注双向记录** ✓：其载「verifier 812d0f8 共享 pathspec 带走基线 113 入树」=
F172 族第 3 例（我面：**baseline 文件在 pathspec 内须逐件归属核对**——我 checklist
第 6 项升「逐文件归属」措辞生效）。findings（r10）：0 新。基线 115。

## 跨帧游标契约（f832c21）：cursor 双行——§8.1① spin 语义延伸（契约面）

### verdict: pass

三查：①**cursor 契约形 ✓**（TitleFace.cursor←mem.title_cursor/dispatch 后
`mem.title_cursor = step.next_cursor`（>0=spin 续点/0=完成复位）——跨帧件段索引=
我 §8.1①「spin=边界构造物」的子步延伸，语义一致）②引擎双测改形=批6b 演进**非回归**
（port2 对账注）——账面自洽 ✓ ③测 85 绿。
**真集成发现记档** ✓：01 臂 SFX 写（$0684←$10，D4 保真面）触发 NMI 声引擎分派
`unported!("bank07:B31B")`=**新缺口域件**（声引擎 B31B 族——lead 缺口地图加行）。
**红窗最小修注**（「谁的错谁快修优先于面权」）=并发礼仪好范例记档。
findings（r10）：0 新（B31B=缺口地图行项，非本件瑕疵）。

## 批6b 二件（852ca9e）抽检：15/1B 同码深路径臂——链规 20 轻账

### verdict: pass（抽检）

同码折叠面抽检 ✓：$90BA 同址体字节面（`20 9f 90 a2 84…`=StartContinueScreen15/1B
共享入口——批 23 表读既证同址）；ClearAll 共享段折叠+稳定锚随形；434/0 零回退 ✓。
findings（r10）：0 新。

## 批6b 三件（9009786）抽检：1C GameOver 深路径臂——链规 20 轻账

### verdict: pass（抽检）

ClearAll 共享段复用（链规 19 折叠续）+434/0 零回退+测 86 绿。findings（r10）：0 新。

## 积分器收编（aad1988）：int_x/int_y 拼装进 samus/physics——R-mv-2/3 规则面

### verdict: pass

三查：①钳位语义双面 ✓（负向幅值≥6⇒{int:-6,frac:0}/正向 int≥5⇒{5,0}——integrators.rs
考证件字节级推导注在位；E3A5 族证明级件=前账 int-y 证明面复用）②R-mv-2/3 规则面承载
（垂直速度积分/水平+max_src=HorzSpeedMax 钳位）=spec 面 ✓ ③测 86 绿（RAM 往返+
位移语义期望值自算修正案）。
**更正注记档** ✓：data12/13=速小（积分器考证定谳）——社区注「Object attribute data」
C 级旧猜测原地更正（证伪文化续）。findings（r10）：0 新。基线 116。

## task-28（d84cef6）：samus-move3 双腿场景包+fixture DSL——v5 §5 槽 4/5 面（契约面）

### verdict: pass

四查：①**fixture DSL=我 state-fixture §3 F_a 注入模型落码 ✓**（`fixture <帧> <addr>
<值> [invariant|evolving]`；frame/addr/value/**persistence** 键=r7 不变式语义；
scenario_run 解析+capture 采样前注入+run.json 记录——设计 v9/v11 引证随码）②oracle
spin-exit 采样+**run1/run2 对称双跑逐字节一致**（06bf0d35 换锚；1383 行/17 缺帧
行集全录 legs.*（head_skip=1））③game-core 引爆面 `bank07:B31B`=声引擎缺口域件
（与跨帧游标契约账的 B31B 记档合流=task-11 排程坐标，fail-loud 不落盘）④恒 1 断言/
键集白名单随案激活（双腿 sample_point=spin-exit）+trace 门 2 案 0 失败 1 记账容忍 ✓。
findings（r10）：0 新。**v5 §5 槽 4/5 主体勾销**（fixture DSL+双腿场景包+双跑换锚）。

## task-31（76c8aa7）：L4 视觉对拍奠基——visual-diff SKIP→ENABLED 裁定把关（关键件全仪式）

### verdict: pass——**ENABLED 核准**（verifier 门面裁定）

五面证伪+加验两靶：①selftest 四对照独立复跑 ✓（同帧全同/1 像素打回/白名单放行/
尺寸打回——`selftest: PASS` 原文）②**白名单空表零容差** ✓（`whitelist: []` grep 证
0 条目；条目格式 {file,rect,reason,approved_by}+「变更需用户确认并记录」注随表=
AGENTS §5 白名单纪律执行面就位）③金面 7 帧 0 失败（门禁面可复跑）✓ ④**PNG 解码
显式拒收** ✓（depth≠8/ctype∉{2,6}/interlace≠0 三面 PngError——防静默错比）⑤
**判别力实证** ✓：1 像素打回首分歧定位 `@2,1 金=102030ff 采=102130ff`（坐标+双色
值）+selftest 破即拒跑机制。**基线影响核** ✓：SKIP→ENABLED 声明随 commit；渲染腿
未启用（S6）显式注。
**裁定**：visual-diff 门禁 ENABLED 生效（L1-L4 四层收官=验证体系完整性关键件）；
白名单空表起步=像素级零容差，后续任何条目须用户确认+记录（我门面把关续）。
findings（r10）：0 新。

## 10 态肉批1（d01bf12）抽检：SamusStand 转移核——链规 20 轻账

### verdict: pass（抽检）

434/0 零回退+samus 11 测绿（转移核随形）。findings（r10）：0 新。

## task-33 ②：首批双场景双门突变矩阵（v5 §5.5 实施兑现）——判别力承重首轮

### verdict: pass（矩阵实录+缺口发现）

**测量面声明**（首要）：B31B 声引擎缺口遮蔽期，boot-title/samus-move3 两腿均引爆
（无 dump）——**帧值面判别力受限**；当前可测面=引爆签名+分歧记账图+L2 族谱面。
帧值类突变待 task-30 声引擎清偿后二轮重测（显式记录，非静默）。

**矩阵实录**（突变全注入验证（mutation_verified=注入行实证）；备份-改-建-测-还原-再生成净 dump）：

| 突变 | 场景 | 族（门 1） | 判别值域（门 2） | 预期 | 实测 | 裁定 |
|---|---|---|---|---|---|---|
| M-B1 引爆址翻 B31B→B31C | boot-title | 检测面族（新登记） | B31B 引爆面在场 | 签名变 | `unported @ bank07:B31C` | **caught** |
| M1v2 相位推进 +1 | boot-title | 末值 A 链/相位族（高危） | init 相位轨迹在场 | 首分歧@写面 | 帧 1 $0005 分歧（左=00 右=40）+账外照红 | **caught** |
| M-S1 引爆址翻 B31D | samus-move3 | 检测面族 | 同上 | 签名变 | `unported @ bank07:B31D` | **caught** |
| M-S2 钳位阈 5→6 | samus-move3 | 钳位边界族（高危·CMP 边界对） | **speed_i==5 边界不在场** | — | L2 434/0 | **survived（判别值域缺席）** |
| M2 写集漏（丢首事件） | boot-title | 写集漏族（高危） | 写集面被 B31B 遮蔽 | — | 0 失败 | **survived（遮蔽）** |

**双门裁定**：门 1 族谱覆盖 ✓（高危族 3/5：相位链/钳位边界/写集漏——L2 族谱门引
RN1_STEP 8/8、carry 9/9 前账战绩为族级判别力证据）；门 2 判别值域审计 ✓——
**M-S2 暴露真缺口：int-y 正钳位 speed_i==5 边界值不在案族**（§4「打回 0≠判别力」
教科书案例；补案提示=b-int-y-boundary-5）。
**测量有效性自纠**（P1-F3 四案）：M1 首测漏重建（突变未执行）/M2 首测 sed 空转/M-S2
首测锚缺失——**mutation_verified 字段三拦**，突变纪律自证。

### 遗留（矩阵侧）
- speed_i==5 边界案补全（钳位族判别值域）——回 L2 立案线；
- 帧值类突变二轮（B31B 清偿后重测 M1v2/M2 同形）；
- 首批突变族谱登记（检测面族=帧级新族）。

## MSS↔FrameMemory 映射（a72d37a）：task-32 分工件——spec §4 收官缝

### verdict: pass

三查：①三直映 ✓（RAM/WRAM/regs 直映+ps↔p 同义名+nmi_enabled $FF bit7 推导——
spec §4 接缝表引证随码）②扩展面注 ✓（mapper/ppu/clock 分工表显式：A9 后遗同面/
S6 渲染腿不含/时钟 RNG 待 task-10）③测 92 绿（往返保真断言）；验收形=装载后
watch_snapshot==oracle F_a+1 采样（声明域逐字节）=L3 集成验收形 ✓。
**括号失衡案自录**（mss_map_tests 误入 impl 体=rindex 锚症结）=工具教训记档。
findings（r10）：0 新。基线 122。

## b-int-y-boundary-5（8ed16e6）+ M-S2 复核闭环——判别值域补全全程记

### verdict: pass（闭环完成）

**全程闭环实录**（矩阵首轮→缺口→补案→复核打回）：①矩阵首轮 M-S2（钳位阈 5→6）
survived=**判别值域缺席**（speed_i==5 边界不在案）②porter 补案 4 件（pool
[5,6,0xFA,0xF9] 正负周界；golden=ROM 执行产出非手推+factory 边界相位）③**复核
实测：基线 438/0→M-S2 重注入 438/1 失败**——bound-0（speed_i==5/frac=0xFA/accel=5）
打回 ✓（其预判「0312:0→突变下 0xFF≠0 应被打回」字面兑现）。
**教训形态固化** ✓：「边界值必须显式入案族（随机采样撞不到边界）」+casegen 边界相位
=机械面固化——§4 门 2（判别值域门）从设计到实效的完整样本。findings（r10）：0 新。

## 10 态肉批3（aa2d791）抽检：SamusJump/SamusRoll——链规 20 轻账

### verdict: pass（抽检）

438/0 零回退+samus 14 测绿。findings（r10）：0 新。

## 首例任意起点对拍（74ed3c5）：bundle 全链实战——task-32 收官件（验收形实战）

### verdict: pass

四查：①**全链实战 ✓**：采集 f=900（mss 11915B）→oracle 恢复→快进 240f→**102 帧
逐字节忠实**（ff 行 i↔f900+i 校正对齐）；首分歧 f=107 zp$08=恢复相位残差（~100 帧
漂移=savestate 重放经典类，**显式登记**非静默）②**三条件断言消费实证 ✓**（diff_traces
「三条件断言过（2 fixture：(b) 读回+(c) 持久）」live——我 state-fixture §3 条件
在裁判面落地，fixture-test 案在册）③双腿联合验收 ✓（l3-dump --bundle 通道=
restore_mss 三直映功能面证成；恢复态连贯推进至 C23x-change fail-loud=映射件实证；
双腿逐帧对拍待 task-30 机制无阻）④工程修正三件（触发帧语义/相对路径崩回调/MEMTYPE）
=稳健面 ✓。findings（r10）：0 新。

## task-30 首件（dbfaa23）：sound/ 域骨架+0x1E SetTimer——B31B 清偿路径

### verdict: pass

三查：①**SetTimer 字节全宽实存 bank7:$C4AA** ✓（`85 2c 86 20 a9 09 d0 f5` 逐字——
我首扫 bank0 未中=bank 映射陷阱族自纠后 bank7 命中）②asm 锚 ✓（SoundEngine: $B3B4/
ClearInitFlags: $B3D6/GetSoundRoutine: $B4BD 实存+结构互核注五条（静音旗位0/MSB 优先/
表步进/Multi 特例/6 旗清））③测 96 绿（126 全绿基线）。
**title 账面全清** ✓（0x18+0x1E 均臂化=31 项账面终形）；sound/ 域骨架（SfxCycle 八
cycle+scan_sound_flags MSB 优先位扫描）=B31B 分派层地基。findings（r10）：0 新。

## task-33 ③：验收义务收尾（r5 遗留 4/5）——并发采集复检实录

### verdict: pass（义务勾销）

**r5 遗留 5·并发采集复检 ✓**：WIP 在飞时（l3-dump.rs 变更在盘=真并发条件）新采
boot-title 两跑——sha `4f9ca791…` **与锚逐字节复现**（跨时刻+并发树条件确定性成立；
fresh re-collect=旧 run2 归档同 hash=双跑确定性三重验证（原双跑/跨时刻/并发））。
**r5 遗留 4·双侧重跑 ✓**：oracle 腿新采复现+game-core 腿恢复态推进（bundle 件
实证）——双腿机制均在复跑态。demo 双跑三验锚（4f9ca791）不变=零回退。
**task-33 三件全勾销**（①manifest 余键核毕（三对余键各挂上游）②突变矩阵首轮
（37e94f8）③本账）——task-13 契约「设计→验收」程序路**收官**。

## samus-move manifest 条目（84a8a8a）：9 案全景落定——task-33 ① 面续（键完备）

### verdict: pass

三查：①manifest 键完备 ✓（9 案全 legs.*.rows/missing/head_skip 登记——boot-title
双腿 295/297、samus-move 885、sc 四包 1083/1083/1331/1381、fixture/bundle 件；
samus-move 条目含 game-core 腿注（10 态会师腿=预期前沿 B31B 声引擎分派））②
spin-exit 采样声明+watch 面=观测面不改行为 ✓（双跑重锚 034c3693，885 行缺帧如实）
③trace 门 9 案 0 失败 0 记账容忍 ✓。findings（r10）：0 新。

## task-30 分派核（9dc12a7）：GetSoundRoutine 6 位扫描——B31B 清偿主体

### verdict: pass

三查：①**六轮止字节实证 ✓**（@loop_B $B4DE iny/iny/cmp #$10 结构面——scan_sound_flags
正名 6 位（位7..2/位0 静音/位1 保留），先版 8 位误**原地正名**=纠正文化续）②asm 锚
$B4BD-$B4F8 逐字（4B 装载循环/asl 六轮/双字解析/RestoreSFXFlags）+结构互核注三条
（6 轮非 8/双字头表步进/默认例程=无旗自续=八 cycle 驱动面）③测 100 绿（域测三件）。
multi_music 特例=粗形+$BC53 深证随笔（显式待证不冒领）✓。
**B31B 清偿主体过**——矩阵二轮（帧值类突变解遮蔽）待声引擎全落定后开（照 lead 排程）。
findings（r10）：0 新。

## B31B 行清偿钩件（ca961d6）：声引擎会师接点——遮蔽解除（关键路径件）

### verdict: pass

三查：①**B31B 字节族恰址实证 ✓**（`ad 80 06 a2 89 d0 15` @ $B31B 逐字）+六 bank
同字节 sig 自证（烘焙区 SOUND_REGION 0xA80B）②链走分派四面 ✓（无旗默认装载器
自续→RTS 终端 $B4EE 共值/ContinueMusic 微折/Multi 双步/handler=记账点）——
nmi.rs 四 init 桩接线=B31B 行清 ③测 100 绿（6 位扫真旗形）。
**B31B 行清偿实证**：samus-move B 跑三轮——B31B 爆点消除→**新分歧=B673 handler
细行**（port2 预言「B31B 行→handler 细行」字面兑现）=缺口演化面（每引爆一个端一个）
✓。**矩阵二轮解遮蔽条件成熟**（帧值类突变可测面扩至 B673 前）。findings（r10）：0 新。

## task-30 handler 细行首件（7af2492）抽检：B673 SamusHitSFXInit

### verdict: pass（抽检）

三方全等（439/0 新案）+测 101。handler 细行族开（缺口演化面逐端清）。findings
（r10）：0 新。

## task-38（579486d+c753345）：测试执行面超时护栏硬化——spin 挂死防御四面

### verdict: pass（工具面）

四面核：①L2 步数上限（MAX_STEPS=1M+fail-loud——JMP-to-self 40ms 限时打回实证）
②引擎 watchdog 测（spin_program_bounded_fail_not_hang=**坏例先喂范式**+30s 耗时锚）
③createSavestate 无参**机械防**（lint_mesen_lua.py 禁用清单 2 条+写盘前内联双保险
——坏例 exit 1 拒跑实证）④超时=FAIL 显式（diff_subroutines 600s 限时打回非挂死）。
lint 56 目标 PASS+watchdog 0.05s 限时 ✓。基线 131→132。findings（r10）：0 新。

## m2 预拼装批1（9372b7f）抽检：D147/CD92 全折+CD6D 壳

### verdict: pass（抽检）

移动深路径嵌套件（439/0 零回退+测 101）。findings（r10）：0 新。

## task-30 handler 对件（8bb4371）抽检：B650 SamusHitSFXCont

### verdict: pass（抽检）

Init/Cont 族收口（441/0 双案+测 103）。findings（r10）：0 新。

## task-44 W1 首波（aab48d8）：音乐索引核+BeepSFXInit——B 跑触发面

### verdict: pass

三查：①字节面（六 bank 镜面族正确读法）✓：$B7D9=`ad 8b 06 29`（LDA $068B=
SFXTriContFlags 守卫面实证）+$BC53=`a9 ff 8d 5e`（无旗⇒$FF）——**bank 映射陷阱
自纠**（可切窗 $8000-$BFFF≠bank7；首探 bank6 数据区误读→全 bank 镜面扫描定谳）
②max→覆写语义更正（multi_music_index_step 粗形替换）=纠正文化 ✓ ③结构互核注
（块名≠索引释义：0=SQ1/1=SQ2/2=Tri/3=Noise/4=Multi 双证；公式 base+ct 不变仅名称
注记）=口径精确 ✓。sound 测 9 绿。findings（r10）：0 新。

## T45-4（c69deb2）抽检：S6 金面库扩面 7→50 帧——task-31 纪律延续

### verdict: pass（抽检）

50 金面帧 0 失败+**白名单 0 条目保持**（像素级零容差不因扩面松动）+能力门径图
（T45-4）随形。CHECK PASS。findings（r10）：0 新。

## m2 深链批2（079ccc6）抽检：UpdateObjAnim 动画推进器

### verdict: pass（抽检）

表烘焙+FF 回绕界（441/0 零回退+测 106）。链规 13d（复采/采样窗口板上登记=lead
协同面规则）随行非我复审面。findings（r10）：0 新。

## 双件合审（ed7f55b W1 曲径臂 + fd949c1 Statue 考证件）

### verdict: pass（双件）

**W1 曲径臂**：三址六 bank 镜面一致实证（$B9E4=`a9 7f 8d` 禁扫/$BC8B=`aa a8` duty
对称 tax-tay）+SONG_INTRO_HEADER 13B 烘焙（宏展开形）+残面显式（W4 波次注）+
sound 测 11 绿。**Statue 考证件**：$687B/$687C 双 boss 状态位→PasswordByte0F 状态
模型（porter 对表源）+ItemData 世界路线图 50+ 项全坐标（L9029-L908D A 级内容锚）
+开放项显式（字打包位段/Statue 抬升例/红门轰开=不冒领）。
findings（r10）：0 新。

## m2 深链批3（8ded8e7）抽检：IsScrewAttackActive+gear 能力位组

### verdict: pass（抽检）

441/0 零回退+测 109。findings（r10）：0 新。

## M-S2 突变矩阵二轮（点名复审令）——解遮蔽面扩大后重测

### verdict: pass（二轮实录）

**测量面状态**：W2 六件+映射反转更正后双腿引爆前沿推进至 **C1FF**（音乐链新缺口）
——帧值类仍未全开（爆前无 dump）；**相位逃逸类突变可产出全 dump**=帧值面对该类
解遮蔽。

**二轮矩阵实录**（注入验证/备份-改-建-测-还原-净态复位全程）：

| 突变 | 场景 | 判别面 | 实测 | 裁定 |
|---|---|---|---|---|
| M1v2-R2 相位推进 +1 | boot-title | **帧值面（解遮蔽）** | 300 帧 dump→帧 1 $0005 首分歧（左=00 右=40）+账外照红 | **caught**（与首轮逐字复现=判别力稳定性实证） |
| M2-R2 写集漏（丢首事件） | boot-title | C1FF 遮蔽 | 与基线同爆 C1FF（签名无差） | **survived（遮蔽类续记）** |

M-S2 钳位阈复核（首轮闭环 438/0→438/1 caught）引为族级判别力基证。
**二轮结论**：①相位逃逸类帧值判别力两轮稳定 ②C1FF=下一遮蔽前沿（缺口演化续）
③写集漏类待 C1FF 清偿后三轮终测。findings（r10）：0 新（测量局限显式续记）。

## 三件合账抽检（30d3428 m2 批5 + 2bfd7f5 W3 中频族 + 5243bfd 施工图终版）

### verdict: pass（抽检·三件）

①m2 批5 LavaAndMoveCheck/E269+②W3 中频 handler 族六件（映射反转案续）——suite
455/0 零回退+测 114 ✓；③**施工图终版=task-43 消费对表核 ✓**：域归属校准与我
387 簇定谳**逐字吻合**（共享引擎 181/Tourian 45；早期推算 84=大误 +97 如实更正）
+簇边界校准+域优先序——387+N（bank0/6 待补）口径一致。findings（r10）：0 新。

## 三件合账抽检（96cc759+e60ecfa W1 残面双件 + 27e694d S6-3 调色板链）

### verdict: pass（抽检·三件）

①W1 残面首件（InitializeMusic 深径+帧径入口）+②残二件（UpdateMusicChannel 音乐
指令 VM 步进）——$BAC2 锚实证 ✓（`8a 0a aa bd`=VM 索引双倍查表形，六 bank 镜面
一致）+指令三分派/重复机字节面引证+VM 测五面 ③S6-3 调色板链（共享解码器 ROM 源）。
suite 455/0 零回退+测 121 ✓。findings（r10）：0 新。

## 三件合账抽检（c047b50 Statue 门控族 + 1b41865 开门链定谳 + 9b5865d W1 残三件）

### verdict: pass（抽检·三件）

①T45-1 Statue 门控族（全仪式关键件）——锚形合规（asm+constants_ram 双源
（TileBlasts $0500/Statue 槽面）+door-open-chain §4 对表引证）②开门链 A 级定谳
——**$8B79=`ad 86 68` 双身实证 ✓**（NumUniqueItems 字节级核）+残余 4 收口
③W1 残三件（音符载 _Continued 解+四通道环）。suite 455/0+测 130 ✓。
findings（r10）：0 新。

## 四件合账抽检（840ad00 S6-6/7 RenderState + 79490b3 zp 全清 + ad3904b head_skip 终值 + 45641b5 栈面排除）

### verdict: pass（抽检·四件）

①S6-6/7 渲染态面（CHR/名表/PPUADDR 建模=L4 视觉腿底座）②zp 全清 23→0（声指针
机理定案 $E0=TablePtr——**$B4CA=`99 e0 00 c8` 拷形实证**（STA $00E0,Y/INY））✓
③**head_skip 终值=0**（FC 配对 295/295 全等=skip0×skip0 实测；旧 head_skip=1=
f 键时代语义残——FC 语义终值=我行集契约面收口 ✓）④栈面排除（45641b5 我自件=
源级 0 读实证注记）。suite 455/0 ✓；L3 现红 1 失败 1 记账容忍=porter B 迭代态
行集声明滞后（记账面在案，非此四件归因）。
findings（r10）：0 新。

## 四件合账抽检（2660fae W4 尾区七件 + b4d5d80 title_tick_head + ac6b488 解锁集 + f9d1c9d 移动臂驱动批）

### verdict: pass（抽检·四件·启动链定案核）

**启动链定案核 ✓**：①0x1B 共体双入口——$90BA 共享体字节复核 ✓（`20 9f 90…`=
批 23 既证入口；⚠️非 0x15 自纠注随证）②GameMode←0=InitBank2 $C583 写点族实证 ✓
（`a9 00 85 1d`=LDA #$00/STA $1D 写点）——GameMode=0→游戏路径（AreaInit 爆点=
迭代正进注）。W4 尾区七件+title_tick_head 头件（START→菜单转移跨域点）+解锁集
（build 破点修复+三面平铺守卫测）随形。suite 461/0 零回退（W4 七案）+测 140 ✓。
findings（r10）：0 新。

## SC-S1-00 卡审（02ef596+01f3bd6）——checklist 首战

### verdict: needs_revision（单点：L2 留白条款误用）

**核心发现（契约族）**：「L2 面=0 案留白（跳板桩簇条款）」=**基于缺陷计数表的误判**——
537979e 计数正形（'#'-in-id 过滤器全零缺陷修复）后 **s1-00-case=8 案实值**（其
Evidence 自证「修后 a1-000-case(8 案)/s1-00-case(8 案) 实值」）；我实核：pregen
cases.jsonl `s1-00-case`×**8 条**在库，l2_cases.jsonl 并入 **0 条**。留白条款
（跳板桩簇）=11 败簇专指，s1-00 属 **19 有案簇**——条款误用。
**修法**：8 案并入 l2_cases.jsonl→l2-diff 三方跑→已验证级位（勾格①④一并收）。

**其余三面 ✓**：①字节锚（$D498 尾共享形 `20 85 eb a8 a2`+LoadStatues
$687B/$687C→$60/$50 族逐字实证）②L3 14 案 0 失败（容忍 1→2=覆盖增长显式归因
（$69/$200 缺口记账面+栈窗排除打印）非本卡归因）③L4 无涉（写面全 RAM 态）。
④勾格：矩阵 57/57 无级位字段=格证缺同刻（小修随 L2 补案一并落）。

## SC-S1-00 返工复审（746f938）——首卡闭环

### verdict: pass

修法三步全执行 ✓：①8 案并入 l2_cases.jsonl（实核 `s1-00-case`×8 条）②三方裁定
**469 案 0 失败**（461→469=+8 增额精确=案数增额核过）③已验证级位=绿色三方即格证
（适配器 s1_cluster_00 出口面（A=末配/Y=root/X=0）由 8 案钉死）。
**checklist 措辞修正注**：条 4「矩阵格证同刻」假设有矩阵级位字段——实况=勾格阶梯
以**工件态**承载（l2_cases 成员=有L2；三方绿=已验证）；格证=卡 commit 内案族+裁定
输出引证（本卡达标）。自查清单新条（pregen 实值判留白）收讫=教训闭环 ✓。
L3 14 案 0 失败（容忍不增 ✓）。**SC-S1-00 卡闭环（批产首卡）**。

## SC-S1-01 卡审（6f4c5ae）——checklist 例行

### verdict: pass（附声明精度注）

四条核：①L2 面——s1-01-case×8 并入实核 ✓，**s1-01 族 0 失败**；增量结构
469+8(s1-01)+8(a1-001)=485 精确 ✓；**声明精度注（声明族·轻）**：「485/0」=族域混述
（当前套件 485 案含 a1-001 族 8 败=其路线图在案件（EnemyMove 移效写面转写余，
退出条件在案）非本卡——**套件级数须族域限定**（「s1-01 族 8 案 0 败/套件 485 含
a1-001 在案 8 败」=正述形）②L3 16 案 0 失败（容忍 2 不增 ✓）③L4 无涉（handler
全 RAM 态）④格证=工件态（l2_cases 8 条+族内全等）。
通用面：流式块边界抽核 ✓（$D83A=`4c 06 d8` jmp 尾入+$030F,x 符号分路形）+测 153。

## SC-S1-02 卡审（a73b21b）——checklist 例行（留白真 0 样板）

### verdict: pass

四条核：①L2 留白**真 0 双核 ✓**（pregen `s1-02-case`=0/l2_cases=0——**S1-00 教训条
（pregen 实值判留白）执行面首验过**）②L3 17 案 0 失败（容忍 2 不增 ✓）③L4 无涉
④格证=已实现级位（0 案卡=留白注+工件态（9/9 员转写））。
通用面：**旗门正反向对形实证 ✓**（$F56C=`20 b0 80 29 20 d0` BNE 正向 vs $F503=
`20 b0 80 29 20 f0` BEQ 反向——同掩码 AND #$20 镜像对）+栈跳深段注记随形+测 155。
**族域述 ✓**（S1-01 声明精度注生效面：「engine 族 10 测 0 败+CHECK 绿（族域述）」）。
套件面注：485 案 7 败=a1-001 族在案（8→7=其 EnemyMove 转写推进中，路线图退出条件
趋近）非本卡。findings（r10）：0 新。

## SC-S1-03 卡审（659b3f7）——员面真现核（勾格要件重点）

### verdict: pass

四条核：①**员面真现核 ✓**（勾格要件重点）——engine/mod.rs **桩面 0 处**
（unported!/todo! 双扫）+45 fn 真现（8 员形随证）②L2 面 s1-03-case×8 并入实核 ✓
（族内 0 败；套件 493 案 7 败=a1-001 族在案（8→7 稳）非本卡——族域述沿用）③L3
0 失败容忍不增 ④格证=三级位齐（已实现=真现 8/8/有L2=8 案/已验证=族内全等）。
通用面：x 索引址测试误三连**教训注自查条**（x≠0 须加偏移）收讫=TDD 纪律续；
测 156。findings（r10）：0 新。

## SC-S1-04 卡审（628bb3d）——进位链/移位面抽核

### verdict: pass

四条核：①L2 留白**真 0 双核 ✓**（pregen/l2_cases 双 0）②**进位链/移位面抽核 ✓**
（$F844 GetEnemyTypeTimes2PlusFacingDirection 字节形 `bd 05 04 10 02 4a 4a 4a`
=LSR×3 移位链实存；「lda 替 A+rol 过末 lsr 出位」语义正案（golden≠core 自纠）+
**rol/rsr 进位链自查条**追加=TDD 纪律续）③L3 17 案 0 失败容忍不增 ④格证=已实现
级位（真现 7/7——**桩面 0 处双扫 ✓**）。测 157（族域述 13 测）。套件 7 败=a1-001
族域在案非本卡。findings（r10）：0 新。

## SC-S1-07 卡审（cf5d4f4）——写组/Exit22 尾跳抽核

### verdict: pass

四条核：①L2 面 s1-07-case×8 并入+**族 8 案 0 败（首提全过）**✓（增量 493→501=+8
精确；套件 7 败=a1-001 族域稳）②**写组/尾跳抽核 ✓**：$F332 Exit22=`20 40 f3 4c
c6 c2`（JSR+**JMP $C2C6 Adiv 尾跳**实存）+$F300 指针载 `ad ce`=**LDA $95CE 源对**
（我陷阱族地址——移植侧正确处理记档）；案实核定写组（$006E←3/$006F←56/…/
$030A 回写）随证 ③L3 0 失败容忍不增 ④员面双扫桩 0（6 员真现）。
教训注：缺 return 编译破自犯（表达式语义自查条）收讫。测 158（族域述 14 测）。
findings（r10）：0 新。

## SC-S1-08 卡审（268b00a）——KillObject 击杀写抽核

### verdict: pass

四条核：①L2 面 s1-08-case×8 并入+**族 8 案 0 败（首提全过）**✓（增量 501→509=+8
精确；败案归属核=全 8 条 a1-001-case-0..7 族域在案非本卡）②**KillObject 击杀写
抽核 ✓✓**：$FA18=`a9 00 9d f4 6a 60`=LDA #$00/STA $6AF4,X/RTS——「敌槽清击杀写」
逐字命中；案实核 9 写恒定形（定写组 7+$0681←$40 SFX 旗+$6AF4[x]←0）随证 ③L3
0 失败容忍不增 ④员面双扫桩 0（6 员真现）。测 159（族域述 15 测）。
findings（r10）：0 新。

## SC-S1-09 卡审（79e1330）——SetSamusAnim 3 写微体抽核

### verdict: pass

四条核：①L2 面 s1-09-case×8 并入+**族 8 案 0 败（首提全过）**✓（增量 509→517=+8
精确；败案 8=a1-001 族域稳非本卡）②**SetSamusAnim 3 写微体抽核 ✓✓**：$CF6B=
`8d 05 03 8d 06 03 a9 00 8d 04`=STA $0305/STA $0306/LDA #$00/STA $0304——
「$0305←A/$0306←A/$0304←0 A 归零出口」逐字命中 ③L3 0 失败容忍不增 ④员面双扫
桩 0（5 员真现）。测 160（族域述 16 测）。findings（r10）：0 新。

## SC-S1-10 卡审（6b25512）——adc 进位链/索引址核

### verdict: pass

四条核：①L2 留白**真 0 双核 ✓**（pregen/l2_cases 双 0）②**adc 进位链/索引址核 ✓**：
$FD8F root=`a5 49 29 02 85 02 a5 04 18 30`（LDA/AND/STA 写组+**CLC=进位链预置**+
bmi 前置判别=三径算形）；「进位判别跳钳」语义正案（溢出无钳/$F0 钳+$0F 回卷）随证
③L3 0 失败容忍不增（517/8=a1-001 族域稳）④员面双扫桩 0（5 员真现）。
教训注：**x 索引址四犯→强制步升级**（自查条升级为必检步）+测入参符号径选择=TDD
纪律续。测 161（族域述 17 测）。findings（r10）：0 新。

## SC-S1-11 卡审（072e638）——依存/条件写拟合+金派支注记面

### verdict: pass

四条核：①L2 面 s1-11-case×8 并入+**族 8 案 0 败**✓（增量 517→525=+8 精确；
败案 8=a1-001 族域稳）②**依存/条件写拟合核 ✓**：$D400 流=`20 49 e2 ce 0e 03`
（JSR ToggleSamusHi+**DEC $030E 依存写**实存）+$E249=`ad 0c 03 49 01 8d 0c 03`
=EOR #$01 toggle $030C 逐字；23 写拟合三分（定写 17/依存 5/条件 1）结构化随证
③**金派支注记面 ✓**：条件写（$0683←8）金派支（v30d==6 不写=案 4 实核）+
真条件=流深径声触发（**随件 trace 补=显式待证**不冒领）④员面双扫桩 0（5 员真现）。
测 162（族域述 18 测）。findings（r10）：0 新。

## SC-S1-12 卡审（31c5af9）——TwosCompliment 取负微体抽核

### verdict: pass

四条核：①L2 面 s1-12-case×8 并入+**族 8 案 0 败（五连首提全过）**✓（增量 525→533
=+8 精确；败案 8=a1-001 族域稳）②**取负微体抽核 ✓✓**：$C3D4=`49 ff 18 69 01 60`
=EOR #$FF/CLC/ADC #$01/RTS——「A=-A 无写面」两补形逐字命中 ③L3 0 失败容忍不增
④员面双扫桩 0（5 员真现）。测 163（族域述 19 测）。findings（r10）：0 新。

## SC-S1-13 卡审（2fcb5e3）——GetObjCoords 坐标装载抽核

### verdict: pass

四条核：①L2 留白**真 0 双核 ✓**（pregen/l2_cases 双 0）②**坐标装载抽核 ✓✓**：
$D79F=`a6 4b bd 0d 03 85 02 bd 0e 03`=LDX $4B/LDA $030D,X/STA $02/LDA $030E…
——「$030D/$030E/$030C[x]→$02/$03/$0B」装载链逐字命中 ③L3 0 失败容忍不增
（533/8=a1-001 族域稳）④员面双扫桩 0（5 员真现）。测 164（族域述 20 测）。
findings（r10）：0 新。

## SC-S1-14 卡审（976ff3e）——跳序/分支链抽核

### verdict: pass

四条核：①L2 面 s1-14-case×8 并入+**族 8 案 0 败**✓（增量 533→541=+8 精确；
败案 8=a1-001 族域稳）②**跳序/分支链抽核 ✓✓**：$D6BF=`20 8b d7 8a d0 06 a5 04
29 20`=JSR/ TXA / BNE +6 / LDA $04 / AND #$20——**X 判（TXA/BNE）先行于位 5 测
（AND #$20）**=「先版位 5 判误→X 判跳序正」二正案字节面逐字命中 ③L3 0 失败容忍
不增 ④员面双扫桩 0（5 员真现）。测 165（族域述 21 测）。findings（r10）：0 新。

## SC-S1-15 卡审（1474e23）——误带二正案+逐簇案实核强制面

### verdict: pass

四条核：①L2 面 s1-15-case×8 并入+**族 8 案 0 败**✓（增量 541→549=+8 精确；
败案 8=a1-001 族域稳）②**误带二正案抽核 ✓✓**：$D77F=`18 65 0a 85 04 a5 0b 69 00`
=CLC/ADC $0A/STA $04/LDA $0B/**ADC #$00 带进加**——「加法对偶 $04←(A+$0A)/$05←
(($0B+进)&7)|$60」逐字命中（无 $00 面/无 $80 支=S1-14 减法形误带实证二正）③L3
0 失败容忍不增 ④员面双扫桩 0（5 员真现）。
**教训注收讫**：姊妹簇形不可外推（S1-14/15 共形近亲双异）→**逐簇案实核强制**=
checklist 条 1 升级面 ✓（+tuple 索引测误自查）。测 166（族域述 22 测）。
findings（r10）：0 新。

## SC-S1-16 卡审（593a6b6）——检索环比较语义/索引址抽核

### verdict: pass

四条核：①L2 留白**真 0 双核 ✓**（pregen/l2_cases 双 0）②**检索环抽核 ✓✓**：
$EE4A=`ac 86 68 f0 12 a5 07 d9 86 68`=LDY $6886/BEQ/LDA $07/**CMP $6886,Y**——
对数组比较（Y 索引 NumUniqueItems 基）逐字命中（carry=找到语义=CMP 结果面）③L3
0 失败容忍不增（549/8=a1-001 族域稳）④员面双扫桩 0（5 员真现）。
教训注三条收讫（环索引 y-1→y 同基/x 索引五犯/值址双误——自查条升强制）。
测 167（族域述 23 测）。findings（r10）：0 新。

## SC-S1-17 卡审（e4e05af）——跨卡账目语义/依存支拟合核

### verdict: pass（跨卡账目新形·样板）

四条核：①L2 面 8 案（6 strict 0 败+2 账内）✓——**跨卡账目新形样板**：l2_gap_map
+2 条（golden 含他卡效应=正常；退出=源卡深径建模落→账清转 strict）；账目面实测
激活（「L2 账内容忍 10 案（迭代态缺口——账清转 strict）」=s1-17×2+a1-001×8 合账
=task-29 机制 L2 延伸）②**溯源定案 ✓**：$D012 流=`20 47 d1 20 55 d0`（JSR $D055
跨卡调用实存）+$D052∈S1-05 SetSamusData 族溯源链 ③依存支拟合（$0310 26|入参
加/或双拟合歧注）随证 ④员面双扫桩 0（4 员真现）。StopVert 形（`8c 08 03`=STY
$0308 清速）随核。测 168（族域述 24 测）。findings（r10）：0 新。

## SC-S1-18 卡审（914b558）——SetSamusStand 恒定写形抽核

### verdict: pass

四条核：①L2 面 s1-18-case×8 并入+**族 8 案 0 败（首提全过）**✓（增量 557→565=+8
精确；账内 10 稳=a1-001×8+s1-17×2 合账不变）②**恒定写形抽核** ✓：$CF55 入口形
（`ad 15 03 d0 03 20 96 cb`=LDA $0315 守卫+JSR 链）结构合形；8 写体由案实核钉死
（8 案恒定写=站姿常量+清组+$0680 声旗）③L3 0 失败容忍不增 ④员面双扫桩 0（4 员
真现）+跨卡面注（$D04D 尾跳）随证。测 169（族域述 24 测）。
findings（r10）：0 新。

## SC-S1-19 卡审（477e7aa）——AnimDrawObject 双 cmp 分路抽核

### verdict: pass

四条核：①L2 留白**真 0 双核 ✓**（pregen/l2_cases 双 0）②**双 cmp 分路抽核 ✓✓**：
$DE47=`20 8f dc a6 4b bd 03 03 c9 f7`=JSR $DC8F/LDX $4B/LDA $0303,X/**CMP #$F7**
——「$0303[x] 双 cmp 分路（$F7⇒$DF2D/$07⇒$6B 位清）」首 cmp 逐字命中 ③L3 0 失败
容忍不增（565/10 账内稳）④员面双扫桩 0（4 员真现）。测 170（族域述 26 测）。
findings（r10）：0 新。

## SC-S1-20 卡审（18b623d）——无写面出口形抽核

### verdict: pass

四条核：①L2 面 s1-20-case×8 并入+**族 8 案 0 败（首提全过）**✓（增量 565→573=+8
精确；账内 10 稳）②**无写面出口形抽核 ✓✓**：$D6E4=`20 6a d7 60`=JSR $D76A/RTS
壳形逐字；$D76A=`8a 48 a0 00 b1 04`=**TXA/PHA**/LDY #$00/LDA ($04),Y——「A=X/Y=0
txa 面」出口+表走例逐字命中 ③L3 0 失败容忍不增 ④员面双扫桩 0（4 员真现）。
测 171（族域述 27 测）。findings（r10）：0 新。

## SC-S1-21 卡审（50c341f）——三写位存形抽核

### verdict: pass

四条核：①L2 面 s1-21-case×8 并入+**族 8 案 0 败（八连首提全过）**✓（增量 573→581
=+8 精确；账内 10 稳）②**三写位存形抽核 ✓**：$E789=`20 92 e7 bd f6 6a`（JSR
root+LDA $6AF6,X 索引位存）结构合形——三写体（$0401[x]→$09/$0400[x]→$08/
$6AFB[x]→$0B）由 8 案实核钉死 ③L3 0 失败容忍不增 ④员面双扫桩 0（4 员真现）。
测 172（族域述 28 测）。findings（r10）：0 新。

## SC-S1-22 卡审（b003794）——EnemyLoop VM 深件三面抽核（重点）

### verdict: pass

四条核：①L2 面 s1-22-case×8 并入+**族 8 案 0 败（首提全过）**✓（增量 581→589=+8
精确；账内 10 稳）②**VM 深件三面抽核 ✓✓**（重点）：走带核 $EF09=`18 65 00 85 00
90 02 e6`=CLC/ADC $00/STA $00/BCC/INC——「$00/$01+=A 进位加」逐字命中（案 0：
181+125=306 定案随证）；VM root $EAD4=`20 09 ef a0 00 b1 00`（走带→fetch→分派序）
+记录长返回形（$EB09=`4c d4 ea` 回驱 JMP）——VM 核心形三解（jmp (ptr) 机理/记录长
返回约定/走带进位加）结构合形 ③L3 0 失败容忍不增 ④员面双扫桩 0（4 员真现）。
测 173（族域述 29 测）。findings（r10）：0 新。

## SC-S1-23 卡审（e0793d6）——VM 迁移同构核

### verdict: pass

四条核：①L2 面 s1-23-case×8 并入+**族 8 案 0 败（VM 形迁移全过）**✓（增量 589→597
=+8 精确；账内 10 稳）②**VM 迁移同构核 ✓✓**：$EDD6=`20 09 ef a0 00 b1 00`=JSR
$EF09（同一走带核）/LDY #$00/LDA ($00),Y——与 S1-22 $EAD4 VM root **逐字同构**
（走带迁移实证）；spawn 分派表 6 项+JumpEngine 面随证 ③L3 0 失败容忍不增 ④员面
双扫桩 0（4 员真现）+终止形（双 PLA 退）案实核。测 174（族域述 30 测）。
findings（r10）：0 新。

## SC-S1-24 卡审（9bf7ef1）——碰撞族抽核

### verdict: pass

四条核：①L2 留白**真 0 双核 ✓**（pregen/l2_cases 双 0）②**碰撞族抽核 ✓✓**：
$F140=`20 bf f1 20 86 f1 4c fa`=JSR GetRadiusSums/JSR $F186/JMP $F1FA——
「判段+尾跳 $F1FA」逐字命中；$F1BF=`bd f5 6a` LDA $6AF5,X 半径载（$05 工作面）
实存 ③L3 0 失败容忍不增（597/10 账内稳）④员面双扫桩 0（4 员真现）。
测 175（族域述 31 测）。findings（r10）：0 新。

## SC-S1-25 卡审（f6a059f）——Yplus16 对偶微体抽核

### verdict: pass

四条核：①L2 留白**真 0 双核 ✓**（pregen/l2_cases 双 0）②**对偶微体抽核 ✓✓**：
$F1EE=`98 18 69 10 a8 60 8a 38`=TYA/CLC/ADC #$10/TAY/RTS（Y+16）+TXA/SEC
（X-16 起）——「Y+16/X-16 对偶微体」逐字命中 ③L3 0 失败容忍不增（597/10 账内
稳）④员面双扫桩 0（4 员真现）。测 176（族域述 32 测）。findings（r10）：0 新。

## SC-S1-26 卡审（c9a95af）——IsSamusDead 三判链抽核

### verdict: pass

四条核：①L2 面 s1-26-case×8 并入+**族 8 案 0 败（首提全过）**✓（增量 597→605=+8
精确；账内 10 稳）②**三判链抽核 ✓✓**：$CE84=`ad 00 03 c9 07 f0 06 c9 08 f0`=
LDA $0300/**CMP #$07/BEQ/CMP #$08/BEQ**——「cmp 7/8/$FF 三判链」逐字命中；LDA
$0300 首载=A 不变面（cmp 不改 A）实证 ③L3 0 失败容忍不增 ④员面双扫桩 0（4 员
真现）。测 177（族域述 33 测）。findings（r10）：0 新。

## SC-S1-27 卡审（e8b6cf2）——delta 计算面抽核

### verdict: pass

四条核：①L2 留白**真 0 双核 ✓**（pregen/l2_cases 双 0）②**delta 计算面抽核 ✓✓**：
$F1FA=`a9 02 85 10 25 49 85 03 a5 07`=LDA #$02/STA $10/**AND $49/STA $03**/
LDA $07——「$49&2→$03 判段」逐字命中+delta 起始载（$07-$06→$00 计算面）实存
③L3 0 失败容忍不增（605/10 账内稳）④员面双扫桩 0（4 员真现）。测 178（族域述
34 测）。findings（r10）：0 新。

## SC-S1-28 卡审（ccc1a6e）——指令级操作数核对面（zp≠立即数陷阱族正案）

### verdict: pass

四条核：①L2 面 s1-28-case×8 并入+**族 8 案 0 败**✓（增量 605→613=+8 精确；
账内 10 稳）②**操作数类型核对面 ✓✓**（指令级）：**`45 ff`（EOR zp $FF）实证**
@$F17F/$F193（本簇 $F186 root 域）vs **`49 ff`（EOR imm）在他处**（$C3D4 取负
微体=正确 imm 形）——「eor $FF=zp 非立即数」双正案两形判别字节级实锤（位翻转误+
A 流尾恒定非 $0A 面）③L3 0 失败容忍不增 ④员面双扫桩 0（4 员真现）。
**教训注收讫**：操作数类型辨识（$FF 无 # 即 zp）——指令级操作数核对入自查条
（gotcha-operand-type-by-opcode-byte 族续案）。测 179（族域述 35 测）。
findings（r10）：0 新。

## S1 批产终审（25b75db 末卡+批终态）——批产首批收官

### verdict: pass（S1 批正式收官）

**末卡 SC-S1-29 抽核 ✓**：真 0 双核（pregen/l2_cases 双 0）+$F536 root=`bd 0f 04
85 0a bd 04 04`（索引载+工作面写=判段双判结构合形）+桩面 0+测 180（族域述 36 测）。
**批终态账核 ✓**：①案族并入 **144 条**（s1-*-case 实计；lead 约 80=保守估——实值
144 更实）②L2 总案 613（批前 ~461→613=S1 批 +152 案族面=pregen 152 对表兑现）
③记账面 10 稳（a1-001×8 路线图+s1-17×2 跨卡——退出条件均在案）④勾格全批态：
28 卡落地（S1-05/06 免重跳接）——有案簇=有L2→已验证（族内 0 败）/真 0 簇=已实现
留白注——三级位全批兑现。**六教训**（留白真 0 判/姊妹簇不可外推/x 索引加偏移/
操作数类型/表达式 return/rol-rsr 进位链）全数收讫入自查条族。
**S1 批产正式收官**（批产制首战：28 逐卡+10 连首提+0 卡打回——verdict 全 pass）。

## SC-S2-00 卡审（c1e1ea8）——S2 批首卡（方法迁移）

### verdict: pass

四条核：①L2 面 s2-030-case×8 并入+**族 8 案 0 败（首提全过）**✓（增量 613→621=+8
精确；账内 10 稳）——**S1 方法迁移 S2 落地实证** ②AND 写面抽核 ✓（$F6E2=`20 b3
f7 a5 49 c9 02`=JSR root/LDA $49/CMP #$02 结构合形；$0405[x]&=A 写面+出口 A=与
结果/X 透传/Y=0 由 8 案实核钉死）③L3 0 失败容忍不增 ④员面双扫桩 0（4 员真现）。
注：族 id=s2-030（cluster 编号制——grep 正形教训随账：族 id 以 pregen 实值为准）。
测 181（族域述 37 测）。findings（r10）：0 新。

## SC-S2-01 卡审（a77b64f）——跨卡账目语义+操作数辨识应用面

### verdict: pass

四条核：①L2 面 s2-031-case×8 并入（4 strict 0 败+4 账内）✓（增量 621→629=+8
精确）——**跨卡账目语义核 ✓**：gap_map 4 条单列（深径 $0200 区写族=S1-17 同款跨卡
效应）+账内容忍 **14 案算术合**（10 前账+4 新条）+「账清转 strict」退出条件在案
②**操作数辨识应用面 ✓**：案 7 $0409 非索引实核（操作数类型自查条应用实案）+
$F702=`20 44 f7 4a 6a 5d 03 04`（JSR root/LSR/ADC/ORA $0403,X 三分径结构合形）
③L3 0 失败容忍不增 ④员面双扫桩 0（4 员真现）。钩面格式自查条收讫。测 182（族域述
37+ 测）。findings（r10）：0 新。

## SC-S2-02 卡审（b566443）——22 写实核/深链入口形抽核

### verdict: pass

四条核：①L2 面 s2-032-case×8 并入+**族 8 案 0 败（首提全过）**✓（增量 629→637=+8
精确；账内 14 稳）②**深链入口形抽核 ✓**：$F97C=`a9 01 20 94 e0 4c`=LDA #$01/JSR
$E094/JMP $DD8B（init/调/尾跳深）结构合形；$FA7A=`4c 7c f9`=JMP $F97C 壳形
（S1-08 同构面=JMP-root+索引载族）✓；22 写实核（定 21+$6AF7←ram[$00] 依存）由
8 案钉死 ③L3 0 失败容忍不增 ④员面双扫桩 0（4 员真现）。测 183。
findings（r10）：0 新。

## SC-S2-03 卡审（46a5957）——SamusHit 判链抽核

### verdict: pass

四条核：①L2 面 s2-033-case×8 并入+**族 8 案 0 败（首提全过）**✓（增量 637→645=+8
精确；账内 14 稳）②**判链/早退径抽核 ✓✓**：$CF2E=`ad 0a 03 4a 29 02 f0 1e`=
LDA $030A/LSR/AND #$02/**BEQ +$1E 早退**——「8 案全早退径（A=0 无写）」逐字命中
③L3 0 失败容忍不增 ④员面双扫桩 0（3 员真现）。测 184。findings（r10）：0 新。

## SC-S2-04 卡审（d5d731f）——S1-21 对偶形/S1-10 同构面抽核

### verdict: pass

四条核：①L2 留白**真 0 三方核 ✓**（s1-pregen/s2-pregen/l2_cases 全 0）②**对偶形
抽核 ✓✓**：$E8BE=`bd 0c 03 85 0b bd 0d 03`=LDA $030C,X/STA $0B/LDA $030D,X——
「对象位置暂存三写」S1-21 对偶逐字命中；$D630=`20 be e8 20 8f fd`（JSR root+JSR
$FD8F）=S1-10 Table18 同构面实存 ③L3 0 失败容忍不增（645/14 账内稳）④员面双扫
桩 0（3 员真现）。测 185。findings（r10）：0 新。

## SC-S2-05 卡审（0046dc5）——BitScan 位扫描微体抽核

### verdict: pass

四条核：①L2 留白**真 0 双核 ✓**（pregen/l2_cases 双 0）②**位扫描微体抽核 ✓✓**：
$E1E1=`86 0e a2 00 4a b0 05 e8 e0 08`=STX $0E/X 透传暂存/LDX #$00/LSR/BCS/INX/
**CPX #$08**——「首置位 0-7/8 无置位」逐字命中 ③L3 0 失败容忍不增（645/14 账内
稳）④员面双扫桩 0（3 员真现）。测 186。findings（r10）：0 新。

## SC-S2-06 卡审（e0e5ab7）——StartMusic 触发判抽核

### verdict: pass

四条核：①L2 留白**真 0 双核 ✓**（pregen/l2_cases 双 0）②**触发判抽核 ✓✓**：
$D92C=`ad 20 03 c9 06 d0 05 ad 2f 03`=LDA $0320/CMP #$06/**BNE 电梯态跳过**/
LDA $032F——「电梯态跳过」逐字命中（$79 三判段 bmi/beq/常态随证）③L3 0 失败
容忍不增（645/14 账内稳）④员面双扫桩 0（3 员真现）。测 187。
注：SC-S2-07 预告（$CDFA=m2 已折同址族——免重/重合判定形）下一卡随形。
findings（r10）：0 新。

## SC-S2-07 卡审（14623b9）——双视注记/分支向抽核（免重判定形）

### verdict: pass

四条核：①L2 面 s2-037-case×8 并入+**族 8 案 0 败**✓（增量 645→653=+8 精确；
账内 14 稳）②**分支向抽核 ✓✓**：$CDFA=`ad 0a 03 29 20 f0 2f a9 32 85`=LDA $030A/
**AND #$20/BEQ +$2F(=47)**/LDA #$32/STA——「beq +47=写径/位 5 置=hit 径」分支向
正案逐字命中 ③**免重判定形处置 ✓**：真转写+双视注记（$D406/$D411=S1-11 流区重合
（跳面已折/员面本卡现）——m2 同址族核对表过=同址双表示显式声明不冒领）④员面双扫
桩 0（3 员真现）。测 188。findings（r10）：0 新。

## SC-S2-08 卡审（af3eb9d）——Amul16/位面抽核

### verdict: pass

四条核：①L2 面 s2-038-case×8 并入+**族 8 案 0 败（首提全过）**✓（增量 653→661=+8
精确；账内 14 稳）②**Amul16/位面抽核 ✓✓**：$CD92=`a5 4d 20 c5 c2 05 6b 85`=LDA
$4D/**JSR $C2C5（Amul16 ×16）**/**ORA $6B**/STA——「$4D Amul16+ora $6B→$6B 镜像
控制位置位」逐字命中 ③L3 0 失败容忍不增 ④员面双扫桩 0（3 员真现）+双视注记
（$D414 尾跳）随证。测 189。findings（r10）：0 新。

## SC-S2-09 卡审（a62b1db）——$E519 三分判抽核

### verdict: pass

四条核：①L2 留白**真 0 双核 ✓**（pregen/l2_cases 双 0）②**三分判抽核 ✓✓**：
$E519=`a6 49 ca f0 0a 10 28 e6 49`=LDX $49/DEX/**BEQ +$0A/BPL +$28/INC $49**——
「beq/bpl/负径 inc」三分判逐字命中（测入参径选择 bpl 前置判=S1-10 同教训收讫）
③L3 0 失败容忍不增（661/14 账内稳）④员面双扫桩 0（3 员真现）。测 190。
findings（r10）：0 新。

## SC-S2-10 卡审（12ce6df）——$2D bit0 早退径抽核

### verdict: pass

四条核：①L2 面 s2-040-case×8 并入+**族 8 案 0 败（首提全过）**✓（增量 661→669=+8
精确；账内 14 稳）②**早退径抽核 ✓✓**：$D47E=`a5 2d 4a 90 08 20 92 cd`=LDA $2D/
LSR（bit0→carry=A>>1）/**BCC +8 早退**/JSR $CD92——「清⇒rts 早退（A=$2D>>1 无写）；
置⇒双接面链」逐字命中（双接面链=调 S2-08 SetmirrorCntrlBit 族 ✓ 跨卡同址族自洽）
③L3 0 失败容忍不增 ④员面双扫桩 0（3 员真现）。测 191。findings（r10）：0 新。

## SC-S2-11 卡审（58ef72b）——$E4F1 对偶 dec 走带抽核

### verdict: pass

四条核：①L2 留白**真 0 双核 ✓**（pregen/l2_cases 双 0）②**对偶抽核 ✓✓**：
$E4F1=`a5 49 f0 0c c9 01 d0 1e c6 49`=LDA $49/**BEQ/CMP #$01/BNE/DEC $49**——
「$E519 对偶（beq/cmp1 二判+dec 族）」逐字命中（$E519=INC 负径/$E4F1=DEC 回退=
状态步进对偶自洽）③L3 0 失败容忍不增（669/14 账内稳）④员面双扫桩 0（3 员真现）
+S1-11 同构面/S1-01 族回驱随证。测 192。findings（r10）：0 新。

## SC-S2-12 卡审（0bd426b）——$8296 镜面共享形/条件取负抽核

### verdict: pass

四条核：①L2 留白**真 0 双核 ✓**（pregen/l2_cases 双 0）②**镜面共享形抽核 ✓✓**：
banks1-5 $8296 **全同**（`0a 08 20 be c2 28 90 04 49 ff`=ASL/PHP/JSR $C2BE/PLP/
BCC +4/**EOR #$FF**）——「asl/plp/条件取负（eor $FF/adc #0）」逐字命中；areas_common
形三=banks1-5 镜面共享实证（首核对比基准错向（对 bank0）→互比自纠=镜面核正读法
教训随账）③L3 0 失败容忍不增（669/14 账内稳）④员面双扫桩 0（3 员真现）。测 193。
findings（r10）：0 新。

## SC-S2-13 卡审（e3973b1）——全账内卡毕形（新判定面：分账/降级注/三前提）

### verdict: pass（新形样板）

四条核：①L2 面 s2-043-case×8 **全账内** ✓（增量 669→677=+8 精确；账内 14→22=
+8 算术合）②**归属分账语义 ✓**（裁定①）：VRAM 缓冲写族=$07A0-$07EF→S6/title
已折域效应（**跨域账**）+zp $00-$06+残迹 regs=移动链效应（**跨卡账** S1-05 族）
——gap_map 2 条分条登记各自退出条件=分账语义精确 ③**降级注记形 ✓**（裁定②）：
勾格=「有L2（账内）」非「已验证」——**三前提全核**：员面真现（0 桩双扫 ✓）+
engine 族测独立盖本卡效应（194 测 ✓）+账目分条退出条件（✓）④员面 3 员真现。
**全账内卡毕形=新判定面样板**（不冒领已验证级位）。测 194。findings（r10）：0 新。

## SC-S2-14 卡审（bed716b）——CheckBlastTile 双判抽核

### verdict: pass

四条核：①L2 面 s2-044-case×8 并入+**族 8 案 0 败（首提全过）**✓（增量 677→685=+8
精确；账内 22 稳）②**双判抽核 ✓✓**：$D651=`a4 74 c0 10 f0 04 c9 70 b0 02`=LDY
$74（Y=ram[$74] 面）/CPX #$10/BEQ/**CMP #$70/BCS**——「$70<=A<$80 可破」双判
下界逐字命中（A/X 透传无写面随证）③L3 0 失败容忍不增 ④员面双扫桩 0（3 员真现）。
测 195。findings（r10）：0 新。

## SC-S2-15 卡审（a2301dc）——SetProjectileAnim 索引对偶抽核

### verdict: pass

四条核：①L2 面 s2-045-case×8 并入+**族 8 案 0 败（首提全过）**✓（增量 685→693=+8
精确；账内 22 稳）②**索引对偶抽核 ✓✓**：$D2FA=`9d 05 03 9d 06 03 a9 00 9d 04`=
STA $0305,X/STA $0306,X/LDA #$00/STA $0304,X——与 S1-09 $CF6B（`8d` 非索引三写）
**完美对偶**（9d=STA abs,X 索引版）逐字命中 ③L3 0 失败容忍不增 ④员面双扫桩 0
（3 员真现）。测 196。findings（r10）：0 新。

## SC-S2-16 卡审（9c08925）——$D66B 动画步进抽核

### verdict: pass

四条核：①L2 留白**真 0 双核 ✓**（pregen/l2_cases 双 0）②**步进形抽核 ✓**：
$D66B=`a9 03 4c 47 de a5 2d 4a 90 19`=LDA #$03/JMP $DE47（S1-19 AnimDrawObject
同构链）/LDA $2D/LSR/**BCC bit0 判**——「$2D bit0 判+dec 计数+表选写」结构合形
（$37/$82 二值=S1-19 同构面+sb c $F7/指针拷贝=S1-13 同构随证）③L3 0 失败容忍不增
（693/22 账内稳）④员面双扫桩 0（3 员真现）。测 197。findings（r10）：0 新。

## SC-S2-17 卡审（5527e59）——$D648 索引 toggle 对偶抽核

### verdict: pass

四条核：①L2 面 s2-047-case×8 并入+**族 8 案 0 败（首提全过）**✓（增量 693→701=+8
精确；账内 22 稳）②**索引 toggle 对偶抽核 ✓✓**：$D648=`bd 0c 03 49 01 9d 0c 03`=
LDA $030C,X/EOR #$01/STA $030C,X——与 S1-11 $E249（`ad/8d` 非索引 toggle）**完美
对偶**（bd/9d=abs,X 索引版）逐字命中 ③L3 0 失败容忍不增 ④员面双扫桩 0（3 员真现）。
测 198。findings（r10）：0 新。

## SC-S2-18 卡审（9374cfb）——跨卡溯源链/记录长表抽核（VM 深件二号）

### verdict: pass

四条核：①L2 面 s2-048-case×8（**7 strict 0 败+1 账内**）✓（增量 701→709=+8 精确；
账内 22→23=+1 算术合（case-4 CROSSCARD））②**溯源链抽核 ✓**：walk 表 handler
跨卡实证（$EC2F∈S1-00/$EC04∈S2-49 溯源定案）+$F0A2 深分派非本卡员→CROSSCARD
记账=归属分判精确 ③**走带/记录长表抽核 ✓**：$EB8C=`20 92 eb 4c d4 ea`（JSR $EB92/
JMP $EAD4=S1-22 VM root 同链回驱）+$EB92=`c8 b1 00 48 20 c5 c2`（INY/LDA ($00),Y
记录 fetch/JSR Amul16）——多记录走带+记录长表（op2=2/op1=3）结构合形 ④员面双扫
桩 0（3 员真现）。测 198+。findings（r10）：0 新。

## SC-S2-19 卡审（365e57b）——自洽核点（账目源卡=退出部分达成）

### verdict: pass

四条核：①L2 面 s2-049-case×8 并入+**族 8 案 0 败（walk 形迁移全过）**✓（增量
709→717=+8 精确；账内 23 稳=case-4 项余 $F0A2 未列如实）②**自洽核点 ✓**（重点）：
$EC04=S2-18 账目源卡——本卡落=case-4 账目退出条件**部分达成**（$EC04 handler 集
建模清；余项如实：$EC2F（S1-00 已落）+$F0A2（未列）——退出进度显式不冒领全清）
③walk 形迁移抽核 ✓：$EC09=`ad 20 03 d0 1e c8 b1 00`=LDA $0320/BNE/INY/LDA ($00),Y
（记录 fetch=S2-18 同形）+$EC04 回驱+电梯 handler 六写恒效面 ④员面双扫桩 0（3 员
真现）。测 200（族域述）。findings（r10）：0 新。

## SC-S2-20 卡审（0347afa）——walk 形三连/记录长 1 抽核

### verdict: pass

四条核：①L2 面 s2-050-case×8 并入+**族 8 案 0 败**✓（增量 717→725=+8 精确；
账内 23 稳）②**记录长 1 实证 ✓✓**：$EEC6=`a9 01 d0 e1 20 b7 95 8a`=LDA #$01/BNE
——「入口记录长 1（lda #1）」逐字命中（入口长族谱 2/2/1=三连 walk 形 VM 长度
谱系实证）③L3 0 失败容忍不增 ④员面双扫桩 0（3 员真现）。测 201。
findings（r10）：0 新。

## SC-S2-21 卡审（f206797）——Xminus16 对偶/依存三对抽核

### verdict: pass

四条核：①L2 面 s2-051-case×8 并入+**族 8 案 0 败（首提全过）**✓（增量 725→733=+8
精确；账内 23 稳）②**Xminus16 对偶抽核 ✓✓**：$F1F4=`8a 38 e9 10 aa 60`=TXA/SEC/
**SBC #$10**/TAX/RTS——X-=16 微体逐字命中（与 S1-25 Yplus16（TYA/CLC/ADC #$10）=
**算术对偶**谱系实证）③依存三对实核（$030D→$06/$07、$030E→$08/$09、0→$0A/$0B
+出口 A=0/X=$C0 环残迹/Y=0）由 8 案钉死 ④员面双扫桩 0（3 员真现）。测 202。
findings（r10）：0 新。

## SC-S2-22 卡审（cc0078c）——SamusHurt 受击族抽核

### verdict: pass

四条核：①L2 留白**真 0 双核 ✓**（pregen/l2_cases 双 0）②**受击族抽核 ✓**：$F311=
`b0 16 a9 e0 8d 0f 01`=BCS cs 分段/LDA #$E0/STA $010F——「cs 分段+$0F 判（$73←1）
+$6E/$6F 指针清」结构合形（受伤态初始化）③L3 0 失败容忍不增（733/23 账内稳）
④员面双扫桩 0（3 员真现）。测 203。findings（r10）：0 新。

## SC-S2-23 卡审（f249cc0）——SubtractHealth 早退径抽核

### verdict: pass

四条核：①L2 面 s2-053-case×8 并入+**族 8 案 0 败（首提全过）**✓（增量 733→741=+8
精确；账内 23 稳）②**早退径抽核 ✓✓**：$CE92=`a5 6e 05 6f f0 f9`/JSR…=LDA $6E/
ORA $6F/**BEQ 指针判早退**——「8 案全 A=0 无写」逐字命中 ③L3 0 失败容忍不增
④员面双扫桩 0（3 员真现）。测 204。findings（r10）：0 新。

## SC-S2-24 卡审（306071c）——IsScrewAttackActive 透传族抽核

### verdict: pass

四条核：①L2 面 s2-054-case×8 并入+**族 8 案 0 败（首提全过）**✓（增量 741→749=+8
精确；账内 23 稳）②**透传族抽核 ✓✓**：$CD9C=`38 ac 00 03 88 d0 1b`=SEC/LDY $0300/
**DEY**（Y=ram[$0300]-1 ldy/dey 链）/BNE——逐字命中（A/X 透传无写面随证）③L3 0
失败容忍不增 ④员面双扫桩 0（3 员真现）。测 205。findings（r10）：0 新。

## SC-S2-25 卡审（bc566ca）——Amul8 asl×3 回卷抽核

### verdict: pass

四条核：①L2 面 s2-055-case×8 并入+**族 8 案 0 败（首提全过）**✓（增量 749→757=+8
精确；账内 23 稳）②**asl×3 回卷抽核 ✓✓**：$C2C6=`0a 0a 0a 60`=ASL/ASL/ASL/RTS
——「A×8 回卷」逐字命中；回卷算术合（133×8 mod 256=40 ✓ 案实核）；**Adiv8 对偶**
谱系（S2-25 Amul8/S1-07 Exit22 Adiv8=乘除对偶）③L3 0 失败容忍不增 ④员面双扫
桩 0（3 员真现）。测 206。findings（r10）：0 新。

## SC-S2-26 卡审（632cc07）——ForceSpeedTowardsSamus 朝向计算抽核

### verdict: pass

四条核：①L2 留白**真 0 双核 ✓**（pregen/l2_cases 双 0）②朝向计算形 ✓（$F6B9=
`a9 00 85 82 20 4b f7 a8`=LDA #$00/STA $82 速初/JSR 链/TAY 结构合形）③L3 0
失败容忍不增（757/23 账内稳）④员面双扫桩 0（3 员真现）。
注：**x 索引七犯自纠**（强制步持续生效=教训条实战价值证明）。测 207。
findings（r10）：0 新。

## SC-S2-27 卡审（8d5e035）——DistanceToSamusThreshold 阈值族抽核

### verdict: pass

四条核：①L2 留白**真 0 双核 ✓**（pregen/l2_cases 双 0）②**阈值族抽核 ✓✓**：
$F75B=`a9 e7 85 06 a9 18 20 44`=LDA #$E7/**STA $06**（$06←$E7 逐字）/LDA #$18/
JSR——「$06←$E7+OrEnData05 调+表走位 1 判段」结构合形（S2-01 同族接面随证）③L3 0
失败容忍不增（757/23 账内稳）④员面双扫桩 0（3 员真现）。测 208。
findings（r10）：0 新。

## SC-S2-28 卡审（3872bd1）——PageIndex 恒定形抽核

### verdict: pass

四条核：①L2 面 s2-058-case×8 并入+**族 8 案 0 败（首提全过）**✓（增量 757→765=+8
精确；账内 23 稳）②恒定形抽核 ✓（$F416=`a6 4b bd 0f 04 10 08`=LDX $4B/LDA
$040F,X/BPL 符号段结构合形；恒定写 $0404←0/$040E←0+A=0/X=0/Y 透传由 8 案钉死）
③L3 0 失败容忍不增 ④员面双扫桩 0（3 员真现）。测 209。findings（r10）：0 新。

## SC-S2-29 卡审（0b52f7f）——y 索引假象排除（操作数辨识面）

### verdict: pass

四条核：①L2 面 s2-059-case×8 并入+**族 8 案 0 败**✓（增量 765→773=+8 精确；
账内 23 稳）②**定址辨识面 ✓**（案 7 trace 实核：$09/$6AF6←zp $FB 定址，y 索引
假象排除=操作数辨识条再应用）——**诚实注**：$FB 定址位在我扫描窗口外（$F515 root
区+邻域未及），定址判据=案 7 trace 证据面+8 案钉死（非我字节定位级——如需字节级
复核可随补）③L3 0 失败容忍不增 ④员面双扫桩 0（3 员真现）。测 210。
findings（r10）：0 新。

## S2 批产终审（S2-00→S2-59 全清）——批产第二批收官（两批连续全胜）

### verdict: pass（S2 批正式收官）

**批终态账核 ✓**：①案族并入 **160 条**（s2-*-case 实计；lead 约 96=保守估——实值
160 更实，同 S1 终审形态）②L2 总案 773（S1 收官 653→773=S2 批 +120 案族面）+
**账内 23 稳**（a1-001×8+s1-17×2+s2-013 分账 2+s2-048 case-4×1+余迭代态——退出
条件均在案）③勾格三分法全批兑现：strict 族=已验证（0 败）/真 0 簇=已实现留白注/
全账内卡（S2-13）=有L2（账内）降级注记——**判定形三分法全用** ✓（首提 15+ 连）。
测 210（族域述 66 测 0 败）。教训再增（x 索引七犯/镜面正读法/操作数辨识应用）。
**S2 批产正式收官=两批连续全胜**（S1 28 逐卡+S2 30 逐卡=verdict 全 pass 零打回）。

## SC-S3-00 卡审（22fcd37）——S3 批首卡（方法迁移 S3）

### verdict: pass

四条核：①L2 面 s3-060-case×8 并入+**族 8 案 0 败（首提全过）**✓（增量 773→781=+8
精确；账内 23 稳）——**S2 方法迁移 S3 落地实证** ②**SFX 恒定形抽核 ✓✓**：$CBD2=
`a9 20 d0 1f a9 02 d0 25`=LDA #$20/BNE/LDA #$02/BNE——「$0683←$20 声旗/A=$20/
X=3/Y 透传」恒定形逐字命中 ③L3 0 失败容忍不增 ④员面双扫桩 0（3 员真现）。
测 211（族域述）。findings（r10）：0 新。

## SC-S3-01 卡审（4ff46d9）——ReadTableAt968B 表读族抽核

### verdict: pass

四条核：①L2 留白**真 0 三方核 ✓**（pregen/l2_cases 全 0）②**表读形抽核 ✓✓**：
$F74B=`bc 02 6b b9 8b 96 60`=LDY $6B02,X/**LDA $968B,Y**/RTS——「表读例」名实
逐字命中（ReadTableAt968B 字节面自证）③L3 0 失败容忍不增（781/23 账内稳）④员面
双扫桩 0（3 员真现）。x 索引**八犯自纠**（强制步持续）。测 212。findings（r10）：0 新。

## SC-S3-02 卡审（1f1a8e1）——SFX_EnemyHit 声旗对偶形抽核

### verdict: pass

四条核：①L2 面 s3-062-case×8 并入+**族 8 案 0 败（首提全过）**✓（增量 781→789=+8
精确；账内 23 稳）②**声旗对偶形抽核 ✓✓**：$CBB0=`a9 04 d0 0e a9 10 d0 0a`=LDA #$04/
BNE/LDA #$10/BNE——与 S3-00 $CBD2（`a9 20` LDA #$20）**声旗对偶逐字**（同构不同
常量：$0681←$04 vs $0683←$20；A=$04/X=1 vs A=$20/X=3）③L3 0 失败容忍不增 ④员面
双扫桩 0（3 员真现）。测 213。findings（r10）：0 新。

## SC-S3-03 卡审（d7a75a0）——LoadEnHiToY 恒定形抽核

### verdict: pass

四条核：①L2 面 s3-063-case×8 并入+**族 8 案 0 败（首提全过）**✓（增量 789→797=+8
精确；账内 23 稳）②**恒定形抽核 ✓✓**：$F752=`bd fb 6a a8 4d 0c 03 4a`=LDA $6AFB,X/
TAY（Y=EnHi）/**EOR $030C**（eor 进位）/LSR——「EnHi 载入+eor 进位」逐字命中
③L3 0 失败容忍不增 ④员面双扫桩 0（3 员真现）。测 214（族域述 70 测里程碑）。
findings（r10）：0 新。

## SC-S3-04 卡审（daa2d72）——转化面/三依存同源抽核

### verdict: pass

四条核：①L2 面 s3-064-case×8 并入+**族 8 案 0 败（首提全过）**✓（增量 797→805=+8
精确；账内 23 稳）②**转化面抽核 ✓**：$FA5B=`bd 04 04 f0 0a a9 00 9d 04 04`=LDA
$0404,X/BEQ/LDA #$00/STA $0404,X（判+清转化）逐字命中；**三依存同源**（ram[$0400]
$02←直值/$04←&$40/$05←(>>6&7)|$60 三形）由 8 案实核钉死 ③L3 0 失败容忍不增 ④员面
双扫桩 0（3 员真现）。测 215。findings（r10）：0 新。

## SC-S3-05 卡审（f9eb2a2）——EnemyBGCollide 恒定形抽核

### verdict: pass

四条核：①L2 面 s3-065-case×8 并入+**族 8 案 0 败（首提全过）**✓（增量 805→813=+8
精确；账内 23 稳）②恒定形抽核 ✓（$FA1E=`a5 74 c9 11 d0 06`=LDA $74/CMP #$11 判段
结构合形；恒定写 $02/$03/$04←0/$05←$60/$0B←0（$0400 面全 0）由 8 案钉死）③L3 0
失败容忍不增 ④员面双扫桩 0（3 员真现）。测 216。findings（r10）：0 新。

## SC-S3-06 卡审（90bede1）——SamusJump 分支向正抽核（教训再证）

### verdict: pass

四条核：①L2 面 s3-066-case×8（**6 strict 0 败+2 账内**）✓（增量 813→821=+8 精确；
账内 23→25=+2 算术合（case-5/6 跨卡））②**分支向正抽核 ✓✓**：$D055 全区=
`a2 01 a0 00 a5 14 4a b0 06 ca 4a 90 30`——**`90 30`=BCC +48**（跳过非执行）定谳
命中（首窗 b0=BCS 一字之差→全区定谳=分支向正案教训再证：分支向辨识须看目标量非
邻近 opcode）③S1-17 链族全同（41 写）迁移+gap_map +2 分条 ④员面双扫桩 0（2 员
真现）。测 217。findings（r10）：0 新。

## SC-S3-07 卡审（e7df1aa）——Table04 环判抽核（S1-17 族三现）

### verdict: pass

四条核：①L2 面 s3-067-case×8（**6 strict+2 账内**）✓（增量 821→829=+8 精确；
账内 25→27=+2 算术合（深径跨卡））②**环判抽核 ✓**：$D09C=`a5 12 05 16 0a 10 f2`=
LDA $12/ORA $16/ASL/**BPL -$0E 回环**——「Table04 环判」逐字命中 ③S1-17 链族
**三现**（41 写全同迁移）+gap_map +2 分条 ④员面双扫桩 0（2 员真现）。
测 218（移位算误自纠随账）。findings（r10）：0 新。

## SC-S3-08 卡审（d347a85）——SetSamusJump 恒定 7 写抽核

### verdict: pass

四条核：①L2 面 s3-068-case×8 并入+**族 8 案 0 败（首提全过）**✓（增量 829→837=+8
精确；账内 27 稳）②恒定 7 写抽核 ✓（$CFC3=`a0 0c 8c 05 03 88 8c 06 03`（LDY #$0C/
STY $0305/DEY/STY $0306 计数恒定链）结构合形；7 写体（$0304-$030F 跳姿常量+$0314+
$0681←2 声旗）由 8 案钉死）③L3 0 失败容忍不增 ④员面双扫桩 0（2 员真现）。测 219。
findings（r10）：0 新。

## SC-S3-09 卡审（36e777a）——$CD6B 态链族抽核

### verdict: pass

四条核：①L2 留白**真 0 三方核 ✓**（pregen/l2_cases 全 0）②**态链族抽核 ✓✓**：
$CD6B=`a9 03 20 8f dc 20 9c cd`=LDA #$03/JSR $DC8F/**JSR $CD9C**——IsScrewAttack
Active 接面（S2-24）逐字命中（cs 分段+$6B 写族随证）③L3 0 失败容忍不增（837/27
账内稳）④员面双扫桩 0（2 员真现）。测 220。findings（r10）：0 新。

## SC-S3-10 卡审（ca39aad）——SetSamusPntUp 恒定 7 写抽核

### verdict: pass

四条核：①L2 面 s3-070-case×8 并入+**族 8 案 0 败（首提全过）**✓（增量 837→845=+8
精确；账内 27 稳）②**恒定 7 写抽核 ✓✓**：$CF77=`a9 04 8d 00 03 a9 27 20 6b cf`=
LDA #$04/STA $0300/**LDA #$27/JSR $CF6B（SetSamusAnim 调面）**——逐字命中（7 写体
$0305/$0306←$27+清组由 8 案钉死）③L3 0 失败容忍不增 ④员面双扫桩 0（2 员真现）。
测 221。findings（r10）：0 新。

## SC-S3-11 卡审（859cba2）——SFX_SamusBall 声旗三现抽核

### verdict: pass

四条核：①L2 面 s3-071-case×8 并入+**族 8 案 0 败（首提全过）**✓（增量 845→853=+8
精确；账内 27 稳）②**声旗三现抽核 ✓✓**：$CBEF=`a9 02 d0 02 a9 08 a2 03`=LDA #$02/
BNE/LDA #$08/**LDX #$03**——「$0683←$02/A=$02/X=3」逐字命中；**SFX 族三部曲**
（S3-00 `a9 20` $20/S3-02 `a9 04` $04/S3-11 `a9 02` $02——同构声旗恒定谱系）③L3 0
失败容忍不增 ④员面双扫桩 0（2 员真现）。测 222。findings（r10）：0 新。

## SC-S3-12 卡审（efb8dfc）——CheckMoveUp 位置差值面抽核

### verdict: pass

四条核：①L2 留白**真 0 双核 ✓**（pregen/l2_cases 双 0）②**位置差值面抽核 ✓✓**：
$E7A2=`a6 4b bd 01 03 18 69 08`=LDX $4B/LDA $0301,X/**CLC/ADC #$08**——
「$0301[x]+8 上移检查」逐字命中（0-$0301[x]→$02 减法面随证；多面复用=S1-21/
S1-10/S2-18 接面）③L3 0 失败容忍不增（853/27 账内稳）④员面双扫桩 0（2 员真现）。
测 223。findings（r10）：0 新。

## SC-S3-13 卡审（63b85db）——LoadObjectPosition 载入对偶抽核

### verdict: pass

四条核：①L2 留白**真 0 双核 ✓**（pregen/l2_cases 双 0）②**载入对偶抽核 ✓✓**：
$D638=`a5 08 9d 0d 03 a5 09 9d 0e 03`=LDA $08/STA $030D,X/LDA $09/STA $030E,X
——「$08→$030D[x]/$09→$030E[x]」逐字命中（S1-21/S2-34 Store 族**完美载入对偶**：
Store=030D→08/Load=08→030D）；$0B 位 0 判（$030C[x] eor 1）随证 ③L3 0 失败容忍
不增（853/27 账内稳）④员面双扫桩 0（2 员真现）。测 224（族域述 80 测里程碑）。
findings（r10）：0 新。

## SC-S3-14 卡审（f356522）——ROM 表烘焙法抽核（新法入册）

### verdict: pass

四条核：①L2 面 s3-074-case×8 并入+**族 8 案 0 败（首提全过）**✓（增量 853→861=+8
精确；账内 27 稳）②**ROM 表烘焙法抽核 ✓✓**（新法入册）：$CCB7=`bd c0 cc 8d 15 03
60`=LDA $CCC0,X/STA $0315/RTS 表读例逐字；**表值双点字节级实核**：TBL[186]=9 ✓/
TBL[243]=56 ✓（$CCC0 256B RUN_ACCEL_TBL 烘焙值与案实核双点吻合）③L3 0 失败容忍
不增 ④员面双扫桩 0（2 员真现）。测 225。findings（r10）：0 新。

## SC-S3-15 卡审（41101c1）——$D150 三判链/asl 回卷面抽核

### verdict: pass（竞态注）

四条核：①L2 留白**真 0 双核 ✓**（pregen/l2_cases 双 0）②**三判链/回卷面抽核 ✓**：
$D150=`ad 78 68 4a 90 41`=LDA $6878/LSR/BCC 结构合形；**asl 回卷面验算合**（0xC0<<1
=0x80 负径入参）+carry 回元组随证 ③L3 0 失败容忍不增（861/27 账内稳）④员面双扫
桩 0（2 员真现）。**竞态注**：首跑全量撞 WIP 写入中间态（s3_075_units_two 1 败）→
单测独跑过+复跑全量 **226/0 全绿**（count 225→226=其 WIP 又落一测）——共享树
WIP 竞争瞬态非卡缺陷（同型前案族）。回卷自查条收讫。findings（r10）：0 新。

## SC-S3-16 卡审（43180c2）——ClearHorzData 恒定 3 清写抽核

### verdict: pass

四条核：①L2 面 s3-076-case×8 并入+**族 8 案 0 败（首提全过）**✓（增量 861→869=+8
精确；账内 27 稳）②恒定 3 清写抽核 ✓（$CFB7=`20 4c cf 8c 15 03 60`=JSR $CFC4/
STY $0315/RTS 清写链合形；$0309/$0313/$0315←0 清族+A/X 透传/Y=0 由 8 案钉死）
③L3 0 失败容忍不增 ④员面双扫桩 0（2 员真现）。测 227。findings（r10）：0 新。

## SC-S3-17 卡审（0f2fae4）——SFX_BombLaunch 声旗四现抽核

### verdict: pass

四条核：①L2 面 s3-077-case×8 并入+**族 8 案 0 败**✓（增量 869→877=+8 精确；
账内 27 稳）②**声旗四现抽核 ✓✓**：$CBA8=`a9 01 d0 49 a9 02 d0 12`=LDA #$01/BNE/
LDA #$02/BNE——「$0683←$01」逐字命中；**SFX 族四部曲**（$20/$04/$02/$01——声旗
恒定谱系四部收官）③L3 0 失败容忍不增 ④员面双扫桩 0（2 员真现）。测 228。
findings（r10）：0 新。

## SC-S3-18 卡审（126646a）——FireWeapon 开火判/槽搜抽核

### verdict: pass

四条核：①L2 留白**真 0 三方核 ✓**（pregen/l2_cases 全 0）②**开火判抽核 ✓✓**：
$D1EE=`a5 14 29 08 f0 1c 4c 75 d2 a0`=LDA $14/**AND #$08（bit3 开火判）**/BEQ/
JMP/LDY（槽搜 Y 环起）——逐字命中 ③L3 0 失败容忍不增（877/27 账内稳）④员面双扫
桩 0（2 员真现）+JumpEngine 同构面注（S1-22）。测 229。findings（r10）：0 新。

## SC-S3-19 卡审（1842afc）——JumpEngine 机理正身（S1-22 本体归位）

### verdict: pass

四条核：①L2 留白**真 0 三方核 ✓**（pregen/l2_cases 全 0）②**JumpEngine 本体抽核
✓✓**：$C27C=`0a 84 6a 86 69 a8 c8 68 85 0c 68 85`=**ASL（A*2 索引）/STY $6A/STX
$69（TempY/TempX 保存）/TAY/INY/PLA×2（返回址读栈）**/STA $0C——S1-22 机理正身
逐字命中（「jmp (ptr) 机理」内联表返回址读栈=VM 深件引用的本体现位）③L3 0 失败
容忍不增（877/27 账内稳）④员面双扫桩 0（2 员真现）。测 230。findings（r10）：0 新。

## SC-S3-20 卡审（95b9270）——清除环抽核（bpl 四环+环破点）

### verdict: pass

四条核：①L2 留白**真 0 三方核 ✓**（pregen/l2_cases 全 0）②**清除环抽核 ✓✓**：
$D48C=`a2 60 38 20 b4 d4 8a e9 20 aa 10 f7`=LDX #$60/SEC/JSR $D4B4/TXA/**SBC
#$20/TAX/BPL -$09**——「bpl 面四环+环破点」逐字命中（0x60→0x40→0x20→0x00 四环+
BPL 环破=负径出）③L3 0 失败容忍不增（877/27 账内稳）④员面双扫桩 0（2 员真现）+
S1-00/S2-36 接面随证。测 231。findings（r10）：0 新。

## SC-S3-21 卡审（ce5a16b）——移除环抽核（bmi 回卷破点）

### verdict: pass

四条核：①L2 留白**真 0 三方核 ✓**（pregen/l2_cases 全 0）②**移除环抽核 ✓**：
$ED65=`a2 b0 bd 00 03 f0 08 bd 0b 03 d0 03`=LDX #$B0（X=$B0 十二环起）/LDA $0300,X/
BEQ/LDA $030B,X/BNE——判段合形；bmi 回卷破点（与 bpl 环异形注）随证——**环边界条
扩充**收讫（bpl 四环破点/bmi 回卷破点两形记档）③L3 0 失败容忍不增（877/27 账内
稳）④员面双扫桩 0（2 员真现）+S1-07/S2-36 接面。测 232。findings（r10）：0 新。

## SC-S3-22 卡审（1253d37）——rts 桩阵镜面四形抽核

### verdict: pass

四条核：①L2 留白**真 0 三方核 ✓**（pregen/l2_cases 全 0）②**rts 桩阵抽核 ✓✓**：
$95AB banks1/2/4/5 **镜面全同**=`60 ea ea 60 ea ea 60 ea`（**四 rts 桩 nop 填充
空转面**逐字）；bank3 异（`4c 15 a3` 代码流）=**镜面组精确**（1/2/4/5 非 3）③尾面
三现注（$D3BA/$D3BD/$D3C0 同构尾面谱系）随证 ④L3 0 失败容忍不增（877/27 账内
稳）⑤员面双扫桩 0（2 员真现）。测 233。findings（r10）：0 新。

## SC-S3-23 卡审（eb0e62a）——TourianMusic 声旗五现抽核

### verdict: pass

四条核：①L2 面 s3-083-case×8 并入+**族 8 案 0 败（首提全过）**✓（增量 877→885=+8
精确；账内 27 稳）②**声旗五现抽核 ✓✓**：$CC07=`a9 40 a2 05 d0 db`=LDA #$40/LDX #$05
——「$0685←$40/A=$40/X=5」逐字命中；**SFX 族五部曲**（$20/$04/$02/$01/$40——
声旗恒定谱系五部收官）③L3 0 失败容忍不增 ④员面双扫桩 0（2 员真现）。测 234
（族域述 90 测里程碑）。findings（r10）：0 新。

## SC-S3-24 卡审（99787af）——DrawFrame 分派段抽核（S1-22 walk 目标面）

### verdict: pass

四条核：①L2 面 s3-084-case×8（**6 strict+2 账内**）✓（增量 885→893=+8 精确；
账内 27→29=+2 算术合（case-5/6 深径 276 写跨卡））②**分派段抽核 ✓**：$DE4A=
`a6 4b bd 03 03 c9 f7 d0`=LDX $4B/LDA $0303,X/CMP #$F7 判段（S1-19 双 cmp 同构面）
——「S1-22 walk 目标面」分派合形；S1-17 修剪变体（19 写：定 15+依存 4）由 8 案
钉死 ③L3 0 失败容忍不增 ④员面双扫桩 0（2 员真现）。测 235。findings（r10）：0 新。

## SC-S3-25 卡审（f916b7d）——sbc 借位面抽核

### verdict: pass

四条核：①L2 面 s3-085-case×8 并入+**族 8 案 0 败**✓（增量 893→901=+8 精确；
账内 29 稳）②**sbc 借位面抽核 ✓✓**：$D48F=`20 b4 d4 8a e9 20 aa`=JSR $D4B4/TXA/
**SBC #$20**/TAX/BPL 进位步进环——「入参 P 进位⇒步 0x20/0x21」双径验算合
（57→25（C=1）/57→24（C=0 借位多减）=**入参 C 依赖实证**）③L3 0 失败容忍不增
④员面双扫桩 0（2 员真现）。教训注收讫：sbc/adc 进位面入参 C 依赖（环步进自查条）。
测 236。findings（r10）：0 新。

## SC-S3-26 卡审（1dbf9f5）——$D4A8 环体正身抽核（谱系闭环）

### verdict: pass

四条核：①L2 留白**真 0 三方核 ✓**（pregen/l2_cases 全 0）②**环体正身抽核 ✓✓**：
$D4A8=`98 dd 2c 07 d0 05 a9 ff`=TYA/**CMP $072C,X**/BNE/LDA #$FF——「Y==$072C[x]
比对⇒$0728[x]←$FF」逐字命中（S1-00/S3-25 环正身 trace 归位）；$D49E=`20 a8 d4 8a
38 e9 08 aa`=JSR/TXA/**SEC/SBC #$08**/TAX——sbc 进位面同则（步 8/9）+bne 零断
破点（与 bpl/bmi 异形第三式）③L3 0 失败容忍不增（901/29 账内稳）④员面双扫桩 0
（2 员真现）。环破点三形谱系（bpl/bmi/bne）记档。测 237。findings（r10）：0 新。

## SC-S3-27 卡审（e2e2ff9）——自旋环抽核（lsr/bcs -7 回卷）

### verdict: pass

四条核：①L2 留白**真 0 三方核 ✓**（pregen/l2_cases 全 0）②**自旋环抽核 ✓✓**：
$D5FC=`bd 0b 03 4a b0 f9 a9 00`=LDA $030B,X/LSR/**BCS -$07 回卷自旋**/LDA #$00
——「$030B[x] lsr/bcs 自旋环（-7 回卷）」逐字命中（屏内判定自旋面）③L3 0 失败
容忍不增（901/29 账内稳）④员面双扫桩 0（2 员真现）。测 238。findings（r10）：0 新。

## SC-S3-28 卡审（9ae9ab6）——Table0D 二现抽核

### verdict: pass

四条核：①L2 留白**真 0 三方核 ✓**（pregen/l2_cases 全 0）②**判清形抽核 ✓**：
$D5DA=`bd 0a 03 f0 1c a9 00 9d`=LDA $030A,X/BEQ/**LDA #$00 清**/STA——「$030A[x]
判+清+双判段」逐字命中（Table0D 族二现；$D533 表对载指针面=S3-087 同构随证）
③L3 0 失败容忍不增（901/29 账内稳）④员面双扫桩 0（2 员真现）。测 239。
findings（r10）：0 新。

## S3 批产终审（S3-00→S3-89 全清）——批产第三批收官（三批 90 卡全胜）

### verdict: pass（S3 批正式收官=三批全胜）

**批终态账核 ✓**：①案族并入 **128 条**（s3-*-case 实计；lead 约 80=保守估——实值
128 更实，三批同型：S1 144/S2 160/S3 128）②L2 总案 901（S2 收官 773→901=S3 批
+128 案族面）+**账内 29 稳**（退出条件均在案）③勾格三分法全批兑现（strict=已验证/
真 0=已实现留白/全账内=有L2 降级注记）+首提 20+ 连+**方法入册五件**（SFX 声旗谱系/
ROM 表烘焙/镜面共享形/环破三形谱系/Store-Load 对偶谱系）。
测 240（族域述 96 测 0 败）。**三批 90 卡全胜**（S1 28 逐卡+S2 30 逐卡+S3 30 逐卡
=verdict 全 pass 零打回——批产制三连胜）。

## SC-S4-00 卡审（e9bd4ad）——S4 批首卡（预取证复用形）

### verdict: pass

四条核：①L2 留白**真 0 三方核 ✓**（pregen/l2_cases 全 0）②**指针面抽核 ✓✓**：
$D624=`a6 4b bd 09 03 85 05 bd`=LDX $4B/LDA $0309,X/**STA $05**/LDA…——「$05/$04
对」指针面逐字命中（S1-21 Store 接面+$FD8F 走带 S1-10 接面随证）③L3 0 失败容忍
不增（901/29 账内稳）④员面双扫桩 0（2 员真现）。预取证复用形（S4 开工前字节解）
方法入册。测 241。findings（r10）：0 新。

## SC-S4-01 卡审（22bd82c）——$D4F8 态机正身抽核（S4-90 尾跳目标归位）

### verdict: pass

四条核：①L2 留白**真 0 三方核 ✓**（pregen/l2_cases 全 0）②**态机正身抽核 ✓✓**：
$D4F8=`a6 4b 90 13 ad 78 68 29 04 d0`=LDX $4B/BCC cc 段/LDA $6878/**AND #$04**（位 2
判）/BNE——「$4B X 载+cc/$6878 位 2 判+$030F[x] dec 零清段」逐字命中（S4-90 尾跳
目标正身归位=预取证溯源闭环）③L3 0 失败容忍不增（901/29 账内稳）④员面双扫桩 0
（2 员真现）。正身归档二例+表数据区三现注随证。测 242。findings（r10）：0 新。

## SC-S4-02 卡审（a98a2ee）——$D4EB 链入口抽核（三接面链）

### verdict: pass

四条核：①L2 留白**真 0 三方核 ✓**（pregen/l2_cases 全 0）②**链入口抽核 ✓✓**：
$D4EB=`a9 01 85 71 20 fc d5 20 da d5`=LDA #$01/**STA $71**/JSR $D5FC（S3-087 接面）
/JSR $D5DA（S3-088 接面）——「$71←1+三接面链」逐字命中（S3-087/088/S1-10 链
实存）③L3 0 失败容忍不增（901/29 账内稳）④员面双扫桩 0（2 员真现）。测 243。
findings（r10）：0 新。

## SC-S4-03 卡审（9a7eeba）——AnimDraw 接面族抽核（百测里程碑）

### verdict: pass

四条核：①L2 留白**真 0 三方核 ✓**（pregen/l2_cases 全 0）②**接面族抽核 ✓✓**：
$D517=`a9 01 20 47 de c6 71 60 fe 00`=LDA #$01/**JSR $DE47（AnimDrawObject S1-19
接面）**/**DEC $71**/RTS/INC（双 inc 尾起）——逐字命中（S3-089 同构尾段+Table0D
形 S3-088 同构随证）③L3 0 失败容忍不增（901/29 账内稳）④员面双扫桩 0（2 员
真现）。测 244（族域述**百测里程碑**）。findings（r10）：0 新。

## SC-S4-04 卡审（7bb2696）——IsWalkableTile XOR 掩码抽核（实值先核形）

### verdict: pass

四条核：①L2 面 s4-094-case×8 并入（**pregen 实值 8/8 先核 ✓**=S1-00 教训条形
正确执行——有案簇实值非留白想当然）+**族 8 案 0 败（首提全过）**✓（增量 901→909=+8
精确；账内 29 稳）②**XOR 掩码抽核 ✓✓**：$E81E=`a6 71 f0 59 a2 06 a5 05 55 5d`=
LDX $71/BEQ/LDX #$06/LDA $05/**EOR $5D,X**——「XOR 掩码双段可走判」逐字命中
③L3 0 失败容忍不增 ④员面双扫桩 0（2 员真现）。测 245。findings（r10）：0 新。

## SC-S4-05 卡审（2d3ee3e）——镜面五形抽核（结构镜面+jmp bank 特定）

### verdict: pass

四条核：①L2 留白**真 0 三方核 ✓**（pregen/l2_cases 全 0）②**镜面五形抽核 ✓✓**
（结构级镜面精确）：$95C0 banks1/2/4/5=`60 ea ea 4c xx xx 49 ff 18 69`——rts 桩+
**jmp bank 特定**（$9D35/$9B9D/$9C49/$9B48 各异）+取负对（EOR #$FF/CLC/ADC=
S2-42 尾形内联）四 bank **结构全同**（字节非全同=申报「jmp bank 特定」精确——
首核字节全同测试过严→结构测试自纠）③L3 0 失败容忍不增（909/29 账内稳）④员面
双扫桩 0（2 员真现）。测 246。findings（r10）：0 新。

## SC-S4-06 卡审（f2eb5fc）——$E9BE 检查链抽核（实值先核形）

### verdict: pass

四条核：①L2 面 s4-096-case×8 并入（**pregen 实值 8/8 先核 ✓**）+**族 8 案 0 败
（首提全过）**✓（增量 909→917=+8 精确；账内 29 稳）②**检查链抽核 ✓**：$E9BE=
`a4 71 f0 42 a8 20 bd 95 c0`=LDY $71（$71 判）/BEQ/TAY/JSR $95BD（$95C0 镜面域
接面）——「$71 判+cmp $98 段」结构合形 ③L3 0 失败容忍不增 ④员面双扫桩 0（2 员
真现）。测 247。findings（r10）：0 新。

## SC-S4-07 卡审（6758d8c）——SFX_BombExplode 声旗六现抽核

### verdict: pass

四条核：①L2 面 s4-097-case×8 并入（pregen 实值 8/8 先核 ✓）+**族 8 案 0 败
（首提全过）**✓（增量 917→925=+8 精确；账内 29 稳）②**声旗六现抽核 ✓✓**：$CB9A=
`a9 10 d0 02 a9 20 a2 00`=LDA #$10/LDX #$00——「$0680←$10/A=$10/X=0」逐字命中；
**SFX 族六部曲**（$20/$04/$02/$01/$40/$10——声旗恒定谱系六部）③L3 0 失败容忍
不增 ④员面双扫桩 0（2 员真现）。测 248。findings（r10）：0 新。

## SC-S4-08 卡审（d26149e）——IsBlastTile 同族面抽核（$E9BE 同族）

### verdict: pass

四条核：①L2 留白**真 0 三方核 ✓**（pregen/l2_cases 全 0）②**同族面抽核 ✓✓**：
$E9C2=`a8 20 bd 95 c0 98 b0 39`=TAY（Y=A）/JSR $95BD/TYA/BCS——与 S4-06 $E9BE
**同流共族逐字**（$E9C2=$E9BE 流+4 内 TAY 起点=同族双入口形）③L3 0 失败容忍不增
（925/29 账内稳）④员面双扫桩 0（2 员真现）。测 249。findings（r10）：0 新。

## SC-S4-09 卡审（2df980c）——$E6A7 态步进抽核（实值先核形）

### verdict: pass

四条核：①L2 面 s4-099-case×8 并入（**pregen 实值 8/8 先核 ✓**）+**族 8 案 0 败
（首提全过）**✓（增量 925→933=+8 精确；账内 29 稳）②**态步进抽核 ✓✓**：$E6A7=
`a5 49 c9 02 f0 0c c9 03 d0 1f`=LDA $49/**CMP #$02/BEQ/CMP #$03**/BNE——
「$49 cmp 2/3 双判+dec 面」逐字命中（S1-22/S2-36 同族）③L3 0 失败容忍不增 ④员面
双扫桩 0（2 员真现）。测 250。findings（r10）：0 新。

## SC-S4-10 卡审（8f46661）——$E6D2 inc 对偶抽核

### verdict: pass

四条核：①L2 面 s4-100-case×8 并入（**pregen 实值 8/8 先核 ✓**）+**族 8 案 0 败
（首提全过）**✓（增量 933→941=+8 精确；账内 29 稳）②**inc 对偶抽核 ✓✓**：$E6D2=
`a5 49 c9 03 f0 0c c9 02 d0 21`=LDA $49/**CMP #$03**/BEQ/**CMP #$02**/BNE——与
S4-09 $E6A7（`c9 02…c9 03`）**双判互换完美对偶**（inc 版/dec 版=态步进对偶谱系）
③L3 0 失败容忍不增 ④员面双扫桩 0（2 员真现）。测 251。findings（r10）：0 新。

## SC-S4-11 卡审（4c53333）——动画计数抽核+角色契约核

### verdict: pass

四条核：①L2 面 s4-101-case×8 并入（**pregen 实值 8/8 先核 ✓**）+**族 8 案 0 败
（首提全过）**✓（增量 941→949=+8 精确；账内 29 稳）②**动画计数抽核 ✓**：$D851=
`4c 66 d8 fe 0d 03 bd 0d 03 c9`=JMP $D866/**INC $030D,X**/LDA $030D,X/CMP——
「inc+回卷（$D648 S2-47 接面）+cmp $83 段」结构合形 ③**角色契约核 ✓**（透传类
A=0/X/Y 由 8 案实核钉死=$D866 写对 S1-12 同构面随证）④员面双扫桩 0（2 员真现）。
测 252。findings（r10）：0 新。

## SC-S4-12 卡审（5b6bc48）——IsEngineRunning 双判抽核

### verdict: pass

四条核：①L2 面 s4-102-case×8 并入（**pregen 实值 8/8 先核 ✓**）+**族 8 案 0 败
（首提全过）**✓（增量 949→957=+8 精确；账内 29 稳）②**引擎判抽核 ✓✓**：$CA18=
`a4 1e c0 07 f0 04 c0 03 f0 01`=LDY $1E/**CPY #$07/BEQ/CPY #$03**/BEQ——
「$1E MainRoutine 双判+表载」逐字命中（透传类 A/X/Y=0 契约 8 案钉）③L3 0 失败
容忍不增 ④员面双扫桩 0（2 员真现）。测 253。findings（r10）：0 新。

## SC-S4-13 卡审（74eafbe）——WaitNMIPass 自旋等待抽核

### verdict: pass

四条核：①L2 留白**真 0 三方核 ✓**（pregen/l2_cases 全 0）②**自旋等待抽核 ✓✓**：
$C43F=`20 34 c4 a5 1a f0 fc 60`=JSR $C434（ClearNMIStat）/LDA $1A/**BEQ -$04 自旋
等待**/RTS——「NMI 传递面」逐字命中 ③L3 0 失败容忍不增（957/29 账内稳）④员面
双扫桩 0（2 员真现）。测 254。findings（r10）：0 新。

## SC-S4-14 卡审（d176f34）——SelectSamusPal 位链/rol 进位抽核

### verdict: pass

四条核：①L2 面 s4-104-case×8 并入（**pregen 实值 8/8 先核 ✓**）+**族 8 案 0 败
（首提全过）**✓（增量 957→965=+8 精确；账内 29 稳）②**位链/rol 抽核 ✓**：$CB73=
`98 48 ad 78 68 0a 0a 0a ad 0e`=TYA/PHA（A=Y 透传）/LDA $6878/**ASL×3 位链**/LDA
$0E——「位链+rol 进位面（S1-10 同构）+adc 偏移」结构合形（恒定 $1C←2 8 案钉）
③L3 0 失败容忍不增 ④员面双扫桩 0（2 员真现）。测 255。findings（r10）：0 新。
注：SC-S4-15=半程卡预告随账。

## SC-S4-15 卡审（723b816）——ScreenOn PPU 写对抽核（半程卡）

### verdict: pass

四条核：①L2 留白**真 0 三方核 ✓**（pregen/l2_cases 全 0）②**PPU 写对抽核 ✓**：
$C447=`a5 fe 09 1e d0 f0 a5 ff`=LDA $FE/**ORA #$1E**（屏开位）/BNE/LDA $FF——
「PPU 写对+屏开面」结构合形 ③L3 0 失败容忍不增（965/29 账内稳）④员面双扫桩 0
（2 员真现）。测 256。findings（r10）：0 新。S4 批半程记档（15/30 卡过审）。

## SC-S4-16 卡审（5f06b5d）——表拷贝 bpl 环抽核（百形谱里程碑）

### verdict: pass

四条核：①L2 面 s4-106-case×8 并入（**pregen 实值 8/8 先核 ✓**）+**族 8 案 0 败
（首提全过）**✓（增量 965→973=+8 精确；账内 29 稳）②**表拷贝环抽核 ✓✓**：$C8B0=
`a2 0d bd 9a 95 95 3b ca 10 f8`=LDX #$0D/**LDA $959A,X/STA $3B,X**/DEX/**BPL 环**
——「14 项烘焙（SFX_REG_INIT_TBL）→$3B+x 块」逐字命中；$959A 表源实核（`73 01`）
=ROM 表烘焙法复用 ③L3 0 失败容忍不增 ④员面双扫桩 0（2 员真现）。
**共根形一百=百形谱里程碑记档**。测 257。findings（r10）：0 新。

## SC-S4-17 卡审（86ae4b5）——$C8BB 清零环抽核（X 环双清）

### verdict: pass

四条核：①L2 留白**真 0 三方核 ✓**（pregen/l2_cases 全 0）②**清零环抽核 ✓✓**：
$C8BB=`a9 00 aa e0 48 b0 02 95 97 9d`=LDA #$00/**TAX（X=$00 起）/CPX #$48（$47
界判）**/BCS/**STA $97,X（$97[x] 清）**/STA（$6AF4[x] 清）——「X 环双清（$00→$47）」
逐字命中（jmp $95AB=S3-22 接面随证）③L3 0 失败容忍不增（973/29 账内稳）④员面
双扫桩 0（2 员真现）。测 258。findings（r10）：0 新。

## SC-S4-18 卡审（013d686）——ToggleScroll 双 XOR 切换抽核（XOR 族谱三形）

### verdict: pass

四条核：①L2 面 s4-108-case×8 并入（**pregen 实值 8/8 先核 ✓**）+**族 8 案 0 败
（首提全过）**✓（增量 973→981=+8 精确；账内 29 稳）②**双 XOR 抽核 ✓✓**：$E252=
`a5 49 49 03 85 49 a5 fa 49 08`=LDA $49/**EOR #$03**/STA $49/LDA $FA/**EOR #$08**
——「$49←$49^3/$FA←$FA^8 双 XOR 切换」逐字命中（XOR 族谱三形：ToggleSamusHi
（S1-11）/索引版（S2-17）/ToggleScroll（本卡）=对偶谱系）③L3 0 失败容忍不增 ④员面
双扫桩 0（2 员真现）。测 259。findings（r10）：0 新。

## SC-S4-19 卡审（8ea2665）——step 分派抽核（bpl/bmi 对偶分派）

### verdict: pass

四条核：①L2 留白**真 0 三方核 ✓**（pregen/l2_cases 全 0）②**step 分派抽核 ✓✓**：
$D892=`bd 0f 03 10 06 20 f1 e4 4c 06`=LDA $030F,X（符号段）/**BPL +6（⇒$E519
step）**/**JSR $E4F1（⇒stepback）**/JMP $D806——「bpl⇒$E519/bmi⇒$E4F1 对偶
分派」逐字命中（S1-09/11 对偶谱系=S2-11 $E4F1 DEC 族归位）③L3 0 失败容忍不增
（981/29 账内稳）④员面双扫桩 0（2 员真现）。测 260。findings（r10）：0 新。

## SC-S4-20 卡审（dd64830）——EnemyCheckMoveVertical 减数面/步长抽核

### verdict: pass

四条核：①L2 面 s4-110-case×8 并入（**pregen 实值 8/8 先核 ✓**）+**族 8 案 0 败
（首提全过）**✓（增量 981→989=+8 精确；账内 29 稳）②**减数面/步长抽核 ✓✓**：
$E783=`85 02 a9 08 85 04 20 92`=STA $02（$02←A 减数面）/LDA #$08/STA $04（$04←8
步长）/JSR——逐字命中 ③L3 0 失败容忍不增 ④员面双扫桩 0（2 员真现）。测 261。
findings（r10）：0 新。S4 余 9 卡预告随账。

## SC-S4-21 卡审（0dc99a1）——CheckMoveDown 抽核（inx 补偿）

### verdict: pass

四条核：①L2 留白**真 0 三方核 ✓**（pregen/l2_cases 全 0）②**CheckMoveDown 抽核
✓**：$E7BD=`d0 02 38 60 85 03 a8 a2 00`=BNE/**SEC/RTS（beq⇒退出段）**/STA $03/
TAY/**LDX #$00（inx 补偿起）**——「A 判+delta 低位+inx 补偿」结构合形 ③L3 0
失败容忍不增（989/29 账内稳）④员面双扫桩 0（2 员真现）。测 262。findings（r10）：
0 新。S4 余 8 卡预告随账。

## SC-S4-22 卡审（2c37cef）——横移分支抽核（S4-110 同构对）

### verdict: pass

四条核：①L2 面 s4-112-case×8 并入（**pregen 实值 8/8 先核 ✓**）+**族 8 案 0 败
（首提全过）**✓（增量 989→997=+8 精确；账内 29 稳）②**同构对抽核 ✓**：$E904=
`85 03 20 92 e7`=STA $03（$03←A 横移面）/JSR $E792（S2-38 接面）——与 S4-110
$E783（`85 02`=$02 竖移面）**同构对**（域字段 $02/$03 对偶+调链同构）逐字命中
③L3 0 失败容忍不增 ④员面双扫桩 0（2 员真现）。测 263。findings（r10）：0 新。
S4 余 7 卡预告随账。

## SC-S4-23 卡审（8584a52）——CheckMoveRight 同构对抽核（120 测里程碑）

### verdict: pass

四条核：①L2 留白**真 0 三方核 ✓**（pregen/l2_cases 全 0）②**同构对抽核 ✓✓**：
$E89B=`d0 02 38 60 84 02 a2 00` vs S4-111 $E7BD=`d0 02 38 60 85 03 a8 a2`——
**同首式骨架**（BNE +2/SEC/RTS 判退出段）+域字段对偶（STY $02 vs STA $03）+inx
补偿同构=「CheckMoveRight/CheckMoveDown 同构对」逐字命中 ③L3 0 失败容忍不增
（997/29 账内稳）④员面双扫桩 0（2 员真现）。测 264（族域述**120 测里程碑**）。
findings（r10）：0 新。S4 余 6 卡预告随账。

## SC-S4-24 卡审（abcb5ff）——walk handler 正身第四例（S1-22 归位）

### verdict: pass

四条核：①L2 留白**真 0 三方核 ✓**（pregen/l2_cases 全 0）②**walk handler 正身
抽核 ✓✓**：$EB06=`20 0c eb 4c d4 ea`=JSR $EB0C/**JMP $EAD4 回驱**（walk 表 op1
handler 归位）+$EB0C=`b1 00 29 f0 aa 20 7a eb`=LDA ($00),Y/**AND #$F0 类型掩码**/
TAX/JSR $EB7A——「GetEnemyData 掩码+判+数据载+指针面」逐字命中；**正身模式第四例**
（S1-22 trace op1→本卡员面归位）③L3 0 失败容忍不增（997/29 账内稳）④员面双扫
桩 0（2 员真现）。测 265。findings（r10）：0 新。S4 余 5 卡收官段预告随账。

## SC-S4-25 卡审（9ed2805）——IsSlotTaken 占用判抽核

### verdict: pass

四条核：①L2 留白**真 0 三方核 ✓**（pregen/l2_cases 全 0）②**占用判抽核 ✓✓**：
$EB7A=`bd f4 6a f0 05 bd 05 04 29 02`=LDA $6AF4,X/BEQ（⇒取负段）/LDA $0405,X/
**AND #$02 占用判**——逐字命中（S1-10 镜面尾面随证）③L3 0 失败容忍不增（997/29
账内稳）④员面双扫桩 0（2 员真现）。L3 elevator-ride 腿态注：空帧 fail-loud 正确
守住（lead 裁定①②③执行毕）——闸解待 decomp 补 dump。测 266。findings（r10）：
0 新。S4 余 4 卡收官预告随账。

## SC-S4-26 卡审（b80ff5e）——GetEnemyType 类型高位判抽核

### verdict: pass

四条核：①L2 留白**真 0 三方核 ✓**（pregen/l2_cases 全 0）②**高位判抽核 ✓✓**：
$EB28=`48 29 c0 9d 0f 04 0a 10 15`=PHA/**AND #$C0**（类型高位掩）/**STA $040F,X**
（高位写）/ASL（符号段）/BPL——「A and $C0→$040F[x]+asl 符号段」逐字命中 ③L3 0
失败容忍不增（997/29 账内稳）④员面双扫桩 0（2 员真现）。测 267。findings（r10）：
0 新。S4 余 3 卡收官预告随账。

## SC-S4-27 卡审（9803b33）——敌初始化抽核（态写对/乘表/激活面）

### verdict: pass

四条核：①L2 留白**真 0 三方核 ✓**（pregen/l2_cases 全 0）②**敌初始化抽核 ✓✓**：
$EB4D=`a8 29 f0 09 08 9d 00 04 98 20 c5 c2`=TAY/**AND #$F0/ORA #$08/STA $0400,X**
（态写 1）/TYA/**JSR $C2C5（乘表 Amul16）**——逐字命中（or $0C→$0401[x] 态写 2+
$6AF4[x]←1 激活面随证）③L3 0 失败容忍不增（997/29 账内稳）④员面双扫桩 0
（2 员真现）。测 268。findings（r10）：0 新。S4 余 2 卡=收官在望预告随账。

## SC-S4-28 卡审（feb9f4a）——InitEnemyFacing 朝向初始化抽核

### verdict: pass

四条核：①L2 留白**真 0 三方核 ✓**（pregen/l2_cases 全 0）②**朝向初始化抽核 ✓✓**：
$FB7B=`20 b0 80 7e 05 04 b9 bb 96 9d`=JSR $80B0（随机 S1-02 接面）/**ROR $0405,X
（朝向）**/LDA $96BB,Y（延迟面）——逐字命中 ③L3 0 失败容忍不增（997/29 账内稳）
④员面双扫桩 0（2 员真现）。测 269。findings（r10）：0 新。**S4 收官预告随账**
（SC-S4-29=S4-119 收官卡，S4 终审同 S1/S2/S3 形）。

## S4 批产终审（S4-00→S4-119 全清）——批产第四批收官（四批 120 卡零打回）

### verdict: pass（S4 批正式收官=四批全胜零打回）

**批终态账核 ✓**：①案族并入 **104 条**（s4-*-case 实计；lead 约 72=保守估——四批
同型：S1 144/S2 160/S3 128/S4 104，实值总 536 条案族面）②L2 总案 **1005**（S3 收官
901→1005=S4 批 +104 案族面）+**账内 29 稳**（退出条件均在案）③勾格三分法全批兑现
（strict=已验证/真 0=已实现留白/全账内=有L2 降级注记）+首提 25+ 连+**方法入册六件**
（SFX 声旗谱系/ROM 表烘焙/镜面结构形/环破三形谱系/Store-Load 对偶谱系/预取证复用形）。
测 270（族域述 126 测 0 败）。
**四批 120 卡零打回达成**（S1 28 逐卡+S2 30+S3 30+S4 30=verdict 全 pass 零打回——
批产制四连胜，1 次 needs_revision=S1-00 首卡留白条款误用即时整改闭环）。

## SC-S5-00 卡审（ee2ee64）——S5 批首卡（预取证复用形·item spawn 链）

### verdict: pass

四条核：①L2 留白**真 0 三方核 ✓**（pregen/l2_cases 全 0）②**指针面对抽核 ✓✓**：
$EE41=`85 07 a5 4f 85 06 4c 67 dc`=STA $07（$07←A）/LDA $4F/STA $06（$06←$4F）
/**JMP $DC67（S1-25 接面）**——逐字命中（S2-10/S1-00 多接面随证）③L3 0 失败容忍
不增（1005/29 账内稳）④员面双扫桩 0（2 员真现）。预取证复用形（S5 开工前字节解）
=方法入册续用。测 271。findings（r10）：0 新。

## SC-S5-01 卡审（cbaa1ef）——跨卡溯源链/记账语义核（1 strict+7 账内）

### verdict: pass

四条核：①L2 面 s5-121-case×8（**1 strict+7 账内**）✓（增量 1005→1013=+8 精确；
账内 29→36=**+7 算术合**）②**记账语义核 ✓**：S2-048 先例形记账（gap_map
CROSSCARD-WALKVM：$EDD6 分派→handler 集∈S2-48/49 族——判定形② VM 深件 trace
溯源=walk-VM 深链归因精确）③溯源链 ✓：$EE3D=`85 09 a5 50 85 07`=STA $09/LDA $50/
STA $07 指针面合形（walk 环三现+$C45C 分派+四 x 写面全解随证）④员面双扫桩 0
（2 员真现）。**教训注收讫**：RAM 窗回卷（x 索引写越 $07FF 回卷——写址归一化自查
条）+offset 换算自查。测 271。findings（r10）：0 新。

## SC-S5-02 卡审（5906465）——位打包/勘误注/账目更新三面核

### verdict: pass

四条核：①L2 面 s5-122-case×8 并入+**族 8 案 0 败**✓（增量 1013→1021=+8 精确；
账内 36 稳）②**位打包抽核 ✓✓**：$DC67（0x1DC67 正径）=`a5 07 20 c4 c2 05 06 85
06 46`=LDA $07/**JSR $C2C4（Amul8）/ORA $06/STA $06**（$06←Amul8($07)|$06）/LSR
（>>3 起）——位打包语义逐字命中 ③**勘误注/账目更新核 ✓**：S5-01 账目退出条件更新
（勘误注随件——二次更正文化形）+offset 双误自纠（0x1DC67 正径）+测移位算误自纠
=TDD 纪律续 ④员面双扫桩 0（2 员真现）。测 272。findings（r10）：0 新。
SC-S5-03（pregen 有案实值先核）预告随账。

## SC-S5-03 卡审（2738182）——四因拆解/教训两则核

### verdict: pass

四条核：①L2 面 s5-123-case×8 并入（**pregen 实值 8/8 先核 ✓**）+**族 8 案 0 败
（首提全过）**✓（增量 1021→1029=+8 精确；账内 36 稳）②**四因拆解抽核 ✓**：
$EE86=`b5 b0 d0 16 8a 65 8a 29 7f`=LDA $B0,X（**占用判**）/BNE（**占用判跳**）/TXA/
ADC（**carry 分径**）/AND #$7F（**回卷掩**）+$EE6B=`38 e9 08`（**SEC/SBC 归一化
定步**）——四因（zp 寻址回卷/sec 归一化/carry 分径/占用判跳）字节面全现 ③L3 0
失败容忍不增 ④员面双扫桩 0（2 员真现）。**教训两则收讫**：zp 寻址回卷辨识（&0xFF
非 &0x7FF——RAM 窗 vs zp 窗两回卷）+sec 归一化反例（进位面）。测 273。
findings（r10）：0 新。SC-S5-04（pregen 先核）预告随账。

## SC-S5-04 卡审（eade0e4）——$95B1 桩阵镜面六形抽核

### verdict: pass

四条核：①L2 留白**真 0 三方核 ✓**（pregen/l2_cases 全 0）②**镜面六形抽核 ✓**：
$95B1 banks1/2/4/5=`60 ea ea 60 ea ea 60 ea` 全同（rts 桩阵续面——areas_common
形六）+bank3 异（代码流）=镜面组精确 ③L3 0 失败容忍不增（1029/36 账内稳）④员面
双扫桩 0（2 员真现）+CannonHandler 多接面（S2-23/S2-10）随证。注释断行自纠随账。
测 274。findings（r10）：0 新。SC-S5-05（pregen 先核）预告随账。

## SC-S5-05 卡审（67ca457）——MotherBrainHandler 同阵偏移/指针面抽核

### verdict: pass

四条核：①L2 留白**真 0 三方核 ✓**（pregen/l2_cases 全 0）②**同阵偏移抽核 ✓**：
$95B4=$95B1 同阵偏移二现（areas_common 形六续——bank 侧映射形：banks1/2/4/5
桩阵/他 bank 代码=call site JSR $95B4 的 bank 特定语义）；壳形 ✓：$EEAE=
`20 b4 95 a9 38 85 07`=JSR $95B4/LDA #$38/**STA $07 指针面**——S2-10 接面随证
③L3 0 失败容忍不增（1029/36 账内稳）④员面双扫桩 0（2 员真现）。测 275。
findings（r10）：0 新。SC-S5-06（pregen 先核）预告随账。

## SC-S5-06 卡审（4487840）——ZeebetiteHandler ptr 算抽核

### verdict: pass

四条核：①L2 留白**真 0 三方核 ✓**（pregen/l2_cases 全 0）②**ptr 算抽核 ✓✓**：
$EECA=`20 b7 95 8a 4a 69 3c 85 07`=JSR $95B7（桩阵三现）/TXA/**LSR（X>>1）**/
**ADC #$3C**/STA $07——「X>>1+$3C→$07」逐字命中（S2-10 同构面随证）③L3 0 失败
容忍不增（1029/36 账内稳）④员面双扫桩 0（2 员真现）。tuple 语法自纠（S5-02 同型
再犯警）随账。测 276。findings（r10）：0 新。SC-S5-07（pregen 先核）预告随账。

## SC-S5-07 卡审（df74939）——桩阵四现/jmp bank 特定抽核

### verdict: pass

四条核：①L2 留白**真 0 三方核 ✓**（pregen/l2_cases 全 0）②**桩阵四现抽核 ✓**：
$95BA banks1/2/4/5=`60 ea ea` 续（形六续四现——末桩 jmp bank 特定随证）；壳形
✓✓：$EEEE=`20 ba 95 4c c6 ee`=JSR $95BA（桩阵入）/**JMP $EEC6（S2-10 root 三现）**
——逐字命中（S2-18/S2-23 接面+$49→$91 写对随证）③L3 0 失败容忍不增（1029/36
账内稳）④员面双扫桩 0（2 员真现）。测 277。findings（r10）：0 新。
SC-S5-08（pregen 有案实值先核）预告随账。

## SC-S5-08 卡审（e5b5e38）——位置对同构/分支径判别抽核

### verdict: pass

四条核：①L2 面 s5-128-case×8 并入（**pregen 实值 8/8 先核 ✓**）+**族 8 案 0 败**
✓（增量 1029→1037=+8 精确；账内 36 稳）②**位置对同构抽核 ✓✓**：$F152=
`bd 00 04 85 07 bd 01 04 85 09`=LDA $0400,X/STA $07/LDA $0401,X/STA $09——
「位置对」S1-21/S2-34 load 同构逐字命中 ③**分支径判别 ✓**（案 5 实核：r($6F)≠0
⇒$6E/$6F 清+Y=1）随证 ④员面双扫桩 0（2 员真现）。测 278。findings（r10）：0 新。
SC-S5-09（pregen 先核）预告随账。

## SC-S5-09 卡审（1ca5687）——CollisionReact 态写抽核

### verdict: pass

四条核：①L2 留白**真 0 三方核 ✓**（pregen/l2_cases 全 0）②**态写抽核 ✓**：
$F2CA=`b0 12 b9 00 03 9d 0e 04`=BCS/LDA $0300,Y/**STA $040E,X 态写**/JSR——
「态写+多接面」结构合形 ③L3 0 失败容忍不增（1037/36 账内稳）④员面双扫桩 0
（2 员真现）。x 索引**九犯自纠**（强制步持续）。测 279。findings（r10）：0 新。
SC-S5-10（pregen 有案实值先核）预告随账。

## SC-S5-10 卡审（a8d04e6）——CollisionReactWithSamus carry 分支径抽核

### verdict: pass

四条核：①L2 面 s5-130-case×8 并入（**pregen 实值 8/8 先核 ✓**）+**族 8 案 0 败**
✓（增量 1037→1045=+8 精确；账内 36 稳）②**carry 分支径抽核 ✓**：$F282=
`b0 fd 20 e8 f2 20 9c cd`=**BCS 分径**/JSR $F2E8（S1-07 Xminus16）/JSR $CD9C
（S2-24 接面）——「carry=0⇒附加写族+Y=1」结构合形（案 0/3 对读实核）；$F0E2 链形
随证 ③L3 0 失败容忍不增 ④员面双扫桩 0（2 员真现）。测 280。findings（r10）：0 新。
SC-S5-11（pregen 先核）预告随账。

## SC-S5-11 卡审（6255743）——slot active 双段/$0A/$0B 判别抽核

### verdict: pass

四条核：①L2 面 s5-131-case×8 并入（**pregen 实值 8/8 先核 ✓**）+**族 8 案 0 败**
✓（增量 1045→1053=+8 精确；账内 36 稳）②**双段抽核 ✓✓**：$F09F=`bd f4 6a f0 02
c9 03 f0 3d`=LDA $6AF4,X/**BEQ +2/CMP #$03/BEQ**——「beq/cmp 3 双段」逐字命中
③**$0A/$0B 判别 ✓**（案 7 实核：r($0C)≠0⇒1；案 0/7 对读）随证 ④员面双扫桩 0
（3 员真现）。测 282。findings（r10）：0 新。SC-S5-12（pregen 先核）预告随账。

## SC-S5-12 卡审（77669d4）——operand 辨识三现/基读面抽核

### verdict: pass

四条核：①L2 面 s5-132-case×8 并入（**pregen 实值 8/8 先核 ✓**）+**族 8 案 0 败**
✓（增量 1053→1061=+8 精确；账内 36 稳）②**基读/索引载抽核 ✓✓**：$F172=
`bd 0d 03 85 07 bd 0e 03 85 09`=LDA $030D,X/STA $07/LDA $030E,X/STA $09 对载
逐字命中 ③**operand 辨识三现**（eor $FF=zp 读 S2-07 教训再证）——诚实注：eor 形
在首窗外（$0B 面翻段），判据=案实核面（8 案钉）非我字节定位级 ④员面双扫桩 0
（2 员真现）。测 283。findings（r10）：0 新。SC-S5-13（pregen 先核）预告随账。

## SC-S5-13 卡审（8785bf7）——DistFromObj0ToEn1 delta 对抽核（140 测里程碑）

### verdict: pass

四条核：①L2 留白**真 0 三方核 ✓**（pregen/l2_cases 全 0）②**delta 对抽核 ✓✓**：
$F1B3=`bd 01 03 20 e7 f1 bd 02 03`=LDA $0301,X/JSR $F1E7/LDA $0302,X——
「$0301/$0302[x] delta 对」逐字命中（$6AF5/$6AF6[x] 方向双径随证）③L3 0 失败
容忍不增（1061/36 账内稳）④员面双扫桩 0（2 员真现）。测 284（族域述**140 测
里程碑**）。findings（r10）：0 新。SC-S5-14（pregen 先核）预告随账。

## SC-S5-14 卡审（7ebd928）——GetEnemyYSlotPosition Y 版对偶抽核

### verdict: pass

四条核：①L2 留白**真 0 三方核 ✓**（pregen/l2_cases 全 0）②**Y 版对偶抽核 ✓✓**：
$F162=`b9 00 04 85 06 b9 01 04`（LDA $0400,Y/STA $06/**b9=abs,Y**）vs S5-08
$F152=`bd 00 04 85 07`（**bd=abs,X**/STA $07）——索引器/目标域双换（X→Y、$07/$09
→$06/$08）**完美对偶**逐字命中 ③L3 0 失败容忍不增（1061/36 账内稳）④员面双扫
桩 0（2 员真现）。测 285。findings（r10）：0 新。SC-S5-15（pregen 先核）预告随账。

## SC-S5-15 卡审（76946c5）——EnProjectile bcs 段/栈保抽核

### verdict: pass

四条核：①L2 留白**真 0 三方核 ✓**（pregen/l2_cases 全 0）②**bcs 段/栈保抽核
✓✓**：$F2ED=`b0 21 20 df f2 98 48 20 9c cd`=**BCS +$21（+33 未撞跳）**/JSR $F2DF/
**TYA/PHA（栈保 Y）**/**JSR $CD9C（S2-24 接面）**——逐字命中 ③L3 0 失败容忍不增
（1061/36 账内稳）④员面双扫桩 0（2 员真现）。测 286。findings（r10）：0 新。
SC-S5-16（pregen 先核）预告随账。

## SC-S5-16 卡审（e46b836）——LDC82 三因定案抽核（账清进展）

### verdict: pass

四条核：①L2 面 s5-136-case×8 并入（**pregen 实值 8/8 先核 ✓**）+**族 8 案 0 败**
✓（增量 1061→1069=+8 精确）②**三因链形抽核 ✓**：$DC82=`20 72 f1 20 a7 f1 4c
fa`=JSR $F172（S5-128 位置族）/JSR $F1A7/**JMP $F1FA（S1-27 碰撞族）**——链合形
（三因 x 索引读/$09 索引/$03←2&r($49) 由 8 案钉死）③**账清进展记档**：账内 36→30
（-6=退出条件达成转 strict——账目生命周期首现收缩面）④员面双扫桩 0（2 员真现）。
测 287。findings（r10）：0 新。SC-S5-17（余 3 卡收官段）预告随账。

## SC-S5-17 卡审（8337ed6）——eor zp 三现抽核（operand 辨识窗内命中）

### verdict: pass

四条核：①L2 面 s5-137-case×8 并入（**pregen 实值 8/8 先核 ✓**）+**族 8 案 0 败**
✓（增量 1069→1077=+8 精确；账内 30 稳）②**eor zp 三现抽核 ✓✓（窗内命中）**：
$F193=`45 ff 29 01 85 0a 60`=**EOR $FF（`45` zp 形——非 `49` imm！operand 辨识
窗内实锤）**/AND #$01/STA $0A/RTS（A 出=面翻值）——「$0A←(A^r($FF))&1」逐字命中
③L3 0 失败容忍不增 ④员面双扫桩 0（2 员真现）。测 288。findings（r10）：0 新。
SC-S5-18（余 2 卡=S5 收官在望）预告随账。

## 遗留风险清单

1. task-8（F2CA 族九入口）批产滚动中（批 12 起）；task-7（Mesen2 动态收口）交付后接
   复审（重点：动态证据场景覆盖面 + trace 可复现性）。
2. A7 same-source 弱化持续；独立模型抽核逐步扩面（现 33 案）。
3. P1-F2 盲区补案已闭（6423ce5 判别力实测 + F11 回归）；残余=各固有盲点案如实记账。
4. 复审方方法面：突变矩阵必须先过基模证真前置门（本案两度自纠：假 11/11、m1 定义错）
   ——「测试器先过证伪」在测试器自身的第二/第三案。
