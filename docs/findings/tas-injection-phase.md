# TAS 注入相位取证：`$4016` 读取时序与跨影片活/死对照

任务 #105（tsk_01M8BTB5QSYT4NZ7MSBRPN）。日期 2026-11-26。
本文件只读取证：不改 `tools/`、`crates/`、判据、`data/status`。临时产物在 `analysis/tas-audit/`（gitignored）。

## 0. 结论摘要

1. **读取相位已定位（A 级）**：ROM 每帧在 NMI 入口 `07:C0D9` 只用一次机会读键——`lda NMIStatus / bne` 为真时整段跳过读键；否则 `07:C0FC jsr ReadJoyPads`（`07:C215`），锁存与 8 次串行读在 `07:C21F`–`07:C22D`（读指令 `07:C22B lda $4016,x`，字节 `BD 16 40`）。
2. **Mesen 注入与 ROM 采样同帧（A 级）**：注入事件（`inputPolled`）恒落在 `$C0D9`（PC=49369），首个 `$4016` 读指令恒在 `$C22B`（`BD 16 40 lda $4016,x`；Mesen 回调回报的 `pc` 是取指后地址 `$C22E`=49710），两者周期差中位数 **808**（min 807 / max 2135），远小于一帧 29780 周期。**不存在 1 帧滞后**。
3. **相位不是根因（A 级）**：同一条 `lordtom-metroid-100p.fm2` 直接交给 FCEUX 2.5.0 播放，6000 帧内**不死**（血量 30→22→14→19→11）；同片经 inputlog 注入 Mesen2，**f3889 死亡**（30→22→14→6→0）。首次掉血就差 431 帧（FCEUX f2190 / Mesen f1759）。⇒ 分歧发生在引擎执行层，不在输入交付相位。
4. **Mesen 腿无法投递 2P（A 级，带读数）**：`emu.setInput(...,1)` 后引擎内部 `stateBuffer=255`，但 ROM 对 `$4017` 的 8 次读**恒为 `0x40`、不随投递内容变化**（`0x40` 位序上是 B 位，恒置位＝未接手柄的开路电平，不是按键）；再次调用还会覆盖端口 1。`eien86` 片在死亡帧之前就用到 2P（首 2P 帧 3266 < 死亡 3863），故其 Mesen 死亡**不能**作 100% 片对照。
5. **语料口径问题**：`movies/metroid-anyp-3666.inputlog` 只有 3260 数据行，而 bk2 头部声明 28690 帧——据 `data/status/movie_ledger.json` 该片是 **pilot 主动截取的 3260 行窗口**（`window_used=3260`、`frames_synced=1039`），不是转换故障，但拿来当「整片」对照无效；`movies/eien86-6725M.inputlog` 头部 `# source` 写成 anyp 的 bk2（内容与 anyp 不同，属头部错标）。#103 里「any% 在 f4000 仍活」的对照因此不成立（见 §4.3）。

**可直接用的结论**：Mesen 腿与 Rust 腿逐帧相等，只能证明「两支实现彼此一致」；它**不能**证明「本片可通关」。唯一可信通关参照是 FCEUX 原生回放 lordtom 片。

## 1. 读取相位（静态，A 级）

`refs/m1disasm/SRC/prg7_engine.asm`：

```asm
NMI: ; 07:C0D9
    php / pha / txa / pha / tya / pha
    lda #$00
    sta OAMADDR
    lda #>SpriteRAM
    sta OAMDMA
    lda NMIStatus          ; $1A
    bne @endIf_A           ; ← 上一帧没跑完 ⇒ 跳过下面整段（含读键）
        ...
        jsr WritePPUCtrl   ; 07:C44D
        jsr WriteScroll    ; 07:C29A
        jsr ReadJoyPads    ; 07:C215   ← :268
    @endIf_A: ; 07:C103
    jsr SoundEngine        ; 声音在"是否跳过"之外，总会跑
    jsr UpdateAge
    ldy #$01
    sty NMIStatus          ; NMI 结束标记
```

```asm
ReadJoyPads: ; 07:C215
    ldx #$00
    stx $01
    jsr ReadOnePad
    inx
    inc $01
    jsr ReadOnePad        ; 端口 2
ReadOnePad: ; 07:C21F
    ldy #$01
    sty JOY1              ; $4016 写 1
    dey
    sty JOY1              ; $4016 写 0 ⇒ 锁存
    ldy #$08
@loop: ; 07:C22A
    lda JOY1,x            ; ← 指令 07:C22B（BD 16 40）；回调 pc=07:C22E（取指后）
    ...                   ; 移位进 A
    dey
    bne @loop
    sta Joy1Status,x      ; 零页 $14(pad1) / $15(pad2)
```

要点：

- **每帧只有一次采样窗口**，在 NMI 里、OAM DMA 之后、声音引擎之前。
- 若上一帧主循环没有在 NMI 到来前跑完（`NMIStatus≠0`），本帧**整段不读键**，`Joy1Status` 保持旧值。
- `NMIStatus` = `$1A`（`constants_ram.asm:496`，「0=NMI in progress. anything else, NMI not in progress.」）；主循环在 `@wait: lda NMIStatus / bne`（`07:C0D3`）等 NMI 结束。
- 位序由硬件串行移位决定，软件侧位定义见 `hardware.asm:89-96`：Right=01/Left=02/Down=04/Up=08/Start=10/Select=20/B=40/A=80。

## 2. Mesen 注入相位实测（A 级）

探针 `analysis/tas-audit/probe_4016.lua`：`emu.addMemoryCallback(..., emu.callbackType.read, 0x4016, 0x4017)` 记读点 PC；`emu.addEventCallback(..., "inputPolled")` 记注入点 PC 与 `cpu.cycleCount`。

实跑 120 帧（同一片）：

| 观测量 | 读数 |
|---|---|
| `inputPolled` 的 PC | 恒 `49369` = `$C0D9`（NMI 入口） |
| `$4016/$4017` 读点 PC（回调值，取指后） | 恒 `49710` = `$C22E`；对应指令 `$C22B` `BD 16 40`（`lda $4016,x`） |
| 每帧读次数 | 16（`$4016` 8 次 + `$4017` 8 次） |
| poll → 首个读的周期差 | 中位数 **808**，min 807，max 2135（首帧例外） |

⇒ 注入落在「NMI 入口那一刻」，ROM 在 808 周期后采样；两者同属本帧。**整帧滞后假设排除。**

2P 探针 `analysis/tas-audit/probe_pad2.lua`：

- `emu.setInput({全键}, 1)` 后 `controlManager.controlDevices[1].stateBuffer = 255`（引擎内部认为已写入）；
- 但 ROM 对 `$4017` 的 8 次读**恒为 `0x40`，与 `setInput` 投递了什么键无关**（`0x40` 是 Bit6，位序上对应 B 位，但恒置位＝未接手柄的开路电平，不是被投递的按键）；
- 随后再调用一次 `emu.setInput(..., 0)`，`$4016` 的 8 次读**全变成 `0x41`**（Right+B 恒置）⇒ API 把第二次调用错误地落到端口 1（正常应随按键变化）。

⇒ **Mesen 2.1.1 腿不能投递 2P 输入**。这条从注释升级为带读数定谳。

## 3. 相位试验

### 3.1 整表平移（已做，否证）

| 试验 | 输入产物 | 结果 |
|---|---|---|
| 基准 | `movies/lordtom-metroid-100p.inputlog` | Mesen 死亡 **f3889** |
| 后移 1 帧 | 前插一行 → `lordtom-shift-minus1.inputlog` | 死亡推到 **f5145**；f19999 仍 `InArea=0 / health=0` |
| 前移 1 帧 | 删首行 → `lordtom-shift-realminus1.inputlog` | f4599 时血量已 0 |

平移只把死亡帧挪动，不救活 ⇒ 「整体差一帧」不成立。

### 3.2 亚帧相位（未做，给出设计）

真正的亚帧相位旋钮 `scenario_run.py` 目前没有：注入点固定为 `inputPolled`（§2）。若要验证「808 周期窗口内的注入位置」，需给 harness 加参数（§9）：

```text
--input-phase <cycles>   注入延后 N 个 CPU 周期（相对 NMI 入口，默认 0）
```

期望读数：N ∈ [−808, +808] 内每 16 周期一档，Mesen 死亡帧不变；若在某档跳变，才说明采样窗口敏感。

### 3.3 用参照引擎替代相位试验（已做，判死）

既然相位被 §2 排除，改用**同片不同引擎**直接对照，见 §5。结论是相位无关、引擎相关。

## 4. 跨影片活/死对照

### 4.1 同一把尺

- 解码器：`tools/movie_import.py`（inputlog 唯一编解码）。
- 注入器：`tools/scenario_run.py` + `tools/scenario_capture.lua`（`emu.setInput`）。
- 参照引擎：FCEUX 2.5.0 原生 `.fm2` 回放（不经过 inputlog）。

### 4.2 对照表

| 影片 | 引擎/长度 | sha256（前 16） | Mesen 注入结果 | FCEUX 原生结果 |
|---|---|---|---|---|
| `movies/lordtom-metroid-100p.fm2` | FCEUX 9816 / 120540 帧 | `471b441cb2368c95` | **f3889 死亡**（30→22→14→6→0） | **6000 帧不死**，f5639 血量 11 |
| `movies/eien86-6725M.inputlog` | quickerNES，provenance `frames=28408`（inputlog 28690 行） | `9ae2aa9c2e619f2a` | f3863 死亡（#103） | 无 `.fm2`，**无法回放** |
| `movies/metroid-anyp-3666.inputlog` | NesHawk，pilot 窗口 **3260 行**（源 28690 帧） | `0312bbec11e471dd` | f3260 后输入耗尽 | 无 `.fm2`，无法回放 |

ROM：`rom/Metroid (USA).nes` sha256 `e6e6b7014685adae…`；lordtom fm2 的 `romChecksum` 解码后与 ROM 无头 md5 一致（#103 已验）。

口径说明：三片长度不是一个量级——lordtom 120540 帧（约 33 分钟，100% 片）、eien86 28408 帧（约 8 分钟，`quickerNES`）、anyp 只有 pilot 窗口 3260 行。**只有 lordtom 是 100% 片**；另外两片不能用来论证「100% 片都会死」。

### 4.3 对 #103 结论的更正

- **any% 对照作废**：`metroid-anyp-3666.inputlog` 只有 3260 数据行，是 pilot 截取的窗口（`data/status/movie_ledger.json`: `window_used=3260`、`frames_synced=1039`；源 bk2 声明 28690 帧）。Mesen 跑到 f3260 后输入耗尽；#103 的「any% 在 f4000 仍活着」测的是**窗口结束后的静止输入**，无对照意义。
- **eien86 头部错标**：`eien86-6725M.inputlog` 的 `# source` 写成 `movies/metroid-anyp-3666.bk2`；其内容与 anyp 不同（第 227 行起分叉），属头部字段错误，数据本身是 eien86 的。
- **eien86 的死亡不能归因为引擎差异**：其首 2P 帧 3266 < 死亡 3863，而 Mesen 无法投递 2P（§2）⇒ 该片在 Mesen 上的死亡有充分的其他解释。若要作 100% 对照，需 FCEUX 能读的 `.fm2` 版本。
- 因此「两片 100% TAS 都死」不成立；成立的只有 lordtom 一片，而它在 FCEUX 上活着。

## 5. FCEUX 参照腿

### 5.1 环境缺陷与绕过

系统包 `/usr/games/fceux`（2.5.0+dfsg1）在本容器**任何参数都崩**：

```text
*** buffer overflow detected ***: terminated
gdb 栈：__realpath_chk ← fceuWrapperInit ← main (SIGABRT)
```

三种 `QT_QPA_PLATFORM`（xcb/offscreen/minimal）都无效。生效的绕过是 LD_PRELOAD 垫片：

```c
/* /tmp/shim.c */
#include <stdlib.h>
#include <stddef.h>
char *__realpath_chk(const char *path, char *resolved, size_t len) {
    (void)len;
    return realpath(path, resolved);
}
```

```bash
gcc -shared -fPIC -o /tmp/shim.so /tmp/shim.c
```

### 5.2 可复跑命令

```bash
LD_PRELOAD=/tmp/shim.so QT_QPA_PLATFORM=offscreen \
FCEUX_LOG=/tmp/fx_lordtom.log FCEUX_MAXF=6000 \
/usr/games/fceux --no-config 1 --sound 0 --pal 0 \
  --playmov "movies/lordtom-metroid-100p.fm2" \
  --loadlua analysis/tas-audit/fceux_watch.lua \
  "rom/Metroid (USA).nes"
```

`analysis/tas-audit/fceux_watch.lua` 用 `emu.registerafter` 逐帧读 `$0106/$0107/$0074`，输出 `<帧> <lo> <hi> <InArea>`。

### 5.3 读数

| 帧 | FCEUX 血量 | Mesen 血量 |
|---|---|---|
| f30 / f28 | 30 | 30 |
| f2190 / f1759 | 22 | 22 |
| f3080 / f2148 | 14 | 14 |
| f5411 / f3244 | 19 | 6 |
| f5639 / f3889 | 11 | **0（死）** |
| f6000 | 11（仍活） | —（已死） |

解码式（`constants_ram.asm:826-830`）：`health = ((hi & 0x0F) * 10) + (lo >> 4)`。

读数说明：
- f14→19 的回升按同一解码式读出（f5411 `lo=90 hi=01`），成因未展开（可能是罐/进位编码的边缘），不影响本条对照——关键是 **f3889 时 FCEUX=14、Mesen=0**，且 f6000 时 FCEUX 仍非 0。
- FCEUX 全程 `InArea=0`（Brinstar，和 Mesen 一致）；f26–f29 的 0 是开机未初始化，非死亡。

⇒ 两条曲线的**掉血时刻**从第一次就差 431 帧，死亡差 570 帧。这不是采样相位能解释的量级。

## 6. 首分歧观测量（免责说明）

`analysis/tas-audit/scn-div1.txt`（2300 帧，watch `$0300-$030F`/`$0400-$041F` + 零页）与 FCEUX `fceux_dump.lua` 同帧同址对照：

- Mesen 采样点 `spin-exit`（`$C0D3`，主循环等 NMI 结束）与 FCEUX `emu.registerafter`（帧末）**不是同一时点**，`$0300` 状态转移常差 1 帧（f166 Mesen 已 15、FCEUX 仍 14）。
- `$030E`（Samus X）自 f660 起反复差 1px（交替帧），f674/f686 起差 3px 并放大。
- 因此这些逐字节差**只能**判定「早期即分叉」，不能当作物理 bug 的定点证据；要定点必须两侧同缝采样（§9 建议）。
- 附带观测：FCEUX 开机未初始化单元为 `0xFF`，Mesen `Nes.RamPowerOnState=AllZeros`。但 ROM `Startup`（`07:C01A`）用 `ldy #$07 … @loop_A` **清空 $0000-$07FF 全部 2KB**，故上电 RAM 内容不影响游戏（此假设否证）。

## 7. 根因层判定

| 层 | 判定 | 依据 |
|---|---|---|
| 影片解码 | 排除 | fm2 ↔ inputlog 120540 行零差（#103） |
| 注入 API / 帧相位 | 排除 | §2：同帧、808 周期窗口；§3.1：整表平移不救 |
| 2P 注入 | **确认缺口** | §2 探针：`$4017` 恒 0x40 |
| 上电 RAM | 排除 | §6：Startup 清 2KB |
| 回放引擎执行语义 | **所在层** | §5：同片 FCEUX 活 / Mesen 死，掉血差 431 帧 |

下一层假设（按可检验性排序）：

- **H1（首选）丢帧决策不同**：主循环若在某帧超时，NMI 会跳过读键（§1）。Mesen 与 FCEUX 对「该帧是否超时」的判定若不同，输入就会在某几帧被丢弃/重复，轨迹随之分叉。检验：两侧统计「NMI 入口 `$1A≠0` 的帧号序列」，比较前 2300 帧；需要 §9 的 `--probe-lag`。
- **H2 CPU/PPU 上电对齐**：Mesen `RandomizeCpuPpuAlignment=False` 固定对齐；FCEUX 缺省对齐可能不同，导致偶发帧的可用周期差 1–2 cycles。可由 H1 的序列间接区分（若丢帧集合相同则 H2 可疑）。
- **H3 APU/DMC 周期抢占**：声音引擎每帧在 NMI 里跑；若 DMC DMA 抢占周期数不同，主循环可用周期变化 ⇒ 与 H1 同一后果。检验：两侧统计 `$4015` DMC 启用与 DMC 触发次数。

## 8. 复跑命令与期望读数

```bash
# ① Mesen 侧健康曲线（预期：2881 帧内 30→22@1759→14@2148→6@3244→0@3889）
python3 tools/scenario_run.py run analysis/tas-audit/scn-pw.txt

# ② FCEUX 侧健康曲线（预期：f2190=22、f3080=14、f5411=19、f5639=11、6000 帧不死）
#    命令见 §5.2；然后解码
python3 - <<'PY'
def dec(lo,hi): return ((hi&0x0F)*10)+(lo>>4)
prev=None
for ln in open('/tmp/fx_lordtom.log'):
    p=ln.split()
    if len(p)<3: continue
    h=dec(int(p[1]),int(p[2]))
    if h!=prev: print(f'f{p[0]}: {h}'); prev=h
PY

# ③ 注入相位（预期：poll pc=49369、read pc=49710、每帧 16 读、delta 中位数 808）
#    探针 analysis/tas-audit/probe_4016.lua，经 Mesen 载入

# ④ 2P 投递（预期：$4017 8 次读全 0x40）
#    探针 analysis/tas-audit/probe_pad2.lua
```

## 9. `scenario_run.py` 最小修法建议（本卡不改代码）

1. `--input-phase <cycles>`：把 `setInput` 从 `inputPolled` 延后 N 个周期再调用（默认 0 保持现状），用于 §3.2。
2. `--probe-lag <path>`：在 `inputPolled` 里先读 `$1A` 再注入，逐帧写 `帧 $1A`；另一侧在 FCEUX `registerbefore` 读同址，得到 H1 的对照序列。
3. 场景字段 `port: 2`：当 `setInput` 目标端口为 1 时**直接报错**而不是静默覆盖端口 1（当前行为会污染 P1，`$4016` 读全变 `0x41`）。
4. `movie import` 结束时对 `--inputlog` 做长度校验：数据行数 ≠ 源影片帧数时默认非零退出，并支持显式 `--window N` 声明窗口（pilot 截取场景），把「主动截取」与「意外截断」区分开。

## 10. 假设区（C 级）与数据卫生

- C1：H1 的「丢帧集合不同」是主因——目前只有 `$1A` 在 spin-exit 恒 1 的旁证，未取到 NMI 入口序列。
- C2：FCEUX 与 Mesen 的 P1 采样窗口在本片前 660 帧内等价（`$0300-$030F` 除 1 帧相位外逐字节相同），故分歧起点在 f660 之后、f1759 之前。
- C3：`movies/eien86-6725M.inputlog` 的 `# source`/`# core` 头部错标（写成 `metroid-anyp-3666.bk2` / NesHawk）与 provenance（`quickerNES`、`frames=28408`、sha `4421f6e3…`）矛盾，成因未展开；`movies/lagshift-plus1-f1756.inputlog` 同族产物也应一并复核。

## 11. 【2026-11-27 更正】根因是录像复位帧未被消费（A 级），不是 NMI/APU/2P

> 本节推翻 §3「注入相位不是根因」的结论与 §4「2P 缺口」对本次死亡的因果解释，按纪律原地保留原文。

### 11.1 数据面事实（A 级，可机械复算）

`movies/lordtom-metroid-100p.inputlog` 共 120540 行，**cmd 列（第 3 字段）非零的只有 1 行**：

```
python3 - <<'PY'
rows=[l.split() for l in open('movies/lordtom-metroid-100p.inputlog') if l.strip() and not l.startswith('#')]
print(len(rows), [(i+1,r) for i,r in enumerate(rows) if r[2]!='00'])
PY
# 120540 [(1, ['00', '00', '01'])]
```

即 **f1 的 `00 00 01` = 软复位帧**。`tools/movie_import.py` 把这个 cmd 写进 inputlog（写出端正确），
但 `tools/scenario_run.py::expand_pads` 的消费循环写成
`for f, (p1, _p2, _cmd) in log_table.items():` —— **`_cmd` 读而不用**，
复位帧被当成普通输入帧喂给游戏。

FCEUX 播 `.fm2` 时，复位帧只执行复位、**不产生一帧游戏输入**；后续每一帧录像输入
都往后错开一帧。旧实现把这帧当输入 ⇒ **整卷输入相对影片晚一帧**，
这就是「注入相位」的真实来源（在工具层，不在 Mesen 的 `inputPolled` 时机）。

2P 与本死亡无关的旁证：p2 列**全片仅 11 帧非零、首帧 f22450**，晚于本次死亡帧 f3889。

### 11.2 修法与前后对照（同一 scenario、只改这一处）

`tools/scenario_run.py::expand_pads`：遇到 `cmd & 0x03`（软/硬复位）`reset_shift += 1` 并跳过该行，
其余行写入 `shifted[f + reset_shift]`。`tools/l4_diff_run.py:1239/:1360` 都调本函数
（唯一展开源），所以 Rust 腿 `.sched` 与 Lua 腿 `PLAN.schedule` 同享修正。

```
python3 tools/scenario_run.py run /tmp/r111/scn-fix.txt     # frames 6000
# → analysis/traces/tas-111-fix.frames.jsonl
```

解 `hp=((hi&0x0F)*10)+(lo>>4)`：

| 片源 | 首伤 | 第二伤 | 第三伤 | 死亡 |
|---|---|---|---|---|
| 旧实现（相位 0） | f1759 (22) | f2148 (14) | f3244 (6) | **f3889** |
| 本修法（消费复位帧） | **f2192 (22)** | f2518 (17) | f4843 (9) | f5145 |
| FCEUX 参照（`/tmp/fx_lordtom.log`，6000 帧不死） | **f2190 (22)** | f3080 (14) | f5411 (19) | 不死 |

结论分两层：

1. **首伤与 FCEUX 对齐到 2 帧**（2192 vs 2190；Δ2 是 `sample_point` 与 FCEUX `registerafter`
   的采样约定差），证明「整卷晚一帧」这个系统性偏移被消除。f3889 / f4000 已能通过。
2. **f2518 之后仍有残余分歧**（我们多掉 5 HP，并最终在 f5145 死亡）：输入相位不再是唯一差异，
   剩余分歧属 H2/H3（CPU/PPU 上电对齐、APU/DMC 抢占）一类**模拟器执行语义**，
   不是工具层一次性可解的；本卡不把「整片 6000 帧不死」当成已达成。

### 11.3 §9 建议状态更新

| 建议 | 状态 |
|---|---|
| ① `--input-phase <cycles>` 延后 `setInput` | **不需要**：根因是录像复位帧未消费，已在 `expand_pads` 修掉（这一处同时覆盖 emu/Rust 双腿） |
| ② `--probe-lag` | 未做（残余分歧定层时仍可能有用） |
| ③ `port: 2` 显式报错 | 未做；2P 与本死亡无关（p2 首帧 f22450） |
| ④ inputlog 长度校验 | 未做（本片长度 120540 与 `movie_ledger` 记录的可用帧一致） |

### 11.4 回归与影响面

- `python3 -m pytest tools/tests/test_expand_schedule.py -q` → **27 passed**（新增
  `test_expand_schedule_consumes_movie_reset_frame`：含 cmd 的录像后移一帧、无 cmd 的录像锚不漂移）。
- 影响面：**只影响带 `input_log` 且含复位 cmd 的场景**（即 TAS 回放族）；手写 DSL 用例
  schedule 字节不变。含录像的旧 trace/golden 会整体位移一帧，重跑后需重新绑定基线。
