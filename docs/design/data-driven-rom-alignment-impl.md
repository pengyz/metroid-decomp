# 数据驱动架构实施：ROM 分离对齐实施实录

> 2026-10-22 · 用户裁定（认可（数据驱动架构））· 纪律强化（refs 引用必写+更新留档可追溯）·
> 四项核（对抗复核核（证伪立场（独立重导证据）））· Evidence A 级（bank:addr 引证）Confidence 95

## 0. ROM 分离识别重导（refs 实证）

代码侧：prg0_title…prg7_engine（8 bank 代码）+enemies/*.asm（AI 例程——内容辨：
`CrawlerAIRoutine_{AREA}` jsr/lda 指令流（enemies/crawler.asm:2）=代码藏数据目录形）
+sound_engine/areas_common/reset。数据侧：common_sprite_data.asm（ObjectAnimIndexTbl
$8572（:12）/ObjFramePtrTable $860B（:282））+区域目录（rooms/structures/metatiles/
palettes/items/*.chr）+songs/。元数据：constants_ram.asm（结构布局）+memory_layout.asm
（bank 槽（:11-18））。

## 1. 实施对齐（四原则逐项核）

| 原则 | 实施态 | 实录 |
|---|---|---|
| ①代码只进 game-core | **✓** | 7 簇批量改写+enemy::movement（第三簇）落 core 域（enemy/sound/ui/weapon 等——pure logic 无 I/O）；enemies/*.asm AI=enemy 域方法素材（F2CA 族九入口） |
| ②数据只进 game-data（ROM 直读禁改写） | **✓** | rom_tables.rs（ROM 直读径）+tables/sprite_data.rs（ObjectAnimIndexTbl（bank01:8572）/ObjFramePtrTable（bank01:860B）typed const（写径实证））+tables/enemy_data.rs+room_data/（room_ram.rs）+constants/ram_layout.rs（结构元数据） |
| ③表值不进代码常量 | **更正注** | 权威面=game-data/tables（typed const+bank:addr 锚）；**对齐项**：game-core/src/samus/physics.rs:419 存在 ObjectAnimIndexTbl 烘焙面（$8572 起 0x99B——bank1-5 镜面注）——待收敛至 game-data 直读（原地记录不删原文（证伪更正文化）） |
| ④registry 双射归宿 | **✓** | l2ports.rs lookup 373 行 `sub_` 注册（bank:addr→crate::module::fn）+`unported!` 桩兜底（bank:addr 爆点）；port_coverage.py 双射核设计（phase0-port-map-registry.md） |

## 2. 边界辨（证伪立场·独立重导）

- enemies/ 目录形=数据区但内容=**代码**（jsr/lda 指令流）——按内容辨归 core（非按目录）。
- common_sprite_data.asm=纯数据表（PtrTableEntry/.byte 帧）——归 game-data。
- constants_ram.asm=**映射元数据**（域结构输入）——core 域结构+game-data ram_layout 共用（constants/ram_layout.rs）。

## 3. refs 引证汇总（bank:addr）

prg7_engine.asm:33/59/199（代码锚）；enemies/crawler.asm:2（AI=代码辨）；
common_sprite_data.asm:12（ObjectAnimIndexTbl $8572）/282（ObjFramePtrTable $860B）；
memory_layout.asm:11-18（bank 槽）；game-data/tables/sprite_data.rs:3/5/11（typed const 锚）；
game-core/src/samus/physics.rs:419（烘焙面对齐项）；l2ports.rs（373 registry 行）。

Evidence: 上述 bank:addr/行号引证（refs 实读+crates 实读）——A 级原文。
Confidence: 95
