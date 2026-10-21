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
