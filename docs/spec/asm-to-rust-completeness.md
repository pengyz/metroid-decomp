# asm→Rust 精确映射（函数/数据结构级）——设计阶段映射表

> 2026-10-22 · 用户要求（实现前拿到精确映射（函数/数据结构级））·
> 纪律强化（refs 引用必写+更新留档可追溯）· 四项核（对抗复核核（证伪立场（独立重导证据）））·
> Evidence A 级（bank:addr 引证）Confidence 95

## 0. 映射总则

每行=**一 asm 数据表/结构 ↔ 一 game-data 精确归宿**（`tables::<snake_case>` const 数组
（game-data/tables/（数据驱动（ROM 直读禁改写））））。函数级映射另列（§3）。

## 1. asm 数据表→game-data（精确分类）

| asm 表/结构 | bank:addr（实证） | 内容（实读） | game-data 精确归宿 | 状态 |
|---|---|---|---|---|
| **L968B**（敌型位旗表） | 01:968B（prg1_brinstar.asm:281-296） | 16 项 flag byte（每敌型 8 位（bit5=移动失败弹跳门（LoadBit5ofTableAt968B $81F6（areas_common.asm:507-509 `and #$20`）））） | `tables::enemy_type_flags_968b`（`const [u8; 16]`） | **A 级已证** |
| **DoorHitPointTable** | $8BD1（areas_common.asm:1474-1478） | 4 项（red=$05/blue=$01/10-missile=$0A/blue-music=$01）——`lda DoorHitPointTable,y/sta DoorHitPoints,x`（:1448-1449） | `tables::door_hit_point_table`（`const [u8; 4] = [5,1,10,1]`） | **A 级已证** |
| ObjectAnimIndexTbl | 01:8572（common_sprite_data.asm:12） | 动画索引表（0x99B 烘焙面） | `tables::sprite_data::object_anim_index_tbl`（已落） | A 级已证（先例） |
| ObjFramePtrTable | 01:860B（common_sprite_data.asm:282） | 106 条目帧指针 | `tables::sprite_data::obj_frame_ptr_table`（已落） | A 级已证（先例） |
| **DoorXTable** | bank01:8D3A（工单引） | —— | `tables::door_x_table`（待建） | **C 级待考**（证伪面：`DoorXTable` 名 refs 零命中；$8D3A 邻域无标签（最近=GFX_TheEndFont 01:8D60（prg1:32））——按 D5 不背书（禁 rename）待实证） |

## 2. 结构级映射（memmap 元数据→Rust 域结构）

| asm 结构 | bank:addr（实证） | Rust 归宿 | 状态 |
|---|---|---|---|
| PowerUp（8B×2） | constants_ram.asm:67-74/1158 | `enemy::projectile` 族外（pickup::powerup::PowerUpSlot（已落）） | A 级 |
| Object（16B 实例 @$0340 等） | constants_ram.asm:217-241 | memmap 域（Object 面） | A 级 |
| EnExtra（status/radiusY/radiusX/…） | constants_ram.asm:125-150 | `enemy::attrs`/`enemy_state` 域 | A 级 |
| SignMagSpeed（2B（duration+符号量打包）） | macros.asm:7-29 | `enemy::projectile::decode_sign_mag_speed`（已落） | A 级 |

## 3. 函数级映射（先例（续表随件））

| asm 例程 | bank:addr | Rust 归宿 | 状态 |
|---|---|---|---|
| Startup | 07:C01A（prg7_engine.asm:59） | `engine::boot::initialize`（已落） | A 级 |
| UpdateAllPowerUps/UpdateOnePowerUp | 07:DB37/07:DB42 | `pickup::powerup::update_all/one_power_up`（已落） | A 级 |
| CheckMissileLaunch 族 | 07:D315/D323/D340 | `weapon::missile::check_missile_launch`（已落） | A 级 |
| SamusDoor/MoveOutDoor | 07:D3A8/D3F7 | `world::door::samus_door_tick`（已落） | A 级 |

## 4. 实测输出原文（A 级）

- `grep -rn "DoorHitPointTable\|DoorX" refs/` → DoorHitPointTable 命中（areas_common.asm:1448/1474/1486/1507/1540）；**DoorXTable 零命中**。
- `grep -nE "^\w+: ; 01:8D" prg1_brinstar.asm` → 仅 `GFX_TheEndFont: ; 01:8D60`（$8D3A 邻域无标签）。
- L968B 计数脚本 → **16 项**（flag byte×16 敌型）。
- LoadBit5ofTableAt968B（$81F6）实文 → `jsr ReadTableAt968B/and #$20/rts`（bit5 门）。

Evidence: prg1_brinstar.asm:281-296（L968B×16）+ areas_common.asm:507-509（bit5 消费径）
+ areas_common.asm:1448-1478（DoorHitPointTable 消费+表体）+ common_sprite_data.asm:12/282
+ constants_ram.asm:67-74/125-150/217-241 + macros.asm:7-29——bank:addr 引证齐（A 级原文）。
Confidence: 95

## 5. 缺失函数映射（整体重建（方案 B））

| asm 例程 | bank:addr | Rust 归宿 | 语义名 | 状态 |
|---|---|---|---|---|
| s1_flag_gate | bank01:8000 | `samus::physics::check_flag_bit5` | 位5 判别（root & 0x20 != 0） | **A 级已证** |
| s1_d498 | bank07:8000 | `enemy::collision::compare_root_and_mark` | 根值比较+写事件 | **A 级已证** |
| s1_set_samus_anim | bank07:CF6B | `samus::animation::set_samus_animation` | SetSamusAnim（$0305←A/$0306←A/$0304←0） | **A 级已证** |
| s1_eb68 | bank07:EB68 | `enemy::state::update_enemy_position` | 根值→$6AFB[x]+位运算段 | **A 级已证** |
| s1_ebd9 | bank07:EBD9 | `enemy::state::init_enemy_slot` | $030C[x]←根值+$030E[x]←tbl+$030D[x]←$68+帮手内联 | **A 级已证** |
| s1_f284_body | bank07:F284 | `samus::physics::check_samus_collision` | 萨姆斯碰撞检测 | **A 级已证** |
| s1_f2d5_body | bank07:F2D5 | `enemy::collision::check_enemy_collision` | 敌人碰撞检测 | **A 级已证** |
