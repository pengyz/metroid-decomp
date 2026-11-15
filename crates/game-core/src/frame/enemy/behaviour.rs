//! 敌人行为层：七态处理器、AI 派发、逐像素移动/碰撞、绘制与动画。
//!
//! 由 `frame/mainloop.rs` 原样搬出（纯搬移）。asm 根：`UpdateEnemy_*` 07:F3BE..F57D、
//! `enemies/{skree,crawler}.asm`、`EnemyMoveOnePixel*` $83F5..、`DrawEnemy` 07:F42E 邻域。

use crate::frame::enemy::update::*;
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
        let mov_idx: u8 = if t == 5 {
            0 // Zoomer manual（choice=0）
        } else if t == 4 {
            7 // Skree（choice=7）
        } else {
            pick // 其他型：RNG 选播（表随件）
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
    const ENEMY_DEATH_ANIM: [u8; 32] = [
        // EnemyDeathAnimIndex（**ROM 字节锚** bank1:$960B（type×2+向（32B）））
        0x27,0x27,0x29,0x29,0x2D,0x2B,0x31,0x2F,0x33,0x33,0x41,0x41,0x4B,0x4B,0x55,0x53,
        0x72,0x74,0x00,0x00,0x00,0x00,0x69,0x69,0x69,0x69,0x00,0x00,0x00,0x00,0x00,0x00,
    ];
    let anim = ENEMY_DEATH_ANIM[idx];
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

/// ChooseEnemyAIRoutine（**asm 内容锚** 区 bank `$95E5`（prg1:156——type 0-0F dispatch）：
/// **05=Zoomer(Crawler)**/**04=Skree（goal-轮2 新端口）**。未端口型=【占位臂记账】
/// （DIVERGENCES 非 panic——title_bridge 同款模式）：静态显示径保可玩，AI 深件随包收敛。
pub(crate) fn choose_enemy_ai(mem: &mut crate::frame::FrameMemory, x: u8) {
    let t = mem.read_wram(0x6AF4 + x as u16 + 14); // type=+14
    match t {
        5 => crawler_ai(mem, x),
        // 【goal-轮2 Rust 实现】SkreeAIRoutine（enemies/skree.asm:2-71 全谱）——
        // 第一房间双敌型之二（sc-shoot-kill slot 0x20 type=4 实测；HP=2（01:962B）符）。
        4 => skree_ai(mem, x),
        _ => {
            crate::frame::DIVERGENCES.with(|d| d.borrow_mut().push(0x95E5));
            update_enemy_common_no_move_no_anim(mem, x);
        }
    }
}

/// SkreeAIRoutine（**asm 内容锚** `SkreeAIRoutine_{AREA}`（refs/m1disasm/SRC/enemies/
/// skree.asm:2-71——A 级逐行））：悬顶静息（PreAI=Resting⇒anim 8）→近距激活（阈距
/// data05 bit3——F75B 件）→坠落（movementInstrIndex($+7)<$0F——EnemyGetDeltaY delta 径）
/// →落地（≥$0F）⇒爆延时←$3A→≥$11⇒爆体径（jumpDsplcmnt($+13) 倒数⇒清槽+**4 弹**）。
pub(crate) fn skree_ai(mem: &mut crate::frame::FrameMemory, x: u8) {
    let pre = mem.read_ram(0x0081); // EnemyStatusPreAI
    if pre == 1 {
        // SkreeExit_Resting（:62-66）：anim 8+noMove
        update_enemy_anim(mem, x, 8);
        update_enemy_common_no_move_no_anim(mem, x);
        return;
    }
    if pre == 3 {
        // SkreeExit_Explode（:68-70）：noMoveNoAnim
        update_enemy_common_no_move_no_anim(mem, x);
        return;
    }
    let instr = mem.read_ram(0x0400 + x as u16 + 6); // movementInstrIndex=+6（ROM BC 06 04）
    if instr >= 0x11 {
        // SkreeBlowUpIntoProjectiles（:27-54）：jumpDsplcmnt 倒数⇒0⇒清槽+4 弹
        let d = mem.read_wram(0x6AF4 + x as u16 + 13); // EnsExtra.jumpDsplcmnt=+13
        let nd = d.wrapping_sub(1);
        mem.write_wram(0x6AF4 + x as u16 + 13, nd);
        if nd == 0 {
            mem.write_wram(0x6AF4 + x as u16, 0); // status←NoEnemy（:33-35）
            // 4 弹生成（skree.asm:36-54 逐字——SkreeProjectiles=$A0-$AF（4 槽×4B））：
            // 每槽 dieDelay←$0A+y/x/hi←Skree 位（Y=12/8/4/0 递减环）
            let ey = mem.read_ram(0x0400 + x as u16 + 0);
            let ex = mem.read_ram(0x0400 + x as u16 + 1);
            let ehi = mem.read_wram(0x6AF4 + x as u16 + 7);
            for yy in [12u16, 8, 4, 0] {
                let base = 0x00A0u16 + yy;
                mem.write_ram(base, 0x0A); // dieDelay←10 帧
                mem.write_ram(base + 1, ey);
                mem.write_ram(base + 2, ex);
                mem.write_ram(base + 3, ehi);
            }
        }
        update_enemy_anim(mem, x, 2);
        update_enemy_common_no_move_no_anim(mem, x);
        return;
    }
    if instr >= 0x0F {
        // 刚落地（:21-25）：爆延时←$3A（约 1 秒）
        mem.write_wram(0x6AF4 + x as u16 + 13, 0x3A);
    }
    // 坠落/落地径（SkreeExit_Active（:56-60）：anim 2+移动）——
    // 【对抗复核收敛⑤（2026-11-01）】EnemyMove 双门（areas_common:115-123 逐字）：
    // ①data05 bit6=30fps 门（`lda data05/asl/bmi`——置⇒**跳过移动**）②status==Active(2)
    // 门（`lda status/cmp #2/bne RTS`）——两门不过⇒只 anim+draw（无位移）。
    let d05 = mem.read_ram(0x0400 + x as u16 + 5);
    let st_now = mem.read_wram(0x6AF4 + x as u16);
    let gates_ok = (d05 & 0x40) == 0 && st_now == 2;
    if gates_ok {
    let type_ = mem.read_wram(0x6AF4 + x as u16 + 14) & 0x3F;
    let d05 = mem.read_ram(0x0400 + x as u16 + 5);
    let mov_idx = mem.read_ram(0x0400 + x as u16 + EN_MOV_IDX); // ROM `BD 08 04`
    let instr2 = mem.read_ram(0x0400 + x as u16 + EN_INSTR); // ROM `BC 06 04`
    let delay = mem.read_ram(0x0400 + x as u16 + EN_DELAY); // ROM `FE 09 04`
    let sub_y = mem.read_ram(0x0400 + x as u16 + EN_SUBPIXEL_Y); // union 同字节
    let speed_y = mem.read_ram(0x0400 + x as u16 + 2);
    let accel_y = mem.read_wram(0x6AF4 + x as u16 + 10);
    let d1f = mem.read_wram(0x6AF4 + x as u16 + 15);
    let can_up = enemy_check_move_up(mem, x);
    let can_down = enemy_check_move_down(mem, x);
    let (dy, dx, ndelay, ninstr, nd1f, trig) = crate::enemy::common::enemy_get_delta_xy(
        crate::frame::cur_area(mem), type_, d05, mov_idx, instr2, delay, sub_y, speed_y, accel_y, d1f, can_up, can_down);
    mem.write_wram(0x6AF4 + x as u16 + 15, nd1f);
    if trig {
        // `$82FB @fail`：`L968B[type]` bit5 置 ⇒ **翻面朝 + 清 bits6-7**
        // （asm `lda data05 / eor #$05 / ora L968B,y / and #$1F / sta data05`）。
        // 对 Skree（`L968B[4]=$86`，bit5=0）不可观测；对 `$18` 一系（`L968B[0x18]=$20`）必需。
        let l968b = game_data::assets::enemy_force_attribs(crate::frame::cur_area(mem))
            .get(type_ as usize)
            .copied()
            .unwrap_or(0);
        if l968b & 0x20 != 0 {
            let d5f = mem.read_ram(0x0400 + x as u16 + 5);
            mem.write_ram(0x0400 + x as u16 + 5, ((d5f ^ 0x05) | l968b) & 0x1F);
        }
        // `EnemyTriggerResting_AndClearEnAccelY`（`$81B1`）：bits5 置 + accelY（EnsExtra+10）清 0。
        let d5t = mem.read_ram(0x0400 + x as u16 + 5);
        mem.write_ram(0x0400 + x as u16 + 5, d5t | 0x20);
        mem.write_wram(0x6AF4 + x as u16 + 10, 0);
    }
    mem.write_ram(0x0400 + x as u16 + EN_DELAY, ndelay);
    mem.write_ram(0x0400 + x as u16 + EN_INSTR, ninstr);
    if dy != 0 {
        // Δ 施加（EnemyMove 径（:10029-10046 同构）：y±|Δ|——名表换补偿）
        let mut y = mem.read_ram(0x0400 + x as u16 + 0);
        let hi = mem.read_wram(0x6AF4 + x as u16 + 7);
        for _ in 0..dy.unsigned_abs() {
            if dy < 0 {
                if y == 0 {
                    y = 0xEF;
                    mem.write_wram(0x6AF4 + x as u16 + 7, hi ^ 1);
                } else {
                    y -= 1;
                }
            } else if y == 0xEF {
                y = 0;
                mem.write_wram(0x6AF4 + x as u16 + 7, hi ^ 1);
            } else {
                y += 1;
            }
        }
        mem.write_ram(0x0400 + x as u16 + 0, y);
    }
    // ⑦ deltaX 整轴（skree 串 packed nibble 低半字节——斜线下坠（复核⑦））
    if dx != 0 {
        let mut xx = mem.read_ram(0x0400 + x as u16 + 1);
        let hi = mem.read_wram(0x6AF4 + x as u16 + 7);
        for _ in 0..dx.unsigned_abs() {
            if dx < 0 {
                if xx == 0 {
                    xx = 0xFF;
                    mem.write_wram(0x6AF4 + x as u16 + 7, hi ^ 1);
                } else {
                    xx -= 1;
                }
            } else if xx == 0xFF {
                xx = 0;
                mem.write_wram(0x6AF4 + x as u16 + 7, hi ^ 1);
            } else {
                xx += 1;
            }
        }
        mem.write_ram(0x0400 + x as u16 + 1, xx);
    }
    } // 门径尾（gates_ok）
    update_enemy_anim(mem, x, 2);
    update_enemy_common_no_move_no_anim(mem, x);
}

/// ShouldCrawlerMove（**asm 内容锚** `07:F852`——`txa/lsr×3（carry←bit2）/adc FrameCount/
/// lsr`）：gate=((X>>3)+bit2(X)+FC)>>1&3（移动门 6/8 帧形）。
/// 【H3 根修（h3-enemy-slot-residual.md §2）】asm 合形直读 FC（无 −1）——goal-轮 43 的
/// FC−1 补偿=把 dump FC（逻辑读+1）代入公式的错位代偿（金方跳格 dump 17/18/25/26=
/// 逻辑读 16/17/24/25 ⇒ raw 公式四点全中）；r14 FC 对齐后失配（与 bit6 +1 同源）。
pub(crate) fn crawler_move_gate(x: u8, fc: u8) -> u8 {
    ((x >> 3).wrapping_add((x >> 2) & 1).wrapping_add(fc) >> 1) & 0x03
}

/// CrawlerAIRoutine（**asm 内容锚** `enemies/crawler.asm`——r26+2026-10-31 PreAI 门补装）：
/// ShouldCrawlerMove（F852——((x>>3+FC+lsr 进位))>>1&3=6/8 帧步门）；不动帧⇒@exit
/// （anim3+显示）；动帧⇒**EnemyStatusPreAI 三连门**（Resting⇒SkreeExit_Resting=anim8+
/// 显示（bank1 共享 skree 出口）/PreAI Explode⇒SkreeExit_Explode（仅显示）/
/// 现役 Explode⇒@exit）→@move。缺门症（已修）：Resting 帧即跑 AI——激活帧半径未写
/// （draw 同 tick 后行）⇒rx=0 全检快径⇒外角下探伪成功转角⇒菱形回路（crawlprobe f797）。
pub(crate) fn crawler_ai(mem: &mut crate::frame::FrameMemory, x: u8) {
    // 【goal-轮30 全径打印】入口+全出口（n=1664 漏步帧出口面直证）。
    // F852：txa/lsr×3（carry←原 bit2）/adc FrameCount（含进位）/lsr——门含 lsr 移出位。
    // 【goal-轮2 采样缝更正】toggle 公式正确——"bit6 反相"=采样缝伪象（我 spin-exit=帧首
    // /emu=帧末，同帧差一次更新）⇒真根=走位首动帧差（f798 早 1px）。
    // 【H3 测试缝（findings/h3-enemy-slot-residual.md）】gate 公式抽函数
    // crawler_move_gate（行为随形——H3 TDD 红靶：FC−1 相位补偿）。
    let gate = crawler_move_gate(x, mem.read_ram(crate::ram_map::FRAME_COUNT));
    // 【goal-轮15 gate 值直证】F852 门值逐帧（金方 2 帧跳 vs 我方移动模式）。
    if gate == 0 {
        update_enemy_anim(mem, x, 3); // @exit `lda #3/jsr UpdateEnemyAnim`
        update_enemy_common_no_move_no_anim(mem, x);
        return;
    }
    // EnemyStatusPreAI 三连门（crawler.asm:8-22 逐条——激活帧拦 Resting=本修核心）。
    let pre_ai = mem.read_ram(0x0081);
    if pre_ai == 1 {
        // SkreeExit_Resting（bank1/4 共享——skree.asm:62 `lda #8/noMove`）
        update_enemy_anim(mem, x, 8);
        update_enemy_common_no_move_no_anim(mem, x);
        return;
    }
    if pre_ai == 3 {
        // SkreeExit_Explode（skree.asm:68——`jmp noMoveNoAnim` 无 anim）
        update_enemy_common_no_move_no_anim(mem, x);
        return;
    }
    if mem.read_wram(0x6AF4 + x as u16) == 3 {
        // 现役 Explode⇒@exit（crawler.asm:20-22——anim3 落穿 noMoveNoAnim）
        update_enemy_anim(mem, x, 3);
        update_enemy_common_no_move_no_anim(mem, x);
        return;
    }
    // @move 径：JumpByRTSToMovementRoutine（areas_common \$8048 表 RTS 跳）
    // Y=(orientation*2+direction)*2→CrawlerMovementRoutinesTable→四例行。
    let d5 = mem.read_ram(0x0400 + x as u16 + 5);
    let orientation = mem.read_ram(0x0400 + x as u16 + 10) & 0x03; // data0A=+10
    // 【goal-轮7 转向路径直证】orientation 变化+原因。
    let direction = d5 & 1;
    // 【岩浆折返径】（crawler.asm:26-64——**asm BUG 注保真**（D4：竖房误触发不修））：
    // orientation==1（下墙径）∧ y==$E4（BANK1/4 阈——BANK2/5=$EB）⇒FlipDirection+
    // orientation←3（上墙）+跳过本帧移动（@afterLavaTurnAround 直入外角检）。
    let ey = mem.read_ram(0x0400 + x as u16 + 0);
    if orientation == 1 && ey == 0xE4 {
        let d = mem.read_ram(0x0400 + x as u16 + 5);
        mem.write_ram(0x0400 + x as u16 + 5, d ^ 0x01); // CrawlerFlipDirection
        mem.write_ram(0x0400 + x as u16 + 10, 0x03); // orientation←3（上墙）
        // @afterLavaTurnAround（跳过 @move 移动+内角检）
        crawler_outside_corner_check(mem, x);
        update_enemy_anim(mem, x, 3);
        update_enemy_common_no_move_no_anim(mem, x);
        return;
    }
    let idx = ((orientation as usize) * 2 + direction as usize) & 0x07;
    let ok = match idx {
        0 | 5 => enemy_move_one_pixel_right(mem, x),
        1 | 4 => enemy_move_one_pixel_left(mem, x),
        2 | 3 => enemy_move_one_pixel_down(mem, x),
        _ => enemy_move_one_pixel_up(mem, x),
    };
    if !ok {
        // CrawlerInsideCornerCheck（bank1:$9AA8——**goal-r16 G3'**）：$00 契约判据。
        // $00=1(离屏/wrap)⇒FlipDirection(data05 eor #1)；$00=0(块界碰撞)⇒data0A=(data0A−1)&3。
        // asm：move($9AE2)后 **InsideCorner($9AA8 失败径)+OutsideCorner($9AC6 探径)都跑**（非互斥）。
        let zero = mem.read_ram(0x00);
        if zero != 0 {
            let d = mem.read_ram(0x0400 + x as u16 + 5);
            mem.write_ram(0x0400 + x as u16 + 5, d ^ 0x01); // FlipDirection（离屏径）
        } else {
            let o = mem.read_ram(0x0400 + x as u16 + 10); // data0A
            mem.write_ram(0x0400 + x as u16 + 10, o.wrapping_sub(1) & 0x03); // 朝向--
            crawler_reorient_sprite(mem, x); // $9A8F（ InsideCorner 碰撞径 `4C 8F 9A`）
        }
    }
    // CrawlerOutsideCornerCheck（bank1:$9AC6——**goal-r16 G3'**）：恒跑
    crawler_outside_corner_check(mem, x);
    update_enemy_anim(mem, x, 3); // @exit `A9 03/20 0C 80`（cadence=3）
    update_enemy_common_no_move_no_anim(mem, x); // @exit 径 `4C 06 80`（anim+显示）
}

/// CrawlerOutsideCornerCheck（bank1:$9AC6——`jsr GetNextOrientation/jsr MoveRoutine`
/// `bcc @RTS`（探移失败⇒RTS）/成功⇒data0A←next+ReorientSprite）——恒跑（@move 与
/// @afterLavaTurnAround 共径）。
pub(crate) fn crawler_outside_corner_check(mem: &mut crate::frame::FrameMemory, x: u8) {
    let o = mem.read_ram(0x0400 + x as u16 + 10) & 0x03;
    let next_o = (o + 1) & 0x03;
    let dir = mem.read_ram(0x0400 + x as u16 + 5) & 1;
    let nidx = ((next_o as usize) * 2 + dir as usize) & 0x07;
    let probe_ok = match nidx {
        0 | 5 => enemy_move_one_pixel_right(mem, x),
        1 | 4 => enemy_move_one_pixel_left(mem, x),
        2 | 3 => enemy_move_one_pixel_down(mem, x),
        _ => enemy_move_one_pixel_up(mem, x),
    };
    // 【goal-轮91 探径碰撞直证】外角探径成功/失败（转向差源）。
    if probe_ok {
        mem.write_ram(0x0400 + x as u16 + 10, next_o); // data0A+1（asm `9D 0A 04`）
        crawler_reorient_sprite(mem, x); // $9A8F（OutsideCorner 径 `20 8F 9A`）
    }
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

/// CrawlerReorientSprite（**ROM 字节锚** bank1:$9A8F——m1disasm 未标注区（D5 四方交叉：
/// ROM 直读 + 三层闭环：$9AA0→EnAnimTable→EnFramePtrTable1→emu OAM 实测一致））：
/// `lda data05/lsr`（flip=bit0 进 carry）`lda data0A/and #3/rol A`（idx=orient*2+flip）
/// `lda $9AA0,y / jmp InitEnAnimIndex`（F68D：reset/index←A、animDelay←0）。
/// 旧形「随件占位」⇒ 敌 animFrame 恒 0 ⇒ 画错帧（emu tile CA/CB/DA/DB vs rust 14）。
pub(crate) fn crawler_reorient_sprite(mem: &mut crate::frame::FrameMemory, x: u8) {
    const CRAWLER_SPRITE_ANIM_TBL: [u8; 8] =
        [0x35, 0x35, 0x3E, 0x38, 0x3B, 0x3B, 0x38, 0x3E]; // ROM $9AA0（orient*2+flip）
    let d05 = mem.read_ram(0x0400 + x as u16 + 5);
    let flip = d05 & 0x01; // lsr→carry=bit0
    let orient = (mem.read_ram(0x0400 + x as u16 + 10) & 0x03) as usize;
    let idx = ((orient << 1) | flip as usize) & 0x07;
    let v = CRAWLER_SPRITE_ANIM_TBL[idx];
    // 【goal-轮86 reorient idx 直证】reset 源（帧面差源）。
    // InitEnAnimIndex(F68D)
    mem.write_wram(0x6AF4 + x as u16 + 5, v); // resetAnimIndex
    mem.write_wram(0x6AF4 + x as u16 + 6, v); // animIndex
    mem.write_wram(0x6AF4 + x as u16 + 4, 0); // animDelay←0
    // 【goal-轮42 bit2/1 清源件】金腿实证（f1838 转角径 d05 0x1A→0x58——bit2/1(0x06)
    // 清+bit6 置）——转角径写面：d05 bit2/1 清（朝向/轴位重组——constants 无注余项位）。
    let d5c = mem.read_ram(0x0400 + x as u16 + 5);
    mem.write_ram(0x0400 + x as u16 + 5, d5c & 0xF9); // bit2/1 清（0x06）
}

/// UpdateEnemyAnim（**asm 内容锚** `UpdateEnemyAnim: ; 07:E094`——敌人动画走表全径）：
/// status==Pickup(5)⇒RTS；delay 门（≠0 递减未归零⇒RTS）→`sta animDelay←A`→走
/// EN_ANIM_TABLE[animIndex]（区表——Brinstar $9D6A）：FF⇒animIndex←reset 续走；
/// 否则 animFrame(+3)←表值、animIndex+1。调用形：CrawlerAI @exit `lda #3/jsr $800C`。
pub(crate) fn update_enemy_anim(mem: &mut crate::frame::FrameMemory, x: u8, a: u8) {
    let base = 0x6AF4u16 + x as u16;
    if mem.read_wram(base) == 5 {
        return; // Pickup 态不更
    }
    let delay = mem.read_wram(base + 4);
    if delay != 0 {
        let d = delay - 1;
        mem.write_wram(base + 4, d);
        if d != 0 {
            return;
        }
    }
    mem.write_wram(base + 4, a); // sta animDelay←A
    let reset = mem.read_wram(base + 5);
    let mut y = mem.read_wram(base + 6) as usize;
    for _ in 0..16u32 {
        let v = *game_data::assets::enemy_anim_table().get(y).unwrap_or(&0xFF);
        if v == 0xFF {
            y = reset as usize; // 回绕续走
            continue;
        }
        mem.write_wram(base + 3, v); // animFrame←表值
        mem.write_wram(base + 6, y.wrapping_add(1) as u8);
        // 【goal-轮85 animIndex 初值直证】ENEMY_*_ANIM 表径（帧面差源）。
        return;
    }
    crate::unported!("bank07:E0BC-wrap"); // 表不变量破裂——fail-loud（update_obj_anim 同式）
}

/// DrawEnemy（**asm 内容锚** `DrawEnemy: ; 07:DD8B`——r26 端口）：animFrame==FrameNone⇒
/// ClearObjectCntrl；帧指针（EnFramePtrTable1/2——animFrame bit7 carry 选表！）→
/// GetSpriteCntrlData→位指针（EnPlacePtrTable）→sprite 管线写（place_disp/四元组）。
pub(crate) fn draw_enemy(mem: &mut crate::frame::FrameMemory, x: u8) {
    // 【goal-轮73 draw 调用直证】双写源（n=803 清→置双写）。
    use game_data::assets;
    // 【②回装（2026-10-29）】asm :6156-6168：帧记字 1/2→EnsExtra.radiusY/X（IsObject-
    // Visible 用半径（序=半径写先行））+[:6178-6186] bit1 逐帧闩（and #$FD/ora（txa<<1））。
    // 爆亡链件已补（F40D/DCF5/F483 全径（415 测））⇒连锁闭环（§10 回装判据达成）。
    let frame0 = mem.read_wram(0x6AF4 + x as u16 + 3);
    if frame0 != 0xFF {
        let fi0 = ((frame0 & 0x7F) as usize) + if frame0 & 0x80 != 0 { 128 } else { 0 };
        let (fo0, fl0) = assets::enemy_frame_off(fi0 as usize).unwrap_or((0, 0));
    // 【goal-轮94 fo0→radiusY 直证】帧面 radiusY（金方 1/9 vs 我 8）。
        if fl0 > 2 {
            mem.write_wram(0x6AF4 + x as u16 + 1, assets::enemy_frames()[fo0 as usize + 1]);
            mem.write_wram(0x6AF4 + x as u16 + 2, assets::enemy_frames()[fo0 as usize + 2]);
        }
    }
    // 【goal-轮75 asm 合形修】draw 走 fresh 检（:6177 `jsr IsObjectVisible` 逐字——
    // 旧形走闩径（免 fresh）=偏离）——闩径 F37F 在 update 面（非 draw）。
    let visible = enemy_check_if_visible_inner(mem, x);
    let e5 = 0x0400 + x as u16 + 5;
    let d5 = mem.read_ram(e5);
    mem.write_ram(e5, (d5 & 0xFD) | ((visible as u8) << 1)); // bit1 闩
    if !visible {
        mem.write_ram(0x006B, 0);
        return;
    }
    let frame = mem.read_wram(0x6AF4 + x as u16 + 3); // animFrame=+3
    if frame == 0xFF {
        mem.write_ram(0x006B, 0);
        return;
    }
    let table2 = frame & 0x80 != 0;
    let fi = ((frame & 0x7F) as usize) + if table2 { 128 } else { 0 };
    let (fo, fl) = assets::enemy_frame_off(fi as usize).unwrap_or((0, 0));
    let blob = |i: usize| -> u8 {
        if fl == 0 || i >= fl as usize { 0 } else { assets::enemy_frames()[fo as usize + i] }
    };
    let ctrl = blob(0);
    // 【X 形根修（2026-10-31 asm WriteSpriteRAM（DEE6）逐字）】$6B=**对象控制字节**
    // （SetMirrorCntrlBit/敌 tough 面写）**非帧 ctrl**——旧形 `sta $6B=帧ctrl` 覆写
    // +镜像映射交叉（bit6→0x80）⇒四精灵翻位成 X 形（金=星芒——贴图错根）。
    // 正解：attr=Temp05 ^（ObjectCntrl bit4<<2 & HFLIP($40)）（asm :6324-6328）；
    // 镜像面 Temp04=ctrl ^ 同源（GetSpriteCntrlData :5913-5920）。
    let obj_mirror = mem.read_ram(0x006B) & 0x10; // bit4=水平镜像控制
    let mirror_flags = if obj_mirror != 0 { 0x40u8 } else { 0 };
    let place_idx = ((ctrl & 0x0F) as usize).min(127);
    let (po, pl) = assets::enemy_place_off(place_idx as usize).unwrap_or((0, 0));
    let pblob = |i: usize| -> u8 {
        if pl == 0 || i >= pl as usize { 0 } else { assets::enemy_places()[po as usize + i] }
    };
    let y = mem.read_ram(0x0400 + x as u16 + 0); // y=+0
    let xpos = mem.read_ram(0x0400 + x as u16 + 1); // x=+1
    let scroll_y = mem.read_ram(crate::ram_map::SCROLL_Y);
    let horz = mem.read_ram(crate::ram_map::SCROLL_DIR) & 0x02 != 0;
    let sy = (if horz { y } else { y.wrapping_sub(scroll_y) }) as i16;
    // 【勘误修（2026-10-29（问题 A 根因））】敌屏位 x=房位−**ScrollX($FD)**——旧形减
    // $26（SpareMem26（constants:488（$25=MMC1CTRL_ZP 同族误读（sprite.rs 勘误注
    // 早已证）））——走滚径写 $FD（scroll_right）而 0x26 停旧值⇒敌屏位不减镜头⇒
    // **随镜恒距**（用户截图症（爬虫悬空错位））。
    let sx = xpos.wrapping_sub(mem.read_ram(crate::ram_map::SCROLL_X)) as i16;
    let mut cntrl05 = ((ctrl >> 4) & 0x03) | (ctrl & 0xC0) | 0x20;
    let mut place_i = 0usize;
    let mut frame_i = 3usize;
    let mut x_sprite = mem.read_ram(0x005B) as u16;
    loop {
        let b = blob(frame_i);
        if b < 0xFC {
            let dy = crate::frame::sprite::place_disp(pblob(place_i), ctrl ^ mirror_flags, true);
            let dx = crate::frame::sprite::place_disp(pblob(place_i + 1), ctrl ^ mirror_flags, false);
            // WriteSpriteRAM（:6324-6328）：attr=Temp05 ^ (ObjectCntrl bit4→HFLIP)
            let attr = cntrl05 ^ mirror_flags;
            let oy = (sy + dy as i16 - 1) as u8;
            let ox = (sx + dx as i16) as u8;
            mem.write_ram(0x0200 + x_sprite, oy);
            mem.write_ram(0x0201 + x_sprite, b);
            mem.write_ram(0x0202 + x_sprite, attr);
            mem.write_ram(0x0203 + x_sprite, ox);
            x_sprite = x_sprite.wrapping_add(4);
            place_i += 2;
            frame_i += 1;
        } else if b == 0xFC {
            frame_i += 3;
        } else if b == 0xFD {
            frame_i += 1;
            cntrl05 = blob(frame_i);
            frame_i += 1;
        } else if b == 0xFE {
            place_i += 2;
            frame_i += 1;
        } else {
            break;
        }
    }
    mem.write_ram(0x005B, x_sprite as u8);
}


/// EnemyMoveOnePixelUp（**asm 内容锚** `areas_common.asm:989 $83F5`——r26）：
/// 顶界块界判（(y-radiusY)&7==0⇒EnemyCheckMoveUp）→y dec（y==0 换名表族）→
/// radiusY 界判→成功（carry=1）/失败（carry=0）。返回 bool（carry）。
pub(crate) fn enemy_move_one_pixel_up(mem: &mut crate::frame::FrameMemory, x: u8) -> bool {
    let y = mem.read_ram(0x0400 + x as u16 + 0); // y=+0
    let ry = mem.read_wram(0x6AF4 + x as u16 + 1); // radiusY=+1
    if y.wrapping_sub(ry) & 0x07 == 0 {
        // 块界⇒碰撞判（EnemyCheckMoveUp——随件 fail-loud）
        if !enemy_check_move_up(mem, x) {
            mem.write_ram(0x00, 0); // $00=0（碰撞）
            return false;
        }
    }
    let ny = y.wrapping_sub(1);
    mem.write_ram(0x0400 + x as u16 + 0, ny);
    if y == 0 {
        // 【wrap 值勘误（asm :1011-1016 逐字：`ldy y/bne`⇒y==0⇒`ldy #SCRN_VY($F0)`）】
        // 旧形 0xEF 偏 1px——asm=**$F0**（屏高 240 名表换补偿）。
        mem.write_ram(0x0400 + x as u16 + 0, 0xF0);
        mem.write_ram(0x00, 1); // $00=1（离屏/wrap）
        return false;
    }
    // 成功：jumpDsplcmnt inc（横朝向时）
    if mem.read_ram(0x0400 + x as u16 + 5) & 0x80 == 0 {
        let j = mem.read_wram(0x6AF4 + x as u16 + 13); // jumpDsplcmnt=+13
        mem.write_wram(0x6AF4 + x as u16 + 13, j.wrapping_add(1));
    }
    true
}

/// EnemyMoveOnePixelDown（**asm 内容锚** `: 1066 $844B`——r26 同构（y inc））。
pub(crate) fn enemy_move_one_pixel_down(mem: &mut crate::frame::FrameMemory, x: u8) -> bool {
    // 【goal-轮104 move_down 调用计数直证】调用/施加径差。
    // 【goal-轮82 animIndex 推进率直证】帧面差源（敌 rx≤4 vs 8）。
    let y = mem.read_ram(0x0400 + x as u16 + 0);
    let ry = mem.read_wram(0x6AF4 + x as u16 + 1);
    if y.wrapping_add(ry) & 0x07 == 0 {
        if !enemy_check_move_down(mem, x) {
            mem.write_ram(0x00, 0); // $00=0（碰撞）
            // 【goal-轮101 ok 失败径直证】块撞。
            return false;
        }
    }
    // 【goal-轮101 ok 失败径直证】wrap。
    let ny = y.wrapping_add(1);
    // 【goal-轮162 animIndex 写源直证】EN_ANIM_TABLE[animIndex] 节拍（paradox 解径）
    // ——金方 animIndex 差 ⇒ animFrame 差 ⇒ fo0 差 ⇒ radiusY=1/9。
    mem.write_ram(0x0400 + x as u16 + 0, ny);
    // 【wrap 判式勘误（goal-轮9 asm :1087-1088 逐字：`ldy Ens.0.y/cpy #SCRN_VY-1($EF)`）
    // 】旧形 `ny==−radiusY` 偏离（wrap 触发点错位）——asm=**当前 y==$EF**（屏底名表换）。
    if y == 0xEF {
        // 【goal-轮110 asm 合形修】wrap 回滚（:1125 `dec Ens.0.y`——旧形缺回滚 ✗）。
        mem.write_ram(0x0400 + x as u16 + 0, y); // 回滚（asm dec）
        mem.write_ram(0x00, 1); // $00=1（离屏/wrap）
        return false;
    }
    // 【2026-10-31 jD 补装】asm $849D @success：`lda data05/bmi @endIf_D`（bit7 清）⇒
    // **dec jumpDsplcmnt**（垂直位移记账：UP=$8441 inc/DOWN=dec——下行爬虫每成功步 −1，
    // 含外角探针的下移步；emu 实录 00→FF(f814 首步)→FE→…；rust 旧形缺写⇒恒 00）。
    if mem.read_ram(0x0400 + x as u16 + 5) & 0x80 == 0 {
        let j = mem.read_wram(0x6AF4 + x as u16 + 13); // jumpDsplcmnt=+13
        mem.write_wram(0x6AF4 + x as u16 + 13, j.wrapping_sub(1));
    }
    true
}

/// EnemyMoveOnePixelLeft（**asm 内容锚** `: 1214 $84A7`——r26（x dec 同构））。
/// 【goal-轮10 asm :1142-1210 逐字】①8px 门检 ②x==0⇒名表换（横滚房 ScrollX 面）
/// ③dec x ④左界==0（x==radiusX）⇒**失败+回滚**（inc x——旧形缺回滚！）⑤@success：
/// jumpDsplcmnt inc（data05 bit7 置时）。
pub(crate) fn enemy_move_one_pixel_left(mem: &mut crate::frame::FrameMemory, x: u8) -> bool {
    let xpos = mem.read_ram(0x0400 + x as u16 + 1); // x=+1
    let rx = mem.read_wram(0x6AF4 + x as u16 + 2); // radiusX=+2
    if xpos.wrapping_sub(rx) & 0x07 == 0 {
        if !enemy_check_move_left(mem, x) {
            mem.write_ram(0x00, 0); // $00=0（碰撞）
            return false;
        }
    }
    mem.write_ram(0x00, 1); // $00=1（移动进行——inc $00（:1161））
    // x==0⇒名表换（:1162-1180——横滚房 ScrollX/GetOtherNameTableIndex 族简化=hi 翻）
    if xpos == 0 {
        let hi = mem.read_wram(0x6AF4 + x as u16 + 7);
        mem.write_wram(0x6AF4 + x as u16 + 7, hi ^ 1); // SwitchEnemyNameTable（简化形）
    }
    let nx = xpos.wrapping_sub(1);
    mem.write_ram(0x0400 + x as u16 + 1, nx);
    // 左界==0（dec 后 x==radiusX）⇒失败+**回滚**（inc x——:1184-1199 逐字）
    if nx == rx {
        mem.write_ram(0x0400 + x as u16 + 1, xpos); // inc x（回滚）
        mem.write_ram(0x00, 0);
        return false;
    }
    // @success（:1201-1208）：data05 bit7 置⇒jumpDsplcmnt inc
    if mem.read_ram(0x0400 + x as u16 + 5) & 0x80 != 0 {
        let j = mem.read_wram(0x6AF4 + x as u16 + 13);
        mem.write_wram(0x6AF4 + x as u16 + 13, j.wrapping_add(1));
    }
    true
}

/// EnemyMoveOnePixelRight（**asm 内容锚** `: 1214 $84FE`——r26（x inc 同构））。
/// 【goal-轮10】右界判（x+radiusX==$FF?——asm :1230-1245 同构回滚）+wrap（x==0 环绕）。
pub(crate) fn enemy_move_one_pixel_right(mem: &mut crate::frame::FrameMemory, x: u8) -> bool {
    let xpos = mem.read_ram(0x0400 + x as u16 + 1);
    let rx = mem.read_wram(0x6AF4 + x as u16 + 2);
    if xpos.wrapping_add(rx) & 0x07 == 0 {
        if !enemy_check_move_right(mem, x) {
            mem.write_ram(0x00, 0); // $00=0（碰撞）
            // 【goal-轮18 右移块碰直证】碰撞返回。
            return false;
        }
    }
    mem.write_ram(0x00, 1);
    let nx = xpos.wrapping_add(1);
    mem.write_ram(0x0400 + x as u16 + 1, nx);
    if nx == 0 {
        // wrap（x 环绕 00⇒名表换——ToggleSamusHi 同族）
        mem.write_ram(0x00, 1);
        let hi = mem.read_wram(0x6AF4 + x as u16 + 7);
        mem.write_wram(0x6AF4 + x as u16 + 7, hi ^ 1);
    }
    // 右界判（x+radiusX 界——asm :1230-1245 回滚同构）
    if nx.wrapping_add(rx) == 0 {
        mem.write_ram(0x0400 + x as u16 + 1, xpos); // 回滚
        mem.write_ram(0x00, 0);
        return false;
    }
    if mem.read_ram(0x0400 + x as u16 + 5) & 0x80 != 0 {
        let j = mem.read_wram(0x6AF4 + x as u16 + 13);
        mem.write_wram(0x6AF4 + x as u16 + 13, j.wrapping_add(1));
    }
    true
}


/// MakeRoomRAMPtr（**asm 内容锚** `: 07:E96A`——r26）：位置→RoomRAM 瓦址——
/// ptr=0x6000|名表页|((y&0xF8)<<2)|(x>>3)（y/8×32+x/8 形！）。
/// 【goal-轮15 页面径回正（asm E96A:8425-8434 逐字：`lda Temp0B_PositionHi/and #$04/
/// ora ptr+1`——**对象 hi 选页**）】轮5 的 RoomRAMPtr(\$38) 选页=偏离（对象住其房间页
/// ——hi==房页不变量（GetNameTableAtScrollDir==SelectRoomRAM 同式）；金腿实证：爬虫
/// hi=0×\$6000（首房固实瓦挡探 ✓）而 \$38=\$6400（现房空 FF）⇒我探径滑过=旋转根）。
pub(crate) fn make_room_ram_ptr(_mem: &mut crate::frame::FrameMemory, y: u8, x: u8, hi: u8) -> u16 {
    let idx = (((y & 0xF8) as u16) << 2) | ((x >> 3) as u16);
    let page = if hi & 1 != 0 { 0x0400u16 } else { 0x0000 };
    0x6000 | page | (idx & 0x3FF)
}

/// 碰撞核（**asm 内容锚** `CheckMoveVertical: ; 07:E7BD`/Horizontal $E89B——r26）：
/// 逐点（块界）→MakeRoomRAMPtr→tile 值类：**<$80=固实（碰撞！）**/$4E=固实空（门/像）/
/// $80-$9F=可炸/≥$A0=可走。返回 carry（可走=true）。
pub(crate) fn collision_point_ok(mem: &mut crate::frame::FrameMemory, y: u8, x: u8, hi: u8) -> bool {
    let ptr = make_room_ram_ptr(mem, y, x, hi);
    let tile = mem.read_wram(ptr);
    if tile == 0x4E {
        return false; // 固实空（门/像——碰撞！）
    }
    if tile < 0x80 {
        return false; // 固实 tile（< $80）——碰撞！
    }
    // $80-$9F=可炸（可走性=IsBlastTile 判——简化径：视为可走）
    // ≥$A0=可走（$A0/$A1=门 tile（Samus 专用判——随件））
    true
}

/// EnemyCheckMoveUp（**asm 内容锚** `: 07:E770`→EnemyCheckMoveVertical `07:E783`→
/// CheckMoveVertical **x 全距走环**——r9 转向族考证修（crawler-corner-divergence.md））：
/// staging=enemy::movement（distY=ry+8 忠实），探行=y−distY（进入块上块），
/// x−rx 起 n 列全距（GetNumBlocksToCheck 公式=vertical_span_cols 同构已验证）；
/// rx==0⇒免检（asm `bne Lx198/sec/rts`）。旧形单点 (y−ry+8, x) 双错：探点 +8 落出
/// 进入块+漏跨块列——爬虫 radius 跨块时转角决策分歧根因。
pub(crate) fn enemy_check_move_up(mem: &mut crate::frame::FrameMemory, x: u8) -> bool {
    let y = mem.read_ram(0x0400 + x as u16 + 0);
    let ry = mem.read_wram(0x6AF4 + x as u16 + 1);
    let xpos = mem.read_ram(0x0400 + x as u16 + 1);
    let rx = mem.read_wram(0x6AF4 + x as u16 + 2); // radiusX=+2
    let hi = mem.read_wram(0x6AF4 + x as u16 + 7); // hi=+7
    let st = crate::enemy::movement::enemy_check_move_up(rx, ry, xpos, y, hi);
    match crate::enemy::movement::check_move_boundary(st, false) {
        crate::enemy::movement::MoveCheckOutcome::NoCollision => true,
        crate::enemy::movement::MoveCheckOutcome::NeedsBgCollision(st) => {
            let row = st.pos_y.wrapping_sub(st.dist_to_center_y);
            let (left, n) = crate::samus::physics::vertical_span_cols(st.pos_x, st.radius_x);
            // 【goal-轮11 块判结果直证】逐列 collision_point_ok 结果。
            for i in 0..n {
                let col = left.wrapping_add((i as u8).wrapping_mul(8));
                if !collision_point_ok(mem, row, col, st.pos_hi) {
                    return false;
                }
            }
            true
        }
    }
}

/// EnemyCheckMoveDown（**asm 内容锚** `: 07:E77B`——r26）：底块判。
/// EnemyCheckMoveDown（**asm 内容锚** `: 07:E77B`→同上 x 全距走环——r9 转向族修）：
/// staging distY=0−ry（两位补码）⇒探行=y−(−ry)=y+ry（进入块下块）。
pub(crate) fn enemy_check_move_down(mem: &mut crate::frame::FrameMemory, x: u8) -> bool {
    let y = mem.read_ram(0x0400 + x as u16 + 0);
    let ry = mem.read_wram(0x6AF4 + x as u16 + 1);
    let xpos = mem.read_ram(0x0400 + x as u16 + 1);
    let rx = mem.read_wram(0x6AF4 + x as u16 + 2);
    let hi = mem.read_wram(0x6AF4 + x as u16 + 7);
    let st = crate::enemy::movement::enemy_check_move_down(rx, ry, xpos, y, hi);
    // 【goal-轮6 块判直证】敌B 穿地源：块判输入/输出。
    match crate::enemy::movement::check_move_boundary(st, false) {
        crate::enemy::movement::MoveCheckOutcome::NoCollision => true,
        crate::enemy::movement::MoveCheckOutcome::NeedsBgCollision(st) => {
            let row = st.pos_y.wrapping_sub(st.dist_to_center_y);
            let (left, n) = crate::samus::physics::vertical_span_cols(st.pos_x, st.radius_x);
            // 【goal-轮11 块判结果直证】逐列 collision_point_ok 结果。
            for i in 0..n {
                let col = left.wrapping_add((i as u8).wrapping_mul(8));
                if !collision_point_ok(mem, row, col, st.pos_hi) {
                    return false;
                }
            }
            true
        }
    }
}

/// EnemyCheckMoveLeft（**asm 内容锚** `: 07:E8F1`——r26）：左块判。
/// EnemyCheckMoveLeft（**asm 内容锚** `: 07:E8F1`→CheckMoveHorizontal **y 全距走环**
/// （07:E89B）——r9 转向族修）：staging distX=rx+8⇒探列=x−(rx+8)（进入块左块），
/// y−ry 起 n 行全距（horizontal_span_rows 同构）；ry==0⇒免检。
pub(crate) fn enemy_check_move_left(mem: &mut crate::frame::FrameMemory, x: u8) -> bool {
    let y = mem.read_ram(0x0400 + x as u16 + 0);
    let xpos = mem.read_ram(0x0400 + x as u16 + 1);
    let rx = mem.read_wram(0x6AF4 + x as u16 + 2); // radiusX=+2
    let ry = mem.read_wram(0x6AF4 + x as u16 + 1);
    let hi = mem.read_wram(0x6AF4 + x as u16 + 7);
    let st = crate::enemy::movement::enemy_check_move_left(rx, ry, xpos, y, hi);
    match crate::enemy::movement::check_move_boundary(st, true) {
        crate::enemy::movement::MoveCheckOutcome::NoCollision => true,
        crate::enemy::movement::MoveCheckOutcome::NeedsBgCollision(st) => {
            let col = st.pos_x.wrapping_sub(st.dist_to_center_x);
            let (top, n) = crate::samus::physics::horizontal_span_rows(st.pos_y, st.radius_y);
            for i in 0..n {
                let row = top.wrapping_add((i as u8).wrapping_mul(8));
                if !collision_point_ok(mem, row, col, st.pos_hi) {
                    return false;
                }
            }
            true
        }
    }
}

/// EnemyCheckMoveRight（**asm 内容锚** `: 07:E8FC`——r26）：右块判。
/// EnemyCheckMoveRight（**asm 内容锚** `: 07:E8FC`→CheckMoveHorizontal y 全距走环
/// ——r9 转向族修）：staging distX=0−rx⇒探列=x+rx（进入块右块），y−ry 起 n 行全距。
pub(crate) fn enemy_check_move_right(mem: &mut crate::frame::FrameMemory, x: u8) -> bool {
    let y = mem.read_ram(0x0400 + x as u16 + 0);
    let xpos = mem.read_ram(0x0400 + x as u16 + 1);
    let rx = mem.read_wram(0x6AF4 + x as u16 + 2);
    let ry = mem.read_wram(0x6AF4 + x as u16 + 1);
    let hi = mem.read_wram(0x6AF4 + x as u16 + 7);
    let st = crate::enemy::movement::enemy_check_move_right(rx, ry, xpos, y, hi);
    match crate::enemy::movement::check_move_boundary(st, true) {
        crate::enemy::movement::MoveCheckOutcome::NoCollision => true,
        crate::enemy::movement::MoveCheckOutcome::NeedsBgCollision(st) => {
            let col = st.pos_x.wrapping_sub(st.dist_to_center_x);
            let (top, n) = crate::samus::physics::horizontal_span_rows(st.pos_y, st.radius_y);
            for i in 0..n {
                let row = top.wrapping_add((i as u8).wrapping_mul(8));
                if !collision_point_ok(mem, row, col, st.pos_hi) {
                    return false;
                }
            }
            true
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

