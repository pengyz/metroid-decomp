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
