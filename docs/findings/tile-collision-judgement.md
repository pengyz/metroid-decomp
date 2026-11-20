# 瓦片碰撞判据：可炸瓦对非弹对象是固实

**结论**：可走性判据恒为 `tile < $A0 ⇒ 阻挡`。区阈值（Brinstar `$80` / 其余区 `$70`）
**只决定是否执行"炸瓦"副作用**，不改变"这一格能不能过"。旧实现把 `>=$80` 一律当可走，
导致 Samus 与敌人都能穿过可炸瓦（`$80-$9F`）。

## 1. asm 链（A 级，逐行）

`refs/m1disasm/SRC/prg7_engine.asm`：

| 地址 | 例程 | 关键行 |
|---|---|---|
| `07:E7BD` | `CheckMoveVertical` | `cmp #$4E / beq SamusProjectileHitDoorOrStatue` → `jsr CheckBlastTile / bcc Exit16` → `cmp #$A0 / bcs IsWalkableTile` → `jmp IsBlastTile` |
| `07:D651` | `CheckBlastTile`（:4554） | `ldy InArea / cpy #$10 / beq @Brinstar`；非 Brinstar `cmp #$70 / bcs @RTS`；`@Brinstar: cmp #$80` —— **CF=1 ⇔ tile ≥ 阈值** |
| `07:E9BE` | `IsBlastTile`（:8595） | 首件 `ldy UpdatingSamusProjectile / beq @RTS` —— LDY 只改 N/Z 不改 CF，CF 仍是上一条 `cmp #$A0` 对 `$80-$9F` 的 0 |
| `07:EA03` | `IsBlastTile @exit`（:8650） | `clc` —— 弹更新径走完"找空槽→破坏瓦"也落这里 |
| `07:E81D` | `Exit16`（:8235） | `rts`（CF=0 = 碰撞） |
| `07:E800` | `IsWalkableTile`（:8215） | `sec`（CF=1 = 可走） |

⇒ 两径出口 CF 都是 0：非弹对象撞可炸瓦 = 碰撞；子弹撞可炸瓦 = 爆炸并销毁瓦（副作用），
同样不算"穿过"。

## 2. 缺陷与修法

| 位置 | 旧形 | 症状 |
|---|---|---|
| `crates/game-core/src/enemy/motion.rs::collision_point_ok` | `tile >= 0x80` 当可走 | room 15 的 Waver 提前 2px（emu 撞 `$82/$83`，rust 放行） |
| `crates/game-core/src/samus/physics.rs::collision_point_samus` | `tile != 0x4E && tile >= 0x80` | Samus 穿可炸瓦（**这一处才是 Samus 的活判据**） |

修法：`world/room_ram.rs::tile_blocks_non_projectile(tile) = tile < 0xA0` 作唯一判据，
两处消费；`tile_blocks_movement(tile, area, updating_projectile)` 已删除（零消费者，
且 `updating_projectile=true` 分支与 asm 相反）。

## 3. 过程中被证伪的两次主张（留档）

1. **「只改两处 `collision_point_ok` 就够了」**——Samus 侧改的是死代码
   （编译器 `warning: function collision_point_ok is never used`），活判据在
   `collision_point_samus`。⇒ 修完必须用**消费路径测试**验证，不能只看函数存在。
2. **「弹更新径可穿可炸瓦」**——`@exit: clc` 实证两径都是碰撞。

## 4. 证据与验收

- room 15 房扫：`f84=129 → 0`、`f88=133 → 0`、`f120=156 → 0`（残 `f960=54/end=290` 另卡）。
- 新测：`tests_tile_blocks` 4 条（全 256 瓦边界）、`tests_samus_tile_collision` 3 条
  （`$80/$82/$9F` 阻挡、`$A0/$A1` 可走且写 `SamusDoorData`、`$4E` 阻挡）。
- `run-all`：match 154 · diff 6 · blocked 0（160 格），与基线逐格相同——官方矩阵无用例
  走过可炸瓦，故零增益零回归（房扫才是这条判据的验收面）。

## 5. 遗留

- 可炸瓦的"炸开"副作用（弹命中后把瓦置空 + 爆散动画）尚未端口；当前子弹径只记账
  `DIVERGENCES`，不销毁瓦。属独立任务卡。
