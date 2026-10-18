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

## 遗留风险清单

1. task-8（F2CA 族九入口）批产滚动中（批 12 起）；task-7（Mesen2 动态收口）交付后接
   复审（重点：动态证据场景覆盖面 + trace 可复现性）。
2. A7 same-source 弱化持续；独立模型抽核逐步扩面（现 33 案）。
3. P1-F2 盲区补案已闭（6423ce5 判别力实测 + F11 回归）；残余=各固有盲点案如实记账。
4. 复审方方法面：突变矩阵必须先过基模证真前置门（本案两度自纠：假 11/11、m1 定义错）
   ——「测试器先过证伪」在测试器自身的第二/第三案。
