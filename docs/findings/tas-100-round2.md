# TAS 100% 二轮整片回放（`6a48c05d`）：覆盖 120540/120540，首分歧后移到 f2084

> 证据分级见 `AGENTS.md` §3。本卡只**定界**不改码：`crates/` 零改动，产出=`tools/` 侧产物 + 本文。
> 上一轮（f1787，Skree 落地 `Ens.delay`）见 [tas-divergence-f1787.md](tas-divergence-f1787.md)。

## 0. 结论

1. 在 `6a48c05d`（含 #49 的 skree 修复）上整片 120540 帧跑完，**覆盖率 100%**（13/13 段、`failed=0`），总耗时 2158.5s ≈ 36 分钟。
2. 首分歧从 f1787 后移，**新的真实首分歧 = f2084 `$0415`（槽 1 的 `Ens.data05` bit2）**：emu `0x1E` / rust `0x1A`，差 1 字节。
3. 定界（A 级）：该帧槽 1 走 Crawler「内角」重定向路径；ROM 那条路**只写 EnsExtra 动画三元组、不写 data05**，而 rust 在这条路上多写了一句 `data05 &= 0xF9`，把 bit2/bit1 清掉——bit1 随后被绘径可见旗重新置回，只剩 bit2 成为可见差异，与「只差 1 字节」自洽。

## 1. 复现（一行）

```bash
python3 tools/tas_replay_diff.py replay \
  --inputlog movies/lordtom-metroid-100p.inputlog \
  --frames 120540 --segments 10000 --out analysis/tas-100
```

- 起点：`git HEAD = 6a48c05d`（`feat(core,tools): 批量落地 #41/#43/#47/#49 代码与工具`）。
- 产物：`analysis/tas-100/{coverage.json,coverage.md}`、`analysis/tas-100/emu/seg-*-*.jsonl`（emu 逐帧 `$0400-$04FF`）、`analysis/tas-100/rust/full.state.jsonl`（rust 逐帧 zp/iram/wram）、`analysis/tas-100/checkpoints/cp-*.{mss,json}`。
- 上一轮产物保留在 `analysis/tas-100-round1/`（首分歧 f1787），**两个目录不要混看**。

## 2. 覆盖表（`analysis/tas-100/coverage.json`）

`ran_frames=120540`、`total_frames=120540`、`coverage_pct=100.0`、`all_pass=false`、`deterministic=true`、`segment_size=10000`。

| 段 | 区间 | status | compared | diverge | 段内首个分歧帧 | 耗时 s | sha256_agg(emu / rust) 前 16 |
|---|---|---|---|---|---|---|---|
| 0 | 0–10000 | diverge | 9962 | 7904 | **2084** | 178.5 | `a285c127de02c5fb` / `ff4f9e4c73b7bac5` |
| 1 | 10000–20000 | diverge | 10001 | 10001 | 10000（段首=承接） | 177.9 | `5f459d1ed2dc38b9` / `27560ba9b8543d5c` |
| 2 | 20000–30000 | diverge | 9970 | 9970 | 20000 | 178.4 | `0e20a8de679d45c4` / `3e1a9556103335af` |
| 3 | 30000–40000 | diverge | 10001 | 10001 | 30000 | 177.9 | `ce5a88df80e03a3a` / `be0260a4b03c0984` |
| 4 | 40000–50000 | diverge | 9964 | 9964 | 40000 | 178.4 | `613ea24dab6cdb75` / `7a2874636d470b27` |
| 5 | 50000–60000 | diverge | 10001 | 10001 | 50000 | 177.9 | `b9c7615043a79fe0` / `43d83be63093c672` |
| 6 | 60000–70000 | diverge | 9949 | 9949 | 60000 | 178.4 | `e5adf0848a3641d4` / `75d6b57b70350e29` |
| 7 | 70000–80000 | diverge | 9986 | 9986 | 70000 | 177.9 | `29f79fb819005595` / `c963e694c0e5b5a0` |
| 8 | 80000–90000 | diverge | 9979 | 9979 | 80000 | 177.9 | `6ae6eee90a8dc868` / `eebbef1fdbb97166` |
| 9 | 90000–100000 | diverge | 9998 | 9998 | 90000 | 177.9 | `b9990734331c116e` / `eda9fcbc3b5bb2bb` |
| 10 | 100000–110000 | diverge | 9946 | 9946 | 100000 | 178.4 | `65b457c8da329f4f` / `48c05b7c60d9a8fa` |
| 11 | 110000–120000 | diverge | 9965 | 9965 | 110000 | 178.4 | `6e74635fedfae6ba` / `75c8839299b6c44b` |
| 12 | 120000–120540 | diverge | 541 | 541 | 120000 | 20.6 | `8444edf10a108c38` / `49f830d86ee1c96e` |

- **全局首分歧只有一个：f2084**。段 1 起的「段内首分歧=段起点」是**承接**（分歧不可自愈），不是 12 个新缺陷——报告里已按 `post_divergence` 标注。
- 合计 120263 个可比帧里 118205 帧不同（98.29%）——这正是「一个早期分歧扩散到后续全部帧」的形状；**不要**把它读成 11 万个独立缺陷。

## 3. 新首分歧三元组

```
{ frame: 2084, addr: $0415, slot: 1, emu: 0x1E, rust: 0x1A }
```

字段归属：`$0400 + 1*16 + 5` = 槽 1 的 `Ens.data05`（`crates/game-core/src/enemy/state.rs`：`ens::DATA05 = 5`）。
`diff_bytes = 1`，只有该字节不同（同帧 `$042D`(data0D)=`01`、`$0082`=`00` 两侧相同）。

## 4. 定界

### 4.1 触发路径（A 级，两侧同帧一致）

f2083→f2084，槽 1 的 `Ens.data0A` 由 `00` 变 `03`（`dey / and #$03` = 内角取向 −1），两侧完全同步 ⇒ 该帧走的是

```
CrawlerInsideCornerCheck → (内角) → jmp CrawlerReorientSprite
```

（`refs/m1disasm/SRC/enemies/crawler.asm:135-149`）

### 4.2 ROM 在这条路上写什么（A 级）

- `CrawlerReorientSprite_{AREA} ; 01:9AA0`：`lda data05 / lsr / lda data0A / and #$03 / rol / tay / lda CrawlerAnimIndexTable,y / jmp CommonJump_InitEnAnimIndex`
  （`refs/m1disasm/SRC/enemies/crawler.asm:83-92`）
- `CommonJump_InitEnAnimIndex` → `InitEnAnimIndex ; 07:F68D` / `SetEnAnimIndex ; 07:F690` / `ClearEnAnimDelay ; 07:F693`：
  只写 `EnsExtra.resetAnimIndex(+5)`、`animIndex(+6)`、`animDelay(+4)`（`refs/m1disasm/SRC/prg7_engine.asm:11290-11296`）。
- ⇒ **这条路径上没有任何 `Ens.data05` 写**。

### 4.3 rust 在这条路上多写了什么（B 级，代码直读）

`crates/game-core/src/enemy/crawler.rs:194-195`：

```rust
    let d5c = mem.read_ram(0x0400 + x as u16 + 5);
    mem.write_ram(0x0400 + x as u16 + 5, d5c & 0xF9); // bit2/1 清（0x06）
```

- `0x1E & 0xF9 = 0x18`（bit1、bit2 都清），随后绘径 `update_enemy_common_no_move_no_anim`
  的可见旗写件把 bit1 重新置回（`(d5 & 0xFD) | 0x02`，`crates/game-core/src/enemy/framework.rs:698-703`）⇒ `0x1A`。
- 与实测完全吻合：emu 保持 `0x1E`，rust 落 `0x1A`，**唯一可见差异就是 bit2**。
- 该行注释自称依据是「金腿实证 f1838 转角径 d05 0x1A→0x58（bit2/1 清 + bit6 置）」——但 f1838 的 `0x58` 里 **bit6 是置的**，而本路径 ROM 不写 bit6；说明当年那次读数混进了别的写面，把「某次观测」当成了「这条路径的行为」。同一断言也被写进了单测（`crates/game-core/src/enemy/crawler.rs:292`：`assert_eq!(got, d5 & 0xF9)`），需要一起改。

### 4.4 排除项

| 候选 | 依据 |
|---|---|
| 速向力 `enemy_force_speed`（07:F6B9，会清/置 bit2） | 该帧帧首 `data05=0x5E`（bit6 置）⇒ `enemy_active` 直跳 BranchB，ForceSpeed 三件不执行；`$042D`(data0D) 两侧都是 `01` 未变 |
| 阈距件 `UpdateEnemy_EnData05DistanceToSamusThreshold`（07:F75B） | 只掩写 bit3/bit4（`or #$18` / `and #~$08` / `#~$10`），不碰 bit2；且该帧两侧 bit3/bit4 相同 |
| 移动失败翻向 `EnemyIfMoveFailedUp/Down → @movementStrings`（`areas_common.asm:239-248`，`XorEnData05 #$04`） | 该帧槽 1 没有移动失败（位置 `$0410/$0411` 两侧相同）；且 rust 侧同一逻辑 `motion.rs:139-190` 已实现 |
| 采集相位/分段伪差 | 与 10000 帧单段、2400 帧短段两轮独立产物给出同一首分歧 f2084 |

## 5. 证据完整性（本轮特有）

跑整片时工作树里带着他人未提交的 `crates/game-core/src/enemy/framework.rs`（typed 面重构，+133 行）、`crates/game-core/src/memmap/{mod,nesmap}.rs`、`spare_face.rs`；
`coverage.json` 的 `env` 已如实记录：

```json
{"git_head":"6a48c05d7b6d","git_dirty_crates":["M crates/game-core/src/enemy/framework.rs",
 " M crates/game-core/src/memmap/mod.rs"," M crates/game-core/src/memmap/nesmap.rs",
 "?? crates/game-core/src/memmap/spare_face.rs"],"rust_screenshot_sha256":"25a299d98d55a4a3"}
```

为排除「分歧是这些 WIP 造成的」，用 `git archive 6a48c05d` 建干净副本 `/tmp/v50` 编 `rust_screenshot`，用**同一张** `analysis/tas-100/rust/full.sched` 重跑 rust 腿 120540 帧，与主树结果逐帧比 `{zp,iram,wram}`：

```
帧数 120349　状态面不一致帧 0　首差异帧 None
```

⇒ 工作树 WIP 对本次结果**零影响**，本报告的读数等价于干净 `6a48c05d`。

## 6. 修复建议（本卡未做，留给修复卡）

1. 删掉 `crates/game-core/src/enemy/crawler.rs:194-195` 那句 `d5c & 0xF9` 写（ROM 该路径无 data05 写），同步改 `crawler.rs:292` 那条跟着它的断言。
2. 重查 f1838 的 `0x58` 读数来源（很可能是另一条路径——例如移动失败翻向或速向力——被误记到重定向路径上）；改完后必须复跑整片确认 f2084 归零、且后续段无新增差异。
3. `data05` bit2 语义（垂直朝向）在别处仍需正确设置：`prep.rs::enemy_force_speed` 的 y 轴力路径（`07:F70F`–`07:F731`）是 ROM 的正当写面，不要一并删。

## 7. 未覆盖

- 本卡只定界首分歧，**未修**；f2084 之后的扩散面（位置/其它槽）没有逐一归类。
- 整片第二处独立分歧（若有）被首分歧的扩散掩盖，需在修复后重跑才能分离。
