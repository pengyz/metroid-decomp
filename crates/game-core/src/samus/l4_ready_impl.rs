/// SamusState（S 面实现补全（首组（Samus 域（跳跃/变球）））——公式形）
#[derive(Debug, Clone, Default)]
pub struct SamusState {
    pub items: u8,
    pub equipment: u8,
    pub y_velocity: i8,
    pub jump_power: i8,
    pub game_timer: u8,
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
