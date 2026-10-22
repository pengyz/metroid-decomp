# Phase 0 架构设计·场景分层清单 v0（用户裁定（认可（按计划推进）））

## 1. 场景分层清单 v0（每域≥2 域内+跨域契约+端到端流程）

### 1.1 每域≥2 域内（L4 场景矩阵（跳跃/变球（功能验证）））

**域内场景**（每域≥2）：
- kernel：帧管线核心（tick 两相+帧预算记账）+NMI 同步
- input：控制器状态（按键序列（跳跃（A）/变球（下）））+输入处理
- render：渲染抽象（PPU 寄存器→渲染输出（256×240 像素））+像素 diff
- world：地图状态（房间/地形）+房间切换
- samus：主角状态（位移/速度/形态（变球态））+跳跃/变球
- weapon：武器系统（射击/导弹）+射击/导弹
- enemy：敌状态（dying/dead/hurt?）+敌 AI
- pickup：物品系统（HighJump/Varia/Gravity）+物品拾取
- ui：UI 系统（能量条/地图）+能量条/地图
- sound：音频系统（SFX/音乐）+SFX/音乐
- game-data：数据表/资产解析（rom_tables（bank:addr（表值（表用途））））+数据表读取

### 1.2 跨域契约（NMI 同步/共享 RAM/调用序）

**跨域契约**：
- NMI 同步：NMI→tick() 两相+帧预算记账
- 共享 RAM：constants_ram.asm 类型化 RAM 结构
- 调用序：bank7:$CB29 UpdateWorld（19 子系统（权威帧管线顺序））

### 1.3 端到端流程（boot→title→gameplay→gameover）

**端到端流程**：
- boot→title：ROM 初始化完（~120 帧（title 屏 init 完））
- title→gameplay：开始游戏（按键序列）
- gameplay→gameover：游戏结束（敌状态特殊态表）

## 2. refs 引用必写（bank:addr 引证（A 级原文））

**场景识别**（L4 场景矩阵（跳跃/变球（功能验证））+跨域契约（NMI 同步/共享 RAM/调用序）+端到端流程（boot→title→gameplay→gameover））：
- bank7:$CB29 UpdateWorld（19 子系统（权威帧管线顺序））
- bank7:$C0D9 NMI（vblank 相）

## 3. 交付判据

设计文档实录（docs/design/phase0-scenario-checklist.md（场景分层清单 v0（每域≥2 域内+跨域契约+端到端流程）））——「候令」禁用。
