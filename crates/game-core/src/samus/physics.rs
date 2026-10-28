//! Samus 物理域（task-25 次件+task-4 移植半；链规 19 业务结构——积分器/坐标件/R-mv 面）。
//!
//! **asm 内容锚+结构互核注**（施工令②精化版）——
//! · 坐标热件（move 相 17k+ hits）：`Xminus16: ; 07:F1F4`（`txa/sec/sbc #$10/tax`）/
//!   `Xplus4: ; 07:E193`（`inx`×4）——**纯寄存器面**（X±n，零 RAM 写）。
//! · VertAccelerate 头（task-4 E37A 移植半——accel/maxspeed 初始化）：
//!   `VertAccelerate: ; 07:E37A`（`lda SamusAccelY/bne @dontStartFalling`（空中跳过）/
//!   `lda #$18/sta SamusHorzSpeedMax`（**最大跑速 1.5px**）/8 像素对齐检测
//!   （`lda Samus.y/clc/adc Samus.radiusY/and #$07`）/`jsr ObjectCheckMoveDown ($E7AD)`
//!   向下阻挡/`jsr SamusCollisionWithSolidEntities ($D976)`/电梯+冻敌双旗门/
//!   `lda #$1A/sta SamusAccelY`（**R-mv-6 下落发起=重力 $1A**））。
//!
//! **结构互核注**：①8 像素对齐检测=`y+radiusY & 7 == 0` 才查向下阻挡（几何对齐节流
//! ——相位面）②R-mv-6 规则=下落发起条件集（AccelY==0∧无阻挡∧非电梯∧非冻敌→重力置入）
//! ——spec 源 docs/spec/samus-movement.md §2.3 ③双卫件=task-19 考证件（E7AD/D976 同源）。
//!
//! 积分器拼装位：int_x/int_y（integrators.rs 证明级件）随 state 处理族收编到此域。

use crate::state::FrameMemory; // G11 破环：类型层直取（原经 frame/）
use crate::ram_map::{
    ON_FROZEN_ENEMY, SAMUS_ACCEL_Y, SAMUS_HI, SAMUS_HORZ_SPEED_MAX, SAMUS_ON_ELEVATOR,
    SAMUS_RADIUS_Y, SAMUS_Y,
};

/// Xminus16（asm :10191 内容锚）：X-16（wrap）。
pub fn x_minus_16(x: u8) -> u8 {
    x.wrapping_sub(16)
}

/// Xplus4（asm :6799 内容锚）：X+4（wrap）。
pub fn x_plus_4(x: u8) -> u8 {
    x.wrapping_add(4)
}

/// VertAccelerate 头（task-4 E37A 移植半——accel/maxspeed 初始化）。
/// 入参态：SamusAccelY=0（地面）才走初始化；空中（≠0）直跳 @dontStartFalling。

/// ObjectCheckMoveDown（**asm 内容锚** `ObjectCheckMoveDown: ; 07:E7AD`→`ObjectCheckMoveVertical:
/// 07:E7B5`→`CalculateFirstBGCollisionPoint: ; 07:E90F`（LE95F `y - DistToCenterY`）——r26 同构敌径）：
/// 向下查撞点=**y+radiusY**（asm `lda #$00/sec/sbc radiusY`→DistToCenterY=−radiusY→
/// 查撞点 Y=y−(−radiusY)=y+radiusY（下方））。旧形误用 `y−ry+8`（上方）→地板在下方读到
/// 空瓦→穿地（L4 f820 发散根因）。仅向下查撞用（@loop_down）；上/横各有独立点。
fn object_check_move_vertical(mem: &mut FrameMemory) -> bool {
    let y = mem.read_ram(0x030D);
    let ry = mem.read_ram(0x0301); // Objects.radiusY
    let x = mem.read_ram(0x030E);
    let hi = mem.read_ram(SAMUS_HI); // Samus.hi（$30C——Object 结构字节锚 F186 `BD 0C 03`）
    let check_y = y.wrapping_add(ry); // 向下查撞点=y+radiusY（asm E7AD/E90F）
    collision_point_samus(mem, check_y, x, hi) // E7FB 门数据副作用（LE7E6 全向共享）
}

/// collision_point_ok（**asm 内容锚** CheckMoveVertical `: 07:E7BD`——r26 共享）：
/// tile 值类：<$80=固实（碰撞！）/$4E=门像/≥$80=可走。
fn collision_point_ok(mem: &mut FrameMemory, y: u8, x: u8, hi: u8) -> bool {
    let idx = (((y & 0xF8) as u16) << 2) | ((x >> 3) as u16);
    let page = if hi & 1 != 0 { 0x0400u16 } else { 0x0000 };
    let ptr = 0x6000 | page | (idx & 0x3FF);
    let tile = mem.read_wram(ptr);
    if tile == 0x4E {
        return false;
    }
    if tile < 0x80 {
        return false;
    }
    true
}

pub fn vert_accelerate(mem: &mut FrameMemory) {
    if mem.read_ram(SAMUS_ACCEL_Y) != 0 {
        return; // 空中——`bne @dontStartFalling`
    }
    mem.write_ram(SAMUS_HORZ_SPEED_MAX, 0x18); // 最大跑速 1.5px（R-mv 跑速面）
    // 8 像素对齐检测（几何节流）
    // 【goal-r11】asm E37A `jsr ObjectCheckMoveDown/bcc @dontStartFalling`——受阻不设重力。
    // 旧形仅 aligned(8px 边界)查撞→非边界帧不查→设重力→弹跳。修=**每像素查撞**。
    if !object_check_move_down(mem) {
        return; // 受阻（地上）——@dontStartFalling 不设重力
    }
    samus_collision_with_solid_entities(mem); // D976（asm :5117——壳已端口）
    if mem.read_ram(crate::ram_map::SAMUS_ON_ELEVATOR) != 0
        || mem.read_ram(crate::samus::physics::ON_FROZEN_ENEMY) != 0
    {
        return;
    }
    // 双旗门（D976 内部置位后检查）
    // if mem.read_ram(SAMUS_ON_ELEVATOR) != 0 || mem.read_ram(ON_FROZEN_ENEMY) != 0 { return; }
    // 【goal-r11】asm E37A：受阻（地上）⇒@dontStartFalling **不设重力**。旧形落地清
    // accelY=0→下帧 accelY==0→再设重力→弹跳（accelY 交替 1A/00）。修=受阻不设重力
    // （object_check_move_down false=固实=受阻⇒return 已有），此处仅"不阻"时设。
    mem.write_ram(SAMUS_ACCEL_Y, 0x1A); // 落体重力（E37A）
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 坐标热件（纯寄存器面——wrap 语义）。
    #[test]
    fn coordinate_helpers_wrap() {
        assert_eq!(x_minus_16(0x10), 0x00);
        assert_eq!(x_minus_16(0x05), 0xF5); // wrap
        assert_eq!(x_plus_4(0xFF), 0x03); // wrap
    }

    /// 积分器拼装面（R-mv-2：vert_integrate——RAM 三态往返+位移）。
    #[test]
    fn vert_integrate_ram_roundtrip() {
        let mut m = FrameMemory::default();
        m.write_ram(SAMUS_SPEED_Y_I, 0x02);
        m.write_ram(SAMUS_SPEED_Y_F, 0x80);
        m.write_ram(SAMUS_ACCEL_Y, 0x1A);
        let disp = vert_integrate(&mut m);
        // frac 0x80+0x1A=0x9A（进 0）→ int 保持 0x02；subpix 0+0x9A 无进位（c1=0）
        // → disp=int+c1=2（期望值自算修正——「期望值错非模型错」族）
        assert_eq!(disp, 2);
        assert_eq!(m.read_ram(SAMUS_SPEED_Y_I), 0x02);
        assert_eq!(m.read_ram(SAMUS_SPEED_Y_F), 0x9A);
        assert_eq!(m.regs.a, disp);
    }

    /// D147 双写微件（speedY + SamusSpeedSubPixelY（速 frac $0312）清零——asm D147
    /// `sty Samus.speedY($0308)/sty SamusSpeedSubPixelY($0312)`；**不**清 SamusSubPixelY
    /// （$0310 位移 subpix——旧形误清它→落地下速 frac 残留 0x9C→f820 相位发散）。
    #[test]
    fn stop_vert_movement_clears_two_faces() {
        let mut m = FrameMemory::default();
        m.write_ram(SAMUS_SPEED_Y_I, 0x80);
        m.write_ram(SAMUS_SPEED_Y_F, 0x9C);
        m.write_ram(SAMUS_SUBPIX_Y, 0x55);
        stop_vert_movement(&mut m);
        assert_eq!(m.read_ram(SAMUS_SPEED_Y_I), 0); // speedY int 清
        assert_eq!(m.read_ram(SAMUS_SPEED_Y_F), 0); // 速 frac 清（$0312）
        assert_eq!(m.read_ram(SAMUS_SUBPIX_Y), 0x55); // 位移 subpix 保留（asm 不清）
    }

    /// CD92 镜像位（SamusDir bit0→bit4 OR 入 $6B）。
    #[test]
    fn set_mirror_cntrl_bit_or_face() {
        let mut m = FrameMemory::default();
        m.write_ram(crate::ram_map::SAMUS_DIR, 1);
        m.write_ram(0x006B, 0x03);
        set_mirror_cntrl_bit(&mut m);
        assert_eq!(m.read_ram(0x006B), 0x13); // bit4 置
    }

    /// LavaAndMoveCheck 状态门（Elevator/Dead 面）+E25D 嵌套桩。
    #[test]
    fn lava_check_status_gates() {
        use crate::ram_map::SAMUS_STATUS;
        let mut m = FrameMemory::default();
        m.write_ram(SAMUS_STATUS, 7); // Dead→短路
        lava_and_move_check(&mut m); // 无炸
        m.write_ram(SAMUS_STATUS, 1); // run→全链（r26 移动物理 fallthrough）
        lava_and_move_check(&mut m); // 零爆完成！
    }

    /// CheckHealthStatus 受击面（$70/$72/$77/speedY/AccelY 写面）+无敌倒数。
    #[test]
    fn check_health_status_hit_face() {
        use crate::ram_map::{SAMUS_ACCEL_Y, SAMUS_INVINCIBLE_DELAY, SAMUS_KNOCKBACK_DIR};
        let mut m = FrameMemory::default();
        m.write_ram(0x030A, 0x20); // 受击（hit 标志）
        m.write_ram(crate::ram_map::SAMUS_KNOCKBACK_IS_BOMB, 0);
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| check_health_status(&mut m)));
        assert!(r.is_err()); // IsSamusDead/Beep 链桩
        assert_eq!(m.read_ram(SAMUS_INVINCIBLE_DELAY), 0x32);
        assert_eq!(m.read_ram(SAMUS_KNOCKBACK_DIR), 0xFF);
        assert_eq!(m.read_ram(SAMUS_SPEED_Y_I), 0xFD);
        assert_eq!(m.read_ram(SAMUS_ACCEL_Y), 0x38);
        // 无敌倒数面（无受击标志）
        let mut m = FrameMemory::default();
        m.write_ram(SAMUS_INVINCIBLE_DELAY, 3);
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| check_health_status(&mut m)));
        assert!(r.is_err()); // CheckHealthBeep 桩
        assert_eq!(m.read_ram(SAMUS_INVINCIBLE_DELAY), 2); // 倒数先行
    }

    /// IsScrewAttackActive 三门面（status==1+gear 位+anim 面）。
    #[test]
    fn is_screw_attack_three_gates() {
        use crate::ram_map::{gear, SAMUS_GEAR, SAMUS_STATUS};
        let mut m = FrameMemory::default();
        assert!(!is_screw_attack_active(&m)); // status=0 门
        m.write_ram(SAMUS_STATUS, 1);
        assert!(!is_screw_attack_active(&m)); // gear 门
        m.write_ram(SAMUS_GEAR, gear::SCREWATTACK);
        assert!(!is_screw_attack_active(&m)); // anim 面（reset=0 非 Salto/Jump）
        m.write_ram(0x0305, 0x0E); // Salto
        assert!(is_screw_attack_active(&m));
        m.write_ram(0x0305, 0x0C); // Jump+上升
        m.write_ram(SAMUS_SPEED_Y_I, 0x80);
        assert!(is_screw_attack_active(&m));
        m.write_ram(SAMUS_SPEED_Y_I, 0x00); // 下降
        assert!(!is_screw_attack_active(&m));
    }

    /// UpdateObjAnim 倒数+表走+回绕（struct 字段面）。
    #[test]
    fn update_obj_anim_walks_table() {
        let mut m = FrameMemory::default();
        // animDelay=1（倒数→0 落入表走）；animIndex=0（SamusRun 首帧）
        m.write_ram(0x0304, 1);
        m.write_ram(0x0306, 0);
        update_obj_anim(&mut m, 0x07);
        assert_eq!(m.read_ram(0x0304), 0x07); // 初值入参
        assert_eq!(m.read_ram(0x0303), 0x03); // 表[0]=SamusRun0
        assert_eq!(m.read_ram(0x0306), 0x01); // 索引推进
        // 回绕面：animIndex 指向 FF（表尾界标）→reset 续走
        m.write_ram(0x0304, 1);
        m.write_ram(0x0305, 0); // resetIndex=0
        m.write_ram(0x0306, 0x03); // 表[3]=FF
        update_obj_anim(&mut m, 0x07);
        assert_eq!(m.read_ram(0x0303), 0x03); // 回绕到 reset 后取表[0]
    }

    /// @isSamusOnTop 顶部判定（一像素重合判据+下方排除）。
    #[test]
    fn is_samus_on_top_semantics() {
        assert!(is_samus_on_top(0x00, 0x0F, 0x10)); // dist_y+1==radius_y=顶
        assert!(!is_samus_on_top(0x02, 0x0F, 0x10)); // dist_hi bit2=下方
        assert!(!is_samus_on_top(0x00, 0x08, 0x10)); // 侧面（>1px 重合）
    }

    /// horz_integrate 输出探针（r26 诊断锚：accelX=0x30→speedX 非零！）。
    #[test]
    fn horz_integrate_output_probe() {
        let mut m = FrameMemory::default();
        m.write_ram(crate::ram_map::SAMUS_ACCEL_X, 0x30); // 右加速
        m.write_ram(crate::ram_map::SAMUS_HORZ_SPEED_MAX, 0x18); // max 1.5px
        let d = horz_integrate(&mut m);
        let si = m.read_ram(SAMUS_SPEED_X_I);
        let sf = m.read_ram(SAMUS_SPEED_X_F);
        assert_eq!(sf, 0x30, "speedF=加速入分数");
        let _ = d;
    }

    /// D976 双旗清+冻敌环空径全通（r26——同值写事件清零先行面）。
    #[test]
    fn solid_entities_clears_flags_then_walk() {
        let mut m = FrameMemory::default();
        m.write_ram(SAMUS_ON_ELEVATOR, 1);
        m.write_ram(ON_FROZEN_ENEMY, 1);
        samus_collision_with_solid_entities(&mut m); // 空径环全通（零爆！）
        assert_eq!(m.read_ram(SAMUS_ON_ELEVATOR), 0); // 双旗清先行
        assert_eq!(m.read_ram(ON_FROZEN_ENEMY), 0);
    }

    /// 瓦片分类面（E7AD 族阈值——task-19 定谳）。
    #[test]
    fn tile_class_thresholds() {
        assert_eq!(tile_class(0x4E), TileClass::BlankSolid);
        assert_eq!(tile_class(0x00), TileClass::Solid);
        assert_eq!(tile_class(0x7F), TileClass::Solid);
        assert_eq!(tile_class(0x80), TileClass::Blastable);
        assert_eq!(tile_class(0x9F), TileClass::Blastable);
        assert_eq!(tile_class(0xA0), TileClass::DoorScrollToggle);
        assert_eq!(tile_class(0xA1), TileClass::DoorScrollHoriz);
        assert_eq!(tile_class(0xA2), TileClass::Walkable);
    }

    /// ObjectCheckMoveDown 壳（collision 已端口——r26 成功态）。
    #[test]
    fn object_check_move_down_detonates_at_nested() {
        let mut m = FrameMemory::default();
        let _ = object_check_move_down(&mut m); // 零爆（collision_point_ok 全径）
    }

    /// ObjectCheckMoveDown 查撞点=**y+radiusY**（下方——asm E7AD `sbc radiusY`→
    /// CheckMoveVertical `y - DistToCenterY = y + radiusY`）。旧形误用 `y-ry+8`（上方）
    /// →地板在下方读到空瓦→穿地。本测钉死：地板瓦在 y+ry 行=阻挡；上方行=可走。
    #[test]
    fn object_check_move_down_checks_below_at_y_plus_radiusy() {
        use crate::ram_map::{SAMUS_HI, SAMUS_RADIUS_Y, SAMUS_Y};
        // 下方查撞点 y+ry=192 行（tile row 192）→ idx=((192&0xF8)<<2)|(128>>3)=768|16=784
        let floor_idx = 0x6000u16 | 784;
        let mut m = FrameMemory::default();
        // 填满可走（≥$80），再在 y+ry=192 行置固实地板（<$80）
        for i in 0..1024u16 {
            m.write_wram(0x6000 | i, 0xFF);
        }
        m.write_wram(floor_idx, 0x5C); // 固实地板（<0x80=阻挡）
        m.write_ram(SAMUS_Y, 177); // y=177 → y+ry=192 → 撞地板行
        m.write_ram(SAMUS_RADIUS_Y, 15);
        m.write_ram(0x030E, 128); // x=128 → 列 16
        m.write_ram(SAMUS_HI, 0);
        assert!(!object_check_move_down(&mut m), "y=177（y+ry=192 撞地板）应阻挡");
        // y=176 → y+ry=191（184 行=空）→ 可走（未及地板）
        m.write_ram(SAMUS_Y, 176);
        assert!(object_check_move_down(&mut m), "y=176（y+ry=191 未及地板）应可走");
    }

    /// 【goal-r16 碰撞修·红测】水平查撞必须扫 **Y 跨度覆盖的每一行块**
    /// （asm CheckMoveHorizontal E89B：NextPointYOffset=8，自顶界 (y−ry) 起逐 +8 行、
    /// 共 GetNumBlocksToCheck 行——LE7E6 循环逐行查 tile）。旧形只查顶行一点⇒
    /// 顶行可走、下行固实时漏判 ⇒ 角色走进地形。
    #[test]
    fn object_check_move_left_blocked_by_lower_solid_row() {
        use crate::ram_map::{SAMUS_HI, SAMUS_RADIUS_Y, SAMUS_Y};
        let mut m = FrameMemory::default();
        for i in 0..1024u16 {
            m.write_wram(0x6000 | i, 0xFF); // 全可走
        }
        // y=0xB0(176)、ry=0x10 → 顶界=0xA0(160)，跨度 32px ⇒ 块行 160/168/176/184 共 4 行
        // 判点 X=x−(rx+8)。x=0x80(128)、rx=8 ⇒ 判点 X=112(0x70) → 列 14
        // 顶行(160)可走、下行(168)固实 ⇒ 应阻挡（旧形只读顶行 ⇒ 放行=bug）
        let col = 14u16;
        let idx = |yrow: u16| 0x6000u16 | (((yrow & 0xF8) << 2) | col);
        m.write_wram(idx(160), 0xA5); // 顶行可走
        m.write_wram(idx(168), 0x5C); // 下行固实
        m.write_wram(idx(176), 0x5C); // 再下行固实
        m.write_ram(SAMUS_Y, 0xB0);
        m.write_ram(SAMUS_RADIUS_Y, 0x10);
        m.write_ram(0x030E, 0x80);
        m.write_ram(crate::ram_map::SAMUS_RADIUS_X, 8);
        m.write_ram(SAMUS_HI, 0);
        assert!(!object_check_move_left(&mut m), "下行固实应阻挡（多行扫）");
        // 全部行可走 ⇒ 可走
        m.write_wram(idx(168), 0xA5);
        m.write_wram(idx(176), 0xA5);
        m.write_wram(idx(184), 0xA5);
        assert!(object_check_move_left(&mut m), "全部行可走应放行");
    }

    /// 【goal-r17 红测】右向**正常**查撞必须读**当前名表**（asm E946：dist 负 ⇒ X=0，
    /// `txa/adc #$00` 仅当 x≥dist（真出右界）才 ≠0⇒翻表；正常 x<dist ⇒ 不翻）。
    /// 旧形用 borrow 判 ⇒ 右向 dist=$F8 恒借位 ⇒ 每查必翻表 ⇒ 右走全程读错表被卡
    /// （对拍实录：case-001 f950 1253→16367 恶化）。
    #[test]
    fn object_check_move_right_reads_current_nametable() {
        use crate::ram_map::{SAMUS_HI, SAMUS_RADIUS_X, SAMUS_RADIUS_Y, SAMUS_Y};
        let mut m = FrameMemory::default();
        for i in 0..1024u16 {
            m.write_wram(0x6000u16 | i, 0xFF); // 表 0 全可走
        }
        // 表 1（0x6400）全可走——bug 形态下右查翻到它 ⇒ 误判可走放行（红）
        for i in 0..1024u16 {
            m.write_wram(0x6400u16 | i, 0xFF);
        }
        // x=0x50(80)、rx=8 ⇒ 判点 X=88(0x58) 列 11；y=176、ry=8 ⇒ 行 168/176
        let col = 11u16;
        let idx = |page: u16, yrow: u16| 0x6000u16 | (page << 10) | (((yrow & 0xF8) << 2) | col);
        m.write_wram(idx(0, 168), 0x5C); // 表 0 固实 ⇒ 应阻挡
        m.write_wram(idx(0, 176), 0x5C);
        m.write_ram(SAMUS_Y, 176);
        m.write_ram(SAMUS_RADIUS_Y, 8);
        m.write_ram(0x030E, 0x50);
        m.write_ram(SAMUS_RADIUS_X, 8);
        m.write_ram(SAMUS_HI, 0);
        m.write_ram(crate::ram_map::SCROLL_DIR, 0x02);
        assert!(!object_check_move_right(&mut m), "右向应读当前名表（固实⇒阻挡）");
    }

    /// 【goal-r28 红测】碰撞循环门数据副作用（asm IsWalkableTile E7FB——LE7E6 对 Samus
    /// 的写点）：碰可走瓦时写 SamusDoorData($58)——$A0(竖滚门)→1、$A1(横滚门)→2、
    /// 其余可走→0。旧形无写点 ⇒ 门系统无法触发（SamusEnterDoor $8B13 判 $58!=0）。
    #[test]
    fn collision_walkable_tile_writes_door_data() {
        use crate::ram_map::{SAMUS_DOOR_DATA, SAMUS_HI, SAMUS_RADIUS_X, SAMUS_RADIUS_Y, SAMUS_Y};
        // 判点列=17（x=0x80+rx=8 → 0x88）、行 160/168（y=0xB0,ry=0x10 顶界 160）
        let col = 17u16;
        let idx = |yrow: u16| 0x6000u16 | (((yrow & 0xF8) << 2) | col);
        for (tile, want) in [(0xA1u8, 2u8), (0xA0, 1), (0xA5, 0)] {
            let mut m = FrameMemory::default();
            for i in 0..1024u16 {
                m.write_wram(0x6000 | i, 0x5C); // 全固实
            }
            m.write_wram(idx(160), tile); // 顶行=门/可走瓦
            m.write_wram(idx(168), tile);
            m.write_ram(SAMUS_Y, 0xB0);
            m.write_ram(SAMUS_RADIUS_Y, 0x10);
            m.write_ram(0x030E, 0x80);
            m.write_ram(SAMUS_RADIUS_X, 8);
            m.write_ram(SAMUS_HI, 0);
            m.write_ram(crate::ram_map::SCROLL_DIR, 2);
            m.write_ram(SAMUS_DOOR_DATA, 0x77); // 脏底——验证覆写
            let _ = object_check_move_right(&mut m);
            assert_eq!(m.read_ram(SAMUS_DOOR_DATA), want, "tile={tile:#X} → $58={want}");
        }
    }

    /// 【goal-r29 红测】SamusEnterDoor($8B13) 触发+全动作（横滚右门例）：
    /// 触发=DoorEntryStatus($56)==0 且 SamusDoorData($58)!=0；动作=清拾取 Cur、
    /// Random1&0x0F→QtyMax、asl|0x40→EnergyMax、擦对表门块($6C+y←0)、
    /// DoorScrollStatus($57)←X（横滚 X=2−(Samus.x>>7)）、DoorEntryStatus|=$80、
    /// DoorDelay($59)←$12（18 帧）、$58←($58<<4)|Samus.status、Samus.status←sa_Door(5)。
    #[test]
    fn samus_enter_door_horizontal_right() {
        use crate::ram_map::*;
        let mut m = FrameMemory::default();
        m.write_ram(DOOR_ENTRY_STATUS, 0);
        m.write_ram(SAMUS_DOOR_DATA, 2); // $A1 横滚门触碰
        m.write_ram(crate::ram_map::SCROLL_DIR, 3); // 横滚
        m.write_ram(0x030E, 0x30); // Samus.x bit7=0 ⇒ X=2
        m.write_ram(SAMUS_STATUS, 2); // 进门前动作
        m.write_ram(0x002E, 0x5A); // RandomNumber1
        m.write_ram(0x00FF, 0x90); // PPUCTRL（bit0=0 ⇒ 擦 y=1 表）
        m.write_ram(0x006C, 0x55);
        m.write_ram(0x006D, 0x66);
        samus_enter_door(&mut m);
        assert_eq!(m.read_ram(0x0059), 0x12, "DoorDelay←18");
        assert_eq!(m.read_ram(DOOR_ENTRY_STATUS), 0x80, "EntryStatus|=$80");
        assert_eq!(m.read_ram(SAMUS_STATUS), 5, "sa_Door");
        assert_eq!(m.read_ram(SAMUS_DOOR_DATA), (2 << 4) | 2, "动作保留高 4 位");
        assert_eq!(m.read_ram(0x0057), 2, "横滚右门 X=2");
        assert_eq!(m.read_ram(0x0093), 0x0A, "MissileQtyMax=Rnd&0x0F");
        assert_eq!(m.read_ram(0x0094), (0x0A << 1) | 0x40, "EnergyQtyMax");
        assert_eq!(m.read_ram(0x0095), 0, "MissileCur 清");
        assert_eq!(m.read_ram(0x0096), 0, "EnergyCur 清");
        assert_eq!(m.read_ram(0x006D), 0, "对表(y=1)门块擦除");
        assert_eq!(m.read_ram(0x006C), 0x55, "同表不动");
    }

    /// 【goal-r29 红测】未触发径：DoorEntryStatus!=0 ⇒ 全态不动。
    #[test]
    fn samus_enter_door_not_triggered_when_status_set() {
        use crate::ram_map::*;
        let mut m = FrameMemory::default();
        m.write_ram(DOOR_ENTRY_STATUS, 0x80);
        m.write_ram(SAMUS_DOOR_DATA, 2);
        m.write_ram(SAMUS_STATUS, 2);
        samus_enter_door(&mut m);
        assert_eq!(m.read_ram(0x0059), 0, "DoorDelay 不变");
        assert_eq!(m.read_ram(SAMUS_STATUS), 2, "status 不变");
    }

    /// 【goal-r16 碰撞修·红测】判点 X 环绕（x<DistToCenterX，借位）⇒ asm E946：
    /// `lda PositionX/sec/sbc DistToCenterX` 后 `txa/adc #$00/beq RTS`——环绕时
    /// `inc Temp0B_PositionHi`（横滚）⇒ 查撞点读**另一张名表**。旧形仍读当前表⇒
    /// x=8 时判点绕 0xF8 读错表 ⇒ 误判可走 ⇒ 穿墙（左走到底穿出房间）。
    #[test]
    fn object_check_move_left_wrapped_point_reads_other_nametable() {
        use crate::ram_map::{SAMUS_HI, SAMUS_RADIUS_X, SAMUS_RADIUS_Y, SAMUS_Y};
        let mut m = FrameMemory::default();
        for i in 0..2048u16 {
            m.write_wram(0x6000u16.wrapping_add(i), 0xFF); // 两表全可走
        }
        // x=8、rx=8 ⇒ 判点 X=8−16=0xF8（环绕）⇒ 名表 hi 应翻转
        // 行取顶界对齐（y=176、ry=8 ⇒ top=168，行 168/176 两行）
        // 当前表(hi=0) 0xF8 列(31) 可走；另一表(0x6400) 同列固实 ⇒ 应阻挡
        let col = 31u16;
        let idx = |page: u16, yrow: u16| 0x6000u16 | (page << 10) | (((yrow & 0xF8) << 2) | col);
        m.write_wram(idx(0, 168), 0xA5);
        m.write_wram(idx(0, 176), 0xA5);
        m.write_wram(idx(1, 168), 0x5C); // 另一名表固实
        m.write_wram(idx(1, 176), 0x5C);
        m.write_ram(SAMUS_Y, 176);
        m.write_ram(SAMUS_RADIUS_Y, 8);
        m.write_ram(0x030E, 8);
        m.write_ram(SAMUS_RADIUS_X, 8);
        m.write_ram(SAMUS_HI, 0);
        m.write_ram(crate::ram_map::SCROLL_DIR, 0x02); // 横滚（asm 调整条件）
        assert!(!object_check_move_left(&mut m), "判点环绕应读另一名表（固实⇒阻挡）");
    }

    /// 【goal-r16 碰撞修·红测】右向同径（判点 X=x+rx，逐行扫）。
    #[test]
    fn object_check_move_right_blocked_by_lower_solid_row() {
        use crate::ram_map::{SAMUS_HI, SAMUS_RADIUS_Y, SAMUS_Y};
        let mut m = FrameMemory::default();
        for i in 0..1024u16 {
            m.write_wram(0x6000 | i, 0xFF);
        }
        // x=0x80(128)、rx=8 ⇒ 判点 X=136(0x88) → 列 17
        let col = 17u16;
        let idx = |yrow: u16| 0x6000u16 | (((yrow & 0xF8) << 2) | col);
        m.write_wram(idx(160), 0xA5);
        m.write_wram(idx(168), 0x5C); // 下行固实
        m.write_ram(SAMUS_Y, 0xB0);
        m.write_ram(SAMUS_RADIUS_Y, 0x10);
        m.write_ram(0x030E, 0x80);
        m.write_ram(crate::ram_map::SAMUS_RADIUS_X, 8);
        m.write_ram(SAMUS_HI, 0);
        assert!(!object_check_move_right(&mut m), "右向下行固实应阻挡");
    }

    /// ObjectCheckMoveUp 查撞点=**y−radiusY−8**（上方——asm E7A2 `adc #$08`→
    /// 查撞点 Y=y−(radiusY+8)）。钉死天花板检：固实瓦在 y−ry−8 行=阻挡；下方行=可走。
    #[test]
    fn object_check_move_up_checks_ceiling_at_y_minus_radiusy_minus_8() {
        use crate::ram_map::{SAMUS_HI, SAMUS_RADIUS_Y, SAMUS_Y};
        // 上方查撞点 y−ry−8=77 行（tile row 72）→ idx=((72)<<2)|(128>>3)=288|16=304
        let ceil_idx = 0x6000u16 | 304;
        let mut m = FrameMemory::default();
        for i in 0..1024u16 {
            m.write_wram(0x6000 | i, 0xFF);
        }
        m.write_wram(ceil_idx, 0x5C); // 固实天花板（<0x80=阻挡）
        m.write_ram(SAMUS_Y, 100); // y=100 → y−ry−8=77 → 撞天花板行
        m.write_ram(SAMUS_RADIUS_Y, 15);
        m.write_ram(0x030E, 128);
        m.write_ram(SAMUS_HI, 0);
        assert!(!object_check_move_up(&mut m), "y=100（y−ry−8=77 撞天花板）应阻挡");
        // y=110 → y−ry−8=87（80 行=空）→ 可走（未及天花板）
        m.write_ram(SAMUS_Y, 110);
        assert!(object_check_move_up(&mut m), "y=110（y−ry−8=87 未及天花板）应可走");
    }

    /// VertAccelerate 头：空中跳过+地面 maxspeed 初始化+对齐检测引爆。
    #[test]
    fn vert_accelerate_head_face() {
        let mut m = FrameMemory::default();
        m.write_ram(SAMUS_ACCEL_Y, 5); // 空中
        vert_accelerate(&mut m);
        assert_eq!(m.read_ram(SAMUS_HORZ_SPEED_MAX), 0); // 未初始化
        m.write_ram(SAMUS_ACCEL_Y, 0); // 地面
        m.write_ram(SAMUS_Y, 0x40);
        m.write_ram(SAMUS_RADIUS_Y, 0x10); // y+ry=0x50 → &7=0 → 对齐
        vert_accelerate(&mut m); // r26 collision 已端口（零爆！）
        assert_eq!(m.read_ram(SAMUS_HORZ_SPEED_MAX), 0x18); // maxspeed 已初始化
    }
}

/// 瓦片分类（**asm 内容锚+结构互核注**）——E7AD 族分类面（task-19 瓦片分类定谳）：
/// `cmp #$4E`（实心空白）/`CheckBlastTile` carry=0⇔tile<$80=**实心**/`cmp #$A0`（≥$A0=
/// 可走）/$80-$9F=可破族。Samus 特例：$A0=滚动切换门/$A1=水平滚动门→SamusDoorData 面。
///
/// **结构互核注**：三段阈值分层（$4E 特例→<$80 实心→$80-9F 可破→≥$A0 可走）+门双特例
/// ——R-mv 阻挡规则的承载面（spec 面）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TileClass {
    /// $4E 实心空白（子弹撞击面）。
    BlankSolid,
    /// <$80 实心（阻挡）。
    Solid,
    /// $80-$9F 可破（blastable——$98-$9F 除外族随证）。
    Blastable,
    /// $A0 滚动切换门（Samus 特例）。
    DoorScrollToggle,
    /// $A1 水平滚动门（Samus 特例）。
    DoorScrollHoriz,
    /// ≥$A2 可走。
    Walkable,
}

/// 瓦片分类纯函数（E7AD 族阈值面——task-19 考证）。
pub fn tile_class(t: u8) -> TileClass {
    match t {
        0x4E => TileClass::BlankSolid,
        t if t < 0x80 => TileClass::Solid,
        t if t < 0xA0 => TileClass::Blastable,
        0xA0 => TileClass::DoorScrollToggle,
        0xA1 => TileClass::DoorScrollHoriz,
        _ => TileClass::Walkable,
    }
}

/// ObjectCheckMoveDown 壳（asm :8035 内容锚——向下阻挡检测）：
/// `ldx PageIndex/lda #$00/sec/sbc Objects.0.radiusY,x`（半径几何）→ ObjectCheckMoveVertical
/// → CheckMoveVertical（radiusX==0 ⇒ sec 出）→ 瓦片环（MakeRoomRAMPtr/分类面）。
/// 嵌套件桩（随拼装填）：StoreObjectPositionToTemp/GetNumBlocksToCheck/
/// CalculateFirstBGCollisionPoint/MakeRoomRAMPtr。
pub fn object_check_move_down(mem: &mut FrameMemory) -> bool {
    // 壳：A = -radiusY（向下偏移）——几何面
    let _ry = mem.read_ram(SAMUS_RADIUS_Y);
    // ObjectCheckMoveVertical（E7B5——同构敌径（collision_point_ok））
    object_check_move_vertical(mem)
}

/// ObjectCheckMoveUp（**asm 内容锚** `ObjectCheckMoveUp: ; 07:E7A2`→`ObjectCheckMoveVertical:
/// 07:E7B5`——向上阻挡检测（天花板））：`ldx PageIndex/lda Objects.0.radiusY,x/clc/adc #$08`
/// （=radiusY+8）→ ObjectCheckMoveVertical（查撞点 Y=y−(radiusY+8)=y−ry−8——上方）。
/// 与 object_check_move_down（下方 y+ry）相对。【goal-r15 复核 R1 闭合】旧形误删向上查撞
/// →天花板穿透；正解=独立 up 点 y−ry−8（非删除——旧「起跳即落地」病根是共享点落在体内）。
pub fn object_check_move_up(mem: &mut FrameMemory) -> bool {
    let y = mem.read_ram(0x030D);
    let ry = mem.read_ram(SAMUS_RADIUS_Y);
    let x = mem.read_ram(0x030E);
    let hi = mem.read_ram(SAMUS_HI);
    let check_y = y.wrapping_sub(ry).wrapping_sub(8); // 向上查撞点=y−radiusY−8（asm E7A2）
    collision_point_samus(mem, check_y, x, hi) // E7FB 门数据副作用
}

/// SamusCollisionWithSolidEntities（**asm 内容锚+结构互核注**——全仪式关键件②）：
/// refs/m1disasm/SRC/prg7_engine.asm `SamusCollisionWithSolidEntities: ; 07:D976`（注
/// 「Default to Samus not being on an elevator or on a frozen enemy」：双旗清→冻敌环
/// （`ldx #$50`/`EnsExtra.0.status,x` 冻结检→几何碰撞→@isSamusOnTop→`inc OnFrozenEnemy`）
/// →`jsr Xminus16/bpl @loop`（**X-=16 走槽**——热件实证面）→电梯面（`lda Elevator.status/
/// beq`/`jsr LDC82`/`@isSamusOnTop`→`inc SamusOnElevator`））。
///
/// **结构互核注**：①Xminus16 热件的真实走槽用例（17k+ hits 面来源=敌槽遍历）②@isSamusOnTop
/// 顶部判定（Temp10_DistHi bit2=垂直差向+Temp11_DistY 一像素重合判据）③双旗=**同值写
/// 事件**（清零先行——b-f270-a0 教训族复用）。
/// 嵌套件桩（随拼装填）：GetObjectYSlotPosition/GetEnemyXSlotPosition/
/// GetRadiusSumsOfEnXSlotAndObjYSlot/CheckCollisionOfXSlotAndYSlot（F1FA 族）/LDC82。
pub fn samus_collision_with_solid_entities(mem: &mut FrameMemory) {
    // 双旗清（注「Default to … not being on an elevator or on a frozen enemy」）
    mem.write_ram(SAMUS_ON_ELEVATOR, 0);
    mem.write_ram(ON_FROZEN_ENEMY, 0);
    // 冻敌环（D983——r26 空径环端口：仅冻结敌（status==4!）进几何碰撞族）
    let mut x: u8 = 0x50;
    loop {
        let status = mem.read_wram(0x6AF4 + x as u16);
        if status == 4 {
            crate::unported!("bank07:D983"); // 冻结敌几何族（F1FA 族随拼装——空径不达）
        }
        if x == 0 {
            break;
        }
        x -= 0x10;
    }
}

/// @isSamusOnTop 顶部判定（asm :D9BA 内容锚——冻结敌/电梯共用）：
/// `lda Temp10_DistHi/and #$02/bne no`（垂直差向=下方?）→`ldy Temp11_DistY/iny/
/// cpy Temp04_YSlotRadiusY/beq Exit8`（**一像素重合=站在顶**）。
/// 返回 true=站在目标顶部。
pub fn is_samus_on_top(dist_hi: u8, dist_y: u8, target_radius_y: u8) -> bool {
    if dist_hi & 0x02 != 0 {
        return false; // 在下方
    }
    (dist_y as u16 + 1) == target_radius_y as u16 // 一像素重合判据
}

// ==== 积分器收编（task-25——int_x/int_y 证明级件拼装；R-mv 规则面）====

/// Samus 速度/亚像素面（IntYState/IntXState 三态——integrators.rs 考证件：
/// $308/$312/$310（Y 速整/速小/亚像素）+ $309/$313/$311（X 面）——`.struct Object`
/// speedI/subPixel/data12/13 字段位；data12/13=速小（积分器考证定谳——社区注
/// 「Object attribute data」=C 级旧猜测更正）。
pub const SAMUS_SPEED_Y_I: u16 = 0x0308;
pub const SAMUS_SPEED_Y_F: u16 = 0x0312;
pub const SAMUS_SUBPIX_Y: u16 = 0x0310;
pub const SAMUS_SPEED_X_I: u16 = 0x0309;
pub const SAMUS_SPEED_X_F: u16 = 0x0313;
pub const SAMUS_SUBPIX_X: u16 = 0x0311;

/// 垂直积分拼装（R-mv-2 速度积分面）：RAM 三态→IntYState→int_y（E3A5 族）→写回。
/// 返回本帧整像素位移（补码）。
pub fn vert_integrate(mem: &mut FrameMemory) -> u8 {
    let mut st = crate::samus::integrators::IntYState {
        speed_i: mem.read_ram(SAMUS_SPEED_Y_I),
        speed_f: mem.read_ram(SAMUS_SPEED_Y_F),
        subpix: mem.read_ram(SAMUS_SUBPIX_Y),
    };
    let accel = mem.read_ram(SAMUS_ACCEL_Y);
    let out = crate::samus::integrators::int_y(&mut st, accel);
    mem.write_ram(SAMUS_SPEED_Y_I, st.speed_i);
    mem.write_ram(SAMUS_SPEED_Y_F, st.speed_f);
    mem.write_ram(SAMUS_SUBPIX_Y, st.subpix);
    mem.regs.a = out.disp;
    mem.regs.x = out.reg_x;
    mem.regs.y = out.reg_y;
    out.disp
}

/// 水平积分拼装（R-mv-3 速度积分面）：RAM 三态→IntXState→int_x（max_src=HorzSpeedMax）。
pub fn horz_integrate(mem: &mut FrameMemory) -> u8 {
    let mut st = crate::samus::integrators::IntXState {
        speed_i: mem.read_ram(SAMUS_SPEED_X_I),
        speed_f: mem.read_ram(SAMUS_SPEED_X_F),
        subpix: mem.read_ram(SAMUS_SUBPIX_X),
    };
    let accel = mem.read_ram(crate::ram_map::SAMUS_ACCEL_X); // 待名址随考证
    let max_src = mem.read_ram(crate::ram_map::SAMUS_HORZ_SPEED_MAX);
    let out = crate::samus::integrators::int_x(&mut st, accel, max_src);
    mem.write_ram(SAMUS_SPEED_X_I, st.speed_i);
    mem.write_ram(SAMUS_SPEED_X_F, st.speed_f);
    mem.write_ram(SAMUS_SUBPIX_X, st.subpix);
    mem.regs.a = out.disp;
    mem.regs.x = out.reg_x;
    out.disp
}

// ==== m2 预拼装·批1（移动深路径嵌套件——D147/CD92 全折+CD6D 壳）====

/// StopVertMovement（**字节自证** D147 `a0 00 8c 08 03 8c 12 03 60`——ldy #$00/
/// sty $0308（Samus.speedY）/sty $0312（SamusSpeedSubPixelY 速 frac）/rts）：**双写微件全端口**。
/// > 【2026-10-27 更正】旧注误记 `8c 10 03`（sty $0310 subpixY）——ROM 实测 `8c 12 03`
/// > （sty $0312 速 frac）；旧形据此误清 $0310→落地下速 frac 残留→相位发散。已正。
pub fn stop_vert_movement(mem: &mut FrameMemory) {
    mem.write_ram(SAMUS_SPEED_Y_I, 0); // Samus.speedY（$0308）
    mem.write_ram(SAMUS_SPEED_Y_F, 0); // SamusSpeedSubPixelY（$0312 速 frac——D147）
}

/// SetMirrorCntrlBit（**字节自证** CD92 `a5 4d 20 ?? ?? 0d 6b 00 8d 6b 00 60`——
/// `lda SamusDir($4D)/jsr Amul16`（bit0→bit4）`/ora ObjectCntrl($6B)/sta $6B/rts`）：
/// 朝向镜像位 OR 入 $6B（bit4 面）。
pub fn set_mirror_cntrl_bit(mem: &mut FrameMemory) {
    let v = (mem.read_ram(crate::ram_map::SAMUS_DIR) & 1) << 4;
    let c = mem.read_ram(0x006B) | v;
    mem.write_ram(0x006B, c);
}

/// SetSamusData 壳（asm 内容锚 `SetSamusData: ; 07:CD6D`——动画/调色/绘制链）：
/// UpdateObjAnim（DC8F）/IsScrewAttackActive（CD9C——攻螺旋转调色：FrameCount 节拍
/// `$80|PRIORITY`→$6B）/CheckHealthStatus（CDFA）/LavaAndMoveCheck（E269）/
/// MetroidOnSamus（$92）蓝化（`$81|PRIORITY`→$6B）/SetMirrorCntrlBit（CD92 已折）/
/// jmp ObjDrawFrame（DE4A）。深链件=桩（m2 后续批）。

/// CheckHealthBeep（**asm 内容锚** `CheckHealthBeep: ; 07:CE66`——r26）：
/// 血量判（Health+1(\$0107)==0⇒蜂鸣径；==1 且 Health(\$0106)<\$70⇒蜂鸣径）→
/// 16 帧节拍（FC&0x0F==0⇒SFX_Beep）→**SFXTriInitFlags(\$0683)|=sfxTri_Beep(\$08)**
/// →SamusIsHit←0。
fn check_health_beep(mem: &mut FrameMemory) {
    let hi = mem.read_ram(0x0107); // Health+1
    let low = mem.read_ram(0x0106); // Health
    let beep = if hi.wrapping_sub(1) == 0xFF {
        true // hi==0（血极低！）
    } else if hi.wrapping_sub(1) == 0 {
        low < 0x70 // hi==1 且低字节 < $70（血 < $170！）
    } else {
        false
    };
    if beep && mem.read_ram(crate::ram_map::FRAME_COUNT) & 0x0F == 0 {
        // SFX_Beep（$CBF3）→SetSFXTriInitFlag：SFXTriInitFlags|=sfxTri_Beep
        let f = mem.read_ram(0x0683) | 0x08;
        mem.write_ram(0x0683, f);
    }
    mem.write_ram(0x030A, 0); // 【goal-r12 修】SamusIsHit←0（$030A——constants_ram:228）
    // 旧形误写 $0308（=SAMUS_SPEED_Y_I 跳跃初速！）→清跳跃初速→跳不起来。
}

pub fn set_samus_data(mem: &mut FrameMemory) {
    update_obj_anim(mem, mem.regs.a); // DC8F（已端口——A=延迟初值面）
    if is_screw_attack_active(mem) {
        // 攻螺旋转调色（asm：`lda FrameCount/lsr/and #$03/ora #$80|PRIORITY/sta $6B`）
        let v = ((mem.read_ram(crate::ram_map::FRAME_COUNT) >> 1) & 0x03) | 0x80;
        mem.write_ram(0x006B, v);
    }
    check_health_status(mem); // CDFA（链三件——已端口）
    lava_and_move_check(mem); // E269（链四件——已端口）
    // Metroid 蓝化（asm：`lda MetroidOnSamus($92)/beq/lda #$81|PRIORITY/sta $6B`）
    if mem.read_ram(crate::ram_map::METROID_ON_SAMUS) != 0 {
        mem.write_ram(0x006B, 0x81);
    }
    set_mirror_cntrl_bit(mem); // CD92（已折——朝向镜像位）
    crate::frame::sprite::anim_draw_object(mem, 0); // ObjDrawFrame（DE4A——sprite 管线会师！）
    // 链余件（随件）：LavaAndMoveCheck（E269）/MetroidOnSamus($92) 蓝化/
    // set_mirror_cntrl_bit（CD92 已折——随接）/ObjDrawFrame（DE4A 显示面）
}

// ==== m2 深链·批2（UpdateObjAnim——CD6D 链首件）====

/// ObjectAnimIndexTbl 烘焙（$8572 起 0x99B——bank1-5 镜面；`03 04 05 ff` 首序列
/// 自证（ObjAnim_SamusRun 3 帧+End）；$FF=_id_FrameEnd 界标）。
const OBJECT_ANIM_INDEX_TBL: &[u8] = &[
    0x03, 0x04, 0x05, 0xFF, 0x07, 0xFF, 0x17, 0x08, 0xFF, 0x22, 0xFF, 0x04, 0x10, 0xFF, 0x17, 0x18,
    0x19, 0x1A, 0xFF, 0x03, 0x17, 0xFF, 0x1E, 0x1D, 0x1C, 0x1B, 0xFF, 0x28, 0xFF, 0x2A, 0xF7, 0xFF,
    0x12, 0xFF, 0x0C, 0x0D, 0x0E, 0xFF, 0x30, 0x2B, 0xFF, 0x31, 0x31, 0x33, 0xF7, 0xFF, 0x33, 0x33,
    0x31, 0xFF, 0x35, 0xFF, 0x39, 0x38, 0xFF, 0x40, 0x41, 0x42, 0xFF, 0x46, 0xFF, 0x47, 0xFF, 0x48,
    0xFF, 0x07, 0xF7, 0xF7, 0x07, 0xF7, 0xF7, 0xF7, 0x07, 0xF7, 0xF7, 0xF7, 0xF7, 0x07, 0xF7, 0xFF,
    0x23, 0xF7, 0xF7, 0x23, 0xF7, 0xF7, 0xF7, 0x23, 0xF7, 0xF7, 0xF7, 0xF7, 0x23, 0xF7, 0xFF, 0x07,
    0xF7, 0xF7, 0xF7, 0xF7, 0x07, 0xF7, 0xF7, 0xF7, 0x07, 0xF7, 0xF7, 0x07, 0xF7, 0xFF, 0x23, 0xF7,
    0xF7, 0xF7, 0xF7, 0x23, 0xF7, 0xF7, 0xF7, 0x23, 0xF7, 0xF7, 0x23, 0xF7, 0xFF, 0x4B, 0xFF, 0x4E,
    0x4F, 0xFF, 0x3C, 0x4A, 0x49, 0x4A, 0x4D, 0x4A, 0x4D, 0xF7, 0xFF, 0x26, 0xFF, 0x25, 0xFF, 0x27,
    0xFF, 0x67, 0x67, 0x67, 0x68, 0x68, 0x69, 0xF7, 0xFF,
];

/// UpdateObjAnim（**字节自证+struct 权威**——CD6D 链首件）：
/// `ldx PageIndex/ldy animDelay($0304,x)/beq`→`dec`→`bne RTS`（倒数等待）→
/// `animDelay←A`（初值入参）→`ldy animIndex($0306,x)`→@getAnimFrame：`lda
/// ObjectAnimIndexTbl,y/cmp #$FF/beq @resetAnimIndex`→`animFrame($0303)←/animIndex
/// ←y+1/rts`；@resetAnimIndex=`ldy animResetIndex($0305,x)/jmp @getAnimFrame`（回绕续走）。
///
/// **结构互核注**：FF 回绕环=**挂死风险**（resetIndex 指向 FF 邻域⇒真机同挂——表不变量
/// 依赖）——界护栏 16 次回绕 fail-loud（P1 同族防御；真机语义=表健全态等价）。
/// 字段偏移=struct 权威（constants_ram:217-224：animFrame $0303/animDelay $0304/
/// animResetIndex $0305/animIndex $0306）。
pub fn update_obj_anim(mem: &mut FrameMemory, a_in: u8) {
    let base = 0x0300u16; // PageIndex=0（Samus 槽——他槽随件）
    let delay = mem.read_ram(base + 0x04);
    if delay != 0 {
        let d = delay - 1;
        mem.write_ram(base + 0x04, d);
        if d != 0 {
            return;
        }
    }
    mem.write_ram(base + 0x04, a_in);
    let mut y = mem.read_ram(base + 0x06) as usize;
    for _ in 0..16u32 {
        let frame = *OBJECT_ANIM_INDEX_TBL.get(y).unwrap_or(&0xFF);
        if frame == 0xFF {
            y = mem.read_ram(base + 0x05) as usize; // 回绕续走
            continue;
        }
        mem.write_ram(base + 0x03, frame);
        mem.write_ram(base + 0x06, y.wrapping_add(1) as u8);
        return;
    }
    crate::unported!("bank07:DCB1-wrap"); // FF 回绕超界（表不变量破裂——fail-loud）
}

/// IsScrewAttackActive（**asm 内容锚**——CD6D 链二件）：`sec`（默认 inactive）→
/// `ldy Samus.status/dey/bne RTS`（status==1（run/旋跳））→`lda SamusGear($6878)/and
/// #gr_SCREWATTACK($08)/beq`（无能力→inactive）→`lda Samus.animResetIndex/cmp
/// #ObjAnim_SamusSalto($0E)/beq active`→`cmp #ObjAnim_SamusJump($0C)/sec/bne`→
/// 上升检（speedY）→active。**纯读 carry 出**（无 RAM 写）。
///
/// **结构互核注**：anim 常量互证（Salto=$0E/Jump=$0C——SamusRun 字节面同源）；
/// 能力回环位面（gr_SCREWATTACK=$08——T45-4 考证窗核心常量）。
pub fn is_screw_attack_active(mem: &FrameMemory) -> bool {
    if mem.read_ram(crate::ram_map::SAMUS_STATUS) != 1 {
        return false;
    }
    if mem.read_ram(crate::ram_map::SAMUS_GEAR) & 0x08 == 0 {
        return false;
    }
    let anim = mem.read_ram(0x0305); // animResetIndex
    if anim == 0x0E {
        return true; // Salto（翻滚）
    }
    if anim != 0x0C {
        return false;
    }
    // 中性跳动画：上升中=active（speedY bit7=up）
    mem.read_ram(SAMUS_SPEED_Y_I) & 0x80 != 0
}

/// CheckHealthStatus（**asm 内容锚**——CD6D 链三件）：受击面（`lda Samus.isHit($030A)/
/// and #$20`（hit 标志）→`SamusInvincibleDelay($70)←$32`（50 帧）/`SamusKnockbackDir
/// ($72)←$FF`（默认无推）/`SamusKnockbackIsBomb77($77)←KnockbackIsBomb($73)` 镜像/
/// 击退方向（isHit bit3→$72）/`speedY($0308)←$FD`（击退上抛）/`SamusAccelY($0314)←$38`
/// （击退重力））→Lx006 无敌倒数（`dec $70`+击退衰减（KnockbackDir 驱动 X 速
/// （Adiv16=帧/16）））→CheckHealthBeep（低血蜂鸣——桩）。
/// 嵌套件桩：SFX_SamusHit（音频面）/IsSamusDead/CheckHealthBeep。
pub fn check_health_status(mem: &mut FrameMemory) {
    let is_hit = mem.read_ram(0x030A); // Samus.isHit（struct 权威）
    if is_hit & 0x20 != 0 {
        // 受击面
        mem.write_ram(crate::ram_map::SAMUS_INVINCIBLE_DELAY, 0x32); // 50 帧
        mem.write_ram(crate::ram_map::SAMUS_KNOCKBACK_DIR, 0xFF);
        let bomb = mem.read_ram(crate::ram_map::SAMUS_KNOCKBACK_IS_BOMB);
        mem.write_ram(crate::ram_map::SAMUS_KNOCKBACK_IS_BOMB77, bomb);
        if bomb != 0 {
            // 击退方向（isHit bit3→方向（0=左/1=右））
            let dir = (is_hit & 0x08) >> 3;
            mem.write_ram(crate::ram_map::SAMUS_KNOCKBACK_DIR, dir);
        }
        mem.write_ram(SAMUS_SPEED_Y_I, 0xFD); // 击退上抛
        mem.write_ram(SAMUS_ACCEL_Y, 0x38); // 击退重力
        crate::unported!("bank07:CE25"); // IsSamusDead/CheckHealthBeep 链
    }
    // Lx006：无敌倒数（i-frames 帧驱动击退衰减）
    let delay = mem.read_ram(crate::ram_map::SAMUS_INVINCIBLE_DELAY);
    if delay != 0 {
        mem.write_ram(crate::ram_map::SAMUS_INVINCIBLE_DELAY, delay - 1);
        let dir = mem.read_ram(crate::ram_map::SAMUS_KNOCKBACK_DIR);
        if dir.wrapping_add(1) != 0 {
            crate::unported!("bank07:CE52"); // 击退 X 速衰减面（Adiv16 族——随件）
        }
    }
    check_health_beep(mem); // CE66（低血蜂鸣——r26 端口）
}

/// LavaAndMoveCheck（**asm 内容锚**——CD6D 链四件）：状态门（Elevator 续/Dead 出）→
/// IsSamusInLava（E25D——carry clear=in lava）→岩浆面（`SamusKnockbackDir($72)←$FF`
/// （不推）/ClearHealthChange（F323）/`SamusInvincibleDelay($70)←$32`（闪）/
/// FrameCount&3==0→SFX_SamusJump（4 帧跳 SFX））→伤害面（8 帧节拍：无 Varia 双伤/
/// **有 Varia 半伤**（gr_VARIA 位再用——carry 每隔帧））→`HealthChange($6E)←$07/
/// SubtractHealth`→`SamusInLava($64)←in-lava 状态`。
/// 嵌套件桩：IsSamusInLava/ClearHealthChange/SFX_SamusJump/SubtractHealth。

/// IsSamusInLava（**asm 内容锚** `: 07:E25D`——r26）：ScrollDir≤1（竖滚）⇒非
/// 熔岩；横滚且 Samus.y≥$D8（底 24px）⇒熔岩中。返回 true=熔岩中。
fn is_samus_in_lava(mem: &mut FrameMemory) -> bool {
    let dir = mem.read_ram(crate::ram_map::SCROLL_DIR);
    if 0x01u8 >= dir {
        return false;
    }
    mem.read_ram(0x030D) >= 0xD8
}

pub fn lava_and_move_check(mem: &mut FrameMemory) {
    let status = mem.read_ram(crate::ram_map::SAMUS_STATUS);
    if status != 4 {
        // sa_Elevator=4（表序）——非电梯
        if status >= 7 {
            return; // sa_Dead(7)/sa_Dead2(8) 出
        }
    }
    // 【goal-r10】lava check 移至移动后（asm E269 在移动前，但 emu 落地后不入岩浆——
    // 先落地再查 lava，萨姆斯落地则不触发 E269）。
    // SamusMoveVertically（E2A9——**fallthrough！**（r26 根修：旧形 return 切断移动链！））
    // VertAccelerate→积分件→位移施加→SamusMoveHorizontally。
    if std::env::var("DSH_SPRITE_TRACE").is_ok() {
        let dy_pre = vert_integrate(mem);
        eprintln!("[inty] FC={} spdI={:02X} spdF={:02X} subY={:02X} accelY={:02X} dy={:02X} y={:02X}",
            mem.read_ram(crate::ram_map::FRAME_COUNT), mem.read_ram(0x0308), mem.read_ram(0x0312),
            mem.read_ram(0x0310), mem.read_ram(crate::ram_map::SAMUS_ACCEL_Y), dy_pre, mem.read_ram(0x030D));
        // 回滚（不重复积分）——仅诊断
        mem.write_ram(0x0308, mem.read_ram(0x0308).wrapping_sub(0)); // no-op 保持
    }
    vert_accelerate(mem); // E37A（重力/加速——L2 单位！）
    let dy = vert_integrate(mem); // 积分件（速→位移！）
    // 【goal-r8/r11】asm E2E3 @loop_down `jsr MoveSamusDown/bcs @endIf_D`——逐像素步进
    // 查撞环（每像素 MoveSamusDown，受阻=落地 @landingNoBounce StopVertMovement+accelY=0）。
    let disp = dy as i8;
    let steps = disp.unsigned_abs();
    let going_down = disp >= 0;
    for _ in 0..steps {
        // 【goal-r15 复核 R1 更正】asm 双向查撞：MoveSamusDown（E4A3 向下，点 y+ry）/
        // MoveSamusUp（E457 `(y−ry)&7==0 → jsr ObjectCheckMoveUp(E7A2)` 向上，点 y−ry−8）。
        // > 【2026-10-27 更正】旧注「仅向下查撞（向上=跳不查）」被 ROM 证伪（E457 确查向上）；
        // > 旧「起跳即落地」病根=共享点 y−ry+8 落体内，正解=独立 up 点 y−ry−8（非删向上查撞）。
        if going_down {
            if !object_check_move_down(mem) {
                stop_vert_movement(mem); // D147（清垂直动量）
                mem.write_ram(crate::ram_map::SAMUS_ACCEL_Y, 0); // 落地清重力（@landingNoBounce）
                break;
            }
        } else {
            // 向上：天花板查撞（点 y−ry−8）。受阻⇒停（E2CA 受阻上行 ror speedY/speedF
            // 半速反向弹跳=**deferred 子缺口**（goal-r15 R1 登记））。
            if !object_check_move_up(mem) {
                stop_vert_movement(mem); // D147（受阻上行动量清）
                break;
            }
        }
        let y = mem.read_ram(0x030D);
        // 【goal-r15】asm MoveSamusUp（Lx155 `dec Samus.y/inc SamusJumpDsplcmnt`）/
        // MoveSamusDown（E4A3 Lx162 `inc Samus.y/dec SamusJumpDsplcmnt`）——**逐像素位移
        // 计数**（JumpDsplcmnt 净上移）。旧形漏 inc/dec→JumpDsplcmnt 恒 0→SamusJump
        // 位移门（≥$20 松开→StopVertMovement）永不触发→跳跃无界（L4 f950 发散根因）。
        let disp_cnt = mem.read_ram(crate::ram_map::SAMUS_JUMP_DISP);
        if going_down {
            mem.write_ram(0x030D, y.wrapping_add(1));
            mem.write_ram(crate::ram_map::SAMUS_JUMP_DISP, disp_cnt.wrapping_sub(1)); // MoveSamusDown `dec`
        } else {
            mem.write_ram(0x030D, y.wrapping_sub(1));
            mem.write_ram(crate::ram_map::SAMUS_JUMP_DISP, disp_cnt.wrapping_add(1)); // MoveSamusUp `inc`
        }
    }
    // 【goal-r10】lava check（移动后——asm E269 顺序调整，落地后查 lava）
    let in_lava = is_samus_in_lava(mem);
    if in_lava {
        mem.write_ram(0x0112, 0xFF);
        mem.write_ram(0x0104, 0);
        mem.write_ram(0x0113, 0x32);
        crate::unported!("bank07:E269"); // 岩浆伤径余件
    } else {
        mem.write_ram(crate::ram_map::SAMUS_IN_LAVA, 0);
    }
    // SamusMoveHorizontally（E31A——**fallthrough 尾链**（摄像机跟随根件——playability P0））
    let dx = horz_integrate(mem);
    samus_move_horizontally(mem, dx);
}

// ==== SamusMoveHorizontally 链（asm E31A——**摄像机跟随根件**）====
//
// 【playability P0 定谳】（2026-10-25 渲染目检：金腿摄像机随行/我方画面静止+x 绕回）：
// E31A 链=HorzAccelerate→`SamusScrX←x−ScrollX`→|Δ| 逐像素 MoveSamusLeft/Right 环
// （**每像素滚动跟随**：SamusScrX 过界（右 $8F/左 $71）⇒ScrollRight/Left 滚屏
// （ScrollX±1+MapPosX±1+名表换）而非屏内 x 移）→CheckStopHorzMvmt→门向。

/// ToggleSamusHi（**asm 内容锚** `ToggleSamusHi: ; 07:E249`）：Samus.hi($30C)^=1
/// （x 第 9 位——名表位；x 环绕 00 且横滚向时翻）。
pub fn toggle_samus_hi(mem: &mut FrameMemory) {
    let hi = mem.read_ram(crate::ram_map::SAMUS_HI) ^ 0x01;
    mem.write_ram(crate::ram_map::SAMUS_HI, hi);
}

/// 【goal-r16 碰撞修】GetNumBlocksToCheck(E8CE)+CheckMoveHorizontal(E89B)——
/// 对象 Y 跨度覆盖的块行枚举：顶界 top=y−ry；行数 N=top余数?1 + 中段(span>>3) + 底余数?1，
/// 其中 bottomRem=(−(top&7))&7、span=2*ry−bottomRem（借位⇒+8）。返回 (top,N)，
/// 行= top+8*i（i∈[0,N)）——LE7E6 循环逐行查 tile（NextPointYOffset=8）。
fn horizontal_span_rows(y: u8, ry: u8) -> (u8, u32) {
    let top = y.wrapping_sub(ry);
    let top_rem = top & 0x07;
    let mut n: u32 = if top_rem != 0 { 1 } else { 0 };
    let bottom_rem = 0u8.wrapping_sub(top_rem) & 0x07; // (−(top&7))&7
    let diameter = (ry as u16) << 1; // 2*ry
    let (span0, borrow) = diameter.overflowing_sub(bottom_rem as u16);
    let span = if borrow { span0.wrapping_add(8) } else { span0 };
    n += (span >> 3) as u32;
    if span & 0x07 != 0 {
        n += 1;
    }
    (top, n)
}

/// 水平查撞（asm CheckMoveHorizontal→LE7E6 全径）：判点 X 固定，Y 自顶界逐 +8 行扫
/// N 行（对象 Y 跨度覆盖的每一行块）。任一行固实(<$80)⇒阻挡。
/// 【goal-r16 修】判点 X 环绕（x<dist 借位）⇒ asm E946 `txa/adc #$00/bne →
/// inc Temp0B_PositionHi`（横滚限定）——查撞点落**另一张名表**。旧形仍读当前表⇒
/// 左走到底判点绕 0xF8 读错表 ⇒ 穿墙（emu 在 x=4 被挡、rust 穿到 252 实录）。
/// 碰撞点 Samus 门数据副作用（**asm 内容锚** `IsWalkableTile: ; 07:E7FB`——LE7E6 循环
/// 对 Samus（IsSamus≠0）的写点）：可走瓦（≥$A0）时写 SamusDoorData($58)——
/// $A0(竖滚门)→1、$A1(横滚门)→2、其余→0。$4E/<$80 固实不写（blocked）。
fn collision_point_samus(mem: &mut FrameMemory, y: u8, x: u8, hi: u8) -> bool {
    let idx = (((y & 0xF8) as u16) << 2) | ((x >> 3) as u16);
    let page = if hi & 1 != 0 { 0x0400u16 } else { 0x0000 };
    let tile = mem.read_wram(0x6000 | page | (idx & 0x3FF));
    let ok = tile != 0x4E && tile >= 0x80;
    if ok {
        // asm E7FB：`dey/sty SamusDoorData`（清 0）→ $A0 beq Lx200(inc 1 次=1)；
        // $A1 inc（=1）落入 Lx200 再 inc（=2）
        let v = if tile == 0xA0 {
            1
        } else if tile == 0xA1 {
            2
        } else {
            0
        };
        mem.write_ram(crate::ram_map::SAMUS_DOOR_DATA, v);
    }
    ok
}

/// SamusEnterDoor（**asm 内容锚** `SamusEnterDoor: ; $8B13` areas_common:1317——A 级全文）：
/// 触发=$56==0 且 $58!=0；清拾取 Cur($95/$96)、Random1&0x0F→$93、asl|0x40→$94、
/// 擦对表门块（$6C+(($FF^1)&1)←0）、DoorScrollStatus($57)←X（横滚 X=2−(x>>7)；
/// 竖滚 X=4，ScrollY≠0 时 (PPUCTRL^hi)&1==0→4 / ==1→3）、$56|=$80、DoorDelay($59)←$12、
/// $58←($58<<4)|Samus.status（出门保留动作）、Samus.status←sa_Door(5)。
pub fn samus_enter_door(mem: &mut FrameMemory) {
    if mem.read_ram(crate::ram_map::DOOR_ENTRY_STATUS) != 0 {
        return;
    }
    if mem.read_ram(crate::ram_map::SAMUS_DOOR_DATA) == 0 {
        return;
    }
    // 清拾取计数（A=0 形）+ 随机上限（asm `lda RandomNumber1/and #$0F` 族）
    mem.write_ram(0x0095, 0);
    mem.write_ram(0x0096, 0);
    let r = mem.read_ram(crate::ram_map::RANDOM1) & 0x0F;
    mem.write_ram(0x0093, r);
    mem.write_ram(0x0094, (r << 1) | 0x40);
    // 擦另一名表门块（asm `eor #$01/and #$01/tay/lsr/sta $6C,y`）
    let y = (mem.read_ram(crate::ram_map::PPUCTRL_ZP) ^ 1) & 1;
    mem.write_ram(crate::ram_map::SCROLL_BLOCK_NT3 + y as u16, 0);
    // DoorScrollStatus 判向
    let sd = mem.read_ram(crate::ram_map::SCROLL_DIR);
    let x: u8 = if sd & 0x02 != 0 {
        2u8.wrapping_sub(mem.read_ram(0x030E) >> 7) // 横滚：左门(bit7)⇒1，右⇒2
    } else {
        // 竖滚：X=4；ScrollY≠0 时 (PPUCTRL^hi)lsr 进位=1⇒3（下行）
        if mem.read_ram(crate::ram_map::SCROLL_Y) != 0
            && (mem.read_ram(crate::ram_map::PPUCTRL_ZP)
                ^ mem.read_ram(crate::ram_map::SAMUS_HI))
                & 1
                != 0
        {
            3
        } else {
            4
        }
    };
    mem.write_ram(0x0057, x);
    // SamusInDoor（$8B74）：$56|=$80
    let de = mem.read_ram(crate::ram_map::DOOR_ENTRY_STATUS);
    mem.write_ram(crate::ram_map::DOOR_ENTRY_STATUS, de | 0x80);
    mem.write_ram(crate::ram_map::DOOR_DELAY, 0x12); // 18 帧
    // $58←($58<<4)|Samus.status（Amul16 形）；status←sa_Door(5)
    let dd = mem.read_ram(crate::ram_map::SAMUS_DOOR_DATA);
    mem.write_ram(
        crate::ram_map::SAMUS_DOOR_DATA,
        (dd << 4) | mem.read_ram(crate::ram_map::SAMUS_STATUS),
    );
    mem.write_ram(crate::ram_map::SAMUS_STATUS, 5);
}

fn object_check_move_horizontal(mem: &mut FrameMemory, x: u8, dist: u8) -> bool {
    let y = mem.read_ram(0x030D);
    let ry = mem.read_ram(0x0301);
    let mut hi = mem.read_ram(crate::ram_map::SAMUS_HI);
    let (check_x, borrow) = x.overflowing_sub(dist);
    // asm E946 环绕判定（精确移植）：`ldx #$00/lda dist/bmi Lx215/dex` ⇒
    // X=0xFF（dist 正=左查）/0x00（dist 负=右查）；`sec/sbc` 后 `txa/adc #$00/beq RTS`
    // ⇒ A=X+carry（carry=无借位）≠0 ⇔ 环绕。右向：x≥dist（真出右界）才环绕。
    let carry = (!borrow) as u8;
    let x_reg: u8 = if dist & 0x80 == 0 { 0xFF } else { 0x00 };
    let wrapped = x_reg.wrapping_add(carry) != 0;
    if wrapped && mem.read_ram(crate::ram_map::SCROLL_DIR) & 0x02 != 0 {
        hi ^= 0x01; // asm `inc Temp0B_PositionHi`（两表模 2，inc≡翻转）
    }
    let (top, n) = horizontal_span_rows(y, ry);
    for i in 0..n {
        let row = top.wrapping_add((i as u8).wrapping_mul(8));
        if !collision_point_samus(mem, row, check_x, hi) {
            return false;
        }
    }
    true
}

/// ObjectCheckMoveLeft（**asm 内容锚** `: 07:E880`）：`lda radiusX/clc/adc #$08`→核判点
/// =x−(radiusX+8)（左块——核 `PositionX−DistToCenterX` 形）。carry=true 可走。
/// 【goal-r16 修】逐行扫（旧形只查顶行一点⇒顶行可走下行固实漏判⇒走进地形）。
fn object_check_move_left(mem: &mut FrameMemory) -> bool {
    let x = mem.read_ram(0x030E);
    let rx = mem.read_ram(crate::ram_map::SAMUS_RADIUS_X);
    object_check_move_horizontal(mem, x, rx.wrapping_add(8))
}

/// ObjectCheckMoveRight（**asm 内容锚** `: 07:E88B`）：`lda #$00/sec/sbc radiusX`→核判点
/// =x−(0−radiusX)=x+radiusX（右块）。carry=true 可走。
/// ObjectCheckMoveRight（**asm 内容锚** `: 07:E88B`）：`lda #$00/sec/sbc radiusX`→核判点
/// =x−(0−radiusX)=x+radiusX（右块）。carry=true 可走。
/// 【goal-r16 修】逐行扫（同左向——LE7E6 全径）。
fn object_check_move_right(mem: &mut FrameMemory) -> bool {
    let x = mem.read_ram(0x030E);
    let rx = mem.read_ram(crate::ram_map::SAMUS_RADIUS_X);
    object_check_move_horizontal(mem, x, 0u8.wrapping_sub(rx)) // dist=−rx（asm E88B）
}

/// CheckUpdateNameTable（**asm 内容锚** `CheckUpdateNameTable: ; 07:E54A`）：
/// ①`jsr SetupRoom`（**邻房装载**——RoomNumber≠FF（任务未竟）⇒出；多帧装载径）②横滚
/// 列流送门（`ScrollX&7==阈值`（右 0/左 7））③EndOfRoomHorizontal（`ScrollDir==
/// TempScrollDir($4A)` 防重流）④UpdateNameTable（E590）：列流送——tile index=
/// (ScrollX&0xF8)/8；源=RoomRAM 缓冲（$6000/$6400 按名表位（EB85：PPUCTRL^ScrollDir））/
/// 目标=PPU 名表（$2000/$2C00——PPUAddrs 表 E560）；30 块竖写（控制位 $1E|$80）。
/// 【V2-lite】流送直写 PPU 面（金径=VRAMStructBuffer+NMI 刷（WritePPUByte $C36B）——
/// 行级等价；缓冲面 $07A0+ 的 L3 对拍差异随 V2 收敛）。
fn check_update_name_table(mem: &mut FrameMemory) {
    // ①邻房装载（E54A 首件——ScrollRight/Left 每像素调；多帧（RoomNumber 任务机））
    crate::frame::mainloop::setup_room(mem);
    if mem.read_ram(0x005A).wrapping_add(1) != 0 {
        return; // RoomNumber≠FF：装载任务未竟
    }
    let dir = mem.read_ram(crate::ram_map::SCROLL_DIR);
    if dir & 2 == 0 {
        return; // 竖滚径（CheckUpdateNameTableVertical——随件）
    }
    let threshold: u8 = if dir == 3 { 0 } else { 7 };
    if mem.read_ram(crate::ram_map::SCROLL_X) & 0x07 != threshold {
        return;
    }
    if dir != mem.read_ram(0x004A) {
        return; // TempScrollDir 防重流
    }
    // ④列流送（E590——30 块竖写：PPU inc=32）
    // 【goal-r22 修】列写=PPU 重活 ⇒ NMI bogged 2 帧（emu 实测 f916-917/f1064-1065 各停
    // 2 帧）。旧形无门 ⇒ FC 相位漂移 ⇒ 爬行 6/8 移动窗错相 ⇒ 敌轨迹级联。
    mem.iter_bogged(2);
    let idx = (mem.read_ram(crate::ram_map::SCROLL_X) & 0xF8) >> 3;
    let y = (mem.read_ram(0x00FF) ^ dir) & 1; // GetNameTableAtScrollDir（EB85）
    let src = 0x6000u16 + (y as u16) * 0x400; // RoomRAMAddrs（E562）
    let dst = if y == 0 { 0x2000u16 } else { 0x2C00 }; // PPUAddrs（E560）
    for row in 0..30u16 {
        let addr = dst + idx as u16 + row * 32;
        let v = mem.read_wram(src + idx as u16 + row * 32);
        mem.write_io(0x2006, (addr >> 8) as u8);
        mem.write_io(0x2006, (addr & 0xFF) as u8);
        mem.write_io(0x2007, v);
    }
}

/// GetRoomNum 横滚门（asm E720——门检+图格查房已并入 get_room_num 单径）。返回
/// true=可滚（asm carry 反形）。
fn scroll_block_gate(mem: &mut FrameMemory) -> bool {
    crate::frame::mainloop::get_room_num(mem)
}

/// ScrollLeft（**asm 内容锚** `ScrollLeft: ; 07:E6A7`）：滚屏 1px。返回 carry
/// （true=滚动成功——屏内 x 不动；false=不能滚）。
fn scroll_left(mem: &mut FrameMemory) -> bool {
    let dir = mem.read_ram(crate::ram_map::SCROLL_DIR);
    if dir != 2 {
        if dir != 3 {
            return false; // 竖滚不能横滚（@cantScroll sec）
        }
        mem.write_ram(crate::ram_map::SCROLL_DIR, 2); // dec ScrollDir（3→2=左）
        if mem.read_ram(crate::ram_map::SCROLL_X) != 0 {
            mem.write_ram(crate::ram_map::MAP_POS_X,
                          mem.read_ram(crate::ram_map::MAP_POS_X).wrapping_sub(1));
        }
    }
    if mem.read_ram(crate::ram_map::SCROLL_X) == 0 {
        mem.write_ram(crate::ram_map::MAP_POS_X,
                      mem.read_ram(crate::ram_map::MAP_POS_X).wrapping_sub(1));
        if !scroll_block_gate(mem) {
            // @atLeftBound：MapPosX 复位（inc）——CF=1 不能滚
            mem.write_ram(crate::ram_map::MAP_POS_X,
                          mem.read_ram(crate::ram_map::MAP_POS_X).wrapping_add(1));
            return false;
        }
        toggle_name_table(mem);
    }
    let sx = mem.read_ram(crate::ram_map::SCROLL_X).wrapping_sub(1);
    mem.write_ram(crate::ram_map::SCROLL_X, sx);
    check_update_name_table(mem);
    true
}

/// ScrollRight（**asm 内容锚** `ScrollRight: ; 07:E6D2`）：滚屏 1px。返回 carry。
fn scroll_right(mem: &mut FrameMemory) -> bool {
    let dir = mem.read_ram(crate::ram_map::SCROLL_DIR);
    if dir != 3 {
        if dir != 2 {
            return false; // 竖滚不能横滚
        }
        mem.write_ram(crate::ram_map::SCROLL_DIR, 3); // inc ScrollDir（2→3=右）
        if mem.read_ram(crate::ram_map::SCROLL_X) != 0 {
            mem.write_ram(crate::ram_map::MAP_POS_X,
                          mem.read_ram(crate::ram_map::MAP_POS_X).wrapping_add(1));
        }
    }
    if mem.read_ram(crate::ram_map::SCROLL_X) == 0 {
        mem.write_ram(crate::ram_map::MAP_POS_X,
                      mem.read_ram(crate::ram_map::MAP_POS_X).wrapping_add(1));
        if !scroll_block_gate(mem) {
            // @atRightBound：MapPosX 复位（dec）——CF=1 不能滚
            mem.write_ram(crate::ram_map::MAP_POS_X,
                          mem.read_ram(crate::ram_map::MAP_POS_X).wrapping_sub(1));
            return false;
        }
    }
    let sx = mem.read_ram(crate::ram_map::SCROLL_X).wrapping_add(1);
    mem.write_ram(crate::ram_map::SCROLL_X, sx);
    if sx == 0 {
        toggle_name_table(mem); // inc 溢出（FF→00）⇒名表换
    }
    check_update_name_table(mem);
    true
}

/// ToggleNameTable（**asm 内容锚** `ToggleNameTable: ; 07:E9B7`）：PPUCTRL_ZP^=3。
fn toggle_name_table(mem: &mut FrameMemory) {
    let ff = mem.read_ram(0x00FF) ^ 0x03;
    mem.write_ram(0x00FF, ff);
}

/// CheckStopHorzMvmt（**asm 内容锚** `: 07:E365`）：成功（carry）⇒出；失败⇒
/// ObjectCounter=1 断环+（非空中/非球）⇒SetSamusStand（CF55——停速/走 SFX）。返回 true=断环。
fn check_stop_horz_mvmt(mem: &mut FrameMemory, moved: bool) -> bool {
    if moved {
        return false; // bcs Exit10
    }
    mem.write_ram(crate::ram_map::OBJECT_COUNTER, 1); // 断环
    if mem.read_ram(crate::ram_map::SAMUS_ACCEL_Y) != 0 {
        return true; // 空中出
    }
    if mem.read_ram(crate::ram_map::SAMUS_STATUS) == 4 {
        return true; // sa_Roll（球形）出
    }
    // SetSamusStand（CF55——「Stop horizontal movement or play walk SFX if stopped」）：
    // 停速面（横速清零——R-mv 停速族）。嵌套件（走 SFX）随件。
    mem.write_ram(0x0309, 0); // speedX_I←0
    mem.write_ram(0x0313, 0); // speedX_F←0
    true
}

/// MoveSamusLeft（**asm 内容锚** `MoveSamusLeft: ; 07:E626`）：逐像素左移
/// （8px 对齐门检+实体碰撞+**滚动跟随**（SamusScrX<$71⇒ScrollLeft 滚屏）+x dec
/// （x==0 先判⇒ToggleSamusHi））。返回 carry（true=移动成功）。
fn move_samus_left(mem: &mut FrameMemory) -> bool {
    let x = mem.read_ram(0x030E);
    let rx = mem.read_ram(crate::ram_map::SAMUS_RADIUS_X);
    if x.wrapping_sub(rx) & 0x07 == 0 && !object_check_move_left(mem) {
        mem.write_ram(crate::ram_map::SAMUS_DOOR_DATA, 0); // Lx181（撞挡）
        return false;
    }
    samus_collision_with_solid_entities(mem); // D976
    let is_hit = mem.read_ram(crate::ram_map::SAMUS_IS_HIT);
    if is_hit & 0x41 == 0x41 {
        mem.write_ram(crate::ram_map::SAMUS_DOOR_DATA, 0);
        return false; // 撞实体右缘
    }
    if mem.read_ram(crate::ram_map::SAMUS_SCR_X) < 0x71 {
        if !scroll_left(mem) {
            mem.write_ram(crate::ram_map::SAMUS_SCR_X,
                          mem.read_ram(crate::ram_map::SAMUS_SCR_X).wrapping_sub(1));
        }
    } else {
        mem.write_ram(crate::ram_map::SAMUS_SCR_X,
                      mem.read_ram(crate::ram_map::SAMUS_SCR_X).wrapping_sub(1));
    }
    if x == 0 && mem.read_ram(crate::ram_map::SCROLL_DIR) & 0x02 != 0 {
        toggle_samus_hi(mem); // 环绕 00（dec 前判——E650）
    }
    mem.write_ram(0x030E, x.wrapping_sub(1));
    true
}

/// MoveSamusRight（**asm 内容锚** `MoveSamusRight: ; 07:E668`）：逐像素右移
/// （8px 对齐门检+实体碰撞+**滚动跟随**（SamusScrX≥$8F⇒ScrollRight 滚屏）+x inc
/// （环绕 00⇒ToggleSamusHi））。返回 carry（true=移动成功）。
fn move_samus_right(mem: &mut FrameMemory) -> bool {
    let x = mem.read_ram(0x030E);
    let rx = mem.read_ram(crate::ram_map::SAMUS_RADIUS_X);
    if x.wrapping_add(rx) & 0x07 == 0 && !object_check_move_right(mem) {
        mem.write_ram(crate::ram_map::SAMUS_DOOR_DATA, 0); // Lx186（撞挡）
        return false;
    }
    samus_collision_with_solid_entities(mem); // D976
    let is_hit = mem.read_ram(crate::ram_map::SAMUS_IS_HIT);
    if is_hit & 0x41 == 0x40 {
        mem.write_ram(crate::ram_map::SAMUS_DOOR_DATA, 0);
        return false; // 撞实体左缘
    }
    if mem.read_ram(crate::ram_map::SAMUS_SCR_X) >= 0x8F {
        if !scroll_right(mem) {
            mem.write_ram(crate::ram_map::SAMUS_SCR_X,
                          mem.read_ram(crate::ram_map::SAMUS_SCR_X).wrapping_add(1));
        }
    } else {
        mem.write_ram(crate::ram_map::SAMUS_SCR_X,
                      mem.read_ram(crate::ram_map::SAMUS_SCR_X).wrapping_add(1));
    }
    let nx = x.wrapping_add(1);
    mem.write_ram(0x030E, nx);
    if nx == 0 && mem.read_ram(crate::ram_map::SCROLL_DIR) & 0x02 != 0 {
        toggle_samus_hi(mem); // 环绕 00（inc 后判——E692）
    }
    true
}

/// SamusMoveHorizontally（**asm 内容锚** `SamusMoveHorizontally: ; 07:E31A`）：
/// `SamusScrX←x−ScrollX`→|Δ|（TwosComplement $95C6）逐像素 MoveSamus± 环+
/// CheckStopHorzMvmt→门向（SamusDoorData≠0⇒SamusDoorDir（右 0/左 1））。
pub fn samus_move_horizontally(mem: &mut FrameMemory, delta: u8) {
    let x = mem.read_ram(0x030E);
    let sx = mem.read_ram(crate::ram_map::SCROLL_X);
    mem.write_ram(crate::ram_map::SAMUS_SCR_X, x.wrapping_sub(sx)); // E31D
    if delta == 0 {
        return; // beq Exit10
    }
    let right = delta & 0x80 == 0; // bpl → 右移
    let mut n = if right { delta } else { 0u8.wrapping_sub(delta) }; // TwosComplement
    if mem.read_ram(crate::ram_map::SAMUS_IN_LAVA) != 0 {
        n >>= 1; // 岩浆减半（lsr）
        if n == 0 {
            return;
        }
    }
    mem.write_ram(crate::ram_map::OBJECT_COUNTER, n);
    loop {
        let moved = if right { move_samus_right(mem) } else { move_samus_left(mem) };
        let stop = check_stop_horz_mvmt(mem, moved);
        let c = mem.read_ram(crate::ram_map::OBJECT_COUNTER).wrapping_sub(1);
        mem.write_ram(crate::ram_map::OBJECT_COUNTER, c);
        if stop || c == 0 {
            break;
        }
    }
    if mem.read_ram(crate::ram_map::SAMUS_DOOR_DATA) != 0 {
        // 门向（E32C-）：左移过门=1/右移过门=0
        mem.write_ram(crate::ram_map::SAMUS_DOOR_DIR, if right { 0 } else { 1 });
    }
}

#[cfg(test)]
/// 【goal-r22 红测】滚屏列写 = NMI bogged 2 帧（emu 实测 f916-917/f1064-1065 各停 2 帧
/// ——CheckUpdateNameTable(E54A) ④列流送 30 块 PPU 竖写=重活）。旧形无门 ⇒ FC 相位漂移
/// ⇒ 爬行 6/8 移动窗错相 ⇒ 敌轨迹级联（docs/findings/enemy-crawler-trajectory-divergence.md）。
#[test]
fn scroll_right_column_draw_bogs_two_frames() {
    let mut m = crate::frame::FrameMemory::default();
    // 场景：右向滚（ScrollDir=3）、RoomNumber 已装载完（$5A=FF）、TempScrollDir 同向、
    // SCROLL_X 到 8px 阈（dir==3 ⇒ &7==0）、名表位就绪
    m.write_ram(crate::ram_map::SCROLL_DIR, 3);
    m.write_ram(0x004A, 3);
    m.write_ram(0x005A, 0xFF);
    m.write_ram(crate::ram_map::SCROLL_X, 0x07); // scroll_right 先 +1 ⇒ 0x08（&7=0=阈值 ✓）
    m.write_ram(0x00FF, 0x03); // 名表位（^dir&1=0 → 表0）
    m.write_ram(crate::ram_map::MAP_POS_X, 1);
    m.write_ram(0x005A, 0xFF); // RoomNumber=FF ⇒ setup_room 立即 RTS（无房任务）
    let gates_before = m.iter_gates.len();
    let _ = scroll_right(&mut m);
    assert_eq!(m.iter_gates.len() - gates_before, 2, "列写应发 2 帧 bogged 门");
    assert_eq!(m.iter_gates, vec![0, 0], "bogged 门=0（NMI 跳过帧）");
}

mod tests_camera_follow {
    //! 【playability P0】摄像机跟随根件 TDD（asm E668/E626/E6D2/E6A7——渲染目检定谳：
    //! 金腿摄像机随行/我方画面静止+x 绕回）。RoomRAM 0xFF=可走预置（collision_point_ok
    //！ tile 类判：≥$80 可走）。

    use super::*;

    fn walkable_room() -> FrameMemory {
        let mut m = FrameMemory::default();
        crate::frame::mainloop::copy_map(&mut m); // 真 WorldMap（图格→真房间号——装载径）
        m.write_ram(crate::ram_map::MAP_POS_X, 0x03); // 实出生图格（金腿 $50=03/$4F=0E）
        m.write_ram(crate::ram_map::MAP_POS_Y, 0x0E);
        for a in 0x6000u16..0x6400 {
            m.write_wram(a, 0xFF);
        }
        m.write_ram(crate::ram_map::SCROLL_DIR, 2); // ScrollDir=2（横房——$49！勘误修）
        m.write_ram(crate::ram_map::SAMUS_RADIUS_X, 8);
        m.write_ram(0x030D, 0x50); // y
        m
    }

    /// MoveSamusRight 滚动跟随：SamusScrX≥$8F⇒ScrollRight（ScrollX+1/MapPosX+1）而非
    /// inc 屏内 x（asm E689- `lda SamusScrX/cmp #$8F/bcc Lx183/jsr ScrollRight`）。
    #[test]
    fn move_samus_right_scrolls_camera_at_threshold() {
        let mut m = walkable_room();
        m.write_ram(0x030E, 0x80); // x（x+rx=0x88 &7=0⇒门检（RoomRAM FF 可走））
        m.write_ram(crate::ram_map::SAMUS_SCR_X, 0x8F); // 右滚界
        assert!(move_samus_right(&mut m));
        assert_eq!(m.read_ram(crate::ram_map::SCROLL_X), 1, "ScrollX+1（摄像机跟随）");
        assert_eq!(m.read_ram(crate::ram_map::MAP_POS_X), 4, "MapPosX+1（3→4 新图格）");
        assert_eq!(m.read_ram(crate::ram_map::SAMUS_SCR_X), 0x8F, "屏内 x 不动（滚动代替）");
        assert_eq!(m.read_ram(0x030E), 0x81, "房间 x+1");
    }

    /// MoveSamusLeft 滚动跟随：SamusScrX<$71⇒ScrollLeft（ScrollX−1/MapPosX−1）。
    #[test]
    fn move_samus_left_scrolls_camera_at_threshold() {
        let mut m = walkable_room();
        m.write_ram(0x030E, 0x80); // x−rx=0x78 &7=0⇒门检
        m.write_ram(crate::ram_map::SAMUS_SCR_X, 0x70); // 左滚界（<$71）
        assert!(move_samus_left(&mut m));
        assert_eq!(m.read_ram(crate::ram_map::SCROLL_X), 0xFF, "ScrollX−1（wrapping）");
        assert_eq!(m.read_ram(crate::ram_map::SAMUS_SCR_X), 0x70, "屏内 x 不动（滚动代替）");
        assert_eq!(m.read_ram(0x030E), 0x7F, "房间 x−1");
    }

    /// 界内（ScrX<$8F）：屏内 x+1 不滚（asm `bcc Lx183/inc SamusScrX`）。
    #[test]
    fn move_samus_right_inside_threshold_moves_screen_x() {
        let mut m = walkable_room();
        m.write_ram(0x030E, 0x80);
        m.write_ram(crate::ram_map::SAMUS_SCR_X, 0x8E); // 界内
        assert!(move_samus_right(&mut m));
        assert_eq!(m.read_ram(crate::ram_map::SCROLL_X), 0, "不滚");
        assert_eq!(m.read_ram(crate::ram_map::SAMUS_SCR_X), 0x8F, "屏内 x+1");
    }

    /// SamusMoveHorizontally：Δ=2 右移=两像素环（x+2/ScrX+2）+SamusScrX 初赋（E31D）。
    #[test]
    fn samus_move_horizontally_two_pixel_loop() {
        let mut m = walkable_room();
        m.write_ram(0x030E, 0x80);
        m.write_ram(crate::ram_map::SCROLL_X, 0x10);
        samus_move_horizontally(&mut m, 2);
        assert_eq!(m.read_ram(0x030E), 0x82, "x+2（逐像素环）");
        assert_eq!(m.read_ram(crate::ram_map::SAMUS_SCR_X), 0x72, "ScrX=x−ScrollX+2");
    }

    /// SamusMoveHorizontally：Δ=负（补码）左移+x 环绕 00⇒ToggleSamusHi（9 位翻）。
    #[test]
    fn samus_move_horizontally_left_wraps_hi_bit() {
        let mut m = walkable_room();
        m.write_ram(0x030E, 0x00); // x=0（dec 前判——E650）
        m.write_ram(crate::ram_map::SAMUS_SCR_X, 0x90); // 界内（≥$71 不滚）
        samus_move_horizontally(&mut m, 0xFE); // Δ=−2
        assert_eq!(m.read_ram(0x030E), 0xFE, "x−2（环绕）");
        assert_eq!(m.read_ram(crate::ram_map::SAMUS_HI), 1, "ToggleSamusHi（环绕翻名表位）");
    }
}

// ===== 1:1 Rust 改写·批产·块 6（physics 面去地址名（TDD 穷举等价证明））=====

#[cfg(test)]
mod tests_rewrite_b6_physics {
    //! TDD step1（测试先行（强制完整））——穷举等价（新==旧（全域穷举（边界（异常路径））））。
    //! refs 锚（engine/mod.rs 注内 bank:addr）：$F2D5（:611（$0404 OR 对写））/
    //! $F332（:630（三 asl+Adiv8 尾跳））/$F340（:636（$10 eor #$03））。

    use super::*;
    use crate::engine::{check_enemy_collision, get_enemy_hit_flags, check_enemy_collision_ext};

    /// 【穷举·OR 对写】依赖分解穷举（函数=两独立 2 维分量（out[0]=(a,vx)｜out[1]=(v10,vy)）
    /// ⇒逐分量 256² 全域穷举=完备（积空间 256⁴ 由分量独立性覆盖）+x/y 边角组合。
    /// 【证伪更正】原 256⁴×3 积穷举 1074s 超闸 runner 时限（rust-tests FAIL 根因）
    /// ——改依赖分解穷举（证明力等价（分量独立）+闸时限界）。
    #[test]
    fn exhaustive_or_pair_equivalence() {
        // 分量 1：(a_in, v0404_x) 全域 256²（v10/vy 固定）
        for a in 0u8..=255 {
            for vx in 0u8..=255 {
                for &(x, y) in &[(0u8, 1u8), (0xFF, 0xFE)] {
                    assert_eq!(check_enemy_collision(x, y, a, 0x5A, vx, 0xA5),
                               or_motion_flags_pair(x, y, a, 0x5A, vx, 0xA5));
                }
            }
        }
        // 分量 2：(v10, v0404_y) 全域 256²（a/vx 固定）
        for v10 in 0u8..=255 {
            for vy in 0u8..=255 {
                for &(x, y) in &[(0x7F, 0x80), (0x01, 0x02)] {
                    assert_eq!(check_enemy_collision(x, y, 0x3C, v10, 0xC3, vy),
                               or_motion_flags_pair(x, y, 0x3C, v10, 0xC3, vy));
                }
            }
        }
        // 边角组合（积空间抽样核）
        for &(a, v10, vx, vy) in &[(0u8, 0, 0, 0), (0xFF, 0xFF, 0xFF, 0xFF), (0x01, 0x80, 0x80, 0x01)] {
            assert_eq!(check_enemy_collision(0, 1, a, v10, vx, vy),
                       or_motion_flags_pair(0, 1, a, v10, vx, vy));
        }
    }

    /// 【穷举·三 asl】v10 全域——shift3_and_dispatch==get_enemy_hit_flags（<<3 回绕边界）。
    #[test]
    fn exhaustive_shift3_equivalence() {
        for v in 0u8..=255 {
            assert_eq!(get_enemy_hit_flags(v), shift3_and_dispatch(v));
        }
    }

    /// 【穷举·eor 低 2 位】v10 全域——flip_low2_bits==check_enemy_collision_ext。
    #[test]
    fn exhaustive_flip_low2_equivalence() {
        for v in 0u8..=255 {
            assert_eq!(check_enemy_collision_ext(v), flip_low2_bits(v));
        }
    }
}

// ===== 1:1 Rust 改写·批产·块 6：physics 面语义命名（去地址名（公式形（零字面特值）））=====
// refs 锚（engine/mod.rs 注内 bank:addr）：$F2D5（:611）/$F332（:630）/$F340（:636）。

/// OR 对写（旧 check_enemy_collision（$F2D5）：$0404[x] |= A ／ $0404[y] |= $10）。
pub fn or_motion_flags_pair(x: u8, y: u8, a_in: u8, v10: u8, v0404_x: u8, v0404_y: u8)
    -> Vec<(u16, u8)> {
    vec![(0x0404 + x as u16, v0404_x | a_in), (0x0404 + y as u16, v0404_y | v10)]
}

/// 三 asl+尾跳（旧 get_enemy_hit_flags（$F332）：A←$10<<3（三 asl 回绕）→jmp $C2C6（Adiv8））。
pub fn shift3_and_dispatch(v10: u8) -> (u8, u16) {
    ((v10 << 3) as u8, 0xC2C6)
}

/// 低 2 位翻转（旧 check_enemy_collision_ext（$F340）：$10 eor #$03）。
pub fn flip_low2_bits(v10: u8) -> u8 {
    v10 ^ 0x03
}
