# TAS 100% 首分歧 f2151：槽 1 `$0416`（`Ens.chunkyExplosionTimer`）初值被写成动画索引

> 结论（A 级）：`ExplodeEnemy ; 07:F611` 在 `jsr InitEnAnimIndex`（`; 07:F68D`）之后紧接
> `sta Ens.0.chunkyExplosionTimer,x`，而 `InitEnAnimIndex` 末尾的 `lda #$00` 已把 A 清成 0
> ⇒ 这里存进去的是 **0**，不是传进去的 `EnemyDeathAnimIndex[y]`。Rust 旧形写的是动画索引
> （本例 0x27）⇒ f2151 首分歧。修后 10000 帧窗口首分歧为 **null**。
> 附带修正：该字节在 `DrawEnemy` 里是**从 0 每帧 +1、到 `$19` 完成死亡**的累加器，
> 不是倒计时——旧形「每帧减 1、≤0 完成」方向也反了。
> 任务 #60（writeScope `crates/game-core/src/enemy/` + 本文件；`frame/tests.rs` 一行断言经 PMO
> 授权作为连带修正）。基线 `6a48c05d`，分支 `lane/roll`，worktree `/home/peng/workspace/wt-roll`。

## 1. 复现

```bash
# 修复后（卡面命令 + --out）：
python3 tools/tas_replay_diff.py replay \
  --inputlog movies/lordtom-metroid-100p.inputlog \
  --frames 2200 --allow-rust-missing --chunk 2200 --out analysis/tas-f2151-final
# → [tas] 分歧帧 0/2200（比较 2175 帧）；首分歧 无
# 放宽到 10000 帧（首分歧已越过 f2151）：
python3 tools/tas_replay_diff.py replay \
  --inputlog movies/lordtom-metroid-100p.inputlog \
  --frames 10000 --allow-rust-missing --chunk 10000 --out analysis/tas-f2151-10000
# → [tas] 分歧帧 0/10000（比较 9967 帧）；首分歧 无
```

修复前的对照（同一份 emu trace，只换 rust 腿代码；worktree 内操作，先备份 `framework.rs`）：

```bash
git checkout HEAD -- crates/game-core/src/enemy/framework.rs
cargo run -q -p game-core --bin rust_screenshot -- \
  --schedule analysis/tas-f2151-final/scenario-rust.sched \
  --out /tmp/old-rust-shots --state-out /tmp/old-rust.state.jsonl
```

再用 `tools/tas_replay_diff.py` 的 `emu_frames` / `rust_frames` / `compare(..., 0x400, 2200)`
对 `$0400-$04FF` 判等（判等面 = `Ens` 敌槽区 256 B）。

## 2. 现象（修前 / 修后）

| | 修前（`framework.rs` = HEAD 版，同一份 emu trace） | 修后（本次实测） |
|---|---|---|
| 首个不等帧 | **f2151** | **无**（2200 帧窗口 + 10000 帧窗口都无） |
| 字段 | `$0416`（槽 1 offset 22 = `Ens.data06`） | — |
| 值 | emu `0x00` / rust `0x27`（39） | f2151 两侧都是 `0x00` |
| 分歧帧数 | 从 f2151 起共 **50** 帧（至 2200） | **0** / 比较 2175；10000 帧窗口 **0** / 比较 9967 |
| f2084 `$0415` | emu `0x1E` == rust `0x1E`（#55 已修） | 仍 `0x1E` == `0x1E` |

判等窗口：`$0400-$04FF`（12 个敌槽 × 16 B）。2200 帧里有 25 帧 rust 腿不产状态行
（载房窗/引擎一跳多帧，`--allow-rust-missing` 显式列出，未静默计一致）。

## 3. 根因

**A 级（符号表）**：`refs/m1disasm/out/M1_NES_NTSC.sym`

```
00000406 Ens.0.chunkyExplosionTimer
00000406 Ens.0.movementInstrIndex
00000406 Ens.0.speedSubPixelY
00000407 Ens.0.speedSubPixelX
00000416 Ens.1.chunkyExplosionTimer
```

敌槽基址 `$0400 + i*16`。`$0416` = 槽 1 的 `+6` = `chunkyExplosionTimer`
（同址三名：`movementInstrIndex` / `speedSubPixelY` 也落在这一字节——`crates/game-core/src/enemy/state.rs:22-45`
已按「同址多义不美化」分别暴露访问器；本路径按 `chunkyExplosionTimer` 语义使用）。

**A 级（ROM 汇编）**：`refs/m1disasm/SRC/prg7_engine.asm`

```asm
11201 ExplodeEnemy: ; 07:F611
...
11224     jsr InitEnAnimIndex                 ; ← A = EnemyDeathAnimIndex[y]
11226     sta Ens.0.chunkyExplosionTimer,x    ; ← 存的是 A 的**当前值**
...
11290 InitEnAnimIndex: ; 07:F68D
11291     sta EnsExtra.0.resetAnimIndex,x
11292 SetEnAnimIndex: ; 07:F690
11293     sta EnsExtra.0.animIndex,x
11294 ClearEnAnimDelay: ; 07:F693
11295     lda #$00                            ; ← A 在这里被清成 0
11296     sta EnsExtra.0.animDelay,x
11297 @RTS: ; 07:F698
11298     rts
```

`jsr` 返回后 A=0 ⇒ `sta chunkyExplosionTimer,x` 初始化的是 **0**。
「`lda EnemyDeathAnimIndex,y` → `jsr InitEnAnimIndex`」只服务动画三连写
（resetAnimIndex / animIndex / animDelay=0），与计时器无关。

**A 级（计时方向）**：`refs/m1disasm/SRC/prg7_engine.asm:6141-6156`（`DrawEnemy`，place==EnPlace1）

```asm
6141     inc Ens.0.chunkyExplosionTimer,x
6142     lda Ens.0.chunkyExplosionTimer,x
...
6153     cmp #$19
6154     bne Lx146
6156         jmp EnemyCompleteDeath
```

`constants_ram.asm:106` 的字段注释也写明「Counts such things as explosion time.」。
⇒ 该字节是**从 0 往上加**、到 `$19`(25) 才完成死亡；旧形「每帧减 1、≤0 完成」方向相反。

**A 级（Rust 落点，HEAD 版）**：

- `crates/game-core/src/enemy/framework.rs:257`：`mem.enemy(slot_of(px as u8)).set_data06(anim);`
- `framework.rs::update_enemy_explode`：读 `data06` 后 `timer - 1`，`timer <= 0` 时 `enemy_complete_death`

两者叠加：初值 0x27 + 每帧减 1 ⇒ f2151 时槽 1 该字节 rust `0x27` / emu `0x00`
（f2151 正是槽 1 从 Active 转 Pickup、`place` 进入 EnPlace1 的第一帧）。

## 4. 修法

只动 `crates/game-core/src/enemy/framework.rs` 的两处 + 一个断言：

1. `framework.rs:262`：`set_data06(0)`（注释引 `07:F68D-07:F698` 的 A 清零）；
2. `update_enemy_explode`（`framework.rs:394-407`）：改为
   `if timer >= 0x19 { enemy_complete_death } else { set_data06(timer + 1) }`；
3. `crates/game-core/src/frame/tests.rs:1415`（PMO 授权的写面例外，单独 commit）：
   `assert!(timer > 0, ...)` → `assert_eq!(timer, 0, ...)`，去掉「循环跑到拾取」的旧断言，
   保留 `assert_eq!(status_wram, 5)`。

新增单测（`framework.rs` 内 `tests_chunky_explode_timer`）：
`timer_increments_from_zero`（0→1）与 `timer_reaching_0x19_completes_death`（0x19 完成、不再加）。

## 5. 验证（本机实跑）

| 项 | 结果 |
|---|---|
| TAS 2200 帧 | 比较 2175 · 分歧 0 · 首分歧 null；f2151 `$0416` 两侧 `0x00` ✓ |
| TAS 10000 帧 | 比较 9967 · 分歧 0 · 首分歧 null（> f2151，已后移出窗口） |
| 修复前对照（同 emu trace） | f2151 `$0416` offset 22：emu `0x00` / rust `0x27`；共 50 帧分歧 ✓ 复现 |
| `cargo test -p game-core`（wt-roll） | 14 个 target 合计 **1071 passed / 0 failed** |
| `python3 gates/cli.py check` | 见 §5.1 实跑输出 |
| `python3 tools/l4_diff_run.py run-all --skip-emu`（wt-roll） | 172 格全 **blocked**（worktree 缺 emu 侧 `.l4prov.json` 锚，工具拒跑）；diff 0 / match 0，**不可与 #55 的 171/172 比较**，见 §6 |

**1071 与卡面 1089 的差**：wt-roll 基线 HEAD `6a48c05d` 是干净基线，缺主线工作树里
#51 未提交的 `tests_b3_common_paths`（18 项）⇒ 主线合入后即 1089。wt-roll 内 1071 与
修复前一致（本次净增 2 个新单测、并改 1 个旧断言），无测试被删。

### 5.1 门禁输出

（`gates/cli.py check` 实跑结果见提交说明；本文件按实跑贴原始行。）

## 6. 未覆盖

1. **整片 120540 帧未重跑**（emu 腿约 36 分钟）。用 10000 帧切片证明首分歧已越过 f2151；
   整片全局首分歧需要下一次全片跑给出（口径同 #55）。
2. **`run-all` 在 worktree 内不可比**：`--skip-emu` 需要主线 `shots_*` 里的 emu 侧锚
   （`.l4prov.json`），worktree 没有 ⇒ 21 用例 / 172 格全 blocked。此项须在主线 pick 后复跑。
3. 未做 emu 写钩「f2151 这一帧谁写 `$0416`」的动态取证；根因结论靠静态 ROM 汇编 + 符号表
   （A 级）与修前/修后差分对照（A 级）双重支撑。
4. `$0406` 同址三名（`chunkyExplosionTimer`/`movementInstrIndex`/`speedSubPixelY`）在爆散态
   不会同时被移动路径写（爆散态无移动），但该别名结构本身未在本卡重新取证，沿用
   `state.rs:22-45` 的既有 A 级登记。
