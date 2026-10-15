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
