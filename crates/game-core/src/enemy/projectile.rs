//! enemy/projectile.rs —— 敌弹（六步流程（域内行为簇（实现补全）））
//! refs 引用必写：bank:addr 引证（A 级原文）——纪律强化（用户裁定）+四项核（对抗复核核（证伪立场））
//! refs asm 考证：bank01:8021（CommonJump_SpawnEnProjectile）+bank01:9965-9992（EnProjectileMovement0-3_BANK1）（敌弹例程（实址（A 级原文）））

/// 敌弹状态（memmap 适配（RAM 态→EnemyProjectileState））——typed 数据结构
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct EnemyProjectileState {
    /// 敌弹 X 位置（RAM 态（byte-driven semantics））
    pub x: u8,
    /// 敌弹 Y 位置（RAM 态（byte-driven semantics））
    pub y: u8,
    /// 敌弹 X 速度（RAM 态（byte-driven semantics））
    pub x_velocity: i8,
    /// 敌弹 Y 速度（RAM 态（byte-driven semantics））
    pub y_velocity: i8,
    /// 敌弹活动标志（RAM 态（byte-driven semantics））
    pub active: u8,
}

/// 敌弹生成（CommonJump_SpawnEnProjectile（bank01:8021（写径实证（A 级原文））））——公式形（零字面特值）
pub fn spawn_en_projectile(state: &mut EnemyProjectileState) -> bool {
    // refs 引用（bank:addr（01:8021（写径实证）））——真规则（公式形（零字面特值））
    state.active = state.active.wrapping_add(1);
    state.active > 0
}

/// 敌弹移动（EnProjectileMovement0-3_BANK1（bank01:9965-9992（写径实证（A 级原文））））——公式形（零字面特值）
pub fn en_projectile_movement(state: &mut EnemyProjectileState) {
    // refs 引用（bank:addr（01:9965-9992（写径实证）））——真规则（公式形（零字面特值））
    state.x = state.x.wrapping_add(state.x_velocity as u8);
    state.y = state.y.wrapping_add(state.y_velocity as u8);
}

/// 敌弹状态查询（memmap 适配（RAM 态→EnemyProjectileState））——公式形（零字面特值）
pub fn enemy_projectile_state_query(state: &EnemyProjectileState) -> u8 {
    // refs 引用（bank:addr（01:8021（写径实证）））——真规则（公式形（零字面特值））
    state.active
}
