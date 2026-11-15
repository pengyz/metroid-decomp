//! 敌人域表读取与地址换算（合成自 `prims.rs`，语义分层）。
//!
//! asm 根：`LoadTableAt977B`、`ReadTableAt968B`（区域属性表）、`EnemyMovementChoiceOffset`。

use crate::frame::tick::*;
use crate::frame::FrameMemory;

pub(crate) use crate::enemy::prep::{EN_BASE, ENS_BASE, ENS_F_TYPE};

/// LoadTableAt977B（**asm 内容锚** `areas_common.asm:184 $80B0`——L977B[type]×2）。
pub(crate) fn load_table_at_977b(mem: &mut crate::frame::FrameMemory, x: u8) -> u8 {
    let t = mem.read_wram(0x6AF4 + x as u16 + 14) as usize;
    let e = game_data::assets::enemy_facing(crate::frame::cur_area(mem))[t & 0x3F];
    e.wrapping_shl(1)
}

/// `L977B[type]` 的**原始字节**。
///
/// 注意：asm 的 `LoadTableAt977B` 返回 `A = L977B[y] << 1`（后面的 `bpl/bmi` 因此测的是
/// **原字节 bit6**）。这里给原始值，各调用点用 raw 位判据，避免再被 `asl` 绕一次。
pub(crate) fn l977b_raw(mem: &mut crate::frame::FrameMemory, x: u8) -> u8 {
    load_table_at_977b(mem, x) >> 1
}

/// `L968B[type]`（`ReadTableAt968B`）——区域相关属性表。
pub(crate) fn l968b_of(mem: &mut crate::frame::FrameMemory, x: u8) -> u8 {
    let t = (mem.read_wram(ens_addr(x, ENS_F_TYPE)) & 0x0F) as usize;
    game_data::assets::enemy_force_attribs(crate::frame::cur_area(mem))
        .get(t)
        .copied()
        .unwrap_or(0)
}

pub(crate) fn en_addr(x: u8, f: u16) -> u16 {
    EN_BASE + x as u16 + f
}

pub(crate) fn ens_addr(x: u8, f: u16) -> u16 {
    ENS_BASE + x as u16 + f
}

/// GetEnemyTypeTimes2PlusFacingDirection（**asm 内容锚**——索引=type×2+facing（data05 bit0?））。
pub(crate) fn enemy_type_times2_plus_facing(mem: &mut crate::frame::FrameMemory, x: u8) -> u16 {
    let t = mem.read_wram(0x6AF4 + x as u16 + 14) as u16; // EnsExtra.type
    // 【实锤②修（2026-10-29）】asm GetEnemyTypeTimes2PlusFacingDirection（$F844）=
    // **axis 感应 facing**（`lda data05/bpl @common/lsr lsr/@common: lsr/lda type/rol`）
    // ——bit7 置（竖轴）⇒carry=bit3/清⇒carry=bit0——旧形固定 bit0（注带「?」）
    // ⇒顶爬虫（竖轴）读错位⇒EnemyRestingAnim 表错项⇒**持续错帧**（f800 106px 根因）。
    let d05 = mem.read_ram(0x0400 + x as u16 + 5);
    let facing: u16 = if d05 & 0x80 != 0 { ((d05 >> 3) & 1) as u16 } else { (d05 & 1) as u16 };
    (t * 2 + facing) & 0x1F
}
