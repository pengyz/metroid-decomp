# R9 对抗复核——审计方案（复盘主张验证+完备性修复）攻击性审查

- 复核员：verifier-scroll（证伪立场，非实施方）
- 日期：2026-11-02
- **verdict：需修订后可用**（五主张 4.5/5 实证成立、骨架正确；但依赖序反转、
  B 可比性缺工具钉、D 判定闭环缺口、及遗漏面——按下修订意见执行后可用）

## 主张事实复核（先证后攻）

| 主张 | 复核结果 |
|---|---|
| ②ratchet 盲区 | **成立**——gates/cli.py `l4_ratchet_violations`（:1079-1108）实读：仅 match 集规则①+blocked 规则，`diff_pixels` 全文件无读取（grep 零命中）；states 无量值字段 |
| ①l380 回退 | **成立**——现 matrix l380=**12085**/61440（fresh 实读；626 为 kick-restore 轮值）；载体缺口实：多 cases 曾 untracked（git status 在案） |
| ③假对照 | **成立且加码**——本复核实测 scrollseam f470-700 双侧 **$FD=00、Samus $030E=80 恒、f470→f620 全 RAM 仅 42 字节变化**——相机根本没动 ⇒ 右滚 10 点 0px=空对空。**牵连更正：我此前批 3 复核「接缝负结果定谳成立」背书作废（原地更正于本档）** |
| ④归因过宽/⑤goal 循环 | 采信（与本复核历史观察一致：f1000 scroll 3B 不可信已登记；探针标签改名模式多轮可见） |

## 修订意见（每条：节/问题/证据/处方）

### R1（方案 A+依赖序）——**A→B 顺序反了：A 全量落地会合法化 12085 现状**
- 问题：A 的 diff 量值 ratchet「只降不升」，若在 B 定谳前首锚，l380=12085 即成
  锚定 diff 永久合法（方案自知但仅「未锚定升阻断」不足以防）。
- 证据：ratchet 规则实读（上表）+12085 现值。
- 处方：**A 拆 A-lite/A-full**。A-lite（先行）=仅「未锚定点必须锁定+blocked 规则」
  （锁定≠判值，防静默，并即时覆盖 morph-ball 21 格未锚定风险——lead 攻击面⑥）；
  B 实验一定谳回退真值后，A-full（量值锚）以**真值为首锚**，嫌疑点（l380 族）
  打 `suspect` 标记不进 ratchet 分母直至 bisect 收口。

### R2（方案 B 实验一）——worktree 复跑可比性缺工具钉
- 问题：0a99466f 时的 tools（l4_diff_run/scenario_capture/rust capture bin——
  dumpPpu/EnsExtra dump 均在此后多轮演进）与现版差异会污染读数；且 emu 重采
  有实测 run 间方差带（f1000 族 ±44px；killenemy hit-2/dead 点级翻转已实证）。
- 证据：dump 面三次演进（本复核历史轮实录）；46/74 vs 48/74 同树翻转。
- 处方：**B1 加 B0 前置**：①worktree 连 tools/ 整体检出目标 commit（双侧同
  当版工具）；②先在现 HEAD 连跑 2 次建方差带，**低于方差带的差异不作判据**；
  ③判据用**分歧形态签名**（审计员自给的形态学：大壁残缺/柱纹/2px 横偏——
  形态可迁移，绝对 px 不可）。

### R3（方案 E+我的历史误判）——加「激励面先行判据」（最重要遗漏）
- 问题：方案 E 只管采样密度，未管**对照有效性**——0px match 前必须证激励发生。
  shootprobe 假绿（v1）与 scrollseam 假对照（本档实锤）=同一教训已二犯，方案
  未升格为机械判据。
- 证据：scrollseam $FD=00/Samus 恒位/42 字节 RAM 冻结（本复核实测）；
  shootprobe 三弹槽双侧全零（r9 在案）。
- 处方：E 加规则——**任何 0px match 计入分母/账清前，须附激励 RAM 证据**
  （相机位移（$FD 变化）/弹槽非零/状态机迁移）；否则该点标 `inactive` 不计
  分母。并补一张卡：**修正 scrollseam 场景（真滚动）重立接缝负结果**——现
  「负结果定谳」无效。

### R4（方案 D）——「四面全同才许进时序桶」判定不闭环
- 问题：截屏相位类（精灵 mid-frame 撕裂）可能四面全同但仍非「内容差」也非
  可归因时序——且四面中 scroll/ctrl 3B 不可信（audit 自认）⇒ 判据含不可信面。
- 证据：r9 终结判轮 scroll 3B emu=000000 回退面在案；48/74↔46/74 点级翻转
  （四面全同的帧也会逐 run 翻）。
- 处方：D 加**双时刻采样判别**——同帧两次 dump（帧界+帧中）：两次全同⇒内容
  面确证（可进「已证实」）；两次不同⇒采样类实锤（直接进时序桶，无需四面）。
  四面只作内容排除用，scroll 3B 从判据中剔除直至字段名修复。

### R5（方案 F）——「探针-only 禁 feat」机械闸误伤面
- 问题：合法的「探针转正」commit（shootprobe v2 形）会被拦。
- 证据：r9 shootprobe v1→v2 返工正是探针修正转正路径。
- 处方：闸加白名单通道——commit body 含显式声明（如 `probe-fix:` 标记）放行，
  并要求附新旧对照（非空转证据）。

### R6（方案 A schema）——total_pixels 分母未钉
- 问题：白名单排除像素随 whitelist 演进变化 ⇒ 量值比较分母漂移。
- 处方：schema v2 存**排除后分母**（diff 计算时的 total 快照），比较同口径。

### R7（遗漏面）——探针证据载体在 analysis/（不入库）
- 问题：r9 多轮 verdict 的关键证据（ens_trace 7479 事件等）在 analysis/_gen/
  ——**不入库目录**，历史不可考。
- 处方：并入 G/F：探针脚本+trace 摘要（PC 分组表）入库（analysis/ 只留原始
  大件）。

### R8（遗漏面）——账迁移未提
- 问题：现账 count 字段=失败项数，A schema v2 是像素——旧 13+32+46 条账语义
  迁移方案缺失，硬切会丢历史容忍理由。
- 处方：A-full 加迁移节：旧账保持 reason/exit，追加 v2 字段（不做语义改写）。

### R9（依赖序重排，综合 R1/R3/R4）
- 原序 A→B→(B-bisect|C/D)→E→G→F ⇒ 改：**A-lite→E 基建（watch/激励判据）→
  B（B0 方差带→B1 工具钉复跑→B2 watch）→A-full→bisect→C/D（D 用双时刻
  判别）→G→F 贯穿**。规模：bisect 98 commit 二分 ~7 轮×全量 L4（~3-5min emu）
  ≈可行；方案未给工期估计（可信度中等，补估计）。

## 裁定

**需修订后可用**：五主张经实证 4.5/5 成立（③经本复核加码确认并牵连更正我方
scrollseam 旧背书）；方案 A-G 骨架方向正确，但 R1（依赖序反转合法化回退）、
R3（对照有效性判据缺=教训三犯风险）、R4（D 判定不闭环）为 must-fix；R2/R5/
R6/R7/R8 为 should-fix。按修订执行后方案可用。

Evidence: gates/cli.py :1079-1108 实读（match 集规则/无 diff_pixels）；本复核
实测 scrollseam $FD=00/Samus 恒位/42 字节冻结；现 matrix l380=12085；
shots_rust/matrix.md 48/74 vs 46/74 翻转实录；shootprobe 假绿案（r9 在案）；
ens_trace 载体位置（analysis/_gen/）。
