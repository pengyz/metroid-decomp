# Phase 0 架构设计·模块划分方案（用户裁定（认可（按计划推进）））

## 1. 域级模块×逻辑功能级单元（kernel/input/render/world/samus/weapon/enemy/pickup/ui/sound/game-data）

**模块 DAG**：
- kernel：帧管线核心（tick/SpriteAllocator/EnemyAi/World/ApuFrame）
- input：控制器状态（NES 手柄（按键序列））
- render：渲染抽象（PPU 寄存器→渲染输出（256×240 像素））
- world：地图状态（房间/地形）
- samus：主角状态（位移/速度/形态（变球态））
- weapon：武器系统（射击/导弹）
- enemy：敌状态（dying/dead/hurt?）
- pickup：物品系统（HighJump/Varia/Gravity）
- ui：UI 系统（能量条/地图）
- sound：音频系统（SFX/音乐）
- game-data：数据表/资产解析（rom_tables（bank:addr（表值（表用途））））

## 2. 接口签名（tick/SpriteAllocator/EnemyAi/World/ApuFrame）

**接口签名**：
- tick()：帧管线核心（两相（NMI→tick() 两相+帧预算记账））
- SpriteAllocator：OAM DMA→SpriteAllocator（精灵分配）
- EnemyAi：敌 AI（敌状态特殊态表（CMP 引证））
- World：地图状态（房间/地形）
- ApuFrame：音频帧（SFX/音乐）

## 3. asm→Rust 对应关系（零页分层映射（暂存→局部（持久→struct 字段）））

**asm→Rust 对应关系**：
- 零页分层映射：暂存→局部（持久→struct 字段）
- A/X/Y/P+carry→域方法参数/返回值
- bank 切换→模块命名空间
- NMI→tick() 两相+帧预算记账
- OAM DMA→SpriteAllocator
- 软件 PPU 光栅器

## 4. refs 引用必写（bank:addr 引证（A 级原文））

**模块边界识别**（m1disasm 语义 banner 节（86 个（prg7_engine.asm（12930 行）））+constants_ram.asm 类型化 RAM 结构+帧管线权威顺序（bank7:$CB29 UpdateWorld（19 子系统）+NMI（bank7:$C0D9（vblank 相））））：
- bank7:$CB29 UpdateWorld（19 子系统）
- bank7:$C0D9（vblank 相）

## 5. 交付判据

设计文档实录（docs/design/phase0-module-division.md（模块 DAG+接口签名+asm→Rust 对应关系））——「候令」禁用。
