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
故 f1561–1570 这一帧：emu 下移 4px、rust 下移 11px。

> 【2026-11-21 更正】原文写作"上移 4/11、多出的 7 次被阻挡回滚"——**方向反了**。
> rust 侧探针实测：`enemy_move_one_pixel_up(x=0x10)` 在整段 1800 帧里 **0 次**，
> `enemy_move_one_pixel_down(x=0x10)` **109 次**；且 `jumpDsplcmnt` 只在
> **移动成功**时更新（`enemy/motion.rs:463-465`，`data05 bit7` 清 ⇒ `dec`），
> 失败分支在写 `y` 之前就 `return false`。⇒ `−11` 是 **11 次成功的下移**，
> 不存在"回滚但计数留下"这回事。

## 3. 推论（待验）

同一帧、同一 `data05`、同一位置，两腿**成功下移的步数不同**（4 vs 11）⇒ 分歧在
「这一步该走几像素」（`EnemyGetDeltaY` 的返回幅度），**不是碰撞判据**。

> 【2026-11-21 更正】原文推断"碰撞判据分歧"，被探针推翻：碰撞只在
> `(y+radiusY)&7==0` 的块界才检，且失败不计数；`−11` 是 11 次成功。
> 下一步应查 `EnemyGetDeltaY`（`07:8244`）在这帧返回的 Δ：它由
> `(EnemyMovementPtr),y` 的符号幅值 + `Ens.delay` 计数决定，与碰撞无关。

## 4. 复现

```
python3 tools/movie_ref/room_sweep.py --areas 1 --rooms 15 --frames 300 \
  --captures 100,110,120,130,140,150,160,170,180,190,200,210 --out /tmp/s15b.json
```
