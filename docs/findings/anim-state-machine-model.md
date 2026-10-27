# anim 状态机完整模型 + $0305 写点清单 + emu f550 animReset=4 根因 + rust 差异点

> 【状态】decomp-analyst 考证（A 级为主（ROM 字节（refs/m1disasm（与 ROM 逐字节等价）+动态 trace（Mesen2 frames.jsonl+rust state.jsonl））））
> 【refs 引用必写】bank:addr 引证（A 级原文）+【留档可追溯】docs commit（追溯径）
> 【生成】refs asm 先读纪律（refs/ bank:addr 查读→trace/字节实证→语义自证）

## 0. 结论摘要（verdict）

**emu f550 animReset=4(Front) 根因**（已证实（A 级））：**SamusIntro 游戏开始淡入闪烁**（bank7:CA00（LCA00））在
Timer3<31 区间（300 帧（Timer3=每 10 帧递减（constants_ram.asm:496）））每奇帧调
`SetSamusAnim(Front(4))`（bank7:CF6B（A→$0305+$0306+delay=0））→`AnimDrawObject`（bank7:DE47
（**首句 `jsr UpdateObjAnim`**（走表）））→(animReset=4, animIndex=5, animFrame=7) 稳态（Front 帧）。
f550 深处淡入闪烁期（f497-f800）——**移动输入被忽略**（SamusIntro≠GameEngine（GoMainRoutine 分发））。

**rust 差异点**（已证实（A 级（state.jsonl 实值）））：rust f550=(animReset=**4**（与 emu 同）, animIndex=**4**（emu=5）,
animFrame=**0**（emu=7）)——**差异非 animResetIndex（双 4）**，是 **animIndex/animFrame（blink 期走表缺失）**：
rust `samus_intro` blink（frame/mainloop.rs:200-210）写 (4,4,0) 后调 `anim_draw_object`
（frame/sprite.rs:26——=ObjDrawFrame（DE4A）**仅 draw（不走表）**（刻意拆分注「动画由 update_obj_anim…
此处不写 $0303/$0304/$0306」））→**漏 UpdateObjAnim**（asm DE47 首句 `jsr UpdateObjAnim`）→
animFrame 停留 0（Stand 帧 id（旧值））⇒渲染 Stand sprite（L4 f550 差 210px（sprite-only（位置同）））。

**修径**（porter 端口——非本件 scope）：blink 径在 `anim_draw_object` 前补 `update_obj_anim(mem, 0)`
（A=0（asm blink 径 `lda #$00`→`jmp AnimDrawObject`（DE47（`jsr UpdateObjAnim`（A=0→sta animDelay）））））
——或恢复 DE47 形（anim_draw_object 首句走表）。

**lead 归因更正**（> 【2026-10-27 更正】）：lead 报「rust animResetIndex=7(Stand)+animIndex=8」——
现行 rust state（state.jsonl（f550）实测=**(4,4,0)**（animReset=4（与 emu 同！））——(7,8,·) 实测见 f801
（fade-in 完成帧（7,8,8））——lead 数字可能取自 f801 或旧码态。**discrepancy 核心=animFrame（7 vs 0）+
animIndex（5 vs 4）**（走表缺失），非 animResetIndex。

## 1. anim 状态机四元组模型（已证实（A 级））

### 1.1 四元组与 RAM 映射（bank:addr 引证）

| 字段 | RAM | asm 标签 | 语义 |
|---|---|---|---|
| animResetIndex | $0305 | Samus.animResetIndex | 动画回绕点（ObjectAnimIndexTbl 偏移（走表遇 $FF 回绕至此）） |
| animIndex | $0306 | Samus.animIndex | 当前走表位置（ObjectAnimIndexTbl 偏移） |
| animDelay | $0304 | Samus.animDelay | 走表倒计时（0=立即走） |
| animFrame | $0303 | Samus.animFrame | 当前帧 id（ObjFramePtrTable 索引（渲染用）） |

### 1.2 核心走表（UpdateObjAnim（bank7:DC8F）——已证实（asm 逐行+trace 自洽））

```
UpdateObjAnim (DC8F):
  if animDelay != 0:
      animDelay--
      if animDelay != 0: return          ; 未到走表时刻
  animDelay ← A                          ; A=调用方 delay 初值
  Y = animIndex
  @getAnimFrame:
  frame = ObjectAnimIndexTbl[Y]
  if frame == $FF:                       ; 动画段尾
      Y = animResetIndex                 ; 回绕
      goto @getAnimFrame                 ; 重取帧
  animFrame ← frame
  animIndex ← Y+1
```

- **稳态**（单帧段动画（如 Front=[Front,End]））：(4,5,7) 每 tick 回绕 4→5（animFrame=Front（7））。
- **SetSamusAnim（CF6B）**：A→animResetIndex+animIndex；animDelay←0（立即走表）。
- **SetSamusNextAnim（CF6E）**：A→animIndex；animDelay←0（**不动 animResetIndex**）。

### 1.3 ObjectAnimIndexTbl（$8572）值域（anim index=字节偏移（common_sprite_data.asm（A 级（地址自证））））

| anim | 偏移 | 帧段 | animFrame id |
|---|---|---|---|
| ObjAnim_SamusRun | 0 | [Run0,Run1,Run2,End] | — |
| ObjAnim_SamusFront | **4** | [Front,End] | **7**（emu f550 animFrame） |
| ObjAnim_SamusUnroll | 6 | [Salto0] | — |
| ObjAnim_SamusStand | **7** | [Stand,End] | **0**（rust f550 animFrame（旧值）） |
| ObjAnim_SamusStandFire | 9 | [StandFire,End] | 34（f801 animFrame） |
| ObjAnim_SamusJumpTransition | 11 | [Run1] | — |
| ObjAnim_SamusJump | 12 | [Jump,End] | — |
| ObjAnim_SamusSalto | 14 | [Salto0-3,End] | — |
| ObjAnim_SamusRunJump | 19 | [Run0,Salto0,End] | — |
| ObjAnim_SamusRoll | 22 | [Roll0-3,End] | — |

（animFrame id↔帧名映射以 ObjFramePtrTable 为准（_id_ObjFrame_SamusFront=7/_id_ObjFrame_SamusStand=0
——trace 实值自证（emu animFrame=7=Front 帧（lead 注）/rust animFrame=0=Stand 帧））。

### 1.4 状态/转换模型（动画态×游戏态）

| 游戏态 | 进入动作 | anim 四元组 | 转换 |
|---|---|---|---|
| 淡入闪烁（SamusIntro（Timer3<31（奇帧）） | SetSamusAnim(Front(4))（CA00）→AnimDrawObject→UpdateObjAnim | **(4,5,7)** 稳态 | Timer3=0→sa_Begin+GameEngine |
| 站立（SamusStand（CC36）） | 按钮按下⇒SetSamusStand_NoFootstep(CF5D)→SetSamusAnim(Stand(7)) | (7,8,·)→走表稳态 | 方向⇒sa_Run；JUMP⇒sa_Jump；FIRE⇒StandFire 走表 |
| 跑（SamusRun（CCC2）） | SetSamusRun(CC98)：前态=Stand⇒animReset=Run(0)；前态=Run⇒**turnaround**（SetSamusNextAnim(Front(4))（animIndex only）+animReset=RunPntUp） | Run 走表 | 停止⇒SamusStand；JUMP⇒Jump/Salto |
| 跳（SamusJump（D002）） | SetSamusJump(CFC3)：animReset=Jump(12)/JumpPntUp（sty $0305）；animIndex=reset-1（JumpTransition）；delay=4 | (12,11,·) | 落地⇒Stand/Run |
| 旋（Salto） | SamusRun spin（CC81）：animReset=Salto(14) | (14,·,·) | unspin⇒Jump |
| 球（SamusRoll（D0E1）） | SetSamusRoll(D0B5)：animReset=Roll(22) | (22,·,·) | unroll⇒Jump |
| 电梯（SamusElevator） | animReset=Front(4)（D866/D8AF）/FadeIn/FadeOutArea | (4,·,·) | 电梯完⇒Stand |
| 死亡 | SetSamusExplode(CC8B)：animReset=Explode | — | — |

## 2. $0305（animResetIndex）全写点清单（已证实（A 级（grep 全 SRC+逐点读上下文）））

### 2.1 直接 sta/sty Samus.animResetIndex（12 点）

| # | bank:addr | 写值 | 触发条件 |
|---|---|---|---|
| 1 | prg0_title.asm:440 | （标题径） | 标题屏 |
| 2 | bank7:CCAF（2840） | RunAnimationTbl[x]（Run(0)/RunPntUp） | SetSamusRun（CC98）LCCBX |
| 3 | bank7:CCD2（2870） | Jump(12) | SamusRun 下落 unspin（JumpDsplcmnt<24px） |
| 4 | bank7:CCDF（2881） | Salto(14) | SamusRun 跳跃 spin（≥24px（非 JumpFire）） |
| 5 | bank7:CCEE（2891） | JumpPntUp | SamusRun 跳跃 aim-up（≥32px（UP）） |
| 6 | bank7:CD07（2903） | Jump(12) | SamusRun samL04（run→jump） |
| 7 | bank7:CF6B（3372） | **A（调用方）** | SetSamusAnim（通用 setter） |
| 8 | bank7:CFC7（3437） | Jump(12)/JumpPntUp（sty） | SetSamusJump（CFC3）LCFC5 |
| 9 | bank7:D01D（3487） | JumpPntUp | SamusJump Lx020（aim-up） |
| 10 | bank7:D0C0（3600） | Roll(22) | SetSamusRoll（D0B5） |
| 11 | bank7:D876（4934） | FadeOutArea_Reset | 电梯下降 fade-out（ElevatorScrollY） |
| 12 | bank7:D8F5（5045） | FadeInArea_Reset | 电梯上升 fade-in |

### 2.2 SetSamusAnim（CF6B）调用点（A→animResetIndex（10 点））

| # | bank:addr | A 值 | 触发条件 |
|---|---|---|---|
| 1 | **bank7:CA00（2230）** | **Front(4)** | **SamusIntro 淡入闪烁（奇帧（f497-f800（★f550 写源）））** |
| 2 | bank7:CC8F（2818） | Explode | SetSamusExplode |
| 3 | bank7:CDB2（3050） | RunAnimationTbl[x] | SamusRun_SetAnim（aim 变化） |
| 4 | bank7:CF85（3384） | PntUp(0x27) | SetSamusPntUp（CF77） |
| 5 | bank7:CFAF（3410） | Table06+1,x（Jump(12)） | SamusRun_CheckHorzMovementMidair（CF88）转向 |
| 6 | bank7:D08B（3553） | Table04+1,y/06+1,y（JumpPntUp/Jump） | 跳转向（Lx025/Lx026） |
| 7 | bank7:D0B2（3588） | JumpFire | 跳射击（@notAimingUp） |
| 8 | bank7:D123（3657） | Roll(22) | 球转向（Lx032） |
| 9 | bank7:D866（4847） | **Front(4)** | 电梯进入（ElevatorMove） |
| 10 | bank7:D8AF（4972） | **Front(4)** | 电梯移动（ElevatorMove 后） |

### 2.3 SetSamusNextAnim（CF6E）调用点（animIndex only（不动 $0305））

| # | bank:addr | A 值 | 触发条件 |
|---|---|---|---|
| 1 | bank7:CCA5（2837） | **Front(4)** | SetSamusRun turnaround（前态=run（**仅 animIndex=4**）） |
| 2 | bank7:CDEE（3073） | RunPntUpFire1-3 | SamusRun_Fire aimingUp |
| 3 | bank7:D26B（3897） | StandFire(9) | 站立射击（LD26B） |

**★ 关键证伪**（lead 原结论复核）：A=Front(4)→$0305 的路径**仅 3 处**（2230（淡入闪烁）/4847/4972（电梯））
——f550（游戏内（无电梯））唯一径=**2230（SamusIntro 淡入闪烁）** ✓（trace 写时点 f497 自证）。
SetSamusRun turnaround（CC98）用 SetSamusNextAnim（**只设 animIndex**）——lead 此点考证正确 ✓。

## 3. emu f550 时间线（已证实（A 级（trace（frames.jsonl（逐帧 w 字段）））））

| 帧 | 事件 | anim 四元组 |
|---|---|---|
| f350 | START GAME（press）→SamusInit（Timer3←0x2C=44（440 帧淡入（asm prg7:1981（`lda #$2C`））） | (0,0,0) |
| f351-f496 | SamusIntro 跑（Timer3≥31→**不画 Samus**（LC9F2（`cmp #$1F/bcs`））） | (0,0,0)（未触） |
| **f497** | Timer3<31→**闪烁起**（LCA00（奇帧 SetSamusAnim(Front(4))→(4,4,0)→AnimDrawObject→UpdateObjAnim（A=0）→(4,5,7)）） | **(4,5,7)** |
| f497-f800 | 闪烁期（Timer3 30→1（300 帧（10 帧/递减）））——**移动输入被忽略**（SamusIntro≠GameEngine） | (4,5,7) 稳态 |
| **f801** | Timer3=0→淡入完成→Samus.status←sa_Begin(255)+MainRoutine←GameEngine→GoSamusHandler（`bmi SamusStand`（255（bit7）⇒直跳））→SetSamusStand_NoFootstep(7) | **(7,8,·)** |

（trace 实值（analysis/traces/case-001…frames.jsonl（w.animReset/animIndex/animFrame/animDelay））
：f1-f496=(0,0,0,0)；f497-f800=(4,5,7,0)；f801=(7,10,34,4)——A 级实录。）

## 4. rust 差异点（已证实（A 级（state.jsonl（iram[$0305]（实值）））））

### 4.1 rust f550 实值 vs emu（state.jsonl（/tmp/case1-state.jsonl（rust_screenshot --state-out）））

| 字段 | emu f550 | rust f550 | 差 |
|---|---|---|---|
| animResetIndex ($0305) | **4** | **4** | **同！** |
| animIndex ($0306) | 5 | **4** | **−1（走表缺失）** |
| animFrame ($0303) | 7（Front 帧） | **0（Stand 帧（旧值））** | **渲染帧差（210px 根源）** |
| animDelay ($0304) | 0 | 0 | 同 |

### 4.2 根因（rust 走表缺失点）

- **asm 双入口均先走表**：`SetSamusData(CD6D)=jsr UpdateObjAnim+…+jmp ObjDrawFrame(DE4A)`
  +`AnimDrawObject(DE47)=jsr UpdateObjAnim+ObjDrawFrame(DE4A)`（bank7:DE47（首句 `jsr UpdateObjAnim`））。
- **rust 刻意拆分**（frame/sprite.rs:26（`anim_draw_object`=ObjDrawFrame only（注「动画由 update_obj_anim…
  此处不写 $0303/$0304/$0306」）））——gameplay 径（set_samus_data 链首调 update_obj_anim）✓ 但
  **blink 径（frame/mainloop.rs:200-210（samus_intro））写 (4,4,0)→anim_draw_object（**无 walk**）**→
  animFrame=0（Stand 帧 id（初始值））⇒渲染 Stand。
- **L4 f550 差 210px**（像素级自证（PIL diff（210/61440（sprite-only（位置同（截图自证）））））。
- **修径**（porter 端口）：blink 径补 `update_obj_anim(mem, 0)`（A=0（asm `lda #$00`→`jmp AnimDrawObject`））
  于 `anim_draw_object` 前；或恢复 DE47 形（anim_draw_object 首句走表（gameplay 径去重））。

### 4.3 rust 与 asm 一致面（已核 ✓）

- `samus_init`（mainloop.rs:137）：Timer3←0x2C（440 帧淡入）✓（=asm prg7:1981）
- `samus_intro`（mainloop.rs:175）：Timer3==0→sa_Begin+MainRoutine←3 ✓；Timer3≥0x1F→不画 ✓；
  奇帧 SetSamusAnim(Front)（$0305=4/$0306=4/$0304=0）✓（=asm LCA00）
- `update_timer`（mainloop.rs:1303）：TIMER_DELAY($29) 预分频（10 帧）+TIMER3($2C) 递减 ✓（=asm C266）
- GoSamusHandler `bmi SamusStand`（sa_Begin=255（bit7）⇒直跳 SamusStand）✓
- SetSamusStand_NoFootstep（handler.rs:634）：$0305=$0306=0x07（Stand（UP⇒0x27（PntUp）））✓

## 5. 遗留假设（C 级（不落结论））

- [C] Timer3「310 frames」注释 vs `cmp #$1F`（31）单位差（asm 注释与码不一致（实测=31 单位×10 帧=310 帧窗（注释或指 310 帧后画 Samus（LC9F2 注"When 310 frames left"）））——不影响本结论（trace 时点自证）。
- [C] prg0_title.asm:440（sta Samus.animResetIndex）写值未核（标题径（非 f550 路径））。
- [C] lead 报「rust animResetIndex=7(Stand)」的取值帧/码态未定位（现行码 f550=(4,4,0)（state 实测））。

> 【refs 引用必写】bank:addr 引证（A 级原文）：bank7:CA00（LCA00（闪烁（SetSamusAnim(Front(4))））+
> bank7:CF6B（SetSamusAnim）+bank7:CF6E（SetSamusNextAnim）+bank7:DC8F（UpdateObjAnim（走表））+
> bank7:DE47（AnimDrawObject（`jsr UpdateObjAnim` 首句））+bank7:CD6D（SetSamusData（同首句））+
> bank7:CC36（SamusStand）+bank7:CC1A（GoSamusHandler（bmi SamusStand））+bank7:C9D7（SamusIntro）+
> constants_ram.asm:496（Timer3（10 帧递减））+common_sprite_data.asm:12（ObjectAnimIndexTbl（$8572））
> ——refs/m1disasm（与 ROM 逐字节等价（docs/findings/rom-disasm-equivalence.md））
> 【留档可追溯】docs commit（追溯径（refs 引用（bank:addr）+已证实/假设分区+实测输出原文（A 级）））
