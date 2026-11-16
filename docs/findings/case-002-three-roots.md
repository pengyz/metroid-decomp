# case-002 三处根因：敌绘制偏移 / 弹越界判据 / Skree 穿地

> 2026-11-16。case-002 从 **3/8 → 8/8 全 0px**，全矩阵 **match 115 → 123 · diff 17 → 9**。
> 三处分歧各自独立，但**同一类病**：asm 的一条例程在 Rust 侧有两份实现，或一份实现遗漏了分支。

## 0. 结论摘要

| # | 现象（采集点） | 根因（bank:addr） | 修在 |
|---|---|---|---|
| 1 | f1160 220px（一只精灵下半身右移 4px） | `OffsetObjectPosition ; 07:DF51` 未实现——`0xFC` 只跳过不入账 | `crates/game-core/src/enemy/anim.rs` |
| 2 | f1250/f1310 10063px（Samus 受伤、滚屏锚点差 29px） | `ApplySpeedToPosition ; 07:FD8F` 的越界判据写成「u8 溢出即杀」 | `crates/game-core/src/weapon/projectile.rs` |
| 3 | f1122 起 269px（Skree 穿地 8px） | `skree_ai` 内联自建 Δ/位移，绕过 `EnemyMove` 的逐像素碰撞 | `crates/game-core/src/enemy/skree.rs` |

## 1. 敌绘制 `0xFC` 偏移（07:DF51）

帧流里的控制字节 `$FC` 后跟两字节 (dy, dx)，含义是**累加到后续精灵的基准屏位**：

```
iny / lda (FramePtr),y / clc / adc Temp10_ScreenY / sta Temp10_ScreenY
（x 同形于 Temp0E_ScreenX）
```

实测对读：帧 37 = `28 0c 08 ce fc 00 fc de ee df fd 62 ee ff`，emu 侧像素是 CE@140 / DE@136 / EE@136 / DF@144，
正是「-4 生效于后两枚 sprite」；rust 侧全部 4px 右 = 只 `frame_i += 3` 跳过。

**这一处最值得记的不是 bug 本身，而是它藏在哪**：`crates/game-core/src/render/oam.rs::draw_enemy` 里
`0xFC` 是**正确实现**的，但那是死件——生产路径 `framework.rs:466 → enemy/anim.rs::draw_enemy` 才是活件，
而活件里是错的。插桩打了 `eprintln!` 一个字都没输出，`grep -ac EN2DBG target/debug/rust_screenshot` = 0，
才反过来证明「这段代码根本不在二进制里」。

> 教训：**插桩无输出，先证插入点进了二进制**。否则会把「死代码」误读成「逻辑没走到」，然后去改对的代码。

## 2. 弹越界判据（07:FD8F）

`ApplySpeedToPosition` 对 y 轴正向速度：

```
adc pos / bcs @then_B
        cmp #$F0 / bcc @endIf_B
@then_B adc #$0F        ; 进位=+$10，回绕到顶部
        ldy Temp02_ScrollDir / bne @exit_failure   ; 水平房 ⇒ 失败
        inc Temp0B_PositionHi                       ; 垂直房 ⇒ 跨名表
```

负向速度则用 `adc pos / bcs @endIf_C`（无借位=正常），借位才走回绕/失败。x 轴**与 y 对称相反**：
垂直房失败，水平房回绕 `inc hi`（永不杀）。

旧 Rust 写成 `overflowing_add` 后「溢出即杀」——而 `pos + 0xFB` 对任何 `pos` 都溢出，
于是**四枚弹一出生就被杀三枚**（实测 f1186 只有一枚有 dieDelay，另三枚停在出生点）。
连带修一处索引：asm 的更新循环 `txa / lsr / tay` 取的是 `x/2` = **槽号**，
旧形用 `enumerate` 让循环序 `[12,8,4,0]` 与槽号错位，速度被装到了别的弹上。

## 3. Skree 穿地（07:F410 / 07:82FB）

`UpdateEnemyCommon ; 07:F410` = `jsr UpdateEnemyAnim` + `jsr EnemyMove` + 尾件；`EnemyMove` 里
逐像素下沉，每像素过一次 `EnemyMoveOnePixelDown ; $844B`（`(y+radiusY)&7==0` 才探块）。
`skree_ai` 当时内联复制了一份 Δ 引擎并直接 `y ± |Δ|` 写回，跳过了这条路 ⇒ 无 BG 碰撞 ⇒ 落地多穿一整块（8px）。

修法是把 trigger 处理归位：`EnemyGetDeltaY@fail ; $82FB` 的「触发休止」本就在 Δ 引擎里，
搬到 `motion.rs::enemy_delta_for_move` 后，`skree.rs` 只需 `update_enemy_common_anim_move(mem, x, 2)`。

## 4. 验证

- 诊断副本 `case-002-transdbg`：8/8 全 0px。
- 正式用例 `case-002-door-blue-interaction`：**8/8**（f420/f1080/f1110/f1160/f1250/f1310/f1460/f1610）。
- 全矩阵：15 案 132 格 → **match 123 · diff 9 · blocked 0**；顺带 `jumpledge` 由 5/8 转 8/8（同一条 `EnemyMove` 径）。
- 测试 602 passed；门禁 PASS=19。

## 5. 未解释项与边界

- 本轮的像素证据只覆盖 Brinstar 首房与门过渡径；`ApplySpeedToPosition` 的**垂直房**分支仍无用例覆盖
  （矩阵 15 案中只有电梯案进垂直滚屏），实为「按 asm 实现、无像素佐证」。
- `render/oam.rs::draw_enemy` 仍是死件（只有 `crates/game-core/src/frame/tests.rs:1480` 调它）。
  按「同一 asm 例程只留一份」纪律应删除并把该测试改指活件——**未做**，登记在此。
