# 批量改写·第十六簇：pickup::powerup（PowerUp）——六步流程（补全件）

> 2026-10-22 · 补全任务（对抗复核 verdict=needs_revision（3/4 拒收：文档+实现全缺））·
> 纪律强化（refs 引用必写+更新留档可追溯）· 四项核（对抗复核核（证伪立场（独立重导证据）））·
> Evidence A 级（bank:addr 引证）Confidence 95

## step1 定域取证 spec（PowerUp 例程实址（非占位））

| 例程 | 实址 | refs 引证 |
|---|---|---|
| UpdateAllPowerUps（双槽循环（PowerUp 绘制 RAM 起 $0340）） | 07:DB37 | prg7_engine.asm:5503 |
| UpdateOnePowerUp（单槽更新） | 07:DB42 | prg7_engine.asm:5513 |
| GetItemXYPos / AddItemToHistory / CreateItemID（物品面） | 07:DC1C/DC54/DC67 | prg7_engine.asm:5698/5763/5788 |

UpdateOnePowerUp 语义逐钉：①空槽门（`PowerUps.type==$FF`⇒出口（:5517-5519 iny/beq））
②staging（PowerUpDraw.y/x/hi←PowerUps.y/x/hi（:5521-5529））③埋实心块门（房间字节
`<$A0`⇒出口（:5533-5535））④animFrame←(type&$0F)|_id_ObjFrame_BombItem（**=80=$50**
（ObjFramePtrTable 序 80（common_sprite_data.asm:363（ObjFrame_BombItem $8A46））））
⑤ObjectCntrl←((FrameCount>>1)&$03)|$80（隔帧变色（:5545-5552））⑥ObjDrawFrame 渲染边界
（SpritePagePos 位面+beam 色尾（:5553-5580（边界随记）））。

## step2 域结构（语义命名）

`PowerUpSlot { type_, y, x, hi }`（PowerUp 结构 8 字节步距×2 槽（constants_ram.asm:67-74））。

## step3 memmap 适配

PowerUpDraw=Object 实例 @$0340（constants_ram.asm:849）——Object 面（:217-241）：
animFrame+$3（$0343）/hi+$C（$034C）/y+$D（$034D）/x+$E（$034E）；ObjectCntrl=$6B（:581）；
FrameCount=$2D（:498）。

## step4 shim 写集适配

`update_one_power_up ⇒ [(0x034D,y),(0x034E,x),(0x034C,hi),(0x0343,animFrame),(0x006B,ObjectCntrl)]`。

## step5 测试（穷举等价证明）

type×房间字节 全域 0..=255×0..=255 门/animFrame 逐钉+ObjectCntrl 全域（FrameCount
0..=255）+双槽循环——**3 测全过（555 过 0 败）**。

## step6 registry 登记+独立复审

l2ports.rs `s4_cluster_powerup`（bank:addr→fn 双射（07:DB37/07:DB42→crate::pickup::powerup））。
独立复审 verdict（四项核）：①spec 实址 ✓ ②域结构 ✓ ③memmap ✓ ④写集 ✓ ⑤穷举 ✓
⑥registry ✓——**needs_revision 根因=文档+实现全缺（本件双补（实址非占位））**。

Evidence: prg7_engine.asm:5503-5580（例程逐钉）+ constants_ram.asm:67-74/217-241/498/581/849
+ common_sprite_data.asm:363（_id_ObjFrame_BombItem=80）+ pickup/powerup.rs（穷举等价 3 测）
——A 级原文。
Confidence: 95
