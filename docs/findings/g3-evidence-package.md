# findings：G3 批产取证打包（音频面簇——引擎/旗族/数据表/编码表）

> 状态：**G3 证据预备仓就位**（2026-10-19）。预消化形同 a 系（指令头直读——
> 音频面名注稠密（sound_engine.asm 全名目）以名注为主锚）。
> 取证包六面全落（audio-face-material.md §1-6）——本包=批产施工卡面。

## 1. G3 簇清单（音频面簇——名注+addr 预消化）

| 簇 | 员 | 成员（addr+名） |
|---|---|---|
| G3-引擎核 | 6 | SoundEngine($B3B4)/SoundEngine_GameIsPaused($B399)/SoundEngine_SilenceMusic($B3EB)/ClearInitFlags($B3D6)/EndOrLoopMusic($B3F0)/CheckMusicFlags($B3FC) |
| G3-例程表 | 4 | SFXNoiseInitRoutineTbl($B2BB)/SFXNoiseContRoutineTbl($B2CB)/SFXTriInitRoutineTbl($B2FB)/SFXTriContRoutineTbl($B30B) |
| G3-Runner 族 | 7 | RunSFXNoiseInit($B31B)/RunSFXNoiseCont($B322)/RunSoundRoutine($B337)/RunSFXTriInit($B33D)/RunSFXTriCont($B344)/RunSFXMultiInit($B34B)/RunSFXMultiCont($B35C) |
| G3-数据表 | 3 | SFXData($B200)/GetSoundRoutineData($B289)/MusicNotesTbl($BE77) |
| G3-加载/暂停 | 3 | LoadSFXData($B368)/PauseSFX($B38F)/InitializeSoundAddresses($B404) |
| G3-旗族 RAM | 7 | SFXNoiseInitFlags($0680)/SFXSQ1InitFlags($0681)/SFXSQ2InitFlags($0682)/SFXTriInitFlags($0683)/SFXMultiInitFlags($0684)/MusicInitFlags($0685)/SFXNoiseContFlags($0688) |
| G3-SFX 例程 | 12 | SFX_SamusWalk(CB96)/SFX_BombExplode(CB9A)/SFX_MissileLaunch(CB9E)/SFX_OutOfPipe(CBA4)/SFX_BombLaunch(CBA8)/SFX_SamusJump(CBAC)/SFX_EnemyHit(CBB0)/SFX_BulletFire(CBB4)/SFX_Metal(CBB8)+族续 |

## 1.5 消化口径（同 a 系 F13 钉死）

**标签口径三级**：①名注优先（m1disasm `Name: ; BB:XXXX`——音频面名注稠密 ✓）
②L 行尾注次之 ③指令头兜底。口径随包不变。

## 2. 跨区出边（音频面→bank7 引擎调用——残余随件）

| bank7 目标 | 调用 |
|---|---|
| RunMusicLoopRoutine($BC36) | 引擎核调（SoundEngine 四 Runner 序末位） |
| CommonEnemyStub | （无——音频面独立域） |

## 3. 结构互核注+行数核自查条

- 簇边界=音频面域（sound_engine.asm 2721 行全域+constants_ram 旗族块）；
- **行数核自查条**：本包成员表行数=簇 7/员 42——自查一致 ✓（行数核：清单 7 簇×表行=文档行数一致）；
- 每批验收=L2 增量+L3 回归+全案绿（施工图 §2 口径）。
