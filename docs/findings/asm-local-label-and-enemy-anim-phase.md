# asm 局部标号 `X@RTS` 的读法与敌人动画相位（Brinstar room 21 案）

> 本文记录 2026-11-21/22 的一次**硬读错**及其修复。错误引入于 `cc69ee85`，
> 修复于 `1d90d08f`，由独立只读复核（`reviewer-anim`，证伪立场）判定并给出字节级证据。

## 1. 结论先行

`refs/m1disasm/SRC/prg7_engine.asm` 的 `InitEnRestingAnimIndex ; 07:F682` 里：

```
07:F682  20 44 F8   jsr $F844          ; GetEnemyTypeTimes2PlusFacingDirection
07:F685  B9 3B 96   lda $963B,y        ; EnemyRestingAnimIndex,y
07:F688  DD F9 6A   cmp $6AF9,x        ; EnsExtra.resetAnimIndex,x
07:F68B  F0 0B      beq +$0B           ; → $F698
07:F68D  9D F9 6A   sta $6AF9,x        ; resetAnimIndex
07:F690  9D FA 6A   sta $6AFA,x        ; animIndex
07:F693  A9 00 / 9D F8 6A              ; animDelay ← 0
07:F698  60         rts                ; ← ClearEnAnimDelay@RTS
```

⇒ **`reset` 索引相等 ⇒ 直接 `rts`**：不写 `resetAnimIndex`/`animIndex`，**也不清 `animDelay`**。

关键点：`ClearEnAnimDelay`（`F693`）与 `ClearEnAnimDelay@RTS`（`F698`）**不是同一处**。
`@local` 是被全名化为 `Global@local` 的**局部标号**（`refs/m1disasm/build.py` 用 WLA-DX，
sym 里即 `07:f698 ClearEnAnimDelay@RTS`），不是"执行 Global 然后 RTS"。

交叉例（同模式，均在 `refs/m1disasm/SRC/areas_common.asm`）：
`bne EnemyIfMoveFailedHorizontal_Bounce@RTS`（`:517`，sym `01:81f5 …@RTS`）、
`SetDoorEntryInfo@RTS`（`:1323`，sym `01:8b6c`，而 `SetDoorEntryInfo` 主体在 `01:8b53`）。

## 2. 错误的后果（可观测）

`UpdateEnemy_Resting ; 07:F748` 处于 `data05.bit6` 门之下，而
`UpdateEnemy_UpdateEnData05Bit6 ; 07:F3AA` 每帧把 `bit6 = ((x/16) ^ FrameCount) & 1` 翻转
（`prg7_engine.asm:10729-10759`）⇒ `InitEnRestingAnimIndex` 实际是**隔帧执行**。

于是"相等也清 `animDelay`"等价于**隔帧把动画计时器清零**：每个执行帧都会立即
`Advance` 一次，休止动画变成约 2 帧/步。

Brinstar room 21 敌槽 s2（type=6 Rio，`EnemyRestingAnimIndex` 表 offset `0x48`，
`[72]=Rio0 [73]=Rio1 [74]=FrameEnd`）实测：

| f | emu (delay, index, frame) | rust（错误实现） |
|---|---|---|
| 940 | (3, 73, 99) | (8, 74, 100) |
| 941 | (2, 73, 99) | (7, 74, 100) |
| 942 | (1, 73, 99) | (8, 73, 99) |
| 943 | (8, 74, 100) | (7, 73, 99) |
| 944 | (7, 74, 100) | (8, 74, 100) |

emu 是正常"每帧递减、归零重装"；rust 是 `8,7,8,7` 振荡 + `index` 每两步往返
⇒ 精灵取到相邻帧、相位反相。

像素面证据（room 21）：f47 与 f53 各 109px，颜色为 `(92,228,48)`（Brinstar 敌绿），
一侧画一侧不画、f50 又归零 ⇒ 典型的"帧相位振铃"。

## 3. 修复

`crates/game-core/src/enemy/prep.rs::init_en_resting_anim_index`：相等支改回纯 `return`。
新增 3 条锚测（`mod tests_en_resting_anim_index`）：

1. `equal_resting_index_preserves_delay_and_index`（相等 ⇒ delay/index 都不动）；
2. `different_resting_index_writes_index_and_clears_delay`（不等 ⇒ 写 index 且 delay←0）；
3. `resting_anim_cadence_is_eight_frames_not_two`（每 2 帧插一次 init 模拟 bit6 门，
   断言 delay 序列为 `8,7,6,5,4,3,2,1`）。

变异验证：把 `set_anim_delay(0)` 加回相等支 ⇒ 第 1、3 条同时红；复原后绿。

## 4. 验收

- room 21 单房：f47 `109→0`、f53 `109→0`、f60 `173→64`、f960 `289→180`、end `282→173`
- Brinstar 47 房全量：`ok 28 → 33`、`mismatch 19 → 14`；净转绿 room 20 / 27 / 39 / 40 / 42
  （共同点是"一房多个 Resting 敌人"）

## 5. 剩余残差不是本因（room 21）

修复后 room 21 仍有 64–206px，但 `ppu` 四区（chr/nt0..nt3/palette）全 0、
`nt_norm=0`、`layer=sprite`、`shift.explains=false`
⇒ 属**纯精灵差**（敌人位置/绘制内容），与名表、图案、调色板、动画相位都无关，是独立族。

## 6. 可复用的读法纪律

1. `beq X@local` 的目标是**局部标号**，不是 `X` 标号本身；看到 `@` 就当"另一处"处理。
2. 判"是否清某字段"必须看**分支落点**，不能只看标号名（本次就是把 `ClearEnAnimDelay` 当落点）。
3. 手工推导的时序主张，必须能被"逐帧字段表"复现（本文第 2 节的表就是判据）；
   若实测与推导不一致，先怀疑推导。
