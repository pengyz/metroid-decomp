# Mesen2 save-state 机制调研（L4 调研工单②——状态注入可行性）

> 【2026-10-21 调研】用户裁定（L2 冻结+转 L4）——瞬时存档/恢复能力调研（供状态注入可行性（理想（一套游戏核心状态数据→灌入 Mesen2）））。

## format（save-state 文件格式）

- **Mesen2 save-state**（.mss?（RAM+CPU regs+PPU 态+mapper 态））——结构（hexdump 核待（文件格式实证））。
- 成分：RAM（0000-07FF）+CPU regs（A/X/Y/P/SP/PC）+PPU 态（VRAM/OAM/寄存器）+mapper 态（MMC1 寄存器/bank 切换）。

## Lua API（state 读写径）

- `emu.createSavestate`（save-state 创建（slot 读写））
- `emu.getState`（state 读（CPU/PPU/RAM 态））
- `emu.loadSavestate`（state 写（state 注入径））
- **state 注入径**：`emu.getState`（读态）→修改→`emu.setState`（写态）——程序化注入。

## 合成成本（state 结构→save-state 文件（字节级合成））

- **可行性**（state 数据→save-state 文件合成）：state 结构（RAM+CPU regs+PPU 态+mapper 态）→.mss 文件（字节级合成）——**待核**（文件格式实证（hexdump）+字段偏移表）。
- **成本评估**：state 结构→.mss 合成（中等（字段偏移表+CRC/校验（如有）））——程序化构建可行（待格式实证）。

## 供状态注入可行性（用户需求（理想（一套游戏核心状态数据→灌入 Mesen2）））

- **径**：游戏核心状态（Rust game-core state）→Mesen2 save-state（.mss 合成）→`emu.loadSavestate`（灌入）。
- **判据**：save-state 格式实证（hexdump）+字段偏移表→合成可行性（字节级）。
