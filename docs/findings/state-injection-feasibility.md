# 状态注入可行性调研（L4 调研工单③——两侧恢复一致状态）

## 1. 状态注入可行性调研（一套游戏核心状态数据（RAM+CPU regs+PPU 态+mapper 态））

### 1.1 灌入 Mesen2（save-state 合成 or Lua API 注入）

**save-state 合成**（decomp save-state 机制调研（在产）照做）：
- save-state 合成：decomp save-state 机制调研（在产）照做

**Lua API 注入**（memory.read/write+emu.setRegister（已核（368b44e）））：
- memory.read/write：RAM 注入径（0000-07FF）
- emu.setRegister：CPU 寄存器注入径（a/x/y/p）

### 1.2 灌入 Rust game-core（初始态构造（game-core 状态结构（verify_shim L2In（ram/regs）（已有）→L4 态构造（+PPU 态？）））

**初始态构造**（game-core 状态结构（verify_shim L2In（ram/regs）（已有）→L4 态构造（+PPU 态？））：
- verify_shim L2In（ram/regs）：已有（L2 态构造）
- L4 态构造（+PPU 态？）：待调研（PPU 态/mapper 态构造）

## 2. 两侧恢复一致状态（成本调研（可行性+成本评估））

### 2.1 可行性+成本评估

**用户需求（理想情况（如果这个做不到，L4 对拍的成本就太高了））**：
- 可行性：save-state 合成 or Lua API 注入（Mesen2）+ 初始态构造（Rust game-core）
- 成本评估：save-state 合成（decomp save-state 机制调研（在产）照做）or Lua API 注入（已核（368b44e））

### 2.2 一致性验证径

**两侧恢复一致状态**：
- Mesen2：save-state 合成 or Lua API 注入（RAM+CPU regs+PPU 态+mapper 态）
- Rust game-core：初始态构造（verify_shim L2In（ram/regs）（已有）→L4 态构造（+PPU 态？））

## 3. 交付判据

调研文档（docs commit（状态注入可行性实录（两侧态构造实录+成本评估）））——「候令」禁用。
