# TAS 100% 首分歧 f2084：槽 1 `$0415`(=Ens.data05) 多出 bit2

> 结论：Rust `crawler_reorient_sprite()` 末尾多写了一条 ROM 里不存在的 `data05 &= $F9`。
> 删掉后 f2084 逐字节相等，首分歧后移到 f2151（新缺陷，另记 §6）。
> 任务：#55（writeScope `crates/game-core/src/enemy/` + 本文件）。HEAD 基线 `6a48c05d`。

## 1. 一行复现

```bash
# emu 腿（已落盘，无需开模拟器）：analysis/tas-100/emu/seg-000000-010000.jsonl
# rust 腿：取 0–10000 帧的调度表跑一次即可（约 6 秒）
python3 - <<'PY'
import sys; sys.path[:0]=['tools','tools/movie_ref']
import tas_replay_diff as t; from pathlib import Path
t.build_segment_rust_sched(Path('movies/lordtom-metroid-100p.inputlog'),0,10000,'r',Path('/tmp/x'),None)
PY
./target/debug/rust_screenshot --schedule /tmp/x/r.sched --out /tmp/x/out --state-out /tmp/x/state.jsonl
```

比对（`emu_rows_abs` + `rust_rows_rel` + `compare_window(..., rust_base=0)`，窗口 `$0400-$04FF`）。

## 2. 现象（修前 / 修后）

| | 修前（`analysis/tas-100/coverage.json`） | 修后（本次实测） |
|---|---|---|
| 首个不等帧 | **f2084** | **f2151** |
| 字段 | `$0415`（槽 1 offset 5 = `Ens.data05`） | `$0416`（槽 1 offset 6，见 §6） |
| 值 | emu `0x1E` / rust `0x1A`（XOR `0x04`） | emu `0x00` / rust `0x27` |
| 1–2083 | 全等 | 全等（且 1–2150 全等） |

> 卡面标题写「`$0415` x 低字节」有误：`$0415 = $0400 + slot1*16 + 5`，是 `data05`；x 在 `$0411`。
> 本条按 `coverage.json` 的 `offset: 21` 与历史证据走。

## 3. 根因

**A 级（ROM）**：`CrawlerReorientSprite ; 01:9A8F`

```asm
CrawlerReorientSprite:
    lda Ens.0.data05,x   ; 只**读** bit0 当 flip
    lsr
    lda Ens.0.data0A,x
    and #$03
    rol A                ; idx = orient*2 + flip
    tay
    lda CrawlerAnimIndexTable,y
    jmp InitEnAnimIndex  ; 01:F68D：只写 EnsExtra(+5) resetAnimIndex、(+6) animIndex、(+4) animDelay←0
```

整条路径**没有任何 `data05` 写**。`data05` 的 bit1 是可见旗（由 `DrawEnemy` 写，ROM `07:DDxx`：
`lda data05 / and #$FD / ora Temp08 / sta data05`），bit2 是「竖向朝向」（由
`EnemyIfMoveFailedUp/Down` 翻转，`areas_common.asm:241-245`）。二者都不属于本径。

**B 级（Rust 落点）**：`crates/game-core/src/enemy/crawler.rs::crawler_reorient_sprite` 末尾曾有

```rust
let d5c = mem.read_ram(0x0400 + x as u16 + 5);
mem.write_ram(0x0400 + x as u16 + 5, d5c & 0xF9); // bit2/1 清
```

其注释称「金腿实证 f1838 转角径 d05 0x1A→0x58」。该读法与写面自身矛盾：
`0x1A & 0xF9 = 0x18`（不是 `0x58`），而 `0x58` 需要 bit6 置位——本径不写 bit6。
即那个观测混了别的写面（或别的对象），据此加的写是错的。

后果：f2084 槽 1（type=5 Crawler）`data05` 的 bit2 被清，emu `0x1E` / rust `0x1A`。

## 4. 修法

- 删除该写与错误注释，改为一句「ROM 该径不写 data05，bit1/bit2 另有写主」的说明。
- 测试 `reorient_clears_bit2_and_bit1_keeps_others` → 改为 `reorient_leaves_data05_untouched`：
  四组取值（`0x1E/0x1A/0x5A/0x7F`）断言 `data05` **原样保留**，且 `animDelay←0` 仍发生
  （保证不是「整段没跑」也能过）。
- 改 1 行、删 2 行，未碰其它文件；crawler.rs 无 `data/status/` 行号锚依赖（只有按文件的计数账本，
  计数只减不增，方向合法）。

## 5. 验证（全部本机实跑）

| 项 | 结果 |
|---|---|
| f2084 `$0415` | emu `0x1E` == rust `0x1E` ✓ |
| 1–2150 帧 | 无任何差异（共同帧 9962，首个不等帧 2151） |
| `cargo test -p game-core --lib enemy::crawler` | 5 passed |
| `cargo test -q -p game-core` | 14 targets 合计 1089 passed / 0 failed |
| `python3 gates/cli.py check` | PASS=24 FAIL=0 SKIP=3 TOLERATED=1 / CHECK: PASS |
| `python3 tools/l4_diff_run.py run-all --skip-emu` | 21 用例 · 172 格 · match 171 / diff 1（c674=84 已知残差）/ blocked 0 |

整片首分歧「后移」按 10000 帧切片判：2084 → 2151（>2084）。整片 120540 帧未重跑（emu 腿 36 分钟，
本卡不需要；口径与 `#50` 一致：切片已覆盖该分歧窗口）。

## 6. 下一处分歧 f2151 的登记（不在本卡修）

- 位置：槽 1 `$0416`（`Ens.data06`，`framework.rs:94` 注释为 `speedSubPixelY`）；emu `0x00` / rust `0x27`。
- 槽 1 身份已换：f2084 是 `status=2(Active) type=5(Crawler) animFrame=0x5A`；f2151 是
  `status=5(Pickup) type=0 animFrame=0x81`（小能量），且 rust 的 `EnsExtra.resetAnimIndex/animIndex`
  恰好都是 `0x27` ⇒ 疑为「捡取径把 animIndex 写进了 `Ens.data06`」的串写。
- 归因分级：**B**（同一帧两个字段与 rust 内部值一致，但尚未取到 ROM 写点行号）。
- 与本卡的关系：`crawler_reorient_sprite` 只在 Crawler 的转角径执行；f2151 的槽 1 已不是 Crawler，
  且 1–2150 逐帧全等 ⇒ 本例不是本卡改动引入，属下一处独立缺陷，建议另开卡。

## 7. 未覆盖

1. 整片 120540 帧未重跑（只用 10000 帧切片确认首分歧后移）；下次整片跑会给出新的全局首分歧。
2. `data05` bit2 的写主只做了 ROM 静态定位（`EnemyIfMoveFailedUp/Down`），未做「本帧谁写的」emu 写钩取证。
3. f2151 未定界到 `bank:addr`（仅 B 级旁证）。
