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
