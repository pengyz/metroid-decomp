# 定界房扫 sprite 族内容残差（Tourian room 10 / Ridley room 37）

- 日期：2026-11-23
- 结论级别：A（ROM 反汇编 + 两腿逐帧 RAM/OAM 字节比对）／B（根因机制推理链）／C（假设区，单独标注）
- 输入：`rom/Metroid (USA).nes`（sha256 `e6e6b7014685adae447ebb3833242815747bc1e5df83ade79f693fb67cf565b6`）；既有 emu 采集 `shots/case-oos2-t10_*`、`shots/case-oos2-r37_*`、`analysis/traces/case-oos2-t*.hex`
- 产出目录（可再生工作区，`.gitignore` 忽略）：`analysis/scratch-sprite-residual/`
- 未改动：`rom/`、`tools/`、`crates/`、参考投影。本次只读调用 `tools/` 的 CLI/函数。

## 0. 结论摘要

残差不是一族，是**两个独立根因**，都落在「敌人更新（AI）遗漏了 ROM 的某条门/怪癖」，最终只表现为精灵层（CHR/名表/调色板逐字节零差，见 §3）：

| 族 | 房 | 首个分歧（emu 显示帧 g / rust 采集帧 c） | 首个分歧字段 | 机制一句话 |
|---|---|---|---|---|
| A | Tourian 10 | g=1683 / c=1682，像素 286 | OAM 槽 0 的 `x`（emu 223 / rust 221） | Metroid 限速钳位：ROM 用 `lda #$00 / cmp sub / sbc speed` 的**字节形**绝对值（= `-speed-(sub>0)`），Rust 用了 16 位真幅值，导致负速+小数非零时 Rust 多钳一次并清掉小数速度 |
| B | Ridley 37 | g=1679 / c=1679，像素 92（Ens 分歧早一帧 c=1678） | OAM 槽 0 的 `tile`（emu `$E2` / rust `$E4`），槽 2/3 的 `y` +4 | Dessgeega（天花）动画切换：ROM `EnemyFlipAfterDisplacement` 用 `EnData1F` 决定 `jumpDsplcmnt` 的符号（`$C0` ⇒ 取负 ⇒ ≥16 ⇒ 不切换），Rust 只看 `jumpDsplcmnt` 自身符号 ⇒ 误切到 hopping 动画 |

Rust 侧落点：

- A：`crates/game-core/src/enemy/metroid.rs:76-89`（`clamp_axis`），调用点 `metroid.rs:120-131`。
- B：`crates/game-core/src/enemy/sidehopper.rs:51-74`（`enemy_flip_after_displacement`），入口 `sidehopper.rs:76-91`（`sidehopper_ai`）。

ROM 侧锚：

- A：`refs/m1disasm/SRC/enemies/metroid.asm:30-85`（y 钳位 30-58、x 钳位 60-85；x 的 `lda #$00 / cmp Ens.0.speedSubPixelX / sbc Ens.0.speedX` 在 73-75 行）。
- B：`refs/m1disasm/SRC/prg7_engine.asm:12271-12323`（`EnemyFlipAfterDisplacement ; 07:FB88`，`EnData1F` 门 12276-12293，切换/提前返回 12294-12323）。

两族的差异形态也互斥：A 是**位置漂移**（tile/attr 不变，x/y 随时间累积），B 是**动画相位错**（tile 与放置 y 差固定值，位置不变）。见 §4、§5。

## 1. 复现命令

探针脚本：`analysis/scratch-sprite-residual/run_room.py`（只读调用 `room_sweep.build_case`、`scenario_run`、`fc_align`、`l4_diff_run`）。

### 1.1 复算残差 + OAM 逐槽（预期输出逐字如下）

```bash
cd /home/peng/workspace/metroid-decomp
python3 analysis/scratch-sprite-residual/run_room.py --area 3 --room 10 --reuse-emu-old case-oos2-t10
python3 analysis/scratch-sprite-residual/run_room.py --area 4 --room 37 --reuse-emu-old case-oos2-r37
```

预期（本次实测，收尾各打印一行摘要）：

```
case-sprite-t10: frames=81 ok=22 nonzero=49 errors=10 max=735
case-sprite-r37: frames=81 ok=19 nonzero=54 errors=8 max=26861
```

产物：

- `analysis/scratch-sprite-residual/case-sprite-t10/<name>.oam.json`（逐帧 diff、首个分歧槽、级联槽、PPU 分层）
- `analysis/scratch-sprite-residual/case-sprite-r37/<name>.oam.json`
- 同目录 `rust.seam.oam.txt` / `rust.live.oam.txt` / `*.state.jsonl`、`seam/` `live/` 截图、`emu/` emu 截图与 hexdump、`run.json`

`--reuse-emu-old case-oos2-t10` 的语义：旧用例 `case-oos2-t10` 与本次 `build_case(3,10,152,[60..140])` 的 fixtures/captures/入口完全同构（唯一差别是 `name` 行与 `watch 002D` 行），故复用其 emu 产物（86 张 PNG + 86 组 `ram/ppu/apu.hex` + `frames.jsonl`），改名复制到本探针目录，来源记在 `emu/reuse.json`。本次同时核对了**新 rust 截图与旧 rust 截图逐字节相同**（f1660/f1681/f1731 sha256 一致），证明引擎侧未变，复用不引入行为差。

### 1.2 不用复用、自己重跑 emu（需要 emu 空闲）

```bash
python3 analysis/scratch-sprite-residual/run_room.py --area 3 --room 10 --only-emu   # 需要 `.sched` 已生成
python3 analysis/scratch-sprite-residual/run_room.py --area 3 --room 10 --analyze-only
```

`--only-emu` 已实跑通过（2026-11-23 21:21，DONE；`emu/oam_nmi.txt` 逐帧 64 槽 × 4 字节），走 `analysis/_gen/probe-case-sprite-t10.lua`（在 `scenario_capture.lua` 产物上追加 NMI `$C0D9` 入口 `$0200-$02FF` 快照，参考 `analysis/scratch-shots/probe673/probe_nmi_page.py` 的取页法）。Mesen 是单实例：被 Team 其他卡占用时表现为 `rc=0 但无 DONE/frames.jsonl`（本卡实测被 `case-001` 抢占一次，trace 停在 f664），脚本按此判据报错退出，不会静默当成成功。

### 1.3 只看首个分歧槽（不重跑，直接读产物）

```bash
python3 - <<'EOF'
import json
for n in ("case-sprite-t10","case-sprite-r37"):
    rows=json.load(open(f"analysis/scratch-sprite-residual/{n}/{n}.oam.json"))["rows"]
    r=next(x for x in rows if x.get("diff"))
    print(n, "c=",r["c"],"g=",r["g"],"phase=",r["phase"],"diff=",r["diff"],
          "first_slot=",r["first_slot"], r["first_detail"])
EOF
```

预期：`case-sprite-t10 c=1682 g=1683 seam diff=286 first_slot=0 {emu (223,59,192,34) / rust (221,59,192,34)}`；`case-sprite-r37 c=1679 g=1680 seam diff=92 first_slot=0 {emu (164,175,226,163) / rust (164,175,228,163)}`（元组序 `x,y,tile,attr`）。

### 1.4 逐帧 RAM/Ens 对比（本报告 §4/§5 的字段证据来源）

emu 帧 g 的收口状态 = 采点 g 的 `ram.hex`（`$0000-$07FF` + `$6AF4-$6BD3`）；rust 帧 c 的收口状态 = `state.jsonl` 的 `iram`（`$0100-$07FF`）与 `wram`（`$6000-$7FFF`）。按 `fc_align` 的 (FC,ScrollX) 配对 `c↔g` 取同一 work 迭代：

```bash
python3 - <<'EOF'
import json
from pathlib import Path
out=Path("analysis/scratch-sprite-residual/case-sprite-t10")
def emu(f):
    b=bytes.fromhex(next((out/"emu").glob(f"case-sprite-t10_f{f:05d}_*.ram.hex")).read_text().strip())
    return b[:2048], b[2048:]
rows={json.loads(l)["f"]:json.loads(l) for l in open(out/"case-sprite-t10.state.jsonl")}
def rust(f):
    w=rows[f]["w"]; return bytes.fromhex(w["iram"]), bytes.fromhex(w["wram"])
for c,g in ((1681,1682),(1682,1683)):
    er,ex=emu(g); rr,rw=rust(c)
    k=4  # enemy slot 4（x 偏移 $40）
    print(c,g,"enc",er[0x400+k*16:0x400+k*16+16].hex(" "),"| rsc",rr[0x300+k*16:0x300+k*16+16].hex(" "))
    print("    encX",ex[k*16:k*16+16].hex(" "),"| rscX",rw[0xAF4+k*16:0xAF4+k*16+16].hex(" "))
EOF
```

## 2. 残差复算（与 `fc-align-out-of-sample.md` 对账）

配对规则沿用 `tools/movie_ref/fc_align.py::align_case`（(FC,ScrollX) 唯一配对，horizon 40；载房窗/emu 连续同帧强制 seam；零容差像素比较）。采集窗 offset 60..140（绝对帧 1660..1740）。

| 房 | 可判帧 | 零差帧 | 非零帧 | 帧差最大 | 帧差合计 |
|---|---|---|---|---|---|
| Tourian 10 | 71 | 22 | 49 | 735 | 21662 |
| Ridley 37 | 73 | 19 | 54 | 26861 | 39028 |

与既有文档对账：

- **零差帧数完全一致**（Tourian 22、Ridley 19）；**极值一致**（Tourian max 735、Ridley 尖峰 26861、Ridley 首帧 92）；**首个非零帧一致**。这三项是本卡结论所依赖的部分，已 ±0 复现。
- Tourian 合计：`21662 − 717(c=1730) − 735(c=1731) = 20210`，与文档 `20210` **逐字节算术相等**；即既有文档的合计不含最后两个 live 相位帧（本探针把它们计入，故 71 帧而非 69 帧）。
- Ridley 合计：本探针 `39028`（含尖峰 26861），文档 `10205`。文档的 `10205 < 26861`，说明其合计口径**排除了尖峰帧**；排除尖峰后本探针为 `12167`，与 `10205` 仍差 1962px。差异来自帧集合口径（文档 69 帧、本探针 73 帧），本报告不复现其 10205 的精确组成，**以本报告的逐帧表为准**（命令见 §1.3；每帧数字可直接复算）。此差异不影响根因结论：首个分歧帧、首个分歧槽、极值三项一致。
- 可判帧数差异来源：本探针的 emu 产物每帧都有采点（86 组），而 `fc_align` 对 `多候选配对` 与 `missing_png` 的计法决定了分母；两者均已如实计入 `errors`（Tourian 10、Ridley 8）。

## 3. PPU 分层：纯精灵层差

对每个非零帧，取同帧 emu/rust 的 `ppu.hex` 按 `room_sweep.PPU_REGIONS` 分解（CHR `$0-$2000`、nt0-3、palette `$3000-$3020`）：

```
Tourian 10 全部非零帧: chr=0 nt0=0 nt1=0 nt2=0 nt3=0 palette=0
Ridley 37 全部非零帧: chr=0 nt0=0 nt1=0 nt2=0 nt3=0 palette=0
```

⇒ 本卡两房的残差**只发生在 OAM（精灵页）**，不是图案表、名表或调色板。这与「sprite 族」定界一致，也排除了滚动/名表镜像类根因。

## 4. 族 A：Tourian room 10 —— Metroid 位置漂移（独立根因）

### 4.1 OAM 证据（A 级）

首个非零帧 c=1682 / g=1683，像素 286。分歧对象是一个 **3×3 metasprite（24×24）**，tile = `$C0..$E2`、attr `$22/$23` = 调色板 2 + 优先级——对照 `refs/m1disasm/SRC/tourian/enemy_sprite_data.asm:292-330`（`EnFrame_Metroid0`）与 `:244-250`（`EnPlace2`，位移 −12..+4），即 **Metroid**。

| c | g | diff | 分歧槽 | emu (x,y,tile,attr) | rust | 差异字段 |
|---|---|---|---|---|---|---|
| 1681 | 1682 | 0 | — | 225,59,C0,22 | 225,59,C0,22 | 无 |
| **1682** | **1683** | **286** | **0** | **223,59,C0,22** | **221,59,C0,22** | **x +2** |
| 1683 | 1684 | 286 | 0 | 221,59,C0,23 | 219,59,C0,23 | x +2 |
| 1690 | 1691 | 314 | 1 | 204,62,C0,22 | 196,62,C0,22 | x +8 |
| 1697 | 1699 | 484 | 0 | 224,92,C0,23 | 221,92,C0,23 | x +3 + 第二只 Metroid 入场（槽 9-17 级联） |
| 1719 | 1725 | 510 | 0 | 148,103,C3,23 | 129,103,C3,23 | x +19 |
| 1731 | 1740 | 735 | 0 | 101,107,C0,23 | 81,125,C0,23 | x +20, y −18 |

同帧可见槽数两腿一致（34→43），tile/attr 一致 ⇒ **不是漏画/多画/换帧，而是位置**。级联槽 1..8 是同一只 Metroid 的其余 8 块 tile（同一对象整体平移）；c≈1697 起槽 9..17 是第二只 Metroid，同样漂移。世界坐标（OAM x + ScrollX）对比：c=1681 两腿同为 385，c=1682 emu 385 / rust 383，误差随后单调累积到 −20px，符合「每帧多走一点」的积分漂移，而非单帧错位。

### 4.2 敌人槽字段证据（A 级）

分歧敌人的 Ens 槽 = **slot 4**（`$0440`）。同 work 迭代对比（emu 采点 g、rust 帧 c）：

| c | g | 字段 | emu | rust |
|---|---|---|---|---|
| 1680 | 1681 | speedX / speedSubPixelX / EnsExtra.subPixelX / x | `FF / D0 / D0 / 8B` | 同左（**完全一致**） |
| 1681 | 1682 | speedX / speedSubPixelX / EnsExtra.subPixelX / x | `FF / A0 / 70 / 8B` | `FD / D0 / A0 / 89` |

即：进入该帧时两腿状态相同，离开该帧时 rust 的 `speedX` 从 −1 变成 −3、位置多走 2px；emu 保持 −1、位置不动。分歧发生在**一次 AI+移动更新之内**。

### 4.3 根因推导（B 级，逐步可验算）

ROM：`MetroidAIRoutine` 的 x 限速段（`refs/m1disasm/SRC/enemies/metroid.asm:60-85`），负速径为：

```asm
    lda MetroidMaxSpeed,y      ; 最大速度（red=2/green=4）
    pha
    lda Ens.0.speedX,x
    bpl @endIf_E
        pla
        jsr TwosComplement_    ; 栈上的 max 取负
        pha
        lda #$00
        cmp Ens.0.speedSubPixelX,x   ; 73-74：只置借位，A 仍为 $00
        sbc Ens.0.speedX,x           ; 75：A = $00 - speedX - (sub>0 ? 1 : 0)
    @endIf_E:
    cmp MetroidMaxSpeed,y      ; A（负速的"幅值低字节"）与无符号 max 比
    pla
    bcc @endIf_F               ; 小于 ⇒ 不钳位
        sta Ens.0.speedX,x
        lda #$00
        sta Ens.0.speedSubPixelX,x
    @endIf_F:
```

关键在 `CMP` **不写 A**：`lda #$00` 后 A 恒为 0，`sbc speedX` 得 `A = -speedX - (sub!=0)`，即负速的「幅值低字节」**只由整数速度与"小数是否非零"决定，丢掉小数值本身**。这是原版 quirk（D4 保真项），不是笔误可改。

Rust：`crates/game-core/src/enemy/metroid.rs:76-89`

```rust
fn clamp_axis(speed: i8, sub: u8, max: u8) -> (i8, u8) {
    let magnitude_low = if speed < 0 {
        (0u16).wrapping_sub(((speed as i16 as u16) << 8) | sub as u16) as u8
    } else { speed as u8 };
    if magnitude_low >= max { (if speed < 0 { max.wrapping_neg() as i8 } else { max as i8 }, 0) }
    else { (speed, sub) }
}
```

它算的是 **16 位真幅值的低字节**（`0 - (speed<<8|sub)`），与 ROM 的字节形不等价。

对分歧帧算账（`max=2`）：

| 量 | ROM（emu） | Rust |
|---|---|---|
| `-speedX-(sub!=0)` vs 16 位幅值低字节 | `0 - $FF - 1 = $00` = 0 | `0 - $FFD0 = $0030`，低字节 $30 = 48 |
| `0 >= 2` ? | 否 ⇒ **不钳位** | 48 ≥ 2 ⇒ **钳位**：speedX←−2、sub←0 |
| 随后加速段 `speedSubPixelX += accelX($D0)` | `$D0+$D0=$1A0` ⇒ sub=$A0、进位 1；`speedX = $FF+$FF+1 = $FF` | 钳位后 sub=$00 ⇒ `$00+$D0=$D0`、进位 0；`speedX = $FE+$FF = $FD` |
| `EnsExtra.subPixelX += 新sub` | `$D0+$A0` ⇒ $70、进位 1 ⇒ 位移 = `$FF+1 = 0` | `$D0+$D0` ⇒ $A0、进位 1 ⇒ 位移 = `$FD+1 = $FE = −2` |
| 实测 | `FF / A0 / 70 / x=8B` | `FD / D0 / A0 / x=89` |

四列与 §4.2 的实测逐字节一致 ⇒ 根因闭合。

### 4.4 影响面与最小复现

- 受影响：Tourian 全部 Metroid（red/green 两型）；只要「负速度 + 小数速度非零」就触发，故从房间滚动开始数帧即出现。
- 更早的症状：slot 3（另一只 Metroid）在**采集窗第一帧 f1660** 就已有 `EnsExtra.subPixelX` 分歧（emu `$40` / rust `$80`，位置尚未分叉），同根因——说明误差在窗口开始前已累积。
- 最小复现：`--area 3 --room 10`，首次像素分歧 g=1683/c=1682（286px）；首次字段分歧 c=1660（`EnsExtra.subPixelX`）。
- 修复方向（本卡不改代码，仅供后续卡）：`clamp_axis` 的负速幅值改为 `0u8.wrapping_sub(speed as u8).wrapping_sub((sub != 0) as u8)`，其余保持；补一个以「speed=−1、sub=$D0、max=2 ⇒ 不钳位」为矢量的单测（当前实现会错误钳位）。

## 5. 族 B：Ridley room 37 —— Dessgeega（天花）动画相位错（独立根因）

### 5.1 OAM 证据（A 级）

首个像素分歧帧 c=1679 / g=1680，92px。分歧对象是 **2×2 metasprite**，tile `$E2/$E3/$E4/$F2/$F3`、attr `$A3/$E3`（调色板 3、水平翻转、优先级）——Ridley bank 的 dessgeega 帧表（`refs/m1disasm/SRC/ridley/enemy_sprite_data.asm`，`EnAnim_DessgeegaIdleCeiling` 段在 `EnAnimTable` 偏移 `$44/$45`，`EnAnim_DessgeegaCeilingHopping` 在 `$48`）。

| c | g | diff | 分歧槽 | emu (x,y,tile,attr) | rust | 差异字段 |
|---|---|---|---|---|---|---|
| 1678 | 1679 | 0 | — | — | — | OAM 无差，但 Ens 已分歧（§5.2） |
| **1679** | **1680** | **92** | **0** | **164,175,E2,A3** | **164,175,E4,A3** | **tile（动画帧）**；槽 2/3 y +4 |
| 1680-1684 | 1681-1685 | 92 | 0 | 164,172/162,172/162,168/160,168 | 同 x,y，tile E4 | tile 恒定差 |
| 1690 | 1691 | 155 | 0 | 154,148,E4,A3 | 154,138,F4,A3 | y −10 + tile |
| 1697 | 1698 | 65 | 6 | 146,128,E4,A3 | 146,118,F4,A3 | 第二只 dessgeega（槽 6-7、10-11） |
| 1729 | 1737 | **26861** | 0 | 128,148,E4,A3 | 128,138,F4,A3 | 尖峰：多对象同时错帧 |

⇒ 位置 x 一致、**y 与 tile 差固定量**（y 差 ±4/±10 来自不同动画帧的放置表位移），这是**动画帧错**，不是移动积分错。级联槽是同一只 metasprite 的其余 tile 与后续 dessgeega。

### 5.2 敌人槽字段证据（A 级）

分歧敌人 = **slot 2**（`$0420`，type `$03` = 「ceiling dessgeega」，见 `refs/m1disasm/SRC/prg5_ridley.asm:161-179` 的 AI 表与注释）。

| c | g | animFrame | animIndex | resetAnimIndex | jumpDsplcmnt | EnsExtra.data1F | animDelay |
|---|---|---|---|---|---|---|---|
| 1677 | 1678 | `33/33` | `45/45` | `44/44` | `08/08` | `C0/C0` | `01/01` |
| **1678** | **1679** | **`33/34`** | **`45/48`** | **`44/48`** | `08/08` | `C0/C0` | `06/06` |
| 1679 | 1680 | `33/34` | `45/48` | `44/48` | `0B/0B` | `C0/C0` | `05/05` |

同帧位置、速度、data05 全部一致，**唯一分歧是动画三元组** `animFrame/animIndex/resetAnimIndex`。emu 在 1678 把动画从 idle 续到下一帧（`$44` 组的第 2 帧），rust 却切到 hopping（`resetAnimIndex←$48`）。

### 5.3 根因推导（B 级）

ROM：`EnemyFlipAfterDisplacement ; 07:FB88`（`refs/m1disasm/SRC/prg7_engine.asm:12271-12323`）：

```asm
    lda EnsExtra.0.jumpDsplcmnt,x
    inc EnsExtra.0.data1F,x
    dec EnsExtra.0.data1F,x     ; data1F 恒等，但**置 N/Z**（N = bit7 of data1F-1）
    bne @endIf_A                ; data1F-1 != 0 ⇒ 跳过 pha/pla，沿用 dec 的标志
        pha
        pla                     ; 6502 的 PLA **会置 N/Z** ⇒ 用 jumpDsplcmnt 自身符号
    @endIf_A:
    bpl @endIf_B                ; N=0 ⇒ 视为正，跳过取负
        jsr TwosComplement      ; N=1 ⇒ 取绝对值
    @endIf_B:
    cmp #$08
    bcc Lx384                   ; <8 ⇒ 回休止动画
    cmp #$10
    bcs @RTS                    ; >=16 ⇒ 什么都不做（保留当前动画）
    ...                         ; 8..15 ⇒ 切到 EnemyFlipAfterDisplacementAnimIndex（此处 $48=hopping）
```

即符号判据是**两路**：`data1F==0` 时用 `jumpDsplcmnt` 的符号；`data1F!=0` 时用 `data1F-1` 的 bit7。本帧 `data1F=$C0` ⇒ `$C0-1=$BF`，N=1 ⇒ 取负：`$08 → $F8` ⇒ `cmp $10` 进位 ⇒ **直接返回，不换动画**。emu 正是如此（1678 只做 idle 内续帧）。

Rust：`crates/game-core/src/enemy/sidehopper.rs:51-74`

```rust
let mut d = mem.read_wram(sq + F_JUMP_DSPL);   // $6B01
if (d as i8) < 0 { d = (!d).wrapping_add(1); } // 只看 jumpDsplcmnt 自身符号
if d < 0x08 { prep::init_en_resting_anim_index(...); return; }
if d >= 0x10 { return; }
... 切 hopping（写 animIndex=idx-1、resetAnimIndex=idx、清 delay）
```

完全没读 `EnsExtra.data1F`（`$6B03`）。所以 `d=08` 被当成正数走进 8..15 分支，切到 `$48`；随后 `UpdateEnemyAnim` 从 `$47` 读帧，得到 `$34`（另一张图）而不是 `$33`，并因放置表不同使 y 差 4/10 —— 与 §5.1 的 OAM 差异逐一对应。

### 5.4 影响面与最小复现

- 受影响：Ridley bank（area 4）的 dessgeega（type 2/3，floor/ceiling）在「跳跃位移 8..15px 且 data1F≠0」的所有帧；其它用 `EnemyFlipAfterDisplacement` 的敌人同受影响（该子程序是公共件，`CommonJump_EnemyFlipAfterDisplacement` 被 sidehopper/ripper 等调用）。
- 最小复现：`--area 4 --room 37`，首个 EnS 分歧 c=1678，首个像素分歧 c=1679（92px）；尖峰 c=1729（26861px）。
- 修复方向（本卡不改代码）：在 `enemy_flip_after_displacement` 里按 ROM 复刻符号判据——
  `let n = if d1f == 0 { jump & 0x80 != 0 } else { (d1f.wrapping_sub(1) & 0x80) != 0 }; if n { negate }`；
  并补单测：`data1F=$C0, jump=$08 ⇒ 不切动画（resetAnimIndex 不变）`、`data1F=0, jump=$F8 ⇒ 取负后 ≥16 ⇒ 不切`。

## 6. 族的划分（同一根因 vs 独立根因）

- **族 A（Tourian）= 独立根因**：Metroid 限速钳位字节形。证据：OAM 只错 x/y，tile/attr 恒同；Ens 分歧字段只出现在 `speedX/speedSubPixelX/EnsExtra.subPixelX`；根因代码在 `metroid.rs`。
- **族 B（Ridley）= 独立根因**：dessgeega 动画切换缺 `data1F` 门。证据：OAM 只错 tile/y，x 恒同；Ens 分歧字段只出现在 `animFrame/animIndex/resetAnimIndex`；根因代码在 `sidehopper.rs`。
- 两族**互不覆盖**：修 A 不会改变 B 的 `animIndex` 分歧，修 B 不会改变 A 的 `speedX` 分歧；实现上分属两个文件、两条 asm 子程序。
- 与既有卡的关系：`#5`（每帧工作量计费/丢帧）是**时间面**（dFC≠0 的丢帧），本卡两房在 dFC==0 的配对下仍有像素差，且本卡已证明差异在精灵页字段，**不是** `#5` 的丢帧族。

## 7. 假设区（C 级，未验证，禁止据此改代码）

1. **C-1**：Ridley 后段（c≥1690）y 差从 4 增大到 10，是否仍是族 B 的级联（动画帧不同 ⇒ 放置表不同 ⇒ y 不同），还是叠加了第二种「跳跃位移/jumpDsplcmnt 更新」差异——本卡只验证了 c=1678 的动画切换路径，未逐帧验算后续 jump 曲线。
2. **C-2**：Tourian c=1722 起出现的 y 差（−5 → −18）是否纯由 x 漂移导致 Metroid 撞墙/换向后的二次效应；未单独隔离「y 轴钳位」路径（同一 `clamp_axis` 也服务 y，机制应同，但未取 y 的逐帧算账）。
3. **C-3**：Tourian 的 `MetroidMaxSpeed` 取 `movementIndex`（red=2/green=4）两腿一致，但本卡未验证 green 型（max=4）的钳位阈值路径；族 A 的修法对两种 max 同式。

## 8. 未覆盖 / 遗留

- 未覆盖：Ridley room 37 之外的 area 4 房间（29/34/38）是否有族 B 的其它表现；Tourian 其它房间的 Metroid 数量更多，误差幅度未扫。
- 未覆盖：`EnemyFlipAfterDisplacement` 的调用者全集（ripper 等）未逐一验算。
- 未执行：任何代码修改、门禁、L4 重跑（本卡 writeScope 仅 `docs/findings/`）。
- emu 侧已在本轮等到空窗重跑成功（`DONE` + `oam_nmi.txt` 80478B，NMI `$C0D9` 入口逐帧抓 `$0200-$02FF`）。两种取法交叉校验：**NMI 入口页 `nmi[k]` 与采点 `ram.hex` 页 `ram[k]` 在 85 个可比帧上逐字节相同（mismatch 0）**，都等于 `work(k)` 收口页；显示帧 g 用 `work(g-1)`，故索引一律 g−1。用 NMI 页重算，首分歧仍是 c=1682/g=1683、槽 0、`x` 223→221（286px），与 `ram.hex` 取法完全一致。

## 9. 证据清单（可复现指针）

| 结论 | 级别 | 指针 |
|---|---|---|
| 残差只落精灵页 | A | §3，`ppu.hex` 分层全 0；`analysis/scratch-sprite-residual/*/*.oam.json` 的 `ppu` 字段 |
| Tourian 首分歧 c=1682/g=1683，槽 0 x+2 | A | §1.3 命令；`case-sprite-t10.oam.json` |
| Tourian 根因 = Metroid 钳位字节形 | A/B | `refs/m1disasm/SRC/enemies/metroid.asm:60-85`；`crates/game-core/src/enemy/metroid.rs:76-89`；§4.3 算账表与 §4.2 实测逐字节一致 |
| Ridley 首分歧 c=1679/g=1680，槽 0 tile | A | §1.3 命令；`case-sprite-r37.oam.json` |
| Ridley 根因 = 缺 `data1F` 门 | A/B | `refs/m1disasm/SRC/prg7_engine.asm:12271-12323`；`crates/game-core/src/enemy/sidehopper.rs:51-74`；§5.2 实测与 §5.3 推导一致 |
| 两族独立 | B | §6（字段面互斥 + 代码位置分离） |
