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

### 3.1 探针实测（`case-probe-r28d0a`，1684–1695 逐帧）

读 emu 与 rust 两侧 12 个敌槽的 `EnsExtra+0`(status)/`+14`(type)、`Ens+12`(data0A)：

- f1690–1692 活跃弹槽（6/7）**两侧 `data0A` 都是 `0`**，`type` 都是 `0`，status 都是 2；
- f1692 两侧 slot6/7 的 `resetAnimIndex=0x4D`、`animIndex=0x4E`、`animFrame=0x42`、
  `animDelay=0x01` **完全一致**。

> 【2026-11-23 02:25 更正】上一版把 room 28 的挂起点写成「派发差（emu 走 Dragon、rust 走 Polyp）」——
> **该结论是错的**：两侧 `data0A` 同为 0，槽面逐字节一致。错因是我拿**上一轮（12 B 窗之前）**的
> 失败信息（`data0A=6` 触发 falling 越读断言）当成了本轮失败信息；本轮的实际 gap 已经变成
> `unported @ bank07:E0BC-wrap idx=4E reset=02 len=138`，而 f1692 的槽面 6/7 是 `reset=4D`，
> 说明报 gap 的**不是这两个槽**（explosion 池 `$6BB4` 的槽也在走 `Anim::step`）。

### 3.2 仍待定位

- 报 gap 的对象身份未取：`Anim::step` 的 `Broken{idx,reset,len}` 目前只带三个数，
  需在 `Broken` 里再带**槽号/池名**（否则同一个 `idx=4E/reset=02` 无法指认是谁）。
- 候选：explosion 池槽（`crates/game-core/src/enemy/explosion.rs:187` 的调用点）
  用了与敌槽不同的 `resetAnimIndex` 来源。

**下一步**：给 `AnimStep::Broken` 增槽标识（一步小改，测试可钉），再复跑 room 28 取身份；
身份定了才谈表基/语义。
