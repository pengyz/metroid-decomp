# 音乐初始化：旗位值 ↔ MusicRoutineTbl 下标互为倒序（2026-11-20）

## 结论

`MusicInitFlags`（`$0685`）的**位值**与 `MusicRoutineTbl`（`$BC26`）的**下标**
是**倒序**映射：`bit0 → idx7`、`bit1 → idx6`、…、`bit7 → idx0`。

原因在 `FindMusicInitIndex ; $BC53`（`refs/m1disasm/SRC/sound_engine.asm:2129`）：

```asm
FindMusicInitIndex:
    lda #$FF
    sta MusicInitIndex
    lda CurrentSoundFlags
    beq @RTS
@loop:
    inc MusicInitIndex      ; ← inc 在 asl 之前，且从 $FF 起算
    asl
    bcc @loop               ; ← 数的是「左移到最高位」的步数
@RTS:
    rts
```

`inc` 先于 `asl`、起点 `$FF` ⇒ 对 `0x01` 要 `asl` 8 次才把 1 送进 carry，
`MusicInitIndex` 走 `FF→00→01→…→07`，得 **idx=7**。

对照实读：

| 项 | 值 |
|---|---|
| `music_Brinstar`（`constants.asm:188`） | `%00000001` = `0x01` |
| `AreaMusicFlag` @ bank1 `$95CD`（ROM 实读） | `0x01` |
| ⇒ `MusicInitIndex` | **7** |
| `MusicRoutineTbl[7]`（`sound_engine.asm:2091`） | `GotoMusic03Init` |
| `Music03Init`（`:2193`） | `lda #$34` |

emu 在 `case-crawlprobe` f346 把 `$0628`（`MusicSQ1DutyEnvelope`）写成 `0x34`，
与上表**逐项吻合**。

## 为什么记这一条

`MusicRoutineTbl` 的注释块写 `#$00=Ridley … #$07=Brinstar`，而 `music_*` 常量
写 `music_RidleyArea=0x80 … music_Brinstar=0x01`。两者**看起来矛盾**
（`0x01` 对应 "idx0=Ridley"），实际并不矛盾——位值与下标互为倒序。

我按「bit 位置 = 下标」推算时得出了「ROM 自相矛盾」的错误结论，并据此判过一轮
`Music03Init` 的归属（见 `docs/findings/apu-write-stream-first-divergence.md` §6 的
定谳过程）。此处留档，避免再次误判。

## 已排除

`crates/game-core/src/sound/mod.rs:178 find_music_init_index_writes` 的实现
（`0xFF` 起算、`inc` 在 `asl` 前、`add8` 特例）与 asm **逐字一致**，链路无罪。

## 未决（下一步）

`case-crawlprobe` f346 的 `duty1` 在 emu 为 `0x34`、rust 仍 `0xB6`，而帧缝处
`TablePtr=0xBC26` / `SoundRoutinePtr=0xBC4B` / `MusicInitIndex=0xFF` 两腿逐字节相同
⇒ rust 在该 NMI 里没看到 `MusicInitFlags≠0`。嫌疑收敛到
`crates/game-core/src/frame/tick.rs:620 start_music` 的调用时机或写入值。
