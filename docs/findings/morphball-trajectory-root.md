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
