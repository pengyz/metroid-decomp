# 时序三件规格：`WaitNMIPass` / `ScreenOff` / `ScreenOn`

状态：A 级（可逐行复现）为主，B 级假设单列于 §5。
上游：`docs/spec/porting-backlog.md` §2 的 B1 批（PPU/帧时序）。
下游用途：#37（密码屏 `$00FE` 相位）、任何需要「精确等一个 VBlank」的端口代码。
证据只取已入库 trace（`analysis/traces/case-password-justin.*`）与 ROM 反汇编，本卡未新开 emu。

## 0. 一句话结论

- 三条例程**都不自增 `FrameCount($2D)`**；`FrameCount` 只在 `MainLoop` 每轮迭代开头 `inc`（`prg7_engine.asm:207`）。
- `WaitNMIPass` = 「等**一个完整 NMI**」：先清 `NMIStatus($1A)`，再自旋到它变非 0。返回时恰好跨过 **1 个 VBlank**。
- `ScreenOff`/`ScreenOn` = 先改 `PPUMASK_ZP($FE)`，再**直接落进** `WaitNMIPass_`（同一段自旋），因此各自也跨 **1 个 VBlank**，且新掩码由**这次等待的 NMI** 里的 `WritePPUCtrl` 写进 `$2001`。
- 一次 `MainLoop` 迭代里调用 k 次这三件中的任意一个 ⇒ 该迭代跨 **k+1 个 VBlank**、`FrameCount` 只加 1 ⇒ 外部看到「画面冻 k 帧」。

## 1. 三条例程的精确语义

地址与行号以 `refs/m1disasm/SRC/prg7_engine.asm` 为准。

### 1.1 `WaitNMIPass`（bank7 `$C42C`，文件 :1108-1116）

```asm
WaitNMIPass: ; 07:C42C
    jsr ClearNMIStat          ; A=0; sta NMIStatus   (:1110 → ClearNMIStat :1117-1121)
    @loop: ; 07:C42F
        lda NMIStatus         ; (:1113)
        beq @loop             ; 0 就继续等
    rts
```

| 项 | 值 |
|---|---|
| 读 | `NMIStatus` = `$1A`（`constants_ram.asm:496`；0 = NMI 进行中，非 0 = NMI 已结束） |
| 写 | `NMIStatus($1A) ← $00`，仅此一处 |
| 退出条件 | `$1A != 0`；唯一置非 0 的地方是 NMI 尾 `ldy #$01 / sty NMIStatus`（`prg7_engine.asm:280`） |
| 跨帧 | **1 个 VBlank**（等的那次 NMI 必须完整跑完才可能置 1） |
| `FrameCount` | 不变（本函数不碰 `$2D`） |
| 返回时 A/X/Y | A=`$01`（循环退出前最后一次 `lda NMIStatus` 的值）；X/Y 不变 |
| 副作用 | 若调用时 NMI 已被关（`ScreenNmiOff` 之后），本函数**会死等** —— 见 §5 H1 |

### 1.2 `ScreenOff`（bank7 `$C439`，文件 :1123-1128）

```asm
ScreenOff: ; 07:C439
    lda PPUMASK_ZP
    and #~(PPUMASK_BG_ON | PPUMASK_OBJ_ON)   ; 清 $08/$10
    ; fallthrough
WriteAndWait: ; 07:C43D
    sta PPUMASK_ZP                            ; 立刻更新影子
WaitNMIPass_: ; 07:C43F
    jsr ClearNMIStat
    @loop: lda NMIStatus / beq @loop
    rts
```

| 项 | 值 |
|---|---|
| 读 | `PPUMASK_ZP($FE)`（`constants_ram.asm:819`） |
| 写 | `PPUMASK_ZP($FE) ← 旧值 & ~($18)`；随后 `$1A ← 0` |
| 典型值 | `$1E → $06`（`$1E`=SHOW8BG(2)+SHOW8OBJ(4)+BG_ON(8)+OBJ_ON(16)，`hardware.asm:30-39`） |
| `$2001` 何时变 | **不是**本函数直接写；由本次等待的 NMI 调 `WritePPUCtrl`（`prg7_engine.asm:264`，实现 :1150-1160，`lda PPUMASK_ZP / sta PPUMASK` 在 :1154-1155）落进真寄存器 |
| 跨帧 | **1 个 VBlank** |
| `FrameCount` | 不变 |
| 返回时 A/X/Y | A=`$01`（同上；`sta PPUMASK_ZP` 之后 A 被 `ClearNMIStat` 覆盖为 0，再被自旋覆盖为 1）；X/Y 不变 |

### 1.3 `ScreenOn`（bank7 `$C447`，文件 :1142-1149）

```asm
ScreenOn: ; 07:C447
    lda PPUMASK_ZP
    ora #PPUMASK_SHOW8BG | PPUMASK_SHOW8OBJ | PPUMASK_BG_ON | PPUMASK_OBJ_ON
    bne WriteAndWait          ; 分支恒真（ora 后 $18 位已置）
```

| 项 | 值 |
|---|---|
| 写 | `PPUMASK_ZP($FE) ← 旧值 \| $1E`（实际置位 `$02/$04/$08/$10`），随后 `$1A ← 0` |
| 典型值 | `$06 → $1E` |
| `$2001` 何时变 | 同 1.2：该次等待的 NMI 里 `WritePPUCtrl` |
| 跨帧 / `FrameCount` / 返回 A | 同 1.2（1 个 VBlank；`$2D` 不变；A=`$01`） |

### 1.4 相关但不等价的四个邻居（避免端口时混用）

| 例程 | bank:addr | 与三件的关系 |
|---|---|---|
| `ClearNMIStat` | 07:C434 | 只 `$1A ← 0`，**不等待** |
| `WaitNMIPass_` | 07:C43F | 与 `WaitNMIPass` 同体（入口少一次 `jsr`），语义相同 |
| `WriteAndWait` | 07:C43D | 先 `sta $FE` 再等 1 个 VBlank |
| `ScreenNmiOff` | 07:C45D | `ScreenOff`(1 次等待) 后再清 `PPUCTRL_ZP($FF)` 的 VBL 位并写 `$2000` ⇒ 之后 NMI 不再来，**后续 `WaitNMIPass` 会死等** |
| `NMIOn` | 07:C487 | 自旋等 `PPUSTATUS` 的 VBlank 标志**结束**，再置 `$2000` 的 NMI 位；不是「等一个 NMI」，不碰 `$1A` |
| `VBOffAndHorzWrite` | 07:C47D | 只写 `$2000/$FF`（关 VBlank + 增量变横），**不等** |


### 1.5 本组例程会动的字段：地址与值（具体数，不是文字描述）

| 字段 | 地址 | 谁写 | 典型值变化 | 何时对硬件生效 |
|---|---|---|---|---|
| `PPUMASK_ZP` 影子 | RAM `$00FE` | `ScreenOff`/`ScreenOn`/`WriteAndWait`（`sta PPUMASK_ZP` :1127/:1146/:1140） | `$1E → $06`（Off）、`$06 → $1E`（On） | 立即可读（RAM）；真 `$2001` 要等下一次 NMI |
| `PPUCTRL_ZP` 影子 | RAM `$00FF` | `ScreenNmiOff`（`and #~PPUCTRL_VBLKNMI_ON`）、`NMIOn`（`ora #$80`）、`VBOffAndHorzWrite` | `$90 → $10`（关 NMI）、`$10 → $90`（开 NMI） | 同上，经 NMI 的 `WritePPUCtrl` |
| `PPUMASK` 真寄存器 | `$2001` | NMI 内 `WritePPUCtrl`（:1154-1155） | 随 `$00FE`：`$06` 或 `$1E` | 该次 NMI 内 |
| `PPUCTRL` 真寄存器 | `$2000` | `WritePPUCtrl`（:1153）、`ScreenNmiOff`（:1176 直接写）、`VBOffAndHorzWrite`（:1193） | `$90`（NMI 开+增量横）/ `$10`（NMI 关+增量横） | 直接写时立即；经 `WritePPUCtrl` 时该次 NMI 内 |
| `NMIStatus` 握手字节 | RAM `$001A` | `ClearNMIStat`→0；NMI 尾→1；`MainLoop` 每轮→0 | `0`（等 NMI）/`1`（NMI 已完） | RAM 内部 |
| `FrameCount` | RAM `$002D` | 仅 `MainLoop` 每轮 `inc`（:207） | 每迭代 +1，256 回绕 | RAM 内部 |

读法：`$00FE/$00FF` 是**影子**，端口侧可以「调用瞬间改影子、NMI 里再写寄存器」；`$001A` 是**握手**，改它不等于等它。

## 2. 帧数契约

### 2.1 推导（A 级，ROM 结构）

`MainLoop`（`prg7_engine.asm:199-217`）：

```text
MainLoop:
  jsr CheckBankSwitch / jsr UpdateTimer / jsr GoMainRoutine
  inc FrameCount                ; :207 唯一自增点
  lda #$00 / sta NMIStatus      ; 宣告「我在等 NMI」
  wait: tay / lda NMIStatus / bne exit / jmp wait
  jsr RandomNumbers / jmp MainLoop
```

`NMI`（`:233-281`）：`$2003/$4014` DMA 影子页 → 若 `$1A != 0` 则跑 `NMIScreenWrite/CheckPaletteWrite/CheckVRAMStructBufferWrite/WritePPUCtrl/WriteScroll/ReadJoyPads`（`:251-265`），`$1A != 0` 时**整段跳过**（掉帧保护，`:250-252`）→ `SoundEngine/UpdateAge` → `$1A ← 1`（`:280`）。

于是：**一次迭代跨几个 VBlank = 1（MainLoop 自己那次）+ 迭代内三件调用次数**；`FrameCount` 每次迭代只 +1。

### 2.2 实测调用点（A 级，来自已入库 trace）

`case-password-justin`，读 `analysis/traces/case-password-justin.frames.jsonl` 的 `w.FC`，
同帧 RAM dump 读 `$2D/$FE`（命令见 §6）：

| `FC` 覆盖区间 | `FC` | 跨 VBlank | `$FE`（同帧 dump） | 归属（A=观察，B=推断） |
|---|---|---|---|---|
| f1188–f1191 | 153 | **4** | f1190=`1E`、f1191=`06` | 密码提交径；见 §3.2 |
| f1192–f1198 | 154 | **7** | f1197=`06` | 同上后续 |
| f300–f303 | 39 | **4** | `1E`（f300/f320 dump） | 屏未关、纯等待 3 次 |
| f501–f504 | 237 | **4** | `1E`（f480/f510 dump） | 同上 |

（区间写法=frames.jsonl 里该 `FC` 值连续出现的首末帧，含两端；跨度 = 末−首+1，等待次数 = 跨度−1。命令见 §6 第 4 条。）
> 【2026-11-25 更正】初稿把这两组写成「f301–f303 / f502–f504，跨 3 帧、2 次等待」：那是把「与前帧同 FC 的帧」当成整段，漏掉了段首帧。按 `w.FC` 实测 `FC=39` 覆盖 **300–303**（f304=40）、`FC=237` 覆盖 **501–504**（f505=238），故跨度 4、等待 3 次；对影复核指出，已改。

**f300–f303 与 f501–f504 这两组（跨度 4、3 次等待）是本卡最干净的「纯等待」证据**：`$FE` 全程 `1E`（屏幕一直亮），说明这 3 次等待里没有任何 `ScreenOff/ScreenOn`，只有 `WaitNMIPass` 被连调 3 次 ⇒ 迭代跨 4 个 VBlank、`FC` 只加 1。

**f1188–f1191（跨度 4）与 f1192–f1198（跨度 7）**：`$FE` 在 f1190→f1191 之间从 `1E` 变 `06`（熄灯），说明该迭代内至少发生一次 `ScreenOff`（或 `ScreenNmiOff`）。跨度 4 ⇒ 3 次等待；跨度 7 ⇒ 6 次等待。
静态扫描（§6 第 3 条命令）给三条相关链的**直接+传递**等待数：

| 链 | 等待次数 | 组成 |
|---|---|---|
| `InitializeGame`（00:92D4） | **1** | `ScreenNmiOff`（内含 1 次 `WriteAndWait`） |
| `MoreInit`（07:C81D） | **2** | `ScreenOff` + 末尾 `jmp ScreenOn` |
| `ClearAll`（00:909F） | **2** | `ScreenOff` + 显式 `WaitNMIPass`（:2331） |
| `DisplayPassword`（00:9359） | **5** | `ClearAll`(2) + `PasswordToScreen`内 1 次(:2888) + `WaitNMIPass`(:2811) + `ScreenOn`(:2818) |

⇒ 跨度 4 的 `$FE=1E` 组只能由「3 次纯 `WaitNMIPass`」构成（上表无纯等待链，故这 3 次是**调用点自己连调**，例如 `PasswordToScreen` :2888 与调用者的两次等待）。而 `FC=153` 的跨度 4（3 次等待）正好等于 `InitializeGame`(1) + `ClearAll`(2) = 3，与静态计数自洽；`FC=154` 的 6 次等待在表里没有单链组合，其拼装属 **B 级**，见 §5 H2。

## 3. 调用序

### 3.1 `MainLoop` 与三件的相对顺序

```text
MainLoop ──┬─ CheckBankSwitch
            ├─ UpdateTimer
            ├─ GoMainRoutine ──→ 各 MainRoutine / TitleRoutine（三件都在这里被调）
            ├─ inc FrameCount
            ├─ NMIStatus ← 0
            └─ 自旋等 NMIStatus≠0  ←── NMI 尾置 1
NMI ──┬─ $2003/$4014 DMA
      ├─ if $1A≠0: NMIScreenWrite/…/WritePPUCtrl($FE→$2001,$FF→$2000)/WriteScroll/ReadJoyPads
      └─ SoundEngine / UpdateAge / $1A←1
```

要点：`ScreenOff` 改的是 `$FE` 影子；**真 `$2001` 在本函数等待的这次 NMI 里才写**。因此「调用返回时 `$2001` 已是新值」成立，但「调用一进函数 `$2001` 就变」不成立。

### 3.2 密码屏链路中的出现次数

`CheckPassword`（00:8C5E）校验通过 → `jmp InitializeGame`（00:92D4）：

```text
InitializeGame: ScreenNmiOff(1 等) → LoadSamusGFX → NMIOn → 设 MainRoutine=MoreInit → rts
MoreInit:       ScreenOff(1) → ClearNameTables/EraseAllSprites/DestroyEnemies
                → SetupRoom 循环(房间号 $FF 起、跑 2 次) → jmp ScreenOn(1)
ClearAll:       ScreenOff(1) → ClearNameTables/EraseAllSprites → WaitNMIPass(1)
                → jmp VBOffAndHorzWrite(不等)
DisplayPassword:ClearAll(2) → PreparePPUProcess_ → InitPasswordFontGFX → CalculatePassword
                → NMIOn → PasswordToScreen(内部 1 次 WaitNMIPass) → WaitNMIPass(:2811)
                → jmp ScreenOn(1)                    ⇒ 合计 5 次等待
```

次数汇总（A 级，静态）：`InitializeGame` 1、`MoreInit` 2、`ClearAll` 2、`DisplayPassword` 5（含 `PasswordToScreen` 内的 1 次）。

与 `docs/findings/password-dark-window-frame-accounting.md` 的对照：该文记录 **emu `FC=153` 跨 f1188–f1191（4 帧）、`FC=154` 跨 f1192–f1198（7 帧）**，`$FE` 落点 f1191→f1192。本卡确认：这两个数就是「该迭代的等待次数 = 跨度−1」，即 3 次与 6 次；而 §2.2 的静态表里没有**单条链**恰好等于 3 次或 6 次（3 次需两条链叠加 `InitializeGame`+`ClearAll`；6 次需至少 3 条链叠加）⇒ **#37 要修的正是这两次迭代的链组成**，本卡只给证据与判据，不给结论性归属（见 §5 H2）。

## 4. 与 #37 的关联点（不要求本卡解决）

1. `$00FE` 的写入时刻 = `ScreenOff/ScreenOn` 被调用的那个迭代（`GoMainRoutine` 内）；`$2001` 的生效时刻 = 该迭代等待的 NMI。Rust 侧若在「采集帧 N」看到 `$FE` 已变而 emu 在 N+1 才变，说明**同一条链在两侧被放进了不同迭代**（多/少了一次等待），而不是掩码值算错。
2. 判据建议：逐帧 `$2D`（`FrameCount`）与 `$FE` 双列对照；跨度不一致 ⇒ 等待次数不一致；跨度一致而 `$FE` 不同 ⇒ 链的先后顺序不一致。
3. 本卡不改 `crates/`；#37 修完后用本文 §2.2 的四处调用点回归。

## 5. 假设区（B/C 级，不得当结论引用）

- **H1（B）**：`WaitNMIPass` 在 NMI 关闭期间会死等。依据是结构（自旋条件只由 NMI 尾置位），`ScreenNmiOff` 之后到 `NMIOn` 之间不得插 `WaitNMIPass`。未实测（没在 emu 里造这个死锁）。
- **H2（B）**：`FC=153` 的跨度 4（3 次等待）= `InitializeGame`(1) + `ClearAll`(2)，算术自洽；`FC=154` 的跨度 7（6 次等待）= `MoreInit`(2) + `ClearAll`(2) + 另 2 次纯 `WaitNMIPass`（或 `DisplayPassword` 5 次 + 1 次）。两组都缺一份带 `PC`/调用点的 trace 来钉死拼装顺序。
- **H3（C）**：f1188 之前的 `FC=150..152` 每帧 +1、无停顿，说明提交键的处理本身不额外等待；未做逐迭代验证。

## 6. 复跑命令（全部实跑过）

```bash
# 1) FC 逐帧与停顿组（输出：停顿帧号列表）
python3 - <<'EOF'
import json
rows=[json.loads(l) for l in open('analysis/traces/case-password-justin.frames.jsonl')]
prev=None; stalls=[]
for r in rows:
    fc=r['w'].get('FC')
    if prev is not None and fc==prev: stalls.append(r['f'])
    prev=fc
print('FC stalls:', stalls)
EOF
# 期望：重复帧（与前帧同 FC）为 1189-1191、1193-1198、301-303、502-504；跨度见第 4 条命令

# 2) 同帧 RAM dump 的 $2D/$FE（$FE 是 PPUMASK 影子）
python3 - <<'EOF'
import glob,re
for p in sorted(glob.glob('analysis/traces/case-password-justin_f*.ram.hex')):
    f=int(re.search(r'_f(\d+)_',p).group(1))
    if f in (1190,1191,1197,1200,300,320,480,510):
        b=bytes.fromhex(''.join(open(p).read().split()))
        print(f, 'FC=%d'%b[0x2D], 'FE=%02x'%b[0xFE])
EOF
# 期望：1190 FC=153 FE=1e / 1191 FC=153 FE=06 / 1197 FC=154 FE=06 / 1200 FC=156 FE=1e

# 4) 由 FC 直接算「覆盖区间 / 跨度 / 等待次数」，避免帧号写法歧义
python3 - <<'EOF'
import json
rows=[json.loads(l) for l in open('analysis/traces/case-password-justin.frames.jsonl')]
prev=None; start=None; runs=[]
for r in rows:
    fc=r['w'].get('FC')
    if fc!=prev:
        if prev is not None: runs.append((start, r['f']-1, prev))
        start=r['f']; prev=fc
runs.append((start, rows[-1]['f'], prev))
for a,b,fc in runs:
    if b-a>=1:
        print(f'FC={fc}: frames {a}-{b} span={b-a+1} waits={b-a}')
EOF
# 期望：FC=1 3-6 span4/3；FC=39 300-303 span4/3；FC=237 501-504 span4/3；
#       FC=153 1188-1191 span4/3；FC=154 1192-1198 span7/6

# 3) 四条链的等待次数（静态扫描，输出 InitializeGame=1 MoreInit=2 ClearAll=2 DisplayPassword=5）
python3 - <<'EOF'
import re
p='refs/m1disasm/SRC/prg7_engine.asm'; q='refs/m1disasm/SRC/prg0_title.asm'
body={}; labels={}
for path in (p,q):
    cur=None; buf=[]
    for l in open(path).read().splitlines():
        m=re.match(r'^([A-Za-z_][A-Za-z0-9_]*):',l)
        if m:
            if cur: body[cur]=buf
            cur=m.group(1); buf=[l]
        elif cur: buf.append(l)
    if cur: body[cur]=buf
def waits(name,depth=0):
    if depth>4 or name not in body: return 0
    n=0
    for l in body[name]:
        m=re.search(r'\b(jsr|jmp)\s+([A-Za-z_][A-Za-z0-9_]*)',l)
        if not m: continue
        t=m.group(2)
        if t in ('WaitNMIPass','WaitNMIPass_','WriteAndWait','ScreenOff','ScreenOn','ScreenNmiOff'): n+=1
        elif t in body: n+=waits(t,depth+1)
    return n
for t in ('InitializeGame','MoreInit','ClearAll','DisplayPassword'):
    print(t, waits(t))
EOF
```

## 7. 未覆盖

- 没有 `PC`/`JSR` 级 trace，所以每组停顿的**等待拼装顺序**未钉死（H2）。
- 未在 emu 里实测 H1 的死锁；未覆盖 PAL/MZM 等非 NTSC 构建（`prg7_engine.asm` 里 `ScreenOn` 的 `.elif` 分支）。
- 未统计游戏运行期（非标题/密码屏）里三件的全部调用点分布；本卡只给了 MainLoop 序与密码链。
