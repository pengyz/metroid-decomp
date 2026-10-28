# 流送内容映射考证（续战①——A 级全链实录（2026-10-28））

> 纪律：先通读 refs/m1disasm（bank:addr 级）后修改——每条规则附 asm 行号证据。
> 背景：左向列流已修（ac951fe2（$4A←ScrollDir））+bog 归因已正（d667d6cc）——
> 本族=载房/流送内容映射链。

## 1. 全链实录（asm（A 级（行号锚）））

| 件 | 址（行） | 语义 |
|---|---|---|
| ScrollLeft/Right | $E6A7/$E6D2（:7836/:7868） | ScrollDir 2/3；ScrollX 越界（0（左）/FF→00（右））⇒GetRoomNum+ToggleNameTable（**PPUCTRL^=3**（双位翻））；每 px⇒CheckUpdateNameTable |
| CheckUpdateNameTable | $E54A（:7587） | SetupRoom 首件；RoomNumber≠$FF⇒RTS；横⇒Horizontal |
| CheckUpdateNameTableHorizontal | $E701（:7906） | ScrollX&7==阈（右 0/左 7）；**ScrollDir==TempScrollDir($4A)**（防重）；idx=(ScrollX&0xF8)/8→UpdateNameTable |
| UpdateNameTable | $E590（:7652） | GetNameAddrs（PPU $20/$2C+RoomRAM $60/$64（GetNT=(PPUCTRL^ScrollDir)&1））；横滚控制位 $1E\|$80（inc32/长 30）；RoomRAM[idx+row*32]→PPU[idx+row*32] |
| GetNameTableAtScrollDir | $EB85（:8853） | (PPUCTRL_ZP^ScrollDir)&1（0=NT0/1=NT3（四象限对角模型（EB84 注块））） |
| SetupRoom | $EA2B（:8578） | $FF→RTS/$FE→占位/$F0+→AttribTableWrite/其余载房：RoomPtrTable→RoomPtr→首字节=RoomPalette→SelectRoomRAM→InitTables→DrawRoom |
| SelectRoomRAM | $EA05（:8617 附近） | GetNT→**asl asl**→ora #$60⇒RoomRAMPtr+1（$6000（NT0）/$6400（NT3）） |
| DrawRoom | $EAAA（:8666） | 对象环：$FF→**EndOfRoom**/$FE→占位/$FD→EndOfObjs（敌门环）/其余→DrawObject |
| DrawObject | $EA60（:8609） | 位=%yyyyxxxx；y 单元→**+$40/单元（2 行）**；x 单元→**×2 瓦**；StructPtrTable[idx]→DrawStruct；RoomPtr+=3 |
| DrawStruct | $EF8C（:9715） | 首字节 $FF⇒出；⇒DrawStructRow |
| DrawStructRow | $EF13（:9590） | 行头：低 nibble=元块计（**0⇒16**）/高 nibble=x 偏移（×2 瓦）；行满（ptr&$1F==0）⇒提前 AdvanceRow |
| DrawMetatile | $EF2B（:9625） | 元块 idx×4→**MetatileDefs[base+k]→TilePosTable[x]（$21/$20/$01/$00（x=3..0 逆）⇒写序=UL,UR,LL,LR（自然序）**）；UpdateAttrib；ptr+=2 |
| EndOfRoom | $EAF4（:8725） | **RoomNumber←$F0+TempScrollDir($4A)←ScrollDir**（双写）+滚向尾径 kick |
| AttribTableWrite/RoomFinished | $EA13/$EA26（:8556/:8572） | $F0-$F3 渐写→$FF |

## 2. rust 对照（映射差异）

- 元块层（TILE_POS=[0x00,0x01,0x20,0x21]（UL,UR,LL,LR）+METATILE_DEFS 序）——**同形 ✓**。
- SelectRoomRAM（$60/$64 择）——**同形 ✓**；DrawObject（y×$40+x×2）——**同形 ✓**；
  DrawStructRow（nibble 语义）——**同形 ✓**；GetNT 公式——**同形 ✓**。
- **差异 ①（实证）**：draw_room 0xFF 直终径**内联 $5A=$F0 漏 $4A←ScrollDir**——asm
  双径（DrawRoom $FF/EnemyLoop $FF）同汇 EndOfRoom（双写）——enemy_loop 尾径经
  end_of_room()（ac951fe2 修）/直终径漏⇒**流送防重判永跳⇒内容映射锁死**（f1200/
  002-f1000 族根因候选（以该径终的房））。**已修**（径汇 end_of_room）。
- 测试留档：静态 ROOM_STREAMS 不可注径（弱测（等价断言））——asm 锚为证（径汇形）。

## 3. 遗留疑点（若 ①修后残差）

- InitTables（$EFF8）attrib 初始化形；UpdateAttrib 位计算；RenderState 环模型
  （ring_x mod 512→半区）与流送半区选择的一致性（f1200 端态族）。

Evidence: prg7_engine.asm 行号锚全列+room.rs/render 对照——A 级原文。
Confidence: 92
