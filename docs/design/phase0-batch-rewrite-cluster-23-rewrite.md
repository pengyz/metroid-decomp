# 批量改写·簇 23 文档+实现双补（用户裁定（认可（数据驱动架构）））

## 1. 六步流程（enemy::mellow（Mellow）（域内行为簇（文档+实现双补）））

### 1.1 定域取证 spec（Mellow 例程（实址（A 级原文）））

**定域取证 spec**（bank:addr（Mellow 例程（实址（A 级原文））））：
- bank07:EE86（SpawnMellow（prg7_engine.asm:9477（refs 原文（A 级原文））））
- bank07:EE63（SpawnMellows（prg7_engine.asm:9452））
- bank07:EE83（SpawnMellows_exit（prg7_engine.asm:9474））
- bank07:F149（CollisionDetectionMellow_CheckWithObjectYSlot（prg7_engine.asm:10082））
- bank07:F19A（GetMellowXSlotPosition（prg7_engine.asm:10127））
- bank07:F2B4（CollisionDetectionMellow_ReactToCollisionWithSamus（prg7_engine.asm:10372））
- bank07:F2BF（CollisionDetectionMellow_Hit（prg7_engine.asm:10381））
- bank07:F32A（CollisionDetectionMellow_ReactToCollisionWithSamusProjectile（prg7_engine.asm:10466））
- bank01:95E4（AreaMellowAnimIndex（prg1_brinstar.asm:152））
- bank02:95E4（AreaMellowAnimIndex（prg2_norfair.asm:150））
- bank03:95E4（AreaMellowAnimIndex（prg3_tourian.asm:159））
- bank04:95E4（AreaMellowAnimIndex（prg4_kraid.asm:157））
- bank05:95E4（AreaMellowAnimIndex（prg5_ridley.asm:158））

### 1.2 域结构（语义命名（data/naming 台账 A/B 级））

**域结构**：
- enemy::mellow（Mellow）
- 光栅器裁定：案 A（单列软件 PPU 光栅器（8-sprite/scanline 丢弃（D4 保真）））

### 1.3 memmap 适配（RAM 态→MellowState）

**memmap 适配**：
- 零页分层映射：暂存→局部（持久→struct 字段）
- bank 切换→模块命名空间

### 1.4 shim 写集适配（MellowState→RAM 写回）

**shim 写集适配**：
- verify_shim L2In（ram/regs）：已有（L2 态构造）

### 1.5 测试（穷举等价证明（先例））

**测试**：
- L2 案（每重组单元补 ≥1 L2 案（期望值来自 oracle 捕获（不手推）））

### 1.6 registry 登记+独立复审（对抗复核核（证伪立场（独立重导证据（正确性核））））

**registry 登记+独立复审**：
- registry 登记：l2ports.rs（registry）（bank07:EE6B（SpawnMellow（SC-S5-03）））
- 独立复审：对抗复核核（证伪立场（独立重导证据（正确性核）））

## 2. refs 引用必写（bank:addr 引证（A 级原文））

**refs 引用必写**（bank:addr 引证（A 级原文））：
- bank:addr（Mellow 例程（实址（A 级原文）））
- 光栅器裁定：案 A（单列软件 PPU 光栅器（8-sprite/scanline 丢弃（D4 保真）））

## 3. 交付判据

补全实录（docs/design/phase0-batch-rewrite-cluster-23-rewrite.md（六步流程（enemy::mellow（Mellow）（域内行为簇（文档+实现双补））））——「候令」禁用。
