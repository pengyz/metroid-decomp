//! Tourian 炮台（`UpdateAllCannons ; 03:9B37` 族）。
//!
//! 母脑房有 16 座炮台（`Cannons instanceof Cannon $10`，`$6BF4-$6C73`，槽 8 B：
//! status/y/x/hi/instrListID/angle/instrDelay/instrID）。每座按一条 11/6 字节的指令表循环：
//! `$00-$07`=设定朝向角度、`$FC/$FD/$FE`=发射一颗向下/左下/右下的敌弹、`$FF`=回到表头。
//!
//! 逃脱计时器启动后炮台不再开火，只在 `FrameCount & $02 == 0` 的帧闪 `CannonTimeBombSet`
//! 帧（`DrawCannon_Escape`）。母脑将死（`MotherBrainStatus >= 4`）时敌弹不再生成。

use crate::enemy::motion::apply_speed_to_position_parts;
use crate::enemy::state::EnemyStatus;
use crate::frame::Manager;

/// 炮台槽数（`_sizeof_Cannons / _sizeof_Cannons.0`）。
const COUNT: u8 = 16;

/// `CannonInstrDelayTable ; 03:9D8F`（五个 `$28`）。
const INSTR_DELAY: [u8; 5] = [0x28; 5];
/// `CannonInstrListsOffset ; 03:9D94`。
const INSTR_OFFSET: [u8; 5] = [0, 11, 22, 33, 39];
// `CannonInstrLists ; 03:9D99`（五张表首尾相接；`$FF`=回表头，$F0-$FE=开火）已迁到
// 资产包（`game_data::assets::cannon_instr_lists()`；45 B = 11+11+11+6+6）。
/// `CannonAnimFrameTable ; 03:9DC6`（角度 0..7 ⇒ `_id_EnFrame_CannonUp..UpRight`）。
/// Tourian `EnFramePtrTable1` 表序：`CannonUp=6`、`UpLeft=7`、`Left=8`、`DownLeft=9`、
/// `Down=10`、`DownRight=11`、`Right=12`、`UpRight=13`。
const ANIM_FRAME: [u8; 8] = [6, 7, 8, 9, 10, 11, 12, 13];
/// `_id_EnFrame_CannonTimeBombSet`（表序第 25 项）。
const FRAME_TIME_BOMB_SET: u8 = 25;
/// `CannonEnProjectileAnimTable-2 ; 03:9C28`：`$FE/$FD/$FC` ⇒ `DownRight/DownLeft/Down`。
/// Tourian `EnAnimTable` 表序：`CannonBulletDownLeft=4`、`DownRight=5`、`Down=6`。
const EN_PROJ_ANIM: [u8; 3] = [5, 4, 6];
/// `CannonEnProjectileXOffsetTable ; 03:9DCE`。
const EN_PROJ_DX: [i8; 3] = [0x09, -0x09, 0x00];
/// `CannonEnProjectileYOffsetTable ; 03:9DD1`。
const EN_PROJ_DY: [i8; 3] = [0x09, 0x09, 0x0B];
/// `EnMotherBrain - Ens`：本模型的"共用绘制槽"（炮台与计时器都借它画）。
const DRAW_SLOT: u8 = 0xE0;

/// 读一格炮台字段（`Cannons` 面收口在 `frame/memory.rs`）。
fn get(mem: &Manager, index: u8, off: u16) -> u8 {
    mem.cannon_byte(index, off)
}

/// 写一格炮台字段。
fn put(mem: &mut Manager, index: u8, off: u16, v: u8) {
    mem.set_cannon_byte(index, off, v);
}

/// `.struct Cannon` 字段偏移。
mod f {
    pub const STATUS: u16 = 0;
    pub const Y: u16 = 1;
    pub const X: u16 = 2;
    pub const HI: u16 = 3;
    pub const INSTR_LIST_ID: u16 = 4;
    pub const ANGLE: u16 = 5;
    pub const INSTR_DELAY: u16 = 6;
    pub const INSTR_ID: u16 = 7;
}

/// `UpdateAllCannons ; 03:9B37`：从槽 15 逐槽到槽 1（**asm 的 `bne` 使槽 0 不被更新**——
/// 原版 quirk，按字节保留）。
pub(crate) fn update_all_cannons(mem: &mut Manager) {
    let mut index = COUNT - 1;
    loop {
        if get(mem, index, f::STATUS) != 0 {
            update_cannon(mem, index);
        }
        if index == 0 {
            break;
        }
        index -= 1;
        if index == 0 {
            break; // `sec sbc #$08 / tax / bne` —— 槽 0 在递减后直接退出
        }
    }
}

/// `UpdateCannon ; 03:9B4C`。
fn update_cannon(mem: &mut Manager, index: u8) {
    if cannon_offscreen(mem, index) {
        return;
    }
    if mem.end_timer_hi().wrapping_add(1) != 0 {
        // 逃脱计时器活动：隔帧闪 `CannonTimeBombSet`
        if mem.frame_count() & 0x02 != 0 {
            return;
        }
        draw(mem, index, FRAME_TIME_BOMB_SET);
        return;
    }
    if get(mem, index, f::INSTR_LIST_ID) == 0x05 {
        return;
    }
    run_instructions(mem, index);
    let angle = get(mem, index, f::ANGLE);
    draw(mem, index, ANIM_FRAME[(angle & 0x07) as usize]);
}

/// `UpdateCannon_CheckIfOnScreen ; 03:9C4D`：返回 true 表示**不可见**。
/// 判据：先在 `Cannons.0.x` 与 `ScrollX`（竖滚时改 `Cannons.0.y` 与 `ScrollY`）之间建立进位，
/// 再与 `hi ^ PPUCTRL_ZP` 的 bit0 组合——同页且进位置、或异页且进位清 ⇒ 在屏。
fn cannon_offscreen(mem: &mut Manager, index: u8) -> bool {
    let x = get(mem, index, f::X);
    let (scroll_x, scroll_y, dir) = {
        let c = mem.camera();
        (c.scroll_x(), c.scroll_y(), c.dir())
    };
    let mut carry = x >= scroll_x;
    if dir & 0x02 == 0 {
        let y = get(mem, index, f::Y);
        carry = y >= scroll_y;
    }
    let hi = get(mem, index, f::HI);
    let ppuctrl = mem.ppuctrl_zp();
    let same_page = (hi ^ ppuctrl) & 0x01 == 0;
    !(if same_page { carry } else { !carry })
}

/// `UpdateCannon_RunInstructions ; 03:9B70`。
fn run_instructions(mem: &mut Manager, index: u8) {
    let list_id = get(mem, index, f::INSTR_LIST_ID);
    let list_id = (list_id as usize).min(4);
    // `lda instrDelay / bne @endIf_A` ⇒ 为 0 时重装延迟并推进指令号
    if get(mem, index, f::INSTR_DELAY) == 0 {
        put(mem, index, f::INSTR_DELAY, INSTR_DELAY[list_id]);
        let id = get(mem, index, f::INSTR_ID);
        put(mem, index, f::INSTR_ID, id.wrapping_add(1));
    }
    let d = get(mem, index, f::INSTR_DELAY);
    put(mem, index, f::INSTR_DELAY, d.wrapping_sub(1));

    // `@getInstruction`：取指令并按类分派（角度 / 开火 / 回表头）
    let base = INSTR_OFFSET[list_id] as usize;
    let lists = game_data::assets::cannon_instr_lists();
    loop {
        let id = get(mem, index, f::INSTR_ID);
        let Some(&instr) = lists.get(base + id as usize) else {
            return;
        };
        if instr & 0x80 == 0 {
            // 角度指令：写 angle 并结束
            put(mem, index, f::ANGLE, instr);
            return;
        }
        if instr == 0xFF {
            // 回表头再取一条
            put(mem, index, f::INSTR_ID, 0x00);
            continue;
        }
        // 开火指令：推进指令号 → 开火 → 回 `@getInstruction`
        put(mem, index, f::INSTR_ID, id.wrapping_add(1));
        shoot_en_projectile(mem, index, instr);
    }
}

/// `Cannon_ShootEnProjectile ; 03:9BAF`（`instr` = `$FC/$FD/$FE`）。
fn shoot_en_projectile(mem: &mut Manager, index: u8, instr: u8) {
    if mem.mb_status() >= 0x04 {
        return; // 母脑将死/已死：不开火
    }
    // 找第一个空敌弹槽（`EnProjectiles.0..3` 之后的 6 槽中的前 4 个循环上限是 `EnProjectiles.4-Ens`）
    let mut found = None;
    for i in 0..crate::enemy::en_projectile::SLOT_COUNT {
        let slot = crate::enemy::en_projectile::base_slot(i);
        if mem.enemy(slot).status() == EnemyStatus::NoEnemy {
            found = Some(slot);
            break;
        }
    }
    let Some(slot) = found else { return };

    // 位置 = 炮台位置；hi = 炮台 hi
    let (cy, cx, chi) = (
        get(mem, index, f::Y),
        get(mem, index, f::X),
        get(mem, index, f::HI),
    );
    {
        let mut e = mem.enemy(slot);
        e.set_y(cy);
        e.set_pos_x(cx);
        e.set_hi(chi);
        e.set_status(EnemyStatus::Active);
        e.set_delay(0x00);
        e.set_anim_delay(0x00);
        e.set_movement_index(0x00);
    }
    // `$FC/$FD/$FE` ⇒ `$04/$03/$02`（`TwosComplement_`）
    let data0a = instr.wrapping_neg();
    let k = (data0a - 2) as usize; // 0/1/2
    {
        let mut e = mem.enemy(slot);
        e.set_data0a(data0a);
        e.set_data05(data0a | 0x02);
        e.set_reset_anim_index(EN_PROJ_ANIM[k]);
        e.set_anim_index(EN_PROJ_ANIM[k]);
    }
    // 炮台位置加偏移后写回敌弹槽（asm：Temp08/09/0B ← 炮台位置，ApplySpeedToPosition，再
    // LoadEnemyPositionFromTemp_）
    let x_off = crate::enemy::en_projectile::ens_offset(
        (slot - crate::enemy::en_projectile::base_slot(0)) as usize,
    );
    let (_, ny, nx, nhi) = apply_speed_to_position_parts(mem, x_off, EN_PROJ_DY[k], EN_PROJ_DX[k]);
    let mut e = mem.enemy(slot);
    e.set_y(ny);
    e.set_pos_x(nx);
    e.set_hi(nhi);
}

/// `DrawCannon_Normal/Escape ; 03:9C2B/9C31`：借母脑槽画炮台帧。
fn draw(mem: &mut Manager, index: u8, frame: u8) {
    let y = get(mem, index, f::Y);
    let x = get(mem, index, f::X);
    let hi = get(mem, index, f::HI);
    {
        let mut e = mem.ens_face(DRAW_SLOT);
        e.set_anim_frame(frame);
        e.set_y(y);
        e.set_pos_x(x);
        e.set_hi(hi);
    }
    crate::render::enemy_sprite::draw_enemy(mem, DRAW_SLOT);
}

#[cfg(test)]
mod tests_cannon {
    use crate::enemy::state::EnemyStatus;
    use crate::frame::Manager;

    /// 造一座炮台：`index` 槽、指令表 `list`、指令号 `id`、位置 (y,x,hi)。
    fn cannon(m: &mut Manager, index: u8, list: u8, id: u8) {
        crate::frame::set_in_area_raw(m, 0x13); // Tourian
        m.camera().set_dir(0x02); // 横滚房：只比 ScrollX
        m.set_ppuctrl_zp(0x00);
        m.camera().set_scroll_x(0x00);
        m.store_end_timer(0x00, 0xFF); // 逃脱计时器停用（asm：高位 $FF=inactive）
        super::put(m, index, super::f::STATUS, 0x01);
        super::put(m, index, super::f::Y, 0x80);
        super::put(m, index, super::f::X, 0x60);
        super::put(m, index, super::f::HI, 0x00);
        super::put(m, index, super::f::INSTR_LIST_ID, list);
        super::put(m, index, super::f::ANGLE, 0x00);
        super::put(m, index, super::f::INSTR_DELAY, 0x00);
        super::put(m, index, super::f::INSTR_ID, id);
    }

    fn prog(m: &mut Manager, i: usize) -> u8 {
        m.enemy(crate::enemy::en_projectile::base_slot(i))
            .status_raw()
    }

    /// asm 的 `bne` 让**槽 0 不被更新**（原版 quirk）。
    #[test]
    fn slot_zero_is_never_updated() {
        let mut m = Manager::default();
        cannon(&mut m, 0, 0, 0);
        super::update_all_cannons(&mut m);
        assert_eq!(
            super::get(&m, 0, super::f::INSTR_ID),
            0,
            "槽 0 的指令号不得推进"
        );
        // 对照：槽 1 会推进
        let mut m2 = Manager::default();
        cannon(&mut m2, 1, 0, 0);
        super::update_all_cannons(&mut m2);
        assert_eq!(super::get(&m2, 1, super::f::INSTR_ID), 1);
    }

    /// 首帧装 40 帧延迟、指令号 +1，并把该位置的**角度**写进 `angle`。
    #[test]
    fn instruction_sets_angle_and_reloads_delay() {
        let mut m = Manager::default();
        cannon(&mut m, 1, 0, 0);
        super::update_all_cannons(&mut m);
        assert_eq!(super::get(&m, 1, super::f::INSTR_DELAY), 0x27);
        assert_eq!(super::get(&m, 1, super::f::INSTR_ID), 1);
        assert_eq!(super::get(&m, 1, super::f::ANGLE), 0x01);
        // 延迟未到 ⇒ 角度不变、延迟递减
        super::update_all_cannons(&mut m);
        assert_eq!(super::get(&m, 1, super::f::ANGLE), 0x01);
        assert_eq!(super::get(&m, 1, super::f::INSTR_DELAY), 0x26);
    }

    /// `$FD` 是开火指令：生成一颗敌弹（`data0A=$03`、`data05=data0A|$02`、动画=DownLeft）。
    #[test]
    fn fire_instruction_spawns_en_projectile() {
        let mut m = Manager::default();
        // 表 0：索引 3 是 $FD ⇒ 让 instrID 从 2 起，加一后读到 $FD
        cannon(&mut m, 1, 0, 2);
        super::update_all_cannons(&mut m);
        assert_eq!(prog(&mut m, 0), 0x02, "敌弹槽 0 变 Active");
        let slot = crate::enemy::en_projectile::base_slot(0);
        assert_eq!(m.enemy(slot).data0a(), 0x03, "$FD 的补码");
        assert_eq!(m.enemy(slot).data05(), 0x03);
        assert_eq!(m.enemy(slot).anim_index(), 4, "EnAnim_CannonBulletDownLeft");
        // 开火后继续取下一条 ⇒ 表 0 索引 4 = $03 ⇒ 角度 3
        assert_eq!(super::get(&m, 1, super::f::ANGLE), 0x03);
    }

    /// 母脑将死（`MotherBrainStatus >= 4`）⇒ 不开火。
    #[test]
    fn no_projectile_when_mother_brain_is_dying() {
        let mut m = Manager::default();
        cannon(&mut m, 1, 0, 2);
        m.set_mb_status(0x04);
        super::update_all_cannons(&mut m);
        assert_eq!(prog(&mut m, 0), 0x00, "敌弹槽保持空");
    }

    /// 逃脱计时器活动：`FrameCount & $02` 的帧闪 `CannonTimeBombSet`(25)，且**不开火**。
    #[test]
    fn escape_mode_flashes_and_stops_firing() {
        let mut m = Manager::default();
        cannon(&mut m, 1, 0, 2);
        m.store_end_timer(0x99, 0x99); // 计时器活动
        m.set_frame_count(0x00);
        super::update_all_cannons(&mut m);
        assert_eq!(prog(&mut m, 0), 0x00, "逃脱期不开火");
        assert_eq!(
            m.ens_face(super::DRAW_SLOT).anim_frame(),
            super::FRAME_TIME_BOMB_SET,
            "画的是时炸弹帧"
        );
        // FrameCount&2 != 0 ⇒ 整帧跳过（不动 animFrame）
        let mut m2 = Manager::default();
        cannon(&mut m2, 1, 0, 2);
        m2.store_end_timer(0x99, 0x99);
        m2.set_frame_count(0x02);
        super::update_all_cannons(&mut m2);
        assert_eq!(m2.ens_face(super::DRAW_SLOT).anim_frame(), 0x00, "不画");
    }

    /// 异页且未进位的炮台算屏外 ⇒ 不推进指令。
    #[test]
    fn offscreen_cannon_is_skipped() {
        let mut m = Manager::default();
        cannon(&mut m, 1, 0, 0);
        super::put(&mut m, 1, super::f::HI, 0x01); // 与 PPUCTRL(=0) 异页
        // 异页 + 进位（cannon.x=$60 >= ScrollX=$40）⇒ 屏外（asm：异页时进位反而不可见）
        m.camera().set_scroll_x(0x40);
        super::update_all_cannons(&mut m);
        assert_eq!(super::get(&m, 1, super::f::INSTR_ID), 0, "屏外跳过");
    }

    /// `instrListID == 5` 的炮台什么都不做（asm 注释「won't do anything」）。
    #[test]
    fn list_five_cannon_is_inert() {
        let mut m = Manager::default();
        cannon(&mut m, 1, 5, 0);
        super::update_all_cannons(&mut m);
        assert_eq!(super::get(&m, 1, super::f::INSTR_ID), 0);
        assert_eq!(m.ens_face(super::DRAW_SLOT).anim_frame(), 0x00);
    }
}
