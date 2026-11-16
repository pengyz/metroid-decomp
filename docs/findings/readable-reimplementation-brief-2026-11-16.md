# 可读重实现的机会、社区全景与交接书（NES Metroid，2026-11-16）

> **本文性质**：决策支持 + 交接文档（给"来做这件事的人"看），**不是取证档案**。
> 取证结论见 [rom-disasm-equivalence.md](rom-disasm-equivalence.md)（ROM ↔ m1disasm 逐字节等价）；
> 社区资料的分级扫描见 [community-sources-scan-2026-11-09.md](community-sources-scan-2026-11-09.md)。
> **证据分级**按 [AGENTS.md](../../AGENTS.md) §3：`[A]` 本地机械复现 / `[B]` 可复现静态推理 / `[C]` 第三方线索（D5 不背书）。

---

## 0. 一页结论

1. **社区里"可读的高级语言重实现"是空位。** 现存三类产物——可读汇编（m1disasm / MetroidMMC3）、机器直译 C（MetroidNESRecomp）、ROM 编辑器（Editroid）——都不是"能读、能改、能验证"的实现。[C]
2. **需求真实但属"未来式"。** 社区明确想要"建立在反汇编之上的编辑器/引擎"，但主流判断是 decomp hacking 短期不会取代 binary hacking（原话见 §2）。[C]
3. **体量对"一个人 + AI"成立，但成本不在代码行数。** 1135 个例程 / 19789 行汇编，67% 的例程 ≤15 行；真正的成本是**证明等价**（行为面分母不可机械导出）与**音频**（132 例程无门禁）。[A]
4. **选择 M1 而非 Super Metroid 的理由不是"小"，是"验证闭环能在单人尺度内闭合"。** 128KB、无协处理器、单 bank、社区已有逐字节等价反汇编。[A]
5. **我们的差异化只有一条：可读 + 可验证等价。** 不是"能玩"（MetroidNESRecomp 已经能跑，还有宽屏/voxel）。

---

## 1. 社区全景

### 1.1 枢纽与资源

| 资源 | 形态 | 状态 |
|---|---|---|
| [Metroid Construction](https://metroidconstruction.com/)（2009 起） | 站点 + wiki + 论坛 + Discord/IRC | M1 唯一枢纽；wiki 的 [m1 页](https://wiki.metroidconstruction.com/doku.php?id=m1) 列编辑器/文档/hack 索引，hacking popularity 排第 3 [C] |
| [metroidret/m1disasm](https://github.com/metroidret/m1disasm) | WLA-DX 反汇编，**可汇编出逐字节一致 ROM** | 我们已复验等价（`70080810`）[A] |
| [ZaneDubya/MetroidMMC3](https://github.com/ZaneDubya/MetroidMMC3) | Ophis 反汇编 + MMC3 移植，重组织和注释 | 行为等价声明，非字节等价 [C] |
| [nmikstas/metroid-disassembly](https://github.com/nmikstas/metroid-disassembly) | 另一份反汇编 | 未做等价验证 [C] |
| [snarfblam/Editroid](https://github.com/snarfblam/Editroid) | C# 关卡编辑器（直接改 ROM 字节） | 社区事实标准；3.7 稳定 / 4.0 alpha 有 bug [C] |
| [Data Crystal: Metroid](https://datacrystal.tcrf.net/wiki/Metroid) | ROM map / RAM map | 自述 "rather incomplete"；**CRC/MD5 与我们 ROM 逐项一致** [A] |
| M1R / m1rand / Metrandomizer | Randomizer + 竞速（[racetime.gg/m1r](https://racetime.gg/m1r)） | 活跃；要求 headerless CRC `70080810` [C] |
| speedrun.com practice hack | 帧计数 / 房间计时 / Kraid HP | 竞速社区工具 [C] |

### 1.2 修改 ROM 到底多不多

- **绝对数量少。** RHDN 快照（`zach-morris/romhack_db`，756 个 NES hack）中以 Metroid 为底包的只有 **24 个（约 3%）**；MetConst 论坛有人直言 "**There are only 4 m1 resources on metroid construction**"。[C]
- **但单个 hack 的深度高。** 代表（括号内为 RHDN hack id，2024 年 RHDN 转型新闻站后原页面已迁 Internet Archive，检索请用 id/名称）：Metroid + Saving (#1186)、Automap (#1974)、wall jump (#1975)、mOTHER (#1988)、Rogue Dawn (#3280)（事实上的 M1 续作）、Deluxe Reduxe (#3134)、MMC5 转换、Mesen HD Pack。[C]
- **趋势**：近两年从"改关卡"转向"改引擎/反汇编级"（H A M 的多分支 m1disasm、m1disasm-editor）。[C]

### 1.3 社区在解决什么问题

1. **QoL 现代化**：密码→存档、地图、满血复活、光束堆叠、门过渡、日版 RNG、8 E-tank。
2. **硬件容量**：8KB CHR-RAM、8 sprites/scanline → MMC3/MMC5、运行时换 CHR tile。
3. **关卡编辑**：Editroid 改布局后还要同步密码表（"改完能跑但密码失效"是经典坑）。
4. **随机化与导航**："Metroid curse"（没地图导致的迷失）是 randomizer 社区的核心痛点。
5. **未用内容/bug 取证**：ROM 里存在原作者未使用的内容，社区靠反汇编挖。
6. **引擎级改造 vs 二进制 hack 的路线之争**（见 §2）。

---

## 2. 社区态度（原话与出处）

**① 想要"建立在反汇编之上的工具/引擎"** —— [MetConst topic=6060](https://forum.metroidconstruction.com/index.php/topic,6060.0/printpage.html)：

> "I wish there's a fork of Editroid that uses a disassembly instead of a ROM. Same thing for SMART and MAGE."
>
> "There are only 4 m1 resources on metroid construction."

**② 对 decomp 路线的时间判断** —— [MetConst topic=6083](https://forum.metroidconstruction.com/index.php/topic=6083.0)，alexman25：

> "It's pretty awesome… **I don't think decomp hacking replacing binary hacking any time soon for NEStroid, but it certainly will eventually.**"

**③ 愿意出人力做"前置工序"（逐例程命名/文档化）** —— [nesdev 2014 帖](https://forums.nesdev.org/viewtopic.php?t=11529)，snarfblam：

> "take on a single undocumented routine … each remaining routine would be a nice bite-sized chunk of code to take on."

**④ 先例的失败面**：M1 重制/引擎复刻帖 2016 问到 2026 无果（[topic=4036](https://forum.metroidconstruction.com/index.php/topic,4036.0/wap2.html)）；RHDN 论坛 "Metroid 1 Ultimate Remake"（基于 Rogue Dawn 引擎，topic 31025）从 2020 做到 2025。[C]

**解读**：社区需要"能改引擎的东西"，但**没人愿意先花两年做看不见效果的基础设施**——这是空位的成因，也是我们的机会：他们已经用手工方式在拆例程，缺的是骨架和可信度背书。

---

## 3. 产物光谱：可读重实现是空位

| 类型 | 代表 | 能读？ | 能改？ | 能验证？ |
|---|---|---|---|---|
| 可读汇编 | m1disasm / MetroidMMC3 | ✅（要 6502 功底） | ✅ | 部分（m1disasm 可字节等价） |
| 机器直译 C | MetroidNESRecomp | ❌ | ❌ | ❌（能跑，不担保等价） |
| ROM 编辑器 | Editroid | —（无源码层） | ✅（改字节） | ❌ |
| **可读重实现（现代语言）** | **无** | ✅ | ✅ | ✅（需自建） |

### 3.1 MetroidNESRecomp 取证结论（不是语义改写）

抓取 `generated/metroid_full_bank07_part00.c` 原文 [C]：**一条 C 语句对应一条 6502 指令**，每个地址一个 `label_XXXX:`，CPU 状态全在全局变量：

```c
void func_FFB0(void) { /* ROMFIXED_RESET */
label_FFB0:;
  /* $FFB0: 78 */ nes_cpu_instruction_boundary(0xFFB0, 2); g_cpu.I = 1;
  /* $FFB7: 8E */            nes_write(0x2000, g_cpu.X);
  /* $FFBD: 10 */            if (!g_cpu.N) { ...; goto label_FFBA; }
  { uint8_t p = (g_cpu.N<<7)|(g_cpu.V<<6)|0x30|(g_cpu.D<<3)|(g_cpu.I<<2)|(g_cpu.Z<<1)|g_cpu.C;
    g_ram[0x100 + g_cpu.S] = p; g_cpu.S--; }
  /* $FFD2: 4C */            func_C01A(); return;
}
```

它的 `game.toml` 里确实有**语义知识**，但只用于**纠正 codegen**，不改写逻辑：`[[inline_dispatch]] 0xC27C`（ChooseRoutine，PLA 式派发）、`0x8024`（area-bank veneer）、`[[stack_bail_func]] 0xC376`（PLA PLA + RTS 上跳两层）、`[[cond_bail_func]] 0xC36B / 0xEF00`、`push_all_jsr`、`extra_func`。手写语义只在外壳：`extras.c`（帧回调、密码→SRAM）、`metroid_ws*.c`（room decoder、宽屏合成）、`game_voxel.c`——是**新增行为**。

结论：它是"等价可执行产物"，**不是可读重实现**；而且它不会转向（codegen 是自动的，转语义重写等于重做）。它替我们证明的是：**需求真实、且有人愿意做到能跑**。

---

## 4. 先例：同类物种在 NES 圈已存在

| 项目 | 定位 | 关键约束 |
|---|---|---|
| [nukep/SMB-Vanilla](https://github.com/nukep/SMB-Vanilla) | "A C decompilation and port of … Super Mario Bros" | 需自带 ROM；不含关卡/美术资源；GPL-3.0；2026-10 仍在提交 |
| [MitchellSternke/SuperMarioBros-C](https://relatedrepos.com/gh/MitchellSternke/SuperMarioBros-C) | "translate … to **readable C/C++**" | 794★，证明"可读"本身是卖点 |
| [recomp.fyi SMB 看板](https://recomp.fyi/game/super-mario-bros-nes) | 9 个项目：4 recomp / 3 source port / 2 decomp | **品类已被社区制度化登记** |
| sm64decomp / sm64ex、snesrev/smw、PrimeDecomp → [MetroidPrimePort](https://github.com/Odrannnn/MetroidPrimePort) | 各代反编译范式 | 都遵循 BYO-ROM / 不含 assets |

**空白位的事实**：Metroid NES 上 `decomp` 与 `source port` 两类都**为 0**，只有 `recomp`（MetroidNESRecomp）。

---

## 5. 体量实测（决定"一个人 + AI"是否成立）

来源：`data/status/routine_inventory.json`（`refs/m1disasm` @ `d6f4d3f` 生成）[A]

| 指标 | 实测 |
|---|---|
| 例程总数 | **1135** |
| 汇编总行数 | **19789**（中位数 9 行 / 均值 17.4 / 最大 192） |
| ≤5 行 | 326（29%） |
| 6–15 行 | 433（38%） |
| 16–40 行 | 252（22%） |
| 41–100 行 | 109（10%） |
| >100 行 | **15（1.3%）** |
| 已移植 | 30 个（2.6%）/ 359 行（**1.8%**） |

**分布的含义**：工作单位是"大量小、独立、可单测、可单独对拍"的单元——这是 AI 最能放大的形状。但要注意两个反直觉点：

1. **已移植的 30 个都是小件**（crawler/skree/waver/ripper 等，均值 12 行），15 个大件（房间绘制、碰撞、NMI 引擎）全部 unported ⇒ 按"难例程"算进度接近 0。
2. **成本不在这 19789 行**，而在：
   - **音频**：`sound_engine` 132 例程 / 1937 行（≈10% 代码），APU 时序/包络/DPCM，**当前无 audio 面门禁**——最大盲区。
   - **行为面分母**：RNG、帧相位、8 sprites/scanline 闪烁、密码系统——[AGENTS.md](../../AGENTS.md) D7 明确"行为空间无机械分母"，这是"能否闭环"的真正天花板。
   - **Stage B 是无限空间**，必须只挑"证明扩展缝存在"的点做。

---

## 6. 为什么是 M1 而不是 Super Metroid

| 维度 | NES Metroid | Super Metroid |
|---|---|---|
| ROM 规模 | 128KB PRG / 8KB CHR-RAM | 3MB，含协处理器外设 |
| 硬件怪癖 | MMC1 单 bank 切换 | 多种 DMA/HDMA、Mode 7、SA-1 类复杂性 |
| 社区反汇编 | **逐字节等价可验证**（我们已验 `70080810`） | 有 `sm_disassembly` 等多年积累，但等价背书分散 |
| 单人闭环验证 | **可行** | 不可行（知识壁垒 + 体量） |

判断：**不是 SM 大到写不完，而是没人能替你担保等价。** 选 M1 = 唯一能让"可读重写"与"可验证等价"落在同一人手里闭环的标的。

---

## 7. 差异化与不可越的红线

**差异化（唯一可辩护的一句）**：我们产出"**能读、能改、且被差分门禁证明等价**"的实现——社区缺的正是最后那一项。

**红线**（违反即失去社区信任 / 触碰法律）：

1. **不分发 ROM 或其派生数据**：遵循 AM2R 2016 DMCA 教训（[RPS](https://www.rockpapershotgun.com/metroid-ii-remake-goes-bye-bye)、[NintendoLife](https://www.nintendolife.com/news/2016/09/nintendo_of_america_issues_takedown_request_on_am2r_ending_the_project)）与 [MetroidPrimePort](https://github.com/Odrannnn/MetroidPrimePort) 的 BYO 镜像范式：仓库不含版权数据，资源由用户自带 ROM 抽取。
2. **不宣称"未验证的等价"**：任何"行为一致"的说法必须挂门禁输出（D3）。
3. **不顺手修 bug**：Stage A 保真保留原版 quirks，登记 `BUG-xxx`，Stage B 才修（D4）。
4. **第三方命名不背书**：m1disasm / MetroidMMC3 的符号是别人的语义结论，采用前自证并落台账（D5）。

---

## 8. 交接：里程碑与完成判据

> 纪律入口：[AGENTS.md](../../AGENTS.md)；长线目标：[north-star.md](../north-star.md)；管线：[ai-decompile-pipeline.md](../ai-decompile-pipeline.md)。
> **每个里程碑的"完成"= 跑通指定的门禁命令，不是"看起来对"。**

### M0 · 解锁并落库 bump（当前阻塞项）

- 现状：`refs/m1disasm` 已 bump 到 `d6f4d3f` 并复验等价；`docs/findings/rom-disasm-equivalence.md` §7 已写；`data/status/routine_inventory.json` 已按新分母重生成。三件套**已改但未 commit**。
- 阻塞原因：工作树里另一会话的未提交 WIP 占据暂存面（`crates/game-core/src/enemy/*.rs`、`weapon/projectile.rs`、`data/status/l4_ratchet_baseline.json`、`docs/findings/case-002-three-roots.md`），门禁 `staged-surface` 判定"暂存面与工作树不同源"。规则：`SURFACE_PREFIXES = ("crates/","gates/","tools/","data/status/")`，staged 触及该前缀且 unstaged 同域即 FAIL。
- **完成判据**：对方 WIP 落地后 `git add` 三件套 → `python3 gates/cli.py check` 全绿 → commit message 含 `Evidence` / `Confidence`（参考 [AGENTS.md](../../AGENTS.md) §9）。
- 禁止：`--no-verify`、替对方提交、动对方的文件。

### M1 · 最小可信展示（对社区唯一有说服力的东西）

- 内容：用已 ported 的 30 个例程 + 一个完整房间，产出**一条可复现命令 + 一份 diff 报告**。
- 复用：`python3 gates/cli.py trace`（L1–L3 差分）、`python3 tools/l4_diff_run.py run <case>`（L4 矩阵）、`cases/*.txt`（场景脚本）、`tools/scenario_run.py`（emu 腿）。
- **完成判据**：`python3 gates/cli.py l4` 逐采集点 ratchet 无回退；报告里每个结论带 trace 采集点 ID。
- 反例警戒：不要用"能通关"当证据（那不可复现）。

### M2 · 可读性基线

- 内容：给已 port 模块定可读标准——语义命名（非 `table_0320` 类伪语义）、结构体替代裸 RAM 偏移、注释挂 `bank:addr`、禁止 `unported!` 桩混入"已完成"模块。
- **同时清债**：`refs/m1disasm` 上游语义改名导致 **40 个旧 label** 仍在仓库文档/台账/crates 中被引用（地址有效、名字过期），全表见 [rom-disasm-equivalence.md](rom-disasm-equivalence.md) §7.3。
- **完成判据**：`python3 gates/cli.py check` 绿 + `grep` 证实 0 处旧名残留（按 §7.3 表逐条）。

### M3 · 对外 alpha

- 内容：BYO-ROM 构建脚本、README（构建/运行/验证三步）、一段 30 秒对比视频（原版 vs 重制同帧）、发帖到 [MetConst 论坛](https://forum.metroidconstruction.com/) 与 [RHDN forum](https://www.romhacking.net/forum/)。
- **完成判据**：干净机器（无 ROM）按 README 能跑到"提示需要 ROM"；有 ROM 能跑起来且 `gates trace` 绿。
- 叙事纪律：主打"**可验证的等价性 + 可读可改**"，不主打"又一个能跑的重制"。

---

## 9. 风险与未解问题

| # | 问题 | 影响 | 处置 |
|---|---|---|---|
| R1 | **音频无门禁**（132 例程 / 1937 行） | Stage A 永远差一块；对外演示音频不可信 | 登记 `docs/knowledge/gap/`，优先建"音频 trace 抽样对拍" |
| R2 | 行为面分母不可机械导出（D7 自述） | 无法回答"还差多少" | 用场景清单 + `unported!` 桩做确定性爆点，不假装完备 |
| R3 | 注意力竞争：MetroidNESRecomp 正在宽屏/voxel 出圈 | 我们 Stage B 叙事被削弱 | 重心压 Stage A；Stage B 只做"证明扩展缝"的最小集 |
| R4 | RHDN 2024 转新闻站、MetConst 曾数据丢失 | 引用链接会失效 | 本文链接均注明抓取日期；引用前先验证仍可访问 |
| R5 | 40 条旧 label 引用（§7.3） | 文档/台账与代码对不上 | M2 批量替换，逐条按表 |
| R6 | commit 面被并发会话占用 | 证据锚悬空 | M0，等对方落地，不用 `--no-verify` |

---

## 10. 参考链接（分级）

**A 级（本地已复现）**
- [rom-disasm-equivalence.md](rom-disasm-equivalence.md) — ROM ↔ m1disasm 逐字节等价与 §7.3 改名影响表
- [community-sources-scan-2026-11-09.md](community-sources-scan-2026-11-09.md) — 社区资料分级扫描
- `data/status/routine_inventory.json` — 1135 例程分母与移植状态
- [Data Crystal: Metroid](https://datacrystal.tcrf.net/wiki/Metroid) — CRC/MD5 与我们 ROM 逐项一致

**C 级（第三方线索，引用前请自行验证可访问）**
- MetConst wiki m1 页：https://wiki.metroidconstruction.com/doku.php?id=m1
- MetConst 帖（态度）：https://forum.metroidconstruction.com/index.php/topic,6060.0/printpage.html 、https://forum.metroidconstruction.com/index.php/topic=6083.0 、https://forum.metroidconstruction.com/index.php/topic,4036.0/wap2.html
- nesdev MMC3 众包文档化提议：https://forums.nesdev.org/viewtopic.php?t=11529
- MetroidNESRecomp：https://github.com/mstan/MetroidNESRecomp （生成代码 `generated/metroid_full_bank07_part00.c`；`game.toml` 的 codegen hints）；[PlayPorted 记录](https://playported.com/ports/175-metroid/)
- SMB 先例：https://github.com/nukep/SMB-Vanilla 、https://relatedrepos.com/gh/MitchellSternke/SuperMarioBros-C 、https://recomp.fyi/game/super-mario-bros-nes
- 分发红线：https://www.rockpapershotgun.com/metroid-ii-remake-goes-bye-bye 、https://www.nintendolife.com/news/2016/09/nintendo_of_america_issues_takedown_request_on_am2r_ending_the_project 、https://github.com/Odrannnn/MetroidPrimePort

---

## 11. 给实现者的阅读顺序

1. [AGENTS.md](../../AGENTS.md) —— 纪律总纲（D1–D7 + §3 证据分级 + §5 门禁表）
2. [north-star.md](../north-star.md) —— Stage A/B 边界与收敛回路
3. [ai-decompile-pipeline.md](../ai-decompile-pipeline.md) —— 完备性矩阵与首分歧点分类路由
4. [rom-disasm-equivalence.md](rom-disasm-equivalence.md) —— 为什么 m1disasm 是可信投影（含复现命令）
5. **本文** —— 为什么做、社区在哪、别人做到哪、体量多大、下一步跑哪个门禁
6. `python3 gates/cli.py status` 先摸清 SKIP / TOLERATED / 恒红项，再动手

### 引用可访问性说明（2026-11-16 实测）

- 本文 C 级链接均通过 `web_fetch` 抓取成功（printpage 版、wiki 页、GitHub raw 均返回正常内容）。
- 本环境 `curl` 直连上述站点一律 `403`（出口/反爬策略），**不代表链接失效**；判断"死链"请以浏览器或 `web_fetch` 为准。
- RHDN 已于 2024 转型新闻站、hack 库交 Internet Archive：`romhacking.net/hacks/<id>/` 是否可直接访问不稳定，**引用 hack 时用 id/名称检索，不要假设原 URL 永久有效**。

> 本文写于 2026-11-16。社区事实会过期：**任何引用前先验证链接仍可访问**；若本文结论被推翻，按 AGENTS.md §3.6 原地标注更正、不删历史主张。
