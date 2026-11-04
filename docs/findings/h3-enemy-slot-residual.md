# H3 敌槽残差定谳——±2px X 位移相位差 + data05 位 6 双槽互换（asm 先行 A 级实录）

> 纪律：refs/m1disasm 逐行读证先行（bank:addr + ROM 字节锚）；实测锚=collide f800 窄域
> 取证档（emu/rust 双腿 .ram.hex 直比）。日期：2026-11-04。
> 任务卡背景：H3 敌槽残差卡（预存真差，r14 取证定谳在案）——F4 族 102-355px×12 格。

## 0. 实测锚（A 级——.ram.hex 双腿直读复核）

case-collide-probe f800（emu=analysis/traces/…_f00800_l240.ram.hex / rust=shots_rust/…）：

| 面 | emu | rust | 差 |
|---|---|---|---|
| Ens.4($0440) x | $BD(189) | $BF(191) | +2px |
| Ens.5($0450) x | $5D(93) | $5C(92) | −1px |
| Ens.4 data05 | $1A | $5A | 位 6 反相 |
| Ens.5 data05 | $5A | $1A | 位 6 反相 |
| 空槽 0-3 data05 | 00 40 00 40 | 40 00 40 00 | **全 6 槽反相** |
| FrameCount($2D) | $11 | $11 | **0 差**（FC 字段已对齐——7 采集点全同：95/0D/5D/AD/11/69/73） |
| OAM spr CC/CD/DC/DD attr$22 | 同 | 同 | 0 差（贴图面无差） |
| EnsExtra.type($6B34/$6B44) | 05/05 | 05/05 | 同（**柱顶敌类型钉死=Zoomer(05)**） |
| EnsExtra+4（animDelay 面） | 06/05 | 05/06 | 随位 6 门相位（下游） |

spawn 面（f420-f700 采集点双腿全同）：x=BC/5C（=LEB4D 公式 (nibble<<4)|$0C
（B→BC/5→5C））+data05=00+status=01——**±2px 非 spawn 放置差、非数据表烘焙**。

## 1. 敌 x 放置写源（asm A 级）

| 件 | 址 | 语义 |
|---|---|---|
| LoadEnemy | $EB06（prg7:8741） | jsr GetEnemyData→jmp EnemyLoop |
| GetEnemyData | $EB0C（prg7:8745） | 槽=首字节高半字节（and #$F0/tax）→IsSlotTaken（$EB7A）→type=次字节→GetEnemyType（$EB28）→pos=第 3 字节→LEB4D |
| **LEB4D** | $EB4D（prg7:8788） | y=(pos&$F0)\|$08→$0400+x；**x=Amul16(pos)\|$0C→$0401+x**；status←1；isHit←0；hi←GetNameTableAtScrollDir |
| GetEnemyType | $EB28（prg7:8758） | specialAttribs←etype&$C0（ROM `9d 0f 04`=**$040F+x**）；type←etype&$3F→**$6B02+x**（ROM `bc 02 6b`） |
| 运动写径 | LoadEnemyPositionFromTemp $FA49（prg7:11867） | x←Temp09_PositionX（ApplySpeedToPosition 后） |
| 1px 步进 | EnemyMoveOnePixelLeft/Right $84A7/$84FE（areas_common:1142/1214） | dec/inc $0401,x（每移动帧 ±1px）+名表换/边界径 |
| Amul16 | $C2C5（prg7:776） | 4×asl（×16 丢高位）——x=(pos&$0F)<<4\|$0C（**置位恒以 $C 尾**） |

ROM 字节锚（bank7 文件偏移 16+7×4000+(addr−C000)）：
`$EB4D: a8 29 f0 09 08 9d 00 04 98 20 c5 c2 09 0c 9d 01 04 a9 01 9d f4 6a a9 00 9d 04 04 …`

**结论**：spawn x=纯代码径（LEB4D 公式）——f420 双腿 BC/5C 0 差实证 rust 合形。
±2px **不是**放置差/不是数据表烘焙（转 game-data 卡的前提不成立）。

## 2. data05 位 6 写源（asm A 级）+ 双槽互换根因

**写点** UpdateEnemy_UpdateEnData05Bit6（$F3AA（prg7:10555））：
```
lda Ens.0.data05,x / asl / rol / tay   ; 位 6 暂存
txa / jsr Adiv16 / eor FrameCount / lsr ; carry=((X/16) xor FrameCount) bit0
tya / ror / ror / sta Ens.0.data05,x   ; 写回位 6
```
公式：**bit6=(slot xor FrameCount) bit0**（slot=X/16=槽号）。写点在 UpdateEnemy 头部
（CheckIfVisible 后、状态分发前）——每帧每槽一写（空槽也写；status 1/2 且不可见时
CheckIfVisible 双返跳过）。

**捕获相位（A 级）**：emu 采集 CPU PC=$C0D9=NMI 入口（captures.jsonl 实录）——主循环序
（$C0BC: GoMainRoutine→**inc FrameCount**→spin）⇒ 采集 FC 值=当帧逻辑读值+1。
f800 dump FC=$11 ⇒ 双腿逻辑读 FC=$10。

**全 6 槽反相实证（f800）**：emu 位 6=slot&1=（slot^$10）&1（raw 公式+逻辑 FC=10 ✓）；
rust 位 6=（slot^$11）&1（**+1 补偿**）。空槽 0-3 无 AI 干扰，纯公式输出即反相
——**rust `update_en_data05_bit6`（mainloop.rs）`(x>>4)^(FC+1)` 的 +1=互换唯一源**。

**+1 补偿的来历（证伪更正）**：goal-轮 24/57 期 emu/rust FC 字段错位 1（「金腿 FC=+1
恒差」）⇒ +1 把公式对到 emu。r14 FC 对齐落定后 7 采集点 FC 双腿全同（本档 §0）
⇒ **补偿失配=全槽反相**（任务卡「公式已对、互换=相位源差」的相位源即此）。

**rust gate 同源偏离**：CrawlerAIRoutine_ShouldCrawlerMove（$F852（prg7:11512））：
`txa/lsr×3/adc FrameCount/lsr` ⇒ gate=((X>>3)+bit2(X)+FC)>>1&3（bit2(X)=0（槽址
×16））。rust crawler_ai 现形 `((x>>3)+((x>>2)&1)+(FC−1))>>1&3`——**FC−1 补偿**
（goal-轮 43 形）同源反向过补。金方跳格自证：emu 槽 4 不动帧=dump FC 17/18/25/26
⇒ 逻辑 FC 16/17/24/25 ⇒ raw 公式 (8+16)>>1&3=0 ✓（四点全中）——**raw 公式（无 ∓1）
与金方自洽**。

## 3. ±2px X 差定性（运动相位差——瞬态）

- f420-f700：双腿 x/data05/status 全同（BC/5C、00、01）——spawn 后未激活（OAM 全
  F0=UpdateWorld 未跑/不可见双返）。
- f700→f800：激活窗（status 01→02、radius 00→08）——首段爬行即分叉：emu BD/5D
  （+1/+1 步）vs rust BF/5C（+3/0 步）。机理：位 6 门反相⇒移动帧互补+gate FC−1 相位
  ⇒ 首动帧/转角翻向时点差⇒净位移 ±2。
- f890/f900：双腿 x 收敛（C8/78 同）——**瞬态相位差非持续偏置**（转角定态自愈）。
- 位 6 反相持续（f890：emu 18/5A vs rust 58/1A）——与 x 收敛并存（30fps 门只定相位
  不定净步数）。

## 4. spawn 来源考证（柱顶敌——任务③）

- **类型钉死（A 级）**：EnsExtra.type=$6B02+x（ROM `bc 02 6b`）——扩展 dump
  （ram.hex 尾 0x80B=0x6AF4-0x6B73 覆域）f800 实读：槽 4/5 type=**05（Zoomer/Crawler）**
  ——与 choose_enemy_ai type 5→crawler_ai 分发合。任务前提「type 在 $6000 不在
  RAM dump」已由 dump 覆域扩展消解（无需另行取证）。
- **spawn 径（A 级）**：房流 enemy 节=`FD 51 05 25 41 05 2B FF`（game-data
  ROOM_STREAMS[**9**]）——slot5/4、etype=05、pos=25/2B ⇒ LEB4D：y=(0x20)|8=0x28 ✓
  x=(5/B<<4)|$0C=5C/BC ✓ 与 f420 双腿实测逐值同。**LoadEnemy 房数据径（非特殊
  spawn）**。
- **「Room00 enemy 节空」前提不成立**：起始房=ROOM_STREAMS[9]（非 [0]）——柱顶双
  Zoomer 即 9 号房 enemy 节双条目（张冠李戴勘误）。无需 Spawner/特殊生成径。
- 柱顶敌=起始房两柱顶 Zoomer（f800 截图实证：两柱顶橙色爬虫 tiles CC/CD/DC/DD）。

## 5. 修径（TDD 判据）

1. **位 6 公式去 +1**：update_en_data05_bit6 改 asm 合形 `((x>>4)^FC)&1`。
2. **gate 公式去 −1**：crawler_ai ShouldCrawlerMove 改 asm 合形
   `((x>>3)+((x>>2)&1)+FC)>>1&3`。
3. 锚测（红→绿）：case-collide-probe f800 双腿 0 差——Ens4.x=0xBD/Ens5.x=0x5D/
   data05=1A/5A（emu 实测值双腿零差）。
4. 判据：F4 族格收敛（collide f800/f890/f900、killenemy f800、leftwalk f800、
   leftwalk2 f1020、walkfcprobe f1020 等≤50px 或归零）+L4 match 只增+943/1383/445
   state 基线 0 差保持+全测绿+gates check。

Evidence: prg7_engine.asm $EB06/$EB0C/$EB28/$EB4D/$EB7A/$F345/$F351/$F3AA/$FA49/
$F852+areas_common.asm $84A7/$84FE+crawler.asm:1-195 逐行；ROM 字节锚（bank7 六址
hexdump）；双侧 .ram.hex 7 采集点逐槽直读；ROOM_STREAMS[9] enemy 节 hex。
Confidence: 95
