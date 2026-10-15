//! F2CA 族组合流暂存面（地址只活在本层——$010F/$6E/$6F）。
//!
//! 地址证据（A 级）：bank07:F2FD `8D 0F 01` STA $010F（值=$80，`A9 80` LDA 立即数）；
//! bank07:F309 `85 6E` STA zp $6E / F30E `85 6F` STA zp $6F（值←ROM $95CE/$95CF）。
//!
//! 【2026-10-15 更正·A9 处置（task-14）】旧主张「ROM 常量按 window_bank=7 约定烘焙
//! （l2-proof-assumptions.md A9）：$95CE=0x03、$95CF=0x38（prg[0x1D5CE..0x1D5D0]=03 38
//! 自证）」随 A9 动态证伪**连带失效**（decode-sweep-r3-dynamic-close.md §1.2，A 级：
//! game 期 $8000 窗口=banks1-5 区域 bank，向量语义=按区域取数）。ROM 向量烘焙已
//! **窗口参数化**并迁 `memmap::rom_tables::{ROM_VEC_BY_BANK, rom_vec}`（ROM 常量正位
//! =rom_tables——two-layer 映射层）；旧 ROM_VEC_LO/ROM_VEC_HI 常量就此退役。
//! 行为钉死见证：oracle/subroutines/l2_cases.jsonl `b-f2ed-full-w*`/`b-f2e8-fall-full-w*`
//! 多窗口族（8 bank 全窗口三方全等）；语义未证（C 级）——向量/指针假设不变。

/// F2ED 流暂存址（栈页）。
pub const SCRATCH_010F: u16 = 0x010F;

/// F2ED 流暂存初值（bank07:F2FD `A9 80` LDA #$80——立即数，语义未证 C 级）。
pub const F2ED_SCRATCH_INIT: u8 = 0x80;

/// zp 向量低位暂存址。
pub const ZP_VEC_LO: u16 = 0x006E;

/// zp 向量高位暂存址。
pub const ZP_VEC_HI: u16 = 0x006F;

/// HealthChange 低/高（**同址双语境**：F2ED/F282 流=ROM 向量暂存面（上 ZP_VEC_*）；
/// FE83 伤害链=「Amount to add/subtract from Health」（constants_ram:596 `HealthChange dw ;$6E`）。
/// 双语境同字节=映射层职责（台账 N-0065 注记）。
pub const HEALTH_CHANGE_LO: u16 = ZP_VEC_LO;
pub const HEALTH_CHANGE_HI: u16 = ZP_VEC_HI;

/// SamusKnockbackIsBomb=$73（constants_ram:603；F311 Lx295 语义：0=斜向/1=竖直击退）。
pub const KNOCKBACK_IS_BOMB: u16 = 0x0073;

// ---- FE83 复活伤害流暂存面（task-9；zp 温度面，constants_ram TempNN 命名锚）----

/// Temp00_RoomRAMPtr 低（constants_ram:321；FE83 房间 RAM 指针装配面）。
pub const PTR_LO: u16 = 0x0000;

/// Temp00_RoomRAMPtr 高（:322）。
pub const PTR_HI: u16 = 0x0001;

/// Temp02_MetatileCenterY（:402；FF3C 输出 Y 像素）。
pub const CENTER_Y: u16 = 0x0002;

/// Temp03_MetatileCenterX（:403；FF3C 输出 X 像素）。
pub const CENTER_X: u16 = 0x0003;

/// Temp04_YSlotRadiusY（:255；F1FA 首轴阈值）。
pub const RAD_Y: u16 = 0x0004;

/// Temp05_YSlotRadiusX（:256；F1FA 次轴阈值）。
pub const RAD_X: u16 = 0x0005;

/// Temp07_XSlotPositionY（:258；X 槽=元块 Y 位）。
pub const XSLOT_Y: u16 = 0x0007;

/// Temp09_XSlotPositionX（:260）。
pub const XSLOT_X: u16 = 0x0009;

/// Temp0B_XSlotPositionHi（:262；nametable 位）。
pub const XSLOT_HI: u16 = 0x000B;

/// Temp0F_DistX（:263；F1FA 出口差值 X——F311 knockback 判据源）。
pub const DIST_X: u16 = 0x000F;

/// Temp10_DistHi（F1FA 差值高位位面；F338 `asl×3` 来源=isHit 掩码）。
pub const DIST_HI: u16 = 0x0010;

/// 写事件映射（暂存址；同值写也是事件）。
pub fn event_010f(v: u8) -> (u16, u8) {
    (SCRATCH_010F, v)
}

/// 写事件映射（zp 向量低位）。
pub fn event_vec_lo(v: u8) -> (u16, u8) {
    (ZP_VEC_LO, v)
}

/// 写事件映射（zp 向量高位）。
pub fn event_vec_hi(v: u8) -> (u16, u8) {
    (ZP_VEC_HI, v)
}
