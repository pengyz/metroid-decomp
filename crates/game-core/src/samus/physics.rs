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
        crate::unported!("bank07:E7AD"); // ObjectCheckMoveDown（向下阻挡——task-19 考证件）
    }
    crate::unported!("bank07:D976"); // SamusCollisionWithSolidEntities（电梯冻敌双旗）
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
        assert!(msg.contains("unported @ bank07:E7AD"), "对齐卫件失准: {msg}");
        assert_eq!(m.read_ram(SAMUS_HORZ_SPEED_MAX), 0x18); // maxspeed 已初始化
    }
}
