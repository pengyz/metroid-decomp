# findings：声音域缺口画像与端口工作单（G3 预备件）

> 状态：**缺口画像完成**（2026-10-24，A 级双腿对账）；端口未开工。本文=声音域的
> 字节级缺口清单 + 波次计划 + 验收判据。渲染残差见
> [palette-chain-f180-residual.md](palette-chain-f180-residual.md)（独立线）。

## 1. 缺口画像（A 级：L3 双腿对齐窗 shift=2 对账）

> 【2026-10-24 复核更正 F4】原「恒差 77B」**口径错误**：77=声域 **56B**+非声块 21B
> （RetrigDelay $18/$19=2+栈页 $01EB-$01FF=19——_legcmp 重算自证）。声域口径=**56B**
> （zp 5+$0600-$068D 51）；下方块表合计 57±1=逐块边界待细核（以 _legcmp 逐址清单为准）。
> **验收**改为「声域 56B→0；恒差集余 21B（Retrig+栈页）」——原「77B→0」与同节排除注
> 自相矛盾（永不可达）。

恒差（声域 **56B**）、形状 f30/f120/f299 **完全一致**（时不变）——我方全零 = 声音引擎从未运行：

| 块 | 差/长 | 真值样例（O/G，f120） | 语义（constants_ram 锚） |
|---|---|---|---|
| `$E6/$E7` | 2/2 | `15/B1` / `00/00` | MusicChannelBase（音乐流游标指针） |
| `$EA` | 1/1 | `05` / `00` | Cntrl0Data（$4000 APU 控制影子） |
| `$EC/$ED` | 2/2 | `03/BD` / `00/00` | VolumeEnvelopePtr（音量包络指针） |
| `$0600-$0609` | 6/10 | `F3,0D…` | MusicSQ1/2 Period/Sweep/Lo（方波周期） |
| `$060A-$0613` | **0/10** | 双零 ✓ | MusicSQ Length/VolumeEnv（本窗静默） |
| `$0614-$061F` | **0/12** | 双零 ✓ | MusicTRI/Noise 族（本窗静默） |
| `$0620-$0631` | 17/18 | `54…` | 包络指针/状态族 |
| `$0632-$0643` | 17/18 | — | 音乐临时/指针族 |
| `$0644-$0653` | 4/16 | — | Tempo/NoteCount 族 |
| `$0654-$0663` | 1/16 | — | TRI/Noise tempo 族 |
| `$0664-$066D` | 5/10 | — | SFX ContFlags 族 |
| `$066E-$068D` | 2/32 | `$68D=10` | 音乐/SFX 队列余段 |

复现：`python analysis/_soundgap.py`（双腿 dump 在位）。

## 2. 工程面定性

- **形态=2A03 主旋律引擎运行态**（intro theme：方波 1/2 + 包络 + tempo 计数；
  TRI/Noise 本窗静默——与"0614-061F 双零"互证）。
- 我方 nmi.rs `sound_engine` = 桩（四 init 子件 flags=0 短路 + APU 帧配置）——
  **音乐路（RunSoundRoutine→MusicRoutine→MusicLoop）整体未端口**。
- 按总案 G3=深度端口 ~35 件 5 波次（声音域独立于渲染线）；F1 复核更正已定性
  （`$E6-$EE`=SoundEngine 态非星态）。

## 3. 波次计划（建议）

| 波 | 件 | 目标 |
|---|---|---|
| W1 | SoundEngine 门/静音/暂停分支 + RunSoundRoutine 分派核 + init 旗语义 | 分派骨架接通（非零旗不炸） |
| W2 | MusicRoutine 首波：MusicChannelBase 流游标 + Cntrl0Data 影子 + tempo 计数（$0644-0653） | `$E6/$E7/$EA`+tempo 块归零 |
| W3 | 音量包络机（$EC/$ED + $0620-$0643） | 包络两块归零 |
| W4 | 方波周期写径（$0600-$0609）+ SQ Length/Volume | 周期块归零 |
| W5 | SFX 族（$0664-$066D/$066E-$068D）+ 声道尾件 | 全块归零；L3 双腿声音域全同 |

（TRI/Noise 两块本窗双零=天然对拍基线，波次中作回归哨兵。）

## 3.5 W3 实施方案（r20——考证毕，四步走）

> 考证产出（A 级：sound_engine.asm 逐字）：UpdateAllMusicChannels($BAA5)→
> UpdateMusicChannel($BAC2)→LoadNextMusicChannelInstr($BADC)→_Continued($BB1C)→
> SetMusicInstrDelayToLength($BBA8) 全径语义+两数据表定位（MusicNotesTbl $BE77 64×2B /
> NoteLengthsTbl $BEF7 三组）。**语义要点**：①RepeatID 存 **inc 后** InstrID（BB04——
> 现端口件注「inc 前」疑 off-by-one，W3-2 TDD 钉）②音符字节=MusicNotesTbl 索引（低字节
> 0=休止）③长度字节（A&$B0==$B0）：(A&$0F)+NoteLengthTblOffset→NoteLengthsTbl ④Tri 通道
> 特例=UpdateMusicTriLinearCount ⑤Noise 通道特例=鼓点 SFXData ⑥包络索引≠0 ⇒ 跳直接音量写
> （包络机接管=$EA 动态面）。

| 步 | 内容 | 验收 |
|---|---|---|
| W3-1 | ✅ **表烘焙**（fe3560a7）：MusicNotesTbl/NoteLengthsTbl ROM 直读+--check+锚测试 | 19 测绿 |
| W3-2 | **单通道 UpdateMusicChannel 闭环**：VM 步进（RepeatID 修）+_Continued 音符载+period/sweep/音量写面 | 单通道 TDD 全绿（对照 asm 六要点） |
| W3-3 | **四通道环编排**（$BAA5 入口+IncrementToNextMusicChannel） | 4 通道步进序 TDD |
| W3-4 | **$EA 包络动态**（包络机接管面+L3 对拍） | _soundgap 51B→0（声域）；恒差集余 21B |

**W3-2 地址规格**（constants_ram 逐行审计——X=乐通道 0..3 / Y=ThisSoundChannel 0/4/8/C）：
`$0620+X` InstrLength｜`$0640+X` InstrDelay｜`$0624+X` RepeatCounter｜`$063C+X` RepeatID｜
`$0638+X` InstrID｜`$0630+2X` Base（$E6/$E7 载入源）｜`$0628+X` DutyEnvelope（X=2 时=TriLinearCount
位——ragged 面随证）｜`$0648+X` Sweep（X≤2）｜`$062E+X` VolumeEnvelopeIndex｜`$066C+X` VolumeData｜
`$0653+X` UsedBySFX｜`$062B` NoteLengthTblOffset｜`$062D` TriCounterCntrl｜`$062A` TriLinearCount｜
`$064B` ThisSoundChannel｜`$EA` Cntrl0Data｜APU 面 `SQ1_VOL/LO/HI/SWEEP + Y`。
坑注：$066E 被 `sta MusicSQ1VolumeData,x` 无知写及（asm 注自证）——D4 保真随件。

## 4. 验收判据

1. `cargo test --workspace` 全绿 + 新件 TDD（每件 asm 锚注）；
2. **L3 双腿对齐窗恒差清单收缩**：声域 **56B** → 逐波下降 → 0（复现：
   `python analysis/_legcmp.py`——**可达目标**=恒差集仅余 21B：`$18/$19` RetrigDelay+
   栈页 `$01EB-$01FF`（复核 F4 更正口径）；
3. `python gates/cli.py trace` PASS；b1-b10 场景回归不退；
4. （G3 扩展）音频输出对拍：APU 寄存器写序 trace 对拍先行，WAV 听感对拍后置。

## 5. 证据指针

- 双腿 dump：`analysis/traces/boot-title.frames.jsonl`（oracle）/
  `analysis/dump/boot-title.frames.jsonl`（game-core）；
- 画像脚本：`analysis/_soundgap.py`；asm：`refs/m1disasm/SRC/sound_engine.asm`
  （`SoundEngine: $B3B4` 起）+ `constants_ram.asm`（$E6-$ED/$0600-$068D 命名锚）；
- 现桩：`crates/game-core/src/frame/nmi.rs::sound_engine`（四 init 子件短路面）。
