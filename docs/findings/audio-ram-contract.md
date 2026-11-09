# 音频引擎 RAM 契约 $0600-$06FF：C→A（2026-11-09）

来源线索：`docs/findings/community-sources-scan-2026-11-09.md` §3.5（C 级）——
音频引擎 RAM 区 $0600-$06FF 契约（sound engine 数据区）。
方法：静态谱（m1disasm constants_ram.asm `.enum $0600` + sound_engine.asm 读写点）×
动态证据（Mesen2 headless 运行 `rom/Metroid (USA).nes`，wram $0600-$06FF dump × 4 时刻）
交叉一致 → **A 级**。**零游戏逻辑改动、零 rename**（m1disasm 符号名仅作线索引用，
本项目命名台账不受影响；同址异名以静态谱登记为准）。

## 结论（verdict）：A 级（静态谱 + 动态 dump 交叉一致）

## 证据采集（动态，A 级）

- 载体：`tools/mesen2/Mesen`（2.1.1）headless + Xvfb :121，隔离 HOME
  `~/.config/Mesen2-metroid-home`，Lua harness
  `tools/oracle/accuracycoin/metroid_wram_dump.lua`（memType=8=NesMemory）。
  流程：标题画面 f=60-90 注入 Start → 开局（标题音乐播放中，sound engine 活跃）→
  f=600/900/1200/1500 各 dump 一次 $0600-$06FF。
- 串行锁：持有 `analysis/.scenario.lock`（flock LOCK_EX|LOCK_NB，fail-fast）；
  **运行前确认锁可获取、无其他 Mesen 实例；运行后确认无残留进程**（`ps aux | grep -i mesen`
  为空 ✓，锁已释放 ✓）。
- dump 产物：`analysis/audio_ram/metroid_wram_dump.log`（可再生工作区，不入库；
  再生成命令见文末）。

## 已证实（A 级：静态谱 + 动态交叉）

### 地址图（静态：`refs/m1disasm/SRC/constants_ram.asm:977` `.enum $0600`）

| 地址 | m1disasm 名（线索） | 语义 | 动态证据 |
|---|---|---|---|
| $0600/01 | MusicSQ1PeriodLow/High | SQ1 音高 period，写 SQ1_LO/HI | t600..t1500 持续变化（C4 09→AB AB→C4 09）✅ |
| $0602 | SFXPaused | 0=非暂停 | 恒 00（游戏中）✅ |
| $0604/05 | MusicSQ2PeriodLow/High | SQ2 period | 变化（75 0C→1C 09→EE 08）✅ |
| $0608/09 | MusicTriPeriodLow/High | Triangle period | 变化（F9 0A→3A 0A→DF 09）✅ |
| $0607 | LoadMusicSQ1SQ2PeriodsFlag | 1=需写 APU | dump 中 00（稳态）— 仅静态 |
| $0610-17 | SFXTri* | Triangle SFX period 处理（除法中间量） | 全 00（无 SFX）— 仅静态 |
| $061F | HasBeamSFX | bit7=long beam SFX / bit0=ice beam | 00 — 仅静态 |
| $0620-23 | Music*InstrLength | 4 声道当前指令帧数 | 变化并重载 ✅ |
| $0624-27 | Music*RepeatCounter | 乐曲循环计数 | 变化并循环回绕 ✅（实测 01 01 03 27 → 02 02 02 23 → 01 01 01 20 → 00 00 03 1D；【2026-11-09 r9 更正】原文「单调递减」与 dump 不符，实为每声道独立计数非单调） |
| $0628-2A | MusicSQ1/2DutyEnvelope, TriLinearCount | APU 帧数据 | 34 34 3C；$062A 3C→1C 变化 ✅ |
| $062B | NoteLengthTblOffset | 音长表偏移（初始化自 $BD31 表，sound_engine.asm:2040） | 0B 恒定 ✅ |
| $062C | MusicLoopsOnEnd | FF=曲终循环 | FF 恒定 ✅ |
| $0630-37 | Music*Base (dw) | 4 声道乐曲数据基址指针 | 00 B0 / 57 B0 / C1 B0 / 2B B1 稳定 ✅ |
| $0638-3B | Music*InstrID | 声道内曲序指针（相对 Base 索引） | 持续推进（03 0D 03 09 → 0E 27 24 04）✅ |
| $063C-3F | Music*RepeatID | 循环起始索引 | 恒定 06 1B 05 01（t900 起四时刻实测；【2026-11-09 r9 更正】原文证据串「01→06/1B/05/1F」对位混乱，系把四时刻各声道值错拼为跨时刻序列） |
| $0640-43 | Music*InstrDelay | **帧计数：倒数到 0 换新数据** | 非单调（03→27→1B→3F）= 重载行为 ✅ |
| $0648-4A | Music*Sweep | sweep 值 | 7F 7F 00 恒定 ✅ |
| $064B | ThisSoundChannel | 当前处理声道（00/04/08/0C） | 0C ✅ |
| $064D | CurrentSoundFlags | 当前 SFX/音乐 flags | 00（无 SFX 时）✅ |
| $0652-55 | *UsedBySFX | 声道占用 | 00 — 仅静态 |
| $065C-5E | ChannelType/MusicLoopFlags/MusicInitIndex | 处理状态/循环 flags/初始化索引 | 00 00 00 — 仅静态 |
| $0660-69 | SFX*Length/Frame | SFX 帧长与当前帧 | 全 00（无 SFX）— 仅静态 |
| $066A-6D | Music*VolumeIndex/Data | 音量包络索引与帧数据 | $066B 变化（07→0A）✅ |
| $066E-6F | SpareMem066E | 注释：被 `sta MusicSQ1VolumeData,x` 无意写入、从不读 | **dump 中 $066E 变化（00→3C→1C）—— 注释被动态证实** ✅ |
| $0670-7B | SFX*Var0/1/2 | SFX 附加参数 | 全 00 — 仅静态 |
| $0680-85 | SFX*/Music*InitFlags | 初始化 flags（入队） | 全 00 稳态 — 仅静态 |
| $0686 | ScrewAttack0686 | screw attack SFX 用 | 00 — 仅静态 |
| $0688-8D | SFX*/Music*ContFlags | 续传 flags | 全 00 — 仅静态 |

四时刻 diff 显示 27 个地址活跃（周期/帧计数/曲序指针/音量索引全部按静态语义运动；
【2026-11-09 r9 更正】原文 22 为程序化 diff 前的手工计数，程序化重 diff 实测 27）；
静态谱与动态行为**零冲突**。

### 已知分歧点交叉：$BBA8 帧计数语义

`docs/findings/adsr-fourth-source-crossref.md` 登记的分歧点 $BBA8（帧计数语义）：
本卡动态 dump 覆盖的是 $0600-$06FF 数据区本身；$BBA8 属 bank0 代码页（采样器侧），
本卡未采 trace 仲裁，分歧维持原档案登记（见遗留假设）。

## 假设区（不进结论）

1. $0607/$0652-55/$0660-7B/$0680-8D 等在本次采集场景（标题曲、无 SFX）恒 0，
   其语义仅静态（B 级：sound_engine.asm 读写语境）。如需升 A 须采「有 SFX」场景
   （如跳跃/开火时刻 dump）。
2. `HasBeamSFX` bit 语义、`SFXTriVar2`（screw attack 专用）等注释为 m1disasm 语义
   线索，未独立复现。
3. 采集仅 4 个采样点 @600Hz 间隔；不排除低频字段（如 MusicLoopsOnEnd 翻转）漏采。

## 复现

```bash
# 静态谱
grep -n '\$06' refs/m1disasm/SRC/sound_engine.asm
sed -n '977,1140p' refs/m1disasm/SRC/constants_ram.asm
# 动态 dump（持有 analysis/.scenario.lock；Mesen 单实例；跑前后查残留进程）
python3 tools/oracle/accuracycoin/run_accuracycoin.py --help  # 载体模式参考
# 本卡实际调用 = run_accuracycoin.py 的内联等价（ROM=Metroid (USA).nes，
# HOME=~/.config/Mesen2-metroid-home，Lua=tools/oracle/accuracycoin/metroid_wram_dump.lua）
ps aux | grep -i mesen   # 必须为空（已验证 ✓）
```

## 证据等级

- $0600-$068D 主干字段：A 级（静态谱 × 动态 dump 交叉一致）。
- 稳态零值字段：B 级（仅静态语境）。
- m1disasm 符号名仅作线索；同址异名并列登记于本表「m1disasm 名」列，不进命名台账。
