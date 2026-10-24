# findings：声引擎分派链 golden 蓝图（W2 接线规范——探针 A 级实证）

> 状态：**接线规范完成**（2026-10-24，`oracle/scenarios/sndchain-boot.txt` 探针实录）。
> 本文=声音域 W2 的实施规范：golden 侧分派链逐帧 walk 全貌 + 我方接线缺口 + 精确修法。
> 缺口画像见 [sound-domain-gap-workorder.md](sound-domain-gap-workorder.md)。

## 1. golden 逐帧 walk 全貌（探针实证）

每帧（f8 起）SoundEngine 的分派序（`$E0-$E5` 指针面写点+caller 栈反推）：

```
① SFX 族 9 loader 链（caller=B337/B350——GetSoundRoutine 调用点）：
   B289(NoiseInit)→B322 | B28E(NoiseCont)→B4EE(终端)
   B350(MultiInit 特例)→B35C + FindMusicInitIndex（$065E：FF→扫描）
   B2AC(MultiCont)→B364 | B293(SQ1Init)→B330 | B298(SQ1Cont)→B4EE
   B29D(TriInit)→B344 | B2A2(TriCont)→B4EE
② 音乐 2 cycle（caller=BC42——RunMusicInitRoutine@Common 自身的 GetSoundRoutine）：
   BC36(MusicLoop,flag=$065D,block B2B6)→resolve BC3D(RunMusicInitRoutine)
   BC3D(,flag=$0685,block B2B1)→resolve BC4B(ContinueMusic)
   每次 @Common 尾：jsr FindMusicInitIndex（$065E←FF 面——BC45 写点）
③ ContinueMusic(BC4B)：MusicContFlags($068D)==0⇒出；≠0⇒jmp UpdateAllMusicChannels($BAA5)
```

**音乐 init 发射窗（f16 实录——8 帧延迟后 MusicInitFlags 就绪）**：

```
MultiInit 特例链 → resolve BC7A(GotoMusic01Init)（经 BC06 表 MusicInitIndex 索引）
→ FindMusicInitIndex 扫描（$065E: FF,00,01,02,03 逐位）→ B356: MusicInitIndexAdd8→0x0B(Intro)
→ CheckMusicFlags(B404: $068D←00) → $065D←00(B40A)
→ XYMusicInit(BC90: $068D←**0x10**) ← MusicContFlags 就位！
→ 其后每帧 ③ 进 UpdateAllMusicChannels（音乐块/包络逐帧更新）
```

## 2. 我方接线缺口（对照）

| 缺口 | 现状 | 修法 |
|---|---|---|
| SoundEngine 分派=4 链简化形 | nmi.rs sound_engine 4 flag 环 | 改 **11-cycle 编排**（9 SFX loader + 2 音乐 cycle——①②序照抄蓝图） |
| BC3D→BC4B 链续 | `_=>None→unported`（实际未达） | ptr==BC3D ⇒ 续走 MusicInit block B2B1 链（解析 BC4B） |
| Multi 特例音乐发射 | 只做 FindMusicInitIndex+Add8 | MusicInitIndex∈表 ⇒ resolve **BC06 表项**（0x0B→BC7A）→ MusicNNInit 组合臂 |
| MusicNNInit 组合臂 | CheckMusicFlags/InitializeMusic/XYMusicInit **纯函数已端口** | 接线组合：B404 面+$065D 清+XYMusicInit 写面 |
| UpdateAllMusicChannels(BAA5) | `unported!` 爆 | entry 面+四通道环**已端口**；VM 步进（MusicVmStep）已端口；**音符载残面**=_Continued 待端口（最后一块） |

## 3. 已端口件索引（sound/mod.rs——纯函数+TDD 在位）

`music_channel_vm_step`（VM 三分派+重复机）/`initialize_music`（13B 头载）/
`xy_music_init`（W1 禁扫双写+duty 对称+Cont 旗）/`update_volume_envelope`+
`update_all_volume_envelopes`（包络机——$EC/$ED 指针面）/`update_all_music_channels_entry`
（ResetVolumeIndex+ThisSoundChannel）/`check_music_flags`/`find_music_init_index`/
`multi_music_index_step`。

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
