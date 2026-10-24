# findings：声引擎分派链 golden 蓝图（W2 接线规范——探针 A 级实证）

> 【2026-10-24 复核更正】verifier 对抗复核 verdict=needs_revision（F1-F4 阻断）——
> 以下 §1/§2/§3 含四处更正（原地标注）；原文主张证伪不删（证伪更正文化 §3.6）。
> 状态：**接线规范 v2**（复核修订后）。

## 1. golden 逐帧 walk 全貌（探针实证）

每帧（f8 起）SoundEngine 的分派序（`$E0-$E5` 指针面写点+caller 栈反推）：

```
① SFX 族 9 loader 链（caller=B337/B350——GetSoundRoutine 调用点）：
   B289(NoiseInit)→B322 | B28E(NoiseCont)→B4EE(终端)
   B350(MultiInit 特例)→B35C + FindMusicInitIndex（$065E：FF→扫描）
   B2AC(MultiCont)→B364 | B293(SQ1Init)→B330 | B298(SQ1Cont)→B4EE
   B29D(TriInit)→B344 | B2A2(TriCont)→B4EE
② 音乐 2 cycle（**asm 终审定型**——RunMusicLoopRoutine $BC36/RunMusicInitRoutine $BC3D）：
   cycle9  BC36：`lda MusicLoopFlags($065D)/ldx #GetSoundRoutineData@MusicLoop(B2B6)`
           → @Common：jsr GetSoundRoutine→jsr FindMusicInitIndex→`jmp (SoundRoutinePtr)`
           （B2B6 块 DefaultRoutine=BC3D ⇒ 链入 cycle10）
   cycle10 BC3D：`lda MusicInitFlags($0685)/ldx #@MusicInit(B2B1)`→@Common
           （B2B1 块 TablePtr=**MusicRoutineTbl($BC26 8 项）**、DefaultRoutine=BC4B）
           ⇒ 旗位 i 命中 ⇒ SoundRoutinePtr=MusicRoutineTbl[i]（GotoMusicNNInit 族）；
           未命中 ⇒ BC4B（ContinueMusic 默认）。
           FindMusicInitIndex（扫 CurrentSoundFlags $064D）⇒ MusicInitIndex($065E)=**曲号**
           （0x0B=Intro——GotoMusicNNInit 经 SongHeaderOffsetTable[曲号] 取曲头）。
   MusicRoutineTbl 8 项：00=GotoMusic04Init(Ridley) 01/02/03/06=GotoMusic00Init
   （Tourian/Item/Kraid/MotherBrain）04=GotoMusic03Init(Norfair) 05=GotoMusic02Init(Escape)
   07=GotoMusic03Init(Brinstar)。
③ ContinueMusic(BC4B)：MusicContFlags($068D)==0⇒出；≠0⇒jmp UpdateAllMusicChannels($BAA5)
```

**音乐 init 发射窗（f16 实录——8 帧延迟后旗就绪）**：

> 【F2 更正】原注「经 BC06 表 MusicInitIndex 索引」**证伪**：探针 f16 全序
> `E2/E3←BC7A` **先于** B353 扫描——真机制=**GetSoundRoutine 旗扫 `$0684`→BC06 表[旗位]**
> （MusicInitIndex 扫描在其后，供曲头选取）。§1② MusicRoutineTbl[旗位] 路径 boot-title
> **零演练**（cycle10 恒走默认 BC4B）=asm 推断 **B 级**，非探针 A 级。

```
MultiInit 特例链 → GetSoundRoutine 旗扫 $0684→**BC06 表[旗位]**⇒BC7A(GotoMusic01Init)
→ FindMusicInitIndex 扫**$064D**（本周期旗副本）：$065E: FF→逐位 inc(00,01,02,03)
→ MusicInitIndexAdd8→0x0B(Intro)
→ CheckMusicFlags(B404: $068D←00) → $065D←00(B40A)
→ XYMusicInit(BC90: $068D←**0x10**) ← MusicContFlags 就位！
→ 其后每帧 ③ 进 UpdateAllMusicChannels（音乐块/包络逐帧更新）
```

> 【F3 更正】walk **非固定序**：f16 仅 7 次 GetSoundRoutine（B2AC/B293/B298 被跳过）——
> **解析即 `jmp(SoundRoutinePtr)`：旗命中 ⇒ 表[旗位]替换链续默认，handler 执行后
> 直接 RTS 回 SoundEngine（本链余下 loader 不再走）**；f16 复走从 TriInit 链恢复
> =下一 cycle。11-cycle=8 薄 loader（共用 B337 尾 `jsr GetSoundRoutine/jmp(ptr)`）
> +MusicLoop(9th)+MusicInit(10th)+ContinueMusic(11th——无 GetSoundRoutine）
> 【M1 更正：原「9 SFX+2 音乐」错拆】。SoundEngine=cycle 入口 jsr 族+链默认串联。

## 2. 我方接线缺口（对照——复核 v2 增补）

| 缺口 | 现状 | 修法 |
|---|---|---|
| **GetSoundRoutine 全语义**（F1/F2/F3 公共根） | 折叠为 get_sound_routine(ptr,hit)——`sta $064D` 旗副本/pha-pla 恢复/表[y]替换默认全缺 | ①`$064D←A`（本周期旗副本——**FindMusicInitIndex 的真实扫描源**）②4B 拷+$065C ③MSB 扫描 y=2×位号→`$E2/$E3=表[y]`替换默认 ④恢复 $064D |
| **$064D 写者缺席**（F1） | sound_hook.rs:281 只读 | 随 GetSoundRoutine 入口写 |
| **FindMusicInitIndex 写序**（F1） | multi_music_index_step=「语义粗形」（max(base)+8——f16 点错：粗形 0x07 vs asm 0x0B）**虚标已端口** | 实现 `$065E←FF→逐位inc（每中间值写）→Add8` 写序；**降格粗形函数**（留档对照）；扫 `$064D` 非 `$0685` |
| **链续=解析驱动**（F3） | LOADER_MAP 固定序递归 | `jmp(SoundRoutinePtr)` 语义：旗命中⇒表[旗位]替换链续默认；handler 执行后**本链终止**（RTS 回 SoundEngine——f16 跳过 B2AC/B293/B298 实证） |
| SoundEngine cycle 编排 | 4 链简化形（结构对——4 cycle 入口族 ✓） | 补 MusicLoop cycle（BC36/B2B6→BC3D）——现缺第 9 cycle；cycle10=B2B1→BC4B 默认 |
| Multi 特例音乐发射 | 只做粗形索引 | 旗扫 $0684→BC06 表[旗位]⇒GotoMusicNNInit→MusicNNInit 组合臂 |
| MusicNNInit 组合臂 | check_music_flags/initialize_music/xy_music_init_w1（**W1 子面**——M3 虚标更正）纯函数在位 | 接线组合（B404 面+$065D 清+XYMusicInit 写面）；duty 形参按曲族双形 |
| UpdateAllMusicChannels(BAA5) | `unported!` 爆 | entry 面+四通道环已端口；**音符载残面被低估**（M5：$0620-$0643 34B+四通道环+tempo+APU 写面）=W3 主体 |
| **超帧路径声引擎缺席**（M6 新漏） | nmi_body:40-42 早退吞掉 SoundEngine/UpdateAge（「移至尾前独立段」=陈旧注） | asm prg7_engine @endIf_A 后**无条件** jsr SoundEngine/UpdateAge——超帧（NMIStatus≠0）也须跑；实施=早退仅跳屏幕/输入面，声/龄面无条件 |
| find_music_init_index 双实现（M4） | mod.rs:322(u8) vs music.rs:49(Option) | 定正典（music.rs 版）+另一件转调或删 |

## 3. 已端口件索引（sound/mod.rs——**复核 v2 校准**：W1 子面≠全件）

`music_channel_vm_step`（VM 三分派+重复机）/`initialize_music`（13B 头载）/
**`xy_music_init_w1`（W1 子面——禁扫双写+duty 对称+Cont 旗；XYMusicInit 全件未端口（M3））**/
`update_volume_envelope`+`update_all_volume_envelopes`（包络机）/
`update_all_music_channels_entry`（entry 面）/`check_music_flags`/`music_nn_init`（曲族双形
组合缝——头载参数化待接）/`goto_music_init`（6 跳板映射）/`SONG_HEADERS` 156B 烘焙。
~~`multi_music_index_step`~~（**降格**：语义粗形非 asm 等价——F1）。
`find_music_init_index` 双实现待归正典（M4）。

> 【M2 帧号口径】探针 frameCount≈oracle trace+9（$068D 探针 f16↔trace f7）——
> 本文 §1 用探针口径、验收用 trace 口径，对照时注意换算；实施验收以 **trace 口径**为准。

## 4. 验收判据（W2 完成定义）

1. L3 双腿恒差清单：`$E6/$E7/$EA/$EC/$ED`+`$0600-$068D` 77B → 0
   （复现：`python analysis/_soundgap.py` + `_legcmp.py`）；
2. `$068D` 轨迹双侧同（f16 就位 0x10）；`$065E` 扫描轨迹同；
3. `cargo test --workspace` 全绿 + 新接线件 TDD；`gates/cli.py trace` PASS。

## 5. 证据指针

- 探针实录：`analysis/traces/sndchain-boot.probe.jsonl`（19198 写点）+
  `analysis/_sndchain.py`（walk 重放脚本）；场景 `oracle/scenarios/sndchain-boot.txt`
- asm：`sound_engine.asm` BC36-BCB0（RunMusicInitRoutine/ContinueMusic/
  FindMusicInitIndex/Add8/GotoMusicNNInit 族——逐字引证见 §1）；
- 既往音频账：`audio-fullload-sizing.md`（SongIntro 触发面）/`b-run-round-w1-audio-clean.md`（W1 全清实证）。
