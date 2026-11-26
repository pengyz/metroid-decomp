# Tourian 掉落配额复位（`LDD5B ; 07:DD5B`）

> 任务 #117。证据分级按 `AGENTS.md` §3：A=ROM 字节/汇编行+可复现命令，B=代码语境推断，C=假设。
> 本文件所有 ROM 行号指 `refs/m1disasm/SRC/prg7_engine.asm`（版本由 `refs` 子模块钉死）。

## 1. 症状：Rust 侧只有记账、没有行为（A）

```console
$ grep -rn "LDD5B\|0x0DD5B" crates/ docs/spec/tourian-gate.md
crates/game-core/src/enemy/framework.rs:578:        crate::frame::DIVERGENCES.with(|d| d.borrow_mut().push(0x0DD5B));
docs/spec/tourian-gate.md:196（§5 抄 ROM）
docs/spec/tourian-gate.md:217（「Rust 侧该支路未端口」）
docs/spec/tourian-gate.md:223（§6 表：A 级 ROM 原文 + B 级 Rust 缺失）
```

唯一命中的 `framework.rs:578` 只在**小 Boss 击杀支路**里记一个 `0x0DD5B` 分歧 token，随后照样
`set_status(NoEnemy)`；普通敌人的「1/16 无掉」径（`framework.rs:587` 附近）直接回收槽，注释写着
「LDD5B 非 Tourian」。全仓库没有任何地方自增 `$95/$96`（配额 Cur），也没有 `LDD30` 的配额比较。

## 2. ROM 原文（A，`prg7_engine.asm:6018-6042`）

```text
LDD5B:                     ; 07:DD5B  pickup failed to spawn
    ldx PageIndex
    lda InArea
    cmp #$13               ; ← 精确比较 $13，不是掩码
    beq Lx140
Lx139:  jmp RemoveEnemy    ; ($FA18)
Lx140:
    lda RandomNumber1
    ldy #$00
    sty EnergyPickupQtyCur ; $96 ← 0
    sty MissilePickupQtyCur; $95 ← 0
    iny
    sty MissilePickupQtyMax; $93 ← 1
    sty EnergyPickupQtyMax ; $94 ← 1
    bne LDD30              ; 恒跳：Z 来自 `c8 iny`（Y=1），与 A 无关
```

触发点（同文件 `07:DD28`/`07:DD34`/`07:DD46`/`07:DD5B`）：1/16 无掉（`bcc LDD5B`）与三类配额打满
（导弹 Cur==Max、`MaxMissiles==0`、能量 Cur==Max）。四个字节语义见 `constants_ram.asm`：
`$93 MissilePickupQtyMax`、`$94 EnergyPickupQtyMax`、`$95 MissilePickupQtyCur`、`$96 EnergyPickupQtyCur`。

## 3. 卡面与 ROM 的两处差异（按 ROM 实现）

1. **掩码 vs 精确比较**。卡面写 `InArea & 0x0F == 3`；ROM 是 `cmp #$13`。`InArea` 高半字节带
   「进区方向」等附加位（见 `crates/game-core/src/frame/mod.rs:42-46` 的 `cur_area` 文档，以及
   `set_in_area_raw` 的存在），`$10` 位有语义。实现按 ROM 比对**原值**：`world::tourian_quota::in_tourian`
   只在 `InArea == $13` 时成立；单测 `in_area_with_other_high_bits_is_not_tourian` 用 `InArea=$1F`
   （低半字节仍是 3）钉住「非 Tourian」。
2. **ROM 原版 quirk：`lda RandomNumber1` 是死读**。紧随的 `ldy #$00` 与三条 `sty` 都不改标志，
   `bne LDD30` 读的是 `c8 iny`（Y:0→1）留下的 Z=0 ⇒ **无条件跳回 LDD30**。所以 Tourian 支路
   没有分支：复位 `$93-$96` → 重试生成，`Rnd` 取任何值（含 0）都一样。
   A 级字节核（`07:DD66` 起）：`a5 2e / a0 00 / 84 96 / 84 95 / c8 / 84 93 / 84 94 / d0 bb`。
   > 【2026-11-27 更正】本文早先写「`Rnd==0` 落 `LDD75` 死循环、每圈 +75」，是把 `d0` 的 Z 误当成
   > `lda RandomNumber1` 的结果；对拍复核（#117 thread seq6）用 ROM 字节指出标志被 `c8 iny` 重写。
   > 已删 `QuotaReset::HungQuirk` 分支并补 `tourian_zero_random_still_spawns_pickup` 用例。

## 4. 实现

- 新模块 `crates/game-core/src/world/tourian_quota.rs`（写面：`crates/game-core/src/world/`）：
  - `in_tourian(mem)`：`InArea == $13`。
  - `ldd5b(mem) -> QuotaReset`：非 Tourian ⇒ `Remove`；Tourian ⇒ 写四字节
    （`$96/$95←0`、`$93/$94←1`）后返回 `Retry`（没有第三出口）。
- 挂钩（**卡面原写面之外，已在 #117 thread seq1/seq2 报 PMO 待裁**）：`crates/game-core/src/enemy/framework.rs`
  的 `enemy_complete_death` 里 `if r < 0x10` 分支改为 `match world::tourian_quota::ldd5b(mem)`：
  `Remove` ⇒ 回收槽（原行为）；`Retry` ⇒ 继续走下面的 `LDD30` 生成。小 Boss 支路同样改走这个调用
  （非 Tourian ⇒ `NoEnemy`，与原行为一致；Tourian 臂对 Kraid/Ridley 不可达）。
  逻辑全在 `world/`，framework 侧只有这一个 match（+11/−3 行）。
- typed 访问器（同上待裁）：`crates/game-core/src/frame/memory.rs` 追加 `in_area_raw`、`random1`/`set_random1`、
  `pickup_{missile,energy}_qty_{max,cur}` 的 get/set。理由：新文件若直接 `read_ram/write_ram`，
  `raw-surface` 判「新文件接入裸面 5 处」、`arch-coupling` 判「新增直接依赖 `RANDOM1`/`IN_AREA`」；
  `frame/memory.rs` 是 `tools/raw_surface.py` 的 EXEMPT 边界件。
- 不改判据/阈值；`$93-$96` 的地址语义只在这两个文件里出现（`frame/tick.rs:576-577`、
  `samus/physics.rs:1437-1441` 是既有写入点，本卡未动）。

## 5. 验收读数（A）

```console
$ CARGO_TARGET_DIR=/home/peng/workspace/wt-117-target cargo test -q -p game-core --lib tourian_quota
running 8 tests ........  test result: ok. 8 passed; 0 failed; 0 ignored; 1130 filtered out
```

八条单测（模块 5 条 + 挂钩 3 条）：

| # | 用例 | 断言 |
|---|---|---|
| 1 | `tourian_first_entry_resets_quotas` | `$96/$95←0`、`$93/$94←1`、结论 `Retry` |
| 2 | `non_tourian_leaves_quotas_untouched` | `InArea=$03`（低半字节=3、原值≠$13）⇒ `Remove`，四字节一字不动 |
| 3 | `repeated_failure_resets_again` | 第二次失败仍复位（ROM 每次失败都复位，不是「只第一次」） |
| 4 | `bne_is_decided_by_iny_not_by_random1` | `Rnd==0` 也 ⇒ `Retry`，四字节已复位（钉住 `d0` 读 `iny`） |
| 5 | `in_area_with_other_high_bits_is_not_tourian` | `InArea=$1F` ⇒ 非 Tourian |
| 6 | `tourian_drop_failure_retries_pickup` | 走真实入口 `enemy_complete_death`（`r=0x05`）：配额复位、`status=Pickup`、`animFrame=PickupTypeTable[5]=0x89` |
| 7 | `non_tourian_drop_failure_still_removes` | 同入口 `InArea=$02` + `r=0x05`：`status=NoEnemy`、配额不动 |
| 8 | `tourian_zero_random_still_spawns_pickup` | 同入口 `InArea=$13` + `r=0x00`：配额复位且 `status=Pickup`（证明没有「掉落被吞」的分支） |

回归（本工作树，含挂钩）：

```console
$ cargo test --workspace
（聚合所有 test binary）1257 passed / 0 failed

$ python3 gates/cli.py check
SUMMARY: PASS=24 FAIL=0 SKIP=3 TOLERATED=1
CHECK: PASS

$ python3 tools/l4_diff_run.py run-all
用例 21 · 采集点格 172 · match 171 · diff 1 · blocked 0 · pass 171/172
唯一 diff = case-mechanism-input-conflict（f674 已知伪差，与基线同值）
```

提交：`04d52017`（lane/117，8 files +382/−6；含两处写面外扩展与 baseline 上调，理由见提交正文与
#117 thread seq1/seq2/seq4）。

## 6. 未覆盖 / 后续

- **配额 Cur 自增与 `LDD30` 的配额比较仍未端口**（`inc MissilePickupQtyCur` / `inc EnergyPickupQtyCur`、
  `MissileQtyMax==MissileQtyCur ⇒ LDD5B`、`MaxMissiles==0 ⇒ LDD5B`、大能量 `lsr Temp00 / bcs`）。
  它们同样在 `enemy/framework.rs` 的掉落生成段，属另一张卡；在本卡实现下，`Retry` 一定成功生成
  （没有 Cur 上限），恢复的正是 ROM「把配额清掉再生成一次」的可见结果。
- `Tourian` 里 `EnsExtra.0.type` 为 `$04`/`$02` 的敌人「永不掉`（`07:DD01-07:DD0B` 的 `cmp #$04/#$02 ⇒ Lx139`）
  也未端口。
- `LDD75` 的 `PowerUpMusic` 在 Rust 侧仍是分歧记账（声包未端口），本卡未改。
