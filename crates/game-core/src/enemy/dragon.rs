//! Dragon（熔岩海马）—— `refs/m1disasm/SRC/enemies/dragon.asm`（88 行逐行对读）。
//!
//! **只有 Norfair include 本件**：`refs/m1disasm/SRC/prg2_norfair.asm:683
//! .include "enemies/dragon.asm"`（全仓仅此一处，`grep -n "dragon.asm" refs/m1disasm/SRC/prg*.asm`
//! 只回这一行）⇒ `{AREA}` 恒为 NORFAIR。因此本文件里的四个动画字面量是
//! **Norfair `EnAnimTable_{AREA}`（`02:9BDA`）的偏移**，绝不可拿到别区复用
//! （`anim_idx` 的按区类型就是为了拦这件事）。
//!
//! asm 原文（bank2 `02:9AE9`）：
//! ```text
//! DragonAIRoutine_{AREA}: ; 02:9AE9
//!     ; branch if not resting
//!     lda EnsExtra.0.status,x / cmp #enemyStatus_Resting / bne @endIf_A
//!         ; enemy is resting / set position to under lava
//!         lda #$E8 / sta Ens.0.y,x
//!     @endIf_A: ; 02:9AF5
//!     ; exit if not active
//!     cmp #enemyStatus_Active / bne @exit_initAnim
//!
//!     ; exit if on the first movement instruction (rising from the lava)
//!     lda Ens.0.movementInstrIndex,x / beq @exit_initAnim
//!     ; exit if moving up or down
//!     lda EnsExtra.0.jumpDsplcmnt,x / bne @exit_initAnim
//!
//!     ; shoot every 32 frames
//!     lda FrameCount / and #$1F / bne @endIf_B
//!         ; branch if misfired (25% random chance)
//!         lda RandomNumber1 / and #$03 / beq @exit_playAnim
//!         ; shoot dragon enProjectile / set expected status to #$02
//!         lda #enemyStatus_Active / sta SpawnEnProjectile_ExpectedStatus
//!         ; set movement to EnProjectileMovement0
//!         lda #$00 / sta SpawnEnProjectile_EnData0A
//!         ; shooting animation (supposed to be parameter to SpawnEnProjectile, but unused there)
//!         lda #EnAnim_DragonIdle_R_{AREA} - EnAnimTable_{AREA}.b / sta SpawnEnProjectile_AnimIndex
//!         lda #EnAnim_DragonIdle_L_{AREA} - EnAnimTable_{AREA}.b / sta SpawnEnProjectile_AnimIndex+1
//!         ; set projectile animation
//!         lda #$03 / sta SpawnEnProjectile_AnimTableIndex
//!         ; spawn projectile
//!         jsr CommonJump_SpawnEnProjectile
//!         ; play shoot sfx
//!         lda SFXNoiseInitFlags / ora #sfxNoise_SpitFlame / sta SFXNoiseInitFlags
//!         ; set shooting animation
//!         lda Ens.0.data05,x / and #$01 / tay
//!         lda SpawnEnProjectile_AnimIndex,y / jsr CommonJump_InitEnAnimIndex
//!         beq @exit_playAnim ; branch always
//!     @endIf_B: ; 02:9B3C
//!     ; set "prepare to spit" animation 15 frames after having shot
//!     cmp #$0F / bcc @exit_playAnim
//!     lda Ens.0.data05,x / and #$01 / tay
//!     lda @prepareToSpitEnAnimTable,y / jsr CommonJump_InitEnAnimIndex
//!     jmp @exit_playAnim
//!
//! @exit_initAnim: ; 02:9B4F
//!     lda EnsExtra.0.status,x / cmp #enemyStatus_Explode / beq @endIf_C
//!         ; enemy is not exploding, set animation to active
//!         jsr CommonJump_InitEnActiveAnimIndex_NoInitOffset
//!     @endIf_C: ; 02:9B59
//! @exit_playAnim: ; 02:9B59
//!     ; change animation frame every frame
//!     lda #$01 / sta $00 / sta $01
//!     jmp UpdateEnemyCommon_Decide_{AREA}
//!
//! @prepareToSpitEnAnimTable: ; 02:9B62
//!     .byte EnAnim_DragonPrepareToSpit_R_{AREA} - EnAnimTable_{AREA}, EnAnim_DragonPrepareToSpit_L_{AREA} - EnAnimTable_{AREA}
//! ```
//!
//! 语义要点（易错处 / 反直觉处）：
//! - **`cmp #enemyStatus_Active`（`02:9AF5`）比较的是 A，不是 status**：Resting 径刚执行过
//!   `lda #$E8 / sta y`，A 恒 `$E8` ⇒ 该 `cmp` 对 Resting 敌永假。行为上等价于「status≠2
//!   就跳到 `@exit_initAnim`」（`$E8` 不等于 2），但**不许**把它"顺手改成读 status"——那样
//!   一旦哪天 `$E8` 常量变了就会分叉。本件保留原形的判据（见 `dragon_ai` 里的注释）。
//! - **生弹结果无人分支**：`jsr CommonJump_SpawnEnProjectile` 之后 asm 直接放音效 + 设吐弹
//!   动画，**不看返回**。池满 / `data05` bit4 清（`07:F872` 首个门）/ status 不匹配时，
//!   弹体没生成，但音效与动画照置。忠实保留（`spawn_failure_still_stages_spit` 锚测）。
//! - **`SpawnEnProjectile_AnimIndex` 那对立即数在本次调用里其实没人读**：`EnProjectile` 只在
//!   `ExpectedStatus` bit6 置时才取它（`07:F80C`），而这里 `#$02` 没有 bit6。asm 注释亦言
//!   "supposed to be parameter ... but unused there"。本件仍按 asm 传真值（不是假装）。
//! - **`@endIf_B` 的 `cmp #$0F` 用的仍是 A=`FrameCount & $1F`**（生弹径已跳走）⇒
//!   `0x0F..=0x1F` 这 17 帧播"准备吐"动画，`0x01..=0x0E` 什么都不改。
//! - 四个出口（生弹 / 未命中 / 准备吐 / `@exit_initAnim`）**都**落到
//!   `UpdateEnemyCommon_Decide` 且 beats 是 `#$01/#$01` ⇒ 每帧推进动画帧（30fps 门由
//!   `Enemy82` 决定），没有任何本件私有的收尾动作。
//!
//! **ROM 逐字节交叉核对（A 级，2026-11-20）**：直接读 `rom/Metroid (USA).nes` 的 bank2
//! 镜像（文件偏移 `0x10 + 2*0x4000 + (addr - $8000)`）解出本件全部字节，与本文件常量一一对上：
//! - `02:9AE9 BD F4 6A`（`lda status,x`）、`9AF5 C9 02`（那个**比 A** 的 cmp）、
//!   `9B01 D0 4C`（jumpDsplcmnt 门）、`9B07 D0 33`（`bne → 9B3C`）、`9B0D F0 4A`
//!   （`beq → 9B59`）、`9B17 A9 43`·`9B1B A9 47`（`SpawnEnProjectile_AnimIndex` 那对立即数）、
//!   `9B1F A9 03`（AnimTableIndex）、`9B23 20 21 80`（`jsr CommonJump_SpawnEnProjectile`）、
//!   `9B29 09 04 / 8D 80 06`（SFX 位或）、`9B3E 90 19`（`bcc → 9B59`）、
//!   `9B4C 4C 59 9B` 与 `9B5F 4C 39 98`（两个 jmp 目标）、`9B62 45 49`（prepare 动画对）。
//! - `EnAnimTable_{AREA} + off` 实字节：`+$43→$30,$FF`、`+$45→$31,$FF`、`+$47→$32,$FF`、
//!   `+$49→$33,$FF`、`+$4B→$34,$FF`（四支 Dragon＋爆炸；`$FF` = `_id_FrameEnd`）。
//! - `02:978B` 上升动画表实字节 `00 00 00 00 4D 4D 53 57`（第 3 对 = `$53/$57` = Dragon 吐弹）；
//!   紧随其后的 `02:979B` 位置偏移 `08 F8 00 00 00 00 08 F8`、`02:97A3` `00 00 00 F8`。
//! - `9B3A F0 1D`（`beq → @exit_playAnim`，asm 注释写 "branch always"）**为什么恒跳**：
//!   `InitEnAnimIndex`（`07:F68D`）末件是 `A9 00`（`lda #$00 / sta animDelay,x`）⇒ 返回时
//!   A=0 ⇒ Z=1 ⇒ 必跳。本件因此把生弹径直接连到"播完吐弹动画即收尾"，不重走 `@endIf_B`
//!   （若哪天把 `init_anim_index` 换成返回值的形，这里就会静默走进 prepare 支——注释留因）。

use crate::enemy::en_projectile;
use crate::enemy::framework;
use crate::enemy::state::{Anim, slot_of};
use crate::sound::hook;

/// `enemyStatus_Resting` / `enemyStatus_Active` / `enemyStatus_Explode`（`constants.asm:135-142`）。
const ENEMY_RESTING: u8 = 1;
const ENEMY_ACTIVE: u8 = 2;
const ENEMY_EXPLODE: u8 = 3;

/// 熔岩面 y（asm `lda #$E8 / sta Ens.0.y,x`——代码立即数，与 Squeept 同源语义）。
const LAVA_Y: u8 = 0xE8;

/// 生弹周期掩码（asm `lda FrameCount / and #$1F`）⇒ 每 32 帧一次机会。
const SHOOT_PERIOD_MASK: u8 = 0x1F;

/// 末次吐弹后 `#$0F` 帧起播"准备吐"动画（asm `cmp #$0F / bcc`）。
const PREPARE_TO_SPIT_FROM: u8 = 0x0F;

/// 未命中概率（asm `lda RandomNumber1 / and #$03 / beq`）⇒ 25% 空放。
const MISFIRE_MASK: u8 = 0x03;

/// `sfxNoise_SpitFlame = %00000100`（`refs/m1disasm/SRC/constants.asm:150`）。
const SFX_NOISE_SPIT_FLAME: u8 = 0x04;

/// 弹体动画组（asm `lda #$03 / sta SpawnEnProjectile_AnimTableIndex`）：
/// `EnProjectileRisingAnimIndexTable`（`02:978B`）第 3 对＝Dragon 吐弹上行动画。
const EN_PROJECTILE_ANIM_TABLE_INDEX: u8 = 0x03;

/// 期望的射主状态（asm `lda #enemyStatus_Active`）——只放行 Active 射主。
const EXPECTED_STATUS: u8 = ENEMY_ACTIVE;

/// 弹体移动串号（asm `lda #$00 / sta SpawnEnProjectile_EnData0A`）＝`EnProjectileMovement0`。
const EN_DATA_0A: u8 = 0x00;

/// `EnAnim_DragonIdle_R - EnAnimTable = $02:9C1D - $02:9BDA`（`norfair/enemy_sprite_data.asm:115`）。
const ANIM_OFF_IDLE_R: u8 = 0x43;
/// `EnAnim_DragonIdle_L - EnAnimTable = $02:9C21 - $02:9BDA`（同上 `:123`）。
const ANIM_OFF_IDLE_L: u8 = 0x47;
/// `EnAnim_DragonPrepareToSpit_R - EnAnimTable = $02:9C1F - $02:9BDA`（同上 `:119`）。
const ANIM_OFF_PREPARE_R: u8 = 0x45;
/// `EnAnim_DragonPrepareToSpit_L - EnAnimTable = $02:9C23 - $02:9BDA`（同上 `:127`）。
const ANIM_OFF_PREPARE_L: u8 = 0x49;

/// `@prepareToSpitEnAnimTable`（`02:9B62`，按 `data05 & $01` 索引）。
const PREPARE_TO_SPIT_ANIM: [u8; 2] = [ANIM_OFF_PREPARE_R, ANIM_OFF_PREPARE_L];
/// `SpawnEnProjectile_AnimIndex`（`02:9B3F-9B46` 写入的一对，按 `data05 & $01` 索引）。
const IDLE_ANIM: [u8; 2] = [ANIM_OFF_IDLE_R, ANIM_OFF_IDLE_L];

/// `InitEnAnimIndex ; 07:F68D`＝`resetAnimIndex ← A / animIndex ← A / animDelay ← 0`
/// （`frame` 不动——`07:11290-11297` 只写 0x6AFA/0x6AF5/0x6AF6 三格）。
///
/// 与 `en_projectile` 里内联的同一动作同源；这里单独成函数是因为 Dragon 有两处调用点。
fn init_anim_index(mem: &mut crate::frame::Manager, x: u8, idx: u8) {
    let slot = slot_of(x);
    let frame = mem.enemy(slot).anim_frame();
    mem.enemy(slot).set_anim(Anim { frame, delay: 0, reset: idx, index: idx });
}

/// `DragonAIRoutine_{AREA}`（`enemies/dragon.asm`，仅 Norfair 生成）。
///
/// 与 asm 的出口结构一一对应：`shoot` / `misfire` / `prepare-to-spit` 三条径都直接
/// 落到 `@exit_playAnim`（`decide(#$01,#$01)`），只有 `@exit_initAnim` 先补一次
/// `InitEnActiveAnimIndex_NoInitOffset`。
pub(crate) fn dragon_ai(mem: &mut crate::frame::Manager, x: u8) {
    let slot = slot_of(x);
    let status = mem.enemy(slot).status_raw();

    // --- asm 首块：Resting ⇒ 沉到熔岩面下（下一帧的"从熔岩里升起"由此成立）---
    // `lda EnsExtra.0.status,x / cmp #enemyStatus_Resting / bne @endIf_A`
    let not_resting = status != ENEMY_RESTING;
    if !not_resting {
        // `lda #$E8 / sta Ens.0.y,x`（只写 y，不碰 hi/x/speed）
        mem.enemy(slot).set_y(LAVA_Y);
    }
    // `@endIf_A: cmp #enemyStatus_Active / bne @exit_initAnim`
    // 【asm 原形】Resting 径走到这里时 A=$E8（`lda #$E8` 的残留），该 cmp 实际比较的是
    // **$E8 vs 2** 而不是 status；结果与「status≠Active」完全一致（1≠2 亦然），故此处
    // 按等价判据书写，并保留这段说明防止后人"优化".
    let active = if not_resting { status == ENEMY_ACTIVE } else { LAVA_Y == ENEMY_ACTIVE };
    if active {
        // `lda Ens.0.movementInstrIndex,x / beq @exit_initAnim`——第一条移动指令
        // （`data06`＝movementInstrIndex 别名，见 `state::ens` 注释）期间不吐弹。
        // 【跨区变量】`movementInstrIndex` 的字段偏移在 `state::ens::DATA06`
        // （`bankvars` 门禁的 39 项按区变量之一，`02`/`05` bank 同名同位）。
        let rising_from_lava = mem.enemy(slot).data06() == 0;
        // `lda EnsExtra.0.jumpDsplcmnt,x / bne @exit_initAnim`——正在上下移动（jump
        // 位移非零）也不吐弹。
        let moving_vertically = mem.enemy(slot).jump_dsplcmnt() != 0;
        if !rising_from_lava && !moving_vertically {
            // `lda FrameCount / and #$1F / bne @endIf_B`——只有 `FrameCount % 32 == 0` 那一帧
            // 进入吐弹块；A 保留 `FrameCount & $1F` 供 `@endIf_B` 的 `cmp #$0F` 使用。
            let fc_phase = mem.frame_count() & SHOOT_PERIOD_MASK;
            if fc_phase == 0 {
                // `lda RandomNumber1 / and #$03 / beq @exit_playAnim`——25% 空放。
                if mem.random_number1() & MISFIRE_MASK != 0 {
                    // 四 zp 入参按 asm 现算：ExpectedStatus=#$02 / EnData0A=#$00 /
                    // AnimTableIndex=#$03 / AnimIndex=[Idle_R,Idle_L]。
                    let _spawned = en_projectile::spawn_en_projectile(
                        mem,
                        x,
                        EXPECTED_STATUS,
                        EN_PROJECTILE_ANIM_TABLE_INDEX,
                        EN_DATA_0A,
                        IDLE_ANIM,
                    );
                    // `lda SFXNoiseInitFlags / ora #sfxNoise_SpitFlame / sta`——放音效
                    // （`SetSFXNoiseInitFlag 07:CBA0` 的位或形）。
                    hook::set_sfx_noise_init(mem, SFX_NOISE_SPIT_FLAME);
                    // `lda data05,x / and #$01 / tay / lda AnimIndex,y / jsr InitEnAnimIndex`
                    // ——**无论生弹是否成功都执行**（asm 不分支；`_spawned` 故意丢弃）。
                    // 朝向重新读一次：SpawnEnProjectile 只写**弹体**槽的 data05
                    // （`07:F8F5 set_data05`），射主那格不变，重读与 asm 的 `lda data05,x` 同值。
                    let facing = mem.enemy(slot).data05() & 0x01;
                    init_anim_index(mem, x, IDLE_ANIM[facing as usize]);
                }
                // `beq @exit_playAnim ; branch always` ⇒ 无论生弹/空放都进 @exit_playAnim。
                framework::update_enemy_common_decide_with_beats(mem, x, 0x01, 0x01);
                return;
            }
            // `@endIf_B: cmp #$0F / bcc @exit_playAnim`——A 仍是 `FrameCount & $1F`：
            // `0x0F..=0x1F` 播"准备吐"动画，`0x01..=0x0E` 不动动画。
            if fc_phase >= PREPARE_TO_SPIT_FROM {
                let facing = mem.enemy(slot).data05() & 0x01;
                init_anim_index(mem, x, PREPARE_TO_SPIT_ANIM[facing as usize]);
            }
            framework::update_enemy_common_decide_with_beats(mem, x, 0x01, 0x01);
            return;
        }
    }

    // `@exit_initAnim: lda status / cmp #enemyStatus_Explode / beq @endIf_C`
    // ——爆炸中的敌不重置成活动动画（否则会把 Explode 帧序列打回 idle）。
    if status != ENEMY_EXPLODE {
        framework::init_en_active_anim_index_no_init_offset(mem, x);
    }
    // `@endIf_C / @exit_playAnim: lda #$01 / sta $00 / sta $01
    //  / jmp UpdateEnemyCommon_Decide_{AREA}`——beats(活动 1 / 休止 1)。
    framework::update_enemy_common_decide_with_beats(mem, x, 0x01, 0x01);
}

#[cfg(test)]
mod tests_dragon {
    use super::*;
    use crate::enemy::en_projectile::{SLOT_COUNT, base_slot};
    use crate::enemy::prep::ENS_F_TYPE;
    use crate::enemy::state::EnemyStatus;
    use crate::enemy::tables::ens_addr;
    use crate::frame::Manager;
    use crate::ram_map;

    const X: u8 = 0x20;
    const SLOT: u8 = X / 16;

    /// 夹具：Norfair + Dragon(type 0x0D) + 指定 status/相位。
    ///
    /// `data05` 的 bit4 是 `SpawnEnProjectile` 的第一道门（`07:F872`）——射主那格必须置着。
    /// 实机取值由 `UpdateEn_EnData05DistanceToSamusThreshold ; 07:F75B` 的距离掩码写
    /// （bits3/4 族），**本卡不断言其运行时取值**；夹具显式置 `$10` 打开该门。
    fn stage(m: &mut Manager, status: EnemyStatus, fc: u8, rng: u8, data05: u8) {
        game_data::pack::init_from_env().ok();
        m.write_ram(ram_map::IN_AREA, 1); // Norfair
        m.write_wram(ens_addr(X, ENS_F_TYPE), 0x0D); // Dragon
        m.write_ram(ram_map::FRAME_COUNT, fc);
        m.write_ram(ram_map::RANDOM1, rng);
        m.enemy(SLOT).set_status(status);
        m.enemy(SLOT).set_y(0x40);
        m.enemy(SLOT).set_pos_x(0x60);
        m.enemy(SLOT).set_data05(data05);
        m.enemy(SLOT).set_data06(0x01); // movementInstrIndex≠0
        m.enemy(SLOT).set_jump_dsplcmnt(0); // 未在上下移动
        m.enemy(SLOT).set_reset_anim_index(0x00);
        m.enemy(SLOT).set_anim_index(0x00);
    }

    /// 弹体池里"有弹"的槽号（`FindSlot` 从槽 6 起），无则 `None`。
    fn spawned_slot(m: &mut Manager) -> Option<u8> {
        (0..SLOT_COUNT).map(|i| base_slot(i)).find(|&s| m.enemy(s).status_raw() != 0)
    }

    /// **状态门（Resting）**：`lda #$E8 / sta Ens.0.y,x`——休止的 Dragon 被钉在熔岩面下，
    /// 且同帧不吐弹（哪怕 FrameCount 相位与随机数都满足）。
    #[test]
    fn resting_dragon_pins_y_under_lava_and_never_shoots() {
        let mut m = Manager::default();
        stage(&mut m, EnemyStatus::Resting, 0x00, 0x05, 0x10);
        m.enemy(SLOT).set_y(0x30);
        dragon_ai(&mut m, X);
        assert_eq!(m.enemy(SLOT).y(), LAVA_Y, "Resting ⇒ y 钉 $E8（熔岩面下）");
        assert_eq!(spawned_slot(&mut m), None, "Resting 不吐弹（asm cmp 之后跳到 @exit_initAnim）");
    }

    /// **状态门（Active 才进吐弹块）**：Explode 的 Dragon 不吐弹，且**不**重置活动动画
    /// （`cmp #enemyStatus_Explode / beq @endIf_C`）。
    #[test]
    fn exploding_dragon_skips_active_anim_init() {
        let mut m = Manager::default();
        stage(&mut m, EnemyStatus::Explode, 0x00, 0x05, 0x10);
        m.enemy(SLOT).set_reset_anim_index(0x4B); // EnAnim_DragonExplode
        dragon_ai(&mut m, X);
        assert_eq!(m.enemy(SLOT).reset_anim_index(), 0x4B, "爆炸中不回退成活动动画");
        assert_eq!(spawned_slot(&mut m), None);
    }

    /// **计时门**：只有 `FrameCount % 32 == 0` 才吐弹；相位 5 时连动画都不动
    /// （`cmp #$0F / bcc @exit_playAnim`）。
    #[test]
    fn shoot_timer_only_fires_on_32_frame_phase() {
        let mut m = Manager::default();
        stage(&mut m, EnemyStatus::Active, 0x05, 0x05, 0x10);
        dragon_ai(&mut m, X);
        assert_eq!(spawned_slot(&mut m), None, "相位 5 ⇒ 不吐弹");
        assert_eq!(m.enemy(SLOT).reset_anim_index(), 0x00, "相位 5 < $0F ⇒ 动画不动");

        let mut m = Manager::default();
        stage(&mut m, EnemyStatus::Active, 0x20, 0x05, 0x10); // 0x20 & $1F == 0
        dragon_ai(&mut m, X);
        assert_eq!(spawned_slot(&mut m), Some(base_slot(0)), "$20 ≡ 0 (mod 32) ⇒ 吐弹");
    }

    /// **生弹条件（成功径）**：四 zp 入参全部按 asm 现算的效果——
    /// 弹体 status←Active、`data0A=(EnData0A*2+朝向)&7 = 朝向`（EnData0A=#$00 ⇒ Movement0）、
    /// 上升动画取 `EnProjectileRisingAnimIndexTable[3*2+朝向]`（`02:978B` 第 3 对＝Dragon 吐弹）。
    #[test]
    fn active_dragon_spawns_projectile_with_movement0_and_dragon_rising_anim() {
        for facing in 0u8..=1 {
            let mut m = Manager::default();
            stage(&mut m, EnemyStatus::Active, 0x00, 0x05, 0x10 | facing);
            dragon_ai(&mut m, X);
            let p = base_slot(0);
            assert_eq!(
                m.enemy(p).status_raw(),
                2,
                "AnimTableIndex=3 ≥ 2 ⇒ 弹体被激活（朝向 {facing}）"
            );
            assert_eq!(m.enemy(p).data0a(), facing, "data0A=(0*2+朝向)&7 ⇒ Movement0/1 的朝向项");
            assert_eq!(m.enemy(p).speed_x(), if facing == 0 { 2 } else { -2 }, "SpawnEnProjectileSpeedX");
            // 上升动画 = RisingAnimIndexTable[$978B][3*2+朝向] = DragonEnProjectileUp_R/L
            let want = game_data::assets::en_projectile_rising_anim(1)[6 + facing as usize];
            assert_eq!(m.enemy(p).reset_anim_index(), want, "animTableIndex=3 ⇒ Dragon 吐弹上行动画");
        }
    }

    /// **`data05` 位义**：bit0 ＝水平朝向，同时决定「吐弹后播哪一支 idle 动画」。
    #[test]
    fn data05_bit0_selects_facing_pair_for_idle_anim() {
        for (facing, want) in [(0u8, ANIM_OFF_IDLE_R), (1u8, ANIM_OFF_IDLE_L)] {
            let mut m = Manager::default();
            stage(&mut m, EnemyStatus::Active, 0x00, 0x05, 0x10 | facing);
            dragon_ai(&mut m, X);
            assert_eq!(
                m.enemy(SLOT).reset_anim_index(),
                want,
                "data05 bit0={facing} ⇒ Idle_{}",
                if facing == 0 { "R" } else { "L" }
            );
        }
    }

    /// **`data05` 位义 + 失败径**：bit4（`07:F872` 的第一个门）清 ⇒ 弹体**没生成**，
    /// 但 asm 不看返回值 ⇒ 音效与吐弹动画照置（"假装生弹"与"忠实空放"的分界）。
    #[test]
    fn spawn_failure_still_stages_spit_anim_and_sfx() {
        let mut m = Manager::default();
        stage(&mut m, EnemyStatus::Active, 0x00, 0x05, 0x00); // bit4 清 ⇒ SpawnEnProjectile 首门即拒
        dragon_ai(&mut m, X);
        assert_eq!(spawned_slot(&mut m), None, "data05 bit4 清 ⇒ 弹体槽全空（真·没生成）");
        assert_eq!(m.enemy(SLOT).reset_anim_index(), ANIM_OFF_IDLE_R, "仍置吐弹动画（asm 不分支）");
        assert_eq!(
            m.read_ram(ram_map::SFX_NOISE_INIT) & SFX_NOISE_SPIT_FLAME,
            SFX_NOISE_SPIT_FLAME,
            "仍置 sfxNoise_SpitFlame（$0680 bit2）"
        );
    }

    /// **失败径（改判据）**：`movementInstrIndex==0`（还停在"从熔岩升起"的第一条移动指令）
    /// 或 `jumpDsplcmnt≠0`（正在上下移动）都跳过吐弹块，直接 `@exit_initAnim`。
    #[test]
    fn movement_gates_block_the_shoot_block() {
        for (mov, jump, why) in [(0x00u8, 0x00u8, "第一条移动指令"), (0x01, 0x80, "jumpDsplcmnt≠0")] {
            let mut m = Manager::default();
            stage(&mut m, EnemyStatus::Active, 0x00, 0x05, 0x10);
            m.enemy(SLOT).set_data06(mov);
            m.enemy(SLOT).set_jump_dsplcmnt(jump);
            dragon_ai(&mut m, X);
            assert_eq!(spawned_slot(&mut m), None, "{why} ⇒ 不吐弹");
            // @exit_initAnim 径：非 Explode ⇒ 置活动动画（type 0x0D/朝向 0 → Norfair 活动表）
            let want = game_data::assets::enemy_active_anim(1)[(0x0D * 2) as usize];
            assert_eq!(m.enemy(SLOT).reset_anim_index(), want, "{why} ⇒ 走 InitEnActiveAnimIndex 径");
        }
    }

    /// **空放径（misfire）**：`RandomNumber1 & $03 == 0` ⇒ 弹体不生成、音效不放、动画不动
    /// （`beq @exit_playAnim` 直接跳过吐弹块的剩余部分）。
    #[test]
    fn misfire_skips_spawn_sfx_and_anim() {
        let mut m = Manager::default();
        stage(&mut m, EnemyStatus::Active, 0x00, 0x00, 0x10); // rng&3 == 0
        dragon_ai(&mut m, X);
        assert_eq!(spawned_slot(&mut m), None, "空放 ⇒ 无弹体");
        assert_eq!(m.enemy(SLOT).reset_anim_index(), 0x00, "空放 ⇒ 不改动画");
        assert_eq!(m.read_ram(ram_map::SFX_NOISE_INIT) & SFX_NOISE_SPIT_FLAME, 0, "空放 ⇒ 不放音效");
    }

    /// **计时门第二段**：`FrameCount & $1F` 落在 `0x0F..=0x1F` ⇒ 播"准备吐"动画
    /// （`@prepareToSpitEnAnimTable` `02:9B62`，按朝向取 R/L）。
    #[test]
    fn frames_0f_to_1f_play_prepare_to_spit_anim() {
        for (facing, want) in [(0u8, ANIM_OFF_PREPARE_R), (1u8, ANIM_OFF_PREPARE_L)] {
            for fc in [0x0Fu8, 0x10, 0x1F] {
                let mut m = Manager::default();
                stage(&mut m, EnemyStatus::Active, fc, 0x05, 0x10 | facing);
                dragon_ai(&mut m, X);
                assert_eq!(
                    m.enemy(SLOT).reset_anim_index(),
                    want,
                    "fc={fc:#04X} 面向 {facing} ⇒ PrepareToSpit_{}",
                    if facing == 0 { "R" } else { "L" }
                );
                assert_eq!(spawned_slot(&mut m), None, "fc={fc:#04X} ≠ 0 (mod 32) ⇒ 不吐弹");
            }
        }
    }

    /// 派发级接线：Norfair type 0x0D 经真实 `choose_enemy_ai` 落到本件
    /// （Resting ⇒ y 钉 `$E8` 是本件唯一的可观测指纹）。
    #[test]
    fn dispatch_wiring_reaches_dragon_for_type_0x0d() {
        let mut m = Manager::default();
        game_data::pack::init_from_env().ok();
        m.write_ram(ram_map::IN_AREA, 1);
        m.write_wram(ens_addr(X, ENS_F_TYPE), 0x0D);
        m.enemy(SLOT).set_status(EnemyStatus::Resting);
        m.enemy(SLOT).set_y(0x30);
        m.enemy(SLOT).set_data05(0x10);
        crate::enemy::slot_cycle::update_all_enemies(&mut m);
        assert_eq!(m.enemy(SLOT).y(), LAVA_Y, "经 choose_enemy_ai 到 Dragon 径（Resting 钉 y）");
    }
}
