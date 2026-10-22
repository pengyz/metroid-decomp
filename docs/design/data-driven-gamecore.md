# 数据驱动架构重组·game-core 逻辑（用户裁定（认可（数据驱动架构）））

## 1. game-core 逻辑设计（消费 game-data（纯数据））

### 1.1 逻辑识别（m1disasm 结构（代码/数据分离））

**逻辑识别**（m1disasm 结构（代码/数据分离）（refs/m1disasm/SRC/（prg0_title.asm/prg1_brinstar.asm/.../prg7_engine.asm（代码（子程序（bank:addr）））））：
- prg0_title.asm/prg1_brinstar.asm/.../prg7_engine.asm（代码（子程序（bank:addr）））
- 代码/数据分离：game-core=逻辑（消费 game-data（纯数据））

### 1.2 game-core 逻辑设计（kernel/+input/+render/+world/{areas,rooms,structures,items}/+samus/+weapon/+enemy/+pickup/+ui/{hud,password}/+title/+ending/+sound/{sfx,music}/+engine/{frame,nmi,boot}/+memmap/）

**game-core 逻辑设计**（消费 game-data（纯数据））：
- kernel/：帧管线核心（tick/SpriteAllocator/EnemyAi/World/ApuFrame）
- input/：控制器状态（NES 手柄（按键序列））
- render/：渲染抽象（PPU 寄存器→渲染输出（256×240 像素））
- world/{areas,rooms,structures,items}/：地图状态（房间/地形）
- samus/：主角状态（位移/速度/形态（变球态））
- weapon/：武器系统（射击/导弹）
- enemy/：敌状态（dying/dead/hurt?）
- pickup/：物品系统（HighJump/Varia/Gravity）
- ui/{hud,password}/：UI 系统（能量条/地图）
- title/：标题菜单
- ending/：游戏结束
- sound/{sfx,music}/：音频系统（SFX/音乐）
- engine/{frame,nmi,boot}/：帧管线核心（tick 两相+帧预算记账）
- memmap/：零页分层映射（暂存→局部（持久→struct 字段））

## 2. refs 引用必写（bank:addr 引证（A 级原文））

**refs 引用必写**（bank:addr 引证（A 级原文））：
- prg0_title.asm/prg1_brinstar.asm/.../prg7_engine.asm（代码（子程序（bank:addr）））
- 代码/数据分离：game-core=逻辑（消费 game-data（纯数据））

## 3. 交付判据

设计文档实录（docs/design/data-driven-gamecore.md（game-core 逻辑设计（消费 game-data（纯数据））））——「候令」禁用。
