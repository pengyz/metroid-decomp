# findings：banks0-6 批产施工图+红门轰开机制（排程预热件，Week 2 施工基准）

> 状态：**施工图定稿**（2026-10-18）。段面=ref_atlas segs 实算（2584 段/86.9KB
> ——巨尾切分 2588 分歧点如实）；387 簇=task-43 聚类对表（码段占比推算吻合：
> 2384 码段按 ~6.2 段/簇≈383≈387 ✓）。

## 1. banks0-6 段面实况（ref_atlas segs 实算）

| bank | 域 | 码段 | 码字节 | 数据段 | 数据字节 |
|---|---|---|---|---|---|
| 00 | 共享引擎/标题尾 | 521 | 4284B | 54 | 7971B |
| 01 | Brinstar | 301 | 2870B | 26 | 9637B |
| 02 | Norfair | 323 | 2825B | 29 | 9314B |
| 03 | Tourian | 584 | 4373B | 31 | 7828B |
| 04 | Kraid | 325 | 2755B | 30 | 9732B |
| 05 | Ridley | 322 | 2755B | 32 | 9443B |
| 06 | 全数据资产 | 8 | 25B | 2 | 12960B |
| 合计 | — | **19887B 码** | | **66890B 数据** | |

## 2. 批产施工图（387 簇→12 批，域优先序+估时）

| 批 | 域 | 簇数(推) | 优先因 | 内容焦点 | 估时 |
|---|---|---|---|---|---|
| P1 | bank00-共享 A | ~35 | **引擎尾依赖根** | UpdateWorld 族尾段/工具例（DoOneTile/UpdateTiles 族） | 1.5d |
| P2 | bank00-共享 B | ~49 | 同上 | 标题/密码尾段（T45 素材直用） | 2d |
| P3 | bank01-Brinstar A | ~28 | **起始域**（场景库覆盖最厚） | 房间/门/道具例（bundle-ff 即用） | 1.5d |
| P4 | bank01-B B | ~20 | 同上 | 敌例/区域专例 | 1d |
| P5 | bank02-Norfair | ~52 | 炎域=回环径中段 | 敌族/岩浆/门例 | 2d |
| P6 | bank04-Kraid A | ~28 | boss 域 | Kraid 房间/雕像链 | 1.5d |
| P7 | bank04-B B | ~24 | 同上 | boss 战例 | 1.5d |
| P8 | bank05-Ridley A | ~28 | boss 域 | Ridley 房间/紫门 | 1.5d |
| P9 | bank05-B B | ~24 | 同上 | boss 战例 | 1.5d |
| P10 | bank03-Tourian A | ~47 | **终局域**（依赖全能力） | Tourian 走廊/门族 | 2d |
| P11 | bank03-B B | ~47 | 同上 | Zeebetite/Mother Brain 例 | 2d |
| P12 | bank06-数据资产 | ~5 | game-data 集成面 | 全数据段（parse 面已好） | 0.5d |
| — | **合计** | **~387** | — | — | **~18d** |

**域优先序原则**：①bank00 依赖根先行 ②起始域（bank01）紧随（场景库对拍最厚）
③中段回环（bank02）④boss 域（04/05 并行可）⑤终局域（bank03）⑥数据域收口。
**批量原则**：~20-30 簇/批（1-2d 批）；码簇均 8-9B 碎段→**按调用图聚类打包**
（task-43 聚类=施工单元）；数据段整域移交 game-data。
**每批验收**：L2 案增量（勾格清单）+L3 trace 回归+全案绿。

## 3. 红门轰开机制考证窗（T45 开放项——UpdateTiles 面定谳）

**5 相状态机**（07:FE3D-FE83，DoOneTile×TileRoutine 槽 $C0-$D0 族）：
```
[1] init: TileAnim 置+TileDelay=$50+TileWRAMLo/Hi=被轰瓦 WRAM 址
[2] anim: UpdateTileAnim($02)
[3] delay: FrameCount&3==0 才递减（~$50×4f=320f 动画窗）→Table19(TileType) 换 anim
[4] anim: 续
[5] respawn: TileRoutine=0（瓦=重生态）+WRAM 址+$21 回写（LFF3C 再生）
```
- 槽族：TileRoutine/TileDelay/TileWRAMLo/Hi/TileType/TileAnimIndex（$05xx/$06xx 索引区）；
- Table19=动画索引表（10 项）+TileBlastAnim（下方表——爆破动画帧组）；
- **红门轰开链**：导弹碰撞门瓦→TileRoutine 槽=1（写入点=投射物×瓦碰撞例——
  F1xx 族/UpdateProjectiles 面，开放项①）→5 相走完=门开（瓦重生态）；
- **tile=$4E 爆破瓦**（IsBlastTile 判定=task-37 collision-family 同源）。

## 4. 开放项（施工图随件）

1. 导弹×门瓦碰撞写入点（UpdateProjectiles 面）；
2. TileType 语义表（Table19 十项↔爆破形态）；
3. 387 簇实际清单（task-43 侧——本图按占比推算，对表后微调批次）。

## 5. 【终版·对表合流】task-43 实际清单校准（2026-10-18）

**域归属对表**（task-43 387 簇实际 vs 本图占比推算）：
| 域 | 实际 | 推算 | 校准 | 判 |
|---|---|---|---|---|
| 纯 bank7（共享引擎） | **181** | ~84 | **+97** | 推算大误——调用聚类使碎段成簇（音引擎/敌/调色板公用面） |
| bank1 Brinstar | 44 | ~48 | ≈ | ✓ |
| bank2 Norfair | 36 | ~52 | -16 | 碎段≠簇 |
| bank3 Tourian | 45 | ~94 | **-49** | 同上（584 碎段→45 簇——逐指令标签形态） |
| bank4 Kraid | 43 | ~52 | ≈ | ✓ |
| bank5 Ridley | 38 | ~52 | -14 | 碎段≠簇 |
| bank0/6 | **0（待补 <50）** | ~5 | +N | 岛 verdict data/未决居多——基数 +N 随补 |
| 跨区 | 0 | — | — | 调用边区内口径 |

**施工图终版批次表**（簇边界校准——Week 2 直接照图开工）：
| 批 | 域 | 簇 | 内容焦点 | 估时 |
|---|---|---|---|---|
| S1-S2 | 共享引擎 A/B | 2×30 | 引擎根（UpdateWorld 族/工具例） | 2×1d |
| S3-S7 | 共享引擎 C-G | 5×25+6 | 音/敌/调色板/标题尾/杂项 | 5×1d |
| A1-A2 | bank1 Brinstar | 2×22 | 起始域（场景库最厚对拍） | 2×1d |
| A3 | bank2 Norfair | 36 | 炎域回环 | 1.5d |
| A4-A5 | bank3 Tourian | 2×23 | 终局域 | 2×1d |
| A6-A7 | bank4 Kraid | 2×22 | boss 域 | 2×1d |
| A8-A9 | bank5 Ridley | 2×19 | boss 域 | 2×1d |
| R1 | bank0/6 待补面 | <50 | 补面收口（岛 verdict 复核后） | 1d |
| — | **合计** | **387+N** | — | **~16.5d** |

**域优先序核**（对域归属表）：共享引擎 181 簇=**绝对主力**（47%）——
S1-S7 七批先行（依赖根+与区域批可 2 线并行）；区域批按 起始→炎→双 boss→终局 序；
R1 待补面殿后。**与推算版差异**：共享引擎批数 2→7（+5d），区域批压缩（Tourian
-2d/Norfair -0.5d）——净 +0.5d（18d→16.5d 实测口径更细）。
