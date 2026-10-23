# engine/mod.rs 语义重构方案（逻辑集合+功能归属）

> 非 asm 原样翻译（语义分析+逻辑集合+Rust 表达+功能归属）。

## 1. 设计原则

| 原则 | 说明 |
|---|---|
| **语义分析** | 理解 asm 逻辑（非机械翻译） |
| **逻辑集合** | 相关函数→完整逻辑单元（Rust 函数） |
| **Rust 表达** | 符合 Rust 惯例（参数/返回值/错误处理） |
| **功能归属** | 按 Rust 功能模块划分（samus/enemy/world） |

## 2. 逻辑集合方案（258 个函数→30 个逻辑单元）

### 2.1 samus 域（萨姆斯（80 个函数→10 个逻辑单元））

| asm 地址 | 语义 | 逻辑单元 | 归属 | 合并 |
|---|---|---|---|---|
| cd6b+cf6b | SetSamusData_3FrameAnimDelay+SetSamusAnim | **set_samus_animation** | samus::animation | 合并 |
| d012+d015+d018 | 输入检测（水平/垂直/跳跃） | **check_samus_input** | samus::handler | 合并 |
| d02b+d036+d039 | 位置更新+碰撞检测+速度应用 | **update_samus_position** | samus::physics | 合并 |
| d048+d04d | 重力应用+状态更新 | **apply_samus_gravity** | samus::physics | 合并 |
| d07a+d088 | 动画选择+动画设置 | **select_samus_animation** | samus::animation | 合并 |
| d0d8+d0fa+d101 | 跳跃检测+跳跃速度+跳跃应用 | **update_samus_jump** | samus::physics | 合并 |
| d10a+d10d+d110 | 着陆检测+着陆速度+下落速度 | **update_samus_landing** | samus::physics | 合并 |
| d116+d120+d12b | 跑步检测+跑步速度+速度限制 | **update_samus_running** | samus::physics | 合并 |
| d130+d133+d136 | 滑行检测+滑行速度+摩擦力 | **update_samus_skidding** | samus::physics | 合并 |
| d13f+d194+d1a9+d1c0 | 伤害检测+伤害计算+生命值检查+死亡处理 | **update_samus_damage** | samus::physics | 合并 |

### 2.2 enemy 域（敌人（100 个函数→10 个逻辑单元））

| asm 地址 | 语义 | 逻辑单元 | 归属 | 合并 |
|---|---|---|---|---|
| d3ba+d3bd+d3c0 | 位置更新+速度应用+边界检查 | **update_enemy_position** | enemy::state | 合并 |
| d3f4+d400+d40e | 速度更新+速度应用+速度限制 | **update_enemy_velocity** | enemy::movement | 合并 |
| d447+d465+d488 | 重力应用+重力检测+重力限制 | **apply_enemy_gravity** | enemy::movement | 合并 |
| d57a+d580+d609 | 状态检测+状态更新+状态重置 | **update_enemy_state** | enemy::state | 合并 |
| d633+d694+d6a7 | 移动检测+移动应用+移动限制 | **update_enemy_movement** | enemy::movement | 合并 |
| d6bf+d6e1+d6ed | 碰撞检测+碰撞计算+碰撞判断 | **check_enemy_collision** | enemy::collision | 合并 |
| d70f+d71a+d735 | 动画检测+动画更新+动画重置 | **update_enemy_animation** | enemy::state | 合并 |
| d746+d761 | 生命值检测+生命值更新 | **update_enemy_health** | enemy::state | 合并 |
| eb68+ebd9 | 位置更新+槽初始化 | **init_enemy_slot** | enemy::state | 合并 |
| ec89+ee33 | 动画更新+移动更新 | **update_enemy_animation_movement** | enemy::movement | 合并 |

### 2.3 world 域（世界（30 个函数→5 个逻辑单元））

| asm 地址 | 语义 | 逻辑单元 | 归属 | 合并 |
|---|---|---|---|---|
| d824+d837+d83a | 电梯滚动 X 居中+右移+左移 | **elevator_scroll_control** | world::elevator | 合并 |
| d86d+d892+d89a | 电梯移动检测+滚动房间+移动结束条件 | **elevator_move_control** | world::elevator | 合并 |
| d8a0+d8b7+d8b4 | 电梯状态检测+移动结束条件+状态重置 | **elevator_state_control** | world::elevator | 合并 |
| d957+d8bc | 门交互检测+门交互判断 | **door_interaction** | world::door | 合并 |
| d970 | 敌人状态更新 | **update_enemy_state_ext** | enemy::state | 独立 |

### 2.4 sound/title/render 域（48 个函数→5 个逻辑单元）

| asm 地址 | 语义 | 逻辑单元 | 归属 | 合并 |
|---|---|---|---|---|
| f530+f598 | 音效播放+音效状态更新 | **play_sfx** | sound::sfx | 合并 |
| f589+f58b | 敌人表读取+表读取 | **read_enemy_table** | enemy::state | 合并 |
| f6bd+f6eb+f71a | 敌人音频检测+更新+应用 | **update_enemy_audio** | enemy::state | 合并 |
| f96a+f96d+f9f2 | 拾取检测+拾取处理+拾取应用 | **handle_pickup** | pickup::item | 合并 |
| f96d+f9f2 | 背景碰撞检测+碰撞计算+碰撞判断 | **check_enemy_bg_collision** | enemy::collision | 合并 |

## 3. 实施计划

| 阶段 | 内容 | 优先级 |
|---|---|---|
| **Phase 1** | samus 域（80 个函数（合并为 10 个）） | 高 |
| **Phase 2** | enemy 域（100 个函数（合并为 10 个）） | 高 |
| **Phase 3** | world 域（30 个函数（合并为 5 个）） | 中 |
| **Phase 4** | sound/title/render 域（48 个函数（合并为 5 个）） | 中 |

## 4. 复核清单

| 复核项 | 检查内容 | 状态 |
|---|---|---|
| **语义正确性** | 逻辑单元是否正确反映 asm 语义 | ⏳ 待复核 |
| **逻辑完整性** | 合并后是否覆盖所有 asm 逻辑 | ⏳ 待复核 |
| **Rust 表达** | 是否符合 Rust 惯例 | ⏳ 待复核 |
| **功能归属** | 是否按 Rust 功能模块划分 | ⏳ 待复核 |
