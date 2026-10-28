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

## 任务卡 1 实录：门对象槽系统（EAD4→LoadDoor→SpawnDoorRoutine——A 级全文）

### EnemyLoop（$EAD4，prg7_engine.asm:8705-8745——A 级）

房间对象条目流（Temp00_RoomPtr）：首字节低半字节=type→JumpEngine 八臂分派
（0/3/5=ExitSub（A=type 推进）/1=LoadEnemy（$EB06（3B：slot|type/敌型/位%yyyyxxxx））/
**2=LoadDoor（$EB8C）**/4=LoadElevator（$EC04）/6=LoadStatues（$EC2F）/
7=LoadPipeBugHole（$EC57））；$FF=EndOfRoom（$EAF4：RoomNumber←$F0+滚向尾径）。
**stride=A=臂返回值**：敌=3（GetEnemyData `lda #$03`）；**门=2**（SpawnDoorRoutine 尾
`lda #$02/rts`（:8853））；梯=2（SpawnElevatorRoutine `lda #$02/rts`（:8962））；
像/虫=待考（占位臂暂 3（非 A 级（下批核）））。

### SpawnDoorRoutine（$EB92，:8871-8853——A 级全文）

门条目=2B：[slot|type(=2), doorInfo]。
1. **side=doorInfo bit4**（`jsr Amul16`（A<<4/CF=bit4——SetMirrorCntrlBit 同 helper
   「bit0→bit4」注（字节面）+本处 CF 面（DoorSlots 几何（0x10/0x30=左）与
   `bcc @endIf_A/dey`（左门-1）双径互证（carry=bit4（非 bit3——`asl×4` 形不符几何）））。
2. **棋盘槽选**：`MapPosX+MapPosY`（adc 后 plp 传 CF=side）`rol/and#$03/tay`——
   y=((MapPosX+MapPosY)&1)<<1|side；X=**DoorSlots[y]**（$EC00：[Doors.0-Objects=$00,
   Doors.3-Objects=$30, Doors.2-Objects=$20, Doors.1-Objects=$10]（白右/白左/黑右/黑左））。
3. **DoorType[x]←doorInfo&3**（DoorType=Objects.0.data07（constants_ram:917））。
   0=红/1=蓝/2=导弹/3=音乐蓝。
4. **status init 分派**：type 1|3（蓝/音乐蓝）⇒ `@if_B: status[x]←1`（$EBD0）；
   type 0|2（红/导弹）⇒ `$09←$0A`（item id=10）/Y=MapPosX/`txa/Amul16`（CF=X bit4
   （左槽（$10/$30）⇒`dey`（Y=MapPosX-1（相邻门同开）））→`tya/jsr LEE41`（$EE41：
   **$07←A（ItemX）/$06←MapPosY（ItemY）**）→`jsr CheckForItem`（$EE4A：Y=**$6886**
   （NumberOfUniqueItems（wram 计数（含红门）））/对表 **$6885+$6886 直索 y**（UniqueItem-
   History=$6887-$68FC（2B/条））/dey/dey/bpl 环（y∈{n,…,0}）/**carry=已收**）
   →`bcs @endIf_B`（**已开（item 在史）⇒跳过 init（status 保持 0（通行））**）
   /未收⇒`@if_B: status[x]←1`（闭门（Init 态））。
5. **位域写**（@endIf_B 后）：y=side；`Objects.0.hi,x←GetNameTableAtScrollDir()`（$EB85：
   (PPUCTRL_ZP^ScrollDir)&1）；`Objects.0.x,x←DoorXs[y]`（$EBFC：右=$F0/左=$10）；
   `Objects.0.y,x←$68`（恒定）；`DoorScrollBlocks[y]`（$EBFE：右=$02/左=$01）→
   `ScrollBlockOnNameTable3[1^GetNT()] |= block`（另一名表挡滚位 OR）。
6. 返回 A=2（stride 2）。

### 字段布局（constants_ram:217-237+844-856——A 级）

Object=16B：status+0/radiusY+1/radiusX+2/animFrame+3/animDelay+4/animResetIndex+5/
animIndex+6/data07+7（**DoorType**）/speedY+8/speedX+9/isHit+$0A（**Door.isHit bit2=被弹击**
（:919））/onScreen+$0B/hi+$0C/y+$0D/x+$0E/data0F+$0F（**DoorHitPoints**（:920））。
**Objects 数组基=$0380**（`Objects .instanceof Object $10`（:844）（Samus..Object0370
占 $0300-$037F 后））；Doors=Objects.0-3（$0380/$0390/$03A0/$03B0（槽偏移 $00/$10/$20/$30））。
⇒ Door 写址：status=$0380+x/DoorType=$0387+x/isHit=$038A+x/hi=$038C+x/y=$030D+x/
x=$030E+x/DoorHitPoints=$038F+x。

> 【端口差异记账】旧占位臂 type 2/4/6/7 统一 `i+=3`——**门/梯 stride 实为 2**
> （`lda #$02/rts`（EBE5/E962））——本卡修正 2/4 臂（像/虫待考（下批 A 级核））。

Evidence: prg7_engine.asm:8705-8745/8867-8853（LoadDoor/SpawnDoorRoutine 全文）+
constants_ram.asm:217-237/844-856/917-920（Object 布局/Objects 基/DoorType/DoorHitPoints）+
9409-9470（LEE41/CheckForItem）+1228-1232（$6886 计数/$6887 历史）——A 级原文。
Confidence: 95

## 任务卡 2 实录：弹击路径（LE7E6→SamusProjectileHitDoorOrStatue——A 级全文）

> 【2026-10-28 证伪更正（三处）】①lead 报址 E7F3——实址 **$E81E**（LE7E6=$E7E6
> （入口碰撞环（:8086））（E7F3 为误记/旧符号位））；②任务卡 1 注「Objects 基=$0380」——
> **正源 Objects 基=$0300**（`Objects .instanceof Object $10`（constants_ram:844）
> （Samus..Object0370=例 0-7（$0300-$037F）/Doors=例 8-11（$0380-$03BF）/Object03C0=
> 例 12/SamusProjectiles=例 13-15（$03D0-$03FF））；任务卡 1 硬码 OBJECTS_BASE=$0380
> +相对槽（0x00/0x30/0x20/0x10）与正源 $0300+槽（0x80/0x90/0xA0/0xB0）**同物理址**
> （算术等价（行为不变（注释面更正）））；③UpdatingSamusProjectile 正源=**$71**
> （constants_ram:600）——现 mainloop.rs:1283 注「$55」疑误（下批核（不在本卡改动））。

### 入口（LE7E6，:8086-8120——A 级）

MakeRoomRAMPtr→(Temp04_RoomRAMPtr) 取瓦：`cmp #$4E`（$4E=门/像实心空瓦）⇒beq 弹击径；
否则 MotherBrain 碰撞+CheckBlastTile（<$80 实心 Exit16（rts）/≥$A0 可走瓦（Samus 的
$A0/$A1 滚门写 SamusDoorData）/$80-$9F 爆破瓦）；Lx201：`dex/beq`（检查点计数）
→CalculateNextBGCollisionPoint→环。

### SamusProjectileHitDoorOrStatue（$E81E，:8122-8185——A 级全文）

1. **弹径门**：`ldx UpdatingSamusProjectile($71) / beq ClcExit`——非弹更新径⇒即退。
2. **门环**：X=6→dex/dex/bpl（4 门（word idx 6/4/2/0）——DoorRoomRAMPtr dsw4（$5C-$63））：
   列匹配（`Temp04_hi eor DoorRoomRAMPtr+1,x / and #$04`==0 **且**
   `Temp04_lo eor DoorRoomRAMPtr,x / and #$1F`==0）。
3. **槽选**：`txa/jsr Amul8（*8）/ora #$80/tay`——Y=X*8|$80（$80/$90/$A0/$B0（Doors 例
   8-11 偏移（$0300 基）））；`Objects.0.status,y==0`⇒@next（门不存在）。
4. **类型门**（`lda DoorType,y($307+y) / lsr / bcs @blueDoor`）：
   - **偶型（红=0/导弹=2）=仅导弹伤**：弹 status（Objects.0.status,PageIndex($4B)）
     ==wa_Missile($0B)⇒@hitByMissile；或（==wa_BulletExplode($04) 且
     animResetIndex==$091（ObjAnim_MissileExplode（$8603-$8572=0x91（表算径
     （SamusJump $857E-$8572=0x0C 互证））））⇒@hitByMissile；否则⇒**GotoSFX_Metal**
     （弹开（SFX 面（随件（记账）））——**不写 isHit**）。
   - **@hitByMissile**（$E85D）：SFXTriInitFlags($0683)|=sfxTri_SamusBall(2)。
   - **奇型（蓝=1/音乐蓝=3）**：任意弹⇒直落 @blueDoor。
   - **@blueDoor**（$E865）：`lda #$04 / sta Objects.0.isHit,y`（**isHit[$30A+y]←$04
     （bit2=被弹击（constants:919）**）→bne ClcExit（即退）。
5. **雕像兜底**（环尽无门）：`Temp04_lo Adiv8 &1 / tax / inc KraidStatueIsHit,x`
   （KraidStatueIsHit=Statue.animIndex（$0366/2 条（inc 环绕（8 位））））。
6. ClcExit：clc/rts（carry=0（已处理面））。

### 依赖面

DoorRoomRAMPtr 写站=areas_common.asm:1737/1739（UpdateDoor Init 域（任务卡 3/4 面——
本卡只读消费（$5C-$63）））；Temp04_RoomRAMPtr=MakeRoomRAMPtr 产出（调用方（LE7E6 入口））。

Evidence: prg7_engine.asm:8086-8185（LE7E6/E81E 全文）+constants_ram.asm:534（PageIndex
$4B）/571（DoorRoomRAMPtr $5C）/600（UpdatingSamusProjectile $71）/903（KraidStatueIsHit）/
1126（SFXTriInitFlags $0683）+constants.asm:105/111/154（wa_BulletExplode=4/wa_Missile=
$0B/sfxTri_SamusBall=2）+common_sprite_data.asm:12/268（表算 0x91）——A 级原文。
Confidence: 95

## 任务卡 3 实录：UpdateDoor 7 态状态机（A 级全文）

### UpdateAllDoors/UpdateDoor（areas_common.asm:1415-1520（$8B79/$8B87）——A 级）

UpdateAllDoors：X=Doors.3-Objects（$B0）环（UpdateDoor；X-=sizeof_Objects($10)；bmi 环）
——4 门（$B0/$A0/$90/$80）。UpdateDoor：stx PageIndex→JumpEngine(status 0-6)：
0=ExitSub/1=Init/2=Closed/3=Open/4=LetSamusIn/5=Scroll/6=LetSamusOut。

### 逐态（:1428-1692——A 级）

- **Init（$8B9D）**：status++（→2）；animResetIndex←ObjAnim_DoorClose_Reset=**$30**
  （表算（$85A2-$8572））经 InitObjectXSlotAnimIndex（$D2FA：**Reset+Index 双写+Delay←0**
  （D2FA=sta Reset 落穿 D2FD=sta Index/lda#0/sta Delay））；WriteDoorBGTiles_Solid
  （实瓦 $4E×6）；DoorHitPoints,x←DoorHitPointTable[DoorType,x]（$8BD1=[5,1,10,1]
  （红/蓝/导弹/音乐蓝））。**落穿 DrawDoor**。
- **DrawDoor（$8BB1）**：A=DoorType（==3⇒1（音乐蓝画同蓝））；ora #$80|OAMDATA_PRIORITY
  (=$20)→**ObjectCntrl($6B)**；**isHit,x←0**；flip：(slot&$10)^$10|→$6B（$80/$A0 槽镜像）；
  `lda #$06 / jmp AnimDrawObject`（**A=6 动画周期**——rust 拆分面：update_obj_anim(mem,6)
  +anim_draw_object(mem,slot)（goal-r16 修（ObjDrawFrame 不更动画）））。
- **Closed（$8BD5）**：isHit&4==0⇒DrawDoor；dec HP；≠0⇒DrawDoor；HP 0：**音乐蓝门控**
  （type==3 且 EndTimer+1($10B)≠$FF（escape timer 活）⇒DrawDoor（不开））⇒
  **status←3/HP←$50（重关延迟）/animResetIndex←$2C（DoorOpen_Reset=859E-8572）
  +SetObjectXSlotAnimIndex（$D2FD=Index←A+Delay←0；A=DoorOpen base=**$29**（8C7E 的
  sec/sbc#(Reset-Base) 差 3））→SFX_Door。**D4 保真 BUG（登记）**：开门帧不 DrawDoor
  （asm 注「BUG! there is no call to DrawDoor」——Stage A 保留）。
- **Open（$8C01）**：进门判（DoorEntryStatus($56)≠0 且 (Samus.hi($30C)^Door.hi)&1==0 且
  (Samus.x($30E)^Door.x)&$80==0）⇒**status←4**→DrawDoor；否则 L8C1D：
  animIndex<animResetIndex（开门动画未完）⇒DrawDoor；**HP==$50（首全开帧）**：
  WriteDoorBGTiles_Air（$FF×6）；非蓝门（0/2）⇒导弹门永久开（$09←$0A/hi/MapPosX
  （Amul16（左槽 dey））→MapScrollRoutine（**桩（随件（记账））**）→**status←0（不可再关）**）；
  蓝门⇒L8C57：偶帧（FrameCount($2D) bit0=0）dec HP；0⇒**DoorSubRoutine8C61**
  （HP←1/实瓦/status←2/闭动画（8C76（Reset=$30+base=$2E（差 2））+SFX））。
- **LetSamusIn（$8C84）**：DoorEntryStatus<5（滚动未始）⇒实瓦回封+闭动画（8C76）+
  调色面（DoorPaletteChangeDir/PaletteToggle（**桩（任务卡 5 面）**））+status←5+
  DoorPaletteChangeDir←0+音乐蓝换曲（MotherBrainMusic（**桩**））；⇒L8CC3→DrawDoor。
- **Scroll（$8CC6）**：DoorEntryStatus≠5⇒DrawDoor；==5（滚动尽）⇒**配对门=X^$10**
  （槽位 bit4 翻转）：配对 status←6+animResetIndex←$2C+SetObjectXSlotAnimIndex($29)
  +SFX_Door+SelectSamusPalette（**桩**）；本门 status←2→DrawDoor。
- **LetSamusOut（$8CF0）**：DoorEntryStatus≠0⇒DrawDoor；==0⇒jmp 8C61（重关径）。

### 依赖件（同卡面）

- **WriteDoorBGTiles（$8CF7-8D33）**：Air=$FF/Solid=$4E→Common：Temp02_PositionY←$50；
  Temp03_PositionX←**DoorXTable[slot Adiv16 &1]**（$8D3A=[$E8,$10]（右/左 bg 瓦列像素））；
  Temp0B_PositionHi←Objects.hi,x；MakeRoomRAMPtr→$04/$05；**6 瓦纵写**
  （`sta (ptr),y`（y=0..$A0 步 $20））；**DoorRoomRAMPtr 写站**（任务卡 2 依赖闭环）：
  y=(slot Adiv8)&6（$80/$90/$A0/$B0→0/2/4/6）→[$5C+y]←ptr lo/hi。
- **MakeRoomRAMPtr（$E96A）**：ptr=**01100HYY YYYXXXXX**（hi=$18|Ybit7<<0|Ybit6<<1|
  (NT&4)；lo=(Y&$38)<<2|X>>3（RoomRAMA=$6000（constants:1190）））。
- **动画微件**：InitObjectXSlotAnimIndex($D2FA)=Reset 双写+Delay0；SetObjectXSlot-
  AnimIndex($D2FD)=Index+Delay0。

Evidence: areas_common.asm:1415-1750（七态+WriteDoorBGTiles+DoorXTable 全文）+
prg7_engine.asm:3984-3992（动画微件）/8401-8437（MakeRoomRAMPtr）+common_sprite_data.asm
:101-113（门 anim 常量（表算 $29/$2C/$2E/$30））+constants_ram.asm:549/812/1190（$56/$10B
/RoomRAMA）——A 级原文。
Confidence: 95

## 任务卡 4 实录：WriteDoorBGTiles 双变体（A 级定谳——含证伪更正）

> 【2026-10-28 证伪更正】任务书称「_Open 变体（门开时瓦面从 $4E 实瓦换空气/通行瓦
> ——找写入站/表）」——**定谳：`WriteDoorBGTiles_Open` 不存在**。全库标签仅二：
> **WriteDoorBGTiles_Air（$8CF7（:1694））**（实名（通行/空气面））与
> **WriteDoorBGTiles_Solid（$8CFB（:1697））**——两者**共径 WriteDoorBGTiles_Common
> （$8CFD）**，差异面**仅瓦字节**（Air=$FF（空气空白瓦）/Solid=$4E（门实瓦））
> （`lda #$FF / bne Common`（恒跳）vs `lda #$4E / fall穿 Common`）。无独立表/独立写站。

### 调用点全列（areas_common.asm 全库（4 处——A 级））

| 行 | 调用者 | 变体 | 语义 |
|---|---|---|---|
| :1445 | UpdateDoor_Init（$8B9D） | **Solid** | 门 init 实瓦（可触碰） |
| :1545 | Open L8C1D 首全开帧（$8C1D 段） | **Air** | 开门放行（通行瓦） |
| :1585 | DoorSubRoutine8C61（重关径） | **Solid** | 重关回封 |
| :1612 | UpdateDoor_LetSamusIn（$8C84） | **Solid** | 进门后回封（滚动始） |

### 差异面（唯一）

tile 参数（$4E↔$FF）；指针算（MakeRoomRAMPtr）/6 瓦纵写（y 步 $20）/DoorRoomRAMPtr
写站（y=(slot Adiv8)&6）**全共享**（Common 单径）。任务卡 3 的 `write_door_bg_tiles
(mem, x_off, tile)` 参数化形=**与 asm 双标签形等价**（tile 入参即 `lda #imm` 面）。

Evidence: areas_common.asm:1445/1545/1585/1612/1694-1699（标签+调用点全文）——A 级原文。
Confidence: 95

## 任务卡 5 实录：滚动过渡链（LetSamusIn→Scroll→LetSamusOut——A 级全文）

### 常量消费面（constants_ram——A 级）

DoorEntryStatus=$56（0=未在门/1=右门/2=左门/3=竖滚…（SamusEnterDoor 面写（b7eba84e
端口件））/DoorScrollStatus=**$57**（判向（进右门=1（横区）））/DoorDelay=**$59**
（进出门延迟帧（SamusEnterDoor SetDoorEntryInfo←**$12（18 帧）**））/PaletteToggle=**$76**
/PaletteDataPending=**$1C**/DoorPaletteChangeDir=**$91**（进调色房存 ScrollDir）。
> 【注记（下批核）】mainloop.rs:1283 旧注「SamusDoorData=$57」与 constants（$57=
> DoorScrollStatus）不符——疑误注（不在本卡改动）。

### LetSamusIn（$8C84，areas_common:1608-1653——A 级）

- `DoorEntryStatus($56) >= 5`⇒L8CC3（仅 DrawDoor）。
- `<5`（滚动未始）：①WriteDoorBGTiles_**Solid**（:1612（**回封**（Samus 已进）））；
  ②DoorSubRoutine8C76（闭动画（Reset=$30+base=$2E））；③调色面：
  DoorPaletteChangeDir($91)≠0 且 **(slot Adiv16)&1 ^ dir bit0 == 1**（同壁判）
  ⇒`PaletteToggle($76) eor #mask / sta PaletteToggle / sta PaletteDataPending($1C)`
  （mask=(_id_Palette00+1)~(_id_Palette05+1)=1^6=**7**（WLA `~`=XOR（sym 取值
  _id_Palette00=0/_id_Palette05=5（out/M1_NES_CNS.sym）+toggle 双验（1^7=6/6^7=1））））；
  ④**status←5（Scroll）**+DoorPaletteChangeDir←0；⑤音乐蓝门（type==3）：
  `txa/Amul16`（CF=slot bit4）：**右门⇒TourianMusic/左门（CF）⇒MotherBrainMusic**
  （声面（最小面（记账）））；⇒L8CC3→DrawDoor。

### Scroll（$8CC6，:1655-1680——A 级）

- DoorEntryStatus≠5（滚动未尽）⇒DrawDoor。
- ==5（滚动尽）：**配对门=slot^$10**（槽 bit4 翻转）：配对 status←**6（LetSamusOut）**
  +animResetIndex←$2C（DoorOpen_Reset）+SetObjectXSlotAnimIndex($29)+**SFX_Door**
  （声面（最小面））+**SelectSamusPalette**（Samus 调色更新（asm 注（metroid 贴头
  留蓝校正））（最小面（记账）））；本门（PageIndex）status←**2（Closed）**→DrawDoor。

### LetSamusOut（$8CF0，:1681-1692——A 级）

- DoorEntryStatus≠0⇒DrawDoor；==0（Samus 出门完）⇒jmp **DoorSubRoutine8C61**
  （重关径（HP←1/实瓦/status←2/闭动画（$30+$2E））（任务卡 3 已实装））。

### 消费面映射（SamusEnterDoor 侧（b7eba84e 端口件——本卡核对不改））

DoorScrollStatus($57)=判向（ScrollDir&2 横（X=2（Samus.x bit7⇒1））/竖（X=3/4））；
DoorDelay($59)=$12（18 帧进出门序）；Samus 隐身/出入动画帧=SamusEnterDoor 面
（DoorEntryStatus←|$80/anim 帧径（其件内））；邻房装载触发=滚动序尾（Scroll 状态
DoorEntryStatus==5 断面（配对门交接）——MapScrollRoutine 消费面**接口边界**：
仅导弹门 item 记录（任务卡 3 桩（$8C4C（记账）））本链不经。

Evidence: areas_common.asm:1608-1692（三态全文）+constants_ram.asm:549/554/566/476
/609/669（六址）+out/M1_NES_CNS.sym（_id_Palette 值）——A 级原文。
Confidence: 95

## 任务卡 6 实录：L4 验证场景（case-002-door-blue-interaction——对拍证据）

### 场景（cases/case-002-door-blue-interaction.txt（8 采集点））

走右（450-620）→射击蓝门（630/640 B）→开门动画（700）→入门（720-780）→
滚动过渡（850）→邻房（880-980）→采集 420/620/650/700/790/850/1000/1150。

### 对拍结果（`tools/l4_diff_run.py run-all`（A 级实测））

**基线**（case-002 入前（4 用例（含 3 探针））：**match 15/27**（无回退基准）。
**本件后**（5 用例×35 格）：**match 20·diff 15**（pass 20/35）——旧 27 格 **15 不回退**
（无回退 ✓）+case-002 新 8 格 **5 match/3 diff**：

| 采集点 | 结果 | 差异像素 |
|---|---|---|
| f420 game-in | ✅match | 0/61440 |
| f620 at-door | ✅match | 0/61440 |
| f650 bullet-out | ✅match | 0/61440 |
| f700 door-open | ✅match | 0/61440 |
| f790 door-enter | ✅match | 0/61440 |
| f850 scrolling | ❌diff | 400/61440（0.65%） |
| f1000 neighbor-room | ❌diff | 11059/61440（18%） |
| f1150 end | ❌diff | 11048/61440（18%） |

### 首分歧点分类（diff 图实证（shots_rust/..._f00850_scrolling.diff.png））

- **门链本体（任务卡 1-5）=全 match**（f420-790 五点 0px——门槽 init/弹击 isHit&4/
  状态机（HP--→开（DoorOpen 动画））/VRAM 瓦面/入门触发逐帧与 emu 对齐）。
- **首分歧=f850（400px）**：diff 图=**稀疏顶面对象位差**（悬挂对象/物件帧位散点
  （背景/地板/萨姆斯全同——**非滚动相位偏移**））——分类：对象绘制/滚动期对象保持面
  残差（**移植缺口候选（对象帧面）**+**调度域相位候选（r14 移交卡）**——需态面对比
  定谳（下批（不许调阈值（diff 现值如实）））。
- **f1000/f1150（18%）**=传播分歧（滚动末位移残差→邻房面放大（非门链本体））。

Evidence: shots_rust/matrix.md（5 用例×35 格实测）+case-002 场景定义+diff 图
（f850（400px 稀疏顶面对象位差））——A 级原文。
Confidence: 95
