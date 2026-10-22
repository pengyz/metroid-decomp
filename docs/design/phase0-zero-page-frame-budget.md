# Phase 0 架构设计：零页映射二分表 + 帧预算记账方案

> 2026-10-22（Phase 0 架构设计（用户裁定（认可（按计划推进））））
> 设计判据=零页映射二分表（暂存→函数局部/参数（禁 struct 字段（同址异义=调用域暂存））
> +持久→struct 字段按语义分组）+帧预算记账方案（周期常数自 Mesen trace 标定（NMI 掉帧复现
> （最大风险）））——refs 引用必写（bank:addr 引证（A 级原文））+留档可追溯（docs commit）。

## 1. 零页映射二分表（设计核心）

### 1.1 暂存区 $00-$11 → **函数局部/参数**（禁 struct 字段）

refs 实证（A 级原文（constants_ram.asm:252-286））：**同址异义**是暂存区本质——

| 零页址 | 同址异义实证（constants_ram.asm 行引） | Rust 映射 |
|---|---|---|
| $00 | Temp00_Diff（:252）/Temp00_Delta（areas_common.asm:903）/Temp00 清零标志（areas_common.asm:1002） | 函数局部（调用域暂存） |
| $01 | Temp01_DiffHi（:253） | 函数局部 |
| $02 | Temp02_ScrollDir（:270）/Temp02_PositionY（:278） | 函数局部（同址异义） |
| $03 | Temp03_ScrollDir（:254）/Temp03_PositionX（:279） | 函数局部（同址异义） |
| $04 | **Temp04_YSlotRadiusY（:255）/Temp04_SpeedY（:271）/Temp04_RoomRAMPtr（:280）**（三义同址） | 函数局部（调用域暂存） |
| $05 | Temp05_YSlotRadiusX（:256）/Temp05_SpeedX（:272）/Temp04_RoomRAMPtr+1（:281） | 函数局部（同址异义） |
| $06 | Temp06_YSlotPositionY（:257）/Temp06_PositionHi（:285） | 函数局部（同址异义） |
| $07 | Temp07_XSlotPositionY（:258） | 函数局部 |
| $08 | Temp08_YSlotPositionX（:259）/Temp08_PositionY（:273）/Temp08_RadiusY（:286） | 函数局部（同址异义） |
| $09 | Temp09_XSlotPositionX（:260）/Temp09_PositionX（:274） | 函数局部（同址异义） |
| $10 | Temp10_DistHi（:264） | 函数局部 |
| $11 | Temp11_DistY（:267） | 函数局部 |

**设计规则**：暂存区 **禁 struct 字段**（同址异义=调用域暂存（跨函数共享=调用约定（参数/
返回值）非全局状态））。每函数用**局部变量**承载 Temp 语义（编译器寄存器分配）。
**NMI 抢占约束**（A 级原文（constants_ram.asm:248））：「BUG! NMI routine overwrites Temp00
and Temp01 without restoring their value」——Rust 侧 NMI 处理与主循环**不得共享暂存局部**
（NMI 用独立栈帧（天然隔离）；但**保真期保留该 quirk**（D4（bug-ledger 登记）））。

### 1.2 持久区 $12+ → **struct 字段按语义分组**

refs 实证（A 级原文（constants_ram.asm:465-796））：

| 语义组 | 零页址 | struct 字段（建议） |
|---|---|---|
| **输入**（Joy*） | $12-$17 | `Input { joy1_change: u8, joy2_change: u8, joy1_status: u8, joy2_status: u8, joy1_retrig: u8, joy2_retrig: u8 }` |
| **NMI/主循环同步** | $1A | `NmiStatus: u8`（0=NMI in progress（:474）） |
| **游戏模式** | $1D-$1E | `GameMode: u8`（0=playing/1=title（:477））+ `MainRoutine: u8`（3=engine/5=paused（:478）） |
| **bank 切换** | $23 | `CurrentBank: u8`（0-7（:484）） |
| **定时器** | $29-$2C | `Timers { delay: u8, t1: u8, t2: u8, t3: u8 }`（t3=10 帧递减（:496）） |
| **随机数** | $2E-$2F | `GameRng { n1: u8, n2: u8 }`（确定性（GameRng（D1））） |
| **卷动** | $49/$FC/$FD | `Scroll { dir: u8, y: u8, x: u8 }` |

**分区 RAM**（A 级原文（constants_ram.asm:1164-1317））：RoomRAMA/B（$6000-$67FF（:1190-1191）/
WorldMapRAM（$7000-$73FF（:1298）/SamusData（$10*3（:1317））→独立 struct（房间缓冲/地图/
Samus 域）。

## 2. 帧预算记账方案

**周期常数自 Mesen trace 标定**（非估算（A 级实录（Mesen2 cycle profiler（NMI/主循环分账））））：

- **NTSC 总预算**：29780.5 CPU 周期/帧（1.789773MHz × 262 扫描线 ÷ 60.0988fps（近似））。
- **NMI 预算**：NMI handler 周期数（Mesen trace 标定（逐分支最大路径））——**NMI 掉帧复现
  （最大风险）**：NMI 超时→主循环被截断→帧不一致（L4 diff 直接红）。记账=**NMI 周期数
  上限**（trace 实测最大值 + 余量）。
- **主循环预算**：29780.5 − NMI 实测周期 − vblank 窗口损失（PPU 等待）。
- **记账实现**：game-core 虚拟时钟（D1（确定性））+每 tick 周期记账器（主循环/NMI 分账）
  （超预算=fail-loud（unported! 同形（bank:addr 定位）））。
- **标定径**：Mesen2 cycle profiler 跑典型场景（跳跃/变球（L4 场景））→NMI/主循环周期
  分布实录（docs/findings/（A 级实录））→常数入 game-core（cycle_budget 常量表）。

## 3. refs 引用（bank:addr 引证（A 级原文））

- 暂存区同址异义：constants_ram.asm:252-286（Temp00-Temp11（$00-$11））+areas_common.asm:903/
  1002（Temp00_Delta/Temp00 用法）。
- 持久区：constants_ram.asm:465-796（Joy1Change=$12…ScrollX=$FD）。
- NMI quirk：constants_ram.asm:248（BUG! NMI overwrites Temp00/Temp01（D4 保真））。
- 分区 RAM：constants_ram.asm:1164-1317（RoomRAMA/B/WorldMapRAM/SamusData）。
- 帧预算：NES NTSC 周期常数（29780.5/帧（社区公知））+Mesen trace 标定（待实录（docs/
  findings/（A 级实录）））。

## 4. 留档可追溯

docs commit（phase0-zero-page-frame-budget.md（本件））——Phase 0 架构设计 verdict（四项核
（A 级原文））留档 r7-porter-review。
