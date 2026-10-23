# engine/mod.rs 重设计（函数归属+语义名+逻辑合并）

> 一次性到位（函数归属+语义名+逻辑合并）——非 asm 函数翻译（语义化改写+结构性改造）。

## 1. 设计原则

| 原则 | 说明 |
|---|---|
| **语义化命名** | 禁止 _ext/_body 等伪语义后缀 |
| **逻辑合并** | 多个 asm 标签→一个 Rust 函数（完整逻辑） |
| **模块归属** | 按领域归属（samus/enemy/world/sound/title/render） |
| **Rust 惯例** | 参数/返回值/错误处理（按 Rust 惯例组织） |

## 2. 函数归属+语义名+逻辑合并（257 个函数）

### 2.1 samus 域（萨姆斯）

| asm 地址 | 原名 | 语义名 | 归属 | 合并 |
|---|---|---|---|---|
| cd6b | s1_cd6b | set_samus_animation | samus::animation | 独立 |
| d012 | s1_d012_body | check_samus_input | samus::handler | 合并 |
| d015 | s1_d015 | check_samus_horizontal_movement | samus::handler | 合并 |
| d018 | s1_d018 | check_samus_vertical_movement | samus::handler | 合并 |
| d02b | s1_d02b | update_samus_position | samus::physics | 合并 |
| d036 | s1_d036 | check_samus_collision | samus::physics | 合并 |
| d039 | s1_d039 | update_samus_velocity | samus::physics | 合并 |
| d048 | s1_d048 | apply_samus_gravity | samus::physics | 合并 |
| d04d | s1_d04d_body | update_samus_state | samus::physics | 合并 |
| d07a | s1_d07a_body | select_samus_animation | samus::animation | 合并 |
| d088 | s1_d088_body | set_samus_animation_full | samus::animation | 合并 |
| d0d8 | s1_d0d8 | check_samus_jump | samus::handler | 合并 |
| d0fa | s1_d0fa | update_samus_jump | samus::physics | 合并 |
| d101 | s1_d101 | apply_samus_jump | samus::physics | 合并 |
| d10a | s1_d10a_body | update_samus_fall | samus::physics | 合并 |
| d10d | s1_d10d | check_samus_landing | samus::physics | 合并 |
| d110 | s1_d110_body | apply_samus_landing | samus::physics | 合并 |
| d116 | s1_d116_body | update_samus_standing | samus::physics | 合并 |
| d120 | s1_d120 | check_samus_running | samus::handler | 合并 |
| d12b | s1_d12b_body | update_samus_running | samus::physics | 合并 |
| d130 | s1_d130 | apply_samus_running | samus::physics | 合并 |
| d133 | s1_d133 | check_samus_skidding | samus::handler | 合并 |
| d136 | s1_d136 | update_samus_skidding | samus::physics | 合并 |
| d13f | s1_d13f | apply_samus_skidding | samus::physics | 合并 |
| d194 | s1_d194 | check_samus_damage | samus::physics | 合并 |
| d1a9 | s1_d1a9 | update_samus_damage | samus::physics | 合并 |
| d1c0 | s1_d1c0 | apply_samus_damage | samus::physics | 合并 |

### 2.2 enemy 域（敌人）

| asm 地址 | 原名 | 语义名 | 归属 | 合并 |
|---|---|---|---|---|
| d3ba | s1_d3ba | update_enemy_position | enemy::state | 合并 |
| d3bd | s1_d3bd | update_enemy_velocity | enemy::movement | 合并 |
| d3c0 | s1_d3c0 | apply_enemy_gravity | enemy::movement | 合并 |
| ... | ... | ... | ... | ... |

### 2.3 world 域（世界）

| asm 地址 | 原名 | 语义名 | 归属 | 合并 |
|---|---|---|---|---|
| d824 | s1_d824 | elevator_scroll_x_to_center | world::elevator | 合并 |
| d837 | s1_d837 | elevator_scroll_right | world::elevator | 合并 |
| d83a | s1_d83a_body | elevator_move | world::elevator | 合并 |
| d86d | s1_d86d_body | elevator_move_end_if_d | world::elevator | 合并 |
| d892 | s1_d892 | elevator_scroll_room | world::elevator | 合并 |
| d89a | s1_d89a_body | elevator_move_end_if_a | world::elevator | 合并 |
| d8a0 | s1_d8a0_body | elevator_move_end_if_b | world::elevator | 合并 |
| d8b7 | s1_d8b7_body | elevator_move_end_if_c | world::elevator | 合并 |
| d970 | s1_d970_body | update_enemy_state | enemy::state | 合并 |

## 3. 逻辑合并规则

| 规则 | 说明 | 示例 |
|---|---|---|
| **地址连续** | asm 地址连续→合并为一个函数 | d824+d837+d83a→elevator_move |
| **逻辑相关** | 功能相关→合并为一个函数 | d012+d015+d018→check_samus_input |
| **参数共享** | 共享参数→合并为一个函数 | d07a+d088→select_samus_animation |

## 4. 实施计划

| 阶段 | 内容 | 优先级 |
|---|---|---|
| **Phase 1** | samus 域（27 个函数（合并为 10 个）） | 高 |
| **Phase 2** | enemy 域（50 个函数（合并为 15 个）） | 高 |
| **Phase 3** | world 域（30 个函数（合并为 10 个）） | 中 |
| **Phase 4** | sound/title/render 域（150 个函数（合并为 50 个）） | 中 |
