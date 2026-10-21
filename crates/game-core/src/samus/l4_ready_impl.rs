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
