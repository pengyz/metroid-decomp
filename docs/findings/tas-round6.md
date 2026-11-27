# 第六轮：f1301 判盒 16 位重建（首分歧 f1301 → f1551）

> 任务 #137（2026-11-27）。写面：`crates/game-core/src/weapon/projectile.rs` + 本文件。
> 上游：#130（规格 `docs/spec/f1301-fix-plan.md`）、#136（判盒向量）、#124/#133（整片基线）。

## 0. 结论

1. **首分歧离开 f1301**：`--frames 4000` 干净树重跑，首个真实分歧从 **f1301 `$0454`（2 B）**
   后移到 **f1551 `$040D`（1 B，emu `$60` / rust `$5F`）**；f1301–f1349 的
   `$0454/$045E/$0424/$042E…` 串随修复一起消失。
2. **次预测命中**：#130 §6 的 C 级次预测写的是「首分歧预期后移到 f1551 `$040D`」——实测正是
   f1551 `$040D`，连字段都对上。这是**预测被观测证实**，不是事后拟合（预测文本在 #130 落盘，
   提交 `0fa0a86f` 之前的规格文件里）。
3. 判据按 ROM `CheckCollisionOfXSlotAndYSlot ; 07:F1FA` 重建：主轴 16 位比较（`sbc` 借位进
   `dhi`）、`dhi!=0` 早退、纵向房 Y 轴 240 行补偿、hi 归一用 `PPUCTRL_ZP($00FF)&1`。
4. 门径窗（#129 的 `3 vs 2`）**未被本卡触碰**，仍是独立链；本卡不掩盖它。

## 1. 改前 / 改后对照（同一处判盒）

改前（`crates/game-core/src/weapon/projectile.rs`，弹×敌判盒）：

```rust
let dy = (ey - py_y).unsigned_abs() as i16;
let dx = (ex - py_x).unsigned_abs() as i16;
if dy >= ery + pry || dx >= erx + prx {
    continue; // 未触碰
}
```

⇒ 敌人 `hi=0`(x=120)、子弹 `hi=1`(x=126) 跨 nametable，真实 262 px，被算成 6 px < 半径和 10
⇒ 误入 `07:F2CA`，写 `$040E/$0454/$045E`。

改后（同一位置）：

```rust
let vertical_room = mem.camera().dir() & 0x02 == 0;   // ScrollDir
let base = mem.ppuctrl_zp() & 0x01;                   // PPUCTRL_ZP($00FF) 的影子
let e_hi = mem.enemy((x / 16) as u8).hi() ^ base;     // EnsExtra.0.hi
let p_hi = mem.object(((py - 0x0300) / 16) as u8).hi() ^ base; // Objects.hi
let (dy, dx) = if vertical_room {
    (rom_axis_delta(ey, e_hi, py_y, p_hi, true, true),   // Y 主轴 + 240 行补偿
     rom_axis_delta(ex, e_hi, py_x, p_hi, false, false)) // X 只比低字节
} else {
    (rom_axis_delta(ey, e_hi, py_y, p_hi, false, false),
     rom_axis_delta(ex, e_hi, py_x, p_hi, true, false))  // 横滚房 X 主轴
};
let (Some(dy), Some(dx)) = (dy, dx) else { continue };   // dhi!=0 ⇒ 不碰
if dy as i16 >= ery + pry || dx as i16 >= erx + prx { continue; }
```

新增 helper `rom_axis_delta(a, ha, b, hb, use_hi, compensate_240) -> Option<u8>`，逐句对应
`07:F1FA` 的 `sec / sbc / lda Temp01_DiffHi / bne @RTS` 与 `NegateTemp00Temp01`；纵向房补偿是
`lo -= (256-240)`、借位则 `dhi -= 1`。

## 2. 四组向量与「改前必红」

单测 `weapon::projectile::tests_rom_axis_delta`（同文件，3 条）：

| a(hi) | b(hi) | 期望 | 实得 |
|---|---|---|---|
| 120(0) | 126(1) | `None`（2 字节跨页） | `None` |
| 120(0) | 130(0) | `Some(10)` | `Some(10)` |
| 130(0) | 120(0) | `Some(10)` | `Some(10)` |
| 0(0) | 0(0) | `Some(0)` | `Some(0)` |

改前红（把同一组向量装到**改前**判盒上跑，`/tmp/wt137-prefix`，判据按低字节复刻）：

```
running 1 test
thread 'weapon::projectile::tests_rom_axis_delta::four_vectors_from_spec' panicked at
crates/game-core/src/weapon/projectile.rs:824:9:
assertion `left == right` failed
  left: Some(6)
 right: None
test result: FAILED. 0 passed; 1 failed
```

`Some(6)` 就是 f1301 被误判成「6 px 相碰」的数值，`None` 是修复后的正确结论。

## 3. 首分歧后移（A 级）

命令（干净工作树 `/home/peng/workspace/wt-137`，`lane/137`，基于 `84dcfe45`）：

```
python3 tools/tas_replay_diff.py replay \
  --inputlog movies/lordtom-metroid-100p.inputlog --frames 4000 --segments 4000 \
  --out /tmp/r137/after --allow-rust-missing --timeout 900
```

- 改前（#133 整片与 #124 一致）：`first = f1301 $0454 offset84 emu=0 rust=32 diff_bytes=2`
- 改后：`first = f1551 $040D offset13 emu=96 rust=95 diff_bytes=1`（`ran_frames=4000`、
  `deterministic=true`、`env.git_head=84dcfe45`）
- `coverage.json` 的 `env.git_dirty_crates = ["M crates/game-core/src/weapon/projectile.rs"]`
  如实记录了「被测树带着本卡的未提交修复」——这是有意的：修复尚未 pick，只能在 lane 树上验证。
- 段内 `compared=4000`、分歧帧从 f1551 起；f1301–f1550 全部逐帧一致。

## 4. 次预测核对

`docs/spec/f1301-fix-plan.md` §6 次预测：「首分歧预期后移到 **f1551 `$040D`**」（原文标注 C1，
即从 #122 差异表推得、非本规格推导）。实测**完全命中**（帧号+字段+方向：emu 96 / rust 95）。
按 A3 级证据记为「预测曾被登记，观测证实」，不回头改规格措辞。

## 5. 门径窗（#129）状态

本卡只动判盒，不碰 `room_load`/滚动调用链；`docs/findings/tas-round5-full.md` 的 3 vs 2
结论不因本卡改变。本卡没有重采该窗（与判盒无因果），下一张处理该链的卡仍应以
`analysis/traces/sc-133-window.frames.jsonl` 为对照。

## 6. 门禁与测试

- `cargo test -q --workspace`：**1261 passed / 0 failed**（原 1258 + 3 条向量/对照单测；
  `tools/baseline_update.py 1261 --reason "#137 …"` 走单源更新）。
- `gates/cli.py check`：**PASS=24 FAIL=0 SKIP=3 TOLERATED=1**（8.92s）。
  - 首版判盒用裸 `read_ram(SCROLL_DIR)`/`read_wram(0x6AF4+x+7)` 触发 `arch-coupling` 3 处红；
    改为 typed 句柄（`mem.camera().dir()`、`mem.ppuctrl_zp()`、`mem.enemy(..).hi()`、
    `mem.object(..).hi()`）后转绿——**这是门禁在提示「用 B0 允许面」，不是放宽判据**。
- `weapon::` 子集 30 passed / 0 failed（既有判盒/命中面测试全绿）。

## 7. 边界（未做/未覆盖）

1. 只改了弹×敌判盒（站点1）；敌×Samus 判盒（站点2）现有的 `hi 不等 ⇒ 两轴都 +0x100` 仍是
   近似形，**本卡不动**（它已过金腿 f1112/f1167 实证；统一到 helper 属另一张卡，需带金腿回归）。
2. 未跑整片 120540 帧：4000 帧已跨过 f1301 且给出新首分歧 f1551，足以回答本卡问题；整片
   可在 pick 后由定期的整片轮次复核。
3. 半径和、`enemy_is_hit_flags`、F2CA 写顺序均未改（按规格 §4 要点④）。
4. 本卡的 `--frames 4000` 与 #133 的 `--frames 120540` 口径不同，二者只共享「first_divergence」
   一个可比值；报告里两处都标了口径，不混用。
