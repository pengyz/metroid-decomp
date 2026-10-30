# Bug 台账（docs/bug-ledger.md）

> 用途：登记**原版（ROM/oracle 行为）的 bug 与 quirks**。按 AGENTS.md D4/§6：
> Stage A **保真保留**原版行为，只登记不修；代码侧以 `// BUG-xxx: 保留原行为` 引用；
> 差分测试遇到已登记 bug = "已知偏差"，基线显式 TOLERATED 并注明 BUG 编号。
>
> 内容维护归 **verifier**（master-plan §4）；本 stub 由 captain 于 2026-10-12 创建
> （轮次日志 round-007，t3 门禁 docs-links 预扫描发现 AGENTS.md §6 引用本文件缺失，
> 裁定方案 A 补齐脚手架）。

## 登记格式（AGENTS.md §6）

```markdown
## BUG-xxx: <一句话现象>
- 现象: <可观察行为描述>
- 复现: <scenario/输入序列/ROM 偏移>
- 影响: <对差分验证/玩法的影响>
- 证据: <trace id | bank:addr | rom:offset | shot:file>
- 处理: Stage A 保真保留（代码注释引用 BUG-xxx）；Stage B 修复计划：<无/说明>
- 登记: <日期 / 登记人>
```

## 台账（当前为空 —— 尚无已证实的原版 bug 登记）

<!-- 新登记追加在下方，按编号递增；证伪不删历史，原地标注日期更正（AGENTS.md §3.6） -->

## BUG-001: morph×楔交互=游戏态损坏（radius 0 退化+位置冻结）

- 现象: morph 球态行进撞楔（球盒 16px 跨 8px 瓦壁——$5D 类）后 un-morph——
  status=8（站立）但 radiusY/radiusX=0 退化+位置冻结（speedY=$FD 持续上推）——
  Samus 卡死不可动；
- 复现: sc-elevator-ride f3380-3750（morph×楔交互）——
  morph 触发（x>120 down）→球滚→撞楔（$5D 壁 x 143）→UP 脱球→状态损坏；
  探针实录：stat($300) 7→8/radiusY($301)=0/radiusX($302)=0/speedY($308)=$FD/ScrY($52)=200 冻结；
- 影响: **差分验证需登记为已知偏差**（TOLERATED+BUG-001 引）——game-core 移植
  面需保真复现（Stage A 保留 quirk——D4）——玩法影响=楔区 morph=永久卡死；
- 证据: 三环探针输出（docs/spec/map-connectivity-graph.md §38）；
  SetSamusRoll/SamusRoll/ObjectCheckMoveUp 指令流（07:D0B5/D0E1/E7A2）；
- trace id: analysis/traces/sc-elevator-ride.frames.jsonl f3380-3750（探针窗）；
- Stage 处置: **Stage A 保真不修**（D4）——代码侧注 `// BUG-001`（porter 移植时引用）；
- 登记: 2026-10-19 decomp（lead D4 台账令）

## BUG-002: Ridley 共用 KraidLintCounter($7E)=弹种计时互扰

- 现象: Ridley 战斗弹种（fireball）发射计时读写 **KraidLintCounter($7E)**——
  Kraid lint 计时表被 Ridley 复用（m1disasm 注："Accidentally used by Ridley too"）——
  两 boss 计时面共享一址；
- 复现: 机制级（代码链）——Ridley 弹种计时路径读 $7E（Kraid lint 计时）——
  触发条件=Ridley 战斗中 fireball 发射计时与 Kraid lint 计时同域；
- 影响: **计时互扰**（若两 boss 同域计时=互相干扰——正常流程单 boss 无感）——
  差分验证登记为已知偏差（TOLERATED+BUG-002 引）——game-core 移植保真复现
  （Stage A 保留 quirk——D4）；
- 证据: constants_ram:627-628（KraidLintCounter $7E 注+KraidNailCounter $7F 注——
  "Accidentally used by Ridley too" 原注）+prg4:171-173（Kraid 弹种派发）+
  prg5:173-174（Ridley 弹种派发）；
- trace id: 未采（boss 战斗场景未达——代码链实证代：disasm 注+RAM 图双锚）；
- Stage 处置: **Stage A 保真不修**（D4）——代码侧注 `// BUG-002: 保留原行为`（porter
  移植 KraidLintCounter/fireball 计时路径时引用）。

## BUG-089：L4 f1000/f1150 残差=渲染/采样时序类（非内容分歧）

- 现象：case-002 f1000/f1150 rust↔emu 截图差 11267/11256px（顶壁/柱纹区）。
- 定性（终结判（2026-10-28））：Mesen2 PPU 真值逐瓦核=**CHR/名表/属性/调色板四面
  全同**（零内容分歧）⇒残差=渲染/采样时序（精灵帧位/细滚相位/截屏瞬间）——
  docs/findings/scroll-content-mapping.md §5。
- 复现：`python3 tools/l4_diff_run.py run-all`（f1000/f1150 点位）+双侧 ppu.hex 对拍。
- 影响：Stage A 像素级验收面（记账容忍（D4））；真值链=语义代码↔Mesen2 RAM/trace
  机器对拍（四面同已证内容一致）。
- trace id：analysis/traces/case-002-door-blue-interaction_f01000_neighbor-room.ppu.hex。

## BUG-090：全程无音乐/音效——shell 音频输出=v1 桩（架构缺口头（待用户裁））

- 现象：用户实测④全程无音乐/音效。
- 定性（终结判（2026-10-28））：声音链全通至 **PcAudio v1 桩**——sound 域产出
  apu_events（sound/mod.rs（域测钉））→sound_hook::sound_dispatch_chain（nmi.rs:203
  （生产径））→mem.write_io($4000-17)→**apu_log 捕获**（memory.rs:49-51（壳层消费源））
  →drain_apu（game-shell-pc/lib.rs:191）→**PcAudio=写流记数不合成**（lib.rs:3/:69
  「音频=v1 桩（cpal 合成随件登记）」）——**无音频设备输出**（架构缺口头（预告类））。
- 处置（lead 裁定）：**不修**——Stage A 视频对拍即可（音频=扩展缝）——待用户裁音频
  方案（cpal 合成随件）。
- 影响：听觉面（非像素验收面）。
- trace id：（apu_log（FrameMemory 内（.ppu.dump 同步面随件））。

## BUG-091：IsObjectVisible 竖滚径轴错配——radiusX 对比 ScreenY（原版怪癖，D4 保真）

- 现象：竖滚房（ScrollDir&2==0 走 VertScrollCheck 07:DFF6）的敌可见性判把
  **radiusX（$09）**与 **ScreenY=y−ScrollY（$10）**做 `cmp`——敌可见 ⟺
  y≥ScrollY 且 radiusX≥y−ScrollY。横向语义应为 radiusY（纵轴跨度）或屏界常量；
  敌在屏内但距屏顶超过 radiusX 即被判不可见（Resting 敌 UpdateEnemy 整体跳过、
  不绘制不激活）。
- 证据：prg7_engine.asm 07:DFF6 `CPY $FC`→`A5 09/C9 10`（radiusX cmp ScreenY）；
  m1disasm 原注自嘲「;why is $09 (radius x) used, but not $08 (radius y)?」
  （:6494 区 IsObjectVisible 头注）——原作者怪癖非转录错。
- 定性：原版行为（D4 保真保留；Stage B 可考虑按轴语义优化）。case-001 走横滚径
  （ScrollDir=0x1E，&2≠0→HorzScrollCheck 轴匹配）未触发——影响面=竖滚房敌可见性。
- 复现：units/房内敌 y=40/radiusX=2/ScrollY=0 竖滚径判不可见（asm 读证即可复现）。
- 影响：竖滚房 Resting 敌激活/绘制时序（玩家可见为「敌睡到屏幕顶部附近才醒」——
  原版即如此）；Stage A 一致性对拍按原版语义。
- trace id：analysis/traces/case-crawlprobe.frames.jsonl（敌槽 d5 bit1 时序面）。
