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
