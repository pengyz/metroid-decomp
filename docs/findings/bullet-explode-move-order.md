# 弹「撞敌爆」与「瓦判移动」的顺序（100% 录像 f2136 定谳）

**日期**：2026-11-16
**证据级**：A（asm 逐字节 + 两侧 RAM 影子 OAM 逐字节对读 + 像素窗口）

## 1. 现象

TASVideos 100% 全收集录像（Lord Tom 33:25.70，`.fm2`）逐帧对拍，采样点 f2135–f2165：

| 帧 | 修前 | 修后 |
|---|---|---|
| f2135 | 一致 | 一致 |
| f2136 | **42px** | **0px** |
| f2137 | 98px | 98px（另一根因，见 §5） |

## 2. 定位

`shots_rust/…_f02136.ram.hex` 与 `analysis/traces/…_f02136.ram.hex` 的 **OAM 影子 $0200** 逐字节比较，只差 4 字节；逐槽读 `$03D0`（弹 0）/`$03E0`（弹 1）得：

```
f2134  弹0 status=1 x=210   （两侧同）
f2135  emu status=4 x=214   ← 先爆（撞敌）再前移一步
       rust status=4 x=210  ← 停在撞点
f2136  emu 4/214, 4/212 ; rust 4/210, 4/208
```

即：撞敌的弹在 rust 侧**少走了一步**。

## 3. 根因

`crates/game-core/src/weapon/projectile.rs::update_bullet` 旧形：

```rust
let mut collided = false;
if mem.read_ram(base + 0x0A) != 0 {   // isHit ≠ 0（撞敌）
    mem.write_ram(base + 0x0A, 0);
    bullet_explode(mem);
    collided = true;                   // ✗ 把「撞敌」当成「已碰撞」
}
if !collided { /* CollisionWithBG */ } // ✗ 于是跳过瓦判与移动
```

asm（`refs/m1disasm/SRC/prg7_engine.asm:4298` `UpdateBullet: ; 07:D4EB`）是三步**串联且互不短路**：

```
jsr UpdateBullet_DeleteIfOffScreen     ; $D5FC
jsr UpdateBullet_ExplodeIfHitSprite    ; $D5DA  ⇒ 不设 carry
jsr UpdateBullet_CollisionWithBG       ; $D609  ⇒ carry 由瓦判给
CheckBulletStat: ; $D4F8
    bcc @collided                      ; carry 清＝瓦判固实
```

`ExplodeIfHitSprite` 只把 `status←4` 并清 `isHit`，**不设置任何「已碰撞」标志**；`CollisionWithBG` 恒执行，`carry` 只由瓦判（`cmp #$A0 / bcs` = 空气 ⇒ 移动）决定。于是**撞敌且前方空气的弹，同帧既爆又前移一步**。

## 4. 修复

`collided` 只表示「瓦判为固实」，撞敌爆不再短路瓦判：

```rust
if mem.read_ram(base + 0x0A) != 0 {
    mem.write_ram(base + 0x0A, 0);
    bullet_explode(mem);               // 只爆，不置 collided
}
let mut collided = false;              // ← 只由瓦判置位
{ /* CollisionWithBG：空气⇒bullet_move；固实⇒collided=true */ }
```

单测（`crates/game-core/src/weapon/projectile.rs::tests_bullet_explode_order`）：

- `hit_sprite_then_move_when_air_ahead`：isHit=1 且前方 `$A0` ⇒ `status=4` **且** `x 210→214`
- `hit_sprite_then_stop_when_solid_ahead`：isHit=1 且前方 `$01` ⇒ `status=4` 且 `x` 不动

## 5. 追加：f2137（受伤敌调色板）——同一条 `$6B` 线的两个缺口

f2136 修好后首分歧前移到 f2137（98px），`$0200` 影子 OAM 槽 0–3 的 attr `rust 0xA2 / emu 0xA0`；调用链 `crawler_ai → update_enemy_common_no_move_no_anim → draw_enemy → draw_sprite_stream`，`ctrl=0xA7`（帧记录表里 `0xA7/0x27/0x67/0xE7` 四个翻转变体，调色板位都＝2）。

两处缺口（都在 `$6B` 这条「覆盖态」线上）：

1. **`UpdateEnemy_Hurt ; 07:F4EE` 的调色板覆盖从未实现**——asm `@exit ; 07:F510`
   `lda #$80 | $00 | OAMDATA_PRIORITY / jmp UpdateEnemyCommon@setHurtPalette`，
   即受伤敌恒用调色板 0。rust 旧形不写 `$6B` ⇒ 受伤敌按帧记录的调色板 2 画。
   同时旧形把「画」放在 hitstun 递减**之前**，hitstun 结束那一帧仍按受伤态画（晚一帧恢复）。
2. **`$6B` 帧终未清**——asm `DrawSpriteObject` 的 `$FF` 终字节直接 fallthrough 到
   `ClearObjectCntrl ; 07:DF2D`。rust 的 `draw_sprite_stream` 缺这一步 ⇒ 受伤敌写的覆盖态
   **泄漏给同一帧后续对象**（槽 0 的爬虫 attr 被污染成 `0xA0`）。

修法：`update_enemy_hurt` 改为 asm 顺序（递减 → 恢复 → `$6B=0x80|0x20` → 画），
`draw_sprite_stream` 在帧终写 `$6B=0`。

结果：录像窗口 f2135–f2165 一致 **7 → 17**（首分歧 f2137 → **f2152**，204px）。

## 6. 遗留

f2152 起 204px，未定界（继续下钻）。

## 7. 方法论补充

## 6. 方法论

- **OAM 影子在 `ram.hex` 的 `$0200` 段**（iram = `$0100-$07FF`），比逐像素反推快一个数量级：一次 `bytes.fromhex` + 逐字节 diff 就能把「差几下」压缩到「哪几个槽的哪几个字节」。
- `--trace oam:<f0>-<f1>` 只给 rust 侧；两侧对齐要靠 `$0200` 或像素。
- `std::backtrace::Backtrace::force_capture()` 在 `draw_sprite_stream` 的写口按屏幕坐标触发，可一击定位「这枚精灵是谁画的」。
- **`$6B`（ObjectCntrl）是「覆盖态」，不是常量**：写入者（tough/冻结/受伤/闪帧/门）与清除者（`DrawSpriteObject` 帧终、`obj_draw_frame` 尾）必须成对；否则同一缺陷会以「某个对象偶尔颜色错」的形式零散复现。
