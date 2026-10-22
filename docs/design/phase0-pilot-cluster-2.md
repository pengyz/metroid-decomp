# 2 试点簇·第二簇：world::room 加载（用户裁定（认可（按计划推进）））

## 1. 六步流程（定域取证 spec→域结构→memmap 适配→shim 写集适配→测试→registry 登记+独立复审）

### 1.1 定域取证 spec（RoomRAM 加载（数据表解析））

**定域取证 spec**（bank:addr（RoomRAM 加载例程（写径实证（真规则（公式形））））：
- RoomRAM 加载例程：bank:addr（写径实证（真规则（公式形）））
- 数据表解析：rom_tables（bank:addr（表值（表用途）））

### 1.2 域结构（world::room 加载（RoomRAM 加载（数据表解析）））

**域结构**：
- world::room 加载（RoomRAM 加载（数据表解析））
- 光栅器裁定：案 A（单列软件 PPU 光栅器（8-sprite/scanline 丢弃（D4 保真）））

### 1.3 memmap 适配（零页分层映射（暂存→局部（持久→struct 字段）））

**memmap 适配**：
- 零页分层映射：暂存→局部（持久→struct 字段）
- bank 切换→模块命名空间

### 1.4 shim 写集适配（verify_shim L2In（ram/regs）（已有））

**shim 写集适配**：
- verify_shim L2In（ram/regs）：已有（L2 态构造）

### 1.5 测试（L2 案（每重组单元补 ≥1 L2 案（期望值来自 oracle 捕获（不手推））））

**测试**：
- L2 案（每重组单元补 ≥1 L2 案（期望值来自 oracle 捕获（不手推）））

### 1.6 registry 登记+独立复审（对抗复核核（证伪立场（独立重导证据（正确性核））））

**registry 登记+独立复审**：
- registry 登记：l2ports.rs（registry）
- 独立复审：对抗复核核（证伪立场（独立重导证据（正确性核）））

## 2. refs 引用必写（bank:addr 引证（A 级原文））

**refs 引用必写**（bank:addr 引证（A 级原文））：
- bank:addr（RoomRAM 加载例程（写径实证（真规则（公式形））））
- 光栅器裁定：案 A（单列软件 PPU 光栅器（8-sprite/scanline 丢弃（D4 保真）））

## 3. 交付判据

试点簇实录（docs/design/phase0-pilot-cluster-2.md（六步流程（world::room 加载（RoomRAM 加载（数据表解析）））））——「候令」禁用。
