# morphball 轨迹域根修——顶头反应（E2C6-E2CB）缺失定谳（2026-11-05）

> 卡面：morphball f1300-1700 七格 20-24k px 残差（L4 92% 权重）首攻。
> 首差链在案（lead 派卡）：f1258 sX→f1264 sY 顶钳→f1266 SamusY→f1268+ X 漂→
> f1300 ScrollX→f1524 MapPosX 换房。本档=asm 先行读证+双侧逐帧钉点+修+域界。

## 1. asm 读证（refs/m1disasm 逐行——A 级）

### 1.1 受阻上行反应（总根）——SamusMoveVertically（07:E2A9，prg7_engine.asm:7060-7117）
逐像素上移环 @loop_up（:7072-7085）：
```asm
@loop_up:
    jsr MoveSamusUp           ; ($E457) 上移 1px（门帧 (y−ry)&7==0 才查撞——E457）
    bcs @endIf_B
        ;Samus blocked upwards. Divide her speed by 2 and set the MSB to reverse her direction of travel.
        sec                   ; ←:7078
        ror Samus.speedY      ; ←:7079 int=(int>>1)|0x80（进位 1 入 bit7）
        ror SamusSpeedSubPixelY ; ←:7081 sub=(sub>>1)|旧int bit0→bit7
        jmp SamusMoveHorizontally ; 出环（不清零！）
```
**语义定谳**：受阻上行=半速+MSB 反向弹跳（`sec/ror/ror` 双字节 17 位环移），**不写 0、
不清 AccelY**。清零径属落地面 @landingNoBounce（:7098-7101 `jsr StopVertMovement/
sty SamusAccelY`（Y=0 来自 D147 `ldy #$00`））——两径互斥。
（§12 旧猜想「上升速度下限钳 −2.00」在本窗不成立：emu 上升序列 FC→FD→FE→FF 自然衰减
穿零（§2 表）——1ca8b7c5 的 −2 钳方向按本窗数据证伪，其「持续升撞顶挂顶」现象真因=
受阻反应半速后仍为负速继续试移→逐次 ror→负速滞留（§3 三写实录）。

### 1.2 重力/上限钳——VertAccelerate（07:E37A，:7263-7320）
AccelY≠0（空中）跳过落起检；重力 `sub+=AccelY, int+=carry`；上限钳（moving up:
`lda #0/cmp sub/sbc int/cmp #6`⇒|速|≥6 钳 $FA；down: cmp #5 钳 $05）——**无 −2 下限钳**
（与 §1.1 勘误一致）。

### 1.3 横速钳（sX $FE.E0 保持面）——HorzAccelerate（07:E3E5，:7340-7395）
max=SamusHorzSpeedMax **半字节单位**（Amul16→sub 面/Adiv16→int 面）；加速
`sub+=AccelX（符号扩展）, int+=carry`；左行取负绝对比 `cpx Temp02/tya/sbc Temp03/
bcc Lx149`——**|速|≥|max| ⇒ 写回有符号 max**（Temp00/01 取负径）。max=$12=1.125px/f：
负向取反 (int=$FE,sub=$E0) = **emu 漂移保持值 $FE.E0 的写源**（钳位写回，非速度清零；
清零属地面停走径 CFB7/ClearHorzMvmntData）。rust int_x（integrators.rs:151）钳径同构
（[ix] CLAMP lim=FEE0 实录）——f1250-1271 双腿 sX 全同实证。

## 2. 双侧逐帧对表（A 级——emu frames.jsonl watch vs rust --state-out）

口径：emu=analysis/traces/case-morphball-probe.frames.jsonl（watch 0308/0309/0312/0313/
030D/030E/00FD——run.json scenario sha 自锚）；rust=rust_screenshot --state-out（zp/iram
全量）+DSH_SPRITE_TRACE。**索引校准：f=n+12**（DIAG_ITER≠场景帧——collide 案 +10 教训
逐案校准；本档 [ixa]/[hxi]/[f1fa]/[hit] 插桩 n 值均按此换算）。

f1250-1272 窗（sX.I/sX.F | sY.I/sY.F | Y X Scl）：
| f | emu | rust（修前） | 判 |
|---|---|---|---|
| 1250-1263 | FE.E0 ｜ FC.F0→FE.28 衰减 ｜ y 78→47 | 同左逐帧全同 | 0 差（含 sX 保持面） |
| **1264** | ｜ **FF.20** ｜ 47 226 | ｜ **00.00** ｜ 47 226 | **首异=sY 顶头写** |
| 1265 | ｜ FF.38 ｜ 47 | ｜ 00.18 ｜ 47 | 随动 |
| 1266-1268 | FF.A8/FF.C0/FF.EC ｜ 47 | 00.30→00.60 ｜ 48 落 | 弧线分叉 |
| 1271 | 落地（sY 清零）y=49 | **提前落地**+AccelY=0 y=49 | 落点同、时点差 3 帧 |
| 1272+ | sX.F 224→176→128（max=$18 翻） | 同翻但提前 4 帧 | 横漂相位差连锁 |

首异钉点=**f1264 Samus.speedY/SpeedSubPixelY 写点**（emu (FE,40)→(FF,20) vs rust 清零）。

## 3. 顶头反应三写逐位验算（emu 实录 vs E2C6-E2CB 公式）

| 帧 | 反应前（f 帧末+重力 $18） | sec/ror/ror 后 | emu 行 |
|---|---|---|---|
| f1264 | (FE,28)+18=(FE,40) | int=(FE>>1)\|80=**FF**，carry=FE&1=0，sub=(40>>1)=**20** | (255,32) ✓ |
| f1266 | (FF,38)+18=(FF,50) | int=(FF>>1)\|80=**FF**，carry=FF&1=1，sub=(50>>1)\|80=**A8** | (255,168) ✓ |
| f1268 | (FF,C0)+18=(FF,D8) | int=**FF**，carry=1，sub=(D8>>1)\|80=**EC** | (255,236) ✓ |

三写逐位全合 ⇒ 反应语义=sec/ror int/ror sub 定谳（f1265/67 无写=积分未出像素步，
与「受阻才写」一致）。负速滞留机理：每次反应半速仍负→下帧重力再试→逐次 ror→
f1269 积分进位转正自然落体（emu (0,4)）——非下限钳。

## 4. 连锁图（总根修后四层链消解链）

f1264 反应缺失（旧形 stop_vert_movement 清零——代码内 goal-r15 R1 deferred 缺口自登记）
→ 负速归零⇒悬停消失 → f1266 Y 分叉 → f1271 提前落地（落点 y=49 同、时点差 3 帧）
→ 落地径 AccelY←0⇒VertAccelerate 起落检⇒**HorzSpeedMax←$18**（E380）提前翻
→ f1272 横速钳界 1.125→1.5px/f⇒sX.F 224→176→128 相位差 ⇒ f1273+ X/Scl 漂
→ （修后 f1255-1278 七字段 0 差全通）

**f1279+ 再分叉（域界——非本卡域）**：f1278 rust 敌撞击退（[hit] n=1266 dx=10<12
dy=9<23 eB=(40,198) s=(49,208)→IsHit=0x30|0x08）而 emu 无击（InvDly=0）——根=**敌槽 B
自生成即异轨**（emu 恒 (63,200) vs rust (99,152)→巡逻，f1100 前已异；敌 A 双腿同轨至
f1270 只差后续 2px）⇒接触盒形参不同（hi/ery/erx attrs 面）⇒单腿触发击退。敌面=
goal-轮134 已登记对账缺口（case watch 注「敌槽 WRAM 面——goal 轮134 对账缺口补」），
属敌槽/生成域非 Samus 轨迹域——**如实停机报卡点，不混域强修**。

## 5. D2-1b 激活面排除（拾取授予径）

拾取窗双腿：emu f1521 gear 00→**10**（gr_MARUMARI）/pw0 04→FF；rust 同值 gear=10/pw0=FF
（授予径工作正常——卡 E D2-1b 修活实证）。**授予值双腿 0 差⇒非状态差源**；时点差
（rust 早授）=轨迹分叉之果（MapPosX 3 vs 1——f1524 换房项同源连锁），非独立根。

## 6. TDD 记录（红→绿）

| 测 | 靶 | 红因 | 绿证 |
|---|---|---|---|
| physics::up_block_applies_ror_bounce_not_clear | E2C6-E2CB 三写 | 旧形清零（0 vs FF） | ok |
| physics::down_block_still_lands_and_clears_gravity | 落地径守门（防过应用） | （先行绿——语义未动） | ok |
| tests/morphball_traj_anchor | f1255-1278 七字段逐帧 0 差锚 | f1264 (0,0) vs (255,32) | ok（修后全通） |

（任务书件 3 的 f1524 MapPosX 锚被 §4 域界阻塞——敌面击退连锁未消前该锚不可达，
如实登记为卡点不伪锚。）

## 7. 可复读面

- emu：analysis/traces/case-morphball-probe.frames.jsonl（watch 0308/0309/0312/0313/
  030D/030E/00FD/0440/0450——cases/case-morphball-probe.txt 已钉，run.json sha 自锚）。
- rust：`DSH_SPRITE_TRACE=1 cargo run -q -p game-core --bin rust_screenshot -- --schedule
  shots_rust/case-morphball-probe.sched --out <dir> --state-out <jsonl>`（[inty]/[ixa]/
  [hxi]/[f1fa]/[hit] 插桩均在源内 n 窗口=n≥1258-1275+1260-1270）。
- 锚测：`cargo test -p game-core --test morphball_traj_anchor`（金值=emu 实录压缩内嵌
  f1255-1278 七字段）。

## 8. 更正链（证伪更正）

- 【2026-11-05 更正】morphball-probe-route.md §12「上升速度下限钳 −2.00 缺失」方向：
  本窗 emu 上升速度自然衰减穿零（FC→FE→FF→00 逐帧实录）——**−2 下限钳不存在**；
  「持续升至撞顶+挂顶」真因=受阻反应 `sec/ror/ror` 半速负速滞留（§3）。§12 的 asm
  候选位置未定项就此收口。
- 【2026-11-05 更正】lead 派卡链「f1258 sX $FE.E0 缺失」：现行树 f1250-1271 sX 双腿
  全同（int_x 钳径已同构——d568bf0c 修向已被先前轮落地）；f1272+ sX 分叉=max 翻连锁
  （§4），归本档总根一并消。

Evidence: asm=prg7_engine.asm:7060-7117（E2A9 全谱）/7263-7320（E37A）/7340-7395（E3E5）/
7098-7101（@landingNoBounce）/3434-3455（CFC3——对表参考）逐行读证；实证=emu
case-morphball-probe.frames.jsonl f1250-1300 逐帧 watch+rust state 行（zp/iram）+
[inty]/[ixa]/[hxi]/[f1fa]/[hit] 插桩 trace（n↔f=n+12 校准）；三写验算（§3 表）逐位合；
TDD 三测红→绿实录；D2-1b 排除=gear/pw0 双腿 0 差实录；域界=敌槽 B f1100 前异轨实录。
Confidence: 93

## 9. 敌槽生成/位形卡（2026-11-05——goal-轮134 面补全）

### 9.1 首异钉点（双侧逐帧）
敌 B（$0440）f376-846 双腿逐帧全同（(40,188)→(63,200) 钻进 6+2 节律）；**首异=f847**：
emu y/闪位全冻结（(63,200,d05=0x18) 恒驻至 f1351、f1376 起闪位复现）vs rust 续钻
64→…→巡逻。f846 双腿同写 d05 0x5A→0x18（bit1 清）。**生成初值/巡逻径不差**（派卡链
「生成初值或巡逻径首帧即分叉」证伪——是着陆后停机径分叉）。

### 9.2 asm 读证（refs/m1disasm 逐行）
- **型表**：ChooseEnemyAIRoutine（prg1_brinstar.asm:156-170）——型 05=**Zoomer
  （CrawlerAIRoutine）**；L968B（:281）型5=%00000100（bit4=0——免伤门不适用）。
- **行为壳**（enemies/crawler.asm:1-73）：ShouldCrawlerMove 节律门（**F852**：((slot>>3)
  +FC)>>1 &3==0 不移——slot=0x40⇒FC%8∈{0,1} 跳＝6+2 节律逐帧合）→status 检
  （EnemyStatusPreAI==Resting⇒出）→朝向（data0A&3==1 壁下行）→熔岩转（y==$E4）→
  @move（JumpByRTSToMovementRoutine 逐像素）→内角检（失败：$00≠0⇒Flip data05^1/
  $00==0⇒data0A−1 重定向）→外角检（orientation+1 探步）→anim。**任何运行帧必有写**
  （Flip/重定向/anim）——emu f847+ 零写⇒**AI 未运行**。
- **帧驱动**（prg7_engine.asm:10488-10530）：UpdateEnemy（F351）序=status∈{1,2}⇒
  **CheckIfVisible（F37F）**（data05 bit1 闩=1 免检；否则 IsObjectVisible——不可见⇒
  **pla/pla 双返回**（闪位写 F3AA 亦免——emu 闪位冻结实录合））→F3AA 闪位
  （bit6=(slot/16^FC)&1）→status dispatch。bit1 闩写面=绘径 `and #$FD/ora(flag<<1)`
  （goal-轮37 注）——f846 离屏绘写 bit1=0 闩开启 ✓ 双腿同。
- **判盒**（F1FA+F140）：半径和（EnsExtra+Objects）+hi 距——**双侧 hi=`(hi^PPUCTRL_ZP)&1`
  后等值比较**（F17F/F193）；F282 状态门=`cmp #enemyStatus_Frozen(4)/bcs Exit17`
  （status≥4 免伤）+L968B bit4 型门（型5 不触发）。

### 9.3 三根定谳（修）
| 根 | 症 | 修 |
|---|---|---|
| **D1 更新门状态域** | 驱动 `status<2` 只护 Resting——**Active(2) 漏门**⇒f847 离屏仍跑 AI | `status<3`（asm F351 域=1\|2） |
| **D2 判盒 hi 源** | `s_hi` 误读 wram[0x6AF4]（=EnsExtra 空槽0 status！）⇒恒 0⇒名表差漏判+256 | 改读 **$030C（Objects.0.hi）**（F193） |
| **D3 F282 状态门缺失** | Frozen(4)/Hurt(6) 敌误伤 | 补 `status<4` 免伤门 |

D2 传导：emu f1167 x 回绕翻 Samus.hi=1、敌 hi=0 ⇒异名表拒触（InvDly=0）；rust 假 hi=0
⇒f1278 误触击退（IsHit=0x30|0x08）⇒轨迹再分叉。修后 **f1255-1290 七字段 0 差+
f1278-1290 无击锚+f1524 换房锚（MapPosX=1/Room=255）全通**。

### 9.4 TDD 记录（红→绿）
| 测 | 红因 | 绿证 |
|---|---|---|
| enemy_active_offscreen_is_skipped | 旧门漏 Active（bit6 翻 88≠24） | ok |
| enemy_active_bit1_latch_skips_visibility_check（守门） | （先行绿） | ok |
| enemy_resting_offscreen_is_skipped（守门） | （先行绿） | ok |
| enemy_hit_box_respects_samus_hi_nametable | 误读 hi⇒hit=56≠0 | ok |
| frozen_enemy_deals_no_damage | hi 修后转红（hit=56——门因） | ok（序列化红→绿） |
| morphball_traj_window_and_enemy_anchor（集成） | f847 敌位形 (66,88)≠(63,24) | ok（敌窗41+轨迹36+无击+换房全通） |

## 10. 残差卡点（如实登记不伪锚）

1. **缝标号伪差**：采样缝缺行（emu 缺 1344-1346、rust 缺 1344-135）邻域 f 标号可偏 1
   （rust f1346 行 FC=47=emu f1347 行同态）——行为无滑（FC 同拍）；对表/锚测以 **FC 为
   共同真值**（n/f=f=n+12 与 FC 双口径在案）。
2. **f1300-1520 X 相位 ±1-2px 残差**（三根修后仍存）⇒f1521 拾取触盒边值差 1px 未触
   （emu (135,98) 触发/冻结能力动画；rust (135,97) 边外）⇒**f1524 gear/pw0 锚不可达**
   （拾取面=触盒边值+X 相位链——下卡）。D2-1b 授予径本体无恙（card E 单测在）。
3. **PPUCTRL_ZP bit2 漂移**（emu 0x94 vs rust 0x90，f1440-1520 持续；bit0/判盒用位同）
   ——名表/滚屏账面残差（goal-轮66 面），待专项。

Evidence: asm=prg1_brinstar.asm:156-170/:281（L968B）+enemies/crawler.asm:1-195 全谱+
prg7_engine.asm:10488-10560（F351/F37F/F3AA）/10077-10300（F140/F17F/F193/F1FA）/
10331-10364（F282）/7060-7117（E2A9——前卡）；实证=emu frames.jsonl 敌窗 f820-860 逐帧+
f846 停机位形+f1167 x 回绕+rust state 行（$030C=1/wram[0x6AF4]=0 直读）+TDD 六测红→绿
（frozen 测 hi 修后转红门因实录）。Confidence: 93

## 11. X 相位残差链卡（2026-11-05——**结构边界停机报**）

### 11.1 首异链（FC 对齐口径——缝标号伪差免疫）
f1255-1290 三根修后 0 差；**首异=FC=120（emu 行 f1420）输入面**：
| 同 FC=120 | Joy1Status | Joy1Change |
|---|---|---|
| emu | **1（右）** | **1** |
| rust | 2（左） | 0 |
⇒ 转脚工作（右输入帧）：emu handler 见新方向→samL12 `sta SamusDir`+`jsr SetSamusRun`
（accel←$30）→SetSamusData(3) 集成 0xA0+0x30=**0xD0**；rust handler 仍见旧方向→
accel 滞后 1 帧→集成 0xA0+0xD0=**0x70**（emu f1421 Sub=208/rust f1420 Sub=112——
±0x60 恒偏）⇒X 周期性 ±1 滑移⇒f1521 拾取触盒边值差 1px（emu (135,98) 触/rust (135,97)
外）⇒f1560-1700 23k。

### 11.2 根定位=输入消费相位（结构性——缝会计族）
输入写点=rust_screenshot.rs:446 `joy1←plan.joy[label]`（tick 前）→NMI 末 read_joy_pads
锁存→缝采样——**写点/锁存序双腿同构（goal-轮68/70）**；差异=**label↔FC 映射**：
缝缺行区双腿 spin-出口会计不齐（emu 缺 1344-1346=3 帧 vs rust 缺 1344-1345=2 帧）
⇒ 同 FC 的 label 差 1 ⇒ sched[label] 喂到错 FC ⇒**输入消费迟 1 帧**（FC 级 joy 面
直证）。「一迭代双步/缝会计」结构族（move-drift R1 自留「同迭代链断裂」同族）——
按任务书件 5 结构性深根**如实停机不强修**。

### 11.3 asm 对读（E31A 族——本卡件 1 完成）
- SamusMoveHorizontally（E31A-:7170-7230）：delta 分向逐像素环+CheckStopHorzMvmt
  （E365：受阻⇒断环/地面⇒**SetSamusStand**（CF55→NoHorzMoveNoDelay→StopHorzMovement
  →**ClearHorzMvmntData（CF4C）=只清 speedX int+frac——SubPixX「linear counter」
  不清**）+stand 态/anim 全形写）。
- **转脚真身=samL11/12（:2945-2962）**：无方向⇒SetSamusStand；有⇒BitScan/cmp
  SamusDir/不等⇒`sta SamusDir`+`jsr SetSamusRun`（CC98——accel←RunAccelTbl[新 dir]）
  →**SetSamusData(3)（集成）后行**——emu 集成用新加速的 asm 确认。
- CDBF（SamusRun_SetAnim）：aim 变⇒SetSamusAnim+尾跳 CD6B（跳过 samL09-12）——
  转脚帧 aim 未变不走此尾。
- 集成面：HorzAccelerate 尾 `SubPixX+=SpeedSubPixelX`（前卡 E3E5 全谱+FEE0 钳在案）
  ——SubPixX 相位差=加速度值差的直接投影（0xA0+0x30 vs 0xA0+0xD0 双向验算合）。

### 11.4 TDD 锚设计（下卡随修落地——本卡不伪绿）
- Joy1Status/Joy1Change 面 FC=120 双腿 0 差锚（输入消费相位）；
- f1300-1520 窗 SubPixX/X 逐帧 FC 对齐 0 差锚（修后 f1521 触盒自通）；
- f1524 gear/pw0 锚（任务书件 3 原靶）。
防回迁回归面：f820-860 敌窗+f1255-1290 轨迹窗（本档 §9 集成锚）保持。

Evidence: 双侧 FC 对齐逐帧面（Joy1Status/Change/accelX/SubPixX/X——emu frames.jsonl
watch+rust state 行）+asm=prg7_engine.asm:2945-2962（samL11/12）/7170-7230（E31A 环）/
7230-7245（E365）/3349-3370（CF4C/CF55）/2824-2843（CC98）/3371-3390（CDBF）+nmi.rs:228
（read_joy_pads）+rust_screenshot.rs:446（输入写点）；数值双向验算（0xA0+0x30=0xD0 /
0xA0+0xD0=0x70）。Confidence: 93

## 12. 缝会计/输入消费相位卡（2026-11-05——**全链收敛**）

### 12.1 缺行集定性（全案对表）
1700 帧双侧缺行集**唯一真差**=1344 载房窗（emu 3/rust 2）；1569-1570 rust 独缺=
拾取分歧下游（rust 无拾取⇒续行撞界载房）；1700=尾伪差。**缝契约双腿同构**（span
tick 不采——walk-fc-capture-seam §2c）；差=**该迭代门长**。
> 【2026-11-05 更正】上段「1569-1570」系缝会计修前初测值；该第二窗帧号随各修段轨迹
> 漂移（1569-1570（修前）→1562-1563（窗修后）→**1563-1564**（verifier 复核实测——
> 源=旧 Oct-31 state.jsonl 作废档））。**修后（SetTimer 冻结+检点修落）二窗整体消除**：
> canonical 修后 trace=`shots_rust/case-morphball-probe.state.jsonl`（2026-11-05 20:29
> 再生，旧 Oct 31 档作废）缺行集=[2,4,5,6,311-313,352-358,812,813,**1344,1345,1346**]
> =emu 同集（除 emu 尾伪差 1700）✓ 全案对齐。复现：`cargo run -q -p game-core --bin
> rust_screenshot -- --schedule shots_rust/case-morphball-probe.sched --out /tmp/x
> --state-out shots_rust/case-morphball-probe.state.jsonl`。

### 12.2 机制=工作超帧（asm 实证）
载房域 E54A-F034 **零显式 Wait**（WaitNMIPass/WriteAndWait 全径 grep）⇒窗长=NMI
边界跨界数=工作周期/29780（NMIStatus 门跳内块（prg7:251）——ReadJoyPads 同跳⇒输入
消费滞后）。窗长随工作量变：812 窗=room23（504 tile/12 对象/1 敌）=2；1344 窗=
room8（1024 tile/16 对象/0 敌）=3——**「每次载房=2」系 crawlprobe 2 例校准的以偏
概全（本日证伪更正）**。工作量实测（[loadq] 探针）+6502 逐条计数落周期模型
（room.rs LOAD_*：InitTables 9262/元 tile 快径 214（UpdateAttrib 早退）全径 349/
对象壳 180/敌 300/基底 27800（约束夹逼 [25850,29780)））。

### 12.3 传导链（三件修——红→绿）
1. **门数计算形**（physics.rs check_update_name_table）：iter_bogged(2) 硬编码⇒
   `dropped=(LOAD_BASE+work_cycles)/29780`——1344 窗 2→3 **与 emu 缺行集全案对齐**；
2. **SetTimer 三写勘误**（mainloop.rs game_engine）：asm C4AA=Timer3←$18+NextRoutine
   ($20)←3+**MainRoutine←_id_WaitTimer(9)**——旧形写 MainRoutine←3（下例行误当本行）
   ⇒无冻结；死亡链同形修；
3. **延时旗检点勘误**：asm 旗址=$0108(MiniBossKill)/$0109(PowerUp)（constants:809-810）
   ——旧形读 $0083/$0084（错位）⇒拾取旗永不可见⇒SetTimer 链不发；
4. **WaitTimer（$C494）端口**（mr=9 分派臂——旧 unported 桩）：Timer3≠0⇒冻结保持/
   尽⇒NextRoutine→MainRoutine（4/6 免起乐否则 StartMusic）。

### 12.4 判据实况（全链收敛）
- **缺行集双腿全案一致**（[2,4,5,6,311-313,352-358,812-813,1344-1346]——emu 同集）；
- **f1347-1700 标签对齐 0 差**（FC/X/Y/SubPixX/gear/MainRoutine 全字段）——输入相位、
  轨迹、拾取（f1521 gear=0x10 双腿同）、冻结（f1521+ MainRoutine=9/位形 (98,135)）全合；
- L4 morphball 七格：f1300 **0px match**、f1460-1700 全 **118px**（<500px 判据✓——
  修前 23k）；f1524 gear/pw0 锚+f1560 冻结锚全恢复（锚测全绿）；495 测全绿；
  防回退面（collide f800/jumpledge f730/killenemy/scrollseam）全 match 0px 保持。
- ⚠ 残差 118px=**物品闪帧相位差 1**（同 tile 异调色板=闪对——item anim 相位）——
  每格恒定、<500px；下卡件。

Evidence: 缺行集双侧实录+asm=prg7_engine.asm:7586-9894（载房域零 Wait grep）+
195-290（NMIStatus 机制）/2025-2050（GameEngine 延时旗）/C4AA（SetTimer 三写）/
C494（WaitTimer）+constants_ram:809-810（旗址）/480/496（NextRoutine/Timer3）+
room.rs LOAD_* 常量（6502 逐条计数）+[loadq] 工作量实测+Lua inputPolled 注入语义
（tools/scenario_capture.lua:35/514/600）+TDD 四测红→绿+锚测（f1524/f1560 恢复）
+L4 矩阵（七格 23k→118）。Confidence: 93

### 12.5 物品闪帧相位卡（2026-11-05——七格 0px 达成）
根=**SpriteAttrsOverride（E038）调色板覆盖未端口**（A级全谱）：UpdateOnePowerUp
（DB42 :5539-5554）每帧写 ObjectCntrl=((FC>>1)&3)|$80（闪帧 4 色轮换）；DCC3 尾
MSB=1⇒E038 `attr=(ctrl_byte&0xC0)|ObjectCntrl`（asl/lsr 对=剥 MSB；PRIORITY 不入）。
旧形 sprite.rs 恒取 ctrl_byte 静态位⇒emu 闪/rust 不闪。修=E038 覆盖端口（&0x7F=
剥 MSB 形；首轮 0x80 留入致 VFLIP 已回退修正）；TDD 双测红→绿。
**判据**：L4 morphball 七格 **0/61440 全零像素**（f1300-1700——连续 3 测（2 HEAD）
一致）✓。⚠ 格态=🎲flaky（工具记账伪象）：code_state 键=git HEAD **不计未提交改动**——
628bbabd 组 5 行=2 diff（0x80 bug worktree）+3 match（修后 worktree）同 HEAD 混账
⇒判 flaky（classify_flaky 同码态 ≥3 不一致）。585dd937 组 3 match ✓ / e3079c52 组
1 match ✓ 均一致；flaky 组账 append-only（不静默删）⇒格标滞留。**物理真值=七格 0px**
（shots_rust/matrix.json 各格 pixels=0/61440）；真增益锁点（match 96→102）待工具
码态键粒度（含 worktree 态）改进或账史滚动后落。l4_run_observations.jsonl f1300 全史
可溯（20261103 起 80+ 行）。

## 13. case-002 邻房 9.4k 归因卡（2026-11-05——恶化归因+模型补全）

### 13.1 恶化归因（worktree 隔离法——git stash 禁用）
**628bbabd^（=8ec44b75）worktree 隔离跑 case-002**（/tmp/wt-pre-seam——ROM/Mesen2
链入）：f1000/f1150=**46/46px**（=lead 11-04 读数复现）⇒现 9464/9469px=**628bbabd..
HEAD 区间 fix 副作用**（①）——非既有显形。RAM 面三点定相位：f1000 FC=215(pre)/216(cur)
+ScrollX 126/128+SamusX 13/15 ⇒**当前时间线前移 1 帧**=载房掉帧计数 2→1。

### 13.2 根=掉帧窗模型边界敏感（缺工作量组件）
模型 dropped=(BASE+work)/29780 对 room20（case-002 邻房：obj=4/ysum=26/1 敌）算出
work=29326→count=1（边界下 4%！）；真值 2（emu 缺行集/pre-seam 46px 双证）。约束反证：
case-002 需 BASE≥30234 与常规帧 BASE<29780 **矛盾**⇒**模型缺组件**（y 位环/行壳/
固定壳）。补全（asm 逐条计数）：y 位环 20c/y 单元（LEA72）+DrawStructRow 行壳 30c/行
+setup_room 固定壳 1300c（ScanForItems/DeleteOffscreen/EndOfRoom/表查）+BASE 27800→
**29000**——三房全合：room20=**2**/room23=**2**/room8=**3**（emu 实录）+常规 0。

### 13.3 判据实况
case-002 f1000/f1150=**46/46px**（9464/9469→46——回 pre-seam 水平 ✓<500px）；
match **102/108 保持**（morphball 七格全 hold ✓ 无回退）；498 测全绿（+1 校准锚
load_window_model_calibrated：三房计数+常规 0 断言）；防回退面全保持。
### 13.4 小残差三格定性（顺带落档——不强修）
f950(case-001 combo)=46px/f960(crawlprobe end)=**174px=双敌 sprite 着色残差**（diff
绿簇=敌 sprite 位——敌帧/调色板相位域）/f1400(walkfcprobe end)=58px——sprite 域小
残差+时序微差；<500px 皆在容内。⚠ 值随模型校准微移（f950 250→46 改善/f960 46→174
微增）——同族 sprite 相位面，登记不强修。
⚠ f1200(case-001 end)=14147px 仍为最大残差族（非本卡靶——后续卡）。

## 14. f1200 端态动量卡（2026-11-05——归因+首异钉点（停机报））

### 14.1 归因（worktree 隔离法——同 case-002 法）
628bbabd^（8ec44b75）worktree 跑 case-001：f1200=**14147px 与现 HEAD 相同**⇒根在
更早区间（11-04 1015→16069→14147 跨多修区变化——非近三卡副作用）。历史件状态核查：
Salto 动画相位✓已端口（173ce165 Phase2：JumpDisp≥$18 触发面）；空中漂移速度字节
（d568bf0c）：rust 现**保持动量**（sX=(FE,80) 恒）——「解旋清速」已消✓；但**门撞检
阻断径**仍存=当前根（下）。

### 14.2 首异钉点+模式（f1150-1210 窗外扩 f1029-1045 逐帧）
首异=**f1033**：X 走相偏 1px。逐帧 dX 模式：emu 恒 −2/−1 交替（−1.5px/帧）；rust
**每 6 帧丢 1 次移动**（f1033/1039/1045…dX=0 代 −1）⇒均速 1.33 vs 1.5⇒漂移累积
（f1150 (3,16) 偏/f1200 14147px）。**每 6=8px 门（(x−rx)&7==0）撞检节律**——
MoveSamusLeft（E626）：门帧 `jsr ObjectCheckMoveLeft / bcc Lx181`（阻断⇒crash 出口）
+每调 `SamusCollisionWithSolidEntities`（isHit&$41==$41⇒出）——rust 门撞检在墙顶角
判阻断/emu 放行（**墙顶角 1px 类**——d568bf0c 已定名根因）。Y 弧差（f1150 +16）=
阻断累积侧效。

### 14.3 停机报（件 4——非伪修）
根已钉至墙顶角碰撞语义（ObjectCheckMoveLeft 门撞检 corner 判定 vs emu 1px 放行
quirk）——修需 E88B/object_check_move_horizontal corner 径 asm 逐行对读（D4 quirk
保真：emu 的墙顶角 1px 穿透=原版行为须保留非修掉）+TDD——**深件另卡**（与 d568bf0c
修法方向（漂移字节）已部分落的并轨）。本卡交付=归因（非近三卡副作用✓）+首异钉点
（f1033/每 6 门撞检模式）+历史件状态核查（Salto✓/漂移字节✓/门撞检✗=当前根）。

Evidence: worktree 隔离实录（/tmp/wt-pre-seam case-001 f1200=14147px=现 HEAD 同值）+
逐帧 dX 双侧表（emu −2/−1 恒 vs rust 每 6 帧 0——f1029-1045 17 帧实录）+rust 态面
（SubPixX 0x50/0xD0 交替+sX=(FE,80) 恒+AccelX=0——ir[0x211/0x209/0x213/0x215]）+
asm=prg7_engine.asm:MoveSamusLeft(E626-:E663 全谱（8px 门+crash 出口+solid-entity 检））
+历史件（173ce165/d568bf0c/5abbe6c7 提交实录）。
Confidence: 91

## 15. 墙顶角碰撞卡（2026-11-05——取证分流+停机报）

### 15.1 asm 全谱（件 1）
- ObjectCheckMoveLeft（**E880**：`lda radiusX/clc/adc #$08`→dist=rx+8）→
  ObjectCheckMoveHorizontalBranch（**E893**：Temp03←dist/StoreObjectPositionToTemp/
  rY→CheckMoveHorizontal）；
- CheckMoveHorizontal（**E89B**：rY=0⇒sec 出/顶界=y−ry/顶余 and#7⇒+1 块/
  GetNumBlocksToCheck）；GetNumBlocksToCheck（**E8CE**：底余=(−顶余)&7/跨度=2ry−底余
  （+8 借位修正）/中心=跨度>>3/右余 and#7⇒+1/总数=X+中心+右余）——**rust span_rows
  公式逐行等价 ✓**；
- 碰撞点环 LE7DE（**E7DE**：`bne Lx202` **非块对齐⇒跳过整检（放行！）**/LE7E6：Make
  RoomRAMPtr→瓦片→$4E 门/CheckBlastTile（< $80=固实⇒Exit16 carry 清=阻断）/>=$A0
  可走（SamusDoorData 写：$A0→1/$A1→2/余→0）/CalculateNextBGCollisionPoint 逐点）；
- MoveSamusLeft（**E626**：8px 门（(x−rx)&7==0）⇒ObjectCheckMoveLeft/bcc Lx181（crash
  出口（清 $58））→每调 SamusCollisionWithSolidEntities（isHit&$41==$41⇒Lx181）→
  ScrX/ScrollLeft→dec x）。

### 15.2 取证分流（件 2——机制钉定非猜）
f1033 窗瓦片级探针（[corner] 729 点）：**查撞点瓦片全 0xFF（可走）**⇒ObjectCheckMoveLeft
**非阻断源**；isHit($30A) 逐帧=0⇒isHit 0x41 **亦非**；[bnc] LEFT_BLOCKED 标记**零次**
⇒查撞从未返回阻断。**块源=移动机构/积分层**（每-6-门相位与移动路径交互——X dX=0 帧
无撞挡证据）。

### 15.3 停机报（件 6——深件另卡）
阻断径已排除碰撞模型（瓦片/isHit/撞挡三清）——剩移动机构/积分层（move_samus_left
调用相位/int_x 积分序/门相位耦合）=**更宽移动模型**深件。本卡交付=asm 全谱（E880/E893/
E89B/E8CE/E7DE/E626 行号锚）+取证分流（三清实录）。

Evidence: asm=prg7_engine.asm:8197-8260（E880-E8BE）+E8CE-E8EC（GetNumBlocks）+
LE7DE-LE81C（碰撞点环）+E626-E663（MoveSamusLeft）；[corner] 729 点瓦片实录（全 0xFF）+
isHit 逐帧表（f1028-1047 全 0）+[bnc] 零标记+rust 端口（object_check_move_horizontal/
span_rows/collision_point_samus/move_samus_left 逐行）。
Confidence: 92

## 16. 移动机构/积分层深读卡（2026-11-05——链谱+实录钉定+停机报）

### 16.1 asm/端口链谱（件 1）
链（physics.rs:1085-1176 端口）：vert_accelerate(E37A)→vert_integrate→Y 逐像素环
→lava→**horz_integrate（IntXState→int_x 加速/钳）→dx→samus_move_horizontally
（E31A：ScrX=x−ScrollX/delta=0⇒Exit10/TwosComplement/岩浆减半/OBJECT_COUNTER=n
逐像素环（move_samus±+check_stop per px）/门向）**。HorzAccelerate（**E3E5**：SubPixX
+=AccelX+符号扩 speedX/取负/NegateTemp 钳 max——速更段）；int_x=速积分（accel+clamp）。

### 16.2 每-6 dX=0 机制钉定（件 2——[smh]/[hxi] 双探针实录）
disp（int_x out）=−1/−2 交替恒正确（255/254）✓；**但每-6 门帧 smh 入口 X 增量=0**
（n=1027→1028：x 143→143——disp=−1 却未移！；n=1033→1034：135→135 同）⇒
**移动施加段丢失**（非积分（disp 对）/非撞挡（三清）/非 delta 计算（对））。窄化至
**move_samus_left 写 X 被覆/未落段**（门帧分支：scroll_left（E63F 列流送链）/SCR_X 写/
x==0 环绕判）——写 x=−1 后被后续写覆回（候选：scroll_left 链的 x 写竞争）。

### 16.3 停机报（件 5——结构性深根）
每-6 门帧行为的丢失点在 move_samus_left 写 X 后的覆写/scroll_left 链（E33F 列流送→
check_update_name_table 载房门→iter_bogged 相位耦合）——**更宽移动/滚屏模型**深件
（件 5 情形）。本卡交付=链谱（E3E5/E31A/int_x 序）+[smh]/[hxi] 双探针实录（disp 对/
入口增量 0——非猜）。
> 【2026-11-06 更正】§16.2「窄化至写 X 后覆写/scroll_left 链段」**被 §17 证伪**
> （[wx] 一击：写全缺=早退径非覆写）——真根=**0xF7 帧径 0xFF→查撞摸地板阻断**（§17.2/
> §18.2）。disp=−1/−2 对+入口增量 0 实录仍有效（早退径的上游证据）。

Evidence: asm=prg7_engine.asm:E3E5-E426（HorzAccelerate）/E31A-（SamusMoveHorizontally）+
端口链 physics.rs:1085-1176+742-776（horz_integrate）；[hxi]/[smh] 双探针 n=1025-1039
15 帧实录（disp 255/254 交替 vs 入口 x 增量 0×2（每 6））+三清（§15.2）+isHit/[bnc] 实录。
Confidence: 91

## 17. E63F scroll_left 写序卡（2026-11-05——覆写点一击钉定（实为早退径）+全链定谳+停机报）

### 17.1 覆写点钉定（件 2——[wx]/[smh]/[bnc] 三探针一击）
「覆写」假设**证伪**：[wx] 写后探针实录 n=1027/1033 **写全缺=早退径**（非覆写）！
[bnc] 标记（窗修后——旧窗 1255+ 不覆 f1033 段=**废证更正**）：n=1027/1033
LEFT_BLOCKED **rx=255** ✓。[corner] 阻断查撞全点：5 点（y=160/168/176/184/192@x=136）
——第 5 点 y=192 瓦片 **0x5D（固实地板）**⇒object_check_move_left 阻断 ✓。

### 17.2 全链定谳（半径 0xFF 根）
state 逐帧径面：f1030/1032/1034 **radiusY/X=255/255**（交替帧）+**animFrame=0xF7**
（vs 常帧 0x08——walk 表交替项！animDelay 4↔0）——帧数据径读（sprite.rs:79-82：
`blob(fptr+1/2)→$301/$302`）在 0xF7 帧得 0xFF ⇒查撞 dist=rx+8=7（wrap）+span
摸地板⇒阻断⇒每-6 dX=0⇒X 均速 1.33 vs emu 1.5⇒f1200 14147px 全链。

### 17.3 停机报（件 5+升维提示——层 5）
emu 同 0xF7 帧径不阻（行为保真目标）——**0xF7 帧径 quirk**：asm CheckMoveHorizontal
（E89B）/GetNumBlocksToCheck（E8CE）对 rY=255 的 8 位算式 vs rust span_rows 差
+径表 0xF7 项语义（walk 表 terminator/帧数据读序）=**更宽动画/碰撞模型**深件。
**升维建议（lead 件 5 提示采纳）**：层 5 仍深根——应转「E31A/E63F 帧内执行序与 emu
整段对拍」（非继续单点窄化）——下卡建议。本卡交付=覆写证伪+[bnc]/[corner]/径面/animFrame
四层一击链（非猜）。

Evidence: [wx] 写后探针（n=1025-1035 全录——1027/1033 写缺）+[bnc]（窗修后 2 标记
rx=255）+[corner] 阻断 5 点（第 5 点 0x5D）+state 径面（f1028-1034 交替 255/255）+
animFrame 面（0xF7 交替）+sprite.rs:79-82 帧数据径读+asm E89B/E8CE。
Confidence: 93

## 18. 0xF7 帧径 quirk 整段对拍卡（2026-11-06——两径对拍+根终谳+停机报）

### 18.1 asm 两径对拍（件 1）
①**GetNumBlocksToCheck（E8CE）8 位算式**：`tya(=rY)/asl`⇒A=0xFE(254)+carry（**8 位
截断**！）/`sec/sbc 底余`/借位修正+8/`lsr×3`=span>>3/右余+1/X(顶余)+中心+右余=总数
——rY=255 时 asm span=254（8 位）⇒n≈32 点 vs **rust span_rows u16（diameter=ry<<1=510）
⇒n=64 点**（**算式不同构**——rust 多字节模型 vs asm 8 位）；
②**帧数据读序**：ObjDrawFrame（**DE4A**）=animFrame→ObjFramePtrTable[×2]→FramePtr→
GetSpriteCntrlData（byte0=ctrl_byte（低半字节=PlacePtrTbl 项号））→帧流（0xFC/0xFD/
0xFE/0xFF 标记）——**DE4A 全径无径写**！径写点全谱：3631（SamusRoll ±8）/3980（子弹
=2）/6294-6298（**敌径=帧数据**（「Get vertical radius in pixels of object」））——
**Samus 径=init/roll 常量**（非帧数据！）。0xF7=animFrame 值（walk 表项）→ObjFramePtr
Table[0xF7] 帧流字节 1/2=**流数据非径**（0xFF=流字节）。

### 18.2 根终谳（sprite.rs:79-82 发明径读）
rust obj_draw_frame:79-82 `blob(fptr+1/2)→$301/$302`（径写）=**发明**（asm DE4A 无此
写）⇒0xF7 帧流字节 1/2=0xFF 被当径⇒radiusY/X=0xFF⇒span_rows u16=64 点⇒第 5 点地板
（0x5D）阻断⇒每-6 dX=0⇒f1200 14147px。emu（DE4A 无径写）径恒 15/4⇒span≈4 点⇒不摸
地板⇒不阻 ✓✓ **全链闭合（含 emu 不阻机制）**。

### 18.3 停机报（件 6——修径=径源语义重模）
修=移除/迁移 sprite.rs:79-82 发明径读（Samus 径=init/roll 常量径；敌径=6294 帧径径
（draw_enemy 侧）；item 径=待考）——**径源语义重模**（Samus init/roll/敌/item 三径源
全谱）+span_rows u16→8 位同构（E8CE 算式）+TDD——需径源全谱 asm 对读后动手（D4 保真
——emu 径恒 15/4=目标非"修正确"）。⚠ morphball 径回归风险（径写移除影响球径面）——
须同卡验 L4 morphball 七格保持。本卡交付=两径对拍+根终谳（emu 不阻机制含）。

Evidence: asm=prg7_engine.asm:E8CE 全谱（8 位算式）+DE4A 全径（无径写）+径写点全谱
（3631/3980/6294-6298）+sprite.rs:79-82 对照+[chk] 入口探针（y=159 ry=255 dist=7
top=160 **n_pts=64**）+state 径面/animFrame 面+GetSpriteCntrlData（DCC3 帧流布局）。
Confidence: 93

## 19. 径源语义重模卡（2026-11-06——守卫修+判据实况）

### 19.1 三径源 asm 全谱（件 1）
①Samus 径=init 15/4 常量+roll 瞬态（D0E1 unroll `radiusY+=8`（3631）/SetSamusRoll
D0B5 **无径写**——球径=帧格式字节面）；②敌径=帧数据径写（**6294-6298**「Get vertical
radius in pixels of object」——帧格式字节 1/2）；③item 径=待考（SpawnPowerUp 槽写
面）。**帧格式=[ctrl_byte, radiusY, radiusX, 流(字节3起)]**（敌径写源证）——
**DE4A 全径无径写**（Samus 径写仅 3631 瞬态）⇒emu 径恒 15/4 ✓（D4 目标）。

### 19.2 rust 侧盘点+修（件 2-3）
sprite.rs:79-82 发明径读消费者=查撞 span（object_check_move_*）/可见性检（radiusX<
screenX）/is_samus_on_top/触盒——影响面=移动查撞+绘制+拾取。修=**帧有效性守卫**：
animFrame≥0xF0（标记/越界段）跳径写（径保持——asm 语义）；常规帧径写保留（格式语义
——敌/item 兼容）。TDD：radius_guard_skips_marker_frames 红→绿（0xF7 跳/常规帧写）。

### 19.3 判据实况
**f1200=14147→1015px**（=11-04 水平复现！13.9x 收敛——每-6 阻断消除 ✓）；⚠ 1015px
>500px 判据差一点=**11-04 期固有残差**（修前历史读数=1015）——非本修回归（下一步
轨迹残差另卡）；**morphball 七格全 hold（0/61440）** ✓✓（径回归风险清——球径 15/4
恒实录）；match 102/108 保持；499 测全绿（+1 守卫测）；防回退面全保持（f950=46/
case-002=46×2/crawlprobe=174/walkfcprobe=58 无恶化）。

Evidence: asm D0B5/D0E1（roll 径）+6294-6298（敌径写源）+DE4A 全径（无径写）+DC8F
（0xFF 终标记/0xF7=帧号 247）+帧格式布局（流字节 3 起）+state 球径面（f1300-1620
ry=15/rx=4 恒）+TDD 守卫测+L4 矩阵（f1200=1015/morphball 七格 0px）。
Confidence: 93

## 20. 端态轨迹残差卡（2026-11-06——前提证伪+三族定性+停机报）

### 20.1 首异钉点=前提证伪（件 1）
修后双腿 state 逐帧对表 f420-1200 全窗：**FC/X/Y/status 全 0 差** ⇒「Salto 动量/
落点相位族」**前提证伪**（轨迹已全收敛——每-6 阻断消除后轨迹 0 差）。f1200 1015px=
**纯渲染/状态面**。

### 20.2 三族定性（diff 图+面实录）
①**HUD 能量族**：rust EN=30 vs emu **EN=22**（状态差——伤害面发散：敌接触伤害径）；
②**双敌 sprite 族**（f1200 顶中 2 簇=敌位——敌 sprite 着色/相位类（同 crawlprobe
f960=174px 族））；③**Samus sprite 族**（f1200 底中簇=萨 sprite 姿态/着色）。
⚠ 三族皆非轨迹面——端态轨迹卡前提（Salto/落点）证伪。

### 20.3 清尾三格定性（件 5）+停机报（件 6）
f950=46/f960=**174（双敌 sprite 族——同 f1200 ②）**/f1400=58——sprite/anim 着色
族小残差（<500px 容内）；f1200 1015px=三族（HUD 能量状态差+双敌+萨 sprite）——
**停机报**：HUD 能量=伤害径状态发散（敌接触伤害——深件）+sprite 着色族=敌/萨 anim
相位面——两族深件另卡。本卡交付=前提证伪（轨迹 0 差实录）+三族定性+清尾落档。

Evidence: 双腿 state 全窗对表（f420-1200 FC/X/Y/status 0 差——[wx]/[chk] 探针体系）
+diff 图三簇定性（HUD 顶左/双敌顶中/萨底中）+EN 值实录（30 vs 22）+morphball 七格
0px 保持+L4 矩阵（f1200=1015/match 102）。
Confidence: 91

## 21. 伤害径状态发散卡（2026-11-06——根定谳+asm 全谱+停机报）

### 21.1 首异钉点+根（件 1）
EN 面：rust 恒 30 / emu 30→22（f950-1200 间伤害事件）。**根=SubtractHealth（CE92）
扣血径=嵌套桩未端口**（check_health_status 击退+i 帧✓已端口但扣血链缺——EN 恒 30）。
### 21.2 asm 全谱（件 2）
SubtractHealth（**CE92**）：HealthChange($6E 双字节 BCD)≠0⇒IsSamusDead/EndTimer/
MotherBrain 检→Varia 减半（gr_VARIA⇒lsr×2+补 $4F）→**Base10Subtract**（C3FB BCD
减）Health/Health+1($0106 双字节——「Lower health digit in upper 4 bits」）→死亡判
（BCD 0⇒sa_Dead+SetSamusExplode）。伤害置径（**LF306**）：HealthChange←AreaEnemyDamage
（各区表 95CE——prg1-4 同址）；SamusHurt_F311（isHit 置+knockback+ClearHealthChange）。
调用点：7049（直调）/F13D GotoSubtractHealth/12232。
### 21.3 停机报（件 6）
修=三件端口（SubtractHealth BCD+Base10Subtract+伤害置径 LF306（AreaEnemyDamage 伤害值
源——game-data 缺伤害表须补）+调用点）——BCD 数学+每区伤害表=中件（本卡交付=根定谳+
全谱）。sprite 着色族（②③+清尾三格）=另卡不碰 ✓。

Evidence: check_health_status 端口对照（击退/i 帧✓/扣血✗）+asm CE92 全谱+LF306/F311
+constants_ram:806（Health $0106 BCD）+132（HealthChange $6E）+AreaEnemyDamage 四区
95CE+Base10Subtract C3FB+L4 矩阵（f1200=1015/HUD 簇）。
Confidence: 92

## 22. 伤害径三件端口卡（2026-11-06——BCD 语义深层差停机报）

### 22.1 asm 全谱（件 1-3——已落 §21.2）+BCD 算式逐字
Base10Subtract（**C3FB**）+ExtractNibbles（**C41D**）逐字：delta 拆 $01(低4)/$02(高4)
/返回 A=health 低4→`sbc $01`（个位减）→借位修（+10/$02 调）→`lda health/and #$F0/
sec/sbc $02`（高位减）→借位修（+$A0）→`ora $01` 合成。
### 22.2 语义深层差（件 6 情形——三处不符）
①**health 格式**：constants:806「Lower health digit in upper 4 bits」——高4=低位数/
低4=高位数（交换 BCD）vs 标准 BCD 读法（0x30=30）两可；②**Base10Subtract 逐字算式
(0x30, 0x80)=0x50** vs emu 实录 0x22（30→22）——减式结果不符；③**伤害值编码**：
AreaEnemyDamage=$0080（bank1）=BCD 80？交换 08？vs 实扣 8——编码语义未定。
### 22.3 停机报（件 6）
BCD 语义（health 格式/减式/伤害编码三处）与 emu 深层差——**须 BCD 语义考证件**（emu
实录驱动：多伤害事件的 health 值序列+伤害值序列对表定格式）后方可端口（不许猜式
实现——D4 保真）。本卡交付=算式逐字+三处不符实录。

Evidence: Base10Subtract/ExtractNibbles 逐字（C3FB/C41D）+constants:806（health 格式
注）+AreaEnemyDamage 四区值（80/100/300/200）+算式 (0x30,0x80)=0x50 推演+emu 实录
30→22+check_health_status 端口对照。
Confidence: 90

## 23. BCD 语义考证件（2026-11-06——实录+前提再证伪+停机报）

### 23.1 emu 实录（件 1——考证副本扩 watch：health $0106/$0107+HealthChange $6E）
case-001 全程 health 面：**唯一事件=f1029 初始化（0→(32,2)）**——无伤害事件（health
恒 (32,2) 至 f1200）。⚠ 前提再证伪：**health 态 $0106/$0107=(32,2) 双腿同值**
（rust state 实录同 (32,2)）——「伤害径状态发散」前提**证伪**（状态不发散！）。
### 23.2 EN 显示差=HUD 渲染面（非 health 态）
EN=30(rust)/22(emu) 显示差 vs health 态同值 ⇒**HUD 渲染源/映射差**（显示地址或
BCD→十进制映射径）——非 SubtractHealth/伤害径。BCD 语义三处（格式/减式/编码）在
health 态同值下**不构成本显示差根**。
### 23.3 停机报（件 4-5）
「事件→health 变化」对表=单事件（init）——case-001 无伤害事件；扩场景取证（伤害事件
多的场景）+HUD 显示源考证（EN 渲染地址/映射）=下步（件 5 情形）。本卡交付=health 态
双腿同值实录（前提证伪）+HUD 渲染面定性。

Evidence: 考证副本（/tmp/case-bcd-forensic.txt——watch 0106/0107/6E/6F）+emu trace
health 逐帧面（f1-1200——唯一事件 f1029 init）+rust state $0106/$0107=(32,2) 同值
实录+diff 图 HUD 簇。
Confidence: 91

## 24. HUD 显示源考证卡（2026-11-06——asm 全谱+端口对表+停机报）

### 24.1 asm 全谱（件 1——行号锚）
DisplayBar（**E0C1**）：①DataDisplayTbl（E1B9,40B=10 精灵）→OAM 拷贝 ②健康 2 位：
`lda Health+1/and #$0F`（**upper=Health+1 低4**）/`lda Health/jsr Adiv16`（**lower=
Health>>4**）→SPRWriteDigit（**E173**：tile=A|$A0 数字图案区）③导弹 3 位（HexToDec
E198）④能量罐（LE14A：EnergyTankCount→满/空罐瓦片 $6F）。
**health 格式定谳（A级）**：upper=Health+1&0x0F / lower=Health>>4——**交换 BCD ✓**
（(32,2)→"22"=emu 实录 ✓）。
### 24.2 端口对表（件 2）
rust uw_display_bar（mainloop.rs:2653-）：①DataDisplayTbl 拷贝✓②digit 映射
（$0107&0xF|$A0/$0106>>4|$A0）✓**与 asm 同构**——映射面无差。⚠ 渲染面差：rust 截图
顶左=块形（罐瓦片？）vs emu="EN=22"（数字形）——**DataDisplayTbl 瓦片/位形/罐瓦面差**
（非映射）。
### 24.3 停机报（件 6）
渲染面（DataDisplayTbl 40B 内容/瓦片 ID/位置/罐瓦 tile）与 emu 差=更深渲染件（HUD
精灵面——非显示源映射）。本卡交付=asm 全谱+格式定谳（交换 BCD A 级）+端口对表（映射
同构）+渲染面定性。

Evidence: DisplayBar E0C1 全谱+SPRWriteDigit E173+DataDisplayTbl E1B9+health 格式
实证（Health+1&0xF/Health>>4——(32,2)→"22" 对表）+rust uw_display_bar 端口（2653-）
对照+双截图 HUD 簇。
Confidence: 92

## 25. HUD 精灵渲染面卡（2026-11-06——矛盾解+三面全合定谳+停机报）

### 25.1 矛盾点先解（件 1——双截图裁剪定性）
rust 顶左=块形（**"EN"标签+数字错瓦片图形渲染**！）+罐方块行✓；emu="EN▪22"✓——
显示差=**sprite 瓦片图形面**（非映射/非表/非态）。
### 25.2 三面全合定谳（件 2）
①DataDisplayTbl（**E1B9** 40B=10 精灵 y/tile/attr/x）逐字 vs rust DATA_DISPLAY_TBL
（mainloop.rs:2660）——**逐字节同 ✓**（含尾 3 行："E"=$21,$76,$01,$18/"N"=$21,$7F,
$01,$20/".."=$21,$3A,$00,$28）；②digit 映射（E173/2655）同构 ✓；③health 态 (32,2)
同 ✓。**三面全合**⇒差=④**sprite 瓦片图形（CHR 图案面）/精灵可见面**（tile 0x76/0x7F/
0xA0+digit 渲成块形——CHR-RAM 图案数据/PPUCTRL sprite 图案表选择/可见性面）。
### 25.3 停机报（件 5）
CHR 图案面（sprite 瓦片图形数据——CHR-RAM 加载/图案表选择）=更深渲染件。本卡交付=
矛盾解（三面全合+图形面定性——非猜：表逐字+映射对照+态实录+双截图裁剪）。

Evidence: E1B9 表逐字（10 行含 "E"/"N"/".."）+rust DATA_DISPLAY_TBL 2660 逐字对照
（同）+digit 映射（E173/2655）+health 态 (32,2)+双截图 HUD 裁剪（rust 块形 vs emu
"EN▪22"）。
Confidence: 92

## 26. CHR 图案面卡（2026-11-06——asm 全谱+CHR 实证+停机报）

### 26.1 asm 全谱（件 2——行号锚）
GFXInfo（**C6E0**：条目=[gfx 源/目的/尺寸]——[SPR]Samus,items Entry0 dest $0000/
[SPR]BrinstarEnemies Entry6 dest $0C00/[BGR] 各区）；LoadGFX（**C7AB**：条目→$00-$06
→BankSwitch→CopyGFXBlock）；CopyGFXBlock（**C7D5**：PPUADDR 目的→ROM→图案表逐字节）。
sprite 图案表选择=PPUCTRL bit3（$0000/$1000）。
### 26.2 CHR 实证（件 1——rust ppu.hex f1200 图案字节）
tile 0x76（"E"）=`FE FE C0 FC FC C0 FE FE...`=**框块图案** ✗（非字形）；tile 0x7F
（"N"）=同块形 ✗；tile 0xA0（"0"）=`38 4C CE C6 E6 64 38` ✓字形正确；tile 0xA2
（"2"）✓正确。⇒**digit 瓦片（0xA0+）图形对；"E"/"N"（0x76/0x7F）图形错=块形**——
gfx 装载覆盖缺口（0x76/0x7F 区未装字形或装错块数据）。
### 26.3 停机报（件 5）
gfx 装载覆盖（0x76/0x7F 区的 gfx 条目/尺寸/源数据——GFXInfo 条目域+CopyGFXBlock 尺寸
边界）=更深装载件。本卡交付=asm 全谱+CHR 实证（字节级：块形 vs 字形——非猜）。

Evidence: GFXInfo C6E0/LoadGFX C7AB/CopyGFXBlock C7D5 全谱+rust ppu.hex f1200 图案
字节实录（0x76=框块/0x7F=块/0xA0="0"✓/0xA2="2"✓）+双截图 HUD 裁剪对照。
Confidence: 92

## 27. gfx 装载覆盖缺口卡（2026-11-06——条目覆盖✓+装载径断点+停机报）

### 27.1 缺口定位（件 1）
GFXInfo 条目表（C6E0）+GFX_INFO 烤焙（gfx_data.rs 29 项×7B）逐条对读：**Entry0
（GFX_Samus bank6 src $8000 dest $0000 size $09A0）覆盖 0x76/0x7F ✓**（0x760/0x7F0
在 $0000-$09A0 内）——覆盖无缺口。⚠ 装载径断点：**init_title_gfx 载全零占位**
（`title_gfx=[0u8;0x2000]`→load_gfx→rs.chr）——GFX_INFO/GFX_DATA（23712B 烤焙✓）
**未接游戏起始 gfx 装载径**⇒0x76/0x7F 字形缺载=块占位（chr 实证 §26.2）。
### 27.2 asm 全谱（件 2——行号锚）
GFXInfoEntry 宏（macros.asm:101：bank+ptr+dest+sizeof=7B）+LoadGFX（C7AB）+
CopyGFXBlock（C7D5：PPUADDR→ROM→图案表）+LoadSamusGFX（C5DC）/InitTitleGFX（C5D7）
双载面。rust 对应：load_gfx（gfx.rs:61——纯拷贝）✓+init_title_gfx（零占位）✗+
游戏起始装载径（GFX_INFO 消费面）**未接**。
### 27.3 停机报（件 5）
游戏起始 gfx 装载径（GFX_INFO 条目消费+CopyGFXBlock 端口+装载时机）=中件（烤焙数据
齐+装载面缺接——非猜：条目覆盖✓/零占位实录/chr 字节✓）。本卡交付=缺口定位（断点环节
钉定）+asm 全谱。

Evidence: GFXInfoEntry 宏+C6E0/C7AB/C7D5/C5D7/C5DC 全谱+GFX_INFO Entry0（size
$09A0 覆盖 0x76/0x7F）+init_title_gfx 零占位实录+gfx.rs:61 纯拷贝+chr 字节（§26.2）。
Confidence: 92

## 28. 游戏起始 gfx 装载径接通卡（2026-11-06——链全端口✓+源实证+停机报）

### 28.1 asm 全谱（件 1）
装载链（C5D7 InitTitleGFX→C5DC LoadSamusGFX（SAMUS/SamusSuitless/IntroSprites/
**Font_Complete/Font_Hud**/Solid×2）→C604 InitBrinstarGFX（BrinBG1/CREBG1/CREBG2/
BrinstarSprites/Solid×2））+GFXInfoEntry 7B 格+LoadGFX（C7AB）/CopyGFXBlock（C7D5）。
### 28.2 端口对表（件 2）——链全端口✓
rust gfx.rs：init_title_gfx（TITLE/SAMUS/IntroSprites/**FONT_COMPLETE/FONT_HUD**/Solid
×2 ✓全链）+load_gfx（GFXInfo 描述子+bank 切换+CopyGFXBlock PPU 写 ✓）+boot.rs
init_bank1（Brinstar 环 ✓）——**装载链全端口 ✓**（font 条目在链）。
### 28.3 源实证（件 1——SAMUS_GFX 源 vs chr）
SAMUS_GFX 源 [0x760]=`FE FE C0 FC...`=**块形** = chr[0x760] ✓（装载对！）——但 emu
渲 "E" ⇒ **烤焙源数据（SAMUS_GFX 提取位）/emu CHR 面不符**：emu tile 0x76 字形来自
别处（提取位偏？emu 侧字形源？）——**非装载径**（链✓/装载✓/源=chr✓ 三面合）。
> 【2026-11-06 更正（复核 0cfee11b 注+§29.5 反转实证——原地）】「烤焙源差」前提
> **证伪**：emu ppu.hex tile 0x76/0x7F 双表逐字节=rust chr **全同**（§29.5）——CHR
> 数据同但渲染异⇒差=**渲染层**（sprite 瓦片选择/图案表基/attr）非源数据。运行时
> CHR-RAM 写入/其他 bank 窗口映射亦排除（CHR 同=无运行时写差）。
### 28.4 停机报（件 6）
源数据/emu CHR 面不符=更深件（ROM gfx 提取位验证/emu CHR 语义——非装载径可修）。
本卡交付=链全端口对表✓+源实证（三面合+源差钉定——非猜）。
> 【2026-11-06 矛盾钉定（复核 ae27e381 注——原地补）】「零占位 vs chr 块形」矛盾解：
> **双 load_gfx 函数**——title/gfx.rs init_title_gfx（零占位（标题径——title_gfx=
> [0u8;0x2000]））vs **frame/gfx.rs load_gfx（真数据（游戏起始径——gfx_blocks
> SAMUS_GFX 真装载））**。chr 0x76 块形=**SAMUS_GFX 真装载产物 ✓**（非零占位——
> game-start 用 frame/gfx.rs 径）——零占位仅标题径不涉游戏起始。§28.3 源差结论不变：
> SAMUS_GFX 源 [0x760]=块形=chr ✓（装载对）但 emu 渲 "E"=烤焙源/emu CHR 面不符。

Evidence: C5D7/C5DC/C604/C7AB/C7D5 全谱+rust gfx.rs/boot.rs 端口对照（链全✓）+
SAMUS_GFX 源 [0x760] 字节（=chr ✓）+emu "E" 渲染（双截图）。
Confidence: 91

## 29. ROM gfx 提取位验证卡（2026-11-06——提取位✓+emu 字形源谜+停机报）

### 29.1 提取位考证（件 1——字节锚）
ROM bank6:8760（file off 0x18770）=`FE FE C0 FC FC C0 FE FE FE C0 C0 FC C0 C0 FE`
=**SAMUS_GFX 烤焙源 [0x760] 同字节 ✓**——**烤焙提取位正确**（无提取位偏）。FONT
bank6:B4C0（FONT_COMPLETE/HUD 源）=数字起（"0"=38 4C CE C6 E6 64 38/"1"=18 38...）
——Font_HUD（dest $0A00 size $A0）=tiles 0xA0-0xAA 数字区 ✓（digit 瓦片图形对呼应）。
"E" 图案（FC 80 80 F8 80 80 FC 形）**不在 bank6**——emu tile 0x76 "E" 字形源≠bank6
gfx。
### 29.2 emu CHR 面（件 2——取证限）
emu 侧**无 ppu.hex dump**（仅截图 png——"E"/"N" 渲染可见✓但字节不可取）——emu
tile 0x76 字形字节实录**不可得**（取证限——scenario_run 无 CHR dump 面）。
### 29.3 停机报（件 5）
提取位✓（ROM=烤焙）排除「提取位偏」——emu 字形源（tile 0x76 "E" 从何装载）+emu CHR
字节取证（需 emu CHR dump 工具面）=**更深件/工具链面**（件 5 情形：转 emu CHR 取证
工具面+字形源考证）。本卡交付=提取位✓考证（字节锚）+取证限声明。
> 【2026-11-06 更正+取证补（§29.5-29.6）】①「E 不在 bank6」主张**不充分更正**（复核
> 0cfee11b）：CHR-RAM 运行时写入/其他 bank 窗口映射未排除——后经 §29.5 反转实证排除
> （CHR 同=无运行时写差）；②取证限更正：emu ppu.hex **有**（dumpPpu 采集点必调——
> analysis/traces/ 落点）；③**"E" 字形定位**：font tile 14（`FE C0 C0 FC C0 C0 FE`）
> =CHR 0x10E0（FONT_COMPLETE dest $1000+14×16——$1000 表 tile 0x0E）——OAM tile
> 0x76→CHR 偏移映射错位（渲染层 tile 索引语义=下步锚）。

Evidence: ROM bank6:8760 字节（=SAMUS_GFX [0x760] 同）+FONT B4C0 数字起字节+Font_HUD
dest/size（0xA0 数字区）+"E"图案 bank6 全搜（NOT FOUND）+emu 无 ppu.hex（仅 png）。
Confidence: 90

> 【2026-11-06 反转定谳（§29.5——emu CHR 取证工具面实证）】emu ppu.hex
> （analysis/traces/case-001...f01200_end.ppu.hex——dumpPpu :232 $0000-$2FFF 全域✓）
> tile 0x76/0x7F/A0/A2 双表逐字节 vs rust chr**全同 ✓✓✓**——**CHR 数据同但渲染异**
> （emu "E" vs rust 块）⇒§28.3/§29「字形源/烤焙源差」**前提证伪**：差=**渲染层**
> （sprite 瓦片选择/图案表基（PPUCTRL bit3）/OAM attr/渲染管线）——非 CHR 数据。
> 字形源三选一定谳：③**别的机制=渲染层**（非运行时 CHR 写/非 bank 源）。修径=
> 渲染层（sprite 瓦片→图案表基选择/attr）对拍。⚠ 采集限更正：emu ppu.hex **有**
> （dumpPpu 采集点必调——analysis/traces/ 落点）——§29.2「无 ppu.hex」系检索位错
> （shots/ 而非 analysis/traces/）。

## 30. 渲染层 tile 索引对拍卡（2026-11-06——OAM 判别+停机报）

### 30.1 关键判别（件 1——emu OAM dump）
emu ram.hex OAM 缓冲（$0200+）：OAM+0x60 **tile=76** attr=01 x=18 / OAM+0x64
**tile=7F** attr=01 x=20 / OAM+0x68 tile=3A——**emu OAM tile=0x76/0x7F 与 rust
DataDisplayTbl 同 ✓**⇒「OAM tile 写入值差」**排除**。emu 从 tile 0x76 渲 "E"——
但 CHR 两表（$0000=块/$1000=杂形）0x76 均非 "E" ⇒**图案表基差候选**（PPUCTRL
bit3）与 **CHR dump 时序 vs 渲染时序**（运行时 CHR 写在 dump 后/渲染用先写值？）
两可余。
### 30.2 asm 先行（件 2——行号锚）
OAM tile 写入径：DataDisplayTbl（E1B9 tile 值（0x76/0x7F）→OAM 拷贝（E0C7））+
SPRWriteDigit（E173 tile=A|$A0）——**tile 值=0x76/0x7F ✓（asm 同 rust 同 emu）**；
PPUCTRL sprite 图案表位（bit3）写点+图案表基映射=下步锚（sprite tile→CHR 偏移语义
——0x76×16=0x760 vs "E" 在 0x10E0 映射径）。
### 30.3 停机报（件 5）
CHR dump 时序 vs 渲染时序（运行时 CHR 写序）/sprite 图案表基映射语义=更深渲染件。
本卡交付=OAM 判别（写入值差排除✓）+两可余钉定+asm 锚。

Evidence: emu ram.hex OAM+0x60-0x68（tile 76/7F/3A）+rust DataDisplayTbl（同）+
CHR 两表 0x76（块/杂形）+"E"=font tile14（0x10E0）+asm E1B9/E0C7/E173。
Confidence: 91

> 【2026-11-06 判别补（§30.2——映射公式差深挖卡）】①OAM 读法复核（434ec4a7
> convention 注）：OAM+0x60（dump 0x260）tile=0x76@y=21/x=18=HUD "E" 位 ✓；sprite8
> tile=0x0E@y=A0=他位（非 HUD）——「0x0E 假设」**排除**（OAM tile 值=0x76 双方同真）；
> ②**PPUCTRL 实录**：rust ctrl=0x93（bit3=0（OBJ 表 $0000）/bit4=1（BG 表 $1000））
> =asm :155 旗（**PPUCTRL_BG_1000|PPUCTRL_OBJ_0000**）✓ 自洽——tile 0x76→$0000+
> 0x760=块 ✓ rust 渲染自洽；③映射公式（tile×16+表基）三方（asm 旗/硬件语义/rust）
> 同 ✓；④运行时 CHR 写径=LoadGFX 仅 init 序（NMI PPUDATA=palette/VRAM 非 CHR）。
> **静态链全合**（OAM✓/CHR✓/PPUCTRL✓/映射✓）——emu 从 tile 0x76 渲 "E" 仍谜⇒
> **dump-vs-render 时序**（CHR 写序）或更深 PPU 渲染语义=停机报（件 6）。

> 【2026-11-06 PPUCTRL 补取证（§30.3——判别收窄到单一假说）】取证缺口修复径=
> **PPUCTRL_ZP（$FF 游戏镜像）ram.hex 直读**（无需工具修——dumpPpu ctrl 字段读失败
> 绕过）。**emu zp[$FF]=0x97 / rust=0x93——bit3（OBJ 表）双侧=0 ✓✓**（表基 $0000
> 同）⇒**图案表基假说彻底死**。⚠ 附带发现：bit2（VRAM 增量）emu=1(+32)/rust=0(+1)
> 差（0x97 vs 0x93 低位差——渲染面无关（增量=CPU VRAM 写向）——登记待考）。
> **矛盾收窄到单一假说**：dump-vs-render 时序（CHR[0x760] 渲染时刻 vs dump 时刻值差
> ——运行时 CHR 写序）——修径=时序验证（dumpPpu 采样点改造（帧内多次 dump）——
> tools/oracle 面（单独 commit））。

> 【2026-11-06 第五面定谳（§30.4——破案：非时序）】时序假说**证伪**（第五面直判）：
> ①调色板 3F10-3F1F 双侧**全同**✓（色差排除）；②**OAM 全扫（0x00-0xFF）：rust
> **零个 HUD tile**（0x76/0x7F/0x3A 无）vs emu OAM+0x60-0x6C 三 sprite 在 ✓✓**——
> **rust DisplayBar OAM 写丢失**（写未落/落位错/后写覆）⇒"块形"=HUD sprite 缺席下
> 背景瓦透出（非 tile 图案差！）——**矛盾收窄到可修点：DisplayBar OAM 写径**
> （SpritePagePos 游标/写落位/覆写序）。修径=uw_display_bar OAM 写路径考证+修+TDD
> （f1200 HUD OAM 双腿 0 差锚）。时序假说/CHR 面全排除（CHR✓/tile✓/PPUCTRL✓/调色板✓）。

> 【2026-11-06 §30.4 更正（复核 ad3d3cba——原地不删史）】「rust 零 HUD tile/写丢失」
> **误判更正**：系陈旧 dump 扫误判（run 间 OAM 落位漂移——非确定）。verifier 独立全扫：
> 双侧**均有** HUD tile——emu=sprite 20-22（OAM+0x50-0x58）tile 0x76/0x7F/0x3A@y=33
> vs rust=sprite 24-26（OAM+0x60-0x68）同 tile 同 y——**真差=OAM 落位差 0x10
> （SpritePagePos 游标差=emu 多 4 sprite（9 vs 5））**。定性收窄：**非写丢失，是
> 写落位/游标差**（修径方向（OAM 写径）正确——§31 游标定谳方向 ✓ 落点殊值以 verifier
> 为准（0x50 vs 0x60））。

## 31. DisplayBar OAM 写径修卡（2026-11-06——游标错位定谳+停机报）

### 31.1 全偏移扫定谳（件 2——决定性）
OAM 全偏移扫（步 1——非对齐！）：emu HUD tile 在 **0x61/0x65/0x69/0x6D**（对齐
sprite 0x60/0x64/0x68/0x6C ✓）vs **rust 在 0x0B/0xF3（错位散）**——**SpritePagePos
游标错位/殊值**（emu=0x60 对齐 vs rust=0x0A 级错位）⇒HUD 写落错位→跨 sprite 边界
→渲染散/灭（"块形"=错位 sprite 跨界垃圾+背景透出）。
### 31.2 asm/rust 对照（件 1-2）
DisplayBar（E0C1）：SpritePagePos（$5B）游标→40B 拷贝（E0C7）→$5B←游标+40→
SPRWriteDigit（E173 Xplus4）；rust uw_display_bar（2681-）写径同构 ✓——**差在游标
值**（SpritePagePos 写者（sprite 写径）advance/对齐殊异）。尾清（CB6A 0xF4 填 y）不灭
tile ✓（tile 散在=写落位差非覆写）。
### 31.3 停机报（件 5）
游标管理（sprite 写径的 SpritePagePos advance/对齐——谁写 $5B/advance 4 序）=中件。
本卡交付=全偏移扫定谳（游标错位——非猜）+写径对照+修径明确（游标管理修+TDD）。
> 【2026-11-06 修径补（复核 ad3d3cba）】①游标差 0x10 来源=S**pritePagePos 初值/
> 推进差**（emu 多 4 sprite×4B=0x10——谁多写 4 sprite=写者差）；②emu 多 4 sprite
> （9 vs 5）=写者差考证（asm 写者序 vs rust——哪个写者缺/多）；③修径=游标管理+
> 写者差双修。

Evidence: OAM 全偏移扫（emu 0x61-0x6D 对齐 vs rust 0x0B/0xF3 错位）+DisplayBar E0C1/
E0C7/E173+rust uw_display_bar 2681-+尾清 CB6A（tile 不灭）。
Confidence: 92

## 32. 游标+写者双修卡（2026-11-06——写者全谱+游标差源钉定+停机报）

### 32.1 SpritePagePos 写者全谱（件 1——行号锚）
①**UpdateWorld（CB29:2497）帧首 `ldx #$00/stx SpritePagePos`（游标归零）**；
②精灵写者推进（DrawSpriteObject 尾 6353 stx）；③DisplayBar（E0C1:6674 stx——HUD
落游标位）；④AddTanks（E16C:6756 stx）；⑤fade-in init（2233——Samus 起 Sprite00）。
序=UpdateWorld 归零→游戏精灵写者推进→DisplayBar HUD→tanks。
### 32.2 游标差源（件 2）
游标差 0x10=**rust 前段多 4 游戏 sprite**（HUD 落 0x60=24 sprite vs emu 0x50=20）——
**sprite 写者产数差**（obj_draw_frame 精灵数/敌写者——emu 9 vs rust 5 中 HUD 段+前段
game sprite 数双重差）。⚠ §30.4/§31 陈旧 dump 扫注已收（verifier 落点为准）。
### 32.3 停机报（件 4）
sprite 写者产数差（obj_draw_frame 精灵条目数/敌写者 OAM 条数）+游标管理=中件。本卡
交付=写者全谱（5 写者行号锚）+游标差源（前段多 4 sprite）钉定+修径（产数差+游标双修）。

Evidence: UpdateWorld:2497（归零）+6353/6674/6756（stx 三写者）+2233（init）+游标差
0x10 算（rust 0x60=24 vs emu 0x50=20）+verifier OAM 全扫（emu 0x50-0x58 vs rust
0x60-0x68）。
Confidence: 91

## 33. sprite 写者产数差考证卡（2026-11-06——OAM 前段对拍定谳+停机报）

### 33.1 双侧 OAM 前段对拍（件 2——决定性）
f1200 OAM 0x00-0x4F 逐条对：**rust 前段=20×同 sprite 星印**（tile 0x6E/y=17/attr=01
/x=4E→0x90 递减 0x0A——IntroStar 恒印）vs **emu=富 sprite 集**（萨姆斯 8 条 CA/CB/DA/
DB@y=45/3D+敌 8 条 0E/0D/1E/1D/2E/2D/3B/3C@y=A0/B0/B8+HUD 数字 A2 等）——**rust 游戏
sprite 写者输出缺/被星印占**⇒产数差根=**星印写者 vs 游戏写者输出面**（非条目数微差）。
### 33.2 asm 对照（件 1——行号锚）
DrawSpriteObject（DE4A-DF19 帧流条目/终止判）+UpdateAllEnemies（F345 敌写者）+星印
（LoadStarSprites/IntroStarsData——98AE/$6E00 源面）——写者全谱 §32.1 基上产数面。
### 33.3 停机报（件 5）
星印写者/游戏写者输出面（obj_draw_frame OAM 落位/星印覆写序）=更深件（条目数差源于
动画帧流/星印写序——非微差）。本卡交付=OAM 前段对拍定谳（星印占位 vs 富 sprite 集
——非猜：逐条实录）。

Evidence: OAM 0x00-0x4F 逐条对拍（rust 20×0x6E 星印 vs emu 富集——全 DIFF 实录）+
星印源（IntroStarsData $6E00）+写者全谱 §32.1。
Confidence: 92

## 34. 星印写者停写时机考证卡（2026-11-06——双侧判别+停机报）

### 34.1 双侧判别（件 2）
GameMode（zp $1D）**双侧同=0x00** ✓+IntroStarOffset（$C3）双侧同=0x00 ✓——
「星印停写条件（GameMode 转换）差」**排除**（停写条件同）⇒OAM 前段差≠停写时机差。
rust OAM 前段=20×tile 0x6E 重复（§33）vs emu 富集——**差在 sprite 写者输出面**
（obj_draw_frame 帧数据流→OAM tile 落值——tile 0x6E 恒印=帧数据读/条目产数面）。
### 34.2 asm/rust 对照（件 1——行号锚）
星印（LoadStarSprites 98AE/IntroStarsData $6E00——title_state.rs STAR_SPRITE_SRC
同源）；游戏写者（DrawSpriteObject DE4A-DF19/UpdateAllEnemies F345）——写者全谱
§32.1/§33.2 基上停写/起写判据面。
### 34.3 停机报（件 5）
sprite 写者输出面（obj_draw_frame 帧数据流→OAM tile 落值语义——条目数/帧流数据/终止
判）=更深件（动画帧数据流语义——件 5 情形）。本卡交付=双侧判别（GameMode 同排除
停写差——非猜）+输出面定性。

Evidence: zp[$1D]/[$C3] 双侧同（0x00/0x00）+OAM 前段对拍（§33 逐条）+STAR_SPRITE_SRC
（title_state.rs）+写者全谱（§32.1/§33.2）。
Confidence: 91

> 【2026-11-06 判别补（§34.2——帧数据流输出面考证卡一击）】①**星印写者没停** ✓
> （非游戏写者写错）：title/mod.rs:641 `w.push((0x0201, 0x6E))`+x 递减 −0x0A 星位形
> =星印写者输出实证（OAM 前段 20×0x6E=其 stamps）；②**矛盾点**：GameMode=0x00
> （游戏态）但星印写者仍写 OAM——**停写 gate 缺/未接**（title 模块星印写者在游戏态
> 仍调）；③修径=星印写者停写 gate（GameMode/状态判）+TDD（f1200 OAM 前段=游戏集）。

> 【2026-11-06 归因细化（§34.3——停写 gate 修卡 asm 先行）】§34.2「title:641=星印
> 写者」**细化更正**：title:641=menu_choose 菜单光标写（单 sprite 0x0201←0x6E）；
> title:1283/1314=密码屏光标写（0x0211/0x0221←0x6E）——**20×0x6E stamps=星域拷贝**
> （IntroStarsData $6E00 数据 tile 0x6E×20 星位形 x 递减 0x0A——STAR_SPRITE_SRC
> title_state.rs:129 源面）——星域写者=**$6E00→OAM 拷贝径**（LoadStarSprites 族
> 端口）停写 gate 待接。修径=星域拷贝径停写 gate（GameMode/TitleRoutine 判）——
> asm 星域写者停写条件考证（IntroStar 族）+rust 调用链核（谁调星域拷贝）=中件。

## 36. asm 星域停写条件考证卡（2026-11-06——asm 面+调用链核+停机报）

### 36.1 asm 面（件 1——行号锚）
星载=IntroStar 例行（prg0:172-173 `jmp LoadStarSprites`/378-379（「useless, stars
are already on the screen」）——**title 期载入**）+星色调（1436-1458 IntroStarOffset
16 帧轮）——**停写=TitleRoutine 例行序**（title→game 转换后星载不再调——emu 星印
停=例行序自然止）。LoadStarSprites（98AE）=$6E00→OAM 拷贝径（IntroStarsData 源）。
### 36.2 rust 调用链核（件 2）
主循环（mainloop.rs）**无星印直调**——星印写者链=title 状态机深层（MenuOutcome.writes
应用径——title 模块写面被游戏帧应用？）——**GameMode 分派漏 gate 环待钉**（谁在游戏态
应用 title 写面）。
### 36.3 停机报（件 5）
rust 星印写者调用链（title 状态机→writes 应用径——游戏帧应用面）=中件深挖。本卡
交付=asm 面（title 期载入+例行序停写）+调用链核（主循环无直调——链在状态机深层）。

Evidence: prg0:172-173/378-379（星载）+1436-1458（星色调）+98AE（LoadStarSprites
$6E00 拷贝）+mainloop.rs 无星印直调（grep 实录）+title 状态机 writes 面。
Confidence: 90

## 37. title 写面应用链深挖卡（2026-11-06——链全映射+gate 反证+停机报）

### 37.1 链钉定（件 1——行号锚）
应用链全映射：title/step_glue.rs:83/139/196/286（writes→events）→**memmap/
title_state.rs:243 apply_title_step（应用点——RAM/WRAM 写）**→frame/title_bridge.rs:
:118（桥应用）。产出侧：title/mod.rs MenuOutcome.writes/OrchestrationOutcome.writes。
### 37.2 gate 反证（件 2——GameMode 分派 gate ✓ 已存在）
**go_main_routine（mainloop.rs:3021——asm GoMainRoutine C114）**：`GameMode≠0 ⇒
main_title_routine（标题路）/GameMode=0 ⇒ game_main_routine（游戏路）`——**gate ✓
存在**⇒星印非 title 写面（gate 挡——§34.2/34.3「星域拷贝径」归因**再证伪需**：
OAM 前段 tile 0x6E×20 源再开（游戏写者帧流（obj_draw_frame）候选——§33 帧数据流
语义）。
### 37.3 停机报（件 5）
OAM 前段 tile 0x6E×20 源=游戏写者帧流 vs 其他（title 写面已被 gate 排除）——
**obj_draw_frame 帧数据流→OAM tile 落值语义深挖**（§33 域）=下步。本卡交付=链全映射
+gate 反证（title 写面排除——非猜：go_main_routine 逐行）。

Evidence: step_glue.rs:83/139/196/286+title_state.rs:243+title_bridge.rs:118+
go_main_routine mainloop.rs:3021（GameMode gate 逐行）+MenuOutcome.writes 产出面。
Confidence: 91

## 38. 游戏写者帧流深挖卡（2026-11-06——写者 trace 一击+第三写者收窄+停机报）

### 38.1 写者 trace 一击（件 1——[oamw]/[oamw2] 探针）
obj_draw_frame（sprite.rs:150）：写**敌 sprite**（OAM 0x0220-0x0240=精灵 8-16——tile
0E/0D/1E/1D/2E/2D/3B/3C/17）**与 emu 同位同 tile ✓**（游戏写者对！）；draw_enemy
（:387）：窗内无写。**OAM 前段 0x00-0x4F 的 0x6E×20 stamps=第三写者**（非 obj_draw_
frame/draw_enemy——§33/§34 归因再收窄）。
### 38.2 第三写者候选（件 2）
apply_title_step（title_state.rs:243）sprite_copy 事件（$6E00→OAM）/其他 OAM 写者
——**GameMode gate（§37.2）挡 title 路**⇒sprite_copy 候选亦受 gate 制——第三写者
径待钉（OAM 写点全谱 grep）。
### 38.3 停机报（件 6）
第三写者径（OAM 0x00-0x4F 写点全谱——谁写 0x6E×20）=中件深挖。本卡交付=写者 trace
一击（obj_draw_frame=敌 sprite 对✓/draw_enemy 无写/0x6E=第三写者——非猜：探针实录）。

Evidence: [oamw] 探针（obj_draw_frame 0x0220-0x0240 敌 tile 9 条/iter——与 emu 同）+
[oamw2]（draw_enemy 窗内无写）+0x6E 前段 stamps 无归因（两路探针零命中）。
Confidence: 91

## 39. 第三写者径考证卡（2026-11-06——OAM 写点全谱+一击续+停机报）

### 39.1 OAM 写点全谱（件 1——行号锚）
全谱 grep：boot.rs:144/mainloop.rs:1785-1788（**敌帧流 OAM 环**——EN_FRAMES/EN_PLACES
——sprite.rs:387 非此路！）/2670-2751（DisplayBar 族）/sprite.rs:150（obj_draw_frame）
——排除 obj_draw_frame（敌 sprite✓§38）+五写者（§32.1）后**候选=mainloop 敌帧流环
（1785）**。
### 39.2 一击续（件 2——[oamw3] 探针）
mainloop 敌帧流环写**萨姆斯 8 sprite**（CA/CB/DA/DB/CC/CD/DC/DD@OAM 0x0200-0x01C）
**与 emu 同 ✓**（游戏写者全对！obj=敌✓/敌环=萨✓）——但 dump=0x6E stamps ⇒**stamps=
后段写者**（探针窗后/帧相位后）覆写 CA/CB ——0x6E 写者=**第三写者径仍待钉**（窗后
写点——OAM 全谱余点：boot:144/DisplayBar 族外）。
### 39.3 停机报（件 6）
0x6E stamps 写者（窗后/帧相位后写点——OAM 全谱余点+探针窗扩展）=中件深挖。本卡交付=
OAM 写点全谱+一击续（敌环=萨✓——非猜：[oamw3] 实录）+第三写者径再收窄。

Evidence: OAM 写点全谱 grep（boot:144/1785-1788/2670-2751/150）+[oamw3] 探针（敌环
萨 8 sprite CA/CB...@0x0200-0x1C 与 emu 同）+dump 0x6E stamps（两路探针零命中——
窗后写者）。
Confidence: 91

## 40. 窗后写者钉定卡（2026-11-06——apply_title_step 一击定谳+gate 缺口）

### 40.1 一击定谳（件 1——[oamw4] 探针 560 写实录）
**apply_title_step（title_state.rs:243）=0x6E 写者 ✓✓✓**（0x0201←0x6E 多次=
menu_choose 光标写（title:641 `w.push((0x0201, 0x6E))`）经 events 应用！）——§39
「窗后写者」钉定=**apply_title_step 应用径**（非游戏写者/非帧末他者）。
### 40.2 gate 缺口（件 2-3）
**矛盾**：GameMode=0x00（游戏态——§34.1/§37.2 gate）但 apply_title_step **仍写 OAM**
（560 写实录）⇒**gate 覆盖缺口**：go_main_routine gate（GameMode≠0⇒title 路）挡
main_title_routine 入口，但 **apply_title_step 经他径调用/写面在 gate 后仍应用**——
gate 面核（谁在游戏帧调 apply_title_step）=修 gate 靶。
### 40.3 停机报（件 6 余+修径）
修径=**gate 覆盖补**（apply_title_step 调用径 gate/游戏帧应用面判）+TDD。本卡交付=
一击定谳（apply_title_step=0x6E 写者——非猜：560 写实录）+gate 缺口定性。

Evidence: [oamw4] 探针（apply_title_step 560 OAM 写——0x0201←0x6E 多次+0x0200-0x0B
←F0）+menu_choose title:641（0x0201←0x6E 源）+go_main_routine gate（§37.2）。
Confidence: 92

## 41. gate 覆盖补修卡（2026-11-06——窗过滤实录+gate 生效+源再开停机报）

### 41.1 窗过滤实录（件 2——[oamw4] 窗过滤）
**f1200 期（DIAG_ITER 1175-1195）apply_title_step OAM 写=零** ✓——**gate ✓ 生效**
（go_main_routine gate 挡 main_title_routine——游戏帧 apply_title_step 不调）——
§40「gate 缺口」**证伪更正**（无窗探针 560 写=title 期全程——窗过滤实录）。560 写
=menu_choose 光标写（title 期正常应用 ✓）。
### 41.2 0x6E stamps 源再开（件 2）
dump（f1200）=0x6E×20 stamps vs 探针（1175-1195）游戏写者写 CA/CB（敌环=萨✓/obj=
敌✓）——**探针时写值 ≠ dump 值**⇒**时序/dump 面**（写者输出后被覆/dump 采样点 vs
写点）=深件（件 5 情形）。
> 【2026-11-06 n/f 校准更正（§41.4——n/f 对齐+dump 时序卡）】DIAG_ITER=**行号**
> （state rows 1182 行 f=1-1200 含 6 gaps（掉帧）——f1200=行 1181）**非帧号**——
> 窗 1175-1195=行≈f1183-1200（**f1200 含 ✓**）——§41.1「f1200 期零写」窗**本就对**
> ✓（gate 生效结论保持）。新窗 1196-1205=行越界（三探针全零=窗外非无写——校准
> 教训域：n/f 口径先校准）。0x6E stamps 源仍开（探针写 CA/CB vs dump 0x6E——第四
> 写者/时序待钉）。
### 41.3 停机报（件 5）
0x6E stamps 源（dump-vs-写点时序——谁在 f1200 后覆写 CA/CB→0x6E）=中件深挖。本卡
交付=窗过滤实录（gate 生效✓——§40 更正）+源再开定性。

Evidence: [oamw4] 窗过滤（1175-1195 零写——gate 生效）vs 无窗 560 写（title 期）+
[oamw]/[oamw3] 游戏写者（CA/CB@0x00-0x1C/敌@0x220+）vs dump 0x6E stamps。
Confidence: 91

## 42. 第四写者/dump 时序卡（2026-11-06——生命周期 trace 升维+dump 语义面停机报）

### 42.1 生命周期 trace（件 1-2——升维（16 层链建议采纳）——[oamlife] 9360 写）
write_ram 钩子 OAM 全写序 trace（行 1175-1182=f≈1183-1200 含 f1200）：
①前段（sprite 0-19=0x0200-0x024F）写 **CA/CB/DA/DB/CC/CD/DC/DD（萨 8）+0E/0D/1E/
1D/2E/2D/3B/3C（敌 8）富集 ✓**（游戏集对——与 emu OAM 前段同！）；②**0x6E 写在
sprite 27+**（0x026D 起步 4——20 stamps）。
### 42.2 dump 语义面（件 1——dump≠trace 写位）
dump（f1200 .ram.hex）前段=0x6E stamps vs trace 前段写=游戏集——**dump≠trace 写位**
⇒**dump 语义面**（采样点/映射（dump 读的是哪段 OAM？iram 偏移？））=下步（非写者
径——写序全程对 ✓）。⚠ iram 偏移疑：dump 的 OAM 位=iram[0x100+]（基 $0100）vs
trace 写=0x0200+（ram 基）——**偏移口径差候选**（dump 解析偏移）。
### 42.3 停机报（件 6——升维面收口）
dump 语义面（采样点+偏移口径）=中件（工具面/oracle）。本卡交付=生命周期 trace 升维
（整帧写序全程——16 层链升级建议采纳）+dump 语义面定性（非猜：9360 写实录）。

Evidence: [oamlife] 9360 写全程 trace（前段游戏集 ✓+0x6E@sprite27+）+dump 前段 0x6E
+iram 偏移口径注（$0100 基 vs 0x0200 裸址）。
Confidence: 91

> 【2026-11-06 dump 偏移口径核定谳（§42.4——dump 偏移口径核卡）】①dump 口径 ✓
> **无假差**：dump[0x200-0x20F]=trace 末写**逐字节同**（17 6E 01 4E...）——dump=
> trace 末写态 ✓（offset 0x200 基双侧同）；②**png-OAM 一致 ✓**（rust png 块形=0x6E
> stamps 渲染✓/emu png "EN"=HUD sprite 0x76/0x7F✓）；③**0x6E stamps=游戏写者后
> 覆写者**（y=17/x 递减星域形——覆盖前段游戏集+HUD 区 0x50-0x68）——**第四写者
> 真存在**（§34 星域形归因方向 ✓）——写者身份（call-stack 探针）=下步。

## 43. call-stack 写者身份卡（2026-11-06——AddTanks 定谳+修靶）

### 43.1 call-stack 一击（件 1——[w6e] backtrace 2056 实录）
0x6E 写者=**uw_display_bar:2749（AddTanks 能量罐显示！）** ✓✓✓——「星域形」
（§34/§42）**误归因更正**（y=17/x 递减=**能量罐排显示**（LE14A 罐位形）非星域）。
罐写 OAM sprite 0-19（**覆前段游戏集**）+sprite 27+——罐位/游标差=修靶。
### 43.2 修靶（件 2-3）
AddTanks 罐位（X=游标——LE14A「右→左 x=0x40 起」）vs rust 罐写落位（sprite 0-19
=游标 0 起？）——**罐位差**（emu 罐在 HUD 区 vs rust 罐在游戏集区）⇒修=AddTanks
游标/罐位（asm LE14A 对读）+TDD。asm 对应：emu 罐位（HUD 区 0x50+）无此覆写 ✓。
### 43.3 停机报余
修+TDD（f1200 OAM 前段游戏集+罐位同）=下卡（修径明确：AddTanks 游标/罐位）。本卡
交付=call-stack 一击（AddTanks——非猜：backtrace 2056 实录）+误归因更正+修靶。

Evidence: [w6e] backtrace（uw_display_bar:2749 AddTanks——2056 写实录：0x0201-
0x024D sprite 0-19+0x026D+ sprite 27+）+LE14A 罐位（asm）+emu OAM 罐位（HUD 区）。
Confidence: 92

## 44. AddTanks 罐位修卡（2026-11-06——asm/rust 对照+游标矛盾停机报）

### 44.1 asm 全径（件 1——行号锚）
AddTanks（**LE14A**）：`ldx SpritePagePos`（游标）→EnergyTankCount→x=0x40（右→左）
→tile 0x6F（满）/0x6E（空）→AddOneTank（**E17B**：y=0x17/tile/attr=01/x+Xplus4 推进）
→`stx SpritePagePos`（收游标）。罐位=**游标起 + 每罐 4B 推进** ✓。
### 44.2 rust 对照（件 2）
rust AddTanks（mainloop:2734-）：`x=read(0x005B)`（游标——asm 同构✓）+罐写
（y=0x17/tile/attr=01/x-0x0A 右→左✓）+Xplus4✓——**游标语义同构 ✓**。⚠ 矛盾：rust
罐落 sprite 0-19（游标=0x00-0x4C）vs asm 游标=游戏写者推进后位——**游标=0x00 矛盾**
（游戏写者（obj_draw_frame/敌环）应推进游标至 HUD 区）⇒**游标推进链疑断**（谁写
$5B/推进序——§32.1 写者全谱的推进面）。
### 44.3 停机报（件 5）
游标推进链（UpdateWorld 归零→游戏写者推进→DisplayBar 游标值）=中件（rust 游标在
AddTanks=0x00 的推进断点）。本卡交付=asm 全径+rust 同构对照+游标矛盾定性。

Evidence: LE14A/E17B 全径+rust mainloop:2734- 同构对照+罐落 sprite 0-19（backtrace
§43）+游标=0x00 矛盾。
Confidence: 91

## 45. 游标推进链卡（2026-11-06——$5B trace+回绕定谳+修靶）

### 45.1 $5B 推进 trace（件 1——[cursor] 48 写实录）
每 iter $5B 序=**00→10→20→44→6C→68**（6 写：UpdateWorld 归零/写者 0x10/0x20/
DisplayBar 表拷 0x44→0x6C（+40）/收 0x68）——推进链 ✓ 在（非断）。
### 45.2 回绕定谳（件 2——罐数过大致 u8 回绕）
罐 loop（AddTanks x+=4 u8）从 0x6C+4 推进 **0xFF→0x00 回绕**→罐落 sprite 0-19
（0x00-0x4C 覆游戏集！）——**罐数（0x6877 EnergyTankCount）过大致 20+ 罐回绕**
（asm 罐数=小（1-5）不回绕）⇒修靶=**罐数源（0x6877 vs asm EnergyTankCount 地址）
或罐 loop 回绕保护**。
### 45.3 停机报余
修+TDD（罐数对/回绕护——f1200 OAM 前段游戏集+罐位+HUD 全同）=下卡（修径明确：
罐数源考证+回绕）。本卡交付=$5B trace（推进链✓）+回绕定谳（非猜：序+backtrace 罐位）。

Evidence: [cursor] 48 写序（00/10/20/44/6C/68）+[w6e] backtrace 罐位（0x6C+→回绕→
0x00-0x4C）+asm EnergyTankCount（constants）+rust 0x6877 罐数源。
Confidence: 91

## 46. 罐数源考证+回绕保护修卡（2026-11-06——罐数实录+矛盾停机报）

### 46.1 罐数实录（件 1）
rust $6877（wram[0x877]）=**0**（f1180-1200 全零实录）——EnergyTankCount 地址 ✓
（constants:1208 对）；emu trace f1200 w=watch 面（无 6877 watch——取证限）。
### 46.2 矛盾（件 1-2）
**罐数=0 但 [w6e] backtrace 有罐写**（AddTanks:2749）——rust AddTanks `if tanks==0
{return}`（mainloop:2737）应止——**罐写条件/读径矛盾**（0x6877 读点/时序/读径（WRAM
映射））⇒罐数源考证断点（「20+ vs 1-5」差根未达——罐数 0 与罐写矛盾先解）。
### 46.3 停机报（件 5）
罐写条件矛盾（0x6877 读=0 但写发）=中件（读点/时序/读径）。本卡交付=罐数实录+矛盾
定性（非猜：state wram 实录+backtrace 对照）。

Evidence: rust state wram[0x877]=0（f1180-1200 实录）+[w6e] backtrace 罐写（AddTanks
:2749）+rust AddTanks if tanks==0 return（2737）+EnergyTankCount $6877（constants
:1208）。
Confidence: 91

## 47. 罐写条件矛盾解卡（2026-11-06——读径映射错破案+全链根定谳）

### 47.1 一击破案（件 1-2——[tank] 探针）
AddTanks tanks 读（mainloop:2737 read(0x6877)）=**255（0xFF 垃圾！）** vs state
wram[0x877]=0——**读径映射错**（read_ram(0x6877)≠wram[0x877]——地址换算/映射径差）
⇒**255 罐→u8 回绕→覆前段游戏集+HUD=全链根**（§33-46 全链收口！）。
### 47.2 全链根（件 2——16+ 层链终）
读径映射错（0x6877 读=垃圾 255）→罐数 255→回绕覆写→OAM 前段/HUD 覆→png 块形/EN
差——**根=读径映射**（read_ram 对 0x6877 的映射 vs wram[0x877]）。修靶=read_ram
映射修（0x6877→wram[0x877]）+TDD。
### 47.3 停机报余
修+TDD（读径映射——f1200 OAM 全段双腿 0 差锚）=下卡（修径明确：read_ram 映射）。本卡
交付=一击破案（255 垃圾读——非猜：[tank] 实录+state 对照）+全链根定谳。

Evidence: [tank] 探针（read(0x6877)=255 实录）vs state wram[0x877]=0（对照）+
EnergyTankCount $6877（constants:1208）+全链（§33-46）收口。
Confidence: 92

## 48. read_ram 映射修卡（2026-11-06——全链根修+f1200 达 match）

### 48.1 根修（件 1-2——read_ram WRAM 分派）
read_ram=`ram[addr&0x7FF]`（2KB 镜像归一——**WRAM 区未分派**）——0x6877&0x7FF=0x077
→ram[0x077] 垃圾 255 ✓（§47 根）——**修=WRAM 分派**（$6000-$7FFF→wram[addr-0x6000]
——read_ram/write_ram 同族修（分派非点修））+TDD（read_ram_wram_dispatch_6877 ✓）。
### 48.2 f1200 达 match（件 3-4——判据全过）
修后 f1200：**OAM 全段 0/256 差 ✓✓✓**（游戏集 CA/CB+HUD 0x76/0x7F/0x3A+罐位全对）
+**png 0/61440 差 ✓✓✓**——**f1200 达 match**（1015px→0——HUD 簇全消！）——
L4 增益（102→103）待锁；503 测绿（462+3+1 过）；防回退面（morphball 七格）待复核。
### 48.3 全链收口（18 层链）
§12-48 全链：BCD 语义→HUD 显示→CHR→OAM→游标→罐→读径映射——**根=read_ram WRAM
分派缺失**（一处修全链消）。判据全过 ✓（f1200 match）。

Evidence: read_ram 原体（& 0x7FF）+WRAM 分派修+TDD 读测+f1200 OAM 0/256+png
0/61440 双零实录。
Confidence: 93

## 49. 萨姆斯受伤行为对拍卡（2026-11-06——asm 全谱+双侧对拍+行为差定谳）

### 49.1 asm 受伤径全谱（件 1——行号锚）
①**CheckHealthStatus（$CDFA——:3082-3180）**：isHit bit5(0x20)⇒InvincibleDelay←
0x32（50 i 帧）+KnockbackDir←0xFF/方向（isHit bit3>>3）+speedY←0xFD/AccelY←0x38
（击退！）+IsSamusDead 判→i 帧递减+击退 X 速（floor(i/16)）+闪烁（偶帧 animFrame←
FrameNone）+蜂鸣（血<17）；②**IsSamusDead（$CE84——:3189）**：status==sa_Dead/
Dead2 判；③**SubtractHealth（$CE92——:3198-）**：HealthChange 判+IsSamusDead 免伤
+Varia 半伤+Base10Subtract+死判（BCD 0⇒sa_Dead）；④SamusCollisionWithSolidEntities
（$D976——:5117）=站敌/电梯判（非 isHit 设点）；⑤isHit 设点=敌接触伤径（F311
SamusHurt 族）。
### 49.2 双侧对拍（件 2——killenemy 场景）
**行为差定谳**：rust f986 伤害事件（(0,3)→(32,2)=30→22 **-8**（BCD 交换形✓§22））
vs **emu 无伤害事件**（健康 (0,3) 恒）——**rust 碰敌触发伤害/emu 未触发**——
敌接触触发差（场景位移差（rust/emu Samus 位不同）或碰撞径差）。
### 49.3 判据（件 4）+停机报
受伤行为双腿逐帧 0 差**未达**（rust 有伤害 emu 无——**行为差**）——按任务纪律
**只对拍不修**：归因候选=敌接触触发（场景位移/碰撞径）——修径待归因定谳（asm
碰撞径（CheckCollisionOfXSlotAndYSlot 族）考证）。crawlprobe/collide 场景双侧均
无伤害（健康恒）✓（对照）。

Evidence: asm CDFA:3082-3180/CE84:3189/CE92:3198/D976:5117 全谱+killenemy 双侧
健康序列（rust f986 30→22 vs emu 恒 (0,3)——逐帧实录）+crawlprobe/collide 对照
（无伤害）。
Confidence: 91

## 50. 敌接触触发差归因卡（2026-11-06——位置定谳+碰撞径锚）

### 50.1 f986 窗位置对拍（件 1——候选①排除）
f982-991 双侧 Samus **y=176/x=128/status=255 全同 ✓✓✓**（逐帧实录）——**位置差无**
⇒**候选①（场景位移差）排除** ⇒**候选②（碰撞径差）** ✓。健康面：双侧 f982-985=
(0,3)（30）同 ✓；f986 起 rust=(32,2)（22——伤害！）vs emu 恒 (0,3)——触发差 ✓
（§49 复证）。
### 50.2 碰撞径 asm 锚（件 2——行号锚）
GetRadiusSumsOfEnXSlotAndObjYSlot（**prg7:5135**——半径和）+CheckCollisionOfXSlot
AndYSlot（**5136**——判定）+GetRadiusSumsOfObjXSlotAndObjYSlot（5821）+SamusHurt010F
（**9898**——isHit 设点族）——判定算式（radius 和比较/接触盒语义）深挖+rust 碰撞径
对读=修径（下步）。
### 50.3 停机报余
碰撞径算式深挖（asm 逐行）+rust 对读+修+TDD（f986 窗伤害事件双腿 0 差）=下卡。本卡
交付=位置定谳（候选①排除——非猜：f982-991 逐帧全同实录）+碰撞径锚。

Evidence: f982-991 双侧 y/x/status 逐帧全同实录+健康序列（rust f986 (32,2) vs emu
(0,3) 恒）+GetRadiusSums/CheckCollision/SamusHurt010F 行号锚（5135/5136/9898）。
Confidence: 92

## 51. 碰撞径算式深挖卡（2026-11-06——asm 算式全谱+rust 对读+触发径定性）

### 51.1 asm 算式全谱（件 1——行号锚）
**GetRadiusSumsOfEnXSlotAndObjYSlot（F1BF——prg7:10147）**：敌 radiusY+萨 radiusY
→Temp04/敌 radiusX+萨 radiusX→Temp05（**半径和**）；**CheckCollisionOfXSlotAndYSlot
（F1FA——10200）**：|dy|（Temp07-Temp06）**cmp Temp04 和/bcs 不触**+|dx| 同式——
**判定=|dy|<radiusY 和 && |dx|<radiusX 和** ✓；SamusHurt010F（9898——isHit 设点族）。
### 51.2 rust 对读（件 2）
rust samus_collision_with_solid_entities（physics.rs:665）：**仅冻结敌环（D983 status
==4）+DIVERGENCES 占位（0x0D983）**——**敌接触伤害触发（isHit 设点）未在此径**——
f986 触发源=他径（SamusHurt010F:9898 族端口面）——「同位置判定差」的算式差未及
（触发径定位先）。
### 51.3 停机报（件 6）
isHit 设点径（SamusHurt010F:9898 族——f986 触发真源）+radius 值源（H3 敌槽面关联
——radius 写撤回回装）=中件。本卡交付=asm 算式全谱（radius 和+比较式）+rust 对读
（占位/触发径未及）。

Evidence: F1BF:10147（radius 和）+F1FA:10200（|dy|<和 && |dx|<和）+9898（isHit
设点）+physics.rs:665（冻结环+0x0D983 占位实录）。
Confidence: 91

## 52. SamusHurt010F 触发径考证卡（2026-11-06——触发径全谱+地址语义冲突定谳）

### 52.1 asm 触发径全谱（件 1——行号锚）
**CollisionDetection（F034——prg7:9900+）**：`lda #$FF/sta SamusKnockbackIsBomb/
sta SamusHurt010F`（**$010F 伤旗初值 0xFF**）→敌环（Mellows 等）→**IsSamusDead+
SamusInvincibleDelay 免伤判**（`bne skip`）→碰撞检（CollisionDetectionMellow_Check
WithObjectYSlot）→React（伤！）。**$010F=SamusHurt010F**（constants:819——"never
read. takes on different values depending on how samus was hit"）。
### 52.2 地址语义冲突定谳（件 2）
**WalkSoundDelay=$53**（constants:546）非 $010F！——**rust handler.rs:291/489 误标
0x010F=WalkSoundDelay**（地址语义冲突——$010F=SamusHurt010F 伤旗）；mainloop.rs:2474
`write_ram(0x010F, 0x80)`=伤旗写点（触发径候选）。f986 触发源=伤旗写径（0x010F
写点调用链）待深挖。
### 52.3 停机报余
f986 触发源（0x010F 写点调用链——rust 侧伤旗写径 vs emu）+handler.rs 地址语义冲突
修（$53 真义）+H3 radius 关联=中件。本卡交付=触发径全谱+地址语义冲突定谳（非猜：
constants:546/819 对表）。

Evidence: F034（CollisionDetection 触发径）+constants:819（$010F=SamusHurt010F）+
constants:546（WalkSoundDelay=$53）+handler.rs:291/489（误标实录）+mainloop.rs:2474
（0x010F←0x80 写点）。
Confidence: 92

## 53. 伤旗写径深挖卡（2026-11-06——触发条件实录+radius 值源差候选）

### 53.1 asm 先行（件 1——行号锚）
$010F 伤旗：CollisionDetection F034 初值 0xFF（constants:819「never read」注）→
React（伤）径写；f986 触发链=碰撞检→React→**LF306（F306:10433）伤害置径**（Health
Change←AreaEnemyDamage——§22 域）✓。
### 53.2 rust 触发条件实录（件 2——mainloop:2466-2480）
rust「Samus 5 伤害（:12214-12232）」：**InvincibleDelay==0+距离判（`dy<sry+4 &&
dx<srx+4`——+4 固定敌半径！）+screw 判→write(0x010F,0x80)+InvincibleDelay←0x32**——
**asm 算式（F1FA:10200）=敌 radiusX/Y 实值**（GetRadiusSums 和）vs **rust +4 固定**——
**radius 值源差候选 ✓（H3 敌槽面关联——radius 写撤回回装在案）**。
### 53.3 停机报余
radius 值源（敌 radius 实值 vs +4 固定——H3 回装卡关联归属）+F306 伤害置径对读=中件。
本卡交付=触发条件实录（非猜：mainloop:2466-2480 逐行）+radius 值源差候选定性。

Evidence: F034/F306:10433 行号锚+constants:819（never read）+mainloop:2466-2480
触发条件逐行（+4 固定实录）+asm F1FA 判定式（radius 实值）对照。
Confidence: 92

## 54. radius 值源对读卡（2026-11-06——radius 全同排除+触发求值深件）

### 54.1 数值实证（件 1——判别）
f984-989 双侧 radius **全同**：sRY=15/sRX=4/eRY=8/eRX=8 ✓（逐帧实录）——**radius
值源差排除**（「radius 值源差候选」（§53）证伪——radius 写源/值双侧同）。敌槽
（0x440/0x441=168/199→198...）双侧同 ✓（敌移动同）。
### 54.2 触发求值深件（件 2）
几何算式：dy=|176-198|=22 vs rust 阈（sry+4=19）**22>19 不应触发**；dx=|128-168|=40
>8 亦不应——**rust 却触发（f986 伤害！）**⇒**触发条件实际求值深件**（py/px 读径
（0x0440/0x0441 语义？）/比较式求值/其他触发径）——非 radius/非算式阈值。
### 54.3 停机报（件 6）
触发条件实际求值（py/px 读径语义+比较式求值链）=中件。本卡交付=radius 全同排除
（候选死——非猜：逐帧实录）+求值深件定性。

Evidence: f984-989 双侧 radius 全同实录（sRY=15/sRX=4/eRY=8/eRX=8）+敌槽同+几何
算式（22>19/40>8 不应触发）vs rust f986 实触（矛盾）。
Confidence: 92

## 55. 触发条件求值链深挖卡（2026-11-06——探针零命中+求值链定性停机报）

### 55.1 求值 trace（件 1——探针实录限）
[eval] 探针（mainloop:2466 触发判四输入 sy/sx/py/px/sry/srx/dy/dx）两轮实录
**零命中**（窗 1160-1182 误（n/f 校准教训——killenemy f986=行~970）+窗 960-995 亦
零——外层 if（InvincibleDelay==0）位/触发求值时点疑）——**探针调试深件**（触发判
求值时点/条件路径未及实录）。
### 55.2 求值链定性（件 2）
触发判四输入（py/px=0x0440/0x0441 读径）+求值时点（f986 帧）=**求值链深件**（探针
位/窗/条件路径）——radius 全同（§54）+几何矛盾（22>19 不应触 vs 实触）+探针零命中
三面收窄=**求值时点/条件路径**（py/px 读径语义或触发径分叉）。
### 55.3 停机报（件 5）
求值链（探针调试+py/px 读径语义+触发径分叉）=中件。本卡交付=探针实录限声明+求值链
定性（非猜：零命中实录+三面收窄）。

Evidence: [eval] 两轮零命中实录（窗 1160-1182 误/960-995 零）+radius 全同（§54）+
几何矛盾（§54.2）+py/px 读径（0x0440/0x0441）。
Confidence: 90

## 56. 第三写点径考证卡（2026-11-06——2543 径全谱+f986 触发实录）

### 56.1 mainloop:2543 径全谱（件 1——行号锚）
「敌×萨姆斯（asm :10009-10019——碰撞伤害）」：InvincibleDelay==0+判盒（ey/ex=
0x0400+x+0/1（敌 y/x）+ery/erx=wram 0x6AF4+x+1/2（敌 radius））+**hi 距调**（s_hi=
0x030C&1/e_hi=wram 0x6AF4+x+7&1——hi 差⇒±0x100）→**radius 和式**（dy_adj<sry+ery
/dx_adj<srx+erx）→F2ED 螺旋转判→SamusHurt($010F)←$80+LF270 击退+LF306 伤害
（AreaEnemyDamage→HealthChange）✓。**判定式=radius 和**（与 mainloop:2466 的 +4
固定式**不同**！）。
### 56.2 f986 触发实录（件 2）
此径 f986：dy=22 < sry+ery=23 ✓ **应触**（hi 同（双侧 0x030C/wram 0x6AF4+x+7 同）→
dy_adj=22）——**rust 触发 ✓ 合理**（radius 和式）；emu 未触⇒**emu 侧条件差**（i 帧
（InvincibleDelay≠0 skip）/hi/螺旋转/其他判）——emu 条件考证=下步。
### 56.3 停机报余
emu 侧条件差（i 帧状态/hi/螺旋转——f986 窗 emu InvincibleDelay/hi 实录）+2543/2466
双径语义（+4 固定 vs radius 和——2466 径疑冗余/误）=中件。本卡交付=2543 径全谱+
f986 触发实录（非猜：mainloop:2543 逐行+几何算式）。

Evidence: mainloop:2543-2570 逐行（radius 和式+hi 距调+三件链）+:10009-10019 asm 对应
+f986 几何算式（22<23 ✓）+双径语义（2466 +4 vs 2543 和式）。
Confidence: 92

## 57. emu 条件考证卡（2026-11-06——条件实录+i 帧/hi 排除+敌分支定性）

### 57.1 emu f986 窗条件实录（件 1）
emu：**InvincibleDelay=0 ✓/hi=0 ✓**（f982-989 逐帧——与 rust 同！）——**i 帧免伤
/hi 距调差排除** ✓。健康：emu (0,3) 恒/rust f986 (32,2)——触发差不变。
### 57.2 敌分支定性（件 2）
emu 未触=**敌碰撞分支候选**（CollisionDetection F034 敌环 type/status 判——该敌
（killenemy 场景）进不进碰撞检？）或**螺旋转**（F2ED skip）——asm 敌环分支考证
（敌类型/状态→碰撞检入径）=下步。
### 57.3 停机报（件 5）
敌碰撞分支（F034 敌环 type/status 判+该敌入径）+螺旋转判=中件。本卡交付=条件实录
（i 帧/hi 排除——非猜：逐帧实录）+敌分支定性。

Evidence: emu InvincibleDelay=0/hi=0 逐帧实录（f982-989）+rust 同+健康差（rust f986
(32,2) vs emu (0,3)）+F034 敌环（§52.1）。
Confidence: 91

## 58. 敌碰撞分支考证卡（2026-11-06——敌态全同排除+螺旋转候选）

### 58.1 asm 敌环分支全谱（件 1——行号锚）
CollisionDetection（F034）敌环（Lx269/F09D:9968+）：slot 空（beq NextEnemy）/
Explode（skip）/**Pickup（beq Lx274——skip 弹碰撞）**——type/status 判 ✓；敌×萨
（:10009-10019）→F2ED 螺旋转判（有效⇒RTS 不伤）。
### 58.2 敌态实录（件 2——分支排除）
f984-988 双侧敌 status=**[0,0,0,0,2,2] 全同**（slot 4/5=2 活——非空/非 Explode/
非 Pickup）⇒**敌分支排除**（双侧入碰撞检 ✓）——emu 未触=**螺旋转（F2ED）候选**或
碰撞内层（AreObjectsTouching/半径和求值）。
### 58.3 停机报（件 5）
螺旋转判（F2ED——emu f986 螺旋转态实录）+碰撞内层=中件。本卡交付=敌环分支全谱+
敌态全同排除（非猜：逐帧实录）+螺旋转候选定性。

Evidence: F034 Lx269/F09D 敌环分支（type/status 判）+f984-988 双侧敌 status 全同
实录+[0,0,0,0,2,2]+F2ED 螺旋转判（:10019 邻）。
Confidence: 91

## 59. 螺旋转+碰撞内层考证卡（2026-11-06——螺旋转排除+8 面全同+内层深件）

### 59.1 asm+实录（件 1-2）
F2ED 螺旋转判（is_screw_attack_active 三门：status==1+gear&8+anim(0x0E/0x0C)——
physics.rs:954）；**f983-988 双侧 screw=0/status=255 全同**（螺旋转排除 ✓）。
### 59.2 8 面全同（件 2——排除链收口）
位置（§50）/radius（§54）/算式（§56 和式）/i 帧（§57）/hi（§57）/敌态（§58）/
螺旋转（本卡）/健康条件——**8 面全同但触发异**（rust f986 触/emu 不触）⇒**碰撞内层**
（F1FA 求值——scroll dir/position hi/Temp 语义）=唯一余面。
### 59.3 停机报（件 5）
碰撞内层（F1FA 求值链——scroll dir/position hi/Temp10-11 语义）=中件（8 面排除后
唯一余）。本卡交付=螺旋转排除+8 面全同收口+内层定性。

Evidence: F2ED 三门（physics.rs:954）+f983-988 双侧 screw/status 全同实录+8 面排除
链（§50/54/56/57/58+本卡）。
Confidence: 91

## 60. 碰撞内层考证卡（2026-11-06——F1FA 全谱+2466 径定谳（Skree 弹伤））

### 60.1 asm F1FA 全谱（件 1——行号锚）
CheckCollisionOfXSlotAndYSlot（F1FA:10200-10261）全谱：y 轴（dy=Temp07-Temp06→
**cmp Temp04 radiusY 和/bcs 不触**）+x 轴（dx=Temp09-Temp08→**cmp Temp05 radiusX
和/bcs 不触**）+hi 距调（@positionHi_notEqual:10262——Temp01_DiffHi/Temp10_DistHi
位）+**scroll dir 分支**（Temp03_ScrollDir——水平/垂直房）✓。Temp 语义：Temp10_DistHi
（hi 距位）/Temp11_DistY（y 距）/Temp0F_DistX（x 距）——ZP 复用组（survey §5.2）。
### 60.2 2466 径定谳（件 2——asm :12214-12232 对应）
> 【2026-11-06 更正（复核 6a630a72——原地不删史）】「2466=Skree 弹伤/py/px=弹槽
> 0x0440」**误读更正**：0x0440=**敌槽**（Objects slot#4——constants_ram:941-943
> .enum $0400 16 slots×16B）；SkreeProjectiles=零页 **$A0-$AF**（4 槽×4B）**双侧全空**
> ⇒f986 无弹在场，弹碰撞不可能。**f986 触发源=敌碰**（2543 和式径或 2466 +4 式径——
> 同值场景两径可能同触发）。归因收口：emu 侧该径未执行（敌环迭代序/入径条件（bd481517
> 第三候选））或 2466/2543 执行面差异——转**动态 trace**（Mesen CPU trace f986 帧——
> emu CollisionDetection 执行实录看 skip 点）=下一步（静态 20 层穷尽）。
**mainloop:2466「Samus 5 伤害（:12214-12232）」=Skree 弹碰撞伤**（asm :12214-12232
实录：AreObjectsTouching（DC7F）+IsScrewAttackActive（CD9C）+SamusHurt_F311+
HealthChange←**0x50（5 伤！）**+SubtractHealth）——**+4=弹 radius**（非敌 radius！）
——**2466 径=Skree 弹伤**（py/px=0x0440/0x0441=弹槽 0x40 y/x）非敌碰（2543）。
**f986 触发源=弹碰撞（2466）**——emu 未触=弹条件差（弹位置/存在/接触判）。
### 60.3 停机报余
f986 弹碰撞条件差（emu 弹态/位置/AreObjectsTouching 判）+F1FA 内层（Temp 语义）=中件。
本卡交付=F1FA 全谱+2466 径定谳（非猜：asm :12214-12232 逐行）。

Evidence: F1FA:10200-10261 全谱（双轴+hi+scroll dir）+:12214-12232（Skree 弹伤五件
链——2466 对应）+Temp 语义（10262-10269）。
Confidence: 92
