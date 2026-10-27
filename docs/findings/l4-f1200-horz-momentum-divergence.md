# L4 f1200 水平动量发散考证（相机滚屏缺口 15873px）——asm 全链+rust 态实证

> 2026-10-27 · 件=功能3 相机滚屏+水平动量发散 · 证伪立场（独立重导）·
> Evidence A 级（bank:addr+state dump）Confidence 90

## 0. 判定汇总（已证实 / 假设分区）

### 已证实（A 级）

1. **CF88 无键清速语义**（`SamusRun_CheckHorzMovementMidair: ; 07:CF88`（prg7_engine.asm:3391-3428））：
   无键径 Lx015——`lda SamusAccelY / bmi @RTS`（上行=保）/`beq @RTS`（地面=保）/
   `cmp #ObjAnim_SamusJump / bne @RTS`（**非 Jump 动画=保**）→否则 fallthrough
   `StopHorzMovement`（`jsr ClearHorzMvmntData / sty SamusAccelX`（07:CFB7））=**清**。
   ⇒ 清速三条件合取：**下落（AccelY>0）+ 动画==SamusJump + 无键**。
2. **地面径即清**（`SetSamusStand: ; 07:CF55`→`SetSamusStand_NoFootstep: ; 07:CF5D`→
   `jsr NoHorzMoveNoDelay`（07:CF81）→`StopHorzMovement`（asm:3358-3389））——
   地面无键=即清（**无摩擦滑行径**（考证更正（lead 疑「摩擦滑行」——证伪））。
3. **跳落地径即清**（`SamusJump: ; 07:D002` Lx021（07:D03C）：`lda SamusAccelY / bne Lx023`
   （空中跳过）/落地⇒`jsr SetSamusStand`（asm:3497-3517））——落地即清。
4. **旋/解旋轴**（`SamusRun: ; 07:CCC2` 空中径（asm:2858-2895））：**下落**
   （speedY bpl）：`cmp #$18 / bcs samL04`（disp≥24⇒**解旋**⇒Jump 动画）；
   **上行**：disp≥24⇒Salto（跳火除外）+disp≥32⇒PntUp/松键停升。
5. **rust 态实证**（rust_screenshot --state-out（case-001 sched（1192 帧 dump）））：
   f920 起跳（Run+RIGHT）→f925 Jump(disp 0x13)→f930-940 **Salto**（disp 0x20 峰）
   →f944 解旋（disp 0x15<0x18）→f948-951 Jump(disp 0x0E→0x08)→
   **f952 无键+下落（accY=0x18）+anim==Jump(0x0C)⇒CF88 清**（accX 0x30→0/spdX 0x01→0）
   →f955 落地（accY→0）→f957 Stand。**水平态 f950 与 lead 报 emu 值全同**
   （accX=0x30/spdX=0x01/max=0x12）——发散在 f951 之后。
6. **输入序**（sched 实文）：f900-949 RIGHT 持续（f920-21 A+RIGHT 起跳（跑跳）/f940-41 B+RIGHT）
   →f950 末键 RIGHT→f951-1200 无输入。

### 假设（B 级（待 emu 侧垂直态/动画面数据定谳））

- **H1（主假设）**：emu 该窗保持动量 ⇒ emu 无键时 anim≠SamusJump（=Salto 未解旋
  （emu disp≥0x18 更久——**垂直轨迹残差**（apex 高/下落相位差几帧））⇒ CF88 bne 保。
  rust f944 解旋（disp 0x15）早于 emu ⇒ f952 清速（emu 无此清）。**待证**：emu f944-952
  disp/animResetIndex 逐帧值（Mesen iram watch（case-001 sched 复采））。
- **H2（备择）**：lead 报「滑行 250 帧 45px（减速形）」若属实=地面摩擦径（本考证
  证伪 asm 有摩擦径（三径皆清））——若 H1 否定，须复核 lead 观察面（worldX 轴向/帧窗）。
- **H3（弱）**：ObjAnim_SamusJump 常量表偏移差（rust 0x0C vs asm 表相对值）——
  可被 H1 数据直接判别（emu animReset 值域）。

## 1. 根因判定（收窄）

rust f952 **非过早落地**（f955 才落地（lead 疑「f952 落地」——证伪））；
f952 清速=CF88 语义内正确行为（Jump+下落+无键）。**真正发散点=旋/解旋时机动画轴**
（垂直轨迹残差（disp 0x18 阈值两侧相位））——emu 保持 Salto ⇒ 动量保 ⇒ 滑行+相机滚屏；
rust 解旋成 Jump ⇒ 清速 ⇒ 停。

## 2. 修复路径（待 H1 数据定谳后）

- 若 H1 证实：对齐垂直轨迹（apex/下落积分（clamp $0300 面（5/-6）/重力 $18 起跳常数
  （Lx017-Lx018（asm:3449-3465）））使 disp 时序=emu（解旋帧对齐）。
- L4 验证：`python3 tools/l4_diff_run.py run-all`（f1200 期望大降（f820/f950 不回退））。

## 3. refs 锚（bank:addr（A 级原文））

SamusRun_CheckHorzMovementMidair 07:CF88（prg7_engine.asm:3391）/StopHorzMovement
07:CFB7（:3424）/NoHorzMoveNoDelay 07:CF81（:3386）/SetSamusStand 07:CF55（:3358）/
SamusJump 07:D002（:3467）/Lx021 落地检 07:D03C（:3497）/SamusRun 07:CCC2（:2858）/
旋解旋轴 samL01-samL04（:2858-2895）/Lx017-Lx018 起跳重力（:3449-3465）/
SamusJump_CheckHorzMovement 07:D055（:3519（stx Samus.speedX=向位（1/0/-1）面））/
ScrollRight 07:E6D2/ScrollLeft 07:E6A7（SamusScrX 过界触发（右 $8F/左 $71））。

Evidence: prg7_engine.asm 上锚行+rust state dump（/tmp/l4state/case.state.jsonl
（f920-960 逐帧 accY/accX/spdX/anim/jumpDisp））——A 级原文。
Confidence: 90
