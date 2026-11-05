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
