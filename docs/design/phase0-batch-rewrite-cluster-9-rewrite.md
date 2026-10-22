# 批量改写·簇 9 文档补全（用户裁定（认可（数据驱动架构）））

## 1. 六步流程（ending::sequence（结局动画）（域内行为簇））

### 1.1 定域取证 spec（结局动画例程（实址（A 级原文）））

**定域取证 spec**（bank:addr（结局动画例程（实址（A 级原文）））：
- 结局动画例程：bank:addr（实址（A 级原文））
- 数据表解析：rom_tables（bank:addr（表值（表用途）））

### 1.2 域结构（语义命名（data/naming 台账 A/B 级））

**域结构**：
- ending::sequence（结局动画）
- 光栅器裁定：案 A（单列软件 PPU 光栅器（8-sprite/scanline 丢弃（D4 保真）））

### 1.3 memmap 适配（RAM 态→EndingState）

**memmap 适配**：
- 零页分层映射：暂存→局部（持久→struct 字段）
- bank 切换→模块命名空间

### 1.4 shim 写集适配（EndingState→RAM 写回）

**shim 写集适配**：
- verify_shim L2In（ram/regs）：已有（L2 态构造）

### 1.5 测试（穷举等价证明（先例））

**测试**：
- L2 案（每重组单元补 ≥1 L2 案（期望值来自 oracle 捕获（不手推）））

### 1.6 registry 登记+独立复审（对抗复核核（证伪立场（独立重导证据（正确性核））））

**registry 登记+独立复审**：
- registry 登记：l2ports.rs（registry）
- 独立复审：对抗复核核（证伪立场（独立重导证据（正确性核）））

## 2. refs 引用必写（bank:addr 引证（A 级原文））

**refs 引用必写**（bank:addr 引证（A 级原文））：
- bank:addr（结局动画例程（实址（A 级原文）））
- 光栅器裁定：案 A（单列软件 PPU 光栅器（8-sprite/scanline 丢弃（D4 保真）））

## 3. 交付判据

补全文档实录（docs/design/phase0-batch-rewrite-cluster-9-rewrite.md（六步流程（ending::sequence（结局动画）（域内行为簇））））——「候令」禁用。
