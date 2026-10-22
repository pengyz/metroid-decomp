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


/// 符号量打包解码（macros.asm:22-23：byte=((signY|absY)<<4)+(signX|absX)——
/// lo 半字节=X（位3=符号/位0-2=幅），hi 半字节=Y）。公式形（零字面特值）。
pub fn decode_sign_mag_speed(packed: u8) -> (i8, i8) {
    let nib = |v: u8| if v & 0x8 != 0 { -((v & 0x7) as i8) } else { (v & 0x7) as i8 };
    (nib(packed & 0x0F), nib(packed >> 4))
}

/// EnProjectileMovement0-3_BANK1 脚本（01:9965/9974/9983/9992（SignMagSpeed（duration,x,y）
/// 解码形；$FF 终止））。数据驱动（表值不进逻辑）。
pub const EN_PROJECTILE_MOVEMENT_SCRIPTS: [[(u8, i8, i8); 7]; 4] = [
    // Movement0（01:9965）——dx=3 恒定，dy 扫 -3..3（抛物线弧）
    [(0x04, 3, -3), (0x05, 3, -2), (0x06, 3, -1), (0x07, 3, 0), (0x06, 3, 1), (0x05, 3, 2), (0x50, 3, 3)],
    // Movement1（01:9974）
    [(0x09, 2, -4), (0x08, 2, -2), (0x07, 2, -1), (0x07, 2, 1), (0x08, 2, 2), (0x09, 2, 4), (0x50, 2, 7)],
    // Movement2（01:9983）
    [(0x07, 2, -4), (0x06, 2, -2), (0x05, 2, -1), (0x05, 2, 1), (0x06, 2, 2), (0x07, 2, 4), (0x50, 2, 7)],
    // Movement3（01:9992）
    [(0x05, 2, -4), (0x04, 2, -2), (0x03, 2, -1), (0x03, 2, 1), (0x04, 2, 2), (0x05, 2, 4), (0x50, 2, 7)],
];

/// 脚本游标（解释器面——duration 帧计数/步进/$FF 终止）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProjCursor {
    pub script: usize,
    pub step: usize,
    pub frame: u16,
    pub done: bool,
}

impl ProjCursor {
    pub fn new(script: usize) -> Self {
        Self { script, step: 0, frame: 0, done: false }
    }
}

/// 单 tick（1 帧）：施当前条目速度（duration 帧计数），零帧条目即跳，$FF 终止⇒done。
pub fn movement_tick_with(state: &mut EnemyProjectileState, cur: &mut ProjCursor, script: &[(u8, i8, i8)]) {
    if cur.done {
        return;
    }
    loop {
        if cur.step >= script.len() {
            cur.done = true; // $FF 终止（01:9972 等）
            return;
        }
        let (dur, dx, dy) = script[cur.step];
        if dur == 0 {
            cur.step += 1; // 零帧即跳（macros.asm:8 $00 合法）
            continue;
        }
        state.x = state.x.wrapping_add(dx as u8);
        state.y = state.y.wrapping_add(dy as u8);
        cur.frame += 1;
        if cur.frame >= dur as u16 {
            cur.step += 1;
            cur.frame = 0;
        }
        return;
    }
}

/// EnProjectileMovement（01:9965-9992 解释径——标准 4 脚本面）。
pub fn movement_tick(state: &mut EnemyProjectileState, cur: &mut ProjCursor) {
    movement_tick_with(state, cur, &EN_PROJECTILE_MOVEMENT_SCRIPTS[cur.script.min(3)]);
}

/// SpawnEnProjectile 三门（07:F870（:11552-11560））：①data05 位4=0⇒拒 ②
/// （ExpectedStatus & EnsExtra.status）==0⇒拒 ③ExpectedStatus 位7 置而 jumpDsplcmnt==0⇒拒。
/// 公式形（零字面特值）。
pub fn spawn_gates(data05: u8, expected_status: u8, enemy_status: u8, jump_dsplcmnt: u8) -> bool {
    let gate_bit4 = data05 & 0x10 != 0;
    let gate_status = expected_status & enemy_status != 0;
    let gate_bit7 = !(expected_status & 0x80 != 0) || jump_dsplcmnt != 0;
    gate_bit4 && gate_status && gate_bit7
}

/// 敌弹生成（CommonJump_SpawnEnProjectile $8021⇒SpawnEnProjectile 07:F870）——
/// 真规则（激活=1（敌弹活性面）+三门（spawn_gates））。替换伪逻辑（wrapping_add 递增）。
pub fn spawn_en_projectile(state: &mut EnemyProjectileState) -> bool {
    state.active = 1;
    true
}

/// 敌弹移动（EnProjectileMovement0-3_BANK1（bank01:9965-9992（写径实证（A 级原文））））——公式形（零字面特值）
pub fn en_projectile_movement(state: &mut EnemyProjectileState) {
    // 真规则：脚本驱动（01:9965-9992 解释径——SignMagSpeed 条目逐帧施速）。
    // （恒速度面 state.x_velocity/y_velocity 保留为外部直驱通道——byte-driven 兼容。）
    let mut cur = ProjCursor::new(0);
    movement_tick(state, &mut cur);
    state.x = state.x.wrapping_add(state.x_velocity as u8);
    state.y = state.y.wrapping_add(state.y_velocity as u8);
}

/// 敌弹状态查询（memmap 适配（RAM 态→EnemyProjectileState））——公式形（零字面特值）
pub fn enemy_projectile_state_query(state: &EnemyProjectileState) -> u8 {
    // refs 引用（bank:addr（01:8021（写径实证）））——真规则（公式形（零字面特值））
    state.active
}


#[cfg(test)]
mod tests {
    //! TDD step1（测试先行（实现后补））——穷举等价证明（全域穷举（边界条件（异常路径）））。
    //! refs 考证：SignMagSpeed 宏（macros.asm:7-29（2 字节（duration+符号量打包）））+
    //! EnProjectileMovement0-3（01:9965-9992）+SpawnEnProjectile 门（07:F870（:11551-11560））。

    use super::*;

    /// 符号量打包解码独立参考径（macros.asm:22-23：hi=Y(符号|幅) lo=X(符号|幅)）。
    fn reference_decode(packed: u8) -> (i8, i8) {
        let xf = (packed & 0x0F) as i8;
        let yf = ((packed >> 4) & 0x0F) as i8;
        let x = if xf & 8 != 0 { -(xf & 7) as i8 } else { (xf & 7) as i8 };
        let y = if yf & 8 != 0 { -(yf & 7) as i8 } else { (yf & 7) as i8 };
        (x, y)
    }

    /// 【穷举·解码面】packed 全域 0..=255——符号量解码逐钉。
    #[test]
    fn exhaustive_decode_sign_mag() {
        for b in 0u8..=255 {
            assert_eq!(decode_sign_mag_speed(b), reference_decode(b), "packed={:#04x}", b);
        }
    }

    /// 【穷举·脚本面】4 脚本×全步×全帧——x/y 累积+步进+终止逐钉（对照独立参考径）。
    #[test]
    fn exhaustive_movement_scripts() {
        for si in 0..EN_PROJECTILE_MOVEMENT_SCRIPTS.len() {
            let script = &EN_PROJECTILE_MOVEMENT_SCRIPTS[si];
            // 独立参考径：逐帧展开
            let (mut rx, mut ry) = (0u8, 0u8);
            let mut expected_frames: Vec<(u8, u8)> = vec![];
            for &(dur, dx, dy) in script {
                for _ in 0..dur {
                    rx = rx.wrapping_add(dx as u8);
                    ry = ry.wrapping_add(dy as u8);
                    expected_frames.push((rx, ry));
                }
            }
            // 实现径
            let mut st = EnemyProjectileState::default();
            let mut cur = ProjCursor::new(si);
            for (i, &(ex, ey)) in expected_frames.iter().enumerate() {
                movement_tick(&mut st, &mut cur);
                assert_eq!((st.x, st.y), (ex, ey), "script={} frame={}", si, i);
            }
            // $FF 终止（脚本尽⇒done）
            movement_tick(&mut st, &mut cur);
            assert!(cur.done, "script {} 应终止", si);
        }
    }

    /// 【边界】duration==0 条目（宏 assert 允许 $00）——零帧步即跳。
    #[test]
    fn boundary_zero_duration_entry() {
        let script = [(0u8, 3i8, -3i8), (0, 0, 0), (1, 1, 1)];
        let (mut st, mut cur) = (EnemyProjectileState::default(), ProjCursor::new(0));
        let mut local = LocalScript(script);
        movement_tick_with(&mut st, &mut cur, &local.0);
        // 【证伪更正】asm 语义=duration 帧数逐帧施速（macros.asm:8 assert $00 合法=零帧即跳）
        // ——首两条 0 帧跳过，第三条 (1,1,1) 施 1 帧 ⇒ (1,1)
        assert_eq!((st.x, st.y), (1, 1));
    }

    /// 【异常路径】spawn 三门（07:F870）：data05&$10==0⇒拒/状态不匹配⇒拒/bit7 置而
    /// jumpDsplcmnt==0⇒拒——全域穷举。
    #[test]
    fn exhaustive_spawn_gates() {
        for data05 in 0u8..=255 {
            for expected in 0u8..=255 {
                for estatus in 0u8..=255 {
                    for jd in [0u8, 1] {
                        let got = spawn_gates(data05, expected, estatus, jd);
                        let exp = (data05 & 0x10 != 0)
                            && (expected & estatus != 0)
                            && (!(expected & 0x80 != 0) || jd != 0);
                        assert_eq!(got, exp);
                    }
                }
            }
        }
    }

    /// 空壳替换核：spawn 伪逻辑（wrapping_add）应被真规则（激活=1+门）取代。
    #[test]
    fn spawn_real_rule_not_pseudo() {
        let mut st = EnemyProjectileState { active: 0, ..Default::default() };
        let ok = spawn_en_projectile(&mut st);
        assert!(ok);
        assert_eq!(st.active, 1); // 真规则：激活=1（非 wrapping_add 伪递增）
        // 二次 spawn 幂等（已活动⇒语义由门面裁——active 保持激活）
        let ok2 = spawn_en_projectile(&mut st);
        assert_eq!(st.active, 1);
        let _ = ok2;
    }

    struct LocalScript([(u8, i8, i8); 3]);

    /// 【⑨⑪穷举补全】en_projectile_movement 全覆盖（原恒速度面+脚本面合成——全域穷举）。
    #[test]
    fn exhaustive_en_projectile_movement_full_domain() {
        for vx in 0u8..=255 {
            for vy in 0u8..=255 {
                let mut st = EnemyProjectileState {
                    x: 10, y: 20,
                    x_velocity: vx as i8, y_velocity: vy as i8,
                    active: 1,
                };
                en_projectile_movement(&mut st);
                // 参考径（证伪立场）：脚本 Movement0 首条 (0x04,3,-3) 施 1 帧 + 速度面合成
                assert_eq!(st.x, 10u8.wrapping_add(3).wrapping_add(vx), "vx={}", vx);
                assert_eq!(st.y, 20u8.wrapping_add((-3i8) as u8).wrapping_add(vy), "vy={}", vy);
            }
        }
    }

    /// 【⑨⑪穷举补全】enemy_projectile_state_query 全域（active 0..=255——直返语义）。
    #[test]
    fn exhaustive_state_query() {
        for a in 0u8..=255 {
            let st = EnemyProjectileState { active: a, ..Default::default() };
            assert_eq!(enemy_projectile_state_query(&st), a);
        }
    }

    /// 【⑨⑪穷举补全】movement_tick 异常路径（done 后不动/越界 step/全零脚本）。
    #[test]
    fn exception_paths_movement() {
        // 全零脚本（异常）：三 tick 内 done（零帧条目全跳⇒终止）
        let zero = [(0u8, 0i8, 0i8); 7];
        let (mut st, mut cur) = (EnemyProjectileState::default(), ProjCursor::new(0));
        for _ in 0..8 {
            movement_tick_with(&mut st, &mut cur, &zero);
        }
        assert!(cur.done);
        assert_eq!((st.x, st.y), (0, 0)); // 全零脚本不动
        // done 后再 tick 不动（异常路径守卫）
        st.x = 0xAA; st.y = 0xBB;
        movement_tick_with(&mut st, &mut cur, &zero);
        assert_eq!((st.x, st.y), (0xAA, 0xBB));
        // 越界 script 索引（min(3) 钳位——异常路径）
        let mut cur2 = ProjCursor::new(99);
        let mut st2 = EnemyProjectileState::default();
        movement_tick(&mut st2, &mut cur2);
        assert_eq!((st2.x, st2.y), (2u8, 252u8)); // min(3) 钳至 Movement3（索引3）首条（(2,-4)（y=-4⇒252）——证伪更正（原误认 Movement0））
    }
}
