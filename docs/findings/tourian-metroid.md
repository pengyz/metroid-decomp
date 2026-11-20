# Metroid AI 端口（Tourian `03:97F9`）

**状态**：已端口（`crates/game-core/src/enemy/metroid.rs`，20 条锚测）
**证据级**：A（asm 逐行 + `InitBank3` 数据装载表），**像素证据：无**（见 §5）

## 1. asm 事实（`refs/m1disasm/SRC/enemies/metroid.asm` 378 行）

`MetroidAIRoutine_{AREA}: ; 03:97F9` 只被 Tourian bank include（`prg3_tourian.asm:166-167`
敌表 00/01 = 红/绿 Metroid）。红/绿的差异**不在代码**，而在 `Ens.movementIndex`（0=红、1=绿）
当作数据表下标：

| 表 | 地址 | 值 | 语义 |
|---|---|---|---|
| `MetroidRepelSpeed` | `$77F0-$77F1` | `F8 08` | 脱离吸附时的弹开速度（下标 = 加速字节 bit6） |
| `MetroidAccel` | `$77F2-$77F5` | `30 D0 60 A0` | red ±48 / green ±96（下标 = `(kind<<1)|方向位`） |
| `MetroidMaxSpeed` | `$77F6-$77F7` | `02 04` | red 限速 2、green 限速 4 |
| `MetroidLatch0400-0450` | `$77F8-$77FD` | 初值全 0 | 六槽吸附锁存 |
| `MetroidLatchOffsetY` | `03:99D8` | `00 FC F9 F7 F6 F6 F5 F5 F5 F6 F6 F8` | 吸附后相对 Samus 的 y 偏移 |

`MetroidData ; 07:C5A8` 由 `InitBank3 ; 07:C59A` 拷进 `$77F0-$77FD`（13 B）。

## 2. 结构（逐段）

1. **逃逸计时器门**：`ldy EndTimer+1 / iny / beq @endIf_A` —— 只有 `EndTimer+1 == $FF`
   才**不**删除自己；否则 `status ← 0`（Tourian 逃逸倒计时一旦启动，Metroid 全体消失）。
2. 动画参数 `lda #$0F / sta $00 / sta $01`；两条早退径：`data05` bit7（爆散帧）、`status == Explode`。
3. **未吸附**：限速（见 §3 quirk）→ 按 `data05` bit0/bit2 写 `accelX/accelY` →
   冻结时若血量仍是 `$FF`（无敌值）给 5 点；非冻结恒置 `$FF`（**无敌**：只有被冻住才可杀）。
4. **`@latchActive`**：`EnemyStatusPreAI == Hurt` 且当前已不是 Hurt ⇒ `status ← Frozen`
   （被打到停止受伤那一帧冻结）。
5. **炸弹计数**：`isHit` bit5 且已吸附，武器是 `wa_Unknown7(7)` 或 `wa_BombExplode($0A)`、
   且 `FrameCount` bit1 清 ⇒ 锁存 bits4-6 += `$10`；到 5 次（`& $70 == $50`）⇒ 脱离：
   `prevStatus ← Active|specialAttribs`、`status ← Hurt`、`specialAttribs ← $20`、`data0D ← 1`。
   每次命中后都清 `isHit`/锁存/小数速度，并把速度设为 `GetMetroidRepelSpeed(accel)` 的弹开值。
6. **吸附判定**：未吸附 + `isHit` bit2（碰到 Samus）⇒ 锁存 `← (speedX & $80) | $01`
   （bit7 记速度符号）并 `ClearMetroidSpeed`（清速度/小数/加速）。
7. **吸附中**：锁存低 nibble 未满 `$0C` 就 +1；位置 = Samus 位置 + 表偏移
   （`y = MetroidLatchOffsetY[(latch&$0F)-1]`、`x = 12-(latch&$0F)`，锁存 bit7 清时 x 取负）。
8. **`@metroidOnSamus`**：六槽锁存的 `& $0C` 有满 `$0C` 且 Samus 血量非 0 ⇒
   `HealthChange ← $0004`（1/4 点）+ `SubtractHealth`，`MetroidOnSamus ← 1`；
   `ObjectCntrl` bit7 清 ⇒ `ObjectCntrl ← type | $82 | OAMDATA_PRIORITY($20)`。
9. 尾件 `UpdateEnemyCommon_Decide`（两拍都是 `$0F`）。

## 3. 原版 quirk：限速只比较低字节（D4 保真，不许"修"）

负速度径算 16 位 `|speed|` 后 `cmp MetroidMaxSpeed,y` **只比较 A（低字节）**：

- `speedY = -3`、小数 0 ⇒ `0 - $FD00 = $0300`，低字节 0 < max(2) ⇒ **不钳位**（速度保持 −3）
- `speedY = -2`、小数 `$80` ⇒ `0 - $FE80 = $0180`，低字节 `$80` ≥ 2 ⇒ 钳到 −2、小数清 0

`tests_metroid::negative_speed_low_byte_quirk_is_preserved` 把这条钉死。

## 4. 依赖的键面

- `MetroidLatch0400+slot`（`$77F8+slot`，六槽）——`frame/memory.rs` 的
  `metroid_latch()/set_metroid_latch()`（槽号 ≥6 fail-loud：原版是读到 `$77FE` 之后的
  `MetroidAccel` 表尾，属越界读，不做静默复刻）。
- `EndTimer+1`（`$010B`）——`end_timer_hi()`。
- `MetroidOnSamus`（`$92`）——`metroid_on_samus()/set_metroid_on_samus()`。
- `ObjectCntrl`（`$6B`）——`obj_cntrl()/set_obj_cntrl()`（`world/object_globals.rs`）。

## 5. 诚实边界

- **像素证据：无**。Tourian 无法用注入法进房（`docs/findings/room-sweep.md` §8.1：非 0 区
  `CopyAreaPointers` 只有 `AreaInit` 与电梯到达两处调用点），现有一张 20 用例 L4 矩阵没有一条
  进 Tourian。本卡证据 = asm 逐行 + 20 条单测（含 2 条 quirk 钉死 + 1 条「绘制尾件会消费
  `ObjectCntrl`」的观测窗说明）。
- 未取证：`movementIndex` 的初值由谁写（哪个生成器）、吸附后的屏幕坐标是否与 Samus 精灵对齐
  （属绘制面）、`SubtractHealth` 的死亡后果（走通用径）。
