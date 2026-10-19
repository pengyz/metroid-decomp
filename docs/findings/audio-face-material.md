# G3 音频面素材预位（refs asm 先读——音频引擎/旗族谱系/音乐面）

> G3 战役首件（音频面素材预位）。Evidence：A（bank:addr 锚）。
> refs asm 先读纪律照执行（constants.asm/prg7 直查）。2026-10-19 decomp。

## 1. 音频引擎锚（A 级：prg7 调用点）

- **SoundEngine/GotoSoundEngine**（prg7:272/274——NMI/主环双调）——引擎入口；
- **SetMusicInitFlag**（07:CC09）：`ldx #MusicInitFlags - SFXNoiseInitFlags`（旗偏移
  数学——旗族线性布局）+`ora MusicInitFlags`（音乐旗 OR 写）；
- **SFX 例程族**（A 级：CB96-CBB8+ 族）：SFX_SamusWalk/SFX_BombExplode/
  SFX_MissileLaunch/SFX_OutOfPipe/SFX_BombLaunch/SFX_SamusJump/SFX_EnemyHit/
  SFX_BulletFire/SFX_Metal…（十二现基础——G2 L2 批产共用）。

## 2. 旗族谱系（A 级：constants.asm:129-173——5 族 38 位）

| 族 | 位 | 旗 |
|---|---|---|
| **sfxNoise** | 7 | ScrewAttack/MissileLaunch/BombExplode/SamusWalk/SpitFlame/PauseMusic/SilenceMusic |
| **sfxSQ1** | 8 | MissilePickup/EnergyPickup/Metal/BulletFire/OutOfPipe/EnemyHit/SamusJump/WaveFire |
| **sfxTri** | 8 | SamusDie/Door/MetroidHit/**StatueRaise**/Beep/BigEnemyHit/SamusBall/BombLaunch |
| **sfxMulti** | 7 | Intro/PowerUp/EndMusic/IntroMusic/SamusHit/BossHit/IncorrectPassword |
| **music** | 8 | RidleyArea/Tourian/ItemRoom/KraidArea/Norfair/Escape/MotherBrain/Brinstar |

- **G2 面交叉注**：sfxTri_**StatueRaise**（statue 门机制声面）+music_Tourian/KraidArea/
  RidleyArea（boss 域音乐面）——G2 素材→G3 音频面接面 ✓；
- **旗族线性布局实证**（CC09 偏移数学）：MusicInitFlags-SFXNoiseInitFlags=旗族连续
  RAM 块（SFXNoise→SQ1→Tri→Multi→Music 序）。

## 3. 遗留（C 级——G3 批产取证包预备）

- SoundEngine 例程体（引擎调度流/声道分配/包络）=待读；
- 音乐数据布局（音乐表/序列格式/音符编码）=待考；
- SFX 例程体（CB96 族——旗置位→发声参数流）=待读；
- 声旗 RAM 布局（SFXNoiseInitFlags 等地址锚——constants_ram 待查）。

## 4. 【2026-10-19 G3 批产取证包】C→A/B 四面升（引擎体/音乐面/SFX 体/声旗 RAM）

- **声旗 RAM 布局**（A 级：constants_ram:1123-1129）：$0680 SFXNoise/$0681 SQ1/
  $0682 SQ2（**never used**）/$0683 Tri/$0684 Multi/$0685 Music——**六旗连续块**
  +$0686 ScrewAttack0686/$0688 SFXNoiseContFlags（续行旗）——CC09 偏移数学吻合；
- **引擎核**（A 级：sound_engine.asm:384+=SoundEngine $B3B4）：APU 5 步同步
  （JOY2=APU_5STEP|IRQDISABLE）→SilenceMusic 判（旗 bit0）→Pause 判（MainRoutine）→
  **四 Runner 序**（NoiseInit→MultiInit→TriInit→**MusicLoopRoutine($BC36)**）→
  ClearInitFlags（六旗清）；
- **引擎结构**（A 级）：SFXData($B200) 数据表+Init/Cont 例程表×4（Noise/Tri 各二）
  +Runner 族×7（B31B-B35C）+LoadSFXData($B368)+PauseSFX($B38F)+EndOrLoopMusic
  （$B3F0——MusicLoopsOnEnd 循环判）；
- **音乐面**（B 级）：引擎二分（song init/loop 分离——sound_engine.asm:19 注）+
  音乐数据（noise 通道鼓点数据+SoundRoutineTablePtr/SoundRoutinePtr 指针对）——
  **音乐数据布局=序列格式/音符编码待考**（C 余）；
- **SFX 体**（B 级）：CB96 族例程=旗置位→SFXData 查表→Init/Cont 例程执行——
  例程体细读=待（C 余）。

## 5. 【2026-10-19 G3 C 余续采】序列格式+SFX 数据体（C→A/B 升）

- **SFXData 全表**（A 级：sound_engine.asm:25-96=$B200-）：**28 SFX 项×4 字节
  APU 参数**（GamePaused/ScrewAttack/MissileLaunch/BombExplode/SamusWalk/SpitFlame/
  SamusHit/BossHit×2/IncorrectPassword×2/TimeBombTick/EnergyPickup/MissilePickup/
  Metal/LongRangeShot/ShortRangeShot/SamusJump/EnemyHit/OutOfPipe/WaveBeam/IceBeam/
  BombLaunch×2/SamusBall/MetroidHit/SamusDie/Beep/BigEnemyHit/**StatueRaise**/Door）
  +鼓点项×4（noise 音乐 3 字节）；
- **序列/表格式**（A 级：GetSoundRoutineData 注=B289 表）：每行=[SoundRoutineTablePtr
  词][SoundRoutinePtr 词][ChannelType 字节]——ChannelType：0=Noise/1=SQ1/3=Tri/
  4=Multi——**指针对+通道型三元组格式**；
- **4 字节 SFX 格式**（B 级）：APU 寄存器参数（byte1=周期/byte2=$7F 常见（音量/旗）/
  byte3-4=包络/时值——细语义待考）；
- **G2 面交叉续**：StatueRaise SFX 项（$B281）+Door（$B285）——statue/门声面 ✓。

## 6. 【2026-10-19 G3 取证包完备】音乐音符编码+例程体（C→A/B 升——批产粮草）

- **MusicNotesTbl**（A 级：sound_engine.asm:2477=$BE77）：**音符索引→APU 周期表**
  （2 字节/项——索引 #00=A1(55Hz)/#02=No sound/#04=C#2…#40=Ab4——**半音阶全表**
  （A1-Ab4+续）——16 位周期 lo/hi；**音符编码=索引×2 查表→APU 周期寄存器**；
- **例程体细读**（A 级）：CheckMusicFlags($B3FC)（MusicContFlags vs CurrentSoundFlags
  ——同曲判/清场分支）+InitializeSoundAddresses($B404)（清 MusicAndSFX/Special
  Addresses——**声道占用旗**：SQ1UsedBySFX/SQ2UsedBySFX/TriUsedBySFX+五 ContFlags
  族+MusicContFlags）+EndOrLoopMusic（MusicLoopsOnEnd 循环判）；
- **音乐播放流**（B 级）：RunMusicLoopRoutine($BC36)→MusicNotesTbl 查表（1905-1910
  双字节读）→APU 周期写——音符流细读=C 余（播放循环体）。
- **G3 取证包完备态**：引擎核/旗族谱系/声旗 RAM/SFXData 全表/序列格式/音符编码表
  ——**批产粮草全**（音符编码+例程体双升）。
