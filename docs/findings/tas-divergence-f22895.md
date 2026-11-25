# TAS 首分歧 f22895 `$0400` 定界（#68）

**状态：定界**。两组根因都已定位到具体指令/文件行；**都不在 `crates/game-core/src/enemy/`**，
按卡面 ③「修必须在 enemy/，若根因在 world/ 或 samus/ 则停手交定界」与 PMO seq5-③ 停手，交本文件。
本卡**没有**改 `crates/`（实验补调已还原，见 §5）。

范围与口径：观察面仍是 TAS 回放工具的比较面 `$0400-$04FF`（`Ens.0..5` 各 16 字节）；
本文件只写本卡结论，`docs/findings/tas-100-round3.md`（#66 已验收）未改动。

---

## 0 结论（先看这里）

| 组 | 现象（f22895） | 根因 | 级别 | 位置 |
|---|---|---|---|---|
| A-1 幽灵槽 | `$0400`(槽0.y) −1、`$0401`(槽0.x) −1、`$040A`(槽0.data0A) +1 | rust `more_init` 漏 port ROM `MoreInit` 的 `jsr DestroyEnemies`(07:C8BB) ⇒ 死亡重开时槽0 status 不清 0，一只已退役的 Crawler 继续跑 AI，在 f22895 走到墙角正常走了一步 | **A** | ROM `refs/m1disasm/SRC/prg7_engine.asm:1874`（`MoreInit ;07:C81D` 内的 `jsr DestroyEnemies`）；Rust `crates/game-core/src/world/room_load.rs:16 pub fn more_init`（`:39 erase_all_sprites` 之后缺调用） |
| A-2 重开相位 | `$0405/$0415/$0425/$0435/$0445/$0455`（六槽 `data05` bit6）整体反相、`$0449/$044A`（槽4 delay/data0A）与 `$0441`（槽4.x）差 1 | 重开后 GameEngine 的恢复时序比原版**早 2 帧**：emu 的 `data05` bit6 从 f22897 起每帧翻转，rust 从 f22895 起翻 ⇒ 同帧相位差 | **A**（相位）/**C**（机制：`more_init`/intro 的帧等待记账多算/少算，尚未定位到具体行） | 相位证据两处：`UpdateEnemy_UpdateEnData05Bit6 ;07:F3AA`（`refs/m1disasm/SRC/prg7_engine.asm:10729`，bit6=(slotIndex xor FrameCount)&1）；rust 侧 `crates/game-core/src/world/room_load.rs:16 more_init`（`iter_wait`/`iter_bogged`/`deferred_screen_on` 段） |

两组不是同一个根因：**实验证明补上 `DestroyEnemies` 后 A-1 消失、A-2 仍在**（§5）。
所以 f22895 要「全相等」需要两张卡：①补 `DestroyEnemies`（world/，本文件给出 A 级修法）；②重开序列相位（world/ 或 frame/，机制待定）。

---

## 1 环境指纹

| 项 | 值 |
|---|---|
| 复现树 | `/home/peng/workspace/wt-roll`（branch `lane/roll`；基线 `3e453ccc`＝主线 HEAD 的 lane 基线，本文件交付在基线之上：内容主体 `bda1fcda`、§9 脚本修正 `d1c237e8`，工作树干净） |
| 主线口径复核树 | `/home/peng/workspace/metroid-decomp`，HEAD `cde7d1b9`，工作树干净 |
| `movies/lordtom-metroid-100p.inputlog` sha256 | `094de4afdc1eff604002bbb144d40f36918833b3c211ed67f0f0b75c231b68b8` |
| rust CLI（`--frames 30000` 那次） | `/home/peng/workspace/wt-roll-target/debug/rust_screenshot`，sha256 `4e9eb066b11501caa131985bdd0fe65ed5efa02af76ab6c722c81631bc8feb1b` |
| Mesen | `tools/mesen2/Mesen`（单实例；本卡所有 Mesen 运行都是串行、跑前 `pgrep Mesen` 为空） |
| 比较面 | `$0400-$04FF`（工具 `--range 400-4FF` 口径；emu `watch_range 0400-04FF ens` ↔ rust `w.iram` 的 `0x300..0x400` 切片） |

## 2 现象复现（主线口径，PMO seq5-② 要求）

`cd /home/peng/workspace/metroid-decomp && CARGO_TARGET_DIR=/home/peng/workspace/wt-roll-target GUNNAC_MESEN=$PWD/tools/mesen2/Mesen python3 tools/tas_replay_diff.py replay --inputlog movies/lordtom-metroid-100p.inputlog --frames 30000 --segments 10000 --allow-rust-missing --out analysis/tas-main-f22895 --timeout 1800`

`analysis/tas-main-f22895/coverage.json`：

```
seg (0,10000)  pass     diverge_frames 0
seg (10000,20000) pass  diverge_frames 0
seg (20000,30000) diverge diverge_frames 7086
first_divergence = {"frame":22895,"field":"$0400","offset":0,"emu":152,"rust":151,"diff_bytes":11}
ran_frames = 30000
```

`git diff --stat 3e453ccc..cde7d1b9 -- crates/` 为空 ⇒ 主线与 lane 的 `crates/` 同源，读数一致。

## 3 f22895 的 11 个差异字节（来源：`analysis/tas-main-f22895`）

> 表来源：emu 侧 `analysis/tas-main-f22895/emu/seg-020000-030000.jsonl` 的 `w.ens`、
> rust 侧 `analysis/tas-main-f22895/rust/full.state.jsonl` 的 `w.iram[0x300..0x400]`，
> 由 `tools/tas_replay_diff.py` 的 `emu_rows_abs` / `rust_frames` 同口径切片得到。

| 地址 | 字段（`refs/m1disasm/out/M1_NES_NTSC.sym`） | emu | rust | 差值 |
|---|---|---|---|---|
| `$0400` | `Ens.0.y` | `0x98` | `0x97` | −1 |
| `$0401` | `Ens.0.x` | `0xCE` | `0xCD` | −1 |
| `$0405` | `Ens.0.data05` | `0x5A` | `0x1A` | bit6（`0x40`）反相 |
| `$040A` | `Ens.0.data0A` | `0x02` | `0x03` | +1（枚举 0-3） |
| `$0415` | `Ens.1.data05` | `0x1E` | `0x5E` | bit6 反相 |
| `$0425` | `Ens.2.data05` | `0x56` | `0x16` | bit6 反相 |
| `$0435` | `Ens.3.data05` | `0x00` | `0x40` | bit6 反相 |
| `$0445` | `Ens.4.data05` | `0x58` | `0x1A` | bit6 反相（低 5 位也不同） |
| `$0449` | `Ens.4.delay` | `0x01` | `0x00` | −1 |
| `$044A` | `Ens.4.data0A` | `0x01` | `0x00` | −1 |
| `$0455` | `Ens.5.data05` | `0x18` | `0x5A` | bit6 反相（低 5 位也不同） |

前后帧（逐字节不同的个数）：f22893=0、f22894=0、**f22895=11**、f22896=5、f22897=4；
f22896 起 `$0441`（槽4.x）持续差 1（f22896 emu `0xBC`/rust `0xBD`，f22897 `0xBC`/`0xBE`）。

## 4 取证 A：整块清零是 ROM 的 `DestroyEnemies`（PC `07:C8C7`）

> 以下命令与产物都在**主树** `/home/peng/workspace/metroid-decomp`（探针产物落主树 `analysis/probe-22460/`，**不在 lane**；lane 里没有这个目录，照抄会找不到文件）。

- 探针（analysis 产物，不入库）：`analysis/probe-22460/scenario.txt`，`frames 22470`、`sample_point spin-exit`、
  `watch_range 0400-04FF ens`、`watch_range 6AF4-6B53 extra`、`ramframe on`、`capture 22450 pre`、
  `capture 22462 post`、`ctwrite 6af4,6b04,6b14,6b24,6b34,6b44`。
- 命令：`GUNNAC_TRACES=$PWD/analysis/probe-22460/traces GUNNAC_SHOTS=$PWD/analysis/probe-22460/shots GUNNAC_MESEN=$PWD/tools/mesen2/Mesen python3 tools/scenario_run.py run analysis/probe-22460/scenario.txt --timeout 900`
  → `DONE: {"frames": 22470, "trace_lines": 22426, "captures": 2}`
- 写回调（`analysis/probe-22460/traces/probe-22460.ctwrite.log`，**A 级**）：

```
[ctwrite] f=22483 addr=6AF4 val=00 pc=C8C7
[ctwrite] f=22483 addr=6B04 val=00 pc=C8C7
[ctwrite] f=22483 addr=6B14 val=00 pc=C8C7
[ctwrite] f=22483 addr=6B24 val=00 pc=C8C7
[ctwrite] f=22483 addr=6B34 val=00 pc=C8C7
[ctwrite] f=22483 addr=6B44 val=00 pc=C8C7
[ctwrite] f=22484 addr=6B44 val=00 pc=ECBE   ; DeleteOffscreenRoomSprites 只清 status
[ctwrite] f=22485 addr=6B44 val=01 pc=EB63   ; GetEnemyData/load_enemy 装载槽4/5
```

`f=` 是 Mesen 的 `frameCount`（含上电/装载预热），本次运行比影片帧号大 23 ⇒ 上述 `22483` 对应影片帧
**f22460**；同一份 `frames.jsonl` 的影片帧行（`f`=影片帧）在 f22460 已见槽0 status=0、槽4/5 status=1，两边吻合。
- `pc=C8C7` 是 `DestroyEnemies ;07:C8BB` 循环体的 `sta EnsExtra.0.status,x`
  （`refs/m1disasm/SRC/prg7_engine.asm:1948` 起，逐行：`lda #$00 / tax / @loop: ... / sta EnsExtra.0.status,x / inx / bne @loop`）。
  全仓写该地址的只有这一个函数（`grep -n "EnsExtra.0.status" refs/m1disasm/SRC/*.asm`）。
- `DestroyEnemies` 的两个调用点：`prg7_engine.asm:1874`（`MoreInit ;07:C81D`，即 `AreaInit ;07:C801` 之后的开局/死亡重开路径）
  与 `:5040`（`ElevatorD8BF` 换区）。本帧 `IN_AREA($74)` 全片恒 0（未换区）⇒ 不可能是电梯路径；
  且同一重开序列里 `SamusInit ;07:C8D1` 把 `Samus.status ← _id_Palette13+1 = 0x14`
  （`prg7_engine.asm:1971` 起），rust 侧 f22463 起 `Samus.status=0x14` 与之吻合 ⇒ 确证是**死亡重开**走 `AreaInit→MoreInit`。

## 5 取证 B：rust 侧漏调用，补调后 A-1 消失（实验，**未提交**）

- 事实（A 级，代码直读）：`crates/game-core/src/world/room_load.rs:16 pub fn more_init` 的注释逐项列了
  `ScreenOff/ClearNameTables（已折）/EraseAllSprites（已折）`，紧接着直接是 `clear_scroll_blocks`；
  **ROM 在 `EraseAllSprites` 与 `ScrollBlock` 双清之间还有一条 `jsr DestroyEnemies`（`:1874`），rust 没有对应调用**。
  rust 已有等价函数：`crates/game-core/src/world/elevator.rs:416 pub fn destroy_enemies`（清 `$97..$DE` + `EnsExtra` 256B +
  `MetroidOnSamus←0` + `GotoClearAllMetroidLatches`），当前只被 `elevator.rs:320`（换区电梯）调用。
- 实验（在 `/home/peng/workspace/wt-roll` 工作树内临时改、跑完还原，`git status` 干净）：
  在 `:39 erase_all_sprites` 后插入 `crate::world::elevator::destroy_enemies(mem);`，重编译后
  用同一条 `rust/full.sched`（`python3 tools/movie_ref/movie_sched.py --inputlog movies/lordtom-metroid-100p.inputlog --frames 30000 --format sched --name tas-rust-full-30000 --out /tmp/r68/full.sched`）
  跑 rust 腿 30000 帧，与**同一份** emu seg2 逐帧比：
  - f22895：11 字节差 → **8 字节**（`$0400/$0401/$040A` 全部归零，槽0 不再移动）；
  - f22896：5 → 3；f22897：4 → **1**（只剩 `$0441`）；
  - 结论：**A-1 的因果链闭合**（漏调用 ⇒ 槽0 幽灵 ⇒ y/x/data0A）。
  - 但整段 diverge 帧数未变（7086/9970）⇒ A-2 是独立根因（见 §6）。
- 交接补丁：`/tmp/r68/room_load-experiment.patch`（3 行插入，可直接 `git apply`）。

## 6 两组差异的归因（按 PMO seq5-③「同一位分组」）

1. **`$0400/$0401`（y/x 各 −1）+ `$040A`（data0A 2→3）：A-1 幽灵 Crawler 的一步**（A 级，§5 实验）。
   rust 侧 `crates/game-core/src/enemy/crawler.rs:150-170`：主移 `idx=(orientation*2+dir)&7`，`orientation=2`、`dir=0` ⇒ `idx=4`=left（x−1）；
   随后外角探径 `next_o=(2+1)&3=3`、`nidx=(3*2+0)=6`=up 成功 ⇒ `data0A←3` + `crawler_reorient_sprite`
   （bank1 表 idx6=`0x38`）⇒ x−1、y−1、data0A 2→3 与观测逐项吻合。emu 侧该槽 f22460 已 status=0，
   f22888–22898 `$0400-$0402` 恒 `98 CE 00`（没动）。
2. **`$0405/$0415/$0425/$0435/$0445/$0455`（六槽 data05 bit6）：A-2 相位**（A 级机制锚 + C 级归因）。
   bit6 由 `UpdateEnemy_UpdateEnData05Bit6 ;07:F3AA` 写成 `(slotIndex xor FrameCount)&1`，
   **每槽每帧翻转**；`UpdateAllEnemies ;07:F345` 对 status=0 的槽也照样调用。
   RAM 读数：emu 的槽0 `$0405` 从 f22897 起 `0x5A→0x1A→0x5A…` 翻转，rust（补调后）从 **f22895** 起翻转
   ⇒ rust 的 GameEngine 比 emu **早 2 帧**恢复。反向旁证：f22462 时 rust 还**落后** 1 帧
   （emu `Timer3($2C)=0x2C`、`Samus.status=0x14`；rust 两者皆 0，f22463 才到 0x14）⇒ 整个重开序列 rust 净短 ≈3 帧。
   机制（哪一段帧等待记账少了）**未定位**，C 级假设：`more_init` 的 `iter_wait`/`iter_bogged(2)`/`deferred_screen_on`
   或 `SamusIntro` 的 440 帧淡入记账。此段属 world/ 或 frame/，不在 enemy/ 写面。
3. **`$0449/$044A`（槽4 delay/data0A）与 `$0441`（槽4.x）：A-2 的连带**。
   槽4/5 是新房间载入的敌人（`load_enemy`，见 §4 的 `pc=EB63` 写 status=1），rust 早 2 帧进入 GameEngine
   ⇒ 槽4 多跑了更新，x 差 1 后一直保持（f22896 起唯一的 1 字节差）。

## 7 修法建议（给 follow-up 卡，不属本卡写面）

1. **A-1（world/，一行调用）**：在 `crates/game-core/src/world/room_load.rs:16 more_init` 内，
   `crate::frame::boot::erase_all_sprites(mem);`（`:39`）之后、`clear_scroll_blocks(mem)` 之前，
   按 ROM 顺序调用 `crate::world::elevator::destroy_enemies(mem);`（`:416`，已 A 级等价）。
   建议同时补一条单测：把 `$6AF4` 槽0 status 置 2、`$0405` 置非零，跑 `more_init` 后断言 status=0 且
   `$0400-$0402` 未被触碰（对应 ROM 只清 `$97-$DE`+`EnsExtra` 的边界）。实验补丁见 `/tmp/r68/room_load-experiment.patch`。
2. **A-2（world/ 或 frame/，需另做）**：以「bit6 首次翻转帧」为观测量（emu f22897 / rust f22895）
   逐步二分 `more_init`、`SamusInit`、`SamusIntro` 的帧等待，把 3 帧净差找出来；验收物=同一段
   `--frames 30000 --segments 10000` 的 seg2 首分歧后移或 null。

## 8 未覆盖 / 未验证（不写成通过）

- **没有改 `crates/`**：A-1 的修法只在实验里验证过，未入库；A-2 机制未定位，未修。
- **没跑整片 120540**：卡面与 PMO seq4 都要求只跑覆盖 f22895 的分段，整片留给修完后的验收轮。
- 实验比对的 rust 腿是**全片连续跑 30000 帧**、emu 腿是 seg2 断点续跑（与工具正式口径一致，但
  f22451–22459 等 43 帧 emu 缺采），所以 §5 的「11→8」只统计两边共同帧。
- A-2 的「早 2 帧」来自 bit6 相位这一个观测量（RAM 直读，A 级），**未**用 Samus/画面面交叉验证；
  机制归属 world/ 还是 frame/ 属 C 级假设。
- `docs/findings/tas-100-round3.md` 未改（#66 已验收）；本卡读数与它的差异处（那里的首分歧字节数 11 与本文件一致）。

## 9 复现命令（本文件所有数字）

```bash
# 1) 主线口径 30000 帧分段（§2；seg2 约 3 分钟 emu + 70s rust）
cd /home/peng/workspace/metroid-decomp
CARGO_TARGET_DIR=/home/peng/workspace/wt-roll-target GUNNAC_MESEN=$PWD/tools/mesen2/Mesen \
  python3 tools/tas_replay_diff.py replay --inputlog movies/lordtom-metroid-100p.inputlog \
  --frames 30000 --segments 10000 --allow-rust-missing --out analysis/tas-main-f22895 --timeout 1800

# 2) 11 字节表（§3）
python3 - <<'PY'
import sys
from pathlib import Path
sys.path.insert(0, 'tools')
import tas_replay_diff as T
# emu_rows_abs: 段内绝对帧；rust_frames: lo/hi 是判等面地址，返回 {'range': bytes}
emu  = T.emu_rows_abs(Path('analysis/tas-main-f22895/emu/seg-020000-030000.jsonl'), 'ens', 'Joy1Status')
rust = T.rust_frames(Path('analysis/tas-main-f22895/rust/full.state.jsonl'), 0x400, 0x4FF, 30000)
for f in (22895, 22896):
    e, r = emu[f]['range'], rust[f]['range']
    print(f, [(hex(0x400+i), e[i], r[i]) for i in range(0x100) if e[i] != r[i]])
PY

# 3) 写回调探针（§4，约 7 分钟）
cd /home/peng/workspace/metroid-decomp
GUNNAC_TRACES=$PWD/analysis/probe-22460/traces GUNNAC_SHOTS=$PWD/analysis/probe-22460/shots \
GUNNAC_MESEN=$PWD/tools/mesen2/Mesen python3 tools/scenario_run.py run \
  analysis/probe-22460/scenario.txt --timeout 900
head analysis/probe-22460/traces/probe-22460.ctwrite.log

# 4) 实验补调（§5；跑完必须还原工作树）
cd /home/peng/workspace/wt-roll && git apply /tmp/r68/room_load-experiment.patch
python3 tools/movie_ref/movie_sched.py --inputlog movies/lordtom-metroid-100p.inputlog \
  --frames 30000 --format sched --name tas-rust-full-30000 --out /tmp/r68/full.sched
CARGO_TARGET_DIR=/home/peng/workspace/wt-roll-target cargo run -q -p game-core --bin rust_screenshot -- \
  --schedule /tmp/r68/full.sched --out /tmp/r68/shots --state-out /tmp/r68/full.state.jsonl
# 比对见 §3 的脚本（把 rust 侧换成 /tmp/r68/full.state.jsonl）
# 还原：用动手前的副本覆盖，不要用 `git checkout --`（会按 index 整文件回写，连别人/自己未提交的改动一起盖掉）
cp /tmp/room_load.rs.orig crates/game-core/src/world/room_load.rs
```

---

## 10 独立复核后的更正（2026-11-25，对影 seq10）

对影按 §9 逐条复跑后按定界口径接收，数字全部对上（f22895=11/f22896=5/f22897=4、实验后 8/3/1、ctwrite `pc=C8C7`、
lane tests 1091/0、check PASS24/FAIL1/SKIP3、run-all 171/1/0）。据此更正本文件三处不影响结论的口径：

1. §1 的「复现树」行：基线是 `3e453ccc`，交付 commit 是 `bda1fcda`（内容）+ `d1c237e8`（§9 脚本修正）；
   提交记录里写的「commit eb077ddc / 199 行」是 amend 前的旧值，**以本文件当前 200/203 行与 `d1c237e8` 为准**。
2. §4 的探针命令与产物在主树 `/home/peng/workspace/metroid-decomp`（`analysis/probe-22460/`），不在 lane。
3. `wc -l` 实际 200（含末尾空行），提交说明写的 199 是少算空行；`$0445/$0455` 除 bit6 外低 5 位也不同（§3 表已注）。

未达成项不变（f22895 未全相等、整片 120540 未跑）——按卡面 ③ / PMO seq5-③ 定界停手，修法留给 follow-up 卡（§7）。
