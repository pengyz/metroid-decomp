//! 敌人框架：七态处理器与 AI 派发（原 `behaviour.rs` 语义分层，纯搬移）。
//!
//! asm 根：`UpdateEnemy_*` 07:F3BE..F57D；`UpdateEnemyCommon` 三入口 07:F410。

use crate::enemy::anim::draw_enemy;
use crate::enemy::crawler::crawler_ai;
use crate::enemy::motion::*;
use crate::enemy::prep::*;
use crate::enemy::skree::skree_ai;
use crate::enemy::sidehopper::{ripper_i_ai, ripper_ii_ai, sidehopper_ai};
use crate::frame::tick::*;
use crate::ram_map;

/// UpdateEnemy_Resting_UpdateEnData1F（**asm 内容锚** `: 07:F676`——r26）：
/// L977B 条目×2→再 asl×3→and #$C0→data1F（**bit2-3→bit6-7**）。
pub(crate) fn update_en_data1f(mem: &mut crate::frame::FrameMemory, x: u8) {
    let v = load_table_at_977b(mem, x);
    let v = v.wrapping_shl(3); // asl×3（含载器 ×2=条目×16）
    mem.write_wram(0x6AF4 + x as u16 + 15, v & 0xC0); // EnsExtra.data1F
}

/// UpdateEnemy_Resting_TryBecomingActive（**asm 内容锚** `: 07:F7BA`——r26）：
/// dec delay 非零⇒RTS；归零：data05 bit3=0⇒inc 退回；=1⇒(PipeBug type7⇒SFX)+
/// status inc→Active+InitEnActiveAnimIndex。
pub(crate) fn try_becoming_active(mem: &mut crate::frame::FrameMemory, x: u8) {
    // 【goal-r16 修】delay 偏移 +7→+9（ROM F7BA `DE 09 04`=dec $0409——Ens.delay=+9；
    // 旧 +7=movementInstrIndex⇒delay 永不归零⇒敌人恒 Resting 不激活）。data0A(+10)=朝向。
    // 【2026-10-31 原地写修】asm `dec Ens.0.delay`（F7BA `DE 09 04`）=**原地写**：
    // dec 结果先落内存（激活径 delay=0——emu f798 实录 delay=00；旧形激活径不写回⇒
    // delay 恒 1 残留）；bit3=0 径 `inc` 退回=写回原值。
    let e = 0x0400 + x as u16 + 9; // Ens.delay（ROM F7BA 实测 $0409）
    let d = mem.read_ram(e);
    mem.write_ram(e, d.wrapping_sub(1)); // dec 原地（asm DE 09 04）
    if d.wrapping_sub(1) != 0 {
        return;
    }
    let d5 = mem.read_ram(0x0400 + x as u16 + 5);
    if d5 & 0x08 == 0 {
        mem.write_ram(e, d); // inc 退回（asm F7C6 `EE 09 04`——undo）
        return;
    }
    // @becomeActive
    let t = mem.read_wram(0x6AF4 + x as u16 + 14);
    if t == 7 {
        crate::unported!("bank07:CBA4"); // SFX_OutOfPipe（声事件——出声面）
    }
    #[allow(unreachable_code)]
    {
        let st = mem.read_wram(0x6AF4 + x as u16);
        mem.write_wram(0x6AF4 + x as u16, st.wrapping_add(1)); // status→Active
        // 【goal-轮14 激活帧直证】status→Active 写点（首动 1 帧差源）。
        init_en_active_anim_index(mem, x); // F699（Active 动画初始化）
        // 【goal-轮28 激活 RNG 播种】（asm :11436-11450 逐字）：movementIndex←
        // EnemyMovementChoices[(FC^RNG1)&bitflag +1]——**激活时 RNG 选播**（旧形
        // load_enemy 固定表=偏离）。Zoomer（type5）choice=单支（bitflag=0）⇒恒 0 ✓；
        // Skree（type4）choice=7 单支 ✓——首房双敌型播种等价（多支敌型随件）。
        let fc = mem.read_ram(crate::ram_map::FRAME_COUNT);
        let rng1 = mem.read_ram(crate::ram_map::RANDOM1);
        let pick = (fc ^ rng1) & 0x01; // bitflag=0x01（2 支形——多支表随件）
        // 【movement choice 表】（prg1_brinstar.asm:492-508 `EnemyMovementChoices` 逐项）：
        // 00 Floor Sidehopper=$01,$02 / 01 Ceiling=$03,$04 / 02 Waver=$05 / 03 Ripper=$06 /
        // 04 Skree=$07 / 05 Zoomer=manual(0) / 06 Rio=$08。两选一型用 `(FC^RNG)&1` 选播。
        let area = crate::frame::cur_area(mem);
        let mov_idx: u8 = if area == 0 {
            match t {
                0 => 0x01 | (pick & 0x01) as u8, // Sidehopper floor：$01/$02
                1 => 0x03 | (pick & 0x01) as u8, // Ceiling sidehopper：$03/$04
                2 => 0x05,                       // Waver
                3 => 0x06,                       // Ripper
                4 => 0x07,                       // Skree
                5 => 0x00,                       // Zoomer（manual——crawler_ai 自己走）
                6 => 0x08,                       // Rio（swoopers）
                _ => {
                    crate::frame::DIVERGENCES.with(|d| d.borrow_mut().push(0x97D1));
                    pick
                }
            }
        } else {
            // 其他区的 choice 表未端口（Norfair/Ridley/Tourian 各有自己的表）——记账不猜
            crate::frame::DIVERGENCES.with(|d| d.borrow_mut().push(0x97D1));
            pick
        };
        mem.write_ram(0x0400 + x as u16 + 8, mov_idx); // Ens.movementIndex=+8
    }
}

/// InitEnActiveAnimIndex（**asm 内容锚** `: 07:F699`——r26）：ENEMY_ACTIVE_ANIM[y]
/// →resetAnimIndex/animIndex；InitOffset 非零⇒animIndex 递减该值。
pub(crate) fn init_en_active_anim_index(mem: &mut crate::frame::FrameMemory, x: u8) {
    let y = enemy_type_times2_plus_facing(mem, x);
    let idx = game_data::assets::enemy_active_anim(crate::frame::cur_area(mem))[y as usize & 0x1F];
    if idx != mem.read_wram(0x6AF4 + x as u16 + 5) {
        mem.write_wram(0x6AF4 + x as u16 + 5, idx);
        mem.write_wram(0x6AF4 + x as u16 + 6, idx);
        mem.write_wram(0x6AF4 + x as u16 + 4, 0);
        let t = mem.read_wram(0x6AF4 + x as u16 + 14) as usize & 0x0F;
        let off = game_data::assets::enemy_active_anim_init_offset(crate::frame::cur_area(mem))[t] & 0x7F;
        let mut ai = idx;
        for _ in 0..off {
            ai = ai.wrapping_sub(1);
        }
        mem.write_wram(0x6AF4 + x as u16 + 6, ai);
    }
}

/// EnemyReactToSamusProjectile（**asm 内容锚** `: 07:F536`——r26）：isHit(+4) bit5=0⇒
/// RTS（未中弹径——移动对拍主径）；中弹径=冰冻/伤害族（打敌人流程件——随件）。
pub(crate) fn enemy_react_to_projectile(mem: &mut crate::frame::FrameMemory, x: u8) {
    // 【实录（asm $F536 全径（findings §11）+探针偏移（+0B=health/+0D=data0D/
    // +0E=weaponAction/+0F=specialAttribs（ens_probe trace（F863/F437/F41B 锚）））】
    let px = x as u16;
    let e4 = 0x0400 + px + 4; // isHit
    if mem.read_ram(e4) & 0x20 == 0 {
        return; // 非被弹⇒RTS
    }
    let sa = mem.read_ram(0x0400 + px + 0x0F); // specialAttribs（Lx321 prevStatus 记账）
    let st0 = mem.read_wram(0x6AF4 + px); // EnsExtra.status（保存面）
    // 【goal-轮31 保存式修】prevStatus←**status 低6|sa 高2**（asm Lx321 语义：恢复=status
    // &$3F+sa&$C0 逆向）——旧形 psc|sa 缺 status 面⇒恢复 status=0 ✗。
    mem.write_ram(0x0400 + px + 0x0C, (st0 & 0x3F) | (sa & 0xC0)); // prevStatus
    let wa = mem.read_ram(0x0400 + px + 0x0E); // weaponAction
    let miniboss = sa & 0x40 != 0;
    // 冰冻臂（IceBeam=wa 3（constants.asm:104！旧形误 2=Wave）+非 miniboss+非冻）
    // ——status 4=冻结
    let st = mem.read_wram(0x6AF4 + px); // EnsExtra.status
    if wa == 3 && !miniboss && st != 4 {
        mem.write_wram(0x6AF4 + px, 4); // status←Frozen
        mem.write_ram(0x0400 + px + 0x0D, 0x40); // data0D←$40（冻计时）
        mem.write_ram(e4, 0); // isHit 清
        return;
    }
    // Lx317：HP==$FF⇒无效（SFX（声=桩）+清 isHit/weaponAction）
    let hp = mem.read_ram(0x0400 + px + 0x0B); // health（trace +0B）
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
    mem.write_wram(0x6AF4 + px, 6);
    mem.write_ram(0x0400 + px + 0x0F, if miniboss { 0x03 } else { 0x0A });
    // HP dec（波=2/束=1（Lx324/325））⇒0⇒爆
    let dec = if wa == 2 { 2u8 } else { 1u8 }; // wa_WaveBeam=2（constants.asm:103！旧形误 4）
    let nhp = hp.wrapping_sub(dec);
    mem.write_ram(0x0400 + px + 0x0B, nhp);
    mem.write_ram(e4, 0);
    if nhp == 0 {
        explode_enemy(mem, x);
    }
}

/// ExplodeEnemy（**asm 内容锚** `: 07:F611`）：status←3+EnemyDeathAnimIndex[type×2+向]
/// →InitEnAnimIndex+chunkyExplosionTimer←A（+06）——EnExplosions 池=小爆效果件
/// （分卡（记账））。
pub(crate) fn explode_enemy(mem: &mut crate::frame::FrameMemory, x: u8) {
    let px = x as u16;
    mem.write_wram(0x6AF4 + px, 3); // status←Explode
    let ty = mem.read_wram(0x6AF4 + px + 14) as usize & 0x1F;
    let d05 = mem.read_ram(0x0400 + px + 5);
    let facing: usize = if d05 & 0x80 != 0 { ((d05 >> 3) & 1) as usize } else { (d05 & 1) as usize };
    let idx = ((ty << 1) | facing) & 0x1F;
    // `EnemyDeathAnimIndex ; 各 bank $960B`（32 B = type×2+朝向）——**按区不同**：
    // bank1 `27 27 29 29…` / bank2 `28 28 28 28…` / bank3 `08 08 08 08…` /
    // bank4 `27 27 29 29…`(尾部与 bank1 不同) / bank5 `23 23 23 23…`。
    // 旧形是单常量取 bank1 ⇒ 其余四区爆体帧全用 Brinstar 的表（bank 维度丢失）。
    let area = crate::frame::cur_area(mem);
    let anim = game_data::assets::enemy_death_anim(area)[idx];
    mem.write_wram(0x6AF4 + px + 5, anim); // resetAnimIndex
    mem.write_wram(0x6AF4 + px + 6, anim); // animIndex
    mem.write_wram(0x6AF4 + px + 4, 0); // animDelay
    mem.write_ram(0x0400 + px + 6, anim); // chunkyExplosionTimer←A（asm `sta`（+06））
}

/// UpdateEnemy_Explode（**asm 内容锚** `: 07:F40D`（jmp ChooseEnemyAIRoutine（爆在 AI
/// Exit 臂（noMoveNoAnim 显示）+chunkyExplosionTimer（+06）计时⇒CompleteDeath））。
pub(crate) fn update_enemy_explode(mem: &mut crate::frame::FrameMemory, x: u8) {
    let px = x as u16;
    update_enemy_anim(mem, x, 3);
    update_enemy_common_no_move_no_anim(mem, x);
    let timer = mem.read_ram(0x0400 + px + 6); // chunkyExplosionTimer（trace +06）
    if timer > 0 {
        mem.write_ram(0x0400 + px + 6, timer - 1);
    } else {
        enemy_complete_death(mem, x);
    }
}

/// UpdateEnemy_Frozen（$F54D）：data0D 计时（×8（asm「frozen timer (* 8)」））⇒回 Active。
pub(crate) fn update_enemy_frozen(mem: &mut crate::frame::FrameMemory, x: u8) {
    let px = x as u16;
    update_enemy_common_no_move_no_anim(mem, x);
    let d = mem.read_ram(0x0400 + px + 0x0D);
    if d > 0 {
        mem.write_ram(0x0400 + px + 0x0D, d - 1);
    } else {
        mem.write_wram(0x6AF4 + px, 2); // 回 Active
    }
}

/// UpdateEnemy_Hurt（**asm 内容锚** `: 07:F4EE`——goal-轮31 全谱）：`dec specialAttribs
/// /bne @exit`（**dec 环绕**——归零帧才恢复）→prevStatus(+0x0C)：高 2 位回
/// specialAttribs+低 6 位回 status（**prevStatus&$3F**——旧形硬编码 2=偏离）→Metroid
/// 冻结保持（L977B bit5）→@exit 调色（$80|PRIORITY→setHurtPalette）。
pub(crate) fn update_enemy_hurt(mem: &mut crate::frame::FrameMemory, x: u8) {
    let px = x as u16;
    update_enemy_common_no_move_no_anim(mem, x);
    let d = mem.read_ram(0x0400 + px + 0x0F); // specialAttribs=+0x0F（hitstun 延）
    let nd = d.wrapping_sub(1); // dec 环绕（asm DE 0F 04）
    mem.write_ram(0x0400 + px + 0x0F, nd);
    if nd != 0 {
        return; // bne @exit（未归零）
    }
    // hitstun over：prevStatus(+0x0C) 高 2 位→specialAttribs/低 6 位→status
    let prev = mem.read_ram(0x0400 + px + 0x0C); // prevStatus
    mem.write_ram(0x0400 + px + 0x0F, prev & 0xC0); // 高 2 位回
    mem.write_wram(0x6AF4 + px, prev & 0x3F); // status←prevStatus&$3F（asm :10810-10812）
}

/// UpdateEnemy_Pickup（**asm 内容锚** `: 07:F483`）：碰触（isHit&$24）⇒RemoveEnemy+
/// 回血/弹（+30/+20/+小（animFrame 0x80=弹药））；data0D 计时（*4）⇒到期移除。
pub(crate) fn update_enemy_pickup(mem: &mut crate::frame::FrameMemory, x: u8) {
    let px = x as u16;
    // 碰触判**先于**显面（静态面尾清 isHit（+4←0）——序正（asm F483 先判后显））
    let hit = mem.read_ram(0x0400 + px + 4);
    if hit & 0x24 != 0 {
        // 碰触⇒回血/弹+移除
        let frame = mem.read_wram(0x6AF4 + px + 3);
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
        mem.write_wram(0x6AF4 + px, 0); // RemoveEnemy（槽释）
        return;
    }
    update_enemy_common_no_move_no_anim(mem, x);
    let d = mem.read_ram(0x0400 + px + 0x0D);
    if d > 0 {
        mem.write_ram(0x0400 + px + 0x0D, d - 1);
    } else {
        mem.write_wram(0x6AF4 + px, 0); // 到期移除
    }
}

/// EnemyCompleteDeath（**asm 内容锚** `: 07:DCF5`）：status←Pickup+data0D←$60+
/// 1/16 无掉+PickupTypeTable[rand&7]→animFrame（表 ROM $DE35=80 81 89）——qty 限
/// 域=分卡（记账）。
pub(crate) fn enemy_complete_death(mem: &mut crate::frame::FrameMemory, x: u8) {
    let px = x as u16;
    mem.write_wram(0x6AF4 + px, 5); // status←Pickup
    mem.write_ram(0x0400 + px + 0x0D, 0x60); // data0D←$60（拾取计时）
    let r = mem.read_ram(crate::ram_map::RANDOM1);
    if r < 0x10 {
        // 1/16 无掉⇒移除（LDD5B 非 Tourian）
        mem.write_wram(0x6AF4 + px, 0);
        return;
    }
    let tbl = [0x80u8, 0x81, 0x89, 0x80, 0x81, 0x89, 0x81, 0x89]; // PickupTypeTable（ROM $DE35）
    let frame = tbl[(r & 7) as usize];
    mem.write_wram(0x6AF4 + px + 3, frame); // animFrame←掉落型
}

/// `ChooseEnemyAIRoutine`——**按区 bank** 的 16 项跳表（`prg{1,2,3,4,5}_*.asm` 各一份）。
/// 同一 `type` 号在不同区含义不同（见 `enemy/ai_table.rs` 的对照表）：派发必须按
/// `(area, type)` 查表，不能用全局 `match`。未端口例程与空指针都记账后走占位臂。
pub(crate) fn choose_enemy_ai(mem: &mut crate::frame::FrameMemory, x: u8) {
    use crate::enemy::ai_table::{routine_for, Routine};
    let t = mem.read_wram(0x6AF4 + x as u16 + 14); // type=+14
    let area = crate::frame::cur_area(mem);
    match routine_for(area, t) {
        Routine::Crawler => crawler_ai(mem, x),
        Routine::Skree => skree_ai(mem, x),
        Routine::SidehopperFloor => sidehopper_ai(mem, x, false),
        Routine::SidehopperCeiling => sidehopper_ai(mem, x, true),
        Routine::RipperI => ripper_i_ai(mem, x),
        Routine::RipperII => ripper_ii_ai(mem, x),
        Routine::Waver => crate::enemy::waver::waver_ai(mem, x),
        // `RemoveEnemy ; 07:FA18`：`lda #enemyStatus_NoEnemy / sta EnsExtra.0.status,x / rts`
        Routine::RemoveEnemy => {
            mem.write_wram(0x6AF4 + x as u16, 0);
        }
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
pub(crate) fn enemy_resting(mem: &mut crate::frame::FrameMemory, x: u8) {
    let d = mem.read_ram(0x0400 + x as u16 + 5);
    if d & 0x40 == 0 {
        // 30fps 标 clear⇒全件（bit6 set 时跳过=30fps 节拍）
        mem.write_wram(0x6AF4 + x as u16 + 13, 0); // jumpDsplcmnt←0
        mem.write_ram(0x0400 + x as u16 + 7, 0); // movementInstrIndex←0
        mem.write_ram(0x0400 + x as u16 + 10, 0); // data0A←0
        enemy_force_speed(mem, x); // F6B9（件序首——速向萨姆斯力）
        enemy_distance_threshold(mem, x); // F75B（件序 2——阈距判）
        init_en_resting_anim_index(mem, x); // F682（件序 3）
        update_en_data1f(mem, x); // F676（件序 4）
        let delay = mem.read_ram(0x0400 + x as u16 + 9);
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
pub(crate) fn enemy_active(mem: &mut crate::frame::FrameMemory, x: u8) {
    let d = mem.read_ram(0x0400 + x as u16 + 5);
    if d & 0x40 == 0 {
        // bit6 clear ⇒ 走 BranchA/bit5 径（bit6 set⇒30fps 直跳 BranchB）
        if d & 0x20 != 0 {
            // bit5 set⇒delay←EnemyInitDelayTbl[type]；status Active→Resting
            let t = mem.read_wram(0x6AF4 + x as u16 + 14); // EnsExtra.type
            let delay = game_data::assets::enemy_init_delay(crate::frame::cur_area(mem)).get(t as usize).copied().unwrap_or(0);
            mem.write_ram(0x0400 + x as u16 + 9, delay); // Ens.delay(+9)
            let st = mem.read_wram(0x6AF4 + x as u16);
            mem.write_wram(0x6AF4 + x as u16, st.wrapping_sub(1));
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
pub(crate) fn enemy_remove_if_in_lava(mem: &mut crate::frame::FrameMemory, x: u8) {
    let scroll_dir = mem.read_ram(crate::ram_map::SCROLL_DIR);
    if scroll_dir < 2 {
        return; // 竖滚房⇒exit（asm cmp #$02/bcc）
    }
    let y = mem.read_ram(0x0400 + x as u16); // Ens.y
    if y < 0xEC {
        return; // 熔岩上方⇒exit
    }
    // 熔岩中⇒RemoveEnemy（清槽）
    mem.write_wram(0x6AF4 + x as u16, 0); // EnsExtra.status←0（槽空）
}

/// `UpdateEnemyCommon ; 07:F410`（入口 1：anim + move）。
pub(crate) fn update_enemy_common_anim_move(mem: &mut crate::frame::FrameMemory, x: u8, anim: u8) {
    update_enemy_anim(mem, x, anim);
    enemy_move(mem, x);
    update_enemy_common_no_move_no_anim(mem, x);
}

/// `UpdateEnemyCommon_Decide`（prg1_brinstar.asm:863）：按 `EnemyStatusPreAI` 选入口；
/// 活动态 anim 索引取 `$00`、休止态取 `$01`（均由调用者预设）。
pub(crate) fn update_enemy_common_decide(mem: &mut crate::frame::FrameMemory, x: u8) {
    match mem.read_ram(0x0081) {
        1 => {
            let anim = mem.read_ram(0x0001);
            update_enemy_common_no_move_anim(mem, x, anim);
        }
        3 => update_enemy_common_no_move_no_anim(mem, x),
        _ => {
            let anim = mem.read_ram(0x0000);
            update_enemy_common_anim_move(mem, x, anim);
        }
    }
}

/// `@noMove ; 07:F438`（入口 3：anim + 尾件，无移动）。
pub(crate) fn update_enemy_common_no_move_anim(mem: &mut crate::frame::FrameMemory, x: u8, anim: u8) {
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
pub(crate) fn update_enemy_common_no_move_no_anim(mem: &mut crate::frame::FrameMemory, x: u8) {
    let special = mem.read_ram(0x0400 + x as u16 + 15); // specialAttribs=+15
    if special & 0x80 != 0 {
        let oc = mem.read_ram(0x006B);
        if oc & 0x80 == 0 {
            mem.write_ram(0x006B, 0x80 | 0x03 | 0x20);
        }
    }
    if mem.read_wram(0x6AF4 + x as u16) != 0 {
        // 【goal-轮39 绘径可见旗写件】（asm :6178-6186 逐字：`and #$FD/ora (flag<<1)/sta
        // data05`——**bit1=可见旗**）——绘径写面（flag=可见判定 ✓）。
        let d5 = mem.read_ram(0x0400 + x as u16 + 5);
        mem.write_ram(0x0400 + x as u16 + 5, (d5 & 0xFD) | 0x02); // 绘⇒可见旗置
        draw_enemy(mem, x);
    }
    mem.write_ram(0x0400 + x as u16 + 4, 0); // isHit=+4
    mem.write_ram(0x0400 + x as u16 + 14, 0); // weaponAction=+14
}
