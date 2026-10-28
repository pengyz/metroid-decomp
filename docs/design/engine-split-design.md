# engine 巨石拆分设计（P2（lead 聚合裁定落形——待确认稿））

> 2026-10-28 · 设计先行（本稿经 lead 确认后开批 1（现存批 1-3=应急拆分（下述再组织面））。
> 裁定理由（lead 再强调）：**第二维=asm 例程族**（机器码是真相（Rust 自创语义分类随
> 理解漂移（禁））；**活代码/投影资产分离**（门禁约束不同（混居使「哪些逻辑在跑」不可
> 机械回答））。

## 0. 现存批 1-3 处置（待确认）

应急拆分已落（engine/samus_common.rs（53 fn）/enemy_common.rs（80）/door_room.rs（21）
——按 lead 早期三类聚合）。本设计=**再组织蓝图**：批 1-3 三件内的簇将按第二维
（一族一文件）二次拆出（机械移动（守恒四维（下述清单）））；或 lead 裁定保留粗聚形态
——请裁定。

## 1. 三性分类盘点表

| 类 | 定义 | 盘点（实证） | 去向 |
|---|---|---|---|
| **A 活代码** | 运行链真实调用 | **8 文件/7 实点名**：frame/room.rs:171（find_item_in_history（门开 item 判））/:179/:705（get_name_table_at_scroll_dir（名表选））；frame/mod.rs:37（FrameEngine re-export）；l2ports.rs:1514/1524/1537（region_slices::draw_frame/wait_nmi_pass/nmi::nmi_handler（**G4 裁定标注：占位公式·非行为源**（桥面）））；samus/physics.rs:1459+samus/handler.rs:989（check_enemy_collision/check_samus_collision 族（**测试引用**））；verify_shim/mod.rs:965/1842（s1_d498/RUN_ACCEL_TBL（**投影消费**）） | **归域模块**（一域一模块（文件名=域语义名）） |
| **B 投影资产** | 写事件投影层（等价对拍/verify_shim 消费（不入运行链）） | 其余 ~1090 fn（SC-S1..S7 簇+Phase-2/4 语义件（写面 (Vec<(u16,u8)>,u8) 形）） | **留 engine**（一族一文件（文件名=首例程语义名（asm 例程族同构））） |
| **C 已取代** | 域真源存在（本件=转发/双写残留） | 2 件实证：set_samus_animation/door_interaction（P1 薄转发（域真源=samus/animation.rs+world/door.rs）） | 转发保留或随 P1 后续清理批 |

## 2. 两级维度

1. **第一维（消费性质）**：A 活代码→域模块（samus/enemy/world/frame 域（按调用方归域））；
   B 投影资产→engine（等价层内聚）；C→转发面。
2. **第二维（asm 例程族）**：**一族=一文件**；文件名=**首例程语义名**（m1disasm 标签
   （snake 化）（如 SetSamusAnim 族→set_samus_anim.rs）；无标签（纯 addr 例程）→
   addr 形名（sub_d806.rs）（asm 真相形（禁 Rust 自创分类））。
   **组合**：目标文件=维度 1 去向 × 维度 2 族名（A 内亦一族一文件（域内）；B=engine/<族名>.rs）。

## 3. 1102 fn→目标文件完整映射表

（fn 数基准=本设计实测（pub fn（四件合计）1102（lead 1111 口径差=含 verify_shim/
非 pub 面（审计口径注）））。族聚=389（SC 簇头/Phase 段头）。**每 fn 均在表内**（族行
列全名（[A]/[B]/[C] 标注））。

| 目标文件 | asm 例程族（原头） | 现所在 | 成员（[类]） |
|---|---|---|---|
| engine/misc.rs | （无簇头（文件头段）） | mod | get_name_table_at_scroll_dir[A]、set_enemy_bit[B]、init_door_room[B]、update_enemy_short[B] |
| engine/misc.rs | SC-S1-00 残员 6 件（共享尾族深件） | mod | compare_root_and_mark[B]、update_enemy_position[B]、init_enemy_slot[B]、load_room_statues[B]、update_enemy_animation[B]、update_enemy_movement[B] |
| engine/sub_d806.rs | SC-S1-01 残面：$D806 驱动器活跃径 handlers（10 员族——jmp 回驱形） | mod | elevator_scroll_x_to_center[B]、elevator_scroll_right[B]、elevator_scroll_room[B] |
| engine/misc.rs | SC-S1-01 残员 6 handler（流式块形——入口=jmp 尾+体至回驱/尾跳） | mod | elevator_move[B]、elevator_move_end_if_d[B]、elevator_move_end_if_a[B]、elevator_move_end_if_b[B]、elevator_move_end_if_c[B]、update_enemy_state[B] |
| engine/sub_80b0.rs | SC-S1-02：$80B0 族（9 员——**跨 bank 镜面实证** banks1-5 共享码） | mod | s1_root_80b0[B]、s1_f676[B]、check_flag_bit5[B] |
| engine/sub_f3d8.rs | SC-S1-02 残 7 员（旗门 handler 族+$F3D8） | mod | s1_f3d8[B]、s1_f503[B]、s1_f55c[B]、update_enemy_movement_ext2[B]、update_enemy_movement_ext3[B]、update_enemy_movement_ext4[B]、update_enemy_movement_ext8[B] |
| engine/misc.rs | SC-S1-03 员转写（8 员——root PlaySnd3 族；一卡一闭面） | mod | update_enemy_movement_ext5[B]、update_enemy_movement_ext6[B]、update_enemy_root[B]、update_enemy_movement_ext7[B]、update_enemy_movement_ext9[B]、remove_enemy[B]、update_enemy_projectile[B]、s1_fbc7_body[B] |
| engine/playsnd3.rs | SC-S1-04 员转写（7 员——PlaySnd3 族二；L2 真 0 留白） | mod | s1_f3d5_body[B]、s1_f628_body[B]、s1_f682_body[B]、s1_f699_body[B]、s1_f7d9_body[B]、s1_f844_body[B]、s1_fb8a_body[B] |
| engine/xminus16_exit22.rs | SC-S1-07 员转写（6 员——Xminus16/Exit22 族） | mod | check_samus_collision[B]、check_enemy_collision[B]、set_enemy_touching_samus_flags[B]、check_enemy_hit_flags[B]、get_enemy_hit_flags[B]、check_enemy_collision_ext[B] |
| engine/table18.rs | SC-S1-10 员转写（5 员——Table18 族；L2 真 0 留白） | mod | s1_root_fd8f[B]、s1_d10a_body[B]、s1_d633_body[B]、s1_f924_body[B]、s1_fced_body[B] |
| engine/twoscompliment.rs | SC-S1-12 员转写（5 员——TwosCompliment 族） | mod | s1_twos_compliment[B]、s1_d57a_body[B]、s1_f7a7_body[B]、s1_f9d9_body[B]、s1_fb9c_body[B] |
| engine/table0d.rs | SC-S1-13 员转写（5 员——Table0D 族；L2 真 0 留白） | mod | s1_get_obj_coords[B]、s1_d580_body[B]、s1_d609_body[B]、s1_d694_body[B]、s1_d6a7_body[B] |
| engine/table0d.rs | SC-S1-14 员转写（5 员——Table0D 族二） | mod | s1_root_d78b[B]、s1_d6bf_body[B]、s1_d6e1_body[B]、s1_d71a_body[B]、s1_d761_body[B] |
| engine/misc.rs | SC-S1-15 员转写（5 员——LD77F 加法族） | mod | s1_root_d77f[B]、s1_d6ed_body[B]、s1_d70f_body[B]、s1_d735_body[B]、s1_d746_body[B] |
| engine/animdrawobject.rs | SC-S1-19 员转写（4 员——AnimDrawObject 族；L2 真 0 留白） | mod | s1_anim_draw_object[B]、s1_d488_body[B]、s1_d66d_body[B]、s1_d8bc_body[B] |
| engine/sub_d76a.rs | SC-S1-20 员转写（4 员——$D76A 表走族） | mod | s1_root_d76a[B]、s1_d6e4_body[B]、s1_d712_body[B]、s1_d73e_body[B] |
| engine/yplus16.rs | SC-S1-25 员转写（4 员——Yplus16 族；L2 真 0 留白） | mod | s1_yplus16_xminus16[B]、s1_f0cf_body[B]、s1_f112_body[B]、s1_f8f0_body[B] |
| engine/issamusdead.rs | SC-S1-26 员转写（4 员——IsSamusDead 族） | mod | s1_is_samus_dead[B]、s1_f0da_body[B]、s1_f101_body[B]、s1_f11b_body[B] |
| engine/checkcollision.rs | SC-S1-27 员转写（4 员——CheckCollision 族；L2 真 0 留白） | mod | s1_check_collision[B]、s1_f10c_body[B]、s1_f146_body[B]、s1_febd_body[B] |
| engine/getobjectyslotposition.rs | SC-S1-28 员转写（4 员——GetObjectYSlotPosition 族） | mod | s1_get_obj_y_slot[B]、s1_f120_body[B]、s1_f143_body[B]、s1_feaa_body[B] |
| engine/startmusic.rs | SC-S2-06（S2-36）员转写（3 员——StartMusic 族；L2 真 0 留白） | mod | s2_start_music[B]、s2_d3c8_body[B]、s2_d900_body[B] |
| engine/setmirrorcntrlbit.rs | SC-S2-08（S2-38）员转写（3 员——SetmirrorCntrlBit 族） | mod | s2_set_mirror_cntrl[B]、s2_d414_body[B]、s2_d483_body[B] |
| engine/setprojectileanim.rs | SC-S2-15（S2-45）员转写（3 员——SetProjectileAnim 族） | mod | s2_set_projectile_anim[B]、s2_d660_body[B]、s2_d685_body[B] |
| engine/sub_d66b.rs | SC-S2-16（S2-46）员转写（3 员——$D66B 动画步进族；L2 真 0 留白） | mod | s2_anim_step[B]、s2_d68e_body[B]、s2_d6a4_body[B] |
| engine/misc.rs | SC-S2-17（S2-47）员转写（3 员——索引 toggle 族） | mod | s2_index_toggle[B]、s2_d847_body[B]、s2_d85e_body[B] |
| engine/misc.rs | SC-S2-19（S2-49）员转写（3 员——SpawnElevator VM 族） | mod | s2_spawn_elevator[B]、s2_ec04_body[B]、s2_eea1_body[B] |
| engine/sfx_metroidhit.rs | SC-S3-00（S3-60）员转写（3 员——SFX_MetroidHit 族） | mod | s3_sfx_metroid_hit[B]、s3_f530_body[B]、s3_f598_body[B] |
| engine/readtableat968b.rs | SC-S3-01（S3-61）员转写（3 员——ReadTableAt968B 族；L2 真 0 留白） | mod | s3_read_table_968b[B]、s3_f589_body[B]、s3_f6bd_body[B] |
| engine/sfx_enemyhit.rs | SC-S3-02（S3-62）员转写（3 员——SFX_EnemyHit 族） | mod | s3_sfx_enemy_hit[B]、s3_f59d_body[B]、s3_f5a2_body[B] |
| engine/table04.rs | SC-S3-07（S3-67）员转写（2 员——Table04 族） | mod | s3_table04[B]、s3_d02b_body[B] |
| engine/sfx_samusball.rs | SC-S3-11（S3-71）员转写（2 员——SFX_SamusBall 族） | mod | s3_sfx_samus_ball[B]、s3_d0d8_body[B] |
| engine/sfx_bomblaunch.rs | SC-S3-17（S3-77）员转写（2 员——SFX_BombLaunch 族） | mod | s3_sfx_bomb_launch[B]、s3_d194_body[B] |
| engine/fireweapon.rs | SC-S3-18（S3-78）员转写（2 员——FireWeapon 族；L2 真 0 留白） | mod | s3_fire_weapon[B]、s3_d1c0_body[B] |
| engine/door_deleteoffscreenenemies.rs | SC-S3-20（S3-80）员转写（2 员——Door_DeleteOffscreenEnemies 族；L2 真 0 留白） | mod | s3_delete_offscreen[B]、s3_d3ba_body[B] |
| engine/doors_removeifoffscreen.rs | SC-S3-21（S3-81）员转写（2 员——Doors_RemoveIfOffScreen 族；L2 真 0 留白） | mod | s3_doors_remove[B]、s3_d3bd_body[B] |
| engine/rts.rs | SC-S3-22（S3-82）员转写（2 员——$95AB rts 桩阵族；L2 真 0 留白） | mod | s3_stub_array[B]、s3_d3c0_body[B] |
| engine/tourianmusic.rs | SC-S3-23（S3-83）员转写（2 员——TourianMusic 族） | mod | s3_tourian_music[B]、s3_d3dd_body[B] |
| engine/drawframe.rs | SC-S3-24（S3-84）员转写（2 员——DrawFrame 族） | mod | s3_draw_frame[B]、s3_d417_body[B] |
| engine/sub_d48f.rs | SC-S3-25（S3-85）员转写（2 员——$D48F 四写环族） | mod | s3_d4b4_body[B]、s3_d48f_body[B] |
| engine/misc.rs | SC-S3-26（S3-86）员转写（2 员——环体内件（S1-00/S3-25 环正身）；L2 真 0 留白） | mod | s3_ring_body[B]、s3_ring_loop[B] |
| engine/updatebullet_deleteifoffscreen.rs | SC-S3-27（S3-87）员转写（2 员——UpdateBullet_DeleteIfOffScreen 族；L2 真 0 留白） | mod | s3_bullet_on_screen[B]、s3_d530_body[B] |
| engine/table0d.rs | SC-S3-28（S3-88）员转写（2 员——Table0D 族二现；L2 真 0 留白） | mod | s3_table0d[B]、s3_d533_body[B] |
| engine/drawbullet.rs | SC-S3-29（S3-89）员转写（2 员——DrawBullet 族；L2 真 0 留白·S3 收官卡） | mod | s3_draw_bullet[B]、s3_d552_body[B] |
| engine/updatebullet_move.rs | SC-S4-00（S4-90）员转写（2 员——UpdateBullet_Move 族；L2 真 0 留白·S4 首卡） | mod | s4_update_bullet_move[B]、s4_d585_body[B] |
| engine/sub_d4f8.rs | SC-S4-01（S4-91）员转写（2 员——$D4F8 态机族；L2 真 0 留白） | mod | s4_d4f8_body[B]、s4_d588_body[B] |
| engine/sub_d4eb.rs | SC-S4-02（S4-92）员转写（2 员——$D4EB 链入口族；L2 真 0 留白） | mod | s4_d4eb_body[B]、s4_d5c5_body[B] |
| engine/animdraw.rs | SC-S4-03（S4-93）员转写（2 员——$D517 AnimDraw 接面族；L2 真 0 留白） | mod | s4_d517_body[B]、s4_d5d7_body[B] |
| engine/iswalkabletile.rs | SC-S4-04（S4-94）员转写（2 员——IsWalkableTile 族） | mod | s4_is_walkable[B]、s4_d606_body[B] |
| engine/sub_95c0.rs | SC-S4-05（S4-95）员转写（2 员——$95C0 桩+取负对；L2 真 0 留白） | mod | s95c0_negate[B]、s4_d614_body[B] |
| engine/sub_e9be.rs | SC-S4-06（S4-96）员转写（2 员——$E9BE 检查链族） | mod | s4_e9be_check[B]、s4_d621_body[B] |
| engine/sfx_bombexplode.rs | SC-S4-07（S4-97）员转写（2 员——SFX_BombExplode 族） | mod | s4_sfx_bomb_explode[B]、s4_d68b_body[B] |
| engine/isblasttile.rs | SC-S4-08（S4-98）员转写（2 员——IsBlastTile 族；L2 真 0 留白） | mod | s4_is_blast_tile[B]、s4_d779_body[B] |
| engine/sub_e6a7.rs | SC-S4-09（S4-99）员转写（2 员——$E6A7 态步进族） | mod | s4_e6a7_step[B]、s4_d831_body[B] |
| engine/sub_e6d2.rs | SC-S4-10（S4-100）员转写（2 员——$E6D2 态步进 inc 对偶族） | mod | s4_e6d2_step[B]、s4_d837_body[B] |
| engine/sub_d851.rs | SC-S4-11（S4-101）员转写（2 员——$D851 动画计数族） | mod | s4_d851_anim[B]、s4_d866_body[B] |
| engine/isenginerunning.rs | SC-S4-12（S4-102）员转写（2 员——IsEngineRunning 族） | mod | s4_is_engine_running[B]、s4_d8e7_body[B] |
| engine/waitnmipass.rs | SC-S4-13（S4-103）员转写（2 员——WaitNMIPass 族；L2 真 0 留白） | mod | s4_wait_nmi_pass[B]、s4_d8fa_body[B] |
| engine/selectsamuspal.rs | SC-S4-14（S4-104）员转写（2 员——SelectSamusPal 族） | mod | s4_select_samus_pal[B]、s4_d8fd_body[B] |
| engine/screenon.rs | SC-S4-15（S4-105）员转写（2 员——ScreenOn 族；L2 真 0 留白） | mod | s4_screen_on[B]、s4_d903_body[B] |
| engine/sub_c8b0.rs | SC-S4-16（S4-106）员转写（2 员——$C8B0 表拷贝族） | mod | s4_c8b0_copy[B]、s4_d906_body[B] |
| engine/sub_c8bb.rs | SC-S4-17（S4-107）员转写（2 员——$C8BB 清零环族；L2 真 0 留白） | mod | s4_c8bb_clear[B]、s4_d909_body[B] |
| engine/togglescroll.rs | SC-S4-18（S4-108）员转写（2 员——ToggleScroll 族） | mod | s2_toggle_scroll[B]、s4_d96b_body[B] |
| engine/step.rs | SC-S4-19（S4-109）员转写（2 员——$D892 step 分派族；L2 真 0 留白） | mod | s4_d892_dispatch[B]、s4_d973_body[B] |
| engine/enemycheckmovevertical.rs | SC-S4-20（S4-110）员转写（2 员——EnemyCheckMoveVertical 族） | mod | s4_enemy_move_vert[B]、s4_e778_body[B] |
| engine/checkmovedown.rs | SC-S4-21（S4-111）员转写（2 员——CheckMoveDown 族；L2 真 0 留白） | mod | s4_check_move_down[B]、s4_e78f_body[B] |
| engine/enemycheckmovehorizontal.rs | SC-S4-22（S4-112）员转写（2 员——EnemyCheckMoveHorizontal 族） | mod | s4_enemy_move_horiz[B]、s4_e8f9_body[B] |
| engine/checkmoveright.rs | SC-S4-23（S4-113）员转写（2 员——CheckMoveRight 族；L2 真 0 留白） | mod | s4_check_move_right[B]、s4_e90c_body[B] |
| engine/misc.rs | SC-S4-24（S4-114）员转写（2 员——LoadEnemy/GetEnemyData（S1-22 walk 表 | mod | s3_get_enemy_data[B]、s3_load_enemy[B] |
| engine/isslottaken.rs | SC-S4-25（S4-115）员转写（2 员——IsSlotTaken 族；L2 真 0 留白） | mod | s4_is_slot_taken[B]、s4_eb11_body[B] |
| engine/getenemytype.rs | SC-S4-26（S4-116）员转写（2 员——GetEnemyType 族；L2 真 0 留白） | mod | s4_get_enemy_type[B]、s4_eb19_body[B] |
| engine/misc.rs | SC-S4-27（S4-117）员转写（2 员——LEB4D 敌初始化族；L2 真 0 留白） | mod | s4_leb4d[B]、s4_eb20_body[B] |
| engine/initenemyfacing.rs | SC-S4-28（S4-118）员转写（2 员——InitEnemyFacing 族；L2 真 0 留白） | mod | s4_init_enemy_facing[B]、s4_eb74_body[B] |
| engine/initenemydelayandhealth.rs | SC-S4-29（S4-119）员转写（2 员——InitEnemyDelayAndHealth 族·S4 收官卡） | mod | s4_init_enemy_delay_health[B]、s4_eb77_body[B] |
| engine/misc.rs | SC-S5-00（S5-120）员转写（2 员——item spawn 链族；L2 真 0 留白·S5 首卡） | mod | s5_lee41[B]、s5_ebc8_body[B] |
| engine/createitemid.rs | SC-S5-02（S5-122）员转写（2 员——CreateItemID 族） | mod | s5_create_item_id[B]、s5_ee47_body[B] |
| engine/spawnmellow.rs | SC-S5-03（S5-123）员转写（2 员——SpawnMellow 族） | mod | s5_spawn_mellow[B]、s5_ee6b_body[B] |
| engine/cannonhandler.rs | SC-S5-04（S5-124）员转写（2 员——CannonHandler 族；L2 真 0 留白） | mod | s95b1_stub_array[B]、s5_eea6_body[B] |
| engine/motherbrainhandler.rs | SC-S5-05（S5-125）员转写（2 员——MotherBrainHandler 族；L2 真 0 留白） | mod | s95b4_stub_array[B]、s5_eeae_body[B] |
| engine/zeebetitehandler.rs | SC-S5-06（S5-126）员转写（2 员——ZeebetiteHandler 族；L2 真 0 留白） | mod | s95b7_stub_array[B]、s5_eecla_body[B] |
| engine/rinkahandler.rs | SC-S5-07（S5-127）员转写（2 员——RinkaHandler 族；L2 真 0 留白） | mod | s95ba_stub_array[B]、s5_eeee_body[B] |
| engine/getenemyxslotposition.rs | SC-S5-08（S5-128）员转写（2 员——GetEnemyXSlotPosition 族） | mod | s5_get_enemy_x_pos[B]、s5_f0a8_body[B] |
| engine/collisionreact.rs | SC-S5-09（S5-129）员转写（2 员——CollisionReact 族；L2 真 0 留白） | mod | s5_collision_react[B]、s5_f0cc_body[B] |
| engine/collisionreactwithsamus.rs | SC-S5-10（S5-130）员转写（2 员——CollisionReactWithSamus 族） | mod | s5_react_with_samus[B]、s5_f0e2_body[B] |
| engine/misc.rs | SC-S5-11（S5-131）员转写（2 员——slot active 族） | mod | s5_slot_active[B]、s5_f0ea_body[B] |
| engine/misc.rs | SC-S5-11（S5-131）员转写（slot active 族——判别面注） | mod | s5_f0ea_disc[B] |
| engine/getobject0coorddata.rs | SC-S5-12（S5-132）员转写（2 员——GetObject0CoordData 族） | mod | s5_get_obj0_coord[B]、s5_f0ef_body[B] |
| engine/distfromobj0toen1.rs | SC-S5-13（S5-133）员转写（2 员——DistFromObj0ToEn1 族；L2 真 0 留白） | mod | s5_dist_obj0_en1[B]、s5_f106_body[B] |
| engine/getenemyyslotposition.rs | SC-S5-14（S5-134）员转写（2 员——GetEnemyYSlotPosition 族；L2 真 0 留白） | mod | s5_get_enemy_y_pos[B]、s5_f109_body[B] |
| engine/misc.rs | SC-S5-15（S5-135）员转写（2 员——EnProjectile 碰撞反应族；L2 真 0 留白） | mod | s5_enproj_react[B]、s5_f10f_body[B] |
| engine/misc.rs | SC-S5-16（S5-136）员转写（2 员——LDC82 链族） | mod | s5_ldc82[B]、s5_f130_body[B] |
| engine/getyslotposition_common.rs | SC-S5-17（S5-137）员转写（2 员——GetYSlotPosition_Common 族） | mod | s5_get_y_common[B]、s5_f16f_body[B] |
| engine/addenemyyslotradiusy.rs | SC-S5-18（S5-138）员转写（2 员——AddEnemyYSlotRadiusY 族） | mod | s5_add_en_y_radius[B]、s5_f1b6_body[B] |
| engine/addenemy1xradius.rs | SC-S5-19（S5-139）员转写（2 员——AddEnemy1XRadius 族·S5 收官卡） | mod | s5_add_en1_x_radius[B]、s5_f1bc_body[B] |
| engine/setsamusprojectileishit.rs | SC-S5-20（S5-140）员转写（2 员——SetSamusProjectileIsHit 族） | mod | s5_set_proj_is_hit[B]、s5_f2d2_body[B] |
| engine/collisionreact_f2df.rs | SC-S5-21（S5-141）员转写（2 员——CollisionReact_F2DF 族） | mod | s5_f2df_or[B]、s5_f2ef_body[B] |
| engine/lf270.rs | SC-S5-22（S5-142）员转写（2 员——LF270 族） | mod | s5_lf270_or[B]、s5_f303_body[B] |
| engine/updateenemy_resting.rs | SC-S5-23（S5-143）员转写（2 员——UpdateEnemy_Resting 族；L2 真 0 留白） | mod | s5_update_resting[B]、s5_f3e0_body[B] |
| engine/removeenemyifitisinlava.rs | SC-S5-24（S5-144）员转写（2 员——RemoveEnemyIfItIsInLava 族；L2 真 0 留白） | mod | s5_remove_lava[B]、s5_f407_body[B] |
| engine/areas_common.rs | SC-S5-25（S5-145）员转写（2 员——$95E5 areas_common 形七；L2 真 0 留白） | mod | s95e5_areas_common[B]、s5_f40d_body[B] |
| engine/updateenemyanim.rs | SC-S5-26（S5-146）员转写（2 员——UpdateEnemyAnim 族） | mod | s5_update_enemy_anim[B]、s5_f410_body[B] |
| engine/areas_common.rs | SC-S5-27（S5-147）员转写（2 员——$8058 areas_common 形八；L2 真 0 留白） | mod | s8058_areas_common[B]、s5_f413_body[B] |
| engine/drawenemy.rs | SC-S5-28（S5-148）员转写（2 员——DrawEnemy 族） | mod | s5_draw_enemy[B]、s5_f42a_body[B] |
| engine/addhealth.rs | SC-S5-29（S5-149）员转写（2 员——AddHealth 族·S5 真收官卡） | mod | s5_add_health[B]、s5_f4b1_body[B] |
| engine/sfx_energypickup.rs | SC-S6-00（S6-150）员转写（2 员——SFX_EnergyPickup 族） | mod | s6_sfx_energy_pickup[B]、s6_f4b4_body[B] |
| engine/sfx_missilepickup.rs | SC-S6-01（S6-151）员转写（2 员——SFX_MissilePickup 族） | mod | s6_sfx_missile_pickup[B]、s6_f4d1_body[B] |
| engine/objectcntrl.rs | SC-S6-02（S6-152）员转写（2 员——ObjectCntrl 族） | mod | s6_object_cntrl[B]、s6_f512_body[B] |
| engine/sub_f673.rs | SC-S6-03（S6-153）员转写（2 员——$F673 桩面族） | mod | s6_f673_stub[B]、s6_f533_body[B] |
| engine/sfx_metal.rs | SC-S6-04（S6-155）员转写（2 员——SFX_Metal 族） | mod | s6_sfx_metal[B]、s6_f573_body[B] |
| engine/sub_f42d.rs | SC-S6-05（S6-156）员转写（2 员——$F42D 清对族） | mod | s6_f42d_clear[B]、s6_f576_body[B] |
| engine/sfx_bosshit.rs | SC-S6-06（S6-157）员转写（2 员——SFX_BossHit 族） | mod | s6_sfx_boss_hit[B]、s6_f584_body[B] |
| engine/sfx_bigenemyhit.rs | SC-S6-07（S6-158）员转写（2 员——SFX_BigEnemyHit 族） | mod | s6_sfx_big_enemy_hit[B]、s6_f5a7_body[B] |
| engine/misc.rs | SC-S6-08（S6-159）员转写（2 员——InArea 检查链族；L2 真 0 留白） | mod | s6_dcfc_area[B]、s6_f623_body[B] |
| engine/sub_81da.rs | SC-S6-09（S6-160）员转写（2 员——$81DA 共享码族） | mod | s6_81da_fill[B]、s6_f70c_body[B] |
| engine/sub_820f.rs | SC-S6-10（S6-161）员转写（2 员——$820F 共享码族；L2 真 0 留白） | mod | s6_820f_shared[B]、s6_f741_body[B] |
| engine/sfx_outofpipe.rs | SC-S6-11（S6-162）员转写（2 员——SFX_OutOfPipe 族） | mod | s6_sfx_out_of_pipe[B]、s6_f7d1_body[B] |
| engine/sub_81d1.rs | SC-S6-12（S6-163）员转写（2 员——$81D1 共享码族；L2 真 0 留白） | mod | s6_81d1_shared[B]、s6_f82c_body[B] |
| engine/sub_f839.rs | SC-S6-13（S6-164）员转写（2 员——$F839 链族） | mod | s6_f839_body[B]、s6_f82f_body[B] |
| engine/sub_8206.rs | SC-S6-14（S6-165）员转写（2 员——$8206 取负写回族；L2 账内深径） | mod | s6_8206_negate[B]、s6_f836_body[B] |
| engine/spawnenprojectile.rs | SC-S6-15（S6-166）员转写（2 员——SpawnEnProjectile 族） | mod | s6_spawn_enproj_slot[B]、s6_f887_body[B] |
| engine/table15.rs | SC-S6-16（S6-167）员转写（2 员——Table15 族） | mod | s6_table15[B]、s6_f88f_body[B] |
| engine/spawnenprojectile_f8f8.rs | SC-S6-17（S6-168）员转写（2 员——SpawnEnProjectile_F8F8 族；L2 真 0 留白） | mod | s6_f8f8_spawn[B]、s6_f8bd_body[B] |
| engine/setenprojectileposition.rs | SC-S6-18（S6-169）员转写（2 员——SetEnProjectilePosition 族） | mod | s6_set_enproj_pos[B]、s6_f8d3_body[B] |
| engine/setenanimindex.rs | SC-S6-19（S6-170）员转写（2 员——SetEnAnimIndex 族） | mod | s6_set_en_anim[B]、s6_f8e5_body[B] |
| engine/loadenemypositionfromtemp.rs | SC-S6-20（S6-171）员转写（2 员——LoadEnemyPositionFromTemp 族） | mod | s6_load_en_pos_tmp[B]、s6_f927_body[B] |
| engine/enemybecomepickup.rs | SC-S6-21（S6-172）员转写（2 员——EnemyBecomePickup 族） | mod | s6_enemy_become_pickup[B]、s6_f979_body[B] |
| engine/misc.rs | SC-S6-22（S6-173）员转写（2 员——LF987 动画计数族；L2 真 0 留白） | mod | s6_lf987_anim[B]、s6_f9b0_body[B] |
| engine/lf518.rs | SC-S6-23（S6-174）员转写（2 员——LF518 族·S6 收官卡） | mod | s6_lf518_state[B]、s6_fa0d_body[B] |
| engine/sub_95a8.rs | SC-S6-24（S6-154）员转写（2 员——$95A8 共享码族；L2 真 0 留白·ID 对齐补落） | mod | s95a8_shared[B]、s6_f568_body[B] |
| engine/jmp.rs | SC-B1-00（a1-000）员转写（2 员——$8000 jmp 桩链族·B 区批产首卡） | mod | a8000_jmp_chain[B]、a99ad_body[B] |
| engine/jmp.rs | SC-B1-01（a1-001）员转写（2 员——$8003 jmp 桩链族·5 员） | mod | a8003_jmp_chain[B]、a99b2_body[B] |
| engine/jmp.rs | SC-B1-02（a1-002）员转写（2 员——$8006 jmp 桩链族·6 员） | mod | a8006_jmp_chain[B]、a99b5_body[B] |
| engine/jmp.rs | SC-B1-03（a1-003）员转写（2 员——$8009 jmp 桩链族） | mod | a8009_jmp_chain[B]、a9a4c_body[B] |
| engine/commonjump_updateenemyanim.rs | SC-B1-04（a1-004）员转写（2 员——CommonJump_UpdateEnemyAnim 族·refs 法直连） | mod | a800c_common_jump[B]、a800c_dispatch[B] |
| engine/initenanimindex.rs | SC-B1-05（a1-005）员转写（2 员——InitEnAnimIndex 族·refs 法） | mod | a800f_common_jump[B]、a800f_init_anim[B] |
| engine/enemyflipafterdisplacement.rs | SC-B1-06（a1-006）员转写（2 员——EnemyFlipAfterDisplacement 族·refs 法） | mod | a801b_common_jump[B]、a801b_flip_face[B] |
| engine/initenactiveanimindex.rs | SC-B1-07（a1-007）员转写（2 员——InitEnActiveAnimIndex 族·refs 法） | mod | a801e_common_jump[B]、a801e_init_active_face[B] |
| engine/applyspeedtoposition.rs | SC-B1-08（a1-008）员转写（2 员——ApplySpeedToPosition 族·refs 法） | mod | a8027_common_jump[B]、a8027_apply_speed[B] |
| engine/commonjump_0e.rs | SC-B1-09（a1-009）员转写（2 员——CommonJump_0E 族·refs 法） | mod | a802a_common_jump[B]、a802a_face[B] |
| engine/enemygetdeltay.rs | SC-B1-10（a1-010）员转写（2 员——EnemyGetDeltaY 族·refs 法） | mod | a802d_common_jump[B]、a802d_face[B] |
| engine/enemygetdeltax.rs | SC-B1-11（a1-011）员转写（2 员——EnemyGetDeltaX 族·refs 法） | mod | a8030_common_jump[B]、a8030_face[B] |
| engine/enemybgcollideorapplyspeed.rs | SC-B1-12（a1-012）员转写（2 员——EnemyBGCollideOrApplySpeed 族·refs 法） | mod | a8033_common_jump[B]、a8033_face[B] |
| engine/enemygetdeltay_usingacceleration.rs | SC-B1-13（a1-013）员转写（2 员——EnemyGetDeltaY_UsingAcceleration 族·refs 法） | mod | a8036_common_jump[B]、a8036_face[B] |
| engine/enemygetdeltax_usingacceleration.rs | SC-B1-14（a1-014）员转写（2 员——EnemyGetDeltaX_UsingAcceleration 族·refs 法） | mod | a8039_common_jump[B]、a8039_face[B] |
| engine/drawdoor.rs | SC-B1-15（a1-015）员转写（2 员——DrawDoor 族·refs 法直得） | mod | a8bb1_draw_door[B]、a8ced_goto2[B] |
| engine/doorsubroutine8c61.rs | SC-B1-16（a1-016）员转写（2 员——DoorSubRoutine8C61 族·refs 法直得） | mod | a8c61_door_sub[B]、a8cf4_goto[B] |
| engine/doorsubroutine8c71.rs | SC-B1-17（a1-017）员转写（2 员——DoorSubRoutine8C71 族·refs 法直得） | mod | a8c71_goto_draw_door[B]、a8cc3_goto[B] |
| engine/doorsubroutine8c76.rs | SC-B1-18（a1-018）员转写（2 员——DoorSubRoutine8C76 族·refs 法直得） | mod | a8c76_door_anim[B]、a8c8d_goto[B] |
| engine/writedoorbgtiles.rs | SC-B1-19（a1-019）员转写（2 员——WriteDoorBGTiles 族·refs 法直得） | mod | a8cfb_write_door_bg[B]、a8c8a_face[B] |
| engine/adiv16.rs | SC-B1-20（a1-020）员转写（2 员——Adiv16 族·refs 法直得） | mod | c2bf_adiv16[B]、a8c97_face[B] |
| engine/amul16.rs | SC-B1-21（a1-021）员转写（2 员——Amul16 族·refs 法直得） | mod | c2c5_amul16[B]、a8cb6_face[B] |
| engine/tourianmusic.rs | SC-B2-00（a2-022）员转写（2 员——TourianMusic 族·refs 法直得·a2 批开） | mod | cc07_tourian_music[B]、a8cbb_face[B] |
| engine/motherbrainmusic.rs | SC-B2-01（a2-023）员转写（2 员——MotherBrainMusic 族·refs 法直得） | mod | cc03_mother_brain_music[B]、a8cc0_face[B] |
| engine/setobjectxslotanimindex.rs | SC-B2-02（a2-024）员转写（2 员——SetObjectXSlotAnimIndex 族·refs 法直得） | mod | d2fd_set_obj_anim[B]、a8cdd_face[B] |
| engine/sfx_door.rs | SC-B2-03（a2-025）员转写（2 员——SFX_Door 族·refs 法直得·正身归档） | mod | cbda_sfx_door[B]、a8ce0_face[B] |
| engine/selectsamuspalette.rs | SC-B2-04（a2-026）员转写（2 员——SelectSamusPalette 族·refs 法直得） | mod | cb73_select_samus_palette[B]、a8ce3_face[B] |
| engine/twoscomplement.rs | SC-B2-05（a2-027）员转写（2 员——TwosComplement 族·refs 法直得） | mod | s95c6_twos_complement[B]、a9b19_face[B] |
| engine/s5.rs | SC-B2-06（a2-028）员转写（2 员——$99A1 S5-146 族同族面） | mod | a99a1_face[B]、a99d0_face[B] |
| engine/s5.rs | SC-B2-07（a2-029）员转写（2 员——$99BA S5-146 族+ptr 对面） | mod | a99ba_face[B]、a99d5_face[B] |
| engine/s5.rs | SC-B2-08（a2-030）员转写（3 员——$99C8 S5-146 族同族面） | mod | a99c8_face[B]、a99e2_face[B] |
| engine/s5.rs | SC-B2-09（a2-031）员转写（2 员——$99CC S5-146 族同族面） | mod | a99cc_face[B]、a9bcf_face[B] |
| engine/15.rs | SC-B2-10（a2-032）员转写（2 员——$9A74 15 写族面） | mod | a9a74_face[B]、a9abd_face[B] |
| engine/x.rs | SC-B2-10（a2-032）员转写（$9A74 x 索引面注） | mod | a9a74_x_face[B] |
| engine/sub_9a7e.rs | SC-B2-11（a2-033）员转写（3 员——$9A7E 链族；L2 真 0 留白） | mod | a9ae2_table_walk[B]、a9a7e_chain[B]、a9ac9_chain[B] |
| engine/16.rs | SC-B2-12（a2-034）员转写（2 员——$9A81 16 写族面） | mod | a9a81_face[B]、a9aa8_face[B] |
| engine/15.rs | SC-B2-13（a2-035）员转写（2 员——$9A84 15 写族面） | mod | a9a84_face[B]、a9ac6_face[B] |
| engine/3.rs | SC-B2-14（a2-036）员转写（2 员——$9A8F 3 写族面） | mod | a9a8f_face[B]、a9aba_face[B] |
| engine/12.rs | SC-B2-15（a2-037）员转写（3 员——$9B8A 12 写族面） | mod | a9b8a_face[B]、a9c84_face[B] |
| engine/x.rs | SC-B2-15（a2-037）员转写（$9B8A x 偏差面注） | mod | a9b8a_x_face[B] |
| engine/8.rs | SC-B2-16（a2-038）员转写（2 员——$9B92 8 写族面） | mod | a9b92_face[B]、a9c96_face[B] |
| engine/x.rs | SC-B2-17（a2-039）员转写（2 员——$9C1D X 环族；L2 真 0 留白） | mod | a9c1d_x_loop[B]、a9bc2_chain[B] |
| engine/9.rs | SC-B2-18（a2-040）员转写（2 员——$9BC5 9 写族面） | mod | a9bc5_face[B]、a9ccc_face[B] |
| engine/8.rs | SC-B2-19（a2-041）员转写（2 员——$9BC8 8 写族面） | mod | a9bc8_face[B]、a9d05_face[B] |
| engine/6.rs | SC-B2-20（a2-042）员转写（2 员——$9BD2 6 写族面·a2 批末卡） | mod | a9bd2_face[B]、a9c1a_face[B] |
| engine/x.rs | SC-B2-20（a2-042）员转写（$9BD2 x 偏差面注·a2 批末卡） | mod | a9bd2_x_face[B] |
| engine/misc.rs | SC-B3-00（a3-000）员转写（2 员——bank2 areas_common 化身·a3 批开） | mod | b2_8000_areas_incarnation[B]、a9845_face[B] |
| engine/misc.rs | SC-B3-01（a3-001）员转写（2 员——bank2 化身 6 员族） | mod | b2_8003_areas_incarnation[B]、a984a_face[B] |
| engine/misc.rs | SC-B3-02（a3-002）员转写（2 员——bank2 化身 5 员族） | mod | b2_8006_areas_incarnation[B]、a984d_face[B] |
| engine/misc.rs | SC-B3-03（a3-003）员转写（2 员——bank2 化身透传族） | mod | b2_8009_areas_incarnation[B]、a9996_face[B] |
| engine/misc.rs | SC-B3-04（a3-004）员转写（2 员——bank2 化身 3 写族面） | mod | b2_800c_areas_incarnation[B]、a99d3_face[B] |
| engine/misc.rs | SC-B3-05（a3-005）员转写（2 员——bank2 化身 4 员族） | mod | b2_800f_areas_incarnation[B]、a99ea_face[B] |
| engine/misc.rs | SC-B3-06（a3-006）员转写（2 员——bank2 化身 4 员族二现） | mod | b2_801e_areas_incarnation[B]、a98c7_face[B] |
| engine/misc.rs | SC-B3-07（a3-007）员转写（2 员——bank2 化身透传族二现） | mod | b2_8021_areas_incarnation[B]、a9b23_face[B] |
| engine/misc.rs | SC-B3-08（a3-008）员转写（2 员——bank2 化身常量单写族） | mod | b2_8027_areas_incarnation[B]、a98ab_face[B] |
| engine/misc.rs | SC-B3-09（a3-009）员转写（2 员——bank2 化身 $04 族面） | mod | b2_802a_areas_incarnation[B]、a995b_face[B] |
| engine/misc.rs | SC-B3-10（a3-010）员转写（2 员——bank2 化身常量零组族） | mod | b2_8036_areas_incarnation[B]、a989c_face[B] |
| engine/misc.rs | SC-B3-11（a3-011）员转写（2 员——bank2 化身 ptr 面族） | mod | b2_8039_areas_incarnation[B]、a98a0_face[B] |
| engine/drawdoor.rs | SC-B3-12（a3-012）员转写（2 员——DrawDoor 族（bank2 化身）） | mod | b2_8bb1_draw_door[B]、b2_8ced_goto2[B] |
| engine/doorsubroutine.rs | SC-B3-13（a3-013）员转写（2 员——DoorSubRoutine 族（bank2 化身）） | mod | b2_8c61_door_sub[B]、b2_8cf4_goto[B] |
| engine/doorsubroutine8c71.rs | SC-B3-14（a3-014）员转写（2 员——DoorSubRoutine8C71 族（bank2 化身）） | mod | b2_8c71_goto_draw_door[B]、b2_8cc3_goto[B] |
| engine/doorsubroutine8c76.rs | SC-B3-15（a3-015）员转写（2 员——DoorSubRoutine8C76 族（bank2 化身）） | mod | b2_8c76_door_anim[B]、b2_8c8d_goto[B] |
| engine/doorsubroutine8c8a.rs | SC-B3-16（a3-016）员转写（2 员——DoorSubRoutine8C8A 族（bank2 化身）） | mod | b2_8c8a_door_sub[B]、b2_8cfb_write_door_bg[B] |
| engine/adiv16.rs | SC-B3-17（a3-017）员转写（2 员——Adiv16 族（bank2 化身）） | mod | b2_8c97_adiv16[B]、b2_c2bf_adiv16[B] |
| engine/amul16.rs | SC-B3-18（a3-018）员转写（2 员——Amul16 族（bank2 化身）） | mod | b2_8cb6_amul16[B]、b2_c2c5_amul16[B] |
| engine/tourianmusic.rs | SC-B3-19（a3-019）员转写（2 员——TourianMusic 族（bank2 化身）） | mod | b2_8cbb_tourian_music[B]、b2_cc07_tourian[B] |
| engine/motherbrainmusic.rs | SC-B3-20（a3-020）员转写（2 员——MotherBrainMusic 族（bank2 化身）） | mod | b2_8cc0_mother_brain_music[B]、b2_cc03_mother_brain[B] |
| engine/setobjectxslotanimindex.rs | SC-B3-21（a3-021）员转写（2 员——SetObjectXSlotAnimIndex 族（bank2 化身）） | mod | b2_8cdd_set_obj_anim[B]、b2_d2fd_set_obj_anim[B] |
| engine/sfx_door.rs | SC-B3-22（a3-022）员转写（2 员——SFX_Door 族（bank2 化身）） | mod | b2_8ce0_sfx_door[B]、b2_cbda_sfx_door[B] |
| engine/selectsamuspalette.rs | SC-B3-23（a3-023）员转写（2 员——SelectSamusPalette 族（bank2 化身）） | mod | b2_8ce3_select_pal[B]、b2_cb73_select_pal[B] |
| engine/twoscomplement.rs | SC-B3-24（a3-024）员转写（2 员——TwosComplement 族（bank2 化身）） | mod | b2_95c6_twos_complement[B]、b2_993d_face[B] |
| engine/misc.rs | SC-B3-25（a3-025）员转写（2 员——bank2 S5-146 族同族面） | mod | b2_9839_face[B]、b2_98d0_face[B] |
| engine/misc.rs | SC-B3-26（a3-026）员转写（2 员——bank2 12 写族面） | mod | b2_98a8_face[B]、b2_9a42_face[B] |
| engine/misc.rs | SC-B3-27（a3-027）员转写（2 员——bank2 8 写族面） | mod | b2_98b0_face[B]、b2_9a52_face[B] |
| engine/misc.rs | SC-B3-28（a3-028）员转写（2 员——bank2 10 写族面） | mod | b2_98eb_face[B]、b2_991b_face[B] |
| engine/misc.rs | SC-B3-29（a3-029）员转写（2 员——bank2 5 写族面） | mod | b2_98ee_face[B]、b2_98f4_face[B] |
| engine/sub_99be.rs | SC-B3-30（a3-030）员转写（2 员——$99BE 链族；L2 真 0 留白） | mod | b2_99be_chain[B]、b2_9a0a_chain[B] |
| engine/sub_99c8.rs | SC-B3-31（a3-031）员转写（3 员——$99C8 链族；L2 真 0 留白） | mod | b2_99c8_chain[B]、b2_9a16_chain[B]、b2_9a2f_y_loop[B] |
| engine/misc.rs | SC-B3-32（a3-032）员转写（2 员——bank2 10 写族面二现） | mod | b2_99cb_face[B]、b2_99f5_face[B] |
| engine/misc.rs | SC-B3-33（a3-033）员转写（2 员——bank2 15 写族面） | mod | b2_99ce_face[B]、b2_9a13_face[B] |
| engine/misc.rs | SC-B3-34（a3-034）员转写（2 员——bank2 3 写族面） | mod | b2_99dc_face[B]、b2_9a07_face[B] |
| engine/misc.rs | SC-B3-35（a3-035）员转写（2 员——bank2 7 写族面·a3 批末卡） | mod | b2_9b4c_face[B]、b2_9b59_face[B] |
| engine/misc.rs | SC-B4-00（a4-000）员转写（2 员——bank3 域开（a4 批）） | mod | b3_8000_areas_incarnation[B]、b3_97ee_face[B] |
| engine/misc.rs | SC-B4-01（a4-001）员转写（2 员——bank3 化身二现（4 员族）） | mod | b3_8003_areas_incarnation[B]、b3_97f3_face[B] |
| engine/misc.rs | SC-B4-02（a4-002）员转写（2 员——bank3 化身三现（2 员族）） | mod | b3_8006_areas_incarnation[B]、b3_97f6_face[B] |
| engine/misc.rs | SC-B4-03（a4-003）员转写（2 员——bank3 化身四现（常量单写族）） | mod | b3_8027_areas_incarnation[B]、b3_995b_face[B] |
| engine/commonjump.rs | SC-B4-04（a4-004）员转写（3 员——$803C CommonJump 表 bank3 化身；L2 真 0 留白） | mod | b3_803c_common_table[B]、b3_a05c_data[B] |
| engine/drawtileblast.rs | SC-B4-05（a4-005）员转写（2 员——DrawTileBlast 族（bank3 化身）） | mod | b3_803f_draw_tile_blast[B]、b3_9f2e_face[B] |
| engine/misc.rs | SC-B4-06（a4-006）员转写（2 员——bank3 化身六现（透传族）） | mod | b3_8042_areas_incarnation[B]、b3_9999_face[B] |
| engine/drawdoor.rs | SC-B4-07（a4-007）员转写（2 员——DrawDoor 族（bank3 化身）） | mod | b3_8bb1_draw_door[B]、b3_8ced_goto2[B] |
| engine/doorsubroutine.rs | SC-B4-08（a4-008）员转写（2 员——DoorSubRoutine 族（bank3 化身）） | mod | b3_8c61_door_sub[B]、b3_8cf4_goto[B] |
| engine/doorsubroutine8c71.rs | SC-B4-09（a4-009）员转写（2 员——DoorSubRoutine8C71 族（bank3 化身）） | mod | b3_8c71_goto_draw_door[B]、b3_8cc3_goto[B] |
| engine/doorsubroutine8c76.rs | SC-B4-10（a4-010）员转写（2 员——DoorSubRoutine8C76 族（bank3 化身）） | mod | b3_8c76_door_anim[B]、b3_8c8d_goto[B] |
| engine/doorsubroutine8c8a.rs | SC-B4-11（a4-011）员转写（2 员——DoorSubRoutine8C8A 族（bank3 化身）） | mod | b3_8c8a_door_sub[B]、b3_8cfb_write_door_bg[B] |
| engine/adiv16.rs | SC-B4-12（a4-012）员转写（2 员——Adiv16 族（bank3 化身）） | mod | b3_8c97_adiv16[B]、b3_c2bf_adiv16[B] |
| engine/amul16.rs | SC-B4-13（a4-013）员转写（2 员——Amul16 族（bank3 化身）） | mod | b3_8cb6_amul16[B]、b3_c2c5_amul16[B] |
| engine/tourianmusic.rs | SC-B4-14（a4-014）员转写（2 员——TourianMusic 族（bank3 化身）） | mod | b3_8cbb_tourian_music[B]、b3_cc07_tourian[B] |
| engine/motherbrainmusic.rs | SC-B4-15（a4-015）员转写（2 员——MotherBrainMusic 族（bank3 化身）） | mod | b3_8cc0_mother_brain_music[B]、b3_cc03_mother_brain[B] |
| engine/setobjectxslotanimindex.rs | SC-B4-16（a4-016）员转写（2 员——SetObjectXSlotAnimIndex 族（bank3 化身）） | mod | b3_8cdd_set_obj_anim[B]、b3_d2fd_set_obj_anim[B] |
| engine/sfx_door.rs | SC-B4-17（a4-017）员转写（2 员——SFX_Door 族（bank3 化身）） | mod | b3_8ce0_sfx_door[B]、b3_cbda_sfx_door[B] |
| engine/selectsamuspalette.rs | SC-B4-18（a4-018）员转写（2 员——SelectSamusPalette 族（bank3 化身）） | mod | b3_8ce3_select_pal[B]、b3_cb73_select_pal[B] |
| engine/twoscomplement.rs | SC-B4-19（a4-019）员转写（2 员——TwosComplement 族（bank3 化身）） | mod | b3_95c6_twos_complement[B]、b3_982f_face[B] |
| engine/misc.rs | SC-B4-20（a4-020）员转写（2 员——bank3 5 写族面） | mod | b3_97e2_face[B]、b3_99ab_face[B] |
| engine/misc.rs | SC-B4-21（a4-021）员转写（2 员——bank3 15 写族面·a4 批末卡） | mod | b3_9817_face[B]、b3_98b0_face[B] |
| engine/misc.rs | SC-B5-00（a5-024）员转写（2 员——bank3 10 写族面（a5 批开）） | mod | b3_9900_face[B]、b3_9871_face[B] |
| engine/misc.rs | SC-B5-01（a5-025）员转写（2 员——bank3 23 写族面） | mod | b3_9926_face[B]、b3_9909_face[B] |
| engine/misc.rs | SC-B5-02（a5-026）员转写（2 员——bank3 17 写族面） | mod | b3_9929_face[B]、b3_99c3_face[B] |
| engine/misc.rs | SC-B5-03（a5-027）员转写（2 员——bank3 14 写族面） | mod | b3_9958_face[B]、b3_99bd_face[B] |
| engine/misc.rs | SC-B5-04（a5-028）员转写（2 员——bank3 10 写族面二现） | mod | b3_995e_face[B]、b3_99e4_face[B] |
| engine/misc.rs | SC-B5-05（a5-029）员转写（2 员——bank3 7 写族面） | mod | b3_9961_face[B]、b3_99f4_face[B] |
| engine/misc.rs | SC-B5-06（a5-030）员转写（2 员——bank3 透传族二现） | mod | b3_99b8_areas_incarnation[B]、b3_9967_face[B] |
| engine/misc.rs | SC-B5-07（a5-031）员转写（2 员——bank3 2 写族面） | mod | b3_99d1_face[B]、b3_99b8_face[B] |
| engine/sub_99d1.rs | SC-B5-08（a5-032）员转写（2 员——$99D1 链族；L2 真 0 留白） | mod | b3_99d1_chain[B]、b3_9a3a_y_loop[B] |
| engine/misc.rs | SC-B5-09（a5-033）员转写（2 员——bank3 3 写族面二现（x 索引）） | mod | b3_9b03_face[B]、b3_9a83_face[B] |
| engine/misc.rs | SC-B5-10（a5-034）员转写（2 员——bank3 透传族三现） | mod | b3_9df2_areas_incarnation[B]、b3_9b03p_face[B] |
| engine/motherbrain.rs | SC-B5-11（a5-035）员转写（2 员——MotherBrain 族（refs 法直得）） | mod | b3_9e25_mother_brain_idle[B]、b3_9df2_mb_collide[B] |
| engine/motherbrain.rs | SC-B5-12（a5-036）员转写（2 员——MotherBrain 族续（refs 法直得）） | mod | b3_9e28_mb_handle_hit[B]、b3_9e25_mb_idle[B] |
| engine/motherbrain.rs | SC-B5-13（a5-037）员转写（2 员——MotherBrain 族续二现（refs 法直得）） | mod | b3_9e2b_mb_update_anim[B]、b3_9e28_mb_handle[B] |
| engine/misc.rs | SC-B5-14（a5-038）员转写（2 员——MotherBrain 数据面；L2 真 0 留白） | mod | b3_9e2b_mb_data[B]、b3_a02e_mb_data[B] |
| engine/misc.rs | SC-B5-15（a5-039）员转写（2 员——MotherBrain 域续四现（refs 法直得）） | mod | b3_9e31_mb_draw_sprites[B]、b3_9e2e_mb_face[B] |
| engine/misc.rs | SC-B5-16（a5-040）员转写（2 员——MotherBrain 域续五现（refs 法直得）） | mod | b3_9e36_mb_clear_hit[B]、b3_9e3e_mb_face[B] |
| engine/misc.rs | SC-B5-17（a5-041）员转写（4 员——MotherBrain 数据面族；L2 真 0 留白） | mod | b3_9e36_mb_hurt_data[B]、b3_9e43_mb_flash_data[B]、b3_9e52_mb_killed_data[B]、b3_9e93_mb_data[B] |
| engine/misc.rs | SC-B5-18（a5-042）员转写（2 员——MotherBrain 域续七现（refs 法直得）·a5 批末卡） | mod | b3_9e8e_mb_face[B]、b3_9e6c_mb_face[B] |
| engine/misc.rs | SC-B6-00（a6-000）员转写（2 员——bank4 域开（a6 批）） | mod | b4_8000_areas_incarnation[B]、b4_9911_face[B] |
| engine/misc.rs | SC-B6-01（a6-001）员转写（2 员——bank4 化身二现（4 员族）） | mod | b4_8003_areas_incarnation[B]、b4_9916_face[B] |
| engine/misc.rs | SC-B6-02（a6-002）员转写（2 员——bank4 化身三现（5 员族）） | mod | b4_8006_areas_incarnation[B]、b4_9919_face[B] |
| engine/misc.rs | SC-B6-03（a6-003）员转写（2 员——bank4 化身四现（透传族）） | mod | b4_8009_areas_incarnation[B]、b4_999b_face[B] |
| engine/misc.rs | SC-B6-04（a6-004）员转写（2 员——bank4 化身五现（3 员族）） | mod | b4_800c_areas_incarnation[B]、b4_99d8_face[B] |
| engine/misc.rs | SC-B6-05（a6-005）员转写（2 员——bank4 化身六现（3 写族二现（x 索引））） | mod | b4_800f_areas_incarnation[B]、b4_99ec_face[B] |
| engine/misc.rs | SC-B6-06（a6-006）员转写（2 员——bank4 化身七现（4 写族）） | mod | b4_801b_areas_incarnation[B]、b4_9929_face[B] |
| engine/misc.rs | SC-B6-07（a6-007）员转写（2 员——bank4 化身八现（3 写族三现）） | mod | b4_801e_areas_incarnation[B]、b4_9943_face[B] |
| engine/misc.rs | SC-B6-08（a6-008）员转写（2 员——bank4 化身九现（常量单写族）） | mod | b4_8027_areas_incarnation[B]、b4_9a9f_face[B] |
| engine/misc.rs | SC-B6-09（a6-009）员转写（2 员——bank4 化身十现（$04 族 x 索引面）） | mod | b4_802a_areas_incarnation[B]、b4_9b6f_face[B] |
| engine/misc.rs | SC-B6-10（a6-010）员转写（2 员——bank4 化身十一现（常量零组族）） | mod | b4_802d_areas_incarnation[B]、b4_9b06_face[B] |
| engine/misc.rs | SC-B6-11（a6-011）员转写（2 员——bank4 化身十二现（ptr 面族）） | mod | b4_8030_areas_incarnation[B]、b4_9b10_face[B] |
| engine/misc.rs | SC-B6-12（a6-012）员转写（2 员——bank4 化身十三现（ptr 面族二现）） | mod | b4_8033_areas_incarnation[B]、b4_9b1a_face[B] |
| engine/misc.rs | SC-B6-13（a6-013）员转写（2 员——bank4 化身十四现（常量零组族二现）） | mod | b4_8036_areas_incarnation[B]、b4_9a90_face[B] |
| engine/misc.rs | SC-B6-14（a6-014）员转写（2 员——bank4 化身十五现（ptr 面族三现）） | mod | b4_8039_areas_incarnation[B]、b4_9a94_face[B] |
| engine/drawdoor.rs | SC-B6-15（a6-015）员转写（2 员——DrawDoor 族（bank4 化身）） | mod | b4_8bb1_draw_door[B]、b4_8ced_goto2[B] |
| engine/doorsubroutine.rs | SC-B6-16（a6-016）员转写（2 员——DoorSubRoutine 族（bank4 化身）） | mod | b4_8c61_door_sub[B]、b4_8cf4_goto[B] |
| engine/doorsubroutine8c71.rs | SC-B6-17（a6-017）员转写（2 员——DoorSubRoutine8C71 族（bank4 化身）） | mod | b4_8c71_goto_draw_door[B]、b4_8cc3_goto[B] |
| engine/doorsubroutine8c76.rs | SC-B6-18（a6-018）员转写（2 员——DoorSubRoutine8C76 族（bank4 化身）） | mod | b4_8c76_door_anim[B]、b4_8c8d_goto[B] |
| engine/doorsubroutine8c8a.rs | SC-B6-19（a6-019）员转写（2 员——DoorSubRoutine8C8A 族（bank4 化身）） | mod | b4_8c8a_door_sub[B]、b4_8cfb_write_door_bg[B] |
| engine/adiv16.rs | SC-B6-20（a6-020）员转写（2 员——Adiv16 族（bank4 化身）） | mod | b4_8c97_adiv16[B]、b4_c2bf_adiv16[B] |
| engine/amul16.rs | SC-B6-21（a6-021）员转写（2 员——Amul16 族（bank4 化身）·a6 批末卡） | mod | b4_8cb6_amul16[B]、b4_c2c5_amul16[B] |
| engine/tourianmusic.rs | SC-B7-00（a7-022）员转写（2 员——TourianMusic 族（bank5 化身）·a7 批开） | mod | b5_8cbb_tourian_music[B]、b5_cc07_tourian[B] |
| engine/motherbrainmusic.rs | SC-B7-01（a7-023）员转写（2 员——MotherBrainMusic 族（bank5 化身）） | mod | b5_8cc0_mother_brain_music[B]、b5_cc03_mother_brain[B] |
| engine/setobjectxslotanimindex.rs | SC-B7-02（a7-024）员转写（2 员——SetObjectXSlotAnimIndex 族（bank5 化身）） | mod | b5_8cdd_set_obj_anim[B]、b5_d2fd_set_obj_anim[B] |
| engine/sfx_door.rs | SC-B7-03（a7-025）员转写（2 员——SFX_Door 族（bank5 化身）） | mod | b5_8ce0_sfx_door[B]、b5_cbda_sfx_door[B] |
| engine/selectsamuspalette.rs | SC-B7-04（a7-026）员转写（2 员——SelectSamusPalette 族（bank5 化身）） | mod | b5_8ce3_select_pal[B]、b5_cb73_select_pal[B] |
| engine/misc.rs | SC-B7-05（a7-027）员转写（2 员——bank5 化身六现（5 写族面）） | mod | b5_9905_areas_incarnation[B]、b5_9934_face[B] |
| engine/misc.rs | SC-B7-06（a7-028）员转写（2 员——bank5 化身七现（11 写族面）） | mod | b5_991e_areas_incarnation[B]、b5_9939_face[B] |
| engine/misc.rs | SC-B7-07（a7-029）员转写（2 员——bank5 化身八现（7 写族面）） | mod | b5_992c_areas_incarnation[B]、b5_9946_face[B] |
| engine/misc.rs | SC-B7-08（a7-030）员转写（2 员——bank5 化身九现（6 写族面）） | mod | b5_9930_areas_incarnation[B]、b5_9ae1_face[B] |
| engine/misc.rs | SC-B7-09（a7-031）员转写（2 员——bank5 化身十现（15 写族面）） | mod | b5_99c3_areas_incarnation[B]、b5_9a0c_face[B] |
| engine/misc.rs | SC-B7-10（a7-032）员转写（3 员——bank5 链族；L2 真 0 留白） | mod | b5_99cd_chain[B]、b5_9a18_chain[B]、b5_9a31_y_loop[B] |
| engine/misc.rs | SC-B7-11（a7-033）员转写（2 员——bank5 化身十一现（15 写族面二现）） | mod | b5_99d0_areas_incarnation[B]、b5_99f7_face[B] |
| engine/misc.rs | SC-B7-12（a7-034）员转写（2 员——bank5 化身十二现（15 写族面三现）） | mod | b5_99d3_areas_incarnation[B]、b5_9a15_face[B] |
| engine/misc.rs | SC-B7-13（a7-035）员转写（2 员——bank5 化身十三现（x 索引 3 写族）） | mod | b5_99de_areas_incarnation[B]、b5_9a09_face[B] |
| engine/misc.rs | SC-B7-14（a7-036）员转写（2 员——bank5 化身十四现（12 写族面）） | mod | b5_9a9c_areas_incarnation[B]、b5_9b98_face[B] |
| engine/misc.rs | SC-B7-15（a7-037）员转写（2 员——bank5 化身十五现（8 写族面）） | mod | b5_9aa4_areas_incarnation[B]、b5_9baa_face[B] |
| engine/misc.rs | SC-B7-16（a7-038）员转写（2 员——MotherBrain 域（refs 法直得）） | mod | b5_9ad4_mb_face[B]、b5_9b2f_mb_face[B] |
| engine/misc.rs | SC-B7-17（a7-039）员转写（2 员——bank5 化身十六现（9 写族面）） | mod | b5_9ad7_areas_incarnation[B]、b5_9be0_face[B] |
| engine/misc.rs | SC-B7-18（a7-040）员转写（2 员——bank5 化身十七现（8 写族面二现）·a7 批末卡） | mod | b5_9ada_areas_incarnation[B]、b5_9c19_face[B] |
| engine/misc.rs | SC-B8-00（a8-000）员转写（2 员——bank6 域开（a8 批）） | mod | b6_8000_areas_incarnation[B]、b6_983c_face[B] |
| engine/misc.rs | SC-B8-01（a8-001）员转写（2 员——bank6 化身二现（5 员族）） | mod | b6_8003_areas_incarnation[B]、b6_9841_face[B] |
| engine/misc.rs | SC-B8-02（a8-002）员转写（2 员——bank6 化身三现（定址清对面）） | mod | b6_8006_areas_incarnation[B]、b6_9844_face[B] |
| engine/misc.rs | SC-B8-03（a8-003）员转写（2 员——bank6 化身四现（透传族）） | mod | b6_8009_areas_incarnation[B]、b6_9967_face[B] |
| engine/misc.rs | SC-B8-04（a8-004）员转写（2 员——bank6 化身五现（3 写族面）） | mod | b6_800c_areas_incarnation[B]、b6_99a4_face[B] |
| engine/misc.rs | SC-B8-05（a8-005）员转写（2 员——bank6 化身六现（x 索引 3 写族）） | mod | b6_800f_areas_incarnation[B]、b6_99bb_face[B] |
| engine/misc.rs | SC-B8-06（a8-006）员转写（2 员——bank6 化身七现（4 写族面）） | mod | b6_801b_areas_incarnation[B]、b6_9854_face[B] |
| engine/misc.rs | SC-B8-07（a8-007）员转写（2 员——bank6 化身八现（3 写族面二现）） | mod | b6_801e_areas_incarnation[B]、b6_9b0a_face[B] |
| engine/misc.rs | SC-B8-08（a8-008）员转写（2 员——bank6 化身九现（透传族二现）） | mod | b6_8021_areas_incarnation[B]、b6_9b45_face[B] |
| engine/misc.rs | SC-B8-09（a8-009）员转写（2 员——bank6 化身十现（常量单写族）） | mod | b6_8027_areas_incarnation[B]、b6_98c2_face[B] |
| engine/misc.rs | SC-B8-10（a8-010）员转写（2 员——bank6 化身十一现（$04 族 x 索引面）） | mod | b6_802a_areas_incarnation[B]、b6_995f_face[B] |
| engine/misc.rs | SC-B8-11（a8-011）员转写（2 员——bank6 化身十二现（常量零组族二现）） | mod | b6_8036_areas_incarnation[B]、b6_98b3_face[B] |
| engine/misc.rs | SC-B8-12（a8-012）员转写（2 员——bank6 化身十三现（ptr 面族二现）） | mod | b6_8039_areas_incarnation[B]、b6_98b7_face[B] |
| engine/drawdoor.rs | SC-B8-13（a8-013）员转写（2 员——DrawDoor 族（bank6 化身）） | mod | b6_8bb1_draw_door[B]、b6_8ced_goto2[B] |
| engine/doorsubroutine.rs | SC-B8-14（a8-014）员转写（2 员——DoorSubRoutine 族（bank6 化身）） | mod | b6_8c61_door_sub[B]、b6_8cf4_goto[B] |
| engine/doorsubroutine8c71.rs | SC-B8-15（a8-015）员转写（2 员——DoorSubRoutine8C71 族（bank6 化身）） | mod | b6_8c71_goto_draw_door[B]、b6_8cc3_goto[B] |
| engine/doorsubroutine8c76.rs | SC-B8-16（a8-016）员转写（2 员——DoorSubRoutine8C76 族（bank6 化身）） | mod | b6_8c76_door_anim[B]、b6_8c8d_goto[B] |
| engine/doorsubroutine8c8a.rs | SC-B8-17（a8-017）员转写（2 员——DoorSubRoutine8C8A 族（bank6 化身）·a8 批末卡段） | mod | b6_8c8a_door_sub[B]、b6_8cfb_write_door_bg[B] |
| engine/amul16.rs | SC-B9-00（a9-019）员转写（2 员——Amul16 族（bank7 化身）·B 区收官批） | mod | b7_8cb6_amul16[B]、b7_c2c5_amul16[B] |
| engine/tourianmusic.rs | SC-B9-01（a9-020）员转写（2 员——TourianMusic 族（bank7 化身）） | mod | b7_8cbb_tourian_music[B]、b7_cc07_tourian[B] |
| engine/motherbrainmusic.rs | SC-B9-02（a9-021）员转写（2 员——MotherBrainMusic 族（bank7 化身）） | mod | b7_8cc0_mother_brain_music[B]、b7_cc03_mother_brain[B] |
| engine/setobjectxslotanimindex.rs | SC-B9-03（a9-022）员转写（2 员——SetObjectXSlotAnimIndex 族（bank7 化身）） | mod | b7_8cdd_set_obj_anim[B]、b7_d2fd_set_obj_anim[B] |
| engine/sfx_door.rs | SC-B9-04（a9-023）员转写（2 员——SFX_Door 族（bank7 化身）） | mod | b7_8ce0_sfx_door[B]、b7_cbda_sfx_door[B] |
| engine/selectsamuspalette.rs | SC-B9-05（a9-024）员转写（2 员——SelectSamusPalette 族（bank7 化身）） | mod | b7_8ce3_select_pal[B]、b7_cb73_select_pal[B] |
| engine/twoscomplement.rs | SC-B9-06（a9-025）员转写（2 员——TwosComplement 族（bank7 化身）） | mod | b7_95c6_twos_complement[B]、b7_9941_face[B] |
| engine/misc.rs | SC-B9-07（a9-026）员转写（2 员——bank7 化身八现（5 写族面）） | mod | b7_9830_areas_incarnation[B]、b7_985f_face[B] |
| engine/misc.rs | SC-B9-08（a9-027）员转写（2 员——bank7 化身九现（11 写族面）） | mod | b7_9849_areas_incarnation[B]、b7_9864_face[B] |
| engine/misc.rs | SC-B9-09（a9-028）员转写（2 员——bank7 化身十现（12 写族面）） | mod | b7_98bf_areas_incarnation[B]、b7_9aae_face[B] |
| engine/misc.rs | SC-B9-10（a9-029）员转写（2 员——bank7 化身十一现（8 写族面）） | mod | b7_98c7_areas_incarnation[B]、b7_9ad9_face[B] |
| engine/misc.rs | SC-B9-11（a9-030）员转写（2 员——bank7 化身十二现（10 写族面）） | mod | b7_98ef_areas_incarnation[B]、b7_991f_face[B] |
| engine/misc.rs | SC-B9-12（a9-031）员转写（3 员——bank7 数据面族；L2 真 0 留白） | mod | b7_98f2_data[B]、b7_98f8_data[B]、b7_994d_data[B] |
| engine/misc.rs | SC-B9-13（a9-032）员转写（2 员——bank7 化身十三现（15 写族面）） | mod | b7_998f_areas_incarnation[B]、b7_99db_face[B] |
| engine/misc.rs | SC-B9-14（a9-033）员转写（3 员——bank7 链族二现；L2 真 0 留白） | mod | b7_9999_chain[B]、b7_99e7_chain[B]、b7_9a00_chain[B] |
| engine/misc.rs | SC-B9-15（a9-034）员转写（2 员——bank7 化身十四现（16 写族面）） | mod | b7_999c_areas_incarnation[B]、b7_99c6_face[B] |
| engine/misc.rs | SC-B9-16（a9-035）员转写（2 员——bank7 化身十五现（15 写族面二现）·a9 批末卡段） | mod | b7_999f_areas_incarnation[B]、b7_99e4_face[B] |
| engine/misc.rs | SC-B9-17（a9-036）员转写（2 员——bank7 化身十六现（3 写族面）·a9 批末卡） | mod | b7_99ad_areas_incarnation[B]、b7_99d8_face[B] |
| engine/misc.rs | a2-043 待补交付（2 员——walk-ring 族（x-=$10 X 环）；L2 真 0 留白） | mod | b2_9c1f_walk_ring[B]、b2_9c2a_chain[B] |
| engine/misc.rs | a4-022 待补交付②（2 员——bank3 化身族面；判别条补触发） | mod | b3_981f_areas_incarnation[B]、b3_9899_face[B] |
| engine/misc.rs | a5-023 待补交付③（3 员——bank5 数据面族+链壳；L2 真 0 留白） | mod | b5_9871_data[B]、b5_987a_data[B]、b5_9a06_chain[B] |
| engine/misc.rs | a5-043 待补交付④（2 员——MotherBrain 域族面；判别条补触发） | mod | b5_9ed0_mb_face[B]、b5_9ed6_mb_face[B] |
| engine/misc.rs | a5-044 待补交付⑤（3 员——MotherBrain 域族面（refs 法直得——命名正身）） | mod | b5_9f49_mb_set_time_bomb[B]、b5_9f69_mb_spawn_door[B]、b5_9fda_mb_scroll[B] |
| engine/misc.rs | a7-041 待补交付⑥（2 员——bank5 化身族面；判别条补触发） | mod | b5_9ae4_areas_incarnation[B]、b5_9b2c_face[B] |
| engine/misc.rs | a7-042 待补交付⑦（2 员——bank5 链壳族面；L2 真 0 留白） | mod | b5_9b31_chain[B]、b5_9b3c_chain[B] |
| engine/adiv16.rs | a8-018 待补交付⑧（2 员——Adiv16 族（bank6 化身）；判别条触发） | mod | b6_8c97_adiv16[B]、b6_c2bf_adiv16[B] |
| engine/misc.rs | a9-037 待补交付⑨（2 员——bank7 化身族面；判别条补触发·末卡） | mod | b7_9a3e_areas_incarnation[B]、b7_9a79_face[B]、dispatch_by_delay_flag[B]、spawn_with_anim_and_slot_search[B]、sync_resting_anim_if_changed[B]、sync_active_anim_if_changed[B]、build_anim_pointer[B]、enemy_type_times2_plus_facing[B]、adjust_counter_with_compliment[B]、set_samus_anim_face[B]、sync_samus_anim_by_table[B]、set_anim_with_motion_state[B]、set_anim_and_dispatch_chain[B]、set_anim_and_return_dispatch[B]、compute_delta_with_clamp[B]、decrement_anim_and_return[B]、set_position_with_flag_cond[B]、set_enemy_pair_and_flag[B]、or_flag_and_conditional_call[B]、toggle_hi_and_step_x[B]、toggle_hi_and_dispatch_de[B]、negate_u8[B]、negate_and_dispatch_d4[B]、negate_shift_and_mask_cond[B]、negate_anim_and_load_type[B]、negate_and_range_dispatch[B]、get_object_coords_dispatch[B]、carry_dispatch_d4f8_d624[B]、coords_and_table_walk[B]、sync_status_if_f7[B]、coords_with_pointer_copy[B]、compute_ptr_with_borrow[B]、ptr_compute_with_high_clamp[B]、ptr_compute_with_count_loop[B]、ptr_compute_with_shift_segment[B]、ptr_compute_with_flag_flip[B]、compute_ptr_with_carry_add[B]、toggle_hi_and_set_y_ff[B]、toggle_hi_and_set_y_f0[B]、ptr_add_with_flag_flip[B]、ptr_add_with_shift_segment[B]、find_item_in_history[A]、apply_item_pickup_face[B]、store_item_position[B]、ptr_add_plain[B]、ptr_add_and_dispatch_d6[B]、store_zp_pair_if_found[B]、store_zebetite_init_if_found[B]、stop_vertical_movement[B]、stop_vert_and_cond_state[B]、stop_vert_and_dispatch_d1[B]、stop_vert_and_step_x_cond[B]、set_samus_stand_face[B]、stand_and_dispatch_cd[B]、stand_and_decrement_anim[B]、stand_and_flip_slot_flag[B]、dispatch_de47[B]、anim_draw_with_counter_select[B]、anim_draw_cond_write[B]、table_walk_root[B]、table_walk_call[B]、sfx_metroid_hit_face[B]、metroid_hit_and_dispatch_f6[B]、metroid_hit_and_dispatch_cb[B]、read_enemy_type_flags[B]、flags_mask_and_sfx_dispatch[B]、flags_cond_write_counter[B]、sfx_enemy_hit_face[B]、enemy_hit_and_check_state[B]、enemy_hit_and_check_state_alt[B]、load_en_hi_and_eor_carry[B]、en_hi_cond_write_single[B]、en_hi_cond_write_double[B]、convert_to_pickup_if_hit[B]、pickup_convert_resting[B]、pickup_convert_active_with_ptr[B]、enemy_bg_collide_check[B]、set_item_ptr_pair[B]、item_spawn_with_check[B]、pack_item_id[B]、pack_item_id_and_dispatch[B]、cannon_handler_face[B]、mother_brain_handler_face[B]、zeebetite_handler_face[B]、rinka_handler_face[B]、get_enemy_x_slot_position[B]、clear_ptr_and_set_y_cond[B]、energy_pickup_and_dispatch[B]、sfx_missile_pickup_face[B]、object_control_and_dispatch[B]、load_x_from_4b_stub[B]、stage_motion_flag_with_dup[B]、sfx_metal_face[B]、sfx_boss_hit_face[B]、boss_hit_passthrough[B]、big_enemy_hit_cond_write[B]、metal_hit_cond_stage[B]、area_hit_cond_write[B]、get_enemy_room_ptr[B]、room_ptr_with_cart_face[B]、make_cart_ram_ptr[B]、clear_enemy_anim_delay[B]、clear_anim_delay_and_dispatch[B]、load_table18_and_pair[B]、load_position_from_temp[B]、mellow_pos_cond_write[B]、table18_with_state_write[B]、cart_ptr_and_dispatch[B]、tile_blast_with_ptr_pair[B]、draw_tile_blast[B]、ref_s7_get_en_room_ptr[B]、ref_s7_fa2a_body[B]、ref_s7_make_cart_ptr[B]、ref_s7_fa8e_body[B]、ref_s7_clear_en_anim_delay[B]、ref_s7_fbbc_body[B]、ref_s7_table18[B]、ref_s7_fcc1_body[B]、ref_s7_mellow_load_pos[B]、ref_s7_fcfa_body[B]、ref_s7_draw_tile_blast[B]、ref_s7_fe95_body[B]、sfx_mother_brain_music[B]、set_object_xslot_anim_index[B]、sfx_door[B]、select_samus_palette[B]、bank1_dispatch_four_stub[B]、bank1_dispatch_a3_stub[B]、stage_slot_state_write[B]、passthrough_anim_state[B]、common_jump_update_enemy_anim[B]、update_enemy_anim_dispatch[B]、store_packed_face_byte[B]、pack_face_and_dispatch[B]、type_match_dispatch[B]、flag_hit_stack_dispatch[B]、flag_hit_set_and_jump[B]、and_data05_cmp_dispatch[B]、store_enemy_position_triple[B]、enemy_pos_and_dispatch_e7[B]、enemy_pos_and_dispatch_e8[B]、enemy_pos_and_dispatch_fa[B]、and_data05_reverse_dispatch[B]、anim_draw_object_root[B]、advance_vector_motion[B]、enemy_object_vm_dispatch[B]、enemy_record_length[B]、vm_return_tail_a[B]、vm_return_tail_b[B]、choose_spawning_routine[B]、compute_radius_sums[B]、collision_check_with_y_slot[B]、collision_loop_cond_call[B]、advance_y16_x_minus16[B]、y16_with_cond_call[B]、y16_with_c0_check[B]、y16_advance_face[B]、collision_variant_tail[B]、samus_dead_and_slot_check[B]、check_samus_dead_state[B]、samus_dead_dispatch_f1[B]、samus_dead_dispatch_f1b[B]、check_collision_delta[B]、load_pos_pair_and_tail[B]、check_collision_and_call[B]、get_object_y_slot_position[B]、obj_y_slot_with_x_ring[B]、obj_y_slot_and_tail[B]、collision_chain_face[B]、update_enemy_active_branch_b[B]、update_enemy_frozen_check[B]、enemy_react_to_projectile[B]、obj_y_slot_with_pos_pair[B]、cond_table_write_on_flag[B]、clear_motion_and_delay[B]、kill_object_clear[B]、load_pos_triple[B] |
| engine/misc.rs | Phase 1：samus 域（语义改写+逻辑合并） | mod | check_samus_input[B]、update_samus_position[B]、apply_samus_gravity[B]、select_samus_animation[B]、update_samus_jump[B]、update_samus_landing[B]、update_samus_running[B]、update_samus_skidding[B]、update_samus_damage[B]、s1_flag_gate[B]、s1_set_samus_anim[B]、s1_d498[A] |
| engine/misc.rs | Phase 2：enemy 域（语义改写+逻辑合并） | mod | update_enemy_velocity[B]、apply_enemy_gravity[B]、update_enemy_health[B] |
| engine/misc.rs | Phase 3：world 域（语义改写+逻辑合并） | mod | elevator_scroll_control[B]、elevator_move_control[B]、elevator_state_control[B]、door_interaction[C] |
| engine/misc.rs | Phase 4：sound/title/render 域（语义改写+逻辑合并） | mod | play_sfx[B]、read_enemy_table[B]、update_sfx_state[B]、update_enemy_audio[B]、handle_pickup[B]、check_enemy_bg_collision[B]、update_samus_jump_s3[B] |
| engine/setsamusanim.rs | SC-S1-09 员转写（5 员——SetSamusAnim 族） | samus_common | set_samus_animation[C]、s1_d07a_body[B]、s1_d088_body[B]、s1_d12b_body[B]、s1_d8b4_body[B] |
| engine/togglesamushi.rs | SC-S1-11 员转写（5 员——ToggleSamusHi 族） | samus_common | s1_toggle_samus_hi[B]、s1_d400_body[B]、s1_d40e_body[B]、s1_d447_body[B]、s1_d465_body[B] |
| engine/stopvertmovement.rs | SC-S1-17 员转写（4 员——StopVertMovement 族） | samus_common | s1_stop_vert_movement[B]、s1_d012_body[B]、s1_d116_body[B]、s1_d3f4_body[B] |
| engine/setsamusstand.rs | SC-S1-18 员转写（4 员——SetSamusStand 族） | samus_common | s1_set_samus_stand[B]、s1_d04d_body[B]、s1_d110_body[B]、s1_d957_body[B] |
| engine/misc.rs | SC-S2-03（S2-33）员转写（3 员——LCF2E SamusHit 判链族） | samus_common | s2_lcf2e[B]、s2_d018_body[B]、s2_d133_body[B] |
| engine/checkhealthstatus.rs | SC-S2-07（S2-37）员转写（3 员——CheckHealthStatus 族；免重判定=真转写+重合注记） | samus_common | s2_check_health[B]、s2_d406_body[B]、s2_d411_body[B] |
| engine/samuselevator.rs | SC-S2-10（S2-40）员转写（3 员——SamusElevator 族） | samus_common | s2_samus_elevator[B]、s2_d450_body[B]、s2_d46e_body[B] |
| engine/samushurt.rs | SC-S2-22（S2-52）员转写（3 员——SamusHurt 族；L2 真 0 留白） | samus_common | s2_samus_hurt[B]、s2_f133_body[B]、s2_fec2_body[B] |
| engine/isscrewattackactive.rs | SC-S2-24（S2-54）员转写（3 员——IsScrewAttackActive 族） | samus_common | s2_is_screw_attack[B]、s2_f287_body[B]、s2_f2f4_body[B] |
| engine/forcespeedtowardssamus.rs | SC-S2-26（S2-56）员转写（3 员——ForceSpeedTowardsSamus 族；L2 真 0 留白） | samus_common | s2_force_speed[B]、s2_f3cf_body[B]、s2_f401_body[B] |
| engine/distancetosamusthreshold.rs | SC-S2-27（S2-57）员转写（3 员——DistanceToSamusThreshold 族；L2 真 0 留白） | samus_common | s2_dist_threshold[B]、s2_f3d2_body[B]、s2_f404_body[B] |
| engine/samusjump.rs | SC-S3-06（S3-66）员转写（2 员——SamusJump 族） | samus_common | s3_samus_jump[B]、s3_d015_body[B] |
| engine/setsamusjump.rs | SC-S3-08（S3-68）员转写（2 员——SetSamusJump 族） | samus_common | s3_set_samus_jump[B]、s3_d036_body[B] |
| engine/sub_cd6b.rs | SC-S3-09（S3-69）员转写（2 员——$CD6B 态链族；L2 真 0 留白） | samus_common | s3_cd6b_body[B]、s3_d039_body[B] |
| engine/setsamuspntup.rs | SC-S3-10（S3-70）员转写（2 员——SetSamusPntUp 族） | samus_common | s3_set_samus_pnt_up[B]、s3_d048_body[B] |
| engine/setsamusrunaccel.rs | SC-S3-14（S3-74）员转写（2 员——SetSamusRunAccel 族） | samus_common | s3_set_samus_run_accel[B]、s3_d130_body[B] |
| engine/sub_d150.rs | SC-S3-15（S3-75）员转写（2 员——$D150 态检查族；L2 真 0 留白） | samus_common | s3_d150_check[B]、s3_d136_body[B] |
| engine/clearhorzdata.rs | SC-S3-16（S3-76）员转写（2 员——ClearHorzData 族） | samus_common | s3_clear_horz_data[B]、s3_d13f_body[B] |
| engine/killobject.rs | SC-S1-08 员转写（6 员——KillObject 族） | enemy_common | update_enemy_movement_x[B]、update_enemy_movement_y[B]、update_enemy_movement_ext[B]、kill_enemy[B]、update_enemy_state_ext[B]、update_enemy_projectile_pickup[B] |
| engine/storeenemypositiontotemp.rs | SC-S1-21 员转写（4 员——StoreEnemyPositionToTemp 族） | enemy_common | s1_store_enemy_pos[B]、s1_e789_body[B]、s1_e906_body[B]、s1_f91f_body[B] |
| engine/misc.rs | SC-S1-22 员转写（4 员——EnemyLoop 对象指令 VM） | enemy_common | s1_vm_advance[B]、s1_enemy_loop[B]、s1_eb09_rec_len[B]、s1_lx230_stub[B]、s1_lx237_stub[B] |
| engine/misc.rs | SC-S1-23 员转写（4 员——ChooseSpawningRoutine spawn VM） | enemy_common | s1_choose_spawning[B]、s1_spawn_mellows[B]、s1_spawn_cannon[B]、s1_eef7[B] |
| engine/collisiondetection.rs | SC-S1-24 员转写（4 员——CollisionDetection 族；L2 真 0 留白） | enemy_common | s1_radius_sums[B]、s1_collision_check[B]、s1_f0c9_body[B]、s1_f0df_body[B] |
| engine/enemyreacttosamusprojectile.rs | SC-S1-29 员转写（4 员——EnemyReactToSamusProjectile 族；L2 真 0 留白·S1 末卡） | enemy_common | s1_enemy_react_projectile[B]、s1_f3e3_body[B]、s1_update_active_b[B]、s1_update_frozen[B] |
| engine/andendata05.rs | SC-S2-00（S2-30）员转写（4 员——AndEnData05 族） | enemy_common | s1_and_en_data05[B]、s2_f6e2_body[B]、s2_f711_body[B]、update_enemy_f83b[B] |
| engine/orendata05.rs | SC-S2-01（S2-31）员转写（4 员——OrEnData05 族） | enemy_common | update_enemy_or_en_data05[B]、update_enemy_f702[B]、update_enemy_f735[B]、update_enemy_f761[B] |
| engine/lf97c.rs | SC-S2-02（S2-32）员转写（4 员——LF97C 族） | enemy_common | s2_lf97c_body[B]、s2_fa15_body[B]、s2_fa7a_body[B]、s2_fa9a_body[B] |
| engine/bitscan.rs | SC-S2-05（S2-35）员转写（3 员——BitScan 族；L2 真 0 留白） | enemy_common | s2_bit_scan[B]、s2_d120_body[B]、s2_d1a9_body[B] |
| engine/sub_e519.rs | SC-S2-09（S2-39）员转写（3 员——$E519 状态步进族；L2 真 0 留白） | enemy_common | s2_state_step[B]、s2_d43d_body[B]、s2_d89d_body[B] |
| engine/sub_e4f1.rs | SC-S2-11（S2-41）员转写（3 员——$E4F1 状态回退族；L2 真 0 留白） | enemy_common | s2_state_stepback[B]、s2_d45d_body[B]、s2_d897_body[B] |
| engine/sub_8296.rs | SC-S2-12（S2-42）员转写（3 员——$8296 条件取负族；L2 真 0 留白） | enemy_common | s2_cond_negate[B]、s2_d560_body[B]、s2_f9be_body[B] |
| engine/sub_832f.rs | SC-S2-13（S2-43）员转写（3 员——$832F 符号扩展族） | enemy_common | s2_sign_extend[B]、s2_d56a_body[B]、s2_f9c8_body[B] |
| engine/misc.rs | SC-S2-21（S2-51）员转写（3 员——Xminus16 位置族） | enemy_common | s2_xminus16[B]、s2_f0e5_body[B]、s2_f136_body[B] |
| engine/subtracthealth.rs | SC-S2-23（S2-53）员转写（3 员——SubtractHealth 族） | enemy_common | s2_subtract_health[B]、s2_f13d_body[B]、s2_fec9_body[B] |
| engine/amul8.rs | SC-S2-25（S2-55）员转写（3 员——Amul8 族） | enemy_common | s2_amul8[B]、s2_f335_body[B]、s2_ff4e_body[B] |
| engine/pageindex.rs | SC-S2-28（S2-58）员转写（3 员——PageIndex 族） | enemy_common | s2_page_index[B]、s2_f480_body[B]、s2_f4eb_body[B] |
| engine/lf515.rs | SC-S2-29（S2-59）员转写（3 员——LF515 族） | enemy_common | s2_lf515[B]、s2_f50b_body[B]、s2_f554_body[B] |
| engine/loadenhitoy.rs | SC-S3-03（S3-63）员转写（3 员——LoadEnHiToY 族） | enemy_common | s3_load_en_hi[B]、s3_f6eb_body[B]、s3_f71a_body[B] |
| engine/enprojectile.rs | SC-S3-04（S3-64）员转写（3 员——EnProjectile 族） | enemy_common | s3_become_pickup[B]、s3_f96a_body[B]、s3_f991_body[B] |
| engine/enemybgcollide.rs | SC-S3-05（S3-65）员转写（3 员——EnemyBGCollide 族） | enemy_common | s3_enemy_bg_collide[B]、s3_f96d_body[B]、s3_f9f2_body[B] |
| engine/misc.rs | SC-S3-19（S3-79）员转写（2 员——ChooseRoutine JumpEngine 本体；L2 真 0 留白） | enemy_common | s3_choose_routine[B]、s3_d1d4_table[B] |
| engine/checkforitem.rs | SC-S1-16 员转写（5 员——CheckForItem 族；L2 真 0 留白） | door_room | s1_check_for_item[B]、s1_ebcb_body[B]、s1_ee14_body[B]、s1_eeb9_body[B]、s1_eed7_body[B] |
| engine/storeobjectpositiontotemp.rs | SC-S2-04（S2-34）员转写（3 员——StoreObjectPositionToTemp 族；L2 真 0 留白） | door_room | s2_store_obj_pos[B]、s2_d101_body[B]、s2_d630_body[B] |
| engine/checkblasttile.rs | SC-S2-14（S2-44）员转写（3 员——CheckBlastTile 族） | door_room | s2_check_blast_tile[B]、s2_d61b_body[B]、s2_d770_body[B] |
| engine/misc.rs | SC-S2-18（S2-48）员转写（3 员——DoorHandler VM 族） | door_room | s2_spawn_door[B]、s2_eb8c_body[B]、s2_eef4_body[B] |
| engine/misc.rs | SC-S2-20（S2-50）员转写（3 员——ItemCheck VM 族） | door_room | s2_eec6_body[B]、s2_eeeb_body[B]、s2_eef1_body[B] |
| engine/checkmoveup.rs | SC-S3-12（S3-72）员转写（2 员——CheckMoveUp 族；L2 真 0 留白） | door_room | s3_check_move_up[B]、s3_d0fa_body[B] |
| engine/loadobjectpositionfromtemp.rs | SC-S3-13（S3-73）员转写（2 员——LoadObjectPositionFromTemp 族；L2 真 0 留白） | door_room | s3_load_obj_pos[B]、s3_d10d_body[B] |
## 4. 每批校验清单（移动 commit 禁语义改动）

1. **doc==fn 数**：移动后各 doc 块数==pub fn 数（脚本核（孤注=0））。
2. **锚守恒**：`bank\d+:` 计数移动前后守恒（对批前 HEAD）。
3. **fn 守恒**：pub fn 总数不变（纯移动证）。
4. **全测**：cargo test --workspace 全绿（计数不变）。
5. **gates check**：PASS。
6. **L4 无回退**：批 3/6/7 后各跑 run-all（门链五点+落地三点保持）。
7. 语义改动（含警告顺手修）**单独 commit**（不混移动）。

Evidence: 审计脚本实测（1102/184 锚/389 族）+8 文件 7 实点 grep 实文+lead 聚合裁定
——A 级原文。
Confidence: 90（族名提取=正则启发（个别族头名待 lead 逐行裁定（表内原头全列）））
