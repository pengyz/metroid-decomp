# 门系统考证基础（功能5——r27 起 audit）

## 状态：考证进行中（SamusEnterDoor 全径 A 级已读；触碰写源未定位）

## SamusEnterDoor（$8B13，areas_common.asm:1317——**A 级全文已读**）

触发条件：`DoorEntryStatus==0` **且** `SamusDoorData!=0`（门数据已被写入=刚碰门）。

进入后动作序：
1. 清 `MissilePickupQtyCur`/`EnergyPickupQtyCur`；随机化拾取上限
   （`RandomNumber1&0x0F`→MissilePickupQtyMax 0-15；`asl|0x40`→EnergyPickupQtyMax 64-127）
2. 擦另一名表门块记录：`y=(PPUCTRL^1)&1; ScrollBlockOnNameTable3,y←(>>1)`
3. DoorScrollStatus 判向：
   - 横滚（ScrollDir&2）：X=2；`Samus.x` bit7（进左门）⇒ X=1
   - 竖滚：X=4；ScrollY≠0 时比较 PPUCTRL hi vs Samus.hi 定 X=3（下行）/4（上行）
   - 竖滚居中特径 L8B6D：`jsr VerticalRoomCentered($E21B)`（滚动态翻转）
4. SetDoorEntryInfo：DoorScrollStatus←X；`SamusInDoor`（DoorEntryStatus|=$80）；
   **DoorDelay←$12（18 帧）**；`SamusDoorData=(SamusDoorData*16)|Samus.status`
   （出门保留进 door 前动作——高 4 位）；`Samus.status←sa_Door`

## UpdateAllDoors（$8B79，areas_common.asm:1415——状态机概览）

门对象 JumpEngine 状态机：Init（WriteDoorBGTiles_Solid + DoorHitPoints←表）→ Closed →
…（动画每 6 帧 1 帧，DoorHitPointTable 门耐久表）。**全状态机逐态考证未完**。

## SamusDoorData 读写站（已绘图）

- 写 0：MoveSamusLeft/Right 的 `Samus.x==0`+滚动态（E663/E6A2 `Lx181/Lx186`）
- 写非零：**触碰写源未定位**（疑在瓦片碰撞/CheckMove 族 door-tile 分支——下步）
- 读：房间过渡 prg7:4130（GameMode 面）/6937（ScrollY toggle 判）/7198/7228

## 依赖与验证规划

- 依赖链：门瓦片射击（tile blast→DoorHitPoints）→ 开门动画 → 触碰 → 18 帧进门 →
  滚动过渡 → 邻房装载
- 验证：新对拍场景（走近门+射击+进门+邻房采集点）——待链路端口后立
- 不依赖 FC 相位（输入驱动事件）——调度重推导（另卡）之外的可行主线
