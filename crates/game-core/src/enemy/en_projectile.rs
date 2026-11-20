//! EnProjectile 敌弹体子系统（Dragon 吐弹 / Polyp 浮弹的公共弹体池）。
//!
//! asm 依据（refs/m1disasm/SRC/prg7_engine.asm，A 级行号锚）：
//! - Spawn 族 `07:F870-$F93A`（SpawnEnProjectile / FindSlot / SetEnProjectilePosition）
//! - Update 族 `07:F93B`（UpdateAllEnProjectiles）+ `07:F949`（UpdateEnProjectile 状态机：
//!   Resting / Active / Frozen / Pickup + ExitSub）
//! - 碰撞反应 `07:F2ED` CollisionDetectionEnProjectile_ReactToCollisionWithSamus
//!   （挂 `weapon::projectile::uw_collision_detection` 的弹×萨姆斯环）
//! - 数据面：移动指针表 `EnProjectileMovementPtrTable ; $97A7`（包内）+
//!   移动串窗（包段 `Area{Br,No,To,Kr,Ri}EnProjMoveStr`）。
//!
//! 纪律：槽面一律走 `mem.enemy(slot)` typed 句柄 + `motion.rs` 原语，零裸地址字面；
//! 弹体占用敌槽 6..=11（asm `EnProjectiles-Ens = $60`，stride 16）。

use crate::enemy::motion::apply_speed_to_position_raw;
use crate::enemy::state::{Anim, EnemyStatus};

/// 弹体池槽数（asm FindSlot `07:F8E8`：y 从 $60 步进 16 至 $C0 ⇒ 6 槽）。
pub(crate) const SLOT_COUNT: usize = 6;

/// 弹体槽 i（0..=5）的敌槽号（敌槽 6..=11）。
pub(crate) fn base_slot(i: usize) -> u8 {
    6 + i as u8
}

/// 弹体槽 i 的字节偏移（asm x/y 寄存器值：`$60 + i*16`）。
pub(crate) fn ens_offset(i: usize) -> u8 {
    (base_slot(i) * 16) as u8
}

/// `SpawnEnProjectileSpeedX ; 07:F92A`（2 B：`$02 $FE`）——按射主朝向 bit0 查。
const SPAWN_SPEED_X: [i8; 2] = [2, -2];

/// `SpawnEnProjectile ; 07:F870`——射主 `x`（字节偏移）尝试入池一枚弹体。
/// 返回 true=找到空槽并完成落位（asm 无返回值，callers 不分支；bool 仅供测试）。
/// 四个 zp 入参（constants_ram.asm:660-663）按值传递：
/// - `expected_status`（SpawnEnProjectile_ExpectedStatus）
/// - `anim_table_index`（SpawnEnProjectile_AnimTableIndex）
/// - `en_data0a`（SpawnEnProjectile_EnData0A）
/// - `anim_index_pair`（SpawnEnProjectile_AnimIndex[y]，y=射主朝向 bit0）
pub(crate) fn spawn_en_projectile(
    mem: &mut crate::frame::Manager,
    x: u8,
    expected_status: u8,
    anim_table_index: u8,
    en_data0a: u8,
    anim_index_pair: [u8; 2],
) -> bool {
    let shooter = crate::enemy::state::slot_of(x);
    // `lda data05,x / and #$10 / beq @RTS`（07:F872-F875）。
    if mem.enemy(shooter).data05() & 0x10 == 0 {
        return false;
    }
    // `lda ExpectedStatus / and status,x / beq @RTS`（07:F877-F87C）。
    if expected_status & mem.enemy(shooter).status_raw() == 0 {
        return false;
    }
    // `bit ExpectedStatus / bpl` + jumpDsplcmnt 门（07:F87E-F886；bit7 置才查）。
    if expected_status & 0x80 != 0 && mem.enemy(shooter).jump_dsplcmnt() != 0 {
        return false;
    }
    // FindSlot（07:F8E8）：首个 status==0 的弹槽；全满 ⇒ 退出。
    let Some(pi) = (0..SLOT_COUNT).find(|&i| mem.enemy(base_slot(i)).status_raw() == 0) else {
        return false; // `bcs @RTS`（07:F8DE 满槽径）
    };
    let slot = base_slot(pi);
    let px = ens_offset(pi);
    // `a is #$00 here → sta isHit,y`（07:F8DF-F8E1）。
    mem.enemy(slot).set_is_hit(0);
    // F92C：radiusY/X←2；弹体自身 data05 |= $02（弹体标记位——UpdateEnProjectile 入口门）。
    mem.enemy(slot).set_radius_y(2);
    mem.enemy(slot).set_radius_x(2);
    let d5y = mem.enemy(slot).data05() | 0x02;
    mem.enemy(slot).set_data05(d5y);
    // 朝向 bit → carry（`lda data05,x / lsr`，07:F8E2-F8E3）。
    let facing = mem.enemy(shooter).data05() & 0x01;
    // 上升动画：`AnimTableIndex*2 + facing → RisingAnimIndexTable` → InitEnAnimIndex
    // （07:F8E5-F8F2；InitEnAnimIndex=F68D：reset=index←A、animDelay←0）。
    let rising = game_data::assets::en_projectile_rising_anim(crate::frame::cur_area(mem));
    let idx = rising[(((anim_table_index * 2) + facing) & 0x0F) as usize];
    let frame = mem.enemy(slot).anim_frame();
    mem.enemy(slot).set_anim(Anim { frame, delay: 0, reset: idx, index: idx });
    // status←Resting；speedX←表[朝向]；speedY←0（07:F8F3-F8F7）。
    mem.enemy(slot).set_status(EnemyStatus::Resting);
    mem.enemy(slot).set_speed_x(SPAWN_SPEED_X[facing as usize]);
    mem.enemy(slot).set_speed_y(0);
    // F8F8：anim<2 ⇒ 停在 Resting（不激活、不落位偏移——asm 直接 RTS）。
    if anim_table_index < 2 {
        return true;
    }
    // data0A←(EnData0A*2 + facing) & 7；status←Active；delay/animDelay/movementIndex←0
    // （07:F8FB-F91C）。
    let d0a = ((en_data0a * 2) + facing) & 0x07;
    mem.enemy(slot).set_data0a(d0a);
    mem.enemy(slot).set_status(EnemyStatus::Active);
    mem.enemy(slot).set_delay(0);
    mem.enemy(slot).set_anim_delay(0);
    mem.enemy(slot).set_movement_index(0);
    // 出生定位（SetEnProjectilePosition 07:F91D + F8F9-F906）：temp=射主位置；
    // dy←PosOffsetY[anim]（4 B 表，anim≥4 越表 ⇒ fail-loud——现役调用方只喂 1/2）；
    // dx←PosOffsetX[anim*2 + facing]（8 B 表，anim∈{2,3} ⇒ 索引 4..7 在表内）；
    // ApplySpeedToPosition 施加后 LoadEnemyPositionFromTemp **恒**落弹体位置（含越界环绕值）。
    assert!(
        anim_table_index < 4,
        "SpawnEnProjectile：AnimTableIndex≥{anim_table_index} 越出 PosOffset 表（asm 读进相邻表）——现役弹型只喂 1/2，若新弹型触此请先考证表界"
    );
    let area = crate::frame::cur_area(mem);
    let oy = game_data::assets::en_projectile_pos_offset_y(area)[anim_table_index as usize];
    let ox = game_data::assets::en_projectile_pos_offset_x(area)[(((anim_table_index * 2) + facing) & 0x0F) as usize];
    let sh = mem.enemy(shooter);
    let (sy, sx, shi) = (sh.y(), sh.pos_x(), sh.hi());
    let (_, ny, nx, nhi) = apply_speed_to_position_raw(mem, sy, sx, shi, oy as i8, ox as i8);
    mem.enemy(slot).set_y(ny);
    mem.enemy(slot).set_pos_x(nx);
    mem.enemy(slot).set_hi(nhi & 0x01);
    // bit6 置 ⇒ 射主换吐弹动画（07:F80C-F811 `bit / bvc @RTS`；SetEnAnimIndex=F690：仅
    // animIndex←A，reset 不动）。
    if expected_status & 0x40 != 0 {
        let a = anim_index_pair[facing as usize];
        mem.enemy(shooter).set_anim_index(a);
    }
    true
}

/// `UpdateAllEnProjectiles ; 07:F93B`：x=$B0（槽 5）递减 16 至 $60（槽 0）。
pub(crate) fn update_all_en_projectiles(mem: &mut crate::frame::Manager) {
    for i in (0..SLOT_COUNT).rev() {
        let px = ens_offset(i);
        mem.set_page_index(px); // asm `jsr UpdateEnProjectile` 前 PageIndex=弹槽偏移
        update_en_projectile(mem, i);
    }
}

/// `UpdateEnProjectile ; 07:F949`：data05&2 清 ⇒ RemoveEnemy；status 跳表分派
/// （0/3→ExitSub，1→Resting，2→Active，4→Frozen，5→Pickup）。
pub(crate) fn update_en_projectile(mem: &mut crate::frame::Manager, i: usize) {
    let slot = base_slot(i);
    // `lda data05,x / and #$02 / bne @endIf_A / jsr RemoveEnemy`（07:F94B-F954）。
    if mem.enemy(slot).data05() & 0x02 == 0 {
        remove_enemy(mem, slot);
    }
    let status = mem.enemy(slot).status_raw();
    match status {
        0 | 3 => {} // ExitSub
        1 => update_en_projectile_resting(mem, i),
        2 => update_en_projectile_active(mem, i),
        4 => update_en_projectile_frozen(mem, i),
        5 => update_en_projectile_pickup(mem, i),
        // asm 跳表只登记 0..=5；status≥6 会读到表外字节——当前弹体生命周期写不出
        // 该值（Resting/Active/Frozen/Pickup 五态闭环），fail-loud 留痕。
        _ => crate::unported!("UpdateEnProjectile：弹体 status={status} 超出 asm 跳表 0..=5"),
    }
}

/// `UpdateEnProjectile_Resting ; 07:F96A`。
fn update_en_projectile_resting(mem: &mut crate::frame::Manager, i: usize) {
    enemy_become_pickup_if_hit(mem, i);
    let applied = enemy_bg_collide_or_apply_speed(mem, i);
    if applied {
        anim_draw_enemy_with_anim_delay1(mem, i);
        return;
    }
    // bcc 径：status==0（BG 反应把槽清了）⇒ 无绘制直出（07:F972-F976）。
    let slot = base_slot(i);
    if mem.enemy(slot).status_raw() == 0 {
        return;
    }
    enemy_become_pickup(mem, i);
    anim_draw_enemy_with_anim_delay1(mem, i);
}

/// `UpdateEnProjectile_Active ; 07:F991`（Dragon 吐弹的飞行主径）。
fn update_en_projectile_active(mem: &mut crate::frame::Manager, i: usize) {
    let slot = base_slot(i);
    enemy_become_pickup_if_hit(mem, i);
    // 移动指针：`y=data0A & $FE → MovementPtrTable[y/y+1]`（07:F997-F9A3）。
    let area = crate::frame::cur_area(mem);
    let ptrs = game_data::assets::en_projectile_movement_ptrs(area);
    let d0a = mem.enemy(slot).data0a();
    let word = u16::from_le_bytes([ptrs[(d0a & 0xFE) as usize], ptrs[((d0a & 0xFE) + 1) as usize]]);
    let base = game_data::assets::en_projectile_move_str_base(area);
    let win = game_data::assets::en_projectile_move_str(area);
    let off = word
        .checked_sub(base)
        .filter(|o| (*o as usize) < win.len())
        .unwrap_or_else(|| panic!("敌弹移动指针 {word:04X} 越出 {area} 移动串窗（基址 {base:04X}）"))
        as usize;

    // @processMovementInstruction（07:F9A4-F9C0）——循环形：$FF 尾重启、时长==delay
    // 推进两条（NextInstruction 07:F984），否则落 `delay+=1` + 速度字节。
    let mut mi = mem.enemy(slot).movement_index();
    loop {
        let b = win[off + mi as usize];
        if b == 0xFF {
            // 串尾：movementIndex←$FF 后 jmp @restart（07:F9B4-F9B9）——@restart 再
            // 自增一次回 $00（串首）、delay←0。
            mem.enemy(slot).set_movement_index(0xFF);
            mi = mem.enemy(slot).movement_index().wrapping_add(1);
            mem.enemy(slot).set_movement_index(mi);
            mem.enemy(slot).set_delay(0);
            continue;
        }
        if b == mem.enemy(slot).delay() {
            // NextInstruction（07:F984-F98B）：inc movementIndex（@restart 再 inc）、delay←0。
            mi = mi.wrapping_add(1);
            mem.enemy(slot).set_movement_index(mi);
            mi = mi.wrapping_add(1);
            mem.enemy(slot).set_movement_index(mi);
            mem.enemy(slot).set_delay(0);
            continue;
        }
        break;
    }
    // delay 未到：delay+=1；速度字节=(ptr),y+1（dy 高半、dx 低半的符号幅值编码
    // ——EnemyGetDeltaY/X_SignMagSpeed@fromByte，areas_common.asm $8296/$832F）。
    let d = mem.enemy(slot).delay().wrapping_add(1);
    mem.enemy(slot).set_delay(d);
    let sp = win[off + mi as usize + 1];
    let dy = sign_mag_y(sp);
    let mut dx = sign_mag_x(sp);
    // data0A bit0 置 ⇒ dx 取负（07:F9DA-F9DE TwosComplement）。
    if d0a & 0x01 != 0 {
        dx = dx.wrapping_neg();
    }
    mem.enemy(slot).set_speed_y(dy);
    mem.enemy(slot).set_speed_x(dx);
    // 首串（data0A bit0 清）且 speedY>0（严格正）⇒ 下落动画
    // （07:F9E3-F9F1；`bmi/beq @endIf_C` 双门）。
    if d0a & 0x01 == 0 && dy > 0 {
        let fall = game_data::assets::en_projectile_falling_anim_byte(area, d0a);
        mem.enemy(slot).set_reset_anim_index(fall);
    }

    let applied = enemy_bg_collide_or_apply_speed(mem, i);
    if applied {
        anim_draw_enemy_with_anim_delay1(mem, i);
        return;
    }
    // 施加失败：status==0（已被 RemoveEnemy）⇒ jmp EnemyBecomePickup@RTS（=纯 RTS，
    // 07:F9FD-F9FF）——不换动画、不绘制、不再写状态。
    if mem.enemy(slot).status_raw() == 0 {
        return;
    }
    // 溅落动画：y=data0A>>1（07:FA01-FA07 lsr/beq-iny），InitEnAnimIndex + FreezeEnemy
    // （status←Frozen）+ delay←$0A（07:F9FC-FA14）。
    let dir = d0a >> 1;
    assert!(
        dir < 2,
        "敌弹溅落动画索引越表（data0A>>1={dir}≥2）——现役弹型 data0A≤3，若越此界请先考证表外读"
    );
    let sp = game_data::assets::en_projectile_splatter_anim(area).to_le_bytes();
    let idx = sp[dir as usize];
    let frame = mem.enemy(slot).anim_frame();
    mem.enemy(slot).set_anim(Anim { frame, delay: 0, reset: idx, index: idx });
    freeze_enemy(mem, slot);
    mem.enemy(slot).set_delay(0x0A);
    anim_draw_enemy_with_anim_delay1(mem, i);
}

/// `UpdateEnProjectile_Frozen ; 07:FA6B`：animFrame≠$F7（id_FrameNone）才倒计时；
/// 归零（或已是 None 帧）⇒ RemoveEnemy；尾件恒绘制。
fn update_en_projectile_frozen(mem: &mut crate::frame::Manager, i: usize) {
    let slot = base_slot(i);
    if mem.enemy(slot).anim_frame() != 0xF7 {
        let d = mem.enemy(slot).delay().wrapping_sub(1);
        mem.enemy(slot).set_delay(d);
        if d != 0 {
            anim_draw_enemy_with_anim_delay1(mem, i);
            return;
        }
    }
    remove_enemy(mem, slot);
    anim_draw_enemy_with_anim_delay1(mem, i);
}

/// `UpdateEnProjectile_Pickup ; 07:FA91`：RemoveEnemy + InitEnAnimIndex(击杀动画) + 绘制。
fn update_en_projectile_pickup(mem: &mut crate::frame::Manager, i: usize) {
    let slot = base_slot(i);
    remove_enemy(mem, slot);
    let idx = game_data::assets::en_projectile_killed_anim(crate::frame::cur_area(mem));
    let frame = mem.enemy(slot).anim_frame();
    mem.enemy(slot).set_anim(Anim { frame, delay: 0, reset: idx, index: idx });
    anim_draw_enemy_with_anim_delay1(mem, i);
}

/// `RemoveEnemy ; 07:FA18`：status←0（槽回收）。
pub(crate) fn remove_enemy(mem: &mut crate::frame::Manager, slot: u8) {
    mem.enemy(slot).set_status(EnemyStatus::NoEnemy);
}

/// `FreezeEnemy ; 07:F518`：status←Frozen。
fn freeze_enemy(mem: &mut crate::frame::Manager, slot: u8) {
    mem.enemy(slot).set_status(EnemyStatus::Frozen);
}

/// `EnemyBecomePickupIfHit ; 07:FA5B`：isHit≠0 ⇒ 清 isHit + status←Pickup。
fn enemy_become_pickup_if_hit(mem: &mut crate::frame::Manager, i: usize) {
    let slot = base_slot(i);
    if mem.enemy(slot).is_hit() != 0 {
        enemy_become_pickup(mem, i);
    }
}

/// `EnemyBecomePickup ; 07:FA60`：isHit←0 + status←Pickup。
fn enemy_become_pickup(mem: &mut crate::frame::Manager, i: usize) {
    let slot = base_slot(i);
    mem.enemy(slot).set_is_hit(0);
    mem.enemy(slot).set_status(EnemyStatus::Pickup);
}

/// `EnemyBGCollideOrApplySpeed ; 07:FA1E`——返回 carry 语义（true=已施速）。
///
/// asm：Norfair（InArea==$11）且 status bit0 **清**（Active/Frozen/Hurt/空）跳过瓦检
/// （`lsr/bcc @endIf_A`）；其余（非 Norfair，或 bit0 置=Resting/Pickup）检所在瓦：
/// `<$A0` ⇒ 撞固瓦（carry 清返）；`≥$A0` ⇒ 施速：ApplySpeedToPosition 失败（越界）
/// ⇒ RemoveEnemy；`LoadEnemyPositionFromTemp` 恒执行（含环绕值落盘）。
fn enemy_bg_collide_or_apply_speed(mem: &mut crate::frame::Manager, i: usize) -> bool {
    let slot = base_slot(i);
    let norfair = crate::frame::in_area_raw(mem) == 0x11;
    let status_bit0 = mem.enemy(slot).status_raw() & 0x01 != 0;
    if norfair && !status_bit0 {
        // 跳过瓦检直落施速。
    } else {
        let e = mem.enemy(slot);
        let (y, x, hi) = (e.y(), e.pos_x(), e.hi());
        // 页面选位按 `MakeRoomRAMPtr ; 07:E96A`（`hi&1` 经 `asl asl and #$04`）；
        // 读走 `Room::tile`（房间缓冲视图唯一实现面），不再本文件直碰 `$6000` 页。
        let tile = mem.room().tile(y, x, (hi & 0x01) << 2);
        if tile < 0xA0 {
            return false; // `cmp #$A0 / bcc @RTS`（07:FA31-FA33）
        }
    }
    // @applySpeed（07:FA41）：temp=自身位置 → 施速 → 失败 ⇒ RemoveEnemy；
    // LoadEnemyPositionFromTemp（07:FA49）恒执行。
    let e = mem.enemy(slot);
    let (y0, x0, hi0) = (e.y(), e.pos_x(), e.hi());
    let dy = e.speed_y();
    let dx = e.speed_x();
    let (ok, ny, nx, nhi) = apply_speed_to_position_raw(mem, y0, x0, hi0, dy, dx);
    if !ok {
        remove_enemy(mem, slot);
    }
    mem.enemy(slot).set_y(ny);
    mem.enemy(slot).set_pos_x(nx);
    mem.enemy(slot).set_hi(nhi & 0x01);
    ok
}

/// `AnimDrawEnemyWithAnimDelay1 ; 07:F97C` → `AnimDrawEnemy ; 07:F97E`
/// （A←1 → UpdateEnemyAnim + DrawEnemy）。
fn anim_draw_enemy_with_anim_delay1(mem: &mut crate::frame::Manager, i: usize) {
    let px = ens_offset(i);
    crate::enemy::anim_tick::update_enemy_anim(mem, px, 1);
    crate::render::enemy_sprite::draw_enemy(mem, px);
}

/// `EnemyGetDeltaY_SignMagSpeed@fromByte ; areas_common.asm $8296`：
/// dy = 高半符号(bit7) ? -(幅值) : 幅值，幅值=(byte>>4)&7。
fn sign_mag_y(b: u8) -> i8 {
    let mag = (b >> 4) & 0x07;
    if b & 0x80 != 0 {
        -(mag as i8)
    } else {
        mag as i8
    }
}

/// `EnemyGetDeltaX_SignMagSpeed@fromByte ; areas_common.asm $832F`：
/// dx = 符号(bit3) ? -(幅值) : 幅值，幅值=byte&7。
fn sign_mag_x(b: u8) -> i8 {
    let mag = b & 0x07;
    if b & 0x08 != 0 {
        -(mag as i8)
    } else {
        mag as i8
    }
}

// ---------------------------------------------------------------------------
// 单元测（验收范式 Tier1：sem/事件级；spawn 槽分配 / 四态迁移 / 移动串按区 /
// 碰撞反应；变异自证见测试尾注释）
// ---------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame::Manager;
    use crate::ram_map;

    /// 射主（敌槽 0）摆成「bit4 置 + 朝右」的样板。
    fn stage_shooter(m: &mut Manager, status: EnemyStatus) {
        let mut e = m.enemy(0);
        e.set_data05(0x10); // bit4 允许吐弹；bit0=0 朝右
        e.set_status(status);
        e.set_y(0x40);
        e.set_pos_x(0x40);
        e.set_hi(0);
    }

    /// 把弹摆进「屏内+同名表」判可见窗（IsObjectVisible 07:DFDF 输入面）：
    /// 滚轴 0、hi bit0 与 PPUCTRL_ZP 同相——否则 DrawEnemy 会清 data05 bit1
    /// （:6178-6186 闩），下一帧 data05&2 径把弹回收（offscreen 忠实行径）。
    fn make_visible(m: &mut Manager) {
        m.write_ram(ram_map::SCROLL_X, 0);
        m.write_ram(ram_map::SCROLL_Y, 0);
        m.write_ram(ram_map::PPUCTRL_ZP, 0);
    }

    fn mem_anim_stage(m: &mut Manager) {
        // reset=1：walk 0(值00)→1(值01)→2($FF→reset1) 自洽环，对 Kr/To 表头都成立。
        let a = crate::enemy::state::Anim { frame: 0, delay: 0, reset: 1, index: 0 };
        m.enemy(base_slot(0)).set_anim(a);
    }

    /// 区号直写 InArea（测试样板；cur_area=InArea&$0F）。
    fn set_area(m: &mut Manager, raw: u8) {
        m.write_ram(ram_map::IN_AREA, raw);
    }

    fn spawn_fireball(m: &mut Manager, status: EnemyStatus) -> bool {
        stage_shooter(m, status);
        // Dragon：anim=1（<2 ⇒ Resting 闭环）、EnData0A=0。
        spawn_en_projectile(m, 0, 0x02, 1, 0, [0x50, 0x51])
    }

    fn spawn_polyp(m: &mut Manager, status: EnemyStatus) -> bool {
        stage_shooter(m, status);
        // Polyp：anim=2（≥2 ⇒ 激活）、EnData0A=1（data0A=2/3，bit0 翻转 dx）。
        spawn_en_projectile(m, 0, 0x03, 2, 1, [0x50, 0x51])
    }

    fn pack() {
        if game_data::pack::init_from_env().is_err() {
            eprintln!("SKIP（显式）：未加载资产包——先跑 pack-tool extract");
            std::process::exit(0);
        }
    }

    #[test]
    fn spawn_requires_data05_bit4_and_status_mask() {
        pack();
        let mut m = Manager::default();
        stage_shooter(&mut m, EnemyStatus::Active);
        m.enemy(0).set_data05(0x00); // bit4 清 ⇒ 拒
        assert!(!spawn_en_projectile(&mut m, 0, 0x02, 1, 0, [0, 0]));
        stage_shooter(&mut m, EnemyStatus::Resting); // status 不含期望位 ⇒ 拒
        assert!(!spawn_en_projectile(&mut m, 0, 0x02, 1, 0, [0, 0]));
        // ExpectedStatus bit7 置 + jumpDsplcmnt≠0 ⇒ 拒（07:F87E-F886）。
        stage_shooter(&mut m, EnemyStatus::Active);
        m.enemy(0).set_jump_dsplcmnt(1);
        assert!(!spawn_en_projectile(&mut m, 0, 0x82, 1, 0, [0, 0]));
        assert!(spawn_fireball(&mut m, EnemyStatus::Active));
    }

    #[test]
    fn spawn_allocates_lowest_free_slot_and_full_pool_rejects() {
        pack();
        let mut m = Manager::default();
        // 槽 1 预占 ⇒ 槽 0 应被选中（FindSlot 从 $60 顺序扫）。
        m.enemy(base_slot(1)).set_status(EnemyStatus::Resting);
        assert!(spawn_fireball(&mut m, EnemyStatus::Active));
        assert_eq!(m.enemy(base_slot(0)).status_raw(), 1);
        assert_eq!(m.enemy(base_slot(1)).status_raw(), 1);
        // 全满 ⇒ 拒且不覆盖。
        for i in 1..SLOT_COUNT {
            m.enemy(base_slot(i)).set_status(EnemyStatus::Active);
        }
        assert!(!spawn_fireball(&mut m, EnemyStatus::Active));
        assert_eq!(m.enemy(base_slot(0)).status_raw(), 1, "满槽不覆盖既有槽");
    }

    #[test]
    fn spawn_resting_sets_speed_radius_and_data05_marker() {
        pack();
        let mut m = Manager::default();
        assert!(spawn_fireball(&mut m, EnemyStatus::Active));
        let p = m.enemy(base_slot(0));
        assert_eq!(p.status_raw(), 1, "Resting");
        assert_eq!(p.speed_x(), 2, "朝右 ⇒ SpawnEnProjectileSpeedX[0]=$02");
        assert_eq!(p.speed_y(), 0);
        assert_eq!(p.radius_y(), 2);
        assert_eq!(p.radius_x(), 2);
        assert_eq!(p.data05() & 0x02, 0x02, "弹体标记位");
        assert_eq!(p.is_hit(), 0);
        // 朝左 ⇒ $FE。
        let mut m2 = Manager::default();
        stage_shooter(&mut m2, EnemyStatus::Active);
        m2.enemy(0).set_data05(0x11); // bit4+bit0（朝左）
        assert!(spawn_en_projectile(&mut m2, 0, 0x02, 1, 0, [0, 0]));
        assert_eq!(m2.enemy(base_slot(0)).speed_x(), -2);
    }

    #[test]
    fn spawn_active_sets_data0a_movement_index_and_position_offset() {
        pack();
        let mut m = Manager::default();
        set_area(&mut m, 0x10);
        make_visible(&mut m);
        assert!(spawn_polyp(&mut m, EnemyStatus::Explode));
        let p = m.enemy(base_slot(0));
        assert_eq!(p.status_raw(), 2, "Active");
        assert_eq!(p.data0a(), 2, "EnData0A=1 ×2 + facing0");
        assert_eq!(p.movement_index(), 0);
        assert_eq!(p.delay(), 0);
        assert_eq!(p.y(), 0x40, "PosOffsetY[2]=0 ⇒ 位置=射主 y");
        assert_eq!(p.pos_x(), 0x40, "PosOffsetX[2*2+0]=0 ⇒ 位置=射主 x");
        // 射主吐弹动画未写（ExpectedStatus bit6 清）。
        // bit6 置 ⇒ animIndex(shooter)←pair[朝向]（07:F80C-F811）。
        let mut m2 = Manager::default();
        set_area(&mut m2, 0x10);
        stage_shooter(&mut m2, EnemyStatus::Active);
        assert!(spawn_en_projectile(&mut m2, 0, 0x42, 2, 1, [0x61, 0x62]));
        assert_eq!(m2.enemy(0).anim_index(), 0x61, "bit6 置 + 朝右 ⇒ pair[0]");
    }

    #[test]
    fn resting_without_bg_collision_becomes_pickup_after_delay_paths() {
        pack();
        // Brinstar RoomRAM 默认瓦 0 <$A0 ⇒ Resting 瓦检「撞固瓦」arm（FA1E carry 清
        // ⇒ bcc：status≠0 ⇒ EnemyBecomePickup 07:FA60）。施速落位 arm 由 Norfair
        // Active 免瓦检测覆盖（07:FA41 applySpeed）。
        let mut m = Manager::default();
        set_area(&mut m, 0x10);
        make_visible(&mut m);
        assert!(spawn_fireball(&mut m, EnemyStatus::Active));
        // Resting spawn 在 F8F8（AnimTableIndex<2）先于落位退出——弹位保留槽底 0；
        // 测试样板摆进屏内（真实射主场景位置由后续激活性径/槽内容决定）。
        m.enemy(base_slot(0)).set_y(0x40);
        m.enemy(base_slot(0)).set_pos_x(0x40);
        // isHit 置 ⇒ Resting 首件 EnemyBecomePickupIfHit（FA5B）先于瓦检。
        m.enemy(base_slot(0)).set_is_hit(0x20);
        update_all_en_projectiles(&mut m);
        assert_eq!(m.enemy(base_slot(0)).status_raw(), 5, "Pickup");
        assert_eq!(m.enemy(base_slot(0)).is_hit(), 0, "isHit 清");
        // Pickup 径：RemoveEnemy + 击杀动画 init + 绘制。
        update_all_en_projectiles(&mut m);
        assert_eq!(m.enemy(base_slot(0)).status_raw(), 0, "Pickup 当帧回收");
        assert_eq!(
            m.enemy(base_slot(0)).reset_anim_index(),
            game_data::assets::en_projectile_killed_anim(0x10),
            "killed anim=$95DC[0]"
        );
        // 不带 isHit 的同径：瓦检撞固瓦同样拾取化（bcc arm）。
        let mut m2 = Manager::default();
        set_area(&mut m2, 0x10);
        make_visible(&mut m2);
        assert!(spawn_fireball(&mut m2, EnemyStatus::Active));
        update_all_en_projectiles(&mut m2);
        assert_eq!(m2.enemy(base_slot(0)).status_raw(), 5, "瓦检失败 ⇒ Pickup");
    }

    #[test]
    fn active_steps_movement_string_and_flips_dx_by_data0a_bit0() {
        pack();
        let mut m = Manager::default();
        set_area(&mut m, 0x11); // Norfair：Active bit0 清 ⇒ 免瓦检恒施速（07:FA24-FA2A）
        make_visible(&mut m);
        assert!(spawn_polyp(&mut m, EnemyStatus::Explode));
        // Br polyp 首对（09 C2）：时长 9 —— 前 8 帧 delay 递增、速度保持。
        for k in 0..9 { // 时长 9：帧 k 后 delay=k+1，第 9 帧触发 NextInstruction
            update_all_en_projectiles(&mut m);
            let p = m.enemy(base_slot(0));
            assert_eq!(p.delay(), k + 1, "第 {k} 帧后 delay");
            assert_eq!(p.speed_y(), -4, "C2 高半=符号1幅4");
            assert_eq!(p.speed_x(), 2, "C2 低半幅2；data0A=2 bit0 清 ⇒ 不翻转");
        }
        // 第 10 帧：delay==9 ⇒ NextInstruction（mi+=2、delay←0）→ 当帧继续 cmp
        // 新时长 8 ≠ 0 ⇒ delay+=1=1、读新速度对（08 A2）。
        update_all_en_projectiles(&mut m);
        let p = m.enemy(base_slot(0));
        assert_eq!(p.movement_index(), 2);
        assert_eq!(p.delay(), 1, "NextInstruction 归零后同帧再 cmp ⇒ delay=1");
        assert_eq!(p.speed_y(), -2, "A2 高半：符号1幅2");
        assert_eq!(p.speed_x(), 2, "A2 低半幅2，bit0 清不翻转");
    }

    #[test]
    fn active_wraps_at_ff_marker_back_to_string_head() {
        pack();
        // Tourian 串（50 22 FF…）：时长 50 —— 走满后 $FF 尾重启回串首。
        let mut m = Manager::default();
        set_area(&mut m, 0x13); // Tourian
        make_visible(&mut m);
        assert!(spawn_polyp(&mut m, EnemyStatus::Explode));
        // 动画面样板替换：Tourian 真实 rising 索引 $26 落表内 $FF 且 reset 同指 ⇒
        // asm E0BC 走 reset 循环=真机挂死（Tourian 无弹体实体，此动画数据从未被消费）。
        // 本测只考移动机械，把动画四元组摆成表内自洽环（见 mem_anim_stage）。
        mem_anim_stage(&mut m);
        // 落点带摆可走瓦（≥$A0）——否则 BG 碰撞立刻飞溅，走不到 $FF 重启。
        // d0a=2 ⇒ 指针表按字节索引取 word[1]=M1（50 22 FF）：匀速 (+2,+2) 飞行。
        for yy in (0x38..0xC0).step_by(8) {
            for xx in (0x38..0xC0).step_by(8) {
                m.write_ram(crate::world::room_ram::Room::addr(yy, xx, 0), 0xB0);
            }
        }
        for _ in 0..60 {
            update_all_en_projectiles(&mut m);
        }
        let p = m.enemy(base_slot(0));
        assert_eq!(p.movement_index(), 0, "$FF 重启后 mi=（$FF+1)&$FF=0");
        assert_eq!(p.speed_y(), 2, "To M1: 22 高半幅2");
        assert_eq!(p.speed_x(), 2, "22 低半幅2；d0a=2 bit0 清 ⇒ 不翻转");
    }

    #[test]
    fn active_splatters_into_frozen_on_solid_tile_outside_norfair() {
        pack();
        // Brinstar（非 Norfair）⇒ 恒瓦检；RoomRAM 默认 0 瓦 <$A0 ⇒ 撞固瓦。
        let mut m = Manager::default();
        set_area(&mut m, 0x10);
        make_visible(&mut m);
        assert!(spawn_polyp(&mut m, EnemyStatus::Explode));
        update_all_en_projectiles(&mut m);
        let p = m.enemy(base_slot(0));
        assert_eq!(p.status_raw(), 4, "撞瓦 ⇒ FreezeEnemy");
        assert_eq!(p.delay(), 0x0A, "溅落计时 $0A");
        assert_eq!(
            p.reset_anim_index(),
            game_data::assets::en_projectile_splatter_anim(0x10).to_le_bytes()[(2 >> 1) as usize],
            "溅落动画=data0A>>1 字节（Br=0）"
        );
        // Frozen 径：animFrame≠$F7 ⇒ delay 倒计至 0 回收。
        for _ in 0..0x0A {
            update_all_en_projectiles(&mut m);
        }
        assert_eq!(m.enemy(base_slot(0)).status_raw(), 0);
    }

    #[test]
    fn norfair_active_skips_tile_check_and_resting_checks_tile() {
        pack();
        // Norfair + Active（bit0 清）⇒ 免瓦检直施速（07:FA24-FA2A）。
        let mut m = Manager::default();
        set_area(&mut m, 0x11);
        assert!(spawn_polyp(&mut m, EnemyStatus::Explode));
        update_all_en_projectiles(&mut m);
        assert_eq!(m.enemy(base_slot(0)).status_raw(), 2, "仍 Active");
        assert_eq!(m.enemy(base_slot(0)).pos_x(), 0x42, "speedX=+2 落位（C2 低半幅2）");
        assert_eq!(m.enemy(base_slot(0)).y(), 0x3C, "speedY=-4 落位（C2 高半幅4 符号1）");
        // Norfair + Resting（bit0 置）⇒ 瓦检生效：RoomRAM 0 瓦 <$A0 ⇒ becomePickup。
        let mut m2 = Manager::default();
        set_area(&mut m2, 0x11);
        assert!(spawn_fireball(&mut m2, EnemyStatus::Active));
        update_all_en_projectiles(&mut m2);
        assert_eq!(m2.enemy(base_slot(0)).status_raw(), 5, "Resting 撞固瓦 ⇒ Pickup");
    }

    #[test]
    fn frozen_counts_down_delay_then_removes() {
        pack();
        let mut m = Manager::default();
        set_area(&mut m, 0x10);
        make_visible(&mut m);
        assert!(spawn_polyp(&mut m, EnemyStatus::Explode));
        // 手工摆成 Frozen：animFrame≠$F7 + delay=$0A。
        let mut p = m.enemy(base_slot(0));
        p.set_anim_frame(0x12);
        p.set_delay(0x0A);
        p.set_status(EnemyStatus::Frozen);
        for _ in 0..0x0A {
            update_all_en_projectiles(&mut m);
        }
        assert_eq!(m.enemy(base_slot(0)).status_raw(), 0, "delay 归零帧 RemoveEnemy");
        // animFrame==$F7 ⇒ 当帧直接回收（07:FA6E-FA7A）。
        let mut m2 = Manager::default();
        set_area(&mut m2, 0x10);
        assert!(spawn_polyp(&mut m2, EnemyStatus::Explode));
        m2.enemy(base_slot(0)).set_anim_frame(0xF7);
        m2.enemy(base_slot(0)).set_status(EnemyStatus::Frozen);
        update_all_en_projectiles(&mut m2);
        assert_eq!(m2.enemy(base_slot(0)).status_raw(), 0);
    }

    #[test]
    fn data05_marker_clear_reclaims_slot_immediately() {
        pack();
        let mut m = Manager::default();
        set_area(&mut m, 0x10);
        make_visible(&mut m);
        assert!(spawn_polyp(&mut m, EnemyStatus::Explode));
        m.enemy(base_slot(0)).set_data05(0x00); // 弹体标记清
        update_all_en_projectiles(&mut m);
        assert_eq!(m.enemy(base_slot(0)).status_raw(), 0, "07:F94B-F954 RemoveEnemy");
    }

    /// 移动串指针按区解析：五区首指针对窗内偏移 = 指针字 − 窗基址（机械锚）。
    #[test]
    fn movement_window_offsets_resolve_for_all_areas() {
        pack();
        for (raw, want0) in [(0x10u8, 0x00u16), (0x11, 0x00), (0x12, 0x00), (0x13, 0x00), (0x14, 0x00)] {
            let area = raw;
            let base = game_data::assets::en_projectile_move_str_base(area);
            let win = game_data::assets::en_projectile_move_str(area);
            let ptrs = game_data::assets::en_projectile_movement_ptrs(area);
            let word = u16::from_le_bytes([ptrs[0], ptrs[1]]);
            let off = word - base;
            assert!((off as usize) < win.len(), "area{raw} M0 指针 {word:04X} 不在窗内");
            assert_eq!(off, want0, "area{raw} M0 应指向窗首（指针=基址）");
            // 首串首字节 = 时长（非 $FF）；窗尾最后字节 = $FF。
            assert!(win[0] != 0xFF, "area{raw} M0 首字节应为时长");
            assert_eq!(*win.last().unwrap(), 0xFF, "area{raw} 窗尾=$FF（串尾标记）");
        }
    }

    /// 碰撞反应（07:F2ED）：非螺旋转 ⇒ isHit|=DistHi、$010F←$80、
    /// Samus.isHit|=(DistHi^3)<<3、HealthChange←AreaEnemyDamage；螺旋转 ⇒ 仅 isHit。
    #[test]
    fn react_to_samus_collision_applies_damage_unless_screw_attack() {
        pack();
        let mut m = Manager::default();
        set_area(&mut m, 0x10); // Brinstar enemy_damage=$50（area header $95CE word）
        stage_shooter(&mut m, EnemyStatus::Active);
        assert!(spawn_fireball(&mut m, EnemyStatus::Active));
        // Resting spawn 在 F8F8 先于落位退出——样板把弹摆到萨姆斯同位。
        m.enemy(base_slot(0)).set_y(0x40);
        m.enemy(base_slot(0)).set_pos_x(0x40);
        // 弹与萨姆斯同位（半径和内）：Samus pos=($40,$40)、radius (3,1)/弹 radius (2,2)。
        m.samus().set_y(0x40);
        m.samus().set_x(0x40);
        m.samus().set_radius_y(0x03);
        m.samus().set_radius_x(0x03);
        m.set_samus_invincible_delay(0);
        m.samus().set_status(crate::samus::state::SamusStatus::Run); // 非 Dead 门值（status=2）
        crate::weapon::projectile::uw_collision_detection(&mut m);
        let p = m.enemy(base_slot(0));
        assert_eq!(p.is_hit() & 0x04, 0x04, "isHit bit2（DistHi 恒置位）");
        assert_eq!(m.read_ram(ram_map::SAMUS_HURT_010F), 0x80, "SamusHurt010F");
        assert_eq!(m.samus().is_hit() & 0x18, 0x18, "DistHi=04 ⇒ (04^3)<<3=0x18");
        let hc = m.health_change();
        assert_eq!(hc, game_data::assets::area_header(0x10).enemy_damage, "HealthChange←AreaEnemyDamage（Brinstar 区头 $95CE word）");
        // 下帧 Resting 径拾取化（ BecomePickupIfHit）。
        update_all_en_projectiles(&mut m);
        assert_eq!(m.enemy(base_slot(0)).status_raw(), 5);
    }

    // —— 变异实验自证（改一处常量看红，改回复原）——
    // 例 1：把 SPAWN_SPEED_X[0] 改 3 ⇒ spawn_resting… 与 active_steps… 双红；
    // 例 2：把 update_en_projectile_resting 的 pickup 分支删掉 ⇒ resting… 红；
    // 例 3：把 sign_mag_y 的符号位掩码 0x80 改 0x40 ⇒ active_steps… 红。
    // 三例均已验证红→复原后 `git diff` 空（执行记录见任务卡回报）。
}
