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

## 4. 批产支援面（WARN 批量上调候裁件预备——2026-10-19）

- **rust_test 增量面清点**：基线 365（改名卡时）→现值 379——**+14 纯增量**
  （A 批批产 A1-015~019 九例预登记制+B1 系批产卡测试——无删测/无跳过）；
- **reason 汇总**（批量上调候裁）：「A 批九例批产（A1-015~019）+B1 系批产+G3 预备
  测试——rust_test 365→379 纯增（无回退三证：全测过/无删测/无跳过）」；
- **烤案 yield 态收**（g3-pregen）：预备态（cases 0——golden 推导=批产期随做）——
  批产开工随行支援面就绪。

## 5. SC-G3-00 预取证形（首卡——porter A 批收口即接）

- **卡面**：G3-引擎核簇（SoundEngine $B3B4 六员——引擎核/例程表×4/旗清）；
- **证据锚**（A 级）：sound_engine.asm:384-425（引擎核流）+constants_ram:1123-1129
  （旗族 RAM）+CC09（旗偏移数学）；
- **行为概要**（B 级）：APU 5 步同步（JOY2 写）→SilenceMusic 判（旗 bit0）→Pause 判
  （MainRoutine）→四 Runner 序（NoiseInit→MultiInit→TriInit→MusicLoop）→六旗清
  （ClearInitFlags）；
- **RAM 面**：$0680-$0688 旗族块+$0686 ScrewAttack0686+五 ContFlags 族；
- **烤案面**（g3-pregen）：g3-000-case（bank07:B3B4×4 LCG→ROM golden）——
  ram_addrs=旗族块；
- **验收**：L2 增量+L3 回归+全案绿（施工图 §2 口径）。

## 6. bank07 窗形修协同面（l2-diff entry 形扩测试面预备）

- **协同注**（porter 在做——l2-diff bank07 窗形修）：g3 案 entry=**bank07: 形**
  （B3B4/B31B/B337/B34B/B368/B3F0——固定 bank $C000 域）+window_bank=7 显式；
- **测试面预备**（g3 六案=形扩测试面）：
  ① bank07 固定窗形（$C000 域——非 $8000 切窗）——entry bank 标签=窗口 bank（合一）；
  ② window_bank=7 显式携带（README 契约面：L2In.window_bank 同取）；
  ③ **写集面**：RAM 写（$0000-$07FF 归一）vs APU 写（$4000 族——非 RAM 不记）——
  g3-004 注（LoadSFXData APU 写不入 out.writes）；
- **验收协同**：窗形修落→ROM golden 抓取→六案三方对拍（golden==6502 核==game-core）
  ——verdict pass 前置=ROM golden 替换（质量底线 #1）。
