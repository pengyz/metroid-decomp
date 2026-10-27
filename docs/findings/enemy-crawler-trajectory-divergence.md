# 敌 Crawler 轨迹首分歧 f799 考证（goal-r19）

## 状态：假设期（未定谳）——本文全部为「实测数据+待验证假设」，无结论

## 实测（emu watch 逐帧，case-001，2026-10-27）

敌 slot4（右柱 Zoomer，type=5）轨迹 emu vs rust：

| 帧 | emu (y,x,orient) | rust (y,x,orient) |
|---|---|---|
| f798 | (40,188,or0) | (40,188,or0) |
| **f799** | **(40,189,or0)** | (40,188,or0) ← **首分歧：emu 动 1px，rust 未动** |
| f819 | (44,200,or1) 拐角下行✓ | (41,190,or3) 误入上向 |

级联：rust 1px 落后 → 提前撞柱角 → InsideCorner(碰撞)orient 0→3（上）——而 emu 是
OutsideCorner 路径 0→1（下）正确拐弯。f950/f1200/f820 全部由此轨迹差主导。

## CrawlerAI($9A4C) 逐指令反汇编（ROM 直读——该段 m1disasm 未标注，grep 零命中）

```
$9A4C jsr $8009   ; ShouldCrawlerMove(F852)
$9A4F and #$03
$9A51 beq $9A87   ; gate==0 ⇒ 跳到 anim+common（跳过移动）✓ rust 门语义对齐
$9A53 lda $81     ; EnemyStatusPreAI
$9A55 cmp #$01 / beq $9B44   ; Resting 早退
$9A59 cmp #$03 / beq $9B49
$9A5D lda $6AF4,x / cmp #$03 / beq $9A87
$9A64 lda $040A,x / and #3 / cmp #1 / bne $9A7E   ; orient==1?
$9A6D ldy $0400,x / cpy #$E4 / bne $9A7E          ; y==$E4?（熔岩线）
$9A74 jsr $9ABD / lda #3 / sta $040A,x / bne $9A84 ; lava turnaround
$9A7E jsr $9AE2   ; MOVE（经 CrawlerMovementRoutinesTable $8048）
$9A81 jsr $9AA8   ; InsideCornerCheck（move 失败才动作：碰撞⇒orient−1；离屏⇒flip）
$9A84 jsr $9AC6   ; OutsideCornerCheck（恒跑：探 (orient+1) 成功⇒orient+1，探移提交）
$9A87 lda #3 / jsr $800C(UpdateEnemyAnim) / jmp $8006(common)
```

## 待验证假设（下轮主攻）

**H-A（主嫌）**：emu 在 gate=0 帧（采样 FC=$10）x 动了 1px ⇒ 存在未建模移动源。
候选：① 采样 FC 与逻辑帧 FC 差一（watch 采样在 spin-exit，FC 递减时点未核）
② OutsideCorner 探移在 gate≠0 帧的提交语义（探移 1px 即数据面移动）
③ ShouldCrawlerMove 的 X 输入（PageIndex=0x40？或 PageIndex 此帧≠slot4）

**H-B**：1px 漂移独立于 gate——rust enemy_move_one_pixel_* 与 asm $83F5 族在某边界差
1px（如 radius/块界判）。

**验证法**：emu 全帧 x 增量时刻表 + 对应帧逻辑 FC + gate 值三列表格化（现 trace 已有
frameCount/en4x，analysis/traces/case-001-*.frames.jsonl），找出 x++ 帧的 gate 分布——
若 gate≠0 帧也 x++ ⇒ H-A②；若全在 gate==0 ⇒ H-A①采样时差。

## 已确认对齐面（勿重复排查）
- gate==0 跳过移动语义：rust `crawler_ai` `if gate==0 {common;return}` ✓ 与 $9A51 对齐
- UpdateEnemyAnim(E094) 走表 ✓（r31 已端口，cadence=3）
- CrawlerReorientSprite($9A8F+$9AA0 表) ✓（r31 已端口）
- InsideCorner/OutsideCorner 结构 ✓（$9AA8/$9AC6 端口，探移提交语义已核）
- 敌渲染管线（DrawEnemy/EnFramePtrTable/enemy_gfx 烘焙）✓（帧 $58 tiles CC/CD/DC/DD 实测）


## 数据质量边界（2026-10-27 补）

f431-446 段 $0441 是**每帧 -1 的递减计数**（29→14）——敌未生成/未更新期的脏值（intro 期
GameEngine 未跑），**不是位置**。可信窗口：敌实际移动段（f795-830 实测 x=188→200 稳定
单调）。下轮重分析须先定位敌 spawn/首更新帧（$0441 值稳定的起点），只取该窗口做
x++/gate 分布表。


## 定谳推进（2026-10-27 r20）：FC 相位差 6——A 级实测

**emu FrameCount 只在 MainLoop(C0BC) 尾递增**（asm prg7:207 `jsr GoMainRoutine / inc FrameCount /
lda #0 sta NMIStatus / 等待 NMI 结束`）。emu f795 FC=$0C=12 ⇒ emu MainLoop 自 **f783** 起才
跑（12 次迭代）。**rust $2D 自 f2 起每帧递增**（f795=$12=18）⇒ rust 主循环相位早 6 迭代。

**后果**：ShouldCrawlerMove(F852) 门=((slot/8+FC)>>1)&3——FC 错相 6 ⇒ 6/8 移动窗错相 ⇒
f799 起 1px 漂移 ⇒ 柱角碰撞/拐角序列级联（or3 vs or1）。

**待修（下轮）**：rust 主循环启动相位（BootPhase::MainLoop 进入点/首迭代语义——r14 唤醒
件工作区）与 emu 对齐；或验证 6 差的构成（title 段 rust 多跑迭代？）后精确修。修后敌轨迹
应全帧对齐 → f820/f950/f1200 连锁收敛。


## 停帧全表定谳（2026-10-27 r21）：emu 18 停帧 vs rust 0 停帧——A 级实测

逐帧 FC 序列（case-001 watch $2D，两侧全程对比）：

| emu 停帧段 | 帧数 | 对应事件（asm 推断） |
|---|---|---|
| f2 | 1 | boot 链 |
| f4-6 | 3 | boot 链续（PostSpin/AwaitNmi 窗） |
| f311-313 | 3 | 标题 Start→菜单过渡（press 310） |
| f352-358 | 7 | **游戏启动+房间加载**（press 350→LoadArea/DrawRoom VRAM 重活） |
| f916-917 | 2 | 游戏中首次横滚（ScrollRight VRAM） |
| f1064-1065 | 2 | 左向横滚（ScrollLeft VRAM） |

**rust：0 停帧**（$2D 每帧 +1）。f799 时 emu 已停 14 帧 ⇒ emu FC=16、rust=22（差 6 ✓ 实测吻合）。

**机理（asm）**：MainLoop(C0BC) 每迭代末等 NMI 握手（C0CB `lda NMIStatus/beq wait`）；
长 PPU 重活（房间加载/滚屏 VRAM）期间迭代跨帧 ⇒ FC 不递增 =「停帧」。rust 的
iter_gates 门队列机制即为此建模（`1 等待帧→0 全跑/0 bogged→1 跳过`）但**未在这些
事件点生成门项**。

**修法（下轮，r14 机制域）**：① 游戏启动/房间加载径（GoMainRoutine 初次 LoadArea/
DrawRoom）补 3+7 帧 bogged 门；② ScrollRight/ScrollLeft VRAM 径各补 2 帧；③ boot 链
1+3 帧。全部对齐后 FC 全帧一致 ⇒ 爬行门相位对齐 ⇒ 敌轨迹对齐 ⇒ f820/f950/f1200 收敛。


## 补偿机制发现（2026-10-27 r23）：局部停帧插入门会打破全局 blink 相位对齐

实验：按 asm $90BA/ClearAll 帧结构（ScreenOff 等+清表 bogged+WaitNMIPass 等=[1,0,1]）在
rust 标题 Start→菜单过渡插入门 → **L4 f550/650/720 回退 0→282px**（blink 相位错 3 帧）。

**判明**：rust 旧时间线（0 停帧）在 f550 的 blink 相位与 emu（14 停帧）**一致**——说明
rust 的 blink/Timer3 相位经**其他补偿机制**（intro Timer3 窗长/相位机）与 emu 对齐，
并非经 FC。局部插门（不动补偿机制）⇒ 相位错位 ⇒ 回退。

**结论**：停帧对齐**不可局部做**——必须整体调度审计（r14 域）：枚举全部帧相位敏感面
（blink/Timer3/爬行门/滚动门/动画 cadence），一次性重排。滚动门（2bd601dc）保留：
实测 L4 无回退（滚动点在敌轨迹已分歧区，且 asm 忠实）。

**已回退**：标题门（未提交即弃）。前置段对齐降级为「整体调度审计」任务卡，挂 D7 缺口
（时序边界类）。


## 全帧探针定谳（2026-10-27 r24）：快照捷径吸收房载停帧——余差 6=boot4+title3−起点1

rust_screenshot 新增 DSH_FC_TRACE 逐帧探针（env 门控，DSH_SPRITE_TRACE 同式）后全帧
实测：**rust 0 停帧**（此前"0 停帧"结论系稀疏采集点误测——本轮更正）。序列唯一缺口
=f352-358：**rust 场景在 f352 快照恢复、整段跳过房间加载**——恰好吸收 emu 的 7 帧房载
停帧（等价抵消）。

**FC 对账（f799）**：emu=16（798 tick−14 停）、rust=22（797 tick−7 跳帧）⇒ 差 6
= boot（f2+f4-6=4）+ title（f311-313=3）共 7 − rust 起点超前 1（emu f1 首帧 vs rust f2）。

**parity 约束**（r23 发现）：blink=2 帧周期 ⇒ 累计插入必须偶数。boot(4)偶 ✓、
title(3)奇 ✗、boot+title=7 奇 ✗、boot+title−1 起点修正=6 偶 ✓——**须与起点语义一并
处理**（起点对齐=boot 首帧语义，非独立旋钮）。

**下轮路径**：boot 链 asm 审计（emu f2/f4-6 停帧的 asm 来源——boot.rs 门结构 vs
InitializeAfterReset 链 NMI 等待）+ 标题门（已实现过 [1,0,1]，asm 依据充分，$90BA/
$909F）——两者合计与起点修正对齐后 FC 全帧一致。blink parity 每步用 f550=0px 验证。


## boot 全貌补定（2026-10-27 r24 续）：rust 场景跳过整个 boot 链

`init_bank1`（boot.rs，窗3 门 1+2+1 在此）存在但 case-001 **没跑**——rust 场景 f352
快照恢复 = 跳过 boot 链+房间加载整段（f2-f352 的 boot/title 由 rust 自身状态机以
每帧 1 迭代推进，无任何停帧）。故 rust 停帧 0 是结构性的，非门缺失。

**FC 全帧对齐的条件**（asm boot 时间表考证后）：
1. rust boot 相位须占 f1-f6 窗（6 tick，内含 f2/f4-6 四个门）——当前 f2 即完成；
2. 标题过渡门 [1,0,1]（已实现过，asm $90BA/$909F 依据充分，f311-313）；
3. f352-358 快照恢复点的 7 帧——快照恢复语义本身等价吸收 ✓（无需动作）；
4. 起点：emu f1 首迭代 vs rust f2——boot 重排后自然对齐。

此为 **boot 调度重排**（r14 域），需 InitializeAfterReset→DrawIntroBackground→…全链
NMI 等待点考证（prg0:8071 起）。非局部旋钮，专属任务卡。


## 相位归因定谳（2026-10-27 r25）：emu TitleRoutine($1F) 逐帧实测

- **f3-5 = phase 1（DrawIntroBackground $80D0）**：单迭代跨 4 帧（`jsr ScreenOff` 等 1 +
  `ClearNameTables` 4KB 重活 + `PreparePPUProcess` 背景 VRAM 写 bogged 2）⇒ FC 停 f4-6。
  rust 现仅 title_spin=1（差 3 帧）。
- **f311-312 = phase 0x1B（StartContinueScreen1B $90BA→ClearAll $909F）**：单迭代跨 3 帧
  （ScreenOff 等+清表 bogged+WaitNMIPass 等=[1,0,1]）⇒ FC 停 f311-313。rust 无门。
- f6-7=phase 2（METROIDFadeIn）、f8+=phase 3（FlashEffect，Timer3=80 帧长相）。

**完整修复方案**（parity 闭合）：① phase1 spin 1→3（或等价门 [1,0,0]+work）；② 标题
0x1B 门 [1,0,1]（重施，asm $909F 依据）。合计 +6（偶）⇒ blink(FC&1) parity 保持 ⇒
f550 不破；FC(f799)=22−6=16=emu ✓；FC(420) 校验：rust 405 mod 256=149=emu ✓ 全帧一致。

**实现要点**：phase1 的 spin 语义与门语义需先核 shim（title_spin 由 dispatch_step 的
NmiYield/shell 语义产生——改值须与 r14 契约对齐，禁硬旋钮）。验证序：单测（相位门形）
→ f420/550/650/720=0px（parity 不破）→ f799 FC 探针=16 → 敌轨迹对拍 → f820/f950/f1200。


## 最终实验定谳（2026-10-27 r26）：intro 循环互锁——局部插门不可收敛（已回退）

实现 phase1 门 [1,0,0]+0x1B 门 [1,0,1]（共 +6 预期）→ 实测 FC(799)=10（-12 过冲）
+ L4 回退（match 14、probe2 f820 16808）。根因：**intro 相位 0-16 循环播放**——phase1
门在每循环重发（已限迭代首 tick），而 rust 的 intro 循环数 ≠ emu（循环数依赖累计时序、
时序依赖循环数——互锁）。门总量=循环数×每循环门数 ⇒ 局部不可收敛。

**结论（最终）**：FC 相位对齐 = **title/intro 调度整体重推导**——须先建 emu
TitleRoutine 全相位日历（watch $1F 全程 f1-f350，含循环边界），再重排 rust shim 时序。
这是 r14 域专属工程（估 1-2 个专程 session），已具备全部输入：停帧全表/相位图/parity
约束/FC 对账式。非局部旋钮可解——实验三次（r23 标题/r24 boot/r26 组合）均证。

**替代路径**：功能5 门系统/其余功能不依赖 FC 相位（门交互是输入驱动事件），可先行。
敌轨迹收敛挂调度重推导任务卡。

## 关联
- 前置：docs/findings/anim-state-machine-model.md（decomp eefc9c3d）
- 下游：f820(359)/f950(1270)/f1200(15929) 全由本轨迹差主导
