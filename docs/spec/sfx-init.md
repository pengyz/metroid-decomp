# B4-a：SFX 初始化与装载（`InitSFXData` / `LoadSFXData`）

> 数据源：`rom/Metroid (USA).nes` 字节（A 级）+ `refs/m1disasm/SRC/sound_engine.asm`（结构/命名，线索，已逐条对字节）。
> 本卡不跑 Mesen2：APU 写序由 `Manager::write_io` 的 `apu_log` 捕获，不造像素判据。

## 0. 结论速览

| 问题 | 结论 | 级别 |
|---|---|---|
| 几份实现？ | **各 1 份**：`LoadSFXData` `$B368`（0x27=39 B）、`InitSFXData` `$B452`（0x41=65 B），都在 `bank0–05` 的 shared 段，六份**逐字节相同** | A |
| `LoadSFXData` 有几个入口？ | 4 个：`@SQ1 $B368`、`@Tri $B36C`、`@Noise $B370`、`@SQ2 $B374`，各 `lda #<APUReg` 起跳 | A |
| 通道映射 | `ct 0=Noise $400C / 1=SQ1 $4000 / 2=SQ2 $4004 / 3=Tri $4008`（ct 来自 `ChannelType $065C`） | A（七案证反转，模块内注） |
| 入参/返回 | A=帧数、Y=记录低字节（高字节恒 `$B2`）、X 进入后被 `ldx ChannelType` 覆盖；出口 A=0、X=ct、Y=4 | A |
| `InitSFXData` 的调用者 | asm 内 **15 处**直接 `jsr/jmp InitSFXData`；其中 2 处是 Goto 包装（`GotoInitSFXData $B587`、`GotoInitSFXData2 $B739`）⇒ **13 个 SFXInit 例程**（与 backlog §2 条目 21 的「13 个调用者」一致） | A |
| `LoadSFXData` 的取数基座 | `SFXData` 表基址 `$B200`，记录 4 B；`Y` 只给低字节 ⇒ 任何记录址必在 `$B200–$B27F`（`$B285` 的 `@Door` 也落在此窗） | A |

## 1. 六 bank 同址核对（A 级）

`bank0–05` 的 shared 段在 `$B000` 一带相同（`sound_engine` 被六 bank 共同 include）。`bank6` 的
`$B368/$B452` **是图形/数据**，不是这两段代码（它不是「第六份实现」，是「这个地址在 bank6 无代码」）。

| bank | `LoadSFXData $B368` 长度 | sha256[:16] | `InitSFXData $B452` 长度 | sha256[:16] |
|---|---|---|---|---|
| 00 | 39 B | `398a98140612afe2` | 65 B | `a7b95cba4bd040f8` |
| 01 | 39 B | `398a98140612afe2` | 65 B | `a7b95cba4bd040f8` |
| 02 | 39 B | `398a98140612afe2` | 65 B | `a7b95cba4bd040f8` |
| 03 | 39 B | `398a98140612afe2` | 65 B | `a7b95cba4bd040f8` |
| 04 | 39 B | `398a98140612afe2` | 65 B | `a7b95cba4bd040f8` |
| 05 | 39 B | `398a98140612afe2` | 65 B | `a7b95cba4bd040f8` |
| 06 | 无代码（图形数据 `3c3cfcecfcfc0000…`） | — | 无代码（`407f000000ff3f30…`） | — |

复跑：

```bash
python3 - <<'PY'
import hashlib
rom=open('rom/Metroid (USA).nes','rb').read()
off=lambda b,a:16+b*0x4000+((a-0xC000) if a>=0xC000 else (a-0x8000))
for name,addr,n in [('LoadSFXData',0xB368,0x27),('InitSFXData',0xB452,0x41)]:
    print(name, hex(addr), n, 'B')
    for b in range(6):
        print('  bank', b, hashlib.sha256(rom[off(b,addr):off(b,addr)+n]).hexdigest()[:16])
PY
```

差异处理：六份 0 字节差、无分支 ⇒ 端口就是**一份 Rust 实现**；将来有差必须在表里逐字节列出，不套用同址。

## 2. `LoadSFXData` 语义（A 级，逐行）

`$B368` 起四个入口按 `lda #<REG` 汇合到 `@Common`：

1. `APURegisterPtr($00E0/$00E1) ← 通道基址`（`$4000/$4004/$4008/$400C`）。
2. `SFXDataPtr($00E2/$00E3) ← $B2 : Y`（`Y` 即记录低字节）。
3. 循环 4 次：读 `(SFXDataPtr),Y` 写 `(APURegisterPtr),Y` ⇒ **4 字节间接写**到 `基址..基址+3`；
   `Y` 走完为 `4`（出口 Y=4）。
4. `rts`（`@Tri` 另行把 `$00` 写入 `TRI_LINEAR`/`DMC_RAW`——见 asm `$B36C` 分支）。

`LoadSFXData` 本身**不写** `$0660+ct` 等通道状态；那些是 `InitSFXData` 的活。

## 3. `InitSFXData` 语义（A 级，逐行 `$B452`）

```
ldx ChannelType        ; $065C
sta SFXNoiseLength,x   ; $0660+ct ← A（帧数）
txa / beq @Noise       ; ct==0 → Noise
cmp #1 beq @SQ1 / #2 @SQ2 / #3 @Tri
rts                    ; ct>3：只写了 length 就返回
@SQ1  jsr LoadSFXData@SQ1  ; 各自汇合
@SQ2  jsr LoadSFXData@SQ2
@Tri  jsr LoadSFXData@Tri
@Noise jsr LoadSFXData@Noise
@Common:
  jsr UpdateContFlags    ; $B493：cont[$0688+ct] = (old & $00) | CurrentSoundFlags
  txa ; sta NoiseUsedBySFX,x   ; $0652+ct ← ct（A=X=ct）
  lda #0
  sta SFXNoiseFrame,x    ; $0665+ct ← 0
  sta SFXNoiseVar0,x     ; $0670+ct ← 0
  sta SFXNoiseVar1,x     ; $0674+ct ← 0
  sta SFXNoiseVar2,x     ; $0678+ct ← 0
  sta LoadMusicSQ1SQ2PeriodsFlag ; $0607 ← 0
  rts                    ; A=0, X=ct, Y=4
```

`UpdateContFlags` 的 `and #$00` 是 ROM 的死操作（清掉旧 cont 位后才 `ora`），**照建**（Stage A 保真）。

### 3.1 通道状态字段表

| 字段 | 地址 | 写者 | 语义 |
|---|---|---|---|
| `SFXNoiseLength` | `$0660+ct` | `InitSFXData` | 本段多少帧后换段（cont 例程递减） |
| `SFXNoiseFrame` | `$0665+ct` | `InitSFXData` 清 0 / `IncrementSFXFrame` | 段内帧计数 |
| `SFXNoiseVar0/1/2` | `$0670 / $0674 / $0678 + ct`（步距 **4**） | `InitSFXData` 清 0 / 各 cont 例程 | 每族自用的三字节状态（如 ScrewAttack=音高索引/两段计数） |
| `SFX*ContFlags` | `$0688+ct` | `UpdateContFlags` | 该通道的 cont 分派旗 |
| `NoiseUsedBySFX` | `$0652+ct` | `InitSFXData` | 通道被哪类 SFX 占用（= ct） |
| `LoadMusicSQ1SQ2PeriodsFlag` | `$0607` | `InitSFXData` 清 0 | SFX 抢占音乐周期装载的标志 |
| `APURegisterPtr` | `$00E0/$00E1` | `LoadSFXData` | 本次写的 APU 目标基址 |
| `SFXDataPtr` | `$00E2/$00E3` | `LoadSFXData` | 记录源指针（`$B2:low`） |

APU 目标寄存器：SQ1 `$4000–$4003`、SQ2 `$4004–$4007`、Tri `$4008–$400B`、Noise `$400C–$400F`（4 字节/通道）。

## 4. 13 个 `InitSFXData` **直接**调用者（backlog §2 条目 21）

> 口径：本节只算**直接 `jsr/jmp InitSFXData`**（或经 `GotoInitSFXData*` 包装）的例程。另有 **9 处**
> `bne GotoInitSFXData / GotoInitSFXData2` 的条件跳转（`SamusWalk / MissilePickup / EnergyPickup /
> SamusJump / EnemyHit / BulletFire / HasLongBeam / OutOfPipe×2`），它们最终也进 `InitSFXData`，
> 但不属本表的「直接调用者"。做 B4 完整调用图时要把这 9 处一起算，别把 13 当全集。

| # | 例程 | 地址 | 帧数（A） | 记录（Y） | 通道 |
|---|---|---|---|---|---|
| 1 | `SpitFlameSFXInit` | `$B50F` | 由 `SpitFlamesTbl` 族给定 | `@SpitFlame $B221` | 见 cont |
| 2 | `ScrewAttackSFXInit` | `$B52B` | `$05` | `@ScrewAttack $B211` | Noise |
| 3 | `MissileLaunchSFXInit` | `$B56E`（经 `GotoInitSFXData $B587`） | `$18` | `@MissileLaunch $B215` | Noise |
| 4 | `HasIceBeamSFXInit` | `$B75E` | 7 | 冰束记录 | SQ1 |
| 5 | `WaveBeamSFXInit` | `$B77A` | 7 | 波束记录 | SQ1 |
| 6 | `DoorOpenCloseSFXInit` | `$B7AC` | `$1F` | `@Door $B285` | Tri |
| 7 | `BeepSFXInit` | `$B7D9` | 见 asm | `@Beep $B279` | 见 cont |
| 8 | `BigEnemyHitSFXInit` | `$B7EF` | 见 asm | 见 asm | SQ1 |
| 9 | `SamusToBallSFXInit` | `$B834` | 见 asm | 见 asm | SQ1 |
| 10 | `BombLaunchSFXInit` | `$B878` | 见 asm | `@BombExplode $B219` | Noise |
| 11 | `MetroidHitSFXInit` | `$B8A7` | 见 asm | 见 asm | SQ1 |
| 12 | `SamusDieSFXInit` | `$B8D2` | `$0E` | `@SamusDie $B275` | Tri |
| 13 | `StatueRaiseSFXInit` | `$B921` | 见 asm | 见 asm | 见 cont |

表里「见 asm」= 该例程在 `InitSFXData` 之前还有若干写面（周期/divisor/var 初始化），本卡只核对
`InitSFXData` 的 A/Y 与通道；完整写序见 `sound_engine.asm` 对应行。清单的机械来源：

```bash
python3 - <<'PY'
import re
lines=open('refs/m1disasm/SRC/sound_engine.asm').read().split('\n')
lab=None
for i,l in enumerate(lines):
    m=re.match(r'^(\w+): ; \$B([0-9A-F]{3})',l)
    if m: lab=(m.group(1),m.group(2))
    if re.search(r'(jsr|jmp) (Goto)?InitSFXData2?\b',l) and not l.startswith('GotoInitSFXData'):
        print(i+1, lab[0], '$B'+lab[1], l.strip())
PY
```

输出 15 行；去掉两行 `GotoInitSFXData`/`GotoInitSFXData2` 自身的 `jmp`（它们是包装，不是第 14/15 个
调用者）⇒ 13 个 SFXInit 例程。

## 5. Rust 落点（`crates/game-core/src/sound/`）

| ROM | Rust | 形态 |
|---|---|---|
| `LoadSFXData@* $B368/B36C/B370/B374` | `sound::sfx::load_sfx_data(mem, SfxLoad)` | **typed 施加层**：`apu_register_ptr`/`sfx_data_ptr` 走 `sound::state` 访问器，4 字节写 `Manager::write_io`（APU 是真实寄存器面，不是 RAM 业务字段） |
| `InitSFXData $B452` | `sound::sfx::init_sfx_data(mem, frames, ct, record_addr, current_sound_flags)` | 写 `length/cont/used/frame/Var0-2/$0607` 全走 typed 访问器；`ct>3` 只写 `length` 后返回（与 ROM 一致） |
| 通道映射 | `SfxChannel {Noise,Sq1,Sq2,Tri}` + `from_channel_type/channel_type/apu_base` | 单一真源，`>3` → `None` |
| 旧纯函数（保留） | `mod.rs::sfx_data_init()` / `*_sfx_init()` | 入参→`HandlerOutcome` 事件表，给差分/域测用，不碰 `Manager`；本卡不动它们 |

分工原则：**纯函数管「会写什么」，施加层管「写进 Manager」**。两条实现不能各写一份语义——
施加层的每个写点都对应上表的一行，单测按行断言。

## 6. 单测（`sound::sfx::tests`，5 例）

| 测试 | 断言 |
|---|---|
| `load_sfx_data_writes_pointer_and_four_apu_bytes` | 四通道各一例：`apu_base`、记录 4 字节、`$E0/$E1` 与 `$E2/$E3`、`apu_log` 里 4 条 `(基址+i, 字节)` |
| `screw_attack_init_5_frames_noise` | 调用者②：`length[$0660]=5`、`cont=0x44`、`used=0`、`frame/Var0=0`、`$0607=0`、`$400C←$35` |
| `door_open_close_init_tri_0x1f_frames` | 调用者⑥：`length[$0663]=0x1F`、`cont[$068B]=0x80`、`used[$0655]=3`、Tri 基址 `$4008`、`$4008←$7F` |
| `samus_die_init_tri_0x0e_frames` | 调用者⑫：`length[$0663]=0x0E`、`frame=0`、`$4008←$7F` |
| `channel_type_out_of_range_stops_after_length` | `ct=4`：`length` 已写、`load=None`、Var0/frame 未清、`apu_log` 空（ROM 的早退） |

## 7. 未覆盖 / 假设区

- **未覆盖**：SpitFlame/Beep/BigEnemyHit/SamusToBall/MetroidHit/StatueRaise 六族的 A/Y 未逐条抄进本表（它们
  另有前置写面）；`Level`/`Frozen` 等状态对 `InitSFXData` 的选择在敌人侧，本卡不含。
- **假设**：`ct>3` 的分支在实机上是否可达（谁会把 `ChannelType` 置 ≥4）未取证——按 B 级处理，代码里保留早退。
- **未取证**：APU 真实发声（Mesen 音频面）未对拍；本卡只保证写序与寄存器值。
