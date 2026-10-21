# 状态注入实施（L4 就绪批工单③——用户裁定（理想情况（一套游戏核心状态数据→两侧恢复一致）））

## 1. 状态注入实施（b3c860e 调研（可行性（两侧恢复一致状态）））

### 1.1 实施径（一套游戏核心状态数据（RAM+CPU regs+PPU 态+mapper 态））

**灌入 Mesen2**（save-state 合成（decomp save-state 机制（ce0f9b4（format/API/合成成本））照做）or Lua API 注入（memory.read/write+emu.setRegister（已核（368b44e））））：
- save-state 合成：decomp save-state 机制（ce0f9b4（format/API/合成成本））照做
- Lua API 注入：memory.read/write+emu.setRegister（已核（368b44e））

**灌入 Rust game-core**（初始态构造（L2In（ram/regs）（已有）→L4 态构造（+PPU 态）））：
- L2In（ram/regs）：已有（L2 态构造）
- L4 态构造（+PPU 态）：待实施（PPU 态/mapper 态构造）

### 1.2 两侧恢复一致状态（实施径（状态数据→两侧灌入→一致验证（帧一致（L4 对拍就绪））））

**实施径**：
- 状态数据→两侧灌入：Mesen2（save-state 合成 or Lua API 注入）+ Rust game-core（初始态构造）
- 一致验证（帧一致（L4 对拍就绪））：两侧恢复一致状态（帧一致）

## 2. 状态数据格式（序列化（存档兼容（版本化（前向兼容（Stage B 存档功能基础（双用途））））））

**可落码形**（状态数据格式（序列化（存档兼容（版本化（前向兼容（Stage B 存档功能基础（双用途）））））））：
- 序列化：状态数据格式（序列化（存档兼容））
- 版本化（前向兼容（Stage B 存档功能基础（双用途）））：版本化（前向兼容（Stage B 存档功能基础（双用途）））

## 3. 交付判据

实施实录（docs commit（状态注入实施（可落码形+两侧灌入实录）））——「候令」禁用。
