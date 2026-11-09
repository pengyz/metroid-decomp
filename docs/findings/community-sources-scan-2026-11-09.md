# 社区资料扫描档案（NES Metroid，2026-11-09）

> 来源：网络扫描（独立调研 subagent 产出，lead 归档）。
> **证据分级纪律**：本文按 AGENTS.md §3 分级——全部第三方结论默认**C 级线索**（D5：
> 第三方命名/结论不背书），仅下述已自证条目升 A/B。每条含升 A 路径。

## 1. 已自证升 A 级（本地机械复现，2026-11-09 实测）

### 1.1 Data Crystal ROM 锚点与我们 ROM 逐项一致 ✅
Data Crystal Metroid 页（https://datacrystal.tcrf.net/wiki/Metroid ）记载的 CRC/MD5 锚
与 `rom/Metroid (USA).nes` 本地实测（md5sum+python zlib.crc32/hashlib，2026-11-09）
逐项一致：

| 项 | Data Crystal | 本地实测 |
|---|---|---|
| MD5 | d77c8053168da14b360bf5caeccc5964 | d77c8053168da14b360bf5caeccc5964 ✓ |
| 整文件 CRC32 | A2C89CB9 | A2C89CB9 ✓ |
| headerless CRC32 | 70080810 | 70080810 ✓ |

（sha256 e6e6b701…/size 131088 另见 AGENTS.md 身份锚点表——同一文件双口径一致。）
复现命令：`md5sum rom/*.nes` + `python3 -c "import zlib,binascii; d=open('rom/Metroid (USA).nes','rb').read(); print(format(binascii.crc32(d)&0xffffffff,'08X'), format(zlib.crc32(d[16:])&0xffffffff,'08X'))"`

## 2. 可升 B 级（规范引用+已定义验证路径）

### 2.1 NMI/APU 帧周期不等（4017.txt，nesdev 考证存档）
https://sources.debian.org/src/fceux/2.3.0+dfsg1-3/documentation/tech/cpu/4017.txt/
——NMI 帧周期 29780⅔ cycles ≠ APU frame IRQ 周期 29829⅚ cycles（NTSC）。
这是 NES 硬件规范级常量（nesdev 共识，非本项目 ROM 结论），引用为 B 级：命中
D7「时序边界缺口」的帧相位面；若音频引擎按帧计数驱动（见 §3.5），两周期差影响
长音保真。**升 A 路径**：Mesen2 trace 实测连续 NMI 间隔周期数（工具战役已有 exec
断点能力，可挂 C0D9 计 cycle）。

### 2.2 nesdev APU 规范页族
https://www.nesdev.org/wiki/APU （channel/frame counter/length counter/sweep 子页）
——社区标准 APU 规范。BUG-090（音频）三选裁决（A 桩/B 最小合成/C 全 APU）的规范
基础面。**B 级**（规范引用）；任何「引擎是否写 $4008 长度计数器」等行为结论仍需
trace 自证后才进 spec。

## 3. C 级线索（未自证，按价值排序；升 A 路径各条标注）

### 3.1 adsr/nes-metroid（第四份反汇编）
https://github.com/adsr/nes-metroid ——6809 风格 `Lxxxx:` 标签+.alias 符号；密码编码
例程（FourLowerAndTwoUpper/SixLowerBits/SixUpperBits @ $8F46/$8F5A/$8F2D）与音频
例程（$BBD0–$BC04 音乐/噪声帧计数、InitMusicIndexTbl @ $BD31）注释最全。
**用途**：命名台账第四源交叉线索（与 m1disasm/MetroidMMC3/nmikstas 对照消歧——
四源一致可提为 B 级候选，rename 仍须逐条自证）。**升 A 路径**：clone 后按 bank:addr
与 ROM 字节逐条对读（等价性核验一次+逐符号语义自证）。

### 3.2 MetroidNESRecomp + nesrecomp（同游戏静态重编译先例）
https://github.com/mstan/MetroidNESRecomp ——6502→C 静态重编译，native PPU/APU 服务
层；v0.2.4 发布纪律：逐生成单元（124 C units）与生成器输出一致 + 720 帧重放比对 +
显式声明 unverified 范围。**用途**：验证方法论映射（≈我们的 seed 回归+golden/ratchet+
显式 TOLERATED）；证明本项目路线在本体上可行。机制结论不引用（其为 C 源非我们判定面）。

### 3.3 RustyNES TESTING.md + AccuracyCoin（oracle 可信度标准具）
https://github.com/doublegate/RustyNES/blob/main/docs/dev/TESTING.md ——AccuracyCoin
146/146 作为模拟器基准可信度独立检验；其「144/144 因 fail-path fall-through 虚高」
证伪更正实录。**用途**：①可跑一次 AccuracyCoin on Mesen2 = oracle 独立证据（低成本
高价值候选工具任务）；②fail-path 虚绿案例佐证「绿≠完整」（D7）。

### 3.4 密码系统独立复算素材
- Neoseeker 密码 FAQ（Sami Khouri, 2005）
  https://www.neoseeker.com/metroid/faqs/110763-password.html ——144 位=24 字符×6bit
  =128 数据+8 校验和（前 136 位累加）+8 shift byte。
- Retropass（https://pypi.org/project/retropass/ ）——多游戏密码生成库，自带机器可读
  密码格式 spec。
**升 A 路径**：按格式 spec 对 ROM 密码编码例程逐位对读 + 生成/校验往返脚本对拍。

### 3.5 音频引擎 RAM 契约（BUG-090 输入面）
Data Crystal Metroid RAM map（https://datacrystal.tcrf.net/wiki/Metroid/RAM_map ）
$0600 段：SQ1/SQ2/Tri Period、0620–0627 FrameCountInit/RepeatCounter、0614/0616
Triangle 百分比斜率——音频引擎=帧计数驱动（若自证）。条目含"not sure"类描述=仅线索。
**升 A 路径**：Mesen2 write 回调挂 $0600-$062F 段实录引擎写行为。

### 3.6 speedrun glitch 操作集（差分场景清单素材）
https://www.speedrun.com/metroid_nes/forums ——door jump/quickmorph/screen wrap 的
逐步操作描述。**用途**：每条 glitch = 一条 Mesen2 输入回放脚本+L4 截图点的候选场景
（D7 跨模块交互缺口的场景面）；机制解释仅线索，需 trace 到 door 碰撞例程自证。

### 3.7 nesdev RNG 帖（NES vs FDS RNG）
https://forums.nesdev.org/viewtopic.php?p=258432 ——作者指认 NES/FDS 随机性差异面
（seahorse/hopper/fireball）。**仅线索**（无字节级引用）；FDS「修复」方案按 D4/D6
与本项目无关，RNG 本体照搬 NES_NTSC；差异场景名可作 RNG 考证任务输入。

### 3.8 其他（低值留档）
- Mesen2 Lua harness gotcha 集（https://skillsmp.com/fr/creators/alondero/nestlin/claude-skills-mesen ）：runScript 绝对路径/getScriptDataFolder 无尾分隔符——对照 oracle/ 防同款坑。
- Mesen issue #259 savestate 约束 → 已落 docs/knowledge/gotcha-mesen-savestate-callback-constraint.md。
- FCEUX Lua 脚本集（registerexec PC-hook）——首分歧点定位的廉价补充工具面。
- TransistorKit NES harness（https://transistorkit.com/blog/how-transistorkits-nes-harness-works-an-emulator-as-a-truth-machine/ ）——「模拟器作为真值机」门禁文化同构参考。
- Hissho Tekunikku Kan Peki-ban 英译 PDF（http://thealmightyguru.com/Wiki/images/e/e6/Metroid_-_Hissho_Tekunikku_Kan_Peki-ban_(Translated).pdf ）——1986 攻略书，仅历史/设计意图佐证。
- jgettig 494P1 Unity Remaster（https://jgettig.github.io/494p1Metroid.html ）——第二方 spawn/掉落机制考证线索，可交叉验证。
- ArnonGuttel/metroid-nes-remake（Unity 课程重制）——反例素材：无差分验证→行为漂移，论证 D3 门禁必要性。

## 4. 与 m1disasm 冲突/互补总评
未发现与 m1disasm 逐字节等价性结论冲突的资料。互补点：① adsr 第四源符号交叉（§3.1）；
② 密码独立复算（§3.4）；③ 重编译先例验证模式（§3.2）；④ oracle 可信度标准具（§3.3）。

## 5. 待裁任务候选（低成本→高成本）
1. AccuracyCoin on Mesen2（oracle 独立证据，半天级）
2. adsr 反汇编 clone+等价性核验+符号交叉台账（一天级）
3. savestate 约束最小踩实（知识库条目验证，小时级）
4. 密码格式逐位自证+往返对拍（一天级）
5. RNG/glitch 场景清单→差分用例批生成（依托 Mesen 工具战役既有 DSL）
