# TAS 首分歧 f22895 的 A-2：`$0405` bit6 相位差——定界报告

> 任务：#79（`tsk_01M8AVJXN7ZYWFHE04YREC`）。本文件只写定界结论与复现命令，不改判据、不改 `analysis/tas-100/coverage.json`。
> 前置：A-1（死亡重开清敌）已由 #71 修复，f22895 的差异从 11 字节降到 8 字节；剩下 8 字节即本篇对象。

## 0. 结论

**A-2 的 `$0405` bit6 是症状，不是根因。**

- 症状：f22895 起，六个敌人槽的 `data05` bit6 与 emu 整组反相，并逐帧持续（seg2 之后 7086 帧 diverged 即此形态）。
- 根因：**主循环迭代计数（`FrameCount` / `TimerDelay` 相位）从首次 GameOver 屏起就对不上**——Rust 在同一段帧门等待里比 ROM 多跑迭代，累计到重开窗时 `FrameCount` 的奇偶性与 ROM 相反；bit6 的生成式 `(slot xor FrameCount) & 1` 于是整组反相。
- 首破帧：**f4024**；f1–4019 两边 `FrameCount`/`TimerDelay` 逐帧相等。
- 落点：`crates/game-core/src/title/mod.rs:1400-1445`（1C GameOver 的 4 段 3 spin）。**超出 #79 卡面写面（`world/` + `frame/`）**，按卡面边界停手待裁。

## 1. 环境指纹

| 项 | 值 |
|---|---|
| 录像 | `movies/lordtom-metroid-100p.inputlog` |
| emu 逐帧（f0–10000） | `analysis/probe-early/traces/probe-early.frames.jsonl`（9964 行，`DONE frames 10000`） |
| emu 逐帧（f10000–11000） | `analysis/probe-mid/traces/probe-mid.frames.jsonl`（从 `analysis/tas-100/checkpoints/cp-010000` 续跑，`DONE frames 11000 trace_lines 1000`） |
| emu 逐帧（f20000–22950） | `analysis/probe-a2/traces/probe-a2.frames.jsonl`（2940 行） |
| rust 逐帧 | `analysis/tas-100/rust/full.state.jsonl`（主线 `847fac49` 整片重跑产物） |
| 工具 | `tools/scenario_run.py`（`snapshot`/`resume_at`/`sample_point spin-exit`/`watch`） |

取址注意：`w.state.jsonl` 的 `w.wram` 是 `$6000-$7FFF`，`$0400-$04FF` 在 **`w.iram`**（`bytes.fromhex(...)[0x300:0x400]`）；`$6AF4` 才在 `wram[0xAF4:]`。

## 2. 证据

### 2.1 帧计数相位：首破帧 f4024（A 级）

`watch 002d`（`FrameCount`）/`watch 0029`（`TimerDelay`）：

| 帧 | emu | rust |
|---|---|---|
| f3000 | `FC=157 TD=7` | `FC=157 TD=7` |
| f4017 | `FC=151` | `FC=151` |
| f4019 | `FC=152` | （无采样行） |
| f4022 | （无采样行） | `FC=155` |
| f4024 | `FC=153` | `FC=157` |

⇒ 同一段 GameOver 等待：ROM 5 帧只推进 1 次迭代（`152→153`），Rust 4 帧推进了 3 次（`152→155`）。**累计偏移的精确值用「FC 差展开」量**（见 2.2）：f4024 一次跳到 **+4**。

### 2.2 偏移是跳变不是匀速（A 级，2026-11-26 补正）

对 f1–10000 逐帧做 `FC_rust - FC_emu` 展开（模 256 消绕），偏移只在两处跳变：

| 帧 | 偏移 | 对应 |
|---|---|---|
| f1–4019 | 0 | 完全一致 |
| **f4024** | **+4** | 首次 GameOver 段结束 |
| **f4187** | **+9** | 同一 GameOver 窗内第二段等待结束 |
| f4187–10000 | 恒定 +9 | 之后不再漂移 |

f4179–4185 更露骨（同一窗内）：emu `f4181 FC=54` → `f4187 FC=55`（6 帧 1 次迭代），rust `f4179 FC=56` → `f4185 FC=62`（6 帧 **6 次**迭代——这段等待 rust 完全不跨帧）。⇒ 前面的「只少 2 帧」是低估：**Rust 在这个 GameOver 窗里有两段等待根本没建模**，合起来 +9。

### 2.6 漂移的绝对读数（A 级）

- f10000（`probe-mid`）：emu `TD=6 FC=236`，rust `TD=4 FC=245` ⇒ `FC` 差 9（与 f4187 后的恒定 +9 一致），`TD` 差 −2 ⇒ `TD+FC` 差 7 ≡ −3（mod 10）。
- f22450：emu `TD=7 FC=142`，rust `TD=4 FC=151` ⇒ `FC` 差 9（奇）。
- 重开窗（`MoreInit`→`SamusIntro`）：emu `SamusInit` f22460、`SamusIntro` f22461、`Timer3=0 & MR=3` f22895、GameEngine f22896；rust 对应 f22462 / f22463 / f22894 / f22895 ⇒ 晚进、早出，净早 1 帧。

### 2.3 bit6 生成式与 8 字节（A 级）

- ROM 写点：`UpdateEnemy_UpdateEnData05Bit6 ; 07:F3AA`（`refs/m1disasm/SRC/prg7_engine.asm:10729`，注释 "toggle bit 6"），被 `UpdateAllEnemies ;07:F345`（`:10655`）对 `status=0` 的槽也调用（`:10683`）。
- Rust 实现：`crates/game-core/src/enemy/slot_cycle.rs` 的 `update_en_data05_bit6(mem,x)`：`parity = ((x >> 4) ^ FRAME_COUNT) & 1`。
- 对照（f800/f801/f1000，emu seg0 vs rust `iram`）：**全 0 差异** ⇒ 公式与实现一致，不一致的只有 `FrameCount` 的相位。
- f22895 八字节：六槽 `$0405`（`5A/1A`、`1E/5E`、`56/16`、`00/40`、`58/1A`、`18/5A`）+ 槽4 `$0449/$044A`（`01/00`）。emu 采样 `FC=0x43`（奇）⇒ 槽0 bit6=1；rust 采样 `FC=74`（偶）⇒ 槽0 bit6=0。

### 2.4 排除项（A 级，均实测）

- **不是敌人 AI 差异**：A-1 修复后 `$0400/$0401/$040A` 已归零，剩余 8 字节与敌人行为无关（`$0405` 只是被主循环每帧重写）。
- **不是重开清槽差一次迭代**：插桩实测 `more_init` 只进 1 次，`settle_iteration` 期间 `gates=0 / span_left=0 / span_fc=false`，与 ROM 相同；我早前 seq3 的「MoreInit FC +2」是口径错误（拿入口 FC 与后续采样行相减），已更正。
- **不是 `update_timer` 有无条件递减**：把 `main_loop_iter` 的 `update_timer` 移出 `if !continuation` 后整树相位前移 10 次迭代，`$0400-$04FF` 差字节 8→30+（破坏性），已还原。

### 2.5 实验：给 GameOver 补 2 帧等待（A 级，未提交）

在 `title/mod.rs` 的 1C GameOver `cursor1`、`cursor2` 各加 `ShellCall::SpinWait(1)`（备份 `/tmp/r79-title.orig`，跑完已还原，`git status` 干净）：

| 帧 | emu | 原 rust | 补 2 等待后 |
|---|---|---|---|
| f22450 | `FC=142` | `FC=151`（+9） | `FC=149`（+7） |
| f22895 | `MR=3` | `MR=3` | `MR=8` |

⇒ 方向对（少跑 2 次迭代），但**只回收了 2**：剩余 7 仍落在**同一个 GameOver 窗内**（§2.2 的逐帧展开显示 f4187 之后再无第二次跳变，说明这 5 次追加漂移也发生在该窗，即 4 段 3 spin 里还有未建模的等待）；本实验只能证明「该窗内还有等待没建模」，**不能推断是门滚动/载房/名字表写**——那是未验证候选，需在该窗内继续逐段定位。另外补错位置会把 `MainRoutine` 相位推错（f22895 rust 变 `MR=8`），**不能只补 GameOver 就收工**。

## 3. 待办（需要裁决）

1. A-2 的修法本质是**逐段审计帧门等待**，且**首要范围已收敛到 title GameOver 这一个窗**（f4024 +4、f4187 +9 两次跳变都在该窗内）；门滚动/载房/名字表写是**未验证候选**，没有实测首破帧支撑，不能据本报告去那几处立项；
2. 若把 #79 写面扩到 `crates/game-core/src/title/`，可先修 GameOver 的 5 帧构成（ROM `prg0_title.asm:2861-2876`：`ClearAll`（ScreenOff+WaitNMIPass）→ `PreparePPUProcess_` → `InitPasswordFontGFX` → `NMIOn` → `Timer3←$10` → `jmp ScreenOn`）；
3. 逐段审计需要 emu 在 11000–20000 段的逐帧基准（当前只有 0–11000、20000–22950），建议补一段 `probe` 或直接用 Mesen trace 全片。

## 4. 复现命令

```bash
# emu 逐帧（f0–10000）
cat > analysis/probe-early/scenario.txt <<'EOF'
name probe-early
frames 10000
inputlog movies/lordtom-metroid-100p.inputlog
sample_point spin-exit
watch 001e
watch 002c
watch 002d
watch 0029
watch 0300
EOF
env GUNNAC_TRACES=$PWD/analysis/probe-early/traces GUNNAC_SHOTS=$PWD/analysis/probe-early/shots \
    GUNNAC_MESEN=$PWD/tools/mesen2/Mesen \
    python3 tools/scenario_run.py run analysis/probe-early/scenario.txt --timeout 900

# 首破帧比对（emu vs rust）
python3 - <<'PY'
import json
from pathlib import Path
def b(w,k):
    v=w.get(k); return None if v is None else (v if isinstance(v,int) else bytes.fromhex(v)[0])
E={}
for line in Path('analysis/probe-early/traces/probe-early.frames.jsonl').read_text().splitlines():
    d=json.loads(line); E[d['f']]=b(d['w'],'002d')
R={}
for line in Path('analysis/tas-100/rust/full.state.jsonl').read_text().splitlines():
    d=json.loads(line); f=d['f']
    if f>10000: break
    R[f]=bytes.fromhex(d['w']['zp'])[0x2d]
common=sorted(set(E)&set(R))
print('first FC-diff', next((f for f in common if E[f]!=R[f]), None))
PY
```

## §7 2026-11-26 追加：A-2 首因定谳（FC 增量时机）+ 五种修法实测与两案互斥证据（卡 #82）

### 7.1 首因（A 级，两条独立读数）
1. **emu 探针**：`analysis/probe-go/scenario.txt`（`frames 4300`、`inputlog movies/lordtom-metroid-100p.inputlog`、`sample_point spin-exit`、watch `001e/001f/001a/002d/0029/00fe/00ff`），`python3 tools/scenario_run.py run analysis/probe-go/scenario.txt` → `analysis/probe-go/traces/probe-go.frames.jsonl`（4264 行）。读数：
   `f4017 TR=16 FC=151` → `f4019 TR=1C FC=152`（`$00FE`=06 熄屏）→ f4020–4023 无采样（阻塞 4 帧）→ `f4024 TR=19 FC=153`（`$00FE`=1E）；第二段 `f4181 TR=19 FC=54` → f4182–4186 阻塞 → `f4187 TR=1A FC=55`。
   ⇒ **整个 GameOver（0x1C）通过只推进 1 次迭代**（跨 5 帧）。
2. **asm**：`GameOver ; 00:939E`（`refs/m1disasm/SRC/prg0_title.asm:2861-2876`）= `jsr ClearAll` → `PreparePPUProcess_` → `InitPasswordFontGFX` → `NMIOn` → `Timer3←$10` → `jmp ScreenOn`。其中 `ClearAll ; 00:909F`（`:2320`）含 ScreenOff 的 `WriteAndWait→WaitNMIPass_ ;07:C43F` 与 `jsr WaitNMIPass`（2 次等待），`ScreenOn ; 07:C447` = `PPUMASK|=0x1E` 后 `WriteAndWait`（1 次等待），`InitPasswordFontGFX ; 07:C6D6` 内两次 `CopyGFXBlock ; 07:C7D5` 只写 `$2007` 不等 NMI。这些等待**都在同一次 MainLoop 迭代内**，末尾 `inc FrameCount`（`prg7_engine.asm:207`，位于分发之前）**只发生一次**。
   > **【2026-11-26 更正（白砚复核，A 级）】上句「位于分发之前」写反了。**实测 `awk 'NR>=196&&NR<=214' refs/m1disasm/SRC/prg7_engine.asm`：`MainLoop: ; 07:C0BC` → `:203 jsr CheckBankSwitch` → `:204 jsr UpdateTimer` → **`:205 jsr GoMainRoutine`** → **`:207 inc FrameCount`** → `:209-214 lda #$00 / sta NMIStatus / @loop_waitNMIEnd` 自旋等 NMI；`grep -rn "inc FrameCount" refs/m1disasm/SRC/*.asm` 全仓仅 `prg7_engine.asm:207` 一处。
   > ⇒ ROM 的一次迭代 = **例程体（可内含多次 NMI 等待）→ inc FC → 等下一次 NMI**；即 inc 在例程体**之后**，不在之前。"链内读到已自增的 FC"这一句因此作废。
3. **Rust 侧**：`crates/game-core/src/frame/tick.rs::main_loop_iter` 每拍尾声 `if !mem.title_no_return && !mem.fc_inced_this_tick { FC += 1 }`，而多段 title 链被拆成多拍 ⇒ GameOver 一次通过多推 4、随后再 +5（累计 +9）；下游 `UpdateEn_UpdateEnData05Bit6 ;07:F3AA`（`prg7_engine.asm:10729`）按 `(slot xor FrameCount)&1` 写 `$0405` bit6 ⇒ f22895 六槽整组反相（8 字节）。

### 7.2 五种修法实测（全部本机实跑；判据=FC 漂移 + `run-all` + password 案）
判 FC 漂移的方法：`replay`/`rust_screenshot --state-out` 的 `w.zp[0x2d]` 对 emu `analysis/probe-early/traces/probe-early.frames.jsonl` 的 `w.$002d`（同为同一 inputlog，f1–9964 逐帧）。

| 变体 | 判据 | f4024 | f4187 | f10000 | f22450 | f22895 | password 案 | run-all |
|---|---|---|---|---|---|---|---|---|
| 基线 | 每拍尾声 inc | +4 | +9 | +9 | +9 | +7 | 14/14 | 171/172 |
| v1 | 只判 `cursor>0 && phase==TITLE_ROUTINE` | +2 | +3 | — | — | — | — | — |
| v2 | 只判 `cursor>0` | +1 | +2 | — | — | — | — | — |
| **v3** | `cursor>0` 起点 **或** 本拍刚进链 ⇒ 不 inc | **0** | **0** | **0** | **0** | **-1** | **12/14**（f950 71 px、f1150 94 px） | **169/172** |
| v4 | 链头抑制只在 `title_no_return` 起点为真 | — | — | +2 | — | +1 | — | — |
| v5 | 链头抑制只在「本拍换相位」 | — | — | +1 | +1 | **0** | — | — |
| v6 | inc 挪到迭代起始（对所有新迭代头） | — | — | — | — | — | **12/14**（f700 71 px、f1100 90 px） | — |

### 7.3 两案互斥的证据（为什么本轮不能落 v3）
- v3 让 FC 漂移全归零、`replay --frames 4300` 分歧 **0/4300**、`replay --frames 24000` 首分歧从 `$0405`（8 字节）后移到 `$0445`（4 字节：`$0445`/`$0455` bit1、`$0459`/`$045a`）——即「FC 相位」这条根因确已消除。
- 但 `run-all` 由 171/172 退到 **169/172**：`case-password-justin` 的 `f950 mid-3` **71 px**、`f1150 pw-done` **94 px**（`--fresh-emu` 连跑两次稳定，非缓存）。
- 这两帧的 FC 与 emu **相等**（f950 都是 160、f1150 都是 104）⇒ 不是「FC 又错」。
- > **【2026-11-26 更正（白砚复核 seq17，A 级）】本句作废。** 实读 v3 下的 `$002D`：f950 rust=**169** / emu=**171**、f1150 rust=**113** / emu=**115**（`shots_rust/case-password-justin_f00950_mid-3.ram.hex` 的 `$2D` 与 `analysis/traces/case-password-justin.frames.jsonl` 的 `w.FC`），即 **v3 每拍 −1、到此已累计少 2**；§8 的 v3 行「每拍 −1」也记着同一事实。故「FC 又错」恰是 71/94 px 的直接原因（闪烁/光标按 FC 位选帧）。
- > **【2026-11-26 更正（白砚复核，A 级）】本段原把差因归为「ROM 的 inc 在分发之前、链内读已自增、v3 让链内读旧值」——该归因建立在写反的调用序上，作废。**按更正后的顺序（例程体 → inc → 等 NMI），v3「链内各段读旧值、链尾 inc 一次」才是与 ROM 同相的读法，故 v3 在 f950/f1150 引发 71/94 px 的**真实原因尚未查明**（假设待验区 A-2-a）。
- > **【2026-11-26 更正（白砚复核 seq17 + 我的逐迭代读数，A 级）】上句「真实原因尚未查明」作废。** 原因已查明并写入 §8：**v3 的判据过宽**——`tr=0x18`（EnterPassword）的值是**常驻相位**而非链（逐迭代打印 `cursor` 恒 0x00、`gates=0`），v3 把它每拍都当「链内」抑制 ⇒ 每拍少 1 次 inc，到 f950 累计 −2。另：本段列的假设 A-2-a 已在 `4c28b9c8` 之后被**0 命中**否证（lane 内插桩跑 24000 帧，`continuation && title_cursor==0` 命中 0 次），不再作为待验假设。
- 待验假设 A-2-a（可证伪）：v3 让链在 `cursor` 回 0（而非 0xFF 哨兵）时结束的那条路径**丢了那一次 inc**，于是链后第一拍的 FC 比 emu 低 1，而密码屏的闪烁/光标在这一拍采样就锁进自身状态（事后 FC 对齐也回不来）；验证命令=在 v3 上打印走到密码屏两次采样前一拍的 `FC` 与 `title_cursor`，与 emu `analysis/probe-pwd-dark` 同帧 `$002d` 比。
- v6（把 inc 整体挪到迭代起始）反而把**另两格**打掉（f700/f1100），与既有 `main_loop_iter` 尾部注释记的「轮26 形重试被 L4 证伪」一致（> 【2026-11-26 更正】原文此处还写着「最贴近 asm」，按 §7.1 更正后的调用序，v6 把 inc 挪到迭代起始**恰恰是与 asm 相反**的动作；该措辞作废。） ⇒ 不能靠单点移动时机解决，需要「链内 FC 相位」与「span 收口」一起重构。

### 7.4 结论与后继判据
- **首因已定谳**（不是 `$0405` 的写点问题，是 FC 增量时机与多段链的映射问题），**本轮不改码**（v3 会把 password 案打回 169/172，属「修好 A 必打穿 B」）。
- 后继卡的验收必须**同时**满足：①`replay --frames 24000` 的 FC 漂移全程 0（含 f22895 前后）；②`run cases/case-password-justin.txt` 14/14（含 f950/f1150/f700/f1100）；③`run-all` ≥171/172；④`cargo test -p game-core` 全绿 + `check` PASS24。
  > **【2026-11-26 补】本条 4 条判据的现行版本以 §8.2 为准**（§8.2 增补了「`cargo test -p game-core --lib` 全绿」「probe-go 口径下 f4024/f4187 漂移 = 0」「首分歧不早于 f22895」三条，且把 14/14 明确为 f950/f1150/f1191/f1430 四点全 0）。
- 复算命令：
```bash
# 1) FC 漂移（rust 侧逐帧 state）
cargo run -q -p game-core --bin rust_screenshot -- \
  --schedule /tmp/g82b.sched --out /tmp/g82out --state-out /tmp/g82.jsonl
python3 - <<'PY'
import json
from pathlib import Path
def b(w,k):
    v=w.get(k); return None if v is None else (v if isinstance(v,int) else bytes.fromhex(v)[0])
E={}
for line in Path('analysis/probe-early/traces/probe-early.frames.jsonl').read_text().splitlines():
    d=json.loads(line); v=b(d['w'],'002d')
    if v is not None: E[d['f']]=v
R={}
for line in Path('/tmp/g82.jsonl').read_text().splitlines():
    d=json.loads(line); R[d['f']]=bytes.fromhex(d['w']['zp'])[0x2d]
prev=None
for f in sorted(set(E)&set(R)):
    dd=(R[f]-E[f])%256
    if prev is not None and dd!=prev: print('jump', f, prev, dd)
    prev=dd
PY
# 2) 两案互斥的最小复现
python3 tools/l4_diff_run.py run cases/case-password-justin.txt --fresh-emu
python3 tools/l4_diff_run.py run-all
```

### 7.5 复核退回后的状态（2026-11-26，白砚 seq3 退回）

| 退回项 | 我的处置 |
|---|---|
| ② 未达成 + 卡面无「否证交付」条款（零代码改动、基线首分歧仍是 f22895 `$0400`） | 未自行改口径；已请 PMO 在 thread 裁 Ⅰ（继续修）／Ⅱ（改 ② 口径或拆 findings 卡） |
| A 级调用序写反（`inc FrameCount` 在 `:205 jsr GoMainRoutine` 之后、`:207`） | 已按 AGENTS §3.6 在 §7.1/§7.3 原文原地加日期更正（原文保留），并把基于该反向事实的两处归因（「链内读已自增」「v6 最贴近 asm」）标作废，新假设 A-2-a 给可证伪命令 |

**对后续修法的影响（重要）**：既然 ROM 的 inc 在例程体之后，那么「多段链」的正确映射是**每拍不 inc、链结束的那一拍 inc 一次**（v3 的方向），而不是「链首前置自增」（我 seq4 提的 span 化方案里那句「FC 在 span 首 tick 前置一次」按此更正为：**span 首 tick 不自增，span 收口 tick 自增一次**）。下一步按 A-2-a 先测「链尾是否真的有那一次 inc」再决定改哪里。

## 8. 修法矩阵（2026-11-26，逐迭代实测；全部在 lane 上跑、跑完即 `git checkout --` 还原）

判据候选与其失败点（同一 password 案 sched，2587 次迭代；基线=干净 HEAD 的逐迭代 FC 序列）：

| 变体 | inc 判据 | 首分歧（vs 基线，迭代序号） | 末端 FC（base/v） | L4 password | 结论 |
|---|---|---|---|---|---|
| 基线 | 每拍尾 `inc` | — | 20/20 | 14/14 | 多推 4 次（f4024 +4 / f4187 +9） |
| v3 | 游标非零即抑制 | idx=500（238/237） | 每拍 −1 | **12/14**（f950 71 px、f1150 94 px） | 过宽：`tr=0x17→0x18` 那条链每拍都是完整迭代 |
| v7 | 本拍声明了 NMI 等待门则抑制 | idx=1186（156/155） | 20/19 | — | 方向对，但排空拍没有补回那 1 次 inc |
| v8 | v7 + 抑制时置 `iter_span_fc` | idx=3（2/3） | 20/20 | — | 总量对、落点整体早一拍 |
| v9 | v7 + 门队列非空才置 `iter_span_fc`，尾部 inc 加「门/span 皆空」门 | replay 4300 首分歧 **f467**（`$0405` 8 B） | — | **14/14** | password 过了，但 `cargo test -p game-core --lib` **13 个既存测试红**（cannon/crawler/dragon/ai_table 等），check FAIL |

**v9 的读数说明**：`run cases/case-password-justin.txt --skip-emu` 14/14（f950/f1150 回 0、f1191/f1430 仍 0）；但同一棵树上 `replay --frames 4300 --allow-rust-missing` 首分歧从「无」劣化到 f467，且 13 个敌人侧单测红 ⇒ 「迭代尾 inc 的时机」不是一个可以只按 title/密码径改的局部判据，它同时是敌人 AI 帧门（`data05` bit6 等）的公共时钟。

### 8.1 精确口径（供后继卡直接用）

- ROM：**例程体（可内含多次 `WaitNMIPass_`）→ `inc FrameCount`（`prg7_engine.asm:207`）→ 等 NMI**（`:209-214`）。⇒ 一次迭代内所有等待帧共用一个 FC 值，迭代结束才 +1。
- Rust 现状：`main_loop_iter` 每拍尾 `inc`；跨帧等待由 `iter_gates`/`iter_span_left` 表达、由 `engine::settle_iteration` 收口（`iter_span_fc` 单增唯一源 + `#47` 的末门同拍 `pending_fc_inc`）。
- 因此正确的判据是「**迭代是否结束**」，而 `iter_gates` 只描述「下一拍 NMI 内块跑不跑」，两者**不同源**（v7 之所以只差 1 次、v9 之所以打穿敌人测试，都是把这两个信号当成同一个用）。

### 8.2 后继判据（下一张卡必须同时满足）

1. `run cases/case-password-justin.txt` 14/14（f950/f1150/f1191/f1430 全 0）；
2. `cargo test -p game-core --lib` 全绿（≥1087，当前 lane 基线）；
3. `analysis/probe-go` 口径下 f4024 / f4187 的 FC 漂移 = 0；
4. `replay --frames 4300` 首分歧不早于 f22895（当前 `--frames 24000` 为 f22895 `$0405` 8 B）；
5. `run-all` ≥171/172。
