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
