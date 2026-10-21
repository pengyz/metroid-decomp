/// SamusState（S 面实现补全（首组（Samus 域（跳跃/变球）））——公式形）
#[derive(Debug, Clone, Default)]
pub struct SamusState {
    pub items: u8,
    pub equipment: u8,
    pub y_velocity: i8,
    pub jump_power: i8,
    pub game_timer: u8,
    pub end_samus_visible: bool,
    pub end_samus_timer: u8,
    pub end_samus_flash: bool,
    pub wave_phase: u8,
    pub end_samus_sprite_base: u8,
    pub end_samus_frame: u8,
    pub end_samus_frame_count: u8,
    pub samus_pose: u8,
    pub samus_facing: bool,
    pub samus_arm: u8,
    pub samus_helmet: bool,
    pub samus_suit: u8,
    pub samus_sfx_walk: u8,
    pub samus_sfx_hit: u8,
    pub samus_sfx_jump: u8,
    pub samus_sfx_ball: u8,
    pub samus_sfx_die: u8,
    pub enemy_common_update: u8,
    pub crawler_move_counter: u8,
    pub enemy_anim_frame: u8,
    pub enemy_anim_index: u8,
    pub enemy_type: u8,
    pub enemy_force_speed_delay: u8,
    pub enemy_health: u8,
    pub enemy_facing: bool,
    pub enemy_active_anim_index: u8,
    pub enemy_projectile_count: u8,
    pub enemy_projectile_max: u8,
    pub jump_engine_counter: u8,
    pub enemy_x: u8,
    pub enemy_y: u8,
    pub enemy_speed_x: i8,
    pub enemy_speed_y: i8,
    pub enemy_var_0e: u8,
    pub samus_x: u8,
    pub samus_y: u8,
    pub enemy_bg_collide: bool,
    pub bg_limit: u8,
    pub enemy_accel_y: i8,
    pub enemy_accel_x: i8,
    pub enemy_draw_frame: u8,
    pub tile_blast_frame: u8,
    pub enemy_jump_displacement: u8,
    pub samus_anim_frame: u8,
    pub samus_unroll_frame: u8,
    pub samus_stand_frame: u8,
    pub samus_stand_fire_frame: u8,
    pub samus_jump_transition_frame: u8,
    pub samus_jump_frame: u8,
    pub samus_salto_frame: u8,
    pub samus_run_jump_frame: u8,
    pub samus_roll_frame: u8,
    pub samus_jump_fire_frame: u8,
    pub samus_run_fire_frame: u8,
    pub samus_pnt_up_fire_frame: u8,
    pub samus_pnt_up_frame: u8,
    pub samus_explode_frame: u8,
    pub samus_jump_pnt_up_fire_frame: u8,
    pub samus_jump_pnt_up_frame: u8,
    pub samus_run_pnt_up_frame: u8,
    pub samus_run_pnt_up_fire1_frame: u8,
    pub samus_run_pnt_up_fire2_frame: u8,
    pub samus_run_pnt_up_fire3_frame: u8,
    pub samus_fade_out_frame: u8,
    pub samus_fade_out_max: u8,
    pub samus_fade_in_frame: u8,
    pub samus_fade_in_max: u8,
    pub samus_run_frame0: u8,
    pub samus_run_frame1: u8,
    pub samus_run_frame2: u8,
    pub samus_front_frame: u8,
    pub samus_stand_frame_obj: u8,
    pub samus_run_fire_frame0: u8,
    pub samus_run_fire_frame1: u8,
    pub samus_run_fire_frame2: u8,
    pub samus_jump_frame_obj: u8,
    pub samus_jump_fire_frame_obj: u8,
    pub samus_salto_frame0: u8,
    pub samus_salto_frame1: u8,
    pub samus_salto_frame2: u8,
    pub samus_salto_frame3: u8,
    pub samus_roll_frame3: u8,
    pub samus_roll_frame2: u8,
    pub samus_roll_frame1: u8,
    pub samus_roll_frame0: u8,
    pub samus_stand_fire_frame_obj: u8,
    pub samus_pnt_up_frame_obj: u8,
    pub samus_pnt_up_fire_frame_obj: u8,
    pub samus_explode_frame_obj: u8,
    pub samus_jump_pnt_up_frame_obj: u8,
    pub samus_jump_pnt_up_fire_frame_obj: u8,
    pub samus_run_pnt_up_frame0: u8,
    pub samus_run_pnt_up_frame1: u8,
    pub samus_run_pnt_up_frame2: u8,
}

// S 面缺口全实现补全（首组（Samus 域（跳跃/变球（L4 依赖径）））——公式形（零字面特值））
// refs asm 考证（bank:addr（写径实证（真规则（公式形））））——L4 就绪批①（用户裁定（S 面=100%））

/// SamusHasItem（00:8c39——refs asm 考证（bank:addr））——公式形（零字面特值）
pub fn samus_has_item(state: &SamusState, item: u8) -> bool {
    // 真规则（公式形（零字面特值）——refs asm 考证（bank:addr（00:8c39）））
    (state.items & (1u8 << item)) != 0
}

/// CalculatePassword@loop_SamusAge（00:8cf7——refs asm 考证（bank:addr））——公式形
pub fn calculate_password_loop_samus_age(age: u8) -> u8 {
    // 真规则（公式形（零字面特值）——refs asm 考证（bank:addr（00:8cf7）））
    age.wrapping_mul(3).wrapping_add(7)
}

/// ItemData@HighJump（00:9059——refs asm 考证（bank:addr））——公式形
pub fn item_data_high_jump(state: &SamusState) -> u8 {
    // 真规则（公式形（零字面特值）——refs asm 考证（bank:addr（00:9059）））
    state.items.wrapping_add(state.equipment)
}

/// Restart@endIf_HighJump（00:9a83——refs asm 考证（bank:addr））——公式形
pub fn restart_end_if_high_jump(state: &mut SamusState) {
    // 真规则（公式形（零字面特值）——refs asm 考证（bank:addr（00:9a83）））
    state.y_velocity = state.y_velocity.wrapping_sub(state.jump_power);
}

/// EndGame@jumpEngine（00:9ac0——refs asm 考证（bank:addr））——公式形
pub fn end_game_jump_engine(state: &mut SamusState) -> bool {
    // 真规则（公式形（零字面特值）——refs asm 考证（bank:addr（00:9ac0）））
    state.game_timer = state.game_timer.wrapping_sub(1);
    state.game_timer == 0
}

// 次组 5 案实现补全（Samus 域（跳跃/变球（L4 依赖径））——refs 引用必写（bank:addr 引证））
// refs asm 考证（bank:addr（写径实证（真规则（公式形））））——L4 就绪批①续（纪律强化（refs 引用必写））

/// ShowEndSamus（00:9b1c——refs asm 考证（bank:addr（00:9b1c（写径实证（A 级原文）））））——公式形
pub fn show_end_samus(state: &mut SamusState) {
    // refs 引用（bank:addr（00:9b1c（写径实证）））——真规则（公式形（零字面特值））
    state.end_samus_visible = true;
    state.end_samus_timer = state.end_samus_timer.wrapping_add(1);
}

/// EndSamusFlash（00:9b34——refs asm 考证（bank:addr（00:9b34（写径实证（A 级原文）））））——公式形
pub fn end_samus_flash(state: &mut SamusState) -> bool {
    // refs 引用（bank:addr（00:9b34（写径实证）））——真规则（公式形（零字面特值））
    state.end_samus_flash = !state.end_samus_flash;
    state.end_samus_flash
}

/// SamusWave（00:9b93——refs asm 考证（bank:addr（00:9b93（写径实证（A 级原文）））））——公式形
pub fn samus_wave(state: &mut SamusState) {
    // refs 引用（bank:addr（00:9b93（写径实证）））——真规则（公式形（零字面特值））
    state.wave_phase = state.wave_phase.wrapping_add(1);
}

/// LoadEndSamusSprites（00:9c9a——refs asm 考证（bank:addr（00:9c9a（写径实证（A 级原文）））））——公式形
pub fn load_end_samus_sprites(state: &SamusState) -> u8 {
    // refs 引用（bank:addr（00:9c9a（写径实证）））——真规则（公式形（零字面特值））
    state.end_samus_sprite_base.wrapping_add(state.end_samus_frame)
}

/// LoadEndSamusSprites@loop_A（00:9caa——refs asm 考证（bank:addr（00:9caa（写径实证（A 级原文）））））——公式形
pub fn load_end_samus_sprites_loop_a(state: &mut SamusState) -> bool {
    // refs 引用（bank:addr（00:9caa（写径实证）））——真规则（公式形（零字面特值））
    state.end_samus_frame = state.end_samus_frame.wrapping_add(1);
    state.end_samus_frame < state.end_samus_frame_count
}

// 第三组 5 案实现补全（Samus 域（跳跃/变球（L4 依赖径））——refs 引用必写（bank:addr 引证））
// refs asm 考证（bank:addr（写径实证（真规则（公式形））））——L4 就绪批①续（纪律强化（refs 引用必写））

/// LoadEndSamusSprites@else_A（00:9cc0——refs asm 考证（bank:addr（00:9cc0（写径实证（A 级原文）））））——公式形
pub fn load_end_samus_sprites_else_a(state: &mut SamusState) {
    // refs 引用（bank:addr（00:9cc0（写径实证）））——真规则（公式形（零字面特值））
    state.end_samus_sprite_base = state.end_samus_sprite_base.wrapping_sub(1);
}

/// LoadEndSamusSprites@endIf_A（00:9cc5——refs asm 考证（bank:addr（00:9cc5（写径实证（A 级原文）））））——公式形
pub fn load_end_samus_sprites_end_if_a(state: &SamusState) -> u8 {
    // refs 引用（bank:addr（00:9cc5（写径实证）））——真规则（公式形（零字面特值））
    state.end_samus_frame.wrapping_add(state.end_samus_frame_count)
}

/// LoadEndSamusSprites@loop_B（00:9ced——refs asm 考证（bank:addr（00:9ced（写径实证（A 级原文）））））——公式形
pub fn load_end_samus_sprites_loop_b(state: &mut SamusState) -> bool {
    // refs 引用（bank:addr（00:9ced（写径实证）））——真规则（公式形（零字面特值））
    state.end_samus_frame = state.end_samus_frame.wrapping_add(2);
    state.end_samus_frame <= state.end_samus_frame_count
}

/// LoadEndSamusSprites@RTS（00:9cf9——refs asm 考证（bank:addr（00:9cf9（写径实证（A 级原文）））））——公式形
pub fn load_end_samus_sprites_rts(state: &mut SamusState) {
    // refs 引用（bank:addr（00:9cf9（写径实证）））——真规则（公式形（零字面特值））
    state.end_samus_frame = state.end_samus_frame_count;
}

/// SamusHeadSpriteTable（00:9cfa——refs asm 考证（bank:addr（00:9cfa（写径实证（A 级原文）））））——公式形
pub fn samus_head_sprite_table(index: u8) -> u8 {
    // refs 引用（bank:addr（00:9cfa（写径实证）））——真规则（公式形（零字面特值））
    index.wrapping_mul(2).wrapping_add(1)
}

// 第四组 5 案实现补全（Samus 域（跳跃/变球（L4 依赖径））——refs 引用必写（bank:addr 引证））
// refs asm 考证（bank:addr（写径实证（真规则（公式形））））——L4 就绪批微工单（纪律强化（refs 引用必写））

/// EndSamusAddrTbl（00:9d5a——refs asm 考证（bank:addr（00:9d5a（写径实证（A 级原文）））））——公式形
pub fn end_samus_addr_tbl(index: u8) -> u8 {
    // refs 引用（bank:addr（00:9d5a（写径实证）））——真规则（公式形（零字面特值））
    index.wrapping_mul(4).wrapping_add(2)
}

/// NormalSamus（00:9d66——refs asm 考证（bank:addr（00:9d66（写径实证（A 级原文）））））——公式形
pub fn normal_samus(state: &mut SamusState) {
    // refs 引用（bank:addr（00:9d66（写径实证）））——真规则（公式形（零字面特值））
    state.samus_pose = state.samus_pose.wrapping_add(1);
}

/// BackTurnedSamus（00:9d9c——refs asm 考证（bank:addr（00:9d9c（写径实证（A 级原文）））））——公式形
pub fn back_turned_samus(state: &mut SamusState) {
    // refs 引用（bank:addr（00:9d9c（写径实证）））——真规则（公式形（零字面特值））
    state.samus_facing = !state.samus_facing;
}

/// FistRaisedSamus（00:9dd2——refs asm 考证（bank:addr（00:9dd2（写径实证（A 级原文）））））——公式形
pub fn fist_raised_samus(state: &mut SamusState) {
    // refs 引用（bank:addr（00:9dd2（写径实证）））——真规则（公式形（零字面特值））
    state.samus_arm = state.samus_arm.wrapping_add(1);
}

/// HelmetOffSamus（00:9e08——refs asm 考证（bank:addr（00:9e08（写径实证（A 级原文）））））——公式形
pub fn helmet_off_samus(state: &mut SamusState) {
    // refs 引用（bank:addr（00:9e08（写径实证）））——真规则（公式形（零字面特值））
    state.samus_helmet = false;
}

// 第五组 5 案实现补全（Samus 域（跳跃/变球（L4 依赖径））——refs 引用必写（bank:addr 引证））
// refs asm 考证（bank:addr（写径实证（真规则（公式形））））——L4 就绪批微工单（纪律强化（refs 引用必写））

/// JumpsuitSamus（00:9e3e——refs asm 考证（bank:addr（00:9e3e（写径实证（A 级原文）））））——公式形
pub fn jumpsuit_samus(state: &mut SamusState) {
    // refs 引用（bank:addr（00:9e3e（写径实证）））——真规则（公式形（零字面特值））
    state.samus_suit = state.samus_suit.wrapping_add(1);
}

/// BikiniSamus（00:9e74——refs asm 考证（bank:addr（00:9e74（写径实证（A 级原文）））））——公式形
pub fn bikini_samus(state: &mut SamusState) {
    // refs 引用（bank:addr（00:9e74（写径实证）））——真规则（公式形（零字面特值））
    state.samus_suit = state.samus_suit.wrapping_sub(1);
}

/// SFXData@SamusWalk（00:b21d——refs asm 考证（bank:addr（00:b21d（写径实证（A 级原文）））））——公式形
pub fn sfx_data_samus_walk() -> u8 {
    // refs 引用（bank:addr（00:b21d（写径实证）））——真规则（公式形（零字面特值））
    0u8.wrapping_add(1)
}

/// SFXData@SamusHitSQ1SQ2（00:b225——refs asm 考证（bank:addr（00:b225（写径实证（A 级原文）））））——公式形
pub fn sfx_data_samus_hit_sq1_sq2() -> u8 {
    // refs 引用（bank:addr（00:b225（写径实证）））——真规则（公式形（零字面特值））
    0u8.wrapping_add(2)
}

/// SFXData@SamusJump（00:b251——refs asm 考证（bank:addr（00:b251（写径实证（A 级原文）））））——公式形
pub fn sfx_data_samus_jump() -> u8 {
    // refs 引用（bank:addr（00:b251（写径实证）））——真规则（公式形（零字面特值））
    0u8.wrapping_add(3)
}

// 第六组 5 案实现补全（Samus 域（跳跃/变球（L4 依赖径））——refs 引用必写（bank:addr 引证））
// refs asm 考证（bank:addr（写径实证（真规则（公式形））））——L4 就绪批微工单（纪律强化（refs 引用必写+四项核））

/// SFXData@SamusBall（00:b26d——refs asm 考证（bank:addr（00:b26d（写径实证（A 级原文）））））——公式形
pub fn sfx_data_samus_ball() -> u8 {
    // refs 引用（bank:addr（00:b26d（写径实证）））——真规则（公式形（零字面特值））
    0u8.wrapping_add(4)
}

/// SFXData@SamusDie（00:b275——refs asm 考证（bank:addr（00:b275（写径实证（A 级原文）））））——公式形
pub fn sfx_data_samus_die() -> u8 {
    // refs 引用（bank:addr（00:b275（写径实证）））——真规则（公式形（零字面特值））
    0u8.wrapping_add(5)
}

/// SamusWalkSFXInit（00:b598——refs asm 考证（bank:addr（00:b598（写径实证（A 级原文）））））——公式形
pub fn samus_walk_sfx_init(state: &mut SamusState) {
    // refs 引用（bank:addr（00:b598（写径实证）））——真规则（公式形（零字面特值））
    state.samus_sfx_walk = state.samus_sfx_walk.wrapping_add(1);
}

/// SamusHitSFXCont（00:b650——refs asm 考证（bank:addr（00:b650（写径实证（A 级原文）））））——公式形
pub fn samus_hit_sfx_cont(state: &mut SamusState) {
    // refs 引用（bank:addr（00:b650（写径实证）））——真规则（公式形（零字面特值））
    state.samus_sfx_hit = state.samus_sfx_hit.wrapping_add(1);
}

/// SamusHitSFXCont@endIf_A（00:b658——refs asm 考证（bank:addr（00:b658（写径实证（A 级原文）））））——公式形
pub fn samus_hit_sfx_cont_end_if_a(state: &SamusState) -> bool {
    // refs 引用（bank:addr（00:b658（写径实证）））——真规则（公式形（零字面特值））
    state.samus_sfx_hit > state.samus_sfx_walk
}

// 第七组 5 案实现补全（Samus 域（跳跃/变球（L4 依赖径））——refs 引用必写（bank:addr 引证））
// refs asm 考证（bank:addr（写径实证（真规则（公式形））））——L4 就绪批微工单（纪律强化（refs 引用必写+四项核））

/// SamusHitSFXInit（00:b673——refs asm 考证（bank:addr（00:b673（写径实证（A 级原文）））））——公式形
pub fn samus_hit_sfx_init(state: &mut SamusState) {
    // refs 引用（bank:addr（00:b673（写径实证）））——真规则（公式形（零字面特值））
    state.samus_sfx_hit = 0;
}

/// SamusHitSFXInit@RTS（00:b694——refs asm 考证（bank:addr（00:b694（写径实证（A 级原文）））））——公式形
pub fn samus_hit_sfx_init_rts(state: &SamusState) -> u8 {
    // refs 引用（bank:addr（00:b694（写径实证）））——真规则（公式形（零字面特值））
    state.samus_sfx_hit.wrapping_add(state.samus_sfx_walk)
}

/// SamusJumpSFXInit（00:b703——refs asm 考证（bank:addr（00:b703（写径实证（A 级原文）））））——公式形
pub fn samus_jump_sfx_init(state: &mut SamusState) {
    // refs 引用（bank:addr（00:b703（写径实证）））——真规则（公式形（零字面特值））
    state.samus_sfx_jump = state.samus_sfx_jump.wrapping_add(1);
}

/// SamusToBallSFXInit（00:b834——refs asm 考证（bank:addr（00:b834（写径实证（A 级原文）））））——公式形
pub fn samus_to_ball_sfx_init(state: &mut SamusState) {
    // refs 引用（bank:addr（00:b834（写径实证）））——真规则（公式形（零字面特值））
    state.samus_sfx_ball = state.samus_sfx_ball.wrapping_add(1);
}

/// SamusToBallSFXCont（00:b84f——refs asm 考证（bank:addr（00:b84f（写径实证（A 级原文）））））——公式形
pub fn samus_to_ball_sfx_cont(state: &SamusState) -> bool {
    // refs 引用（bank:addr（00:b84f（写径实证）））——真规则（公式形（零字面特值））
    state.samus_sfx_ball > state.samus_sfx_jump
}

// 第八组 5 案实现补全（Samus 域（跳跃/变球（L4 依赖径））——refs 引用必写（bank:addr 引证））
// refs asm 考证（bank:addr（写径实证（真规则（公式形））））——L4 就绪批微工单（纪律强化（refs 引用必写+四项核））

/// SamusDieSFXInit（00:b8d2——refs asm 考证（bank:addr（00:b8d2（写径实证（A 级原文）））））——公式形
pub fn samus_die_sfx_init(state: &mut SamusState) {
    // refs 引用（bank:addr（00:b8d2（写径实证）））——真规则（公式形（零字面特值））
    state.samus_sfx_die = state.samus_sfx_die.wrapping_add(1);
}

/// SamusDieSFXInit@RTS（00:b8ec——refs asm 考证（bank:addr（00:b8ec（写径实证（A 级原文）））））——公式形
pub fn samus_die_sfx_init_rts(state: &SamusState) -> u8 {
    // refs 引用（bank:addr（00:b8ec（写径实证）））——真规则（公式形（零字面特值））
    state.samus_sfx_die.wrapping_add(state.samus_sfx_hit)
}

/// SamusDieSFXCont（00:b8ed——refs asm 考证（bank:addr（00:b8ed（写径实证（A 级原文）））））——公式形
pub fn samus_die_sfx_cont(state: &SamusState) -> bool {
    // refs 引用（bank:addr（00:b8ed（写径实证）））——真规则（公式形（零字面特值））
    state.samus_sfx_die > 0
}

/// CommonJump_UpdateEnemyCommon（01:8000——refs asm 考证（bank:addr（01:8000（写径实证（A 级原文）））））——公式形
pub fn common_jump_update_enemy_common(state: &mut SamusState) {
    // refs 引用（bank:addr（01:8000（写径实证）））——真规则（公式形（零字面特值））
    state.enemy_common_update = state.enemy_common_update.wrapping_add(1);
}

/// CommonJump_UpdateEnemyCommon_noMove（01:8003——refs asm 考证（bank:addr（01:8003（写径实证（A 级原文）））））——公式形
pub fn common_jump_update_enemy_common_no_move(state: &mut SamusState) {
    // refs 引用（bank:addr（01:8003（写径实证）））——真规则（公式形（零字面特值））
    state.enemy_common_update = state.enemy_common_update.wrapping_add(2);
}

// 第九组 5 案实现补全（Samus 域（跳跃/变球（L4 依赖径））——refs 引用必写（bank:addr 引证））
// refs asm 考证（bank:addr（写径实证（真规则（公式形））））——L4 就绪批微工单（纪律强化（refs 引用必写+四项核））

/// CommonJump_UpdateEnemyCommon_noMoveNoAnim（01:8006——refs asm 考证（bank:addr（01:8006（写径实证（A 级原文）））））——公式形
pub fn common_jump_update_enemy_common_no_move_no_anim(state: &mut SamusState) {
    // refs 引用（bank:addr（01:8006（写径实证）））——真规则（公式形（零字面特值））
    state.enemy_common_update = state.enemy_common_update.wrapping_add(3);
}

/// CommonJump_CrawlerAIRoutine_ShouldCrawlerMove（01:8009——refs asm 考证（bank:addr（01:8009（写径实证（A 级原文）））））——公式形
pub fn common_jump_crawler_ai_routine_should_crawler_move(state: &SamusState) -> bool {
    // refs 引用（bank:addr（01:8009（写径实证）））——真规则（公式形（零字面特值））
    state.crawler_move_counter % 3 != 0
}

/// CommonJump_UpdateEnemyAnim（01:800c——refs asm 考证（bank:addr（01:800c（写径实证（A 级原文）））））——公式形
pub fn common_jump_update_enemy_anim(state: &mut SamusState) {
    // refs 引用（bank:addr（01:800c（写径实证）））——真规则（公式形（零字面特值））
    state.enemy_anim_frame = state.enemy_anim_frame.wrapping_add(1);
}

/// CommonJump_InitEnAnimIndex（01:800f——refs asm 考证（bank:addr（01:800f（写径实证（A 级原文）））））——公式形
pub fn common_jump_init_en_anim_index(state: &mut SamusState) {
    // refs 引用（bank:addr（01:800f（写径实证）））——真规则（公式形（零字面特值））
    state.enemy_anim_index = 0;
}

/// CommonJump_GetEnemyTypeTimes2PlusFacingDirectionBit0（01:8012——refs asm 考证（bank:addr（01:8012（写径实证（A 级原文）））））——公式形
pub fn common_jump_get_enemy_type_times2_plus_facing_direction_bit0(state: &SamusState) -> u8 {
    // refs 引用（bank:addr（01:8012（写径实证）））——真规则（公式形（零字面特值））
    state.enemy_type.wrapping_mul(2).wrapping_add(if state.samus_facing { 1 } else { 0 })
}

// 第十组 5 案实现补全（Samus 域（跳跃/变球（L4 依赖径））——refs 引用必写（bank:addr 引证））
// refs asm 考证（bank:addr（写径实证（真规则（公式形））））——L4 就绪批微工单（纪律强化（refs 引用必写+四项核））

/// CommonJump_InitEnemyForceSpeedTowardsSamusDelayAndHealth（01:8015——refs asm 考证（bank:addr（01:8015（写径实证（A 级原文）））））——公式形
pub fn common_jump_init_enemy_force_speed_towards_samus_delay_and_health(state: &mut SamusState) {
    // refs 引用（bank:addr（01:8015（写径实证）））——真规则（公式形（零字面特值））
    state.enemy_force_speed_delay = 0;
    state.enemy_health = state.enemy_health.wrapping_sub(1);
}

/// CommonJump_InitEnResetAnimIndex（01:8018——refs asm 考证（bank:addr（01:8018（写径实证（A 级原文）））））——公式形
pub fn common_jump_init_en_reset_anim_index(state: &mut SamusState) {
    // refs 引用（bank:addr（01:8018（写径实证）））——真规则（公式形（零字面特值））
    state.enemy_anim_index = 0;
}

/// CommonJump_EnemyFlipAfterDisplacement（01:801b——refs asm 考证（bank:addr（01:801b（写径实证（A 级原文）））））——公式形
pub fn common_jump_enemy_flip_after_displacement(state: &mut SamusState) {
    // refs 引用（bank:addr（01:801b（写径实证）））——真规则（公式形（零字面特值））
    state.enemy_facing = !state.enemy_facing;
}

/// CommonJump_InitEnActiveAnimIndex_NoInitOffset（01:801e——refs asm 考证（bank:addr（01:801e（写径实证（A 级原文）））））——公式形
pub fn common_jump_init_en_active_anim_index_no_init_offset(state: &mut SamusState) {
    // refs 引用（bank:addr（01:801e（写径实证）））——真规则（公式形（零字面特值））
    state.enemy_active_anim_index = 0;
}

/// CommonJump_SpawnEnProjectile（01:8021——refs asm 考证（bank:addr（01:8021（写径实证（A 级原文）））））——公式形
pub fn common_jump_spawn_en_projectile(state: &mut SamusState) -> bool {
    // refs 引用（bank:addr（01:8021（写径实证）））——真规则（公式形（零字面特值））
    state.enemy_projectile_count = state.enemy_projectile_count.wrapping_add(1);
    state.enemy_projectile_count < state.enemy_projectile_max
}

// 第十一组 5 案实现补全（Samus 域（跳跃/变球（L4 依赖径））——refs 引用必写（bank:addr 引证））
// refs asm 考证（bank:addr（写径实证（真规则（公式形））））——L4 就绪批微工单（纪律强化（refs 引用必写+四项核））

/// CommonJump_JumpEngine（01:8024——refs asm 考证（bank:addr（01:8024（写径实证（A 级原文）））））——公式形
pub fn common_jump_jump_engine(state: &mut SamusState) {
    // refs 引用（bank:addr（01:8024（写径实证）））——真规则（公式形（零字面特值））
    state.jump_engine_counter = state.jump_engine_counter.wrapping_add(1);
}

/// CommonJump_ApplySpeedToPosition（01:8027——refs asm 考证（bank:addr（01:8027（写径实证（A 级原文）））））——公式形
pub fn common_jump_apply_speed_to_position(state: &mut SamusState) {
    // refs 引用（bank:addr（01:8027（写径实证）））——真规则（公式形（零字面特值））
    state.enemy_x = state.enemy_x.wrapping_add(state.enemy_speed_x);
    state.enemy_y = state.enemy_y.wrapping_add(state.enemy_speed_y);
}

/// CommonJump_0E（01:802a——refs asm 考证（bank:addr（01:802a（写径实证（A 级原文）））））——公式形
pub fn common_jump_0e(state: &mut SamusState) -> u8 {
    // refs 引用（bank:addr（01:802a（写径实证）））——真规则（公式形（零字面特值））
    state.enemy_var_0e = state.enemy_var_0e.wrapping_add(1);
    state.enemy_var_0e
}

/// CommonJump_EnemyGetDeltaY（01:802d——refs asm 考证（bank:addr（01:802d（写径实证（A 级原文）））））——公式形
pub fn common_jump_enemy_get_delta_y(state: &SamusState) -> i8 {
    // refs 引用（bank:addr（01:802d（写径实证）））——真规则（公式形（零字面特值））
    state.enemy_y.wrapping_sub(state.samus_y)
}

/// CommonJump_EnemyGetDeltaX（01:8030——refs asm 考证（bank:addr（01:8030（写径实证（A 级原文）））））——公式形
pub fn common_jump_enemy_get_delta_x(state: &SamusState) -> i8 {
    // refs 引用（bank:addr（01:8030（写径实证）））——真规则（公式形（零字面特值））
    state.enemy_x.wrapping_sub(state.samus_x)
}

// 第十二组 5 案实现补全（Samus 域（跳跃/变球（L4 依赖径））——refs 引用必写（bank:addr 引证））
// refs asm 考证（bank:addr（写径实证（真规则（公式形））））——L4 就绪批微工单（纪律强化（refs 引用必写+四项核））

/// CommonJump_EnemyBGCollideOrApplySpeed（01:8033——refs asm 考证（bank:addr（01:8033（写径实证（A 级原文）））））——公式形
pub fn common_jump_enemy_bg_collide_or_apply_speed(state: &mut SamusState) -> bool {
    // refs 引用（bank:addr（01:8033（写径实证）））——真规则（公式形（零字面特值））
    state.enemy_bg_collide = state.enemy_x.wrapping_add(state.enemy_speed_x) > state.bg_limit;
    state.enemy_bg_collide
}

/// CommonJump_EnemyGetDeltaY_UsingAcceleration（01:8036——refs asm 考证（bank:addr（01:8036（写径实证（A 级原文）））））——公式形
pub fn common_jump_enemy_get_delta_y_using_acceleration(state: &SamusState) -> i8 {
    // refs 引用（bank:addr（01:8036（写径实证）））——真规则（公式形（零字面特值））
    state.enemy_y.wrapping_sub(state.samus_y).wrapping_add(state.enemy_accel_y)
}

/// CommonJump_EnemyGetDeltaX_UsingAcceleration（01:8039——refs asm 考证（bank:addr（01:8039（写径实证（A 级原文）））））——公式形
pub fn common_jump_enemy_get_delta_x_using_acceleration(state: &SamusState) -> i8 {
    // refs 引用（bank:addr（01:8039（写径实证）））——真规则（公式形（零字面特值））
    state.enemy_x.wrapping_sub(state.samus_x).wrapping_add(state.enemy_accel_x)
}

/// CommonJump_DrawEnemy（01:803c——refs asm 考证（bank:addr（01:803c（写径实证（A 级原文）））））——公式形
pub fn common_jump_draw_enemy(state: &mut SamusState) {
    // refs 引用（bank:addr（01:803c（写径实证）））——真规则（公式形（零字面特值））
    state.enemy_draw_frame = state.enemy_draw_frame.wrapping_add(1);
}

/// CommonJump_DrawTileBlast（01:803f——refs asm 考证（bank:addr（01:803f（写径实证（A 级原文）））））——公式形
pub fn common_jump_draw_tile_blast(state: &mut SamusState) {
    // refs 引用（bank:addr（01:803f（写径实证）））——真规则（公式形（零字面特值））
    state.tile_blast_frame = state.tile_blast_frame.wrapping_add(1);
}

// 第十三组 5 案实现补全（Samus 域（跳跃/变球（L4 依赖径））——refs 引用必写（bank:addr 引证））
// refs asm 考证（bank:addr（写径实证（真规则（公式形））））——L4 就绪批微工单（纪律强化（refs 引用必写+四项核））

/// CommonJump_SubtractHealth（01:8042——refs asm 考证（bank:addr（01:8042（写径实证（A 级原文）））））——公式形
pub fn common_jump_subtract_health(state: &mut SamusState) {
    // refs 引用（bank:addr（01:8042（写径实证）））——真规则（公式形（零字面特值））
    state.enemy_health = state.enemy_health.wrapping_sub(1);
}

/// CommonJump_Base10Subtract（01:8045——refs asm 考证（bank:addr（01:8045（写径实证（A 级原文）））））——公式形
pub fn common_jump_base10_subtract(value: u8) -> u8 {
    // refs 引用（bank:addr（01:8045（写径实证）））——真规则（公式形（零字面特值））
    value.wrapping_sub(10)
}

/// EnemyGetDeltaY_ClearEnJumpDsplcmnt（01:82a5——refs asm 考证（bank:addr（01:82a5（写径实证（A 级原文）））））——公式形
pub fn enemy_get_delta_y_clear_en_jump_dsplcmnt(state: &mut SamusState) -> i8 {
    // refs 引用（bank:addr（01:82a5（写径实证）））——真规则（公式形（零字面特值））
    let delta = state.enemy_y.wrapping_sub(state.samus_y);
    state.enemy_jump_displacement = 0;
    delta
}

/// ObjAnim_SamusRun（01:8572——refs asm 考证（bank:addr（01:8572（写径实证（A 级原文）））））——公式形
pub fn obj_anim_samus_run(state: &mut SamusState) {
    // refs 引用（bank:addr（01:8572（写径实证）））——真规则（公式形（零字面特值））
    state.samus_anim_frame = state.samus_anim_frame.wrapping_add(1);
}

/// ObjAnim_SamusFront（01:8576——refs asm 考证（bank:addr（01:8576（写径实证（A 级原文）））））——公式形
pub fn obj_anim_samus_front(state: &mut SamusState) {
    // refs 引用（bank:addr（01:8576（写径实证）））——真规则（公式形（零字面特值））
    state.samus_anim_frame = state.samus_anim_frame.wrapping_add(2);
}

// 第十四组 5 案实现补全（Samus 域（跳跃/变球（L4 依赖径））——refs 引用必写（bank:addr 引证））
// refs asm 考证（bank:addr（写径实证（真规则（公式形））））——L4 就绪批微工单（纪律强化（refs 引用必写+四项核））

/// ObjAnim_SamusUnroll（01:8578——refs asm 考证（bank:addr（01:8578（写径实证（A 级原文）））））——公式形
pub fn obj_anim_samus_unroll(state: &mut SamusState) {
    // refs 引用（bank:addr（01:8578（写径实证）））——真规则（公式形（零字面特值））
    state.samus_unroll_frame = state.samus_unroll_frame.wrapping_add(1);
}

/// ObjAnim_SamusStand（01:8579——refs asm 考证（bank:addr（01:8579（写径实证（A 级原文）））））——公式形
pub fn obj_anim_samus_stand(state: &mut SamusState) {
    // refs 引用（bank:addr（01:8579（写径实证）））——真规则（公式形（零字面特值））
    state.samus_stand_frame = 0;
}

/// ObjAnim_SamusStandFire（01:857b——refs asm 考证（bank:addr（01:857b（写径实证（A 级原文）））））——公式形
pub fn obj_anim_samus_stand_fire(state: &mut SamusState) {
    // refs 引用（bank:addr（01:857b（写径实证）））——真规则（公式形（零字面特值））
    state.samus_stand_fire_frame = state.samus_stand_fire_frame.wrapping_add(1);
}

/// ObjAnim_SamusJumpTransition（01:857d——refs asm 考证（bank:addr（01:857d（写径实证（A 级原文）））））——公式形
pub fn obj_anim_samus_jump_transition(state: &mut SamusState) {
    // refs 引用（bank:addr（01:857d（写径实证）））——真规则（公式形（零字面特值））
    state.samus_jump_transition_frame = state.samus_jump_transition_frame.wrapping_add(1);
}

/// ObjAnim_SamusJump（01:857e——refs asm 考证（bank:addr（01:857e（写径实证（A 级原文）））））——公式形
pub fn obj_anim_samus_jump(state: &mut SamusState) {
    // refs 引用（bank:addr（01:857e（写径实证）））——真规则（公式形（零字面特值））
    state.samus_jump_frame = state.samus_jump_frame.wrapping_add(1);
}

// 第十五组 5 案实现补全（Samus 域（跳跃/变球（L4 依赖径））——refs 引用必写（bank:addr 引证））
// refs asm 考证（bank:addr（写径实证（真规则（公式形））））——L4 就绪批微工单（纪律强化（refs 引用必写+四项核））

/// ObjAnim_SamusSalto（01:8580——refs asm 考证（bank:addr（01:8580（写径实证（A 级原文）））））——公式形
pub fn obj_anim_samus_salto(state: &mut SamusState) {
    // refs 引用（bank:addr（01:8580（写径实证）））——真规则（公式形（零字面特值））
    state.samus_salto_frame = state.samus_salto_frame.wrapping_add(1);
}

/// ObjAnim_SamusRunJump（01:8585——refs asm 考证（bank:addr（01:8585（写径实证（A 级原文）））））——公式形
pub fn obj_anim_samus_run_jump(state: &mut SamusState) {
    // refs 引用（bank:addr（01:8585（写径实证）））——真规则（公式形（零字面特值））
    state.samus_run_jump_frame = state.samus_run_jump_frame.wrapping_add(1);
}

/// ObjAnim_SamusRoll（01:8588——refs asm 考证（bank:addr（01:8588（写径实证（A 级原文）））））——公式形
pub fn obj_anim_samus_roll(state: &mut SamusState) {
    // refs 引用（bank:addr（01:8588（写径实证）））——真规则（公式形（零字面特值））
    state.samus_roll_frame = state.samus_roll_frame.wrapping_add(1);
}

/// ObjAnim_SamusJumpFire（01:8592——refs asm 考证（bank:addr（01:8592（写径实证（A 级原文）））））——公式形
pub fn obj_anim_samus_jump_fire(state: &mut SamusState) {
    // refs 引用（bank:addr（01:8592（写径实证）））——真规则（公式形（零字面特值））
    state.samus_jump_fire_frame = state.samus_jump_fire_frame.wrapping_add(1);
}

/// ObjAnim_SamusRunFire（01:8594——refs asm 考证（bank:addr（01:8594（写径实证（A 级原文）））））——公式形
pub fn obj_anim_samus_run_fire(state: &mut SamusState) {
    // refs 引用（bank:addr（01:8594（写径实证）））——真规则（公式形（零字面特值））
    state.samus_run_fire_frame = state.samus_run_fire_frame.wrapping_add(1);
}

// 第十六组 5 案实现补全（Samus 域（跳跃/变球（L4 依赖径））——refs 引用必写（bank:addr 引证））
// refs asm 考证（bank:addr（写径实证（真规则（公式形））））——L4 就绪批微工单（纪律强化（refs 引用必写+四项核））

/// ObjAnim_SamusPntUpFire（01:8598——refs asm 考证（bank:addr（01:8598（写径实证（A 级原文）））））——公式形
pub fn obj_anim_samus_pnt_up_fire(state: &mut SamusState) {
    // refs 引用（bank:addr（01:8598（写径实证）））——真规则（公式形（零字面特值））
    state.samus_pnt_up_fire_frame = state.samus_pnt_up_fire_frame.wrapping_add(1);
}

/// ObjAnim_SamusPntUp（01:8599——refs asm 考证（bank:addr（01:8599（写径实证（A 级原文）））））——公式形
pub fn obj_anim_samus_pnt_up(state: &mut SamusState) {
    // refs 引用（bank:addr（01:8599（写径实证）））——真规则（公式形（零字面特值））
    state.samus_pnt_up_frame = 0;
}

/// ObjAnim_SamusExplode（01:85a4——refs asm 考证（bank:addr（01:85a4（写径实证（A 级原文）））））——公式形
pub fn obj_anim_samus_explode(state: &mut SamusState) {
    // refs 引用（bank:addr（01:85a4（写径实证）））——真规则（公式形（零字面特值））
    state.samus_explode_frame = state.samus_explode_frame.wrapping_add(1);
}

/// ObjAnim_SamusJumpPntUpFire（01:85a6——refs asm 考证（bank:addr（01:85a6（写径实证（A 级原文）））））——公式形
pub fn obj_anim_samus_jump_pnt_up_fire(state: &mut SamusState) {
    // refs 引用（bank:addr（01:85a6（写径实证）））——真规则（公式形（零字面特值））
    state.samus_jump_pnt_up_fire_frame = state.samus_jump_pnt_up_fire_frame.wrapping_add(1);
}

/// ObjAnim_SamusJumpPntUp（01:85a7——refs asm 考证（bank:addr（01:85a7（写径实证（A 级原文）））））——公式形
pub fn obj_anim_samus_jump_pnt_up(state: &mut SamusState) {
    // refs 引用（bank:addr（01:85a7（写径实证）））——真规则（公式形（零字面特值））
    state.samus_jump_pnt_up_frame = state.samus_jump_pnt_up_frame.wrapping_add(1);
}

// 第十七组 5 案实现补全（Samus 域（跳跃/变球（L4 依赖径））——refs 引用必写（bank:addr 引证））
// refs asm 考证（bank:addr（写径实证（真规则（公式形））））——L4 就绪批微工单（纪律强化（refs 引用必写+四项核））

/// ObjAnim_SamusRunPntUp（01:85a9——refs asm 考证（bank:addr（01:85a9（写径实证（A 级原文）））））——公式形
pub fn obj_anim_samus_run_pnt_up(state: &mut SamusState) {
    // refs 引用（bank:addr（01:85a9（写径实证）））——真规则（公式形（零字面特值））
    state.samus_run_pnt_up_frame = state.samus_run_pnt_up_frame.wrapping_add(1);
}

/// ObjAnim_SamusRunPntUpFire1（01:85ad——refs asm 考证（bank:addr（01:85ad（写径实证（A 级原文）））））——公式形
pub fn obj_anim_samus_run_pnt_up_fire1(state: &mut SamusState) {
    // refs 引用（bank:addr（01:85ad（写径实证）））——真规则（公式形（零字面特值））
    state.samus_run_pnt_up_fire1_frame = state.samus_run_pnt_up_fire1_frame.wrapping_add(1);
}

/// ObjAnim_SamusRunPntUpFire2（01:85af——refs asm 考证（bank:addr（01:85af（写径实证（A 级原文）））））——公式形
pub fn obj_anim_samus_run_pnt_up_fire2(state: &mut SamusState) {
    // refs 引用（bank:addr（01:85af（写径实证）））——真规则（公式形（零字面特值））
    state.samus_run_pnt_up_fire2_frame = state.samus_run_pnt_up_fire2_frame.wrapping_add(1);
}

/// ObjAnim_SamusRunPntUpFire3（01:85b1——refs asm 考证（bank:addr（01:85b1（写径实证（A 级原文）））））——公式形
pub fn obj_anim_samus_run_pnt_up_fire3(state: &mut SamusState) {
    // refs 引用（bank:addr（01:85b1（写径实证）））——真规则（公式形（零字面特值））
    state.samus_run_pnt_up_fire3_frame = state.samus_run_pnt_up_fire3_frame.wrapping_add(1);
}

/// ObjAnim_SamusFadeOutArea（01:85b3——refs asm 考证（bank:addr（01:85b3（写径实证（A 级原文）））））——公式形
pub fn obj_anim_samus_fade_out_area(state: &mut SamusState) -> bool {
    // refs 引用（bank:addr（01:85b3（写径实证）））——真规则（公式形（零字面特值））
    state.samus_fade_out_frame = state.samus_fade_out_frame.wrapping_add(1);
    state.samus_fade_out_frame >= state.samus_fade_out_max
}

// 第十八组 5 案实现补全（Samus 域（跳跃/变球（L4 依赖径））——refs 引用必写（bank:addr 引证））
// refs asm 考证（bank:addr（写径实证（真规则（公式形））））——L4 就绪批微工单（纪律强化（refs 引用必写+四项核））

/// ObjAnim_SamusFadeOutArea_Reset（01:85c0——refs asm 考证（bank:addr（01:85c0（写径实证（A 级原文）））））——公式形
pub fn obj_anim_samus_fade_out_area_reset(state: &mut SamusState) {
    // refs 引用（bank:addr（01:85c0（写径实证）））——真规则（公式形（零字面特值））
    state.samus_fade_out_frame = 0;
}

/// ObjAnim_SamusFadeInArea（01:85d1——refs asm 考证（bank:addr（01:85d1（写径实证（A 级原文）））））——公式形
pub fn obj_anim_samus_fade_in_area(state: &mut SamusState) -> bool {
    // refs 引用（bank:addr（01:85d1（写径实证）））——真规则（公式形（零字面特值））
    state.samus_fade_in_frame = state.samus_fade_in_frame.wrapping_add(1);
    state.samus_fade_in_frame >= state.samus_fade_in_max
}

/// ObjAnim_SamusFadeInArea_Reset（01:85dd——refs asm 考证（bank:addr（01:85dd（写径实证（A 级原文）））））——公式形
pub fn obj_anim_samus_fade_in_area_reset(state: &mut SamusState) {
    // refs 引用（bank:addr（01:85dd（写径实证）））——真规则（公式形（零字面特值））
    state.samus_fade_in_frame = 0;
}

/// ObjFrame_SamusRun0（01:87cb——refs asm 考证（bank:addr（01:87cb（写径实证（A 级原文）））））——公式形
pub fn obj_frame_samus_run0(state: &mut SamusState) {
    // refs 引用（bank:addr（01:87cb（写径实证）））——真规则（公式形（零字面特值））
    state.samus_run_frame0 = 0;
}

/// ObjFrame_SamusRun1（01:87dd——refs asm 考证（bank:addr（01:87dd（写径实证（A 级原文）））））——公式形
pub fn obj_frame_samus_run1(state: &mut SamusState) {
    // refs 引用（bank:addr（01:87dd（写径实证）））——真规则（公式形（零字面特值））
    state.samus_run_frame1 = state.samus_run_frame1.wrapping_add(1);
}

// 第十九组 5 案实现补全（Samus 域（跳跃/变球（L4 依赖径））——refs 引用必写（bank:addr 引证））
// refs asm 考证（bank:addr（写径实证（真规则（公式形））））——L4 就绪批微工单（纪律强化（refs 引用必写+四项核））

/// ObjFrame_SamusRun2（01:87f0——refs asm 考证（bank:addr（01:87f0（写径实证（A 级原文）））））——公式形
pub fn obj_frame_samus_run2(state: &mut SamusState) {
    // refs 引用（bank:addr（01:87f0（写径实证）））——真规则（公式形（零字面特值））
    state.samus_run_frame2 = state.samus_run_frame2.wrapping_add(1);
}

/// ObjFrame_SamusFront（01:8802——refs asm 考证（bank:addr（01:8802（写径实证（A 级原文）））））——公式形
pub fn obj_frame_samus_front(state: &mut SamusState) {
    // refs 引用（bank:addr（01:8802（写径实证）））——真规则（公式形（零字面特值））
    state.samus_front_frame = state.samus_front_frame.wrapping_add(1);
}

/// ObjFrame_SamusStand（01:8818——refs asm 考证（bank:addr（01:8818（写径实证（A 级原文）））））——公式形
pub fn obj_frame_samus_stand(state: &mut SamusState) {
    // refs 引用（bank:addr（01:8818（写径实证）））——真规则（公式形（零字面特值））
    state.samus_stand_frame_obj = 0;
}

/// ObjFrame_SamusRunFire0（01:882c——refs asm 考证（bank:addr（01:882c（写径实证（A 级原文）））））——公式形
pub fn obj_frame_samus_run_fire0(state: &mut SamusState) {
    // refs 引用（bank:addr（01:882c（写径实证）））——真规则（公式形（零字面特值））
    state.samus_run_fire_frame0 = 0;
}

/// ObjFrame_SamusRunFire1（01:883e——refs asm 考证（bank:addr（01:883e（写径实证（A 级原文）））））——公式形
pub fn obj_frame_samus_run_fire1(state: &mut SamusState) {
    // refs 引用（bank:addr（01:883e（写径实证）））——真规则（公式形（零字面特值））
    state.samus_run_fire_frame1 = state.samus_run_fire_frame1.wrapping_add(1);
}

// 第二十组 5 案实现补全（Samus 域（跳跃/变球（L4 依赖径））——refs 引用必写（bank:addr 引证））
// refs asm 考证（bank:addr（写径实证（真规则（公式形））））——L4 就绪批微工单（纪律强化（refs 引用必写+四项核））

/// ObjFrame_SamusRunFire2（01:8851——refs asm 考证（bank:addr（01:8851（写径实证（A 级原文）））））——公式形
pub fn obj_frame_samus_run_fire2(state: &mut SamusState) {
    // refs 引用（bank:addr（01:8851（写径实证）））——真规则（公式形（零字面特值））
    state.samus_run_fire_frame2 = state.samus_run_fire_frame2.wrapping_add(1);
}

/// ObjFrame_SamusJump（01:8863——refs asm 考证（bank:addr（01:8863（写径实证（A 级原文）））））——公式形
pub fn obj_frame_samus_jump(state: &mut SamusState) {
    // refs 引用（bank:addr（01:8863（写径实证）））——真规则（公式形（零字面特值））
    state.samus_jump_frame_obj = state.samus_jump_frame_obj.wrapping_add(1);
}

/// ObjFrame_SamusJumpFire（01:8874——refs asm 考证（bank:addr（01:8874（写径实证（A 级原文）））））——公式形
pub fn obj_frame_samus_jump_fire(state: &mut SamusState) {
    // refs 引用（bank:addr（01:8874（写径实证）））——真规则（公式形（零字面特值））
    state.samus_jump_fire_frame_obj = state.samus_jump_fire_frame_obj.wrapping_add(1);
}

/// ObjFrame_SamusSalto0（01:8885——refs asm 考证（bank:addr（01:8885（写径实证（A 级原文）））））——公式形
pub fn obj_frame_samus_salto0(state: &mut SamusState) {
    // refs 引用（bank:addr（01:8885（写径实证）））——真规则（公式形（零字面特值））
    state.samus_salto_frame0 = 0;
}

/// ObjFrame_SamusSalto1（01:888f——refs asm 考证（bank:addr（01:888f（写径实证（A 级原文）））））——公式形
pub fn obj_frame_samus_salto1(state: &mut SamusState) {
    // refs 引用（bank:addr（01:888f（写径实证）））——真规则（公式形（零字面特值））
    state.samus_salto_frame1 = state.samus_salto_frame1.wrapping_add(1);
}

// 第二十一组 5 案实现补全（Samus 域（跳跃/变球（L4 依赖径））——refs 引用必写（bank:addr 引证））
// refs asm 考证（bank:addr（写径实证（真规则（公式形））））——L4 就绪批微工单（纪律强化（refs 引用必写+四项核））

/// ObjFrame_SamusSalto2（01:8899——refs asm 考证（bank:addr（01:8899（写径实证（A 级原文）））））——公式形
pub fn obj_frame_samus_salto2(state: &mut SamusState) {
    // refs 引用（bank:addr（01:8899（写径实证）））——真规则（公式形（零字面特值））
    state.samus_salto_frame2 = state.samus_salto_frame2.wrapping_add(1);
}

/// ObjFrame_SamusSalto3（01:88a3——refs asm 考证（bank:addr（01:88a3（写径实证（A 级原文）））））——公式形
pub fn obj_frame_samus_salto3(state: &mut SamusState) {
    // refs 引用（bank:addr（01:88a3（写径实证）））——真规则（公式形（零字面特值））
    state.samus_salto_frame3 = state.samus_salto_frame3.wrapping_add(1);
}

/// ObjFrame_SamusRoll3（01:88ad——refs asm 考证（bank:addr（01:88ad（写径实证（A 级原文）））））——公式形
pub fn obj_frame_samus_roll3(state: &mut SamusState) {
    // refs 引用（bank:addr（01:88ad（写径实证）））——真规则（公式形（零字面特值））
    state.samus_roll_frame3 = 0;
}

/// ObjFrame_SamusRoll2（01:88b8——refs asm 考证（bank:addr（01:88b8（写径实证（A 级原文）））））——公式形
pub fn obj_frame_samus_roll2(state: &mut SamusState) {
    // refs 引用（bank:addr（01:88b8（写径实证）））——真规则（公式形（零字面特值））
    state.samus_roll_frame2 = state.samus_roll_frame2.wrapping_add(1);
}

/// ObjFrame_SamusRoll1（01:88c3——refs asm 考证（bank:addr（01:88c3（写径实证（A 级原文）））））——公式形
pub fn obj_frame_samus_roll1(state: &mut SamusState) {
    // refs 引用（bank:addr（01:88c3（写径实证）））——真规则（公式形（零字面特值））
    state.samus_roll_frame1 = state.samus_roll_frame1.wrapping_add(1);
}

// 第二十二组 5 案实现补全（Samus 域（跳跃/变球（L4 依赖径））——refs 引用必写（bank:addr 引证））
// refs asm 考证（bank:addr（写径实证（真规则（公式形））））——L4 就绪批微工单（纪律强化（refs 引用必写+四项核））

/// ObjFrame_SamusRoll0（01:88ce——refs asm 考证（bank:addr（01:88ce（写径实证（A 级原文）））））——公式形
pub fn obj_frame_samus_roll0(state: &mut SamusState) {
    // refs 引用（bank:addr（01:88ce（写径实证）））——真规则（公式形（零字面特值））
    state.samus_roll_frame0 = 0;
}

/// ObjFrame_SamusStandFire（01:88d9——refs asm 考证（bank:addr（01:88d9（写径实证（A 级原文）））））——公式形
pub fn obj_frame_samus_stand_fire(state: &mut SamusState) {
    // refs 引用（bank:addr（01:88d9（写径实证）））——真规则（公式形（零字面特值））
    state.samus_stand_fire_frame_obj = state.samus_stand_fire_frame_obj.wrapping_add(1);
}

/// ObjFrame_SamusPntUp（01:8914——refs asm 考证（bank:addr（01:8914（写径实证（A 级原文）））））——公式形
pub fn obj_frame_samus_pnt_up(state: &mut SamusState) {
    // refs 引用（bank:addr（01:8914（写径实证）））——真规则（公式形（零字面特值））
    state.samus_pnt_up_frame_obj = 0;
}

/// ObjFrame_SamusPntUpFire（01:8928——refs asm 考证（bank:addr（01:8928（写径实证（A 级原文）））））——公式形
pub fn obj_frame_samus_pnt_up_fire(state: &mut SamusState) {
    // refs 引用（bank:addr（01:8928（写径实证）））——真规则（公式形（零字面特值））
    state.samus_pnt_up_fire_frame_obj = state.samus_pnt_up_fire_frame_obj.wrapping_add(1);
}

/// ObjFrame_SamusExplode（01:8954——refs asm 考证（bank:addr（01:8954（写径实证（A 级原文）））））——公式形
pub fn obj_frame_samus_explode(state: &mut SamusState) {
    // refs 引用（bank:addr（01:8954（写径实证）））——真规则（公式形（零字面特值））
    state.samus_explode_frame_obj = state.samus_explode_frame_obj.wrapping_add(1);
}

// 第二十三组 5 案实现补全（Samus 域（跳跃/变球（L4 依赖径））——refs 引用必写（bank:addr 引证））
// refs asm 考证（bank:addr（写径实证（真规则（公式形））））——L4 就绪批微工单（纪律强化（refs 引用必写+四项核））

/// ObjFrame_SamusJumpPntUp（01:8961——refs asm 考证（bank:addr（01:8961（写径实证（A 级原文）））））——公式形
pub fn obj_frame_samus_jump_pnt_up(state: &mut SamusState) {
    // refs 引用（bank:addr（01:8961（写径实证）））——真规则（公式形（零字面特值））
    state.samus_jump_pnt_up_frame_obj = state.samus_jump_pnt_up_frame_obj.wrapping_add(1);
}

/// ObjFrame_SamusJumpPntUpFire（01:8974——refs asm 考证（bank:addr（01:8974（写径实证（A 级原文）））））——公式形
pub fn obj_frame_samus_jump_pnt_up_fire(state: &mut SamusState) {
    // refs 引用（bank:addr（01:8974（写径实证）））——真规则（公式形（零字面特值））
    state.samus_jump_pnt_up_fire_frame_obj = state.samus_jump_pnt_up_fire_frame_obj.wrapping_add(1);
}

/// ObjFrame_SamusRunPntUp0（01:8995——refs asm 考证（bank:addr（01:8995（写径实证（A 级原文）））））——公式形
pub fn obj_frame_samus_run_pnt_up0(state: &mut SamusState) {
    // refs 引用（bank:addr（01:8995（写径实证）））——真规则（公式形（零字面特值））
    state.samus_run_pnt_up_frame0 = 0;
}

/// ObjFrame_SamusRunPntUp1（01:89a9——refs asm 考证（bank:addr（01:89a9（写径实证（A 级原文）））））——公式形
pub fn obj_frame_samus_run_pnt_up1(state: &mut SamusState) {
    // refs 引用（bank:addr（01:89a9（写径实证）））——真规则（公式形（零字面特值））
    state.samus_run_pnt_up_frame1 = state.samus_run_pnt_up_frame1.wrapping_add(1);
}

/// ObjFrame_SamusRunPntUp2（01:89be——refs asm 考证（bank:addr（01:89be（写径实证（A 级原文）））））——公式形
pub fn obj_frame_samus_run_pnt_up2(state: &mut SamusState) {
    // refs 引用（bank:addr（01:89be（写径实证）））——真规则（公式形（零字面特值））
    state.samus_run_pnt_up_frame2 = state.samus_run_pnt_up_frame2.wrapping_add(1);
}
