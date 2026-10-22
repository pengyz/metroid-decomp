# 数据驱动架构（game-data=纯数据（数据驱动）+game-core=逻辑（消费 game-data）+ROM 分离对齐）

> 【2026-10-22 用户裁定】**认可**（数据驱动架构）——ROM 代码本身已做数据分离（m1disasm 结构）——game-data 应承载纯数据（数据驱动）——game-core 消费 game-data（纯数据）。

## 核心洞察

**ROM 代码本身已做数据分离**——m1disasm 汇编中：
- **代码**（子程序（bank:addr））→ `game-core`（逻辑）
- **数据**（数据表（ObjFramePtrTable/ObjAnim_*/RoomPtrs/MetatileSet/AnimFrame/MovementInstr/Song））→ `game-data`（纯数据）

**数据驱动架构**：`game-core`（逻辑）消费 `game-data`（纯数据）——**不是「解析」而是「承载」**。

## game-data（纯数据（数据驱动））

```
game-data/src/：
  tables/（数据表（typed 数据结构））
    obj_frame.rs（ObjFramePtrTable（帧指针表））
    obj_anim.rs（ObjAnim_*（动画表））
    room_ptrs.rs（RoomPtrs（房间指针表））
    metatile_set.rs（MetatileSet（元瓦片集））
    anim_frame.rs（AnimFrame（动画帧表））
    movement_instr.rs（MovementInstr（移动指令表））
    song.rs（Song（音乐表））
  constants/（常量（RAM 布局（constants_ram.asm）））
    ram_layout.rs（RAM 布局（typed 常量））
    zero_page.rs（零页映射（暂存→局部（持久→struct 字段）））
  room_data/（房间数据（RoomRAM（$6000-$67FF）））
    areas/（区域数据（brinstar/norfair/tourian/kraid/ridley））
    structures/（结构数据）
    items/（道具数据）
```

## game-core（逻辑（消费 game-data（纯数据）））

```
game-core/src/：
  kernel/（帧调度 tick 两相/NMIStatus/Timer/enum 分派/bank 窗口/RNG）
  input/（Joy*（$12-$17））
  render/（SpriteAllocator/OAM 影子/软件 PPU 光栅器+palette+scroll）
  world/{areas,rooms,structures,items}（消费 game-data/room_data/）
  samus/（状态机/物理积分/受伤/熔岩（消费 game-data/tables/））
  weapon/（导弹/炸弹/波束/TileBlasts（消费 game-data/tables/））
  enemy/（6 敌槽+16 AI+敌弹+爆炸+Mellow（消费 game-data/tables/））
  pickup/（PowerUp/Tourian items（消费 game-data/tables/））
  ui/{hud,password}（消费 game-data/tables/）
  title/ ending/（消费 game-data/tables/）
  sound/{sfx,music}（消费 game-data/tables/）
  engine/{frame,nmi,boot}（消费 game-data/constants/）
  memmap/（零页映射（消费 game-data/constants/zero_page.rs））
```

## 数据驱动的关键

| 数据类型 | game-data（纯数据） | game-core（逻辑消费） |
|---|---|---|
| **数据表**（ObjFramePtrTable/ObjAnim_*/RoomPtrs） | typed 数据结构（const 数组） | 消费数据表（渲染/动画/房间加载） |
| **常量**（RAM 布局/零页映射） | typed 常量（constants_ram.asm） | 消费常量（memmap/shim） |
| **房间数据**（RoomRAM（$6000-$67FF）） | 房间数据（areas/structures/items） | 消费房间数据（world::room 加载） |
| **ROM 分离** | **数据侧**（ROM 已分离） | **代码侧**（ROM 已分离） |

## 核心价值

1. **数据驱动**（数据决定行为（逻辑消费数据））
2. **ROM 分离**（ROM 已分离（代码/数据分离（m1disasm 结构）））
3. **可测试性**（数据可独立测试（逻辑可独立测试））
4. **Stage B 基础**（数据可替换（逻辑可复用））

## refs 引用必写

bank:addr 引证（A 级原文）：m1disasm 结构（代码/数据分离）——`refs/m1disasm/SRC/`（prg0_title.asm/prg1_brinstar.asm/prg2_norfair.asm/prg3_tourian.asm/prg4_kraid.asm/prg5_ridley.asm/prg6_graphics.asm/prg7_engine.asm）+`common_sprite_data.asm`（数据表）+`constants_ram.asm`（RAM 布局）+`rooms/`（房间数据）。
