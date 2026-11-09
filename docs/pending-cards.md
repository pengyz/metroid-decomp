# 挂起卡台账（docs/pending-cards.md）

> 用途：登记**移植侧分歧 / 待深件**的挂起卡（挂起=当前工具面/证据面已穷尽，待工具战役或
> 深件排期后重启）。**非原版 bug**——原版 quirks 归 [docs/bug-ledger.md](bug-ledger.md)（D4）。
> 格式：现象 / 复现 / 已穷尽面 / 挂起原因 / 重启条件。
> 本文件由 2b914653（decomp-analyst 任务卡）于 2026-11-06 创建（敌接触触发差挂起落账——
> lead 裁决 2+3 组合件 1）。

## PENDING-001 敌接触触发差（killenemy rust f986 误触发 / emu 未触发）

- **现象**：`case-killenemy-probe` 场景 f986 起血量分歧——**rust 端 (Health,Health+1)
  (0,3)→(32,2)（显示 30→22，扣 8 伤害）**，**emu 端恒 (0,3) 不受伤**。f982-985 两腿
  同 (0,3)；f982-991 两腿 Samus 位置 (y=176/x=128/status=255) 与半径 (sRY=15/sRX=4/
  eRY=8/eRX=8) 全同 ⇒ 非场景位置/半径径差，为**触发径条件求值差**。
- **复现**：
  1. `python3 tools/scenario_run.py run cases/case-killenemy-probe.txt --timeout 240`
     （emu 腿；或 L4 矩阵 `python3 tools/l4_diff_run.py run case-killenemy-probe`）；
  2. rust 腿同场景跑（`--rust` 或 L4 矩阵 rust 侧）；
  3. 对表 f982-991 窗血量字节（watch $0106/$0107）：rust f986+ (32,2) vs emu (0,3)。
- **已穷尽面**（findings §49-62 全档案，8 面+动态 trace 工具面）：
  ① 场景位置差（两腿同）✗ ② 触发条件公式/半径值源（rust +4 固定 vs asm 半径和——
  f986 dy=22 < sry+ery=23 公式下**两腿皆应触发**，emu 不触发=emu 侧条件差）✗
  ③ i-frame/InvincibleDelay（两腿 0）✗ ④ hi-distance 位（两腿 0）✗ ⑤ 敌状态/分支
  （status=[0,0,0,0,2,2] 两腿同）✗ ⑥ 螺旋转攻击（两腿 0）✗ ⑦ 2466/2543 两触发点
  定谳（2466=Skree 弹碰撞（§60 更正：0x0440=敌 slot 非弹——弹径 A0-AF 两腿空，
  f986 无弹在场）；f986=敌碰撞径（2543 和公式或 2466 +4 公式——同值场景两径皆可能））✗
  ⑧ 动态 trace：F1FA 全槽位进入 ✓（迭代序/入径候选排）→ 内层 cmp 实录需深断点——
  Mesen Lua 工具面限（exec 枚举注册不可达+read 注册三证失败——findings §62 五证）✗。
- **挂起原因**：**Mesen Lua 工具面限**（exec 不可达+read val 注册三证失败）→ 待
  「Mesen 工具战役」（C++ trace logger CPU 全序 trace / Mesen 回调 API 修）排期。
- **重启条件**：Mesen 工具战役完成后重启 F1FA 内层 cmp/Temp zp read val 实录
  （Temp00=$00/Temp01=$01/Temp03=$03/Temp04=$04/Temp05=$05/Temp06=$06/Temp07=$07/
  Temp09=$09/Temp10=$10/Temp11=$11——constants_ram:252-267）→ 定谳 emu 不触发径
  （cmp 求值序/scroll-dir 分支/position-hi 补偿）→ 修移植或登记已知偏差。
