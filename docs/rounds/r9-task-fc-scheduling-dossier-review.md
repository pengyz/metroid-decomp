# R9 对抗复核 verdict——r14 FC 调度域调研 dossier（六问攻击）

- 复核员：verifier-scroll（证伪立场，非实施方）
- 日期：2026-11-03
- **verdict：计划可用（带 3 处修订：双调修范围扩全 caller 审计/A=4 拟合须单测化/f1266 共因排除）**

## 1. 走表双调实锤复核（最重） —— **成立（全例程通读复验）**

- **asm 全例程通读**（r9 截读教训标准）：
  - SetSamusData（$CD6D，prg7_engine.asm:2971-2989）：首指令 `jsr UpdateObjAnim
    ($DC8F)`=**走表**；尾 `jmp ObjDrawFrame ($DE4A)`——DE4A（:6228）起于
    `ldx PageIndex` 纯绘制、**无走表**——CD6D 全程恰一次走表 ✓。
  - AnimDrawObject（$DE47，:6225-6226）：`jsr UpdateObjAnim` 后落入 ObjDrawFrame
    ——经 $DE47 的调用方（主环 :2236 `jmp AnimDrawObject`）恰一次走表+绘制 ✓。
- **rust 双调代码实读**：`set_samus_data`（physics.rs:817-860）头部
  `update_obj_anim(mem, regs.a)`（:826，DC8F 忠实）+尾部
  `anim_draw_object(mem, 0, regs.a)`（:859，**注释误标 ObjDrawFrame/DE4A，实调
  DE47 形**——sprite.rs:37 首行 `update_obj_anim` 再走表）⇒ **同帧双走表实锤**
  （注释标错=对 DE47/DE4A 分界的误解，非端口笔误）。
- **A=4 拟合算术复算**：DC8F 形（delay==0⇒推进+reload A；≠0⇒dec；dec→0⇒推进+
  reload）：asm 单走 A=4 ⇒ 推进周期 4 帧；rust 双走（每帧两次调用均带 A=4）⇒
  f1: adv,set4→f2: dec,dec→2→f3: dec,dec→0 推进+set4 ⇒ **3 帧周期（推进/跳替
  3,1,3,1 别名）**——机制算术成立，拟合观察形可信。
- **修订①（范围）**：双调非孤点——**room.rs:248 注释自认另一处**
  「update_obj_anim(6)+anim_draw_object（goal-r16 修）」=已知第二双调；
  mainloop.rs:2249 注释亦载 bullet 面历史双调已删。处方：修径范围扩为
  **anim_draw_object 全 caller 走表审计**（grep 全部调用点逐一判 asm 对应物
  是 DE47（应走）还是 DE4A（不应走）），否则修一处漏一处。

## 2. FC 无罪结论 —— PASS（附指针补全）

- 1383 帧 watch 基线实锚：walk-fc-capture-seam.md §3 表——FC/ScrollX/ScrollY/
  ScrollDir/SamusX 五通道 **1383 帧 0 差**（B2 前提作废实证）✓。
- **归因完备性**：f1347 FC 次级残余的因果方向——physics 分歧（f1266）**时间上
  先于** FC 分歧（f1347，晚 80 帧）⇒ FC→physics 因果倒置不可能（时间序排除）
  ✓；下游机制（位置差→分支差→重活差→FC）自洽。
- **注记（指针缺失）**：943/445 两基线的场景归属未在调研文本给出指针——归档
  时补（防「三基线」引用悬空）。

## 3. f1266=physics 主因 —— PASS（附共因排除要求）

- 因果序复算：SpeedY f1264→SamusY f1266→X 漂移→FC f1347→换房 f1524——时间
  单调序与「FC 下游」一致；且三条 0 差基线证明 FC 机制本身无罪。
- **不可排除项（修订②）**：**共因假说未排除**——同一上游根因（如调度 span
  计价/采样缝）可同时喂 physics 表与 FC 表（不同时刻显影）。处方：B2 watch
  帧窗（f1260-1270 physics 字段+f1340-1350 FC 字段）逐帧对读，验证两分歧
  之间是否存在传递链（X 漂移值与 FC 跳变帧的数值耦合）；无耦合链⇒共因坐实
  需另立根因卡。

## 4. goal 遗产判级抽查 —— PASS

- 内角判据链 asm 对读：crawler.asm:132-154 实录 `dey/tya/and #$03
  （=orient−1&3）/sta data0A/jmp CrawlerReorientSprite`+FlipDirection 分支——
  与调研引用一致 ✓。
- 「轮 158-181 有真证据/182-256 空转」分界：与 gate-debt 轮观察（485 commit
  拉取后债清理）及 r9 历史轮次记录一致——采信（未逐 commit 复核，抽样无反例）。

## 5. 作战计划 —— 可用（附修订③与两点核查）

- **阶段 1 验收硬度**：AD 序列双腿同值+943 基线保持+L4 ratchet——三点够硬，
  **须补两条**：①双相位回归锚保持（capture_seam 测+kick/l380 基线——新修不得
  再动采样缝，动则必走 for_rebase 重基流程）；②双调修以 **animIndex 帧序单测**
  验收（把「13 停 7 帧」疑点编码为确定性断言——修后必消；同时把 A=4 vs 双调
  的 delay 序列（4,3,2,1 vs 4,2,0 别名）编码为表驱动单测）。
- **阶段 2 依赖序**：F40D/DE05 阻塞撤回件回装按既定「回装证据包」流程（r9
  dump-ens-extra 轮在案）——一致 ✓。**SetPPUInc（修径 6b）提前性核查**：若其
  属渲染面（PPU 地址增量模式→BG 查点方向），阶段 1 的像素级验收点可能被其
  缺失阻塞——处方：阶段 1 开工前先对该修做一次「影响面标注」（哪些采集点
  依赖），依赖者提前或显式 TOLERATED，避免阶段 1 验收空转。
- **规模估计**：未在调研文本见到量化估计——按 bisect/回装/双调修三件估
  （双调修+caller 审计≈1 轮；B2 watch≈1 轮；阶段 2 回装≈2-3 轮）——建议补
  估计并与反空转硬约束联动。

## 裁定

| 问 | 结论 |
|---|---|
| 1 走表双调 | **实锤确认**（全例程通读+rust 实读+A=4 算术）——修订①扩全 caller 审计 |
| 2 FC 无罪 | 成立（1383 帧五通道 0 差+时间序排除倒置）；补 943/445 指针 |
| 3 f1266 physics | 因果序成立；**共因假说须 B2 watch 排除**（修订②） |
| 4 goal 遗产 | 抽查一致（crawler 内角链 asm 对读 ✓） |
| 5 作战计划 | 可用；修订③（animIndex 序列单测+双相位锚保持+SetPPUInc 影响面前置核查） |

**verdict = 计划可用（3 处修订后执行）**。

Evidence: prg7_engine.asm :2971-2989（CD6D 全文）/ :6225-6240（DE47/DE4A 分界）/
:2236（主环 jmp AnimDrawObject）；physics.rs :817-860/:826/:859；sprite.rs :37；
room.rs :248；mainloop.rs :2249-2264；enemies/crawler.asm :132-154；
walk-fc-capture-seam.md §3（1383 帧五通道 0 差表）；本复核 delay 序列算术推演。
