# APU 写流对拍：仪器噪声过滤与首分歧（$4000 duty/volume）

日期：2026-11-20。仪器：`tools/apu_diff.py`（逐帧比 `$4000-$4017` 写序列）。
样本：`cases/case-crawlprobe.txt`（L4 6/6 全 0px，即**像素面完全一致**的用例）。

## 1 仪器层修正（本次）

- Mesen 侧 `emu.callbackType.write` 把 CPU 写**所有**地址都抛回来，`$4016`（手柄
  strobe，`ReadJoyPads ; 07:C215` 每帧写两次）因此混进 APU 桶，被当成"emu 独有写"。
  已在 `tools/apu_diff.py` 加 `NOISE_ADDRS = {0x4016}` 过滤并打印过滤条数；
  `$4017` 是**真** APU 帧计数器，不过滤。
- 新增「地址写次数对照」表：判「每帧都写」还是「音符边界才写」的直接读数。

## 2 首分歧（未定谳）

过滤后同一批桶的首分歧：

```
首分歧帧 f420：写数 emu=3 rust=4
  #2: emu=$4000 34   rust=$4000 B6
  #3: emu=（无）     rust=$4004 10
```

`$4000` 位义 `DDLC VVVV`：

| 侧 | 值 | duty | halt | const-vol | 音量 |
|---|---|---|---|---|---|
| emu（原版） | `$34` | 00 | 1 | 1 | 4 |
| rust | `$B6` | 10 | 1 | 1 | 6 |

⇒ **我方写进去的是「duty=2、音量=6」，原版是「duty=0、音量=4」**。这不是"多写一条"，
而是**写错了值**；同时 rust 多写一条 `$4004`（pulse2 音量）。

## 3 未证实的假设（列出以免被当成结论）

1. 我方 `c0` 的来源可能是 `MusicSQ1DutyEnvelope($0628)` 的**装载时机**与原版不同
   （原版在 `MusicSQ1Init` 期写 `$4000`，之后由包络机接管；我方每步都写）。
2. `$4004` 多写一条，可能是「包络索引≠0 应跳过音量写回」的门（`music.rs` 的
   `if ch == 2 || ctx.volume_env_index == 0`）在通道 1 上判错。
3. 也不能排除「两侧音乐起步不同步」——6 个桶只覆盖 f960–1370，样本窄。

## 4 下一步

- 逐帧读两侧 `$4000-$400F` 写次数与值（本仪器已能输出），确认是"值错"还是"相位错"。
- 对读 `MusicSQ1Init`/`UpdateMusicChannel` 的 `SQ1_VOL` 写点（`sound_engine.asm`
  的 `$B3xx-$BCxx` 区）与 `crates/game-core/src/sound/music.rs` 的 `c0` 计算链。
- 判据不许放宽：APU 写流等价是"声音对不对"的硬判据，像素面 0px 不能替代它。

## 5 后续取证（2026-11-20 补）

1. **活路径已核实无误**：`UpdateAllMusicChannels ; $BAA5` 在四通道轮完后 `beq
   GotoUpdateAllVolumeEnvelopes` → `jsr UpdateAllVolumeEnvelopes($BA37)`；我方对应
   `crates/game-core/src/sound/music.rs::update_all_music_channels` 的包络尾
   `update_volume_envelope_frame`，公式已正确实现
   `(duty & 0xF0) | 包络字节`（`music.rs:589`）。⇒ **公式不是分歧点**。
2. 因此 `$4000 = $B6` 只能解释为 `duty` 原始值本身是 `$B6` 且包络字节恰为 `$06`
   （`$B6 & 0xF0 | $06 = $B6`）；而原版是 `duty=$3?`、包络字节 `$04`。⇒ 分歧在
   **duty 来源与包络步进起点**，不是写回逻辑。
3. `sound/mod.rs::update_volume_envelope`（旧占位版，取 `env_bytes` 末字节当音量）**不是
   活路径**（生产代码无调用者，只有单测）——已按真实表读取修正（`env_idx-1` 选表、
   `volume_index` 选步），并更正单测期望 `$9C→$9D`（第 3 张包络表首字节 `00 0D 09 07…`
   的 index=1 是 `$0D`）。**注意：该修改对实际音频无影响**（导出 WAV 逐字节不变，
   md5 `331c4f8e…` 前后相同）——不要把它当成"修好了声音"。
4. 下一步（未做）：逐帧 dump 两侧 `MusicSQ1DutyEnvelope($0628)`、`MusicSQ1VolumeEnvelopeIndex
   ($062E)`、`MusicSQ1VolumeIndex($066A)`，定位是「duty 字节来源」还是「包络起点差一」。

## 6 定谳进展：首个音乐状态分歧在 f346，rust 漏了第二次曲目 init（2026-11-20）

样本：`cases/case-tmp-apu-env.txt`（`case-crawlprobe` 副本 + `watch_range 0620-0670`，L4 仍 6/6 0px）。
两侧逐帧对读 `$0628/$0629`(DutyEnvelope) `$062E/$062F`(VolumeEnvelopeIndex) `$066A/$066B`(VolumeIndex)：

| 帧 | emu | rust |
|---|---|---|
| 343–345 | d1=`B6` d2=`F6` e1=`02` e2=`05` | 同 emu |
| **346 起** | d1=`34` d2=`34` **e1=`02` e2=`00`** | 仍 `B6/F6/02/05` |

⇒ **f346 原版执行了一次新的曲目 init**（`SetVolumeAndDisableSweep` 装载 X=Y=`$34`、
SQ2 音量包络清零），rust **没有执行**，整个音乐状态块就停在上一首的状态上。此前
`$4000` 的 `$34` vs `$B6` 分歧只是这条的**下游症状**。

`crates/game-core/src/sound/hook.rs:246` 的 `Music03Init => (0x34,0x34)` 映射本身是有的
（`BC96 lda #$34`），所以问题在**派发条件**（`$065E` 曲号 / `$0685` init 旗 / 10th cycle
时序），不在表值。

## 7 下一步（具体）
1. 在 f340–350 窗内对读 `$065E`(FindMusicInitIndex)、`$0685`(MusicInitFlags)、
   `$065D`(MusicLoop) 三面，定谳 rust 为何没进第二次 `Music03Init`。
2. 判据不变：修好后同一桶的 `$4000-$400F` 写序列应逐写相等（`tools/apu_diff.py` 报全等）。
