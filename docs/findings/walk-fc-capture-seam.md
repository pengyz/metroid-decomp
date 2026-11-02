# walk 窗 FC ±1 定谳：前提翻转 + 截取缝双相位模型（2026-11-03）

> **状态：已证实**（A 级——双腿逐帧实测 + asm 逐行锚）。B 卡 l380 回归战役收口件。
> 本文档替代 B2「walk 窗 FC 计数 ±1 真差」的旧定性（**作废重判**，见 §4）。

## 0. 定谳摘要

| # | 命题 | 裁定 | 证据 |
|---|------|------|------|
| 1 | walk 窗 FC 双腿真差 ±1 | **伪**（前提翻转） | state 层 1383 帧逐帧 **0 差**（§3） |
| 2 | B2「$2D=23/24 同点真差」 | **作废**——capture 层采样缝投影 | §2 双向实测锚 |
| 3 | l380=12085px 根因 | **截取缝相位**（rust 截取点=emu 帧 N 图像相位的 1 拍后） | §2 dx 配准三点 + §2c |
| 4 | $FF bit2 582 帧差 | **真差**（V2-lite 直写未模 SetPPUInc） | §5 |
| 5 | 修径 | 截取点双相位对齐（§6a）+ SetPPUInc 补模（§6b） | lead 裁定 2026-11-03 |

## 1. asm 实录（walk 窗 FC 计数径 + 截取缝逐行依据）

### 1a. FC 唯一写点——迭代节律（非帧节律）

`refs/m1disasm/SRC/prg7_engine.asm` `MainLoop`（07:C0BC）逐行：

    :199  MainLoop:
    :201    jsr CheckBankSwitch
    :203    jsr UpdateTimer
    :205    jsr GoMainRoutine        ; 帧工作（MoveSamus/滚动/流送）
    :207    inc FrameCount           ; ← $2D 唯一写点：工作完成尾 +1（迭代节律）
    :209    lda #$00
    :211    sta NMIStatus            ; $1A=0 → 自旋等 NMI
    :213  @wait: lda NMIStatus / beq @wait
    :218    jsr RandomNumbers
    :221    jmp MainLoop

**结论（A 级）**：FC 计数=主循环迭代数；一迭代一 +1。MoveSamus 逐像素环
（LE335/LE352，ObjectCounter）每像素可走 ScrollLeft（07:E6A7）/ScrollRight（07:E6D2）
→ CheckUpdateNameTable（07:E54A）→ SetupRoom（07:EA2B 载房任务机）——
**滚动跟随逐像素 1:1，与 FC 无耦合面**。

### 1b. NMI 内块（截取缝的「渲染管序」依据）

`NMI:`（07:C0D9）逐行：

    :2157  OAMDMA                ; SpriteRAM($200-2FF)→OAM
    :2161  lda NMIStatus
    :2163  bne @endIf_A          ; ← prg7:251 bogged 门：帧未完则跳过内块
             jsr CheckPaletteWrite          ; ($C1E0)
             jsr CheckVRAMStructBufferWrite ; ($C2CA) ←VRAMStructWrite→SetPPUInc→jmp WriteScroll 尾跳
             jsr WritePPUCtrl               ; ($C44D) $2000/$2001 ← $FF/$FE 影子
             jsr WriteScroll                ; ($C29A) $2005 ← $FD/$FC（滚动写在 VRAM 冲刷后）
             jsr ReadJoyPads                ; ($C215)
    @endIf_A:
           jsr SoundEngine / jsr UpdateAge
           ... NMIStatus=1（自旋出口）...

**结论（A 级）**：帧 N 的可见区图像=NMI(N-1) 锁存态（滚动/名表/OAM 三面同源）；
NMI 内块可被 NMIStatus 门整跳（bogged 帧渲染重复上一帧）。

### 1c. SetPPUInc（$FF bit2）——列流送的控制字节面

    SetPPUInc:            ; 07:C318 ——CF=1→增量32（$FF bit2 置位），CF=0→增量1（清）
    VRAMStructWriteData:  ; 07:C2E4 ——ctrl asl→jsr SetPPUInc（CF=bit7=增量位）
    @controlBitsTable:    ; 07:E5E0
      .byte $20           ; 横写：PPU inc=1，32 块
      .byte $1E|$80       ; 竖写：PPU inc=32，30 块 ←列流送用（bit7=1）
    WritePPUAttribTbl:    ; 07:E5E2 ——控制字节 $20（横写 inc=1）⇒SetPPUInc 清位
    LC86F 载房直拷前：    ; and #$FB（PPU increment=1）——区域装载清位

**结论（A 级）**：列流送（竖写 $1E|$80）置 $FF bit2=1；属性行写（$20）与载房直拷清 0。
两腿窗口 f819-1399 emu 恒 0x97（末拍 SetPPUInc=列流送）——V2-lite 直写径不补模则恒 0x93。

## 2. endFrame 截取缝实证锚（双相位模型）

### 2a. 采样点坐标系（可复现）

`tools/scenario_capture.lua`：
- `:35-36` 帧计数语义：`frame` 在 **inputPolled** 递增；`endFrame`（帧 N 渲染完成后）写 trace 行/截图/RAM dump。
- `:627-633` spin-exit 采样（`$C0D3` exec 回调——NMI 后/帧工作前）。
- `:143` `emu.takeScreenshot`（可见区 N 的帧缓冲）；`:153` `dumpRam`（0000-07FF 活体）；`:163` `dumpPpu`（$0000-2FFF 活体+palette+getState 尾 3B）。

### 2b. 双向实测锚（case-walkfcprobe，1400 帧持左）

**锚 A（RAM 面）**：emu capture(N) ≡ emu state-row(N)；rust capture(N) ≡ rust state-row(N+1)。

| 面 | emu cap f820 | emu 行 f819 / f820 / f821 | rust cap f820 | rust 行 f819 / f820 / f821 |
|---|---|---|---|---|
| FC($2D) | 23 | 22 / **23** / 24 | **24** | 22 / 23 / **24** |
| ScrollX($FD) | F5 | F7 / **F5** / F4 | **F4** | F7 / F5 / **F4** |
| SamusX($30E) | 65 | 67 / **65** / 64 | **64** | 67 / 65 / **64** |
| NMIStatus($1A) | **00** | —（行=01） | 00 | 01 / 01 / 01 |

**$1A 判别位（A 级）**：emu capture $1A=00 = NMI(N) 内块**未跑**（内块尾置 1）；
rust state 行 $1A=01 = NMI(N) 已跑。⇒ emu capture(N) 落在
**work(N-1) 完成后、NMI(N) 之前**的自旋窗（=FC 已 inc 至 state-row(N) 值、$1A 未置 1）。

**锚 B（图像/PPU 面）**：emu capture(N) 的截图=可见区 N=NMI(N-1) 锁存态。

| 点 | emu 图像滚动 | rust 图像滚动（现状） | dx 最优 | 残差 |
|---|---|---|---|---|
| f620（滚动静止 00） | 00 | 00 | **0** | **0px** |
| f1020（滚动静止 94） | 94 | 94 | 0 | 332px（精灵/动画面小差） |
| f820（走动 F7→F5→F4） | **F7**（=行 f819） | **F5**（=行 f820） | **+2** | **674px**（自 12085px 崩落） |

f820 的 2px=|F7-F5|=1 拍走动量 ⇒ **emu 图像滚动=state-row(N-1)**。

复现命令：

    cd /home/peng/workspace/metroid-decomp
    # 双腿采集（spin-exit 采样 + 5 采集点）
    python3 tools/l4_diff_run.py align case-walkfcprobe --frames 1400 --timeout 600
    # 锚 A：capture(N) RAM vs state 行 N/N+1
    python3 - <<'EOF'
    import json, re
    def ram(p):
        return bytes.fromhex(re.sub(r'[^0-9a-fA-F]', '', open(p).read()))
    def rows(p):
        return {json.loads(l)['f']: json.loads(l)['w'] for l in open(p) if l.strip()}
    cap = ram('analysis/traces/case-walkfcprobe_f00820_l380.ram.hex')
    r = rows('shots_rust/case-walkfcprobe.state.jsonl')
    print('cap FC=%02X ScrollX=%02X' % (cap[0x2D], cap[0xFD]))
    for f in (819, 820, 821):
        z = bytes.fromhex(r[f]['zp'])
        print('row', f, 'FC=%02X ScrollX=%02X' % (z[0x2D], z[0xFD]))
    EOF
    # 锚 B：dx 配准
    python3 - <<'EOF'
    from PIL import Image
    import numpy as np
    for f, t in [(620, 'l180'), (820, 'l380'), (1020, 'l580')]:
        e = np.asarray(Image.open(f'shots/case-walkfcprobe_f0{f:04d}_{t}.png').convert('RGB')).astype(int)
        r = np.asarray(Image.open(f'shots_rust/case-walkfcprobe_f0{f:04d}_{t}.png').convert('RGB')).astype(int)
        print(f, [(dx, int((np.abs(e[:, :-dx] - r[:, dx:]).sum(2) > 0).sum())) for dx in range(0, 3)])
    EOF

### 2c. 双相位截取模型（本文档核心结论）

    emu capture(N)（endFrame N 一次回调内的两个产物，天然两相位）：
      ├─ ram.hex（活体读）= work(N-1) 完成后 ——≡ rust tick(N-1) 收口
      └─ screenshot + ppu.hex（图像/PPU=渲染缓冲）= NMI(N-1) 锁存
           （滚动=state-row(N-1)、名表≤work(N-2) 冲刷、OAM=OAMDMA(N-1)）
           ——≡ rust tick(N-1) 的 on_spin_exit 缝（nmi_handler(N-1) 后/工作前）

**⇒ rust 采集帧 N 的产物在 tick(N-1) 收口：RAM=活体、PNG/PPU=seam(N-1) 快照。**
现状（tick(N) 收口、PNG 用活体 render 态）三面全差 1 拍：RAM=行(N+1)、
图像滚动=行(N)（应为行(N-1)）、名表=work(N) 直写（应≤work(N-2)）。

已知仪器面（非状态差）：emu dumpPpu 尾 3B 用 `emu.getState()`（:177-185 字段名
容错双形仍取 0）——尾字节恒 00，rust 尾字节为真值 ⇒ ppu.hex 对账尾 2B 恒差，
判读时剔除（挂账 harness 修）。

## 3. walkfcprobe state 层逐帧表（前提翻转的根据）

case-walkfcprobe（leftwalk-probe2 同输入：boot→菜单→game-in→f440-1400 持左；
sample_point=spin-exit；watch FC/ScrollX/ScrollY/ScrollDir/PPUCTRL/SamusX）：

| 面 | 共有帧 | 差帧 | 结论 |
|---|---|---|---|
| FC($2D) | 1383 | **0** | 0 差——B2 前提作废 |
| ScrollX($FD) | 1383 | **0** | 0 差 |
| ScrollY($FC) | 1383 | **0** | 0 差 |
| ScrollDir($49) | 1383 | **0** | 0 差 |
| SamusX($30E) | 1383 | **0** | 0 差 |
| PPUCTRL($FF) | 1383 | **582** | 真差（§5——(814)、(819-1399)，emu=0x97/rust=0x93） |

数据源：`shots_rust/case-walkfcprobe.state.jsonl`（rust 1384 行 zp/iram/wram/pw）+
`analysis/traces/case-walkfcprobe.frames.jsonl`（emu 1383 行具名 watch）。
复跑：`python3 tools/l4_diff_run.py align case-walkfcprobe --frames 1400 --timeout 600`。

## 4. B2 作废重判注（证伪更正文化）

> 【2026-11-03 更正】B2「walk 窗 FC 计数径根修」的前提「$2D=23/24 同点真差」**证伪**：
> 该差值=capture 层采样缝投影（rust 截取点=emu 状态行(N+1)，§2b 锚 A）。
> state 层（spin-exit 缝）FC 逐帧 0 差（§3）——FC 计数径无罪，无需根修。
> B2 的「仪器缝臂未中」结论随前提翻转**作废重判**：真差面=**截屏相位**（§2c）。
> B0/B1（626→12085 回归坐实）与 bisect 定罪（db745dbd 相位移根因）不受影响——
> 只改「真差面」的定性（FC 计数→截屏相位）。旧 goal-r4 +6 过量=mod-8 补偿巧合
> （≡-3 恰合 f820 相位）的结论维持。

## 5. SetPPUInc（$FF bit2）真差面

- 现象：state 层 582 帧（f814、f819-1399）emu=0x97 / rust=0x93（bit2=VRAM 增量 32 位）。
- 根因（A 级）：emu 列流送走 VRAMStructBuffer→NMI 刷→`VRAMStructWriteData`（C2E4）
  按控制字节 bit7 调 `SetPPUInc`（C318）置位；载房属性行（控制 $20）与区域装载直拷
  （LC86F `and #$FB`）清位。rust V2-lite `stream_column_kick`（30 行直写 $2006/$2007）
  **未模 SetPPUInc 的 $FF 副作用**⇒位恒 0。
- 渲染影响：**无**（$2000 bit2 只影响 $2007 自增步长，不影响渲染）；
  但 write_io(0x2007) 的地址步进读 $FF bit2（memory.rs）——直写径逐行显式 $2006 不受影响，
  载房 4 页直拷依赖 inc=1 ⇒ 补模须成对（置/清），否则破坏直拷。
- 修法（§6b）：直写径按 asm 控制字节补 SetPPUInc 副作用（列流送置位/横写清位）。

## 6. 修径（lead 裁定 2026-11-03，判据不修订）

### 6a. 截取点双相位对齐（主修）

`crates/game-core/src/bin/rust_screenshot.rs`：采集帧 N 的产物移至 tick(N-1) 收口——
- ram.hex：tick(N-1) 收口的活体读（work(N-1) 后、NMI(N) 前——$1A=00 面已证）。
- png + ppu.hex：tick(N-1) 的 on_spin_exit 缝（nmi_handler(N-1) 后）快照
  （RenderState + OAM $200-2FF）——图像/PPU=渲染管序相位。
- TDD 双锚：capture(N)≡state-row(N) 断言（FC/ScrollX/SamusX 面）+
  walk 窗 FC 双腿逐帧 0 差锚（已绿，随修保持）；判据=l380<700px。

### 6b. SetPPUInc 补模（状态面锚：双腿 $FF 0 差）

`stream_column_kick` 置 $FF bit2（竖写 $1E|$80）；横写径（attrib_table_write 等
控制 $20）清位；载房直拷前清位（LC86F 对齐）。TDD：$FF 状态面双腿 0 差。

### 6c. 判据（不修订）

l380<700px + scrollseam 左滚两点同降 + case-002 邻房族收益（166/170px）保持 +
门链五点/落地三点/killenemy 无回退 + L4 矩阵无回退（近期锚集 ratchet 守）+
cargo test 全绿 + gates check。**capture 面移动后全矩阵 fresh 复跑一次落新锚**
（相位对齐后旧 diff 值语义已变，A-full 前量值锚随修复重基）。
