# 去 NES 化可行性调研（用户裁定（架构优化调研（去 NES 化）——边界约束+正确性验证））

## 1. 现状分析（NES 架构概念携带（grep 实录））

### 1.1 NES 架构概念携带（grep 实录（crates/game-core/src/））

**memory 地址**（0x0400/0x6AF7 等（u16 地址字面））：
- 0x0400/0x6AF7 等：u16 地址字面（NES memory 地址）

**CPU 寄存器**（a/x/y/p（参数/返回值））：
- a/x/y/p：参数/返回值（NES CPU 寄存器）

**bank 切换**（MMC1（bank0-6（$8000 可切）+bank7（$C000 固定）））：
- bank0-6（$8000 可切）+bank7（$C000 固定）：MMC1 bank 切换

**OAM/PPU/NMI 时序**（帧级）：
- OAM/PPU/NMI 时序：帧级（NES 渲染时序）

### 1.2 可读性问题（代码形（NES 概念密布（抽象缺失）））

**可读性问题**：
- NES 概念密布（抽象缺失）：memory 地址/CPU 寄存器/bank 切换/OAM/PPU/NMI 时序

## 2. 去 NES 化可行性（抽象层设计（可落码形（Rust 类型（struct/enum/trait（抽象层）））））

### 2.1 抽象层设计（memory 地址→语义字段（struct（状态结构）））

**memory 地址→语义字段**（struct（状态结构（SamusState/enemy state（已有（7a2c5fc（SamusState 结构补（定义））））））：
- SamusState/enemy state：已有（7a2c5fc（SamusState 结构补（定义）））

### 2.2 CPU 寄存器→函数参数/返回值（a/x/y/p→函数签名（参数/返回值））

**CPU 寄存器→函数参数/返回值**：
- a/x/y/p→函数签名（参数/返回值）

### 2.3 bank 切换→模块组织（MMC1（bank0-6）→Rust 模块（domain 模块（engine/enemy/samus（已有（域模块））））

**bank 切换→模块组织**：
- MMC1（bank0-6）→Rust 模块（domain 模块（engine/enemy/samus（已有（域模块））））

### 2.4 OAM→sprite 系统（OAM 数据（精灵表）→sprite 系统（渲染抽象））

**OAM→sprite 系统**：
- OAM 数据（精灵表）→sprite 系统（渲染抽象）

### 2.5 PPU→渲染抽象（PPU 寄存器（渲染输出（256×240 像素））→渲染抽象（帧缓冲（像素输出）））

**PPU→渲染抽象**：
- PPU 寄存器（渲染输出（256×240 像素））→渲染抽象（帧缓冲（像素输出））

## 3. 边界约束（去 NES 化边界（什么必须保留 NES 特性？））

### 3.1 ROM 兼容（Stage A（保真期（NES_NTSC（D6（版本范围（保真期只做 NES_NTSC）））））

**ROM 兼容**：
- Stage A（保真期（NES_NTSC（D6（版本范围（保真期只做 NES_NTSC）））））

### 3.2 时序（帧级（60Hz tick（NES 帧时序（frame-based（虚拟时钟（GameRng（已有））））））

**时序**：
- 帧级（60Hz tick（NES 帧时序（frame-based（虚拟时钟（GameRng（已有））））））

### 3.3 渲染（像素输出（256×240（NES 原生分辨率（L4 像素锚点永存（用户确认）））））

**渲染**：
- 像素输出（256×240（NES 原生分辨率（L4 像素锚点永存（用户确认））））

### 3.4 输入处理（控制器状态（NES 手柄（按键序列（场景 DSL（input sequence（跳跃（A）/变球（下））））））

**输入处理**：
- 控制器状态（NES 手柄（按键序列（场景 DSL（input sequence（跳跃（A）/变球（下））））））

### 3.5 存档兼容（状态数据格式（双用途（L4 测试（状态注入（febb9a8（两侧恢复一致）））+Stage B 存档功能（类零点任务（房间存档）（版本化（前向兼容）））））

**存档兼容**：
- 状态数据格式（双用途（L4 测试（状态注入（febb9a8（两侧恢复一致）））+Stage B 存档功能（类零点任务（房间存档）（版本化（前向兼容）））））

### 3.6 边界约束清单（必须保留 NES 特性 vs 可抽象）

**必须保留 NES 特性**：
- ROM 兼容+时序+渲染+输入+存档

**可抽象**：
- memory 地址（语义字段）+CPU 寄存器（函数参数/返回值）+bank 切换（模块组织）+OAM（sprite 系统）

## 4. 正确性验证（去 NES 化改写后如何验证正确性？）

### 4.1 L4 帧对拍（架构无关（像素 diff（白名单外全等（容差=0）））

**L4 帧对拍**：
- 架构无关（像素 diff（白名单外全等（容差=0）））
- **像素锚点永存**（用户确认（L4 判据（架构无关（Stage B 重写后判据不变））））

### 4.2 场景 DSL（功能测试（跳跃/变球（功能验证（a97264b（state setup+input sequence+frame capture+comparison）））））

**场景 DSL**：
- 功能测试（跳跃/变球（功能验证（a97264b（state setup+input sequence+frame capture+comparison））））

### 4.3 回归（L2（fixture 级（冻结（Stage B 桥（缓）））+L4（帧级（架构无关）））

**回归**：
- L2（fixture 级（冻结（Stage B 桥（缓）））+L4（帧级（架构无关）））

### 4.4 特征化测试重投影（L2 判据（NES 写序列）→特征化测试（架构无关契约层（用户确认（L2 重构存续问答））））

**特征化测试重投影**：
- L2 判据（NES 写序列）→特征化测试（架构无关契约层（用户确认（L2 重构存续问答）））

## 5. 交付判据

调研文档（docs commit（去 NES 化可行性调研（四面（现状分析+可行性（抽象层设计（可落码形））+边界约束（清单）+正确性验证（L4 帧对拍（像素锚点永存）+场景 DSL+回归）））+**refs 引用**（bank:addr 引证（NES 架构概念（A 级原文）））+**留档可追溯**（docs commit（追溯径））））——「候令」禁用。
