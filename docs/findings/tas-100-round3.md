# TAS 100% 整片第三轮重跑：f2151 收口后的下一处首分歧（f22895 `$0400`）

> 日期：2026-11-25
> 卡片：#66（`tsk_01M894WFDPA9Y9J1CXHJMA`）
> 结论级别：首分歧帧与字节级差异为 **A 级**（可直接用本文命令复算）；根因方向为 **C 级假设**（卡面要求「只登记不修」，本轮不动代码）。

## 0. 结论先行

1. **f2151 已彻底收口**：整片 0–22894 帧在 `$0400-$04FF` 观察面上逐字节一致（f2151、f2084 都在其中），不再是首分歧。
2. **下一处真实首分歧：f22895，字段 `$0400`（槽0 `Ens.0.y`），emu=`0x98`、rust=`0x97`**，同帧共 **11 字节**不同，且此后不再自愈（11/13 段 diverge）。
3. 整片覆盖：**120540/120540 帧，13/13 段执行完，失败段 0**；`all_pass=false`，`coverage_pct=100.0`，`deterministic=true`。
4. **根因未定**：卡面边界「有新分歧只登记不修」，见 §5 的 C 级假设与 §6 建议的下一张卡。

## 1. 环境指纹（本报告所有数字的出处）

| 项 | 值 |
|---|---|
| 工作树 | `/home/peng/workspace/wt-roll`，分支 `lane/roll` |
| HEAD | `e4d23d6f4d838fe78dfc935dbd29852183629be0`（`e4d23d6f4d83`），含 f2151 修复 `94d866bb` |
| 工作树状态 | 跑前 `git status --porcelain` 空；`coverage.json.env.git_dirty_crates = []` |
| emu inputlog | `movies/lordtom-metroid-100p.inputlog`，sha256 `094de4afdc1eff604002bbb144d40f36918833b3c211ed67f0f0b75c231b68b8`（前 24 位 `094de4afdc1eff604002bbb1`） |
| rust CLI | `/home/peng/workspace/wt-roll-target/debug/rust_screenshot`，mtime 2026-11-25 18:28，sha256 `4e9eb066b11501caa131985bdd0fe65ed5efa02af76ab6c722c81631bc8feb1b`（前 24 位 `4e9eb066b11501caa131985b`） |
| 构建目录 | `CARGO_TARGET_DIR=/home/peng/workspace/wt-roll-target` |
| Mesen | `/home/peng/workspace/metroid-decomp/tools/mesen2/Mesen`（跑前确认无并发采集） |
| 观察面 | 每帧 `$0400-$04FF`（`--tag ens`，256 B），符号取 `refs/m1disasm/out/M1_NES_NTSC.sym` |

## 2. 跑法与耗时

```bash
cd /home/peng/workspace/wt-roll
CARGO_TARGET_DIR=/home/peng/workspace/wt-roll-target \
GUNNAC_MESEN=/home/peng/workspace/metroid-decomp/tools/mesen2/Mesen \
python3 -u tools/tas_replay_diff.py replay \
  --inputlog movies/lordtom-metroid-100p.inputlog \
  --frames 120540 --segments 10000 --allow-rust-missing \
  --out analysis/tas-100-round3 --timeout 1800
```

- **rust 腿**：全片连续跑 120540 帧（不分段，工具设计要求；分段续跑会与连续跑分叉，见 `docs/design/tas-replay-pipeline.md:188` 的已知限制），耗时 **70.0 s**。
- **emu 腿**：13 段 × ≤10000 帧，段间走 `checkpoints/cp-<帧号>.mss` 续跑；各段耗时见表 §3。
- **总耗时**：分段墙钟合计 **2160.1 s** + rust 腿 **70.0 s** = **2230.1 s ≈ 37.2 分钟**（另有 1200 帧单段校验 31.7 s，见 §4）。

## 3. 逐段结果（`analysis/tas-100-round3/coverage.json`）

| 段 | 帧窗 | 状态 | 墙钟(s) | 可比较帧 | 分歧帧 | emu 聚合 sha（前16） |
|---|---|---|---|---|---|---|
| 0 | [0,10000) | pass | 178.5 | 9962 | 0 | `a285c127de02c5fb` |
| 1 | [10000,20000) | pass | 177.9 | 10001 | 0 | `5f459d1ed2dc38b9` |
| 2 | [20000,30000) | **diverge** | 178.4 | 9970 | 7086 | `0e20a8de679d45c4` |
| 3 | [30000,40000) | diverge | 178.0 | 10001 | 10001 | `ce5a88df80e03a3a` |
| 4 | [40000,50000) | diverge | 178.8 | 9964 | 9964 | `613ea24dab6cdb75` |
| 5 | [50000,60000) | diverge | 178.5 | 10001 | 10001 | `b9c7615043a79fe0` |
| 6 | [60000,70000) | diverge | 178.5 | 9949 | 9949 | `2dfae3a8521033ef` |
| 7 | [70000,80000) | diverge | 178.0 | 9986 | 9986 | `29f79fb819005595` |
| 8 | [80000,90000) | diverge | 177.9 | 9979 | 9979 | `6ae6eee90a8dc868` |
| 9 | [90000,100000) | diverge | 178.0 | 9998 | 9998 | `b9990734331c116e` |
| 10 | [100000,110000) | diverge | 178.5 | 9943 | 9943 | `ec127583e6b099e1` |
| 11 | [110000,120000) | diverge | 178.5 | 9964 | 9964 | `c0bf9b4d7b3b3681` |
| 12 | [120000,120540) | diverge | 20.6 | 541 | 541 | `8444edf10a108c38` |

分段 2 之后「段段分歧」是工具的既定口径：分歧不会自愈，后续段按合法结论记 `diverge`（`tools/tas_replay_diff.py:730-740` 的注释与退出码设计）。可比较帧少于 10000 的段是 rust 腿在载房窗/未 tick 帧只记「不可比」（`--allow-rust-missing` 口径，命中数与该轮载房窗次数一致）。

## 4. 单段 sha 对照（验收项「1200 帧单段 sha」）

```bash
python3 -u tools/tas_replay_diff.py replay \
  --inputlog movies/lordtom-metroid-100p.inputlog \
  --frames 1200 --segments 1200 --allow-rust-missing \
  --out analysis/tas-1200-check
```

结果：段 0 `[0,1200)` **pass**，31.7 s；双方聚合 sha 完全相同：

- `sha256_agg_emu = sha256_agg_rust = 1bb9ddea68a4514016fe9d9b548f25d62206295ce80ad052d37d4a6967a1bef4`

与 `docs/design/tas-replay-pipeline.md:188` 记录的参考值 `1bb9ddea68a45140…`（以及历史 `analysis/tas45-seg1200/coverage.json`）**逐字节相同** ⇒ emu 腿在本机当前配置下可复现。

## 5. f22895 的 11 个差异字节（A 级）

槽布局按 `crates/game-core/src/enemy/state.rs:18-40`（y,x,speedY,speedX,isHit,data05,data06,speedSubX,movementIndex,delay,data0A,health,prevStatus,data0D,weaponAction,specialAttribs）。

| 地址 | 字段 | emu | rust | 差 |
|---|---|---|---|---|
| `$0400` | `Ens.0.y` | `0x98` | `0x97` | −1 |
| `$0401` | `Ens.0.x` | `0xCE` | `0xCD` | −1 |
| `$0405` | `Ens.0.data05` | `0x5A` | `0x1A` | bit6 差 |
| `$040A` | `Ens.0.data0A`（crawler 朝向，`crawler.rs:91`） | `0x02` | `0x03` | +1 |
| `$0415` | `Ens.1.data05` | `0x1E` | `0x5E` | bit6 差 |
| `$0425` | `Ens.2.data05` | `0x56` | `0x16` | bit6 差 |
| `$0435` | `Ens.3.data05` | `0x00` | `0x40` | bit6 差 |
| `$0445` | `Ens.4.data05` | `0x58` | `0x1A` | bit6 差 |
| `$0449` | `Ens.4.delay` | `0x01` | `0x00` | −1 |
| `$044A` | `Ens.4.data0A` | `0x01` | `0x00` | −1 |
| `$0455` | `Ens.5.data05` | `0x18` | `0x5A` | bit6 差 |

前后帧（同窗口差异字节数）：f22893=0、f22894=0、**f22895=11**、f22896=5、f22897=4 ⇒ 分歧从 f22895 开始且持续演化。

**关键否证**：f22895 时 `$0406`（`chunkyExplosionTimer`）两边**相同**，所以这一帧**不是**「爆炸计时晚一帧」那条已登记语义偏差触发的（后者仍是独立待办，见 §7）。

## 6. 与上一轮的对比

| 轮次 | 首分歧 | 备注 |
|---|---|---|
| round2（`analysis/tas-100/coverage.round2.json.bak`） | f2084 `$0415` emu 30 / rust 26 | 100% 录像旧码 |
| #60 后切片 | f2151 收口；2200 帧 0 分歧、10000 帧 0 分歧 | `docs/findings/tas-divergence-f2151.md` |
| **round3（本报告）** | **f22895 `$0400` emu 0x98 / rust 0x97** | 首分歧后移 ~20744 帧 |

⇒ 从 f2151 到 f22895 之间（含 f2151 本身）**1–22894 帧逐字节一致**，f2151 的修复在整片上成立。

## 7. C 级假设（只登记，不修）

1. **C 级（主）**：槽0 的 `data0A`（crawler 朝向）emu 2 / rust 3、同时 `y`/`x` 各差 1 ⇒ 最像是「槽0 这只爬行敌在某次转角/移动分支上走了另一条路」，位移差再向下游传播。对应实现 `crates/game-core/src/enemy/crawler.rs:150-170`（`crawler_outside_corner_check` 的探径成功/失败分支、`data0A = next_o` 与 `reorient`）。
2. **C 级（次）**：槽1–5 的 `data05` bit6(0x40) 差在方向与幅度上互不一致（有的 emu 置位、有的 rust 置位），更像 f22895 之后绘制/动画顺序变化的**瞬态**而非独立根因；`framework.rs:712` 每帧写 `data05` 可见位的邻近位可作线索。
3. **独立待办（不属本轮首分歧）**：`docs/findings/tas-divergence-f2151.md` 与 thread seq19 记录的一帧差——ROM `prg7_engine.asm:6141-6158` 是「先 `inc` 再 `cmp #$19`，本帧到 `$19` 即死」，当前 Rust 为 `timer >= 0x19 → 死，否则 +1`，从 `0x18` 起恒晚一帧。本轮 f22895 时 `$0406` 相同，尚无证据链表明它触发本帧，但整片后续仍可能撞上，建议单独开卡修（1 行 + 2 单测）。

## 8. 复现与校验命令（全部实跑）

```bash
# 1) 读本轮覆盖率与首分歧
python3 -c "import json;d=json.load(open('analysis/tas-100/coverage.json'));print(d['ran_frames'],d['total_frames'],d['segments'] and len(d['segments']),d['first_divergence'])"
# → 120540 120540 13 {'diff_bytes': 11, 'emu': 152, 'field': '$0400', 'frame': 22895, 'offset': 0, 'rust': 151}

# 2) 复算 f22895 的 11 字节（wt-roll，本轮产物在 analysis/tas-100-round3/）
python3 - <<'PY'
import importlib.util
from pathlib import Path
spec=importlib.util.spec_from_file_location("tas","tools/tas_replay_diff.py")
m=importlib.util.module_from_spec(spec); spec.loader.exec_module(m)
emu=m.emu_rows_abs(Path('analysis/tas-100-round3/emu/seg-020000-030000.jsonl'),'ens','Joy1Status')
rust=m.rust_rows_rel(Path('analysis/tas-100-round3/rust/full.state.jsonl'),0x400,0x4ff)
for f in (22894,22895):
    e=emu[f]['range']; r=rust[f]['range']
    print(f,[hex(0x400+i) for i in range(256) if e[i]!=r[i]])
PY

# 3) 单段 sha 复核
python3 -c "import json;d=json.load(open('analysis/tas-1200-check/coverage.json'));e=d['segments'][0];print(e['sha256_agg_emu'])"
# → 1bb9ddea68a4514016fe9d9b548f25d62206295ce80ad052d37d4a6967a1bef4
```

## 9. 未覆盖 / 声明事项

- 本轮是 **lane/roll（含 f2151 修复）** 上的读数，不是主线读数；主线 pick 后如需主线口径，需重跑一次（工具、inputlog、二进制指纹已在 §1 固定，可直接比对此表）。
- 观察面仅 `$0400-$04FF`（与既有 TAS 对拍口径一致），不覆盖 Samus/世界侧 RAM；全 RAM 面差异不属本轮结论。
- f22895 的根因**未取证到 bank:addr**（卡面要求只登记不修）；§7 的假设均标 C 级，不得据此改代码。
- `analysis/` 产物不入库；本轮 coverage 已覆盖到主树 `analysis/tas-100/coverage.json`，旧轮次备份为 `coverage.round2.json.bak` / `coverage.round2.md.bak`。
