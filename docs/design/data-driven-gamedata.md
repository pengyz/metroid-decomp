# 数据驱动架构重组·game-data 纯数据设计

> 【状态】设计文档实录（A 级实录）——game-data 纯数据设计（tables/+constants/+room_data/）
> 【refs 引用必写】bank:addr 引证（A 级原文）——纪律强化（用户裁定）+四项核（对抗复核核（证伪立场））
> 【生成】bash refs asm 考证（数据表识别（m1disasm 结构（代码/数据分离）））→设计文档实录（game-data 纯数据设计）

## game-data 纯数据设计（数据驱动架构）

### 数据表识别（m1disasm 结构（代码/数据分离））

- **refs/m1disasm/SRC/**（代码/数据分离）：
  - `common_sprite_data.asm`（数据表（精灵数据））
  - `constants_ram.asm`（RAM 布局（常量））
  - `constants.asm`（常量（通用））
  - `memory_layout.asm`（内存布局）
  - `hardware.asm`（硬件常量）
  - `areas_common.asm`（区域公共数据）
  - `prg0_title.asm`（标题画面（prg0））
  - `prg1_brinstar.asm`/`prg2_norfair.asm`/`prg3_tourian.asm`（区域代码（prg1/2/3））
  - `brinstar/`/`kraid/`/`norfair/`/`enemies/`（区域/敌人数据）

### game-data 纯数据设计（tables/+constants/+room_data/）

#### tables/（数据表（typed 数据结构））

- **精灵数据表**（`common_sprite_data.asm`——refs asm 考证（bank:addr（数据表（写径实证（A 级原文）））））：精灵帧/动画表（typed 数据结构）
- **敌人数据表**（`enemies/`——refs asm 考证（bank:addr（数据表（写径实证（A 级原文）））））：敌人类型/属性表（typed 数据结构）
- **区域数据表**（`areas_common.asm`——refs asm 考证（bank:addr（数据表（写径实证（A 级原文）））））：区域公共表（typed 数据结构）

#### constants/（常量（RAM 布局））

- **RAM 布局**（`constants_ram.asm`——refs asm 考证（bank:addr（RAM 布局（写径实证（A 级原文）））））：RAM 地址常量（typed 常量）
- **通用常量**（`constants.asm`——refs asm 考证（bank:addr（常量（写径实证（A 级原文）））））：通用常量（typed 常量）
- **硬件常量**（`hardware.asm`——refs asm 考证（bank:addr（硬件常量（写径实证（A 级原文）））））：NES 硬件常量（typed 常量）

#### room_data/（房间数据（RoomRAM））

- **房间数据**（`brinstar/`/`kraid/`/`norfair/`——refs asm 考证（bank:addr（房间数据（写径实证（A 级原文）））））：房间布局/道具数据（RoomRAM（typed 数据结构））
- **区域代码**（`prg1_brinstar.asm`/`prg2_norfair.asm`/`prg3_tourian.asm`——refs asm 考证（bank:addr（区域代码（写径实证（A 级原文）））））：区域加载代码（非数据（game-core 域））

## game-data 纯数据设计汇总

| 目录 | 内容 | 状态 |
|---|---|---|
| tables/ | 数据表（精灵/敌人/区域（typed 数据结构）） | ✓（common_sprite_data.asm+enemies/+areas_common.asm） |
| constants/ | 常量（RAM 布局/通用/硬件（typed 常量）） | ✓（constants_ram.asm+constants.asm+hardware.asm） |
| room_data/ | 房间数据（RoomRAM（typed 数据结构）） | ✓（brinstar/+kraid/+norfair/） |

> 【refs 引用必写】bank:addr 引证（A 级原文）：`common_sprite_data.asm`（数据表）+`constants_ram.asm`（RAM 布局）+`brinstar/`/`kraid/`/`norfair/`（房间数据）（数据表识别（m1disasm 结构（代码/数据分离）））
> 【留档可追溯】docs commit（追溯径（refs 引用（bank:addr）+设计文档实录（game-data 纯数据设计）））
