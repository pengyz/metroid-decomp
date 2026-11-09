# r9 对抗复核：AccuracyCoin oracle 卡（596cbebd）+ adsr 第四源卡（4b611739）

- 日期：2026-11-09
- 复核人：verifier（对抗复核，证伪立场；未修改实施产物业务内容）
- **verdict：needs_revision**（卡 1 为主；卡 2 = pass）
- **Confidence：93**（本档全部关键结论均带本轮实测实录；A 级标注 = 复核人本轮亲手重导）

---

## 卡 1：AccuracyCoin oracle 卡（commit 596cbebd）

### ① 结果真实性：数字真实 ✓，复现链完整性 ✗ → needs_revision 主因

**✓ 结果与 FAIL 表真实（A 级独立重导）**：对既有 `analysis/accuracycoin/acc_run5.log` 独立跑
`python3 tools/oracle/accuracycoin/decode_accuracycoin.py acc_run5.log AccuracyCoin.asm`，实录：

```
$0037 TestTally=147  $0038 PassTally=128
tests=149 pass=128 fail=19
  045B Address2004_Behavior v=2A err=0Ah   …（19 条逐条 err 码与档案 §4 表完全一致，含 v 值）
```

128+19=147 与 `$0037/$0038` 精确闭合 ✓；19 条 FAIL 的槽位/名字/错码逐项一致 ✓；
槽语义对源自洽（asm:80-81 `PostAllTestTally=$37/PostAllPassTally=$38` ✓；
asm:1009-1015 `AND #01 / BEQ fail / INC PostAllPassTally` 证 bit0=1=PASS、err=byte>>2 ✓）。

**✗ 但「复现链可独立复跑出 128/147」被证伪（A 级实测，两轮）**：

- 实测 run2（21:06–21:13，committed 链原样 `run_accuracycoin.py`，无任何干扰）：ENDDUMP 到达
  （f=16000，10 帧全 dump），**全部帧 `$0037=0 $0038=0`、RAM nonzero 恒 352**（battery 未跑，
  停在菜单）。实录：`f=600 $37=0 $38=0 … f=16000 $37=0 $38=0`。
- 根因（A 级，asm 证实）：AccuracyCoin battery **只在菜单按 Start 才跑**——
  `AccuracyCoin.asm:18183-18186`：`AND #$10 ; Start … JSR AutomaticallyRunEveryTestInROM`；
  committed 的 `accuracycoin_dump.lua` **无任何输入注入**（对照 `tools/scenario_capture.lua:28`
  有 `emu.setInput` 先例而本 lua 未用）。
- 反证闭环（A 级，21:24）：复核人自写 probe lua 在 f=60-90 注入 `emu.setInput({start=true},0)`
  （inputPolled 相位，其余与 committed 链同环境同 ROM 同 binary），实测：

  ```
  f=600 $37=147 $38=128
  f=2400 $37=147 $38=128
  f=4200 $37=147 $38=128
  f=6000 $37=147 $38=128
  ```

  ⇒ **128/147 结果本身可复现且稳定（Mesen2 2.1.1 判定无回归），缺的是 Start 注入这一步**。
  档案 §3.5「开机自动跑全 battery、无需逐项选择」表述与 asm 事实不符（自动跑≠开机自动，
  需一次 Start；`AutomaticallyRunEveryTestInROM` 是按键后调用的子程序名，非开机特性）。

**附随工具链缺陷（A 级实测，同属本卡复现链）**：

1. `run_accuracycoin.py` 的 `--out` 监听路径永不出现：Lua 未收到 out 参数，`io.open` 相对路径
   实测落在 **Mesen 配置目录**（`~/.config/Mesen2/accuracycoin_dump.log`），不是
   `analysis/accuracycoin/…` ⇒ 脚本恒 timeout（`timeout — partial dump kept`），且该 dump 文件
   会被下一次运行 `io.open(…,"w")` 静默覆盖。`acc_run5.log` 的产生方式（谁按了 Start、
   dump 从哪里搬来）在档案中无记载——provenance 链断点。
2. HOME 隔离无效（A 级）：`run_accuracycoin.py` 传 `env HOME=~/.config/acc-home`，但实测
   dump/`RecentGames/AccuracyCoin.rgd`/`SaveStates/*.mss` 全部落真实用户目录
   `~/.config/Mesen2/`——本 Mesen2 2.1.1 build 忽略 HOME 覆盖（或仅部分路径响应）。
   档案 §3.2 的「向 HOME 拷 settings.json」处置据此存疑。

**最小修复处方（卡 1-①）**：
a. `accuracycoin_dump.lua` 增加 Start 注入（inputPolled 相位，f≈60-90 注入 8 帧，参照
   `tools/scenario_capture.lua:28` 的 `emu.setInput` 用法）；
b. Lua OUT 改绝对路径或 run 脚本把 out 经 Mesen 参数传给 Lua（`arg[1]`），并对旧 dump 改为
   覆盖前先 rename 备份；
c. findings §3.5 更正为「需一次 Start 注入后自动跑全 battery（asm:18183-18186）」；
d. 档案补记 acc_run5.log 的实际产生步骤（输入注入/人工按键），否则 provenance 断链。

### ② 「CPU 语义面背书 / PPU 面不做主张」适用面划分：✗（1 项误归 + 1 项数字错）→ needs_revision

逐项核对 asm suite 归属与 §5 定性：

- **✗ IFlagLatency（槽 $0461，err $0E）被误归**：它在 asm menu **「CPU Interrupts」suite**
  （asm:703 `table "Interrupt flag latency"`），测的是**中断标志（I flag）更新时延**
  （TEST_IFlagLatency* 用 DMC DMA IRQ + clockslide 验 CLI/SEI/PLP 后 I 生效时点）——
  是 **CPU 中断语义/时序项**，不是 PPU 渲染、不是 DMA 冲突、也不是虚拟读。
  §5 的 FAIL 枚举清单（$2004/$2007 应力、移位寄存器残态、branch 虚读、internal bus/ALE/
  HybridAddresses、OAM、DMC reload）**未包含 IFlagLatency**；「19 FAIL 全聚 PPU/DMA/虚拟读
  前沿」作为全称判断被此一项推翻。同族边缘项：`ImpliedDummyRead`（CPU Behavior 2 suite，
  asm:741）虽属 CPU 面 dummy read，§5 只列了 branch 虚读、漏列 ImpliedDummyRead。
  **对主结论的实质影响有限**（IFlagLatency 是周期级 IRQ 时序前沿测试，不动摇「指令语义
  100% 通过 ⇒ CPU 行为级差分背书」的核心——NMI×5/APU 计数器/手柄/InstructionTiming 均
  PASS；但边界声明必须如实列举）。
- **✗ 「全部 68 项未文档化指令」数字错（A 级）**：`grep -cE "^result_UnOp_" AccuracyCoin.asm`
  = **66**（SLO7+ANC2+ASR/ARR/ANE/LXA/AXS/SBC_EB6+RLA7+SRE7 含 SRE_47+RRA7+SAX4+LAX6+
  DCP6+ISC6+SHA2+SHS/SHY/SHX/LAE4）；解码器 PASS 列表逐名清点亦 66 条 UnOp。
  应更正为 66（或注明 68 的出处口径，本轮 asm 证据只支持 66）。
- **引用行号漂移（轻）**：档案 §4「SRE_47 哨兵（asm:159）」实际为
  `AccuracyCoin.asm:179`（`result_UnOp_SRE_47 = $47F ; …I need address $421 to always be $00`）；
  asm:159 处是 `result_UnOp_SLO_1F = $40F`。
- **decoder 附注（轻，非错）**：decode 输出 `tests=149` 含 2 个 NOT-RUN 槽（`$0401 CPUInstr`、
  `$0464 RMW2007`）；后者上游已注释停用（asm:807 `;table "RMW 2007 Extra Write" … Commented out
  for now`）。128+19+2=149，与「128+19=147 精确闭合」不矛盾，但档案未提 RMW2007 停用项。

**最小修复处方（卡 1-②）**：§5 枚举补 IFlagLatency（归类「CPU 中断标志时序前沿」）、
补 ImpliedDummyRead；「全部 68 项」→「全部 66 项（result_UnOp_* 机械计数）」；
asm:159 → asm:179；§4 或 §6 附注 RMW2007/CPUInstr 两 NOT-RUN 槽的存在与原因。

### ③ headless 陷阱四件证据强度

| # | 陷阱 | 复核强度 | 实录 |
|---|---|---|---|
| 1 | 单实例静默转发 exit 0 | **A 级 ✓（本轮亲手复现）** | 在跑实例存在时二次启动（两种 HOME 配置各一次）：`exit code: 0`，stdout+stderr 共 **0 字节**。副作用实测推证：run1（20:54 启动）期间 20:58 的二次启动疑似把 Metroid ROM 转发进在跑实例（run1 无任何 dump 产物）——转发会**劫持在跑实例当前 ROM**，危害比档案描述（只是「转发参数」）更重，建议补记 |
| 2 | 全新 HOME 无 settings.json → 向导秒退 | B 级（本轮未独立复现；本地 Mesen 忽略 HOME 覆盖使该场景难以构造，见 ①附-2） | 档案称 strace/进程证据，未见留档命令 |
| 3 | `pgrep -f`/`pkill -f` 孤儿清扫自匹配误杀 | **A 级 ✓（本轮两次踩中）** | `pkill -f "tools/mesen2/Mesen"` 内联执行两次把 bash -c 自身 SIGTERM（cmdline 含同串）——`scenario_run.py:754-788` 的 `pgrep -f "tools/mesen2/Mesen"` 自匹配风险逐行核实成立；脚本文件执行（pkill -x）可避险，与档案处方一致 |
| 4 | `emu.read(returnSigned, memType, address)` 签名 / 缺省全 0 | B 级（行为面旁证 ✓：本轮无注入轮 RAM 恒 352 nonzero 与全 0 tally 并存，与「缺省 0 全返 0」现象一致；LuaApi.cpp:244-262 行号本地无 Mesen 源码不可证） | 本地未找到 Mesen C++ 源码 checkout，行号引用应标注为 GitHub 源引 |

### ④ 防弱化审计：✓

- `git show 596cbebd --stat`：4 个全新文件、307 insertions、0 deletions——**未触碰** gates/
  baselines/白名单/判读阈值 ✓。
- decoder 无吞错路径：NOT-RUN 显式打印（`status = PASS if v&1 else FAIL if v else NOT-RUN`），
  无 tolerance、无白名单、无 err 码改判 ✓。
- run 脚本 timeout 分支保留 partial dump 并显式打印，不静默 ✓（其监听路径缺陷见 ①，属 bug
  非弱化）。

### 卡 1 小结

结果数字、FAIL 表、槽语义、防弱化全部成立；但 (a) committed 复现链缺 Start 注入、独立复跑
得 0/147（A 级证伪），(b) §5 的 FAIL 全称归类含 IFlagLatency 误归、68→66 数字错。两者均为
**轻量文档/脚本修复**，不动摇「CPU 语义面 oracle 背书、扫描线级 PPU 不做主张」的核心边界。
**needs_revision（轻）**。

---

## 卡 2：adsr 第四源卡（commit 4b611739）

### ① 五地址等价性独立重导：✓（A 级）

按 `file=0x10+bank*0x4000+(cpu-0x8000)` 从 `rom/Metroid (USA).nes` 独立提取，实录：

```
$8F2D bank0 file=0xF3D : b9 88 69 4a 4a 60 b9 88 69 29 03 20 c5 c2 85 00 b9 89 69 20 bf c2 05 00
$8F33 bank0 file=0xF43 : b9 88 69 29 03 20 c5 c2 85 00 b9 89 69 20 bf c2 05 00 60 b9 88 69 29 0f
$8F46 bank0 file=0xF56 : b9 88 69 29 0f 0a 0a 85 00 b9 89 69 2a 2a 2a 29 03 05 00 60 b9 88 69 29
$8F5A bank0 file=0xF6A : b9 88 69 29 3f 60 a0 00 20 f1 8f 8d 88 69 a0 01 20 01 90 8d 89 69 a0 02
$BBD0 bank1 file=0x7BE0: 0a 0a c9 3c 90 02 a9 3c 8d 2a 06 4c 37 bb ad 88 06 29 fc d0 12 b9 00 b2
$BBD0 bank0 file=0x3BE0: 0a 0a c9 3c 90 02 a9 3c 8d 2a 06 4c 37 bb ad 88 06 29 fc d0 12 b9 00 b2
```

与档案 §1 各表逐字节一致 ✓；`$BBD0` bank0/bank1 双副本同字节 ✓（旁证 m1disasm §6 副本结构）；
档案 §1.5 复现命令自检 ✓（其 `d[0x10+0xF2D:…]` 与公式自洽）。

### ② 谱系警示独立抽验：✓（A 级，三方逐字符一致）

- 抽验符号 1 `SixUpperBits` 调用行：
  - adsr `analysis/adsr-nes-metroid/repo/MetroidTitlePage.asm:1720`：
    `L8E6E:	JSR SixUpperBits		;($8F2D)`
  - nmikstas `refs/nmikstas-metroid-disassembly/Source_Files/Bank00.asm:1889`：同文（仅空白对齐差）
  - MMC3 `refs/MetroidMMC3/dev.Original/code/Title.asm:1720`：同文
  ⇒ 标签+符号+行内注释三方逐字符一致 ✓。
- 抽验符号 2 同址异名 `$BBFA`：adsr `MetroidBrinstarPage.asm:5453 InitMusicIndexTbl:` vs
  m1disasm `refs/m1disasm/SRC/sound_engine.asm:2042 SongHeaderOffsetTable: ; $BBFA` ✓。
- 抽验符号 3 m1disasm 同名同址行号抽查：prg0_title.asm:2126/2134/2147/2165/2174
  （SixUpperBits/TwoLowerAndFourUpper/FourLowerAndTwoUpper/SixLowerBits/ConsolidatePassword）
  与档案 §3.1 引用行号逐一吻合 ✓。
- m1disasm `SixUpperBits: ; 00:8F2D`（prg0_title.asm:2126）与 adsr `SixUpperBits:`（repo
  MetroidTitlePage.asm:1793，段体前）同址同名 ✓。
- 交叉调用字面自证：`JSR $C2C5`/`JSR $C2BF` 操作数在 §1.1 hexdump 内可见（`20 c5 c2`、
  `20 bf c2`）✓。
⇒ 「adsr=metroid-database mirror，名义四源实为两谱系」的警示成立且证据可独立重导 ✓。

### ③ 零 rename 断言与 C 级纪律：✓

- `git show 4b611739 --stat`：仅 `docs/findings/adsr-fourth-source-crossref.md`（+118）与
  `docs/knowledge/gotcha-mesen2-single-instance-silent-forward.md`（+30）——**data/naming/ 未写** ✓。
- 文档纪律：候选全部标注「B：」证据级；$BBA8 显式标注语义分歧点并写明「升 A 需 trace」；
  §3.3 明示「adsr .alias RAM 符号……本轮未逐条对读，遗留假设区」——未发现伪装成结论的
  C 级假设 ✓。唯一注记：§2「同一套标签同文注释」的逐字符一致性，复核仅独立抽验 1 个符号
  三方 + 2 个同址异名/同名同址；样本量与文档自述口径（「如 L8E6E…」举例式主张）相称，不构成
  needs_revision，但后续引用「谱系同一」时建议补 2-3 个符号的批量 diff（`diff` 两 asm 文件
  归一空白后）作为机械化锚点。

### ④ 误卷入的 gotcha（docs/knowledge/gotcha-mesen2-single-instance-silent-forward.md）技术准确性

- 行为面主张（单实例并发 → 转发后 exit 0、不报错无日志）：**A 级 ✓ 本轮复现**（见卡 1-③#1）。
- 「转发给已有实例」的具体语义（在跑实例会打开被转发 ROM）：本轮间接证据（run1 疑被劫持）
  支持但未直接观测到实例切换画面——建议 gotcha 措辞降为「疑似会在跑实例加载转发 ROM（有
  run1 无产物旁证）」或补一次直接观测后升 A。
- `Mesen2 Program.cs:78 SingleInstance`：本地无 Mesen C++ 源码 checkout，行号不可本地复核
  （B 级）；建议 gotcha 注明「行号引 GitHub master 源，行为面已本地实测」。
- 「全新 HOME 无 settings.json → wizard 秒退」：本轮未复现（与卡 1-①附-2 的 HOME 隔离失效
  现象存在张力——HOME 覆盖对本 build 部分无效， wizard 分支的构造方式需复核），标注
  B 级待复测。
- 处置范式（`pgrep -x Mesen` + 存活确认；pkill 改脚本文件）：与本轮两次 `pkill -f` 自杀实录
  相容 ✓。

### 卡 2 小结

五地址字节等价、谱系警示、零 rename、B 级纪律全部独立复核成立；gotcha 行为主张成立、
两处源码行号主张本地不可证（建议标注来源）。**verdict：pass**（附两条轻量注记，不阻塞）。

---

## 总 verdict 与处方汇总

**verdict：needs_revision**（卡 1；卡 2 pass）
**Confidence：93**

needs_revision 项（均轻量，最小修复）：

1. 【卡1-①】`tools/oracle/accuracycoin/accuracycoin_dump.lua` 补 Start 注入
   （emu.setInput @inputPolled，参照 tools/scenario_capture.lua:28）；run_accuracycoin.py 修
   --out 传递/监听路径（Lua io 落点=Mesen 配置目录）+ dump 覆盖前备份；findings §3.5 改为
   「需一次 Start 注入（asm:18183-18186）后自动跑全 battery」；补记 acc_run5.log 产生步骤。
2. 【卡1-②】findings §5：FAIL 归类补 IFlagLatency（CPU 中断标志时序，asm suite「CPU
   Interrupts」asm:703）与 ImpliedDummyRead；「68 项」→「66 项（result_UnOp_* 机械计数）」；
   asm:159 → asm:179；附注 RMW2007（asm:807 上游停用）与 CPUInstr 两个 NOT-RUN 槽。
3. 【卡1-③/卡2-④】Program.cs:78、LuaApi.cpp:244-262 行号主张标注「GitHub 源引，本地无
   checkout」；gotcha「转发会劫持在跑实例 ROM」按本轮 run1 旁证降级措辞或补直接观测；
   HOME 隔离失效现象（dump/rgd/mss 落真实用户目录）补进陷阱档。
4. 【卡2，注记不阻塞】谱系「逐字符一致」建议补归一空白后的批量 diff 锚点。

复核通过后本卡核心结论（CPU 语义面 oracle 背书 128/66 undocumented 全 PASS、扫描线级 PPU
不做主张、adsr 双谱系交叉合法、零 rename）可按现状消费，无需回滚。
