# findings：开门链 A 级定谳（UpdateAllDoors——残余 4 收口，T45-1 施工输入）

> 状态：**门开链 A 级定谳**（2026-10-18，m1disasm areas_common.asm+prg7_engine
> 内容锚×ROM 字节）。F9 阻断解除条件达成（本定谳后「门开」结论可引用）。
> 复现：`sed -n '1415,1490p' …/areas_common.asm`+`sed -n '8150,8185p' …/prg7_engine.asm`。

## 1. 窗口语义先决（关键发现）

**$8B79 双身**：bank0=$8B79=ProcessUniqueItems（唯一道具处理）；**area 窗
$8B79=UpdateAllDoors**（areas_common）——UpdateWorld `jsr $8B79` 为**窗口调用**，
游戏期窗口=当前区域 bank（bank1-5 各自同址门例）。nmikstas Bank00 注释
「display of doors」对应 area 窗语义（跨库注释位移实录——D5 互核价值样本）。

## 2. 门状态机（A 级——7 态 JumpEngine）

```
UpdateAllDoors($8B79): 门槽循环（Doors.3-Objects）→ UpdateDoor($8B87)
status 分发：0=无门/1=Init/2=Closed/3=Open/4=LetSamusIn/5=Scroll/6=LetSamusOut
Init: status→Closed+关闭动画+实心 BG 瓦+DoorHitPoints=DoorHitPointTable[DoorType]
Closed: isHit&4==0→DrawDoor；命中→dec DoorHitPoints→归零→Open！
Open/LetSamusIn/Scroll/LetSamusOut: 开门→放行→滚屏→放出（门洞流程）
DrawDoor: isHit 清 0+动画帧（6f/帧）
```

## 3. 命中链（A 级——isHit=4 写点闭环）

```
投射物×对象碰撞（$4E 分流后，07:E850 族）：
  DoorType bit0 分流：蓝门=任意投射物；导弹门=仅 wa_Missile 或 wa_BulletExplode
  +MissileExplode 动画（否则 GotoSFX_Metal=金属弹开）
  → lda #$04 / sta Objects.0.isHit,y（07:E865）← isHit 写点
→ UpdateDoor_Closed（area 窗）：isHit&4 → dec DoorHitPoints
→ 归零 → Open
```
**DoorHitPointTable**：红门=**5 击**/蓝门=1 击/10 导弹门=**10 击**/音乐蓝门=1 击。

## 4. Statue 分流（T45-1 施工输入——同例钩子）

同一碰撞例后段（07:E86C+）：**非门即雕像**判定——
```
lda Temp04_RoomRAMPtr / jsr Adiv8 / and #$01
← 房间 RAM 指针位 3 分流：Kraid 雕像=指针低半字节 $0/$1；Ridley=$C/$D
```
- **T45-1 实施对表点**：isHit=4 写同源（雕像命中与门命中同一碰撞族）；
- 雕像后续动作（抬升/闪烁）=statue 面（KraidStatueStat/$687B 位 7 抬升=T45-4 §2
  已考状态模型）——porter 实施时按本链+状态模型双表对表。

## 5. 与 F9 阻断注关系

F9 收回条件达成：5 相状态机=TileBlasts 爆破块生命周期（瓦面）；门开=本链
（对象面）——两面分离定谳完毕，「门开」结论**现可引用**（引本 findings）。
