# 音频全量定尺寸（task-44——sound_engine.asm 全文 asm-first 勘测）

> 【主文档】refs/m1disasm/SRC/sound_engine.asm（2721 行/255 标签/$B200-$BF19 裸址域）。
> 【方法】asm-first：全文结构勘测→handler 全枚举→MusicLoop 深径→曲表族→定尺寸排程。
> 【同源纪律】与 handler 细行清偿（task-30）同一主文档——**读一件清一件随录本清单**
> （已清行随批划销），考证即排程。
> 【分类】B 级（标签图谱+行距推断）；区间切分 A 级（标签址=字节锚）。

## 1. 全文结构（区域×地址段）

| 区 | 地址段 | 内容 | 估（行/件） |
|---|---|---|---|
| SFXData 数据岛 | $B200-$B2B6 | **35 SFX 数据块**（DrumBeat×4/GamePaused/…/Door） | 数据（烘焙面） |
| 分派表 | $B2BB-$B30B | SFXInitPointers/SFXContPointers（8 cycle×8 位表目） | 数据+语义 1 件 |
| 分派核 | $B31B-$B4EE | 八 cycle 薄装载器+LoadSFXData+SoundEngine+ClearInitFlags+PauseSFX | **已清 2 件**（scan/get_sound_routine） |
| 旗/帧计数 | $B493-$B4BD | UpdateContFlags/IncrementSFXFrame/ClearCurrentSoundFlags | 已清（B650 随件） |
| **SFX handler 族** | $B50F-$B7A6 | **26 件**（下表全枚举） | 件均 15-40 行 |
| handler 尾区 | $B7A7-$BBB7 | 续 handler/工具（Beam/Metroid/Statue/Door/Pipe 族+Timer 帮手） | 件均 15-40 行 |
| **MusicLoop 深径** | $BC06-$BC76 | MusicRoutineTbl+RunMusicLoopRoutine/RunMusicInitRoutine/ContinueMusic/FindMusicInitIndex/MusicInitIndexAdd8 | 5 件（Multi 特例面） |
| 曲 Init 族 | $BC77-$BCAF | GotoMusic00-05Init+Music00-05Init+XYMusicInit | 7 件 |
| 音量包络 | $BCB0-$BD03 | VolumeEnvelopePtrTable+5 包络 | 数据 1 件 |
| **曲表族** | $BD31-$BDC0 | **12 曲头**（MthrBrnRoom/Escape/Norfair/Kraid/ItemRoom/Ridley/End/Intro/FadeIn/PowerUp/Brinstar/Tourian） | 数据+语义 1 件 |
| 音符表 | $BE77-$BF18 | MusicNotesTbl+NoteLengthsTbl | 数据 1 件 |
| 初始化 | $BF19+ | InitializeMusic | 1 件 |

## 2. SFX handler 全枚举（26 件——**已清 2/26**）

| # | handler | 地址 | 态 | 备注 |
|---|---|---|---|---|
| 1 | SamusHitSFXInit/Cont | $B673/$B650 | ✅**已清**（7af2492/8bb4371） | 对件收口（B673 钩接点待 porter 钩） |
| 2 | SpitFlameSFXInit/Cont | $B50F/$B516 | 未清 | 噪声族 |
| 3 | ScrewAttackSFXInit/Cont | $B52B/$B539 | 未清 | 含 @part2 子径 |
| 4 | MissileLaunchSFXInit/Cont | $B56E/$B57B | 未清 | |
| 5 | BombExplodeSFXInit+NoiseSFXCont | $B583/$B58A | 未清 | GotoInitSFXData/EndNoiseSFX 帮手 |
| 6 | SamusWalkSFXInit | $B598 | 未清 | Init 单件 |
| 7 | BossHitSFXInit/Cont | $B5EC/$B5F6 | 未清 | WriteSQ1SQ2PeriodLow+数据表 |
| 8 | IncorrectPasswordSFXInit/Cont | $B695/$B6A1 | 未清 | |
| 9 | MissilePickupSFXInit/Cont | $B6CD/$B6B0 | 未清 | SFXTbl |
| 10 | EnergyPickupSFXInit/Cont | $B6E7/$B6D3 | 未清 | |
| 11 | SQ1SFXCont+EndSQ1SFX | $B6ED/$B6F2 | 未清 | 共享尾族 |
| 12 | SamusJumpSFXInit | $B703 | 未清 | |
| 13 | EnemyHitSFXInit | $B710 | 未清 | |
| 14 | BulletFireSFXInit/Cont | $B716/$B74F | 未清 | |
| 15 | HasLongBeamSFXInit | $B72F | 未清 | |
| 16 | MetalSFXInit | $B735 | 未清 | GotoInitSFXData2 |
| 17 | OutOfPipeSFXInit | $B73C | 未清 | |
| 18 | HasIceBeamSFXInit/Cont | $B75E/$B765 | 未清 | IceBeamTbl |
| 19 | WaveBeamSFXInit/Cont | $B77A/$B781 | 未清 | |
| 20-26 | 尾区（Beam/Metroid/Statue/Door/TimeBomb/Pickup/Timer 族） | $B7A7-$BBB7 | 未清 | 逐件随录补全 |

工具帮手（随件折叠）：IncrementNoisePeriod $B564/GotoInitSFXData $B587/$B739/
WriteSQ1SQ2PeriodLow $B62C/LoadSQ1PeriodLow $B7A0/EndNoiseSFX/EndSQ1SFX/EndMultiSFX。

## 3. MusicLoop 深径（$BC06-$BC76——5 件）

- RunMusicLoopRoutine $BC36（主径——每帧音乐推进）→ RunMusicInitRoutine $BC3D（init 面）
  → ContinueMusic $BC4B（续奏面）→ FindMusicInitIndex $BC53+MusicInitIndexAdd8 $BC64
  （Multi 特例双步——**已粗形**于 sound::multi_music_index_step，深证随件补）。
- 曲 Init 族 7 件：GotoMusic00-05Init（跳板）+Music00-05Init+XYMusicInit（$BC77-$BCAF）。

## 4. 曲表族（数据+语义 2-3 件）

12 曲头（$BD31-$BDC0）：MthrBrnRoom/Escape/Norfair/Kraid/ItemRoom/Ridley/End/Intro/
FadeIn/PowerUp/Brinstar/Tourian——**boot-title 触发面=SongIntro**（IntroMusic $0684←$10
路径）。音符表 MusicNotesTbl $BE77+NoteLengthsTbl $BEF7+5 音量包络（$BCB0 族）。

## 5. 定尺寸排程（考证即排程）

| 波次 | 内容 | 件数 | 估 |
|---|---|---|---|
| W1（B 跑触发面） | SongIntro 音乐径（MusicLoop 5 件中 3+曲头烘焙）+Beep 族（Tri） | 4-5 | 2-3 天 |
| W2（SFX 高频） | SamusWalk/EnemyHit/BulletFire/MissileLaunch/MissilePickup/EnergyPickup | 6 对件 | 3-4 天 |
| W3（SFX 中频） | SpitFlame/ScrewAttack/BombExplode/BossHit/IncorrectPassword/SamusJump | 6 件 | 3 天 |
| W4（SFX 尾区+曲族） | 尾区 7 件+曲 Init 族 7 件+曲表/音符/包络烘焙 | 15 | 3-4 天 |
| 收口 | InitializeMusic+分派表烘焙+全族 L2/域用例补 | 2-3 | 1-2 天 |
| **合计** | | **~35 件** | **12-16 天（1-2 周段精确化）** |

【随录区】（读一件清一件）：~~B673~~/~~B650~~（已清）；W1 SongIntro 径=下一波（B 跑 boot-title
音乐行触发面）。
【证据指针】标签图谱=grep `^[A-Za-z_@].*: ; \$B` sound_engine.asm（255 行命中）；
已清件=commit 7af2492/8bb4371；B 跑触发面=docs/findings/b-run-round2-title-clear.md。
