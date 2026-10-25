# findings：samus-move3 位移漂移根因定谳——迭代↔帧调度错位（原版掉帧窗未建模）

> 状态：**根因已定谳（A 级证据）**，2026-10-25。物理/状态机逻辑同迭代**逐迭代一致**
> （判别实证）；漂移=两侧「迭代↔墙钟帧」映射错位——金腿存在 **4 处无迭代窗**（原版
> bogged/掉帧窗——D4 保真项「掉帧」），我方端口逐帧恒一迭代未建模 span。

## 0. 前置更正（旧构建误诊——原地记录）

- 本件初版基于 19:32:58 生成的 `analysis/dump/samus-move3.frames.jsonl`——**旧构建产物**
  （`nmi.rs` r26 Joy1Change 边沿清修 mtime 19:36:41/commit 4139c13c 在其后）。旧 dump 的
  「$12 长按逐帧重触发」为 r26 前旧形 bug 本体（a82529e6「旧构建误诊」同款）。
- 21:45 当前构建重生成后：$12 边沿=1 帧 ✓ 与金腿同（f360 一帧 10、f361 清 00）。
  **r26 修已生效**；但位移漂移数字不变——驱动根因转向迭代调度（本件）。

## 1. 对齐口径定谳（双钟三口径实测）

| 口径 | FC($2D) 自校验 | 判定 |
|---|---|---|
| A. f 键直比 | **破裂**（f=8 起 FC 不等） | 伪偏移源（l3-b 已更正同款） |
| B. 行序+head_skip（manifest 旧口径） | **破裂**（首对 FC 01/00） | samus-move3 game-core 腿未声明（rows=null） |
| **C. FC($2D) 迭代序配对** | **全程逐对相等 ✓** | **真对齐单位=游戏迭代** |

- **asm 铁证**（prg7_engine.asm:199-223 MainLoop C0BC 体）：`jsr CheckBankSwitch`→
  `jsr UpdateTimer`→`jsr GoMainRoutine`→**`inc FrameCount`（无条件，每迭代一次）**→
  `sta NMIStatus=0`→spin（@loop_waitNMIEnd）→`jsr RandomNumbers`→`jmp MainLoop`。
- 金腿 FC≡行序（每 spin-exit 行恰一次 FC 递增）= **行=迭代**；臂内 spin/重活跨帧时
  迭代不完成→**无行帧**（missing f）。
- 双钟：trace f=inputPolled 计数（scenario_capture.lua:387 `frame=frame+1` @inputPolled
  回调）；行采样=$C0D3 spin 出口（:553-563，once-per-frame 守卫）。probe f=st.frameCount
  （+12 偏移）——本件全用 trace 钟（双腿同源：输入沿 f360/420/540/850/1180 两侧同帧 ✓）。

## 2. 漂移=纯调度（决定性判别——analysis/driftjudge2.py）

**同迭代序号配对**（金迭代 k=行 k；我方迭代 k=第 k 次 FC 递增块）：

- it=0..532：x/status/y/sub/acc/speed **全一致**（物理逻辑零分歧）；
- it=533：x/status/y 瞬态相位差（就位步早 1 迭代——配对惯例面）；
- it=972/973/978：acc/sub/speedX 瞬态相位差（起跑步）；
- **调度表**（迭代→帧偏移）：it≤350: +1 → it400-500: +3 → it550-950: **−4** → it1000+: **−6**
  ——**阶梯恰=金腿 4 处无迭代窗**。

## 3. 金腿无迭代窗全集（A 级——行集/FC/重触发计数三源互证）

| 窗 | 金 missing f | 帧数 | 我方对应 | 窗内迭代数（FCΔ） | ReadJoyPads 次数（$19 decΔ） | 性质 |
|---|---|---|---|---|---|---|
| 1 | 2,4,5,6 | 4 | f2,f4,f6-8（5 行冻结） | 0 | 4 | boot 标题臂 spin（位置差 +1 净） |
| 2 | 361-363 | 3 | f363-367（5 行冻结） | 1（0x63→0x64） | 4（$19 02→06 含重触发装载） | 标题臂 spin（我方长 2 帧+迟 2 帧） |
| 3 | **542-548** | **7** | **无！** | 1（0x15→0x16） | **4**（$19 05→01；$18 1F→1D=1 变更装载+3 dec） | **MoreInit 重活窗（游戏入口链）** |
| 4 | 1002-1003 | 2 | **无！** | 1（0xDA→0xDB） | **1**（$19 05→04；$18 01→08=归零重触发） | 游戏态重活窗（敌生成/门类） |

- 窗3 窗内迭代做了什么（f541→f549 状态差 178 处，stallwin.py）：$24 02→00（bank 切换
  消费）/$28 00→01/$1D GameMode 01→00/$1E MainRoutine 00→**02**（AreaInit+MoreInit 同迭代
  完成——mr=01 不出现于行=工作内瞬态）/$29 TimerDelay −1/FC +1/$2E,$2F +$19,+$5F 各一次
  （RandomNumbers 一次）/$33-$50 区域指针拷贝/$04xx 房间缓冲/$07A0+ VRAM 队列填充/
  $200 全清 F0——**一次游戏入口重活迭代（房间解压类）**。
- 窗4 窗内（f1001→f1004）：FC +1/$29 −1/$2E,$2F +once/$030E x +2px（正常移动步）/
  $04xx 敌槽装载（0405 40→00、0415 00→40、042x 置数）——**敌生成装载重活**。

## 4. NMI 体跳过语义（asm 铁证——窗内 ReadJoyPads 计数的解释）

prg7_engine.asm:233-292 NMI：OAM DMA 恒跑→**`lda NMIStatus/bne @endIf_A`** 内块
（NMIScreenWrite/CheckPaletteWrite/CheckVRAMStructBufferWrite/WritePPUCtrl/WriteScroll/
**ReadJoyPads**）→块外恒跑 SoundEngine/UpdateAge→尾 `NMIStatus=1`。

- **bogged 帧**（迭代工作进行中、NMIStatus 未清）→ 内块跳过：无 ReadJoyPads（$18/$19 不减）；
- **臂 wait 帧**（臂自清 NMIStatus 后 spin）→ 内块全跑：ReadJoyPads 每帧（$18/$19 减）。
- 窗界 $18/$19 值只取决于**总调用次数**（内部拆分不影响行级状态）——端口 span 只需
  建模 (span 帧数, 内块跑帧数) 二元组 ✓。

## 5. 根因三分解（全部 A 级 asm 铁证——2026-10-25 补锤）

### 类 R1：迭代内链断裂（同一 GoMainRoutine 调用内的 fallthrough/tail-jump 被拆成多迭代）

| 链 | asm 铁证 | 金行为 | 我方错形 |
|---|---|---|---|
| AreaInit→MoreInit | prg7_engine.asm:1826-1843 **AreaInit 无 RTS fallthrough 进 MoreInit**（:1839 `jsr IsEngineRunning` 后直落 C81D） | 一迭代 mr 00→02（窗3 状态差实证） | 拆两迭代（mr=01 现于行） |
| 标题头→1B 体 | prg0_title.asm:27-55 MainTitleRoutine 头：`sta TitleRoutine=0x1B` 后 **`bne L8027` 落进同一 JumpEngine 分发** | 一迭代（头+1B 体，TR 04→16） | 头设 TR=1B 即返，下迭代才跑体（TR=1B 现于行 f361-365） |

### 类 R2：迭代内帧等待（WaitNMIPass 族）未建模/错价

- **等待语义**（asm :1108-1140）：`WaitNMIPass`/`WaitNMIPass_`=`ClearNMIStat`（NMIStatus←0）
  +spin 等下一 NMI=**1 帧等待+NMI 内块全跑（含 ReadJoyPads）**；`WriteAndWait`（ScreenOff/
  ScreenOn/ScreenNmiOff 共享）fallthrough 同款。
- **窗内 ReadJoyPads 计数**（$18/$19 RetrigDelay 递减数）三源互证：窗2=3 等待+wake=4；
  窗3=3 等待（ScreenNmiOff@InitBank1:1402 + ScreenOff@MoreInit:1871 + ScreenOn@MoreInit
  尾:1930 `jmp ScreenOn`）+wake=4；窗1 iterA=1 等待+wake、iterB=1 等待+2 bogged+wake。
- 我方 title_spin 把 (工作+等待) 计成 2 tick/段（工作 tick+spin tick），金腿=同帧共享
  （等待占帧、工作在帧内）——1B 体我方 6 tick vs 金 3 帧。

### 类 R3：重活 bogged 帧（原版掉帧=D4 保真项）未建模

- 迭代工作进行中 NMIStatus=1→NMI 内块**跳过**（asm :251 `lda NMIStatus/bne @endIf_A`
  ——ReadJoyPads/palette/VRAM/scroll 全跳；SoundEngine/UpdateAge 恒跑 :270-277）；
  重活（InitBrinstarGFX/SetupRoom 解压/名表拷贝/敌生成）占多帧=掉帧窗。
- 窗3=3 等待+**4 bogged**；窗4（f1002-1003 敌槽 2 生成（$04xx 实证：Ens.2 置位））=**2 bogged**+wake。

### 金腿窗全表（跨场景自证一致——game-start/samus-move3 同构 ✓）

| 窗 | 触发 | 帧形 | 构成 |
|---|---|---|---|
| 1a | boot 臂 0x00（InitializeAfterReset） | 1 帧 | 1W+wake |
| 1b | boot 臂 0x01（DrawIntroBackground） | 3 帧 | 1W+2B+wake |
| 2 | 标题头→1B 体（START@intro） | press+1 起 3 帧 | 3W+wake |
| 3 | 游戏入场链（InitBank1+AreaInit+MoreInit） | press+2 起 7 帧 | 3W+4B+wake |
| 4 | 敌生成（Ens 槽装载） | 2 帧 | 2B+wake |

我方错形对照：1a=2 帧/1b=2+4 帧错位（TR=01 多步拆分）/2=7 帧 2 迭代（头断裂+2×计价）/
3=无（拆 2 迭代+无 span）/4=无。

## 6. 修复施工令（TDD 增量——下一 round 执行）

1. **F1 链合一**：`game_main_routine` dispatch 0=area_init 后 fallthrough more_init
   （一迭代，mr 00→02）；`title_tick_head` 命中时同迭代续跑 0x1B 体（不落 TR 下迭代）。
   TDD：单测断言「一 main_loop_iter 内 mr 00→02 / TR 04→16」。
2. **F2 引擎 gate 队列**：`FrameMemory.iter_gates: VecDeque<bool>`（true=NMI 内块跑（等待
   帧）/false=跳过（bogged 帧））；span 内不跑 MainLoop 四件/臂/采样缝；FC inc 延至 span
   尾（UpdateAge FC==0 相位保真）；NMIStatus 门=队列项。语义锚=asm :199-223/:251。
3. **F3 等待/重活声明**（组装式——各 asm 站点自带）：ScreenNmiOff/ScreenOff/ScreenOn/
   WaitNMIPass 端口→enqueue Wait；InitBrinstarGFX/SetupRoom 环/名表拷贝/敌生成端口→
   enqueue Bogged(n)。窗3 组装=W,B,B,W,B,B,W；窗4=B,B。
4. **F4 标题体计价重排**：1B 体（及 15/1C/17 同族）=一迭代+3W gate（非 6 tick）；
   boot 臂 0x00=[W]、0x01=[W,B,B]。按金腿窗全表对齐。
5. **验收**：samus-move3 4 金面（f800/1000/1150/1330）px 收零 + L3 逐帧 watch 零
   （FC 迭代序口径+行集声明）+ game-start 5 面/boot-title 7 面保持零 + 全测 + gates 不恶化。

## 7.5 视觉摄像机包（2026-10-25 渲染目检定谳——用户反馈「摄像机不跟随/移动帧不对」）

**已修**（commit 7ccefb0e）：asm E31A 移动链全端口（逐像素 MoveSamus±/ScrollRight·Left/
SamusScrX/CheckStopHorzMvmt）+勘误群（ScrollDir=$49/ScrollX,Y=$FD,$FC/SamusInLava=$64/
Samus.hi=$30C）——**状态级摄像机已跟随**（samus-move3：$FD 00→0x75 金 00→0xDD 同构）。

**余根（渲染腿包——视觉摄像机收口）**：

| # | 缺口 | 证据 | 施工 |
|---|---|---|---|
| V1 | render_frame_bg **只绘 NT0 且不应用 scroll**（render/mod.rs:87-137；write_io $2005 未捕获——memory.rs write_io match 缺臂） | 渲染目检：ScrollX 已滚而画面静止 | RenderState 捕获 PPUSCROLL（首写 X/次写 Y latch）+BG 查点加 (scrollX,scrollY)+名表选位（PPUCTRL bit0/1）+镜像映射（横镜=$2000/$2C00 双屏） |
| V2 | 跨房列流送 **UpdateNameTable 未端口**（E70C→流送 payload——现空壳） | 金腿第二屏内容=滚动流送（EndOfRoomHorizontal 每 8px 一列） | 房间双缓冲（RoomRAMA $6000/RoomRAMB $6400——「load room before PPU」）+列流送+属性流送 |
| V3 | HUD「EN」能量条缺失（uw_display_bar 空壳臂） | 渲染目检：金腿左上「EN 30/22」全程在 | DisplayBar 端口（名表写面） |
| V4 | 敌显链缺（DrawEnemy $DD8B+AreaPointers_RAM） | 金腿 2 敌机在屏/我方无 | 敌显工作单（enemy-display 既有工单） |

**移动帧/姿态面**：MoveSamus 逐像素环+SetSamusStand 停速已端口；姿态动画差=SetSamusData
链深件（UpdateObjAnim）随 V 包收口对拍再判。

## 8. 留档

- 工具：`analysis/firstdiv.py`（三口径首分歧）、`analysis/fcalign.py`（FC 行序对照）、
  `analysis/eventfc.py`（里程碑迭代对表）、`analysis/driftjudge2.py`（同迭代判别）、
  `analysis/edgefc.py`/`joyraw.py`（输入面）、`analysis/stallwin.py`（停滞窗状态差）、
  `analysis/win3seq.py`/`bootseq.py`（窗内步序细排）。
- 三源互证：行集（missing f）×FC 递增（每窗恰一次）×$18/$19 重触发计数（窗内
  ReadJoyPads 次数）。
- 旧构建误诊面收口：**生成 dump/截图前必须 `cargo build` 当前树**——取证工具链纪律
  增补随修复 commit 落 knowledge。
