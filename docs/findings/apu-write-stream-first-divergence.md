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
