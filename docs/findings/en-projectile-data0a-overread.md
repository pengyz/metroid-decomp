# 敌弹 `data0A` 的越读面：窗口、可达值与一个未定谳的挂起点（2026-11-23）

## 1. 事实链（A 级）

- `EnProjectileSpawn ; 07:F8FB`：`lda Ens.0.data05,x / lsr`（把朝向 bit0 移进 carry）→
  `lda SpawnEnProjectile_EnData0A / rol / and #$07 / sta Ens.0.data0A,y`
  ⇒ `data0A = (EnData0A*2 + facing) & 7`，**`EnData0A ∈ 1..3` 是 asm 允许值**。
- `refs/m1disasm/SRC/enemies/polyp.asm:47-55`：`lda RandomNumber1 / lsr / rol data05,x /
  and #$03 / beq RTS / sta SpawnEnProjectile_EnData0A` ⇒ `EnData0A = (RNG>>1)&3 ∈ {1,2,3}`。
  `dragon.asm` 侧写死 `#$00`。
- `prg7_engine.asm:11986-11989`（下落动画）`ldy Ens.0.data0A,x / lda AreaEnProjectileFallingAnimIndex,y`
  ⇒ 字节直索引 `$95E0 + data0A`；`data0A ≥ 2` 会读进 splatter/mellow 以及
  `$95E5` 之后的 `ChooseEnemyAIRoutine` 指令字节。
- ROM 实读 bank2 `$95DC..$95E7` = `03 77 53 57 55 59 5b 4f 32 bd 02 6b`
  ⇒ 偏移 4+6 = `$95E6` = **`0x02`**，偏移 4+7 = `$95E7` = `0x6B`。
- `prg7_engine.asm:11996-12014`（溅落动画）`lda data0A / lsr / beq / iny /
  lda AreaEnProjectileSplatterAnimIndex,y` ⇒ `$95E2 + (data0A>>1)`，`data0A>>1 ∈ 0..3`
  （`y=3` 落 `$95E5` = `0xBD`）。

## 2. 已落实现

- `crates/game-data/src/extract.rs`：`EN_ANIM_IDX_LEN` 9 → **12**（`$95DC-$95E7`），
  SegMeta count 同步；`pack-tool verify` 五区 12 B 全过。
- `crates/game-data/src/assets.rs`：`en_projectile_falling_anim_byte` 断言放宽到 `y < 8`；
  新增 `en_projectile_splatter_anim_byte(area, dir)`（`$95E2+dir`）。
- `crates/game-core/src/enemy/en_projectile.rs`：溅落径改用字节直索引访问器。

## 3. 挂起点（未定谳）

Norfair room 28 在 f1692 触发：某弹 `data0A=6` ⇒ `resetAnimIndex = $95E6 = 0x02`
⇒ `EnAnimTable(2)`（bank2 `$9BDA+2`）= **`0xFF`** ⇒ `@resetAnimIndex` 回到 `resetAnimIndex=2`
= **ROM 语义下的死循环**。而 Mesen 实跑该录像并未卡死 ⇒ 二者必有一不成立：

1. emu 侧该弹 `data0A ≠ 6`（则我方 `EnData0A`/facing 与 ROM 不一致，或 polyp 的
   `(RNG>>1)&3` 取值分叉 = RNG 状态差）；
2. 或 `AnimDrawEnemy` 的帧推进并非按 `EnAnimTable[animIndex]` 直取（另有表基/偏移）。

现有 `Anim::step` 在 16 次回落后报 `Broken{idx,reset,len}` 并 `unported!("bank07:E0BC-wrap")`，
把这条未决路径变成 fail-loud（正确做法：不猜）。

### 3.1 已定谳（2026-11-23 02:20，探针 `case-probe-r28d0a`）

逐帧（1684–1695）读 emu 12 个敌槽的 `Ens+12`（`data0A`）与 `EnsExtra+0`（status）：

| f | 活跃槽 (status, data0A, y) |
|---|---|
| 1690 | slot6 (2, **0**, 0x2E) / slot7 (2, **0**, 0x1E) |
| 1692 | slot6 (2, **0**, 0x32) / slot7 (2, **0**, 0x22) |

⇒ **emu 的弹体 `data0A = 0`**（即 `EnData0A=0, facing=0` = **Dragon 径**
`dragon.asm` 写死 `#$00`），而 rust 走到了 `data0A=6`（= `EnData0A=3` = **Polyp 径**）。
所以不是"越读要不要保真"的问题，而是**同一敌槽两侧派发到了不同 AI**：
emu 是 Dragon 在吐弹，rust 是 Polyp 在吐弹。

下一步（下轮）：比对 room 28 的敌槽 `type` 与 `NORFAIR[type]` 表项，确认是
`ai_table` 映射错、还是 `polyp`/`dragon` 的触发条件（`movement`/帧相位）判据不同。

（越读窗 12 B 的实作仍然正确且必要——`(RNG>>1)&3` 三点取值确实可达；本节的结论只是说
room 28 这一处不是它。）
