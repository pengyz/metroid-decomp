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

use crate::frame::FrameMemory;
use crate::ram_map::{
    ON_FROZEN_ENEMY, SAMUS_ACCEL_Y, SAMUS_HORZ_SPEED_MAX, SAMUS_ON_ELEVATOR, SAMUS_RADIUS_Y,
    SAMUS_Y,
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
pub fn vert_accelerate(mem: &mut FrameMemory) {
    if mem.read_ram(SAMUS_ACCEL_Y) != 0 {
        return; // 空中——`bne @dontStartFalling`
    }
    mem.write_ram(SAMUS_HORZ_SPEED_MAX, 0x18); // 最大跑速 1.5px（R-mv 跑速面）
    // 8 像素对齐检测（几何节流）
    let aligned = (mem.read_ram(SAMUS_Y).wrapping_add(mem.read_ram(SAMUS_RADIUS_Y)) & 0x07) == 0;
    if aligned {
        object_check_move_down(mem); // ObjectCheckMoveDown（asm :8035——壳已端口）
    }
    samus_collision_with_solid_entities(mem); // D976（asm :5117——壳已端口）
    // 双旗门（D976 内部置位后检查）
    // if mem.read_ram(SAMUS_ON_ELEVATOR) != 0 || mem.read_ram(ON_FROZEN_ENEMY) != 0 { return; }
    // mem.write_ram(SAMUS_ACCEL_Y, 0x1A); // R-mv-6 下落发起
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

    /// D147 双写微件（speedY/subpixY 清零全端口）。
    #[test]
    fn stop_vert_movement_clears_two_faces() {
        let mut m = FrameMemory::default();
        m.write_ram(SAMUS_SPEED_Y_I, 0x80);
        m.write_ram(SAMUS_SUBPIX_Y, 0x55);
        stop_vert_movement(&mut m);
        assert_eq!(m.read_ram(SAMUS_SPEED_Y_I), 0);
        assert_eq!(m.read_ram(SAMUS_SUBPIX_Y), 0);
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
        lava_and_move_check(&mut m); // 无炸（短路）
        m.write_ram(SAMUS_STATUS, 1); // run→E25D 桩
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| lava_and_move_check(&mut m)));
        let err = r.unwrap_err();
        let msg = err.downcast_ref::<String>().cloned()
            .or_else(|| err.downcast_ref::<&'static str>().map(|s| (*s).to_string()))
            .unwrap_or_default();
        assert!(msg.contains("unported @ bank07:E25D"), "岩浆检桩失准: {msg}");
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

    /// D976 双旗清（同值写事件——清零先行面）。
    #[test]
    fn solid_entities_clears_flags_then_walk() {
        let mut m = FrameMemory::default();
        m.write_ram(SAMUS_ON_ELEVATOR, 1);
        m.write_ram(ON_FROZEN_ENEMY, 1);
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            samus_collision_with_solid_entities(&mut m)
        }));
        let err = r.unwrap_err();
        let msg = err.downcast_ref::<String>().cloned()
            .or_else(|| err.downcast_ref::<&'static str>().map(|s| (*s).to_string()))
            .unwrap_or_default();
        assert!(msg.contains("unported @ bank07:D983"), "冻敌环失准: {msg}");
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

    /// ObjectCheckMoveDown 壳引爆（嵌套件 E7B5）。
    #[test]
    fn object_check_move_down_detonates_at_nested() {
        let mut m = FrameMemory::default();
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            object_check_move_down(&mut m)
        }));
        let err = r.unwrap_err();
        let msg = err.downcast_ref::<String>().cloned()
            .or_else(|| err.downcast_ref::<&'static str>().map(|s| (*s).to_string()))
            .unwrap_or_default();
        assert!(msg.contains("unported @ bank07:E7B5"), "嵌套件失准: {msg}");
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
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| vert_accelerate(&mut m)));
        let err = r.unwrap_err();
        let msg = err.downcast_ref::<String>().cloned()
            .or_else(|| err.downcast_ref::<&'static str>().map(|s| (*s).to_string()))
            .unwrap_or_default();
        assert!(msg.contains("unported @ bank07:E7B5"), "对齐卫件失准: {msg}"); // E7AD 壳已通→嵌套件
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
    crate::unported!("bank07:E7B5"); // ObjectCheckMoveVertical（几何+瓦片环——嵌套件）
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
    crate::unported!("bank07:D983"); // 冻敌环（嵌套几何件族——F1FA 族随拼装）
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
    let mut st = crate::integrators::IntYState {
        speed_i: mem.read_ram(SAMUS_SPEED_Y_I),
        speed_f: mem.read_ram(SAMUS_SPEED_Y_F),
        subpix: mem.read_ram(SAMUS_SUBPIX_Y),
    };
    let accel = mem.read_ram(SAMUS_ACCEL_Y);
    let out = crate::integrators::int_y(&mut st, accel);
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
    let mut st = crate::integrators::IntXState {
        speed_i: mem.read_ram(SAMUS_SPEED_X_I),
        speed_f: mem.read_ram(SAMUS_SPEED_X_F),
        subpix: mem.read_ram(SAMUS_SUBPIX_X),
    };
    let accel = mem.read_ram(crate::ram_map::SAMUS_ACCEL_X); // 待名址随考证
    let max_src = mem.read_ram(crate::ram_map::SAMUS_HORZ_SPEED_MAX);
    let out = crate::integrators::int_x(&mut st, accel, max_src);
    mem.write_ram(SAMUS_SPEED_X_I, st.speed_i);
    mem.write_ram(SAMUS_SPEED_X_F, st.speed_f);
    mem.write_ram(SAMUS_SUBPIX_X, st.subpix);
    mem.regs.a = out.disp;
    mem.regs.x = out.reg_x;
    out.disp
}

// ==== m2 预拼装·批1（移动深路径嵌套件——D147/CD92 全折+CD6D 壳）====

/// StopVertMovement（**字节自证** D147 `a0 00 8c 08 03 8c 10 03 60`——ldy #$00/
/// sty $0308（speedY）/sty $0310（subpixY）/rts）：**双写微件全端口**。
pub fn stop_vert_movement(mem: &mut FrameMemory) {
    mem.write_ram(SAMUS_SPEED_Y_I, 0);
    mem.write_ram(SAMUS_SUBPIX_Y, 0);
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
pub fn set_samus_data(mem: &mut FrameMemory) {
    update_obj_anim(mem, mem.regs.a); // DC8F（已端口——A=延迟初值面）
    if is_screw_attack_active(mem) {
        // 攻螺旋转调色（asm：`lda FrameCount/lsr/and #$03/ora #$80|PRIORITY/sta $6B`）
        let v = ((mem.read_ram(crate::ram_map::FRAME_COUNT) >> 1) & 0x03) | 0x80;
        mem.write_ram(0x006B, v);
    }
    check_health_status(mem); // CDFA（链三件——已端口）
    lava_and_move_check(mem); // E269（链四件——已端口）
    crate::unported!("bank07:CD92-tail"); // 链余件（Metroid 蓝化/DE4A 显示面——随件）
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
    crate::unported!("bank07:CE7x"); // CheckHealthBeep（低血蜂鸣——随件）
}

/// LavaAndMoveCheck（**asm 内容锚**——CD6D 链四件）：状态门（Elevator 续/Dead 出）→
/// IsSamusInLava（E25D——carry clear=in lava）→岩浆面（`SamusKnockbackDir($72)←$FF`
/// （不推）/ClearHealthChange（F323）/`SamusInvincibleDelay($70)←$32`（闪）/
/// FrameCount&3==0→SFX_SamusJump（4 帧跳 SFX））→伤害面（8 帧节拍：无 Varia 双伤/
/// **有 Varia 半伤**（gr_VARIA 位再用——carry 每隔帧））→`HealthChange($6E)←$07/
/// SubtractHealth`→`SamusInLava($64)←in-lava 状态`。
/// 嵌套件桩：IsSamusInLava/ClearHealthChange/SFX_SamusJump/SubtractHealth。
pub fn lava_and_move_check(mem: &mut FrameMemory) {
    let status = mem.read_ram(crate::ram_map::SAMUS_STATUS);
    if status != 4 {
        // sa_Elevator=4（表序）——非电梯
        if status >= 7 {
            return; // sa_Dead(7)/sa_Dead2(8) 出
        }
    }
    crate::unported!("bank07:E25D"); // IsSamusInLava（岩浆检——链内件）
    // 岩浆面（后随 E25D 裁定）：
    // mem.write_ram(SAMUS_KNOCKBACK_DIR, 0xFF); mem.write_ram(SAMUS_INVINCIBLE_DELAY, 0x32);
    // FrameCount&3==0→SFX_SamusJump；8 帧伤害（Varia 半伤）；HealthChange←$07；
    // mem.write_ram(SAMUS_IN_LAVA, 1);
}
