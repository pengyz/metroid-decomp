# 数据驱动架构重组：ROM 分离对齐（代码/数据分离）

> 2026-10-22 · 用户裁定（认可（数据驱动架构））· 纪律强化（refs 引用必写+更新留档可追溯）·
> 四项核（对抗复核核（证伪立场（独立重导证据）））· Evidence A 级（bank:addr 引证）Confidence 95

## 0. m1disasm 结构盘点（代码/数据分离辨（refs 实证））

| 类 | m1disasm 形 | refs 实证 |
|---|---|---|
| **代码侧** | prg0_title.asm…prg7_engine.asm（8 bank 代码）+ enemies/*.asm（16 敌 AI——代码藏数据目录形！）+ sound_engine.asm + areas_common.asm + reset.asm | prg7_engine.asm:33（RandomNumbers 07:C000）/:59（Startup 07:C01A）/:199（MainLoop 07:C0BC）；enemies/crawler.asm:2（`CrawlerAIRoutine_{AREA}` jsr/lda AI 例程=代码） |
| **数据侧** | common_sprite_data.asm（帧/动画表）+ 区域目录（brinstar/norfair/tourian/kraid/ridley/ending：rooms.asm/room_ptrs.asm/structure_ptrs.asm/metatiles.asm/palettes.asm/items.asm/enemy_sprite_data.asm + *.chr）+ songs/(ntsc/pal) + common_chr/ | common_sprite_data.asm:12（ObjectAnimIndexTbl $8572）/:282（ObjFramePtrTable $860B）；brinstar/rooms.asm（房间布局数据） |
| **元数据** | constants_ram.asm（RAM 结构布局=memmap 元数据）+ memory_layout.asm（bank 槽：slot3 $8000 切换/slot4 $C000 固定（:11-18））+ hardware.asm + linkfile + charmap.tbl | constants_ram.asm:125-150（EnExtra 结构）/950（Ens instanceof）；memory_layout.asm:15-18（bankstotal $8/banksize $4000） |

**边界辨（证伪立场）**：①enemies/=**代码**（AI 例程）非数据——目录形不可作判据，按内容辨
（jsr/lda 指令流=代码）；②common_sprite_data.asm 纯数据表（PtrTableEntry/.byte 帧）；
③constants_ram.asm=**映射元数据**（域结构定义输入），代码数据两侧共用。

## 1. 代码侧→game-core 对齐

| m1disasm | game-core | 说明 |
|---|---|---|
| prg7_engine.asm（引擎：NMI/MainLoop/UpdateWorld/Samus 族） | kernel/input/world/samus/weapon/enemy/pickup/ui 域方法 | 纯逻辑（无 I/O/确定性（虚拟时钟+GameRng）） |
| prg0-6 + areas_common.asm | world/rooms 逻辑（区域规则）+ enemy（EnemyMove $8058 族——common.rs 已折） | 逻辑归 core |
| enemies/*.asm（AI 例程） | enemy/ai 域方法（F2CA 族九入口素材） | AI=代码→core |
| sound_engine.asm | sound 域（7 簇 sound::sfx 已落） | 引擎逻辑→core |

## 2. 数据侧→game-data 对齐

| m1disasm | game-data | 说明 |
|---|---|---|
| common_sprite_data.asm（$8572/$860B 表族） | 帧/动画表直读（ObjFramePtrTable 106 条目/D 面考证件 113-126 系列） | ROM 直读，禁改写 |
| 区域目录 rooms/structures/metatiles/palettes/items | room/metatile/palette/item 资产解析 | 数据驱动主面 |
| songs/(ntsc/pal)+common_chr/ | 音频/CHR 资产 | S 面×registry 双射后随 |
| *.chr 图案表 | CHR-RAM 图案数据（渲染面输入） | L4 像素 diff 基 |

## 3. 对齐原则（数据驱动架构）

1. **代码只进 game-core**（纯逻辑/确定性），**数据只进 game-data**（ROM 直读/解析）——
   m1disasm 代码/数据分离形 = 架构两层红线的取证基础。
2. 数据表读取径=bank:addr 直读（bank7 固定 $C000；$8000 槽按 window_bank 索引
   （L2In.window_bank 契约——l2ports.rs））；**表值不进代码常量**（数据驱动）。
3. 域结构（EnemyState/Temp 面）= constants_ram.asm 元数据→Rust 域字段（零页二分表
   （phase0-rewrite-strategy.md §4））。
4. 每簇 port-map registry 双射（bank:addr→crate::module::fn（phase0-port-map-registry.md））
   ——代码/数据归宿各居其一（unported! 桩兜底）。

## 4. refs 引证汇总（bank:addr）

prg7_engine.asm:33/59/199（代码锚）；enemies/crawler.asm:2（AI=代码辨）；
common_sprite_data.asm:12/282（数据表锚 $8572/$860B）；constants_ram.asm:125-150/950
（结构元数据）；memory_layout.asm:11-18（bank 槽）；brinstar/rooms.asm（区域数据形）。

Evidence: 上述 bank:addr/行号引证（m1disasm SRC 实读）——A 级原文。
Confidence: 95
