# 雕像 / 桥断链修复（#112）——Boss 死亡 → 雕像状态 → 桥生成

状态：**已修**（两处断链补齐 + 6 条字段级单测 + 门禁/L4 回归）。
证据级别按 AGENTS.md §3 标注；本文所有地址均给出 ROM 原文或 `file:line`。

## 0. 卡面口径更正（A 级，先看这条）

卡面写「Boss 死亡后 `$687B/$687C` 的 **bit7** 必须写入」。**ROM 不是这样**：

| 事件 | 写入值 | ROM 出处 |
|---|---|---|
| Kraid 被击杀 | `$687B ← 1` | `prg7_engine.asm` `LDD75: ; 07:DD75`（`lsr / tay / sta KraidStatueStatus-1,y`，Kraid 区号低半字节=2 ⇒ y=1）|
| Ridley 被击杀 | `$687C ← 2` | 同上（Ridley 区号=4 ⇒ y=2）|
| 石像被击中并抬到位 | `$687B/$687C ← $81/$82`（`ora #$80`）| `UpdateStatue_Raise ; 07:DA3D` 末段 `ora #$80 / sta KraidStatueRaiseState-… / sta KraidStatueStatus-…` |

读侧（`UpdateStatue ; 07:DA1A`）对这两个值只做三分类：`beq`（0=未击败）、`bmi`（bit7=已抬起，
每帧画）、其余（1/2=闪烁，只在奇数帧画）。桥（`UpdateAllStatues_Bridge ; 07:DADA`）要求两值
**bit7 相与**为真。

⇒ 原版链路是三段：**死亡写闪烁值 1/2 → 玩家击像抬起到位写 bit7 → 桥解锁**。把「死亡」直接
写成 bit7 会跳过中间的抬像过程（与原版行为不符）。修复按 ROM 三段实装；「Boss 死亡后桥能解
锁」在测试里按三段串起来验证，而不是伪造一步到位。

## 1. 修复前的断链点

| 位置（HEAD） | 现状 | 缺什么 |
|---|---|---|
| `crates/game-core/src/enemy/framework.rs:520` `enemy_complete_death` | 直接 `status←Pickup`、`data0D←$60`、抽 1/16、按 `PickupTypeTable` 选掉落 | 完全没有 `lda prevStatus / asl / bmi LDD75` 这一支：小 Boss 死亡被当成普通敌炸成掉落物 |
| `crates/game-core/src/world/room_draw.rs:823` `load_enemy`（`GetEnemyType ; 07:EB28`）| 只写 `specialAttribs←etype&$C0` 与 `type←etype&$3F` | 缺「小 Boss 雕像已击败 ⇒ 整条敌记录不装载」的门，也缺 `KraidRidleyPresent←1` |
| `crates/game-core/src/world/statue.rs` | **已完整端口** `UpdateAllStatues`（含 `$54`、8 块 TileBlast、`StatueTileBlastRoomRAMPtr*` 表）与 `UpdateStatue_Raise` 的 `ora #$80` | 无缺口——桥的读侧本来就存在，缺的是上游「状态被写出来」 |

## 2. ROM 原文（A 级）

`EnemyCompleteDeath ; 07:DCF5` 开头（`refs/m1disasm/SRC/prg7_engine.asm`）：

```text
    lda Ens.0.prevStatus,x
    asl
    bmi LDD75              ; 上一个状态的高位=小 Boss ⇒ 走击杀支路
```

`LDD75 ; 07:DD75`：

```text
LDD75:
    jsr PowerUpMusic
    lda InArea
    and #$0F
    sta MiniBossKillDelayFlag
    lsr
    tay
    sta KraidStatueStatus-1,y
    lda #75
    jsr AddToMaxMissiles
    bne LDD5B
```

`AddToMaxMissiles ; 07:DD97`：`MissileCount += A`、`MaxMissiles += A`，各自进位时钉 `$FF`。
`LDD5B ; 07:DD5B`：非 Tourian（`InArea != $13`）⇒ `RemoveEnemy`；Tourian ⇒ 重置掉落配额
（`$93-$96`）后**不回收**。

`asl` 判的是 **`prevStatus` 的 bit6**（`asl` 后进 N 标志）。该位由
`EnemyReactToSamusProjectile ; 07:F5BA-07:F5C4` 写入：

```text
    lda EnsExtra.0.status,x
    cmp #enemyStatus_Frozen
    bne @endIf_C
        lda Ens.0.prevStatus,x
    @endIf_C:
    ora Temp0A_EnSpecialAttribs
    sta Ens.0.prevStatus,x
```

`Temp0A_EnSpecialAttribs` 就是 `etype & $C0`；Kraid/Ridley 的 etype 是 `$48/$49` ⇒ 存进去的是
`status | $40`。所以判据是 **bit6**（`$40`），不是 bit7（`$80` 是 tough 位，单独置位时
`asl` 会把 bit7 移出、不进 N，**不会**触发击杀支路）。

`GetEnemyType ; 07:EB28` 的读侧门：

```text
    and #$C0
    sta Ens.0.specialAttribs,x
    asl
    bpl Lx228                 ; bit6=0 ⇒ 普通敌，直接走 type 掩码
    lda InArea
    and #$06
    lsr
    tay
    lda KraidStatueStatus-1,y ; y=1→$687B(Kraid) / y=2→$687C(Ridley)
    beq Lx227
    pla
    pla                       ; 雕像非 0 ⇒ 把敌信息与返回地址都弹掉：该记录不装载
    jmp Lx225
Lx227:
    lda #$01
    sta KraidRidleyPresent
Lx228:
    pla
    and #$3F
    sta EnsExtra.0.type,x
```

## 3. 实装（只改两处业务文件）

### 3.1 `crates/game-core/src/enemy/framework.rs`

- 新增 `mini_boss_killed`（`LDD75`）：
  - `DIVERGENCES.push(0x0CBF9)` 记 `PowerUpMusic`（声包未端口，与 `world/update.rs` 的拾取音乐同口径）；
  - `MiniBossKillDelayFlag($0108) ← cur_area()`；
  - `y = area >> 1`；`write_wram($687A + y, y)`（内部常量 `STATUE_STATUS_BASE`）——即 Kraid `$687B←1`、
    Ridley `$687C←2`；
  - `add_to_max_missiles(75)`：`missile_count()/max_missiles()` 各 +75，进位钉 `$FF`。
- `enemy_complete_death` 开头插入 `prev_status() & 0x40 != 0` 分支：命中 ⇒ 走 `LDD75`，再按
  `LDD5B` 回收（`cur_area()==3` 的 Tourian 配额重置支路未端口且对 Kraid/Ridley 不可达 ⇒
  `DIVERGENCES.push(0x0DD5B)` 后按 `RemoveEnemy` 处理）。

### 3.2 `crates/game-core/src/world/room_draw.rs`

`load_enemy` 在写完 `specialAttribs` 之后、写 `type` 之前插入：

```rust
if etype & 0x40 != 0 {
    let y = (crate::frame::cur_area(mem) & 0x06) >> 1;
    if mem.read_wram(STATUE_STATUS_BASE_ADDR + y as u16) != 0 {
        return true; // pla/pla：跳过该记录
    }
    mem.write_ram(KRAID_RIDLEY_PRESENT_ADDR, 0x01);
}
```

忠实保留 asm 的 `y=0` 无区号护栏（此时读的是 `MaxMissiles`）；原版数据不会在区号 0/1 出现
bit6 敌记录，所以这条行为实际不可达。

## 4. 单测（6 条，字段级）

`crates/game-core/src/enemy/framework.rs` `mod tests_mini_boss_kill`：

1. `kraid_kill_writes_statue_blink_and_missiles`：区号 2 + `prevStatus=0x41` ⇒ `statue_status_word(0)==1`、
   Ridley 槽仍 0、`$0108==2`、导弹 10→85、上限 20→95、槽状态 `NoEnemy`（不是 Pickup）。
2. `ridley_kill_writes_own_statue_slot`：区号 4 ⇒ `statue_status_word(1)==2`、Kraid 槽仍 0。
3. `tough_non_miniboss_uses_pickup_path`：`prevStatus=0x81`（只有 tough 位）⇒ 雕像与导弹都不动。

`crates/game-core/src/world/room_draw.rs` `mod tests_statue_gate_and_bridge`：

4. `load_enemy_skips_miniboss_when_statue_already_defeated`：`$687B=1`、etype=`0x48` ⇒ 槽保持空。
5. `load_enemy_loads_miniboss_and_marks_present`：`$687B=0` ⇒ 装载（status=1）且 `KraidRidleyPresent==1`。
6. `bridge_spawns_only_when_both_statues_are_raised`：只有 `$687B=0x81` ⇒ `$54` 不置位；补上
   `$687C=0x82` ⇒ `$54 bit7` 置位且 8 块 TileBlast 的 `routine==3`（桥的读侧本就存在，这里钉住
   「双 bit7 才解锁」）。

## 5. 回归

| 项 | 结果 |
|---|---|
| `cargo test -q --workspace` | **1246 passed / 0 failed**（基线 1240，本卡 +6）|
| `python3 gates/cli.py check` | **PASS=24 FAIL=0 SKIP=3 TOLERATED=1**（`ratchet:rust_test_count` 1240→1246 报 PENDING，属预期上调）|
| `python3 tools/l4_diff_run.py run-all` | 21 案 / 172 格：match 171 · diff 1（`case-mechanism-input-conflict` f674=84px，见 `docs/findings/input-conflict-c674-residual.md`，已定谳参考图代际伪差）· blocked 0 |

## 6. 复跑命令

```bash
cargo test -q -p game-core --lib mini_boss_kill
cargo test -q -p game-core --lib statue_gate_and_bridge
cargo test -q --workspace
python3 gates/cli.py check
python3 tools/l4_diff_run.py run-all
```

ROM 原文定位：

```bash
grep -n "LDD75\|AddToMaxMissiles\|GetEnemyType\|UpdateAllStatues_Bridge\|UpdateStatue_Raise" \
  refs/m1disasm/SRC/prg7_engine.asm
sed -n '/^GetEnemyType:/,/^LEB4D:/p' refs/m1disasm/SRC/prg7_engine.asm
```

## 7. 未覆盖 / C 级

- **Tourian 配额重置支路未端口**：`LDD5B` 的 `$93-$96`（`MissilePickupQtyMax/EnergyPickupQtyMax/
  MissilePickupQtyCur/EnergyPickupQtyCur`）在 Rust 无变量；该支路要求小 Boss 死在 Tourian，
  原版数据不可达。已记账 `0x0DD5B`。
- **`KraidRidleyPresent` 的写入时机**：本修复按 ROM 在「装载小 Boss」时置 1；Tourian 音乐读它
  （`samus/handler.rs:1681`）后清零。若后续发现别的写点，属另一卡。
- **桥「可走」的观测**：桥由 `TileBlasts` 往 RoomRAM 写实心瓦生成，Rust 侧瓦面写入已在
  `statue.rs` 内；本卡只验证生成条件与 8 块 TileBlast 的初始化，未做像素级 L4 对拍（需要
  走到「双 Boss 已击败 + 石像已抬」的存档，现有 L4 场景不含该状态）。
