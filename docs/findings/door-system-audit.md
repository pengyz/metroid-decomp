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


## UpdateAllDoors 全状态机（2026-10-28 r30——A 级全文 areas_common:1415-1520）

门对象驻 Objects 槽（X 自 `Doors.3-Objects` 起、按 sizeof_Objects 步进）。状态机
（Objects.0.status 经 JumpEngine $8B87 分派）：

| 值 | 态 | 行为 |
|---|---|---|
| 0 | none | 跳过 |
| 1 | Init | status++→Closed；anim=DoorClose_Reset；WriteDoorBGTiles_Solid（实瓦）；DoorHitPoints←DoorHitPointTable[DoorType]；DrawDoor（ObjectCntrl←DoorType(3→1)\|$80\|PRI，槽位&$10^$10=h-flip；AnimDrawObject 每 6 帧） |
| 2 | Closed | isHit&4 ⇒ DoorHitPoints--；0⇒Open（status←3、HP←$50 重关延迟、DoorOpen 动画；音乐蓝门受 EndTimer 门控）；每帧 DrawDoor |
| 3 | Open | DoorEntryStatus 分派 → LetSamusIn/Scroll |
| 4/5/6 | LetSamusIn/Scroll/LetSamusOut | 过门滚动序（未逐行，下批） |

DoorHitPointTable($8BD1)：红=5、蓝=1、导弹门=10、音乐蓝=1。
被击判定：弹撞 $4E 瓦（关门瓦）→ SamusProjectileHitDoorOrStatue（E7F3，LE7E6 弹径）→
isHit&4 置位路径（未逐行）。

## 移交任务卡（功能5 完整链）

1. 门对象槽系统（Objects 门槽 init——setup_room 敌门环 EAD4 unported 臂即此处）
2. 弹击路径（SamusProjectileHitDoorOrStatue→isHit&4）
3. UpdateDoor 状态机 7 态（本档表）
4. WriteDoorBGTiles_Solid/Open VRAM 写
5. LetSamusIn→Scroll→LetSamusOut（滚动过渡+邻房）
6. 验证场景：走右→射击蓝门→开门动画→入门→邻房采集（L4 阶梯）

依赖前置：弹系（UpdateSamusProjectiles 端口度待查）。

## 依赖与验证规划

- 依赖链：门瓦片射击（tile blast→DoorHitPoints）→ 开门动画 → 触碰 → 18 帧进门 →
  滚动过渡 → 邻房装载
- 验证：新对拍场景（走近门+射击+进门+邻房采集点）——待链路端口后立
- 不依赖 FC 相位（输入驱动事件）——调度重推导（另卡）之外的可行主线
