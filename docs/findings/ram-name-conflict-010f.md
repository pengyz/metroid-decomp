# `$010F` 命名冲突取证：`SamusHurt010F` vs `WalkSoundDelay=$53`

日期：2026-11-26　角色：拆骨（二进制分析）　卡：#89
范围：只读 `refs/m1disasm/` 与 `crates/game-core/src/`；实际改动只有 `samus/handler.rs` 与本文件（`ram_map.rs` **零改动**，理由见 §4）。

## 0. 结论

- `$010F` 的 asm 语义是 **`SamusHurt010F`**：四处只写、具名读 0 处（原版 quirk，D4 保真保留）。Rust 常量 `SAMUS_HURT_010F=0x010F`（`ram_map.rs:203`）**正确，保留不动**。
- `WalkSoundDelay` 在 **`$53`**：`constants_ram.asm:568`，写点 `prg7_engine.asm:2826`（`SetSamusRun` 置 9）与 `:2929`（归零重载 9），递减点 `:2925`（`dec WalkSoundDelay`）。
- 错的一侧是 **`samus/handler.rs`**：改前 `:289`（`let wd_addr = 0x010F`）与 `:586`（`mem.write_ram(0x010F, 0x09)`）把 `0x010F` 当成 `WalkSoundDelay`（注释写「WalkSoundDelay（constants 随核）」）。
- 修正形态：地址收敛为 **`handler.rs` 文件内单点常量** `const WALK_SOUND_DELAY: u16 = 0x0053;`（改后 `handler.rs:338`），只经 `walk_sound_tick()` / `set_walk_sound_delay()` 两个 helper 访问；**没有**在 `ram_map.rs` 新增常量（原因见 §4）。`SAMUS_HURT_010F` 保持原址。
- 影响面：旧形让走音计时器占用 `SamusHurt010F`，等于**每 10 帧覆盖一次「Samus 被什么击中」的 quirk 记录**（该字节原本只由受击路径写、无人读）。像素面不显形（该字节无人读），但状态面与「保真原版 quirk」的语义被污染，属 B 级债。

## 1. `$010F` 全表（`SamusHurt010F`）

| 类别 | 位置 | 内容 |
|---|---|---|
| 具名声明 | `constants_ram.asm:841` | `SamusHurt010F db ;$010F ;never read. takes on different values depending on how samus was hit.` |
| 写 | `prg7_engine.asm:10063` | `sta SamusHurt010F` |
| 写 | `prg7_engine.asm:10521` | 注释 `:10518`「set SamusHurt010F to enemy type」后 `sta` |
| 写 | `prg7_engine.asm:10602` | `sta SamusHurt010F` |
| 写 | `prg7_engine.asm:10621` | 注释 `:10619`「set SamusHurt010F to #$E0」后 `sta` |
| 具名读 | （无） | 全仓 `grep -rn 'SamusHurt010F'` 只有上述 4 写 + 声明 |

复现：

```bash
grep -rn 'SamusHurt010F' refs/m1disasm/SRC/*.asm
# 期望：5 行（1 声明 + 4 写），无 lda/cmp 类读
```

Rust 侧对应（A 级一致）：`ram_map.rs:203` 常量、`samus/state.rs:538/541` 访问器、`weapon/projectile.rs:507`（`$FF` 重置）、`:631`（`$F298` 写敌 type）、`:695`（`$F313` 写 `$80`）、单测 `enemy/en_projectile.rs:744`。

## 2. `WalkSoundDelay` 全表（`$53`）

| 类别 | 位置 | 内容 |
|---|---|---|
| 具名声明 | `constants_ram.asm:568` | `WalkSoundDelay db ;$53` |
| 写 | `prg7_engine.asm:2826` | `SetSamusRun`（`07:CC98`）开头 `lda #$09 / sta WalkSoundDelay` |
| 递减 | `prg7_engine.asm:2925` | `dec WalkSoundDelay`（`samL08`，`07:CD2B`） |
| 归零重载 | `prg7_engine.asm:2929` | `bne` 不跳时 `lda #$09 / sta WalkSoundDelay` + `jsr SFX_SamusWalk` |

```bash
grep -rn 'WalkSoundDelay' refs/m1disasm/SRC/*.asm
# 期望：4 行 —— constants_ram.asm:568 声明；prg7_engine.asm:2826 / :2925 / :2929
```

语义：`SetSamusRun` 把计数置 9，地面跑动帧每帧减 1，归零时重载 9 并起一次走路音效 ⇒ 10 帧一响。计数器**只在跑动径递减**，其它状态不碰。

## 3. 错的一侧：`samus/handler.rs`

```bash
sed -n '286,296p;583,589p' crates/game-core/src/samus/handler.rs
```

改前：

- `:289` `let wd_addr = 0x010F; // WalkSoundDelay（constants 随核）`
- `:586` `mem.write_ram(0x010F, 0x09); // WalkSoundDelay（constants 随核）`

两处都在「占位臂记账」框架下（SFX 未端口，只保计时面）。计时逻辑本身（减 1、归零重载 9、只在地面跑动径）与 asm `:2919-2932` 一致；错的只是**地址**。

## 4. 修正

- `handler.rs` 抽出 `fn walk_sound_tick(mem: &mut Manager)`，`samL08` 调它；`fn set_walk_sound_delay(mem, v)` 对应 asm `:2825-2826`（`SetSamusRun` 的 `lda #$09 / sta WalkSoundDelay`）。两处 `0x010F` 清零（改后 `set_samus_run` 内是 `set_walk_sound_delay(mem, 0x09)`，原 `:586` 那行已不存在）。
- **地址放 `handler.rs` 而不入 `ram_map.rs`**，两个门禁约束：
  1. `nes-map`：`data/status/nes_map.json` 的 `undeclared.refs` 用**行号**定位（含 `ram_map.rs:264`），在 `ram_map.rs` 插 11 行会让它指向注释行、报 `bad-refs $004C`；
  2. `arch-coupling`：`mem.write_ram(ram_map::X, …)` 属「新增直接依赖」，账本只减不增，不许新增条目。
  故 `const WALK_SOUND_DELAY: u16 = 0x0053;` 收在 `handler.rs` 单点，生产面不散落裸字面量。真正 typed 化 `$53` 需改 `nes_map.json` 的 refs + 加域访问器，属另一张卡。
- 不碰 `weapon/projectile.rs` 的 `SAMUS_HURT_010F` 写点（那是真语义）。

自证（实跑输出）：

```bash
grep -rn '0x010F' crates/game-core/src/samus/handler.rs
# 期望：无输出（改后该文件已无 $010F 字面量）
grep -rn 'WALK_SOUND_DELAY\|walk_sound_tick\|set_walk_sound_delay' crates/game-core/src/
# 期望：全部在 samus/handler.rs —— :338 常量定义、:342 tick、:354 set、:613 调用点，
#       加 mod tests_walk_sound_delay 内 4 条单测；ram_map.rs 无命中
```

## 5. 单测（互不串）

`handler.rs` 新增 `mod tests_walk_sound_delay`：

1. `walk_sound_tick` 递减 `$53`、不碰 `$010F`；
2. 归零帧重载 9 且仍不碰 `$010F`；
3. 反向：写 `SAMUS_HURT_010F=0x80` 不影响 `$53` 的计数读回；
4. `set_walk_sound_delay(mem, 0x09)` 只写 `$53`、不碰 `$010F`（对应 `SetSamusRun`）。

复现：

```bash
cargo test -p game-core walk_sound
```

## 6. 未覆盖 / 假设

- C1（假设，未取证）：SFX 端口后 `jsr SFX_SamusWalk` 的调用点应落在 `walk_sound_tick` 的归零分支内；本次只保留 DIVERGENCES 记账点 `$CBAC`（原形已在 `samL08` 内，位置不变）。
- C2（假设）：`SetSamusRun` 的置 9 是「进跑动即响下一次」的语义；原版无其它重置点（全表 4 行已列）。
- C3（复核非阻断，保留现状）：`walk_sound_tick` 的归零帧每次 `DIVERGENCES.push(0xCBAC)`，属事件流口径；消费者按唯一地址集合统计，结论不受影响，但会随跑长线性膨胀。若要改「只记首帧/去重」，会动 `frame::DIVERGENCES` 的消费约定，不在本卡写面。
