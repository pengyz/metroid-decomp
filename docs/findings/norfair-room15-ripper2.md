# Norfair room 15：RipperII 的 `jumpDsplcmnt` 差 7（B 族首个定界）

> 2026-11-21。入口：`tools/movie_ref/room_sweep.py --areas 1`（电梯过区后注房号）。

## 1. 现象（A 级，逐帧）

房 15 在 f1500–f1560 两腿全 0px，**f1570 起 94px**，f2000 稳定 226px。

逐帧对读 `EnsExtra.slot1` 的 `jumpDsplcmnt`（`$6AF4+1*16+13`）：

| 帧 | emu | rust | OAM slot1 y (emu/rust) |
|---|---|---|---|
| 1500 | 18 | 18 | — |
| 1510 | 22 | 22 | — |
| 1520 | 24 | 24 | — |
| 1530 | 20 | 20 | — |
| 1540 | 14 | 14 | — |
| 1550 | 244 | 244 | — |
| 1560 | 212 | 212 | — |
| **1570** | **208** | **201** | 227 / 234 |
| 1600 | 208 | 201 | 227 / 234 |
| 1610 | 211 | 204 | — |

- **主面（`$0400` 槽 1 的 y/x/speed/isHit/data05）两腿逐字节相同**（整块 16 B 全等）。
- 只有 `EnsExtra` 的 `jumpDsplcmnt` 差 **7**，而 OAM y 恰好差 7 ⇒ 像素差完全由它派生。
- 槽 1 的 `type=2`，Norfair 表 `NORFAIR[2] = RipperII`（`enemy/ai_table.rs:106`），
  实现为 `enemy/sidehopper.rs:118 ripper_ii_ai`（只置动画档 + `update_enemy_common_decide`），
  移动由通用移动指令串驱动。

## 2. 语义

`jumpDsplcmnt` 是「本段位移的累计像素数」：`enemy_move_one_pixel_up` 在 `data05 bit7` 清时
`+1`，`_down` 时 `-1`；**受阻回滚只还原 y，不还原 `jumpDsplcmnt`**（asm 原样保留）。
故 f1561–1570 这一帧：emu 净上移 4px、rust 净上移 11px，**但最终 y 相同** ⇒
rust 多出的 7 次上移**全部被阻挡回滚**（y 回退、计数留下）。

## 3. 推论（待验）

同一帧、同一位置、同一 `type`，两腿对「这个方向上能否通行」的判断不同 ⇒
**碰撞判据分歧**，而不是移动引擎分歧。已排除：区阈值（`CheckBlastTile` 的
`$70`/`$80` 只决定可炸瓦副作用；非弹对象两径出口都是「碰撞」，
`tile_blocks_non_projectile(tile) = tile < 0xA0` 对 `$70-$9F` 恒真）。

待验方向：碰撞点所在的 **RoomRAM 页选择**（`(hi&1)<<2`）在该房竖向滚动下是否与 emu 一致。

## 4. 复现

```
python3 tools/movie_ref/room_sweep.py --areas 1 --rooms 15 --frames 300 \
  --captures 100,110,120,130,140,150,160,170,180,190,200,210 --out /tmp/s15b.json
```
