//! 敌人框架：七态处理器与 AI 派发（原 `behaviour.rs` 语义分层，纯搬移）。
//!
//! asm 根：`UpdateEnemy_*` 07:F3BE..F57D；`UpdateEnemyCommon` 三入口 07:F410。

use crate::enemy::anim_tick::update_explosion_anim;
use crate::render::enemy_sprite::{draw_enemy, draw_explosion};
use crate::enemy::crawler::crawler_ai;
use crate::enemy::motion::*;
use crate::enemy::prep::*;
use crate::enemy::skree::skree_ai;
use crate::enemy::sidehopper::{ripper_i_ai, ripper_ii_ai, sidehopper_ai};
use crate::frame::tick::*;
use crate::enemy::state::{slot_of, EnemyStatus};
use crate::ram_map;

/// UpdateEnemy_Resting_UpdateEnData1F（**asm 内容锚** `: 07:F676`——r26）：
/// L977B 条目×2→再 asl×3→and #$C0→data1F（**bit2-3→bit6-7**）。
pub(crate) fn update_en_data1f(mem: &mut crate::frame::Manager, x: u8) {
    let v = load_table_at_977b(mem, x);
    let v = v.wrapping_shl(3); // asl×3（含载器 ×2=条目×16）
    mem.write_wram(0x6AF4 + x as u16 + 15, v & 0xC0); // EnsExtra.data1F
}

/// UpdateEnemy_Resting_TryBecomingActive（**asm 内容锚** `: 07:F7BA`——r26）：
/// dec delay 非零⇒RTS；归零：data05 bit3=0⇒inc 退回；=1⇒(PipeBug type7⇒SFX)+
/// status inc→Active+InitEnActiveAnimIndex。
pub(crate) fn try_becoming_active(mem: &mut crate::frame::Manager, x: u8) {
    // 【goal-r16 修】delay 偏移 +7→+9（ROM F7BA `DE 09 04`=dec $0409——Ens.delay=+9；
    // 旧 +7=movementInstrIndex⇒delay 永不归零⇒敌人恒 Resting 不激活）。data0A(+10)=朝向。
    // 【2026-10-31 原地写修】asm `dec Ens.0.delay`（F7BA `DE 09 04`）=**原地写**：
    // dec 结果先落内存（激活径 delay=0——emu f798 实录 delay=00；旧形激活径不写回⇒
    // delay 恒 1 残留）；bit3=0 径 `inc` 退回=写回原值。
    let d = mem.enemy(slot_of(x)).delay(); // Ens.delay（ROM F7BA 实测 $0409）
    mem.enemy(slot_of(x)).set_delay(d.wrapping_sub(1)); // dec 原地（asm DE 09 04）
    if d.wrapping_sub(1) != 0 {
        return;
    }
    let d5 = mem.enemy(slot_of(x)).data05();
    if d5 & 0x08 == 0 {
        mem.enemy(slot_of(x)).set_delay(d); // inc 退回（asm F7C6 `EE 09 04`——undo）
        return;
    }
    // @becomeActive
    let t = mem.enemy(slot_of(x)).enemy_type();
    if t == 7 {
        crate::unported!("bank07:CBA4"); // SFX_OutOfPipe（声事件——出声面）
    }
    #[allow(unreachable_code)]
    {
        mem.enemy(slot_of(x)).inc_status(); // status→Active（asm `inc`，非枚举映射）
        // 【goal-轮14 激活帧直证】status→Active 写点（首动 1 帧差源）。
        init_en_active_anim_index(mem, x); // F699（Active 动画初始化）
        // 【goal-轮28 激活 RNG 播种】（asm :11436-11450 逐字）：movementIndex←
        // EnemyMovementChoices[(FC^RNG1)&bitflag +1]——**激活时 RNG 选播**（旧形
        // load_enemy 固定表=偏离）。Zoomer（type5）choice=单支（bitflag=0）⇒恒 0 ✓；
        // Skree（type4）choice=7 单支 ✓——首房双敌型播种等价（多支敌型随件）。
        // 【asm 07:F7E0-07:F84B 逐字，按区表】movementIndex 播种：
        //   ldy type ; lda EnemyMovementChoiceOffset,y      ← 每区 16 项偏移表
        //   clc adc #<EnemyMovementChoices ; → Temp00          ← 指针 = 表基 + 偏移
        //   lda FrameCount ; eor RandomNumber1 ; and (Temp00),y ← 掩码=记录首字节（可能值数-1）
        //   tay ; iny ; lda (Temp00),y → movementIndex
        // 旧形把「Brinstar 七型」硬编码成 match + 恒 `&0x01` 掩码，等于**替 ROM 猜表**；
        // 现改为真读 `area.{x}.en_aux` 的两张按区表。
        let fc = mem.read_ram(crate::ram_map::FRAME_COUNT);
        let rng1 = mem.read_ram(crate::ram_map::RANDOM1);
        let area = crate::frame::cur_area(mem);
        let ti = (t as usize) & 0x0F;
        let base = game_data::assets::enemy_movement_choice_offset(area)[ti] as usize;
        let tbl = game_data::assets::enemy_movement_choices(area);
        let mask = tbl[base]; // 记录首字节 = 可能值数-1
        let pick = (fc ^ rng1) & mask;
        let slot = base + 1 + pick as usize;
        let mov_idx = match tbl.get(slot) {
            Some(v) => *v,
            None => {
                // 原版此处读表外 ROM（Tourian 的 Rinka 偏移即落在表尾），行为不可从包内复现
                crate::frame::DIVERGENCES.with(|d| d.borrow_mut().push(0x97D1));
                0
            }
        };
        mem.enemy(slot_of(x)).set_movement_index(mov_idx); // Ens.movementIndex（$0408）

        // 【asm 07:F81A-07:F839】`jsr LoadTableAt977B`（= L977B[type]<<1）：bit7 置位
        // ⇒ 该敌型走「加速度+速度+子像素」模型 ⇒ 按 movementIndex 从四张表初始化，
        // 再按朝向翻转符号。四表在 ROM 里连续（accelY/accelX/speedY/speedX 各 20 B），
        // asm 各自 base+y 查表（y 越界即读进下一张表）——故这里用同一 80 B 窗按 base 索引。
        if game_data::assets::enemy_facing(area)[ti] & 0x40 != 0 {
            mem.enemy(slot_of(x)).set_data06(0); // Ens.speedSubPixelY（$0406）
            mem.enemy(slot_of(x)).set_speed_subpixel_x(0); // Ens.speedSubPixelX（$0407）
            let w = game_data::assets::en_accel_speed_window(area);
            let mi = mov_idx as usize;
            let load = |off: usize| -> u8 {
                match w.get(off + mi) {
                    Some(v) => *v,
                    None => {
                        crate::frame::DIVERGENCES.with(|d| d.borrow_mut().push(0x972B));
                        0
                    }
                }
            };
            mem.enemy(slot_of(x)).set_accel_y(load(0) as i8); // EnsExtra.accelY（$6AFE）
            mem.enemy(slot_of(x)).set_accel_x(load(20) as i8); // EnsExtra.accelX（$6AFF）
            mem.enemy(slot_of(x)).set_speed_y(load(40) as i8); // Ens.speedY（$0402）
            mem.enemy(slot_of(x)).set_speed_x(load(60) as i8); // Ens.speedX（$0403）
            // 朝向：data05 bit7=0 ⇒ 水平（bit0=1 表示朝左，需翻 X）；
            //       bit7=1 ⇒ 垂直（bit2=1 表示朝下，需翻 Y）。asm 07:F832。
            let d5b = mem.enemy(slot_of(x)).data05();
            let bank = crate::frame::area_bank(area);
            let no_flip = game_data::assets::rom_table_968b(bank as usize, t) & 0x20 != 0;
            if d5b & 0x80 == 0 {
                if d5b & 0x01 != 0 && !no_flip {
                    // EnemyIfMoveFailedHorizontal_Bounce@flipSpeedAndAccel（$81D1）
                    let a = mem.enemy(slot_of(x)).accel_x();
                    mem.enemy(slot_of(x)).set_accel_x(a.wrapping_neg()); // TwosComplement
                    let sp = mem.enemy(slot_of(x)).speed_x();
                    mem.enemy(slot_of(x)).set_speed_x(sp.wrapping_neg());
                    let sub = mem.enemy(slot_of(x)).speed_subpixel_x();
                    mem.enemy(slot_of(x)).set_speed_subpixel_x(sub.wrapping_neg());
                }
            } else if d5b & 0x04 != 0 && !no_flip {
                // EnemyIfMoveFailedVertical_Bounce@flipSpeedAndAccel（$8206）
                let a = mem.enemy(slot_of(x)).accel_y();
                mem.enemy(slot_of(x)).set_accel_y(a.wrapping_neg());
                let sp = mem.enemy(slot_of(x)).speed_y();
                mem.enemy(slot_of(x)).set_speed_y(sp.wrapping_neg());
                let sub = mem.enemy(slot_of(x)).data06();
                mem.enemy(slot_of(x)).set_data06(sub.wrapping_neg());
            }
        }
        // 【asm 07:F839】`lda #~$20 ; jmp AndEnData05` ⇒ 清 data05 bit5（解除休止标记）。
        let d5c = mem.enemy(slot_of(x)).data05();
        mem.enemy(slot_of(x)).set_data05(d5c & 0xDF);
    }
}

/// InitEnActiveAnimIndex（**asm 内容锚** `: 07:F699`——r26）：ENEMY_ACTIVE_ANIM[y]
/// →resetAnimIndex/animIndex；InitOffset 非零⇒animIndex 递减该值。
pub(crate) fn init_en_active_anim_index(mem: &mut crate::frame::Manager, x: u8) {
    use crate::enemy::anim_idx::AnimIdx;
    let y = enemy_type_times2_plus_facing(mem, x);
    let area = crate::frame::cur_area(mem);
    let idx = AnimIdx::at(area, game_data::assets::enemy_active_anim(area)[y as usize & 0x1F]);
    if idx.off() != mem.enemy(slot_of(x)).reset_anim_index() {
        mem.enemy(slot_of(x)).set_reset_anim_index(idx.off()); // resetAnimIndex
        mem.enemy(slot_of(x)).set_anim_index(idx.off()); // animIndex
        mem.enemy(slot_of(x)).set_anim_delay(0);
        let t = mem.enemy(slot_of(x)).enemy_type() as usize & 0x0F;
        let off = game_data::assets::enemy_active_anim_init_offset(area)[t] & 0x7F;
        // InitOffset 递减只在**本区**内（`AnimIdx` 不换区）
        mem.enemy(slot_of(x)).set_anim_index(idx.back_within_area(off).off());
    }
}

/// `InitEnActiveAnimIndex_NoInitOffset`（**asm 内容锚** `07:FBCA`，multiviola 用）：
/// 与 F699 同径但**不做** InitOffset 递减——resetAnimIndex/animIndex 都直接置
/// `EnemyActiveAnimIndex[y]`，animDelay 清零（`SetEnAnimIndex 07:F690` + `ClearEnAnimDelay`）。
pub(crate) fn init_en_active_anim_index_no_init_offset(mem: &mut crate::frame::Manager, x: u8) {
    use crate::enemy::anim_idx::AnimIdx;
    let y = enemy_type_times2_plus_facing(mem, x);
    let area = crate::frame::cur_area(mem);
    let idx = AnimIdx::at(area, game_data::assets::enemy_active_anim(area)[y as usize & 0x1F]);
    if idx.off() != mem.enemy(slot_of(x)).reset_anim_index() {
        mem.enemy(slot_of(x)).set_reset_anim_index(idx.off());
        mem.enemy(slot_of(x)).set_anim_index(idx.off());
        mem.enemy(slot_of(x)).set_anim_delay(0);
    }
}

/// EnemyReactToSamusProjectile（**asm 内容锚** `: 07:F536`——r26）：isHit(+4) bit5=0⇒
/// RTS（未中弹径——移动对拍主径）；中弹径=冰冻/伤害族（打敌人流程件——随件）。
pub(crate) fn enemy_react_to_projectile(mem: &mut crate::frame::Manager, x: u8) {
    // 【实录（asm $F536 全径（findings §11）+探针偏移（+0B=health/+0D=data0D/
    // +0E=weaponAction/+0F=specialAttribs（ens_probe trace（F863/F437/F41B 锚）））】
    let px = x as u16;
    let e4 = 0x0400 + px + 4; // isHit
    if mem.read_ram(e4) & 0x20 == 0 {
        return; // 非被弹⇒RTS
    }
    let sa = mem.enemy(slot_of(px as u8)).special_attribs(); // specialAttribs（Lx321 prevStatus 记账）
    let st0 = mem.enemy(slot_of(x)).status_raw(); // EnsExtra.status（保存面）
    // 【goal-轮31 保存式修】prevStatus←**status 低6|sa 高2**（asm Lx321 语义：恢复=status
    // &$3F+sa&$C0 逆向）——旧形 psc|sa 缺 status 面⇒恢复 status=0 ✗。
    mem.write_ram(0x0400 + px + 0x0C, (st0 & 0x3F) | (sa & 0xC0)); // prevStatus
    let wa = mem.enemy(slot_of(px as u8)).weapon_action(); // weaponAction
    let miniboss = sa & 0x40 != 0;
    // 冰冻臂（IceBeam=wa 3（constants.asm:104！旧形误 2=Wave）+非 miniboss+非冻）
    // ——status 4=冻结
    let st = mem.enemy(slot_of(x)).status_raw(); // EnsExtra.status
    if wa == 3 && !miniboss && st != 4 {
        mem.enemy(slot_of(x)).set_status(EnemyStatus::Frozen); // status←Frozen
        mem.enemy(slot_of(px as u8)).set_data0d(0x40); // data0D←$40（冻计时）
        mem.write_ram(e4, 0); // isHit 清
        return;
    }
    // Lx317：HP==$FF⇒无效（SFX（声=桩）+清 isHit/weaponAction）
    let hp = mem.enemy(slot_of(px as u8)).health(); // health（trace +0B）
    if hp == 0xFF {
        mem.write_ram(e4, 0);
        mem.write_ram(0x0400 + px + 0x0E, 0);
        return;
    }
    // 弹（wa_Missile）/螺（$81）⇒瞬杀（非 miniboss）
    if !miniboss && (wa == 0x0B || wa == 0x81) {
        mem.write_ram(e4, 0);
        explode_enemy(mem, x);
        return;
    }
    // Lx322：status←Hurt(6)+specialAttribs←$0A/$03
    mem.enemy(slot_of(x)).set_status(EnemyStatus::Hurt);
    mem.write_ram(0x0400 + px + 0x0F, if miniboss { 0x03 } else { 0x0A });
    // HP dec（波=2/束=1（Lx324/325））⇒0⇒爆
    let dec = if wa == 2 { 2u8 } else { 1u8 }; // wa_WaveBeam=2（constants.asm:103！旧形误 4）
    let nhp = hp.wrapping_sub(dec);
    mem.enemy(slot_of(px as u8)).set_health(nhp);
    mem.write_ram(e4, 0);
    if nhp == 0 {
        explode_enemy(mem, x);
    }
}

/// ExplodeEnemy（**asm 内容锚** `: 07:F611`）：status←3+EnemyDeathAnimIndex[type×2+向]
/// →InitEnAnimIndex+chunkyExplosionTimer←A（+06）——EnExplosions 池=小爆效果件
/// （分卡（记账））。
pub(crate) fn explode_enemy(mem: &mut crate::frame::Manager, x: u8) {
    let px = x as u16;
    mem.enemy(slot_of(x)).set_status(EnemyStatus::Explode); // status←Explode
    // `bit Temp0A_EnSpecialAttribs / bvs @endIf_A` + `lda weaponAction / cmp #wa_RegularBeam+1 /
    // bcs @endIf_A`（07:F617-F623）：**非 miniboss 且被普通束击中（weaponAction ≤ wa_RegularBeam=1）
    // ⇒ 跳过 chunky 爆炸动画，直接 EnemyCompleteDeath@SkipChunkyExplosion**（立刻变掉落物）。
    // 旧形无此分支 ⇒ 一律播 0x40 帧爆炸动画，掉落物晚 ~64 帧出现（录像 f2151：emu status=5
    // 画掉落帧、rust status=3 画爆散帧 ⇒ OAM 多/少各 4 枚精灵）。
    // 注意 asm **不 return**：随后仍执行 InitEnAnimIndex + `sta chunkyExplosionTimer`。
    let sa_x = mem.enemy(slot_of(px as u8)).special_attribs(); // specialAttribs
    let wa_x = mem.enemy(slot_of(px as u8)).weapon_action(); // weaponAction
    if sa_x & 0x40 == 0 && wa_x < 0x02 {
        enemy_complete_death(mem, x);
    }
    let ty = mem.enemy(slot_of(px as u8)).enemy_type() as usize & 0x1F;
    let d05 = mem.enemy(slot_of(px as u8)).data05();
    let facing: usize = if d05 & 0x80 != 0 { ((d05 >> 3) & 1) as usize } else { (d05 & 1) as usize };
    let idx = ((ty << 1) | facing) & 0x1F;
    // `EnemyDeathAnimIndex ; 各 bank $960B`（32 B = type×2+朝向）——**按区不同**：
    // bank1 `27 27 29 29…` / bank2 `28 28 28 28…` / bank3 `08 08 08 08…` /
    // bank4 `27 27 29 29…`(尾部与 bank1 不同) / bank5 `23 23 23 23…`。
    // 旧形是单常量取 bank1 ⇒ 其余四区爆体帧全用 Brinstar 的表（bank 维度丢失）。
    let area = crate::frame::cur_area(mem);
    let anim = game_data::assets::enemy_death_anim(area)[idx];
    // `InitEnAnimIndex` 的落点：reset/index 同写、delay 归零——三件是一个动画切换动作。
    let frame = mem.enemy(slot_of(px as u8)).anim_frame();
    mem.enemy(slot_of(px as u8)).set_anim(crate::enemy::state::Anim { frame, delay: 0, reset: anim, index: anim });
    mem.enemy(slot_of(px as u8)).set_data06(anim); // chunkyExplosionTimer←A（asm `sta`（+06））

    // 【步骤 1/3：只做 spawn，update/draw 未接】`ExplodeEnemy` 尾件 07:F62E-07:F673：
    // 在 `EnExplosions` 池（`$04C0`，4 槽 × 8 B，额外面 `$6BB4`）里找第一个 status==0 的槽，
    // `InitEnAnimIndex(AreaExplosionAnimIndex)` + `delay←$0A` + `inc status` +
    // `quantity←0`（miniboss ⇒ 3）+ 位置三字节抄自敌槽。
    // asm：`ldx #EnExplosions.0-Ens / @loop: lda EnsExtra.status,x / beq @exitLoop /
    // adc #_sizeof_EnExplosions.0 / cmp #EnExplosions-Ens+_sizeof` ⇒ 池满则整段跳过。
    let area_now = crate::frame::cur_area(mem);
    for slot in 0..crate::enemy::explosion::SLOT_COUNT {
        if !mem.explosion(slot).is_free() {
            continue;
        }
        let ex_anim = game_data::assets::explosion_anim(area_now);
        let enemy_y = mem.enemy(slot_of(px as u8)).y();
        let enemy_x = mem.enemy(slot_of(px as u8)).pos_x();
        let enemy_hi = mem.enemy(slot_of(px as u8)).hi();
        // 注意：位置三字节抄自**敌槽**，不是爆炸池原值。
        let mut e = mem.explosion(slot);
        e.set_anim(ex_anim);
        e.set_delay(0x0A);
        e.occupy(); // asm `inc status`（池域：0→1＝占用）
        e.set_quantity(if sa_x & 0x40 != 0 { 3 } else { 0 });
        e.set_pos(enemy_y, enemy_x);
        e.set_hi(enemy_hi);
        break;
    }
}


/// 【步骤 2/3：update 接上，绘制待接】`UpdateAllEnemyExplosions ; 07:FA9D`：
/// 4 槽逐个 `stx PageIndex`，status≠0 才 `jsr UpdateEnemyExplosion`。
pub(crate) fn update_all_enemy_explosions(mem: &mut crate::frame::Manager) {
    for slot in 0..crate::enemy::explosion::SLOT_COUNT {
        let x = crate::enemy::explosion::ens_offset(slot);
        mem.set_page_index(x); // PageIndex（asm `stx PageIndex`）
        if mem.explosion(slot).is_free() {
            continue;
        }
        update_enemy_explosion(mem, slot, x);
    }
}

/// `UpdateEnemyExplosion ; 07:FAB4`：`dec delay`；归零 ⇒ `delay←$0C`、`dec quantity`，
/// 负或零 ⇒ `RemoveEnemy`（07:FA18 把 status 清 0）；`delay==9` 时按 quantity 查
/// `@explosionMovementTable ; 07:FAEA`（(dy,dx)：单爆 (0,0) / 三连末 ($0C,$1C) /
/// 次 ($10,$F0) / 首 ($F0,$08)）位移；尾件 `lda #$80 / sta ObjectCntrl / lda #$03 /
/// jmp AnimDrawEnemy`（`AnimDrawEnemy ; 07:F97E` = `jsr UpdateEnemyAnim` + `jmp DrawEnemy`）。
fn update_enemy_explosion(mem: &mut crate::frame::Manager, slot: u8, x: u8) {
    let d = mem.explosion(slot).delay().wrapping_sub(1);
    mem.explosion(slot).set_delay(d);
    if d == 0 {
        mem.explosion(slot).set_delay(0x0C); // 3 视频帧 × 4 动画帧
        let q = mem.explosion(slot).quantity().wrapping_sub(1);
        mem.explosion(slot).set_quantity(q);
        // `bmi @removeExplosion / bne @dontRemoveExplosion` ⇒ 负或零都移除
        if q == 0 || q & 0x80 != 0 {
            mem.explosion(slot).free(); // RemoveEnemy
        }
    }
    if mem.explosion(slot).delay() == 0x09 {
        // @applySpeed（跳过瓦判）：直接给 y/x 速度
        const TBL: [(u8, u8); 4] = [(0x00, 0x00), (0x0C, 0x1C), (0x10, 0xF0), (0xF0, 0x08)];
        let q = (mem.explosion(slot).quantity() as usize).min(3);
        let (dy, dx) = TBL[q];
        apply_explosion_speed(mem, slot, dy, dx);
    }
    // 尾件（07:FAE1）：`lda #$80 / sta ObjectCntrl / lda #$03 / jmp AnimDrawEnemy`
    // `AnimDrawEnemy ; 07:F97E` = `jsr UpdateEnemyAnim` + `jmp DrawEnemy`；
    // `DrawEnemy ; 07:DD8B` 先比 `animFrame == _id_FrameNone($FF)` ⇒ 空帧转 ClearObjectCntrl。
    mem.set_obj_cntrl(0x80); // ObjectCntrl（覆盖形：非 MSB 位作调色板/优先级）
    update_explosion_anim(mem, slot, 0x03);
    draw_explosion(mem, slot);
}

/// `@applySpeed`（`ApplySpeedToPosition ; 07:FD8F` 的敌侧入口）。失败（carry clear）
/// ⇒ 调用方 `RemoveEnemy`。
fn apply_explosion_speed(mem: &mut crate::frame::Manager, slot: u8, dy: u8, dx: u8) {
    let horizontal_scroll = mem.read_ram(0x0049) & 0x02 != 0; // ScrollDir
    let mut hi = mem.explosion(slot).hi();
    let mut y = mem.explosion(slot).y();
    let mut pos_x = mem.explosion(slot).x();
    let mut fail = false;
    if dy != 0 {
        if dy & 0x80 == 0 {
            let (n, c) = y.overflowing_add(dy);
            if c || n >= 0xF0 {
                if horizontal_scroll {
                    fail = true;
                } else {
                    hi = hi.wrapping_add(1);
                }
                y = n.wrapping_add(0x10);
            } else {
                y = n;
            }
        } else {
            let (n, c) = y.overflowing_add(dy);
            if !c {
                if horizontal_scroll {
                    fail = true;
                } else {
                    hi = hi.wrapping_add(1);
                }
                y = n.wrapping_sub(0x10);
            } else {
                y = n;
            }
        }
    }
    if !fail && dx != 0 {
        let (n, c) = pos_x.overflowing_add(dx);
        if c != (dx & 0x80 == 0) {
            // 正速进位 / 负速借位 ⇒ 越界
            if !horizontal_scroll {
                fail = true;
            } else {
                hi = hi.wrapping_add(1);
            }
        }
        pos_x = n;
    }
    if fail {
        mem.explosion(slot).free(); // RemoveEnemy
        return;
    }
    mem.explosion(slot).set_pos(y, pos_x);
    mem.explosion(slot).set_hi(hi & 0x01);
}

/// UpdateEnemy_Explode（**asm 内容锚** `: 07:F40D`（jmp ChooseEnemyAIRoutine（爆在 AI
/// Exit 臂（noMoveNoAnim 显示）+chunkyExplosionTimer（+06）计时⇒CompleteDeath））。
pub(crate) fn update_enemy_explode(mem: &mut crate::frame::Manager, x: u8) {
    let px = x as u16;
    update_enemy_anim(mem, x, 3);
    update_enemy_common_no_move_no_anim(mem, x);
    let timer = mem.enemy(slot_of(px as u8)).data06(); // chunkyExplosionTimer（trace +06）
    if timer > 0 {
        mem.enemy(slot_of(px as u8)).set_data06(timer - 1);
    } else {
        enemy_complete_death(mem, x);
    }
}

/// UpdateEnemy_Frozen（$F54D）：data0D 计时（×8（asm「frozen timer (* 8)」））⇒回 Active。
pub(crate) fn update_enemy_frozen(mem: &mut crate::frame::Manager, x: u8) {
    let px = x as u16;
    update_enemy_common_no_move_no_anim(mem, x);
    let d = mem.enemy(slot_of(px as u8)).data0d();
    if d > 0 {
        mem.enemy(slot_of(px as u8)).set_data0d(d - 1);
    } else {
        mem.enemy(slot_of(x)).set_status(EnemyStatus::Active); // 回 Active
    }
}

/// UpdateEnemy_Hurt（**asm 内容锚** `: 07:F4EE`——goal-轮31 全谱）：`dec specialAttribs
/// /bne @exit`（**dec 环绕**——归零帧才恢复）→prevStatus(+0x0C)：高 2 位回
/// specialAttribs+低 6 位回 status（**prevStatus&$3F**——旧形硬编码 2=偏离）→Metroid
/// 冻结保持（L977B bit5）→@exit 调色（$80|PRIORITY→setHurtPalette）。
pub(crate) fn update_enemy_hurt(mem: &mut crate::frame::Manager, x: u8) {
    // `UpdateEnemy_Hurt ; 07:F4EE` 的**顺序**：先递减 hitstun（并可能恢复 status），
    // 再 `$6B←$80|$00|PRIORITY`（**受伤敌恒用调色板 0**，`@exit ; 07:F510`），
    // 最后 `jmp UpdateEnemyCommon@setHurtPalette`（只画、不动不推帧）。
    // 旧形把「画」放在递减之前 ⇒ ①hitstun 结束那一帧仍按受伤态画（晚一帧恢复）
    // ②从不写 $6B ⇒ 受伤敌的调色板取自帧记录（录像 f2137 实证 attr=A2 vs A0）。
    let px = x as u16;
    let d = mem.enemy(slot_of(px as u8)).special_attribs(); // specialAttribs=+0x0F（hitstun 延）
    let nd = d.wrapping_sub(1); // dec 环绕（asm DE 0F 04）
    mem.write_ram(0x0400 + px + 0x0F, nd);
    if nd == 0 {
        // hitstun over：prevStatus(+0x0C) 高 2 位→specialAttribs/低 6 位→status
        let prev = mem.enemy(slot_of(px as u8)).prev_status(); // prevStatus
        mem.write_ram(0x0400 + px + 0x0F, prev & 0xC0); // 高 2 位回
        mem.enemy(slot_of(x))
        .set_status(EnemyStatus::from_byte(prev & 0x3F)); // status←prevStatus&$3F（asm :10810-10812）
        // `jsr LoadTableAt977B / and #$20`（该例程返回 `val<<1` ⇒ 实为原字节 bit4）：
        // 置位=Metroid 被导弹打中时仍保持冻结（`FreezeEnemyAndRememberPrevStatus`）。
        let t = (mem.read_wram(0x6AF4 + px + 0x0E) & 0x0F) as usize;
        let flags = game_data::assets::enemy_facing(crate::frame::cur_area(mem))
            .get(t)
            .copied()
            .unwrap_or(0);
        if flags & 0x10 != 0 {
            mem.write_ram(0x0400 + px + 0x0C, mem.read_wram(0x6AF4 + px)); // prevStatus←status
            mem.enemy(slot_of(x)).set_status(EnemyStatus::Frozen); // status←enemyStatus_Frozen
        }
    }
    // @exit：受伤敌调色板覆盖（$80|$00|PRIORITY）——置 MSB 后 @noMoveNoAnim 的
    // tough 判据（`lda ObjectCntrl / bmi`）自然跳过，等价于 asm 直跳 @setHurtPalette。
    mem.set_obj_cntrl(0x80 | 0x20);
    update_enemy_common_no_move_no_anim(mem, x);
}

/// UpdateEnemy_Pickup（**asm 内容锚** `: 07:F483`）：碰触（isHit&$24）⇒RemoveEnemy+
/// 回血/弹（+30/+20/+小（animFrame 0x80=弹药））；data0D 计时（*4）⇒到期移除。
pub(crate) fn update_enemy_pickup(mem: &mut crate::frame::Manager, x: u8) {
    let px = x as u16;
    // 碰触判**先于**显面（静态面尾清 isHit（+4←0）——序正（asm F483 先判后显））
    let hit = mem.enemy(slot_of(px as u8)).is_hit();
    if hit & 0x24 != 0 {
        // 碰触⇒回血/弹+移除
        let frame = mem.enemy(slot_of(px as u8)).anim_frame();
        if frame == 0x80 {
            let m = mem.read_wram(crate::ram_map::MISSILE_COUNT);
            mem.write_wram(crate::ram_map::MISSILE_COUNT, m.saturating_add(5));
        } else if frame == 0x89 {
            let h = mem.read_ram(0x0106u16);
            mem.write_ram(0x0106u16, h.saturating_add(30));
        } else {
            let h = mem.read_ram(0x0106u16);
            mem.write_ram(0x0106u16, h.saturating_add(20));
        }
        mem.enemy(slot_of(x)).set_status(EnemyStatus::NoEnemy); // RemoveEnemy（槽释）
        return;
    }
    // `@pickupWasNotTouched ; 07:F4D4`：`lda FrameCount / and #$03 / bne @endIf_D` ⇒
    // **每 4 帧**才递减 data0D；`dec data0D / bne @endIf_D` ⇒ 减后为 0 才 RemoveEnemy。
    if mem.read_ram(crate::ram_map::FRAME_COUNT) & 0x03 == 0 {
        let d = mem.enemy(slot_of(px as u8)).data0d().wrapping_sub(1);
        mem.enemy(slot_of(px as u8)).set_data0d(d);
        if d == 0 {
            mem.enemy(slot_of(x)).set_status(EnemyStatus::NoEnemy); // RemoveEnemy（到期）
        }
    }
    // 闪色（`07:F4E2`）：`lda FrameCount / and #$02 / lsr / ora #$80|OAMDATA_PRIORITY /
    // sta ObjectCntrl` ⇒ 覆盖态在 0x80（调色板 0）与 0xA1（调色板 1）间每 2 帧交替。
    let fc = mem.read_ram(crate::ram_map::FRAME_COUNT);
    mem.set_obj_cntrl(((fc & 0x02) >> 1) | 0x80 | 0x20);
    update_enemy_common_no_move_no_anim(mem, x);
}

/// EnemyCompleteDeath（**asm 内容锚** `: 07:DCF5`）：status←Pickup+data0D←$60+
/// 1/16 无掉+PickupTypeTable[rand&7]→animFrame（表 ROM $DE35=80 81 89）——qty 限
/// 域=分卡（记账）。
pub(crate) fn enemy_complete_death(mem: &mut crate::frame::Manager, x: u8) {
    let px = x as u16;
    mem.enemy(slot_of(x)).set_status(EnemyStatus::Pickup); // status←Pickup
    mem.enemy(slot_of(px as u8)).set_data0d(0x60); // data0D←$60（拾取计时）
    let r = mem.read_ram(crate::ram_map::RANDOM1);
    if r < 0x10 {
        // 1/16 无掉⇒移除（LDD5B 非 Tourian）
        mem.enemy(slot_of(x)).set_status(EnemyStatus::NoEnemy);
        return;
    }
    // `jsr ReadTableAt968B / sta Temp00`（$968B 项存 Temp00，本径后续不读）→
    // `jsr LoadTableAt977B / and #$20 / sta EnsExtra.0.type,x`（07:DD1A-07:DD23）：
    // **把 type 覆盖成 0 / $20**（`LoadTableAt977B` 返回 `val<<1` ⇒ `and #$20` 测的是
    // 原字节 bit4，置位写 $20）。这一步不能省——紧随其后的
    // `jsr GetEnemyTypeTimes2PlusFacingDirection` 用的是**覆盖后的 type**：
    // 录像 f2152 实证 om 掉它会让死亡动画索引从 0x27 变成 0x41（OAM/调色板全错）。
    let t_idx = (mem.enemy(slot_of(px as u8)).enemy_type() & 0x3F) as usize;
    let facing_bit = if t_idx < 64 {
        game_data::assets::enemy_facing(crate::frame::cur_area(mem))[t_idx] & 0x20
    } else { 0 };
    mem.write_wram(0x6AF4 + px + 14, facing_bit); // type←0 / $20
    let tbl = [0x80u8, 0x81, 0x89, 0x80, 0x81, 0x89, 0x81, 0x89]; // PickupTypeTable（ROM $DE35）
    let frame = tbl[(r & 7) as usize];
    mem.enemy(slot_of(px as u8)).set_anim_frame(frame); // animFrame←掉落型
}

/// `ChooseEnemyAIRoutine`——**按区 bank** 的 16 项跳表（`prg{1,2,3,4,5}_*.asm` 各一份）。
/// 同一 `type` 号在不同区含义不同（见 `enemy/ai_table.rs` 的对照表）：派发必须按
/// `(area, type)` 查表，不能用全局 `match`。未端口例程与空指针都记账后走占位臂。
pub(crate) fn choose_enemy_ai(mem: &mut crate::frame::Manager, x: u8) {
    use crate::enemy::ai_table::{routine_for, Routine};
    let t = mem.enemy(slot_of(x)).enemy_type(); // type=+14
    let area = crate::frame::cur_area(mem);
    match routine_for(area, t) {
        Routine::Crawler => crawler_ai(mem, x),
        Routine::Skree => skree_ai(mem, x),
        Routine::SidehopperFloor => sidehopper_ai(mem, x, false),
        Routine::SidehopperCeiling => sidehopper_ai(mem, x, true),
        Routine::RipperI => ripper_i_ai(mem, x),
        Routine::RipperII => ripper_ii_ai(mem, x),
        Routine::Waver => crate::enemy::waver::waver_ai(mem, x),
        Routine::Rio => crate::enemy::rio::rio_ai(mem, x),
        Routine::PipeBug => crate::enemy::pipe_bug::pipe_bug_ai(mem, x),
        Routine::Rinka => crate::enemy::rinka::rinka_ai(mem, x),
        Routine::Metroid => crate::enemy::metroid::metroid_ai(mem, x),
        Routine::Squeept => crate::enemy::squeept::squeept_ai(mem, x),
        Routine::Swooper00 => crate::enemy::swooper::swooper_ai_00(mem, x),
        Routine::Swooper01 => crate::enemy::swooper::swooper_ai_01(mem, x),
        Routine::Multiviola => crate::enemy::multiviola::multiviola_ai(mem, x),
        // `DragonAIRoutine`（`02:9AE9`，仅 Norfair）；`PolypAIRoutine`（Norfair `02:9B64` /
        // Ridley `05:9B16`，本卡只接 Norfair 表项）。两者都自吐 EnProjectile。
        Routine::Dragon => crate::enemy::dragon::dragon_ai(mem, x),
        Routine::Polyp => crate::enemy::polyp::polyp_ai(mem, x),
        // `KraidAIRoutine` / `KraidLintAIRoutine` / `KraidNailAIRoutine`
        // （bank4 `04:9AB4/9AE4/9B2C`，`prg4_kraid.asm:757` include）。
        Routine::Kraid => crate::enemy::kraid::kraid_ai(mem, x),
        Routine::KraidLint => crate::enemy::kraid::kraid_lint_ai(mem, x),
        Routine::KraidNail => crate::enemy::kraid::kraid_nail_ai(mem, x),
        // `RemoveEnemy ; 07:FA18`：`lda #enemyStatus_NoEnemy / sta EnsExtra.0.status,x / rts`
        Routine::RemoveEnemy => mem.enemy(slot_of(x)).set_status(EnemyStatus::NoEnemy),
        // `RTS_95CB`：Kraid bank4 的占位项，asm 直接 `rts`——什么都不做。
        Routine::Rts95CB => {}
        // 空指针（Brinstar 0B–0F）与未端口例程：记账 + 占位臂保可玩。
        Routine::Null | Routine::Unported(_) => {
            crate::frame::DIVERGENCES.with(|d| d.borrow_mut().push(0x95E5));
            update_enemy_common_no_move_no_anim(mem, x);
        }
    }
}

/// UpdateEnemy_Resting（**asm 内容锚** `: 07:F3BE`——r26 壳：30fps 门/件序 fail-loud）。
pub(crate) fn enemy_resting(mem: &mut crate::frame::Manager, x: u8) {
    let d = mem.enemy(slot_of(x)).data05();
    if d & 0x40 == 0 {
        // 30fps 标 clear⇒全件（bit6 set 时跳过=30fps 节拍）
        mem.write_wram(0x6AF4 + x as u16 + 13, 0); // jumpDsplcmnt←0
        mem.enemy(slot_of(x)).set_speed_subpixel_x(0); // movementInstrIndex←0
        mem.write_ram(0x0400 + x as u16 + 10, 0); // data0A←0
        enemy_force_speed(mem, x); // F6B9（件序首——速向萨姆斯力）
        enemy_distance_threshold(mem, x); // F75B（件序 2——阈距判）
        init_en_resting_anim_index(mem, x); // F682（件序 3）
        update_en_data1f(mem, x); // F676（件序 4）
        let delay = mem.enemy(slot_of(x)).delay();
        if delay != 0 {
            try_becoming_active(mem, x); // F7BA（件序 5——延迟归零⇒激活）
        }
    }
    enemy_react_to_projectile(mem, x);
    choose_enemy_ai(mem, x);
}

/// UpdateEnemy_Active（**asm 内容锚** `: 07:F3E6`→Active_BranchA(F401)→BranchB(F40A)→F40D）。
/// 【goal-r16 复核修正 G2'】F401 三件=ForceSpeed+Threshold+RemoveIfInLava（ROM
/// `20 B9 F6 20 5B F7 20 1E F5`）；AI 分派在 BranchB→F40D（Resting/Active 共用）。
/// 旧形三径皆 `unported!` ⇒ 敌人激活即 panic。
pub(crate) fn enemy_active(mem: &mut crate::frame::Manager, x: u8) {
    let d = mem.enemy(slot_of(x)).data05();
    if d & 0x40 == 0 {
        // bit6 clear ⇒ 走 BranchA/bit5 径（bit6 set⇒30fps 直跳 BranchB）
        if d & 0x20 != 0 {
            // bit5 set⇒delay←EnemyInitDelayTbl[type]；status Active→Resting
            let t = mem.enemy(slot_of(x)).enemy_type(); // EnsExtra.type
            let delay = game_data::assets::enemy_init_delay(crate::frame::cur_area(mem)).get(t as usize).copied().unwrap_or(0);
            mem.enemy(slot_of(x)).set_delay(delay); // Ens.delay(+9)
            mem.enemy(slot_of(x)).dec_status(); // Active→Resting（asm `dec`）
            // 【goal-轮17 Resting 转换标记】Active→Resting（bit5 静息径）。
        } else {
            // Active_BranchA（F401）：ForceSpeed(F6B9)+Threshold(F75B)+RemoveIfInLava(F51E)
            enemy_force_speed(mem, x);
            enemy_distance_threshold(mem, x);
            enemy_remove_if_in_lava(mem, x);
        }
    }
    // BranchB（F40A→F40D）：enemy_react_to_projectile + choose_enemy_ai（AI 分派）
    enemy_react_to_projectile(mem, x);
    choose_enemy_ai(mem, x);
}

/// RemoveEnemyIfItIsInLava（**asm 内容锚** `: 07:F51E`）：横滚房+敌 y≥$EC（熔岩）⇒清槽。
pub(crate) fn enemy_remove_if_in_lava(mem: &mut crate::frame::Manager, x: u8) {
    let scroll_dir = mem.read_ram(crate::ram_map::SCROLL_DIR);
    if scroll_dir < 2 {
        return; // 竖滚房⇒exit（asm cmp #$02/bcc）
    }
    let y = mem.enemy(slot_of(x)).y(); // Ens.y
    if y < 0xEC {
        return; // 熔岩上方⇒exit
    }
    // 熔岩中⇒RemoveEnemy（清槽）
    mem.enemy(slot_of(x)).set_status(EnemyStatus::NoEnemy); // 槽空
}

/// `UpdateEnemyCommon ; 07:F410`（入口 1：anim + move）。
pub(crate) fn update_enemy_common_anim_move(mem: &mut crate::frame::Manager, x: u8, anim: u8) {
    update_enemy_anim(mem, x, anim);
    enemy_move(mem, x);
    update_enemy_common_no_move_no_anim(mem, x);
}

/// `UpdateEnemyCommon_Decide`（prg1_brinstar.asm:863）：按 `EnemyStatusPreAI` 选入口；
/// 活动态 anim 索引取 `$00`、休止态取 `$01`（均由调用者预设）。
pub(crate) fn update_enemy_common_decide(mem: &mut crate::frame::Manager, x: u8) {
    let active = mem.read_ram(0x0000);
    let resting = mem.read_ram(0x0001);
    update_enemy_common_decide_with_beats(mem, x, active, resting);
}

/// Decide 的**节拍直传**变体：asm 调用方（Squeept/Swooper/Multiviola 等）在尾件前
/// `lda #n / sta $00 / sta $01`，Decide 随即原样读回——零页槽是纯中间量，本变体把
/// 两个即时量直接作参数传入（行为等价，避免新增零页裸写）。
pub(crate) fn update_enemy_common_decide_with_beats(
    mem: &mut crate::frame::Manager,
    x: u8,
    active: u8,
    resting: u8,
) {
    match mem.read_ram(0x0081) {
        1 => update_enemy_common_no_move_anim(mem, x, resting),
        3 => update_enemy_common_no_move_no_anim(mem, x),
        _ => update_enemy_common_anim_move(mem, x, active),
    }
}

/// `@noMove ; 07:F438`（入口 3：anim + 尾件，无移动）。
pub(crate) fn update_enemy_common_no_move_anim(mem: &mut crate::frame::Manager, x: u8, anim: u8) {
    update_enemy_anim(mem, x, anim);
    update_enemy_common_no_move_no_anim(mem, x);
}

/// UpdateEnemyCommon@noMoveNoAnim（**asm 内容锚** `: 07:F416`——r26）：tough 调色→
/// status≠0⇒DrawEnemy→isHit/weaponAction 清。
/// 【2026-10-31 伪 init 摘除】旧形头部多一次 init_en_resting_anim_index（自称"缺面修"
/// ——asm F416 **无此调用**；resting init 属 Resting 壳 F3BE 件序 3，enemy_resting 已
/// 接）。副作用=每帧把 rAI/aI 打回 resting 序列：转角 reorient（CrawlerReorientSprite
/// 同 tick 更早写 3E）被抹（crawlprobe f814 起 anim 恒地板序列 35/36/37 根因）+激活帧
/// resting init 绕过 bit6 30fps 门（f797 rAI=35 vs emu 00——壳本应被门跳过）。
pub(crate) fn update_enemy_common_no_move_no_anim(mem: &mut crate::frame::Manager, x: u8) {
    let special = mem.enemy(slot_of(x)).special_attribs(); // specialAttribs=+15
    if special & 0x80 != 0 {
        let oc = mem.obj_cntrl();
        if oc & 0x80 == 0 {
            mem.set_obj_cntrl(0x80 | 0x03 | 0x20);
        }
    }
    if mem.read_wram(0x6AF4 + x as u16) != 0 {
        // 【goal-轮39 绘径可见旗写件】（asm :6178-6186 逐字：`and #$FD/ora (flag<<1)/sta
        // data05`——**bit1=可见旗**）——绘径写面（flag=可见判定 ✓）。
        let d5 = mem.enemy(slot_of(x)).data05();
        mem.enemy(slot_of(x)).set_data05((d5 & 0xFD) | 0x02); // 绘⇒可见旗置
        draw_enemy(mem, x);
    }
    mem.write_ram(0x0400 + x as u16 + 4, 0); // isHit=+4
    mem.write_ram(0x0400 + x as u16 + 14, 0); // weaponAction=+14
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame::Manager;

    /// Resting→Active 的 movementIndex 播种与四表初始化（asm `07:F7E0-07:F839`）。
    ///
    /// 断言的是**从 ROM 表推出来的值**，不是实现的中间量：
    /// `movementIndex = EnemyMovementChoices[ off + 1 + ((FC^RNG1) & mask) ]`，
    /// 且当 `L977B[type] & $40` 置位时，四张表按该索引装载（越界读=同窗下一张表）。
    #[test]
    fn become_active_reads_per_area_movement_choices_and_tables() {
        let mut m = Manager::default();
        m.write_ram(ram_map::IN_AREA, 0x00); // Brinstar
        let x = 0u8;
        let e = 0x0400 + x as u16;
        let q = 0x6AF4 + x as u16;
        // type 0（Sidehopper 地面）：L977B[0]=0x64（bit6 置 ⇒ 走加速模型）
        m.write_wram(q + 14, 0x00);
        // status 语义（asm 07:F351 跳表 0=ExitSub/1=Resting/2=Active/…/>=7 RemoveEnemy）
        m.write_wram(q, 0x01); // status=Resting
        m.write_ram(e + 9, 0x01); // delay=1（dec 后归零）
        m.write_ram(e + 5, 0x08); // data05 bit3=1（允许激活）；bit0=0（朝右）
        m.write_ram(ram_map::FRAME_COUNT, 0x05);
        m.write_ram(ram_map::RANDOM1, 0x00);

        try_becoming_active(&mut m, x);

        // 表推：offset[0]=0x00 ⇒ mask=0x01 ⇒ pick=(5^0)&1=1 ⇒ choices[0+1+1]=0x02
        let mov = m.read_ram(e + 8);
        assert_eq!(mov, 0x02, "movementIndex 应来自按区表（Brinstar Sidehopper 双支）");
        // 四表（Brinstar，index=2，ROM `$972B/$973F/$9753/$9767` 实读）：
        // accelY 0x30 / accelX 0x00 / speedY 0xFE / speedX 0x02
        assert_eq!(m.read_wram(q + 10), 0x30, "accelY");
        assert_eq!(m.read_wram(q + 11), 0x00, "accelX");
        assert_eq!(m.read_ram(e + 2), 0xFE, "speedY");
        assert_eq!(m.read_ram(e + 3), 0x02, "speedX");
        assert_eq!(m.read_ram(e + 6), 0x00, "speedSubPixelY 清 0");
        assert_eq!(m.read_ram(e + 7), 0x00, "speedSubPixelX 清 0");
        assert_eq!(m.read_ram(e + 5) & 0x20, 0x00, "data05 bit5 清（解除休止）");
        assert_eq!(m.read_wram(q) & 0x0F, 0x02, "status Resting(1)→Active(2)");
    }

    /// 朝左（data05 bit0=1）⇒ 水平四量取反（asm `$81D1 @flipSpeedAndAccel`）。
    #[test]
    fn become_active_flips_horizontal_when_facing_left() {
        let mut m = Manager::default();
        m.write_ram(ram_map::IN_AREA, 0x00);
        let x = 0u8;
        let (e, q) = (0x0400u16, 0x6AF4u16);
        m.write_wram(q + 14, 0x00);
        m.write_wram(q, 0x00);
        m.write_ram(e + 9, 0x01);
        m.write_ram(e + 5, 0x09); // bit3=1（可激活）+ bit0=1（朝左）
        m.write_ram(ram_map::FRAME_COUNT, 0x05);
        m.write_ram(ram_map::RANDOM1, 0x00);

        try_becoming_active(&mut m, x);

        assert_eq!(m.read_ram(e + 8), 0x02);
        assert_eq!(m.read_wram(q + 11), 0x00u8.wrapping_sub(0x00), "accelX 取反");
        assert_eq!(m.read_ram(e + 3), 0x00u8.wrapping_sub(0x02), "speedX 取反（0x02→0xFE）");
    }
}

#[cfg(test)]
mod tests_hurt_palette {
    use super::*;
    use crate::frame::Manager;

    /// `UpdateEnemy_Hurt ; 07:F4EE`：受伤敌**恒用调色板 0**（`@exit ; 07:F510`
    /// `$6B←$80|$00|PRIORITY`），且 `$FF` 帧终由 `ClearObjectCntrl` 收尾。
    /// 录像 f2137 实证：旧形从不写 `$6B` ⇒ 受伤爬虫按帧记录调色板 2 画
    /// （attr 0xA2 vs 0xA0，98px）。
    #[test]
    fn hurt_enemy_uses_palette_zero_and_clears_cntrl() {
        let mut m = Manager::default();
        let x = 0x10u16; // 槽 1
        m.write_wram(0x6AF4 + x, 6); // status = Hurt
        m.write_ram(0x0400 + x, 0x40); // y（屏幕内）
        m.write_ram(0x0400 + x + 1, 0x40); // x
        m.write_ram(0x0400 + x + 0x0F, 2); // hitstun
        m.enemy(slot_of(x as u8)).set_anim_frame(0x39); // animFrame（表 1）
        m.write_ram(0x005B, 0);
        super::update_enemy_hurt(&mut m, 0x10);
        assert_eq!(m.read_ram(0x006B), 0, "$FF 帧终清 ObjectCntrl");
        assert_eq!(m.read_ram(0x0400 + x + 0x0F), 1, "hitstun 递减");
        // attr 的低 2 位＝调色板：受伤径为 0x80|0x00|PRIORITY ⇒ 调色板位为 0
        let attr = m.read_ram(0x0202);
        assert_eq!(attr & 0x03, 0, "受伤敌调色板 0（attr={attr:#04X}）");
    }
}

#[cfg(test)]
mod tests_explode_regular_beam {
    use crate::frame::Manager;

    /// `ExplodeEnemy ; 07:F611`：被**普通束**打死的非 miniboss **立刻变掉落物**
    /// （status←Pickup=5，`EnemyCompleteDeath@SkipChunkyExplosion`），跳过 chunky 爆炸动画。
    /// 录像 f2151 实证：旧形一律 status=3 播 0x40 帧爆炸 ⇒ 掉落晚 ~64 帧、OAM 差 4 枚精灵。
    #[test]
    fn regular_beam_kill_becomes_pickup_immediately() {
        let mut m = Manager::default();
        let x = 0x10u16; // 槽 1
        m.write_ram(0x0400 + x + 0x0E, 0x01); // weaponAction = wa_RegularBeam
        m.write_ram(0x0400 + x + 0x0F, 0x00); // specialAttribs：非 miniboss
        m.write_ram(crate::ram_map::RANDOM1, 0x20); // ≥$10 ⇒ 有掉落
        super::explode_enemy(&mut m, 0x10);
        assert_eq!(m.read_wram(0x6AF4 + x), 5, "status←Pickup（不等爆炸动画）");
        assert_eq!(m.read_ram(0x0400 + x + 0x0D), 0x60, "data0D←$60 拾取计时");
    }

    /// miniboss（specialAttribs bit6）或非普通束（如导弹 0x0B）⇒ **仍走爆炸动画**
    /// （status=3），与 asm 的 `bvs @endIf_A` / `bcs @endIf_A` 一致。
    #[test]
    fn miniboss_or_missile_kill_stays_exploding() {
        let mut m = Manager::default();
        let x = 0x10u16;
        m.write_ram(0x0400 + x + 0x0E, 0x0B); // 导弹
        m.write_ram(0x0400 + x + 0x0F, 0x00);
        super::explode_enemy(&mut m, 0x10);
        assert_eq!(m.read_wram(0x6AF4 + x), 3, "导弹杀：爆炸动画");
        let mut m2 = Manager::default();
        m2.write_ram(0x0400 + x + 0x0E, 0x01);
        m2.write_ram(0x0400 + x + 0x0F, 0x40); // miniboss
        super::explode_enemy(&mut m2, 0x10);
        assert_eq!(m2.read_wram(0x6AF4 + x), 3, "miniboss：爆炸动画");
    }
}

#[cfg(test)]
mod tests_enemy_explosion_pool {
    use crate::frame::Manager;

    fn stage_enemy(m: &mut Manager, x: u16) {
        m.write_ram(0x0400 + x, 0x50); // y
        m.write_ram(0x0400 + x + 1, 0x80); // x
        m.write_wram(0x6AF4 + x + 14, 0); // type
        m.write_ram(0x0400 + x + 0x0E, 0x0B); // weaponAction＝导弹（走爆炸动画）
    }

    /// `ExplodeEnemy` 尾件（07:F62E-F673）：杀敌后在该池里放一个爆炸特效，
    /// 位置三字节抄自敌槽；单爆 quantity=0、miniboss quantity=3。
    #[test]
    fn explode_enemy_spawns_explosion_slot() {
        let mut m = Manager::default();
        let x = 0x10u16;
        stage_enemy(&mut m, x);
        super::explode_enemy(&mut m, x as u8);
        assert_eq!(m.read_wram(0x6AF4 + 0xC0), 1, "池槽 0 status←Active");
        assert_eq!(m.read_ram(0x04C0), 0x50, "y 抄自敌槽");
        assert_eq!(m.read_ram(0x04C1), 0x80, "x 抄自敌槽");
        assert_eq!(m.read_ram(0x04C6), 0x0A, "delay←$0A");
        assert_eq!(m.read_ram(0x04C7), 0x00, "单爆 quantity=0");
        assert_eq!(m.read_wram(0x6AF4 + 0xC6), 0x43, "animIndex＝AreaExplosionAnimIndex（Brinstar $43）");
    }

    /// miniboss（specialAttribs bit6）⇒ quantity=3（三连爆）。
    #[test]
    fn miniboss_spawns_triple_explosion() {
        let mut m = Manager::default();
        let x = 0x10u16;
        stage_enemy(&mut m, x);
        m.write_ram(0x0400 + x + 0x0F, 0x40);
        super::explode_enemy(&mut m, x as u8);
        assert_eq!(m.read_ram(0x04C7), 3, "miniboss ⇒ quantity=3");
    }

    /// `UpdateEnemyExplosion ; 07:FAB4`：delay 递减；归零 ⇒ delay←$0C、quantity--；
    /// 单爆（quantity 0→$FF）当拍 `RemoveEnemy`（status 清 0）。池满时 spawn 跳过。
    #[test]
    fn explosion_lifetime_expires_and_frees_slot() {
        let mut m = Manager::default();
        m.write_wram(0x6AF4 + 0xC0, 1); // status=Active
        m.write_ram(0x04C6, 0x02); // delay
        m.write_ram(0x04C0, 0x40);
        m.write_ram(0x04C1, 0x40);
        super::update_all_enemy_explosions(&mut m);
        assert_eq!(m.read_ram(0x04C6), 0x01, "delay 递减");
        assert_eq!(m.read_wram(0x6AF4 + 0xC0), 1, "未到期仍 Active");
        super::update_all_enemy_explosions(&mut m);
        assert_eq!(m.read_ram(0x04C6), 0x0C, "delay 归零后重置 $0C");
        assert_eq!(m.read_wram(0x6AF4 + 0xC0), 0, "单爆 quantity 用尽 ⇒ RemoveEnemy");
    }

    /// 池满（4 槽都占）⇒ spawn 整段跳过，不覆盖任何槽。
    #[test]
    fn spawn_skipped_when_pool_full() {
        let mut m = Manager::default();
        for i in 0..4u16 {
            m.write_wram(0x6AF4 + 0xC0 + i * 8, 1);
        }
        let x = 0x10u16;
        stage_enemy(&mut m, x);
        super::explode_enemy(&mut m, x as u8);
        assert_eq!(m.read_ram(0x04C0), 0x00, "槽 0 未被覆盖");
        assert_eq!(m.read_ram(0x04C6), 0x00, "槽 0 delay 未被写");
    }
}

#[cfg(test)]
mod tests_pickup_flicker {
    use super::*;
    use crate::frame::Manager;

    fn stage(m: &mut Manager, fc: u8, die_delay: u8) -> u8 {
        let x: u8 = 0x10;
        m.write_ram(crate::ram_map::FRAME_COUNT, fc);
        m.write_wram(0x6AF4 + x as u16, 0x05); // status＝Pickup
        m.write_ram(0x0400 + x as u16, 0x50); // y（屏内）
        m.write_ram(0x0400 + x as u16 + 1, 0x60); // x（屏内）
        m.enemy(slot_of(x as u8)).set_radius_y(0x08); // radiusY
        m.enemy(slot_of(x as u8)).set_radius_x(0x08); // radiusX
        m.write_ram(0x0400 + x as u16 + 0x0D, die_delay);
        x
    }

    /// `@pickupWasNotTouched ; 07:F4D4` 尾：`lda FrameCount / and #$02 / lsr /
    /// ora #$80|OAMDATA_PRIORITY / sta ObjectCntrl` ⇒ 覆盖态在 0x80/0xA1 间每 2 帧交替
    /// （emu f2170 attr=0x21 vs rust 0x20 的成因）。
    #[test]
    fn pickup_flickers_palette_every_two_frames() {
        let mut m = Manager::default();
        let x = stage(&mut m, 0x00, 0x60);
        m.enemy(slot_of(x as u8)).set_anim_frame(0x89); // animFrame＝小能量
        super::update_enemy_pickup(&mut m, x);
        assert_eq!(m.read_ram(0x0202), 0x20, "bit1 清 ⇒ 调色板 0（+PRIORITY）");
        assert_eq!(m.read_ram(0x006B), 0x00, "帧终 ClearObjectCntrl");
        let mut m = Manager::default();
        let x = stage(&mut m, 0x02, 0x60);
        m.enemy(slot_of(x as u8)).set_anim_frame(0x89);
        super::update_enemy_pickup(&mut m, x);
        assert_eq!(m.read_ram(0x0202), 0x21, "bit1 置 ⇒ 调色板 1（$80|$20|1）");
    }

    /// `lda FrameCount / and #$03 / bne @endIf_D` ⇒ data0D **每 4 帧**才递减一次。
    #[test]
    fn pickup_die_delay_ticks_every_four_frames() {
        for (fc, want) in [(0u8, 0x5Fu8), (1, 0x60), (2, 0x60), (3, 0x60)] {
            let mut m = Manager::default();
            let x = stage(&mut m, fc, 0x60);
            super::update_enemy_pickup(&mut m, x);
            assert_eq!(m.read_ram(0x0400 + x as u16 + 0x0D), want, "FrameCount={fc}");
        }
    }
}
