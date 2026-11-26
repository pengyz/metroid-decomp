//! Kraid 家族 AI（Kraid 本体 + 他的 Lint / Nail 件）——`refs/m1disasm/SRC/enemies/kraid.asm`
//! （348 行，`prg4_kraid.asm:757` `.include`；bank4 = Kraid 区，`InArea&$0F==2`）。
//!
//! # asm 内容锚（标签 → `bank4:$XXXX` → asm 行号）
//!
//! | 标签 | 地址 | 行号 |
//! |---|---|---|
//! | `KraidAIRoutine` | `04:9AB4` | 4-34 |
//! | `KraidBranch_Explode` | `04:9AC1` | 14-22 |
//! | `KraidBranch_Normal` | `04:9AD4` | 24-27 |
//! | `KraidBranch_Exit` | `04:9ADD` | 30-34 |
//! | `KraidLintAIRoutine` | `04:9AE4` | 38-87 |
//! | `KraidLintRemove` | `04:9AF2` | 48-52 |
//! | `KraidLintMain` | `04:9AF9` | 54-81 |
//! | `KraidLintDraw` | `04:9B24` | 83-87 |
//! | `KraidNailAIRoutine` | `04:9B2C` | 91-92（就是 `jmp KraidLintAIRoutine`） |
//! | `KraidUpdateAllProjectiles` | `04:9B2F` | 96-105 |
//! | `KraidUpdateProjectile` | `04:9B3C` | 109-222 |
//! | `KraidProjectileOffsetY` | `04:9BCC` | 233-234 |
//! | `KraidProjectileOffsetX` | `04:9BD1` | 235-241 |
//! | `KraidProjectileType` | `04:9BDB` | 242-243 |
//! | `KraidTryToLaunchLint` | `04:9BE0` | 248-298 |
//! | `KraidTryToLaunchNail` | `04:9C19` | 303-347 |
//!
//! # 三件的表项
//!
//! `prg4_kraid.asm:171-173`（`04:95E5 ChooseEnemyAIRoutine`）：
//! `08 → KraidAIRoutine` / `09 → KraidLintAIRoutine` / `0A → KraidNailAIRoutine`。
//! 三个 Lint/Nail 槽（`EnsExtra.1..5`）不是 `EnProjectile` 池：Kraid 自己在
//! `EnsExtra.0`，1..5 号槽被他当作「手部弹体」直接管理（`KraidUpdateAllProjectiles`
//! 从 `x=$50` 每个 `-$10` 直到 0，**不含槽 0**）。
//!
//! # 状态门用的是「现役」status
//!
//! `KraidAIRoutine` 的门是 `lda EnsExtra.0.status,x`（现役 `$6AF4+x`），**不是** `$0081`
//! `EnemyStatusPreAI` 快照——所以本文件读 `status_raw()`，与 `rio.rs`
//! （asm 读 `EnemyStatusPreAI`）的读面不同。只有尾件 `CommonEnemyStub` →
//! `UpdateEnemyCommon_Decide` 才读 `$0081`（由
//! [`crate::enemy::framework::update_enemy_common_decide_with_beats`] 内部完成）。
//!
//! # 保留原样（不"修"）
//!
//! - `KraidBranch_Explode` 清的是**绝对地址** `EnsExtra.1..5.status`（无 `,x`）：
//!   无论哪个槽进 Explode，清掉的都是 1..5 号槽——按字节保真。
//! - `KraidUpdateProjectile` 的 `@remove` 先 `jsr CommonJump_0E`（用**旧** type 查
//!   health/initDelay/forceSpeedDelay 三张表），**之后**才在 `@resting` 用
//!   `KraidProjectileType` 覆盖 type——顺序按 asm 保留，故新槽的 health/delay 来自旧
//!   type（刚出生的槽 type=0）。这是行为差异的来源，不许"顺手修正"。
//! - `KraidLintMain` 的 `lda data05 / asl / bmi` 测的是 **bit6**（asm 注释写 bit7，
//!   但字节语义是 bit6）；本件按字节。
//! - `KraidLintMain` 的 BG 碰撞失败径会先被 `EnemyBGCollideOrApplySpeed` 里的
//!   `RemoveEnemy` 清槽，再被本件写 `status=Explode`（3）——顺序照 asm 保留。

use crate::enemy::state::{ens, slot_of, EnemyStatus};
use crate::enemy::tables::{en_addr, ens_addr};
use crate::frame::Manager;

/// `KraidLintCounter`（`constants_ram.asm:649`，`$7E`）——`KraidTryToLaunchLint` 的周期
/// 计数器（每 2 帧减 1；`$7F..=$40` 是 128 帧停顿段）。
///
/// 收口：`ram_map::{KRAID_LINT_COUNTER, KRAID_NAIL_COUNTER}` 是地址真源，读写经
/// `Manager::kraid_lint_counter()/set_kraid_lint_counter()`（`frame/memory.rs` 的
/// 豁免辖），本文件不接触裸字节面。

/// `KraidProjectileOffsetY`（`04:9BCC`，5 B，槽 1..5）。
const PROJECTILE_OFFSET_Y: [i8; 5] = [-11, -3, 5, -10, -2];
// `KraidProjectileOffsetX`（`04:9BD1`，每槽 2 B = [朝右, 朝左]）已迁到资产包
// （`game_data::assets::kraid_projectile_offset_x()`；源表见 `extract.rs::ENEMY_ROM_TABLES`）。
/// `KraidProjectileType`（`04:9BDB`，5 B）——前三个 Lint、后两个 Nail。
const PROJECTILE_TYPE: [u8; 5] = [0x09, 0x09, 0x09, 0x0A, 0x0A];

/// `KraidAIRoutine ; 04:9AB4`（kraid.asm:4-34）。
pub(crate) fn kraid_ai(mem: &mut Manager, x: u8) {
    let slot = slot_of(x);
    // asm:4-12 —— 门读**现役** status（不是 $0081 快照）：
    //   `cmp #3 / bcc Normal`（0/1/2）、`beq Explode`（3）、`cmp #5 / bne Exit`、
    //   落到 Explode（5）。
    match mem.enemy(slot).status_raw() {
        0..=2 => {
            // asm:24-27 `KraidBranch_Normal`
            kraid_update_all_projectiles(mem);
            kraid_try_launch_lint(mem);
            kraid_try_launch_nail(mem);
        }
        3 | 5 => {
            // asm:14-22 `KraidBranch_Explode`：清**绝对** `EnsExtra.1..5.status`。
            for s in 1..=5u8 {
                mem.enemy(s).set_status(EnemyStatus::NoEnemy);
            }
        }
        _ => {
            // 4/6 直落 Exit（asm:12）。
        }
    }
    // asm:30-34 `KraidBranch_Exit`：`lda #$0A / sta $00` 后 `jmp CommonEnemyStub`
    // （sidehopper.asm:19-22 = `lda #$08 / sta $01 / jmp UpdateEnemyCommon_Decide`）。
    // Decide 内部读 `$0081`：1⇒@noMove（beat 8）、3⇒@noMoveNoAnim、其余⇒全件（beat $0A）。
    crate::enemy::framework::update_enemy_common_decide_with_beats(mem, x, 0x0A, 0x08);
}

/// `KraidUpdateAllProjectiles ; 04:9B2F`（kraid.asm:96-105）。
///
/// `ldx #$50` → 每轮 `-$10` → `bne` 停 ⇒ `x = $50,$40,$30,$20,$10`（槽 1..5，不含槽 0）。
fn kraid_update_all_projectiles(mem: &mut Manager) {
    for x in [0x50u8, 0x40, 0x30, 0x20, 0x10] {
        kraid_update_projectile(mem, x);
    }
}

/// `KraidUpdateProjectile ; 04:9B3C`（kraid.asm:109-222）。
fn kraid_update_projectile(mem: &mut Manager, x: u8) {
    let slot = slot_of(x);
    // asm:111-113 `ldy status / beq @remove`（空槽也走 @remove，asm 注释自认"a bit odd"）。
    if mem.enemy(slot).status_raw() == 0 {
        kraid_projectile_remove_and_resting(mem, x);
        return;
    }
    // asm:115-118：只处理 lint($09)/nail($0A)，其余直出。
    let type_ = mem.enemy(slot).enemy_type();
    if type_ != 0x09 && type_ != 0x0A {
        return;
    }
    // asm:120-123 `@branchA`：不可见（data05 bit1 清）⇒ @remove。
    if mem.enemy(slot).data05() & 0x02 == 0 {
        kraid_projectile_remove_and_resting(mem, x);
        return;
    }
    // asm:124-126 `dey / beq @resting`：status-1==0 ⇒ 1(Resting) 直落 @resting。
    let d = mem.enemy(slot).status_raw().wrapping_sub(1);
    if d == 0 {
        kraid_projectile_resting(mem, x);
        return;
    }
    // asm:127-128 `cpy #enemyStatus_Explode-1(=2) / beq @remove`：status 3 ⇒ @remove。
    if d == 2 {
        kraid_projectile_remove_and_resting(mem, x);
        return;
    }
    // asm:129-131 `cpy #enemyStatus_Frozen-1(=3) / bne Exit`：非 Frozen(4) 直出。
    if d != 3 {
        return;
    }
    // asm:132-136：冻结弹只有"冻前是 Resting(1)"才回 @resting，否则直出。
    if mem.enemy(slot).prev_status() != 0x01 {
        return;
    }
    kraid_projectile_resting(mem, x);
}

/// `@remove ; 04:9B67`（kraid.asm:143-147）+ 落进 `@resting`。
fn kraid_projectile_remove_and_resting(mem: &mut Manager, x: u8) {
    let slot = slot_of(x);
    // asm:143 `lda #0 / sta EnsExtra.0.status,x`
    mem.enemy(slot).set_status(EnemyStatus::NoEnemy);
    // asm:144 `sta Ens.0.specialAttribs,x`（typed 写侧：`enemy/state.rs` 的
    // `set_special_attribs`——它同时是 `@applySpeed` 里 bit7 翻倍 health 的判据）。
    mem.enemy(slot).set_special_attribs(0);
    // asm:145 `jsr CommonJump_0E`（`$802A` → `07:EB6E LEB6E`）：用**旧** type。
    // `ldy type / asl data05 / jsr InitEnemyFacingDirectionAxisAndDelay
    // / jmp InitEnemyForceSpeedTowardsSamusDelayAndHealth`
    let type_ = mem.enemy(slot).enemy_type();
    let area = crate::frame::cur_area(mem);
    let l977b = game_data::assets::enemy_facing(area)
        .get((type_ & 0x3F) as usize)
        .copied()
        .unwrap_or(0);
    let d5 = mem.enemy(slot).data05();
    // `asl data05` 后 `ror`（`07:FB7B` 的 `ror Ens.0.data05,x`）⇒ 净效果：bit7 ← L977B 的
    // bit6，低 7 位不变。
    mem.enemy(slot)
        .set_data05((d5 & 0x7F) | (l977b.wrapping_shl(1) & 0x80));
    // `InitEnemyFacingDirectionAxisAndDelay` 尾件：`delay ← EnemyInitDelayTbl[type]`。
    let init_delay = game_data::assets::enemy_init_delay(area)
        .get(type_ as usize)
        .copied()
        .unwrap_or(0);
    mem.enemy(slot).set_delay(init_delay);
    // `InitEnemyForceSpeedTowardsSamusDelayAndHealth ; 07:FB8E`：
    // `data0D ← EnemyForceSpeedTowardsSamusDelayTbl[type]`；
    // `health ← EnemyHealthTbl[type]`（`specialAttribs` bit7 置则 ×2——本径刚清 0）。
    let force_delay = game_data::assets::enemy_force_speed_delay(area)
        .get(type_ as usize)
        .copied()
        .unwrap_or(0);
    mem.enemy(slot).set_data0d(force_delay);
    let mut health = game_data::assets::enemy_health(area)
        .get(type_ as usize)
        .copied()
        .unwrap_or(0);
    if mem.enemy(slot).special_attribs() & 0x80 != 0 {
        health = health.wrapping_shl(1);
    }
    mem.enemy(slot).set_health(health);
    kraid_projectile_resting(mem, x);
}

/// `@resting ; 04:9B72`（kraid.asm:149-219）——把弹体摆到 Kraid 手上并起 Resting。
fn kraid_projectile_resting(mem: &mut Manager, x: u8) {
    let slot = slot_of(x);
    let idx = slot as usize; // 槽 1..5
    // asm:151-153 `lda Ens.0.data05`（**无 x = 槽 0 = Kraid 本体**）/ `sta data05,x`
    let kraid_d5 = mem.enemy(0).data05();
    mem.enemy(slot).set_data05(kraid_d5);
    // asm:155 `lsr`：carry ← Kraid data05 bit0（朝向；0=右 1=左）。
    let facing_left = kraid_d5 & 0x01;
    // asm:157-160 `txa / lsr×4 / tay` ⇒ y = 槽号；`lda KraidProjectileOffsetY-1,y`。
    let off_y = PROJECTILE_OFFSET_Y[idx - 1];
    // asm:161-163 `lda KraidProjectileType-1,y / sta EnsExtra.0.type,x`
    mem.enemy(slot).set_enemy_type(PROJECTILE_TYPE[idx - 1]);
    // asm:165-170 `tya / plp / rol / tay` ⇒ y = 槽号*2 + 朝向 bit；
    // `lda KraidProjectileOffsetX-2,y`。
    let off_x = game_data::assets::kraid_projectile_offset_x()[(idx - 1) * 2 + facing_left as usize] as i8;
    // asm:174-186 `StoreEnemyPositionToTemp`（bank4 用 `ldx #$00` 取 Kraid 槽，
    // 无 bank1 的 PageIndex BUG）→ `CommonJump_ApplySpeedToPosition`。
    let (ky, kx, khi) = {
        let k = mem.enemy(0);
        (k.y(), k.pos_x(), k.hi())
    };
    let (ok, ny, nx, nhi) = crate::enemy::motion::apply_speed_to_position_raw(
        mem, ky, kx, khi, off_y, off_x,
    );
    // asm:189-190 `bcc Exit`：初始位置越界 ⇒ 不落 status/位置。
    if !ok {
        return;
    }
    // asm:191-193 `lda status / bne Load... / inc status`：0(NoEnemy)→1(Resting)。
    if mem.enemy(slot).status_raw() == 0 {
        mem.enemy(slot).set_status(EnemyStatus::Resting);
    }
    // asm:194-201 `LoadEnemyPositionFromTemp`。
    mem.enemy(slot).set_y(ny);
    mem.enemy(slot).set_pos_x(nx);
    mem.enemy(slot).set_hi(nhi & 0x01);
}

/// `KraidLintAIRoutine ; 04:9AE4`（kraid.asm:38-87）。
pub(crate) fn kraid_lint_ai(mem: &mut Manager, x: u8) {
    let slot = slot_of(x);
    // asm:39-46 —— `data05 & $02` 清（不可见）⇒ Remove；否则 status==3 也 ⇒ Remove。
    if mem.enemy(slot).data05() & 0x02 == 0 || mem.enemy(slot).status_raw() == 3 {
        // asm:48-52 `KraidLintRemove`：`lda #0 / sta EnsExtra.0.status,x`，然后落 Draw。
        mem.enemy(slot).set_status(EnemyStatus::NoEnemy);
    } else {
        // asm:54-81 `KraidLintMain`
        kraid_lint_main(mem, x);
    }
    // asm:83-87 `KraidLintDraw`：`lda #$01 / jsr UpdateEnemyAnim
    // / jmp UpdateEnemyCommon_noMoveNoAnim`。
    crate::enemy::framework::update_enemy_common_no_move_anim(mem, x, 0x01);
}

/// `KraidNailAIRoutine ; 04:9B2C`（kraid.asm:91-92）——`jmp KraidLintAIRoutine`。
///
/// 行为与 Lint **完全相同**；差别只在表项 type（0x0A）与
/// `EnemyMovementChoice_KraidNail`（0x0F/0x10/0x11）——后者由装载期/静息壳负责。
pub(crate) fn kraid_nail_ai(mem: &mut Manager, x: u8) {
    kraid_lint_ai(mem, x);
}

/// `KraidLintMain ; 04:9AF9`（kraid.asm:54-81）。
fn kraid_lint_main(mem: &mut Manager, x: u8) {
    let slot = slot_of(x);
    // asm:55-59 `lda data05 / asl / bmi` ⇒ 跳过移动。注意 asm 注释写 "bit7"，但
    // `asl` 后的符号位是**原 bit6**——按字节保真 bit6。
    if mem.enemy(slot).data05() & 0x40 != 0 {
        return;
    }
    // asm:61-63 `lda status / cmp #enemyStatus_Active / bne Draw`。
    if mem.enemy(slot).status_raw() != 0x02 {
        return;
    }
    // asm:65-72 `jsr EnemyGetDeltaY / ldx PageIndex / lda $00 / sta Ens.speedY,x`
    // asm:73-79 同构 X 轴 → `Ens.speedX,x`
    let (dy, dx) = lint_movement_delta(mem, x);
    mem.enemy(slot).set_speed_y(dy);
    mem.enemy(slot).set_speed_x(dx);
    // asm:80-81 `jsr EnemyBGCollideOrApplySpeed / bcs Draw`
    if !bg_collide_or_apply_speed_slot(mem, x) {
        // asm:82-83 `lda #enemyStatus_Explode / sta EnsExtra.0.status,x`（撞墙/越界 ⇒ 爆）。
        mem.enemy(slot).set_status(EnemyStatus::Explode);
    }
}

/// Kraid 家族用到的 bank4 移动串偏移（**值 = asm 标签 − 串区基址 `$97EF`**）。
///
/// `[movementIndex, 右向偏移, 左向偏移]`——来自 `prg4_kraid.asm` 的
/// `EnemyMovement0E_R/L`（`$9839/$983C`）、`0F_R/L`（`$983F/$9856`）、
/// `10_R/L`（`$986D/$9884`）、`11_R/L`（`$989B/$98B2`）。
///
/// 【为什么不用引擎的移动串入口】`areas_common.asm:555 LoadEnemyMovementPtr` 走
/// `EnemyMovementPtrs`（bank4 `04:96DB`，asm 侧 18 索引 × R/L = 72 B）；而资产包里的
/// `enemy_movement_ptrs(2)` 只有 36 B（9 索引），索引 0x0C-0x11 的指针缺失 ⇒
/// `common::enemy_get_delta_xy` 会回落到 `$97F5`（Skree 串）。这是**共享数据缺口**
/// （`assets/metroid.pack` 的 `area.kr.en_mov_ptrs` 段被截），不在本卡允许改的文件内
/// （见回报的诚实边界）。Kraid 家族的 0x0E-0x11 串只含 `SignMagSpeed` + `$FF`（ROM
/// 字节 `0x9839..0x98C3` 逐字节核对），故这里按 asm 标签就地锚定偏移、用同一串区走
/// 同一套 SignMagSpeed 语义，行为与 asm 一致。
/// 四个索引的串**绝对 ROM 地址**（asm 标签；R/L 各一）。消费侧减去区基址
/// `assets::enemy_movement_string_base(area)` 得窗内偏移——Kraid 区基址为 `$97E9`。
const KRAID_MOV_STR_ADDRS: [(u8, u16, u16); 4] = [
    (0x0E, 0x9839, 0x983C),
    (0x0F, 0x983F, 0x9856),
    (0x10, 0x986D, 0x9884),
    (0x11, 0x989B, 0x98B2),
];

/// `SignMagSpeed` 打包字节的 Δy（高 nibble，bit7=符号）。
fn sign_mag_dy(packed: u8) -> i8 {
    let mag = ((packed >> 4) & 0x07) as i8;
    if packed & 0x80 != 0 { -mag } else { mag }
}

/// `SignMagSpeed` 打包字节的 Δx（低 nibble，bit3=符号）。
fn sign_mag_dx(packed: u8) -> i8 {
    let mag = (packed & 0x07) as i8;
    if packed & 0x08 != 0 { -mag } else { mag }
}

/// `EnemyGetDeltaY ; $8244` + `EnemyGetDeltaX ; $8318` 的成对语义（kraid.asm:65-79）。
///
/// 与 [`crate::enemy::motion::enemy_move`] 内部 `enemy_delta_for_move` 同构：
/// 写回 `delay` 与 `data06`（movementInstrIndex），只是**不逐像素施移**——Kraid 的
/// Lint 把 Δ 存进 speedY/speedX，随后由 `EnemyBGCollideOrApplySpeed` 用
/// ApplySpeedToPosition 一次施加（这就是 lint 与普通敌"每像素移动"的分歧点）。
///
/// 本件覆盖的串只有 `SignMagSpeed` + `$FF`（Restart）：没有 `$FA-$FE` 的面检/触发指令，
/// 故 Trig 径（`L968B` bit5）在本件不可达，未实现——若将来把 0x0C/0x0D 也接入需补。
fn lint_movement_delta(mem: &mut Manager, x: u8) -> (i8, i8) {
    let slot = slot_of(x);
    let area = crate::frame::cur_area(mem);
    let (d05, mov_idx, instr, delay) = {
        let e = mem.enemy(slot);
        (e.data05(), e.movement_index(), e.data06(), e.delay())
    };
    // `EnemyGetDeltaY@bit5`（`$8244`：`and #$20 / eor #$20 / beq Store`）⇒ Δ=0。
    if d05 & 0x20 != 0 {
        return (0, 0);
    }
    // 朝向（data05 bit7=0 时取 bit0；lint/nail 的 L977B 字节为 0 ⇒ 恒 bit0）。
    let facing_left = d05 & 0x01 != 0;
    let base = game_data::assets::enemy_movement_string_base(area);
    let off = match KRAID_MOV_STR_ADDRS.iter().find(|e| e.0 == mov_idx) {
        Some(e) => (if facing_left { e.2 } else { e.1 }).wrapping_sub(base) as usize,
        // 不会发生（lint=0x0E / nail=0x0F-0x11）；fail-safe 不产生位移。
        None => return (0, 0),
    };
    let strings = game_data::assets::enemy_movement_strings(area);
    let mut dly = delay;
    let mut ii = instr;
    for _ in 0..16u32 {
        let d = strings.get(off + ii as usize).copied().unwrap_or(0xFF);
        if d >= 0xF0 {
            if d == 0xFF {
                ii = 0; // EnemyMovementInstr_Restart
                continue;
            }
            // 本件未实现的面检/触发指令（见函数 doc）。
            return (0, 0);
        }
        if d == dly {
            // 到期：delay=0、instr+=2、**当帧继续读下一条**（`$827F / beq @ReadByte`）。
            dly = 0;
            ii = ii.wrapping_add(2);
            continue;
        }
        let packed = strings.get(off + ii as usize + 1).copied().unwrap_or(0);
        dly = dly.wrapping_add(1);
        {
            let mut e = mem.enemy(slot);
            e.set_delay(dly);
            // `$0406` 在本径按 `movementInstrIndex` 语义使用。
            e.set_data06(ii);
        }
        return (sign_mag_dy(packed), sign_mag_dx(packed));
    }
    (0, 0)
}

/// `EnemyBGCollideOrApplySpeed ; 07:FA1E`——**按敌槽 `x`** 的版本。
///
/// `en_projectile.rs` 里同名函数按**弹体索引**封装（`base_slot(i)`），Kraid 的 Lint
/// 对自己的 1..5 号敌槽直接调用（asm `jsr CommonJump_EnemyBGCollideOrApplySpeed` 时
/// `x` 就是 `$10..$50`），故本件按 asm 原形在本地实现，语义逐条对齐 07:FA1E：
/// Norfair 且 status bit0 清 ⇒ 免瓦检；否则 `tile < $A0` ⇒ 撞固瓦（carry 清）；
/// 施速失败 ⇒ `RemoveEnemy`；`LoadEnemyPositionFromTemp` 恒执行（含环绕值落盘）。
fn bg_collide_or_apply_speed_slot(mem: &mut Manager, x: u8) -> bool {
    let slot = slot_of(x);
    let norfair = crate::frame::in_area_raw(mem) == 0x11;
    let status_bit0 = mem.enemy(slot).status_raw() & 0x01 != 0;
    if !norfair || status_bit0 {
        let (y, px, hi) = {
            let e = mem.enemy(slot);
            (e.y(), e.pos_x(), e.hi())
        };
        let tile = mem.room().tile(y, px, (hi & 0x01) << 2);
        if crate::world::room_ram::tile_blocks_non_projectile(tile) {
            return false; // `cmp #$A0 / bcc @RTS`
        }
    }
    let (y0, x0, hi0, dy, dx) = {
        let e = mem.enemy(slot);
        (e.y(), e.pos_x(), e.hi(), e.speed_y(), e.speed_x())
    };
    let (ok, ny, nx, nhi) =
        crate::enemy::motion::apply_speed_to_position_raw(mem, y0, x0, hi0, dy, dx);
    if !ok {
        // `@applySpeed` 失败 ⇒ `RemoveEnemy ; 07:FA18`。
        crate::enemy::en_projectile::remove_enemy(mem, slot);
    }
    {
        let mut e = mem.enemy(slot);
        e.set_y(ny);
        e.set_pos_x(nx);
        e.set_hi(nhi & 0x01);
    }
    ok
}

/// `KraidTryToLaunchLint ; 04:9BE0`（kraid.asm:248-298）。
fn kraid_try_launch_lint(mem: &mut Manager) {
    // asm:250-255 `ldy KraidLintCounter / bne @endIf_A / ldy #$80`
    let mut y = mem.kraid_lint_counter();
    if y == 0 {
        y = 0x80;
    }
    // asm:257-259 `lda FrameCount / and #$02 / bne @RTS`——本帧不动（连计数器都不写）。
    if mem.frame_count() & 0x02 != 0 {
        return;
    }
    // asm:261-262 `dey / sty KraidLintCounter`
    y = y.wrapping_sub(1);
    mem.set_kraid_lint_counter(y);
    // asm:264-270 `tya / asl / bmi / and #$0F / cmp #$0A / bne @RTS`：
    // A = y*2（补偿每 2 帧一减）——bit7 置（y bit6）⇒ 128 帧停顿段；低 nibble 必须 $0A。
    let a = y.wrapping_shl(1);
    if a & 0x80 != 0 || a & 0x0F != 0x0A {
        return;
    }
    // asm:272-285 依次试槽 $10/$20/$30，第一个 Resting(1) 的进 @primeForLaunch。
    for sx in [0x10u8, 0x20, 0x30] {
        if mem.enemy(slot_of(sx)).status_raw() == 0x01 {
            // asm:288-290 `lda #$08 / sta Ens.0.delay,x`（静息 8 帧后发射）。
            mem.enemy(slot_of(sx)).set_delay(0x08);
            return;
        }
    }
    // asm:292-294 三个 lint 全在空中 ⇒ 撤销这一减，下帧再试。
    mem.set_kraid_lint_counter(y.wrapping_add(1));
}

/// `KraidTryToLaunchNail ; 04:9C19`（kraid.asm:303-347）——与 Lint 同构，差别：
/// 空计数器初值 `$60`、低 nibble 必须 **0**、只试槽 `$40/$50`。
fn kraid_try_launch_nail(mem: &mut Manager) {
    // asm:305-310
    let mut y = mem.kraid_nail_counter();
    if y == 0 {
        y = 0x60;
    }
    // asm:312-314
    if mem.frame_count() & 0x02 != 0 {
        return;
    }
    // asm:316-317
    y = y.wrapping_sub(1);
    mem.set_kraid_nail_counter(y);
    // asm:319-324 `and #$0F / bne @RTS`：低 nibble 必须 0。
    let a = y.wrapping_shl(1);
    if a & 0x80 != 0 || a & 0x0F != 0x00 {
        return;
    }
    // asm:326-337 试槽 $40/$50。
    for sx in [0x40u8, 0x50] {
        if mem.enemy(slot_of(sx)).status_raw() == 0x01 {
            mem.enemy(slot_of(sx)).set_delay(0x08);
            return;
        }
    }
    // asm:339-341
    mem.set_kraid_nail_counter(y.wrapping_add(1));
}

#[cfg(test)]
mod tests_kraid {
    //! 验收范式 Tier1（sem/事件级）：每件的主分支 + 每个出口，全部指回 asm 行号。
    use super::*;
    use crate::enemy::state::Anim;
    use crate::ram_map;

    fn stage() -> Manager {
        if !game_data::pack::is_ready() {
            let _ = game_data::pack::init_from_env();
        }
        let mut m = Manager::default();
        // 区序 2 = Kraid（`InArea & $0F`）——本卡的按区表（health/initDelay/
        // forceSpeedDelay/L977B/movement/anim）都走它。
        m.write_ram(ram_map::IN_AREA, 0x02);
        m
    }

    /// Kraid 本体（槽 0）定位 + data05。
    fn kraid_at(m: &mut Manager, y: u8, x: u8, hi: u8, d05: u8) {
        let mut k = m.enemy(0);
        k.set_y(y);
        k.set_pos_x(x);
        k.set_hi(hi);
        k.set_data05(d05);
    }

    /// 手部弹体槽（1..5）样板：位置随便给一个可区分值。
    fn proj(m: &mut Manager, slot: u8, status: EnemyStatus, type_: u8, d05: u8) {
        let mut e = m.enemy(slot);
        e.set_status(status);
        e.set_enemy_type(type_);
        e.set_data05(d05);
        e.set_y(0x10);
        e.set_pos_x(0x10);
        e.set_hi(0);
        e.set_delay(0xAA);
        e.set_health(0x11);
        e.set_data0d(0x22);
    }

    /// 本文件就地锚定的四个串偏移必须与**资产包**的指针表逐个相等（防未来漂移：
    /// 指针表 2026-11-20 由 36 B 修成 72 B，此后两处口径必须一致）。
    #[test]
    fn hardcoded_offsets_agree_with_pack_pointer_table() {
        let ptrs = game_data::assets::enemy_movement_ptrs(2); // area 2 = Kraid（InArea 序）
        assert_eq!(ptrs.len(), 72, "18 索引 × R/L × 2 B");
        let base = game_data::assets::enemy_movement_string_base(2); // Kraid 区基址 $97E9
        for (mov_idx, right, left) in KRAID_MOV_STR_ADDRS {
            for (facing, want) in [(0usize, right), (1usize, left)] {
                let rolled = (mov_idx as usize) * 2 + facing;
                let e = rolled * 2;
                let ptr = u16::from_le_bytes([ptrs[e], ptrs[e + 1]]) as usize;
                assert_eq!(
                    ptr,
                    want as usize,
                    "movIdx {mov_idx:#04X} facing {facing}：包内指针 {ptr:#06X} 与 asm 标签 {want:#06X} 不一致"
                );
                assert!(ptr >= base as usize, "指针必须落在区基址之后");
            }
        }
    }

    // ---- KraidAIRoutine（asm:4-34）-------------------------------------------

    /// `KraidBranch_Explode`（asm:14-22）清的是**绝对** `EnsExtra.1..5.status`：
    /// 五个手部槽全部归零。单帧判别：同一份槽状态，Kraid 现役 status 换成 2（Normal）
    /// 就**不清**——证明门读的确实是现役 status 而不是别的输入。
    #[test]
    fn explode_gate_clears_slots_one_to_five_live_status_is_the_discriminator() {
        // asm: kraid.asm:14-22
        let mut m = stage();
        for s in 1..=5u8 {
            proj(&mut m, s, EnemyStatus::Active, 0x0B, 0x02);
        }
        let mut k = m.enemy(0);
        k.set_status(EnemyStatus::Explode);
        k.set_anim_delay(0);
        drop(k);
        kraid_ai(&mut m, 0);
        for s in 1..=5u8 {
            assert_eq!(
                m.enemy(s).status_raw(),
                0,
                "槽 {s} 应被 Explode 分支清掉（绝对地址 EnsExtra.1..5.status）"
            );
        }

        // 单帧判别：只换现役 status（3→2），槽必须保持原状（type 0x0B 不属 lint/nail，
        // KraidUpdateProjectile 直出，Normal 分支不会碰它们）。
        let mut m = stage();
        for s in 1..=5u8 {
            proj(&mut m, s, EnemyStatus::Active, 0x0B, 0x02);
        }
        let mut k = m.enemy(0);
        k.set_status(EnemyStatus::Active);
        k.set_anim_delay(0);
        drop(k);
        kraid_ai(&mut m, 0);
        for s in 1..=5u8 {
            assert_eq!(
                m.enemy(s).status_raw(),
                2,
                "Normal 分支不得清槽 {s}（status=Active 走 asm:24-27）"
            );
        }
    }

    /// 门读**现役** status，不读 `$0081 EnemyStatusPreAI` 快照：
    /// 现役=3（Explode）、快照=2 ⇒ 仍清槽；现役=2、快照=3 ⇒ 不清槽（只是尾件 Decide
    /// 走 @noMoveNoAnim）。这条是 rio.rs 同类回归测的镜像（读面相反）。
    #[test]
    fn ai_gate_reads_live_status_not_pre_ai_snapshot() {
        // asm: kraid.asm:5-12（`lda EnsExtra.0.status,x`）
        let mut m = stage();
        for s in 1..=5u8 {
            proj(&mut m, s, EnemyStatus::Active, 0x0B, 0x02);
        }
        m.enemy(0).set_status(EnemyStatus::Explode);
        m.enemy(0).set_anim_delay(0);
        m.write_ram(ram_map::ENEMY_STATUS_PRE_AI, 2);
        kraid_ai(&mut m, 0);
        for s in 1..=5u8 {
            assert_eq!(m.enemy(s).status_raw(), 0, "现役 3 ⇒ 必须清槽 {s}");
        }

        let mut m = stage();
        for s in 1..=5u8 {
            proj(&mut m, s, EnemyStatus::Active, 0x0B, 0x02);
        }
        m.enemy(0).set_status(EnemyStatus::Active);
        m.enemy(0).set_anim_delay(0);
        m.write_ram(ram_map::ENEMY_STATUS_PRE_AI, 3);
        kraid_ai(&mut m, 0);
        for s in 1..=5u8 {
            assert_eq!(m.enemy(s).status_raw(), 2, "现役 2 ⇒ 不得清槽 {s}");
        }
    }

    /// `KraidBranch_Exit`（asm:30-34）→ `UpdateEnemyCommon_Decide`：`$0081` 三拍。
    /// 用 Kraid 现役 3 走 Explode→Exit，避免 Normal 分支的弹体/计数器噪声。
    #[test]
    fn exit_decide_uses_pre_ai_snapshot_beats_0a_08() {
        // asm: kraid.asm:30-34 + prg4_kraid.asm:715（`lda EnemyStatusPreAI`）
        for (pre, want) in [(1u8, 0x08u8), (2, 0x0A), (3, 0)] {
            let mut m = stage();
            m.enemy(0).set_status(EnemyStatus::Explode);
            m.enemy(0).set_anim_delay(0);
            m.write_ram(ram_map::ENEMY_STATUS_PRE_AI, pre);
            kraid_ai(&mut m, 0);
            assert_eq!(
                m.enemy(0).anim_delay(),
                want,
                "$0081={pre} ⇒ Decide 应选 beat {want:#04X}（1=@noMove 8 / 2=全件 $0A / 3=@noMoveNoAnim 不动画）"
            );
            assert_eq!(m.enemy(0).status_raw(), 3, "Exit 不改 Kraid 现役 status");
        }
    }

    // ---- KraidLintAIRoutine / KraidNailAIRoutine（asm:38-92）-----------------

    /// 不可见门（asm:39-41）：`data05 & $02 == 0` ⇒ 清槽为 NoEnemy。
    #[test]
    fn lint_invisible_clears_status() {
        // asm: kraid.asm:39-41
        let mut m = stage();
        proj(&mut m, 1, EnemyStatus::Active, 0x09, 0x00); // bit1 清 = 不可见
        kraid_lint_ai(&mut m, 0x10);
        assert_eq!(m.enemy(1).status_raw(), 0);
    }

    /// 爆炸态门（asm:42-46）：可见但 status==3 ⇒ 也清槽。
    #[test]
    fn lint_exploding_clears_status() {
        // asm: kraid.asm:42-46
        let mut m = stage();
        proj(&mut m, 1, EnemyStatus::Explode, 0x09, 0x02);
        kraid_lint_ai(&mut m, 0x10);
        assert_eq!(m.enemy(1).status_raw(), 0);
    }

    /// 移动门（asm:55-59）：`data05` bit6 置 ⇒ 只画不走。
    /// 单帧判别：同一格换个 bit6（0x40 vs 0x00）结果相反——0x00 时同帧会走 4 px。
    #[test]
    fn lint_bit6_gate_skips_movement_but_still_draws() {
        // asm: kraid.asm:55-59（`lda data05 / asl / bmi`——字节语义是 bit6）
        let mut m = stage();
        proj(&mut m, 1, EnemyStatus::Active, 0x09, 0x42); // bit6 置 + bit1 可见
        m.enemy(1).set_movement_index(0x0E);
        m.enemy(1).set_delay(0);
        m.room().set_tile(0x10, 0x10, 0, 0xA0);
        kraid_lint_ai(&mut m, 0x10);
        assert_eq!(m.enemy(1).pos_x(), 0x10, "bit6 置 ⇒ 不得位移");
        assert_eq!(m.enemy(1).status_raw(), 2, "bit6 置 ⇒ 不得爆");
        assert_eq!(m.enemy(1).anim_delay(), 0x01, "仍走 Draw 的 anim 1");

        // 只清 bit6（bit1 仍置）：同帧必须走 4 px。
        m.enemy(1).set_data05(0x02);
        m.enemy(1).set_anim_delay(0);
        kraid_lint_ai(&mut m, 0x10);
        assert_eq!(m.enemy(1).pos_x(), 0x14, "bit6 清 ⇒ asm:65-80 施速 +4");
    }

    /// 非 Active 门（asm:61-63）：Resting(1) 只画不走。
    #[test]
    fn lint_resting_draws_without_moving() {
        // asm: kraid.asm:61-63
        let mut m = stage();
        proj(&mut m, 1, EnemyStatus::Resting, 0x09, 0x02);
        m.enemy(1).set_movement_index(0x0E);
        m.enemy(1).set_delay(0);
        m.room().set_tile(0x10, 0x10, 0, 0xA0);
        kraid_lint_ai(&mut m, 0x10);
        assert_eq!(m.enemy(1).pos_x(), 0x10);
        assert_eq!(m.enemy(1).anim_delay(), 0x01);
        assert_eq!(m.enemy(1).status_raw(), 1);
    }

    /// 撞固瓦径（asm:80-83）：`tile < $A0` ⇒ BG 碰撞失败 ⇒ status=Explode(3)。
    #[test]
    fn lint_dies_on_solid_tile() {
        // asm: kraid.asm:77-83
        let mut m = stage();
        proj(&mut m, 1, EnemyStatus::Active, 0x09, 0x02);
        m.enemy(1).set_movement_index(0x0E);
        m.enemy(1).set_delay(0);
        m.room().set_tile(0x10, 0x10, 0, 0x9F); // < $A0 = 固瓦
        kraid_lint_ai(&mut m, 0x10);
        assert_eq!(m.enemy(1).status_raw(), 3, "撞固瓦 ⇒ Explode");
        assert_eq!(m.enemy(1).pos_x(), 0x10, "失败径不落位置");
    }

    /// 空瓦径（asm:65-80）：`tile >= $A0` ⇒ 施速成功，位置 +4、status 保持 2。
    #[test]
    fn lint_moves_four_pixels_on_free_tile() {
        // asm: kraid.asm:65-80
        let mut m = stage();
        proj(&mut m, 1, EnemyStatus::Active, 0x09, 0x02);
        m.enemy(1).set_movement_index(0x0E); // EnemyMovement0E_R = `SignMagSpeed $50, 4, 0`
        m.enemy(1).set_delay(0);
        m.room().set_tile(0x10, 0x10, 0, 0xA0);
        kraid_lint_ai(&mut m, 0x10);
        assert_eq!(m.enemy(1).pos_x(), 0x14, "0E_R 串 Δx=+4");
        assert_eq!(m.enemy(1).status_raw(), 2);
        assert_eq!(m.enemy(1).speed_x(), 4, "Δx 落 speedX（asm:73-79）");
    }

    /// `KraidNailAIRoutine`（asm:91-92）= `jmp KraidLintAIRoutine`：同输入同输出。
    #[test]
    fn nail_routine_is_identical_to_lint() {
        // asm: kraid.asm:91-92
        let mk = || {
            let mut m = stage();
            proj(&mut m, 1, EnemyStatus::Active, 0x0A, 0x02);
            m.enemy(1).set_movement_index(0x0E);
            m.enemy(1).set_delay(0);
            m.room().set_tile(0x10, 0x10, 0, 0xA0);
            m
        };
        let mut a = mk();
        kraid_lint_ai(&mut a, 0x10);
        let mut b = mk();
        kraid_nail_ai(&mut b, 0x10);
        assert_eq!(a.enemy(1).pos_x(), b.enemy(1).pos_x());
        assert_eq!(a.enemy(1).status_raw(), b.enemy(1).status_raw());
        assert_eq!(a.enemy(1).anim_delay(), b.enemy(1).anim_delay());
        assert_eq!(a.enemy(1).pos_x(), 0x14);
    }

    // ---- 计数器/发射（asm:248-347）------------------------------------------

    /// 计数器初始化 + 帧门（asm:250-259）：空计数器 ⇒ 存 `$7F`；`FrameCount&2` 置 ⇒
    /// 连计数器都不写。
    #[test]
    fn lint_counter_init_and_frame_gate() {
        // asm: kraid.asm:250-259
        let mut m = stage();
        m.set_kraid_lint_counter(0);
        m.write_ram(ram_map::FRAME_COUNT, 0);
        kraid_try_launch_lint(&mut m);
        assert_eq!(m.kraid_lint_counter(), 0x7F, "空 ⇒ #$80-1");

        let mut m = stage();
        m.set_kraid_lint_counter(0);
        m.write_ram(ram_map::FRAME_COUNT, 0x02);
        kraid_try_launch_lint(&mut m);
        assert_eq!(m.kraid_lint_counter(), 0, "帧门命中 ⇒ 不写计数器");
    }

    /// 停顿时段（asm:264-270）：y 的 bit6 置（停顿 128 帧段）⇒ 只减不发。
    #[test]
    fn lint_counter_pause_window_does_not_launch() {
        // asm: kraid.asm:264-270
        let mut m = stage();
        m.set_kraid_lint_counter(0x41); // 减 1 → 0x40，bit6 置
        m.write_ram(ram_map::FRAME_COUNT, 0);
        let mut l = m.enemy(1);
        l.set_status(EnemyStatus::Resting);
        l.set_delay(0xAA);
        drop(l);
        kraid_try_launch_lint(&mut m);
        assert_eq!(m.kraid_lint_counter(), 0x40, "仍在停顿时段");
        assert_eq!(m.enemy(1).delay(), 0xAA, "不得装填发射延时");
    }

    /// 装填径（asm:272-290）：y=5（(5*2)&$0F==$0A）⇒ 第一个 Resting 槽 delay←8。
    #[test]
    fn lint_launch_primes_first_resting_lint() {
        // asm: kraid.asm:272-290
        let mut m = stage();
        m.set_kraid_lint_counter(0x06); // 减 1 → 0x05
        m.write_ram(ram_map::FRAME_COUNT, 0);
        for s in 1..=5u8 {
            let mut e = m.enemy(s);
            e.set_status(EnemyStatus::Active);
            e.set_delay(0xAA);
        }
        m.enemy(2).set_status(EnemyStatus::Resting); // 槽 2 = x $20
        kraid_try_launch_lint(&mut m);
        assert_eq!(m.kraid_lint_counter(), 0x05);
        assert_eq!(m.enemy(2).delay(), 0x08, "槽 $20 装填 8 帧");
        assert_eq!(m.enemy(1).delay(), 0xAA, "其余槽不动");
    }

    /// 全在空中（asm:292-294）：三个 lint 都非 Resting ⇒ 计数回滚到本帧初值。
    #[test]
    fn lint_launch_rolls_back_when_all_airborne() {
        // asm: kraid.asm:292-294
        let mut m = stage();
        m.set_kraid_lint_counter(0x06);
        m.write_ram(ram_map::FRAME_COUNT, 0);
        for s in 1..=3u8 {
            m.enemy(s).set_status(EnemyStatus::Active);
        }
        kraid_try_launch_lint(&mut m);
        assert_eq!(m.kraid_lint_counter(), 0x06, "回滚这一减");
    }

    /// Nail 装填径（asm:303-341）：y=0x38（(0x38*2)&$0F==0）⇒ 槽 $40 delay←8。
    #[test]
    fn nail_launch_primes_first_resting_nail() {
        // asm: kraid.asm:326-341
        let mut m = stage();
        m.set_kraid_nail_counter(0x39); // 减 1 → 0x38
        m.write_ram(ram_map::FRAME_COUNT, 0);
        for s in 1..=5u8 {
            let mut e = m.enemy(s);
            e.set_status(EnemyStatus::Active);
            e.set_delay(0xAA);
        }
        m.enemy(4).set_status(EnemyStatus::Resting); // 槽 4 = x $40
        kraid_try_launch_nail(&mut m);
        assert_eq!(m.kraid_nail_counter(), 0x38);
        assert_eq!(m.enemy(4).delay(), 0x08);
        assert_eq!(m.enemy(5).delay(), 0xAA);
    }

    // ---- KraidUpdateProjectile（asm:109-222）--------------------------------

    /// `@remove` + `@resting`（asm:143-201）：空槽被重新摆到 Kraid 手上并起 Resting。
    /// 旧 type=0x09 ⇒ bank4 表给 health=$FF / initDelay=0 / forceDelay=1。
    #[test]
    fn projectile_remove_reinitializes_on_kraid_hand() {
        // asm: kraid.asm:143-201
        let mut m = stage();
        kraid_at(&mut m, 0x50, 0x40, 0, 0x00); // Kraid 朝右
        let mut e = m.enemy(1);
        e.set_status(EnemyStatus::NoEnemy);
        e.set_enemy_type(0x09);
        e.set_y(0x77);
        e.set_pos_x(0x77);
        e.set_hi(1);
        drop(e);
        kraid_update_projectile(&mut m, 0x10);
        let e = m.enemy(1);
        assert_eq!(e.status_raw(), 1, "0→1 Resting（asm:191-193）");
        assert_eq!(e.enemy_type(), 0x09, "type 由 KraidProjectileType 覆盖");
        assert_eq!(e.health(), 0xFF, "EnemyHealthTbl[09]");
        assert_eq!(e.delay(), 0x00, "EnemyInitDelayTbl[09]");
        assert_eq!(e.data0d(), 0x01, "EnemyForceSpeedTowardsSamusDelayTbl[09]");
        assert_eq!(e.data05(), 0x00, "抄 Kraid 的整字节 data05");
        assert_eq!(e.y(), 0x45, "0x50 + OffsetY[0]=-11");
        assert_eq!(e.pos_x(), 0x4A, "0x40 + OffsetX[0]=+10（朝右）");
        assert_eq!(e.hi(), 0, "Temp0B & $01");
    }

    /// 朝左（Kraid data05 bit0=1）走 OffsetX 第二列（-10），并抄 bit0。
    #[test]
    fn projectile_offset_x_uses_kraid_facing_bit() {
        // asm: kraid.asm:155-170
        let mut m = stage();
        kraid_at(&mut m, 0x50, 0x40, 0, 0x01); // 朝左
        m.enemy(3).set_status(EnemyStatus::NoEnemy);
        m.enemy(3).set_enemy_type(0x09);
        kraid_update_projectile(&mut m, 0x30);
        // 槽 3 的 OffsetX = [+14, -14]（`KraidProjectileOffsetX` 每槽 2 B）。
        assert_eq!(m.enemy(3).pos_x(), 0x32, "0x40 - 14");
        assert_eq!(m.enemy(3).data05(), 0x01, "data05 整字节含朝向 bit0");
        assert_eq!(m.enemy(3).enemy_type(), 0x09);
    }

    /// **表序保真**：`CommonJump_0E` 用的是**旧** type（asm:145），而 type 在
    /// `@resting` 才被覆盖（asm:162）。旧 type=0x0B ⇒ health/initDelay 取 0B 的表值，
    /// 而新 type=0x09 的表值不同——证明顺序没被"顺手修正"。
    #[test]
    fn projectile_init_tables_use_old_type_before_type_overwrite() {
        // asm: kraid.asm:143-163
        let mut m = stage();
        kraid_at(&mut m, 0x50, 0x40, 0, 0x00);
        m.enemy(1).set_status(EnemyStatus::NoEnemy);
        m.enemy(1).set_enemy_type(0x0B);
        kraid_update_projectile(&mut m, 0x10);
        let e = m.enemy(1);
        assert_eq!(e.enemy_type(), 0x09, "新 type 已覆盖");
        let health_old = game_data::assets::enemy_health(2)[0x0B];
        let health_new = game_data::assets::enemy_health(2)[0x09];
        assert_ne!(health_old, health_new, "表值必须可分（否则测不出顺序）");
        assert_eq!(e.health(), health_old, "health 来自旧 type（CommonJump_0E 先跑）");
        let delay_old = game_data::assets::enemy_init_delay(2)[0x0B];
        let delay_new = game_data::assets::enemy_init_delay(2)[0x09];
        assert_ne!(delay_old, delay_new, "表值必须可分");
        assert_eq!(e.delay(), delay_old, "delay 同来自旧 type");
    }

    /// 越界径（asm:186-190）：初始位置越界 ⇒ 只留 @remove 已写的 type/表值，
    /// **不**落 status（仍 0）、不落位置。
    #[test]
    fn projectile_out_of_bounds_leaves_status_zero_and_position_untouched() {
        // asm: kraid.asm:186-190
        let mut m = stage();
        // Kraid x=0xFA + 10 进位 ⇒ ApplySpeedToPosition 失败（横滚才允许环绕，默认竖直）。
        kraid_at(&mut m, 0x50, 0xFA, 0, 0x00);
        let mut e = m.enemy(1);
        e.set_status(EnemyStatus::NoEnemy);
        e.set_enemy_type(0x09);
        e.set_y(0x31);
        e.set_pos_x(0x32);
        drop(e);
        kraid_update_projectile(&mut m, 0x10);
        let e = m.enemy(1);
        assert_eq!(e.status_raw(), 0, "越界 ⇒ 不起 Resting");
        assert_eq!(e.y(), 0x31, "位置不落盘");
        assert_eq!(e.pos_x(), 0x32);
        assert_eq!(e.enemy_type(), 0x09, "@remove 的写已发生（顺序保真）");
    }

    /// 非 lint/nail type（asm:115-118）+ 不可见（asm:120-123）两径直出/回收。
    #[test]
    fn projectile_skips_other_types_and_recycles_invisible_ones() {
        // asm: kraid.asm:115-123
        let mut m = stage();
        kraid_at(&mut m, 0x50, 0x40, 0, 0x00);
        // 非 lint/nail：原样不动。
        proj(&mut m, 1, EnemyStatus::Active, 0x0B, 0x02);
        kraid_update_projectile(&mut m, 0x10);
        assert_eq!(m.enemy(1).status_raw(), 2);
        assert_eq!(m.enemy(1).y(), 0x10, "非 lint/nail 直出");

        // lint 但不可见（data05 bit1 清）⇒ @remove（回收成 Resting 并在手上）。
        proj(&mut m, 2, EnemyStatus::Active, 0x09, 0x00);
        kraid_update_projectile(&mut m, 0x20);
        assert_eq!(m.enemy(2).status_raw(), 1, "不可见 ⇒ 回收");
        // 槽 2 的 Offset = (Y -3, X +12)：0x50-3 / 0x40+12。
        assert_eq!(m.enemy(2).y(), 0x4D);
        assert_eq!(m.enemy(2).pos_x(), 0x4C);
    }

    /// 冻结径（asm:124-136）：status=4(Frozen) 且 prevStatus=1(Resting) ⇒ 回 @resting；
    /// prevStatus≠1 ⇒ 直出（位置不动）。同一格两分支 ⇒ 单帧判别。
    #[test]
    fn projectile_frozen_branch_depends_on_prev_status() {
        // asm: kraid.asm:124-136
        let mut m = stage();
        kraid_at(&mut m, 0x50, 0x40, 0, 0x00);
        for s in [1u8, 2] {
            let mut e = m.enemy(s);
            e.set_status(EnemyStatus::Frozen);
            e.set_enemy_type(0x09);
            e.set_data05(0x02);
            e.set_y(0x10);
            e.set_pos_x(0x10);
            e.set_hi(0);
            drop(e);
            // `PrevStatus`（`$040C`）只有 getter（`state.rs:465`）——用 `en_addr` 具名换算写。
            m.enemy(s).set_prev_status(0x01); // 冻前 Resting
        }
        kraid_update_projectile(&mut m, 0x10);
        assert_eq!(m.enemy(1).y(), 0x45, "prevStatus=1 ⇒ 回 @resting 重新定位");
        assert_eq!(m.enemy(1).status_raw(), 4, "@resting 不改非 0 status");

        m.enemy(2).set_prev_status(0x02); // 冻前 Active
        m.enemy(2).set_y(0x10);
        m.enemy(2).set_pos_x(0x10);
        kraid_update_projectile(&mut m, 0x20);
        assert_eq!(m.enemy(2).y(), 0x10, "prevStatus≠1 ⇒ 直出");
    }

    /// 爆炸态回收（asm:127-128）：status=3 ⇒ @remove ⇒ 回收成 Resting。
    #[test]
    fn projectile_explode_status_recycles() {
        // asm: kraid.asm:127-128
        let mut m = stage();
        kraid_at(&mut m, 0x50, 0x40, 0, 0x00);
        proj(&mut m, 1, EnemyStatus::Explode, 0x09, 0x02);
        kraid_update_projectile(&mut m, 0x10);
        assert_eq!(m.enemy(1).status_raw(), 1);
        assert_eq!(m.enemy(1).y(), 0x45);
    }

    /// `KraidUpdateAllProjectiles`（asm:96-105）只走槽 1..5、**不含槽 0**（Kraid 本体）。
    #[test]
    fn all_projectiles_loop_skips_slot_zero() {
        // asm: kraid.asm:96-105
        let mut m = stage();
        kraid_at(&mut m, 0x50, 0x40, 0, 0x00);
        m.enemy(0).set_status(EnemyStatus::Active);
        m.enemy(0).set_y(0x60);
        for s in 1..=5u8 {
            let mut e = m.enemy(s);
            e.set_status(EnemyStatus::NoEnemy);
            e.set_enemy_type(0x09);
        }
        kraid_update_all_projectiles(&mut m);
        assert_eq!(m.enemy(0).status_raw(), 2, "槽 0 不被弹体循环处理");
        assert_eq!(m.enemy(0).y(), 0x60);
        for s in 1..=5u8 {
            assert_eq!(m.enemy(s).status_raw(), 1, "槽 {s} 回收成 Resting");
        }
        // 槽 4/5 是 Nail（KraidProjectileType 后两位）
        assert_eq!(m.enemy(4).enemy_type(), 0x0A);
        assert_eq!(m.enemy(5).enemy_type(), 0x0A);
    }
}
