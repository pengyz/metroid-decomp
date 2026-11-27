# 音频/音乐引擎规格（调用链、数据表与移植契约）

> 任务：#126。证据分级：**A** = ROM 字节/汇编逐行可复现；**B** = 调用图/数据流推断；**C** = 假设（本章 §9 单列，附推翻方法）。
> 唯一引擎源：`refs/m1disasm/SRC/sound_engine.asm`（2,721 行）。所有 `bank:addr` 均为 bank1 映射下的 CPU 地址；bank 副本见 §1.3。

## §0 结论摘要

1. 声引擎是**一份独立子程序**，`SoundEngine` 入口 `sound_engine.asm:384` = `$B3B4`，由 **NMI** 每帧调用（`prg7_engine.asm:272` `jsr SoundEngine`）；`$1A`（NMIStatus）≠0 时整段 NMI 跳过，音乐也随之少走一帧。
2. 引擎内部是**十个 cycle 的固定顺序扫描**：Noise init → Noise cont → Multi init → Multi cont → SQ1 init → SQ1 cont → Tri init → Tri cont → Music loop → Music init，最后 fallthrough 清 6 个 init 旗（`$B3D6 ClearInitFlags`）。顺序表在 `sound_engine.asm:360-370` 的注释里逐字节列出。
3. **SFX 优先于音乐**：某通道被 SFX 占用时，音乐载音符与音量包络都跳过写 APU 寄存器；SFX 结束后由 `LoadMusicSQ1SQ2Periods $BA08` 从 `$0600-$0605` 恢复周期。
4. **卡面更正**：卡面写「RAM 面 `$07xx` 音乐状态」与 ROM 不符。音乐/SFX 状态全在 **`$0600-$068D`**（`constants_ram.asm:999-1163` 的 `.enum $0600`，142 B）；`$07xx` 是 `$0700` 对象池与 `$0780-$07EF` VRAM buffer。Rust 已有的 `sound_ram_face`（`BASE=0x0600`/`FACE_END=0x068E`）与 ROM 一致。
5. 12 首歌的数据按 bank 分发：Tourian/ItemRoom/FadeIn/PowerUp **每份副本都带**，其余按区域 bank 携带（§3.3 表）。**代码面已有部分端口**（`crates/game-core/src/sound/`），缺的是「APU 合成/混音」最后一环（`crates/game-platform/src/apu.rs` 的 `AudioSink` 只收寄存器写流）。

---

## §1 调用链

### 1.1 每帧次序（A）

```
NMI (07:C0D9, bank7 固定窗口)
  ├ lda NMIStatus / bne @endIf_A        ; 上一帧没跑完 → 跳过整段（含读键与音乐）
  ├ jsr ReadJoyPads                     ; 07:C215
  └ jsr SoundEngine                     ; 07:C22E 之后，prg7_engine.asm:272
```

- 证据：`prg7_engine.asm:235`（`NMI:` 标签）、`:268`（`jsr ReadJoyPads`）、`:272`（`jsr SoundEngine`）。
- 与 #42 的时序件一致：音乐被 NMI 帧边界驱动，**不是**主循环工具函数，移植时不能放到 frame tick 的任意位置。

### 1.2 引擎内十 cycle（A）

来源：`sound_engine.asm:375-377`（十 cycle 注释逐字节列出旗序）+ `SoundEngine` 主体 `:384-410`。

| # | 扫描的旗 | 旗地址 | 进入的例程 | 说明 |
|---|---|---|---|---|
| 1 | `SFXNoiseInitFlags` | `$0680` | `RunSFXNoiseInitRoutine $B31B` | bit0 是**静音音乐**专用（先判，命中直接跳 `SoundEngine_SilenceMusic`） |
| 2 | `SFXNoiseContFlags` | `$0688` | `RunSFXNoiseContRoutine $B322` | 噪声持续音 |
| 3 | `SFXMultiInitFlags` | `$0684` | `RunSFXMultiInitRoutine $B34B` | 多声道 SFX + 4 首「非区域曲」（FadeIn/PowerUp/End/Intro） |
| 4 | `SFXMultiContFlags` | `$068C` | `RunSFXMultiContRoutine $B35C` | |
| 5 | `SFXSQ1InitFlags` | `$0681` | `RunSFXSQ1InitRoutine $B329` | |
| 6 | `SFXSQ1ContFlags` | `$0689` | `RunSFXSQ1ContRoutine $B330` | |
| 7 | `SFXTriInitFlags` | `$0683` | `RunSFXTriInitRoutine $B33D` | |
| 8 | `SFXTriContFlags` | `$068B` | `RunSFXTriContRoutine $B344` | |
| 9 | `MusicLoopFlags` | `$065D` | `RunMusicLoopRoutine $BC36` | 曲终循环 |
| 10 | `MusicInitFlags` | `$0685` | `RunMusicInitRoutine $BC3D` | 起播/切歌 |
| — | （继续）| `MusicContFlags` | `ContinueMusic $BC4B` | 第 11 段：没有任何 music 旗时 rts，否则 `UpdateAllMusicChannels $BAA5` |

- 每个 init 例程处理完**尾跳**到对应 cont 例程（如 `RunSFXNoiseInitRoutine` → `RunSFXNoiseContRoutine`，`sound_engine.asm:220-227`），cont 若无旗则 `jmp (SoundRoutinePtr)` 回 `GetSoundRoutine@RTS`，整条链自然继续到下一 cycle。
- 十个 cycle 走完：`ClearInitFlags $B3D6` 把 `$0680-$0685` 六旗清零（A：`:407-416`）。
- 暂停分支优先于所有 cycle：`MainRoutine == _id_PauseMode` ⇒ `SoundEngine_GameIsPaused $B399`（`:345-363`）；见 §2.4。

### 1.3 bank 驻留模型（A，C1 为假设）

- `.include "sound_engine.asm"` 出现在 **6 个 bank**：`prg0_title.asm:5238`、`prg1_brinstar.asm:1063`、`prg2_norfair.asm:843`、`prg3_tourian.asm:2410`、`prg4_kraid.asm:924`、`prg5_ridley.asm:820`。bank6（图形）与 bank7（固定引擎）**没有**副本。
- 因为 NMI 在 bank7 固定段，`$B3B4` 只有在当前驻留 bank 的窗口里有代码；换 bank 时 NMI 会读到别的 bank 的同一地址，除非引擎只在这些 bank 间切换。**C1**：bank6 驻留期间不调用音乐（图形解压窗口短、且解压期一般关 NMI），需要用 trace 证；见 §9。
- 每首曲的**流数据**只跟随拥有它的 bank（§3.3），所以「当前 bank 没有这首曲的数据」时，`SongHeaders` 里的指针是占位 `$0100/$0300/$0500/$0700`（指向 RAM）。原版靠「区域曲只在对应区域播」避免踩到，移植若允许任意切歌就必须保真这段（读占位地址 = 读 RAM 垃圾）；见 §9 C2。

### 1.4 请求面（A）

音乐请求（写 `MusicInitFlags $0685`）：

- `StartMusic 07:D92C`（`prg7_engine.asm:5061`）：
  `lda AreaMusicFlag`（bank1 `01:95CD=$01`、bank2 `02:95CD=$08`、bank3=$40、bank4=$10、bank5=$80）→ 若 `ItemRoomMusicStatus` 非 0 则 `ora #music_ItemRoom($20)` → `ora MusicInitFlags` → `sta MusicInitFlags`。
- 4 个调用点（`grep -n "jsr StartMusic" prg7_engine.asm`）：`:1232`（离开 PowerUp/密码导入流程前）、`:2201`（进区淡入完成）、`:4114`（清屏后按 ItemRoomMusicStatus 重启）、`:5034`（电梯换区完成）。
- 静音：`SilenceMusic 07:CB8E`（`prg7_engine.asm:2570`）置 `sfxNoise_SilenceMusic = $01`；引擎看到 bit0 立刻 `InitializeSoundAddresses $B404` 并清旗。
- 暂停：`PauseMusic 07:CB92`（`:2574`）置 `sfxNoise_PauseMusic = $02`；由 `$B3B4` 开头的 `lda SFXNoiseInitFlags / lsr / bcs` 命中，效果同「静音音乐」但只清当前发声（见 §2.4 与 §6.3）。

SFX 请求（写 `$0680-$0684/$0688-$068C` 的某一位）：

- 一族 `SFX_*` 入口在 `prg7_engine.asm:2578-2680`（`07:CB96`–`07:CC0D`），统一形制 `lda #sfxXxxFlag / ora <对应 InitFlags> / sta`（`SetSoundInitFlag 07:CBE8`、`SetMusicInitFlag 07:CC09`）。
- 位旗总表（`constants.asm:145-179`，A）：

| 通道（init 表） | 位7 | 位6 | 位5 | 位4 | 位3 | 位2 | 位1 | 位0 |
|---|---|---|---|---|---|---|---|---|
| Noise（7 位有效） | 未用 | ScrewAttack | MissileLaunch | BombExplode | SamusWalk | SpitFlame | PauseMusic | SilenceMusic |
| SQ1 | MissilePickup | EnergyPickup | Metal | BulletFire | OutOfPipe | EnemyHit | SamusJump | WaveFire |
| SQ2 | **未定义任何旗**（表存在但全 `@RTS`） | | | | | | | |
| Tri | SamusDie | Door | MetroidHit | StatueRaise | Beep | BigEnemyHit | SamusBall | BombLaunch |
| Multi | IntroMusic | PowerUp | EndMusic | IntroMusic?（见注） | — | SamusHit | BossHit | IncorrectPassword |

> Multi 位表原文：`sfxMulti_Intro=80 / PowerUp=40 / EndMusic=20 / IntroMusic=10 / （08 未用）/ SamusHit=04 / BossHit=02 / IncorrectPassword=01`（`constants.asm:172-179`）。上表「位4」按原文是 `IntroMusic`，与位7 同名不同位，属原版命名（保真照抄）。

---

## §2 分派与优先级

### 2.1 `GetSoundRoutine`：8 位 MSB 优先扫描（A）

`sound_engine.asm:561`（`$B4BD`）：

1. `sta CurrentSoundFlags`（`$064D`），`stx GetSoundRoutineDataPtr`（`$E4/E5`）。
2. 取 `GetSoundRoutineData` 该 cycle 的 5 B：4 B 表指针（2 张）+ 第 5 B → `ChannelType`（`$065C`）。
3. `asl CurrentSoundFlags / bcs @SoundFlagFound`：**从 bit7 往 bit0 扫**；`iny/iny` 每轮步 2；8 轮（`tya/cmp #$10`）。
4. 命中：`lda (SoundRoutineTablePtr),y` / `iny` / 高位 → `SoundRoutinePtr`（`$E2/E3`），`jmp (SoundRoutinePtr)`。
5. 未命中：`pla/sta CurrentSoundFlags` 恢复，回到 chain。

⇒ 同一帧同通道多旗时，**高位优先**（如 MissilePickup(bit7) 压过 WaveFire(bit0)）。

### 2.2 `GetSoundRoutineData` 10 项（A，`sound_engine.asm:106-128`，`$B289`）

| 项 | 地址 | init 表 | 下一 cycle | ChannelType |
|---|---|---|---|---|
| SFXNoiseInit | `$B289` | `SFXNoiseInitRoutineTbl $B2BB` | `RunSFXNoiseContRoutine` | 0 |
| SFXNoiseCont | `$B28E` | `SFXNoiseContRoutineTbl $B2CB` | `GetSoundRoutine@RTS` | 0 |
| SFXSQ1Init | `$B293` | `SFXSQ1InitRoutineTbl $B2DB` | `RunSFXSQ1ContRoutine` | 1 |
| SFXSQ1Cont | `$B298` | `SFXSQ1ContRoutineTbl $B2EB` | `GetSoundRoutine@RTS` | 1 |
| SFXTriInit | `$B29D` | `SFXTriInitRoutineTbl $B2FB` | `RunSFXTriContRoutine` | 3 |
| SFXTriCont | `$B2A2` | `SFXTriContRoutineTbl $B30B` | `GetSoundRoutine@RTS` | 3 |
| SFXMultiInit | `$B2A7` | `SFXMultiInitRoutineTbl $BC06` | `RunSFXMultiContRoutine` | 4 |
| SFXMultiCont | `$B2AC` | `SFXMultiContRoutineTbl $BC16` | `GotoRunSFXSQ1InitRoutine` | 4 |
| MusicInit | `$B2B1` | `MusicRoutineTbl $BC26` | `ContinueMusic $BC4B` | 0 |
| MusicLoop | `$B2B6` | `MusicRoutineTbl $BC26` | `RunMusicInitRoutine` | 0 |

`ChannelType` 语义：0=Noise / 1=SQ1 / 2=SQ2 / 3=Tri / 4=Multi（`sound_engine.asm:534-535` 注释）。音乐侧通道号另有一套：SQ1=0/SQ2=1/Tri=2/Noise=3（`:366-369`）——两套编号不可混用。

### 2.3 SFX 与音乐的冲突规则（A）

1. **占用标记**：SFX 起播时 `InitSFXData $B452` 在 `@Common` 执行 `txa / sta NoiseUsedBySFX,x`（`sound_engine.asm:517`），即 `UsedBySFX[ChannelType] = ChannelType`（1=SQ1/2=SQ2/3=Tri），Noise 对应 `$0652`、SQ1 `$0653`、SQ2 `$0654`、Tri `$0655`。
2. **音乐载音符时让路**：`LoadNextMusicChannelInstr_Continued` 写寄存器前 `txa / dec SQ1UsedBySFX,x / cmp SQ1UsedBySFX,x / beq MusicChannelIsUsedBySFX`（`sound_engine.asm:1944-1946`）。因为计数存的是「通道号+1」，被占用时 `dec` 后恰好等于当前通道号 ⇒ 跳 `MusicChannelIsUsedBySFX $BBB1`：`inc` 回原值并 `jmp SetMusicInstrDelayToLength`，**只推进帧计数、不写 `SQ1_VOL/LO/HI/SWEEP`**。
3. **音量包络也检查**：`UpdateVolumeEnvelope $BA41` 在写 `SQ_VOL` 前 `lda SQ1UsedBySFX,x / bne @RTS`（`:1711`），占用时不写音量。
4. **SFX 结束恢复音乐**：各 `End*SFX`（如 `EndSQ1SFX $B6F2`）写 `SQ1_VOL=$10`、清 `UsedBySFX` 与对应 cont 旗、`inc LoadMusicSQ1SQ2PeriodsFlag $0607`；下一帧 `UpdateVolumeEnvelope` 开头 `jsr LoadMusicSQ1SQ2Periods $BA08` 从 `$0600-$0605` 恢复周期+sweep 再继续。
5. **切歌/重播**：`CheckMusicFlags $B3FC`（`InitializeMusic $BF19` 开头）比较 `MusicContFlags $068D` 与 `CurrentSoundFlags`：相同 ⇒ 仅 `ClearSpecialAddresses $B40E`（清 Tri 计数/暂停/loop 旗），不同 ⇒ `InitializeSoundAddresses $B404` 全清后重载。
6. **曲终**：`EndOrLoopMusic $B3F0`——`MusicLoopsOnEnd $062C == 0` ⇒ 全清；非 0 ⇒ `MusicLoopFlags ← MusicContFlags`，下一帧由 cycle 9 重播（`RunMusicLoopRoutine`）。
7. **静音优先级**：`SFXNoiseInitFlags` bit0 在**任何 cycle 之前**判（`:396-398`），所以静音请求同帧压过所有 SFX/音乐。

### 2.4 暂停（A）

`sound_engine.asm:345-363`：
- 进暂停首帧：`PauseSFX $B38F` → `inc SFXPaused`（`$0602`），`jsr ClearSounds`，`PauseSFXStatus=0`。
- 之后 `$12`(18) 帧内：`PauseSFXStatus & 3 == 3`（每 4 帧）用 `LoadSFXData@SQ1` 重放 `GamePaused`（`SFXData@GamePaused = $B20D`），直到 `PauseSFXStatus==$12` 停止。
- 恢复游戏后主路径把 `SFXPaused` 清零（`:405-406`），音乐不会自动重启——由外部 `StartMusic` 再请求。

---

## §3 数据表

### 3.1 `SFXData $B200`（A，`sound_engine.asm:25-104`）

布局：`[0]` 基址 0；`[1..12]` 4 组鼓点、每组 3 B（Noise 轨专用，见 §4.3）；`[0x0D..0x85]` **31 条 SFX，每条 4 B**。

| SFX | 地址 | 4 B | 对应旗 |
|---|---|---|---|
| GamePaused | `$B20D` | `80 7F 80 48` | 暂停专用（非旗） |
| ScrewAttack | `$B211` | `35 7F 00 B0` | Noise bit6 |
| MissileLaunch | `$B215` | `19 7F 0E A0` | Noise bit5 |
| BombExplode | `$B219` | `0D 7F 0F 08` | Noise bit4 |
| SamusWalk | `$B21D` | `16 7F 0B 18` | Noise bit3 |
| SpitFlame | `$B221` | `13 7F 0E F8` | Noise bit2 |
| SamusHitSQ1SQ2 | `$B225` | `C1 89 02 0F` | Multi bit2 |
| BossHitSQ2 | `$B229` | `34 BA E0 05` | Multi bit1（SQ2 侧） |
| BossHitSQ1 | `$B22D` | `34 BB CE 05` | Multi bit1（SQ1 侧） |
| IncorrectPasswordSQ1 | `$B231` | `B6 7F 00 C2` | Multi bit0（SQ1 侧） |
| IncorrectPasswordSQ2 | `$B235` | `B6 7F 04 C2` | Multi bit0（SQ2 侧） |
| TimeBombTick | `$B239` | `17 7F 66 89` | SFX_TimeBombTick |
| EnergyPickup | `$B23D` | `89 7F 67 18` | SQ1 bit6 |
| MissilePickup | `$B241` | `8B 7F FD 28` | SQ1 bit7 |
| Metal | `$B245` | `02 7F A8 F8` | SQ1 bit5 |
| LongRangeShot | `$B249` | `D7 83 58 F8` | SQ1 bit4（变体） |
| ShortRangeShot | `$B24D` | `D6 82 58 F8` | SQ1 bit4（变体） |
| SamusJump | `$B251` | `95 8C 40 B9` | SQ1 bit1 |
| EnemyHit | `$B255` | `1D 9A 20 8F` | SQ1 bit2 |
| OutOfPipe | `$B259` | `16 8D E0 42` | SQ1 bit3 |
| WaveBeam | `$B25D` | `19 7F 6F 40` | SQ1 bit0 |
| IceBeam | `$B261` | `18 7F 80 40` | SQ1 bit4（HasBeamSFX 变体） |
| BombLaunch1 | `$B265` | `07 7F 40 28` | Tri bit0（滚球中） |
| BombLaunch2 | `$B269` | `07 7F 45 28` | Tri bit0（滚球外） |
| SamusBall | `$B26D` | `7F 7F DD 3B` | Tri bit1 |
| MetroidHit | `$B271` | `7F 7F FF 98` | Tri bit5 |
| SamusDie | `$B275` | `7F 7F 40 08` | Tri bit7 |
| Beep | `$B279` | `09 7F 30 48` | Tri bit3 |
| BigEnemyHit | `$B27D` | `03 7F 42 18` | Tri bit2 |
| StatueRaise | `$B281` | `03 7F 11 09` | Tri bit4 |
| Door | `$B285` | `7F 7F 30 B2` | Tri bit6 |

4 B 的写法（`LoadSFXData $B368` / `@loop_LoadSFXRegisters $B384`）：按通道把 4 B 依次写进该通道 4 个 APU 寄存器（SQ1 `$4000-$4003`、SQ2 `$4004-$4007`、Tri `$4008-$400B`、Noise `$400C-$400F`）。

### 3.2 8 张 SFX handler 表 + 1 张音乐分派表（A）

| 表 | 地址 | 条目（位7→位0） |
|---|---|---|
| `SFXNoiseInitRoutineTbl` | `$B2BB` | 无、ScrewAttack、MissileLaunch、BombExplode、SamusWalk、SpitFlame、无、无 |
| `SFXNoiseContRoutineTbl` | `$B2CB` | 无、ScrewAttack、MissileLaunch、NoiseSFXCont、NoiseSFXCont、SpitFlameCont、无、无 |
| `SFXSQ1InitRoutineTbl` | `$B2DB` | MissilePickup、EnergyPickup、Metal、BulletFire、OutOfPipe、EnemyHit、SamusJump、WaveBeam |
| `SFXSQ1ContRoutineTbl` | `$B2EB` | MissilePickupCont、EnergyPickupCont、SQ1SFXCont、BulletFireCont、SQ1SFXCont、SQ1SFXCont、SQ1SFXCont、WaveBeamCont |
| `SFXTriInitRoutineTbl` | `$B2FB` | SamusDie、DoorOpenClose、MetroidHit、StatueRaise、Beep、BigEnemyHit、SamusToBall、BombLaunch |
| `SFXTriContRoutineTbl` | `$B30B` | SamusDieCont、DoorOpenCloseCont、MetroidHitCont、StatueRaiseCont、BeepCont、BigEnemyHitCont、SamusToBallCont、BombLaunchCont |
| `SFXMultiInitRoutineTbl` | `$BC06` | FadeIn(music03)、PowerUp(music01)、EndGame(music05)、IntroMusic(music01)、无、SamusHit、BossHit、IncorrectPassword |
| `SFXMultiContRoutineTbl` | `$BC16` | 无×5、SamusHitCont、BossHitCont、IncorrectPasswordCont |
| `MusicRoutineTbl` | `$BC26` | Ridley→Music04、Tourian→Music00、ItemRoom→Music00、Kraid→Music00、Norfair→Music03、Escape→Music02、MotherBrain→Music00、Brinstar→Music03 |

音乐 init 的 duty/音量初值（`sound_engine.asm:2180-2214`）：`Music00Init=$92/$96`、`Music01Init=$B6/$F6`、`Music02Init=$F4/$F4`、`Music03Init=$34/$34`、`Music04Init=$B3/$B3`、`Music05Init=$F5/$F6`（X=SQ1、Y=SQ2）。`SetVolumeAndDisableSweep $B9E4` 同时把 `SQ1/SQ2 Sweep` 置 `$7F`（关 sweep）。

### 3.3 音乐头 `SongHeaders $BD31`（A）

13 B 布局（`macros.asm:163` `SongHeader` 宏；`InitializeMusic $BF19` 原样拷到 `$062B-$0637`）：

| 偏移 | 目标 RAM | 字段 |
|---|---|---|
| +0 | `$062B` | `NoteLengthTblOffset`（`NoteLengthsTbl` 组低字节，见 §3.5） |
| +1 | `$062C` | `MusicLoopsOnEnd`（`$FF`=循环，`$00`=曲终停） |
| +2 | `$062D` | `MusicTriCounterCntrl`（高低半字节控制 Tri 长度，见 §4.4） |
| +3 | `$062E` | SQ1 音量包络索引 |
| +4 | `$062F` | SQ2 音量包络索引 |
| +5..+12 | `$0630-$0637` | 4 条通道流指针（SQ1/SQ2/Tri/Noise，`.word`） |

`SongHeaderOffsetTable $BBFA`：12 B，**按音乐 id 索引**，值是「头表字节偏移」（都是 13 的倍数）：`[65,143,52,39,26,13,0,130,104,117,78,91]` ⇒ id0(Ridley)→slot5、id6(MB)→slot0、id7(Brinstar)→slot10、id0A(End)→slot6、id0B(Intro)→slot7。

**每 bank 实有曲目**（A，用 `android` 判据：4 条流指针都 ≥ `$8000`；bank0 用同一判据取 `$AC00-$AE8E`）：

| bank | 实有曲目（判据见 §8 脚本：4 条指针逐条为 0 或 ≥ `$8000`） |
|---|---|
| 0 | Tourian、ItemRoom、FadeIn、PowerUp、End、Intro |
| 1 | Tourian、ItemRoom、Brinstar、FadeIn、PowerUp |
| 2 | Tourian、ItemRoom、Norfair、FadeIn、PowerUp |
| 3 | Tourian、ItemRoom、Escape、MotherBrain、FadeIn、PowerUp |
| 4 | Ridley、Tourian、ItemRoom、Kraid、FadeIn、PowerUp |
| 5 | Ridley、Tourian、ItemRoom、Kraid、FadeIn、PowerUp |

非本 bank 的曲目头里，4 条指针是 `$0100/$0300/$0500` 与 `$0700`（Noise 轨常为 `$0000`）这类**指向 RAM 的占位值**（实测 bank1 的 Ridley 头 = `0B FF F0 01 01 00 01 00 03 00 05 00 00`，第 4 指针为 0），播它会读 RAM 当乐谱（原版靠「区域曲只在本区播」规避）。

### 3.4 `MusicNotesTbl $BE77`（A）

64 项 ×2 B。index0 = A1（`07 F0`，55 Hz 附近），index0x7E = F7（`00 27`）。**频率公式**：`f = 1790000 / 16 / (period + 1)`（PAL 用 1663000，注释 `:2474`）。写入时 `byte1 → 周期低`、`byte0 | $08 → 周期高`（保最低 3 位为 1 是 NES 要求的最小周期长度）。

### 3.5 `NoteLengthsTbl $BEF7`（A）

三组各 12 B，组偏移写进音乐头 +0 的低字节（`@4=$04` → 数组从 `$BEFB` 起按 `索引+4` 读）：

| 组 | 偏移 | 地址 | 12 B |
|---|---|---|---|
| @4 | `$04` | `$BEF7` | `04 08 10 20 40 18 30 0C 0B 05 02 06` |
| @6 | `$06` | `$BEF7` | 同上表的 `$BEFB` 起按 +6 读（`10 20 40 18…`） |
| @7 | `$07` | `$BEF7` | 从 `$BEFC` 起 |

语义：音符指令 `$B0-$BF` 的低 4 位是索引，`NoteLengthTblOffset + index` 取表项 = 帧数（注释按 64 fps 估）。

### 3.6 音量包络（A，`sound_engine.asm:2216-2260`）

`VolumeEnvelopePtrTable $BCB0` 5 指针：`$BCBA/$BCC5/$BCCF/$BCDA/$BD03`；包络以 `$FF`（保持当前音量）或 `$F0`（关到 `$10`）结束。逐帧 `UpdateVolumeEnvelope $BA41` 写 `SQ1_VOL`/`SQ2_VOL`，duty 取自 `MusicSQ1DutyEnvelope $0628`（高 4 位）与包络值（低 4 位）拼成。

---

## §4 音乐流编码与帧行为

### 4.1 指令字节（A，`LoadNextMusicChannelInstr_Continued $BB1C`）

| 字节 | 含义 |
|---|---|
| `$00` | 曲终（`GotoEndOrLoopMusic`→`EndOrLoopMusic $B3F0`） |
| `$02` | 休止（SQ 写 `$10` 关音量；Tri 写 `0`） |
| `$B0-$BF` | 音符长度指令：长度 = `NoteLengthsTbl[offset + (b & $0F)]` 帧 |
| `$C0-$FF` | repeat 起点：次数 = `b & $3F` → `RepeatCounter`，记下当前 `RepeatID` |
| `$FF` | repeat 尾：计数减到 0 则跳回 `RepeatID` |
| 其余字节 | SQ1/SQ2/Tri：`MusicNotesTbl` 的字节偏移（因此实际出现的都是偶数）；Noise：直接当 `SFXData` 的 Y 索引（合法 `$01/$04/$07/$0A`，见 §4.3） |

### 4.2 每帧音乐推进（A）

`UpdateAllMusicChannels $BAA5`：先 `ResetVolumeIndex`（新音符首帧把 `VolumeIndex` 置 1），然后 4 个通道依次 `UpdateMusicChannel $BAC2`：`MusicSQ1Base,x` 高字节为 0 → 空槽跳过；`dec MusicSQ1InstrDelay,x` 到 0 才 `LoadNextMusicChannelInstr $BADC` 取下一字节。

### 4.3 Noise 轨（鼓点）（A）

`MusicChannelInstr_SongNoteNoise $BBDE`（`sound_engine.asm:2023`）：Noise 通道**不查 `MusicNotesTbl`**，流里的音符字节直接当 Y 索引 `SFXData`：`SFXData,y → NOISE_VOL`、`SFXData+1,y → NOISE_LO`、`SFXData+2,y → NOISE_HI`。注释写明合法值是 `$01/$04/$07/$0A`（对应 `$B201/$B204/$B207/$B20A` 四组鼓点）；其他值会读到 SFXData 内部的其它偏移（原版怪音）。若 `SFXNoiseContFlags & ~(Pause|Silence)` 非 0（有噪声 SFX 在响）则不写，直接进 `SetMusicInstrDelayToLength`。

### 4.4 Tri 长度控制（A，`UpdateMusicTriLinearCount $BBB7`）

- `MusicTriCounterCntrl $062D` 低半字节 ≠0 → 用它当「短音符长度」；
- 高半字节 ≠0 → `$FF`（不自动关，随数据变化才停）；
- 两者都 0 → `(noteLen-1)*4`，上限 `$3C`。

### 4.5 曲终/循环/重播（A）

- `EndOrLoopMusic $B3F0`：`MusicLoopsOnEnd==0` ⇒ `InitializeSoundAddresses`（全清）；非 0 ⇒ `MusicLoopFlags ← MusicContFlags`，下一帧 cycle 9 重放（`RunMusicLoopRoutine $BC36` → `ContinueMusic`）。
- `InitializeMusic $BF19` 先 `CheckMusicFlags`：同曲 ⇒ 只清特殊地址；异曲 ⇒ 全清后从 `SongHeaders` 重新拷 13 B，`InstrDelay` 全部置 1、`InstrID` 置 0。

---

## §5 RAM 面 `$0600-$068D`（A，`constants_ram.asm:999-1163`）

> 卡面写 `$07xx` 的更正见 §0.4。142 B 与 `sound_ram_face` 一一对应。

| 区段 | 字段 | 语义 |
|---|---|---|
| `$0600-$0605` | `MusicSQ1PeriodLow/High`、`SQ2…` | 音乐周期备份（SFX 结束后恢复用） |
| `$0606/$0607` | `LoadMusicSQ1SQ2PeriodsFlag` | 「SFX 刚结束、下帧恢复周期」 |
| `$0608/$0609` | `MusicTriPeriodLow/High` | Tri 周期 |
| `$060A-$0617` | `SFXTriPeriod*`、`SFXTriChange*`、`SFXTriDivisor` | Tri SFX 的周期/步进/除法 |
| `$0618-$061E` | `SpareMem0618`（7 B） | asm 标注未用 |
| `$061F` | `HasBeamSFX` | bit7=LongBeam / bit0=IceBeam（BulletFire 变体） |
| `$0620-$0623` | `Music{Ch}InstrLength` | 当前音符剩余帧 |
| `$0624-$0627` | `Music{Ch}RepeatCounter` | repeat 计数 |
| `$0628/$0629` | `MusicSQ1/SQ2DutyEnvelope` | 高 4 位=duty（初值来自 Music*Init） |
| `$062A` | `MusicTriLinearCount` | Tri 线性计数 |
| `$062B` | `NoteLengthTblOffset` | 音符长度表组偏移 |
| `$062C` | `MusicLoopsOnEnd` | 曲终是否循环 |
| `$062D` | `MusicTriCounterCntrl` | Tri 长度覆盖 |
| `$062E/$062F` | `MusicSQ1/SQ2VolumeEnvelopeIndex` | 包络索引（0=无） |
| `$0630-$0637` | `Music{Ch}Base`（4×dw） | 通道流速指针 |
| `$0638-$063B` | `Music{Ch}InstrID` | 当前流偏移 |
| `$063C-$063F` | `Music{Ch}RepeatID` | repeat 回跳点 |
| `$0640-$0643` | `Music{Ch}InstrDelay` | 帧倒计时 |
| `$0648-$064A` | `MusicSQ1/SQ2Sweep` | sweep 值（初值 `$7F`） |
| `$064B` | `ThisSoundChannel` | 当前处理通道（0/4/8/0C） |
| `$064D` | `CurrentSoundFlags` | 本轮扫描的旗 |
| `$0652-$0655` | `{Noise,SQ1,SQ2,Tri}UsedBySFX` | 占用标记（SQ1=1/SQ2=2/Tri=3；`NoiseUsedBySFX` asm 注「Not used」） |
| `$065C` | `ChannelType` | 0=Noise/1=SQ1/2=SQ2/3=Tri/4=Multi |
| `$065D` | `MusicLoopFlags` | 曲终重播请求 |
| `$065E` | `MusicInitIndex` | 音乐 id（含 Multi 路径 +8） |
| `$0660-$0664` | `SFX{Ch}Length` | SFX 总帧数 |
| `$0665-$0669` | `SFX{Ch}Frame` | SFX 当前帧 |
| `$066A/$066B` | `VolumeIndex` | 包络读位置 |
| `$066C/$066D` | `VolumeData` | 当前音量值 |
| `$0670-$067B` | `SFX{Ch}Var0/1/2` | SFX 私有变量 |
| `$0680-$0685` | 六个 init 旗 | 见 §1.2 |
| `$0688-$068C` | `SFX{Ch}ContFlags` | 持续音旗 |
| `$068D` | `MusicContFlags` | 正在播的音乐（`FindMusicInitIndex` 得位号） |

---

## §6 APU 寄存器写序

### 6.1 每帧固定（A）

| 寄存器 | 值/时机 | 证据 |
|---|---|---|
| `$4017` | `$C0`（`APU_5STEP|IRQDISABLE`），**每帧一次**，在 `SoundEngine` 最开头 | `sound_engine.asm:387-388` |
| `$4015` | `$0F`（四通道全开），**全局只在开机写一次** | `prg7_engine.asm:176-177` |

### 6.2 事件驱动（A）

| 触发 | 写入 | 说明 |
|---|---|---|
| 音乐载新音符（SQ） | `SQ1_VOL,$4000`、`SQ1_LO`、`SQ1_HI`、`SQ1_SWEEP`（或 SQ2 对应 `$4004-$4007`） | 只在音符/休止变化时；被 SFX 占用则跳过（§2.3） |
| 音乐载新音符（Tri） | `TRI_LINEAR $4008`（音量/线性计数）、`TRI_LO $400A`、`TRI_HI $400B` | 线性计数由 §4.4 计算 |
| 音乐载 Noise 鼓点 | `NOISE_VOL/LO/HI $400C-$400E` | 噪声周期由固定表给 |
| 音量包络每帧 | `SQ1_VOL` 或 `SQ2_VOL` | 有包络时每帧重写低 4 位 |
| SFX 起播 | 该通道 4 寄存器连写（`@loop_LoadSFXRegisters`） | 4 B 表项顺序 |
| SFX 持续音 | 取决于具体 cont 例程（周期步进/音量开关） | 按 handler 表分类，逐例程见 §3.2 |
| 静音 / 暂停 | `SQ1_VOL=$10, SQ2_VOL=$10, NOISE_VOL=$10, TRI_LINEAR=0, DMC_RAW($4011)=0` | `ClearSounds $B43E` |

### 6.3 帧内顺序（A）

一帧内：`$4017` → Noise SFX → Multi SFX → Tri SFX → 音乐（loop/init/continue）→ 清旗。同帧内 SFX 的写发生在音乐之前；音乐若发现通道被占则整段跳过。

---

## §7 Rust 移植契约

### 7.1 现状（A）

- `crates/game-core/src/sound/`：`mod.rs`（地址面 `addrs`、8 cycle 分派、主循环四调用序）、`sfx.rs`、`music.rs`、`state.rs`、`hook.rs` —— 引擎骨架与分派已存在。
- `crates/game-platform/src/lib.rs:62`：`AudioSink { write_reg(reg,val); frame_boundary() }`（trait 定义处）；`apu.rs` 是 `Apu` 结构，只做文件头注释提到的寄存器流收尾。`runtime.rs:29-35` 每帧把写流转发给 sink。壳侧此前只计数，**没有合成器/混音输出**。

### 7.2 建议最小接口（语义，不含实现）

```rust
/// 一帧一次，由帧循环在输入读完之后调用（对应 NMI 里的 jsr SoundEngine）。
pub fn tick(ram: &mut SoundRam, bank: BankId, sink: &mut dyn ApuSink);

/// 请求面：只写旗，不直接写 APU。
pub fn request_music(ram: &mut SoundRam, id: MusicId);
pub fn request_multi_music(ram: &mut SoundRam, id: MultiMusicId); // FadeIn/PowerUp/End/Intro
pub fn silence_music(ram: &mut SoundRam);
pub fn pause_sfx(ram: &mut SoundRam);
pub fn request_sfx(ram: &mut SoundRam, channel: SfxChannel, flags: u8);
```

约束：

1. **状态只存在 `$0600-$068D`**（typed face 已有），`tick` 不得读墙钟/浮点时钟；帧率固定 60.0988 Hz（NES NTSC）。
2. **bank 参数**：`tick` 需要知道当前驻留 bank，因为不同 bank 的 `SongHeaders` 流指针不同；移植若不模拟 bank，可把「本 bank 实有曲目」表（§3.3）作为数据表，遇非本 bank 曲目按占位指针处理（读 RAM）或拒绝——两种都必须**显式记录**，不能静默换成正确数据。
3. **写流是对拍接口**：`ApuSink` 只观测，不得反向修改引擎状态；同一帧写入序列必须与 ROM 一致（§6）。
4. **保真 quirk**：`UsedBySFX` 的「通道号+1」语义、暂停只播前 18 帧、`$4015` 只写一次、SFX 帧计数到 `Length` 归 0 而不是停、repeat 计数在 `$0624-$0627`。

### 7.3 需要的数据段（建议由 `game-data` 资产包提供）

`SFXData`（31×4 B + 4 组鼓点）、8 张 handler 表、`GetSoundRoutineData`、`SongHeaderOffsetTable`、`SongHeaders`（按 bank 的 6 份）、`MusicNotesTbl`（64×2 B）、`NoteLengthsTbl`（3×12 B）、`VolumeEnvelopePtrTable` 与 5 条包络。音频流（每首 4 条）体量大，建议按需加载并记录「当前 bank 拥有哪些曲」。

---

## §8 复跑命令

ROM 映射：bank b（0–6）的 `$8000` 窗口在文件偏移 `16 + b*0x4000`；bank7 的 `$C000` 在 `16 + 7*0x4000`。

```bash
cd /home/peng/workspace/metroid-decomp
python3 - <<'PY'
rom=open('rom/Metroid (USA).nes','rb').read(); prg=rom[16:16+128*1024]
def rb(b,a): return prg[b*0x4000+(a-0x8000)] if b<7 else prg[7*0x4000+(a-0xC000)]
# 1) SFX 表前 14 B（期望 00 10 01 18 00 01 38 01 02 40 00 09 58 80）
print('SFXData', ' '.join(f'{rb(1,0xB200+i):02X}' for i in range(14)))
# 2) GamePaused 4 B（期望 80 7F 80 48）
print('GamePaused', ' '.join(f'{rb(1,0xB20D+i):02X}' for i in range(4)))
# 3) Brinstar 头（bank1 slot10 = 0A*13）（期望 0B FF 00 02 03 00 B0 57 B0 C1 B0 2B B1）
print('HdrBrinstar', ' '.join(f'{rb(1,0xBD31+10*13+i):02X}' for i in range(13)))
# 4) Tourian 头（bank1 slot11）（期望 0B FF 03 00 00 59 BE 47 BE 62 BE 00 00）
print('HdrTourian', ' '.join(f'{rb(1,0xBD31+11*13+i):02X}' for i in range(13)))
# 5) 音乐 id→头槽偏移（期望 [65,143,52,39,26,13,0,130,104,117,78,91]）
print('Offsets', [rb(1,0xBBFA+i) for i in range(12)])
# 6) 每 bank 实有曲目（判据：4 条流指针 ≥ $8000）
off=[65,143,52,39,26,13,0,130,104,117,78,91]
names="Ridley Tourian ItemRoom Kraid Norfair Escape MotherBrain Brinstar FadeIn PowerUp End Intro".split()
for b in range(6):
    real=[]
    for mid in range(12):
        a=0xBD31+(off[mid]//13)*13
        ptrs=[rb(b,a+5+2*k)|rb(b,a+6+2*k)<<8 for k in range(4)]
        if all(p == 0 or p >= 0x8000 for p in ptrs): real.append(names[mid])
    print('bank',b,real)
# 7) 音符表首项（期望 07 F0）与包络指针（期望 bcba/bcc5/bccf/bcda/bd03）
print('NoteA1', ' '.join(f'{rb(1,0xBE77+i):02X}' for i in range(2)))
print('EnvPtrs', [hex(rb(1,0xBCB0+i)|rb(1,0xBCB1+i)<<8) for i in (0,2,4,6,8)])
PY
```

---

## §9 假设区（C 级，附推翻方法）

| # | 假设 | 推翻方法 |
|---|---|---|
| C1 | bank6 驻留期间引擎不跑音乐（否则 `$B3B4` 在 bank6 是别的代码） | trace：换到 bank6（图形解压）帧的 `$1A` 与 `MainRoutine`；若 NMI 未跳过而 PC 落进 `$B3B4` 以外地址，假设成立 |
| C2 | 播「非本 bank 曲目」会读 `$0100/$0300/$0500/$0700`（RAM）当乐谱，原版靠区域曲表规避、正常游戏不可达 | 构造 `StartMusic` 请求非本 bank 曲（如 bank1 请求 Ridley）跑 300 帧，看 `$0630-$0637` 是否写到占位地址并读 RAM |
| C3 | `GamePaused` SFX 只在暂停后 18 帧内重播，之后静音直到恢复 | trace `$0602/$064D` 与 `$4000/$4004` 写，pause 40 帧观察 |
| C4 | Trie SFX 的 `SFXTriDivisor $0616` 用于把周期换算成三角波计数，具体除数只在各 cont 例程内部 | 逐例程反汇编 `SamusDieSFXCont` 等，列算式 |

## §10 未覆盖 / 后续

- **每个 SFX 的 cont 例程细节**（周期步进量、音量开关、长度）未逐条展开，只给分类与入口；下一张「SFX 逐例程行为表」卡承接。
- **音频输出**（混音/重采样/缓冲）属于平台层，不在本规格范围；本卡只定义「引擎 → APU 寄存器写流」这一层契约。
- **NTSC/PAL 差异**：本规格只保 NTSC（音长表/PAL 频率 1663000 只在注释里出现一次）。
