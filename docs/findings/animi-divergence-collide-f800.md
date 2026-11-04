# animI 走表轨迹域·collide f800 animI 5 步滞后——偏离点定位（卡 A）

> 战役：animI/走表轨迹域。卡 A=asm 逐行读证 SetSamusAnim 族 + animResetIndex 写源
> vs rust 初值路径，定位 collide f800 animI 5 步滞后（emu 0D/rust 08）的偏离点。
> 结论分级见 §5。纪律=AGENTS.md（证据 A/B/C 分级 + 禁试错式修改 + 证伪更正）。

## 1. 症状（A 级实录——两腿 capture 点 RAM 直读）

collide f800（`case-collide-probe_f00800_l240.ram.hex`，两腿全 2KB RAM hexdump 直读）：

| 字段 | 地址 | emu（oracle 腿） | rust（shots_rust 腿） | 判 |
|---|---|---|---|---|
| animFrame | $0303 | **0x10** | 0x08 | 分歧 |
| animDelay | $0304 | **0x02** | 0x01 | 分歧（相位差 1） |
| animResetIndex | $0305 | **0x0C** | 0x0C | **一致** |
| animIndex | $0306 | **0x0D** | 0x08 | 分歧（rust 落后一走表步） |

**关键**：animResetIndex 两腿一致（0x0C=SamusJump）⇒**写源值正确**，非 reset 写值错。
分歧在 **animIndex/animFrame/animDelay**（走表推进态）。

## 2. 走表时序（全 capture 点两腿对表——A 级）

| frame | emu(frm,dly,rst,idx) | rust(frm,dly,rst,idx) | 判 |
|---|---|---|---|
| f420 | (00,00,00,00) | (00,00,00,00) | 同 |
| f540 | (07,00,04,05) | (07,00,04,05) | 同 |
| f620 | (07,00,04,05) | (07,00,04,05) | 同 |
| f700 | (07,00,04,05) | (07,00,04,05) | 同 |
| **f800** | (10,02,**0c**,0d) | (08,01,**0c**,08) | **首分歧** |
| f890 | (08,00,07,07) | (08,00,07,08) | 分歧（idx 差 1） |
| f900 | (08,02,00,08) | (05,01,00,03) | 分歧 |

anim 常量（handler.rs:125-134）：RUN=0x00/FRONT=0x04/STAND=0x07/JUMP=0x0C/SALTO=0x0E。
collide 动画径（f700→f800）：**FRONT(0x04)→STAND(0x07)→JUMP(0x0C)**。

## 3. 偏离点（A 级定位）

f800 时：**emu 已收敛到 JUMP 走表**（animIndex=0x0D=JUMP 稳态（reset=0x0C 单帧表
table[0x0C]=0x10→idx←0x0D→table[0x0D]=FF→wrap 回 0x0C→稳态 0x0D）；frame=0x10=JUMP 帧）。
**rust 仍显示 STAND 帧**（frame=0x08=table[0x07]=STAND 帧；animIndex=0x08=STAND 走表位
（table[0x08]=FF，下一步 wrap））。即 **rust 落后 emu 一走表推进步**（STAND→JUMP 的
FF-wrap 尚未发生）。

**偏离点 = animResetIndex-only 写后的 animIndex 收敛时机差**：
- asm 走表族（SamusRun samL01/02/03/04 + falling 臂）切动画时**只写 animResetIndex**
  （`sta Samus.animResetIndex`，不写 animIndex）——animIndex 靠 FF-wrap 收敛到新 reset。
- rust 同形（handler.rs:166/247 等**只写 reset**）——**忠实**。
- 但 **emu 在 f800 已 wrap 收敛（JUMP 0x0D），rust 尚未 wrap（STAND 0x08）**——收敛时机
  差 **1 走表步（≈2 帧 @ A=2 空中节律）**，伴随 animDelay 相位差 1（emu=0x02/rust=0x01）。

## 4. SetSamusAnim 族 asm 语义（A 级逐行读证）

| 件 | bank:addr | 写面 | 备注 |
|---|---|---|---|
| SetSamusAnim | 07:CF6B | reset←A；（fallthrough）index←A；delay←0 | 双写 reset+index |
| SetSamusNextAnim | 07:CF6E | index←A；delay←0 | **只写 index**（不写 reset） |
| InitObjectXSlotAnimIndex | 07:D2FA | reset,x←A；（D2FD）index,x←A；delay,x←0 | 对象槽索引版 |
| SetSamusJump | 07:CFC3 | reset←Jump；**index←Jump-1**（dey=JumpTransition）；delay←4 | jump 起手带过渡帧 |
| SetSamusData_3Frame | 07:CD6B | A=#$03→UpdateObjAnim | 地面走表节律（3 帧） |
| SetSamusData（空中 samL06） | 07:CD16 | A=#$02→UpdateObjAnim | 空中走表节律（2 帧） |
| UpdateObjAnim | 07:DC8F | animDelay 倒数→0 时重载 A + animIndex 推进；FF→wrap 回 reset | 走表核 |

rust 对应件（忠实面 ✓）：
- `set_samus_animation`（animation.rs:47）=[$0305←A/$0306←A/$0304←0] ✓（SetSamusAnim 全形）
- `set_samus_jump`（handler.rs:447）=[reset←0x0C/index←0x0B/delay←4] ✓（SetSamusJump dey 形）
- `update_obj_anim`（physics.rs:892）走表核 ✓（FF-wrap 回 reset，PageIndex 基址）
- `set_samus_data`（physics.rs:817）→`update_obj_anim(mem, mem.regs.a)`（A=节律重载值）
- 节律 A 值：空中共 `mem.regs.a=2`（handler.rs:265，asm samL06 `lda #$02` ✓）/地面共
  `mem.regs.a=3`（handler.rs:329 等，asm CD6B `lda #$03` ✓）。

## 5. 结论分级

- **已证实（A 级）**：
  1. animResetIndex 写源值正确（f800 两腿均 0x0C=JUMP）——非 reset 写值错。
  2. SetSamusAnim 族 asm 语义（§4 表）+rust 对应件忠实（set_samus_animation/
     set_samus_jump/update_obj_anim/set_samus_data 节律 A 值全对表）。
  3. 偏离点=animResetIndex-only 写后 **animIndex 收敛时机差 1 走表步**（emu f800 已
     wrap 到 JUMP 稳态 0x0D，rust 尚在 STAND 位 0x08）+animDelay 相位差 1（0x02/0x01）。
  4. collide 动画径=FRONT(0x04)→STAND(0x07)→JUMP(0x0C)（f700→f800）。
- **假设（待证）**：
  - 收敛时机差 1 步的**根**=①animDelay 走表节律相位累积差（r14「AD 周期别名」残余）
    或②切动画触发时机差（physics：SamusAccelY/falling 判定帧相）——**需逐帧 trace 定帧**
    才能钉单一修点（禁试错式修改——AGENTS.md §3.5）。二者均表现为 ~1-2 帧偏移。

## 6. 下一步（卡 B 候选）

1. 逐帧 trace（DSH_SPRITE_TRACE 桩已备：`[uoa]/[anim]/[ssd]` 打 animIndex/reset/delay/A
   逐帧）对 collide f700-f800 两腿逐帧对表——定 STAND→JUMP 切换的**精确分歧帧**。
2. 依分歧帧定根（①节律相位 or ②切换触发时机）→TDD 红（钉收敛时机）→修→绿。
3. 判据：collide f800 animIndex 两腿 0 差（+全族 l4 无回退）。


## 7. 卡 B·逐帧 trace 定根（2026-11-04）——近因①证实 + 深根=anim-set 路径/移动态

【双腿逐帧实录（A 级）】emu watch_range 0305-0306 逐帧 + rust DSH_SPRITE_TRACE（[uoa]/[anim]/[ssd]）。
- **EMU**：f778-796 FRONT(reset=04,idx=05 稳)→**f797 (reset=00,idx=08)**→**f798 JUMP(0C,0D 稳态)**。
  f797→f798：idx 0x08→0x0D=FF-wrap **即时**收 wrap（animDelay=0⇒推进即刻，同帧切同帧 wrap）。
- **RUST**：n=785 FRONT(04,04)→**n=787 STAND(07,07) a_in=4**→**n=788 JUMP(0C,idx=08) ad=4 a_in=2**
  →n=789 ad=3→n=790 ad=2→…→n≈792 ad=0 才 wrap 收敛。idx=0x08（STAND 位滞）阻 ~4 帧。

【近因裁定=假设① animDelay 走表节律相位（证实）】切换点（reset←JUMP）animDelay 值：
**emu=0**（推进帧，idx 即 wrap 到 JUMP 0x0D）vs **rust=4**（STAND a_in=4 重载继承（asm LCC6E
`lda #$04` 忠实）阻 wrap）。收敛 wrap emu@f798 vs rust@n≈792（迟 ~4 帧）。

【深根=anim-set 路径/移动态分歧（A 级）】切换点**前一 anim**不同：
- emu：**RUN(0x00)**@f797→samL04（`cmp animReset==Run`⇒`sta animReset←Jump`）→JUMP。
- rust：**STAND(0x07)**@n=787→falling 径（asm 2868 `sta animReset←Jump`（falling+disp<0x18））→JUMP。
即 emu 走 samL04（RUN→JUMP），rust 走 falling（STAND→JUMP）——**前 anim 移动态不同**（emu 判
Samus 在跑 RUN，rust 判停 STAND）。碰撞后移动态（跑 vs 停）分歧⇒anim-set 路径分叉⇒animDelay
相位（RUN/STAND 节律不同）⇒收敛时机差。触发径本身（falling+disp<0x18（handler.rs:224≈asm
2864-2870））忠实；samL04（handler.rs:246）忠实。**分叉源=前 anim 移动态（RUN vs STAND）**。

【结论分级】
- 已证实（A 级）：近因=animDelay 相位（emu 0/rust 4 阻 wrap）；深根=anim-set 路径分叉（emu
  RUN→samL04 vs rust STAND→falling）=**移动态分歧**；触发径/samL04/LCC6E 节律全忠实。
- 待证：移动态分歧（RUN vs STAND）根=碰撞/physics（Samus 移速/位移判）——属 physics 域，
  非 anim 域局部可修（AGENTS.md §4 定位层：跨层→physics）。

【修径】
- anim 域局部（近因）：切换点 reset←JUMP 时 idx 已在 FF（旧 anim 末）⇒即 wrap（不阻于
  animDelay）——但 asm 走表核 animDelay=0 才推进（忠实）⇒此修**非 asm 忠实**（需 emu 反证
  emu 亦仅巧合 ad=0）。
- physics 域（深根）：移动态（碰撞后跑/停）对齐 emu——Samus 速/位移/碰撞反应径 asm 对读。
判据：collide f800 animIndex 两腿 0 差（idx=0x0D）+l4 无回退+443 测全绿。


## 8. 卡 C·physics 移动态定根（2026-11-04）——深根=房间过渡/门入 intro 分歧（结构性）

> 【2026-11-04 更正】本节「房间过渡/门入 intro 分歧」与「emu 判 RUN 走 samL04 vs rust
> 判 STAND 走 falling 移动态分叉」定性**撤销**——卡 E 读数重对齐（§11.0-11.2）：f797
> 两侧收口同态（RUN reset-only），「分叉」为探针 n/f 错位+walk-entry 中间态伪影。真根
> =D1（CF88 pressed 尾 SetSamusAnim 全形缺失——§11.4）。现象层观察（anim-set 路径差异
> 表象）维持存档。

【逐帧实录（A 级）】watch 0300-030F（status/radius/anim/x）+ 0305-0306（anim）两腿对表。
- **EMU**：f785-795 **status=$300=0x18**（intro/fade-in 态）+anim=FRONT(04)+x=80（停）→
  f796 status=**FF/sa_Begin**（`SamusIntro C9D7`：`lda #sa_Begin/sta Samus.status`「Fade in
  complete…Samus facing forward」）→ f797 status=**01/sa_Run**+anim=RUN(00)→f798 JUMP(0C)。
  即 emu 走**房间过渡 intro/fade-in**（status 0x18→sa_Begin(FF)→sa_Run(01)）。
- **RUST**（DSH_SPRITE_TRACE [ssd]）：n=787 **status=01(sa_Run)**+anim=STAND(07)+x=80+AccelY=00
  →n=788 anim=JUMP(0C)+AccelY=1A→n=789+ x=7F（跑）→n=793 AccelY=00（落地）。
  即 rust **不走 intro/过渡**——status 恒 01(sa_Run)。

【偏离点】status($300)：**emu=0x18（intro/fade 态）vs rust=01(sa_Run)**——emu 做房间过渡
（intro fade-in→sa_Begin→sa_Run），rust 不做（恒 Run）⇒anim-set 路径分叉（emu FRONT→RUN→
JUMP vs rust FRONT→STAND→JUMP）⇒animDelay 相位⇒收敛时机差（卡 B 近因）。深根=**房间过渡/
门入 intro 分歧**（emu 进房 intro，rust 不进）。

【asm 考证（A 级）】
- SamusIntro（07:C9D7）：`ldy Samus.status`（fade-in 态）/`lda Timer3/bne`（intro 未完）/
  fade 完成⇒`lda #sa_Begin/sta Samus.status`。status=0x18=intro/fade-in 进行态。
- CheckStopHorzMvmt（07:E365）：撞停⇒（AccelY==0 地面）`jmp SetSamusStand`（→STAND）；
  空中（AccelY≠0）出（不动 anim）——rust n=787 AccelY=00⇒SetSamusStand（STAND）✓逻辑忠实。
- MoveSamusLeft（07:E626）+ObjectCheckMoveLeft：8px 门检（(x-radiusX)&7==0 才 crash 检）——忠实。
- CheckHealthStatus（07:CDFA）：isHit bit5⇒knockback（speedY=FD/AccelY=38）**不改 status**。

【结论=结构性深根（lead 卡 C 步 4）】status=0x18 intro/fade（房间过渡）域——emu 进房 intro
（status 0x18→sa_Begin→sa_Run），rust 不进（恒 Run）⇒全下游分歧（anim/收敛/l4）。
**非 anim/局部 physics 可修**——属**房间过渡/门入**域（结构层）。status=0x18 写源非直接
`sta Samus.status`（计算/表径）——需房间过渡/门入 asm 全径考证（SamusEnterDoor $8B13/SetupRoom
$EA2B/门滚动族）定根。**如实停机报证据不强修**（AGENTS.md §3.5 禁试错式修改）。

【修径建议（转房间过渡/门入战役）】
1. asm 全径考证：门入→房间加载→intro fade-in（SamusEnterDoor $8B13→SetupRoom $EA2B→
   SamusIntro C9D7）+ status=0x18 写源（何处置 0x18）。
2. rust 对应径：collide 门入判定（撞门→status=0x18/过渡）vs 现状（撞墙→STAND/恒 Run）。
3. 判据：collide f800 animIndex 两腿 0 差（深根修后近因自动消）+l4 无回退+443 绿+基线保持。


## 9. 卡 D·status=0x18 写源+触发径考证（2026-11-04）——研究定谳（不修）

> 【2026-11-04 更正】§9.4/§9.5 的「缺口=淡入节律时序分叉」定性**撤销**——卡 E 双侧
> 逐帧对表（§11.1）实证淡入写源/逻辑/时序（含 TimerDelay 相位）**全窗 0 差**；原
> 「rust 提前收」读数=探针 n/f 索引错位伪影（§11.0）。0x18 写源考证（§9.1-9.2）维持。

### 9.1 0x18 写源定位（A 级 asm 行号锚）
status=0x18 = **Samus 淡入序第 4 调色步**（非直写常量——SamusInit 起点 + SamusIntro 步进合成）：
- **SamusInit（07:C8D1，prg7:1968-1983）**：`ldy #_id_Palette13+1.b / sty Samus.status`
  ⇒ status←**0x14**（_id_Palette13=**0x13**（out/M1_NES_CNS.sym:14220 `00000013 _id_Palette13`）
  ⇒ +1=0x14）。注「Set Samus status as fading onto screen」。同程 `sta MainRoutine←_id_SamusIntro`
  +`sta Timer3←$2C`（440 帧淡入）。
- **SamusIntro（07:C9D7，prg7:2186-2237）**：`cmp SamusFadeInTimeTbl-(_id_Palette13+1),y`
  （**status=表索引**）+命中⇒`inc Samus.status`（步进）+`sty PaletteDataPending`。
  SamusFadeInTimeTbl（07:CA13）=`$1E,$14,$0B,$04,$FF`（5 值）。status 轨迹：
  **0x14**（起点）→0x15（Timer3=0x1E）→0x16（0x14）→0x17（0x0B）→**0x18**（0x04）→（表[4]=FF
  不再 inc）→**FF/sa_Begin**（Timer3==0 淡入完成（prg7:2194-2199））→**01/sa_Run**。
  ⇒ **status=0x18=淡入第 4 调色步**（末步，等 Timer3==0 收 sa_Begin）。

### 9.2 status 谱系全表（0x18/FF/01 写源）
| 值 | 语义 | 写源（bank:addr） |
|---|---|---|
| 0x14 | 淡入起点（_id_Palette13+1） | SamusInit 07:C8D1（`sty Samus.status`，prg7:1983） |
| 0x15-0x18 | 淡入调色步进（inc status） | SamusIntro 07:C9D7（`inc Samus.status`，prg7:2215，Timer3 表触发） |
| FF/sa_Begin(255) | 淡入完成（Samus facing forward） | SamusIntro 07:C9D7（`lda #sa_Begin/sta`，prg7:2198-2199） |
| 01/sa_Run | 跑动（ActionTable） | SamusStand 07:CC36（prg7:2765-2769）/ActionTable($CC87) |

### 9.3 触发径（双侧 trace 对表）
- **触发事件=游戏开场 SamusInit**（非门入/非撞墙）：场景 cases/case-collide-probe.txt
  press 310/350 start（菜单→游戏起）⇒SamusInit（status←0x14）⇒SamusIntro 淡入（440 帧 Timer3）。
- **emu 逐帧**（watch 0300-030F）：f785-795 **status=0x18**（淡入第 4 步，等 Timer3==0）→
  f796 **FF/sa_Begin**（淡入完成）→f797 **01/sa_Run**。即 emu 淡入在 f785-795 仍在走（0x18）。
- **rust 逐帧**（[ssd]）：n=787 **status=01/sa_Run**（淡入已完）。即 rust 淡入提前收（sa_Run）。

### 9.4 缺口定性（A 级）
- **rust SamusInit 写源正确**（mainloop.rs:167 `SAMUS_STATUS←0x14`=_id_Palette13+1，sym 铁证
  0x13+1=0x14）✓；**rust SamusIntro 淡入逻辑忠实**（mainloop.rs:203-245：Timer3==0⇒sa_Begin/
  表[0x1E,0x14,0x0B,0x04,0xFF] 步进/奇帧 Front 闪现——asm C9D7 逐字）✓。
- **缺口=淡入时序差**（非写源）：emu f785-795 仍在淡入（status=0x18，11 帧）vs rust n=787 已
  sa_Run（淡入提前收）。status=0x18 持续 11 帧 ⇒ emu 步进/Timer3 递减速率 vs rust 差——**淡入
  节律（Timer3 递减/步进触发帧）分叉**。下游（anim-set RUN vs STAND/收敛）随之。
- **非门入径**（lead 疑）：此场景 status=0x18=**游戏开场淡入**（SamusInit），非门入/房间过渡
  （SamusEnterDoor 已端口五点 match 不涉）。卡 C「房间过渡」判**更正**：实为**开场淡入序**。

### 9.5 结论（研究定谳，不修）
status=0x18 写源=SamusInit(0x14 起)+SamusIntro(inc 至 0x18，淡入调色第 4 步)。触发=游戏开场。
缺口=**淡入节律时序**（Timer3 递减/步进触发帧 emu vs rust 分叉）——非写源/非门入。修径=淡入节律
（Timer3 递减率+步进触发帧）asm 对读定根（下一卡）。【2026-11-04 更正】卡 C §8「房间过渡/门入」
定性改为「开场淡入序」（写源=SamusInit 非 SamusEnterDoor）。

## 10. 卡 E·淡入节律 asm 全径逐行读证（2026-11-04）——Timer3 递减率/步进触发帧/440 帧结构

### 10.1 递减率（谁减/几帧一减——UpdateTimer 族）
- **UpdateTimer 07:C266**（prg7_engine.asm:654-678，头注「Derived from FDS BIOS's
  CounterLogic routine」——即任务书「UpdateAge 族」所指计数器族）逐行：
  `ldx #$01 / dec TimerDelay($29) / bpl @endIf_A`——$29 每帧 dec；**结果≥0（原值≥1）⇒
  X=$1**（本帧只减 Timer2/Timer1）；**原值==0 ⇒ dec 出 $FF（N=1）**⇒`lda #$09/sta
  TimerDelay`+`ldx #$02`（含 Timer3）。`@loop_decTimer`（:672-677）=`lda Timer1,x/beq
  跳/dec Timer1,x`（**非零才减**——Timer3 到 0 后冻结）+`dex/bpl`（X=2→1→0=Timer3→2→1）。
  ⇒ **TimerDelay 9→0 十态循环；Timer3 每 10 帧减 1（于 $29 的 0→$FF 跨越帧）**。
- **调用序 MainLoop 07:C0BC**（prg7_engine.asm:199-210）：CheckBankSwitch→**UpdateTimer**→
  GoMainRoutine→inc FrameCount→等 NMI→RandomNumbers→循环。**UpdateTimer 先于状态机
  分发**：SamusInit 同帧置 Timer3=44 不会被同帧减（首减最早在次帧）。
- **rust update_timer（mainloop.rs:2959-2980）逐语义对表=忠实**：wrapping_sub-1+bpl 等价、
  reset 0x09、非零门、i=last→0 降序（Timer3→2→1）、`regs.x=0xFF` 残迹注（asm dex 出口）✓。
  调用序（mainloop.rs 迭代注）=UpdateTimer→GoMainRoutine→inc FrameCount ✓ 对表。

### 10.2 440 帧计数结构（谁设/如何折算 440）
- **SamusInit 07:C8D1**（prg7_engine.asm:1971-1983）：`lda #_id_SamusIntro/sta MainRoutine`；
  **`lda #$2C/sta Timer3`**（NES_NTSC——注「440 frames to fade in Samus(7.3 seconds)」；
  PAL=$26）；`jsr IntroMusic($CBFD)`；`ldy #_id_Palette13+1/sty Samus.status`（←**0x14**）；
  SamusInvincibleDelay←0；PipeBugHoles/EndTimer/Rinka 簇初始化。**本径无 TimerDelay($29) 写**。
- **结构=Timer3=44 单位 × $29 十帧预分频=440 名义帧**。收尾帧精确式：
  **fade-done 帧 = S + 431 + d**——S=SamusInit 分发帧（该帧 UpdateTimer 已过）；d=该帧
  UpdateTimer 后 $29 残值（0..9）⇒ 首个 Timer3 减落点=S+d+1、末落点（1→0）=S+d+431、
  同帧 GoMainRoutine 内 SamusIntro 见 Timer3==0 收尾。**收尾帧抖动=±9 帧=$29 相位 d**。
- **TimerDelay($29) 生命周期（SRC 全文 grep）：仅 UpdateTimer :658(dec)/:663(reset 9)
  两写点，无任何初始化写**。boot 值源=**Startup 07:C01A**（prg7_engine.asm:59-105）清零环：
  `$01←7/$00←0/A=0` 逐页 `sta ($00),y`——`dec $01/bmi 出/ldx $01/cpx #$01/bne 环`：
  **$01==1 落出 ⇒ 只清 $0200-$07FF**（$01 栈页与 **$00 零页不清**——环自指保护）。
  ⇒ **$29 boot 值=主机 RAM 残值（游戏从未写初值）**；从 MainLoop 首迭代起自由跑，
  跨越相位=(boot 值 V+1) mod 10。**【首异主嫌①：emu（Mesen RAM init 策略）与 rust
  （FrameMemory 初值）的 $29 boot 值若不同 ⇒ 预分频相位偏 ⇒ 收尾帧偏整周期】**。

### 10.3 步进触发帧（表触发结构）
- **SamusIntro 07:C9D7**（prg7_engine.asm:2186-2237）逐行：
  `jsr EraseAllSprites($C1A3)`→`ldy Samus.status`→`lda Timer3/bne LC9F2`：
  **Timer3==0 ⇒ 收尾**（sta ItemRoomMusicStatus←0/status←sa_Begin/StartMusic($D92C)/
  SelectSamusPalette($CB73)/MainRoutine←_id_GameEngine，:2193-2203）。
  LC9F2（:2205-2207）：`cmp #$1F/bcs RTS`——A=Timer3（bne 不改 A）：**Timer3≥31（剩
  310 帧内）前不显示 Samus**。
  :2209-2216：`cmp SamusFadeInTimeTbl-(_id_Palette13+1),y`（**Y=status=表索引**：0x14→表[0]）
  命中⇒`inc Samus.status`+`sty PaletteDataPending`。**表 CA13**（:2242-2243）=
  **$1E,$14,$0B,$04,$FF** ⇒ 步进触发帧=Timer3 首达 30/20/11/4 的减落点帧（第 14/24/33/40
  减落点）；status 0x14→0x15→0x16→0x17→0x18（第 4 步后等 Timer3==0）。
  :2217-2235：奇 FrameCount 闪现门（`lda FrameCount/lsr/bcc`）→SetSamusAnim($CF6B,
  SamusFront)→SpritePagePos←0/PageIndex←0→`jmp AnimDrawObject($DE47)`。
- **触发=逐帧比较制（cmp 表值）非边沿制** ⇒ 步进帧严格跟随 Timer3 减落点（相位由 $29 决定）；
  4 步+收尾全部锚在预分频相位上。rust mainloop.rs:203-245 samus_intro 逻辑=asm 逐字
  （卡 D §9.4 已证）——**写源/逻辑忠实，纯时序差**。

### 10.4 定性（收窄表述——verifier 复核处方 a 并入）
四层链总根表述**收窄**为：**fade-COMPLETION 路径分歧（transient f785-797）**——
rust/emu 淡入写源与逻辑全忠实；分歧在**淡入收尾时序**（Timer3 减落点相位）。
两候选根（§11 钉首异帧/首异写点后定谳）：
① **$29 boot 相位差**（§10.2——游戏无初值写，双腿 RAM 初值策略不同即偏整周期）【主嫌】；
② **S 帧差**（标题→GameMode 切换→SamusInit 时间线，双腿首异帧可能更早）。
证据面纪律：本节 asm 引证全为 A 级（bank:addr+file:line 逐行读证）；结论 ① 为 A 级
结构事实（无初值写=可复现 grep+读证）+ **B 级推断**（初值不同⇒相位偏——待 §11 双侧
$29 轨迹实测定谳）。

## 11. 卡 E·双侧逐帧对表+总根定谳（2026-11-04）——fade 0 差实证 + 首异帧 f798 + D1/D2 修

### 11.0 探针索引勘误（前置更正——影响卡 B/C/D 读数）
【2026-11-04 更正】rust DSH_SPRITE_TRACE 的 **n=DIAG_ITER ≠ 场景帧 f**：**f=n+10**
（FC 读于处理中、state 行采于收口 inc 后：探针 FC=k ⇒ 行 FC=k+1——[fade] n=786 FC=0C↔
行 f796 FC=0D、[ssd] n=787 FC=0D/st=01↔行 f797 FC=0E/st=01 双锚合）。卡 B/C 的
「rust n=787 status=01 提前收」实为 f797（=emu 同帧同值）——**淡入收尾时序双腿无差**。
卡 B 的「emu f797 ad=0」同源错位。本节读数全部按 f=n+10 重对齐。

### 11.1 淡入轨迹双腿对表（A 级——emu frames.jsonl watch vs rust --state-out）
- 对表口径：emu（cases/case-collide-probe.txt watch 001E/0029/002C/002D/0300-030F——
  run.json scenario_sha256=38fbb637 与文件直算一致）逐帧 vs rust state 行
  （spin 出口采样缝——同相位锚）五键（MainRoutine/TimerDelay/t3/FrameCount/status）。
- **f350-800 全窗 0 差**（除采样缝固有缺行 f352-358（标题迁移 7 帧）——缺行集恒定已钉）。
- 淡入结构锚（双腿同）：f350-359 t3=0x12（标题 SetTimer 残值）→**f360=S**（MR←8/SamusIntro、
  t3←$2C=44、status←0x14、**TimerDelay@S=5**）→减落点 f366+10k（44→43→…）→步进
  st 0x14→0x15（t3=1E）→0x16（14）→0x17（0B）→0x18（04）→**f796 t3=0 收尾**（MR←3/
  status←FF）→f797 status=01。**收尾帧=S+d+431=360+5+431=796**（§10.2 公式实证）。
- ⇒ **卡 D「淡入节律时序分叉（总根）」定性撤销**（§9.5 更正）：淡入写源/逻辑/时序
  **三面全同**（TimerDelay 自由跑相位亦同——boot 清零环不涉零页（§10.2）但双腿初值同 0）。

### 11.2 首异帧/首异写点（A 级——state 行+probe_mem 写点 caller 归属）
- **首异帧=f798（anim 切换帧）**，首异字段=animFrame/animDelay/animIndex（$0303/4/6）：
  emu(af=10,ad=02,ar=0C,ai=0D)=JUMP 稳态 vs rust(af=08,ad=03,ar=0C,ai=08)——f797 两侧
  完全同态(08,04,00,08)；f798 起分歧；f803 着陆切 RUN 后再对齐。
- **写点实录**（cardi-f798probe.probe.jsonl——probe_mem 0303-0306，caller=jsr 站）：
  | 场景帧 | 写序（addr←val@caller） | 判 |
  |---|---|---|
  | f794-795（fade） | (05←4,06←4,04←0)@CA07 + (04←0,03←7,06←5)@DE47 | SetSamusAnim 全形(Front)+DE47 走表（奇帧闪现门） |
  | f796（fade 收尾帧） | (04←0)@CF5D + (05←7,06←7,04←0)@CC3C + (04←4,03←8,06←8)@CD6D + (05←0)@CC14 | 地面径 SetSamusAnim(STAND) 全形+CD6D 走表(A=4)+RUN reset-only |
  | **f797-799（下落）** | (05←0C)@CC14 + **(05←0C,06←0C,04←0)@CD16** + (04←2,03←10,06←0D)@CD6D | **CF88 pressed 尾 SetSamusAnim(Jump) 全形（caller CD16=jsr SamusRun_CheckHorzMovementMidair）**+CD6D 走表(A=2)同帧 wrap |
  | f802+（落地跑动） | (05←0,06←0,04←0)@CDCF + (04←3,…)@CD6D | RUN 全形+CD6D 走表(A=3) |
- **emu 每下落帧=SetSamusAnim 全形(0x0C)+1 走表**（非多走表——写点计数与 caller 归属唯一）；
  全形 delay←0 ⇒ 同帧走表即 wrap（ad 末=02=reload A）⇒ JUMP 稳态即达。

### 11.3 走表预算（A 级——rust [uoa] 逐调用 vs emu 写点数）
- rust f798-801（n=788-791）：**1 走表/迭代**（a_in=2）——asm 忠实（samL06 `lda #$02`）；
  n≥802（f812+）：**3 走表/迭代**——1 正当（CD6D a_in=3）+**2 幻影（a_in=0）**：
  ENTRY 态（ai/ar/ad via $4B=0）=Samus 槽 RUN 态实锤（错槽）——=pickup/spawn.rs:172
  DE47 形（asm :5555-5561 **`jsr ObjDrawFrame`（DE4A 纯绘）**）双 item 槽各一。
- 幻影走表后果：RUN 动画 3× 过速（ad 1→0+advance 每迭代（3,2,1→…循环））+f814-900
  anim 轨迹全偏（82/87 帧 diff）。

### 11.4 总根定谳（D1+D2——非卡 D 淡入节律）
- **D1（f798 分叉根=卡 A/B/C 链真正近因）**：rust `samus_run_check_horz_movement_midair`
  （handler.rs）pressed 尾被「goal-轮77 修」换成了 samL04 reset-only 形——asm 真身
  （prg7_engine.asm:3404-3409）=`stx SamusDir / lda Table06+1,x / jmp SetSamusAnim`
  且 **Table06（:3566-3568）=0x0C×3**（ObjAnim_SamusJump 三连）⇒**SetSamusAnim 全形
  （CF6B：reset←A/index←A/delay←0）每下落帧**。delay←0 缺失⇒ad 相位阻 wrap（卡 B 近因）
  ⇒animIndex 收敛差（卡 A）⇒f800 animI 08/0D。samL04 真身=samus_run 空中径
  （:2885-2903——rust :244-247 另折 ✓ 保留 reset-only）。
- **D2（f812+ 过速根=r14 残留）**：DE4A 站幻影走表×2/帧（§11.3）。修径=两站换纯绘形
  （obj_draw_frame）+`lda #$40/sta PageIndex`（:5502-5504/:12146-12148）端口。
  随件勘误（同 asm 块）：①「未绘出」判据=SpritePagePos(**$5B**) 非 $4B（:5555-5566
  pha/pla/cmp）——旧形恒等⇒**拾取授予径永不可达**（原版行为缺失）②skree 弹暂存
  PowerUpDraw.hi=Object.hi(**$34C**) 非 +7（data07）③skree 绘槽=staged $40 槽
  （旧形错绘 $3A0+yy=Door 槽）。
- 修后验证：**f798-802 全窗 JUMP 稳态（10/02/0C/0D）+f800 animIndex=0x0D=emu 锚
  0 差**（TDD 锚 card_e_animi_anchor 实跑）；淡入轨迹锚=逐帧双腿 0 差恒定。

### 11.5 TDD 记录（红→绿）
| 测 | 靶 | 红因 | 绿证 |
|---|---|---|---|
| handler::midair_steering_full_sets_jump_anim | D1 全形三写 | 旧形只写 reset（index/delay 断言红） | lib 全绿 |
| handler::midair_steering_guards_do_not_touch_anim | D1 守门（上升禁写） | （防过应用——先行绿） | lib 全绿 |
| spawn::powerup_display_is_pure_draw_no_walk | D2-1 纯绘+PageIndex 端口 | 幻影走表 dec animDelay（红） | lib 全绿 |
| spawn::powerup_touch_grants_when_drawn | D2-1b $5B 判据+授予径可达 | 恒 continue⇒Some 断言红 | lib 全绿 |
| mainloop::skree_projectile_display_is_pure_draw_powerup_slot | D2-2 纯绘+暂存位移 | 幻影走表+hi 位移错（红） | lib 全绿 |
| tests/card_e_animi_anchor（集成） | 淡入轨迹锚+f800 animI 锚 | f798 animFrame=8≠16（红） | ok（0 差） |

### 11.6 可复读面
- emu 采点=cases/case-collide-probe.txt（已 commit 0b8ae14c——watch 5 点）；emu 帧级=
  analysis/traces/case-collide-probe.frames.jsonl（run.json sha=38fbb637 自锚）。
- 写点探针场景（§11.2 证据源）——全文（独立 name 防产物踩踏；内容即定义，可复跑）：
  ```text
  name cardi-f798probe
  frames 900
  watch_range 0300-030F samus
  watch 001E MainRoutine
  watch 0029 TimerDelay
  watch 002C t3
  watch 002D FrameCount
  snapshot none
  press 310 start
  press 350 start
  capture 420 game-in
  hold 440-540 right
  capture 540 r100
  press 560 a
  hold 560-700 left
  capture 620 l60
  capture 700 l140
  hold 700-900 left
  capture 800 l240
  capture 890 l330
  endFrame 900
  probe_mem 0303 0306
  watch 0314
  ```
  运行：`python3 tools/scenario_run.py run <该文件>` ⇒analysis/traces/cardi-f798probe.
  {frames.jsonl,probe.jsonl}。
- 对表脚本（f 键提取+五键对比+首异帧/首异写点）：§11.1-11.2 表由 frames.jsonl+state
  行（rust_screenshot --state-out——zp/iram 全量编码）直接提取；state 行=spin 出口
  采样缝（与 emu endFrame 同相位——FC 逐帧 0 差自证）。

### 11.7 更正链（证伪更正——AGENTS §3.6）
- 【2026-11-04 更正】§8 卡 C「房间过渡/门入 intro 分歧（结构性深根）」及「emu 判 RUN 走
  samL04 vs rust 判 STAND 走 falling 移动态分叉」：**撤销**——读数为 n/f 错位伪影
  （§11.0）+walk-entry 中间态误读（f797 两侧收口同为 RUN reset-only）。真根=D1（CF88
  尾形缺失）。
- 【2026-11-04 更正】§9.5 卡 D「缺口=淡入节律时序（Timer3 递减/步进触发帧分叉）」：
  **撤销**——§11.1 全窗 0 差实证（含 TimerDelay 相位）。淡入径（写源/逻辑/时序）无缺口。
- 卡 A/B 的现象层结论（animIndex 收敛差/animDelay 相位阻 wrap）**维持**——其根由 D1 承担
  （§11.4）。

Evidence: emu=analysis/traces/case-collide-probe.frames.jsonl（watch 5 点逐帧——
scenario sha 38fbb637 自锚）+cardi-f798probe.probe.jsonl（probe_mem 写点 caller 归属）；
rust=/tmp/cardi_rust_state.jsonl（--state-out zp/iram 逐帧）+DSH_SPRITE_TRACE [uoa]/
[ssd]/[fade] 逐调用；asm=prg7_engine.asm:2885-2903(samL04)/3390-3409(CF88)/3566-3568
(Table06)/3434-3440(CFC3)/3371-3378(CF6B)/5502-5566(UpdateAllPowerUps)/12146-12212
(UpdateSkreeProjectile)；TDD=tests/card_e_animi_anchor.rs+lib 5 测（红→绿实录）。
Confidence: 93
