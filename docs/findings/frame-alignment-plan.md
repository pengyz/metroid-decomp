# 帧对齐完整方案——爬虫激活差 3 帧的第一性对抗分析（round r17 前置）

- 日期：2026-10-30
- 作者：独立对抗分析员（全新上下文、证伪立场；只读分析+本文档，未改任何代码/门禁/基线）
- 状态：**方案定稿（待 porter 轮实施）**；对 crawler-corner-divergence.md 2026-10-30 各补注
  的逐条确认/推翻见 §2
- 前置阅读：docs/findings/crawler-corner-divergence.md（分歧地图与 S1-S6 工单原始形）、
  findings/move-drift-iteration-scheduling.md（span 机制定谳）、findings/fc-phase-residual.md

---

## 0. 结论速览（30 秒版）

1. **头号发现（新，仪器级）**：现役 crawlprobe 对拍**双侧采样缝错位**——emu 腿
   `sample_point=endFrame`（run.json 实证）vs rust 腿=spin 出口缝（rust_screenshot.rs
   `--state-out`）。两腿行相位差一整个窗，多帧工作被 endFrame 拦腰切断。
   **"TD 相位差自 f2 起 949 帧"地图是"约定错位"+"真实分歧"的叠加体，不可直接用作施工图**。
   Lua 侧 `$C0D3` spin-exit 采样模式**已存在**（scenario_capture.lua task-24 3b），
   DSL 已支持 `sample_point spin-exit`（scenario_run.py:328）——只是 crawlprobe 用例没写。
   **已失败三次的手工帧算术，死因全部是这一条**（行距推演在混合采样语义下无定义）。
2. **GameEngine 进入差 3 帧 = TD($29) prescaler 相位差的唯一投影**（单因果链，§3.E）：
   T3←44 后首递减发生在 TD 回绕帧，emu/rust 在 SamusInit 行的 TD 相位差 3 ⇒ GameEngine
   进入差 3。TD 相位差 3 = boot 行索引 ±1 + 转换臂（0x1B）边界数差 2。**修好上游两处，
   GameEngine 自动对齐；不存在独立的 game-start 机制缺陷**（game-start 窗净贡献=0，§2-F）。
3. **boot ±1 是行簿记缺陷，不是行为缺陷**（§3.B）：HEAD 的 r14 融合唤醒行为已经正确
   （tick2=NMI#1+boot_post+首迭代）；差的是 rust 把 tick1（PreSpin）空耗为一帧且不出行。
   修=把 boot_pre_spin 内联到 tick1 的 NMI **之前**（严格序），S1 失败版的红是**排序错误**
   （main_loop_iter 跑在 GameMode←1 之前），非模型问题。
4. **0x00 臂的 SpinWait(1) 是伪造帧**（§3.C，A 级反证）：InitializeAfterReset+LoadStarSprites
   全文无任何 WaitNMIPass/ScreenOff/ScreenOn（prg0_title.asm:104-173、:3469-3481）——
   emu 该迭代跨多帧是**重活跨帧（bogged）**，不是内旋。机制错了：NMI 内块行为（bogged 跳
   ReadJoyPads）与 seam 行集（bogged 帧无行）都应按 span 门建模。
5. **六开关收敛裁决（§5）**：不引入"帧模式第一公民"（第 7 个机制）；正确轴=把 title 臂
   的续段/等待/完成 tick **统一到既有 span 语义**（无行、无 prelude、FC 冻结三性质同源）。
   净效果：title_spin 退役、title_no_return 族收缩、cursor 保留（段寻址）、iter_gates
   成为唯一帧界台账、nmi_enabled 保持硬件面。
6. **L4 锚定集重立前提被推翻**（§6）：修的是 rust 向地面真值收敛，emu 腿不动 ⇒ 既有像素
   锚点（f550/650/720/820/960）全部保持有效且预期改善；无需重锚、无需用户确认重锚。
   唯一要用户确认的是：case 文件加 `sample_point spin-exit` 行（仪器配置变更，单独 commit）。

---

## 1. 方法与数据源（本档所有新结论的证据基座）

本档**不做任何手工帧算术**（findings/crawler-corner-divergence.md 2026-10-30 方法论定谳
——三次采样沿干扰教训）。全部结论出自两类可复现原料：

| 原料 | 路径 | 采样点 | 说明 |
|---|---|---|---|
| emu 腿原始行 | `analysis/traces/case-crawlprobe.frames.jsonl`（run.json: `sample_point: "endFrame"`） | **endFrame**（帧渲染完成回调） | Mesen2 Lua，scenario_run.py 产物；960 行无缺 |
| rust 腿原始行 | `shots_rust/case-crawlprobe.state.jsonl` | **spin 出口缝**（tick 开始，帧工作前；rust_screenshot.rs:425 `tick_frame_with` 闭包） | f=1 无行（PreSpin 不采）；f352-358 缺 7 行（span 抑制） |

本会话实测复核脚本（只读，一次性，未入库）：对 FC($002D)/TD($0029)/T3($002C)/MR($001E)
逐行比对。关键读数（值均为十进制 FC / TD；rust 行号=其 f 字段）：

```
boot:      emu f1=[FC0,TD0] f2=[0,9] f3=[1,9] f4=[1,8] … f7=[2,8] f8=[3,7]
           rust f2=[0,0]    f3=[1,9] f4=[1,9] f5=[1,8] … f7=[2,7] f8=[3,6]
转换:      emu f310=[49,5] f311=[49,4] f312=[49,4] f313=[49,4] f314=[50,4] f315=[51,3] f316=[52,2]
           rust f311=[0x31,3]→TD 逐 tick 减；f313 行 TR 已=0x16；f316 行 TD=9（回绕）
game-start: emu f351=[87,7] f352=[87,7] f353=[87,6,MR1] f354-357=[87,6,MR1] f358=[87,6,MR2]
            f359=[88,6,MR2] f360=[89,5,MR8,T3=44]
           rust f351=[0x57,4] …（f352-358 无行）… f359=[0x58,3,MR2] f360=[0x59,2,MR8,T3=2C]
GameEngine: emu f793=[10,2,MR8] f796=[13,9,T3=0,MR3]  rust f793=[0x0A,9,T3=0,MR3]
           （FC 双侧全程相等；MR 差窗=f793-795；TD 差恒 3）
尾窗:      emu f916/f917 FC=132 冻两帧、TD 冻 9 —— rust 照常 +1/+1（未建模的 emu 侧冻结）
```

asm 引证（全部 A 级，refs/m1disasm/SRC/）：
- `prg7_engine.asm:199-223` MainLoop 体序（CheckBankSwitch→UpdateTimer→GoMainRoutine→
  inc FrameCount→NMIStatus←0→自旋@C0CB→**C0D3 出口**→RandomNumbers→jmp MainLoop）
- `prg7_engine.asm:193` **Startup 直入 `MainLoop@loop_waitNMIEnd`（C0CB）**——boot 不经
  首迭代头部，且 boot_post 后 NMIStatus=1 ⇒ 首自旋**立即**穿过 C0D3（零等待）
- `prg7_engine.asm:654-677` UpdateTimer：`dec TimerDelay/bpl`——TD=0 时回绕 0→FF→**重载 9**
  且该帧多减 Timer3（10 帧 prescale）
- `prg7_engine.asm:1108-1146` WaitNMIPass/WaitNMIPass_（ClearNMIStat+自旋）、ScreenOff/
  ScreenOn（**两者尾部都带 WaitNMIPass_**）、`C487` NMIOn（**读 PPUSTATUS 自清 vbl 旗**
  ——两次读内退出，零帧消耗）
- `prg0_title.asm:104-173` InitializeAfterReset（**无任何内旋**；尾 `inc TitleRoutine`+
  `jmp LoadStarSprites`）、`:3469-3481` LoadStarSprites（无内旋）
- `prg0_title.asm:182-209` DrawIntroBackground（`jsr ScreenOff` ①…`inc TitleRoutine`…
  `jmp ScreenOn` ②——**2 个 NMIStatus 内旋**）
- `prg0_title.asm:2316-2350` ClearAll（ScreenOff①→ClearNameTables→EraseAllSprites→
  `jsr WaitNMIPass`②→jmp VBOffAndHorzWrite[无旋]）+StartContinueScreen（…→**落穿**
  TurnOnDisplay：NMIOn[零帧]+jmp ScreenOn③）——**3 个内旋**
- `prg0_title.asm:2769-2788` InitializeStats（纯写+BankSwitchPending←2，无旋；**被
  ChooseStartContinue `jmp` 同迭代调用**）
- `prg7_engine.asm:1376-1394` InitBank0（GameMode←1 **在 ScreenNmiOff 自旋之前**）、
  `:142-144` Startup GameMode←1（同样在自旋前）、`:1826-1930` AreaInit（inc MR 后
  **无 RTS 落穿 MoreInit**）与 MoreInit（ScreenOff 内旋→…房间环+4 页 PPU 拷→inc MR→
  `jmp ScreenOn` 内旋）
- `tools/scenario_capture.lua:502-600`：`sample_point=spin-exit` 模式=**$C0D3 exec 回调**
  （task-24 3b，once-per-frame 守卫）——与 rust 缝（NMI 尾写 1 后/帧工作前，engine.rs:98-110
  注释锚 $C0D3）**同相位定义**
- `tools/scenario_run.py:328-333`：DSL `sample_point spin-exit|endFrame` 解析已实现

---

## 2. 对抗分析结论——对既有结论的逐条确认/推翻清单

每条标注【确认】【推翻】【新发现】与证据级（A=asm/字节/原始行；B=可复现静态推理）。

### F1【新发现，A 级】对拍仪器采样缝错位——三次失败尝试的共同死因

- emu crawlprobe 腿 `sample_point=endFrame`（`analysis/traces/case-crawlprobe.run.json`
  `"sample_point":"endFrame"` 直证）；rust 腿=spin 出口缝（`--state-out` 行）。
- 两缝相位差一整个窗：rust 行 f=pre-tick-f（=后 tick f-1 的工作），emu endFrame 行 f=
  帧 f 全窗工作之后。**同一行号比的是不同时刻**。多帧工作（0x00 臂 WRAM 填充、MoreInit
  房间环）被 endFrame 从中间切开，产出"半工作态"行——rust 缝在架构上**永远产不出**
  这些行（3/952 行匹配的实测，任何常量偏移都拟合失败：0/1/2/3 偏移全试，见 §1 脚本）。
- un-fuse 转换臂实验（r16 前回滚）死于"行距推演"——行距在混合语义下无定义，**死因
  诊断正确但归因对象错误**：不是"emu prelude 帧集不可知"，是"观测仪器没对齐"。Lua 的
  $C0D3 模式已存在且语义与 rust 缝同锚（task-24 当年就是对齐设计的），只是用例没启用。
- **推论（推翻"既定仪器=UpdateTimer Lua hook 另取真值"计划）**：不需要新仪器，需要
  **启用既有仪器**。Step 0（§4）=case 文件加一行 + 重跑 + 重立分歧地图。

### F2【推翻】"boot：rust 整个 MainLoop 流晚 1 帧启动（行为缺陷）"

- HEAD 行为已正确：tick2=NMI#1+boot_post+首迭代融合（engine.rs:116-128 r14 件），
  与 emu 的 boot 链序（Startup 直入 C0CB 自旋→NMI#1 唤醒→boot_post→**NMIStatus=1
  残留⇒C0CB 立即穿过**→C0D3→RandomNumbers→首迭代）逐拍对应（A 级：prg7:193+
  engine.rs 对照）。
- 实测差异（rust f2=[FC0,TD0] vs emu f2=[FC0,TD9]）是**行索引错位**：rust tick1
  （PreSpin）不出行但占行号，emu 首个缝行是行 1。S1 失败版把"合帧"理解成搬行为，
  实际只需搬**行的出生点**+把 boot_pre 挪进同一 tick 的 NMI 之前（§3.B）。
- `$0001=0x67` 断言红的机理（B 级复盘）：失败版在 GameMode←1（boot_pre_spin 内，
  boot.rs:73 与 c531_unit_pre:305）**之前**执行了 main_loop_iter（或打乱了 NMI-before-work
  的 tick 内序），GoMainRoutine 读 GameMode=0 走游戏路分派 AreaInit——纯排序违例。
  正确序（§3.B 五步线性序）下 GameMode 在首迭代前必为 1（asm 两处写点都在自旋前：
  prg7:142-144、:1377-1380）。

### F3【推翻】"0x00 臂 spin 1 帧"（S2 工单的前提）

- A 级反证：InitializeAfterReset（prg0_title.asm:104-173）与 LoadStarSprites（:3469-3481）
  全文无 WaitNMIPass/ScreenOff/ScreenOn/VBOff 调用——**0x00 臂没有任何内旋**。
- emu 该迭代跨多帧是**重活跨帧**（2KB WRAM 表驱填充+星精灵拷贝 ≈ 数万周期）——真实
  机制=bogged（NMIStatus=1 从 boot 残留、GoMainRoutine 未返回不清 ⇒ NMI 内块跳过，
  prg7:251 门）。
- 机制差异可观测：①NMI 内块（bogged 跳 ReadJoyPads/palette/VRAM 写 vs 等待帧全跑）；
  ②seam 行（bogged 帧无 C0D3 行 vs rust SpinWait tick 出行）；③FC 时点。三者都要求
  用 `iter_bogged`+`iter_span_fc` 建模，**SpinWait(1) 退役**（§3.C）。
- S2 工单的"0x00 入冻结族+spin 末尾 no_return=false"规则**作废**——被 span 的
  deferred-FC 机制（iter_span_fc，engine.rs:150-163 既有）取代，无新增规则。

### F4【确认+精化】0x01 臂与 0x1B 臂 rust 边界数不足；un-fuse 方向正确、死于仪器

- 0x01：asm=2 个 NMIStatus 内旋（ScreenOff①+ScreenOn②，prg0_title.asm:188/209）；
  ClearNameTables/PreparePPUProcess 无旋（prg7:366-431、C20E 直证）。emu 实测
  prelude→完成跨 3 个采样窗（TD dec@f4 / FC inc@f7），即 2 旋+工作溢出。rust 现 2 tick
  （cursor0+cursor1 融合段）+完成并入下迭代——**边界数不足且完成拍被折叠**。
- 0x1B：asm=3 个内旋（ClearAll 内 ScreenOff①+WaitNMIPass②+落穿 TurnOnDisplay 的
  ScreenOn③；NMIOn 零帧——PPUSTATUS 读自清，prg7:1202-1211 两次读内退出）。
  emu 实测 4 窗（f311 冻/f312 冻/f313 冻+TR→0x16/f314 FC inc）；rust 现 2 tick。
  **差 2 边界**——与 TD 相位差从 +1（f310 前）涨到 +3（f316 起）的 +2 增量精确吻合
  （原始行直证，§1）。
- r16 un-fuse 回滚案的 L4 41 违规**不再构成对 un-fuse 的证伪**：那是端到端像素在
  时间线平移下的必然（见 F7），且当时无 matched-seam 行集做验收。S4 重启（§3.E）。

### F5【确认，A 级】GameEngine -3 = TD 相位投影；单因果链闭合

- 链条（原始行全程可追）：SamusInit 行 TD 相位 emu=5 vs rust=2（差 3）→ 首递减=
  TD 回绕帧（UpdateTimer `dec/bpl` 语义）⇒ emu f366 vs rust f363（差 3）→ T3 每 10
  调用递减 ⇒ T3=0 到达帧 emu f796 vs rust f793（差 3）→ SamusIntro 见 T3==0 ⇒
  MR←3（差 3）。**44 次递减双侧计数相同（纯相位非速率）**——crawler-corner-divergence.md
  2026-10-30 续挖注的判断成立，本档补齐最后一环：game-start 窗净贡献=0（F6）。
- 因此**不存在**"GameEngine 进入时机"独立机制；修 boot 行索引（-1）+0x1B 边界数（-2）
  后 MR 差窗自动消失。禁止任何"TD 注入偏移"类捷径（伪对齐，违 §5 门禁纪律）。

### F6【推翻（降级）】"S5 game-start span 拆分"是 MR-3 的必要步骤

- 实测：f360 行双侧 FC=89/MR=8/T3=44 **全等**（TD 差 3 为上游 inherited）——game-start
  窗对 TD 相位差的**净贡献为零**（rust 的 [wait,bogged×2,wait] 门结构近似已够格）。
- 保留的差距是**行集形状**：rust 缺行 f352-358（7 行）vs emu endFrame 全行——matched-seam
  下 emu 该窗同样缺行（MoreInit 长工作无 C0D3），**缺行区间是否重合**才是验收项。
  MoreInit 内旋/重活的精确门序列（emu 实测 TD 冻 f354-358 共 5 窗 vs rust bogged(2)）
  由 Step 0 图谱定数，**禁止手工推**。S5 降级为"行集对齐面"并入 Step 4。

### F7【推翻】"GameEngine 进入帧移动 ⇒ 锚定像素集需重立+基线重锚（用户确认）"

- 方向性错误：所有修复都是 **rust 向 emu 地面真值收敛**，emu 腿（含全部截图与锚定帧
  f420/550/650/720/820/960 的 emu 侧）一个字节不动。锚点是"同帧号双侧对比"，修复后
  rust@f550 的状态向 emu@f550 靠拢——**既有锚点只会改善**。若某锚点反而回退=修复有 bug
  （回滚该步），不是重锚理由。
- `sample_point spin-exit` 只改 trace 行采集时点，**不改任何渲染帧/PNG/像素**——不触
  L4 白名单、不动基线。需要用户知悉的仅是仪器配置 commit（§4 Step 0），非"重锚确认"。

### F8【确认】f916-917 emu 冻结未建模（独立面，不阻塞主线）

- 原始行：emu f915/f916/f917 FC=132/132/132、TD 冻 9（两窗无迭代完成），rust 照常递增
  ⇒ Samus x 漂移 +2px（f918 起）。MR=3 游戏相内、hold 900-950 right 中——疑似某重活
  （爆源不明）。matched-seam 重跑后 emu 缺行区间会直接暴露该窗，届时按门序列补
  （Step 5 开放项，不阻塞爬虫激活主链——激活差在 f793-798，先闭合）。

### F9【确认】SS 15 帧簇 = T3 纪元下游投影（无独立修）

- fade 步进由 T3 阈值驱动（SamusFadeInTimeTbl 门，mainloop.rs samus_intro:196-202），
  T3 纪元 +3 ⇒ 步进帧 +3，簇宽由表间隔放大为 15——F5 闭合后自动消失。crawlprobe 的
  SS 列纳入逐行验收即可。

### F10【新发现，A 级】rust 续段（continuation）tick 泄漏 seam 行

- emu 臂内帧无 C0D3 执行 ⇒ **无行**；rust 的 continuation tick（cursor>0 或 spin>0）
  走 mainloop 逻辑但**照常触发出口缝**（engine.rs:108-110 只挡 span 不挡续段）⇒ 多出行
  （如 0x01 cursor1 tick 的行 f5/f6 区段）。matched-seam 下行集必须同构 ⇒ 续段 tick 的
  缝必须抑制（§3.A 一行守卫扩展）。这是"六开关"里 continuation 与 span 的**本质同构**
  的直接证据（§5 裁决依据）。

---

## 3. emu 帧结构定谳表（asm A 级骨架 + 实测修数；施工对照基准）

> 「边界」=一次 NMI 等待（一个无行窗）。「缝行」=C0D3 执行（=MainLoop 迭代界，双侧同定义）。
> 「bogged(n)」=连续 n 个 NMIStatus=1 的重活窗（无行）。「定数待图」=asm 推不出周期数、
> 由 Step 0 matched-seam 行集缺口宽度机械读出——**禁止手工帧算术补**。

| 窗 | asm 结构（锚） | 边界/门序列 | rust 现状 | 差 |
|---|---|---|---|---|
| boot | Startup(:193) 直入 C0CB；ScreenNmiOff 自旋=唯一 boot 等待；NMIStatus=1 残留⇒穿 C0D3 零等待 | 1 缝行（首迭代界）；GameMode←1 在自旋前（:142-144/:1377-1380） | tick1 PreSpin 空耗且不出行 | 行索引 -1（非行为） |
| 0x00 | 无内旋（:104-173/:3469-3481）；尾 inc TR+jmp LoadStarSprites | 工作 tick + bogged(n₀)+FC@span 尾；n₀=定数待图（emu FC inc#1 与 TD dec#2 隔 1 窗） | SpinWait(1)+FC@工作拍 | 机制错+完成拍折叠 |
| 0x01 | ScreenOff①→ClearNameTables→PrepareVram→**inc TR**→ScreenOn②（:182-209） | 入口拍+①+段拍+②+完成拍（2 旋+溢出；TD dec→FC inc 实测隔 3 窗） | 2 tick（段融合）+完成并入下迭代 | -2 边界+完成折叠 |
| 0x1B | ClearAll（①②）+体+落穿 TurnOnDisplay（ScreenOn③；NMIOn 零帧）（:2316-2350） | 入口拍+①+ClearAll 尾拍+②+段拍（TR→0x16 在此）+③+完成拍 = 4 窗 | 2 tick（r9 融合形） | -2 边界 |
| 0x16 菜单 | 无旋（:2352-…） | 每迭代 1 窗 | 1 tick | ✓（f315-350 稳态对齐实测） |
| game-start | CheckBankSwitch 内 InitBank1（GameMode←0+ScreenNmiOff 自旋）**在 UpdateTimer 前**→gfx→prelude→AreaInit+MoreInit 头→ScreenOff 旋→房间环+PPU 拷（TD 冻 f354-358 实测 5 窗）→inc MR→ScreenOn 旋→完成 | 门=[wait, bogged(定数), wait]；InitializeStats 无旋同迭代完成 | [wait,wait,bogged(2),wait] | 行集形状待图（净 TD 贡献=0 实测） |
| SamusInit→intro | T3←0x2C（:C8D1 族；rust mainloop.rs:141 ✓）→SamusIntro T3==0⇒MR←3 | 每迭代 1 窗；T3 由 TD 回绕驱动每 10 窗递减 | 同构 | 无（上游相位修复后自动齐） |
| 尾窗 f916+ | 不明重活（MR=3 内） | 冻 2 窗 | 未建模 | 开放项（F8） |

---

## 4. 完整方案——逐文件改动清单

> 实施者假设：新会话。开工前只读预检 `python3 gates/cli.py status`；每步一个 commit；
> 触及 reference/data/rom 面的本方案**不涉及**（全部在 crates/、cases/、tools 无关面）。
> 每步验收命令统一为 §5 的 crawlprobe matched-seam 对拍；单元测试红=先判断"断言的是
> 旧契约还是真回归"，只允许"契约迁移"式修改（断言意图不变、锚点更新），**禁止删弱**。

### Step 0（先行，独立 commit）：仪器对齐——启用 spin-exit 采样并重立分歧地图

文件：
1. `cases/case-crawlprobe.txt`：加一行 `sample_point spin-exit`（解析已支持，
   scenario_run.py:328；Lua 走 $C0D3 exec 回调，scenario_capture.lua:590-596）。
   **不改 frames/press/capture/watch 任何行**（像素锚点不受影响——F7）。
2. 同样处理**用于 trace 级对拍的用例**：`cases/case-fcprobe.txt`（FC 相位专用探针）；
   `cases/case-001-*.txt` 若其 frames.jsonl 参与 L1-L3 行对拍（`gates/cli.py trace`
   消费面）——逐个检查其 run.json 的 sample_point 字段，凡参与行对拍的一律加。
   只做像素锚的用例**不动**（endFrame 与像素无关）。
3. 重跑双腿，产出**分歧地图 v2**（matched-seam）：逐行 FC/TD/T3/MR/TR/SS 相等性 +
   **双侧缺行区间集合的相等性**（缺行集也是对拍对象：单侧缺行=分歧，不允许"跳过缺失行"
   的单向容忍）。

验收：run.json `"sample_point":"spin-exit"`；地图 v2 落档（本文件附注或新 findings 档）。
预期地图 v2 形态（用于后续步骤的定数）：boot 区 rust 行整体后移 1 行的错位消失与否
（此时 rust 仍未修，预期 boot 错位**显形为纯行号差 1**：rust 行 k 内容==emu 行 k-1 内容）；
0x01/0x1B 窗的 emu 缺行宽度（=该臂内旋+溢出窗数，n₀/段数由此读出）。

风险：无（只读重跑+用例一行）。回滚：revert 用例行。

### Step 1：boot 行索引修复（S1 重定义——行为不动，只动 tick 内序与行出生点）

文件：`crates/game-core/src/frame/engine.rs`

改动（tick_frame_with 开头，NMI 调用**之前**插入 PreSpin 内联；严格保持五步线性序）：

```rust
// tick1=完整 boot 帧：boot_pre（GameMode←1 在内）→ NMI#1 → boot_post → 缝行 → 首迭代。
// asm 序锚：prg7:142-144/:1377-1380（GameMode 写点均在 ScreenNmiOff 自旋前）+
// :193（直入 C0CB，NMIStatus=1 残留⇒零等待穿 C0D3）。
if self.phase == BootPhase::PreSpin {
    super::boot::boot_pre_spin(&mut self.mem);
    self.phase = BootPhase::AwaitNmi; // 同 tick 内继续，不 yield
}
// ……（既有 NMI 块不动：skip_next_nmi/nmi_enabled 判定）……
```

AwaitNmi 臂保持现状（boot_post→on_spin_exit→main_loop_iter→MainLoop）——即原 tick2
内容原样落在 tick1。删除 `BootPhase::AwaitNmi` 的跨帧到达路径后，该变体仅存在于
tick 内中途（可保留枚举，加注释"单 tick 内瞬态"）；`restore_mss`（phase=MainLoop、
skip_next_nmi）路径**零改动**。

测试迁移（engine.rs tests）：
- `tick_frame_runs_boot_unit_before_detonation`：tick1 后断言 phase==MainLoop（原
  AwaitNmi）；$0355/$0200/$0001/nmi_enabled/NMI_STATUS 五断言**原样保留**（意图=boot
  已跑+面态正确，锚点不变）。
- `spin_exit_seam_mainloop_only_constant_one`：注释"tick4"改"tick3"（首主循环采样提前
  1 tick），断言本体不动。
- `restore_mss_starts_from_snapshot_state_not_boot`：不动（skip 首 NMI 语义独立于 boot 相位）。
- `fc_increments_per_tick_not_delayed_by_span`：阈值断言不动。

验收（crawlprobe matched-seam）：**rust 行 1 存在**且 ==emu 行 1（FC0/TD0/pre-everything）；
boot→0x00 窗 rust 行 k == emu 行 k-1 的错位消失——TD dec#1/FC inc#1 落在**同行号**。
预期地图 v2→v3 变化：f1-f10 区间逐行相等（除 0x00 完成拍外）。

风险：NMI 时序面（r14 调优史）——但行为零变化（同 tick 内序=原 tick2 的序），风险实质
在测试迁移。回滚点：单独 commit，revert 即回。

### Step 2：0x00 臂 SpinWait→bogged span（机制矫正，S2 作废重写）

文件：
1. `crates/game-core/src/frame/title_bridge.rs`：
   - 删除 `0x00` 臂的 `s.shell.push(ShellCall::SpinWait(1))`；
   - `main_title_routine` 的 0x00 分派后追加 `mem.iter_bogged(N0); mem.iter_span_fc = true;`
     （N0=Step 0 图谱读数，起点先取 1，图谱写实后改定）；
   - FC 冻结旗：0x00 工作拍不得自增 FC——现 `title_no_return` 族（matches 0x01|0x15|0x1B）
     **不含 0x00** ⇒ 加 0x00 入族（工作拍冻结），完成由 iter_span_fc 在 span 尾收口
     （engine.rs end_of_tick_gate 既有路径），**不需要** S2 的"spin==0 ⇒ no_return=false"规则。
2. `crates/game-core/src/frame/title/mod.rs`：0x00 臂 dispatch（`0x00 =>`）去掉 SpinWait 壳，
   其余事件面不动。
3. `ShellCall::SpinWait` 变体与 `mem.title_spin` 字段：**本步暂留**（0x1C/0x17 臂仍在用），
   注记退役计划（Step 4 后清理）；`main_loop_iter` 的 continuation 判定中 `|| mem.title_spin > 0`
   保持（0x1C/0x17 兼容）。

验收：boot 后第一臂的行集={工作行, 缺行×N0, 下一行}，与 emu 缺行区间**重合**；
FC=1 首现行号与 emu 相同；TD dec#2 行号与 emu 相同。
预期地图变化：0x00 窗从"逐行错 1"变"逐行相等"。

风险：NMI 内块行为面（bogged 跳 ReadJoyPads——boot 后无输入语义，实际无感；但
`press 310` 在 f310，远离本窗）。回滚点：独立 commit。

### Step 3：续段 tick 缝抑制（F10——六开关统一的落点，一行守卫）

文件：`crates/game-core/src/frame/engine.rs`

tick_frame_with 的缝守卫由 `!in_span` 扩为 `!in_span && !arm_internal`，其中：

```rust
// 臂内帧（asm 例程体内自旋/续段/完成拍——无 C0D3 执行 ⇒ 无缝行；与 span 同构）。
// 判定用 tick 首态（cursor/spin 为上一拍分派遗留——与 main_loop_iter 的
// continuation 谓词同源同值，r14 F3/F4 语义不动）；0xFF=完成拍哨兵（Step 4）。
let arm_internal = (mem.title_cursor > 0
                    && mem.title_cursor != 0xFF
                    && mem.title_cursor_phase == mem.read_ram(crate::ram_map::TITLE_ROUTINE))
                   || mem.title_cursor == 0xFF
                   || mem.title_spin > 0;
```

`FrameMemory` 上提为方法（`crate::state` 定义处）`pub fn title_continuation(&self) -> bool`，
engine.rs 与 mainloop.rs:35 共用**同一实现**（消灭双处手写谓词——这是本步的第二目的：
谓词单源化）。

验收：0x01/0x1B 臂内 rust 多出的行消失（行集缺口形状与 emu 缺口**初步**重合——段工作
分布还没改，先只看"多出的行"消失）；稳态段（0x16 菜单、intro）行不受影响（cursor==0）。
预期地图变化：行数下降，缺行区间出现。

风险：与 span 判定的先后交互（in_span 优先级）——守卫顺序：span 判定（取 tick 首态，
l3-dump.rs:176-178 既有约定）优先，arm_internal 只在非 span 时再挡缝。回滚点：独立 commit。

### Step 4：title 臂边界数矫正（S3+S4 合并实施；0x01 与 0x1B 同轮）

**tick 模型先行（本步的设计基座，防再犯 r16 的窗口模型错误）**：rust 一个 tick=
emu 一个窗——[NMI 处理 → 自旋出口工作 → 隐式等待到下一 NMI]。**自旋的"等待"是窗尾，
不是独立窗**；自旋出口后的下一段工作是**下一个窗的开头**（=下一个 tick 的工作）。
因此：臂内每段真实工作=一个 cursor 步；**不需要也不允许插入"零等待空步"**
（r17 分析初版此处犯过错，原地更正：空步会凭空多出窗）。跨帧的重活溢出才用
iter_bogged 门（span tick=工作仍在进行的窗，无行）。

**完成拍机制（单一哨兵，替换 S2 的旗规则）**：分派器以 `next_cursor=0xFF` 声明
"下一 tick=完成拍"（rts+inc FC，无 prelude、无分发、无行）。0xFF 与相位值域
（<0x1E）无碰撞；F4 孤儿规则**零改动**（孤儿=cursor>0 且 0<cursor<0xFF 且相不匹配
=新相首迭代，语义不变）。完成拍判定加在 `main_loop_iter` 开头：

```rust
// 完成拍（臂末步哨兵）：asm 的 rts+inc FrameCount——无 prelude（自旋出口即返回，
// 不再过 RandomNumbers/CheckBankSwitch/UpdateTimer）、无行（Step 3 谓词覆盖：cursor>0）。
if mem.title_cursor == 0xFF {
    mem.title_cursor = 0;
    mem.write_ram(FRAME_COUNT, mem.read_ram(FRAME_COUNT).wrapping_add(1));
    mem.write_ram(NMI_STATUS, 0);
    return;
}
```

（此时 TITLE_ROUTINE 已被段步推进 ⇒ `title_no_return` 旗自为 false，FC 自增不被拦——
旗面逻辑零改动。）

文件：`crates/game-core/src/frame/title/mod.rs`（dispatch_step）+ `title_bridge.rs`（哨兵
透传：`step.next_cursor` 已原样存回，无需改——只需确认 0xFF 不被任何 match 臂拦截）+
`mainloop.rs`（完成拍分支）。

1. **0x01 臂**（asm：入口段+ScreenOff① / 段工作+ScreenOn②；段工作若跨帧由图谱定 n₁）：

```text
cursor 0: APU 事件面+ScreenOff 面（现事件不动）；next_cursor=1
cursor 1: 段工作=ClearNameTables+PrepareVram($82F4)+体写+**inc TR→0x02**+ScreenOn 面
          （现 cursor1 事件面拆出相位推进保留）；
          若 Step 0 图谱显示该段跨帧（预期 n₁=1：emu prelude→完成隔 3 窗、旋只有 2 个，
          差 1 窗=溢出）：追加 mem.iter_bogged(n₁)；
          next_cursor=0xFF
[span tick ×n₁（如有）] → 完成拍（哨兵）→ 下一迭代（prelude+0x02 FadeInDelay）
```

2. **0x1B（|0x15）臂 4 步**（asm：入口段+① / ClearAll 尾段+② / VBOff+段+③ / 完成；
   emu 实测 4 窗无溢出——w311 入口/w312 ClearAll 尾/w313 段(TR←0x16)/w314 完成(FC inc)）：

```text
cursor 0: ClearAll seg0（现事件：$FE&=~$18+$1A←0）+ScreenOff 面；next=1
cursor 1: ClearAll 段1=ClearNameTables+EraseAllSprites+DecSpriteYCoord 面+scroll 清+
          $FF&=$FC/$FC/$FD←0+$1A←0（现 clear_all_seg1_events 原样）；next=2
cursor 2: ClearAll 尾=VBOffAndHorzWrite 面+PrepareVram($9984)+StartContinue←0+
          PaletteDataPending←$0D+**TR←0x16**+NMIOn 面+ScreenOn 面
          （r16 拆分本体在 matched-seam 验收下重落）；next=0xFF
cursor 0xFF → 完成拍 → 0x16 迭代（prelude）
```

   （0x15 共体同构——同一段代码。）

3. **0x1C/0x17 臂**：现 3 段+SpinWait 形在本轮**不动**（GameOver/密码屏不在 crawlprobe
   时线；标注随件——其收尾步迁移到哨兵机制延后，避免无对拍保护的改动）。

4. `mainloop.rs`：`main_loop_iter` 开头加完成拍分支（上码）；continuation 判定改调
   Step 3 的单源谓词（0xFF 不算 continuation——`cursor>0` 判定改为
   `cursor > 0 && cursor != 0xFF`，或谓词内显式排除）。

验收（crawlprobe matched-seam，预期行变化逐项）：
- 0x01 窗：rust 缺口宽度（cursor1 后的 span tick 数+完成拍无行）==emu 缺口宽度；
  T3←8（0x02 FadeInDelay）行号与 emu 一致（实测 emu=f8）。
- 0x1B 窗：TR←0x16 行号、FC 恢复递增行号（emu=f314/f315）、TD 回绕行号（emu=f316）
  三点逐行相等；行集缺口 {入口拍后 3 拍} 与 emu 重合。
- **TD 相位差在 f316 后归零**（-2 项到位）；GameEngine 进入差随 Step 1 归零（§2-F5
  单因果链的终验点：MR 差窗 f793-795 消失）。

风险：本步是行为变更主体。r16 的教训（像素集缩水）在 matched-seam 行集验收+既有 L4
锚点双保险下受控；任一验收红 ⇒ 按 §5 步骤就地停，不并轮。回滚点：0x01 与 0x1B **分两个
commit**（可独立 revert）。

### Step 5：game-start 行集对齐 + 尾窗开放项调查

文件：`crates/game-core/src/frame/mainloop.rs`（more_init 的门序列）、`boot.rs`（init_bank1
门序列，如图谱要求）。

- 以 Step 0 图谱的 emu 缺行区间为准调整 `init_bank1`/`more_init` 的
  `iter_wait/iter_bogged` 计数（现形 [wait,wait,bogged(2),wait]；emu 实测 TD 冻 5 窗
  提示 interior≥3——**定数从图谱读，不从本档抄**）。`move-drift` 金腿窗（迭代分类面）
  若受影响：跑 `gates/cli.py trace` 确认 L1-L3 无回退（门计数变化会平移 span 窗，
  金腿行集若红=重新核对该窗 asm 结构，不允许调阈值）。
- f916-917 冻结（F8）：matched-seam 地图上定位 emu 缺行区间，对照 MR=3 内当时的
  敌情/弹况（crawlprobe 敌槽 watch 列已有），识别重活来源后按同法补门。**独立子步**，
  允许落在主线验收之后。

验收：game-start 窗行集重合 + FC/TD/T3/MR/SS 逐行相等（f350-f370）；case-001 双腿
trace 对拍（`gates/cli.py trace`）无回退。

### Step 6：全链验收与既有门禁

1. crawlprobe matched-seam 全 960 帧：行集相等 + FC/TD/T3/MR/TR/SS 逐行相等
   （f916-917 开放项除外，显式记账）。
2. 爬虫激活终判：敌槽 slot4/5 激活行双侧同帧（emu f797-799 的 d5 bit1/bit3 两帧置位
   序列 vs rust 同相）——crawler-corner-divergence.md 的首分叉判据闭合。
3. `python3 gates/cli.py check`（总闸）+ `trace`（L1-L3）+ `visual`（L4 既有锚点，
   预期改善或持平；回退=本方案某步有 bug，按 §5 处理）。**不重锚、不动白名单、不更新
   基线**（F7；若 trace 类基线因行集同构化产生格式性变化，按 baseline_update 工具的
   正常流程单独 commit 并注明）。
4. 复核轮：verifier 角色（非实施方）按 §1 数据源独立重导 F1-F10 结论。

---

## 5. 风险与回滚点（汇总）

| 步 | 风险 | 概率/影响 | 回滚 |
|---|---|---|---|
| 0 | sample_point 变更后部分历史脚本假定 endFrame 行集 | 低/低 | revert 用例行；工具双模并存 |
| 1 | boot 测试迁移漏断言；restore 路径回归 | 低/中 | 独立 commit；restore_mss 测试是哨兵 |
| 2 | N0 定数错 → boot 窗行集差 | 中/低 | 图谱重读改常量（一行） |
| 3 | 谓词单源化误伤 0x1C/0x17 的 SpinWait 路径 | 中/中 | 谓词含 `title_spin>0` 项，兼容保留；独立 commit |
| 4 | 主体行为变更；像素锚回退；F4 与 completion 判定互斥破洞 | 中/高 | 0x01/0x1B 分 commit；任一锚回退即停轮定位 |
| 5 | 门计数平移触发 move-drift 金腿红 | 中/中 | 停轮按 asm 重核；禁止阈值容忍 |
| 全局 | 「顺手修」f916 等开放项混入 | — | 开放项只允许独立子步+独立 commit |

停轮纪律：任何一步验收红 ⇒ §4 动手前三问定位层（asm 投影/rust 移植/仪器），修复根因，
**禁止调阈值/加白名单/改基线掩盖**。

---

## 6. 开放问题清单

1. **f916-917 emu 冻结的来源**（F8）：MR=3 内两窗无迭代完成。Step 5 子步定位；
   候选嫌疑（C 级，仅记录）：敌槽激活后的首帧重活/弹面初始化——无证据，不许据此动手。
2. **0x00 溢出窗数 n₀ 与 MoreInit interior 精确门序列**：asm 周期数推不出（正是"手工帧
   算术不可靠"的边界），Step 0 图谱机械读数后回填本档 §3 表（原地补注，带日期）。
3. **Mesen endFrame 回调的确切扫描线相位**：Step 0 后与对拍无关（双缝同相），仅存档
   备查——若未来要做逐**扫描线**级对拍才需钉死。
4. **0x1C/0x17/0x12 臂的 SpinWait→completion 机制迁移**：无对拍时线保护，随件（需要
   先建 GameOver 路径探针才有验收面）。
5. **案例矩阵其余用例的 sample_point 普查**：Step 0 只处理参与行对拍的用例；全矩阵
   普查（哪些 trace 腿在什么采样点）作为工具债登记。

---

## 7. 假设区（未证实，禁止据此动手）

- H1：f916-917 冻结与敌槽激活重活同源（C 级直觉，无证据）。
- H2：0x01 臂的 3 窗间距中存在一段工作溢出（而非第三个隐藏内旋）——依据=ClearNameTables/
  PreparePPUProcess/V RAMStructWrite 均无 WaitNMIPass（A 级反证已列），但"溢出落在
  入口段还是段拍之后"未钉；图谱会给出（若缺口形状与 2 旋+1 溢出模型不符，回炉 asm）。
- H3：~~completion_tick 与 F4 孤儿规则的互斥条件~~ **已消解**（Step 4 终稿改用
  `next_cursor=0xFF` 哨兵显式声明完成拍，与 F4 孤儿规则值域不相交——0xFF 不在任何
  合法相位/游标值域内）。残余假设：现有全部臂的 next_cursor 链中无其他臂需要完成拍
  （0x12 的 cursor 机不走完成拍，天然无涉；0x17/0x1C 随件迁移时再核）。
- H5（新）：0x01 段工作溢出窗数 n₁=1（由"emu prelude→完成隔 3 窗、NMIStatus 旋只有
  2 个"的差值反推）——落实施时以 Step 0 图谱缺口宽度为准，不符则回炉 asm 找第三内旋
  或段切分遗漏。
- H4：case-001 与 crawlprobe 输入相（press 310/350）在修复后无需重调——输入锚在
  inputPolled 注入面，与采样缝无关（A 级：两机制独立）；但 case-001 的**语义场景**
  （标题停留时长）若依赖 TD 相位，其预期事件帧会平移——验收时以 emu 腿为准读数即可。

## 8. 证据指针汇总

- asm：prg7_engine.asm:142-144/193/199-223/251/654-677/1108-1211/1376-1394/1826-1930；
  prg0_title.asm:27-55/104-209/2316-2350/2769-2788/3469-3481
- rust：engine.rs:86-163（tick 序+span+FC 收口）、mainloop.rs:31-53（迭代序门+FC 冻结）、
  :117-127/138-171/1683-1755（AreaInit 落穿/SamusInit/MoreInit 门）、title_bridge.rs:20-131
  （旗/spin/jumped）、title/mod.rs:841-1143（dispatch cursor 机）、boot.rs:48-100/303-317
  （boot_pre/Post 序）、rust_screenshot.rs:410-445（rust 腿缝采样）
- 工具：scenario_capture.lua:502-600（$C0D3 模式）、scenario_run.py:328-333（DSL）、
  analysis/traces/case-crawlprobe.run.json（endFrame 直证）
- 实测：analysis/traces/case-crawlprobe.frames.jsonl × shots_rust/case-crawlprobe.state.jsonl
  （本档 §1 读数表；复核脚本为一次性只读，未入库——结论可由两文件直接重导）
